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

// The flammability grid shadow: a same-process differential of the
// flammability simulator (tooling; no spec behaviour). Opt-in, off by default:
//
//   flammability_shadow = 1         (env FC4_FLAMMABILITY_SHADOW)
//   flammability_shadow_cells = N   differing tracts listed per grid and line
//                                   (env FC4_FLAMMABILITY_SHADOW_CELLS, default 4, 1..64)
//
// It needs the ORIGINAL flammability simulator (the default: `flammability`
// is not enabled). Its own message target (started at PostAppInit) hears:
//   - the original's six messages (flammability.md §5: InsertOccupant,
//     RemoveOccupant, ChangeOccupant, SeasonChange, OrdinanceUpdated,
//     ConditionChanged): each is handed to a mirror simulator (ours, on its
//     own grids) that reads the occupants, lots, ordinances, weather,
//     plumbing and zones live and read-only;
//   - SimCycleComplete (0x08F415FE, §9: posted by EndOfCycle, delivered
//     queued after the day agent): the original's record (its own Write into
//     memory: both grids through their references, the multiplier, the
//     average, the hotspots) and its getters (slot 6 per city cell, slots 7
//     and 8) are read, and fc4_flammability_shadow_cycle compares them with
//     the mirror (the first cycle seen is the baseline);
//   - the plumbing pass end (0x6946DC34, plumbing.md §6.7, after the grid
//     swap): handed to the core as a message it does not mirror; like every
//     event, it first polls the plumbing simulator's GetWateredGrid (slot 7):
//     a changed pointer means a pass ended, and the tracts whose watered
//     state changed may hold their value under either state in this cycle
//     (flammability.md §6.5, FL5/FL6);
//   - PostCityShutdown: the shadow core is dropped.
//
// Log lines (always written, not the verbose channel; longer lines are split
// into "flammability shadow (cont.)" lines):
//   flammability shadow: ON (...) | requested but INACTIVE (...)    game start
//   flammability shadow: NOT readable read-only (left out): ...     game start
//   flammability shadow YYYY-MM-DD cycle N: baseline (...): ...     first cycle
//   flammability shadow YYYY-MM-DD cycle N: tracts=CXxCZ (S=1) ...; raw
//     cells=.. mismatched=.. max|d|=..; adjusted ...; average orig/ours=..;
//     multiplier orig/ours=..; hotspots orig/ours=..; events=..; plumbing
//     passes=P (tracts whose watered state changed: T); ...getters...;
//     mismatches=N; cycles=K clean=C; not compared: ...           per cycle
#pragma once

#include <cstdint>

class cIGZUnknown;
class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the settings and logs
// whether the shadow can run in this configuration.
void FlammabilityShadowStartup(bool flammabilityReplaced);

// PostAppInit / PreAppShutdown (cFC4Director.cpp): the shadow's own message
// target. No-ops unless flammability_shadow = 1 and flammability is the
// original.
void FlammabilityShadowStart();
void FlammabilityShadowStop();

}  // namespace fc4

// Test seam op 24 (FC4TestSeam.h): one event on a fake city. The shadow calls
// the city's GetFlammabilitySimulator (compiled slot 58), CellCountX / Z (93,
// 94), the original's slots 6-8 and its serializable Write, the written grids'
// cISC4SimGrid identity (QueryInterface 0xC9BD939A) with their geometry and
// GetTractValue slots, and the read-only host's services (zones, plumbing,
// weather, ordinances, occupants, lots) only when the core asks for them.
struct FC4FlammabilityShadowTestInput {
    cISC4City* city;
    int32_t julian;
    uint32_t event;     // 0 = a message (msg_type), 1 = SimCycleComplete, 2 = PostCityShutdown
    uint32_t msg_type;  // event 0: one of the six (§5), or the plumbing pass end 0x6946DC34
    cIGZUnknown* void1;        // event 0: the message's void1 (occupant messages), may be null
    cIGZUnknown* igz_unknown;  // event 0: the message's IGZUnknown (ConditionChanged), may be null
};
// A cycle: its difference count, -1 (not run: no city, no original, ours) or
// -2 (ran, nothing compared: the baseline or INCOMPLETE). A message: 1 when
// mirrored, 0 when not (no baseline yet, another type), -1 not run. A
// shutdown: 0.
intptr_t FlammabilityShadowTest(const FC4FlammabilityShadowTestInput* in);
