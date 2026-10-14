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

// The residential shadow: a same-process differential of the residential
// simulator (tooling; no spec behaviour). Opt-in, off by default:
//
//   residential_shadow = 1          (env FC4_RESIDENTIAL_SHADOW)
//   residential_shadow_cells = N    differing cells listed per grid and line
//                                   (env FC4_RESIDENTIAL_SHADOW_CELLS, default 4, 1..64)
//
// It needs FreeCity4's simulator and the ORIGINAL residential simulator (the
// default: `residential` is not enabled). Our simulator delivers the clock to
// the original's agent, and the shadow wraps each of its SimNewDay deliveries
// (cFC4Simulator::HostDeliver). The moment is the delivery itself because the
// original runs ONE state of its monthly cycle per day (residential.md §5.1)
// and a time-sliced state (1, 2, 4) can stop part-way (§5.2): only a wrap of
// each delivery sees the inputs that state read and where it stopped.
//   1. before: the original's own Write (cIGZSerializable) into a capture
//      stream (bytes, and each reference queried for its cISC4SimGrid,
//      cISC4Occupant and cISC4DepartmentBudget identities);
//      fc4_residential_shadow_begin copies the 25 live grids the record
//      refers to, loads the record into a private core over the copies and
//      runs FreeCity4's daily state, reading every input through a read-only
//      host table. States 1, 2 and 4 read nothing the delivery can change,
//      so they run at the finish with the original's progress; every other
//      state runs here, reading the game as the original is about to;
//   2. the original's DoMessage, unchanged;
//   3. after: the record again and the original's population, global EQ /
//      HQ and LE (slots 16, 32, 33, 34); fc4_residential_shadow_finish
//      compares the 25 grids cell by cell, the saved fields, the occupant
//      properties and local funding, and the four scalars.
// A day in state 10 (waiting for SimNewMonth) is not compared and not logged.
//
// What is not readable read-only, hence not compared, is logged once at game
// start (fc4_residential_shadow_describe).
//
// Log lines (always written; a longer line is split into "residential shadow
// (cont.)" lines), one per delivery outside state 10:
//   residential shadow YYYY-MM-DD state N (name): ...; mismatches=N
//   residential shadow YYYY-MM-DD ...: INCOMPLETE (why); not compared
// and after the delivery that ends a monthly cycle (state 9 -> 10) the cycle line:
//   residential shadow cycle to YYYY-MM-DD: deliveries=.. compared=.. clean=.. incomplete=..; cells=..
//     mismatches=..; grids with differences: ...
#pragma once

#include <cstdint>

class cIGZMessage2;
class cIGZMessageTarget2;
class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the settings and logs
// whether the shadow can run in this configuration.
void ResidentialShadowStartup(bool simulatorReplaced, bool residentialReplaced);

// cFC4Simulator::HostDeliver, for every agent delivery: if the shadow is on
// and `target` is the original residential simulator's agent receiving
// SimNewDay, runs before / target->DoMessage(msg) / after, stores the
// agent's result in `finished` and returns true. Otherwise returns false and
// does nothing (the caller delivers).
bool ResidentialShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                              int64_t data1, bool* finished);

}  // namespace fc4

// Test seam op 25 (FC4TestSeam.h): one shadowed delivery to a fake original.
// Only the city's GetResidentialSimulator (compiled slot 68) is called on
// `city`; the read callbacks see no city (neutral reads).
struct FC4ResidentialShadowTestInput {
    cISC4City* city;
    cIGZMessageTarget2* target;  // the delivery's target
    uint32_t msgType;            // SimNewDay 0x66956814 (anything else is not taken)
    int32_t julian;              // data1
};
// Returns the delivery's difference count, -1 if the shadow did not take the
// delivery, -2 if it delivered but compared nothing.
intptr_t ResidentialShadowTestDeliver(const FC4ResidentialShadowTestInput* in);
