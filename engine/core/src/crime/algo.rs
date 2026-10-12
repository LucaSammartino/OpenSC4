// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The arithmetic of §6.1–§6.5 as pure functions. The game runs the x87 at
//! 24-bit precision (foundations §18): every add, subtract, multiply and
//! divide rounds to `float32` once. An integer operand enters unrounded, so a
//! `float32 × integer` product is formed in `f64` (exact for these operand
//! sizes) and rounded to `f32` once ([`mul_f32`]) before the truncation or
//! rounding the spec names (§6 notation).

use crate::curve::ResponseCurve;
/// `trunc` to 32 bits (§6 notation): the CRT `_ftol` of foundations §18.1,
/// the low half of the int64 truncation (NaN, ±∞ and magnitudes ≥ 2⁶³ give
/// 0, the low half of the int64 indefinite).
pub use crate::x87::ftol32;
use crate::x87::r24;

/// `1/255` as the prop requester scales the average crime (§11.6).
pub const PROP_SCALE: f32 = 1.0 / 255.0;

/// 2³², added to a negative `int32` to convert it as unsigned (§6.4).
const TWO_POW_32: f64 = 4_294_967_296.0;

/// One x87 multiplication at 24-bit precision: the exact product of the two
/// unrounded operands, rounded once to `float32` (foundations §18.3).
pub fn mul_f32(a: f64, b: f64) -> f32 {
    (a * b) as f32
}

/// `rhe` exactly as the original's idiom computes it (§6 notation,
/// [compat]): `int32(bits(f32(x + 1.5·2²³)) − 0x4B400000)` with a 32-bit
/// wrapping subtraction. Round half to even for |x| ≤ 2²²; beyond that the
/// bit pattern of the sum (5,000,000 → 4,597,152; 4,294,967,296 →
/// 71,327,744; −5,000,000 → −5,805,696). ±∞ give the bits of ±∞; a NaN
/// input passes through the addition as the same NaN, made quiet (x87).
pub fn rhe(x: f32) -> i32 {
    let sum = if x.is_nan() {
        f32::from_bits(x.to_bits() | 0x0040_0000)
    } else {
        x + 12_582_912.0f32
    };
    sum.to_bits().wrapping_sub(0x4B40_0000) as i32
}

/// `float(uint32 v)` (§6.4): the `int32` is loaded exactly; a negative one
/// has 2³² added, rounded once to 24 bits.
pub fn u32_to_float(v: i32) -> f64 {
    if v < 0 {
        r24(f64::from(v) + TWO_POW_32)
    } else {
        f64::from(v)
    }
}

/// The census bucket of an EQ value (§6.1): `trunc(f32(EQ × 0.02f))`;
/// `None` when it is outside 0..3 ([choice]: such populations are ignored).
/// With the 24-bit product, EQ 50, 100, 150 and 200 round up to the next
/// bucket (EQ 200 → 4, ignored).
pub fn census_bucket(eq: f32) -> Option<usize> {
    let p = mul_f32(f64::from(eq), f64::from(0.02f32));
    if !p.is_finite() {
        return None;
    }
    let t = p.trunc();
    if (0.0..4.0).contains(&t) {
        Some(t as usize)
    } else {
        None
    }
}

/// The jobless factor `2 · (A − C) / (B − C) − 1` (§6.2), each operation a
/// `float32` one. [choice] 0 when `B = C`.
pub fn jobless_factor(a: f32, b: f32, c: f32) -> f32 {
    if b == c {
        return 0.0;
    }
    2.0 * (a - c) / (b - c) - 1.0
}

/// One wealth's term of the criminal base (§6.3):
/// `trunc(joblessY × popSum + Σ_b eqY[b] × pop[b])` with a `float32`
/// accumulator; every product and sum is one `float32` operation. `eq_y[b]`
/// is the EQ curve at `25·b`, `jobless_y` the jobless curve at the factor.
pub fn criminal_base_term(eq_y: [f32; 4], pop: [u32; 4], jobless_y: f32) -> i32 {
    let mut acc = 0.0f32;
    let mut pop_sum = 0u32;
    for b in 0..4 {
        acc += mul_f32(f64::from(eq_y[b]), f64::from(pop[b]));
        pop_sum = pop_sum.wrapping_add(pop[b]);
    }
    let jobless = mul_f32(f64::from(jobless_y), f64::from(pop_sum));
    ftol32(f64::from(jobless + acc))
}

