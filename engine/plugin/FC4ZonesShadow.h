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

// The zone manager shadow: a same-process differential of the zone manager
// (docs/specs/zones.md; tooling, no spec behaviour). Opt-in, off by default:
//
//   zones_shadow = 1             (env FC4_ZONES_SHADOW)
//
// It needs the ORIGINAL zone manager (the default: `zones` is not enabled)
// and only reads it. At PostCityInit ("load") and at every SimNewMonth
// ("month"), on the main thread:
//   1. the original's answers through cISC4ZoneManager: the 16 zone counts
//      (slot 6), slots 7, 8 and 10, the zone grid (slot 5) and the tuning
//      (slots 14-17, 19, 20). Never slot 9 (it inserts a map entry, §11.1) or
//      slot 18 (a setter);
//   2. the counts recomputed from that grid (GetTractValue per cell);
//   3. FreeCity4's core in shadow mode (fc4_zones_create_shadow) loads the
//      original's own record - its cIGZSerializable::Write into memory, the
//      zone grid by reference (§10.1) - over a read-only host table on the
//      city's borrowed services, then Init (the tuning exemplar, §4). Its
//      getters are compared with the original's and its counts with the
//      recount;
//   4. PlaceZone dry runs (commit = false, outLots null): four sample
//      regions (sizes 1x1, 2x3, 4x4 and a 7x3 checkerboard; positions drawn
//      from the date, inside the grid; flags per region) x the 16 zone
//      types, the original's slot 11 on a region in the §11.4 layout whose
//      memory is ours (the game only clears bits in it, §13.2) against the
//      core's place_zone on the same cells: ok, error, cost and the cleared
//      bits. zones.md §6 notes that the first DemolishLots call "(false) only
//      prices the lots", so the dry run does not change the city.
//
// Not readable read-only, hence not compared (logged at game start): the
// development-failure maps (slot 9 inserts), the creation / destruction
// costs and max slopes except through PlaceZone's cost and rejections, the
// unnamed record fields except as the record carries them.
//
// Log lines (always written, not the verbose channel), per run:
//   zones shadow (counts) YYYY-MM-DD load|month: grid=WxH cells=N other=K
//     gridRef=same|DIFFERENT mismatches=M orig/recount/ours ...
//   zones shadow (bookkeeping) YYYY-MM-DD ...: mismatches=M (slots 7, 8, 10)
//   zones shadow (tuning) YYYY-MM-DD ...: mismatches=M (slots 14-17, 19, 20)
//   zones shadow (PlaceZone) YYYY-MM-DD ...: calls=64 mismatches=M ...
// A run that cannot compare logs "... INCOMPLETE (why)" instead.
#pragma once

#include "fc4.h"

#include <cstdint>

class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the setting and logs
// whether the shadow can run in this configuration.
void ZonesShadowStartup(bool zonesReplaced);

// FC4Observer.cpp at PostCityInit: one run (the date from the city's
// simulator). No-op unless zones_shadow = 1.
void ZonesShadowCityLoaded(cISC4City* city);

// FC4Observer.cpp at every SimNewMonth: one run. `julian` is the month
// message's date. No-op unless zones_shadow = 1.
void ZonesShadowMonth(cISC4City* city, int32_t julian);

// Test seam (op 38, kFC4ZonesTestShadow): one run whatever the setting.
// `event` 0 = load, 1 = month. Only the city's GetZoneManager (slot 36) is
// called on `city`; with `testHost` set, its non-NULL callbacks answer the
// core's host calls instead of the game and the core sees no city services.
// Returns the run's mismatches (all four lines), -1 without a city or zone
// manager, -2 when the zone manager is FreeCity4's (nothing compared), -3
// when a line was INCOMPLETE.
intptr_t ZonesShadowTestRun(cISC4City* city, int32_t julian, uint32_t event, const fc4_zones_host* testHost);

}  // namespace fc4
