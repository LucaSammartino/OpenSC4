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

// The demolition utility shadow: a same-process differential of the
// demolition utility (docs/specs/demolition.md; tooling, no spec behaviour).
// Opt-in, off by default:
//
//   demolition_shadow = 1          (env FC4_DEMOLITION_SHADOW)
//
// It needs the ORIGINAL utility (the default: `demolition` is not enabled)
// and only reads it. At PostCityInit ("load") and at every SimNewMonth
// ("month"), on the main thread:
//   1. the original's record (its cIGZSerializable::Write into memory, §7: a
//      u16 version, always 1, and nothing else) is read into a FreeCity4 core
//      made for the run over a read-only host table (nothing is wired: the
//      record holds no state and no references). The core's Write must give
//      the same bytes back;
//   2. the core's fc4_demolition_get_state (initialized, the filter, the
//      held services and the §6 tuning) is read back at every event, so a
//      core that drifted out of its constructor defaults is caught.
//
// What this shadow does NOT compare, and why (the record is the only thing
// the original exposes read-only):
//   - DemolishRegion / DemolishLots / DemolishOccupant prices and the
//     demolished set (§4.4, §4.5, §4.3): a dry run (doIt = false) would
//     price the same region on both sides, but our core needs the demolition
//     host's occupant queries (fc4_demolition_host, 84 callbacks: the
//     occupant manager's box and cell-rect queries, the occupant slots, the
//     building / flora / network / pole prices and the two shared filters)
//     and this lane did not wire that table. Filed as a question in the lane
//     report; the core is covered by its conformance vectors (dm00-dm12) in
//     the meantime;
//   - ModifyTerrainHeight (§4.7): the same, plus the terrain's altitude and
//     water-vertex queries and the lot manager's locked-vertex test;
//   - the two effect methods (§4.8): presentation only; the spec lets a
//     replaced utility do nothing there [choice];
//   - the §6 tuning, which lives in module globals and has no getter.
//
// Log lines (always written, not the verbose channel), one per event:
//   demolition shadow YYYY-MM-DD load|month: record=N bytes roundtrip=same|DIFFERENT; compared=C mismatches=M first=...
// The shutdown line summarises the run:
//   demolition shadow: city shutdown: L lines, C with mismatches=0; compared K values, M mismatched; differences: ...
#pragma once

#include <cstdint>

class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the setting and logs
// whether the shadow can run in this configuration.
void DemolitionShadowStartup(bool demolitionReplaced);
bool DemolitionShadowEnabled();

// PostCityInit (FC4Observer.cpp): builds the shadow core from the original's
// record and logs the load line.
void DemolitionShadowCityLoaded(cISC4City* city);

// SimNewMonth (FC4Observer.cpp): the month comparison.
void DemolitionShadowMonth(cISC4City* city, int32_t julian);

// PreCityShutdown (FC4Observer.cpp): the summary line, then the core goes.
void DemolitionShadowCityShutdown();

// One comparison against `city`'s utility (the test seam).
intptr_t DemolitionShadowTestRun(cISC4City* city, int32_t julian, uint32_t event);
void DemolitionShadowTally(uint32_t* lines, uint32_t* clean, uint32_t* compared, uint32_t* mismatched);

}  // namespace fc4

// Test seam op 60 (FC4TestSeam.h).
struct FC4DemolitionShadowTestInput {
    uint32_t struct_size;  // sizeof(FC4DemolitionShadowTestInput)
    uint32_t event;        // 0 = report, 1 = city shutdown summary
    int32_t julian;
    uint32_t lines, clean, compared, mismatched;  // out
};
intptr_t DemolitionShadowTest(FC4DemolitionShadowTestInput* in);
