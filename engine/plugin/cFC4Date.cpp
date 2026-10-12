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

#include "cFC4Date.h"

#include "cIGZString.h"
#include "FC4Log.h"
#include "fc4.h"

#include <atomic>
#include <cctype>
#include <cstdio>
#include <cstring>

namespace {

const char* const kMonthNames[12] = {"January", "February", "March",     "April",   "May",      "June",
                                     "July",    "August",   "September", "October", "November", "December"};

fc4_date Fields(int32_t julian) {
    fc4_date d{};
    fc4_date_from_julian(julian, &d);
    return d;
}

bool IsLeap(int32_t y) { return (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0); }

int32_t DaysInMonth(int32_t y, int32_t m) {
    static const int32_t kDays[12] = {31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31};
    if (m < 1 || m > 12) return 0;
    return (m == 2 && IsLeap(y)) ? 29 : kDays[m - 1];
}


// ---- diagnostics: trace calls on our date objects (FC4_LOG_LEVEL=2) ----------------------
// The first 200 calls (all date objects together) are logged with their
// result, to show whether and how the game reads the date from us.
std::atomic<long> g_dateTraces{0};
constexpr long kMaxDateTraces = 200;

long long AsNum(bool v) { return v ? 1 : 0; }
long long AsNum(uint32_t v) { return static_cast<long long>(v); }
long long AsNum(int32_t v) { return static_cast<long long>(v); }

bool TraceSlot() { return fc4::LogLevel() >= fc4::kLogVerbose && g_dateTraces.fetch_add(1) < kMaxDateTraces; }

template <class T>
T Ret(const cFC4Date* self, const char* method, T value) {
    if (TraceSlot()) {
        FC4_LOGV("cFC4Date(%p)::%s -> %lld (julian %d)", static_cast<const void*>(self), method, AsNum(value),
                 self->Julian());
    }
    return value;
}

cIGZDate& RetRef(const cFC4Date* self, const char* method, cIGZDate& value) {
    if (TraceSlot()) {
        FC4_LOGV("cFC4Date(%p)::%s -> date (julian now %d)", static_cast<const void*>(self), method, self->Julian());
    }
    return value;
}

}  // namespace

cFC4Date::cFC4Date(int32_t julian, bool heapOwned) : julian_(julian), heapOwned_(heapOwned) {}

int32_t cFC4Date::Other(cIGZDate const& d) {
    // cIGZDate's accessors are not const; DayNumber() does not modify the object.
    return static_cast<int32_t>(const_cast<cIGZDate&>(d).DayNumber());
}

bool cFC4Date::QueryInterface(uint32_t riid, void** ppvObj) {
    // [choice] The SDK does not publish an IID for cIGZDate, so only
    // cIGZUnknown is answered.
    if (riid == GZIID_cIGZUnknown) {
        *ppvObj = static_cast<cIGZUnknown*>(this);
        AddRef();
        return true;
    }
    *ppvObj = nullptr;
    return false;
}

uint32_t cFC4Date::AddRef() { return ++refCount_; }

uint32_t cFC4Date::Release() {
    if (refCount_ > 0) --refCount_;
    if (refCount_ == 0 && heapOwned_) {
        delete this;
        return 0;
    }
    return refCount_;
}

bool cFC4Date::Set(uint32_t dwDay, uint32_t dwYear) {
    // Day of year + year.
    const int32_t y = static_cast<int32_t>(dwYear);
    if (dwDay < 1 || dwDay > static_cast<uint32_t>(IsLeap(y) ? 366 : 365)) return Ret(this, "Set(uint32_t dwDay, uint32_t dwYear)", false);
    julian_ = fc4_julian_from_civil(y, 1, 1) + static_cast<int32_t>(dwDay) - 1;
    return Ret(this, "Set(uint32_t dwDay, uint32_t dwYear)", true);
}

bool cFC4Date::Set(uint32_t dwMonth, uint32_t dwDay, uint32_t dwYear) {
    if (!DayWithinMonth(dwMonth, dwDay, dwYear)) return Ret(this, "Set(uint32_t dwMonth, uint32_t dwDay, uint32_t dwYear)", false);
    julian_ = fc4_julian_from_civil(static_cast<int32_t>(dwYear), static_cast<int32_t>(dwMonth), static_cast<int32_t>(dwDay));
    return Ret(this, "Set(uint32_t dwMonth, uint32_t dwDay, uint32_t dwYear)", true);
}

bool cFC4Date::Set(cRZTime const&) {
    // [choice] cRZTime's layout is not part of the SDK.
    return Ret(this, "Set(cRZTime const&)", false);
}

bool cFC4Date::Set(uint32_t dwDay) {
    julian_ = static_cast<int32_t>(dwDay);
    return Ret(this, "Set(uint32_t dwDay)", true);
}

