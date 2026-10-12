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

// Runtime configuration for in-game bisection (tooling only; no spec
// behaviour).
//
// FreeCity4.ini next to FreeCity4.dll is a plain `key = value` file (`#` or
// `;` starts a comment, `[sections]` are ignored, keys are case-insensitive).
// A key in the ini wins over the matching environment variable.
//
//   disable = budget, simulator   subsystems left to the game (as FC4_DISABLE);
//                                 read once, when the game enumerates classes
//   <switch> = 0 / 1              one budget behaviour that touches game
//                                 objects (below); re-read at every city load
//   budget_default = 0 / 1        value of every switch the ini/env leave unset
//   refaudit = 0 / 1              reference-count audit (default 1)
//
// A switch that is off turns its behaviour into a safe no-op that returns a
// neutral value; FreeCity4.log lists the switches that are off at each load.
#pragma once

#include <string>

namespace fc4 {

enum class Sw : unsigned {
    LotScan,         // lot_scan: ForEachLot (lot list, GetBuilding, profile, building value)
    PropertyWrites,  // property_writes: AddPropertyVariant / property copies
    PropertyReads,   // property_reads: GetProperty / HasProperty on holders
    MessagePosting,  // message_posting: budget messages posted to the message server
    History,         // history: history warehouse records and queue lengths
    BuildingName,    // building_name: GetBuildingName (line item names)
    ExemplarLoads,   // exemplar_loads: tuning exemplar from the resource manager
    Disaster,        // disaster: AttemptDisasterStart (RiskyMoney)
    Cheats,          // cheats: the budget's cheat manager registration
    HolderRefs,      // holder_refs: references the budget core holds on game objects
    Ltext,           // ltext: localized string lookups (LTEXT resources)
    Count
};

// True if the switch is on (default) - cheap, any thread.
bool On(Sw s);
const char* SwitchName(Sw s);

// Counts (and, verbose, logs the first few of) the calls a switch skipped.
void NoteSkipped(Sw s, const char* what);
// One line with the skip counts since the last call (budget Shutdown).
void LogSkipCounts(const char* why);

// Re-reads FreeCity4.ini and the environment, and logs the result.
void ConfigReload(const char* why);

// The ini value of `iniKey`, else the environment variable `envName` (may be
// null), else "". `source` (optional) receives "FreeCity4.ini", `envName` or "".
std::string Setting(const char* iniKey, const char* envName, const char** source = nullptr);

// Full path of FreeCity4.ini (next to the DLL).
std::string IniPath();

}  // namespace fc4
