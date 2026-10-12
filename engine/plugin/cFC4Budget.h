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

// The budget subsystem's GZCOM classes (docs/specs/budget.md §1): the budget
// simulator, department budgets and line items. All behaviour lives in the
// Rust core behind fc4.h; these classes adapt the SDK interfaces to it.
#pragma once

#include "FC4Interfaces.h"
#include "FC4Streams.h"
#include "SC4Percentage.h"
#include "cIGZMessageTarget2.h"
#include "cIGZSerializable.h"
#include "cISC4BudgetSimulator.h"
#include "cISC4DepartmentBudget.h"
#include "cISC4LineItem.h"
#include "fc4.h"

#include <cstdint>

class cIGZCheatCodeManager;
class cIGZMessageServer2;
class cISC4City;

static const uint32_t kFC4_CLSID_cSC4BudgetSimulator = 0xE990BE01;
static const uint32_t kFC4_CLSID_cSC4DepartmentBudget = 0xE990BFFC;
static const uint32_t kFC4_CLSID_cSC4LineItem = 0xAA313C9F;
static const uint32_t kFC4_IID_cISC4BudgetSimulator = 0x6772BD23;
static const uint32_t kFC4_IID_cISC4DepartmentBudget = 0x223E2B67;

namespace fc4 {
// The host callbacks every budget object shares (FC4BudgetHost.cpp).
const fc4_budget_host* BudgetHost();
// The city the budget simulator serves (set by its Init, cleared by Shutdown).
void SetBudgetCity(cISC4City* city);
}  // namespace fc4

// ---- budget simulator ------------------------------------------------------------------------------

