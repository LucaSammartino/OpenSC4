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

// The growth probe (building-development.md §12) without the game:
//
//   - the counting hook (plugin/FC4GrowthProbeHook.cpp) on fake tract
//     developer vtables in this process, on read-only and execute-read pages:
//     a call through slot 5 the way the game makes it (__thiscall, three
//     dword arguments, callee-pop 12; tract-developer.md §7.1) reaches the
//     original with the same this, EDX and arguments, returns its EAX
//     unchanged, pops 12 bytes, keeps EBX/ESI/EDI/EBP, the x87 stack and the
//     control word; the calls and their time are counted; the exit check
//     runs; remove puts the original back with the page protection as it
//     was; the refusals (no vtable, unaligned, unreadable, unexpected entry,
//     a second install, an entry patched over) write nothing;
//   - its arithmetic and lines (plugin/FC4GrowthProbeLogic.h): trunc(·) as
//     the CRT _ftol, the qualifying count over a hand-built developer map,
//     the day line and the summary, character for character.

#include "FC4GrowthProbeHook.h"
#include "FC4GrowthProbeLogic.h"

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

#include <immintrin.h>

#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <limits>
#include <string>
#include <vector>

using fc4::GrowthHookReport;
using fc4::GrowthHookStatus;

namespace {

int g_failures = 0;
#define CHECK(cond)                                                     \
    do {                                                                \
        if (!(cond)) {                                                  \
            std::printf("FAIL %s:%d: %s\n", __FILE__, __LINE__, #cond); \
            ++g_failures;                                               \
        }                                                               \
    } while (0)

#define CHECK_EQ_STR(a, b)                                                                                 \
    do {                                                                                                   \
        const std::string a_ = (a), b_ = (b);                                                              \
        if (a_ != b_) {                                                                                    \
            std::printf("FAIL %s:%d:\n  got:      %s\n  expected: %s\n", __FILE__, __LINE__, a_.c_str(), \
                        b_.c_str());                                                                       \
            ++g_failures;                                                                                  \
        }                                                                                                  \
    } while (0)

// ---- the fake original Grow ------------------------------------------------------------------------

struct GrowCall {
    void* self;
    void* edx;
    const void* profile;
    void* outcomes;
    void* flag;
};

std::vector<GrowCall> g_growCalls;
int32_t g_growResult = 0;
uint64_t g_growSpinTicks = 0;  // QPC ticks the fake spends inside (0 = none)

int32_t __fastcall FakeGrow(void* self, void* edx, const void* profile, void* outcomes, void* flag) {
    g_growCalls.push_back(GrowCall{self, edx, profile, outcomes, flag});
    if (flag != nullptr) *static_cast<bool*>(flag) = true;
    if (g_growSpinTicks != 0) {
        LARGE_INTEGER a, b;
        QueryPerformanceCounter(&a);
        do {
            QueryPerformanceCounter(&b);
        } while (static_cast<uint64_t>(b.QuadPart - a.QuadPart) < g_growSpinTicks);
    }
    return g_growResult;
}

int32_t __fastcall OtherGrow(void*, void*, const void*, void*, void*) { return 77; }

template <unsigned N>
void __fastcall Trap(void*, void*) {
    std::printf("FAIL fake tract developer: unexpected call of slot %u\n", N);
    std::abort();
}

// A fake vtable of 8 entries, slot 5 = FakeGrow, on its own page with the
// given protection (the game's vtables live in read-only data).
void** MakeVtable(DWORD protect) {
    void** vt = static_cast<void**>(VirtualAlloc(nullptr, 4096, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE));
    if (vt == nullptr) return nullptr;
    void* const slots[8] = {reinterpret_cast<void*>(&Trap<0>), reinterpret_cast<void*>(&Trap<1>),
                            reinterpret_cast<void*>(&Trap<2>), reinterpret_cast<void*>(&Trap<3>),
                            reinterpret_cast<void*>(&Trap<4>), reinterpret_cast<void*>(&FakeGrow),
                            reinterpret_cast<void*>(&Trap<6>), reinterpret_cast<void*>(&Trap<7>)};
    std::memcpy(vt, slots, sizeof(slots));
    DWORD old = 0;
    VirtualProtect(vt, 4096, protect, &old);
    return vt;
}

DWORD Protection(const void* p) {
    MEMORY_BASIC_INFORMATION mbi{};
    if (VirtualQuery(p, &mbi, sizeof(mbi)) != sizeof(mbi)) return 0;
    return mbi.Protect;
}

void ForceEntry(void** vt, unsigned slot, void* value) {
    DWORD old = 0, ignored = 0;
    VirtualProtect(vt + slot, sizeof(void*), PAGE_READWRITE, &old);
    vt[slot] = value;
    VirtualProtect(vt + slot, sizeof(void*), old, &ignored);
}

// ---- a call through slot 5 the way the game makes it ------------------------------------------------

struct GameCall {
    int32_t eax = 0;
    int32_t espDelta = 0;  // 0 = the callee popped exactly the 12 bytes pushed
    bool ebxKept = false, esiKept = false, ediKept = false, ebpKept = false;
    unsigned x87TopBefore = 0, x87TopAfter = 0;
    uint16_t cwBefore = 0, cwAfter = 0;
    uint8_t tagsBefore = 0, tagsAfter = 0;  // FXSAVE abridged tag byte (1 bit per occupied register)
};

constexpr uint32_t kEbx = 0x1B1B1B1Bu, kEsi = 0x51515151u, kEdi = 0xD1D1D1D1u, kEdx = 0xEDEDEDEDu;

struct alignas(16) FxArea {
    uint8_t b[512];
};

__declspec(noinline) GameCall CallGrow(void* self, const void* profile, void* outcomes, void* flag) {
    GameCall r;
    static FxArea fx;
    _fxsave(&fx);
    r.tagsBefore = fx.b[4];
    uint32_t espBefore = 0, espAfter = 0, ebxA = 0, esiA = 0, ediA = 0, ebpB = 0, ebpA = 0;
    int32_t result = 0;
    uint16_t swB = 0, swA = 0, cwB = 0, cwA = 0;
    __asm {
        fnstsw swB
        fnstcw cwB
        mov ebpB, ebp
        mov espBefore, esp
        push flag
        push outcomes
        push profile
        mov ebx, kEbx
        mov esi, kEsi
        mov edi, kEdi
        mov edx, kEdx
        mov ecx, self
        mov eax, [ecx]
        call dword ptr [eax + 0x14]
        mov espAfter, esp
        mov result, eax
        mov ebxA, ebx
        mov esiA, esi
        mov ediA, edi
        mov ebpA, ebp
        mov esp, espBefore
        fnstsw swA
        fnstcw cwA
    }
    _fxsave(&fx);
    r.tagsAfter = fx.b[4];
    r.eax = result;
    r.espDelta = static_cast<int32_t>(espBefore - espAfter);
    r.ebxKept = ebxA == kEbx;
    r.esiKept = esiA == kEsi;
    r.ediKept = ediA == kEdi;
    r.ebpKept = ebpA == ebpB;
    r.x87TopBefore = (swB >> 11) & 7u;
    r.x87TopAfter = (swA >> 11) & 7u;
    r.cwBefore = cwB;
    r.cwAfter = cwA;
    return r;
}

bool CleanCall(const GameCall& c) {
    return c.espDelta == 0 && c.ebxKept && c.esiKept && c.ediKept && c.ebpKept && c.x87TopBefore == c.x87TopAfter &&
           c.cwBefore == c.cwAfter && c.tagsBefore == c.tagsAfter;
}

uint16_t RawCw() {
    uint16_t cw = 0;
    __asm fnstcw cw
    return cw;
}

void LoadCw(uint16_t cw) { __asm fldcw cw }

int g_exitChecks = 0;
void ExitCheck() { ++g_exitChecks; }

// ---- the hook -----------------------------------------------------------------------------------------

void TestRefusals() {
    GrowthHookReport r;
    CHECK(fc4::InstallGrowthHook(nullptr, reinterpret_cast<uintptr_t>(&FakeGrow), &r) == GrowthHookStatus::NullVtable);
    CHECK(r.written == 0);

    void** vt = MakeVtable(PAGE_READONLY);
    CHECK(vt != nullptr);
    if (vt == nullptr) return;
    // Not the expected original: nothing written.
    CHECK(fc4::InstallGrowthHook(vt, reinterpret_cast<uintptr_t>(&OtherGrow), &r) ==
          GrowthHookStatus::UnexpectedEntry);
    CHECK(r.entryAddress == reinterpret_cast<uintptr_t>(vt + 5));
    CHECK(r.found == reinterpret_cast<uintptr_t>(&FakeGrow));
    CHECK(r.written == 0);
    CHECK(vt[5] == reinterpret_cast<void*>(&FakeGrow));
    CHECK(Protection(vt) == PAGE_READONLY);
    CHECK(!fc4::GrowthHookInstalled());
    // An empty entry is never "the original", even when 0 is expected.
    ForceEntry(vt, 5, nullptr);
    CHECK(fc4::InstallGrowthHook(vt, 0, &r) == GrowthHookStatus::UnexpectedEntry);
    CHECK(vt[5] == nullptr);
    ForceEntry(vt, 5, reinterpret_cast<void*>(&FakeGrow));
    // An unaligned vtable.
    void** odd = reinterpret_cast<void**>(reinterpret_cast<uintptr_t>(vt) + 1);
    CHECK(fc4::InstallGrowthHook(odd, reinterpret_cast<uintptr_t>(&FakeGrow), &r) == GrowthHookStatus::Unreadable);
    // Reserved, not committed; and a no-access page.
    void* reserved = VirtualAlloc(nullptr, 65536, MEM_RESERVE, PAGE_NOACCESS);
    CHECK(reserved != nullptr);
    CHECK(fc4::InstallGrowthHook(static_cast<void**>(reserved), reinterpret_cast<uintptr_t>(&FakeGrow), &r) ==
          GrowthHookStatus::Unreadable);
    void** noAccess = MakeVtable(PAGE_NOACCESS);
    CHECK(fc4::InstallGrowthHook(noAccess, reinterpret_cast<uintptr_t>(&FakeGrow), &r) ==
          GrowthHookStatus::Unreadable);
    CHECK(!fc4::GrowthHookInstalled());
    CHECK(fc4::RemoveGrowthHook(&r) == GrowthHookStatus::NotInstalled);
    VirtualFree(reserved, 0, MEM_RELEASE);
    VirtualFree(noAccess, 0, MEM_RELEASE);
    VirtualFree(vt, 0, MEM_RELEASE);
}

void TestForwarding(DWORD protect) {
    void** vt = MakeVtable(protect);
    CHECK(vt != nullptr);
    if (vt == nullptr) return;
    struct Obj {
        void** vtbl;
    } obj{vt};
    int profile = 0;
    int outcomes = 0;
    bool flag = false;

    // The stub itself, before the hook: the fake pops 12 and keeps everything.
    g_growCalls.clear();
    g_growResult = 0x55;
    GameCall c = CallGrow(&obj, &profile, &outcomes, &flag);
    CHECK(c.eax == 0x55 && CleanCall(c));
    CHECK(g_growCalls.size() == 1 && g_growCalls[0].edx == reinterpret_cast<void*>(kEdx));

    const fc4::GrowthHookTotals t0 = fc4::GrowthHookRead();
    GrowthHookReport r;
    CHECK(fc4::InstallGrowthHook(vt, reinterpret_cast<uintptr_t>(&FakeGrow), &r) == GrowthHookStatus::Installed);
    CHECK(r.entryAddress == reinterpret_cast<uintptr_t>(vt + 5));
    CHECK(r.found == reinterpret_cast<uintptr_t>(&FakeGrow));
    CHECK(r.written == fc4::GrowthHookAddress());
    CHECK(r.protectBefore == protect);
    CHECK(r.protectRestored);
    CHECK(Protection(vt) == protect);  // the page as it was (execute bit kept)
    CHECK(vt[5] == reinterpret_cast<void*>(fc4::GrowthHookAddress()));
    CHECK(vt[4] == reinterpret_cast<void*>(&Trap<4>) && vt[6] == reinterpret_cast<void*>(&Trap<6>));
    CHECK(fc4::GrowthHookInstalled());
    CHECK(fc4::GrowthHookVtable() == vt);
    CHECK(fc4::GrowthHookOriginal() == reinterpret_cast<uintptr_t>(&FakeGrow));
    // One at a time.
    GrowthHookReport again;
    CHECK(fc4::InstallGrowthHook(vt, reinterpret_cast<uintptr_t>(&FakeGrow), &again) ==
          GrowthHookStatus::AlreadyInstalled);
    CHECK(again.written == 0);
    fc4::GrowthHookTakeMaxTicks();

    // Three calls the way the game makes them, at the game's 24-bit precision.
    const uint16_t cw = RawCw();
    LoadCw(static_cast<uint16_t>(cw & ~0x0300u));
    fc4::SetGrowthHookExitCheck(&ExitCheck);
    g_exitChecks = 0;
    g_growCalls.clear();
    const int32_t results[3] = {0x100, -3, 0x12345678};
    LARGE_INTEGER freq{};
    QueryPerformanceFrequency(&freq);
    for (int i = 0; i < 3; ++i) {
        g_growResult = results[i];
        g_growSpinTicks = i == 1 ? static_cast<uint64_t>(freq.QuadPart) / 500 : 0;  // 2 ms in the second call
        flag = false;
        c = CallGrow(&obj, &profile, &outcomes, &flag);
        CHECK(c.eax == results[i]);
        CHECK(c.espDelta == 0);
        CHECK(c.ebxKept && c.esiKept && c.ediKept && c.ebpKept);
        CHECK(c.x87TopBefore == c.x87TopAfter && c.tagsBefore == c.tagsAfter);
        CHECK(c.cwBefore == c.cwAfter);
        CHECK(flag);  // the original wrote through the forwarded bool&
    }
    g_growSpinTicks = 0;
    LoadCw(cw);
    fc4::SetGrowthHookExitCheck(nullptr);
    CHECK(g_exitChecks == 3);
    CHECK(g_growCalls.size() == 3);
    for (const GrowCall& g : g_growCalls) {
        CHECK(g.self == &obj);
        CHECK(g.edx == reinterpret_cast<void*>(kEdx));  // EDX passed through
        CHECK(g.profile == &profile && g.outcomes == &outcomes && g.flag == &flag);
    }
    const fc4::GrowthHookTotals t1 = fc4::GrowthHookRead();
    CHECK(t1.calls - t0.calls == 3);
    CHECK(t1.lastResult == 0x12345678);
    const uint64_t ticks = t1.qpcTicks - t0.qpcTicks;
    const uint64_t maxTicks = fc4::GrowthHookTakeMaxTicks();
    CHECK(ticks >= static_cast<uint64_t>(freq.QuadPart) / 500);  // the 2 ms call is inside
    CHECK(maxTicks >= static_cast<uint64_t>(freq.QuadPart) / 500 && maxTicks <= ticks);
    CHECK(fc4::GrowthHookTakeMaxTicks() == 0);  // taken
    CHECK(t1.tscCycles - t0.tscCycles > 0);
    // A call outside the hook's exit check leaves the counter alone.
    CHECK(g_exitChecks == 3);

    // Remove: the original back, the page protection as it was, no more counting.
    GrowthHookReport rm;
    CHECK(fc4::RemoveGrowthHook(&rm) == GrowthHookStatus::Removed);
    CHECK(rm.found == fc4::GrowthHookAddress());
    CHECK(rm.written == reinterpret_cast<uintptr_t>(&FakeGrow));
    CHECK(rm.protectBefore == protect && rm.protectRestored);
    CHECK(vt[5] == reinterpret_cast<void*>(&FakeGrow));
    CHECK(Protection(vt) == protect);
    CHECK(!fc4::GrowthHookInstalled());
    CHECK(fc4::GrowthHookVtable() == nullptr && fc4::GrowthHookOriginal() == 0);
    g_growResult = 9;
    c = CallGrow(&obj, &profile, &outcomes, &flag);
    CHECK(c.eax == 9 && CleanCall(c));
    CHECK(fc4::GrowthHookRead().calls == t1.calls);
    CHECK(fc4::RemoveGrowthHook(&rm) == GrowthHookStatus::NotInstalled);

    // Patched over by someone else: remove leaves their entry; with the
    // forwarder back in place it removes as usual.
    CHECK(fc4::InstallGrowthHook(vt, reinterpret_cast<uintptr_t>(&FakeGrow), &r) == GrowthHookStatus::Installed);
    ForceEntry(vt, 5, reinterpret_cast<void*>(&OtherGrow));
    CHECK(fc4::RemoveGrowthHook(&rm) == GrowthHookStatus::EntryChanged);
    CHECK(rm.written == 0);
    CHECK(vt[5] == reinterpret_cast<void*>(&OtherGrow));
    CHECK(fc4::GrowthHookInstalled());
    ForceEntry(vt, 5, reinterpret_cast<void*>(fc4::GrowthHookAddress()));
    CHECK(fc4::RemoveGrowthHook(&rm) == GrowthHookStatus::Removed);
    CHECK(vt[5] == reinterpret_cast<void*>(&FakeGrow));
    CHECK(Protection(vt) == protect);
    VirtualFree(vt, 0, MEM_RELEASE);
}

// ---- trunc(·) and the qualifying count ----------------------------------------------------------------

void TestTrunc() {
    using fc4::GrowthQualifies;
    using fc4::GrowthTrunc;
    CHECK(GrowthTrunc(5.9f) == 5);
    CHECK(GrowthTrunc(6.0f) == 6);
    CHECK(GrowthTrunc(-3.0f) == -3);
    CHECK(GrowthTrunc(-0.5f) == 0);
    CHECK(GrowthTrunc(-5.9f) == -5);
    CHECK(GrowthTrunc(std::numeric_limits<float>::quiet_NaN()) == 0);
    CHECK(GrowthTrunc(std::numeric_limits<float>::infinity()) == 0);
    CHECK(GrowthTrunc(-std::numeric_limits<float>::infinity()) == 0);
    CHECK(GrowthTrunc(1e19f) == 0);   // outside the int64 range: the indefinite's low half
    CHECK(GrowthTrunc(-1e19f) == 0);
    CHECK(GrowthTrunc(4294967296.0f) == 0);    // 2^32: the low 32 bits
    CHECK(GrowthTrunc(4294967808.0f) == 512);  // 2^32 + 512
    CHECK(GrowthTrunc(2147483648.0f) == std::numeric_limits<int32_t>::min());  // 2^31 wraps negative
    CHECK(!GrowthQualifies(5.99f));
    CHECK(!GrowthQualifies(5.0f));
    CHECK(GrowthQualifies(6.0f));
    CHECK(GrowthQualifies(100.0f));
    CHECK(!GrowthQualifies(-3.0f));
    CHECK(!GrowthQualifies(2147483648.0f));
    CHECK(GrowthQualifies(4294967808.0f));
    CHECK(!GrowthQualifies(std::numeric_limits<float>::quiet_NaN()));
}

// A balanced map built as docs/abi/allocators.md describes it (header colour
// 0, header.parent = root, header.left / right = smallest / largest).
struct BuiltMap {
    fc4::GameMapNode header{};
    std::vector<fc4::GameMapNode> nodes;
    fc4::GameMapObject map{};
};

fc4::GameMapNode* Link(std::vector<fc4::GameMapNode>& n, int lo, int hi, fc4::GameMapNode* parent) {
    if (lo > hi) return nullptr;
    const int mid = (lo + hi) / 2;
    fc4::GameMapNode* x = &n[static_cast<size_t>(mid)];
    x->color = fc4::kGameMapBlack;
    x->parent = parent;
    x->left = Link(n, lo, mid - 1, x);
    x->right = Link(n, mid + 1, hi, x);
    return x;
}

void Build(BuiltMap& m, const std::vector<std::pair<uint32_t, uint32_t>>& entries) {
    m.nodes.assign(entries.size(), fc4::GameMapNode{});
    for (size_t i = 0; i < entries.size(); ++i) {
        m.nodes[i].key = entries[i].first;
        m.nodes[i].value = entries[i].second;
    }
    m.header.color = fc4::kGameMapRed;
    if (entries.empty()) {
        m.header.parent = nullptr;
        m.header.left = m.header.right = &m.header;
    } else {
        m.header.parent = Link(m.nodes, 0, static_cast<int>(entries.size()) - 1, &m.header);
        m.header.left = &m.nodes.front();
        m.header.right = &m.nodes.back();
    }
    m.map = fc4::GameMapObject{&m.header, static_cast<uint32_t>(entries.size()), 0};
}

void TestQualifying() {
    // Developer type -> developer; the "developer" here is an index into the
    // active demands (1-based; 0xFF: its demand is not readable).
    const float demands[] = {0.0f, 6.0f, 5.9f, 100.0f, -3.0f, 6.5f};
    auto active = [&](uint32_t value, float* out) {
        if (value == 0xFF) return false;
        *out = demands[value];
        return true;
    };
    BuiltMap m;
    Build(m, {{0x1010, 1}, {0x1020, 2}, {0x1030, 3}, {0x1040, 4}, {0x3110, 0xFF}, {0x3120, 5}});
    fc4::GrowthQualifyingCount q = fc4::CountQualifyingDevelopers(&m.map, active);
    CHECK(q.status == fc4::GameMapStatus::Ok);
    CHECK(q.developers == 6);
    CHECK(q.qualifying == 3);  // 6.0, 100.0, 6.5
    CHECK(q.noDemand == 1);

    BuiltMap empty;
    Build(empty, {});
    q = fc4::CountQualifyingDevelopers(&empty.map, active);
    CHECK(q.status == fc4::GameMapStatus::Ok && q.developers == 0 && q.qualifying == 0);

    q = fc4::CountQualifyingDevelopers(nullptr, active);
    CHECK(q.status == fc4::GameMapStatus::NullMap && q.developers == 0 && q.qualifying == 0);

    // A corrupt map is refused whole: nothing counted.
    BuiltMap bad;
    Build(bad, {{0x1010, 1}, {0x1020, 3}});
    bad.map.count = 3;
    q = fc4::CountQualifyingDevelopers(&bad.map, active);
    CHECK(q.status == fc4::GameMapStatus::CountMismatch && q.developers == 0 && q.qualifying == 0);
    bad.map.count = fc4::kGrowthMaxDevelopers + 1;
    q = fc4::CountQualifyingDevelopers(&bad.map, active);
    CHECK(q.status == fc4::GameMapStatus::TooMany && q.qualifying == 0);
}

// ---- the lines ----------------------------------------------------------------------------------------

void TestArithmetic() {
    CHECK(fc4::GrowthMicros(30000, 10000000) == 3000);
    CHECK(fc4::GrowthMicros(7, 3) == 2333333);
    CHECK(fc4::GrowthMicros(1ull << 40, 10000000) == 109951162777ull);
    CHECK(fc4::GrowthMicros(5, 0) == 0);
    CHECK(fc4::GrowthTenthsPer(7, 3, 3) == 7777778);  // 23333333 tenths / 3, rounded
    CHECK(fc4::GrowthTenthsPer(30000, 10000000, 3) == 10000);
    CHECK(fc4::GrowthTenthsPer(1, 10000000, 0) == 0);
    CHECK(fc4::GrowthCut(3, 2) == 0);
    CHECK(fc4::GrowthCut(2, 2) == 0);
    CHECK(fc4::GrowthCut(1, 2) == 1);
    CHECK(fc4::GrowthCut(0, 0) == 0);
    CHECK(fc4::GrowthCut(5, -1) == -1);
}

void TestDayLine() {
    fc4::GrowthDayValues v;
    v.date = "2001-03-04";
    v.attempts = 3;
    v.qualifying = 2;
    v.ticks = 30000;
    v.maxTicks = 15000;
    v.cycles = 999;
    v.frequency = 10000000;
    v.lots = 42;
    v.population = 1234;
    CHECK_EQ_STR(fc4::FormatGrowthDayLine(v),
                 "growth probe 2001-03-04: attempts=3 qualifying=2 cut=0 elapsed_us=3000 per_attempt_us=1000.0 "
                 "lots=42 population=1234 max_us=1500 cycles=999");
    v.attempts = 1;
    v.qualifying = 13;
    v.ticks = 7;
    v.maxTicks = 7;
    v.frequency = 3;
    v.cycles = 0;
    v.lots = 0;
    v.population = 0;
    CHECK_EQ_STR(fc4::FormatGrowthDayLine(v),
                 "growth probe 2001-03-04: attempts=1 qualifying=13 cut=1 elapsed_us=2333333 "
                 "per_attempt_us=2333333.3 lots=0 population=0 max_us=2333333 cycles=0");
    fc4::GrowthDayValues none;
    none.date = "(no date)";
    none.frequency = 10000000;
    CHECK_EQ_STR(fc4::FormatGrowthDayLine(none),
                 "growth probe (no date): attempts=0 qualifying=- cut=- elapsed_us=0 per_attempt_us=- lots=- "
                 "population=- max_us=0 cycles=0");
    // Zero attempts with a known count: cut=1 unless nothing qualifies.
    none.qualifying = 0;
    CHECK(fc4::FormatGrowthDayLine(none).find(" qualifying=0 cut=0 ") != std::string::npos);
    none.qualifying = 4;
    CHECK(fc4::FormatGrowthDayLine(none).find(" qualifying=4 cut=1 ") != std::string::npos);
}

void TestSummary() {
    std::vector<fc4::GrowthDayRecord> days;
    auto day = [&](uint32_t attempts, int32_t qualifying, uint64_t ticks, uint64_t cycles) {
        fc4::GrowthDayRecord d;
        d.attempts = attempts;
        d.qualifying = qualifying;
        d.ticks = ticks;
        d.cycles = cycles;
        days.push_back(d);
    };
    day(3, 2, 75, 300);
    day(1, 2, 25, 100);  // cut
    day(0, -1, 0, 0);    // qualifying unknown
    day(5, 4, 125, 500);
    day(2, 4, 50, 200);  // cut
    CHECK_EQ_STR(fc4::FormatGrowthSummary("test", days, 1000000),
                 "growth probe summary (test): days=5; attempts min/median/max=0/2/5 (total 11); cut days=2, "
                 "attempts on cut days min/median/max=1/1.5/2; qualifying min/median/max=2/3/4 (unknown on 1 "
                 "day(s)); mean_us_per_attempt=25.0 (elapsed_us=275); mean_cycles_per_attempt=100");
    days.clear();
    CHECK_EQ_STR(fc4::FormatGrowthSummary("city shutdown", days, 1000000),
                 "growth probe summary (city shutdown): days=0; attempts min/median/max=- (total 0); cut days=0, "
                 "attempts on cut days min/median/max=-; qualifying min/median/max=- (unknown on 0 day(s)); "
                 "mean_us_per_attempt=- (elapsed_us=0); mean_cycles_per_attempt=-");
    day(4, 4, 10, 7);
    CHECK_EQ_STR(fc4::FormatGrowthSummary("app shutdown", days, 10000000),
                 "growth probe summary (app shutdown): days=1; attempts min/median/max=4/4/4 (total 4); cut days=0, "
                 "attempts on cut days min/median/max=-; qualifying min/median/max=4/4/4 (unknown on 0 day(s)); "
                 "mean_us_per_attempt=0.3 (elapsed_us=1); mean_cycles_per_attempt=2");
    const fc4::GrowthDistribution d = fc4::GrowthDistributionOf({7, 1, 4, 9});
    CHECK(d.n == 4 && d.min == 1 && d.max == 9 && d.medianTimesTwo == 11);
    CHECK_EQ_STR(fc4::FormatGrowthDistribution(d), "1/5.5/9");
}

}  // namespace

int main() {
    TestRefusals();
    TestForwarding(PAGE_READONLY);
    TestForwarding(PAGE_EXECUTE_READ);
    TestTrunc();
    TestQualifying();
    TestArithmetic();
    TestDayLine();
    TestSummary();
    if (g_failures != 0) {
        std::printf("growth_probe_test: %d failure(s)\n", g_failures);
        return 1;
    }
    std::printf("growth_probe_test: all checks passed\n");
    return 0;
}
