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
// cISC4Lot / cIGZVariant / cISC4NeighborsSimulator / cISC4Ordinance /
// cISC4Demand / cISC4DemandSimulator / cISC4ResidentialSimulator /
// cISC4RegionalCity - to call the game's objects or to implement ours.
// tests/abi_conformance_test.cpp checks every declaration against
// docs/abi/interfaces.tsv (callee-pop bytes and return channel per slot).
// Besides overload order, they also fix return types the SDK gets wrong
// (cISC4DepartmentBudget slot 26).
#pragma once

#include "SC4Percentage.h"
#include "cIGZUnknown.h"
#include "SC4Rect.h"
#include "cISC4BudgetSimulator.h"
#include "cISC4BuildingDevelopmentSimulator.h"
#include "cISC4BuildingOccupant.h"
#include "cISC4City.h"
#include "cISC4Lot.h"
#include "cISC4NeighborsSimulator.h"
#include "cISC4ZoneManager.h"

#include <cstdint>
#include <map>

class cFC4IGZDate;
class cIGZDate;
class cISC4LotManager;
class cIGZMessageTarget2;
class cIGZString;
class cIGZVariant;
class cISC4DepartmentBudget;
class cISC4LineItem;
class cISCPropertyHolder;
class SC4String;
class cISC4Demand;
class cISC4DemandSimulator;
class cISC4NeighborDeal;
class SC4NewCityPreferences;
template <typename T> class SC4List;
template <typename T> class SC4Vector;
template <typename T> class cISC4SimGrid;

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
    // 20-23: the SDK returns int64; docs/abi says the game's methods return 32
    // bits in eax. [choice] ours return the low 32 bits of the stored value
    // (budget spec: "return v"), which is the eax an int64 return gave too.
    virtual int32_t SetTotalMonthlyExpense(int64_t v) = 0;    // 20
    virtual int32_t SetTotalYearlyExpense(int64_t v) = 0;     // 21
    virtual int32_t SetTotalMonthlyIncome(int64_t v) = 0;     // 22
    virtual int32_t SetTotalYearlyIncome(int64_t v) = 0;      // 23
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

