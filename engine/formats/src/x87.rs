// SPDX-License-Identifier: GPL-3.0-or-later
//! The game's x87 arithmetic as the exemplar getters and the text parsers see it (`docs/specs/foundations.md`
//! §18, `docs/specs/data-formats.md` §4.3 and §5.1). The main thread runs at **24-bit precision**: every operation
//! rounds its exact result once to a 24-bit significand (round to nearest, ties to even). The exponent keeps the
//! register's wide range, so no intermediate value overflows or becomes subnormal; only the final store to a
//! Float32 does. Values are carried in `f64`, which holds every 24-bit significand exactly.
//!
//! The helpers compute each result exactly (integer arithmetic, or an error-free sum) before the one rounding, so
//! there is no double rounding through `f64`.

/// `x = (-1)^neg · m · 2^e` for a finite non-zero `x` (`m < 2^53`).
fn parts(x: f64) -> (bool, u64, i32) {
    let bits = x.to_bits();
    let neg = bits >> 63 != 0;
    let exp = ((bits >> 52) & 0x7FF) as i32;
    let frac = bits & ((1 << 52) - 1);
    if exp == 0 {
        (neg, frac, -1074)
    } else {
        (neg, frac | (1 << 52), exp - 1075)
    }
}

/// `x · 2^e`, exact while the result is a normal `f64`.
fn ldexp(mut x: f64, mut e: i32) -> f64 {
    while e > 1000 {
        x *= 2f64.powi(1000);
        e -= 1000;
    }
    while e < -1000 {
        x *= 2f64.powi(-1000);
        e += 1000;
    }
    x * 2f64.powi(e)
}

/// The exact value `(-1)^neg · (m + δ) · 2^e` rounded to a 24-bit significand, nearest with ties to even. `sticky`
/// says that a non-zero `δ` (`0 < δ < 1`) was cut off below `m`'s last bit.
fn round_parts(neg: bool, mut m: u128, mut e: i32, sticky: bool) -> f64 {
    if m == 0 {
        return if neg { -0.0 } else { 0.0 };
    }
    // Keep at least two bits below the 24 kept ones, so `sticky` is only ever "below the half".
    let bits = 128 - m.leading_zeros() as i32;
    if bits < 26 {
        m <<= 26 - bits;
        e -= 26 - bits;
    }
    let bits = 128 - m.leading_zeros() as i32;
    let shift = bits - 24;
    let mut kept = m >> shift;
    let rem = m & ((1u128 << shift) - 1);
    let half = 1u128 << (shift - 1);
    if rem > half || (rem == half && (sticky || kept & 1 == 1)) {
        kept += 1;
    }
    let v = ldexp(kept as f64, e + shift);
    if neg {
        -v
    } else {
        v
    }
}

/// One rounding of `x` to a 24-bit significand (the register keeps its exponent).
pub(crate) fn round24(x: f64) -> f64 {
    if x == 0.0 || !x.is_finite() {
        return x;
    }
    let (neg, m, e) = parts(x);
    round_parts(neg, u128::from(m), e, false)
}

/// `a × b` (`fmul`) at 24-bit precision.
pub(crate) fn mul(a: f64, b: f64) -> f64 {
    if a == 0.0 || b == 0.0 || !a.is_finite() || !b.is_finite() {
        return a * b;
    }
    let (na, ma, ea) = parts(a);
    let (nb, mb, eb) = parts(b);
    round_parts(na != nb, u128::from(ma) * u128::from(mb), ea + eb, false)
}

/// `a ÷ b` (`fdiv`) at 24-bit precision. Division by zero gives ±∞, and 0 ÷ 0 NaN (exceptions masked).
pub(crate) fn div(a: f64, b: f64) -> f64 {
    if a == 0.0 || b == 0.0 || !a.is_finite() || !b.is_finite() {
        return a / b;
    }
    let (na, ma, ea) = parts(a);
    let (nb, mb, eb) = parts(b);
    // Normalise both significands to 53 bits; the quotient then has 74 or 75 bits.
    let (za, zb) = (ma.leading_zeros() - 11, mb.leading_zeros() - 11);
    let num = u128::from(ma << za) << 74;
    let den = u128::from(mb << zb);
    round_parts(
        na != nb,
        num / den,
        ea - za as i32 - 74 - (eb - zb as i32),
        num % den != 0,
    )
}

/// `a + b` (`fadd`) at 24-bit precision.
pub(crate) fn add(a: f64, b: f64) -> f64 {
    let s = a + b;
    if s == 0.0 || !s.is_finite() {
        return s;
    }
    // Error-free sum: a + b = s + err exactly, |err| ≤ half an f64 ulp of s.
    let bb = s - a;
    let err = (a - (s - bb)) + (b - bb);
    if err == 0.0 {
        return round24(s);
    }
    let (neg, m, e) = parts(s);
    // Any point strictly between s and s ± half an ulp lies on the same side of every 24-bit midpoint as the exact
    // sum, so a quarter ulp towards err stands in for it.
    let away = (err > 0.0) != neg;
    let m4 = if away {
        u128::from(m) * 4 + 1
    } else {
        u128::from(m) * 4 - 1
    };
    round_parts(neg, m4, e - 2, false)
}

