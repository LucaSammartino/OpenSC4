// SPDX-License-Identifier: GPL-3.0-or-later

//! Shared Rule N conversion (docs/specs/data-formats.md §4.3).
//! Kept in core so framework owners and format readers use the same primitive.

/// Rule N: select hexadecimal on a raw `0x` prefix or a hex letter before
/// the first non-hex byte; otherwise decimal. Conversion then skips C whitespace,
/// accepts a sign, stops at a non-digit and saturates unsigned 32-bit overflow.
pub fn rule_n(s: &[u8]) -> u32 {
    let hex = s.starts_with(b"0x")
        || s.starts_with(b"0X")
        || s.iter()
            .take_while(|b| b.is_ascii_hexdigit())
            .any(u8::is_ascii_alphabetic);
    strtoul(s, if hex { 16 } else { 10 })
}

fn strtoul(s: &[u8], base: u32) -> u32 {
    let mut i = s
        .iter()
        .take_while(|&&b| matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C))
        .count();
    let neg = match s.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };
    if base == 16
        && s.get(i) == Some(&b'0')
        && matches!(s.get(i + 1), Some(b'x' | b'X'))
        && s.get(i + 2).is_some_and(u8::is_ascii_hexdigit)
    {
        i += 2;
    }
    let mut v: u32 = 0;
    let mut over = false;
    while let Some(d) = s.get(i).and_then(|&c| char::from(c).to_digit(base)) {
        match v.checked_mul(base).and_then(|v| v.checked_add(d)) {
            Some(n) => v = n,
            None => over = true,
        }
        i += 1;
    }
    if over {
        u32::MAX
    } else if neg {
        v.wrapping_neg()
    } else {
        v
    }
}