// cIGZVariant (IID 0x60FD4D2B, 127 slots; foundations §17.1),
// in compiled order with unique names. SDK errors fixed: slot 8 takes the
// type, slot 60 takes (riid, void**). Overload groups as compiled: the
// value getter comes before GetValXTo(X&), the const AsX() before AsXRef(),
// and the SetRefX(X*, count) setter before the RefX() getter.
class cFC4IGZVariant : public cIGZUnknown {
public:
    virtual uint16_t GetType() const = 0;                                                  // 3
    virtual uint32_t GetCount() const = 0;                                                 // 4
    virtual bool Erase() = 0;                                                              // 5
    virtual bool CopyFrom(cFC4IGZVariant const& value) = 0;                                // 6
    virtual bool CreateCopy(cFC4IGZVariant** value) const = 0;                             // 7
    virtual bool CanGetVal(uint16_t type) const = 0;                                       // 8
    virtual bool GetValBool() const = 0;                                                   // 9
    virtual bool GetValBoolTo(bool& value) const = 0;                                      // 10
    virtual void SetValBool(bool value) = 0;                                               // 11
    virtual uint8_t GetValUint8() const = 0;                                               // 12
    virtual bool GetValUint8To(uint8_t& value) const = 0;                                  // 13
    virtual void SetValUint8(uint8_t value) = 0;                                           // 14
    virtual int8_t GetValSint8() const = 0;                                                // 15
    virtual bool GetValSint8To(int8_t& value) const = 0;                                   // 16
    virtual void SetValSint8(int8_t value) = 0;                                            // 17
    virtual uint16_t GetValUint16() const = 0;                                             // 18
    virtual bool GetValUint16To(uint16_t& value) const = 0;                                // 19
    virtual void SetValUint16(uint16_t value) = 0;                                         // 20
    virtual int16_t GetValSint16() const = 0;                                              // 21
    virtual bool GetValSint16To(int16_t& value) const = 0;                                 // 22
    virtual void SetValSint16(int16_t value) = 0;                                          // 23
    virtual uint32_t GetValUint32() const = 0;                                             // 24
    virtual bool GetValUint32To(uint32_t& value) const = 0;                                // 25
    virtual void SetValUint32(uint32_t value) = 0;                                         // 26
    virtual int32_t GetValSint32() const = 0;                                              // 27
    virtual bool GetValSint32To(int32_t& value) const = 0;                                 // 28
    virtual void SetValSint32(int32_t value) = 0;                                          // 29
    virtual uint64_t GetValUint64() const = 0;                                             // 30
    virtual bool GetValUint64To(uint64_t& value) const = 0;                                // 31
    virtual void SetValUint64(uint64_t value) = 0;                                         // 32
    virtual int64_t GetValSint64() const = 0;                                              // 33
    virtual bool GetValSint64To(int64_t& value) const = 0;                                 // 34
    virtual void SetValSint64(int64_t value) = 0;                                          // 35
    virtual float GetValFloat32() const = 0;                                               // 36
    virtual bool GetValFloat32To(float& value) const = 0;                                  // 37
    virtual void SetValFloat32(float value) = 0;                                           // 38
    virtual double GetValFloat64() const = 0;                                              // 39
    virtual bool GetValFloat64To(double& value) const = 0;                                 // 40
    // [choice] docs/abi: the game's SetValFloat64 returns a double in ST0
    // (x87, high confidence; the SDK says void). The value is not specified;
    // ours returns the value stored.
    virtual double SetValFloat64(double value) = 0;                                        // 41
    virtual char GetValChar() const = 0;                                                   // 42
    virtual bool GetValCharTo(char& value) const = 0;                                      // 43
    virtual void SetValChar(char value) = 0;                                               // 44
    virtual uint16_t GetValRZUnicodeChar() const = 0;                                      // 45
    virtual bool GetValRZUnicodeCharTo(uint16_t& value) const = 0;                         // 46
    virtual void SetValRZUnicodeChar(uint16_t value) = 0;                                  // 47
    virtual char GetValRZChar() const = 0;                                                 // 48
    virtual bool GetValRZCharTo(char& value) const = 0;                                    // 49
    virtual void SetValRZChar(char value) = 0;                                             // 50
    virtual void* GetValVoidPtr() const = 0;                                               // 51
    virtual bool GetValVoidPtrTo(void** value) const = 0;                                  // 52
    virtual void SetValVoidPtr(void* value) = 0;                                           // 53
    virtual bool CreateValString(cIGZString** value) const = 0;                            // 54
    virtual bool GetValString(cIGZString& value) const = 0;                                // 55
    virtual void SetValString(cIGZString const& value) = 0;                                // 56
    virtual bool GetValVoid(void* value, uint32_t& length) const = 0;                      // 57
    virtual void SetValVoid(void* value, uint32_t length) = 0;                             // 58
    virtual cIGZUnknown* GetValUnknown() = 0;                                              // 59
    virtual bool GetValUnknownAs(uint32_t riid, void** ppvObj) = 0;                        // 60
    virtual void SetValUnknown(cIGZUnknown* value) = 0;                                    // 61
    virtual bool AsBool() const = 0;                                                       // 62
    virtual bool& AsBoolRef() = 0;                                                         // 63
    virtual uint8_t AsUint8() const = 0;                                                   // 64
    virtual uint8_t& AsUint8Ref() = 0;                                                     // 65
    virtual int8_t AsSint8() const = 0;                                                    // 66
    virtual int8_t& AsSint8Ref() = 0;                                                      // 67
    virtual uint16_t AsUint16() const = 0;                                                 // 68
    virtual uint16_t& AsUint16Ref() = 0;                                                   // 69
    virtual int16_t AsSint16() const = 0;                                                  // 70
    virtual int16_t& AsSint16Ref() = 0;                                                    // 71
    virtual uint32_t AsUint32() const = 0;                                                 // 72
    virtual uint32_t& AsUint32Ref() = 0;                                                   // 73
    virtual int32_t AsSint32() const = 0;                                                  // 74
    virtual int32_t& AsSint32Ref() = 0;                                                    // 75
    virtual uint64_t AsUint64() const = 0;                                                 // 76
    virtual uint64_t& AsUint64Ref() = 0;                                                   // 77
    virtual int64_t AsSint64() const = 0;                                                  // 78
    virtual int64_t& AsSint64Ref() = 0;                                                    // 79
    virtual float AsFloat32() const = 0;                                                   // 80
    virtual float& AsFloat32Ref() = 0;                                                     // 81
    virtual double AsFloat64() const = 0;                                                  // 82
    virtual double& AsFloat64Ref() = 0;                                                    // 83
    virtual char AsChar() const = 0;                                                       // 84
    virtual char& AsCharRef() = 0;                                                         // 85
    virtual uint16_t AsRZUnicodeChar() const = 0;                                          // 86
    virtual uint16_t& AsRZUnicodeCharRef() = 0;                                            // 87
    virtual char AsRZChar() const = 0;                                                     // 88
    virtual char& AsRZCharRef() = 0;                                                       // 89
    virtual void* AsVoidPtr() const = 0;                                                   // 90
    virtual void*& AsVoidPtrRef() = 0;                                                     // 91
    virtual void SetRefBool(bool* value, uint32_t length) = 0;                             // 92
    virtual bool* RefBool() const = 0;                                                     // 93
    virtual void SetRefUint8(uint8_t* value, uint32_t length) = 0;                         // 94
    virtual uint8_t* RefUint8() const = 0;                                                 // 95
    virtual void SetRefSint8(int8_t* value, uint32_t length) = 0;                          // 96
    virtual int8_t* RefSint8() const = 0;                                                  // 97
    virtual void SetRefUint16(uint16_t* value, uint32_t length) = 0;                       // 98
    virtual uint16_t* RefUint16() const = 0;                                               // 99
    virtual void SetRefSint16(int16_t* value, uint32_t length) = 0;                        // 100
    virtual int16_t* RefSint16() const = 0;                                                // 101
    virtual void SetRefUint32(uint32_t* value, uint32_t length) = 0;                       // 102
    virtual uint32_t* RefUint32() const = 0;                                               // 103
    virtual void SetRefSint32(int32_t* value, uint32_t length) = 0;                        // 104
    virtual int32_t* RefSint32() const = 0;                                                // 105
    virtual void SetRefUint64(uint64_t* value, uint32_t length) = 0;                       // 106
    virtual uint64_t* RefUint64() const = 0;                                               // 107
    virtual void SetRefSint64(int64_t* value, uint32_t length) = 0;                        // 108
    virtual int64_t* RefSint64() const = 0;                                                // 109
    virtual void SetRefFloat32(float* value, uint32_t length) = 0;                         // 110
    virtual float* RefFloat32() const = 0;                                                 // 111
    virtual void SetRefFloat64(double* value, uint32_t length) = 0;                        // 112
    virtual double* RefFloat64() const = 0;                                                // 113
    virtual void SetRefChar(char* value, uint32_t length) = 0;                             // 114
    virtual char* RefChar() const = 0;                                                     // 115
    virtual void SetRefRZUnicodeChar(uint16_t* value, uint32_t length) = 0;                // 116
    virtual uint16_t* RefRZUnicodeChar() const = 0;                                        // 117
    virtual void SetRefRZChar(char* value, uint32_t length) = 0;                           // 118
    virtual char* RefRZChar() const = 0;                                                   // 119
    virtual void SetRefVoidPtr(void** value, uint32_t length) = 0;                         // 120
    virtual void** RefVoidPtr() const = 0;                                                 // 121
    virtual void SetRefVoid(void* value, uint32_t length) = 0;                             // 122
    virtual void* RefVoid() const = 0;                                                     // 123
    virtual void SetRefIGZUnknown(cIGZUnknown* value) = 0;                                 // 124
    virtual bool RefIGZUnknownAs(uint32_t riid, void** ppvObj) = 0;                        // 125
    virtual cIGZUnknown* RefIGZUnknown() const = 0;                                        // 126
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
    virtual bool AddPropertyVariant(uint32_t id, cFC4IGZVariant const* value, bool b) = 0;   // 14
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
    // docs/abi: returns in ST0 (x87, high confidence; the SDK says bool). What
    // the value is is not specified; the plugin does not call it.
    virtual float GetCenter(float* center) const = 0;                            // 15
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

// cISC4NeighborsSimulator (38 slots), from the SDK declaration with the
// return channels of docs/abi/interfaces.tsv. SDK errors fixed: slots 32/33
// return 32 bits in eax (SDK: int64, so the high half read from edx was
// garbage), slot 35 returns in ST0 (SDK: int64). Overload groups as compiled
// (13/14 and 29/30, checked by abi_conformance_test).
class cFC4ISC4NeighborsSimulator : public cIGZUnknown {
public:
    using ConnectionType = cISC4NeighborsSimulator::ConnectionType;

