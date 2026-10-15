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

// The zone developer / lot manager shadow: a same-process differential of
// the lot manager (docs/specs/zones.md §8; tooling, no spec behaviour).
// Opt-in, off by default:
//
//   zonedev_shadow = 1           (env FC4_ZONEDEV_SHADOW)
//
// It needs the ORIGINAL zone developer (the default: `zonedev` is not
// enabled) and only reads it. At PostCityInit ("load") and at every
// SimNewMonth ("month"), on the main thread, FreeCity4's core in shadow mode
// (fc4_zonedev_create_shadow) loads the original's own record - its
// cIGZSerializable::Write into memory, each cell's lot by reference, read
// back with IID 0xE818FCED (§10.2) - over a read-only host table on the
// city's borrowed services, then Init; and its answers are compared with the
// original's lot manager (the +4 sub-object, city component 8):
//   1. lots: the lot count (slot 16) and the record read to its end;
//   2. cells: every cell through slot 6, originOnly false and true;
//   3. lists: slot 7 (GetLotsInCellRect, originOnly false and true) and slot
//      10 (GetLotsSurroundingRect, flags 0xF and one drawn side mask) on
//      sample rects drawn from the date (no game random number is drawn):
//      the lots in order and the returned count;
//   4. occupants: slot 14 (GetOccupantLot) on the occupants of sample cells
//      (the occupant manager's slot 40 lists);
//   5. CanCreateLot: slot 20 on sample cells and sizes, both flags, no
//      configuration (§8.4 reads only, in real mode).
//
// Parcellization changes the city and draws from the zone developer's
// generator (§7, §9), so it cannot be re-run; HighlightParcels is not a dry
// run either (it draws, posts DeferOccupantOperations and places street
// drafts, §7.1). Neither is compared (logged once at game start).
//
// Log lines (always written), per run, each with mismatches=N:
//   zonedev shadow (lots|cells|lists|occupants|CanCreateLot) YYYY-MM-DD load|month: ...
// A run that cannot compare logs "... INCOMPLETE (why)" instead.
#pragma once

#include "fc4.h"

#include <cstdint>

class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the setting and logs
// whether the shadow can run in this configuration.
void ZoneDevShadowStartup(bool zonedevReplaced);

// FC4Observer.cpp at PostCityInit: one run (the date from the city's
// simulator). No-op unless zonedev_shadow = 1.
void ZoneDevShadowCityLoaded(cISC4City* city);

// FC4Observer.cpp at every SimNewMonth: one run. No-op unless zonedev_shadow = 1.
void ZoneDevShadowMonth(cISC4City* city, int32_t julian);

// Test seam (op 53, kFC4ZoneDevTestShadow): one run whatever the setting.
// `event` 0 = load, 1 = month. On `city` only GetZoneDeveloper, GetLotManager
// and the cell counts are called; with `testHost` set, its non-NULL callbacks
// answer the core's host calls instead of the game and the core sees no city
// services. Returns the run's mismatches (all five lines), -1 without a city,
// zone developer or lot manager, -2 when the zone developer is FreeCity4's
// (nothing compared), -3 when a line was INCOMPLETE.
intptr_t ZoneDevShadowTestRun(cISC4City* city, int32_t julian, uint32_t event, const fc4_zonedev_host* testHost);

}  // namespace fc4