/// `_ftol` (foundations §18.1, data-formats §5.1): truncation toward zero to an `int64`. NaN, ±∞ and magnitudes
/// ≥ 2⁶³ give `0x8000000000000000`; a narrower target keeps the low bits (it wraps, it does not saturate).
pub(crate) fn ftol(x: f64) -> i64 {
    const TWO_POW_63: f64 = 9_223_372_036_854_775_808.0;
    if x.is_nan() || !(-TWO_POW_63..TWO_POW_63).contains(&x) {
        i64::MIN
    } else {
        x as i64
    }
}

/// An unsigned 64-bit value as the to-double converter loads it (data-formats §5.1, EX2): low 63 bits as a signed
/// integer plus 2⁶³ or 0, added at 24-bit precision, so the result has 24 significant bits even for a Float64 target.
pub(crate) fn u64_to_f64(v: u64) -> f64 {
    // One rounding of the exact value (Rust's integer-to-float conversion rounds to nearest, ties to even).
    f64::from(v as f32)
}

/// An unsigned 32-bit value as the to-double converter loads it (§5.1): below 2³¹ exactly; from 2³¹ the value loaded
/// as signed plus 2³², added at 24-bit precision.
pub(crate) fn u32_to_f64(v: u32) -> f64 {
    if v < 0x8000_0000 {
        f64::from(v)
    } else {
        f64::from(v as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Exact reference: round the exact rational `num / den` (both positive) to 24 bits.
    fn reference(num: u128, den: u128) -> f64 {
        // Scale so the quotient has at least 26 bits, then round with a sticky remainder.
        let mut shift = 0;
        while (num << shift) / den < (1 << 26) {
            shift += 1;
        }
        let q = (num << shift) / den;
        round_parts(false, q, -shift, (num << shift) % den != 0)
    }

    #[test]
    fn round24_is_one_rounding_with_ties_to_even() {
        assert_eq!(round24(0.1), f64::from(0.1f32));
        assert_eq!(round24(1.0 + 2f64.powi(-24)), 1.0);
        assert_eq!(round24(1.0 + 3.0 * 2f64.powi(-24)), 1.0 + 2f64.powi(-22));
        assert_eq!(round24(-(1.0 + 3.0 * 2f64.powi(-24))), -(1.0 + 2f64.powi(-22)));
        // The register's exponent range: 1e50 and 1e-50 keep a 24-bit significand, neither infinite nor zero.
        for x in [1e50, 1e-50] {
            let r = round24(x);
            assert_eq!(r.to_bits() & ((1 << 29) - 1), 0, "{x}");
            assert!((r - x).abs() <= x * 2f64.powi(-24), "{x}");
        }
        assert_eq!(round24(16_777_217.0), 16_777_216.0);
        assert_eq!(round24(16_777_219.0), 16_777_220.0);
    }

    #[test]
    fn products_and_quotients_are_exact_before_rounding() {
        // scale × 0.1 (a double constant): the 77-bit product is rounded once.
        let s = f64::from(0.1f32);
        let p = mul(s, 0.1);
        assert_eq!(p, f64::from(f32::from_bits(0x3C23_D70A)));
        // 5 × 0x3C23D70A is a tie at 24 bits; it goes to the even significand.
        assert_eq!(mul(5.0, p), f64::from(f32::from_bits(0x3D4C_CCCC)));
        // Division: 1 / 10^10 mod 2^32, and a few against an exact reference.
        assert_eq!(div(1.0, 1_410_065_408.0), reference(1, 1_410_065_408));
        assert_eq!(div(2.0, 3.0), reference(2, 3));
        assert_eq!(div(-2.0, 3.0), -reference(2, 3));
        assert_eq!(div(1.0, 0.0), f64::INFINITY);
        assert!(div(0.0, 0.0).is_nan());
        assert_eq!(mul(3.0, 1_000_000_007.0), reference(3_000_000_021, 1));
    }

    #[test]
    fn sums_round_once() {
        // 1 + 2^-24 + 2^-60: the exact sum is just above the midpoint, so it rounds up although the f64 sum is the
        // midpoint itself.
        let a = 1.0 + 2f64.powi(-24);
        assert_eq!(add(a, 2f64.powi(-60)), 1.0 + 2f64.powi(-23));
        assert_eq!(add(a, -(2f64.powi(-60))), 1.0);
        assert_eq!(
            add(3.0, 0.100_000_001_490_116_12),
            f64::from(f32::from_bits(0x4046_6666))
        );
        assert_eq!(add(1.0, -1.0), 0.0);
    }

    #[test]
    fn ftol_wraps_and_unsigned_loads_keep_24_bits() {
        assert_eq!(ftol(-2.75), -2);
        assert_eq!(ftol(f64::NAN), i64::MIN);
        assert_eq!(ftol(f64::INFINITY), i64::MIN);
        assert_eq!(ftol(1e19), i64::MIN);
        assert_eq!(ftol(300.7) as u8, 44);
        assert_eq!(u64_to_f64(16_777_217), 16_777_216.0);
        assert_eq!(u32_to_f64(16_777_217), 16_777_217.0);
        assert_eq!(u32_to_f64(0x8000_0001), 2_147_483_648.0);
    }
}
