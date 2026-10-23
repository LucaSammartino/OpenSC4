// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit PC24 / nearest-even / masked model, normal or zero intermediates.
//! These APIs do not inspect/set the host FP control word or promise ambient
//! x87, PC64, extended-exponent, subnormal, transcendental or trap equivalence.
use super::{Plane, Vec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NumericUnsupported;
type Result<T> = std::result::Result<T, NumericUnsupported>;

fn input(x: f32) -> Result<f32> {
    if x == 0.0 || x.is_normal() {
        Ok(x)
    } else {
        Err(NumericUnsupported)
    }
}
fn checked(value: f32, exact: f64) -> Result<f32> {
    if !exact.is_finite() || (exact != 0.0 && exact.abs() < f64::from(f32::MIN_POSITIVE)) {
        return Err(NumericUnsupported);
    }
    input(value)
}
pub(super) fn add(a: f32, b: f32) -> Result<f32> {
    input(a)?;
    input(b)?;
    checked(a + b, f64::from(a) + f64::from(b))
}
pub(super) fn sub(a: f32, b: f32) -> Result<f32> {
    add(a, -b)
}
pub(super) fn mul(a: f32, b: f32) -> Result<f32> {
    input(a)?;
    input(b)?;
    checked(a * b, f64::from(a) * f64::from(b))
}
pub(super) fn div(a: f32, b: f32) -> Result<f32> {
    input(a)?;
    input(b)?;
    checked(a / b, f64::from(a) / f64::from(b))
}

pub fn squared_length(a: Vec3, b: Vec3) -> Result<f32> {
    let d = [sub(b[0], a[0])?, sub(b[1], a[1])?, sub(b[2], a[2])?];
    add(add(mul(d[2], d[2])?, mul(d[1], d[1])?)?, mul(d[0], d[0])?)
}
pub fn length(a: Vec3, b: Vec3) -> Result<f32> {
    input(squared_length(a, b)?.sqrt())
}
pub fn plane(p: Vec3, n: Vec3) -> Result<Plane> {
    let w = -add(add(mul(p[1], n[1])?, mul(p[2], n[2])?)?, mul(p[0], n[0])?)?;
    Ok([n[0], n[1], n[2], w])
}

fn parts32(x: f32) -> Result<(i128, i32)> {
    input(x)?;
    if x == 0.0 {
        return Ok((0, 0));
    }
    let bits = x.to_bits();
    let m = i128::from((bits & 0x7fffff) | 0x800000);
    Ok((if bits >> 31 == 0 { m } else { -m }, ((bits >> 23) & 255) as i32 - 150))
}

// Exact dyadic integer rounding avoids a binary64-to-PC24 double round when
// the instruction has a double-width constant operand (public §6.1).
fn round24(m: i128, exponent: i32) -> Result<f32> {
    if m == 0 {
        return Ok(0.0);
    }
    let sign = if m < 0 { 0x80000000 } else { 0 };
    let m = m.unsigned_abs();
    let bits = 128 - m.leading_zeros() as i32;
    let shift = bits - 24;
    let mut significand = if shift > 0 {
        let q = m >> shift;
        let remainder = m & ((1u128 << shift) - 1);
        let half = 1u128 << (shift - 1);
        q + u128::from(remainder > half || (remainder == half && q & 1 != 0))
    } else {
        m << -shift
    };
    let mut e = exponent + shift + 23;
    if significand == 1 << 24 {
        significand >>= 1;
        e += 1;
    }
    if !(-126..=127).contains(&e) {
        return Err(NumericUnsupported);
    }
    Ok(f32::from_bits(
        sign | (((e + 127) as u32) << 23) | (significand as u32 & 0x7fffff),
    ))
}

fn double_operand(x: f32, constant: f64, multiply: bool) -> Result<f32> {
    let (m, e) = parts32(x)?;
    let bits = constant.to_bits();
    let c = i128::from((bits & ((1u64 << 52) - 1)) | (1u64 << 52));
    let ce = ((bits >> 52) & 2047) as i32 - 1075;
    if multiply {
        return round24(m * c, e + ce);
    }
    if m == 0 {
        return round24(c, ce);
    }
    let base = e.min(ce);
    let left_shift = e - base;
    let right_shift = ce - base;
    // Explicit finite supported domain, no overflowing portable accumulator.
    if left_shift > 70 || right_shift > 70 {
        return Err(NumericUnsupported);
    }
    let left = m.checked_shl(left_shift as u32).ok_or(NumericUnsupported)?;
    let right = c.checked_shl(right_shift as u32).ok_or(NumericUnsupported)?;
    round24(left.checked_add(right).ok_or(NumericUnsupported)?, base)
}

pub fn colour_byte(value: f32) -> Result<u8> {
    let stored = add(value, 49152.0)?;
    let bits = stored.to_bits().wrapping_add(0xb8c00000);
    Ok((bits as u8).wrapping_sub((bits >> 8) as u8))
}
pub fn lit_colour(light: Vec3, normal: Vec3, rgb: Vec3, alpha: f32) -> Result<[u8; 4]> {
    let dot = add(
        add(mul(light[2], normal[2])?, mul(light[1], normal[1])?)?,
        mul(light[0], normal[0])?,
    )?;
    let k = double_operand(double_operand(dot, 0.7, true)?, 0.3, false)?;
    // PC24 normal-range products equal their binary32 stores; this does not
    // generalize the R resident vs G/B stored distinction to PC64.
    let r = colour_byte(mul(k, rgb[0])?)?;
    let g = colour_byte(mul(k, rgb[1])?)?;
    let b = colour_byte(mul(k, rgb[2])?)?;
    Ok([b, g, r, colour_byte(alpha)?])
}