    virtual bool Init() = 0;                                                     // 3
    virtual bool Shutdown() = 0;                                                 // 4
    virtual bool RequestCreateConnection(ConnectionType type, int32_t x, int32_t z, bool flag) = 0;  // 5
    virtual cISC4NeighborConnection* CreateConnection(ConnectionType type, int32_t x, int32_t z, bool flag) = 0;  // 6
    virtual void DeleteConnection(ConnectionType type, int32_t x, int32_t z) = 0;  // 7
    virtual void DeleteOccupantConnection(cISC4Occupant* occupant) = 0;          // 8
    virtual void DeleteAllConnectionsOfType(ConnectionType type) = 0;            // 9
    virtual bool HasConnection(ConnectionType type, int32_t x, int32_t z) const = 0;  // 10
    virtual cISC4NeighborConnection* FindConnection(ConnectionType type, int32_t x, int32_t z) = 0;  // 11
    virtual cISC4RegionalCity* FindCityOnOtherSideOfConnection(int32_t x, int32_t z) = 0;  // 12
    virtual cISC4NeighborConnection* FindConnectionOnOtherSideOf(cISC4NeighborConnection* connection) = 0;  // 13
    virtual cISC4NeighborConnection* FindConnectionOnOtherSideAt(ConnectionType type, int32_t x,
                                                                 int32_t z) = 0;  // 14
    virtual cISC4NeighborConnection* FindUnvisitedConnection(ConnectionType type,
                                                             cISC4RegionalCity const* city) = 0;  // 15
    virtual void GetAllConnections(SC4List<cISC4NeighborConnection*>& list) const = 0;  // 16
    virtual void GetConnectionsOfType(ConnectionType type, SC4List<cISC4NeighborConnection*>& list) const = 0;  // 17
    virtual void GetConnectionsOfTypeToCity(ConnectionType type, cISC4RegionalCity const* city,
                                            SC4List<cISC4NeighborConnection*>& list) const = 0;  // 18
    virtual void GetConnectionsToCity(cISC4RegionalCity const* city, SC4List<cISC4NeighborConnection*>& list) = 0;  // 19
    virtual void GetAdjacentCities(SC4List<cISC4RegionalCity*>& list, bool includeUnestablished) = 0;  // 20
    virtual void GetConnectedCities(SC4List<cISC4RegionalCity*>& list, bool includeUnestablished) = 0;  // 21
    virtual void GetAllConnectedCities(SC4List<cISC4RegionalCity*>& list, bool includeUnestablished) = 0;  // 22
    virtual cISC4NeighborConnection* FindConnectionSuitableForDeal(int32_t dealType,
                                                                   cISC4RegionalCity const* city) = 0;  // 23
    virtual bool ProposeDeal(int32_t dealType, cISC4RegionalCity* otherCity, cISC4NeighborDeal** outDeal) = 0;  // 24
    virtual bool ProposeDeals(SC4List<cISC4NeighborConnection*>& deals, bool flag) = 0;  // 25
    virtual bool AddDeal(cISC4NeighborDeal* deal) = 0;                           // 26
    virtual void SetUIDisableFlag(bool flag) = 0;                                // 27
    virtual void FindDealsAll(SC4List<cISC4NeighborConnection*>& deals) = 0;     // 28
    virtual void FindDealsWithCity(int32_t dealType, cISC4RegionalCity* city,
                                   SC4List<cISC4NeighborConnection*>& deals) = 0;  // 29  FindDeals(3 args)
    virtual void FindDealsOfType(int32_t dealType, SC4List<cISC4NeighborConnection*>& deals) = 0;  // 30  FindDeals(2 args)
    virtual void MarkDealsDirty() = 0;                                           // 31
    // [choice] signed 32-bit money (sign-extended by the caller); spec question.
    virtual int32_t GetTotalMonthlyNeighborDealIncome() = 0;                     // 32  (SDK: int64)
    virtual int32_t GetTotalMonthlyNeighborDealExpense() = 0;                    // 33  (SDK: int64)
    virtual void UpdateCommodityStats(cISC4City* city) = 0;                      // 34
    virtual double GetDealCost(int32_t dealType, double amount, cISC4RegionalCity const* city) = 0;  // 35  (SDK: int64)
    // [choice] the table says x87|eax (low confidence); not called.
    virtual double GetDealMaxAmount(int32_t dealType, cISC4RegionalCity const* city) = 0;  // 36  (SDK: int64)
    virtual cISCResExemplar* GetTuningExemplar() = 0;                            // 37
};

// cISC4WeatherSimulator (IID 0xC6C66245), 24 slots (weather spec "Compiled
// slot tables": the SDK order). Implemented by cFC4WeatherSimulator. Two
// return channels differ from the SDK (docs/abi: CellToPosition returns in
// ST0, StandardCityCellToCell in EDX:EAX); no game code calls either
// (weather §11.2), but a replacement keeps the channels so a caller's x87
// stack stays balanced. GetMoistureValueMapPtr takes the caller's pointer
// variable (§11.2 [compat]; the SDK says a vector reference).
struct FC4ByteGridVector;  // SC4Vector<SC4Vector<uint8_t>>-compatible (cFC4Weather.h)
class cS3DVector2;
class cFC4ISC4WeatherSimulator : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                                      // 3
    virtual bool Shutdown() = 0;                                                  // 4
    virtual bool DoPostCityInit() = 0;                                            // 5
    virtual float CellWidthX() = 0;                                               // 6
    virtual float CellWidthZ() = 0;                                               // 7
    virtual uint32_t CellCountX() = 0;                                            // 8
    virtual uint32_t CellCountZ() = 0;                                            // 9
    virtual int32_t PositionToCell(float x, float z, int32_t& cx, int32_t& cz) = 0;     // 10
    virtual float CellToPosition(int32_t cx, int32_t cz, float& x, float& z) = 0;       // 11  (SDK: void)
    virtual void CellToStandardCityCell(int32_t cx, int32_t cz, int32_t& sx, int32_t& sz) = 0;  // 12
    virtual int64_t StandardCityCellToCell(int32_t sx, int32_t sz, int32_t& cx, int32_t& cz) = 0;  // 13  (SDK: void)
    virtual int32_t GetCurrentSeason() = 0;                                       // 14  (SDK: enum Season)
    virtual int32_t GetCurrentSeasonMonth() = 0;                                  // 15
    virtual void UpdateMoisture(SC4Rect<int32_t> const& rect) = 0;                // 16
    virtual uint8_t GetMoistureValue(float x, float z) = 0;                       // 17
    virtual void GetMoistureValueMapPtr(FC4ByteGridVector*& out) = 0;             // 18
    virtual uint8_t GetMoistureType(float x, float z) = 0;                        // 19
    virtual void SetManualMoistureAddition(float x0, float z0, float x1, float z1, float v) = 0;  // 20
    virtual float GetHumidity(float x, float z) = 0;                              // 21
    virtual float GetAmbientWind(cS3DVector2& wind) = 0;                          // 22
    virtual float GetWindAtCell(int32_t x, int32_t z, cS3DVector2& wind) = 0;     // 23
};

