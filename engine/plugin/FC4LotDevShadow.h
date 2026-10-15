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

// The lot developer shadow: a same-process differential of the lot developer
// (docs/specs/lot-developer.md; tooling, no spec behaviour). Opt-in, off by
// default:
//
//   lotdev_shadow = 1             (env FC4_LOTDEV_SHADOW)
//
// It needs the ORIGINAL lot developer (the default: `lotdev` is not enabled)
// and only reads it. At PostCityInit ("load") and at every SimNewMonth
// ("month"), on the main thread:
//   1. the original's record (its cIGZSerializable::Write into memory, §13)
//      is read into a FreeCity4 core made for the run over a read-only host
//      table on the city's borrowed services (Read, then Init, as the city
//      does on a load, §1.2; Init stops at §1.3 step 8: the table has no
//      simulator). The core's Write must give the same bytes back (vertsX and
//      the extents from the terrain, §1.3 step 5; the night deque's buildings
//      as the same object numbers) and its state must hold the record's
//      night deque length, cursor, nightMode and litFlag;
//   2. on up to kSampleLots lots (an even stride over the city's lots), the
//      original's terrain and geometry slots against the core's, bit for bit:
//      15, 16, 17 (lowest true / false), 18, 22, 25-32, 39 (the first
//      kSampleObjects objects of the lot's configuration), 40, 48, 51, 56,
//      57 (each with and without water where the slot takes it), and 46.
//
// Not compared (logged at game start): the daily pass, construction,
// demolition and explosion, the night lights (they change the city and draw
// from the generators: a read-only rerun cannot reproduce them); the
// levelling slots 19-21 and the foundation add / remove slots 52-55, 58-61
// (they write the terrain or the city); slots 42 / 43 (the boundary map is
// filled at §1.3 step 11, past the read-only Init); the §5 tests (no
// getter); the tuning (no getter); the night deque's buildings themselves
// (references: compared as object numbers only).
//
// Log line (always written, not the verbose channel), per run:
//   lotdev shadow YYYY-MM-DD load|month: record=N bytes roundtrip=same|DIFFERENT night=L cursor=C mode=M lit=B
//     lots=N compared=C mismatches=M first=...
// A run that cannot compare logs "... INCOMPLETE (why)" instead.
#pragma once

#include "fc4.h"

#include <cstdint>

class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the setting and logs
// whether the shadow can run in this configuration.
void LotDevShadowStartup(bool lotdevReplaced);

// FC4Observer.cpp at PostCityInit: one run. No-op unless lotdev_shadow = 1.
void LotDevShadowCityLoaded(cISC4City* city);

// FC4Observer.cpp at every SimNewMonth: one run. No-op unless lotdev_shadow = 1.
void LotDevShadowMonth(cISC4City* city, int32_t julian);

// FC4Observer.cpp at PreCityShutdown (nothing is kept between runs: no-op).
void LotDevShadowCityShutdown();

// Test seam (op 52, kFC4LotDevTestShadow): one run whatever the setting.
// `event` 0 = load, 1 = month. With `testHost` set, its non-NULL callbacks
// answer the shadow core's host calls instead of the game. Returns the run's
// mismatches, -1 without a city or lot developer, -2 when the lot developer
// is FreeCity4's (nothing compared), -3 when the line was INCOMPLETE.
intptr_t LotDevShadowTestRun(cISC4City* city, int32_t julian, uint32_t event, const fc4_lotdev_host* testHost);

}  // namespace fc4
