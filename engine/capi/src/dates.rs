// SPDX-License-Identifier: GPL-3.0-or-later
//! Scalar bridge for the compiled cIGZDate slots (dates.md §§3–4).
use fc4_core::dates::{self, RzDate};

/// Pure value operations; pointer dispatch, ownership and in-place writes stay
/// in the COM adapter. Unsupported slots return 0 and are not a success result.
#[no_mangle]
pub extern "C" fn fc4_gzdate_value(slot: u32, n: u32, a: u32, b: u32, c: u32) -> u32 {
    super::guard(0, || {
        let mut d = RzDate::from_day(n);
        match slot {
            4 | 8 => a,
            6 | 44 => dates::jday(a, b, c),
            7 => d.set_day_of_year(a, b),
            10 => u32::from(d.between(a, b)),
            11 => d.compare_to(a) as u32,
            12 => d.weekday(),
            13 => d.year(),
            14 => d.day_of_year(),
            15 => d.day_of_month(),
            16 => d.first_day_of_month(a),
            17 => d.first_day_of_month(d.month()),
            18 | 26 => n,
            19 => u32::from(d.is_valid()),
            20 => u32::from(d.leap()),
            21 => d.max_with(a),
            22 => d.min_with(a),
            23 => d.month(),
            24 => d.previous(a).n,
            25 => d.year() % 100,
            27 => u32::from(n < a),
            28 => u32::from(n <= a),
            29 => u32::from(n > a),
            30 => u32::from(n >= a),
            31 => u32::from(n == a),
            32 => u32::from(n != a),
            33 | 34 | 40 => n.wrapping_sub(a),
            35 | 36 | 39 => n.wrapping_add(a),
            37 => n.wrapping_add(1),
            38 => n.wrapping_sub(1),
            42 => u32::from(dates::day_within_month(a, b, c)),
            43 => dates::days_in_year(a),
            45 => u32::from(dates::leap_year(a)),
            _ => 0,
        }
    })
}

/// name_len is at most three readable bytes, including any stopping NUL.
/// fold null explicitly selects C locale; otherwise 256 captured u32 entries.
#[no_mangle]
pub unsafe extern "C" fn fc4_gzdate_month(name: *const u8, name_len: usize, fold: *const u32) -> u32 {
    super::guard(1, || {
        if name.is_null() {
            return 1;
        } // bridge refusal; COM must not hide native null-name faults
        let bytes = std::slice::from_raw_parts(name, name_len.min(3));
        if fold.is_null() {
            dates::month_from_name(bytes)
        } else {
            let map = std::slice::from_raw_parts(fold, 256);
            dates::month_from_name_folded(bytes, |b| map[usize::from(b)])
        }
    })
}

/// Returns required bytes including NUL; writes only when capacity suffices.
/// Text only: the COM return register belongs to native temporary cleanup.
#[no_mangle]
pub unsafe extern "C" fn fc4_gzdate_text(n: u32, format: u32, out: *mut u8, cap: usize) -> usize {
    super::guard(0, || {
        let text = dates::date_string(n, format);
        let needed = text.len() + 1;
        if !out.is_null() && cap >= needed {
            std::ptr::copy_nonoverlapping(text.as_ptr(), out, text.len());
            *out.add(text.len()) = 0;
        }
        needed
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corrected_values_and_text() {
        assert_eq!(fc4_gzdate_value(6, 99, 2, 30, 2000), 0);
        assert_eq!(fc4_gzdate_value(7, 99, 366, 1999, 0), 2451545);
        assert_eq!(fc4_gzdate_value(16, 2451545, 3, 0, 0), 61);
        assert_eq!(fc4_gzdate_value(27, u32::MAX, 0, 0, 0), 0);
        unsafe {
            let mut text = [0u8; 40];
            assert_eq!(fc4_gzdate_text(2451546, 0, text.as_mut_ptr(), text.len()), 9);
            assert_eq!(&text[..9], b"1/2/2000\0");
            assert_eq!(fc4_gzdate_month(b"May\0Jun".as_ptr(), 7, std::ptr::null()), 5);
            assert_eq!(fc4_gzdate_month(b"M\0ay".as_ptr(), 4, std::ptr::null()), 1);
        }
    }
}
