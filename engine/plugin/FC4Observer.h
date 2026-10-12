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
//     (`digest = 0` turns it off).
#pragma once

#include <cstdint>

class cISC4City;

namespace fc4 {

// Registers the observer with the framework and the message server (once).
void ObserverStart();
// Unregisters it (PreAppShutdown).
void ObserverStop();

// Test seam (FC4TestSeam.h): a city to use instead of the application's, and
// direct calls of OnTick / DoMessage (tests have no framework).
void ObserverTestSetCity(cISC4City* city);
void ObserverTestTick();
void ObserverTestMessage(uint32_t type, intptr_t data1);

}  // namespace fc4
