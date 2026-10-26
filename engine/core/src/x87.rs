// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The game's x87 arithmetic (`docs/specs/foundations.md` §18): ordinary
//! PC24 operations round once to a 24-bit significand; native Lua re-entry
//! carries live precision (§18.4). The
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

/// Invocation context for the PC24 and PC53 simulation paths (foundations
/// §18.4). This changes arithmetic, never the width of an explicit store.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Precision {
    #[default]
    Bits24,
    Bits53,
}

impl Precision {
    #[inline]
    pub fn round(self, x: f64) -> f64 {
        match self {
            Self::Bits24 => r24(x),
            Self::Bits53 => x,
        }
    }
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

/// Round half to even exactly as the original's idiom computes it (crime §6
/// notation `rhe`, pollution deals) *[compat]*:
/// `int32(bits(f32(x + 1.5·2²³)) − 0x4B400000)` with a 32-bit wrapping
/// subtraction. Round half to even for |x| ≤ 2²²; beyond that the bit
/// pattern of the sum (5,000,000 → 4,597,152; 4,294,967,296 → 71,327,744;
/// −5,000,000 → −5,805,696). ±∞ give the bits of ±∞; a NaN input passes
/// through the addition as the same NaN, made quiet (x87).
pub fn rhe(x: f32) -> i32 {
    let sum = if x.is_nan() {
        f32::from_bits(x.to_bits() | 0x0040_0000)
    } else {
        x + 12_582_912.0f32
    };
    sum.to_bits().wrapping_sub(0x4B40_0000) as i32
}

/// `fild qword` then `fmul dword` under 24-bit precision (§18.3 point 1): the
/// 64-bit integer enters exactly and the product is rounded once to `f32`
/// (round to nearest even). Exact for every `i64`, unlike going through `f64`.
pub fn mul_i64_f32(i: i64, f: f32) -> f32 {
    if i == 0 || f == 0.0 || !f.is_finite() {
        return (i as f64 * f64::from(f)) as f32;
    }
    // f = ±m · 2^e with an integer significand m < 2^24.
    let bits = f.to_bits();
    let neg = bits >> 31 != 0;
    let exp = ((bits >> 23) & 0xFF) as i32;
    let frac = i128::from(bits & 0x7F_FFFF);
    let (m, e) = if exp == 0 {
        (frac, -149)
    } else {
        (frac | 0x80_0000, exp - 150)
    };
    // |p| < 2^87: exact.
    let p = i128::from(i) * if neg { -m } else { m };
    // i128 → f32 rounds to nearest even; scaling by 2^e is exact in range.
    (f64::from(p as f32) * 2f64.powi(e)) as f32
}

/// `float(uint32 v)` (crime §6.4): the `int32` is loaded exactly; a negative
/// one has 2³² added, rounded once to 24 bits.
pub fn u32_to_float(v: i32) -> f64 {
    if v < 0 {
        r24(f64::from(v) + 4_294_967_296.0)
    } else {
        f64::from(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mul_i64_f32_is_one_rounding() {
        // budget §15 test 2: (998857 × 1.2f) is rounded once.
        assert_eq!(mul_i64_f32(998_857, 1.2), (998_857f64 * f64::from(1.2f32)) as f32);
        assert_eq!(mul_i64_f32(-3, 0.5), -1.5);
        assert_eq!(mul_i64_f32(0, 7.0), 0.0);
        // Beyond 2^53 the integer still enters exactly: 2^60 + 1 times 1.0
        // rounds to 2^60 in f32, and (2^24 + 1) · 2^36 rounds to even.
        assert_eq!(mul_i64_f32((1i64 << 60) + 1, 1.0), (1u64 << 60) as f32);
        assert_eq!(mul_i64_f32(((1i64 << 24) + 1) << 36, 1.0), (1u64 << 60) as f32);
        assert_eq!(
            mul_i64_f32(((1i64 << 24) + 3) << 36, 1.0),
            (((1u64 << 24) + 4) << 36) as f32
        );
    }

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
    fn rhe_and_unsigned_load() {
        assert_eq!((rhe(2.5), rhe(3.5), rhe(-2.5)), (2, 4, -2));
        assert_eq!((rhe(5_000_000.0), rhe(4_294_967_296.0)), (4_597_152, 71_327_744));
        assert_eq!((u32_to_float(-1), u32_to_float(7)), (4_294_967_296.0, 7.0));
    }

    #[test]
    fn r24_rounds_once() {
        assert_eq!(r24(0.1), f64::from(0.1f32));
        // Ties go to even: 1 + 2^-24 → 1, 1 + 3·2^-24 → 1 + 2^-22.
        assert_eq!(r24(1.0 + 2f64.powi(-24)), 1.0);
        assert_eq!(r24(1.0 + 3.0 * 2f64.powi(-24)), 1.0 + 2f64.powi(-22));
    }
}
