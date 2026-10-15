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

// The history warehouse shadow: a same-process differential of the history
// warehouse (tooling; no spec behaviour). Opt-in, off by default:
//
//   history_shadow = 1              (env FC4_HISTORY_SHADOW)
//
// It needs the ORIGINAL warehouse (the default: `history` is not enabled).
// A shadow core (FreeCity4's warehouse, fc4_history) follows it, read-only:
//   - city load (PostCityInit, FC4Observer.cpp): the original's record (its
//     own Write) is read into a new shadow core, which writes it back; the
//     two records are compared byte for byte (a load line);
//   - its own message target (started at PostAppInit) hears the record
//     (0x89EFA536) and queue-length (0x89EFA537) messages every producer
//     sends (history.md §3, §4.1): each goes to the shadow core with the
//     simulation date the original's Store reads (simulator slot 8);
//   - through cFC4Simulator's delivery (HistoryShadowDeliver), the
//     original's SimNewYear and SimNewDay agent deliveries (§3, §4.3): after
//     the original's step the shadow core runs its own, and the two results
//     are compared. The shadow core's pass timer never runs out; when the
//     original's 5 ms timer splits a step (it answers false where the
//     shadow's answered true), the shadow skips the original's repeats until
//     it answers true;
//   - SimNewMonth (a month line) and the delivery at which the shadow's pass
//     returns to idle (a pass-end line): the original's record (its Write)
//     against the shadow's, byte for byte, series by series when they
//     differ; then, per series, the queries (slots 5-8, §4.6-§4.7) on a few
//     ranges around the current date and the series' ends, through the
//     original's interface and the shadow core, at 24-bit and 53-bit x87
//     precision, bit for bit, results and outputs (an output left untouched
//     must stay untouched);
//   - PostCityShutdown: the shadow core is dropped.
// Queue lengths set through the original's slot 13 (the producers' Init, not
// a message, §3) are not seen; a series that differs only so is counted as a
// mismatch and named. After a record mismatch the shadow core is rebuilt from
// the original's record while both passes are idle.
//
// Log lines (always written):
//   history shadow: ON (...) | requested but INACTIVE (...)        game start
//   history shadow YYYY-MM-DD load: ... mismatches=N               city load
//   history shadow YYYY-MM-DD month|pass end: ... mismatches=N     comparisons
//   history shadow (cont.) ...                                     details
#pragma once

#include <cstdint>

class cIGZMessage2;
class cIGZMessageTarget2;
class cIGZUnknown;
class cISC4City;

namespace fc4 {

// Game start (after the subsystems are resolved): reads the setting and logs
// whether the shadow can run in this configuration.
void HistoryShadowStartup(bool historyReplaced);

// PostAppInit / PreAppShutdown (cFC4Director.cpp): the shadow's own message
// target. No-ops unless history_shadow = 1 and history is the original.
void HistoryShadowStart();
void HistoryShadowStop();

// PostCityInit (FC4Observer.cpp): builds the shadow core from the original's
// record and logs the load line.
void HistoryShadowCityLoaded(cISC4City* city);

// cFC4Simulator::HostDeliver, for every agent delivery: if the shadow is
// listening and `target` is the original warehouse's +4 sub-object receiving
// SimNewDay or SimNewYear, runs target->DoMessage(msg), stores its result in
// `finished`, runs the shadow core's step and returns true. Otherwise returns
// false and does nothing (the caller delivers).
bool HistoryShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                          int64_t data1, bool* finished);

}  // namespace fc4

// Test seam op 54, op 1 (FC4HistClock.h): one shadow event against
// `original` (any warehouse; FreeCity4's own is accepted here) with the date
// from the installed hooks.
struct FC4HistoryShadowTestInput {
    uint32_t struct_size;  // sizeof(FC4HistoryShadowTestInput)
    // 0 = city load, 1 = a message to the shadow's target (type, data1,
    // data2), 2 = an agent delivery to the original's +4 target (type:
    // SimNewDay or SimNewYear) through HistoryShadowDeliver, 3 = a month
    // comparison, 4 = city shutdown (the core dropped).
    uint32_t event;
    cIGZUnknown* original;  // the warehouse's primary interface
    uint32_t type, data1, data2;
    int32_t julian;      // the date the lines carry (0: the hooks' date)
    uint8_t finished;    // event 2, out: the original's result
    uint8_t pad[3];
    uint32_t compared;   // out: the comparisons the event made (series + queries)
};
// The event's mismatch count (0 = exact), or -1 (not run: no shadow core, no
// original, a wrong input), or for events 1 and 2: 1 when the shadow core took
// the message, 0 when not.
intptr_t HistoryShadowTest(FC4HistoryShadowTestInput* in);
