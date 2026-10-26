// SPDX-License-Identifier: GPL-3.0-or-later
//! Finite binary80 operations for minimap §5.3. Reuse the shared x87 add/store.
use super::{Result, Unsupported};
use crate::effects_parser::numeric::{Control, Extended, Precision, Rounding};
use std::cmp::Ordering;

const CONTROL: Control = Control {
    precision: Precision::P64,
    rounding: Rounding::Nearest,
};

#[derive(Clone, Copy)]
pub(super) struct E(Extended);

impl E {
    pub fn input(x: f32) -> Result<Self> {
        if !x.is_finite() {
            return Err(Unsupported::Precondition("finite terrain values"));
        }
        Ok(Self::constant(f64::from(x)))
    }

    pub fn constant(x: f64) -> Self {
        Self(Extended::binary64(x.to_bits()))
    }
    fn negative(self) -> bool {
        self.0.sign_exponent & 0x8000 != 0
    }
    fn exponent(self) -> i32 {
        i32::from(self.0.sign_exponent & 0x7fff) - 16383
    }
    fn pack(raw: u128, power: i32, negative: bool) -> Self {
        if raw == 0 {
            return Self::constant(if negative { -0.0 } else { 0.0 });
        }
        let k = 127 - raw.leading_zeros() as i32;
        let mut significand = if k <= 63 {
            raw << (63 - k)
        } else {
            let shift = (k - 63) as u32;
            let q = raw >> shift;
            let rem = raw & ((1u128 << shift) - 1);
            let half = 1u128 << (shift - 1);
            q + u128::from(rem > half || (rem == half && q & 1 != 0))
        };
        let mut exponent = power + k;
        if significand > u128::from(u64::MAX) {
            significand >>= 1;
            exponent += 1;
        }
        Self(Extended {
            sign_exponent: ((exponent + 16383) as u16) | if negative { 0x8000 } else { 0 },
            significand: significand as u64,
        })
    }
    pub fn add(self, b: Self) -> Self {
        Self(self.0.add(b.0, CONTROL))
    }
    pub fn neg(mut self) -> Self {
        self.0.sign_exponent ^= 0x8000;
        self
    }
    pub fn sub(self, b: Self) -> Self {
        self.add(b.neg())
    }
    pub fn abs(mut self) -> Self {
        self.0.sign_exponent &= 0x7fff;
        self
    }
    pub fn mul(self, b: Self) -> Self {
        Self::pack(
            u128::from(self.0.significand) * u128::from(b.0.significand),
            self.exponent() + b.exponent() - 126,
            self.negative() ^ b.negative(),
        )
    }
    pub fn div(self, b: Self) -> Self {
        assert_ne!(b.0.significand, 0);
        let shift = if self.0.significand >= b.0.significand { 63 } else { 64 };
        let raw = u128::from(self.0.significand) << shift;
        let divisor = u128::from(b.0.significand);
        let q = raw / divisor;
        let rem = raw % divisor;
        let q = q + u128::from(rem * 2 > divisor || (rem * 2 == divisor && q & 1 != 0));
        Self::pack(
            q,
            self.exponent() - b.exponent() - shift,
            self.negative() ^ b.negative(),
        )
    }
    pub fn sqrt(self) -> Self {
        assert!(!self.negative());
        if self.0.significand == 0 {
            return self;
        }
        let e = self.exponent();
        let mut remainder = u128::from(self.0.significand) << (63 + (e & 1));
        let mut root = 0u128;
        let mut bit = 1u128 << 126;
        while bit > remainder {
            bit >>= 2;
        }
        while bit != 0 {
            if remainder >= root + bit {
                remainder -= root + bit;
                root = (root >> 1) + bit;
            } else {
                root >>= 1;
            }
            bit >>= 2;
        }
        if remainder > root {
            root += 1;
        }
        Self::pack(root, (e >> 1) - 63, false)
    }
    pub fn cmp(self, b: Self) -> Ordering {
        if self.0.significand == 0 && b.0.significand == 0 {
            return Ordering::Equal;
        }
        if self.negative() != b.negative() {
            return if self.negative() {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        let a_mag = if self.0.significand == 0 {
            (i32::MIN, 0)
        } else {
            (self.exponent(), self.0.significand)
        };
        let b_mag = if b.0.significand == 0 {
            (i32::MIN, 0)
        } else {
            (b.exponent(), b.0.significand)
        };
        let order = a_mag.cmp(&b_mag);
        if self.negative() {
            order.reverse()
        } else {
            order
        }
    }
    pub fn rhe(self) -> i32 {
        self.add(Self::constant(12_582_912.0))
            .0
            .store(CONTROL)
            .wrapping_sub(0x4b40_0000) as i32
    }
    pub fn single(self) -> f32 {
        f32::from_bits(self.0.store(CONTROL))
    }
    pub fn floor(self) -> i32 {
        let k = self.sub(Self::constant(0.5)).rhe();
        if self.cmp(Self::constant(f64::from(k) + 1.0)) == Ordering::Equal {
            k.wrapping_add(1)
        } else {
            k
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn extended_products_cancellation_and_rounding_stores() {
        let a = E::constant(16_777_217.0);
        let b = E::constant(16_777_215.0);
        assert_eq!(a.mul(b).sub(E::constant(281_474_976_710_656.0)).rhe(), -1);
        assert_eq!(E::constant(9.0).sqrt().rhe(), 3);
        assert_eq!(E::constant(2.0).sqrt().mul(E::constant(2.0).sqrt()).rhe(), 2);
        assert_eq!(
            E::constant(1.0).div(E::constant(4.0)).cmp(E::constant(0.25)),
            Ordering::Equal
        );
        for (v, expected) in [(2.0, 2), (2.5, 2), (3.5, 4), (-2.5, -2)] {
            assert_eq!(E::constant(v).rhe(), expected);
        }
        for (v, expected) in [(2.0, 2), (2.5, 2), (-2.5, -3), (-2.0, -2)] {
            assert_eq!(E::constant(v).floor(), expected);
        }
    }
}
