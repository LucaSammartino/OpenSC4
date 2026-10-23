// SPDX-License-Identifier: GPL-3.0-or-later
//! Standalone counted-byte `UTF8IsStringValid` policy, public utf8-validation.md §2–§3.
//!
//! This is the historical byte policy, including five/six-byte forms, surrogates,
//! and values above modern Unicode's limit. It is not a Unicode scalar decoder.
//! A slice supplies exactly the readable counted span, with no NUL convention.
//! Native pointer validity, wrapping addresses, caller lifetime and return-register
//! upper bits are outside this portable API; there is no native/C ABI exposure.

/// Largest supported counted span (public spec §2).
pub const MAX_COUNT: usize = 0x7fff_ffff;

/// The requested slice length is outside the supported count domain.
/// This is distinct from a supported span whose byte policy returns false.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnsupportedCount;

/// Validate the entire explicitly counted slice using the public historical policy.
///
/// To validate a prefix of a larger backing buffer, pass `&bytes[..count]`.
/// Empty spans succeed without a byte read; embedded zero is an ordinary byte.
/// No allocation, scalar decoding, normalization or host validator is used.
pub fn utf8_is_string_valid(bytes: &[u8]) -> Result<bool, UnsupportedCount> {
    check_count(bytes.len())?;
    Ok(walk(bytes.len(), |offset| bytes[offset]))
}

fn check_count(count: usize) -> Result<(), UnsupportedCount> {
    if count > MAX_COUNT {
        Err(UnsupportedCount)
    } else {
        Ok(())
    }
}

// Both production and read-order tests use this single owned byte walker. The
// reader returns raw bytes, not prevalidated classes or decisions. Callers have
// already checked the count; every read stays within that counted span (§3).
fn walk(count: usize, mut read: impl FnMut(usize) -> u8) -> bool {
    let mut current = 0;
    while current < count {
        let lead = read(current);
        let width = match lead {
            0x00..=0x7f => 1,
            0x80..=0xc1 | 0xfe..=0xff => return false,
            0xc2..=0xdf => 2,
            0xe0..=0xef => 3,
            0xf0..=0xf7 => 4,
            0xf8..=0xfb => 5,
            0xfc..=0xfd => 6,
        };
        // Guard the TOTAL width before any continuation read, including when
        // backing storage happens to extend beyond the specified span (§3).
        if count - current < width {
            return false;
        }
        let mut first = 0;
        for position in 1..width {
            let byte = read(current + position);
            if !(0x80..=0xbf).contains(&byte) {
                return false;
            }
            if position == 1 {
                first = byte;
            }
        }
        if width >= 3 {
            // §3 and §5.4: only after ALL continuation checks, re-read the lead
            // for the exceptional first-continuation bound. Cache the first
            // continuation; the public ordered traces do not re-read it.
            let minimum = match read(current) {
                0xe0 => 0xa0,
                0xf0 => 0x90,
                0xf8 => 0x88,
                0xfc => 0x84,
                _ => 0x80,
            };
            if first < minimum {
                return false;
            }
        }
        current += width;
    }
    true
}

#[cfg(test)]
mod tests;
