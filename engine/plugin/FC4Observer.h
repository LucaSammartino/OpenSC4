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

// The observer (tooling only; no spec behaviour): a system service and
// message target the director registers at PostAppInit in EVERY mode,
// including FC4_DISABLE=all (passthrough), because it is not one of the
// game's classes. It
//   - ticks with the framework: starts the watchdog on the main thread, keeps
//     the "no OnTick" rule fed, drives the autotest clock and unpause;
//   - hears PostCityInit (autotest arming), SimNewDay (the date for the
//     heartbeat and the verdict) and SimNewMonth;
//   - at every SimNewMonth writes one non-verbose line, read only through
//     public interfaces (checked against docs/abi by abi_conformance_test):
//       DIGEST julian=… (Y-M-D) funds=… population R/C/I=… demand
//       R$/R$$/R$$$/Cs$/Cs$$/Cs$$$/Co$$/Co$$$/I-R/I-D/I-M/I-HT=… budget
//       income/expense=…
//     so the original and FreeCity4 can be diffed on the same save
//     (`digest = 0` turns it off);
//   - with `observe = 1` (default on in checked builds, else 0;
//     FC4_OBSERVE) also, at every SimNewMonth,
//       observe month YYYY-MM: date=… julian=… funds=… income=… expense=…
//       tax[0..11]=… dept_expense=0xID:…,… demand R$/…/I-HT=… population R/C/I=…
//     and, at the first tick after every city load,
//       observe snapshot (after load): date=… julian=… funds=… fundings=0xID:…,…
//     (the slots used are listed in FC4Observer.cpp);
//   - with `autotest_save_after_months = N` (FC4_AUTOTEST_SAVE_AFTER_MONTHS):
//     at the first tick after the N-th SimNewMonth since the city load, writes
//     "observe snapshot (before save)", saves the city in place
//     (cISC4App::SaveCity(bool), `autotest_save_fast = 1` for a fast save),
//     then ends like the autotest (AutotestFinish: verdict, TerminateProcess).
//     A second run that loads the save logs the "after load" snapshot to
//     compare with.
#pragma once

#include <cstdint>

class cISC4App;
class cISC4City;

namespace fc4 {

// Reads its settings (digest, observe, autotest_save_*); game start.
void ObserverReload();
// Registers the observer with the framework and the message server (once).
void ObserverStart();
// Unregisters it (PreAppShutdown).
void ObserverStop();

// Test seam (FC4TestSeam.h): a city to use instead of the application's, and
// direct calls of OnTick / DoMessage (tests have no framework).
void ObserverTestSetCity(cISC4City* city);
// An app the save autotest uses instead of the framework's (null: back to it).
void ObserverTestSetApp(cISC4App* app);
void ObserverTestTick();
void ObserverTestMessage(uint32_t type, intptr_t data1);

}  // namespace fc4
