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

// The turbo clock (tooling, not simulator.md; the core's fc4_core::Turbo):
// our simulator's days are no longer paced by msPerDay. Settings, read at game
// start (FreeCity4.ini, else the environment):
//   sim_turbo = 0|1            (FC4_SIM_TURBO; default 0 = off, no change at all)
//   sim_turbo_frame_ms = N     (FC4_SIM_TURBO_FRAME_MS; default 250, 1..10000)
//   sim_turbo_until = YYYY-MM-DD (FC4_SIM_TURBO_UNTIL; optional)
// On, our simulator's Init switches it on for every city: the day gate is
// always ready (a day still waits for its agents), the frame budget is
// sim_turbo_frame_ms at any speed while running, and pause / hidden pause
// stop time as before. It turns itself off at the first new day on or after
// sim_turbo_until (one log line). While on, a line at every new month: the
// date, the game days per real second over that month and the frames per
// second. With `disable = simulator` the original's clock runs and turbo has
// no effect (one WARNING at game start).
#pragma once

#include <cstdint>

namespace fc4 {

struct TurboSettings {
    bool on = false;
    uint32_t frameMs = 250;
    int32_t untilJulian = 0;  // 0 = none
};

// Game start (FC4Subsystems.cpp, after the subsystems are resolved): reads
// and checks the settings, logs them when on.
void TurboReload(bool simulatorEnabled);
const TurboSettings& TurboConfig();

// The script's `turbo on|off` (main thread), through our live simulator.
// `on`: the configured frame budget and no end date. Returns false (nothing
// done) without a live simulator of ours.
bool TurboScriptSet(bool on);

}  // namespace fc4