class cFC4BudgetSimulator final : public cFC4ISC4BudgetSimulator,
                                  public cIGZMessageTarget2,
                                  public cIGZSerializable,
                                  public cIFC4CoreObject {
public:
    cFC4BudgetSimulator();
    ~cFC4BudgetSimulator();

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cISC4BudgetSimulator, in the game's slot order (FC4Interfaces.h)
    bool Init() override;
    bool Shutdown() override;
    bool SetTotalFunds(int64_t llFunds) override;
    int64_t GetTotalFunds() override;
    bool DepositFunds(int64_t llFunds) override;
    bool WithdrawFunds(int64_t llFunds) override;
    int64_t GetMinAllowableFunds() override;
    int64_t GetYTDIncome() override;
    int64_t GetEstIncome() override;
    int64_t GetYTDExpenses() override;
    int64_t GetEstExpenses() override;
    int64_t GetTotalMonthlyExpense() override;
    int64_t GetTotalYearlyExpense() override;
    int64_t GetTotalMonthlyIncome() override;
    int64_t GetTotalYearlyIncome() override;
    int64_t GetTaxIncomeOfType(int32_t nTaxType) override;
    int64_t GetTaxIncome() override;
    int64_t SetTotalMonthlyExpense(int64_t llExpense) override;
    int64_t SetTotalYearlyExpense(int64_t llExpense) override;
    int64_t SetTotalMonthlyIncome(int64_t llIncome) override;
    int64_t SetTotalYearlyIncome(int64_t llIncome) override;
    bool ShowBudgetWindow() override;
    cISC4DepartmentBudget* CreateDepartmentBudget(uint32_t dwDepartmentID, uint32_t dwBudgetGroup) override;
    bool RemoveDepartmentBudget(uint32_t dwDepartmentID) override;
    cISC4DepartmentBudget* GetDepartmentBudgetByName(cIGZString const& szDepartmentName) override;
    cISC4DepartmentBudget* GetDepartmentBudget(uint32_t dwDepartmentID) override;
    bool GetAllGroups(SC4Vector<BudgetGroupInfo>& sGroups) override;
    bool SetGroupName(uint32_t dwGroupID, cIGZString& szName) override;
    bool GetDepartmentBudgetsInGroup(uint32_t dwGroupID, SC4Vector<cISC4DepartmentBudget*>& sGroups) override;
    void NeededFundingChanged(cISC4DepartmentBudget* pDepartmentBudget) override;
    void FundingPercentageChanged(cISC4DepartmentBudget* pDepartmentBudget) override;
    int64_t GetFundingById(uint32_t dwUnknownID) override;
    int64_t GetFundingOf(cISC4DepartmentBudget* pDepartmentBudget) override;
    float GetTaxRate(uint32_t dwTaxGroup) override;
    bool SetTaxRate(uint32_t dwTaxGroup, float fRate) override;
    int64_t GetWeightedAssessedTaxValue(uint32_t dwTaxGroup) override;
    int64_t GetBondIncrement() override;
    bool IssueBond(uint32_t dwBondAmount) override;
    int64_t GetTotalBorrowed() override;
    int64_t GetCurrentBorrowingLimit() override;
    int64_t GetCurrentBondLimit() override;
    int64_t GetCurrentMaxOutstandingBondsLimit() override;
    int64_t GetTotalMonthlyBondPayments() override;
    bool GetAllLoans(SC4Vector<LoanInfo>& sLoans) override;
    int32_t GetLoanTimeInMonths() override;
    int64_t GetMonthlyPaymentForLoan(int64_t llLoanAmount) override;
    int64_t GetFullCostOfLoan(int64_t llLoanAmount) override;
    bool GetBudgetItemInfo(cISCPropertyHolder* pProperty, SC4Vector<BudgetItem>& sBudgetInfo) override;
    bool GetBudgetItemForPurpose(cISCPropertyHolder* pProperty, uint32_t dwPurpose, BudgetItem& sBudgetItem) override;
    bool ChangeBudgetItemLine(cISCPropertyHolder* pProperty, uint32_t dwIndex, uint32_t dwLine) override;
    bool ChangeBudgetItemCost(cISCPropertyHolder* pProperty, uint32_t dwIndex, int64_t llCost) override;
    bool ChangeBudgetItemLocalFunding(cISCPropertyHolder* pProperty, uint32_t dwIndex,
                                      SC4Percentage const& sFunding) override;
    bool AddBudgetItemToBudget(cISCPropertyHolder* pProperty, uint32_t dwIndex) override;
    bool RemoveBudgetItemFromBudget(cISCPropertyHolder* pProperty, uint32_t dwIndex) override;
    cISC4LineItem* GetLineItemFromBudgetItem(BudgetItem& pBudgetItem) override;
    bool CopyBudgetItemProperties(cISCPropertyHolder* pOriginal, cISCPropertyHolder* pCopy) override;

    // cIGZMessageTarget2
    bool DoMessage(cIGZMessage2* pMessage) override;

    // cIGZSerializable
    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    // cIFC4CoreObject
    void* CoreHandle() override { return core_; }
    uint32_t CoreClass() override { return kFC4_CLSID_cSC4BudgetSimulator; }

    uint64_t Token() const;

private:
    bool QueryInterfaceImpl(uint32_t riid, void** ppvObj);
    void RegisterAgents(bool add);
    void RegisterCheats();
    void UnregisterCheats();
    void LogSettlement(const fc4_budget_state& before, const fc4_budget_state& after);
    void LogDepartments();

    uint32_t refCount_ = 0;
    fc4_budget* core_ = nullptr;
    cISC4City* city_ = nullptr;
    cIGZMessageServer2* msgServer_ = nullptr;
    cIGZCheatCodeManager* cheatMgr_ = nullptr;
    bool moolahRegistered_ = false;
    bool agentsRegistered_ = false;
    uint32_t messagesLogged_ = 0;
};

// ---- department budget -----------------------------------------------------------------------------

