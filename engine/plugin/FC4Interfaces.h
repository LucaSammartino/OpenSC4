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
// cISC4DepartmentBudget / cIGZDate / cISCPropertyHolder / cISC4LotManager /
// cISC4Lot - to call the game's objects or to implement ours.
// Besides overload order, they also fix return types the SDK gets wrong
// (cISC4DepartmentBudget slot 26).
#pragma once

#include "SC4Percentage.h"
#include "cIGZUnknown.h"
#include "SC4Rect.h"
#include "cISC4BudgetSimulator.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4Lot.h"
#include "cISC4ZoneManager.h"

#include <cstdint>
#include <map>

class cFC4IGZDate;
class cIGZDate;
class cISC4LotManager;
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
    virtual bool GetSimStartDate(cFC4IGZDate& date) = 0;      // 5
    virtual void GetSimDateParts(uint32_t* year, uint32_t* month, uint32_t* day, uint32_t* dayOfYear,
                                 uint32_t* weekDay) = 0;      // 6  GetSimDate(5 pointers)
    virtual cFC4IGZDate* GetSimDate() = 0;                    // 7  GetSimDate()
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

// ---- foundations §17: framework interfaces we implement or call ------------------------------------

class cFC4ISCPropertyHolder;
class cIGZUnknownList;
class cIGZVariant;
class cISCProperty;
class cRZTime;
class cS3DVector3;
class cISC4BusinessUnit;
class cISC4LotConfiguration;
class cISC4Occupant;
class cISC4TrafficSource;
class cSC4LotConfigurationObject;
template <typename T> class cISC4SimGrid;

// cIGZDate (cGZDate vtable, 46 slots; foundations §17.4). Implemented by
// cFC4Date. SDK error: slots 33-36 (operator- / operator+) return a uint32
// day number and do not modify the date.
class cFC4IGZDate : public cIGZUnknown {
public:
    virtual bool SetMonthNameDayYear(char const* monthName, uint32_t day, uint32_t year) = 0;  // 3  Set(char const*, d, y)
    virtual bool SetDayNumber(uint32_t dayNumber) = 0;                         // 4  Set(uint32)
    virtual bool SetTime(cRZTime const& time) = 0;                             // 5  Set(cRZTime const&)
    virtual bool SetMonthDayYear(uint32_t month, uint32_t day, uint32_t year) = 0;  // 6  Set(m, d, y)
    virtual bool SetDayOfYear(uint32_t day, uint32_t year) = 0;                // 7  Set(d, y)
    virtual bool Copy(cFC4IGZDate const& from) = 0;                            // 8
    virtual bool Clone(cFC4IGZDate** out) = 0;                                 // 9
    virtual bool Between(cFC4IGZDate const& start, cFC4IGZDate const& end) = 0;  // 10
    virtual int32_t CompareTo(cFC4IGZDate const& other) = 0;                   // 11
    virtual uint32_t WeekDay() = 0;                                            // 12
    virtual uint32_t Year() = 0;                                               // 13
    virtual uint32_t DayOfYear() = 0;                                          // 14
    virtual uint32_t DayOfMonth() = 0;                                         // 15
    virtual uint32_t FirstDayOfMonthOf(uint32_t month) = 0;                    // 16  FirstDayOfMonth(uint32)
    virtual uint32_t FirstDayOfMonth() = 0;                                    // 17  FirstDayOfMonth()
    virtual uint32_t Hash() = 0;                                               // 18
    virtual bool IsValid() = 0;                                                // 19
    virtual bool Leap() = 0;                                                   // 20
    virtual bool MaxDate(cFC4IGZDate const& other, cFC4IGZDate& out) = 0;      // 21
    virtual bool MinDate(cFC4IGZDate const& other, cFC4IGZDate& out) = 0;      // 22
    virtual uint32_t Month() = 0;                                              // 23
    virtual uint32_t Previous(uint32_t dayOfPriorWeek, cFC4IGZDate& out) = 0;  // 24
    virtual uint32_t YearLastTwoDigits() = 0;                                  // 25
    virtual uint32_t DayNumber() = 0;                                          // 26
    virtual bool IsLess(cFC4IGZDate const& other) = 0;                         // 27  operator<
    virtual bool IsLessOrEqual(cFC4IGZDate const& other) = 0;                  // 28  operator<=
    virtual bool IsGreater(cFC4IGZDate const& other) = 0;                      // 29  operator>
    virtual bool IsGreaterOrEqual(cFC4IGZDate const& other) = 0;               // 30  operator>=
    virtual bool IsEqual(cFC4IGZDate const& other) = 0;                        // 31  operator==
    virtual bool IsNotEqual(cFC4IGZDate const& other) = 0;                     // 32  operator!=
    virtual uint32_t MinusDays(int32_t days) = 0;                              // 33  operator-(int32): DayNumber() - n
    virtual uint32_t MinusDate(cFC4IGZDate const& other) = 0;                  // 34  operator-(date): DayNumber() - other
    virtual uint32_t PlusDays(int32_t days) = 0;                               // 35  operator+(int32): DayNumber() + n
    virtual uint32_t PlusDate(cFC4IGZDate const& other) = 0;                   // 36  operator+(date): DayNumber() + other
    virtual cFC4IGZDate& Increment() = 0;                                      // 37  operator++
    virtual cFC4IGZDate& Decrement() = 0;                                      // 38  operator--
    virtual cFC4IGZDate& AddDays(int32_t days) = 0;                            // 39  operator+=
    virtual cFC4IGZDate& SubtractDays(int32_t days) = 0;                       // 40  operator-=
    virtual bool DateString(cIGZString& out, uint32_t formatID) = 0;           // 41
    virtual bool DayWithinMonth(uint32_t month, uint32_t day, uint32_t year) = 0;  // 42
    virtual uint32_t DaysInYear(uint32_t year) = 0;                            // 43
    virtual uint32_t Jday(uint32_t month, uint32_t day, uint32_t year) = 0;    // 44
    virtual bool LeapYear(uint32_t year) = 0;                                  // 45
};

