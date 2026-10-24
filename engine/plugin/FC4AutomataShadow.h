// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR
// PURPOSE. See the GNU General Public License for more details. You should have
// received a copy of the GNU General Public License along with this program; if
// not, see <https://www.gnu.org/licenses/>.

// The automata layer shadow: a same-process differential of the six saved
// components of docs/specs/automata.md (40 the script system, 41 the
// controller manager, 43-46 the four automata managers) and of the objects
// their records reference. Opt-in, off by default:
//
//   automata_shadow = 1             (env FC4_AUTOMATA_SHADOW)
//   automata_owners_shadow = 1      (env FC4_AUTOMATA_OWNERS_SHADOW)
//   helicopter_shadow = 1          (env FC4_HELICOPTER_SHADOW)
// The helicopter switch is an independent, partial AF11 metadata comparison:
// class and four nested record versions only; private COM fixture when no
// saved aircraft-manager reference names a helicopter. It does not tick or
// initialize that fixture. Parameters, home, goals, physics and complete live
// enumeration remain uncompared under AF2-AF6/AF9/AF11.
// The latter also compares referenced automaton/controller records recursively
// and construction records from the occupant manager's saved references. Its
// automataowners shadow lines report each inherited owner's record counts;
// this validates record codecs, not movement or lifecycle behavior.
//
// It needs the ORIGINAL components (the default: `automata` is not enabled).
// A shadow core (FreeCity4's automata layer, fc4_automata_*) follows them,
// read-only, at the one surface §0 says the simulation and the save file
// depend on: the records.
//
//   - city load (PostCityInit, FC4Observer.cpp): each component's own Write
//     is captured into memory and read by a fresh shadow core, which writes
//     it back. The two byte sequences are compared, byte for byte, and the
//     first difference is named (a load line per component).
//   - SimNewMonth (a month line): the same capture, compare and write-back of
//     the live records; the components' visible state (max count, hard limit,
//     active / paused, automata and controller counts, the controller
//     manager's two times) is compared through the getters as well.
//   - PreCityShutdown: the totals line, then the shadow cores go.
//
// Nothing here writes to the game: the originals are only ever read (their
// own Write, and the getters). The shadow cores hold their own references on
// the objects the captured records name, taken and dropped through
// fc4_automata_ref_host, so no reference is leaked or released twice
// (HANDOFF §5.1b).
//
// Log lines (always written):
//   automata shadow: ON (...) | requested but INACTIVE (...)     game start
//   automata shadow YYYY-MM-DD load: <per component>; mismatches=N   city load
//   automata shadow YYYY-MM-DD month: <per component>; mismatches=N  each month
//   automata shadow (cont.) ...                                  details
//   automata shadow: city shutdown: N lines, M with mismatches=0  the end
//
// Success is mismatches=0 on every line: each component's record reads into
// the shadow core and writes back byte for byte.
#pragma once

#include <cstdint>

class cISC4City;

// Game start: reads `automata_shadow`; `automataReplaced` is whether
// `enable = automata` asked for our own components (then nothing is compared).
void AutomataShadowStartup(bool automataReplaced);
// PostAppInit / the reverse: the message and idle hooks (none yet).
void AutomataShadowStart();
void AutomataShadowStop();
// The city's PostCityInit and PreCityShutdown.
void AutomataShadowCityLoaded(cISC4City* city, int32_t julian);
void AutomataShadowCityShutdown();
// SimNewMonth, with the game date (0 = none).
void AutomataShadowMonth(int32_t julian);

class cIGZUnknown;
namespace fc4 {
intptr_t AutomataCaptureTest(cIGZUnknown* object, uint32_t iid);
intptr_t ConstructionCaptureTest(cIGZUnknown* object);
}
