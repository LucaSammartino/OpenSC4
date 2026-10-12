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

// Slot layout of every game interface the plugin implements (foundations §16).
//
// For each interface a probe class implements every method so that it records
// the slot number the SPEC gives that method. The test then calls every slot
// N through the vtable, the way the game does (__thiscall, the method's
// number of 32-bit stack arguments), and checks that (a) the method that ran
// is the one the spec puts at slot N and (b) the stack is balanced. The
// tables below are the spec tables: simulator §5.6, budget "Verified slot
// order", and SDK order for the interfaces without overloads.

#include "FC4Interfaces.h"
#include "game_abi.h"

#include "cISC4DepartmentBudget.h"
#include "cISC4LineItem.h"

#include <cstdio>

namespace {

int g_failures = 0;
int g_hit = -1;

#define P(N, R, NAME, PARAMS, Q) \
    R NAME PARAMS Q override {   \
        g_hit = N;               \
        return static_cast<R>(0); \
    }

struct Expect {
    int slot;
    int dwords;       // 32-bit stack arguments the game pushes
    bool floatResult;  // returns in ST0 (popped by the test)
    const char* name;
};

template <size_t N>
void Check(const char* iface, void* probe, const Expect (&table)[N]) {
    int checked = 0;
    for (const Expect& e : table) {
        g_hit = -1;
        fc4test::SlotCall r;
        switch (e.dwords) {
            case 0: r = fc4test::CallSlot(probe, e.slot, {}); break;
            case 1: r = fc4test::CallSlot(probe, e.slot, {0}); break;
            case 2: r = fc4test::CallSlot(probe, e.slot, {0, 0}); break;
            case 3: r = fc4test::CallSlot(probe, e.slot, {0, 0, 0}); break;
            case 4: r = fc4test::CallSlot(probe, e.slot, {0, 0, 0, 0}); break;
            case 5: r = fc4test::CallSlot(probe, e.slot, {0, 0, 0, 0, 0}); break;
            default: break;
        }
        if (e.floatResult) {
            __asm fstp st(0)
        }
        if (g_hit != e.slot || r.espDelta != 0) {
            std::fprintf(stderr, "%s slot %d (%s, %d dwords): ran slot %d's method, stack off by %d bytes\n", iface,
                         e.slot, e.name, e.dwords, g_hit, r.espDelta);
            ++g_failures;
        }
        ++checked;
    }
    std::printf("%s: %d slots checked\n", iface, checked);
}

// ---- cISC4Simulator (simulator §5.6) -----------------------------------------------------------------

struct SimProbe final : cFC4ISC4Simulator {
    bool QueryInterface(uint32_t, void**) override { return false; }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }
    P(3, bool, Init, (), )
    P(4, bool, Shutdown, (), )
    P(5, bool, GetSimStartDate, (cIGZDate&), )
    P(6, void, GetSimDateParts, (uint32_t*, uint32_t*, uint32_t*, uint32_t*, uint32_t*), )
    P(7, cIGZDate*, GetSimDate, (), )
    P(8, int32_t, GetSimDateNumber, (), )
    P(9, bool, Pause, (), )
    P(10, bool, HiddenPause, (), )
    P(11, bool, EmergencyPause, (), )
    P(12, bool, Resume, (), )
    P(13, bool, HiddenResume, (), )
    P(14, bool, EmergencyResume, (), )
    P(15, bool, IsPaused, (), )
    P(16, bool, IsHiddenPaused, (), )
    P(17, bool, IsEmergencyPaused, (), )
    P(18, bool, IsAnyPaused, (), )
    P(19, bool, AddAgent, (cIGZMessageTarget2*, uint32_t, cIGZString const&, uint32_t), )
    P(20, bool, RemoveAgentAllLevels, (cIGZMessageTarget2*), )
    P(21, bool, RemoveAgent, (cIGZMessageTarget2*, uint32_t), )
    P(22, bool, RemoveAllAgentsAt, (uint32_t), )
    P(23, bool, RemoveAllAgents, (), )
    P(24, bool, EnumerateAgentsByName, (SC4Vector<SC4String>&), )
    P(25, bool, GetAgentEnabled, (cIGZString const&), )
    P(26, bool, SetAgentEnabled, (cIGZString const&, bool), )
    P(27, int32_t, GetSimSpeed, (), )
    P(28, bool, SetSimSpeed, (int32_t), )
    P(29, int32_t, GetSimTime, (), )
    P(30, bool, SetSimTime, (int32_t), )
    P(31, bool, SetMaxMillisecondsPerTick, (uint32_t), )
    P(32, float, GetAnimationTimeDilation, (), )
    P(33, bool, SetCityEstablished, (bool), )
};

