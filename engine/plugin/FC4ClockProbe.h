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

// The clock probe (tooling only; no spec behaviour), for the M1 differential
// of the simulator's clock against the original on the same save. Off unless
// `clock_probe` (FC4_CLOCK_PROBE) is 1 or 2. When on, the director starts it
// at PostAppInit in every mode, next to the observer, and it
//   - listens to the messages the simulator posts (simulator.md §5.2, §6) from
//     game start, so the load sequence is seen too, and logs one line each:
//       CLOCK post day|week|month|year j=<data1> c=<data3>
//       CLOCK post pause|hidden|emergency|speed d1=<data1>
//   - at every PostCityInit registers one agent (enabled for every city) with
//     the city's simulator at the day, week, month and year levels (§4) and
//     logs every delivery (data1 = Julian day, data3 = the level counter):
//       CLOCK agent day|week|month|year j=<data1> c=<data3>
//     With `clock_probe = 2` the week agent reports "not finished" on the
//     first delivery of each week, so the repeat delivery of §5.3 shows as a
//     second line marked "again".
// tools/clock_diff.ps1 compares the two streams of two logs (the original's
// with `disable = simulator`, and ours) from the first day both saw. The
// interleaving of the streams depends on frame timing, the streams do not.
#pragma once

namespace fc4 {

// Reads clock_probe and, if it is on, subscribes to the posted messages (once).
void ClockProbeStart();
// Unsubscribes (PreAppShutdown).
void ClockProbeStop();

}  // namespace fc4
