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

// Single ownership through SC4Vector::push_back (FC4GameContainers.h), and
// game-filled lists that stay in one allocator instance (FC4GameList.h).
//
// SC4Vector and SC4String allocate through the framework's allocator service;
// this test provides a counting one that never really frees, so a double free
// is recorded instead of corrupting the heap. The old pattern (push a local,
// let its destructor run) is the negative control: it must be caught.

#include "FC4GameContainers.h"
#include "FC4GameList.h"
#include "SC4List.h"
#include "SC4String.h"
#include "SC4Vector.h"
#include "cIGZAllocatorService.h"
#include "cIGZFrameWork.h"
#include "cISC4BudgetSimulator.h"
#include "cRZCOMDllDirector.h"

#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <map>
#include <string>

namespace {

int g_failures = 0;
#define CHECK(cond)                                                       \
    do {                                                                  \
        if (!(cond)) {                                                    \
            std::printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond);   \
            ++g_failures;                                                 \
        }                                                                 \
    } while (0)

// Counting allocator: live blocks, frees and double frees. Blocks are never
// returned to the CRT, so a double free cannot corrupt anything here.
class CountingAllocator final : public cIGZAllocatorService {
public:
    std::map<void*, bool> blocks;  // pointer -> live
    std::map<void*, uint32_t> sizes;  // pointer -> bytes requested
    int allocs = 0, frees = 0, doubleFrees = 0, foreignFrees = 0;

    bool QueryInterface(uint32_t, void** out) override {
        *out = this;
        return true;
    }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }
    void* Allocate(uint32_t size) override {
        void* p = std::malloc(size == 0 ? 1 : size);
        blocks[p] = true;
        sizes[p] = size;
        ++allocs;
        return p;
    }
    bool Deallocate(void* p) override {
        if (p == nullptr) return true;
        auto it = blocks.find(p);
        if (it == blocks.end()) {
            ++foreignFrees;
            return false;
        }
        if (!it->second) {
            ++doubleFrees;
            return false;
        }
        it->second = false;
        ++frees;
        return true;
    }
    void* Reallocate(void*, uint32_t) override { return nullptr; }
    int Live() const {
        int n = 0;
        for (const auto& b : blocks) n += b.second ? 1 : 0;
        return n;
    }
    void Reset() {
        for (auto& b : blocks) std::free(b.first);
        blocks.clear();
        sizes.clear();
        allocs = frees = doubleFrees = foreignFrees = 0;
    }
};

CountingAllocator g_alloc;

// Only GetSystemService is used (cRZSysServPtr).
class FakeFrameWork final : public cIGZFrameWork {
public:
    bool QueryInterface(uint32_t, void** out) override {
        *out = nullptr;
        return false;
    }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }
    bool AddSystemService(cIGZSystemService*) override { return false; }
    bool RemoveSystemService(cIGZSystemService*) override { return false; }
    bool GetSystemService(uint32_t, uint32_t, void** out) override {
        *out = static_cast<cIGZAllocatorService*>(&g_alloc);
        return true;
    }
    bool EnumSystemServices(void*, cIGZUnknown*, uint32_t) override { return false; }
    bool AddHook(cIGZFrameWorkHooks*) override { return false; }
    bool RemoveHook(cIGZFrameWorkHooks*) override { return false; }
    bool AddToTick(cIGZSystemService*) override { return false; }
    bool RemoveFromTick(cIGZSystemService*) override { return false; }
    bool AddToOnIdle(cIGZSystemService*) override { return false; }
    bool RemoveFromOnIdle(cIGZSystemService*) override { return false; }
    int32_t GetOnIdleInterval() override { return 0; }
    bool SetOnIdleInterval(int32_t) override { return false; }
    bool OnTick(uint32_t) override { return false; }
    bool OnIdle(uint32_t) override { return false; }
    bool IsTickEnabled() override { return false; }
    void ToggleTick(bool) override {}
    void Quit(int32_t) override {}
    void AbortiveQuit(int32_t) override {}
    cIGZCmdLine* CommandLine() override { return nullptr; }
    bool IsInstall() override { return false; }
    cIGZCOM* GetCOMObject() override { return nullptr; }
    FrameworkState GetState() override { return kStatePostAppInit; }
    void* GetDebugStream() override { return nullptr; }
    int32_t DefaultDebugStream() override { return 0; }
    int32_t DebugStream() override { return 0; }
    bool SetDebugStream(void*) override { return false; }
    bool SetDebugLevel(int32_t) override { return false; }
    int32_t GetDebugLevel() override { return 0; }
    cIGZOStream* StdOut() override { return nullptr; }
    cIGZOStream* StdErr() override { return nullptr; }
    cIGZIStream* StdIn() override { return nullptr; }
    bool GetStream(tStreamType, uint32_t, void**) override { return false; }
    bool SetStream(tStreamType, cIGZUnknown*) override { return false; }
    bool SetApplication(cIGZApp* const) override { return false; }
    cIGZApp* const Application() override { return nullptr; }
    void ReportException(char const*) override {}
    cIGZExceptionNotification* ExceptionNotificationObj() override { return nullptr; }
};