const Expect kSimulator[] = {
    {3, 0, false, "Init"}, {4, 0, false, "Shutdown"}, {5, 1, false, "GetSimStartDate"},
    {6, 5, false, "GetSimDate(5 pointers)"}, {7, 0, false, "GetSimDate()"}, {8, 0, false, "GetSimDateNumber"},
    {9, 0, false, "Pause"}, {10, 0, false, "HiddenPause"}, {11, 0, false, "EmergencyPause"},
    {12, 0, false, "Resume"}, {13, 0, false, "HiddenResume"}, {14, 0, false, "EmergencyResume"},
    {15, 0, false, "IsPaused"}, {16, 0, false, "IsHiddenPaused"}, {17, 0, false, "IsEmergencyPaused"},
    {18, 0, false, "IsAnyPaused"}, {19, 4, false, "AddAgent"}, {20, 1, false, "RemoveAgent(agent)"},
    {21, 2, false, "RemoveAgent(agent, level)"}, {22, 1, false, "RemoveAllAgents(level)"},
    {23, 0, false, "RemoveAllAgents()"}, {24, 1, false, "EnumerateAgentsByName"}, {25, 1, false, "GetAgentEnabled"},
    {26, 2, false, "SetAgentEnabled"}, {27, 0, false, "GetSimSpeed"}, {28, 1, false, "SetSimSpeed"},
    {29, 0, false, "GetSimTime"}, {30, 1, false, "SetSimTime"}, {31, 1, false, "SetMaxMillisecondsPerTick"},
    {32, 0, true, "GetAnimationTimeDilation"}, {33, 1, false, "SetCityEstablished"},
};

// ---- cISC4BudgetSimulator (budget "Verified slot order") -----------------------------------------------

struct BudgetProbe final : cFC4ISC4BudgetSimulator {
    bool QueryInterface(uint32_t, void**) override { return false; }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }
    P(3, bool, Init, (), )
    P(4, bool, Shutdown, (), )
    P(5, bool, SetTotalFunds, (int64_t), )
    P(6, int64_t, GetTotalFunds, (), )
    P(7, bool, DepositFunds, (int64_t), )
    P(8, bool, WithdrawFunds, (int64_t), )
    P(9, int64_t, GetMinAllowableFunds, (), )
    P(10, int64_t, GetYTDIncome, (), )
    P(11, int64_t, GetEstIncome, (), )
    P(12, int64_t, GetYTDExpenses, (), )
    P(13, int64_t, GetEstExpenses, (), )
    P(14, int64_t, GetTotalMonthlyExpense, (), )
    P(15, int64_t, GetTotalYearlyExpense, (), )
    P(16, int64_t, GetTotalMonthlyIncome, (), )
    P(17, int64_t, GetTotalYearlyIncome, (), )
    P(18, int64_t, GetTaxIncomeOfType, (int32_t), )
    P(19, int64_t, GetTaxIncome, (), )
    P(20, int64_t, SetTotalMonthlyExpense, (int64_t), )
    P(21, int64_t, SetTotalYearlyExpense, (int64_t), )
    P(22, int64_t, SetTotalMonthlyIncome, (int64_t), )
    P(23, int64_t, SetTotalYearlyIncome, (int64_t), )
    P(24, bool, ShowBudgetWindow, (), )
    P(25, cISC4DepartmentBudget*, CreateDepartmentBudget, (uint32_t, uint32_t), )
    P(26, bool, RemoveDepartmentBudget, (uint32_t), )
    P(27, cISC4DepartmentBudget*, GetDepartmentBudgetByName, (cIGZString const&), )
    P(28, cISC4DepartmentBudget*, GetDepartmentBudget, (uint32_t), )
    P(29, bool, GetAllGroups, (SC4Vector<BudgetGroupInfo>&), )
    P(30, bool, SetGroupName, (uint32_t, cIGZString&), )
    P(31, bool, GetDepartmentBudgetsInGroup, (uint32_t, SC4Vector<cISC4DepartmentBudget*>&), )
    P(32, void, NeededFundingChanged, (cISC4DepartmentBudget*), )
    P(33, void, FundingPercentageChanged, (cISC4DepartmentBudget*), )
    P(34, int64_t, GetFundingById, (uint32_t), )
    P(35, int64_t, GetFundingOf, (cISC4DepartmentBudget*), )
    P(36, float, GetTaxRate, (uint32_t), )
    P(37, bool, SetTaxRate, (uint32_t, float), )
    P(38, int64_t, GetWeightedAssessedTaxValue, (uint32_t), )
    P(39, int64_t, GetBondIncrement, (), )
    P(40, bool, IssueBond, (uint32_t), )
    P(41, int64_t, GetTotalBorrowed, (), )
    P(42, int64_t, GetCurrentBorrowingLimit, (), )
    P(43, int64_t, GetCurrentBondLimit, (), )
    P(44, int64_t, GetCurrentMaxOutstandingBondsLimit, (), )
    P(45, int64_t, GetTotalMonthlyBondPayments, (), )
    P(46, bool, GetAllLoans, (SC4Vector<LoanInfo>&), )
    P(47, int32_t, GetLoanTimeInMonths, (), )
    P(48, int64_t, GetMonthlyPaymentForLoan, (int64_t), )
    P(49, int64_t, GetFullCostOfLoan, (int64_t), )
    P(50, bool, GetBudgetItemInfo, (cISCPropertyHolder*, SC4Vector<BudgetItem>&), )
    P(51, bool, GetBudgetItemForPurpose, (cISCPropertyHolder*, uint32_t, BudgetItem&), )
    P(52, bool, ChangeBudgetItemLine, (cISCPropertyHolder*, uint32_t, uint32_t), )
    P(53, bool, ChangeBudgetItemCost, (cISCPropertyHolder*, uint32_t, int64_t), )
    P(54, bool, ChangeBudgetItemLocalFunding, (cISCPropertyHolder*, uint32_t, SC4Percentage const&), )
    P(55, bool, AddBudgetItemToBudget, (cISCPropertyHolder*, uint32_t), )
    P(56, bool, RemoveBudgetItemFromBudget, (cISCPropertyHolder*, uint32_t), )
    P(57, cISC4LineItem*, GetLineItemFromBudgetItem, (BudgetItem&), )
    P(58, bool, CopyBudgetItemProperties, (cISCPropertyHolder*, cISCPropertyHolder*), )
};

