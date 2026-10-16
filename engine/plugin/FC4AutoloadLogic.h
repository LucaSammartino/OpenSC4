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

// The autoload's wait for the region's city list (FC4Autoload.cpp; tooling,
// no spec behaviour), kept free of game types so tests/diag_logic_test.cpp
// can check it.
//
// At start-up the framework ticks about once a millisecond, so a retry
// counted in ticks (10 checks, 60 ticks apart) spent its budget within
// ~130 ms. The wait is therefore on the wall clock: a check about once a
// second, giving up only after two minutes. (The "no city load" runs of
// 2026-10-15/16 were not an empty list but GetCityLocations' meaningless
// bool read as false; FC4Autoload.cpp now judges by the list itself.)
#pragma once

#include "FC4DiagLogic.h"

#include <cstdint>

namespace fc4 {

constexpr uint32_t kAutoloadRetryMs = 1000;     // between two checks of an empty list
constexpr uint32_t kAutoloadGiveUpMs = 120000;  // from the first empty check

struct AutoloadEmptyWait {
    int checks = 0;        // checks that found no city
    uint32_t firstMs = 0;  // GetTickCount of the first such check
    uint32_t lastMs = 0;   // and of the latest
};

// Whether the region's city list should be read at `now`: always before the
// first empty check, then once `retryMs` have passed since the latest.
inline bool AutoloadCheckDue(const AutoloadEmptyWait& w, uint32_t now, uint32_t retryMs = kAutoloadRetryMs) {
    return w.checks == 0 || ElapsedMs(now, w.lastMs) >= retryMs;
}

// Records a check at `now` that found no city. True: retry later; false:
// `giveUpMs` have passed since the first empty check, give up.
inline bool AutoloadRecordEmpty(AutoloadEmptyWait& w, uint32_t now, uint32_t giveUpMs = kAutoloadGiveUpMs) {
    if (w.checks == 0) w.firstMs = now;
    w.lastMs = now;
    ++w.checks;
    return ElapsedMs(now, w.firstMs) < giveUpMs;
}

// Milliseconds since the first empty check (0 before it).
inline uint32_t AutoloadWaitedMs(const AutoloadEmptyWait& w, uint32_t now) {
    return w.checks == 0 ? 0u : ElapsedMs(now, w.firstMs);
}

}  // namespace fc4
