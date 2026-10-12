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

// Game interfaces with overloaded methods, declared in the game's verified
// slot order (foundations §16, simulator §5.6, budget "Verified slot order").
//
// Rule: these declarations contain NO overloads. MSVC lays out overloaded
// virtual functions in its own order (it groups them and reverses each
// group), so with overloads the slot order depends on the compiler rather
// than on the declaration. Every method here has a unique name and the vtable
// follows declaration order exactly: slot N is the N-th virtual function,
// counting QueryInterface, AddRef and Release as slots 0-2.
// tests/vtable_layout_test.cpp checks every slot against the spec tables.
//
// Use these types - never the SDK's cISC4Simulator / cISC4BudgetSimulator /
// cISC4DepartmentBudget - to call the game's objects or to implement ours.
// Besides overload order, they also fix return types the SDK gets wrong
// (cISC4DepartmentBudget slot 26).
#pragma once

#include "SC4Percentage.h"
#include "cIGZUnknown.h"
#include "cISC4BudgetSimulator.h"

#include <cstdint>

class cIGZDate;
class cIGZMessageTarget2;
class cIGZString;
class cISC4DepartmentBudget;
class cISC4LineItem;
class cISCPropertyHolder;
class SC4String;
template <typename T> class SC4List;
template <typename T> class SC4Vector;

// cISC4Simulator (IID 0x8695664E), 34 slots.
class cFC4ISC4Simulator : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                  // 3
    virtual bool Shutdown() = 0;                              // 4
    virtual bool GetSimStartDate(cIGZDate& date) = 0;         // 5
    virtual void GetSimDateParts(uint32_t* year, uint32_t* month, uint32_t* day, uint32_t* dayOfYear,
                                 uint32_t* weekDay) = 0;      // 6  GetSimDate(5 pointers)
    virtual cIGZDate* GetSimDate() = 0;                       // 7  GetSimDate()
    virtual int32_t GetSimDateNumber() = 0;                   // 8
    virtual bool Pause() = 0;                                 // 9
    virtual bool HiddenPause() = 0;                           // 10
    virtual bool EmergencyPause() = 0;                        // 11
    virtual bool Resume() = 0;                                // 12
    virtual bool HiddenResume() = 0;                          // 13
    virtual bool EmergencyResume() = 0;                       // 14
    virtual bool IsPaused() = 0;                              // 15
    virtual bool IsHiddenPaused() = 0;                        // 16
    virtual bool IsEmergencyPaused() = 0;                     // 17
    virtual bool IsAnyPaused() = 0;                           // 18
    virtual bool AddAgent(cIGZMessageTarget2* agent, uint32_t level, cIGZString const& name,
                          uint32_t flags) = 0;                // 19
    virtual bool RemoveAgentAllLevels(cIGZMessageTarget2* agent) = 0;          // 20  RemoveAgent(agent)
    virtual bool RemoveAgent(cIGZMessageTarget2* agent, uint32_t level) = 0;   // 21  RemoveAgent(agent, level)
    virtual bool RemoveAllAgentsAt(uint32_t level) = 0;       // 22  RemoveAllAgents(level)
    virtual bool RemoveAllAgents() = 0;                       // 23  RemoveAllAgents()
    virtual bool EnumerateAgentsByName(SC4Vector<SC4String>& names) = 0;  // 24
    virtual bool GetAgentEnabled(cIGZString const& name) = 0;  // 25
    virtual bool SetAgentEnabled(cIGZString const& name, bool enabled) = 0;  // 26
    virtual int32_t GetSimSpeed() = 0;                        // 27
    virtual bool SetSimSpeed(int32_t speed) = 0;              // 28
    virtual int32_t GetSimTime() = 0;                         // 29
    virtual bool SetSimTime(int32_t time) = 0;                // 30
    virtual bool SetMaxMillisecondsPerTick(uint32_t ms) = 0;  // 31
    virtual float GetAnimationTimeDilation() = 0;             // 32
    virtual bool SetCityEstablished(bool established) = 0;    // 33
};

// cISC4BudgetSimulator (IID 0x6772BD23), 59 slots. GetTotalMonthlyExpense
// and GetTotalMonthlyIncome return 64-bit values (budget §6 [compat]): in
// EDX:EAX, so callers using a 32-bit declaration still see the low 32 bits.
class cFC4ISC4BudgetSimulator : public cIGZUnknown {
public:
    using BudgetGroupInfo = cISC4BudgetSimulator::BudgetGroupInfo;
    using BudgetItem = cISC4BudgetSimulator::BudgetItem;
    using LoanInfo = cISC4BudgetSimulator::LoanInfo;