// cISCPropertyHolder (IID 0x25216283, 22 slots; foundations §17.2). The
// game's holders (buildings, exemplars) are only ever called through this.
// SDK errors fixed: slots 11, 12, 17, 18, 19. The bools' meaning is unknown;
// callers pass false.
class cFC4ISCPropertyHolder : public cIGZUnknown {
public:
    using EnumFn = void (*)(cISCProperty*, void*);
    using FilterFn = bool (*)(cISCProperty*, void*);

    virtual bool HasProperty(uint32_t id) const = 0;                                         // 3
    virtual bool GetPropertyList(cIGZUnknownList** list) const = 0;                          // 4
    virtual bool GetPropertyData(uint32_t id, void* buffer, uint32_t& length) const = 0;     // 5
    virtual bool GetPropertyInterface(uint32_t id, uint32_t riid, void** ppv) const = 0;     // 6
    virtual bool GetPropertyString(uint32_t id, cIGZString& value) const = 0;                // 7
    virtual bool GetPropertyU32(uint32_t id, uint32_t& value) const = 0;                     // 8
    virtual cISCProperty* GetProperty(uint32_t id) const = 0;                                // 9
    virtual bool AddPropertyData(uint32_t id, void* data, uint32_t length, bool b) = 0;      // 10
    virtual bool AddPropertyObject(uint32_t id, cIGZUnknown* value, bool b) = 0;            // 11  (SDK: int32)
    virtual bool AddPropertyString(uint32_t id, cIGZString const& value, bool b) = 0;        // 12  (SDK: no bool)
    virtual bool AddPropertyU32(uint32_t id, uint32_t value, bool b) = 0;                    // 13
    virtual bool AddPropertyVariant(uint32_t id, cIGZVariant const* value, bool b) = 0;      // 14
    virtual bool AddPropertyRecord(cISCProperty* property, bool b) = 0;                      // 15
    virtual bool CopyAddProperty(cISCProperty* property, bool b) = 0;                        // 16
    virtual bool RemoveProperty(uint32_t id, bool b) = 0;                                    // 17  (SDK: no bool)
    virtual bool RemoveAllProperties(bool b) = 0;                                            // 18  (SDK: no bool)
    virtual bool EnumPropertiesFiltered(FilterFn filter, EnumFn fn, void* data) const = 0;   // 19  (SDK: 2 args)
    virtual bool EnumProperties(EnumFn fn, void* data) const = 0;                            // 20
    virtual bool CompactProperties() = 0;                                                    // 21
};

