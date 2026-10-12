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

// Version-specific interop with the host executable. This is the ONLY place
// that knows fixed addresses inside a particular SimCity 4 build; every entry
// comes from docs/specs (never from inspecting the executable). Anything here
// is used only after the running executable has been positively identified.
#pragma once

#include <cstdint>

namespace fc4 {
namespace compat {

// One supported executable and its interop addresses (spec §5.5).
struct ExecutableVersion {
    const char* name;
    // Identification (main module PE headers, as loaded in memory).
    uint32_t timeDateStamp;
    uint32_t sizeOfImage;
    uint32_t imageBase;
    // CPU-cycle limit-timer coefficients (32-bit floats, virtual addresses).
    uint32_t cyclesToMillisecsVA;
    uint32_t millisecsToCyclesVA;
};

// The compatibility table.
const ExecutableVersion* SupportedVersions(int* count);

// Identifies the running executable and validates the limit-timer globals
// (once per process; every check and its outcome is logged). Returns true if
// writes are allowed.
bool LimitTimersAvailable();

// Writes CyclesToMillisecs / MillisecsToCycles if LimitTimersAvailable() and
// the values currently stored there are still plausible. Returns true if the
// values were written. Never writes otherwise (logs a warning once).
bool PublishLimitTimers(float cyclesToMs, float msToCycles);

// Number of successful publishes (diagnostics).
uint64_t PublishCount();

}  // namespace compat
}  // namespace fc4
