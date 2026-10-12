// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The game's x87 arithmetic (`docs/specs/foundations.md` §18): every
//! floating-point operation is rounded once to a 24-bit significand, and the
//! C runtime's truncating conversion `_ftol` works through a 64-bit integer.
//!
//! Compute one operation in `f64` (its operands are `f32` values, `double`
//! constants or integers, all of which enter unrounded) and pass the result
//! through [`r24`].

/// One rounding to a 24-bit significand (round to nearest even), the result
/// of any x87 operation under the game's precision control (§18.1).
#[inline]
pub fn r24(x: f64) -> f64 {
    f64::from(x as f32)
}

const TWO_POW_63: f64 = 9_223_372_036_854_775_808.0;

/// `_ftol` (§18.1): truncation toward zero as an `int64`. NaN, ±∞ and
/// magnitudes ≥ 2^63 give `0x8000000000000000`.
pub fn ftol(x: f64) -> i64 {
    if x.is_nan() || !(-TWO_POW_63..TWO_POW_63).contains(&x) {
        i64::MIN
    } else {
        x as i64
    }
}

/// `_ftol` kept to 32 bits (§18.1): the low half of [`ftol`]. NaN and
/// out-of-range values therefore give 0, and values above `int32` wrap.
pub fn ftol32(x: f64) -> i32 {
    ftol(x) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ftol_is_int64_truncation() {
        assert_eq!(ftol(-2.75), -2);
        assert_eq!(ftol(f64::NAN), i64::MIN);
        assert_eq!(ftol(f64::INFINITY), i64::MIN);
        assert_eq!(ftol(-TWO_POW_63), i64::MIN);
        assert_eq!(ftol(3.0e9), 3_000_000_000);
        // Low 32 bits: NaN → 0, 3e9 wraps, -2^31 stays.
        assert_eq!(ftol32(f64::NAN), 0);
        assert_eq!(ftol32(3.0e9), 3_000_000_000u32 as i32);
        assert_eq!(ftol32(-2_147_483_648.9), i32::MIN);
        assert_eq!(ftol32(4_294_967_296.5), 0);
    }

    #[test]
    fn r24_rounds_once() {
        assert_eq!(r24(0.1), f64::from(0.1f32));
        // Ties go to even: 1 + 2^-24 → 1, 1 + 3·2^-24 → 1 + 2^-22.
        assert_eq!(r24(1.0 + 2f64.powi(-24)), 1.0);
        assert_eq!(r24(1.0 + 3.0 * 2f64.powi(-24)), 1.0 + 2f64.powi(-22));
    }
}
