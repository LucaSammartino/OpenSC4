// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

// See FC4ExemplarFactory.h. Spec: data-formats.md §4.1-§4.6, §8, §8.1;
// foundations.md §10.1/§10.2/§17.2/§17.5/§17.6. The remaining slots follow
// data-formats §8.2 (answers EX46-EX49, glue-exemplar-contractfix4); EX50
// (GetPropertyList callers) is open: the plugin logs its callers.

#include "FC4ExemplarFactory.h"

#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4PropertyHolder.h"
#include "FC4RefAudit.h"
#include "fc4.h"
#include "GZServPtrs.h"
#include "cGZPersistResourceKey.h"
#include "cIGZPersistDBRecord.h"
#include "cIGZPersistDBSerialRecord.h"
#include "cIGZPersistResource.h"
#include "cIGZPersistResourceFactory.h"
#include "cIGZPersistResourceManager.h"
#include "cISCResExemplar.h"
#include "cISCResExemplarCohort.h"
#include "cFC4Message2Standard.h"
#include "cIGZMessageServer2.h"
#include "cIGZUnknownList.h"
#include "cRZBaseString.h"
#include "compat/FC4Compat.h"
#include <windows.h>
#include <intrin.h>
#include <algorithm>
#include <atomic>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <deque>
#include <map>
#include <utility>
#include <string>
#include <mutex>
#include <vector>

namespace fc4 {
namespace {

constexpr uint32_t kTypeExemplar = 0x6534284A;
constexpr uint32_t kTypeCohort = 0x05342861;
constexpr uint32_t kClsExemplarFactory = 0x453429B3;  // §8.1: the game's registration
constexpr uint32_t kClsCohortFactory = 0x053429C8;
constexpr uint32_t kIIDUnknown = 0x00000001;
constexpr uint32_t kIIDExemplar = 0xA52160F5;  // §4 table
constexpr uint32_t kIIDCohort = 0x452160FB;
constexpr uint32_t kIIDHolder = 0x25216283;
constexpr uint32_t kIIDResource = 0x456B8F1D;
constexpr uint32_t kIIDFactory = 0xA56B8F17;
// Private to FreeCity4: answers our object core (with a reference). Never
// asked by the game.
constexpr uint32_t kIIDFC4Core = 0x4FC4E5A1;
constexpr uint32_t kIIDSerialRecord = 0xC56B8F08;  // persistence §3.1
constexpr uint32_t kIIDResourceProperty = 0xE52184AB;  // §8.2 SerialWrite: kind-1 properties answer it
// §4.4 [choice]: at most 32 ancestors load below the record requested.
constexpr size_t kMaxDepth = 32;

std::atomic<bool> g_enabled{false};
std::atomic<bool> g_active{false};

// Parent loads can recursively contribute raw/parse/build time while the
// outer read waits in its parent stage.
std::atomic<uint64_t> g_readTicks[4]{};
std::atomic<uint32_t> g_deferred{0}, g_materialized{0};
std::atomic<uint64_t> g_materializeTicks{0};
uint64_t ReadClock() {
    LARGE_INTEGER t;
    QueryPerformanceCounter(&t);
    return static_cast<uint64_t>(t.QuadPart);
}
class ReadTimer {
public:
    ReadTimer() : start_(ReadClock()) {}
    ~ReadTimer() { g_readTicks[stage_] += ReadClock() - start_; }
    void Stage(unsigned stage) {
        const uint64_t now = ReadClock();
        g_readTicks[stage_] += now - start_;
        start_ = now;
        stage_ = stage;
    }
private:
    uint64_t start_;
    unsigned stage_ = 0;
};

struct Counters {
    std::atomic<uint32_t> reads{0}, readFails{0}, parentFails{0}, cycleFails{0}, resourceProps{0}, emptyProps{0};
    std::atomic<uint32_t> created{0}, lists{0}, listFails{0}, unsupported{0}, msgAdds{0};
    // §8.2 slots: messages posted / not posted (no message), slot 14 / 15,
    // serializable stubs 3-5, holder stubs 23/24, factory slot 8.
    std::atomic<uint32_t> msgPosted{0}, msgFailed{0}, tags{0}, rebuilds{0}, rebuildDest{0}, serialStubs{0};
    std::atomic<uint32_t> holderStubs{0}, slot8{0};
    // EX51: slot 18 with true on an empty own list (no message), slot 15
    // dest failures, SerialWrite calls / failures / text-mode refusals.
    std::atomic<uint32_t> removeAllNoPost{0}, rebuildDestFailed{0}, serialWrites{0}, serialWriteFails{0};
    std::atomic<uint32_t> serialWriteText{0};
    // EX50: GetPropertyList calls by caller module: SimCity 4.exe, FreeCity4
    // outside the contract fixture, the contract fixture, any other module.
    std::atomic<uint32_t> listsGame{0}, listsFC4{0}, listsFixture{0}, listsOther{0};
};
thread_local bool t_fixture = false;  // set while ContractTest builds its own objects
Counters g_count;

constexpr uint32_t kIIDProperty = 0x452160EC;     // §8.2: cISCProperty, the readers' PopFront IID
constexpr uint32_t kIIDUnknownList = 0x0000012E;  // §8.2: the list answers 0x12E and 1
constexpr uint32_t kMsgExemplarChanged = 0xC53D10AA;  // §8.2 change messages
constexpr uint32_t kMsgCohortChanged = 0x853D11F1;

// EX50 (open): who calls GetPropertyList on our objects. Each distinct return
// address is logged once when first seen (module + offset, thread, the read
// counter at that time) and with its count in the summary. Evidence for RE,
// no behaviour depends on it.
struct Caller {
    void* ra = nullptr;
    uint32_t count = 0, firstCall = 0, readsAtFirst = 0;
    bool mainThread = false;
};
constexpr size_t kMaxCallers = 24;
std::mutex g_callerLock;
Caller g_callers[kMaxCallers];
size_t g_callerCount = 0;
std::atomic<uint32_t> g_callerOverflow{0};
std::atomic<unsigned long> g_mainThread{0};

void DescribeAddress(void* ra, char* out, size_t n) {
    HMODULE mod = nullptr;
    char path[MAX_PATH] = "?";
    if (GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                           static_cast<LPCSTR>(ra), &mod) &&
        mod != nullptr) {
        GetModuleFileNameA(mod, path, MAX_PATH);
    }
    const char* base = path;
    for (const char* p = path; *p != 0; ++p)
        if (*p == '\\' || *p == '/') base = p + 1;
    const uintptr_t off = reinterpret_cast<uintptr_t>(ra) - reinterpret_cast<uintptr_t>(mod);
    snprintf(out, n, "%s+0x%06lX (abs 0x%08lX)", base, static_cast<unsigned long>(off),
             static_cast<unsigned long>(reinterpret_cast<uintptr_t>(ra)));
}

const char* ClassifyCaller(void* ra) {
    HMODULE mod = nullptr;
    GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                       static_cast<LPCSTR>(ra), &mod);
    HMODULE self = nullptr;
    GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                       reinterpret_cast<LPCSTR>(&ClassifyCaller), &self);
    if (mod != nullptr && mod == GetModuleHandleA(nullptr)) {
        ++g_count.listsGame;
        return "game";
    }
    if (mod != nullptr && mod == self) {
        if (t_fixture) {
            ++g_count.listsFixture;
            return "fc4-fixture";
        }
        ++g_count.listsFC4;
        return "fc4";
    }
    ++g_count.listsOther;
    return "other";
}

void NoteCaller(void* ra, uint32_t call) {
    const char* cls = ClassifyCaller(ra);
    std::lock_guard<std::mutex> g(g_callerLock);
    for (size_t i = 0; i < g_callerCount; ++i) {
        if (g_callers[i].ra == ra) {
            ++g_callers[i].count;
            return;
        }
    }
    if (g_callerCount == kMaxCallers) {
        ++g_callerOverflow;
        return;
    }
    Caller& c = g_callers[g_callerCount++];
    c.ra = ra;
    c.count = 1;
    c.firstCall = call;
    c.readsAtFirst = g_count.reads.load();
    c.mainThread = GetCurrentThreadId() == g_mainThread.load();
    char where[MAX_PATH + 48];
    DescribeAddress(ra, where, sizeof where);
    Log("exemplars: EX50 GetPropertyList new caller %s class=%s first_call=%u reads=%u thread=%s", where, cls, call,
        c.readsAtFirst, c.mainThread ? "main" : "other");
}

void LogCallers() {
    std::lock_guard<std::mutex> g(g_callerLock);
    for (size_t i = 0; i < g_callerCount; ++i) {
        char where[MAX_PATH + 48];
        DescribeAddress(g_callers[i].ra, where, sizeof where);
        Log("exemplars: EX50 caller %s count=%u first_call=%u reads_at_first=%u thread=%s", where,
            g_callers[i].count, g_callers[i].firstCall, g_callers[i].readsAtFirst,
            g_callers[i].mainThread ? "main" : "other");
    }
    if (g_callerOverflow.load() != 0) Log("exemplars: EX50 callers past the table: %u calls", g_callerOverflow.load());
    Log("exemplars: EX50 GetPropertyList by module game=%u fc4=%u fc4_fixture=%u other=%u", g_count.listsGame.load(),
        g_count.listsFC4.load(), g_count.listsFixture.load(), g_count.listsOther.load());
}

// EX50 entry tracing: the concrete property / serialization virtual entries
// record the return address of THEIR caller (taken in the entry with
// _ReturnAddress and passed in), only for the outermost of our entries on the
// thread. Calls nested inside another of our entries (the rebuild's
// GetPropertyList on the parent, say) are counted under that outer entry, so
// an internal FC4 frame never stands for a consumer. Classes: game
// (SimCity 4.exe), other (any other module), fc4 (our plugin outside the
// contract fixture), fixture (the contract test). Evidence only.
enum EntryId { kEnGetList, kEnSetCohort, kEnSetParent, kEnRebuild, kEnTag, kEnCreateRec, kEnFRead, kEnFWrite,
               kEnSerialWrite, kEnCount };
const char* const kEntryNames[kEnCount] = {"GetPropertyList", "SetCohort", "SetParent", "RebuildEffective",
                                           "DebugInstanceTag", "CreateInstance(record)", "Factory.Read",
                                           "Factory.Write", "SerialWrite"};
enum CallerClass { kClGame, kClOther, kClFC4, kClFixture, kClCount };
const char* const kClassNames[kClCount] = {"game", "other", "fc4", "fixture"};
std::atomic<uint32_t> g_entryOuter[kEnCount][kClCount];
std::atomic<uint32_t> g_listsNestedUnder[kEnCount];
thread_local int t_entry = -1;

CallerClass ModuleClass(void* ra) {
    HMODULE mod = nullptr;
    GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                       static_cast<LPCSTR>(ra), &mod);
    HMODULE self = nullptr;
    GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
                       reinterpret_cast<LPCSTR>(&ModuleClass), &self);
    if (mod != nullptr && mod == GetModuleHandleA(nullptr)) return kClGame;
    if (mod != nullptr && mod == self) return t_fixture ? kClFixture : kClFC4;
    return kClOther;
}

struct EntrySite {
    int entry = -1;
    void* ra = nullptr;
    uint32_t count = 0;
    CallerClass cls = kClOther;
};
constexpr size_t kMaxSites = 48;
std::mutex g_siteLock;
EntrySite g_sites[kMaxSites];
size_t g_siteCount = 0;
std::atomic<uint32_t> g_siteOverflow{0};

class EntryScope {
public:
    EntryScope(EntryId id, void* ra) : outer_(t_entry < 0) {
        if (!outer_) {
            if (id == kEnGetList) ++g_listsNestedUnder[t_entry];
            return;
        }
        t_entry = id;
        const CallerClass cls = ModuleClass(ra);
        ++g_entryOuter[id][cls];
        std::lock_guard<std::mutex> g(g_siteLock);
        for (size_t i = 0; i < g_siteCount; ++i) {
            if (g_sites[i].entry == id && g_sites[i].ra == ra) {
                ++g_sites[i].count;
                return;
            }
        }
        if (g_siteCount == kMaxSites) {
            ++g_siteOverflow;
            return;
        }
        g_sites[g_siteCount++] = EntrySite{id, ra, 1, cls};
        char where[MAX_PATH + 48];
        DescribeAddress(ra, where, sizeof where);
        Log("exemplars: EX50 entry %s new caller %s class=%s thread=%s", kEntryNames[id], where, kClassNames[cls],
            GetCurrentThreadId() == g_mainThread.load() ? "main" : "other");
    }
    ~EntryScope() {
        if (outer_) t_entry = -1;
    }
    EntryScope(const EntryScope&) = delete;
    EntryScope& operator=(const EntryScope&) = delete;

private:
    bool outer_;
};

