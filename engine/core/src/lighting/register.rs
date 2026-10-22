// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable register arithmetic for the explicit stores in lighting.md §5.8.
//! No host f64 accumulator, reassociation, or fused operations. Masked exceptions
//! and nearest-even rounding are explicit preconditions, not a guessed game CW.
use std::cmp::Ordering;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extended_register_retains_bit_lost_by_binary64_and_spills_only_at_q() {
        for (precision, expected) in [(RegisterPrecision::Bits53, 0.0), (RegisterPrecision::Bits64, 1.0)] {
            let large = Number::double(9223372036854775808.0, precision);
            let one = Number::new(1.0, precision);
            assert_eq!(((large + one) - large).store(), expected);
            assert_eq!(((large + one).q() - large).store(), 0.0);
        }
    }
    #[test]
    fn pc53_primitives_match_independent_ieee_binary64_operations() {
        let mut seed = 0xa1234567_u32;
        for _ in 0..4000 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let a = f32::from_bits(seed & 0xfeffffff);
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let b = f32::from_bits(seed & 0xfeffffff);
            let x = Number::new(a, RegisterPrecision::Bits53);
            let y = Number::new(b, RegisterPrecision::Bits53);
            for (actual, expected) in [
                ((x + y).store(), (f64::from(a) + f64::from(b)) as f32),
                ((x - y).store(), (f64::from(a) - f64::from(b)) as f32),
                ((x * y).store(), (f64::from(a) * f64::from(b)) as f32),
                ((x / y).store(), (f64::from(a) / f64::from(b)) as f32),
            ] {
                assert!(
                    actual.to_bits() == expected.to_bits() || (actual.is_nan() && expected.is_nan()),
                    "{a} {b}: {actual} vs {expected}"
                );
            }
            if a >= 0.0 {
                assert_eq!(x.sqrt().store().to_bits(), (f64::from(a).sqrt() as f32).to_bits());
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum RegisterPrecision {
    Bits24,
    Bits53,
    Bits64,
}
impl RegisterPrecision {
    fn bits(self) -> u32 {
        match self {
            Self::Bits24 => 24,
            Self::Bits53 => 53,
            Self::Bits64 => 64,
        }
    }
}

/// Normalized binary register: sign * significand * 2^(exponent-63).
/// Exponent MAX denotes infinity (significand 0) or NaN (significand 1).
#[derive(Clone, Copy, Debug)]
pub(super) struct Reg {
    negative: bool,
    significand: u64,
    exponent: i32,
}
impl Reg {
    const ZERO: Self = Self {
        negative: false,
        significand: 0,
        exponent: 0,
    };
    fn special(self) -> bool {
        self.exponent == i32::MAX
    }
    fn nan(self) -> bool {
        self.special() && self.significand != 0
    }
    fn zero(self) -> bool {
        !self.special() && self.significand == 0
    }
    fn exceptional(negative: bool, nan: bool) -> Self {
        // LT13: canonical quiet NaNs are a portable policy. The public contract
        // does not give x87 NaN payload/sign selection or signaling-NaN effects.
        Self {
            negative,
            significand: u64::from(nan),
            exponent: i32::MAX,
        }
    }
    pub(super) fn neg(mut self) -> Self {
        self.negative = !self.negative;
        self
    }
    pub(super) fn load(value: f32) -> Self {
        Self::double(f64::from(value))
    }
    pub(super) fn double(value: f64) -> Self {
        let bits = value.to_bits();
        let negative = bits >> 63 != 0;
        let exponent = ((bits >> 52) & 2047) as i32;
        let fraction = bits & ((1_u64 << 52) - 1);
        if exponent == 2047 {
            return Self::exceptional(negative, fraction != 0);
        }
        let sig = fraction | if exponent == 0 { 0 } else { 1 << 52 };
        Self::rounded(
            negative,
            u128::from(sig),
            if exponent == 0 { -1074 } else { exponent - 1075 },
            false,
            64,
        )
    }
    fn rounded(negative: bool, value: u128, power: i32, sticky: bool, precision: u32) -> Self {
        if value == 0 {
            return Self { negative, ..Self::ZERO };
        }
        let length = 128 - value.leading_zeros();
        let shift = length.saturating_sub(precision);
        let sig = round_shift(value, shift, sticky);
        let length2 = 128 - sig.leading_zeros();
        let exponent = power + shift as i32 + length2 as i32 - 1;
        let significand = if length2 > 64 {
            (sig >> 1) as u64
        } else {
            (sig << (64 - length2)) as u64
        };
        Self {
            negative,
            significand,
            exponent,
        }
    }
    pub(super) fn store(self) -> f32 {
        let sign = u32::from(self.negative) << 31;
        if self.special() {
            return f32::from_bits(sign | 0x7f800000 | if self.nan() { 0x400000 } else { 0 });
        }
        if self.zero() {
            return f32::from_bits(sign);
        }
        let mut exponent = self.exponent;
        if exponent < -126 {
            return f32::from_bits(
                sign | round_shift(u128::from(self.significand), (-exponent - 86) as u32, false) as u32,
            );
        }
        let mut sig = round_shift(u128::from(self.significand), 40, false) as u32;
        if sig == 1 << 24 {
            sig >>= 1;
            exponent += 1;
        }
        if exponent > 127 {
            return f32::from_bits(sign | 0x7f800000);
        }
        f32::from_bits(sign | ((exponent + 127) as u32) << 23 | (sig & 0x7fffff))
    }
    pub(super) fn spill(self) -> Self {
        Self::load(self.store())
    }
    pub(super) fn compare(self, other: Self) -> Option<Ordering> {
        if self.nan() || other.nan() {
            return None;
        }
        if self.zero() && other.zero() {
            return Some(Ordering::Equal);
        }
        if self.negative != other.negative {
            return Some(if self.negative {
                Ordering::Less
            } else {
                Ordering::Greater
            });
        }
        let cmp = (self.exponent, self.significand).cmp(&(other.exponent, other.significand));
        // Zero has no meaningful exponent.
        let cmp = if self.zero() {
            Ordering::Less
        } else if other.zero() {
            Ordering::Greater
        } else {
            cmp
        };
        Some(if self.negative { cmp.reverse() } else { cmp })
    }
    pub(super) fn add(self, other: Self, p: RegisterPrecision) -> Self {
        if self.nan() || other.nan() {
            return Self::exceptional(false, true);
        }
        if self.special() || other.special() {
            if self.special() && other.special() && self.negative != other.negative {
                return Self::exceptional(false, true);
            }
            return if self.special() { self } else { other };
        }
        if self.zero() && other.zero() {
            return Self {
                negative: self.negative && other.negative,
                ..Self::ZERO
            };
        }
        if self.zero() {
            return Self::rounded(
                other.negative,
                other.significand.into(),
                other.exponent - 63,
                false,
                p.bits(),
            );
        }
        if other.zero() {
            return Self::rounded(
                self.negative,
                self.significand.into(),
                self.exponent - 63,
                false,
                p.bits(),
            );
        }
        let (large, small) = if (self.exponent, self.significand) >= (other.exponent, other.significand) {
            (self, other)
        } else {
            (other, self)
        };
        let left = u128::from(large.significand) << 3;
        let right = shift_jam(
            u128::from(small.significand) << 3,
            (large.exponent - small.exponent) as u32,
        );
        let value = if large.negative == small.negative {
            left + right
        } else {
            left - right
        };
        Self::rounded(
            large.negative && value != 0,
            value,
            large.exponent - 66,
            false,
            p.bits(),
        )
    }
    pub(super) fn sub(self, other: Self, p: RegisterPrecision) -> Self {
        self.add(other.neg(), p)
    }
    pub(super) fn mul(self, other: Self, p: RegisterPrecision) -> Self {
        let sign = self.negative != other.negative;
        if self.nan() || other.nan() || (self.special() && other.zero()) || (other.special() && self.zero()) {
            return Self::exceptional(sign, true);
        }
        if self.special() || other.special() {
            return Self::exceptional(sign, false);
        }
        Self::rounded(
            sign,
            u128::from(self.significand) * u128::from(other.significand),
            self.exponent + other.exponent - 126,
            false,
            p.bits(),
        )
    }
    pub(super) fn div(self, other: Self, p: RegisterPrecision) -> Self {
        let sign = self.negative != other.negative;
        if self.nan() || other.nan() || (self.special() && other.special()) || (self.zero() && other.zero()) {
            return Self::exceptional(sign, true);
        }
        if self.special() || other.zero() {
            return Self::exceptional(sign, false);
        }
        if other.special() || self.zero() {
            return Self {
                negative: sign,
                ..Self::ZERO
            };
        }
        let numerator = u128::from(self.significand) << 64;
        let denominator = u128::from(other.significand);
        let mut quotient = numerator / denominator;
        let remainder = numerator % denominator;
        if p.bits() == 64 && quotient < 1_u128 << 64 {
            quotient += u128::from(remainder * 2 > denominator || (remainder * 2 == denominator && quotient & 1 != 0));
            return Self::rounded(sign, quotient, self.exponent - other.exponent - 64, false, 64);
        }
        Self::rounded(
            sign,
            quotient,
            self.exponent - other.exponent - 64,
            remainder != 0,
            p.bits(),
        )
    }
    pub(super) fn sqrt(self, p: RegisterPrecision) -> Self {
        if self.nan() || (self.negative && !self.zero()) {
            return Self::exceptional(false, true);
        }
        if self.special() || self.zero() {
            return self;
        }
        let power = self.exponent.div_euclid(2);
        let number = u128::from(self.significand) << (63 + self.exponent.rem_euclid(2));
        let mut remainder = number;
        let mut root = 0_u128;
        let mut bit = 1_u128 << 126;
        while bit != 0 {
            if remainder >= root + bit {
                remainder -= root + bit;
                root = (root >> 1) + bit;
            } else {
                root >>= 1;
            }
            bit >>= 2;
        }
        if p.bits() == 64 {
            // sqrt(number) >= root + 1/2 iff remainder > root.
            Self::rounded(false, root + u128::from(remainder > root), power - 63, false, 64)
        } else {
            Self::rounded(false, root, power - 63, remainder != 0, p.bits())
        }
    }
}
fn round_shift(value: u128, shift: u32, sticky: bool) -> u128 {
    if shift == 0 {
        return value;
    }
    if shift > 128 {
        return 0;
    }
    let half = 1_u128 << (shift - 1);
    let lower = value & (half - 1);
    let top = if shift == 128 { 0 } else { value >> shift };
    top + u128::from(value & half != 0 && (lower != 0 || sticky || top & 1 != 0))
}
fn shift_jam(value: u128, shift: u32) -> u128 {
    if shift == 0 {
        value
    } else if shift >= 128 {
        u128::from(value != 0)
    } else {
        (value >> shift) | u128::from(value & ((1_u128 << shift) - 1) != 0)
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Number {
    value: Reg,
    precision: RegisterPrecision,
}
impl Number {
    pub(super) fn new(value: f32, precision: RegisterPrecision) -> Self {
        Self {
            value: Reg::load(value),
            precision,
        }
    }
    pub(super) fn double(value: f64, precision: RegisterPrecision) -> Self {
        Self {
            value: Reg::double(value),
            precision,
        }
    }
    pub(super) fn precision(self) -> RegisterPrecision {
        self.precision
    }
    pub(super) fn store(self) -> f32 {
        self.value.store()
    }
    pub(super) fn q(self) -> Self {
        Self {
            value: self.value.spill(),
            ..self
        }
    }
    pub(super) fn sqrt(self) -> Self {
        Self {
            value: self.value.sqrt(self.precision),
            ..self
        }
    }
}
impl PartialEq for Number {
    fn eq(&self, rhs: &Self) -> bool {
        self.value.compare(rhs.value) == Some(Ordering::Equal)
    }
}
impl PartialOrd for Number {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        self.value.compare(rhs.value)
    }
}
macro_rules! operator {
    ($trait:ident, $method:ident) => {
        impl std::ops::$trait for Number {
            type Output = Self;
            fn $method(self, rhs: Self) -> Self {
                Self {
                    value: self.value.$method(rhs.value, self.precision),
                    ..self
                }
            }
        }
    };
}
operator!(Add, add);
operator!(Sub, sub);
operator!(Mul, mul);
operator!(Div, div);
impl std::ops::Neg for Number {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            value: self.value.neg(),
            ..self
        }
    }
}
