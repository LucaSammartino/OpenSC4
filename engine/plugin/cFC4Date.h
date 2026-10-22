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

// cIGZDate implementation backed by core dates (fc4.h),
// in the game's compiled slot order (FC4Interfaces.h, foundations §17.4).
// Published contract: docs/specs/dates.md §§3–4. DT4/DT5 record remaining host inputs.
#pragma once

#include "FC4Interfaces.h"

#include <cstdint>
#include <atomic>

class cFC4Date final : public cFC4IGZDate {
public:
    // `heapOwned`: deleted when its reference count drops to zero (clones).
    // The simulator's embedded date object is not heap-owned.
    explicit cFC4Date(int32_t julian = 2451545, bool heapOwned = false);
    virtual ~cFC4Date() = default;  // after slot 45: does not move any interface slot

    int32_t Julian() const { return static_cast<int32_t>(julian_.load()); }
    void SetJulian(int32_t j) { julian_.store(static_cast<uint32_t>(j)); }

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cIGZDate (slots 3-45)
    uint32_t SetMonthNameDayYear(char const* monthName, uint32_t day, uint32_t year) override;
    uint32_t SetDayNumber(uint32_t dayNumber) override;
    uint32_t SetTime(cRZTime const& time) override;
    uint32_t SetMonthDayYear(uint32_t month, uint32_t day, uint32_t year) override;
    uint32_t SetDayOfYear(uint32_t day, uint32_t year) override;
    uint32_t Copy(cFC4IGZDate const& from) override;
    uint32_t Clone(cFC4IGZDate** out) override;
    bool Between(cFC4IGZDate const& start, cFC4IGZDate const& end) override;
    int32_t CompareTo(cFC4IGZDate const& other) override;
    uint32_t WeekDay() override;
    uint32_t Year() override;
    uint32_t DayOfYear() override;
    uint32_t DayOfMonth() override;
    uint32_t FirstDayOfMonthOf(uint32_t month) override;
    uint32_t FirstDayOfMonth() override;
    uint32_t Hash() override;
    bool IsValid() override;
    bool Leap() override;
    uint32_t MaxDate(cFC4IGZDate const& other, cFC4IGZDate& out) override;
    uint32_t MinDate(cFC4IGZDate const& other, cFC4IGZDate& out) override;
    uint32_t Month() override;
    uint32_t Previous(uint32_t dayOfPriorWeek, cFC4IGZDate& out) override;
    uint32_t YearLastTwoDigits() override;
    uint32_t DayNumber() override;
    bool IsLess(cFC4IGZDate const& other) override;
    bool IsLessOrEqual(cFC4IGZDate const& other) override;
    bool IsGreater(cFC4IGZDate const& other) override;
    bool IsGreaterOrEqual(cFC4IGZDate const& other) override;
    bool IsEqual(cFC4IGZDate const& other) override;
    bool IsNotEqual(cFC4IGZDate const& other) override;
    uint32_t MinusDays(int32_t days) override;
    uint32_t MinusDate(cFC4IGZDate const& other) override;
    uint32_t PlusDays(int32_t days) override;
    uint32_t PlusDate(cFC4IGZDate const& other) override;
    cFC4IGZDate& Increment() override;
    cFC4IGZDate& Decrement() override;
    cFC4IGZDate& AddDays(int32_t days) override;
    cFC4IGZDate& SubtractDays(int32_t days) override;
    uint32_t DateString(cIGZString& out, uint32_t formatID) override;
    bool DayWithinMonth(uint32_t month, uint32_t day, uint32_t year) override;
    uint32_t DaysInYear(uint32_t year) override;
    uint32_t Jday(uint32_t month, uint32_t day, uint32_t year) override;
    bool LeapYear(uint32_t year) override;

private:
    static uint32_t Other(cFC4IGZDate const& d);
    uint32_t Value(uint32_t slot, uint32_t a = 0, uint32_t b = 0, uint32_t c = 0) const;
    uint32_t Store(uint32_t value);

    std::atomic<uint32_t> julian_;
    std::atomic<uint32_t> refCount_{0};
    bool heapOwned_;
};

class cISC4City;
namespace fc4 {
void DatesProbe(cISC4City* city, const char* phase);
void DatesProbeSummary(const char* phase);
}