void LogEntries() {
    {
        std::lock_guard<std::mutex> g(g_siteLock);
        for (size_t i = 0; i < g_siteCount; ++i) {
            char where[MAX_PATH + 48];
            DescribeAddress(g_sites[i].ra, where, sizeof where);
            Log("exemplars: EX50 entry %s caller %s class=%s count=%u", kEntryNames[g_sites[i].entry], where,
                kClassNames[g_sites[i].cls], g_sites[i].count);
        }
    }
    if (g_siteOverflow.load() != 0) Log("exemplars: EX50 entry callers past the table: %u calls", g_siteOverflow.load());
    for (int e = 0; e < kEnCount; ++e) {
        Log("exemplars: EX50 entry %s natural game=%u other=%u | ours fc4=%u fixture=%u | lists_nested=%u",
            kEntryNames[e], g_entryOuter[e][kClGame].load(), g_entryOuter[e][kClOther].load(),
            g_entryOuter[e][kClFC4].load(), g_entryOuter[e][kClFixture].load(), g_listsNestedUnder[e].load());
    }
}

// §8.2 (EX49): the list GetPropertyList returns. The original's class has no
// GZCOM class ID, so it is rebuilt here with the documented slot table: 3
// count, 4 element QI, 5/17 visit, 6/8 insert a list front/back, 7/9 insert
// one front/back, 10/11 peek, 12/13 pop, 14 clear, 15 assign, 16 clone, 18
// deleting destructor. The SDK interface compiles to that order (MSVC puts an
// overload pair in reverse declaration order: Push_Front(list) at 6).
class cFC4UnknownList final : public cIGZUnknownList {
public:
    // §8.2: answers IIDs 0x12E and 1 only. [choice] a failed query leaves the
    // out-pointer as it was (the spec does not say it is cleared).
    bool QueryInterface(uint32_t riid, void** ppv) override {
        if (ppv == nullptr) return false;
        if (riid != kIIDUnknownList && riid != kIIDUnknown) return false;
        AddRef();
        *ppv = static_cast<cIGZUnknownList*>(this);
        return true;
    }
    uint32_t AddRef() override { return ++refs_; }
    // §8.2: a count of 1 or 0 at Release destroys the list, releasing every
    // element. [choice] returns the count left (0 when destroyed).
    uint32_t Release() override {
        if (refs_ <= 1) {
            delete this;
            return 0;
        }
        return --refs_;
    }
    size_t GetSize() override { return items_.size(); }
    bool GetElement(uint32_t index, uint32_t riid, void** ppv) override {
        if (index >= items_.size()) return false;
        return items_[index]->QueryInterface(riid, ppv);
    }
    bool EnumElements(cIGZUnknown& visitor, cIGZUnknown* a, uint32_t b) override { return Visit(visitor, a, b); }
    // §8.2 slot 7 / 9: insert one element at the front / back with an added
    // reference; true. [choice] a null element is refused (false).
    bool Push_Front(cIGZUnknown* e) override { return Insert(e, true); }
    bool Push_Front(cIGZUnknownList const& other) override { return InsertList(other, true); }
    bool Push_Back(cIGZUnknown* e) override { return Insert(e, false); }
    bool Push_Back(cIGZUnknownList const& other) override { return InsertList(other, false); }
    bool Peek_Front(uint32_t riid, void** ppv) override {
        return !items_.empty() && items_.front()->QueryInterface(riid, ppv);
    }
    bool Peek_Back(uint32_t riid, void** ppv) override {
        return !items_.empty() && items_.back()->QueryInterface(riid, ppv);
    }
    // §8.2 slot 12 / 13: as peek, then remove the element if the query
    // succeeded (the list's own reference is released).
    bool Pop_Front(uint32_t riid, void** ppv) override {
        if (!Peek_Front(riid, ppv)) return false;
        items_.front()->Release();
        items_.pop_front();
        return true;
    }
    bool Pop_Back(uint32_t riid, void** ppv) override {
        if (!Peek_Back(riid, ppv)) return false;
        items_.back()->Release();
        items_.pop_back();
        return true;
    }
    bool Clear() override {
        std::deque<cIGZUnknown*> old;
        old.swap(items_);
        for (auto* e : old) e->Release();
        return true;
    }
    // §8.2 slot 15: clear, then slot 6 with the other list.
    bool CopyFrom(cIGZUnknownList const& other) override {
        Clear();
        return Push_Front(other);
    }
    // §8.2 slot 16: a new list (one reference) holding the same elements.
    bool CreateClone(cIGZUnknownList** out) override {
        if (out == nullptr) return false;  // [choice]
        auto* l = new cFC4UnknownList();
        for (auto* e : items_) l->Insert(e, false);
        l->AddRef();
        *out = l;
        return true;
    }
    bool ConstEnumElements(cIGZUnknownEnumerator& visitor, cIGZUnknown* a, uint32_t b) override {
        return Visit(reinterpret_cast<cIGZUnknown&>(visitor), a, b);
    }
    // Slot 18: the deleting destructor (releases every element).
    virtual ~cFC4UnknownList() { Clear(); }

private:
    bool Insert(cIGZUnknown* e, bool front) {
        if (e == nullptr) return false;
        e->AddRef();
        if (front) items_.push_front(e);
        else items_.push_back(e);
        return true;
    }
    // §8.2 slot 6 / 8: every element of the other list, in its order, ahead
    // of / after ours. [choice] the other list is read through its own
    // public slots (4: element by index), so any cIGZUnknownList works.
    bool InsertList(cIGZUnknownList const& other, bool front) {
        auto& o = const_cast<cIGZUnknownList&>(other);
        std::vector<cIGZUnknown*> add;
        const size_t n = o.GetSize();  // [abi-ret] probed: data-formats 8.2 / EX51
        for (size_t i = 0; i < n; ++i) {
            void* p = nullptr;
            if (o.GetElement(static_cast<uint32_t>(i), kIIDUnknown, &p) && p != nullptr)
                add.push_back(static_cast<cIGZUnknown*>(p));  // one reference: the list's
        }
        items_.insert(front ? items_.begin() : items_.end(), add.begin(), add.end());
        return true;
    }
    // §8.2 slot 5 / 17: for each element front to back, the visitor's slot 3
    // (element, a, b); true.
    bool Visit(cIGZUnknown& visitor, cIGZUnknown* a, uint32_t b) {
        using Fn = bool(__thiscall*)(void*, cIGZUnknown*, cIGZUnknown*, uint32_t);
        Fn fn = (*reinterpret_cast<Fn**>(&visitor))[3];
        const std::vector<cIGZUnknown*> copy(items_.begin(), items_.end());
        for (auto* e : copy) fn(&visitor, e, a, b);
        return true;
    }
    uint32_t refs_ = 0;  // §8.2: the constructor leaves the count at 0
    std::deque<cIGZUnknown*> items_;
};

void Unsupported(const char* what) {
    // [choice, EX46-EX49] a slot whose behaviour the spec does not give: fail
    // and log the first few calls (evidence of whether the game uses it).
    const uint32_t n = ++g_count.unsupported;
    if (n <= 8) Log("exemplars: unsupported call %s (spec question; returns failure)", what);
}

bool SameKey(const cGZPersistResourceKey& a, const cGZPersistResourceKey& b) {
    return a.type == b.type && a.group == b.group && a.instance == b.instance;
}

// The records being read on this thread (outermost first): §4.4 cycle and
// depth guard. Failures are not remembered (§4.4).
thread_local std::vector<cGZPersistResourceKey> t_loading;

cIGZPersistResourceFactory* g_original[2] = {nullptr, nullptr};  // the game's, one reference each
cIGZPersistResourceFactory* g_ours[2] = {nullptr, nullptr};

int KindIndex(uint32_t type) { return type == kTypeCohort ? 1 : 0; }

class ExCore;

// The holder interface of our exemplar / cohort (§4.4): lookups on the
// effective list, EnumProperties on the own list (DG29), GetPropertyList the
// effective list. Lifetime goes to the owning object.
class ExHolder final : public cFC4ISCPropertyHolder {
public:
    explicit ExHolder(ExCore* o) : o_(o) {}
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;
    bool HasProperty(uint32_t id) const override;
    bool GetPropertyList(cIGZUnknownList** list) const override;
    bool GetPropertyData(uint32_t id, void* buffer, uint32_t& length) const override;
    bool GetPropertyInterface(uint32_t id, uint32_t riid, void** ppv) const override;
    bool GetPropertyString(uint32_t id, cIGZString& value) const override;
    bool GetPropertyU32(uint32_t id, uint32_t& value) const override;
    cISCProperty* GetProperty(uint32_t id) const override;
    bool AddPropertyData(uint32_t id, void* data, uint32_t length, bool b) override;
    bool AddPropertyObject(uint32_t id, cIGZUnknown* value, bool b) override;
    bool AddPropertyString(uint32_t id, cIGZString const& value, bool b) override;
    bool AddPropertyU32(uint32_t id, uint32_t value, bool b) override;
    bool AddPropertyVariant(uint32_t id, cFC4IGZVariant const* value, bool b) override;
    bool AddPropertyRecord(cISCProperty* property, bool b) override;
    bool CopyAddProperty(cISCProperty* property, bool b) override;
    bool RemoveProperty(uint32_t id, bool b) override;
    bool RemoveAllProperties(bool b) override;
    bool EnumPropertiesFiltered(FilterFn filter, EnumFn fn, void* data) const override;
    bool EnumProperties(EnumFn fn, void* data) const override;
    bool CompactProperties() override { return true; }
    // §8.2 holder slots 22-24 (the original's holder vtable has 25): 22 is
    // the deleting destructor of the whole object (releases the parent,
    // frees both lists); 23 and 24 take one argument and return false.
    // Nothing found calls them through the holder.
    virtual void* DeletingDestructor(uint32_t flags);
    virtual bool Slot23(uint32_t) {
        ++g_count.holderStubs;
        return false;
    }
    virtual bool Slot24(uint32_t) {
        ++g_count.holderStubs;
        return false;
    }

private:
    ExCore* o_;
};

// What both objects share: reference count, key, own / effective lists,
// parent cohort (one reference of ours).
class ExCore {
public:
    explicit ExCore(uint32_t type, cIGZUnknown* outer)
        : type_(type), key_(type, 0, 0), own_(outer), eff_(outer), holder_(this) {}
    virtual ~ExCore() {
        fc4_exrec_destroy(pendingRecord_);
        own_.Clear();
        eff_.Clear();
        if (parent_ != nullptr) FC4_RELEASE(parent_);
    }
    virtual cIGZUnknown* Outer() = 0;

    uint32_t type_;
    std::atomic<uint32_t> refs_{0};
    mutable std::recursive_mutex lock_;  // key and parent
    cGZPersistResourceKey key_;
    cFC4PropertyHolder own_;
    cFC4PropertyHolder eff_;
    ExHolder holder_;
    cISCResExemplarCohort* parent_ = nullptr;
    // Values are immutable in the parsed record; each exposed property gets
    // its own mutable holder object before any caller can observe it.
    // SPEC-GAP: value-property allocation is deferred until first access.
    fc4_exrec* pendingRecord_ = nullptr;
    std::vector<std::pair<uint32_t, size_t>> pending_;
    // Once the record is materialized, getters need only the holder's lock.
    // Getters also run off the main thread (lessons 5.1).
    std::atomic<bool> hasPending_{false};

    void ClearPending() {
        hasPending_.store(false, std::memory_order_release);
        pending_.clear();
        fc4_exrec_destroy(pendingRecord_);
        pendingRecord_ = nullptr;
    }
    void Materialize(uint32_t id) {
        if (!hasPending_.load(std::memory_order_acquire)) return;
        std::lock_guard<std::recursive_mutex> g(lock_);
        auto it = std::lower_bound(pending_.begin(), pending_.end(), id,
            [](const auto& p, uint32_t key) { return p.first < key; });
        if (it == pending_.end() || it->first != id) return;
        const uint64_t start = ReadClock();
        uint16_t mt = 0;
        bool arr = false;
        const uint8_t* data = nullptr;
        size_t count = 0;
        fc4_exrec_prop(pendingRecord_, it->second, nullptr, &mt, &arr, &data, &count, nullptr);
        cFC4PropertyHolder::FromCoreProperty(&own_, id, mt, arr, data, count);
        Mirror(id);
        pending_.erase(it);
        ++g_materialized;
        if (pending_.empty()) ClearPending();
        g_materializeTicks += ReadClock() - start;
    }
    void MaterializeAll() {
        if (!hasPending_.load(std::memory_order_acquire)) return;
        std::lock_guard<std::recursive_mutex> g(lock_);
        while (!pending_.empty()) Materialize(pending_.back().first);
    }

