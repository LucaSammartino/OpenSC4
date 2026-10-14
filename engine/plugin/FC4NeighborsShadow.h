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

// The neighbours shadow: a same-process differential of the neighbours
// simulator (tooling; no spec behaviour). Opt-in, off by default:
//
//   neighbours_shadow = 1           (env FC4_NEIGHBOURS_SHADOW)
//
// It needs FreeCity4's simulator and the ORIGINAL neighbours simulator (the
// default: `neighbors` is not enabled). Our simulator delivers the clock to
// the original's month agent, and the shadow wraps each of its SimNewMonth
// deliveries (cFC4Simulator::HostDeliver):
//   1. before: reads, through the game's interfaces only, what the original's
//      month reads (neighbors.md §7.3, §8.2): its connections
//      (GetAllConnections) and deals (FindDealsAll, each through
//      cISC4NeighborDeal), the current regional city and its adjacent cities
//      (GetAdjacentCities: position, size, established, budget, income and the
//      five statistics of commodities 1-3) with their connections, the city's
//      budget, utility and landfill figures, the date and the tuning exemplar;
//      builds the shadow (fc4_neighbors_shadow_create) and runs FreeCity4's
//      month on it (all of it in memory: nothing reaches the game);
//   2. the original's DoMessage, unchanged;
//   3. after: the original's connections and deals again, its totals (slots
//      32 / 33) and the cities; compares
//        - the connection list: count, the objects in order, and each one's
//          type, cell, other-side cell and flags (our month changes none, so
//          the original's after-state is compared with the state ours started
//          from);
//        - each deal (index by index): type, both region cells, flags (the
//          state), cost, amount, the other amounts and the four dates;
//        - the income and expense totals (low 32 bits, as slots 32 / 33);
//        - the commodity statistics of every city of the snapshot: budget,
//          income, exported / imported / produced / demanded / cost per unit
//          (relative tolerance 1e-6; NaN equals NaN).
// A month without deals still compares the connections and the statistics.
//
// What is not readable read-only, hence assumed or not compared, is logged
// once at game start.
//
// Log lines (always written; a longer line is split into "neighbours shadow
// (cont.)" lines), one per month:
//   neighbours shadow month to YYYY-MM-DD: connections=.. deals=.. income=../.. expense=../.. cities=..;
//     ours: posts=.. sends=.. ...; mismatches=N[; differences: ...]
//   neighbours shadow month to YYYY-MM-DD: INCOMPLETE (why); not compared
#pragma once

#include <cstdint>

class cIGZMessage2;
class cIGZMessageTarget2;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the settings and logs
// whether the shadow can run in this configuration.
void NeighborsShadowStartup(bool simulatorReplaced, bool neighborsReplaced);

// cFC4Simulator::HostDeliver, for every agent delivery: if the shadow is on
// and `target` is the original neighbours simulator's agent receiving
// SimNewMonth, runs before / target->DoMessage(msg) / after, stores the
// agent's result in `finished` and returns true. Otherwise returns false and
// does nothing (the caller delivers).
bool NeighborsShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                            int64_t data1, bool* finished);

}  // namespace fc4
