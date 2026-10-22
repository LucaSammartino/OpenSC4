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

// FC4AdvisorApi.h against advisors.md §17 cases 23-27 (the parts the plugin
// owns; the native draws and the in-game reload are checked in the game).

#include "FC4AdvisorApi.h"

#include <cstdio>
#include <cstring>
#include <set>
#include <string>

namespace {

int g_failures = 0;
void Check(bool ok, const char* what) {
    if (!ok) {
        std::printf("FAIL: %s\n", what);
        ++g_failures;
    }
}

using namespace fc4::advisors;

// 23. Registration order (§8.2).
void RegistrationOrder() {
    static const char* const kExpected[47] = {
        "advice_info", "trend_value", "trend_slope", "trend_count", "camera_jump", "camera_zoom",
        "ordinance_is_on", "ordinance_set_off", "ordinance_set_on", "ordinance_is_available", "ordinance_name",
        "ordinance_description", "ordinance_enact_fee", "ordinance_repeal_fee", "ordinance_monthly_cost",
        "difficulty_level", "event_feedback", "expire_advice", "retire_advice", "advice_subject", "trigger_event",
        "building_is_group", "event_building_distance", "mysim_distance_to_closest_building",
        "mysim_distance_to_closest_landmark_or_reward", "mysim_local_school_grade", "mysim_local_hospital_grade",
        "mysim_dispatch_tool", "event_disaster_distance", "random_chance", "nd_cancel", "reward_instance_count",
        "unlock_reward_building", "get_reward_building_state", "is_tutorial", "_combine_guids",
        "_dofile_from_resource", "print", "tool_plop_network", "tool_plop_zone", "tool_plop_building",
        "tool_plop_flora", "tool_button", "window_budget", "window_map", "window_query", "window_graph"};
    Check(kApiCount == 47, "23: 47 entries");
    std::set<std::string> names;
    const char* lastGlobal = nullptr;
    const char* lastGame = nullptr;
    for (size_t i = 0; i < kApiCount; ++i) {
        Check(std::strcmp(kApi[i].name, kExpected[i]) == 0, "23: vector order");
        Check(kApi[i].global == (i >= 35 && i <= 37), "23: only entries 36-38 are global");
        names.insert(kApi[i].name);
        (kApi[i].global ? lastGlobal : lastGame) = kApi[i].name;
    }
    Check(names.size() == 47, "23: no duplicate names");
    Check(lastGlobal && std::strcmp(lastGlobal, "print") == 0, "23: _tf_ = print after register_api(true)");
    Check(lastGame && std::strcmp(lastGame, "window_graph") == 0, "23: _tf_ = window_graph after register_api(false)");
    Check(kApi[29].native == fc4::compat::Direct::LuaRandomChance, "24: random_chance is the original (shared generator)");
    Check(kApi[36].native == fc4::compat::Direct::LuaDofileFromResource, "23: _dofile_from_resource is the original");
    Check(kApi[37].native == fc4::compat::Direct::LuaPrint, "23: print is the original");
}

// 24. GetRandom (§12.1): the state the original draws on sits at +4.
void RandomShape() {
    Check(offsetof(RZRandomShape, mul) == 4, "24: the generator state at P+4");
    Check(WallClockSeed(0x12345678u, 0x9ABCu) == (0x12345678u ^ 0x9ABC9ABCu), "24: Seed(-1) wall-clock form");
}

// 25. DoLink (§12.1): the text cuts.
void Links() {
    const char* t1 = "0000001D00000000x=1";
    AdvisorLink l = ParseAdvisorLink(t1, std::strlen(t1));
    Check(l.parsed && l.advisorID == 0x1D && l.sessionID == 0, "25: 0x1D link parses");
    Check(!l.TriesAdvice() && l.LuaChunk() == "x=1", "25: 0x1D runs the text from character 16");
    l = ParseAdvisorLink("00000000", 8);
    Check(!l.parsed, "25: one field returns false");
    const char* t2 = "  0000000500000007hello world \t";
    l = ParseAdvisorLink(t2, std::strlen(t2));
    Check(l.parsed && l.TriesAdvice() && l.AdviceText() == "hello world", "25: advice link, text after character 16");
    Check(l.LuaChunk() == "00000007hello world", "25: unknown advisor runs the text from character 8");
    const char* t3 = "0000000500000000y=2";
    l = ParseAdvisorLink(t3, std::strlen(t3));
    Check(l.parsed && !l.TriesAdvice() && l.LuaChunk() == "00000000y=2", "25: session 0 falls back from character 8");
}

// 26. Cheat payload (§5.2): data1 at +0x24, void2 at +0x3C.
uint32_t g_text = 0;
int32_t __fastcall FakeData1(const void*, void*) { return static_cast<int32_t>(0xA919D25E); }
const void* __fastcall FakeVoid2(const void*, void*) { return &g_text; }
void CheatPayloadSlots() {
    const void* vt[18] = {};
    vt[0x24 / 4] = reinterpret_cast<const void*>(&FakeData1);
    vt[0x3C / 4] = reinterpret_cast<const void*>(&FakeVoid2);
    struct {
        const void* const* vtable;
    } msg{vt};
    const CheatPayload c = ReadCheatPayload(&msg);
    Check(c.id == 0xA919D25E && c.text == &g_text, "26: cheat ID = data1, text = void2");
    Check(kCheatMessage == 0x230E27AC, "26: cheat message type");
}

// 27. Reload directories (§4.2): plugin first, SKU-specific last (it wins).
void ReloadDirs() {
    const ReloadDirectories d = AdvisorReloadDirectories("P\\Plugins\\", "S\\");
    Check(d.first == "P\\Plugins\\advisors\\" && d.second == "S\\advisors\\", "27: plugin then SKU, advisors\\ appended");
}

}  // namespace

int main() {
    RegistrationOrder();
    RandomShape();
    Links();
    CheatPayloadSlots();
    ReloadDirs();
    if (g_failures == 0) std::printf("advisor_api_test: all passed\n");
    return g_failures == 0 ? 0 : 1;
}
