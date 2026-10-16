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

// The session seed (docs/specs/foundations.md §6.3 [choice]; fc4.h "The
// session seed"). The plugin picks one 32-bit session seed S per process and
// hands it to every core that owns a generator; the core seeds each generator
// with derive(S, K). S is the cRZTime wall clock at game start (seconds since
// 1901, crime.md §9: the value every clock-seeded generator of the original
// reads), unless FreeCity4.ini sets
//   session_seed = N     (env FC4_SESSION_SEED; decimal or 0x...; tooling:
//                         two runs with the same N and the same inputs seed
//                         every generator of ours the same)
// Read once, at the first call; SessionSeedStartup logs it at game start.
// A subsystem's own `<key>_seed` setting (crime_seed, tract_seed, ...) still
// overrides S for that subsystem (SubsystemSeed).
#pragma once

#include <cstdint>

namespace fc4 {

// The process's session seed; `fixed` (optional) = it came from session_seed.
uint32_t SessionSeed(bool* fixed = nullptr);

// One log line with S and where it came from (game start).
void SessionSeedStartup();

// A subsystem's session seed: its own setting `iniKey` (env `envName`) when
// set to a number (`*fixed` = true), else SessionSeed() (`*fixed` = whether
// session_seed set it). `source` (optional) names where it came from.
uint32_t SubsystemSeed(const char* iniKey, const char* envName, bool* fixed, const char** source = nullptr);

}  // namespace fc4
