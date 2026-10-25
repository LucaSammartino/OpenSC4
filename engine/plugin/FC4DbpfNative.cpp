// SPDX-License-Identifier: GPL-3.0-or-later
// data-formats sections 2 and 8.1. Public SDK calls only; no private ABI.
#include "FC4DbpfNative.h"
#include "FC4DbpfRotation.h"
#include "FC4HarnessLogic.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "FC4Observer.h"
#include "compat/FC4Compat.h"
#include "fc4.h"
#include "cIGZCOM.h"
#include "cIGZDBSegmentPackedFile.h"
#include "cIGZPersistDBSegment.h"
#include "cIGZPersistDBRecord.h"
#include "cIGZPersistResourceKeyList.h"
#include "cGZPersistResourceKey.h"
#include "cRZCOMDllDirector.h"
#include "cRZBaseString.h"
#include "GZServPtrs.h"
#include "cIGZPersistResourceManager.h"
#include "cIGZPersistDBSegmentMultiPackedFiles.h"
#include "cISC4City.h"
#include <windows.h>
#include <algorithm>
#include <array>
#include <cstring>
#include <cctype>
#include <deque>
#include <iterator>
#include <map>
#include <memory>
#include <set>
#include <string>
#include <vector>

namespace fc4 {
namespace {
using Key = std::array<uint32_t, 3>;
struct Entry { Key key; uint32_t offset, size; };
constexpr uint32_t kLimit = 16 * 1024 * 1024;
struct Counts {
    unsigned files = 0, opened = 0, rejected = 0, refusal = 0, differ = 0;
    unsigned indexes = 0, records = 0, raw = 0, payload = 0, compressed = 0, empty = 0;
    unsigned zero = 0, one = 0, multi = 0, failed = 0;
    unsigned listFalse = 0, keyDifferences = 0, countDifferences = 0;
    unsigned bounds = 0, boundsExact = 0;
};
struct HeldFile {
    cIGZPersistDBSegment* segment;
    cIGZDBSegmentPackedFile* packed;
    std::string path;
    bool owned = false, city = false;
    WIN32_FILE_ATTRIBUTE_DATA stamp{};
};
struct HeldRead {
    fc4_dbpf* rust = nullptr;
    std::map<Key, std::array<uint32_t, 7>> entries;
    decltype(entries)::const_iterator next;
    Counts before;
    DWORD elapsed = 0;
    ~HeldRead() { if (rust) fc4_dbpf_close(rust); }
};
struct HeldSample {
    int32_t julian = 0;
    std::vector<HeldFile> files;
    size_t next = 0;
    Counts counts;
    bool full = false;
    uint64_t month = 0;
    size_t available = 0, first = 0;
    std::unique_ptr<HeldRead> read;
    unsigned ticks = 0, maxSliceMs = 0;
    std::string cityPath;
    unsigned roots = 0, children = 0, nonpacked = 0, closed = 0;
    unsigned cityDiscovered = 0, citySelected = 0, cityCompared = 0;
    uint64_t generation = 0;
};
// Main-thread only. Retain only references explicitly acquired by QI, never
// borrowed manager/index pointers. Both readers run together for each file.
// An empty deque allocates too. Keep the default-off observer out of DLL
// initialization: allocating its queue there perturbs the game's allocation
// order even when no DBPF sample was requested. The pointer borrows the
// function-local queue below, whose lifetime still extends to DLL shutdown.
std::deque<HeldSample>* pending = nullptr;
bool shadowLoaded = false;
uint64_t shadowMonth = 0, completedFiles = 0, completedRecords = 0;
uint64_t coverageCycle = 0;
uint64_t saveGeneration = 0;
bool cityAfterSave = false;
std::map<std::string, unsigned> covered, rotationCovered;

std::string NormalPath(std::string path) {
    for (char& c : path) c = c == '/' ? '\\' : static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
    return path;
}
void ReleaseHeld(HeldFile& file) {
    if (!file.packed) return;
    // Only our fallback reader may be closed. Live game segments are untouched.
    if (file.owned) {
        file.segment->Close();
        FC4_RELEASE_CREATED(file.packed);
    } else FC4_RELEASE(file.packed);
    file.packed = nullptr;
}
bool Stable(const HeldFile& file) {
    WIN32_FILE_ATTRIBUTE_DATA now{};
    cRZBaseString path;
    file.packed->GetPath(path);
    return file.segment->IsOpen() && NormalPath(path.ToChar()) == NormalPath(file.path) &&
        GetFileAttributesExA(file.path.c_str(), GetFileExInfoStandard, &now) &&
        now.nFileSizeHigh == file.stamp.nFileSizeHigh && now.nFileSizeLow == file.stamp.nFileSizeLow &&
        CompareFileTime(&now.ftLastWriteTime, &file.stamp.ftLastWriteTime) == 0;
}

unsigned CoverageRecords(const std::map<std::string, unsigned>& files) {
    unsigned records = 0;
    for (const auto& file : files) records += file.second;
    return records;
}

// Validate interface table storage and every published method address against
// the identified supported image. These addresses are never called untyped.
bool Interface(const void* object, size_t methods) {
    const auto* version = compat::IdentifiedVersion();
    if (!version || !object) return false;
    MEMORY_BASIC_INFORMATION m{};
    if (!VirtualQuery(object, &m, sizeof(m)) || m.State != MEM_COMMIT ||
        (m.Protect & (PAGE_GUARD | PAGE_NOACCESS))) return false;
    const auto table = *static_cast<const void* const* const*>(object);
    if (!VirtualQuery(table, &m, sizeof(m)) || m.State != MEM_COMMIT || m.Type != MEM_IMAGE ||
        reinterpret_cast<uintptr_t>(m.AllocationBase) != version->imageBase ||
        reinterpret_cast<uintptr_t>(table) + methods * sizeof(void*) >
            reinterpret_cast<uintptr_t>(m.BaseAddress) + m.RegionSize) return false;
    for (size_t i = 0; i < methods; ++i) {
        if (!VirtualQuery(table[i], &m, sizeof(m)) || m.State != MEM_COMMIT || m.Type != MEM_IMAGE ||
            reinterpret_cast<uintptr_t>(m.AllocationBase) != version->imageBase ||
            !(m.Protect & (PAGE_EXECUTE | PAGE_EXECUTE_READ | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY)))
            return false;
    }
    return true;
}

void Difference(Counts& c, const char* file, const char* what, const Key& key = {}) {
    ++c.differ;
    if (c.differ <= 12) Log("dbpf native difference: file=%s check=%s key=%08X/%08X/%08X", file, what,
                             key[0], key[1], key[2]);
}
void Bytes(void* ctx, const uint8_t* p, size_t n) {
    auto& out = *static_cast<std::vector<uint8_t>*>(ctx);
    out.assign(p, p + n);
}
uint32_t Word(const uint8_t* p) {
    return uint32_t(p[0]) | (uint32_t(p[1]) << 8) | (uint32_t(p[2]) << 16) | (uint32_t(p[3]) << 24);
}
bool ReadAt(HANDLE f, uint32_t off, void* p, uint32_t n) {
    LARGE_INTEGER pos{}; pos.QuadPart = off;
    DWORD got = 0;
    return SetFilePointerEx(f, pos, nullptr, FILE_BEGIN) && ReadFile(f, p, n, &got, nullptr) && got == n;
}

// Independent byte provenance only: decode literal 7.0 index words from the
// locked file. Never seed the Rust index from native enumeration or this list.
bool RawIndex(HANDLE f, std::vector<Entry>& out) {
    uint8_t h[96]{};
    LARGE_INTEGER length{};
    if (!GetFileSizeEx(f, &length) || !ReadAt(f, 0, h, sizeof(h)) || std::memcmp(h, "DBPF", 4)) return false;
    const uint32_t count = Word(h + 36), offset = Word(h + 40);
    if (count > 1000000 || uint64_t(offset) + uint64_t(count) * 20 > uint64_t(length.QuadPart)) return false;
    std::vector<uint8_t> index(size_t(count) * 20);
    if (!ReadAt(f, offset, index.data(), static_cast<uint32_t>(index.size()))) return false;
    for (size_t i = 0; i < count; ++i) {
        const auto p = index.data() + i * 20;
        out.push_back({{Word(p), Word(p + 4), Word(p + 8)}, Word(p + 12), Word(p + 16)});
    }
    return true;
}

void CompareFile(cIGZCOM* com, const std::string& path, bool natural, Counts& c) {
    ++c.files;
    // Deny writes for the whole comparison; both engines open this exact file.
    HANDLE lock = CreateFileA(path.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING, 0, nullptr);
    if (lock == INVALID_HANDLE_VALUE) { ++c.refusal; return; }
    void* object = nullptr;
    if (!com->GetClassObject(GZCLSID_cGZDBSegmentPackedFile, GZIID_cIGZDBSegmentPackedFile, &object) || !object) {
        ++c.refusal; CloseHandle(lock);
        Log("dbpf native refusal: packed-file public factory unavailable clsid=086D4A89 iid=286D4A6A");
        return;
    }
    auto* packed = static_cast<cIGZDBSegmentPackedFile*>(object);
    FC4_ACQUIRED(packed, "DBPF packed-file factory");
    if (!Interface(packed, 39)) {
        ++c.refusal;
        Log("dbpf native refusal: packed-file interface not in supported image");
        // No methods through an unvalidated interface, including Release.
        CloseHandle(lock); return;
    }
    cIGZPersistDBSegment* segment = nullptr;
    if (!packed->QueryInterface(GZIID_cIGZPersistDBSegment, reinterpret_cast<void**>(&segment)) || !segment) {
        ++c.refusal; FC4_RELEASE_CREATED(packed); CloseHandle(lock); return;
    }
    FC4_ACQUIRED(segment, "DBPF persistence QI");
    if (!Interface(segment, 30)) {
        ++c.refusal; Log("dbpf native refusal: persistence interface not in supported image");
        FC4_RELEASE_CREATED(packed); CloseHandle(lock); return;
    }
    // AsIGZPersistDBSegment is borrowed. QI above owns the reference we release.
    if (packed->AsIGZPersistDBSegment() != segment) Difference(c, path.c_str(), "borrowed-view-QI");
    const bool init = packed->Init();
    cRZBaseString nativePath(path.c_str());
    const bool configured = init && packed->SetPath(nativePath);
    // [abi-ret] SDK cIGZPersistDBSegment::Open(bool,bool) returns bool;
    // cIGZPersistDBSegment(SC4) OpenIStream is a distinct streaming interface.
    // MSVC groups the SDK's Init overloads; Open occupies slot 6, not slot 5.
    const bool nativeOpen = configured && segment->Open(true, false);
    fc4_dbpf* rust = fc4_dbpf_open(reinterpret_cast<const uint8_t*>(path.data()), path.size());
    const bool rustOpen = rust != nullptr;
    if (nativeOpen != rustOpen) Difference(c, path.c_str(), "open");
    if (!configured) ++c.refusal;
    if (!nativeOpen && !rustOpen && configured) ++c.rejected;
    if (nativeOpen && rustOpen) {
        ++c.opened;
        std::vector<Entry> independent;
        if (!RawIndex(lock, independent)) { ++c.refusal; }
        else {
            const size_t rustCount = fc4_dbpf_entry_count(rust);
            if (rustCount != independent.size()) Difference(c, path.c_str(), "index-count");
            std::map<Key, Entry> first;
            std::map<Key, size_t> firstIndex;
            for (size_t i = 0; i < independent.size(); ++i) {
                const auto& e = independent[i];
                uint32_t words[7]{};
                if (!fc4_dbpf_entry(rust, i, words) || Key{words[0], words[1], words[2]} != e.key ||
                    words[3] != e.offset || words[4] != e.size) Difference(c, path.c_str(), "raw-Rust-index", e.key);
                first.emplace(e.key, e);
                firstIndex.emplace(e.key, i);
            }
            void* listObject = nullptr;
            if (!com->GetClassObject(GZCLSID_cIGZPersistResourceKeyList, GZIID_cIGZPersistResourceKeyList, &listObject) || !listObject) {
                ++c.refusal;
            } else {
                auto* list = static_cast<cIGZPersistResourceKeyList*>(listObject);
                FC4_ACQUIRED(list, "DBPF key-list factory");
                if (!Interface(list, 11)) { ++c.refusal; }
                else {
                    const bool listed = segment->GetResourceKeyList(*list);
                    std::vector<Key> actual;
                    const uint32_t listSize = list->Size();
                    const uint32_t recordCount = segment->GetRecordCount(nullptr);
                    for (uint32_t i = 0; listed && i < listSize; ++i) {
                        const auto& k = list->GetKey(i); // borrowed reference; copy only
                        actual.push_back({k.type, k.group, k.instance});
                    }
                    std::sort(actual.begin(), actual.end());
                    std::vector<Key> expected;
                    for (const auto& pair : first)
                        if (pair.first[0] != 0xE86B1EEF) expected.push_back(pair.first);
                    ++c.indexes;
                    // EX55: type-only DIR exclusion; false for an empty list.
                    const bool keysEqual = actual == expected;
                    const bool countEqual = recordCount == first.size();
                    c.listFalse += !listed;
                    c.keyDifferences += !keysEqual;
                    c.countDifferences += !countEqual;
                    Log("dbpf native key-list: natural=%d path=%s listed=%d list_size=%u copied=%u expected=%u "
                        "record_count=%u keys_equal=%d count_equal=%d",
                        natural, path.c_str(), listed, listSize, static_cast<unsigned>(actual.size()),
                        static_cast<unsigned>(expected.size()), recordCount, keysEqual, countEqual);
                    if (!keysEqual) {
                        std::vector<Key> missing, extra;
                        std::set_difference(expected.begin(), expected.end(), actual.begin(), actual.end(),
                                            std::back_inserter(missing));
                        std::set_difference(actual.begin(), actual.end(), expected.begin(), expected.end(),
                                            std::back_inserter(extra));
                        Log("dbpf native key delta: path=%s missing=%u extra=%u", path.c_str(),
                            static_cast<unsigned>(missing.size()), static_cast<unsigned>(extra.size()));
                        for (size_t i = 0; i < std::min<size_t>(missing.size(), 4); ++i)
                            Log("dbpf native missing key: %08X/%08X/%08X", missing[i][0], missing[i][1], missing[i][2]);
                        for (size_t i = 0; i < std::min<size_t>(extra.size(), 4); ++i)
                            Log("dbpf native extra key: %08X/%08X/%08X", extra[i][0], extra[i][1], extra[i][2]);
                    }
                    // Preserve the previous combined difference count per file.
                    if (listed != !expected.empty() || !keysEqual || !countEqual)
                        Difference(c, path.c_str(), "native-key-list");
                    FC4_RELEASE_CREATED(list);
                }
            }
            if (first.empty()) ++c.zero; else if (first.size() == 1) ++c.one; else ++c.multi;
            for (const auto& pair : first) {
                const auto& e = pair.second;
                const cGZPersistResourceKey key(e.key[0], e.key[1], e.key[2]);
                // Natural payloads include arbitrary types; no exemplar parsing.
                const auto rawSize = packed->GetRecordSizeRaw(key), size = segment->GetRecordSize(key);
                if (rawSize > kLimit || size > kLimit || e.size > kLimit) { ++c.refusal; continue; }
                ++c.records;
                uint32_t words[7]{};
                // Find the Rust entry in its own index, never via native keys.
                fc4_dbpf_entry(rust, firstIndex.at(e.key), words);
                if (!segment->TestForRecord(key) || rawSize != words[4] || size != words[5] ||
                    packed->GetRecordPosition(key) != words[3] || packed->GetRecordCompressed(key) != (words[6] != 0))
                    Difference(c, path.c_str(), "metadata", e.key);
                std::vector<uint8_t> original(std::max<uint32_t>(rawSize, 1)), own, disk(std::max<uint32_t>(e.size, 1));
                uint32_t got = rawSize;
                const uint32_t rawReturn = packed->ReadRecordRaw(key, original.data(), got);
                const bool ownRaw = fc4_dbpf_read_record(rust, key.type, key.group, key.instance, true, Bytes, &own);
                const bool diskRead = ReadAt(lock, e.offset, disk.data(), e.size);
                original.resize(rawSize); disk.resize(e.size);
                ++c.raw;
                if (!ownRaw || !diskRead || rawReturn != rawSize || got != rawSize || own != original || own != disk)
                    Difference(c, path.c_str(), "raw-bytes", e.key);
                // EX54: full successful raw reads return the stored byte count.
                if (c.records <= 2) Log("dbpf native raw status: natural=%d return=%u inout=%u", natural, rawReturn, got);
                cIGZPersistDBRecord* record = nullptr;
                const bool recordOpen = segment->OpenRecord(key, &record, cIGZFile::AccessMode::Read);
                own.clear();
                const bool ownRead = fc4_dbpf_read_record(rust, key.type, key.group, key.instance, false, Bytes, &own);
                if (!recordOpen || !record) {
                    ++c.failed;
                    if (ownRead != recordOpen) Difference(c, path.c_str(), "record-open", e.key);
                    continue;
                }
                FC4_ACQUIRED(record, "DBPF OpenRecord");
                if (!Interface(record, 13)) { ++c.refusal; continue; }
                cGZPersistResourceKey returned;
                record->GetKey(returned);
                original.assign(std::max<uint32_t>(size, 1), 0);
                const bool read = size == 0 || record->GetFieldVoid(original.data(), size);
                original.resize(size);
                ++c.payload;
                if (words[6]) ++c.compressed;
                if (!size) ++c.empty;
                if (!ownRead || !read || own != original || record->GetSize() != size ||
                    Key{returned.type, returned.group, returned.instance} != e.key)
                    Difference(c, path.c_str(), "payload", e.key);
                if (!segment->CloseRecord(record)) Difference(c, path.c_str(), "record-close", e.key);
                FC4_RELEASE_CREATED(record);
            }
        }
    }
    if (rust) fc4_dbpf_close(rust);
    // [abi-ret] SDK cIGZPersistDBSegment::Close() returns bool, not the
    // distinct SC4 streaming interface's slot-8 OpenOStream(key,out,bool).
    if (nativeOpen && !segment->Close()) Difference(c, path.c_str(), "segment-close");
    if (init && !packed->Shutdown()) Difference(c, path.c_str(), "shutdown");
    FC4_RELEASE(segment);
    FC4_RELEASE_CREATED(packed);
    CloseHandle(lock);
    Log("dbpf native file: natural=%d path=%s native_open=%d rust_open=%d", natural, path.c_str(), nativeOpen, rustOpen);
}
void Summary(const char* origin, const Counts& c) {
    Log("dbpf native summary: origin=%s files=%u opened=%u rejected=%u zero=%u one=%u multi=%u indexes=%u "
        "records=%u raw=%u payload=%u compressed=%u empty=%u failed=%u refusals=%u differences=%u fallback=0 resync=0 "
        "list_false=%u key_differences=%u count_differences=%u",
        origin, c.files, c.opened, c.rejected, c.zero, c.one, c.multi, c.indexes, c.records, c.raw, c.payload,
        c.compressed, c.empty, c.failed, c.refusal, c.differ, c.listFalse, c.keyDifferences, c.countDifferences);
}

void CollectKey(void* context, uint32_t type, uint32_t group, uint32_t instance) {
    static_cast<std::vector<Key>*>(context)->push_back({type, group, instance});
}
void CollectNativeKey(const cGZPersistResourceKey& key, void* context) {
    CollectKey(context, key.type, key.group, key.instance);
}

// Read the game's actual held segment, never Open/Close/Init/Shutdown it.
// The independent Rust handle opens only GENERIC_READ with normal sharing:
// no exclusive lock, changed sharing policy, factory replacement or registration.
bool CompareHeld(cIGZCOM* com, cIGZPersistDBSegment* segment, cIGZDBSegmentPackedFile* packed,
    Counts& c, HeldRead& read, DWORD started) {
    auto& rust = read.rust;
    auto& entries = read.entries;
    cRZBaseString path;
    packed->GetPath(path);
    if (!rust) {
        ++c.files;
        // Public SDK paths use the Windows ANSI code page; Rust paths are UTF-8.
        const int wn = MultiByteToWideChar(CP_ACP, 0, path.ToChar(), -1, nullptr, 0);
        if (wn <= 1) { ++c.refusal; return true; }
        std::vector<wchar_t> wide(wn);
        MultiByteToWideChar(CP_ACP, 0, path.ToChar(), -1, wide.data(), wn);
        const int un = WideCharToMultiByte(CP_UTF8, 0, wide.data(), wn - 1, nullptr, 0, nullptr, nullptr);
        std::string utf8(static_cast<size_t>(un), '\0');
        if (un <= 0 || !WideCharToMultiByte(CP_UTF8, 0, wide.data(), wn - 1, &utf8[0], un, nullptr, nullptr)) {
            ++c.refusal; return true;
        }
        rust = fc4_dbpf_open(reinterpret_cast<const uint8_t*>(utf8.data()), utf8.size());
        if (!rust) { ++c.refusal; Difference(c, path.ToChar(), "Rust-open"); return true; }
        ++c.opened;
        for (size_t i = 0; i < fc4_dbpf_entry_count(rust); ++i) {
            std::array<uint32_t, 7> words{};
            if (!fc4_dbpf_entry(rust, i, words.data())) { ++c.refusal; continue; }
            entries.emplace(Key{words[0], words[1], words[2]}, words);
        }
        void* object = nullptr;
        if (!com->GetClassObject(GZCLSID_cIGZPersistResourceKeyList, GZIID_cIGZPersistResourceKeyList, &object) || !object) {
            ++c.refusal;
        } else {
            auto* list = static_cast<cIGZPersistResourceKeyList*>(object);
            FC4_ACQUIRED(list, "DBPF shadow key-list factory");
            if (!Interface(list, 11)) { ++c.refusal; }
            else {
                const bool listed = segment->GetResourceKeyList(*list);
                std::vector<Key> original, own;
                // Avoid repeated indexed lookups: public EnumKeys visits the same
                // owned list once without assuming its container representation.
                list->EnumKeys(CollectNativeKey, &original);
                const uint32_t n = fc4_dbpf_packed_keys(rust, CollectKey, &own);
                std::sort(original.begin(), original.end());
                std::sort(own.begin(), own.end());
                ++c.indexes;
                if (listed != (n != 0) || original != own ||
                    segment->GetResourceKeyList(nullptr, nullptr) != n ||
                    segment->GetRecordCount(nullptr) != fc4_dbpf_packed_count(rust, false))
                    Difference(c, path.ToChar(), "keys/count/Boolean");
                FC4_RELEASE_CREATED(list);
            }
        }
        read.next = entries.cbegin();
    }
    while (read.next != entries.cend()) {
        // Commit the cursor only for the record processed on this slice. No
        // native record object or cursor is retained; both reads stay together.
        const auto& pair = *read.next++;
        const auto& w = pair.second;
        const cGZPersistResourceKey key(w[0], w[1], w[2]);
        // A bounded allocation refusal is incomplete coverage, never exact.
        constexpr uint32_t liveLimit = 256 * 1024 * 1024;
        if (w[4] > liveLimit || w[5] > liveLimit) {
            ++c.refusal;
            Log("dbpf refusal: path=%s key=%08X/%08X/%08X stored=%u logical=%u limit=%u",
                path.ToChar(), w[0], w[1], w[2], w[4], w[5], liveLimit);
            continue;
        }
        ++c.records;
        c.compressed += w[6] != 0;
        c.empty += w[5] == 0;
        if (!segment->TestForRecord(key) || packed->GetRecordSizeRaw(key) != w[4] ||
            segment->GetRecordSize(key) != w[5] || packed->GetRecordPosition(key) != w[3] ||
            packed->GetRecordCompressed(key) != (w[6] != 0))
            Difference(c, path.ToChar(), "held-metadata", pair.first);
        // Oversized, fully bounded buffers also check the untouched tail.
        std::vector<uint8_t> original(size_t(w[4]) + 2, 0xcc), own(original);
        uint32_t got = static_cast<uint32_t>(original.size()), expectedSize = got;
        const uint32_t result = packed->ReadRecordRaw(key, original.data(), got);
        const uint32_t expected = fc4_dbpf_read_raw(rust, w[0], w[1], w[2], own.data(), &expectedSize);
        ++c.raw;
        if (result != expected || got != expectedSize || original != own)
            Difference(c, path.ToChar(), "held-raw-return/size/bytes", pair.first);
        if (SettingInt("dbpf_bounds_shadow", nullptr, 0) != 0) {
            // Section 2.6.1: raw reads truncate to capacity, including QFS headers;
            // a successful zero-capacity read returns zero, not decoded size.
            // Both implementations start each read at the indexed record start.
            std::vector<uint32_t> capacities{0, 1, w[4] ? w[4] - 1 : 0, w[4]};
            std::sort(capacities.begin(), capacities.end());
            capacities.erase(std::unique(capacities.begin(), capacities.end()), capacities.end());
            for (uint32_t capacity : capacities) {
                std::vector<uint8_t> native(size_t(capacity) + 2, 0xcc), core(native);
                uint32_t nativeSize = capacity, coreSize = capacity;
                const uint32_t nativeResult = packed->ReadRecordRaw(key, native.data() + 1, nativeSize);
                const uint32_t coreResult = fc4_dbpf_read_raw(rust, w[0], w[1], w[2],
                    core.data() + 1, &coreSize);
                ++c.bounds;
                if (nativeResult == coreResult && nativeSize == coreSize && native == core &&
                    native.front() == 0xcc && native.back() == 0xcc) ++c.boundsExact;
                else Difference(c, path.ToChar(), "held-raw-bounds-return/size/bytes/guards", pair.first);
            }
        }
        got = expectedSize = 99;
        if (packed->ReadRecordRaw(key, nullptr, got) !=
            fc4_dbpf_read_raw(rust, w[0], w[1], w[2], nullptr, &expectedSize) || got != expectedSize)
            Difference(c, path.ToChar(), "held-null-raw-return/size", pair.first);
        original.assign(size_t(w[5]) + 2, 0xcc);
        own.clear();
        got = w[5];
        // Section 8.3: unsigned byte count, not Boolean; no record-object operation.
        const uint32_t payloadResult = segment->ReadRecord(key, original.data(), got);
        const bool payloadRead = fc4_dbpf_read_record(rust, w[0], w[1], w[2], false, Bytes, &own);
        ++c.payload;
        if (!payloadRead || payloadResult != w[5] || got != w[5] || own.size() != w[5] ||
            !std::equal(own.begin(), own.end(), original.begin()) ||
            original[w[5]] != 0xcc || original[size_t(w[5]) + 1] != 0xcc)
            Difference(c, path.ToChar(), "held-payload", pair.first);
        if (!w[6] && w[5]) {
            got = 1;
            uint8_t prefix = 0xcc;
            if (segment->ReadRecord(key, &prefix, got) != 1 || got != 1 ||
                own.empty() || prefix != own.front())
                Difference(c, path.ToChar(), "held-payload-prefix", pair.first);
        }
        got = 99;
        if (segment->ReadRecord(key, nullptr, got) != w[5] || got != 0)
            Difference(c, path.ToChar(), "held-null-payload-return/size", pair.first);
        // Return between records, rather than after a potentially >5 s file.
        // A single SDK call cannot be preempted; report the observed max slice.
        if (GetTickCount() - started >= 800) return false;
    }
    // A key absent from the independent unique index: rejection preserves size.
    Key absent{0, 0, 0};
    while (entries.count(absent)) ++absent[2];
    const cGZPersistResourceKey key(absent[0], absent[1], absent[2]);
    uint32_t size = 99;
    if (packed->ReadRecordRaw(key, nullptr, size) != UINT32_MAX || size != 99)
        Difference(c, path.ToChar(), "held-missing-raw", absent);
    size = 99;
    if (segment->ReadRecord(key, nullptr, size) != UINT32_MAX || size != 0)
        Difference(c, path.ToChar(), "held-missing-payload", absent);
    return true;
}

void VisitHeld(cIGZPersistDBSegment* segment, std::set<cIGZPersistDBSegment*>& visited, HeldSample& sample,
    bool child = false) {
    auto& c = sample.counts;
    if (!segment) { ++c.refusal; Log("dbpf refusal: null indexed segment"); return; }
    if (!visited.insert(segment).second) return;
    if (!Interface(segment, 30)) { ++c.refusal; Log("dbpf refusal: unsupported segment interface"); return; }
    cIGZDBSegmentPackedFile* packed = nullptr;
    if (segment->QueryInterface(GZIID_cIGZDBSegmentPackedFile, reinterpret_cast<void**>(&packed)) && packed) {
        FC4_ACQUIRED(packed, "DBPF shadow packed QI");
        if (!Interface(packed, 39)) { ++c.refusal; return; }
        if (segment->IsOpen()) {
            cRZBaseString path;
            packed->GetPath(path);
            HeldFile file{segment, packed, path.ToChar()};
            file.city = NormalPath(file.path) == NormalPath(sample.cityPath);
            if (!GetFileAttributesExA(file.path.c_str(), GetFileExInfoStandard, &file.stamp)) ++c.refusal;
            sample.files.push_back(file);
            if (child) ++sample.children; else ++sample.roots;
            sample.cityDiscovered += file.city;
            if (sample.full || file.city)
                Log("dbpf discovery: class=%s city=%d path=%s", child ? "packed-child" : "packed-root", file.city, file.path.c_str());
        }
        else { ++sample.closed; FC4_RELEASE(packed); }
        return;
    }
    cIGZPersistDBSegmentMultiPackedFiles* multi = nullptr;
    if (segment->QueryInterface(GZIID_cIGZPersistDBSegmentMultiPackedFiles, reinterpret_cast<void**>(&multi)) && multi) {
        FC4_ACQUIRED(multi, "DBPF shadow multi QI");
        if (!Interface(multi, 11)) { ++c.refusal; return; }
        const uint32_t count = multi->GetSegmentCount();
        // EX56: SDK gives direct pointers; treat them as borrowed, never Release
        // a pointer whose reference the shadow did not acquire.
        for (uint32_t i = 0; i < count; ++i) VisitHeld(multi->GetSegmentByIndex(i), visited, sample, true);
        FC4_RELEASE(multi);
        return;
    }
    // Directory/streaming segments do not expose the packed-file interface.
    ++sample.nonpacked;
}

void AddCityReader(cIGZCOM* com, HeldSample& sample) {
    // Section 8.3 explicitly permits a separate read-only reader if discovery
    // misses the current backing path. Never register it or change the live file.
    void* object = nullptr;
    if (!com || sample.cityPath.empty() ||
        !com->GetClassObject(GZCLSID_cGZDBSegmentPackedFile, GZIID_cIGZDBSegmentPackedFile, &object) || !object) {
        ++sample.counts.refusal; return;
    }
    auto* packed = static_cast<cIGZDBSegmentPackedFile*>(object);
    FC4_ACQUIRED(packed, "DBPF city fallback factory");
    if (!Interface(packed, 39)) { ++sample.counts.refusal; return; }
    auto* segment = packed->AsIGZPersistDBSegment(); // borrowed, held by packed
    cRZBaseString path(sample.cityPath.c_str());
    // [abi-ret] SDK persistence Open(bool,bool) returns bool, as in CompareFile;
    // this packed reader view is distinct from SC4's streaming interface.
    if (!Interface(segment, 30) || !packed->Init() || !packed->SetPath(path) || !segment->Open(true, false)) {
        ++sample.counts.refusal; FC4_RELEASE_CREATED(packed); return;
    }
    HeldFile file{segment, packed, sample.cityPath, true, true};
    if (!GetFileAttributesExA(file.path.c_str(), GetFileExInfoStandard, &file.stamp)) ++sample.counts.refusal;
    sample.files.push_back(file);
    ++sample.cityDiscovered;
    Log("dbpf discovery: class=city-readonly-fallback city=1 path=%s", file.path.c_str());
}
} // namespace

void DbpfShadowObserve(int32_t julian) {
    if (SettingInt("dbpf_shadow", nullptr, 0) == 0 &&
        SettingInt("dbpf_bounds_shadow", nullptr, 0) == 0) return;
    harness::Date end;
    const bool dated = harness::ParseIsoDate(Setting("autotest_end_date", "FC4_AUTOTEST_END_DATE"), &end);
    const int32_t endJulian = dated ? harness::JulianFromCivil(end.year, end.month, end.day) : 0;
    if (!DbpfSampleInWindow(julian, endJulian)) return;
    static std::deque<HeldSample> samples;
    pending = &samples;
    pending->emplace_back();
    auto& sample = pending->back();
    sample.julian = julian;
    sample.generation = saveGeneration;
    cRZBaseString cityPath;
    if (auto* city = ObserverCity()) city->GetCitySaveFilePath(cityPath);
    sample.cityPath = cityPath.ToChar();
    sample.full = !shadowLoaded;
    shadowLoaded = true;
    sample.month = sample.full ? 0 : shadowMonth++;
    auto& c = sample.counts;
    std::set<cIGZPersistDBSegment*> visited;
    auto* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    cIGZPersistResourceManagerPtr manager;
    if (!manager || !Interface(com, 4)) ++c.refusal;
    else {
        // EX56: public resource-manager roots, recursively expanding public
        // multi-packed segments. No private city-streaming ABI is inferred.
        for (uint32_t i = 0; i < manager->GetSegmentCount(); ++i)
            VisitHeld(manager->GetSegmentByIndex(i), visited, sample);
    }
    if (!sample.cityDiscovered) AddCityReader(com, sample);
    Log("dbpf city discovery: path=%s discovered=%u save_generation=%llu", sample.cityPath.c_str(),
        sample.cityDiscovered, static_cast<unsigned long long>(saveGeneration));
    std::sort(sample.files.begin(), sample.files.end(), [](const HeldFile& a, const HeldFile& b) {
        return a.path < b.path;
    });
    sample.available = sample.files.size();
    if (!sample.full) {
        const auto subset = DbpfMonthlySubset(sample.available, sample.month);
        sample.first = subset.first;
        std::vector<HeldFile> selected;
        for (size_t i = 0; i < sample.files.size(); ++i) {
            if (subset.Contains(i, sample.available) || (cityAfterSave && sample.files[i].city))
                selected.push_back(std::move(sample.files[i]));
            else ReleaseHeld(sample.files[i]);
        }
        sample.files = std::move(selected);
    }
    for (const auto& file : sample.files) sample.citySelected += file.city;
    cityAfterSave = false;
}

namespace {
void HeldSummary(const HeldSample& sample) {
    auto c = sample.counts;
    if (!c.files || !c.records) ++c.refusal;
    fc4_date date{};
    fc4_date_from_julian(sample.julian, &date);
    Log("dbpf shadow %04d-%02d-%02d: files=%u opened=%u indexes=%u records=%u raw=%u payload=%u "
        "compressed=%u empty=%u refusals=%u scope=registered-packed-roots+multi-children+city-backing transient=unobserved "
        "subset=%s first=%u selected=%u available=%u completed=%u/%u "
        "cumulative_files=%llu cumulative_records=%llu covered_files=%u/%u covered_records=%u "
        "rotation_cycle=%llu rotation_files=%u/%u rotation_records=%u/%u ticks=%u max_slice_ms=%u "
        "packed_roots=%u packed_children=%u nonpacked=%u closed=%u city_discovered=%u city_selected=%u city_compared=%u save_generation=%llu mismatches=%u",
        date.year, date.month, date.day_of_month, c.files, c.opened, c.indexes, c.records, c.raw, c.payload,
        c.compressed, c.empty, c.refusal, sample.full ? "full" : "rotate",
        static_cast<unsigned>(sample.first), static_cast<unsigned>(sample.files.size()),
        static_cast<unsigned>(sample.available), static_cast<unsigned>(sample.next),
        static_cast<unsigned>(sample.files.size()),
        static_cast<unsigned long long>(completedFiles), static_cast<unsigned long long>(completedRecords),
        static_cast<unsigned>(covered.size()), static_cast<unsigned>(sample.available), CoverageRecords(covered),
        static_cast<unsigned long long>(coverageCycle), static_cast<unsigned>(rotationCovered.size()),
        static_cast<unsigned>(sample.available), CoverageRecords(rotationCovered), CoverageRecords(covered),
        sample.ticks, sample.maxSliceMs, sample.roots, sample.children, sample.nonpacked, sample.closed,
        sample.cityDiscovered, sample.citySelected, sample.cityCompared,
        static_cast<unsigned long long>(sample.generation), c.differ + c.refusal);
    if (SettingInt("dbpf_bounds_shadow", nullptr, 0) != 0)
        Log("dbpf_bounds shadow %04d-%02d-%02d: %u/%u clean, records=%u compressed=%u empty=%u "
            "mismatches=%u", date.year, date.month, date.day_of_month, c.boundsExact, c.bounds,
            c.records, c.compressed, c.empty, c.bounds - c.boundsExact + c.refusal);
}
}

void DbpfShadowTick() {
    if (pending == nullptr || pending->empty()) return;
    auto& sample = pending->front();
    if (!sample.full && coverageCycle != sample.month / 12 + 1) {
        coverageCycle = sample.month / 12 + 1;
        rotationCovered.clear();
    }
    const DWORD started = GetTickCount();
    auto* com = RZGetCOMDllDirector() ? GZCOM() : nullptr;
    ++sample.ticks;
    // Sample dates select the file set. Comparisons retain the independent
    // reader and next record across ticks, without holding native record objects.
    while (sample.next < sample.files.size()) {
        auto& file = sample.files[sample.next];
        if (!sample.read) {
            sample.read = std::make_unique<HeldRead>();
            sample.read->before = sample.counts;
        }
        if (!com || sample.generation != saveGeneration || !Stable(file)) {
            ++sample.counts.refusal;
            Log("dbpf refusal: changed/closed/save-transition path=%s; rediscover at next sample", file.path.c_str());
        }
        else {
            const DWORD fileStarted = GetTickCount();
            const bool done = CompareHeld(com, file.segment, file.packed, sample.counts, *sample.read, started);
            sample.read->elapsed += GetTickCount() - fileStarted;
            if (!Stable(file)) {
                ++sample.counts.refusal;
                Log("dbpf refusal: file changed during slice path=%s", file.path.c_str());
            }
            if (!done) break;
            const auto before = sample.read->before;
            // Coverage credit requires the entire file to finish exactly. The
            // rotation always compares its complete index, raw and payload data.
            if (before.refusal == sample.counts.refusal && before.differ == sample.counts.differ) {
                const unsigned records = sample.counts.records - before.records;
                covered[file.path] = records;
                if (!sample.full) rotationCovered[file.path] = records;
                ++completedFiles;
                completedRecords += records;
                sample.cityCompared += file.city;
                if (file.city) Log("dbpf city compared: path=%s records=%u save_generation=%llu", file.path.c_str(),
                    records, static_cast<unsigned long long>(sample.generation));
            }
            if (sample.read->elapsed > 2000)
                Log("dbpf slow file: path=%s records=%u active_ms=%lu", file.path.c_str(),
                    static_cast<unsigned>(sample.read->entries.size()),
                    static_cast<unsigned long>(sample.read->elapsed));
        }
        sample.read.reset();
        ReleaseHeld(file);
        ++sample.next;
        if (GetTickCount() - started >= 800) break;
    }
    sample.maxSliceMs = (std::max)(sample.maxSliceMs, static_cast<unsigned>(GetTickCount() - started));
    if (sample.next == sample.files.size()) {
        HeldSummary(sample);
        pending->pop_front();
    }
}

bool DbpfShadowPending() { return pending != nullptr && !pending->empty(); }

void DbpfShadowBeforeSave() {
    if (SettingInt("dbpf_shadow", nullptr, 0) == 0 &&
        SettingInt("dbpf_bounds_shadow", nullptr, 0) == 0) return;
    ++saveGeneration;
    cityAfterSave = true;
    // Release acquired views while the game's owners are still registered;
    // do not become the last holder of an old city segment after re-registration.
    if (pending != nullptr)
        for (auto& sample : *pending)
            for (auto& file : sample.files)
                ReleaseHeld(file);
    // Literal section 8.3: in-flight reads lose stable-read status. Tick reports
    // refusals for them; the next monthly discovery uses the new registration.
    Log("dbpf save transition: generation=%llu rediscovery=next-month transient=unobserved",
        static_cast<unsigned long long>(saveGeneration));
}

void DbpfShadowShutdown() {
    // Never silently discard an unfinished monthly sample. A deadline that
    // interrupts it is explicitly non-exact, while all acquired QIs are released.
    if (pending == nullptr) return;
    for (auto& sample : *pending) {
        ++sample.counts.refusal;
        for (size_t i = sample.next; i < sample.files.size(); ++i)
            ReleaseHeld(sample.files[i]);
        HeldSummary(sample);
    }
    pending->clear();
    shadowLoaded = false;
    shadowMonth = completedFiles = completedRecords = coverageCycle = 0;
    covered.clear();
    rotationCovered.clear();
    saveGeneration = 0;
    cityAfterSave = false;
}

void DbpfNativeCityLoaded() {
    static bool done = false;
    if (done || SettingInt("dbpf_native", nullptr, 0) == 0) return;
    done = true;
    Log("dbpf native scope: model=production-Rust-Package transport=SDK-record-adapter "
        "packed_owner=original-independent file_owner=unclaimed");
    auto* com = RZGetCOMDllDirector() ? GZCOM() : nullptr; // borrowed service
    if (!Interface(com, 4)) { Log("dbpf native refusal: unsupported COM image"); return; }
    const std::string dir = Setting("dbpf_native_fixtures", nullptr);
    Counts constructed;
    WIN32_FIND_DATAA fd{};
    HANDLE search = FindFirstFileA((dir + "/*.dat").c_str(), &fd);
    if (dir.empty() || search == INVALID_HANDLE_VALUE) { Log("dbpf native refusal: no fixture inputs"); return; }
    do {
        if (!(fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY)) CompareFile(com, dir + "/" + fd.cFileName, false, constructed);
        if (constructed.refusal) break; // precise public-route refusal, no fallback
    } while (FindNextFileA(search, &fd));
    FindClose(search);
    Summary("constructed", constructed);
    if (constructed.refusal) return;
    std::string natural = Setting("dbpf_native_file", nullptr);
    if (natural.empty()) {
        char module[MAX_PATH * 2]{};
        const DWORD n = GetModuleFileNameA(nullptr, module, sizeof(module));
        if (n && n < sizeof(module)) {
            natural.assign(module, n);
            for (int i = 0; i < 2; ++i) natural.resize(natural.find_last_of("/\\"));
            natural += "/SimCity_1.dat";
        }
    }
    if (!natural.empty()) { Counts c; CompareFile(com, natural, true, c); Summary("natural", c); }
}
} // namespace fc4