    // The parent's holder (cohort slot 9), borrowed; null without a parent.
    cFC4ISCPropertyHolder* ParentHolder() const {
        if (parent_ == nullptr) return nullptr;
        return reinterpret_cast<cFC4ISCPropertyHolder*>(parent_->AsISCPropertyHolder());
    }
    // §8.2 slot 15 rebuild(null): clear the effective list; with a parent,
    // take the parent holder's GetPropertyList and pop every property from
    // the front (slot 3 count, slot 12 PopFront(0x452160EC)), inserting by ID;
    // then the own list on top (own wins). Entries are the same property
    // objects (shared with the parent).
    void Rebuild() {
        std::lock_guard<std::recursive_mutex> g(lock_);
        MaterializeAll();
        ++g_count.rebuilds;
        eff_.Clear();
        if (cFC4ISCPropertyHolder* ph = ParentHolder()) {
            cIGZUnknownList* list = nullptr;
            if (ph->GetPropertyList(&list) && list != nullptr) {  // [abi-ret] probed: data-formats 8.2 / EX51
                FC4_ACQUIRED(list, "GetPropertyList");
                for (size_t n = list->GetSize(); n > 0; --n) {
                    void* p = nullptr;
                    if (!list->Pop_Front(kIIDProperty, &p) || p == nullptr) break;
                    auto* prop = static_cast<cISCProperty*>(p);
                    FC4_ACQUIRED(prop, "PopFront");
                    eff_.AddPropertyRecord(prop, false);  // the holder keeps its own reference
                    FC4_RELEASE(prop);
                }
                FC4_RELEASE_CREATED(list);  // made for us by GetPropertyList
            }
        }
        own_.EnumProperties(&AddTo, &eff_);
    }
    // §8.2 RemoveProperty / RemoveAllProperties, effective-list step for one
    // ID: the parent's property when the parent's holder has the ID (slot 3,
    // then slot 9), otherwise erased.
    void RevealOrErase(uint32_t id) {
        cFC4ISCPropertyHolder* ph = ParentHolder();
        cISCProperty* p = (ph != nullptr && ph->HasProperty(id)) ? ph->GetProperty(id) : nullptr;
        if (p != nullptr) eff_.AddPropertyRecord(p, false);
        else eff_.RemoveProperty(id, false);
    }
    bool RemoveOwn(uint32_t id, bool b) {
        {
            std::lock_guard<std::recursive_mutex> g(lock_);
            Materialize(id);
            if (!own_.RemoveProperty(id, false)) return false;  // nothing removed: false, no message
            RevealOrErase(id);
        }
        Post(b);
        return true;
    }
    // §8.2 slot 18 (EX51 c): no return value in the original (RET 4, EAX not
    // loaded); FreeCity4 returns true and nothing may test it. Posts only when
    // the bool is true AND the own list was non-empty before the call.
    bool RemoveAllOwn(bool b) {
        bool hadOwn = false;
        {
            std::lock_guard<std::recursive_mutex> g(lock_);
            hadOwn = own_.Count() != 0 || !pending_.empty();
            ClearPending();
            own_.RemoveAllProperties(false);
            std::vector<uint32_t> ids;
            eff_.EnumProperties([](cISCProperty* p, void* d) {
                static_cast<std::vector<uint32_t>*>(d)->push_back(p->GetPropertyID());
            }, &ids);
            for (uint32_t id : ids) RevealOrErase(id);
        }
        if (hadOwn) Post(b);
        else if (b) ++g_count.removeAllNoPost;
        return true;
    }
    // §8.2 slot 14 DebugInstanceTag(out) (EX51 e/f): the base resource tag
    // (shared stub 0x0093878E, which appends nothing: drawing-fonts §16
    // table), then " Exemplar " (exemplar) or " Exemplar Cohort " (cohort);
    // if ExemplarType 0x10 is found through the holder's GetProperty (slot 9,
    // the effective list) "(Other)" when the first 4 bytes of its value
    // storage are 0 (variant slot 73, no conversion) and "(Unknown)"
    // otherwise; if ExemplarName 0x20 is found, " '" + the converting
    // GetValString (variant slot 55, result not tested) + "'"; true.
    bool DebugTag(cIGZString& out) {
        ++g_count.tags;
        static const char kExemplar[] = " Exemplar ";
        static const char kCohort[] = " Exemplar Cohort ";
        if (type_ == kTypeCohort) out.Append(kCohort, sizeof kCohort - 1);
        else out.Append(kExemplar, sizeof kExemplar - 1);
        if (cISCProperty* p = holder_.GetProperty(0x10)) {
            cFC4IGZVariant* v = fc4::AsGame(p->GetPropertyValue());
            const uint32_t t = v != nullptr ? v->AsUint32Ref() : 0;
            const char* s = t == 0 ? "(Other)" : "(Unknown)";
            out.Append(s, static_cast<uint32_t>(strlen(s)));
        }
        if (cISCProperty* p = holder_.GetProperty(0x20)) {
            cRZBaseString name;
            if (cFC4IGZVariant* v = fc4::AsGame(p->GetPropertyValue())) v->GetValString(name);
            out.Append(" '", 2);
            out.Append(name.ToChar(), name.Strlen());
            out.Append("'", 1);
        }
        return true;
    }
    // §8.2 slot 15 rebuild(dest) (EX51 a). Null: the own effective list. A
    // non-null dest is the original's 12-byte vector {begin, end, capacity}
    // of 8-byte {uint32 id, cISCProperty*} elements, ascending ID, one
    // reference per element (the vector owns it): its elements are erased
    // (each reference released), then the parent's effective properties and
    // the own properties on top (own wins) are stored, each with one
    // reference; the object's own effective list is not touched. The original
    // has no return value (RET 4); FreeCity4 returns true, or false when the
    // dest buffer has to grow and the game's allocator is unavailable.
    bool RebuildInto(void* dest) {
        if (dest == nullptr) {
            Rebuild();
            return true;
        }
        ++g_count.rebuildDest;
        return FillVector(static_cast<NativeVec*>(dest));
    }
    struct NativeElem {
        uint32_t id;
        cISCProperty* prop;
    };
    struct NativeVec {
        NativeElem* begin;
        NativeElem* end;
        NativeElem* cap;  // [choice] end-of-storage pointer (question EX52)
    };
    bool FillVector(NativeVec* v) {
        // Collect first, in our heap: parent (one reference each from the
        // popped list reference), then own on top (AddRef new, release old).
        std::map<uint32_t, cISCProperty*> m;
        {
            std::lock_guard<std::recursive_mutex> g(lock_);
            MaterializeAll();
            if (cFC4ISCPropertyHolder* ph = ParentHolder()) {
                cIGZUnknownList* list = nullptr;
                if (ph->GetPropertyList(&list) && list != nullptr) {  // [abi-ret] probed: data-formats 8.2 / EX51
                    FC4_ACQUIRED(list, "GetPropertyList");
                    for (size_t n = list->GetSize(); n > 0; --n) {
                        void* p = nullptr;
                        if (!list->Pop_Front(kIIDProperty, &p) || p == nullptr) break;
                        auto* prop = static_cast<cISCProperty*>(p);
                        FC4_ACQUIRED(prop, "PopFront");  // kept by the vector element
                        auto it = m.find(prop->GetPropertyID());
                        if (it != m.end()) {
                            FC4_RELEASE(it->second);
                            it->second = prop;
                        } else {
                            m.emplace(prop->GetPropertyID(), prop);
                        }
                    }
                    FC4_RELEASE_CREATED(list);  // made for us by GetPropertyList
                }
            }
            own_.EnumProperties(
                [](cISCProperty* p, void* d) {
                    auto& mm = *static_cast<std::map<uint32_t, cISCProperty*>*>(d);
                    FC4_ADDREF(p);
                    auto it = mm.find(p->GetPropertyID());
                    if (it != mm.end()) {
                        FC4_RELEASE(it->second);
                        it->second = p;
                    } else {
                        mm.emplace(p->GetPropertyID(), p);
                    }
                },
                &m);
        }
        // Erase the destination's elements: EX51 a puts the release of each
        // element's reference (the vector is its holder) here.
        for (NativeElem* e = v->begin; e != nullptr && e != v->end; ++e) {
            if (e->prop != nullptr) FC4_RELEASE_LAST_HOLDER(e->prop);
        }
        v->end = v->begin;
        const size_t need = m.size();
        const size_t have = v->begin != nullptr ? static_cast<size_t>(v->cap - v->begin) : 0;
        if (need > have) {
            // [choice, question EX52] the game's std::vector with the default
            // allocator: the STL node allocator (docs/abi/allocators.md B).
            compat::StlNodeAllocateFn alloc = nullptr;
            compat::StlNodeFreeFn release = nullptr;
            if (!compat::StlNodeAllocator(&alloc, &release) || alloc == nullptr || release == nullptr) {
                ++g_count.rebuildDestFailed;
                for (auto& kv : m) FC4_RELEASE(kv.second);
                return false;
            }
            auto* buf = static_cast<NativeElem*>(alloc(static_cast<uint32_t>(need * sizeof(NativeElem))));
            if (buf == nullptr) {
                ++g_count.rebuildDestFailed;
                for (auto& kv : m) FC4_RELEASE(kv.second);
                return false;
            }
            if (v->begin != nullptr) release(v->begin);
            v->begin = buf;
            v->end = buf;
            v->cap = buf + need;
        }
        for (auto& kv : m) {  // ascending ID; one reference per element for the vector
            v->end->id = kv.first;
            v->end->prop = kv.second;
            FC4_ADDREF_FOR_CALLER(kv.second);
            FC4_RELEASE(kv.second);
            ++v->end;
        }
        return true;
    }
    static void AddTo(cISCProperty* p, void* holder) {
        static_cast<cFC4PropertyHolder*>(holder)->AddPropertyRecord(p, false);
    }
    // After an add to own_: the effective entry for `id` is the own object.
    void Mirror(uint32_t id) {
        if (cISCProperty* p = own_.GetProperty(id)) eff_.AddPropertyRecord(p, false);
    }
    // §8.2 change message, when the bool is true: a standard message
    // (foundations §5.1) of type 0xC53D10AA (exemplar) / 0x853D11F1 (cohort),
    // data1 / data2 = the group / instance words (+4 / +8) of the object's
    // own key, SetExtra(the object, an added reference), MessagePost(msg,
    // false) on the message server, then our reference released.
    void Post(bool b) {
        if (!b) return;
        ++g_count.msgAdds;
        cGZPersistResourceKey k;
        GetKey(k);
        cIGZMessageServer2Ptr ms;
        if (!ms) {
            ++g_count.msgFailed;
            return;
        }
        cFC4Message2Standard* msg =
            FC4MakeMessage(type_ == kTypeCohort ? kMsgCohortChanged : kMsgExemplarChanged,
                           static_cast<intptr_t>(k.group), static_cast<intptr_t>(k.instance), 0);
        if (msg == nullptr) {
            ++g_count.msgFailed;  // §8.2: no message, nothing posted
            return;
        }
        msg->SetExtra(Serializable());
        ms->MessagePost(msg, false);
        ++g_count.msgPosted;
        msg->Release();  // FC4MakeMessage's reference (ours)
    }
    virtual cIGZSerializable* Serializable() = 0;
    // §8.2 SetCohort / SetParent(parent, bool): the same parent is a no-op (no
    // rebuild, no message); otherwise AddRef the new (if any), release the
    // old, store, rebuild (slot 15 with null), post when the bool is true.
    bool SetParent(cISCResExemplarCohort* c, bool b) {
        {
            std::lock_guard<std::recursive_mutex> g(lock_);
            // EX51 c: SetCohort has no return value in the original (RET 8, EAX
            // not loaded) in either path; FreeCity4 returns true, nothing tests it.
            if (c == parent_) return true;
            if (c != nullptr) FC4_ADDREF(c);
            if (parent_ != nullptr) FC4_RELEASE(parent_);
            parent_ = c;
            Rebuild();
        }
        Post(b);
        return true;
    }
    bool GetKey(cGZPersistResourceKey& k) const {
        std::lock_guard<std::recursive_mutex> g(lock_);
        k = key_;
        return true;
    }
    bool SetKey(cGZPersistResourceKey const& k) {
        std::lock_guard<std::recursive_mutex> g(lock_);
        key_ = k;
        return true;
    }
    bool CoreQI(uint32_t riid, void** ppv, void* face, uint32_t faceIID, cIGZPersistResource* res) {
        if (ppv == nullptr) return false;
        void* out = nullptr;
        if (riid == faceIID || riid == kIIDUnknown) out = face;
        else if (riid == kIIDHolder) out = static_cast<cFC4ISCPropertyHolder*>(&holder_);
        else if (riid == kIIDResource) out = res;
        else if (riid == kIIDFC4Core) out = this;
        if (out == nullptr) {
            *ppv = nullptr;
            return false;
        }
        ++refs_;
        *ppv = out;
        return true;
    }
    uint32_t CoreRelease() {
        const uint32_t n = --refs_;
        if (n == 0) delete this;
        return n;
    }

