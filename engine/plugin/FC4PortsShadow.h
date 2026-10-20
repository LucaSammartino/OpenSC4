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

// The ports shadow: a same-process differential of the airport and seaport
// developers (port-developers.md; tooling, no spec behaviour). Opt-in, off by
// default:
//
//   ports_shadow = 1        (env FC4_PORTS_SHADOW)
//
// It needs FreeCity4's simulator (the originals' months pass through
// cFC4Simulator::HostDeliver) and the ORIGINAL developers (`ports` not
// enabled). One shadow core per developer (fc4_port_developer with a
// read-only host: writes are recorded, not made) follows the original:
//   - at the first SimNewMonth delivery to the original's agent (both
//     developers register "cSC4PortDeveloper"; the target tells them apart)
//     it starts from the original's record (its own Write into memory, §7):
//     Read, Init, PostCityInit (the lots looked up);
//   - its own subscription receives the messages of §3 and runs them on the
//     shadow cores (occupant inserted, lot state changed, PostCityInit, the
//     seaport's terrain message); a growth message (§4.5) makes the original
//     rebuild lots, which the read-only host cannot follow: the next month
//     restarts ours from the record (followed=regrown, not a mismatch);
//   - at each SimNewMonth: the original's record (version, tuning instance,
//     zone type, ports: stage, type index, location) is compared with ours;
//     a difference re-starts ours (followed=resynced). Ours runs its month
//     first with writes recorded; the original's month runs; then compared:
//     the history value it sends (0x89EFA536 with its key, seen by the
//     shadow's subscription) with ours, GetAvgEfficiency (bitwise),
//     GetNum*Trips, GetNumPorts, GetAllPortLots, every port lot's
//     Get*UsageValues, GetMaintenanceCost and condition after the month (ours
//     recorded), and line 1's full expenses with our RequiredBudget;
//   - seaport: CityHasSea and every cell's GetCellSeaAccess at the first
//     month and after each terrain message.
//   Not compared: the advisor event (ours is logged; the original's push is
//   not observable) and the demand effects (ours are logged).
//
// Log lines (always written, not the verbose channel):
//   ports shadow: ON (...) | requested but INACTIVE (...)                      game start
//   ports shadow: following the original <kind> developer ... (agent ...)
//   ports shadow <kind> month YYYY-MM-DD: followed=yes|first|resynced|regrown ports=.. avg orig/ours
//     trips orig/ours history orig/ours sea=.. map=..; ours: skipped=.. event=.. effects=.. conditions=..;
//     messages followed=..; mismatches=N[; differences: ...]
//   ports shadow: city shutdown: <kind> N month lines, M with mismatches=0
#pragma once

#include <cstdint>

class cIGZMessage2;
class cIGZMessageTarget2;

namespace fc4 {

void PortsShadowStartup(bool simulatorReplaced, bool portsReplaced);

// cFC4Simulator::HostDeliver: an original port developer's SimNewMonth.
bool PortsShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                        int64_t data1, bool* finished);

}  // namespace fc4
