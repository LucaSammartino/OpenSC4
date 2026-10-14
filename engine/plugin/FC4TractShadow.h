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

// The tract developer shadow: a same-process differential of the tract
// developer (docs/specs/tract-developer.md; tooling, no spec behaviour).
// Opt-in, off by default:
//
//   tract_shadow = 1             (env FC4_TRACT_SHADOW)
//
// It needs the ORIGINAL tract developer (the default: `tract` is not
// enabled) and only reads it. At PostCityInit ("load") and at every
// SimNewMonth ("month"), on the main thread:
//   1. the original's style state through cISC4TractDeveloper slots 8, 9, 11
//      and 13 (never a setter);
//   2. FreeCity4's core in shadow mode (fc4_tract_create_shadow) reads the
//      original's own record (its cIGZSerializable::Write into memory, §6),
//      over a read-only host table on the city's borrowed services, then
//      ReadTuning (§4.21, the fix-ups Init makes). Its Write must give the
//      same bytes back and its getters must match the original's;
//   3. UpdatePopulation (§4.18): for every growth developer of the BDS
//      (GetDeveloper(purpose 1-8, wealth 1-3), slot 33) and up to
//      kSampleTracts tracts holding lots (picked from cells drawn from the
//      date), the populations the core would set (fc4_tract_shadow_update_
//      population, nothing written) against what the lots hold now (lot slot
//      53 GetPopulation(devType)). Tract (0, 0) is left out (§4.18 [quirk]:
//      the original seeds it from the clock).
//
// Not compared (logged at game start): Grow and Decline (they change the
// city and draw from the generators; a read-only rerun cannot reproduce
// them), the histogram, the filter map.
//
// Log lines (always written, not the verbose channel), per run:
//   tract shadow (styles) YYYY-MM-DD load|month: record=N bytes roundtrip=same|DIFFERENT current=0x... index=...
//     styles=... all=... years=... mismatches=M first=...
//   tract shadow (population) YYYY-MM-DD load|month: developers=D tracts=T lots=L mismatches=M first=...
// A run that cannot compare logs "... INCOMPLETE (why)" instead.
#pragma once

#include "fc4.h"

#include <cstdint>

class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the setting and logs
// whether the shadow can run in this configuration.
void TractShadowStartup(bool tractReplaced);

// FC4Observer.cpp at PostCityInit: one run. No-op unless tract_shadow = 1.
void TractShadowCityLoaded(cISC4City* city);

// FC4Observer.cpp at every SimNewMonth: one run. No-op unless tract_shadow = 1.
void TractShadowMonth(cISC4City* city, int32_t julian);

// Test seam (op 45, kFC4TractTestShadow): one run whatever the setting.
// `event` 0 = load, 1 = month. With `testHost` set, its non-NULL callbacks
// answer the shadow core's host calls instead of the game. Returns the run's
// mismatches (both lines), -1 without a city or tract developer, -2 when the
// tract developer is FreeCity4's (nothing compared), -3 when a line was
// INCOMPLETE.
intptr_t TractShadowTestRun(cISC4City* city, int32_t julian, uint32_t event, const fc4_tract_host* testHost);

}  // namespace fc4
