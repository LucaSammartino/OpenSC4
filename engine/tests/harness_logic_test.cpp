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

// Unit tests of plugin/FC4HarnessLogic.h (tooling: sim_turbo,
// autotest_end_date, script): dates and rates.

#include "FC4HarnessLogic.h"

#include <cstdio>
#include <string>

namespace {

int g_failures = 0;

#define CHECK(cond)                                                          \
    do {                                                                     \
        if (!(cond)) {                                                       \
            std::printf("FAILED: %s (line %d)\n", #cond, __LINE__);          \
            ++g_failures;                                                    \
        }                                                                    \
    } while (0)

void DateTests() {
    using namespace fc4::harness;
    Date d;
    CHECK(ParseIsoDate("2000-01-01", &d) && d.year == 2000 && d.month == 1 && d.day == 1);
    CHECK(d.Key() == 20000101);
    CHECK(ParseIsoDate("2000-02-29", &d) && d.day == 29);   // leap (divisible by 400)
    CHECK(!ParseIsoDate("1900-02-29", &d));                  // not leap (divisible by 100)
    CHECK(ParseIsoDate("2004-02-29", nullptr));
    CHECK(!ParseIsoDate("2001-02-29", nullptr));
    CHECK(ParseIsoDate("2001-12-31", nullptr));
    CHECK(!ParseIsoDate("2001-13-01", nullptr));
    CHECK(!ParseIsoDate("2001-00-10", nullptr));
    CHECK(!ParseIsoDate("2001-04-31", nullptr));
    CHECK(!ParseIsoDate("2001-04-00", nullptr));
    CHECK(!ParseIsoDate("0000-01-01", nullptr));
    CHECK(!ParseIsoDate("2001-1-01", nullptr));
    CHECK(!ParseIsoDate("2001/01/01", nullptr));
    CHECK(!ParseIsoDate("2001-01-01 ", nullptr));
    CHECK(!ParseIsoDate("+001-01-01", nullptr));
    CHECK(!ParseIsoDate("", nullptr));
    CHECK(FormatDate(Date{2000, 3, 1}) == "2000-03-01");
    CHECK((Date{2000, 12, 31}).Key() < (Date{2001, 1, 1}).Key());
    CHECK((Date{2000, 2, 1}).Key() > (Date{2000, 1, 31}).Key());
}

void RateTests() {
    using namespace fc4::harness;
    CHECK(RatePer100(31, 2480) == 1250);  // 12.50 days/s
    CHECK(FormatPer100(1250) == "12.50");
    CHECK(FormatPer100(RatePer100(10, 2480)) == "4.03");
    CHECK(RatePer100(5, 0) == 0);
    CHECK(FormatPer100(7) == "0.07");
    CHECK(FormatPer100(0) == "0.00");
}

}  // namespace

int main() {
    DateTests();
    RateTests();
    if (g_failures == 0) std::printf("harness logic test: all checks passed\n");
    return g_failures == 0 ? 0 : 1;
}
