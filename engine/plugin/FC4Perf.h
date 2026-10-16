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

// The perf profile (`perf_profile = 1`, env FC4_PERF_PROFILE; off by default;
// tooling, no behaviour change): where the real time of a game month goes.
//
// - Every agent delivery of our simulator is timed per agent and level (the
//   original's agents and ours alike), and every OnTick as a whole.
// - Named sections time the development unit's inner paths (the outermost
//   core calls, the publishes, the calls into the tract developer, each growth
//   state); count-only sections count the grid callbacks.
// - One `perf month` line per turbo month line (the simulator's SimNewMonth
//   under turbo), then every counter starts over:
//
//     perf month yyyy-mm-dd: real=…ms days=N ms/day=… tick=…ms (… ticks) agents=…ms; outside ticks=…ms
//     perf month yyyy-mm-dd agents: <name>/L …ms (n) …            (top agents by time)
//     perf month yyyy-mm-dd sections: <section> …ms (n) …         (non-zero sections)
//
// Off: no clock is read and nothing is logged (one branch per site).

#pragma once

#include <cstdint>

namespace fc4 {
namespace perf {

// Sections are inclusive (a section inside another is part of both).
enum Section : uint32_t {
    kBdsCall,           // an outermost call into the BDS core (any slot or message), Publish included
    kBdsPublish,        // the BDS's Publish after an outermost call
    kBdsDayMsg,         // the BDS's SimNewDay delivery (DoIdle, Grow, decline)
    kBdsMonthMsg,       // the BDS's SimNewMonth delivery
    kBdsIdleMsg,        // the BDS's SimIdle delivery
    kBdsOtherMsg,       // every other message to the BDS (posted ones too)
    kBdsTractGrow,      // the BDS core's calls of the tract developer's Grow (slot 5)
    kBdsTractDecline,   // the BDS core's calls of the tract developer's Decline (slot 6)
    kGrowthCall,        // an outermost call into a growth developer core, Publish included
    kGrowthPublish,     // a growth developer's Publish after an outermost call
    kGrowthLists,       // a growth developer's RefreshLists (inside its Publish)
    kGrowthUpdatePop,   // the growth cores' calls of the tract developer's UpdatePopulation (slot 7)
    kGrowthOtherMsg,    // messages to the growth developers other than SimIdle and SimNewDay
    kGrowthState0,      // SimIdle / SimNewDay deliveries by the state at entry: 0 (idle) ...
    kGrowthStateLast = kGrowthState0 + 20,  // ... to 20
    kGridGet,           // count only: grid callbacks GetTractValue
    kGridSet,           // count only: grid callbacks SetTractValue
    kGridBulk,          // count only: whole-grid reads (fc4_grid_set_bulk_reader)
    kBdsPeek,           // count only: BDS getters answered from the published copies (no call in progress)
    kGrowthPeek,        // count only: growth getters answered from the published copies
    kSectionCount;
};

// perf_profile, read once (the ini is written before the game starts).
bool On();
// The performance counter.
uint64_t Now();
void Add(Section s, uint64_t ticks);
void Count(Section s);
// Count only: an outermost call into a core, by the glue function that made it
// (what is a string literal such as __FUNCTION__; reported per name).
void CountCall(const char* unit, const char* what);
// One delivery of our simulator (inclusive, any agent).
void AddDelivery(const void* agent, uint32_t level, const char* name, uint64_t ticks);
// One OnTick of our simulator.
void AddTick(uint64_t ticks);
// Logs the perf month lines and starts every counter over (turbo month line).
void Report(const char* date, int32_t days, uint32_t realMs);

class Scope {
public:
    explicit Scope(Section s) : s_(s), t0_(On() ? Now() : 0) {}
    ~Scope() {
        if (t0_ != 0) Add(s_, Now() - t0_);
    }
    Scope(const Scope&) = delete;
    Scope& operator=(const Scope&) = delete;

private:
    Section s_;
    uint64_t t0_;
};

}  // namespace perf
}  // namespace fc4