// cISC4Ordinance (IID 0xAA5E2247), 30 slots (ordinance spec "Compiled slot
// tables": the SDK order). Implemented by cFC4Ordinance and called on foreign
// ordinances (DLL plugins). One return channel differs from the SDK: slot 10
// returns the SC4Percentage's f32 bits in EAX and pops nothing (docs/abi); the
// SDK's by-value SC4Percentage would be a hidden result pointer under MSVC.
class cFC4ISC4Ordinance : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                                      // 3
    virtual bool Shutdown() = 0;                                                  // 4
    virtual int64_t GetCurrentMonthlyIncome() = 0;                                // 5
    virtual uint32_t GetID() const = 0;                                           // 6
    virtual cIGZString* GetName() = 0;                                            // 7
    virtual cIGZString* GetDescription() = 0;                                     // 8
    virtual uint32_t GetYearFirstAvailable() = 0;                                 // 9
    virtual uint32_t GetChanceAvailability() = 0;                                 // 10  (SDK: SC4Percentage)
    virtual int64_t GetEnactmentIncome() = 0;                                     // 11
    virtual int64_t GetRetracmentIncome() = 0;                                    // 12
    virtual int64_t GetMonthlyConstantIncome() = 0;                               // 13
    virtual float GetMonthlyIncomeFactor() = 0;                                   // 14
    virtual uint32_t GetAdvisorID() = 0;                                          // 15
    virtual cISCPropertyHolder* GetMiscProperties() = 0;                          // 16
    virtual bool IsAvailable() = 0;                                               // 17
    virtual bool IsOn() = 0;                                                      // 18
    virtual bool IsEnabled() = 0;                                                 // 19
    virtual int64_t GetMonthlyAdjustedIncome() = 0;                               // 20
    virtual bool CheckConditions() = 0;                                           // 21
    virtual bool IsIncomeOrdinance() = 0;                                         // 22
    virtual bool Simulate() = 0;                                                  // 23
    virtual bool SetAvailable(bool isAvailable) = 0;                              // 24
    virtual bool SetOn(bool isOn) = 0;                                            // 25
    virtual bool SetEnabled(bool isEnabled) = 0;                                  // 26
    virtual bool ForceAvailable(bool isAvailable) = 0;                            // 27
    virtual bool ForceOn(bool isOn) = 0;                                          // 28
    virtual bool ForceMonthlyAdjustedIncome(int64_t monthlyAdjustedIncome) = 0;   // 29
};

