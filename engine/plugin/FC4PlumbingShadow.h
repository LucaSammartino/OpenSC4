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

// The plumbing (water) shadow: a same-process differential of the plumbing
// simulator (tooling; no spec behaviour). Opt-in, off by default:
//
//   plumbing_shadow = 1          (env FC4_PLUMBING_SHADOW)
//   plumbing_shadow_cells = N    differing cells and list items shown per line
//                                (env FC4_PLUMBING_SHADOW_CELLS, default 8, 1..64)
//   plumbing_shadow_passes = N   passes replayed per game day (env
//                                FC4_PLUMBING_SHADOW_PASSES, default 4; 0 = every
//                                pass); the others are only followed
//
// It needs FreeCity4's simulator and the ORIGINAL plumbing simulator (the
// default: `plumbing` is not enabled). Our simulator delivers the clock to
// the original's three agents, and the shadow wraps their SimIdle, SimNewDay
// (one pass state each, plumbing.md §6) and SimNewMonth (§7) deliveries
// (cFC4Simulator::HostDeliver). The core (fc4_plumbing_shadow, fc4.h) does the
// work; this file gives it:
//   - a read-only host table over the city's services, borrowed for one
//     delivery (fc4::PlumbingReadOnlyHost: every write callback NULL, the
//     grid table geometry and get only);
//   - the original, read through its interface: GetWateredGrid (slot 7), the
//     counters of slots 12-23, GetWaterProducerInfo (24), each water
//     connection's GetVisitedThisCycle, and its record through its own Write
//     (cIGZSerializable) into a memory stream (references queried for the
//     grid, occupant, pipe and department interfaces).
// The shadow sees a pass start (a cell of the next working grid changes:
// state 2 ran), replays FreeCity4's pass on the original's record over
// private grid copies, compares at the original's swap (the published grid
// cell by cell, the producers, deal usage, water connections) and after its
// state 8 (totals, building counts, cost, demand, pipes, sources, budget
// items, ageing). A month compares the pipe funding (§7.1): the distressed
// count exactly, the random new bursts by range ("not exact").
//
// Not readable read-only, hence not compared (logged at game start): the
// pipes' watered states (a non-virtual call), redisplay, posts, history,
// line-item expenses, lot conditions, deal flags, the original's pipe
// condition (not saved) and pass state. Demand is an incremental sum kept by
// the inserts (§5.3): carried from the original's record, not recomputed.
//
// Log lines (always written, not the verbose channel; longer lines are split
// into "plumbing shadow (cont.)" lines):
//   plumbing shadow: ON (...) | requested ... but INACTIVE (...)   game start
//   plumbing shadow YYYY-MM-DD pass N: aligned=start|swap exact=1|0 ...;
//     grid WxH cells=.. differing=.. max|diff|=..; ...; mismatches=N
//   plumbing shadow YYYY-MM-DD month: ...; mismatches=N; months=M clean=C
//   plumbing shadow YYYY-MM-DD[ pass N]: INCOMPLETE (why); not compared
//
// Threads: main thread only (HostDeliver); the state below is not shared.
#pragma once

#include <cstdint>

class cIGZMessage2;
class cIGZMessageTarget2;
class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the settings and logs
// whether the shadow can run in this configuration.
void PlumbingShadowStartup(bool simulatorReplaced, bool plumbingReplaced);

// cFC4Simulator::HostDeliver, for every agent delivery: if the shadow is on
// and `target` is the original plumbing simulator's agent receiving SimIdle,
// SimNewDay or SimNewMonth, runs before / target->DoMessage(msg) / after,
// stores the agent's result in `finished` and returns true. Otherwise returns
// false and does nothing (the caller delivers).
bool PlumbingShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                           int64_t data1, bool* finished);

}  // namespace fc4

// Test seam op 22 (FC4TestSeam.h): a run of deliveries to a fake original,
// with one shadow for the whole run (a fresh one per call). Only the city's
// GetPlumbingSimulator (compiled slot 64) is called on `city`; the read-only
// host's services stay null (neutral reads).
struct FC4PlumbingShadowTestInput {
    cISC4City* city;
    cIGZMessageTarget2* target;  // the deliveries' target
    const uint32_t* types;       // per delivery: SimIdle 0x6695681A, SimNewDay 0x66956814, SimNewMonth 0x66956816
    const int32_t* julians;      // per delivery: data1
    uint32_t count;
    uint32_t passesPerDay;  // plumbing_shadow_passes for the run (0 = every pass)
    int32_t* results;       // out, may be null: per delivery, the counted differences of a comparison
                            // finished in it, -1 none, -3 not taken (the caller delivered it)
};
// Returns the sum of the counted differences of the comparisons finished
// (passes and months), -1 if the shadow took no delivery, -2 if it took some
// but finished no comparison.
intptr_t PlumbingShadowTestRun(const FC4PlumbingShadowTestInput* in);
