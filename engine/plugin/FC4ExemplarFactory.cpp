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
#include <windows.h>
#include <intrin.h>
#include <algorithm>
#include <atomic>
#include <cstdint>
#include <cstdio>
#include <cstring>
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
// §4.4 [choice]: at most 32 ancestors load below the record requested.
constexpr size_t kMaxDepth = 32;

std::atomic<bool> g_enabled{false};
std::atomic<bool> g_active{false};

struct Counters {
    std::atomic<uint32_t> reads{0}, readFails{0}, parentFails{0}, cycleFails{0}, resourceProps{0}, emptyProps{0};
    std::atomic<uint32_t> created{0}, lists{0}, listFails{0}, unsupported{0}, msgAdds{0};
    // §8.2 slots: messages posted / not posted (no message), slot 14 / 15,
    // serializable stubs 3-5, holder stubs 23/24, factory slot 8.
    std::atomic<uint32_t> msgPosted{0}, msgFailed{0}, tags{0}, rebuilds{0}, rebuildDest{0}, serialStubs{0};
    std::atomic<uint32_t> holderStubs{0}, slot8{0};
};
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

void NoteCaller(void* ra, uint32_t call) {
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
    Log("exemplars: EX50 GetPropertyList new caller %s first_call=%u reads=%u thread=%s", where, call,
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
        items_.erase(items_.begin());
        return true;
    }
    bool Pop_Back(uint32_t riid, void** ppv) override {
        if (!Peek_Back(riid, ppv)) return false;
        items_.back()->Release();
        items_.pop_back();
        return true;
    }
    bool Clear() override {
        std::vector<cIGZUnknown*> old;
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
        if (front) items_.insert(items_.begin(), e);
        else items_.push_back(e);
        return true;
    }
    // §8.2 slot 6 / 8: every element of the other list, in its order, ahead
    // of / after ours. [choice] the other list is read through its own
    // public slots (4: element by index), so any cIGZUnknownList works.
    bool InsertList(cIGZUnknownList const& other, bool front) {
        auto& o = const_cast<cIGZUnknownList&>(other);
        std::vector<cIGZUnknown*> add;
        const size_t n = o.GetSize();
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
        const std::vector<cIGZUnknown*> copy = items_;
        for (auto* e : copy) fn(&visitor, e, a, b);
        return true;
    }
    uint32_t refs_ = 0;  // §8.2: the constructor leaves the count at 0
    std::vector<cIGZUnknown*> items_;
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
        ++g_count.rebuilds;
        eff_.Clear();
        if (cFC4ISCPropertyHolder* ph = ParentHolder()) {
            cIGZUnknownList* list = nullptr;
            if (ph->GetPropertyList(&list) && list != nullptr) {
                FC4_ACQUIRED(list, "GetPropertyList");
                for (size_t n = list->GetSize(); n > 0; --n) {
                    void* p = nullptr;
                    if (!list->Pop_Front(kIIDProperty, &p) || p == nullptr) break;
                    auto* prop = static_cast<cISCProperty*>(p);
                    FC4_ACQUIRED(prop, "PopFront");
                    eff_.AddPropertyRecord(prop, false);  // the holder keeps its own reference
                    FC4_RELEASE(prop);
                }
                FC4_RELEASE(list);
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
            if (!own_.RemoveProperty(id, false)) return false;  // nothing removed: false, no message
            RevealOrErase(id);
        }
        Post(b);
        return true;
    }
    bool RemoveAllOwn(bool b) {
        {
            std::lock_guard<std::recursive_mutex> g(lock_);
            own_.RemoveAllProperties(false);
            std::vector<uint32_t> ids;
            eff_.EnumProperties([](cISCProperty* p, void* d) {
                static_cast<std::vector<uint32_t>*>(d)->push_back(p->GetPropertyID());
            }, &ids);
            for (uint32_t id : ids) RevealOrErase(id);
        }
        Post(b);
        // [choice] the spec gives no return value for slot 18; true as slot 17's
        // success value.
        return true;
    }
    // §8.2 slot 14 DebugInstanceTag(out): the base resource tag (shared stub
    // 0x0093878E, which appends nothing: drawing-fonts §16 table), then
    // " Exemplar ", "(Other)" / "(Unknown)" by ExemplarType 0x10 (uint 0 /
    // otherwise), then " '<name>'" for ExemplarName 0x20; true. [choice] the
    // properties are looked up in the effective list (the holder's view). The
    // same text for cohorts (the spec describes both bodies together).
    bool DebugTag(cIGZString& out) {
        ++g_count.tags;
        static const char kExemplar[] = " Exemplar ";
        out.Append(kExemplar, sizeof kExemplar - 1);
        uint32_t t = 0;
        if (eff_.HasProperty(0x10)) {
            eff_.GetPropertyU32(0x10, t);
            const char* s = t == 0 ? "(Other)" : "(Unknown)";
            out.Append(s, static_cast<uint32_t>(strlen(s)));
        }
        if (eff_.HasProperty(0x20)) {
            cRZBaseString name;
            eff_.GetPropertyString(0x20, name);
            out.Append(" '", 2);
            out.Append(name.ToChar(), name.Strlen());
            out.Append("'", 1);
        }
        return true;
    }
    // §8.2 slot 15 rebuild(dest). Null: the own effective list. [choice,
    // question EX51] a non-null dest is the original's {id, property} vector,
    // whose memory layout docs/abi does not give: refused (false), counted.
    // [choice] true after a rebuild (the return value is not specified).
    bool RebuildInto(void* dest) {
        if (dest != nullptr) {
            ++g_count.rebuildDest;
            return false;
        }
        Rebuild();
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
            if (c == parent_) return true;  // [choice] the no-op's return value is not specified
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
};

bool ExHolder::QueryInterface(uint32_t riid, void** ppvObj) { return o_->Outer()->QueryInterface(riid, ppvObj); }
uint32_t ExHolder::AddRef() { return o_->Outer()->AddRef(); }
uint32_t ExHolder::Release() { return o_->Outer()->Release(); }
bool ExHolder::HasProperty(uint32_t id) const { return o_->eff_.HasProperty(id); }
bool ExHolder::GetPropertyData(uint32_t id, void* buffer, uint32_t& length) const {
    return o_->eff_.GetPropertyData(id, buffer, length);
}
bool ExHolder::GetPropertyInterface(uint32_t id, uint32_t riid, void** ppv) const {
    return o_->eff_.GetPropertyInterface(id, riid, ppv);
}
bool ExHolder::GetPropertyString(uint32_t id, cIGZString& value) const { return o_->eff_.GetPropertyString(id, value); }
bool ExHolder::GetPropertyU32(uint32_t id, uint32_t& value) const { return o_->eff_.GetPropertyU32(id, value); }
cISCProperty* ExHolder::GetProperty(uint32_t id) const { return o_->eff_.GetProperty(id); }
// [choice, question EX51] §8.2 names only holder slots 15/17/18 as posting the
// change message; the other adds (10-14, 16) update the own and effective
// lists and post nothing.
bool ExHolder::AddPropertyData(uint32_t id, void* data, uint32_t length, bool b) {
    const bool ok = o_->own_.AddPropertyData(id, data, length, b);
    o_->Mirror(id);
    return ok;
}
bool ExHolder::AddPropertyObject(uint32_t id, cIGZUnknown* value, bool b) {
    const bool ok = o_->own_.AddPropertyObject(id, value, b);
    o_->Mirror(id);
    return ok;
}
bool ExHolder::AddPropertyString(uint32_t id, cIGZString const& value, bool b) {
    const bool ok = o_->own_.AddPropertyString(id, value, b);
    o_->Mirror(id);
    return ok;
}
bool ExHolder::AddPropertyU32(uint32_t id, uint32_t value, bool b) {
    const bool ok = o_->own_.AddPropertyU32(id, value, b);
    o_->Mirror(id);
    return ok;
}
bool ExHolder::AddPropertyVariant(uint32_t id, cFC4IGZVariant const* value, bool b) {
    const bool ok = o_->own_.AddPropertyVariant(id, value, b);
    o_->Mirror(id);
    return ok;
}
bool ExHolder::AddPropertyRecord(cISCProperty* property, bool b) {
    if (property == nullptr) return false;
    const bool ok = o_->own_.AddPropertyRecord(property, b);
    o_->Mirror(property->GetPropertyID());
    o_->Post(b);  // §8.2: slot 15 posts the change message
    return ok;
}
bool ExHolder::CopyAddProperty(cISCProperty* property, bool b) {
    if (property == nullptr) return false;
    const bool ok = o_->own_.CopyAddProperty(property, b);
    o_->Mirror(property->GetPropertyID());
    return ok;
}
// §8.2 (EX47): removal edits the own list, then the effective entry shows the
// parent's property again or is erased; false (no message) if nothing was
// removed; the change message when the bool is true.
bool ExHolder::RemoveProperty(uint32_t id, bool b) { return o_->RemoveOwn(id, b); }
bool ExHolder::RemoveAllProperties(bool b) { return o_->RemoveAllOwn(b); }
bool ExHolder::EnumPropertiesFiltered(FilterFn filter, EnumFn fn, void* data) const {
    return o_->own_.EnumPropertiesFiltered(filter, fn, data);
}
bool ExHolder::EnumProperties(EnumFn fn, void* data) const { return o_->own_.EnumProperties(fn, data); }

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
    NoteCaller(_ReturnAddress(), call);
    if (list == nullptr) {
        ++g_count.listFails;
        return false;
    }
    std::vector<cISCProperty*> props;
    {
        std::lock_guard<std::recursive_mutex> g(o_->lock_);
        o_->eff_.EnumProperties([](cISCProperty* p, void* d) { static_cast<std::vector<cISCProperty*>*>(d)->push_back(p); },
                                &props);
        std::sort(props.begin(), props.end(),
                  [](cISCProperty* a, cISCProperty* b) { return a->GetPropertyID() < b->GetPropertyID(); });
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
    cGZPersistResourceKey rkey;
    record.GetKey(rkey);
    // §4.1: the type the record is stored under must be ours (the core
    // parse_record checks it with the signature).
    const uint32_t size = record.GetSize();
    std::vector<uint8_t> bytes(size);
    if (size > 0 && !record.GetFieldVoid(bytes.data(), size)) return false;
    fc4_exrec* rec = fc4_exrec_parse(rkey.type, bytes.data(), bytes.size());
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
        if (parent_ != nullptr) FC4_RELEASE(parent_);
        parent_ = parent;  // our reference from GetResource
        own_.Clear();
        Rebuild();
    }
    // §4.2: properties in record order; a later one with the same ID
    // replaces the earlier (§4.5); each add updates the effective list.
    const size_t n = fc4_exrec_count(rec);
    for (size_t i = 0; i < n; ++i) {
        uint32_t id = 0, key3[3] = {0, 0, 0};
        uint16_t mt = 0;
        bool arr = false;
        const uint8_t* data = nullptr;
        size_t count = 0;
        const int32_t kind = fc4_exrec_prop(rec, i, &id, &mt, &arr, &data, &count, key3);
        if (kind == 0) {
            cFC4PropertyHolder::FromCoreProperty(&own_, id, mt, arr, data, count);
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
    bool IsPropertyLocal(uint32_t id) override { return own_.HasProperty(id); }  // §4.4: own list only
    // [choice, EX48] borrowed: the reference stays with the exemplar.
    cISCResExemplarCohort* GetCohort() override {
        std::lock_guard<std::recursive_mutex> g(lock_);
        return parent_;
    }
    bool SetCohort(cISCResExemplarCohort* c, bool b) override { return SetParent(c, b); }
    bool GetKey(cGZPersistResourceKey& k) override { return ExCore::GetKey(k); }
    bool SetKey(cGZPersistResourceKey const& k) override { return ExCore::SetKey(k); }
    bool CompactProperties() override { return true; }
    // §8.2 (EX47): slot 14 DebugInstanceTag(out), slot 15 rebuild(dest).
    virtual bool DebugInstanceTag(cIGZString& out) { return DebugTag(out); }
    virtual bool RebuildEffective(void* dest) { return RebuildInto(dest); }
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
    bool SetParent(cISCResExemplarCohort* c, bool b) override { return ExCore::SetParent(c, b); }
    cISCPropertyHolder* AsISCPropertyHolder() override { return HolderSdk(); }
    cISCPropertyHolder* AsISCPropertyHolder() const override { return const_cast<cFC4ResCohort*>(this)->HolderSdk(); }
    bool IsPropertyLocal(uint32_t id) override { return own_.HasProperty(id); }
    bool GetKey(cGZPersistResourceKey& k) override { return ExCore::GetKey(k); }
    bool SetKey(cGZPersistResourceKey const& k) override { return ExCore::SetKey(k); }
    bool CompactProperties() override { return true; }
    virtual bool DebugInstanceTag(cIGZString& out) { return DebugTag(out); }  // §8.2, as the exemplar's
    virtual bool RebuildEffective(void* dest) { return RebuildInto(dest); }
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
        void* c = nullptr;
        if (!resource.QueryInterface(kIIDFC4Core, &c) || c == nullptr) return false;
        auto* core = static_cast<ExCore*>(c);
        const bool ok = core->type_ == type_ && core->ReadRecord(record);
        core->CoreRelease();
        return ok;
    }
    bool Write(cIGZPersistResource const&, cIGZPersistDBRecord&) override {
        Unsupported("factory Write (SerialWrite)");
        return false;
    }
    // Slot 7: the original's destructor slot. Our lifetime is the reference
    // count; nothing is deleted here [choice].
    virtual void* Slot7Destroy(uint32_t) { return this; }
    // §8.2 (EX46) slot 8 (record, iid, out): slot 3 with the trailing 0, 0.
    virtual bool CreateFromRecord(cIGZPersistDBRecord& record, uint32_t riid, void** ppv) {
        ++g_count.slot8;
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
    virtual bool SerialWrite(cIGZPersistResource*, cIGZPersistDBSerialRecord*) {
        Unsupported("factory SerialWrite");
        return false;
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
    if (!h->GetPropertyList(&l) || l == nullptr) return "nolist";
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
    FC4_RELEASE(l);
    return s;
}

template <class Fn>
Fn Slot(void* obj, int i) {
    return (*reinterpret_cast<Fn**>(obj))[i];
}

// The four scenarios on one side (fe / fc: exemplar / cohort factory).
void RunSide(cIGZPersistResourceFactory* fe, cIGZPersistResourceFactory* fc, std::string out[4]) {
    auto* c = Make<cISCResExemplarCohort>(fc, kTypeCohort, kIIDCohort);
    auto* e1 = Make<cISCResExemplar>(fe, kTypeExemplar, kIIDExemplar);
    auto* e2 = Make<cISCResExemplar>(fe, kTypeExemplar, kIIDExemplar);
    auto* e3 = Make<cISCResExemplar>(fe, kTypeExemplar, kIIDExemplar);
    auto* e4 = Make<cISCResExemplar>(fe, kTypeExemplar, kIIDExemplar);
    if (c == nullptr || e1 == nullptr || e2 == nullptr || e3 == nullptr || e4 == nullptr) {
        for (int i = 0; i < 4; ++i) out[i] = "create failed";
    } else {
        Holder* hc = HolderOf(c);
        hc->AddPropertyU32(0x10, 2, false);
        // 1. list order (+ EX51 probe: a slot-13 add after the parent is set).
        Holder* h1 = HolderOf(e1);
        h1->AddPropertyU32(0x20, 5, false);
        h1->AddPropertyU32(0x27812870, 1, false);
        Add(out[0], "set=%lu ", e1->SetCohort(c, false));
        out[0] += ListIds(h1);
        h1->AddPropertyU32(0x30, 9, false);
        Add(out[0], " add13_has=%lu", h1->HasProperty(0x30));
        Add(out[0], " local=%lu", e1->IsPropertyLocal(0x30));
        out[0] += " after: " + ListIds(h1);
        // 2. removal reveals the parent.
        Holder* h2 = HolderOf(e2);
        h2->AddPropertyU32(0x10, 7, false);
        e2->SetCohort(c, false);
        uint32_t v = 0;
        Add(out[1], "pre=%lu", h2->GetPropertyU32(0x10, v) ? v : 999);
        Add(out[1], " rm=%lu", h2->RemoveProperty(0x10, false));
        v = 0;
        Add(out[1], " get=%lu", h2->GetPropertyU32(0x10, v) ? v : 999);
        Add(out[1], " local=%lu", e2->IsPropertyLocal(0x10));
        Add(out[1], " rm2=%lu", h2->RemoveProperty(0x10, false));
        h2->AddPropertyU32(0x40, 1, false);
        Add(out[1], " rmall=%lu ", h2->RemoveAllProperties(false));
        out[1] += ListIds(h2);
        // 3. SetCohort references (counts relative to the start).
        const uint32_t r0 = RefsOf(c);
        Add(out[2], "set=%lu", e3->SetCohort(c, false));
        Add(out[2], " d_set=%ld", static_cast<long>(RefsOf(c) - r0));
        Add(out[2], " get_same=%lu", e3->GetCohort() == c);
        Add(out[2], " d_get=%ld", static_cast<long>(RefsOf(c) - r0));
        Add(out[2], " set_again=%lu", e3->SetCohort(c, false));
        Add(out[2], " d_again=%ld", static_cast<long>(RefsOf(c) - r0));
        Add(out[2], " set_null=%lu", e3->SetCohort(nullptr, false));
        Add(out[2], " d_null=%ld", static_cast<long>(RefsOf(c) - r0));
        Add(out[2], " has10=%lu", HolderOf(e3)->HasProperty(0x10));
        Add(out[2], " cohort_null=%lu", e3->GetCohort() == nullptr);
        // 4. stubs, slot 14 text, slot 15 (null), holder slots 23 / 24.
        alignas(8) static uint8_t dummy[64] = {};
        Add(out[3], "w=%lu", e4->Write(*reinterpret_cast<cIGZOStream*>(dummy)));
        Add(out[3], " r=%lu", e4->Read(*reinterpret_cast<cIGZIStream*>(dummy)));
        Add(out[3], " cls=%lX", e4->GetGZCLSID());
        Add(out[3], " cw=%lu", c->Write(*reinterpret_cast<cIGZOStream*>(dummy)));
        Add(out[3], " ccls=%lX", c->GetGZCLSID());
        Holder* h4 = HolderOf(e4);
        using B1 = bool(__thiscall*)(void*, uint32_t);
        Add(out[3], " h23=%lu", Slot<B1>(h4, 23)(h4, 0));
        Add(out[3], " h24=%lu", Slot<B1>(h4, 24)(h4, 0));
        h4->AddPropertyU32(0x10, 0, false);
        cRZBaseString nm("Fc4Test");
        h4->AddPropertyString(0x20, nm, false);
        using Tag = bool(__thiscall*)(void*, cIGZString&);
        cRZBaseString t1, t2, t3;
        Add(out[3], " tag=%lu", Slot<Tag>(e4, 14)(e4, t1));
        out[3] += std::string(" [") + t1.ToChar() + "]";
        Add(out[3], " ctag=%lu", Slot<Tag>(c, 14)(c, t2));
        out[3] += std::string(" [") + t2.ToChar() + "]";
        Slot<Tag>(e1, 14)(e1, t3);
        out[3] += std::string(" inh[") + t3.ToChar() + "]";
        using Re = bool(__thiscall*)(void*, void*);
        Add(out[3], " s15=%lu ", Slot<Re>(e1, 15)(e1, nullptr));
        out[3] += ListIds(h1);
    }
    for (cIGZUnknown* u : {static_cast<cIGZUnknown*>(e4), static_cast<cIGZUnknown*>(e3),
                           static_cast<cIGZUnknown*>(e2), static_cast<cIGZUnknown*>(e1), static_cast<cIGZUnknown*>(c)})
        if (u != nullptr) FC4_RELEASE(u);
}

void ContractTest() {
    static const char* kNames[4] = {"list_order", "remove_reveals_parent", "setcohort_refs", "stubs_tag_rebuild"};
    std::string nat[4], ours[4];
    RunSide(g_original[0], g_original[1], nat);
    RunSide(g_ours[0], g_ours[1], ours);
    int match = 0;
    for (int i = 0; i < 4; ++i) {
        const bool same = nat[i] == ours[i];
        match += same ? 1 : 0;
        Log("exemplars: contract %s %s native={%s} ours={%s}", kNames[i], same ? "MATCH" : "DIFF", nat[i].c_str(),
            ours[i].c_str());
    }
    Log("exemplars: contract native-vs-ours %d/4 match", match);
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
    if (rm) {
        Restore(rm, 0, kTypeExemplar, kClsExemplarFactory);
        Restore(rm, 1, kTypeCohort, kClsCohortFactory);
    }
    g_active = false;
}

bool ExemplarFactoryActive() { return g_active; }

}  // namespace fc4