// cISC4Demand (IID 0x0797A4E8), 34 slots (demand spec §3.2, "Compiled slot
// tables": the SDK order). Implemented by cFC4Demand. Return channels as the
// game has them (docs/abi): AddToSupplyValue / AddToDemandValue return in ST0
// (SDK: bool), EndOfCycle returns a bool in EAX (SDK: float; §3.2 [compat]).
class cFC4ISC4Demand : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                                      // 3
    virtual bool Shutdown() = 0;                                                  // 4
    virtual bool SimulationBegin() = 0;                                           // 5
    virtual uint32_t GetId() const = 0;                                           // 6
    virtual bool SetId(uint32_t id) = 0;                                          // 7
    virtual float QuerySupplyValue() const = 0;                                   // 8
    virtual float QueryDemandValue() const = 0;                                   // 9
    virtual float QueryNewSupply() const = 0;                                     // 10
    virtual float QueryNewDemand() const = 0;                                     // 11
    virtual float QueryActiveDemandValue() const = 0;                             // 12
    virtual float QueryEconomyModifier() const = 0;                               // 13
    virtual float QueryActiveDemandMax() const = 0;                               // 14
    virtual float QueryActiveDemandMin() const = 0;                               // 15
    virtual float AddToSupplyValue(float value) = 0;                              // 16  (SDK: bool)
    virtual float AddToDemandValue(float value) = 0;                              // 17  (SDK: bool)
    virtual bool SetSupplyValue(float value) = 0;                                 // 18
    virtual bool SetDemandValue(float value) = 0;                                 // 19
    virtual bool SetActiveDemandMax(float value) = 0;                             // 20
    virtual bool SetActiveDemandMin(float value) = 0;                             // 21
    virtual float GetEconomyModifier() const = 0;                                 // 22
    virtual bool SetEconomyModifier(float value) = 0;                             // 23
    virtual float GetTaxModifier() const = 0;                                     // 24
    virtual bool SetTaxModifier(float value) = 0;                                 // 25
    virtual SC4Percentage* GetDemandCap() const = 0;                              // 26
    virtual bool SetDemandCap(SC4Percentage const& demandCap) = 0;                // 27
    virtual uint32_t GetRegionUse() = 0;                                          // 28
    virtual bool SetRegionUse(uint32_t regionUse) = 0;                            // 29
    virtual bool EndOfCycle() = 0;                                                // 30  (SDK: float)
    virtual void DebugLockValue(float value) = 0;                                 // 31
    virtual void DebugUnlockValue() = 0;                                          // 32
    virtual bool DebugIsValueLocked() = 0;                                        // 33
};

// cISC4DemandSimulator (IID 0x47967E16), 12 slots (demand spec §5, "Compiled
// slot tables"). Implemented by cFC4DemandSimulator. SDK errors fixed: slot 7
// takes the building type (uint32) first (§5 [compat]); slot 8 returns in ST0
// (docs/abi; SDK: void).
class cFC4ISC4DemandSimulator : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                                      // 3
    virtual bool Shutdown() = 0;                                                  // 4
    virtual uint32_t GetSimulatorType() = 0;                                      // 5
    virtual cISC4Demand* GetDemand(uint32_t demandID, uint32_t demandIndex) = 0;  // 6
    virtual void UpdateOccupantEffects(uint32_t buildingType, SC4Percentage const& oldPct,
                                       SC4Percentage const& newPct) = 0;          // 7
    virtual float CalculateJobsPerUnitOfDemand(float* jobs, uint32_t demandID, uint32_t demandIndex) = 0;  // 8
    virtual uint32_t GetJobsBySensus(uint32_t type) = 0;                          // 9
    virtual float GetNeutralTaxRate() = 0;                                        // 10
    virtual void GetLocalPopulationSummary(std::map<uint32_t, int32_t>& summary) const = 0;  // 11
};

// cISC4ResidentialSimulator (60 slots), from the SDK declaration with the
// argument sizes and return channels of docs/abi/vtables.tsv
// (cSC4ResidentialSimulator::vftable). The demand census calls GetCellEQ and
// GetCellWorkforcePercent. SDK errors fixed: slots 10-13, 30 and 31 return an
// SC4Percentage BY VALUE (hidden result pointer: the callee pops 4 more bytes
// and returns the pointer in EAX); slot 34 returns 32 bits in EAX [choice:
// uint32, not called]; slots 56 / 57 take one more 32-bit argument [choice:
// uint32, not called].
class cFC4ISC4ResidentialSimulator : public cIGZUnknown {
public:
    using WealthType = cISC4BuildingOccupant::WealthType;

