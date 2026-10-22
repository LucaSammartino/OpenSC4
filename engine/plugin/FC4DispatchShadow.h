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

// The dispatch manager shadow: a same-process differential of the dispatch
// manager (docs/specs/dispatch.md; tooling, no spec behaviour). Opt-in, off
// by default:
//
//   dispatch_shadow = 1            (env FC4_DISPATCH_SHADOW)
//
// It needs the ORIGINAL dispatch manager (the default: `dispatch` is not
// enabled) and only reads it. At PostCityInit ("load") and at every
// SimNewMonth ("month"), on the main thread:
//   1. the original's record (its cIGZSerializable::Write into memory, §7) is
//      read into a FreeCity4 core made for the run over a read-only host table
//      (reference callbacks and read-only source queries are wired; callbacks
//      that change game behavior are NULL). Sources are independently recovered
//      from city protection providers and their saved air-source references.
//      The core's Write must give the same bytes back (a reference is written
//      as the capture's own object number, FC4CoreShadow.h);
//   2. the original's answers through cISC4DispatchManager against the shadow
//      core's: GetDispatchCount (slot 11) for the four dispatch types and 8,
//      GetDispatchIDArray (12) and GetDispatchInfo (10) record by record
//      (type, ID, call ID, active, the vehicle-view handle and the station,
//      automaton and pylon as object identities), and GetActivePylon (18) as
//      an object identity. Source identity (15) and available units (21) are
//      compared for every type, with original/core counts printed in the log.
//
// Not readable read-only, hence not compared (logged at game start): the
// automaton events (0x09D92FC3) the manager receives as the vehicle's event
// target, the messages it POSTS (0xE94671EB-ED, 0x894671F6), the tool's pylon
// list (Begin / End, slots 16, 17) and the vehicle view's tracking
// (slot 22 is presentation).
//
// Log lines (always written, not the verbose channel), one per event:
//   dispatch shadow YYYY-MM-DD load|month: record=N bytes roundtrip=same|DIFFERENT; compared=C mismatches=M first=...
// A run that cannot compare logs "... INCOMPLETE (why)" instead. The
// shutdown line summarises the run:
//   dispatch shadow: city shutdown: L lines, C with mismatches=0; compared K values, M mismatched; differences: ...
#pragma once

#include <cstdint>

class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the setting and logs
// whether the shadow can run in this configuration.
void DispatchShadowStartup(bool dispatchReplaced);

// Whether the shadow is running (ini key read and not turned off).
bool DispatchShadowEnabled();

// PostCityInit (FC4Observer.cpp): builds the shadow core from the original's
// record and logs the load line.
void DispatchShadowCityLoaded(cISC4City* city);

// SimNewMonth (FC4Observer.cpp): the month comparison.
void DispatchShadowMonth(cISC4City* city, int32_t julian);

// PreCityShutdown (FC4Observer.cpp): the summary line, then the core goes.
void DispatchShadowCityShutdown();

// One comparison against `city`'s dispatch manager (the test seam): 0 = exact,
// > 0 = that many mismatches, -1 = not run, -3 = nothing compared.
intptr_t DispatchShadowTestRun(cISC4City* city, int32_t julian, uint32_t event);

// The run's tallies (any pointer may be null).
void DispatchShadowTally(uint32_t* lines, uint32_t* clean, uint32_t* compared, uint32_t* mismatched);

}  // namespace fc4

// Test seam op 61 (FC4TestSeam.h).
struct FC4DispatchShadowTestInput {
    uint32_t struct_size;  // sizeof(FC4DispatchShadowTestInput)
    uint32_t event;        // 0 = report (enabled, tally), 1 = city shutdown summary
    int32_t julian;
    uint32_t lines;      // out: the summary's line count
    uint32_t clean;      // out: the lines with mismatches=0
    uint32_t compared;   // out: the values compared
    uint32_t mismatched; // out: the mismatching values
};
intptr_t DispatchShadowTest(FC4DispatchShadowTestInput* in);
