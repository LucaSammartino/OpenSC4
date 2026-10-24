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

// The disasters shadow: a same-process differential of the ORIGINAL disaster
// layer (docs/specs/disasters.md; tooling, no spec behaviour). Opt-in, off by
// default:
//
//   disasters_shadow = 1        (env FC4_DISASTERS_SHADOW)
//   disasters_scripted = 1      (env FC4_DISASTERS_SCRIPTED; requires shadow)
// Starts one surveyed fire followed by one surveyed riot after the first new
// month. After 200 riot samples, removes leader occupants through the game's
// occupant manager; both instances then observe the removed flag and end normally.
// Holds turbo off until both lifecycles finish, then restores configured turbo.
// Tooling only; no UI, forced phase changes or forced resumes.
// Monthly dated summaries retain all earlier differences, failures and resyncs.
//
// The original layer runs unchanged. On the identified executable only
// (compat table, DisasterHook), entries of the original's vtables are pointed
// at forwarders in this DLL (each one only if it holds the original function;
// all put back at PreAppShutdown and at DLL detach). A mirror core
// (fc4_disasters, FC4DisasterHost.h in shadow mode: it reads the city, and
// every write it would make is recorded instead; the BURNING property of the
// targets is answered from its own fires) follows the original:
//
//   - fire: every original Ignite (instance slot 6) is mirrored with the same
//     arguments after the original ran (an original fire ID is passed as it
//     is; an ID the original drew itself is matched, at its first use, with
//     the oldest ID the mirror drew [choice, Q26]); an IgniteCell (slot 7)
//     that reached no Ignite forwarder is mirrored from the occupants it set
//     burning. ExtinguishCell, QuickBurnCell, IgniteCellRadius, Extinguish,
//     BurnDown and SetCause are mirrored as calls (their nested Ignite calls
//     are then not mirrored again). The mirror's generator is the
//     deterministic one (fc4_disasters_set_mirror_random): it never spreads by
//     itself and its covered-fire draws are 0, so a fire with coverage
//     resistance above 0 cannot be followed (it is reported, not compared).
//   - the fire's animation tick (DoMessage 0x86AD10EE): the original's Update
//     runs first; its steps are counted by the tutorial's IsTutorial calls
//     inside it (UpdateAllFires reads the no-burn-down flag once per step,
//     §5.8); the mirror's wall clock (now_ms) is a virtual clock that reaches
//     the mirror's next step exactly when the original stepped.
//   - riot: AttemptDisasterStart(x, z) (manager slot 6) is mirrored after the
//     original; each leader the original registers as a location (layer slot
//     13) is handed to the mirror's create_riot_leader; the riot's animation
//     tick runs the original first, then the mirror on the real clock; the
//     original's LockNetworkOccupant calls (riot interface slot 9) give the
//     original's locked set.
//
// Compared after every mirrored call and every fire / riot tick (one log line
// per original fire step and per riot update):
//   fire: the fire count (instance slot 16), the burning targets (BURNING of
//     every target either side knows), the phase, the lot conditions the
//     mirror set (lot slot 35), the original's step count;
//   riot: the active count (manager slot 10), each riot's phase and leaders
//     (the automata), the locked network occupants (and CanRemove(2) false);
//   layer: the ongoing count (DisasterHasBegun / HasEnded calls), the location
//     list (slots 16, 17: instance, x, z);
//   messages: at each quiet point (no disaster ongoing either side for 30
//     frames) the original's posts (counted by a subscription) and the
//     mirror's (recorded) per type.
//
// Log lines (always written):
//   disasters shadow: ON (...) | requested but INACTIVE (...)
//   disasters shadow: <hook> forwarder installed | NOT installed (...)
//   disasters shadow fire step N: orig steps=S fires orig=a ours=b burning orig=.. ours=.. phase o/o ongoing o/o
//     locations o/o lots=L mismatches=K[; differences: ...]
//   disasters shadow riot R update N: phase o/o leaders o/o active o/o locked o/o locations o/o mismatches=K[...]
//   disasters shadow ignite|start|...: what was mirrored and both results
//   disasters shadow quiet: posts per type orig/ours ... mismatches=K
//   disasters shadow: city shutdown: N lines, M with mismatches=0 (fire a/b, riot c/d, quiet e/f); lines whose
//     every difference is a numbered question: ...; differences by kind: <kind> [Qnn] count, ...
//
// A difference the lane report explains carries its question number ([Q37] a riot leader's location entry,
// [Q38] the network the original locks as a leader walks, [Q39] the original's riot second one ahead); the
// riot instance the original started is the one its first leader registered (its AttemptDisasterStart
// returns null, §6.2; so does ours, whose new riot is the new last entry of
// fc4_disasters_manager_instances). An ignition inside the original's fire step (a spread) is mirrored after the
// mirror's own step of that tick, so the mirror cannot step it in that pass even though the
// core now would (§5.8, DS40); the stage and lot-condition difference it leaves carries [Q42]. An ignition the
// original refuses and the mirror takes (Q40) is put out in the mirror at once (its extinguish delay
// is 0; when the original has no fire tick the mirror's runs from the framework tick until its episode
// ends), and the differences it leaves carry [Q40]. A spread's fire that still burns in the mirror after
// the original's has gone (the original now and then steps a fire in the pass that lit it, Q42) and the
// differences it leaves carry [Q42].
#pragma once

#include <cstdint>
#include <string>

class cISC4City;

namespace fc4 {

// FC4Subsystems.cpp (game start): reads disasters_shadow; `layerReplaced`
// when FreeCity4's disaster layer replaces the original (then INACTIVE).
void DisastersShadowStartup(bool layerReplaced);

// FC4Observer.cpp: PostCityInit (the forwarders are installed at the first
// city, the mirror core made per city), PreCityShutdown, and each framework
// tick (the quiet-point comparison).
void DisastersShadowCityLoaded(cISC4City* city);
void DisastersShadowCityShutdown();
void DisastersShadowOnTick();

// PreAppShutdown and DLL detach: every original entry back.
void DisastersShadowStop();
void DisastersShadowDetach();

// The script's `disaster` action (FC4Script.cpp; FC4HarnessLogic.h
// kDisaster*): fire / riot = the layer's manager AttemptDisasterStart(x, z)
// (slot 6); x or z < 0 = the first candidate of the survey; survey = a log of
// candidate cells (no call into the layer). Returns the text for the script
// line.
std::string DisasterScriptAction(cISC4City* city, int32_t kind, int32_t x, int32_t z);

}  // namespace fc4
