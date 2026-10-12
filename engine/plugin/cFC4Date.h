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

// cIGZDate implementation backed by the core's Julian-day utilities (fc4.h).
// The SDK documents only the method list of cIGZDate, so the semantics of
// several methods are our own choices (marked [choice]).
#pragma once

#include "cIGZDate.h"

#include <cstdint>

class cFC4Date final : public cIGZDate {
public:
    // `heapOwned`: deleted when its reference count drops to zero (clones).
    // The simulator's embedded date object is not heap-owned.
    explicit cFC4Date(int32_t julian = 2451545, bool heapOwned = false);
    virtual ~cFC4Date() = default;

    int32_t Julian() const { return julian_; }
    void SetJulian(int32_t j) { julian_ = j; }

    // cIGZUnknown
    bool QueryInterface(uint32_t riid, void** ppvObj) override;
    uint32_t AddRef() override;
    uint32_t Release() override;

    // cIGZDate
    bool Set(uint32_t dwDay, uint32_t dwYear) override;
    bool Set(uint32_t dwMonth, uint32_t dwDay, uint32_t dwYear) override;
    bool Set(cRZTime const& sTime) override;
    bool Set(uint32_t dwDay) override;
    bool Set(char const* pszMonthName, uint32_t dwDay, uint32_t dwYear) override;
    bool Copy(cIGZDate const& sCopyFrom) override;
    bool Clone(cIGZDate** ppCopyTo) override;
    bool Between(cIGZDate const& sStart, cIGZDate const& sEnd) override;
    int32_t CompareTo(cIGZDate const& sOther) override;
    uint32_t WeekDay() override;
    uint32_t Year() override;
    uint32_t DayOfYear() override;
    uint32_t DayOfMonth() override;
    uint32_t FirstDayOfMonth() override;
    uint32_t FirstDayOfMonth(uint32_t dwMonth) override;
    uint32_t Hash() override;
    bool IsValid() override;
    bool Leap() override;
    bool MaxDate(cIGZDate const& sOther, cIGZDate& sMaxOut) override;
    bool MinDate(cIGZDate const& sOther, cIGZDate& sMinOut) override;
    uint32_t Month() override;
    uint32_t Previous(uint32_t dwDayOfPriorWeek, cIGZDate& sOut) override;
    uint32_t YearLastTwoDigits() override;
    uint32_t DayNumber() override;
    bool operator<(cIGZDate const& sOther) override;
    bool operator<=(cIGZDate const& sOther) override;
    bool operator>(cIGZDate const& sOther) override;
    bool operator>=(cIGZDate const& sOther) override;
    bool operator==(cIGZDate const& sOther) override;
    bool operator!=(cIGZDate const& sOther) override;
    cIGZDate& operator-(cIGZDate const& sOther) override;
    cIGZDate& operator+(cIGZDate const& sOther) override;
    cIGZDate& operator-(int32_t nDays) override;
    cIGZDate& operator+(int32_t nDays) override;
    cIGZDate& operator++() override;
    cIGZDate& operator--() override;
    cIGZDate& operator+=(int32_t nDays) override;
    cIGZDate& operator-=(int32_t nDays) override;
    bool DateString(cIGZString& sDateOut, uint32_t dwFormatID) override;
    bool DayWithinMonth(uint32_t dwMonth, uint32_t dwDay, uint32_t dwYear) override;
    uint32_t DaysInYear(uint32_t dwYear) override;
    uint32_t Jday(uint32_t dwMonth, uint32_t dwDay, uint32_t dwYear) override;
    bool LeapYear(uint32_t dwYear) override;

private:
    static int32_t Other(cIGZDate const& d);

    int32_t julian_;
    uint32_t refCount_ = 0;
    bool heapOwned_;
};
