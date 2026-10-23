// SPDX-License-Identifier: GPL-3.0-or-later
//! Exact-dyadic admission, not a general x87 emulator (RB6).
use super::{Result, Unsupported};

pub(super) fn valid(v: f64) -> Result<()> {
    if !v.is_finite() || (v != 0.0 && v.abs() < f64::from(f32::MIN_POSITIVE)) {
        Err(Unsupported::NonFiniteOrSubnormal)
    } else {
        Ok(())
    }
}

// Decode exact binary64 as odd signed integer * 2^exponent. All callers have
// finite normal binary64 values or zero, checked before decoding.
fn dyadic(v: f64) -> (i128, i32) {
    if v == 0.0 {
        return (0, 0);
    }
    let bits = v.to_bits();
    let mantissa = (bits & ((1_u64 << 52) - 1)) | (1_u64 << 52);
    let trailing = mantissa.trailing_zeros();
    let sign = if bits >> 63 == 0 { 1 } else { -1 };
    (
        i128::from(mantissa >> trailing) * sign,
        ((bits >> 52) & 0x7ff) as i32 - 1023 - 52 + trailing as i32,
    )
}

fn admitted(n: i128) -> Result<()> {
    let m = n.unsigned_abs();
    let width = if m == 0 {
        0
    } else {
        128 - m.leading_zeros() - m.trailing_zeros()
    };
    if width > 53 {
        Err(Unsupported::InexactUnstoredArithmetic)
    } else {
        Ok(())
    }
}

pub(super) fn add(a: f64, b: f64) -> Result<f64> {
    valid(a)?;
    valid(b)?;
    let (am, ae) = dyadic(a);
    let (bm, be) = dyadic(b);
    if am != 0 && bm != 0 {
        let e = ae.min(be);
        // Reject an unresolved wide exact sum instead of silently rounding it.
        let shifted = |m: i128, shift: i32| -> Result<i128> {
            if shift >= 74 {
                return Err(Unsupported::InexactUnstoredArithmetic);
            }
            m.checked_mul(1_i128 << shift)
                .ok_or(Unsupported::InexactUnstoredArithmetic)
        };
        admitted(
            shifted(am, ae - e)?
                .checked_add(shifted(bm, be - e)?)
                .ok_or(Unsupported::InexactUnstoredArithmetic)?,
        )?;
    }
    let result = a + b;
    valid(result)?;
    Ok(result)
}
pub(super) fn sub(a: f64, b: f64) -> Result<f64> {
    add(a, -b)
}
pub(super) fn mul(a: f64, b: f64) -> Result<f64> {
    valid(a)?;
    valid(b)?;
    admitted(
        dyadic(a)
            .0
            .checked_mul(dyadic(b).0)
            .ok_or(Unsupported::InexactUnstoredArithmetic)?,
    )?;
    let result = a * b;
    valid(result)?;
    Ok(result)
}
pub(super) fn dot(a: [f64; 2], b: [f64; 2]) -> Result<f64> {
    add(mul(a[0], b[0])?, mul(a[1], b[1])?)
}
pub(super) fn store(v: f64) -> Result<f32> {
    valid(v)?;
    let result = v as f32;
    if !result.is_finite() || (v != 0.0 && result == 0.0) || result.is_subnormal() {
        Err(Unsupported::Binary32StoreRange)
    } else {
        Ok(result)
    }
}
