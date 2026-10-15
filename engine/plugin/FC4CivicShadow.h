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

// The civic shadow: a same-process differential of the civic building
// simulator (civic-buildings.md; tooling, no spec behaviour). Opt-in, off by
// default:
//
//   civic_shadow = 1        (env FC4_CIVIC_SHADOW)
//
// It needs FreeCity4's simulator (the original's month passes through
// cFC4Simulator::HostDeliver) and the ORIGINAL civic simulator (`civic` not
// enabled). A shadow core (fc4_civic with a read-only host: nothing is
// written to the game) follows the original:
//   - its own subscription receives the occupant inserted / removed and lot
//     state messages (§3) the original receives, and runs them (§4.3-§4.5:
//     counts, exclusions, the maintained and disease-center lists; budget
//     writes are not made);
//   - at each SimNewMonth delivery to the original's agent:
//     1. the original's record (its own Write into memory, §7) is captured
//        and compared with the shadow core's state: the exclusion IDs and the
//        status keys in hash order, each status's building count, the
//        maintained and disease-center lists (objects, in order), the
//        landmark count (slot 8) and IsBuildingExcluded (slot 5) for every ID.
//        A status byte or tooltip that differs is an advisor / Lua write
//        through slot 6 or 7 on the original (not followed): counted, not a
//        mismatch;
//     2. the shadow core restarts from that record (statuses, lists and the
//        Landmarks department as the original has them) and runs its month
//        (§4.6) with lot writes recorded instead of made; the original's
//        month runs; each maintained building's lot condition after it is
//        compared with ours (the department funding read, the 0.5 threshold,
//        the 0 <-> 2 change);
//   - the ape escape (§4.6 step 1) is NOT compared: the original draws from a
//     temporary generator seeded from the clock (§5); the line says how many
//     disease centers there were and whether ours escaped.
//
// Log lines (always written, not the verbose channel):
//   civic shadow: ON (...) | requested but INACTIVE (...)        game start
//   civic shadow: following the original civic simulator ... | NOT followed (why)
//   civic shadow month YYYY-MM-DD: followed=yes|first exclusions=a/b statuses=a/b counts=..
//     maintained=a/b centers=a/b landmarks=a/b excluded-slot5=n/m; external status writes=k;
//     pass: lots=L funding-reads=F run-down=r recovered=s; ape escape not compared (centers=c, ours
//     escaped=e); mismatches=N[; differences: ...]
#pragma once

#include <cstdint>

class cIGZMessage2;
class cIGZMessageTarget2;

namespace fc4 {

void CivicShadowStartup(bool simulatorReplaced, bool civicReplaced);

// cFC4Simulator::HostDeliver: the original civic simulator's SimNewMonth.
bool CivicShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                        int64_t data1, bool* finished);

// FC4Observer.cpp, PreCityShutdown: the shadow core and its references go.
void CivicShadowCityShutdown();

}  // namespace fc4
