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

// The aura grid shadow: a same-process differential of the aura (mayor
// rating) simulator (tooling; no spec behaviour). Opt-in, off by default:
//
//   aura_shadow = 1         (env FC4_AURA_SHADOW)
//   aura_shadow_cells = N   differing tracts listed per grid and line
//                           (env FC4_AURA_SHADOW_CELLS, default 4, 1..64)
//
// It needs the ORIGINAL aura simulator (the default: `aura` is not enabled);
// the simulator may be ours or the original. The observer (FC4Observer.cpp)
// subscribes to the original's cycle-completed message (0x2A09B21A, aura.md
// §11.1: posted, queued, at the end of state 17; the original is then idle
// in state 0 until the next month):
//   - at city load (PostCityInit): the original's record (its own Write,
//     cIGZSerializable, into memory) becomes the baseline;
//   - at every cycle-completed message: the record again; FreeCity4's cycle
//     (states 1-17, fc4_aura_shadow_cycle) runs on core copies of the
//     record's grids with the previous record's base and transient grids,
//     reading every other input live and read-only (fc4::AuraReadOnlyHost:
//     lots, populations, land value, occupants, funding, tax rates,
//     ordinances, the other simulators' grids; nothing is written to the
//     game), and the result is compared tract by tract with the record:
//     base and aura grids (aura differences in tracts that got a transient
//     effect during the cycle are counted apart), park and landmark maps,
//     the mayor rating, best / worst, hotspots. The riot roll is NOT compared
//     (the original's generator is seeded from the wall clock, §9).
//
// Log lines (always written, not the verbose channel; longer lines are split
// into "aura shadow (cont.)" lines):
//   aura shadow: ON (...) | requested but INACTIVE (...)        game start
//   aura shadow: baseline from the original's record at city load: ...
//   aura shadow YYYY-MM-DD cycle N: tracts=..; base mismatches=..; aura
//     mismatches=..; ...; mayor rating orig/ours=../..; ...; riot roll not
//     compared (wall-clock RNG); inputs: all read | inputs not available
//     (neutral): ...                                            per cycle
#pragma once

#include <cstdint>

class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the settings and logs
// whether the shadow can run in this configuration.
void AuraShadowStartup(bool auraReplaced);

// FC4Observer.cpp: PostCityInit (the baseline) and the original's
// cycle-completed message (`julian`: the current date, for the line). No-ops
// unless aura_shadow = 1.
void AuraShadowCityLoaded(cISC4City* city);
void AuraShadowCycle(cISC4City* city, int32_t julian);

}  // namespace fc4

// Test seam op 14 (FC4TestSeam.h): the shadow on a fake city. Only the city's
// GetAuraSimulator (compiled slot 51) and its cell counts / widths are called
// on `city` by the shadow itself; the read-only host's other services may be
// null (neutral reads).
struct FC4AuraShadowTestInput {
    cISC4City* city;
    int32_t julian;
    uint32_t baseline;  // non-zero: the city-load baseline, else a cycle
};
// Returns the cycle's difference count (or 1 for a baseline taken, 0 for
// none), -1 if the shadow did not run, -2 if it ran and compared nothing.
intptr_t AuraShadowTestCycle(const FC4AuraShadowTestInput* in);
