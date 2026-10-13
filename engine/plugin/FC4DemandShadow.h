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

// The demand shadow: a same-process differential of RCI demand (tooling; no
// spec behaviour). Opt-in, off by default:
//
//   demand_shadow = 1         (env FC4_DEMAND_SHADOW)
//   demand_shadow_nodes = N   node lines per mismatched update
//                             (env FC4_DEMAND_SHADOW_NODES, default 8, 0..64)
//
// It needs FreeCity4's simulator and the ORIGINAL demand simulator (the
// default: `demand` is not enabled). Our simulator delivers the clock to the
// original demand simulator's agent, and the shadow wraps two deliveries
// (cFC4Simulator::HostDeliver):
//
// NewDay (demand spec §10.2: DoIdle(5), then DoDemandIndexLinks, then the 12
// daily history series) - the only message that changes demand values:
//   1. before: every node of the original's graph (the keys of FreeCity4's
//      graph, built from the same demand exemplars, §4.2) is read through
//      cISC4Demand getters: supply, demand, pending supply/demand, active
//      demand, min, max, economy and tax modifiers, cap, region use;
//   2. the original's DoMessage(NewDay), unchanged;
//   3. after: the same reads, and GetNeutralTaxRate(); then FreeCity4's own
//      propagation (fc4_demand_shadow_propagate, the code our demand simulator
//      runs) over the states of step 1, compared field by field, bitwise.
//   Propagation carries all its state in the nodes (§4.4: every quantity is
//   cumulative), so seeding ours with the original's states each day makes
//   every comparison a single step on identical inputs; nothing drifts.
//   The DoIdle(5) slice can change two inputs before the propagation:
//   - census state 0 sets the 12 total nodes' tax and economy modifiers
//     (§8.1): seen as a modifier change between 1 and 3; ours then uses the
//     original's new values (exact: nothing else sets them), and the line says
//     so; and the modifiers themselves are compared (below);
//   - a census that finishes inside the slice adds its deltas to the pending
//     values of the 24 census nodes (§6.4): seen as a census node whose totals
//     differ from ours (a census node only folds its own pending values, so
//     that cannot be a propagation difference). Their pending values are then
//     inferred from the result (post - pre totals) and ours runs again; the
//     line says "census applied in this delivery" and counts it under
//     inferred=: the inference can be off by an ulp, so an ulp-level
//     difference on such a day is not conclusive.
// SimIdle (DoIdle(data3)): only the 12 total nodes' modifiers and the three
//   census sums are read before and after; when a modifier changed (census
//   state 0 ran), FreeCity4's §8.1 (fc4_demand_shadow_modifiers, the code our
//   demand simulator runs) is computed from the census sums read before and the
//   same history, ordinance and nation getters, and compared. The saved CO/I
//   economy bonuses cannot be read; they are taken as 0 (§15 Q5).
//
// Read-only: the core gets a host table with the exemplar reads and the §8.1
// getters only (DemandReadOnlyHost); only getters are called on the original
// simulator and its nodes (QueryInterface for the agent is released at once).
// With the original simulator (`disable = simulator`) there is no delivery to
// wrap and the shadow stays inactive (logged at game start).
//
// Log lines (always written, not the verbose channel):
//   demand shadow YYYY-MM-DD: demand orig/ours=a/b,...(R$..I-HT local demand,
//     as DIGEST) ; active orig/ours=...(the 12 total nodes) ; neutral orig/ours=..;
//     nodes=N; mismatches=M [nodes 0x..,...] max|orig-ours|=.. maxulps=..;
//     passes=P; updates=K clean=C [inferred=I] [; notes]
//   demand shadow YYYY-MM-DD node 0xKEY: ...           (capped per update)
//   demand shadow YYYY-MM-DD modifiers (SimIdle|NewDay): tax orig/ours=...;
//     econ orig/ours=...; neutral=..; mismatches=M; checks=K clean=C
//   demand shadow YYYY-MM-DD modifiers R$ (0x21010): ...  (per mismatch)
#pragma once

#include <cstdint>

#include "fc4.h"

class cIGZMessage2;
class cIGZMessageTarget2;
class cISC4City;

namespace fc4 {

// cFC4Demand.cpp: the demand host table for the shadow (user = the city):
// for_each_exemplar_key, exemplar, warn (our log), today, history_max,
// ordinance_factor, nation_economy_modifier; every other callback NULL.
fc4_demand_host DemandReadOnlyHost(cISC4City* city);

// Game start (after the subsystems are resolved): reads the settings and logs
// whether the shadow can run in this configuration.
void DemandShadowStartup(bool simulatorReplaced, bool demandReplaced);

// cFC4Simulator::HostDeliver, for every agent delivery: if the shadow is on
// and `target` is the original demand simulator's agent receiving NewDay or
// SimIdle, runs before / target->DoMessage(msg) / after, stores the agent's
// result in `finished` and returns true. Otherwise returns false and does
// nothing (the caller delivers).
bool DemandShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                         int64_t data1, bool* finished);

}  // namespace fc4

// Test seam op 11 (FC4TestSeam.h): one shadowed delivery to a fake original.
struct FC4DemandShadowTestLink {
    uint32_t source;
    uint32_t kind;     // the exemplar property: 0x47BB3F30 Drives, ...31 Satisfies, ...32 Contributes, ...41 Caps, ...50 Sums
    uint32_t target;
    uint32_t percent;  // Drives / Satisfies / Contributes only
};
struct FC4DemandShadowTestInput {
    cISC4City* city;              // GetDemandSimulator
    cIGZMessageTarget2* target;   // the delivery's target
    const uint32_t* exemplarKeys;  // the demand exemplars (instance = key), in this order
    uint32_t exemplarCount;
    const FC4DemandShadowTestLink* links;  // their links (by source, in this order)
    uint32_t linkCount;
    uint32_t msgType;  // NewDay 0x66956814 or SimIdle 0x6695681A
    int32_t julian;    // data1 (and `today`)
    float historyMaxRate;  // < 0: no history in the window
    float ordinanceFactor;
    float nationModifier;
};
// Returns the update's mismatch count, -1 if the shadow did not take the
// delivery, -2 if it delivered but compared nothing.
intptr_t DemandShadowTestDeliver(const FC4DemandShadowTestInput* in);
