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

// The aura shadow: a same-process differential of the aura (mayor rating)
// simulator that follows the original step by step (aura.md §14.3; tooling,
// no spec behaviour). Opt-in, off by default:
//
//   aura_shadow = 1         (env FC4_AURA_SHADOW)
//   aura_shadow_cells = N   differing tracts listed per grid and line
//                           (env FC4_AURA_SHADOW_CELLS, default 4, 1..64)
//
// It needs the ORIGINAL aura simulator (`aura` not enabled) and FreeCity4's
// simulator (`simulator` enabled, the default): the original's deliveries
// pass through cFC4Simulator::HostDeliver.
//   - City load (PostCityInit, FC4Observer.cpp): the original's record (its
//     own Write, cIGZSerializable, into memory) starts FreeCity4's aura core
//     (Read, then Init: state 1, as the original's after a load).
//   - Each SimIdle / SimNewDay / SimNewMonth delivery to the original's agent
//     (HostDeliver): its pipeline state is read before and after (a raw,
//     read-only s32 at +0x70 of its cISC4AuraSimulator pointer, under a
//     structured-exception guard, only while its vtable is the original's
//     from the compat table, §12.1). When the state moved from s to s', our
//     steps s..s'-1 run at once, before any other agent, reading every input
//     live and read-only (fc4::AuraReadOnlyHost: lots, populations, land
//     value, occupants, funding, tax rates, ordinances, the other
//     simulators' grids; nothing is written to the game). Before a delivery
//     in state 15 or 17 the original's transient grid is copied into ours
//     (effects we cannot see). After our steps 5-13 our work grid is
//     compared with the original's (+0x44).
//   - Occupant inserted / removed (0x99EF1142 / 0x99EF1143, posted): ours
//     receives them when the original does (our own subscription).
//   - When the original's state returns to 0 (the end of state 17, or state
//     4 without lots): its record is captured and compared tract by tract
//     with ours (base, aura, park and landmark maps, transient; mayor rating,
//     best / worst, hotspots); ours then restarts from the record.
// The riot roll is NOT compared (the original's generator is seeded from the
// wall clock, §9). The cycle-completed message (0x2A09B21A) is no longer used:
// it arrives late (§5.3).
//
// Log lines (always written, not the verbose channel; longer lines are split
// into "aura shadow (cont.)" lines):
//   aura shadow: ON (...) | requested but INACTIVE (...)        game start
//   aura shadow: following the original aura simulator ... | NOT followed (why)
//   aura shadow: city load: ours starts from the original's record ...
//   aura shadow YYYY-MM-DD cycle N (followed states 1-17 over D days: M moves,
//     C calls, K occupant messages): tracts=T; base mismatches=a max|d|=b;
//     aura ...; park map ...; landmark map ...; transient ... (copied before
//     states 15 and 17: n times); work after states 5-13: w5,...,w13; mayor
//     rating orig/ours=r/r; max ...; min ...; hotspots orig/ours=h/h (same);
//     tax baseline: ...; [first differing (x,z) orig/ours: ...;] [desync:
//     ...;] riot roll not compared (wall-clock RNG); inputs: all read
//                                                               per cycle
#pragma once

#include <cstdint>

class cIGZMessage2;
class cIGZMessageTarget2;
class cIGZUnknown;
class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the settings and logs
// whether the shadow can run in this configuration.
void AuraShadowStartup(bool simulatorReplaced, bool auraReplaced);

// FC4Observer.cpp: PostCityInit (ours from the original's record). No-op
// unless aura_shadow = 1.
void AuraShadowCityLoaded(cISC4City* city);
// FC4Observer.cpp: the original's cycle-completed message. Not used: the
// comparison runs when the original's state returns to 0 (the message is
// posted and arrives later, §5.3).
void AuraShadowCycle(cISC4City* city, int32_t julian);

// cFC4Simulator::HostDeliver, for every agent delivery: if the shadow is on
// and `target` is the original aura simulator's agent, wraps a SimIdle /
// SimNewDay / SimNewMonth delivery (state read, target->DoMessage(msg), state
// read, our steps), stores the agent's result in `finished` and returns
// true. Otherwise returns false (the caller delivers).
bool AuraShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                       int64_t data1, bool* finished);

}  // namespace fc4

// Test seams ops 14 and 35 (FC4TestSeam.h), on a fake city whose aura
// simulator is the "original", with `vtable` as its expected vtable (the
// compat table's in the game). The shadow itself calls only the city's
// GetAuraSimulator (compiled slot 51) and its cell counts / widths; the
// read-only host's other services may be null (neutral reads).
struct FC4AuraShadowDeliverInput {
    cISC4City* city;             // GetAuraSimulator (51), cell counts / widths
    cIGZMessageTarget2* target;  // the original's agent (its cIGZMessageTarget2)
    const void* vtable;          // the original's expected cISC4AuraSimulator vtable
    uint32_t type;               // SimIdle / SimNewDay / SimNewMonth, or an occupant message
    int32_t julian;              // the current date
    cIGZUnknown* void1;          // the occupant messages' void1
};
// Op 14, the city load (PostCityInit; `type`, `julian` and `void1` unused):
// each call is a fresh "game". Returns 1 when ours started from the
// original's record, 0 when not, -1 if the shadow did not run.
intptr_t AuraShadowTestCityLoad(const FC4AuraShadowDeliverInput* in);
// Op 35, one delivery through AuraShadowDeliver: the original's state after
// it (0..17), -1 if the shadow did not take it (the seam then delivers it
// itself), -2 if it took it but could not read the state after. For the
// occupant messages (0x99EF1142 / 0x99EF1143) `void1` goes to the shadow's
// own subscription handler: 1 when ours received it, 0 when not.
intptr_t AuraShadowTestDeliver(const FC4AuraShadowDeliverInput* in);
