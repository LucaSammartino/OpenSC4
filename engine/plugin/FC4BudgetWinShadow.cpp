// SPDX-License-Identifier: GPL-3.0-or-later
// budget-window §5.1: natural RestoreStartSettings calls, no fixture or resync.
#include "FC4BudgetWinShadow.h"
#include "FC4Config.h"
#include "FC4InplaceDetour.h"
#include "FC4LegacyCtlHooks.h"
#include "FC4Log.h"
#include "compat/FC4Compat.h"
#include "fc4.h"
#include <array>
#include <atomic>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

extern "C" bool fc4_budgetwin_restore_replay(const uint32_t*, uint32_t, uint32_t*);

namespace fc4 {
namespace {
std::atomic<uint32_t> calls{0}, complete{0}, mismatches{0}, partial{0};
bool installed = false, enabled = false;
#if defined(_M_IX86)
using RestoreFn = void(__fastcall*)(void*, void*, void*);
using FundingFn = bool(__fastcall*)(void*, void*, const float*, uint32_t);
using RefFn = uint32_t(__fastcall*)(void*, void*);
using QueryFn = bool(__fastcall*)(void*, void*, uint32_t, void**);
RestoreFn original = nullptr;
thread_local std::vector<uint32_t>* recording = nullptr;

uint32_t Word(const void* p, uint32_t at) {
    uint32_t value;
    std::memcpy(&value, static_cast<const uint8_t*>(p) + at, 4);
    return value;
}
void Call(uint32_t slot, void* self, uint32_t value = 0, uint32_t line = 0) {
    if (recording) recording->insert(recording->end(), {
        slot, static_cast<uint32_t>(reinterpret_cast<uintptr_t>(self)), value, line});
}
// Record only the owner's direct call, not the collaborator's internal work.
struct NativeCall {
    std::vector<uint32_t>* saved = recording;
    NativeCall() { recording = nullptr; }
    ~NativeCall() { recording = saved; }
};
bool __fastcall FundingHook(void* self, void*, const float* fraction, uint32_t line) {
    Call(11, self, Word(fraction, 0), line);
    NativeCall scope;
    return reinterpret_cast<FundingFn>(legacyrecord::Original(self, 0x2C))(self, nullptr, fraction, line);
}
template<unsigned Slot> uint32_t __fastcall RefHook(void* self, void*) {
    Call(Slot, self);
    NativeCall scope;
    return reinterpret_cast<RefFn>(legacyrecord::Original(self, Slot * 4))(self, nullptr);
}
bool __fastcall QueryHook(void* self, void*, uint32_t iid, void** out) {
    Call(0, self, iid);
    NativeCall scope;
    return reinterpret_cast<QueryFn>(legacyrecord::Original(self, 0))(self, nullptr, iid, out);
}

void __fastcall RestoreHook(void* self, void*, void* win) {
    ++calls;
    // Individual restore needs a service-query/funding call recorder;
    // no individual call earns complete evidence until those outputs are bound.
    if (Word(self, 0x24) != 0) {
        ++partial;
        original(self, nullptr, win);
        return;
    }
    std::array<uint8_t, 0x140> before{};
    std::memcpy(before.data(), self, before.size());
    const uint32_t begin = Word(self, 0x38), end = Word(self, 0x3C);
    if (end < begin || (end - begin) % 8 != 0) {
        ++partial;
        original(self, nullptr, win);
        return;
    }
    const uint32_t count = (end - begin) / 8;
    // Documented +0x2C SliderInfo identities let a later script address the
    // controls actually created; these observations are not comparison credit.
    const uint32_t sliderBegin = Word(self, 0x2C), sliderEnd = Word(self, 0x30);
    std::string ids;
    if (sliderEnd >= sliderBegin && (sliderEnd - sliderBegin) % 8 == 0) {
        for (uint32_t p = sliderBegin; p < sliderEnd; p += 8) {
            char id[16];
            std::snprintf(id, sizeof(id), " %08X", Word(reinterpret_cast<void*>(p), 0));
            ids += id;
        }
    }
    Log("budgetwin observation: selector=%08X saved=%u slider_ids=%s (not comparison credit)",
        Word(self, 0x20), count, ids.c_str());
    std::vector<uint32_t> saved(count * 2), expected(count * 4), observed;
    if (count) std::memcpy(saved.data(), reinterpret_cast<void*>(begin), count * 8);
    const bool replayed = fc4_budgetwin_restore_replay(saved.data(), count, expected.data());
    legacyrecord::HookScope hooks;
    bool bound = replayed;
    for (uint32_t i = 0; i < count; ++i) {
        auto* dept = reinterpret_cast<void*>(saved[i * 2 + 1]);
        bound = legacyrecord::Patch(dept, 0x2C, reinterpret_cast<void*>(&FundingHook), true) && bound;
        bound = legacyrecord::Patch(dept, 0, reinterpret_cast<void*>(&QueryHook), true) && bound;
        bound = legacyrecord::Patch(dept, 4, reinterpret_cast<void*>(&RefHook<1>), true) && bound;
        bound = legacyrecord::Patch(dept, 8, reinterpret_cast<void*>(&RefHook<2>), true) && bound;
    }
    auto* outer = recording;
    recording = &observed;
    original(self, nullptr, win); // Forward ONCE, with the original pointer arguments.
    recording = outer;
    bound = hooks.Finish() && bound;
    const bool ownerSame = std::memcmp(before.data(), self, before.size()) == 0;
    const bool pairsSame = count == 0 || std::memcmp(saved.data(), reinterpret_cast<void*>(begin), count * 8) == 0;
    if (!bound) { ++partial; return; }
    ++complete; // Void return; no out args. All named calls and owner writes checked.
    if (observed != expected || !ownerSame || !pairsSame) {
        ++mismatches;
        Log("budgetwin difference: RestoreStartSettings calls=%u/%u owner_same=%u pairs_same=%u",
            static_cast<unsigned>(observed.size() / 4), count, ownerSame, pairsSame);
    }
}

bool Install() {
    if (!compat::IdentifiedVersion()) return false;
    // Entry address and RET 4: budget-window §5.1 and docs/abi/functions.tsv.
    auto* entry = reinterpret_cast<uint8_t*>(0x0077C5E0);
    MEMORY_BASIC_INFORMATION memory{};
    if (!VirtualQuery(entry, &memory, sizeof(memory)) || memory.Type != MEM_IMAGE ||
        (memory.Protect & 0xff) != PAGE_EXECUTE_READ ||
        reinterpret_cast<uintptr_t>(memory.BaseAddress) + memory.RegionSize - 0x0077C5E0 < 24) return false;
    auto* trampoline = static_cast<uint8_t*>(VirtualAlloc(nullptr, 64, MEM_COMMIT | MEM_RESERVE, PAGE_READWRITE));
    if (!trampoline) return false;
    const auto bytes = inplace::Trampoline(entry, 24, 0x0077C5E0, trampoline, reinterpret_cast<uintptr_t>(trampoline));
    DWORD previous = 0;
    if (!bytes || !VirtualProtect(trampoline, 64, PAGE_EXECUTE_READ, &previous)) {
        VirtualFree(trampoline, 0, MEM_RELEASE);
        return false;
    }
    FlushInstructionCache(GetCurrentProcess(), trampoline, 64);
    // Startup precedes city/UI construction, so no native restore call is active.
    if (!VirtualProtect(entry, 5, PAGE_EXECUTE_READWRITE, &previous)) {
        VirtualFree(trampoline, 0, MEM_RELEASE);
        return false;
    }
    original = reinterpret_cast<RestoreFn>(trampoline);
    inplace::Jump(entry, 0x0077C5E0, reinterpret_cast<uintptr_t>(&RestoreHook));
    DWORD ignored = 0;
    VirtualProtect(entry, 5, previous, &ignored);
    FlushInstructionCache(GetCurrentProcess(), entry, 5);
    return true;
}
#endif
}

void BudgetWinShadowStartup() {
    enabled = SettingInt("budgetwin_shadow", nullptr, SettingInt("budget_shadow", nullptr, 0)) != 0;
    if (!enabled || installed) return;
#if defined(_M_IX86)
    installed = Install();
#endif
    Log("budgetwin: RestoreStartSettings recorder %s; individual service outputs unbound (complete=0)",
        installed ? "installed" : "NOT installed");
}
void BudgetWinShadowMonth(int32_t julian) {
    if (!enabled) return;
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    Log("budgetwin shadow (month) %04d-%02d-%02d: calls=%u complete=%u partial=%u mismatches=%u; resyncs=0 installed=%u",
        date.year, date.month, date.day_of_month, calls.load(), complete.load(), partial.load(), mismatches.load(), installed);
    Log("budgetwin method 0x0077C5E0 cSC4SubBudgetWindow::RestoreStartSettings (month) %04d-%02d-%02d: calls=%u complete=%u mismatches=%u; resyncs=0",
        date.year, date.month, date.day_of_month, calls.load(), complete.load(), mismatches.load());
}
}
