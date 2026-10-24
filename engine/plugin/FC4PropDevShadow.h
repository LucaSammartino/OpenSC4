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

// The prop developer shadow: a same-process differential of the prop
// developer (prop-developer.md; tooling, no spec behaviour). Opt-in, off by
// default:
//
//   propdev_shadow = 1        (env FC4_PROPDEV_SHADOW)
//
// It needs FreeCity4's simulator (`simulator` enabled: the original's agent
// deliveries pass through it) and the ORIGINAL prop developer (`propdev` not
// enabled). At the first block step it follows (Idle 0x6695681A or Day
// 0x66956814 to the agent "cSC4PropDeveloper"), a shadow core (read-only
// host: it changes nothing in the game) reads the original's record (§7)
// and inits; it then follows the 13 messages of §3.1 step 5 and the
// animation tick through its own listener. Each block step of the original
// is wrapped: inputs freeze first, the original steps exactly once,
// then genuine selector transcripts replay through the independent owner,
// and then
//   - the scan flag and the block scanner (§2, raw +0x14 / +0x18..+0x34,
//     vtable-checked) are compared with ours (exact);
//   - selectors validate identity, purpose, depth, ordered candidates, x/z
//     and complete event count; zero and out-of-list returns are preserved;
//   - eligible operations compare every removal identity and saved projection.
// Creation attempts/failures and dated effects remain incomplete. Output
// matches are not full-operation exactness. At each SimNewMonth
// the original's record is read into a second core and its state and lists
// compared with the running shadow core; differences never resync it.
//
// Log lines (always written):
//   propdev shadow: ON (...) | requested but INACTIVE (...)                  game start
//   propdev shadow month YYYY-MM-DD: followed=first|yes|resynced steps=.. blocks=.. certain=.. removed orig/ours ..;
//     created orig/ours .. (not compared); record ...; mismatches=N[; differences: ...]
//   propdev shadow: city shutdown: N month lines, M with mismatches=0
#pragma once

#include <cstdint>

class cIGZMessage2;
class cIGZMessageTarget2;
class cISC4City;

namespace fc4 {

void PropDevShadowStartup(bool simulatorReplaced, bool propdevReplaced);

// cFC4Simulator.cpp HostDeliver: the original's Idle and Day deliveries.
bool PropDevShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                          int64_t data1, bool* finished);

// FC4Observer.cpp: SimNewMonth and PreCityShutdown.
void PropDevShadowMonth(cISC4City* city, int32_t julian);
void PropDevShadowCityShutdown();

}  // namespace fc4
