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

// The flora census shadow: a same-process differential of the ORIGINAL flora
// simulator (tooling; no spec behaviour). Opt-in, off by default:
//
//   flora_shadow = 1             (env FC4_FLORA_SHADOW)
//
// It needs FreeCity4's simulator and the original flora simulator (the
// default: `flora` is not enabled). Our simulator delivers the clock to the
// original's agent; the shadow wraps its SimNewMonth deliveries
// (cFC4Simulator::HostDeliver). The comparisons themselves are the core's
// (engine/capi/src/flora_shadow.rs has the full list):
//
//   1. once per city and simulator, at its first month: the registry after
//      load. FreeCity4 builds a registry from the same flora exemplars, in
//      the resource manager's order, and compares it with the original's
//      type array (slot 9), IsFloraTypeValid(0) (slot 11) and each record it
//      returns from GetStaticFloraData (slot 7, read from its block: next and
//      seed type, water, the 14 floats, cluster radius and height, the 256
//      table bytes, the unnamed value, flag bits 0-2, the families, the plop
//      cost when the exemplar has one);
//   2. each SimNewMonth: before, the original's own Write (cIGZSerializable)
//      into memory and the census of every flora occupant of the city (type,
//      state, birth date, last seeding date, position, box, flag 1, and the
//      moisture type and terrain altitude at its position); the original's
//      DoMessage, unchanged; after, the record and the census again. The
//      core recovers the blocks the pass visited from the record's iterator
//      and applies the deterministic part of the aging rule (flora.md
//      §6.7.2: rebirth, stage, removal, last seeding date) to each occupant
//      of a visited block, and checks that no other occupant changed and
//      that only `today` and the iterator moved in the record.
//
// Not compared (the random part, §8, and what the game would allocate into
// our containers): where seeds land and whether they are accepted, cluster
// merges (occupants that vanish while predicted present are counted as
// "vanished", new ones as "new"), wild seeds, the generator state, the
// family map and the type sets of slots 8, 10, 13 and 14, flag bits 3-7 and
// the key-0 record's flags and plop cost. A pass that wraps the iterator
// twice is not compared. Reads only: nothing is written to the game.
//
// Log lines (always written; longer lines are split into "flora shadow
// (cont.)" lines):
//   flora shadow registry: ...                 once per city (differences, or clean)
//   flora shadow month YYYY-MM-DD: ...         each month (mismatches, predictions, or INCOMPLETE (why))
//   flora shadow census YYYY-MM-DD: before N (...) after M (...)   by stage and type
#pragma once

#include <cstdint>

class cIGZMessage2;
class cIGZMessageTarget2;
class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the setting and logs
// whether the shadow can run in this configuration.
void FloraShadowStartup(bool simulatorReplaced, bool floraReplaced);

// cFC4Simulator::HostDeliver, for every agent delivery: if the shadow is on
// and `target` is the original flora simulator's agent receiving SimNewMonth,
// runs before / target->DoMessage(msg) / after, stores the agent's result in
// `finished` and returns true. Otherwise returns false and does nothing (the
// caller delivers).
bool FloraShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                        int64_t data1, bool* finished);

}  // namespace fc4

// Test seam op 17 (FC4TestSeam.h): one shadowed SimNewMonth to a fake
// original. Only the city's GetFloraSimulator (compiled slot 59) is called on
// `city`; the occupant manager, terrain and weather stay null (an empty
// census), and the registry check runs against the fake's slots 7, 9 and 11.
struct FC4FloraShadowTestInput {
    cISC4City* city;
    cIGZMessageTarget2* target;  // the delivery's target
    uint32_t msgType;            // SimNewMonth 0x66956816 (anything else is not taken)
    int32_t julian;              // data1
};
// Returns the month's mismatch count, -1 if the shadow did not take the
// delivery, -2 if it delivered but compared nothing. The registry check's
// difference count goes to `*registryOut` when not null (-1 = not run).
intptr_t FloraShadowTestDeliver(const FC4FloraShadowTestInput* in, intptr_t* registryOut);