    virtual bool Init() = 0;                                  // 3
    virtual bool Shutdown() = 0;                              // 4
    virtual bool SetTotalFunds(int64_t funds) = 0;            // 5
    virtual int64_t GetTotalFunds() = 0;                      // 6
    virtual bool DepositFunds(int64_t funds) = 0;             // 7
    virtual bool WithdrawFunds(int64_t funds) = 0;            // 8
    virtual int64_t GetMinAllowableFunds() = 0;               // 9
    virtual int64_t GetYTDIncome() = 0;                       // 10
    virtual int64_t GetEstIncome() = 0;                       // 11
    virtual int64_t GetYTDExpenses() = 0;                     // 12
    virtual int64_t GetEstExpenses() = 0;                     // 13
    virtual int64_t GetTotalMonthlyExpense() = 0;             // 14  (SDK: uint32_t)
    virtual int64_t GetTotalYearlyExpense() = 0;              // 15
    virtual int64_t GetTotalMonthlyIncome() = 0;              // 16  (SDK: int32_t)
    virtual int64_t GetTotalYearlyIncome() = 0;               // 17
    virtual int64_t GetTaxIncomeOfType(int32_t taxType) = 0;  // 18  GetTaxIncome(int32)
    virtual int64_t GetTaxIncome() = 0;                       // 19  GetTaxIncome()
    virtual int64_t SetTotalMonthlyExpense(int64_t v) = 0;    // 20
    virtual int64_t SetTotalYearlyExpense(int64_t v) = 0;     // 21
    virtual int64_t SetTotalMonthlyIncome(int64_t v) = 0;     // 22
    virtual int64_t SetTotalYearlyIncome(int64_t v) = 0;      // 23
    virtual bool ShowBudgetWindow() = 0;                      // 24
    virtual cISC4DepartmentBudget* CreateDepartmentBudget(uint32_t id, uint32_t group) = 0;  // 25
    virtual bool RemoveDepartmentBudget(uint32_t id) = 0;     // 26
    virtual cISC4DepartmentBudget* GetDepartmentBudgetByName(cIGZString const& name) = 0;  // 27
    virtual cISC4DepartmentBudget* GetDepartmentBudget(uint32_t id) = 0;  // 28
    virtual bool GetAllGroups(SC4Vector<BudgetGroupInfo>& groups) = 0;    // 29
    virtual bool SetGroupName(uint32_t id, cIGZString& name) = 0;         // 30
    virtual bool GetDepartmentBudgetsInGroup(uint32_t id, SC4Vector<cISC4DepartmentBudget*>& out) = 0;  // 31
    virtual void NeededFundingChanged(cISC4DepartmentBudget* dept) = 0;      // 32
    virtual void FundingPercentageChanged(cISC4DepartmentBudget* dept) = 0;  // 33
    virtual int64_t GetFundingById(uint32_t id) = 0;          // 34  GetFunding(uint32) (same body as 35)
    virtual int64_t GetFundingOf(cISC4DepartmentBudget* dept) = 0;  // 35  GetFunding(dept)
    virtual float GetTaxRate(uint32_t group) = 0;             // 36
    virtual bool SetTaxRate(uint32_t group, float rate) = 0;  // 37
    virtual int64_t GetWeightedAssessedTaxValue(uint32_t group) = 0;  // 38
    virtual int64_t GetBondIncrement() = 0;                   // 39
    virtual bool IssueBond(uint32_t amount) = 0;              // 40
    virtual int64_t GetTotalBorrowed() = 0;                   // 41
    virtual int64_t GetCurrentBorrowingLimit() = 0;           // 42
    virtual int64_t GetCurrentBondLimit() = 0;                // 43
    virtual int64_t GetCurrentMaxOutstandingBondsLimit() = 0;  // 44
    virtual int64_t GetTotalMonthlyBondPayments() = 0;        // 45
    virtual bool GetAllLoans(SC4Vector<LoanInfo>& loans) = 0;  // 46
    virtual int32_t GetLoanTimeInMonths() = 0;                // 47
    virtual int64_t GetMonthlyPaymentForLoan(int64_t loan) = 0;  // 48
    virtual int64_t GetFullCostOfLoan(int64_t loan) = 0;      // 49
    virtual bool GetBudgetItemInfo(cISCPropertyHolder* holder, SC4Vector<BudgetItem>& out) = 0;  // 50
    virtual bool GetBudgetItemForPurpose(cISCPropertyHolder* holder, uint32_t purpose, BudgetItem& item) = 0;  // 51
    virtual bool ChangeBudgetItemLine(cISCPropertyHolder* holder, uint32_t index, uint32_t line) = 0;  // 52
    virtual bool ChangeBudgetItemCost(cISCPropertyHolder* holder, uint32_t index, int64_t cost) = 0;  // 53
    virtual bool ChangeBudgetItemLocalFunding(cISCPropertyHolder* holder, uint32_t index,
                                              SC4Percentage const& funding) = 0;  // 54
    virtual bool AddBudgetItemToBudget(cISCPropertyHolder* holder, uint32_t index) = 0;  // 55
    virtual bool RemoveBudgetItemFromBudget(cISCPropertyHolder* holder, uint32_t index) = 0;  // 56
    virtual cISC4LineItem* GetLineItemFromBudgetItem(BudgetItem& item) = 0;  // 57
    virtual bool CopyBudgetItemProperties(cISCPropertyHolder* src, cISCPropertyHolder* dst) = 0;  // 58
};

