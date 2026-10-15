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

// The 24-hour clock shadow: a same-process differential of the 24-hour clock
// (tooling; no spec behaviour). Opt-in, off by default:
//
//   clock24_shadow = 1              (env FC4_CLOCK24_SHADOW)
//
// It needs the ORIGINAL clock (the default: `clock24` is not enabled). The
// original's timer has no read path in the spec (clock24.md §2: a cRZTimer at
// +0x50, its layout unspecified), so the real time between the original's
// steps cannot be replayed. Instead a shadow core (FreeCity4's clock,
// fc4_clock24, with its own synthetic timer) mirrors each of the original's
// steps and compares everything the step derives from it:
//   - city load (PostCityInit, FC4Observer.cpp): the original's record (its
//     own Write) is read into a new shadow core, which is then initialised
//     with the same "Main Simulator" tuning (§4.2) and written back; the two
//     records are compared byte for byte, then the derived values (§4.1),
//     the mode and the pause flags (a load line);
//   - every idle pass (its own system service on the framework's on-idle
//     list, as the clock's, §4.3) and before each comparison: the original's
//     game time (GetHour, GetMinute, GetSeconds) is read. A change of 61 to
//     1200 game seconds is one step of the original (§4.3: steps are > 60 and
//     at most 1200; at most one per idle pass, so a pass sees at most one):
//     the shadow core takes the same step, through a synthetic elapsed time
//     at magnification 1 (half a game second over, so the truncation lands on
//     the same step). Any other change (WhatTimeIzIt, a setter) is mirrored
//     with SetHour, SetMinute, SetSeconds (a jump). The visual mode and the
//     two pause flags are mirrored from the original's getters before the
//     step (a resume) and after it (a pause);
//   - then the values the shadow core derives itself are compared with the
//     original's getters: the visual time (hour, minute, second), both
//     percentages (bits), IsNight, the event hours, the magnification and its
//     limits (bits);
//   - its own message target hears the seven clock messages (§3.3; the
//     original's: void1 or the IGZUnknown field = the original). Each must
//     equal, in order, the next message the shadow core posted for the same
//     mirrored step (type, data2, data3). A pause message the shadow core has
//     not posted is mirrored when it arrives (a pause and resume between two
//     idle passes is not seen by the getters);
//   - SimNewMonth: a month line; PostCityShutdown: the core is dropped.
// Log lines (always written):
//   clock shadow: ON (...) | requested but INACTIVE (...)        game start
//   clock shadow YYYY-MM-DD load: ... mismatches=N               city load
//   clock shadow YYYY-MM-DD month: ... mismatches=N              each month
//   clock shadow (cont.) ...                                     details
#pragma once

#include <cstdint>

class cIGZUnknown;
class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the setting and logs
// whether the shadow can run in this configuration.
void Clock24ShadowStartup(bool clockReplaced);

// PostAppInit / PreAppShutdown (cFC4Director.cpp): the shadow's own message
// target. No-ops unless clock24_shadow = 1 and the clock is the original.
void Clock24ShadowStart();
void Clock24ShadowStop();

// PostCityInit (FC4Observer.cpp): builds the shadow core from the original's
// record, logs the load line and adds the shadow's idle service.
void Clock24ShadowCityLoaded(cISC4City* city);

}  // namespace fc4

// Test seam op 54, op 2 (FC4HistClock.h): one shadow event against
// `original` (any clock; FreeCity4's own is accepted here).
struct FC4Clock24ShadowTestInput {
    uint32_t struct_size;  // sizeof(FC4Clock24ShadowTestInput)
    // 0 = city load, 1 = an idle pass (the sample), 2 = a clock message to the
    // shadow's target (type, data2, data3; void1 = the original), 3 = a month
    // comparison, 4 = city shutdown (the core dropped).
    uint32_t event;
    cIGZUnknown* original;  // the clock's primary interface
    uint32_t type, data2, data3;
    int32_t julian;     // the date the lines carry
    uint32_t compared;  // out: the comparisons the event made
};
// The event's mismatch count (0 = exact), or -1 (not run: no shadow core, no
// original, a wrong input).
intptr_t Clock24ShadowTest(FC4Clock24ShadowTestInput* in);
