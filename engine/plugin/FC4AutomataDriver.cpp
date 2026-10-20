// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR
// PURPOSE. See the GNU General Public License for more details. You should have
// received a copy of the GNU General Public License along with this program; if
// not, see <https://www.gnu.org/licenses/>.

// The automata layer's riot and fire driver (FC4AutomataDriver.h).
#include "FC4AutomataDriver.h"

#include "FC4AutomataHost.h"
#include "FC4Log.h"

#include "cISC4City.h"

#include <cstdint>
#include <map>

#include "fc4.h"

namespace fc4 {
namespace {

// One city's bridge state. The riot core holds its leaders' automata with a
// reference (the 8.1 start's step 4); the fire core holds none.
struct Driver {
    AutomataHostContext hostCtx{};  // city borrowed for the city session
    fc4_automata_riot* riot = nullptr;
    fc4_automata_fire* fire = nullptr;
};

Driver* g_driver = nullptr;  // the one city (main thread only)

// The leader index of an automaton token: the cores' leader record is by
// index, the disaster core's events are by automaton.
int32_t LeaderIndexOf(Driver& d, uint64_t automaton) {
    if (d.riot == nullptr) return -1;
    const uint32_t n = fc4_automata_riot_count(d.riot);
    for (uint32_t i = 0; i < n; ++i) {
        fc4_automata_riot_leader l{};
        if (fc4_automata_riot_get_leader(d.riot, i, &l) && l.automaton == automaton) {
            return static_cast<int32_t>(i);
        }
    }
    return -1;
}

}  // namespace

void AutomataDriverCityLoaded(cISC4City* city) {
    if (g_driver != nullptr) {
        AutomataDriverCityShutdown();
    }
    if (city == nullptr) return;
    auto* d = new Driver();
    d->hostCtx.city = city;
    const fc4_automata_riot_host rh = MakeAutomataRiotHost(&d->hostCtx);
    const fc4_automata_fire_host fh = MakeAutomataFireHost(&d->hostCtx);
    d->riot = fc4_automata_riot_create(&rh);
    d->fire = fc4_automata_fire_create(&fh);
    g_driver = d;
    Log("automata driver: on (city %p): the disaster layer's riots and fires drive fc4_automata_riot / _fire "
        "(automata.md 8); the fire bridge waits for Q27",
        static_cast<void*>(city));
}

void AutomataDriverCityShutdown() {
    if (g_driver == nullptr) return;
    if (g_driver->riot != nullptr) {
        fc4_automata_riot_destroy(g_driver->riot);
        g_driver->riot = nullptr;
    }
    if (g_driver->fire != nullptr) {
        fc4_automata_fire_destroy(g_driver->fire);
        g_driver->fire = nullptr;
    }
    delete g_driver;
    g_driver = nullptr;
}

uint64_t AutomataDriverRiotAdd(int32_t x, int32_t z) {
    if (g_driver == nullptr || g_driver->riot == nullptr) return 0;
    const int32_t index = fc4_automata_riot_add(g_driver->riot, x, z);
    if (index < 0) return 0;
    fc4_automata_riot_leader l{};
    if (!fc4_automata_riot_get_leader(g_driver->riot, static_cast<uint32_t>(index), &l)) return 0;
    return l.automaton;
}

bool AutomataDriverRiotSetStage(uint64_t automaton, int32_t stage) {
    if (g_driver == nullptr || g_driver->riot == nullptr) return false;
    const int32_t index = LeaderIndexOf(*g_driver, automaton);
    if (index < 0) return false;
    return fc4_automata_riot_set_stage(g_driver->riot, static_cast<uint32_t>(index), stage);
}

bool AutomataDriverRiotEnd(uint64_t automaton) {
    if (g_driver == nullptr || g_driver->riot == nullptr) return false;
    const int32_t index = LeaderIndexOf(*g_driver, automaton);
    if (index < 0) return false;
    return fc4_automata_riot_end(g_driver->riot, static_cast<uint32_t>(index));
}

}  // namespace fc4