    // §4.1/§4.2/§4.6 and §8.1 SerialRead: the record at position 0, already
    // decompressed. Reads the whole record, parses it in the core, loads the
    // parent (cohort IID, failure fails the read) and kind-1 resources.
    bool ReadRecord(cIGZPersistDBRecord& record);
    // §8.2 SerialWrite binary form (EX51 d), on the record's spec slots.
    bool WriteBinary(cIGZPersistDBSerialRecord* record);
};

namespace serial {
// The serial record's write slots as data-formats §8.2 EX51 d numbers them
// (thiscall, callee pops). [choice, question EX52] slot 28's arguments are
// taken as (const void* bytes, uint32_t count), the SDK's raw-write shape.
using RawWriteFn = bool(__thiscall*)(void*, const void*, uint32_t);
using U8Fn = bool(__thiscall*)(void*, uint8_t);
using U32Fn = bool(__thiscall*)(void*, uint32_t);
using KeyFn = bool(__thiscall*)(void*, const cGZPersistResourceKey*);
using VariantFn = bool(__thiscall*)(void*, const cIGZVariant*);
template <class F>
F At(void* obj, int slot) {
    return reinterpret_cast<F>((*reinterpret_cast<void***>(obj))[slot]);
}
}  // namespace serial

bool ExCore::WriteBinary(cIGZPersistDBSerialRecord* record) {
    void* r = record;
    static const char kExSig[8] = {'E', 'Q', 'Z', 'B', '1', '#', '#', '#'};
    static const char kCoSig[8] = {'C', 'Q', 'Z', 'B', '1', '#', '#', '#'};
    std::vector<cISCProperty*> props;
    cGZPersistResourceKey pk(0, 0, 0);
    {
        std::lock_guard<std::recursive_mutex> g(lock_);
        MaterializeAll();
        if (parent_ != nullptr) {
            void* pr = nullptr;
            if (parent_->QueryInterface(kIIDResource, &pr) && pr != nullptr) {
                auto* res = static_cast<cIGZPersistResource*>(pr);
                FC4_ACQUIRED(res, "QueryInterface");
                res->GetKey(pk);
                FC4_RELEASE(res);
            }
        }
        own_.EnumProperties(
            [](cISCProperty* p, void* d) {
                FC4_ADDREF(p);
                static_cast<std::vector<cISCProperty*>*>(d)->push_back(p);
            },
            &props);
    }
    std::sort(props.begin(), props.end(),
              [](cISCProperty* a, cISCProperty* b) { return a->GetPropertyID() < b->GetPropertyID(); });
    bool ok = serial::At<serial::RawWriteFn>(r, 28)(r, type_ == kTypeCohort ? kCoSig : kExSig, 8) &&
              serial::At<serial::KeyFn>(r, 49)(r, &pk) &&
              serial::At<serial::U32Fn>(r, 37)(r, static_cast<uint32_t>(props.size()));
    for (cISCProperty* p : props) {
        if (!ok) break;
        ok = serial::At<serial::U32Fn>(r, 37)(r, p->GetPropertyID());
        if (!ok) break;
        // Kind 1: the property answers 0xE52184AB. Our own properties hold a
        // loaded resource as an IGZUnknown value (§4.6 read path) and never
        // answer it; [choice] one whose value answers cIGZPersistResource is
        // written as kind 1 with that resource's key, so a read round-trips.
        cGZPersistResourceKey rk;
        bool kind1 = false;
        void* q = nullptr;
        if (p->QueryInterface(kIIDResourceProperty, &q) && q != nullptr) {
            // [choice, question EX52] the spec does not say how the key is read
            // from a 0xE52184AB property: a game-made one fails the write.
            auto* u = static_cast<cIGZUnknown*>(q);
            FC4_ACQUIRED(u, "QueryInterface");
            FC4_RELEASE(u);
            ok = false;
            break;
        }
        if (cFC4IGZVariant* v = fc4::AsGame(p->GetPropertyValue())) {
            void* res = nullptr;
            // Only an IGZUnknown value (type 0x10) holds an object pointer.
            if (v->GetType() == cIGZVariant::IGZUnknown && v->GetValUnknownAs(kIIDResource, &res) && res != nullptr) {  // [abi-ret] reviewed: our own properties carry cFC4Variant; slot 60 returns bool (foundations 17.1)
                auto* pr = static_cast<cIGZPersistResource*>(res);
                FC4_ACQUIRED(pr, "GetValUnknownAs");
                pr->GetKey(rk);
                FC4_RELEASE(pr);
                kind1 = true;
            }
        }
        if (kind1) {
            ok = serial::At<serial::U8Fn>(r, 29)(r, 1) && serial::At<serial::KeyFn>(r, 49)(r, &rk);
        } else {
            ok = serial::At<serial::U8Fn>(r, 29)(r, 0) &&
                 serial::At<serial::VariantFn>(r, 50)(r, p->GetPropertyValue());
        }
    }
    for (cISCProperty* p : props) FC4_RELEASE(p);
    return ok;
}

bool ExHolder::QueryInterface(uint32_t riid, void** ppvObj) { return o_->Outer()->QueryInterface(riid, ppvObj); }
uint32_t ExHolder::AddRef() { return o_->Outer()->AddRef(); }
uint32_t ExHolder::Release() { return o_->Outer()->Release(); }
bool ExHolder::HasProperty(uint32_t id) const {
    o_->Materialize(id);
    return o_->eff_.HasProperty(id);
}
bool ExHolder::GetPropertyData(uint32_t id, void* buffer, uint32_t& length) const {
    o_->Materialize(id);
    return o_->eff_.GetPropertyData(id, buffer, length);
}
bool ExHolder::GetPropertyInterface(uint32_t id, uint32_t riid, void** ppv) const {
    o_->Materialize(id);
    return o_->eff_.GetPropertyInterface(id, riid, ppv);
}
bool ExHolder::GetPropertyString(uint32_t id, cIGZString& value) const {
    o_->Materialize(id);
    return o_->eff_.GetPropertyString(id, value);
}
bool ExHolder::GetPropertyU32(uint32_t id, uint32_t& value) const {
    o_->Materialize(id);
    return o_->eff_.GetPropertyU32(id, value);
}
cISCProperty* ExHolder::GetProperty(uint32_t id) const {
    o_->Materialize(id);
    return o_->eff_.GetProperty(id);
}
// §8.2 holder add slots (EX51 b). Slots 10-13 are the base bodies: they ignore
// their bool, never post, and touch the OWN list only: an existing own entry
// gets its value set in place (the effective list holds the same object, so
// the value shows at once); a new property goes into the own list only and
// stays invisible through slots 3/9 until the next rebuild. Slots 14 and 16
// build a new property and go through slot 15 (effective list updated, posts
// when the bool is true).
bool ExHolder::AddPropertyData(uint32_t id, void* data, uint32_t length, bool) {
    std::lock_guard<std::recursive_mutex> g(o_->lock_);
    o_->Materialize(id);
    return o_->own_.AddPropertyData(id, data, length, false);
}
bool ExHolder::AddPropertyObject(uint32_t id, cIGZUnknown* value, bool) {
    std::lock_guard<std::recursive_mutex> g(o_->lock_);
    o_->Materialize(id);
    return o_->own_.AddPropertyObject(id, value, false);
}
bool ExHolder::AddPropertyString(uint32_t id, cIGZString const& value, bool) {
    std::lock_guard<std::recursive_mutex> g(o_->lock_);
    o_->Materialize(id);
    return o_->own_.AddPropertyString(id, value, false);
}
bool ExHolder::AddPropertyU32(uint32_t id, uint32_t value, bool) {
    std::lock_guard<std::recursive_mutex> g(o_->lock_);
    o_->Materialize(id);
    return o_->own_.AddPropertyU32(id, value, false);
}
// Slot 14: a new property with the ID, its value set from the variant
// (property slot 7), then slot 15 virtually with the bool. [choice] a null
// variant leaves the new property Empty (the spec does not cover null).
bool ExHolder::AddPropertyVariant(uint32_t id, cFC4IGZVariant const* value, bool b) {
    auto* p = new cFC4Property(id);
    FC4_ADDREF(p);
    if (value != nullptr) p->Value().CopyFromGame(*value);
    const bool ok = AddPropertyRecord(p, b);
    FC4_RELEASE(p);
    return ok;
}
bool ExHolder::AddPropertyRecord(cISCProperty* property, bool b) {
    if (property == nullptr) return false;
    bool ok;
    {
        std::lock_guard<std::recursive_mutex> g(o_->lock_);
        o_->Materialize(property->GetPropertyID());
        ok = o_->own_.AddPropertyRecord(property, b);
        o_->Mirror(property->GetPropertyID());
    }
    o_->Post(b);  // §8.2: slot 15 posts the change message
    return ok;
}
// Slot 16: a new property with the source's ID and a copy of its value, then
// slot 15 virtually with the bool.
bool ExHolder::CopyAddProperty(cISCProperty* property, bool b) {
    if (property == nullptr) return false;
    auto* p = new cFC4Property(property->GetPropertyID());
    FC4_ADDREF(p);
    if (const cIGZVariant* v = property->GetPropertyValue()) p->Value().CopyFromGame(*fc4::AsGame(v));
    const bool ok = AddPropertyRecord(p, b);
    FC4_RELEASE(p);
    return ok;
}
// §8.2 (EX47): removal edits the own list, then the effective entry shows the
// parent's property again or is erased; false (no message) if nothing was
// removed; the change message when the bool is true.
bool ExHolder::RemoveProperty(uint32_t id, bool b) { return o_->RemoveOwn(id, b); }
bool ExHolder::RemoveAllProperties(bool b) { return o_->RemoveAllOwn(b); }
bool ExHolder::EnumPropertiesFiltered(FilterFn filter, EnumFn fn, void* data) const {
    o_->MaterializeAll();
    return o_->own_.EnumPropertiesFiltered(filter, fn, data);
}
bool ExHolder::EnumProperties(EnumFn fn, void* data) const {
    o_->MaterializeAll();
    return o_->own_.EnumProperties(fn, data);
}

// §8.2 slot 22: the deleting destructor of the whole object. [choice] only
// the delete bit (1) is honoured; the object goes regardless of its count,
// as the original's destructor would.
void* ExHolder::DeletingDestructor(uint32_t flags) {
    if ((flags & 1) != 0) delete o_;
    return this;
}

// §4.4 / §8.2 (EX49): a new list, one reference for the caller, holding every
// effective property, built by inserting each at the FRONT in ascending ID
// order, so the list reads in descending ID order. False only for a null
// out-pointer. EX50: the caller's return address is recorded.
bool ExHolder::GetPropertyList(cIGZUnknownList** list) const {
    const uint32_t call = ++g_count.lists;
    EntryScope scope(kEnGetList, _ReturnAddress());
    NoteCaller(_ReturnAddress(), call);
    if (list == nullptr) {
        ++g_count.listFails;
        return false;
    }
    std::vector<cISCProperty*> props;
    {
        std::lock_guard<std::recursive_mutex> g(o_->lock_);
        o_->MaterializeAll();
        o_->eff_.EnumProperties([](cISCProperty* p, void* d) { static_cast<std::vector<cISCProperty*>*>(d)->push_back(p); },
                                &props);
        // The holder enumerates its map in ascending unsigned ID order.
        auto* l = new cFC4UnknownList();
        for (auto* p : props) l->Push_Front(p);
        l->AddRef();
        *list = l;
    }
    return true;
}

struct LoadingScope {
    explicit LoadingScope(const cGZPersistResourceKey& k) { t_loading.push_back(k); }
    ~LoadingScope() { t_loading.pop_back(); }
};

