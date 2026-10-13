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

// The pollution grid shadow: a same-process differential of the pollution
// simulator (tooling; no spec behaviour). Opt-in, off by default:
//
//   pollution_shadow = 1         (env FC4_POLLUTION_SHADOW)
//   pollution_shadow_cells = N   differing cells listed per grid and line
//                                (env FC4_POLLUTION_SHADOW_CELLS, default 4, 1..64)
//
// It needs FreeCity4's simulator and the ORIGINAL pollution simulator (the
// default: `pollution` is not enabled). Our simulator delivers the clock to
// the original's agent, and the shadow wraps its SimNewDay and SimNewMonth
// deliveries (cFC4Simulator::HostDeliver):
//   1. before: the original's own Write (cIGZSerializable) into a capture
//      stream (bytes, and each reference queried for the grid, occupant and
//      department interfaces); fc4_pollution_shadow_begin copies the eight
//      live grids that record refers to and runs FreeCity4's day or month on
//      the copies, reading every input through a read-only host table (city
//      geometry, tuning, terrain, zones, air traffic, ordinances, deals,
//      power, population, landfill, occupant properties, position and
//      bounding box, department funding, budget items) and the grid table's
//      geometry and get only. Nothing is written to the game;
//   2. the original's DoMessage, unchanged;
//   3. after: the record again; fc4_pollution_shadow_finish compares our
//      eight grids with the live ones cell by cell, the saved fields, and
//      the occupant properties and deal usage.
//
// Not readable read-only, hence not compared (logged at game start): the
// radiation sum (not saved), the posted messages, history and budget values,
// visual effects and the worst tracts. The record is the original's own
// Write: calling it outside a save is the only way to read its state.
//
// Log lines (always written, not the verbose channel; longer lines are split
// into "pollution shadow (cont.)" lines):
//   pollution shadow YYYY-MM-DD day|month: ...      a delivery with a
//     difference (at most 8 a month), a rerun, or INCOMPLETE (why)
//   pollution shadow month to YYYY-MM-DD: deliveries=N (days D, months M)
//     clean=C incomplete=I reruns=R uncached=U; cells=X mismatches=Y;
//     airSrc a/b max|d|=.. (per grid); first differing cells: ...; fields: ...;
//     props compared=.. mismatched=..; deals compared=.. mismatched=..
#pragma once

#include <cstdint>

class cIGZMessage2;
class cIGZMessageTarget2;
class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the settings and logs
// whether the shadow can run in this configuration.
void PollutionShadowStartup(bool simulatorReplaced, bool pollutionReplaced);

// cFC4Simulator::HostDeliver, for every agent delivery: if the shadow is on
// and `target` is the original pollution simulator's agent receiving
// SimNewDay or SimNewMonth, runs before / target->DoMessage(msg) / after,
// stores the agent's result in `finished` and returns true. Otherwise returns
// false and does nothing (the caller delivers).
bool PollutionShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                            int64_t data1, bool* finished);

}  // namespace fc4

// Test seam op 16 (FC4TestSeam.h): one shadowed delivery to a fake original.
// Only the city's GetPollutionSimulator (compiled slot 66) is called on
// `city`; the other services stay null (neutral reads).
struct FC4PollutionShadowTestInput {
    cISC4City* city;
    cIGZMessageTarget2* target;  // the delivery's target
    uint32_t msgType;            // SimNewDay 0x66956814 or SimNewMonth 0x66956816
    int32_t julian;              // data1
};
// Returns the delivery's difference count, -1 if the shadow did not take the
// delivery, -2 if it delivered but compared nothing.
intptr_t PollutionShadowTestDeliver(const FC4PollutionShadowTestInput* in);