// cISC4LotManager (28 slots; foundations §17.8). SDK error fixed: slot 22.
class cFC4ISC4LotManager : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                                     // 3
    virtual bool Shutdown() = 0;                                                 // 4
    virtual cISC4Lot* GetLotAtPoint(cS3DVector3 const& point) = 0;               // 5  GetLot(cS3DVector3 const&)
    virtual cISC4Lot* GetLot(int32_t cellX, int32_t cellZ, bool b) = 0;          // 6  GetLot(x, z, bool)
    virtual bool GetLotsInCellRect(SC4List<cISC4Lot*>& lots, int32_t x1, int32_t z1, int32_t x2, int32_t z2,
                                   bool locationOnly) = 0;                       // 7
    virtual bool GetLotsSurroundingCell(int32_t cellX, int32_t cellZ, SC4List<cISC4Lot*>& lots,
                                        int32_t flags) = 0;                      // 8  GetLotsSurroundingLot(x, z, ...)
    virtual bool GetLotsSurroundingLot(cISC4Lot const* lot, SC4List<cISC4Lot*>& lots, int32_t flags) = 0;  // 9
    virtual bool GetLotsSurroundingRect(SC4Rect<long> const& rect, SC4List<cISC4Lot*>& lots, int32_t flags) = 0;  // 10
    virtual bool GetVerticesAtEdgeOfLot(SC4Rect<long> const& cells, int32_t flags, SC4Rect<long>& vertices) = 0;  // 11
    virtual bool GetNeighborCellsAtEdgeOfLot(SC4Rect<long> const& cells, int32_t flags, SC4Rect<long>& out) = 0;  // 12
    virtual cISC4Lot* GetRandomLot() = 0;                                        // 13
    virtual cISC4Lot* GetOccupantLot(cISC4Occupant* occupant) = 0;              // 14
    virtual intptr_t GetSafeLotIterator() = 0;                                   // 15
    virtual int32_t GetLotCount() = 0;                                           // 16
    virtual int32_t GetLotSavvyTerrainVertexAltitude(int32_t x, int32_t y, cISC4Lot* lot, bool& ok) = 0;  // 17
    virtual int32_t GetLotSavvyTerrainAltitude(float x, float y, bool& ok) = 0;  // 18
    virtual int32_t GetLotSavvyCellAltitude(int32_t cellX, int32_t cellZ, bool& ok) = 0;  // 19
    virtual int32_t CanCreateLot(int32_t x, int32_t z, int32_t w, int32_t h, bool zoneNotNeeded, bool ignoreConfig,
                                 cISC4LotConfiguration* config) = 0;             // 20
    virtual int32_t CanCreatePloppedLot(SC4Rect<long> bounds, int32_t n, cISC4LotConfiguration* config) = 0;  // 21
    virtual bool CreateLotIfPossible(int32_t x, int32_t z, int32_t w, int32_t h, int32_t n, cISC4Lot*& lot,
                                     bool zoneNotNeeded, bool ignoreConfig,
                                     cISC4LotConfiguration* config) = 0;         // 22  (SDK: no lot out-parameter)
    virtual bool CreateLot(int32_t x, int32_t z, int32_t w, int32_t h, int32_t facing, cISC4Lot*& lot) = 0;  // 23
    virtual bool DeleteLot(cISC4Lot* lot) = 0;                                   // 24
    virtual bool DeleteLots(int32_t x1, int32_t z1, int32_t x2, int32_t z2) = 0;  // 25
    virtual bool GetLotVertexAltitudeAdjustmentRange(int32_t x, int32_t z, float& rangeX, float& rangeY) = 0;  // 26
    virtual cISC4SimGrid<short>* GetMaximumCapacityGrid() = 0;                   // 27
};

