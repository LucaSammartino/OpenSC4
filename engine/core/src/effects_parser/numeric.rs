// SPDX-License-Identifier: GPL-3.0-or-later
//! Canonical x87 operands and masked arithmetic for effects-parser §14.
//! Integer arithmetic keeps the raw reply separate from every arithmetic/store
//! boundary. Exception status/delivery belongs to the foreign runtime.
use super::{NumericPolicy, RawFloat, Unsupported};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rounding {
    Nearest,
    Down,
    Up,
    Zero,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Precision {
    P24,
    P53,
    P64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Control {
    pub precision: Precision,
    pub rounding: Rounding,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Extended {
    pub sign_exponent: u16,
    pub significand: u64,
}
impl Extended {
    pub fn binary64(bits: u64) -> Self {
        let negative = bits >> 63 != 0;
        let exponent = ((bits >> 52) & 2047) as i32;
        let fraction = bits & 0xfffffffffffff;
        if exponent == 2047 {
            Self::new(
                (if negative { 0x8000 } else { 0 }) | 0x7fff,
                (1 << 63) | (fraction << 11),
            )
        } else {
            pack_exact(
                negative,
                u128::from(fraction | if exponent == 0 { 0 } else { 1 << 52 }),
                if exponent == 0 { -1074 } else { exponent - 1075 },
            )
        }
    }
    pub fn positive(self) -> bool {
        !self.negative() && !self.nan() && self.significand != 0
    }
    pub fn reciprocal(self, control: Control) -> Self {
        if self.special() {
            return if self.nan() {
                self.quiet()
            } else {
                Self::new(self.sign_exponent & 0x8000, 0)
            };
        }
        if self.significand == 0 {
            return Self::new((self.sign_exponent & 0x8000) | 0x7fff, 1 << 63);
        }
        let dividend = 1u128 << 127;
        let shift = self.significand.leading_zeros();
        let divisor = u128::from(self.significand) << shift;
        // Keep three actual fractional guard bits and a separate sticky bit.
        // Treating every nonzero remainder as exactly half a unit would lose
        // the nearest-even decision at PC64, before the binary32 store.
        let remainder = (dividend % divisor) << 3;
        let quotient =
            ((dividend / divisor) << 4) | ((remainder / divisor) << 1) | u128::from(remainder % divisor != 0);
        finish(self.negative(), quotient, -self.power() + shift as i32 - 131, control)
    }
    pub const fn new(sign_exponent: u16, significand: u64) -> Self {
        Self {
            sign_exponent,
            significand,
        }
    }
    fn negative(self) -> bool {
        self.sign_exponent & 0x8000 != 0
    }
    fn exponent(self) -> u16 {
        self.sign_exponent & 0x7fff
    }
    fn special(self) -> bool {
        self.exponent() == 0x7fff
    }
    fn nan(self) -> bool {
        self.special() && self.significand != 1 << 63
    }
    pub fn is_one(self) -> bool {
        self == Self::new(0x3fff, 1 << 63)
    }
    pub fn ordered_equal(self, other: Self) -> bool {
        !self.nan() && !other.nan() && (self == other || (self.significand == 0 && other.significand == 0))
    }
    fn power(self) -> i32 {
        i32::from(self.exponent().max(1)) - 16383 - 63
    }
    pub fn input(raw: RawFloat) -> Result<Self, Unsupported> {
        let value = match raw {
            RawFloat::Extended(x) => x,
            RawFloat::Dyadic(c, e) => {
                let n = c.unsigned_abs();
                if n != 0
                    && (31 - n.leading_zeros() as i32 + i32::from(e) > 16383
                        || n.trailing_zeros() as i32 + i32::from(e) < -16445)
                {
                    return Err(Unsupported::Numeric);
                }
                return Ok(pack_exact(c < 0, u128::from(n), i32::from(e)));
            }
            RawFloat::Binary32(bits) => {
                let sign = ((bits >> 16) & 0x8000) as u16;
                let exponent = (bits >> 23) & 255;
                let fraction = bits & 0x7fffff;
                if exponent == 255 {
                    Self::new(sign | 0x7fff, (1 << 63) | (u64::from(fraction) << 40))
                } else {
                    return Ok(if exponent == 0 {
                        pack_exact(sign != 0, u128::from(fraction), -149)
                    } else {
                        pack_exact(sign != 0, u128::from(fraction | 0x800000), exponent as i32 - 150)
                    });
                }
            }
            RawFloat::Unsupported => return Err(Unsupported::Numeric),
        };
        if (value.exponent() == 0) != (value.significand >> 63 == 0) {
            return Err(Unsupported::Numeric);
        }
        Ok(value)
    }
    pub fn store(self, control: Control) -> u32 {
        let sign = u32::from(self.negative()) << 31;
        if self.special() {
            return sign
                | 0x7f800000
                | if self.nan() {
                    ((self.significand >> 40) as u32 & 0x7fffff) | 0x400000
                } else {
                    0
                };
        }
        if self.significand == 0 {
            return sign;
        }
        let top = 63 - self.significand.leading_zeros() as i32 + self.power();
        let target = (top - 23).max(-149);
        let mut n = rounded(
            u128::from(self.significand),
            target - self.power(),
            self.negative(),
            control.rounding,
        );
        let mut exponent = target + 23;
        if n >= 1 << 24 {
            n >>= 1;
            exponent += 1;
        }
        if exponent > 127 {
            let infinity = match control.rounding {
                Rounding::Nearest => true,
                Rounding::Up => !self.negative(),
                Rounding::Down => self.negative(),
                Rounding::Zero => false,
            };
            return sign | if infinity { 0x7f800000 } else { 0x7f7fffff };
        }
        sign | if n < 1 << 23 {
            n as u32
        } else {
            (((exponent + 127) as u32) << 23) | (n as u32 & 0x7fffff)
        }
    }
    pub fn mul_uint(self, factor: u32, control: Control) -> Self {
        if self.special() {
            return self.quiet();
        }
        finish(
            self.negative(),
            u128::from(self.significand) * u128::from(factor),
            self.power(),
            control,
        )
    }
    pub fn add(self, other: Self, control: Control) -> Self {
        if self.nan() {
            return self.quiet();
        }
        if other.nan() {
            return other.quiet();
        }
        if self.special() || other.special() {
            if self.special() && other.special() && self.negative() != other.negative() {
                return Self::new(0xffff, 0xc000000000000000);
            }
            return if self.special() { self } else { other };
        }
        if self.significand == 0 && other.significand == 0 {
            return Self::new(
                if self.negative() == other.negative() {
                    self.sign_exponent & 0x8000
                } else if control.rounding == Rounding::Down {
                    0x8000
                } else {
                    0
                },
                0,
            );
        }
        // Three guard bits plus jam retain all rounding information even when
        // the exponents differ by thousands. Cancellation is exact when close.
        let power = self.power().max(other.power());
        let a = jam(u128::from(self.significand) << 3, power - self.power());
        let b = jam(u128::from(other.significand) << 3, power - other.power());
        let (negative, magnitude) = if self.negative() == other.negative() {
            (self.negative(), a + b)
        } else if a > b {
            (self.negative(), a - b)
        } else if b > a {
            (other.negative(), b - a)
        } else {
            (control.rounding == Rounding::Down, 0)
        };
        finish(negative, magnitude, power - 3, control)
    }
    fn quiet(self) -> Self {
        if self.nan() {
            Self::new(self.sign_exponent, self.significand | 1 << 62)
        } else {
            self
        }
    }
}
fn jam(n: u128, shift: i32) -> u128 {
    if shift == 0 {
        n
    } else if shift >= 128 {
        u128::from(n != 0)
    } else {
        (n >> shift) | u128::from(n & ((1u128 << shift) - 1) != 0)
    }
}
fn rounded(n: u128, shift: i32, negative: bool, mode: Rounding) -> u128 {
    if shift <= 0 {
        return n << -shift;
    }
    let q = if shift >= 128 { 0 } else { n >> shift };
    let remainder = if shift >= 128 { n } else { n & ((1u128 << shift) - 1) };
    let increment = match mode {
        Rounding::Nearest if shift <= 128 => {
            let half = 1u128 << (shift - 1);
            remainder > half || (remainder == half && q & 1 != 0)
        }
        Rounding::Nearest | Rounding::Zero => false,
        Rounding::Down => negative && remainder != 0,
        Rounding::Up => !negative && remainder != 0,
    };
    q + u128::from(increment)
}
fn pack_exact(negative: bool, n: u128, power: i32) -> Extended {
    if n == 0 {
        return Extended::new(if negative { 0x8000 } else { 0 }, 0);
    }
    let top = 127 - n.leading_zeros() as i32;
    let exponent = top + power + 16383;
    let significand = if top <= 63 { n << (63 - top) } else { n >> (top - 63) };
    if exponent <= 0 {
        Extended::new(
            if negative { 0x8000 } else { 0 },
            (significand >> (1 - exponent).min(127)) as u64,
        )
    } else {
        Extended::new(
            (if negative { 0x8000 } else { 0 }) | exponent as u16,
            significand as u64,
        )
    }
}
fn finish(negative: bool, n: u128, power: i32, control: Control) -> Extended {
    if n == 0 {
        return pack_exact(negative, 0, 0);
    }
    let precision = match control.precision {
        Precision::P24 => 24,
        Precision::P53 => 53,
        Precision::P64 => 64,
    };
    let top = 127 - n.leading_zeros() as i32;
    let target = (top + 1 - precision + power).max(-16381 - precision);
    let q = rounded(n, target - power, negative, control.rounding);
    if q != 0 && 127 - q.leading_zeros() as i32 + target > 16383 {
        let infinity = match control.rounding {
            Rounding::Nearest => true,
            Rounding::Up => !negative,
            Rounding::Down => negative,
            Rounding::Zero => false,
        };
        return Extended::new(
            (if negative { 0x8000 } else { 0 }) | if infinity { 0x7fff } else { 0x7ffe },
            if infinity {
                1 << 63
            } else {
                u64::MAX << (64 - precision)
            },
        );
    }
    pack_exact(negative, q, target)
}
pub fn control(policy: NumericPolicy) -> Result<Control, Unsupported> {
    match policy {
        NumericPolicy::NearestAtLeast53 => Ok(Control {
            precision: Precision::P53,
            rounding: Rounding::Nearest,
        }),
        NumericPolicy::X87(control) => Ok(control),
        NumericPolicy::Unqualified => Err(Unsupported::Numeric),
    }
}
pub fn probability(raw: RawFloat, policy: NumericPolicy) -> Result<(Extended, Extended, u32), Unsupported> {
    let c = control(policy)?;
    let x = Extended::input(raw)?;
    let m = x.mul_uint(65535, c);
    let a = m.add(Extended::input(RawFloat::Dyadic(12582912, 0))?, c);
    Ok((m, a, a.store(c)))
}

#[cfg(test)]
mod tests;
