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

#[cfg(test)]
mod tests {
    use super::rule_n;

    #[test]
    fn raw_base_selection_sign_and_unsigned_saturation() {
        let cases: &[(&[u8], u32)] = &[
            (b"16", 16),
            (b"1A", 26),
            (b"abc", 0xABC),
            (b"xyz", 0),
            (b"0X10", 16),
            (b" 1A", 1),
            (b" 0x10", 0),
            (b"\t16", 16),
            (b"-1", u32::MAX),
            (b"+12", 12),
            (b"-0x10", 0),
            (b"4294967295", u32::MAX),
            (b"4294967296", u32::MAX),
            (b"-4294967295", 1),
            (b"-4294967296", u32::MAX),
            (b"0xFFFFFFFF", u32::MAX),
            (b"0x100000000", u32::MAX),
            (b"12345678", 12_345_678),
            (b"12gA", 12),
            (b"0x12\xFF", 18),
            (b"", 0),
        ];
        for &(text, expected) in cases {
            assert_eq!(rule_n(text), expected, "{text:?}");
        }
    }
}