const Expect kBudget[] = {
    {3, 0, false, "Init"}, {4, 0, false, "Shutdown"}, {5, 2, false, "SetTotalFunds"}, {6, 0, false, "GetTotalFunds"},
    {7, 2, false, "DepositFunds"}, {8, 2, false, "WithdrawFunds"}, {9, 0, false, "GetMinAllowableFunds"},
    {10, 0, false, "GetYTDIncome"}, {11, 0, false, "GetEstIncome"}, {12, 0, false, "GetYTDExpenses"},
    {13, 0, false, "GetEstExpenses"}, {14, 0, false, "GetTotalMonthlyExpense"},
    {15, 0, false, "GetTotalYearlyExpense"}, {16, 0, false, "GetTotalMonthlyIncome"},
    {17, 0, false, "GetTotalYearlyIncome"}, {18, 1, false, "GetTaxIncome(int32)"}, {19, 0, false, "GetTaxIncome()"},
    {20, 2, false, "SetTotalMonthlyExpense"}, {21, 2, false, "SetTotalYearlyExpense"},
    {22, 2, false, "SetTotalMonthlyIncome"}, {23, 2, false, "SetTotalYearlyIncome"},
    {24, 0, false, "ShowBudgetWindow"}, {25, 2, false, "CreateDepartmentBudget"},
    {26, 1, false, "RemoveDepartmentBudget"}, {27, 1, false, "GetDepartmentBudget(name)"},
    {28, 1, false, "GetDepartmentBudget(id)"}, {29, 1, false, "GetAllGroups"}, {30, 2, false, "SetGroupName"},
    {31, 2, false, "GetDepartmentBudgetsInGroup"}, {32, 1, false, "NeededFundingChanged"},
    {33, 1, false, "FundingPercentageChanged"}, {34, 1, false, "GetFunding(uint32)"},
    {35, 1, false, "GetFunding(department)"}, {36, 1, true, "GetTaxRate"}, {37, 2, false, "SetTaxRate"},
    {38, 1, false, "GetWeightedAssessedTaxValue"}, {39, 0, false, "GetBondIncrement"}, {40, 1, false, "IssueBond"},
    {41, 0, false, "GetTotalBorrowed"}, {42, 0, false, "GetCurrentBorrowingLimit"},
    {43, 0, false, "GetCurrentBondLimit"}, {44, 0, false, "GetCurrentMaxOutstandingBondsLimit"},
    {45, 0, false, "GetTotalMonthlyBondPayments"}, {46, 1, false, "GetAllLoans"},
    {47, 0, false, "GetLoanTimeInMonths"}, {48, 2, false, "GetMonthlyPaymentForLoan"},
    {49, 2, false, "GetFullCostOfLoan"}, {50, 2, false, "GetBudgetItemInfo"},
    {51, 3, false, "GetBudgetItemForPurpose"}, {52, 3, false, "ChangeBudgetItemLine"},
    {53, 4, false, "ChangeBudgetItemCost"}, {54, 3, false, "ChangeBudgetItemLocalFunding"},
    {55, 2, false, "AddBudgetItemToBudget"}, {56, 2, false, "RemoveBudgetItemFromBudget"},
    {57, 1, false, "GetLineItemFromBudgetItem"}, {58, 2, false, "CopyBudgetItemProperties"},
};

