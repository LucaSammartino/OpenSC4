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

// The development shadow: a same-process differential of the building
// development simulator and the growth developers (tooling; no spec
// behaviour). Opt-in, off by default:
//
//   development_shadow = 1           (env FC4_DEVELOPMENT_SHADOW)
//
// It needs FreeCity4's simulator and the ORIGINAL development unit (the
// default: `development` is not enabled). Everything it reads goes through
// the game's interfaces; its cores run on read-only contexts
// (cFC4GrowthDeveloper.h GrowthContext, cFC4Development.h DevelopmentContext:
// no post, registration, creation or write reaches the game).
//
// Growth developers (growth-developer.md). Our simulator delivers SimIdle and
// SimNewDay to each original developer's agent ("cSC4GrowthDeveloper"); the
// shadow wraps those deliveries (cFC4Simulator::HostDeliver):
//   - the first delivery after the BDS started the developer's monthly update
//     (IsMapUpdateFinished, slot 28, went from true to false; the original is
//     at state 1 and has not run since) re-baselines the developer's shadow
//     core: the original's record (§9, through its own Write) with copies of
//     its five grids read into a new core (fc4_growth_read), Init with the
//     developer exemplar (type 0x6534284A, group 0x67CD5FA1, instance = the
//     type), StartMapUpdate. Before that, the previous month's core (which
//     followed every ZoningChange and StateChanged since its baseline) is
//     compared with the record: zoned-cell counts, built and maximum capacity
//     tract by tract, total capacity, last phantom sum ("month bookkeeping");
//   - every delivery is then passed on, unchanged, to the original and to our
//     core (the same message: ours follows the original call by call);
//   - after the delivery in which the original finished (slot 28 true again),
//     ours is run to its end if it is behind, and compared: D (slot 13) and
//     the population (15) tract by tract, the built and maximum capacity
//     (14, 16), the total capacity (17), the growth list (11) in order, the
//     decline list (12) in order (purposes 5-8: as a multiset, see below),
//     EstimateDesirability (29) at one cell per tract, and the construction
//     info, threshold and caps (9, 19-23).
// A static message target subscribed to ZoningChange (0x07699455) and
// StateChanged (0xE8BDA2E7) hands both to every shadow core; PreCityShutdown
// (0x26D31EC2) drops every core and reference.
//
// The BDS (building-development.md). A read-only BDS core over the city (its
// catalogue loaded once per process from the same exemplars, §4.7) is
// compared with the ORIGINAL BDS after the original's SimNewMonth delivery:
//   - once per city: the building types (slot 12, in order and as a set),
//     every profile (slot 40: purpose, wealth, capacity, the 0x499AFA38 byte,
//     the 0x27812836 flag, category flags, occupant size, plop cost and the
//     capacity maps) and the maximum density table (slot 45 for zones 0..15,
//     purposes 1..8, wealths 0..3);
//   - every month: IsSuitableForConstruction (slot 46) for every catalogue
//     type, the result and the reason.
//
// What cannot be compared is logged once at game start.
//
// Log lines (always written; a longer line is split into "development shadow
// (cont.)" lines):
//   development shadow growth 0x<type> update YYYY-MM-DD: tracts=XxZ D=.. population=.. ...; mismatches=N[; differences (original/ours): ...]
//   development shadow growth 0x<type> update YYYY-MM-DD: INCOMPLETE (why); not compared
//   development shadow catalogue YYYY-MM-DD: types=O/S profiles=N density=N; mismatches=N[; ...]
//   development shadow suitability YYYY-MM-DD: profiles=N suitable=O/S reasons ...; mismatches=N[; ...]
#pragma once

#include <cstddef>
#include <cstdint>

class cIGZMessage2;
class cIGZMessageTarget2;
class cIGZUnknown;
class cISCPropertyHolder;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the settings and logs
// whether the shadow can run in this configuration.
void DevelopmentShadowStartup(bool simulatorReplaced, bool developmentReplaced);

// cFC4Simulator::HostDeliver, for every agent delivery: if the shadow is on
// and `target` is an original growth developer's agent (SimIdle, SimNewDay)
// or the original BDS's agent receiving SimNewMonth, delivers `msg` to it
// (target->DoMessage) between the shadow's steps, stores the agent's result
// in `finished` and returns true. Otherwise returns false and does nothing
// (the caller delivers).
bool DevelopmentShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                              int64_t data1, bool* finished);

}  // namespace fc4

// Test seam op 34 (FC4TestSeam.h), without a game:
//   mode 0: the shadow's list comparison on two {x, z, maxStage} lists (in
//           order, or as a multiset with asSet); returns the mismatch count;
//   mode 1: the shadow's record bridge: `developer`'s record (through its own
//           Write) read into a new read-only growth core (fc4_growth_read);
//           returns 1 (read), 0 (refused) or -2 (no record);
//   mode 2: the shadow's developer identity: `developer` taken as an
//           original growth developer's agent (QueryInterface 0xE7CDD662, as
//           the deliveries are); returns the developer type the shadow names
//           it by and loads its exemplar with (slot 6), or -2 when it is not
//           one to shadow; the entry is dropped again;
//   mode 3: the properties a core gets from an exemplar's holder (FC4CoreProps
//           ToCoreAll: its own and those inherited from its cohorts); returns
//           the number copied and logs each (own or the cohort depth).
// -1: a null or malformed input.
struct FC4DevelopmentShadowTestInput {
    uint32_t mode;
    const int32_t* original;  // mode 0: originalCount triples
    size_t originalCount;
    const int32_t* ours;  // mode 0: oursCount triples
    size_t oursCount;
    int32_t asSet;
    cIGZUnknown* developer;  // modes 1 and 2 (borrowed)
    const cISCPropertyHolder* holder;  // mode 3 (borrowed)
};

intptr_t DevelopmentShadowTest(const FC4DevelopmentShadowTestInput* in);
