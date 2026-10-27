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

#pragma once

#include <cstdint>

// The fixed schedule (docs/specs/simulator.md §5.8) for the original game:
// `schedule = fixed` in FreeCity4.ini (tooling; default off) holds its work to
// rules that do not depend on the machine, so the same inputs give the same
// results. In-memory hooks of the 1.1.641 executable only; nothing happens with
// any other executable.

namespace fc4 {

bool ScheduleFixed();               // `schedule = fixed` and the executable is the supported one
void ScheduleStartup();             // install the hooks; one log line per lever
void ScheduleBeforeCityLoad();      // re-seed the file-local generators again, set rand()
void ScheduleDayOpened(int32_t julian);
void ScheduleFrame();               // one simulator frame (cFC4Simulator::OnTick, fixed step)
// One digest line (the rand() state the delivery's predecessors left, a hash of
// the 34 generators, the target's class), then the rand() rule. Main thread only.
void ScheduleBeforeDelivery(const void* target, uint32_t msgType);
void ScheduleMonthLine(int32_t julian);  // counters, once a month

}  // namespace fc4