// ---- cISC4DepartmentBudget (no overloads: SDK order, foundations §16) --------------------------------

struct DepartmentProbe final : cISC4DepartmentBudget {
    bool QueryInterface(uint32_t, void**) override { return false; }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }
    P(3, uint32_t, GetDepartmentID, (), const)
    P(4, bool, GetDepartmentName, (cIGZString&), )
    P(5, bool, SetDepartmentName, (uint32_t, uint32_t), )
    P(6, uint32_t, GetBudgetGroup, (), const)
    P(7, bool, SetBudgetGroup, (uint32_t), )
    P(8, bool, GetIsFixedFunding, (), const)
    P(9, bool, SetFixedFunding, (bool), )
    P(10, SC4Percentage*, GetFundingPercentage, (), const)
    P(11, bool, SetFundingPercentage, (SC4Percentage const&, uint32_t), )
    P(12, SC4Percentage*, GetMaxAllowedFundingPercentage, (), const)
    P(13, bool, SetMaxAllowedFundingPercentage, (SC4Percentage const&), )
    P(14, int64_t, GetIdealMonthlyFunding, (), const)
    P(15, int64_t, GetTotalExpenses, (), const)
    P(16, int64_t, GetTotalIncome, (), const)
    P(17, cISC4LineItem*, CreateLineItem, (uint32_t, bool), )
    P(18, cISC4LineItem*, CreateLineItemForBuildingType, (uint32_t, bool), )
    P(19, bool, RemoveLineItem, (uint32_t), )
    P(20, cISC4LineItem*, GetLineItem, (uint32_t), )
    P(21, bool, GetAllLineItems, (SC4Vector<cISC4LineItem*>&), )
    P(22, bool, AddLocallyFundedObject, (cISCPropertyHolder*, uint32_t), )
    P(23, bool, RemoveLocallyFundedObject, (cISCPropertyHolder*, uint32_t), )
    P(24, bool, SetLocalFundingPercent, (cISCPropertyHolder*, SC4Percentage*, uint32_t), )
    P(25, bool, SetLocalFullFunding, (cISCPropertyHolder*, int64_t, uint32_t), )
    P(26, float, GetLocalFundingPercent, (cISCPropertyHolder*, uint32_t), )
    P(27, int64_t, GetLocalFullFunding, (cISCPropertyHolder*, uint32_t), )
    P(28, bool, IsLocallyFundedObjectInDepartment, (cISCPropertyHolder*, uint32_t), )
    P(29, bool, RecalculateAllLineItemCurrentExpenses, (), )
    P(30, int64_t, GetTotalSpending, (), const)
    P(31, bool, SetTotalSpending, (int64_t), )
    P(32, bool, GetLocallyFundedItemsByPurpose, (uint32_t, SC4List<cISCPropertyHolder*>&), )
    P(33, bool, HasLocallyFundedItemsByPurpose, (uint32_t), )
};

