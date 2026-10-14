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

// The growth calibration probe (tooling only; building-development.md §12),
// for setting the default of growthAttemptsPerDay (§4.4.1) from the
// original's behaviour. Off unless `growth_probe = 1` (env FC4_GROWTH_PROBE).
// It is meant to run with the ORIGINAL building development simulator and
// tract developer (FreeCity4 replaces neither on this branch; a city whose
// classes are not the originals is reported at its load).
//
// At PostAppInit, on the identified 1.1.641 executable only (compat table),
// and only if the original tract developer vtable's slot 5 holds the original
// Grow (tract-developer.md §7.1, `0x0070D130`), it patches that one entry
// with a counting forwarder (FC4GrowthProbeHook.h), and subscribes to the
// posted SimNewDay (simulator.md §5.2: posted whichever simulator runs, as
// the clock probe hears it), PostCityInit and PreCityShutdown. Otherwise it
// logs why and changes nothing. Log lines:
//
//   growth probe: ON (growth_probe from …): …                       (game start)
//   growth probe: NOTE cut=1 … is approximate: …                    (game start)
//   growth probe: hook installed at 0x… (…): 0x0070D130 -> 0x… …    (game start)
//   growth probe: NOT installed: …                                  (instead)
//   growth probe: city load: tract developer … ; BDS …              (each city)
//   growth probe yyyy-mm-dd: attempts=A qualifying=Q cut=0|1 elapsed_us=E
//     per_attempt_us=P lots=L population=N max_us=M cycles=C        (each day)
//   growth probe summary (city shutdown|app shutdown): days=D; attempts
//     min/median/max=…; cut days=K, attempts on cut days …; qualifying …;
//     mean_us_per_attempt=…; mean_cycles_per_attempt=…               (each city)
//   growth probe: hook removed …                                    (app shutdown)
//
// attempts = Grow calls since the previous day line (one BDS day: §4.4 makes
// every call from the BDS's day agent); qualifying = developers in the BDS
// developer map (slot 34) with trunc(active demand) > 5, read when the line
// is written; cut = attempts < qualifying; elapsed_us / max_us / cycles = the
// time inside those calls (QueryPerformanceCounter, rdtsc); lots = lot
// manager slot 16; population = the residential simulator's slot 16.
#pragma once

#include <cstdint>

class cISC4City;

namespace fc4 {

// Reads growth_probe and, if it is 1, installs the hook and subscribes
// (once; PostAppInit, main thread).
void GrowthProbeStart();
// The summary of the days not yet summarised, then unsubscribes and puts the
// original Grow back (PreAppShutdown).
void GrowthProbeStop();
// DLL_PROCESS_DETACH: puts the original back if it is still patched (the
// forwarder lives in this DLL).
void GrowthProbeDetach();

}  // namespace fc4

// Test seam op 39 (kFC4SeamGrowthProbe).
enum : uint32_t {
    kFC4GrowthProbeTestStart = 1,    // GrowthProbeStart on `vtable` / `expected` instead of the compat table's
    kFC4GrowthProbeTestMessage = 2,  // the probe's DoMessage: `msgType` with data1 `data1`, reading `city`
    kFC4GrowthProbeTestStop = 3,     // GrowthProbeStop
};

struct FC4GrowthProbeTestInput {
    uint32_t action;
    void** vtable;          // start: the vtable to patch (its slot 5 must hold `expected`)
    uintptr_t expected;     // start: the original Grow
    const void* bdsVtable;  // start: the vtable that counts as the original BDS's (null: not checked)
    cISC4City* city;        // message: the city the probe reads (borrowed; null: none)
    uint32_t msgType;       // message: SimNewDay 0x66956814, PostCityInit 0x26D31EC1, PreCityShutdown 0x26D31EC2
    int32_t data1;          // message: data1 (a SimNewDay's Julian day)
};

// Start: 1 installed, 0 the probe is off, -1 not installed. Message: a
// SimNewDay's attempts, 0 for the others, -1 when the probe is not running.
// Stop: 1 the original was put back, 0 not.
intptr_t GrowthProbeTest(const FC4GrowthProbeTestInput* in);
