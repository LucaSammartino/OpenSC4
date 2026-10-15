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

// The parks shadow: a same-process differential of the park manager
// (parks.md; tooling, no spec behaviour). Opt-in, off by default:
//
//   parks_shadow = 1        (env FC4_PARKS_SHADOW)
//
// It needs FreeCity4's simulator (the original's month passes through
// cFC4Simulator::HostDeliver) and the ORIGINAL park manager (`parks` not
// enabled). A shadow core (fc4_park with a read-only host: nothing is
// written to the game) follows the original:
//   - at the first SimNewMonth delivery to the original's agent it starts
//     from the original's record (its own Write into memory, §7) and the
//     shipped tuning (§6), the latch clear as the original's after its Init.
//     If the original's threshold (read raw) is the code default 0.25 while
//     the exemplar gives another value, ours starts as without the exemplar
//     (threshold 0.25, empty curve) and every line says
//     `tuning=as-without-exemplar` (question PKG-1);
//   - its own subscription receives the occupant inserted / removed messages
//     (§3) the original receives and runs them (§4.2, §4.3: the lists; budget
//     writes are not made); a third subscription counts the original's
//     ZooEscape posts (void1 = the original);
//   - at each SimNewMonth: the original's record is compared with ours (the
//     park and zoo lists, objects in order); a difference re-starts ours from
//     the record (its latch then clear). The original's latch (+0x15) and
//     threshold (+0x34) are read raw (only while its vtable is the original's
//     from the compat table) and compared with ours before and after the
//     month. Ours runs its month first with lot writes recorded; the
//     original's month runs; every park lot's condition after it is compared
//     with ours (unchanged where ours wrote nothing);
//   - the zoo escape (§4.4 step 5, §4.5) is NOT compared: the original's
//     generator is seeded from the clock (§5); the line gives both escape
//     counts and our probability.
//
// Log lines (always written, not the verbose channel):
//   parks shadow: ON (...) | requested but INACTIVE (...)        game start
//   parks shadow: following the original park manager ... (agent ..., vtable ...): latch=.. threshold=..
//   parks shadow month YYYY-MM-DD: followed=yes|first|resynced parks=.. zoos=.. latch orig a->b ours c->d
//     threshold orig/ours tuning=exemplar|as-without-exemplar (...); funding=f p=..; pass: lots=L
//     run-down=r restored=s; zoo escapes orig=e ours=e' (not compared); messages followed=k;
//     mismatches=N[; differences: ...]
//   parks shadow: city shutdown: N month lines, M with mismatches=0
#pragma once

#include <cstdint>

class cIGZMessage2;
class cIGZMessageTarget2;

namespace fc4 {

void ParksShadowStartup(bool simulatorReplaced, bool parksReplaced);

// cFC4Simulator::HostDeliver: the original park manager's SimNewMonth.
bool ParksShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                        int64_t data1, bool* finished);

// FC4Observer.cpp, PreCityShutdown: the shadow core and its references go.
void ParksShadowCityShutdown();

}  // namespace fc4
