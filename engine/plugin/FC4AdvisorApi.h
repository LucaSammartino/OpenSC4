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

// The advisor system's pure pieces (no game calls), shared by
// cFC4AdvisorSystem.cpp and tests/advisor_api_test.cpp:
// - the 47-entry register_api vector (advisors.md §8.2, AD7);
// - the slot 11 DoLink text split (§12.1, AD8);
// - the slot 9 generator's shape (§9, §12.1, AD8);
// - the reload directories (§4.2, AD10).
#pragma once

#include "compat/FC4Compat.h"

#include <cctype>
#include <cstddef>
#include <cstdint>
#include <cstdio>
#include <string>

namespace fc4 {
namespace advisors {

using compat::Direct;

// `native` names an original function registered as it is (a compat direct
// call); kCore means the core's function through a trampoline.
// [choice] advice_info (entry 1, a debug message box, §8.3) has no published
// address: it stays the core's (no effect, no results).
struct ApiEntry {
    const char* name;
    bool global;
    Direct native;
};
constexpr Direct kCore = Direct::Count;

// §8.2: simulation collector (1-38), then UI collector (39-47), each in its
// constructor's order; only 36-38 are global.
constexpr ApiEntry kApi[] = {
    {"advice_info", false, kCore},                                   // 1
    {"trend_value", false, kCore},                                   // 2
    {"trend_slope", false, kCore},                                   // 3
    {"trend_count", false, kCore},                                   // 4
    {"camera_jump", false, Direct::LuaCameraJump},                   // 5 (AD6)
    {"camera_zoom", false, Direct::LuaCameraZoom},                   // 6 (AD6)
    {"ordinance_is_on", false, kCore},                               // 7
    {"ordinance_set_off", false, kCore},                             // 8
    {"ordinance_set_on", false, kCore},                              // 9
    {"ordinance_is_available", false, kCore},                        // 10
    {"ordinance_name", false, kCore},                                // 11
    {"ordinance_description", false, kCore},                         // 12
    {"ordinance_enact_fee", false, kCore},                           // 13
    {"ordinance_repeal_fee", false, kCore},                          // 14
    {"ordinance_monthly_cost", false, kCore},                        // 15
    {"difficulty_level", false, kCore},                              // 16
    {"event_feedback", false, kCore},                                // 17
    {"expire_advice", false, kCore},                                 // 18
    {"retire_advice", false, kCore},                                 // 19
    {"advice_subject", false, kCore},                                // 20
    {"trigger_event", false, kCore},                                 // 21
    {"building_is_group", false, kCore},                             // 22
    {"event_building_distance", false, kCore},                       // 23
    {"mysim_distance_to_closest_building", false, kCore},            // 24
    {"mysim_distance_to_closest_landmark_or_reward", false, kCore},  // 25
    {"mysim_local_school_grade", false, kCore},                      // 26
    {"mysim_local_hospital_grade", false, kCore},                    // 27
    {"mysim_dispatch_tool", false, kCore},                           // 28
    {"event_disaster_distance", false, kCore},                       // 29
    // 30: the original, so its draws use the generator slot 9 returns (AD8)
    // and no second generator mirrors it.
    {"random_chance", false, Direct::LuaRandomChance},               // 30
    {"nd_cancel", false, kCore},                                     // 31
    {"reward_instance_count", false, kCore},                         // 32
    {"unlock_reward_building", false, kCore},                        // 33
    {"get_reward_building_state", false, kCore},                     // 34
    {"is_tutorial", false, kCore},                                   // 35
    {"_combine_guids", true, kCore},                                 // 36
    {"_dofile_from_resource", true, Direct::LuaDofileFromResource},  // 37 (AD10)
    {"print", true, Direct::LuaPrint},                               // 38 (AD7)
    {"tool_plop_network", false, Direct::LuaToolPlopNetwork},        // 39 (AD5)
    {"tool_plop_zone", false, Direct::LuaToolPlopZone},              // 40
    {"tool_plop_building", false, Direct::LuaToolPlopBuilding},      // 41
    {"tool_plop_flora", false, Direct::LuaToolPlopFlora},            // 42
    {"tool_button", false, Direct::LuaToolButton},                   // 43
    {"window_budget", false, Direct::LuaWindowBudget},               // 44
    {"window_map", false, Direct::LuaWindowMap},                     // 45
    {"window_query", false, Direct::LuaWindowQuery},                 // 46
    {"window_graph", false, Direct::LuaWindowGraph},                 // 47
};
constexpr size_t kApiCount = sizeof(kApi) / sizeof(kApi[0]);
static_assert(kApiCount == 47, "advisors.md §8.2: 47 entries");

// §12.1 slot 11 steps 1-2 and the two text cuts of steps 3-4.
struct AdvisorLink {
    bool parsed = false;  // sscanf("%08x%08x") returned 2
    uint32_t advisorID = 0;
    uint32_t sessionID = 0;
    std::string text;  // the trimmed link
    // step 3 applies (the advisor and advice lookups still decide)
    bool TriesAdvice() const { return advisorID != 0x1D && advisorID != 0 && sessionID != 0; }
    // step 3: the rightmost len - 16 characters
    std::string AdviceText() const { return text.size() > 16 ? text.substr(16) : std::string(); }
    // step 4: from character 16 for 0x1D, from 8 otherwise [quirk]
    std::string LuaChunk() const {
        const size_t from = advisorID == 0x1D ? 16 : 8;
        return text.size() > from ? text.substr(from) : std::string();
    }
};

// [choice] "trim" = ASCII whitespace off both ends (the spec names no set).
inline AdvisorLink ParseAdvisorLink(const char* data, size_t len) {
    AdvisorLink l;
    std::string t(data ? data : "", data ? len : 0);
    size_t b = 0, e = t.size();
    while (b < e && std::isspace(static_cast<unsigned char>(t[b]))) ++b;
    while (e > b && std::isspace(static_cast<unsigned char>(t[e - 1]))) --e;
    l.text = t.substr(b, e - b);
    unsigned int a = 0, s = 0;
#if defined(_MSC_VER)
    l.parsed = sscanf_s(l.text.c_str(), "%08x%08x", &a, &s) == 2;
#else
    l.parsed = std::sscanf(l.text.c_str(), "%08x%08x", &a, &s) == 2;
#endif
    l.advisorID = a;
    l.sessionID = s;
    return l;
}

// §9 / §12.1 slot 9 (AD8): the original random_chance calls the non-virtual
// RandomDoubleRangeUniform on this, which reads and writes only the dword at
// +4 (foundations §6.1 `mul`). [choice] +0 holds no vtable (nothing reads it,
// AD8); +8 is `lcg`, read by no caller.
struct RZRandomShape {
    const void* vtable;
    uint32_t mul;  // +4: the double-path state
    uint32_t lcg;  // +8: the integer-path state
};
static_assert(offsetof(RZRandomShape, mul) == 4 || sizeof(void*) != 4, "AD8: the state at +4");

// foundations §6.1 Seed(0xFFFFFFFF): time(NULL) XOR ((c << 16) | c), c = clock().
inline uint32_t WallClockSeed(uint32_t timeNow, uint32_t clockNow) {
    return timeNow ^ ((clockNow << 16) | clockNow);
}

// A thiscall through a vtable offset (the __fastcall form with an unused EDX; x86).
template <typename R, typename... A>
R VCall(const void* obj, uint32_t offset, A... args) {
    using Fn = R(__fastcall*)(const void*, void*, A...);
    const uintptr_t* vt = *static_cast<const uintptr_t* const*>(obj);
    return reinterpret_cast<Fn>(vt[offset / sizeof(uintptr_t)])(obj, nullptr, args...);
}

// §5.2 (AD9): a cheat notification is a message 2 of type 0x230E27AC; the ID is
// data1 (GetData1 +0x24), the typed line data2 as a borrowed cIGZString*
// (GetVoid2 +0x3C), read with no null test as the original.
constexpr uint32_t kCheatMessage = 0x230E27AC;
struct CheatPayload {
    uint32_t id;
    const void* text;  // cIGZString*
};
inline CheatPayload ReadCheatPayload(const void* message2) {
    return {static_cast<uint32_t>(VCall<int32_t>(message2, 0x24)), VCall<const void*>(message2, 0x3C)};
}

// §4.2 step 2 (AD10): plugin directory first, then the SKU-specific one (the
// later file wins); `advisors\` appended to each; the data directory unused.
// AD11: both values are taken as the app returns them.
struct ReloadDirectories {
    std::string first, second;
};
inline ReloadDirectories AdvisorReloadDirectories(const std::string& pluginDir, const std::string& skuDir) {
    return {pluginDir + "advisors\\", skuDir + "advisors\\"};
}

}  // namespace advisors
}  // namespace fc4
