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

// Autotest: unattended in-game bisection runs (tooling only; no spec
// behaviour). Enabled by `autotest_seconds = N` (FreeCity4.ini, else
// FC4_AUTOTEST_SECONDS), read when the game starts.
//
// Exactly one line
//   AUTOTEST VERDICT: clean|corrupt|stalled (foreign_exceptions=…,
//   watchdog_stalls=…, rtc=…, overruns=…, ref_warnings=…[, dialogs_closed=…
//   (close_dialogs = 1 only, FC4Dialogs.h)], julian=…,
//   switches_off=…, disabled=…)
// is written, the log is flushed, and the process ends with
// TerminateProcess(GetCurrentProcess(), 0):
//   - clean / corrupt: N s after the first framework tick following a city
//     load (PostCityInit), decided by VerdictAtDeadline (FC4DiagLogic.h);
//   - corrupt at once on a first-chance exception outside FreeCity4.dll (the
//     process lives 5 s longer so a following stall is still recorded);
//   - stalled: the watchdog saw the main thread stalled for more than 10 s
//     (it terminates from its own thread).
// `autotest_end = quit` (FC4_AUTOTEST_END; default `terminate`) ends a clean
// run through the game instead: at the deadline the main thread calls
// cISC4App::RequestQuit(false, false) (no dialog, no save), the city and the
// app shut down as on a normal quit, and the verdict (counting any reference
// warnings of the Shutdown path) is written at PostSystemServiceShutdown. A
// quit that has not got there in 90 s is "stalled"; a process still alive 30 s
// after the verdict is terminated.
// `autotest_quit_post = 1` (FC4_AUTOTEST_QUIT_POST; default off) posts the
// application's native quit command instead of calling RequestQuit inside
// the observer's tick. The main loop owns delivery; the same shutdown hook
// and quit timeout still determine the verdict.
// `autotest_unpause = 1|2` makes the observer run the loaded city at that
// speed, unpaused, so month-start code runs too.
// `autotest_save_after_months = N` (the observer, FC4Observer.h): after N
// month-ends the city is saved in place, then AutotestFinish (verdict as at
// the deadline, TerminateProcess) - with or without autotest_seconds.
// `autotest_end_date = YYYY-MM-DD` (FC4_AUTOTEST_END_DATE; the harness lane,
// tooling): the run ends at the first SimNewDay on or after that date, as
// autotest_end says (verdict as at the deadline); autotest_seconds stays a
// timeout, whichever comes first ends the run. With an end date alone the
// autotest is on, without a timeout. A script `save` (FC4Script.h) still
// waiting for its tick at the end date runs first: the verdict then comes at
// that tick, after it.
// The verdict line also gives, after julian=, `date=YYYY-MM-DD` (that
// julian), `real_seconds=S.mmm` since the clock started and
// `days_per_second=X.YY` (new days since the clock started over those
// seconds); "-" where unknown (FC4HarnessLogic.h, FormatRunFigures).
#pragma once

#include <cstdint>

namespace fc4 {

// Reads the settings (game start) and starts the autotest thread when enabled.
void AutotestReload();
bool AutotestEnabled();
// The autotest thread's ID (0 before its ThreadProc starts).
uint32_t AutotestThreadId();

// PostCityInit seen by the observer (main thread).
void AutotestOnCityLoaded();
// Every framework tick (observer, main thread). Starts the clock on the first
// tick after a city load. Returns the speed (1 or 2) to unpause the city to
// now, or 0 for nothing to do.
int AutotestOnTick();
// Every SimNewDay seen by the observer (main thread), after it published the
// day. Counts the days since the clock started (the verdict's
// days_per_second) and ends the run at autotest_end_date: verdict and
// TerminateProcess, or the quit at the next tick (autotest_end = quit).
void AutotestOnNewDay(int32_t julian);
// Ends a run that finished its own task (the save autotest, main thread):
// the verdict as at the deadline (unless one was written), then
// TerminateProcess like every autotest ending.
[[noreturn]] void AutotestFinish(const char* reason, const char* from);
// PreCityShutdown (post = false) and PostCityShutdown (post = true) seen by the
// observer (main thread). A shutdown the autotest did not ask for, after the
// clock started and before the verdict, ends the run with verdict "ended": at
// PostCityShutdown it quits the game (autotest_end = quit; the verdict follows
// at PostSystemServiceShutdown) or writes the verdict and terminates.
void AutotestOnCityShutdown(bool post);
// The director's last shutdown hook (main thread): with autotest_end = quit,
// writes the verdict once the requested quit got this far.
void AutotestOnShutdownComplete(const char* hook);
// A first-chance exception outside our image (vectored handler; lock-free,
// never blocks).
void AutotestOnForeignException();
// The watchdog saw a stall longer than kAutotestStallMs (watchdog thread):
// verdict "stalled" (unless one was written) and terminate.
void AutotestOnStall(const char* kind, uint32_t stalledMs);

}  // namespace fc4