const Expect kDepartment[] = {
    {3, 0, false, "GetDepartmentID"}, {4, 1, false, "GetDepartmentName"}, {5, 2, false, "SetDepartmentName"},
    {6, 0, false, "GetBudgetGroup"}, {7, 1, false, "SetBudgetGroup"}, {8, 0, false, "GetIsFixedFunding"},
    {9, 1, false, "SetFixedFunding"}, {10, 0, false, "GetFundingPercentage"},
    {11, 2, false, "SetFundingPercentage"}, {12, 0, false, "GetMaxAllowedFundingPercentage"},
    {13, 1, false, "SetMaxAllowedFundingPercentage"}, {14, 0, false, "GetIdealMonthlyFunding"},
    {15, 0, false, "GetTotalExpenses"}, {16, 0, false, "GetTotalIncome"}, {17, 2, false, "CreateLineItem"},
    {18, 2, false, "CreateLineItemForBuildingType"}, {19, 1, false, "RemoveLineItem"}, {20, 1, false, "GetLineItem"},
    {21, 1, false, "GetAllLineItems"}, {22, 2, false, "AddLocallyFundedObject"},
    {23, 2, false, "RemoveLocallyFundedObject"}, {24, 3, false, "SetLocalFundingPercent"},
    {25, 4, false, "SetLocalFullFunding"}, {26, 2, true, "GetLocalFundingPercent"},
    {27, 2, false, "GetLocalFullFunding"}, {28, 2, false, "IsLocallyFundedObjectInDepartment"},
    {29, 0, false, "RecalculateAllLineItemCurrentExpenses"}, {30, 0, false, "GetTotalSpending"},
    {31, 2, false, "SetTotalSpending"}, {32, 2, false, "GetLocallyFundedItemsByPurpose"},
    {33, 1, false, "HasLocallyFundedItemsByPurpose"},
};

// ---- cISC4LineItem (no overloads: SDK order) ------------------------------------------------------------

struct LineProbe final : cISC4LineItem {
    bool QueryInterface(uint32_t, void**) override { return false; }
    uint32_t AddRef() override { return 1; }
    uint32_t Release() override { return 1; }
    P(3, uint32_t, GetID, (), const)
    P(4, Type, GetType, (), const)
    P(5, bool, SetType, (Type), )
    P(6, bool, GetName, (intptr_t&), )
    P(7, bool, SetName, (uint32_t, uint32_t), )
    P(8, int64_t, GetSecondaryInfoField, (), const)
    P(9, bool, SetSecondaryInfoField, (int64_t), )
    P(10, int64_t, GetFullExpenses, (), const)
    P(11, int64_t, GetCurrentExpenses, (), const)
    P(12, bool, SetFullExpenses, (int64_t), )
    P(13, bool, AddToFullExpenses, (int64_t), )
    P(14, int64_t, GetIncome, (), const)
    P(15, bool, SetIncome, (int64_t), )
    P(16, bool, AddToIncome, (int64_t), )
    P(17, DisplayFlag, GetDisplayFlags, (), )
    P(18, bool, SetDisplayFlag, (DisplayFlag, bool), )
    P(19, bool, IsLocalFundingItem, (), const)
};

const Expect kLineItem[] = {
    {3, 0, false, "GetID"}, {4, 0, false, "GetType"}, {5, 1, false, "SetType"}, {6, 1, false, "GetName"},
    {7, 2, false, "SetName"}, {8, 0, false, "GetSecondaryInfoField"}, {9, 2, false, "SetSecondaryInfoField"},
    {10, 0, false, "GetFullExpenses"}, {11, 0, false, "GetCurrentExpenses"}, {12, 2, false, "SetFullExpenses"},
    {13, 2, false, "AddToFullExpenses"}, {14, 0, false, "GetIncome"}, {15, 2, false, "SetIncome"},
    {16, 2, false, "AddToIncome"}, {17, 0, false, "GetDisplayFlags"}, {18, 2, false, "SetDisplayFlag"},
    {19, 0, false, "IsLocalFundingItem"},
};

}  // namespace

int main() {
    SimProbe sim;
    BudgetProbe budget;
    DepartmentProbe dept;
    LineProbe line;
    Check("cISC4Simulator", static_cast<cFC4ISC4Simulator*>(&sim), kSimulator);
    Check("cISC4BudgetSimulator", static_cast<cFC4ISC4BudgetSimulator*>(&budget), kBudget);
    Check("cISC4DepartmentBudget", static_cast<cISC4DepartmentBudget*>(&dept), kDepartment);
    Check("cISC4LineItem", static_cast<cISC4LineItem*>(&line), kLineItem);
    if (g_failures == 0) std::printf("vtable layout test: all checks passed\n");
    return g_failures == 0 ? 0 : 1;
}
