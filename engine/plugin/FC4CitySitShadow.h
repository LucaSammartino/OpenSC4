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

// The city situation manager shadow: a same-process differential of the city
// situation manager (docs/specs/city-situation.md; tooling, no spec
// behaviour). Opt-in, off by default:
//
//   citysit_shadow = 1             (env FC4_CITYSIT_SHADOW)
//
// It needs the ORIGINAL manager (the default: `citysit` is not enabled) and
// only reads it. At PostCityInit ("load") and at every SimNewMonth ("month"),
// on the main thread:
//   1. the original's record (its cIGZSerializable::Write into memory, §7) is
//      read into a FreeCity4 core made for the run over a read-only host table
//      (nothing is wired: the record is the success counts and the indicator
//      flag and holds no references). The core's Write must give the same
//      bytes back;
//   2. the original's answers through cISC4CitySituationManager against the
//      shadow core's: the success count of every GUID the record carries
//      (slot 5), the indicator flag (30), the status (6-12), the three float
//      times (13-15, bit for bit), the situation aura and money (17, 18),
//      the tracked count (28), the max CSI (27), the countdown flag (32) and
//      the core's fc4_citysit_get_state.
//
// This is the only in-game check of this core: without a shadow nothing calls
// it (city-situation.md §0 says a replacement is not planned, and the
// original stays by default).
//
// Not readable read-only, hence not compared (logged at game start): the
// indicator tracking's automata and handles (slots 19, 20, 22-25 mark
// vehicles), the mission itself (targets, pursuit, coverage, the player-drive
// camera: §0 leaves those to the host), the Lua table, the popup, the reward
// list it writes into the preferences, and the messages it sends.
//
// Log lines (always written, not the verbose channel), one per event:
//   citysit shadow YYYY-MM-DD load|month: record=N bytes roundtrip=same|DIFFERENT; compared=C mismatches=M first=...
// The shutdown line summarises the run:
//   citysit shadow: city shutdown: L lines, C with mismatches=0; compared K values, M mismatched; differences: ...
#pragma once

#include <cstdint>

class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the setting and logs
// whether the shadow can run in this configuration.
void CitySitShadowStartup(bool citysitReplaced);
bool CitySitShadowEnabled();

// PostCityInit (FC4Observer.cpp): builds the shadow core from the original's
// record and logs the load line.
void CitySitShadowCityLoaded(cISC4City* city);

// SimNewMonth (FC4Observer.cpp): the month comparison.
void CitySitShadowMonth(cISC4City* city, int32_t julian);

// PreCityShutdown (FC4Observer.cpp): the summary line, then the core goes.
void CitySitShadowCityShutdown();

// One comparison against `city`'s manager (the test seam).
intptr_t CitySitShadowTestRun(cISC4City* city, int32_t julian, uint32_t event);
void CitySitShadowTally(uint32_t* lines, uint32_t* clean, uint32_t* compared, uint32_t* mismatched);

}  // namespace fc4

// Test seam op 59 (FC4TestSeam.h).
struct FC4CitySitShadowTestInput {
    uint32_t struct_size;  // sizeof(FC4CitySitShadowTestInput)
    uint32_t event;        // 0 = report, 1 = city shutdown summary
    int32_t julian;
    uint32_t lines, clean, compared, mismatched;  // out
};
intptr_t CitySitShadowTest(FC4CitySitShadowTestInput* in);