// cISC4DepartmentBudget (IID 0x223E2B67), 34 slots. No overloads; this
// declaration exists because the SDK declares slot 26 with the wrong return
// type (budget "Erratum: GetLocalFundingPercent returns a pointer").
class cFC4ISC4DepartmentBudget : public cIGZUnknown {
public:
    virtual uint32_t GetDepartmentID() const = 0;             // 3
    virtual bool GetDepartmentName(cIGZString& name) = 0;     // 4
    virtual bool SetDepartmentName(uint32_t ltextGroupID, uint32_t ltextInstanceID) = 0;  // 5
    virtual uint32_t GetBudgetGroup() const = 0;              // 6
    virtual bool SetBudgetGroup(uint32_t budgetGroup) = 0;    // 7
    virtual bool GetIsFixedFunding() const = 0;               // 8
    virtual bool SetFixedFunding(bool value) = 0;             // 9
    virtual SC4Percentage* GetFundingPercentage() const = 0;  // 10
    virtual bool SetFundingPercentage(SC4Percentage const& percentage, uint32_t lineItem) = 0;  // 11
    virtual SC4Percentage* GetMaxAllowedFundingPercentage() const = 0;  // 12
    virtual bool SetMaxAllowedFundingPercentage(SC4Percentage const& percentage) = 0;  // 13
    virtual int64_t GetIdealMonthlyFunding() const = 0;       // 14
    virtual int64_t GetTotalExpenses() const = 0;             // 15
    virtual int64_t GetTotalIncome() const = 0;               // 16
    virtual cISC4LineItem* CreateLineItem(uint32_t lineNumber, bool isLocallyFunded) = 0;  // 17
    virtual cISC4LineItem* CreateLineItemForBuildingType(uint32_t buildingIID, bool isLocallyFunded) = 0;  // 18
    virtual bool RemoveLineItem(uint32_t lineNumber) = 0;     // 19
    virtual cISC4LineItem* GetLineItem(uint32_t lineNumber) = 0;  // 20
    virtual bool GetAllLineItems(SC4Vector<cISC4LineItem*>& destination) = 0;  // 21
    virtual bool AddLocallyFundedObject(cISCPropertyHolder* holder, uint32_t purpose) = 0;  // 22
    virtual bool RemoveLocallyFundedObject(cISCPropertyHolder* holder, uint32_t purpose) = 0;  // 23
    virtual bool SetLocalFundingPercent(cISCPropertyHolder* holder, SC4Percentage* percentage,
                                        uint32_t purpose) = 0;  // 24
    virtual bool SetLocalFullFunding(cISCPropertyHolder* holder, int64_t cost, uint32_t purpose) = 0;  // 25
    // 26: returns a POINTER in EAX (SDK: float). Never null.
    virtual SC4Percentage const* GetLocalFundingPercentPtr(cISCPropertyHolder* holder, uint32_t purpose) = 0;
    virtual int64_t GetLocalFullFunding(cISCPropertyHolder* holder, uint32_t purpose) = 0;  // 27
    virtual bool IsLocallyFundedObjectInDepartment(cISCPropertyHolder* holder, uint32_t purpose) = 0;  // 28
    virtual bool RecalculateAllLineItemCurrentExpenses() = 0;  // 29
    virtual int64_t GetTotalSpending() const = 0;             // 30
    virtual bool SetTotalSpending(int64_t value) = 0;         // 31
    virtual bool GetLocallyFundedItemsByPurpose(uint32_t purpose, SC4List<cISCPropertyHolder*>& list) = 0;  // 32
    virtual bool HasLocallyFundedItemsByPurpose(uint32_t purpose) = 0;  // 33
};

class cISC4Simulator;

namespace fc4 {
// A simulator handed out by the game (cISC4City::GetSimulator), seen through
// the verified slot order instead of the SDK's declaration.
inline cFC4ISC4Simulator* AsGame(cISC4Simulator* p) { return reinterpret_cast<cFC4ISC4Simulator*>(p); }

// Department budgets travel as the SDK's cISC4DepartmentBudget* (the type the
// SDK's budget signatures and containers use). Both are single-inheritance
// interfaces whose vtable is at offset 0, so the pointer is the same; only
// the view of the slots differs. Never call slot 26 through the SDK type.
inline cFC4ISC4DepartmentBudget* AsGame(cISC4DepartmentBudget* p) {
    return reinterpret_cast<cFC4ISC4DepartmentBudget*>(p);
}
inline cISC4DepartmentBudget* AsSdk(cFC4ISC4DepartmentBudget* p) { return reinterpret_cast<cISC4DepartmentBudget*>(p); }
}  // namespace fc4