// cISC4Lot (70 slots; foundations §17.8). SDK error fixed: slot 61 returns
// SC4Percentage BY VALUE (hidden result pointer under MSVC).
class cFC4ISC4Lot : public cIGZUnknown {
public:
    using DeveloperType = cISC4BuildingDevelopmentSimulator::DeveloperType;
    using WealthType = cISC4BuildingOccupant::WealthType;
    using HabitationState = cISC4Lot::HabitationState;
    using VacateReason = cISC4Lot::VacateReason;

    virtual cISCPropertyHolder* AsPropertyHolder() = 0;                          // 3
    virtual cISC4TrafficSource* AsTrafficSource() = 0;                           // 4
    virtual bool Init() = 0;                                                     // 5
    virtual bool IsInitialized() const = 0;                                      // 6
    virtual bool Shutdown() = 0;                                                 // 7
    virtual bool PostCityInit() = 0;                                             // 8
    virtual cISC4LotConfiguration* GetLotConfiguration() = 0;                    // 9
    virtual bool SetLotConfiguration(cISC4LotConfiguration* config) = 0;         // 10
    virtual cSC4LotConfigurationObject* GetLotConfigurationObject(cISC4Occupant* occupant) = 0;  // 11
    virtual uint32_t GetFlags() const = 0;                                       // 12
    virtual bool SetFlags(uint32_t flags) = 0;                                   // 13
    virtual bool GetBoundingRect(SC4Rect<int32_t>& rect) const = 0;              // 14
    virtual bool GetCenter(float* center) const = 0;                             // 15
    virtual float GetAltitude() const = 0;                                       // 16
    virtual bool SetAltitude(float altitude) = 0;                                // 17
    virtual float GetSlope() const = 0;                                          // 18
    virtual float GetSlopeAtTimeOfConstruction() const = 0;                      // 19
    virtual bool UpdateSlope() = 0;                                              // 20
    virtual bool GetLocation(int32_t& x, int32_t& z) const = 0;                  // 21
    virtual bool GetSize(uint8_t& sizeX, uint8_t& sizeZ) const = 0;              // 22
    virtual int32_t GetFacing() const = 0;                                       // 23
    virtual bool SetFacing(int32_t facing) = 0;                                  // 24
    virtual cISC4BusinessUnit* GetUnit() = 0;                                    // 25
    virtual bool SetUnit(cISC4BusinessUnit* unit) = 0;                           // 26
    virtual bool HasJobAccess() const = 0;                                       // 27
    virtual bool HasRoadAccess() const = 0;                                      // 28
    virtual bool HasOrganicRoadAccess(uint32_t purposeType) const = 0;           // 29
    virtual bool UpdateRoadAccess() = 0;                                         // 30
    virtual cISC4ZoneManager::ZoneType GetZoneType() const = 0;                  // 31
    virtual bool UpdateZoneType() = 0;                                           // 32
    virtual HabitationState GetState() const = 0;                                // 33
    virtual bool SetState(HabitationState state, WealthType wealth, VacateReason reason) = 0;  // 34
    virtual uint32_t GetCondition() const = 0;                                   // 35
    virtual bool SetCondition(uint32_t condition) = 0;                           // 36
    virtual bool IsInLivableState() const = 0;                                   // 37
    virtual bool HasCapacity() const = 0;                                        // 38
    virtual bool IsInDemolishableState() const = 0;                              // 39
    virtual uint32_t GetDaysSinceLastStateChange() const = 0;                    // 40
    virtual uint32_t GetOccupantWealth() const = 0;                              // 41
    virtual uint32_t GetLandValue() const = 0;                                   // 42
    virtual bool IsHistorical() const = 0;                                       // 43
    virtual bool SetHistorical(bool historical) = 0;                             // 44
    virtual VacateReason GetVacateReason() const = 0;                            // 45
    virtual uint32_t GetBuildingType(bool forceCalculation) const = 0;           // 46
    virtual bool SetBuildingType(uint32_t buildingType) = 0;                     // 47
    virtual cISC4BuildingOccupant* GetBuilding() const = 0;                      // 48
    virtual void GetLotOccupants(SC4List<cISC4Occupant*>& occupants) const = 0;  // 49
    virtual uint16_t GetCapacityOfDeveloper(DeveloperType type, bool forceCalculation) const = 0;  // 50  GetCapacity(dev, bool)
    virtual uint16_t GetCapacity(WealthType wealth) const = 0;                   // 51  GetCapacity(wealth)
    virtual uint16_t GetAvailableCapacity(DeveloperType type, bool forceCalculation) const = 0;  // 52
    virtual uint16_t GetPopulation(DeveloperType type) const = 0;                // 53
    virtual intptr_t GetPopulations() const = 0;                                 // 54
    virtual bool SetPopulation(DeveloperType type, uint16_t population) = 0;     // 55
    virtual uint16_t GetTravelPopulation(DeveloperType type) const = 0;          // 56
    virtual bool ResetTravelPopulation() = 0;                                    // 57
    virtual bool AddTravelPopulation(cISC4Lot* lot) = 0;                         // 58
    virtual cISC4Lot* GetTravelDesignate() const = 0;                            // 59
    virtual bool SetTravelDesignate(cISC4Lot* designate) = 0;                    // 60
    virtual SC4Percentage GetOccupancy() const = 0;                              // 61  (SDK: float)
    virtual bool GetOccupancyByType(std::map<uint32_t, SC4Percentage>& out) const = 0;  // 62
    virtual float GetTravelJobs(WealthType wealth) const = 0;                    // 63
    virtual bool GetJobs(float jobs[]) const = 0;                                // 64
    virtual bool GetNeedsPower() const = 0;                                      // 65
    virtual bool GetHasPower() const = 0;                                        // 66
    virtual bool SetHasPower(bool hasPower) = 0;                                 // 67
    virtual bool GetVerticesAreLocked() const = 0;                               // 68
    virtual bool SetVerticesAreLocked(bool locked) = 0;                          // 69
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

// The same for the framework interfaces of foundations §17: objects arrive as
// SDK pointers and are called only through the verified declarations.
inline cFC4IGZDate* AsGame(cIGZDate* p) { return reinterpret_cast<cFC4IGZDate*>(p); }
inline cFC4IGZDate const* AsGame(cIGZDate const* p) { return reinterpret_cast<cFC4IGZDate const*>(p); }
inline cIGZDate* AsSdk(cFC4IGZDate* p) { return reinterpret_cast<cIGZDate*>(p); }
inline cFC4ISCPropertyHolder* AsGame(cISCPropertyHolder* p) { return reinterpret_cast<cFC4ISCPropertyHolder*>(p); }
inline cFC4ISCPropertyHolder const* AsGame(cISCPropertyHolder const* p) {
    return reinterpret_cast<cFC4ISCPropertyHolder const*>(p);
}
inline cFC4ISC4LotManager* AsGame(cISC4LotManager* p) { return reinterpret_cast<cFC4ISC4LotManager*>(p); }
inline cFC4ISC4Lot* AsGame(cISC4Lot* p) { return reinterpret_cast<cFC4ISC4Lot*>(p); }
}  // namespace fc4
