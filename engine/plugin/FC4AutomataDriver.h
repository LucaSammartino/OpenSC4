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

// The automata layer's riot and fire driver (docs/specs/automata.md 8): the
// bridge from the disaster layer's events to the fc4_automata riot and fire
// cores. The disaster layer (the original's, or ours under `enable =
// disasters`) runs the episodes; the automata layer's part of them is the
// calls of 8.1 (the riot leader's start, stage rise and end) and 8.2 (the
// fire occupant's SetTarget and Shutdown), which fc4_automata_riot_* and
// fc4_automata_fire_* make through their host tables (FC4AutomataHost.h).
//
// The bridge is live-side only: with the disasters shadow (the original
// layer runs unchanged and the mirror follows) the original layer makes its
// own 8.1 / 8.2 calls, and driving ours too would double every one.
//
// Call order per riot (disasters.md 6.2-6.5 against automata.md 8.1):
//   create_riot_leader(x, z)   -> FC4AutomataRiotAdd(x, z): the whole 8.1
//                                 start chain (the road tile, the cap,
//                                 CreateAutomaton on the pedestrian manager,
//                                 AddRef, SetState(8, ...), SetVisible, the
//                                 location); returns the leader's automaton.
//   send_riot_message(a, s)    -> FC4AutomataRiotSetStage(a, s): the stage
//                                 rise's AddControllerFromTemplateName.
//   end_riot_leader(a)         -> FC4AutomataRiotEnd(a): SetState(1),
//                                 RemoveControllers, the location, Release.
// The fire occupant's SetTarget / Shutdown (8.2) ride on the fire occupant
// the layer creates; our live layer does not make one yet (Q27), so the
// fire bridge is wired but does not fire until Q27 lands.
#pragma once

#include <cstdint>

class cISC4City;

namespace fc4 {

// Per-city session (FC4Subsystems.cpp city init; released at city shutdown).
void AutomataDriverCityLoaded(cISC4City* city);
void AutomataDriverCityShutdown();

// The disaster host's riot events (live mode). Each returns what the
// disaster core expects: the created leader's automaton token, and true when
// the stage-rise / end call was made.
uint64_t AutomataDriverRiotAdd(int32_t x, int32_t z);
bool AutomataDriverRiotSetStage(uint64_t automaton, int32_t stage);
bool AutomataDriverRiotEnd(uint64_t automaton);

}  // namespace fc4
