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

// The budget shadow: a same-process differential of property tax (tooling;
// no spec behaviour). Opt-in, off by default:
//
//   budget_shadow = 1        (env FC4_BUDGET_SHADOW)
//   budget_shadow_lots = N   lot lines per mismatched tax type and month
//                            (env FC4_BUDGET_SHADOW_LOTS, default 6, 0..64)
//
// It needs FreeCity4's simulator and the ORIGINAL budget (`disable = budget`):
// our simulator delivers SimNewMonth to the original budget's month agent,
// and the shadow wraps exactly that delivery (cFC4Simulator::HostDeliver):
//
//   1. before: reads the original budget's funds, year-to-date income and
//      expense, its 12 tax rates, and the non-tax income/expense inputs of the
//      settlement (ordinance, neighbour deals, department income, bond
//      payments); then runs FreeCity4's own ComputeTax (fc4_budget_shadow_tax,
//      the same core code our budget runs) over the live lots;
//   2. the original budget's DoMessage(SimNewMonth) (its settlement: budget
//      spec §5.1, whose ComputeTax is the only tax computation of the month,
//      §5.2; days move no money, §5.7);
//   3. after: reads what the original computed through its interface
//      (GetWeightedAssessedTaxValue(t), GetTaxIncome(t), GetTaxIncome(), the
//      year-to-date deltas, funds) and logs the comparison.
//
// Nothing else runs between 1 and 3, and the settlement does not change any
// tax input (lots, buildings, land values, rates), so both sides see the same
// city. Read-only: the core scan gets a host table without any write callback
// (BudgetReadOnlyHost), and only getters are called on the original budget,
// its departments and the city. With the original simulator
// (`disable = simulator`) there is no delivery to wrap and the shadow stays
// inactive (logged at game start).
//
// Log lines (always written, not the verbose channel):
//   budget shadow YYYY-MM-DD: tax orig/ours=a/b,... total orig/ours=..;
//     income orig/ours=..; expense orig/ours=..; net orig/ours=..; lots=N;
//     mismatches=M [...]; months=K clean=C
//   budget shadow YYYY-MM-DD type T: ...            (one per mismatched type)
//   budget shadow YYYY-MM-DD type T lot (x,z) ...   (capped per type)
#pragma once

#include <cstdint>

class cIGZMessage2;
class cIGZMessageTarget2;
class cISC4City;
class cFC4ISC4Lot;
struct fc4_budget_host;
struct fc4_lot_info;

namespace fc4 {

// FC4BudgetHost.cpp: the budget host table with read callbacks only, and a
// tap that sees each lot of ForEachLot just before its visit.
using BudgetLotTapFn = void (*)(void* context, cFC4ISC4Lot* lot);
const fc4_budget_host* BudgetReadOnlyHost();
void SetBudgetLotTap(BudgetLotTapFn fn, void* context);
// The walk ForEachLot used last: "master list" (the original's order, BQ3),
// "rect query", "cells" or "none".
const char* BudgetLastLotWalk();

// Game start (after the subsystems are resolved): reads the settings and logs
// whether the shadow can run in this configuration.
void BudgetShadowStartup(bool simulatorReplaced, bool budgetReplaced);

// cFC4Simulator::HostDeliver, for every agent delivery: if the shadow is on
// and `target` is the original budget simulator's month agent receiving
// SimNewMonth (the budget's cIGZMessageTarget2 interface, or failing that an
// agent registered as "cSC4BudgetSimulator"), runs before /
// target->DoMessage(msg) / after, stores the agent's result in `finished` and
// returns true. Otherwise returns false and does nothing (the caller delivers).
bool BudgetShadowDeliver(cIGZMessageTarget2* target, const char* agentName, cIGZMessage2* msg, uint32_t type,
                         int64_t data1, bool* finished);

}  // namespace fc4

// Test seam op 10 (FC4TestSeam.h): one shadowed month on a fake city.
struct FC4ShadowTestLot {
    const fc4_lot_info* info;
    int32_t x, z;
};
struct FC4ShadowTestInput {
    cISC4City* city;            // GetBudgetSimulator, GetDifficultyLevel, GetNeighborsSimulator, GetOrdinanceSimulator
    cIGZMessageTarget2* target;  // the budget's month agent (must be what the budget's QueryInterface returns)
    const FC4ShadowTestLot* lots;  // fed to the scan instead of the city's lot manager
    uint32_t lotCount;
    int32_t julian;  // SimNewMonth data1
};
// Returns the month's mismatch count, -1 if the shadow did not take the
// delivery, -2 if it delivered but could not compare.
intptr_t BudgetShadowTestMonth(const FC4ShadowTestInput* in);
