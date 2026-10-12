// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Julian Day Number <-> proleptic Gregorian calendar (spec §2.1).
//!
//! The conversions use the well-known "days from civil" construction: shift
//! the year so that it starts on March 1st (the leap day becomes the last day
//! of the year), then count whole 400-year eras of 146 097 days, years of 365
//! days plus leap days, and months with the 153-days-per-5-months pattern.

/// 2000-01-01 as a Julian Day Number (spec §2.1, [compat]).
pub const JULIAN_2000_01_01: i32 = 2_451_545;

/// Julian Day Number of 0000-03-01 in the proleptic Gregorian calendar.
const JDN_0000_03_01: i64 = 1_721_120;

/// A calendar date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CivilDate {
    pub year: i32,
    /// 1..=12
    pub month: i32,
    /// 1..=31
    pub day: i32,
}

/// All calendar fields of one day.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DateFields {
    pub julian: i32,
    pub year: i32,
    pub month: i32,
    pub day_of_month: i32,
    /// 1..=366
    pub day_of_year: i32,
    /// 1 = Monday .. 7 = Sunday
    pub weekday: i32,
}

pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn days_in_year(year: i32) -> i32 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

/// Days in `month` of `year`, or 0 for an invalid month.
pub fn days_in_month(year: i32, month: i32) -> i32 {
    const DAYS: [i32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    match month {
        2 if is_leap_year(year) => 29,
        1..=12 => DAYS[(month - 1) as usize],
        _ => 0,
    }
}

pub fn is_valid_civil_date(year: i32, month: i32, day: i32) -> bool {
    let dim = days_in_month(year, month);
    dim != 0 && day >= 1 && day <= dim
}

/// Proleptic Gregorian date -> Julian Day Number.
pub fn julian_from_civil(year: i32, month: i32, day: i32) -> i32 {
    let y = i64::from(year) - i64::from(month <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400; // [0, 399]
    let mp = i64::from((month + 9) % 12); // March = 0
    let doy = (153 * mp + 2) / 5 + i64::from(day - 1); // [0, 365]
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
    (era * 146_097 + doe + JDN_0000_03_01) as i32
}

/// Julian Day Number -> proleptic Gregorian date.
pub fn civil_from_julian(julian: i32) -> CivilDate {
    let z = i64::from(julian) - JDN_0000_03_01;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let day = (doy - (153 * mp + 2) / 5 + 1) as i32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as i32;
    let year = (yoe + era * 400) as i32 + i32::from(month <= 2);
    CivilDate { year, month, day }
}

/// 1 = Monday .. 7 = Sunday. JDN 0 is a Monday.
pub fn weekday_of_julian(julian: i32) -> i32 {
    julian.rem_euclid(7) + 1
}

/// 1..=366
pub fn day_of_year_of_julian(julian: i32) -> i32 {
    let c = civil_from_julian(julian);
    julian - julian_from_civil(c.year, 1, 1) + 1
}

/// The scheduler's week counter: `(dayOfYear - 1) / 7 + 1` (spec §2.3).
pub fn week_of_year(day_of_year: i32) -> i32 {
    (day_of_year - 1) / 7 + 1
}

pub fn date_fields(julian: i32) -> DateFields {
    let c = civil_from_julian(julian);
    DateFields {
        julian,
        year: c.year,
        month: c.month,
        day_of_month: c.day,
        day_of_year: julian - julian_from_civil(c.year, 1, 1) + 1,
        weekday: weekday_of_julian(julian),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_2000() {
        assert_eq!(julian_from_civil(2000, 1, 1), JULIAN_2000_01_01);
        assert_eq!(
            civil_from_julian(JULIAN_2000_01_01),
            CivilDate {
                year: 2000,
                month: 1,
                day: 1
            }
        );
        assert_eq!(weekday_of_julian(JULIAN_2000_01_01), 6);
        assert_eq!(day_of_year_of_julian(JULIAN_2000_01_01), 1);
    }

    #[test]
    fn leap_years() {
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(2100));
        assert!(is_leap_year(2400));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2004));
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(2100, 2), 28);
        assert_eq!(days_in_month(2400, 2), 29);
        // Feb 29 exists exactly in leap years.
        assert_eq!(julian_from_civil(2100, 3, 1) - julian_from_civil(2100, 2, 28), 1);
        assert_eq!(julian_from_civil(2000, 3, 1) - julian_from_civil(2000, 2, 28), 2);
        assert_eq!(julian_from_civil(2400, 3, 1) - julian_from_civil(2400, 2, 28), 2);
    }

    #[test]
    fn known_dates() {
        // Published reference values.
        assert_eq!(julian_from_civil(1858, 11, 17), 2_400_001); // MJD 0 is noon of JD 2400000.5
        assert_eq!(julian_from_civil(1970, 1, 1), 2_440_588);
        assert_eq!(julian_from_civil(-4713, 11, 24), 0); // proleptic Gregorian epoch of JDN 0
        assert_eq!(weekday_of_julian(julian_from_civil(1970, 1, 1)), 4); // Thursday
    }

    #[test]
    fn round_trip_wide_range() {
        let start = julian_from_civil(1600, 1, 1);
        let end = julian_from_civil(2800, 12, 31);
        let mut prev = civil_from_julian(start - 1);
        for j in start..=end {
            let c = civil_from_julian(j);
            assert_eq!(julian_from_civil(c.year, c.month, c.day), j);
            assert!(is_valid_civil_date(c.year, c.month, c.day));
            // Consecutive days advance by exactly one calendar day.
            if c.day == 1 {
                assert_eq!(prev.day, days_in_month(prev.year, prev.month));
            } else {
                assert_eq!(c.day, prev.day + 1);
            }
            prev = c;
        }
    }

    #[test]
    fn day_of_year_and_weeks() {
        assert_eq!(day_of_year_of_julian(julian_from_civil(2000, 12, 31)), 366);
        assert_eq!(day_of_year_of_julian(julian_from_civil(2001, 12, 31)), 365);
        assert_eq!(week_of_year(1), 1);
        assert_eq!(week_of_year(7), 1);
        assert_eq!(week_of_year(8), 2);
        assert_eq!(week_of_year(366), 53);
    }
}
