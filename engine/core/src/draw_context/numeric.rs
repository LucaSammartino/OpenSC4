// SPDX-License-Identifier: GPL-3.0-or-later
//! Qualified DC8 arithmetic, not a generic x87 emulator. Only nearest/even,
//! 64-significand-bit control with gradual underflow is modeled. Arithmetic is
//! admitted only when its exact dyadic intermediate fits binary64 (53 bits).
//! Thus binary64 and x87-64 agree without a guessed ambient control word. Wider
//! intermediate cancellation, nonfinite operands and unproved reciprocals are
//! UnsupportedNumeric, rather than silently using a generic matrix multiply.
use super::Error;

fn dyadic(value: f64) -> Result<(i128, i32), Error> {
    if !value.is_finite() {
        return Err(Error::UnsupportedNumeric);
    }
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 2047) as i32;
    let mut significand = i128::from(bits & ((1_u64 << 52) - 1));
    if exponent != 0 {
        significand |= 1_i128 << 52;
    }
    if bits >> 63 != 0 {
        significand = -significand;
    }
    Ok(normalize(significand, if exponent == 0 { -1074 } else { exponent - 1075 }))
}

fn normalize(mut significand: i128, mut exponent: i32) -> (i128, i32) {
    if significand == 0 {
        return (0, 0);
    }
    let zeros = significand.trailing_zeros();
    significand >>= zeros;
    exponent += zeros as i32;
    (significand, exponent)
}

fn admit(significand: i128, exponent: i32) -> Result<(), Error> {
    let (significand, exponent) = normalize(significand, exponent);
    let bits = 128 - significand.unsigned_abs().leading_zeros();
    if bits > 53 || (bits != 0 && (exponent < -1022 || exponent + bits as i32 > 1024)) {
        return Err(Error::UnsupportedNumeric);
    }
    Ok(())
}

pub(super) fn add(a: f64, b: f64) -> Result<f64, Error> {
    let (left, le) = dyadic(a)?;
    let (right, re) = dyadic(b)?;
    if left == 0 { return Ok(b); }
    if right == 0 { return Ok(a); }
    let exponent = le.min(re);
    let shift = |value: i128, count: i32| {
        1_i128.checked_shl(count as u32).and_then(|factor| value.checked_mul(factor))
    };
    let sum = shift(left, le - exponent)
        .and_then(|l| shift(right, re - exponent).and_then(|r| l.checked_add(r)))
        .ok_or(Error::UnsupportedNumeric)?;
    admit(sum, exponent)?;
    Ok(a + b)
}

pub(super) fn mul(a: f64, b: f64) -> Result<f64, Error> {
    let (left, le) = dyadic(a)?;
    let (right, re) = dyadic(b)?;
    admit(left.checked_mul(right).ok_or(Error::UnsupportedNumeric)?, le + re)?;
    Ok(a * b)
}

pub(super) fn store(value: f64) -> Result<f32, Error> {
    if !value.is_finite() { return Err(Error::UnsupportedNumeric); }
    let result = value as f32;
    if !result.is_finite() { return Err(Error::UnsupportedNumeric); }
    Ok(result)
}

pub(super) fn reciprocal(scale: f32) -> Result<f64, Error> {
    let (significand, exponent) = dyadic(f64::from(scale))?;
    // Powers of two only: division is exact in both qualified precisions.
    if significand.unsigned_abs() != 1 || !(-126..=126).contains(&exponent) {
        return Err(Error::UnsupportedNumeric);
    }
    Ok(1.0 / f64::from(scale))
}
