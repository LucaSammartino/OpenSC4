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

// The power shadow: a same-process differential of the power simulator
// (tooling; no spec behaviour). Opt-in, off by default:
//
//   power_shadow = 1         (env FC4_POWER_SHADOW)
//   power_shadow_cells = N   differing cells listed per line
//                            (env FC4_POWER_SHADOW_CELLS, default 8, 1..64)
//   power_shadow_lines = N   cycle lines with a difference per month
//                            (env FC4_POWER_SHADOW_LINES, default 32; -1 = all)
//
// It needs FreeCity4's simulator and the ORIGINAL power simulator (the
// default: `power` is not enabled). Our simulator delivers the clock to the
// original's agent, and the shadow wraps (cFC4Simulator::HostDeliver) every
// SimNewMonth and each SimNewDay / SimIdle that runs one step of the
// original's DoIdle (power.md §6: 4 -> 0 -> 1 -> 2 -> 3, one per delivery).
// A SimNewDay / SimIdle while the original is idle (state 4 without a
// recompute request after the last wrapped delivery) passes straight through
// unless one of its §5 messages arrived since (a static message target counts
// them):
//   1. before: the original's own Write (cIGZSerializable) into a capture
//      stream (bytes, and each reference queried for the occupant, pole, grid
//      and department interfaces, held until the delivery ends) and its
//      unsaved plant counts (slots 27-29); fc4_power_shadow_begin copies the
//      live powered grid the record refers to and runs FreeCity4's step on
//      the record, reading every input through a read-only host table
//      (fc4::PowerReadOnlyHost: city geometry, tuning, the ordinance factor,
//      occupants and properties, poles, lines, network occupants, the occupant
//      query, zones, lots, deals, neighbour connections, departments, line and
//      budget items) and the grid table's geometry and get only. Nothing is
//      written to the game;
//   2. the original's DoMessage, unchanged;
//   3. after: the record and the counts again; fc4_power_shadow_finish
//      compares (power_shadow.rs).
//
// The moment: the end of each recompute cycle (state 3, §6.6: the published
// totals) is where every quantity of the task is defined at once, so the
// cycle line comes then; each step of the cycle is compared on its own as
// well (the line names the steps with a difference). The month (§7.5) has its
// own line, and the bookkeeping line compares the original's running counts
// with a from-scratch §5.1 insertion of the live occupants.
//
// Log lines (always written, not the verbose channel; longer lines are split
// into "power shadow (cont.)" lines), each ending in "mismatches=N":
//   power shadow: ON (...) | requested but INACTIVE (...)        game start
//   power shadow YYYY-MM-DD cycle: steps 4>0 0 1 2 3 on WxH cells; grid N
//     cells compared, max|d|=..; first differing cells: ...; production=o/b
//     consumption=o/b import=o/b export=o/b demand=o/b requirement=o/b
//     consumers=o/b powered=o/b unpowered=o/b plants=o/b aged=o/b
//     distressed=o/b overworked=o/b deficit=o/b poles=o/b broken=o/b; ...;
//     not_exact=..; mismatches=N                   (orig/ours) per cycle
//   power shadow YYYY-MM-DD month: N cells compared; <scalars>; ...;
//     not_exact=..; mismatches=N                                 per month
//   power shadow YYYY-MM-DD bookkeeping: ...; mismatches=N       per month
//   power shadow month to YYYY-MM-DD: deliveries=.. steps=.. ...;
//     not_exact=..; mismatches=N                                 per month
#pragma once

#include <cstdint>

class cIGZMessage2;
class cIGZMessageTarget2;
class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the settings and logs
// whether the shadow can run in this configuration.
void PowerShadowStartup(bool simulatorReplaced, bool powerReplaced);

// cFC4Simulator::HostDeliver, for every agent delivery: if the shadow is on
// and `target` is the original power simulator's agent receiving SimNewDay,
// SimIdle or SimNewMonth (and the delivery is wanted, see above), runs before
// / target->DoMessage(msg) / after, stores the agent's result in `finished`
// and returns true. Otherwise returns false and does nothing (the caller
// delivers).
bool PowerShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                        int64_t data1, bool* finished);

}  // namespace fc4

// Test seam op 21 (FC4TestSeam.h): one shadowed delivery to a fake original.
// Only the city's GetPowerSimulator (compiled slot 67) and its cell widths /
// counts (91-94) are called on `city`; the other services stay null (neutral
// reads: no occupants, zones, lots, deals or budget; ordinance factor 1).
// `fresh`: a new shadow for this call (else the previous call's shadow, its
// mirror and its cycle, are kept).
struct FC4PowerShadowTestInput {
    cISC4City* city;
    cIGZMessageTarget2* target;  // the agent the delivery goes to
    uint32_t msgType;            // SimNewDay, SimIdle or SimNewMonth
    int32_t julian;              // data1
    uint32_t fresh;
};
// Returns the delivery's difference count, -1 if the shadow did not take the
// delivery (left to the caller), -2 if it took it and compared nothing, -3 if
// it passed it through uncompared (the original idle and no event).
intptr_t PowerShadowTestDeliver(const FC4PowerShadowTestInput* in);
