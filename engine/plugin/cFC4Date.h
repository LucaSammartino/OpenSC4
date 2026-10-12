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

// cIGZDate implementation backed by the core's Julian-day utilities (fc4.h),
// in the game's compiled slot order (FC4Interfaces.h, foundations §17.4).
// The SDK documents only the method list, so the semantics of several methods
// are our own choices (marked [choice]).
#pragma once

#include "FC4Interfaces.h"

#include <cstdint>

class cFC4Date final : public cFC4IGZDate {
public:
    // `heapOwned`: deleted when its reference count drops to zero (clones).
    // The simulator's embedded date object is not heap-owned.
    explicit cFC4Date(int32_t julian = 2451545, bool heapOwned = false);
    virtual ~cFC4Date() = default;  // after slot 45: does not move any interface slot

    int32_t Julian() const { return julian_; }
    void SetJulian(int32_t j) { julian_ = j; }

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cIGZDate (slots 3-45)
    bool SetMonthNameDayYear(char const* monthName, uint32_t day, uint32_t year) override;
    bool SetDayNumber(uint32_t dayNumber) override;
    bool SetTime(cRZTime const& time) override;
    bool SetMonthDayYear(uint32_t month, uint32_t day, uint32_t year) override;
    bool SetDayOfYear(uint32_t day, uint32_t year) override;
    bool Copy(cFC4IGZDate const& from) override;
    bool Clone(cFC4IGZDate** out) override;
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
    bool MaxDate(cFC4IGZDate const& other, cFC4IGZDate& out) override;
    bool MinDate(cFC4IGZDate const& other, cFC4IGZDate& out) override;
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
    bool DateString(cIGZString& out, uint32_t formatID) override;
    bool DayWithinMonth(uint32_t month, uint32_t day, uint32_t year) override;
    uint32_t DaysInYear(uint32_t year) override;
    uint32_t Jday(uint32_t month, uint32_t day, uint32_t year) override;
    bool LeapYear(uint32_t year) override;

private:
    static int32_t Other(cFC4IGZDate const& d);

    int32_t julian_;
    uint32_t refCount_ = 0;
    bool heapOwned_;
};
