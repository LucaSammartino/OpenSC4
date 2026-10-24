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

// The effects layer's same-process shadow (FreeCity4.ini `effects_shadow = 1`).
//
// With the native manager, effects_shadow compares the section 3 packed
// directory name map against native HasVisualEffect at load and monthly.
// It records only. EF22 attachment is refused: this host lacks pre-Init
// interception, exception-transparent gateways and all-writer exclusion.
// Resource/sequence/live-instance payloads and history remain unavailable;
// name-membership exactness cannot earn whole-manager/resource credit.
// The native path has no ticking mirror, message target or host writes.
//
// With the replacement enabled, a second fc4_effects core runs beside it,
// over an EffectsHostContext in shadow mode: its
// reads come from the game and every write is recorded in the context instead
// of being made (FC4EffectsHost.h). It is ticked with the same microseconds as
// the animation tick and finishes through the same §7 sweep, so its end
// messages, brush applications and game-record applies can be compared with
// the live manager's.
//
// What it does NOT do: it does not hook the original's vtable, so the
// comparison is of the mirror core's own state and the host writes it
// recorded. When `enable = effects` is also set, the live side is our class and
// the two are compared call for call (that is the mode the game runs use).
//
// Animations stay stubbed on both sides (FC4EffectsHost.h), so the finish test
// is the stubbed one on both sides too (question 2).
#pragma once

#include <cstdint>

class cISC4City;

namespace fc4 {

// Starts or stops the shadow from the ini (effects_shadow / FC4_EFFECTS_SHADOW,
// default off). Native record comparison and replacement-call mirror are
// separate paths; replacement-call counters are not native equivalence.
// `effectsReplaced` is whether `enable = effects` put our class in place.
void EffectsShadowStartup(bool effectsReplaced);
void EffectsShadowStop();

// The city session.
void EffectsShadowCityLoaded(cISC4City* city);
void EffectsShadowCityShutdown();

// Native record-only directory name membership, at load and each month.
void EffectsShadowObserve(cISC4City* city, int32_t julian, bool load);

// One animation tick: the mirror is ticked with `us` and its finish hook runs.
void EffectsShadowOnTick(uint32_t us);

// The shadow's counters, for the run's shadow summary line.
struct EffectsShadowCounts {
    uint64_t ticks = 0;
    uint64_t created = 0;
    uint64_t endPosts = 0;
    uint64_t brushApplies = 0;
    uint64_t selectDraws = 0;
    uint64_t sweeps = 0;
    uint64_t writes = 0;
    uint64_t differences = 0;
};
EffectsShadowCounts EffectsShadowGetCounts();

// One summary line, "effects shadow: ...", logged at shutdown and on demand.
const char* EffectsShadowSummary(char* buf, unsigned cap);

// The live side's calls, mirrored into the shadow's core (cFC4Effects.cpp).
// `name` is the effect's description name; the mirror keeps one twin per name.
void EffectsShadowObserveCreate(const char* name);
void EffectsShadowObserveEndMessage(const char* name, uint32_t type, uint64_t data1);
void EffectsShadowObserveStartStop(const char* name, bool start, uint32_t mode);

}  // namespace fc4