bool cFC4Date::Set(char const* pszMonthName, uint32_t dwDay, uint32_t dwYear) {
    // [choice] English month names, matched case-insensitively on the first 3 letters.
    if (pszMonthName == nullptr || strlen(pszMonthName) < 3) return Ret(this, "Set(char const* pszMonthName, uint32_t dwDay, uint32_t dwYear)", false);
    for (uint32_t m = 0; m < 12; ++m) {
        bool match = true;
        for (int i = 0; i < 3; ++i) {
            if (std::tolower(static_cast<unsigned char>(pszMonthName[i])) !=
                std::tolower(static_cast<unsigned char>(kMonthNames[m][i]))) {
                match = false;
                break;
            }
        }
        if (match) return Ret(this, "Set(char const* pszMonthName, uint32_t dwDay, uint32_t dwYear)", Set(m + 1, dwDay, dwYear));
    }
    return Ret(this, "Set(char const* pszMonthName, uint32_t dwDay, uint32_t dwYear)", false);
}

bool cFC4Date::Copy(cIGZDate const& sCopyFrom) {
    julian_ = Other(sCopyFrom);
    return Ret(this, "Copy(cIGZDate const& sCopyFrom)", true);
}

bool cFC4Date::Clone(cIGZDate** ppCopyTo) {
    if (ppCopyTo == nullptr) return Ret(this, "Clone(cIGZDate** ppCopyTo)", false);
    cFC4Date* copy = new cFC4Date(julian_, true);
    copy->AddRef();
    *ppCopyTo = copy;
    return Ret(this, "Clone(cIGZDate** ppCopyTo)", true);
}

bool cFC4Date::Between(cIGZDate const& sStart, cIGZDate const& sEnd) {
    return Ret(this, "Between(cIGZDate const& sStart, cIGZDate const& sEnd)", julian_ >= Other(sStart) && julian_ <= Other(sEnd));
}

int32_t cFC4Date::CompareTo(cIGZDate const& sOther) {
    const int32_t o = Other(sOther);
    return Ret(this, "CompareTo(cIGZDate const& sOther)", julian_ < o ? -1 : (julian_ > o ? 1 : 0));
}

// [choice] 1 = Monday .. 7 = Sunday, the convention of the simulator spec.
uint32_t cFC4Date::WeekDay() { return Ret(this, "WeekDay()", static_cast<uint32_t>(Fields(julian_).weekday)); }
uint32_t cFC4Date::Year() { return Ret(this, "Year()", static_cast<uint32_t>(Fields(julian_).year)); }
uint32_t cFC4Date::DayOfYear() { return Ret(this, "DayOfYear()", static_cast<uint32_t>(Fields(julian_).day_of_year)); }
uint32_t cFC4Date::DayOfMonth() { return Ret(this, "DayOfMonth()", static_cast<uint32_t>(Fields(julian_).day_of_month)); }
uint32_t cFC4Date::Month() { return Ret(this, "Month()", static_cast<uint32_t>(Fields(julian_).month)); }

// [choice] Day number of the first day of this (or the given) month of this year.
uint32_t cFC4Date::FirstDayOfMonth() { return Ret(this, "FirstDayOfMonth()", FirstDayOfMonth(Month())); }
uint32_t cFC4Date::FirstDayOfMonth(uint32_t dwMonth) {
    if (dwMonth < 1 || dwMonth > 12) return Ret(this, "FirstDayOfMonth(uint32_t dwMonth)", 0);
    return Ret(this, "FirstDayOfMonth(uint32_t dwMonth)", static_cast<uint32_t>(fc4_julian_from_civil(Fields(julian_).year, static_cast<int32_t>(dwMonth), 1)));
}

uint32_t cFC4Date::Hash() { return Ret(this, "Hash()", static_cast<uint32_t>(julian_)); }
uint32_t cFC4Date::DayNumber() { return Ret(this, "DayNumber()", static_cast<uint32_t>(julian_)); }
bool cFC4Date::IsValid() { return Ret(this, "IsValid()", julian_ > 0); }
bool cFC4Date::Leap() { return Ret(this, "Leap()", IsLeap(Fields(julian_).year)); }

bool cFC4Date::MaxDate(cIGZDate const& sOther, cIGZDate& sMaxOut) {
    const int32_t o = Other(sOther);
    return Ret(this, "MaxDate(cIGZDate const& sOther, cIGZDate& sMaxOut)", sMaxOut.Set(static_cast<uint32_t>(julian_ > o ? julian_ : o)));
}

bool cFC4Date::MinDate(cIGZDate const& sOther, cIGZDate& sMinOut) {
    const int32_t o = Other(sOther);
    return Ret(this, "MinDate(cIGZDate const& sOther, cIGZDate& sMinOut)", sMinOut.Set(static_cast<uint32_t>(julian_ < o ? julian_ : o)));
}