class cFC4DepartmentBudget final : public cISC4DepartmentBudget, public cIGZSerializable, public cIFC4CoreObject {
public:
    cFC4DepartmentBudget();
    ~cFC4DepartmentBudget();

    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    uint32_t GetDepartmentID() const override;
    bool GetDepartmentName(cIGZString& name) override;
    bool SetDepartmentName(uint32_t ltextGroupID, uint32_t ltextInstanceID) override;
    uint32_t GetBudgetGroup() const override;
    bool SetBudgetGroup(uint32_t budgetGroup) override;
    bool GetIsFixedFunding() const override;
    bool SetFixedFunding(bool value) override;
    SC4Percentage* GetFundingPercentage() const override;
    bool SetFundingPercentage(SC4Percentage const& percentage, uint32_t arg) override;
    SC4Percentage* GetMaxAllowedFundingPercentage() const override;
    bool SetMaxAllowedFundingPercentage(SC4Percentage const& percentage) override;
    int64_t GetIdealMonthlyFunding() const override;
    int64_t GetTotalExpenses() const override;
    int64_t GetTotalIncome() const override;
    cISC4LineItem* CreateLineItem(uint32_t lineNumber, bool isLocallyFunded) override;
    cISC4LineItem* CreateLineItemForBuildingType(uint32_t buildingIID, bool isLocallyFunded) override;
    bool RemoveLineItem(uint32_t lineNumber) override;
    cISC4LineItem* GetLineItem(uint32_t lineNumber) override;
    bool GetAllLineItems(SC4Vector<cISC4LineItem*>& destination) override;
    bool AddLocallyFundedObject(cISCPropertyHolder* pPropertyHolder, uint32_t purposeID) override;
    bool RemoveLocallyFundedObject(cISCPropertyHolder* pPropertyHolder, uint32_t purposeID) override;
    bool SetLocalFundingPercent(cISCPropertyHolder* pPropertyHolder, SC4Percentage* percentage,
                                uint32_t purposeID) override;
    bool SetLocalFullFunding(cISCPropertyHolder* pPropertyHolder, int64_t cost, uint32_t purposeID) override;
    float GetLocalFundingPercent(cISCPropertyHolder* pPropertyHolder, uint32_t purposeID) override;
    int64_t GetLocalFullFunding(cISCPropertyHolder* pPropertyHolder, uint32_t purposeID) override;
    bool IsLocallyFundedObjectInDepartment(cISCPropertyHolder* pPropertyHolder, uint32_t purposeID) override;
    bool RecalculateAllLineItemCurrentExpenses() override;
    int64_t GetTotalSpending() const override;
    bool SetTotalSpending(int64_t value) override;
    bool GetLocallyFundedItemsByPurpose(uint32_t purpose, SC4List<cISCPropertyHolder*>& list) override;
    bool HasLocallyFundedItemsByPurpose(uint32_t purpose) override;

    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    void* CoreHandle() override { return core_; }
    uint32_t CoreClass() override { return kFC4_CLSID_cSC4DepartmentBudget; }

    fc4_department* Core() const { return core_; }

private:
    bool QueryInterfaceImpl(uint32_t riid, void** ppvObj);

    uint32_t refCount_ = 0;
    fc4_department* core_ = nullptr;
    // The SDK returns the percentages by pointer; these mirror the core values.
    mutable SC4Percentage pct_{1.0f};
    mutable SC4Percentage maxPct_{1.0f};
};

// ---- line item -----------------------------------------------------------------------------------------

class cFC4LineItem final : public cISC4LineItem, public cIGZSerializable, public cIFC4CoreObject {
public:
    cFC4LineItem();
    ~cFC4LineItem();

    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    uint32_t GetID() const override;
    Type GetType() const override;
    bool SetType(Type value) override;
    bool GetName(intptr_t& name) override;
    bool SetName(uint32_t groupID, uint32_t instanceID) override;
    int64_t GetSecondaryInfoField() const override;
    bool SetSecondaryInfoField(int64_t value) override;
    int64_t GetFullExpenses() const override;
    int64_t GetCurrentExpenses() const override;
    bool SetFullExpenses(int64_t value) override;
    bool AddToFullExpenses(int64_t value) override;
    int64_t GetIncome() const override;
    bool SetIncome(int64_t value) override;
    bool AddToIncome(int64_t value) override;
    DisplayFlag GetDisplayFlags() override;
    bool SetDisplayFlag(DisplayFlag flag, bool value) override;
    bool IsLocalFundingItem() const override;

    bool Write(cIGZOStream& gzOut) override;
    bool Read(cIGZIStream& gzIn) override;
    uint32_t GetGZCLSID() override;

    void* CoreHandle() override { return core_; }
    uint32_t CoreClass() override { return kFC4_CLSID_cSC4LineItem; }

private:
    bool QueryInterfaceImpl(uint32_t riid, void** ppvObj);

    uint32_t refCount_ = 0;
    fc4_line_item* core_ = nullptr;
};

// Factories for the subsystem table (reference count 0).
cIGZUnknown* CreateFC4BudgetSimulator();
cIGZUnknown* CreateFC4DepartmentBudget();
cIGZUnknown* CreateFC4LineItem();
