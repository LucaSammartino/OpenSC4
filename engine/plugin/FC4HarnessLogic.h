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

// Pure logic of the test-harness switches (sim_turbo, autotest_end_date,
// script): dates, rates. Header-only and free of Windows and game calls, so
// tests/harness_logic_test.cpp checks it directly. Tooling only; no spec
// behaviour.
#pragma once

#include <cstdint>
#include <cstdio>
#include <string>

namespace fc4 {
namespace harness {

// A civil date (Gregorian, as the core's julian.rs).
struct Date {
    int32_t year = 0;
    int32_t month = 0;
    int32_t day = 0;
    // yyyymmdd: orders like the dates themselves.
    int32_t Key() const { return year * 10000 + month * 100 + day; }
};

inline bool IsLeapYear(int32_t y) { return (y % 4 == 0 && y % 100 != 0) || y % 400 == 0; }

inline int32_t DaysInMonth(int32_t y, int32_t m) {
    static const int32_t kDays[12] = {31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31};
    if (m < 1 || m > 12) return 0;
    return (m == 2 && IsLeapYear(y)) ? 29 : kDays[m - 1];
}

// Exactly "YYYY-MM-DD" (four, two and two digits), a real day, year >= 1.
inline bool ParseIsoDate(const std::string& s, Date* out) {
    if (s.size() != 10 || s[4] != '-' || s[7] != '-') return false;
    int32_t v[3] = {0, 0, 0};
    const size_t start[3] = {0, 5, 8};
    const size_t len[3] = {4, 2, 2};
    for (int f = 0; f < 3; ++f) {
        for (size_t i = 0; i < len[f]; ++i) {
            const char c = s[start[f] + i];
            if (c < '0' || c > '9') return false;
            v[f] = v[f] * 10 + (c - '0');
        }
    }
    if (v[0] < 1 || v[1] < 1 || v[1] > 12 || v[2] < 1 || v[2] > DaysInMonth(v[0], v[1])) return false;
    if (out != nullptr) {
        out->year = v[0];
        out->month = v[1];
        out->day = v[2];
    }
    return true;
}

inline std::string FormatDate(const Date& d) {
    char b[16];
    std::snprintf(b, sizeof(b), "%04d-%02d-%02d", d.year, d.month, d.day);
    return b;
}

// `count` per second over `ms` milliseconds, times 100 (two decimals in
// integer arithmetic; 0 for ms = 0).
inline uint64_t RatePer100(uint64_t count, uint64_t ms) { return ms == 0 ? 0 : count * 100000ull / ms; }

// "12.34" for a RatePer100 value.
inline std::string FormatPer100(uint64_t v) {
    char b[32];
    std::snprintf(b, sizeof(b), "%llu.%02llu", static_cast<unsigned long long>(v / 100),
                  static_cast<unsigned long long>(v % 100));
    return b;
}

}  // namespace harness
}  // namespace fc4
