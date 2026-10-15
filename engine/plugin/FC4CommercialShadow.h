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

// The commercial shadow: a same-process differential of the commercial
// simulator's proximity maps (commercial-industrial.md §4.3, §4.4; tooling,
// no spec behaviour). Opt-in, off by default:
//
//   commercial_shadow = 1         (env FC4_COMMERCIAL_SHADOW)
//   commercial_shadow_cells = N   differing cells listed per grid and line
//                                 (env FC4_COMMERCIAL_SHADOW_CELLS, default 4, 1..64)
//
// It needs FreeCity4's simulator (the original's deliveries pass through
// cFC4Simulator::HostDeliver) and the ORIGINAL commercial simulator
// (`commercial` not enabled). For each SimIdle / SimNewDay delivery to the
// original's agent (both run one DoIdle step, §3):
//   1. before: its `dirty` byte (+0x20) and `state` (+0x24) are read raw and
//      read-only (under a structured-exception guard, only while the object's
//      first word is the original's vtable from the compat table). When the
//      step will rebuild a map (state 0, 1 or 2), the original's CO$$ and
//      CO$$$ grids (slot 5 GetProximityMap) are copied into the shadow core's
//      own grids, so both start from the same cells (§4.4: cells off the
//      edge keep their value);
//   2. the original's DoMessage, unchanged;
//   3. after: the shadow core (fc4_commercial, its own game grids, reading
//      the city's live building map through BDS slots 41 / 42) is set to the
//      original's before-state (fc4_commercial_set_machine) and runs the same
//      step; its dirty / state and, after a rebuild step, both grids are
//      compared cell by cell with the original's.
// Steps in state 3 (idle) are compared too (machine only) and counted in the
// next line. Nothing is written to the game.
//
// Log lines (always written, not the verbose channel):
//   commercial shadow: ON (...) | requested but INACTIVE (...)        game start
//   commercial shadow: following the original commercial simulator ... | NOT followed (why)
//   commercial shadow YYYY-MM-DD rebuild N CO$$|CO$$$ (state s->s' orig, s->s' ours; dirty d/d):
//     cells=C edge1=E zeros=Z; idle steps since the last line=K (machine mismatches=m);
//     mismatches=M[; first differing (x,z) orig/ours: ...]
#pragma once

#include <cstdint>

class cIGZMessage2;
class cIGZMessageTarget2;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the settings and logs
// whether the shadow can run in this configuration.
void CommercialShadowStartup(bool simulatorReplaced, bool commercialReplaced);

// cFC4Simulator::HostDeliver, for every agent delivery: if the shadow is on
// and `target` is the original commercial simulator's agent receiving SimIdle
// or SimNewDay, runs before / target->DoMessage(msg) / after, stores the
// agent's result in `finished` and returns true. Otherwise returns false and
// does nothing (the caller delivers).
bool CommercialShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                             int64_t data1, bool* finished);

// FC4Observer.cpp, PreCityShutdown: the shadow core and its grids go with the
// city (one summary line). No-op unless the shadow ran.
void CommercialShadowCityShutdown();

}  // namespace fc4
