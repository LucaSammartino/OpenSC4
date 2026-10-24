// SPDX-License-Identifier: GPL-3.0-or-later
// lua-host.md §8 and advisors.md §8.2. Record original state, never execute
// scripts, register callbacks, install factories or modify the native vector.
#include "FC4LuaShadow.h"
#include "FC4Config.h"
#include "FC4Interfaces.h"
#include "FC4Log.h"
#include "FC4RefAudit.h"
#include "GZServPtrs.h"
#include "cIGZApp.h"
#include "cIGZFrameWork.h"
#include "cRZCOMDllDirector.h"
#include "cISC4App.h"
#include "cISCLua.h"
#include "fc4.h"
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <cstring>
#include <vector>

namespace {
struct Entry {
    char name[80];
    uint32_t function, isGlobal, registered;
};
struct Comparison {
    uint32_t entries, collectorDiffs, bindingDiffs, scratchDiffs, stackDiffs;
};
static_assert(sizeof(Entry) == 92, "private Rust transport layout");
static_assert(sizeof(Comparison) == 20, "private Rust transport layout");
}
// Private static-link symbol, absent from the public DLL export table.
extern "C" bool fc4_lua_registry_shadow_compare(const Entry*, size_t, uint32_t, Comparison*);

namespace fc4 {
namespace {
bool g_enabled = false, g_active = false;
uint32_t g_lines = 0, g_clean = 0;

bool Read(uintptr_t address, void* buffer, size_t size) {
    SIZE_T got = 0;
    return ReadProcessMemory(GetCurrentProcess(), reinterpret_cast<const void*>(address), buffer, size, &got) && got == size;
}

// The only private layout read: the immutable CRT collector in advisors §8.2.
// Addresses are spec VAs rebased from the preferred 0x00400000 image base.
// Never use this layout to infer a method signature or access a Lua VM heap.
bool CaptureCollector(std::vector<Entry>& entries) {
    const uintptr_t image = reinterpret_cast<uintptr_t>(GetModuleHandleW(nullptr));
    uint32_t bounds[3]{};
    if (!Read(image + (0x00B207CC - 0x00400000), bounds, sizeof(bounds))) return false;
    if (bounds[0] == 0 || bounds[1] < bounds[0] || bounds[2] < bounds[1] ||
        (bounds[1] - bounds[0]) % 12 != 0 || (bounds[1] - bounds[0]) / 12 > 128) return false;
    const uint32_t count = (bounds[1] - bounds[0]) / 12;
    entries.resize(count);
    for (uint32_t i = 0; i < count; ++i) {
        uint32_t native[3]{};
        if (!Read(bounds[0] + 12 * i, native, sizeof(native))) return false;
        Entry& e = entries[i];
        // isGlobal is a byte; the other three bytes are unspecified padding.
        e.function = native[1];
        e.isGlobal = native[2] & 0xFF;
        for (size_t n = 0; n < sizeof(e.name); ++n) {
            if (!Read(static_cast<uintptr_t>(native[0]) + n, &e.name[n], 1)) return false;
            if (e.name[n] == 0) break;
        }
        if (std::memchr(e.name, 0, sizeof(e.name)) == nullptr) return false;
    }
    return true;
}

void RawNamed(cISCLua* lua, int32_t index, const char* name) {
    // Resolve relative table indices before pushing the key. Raw reads cannot
    // invoke a script's __index callback; pseudo-indices stay unchanged (§5).
    const int32_t table = index < 0 && index > -10000 ? lua->GetTop() + index + 1 : index;
    lua->PushString(name);
    lua->RawGet(table);
}
uint32_t FunctionAtTop(cISCLua* lua) {
    // [abi-ret] lua-host.md §5 slot 16: IsCFunction returns a bool in AL.
    return lua->IsCFunction(-1) ? static_cast<uint32_t>(reinterpret_cast<uintptr_t>(lua->ToCFunction(-1))) : 0;
}

void Compare(const char* event, int32_t julian) {
    if (!g_enabled || !g_active) return;
    Comparison result{};
    uint32_t captureDiffs = 0, stackDiffs = 0, scratch = 0;
    std::vector<Entry> entries;
    cIGZFrameWork* fw = RZGetFrameWork();
    cIGZApp* base = fw ? fw->Application() : nullptr;  // borrowed
    cISC4App* app = nullptr;
    if (base && base->QueryInterface(GZIID_cISC4App, reinterpret_cast<void**>(&app)) && app) {
        FC4_ACQUIRED(app, "QueryInterface");
    }
    cISCLua* lua = app ? app->GetLuaInterpreter() : nullptr;  // borrowed, app retained
    if (!lua || !CaptureCollector(entries)) {
        ++captureDiffs;
    } else {
        const int32_t top = lua->GetTop();
        for (auto& e : entries) {
            if (e.isGlobal) {
                RawNamed(lua, -10001, e.name);
            } else {
                RawNamed(lua, -10001, "game");
                if (lua->Type(-1) != cIGZLua5Thread::LuaTypeTable) {
                    lua->Pop(1);
                    ++captureDiffs;
                    continue;
                }
                RawNamed(lua, -1, e.name);
            }
            e.registered = FunctionAtTop(lua);
            lua->Pop(e.isGlobal ? 1 : 2);
            Log("lua-registry record %s: target=%s name=%s callback=0x%08X registered=0x%08X",
                event, e.isGlobal ? "globals" : "game", e.name, e.function, e.registered);
        }
        RawNamed(lua, -10001, "_tf_");
        scratch = FunctionAtTop(lua);
        lua->Pop(1);
        stackDiffs += lua->GetTop() != top;
        lua->SetTop(top);
        if (!fc4_lua_registry_shadow_compare(entries.data(), entries.size(), scratch, &result)) ++captureDiffs;
    }
    if (app) FC4_RELEASE(app);
    const uint32_t mismatches = captureDiffs + stackDiffs + result.collectorDiffs + result.bindingDiffs + result.scratchDiffs + result.stackDiffs;
    fc4_date date{};
    fc4_date_from_julian(julian, &date);
    Log("lua-registry shadow %04d-%02d-%02d %s: entries=%u collector_diffs=%u binding_diffs=%u scratch=0x%08X scratch_diffs=%u stack_diffs=%u capture_diffs=%u; mismatches=%u",
        date.year, date.month, date.day_of_month, event, result.entries, result.collectorDiffs, result.bindingDiffs,
        scratch, result.scratchDiffs, stackDiffs + result.stackDiffs, captureDiffs, mismatches);
    ++g_lines;
    if (mismatches == 0) ++g_clean;
}
}

void LuaShadowCityLoaded(cISC4City* city, int32_t julian) {
    g_enabled = SettingInt("lua_shadow", nullptr, 0) != 0;
    g_active = city != nullptr;
    g_lines = g_clean = 0;
    if (!g_enabled) return;
    Log("lua-registry: ON; record-only collector and registration state; no nondeterminism exclusions; LH6-LH7 block full chunk/call replay; not full Lua verification");
    if (city) {
        auto* sim = AsGame(city->GetSimulator());  // borrowed
        if (sim) julian = sim->GetSimDateNumber();
    }
    Compare("load", julian);
}
void LuaShadowMonth(int32_t julian) { Compare("month", julian); }
void LuaShadowCityShutdown() {
    if (g_enabled && g_active) Log("lua-registry shadow: city shutdown: %u lines, %u with mismatches=0", g_lines, g_clean);
    g_active = false;
}
}
