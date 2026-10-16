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

// The demolition PRICING differential: our DemolishOccupant /
// DemolishRegion prices, run side by side with the original's over the same
// live city and compared one for one. Opt-in, off by default:
//
//   demolition_price_shadow = 1     (env FC4_DEMOLITION_PRICE_SHADOW)
//
// glue17 left the demolition host unwired and filed this as the missing
// proof: "A dry run of DemolishRegion / DemolishOccupant on both sides needs
// that same host." Now that FC4DemolitionHost wires the whole table, the two
// can be priced against each other. A dry run (doIt = false) is side-effect
// free on both sides: demolition.md §4.3 prices only, and §4.4's removals,
// effects and network updates are all behind `doIt`.
//
// It needs the ORIGINAL utility (do not `enable = demolition`) and reads it
// only. On the main thread, at PostCityInit ("load") and at every SimNewMonth
// ("month"):
//   1. every occupant the occupant manager lists over the city is priced on
//      both sides with the same (privilege 0, flags 0) arguments and the
//      results compared: the cost (int64) and the bool the call returns;
//   2. one DemolishRegion dry run over the whole city is compared the same
//      way (the total cost and the result bool).
//
// Log lines (always written, not the verbose channel), one per event:
//   demolition price shadow YYYY-MM-DD load|month: priced=N mismatches=M first=...
// and the shutdown summary:
//   demolition price shadow: city shutdown: L lines, C with mismatches=0; compared K prices, M mismatched; ...
#pragma once

#include <cstdint>

class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the setting and logs
// whether the differential can run in this configuration.
void DemolitionPriceShadowStartup(bool demolitionReplaced);
bool DemolitionPriceShadowEnabled();

// PostCityInit (FC4Observer.cpp): builds our core over the wired host and
// logs the load line.
void DemolitionPriceShadowCityLoaded(cISC4City* city);

// SimNewMonth (FC4Observer.cpp): the month comparison.
void DemolitionPriceShadowMonth(cISC4City* city, int32_t julian);

// PreCityShutdown (FC4Observer.cpp): the summary line, then the core goes.
void DemolitionPriceShadowCityShutdown();

}  // namespace fc4
