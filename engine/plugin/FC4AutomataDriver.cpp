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
#include "cISC4CitySituationManager.h"
#include "cIGZMessageServer2.h"
#include "cIGZMessageTarget2.h"
#include "cIGZMessage2.h"
#include "cIGZMessage2Standard.h"
#include "cIGZFrameWork.h"
#include "cRZCOMDllDirector.h"

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
    // The one fire occupant the disaster layer keeps (disasters.md 5.5: the
    // fire instance's list; the automata layer's 8.2 hook is per fire
    // occupant, so the last one created is the one SetTarget / Shutdown talk
    // about). Set by AutomataDriverFireSetTarget, cleared by Shutdown.
    bool fireTargetKnown = false;
    // U-Drive-It (9): paused by a mission activation, resumed when the city
    // situation manager's start countdown ends (see SituationTick).
    bool udrivePaused = false;
    // The countdown has been seen running since the pause (the pause comes
    // with the activation message, the countdown starts on the manager's next
    // animation tick).
    bool countdownSeen = false;
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
    const AutomataHostContext& h = g_driver->hostCtx;
    Log("automata driver: off: leaders created %u, refused %u, SetState calls %u, stage-rise calls %u, "
        "fire controller calls %u",
        h.created, h.refused, h.stateCalls, h.stageCalls, h.fireCalls);
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

// ---- fire (8.2) ---------------------------------------------------------------------------------------------

// The disaster layer's fire occupant (5.11, DS26; the live host makes one
// through GZCOM, FC4DisasterHost.cpp) carries the automata layer's part of a
// fire: its SetTarget gives a burning building the "fire_occupant" controllers
// and its Shutdown removes them again. The host hands the target here when it
// makes an occupant and calls the shutdown when it removes it; until the
// occupant lands (Q27 on this branch) the calls are wired but never fire.
void AutomataDriverFireSetTarget(uint64_t target) {
    if (g_driver == nullptr || g_driver->fire == nullptr) return;
    g_driver->fireTargetKnown = true;
    fc4_automata_fire_set_target(g_driver->fire, target);
}

void AutomataDriverFireShutdown() {
    if (g_driver == nullptr || g_driver->fire == nullptr) return;
    if (!g_driver->fireTargetKnown) return;  // 8.2: nothing is called without a target
    fc4_automata_fire_shutdown(g_driver->fire);
    g_driver->fireTargetKnown = false;
}

// ---- U-Drive-It pause / resume (9) ---------------------------------------------------------------------------

// The city situation manager pauses the four managers when a mission
// activates and resumes them when its 4000 ms start countdown ends
// (city-situation.md 4.4 steps 1-2). The automata layer supplies the calls
// (manager slot 9 Pause / slot 10 Resume, automata.md 9); the manager runs
// them from its own animation tick.
//
// The activation is observable as the manager's own activation message
// 0xABBBCA64 (city-situation.md 3: sent, data1 = 1 when a mission becomes
// active or a joyride starts, 0 when one is deactivated): ActivateSituation
// sends it with data1 = 1 and sets justActivated, and the justActivated tick
// is the one that pauses. DeactivateSituation sends data1 = 0 while the
// countdown it does not touch keeps running (CS6), so the resume is not the
// deactivation: it is the countdown running out.
//
// So: pause on 0xABBBCA64 data1 = 1; resume when the countdown has been seen
// running and is no longer in progress (the manager's slot 32
// CountdownIsInProgress, city-situation.md 8) - which is exactly the tick
// that ends it - or when the mission is no longer active with no countdown
// running (a mission that ended before its countdown ever started). The
// plugin's observer calls AutomataDriverSituationMessage / _Tick; the
// underlying game calls are fc4::AutomataPauseAll / AutomataResumeAll
// (FC4AutomataHost.h), which reach our managers when `enable = automata` and
// the original's when not, in either case the slots the spec names.

constexpr uint32_t kMsgSituationActive = 0xABBBCA64;

void AutomataDriverSituationMessage(int32_t data1) {
    if (g_driver == nullptr) return;
    if (data1 != 0) {
        if (!g_driver->udrivePaused) {
            g_driver->udrivePaused = true;
            g_driver->countdownSeen = false;
            AutomataPauseAll(g_driver->hostCtx.city);  // slot 9 on each of the four
        }
    } else if (g_driver->udrivePaused && g_driver->countdownSeen) {
        // A deactivation while the countdown still runs: the pause stays
        // until the countdown ends (CS6: DeactivateSituation does not
        // unfreeze; the automata are unfrozen when the countdown runs out).
    }
}

void AutomataDriverSituationTick() {
    if (g_driver == nullptr || !g_driver->udrivePaused) return;
    cISC4City* city = g_driver->hostCtx.city;
    cISC4CitySituationManager* m = city != nullptr ? city->GetCitySituationManager() : nullptr;  // city slot 49
    if (m == nullptr) return;
    const bool countdown = m->CountdownIsInProgress();  // slot 32 (city-situation.md 8)
    if (countdown) {
        g_driver->countdownSeen = true;
        return;
    }
    // No countdown running. Resume when one ran out since the pause, or when
    // the mission ended without one ever starting (the brief window between
    // the activation message and the justActivated tick has the countdown
    // clear but the mission active, and does not resume).
    if (g_driver->countdownSeen) {
        g_driver->udrivePaused = false;
        g_driver->countdownSeen = false;
        AutomataResumeAll(city);  // slot 10 on each of the four
    }
}

}  // namespace fc4