FakeFrameWork g_fw;

using GroupInfo = cISC4BudgetSimulator::BudgetGroupInfo;

std::string Str(const SC4String& s) { return std::string(s.ToChar(), s.Strlen()); }

// A second instance: the game's STL node allocator (docs/abi/allocators.md).
CountingAllocator g_stl;
const void* const kFakeStringVtable[4] = {};
struct TestStlAlloc {
    static bool Available() { return true; }
    static void* Allocate(uint32_t size) { return g_stl.Allocate(size); }
    static void Free(void* p) { g_stl.Deallocate(p); }
    static const void* StringVtable() { return kFakeStringVtable; }
};
struct NoAlloc {
    static bool Available() { return false; }
    static void* Allocate(uint32_t) { return nullptr; }
    static void Free(void*) {}
    static const void* StringVtable() { return nullptr; }
};

// A game-owned container: built in raw storage, never destroyed by us.
template <class T>
T* GameOwned() {
    static_assert(sizeof(T) <= 64, "storage");
    alignas(16) static unsigned char storage[8][64];
    static int next = 0;
    return new (storage[next++ % 8]) T();
}

class FakeLot final : public cIGZUnknown {
public:
    bool QueryInterface(uint32_t, void** out) override {
        *out = nullptr;
        return false;
    }
    uint32_t AddRef() override { return ++refs; }
    uint32_t Release() override { return refs > 0 ? --refs : 0; }
    uint32_t refs = 1;  // the lot manager's
};

using LotNode = SC4ListNode<cIGZUnknown*>;

// What GetLotsInCellRect does (allocators.md): clear the list keeping its
// head (nodes freed to the STL node allocator), then append one node per lot
// from that allocator, with one reference each.
void GameFill(LotNode* head, FakeLot* lots, int n) {
    for (LotNode* x = head->next; x != head;) {
        LotNode* next = x->next;
        g_stl.Deallocate(x);
        x = next;
    }
    head->next = head->prev = head;
    for (int i = 0; i < n; ++i) {
        auto* node = static_cast<LotNode*>(g_stl.Allocate(sizeof(LotNode)));
        lots[i].AddRef();
        node->value = &lots[i];
        node->next = head;
        node->prev = head->prev;
        head->prev->next = node;
        head->prev = node;
    }
}

// The game's side: destroy every element once, then the vector's storage.
// (SC4Vector<SC4String>'s own destructor calls Release, not ~SC4String.)
void GameDestroysStrings(SC4Vector<SC4String>* v) {
    for (SC4String& s : *v) s.~SC4String();
}

}  // namespace

// The SDK's service pointers reach the framework through the director.
namespace {
class TestDirector final : public cRZCOMDllDirector {
public:
    TestDirector() { mpFrameWork = &g_fw; }
    uint32_t GetDirectorID() const override { return 0x6C0FFEE0; }
};
}  // namespace

cRZCOMDllDirector* RZGetCOMDllDirector() {
    static TestDirector director;
    return &director;
}