    virtual bool Init() = 0;                                                      // 3
    virtual bool Shutdown() = 0;                                                  // 4
    virtual cISC4SimGrid<uint8_t>* GetProximityMap(WealthType wealth) = 0;        // 5
    virtual bool SchoolIsOnStrike() = 0;                                          // 6
    virtual bool HealthIsOnStrike() = 0;                                          // 7
    virtual bool EndSchoolStrike() = 0;                                           // 8
    virtual bool EndHealthStrike() = 0;                                           // 9
    virtual SC4Percentage ChanceOfSchoolStrike() = 0;                             // 10  (SDK: float)
    virtual SC4Percentage ChanceOfHealthStrike() = 0;                             // 11  (SDK: float)
    virtual SC4Percentage GetSchoolSystemRating() = 0;                            // 12  (SDK: float)
    virtual SC4Percentage GetHealthSystemRating() = 0;                            // 13  (SDK: float)
    virtual bool GetSchoolSystemTotals(intptr_t queryData, SC4Vector<uint32_t>* groups) = 0;    // 14
    virtual bool GetHospitalSystemTotals(intptr_t queryData, SC4Vector<uint32_t>* groups) = 0;  // 15
    virtual int32_t GetPopulation() = 0;                                          // 16
    virtual int32_t GetTotalCityEducationUpkeepCost() = 0;                        // 17
    virtual int32_t GetTotalCityHealthUpkeepCost() = 0;                           // 18
    virtual bool SetOccupantFundingPercentages(cISC4Occupant* occupant, SC4Percentage const& school,
                                               SC4Percentage const& health, bool flag) = 0;  // 19
    virtual bool GetOccupantFundingPercentages(cISC4Occupant* occupant, SC4Percentage& school,
                                               SC4Percentage& health, bool flag) = 0;  // 20
    virtual bool GetAverageEQGrid(cISC4SimGrid<float>*& grid, float* min, float* max) = 0;  // 21
    virtual bool GetAverageHQGrid(cISC4SimGrid<float>*& grid, float* min, float* max) = 0;  // 22
    virtual bool GetEQGrids(cISC4SimGrid<float>*& grid, cISC4SimGrid<float>* a, cISC4SimGrid<float>* b) = 0;  // 23
    virtual bool GetHQGrids(cISC4SimGrid<float>*& grid, cISC4SimGrid<float>* a, cISC4SimGrid<float>* b) = 0;  // 24
    virtual bool GetPopulationGrids(cISC4SimGrid<uint16_t>*& grid, cISC4SimGrid<uint16_t>* a,
                                    cISC4SimGrid<uint16_t>* b) = 0;               // 25
    virtual bool GetSchoolQueryData(cISC4Occupant* occupant, intptr_t queryData) = 0;    // 26
    virtual bool GetHospitalQueryData(cISC4Occupant* occupant, intptr_t queryData) = 0;  // 27
    virtual bool EstimateCurrentOccupantCapacity(cISC4Occupant* occupant, uint32_t& a, uint32_t& b) = 0;  // 28
    virtual int32_t GetCellLifeExpectancy(uint32_t cellX, uint32_t cellZ) = 0;    // 29
    virtual SC4Percentage GetCellWorkforcePercent(uint32_t cellX, uint32_t cellZ) = 0;  // 30  (SDK: float)
    virtual SC4Percentage GetGlobalWorkforcePercent() = 0;                        // 31  (SDK: float)
    virtual float GetGlobalEQ() = 0;                                              // 32
    virtual float GetGlobalHQ() = 0;                                              // 33
    virtual uint32_t GetGlobalLE() = 0;                                           // 34  (SDK: float)
    virtual float GetCellEQ(uint32_t cellX, uint32_t cellZ) = 0;                  // 35
    virtual float GetCellHQ(uint32_t cellX, uint32_t cellZ) = 0;                  // 36
    virtual float GetCellEQByWealth(uint32_t cellX, uint32_t cellZ, WealthType wealth) = 0;  // 37
    virtual float GetCellHQByWealth(uint32_t cellX, uint32_t cellZ, WealthType wealth) = 0;  // 38
    virtual int32_t GetSchoolAverageGradeMap() = 0;                               // 39
    virtual int32_t GetHospitalAverageGradeMap() = 0;                             // 40
    virtual int32_t GetAverageAgeMap() = 0;                                       // 41
    virtual int32_t GetAverageNewAgeByWealth(WealthType wealth) = 0;              // 42
    virtual bool GetEQMinAndMaxCellCoords(uint32_t& minX, uint32_t& minZ, uint32_t& maxX, uint32_t& maxZ, float& min,
                                          float& max) = 0;                        // 43
    virtual bool GetHQMinAndMaxCellCoords(uint32_t& minX, uint32_t& minZ, uint32_t& maxX, uint32_t& maxZ, float& min,
                                          float& max) = 0;                        // 44
    virtual bool GetOccupantCoverage(cISC4Occupant* occupant, SC4Percentage const& effectiveness, float& rangeX,
                                     float& rangeZ) = 0;                          // 45
    virtual int32_t GetSchoolBuildingCount() = 0;                                 // 46
    virtual bool GetSchoolBuildings(SC4List<cISC4Occupant*>& buildings, SC4Vector<uint32_t>* groups) = 0;  // 47
    virtual int32_t GetHospitalBuildingCount() = 0;                               // 48
    virtual bool GetHospitalBuildings(SC4List<cISC4Occupant*>& buildings, SC4Vector<uint32_t>* groups) = 0;  // 49
    virtual int32_t GetMaxEQ() = 0;                                               // 50
    virtual int32_t GetMaxHQ() = 0;                                               // 51
    virtual bool GetGlobalAutoBudgetForSchools() = 0;                             // 52
    virtual bool SetGlobalAutoBudgetForSchools(bool enable) = 0;                  // 53
    virtual bool GetGlobalAutoBudgetForHospitals() = 0;                           // 54
    virtual bool SetGlobalAutoBudgetForHospitals(bool enable) = 0;                // 55
    virtual bool GetAutoBudget(uint32_t which) = 0;                               // 56  (SDK: no argument)
    virtual bool SetAutoBudget(uint32_t which, bool enable) = 0;                  // 57  (SDK: one argument)
    virtual bool EstimateIdealFunding(cISC4Occupant* occupant, SC4Percentage& funding) = 0;  // 58
    virtual void ToggleTractTracking(int32_t a, int32_t b) = 0;                   // 59
};

