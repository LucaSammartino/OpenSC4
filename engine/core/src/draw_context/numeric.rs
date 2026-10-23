// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable x87 operations, with explicit precision and rounding. No host
//! floating arithmetic occurs between loads and the specified binary32 stores.
use super::Error;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Precision {
    Bits24,
    Bits53,
    Bits64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rounding {
    NearestEven,
    Down,
    Up,
    TowardZero,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arithmetic {
    pub precision: Precision,
    pub rounding: Rounding,
}
impl Default for Arithmetic {
    fn default() -> Self {
        Self {
            precision: Precision::Bits64,
            rounding: Rounding::NearestEven,
        }
    }
}

/// Finite value = (-1)^negative * significand * 2^(exponent-63).
/// SPECIAL denotes infinity (zero significand) or quiet NaN (nonzero).
/// This owner's finite expressions stay within the extended exponent range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Float {
    negative: bool,
    significand: u64,
    exponent: i32,
}
const SPECIAL: i32 = i32::MAX;
impl From<f64> for Float {
    fn from(value: f64) -> Self {
        let bits = value.to_bits();
        let negative = bits >> 63 != 0;
        let e = ((bits >> 52) & 2047) as i32;
        let fraction = bits & ((1_u64 << 52) - 1);
        if e == 2047 {
            return Self {
                negative,
                significand: u64::from(fraction != 0),
                exponent: SPECIAL,
            };
        }
        let sig = fraction | if e == 0 { 0 } else { 1 << 52 };
        if sig == 0 {
            return Self::zero(negative);
        }
        let shift = sig.leading_zeros();
        Self {
            negative,
            significand: sig << shift,
            exponent: if e == 0 {
                -1011 - shift as i32
            } else {
                e - 1012 - shift as i32
            },
        }
    }
}
impl std::ops::Neg for Float {
    type Output = Self;
    fn neg(mut self) -> Self {
        self.negative = !self.negative;
        self
    }
}
impl Float {
    fn zero(negative: bool) -> Self {
        Self {
            negative,
            significand: 0,
            exponent: 0,
        }
    }
    fn infinity(negative: bool) -> Self {
        Self {
            negative,
            significand: 0,
            exponent: SPECIAL,
        }
    }
    fn nan() -> Self {
        Self {
            negative: true,
            significand: 1,
            exponent: SPECIAL,
        }
    }
    fn special(self) -> bool {
        self.exponent == SPECIAL
    }
    fn is_nan(self) -> bool {
        self.special() && self.significand != 0
    }
    fn is_zero(self) -> bool {
        !self.special() && self.significand == 0
    }
}
fn jam(value: u128, shift: u32) -> u128 {
    if shift == 0 {
        value
    } else if shift >= 128 {
        u128::from(value != 0)
    } else {
        (value >> shift) | u128::from(value << (128 - shift) != 0)
    }
}
impl Arithmetic {
    fn bits(self) -> u32 {
        match self.precision {
            Precision::Bits24 => 24,
            Precision::Bits53 => 53,
            Precision::Bits64 => 64,
        }
    }
    fn rounded(self, negative: bool, value: u128, shift: u32) -> u128 {
        if shift == 0 {
            return value;
        }
        let kept = if shift >= 128 { 0 } else { value >> shift };
        let rem = if shift >= 128 {
            value
        } else {
            value & ((1_u128 << shift) - 1)
        };
        let increment = match self.rounding {
            Rounding::NearestEven if shift <= 128 => {
                let half = 1_u128 << (shift - 1);
                rem > half || (rem == half && kept & 1 != 0)
            }
            Rounding::NearestEven | Rounding::TowardZero => false,
            Rounding::Down => negative && rem != 0,
            Rounding::Up => !negative && rem != 0,
        };
        kept + u128::from(increment)
    }
    fn finite(self, negative: bool, value: u128, exponent: i32) -> Float {
        if value == 0 {
            return Float::zero(negative);
        }
        let top = 127 - value.leading_zeros();
        let shift = (top + 1).saturating_sub(self.bits());
        let rounded = self.rounded(negative, value, shift);
        let top = 127 - rounded.leading_zeros();
        Float {
            negative,
            significand: if top > 63 {
                (rounded >> (top - 63)) as u64
            } else {
                (rounded << (63 - top)) as u64
            },
            exponent: exponent + shift as i32 + top as i32,
        }
    }
    pub(super) fn add(self, a: impl Into<Float>, b: impl Into<Float>) -> Result<Float, Error> {
        let (mut a, mut b) = (a.into(), b.into());
        if a.is_nan() || b.is_nan() || (a.special() && b.special() && a.negative != b.negative) {
            return Ok(Float::nan());
        }
        if a.special() || b.is_zero() && !a.is_zero() {
            return Ok(a);
        }
        if b.special() || a.is_zero() && !b.is_zero() {
            return Ok(b);
        }
        if a.is_zero() && b.is_zero() {
            return Ok(Float::zero(if a.negative == b.negative {
                a.negative
            } else {
                self.rounding == Rounding::Down
            }));
        }
        if (a.exponent, a.significand) < (b.exponent, b.significand) {
            std::mem::swap(&mut a, &mut b);
        }
        let left = u128::from(a.significand) << 3;
        let right = jam(u128::from(b.significand) << 3, (a.exponent - b.exponent) as u32);
        let value = if a.negative == b.negative {
            left + right
        } else {
            left - right
        };
        Ok(self.finite(
            if value == 0 {
                self.rounding == Rounding::Down
            } else {
                a.negative
            },
            value,
            a.exponent - 66,
        ))
    }
    pub(super) fn mul(self, a: impl Into<Float>, b: impl Into<Float>) -> Result<Float, Error> {
        let (a, b) = (a.into(), b.into());
        let negative = a.negative ^ b.negative;
        if a.is_nan() || b.is_nan() || (a.special() && b.is_zero()) || (b.special() && a.is_zero()) {
            return Ok(Float::nan());
        }
        if a.special() || b.special() {
            return Ok(Float::infinity(negative));
        }
        Ok(self.finite(
            negative,
            u128::from(a.significand) * u128::from(b.significand),
            a.exponent + b.exponent - 126,
        ))
    }
    pub(super) fn store(self, value: impl Into<Float>) -> Result<f32, Error> {
        let value = value.into();
        let sign = u32::from(value.negative) << 31;
        if value.special() {
            return Ok(f32::from_bits(
                sign | if value.is_nan() { 0x7fc0_0000 } else { 0x7f80_0000 },
            ));
        }
        if value.is_zero() {
            return Ok(f32::from_bits(sign));
        }
        // Stores always round to binary32, independently of precision control.
        let shift = 40 + (-126 - value.exponent).max(0) as u32;
        let mut sig = self.rounded(value.negative, u128::from(value.significand), shift);
        let mut exponent = value.exponent.max(-126);
        if sig >= 1 << 24 {
            sig >>= 1;
            exponent += 1;
        }
        if exponent > 127 {
            let infinity = match self.rounding {
                Rounding::NearestEven => true,
                Rounding::Down => value.negative,
                Rounding::Up => !value.negative,
                Rounding::TowardZero => false,
            };
            return Ok(f32::from_bits(sign | if infinity { 0x7f80_0000 } else { 0x7f7f_ffff }));
        }
        let field = if sig < 1 << 23 {
            0
        } else {
            ((exponent + 127) as u32) << 23
        };
        Ok(f32::from_bits(sign | field | (sig as u32 & 0x7f_ffff)))
    }
    pub(super) fn reciprocal(self, scale: f32) -> Result<Float, Error> {
        let scale = Float::from(f64::from(scale));
        if scale.is_nan() {
            return Ok(Float::nan());
        }
        if scale.special() {
            return Ok(Float::zero(scale.negative));
        }
        if scale.is_zero() {
            return Ok(Float::infinity(scale.negative));
        }
        // 67 quotient bits plus sticky; the denominator is a binary32 load.
        let denominator = u128::from(scale.significand >> 40);
        let numerator = 1_u128 << 90;
        let quotient = (numerator / denominator) | u128::from(numerator % denominator != 0);
        Ok(self.finite(scale.negative, quotient, -scale.exponent - 67))
    }
    pub(super) fn material_threshold(self, threshold: u8) -> Result<f32, Error> {
        self.store(self.mul(f64::from(threshold), f64::from_bits(0x3f70_1010_1010_1010))?)
    }
}
#[cfg(test)]
pub(super) fn add(a: impl Into<Float>, b: impl Into<Float>) -> Result<Float, Error> {
    Arithmetic::default().add(a, b)
}
#[cfg(test)]
pub(super) fn material_threshold(threshold: u8) -> f32 {
    Arithmetic::default()
        .material_threshold(threshold)
        .expect("no foreign call")
}