/// Crimes to commit (§6.4) from the criminal base, `GetJailPopulation`,
/// `ResetUnjailedCriminals`, the reoffend multiplier, the ordinance factor
/// and the "criminality → crimes" curve.
pub fn crimes_to_commit(
    base: i32,
    jailed: u32,
    released: i32,
    reoffend: f32,
    ordinance_factor: f32,
    curve: &ResponseCurve,
) -> i32 {
    let reoffenders = ftol32(f64::from(mul_f32(f64::from(released), f64::from(reoffend))));
    let crimes = if (base as u32) > jailed {
        let input = (base as u32).wrapping_sub(jailed) as i32 as f32;
        ftol32(f64::from(curve.eval(input))).wrapping_add(reoffenders)
    } else {
        reoffenders
    };
    rhe(mul_f32(u32_to_float(crimes), f64::from(ordinance_factor)))
}

/// One cell of the decay (§6.5): non-zero cells become the low 8 bits of
/// `trunc(f32(v × decay))` (a decay above 1 wraps, [quirk]).
pub fn decay_value(v: u8, decay: f32) -> u8 {
    if v == 0 {
        0
    } else {
        ftol32(f64::from(mul_f32(f64::from(v), f64::from(decay)))) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped_crimes() -> ResponseCurve {
        ResponseCurve::from_points(&[
            (0.0, 0.0),
            (200.0, 1.0),
            (5000.0, 5.0),
            (10000.0, 10.0),
            (20000.0, 25.0),
            (200000.0, 40.0),
            (400000.0, 55.0),
            (600000.0, 60.0),
            (900000.0, 65.0),
        ])
    }

    /// Test 1.
    #[test]
    fn decay() {
        let cases: [(u8, f32, u8); 11] = [
            (200, 0.5, 100),
            (255, 0.5, 127),
            (1, 0.5, 0),
            (0, 0.5, 0),
            (200, 0.8, 160),
            (255, 0.8, 204),
            (5, 0.8, 4),
            (200, 1.5, 44),
            (0, 1.5, 0),
            (200, -0.5, 156),
            (200, f32::NAN, 0),
        ];
        for (v, d, want) in cases {
            assert_eq!(decay_value(v, d), want, "{v} × {d}");
        }
    }

    /// Test 2, with the 24-bit products of foundations §18 (the product
    /// 50 × 0.02f = 0.99999997765 rounds to 1.0f).
    #[test]
    fn buckets() {
        let b = |x: f32| census_bucket(x);
        assert_eq!(b(0.0), Some(0));
        assert_eq!(b(49.99), Some(0));
        assert_eq!(b(49.999_996), Some(0), "the float below 50");
        assert_eq!(b(50.0), Some(1));
        assert_eq!(b(50.000_004), Some(1));
        assert_eq!(b(99.99), Some(1));
        assert_eq!(b(100.0), Some(2));
        assert_eq!(b(100.000_01), Some(2));
        assert_eq!(b(150.0), Some(3));
        assert_eq!(b(199.99), Some(3));
        assert_eq!(b(200.0), None);
        assert_eq!(b(250.0), None);
        assert_eq!(b(-49.9), Some(0));
        assert_eq!(b(-50.1), None);
        assert_eq!(b(f32::NAN), None);
        assert_eq!(b(f32::INFINITY), None);
    }

    /// Test 3 (the factor; the curve values are in the vector).
    #[test]
    fn jobless() {
        assert_eq!(jobless_factor(0.0, 1000.0, -1000.0), 0.0);
        assert_eq!(jobless_factor(500.0, 1000.0, -1000.0), 0.5);
        assert_eq!(jobless_factor(-1000.0, 1000.0, -1000.0), -1.0);
        assert_eq!(jobless_factor(5.0, 3.0, 3.0), 0.0);
        assert!(jobless_factor(f32::NAN, 1.0, 0.0).is_nan());
        // f32 steps: 2 · 1 / 3 = 0.6666667f, − 1 = −0.3333333f.
        assert_eq!(jobless_factor(1.0, 3.0, 0.0), 0.666_666_7f32 - 1.0);
    }

    /// Test 4.
    #[test]
    fn criminal_base() {
        assert_eq!(criminal_base_term([0.4, 0.375, 0.35, 0.325], [1000, 0, 0, 0], 0.1), 500);
        assert_eq!(criminal_base_term([0.4, 0.375, 0.35, 0.325], [1234, 0, 0, 0], 0.0), 493);
        assert_eq!(criminal_base_term([0.0; 4], [100, 0, 0, 0], -0.1), -10);
        assert_eq!(
            criminal_base_term([0.0; 4], [u32::MAX, 1, 0, 0], 1.0),
            0,
            "wrapping popSum"
        );
        // 0.1f × 999999999 rounds to 1e8 in f32: 100000000.
        assert_eq!(criminal_base_term([0.0; 4], [999_999_999, 0, 0, 0], 0.1), 100_000_000);
    }

    /// Tests 5 and 6 (test 6 with the 24-bit products of foundations §18).
    #[test]
    fn crimes() {
        let c = shipped_crimes();
        let n = |base, jailed, released, factor| crimes_to_commit(base, jailed, released, 0.66, factor, &c);
        assert_eq!(n(200, 0, 0, 1.0), 1);
        assert_eq!(n(7500, 0, 0, 1.0), 7);
        assert_eq!(n(15000, 0, 0, 1.0), 17);
        assert_eq!(n(100, 0, 0, 1.0), 0);
        assert_eq!(n(1_000_000, 0, 0, 1.0), 65);
        assert_eq!(n(7500, 0, 10, 1.0), 13);
        assert_eq!(n(100, 100, 10, 1.0), 6, "base ≤ jailed");
        assert_eq!(n(100, 200, 10, 1.0), 6);
        assert_eq!(n(-5, 0, 0, 1.0), 0, "negative base: unsigned compare, curve clamps");
        let f =
            |k: i32, factor| crimes_to_commit(1, 0, 0, 1.0, factor, &ResponseCurve::from_points(&[(0.0, k as f32)]));
        assert_eq!(f(10, 0.95), 10, "f32(9.49999988) = 9.5 → 10");
        assert_eq!(f(50, 0.95), 48, "f32(47.4999994) = 47.5 → 48");
        assert_eq!(f(15, 1.2), 18);
        assert_eq!(f(2, 0.95), 2);
        assert_eq!(f(3, 0.5), 2, "1.5 → 2");
        assert_eq!(f(5, 0.5), 2, "2.5 → 2");
        // crimes = 0 + trunc(−10 × 1) = −10 → 2³² − 10, rounded to 2³² →
        // rhe beyond 2²² (§6.4, test 19).
        assert_eq!(
            crimes_to_commit(1, 0, -10, 1.0, 1.0, &ResponseCurve::default()),
            71_327_744
        );
    }

    /// Test 19 (§6 notation) and the edges of the formula.
    #[test]
    fn rhe_beyond_2_22() {
        let cases: [(f32, i32); 12] = [
            (4_194_304.0, 4_194_304),
            (4_194_305.0, 4_194_304),
            (4_194_307.0, 4_194_306),
            (5_000_000.0, 4_597_152),
            (8_388_608.0, 6_291_456),
            (100_000_000.0, 26_655_776),
            (4_294_967_296.0, 71_327_744),
            (-5_000_000.0, -5_805_696),
            (-4_194_304.0, -4_194_304),
            (2.5, 2),
            (-2.5, -2),
            (0.0, 0),
        ];
        for (x, want) in cases {
            assert_eq!(rhe(x), want, "rhe({x})");
        }
        assert_eq!(rhe(f32::INFINITY), 0x7F80_0000 - 0x4B40_0000);
        assert_eq!(
            rhe(f32::from_bits(0xFFC0_0000)),
            0xFFC0_0000u32.wrapping_sub(0x4B40_0000) as i32
        );
        assert_eq!(u32_to_float(-1), 4_294_967_296.0);
        assert_eq!(u32_to_float(7), 7.0);
        let c = ResponseCurve::from_points(&[(0.0, -1.0)]);
        assert_eq!(crimes_to_commit(1, 0, 0, 1.0, 1.0, &c), 71_327_744);
    }

    #[test]
    fn ftol() {
        assert_eq!(ftol32(-2.9), -2);
        assert_eq!(ftol32(4_294_967_297.5), 1);
        assert_eq!(ftol32(f64::NAN), 0);
        assert_eq!(ftol32(1e300), 0);
    }
}
