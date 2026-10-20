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

// The automata layer's host tables: the game calls the riot leader hooks
// (docs/specs/automata.md 8.1) and the fire occupant hooks (8.2) make, and
// the U-Drive-It pause / resume of the four managers (9).
//
// Each fc4_automata_riot_host / fc4_automata_fire_host callback is one call
// into the game, in the order the core makes it:
//
//   riot  nearest_road_tile        the clockwise spiral of disasters.md 6.4
//                                  step 1 over the traffic network map
//   riot  leader_cap_reached       leaders >= the maximum (6.1; 10 shipped)
//   riot  create_automaton         pedestrian manager (city slot 44) slot 15
//   riot  add_ref / release        automaton AddRef / Release (slot 2)
//   riot  set_state                automaton slot 51 SetState
//   riot  set_visible              automaton slot 9
//   riot  as_occupant              automaton slot 5
//   riot  register_location        disaster layer (city slot 72) slot 13
//   riot  add_controller_from_template_name   controller manager (city slot
//                                             47) slot 5
//   riot  remove_controllers       controller manager slot 14
//   riot  unregister_location      disaster layer slot 15
//   fire  occupant_type            the occupant's GetType
//   fire  add_controllers_from_occupant_group_name  controller manager slot 10
//   fire  remove_controllers       controller manager slot 14
//
// Tokens are game object pointers (TokenOf): an automaton is its
// cISC4Automaton*, an occupant its cISC4Occupant*. The tables take no
// reference of their own; the core's own add_ref / release pair is passed
// straight through (HANDOFF 5.1b).
#pragma once

#include "fc4.h"

#include <cstdint>

class cISC4City;

namespace fc4 {

// The game services the callbacks need, borrowed for the city session.
struct AutomataHostContext {
    cISC4City* city = nullptr;
    // The maximum leaders of disasters.md 6.1 (the shipped tuning is 10).
    uint32_t maxLeaders = 10;
    // Counts for the log.
    uint32_t created = 0, refused = 0, stateCalls = 0, stageCalls = 0, fireCalls = 0;
};

// The two host tables over `ctx` (which must outlive the core).
fc4_automata_riot_host MakeAutomataRiotHost(AutomataHostContext* ctx);
fc4_automata_fire_host MakeAutomataFireHost(AutomataHostContext* ctx);

// U-Drive-It (9): the city situation manager pauses all four managers when a
// mission activates and resumes them when its start countdown ends.
void AutomataPauseAll(cISC4City* city);
void AutomataResumeAll(cISC4City* city);

}  // namespace fc4
