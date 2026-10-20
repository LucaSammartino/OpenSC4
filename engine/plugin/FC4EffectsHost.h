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

// The effects layer's host table (fc4_effects_host, fc4.h; effects.md) on the
// city's managers: the simulator (the animation dilation and the pauses), the
// 24-hour clock, the terrain (ApplyBrush and the long terrain operation), the
// fire protection simulator (IgniteCellRadius, SetCause, ExtinguishCell), the
// toxic-spill disaster, the demolition utility, the message server and the
// city's bounds.
//
// Two modes, the same shape as FC4DisasterHost.h:
//   - live (an effects replacement, `enable = effects`): every write goes to
//     the game;
//   - shadow (FC4EffectsShadow.h, `effects_shadow = 1`): the reads come from
//     the game and every write is recorded in the context instead, so the
//     mirror core never changes the city.
//
// Render components are NOT ours (effects.md "out of scope"): particles,
// decals, sequences, sounds, cameras and automata attractors are not animated
// here. `render_component_active` therefore answers false — a stubbed render
// component does not hold an effect alive. That is the core's documented
// [choice, EF1], and it is why an effect finishes when its brush, game and
// meta work is done rather than after the shipped decals' 45-50 s (see
// docs/lanes/glue22.md question 2).
#pragma once

#include "fc4.h"

#include <cstdint>
#include <map>
#include <set>
#include <string>

class cISC4City;

namespace fc4 {

struct EffectsHostContext {
    cISC4City* city = nullptr;  // borrowed for the city session
    bool shadow = false;

    // ---- shadow mode: recorded writes, never made ----
    std::map<std::string, uint64_t> writes;  // per callback name
    std::map<uint32_t, uint64_t> posts;      // per message type
    std::map<uint32_t, uint64_t> postSums;   // data1 sums per message type
    std::set<uint32_t> endPostTypes;         // end-message types seen
    int32_t simPauses = 0, hiddenPauses = 0, clockPauses = 0;
    int32_t brushApplies = 0, longOps = 0;

    // ---- both modes ----
    // A brush stroke's long terrain operation is open.
    bool longOpOpen = false;
    // Callbacks whose write is not made (live mode), logged once each.
    std::set<std::string> notWired;
};

// The host table over `ctx` (which must outlive the manager).
fc4_effects_host MakeEffectsHost(EffectsHostContext* ctx);

// Records a callback name (shadow: as a write; live: nothing).
void EffectsHostNote(EffectsHostContext* ctx, const char* name);

// A write the live mode makes and the shadow records only. Returns true when
// the live mode may proceed.
bool EffectsHostMayWrite(EffectsHostContext* ctx, const char* name);

}  // namespace fc4