bool ExCore::ReadRecord(cIGZPersistDBRecord& record) {
    ++g_count.reads;
    ReadTimer timer;
    cGZPersistResourceKey rkey;
    record.GetKey(rkey);
    // §4.1: the type the record is stored under must be ours (the core
    // parse_record checks it with the signature).
    const uint32_t size = record.GetSize();
    std::vector<uint8_t> bytes(size);
    if (size > 0 && !record.GetFieldVoid(bytes.data(), size)) return false;
    timer.Stage(1);
    fc4_exrec* rec = fc4_exrec_parse(rkey.type, bytes.data(), bytes.size());
    timer.Stage(2);
    if (rec == nullptr) return false;
    struct Free {
        fc4_exrec* r;
        ~Free() { fc4_exrec_destroy(r); }
    } free_{rec};
    LoadingScope scope(rkey);
    uint32_t pk[3] = {0, 0, 0};
    fc4_exrec_parent(rec, pk);
    cIGZPersistResourceManagerPtr rm;
    cISCResExemplarCohort* parent = nullptr;
    if (pk[0] != 0 || pk[1] != 0 || pk[2] != 0) {
        const cGZPersistResourceKey parentKey(pk[0], pk[1], pk[2]);
        for (const auto& k : t_loading) {
            if (SameKey(k, parentKey)) {
                ++g_count.cycleFails;
                return false;  // §4.4 [choice]: first repeated key
            }
        }
        if (t_loading.size() > kMaxDepth) {
            ++g_count.cycleFails;
            return false;  // §4.4 [choice]: depth 32
        }
        void* out = nullptr;
        if (!rm || !rm->GetResource(parentKey, kIIDCohort, &out, 0, nullptr) || out == nullptr) {
            ++g_count.parentFails;
            return false;  // §4.2: the whole read fails
        }
        parent = static_cast<cISCResExemplarCohort*>(out);
        FC4_ACQUIRED(parent, "GetResource");
    }
    {
        std::lock_guard<std::recursive_mutex> g(lock_);
        ClearPending();
        if (parent_ != nullptr) FC4_RELEASE(parent_);
        parent_ = parent;  // our reference from GetResource
        own_.Clear();
        Rebuild();
    }
    // §4.2: properties in record order; a later one with the same ID
    // replaces the earlier (§4.5); each add updates the effective list.
    timer.Stage(3);
    std::lock_guard<std::recursive_mutex> g(lock_);
    pendingRecord_ = rec;
    free_.r = nullptr;
    const size_t n = fc4_exrec_count(rec);
    pending_.reserve(n);
    for (size_t i = 0; i < n; ++i) {
        uint32_t id = 0, key3[3] = {0, 0, 0};
        const int32_t kind = fc4_exrec_prop(rec, i, &id, nullptr, nullptr, nullptr, nullptr, key3);
        if (kind == 0) {
            pending_.emplace_back(id, i);
            ++g_deferred;
            continue;
        } else if (kind == 1) {
            // §4.2: Empty is added like any other (memory type 0, no data).
            auto* p = new cFC4Property(id);
            own_.AddPropertyRecord(p, false);  // the holder takes its reference
            ++g_count.emptyProps;
        } else if (kind == 2) {
            // §4.6: the key is loaded through the resource manager as
            // cIGZPersistResource; failure fails the read.
            void* out = nullptr;
            if (!rm || !rm->GetResource(cGZPersistResourceKey(key3[0], key3[1], key3[2]), kIIDResource, &out, 0,
                                         nullptr) ||
                out == nullptr) {
                return false;
            }
            auto* r = static_cast<cIGZUnknown*>(out);
            FC4_ACQUIRED(r, "GetResource");
            own_.AddPropertyObject(id, r, false);  // the variant keeps its own reference
            FC4_RELEASE(r);
            ++g_count.resourceProps;
        } else {
            return false;
        }
        Mirror(id);
    }
    if (pending_.empty()) ClearPending();
    else hasPending_.store(true, std::memory_order_release);
    return true;
}

