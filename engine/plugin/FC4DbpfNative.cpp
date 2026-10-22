// SPDX-License-Identifier: GPL-3.0-or-later
// data-formats sections 2 and 8.1. Public SDK calls only; no private ABI.
#include "FC4DbpfNative.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
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
#include <windows.h>
#include <algorithm>
#include <array>
#include <cstring>
#include <iterator>
#include <map>
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
};

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
                    for (const auto& pair : first) expected.push_back(pair.first);
                    ++c.indexes;
                    // EX55: the spec/SDK do not define empty-list return or DIR
                    // visibility/count semantics. Keep the literal full index;
                    // diagnose each predicate without fitting either output.
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
                    if (!listed || !keysEqual || !countEqual)
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
                if (!ownRaw || !diskRead || got != rawSize || own != original || own != disk)
                    Difference(c, path.c_str(), "raw-bytes", e.key);
                // SDK declares uint32_t but does not publish its success-value
                // meaning. Record it; do not invent an error contract (EX54).
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
} // namespace

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
