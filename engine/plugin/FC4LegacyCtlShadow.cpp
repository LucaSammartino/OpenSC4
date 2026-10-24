// SPDX-License-Identifier: GPL-3.0-or-later
// Published vtable boundaries only (legacy-control sections 2,3,6,7).
// Owner-only replay: SetParameters is counted, NOT claimed as a build replay.
#include "FC4LegacyCtlShadow.h"
#include "FC4Config.h"
#include "FC4Log.h"
#include "compat/FC4Compat.h"
#include "fc4.h"
#include <windows.h>
#include <atomic>
#include <cstring>

extern "C" int32_t fc4_legacyctl_shadow_replay(const uint32_t*, uint32_t, uint32_t, uint32_t*);

namespace fc4 {
namespace {
std::atomic<uint32_t> compared{0}, differences{0}, builds{0}, buildGetters{0};
std::atomic<uint32_t> operationCalls[4]{};
bool installed = false;
#if defined(_M_IX86)
using QueryFn = bool(__fastcall*)(void*, void*, uint32_t, void**);
using RefFn = uint32_t(__fastcall*)(void*, void*);
using OwnFn = void(__fastcall*)(void*, void*, bool);
using ControlFn = void*(__fastcall*)(void*, void*);
using BuildFn = bool(__fastcall*)(void*, void*, void*);

struct State { uint32_t words[5]; };
State Snapshot(const void* owner) {
    State s{};
    const auto* p = static_cast<const uint8_t*>(owner);
    std::memcpy(s.words, p + 0x0C, 16);
    s.words[4] = p[0x1C];
    return s;
}
enum Slot { Query, AddRef, Build, Own, Control, LegacyOwn, SlotCount };
struct Entry { uintptr_t address, expected; void* replacement; };
void* originals[SlotCount]{};

// No native post-state enters Rust: inputs are BEFORE state and method argument.
void Compare(const State& before, const void* owner, uint32_t op, uint32_t argument,
             uint32_t nativeValue, bool checkValue, void* queryOut = nullptr, void* rejectedOut = nullptr) {
    uint32_t result[6]{};
    const bool ok = fc4_legacyctl_shadow_replay(before.words, op, argument, result) == 1;
    const State after = Snapshot(owner);
    uint32_t bad = ok ? 0 : 1;
    if (ok) {
        for (unsigned f = 0; f < 5; ++f) {
            if (after.words[f] != result[f]) {
                ++bad;
                if (differences.load() < 8) Log("legacyctl field difference: op=%u field=%u native=%08X core=%08X",
                    op, f, after.words[f], result[f]);
            }
        }
        if (op == 3) {
            const bool accepted = result[5] != 0;
            const void* expected = accepted ? static_cast<const uint8_t*>(owner) + result[5] - 1 : rejectedOut;
            if ((nativeValue != 0) != accepted || queryOut != expected) ++bad;
        } else if (checkValue && nativeValue != result[5]) {
            ++bad;
        }
    }
    ++compared;
    ++operationCalls[op];
    differences += bad;
}
bool __fastcall QueryHook(void* self, void*, uint32_t iid, void** out) {
    const State before = Snapshot(self);
    // QI callers may supply an uninitialized out-pointer. Copy representation
    // bytes rather than evaluating an indeterminate C++ pointer value.
    uint32_t untouched = 0;
    if (out) std::memcpy(&untouched, out, sizeof(untouched));
    const bool value = reinterpret_cast<QueryFn>(originals[Query])(self, nullptr, iid, out);
    uint32_t returned = 0;
    if (out) std::memcpy(&returned, out, sizeof(returned));
    Compare(before, self, 3, iid, value, true, reinterpret_cast<void*>(returned), reinterpret_cast<void*>(untouched));
    return value;
}
uint32_t __fastcall RefHook(void* self, void*) {
    const State before = Snapshot(self);
    const uint32_t value = reinterpret_cast<RefFn>(originals[AddRef])(self, nullptr);
    Compare(before, self, 0, 0, value, true);
    return value;
}
void __fastcall OwnHook(void* self, void*, bool own) {
    const State before = Snapshot(self);
    reinterpret_cast<OwnFn>(originals[Own])(self, nullptr, own);
    Compare(before, self, 1, own, 0, false);
}
void __fastcall LegacyOwnHook(void* self, void*, bool own) {
    void* owner = static_cast<uint8_t*>(self) - 8;
    const State before = Snapshot(owner);
    reinterpret_cast<OwnFn>(originals[LegacyOwn])(self, nullptr, own);
    Compare(before, owner, 1, own, 0, false);
}
void* __fastcall ControlHook(void* self, void*) {
    void* owner = static_cast<uint8_t*>(self) - 8;
    const State before = Snapshot(owner);
    void* value = reinterpret_cast<ControlFn>(originals[Control])(self, nullptr);
    Compare(before, owner, 2, 0, reinterpret_cast<uint32_t>(value), true);
    return value;
}
bool __fastcall BuildHook(void* self, void*, void* props) {
    const bool value = reinterpret_cast<BuildFn>(originals[Build])(self, nullptr, props);
    ++builds;
    // GETTER probe of a native build, explicitly not a replay of props.
    // Published borrowed GetControl: never AddRef/Release its result.
    ControlHook(static_cast<uint8_t*>(self) + 8, nullptr);
    ++buildGetters;
    return value;
}
bool Install() {
    // Compatibility values exclusively from legacy-control section 3.
    Entry entries[SlotCount] = {
        {0x00AD5360, 0x0094C431, reinterpret_cast<void*>(&QueryHook)},
        {0x00AD5364, 0x0094C47C, reinterpret_cast<void*>(&RefHook)},
        {0x00AD536C, 0x00957A9B, reinterpret_cast<void*>(&BuildHook)},
        {0x00AD537C, 0x0094C314, reinterpret_cast<void*>(&OwnHook)},
        {0x00AD5330, 0x009C32AC, reinterpret_cast<void*>(&ControlHook)},
        {0x00AD5334, 0x0094C4D8, reinterpret_cast<void*>(&LegacyOwnHook)},
    };
    if (!compat::IdentifiedVersion()) return false;
    // Validate EVERY entry before changing any entry; no partial installation.
    for (unsigned i = 0; i < SlotCount; ++i) {
        MEMORY_BASIC_INFORMATION memory{};
        auto** entry = reinterpret_cast<void**>(entries[i].address);
        if (VirtualQuery(entry, &memory, sizeof(memory)) != sizeof(memory) || memory.State != MEM_COMMIT ||
            (memory.Protect & (PAGE_NOACCESS | PAGE_GUARD)) != 0 ||
            reinterpret_cast<uintptr_t>(*entry) != entries[i].expected) return false;
        originals[i] = *entry;
    }
    auto* start = reinterpret_cast<void*>(0x00AD5324);
    DWORD previous = 0;
    if (!VirtualProtect(start, 0x68, PAGE_READWRITE, &previous)) return false;
    for (const auto& entry : entries) *reinterpret_cast<void**>(entry.address) = entry.replacement;
    DWORD ignored = 0;
    VirtualProtect(start, 0x68, previous, &ignored);
    return true;
}
#endif
}  // namespace

void LegacyCtlShadowStartup() {
    if (SettingInt("legacyctl_shadow", nullptr, 0) == 0 || installed) return;
#if defined(_M_IX86)
    installed = Install();
#endif
    Log("legacyctl: owner-method shadow %s; SetParameters build replay excluded; LC14 return contracts open",
        installed ? "installed" : "NOT installed");
}
void LegacyCtlShadowMonth(int32_t julian) {
    if (SettingInt("legacyctl_shadow", nullptr, 0) == 0) return;
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    // Cumulative counts retain sparse panel calls. Empty/refused != exact.
    const uint32_t calls = compared.load();
    Log("%04d-%02d-%02d legacyctl shadow owner-only: compared=%u builds=%u build_getters=%u "
        "addref=%u own=%u control=%u qi=%u build_replay=excluded: mismatches=%u",
        date.year, date.month, date.day_of_month, calls, builds.load(), buildGetters.load(),
        operationCalls[0].load(), operationCalls[1].load(), operationCalls[2].load(), operationCalls[3].load(),
        differences.load() + ((!installed || calls == 0) ? 1u : 0u));
}
}  // namespace fc4
