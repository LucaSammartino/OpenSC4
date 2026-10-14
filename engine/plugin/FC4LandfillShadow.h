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

// The landfill shadow: a same-process differential of the landfill developer
// (tooling; no spec behaviour). Opt-in, off by default:
//
//   landfill_shadow = 1          (env FC4_LANDFILL_SHADOW)
//   landfill_shadow_diffs = N    differences listed per month
//                                (env FC4_LANDFILL_SHADOW_DIFFS, default 8, 1..64)
//
// It needs FreeCity4's simulator and the ORIGINAL landfill developer (the
// default: `landfill` is not enabled). Our simulator delivers the clock to the
// original's agent, and the shadow wraps its SimNewMonth deliveries
// (cFC4Simulator::HostDeliver):
//   1. before: the original's own Write (cIGZSerializable) into a capture
//      stream (bytes, and each reference queried for its cISC4Occupant
//      identity: the zot props) and its slot 7 (GetMaintenanceCosts, not
//      saved); fc4_landfill_shadow_begin loads that record into a private
//      developer, restores the maintenance and runs FreeCity4's month on it,
//      reading every input through a read-only host table (city geometry,
//      tuning, terrain, pollution's garbage supply, capacities and
//      active-landfill flags, the sanitation department's funding and line
//      item, zones, occupants, traffic). Every write is recorded, none sent;
//   2. the original's DoMessage, unchanged;
//   3. after: the record and slot 7 again; fc4_landfill_shadow_finish
//      compares the zones and their heaps, the free capacity reported to
//      pollution (its live GetGarbageCapacity), the garbage moved (the live
//      supply's change), the maintenance and the active-landfill flags.
//
// Not readable read-only, hence not compared (logged at game start): the
// zots, garbage trucks and seagulls the original drew (game objects), the
// tile occupants it changed, the budget line item's expenses it booked and
// the history records it sent. The record is the original's own Write:
// calling it outside a save is the only way to read its state.
//
// Log line (always written, not the verbose channel; a longer line is split
// into "landfill shadow (cont.)" lines), one per month:
//   landfill shadow month to YYYY-MM-DD: clean|mismatches=N; zones orig/ours
//     a/b; heaps compared=.. mismatched=.. max|d contained|=..; free capacity
//     orig/ours ..; garbage moved orig/ours ../.. (supply x -> y); maintenance
//     orig/ours ../.. (before .., recomputed ..); active cells compared=..
//     mismatched=..; ours: ...; first differences: ...
//   landfill shadow month to YYYY-MM-DD: INCOMPLETE (why); not compared
#pragma once

#include <cstdint>

class cIGZMessage2;
class cIGZMessageTarget2;
class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the settings and logs
// whether the shadow can run in this configuration.
void LandfillShadowStartup(bool simulatorReplaced, bool landfillReplaced);

// cFC4Simulator::HostDeliver, for every agent delivery: if the shadow is on
// and `target` is the original landfill developer's agent receiving
// SimNewMonth, runs before / target->DoMessage(msg) / after, stores the
// agent's result in `finished` and returns true. Otherwise returns false and
// does nothing (the caller delivers).
bool LandfillShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                           int64_t data1, bool* finished);

}  // namespace fc4

// Test seam op 18 (FC4TestSeam.h): one shadowed delivery to a fake original.
// Only the city's GetLandfillDeveloper (compiled slot 78) is called on
// `city`; the read callbacks see no city (neutral reads).
struct FC4LandfillShadowTestInput {
    cISC4City* city;
    cIGZMessageTarget2* target;  // the delivery's target
    uint32_t msgType;            // SimNewMonth 0x66956816 (anything else is not taken)
    int32_t julian;              // data1
};
// Returns the month's difference count, -1 if the shadow did not take the
// delivery, -2 if it delivered but compared nothing.
intptr_t LandfillShadowTestDeliver(const FC4LandfillShadowTestInput* in);
