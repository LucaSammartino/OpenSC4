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

// The protection grid shadow: a same-process differential of the fire and
// police coverage grids (tooling; no spec behaviour). Opt-in, off by default:
//
//   protection_shadow = 1     (env FC4_PROTECTION_SHADOW)
//
// It needs the ORIGINAL fire and police simulators (the default: `protection`
// is not enabled). At every SimNewMonth (the observer's notification, after
// the month's own handling order, see FC4Observer.cpp), per simulator:
//   1. reads, through the original's public getters only (§15.1 / §15.2):
//      the adjusted grid (slot 5) and its geometry and tract values, the
//      department (slot 6), the city strike flag (fire 11 / police 10), the
//      station list (fire 22 / police 21: one reference each, released here,
//      nodes freed with the game allocator), the chance of strike, the total
//      dispatch count, the average and percent coverage, and per station the
//      efficiency and the dispatch radius;
//   2. runs FreeCity4's code on those inputs (fc4_protection_shadow_run over
//      the read-only protection host: the tuning, the stations' properties,
//      budget items and positions, the dispatch manager's active dispatches;
//      every callback that would change the game is NULL): the unadjusted grid
//      rebuilt from scratch (§5.1) and the adjusted grid derived from it (§2);
//   3. compares the two adjusted grids cell by cell, and the per-station and
//      scalar values;
//   4. compares the two unadjusted int16 grids cell by cell, when the
//      original's can be read (protection.md §2, PR5): both original classes
//      keep its pointer at primary +0x84 (borrowed). It is read only from an
//      object whose primary vtable is the original class's (§15.1 fire
//      0x00AB1160, §15.2 police 0x00AB2490, from the compat table of the
//      identified executable), with the same geometry as the adjusted grid.
//
// Expected differences: the original's adjusted grid follows its unadjusted
// grid only around a changed station (§5.2) and through the daily iterator
// (§5.3, one full pass takes about 51 fire / 41 police days), and a dispatch
// draws a cone while it is active. So the grids must agree exactly only when
// our grid has been the same for two month ends in a row (no station,
// funding, strike or dispatch change for a whole pass) and no dispatch is
// active now: `expectExact=1` on the line. A mismatch with expectExact=1 is a
// finding.
//
// The original's unadjusted grid is rebuilt from scratch at every station,
// funding, strike or dispatch change (§5.1), so it should equal ours unless
// something changed without a rebuild; its mismatches are logged, not judged.
//
// NOT readable read-only (logged at game start): the cycle accumulators
// behind avg / pct (zoned, sum, nonZero) and the iterator position, so avg /
// pct are logged, not compared.
//
// Log lines (always written, not the verbose channel):
//   protection shadow: ON (...)                       game start
//   protection shadow (fire|police) YYYY-MM-DD: cells=N mismatches=M
//     max|diff|=D stations=S dispatches=K stable=s expectExact=0|1
//     first=(x,z):orig/ours,... ; efficiency mismatches=a/S ; radius
//     mismatches=b/S ; strike=0|1 ; chance orig/ours=..../.... ;
//     totalDispatch orig/ours=../.. ; avg=.. pct=.. (original)
//   protection shadow (fire|police) YYYY-MM-DD: unadjusted cells=N
//     mismatches=M max|diff|=D first=(x,z):orig/ours,...
//   protection shadow (fire|police) YYYY-MM-DD: unadjusted grid not
//     compared (<why>)                               once per simulator
//   protection shadow (fire|police) YYYY-MM-DD: skipped (<why>)
#pragma once

#include <cstdint>

class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the setting and logs
// whether the shadow can run in this configuration.
void ProtectionShadowStartup(bool protectionReplaced);

// FC4Observer.cpp, at every SimNewMonth: one comparison per simulator (a
// no-op unless protection_shadow = 1). `julian` is the month message's date.
void ProtectionShadowMonth(cISC4City* city, int32_t julian);

// Test seam (FC4TestSeam.h kFC4SeamProtectionShadowVtables): the primary
// vtables accepted as the original fire / police classes' instead of the
// compat table's (both null: back to the compat table).
void ProtectionShadowTestOriginalVtables(const void* fire, const void* police);

}  // namespace fc4
