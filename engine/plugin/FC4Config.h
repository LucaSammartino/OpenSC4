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
//   enable = landvalue            default-off subsystems to use (as FC4_ENABLE);
//                                 `disable` wins; read once, like `disable`
//   <switch> = 0 / 1              one budget behaviour that touches game
//                                 objects (below); re-read at every city load
//   budget_default = 0 / 1        value of every switch the ini/env leave unset
//   refaudit = 0 / 1              reference-count audit (default 1)
//   autotest_seconds = N          unattended run: verdict line and process
//                                 exit N s after the first tick of a loaded
//                                 city (FC4Autotest.h); 0 = off
//   autotest_unpause = 0 / 1 / 2  autotest: run the loaded city at speed 1 or 2
//   digest = 0 / 1                monthly DIGEST line (default 1; FC4Observer.h)
//   autoload_city = <path.sc4>    load this save once the region view is up
//                                 (FC4Autoload.h); tooling for unattended runs
//   autoload_delay_ticks = N      framework ticks to wait once the region is up
//                                 (default 60)
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
    // Sub-switches of lot_scan (ForEachLot), for bisecting it; lot_scan = 0
    // still skips the whole scan.
    LotList,           // lot_list: 1 = GetLotsInCellRect list (one reference per lot,
                       //   released by us, nodes freed by our SC4List); 0 = walk every
                       //   cell with GetLot(x, z, true) (borrowed; no list, no releases)
    LotBuilding,       // lot_building: lot->GetBuilding() (off: no building)
    LotProfile,        // lot_profile: building->GetBuildingProfile() purpose / wealth
    LotBuildingValue,  // lot_building_value: QI cISCPropertyHolder + property 0xE91A0B5F
    LotLandValue,      // lot_land_value: GetLocation + GetLandValue
    LotSize,           // lot_size: GetSize (off: 1 x 1)
    LotState,          // lot_state: IsInLivableState + GetOccupantWealth (off: livable, wealth 0)
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

// Setting() as an integer: "on/true/yes" = 1, "off/false/no" = 0, a number
// as is; empty or unparsable = `fallback`.
long SettingInt(const char* iniKey, const char* envName, long fallback);

// The switches off at the last ConfigReload, comma-separated without spaces,
// or "-" (static buffer; lock-free readers such as the autotest verdict).
const char* SwitchesOffList();

// Full path of FreeCity4.ini (next to the DLL).
std::string IniPath();

}  // namespace fc4