// cISC4RegionalCity (73 slots), from the SDK declaration with the argument
// sizes and return channels of docs/abi/interfaces.tsv. The demand simulator
// calls the neighbour methods of §9 (slots 9, 13 and 16-19). Overload groups
// as compiled (MSVC puts each group at its first declaration, reversed):
// GetPopulation(type) is slot 9 and GetPopulation() slot 10, the two
// UpdateCityCache overloads slots 24 (city) / 25 (preferences). SDK errors
// fixed: slot 13 returns 32 bits in EAX (SDK: float) [choice: the f32 bits,
// see the demand host]; slot 48 is the elevation SETTER (the SDK names it
// GetWorldBaseElevation); the setters 51, 53, 55 ... 63 take a double and
// return in ST0 (SDK: float argument, void).
class cFC4ISC4RegionalCity : public cIGZUnknown {
public:
    virtual bool Init() = 0;                                                      // 3
    virtual bool Shutdown() = 0;                                                  // 4
    virtual void GetPosition(int32_t& x, int32_t& z) const = 0;                   // 5
    virtual void SetPosition(int32_t x, int32_t z, bool rearrange) = 0;           // 6
    virtual void GetCitySize(int32_t& x, int32_t& z) const = 0;                   // 7
    virtual void SetCitySize(int32_t x, int32_t z) = 0;                           // 8
    virtual int32_t GetPopulationOfType(uint32_t populationType) const = 0;       // 9   GetPopulation(type)
    virtual int32_t GetPopulation() const = 0;                                    // 10  GetPopulation()
    virtual int32_t GetCommercialJobs() const = 0;                                // 11
    virtual int32_t GetIndustrialJobs() const = 0;                                // 12
    virtual uint32_t GetWorkforcePercentage() const = 0;                          // 13  (SDK: float)
    virtual int8_t GetMayorRating() const = 0;                                    // 14
    virtual cISC4City::DifficultyLevel GetDifficultyLevel() const = 0;            // 15
    virtual float GetTaxRate(uint32_t taxType) const = 0;                         // 16
    virtual int32_t GetExtrapolatedPopulation(uint32_t populationType) const = 0;  // 17
    virtual int32_t GetAllowableExtrapolation(uint32_t populationType) const = 0;  // 18
    virtual void ExtrapolateGrowth(uint32_t populationType, float addedPopulation) = 0;  // 19
    virtual cISC4RegionalCity* FindConnection(int32_t a, int32_t b, int32_t c) const = 0;  // 20
    virtual void GetAllConnections(SC4List<cISC4NeighborConnection*>& list) const = 0;  // 21
    virtual void ChangeSymmetricConnection(cISC4NeighborConnection* connection, bool flag) = 0;  // 22
    virtual void SetupPreferences(SC4NewCityPreferences* preferences) = 0;        // 23
    virtual void UpdateCityCacheFromCity(cISC4City* city) = 0;                    // 24  UpdateCityCache(city)
    virtual void UpdateCityCacheFromPreferences(SC4NewCityPreferences* preferences) = 0;  // 25  UpdateCityCache(prefs)
    virtual void SetupCity(cISC4City* city) = 0;                                  // 26
    virtual uint32_t GetCitySerialNumber() const = 0;                             // 27
    virtual void SetCitySerialNumber(uint32_t serialNumber) = 0;                  // 28
    virtual bool GetOriginalLanguageAndCountry(int32_t& language, int32_t& country) const = 0;  // 29
    virtual bool GetLastLanguageAndCountry(int32_t& language, int32_t& country) = 0;  // 30
    virtual void GetCitySaveFilePath(cIGZString& path) const = 0;                 // 31
    virtual void SetCitySaveFilePath(cIGZString const& path) = 0;                 // 32
    virtual void GetCityName(cIGZString& name) const = 0;                         // 33
    virtual void SetCityName(cIGZString const& name) = 0;                         // 34
    virtual void GetMayorName(cIGZString& name) const = 0;                        // 35
    virtual void SetMayorName(cIGZString const& name) = 0;                        // 36
    virtual void GetUtilityAdvisorName(cIGZString& name) const = 0;               // 37
    virtual void SetUtilityAdvisorName(cIGZString const& name) = 0;               // 38
    virtual void GetCityDescription(cIGZString& description) const = 0;           // 39
    virtual void SetCityDescription(cIGZString const& description) = 0;          // 40
    virtual uint32_t GetBirthDate() const = 0;                                    // 41
    virtual void SetBirthDate(uint32_t birthDate) = 0;                            // 42
    virtual bool GetEstablished() const = 0;                                      // 43
    virtual void SetEstablished(bool established) = 0;                           // 44
    virtual void GetWorldPosition(float& longitude, float& latitude) const = 0;   // 45
    virtual void SetWorldPosition(float longitude, float latitude) = 0;           // 46
    virtual float GetWorldBaseElevation() const = 0;                              // 47
    virtual void SetWorldBaseElevation(float elevation) = 0;                      // 48  (SDK: GetWorldBaseElevation)
    virtual cISC4City::WorldHemisphere GetWorldHemisphere() const = 0;            // 49
    virtual float GetBudget() const = 0;                                          // 50
    virtual float SetBudget(double budget) = 0;                                   // 51
    virtual float GetIncome() const = 0;                                          // 52
    virtual float SetIncome(double income) = 0;                                   // 53
    virtual float GetExported(int32_t commodity) const = 0;                       // 54
    virtual float SetExported(int32_t commodity, double value) = 0;               // 55
    virtual float GetImported(int32_t commodity) const = 0;                       // 56
    virtual float SetImported(int32_t commodity, double value) = 0;               // 57
    virtual float GetProduced(int32_t commodity) const = 0;                       // 58
    virtual float SetProduced(int32_t commodity, double value) = 0;              // 59
    virtual float GetDemanded(int32_t commodity) const = 0;                       // 60
    virtual float SetDemanded(int32_t commodity, double value) = 0;               // 61
    virtual float GetCostPerUnit(int32_t commodity) const = 0;                    // 62
    virtual float SetCostPerUnit(int32_t commodity, double value) = 0;            // 63
    virtual float GetCommodityBalance(int32_t commodity) const = 0;               // 64
    virtual uint32_t GetTutorialGUID() const = 0;                                 // 65
    virtual void SetTutorialGUID(uint32_t guid) = 0;                              // 66
    virtual bool IsTutorial() const = 0;                                          // 67
    virtual bool UpdateLocalDeals() = 0;                                          // 68
    virtual bool SetLocalDeals(SC4List<cISC4NeighborDeal*>& list) = 0;            // 69
    virtual bool GetLocalDeals(SC4List<cISC4NeighborDeal*>& list) = 0;            // 70
    virtual bool UpdateImportExport() = 0;                                        // 71
    virtual void GetPointsOfInterest(uint32_t type, SC4Vector<uint32_t>& list) = 0;  // 72
};

