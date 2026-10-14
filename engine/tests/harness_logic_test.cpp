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

void JulianTests() {
    using namespace fc4::harness;
    CHECK(JulianFromCivil(2000, 1, 1) == 2451545);
    CHECK(JulianFromCivil(2000, 2, 1) == 2451576);
    CHECK(JulianFromCivil(2000, 2, 29) == 2451604);
    CHECK(JulianFromCivil(2000, 3, 1) == 2451605);
    CHECK(JulianFromCivil(1970, 1, 1) == 2440588);
    CHECK(JulianFromCivil(2001, 1, 1) == 2451911);
    const Date d = CivilFromJulian(2451600);
    CHECK(d.year == 2000 && d.month == 2 && d.day == 25);
    // Round trip over 1899..2101 (leap rules of 100 and 400 included), and
    // consecutive julians are consecutive dates.
    int bad = 0;
    Date prev = CivilFromJulian(JulianFromCivil(1899, 1, 1) - 1);
    for (int32_t j = JulianFromCivil(1899, 1, 1); j <= JulianFromCivil(2101, 12, 31); ++j) {
        const Date c = CivilFromJulian(j);
        if (JulianFromCivil(c.year, c.month, c.day) != j) ++bad;
        if (!ParseIsoDate(FormatDate(c), nullptr)) ++bad;
        if (c.Key() <= prev.Key()) ++bad;
        prev = c;
    }
    CHECK(bad == 0);
}

void EndDateTests() {
    using namespace fc4::harness;
    CHECK(!EndDateDue(0, 2451605, true));               // no end date
    CHECK(!EndDateDue(2451605, 2451604, true));         // the day before
    CHECK(EndDateDue(2451605, 2451605, true));          // the day itself
    CHECK(EndDateDue(2451605, 2451700, true));          // already past (a city loaded after it)
    CHECK(!EndDateDue(2451605, 2451700, false));        // the autotest clock has not started

    RunFigures f;
    FormatRunFigures(2451605, true, 2480, 31, &f);
    CHECK(std::string(f.date) == "2000-03-01");
    CHECK(std::string(f.realSeconds) == "2.480");
    CHECK(std::string(f.daysPerSecond) == "12.50");
    FormatRunFigures(2451545, true, 12, 0, &f);
    CHECK(std::string(f.realSeconds) == "0.012");
    CHECK(std::string(f.daysPerSecond) == "0.00");
    FormatRunFigures(2451545, true, 0, 3, &f);  // no time yet: 0, not a division by zero
    CHECK(std::string(f.daysPerSecond) == "0.00");
    FormatRunFigures(0, false, 999, 5, &f);
    CHECK(std::string(f.date) == "-");
    CHECK(std::string(f.realSeconds) == "-");
    CHECK(std::string(f.daysPerSecond) == "-");
}

}  // namespace

int main() {
    DateTests();
    RateTests();
    JulianTests();
    EndDateTests();
    if (g_failures == 0) std::printf("harness logic test: all checks passed\n");
    return g_failures == 0 ? 0 : 1;
}
