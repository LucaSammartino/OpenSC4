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

#include "FC4Turbo.h"

#include "FC4Config.h"
#include "FC4HarnessLogic.h"
#include "FC4Log.h"
#include "fc4.h"

#include <cctype>
#include <cstdio>
#include <string>

namespace fc4 {
namespace {

TurboSettings g_turbo;

std::string Trimmed(std::string s) {
    size_t a = 0, b = s.size();
    while (a < b && std::isspace(static_cast<unsigned char>(s[a]))) ++a;
    while (b > a && std::isspace(static_cast<unsigned char>(s[b - 1]))) --b;
    return s.substr(a, b - a);
}

}  // namespace

void TurboReload(bool simulatorEnabled) {
    TurboSettings t;
    t.on = SettingInt("sim_turbo", "FC4_SIM_TURBO", 0) != 0;
    const long frame = SettingInt("sim_turbo_frame_ms", "FC4_SIM_TURBO_FRAME_MS", 250);
    if (frame < 1 || frame > 10000) {
        Log("simulator: WARNING sim_turbo_frame_ms=%ld is not 1..10000; using 250", frame);
    } else {
        t.frameMs = static_cast<uint32_t>(frame);
    }
    const long wait = SettingInt("sim_turbo_wait_days", "FC4_SIM_TURBO_WAIT_DAYS", kTurboDefaultWaitDays);
    if (wait < 0 || wait > 3650) {
        Log("simulator: WARNING sim_turbo_wait_days=%ld is not 0..3650; using %d", wait, kTurboDefaultWaitDays);
    } else {
        t.waitDays = static_cast<int32_t>(wait);
    }
    const std::string until = Trimmed(Setting("sim_turbo_until", "FC4_SIM_TURBO_UNTIL"));
    harness::Date d;
    if (!until.empty()) {
        if (harness::ParseIsoDate(until, &d)) {
            t.untilJulian = fc4_julian_from_civil(d.year, d.month, d.day);
        } else {
            Log("simulator: WARNING sim_turbo_until='%s' is not a date YYYY-MM-DD; turbo has no end date",
                until.c_str());
        }
    }
    g_turbo = t;
    if (!t.on) return;  // off: nothing logged, nothing changed
    if (!simulatorEnabled) {
        Log("simulator: WARNING sim_turbo = 1 has no effect: our simulator is disabled (disable = simulator), the "
            "original's clock runs");
        return;
    }
    char waitText[96];
    if (t.waitDays > 0) {
        std::snprintf(waitText, sizeof(waitText), "or after %d game day(s) without it (sim_turbo_wait_days)",
                      t.waitDays);
    } else {
        std::snprintf(waitText, sizeof(waitText), "no fallback (sim_turbo_wait_days = 0)");
    }
    if (t.untilJulian != 0) {
        Log("simulator: turbo ON (sim_turbo = 1; tooling deviation from simulator.md section 5): days not paced by "
            "msPerDay, frame budget %u ms at any speed; until %s (julian %d); engages at each city's first "
            "ConnectionsReady, %s",
            t.frameMs, harness::FormatDate(d).c_str(), t.untilJulian, waitText);
    } else {
        Log("simulator: turbo ON (sim_turbo = 1; tooling deviation from simulator.md section 5): days not paced by "
            "msPerDay, frame budget %u ms at any speed; no end date; engages at each city's first ConnectionsReady, "
            "%s",
            t.frameMs, waitText);
    }
}

const TurboSettings& TurboConfig() { return g_turbo; }

}  // namespace fc4