class cISC4Simulator;
class cISC4BudgetSimulator;
class cISC4Ordinance;
class cISC4ResidentialSimulator;
class cISC4RegionalCity;

namespace fc4 {
// A simulator handed out by the game (cISC4City::GetSimulator), seen through
// the verified slot order instead of the SDK's declaration.
inline cFC4ISC4Simulator* AsGame(cISC4Simulator* p) { return reinterpret_cast<cFC4ISC4Simulator*>(p); }
// The same for cISC4City::GetBudgetSimulator (the SDK's declaration has the
// 32-bit monthly totals; the verified one returns 64-bit values).
inline cFC4ISC4BudgetSimulator* AsGame(cISC4BudgetSimulator* p) {
    return reinterpret_cast<cFC4ISC4BudgetSimulator*>(p);
}

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
inline cFC4IGZVariant* AsGame(cIGZVariant* p) { return reinterpret_cast<cFC4IGZVariant*>(p); }
inline cFC4IGZVariant const* AsGame(cIGZVariant const* p) { return reinterpret_cast<cFC4IGZVariant const*>(p); }
inline cIGZVariant const* AsSdk(cFC4IGZVariant const* p) { return reinterpret_cast<cIGZVariant const*>(p); }
inline cFC4ISC4LotManager* AsGame(cISC4LotManager* p) { return reinterpret_cast<cFC4ISC4LotManager*>(p); }
inline cFC4ISC4Lot* AsGame(cISC4Lot* p) { return reinterpret_cast<cFC4ISC4Lot*>(p); }
inline cFC4ISC4NeighborsSimulator* AsGame(cISC4NeighborsSimulator* p) {
    return reinterpret_cast<cFC4ISC4NeighborsSimulator*>(p);
}
// Ordinances travel as the SDK's cISC4Ordinance* (single inheritance, vtable
// at offset 0); slot 10 is only ever called through cFC4ISC4Ordinance.
inline cFC4ISC4Ordinance* AsGame(cISC4Ordinance* p) { return reinterpret_cast<cFC4ISC4Ordinance*>(p); }
inline cISC4Ordinance* AsSdk(cFC4ISC4Ordinance* p) { return reinterpret_cast<cISC4Ordinance*>(p); }
// Demand nodes travel as the SDK's cISC4Demand* (single inheritance); slots
// 16, 17 and 30 are only ever called through cFC4ISC4Demand.
inline cFC4ISC4Demand* AsGame(cISC4Demand* p) { return reinterpret_cast<cFC4ISC4Demand*>(p); }
inline cISC4Demand* AsSdk(cFC4ISC4Demand* p) { return reinterpret_cast<cISC4Demand*>(p); }
inline cFC4ISC4DemandSimulator* AsGame(cISC4DemandSimulator* p) {
    return reinterpret_cast<cFC4ISC4DemandSimulator*>(p);
}
inline cISC4DemandSimulator* AsSdk(cFC4ISC4DemandSimulator* p) { return reinterpret_cast<cISC4DemandSimulator*>(p); }
inline cFC4ISC4ResidentialSimulator* AsGame(cISC4ResidentialSimulator* p) {
    return reinterpret_cast<cFC4ISC4ResidentialSimulator*>(p);
}
inline cFC4ISC4RegionalCity* AsGame(cISC4RegionalCity* p) { return reinterpret_cast<cFC4ISC4RegionalCity*>(p); }
}  // namespace fc4