int main() {
    std::setvbuf(stdout, nullptr, _IONBF, 0);
    static const char* const kNames[] = {"Health", "Education", "Safety", "Utilities", "Transportation",
                                         "a name longer than any small-string buffer would hold"};
    const int kCount = static_cast<int>(sizeof(kNames) / sizeof(kNames[0]));

    // 1. SC4String elements (EnumerateAgentsByName), through reallocations.
    {
        g_alloc.Reset();
        auto* v = new SC4Vector<SC4String>();
        for (int i = 0; i < kCount; ++i)
            fc4::PushOwned(*v, kNames[i], static_cast<uint32_t>(std::strlen(kNames[i])));
        CHECK(static_cast<int>(v->size()) == kCount);
        for (int i = 0; i < kCount; ++i) CHECK(Str(v->begin()[i]) == kNames[i]);
        CHECK(g_alloc.Live() == kCount + 1);  // one buffer per string, plus the vector's storage
        CHECK(g_alloc.doubleFrees == 0);
        GameDestroysStrings(v);
        delete v;
        CHECK(g_alloc.Live() == 0);
        CHECK(g_alloc.doubleFrees == 0);
        CHECK(g_alloc.foreignFrees == 0);
        std::printf("SC4String: %d allocations, %d frees, %d double frees\n", g_alloc.allocs, g_alloc.frees,
                    g_alloc.doubleFrees);
    }

    // 2. BudgetGroupInfo (GetAllGroups): the vector's destructor runs
    // ~BudgetGroupInfo, as the game's does.
    {
        g_alloc.Reset();
        auto* v = new SC4Vector<GroupInfo>();
        for (int i = 0; i < kCount; ++i) {
            fc4::PushOwned(*v, static_cast<uint32_t>(i + 1),
                           SC4String(kNames[i], static_cast<uint32_t>(std::strlen(kNames[i]))),
                           static_cast<int64_t>(100 * i), static_cast<int64_t>(-7 * i));
        }
        CHECK(g_alloc.doubleFrees == 0);
        for (int i = 0; i < kCount; ++i) {
            const GroupInfo& g = v->begin()[i];
            CHECK(g.groupID == static_cast<uint32_t>(i + 1));
            CHECK(Str(g.groupName) == kNames[i]);
            CHECK(g.totalExpense == 100 * i);
            CHECK(g.totalIncome == -7 * i);
        }
        v->clear();  // GetAllGroups clears the caller's vector first
        CHECK(g_alloc.doubleFrees == 0);
        for (int i = 0; i < 2; ++i)
            fc4::PushOwned(*v, 9u, SC4String(kNames[i], static_cast<uint32_t>(std::strlen(kNames[i]))),
                           int64_t{0}, int64_t{0});
        delete v;
        CHECK(g_alloc.Live() == 0);
        CHECK(g_alloc.doubleFrees == 0);
        CHECK(g_alloc.foreignFrees == 0);
        std::printf("BudgetGroupInfo: %d allocations, %d frees, %d double frees\n", g_alloc.allocs, g_alloc.frees,
                    g_alloc.doubleFrees);
    }

    // 3. Negative control: the old PushGroup (a local pushed bitwise, then
    // destroyed) frees every name twice. The counter must see it.
    {
        g_alloc.Reset();
        auto* v = new SC4Vector<GroupInfo>();
        for (int i = 0; i < 3; ++i) {
            GroupInfo info;
            info.groupID = 1;
            info.groupName = SC4String(kNames[i], static_cast<uint32_t>(std::strlen(kNames[i])));
            info.totalExpense = 0;
            info.totalIncome = 0;
            v->push_back(info);
        }  // ~GroupInfo frees the buffer the element points at
        delete v;  // ... and the owner frees it again
        CHECK(g_alloc.doubleFrees == 3);
        std::printf("negative control (old pattern): %d double frees detected\n", g_alloc.doubleFrees);
    }

    // 4. GameList: every node, head included, stays in the STL node instance;
    // the service sees nothing. Filled twice (the second fill clears first),
    // released the plugin's way (release, null), then destroyed.
    {
        g_alloc.Reset();
        g_stl.Reset();
        FakeLot lots[5];
        {
            fc4::GameList<cIGZUnknown*, TestStlAlloc> list;
            CHECK(list.valid());
            GameFill(list.head(), lots, 3);
            for (cIGZUnknown*& v : list) {  // a first scan, released the plugin's way
                v->Release();
                v = nullptr;
            }
            GameFill(list.head(), lots, 5);  // clears the 3 nodes, keeps the head
            int n = 0;
            for (cIGZUnknown*& v : list) {
                ++n;
                v->Release();
                v = nullptr;
            }
            CHECK(n == 5);
        }
        CHECK(g_stl.allocs == 1 + 3 + 5);
        CHECK(g_stl.frees == g_stl.allocs);
        CHECK(g_stl.Live() == 0 && g_stl.doubleFrees == 0 && g_stl.foreignFrees == 0);
        CHECK(g_alloc.allocs == 0 && g_alloc.frees == 0 && g_alloc.foreignFrees == 0);
        bool refsOk = true;
        for (const FakeLot& l : lots) refsOk = refsOk && l.refs == 1;
        CHECK(refsOk);
        std::printf("GameList: STL instance %d allocations / %d frees, service %d / %d, foreign frees %d / %d\n",
                    g_stl.allocs, g_stl.frees, g_alloc.allocs, g_alloc.frees, g_stl.foreignFrees, g_alloc.foreignFrees);
    }

    // 5. GameList destructor: values left in the list are released once.
    {
        g_stl.Reset();
        FakeLot lots[2];
        {
            fc4::GameList<cIGZUnknown*, TestStlAlloc> list;
            GameFill(list.head(), lots, 2);
            CHECK(lots[0].refs == 2 && lots[1].refs == 2);
        }
        CHECK(lots[0].refs == 1 && lots[1].refs == 1);
        CHECK(g_stl.Live() == 0 && g_stl.doubleFrees == 0 && g_stl.foreignFrees == 0);
    }

    // 6. No allocator (unknown exe): not valid, nothing allocated or freed.
    {
        fc4::GameList<cIGZUnknown*, NoAlloc> list;
        CHECK(!list.valid());
    }

    // 7. Negative control: the SDK SC4List frees the game's nodes to the
    // service, a different instance. The counters must see every crossing.
    {
        g_alloc.Reset();
        g_stl.Reset();
        FakeLot lots[4];
        {
            SC4List<cIGZUnknown*> list;  // head from the service
            LotNode* head = *reinterpret_cast<LotNode**>(&list);
            GameFill(head, lots, 4);
            for (cIGZUnknown*& v : list) {
                v->Release();
                v = nullptr;
            }
        }
        CHECK(g_alloc.foreignFrees == 4);  // the 4 STL nodes freed to the service
        CHECK(g_stl.Live() == 4);          // and never returned to their own instance
        std::printf("negative control (SDK SC4List): %d foreign frees into the service detected\n",
                    g_alloc.foreignFrees);
    }

    // 8. GameVector growth from a buffer the game already owns (GetAllLoans,
    // GetAllLineItems): reserve() frees the old B buffer to B; the game then
    // frees the final buffer to B.
    {
        g_alloc.Reset();
        g_stl.Reset();
        auto* v = GameOwned<fc4::GameVector<cISC4BudgetSimulator::LoanInfo, TestStlAlloc>>();
        CHECK(v->reserve(2));  // the window's member vector from an earlier call
        cISC4BudgetSimulator::LoanInfo loan{};
        v->push_back(loan);
        v->clear();  // arrives emptied, still owning its B buffer
        CHECK(v->capacity() == 2 && g_stl.Live() == 1);
        for (int i = 0; i < 9; ++i) {
            loan.monthlyPayment = i;
            CHECK(v->push_back(loan));
        }
        CHECK(v->size() == 9 && (*v)[8].monthlyPayment == 8);
        g_stl.Deallocate(v->begin());  // the game destroys it
        CHECK(g_stl.Live() == 0 && g_stl.doubleFrees == 0 && g_stl.foreignFrees == 0);
        CHECK(g_alloc.allocs == 0 && g_alloc.frees == 0);
        std::printf("GameVector growth: STL instance %d allocations / %d frees, service %d / %d\n", g_stl.allocs,
                    g_stl.frees, g_alloc.allocs, g_alloc.frees);
    }

    // 9. Strings inside elements (GetAllGroups): B buffers with the game's
    // vtable; the game destroys them inline (free begin with B), then the
    // vector's buffer with B.
    {
        g_alloc.Reset();
        g_stl.Reset();
        auto* v = GameOwned<fc4::GameVector<fc4::GameBudgetGroupInfo, TestStlAlloc>>();
        for (int i = 0; i < kCount; ++i) {
            fc4::GameBudgetGroupInfo g{};
            g.groupID = static_cast<uint32_t>(i + 1);
            g.totalExpense = 10 * i;
            CHECK(fc4::GameStringInit<TestStlAlloc>(g.groupName, kNames[i], std::strlen(kNames[i])));
            CHECK(v->push_back(g));
        }
        bool layoutOk = true;
        for (int i = 0; i < kCount; ++i) {
            const fc4::GameString& s = (*v)[i].groupName;
            const size_t n = std::strlen(kNames[i]);
            layoutOk = layoutOk && s.vtable == kFakeStringVtable && s.size() == n &&
                       std::memcmp(s.begin, kNames[i], n) == 0 && *s.end == 0 && s.capacityEnd == s.end + 1 &&
                       g_stl.sizes[s.begin] == n + 1 &&  // exactly n + 1 bytes, as the game allocates
                       s.refCount == 0 && (*v)[i].groupID == static_cast<uint32_t>(i + 1);
        }
        CHECK(layoutOk);
        for (fc4::GameBudgetGroupInfo& g : *v) g_stl.Deallocate(g.groupName.begin);  // the game, inline
        g_stl.Deallocate(v->begin());
        CHECK(g_stl.Live() == 0 && g_stl.doubleFrees == 0 && g_stl.foreignFrees == 0);
        CHECK(g_alloc.allocs == 0 && g_alloc.frees == 0);
        std::printf("GameBudgetGroupInfo strings: STL instance %d allocations / %d frees, service %d / %d\n",
                    g_stl.allocs, g_stl.frees, g_alloc.allocs, g_alloc.frees);
    }

    // 10. EnumerateAgentsByName: clear() destroys the strings with B and keeps
    // the buffer; refill; our own vector's destructor frees everything to B.
    {
        g_alloc.Reset();
        g_stl.Reset();
        {
            fc4::GameVector<fc4::GameString, TestStlAlloc> names;
            for (int round = 0; round < 2; ++round) {
                names.clear();
                for (int i = 0; i < kCount; ++i) {
                    fc4::GameString s{};
                    CHECK(fc4::GameStringInit<TestStlAlloc>(s, kNames[i], std::strlen(kNames[i])));
                    CHECK(names.push_back(s));
                }
            }
            CHECK(names.size() == static_cast<size_t>(kCount));
        }
        CHECK(g_stl.Live() == 0 && g_stl.doubleFrees == 0 && g_stl.foreignFrees == 0);
        CHECK(g_alloc.allocs == 0);
        fc4::GameString none{};
        CHECK(!fc4::GameStringInit<NoAlloc>(none, "x", 1));  // unknown exe: refused
    }

    // 11. GetLocallyFundedItemsByPurpose: the game allocates the head with B,
    // we append nodes with B (one reference each), the game releases each and
    // frees nodes and head with B.
    {
        g_alloc.Reset();
        g_stl.Reset();
        FakeLot lots[3];
        auto* list = GameOwned<fc4::GameList<cIGZUnknown*, TestStlAlloc>>();  // head: B
        for (FakeLot& l : lots) {
            CHECK(list->push_back(&l));
            l.AddRef();
        }
        int n = 0;
        LotNode* head = list->head();
        for (LotNode* x = head->next; x != head;) {  // the game destroys it
            LotNode* next = x->next;
            x->value->Release();
            g_stl.Deallocate(x);
            x = next;
            ++n;
        }
        g_stl.Deallocate(head);
        CHECK(n == 3);
        CHECK(lots[0].refs == 1 && lots[1].refs == 1 && lots[2].refs == 1);
        CHECK(g_stl.Live() == 0 && g_stl.doubleFrees == 0 && g_stl.foreignFrees == 0);
        CHECK(g_alloc.allocs == 0 && g_alloc.frees == 0);
        std::printf("GameList append: STL instance %d allocations / %d frees, service %d / %d\n", g_stl.allocs,
                    g_stl.frees, g_alloc.allocs, g_alloc.frees);
    }

    // 12. GetFloraTypesByFilter / ByFamily (flora slots 13, 14): the game
    // builds a hash_set<uint32_t> with 193 buckets (hint 100), we insert with
    // B (growing it past 193 elements), the game walks and destroys it (each
    // node, then the bucket buffer, freed with B).
    {
        g_alloc.Reset();
        g_stl.Reset();
        using Set = fc4::GameHashSetU32<TestStlAlloc>;
        CHECK(Set::NextPrime(100) == 193 && Set::NextPrime(1) == 53 && Set::NextPrime(194) == 389);
        CHECK(Set::NextPrime(4294967291u) == 4294967291u && Set::NextPrime(0xFFFFFFFFu) == 4294967291u);
        alignas(8) static unsigned char raw[sizeof(Set)];
        auto* set = new (raw) Set(100);  // game-owned: never destroyed by us
        CHECK(set->bucket_count() == 193 && set->size() == 0);
        // Head insertion: two values of one chain come out newest first.
        CHECK(set->insert(5) && set->insert(5 + 193));
        CHECK(!set->insert(5));  // present: not inserted again
        CHECK(set->bucket(5) != nullptr && set->bucket(5)->value == 5 + 193 && set->bucket(5)->next->value == 5);
        for (uint32_t v = 1000; v < 1250; ++v) set->insert(v * 7919u);
        CHECK(set->size() == 252 && set->bucket_count() == 389);  // resize(count + 1) at 194 -> 389
        size_t nodes = 0;
        bool placed = true;
        for (size_t b = 0; b < set->bucket_count(); ++b) {
            for (auto* p = set->bucket(b); p != nullptr; p = p->next) {
                ++nodes;
                placed = placed && (p->value % set->bucket_count()) == b;
            }
        }
        CHECK(nodes == set->size() && placed);
        CHECK(set->contains(5) && set->contains(5 + 193) && set->contains(1000u * 7919u) && !set->contains(6));
        // The game destroys it (0x008EBA60, then the bucket buffer).
        for (size_t b = 0; b < set->bucket_count(); ++b) {
            for (auto* p = set->bucket(b); p != nullptr;) {
                auto* next = p->next;
                g_stl.Deallocate(p);
                p = next;
            }
        }
        g_stl.Deallocate(set->buckets());
        CHECK(g_stl.Live() == 0 && g_stl.doubleFrees == 0 && g_stl.foreignFrees == 0);
        CHECK(g_alloc.allocs == 0);
        // Our own set frees everything to B; no allocator: nothing inserted.
        {
            Set own(1);
            for (uint32_t v = 0; v < 60; ++v) own.insert(v);
            CHECK(own.size() == 60 && own.bucket_count() == 97);
        }
        CHECK(g_stl.Live() == 0 && g_stl.doubleFrees == 0 && g_stl.foreignFrees == 0);
        fc4::GameHashSetU32<NoAlloc> none(100);
        CHECK(none.bucket_count() == 0 && !none.insert(1) && none.size() == 0);
        std::printf("GameHashSetU32: STL instance %d allocations / %d frees, service %d / %d\n", g_stl.allocs,
                    g_stl.frees, g_alloc.allocs, g_alloc.frees);
    }

    g_stl.Reset();
    g_alloc.Reset();
    if (g_failures != 0) {
        std::printf("%d failure(s)\n", g_failures);
        return 1;
    }
    std::printf("game_container_test: ok\n");
    return 0;
}