uint32_t cFC4Date::Previous(uint32_t dwDayOfPriorWeek, cIGZDate& sOut) {
    // [choice] The latest date strictly before this one whose WeekDay() is
    // dwDayOfPriorWeek (1..7); returns its day number, or 0 if out of range.
    if (dwDayOfPriorWeek < 1 || dwDayOfPriorWeek > 7) return Ret(this, "Previous(uint32_t dwDayOfPriorWeek, cIGZDate& sOut)", 0);
    int32_t j = julian_ - 1;
    while (Fields(j).weekday != static_cast<int32_t>(dwDayOfPriorWeek)) --j;
    sOut.Set(static_cast<uint32_t>(j));
    return Ret(this, "Previous(uint32_t dwDayOfPriorWeek, cIGZDate& sOut)", static_cast<uint32_t>(j));
}

uint32_t cFC4Date::YearLastTwoDigits() { return Ret(this, "YearLastTwoDigits()", Year() % 100); }

bool cFC4Date::operator<(cIGZDate const& o) { return Ret(this, "operator<(cIGZDate const& o)", julian_ < Other(o)); }
bool cFC4Date::operator<=(cIGZDate const& o) { return Ret(this, "operator<=(cIGZDate const& o)", julian_ <= Other(o)); }
bool cFC4Date::operator>(cIGZDate const& o) { return Ret(this, "operator>(cIGZDate const& o)", julian_ > Other(o)); }
bool cFC4Date::operator>=(cIGZDate const& o) { return Ret(this, "operator>=(cIGZDate const& o)", julian_ >= Other(o)); }
bool cFC4Date::operator==(cIGZDate const& o) { return Ret(this, "operator==(cIGZDate const& o)", julian_ == Other(o)); }
bool cFC4Date::operator!=(cIGZDate const& o) { return Ret(this, "operator!=(cIGZDate const& o)", julian_ != Other(o)); }

// [choice] The interface returns references, so arithmetic is applied in place.
cIGZDate& cFC4Date::operator-(cIGZDate const& o) { julian_ -= Other(o); return RetRef(this, "operator-(cIGZDate const& o)", *this); }
cIGZDate& cFC4Date::operator+(cIGZDate const& o) { julian_ += Other(o); return RetRef(this, "operator+(cIGZDate const& o)", *this); }
cIGZDate& cFC4Date::operator-(int32_t n) { julian_ -= n; return RetRef(this, "operator-(int32_t n)", *this); }
cIGZDate& cFC4Date::operator+(int32_t n) { julian_ += n; return RetRef(this, "operator+(int32_t n)", *this); }
cIGZDate& cFC4Date::operator++() { ++julian_; return RetRef(this, "operator++()", *this); }
cIGZDate& cFC4Date::operator--() { --julian_; return RetRef(this, "operator--()", *this); }
cIGZDate& cFC4Date::operator+=(int32_t n) { julian_ += n; return RetRef(this, "operator+=(int32_t n)", *this); }
cIGZDate& cFC4Date::operator-=(int32_t n) { julian_ -= n; return RetRef(this, "operator-=(int32_t n)", *this); }

bool cFC4Date::DateString(cIGZString& sDateOut, uint32_t /*dwFormatID*/) {
    // [choice] The format IDs are not documented; always "Mon D, YYYY".
    const fc4_date d = Fields(julian_);
    char buf[32];
    snprintf(buf, sizeof(buf), "%.3s %d, %d", kMonthNames[(d.month - 1) % 12], d.day_of_month, d.year);
    sDateOut.FromChar(buf);
    return Ret(this, "DateString(cIGZString& sDateOut, uint32_t /*dwFormatID*/)", true);
}

bool cFC4Date::DayWithinMonth(uint32_t dwMonth, uint32_t dwDay, uint32_t dwYear) {
    const int32_t dim = DaysInMonth(static_cast<int32_t>(dwYear), static_cast<int32_t>(dwMonth));
    return Ret(this, "DayWithinMonth(uint32_t dwMonth, uint32_t dwDay, uint32_t dwYear)", dim != 0 && dwDay >= 1 && dwDay <= static_cast<uint32_t>(dim));
}

uint32_t cFC4Date::DaysInYear(uint32_t dwYear) { return Ret(this, "DaysInYear(uint32_t dwYear)", IsLeap(static_cast<int32_t>(dwYear)) ? 366u : 365u); }

uint32_t cFC4Date::Jday(uint32_t dwMonth, uint32_t dwDay, uint32_t dwYear) {
    return Ret(this, "Jday(uint32_t dwMonth, uint32_t dwDay, uint32_t dwYear)", static_cast<uint32_t>(
        fc4_julian_from_civil(static_cast<int32_t>(dwYear), static_cast<int32_t>(dwMonth), static_cast<int32_t>(dwDay))));
}

bool cFC4Date::LeapYear(uint32_t dwYear) { return Ret(this, "LeapYear(uint32_t dwYear)", IsLeap(static_cast<int32_t>(dwYear))); }