// cSCResExemplar replacement: cISCResExemplar (16 slots, foundations §17.5),
// the holder, cIGZPersistResource.
class cFC4ResExemplar final : public cISCResExemplar, public cIGZPersistResource, public ExCore {
public:
    cFC4ResExemplar() : ExCore(kTypeExemplar, static_cast<cISCResExemplar*>(this)) {}
    cIGZUnknown* Outer() override { return static_cast<cISCResExemplar*>(this); }
    bool QueryInterface(uint32_t riid, void** ppv) override {
        return CoreQI(riid, ppv, static_cast<cISCResExemplar*>(this), kIIDExemplar,
                      static_cast<cIGZPersistResource*>(this));
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override { return CoreRelease(); }
    // §8.2 (EX46) cIGZSerializable slots 3-5: Write / Read return false and
    // touch nothing; GetGZCLSID returns 0. Counted only.
    bool Write(cIGZOStream&) override {
        ++g_count.serialStubs;
        return false;
    }
    bool Read(cIGZIStream&) override {
        ++g_count.serialStubs;
        return false;
    }
    uint32_t GetGZCLSID() override {
        ++g_count.serialStubs;
        return 0;
    }
    cIGZSerializable* Serializable() override { return static_cast<cISCResExemplar*>(this); }
    cISCPropertyHolder* AsISCPropertyHolder() override { return HolderSdk(); }
    cISCPropertyHolder* AsISCPropertyHolder() const override { return const_cast<cFC4ResExemplar*>(this)->HolderSdk(); }
    bool IsPropertyLocal(uint32_t id) override { Materialize(id); return own_.HasProperty(id); }  // §4.4: own list only
    // [choice, EX48] borrowed: the reference stays with the exemplar.
    cISCResExemplarCohort* GetCohort() override {
        std::lock_guard<std::recursive_mutex> g(lock_);
        return parent_;
    }
    bool SetCohort(cISCResExemplarCohort* c, bool b) override {
        EntryScope scope(kEnSetCohort, _ReturnAddress());
        return SetParent(c, b);
    }
    bool GetKey(cGZPersistResourceKey& k) override { return ExCore::GetKey(k); }
    bool SetKey(cGZPersistResourceKey const& k) override { return ExCore::SetKey(k); }
    bool CompactProperties() override { return true; }
    // §8.2 (EX47): slot 14 DebugInstanceTag(out), slot 15 rebuild(dest).
    virtual bool DebugInstanceTag(cIGZString& out) {
        EntryScope scope(kEnTag, _ReturnAddress());
        return DebugTag(out);
    }
    virtual bool RebuildEffective(void* dest) {
        EntryScope scope(kEnRebuild, _ReturnAddress());
        return RebuildInto(dest);
    }
    // cIGZPersistResource
    void GetKey(cGZPersistResourceKey& k) const override { ExCore::GetKey(k); }
    uint32_t GetType() const override { return Key().type; }
    uint32_t GetGroup() const override { return Key().group; }
    uint32_t GetInstance() const override { return Key().instance; }
    bool SetType(uint32_t v) override { return SetPart(0, v); }
    bool SetGroup(uint32_t v) override { return SetPart(1, v); }
    bool SetInstance(uint32_t v) override { return SetPart(2, v); }
    uint32_t GetRefCount() const override { return refs_; }

private:
    cISCPropertyHolder* HolderSdk() {
        return reinterpret_cast<cISCPropertyHolder*>(static_cast<cFC4ISCPropertyHolder*>(&holder_));
    }
    cGZPersistResourceKey Key() const {
        cGZPersistResourceKey k;
        ExCore::GetKey(k);
        return k;
    }
    bool SetPart(int i, uint32_t v) {
        std::lock_guard<std::recursive_mutex> g(lock_);
        (i == 0 ? key_.type : i == 1 ? key_.group : key_.instance) = v;
        return true;
    }
};

// cSCResExemplarCohort replacement: the cohort interface (IID 0x452160FB,
// 16 slots; §8 table: slot 7 SetCohort, slot 9 the holder, slot 15 rebuild).
class cFC4ResCohort final : public cISCResExemplarCohort, public cIGZPersistResource, public ExCore {
public:
    cFC4ResCohort() : ExCore(kTypeCohort, static_cast<cISCResExemplarCohort*>(this)) {}
    cIGZUnknown* Outer() override { return static_cast<cISCResExemplarCohort*>(this); }
    bool QueryInterface(uint32_t riid, void** ppv) override {
        return CoreQI(riid, ppv, static_cast<cISCResExemplarCohort*>(this), kIIDCohort,
                      static_cast<cIGZPersistResource*>(this));
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override { return CoreRelease(); }
    // §8.2 (EX46): as the exemplar's.
    bool Write(cIGZOStream&) override {
        ++g_count.serialStubs;
        return false;
    }
    bool Read(cIGZIStream&) override {
        ++g_count.serialStubs;
        return false;
    }
    uint32_t GetGZCLSID() override {
        ++g_count.serialStubs;
        return 0;
    }
    cIGZSerializable* Serializable() override { return static_cast<cISCResExemplarCohort*>(this); }
    // [choice, EX48] borrowed, as GetCohort.
    cISCResExemplarCohort* GetParent() override {
        std::lock_guard<std::recursive_mutex> g(lock_);
        return parent_;
    }
    bool SetParent(cISCResExemplarCohort* c, bool b) override {
        EntryScope scope(kEnSetParent, _ReturnAddress());
        return ExCore::SetParent(c, b);
    }
    cISCPropertyHolder* AsISCPropertyHolder() override { return HolderSdk(); }
    cISCPropertyHolder* AsISCPropertyHolder() const override { return const_cast<cFC4ResCohort*>(this)->HolderSdk(); }
    bool IsPropertyLocal(uint32_t id) override { Materialize(id); return own_.HasProperty(id); }
    bool GetKey(cGZPersistResourceKey& k) override { return ExCore::GetKey(k); }
    bool SetKey(cGZPersistResourceKey const& k) override { return ExCore::SetKey(k); }
    bool CompactProperties() override { return true; }
    virtual bool DebugInstanceTag(cIGZString& out) {  // §8.2, as the exemplar's
        EntryScope scope(kEnTag, _ReturnAddress());
        return DebugTag(out);
    }
    virtual bool RebuildEffective(void* dest) {
        EntryScope scope(kEnRebuild, _ReturnAddress());
        return RebuildInto(dest);
    }
    void GetKey(cGZPersistResourceKey& k) const override { ExCore::GetKey(k); }
    uint32_t GetType() const override { return Key().type; }
    uint32_t GetGroup() const override { return Key().group; }
    uint32_t GetInstance() const override { return Key().instance; }
    bool SetType(uint32_t v) override { return SetPart(0, v); }
    bool SetGroup(uint32_t v) override { return SetPart(1, v); }
    bool SetInstance(uint32_t v) override { return SetPart(2, v); }
    uint32_t GetRefCount() const override { return refs_; }

private:
    cISCPropertyHolder* HolderSdk() {
        return reinterpret_cast<cISCPropertyHolder*>(static_cast<cFC4ISCPropertyHolder*>(&holder_));
    }
    cGZPersistResourceKey Key() const {
        cGZPersistResourceKey k;
        ExCore::GetKey(k);
        return k;
    }
    bool SetPart(int i, uint32_t v) {
        std::lock_guard<std::recursive_mutex> g(lock_);
        (i == 0 ? key_.type : i == 1 ? key_.group : key_.instance) = v;
        return true;
    }
};

// The factory (§8.1): slots 3-6 from the SDK interface (compiled slot 3 is
// the record path), then slots 7-12 with the argument counts of docs/abi
// (vtable 0x00AAEEF8) so the table has the original's shape.
class cFC4ExemplarFactory final : public cIGZPersistResourceFactory {
public:
    explicit cFC4ExemplarFactory(uint32_t type) : type_(type) {}
    bool QueryInterface(uint32_t riid, void** ppv) override {
        if (ppv == nullptr) return false;
        if (riid == kIIDFactory || riid == kIIDUnknown) {
            AddRef();
            *ppv = static_cast<cIGZPersistResourceFactory*>(this);
            return true;
        }
        *ppv = nullptr;
        return false;
    }
    uint32_t AddRef() override { return ++refs_; }
    uint32_t Release() override {
        const uint32_t n = --refs_;
        if (n == 0) delete this;
        return n;
    }
    // Slot 4 (type path): the trailing values are ignored (§8.1, base slot 4).
    bool CreateInstance(uint32_t type, uint32_t riid, void** ppv, uint32_t, cIGZUnknown*) override {
        return CreateInstance3(type, riid, ppv);
    }
    // Slot 3 (record path, §8.1 step 2): create, give the key, read, then
    // QueryInterface for the caller; our own reference released in every
    // case. The manager closes or aborts the record.
    bool CreateInstance(cIGZPersistDBRecord& record, uint32_t riid, void** ppv, uint32_t, cIGZUnknown*) override {
        EntryScope scope(kEnCreateRec, _ReturnAddress());
        std::lock_guard<std::recursive_mutex> g(lock_);
        if (ppv == nullptr) return false;
        *ppv = nullptr;
        cGZPersistResourceKey key;
        record.GetKey(key);
        ExCore* core = NewObject(key.type);
        if (core == nullptr) return false;
        core->SetKey(key);
        bool ok = core->ReadRecord(record);
        if (ok) ok = core->Outer()->QueryInterface(riid, ppv);
        if (!ok) ++g_count.readFails;
        core->CoreRelease();
        return ok;
    }
    // Slot 5 Read(resource, record) (§8.1: through the serial record to
    // SerialRead).
    bool Read(cIGZPersistResource& resource, cIGZPersistDBRecord& record) override {
        EntryScope scope(kEnFRead, _ReturnAddress());
        void* c = nullptr;
        if (!resource.QueryInterface(kIIDFC4Core, &c) || c == nullptr) return false;
        auto* core = static_cast<ExCore*>(c);
        const bool ok = core->type_ == type_ && core->ReadRecord(record);
        core->CoreRelease();
        return ok;
    }
    // Slot 6 Write(resource, record) (§8.2): QueryInterface of the record for
    // the serial record (0xC56B8F08), then SerialWrite.
    bool Write(cIGZPersistResource const& resource, cIGZPersistDBRecord& record) override {
        EntryScope scope(kEnFWrite, _ReturnAddress());
        void* s = nullptr;
        if (!record.QueryInterface(kIIDSerialRecord, &s) || s == nullptr) return false;
        auto* sr = static_cast<cIGZPersistDBSerialRecord*>(s);
        FC4_ACQUIRED(sr, "QueryInterface");
        const bool ok = SerialWrite(const_cast<cIGZPersistResource*>(&resource), sr);
        FC4_RELEASE(sr);
        return ok;
    }
    // Slot 7: the original's destructor slot. Our lifetime is the reference
    // count; nothing is deleted here [choice].
    virtual void* Slot7Destroy(uint32_t) { return this; }
    // §8.2 (EX46) slot 8 (record, iid, out): slot 3 with the trailing 0, 0.
    virtual bool CreateFromRecord(cIGZPersistDBRecord& record, uint32_t riid, void** ppv) {
        ++g_count.slot8;
        EntryScope scope(kEnCreateRec, _ReturnAddress());  // the inner call below is nested
        return CreateInstance(record, riid, ppv, 0, nullptr);
    }
    // Slot 9 CreateInstance(type, IID, out) (§8.1).
    virtual bool CreateInstance3(uint32_t type, uint32_t riid, void** ppv) {
        if (ppv == nullptr) return false;
        *ppv = nullptr;
        if (type != type_) return false;
        ExCore* core = NewObject(type);
        const bool ok = core != nullptr && core->Outer()->QueryInterface(riid, ppv);
        if (core != nullptr) core->CoreRelease();  // unknown IID: destroyed, null, false
        return ok;
    }
    // Slot 10 SerialRead(resource, serial record).
    virtual bool SerialRead(cIGZPersistResource* resource, cIGZPersistDBSerialRecord* record) {
        if (resource == nullptr || record == nullptr) return false;
        cIGZPersistDBRecord* base = record->AsIGZPersistDBRecord();
        return base != nullptr && Read(*resource, *base);
    }
    // Slot 11 SerialWrite(resource, serial record) (§8.2, EX51 d): false
    // unless the resource's type (slot 5) is ours. Binary form (write mode
    // false): signature EQZB1### / CQZB1### (record slot 28, raw 8 bytes), the
    // parent's key (slot 49; all zero without a parent), the own-property
    // count (slot 37), then per own property in ascending ID: its ID (slot
    // 37), kind 0 + the variant (slots 29, 50) or kind 1 + the held
    // resource's key (slots 29, 49). Stops at the first failed write and
    // returns the last write's result. The SDK header of the serial record
    // lacks the raw write at slot 28 (its slots from 28 on are one lower), so
    // the record is called by the spec's slot numbers.
    virtual bool SerialWrite(cIGZPersistResource* resource, cIGZPersistDBSerialRecord* record) {
        EntryScope scope(kEnSerialWrite, _ReturnAddress());
        ++g_count.serialWrites;
        if (resource == nullptr || record == nullptr || resource->GetType() != type_) return false;
        if (writeMode_ != 0) {
            // [choice] the text form needs GetPropertyReadableName (0x0067FE80),
            // whose name table the spec does not give: refused.
            ++g_count.serialWriteText;
            Unsupported("factory SerialWrite text form");
            return false;
        }
        void* c = nullptr;
        if (!resource->QueryInterface(kIIDFC4Core, &c) || c == nullptr) return false;
        auto* core = static_cast<ExCore*>(c);
        const bool ok = core->WriteBinary(record);
        core->CoreRelease();
        if (!ok) ++g_count.serialWriteFails;
        return ok;
    }
    virtual void SetWriteMode(uint32_t v) { writeMode_ = v; }  // slot 12: no reader uses it [choice]

private:
    ExCore* NewObject(uint32_t type) {
        if (type != type_) return nullptr;
        ExCore* core = type == kTypeCohort ? static_cast<ExCore*>(new cFC4ResCohort())
                                           : static_cast<ExCore*>(new cFC4ResExemplar());
        ++core->refs_;
        ++g_count.created;
        return core;
    }
    uint32_t type_;
    std::atomic<uint32_t> refs_{0};
    std::recursive_mutex lock_;
    uint32_t writeMode_ = 0;
};

// §8.1: unregister (slot 20) takes a class ID; the SDK declares a factory
// pointer there (same size on x86), so the class ID is passed in it.
bool UnregisterClass(cIGZPersistResourceManager* rm, uint32_t cls) {
    // [abi-ret] §8.1 slot 20: true if at least one entry was removed.
    return rm->UnregisterObjectFactory(reinterpret_cast<cIGZPersistResourceFactory*>(static_cast<uintptr_t>(cls)));
}

bool TakeOver(cIGZPersistResourceManager* rm, int i, uint32_t type, uint32_t cls) {
    cIGZPersistResourceFactory* orig = nullptr;
    // The (type, out) overload is slot 21 (MSVC places overloads in reverse
    // declaration order), which docs/abi gives as 8 argument bytes like the
    // SDK. Taken as a member pointer so the abi_conformance name scan does not
    // read it as the (out) overload (slot 22, SDK 4 bytes, game pops 8), which
    // the plugin never calls.
    constexpr auto kFindByType = static_cast<bool (cIGZPersistResourceManager::*)(
        uint32_t, cIGZPersistResourceFactory**)>(&cIGZPersistResourceManager::FindObjectFactory);
    // [abi-ret] §8.1 slot 21: true with the factory and one added reference.
    if (!(rm->*kFindByType)(type, &orig) || orig == nullptr) {  // slot 21: one reference for us
        Log("exemplars: no factory registered for type 0x%08X; not replaced", type);
        return false;
    }
    FC4_ACQUIRED(orig, "FindObjectFactory");
    auto* ours = new cFC4ExemplarFactory(type);
    FC4_ADDREF(ours);
    const bool removed = UnregisterClass(rm, cls);
    // [abi-ret] §8.1 slot 19: returns true when registered.
    const bool added = rm->RegisterObjectFactory(cls, type, ours);  // the manager AddRefs
    if (!added) {
        if (removed) rm->RegisterObjectFactory(cls, type, orig);
        FC4_RELEASE(ours);
        FC4_RELEASE(orig);
        Log("exemplars: register failed for type 0x%08X; original kept", type);
        return false;
    }
    g_original[i] = orig;
    g_ours[i] = ours;
    Log("exemplars: type 0x%08X class 0x%08X now ours (unregistered original: %s)", type, cls,
        removed ? "yes" : "no");
    return true;
}

void Restore(cIGZPersistResourceManager* rm, int i, uint32_t type, uint32_t cls) {
    if (g_ours[i] == nullptr) return;
    const bool removed = UnregisterClass(rm, cls);
    // [abi-ret] §8.1 slot 19: returns true when registered.
    const bool added = g_original[i] != nullptr && rm->RegisterObjectFactory(cls, type, g_original[i]);
    Log("exemplars: type 0x%08X restored (ours unregistered: %s, original registered: %s)", type,
        removed ? "yes" : "no", added ? "yes" : "no");
    FC4_RELEASE(g_ours[i]);
    g_ours[i] = nullptr;
    if (g_original[i] != nullptr) FC4_RELEASE(g_original[i]);
    g_original[i] = nullptr;
}

// ---- native original versus ours (data-formats §10.8 1-4) ----------------
// Run once at install when both types were taken over: the same scenario on
// objects made by the game's factories (kept since the takeover, §8.1) and by
// ours; every step's result becomes text and each scenario logs MATCH or DIFF
// with both texts. Only bool false is passed (no message is posted).

using Holder = cFC4ISCPropertyHolder;
Holder* HolderOf(cISCResExemplar* e) { return reinterpret_cast<Holder*>(e->AsISCPropertyHolder()); }
Holder* HolderOf(cISCResExemplarCohort* c) { return reinterpret_cast<Holder*>(c->AsISCPropertyHolder()); }

template <class T>
T* Make(cIGZPersistResourceFactory* f, uint32_t type, uint32_t iid) {
    void* out = nullptr;
    if (f == nullptr || !f->CreateInstance(type, iid, &out, 0, nullptr) || out == nullptr) return nullptr;
    FC4_ACQUIRED(static_cast<T*>(out), "CreateInstance");
    return static_cast<T*>(out);
}

// A balanced AddRef / Release probe: the count AddRef reports, minus one.
uint32_t RefsOf(cIGZUnknown* u) {
    const uint32_t n = u->AddRef();
    u->Release();
    return n - 1;
}

void Add(std::string& s, const char* fmt, unsigned long v) {
    char b[64];
    snprintf(b, sizeof b, fmt, v);
    s += b;
}

std::string ListIds(Holder* h) {
    cIGZUnknownList* l = nullptr;
    if (!h->GetPropertyList(&l) || l == nullptr) return "nolist";  // [abi-ret] probed: data-formats 8.2 / EX51
    FC4_ACQUIRED(l, "GetPropertyList");
    std::string s;
    Add(s, "n=%lu:", static_cast<unsigned long>(l->GetSize()));
    for (int guard = 0; guard < 64; ++guard) {
        void* p = nullptr;
        if (!l->Pop_Front(kIIDProperty, &p) || p == nullptr) break;
        auto* prop = static_cast<cISCProperty*>(p);
        FC4_ACQUIRED(prop, "PopFront");
        Add(s, " %lX", prop->GetPropertyID());
        FC4_RELEASE(prop);
    }
    Add(s, " left=%lu", static_cast<unsigned long>(l->GetSize()));
    FC4_RELEASE_CREATED(l);  // made for us by GetPropertyList (lessons §5.1b)
    return s;
}

template <class Fn>
Fn Slot(void* obj, int i) {
    return (*reinterpret_cast<Fn**>(obj))[i];
}

// The four scenarios on one side (fe / fc: exemplar / cohort factory).
void RunSide(cIGZPersistResourceFactory* fe, cIGZPersistResourceFactory* fc, std::string out[5]) {
    auto* c = Make<cISCResExemplarCohort>(fc, kTypeCohort, kIIDCohort);
    auto* e1 = Make<cISCResExemplar>(fe, kTypeExemplar, kIIDExemplar);
    auto* e2 = Make<cISCResExemplar>(fe, kTypeExemplar, kIIDExemplar);
    auto* e3 = Make<cISCResExemplar>(fe, kTypeExemplar, kIIDExemplar);
    auto* e4 = Make<cISCResExemplar>(fe, kTypeExemplar, kIIDExemplar);
    if (c == nullptr || e1 == nullptr || e2 == nullptr || e3 == nullptr || e4 == nullptr) {
        for (int i = 0; i < 5; ++i) out[i] = "create failed";
    } else {
        // Setup: properties go in through holder slot 13 (own list), then the
        // documented rebuild (slot 15 with null, or SetCohort) builds the
        // effective list (EX51 b: slot 13 alone does not).
        using Re = bool(__thiscall*)(void*, void*);
        Holder* hc = HolderOf(c);
        hc->AddPropertyU32(0x10, 2, false);
        Slot<Re>(c, 15)(c, nullptr);
        // 1. list order.
        Holder* h1 = HolderOf(e1);
        h1->AddPropertyU32(0x20, 5, false);
        h1->AddPropertyU32(0x27812870, 1, false);
        // EX51 c: SetCohort, RemoveAllProperties and slot 15 have no return
        // value in the original; their results are not compared.
        e1->SetCohort(c, false);
        out[0] += ListIds(h1);
        h1->AddPropertyU32(0x30, 9, false);
        Add(out[0], " add13_has=%lu", h1->HasProperty(0x30));  // EX51 b: own list only, expect 0
        Add(out[0], " local=%lu", e1->IsPropertyLocal(0x30));  // [abi-ret] probed: data-formats 8.2 / EX51
        Slot<Re>(e1, 15)(e1, nullptr);
        out[0] += " after_rebuild: " + ListIds(h1);
        // 2. removal reveals the parent.
        Holder* h2 = HolderOf(e2);
        h2->AddPropertyU32(0x10, 7, false);
        e2->SetCohort(c, false);
        uint32_t v = 0;
        Add(out[1], "pre=%lu", h2->GetPropertyU32(0x10, v) ? v : 999);  // [abi-ret] probed: data-formats 8.2 / EX51
        Add(out[1], " rm=%lu", h2->RemoveProperty(0x10, false));
        v = 0;
        Add(out[1], " get=%lu", h2->GetPropertyU32(0x10, v) ? v : 999);  // [abi-ret] probed: data-formats 8.2 / EX51
        Add(out[1], " local=%lu", e2->IsPropertyLocal(0x10));  // [abi-ret] probed: data-formats 8.2 / EX51
        Add(out[1], " rm2=%lu", h2->RemoveProperty(0x10, false));
        h2->AddPropertyU32(0x40, 1, false);
        h2->RemoveAllProperties(false);
        out[1] += " " + ListIds(h2);
        // 3. SetCohort references (counts relative to the start).
        const uint32_t r0 = RefsOf(c);
        e3->SetCohort(c, false);
        Add(out[2], " d_set=%ld", static_cast<long>(RefsOf(c) - r0));
        Add(out[2], " get_same=%lu", e3->GetCohort() == c);
        Add(out[2], " d_get=%ld", static_cast<long>(RefsOf(c) - r0));
        e3->SetCohort(c, false);
        Add(out[2], " d_again=%ld", static_cast<long>(RefsOf(c) - r0));
        e3->SetCohort(nullptr, false);
        Add(out[2], " d_null=%ld", static_cast<long>(RefsOf(c) - r0));
        Add(out[2], " has10=%lu", HolderOf(e3)->HasProperty(0x10));
        Add(out[2], " cohort_null=%lu", e3->GetCohort() == nullptr);
        // 4. stubs, slot 14 text, slot 15 (null), holder slots 23 / 24.
        alignas(8) static uint8_t dummy[64] = {};
        Add(out[3], "w=%lu", e4->Write(*reinterpret_cast<cIGZOStream*>(dummy)));
        Add(out[3], " r=%lu", e4->Read(*reinterpret_cast<cIGZIStream*>(dummy)));
        Add(out[3], " cls=%lX", e4->GetGZCLSID());
        Add(out[3], " cw=%lu", c->Write(*reinterpret_cast<cIGZOStream*>(dummy)));  // [abi-ret] probed: data-formats 8.2 / EX51
        Add(out[3], " ccls=%lX", c->GetGZCLSID());
        Holder* h4 = HolderOf(e4);
        using B1 = bool(__thiscall*)(void*, uint32_t);
        Add(out[3], " h23=%lu", Slot<B1>(h4, 23)(h4, 0));
        Add(out[3], " h24=%lu", Slot<B1>(h4, 24)(h4, 0));
        h4->AddPropertyU32(0x10, 0, false);
        cRZBaseString nm("Fc4Test");
        h4->AddPropertyString(0x20, nm, false);
        Slot<Re>(e4, 15)(e4, nullptr);
        using Tag = bool(__thiscall*)(void*, cIGZString&);
        cRZBaseString t1, t2, t3;
        Add(out[3], " tag=%lu", Slot<Tag>(e4, 14)(e4, t1));
        out[3] += std::string(" [") + t1.ToChar() + "]";
        Add(out[3], " ctag=%lu", Slot<Tag>(c, 14)(c, t2));
        out[3] += std::string(" [") + t2.ToChar() + "]";
        Slot<Tag>(e1, 14)(e1, t3);
        out[3] += std::string(" inh[") + t3.ToChar() + "]";
    }
    for (cIGZUnknown* u : {static_cast<cIGZUnknown*>(e4), static_cast<cIGZUnknown*>(e3),
                           static_cast<cIGZUnknown*>(e2), static_cast<cIGZUnknown*>(e1), static_cast<cIGZUnknown*>(c)})
        if (u != nullptr) FC4_RELEASE_CREATED(u);  // made for us by CreateInstance
}

// A recording serial record for the SerialWrite scenario: the write slots
// data-formats §8.2 EX51 d names (28 raw, 29 u8, 37 u32, 49 key, 50 variant)
// append text; QueryInterface / AddRef / Release are inert; any other slot
// appends "?N" (taken as argument-free).
std::string* g_recLog = nullptr;
struct FakeRec {
    void** vtbl;
};
void RecAdd(const char* fmt, unsigned long a, unsigned long b = 0, unsigned long c = 0) {
    char t[96];
    snprintf(t, sizeof t, fmt, a, b, c);
    if (g_recLog != nullptr) *g_recLog += t;
}
bool __fastcall RecQI(FakeRec*, void*, uint32_t, void**) { return false; }
uint32_t __fastcall RecRef(FakeRec*, void*) { return 1; }
bool __fastcall RecRaw(FakeRec*, void*, const void* p, uint32_t n) {
    RecAdd(" raw%lu:", n, 0);
    for (uint32_t i = 0; i < n && i < 16; ++i) RecAdd("%02lX", static_cast<const uint8_t*>(p)[i]);
    return true;
}
bool __fastcall RecU8(FakeRec*, void*, uint8_t v) {
    RecAdd(" u8:%lX", v);
    return true;
}
bool __fastcall RecU32(FakeRec*, void*, uint32_t v) {
    RecAdd(" u32:%lX", v);
    return true;
}
bool __fastcall RecKey(FakeRec*, void*, const cGZPersistResourceKey* k) {
    if (k == nullptr) RecAdd(" key:null", 0);
    else RecAdd(" key:%lX,%lX,%lX", k->type, k->group, k->instance);
    return true;
}
bool __fastcall RecVar(FakeRec*, void*, const cIGZVariant* v) {
    if (v == nullptr) {
        RecAdd(" var:null", 0);
        return true;
    }
    auto* g = const_cast<cFC4IGZVariant*>(fc4::AsGame(v));
    RecAdd(" var:t%lX,c%lu,w%lX", g->GetType(), g->GetCount(), g->AsUint32Ref());
    return true;
}
template <int N>
bool __fastcall RecOther(FakeRec*, void*) {
    RecAdd(" ?%lu", N);
    return false;
}
template <int... N>
void FillOthers(void** t, std::integer_sequence<int, N...>) {
    ((t[N] = reinterpret_cast<void*>(&RecOther<N>)), ...);
}
FakeRec* MakeRec() {
    static void* vt[64];
    static FakeRec rec{vt};
    FillOthers(vt, std::make_integer_sequence<int, 64>{});
    vt[0] = reinterpret_cast<void*>(&RecQI);
    vt[1] = vt[2] = reinterpret_cast<void*>(&RecRef);
    vt[28] = reinterpret_cast<void*>(&RecRaw);
    vt[29] = reinterpret_cast<void*>(&RecU8);
    vt[37] = reinterpret_cast<void*>(&RecU32);
    vt[49] = reinterpret_cast<void*>(&RecKey);
    vt[50] = reinterpret_cast<void*>(&RecVar);
    return &rec;
}

cIGZPersistResource* ResOf(cIGZUnknown* u) {
    void* r = nullptr;
    if (!u->QueryInterface(kIIDResource, &r) || r == nullptr) return nullptr;
    FC4_ACQUIRED(static_cast<cIGZPersistResource*>(r), "QueryInterface");
    return static_cast<cIGZPersistResource*>(r);
}

// The EX51 scenarios (data-formats §8.2 as answered): 0 holder add slots
// (b), 1 slot 15 into a native vector (a), 2 SerialWrite bytes (d), 3 tag
// text (e/f). Only bool false is passed.
void RunSide2(cIGZPersistResourceFactory* fe, cIGZPersistResourceFactory* fc, std::string out[4]) {
    using Re = void(__thiscall*)(void*, void*);
    using Tag = bool(__thiscall*)(void*, cIGZString&);
    using SW = bool(__thiscall*)(void*, cIGZPersistResource*, void*);
    auto* c = Make<cISCResExemplarCohort>(fc, kTypeCohort, kIIDCohort);
    auto* c2 = Make<cISCResExemplarCohort>(fc, kTypeCohort, kIIDCohort);
    cISCResExemplar* e[5] = {};
    for (auto*& x : e) x = Make<cISCResExemplar>(fe, kTypeExemplar, kIIDExemplar);
    bool all = c != nullptr && c2 != nullptr;
    for (auto* x : e) all = all && x != nullptr;
    if (!all) {
        for (int i = 0; i < 4; ++i) out[i] = "create failed";
    } else {
        Holder* hc = HolderOf(c);
        hc->AddPropertyU32(0x10, 2, false);
        Slot<Re>(c, 15)(c, nullptr);
        uint32_t v = 0;
        // 0. add slots.
        Holder* h = HolderOf(e[0]);
        e[0]->SetCohort(c, false);
        h->AddPropertyU32(0x10, 7, false);
        Add(out[0], "s13_new_inherited=%lu", h->GetPropertyU32(0x10, v) ? v : 999);  // [abi-ret] probed: data-formats 8.2 / EX51
        h->AddPropertyU32(0x50, 1, false);
        Add(out[0], " s13_new_has=%lu", h->HasProperty(0x50));
        h->AddPropertyU32(0x60, 3, false);
        Slot<Re>(e[0], 15)(e[0], nullptr);
        h->AddPropertyU32(0x60, 4, false);
        v = 0;
        Add(out[0], " s13_inplace=%lu", h->GetPropertyU32(0x60, v) ? v : 999);  // [abi-ret] probed: data-formats 8.2 / EX51
        cISCProperty* p60 = h->GetProperty(0x60);
        Add(out[0], " s14=%lu", p60 != nullptr ? h->AddPropertyVariant(0x10, fc4::AsGame(p60->GetPropertyValue()), false)
                                               : 999);  // [abi-ret] probed: data-formats 8.2 / EX51
        v = 0;
        Add(out[0], " s14_get=%lu", h->GetPropertyU32(0x10, v) ? v : 999);  // [abi-ret] probed: data-formats 8.2 / EX51
        Add(out[0], " s14_has50=%lu", h->HasProperty(0x50));
        HolderOf(e[1])->AddPropertyU32(0x20, 1, false);
        e[1]->SetCohort(c, false);
        cISCProperty* p10 = hc->GetProperty(0x10);
        Add(out[0], " s16=%lu", HolderOf(e[1])->CopyAddProperty(p10, false));  // [abi-ret] probed: data-formats 8.2 / EX51
        cISCProperty* q10 = HolderOf(e[1])->GetProperty(0x10);
        Add(out[0], " s16_copy=%lu", q10 != nullptr && q10 != p10);
        Add(out[0], " s16_local=%lu", e[1]->IsPropertyLocal(0x10));  // [abi-ret] probed: data-formats 8.2 / EX51
        // 1. slot 15 into a vector {begin, end, capacity} of {id, property}.
        struct Elem {
            uint32_t id;
            cISCProperty* p;
        };
        struct Vec {
            Elem* b;
            Elem* e;
            Elem* c;
        } vec = {nullptr, nullptr, nullptr};
        Holder* h2 = HolderOf(e[2]);
        h2->AddPropertyU32(0x20, 5, false);
        h2->AddPropertyU32(0x10, 9, false);
        e[2]->SetCohort(c, false);
        const std::string before = ListIds(h2);
        const uint32_t r10 = RefsOf(p10);
        for (int round = 0; round < 2; ++round) {
            Slot<Re>(e[2], 15)(e[2], &vec);
            Add(out[1], " r%lu:", static_cast<unsigned long>(round));
            Add(out[1], "n=%lu", static_cast<unsigned long>(vec.e - vec.b));
            for (Elem* x = vec.b; x != nullptr && x != vec.e; ++x) {
                Add(out[1], " %lX", x->id);
                Add(out[1], "=%lu", x->p != nullptr && x->id == x->p->GetPropertyID());
            }
            Add(out[1], " d_parent10=%ld", static_cast<long>(RefsOf(p10) - r10));  // the own 0x10 wins: 0
            Add(out[1], " d_own20=%lu", static_cast<unsigned long>(vec.b != nullptr && vec.e != vec.b));
        }
        Add(out[1], " eff_same=%lu", before == ListIds(h2));
        for (Elem* x = vec.b; x != nullptr && x != vec.e; ++x) {
            if (x->p == nullptr) continue;
            FC4_ACQUIRED(x->p, "slot 15 vector element");  // the vector's reference, now ours
            FC4_RELEASE(x->p);
        }
        // The buffer is left allocated: which game allocator 0x0067CB20 uses
        // is question EX52, so it is not freed here (a few bytes, once).
        // 2. SerialWrite (factory slot 11) into the recording record.
        // Setup, the same on both sides: the binary form (factory slot 12
        // SetWriteMode(false), §8.2 EX51 d) on both factories, and explicit
        // keys (SerialWrite returns false unless the resource's type is the
        // factory's own; CreateInstance(type) alone leaves the key's type 0 on
        // the native side). The native factories are no longer registered
        // (ours replaced them, §8.1), so their mode does not reach a save.
        using WM = void(__thiscall*)(void*, bool);
        Slot<WM>(fe, 12)(fe, false);
        Slot<WM>(fc, 12)(fc, false);
        FakeRec* rec = MakeRec();
        g_recLog = &out[2];
        Holder* h3 = HolderOf(e[3]);
        h3->AddPropertyU32(0x30, 9, false);
        h3->AddPropertyU32(0x20, 5, false);
        if (cIGZPersistResource* r = ResOf(e[3])) {
            r->SetKey(cGZPersistResourceKey(kTypeExemplar, 0x33, 0x44));
            Add(out[2], "ex:", 0);
            Add(out[2], " ret=%lu", Slot<SW>(fe, 11)(fe, r, rec));  // [abi-ret] probed: data-formats 8.2 / EX51
            FC4_RELEASE(r);
        }
        if (cIGZPersistResource* rc = ResOf(c)) {
            rc->SetKey(cGZPersistResourceKey(kTypeCohort, 0x11, 0x22));
            Add(out[2], " wrongtype:", 0);
            Add(out[2], " ret=%lu", Slot<SW>(fe, 11)(fe, rc, rec));  // [abi-ret] probed: data-formats 8.2 / EX51
            Add(out[2], " co:", 0);
            Add(out[2], " ret=%lu", Slot<SW>(fc, 11)(fc, rc, rec));  // [abi-ret] probed: data-formats 8.2 / EX51
            FC4_RELEASE(rc);
        }
        Holder* h4 = HolderOf(e[4]);
        h4->AddPropertyU32(0x40, 1, false);
        e[4]->SetCohort(c, false);
        if (cIGZPersistResource* r = ResOf(e[4])) {
            r->SetKey(cGZPersistResourceKey(kTypeExemplar, 0x33, 0x45));
            Add(out[2], " child:", 0);
            Add(out[2], " ret=%lu", Slot<SW>(fe, 11)(fe, r, rec));  // [abi-ret] probed: data-formats 8.2 / EX51
            FC4_RELEASE(r);
        }
        g_recLog = nullptr;
        // 3. tag text: cohort 0x10 = 1 and 0x20 = Uint32 5; an inheriting
        // child; a string name; an add since the last rebuild (not shown).
        Holder* hc2 = HolderOf(c2);
        hc2->AddPropertyU32(0x10, 1, false);
        hc2->AddPropertyU32(0x20, 5, false);
        Slot<Re>(c2, 15)(c2, nullptr);
        cRZBaseString t1, t2, t3, t4;
        Slot<Tag>(c2, 14)(c2, t1);
        out[3] += std::string("c[") + t1.ToChar() + "]";
        e[3]->SetCohort(c2, false);  // own 0x20 = 5, 0x30; inherits 0x10 = 1
        Slot<Tag>(e[3], 14)(e[3], t2);
        out[3] += std::string(" e[") + t2.ToChar() + "]";
        cRZBaseString abc("abc");
        h4->AddPropertyString(0x20, abc, false);  // since the rebuild: not in the effective list
        Slot<Tag>(e[4], 14)(e[4], t3);
        out[3] += std::string(" pending[") + t3.ToChar() + "]";
        Slot<Re>(e[4], 15)(e[4], nullptr);
        Slot<Tag>(e[4], 14)(e[4], t4);
        out[3] += std::string(" str[") + t4.ToChar() + "]";
    }
    for (int i = 4; i >= 0; --i)
        if (e[i] != nullptr) FC4_RELEASE_CREATED(e[i]);  // made for us by CreateInstance
    if (c2 != nullptr) FC4_RELEASE_CREATED(c2);
    if (c != nullptr) FC4_RELEASE_CREATED(c);
}

// Exercise deferred values through the public holder operations: a borrowed
// property's identity and in-place writes must survive materializing others,
// removal, the own-only add slots, enumeration and an effective-list rebuild.
void LazyContractTest() {
    auto* e = new cFC4ResExemplar();
    auto* outer = static_cast<cISCResExemplar*>(e);
    FC4_ADDREF(outer);
    std::vector<uint8_t> bytes = {'E', 'Q', 'Z', 'B', '1', '#', '#', '#'};
    auto word = [&bytes](uint32_t v) {
        for (unsigned b = 0; b < 4; ++b) bytes.push_back(static_cast<uint8_t>(v >> (b * 8)));
    };
    word(0); word(0); word(0); word(4);
    for (uint32_t id : {0x10u, 0x20u, 0x30u, 0x50u}) {
        word(id); bytes.push_back(0); word(3); word(id + 1);
    }
    e->pendingRecord_ = fc4_exrec_parse(kTypeExemplar, bytes.data(), bytes.size());
    unsigned passed = 0;
    if (e->pendingRecord_ != nullptr) {
        e->pending_ = {{0x10, 0}, {0x20, 1}, {0x30, 2}, {0x50, 3}};
        e->hasPending_.store(true, std::memory_order_release);
        ExHolder& h = e->holder_;
        cISCProperty* first = h.GetProperty(0x10);
        passed += first != nullptr && e->own_.Count() == 1;
        if (first != nullptr) fc4::AsGame(first->GetPropertyValue())->SetValUint32(117);
        h.AddPropertyU32(0x20, 19, false);
        uint32_t value = 0;
        passed += h.GetPropertyU32(0x20, value) && value == 19;  // [abi-ret] concrete ExHolder returns bool, no native call
        passed += h.RemoveProperty(0x30, false) && !h.HasProperty(0x30);
        h.AddPropertyU32(0x40, 23, false);
        passed += !h.HasProperty(0x40);
        size_t count = 0;
        h.EnumProperties([](cISCProperty*, void* p) { ++*static_cast<size_t*>(p); }, &count);
        passed += count == 4 && e->pendingRecord_ == nullptr;
        e->Rebuild();
        passed += h.GetPropertyU32(0x40, value) && value == 23;  // [abi-ret] concrete ExHolder returns bool, no native call
        passed += h.GetProperty(0x10) == first && h.GetPropertyU32(0x10, value) && value == 117;  // [abi-ret] concrete ExHolder returns bool, no native call
    }
    Log("formats lazy contract: %u/7 %s", passed, passed == 7 ? "MATCH" : "DIFF");
    FC4_RELEASE_CREATED(outer);
}

// data-formats 4.4 / foundations 10.2: effective lists retain the actual
// property, variant and array storage, including an inherited snapshot.
void IdentityContractTest() {
    auto* parent = new cFC4ResCohort();
    auto* child = new cFC4ResExemplar();
    auto* po = static_cast<cISCResExemplarCohort*>(parent);
    auto* co = static_cast<cISCResExemplar*>(child);
    FC4_ADDREF(po);
    FC4_ADDREF(co);
    auto* property = new cFC4Property(0x100);
    FC4_ADDREF(property);
    uint32_t initial[] = {7, 11, 13};
    property->Value().SetRefUint32(initial, 3);
    parent->holder_.AddPropertyRecord(property, false);
    child->SetParent(po, false);
    cISCProperty* inherited = child->holder_.GetProperty(0x100);
    auto* value = fc4::AsGame(inherited->GetPropertyValue());
    uint32_t* data = value->RefUint32();
    unsigned passed = inherited == property && data != initial && data[1] == 11;
    data[1] = 17;
    passed += property->Value().RefUint32()[1] == 17;
    parent->holder_.RemoveProperty(0x100, false);
    passed += child->holder_.GetProperty(0x100) == inherited && value->RefUint32() == data;
    child->holder_.AddPropertyRecord(inherited, false);
    child->SetParent(nullptr, false);
    passed += child->holder_.GetProperty(0x100) == inherited && value->RefUint32() == data && data[1] == 17;
    cIGZUnknownList* list = nullptr;
    child->holder_.GetPropertyList(&list);
    void* popped = nullptr;
    passed += list != nullptr && list->Pop_Front(kIIDProperty, &popped) && popped == inherited;
    if (popped != nullptr) static_cast<cISCProperty*>(popped)->Release();
    if (list != nullptr) list->Release();
    Log("formats identity contract: %u/5 %s", passed, passed == 5 ? "MATCH" : "DIFF");
    FC4_RELEASE(property);
    FC4_RELEASE_CREATED(co);
    FC4_RELEASE_CREATED(po);
}

void ContractTest() {
    static const char* kNames[8] = {"list_order",    "remove_reveals_parent", "setcohort_refs", "stubs_tag_rebuild",
                                    "add_slots_b",   "rebuild_dest_a",        "serialwrite_d",  "tag_text_ef"};
    std::string nat[5], ours[5], nat2[4], ours2[4];
    t_fixture = true;  // EX50: our calls from here are the fixture's
    RunSide(g_original[0], g_original[1], nat);
    RunSide(g_ours[0], g_ours[1], ours);
    RunSide2(g_original[0], g_original[1], nat2);
    RunSide2(g_ours[0], g_ours[1], ours2);
    LazyContractTest();
    IdentityContractTest();
    t_fixture = false;
    int match = 0;
    for (int i = 0; i < 8; ++i) {
        const std::string& a = i < 4 ? nat[i] : nat2[i - 4];
        const std::string& b = i < 4 ? ours[i] : ours2[i - 4];
        const bool same = a == b;
        match += same ? 1 : 0;
        Log("exemplars: contract %s %s native={%s} ours={%s}", kNames[i], same ? "MATCH" : "DIFF", a.c_str(),
            b.c_str());
    }
    Log("exemplars: contract native-vs-ours %d/8 match", match);
}

}  // namespace

void ExemplarFactorySetEnabled(bool enabled) { g_enabled = enabled; }

void ExemplarFactoryInstall() {
    if (!g_enabled || g_active) return;
    cIGZPersistResourceManagerPtr rm;
    if (!rm) {
        Log("exemplars: no resource manager; not replaced");
        return;
    }
    g_mainThread = GetCurrentThreadId();
    const bool e = TakeOver(rm, 0, kTypeExemplar, kClsExemplarFactory);
    const bool c = TakeOver(rm, 1, kTypeCohort, kClsCohortFactory);
    g_active = e || c;
    if (e && c) ContractTest();
}

void ExemplarFactoryRestore() {
    if (!g_active) return;
    cIGZPersistResourceManagerPtr rm;
    LARGE_INTEGER frequency;
    QueryPerformanceFrequency(&frequency);
    const double ms = 1000.0 / static_cast<double>(frequency.QuadPart);
    Log("formats factory read time: raw=%.3f parse=%.3f parent=%.3f build=%.3f ms",
        g_readTicks[0].load() * ms, g_readTicks[1].load() * ms,
        g_readTicks[2].load() * ms, g_readTicks[3].load() * ms);
    Log("formats factory lazy values: deferred=%u materialized=%u build=%.3f ms",
        g_deferred.load(), g_materialized.load(), g_materializeTicks.load() * ms);
    Log("exemplars: summary reads=%u failed=%u parent_failed=%u cycle_or_depth=%u created=%u empty=%u "
        "resource_props=%u property_lists=%u list_failed=%u message_adds=%u unsupported=%u",
        g_count.reads.load(), g_count.readFails.load(), g_count.parentFails.load(), g_count.cycleFails.load(),
        g_count.created.load(), g_count.emptyProps.load(), g_count.resourceProps.load(), g_count.lists.load(),
        g_count.listFails.load(), g_count.msgAdds.load(), g_count.unsupported.load());
    Log("exemplars: summary2 messages_posted=%u message_failed=%u debug_tags=%u rebuilds=%u rebuild_dest_refused=%u "
        "serial_stubs=%u holder_stubs=%u factory_slot8=%u",
        g_count.msgPosted.load(), g_count.msgFailed.load(), g_count.tags.load(), g_count.rebuilds.load(),
        g_count.rebuildDest.load(), g_count.serialStubs.load(), g_count.holderStubs.load(), g_count.slot8.load());
    LogCallers();
    LogEntries();
    if (rm) {
        Restore(rm, 0, kTypeExemplar, kClsExemplarFactory);
        Restore(rm, 1, kTypeCohort, kClsCohortFactory);
    }
    g_active = false;
}

bool ExemplarFactoryActive() { return g_active; }

}  // namespace fc4
