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

// The land value grid shadow: a same-process differential of the land value
// simulator (tooling; no spec behaviour). Opt-in, off by default:
//
//   landvalue_shadow = 1         (env FC4_LANDVALUE_SHADOW)
//   landvalue_shadow_cells = N   differing tracts listed per grid and line
//                                (env FC4_LANDVALUE_SHADOW_CELLS, default 4, 1..64)
//
// It needs the ORIGINAL land value simulator (the default: `landvalue` is not
// enabled). Its own message target (started at PostAppInit) hears:
//   - LandValueMapChanged (0x87CC75B3, landvalue.md §9: posted at the end of
//     state 8, so the original has just finished a pass): the original's
//     grids (slots 7, 8, 9, every tract), GetLandValueType (slot 5) at each
//     total tract's first cell, the average (slot 10), the mode altitude
//     (slot 11) and its record (its own Write into memory: the dirty / running
//     flag and the mean altitude, which have no getter) are read; FreeCity4's
//     pass (states 0-8, fc4_landvalue_shadow_compare) runs on local grids,
//     reading the terrain, the zones, the lot developer's area slope and the
//     growth developers' desirability maps live and read-only, and the two are
//     compared tract by tract;
//   - PostCityInit and SimNewDay (posted): the listed developers' desirability maps are
//     snapshotted while their GrowthAndDeclineListsAreValid (slot 10) is true
//     (landvalue.md §7.4, LV2); a map republished after the original's pass
//     read it is retried with the map from before the developer's last
//     state-17 copy. Several developers apply in ascending pointer order, as
//     in the original (LV1);
//   - PostCityShutdown: the shadow core is dropped.
//
// Log lines (always written, not the verbose channel; longer lines are split
// into "landvalue shadow (cont.)" lines):
//   landvalue shadow: ON (...) | requested but INACTIVE (...)       game start
//   landvalue shadow: NOT readable read-only (left out): ...        game start
//   landvalue shadow YYYY-MM-DD pass N: compared on CXxCZ tracts (S=s), ...;
//     intrinsic cells=.. mismatched=.. max|d|=..; total ...; slope ...; types
//     mismatched=t/n; average orig/ours=a/b; modeAlt orig/ours=..; meanAlt
//     orig/ours=..; settled=0|1; desirability=...; mismatches=N; passes=K
//     clean=C[; first differing (x,z) orig/ours: ...]               per pass
#pragma once

#include <cstdint>

class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the settings and logs
// whether the shadow can run in this configuration.
void LandValueShadowStartup(bool landValueReplaced);

// PostAppInit / PreAppShutdown (cFC4Director.cpp): the shadow's own message
// target. No-ops unless landvalue_shadow = 1 and land value is the original.
void LandValueShadowStart();
void LandValueShadowStop();

}  // namespace fc4

// Test seam op 23 (FC4TestSeam.h): one event on a fake city. The shadow calls
// only the city's GetLandValueSimulator (compiled slot 61), the original's
// getters (slots 5, 7-11) and its serializable Write, and the grids' geometry
// and GetTractValue slots; the read-only host's other services (terrain,
// zones, developers) are called only when the core asks for them.
struct FC4LandValueShadowTestInput {
    cISC4City* city;
    int32_t julian;
    uint32_t event;  // 0 = pass end, 1 = SimNewDay, 2 = PostCityShutdown, 3 = PostCityInit
};
// A pass end: its difference count, -1 (not run: no city, no original, ours)
// or -2 (ran, nothing compared). A day: the maps that changed, or -1. A
// shutdown: 0.
intptr_t LandValueShadowTest(const FC4LandValueShadowTestInput* in);
