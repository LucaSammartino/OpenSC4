// SPDX-License-Identifier: GPL-3.0-or-later
//! Safe exact dyadic carrier, not host trig or an assumed x87 mode. An inexact
//! transient/store propagates an unsupported result. Explicit PC/RC emulation
//! and hardware approximation/native numeric qualification remain AS6/AS7.
#[derive(Clone, Copy)]
pub(crate) struct X {
    n: i128,
    e: i32,
    negative_zero: bool,
    valid: bool,
}
impl X {
    pub(crate) fn f(value: f32) -> Self {
        let bits = value.to_bits();
        let exp = ((bits >> 23) & 255) as i32;
        let fraction = bits & 0x7fffff;
        let n = if exp == 0 { fraction } else { fraction | 0x800000 };
        Self {
            n: if bits >> 31 != 0 { -i128::from(n) } else { i128::from(n) },
            e: if exp == 0 { -149 } else { exp - 150 },
            negative_zero: bits >> 31 != 0,
            valid: exp != 255,
        }
        .normalized()
    }
    pub(crate) fn integer(value: i64) -> Self {
        Self {
            n: i128::from(value),
            e: 0,
            negative_zero: false,
            valid: true,
        }
        .normalized()
    }
    fn normalized(mut self) -> Self {
        if self.n != 0 {
            let zeros = self.n.unsigned_abs().trailing_zeros();
            self.n >>= zeros;
            self.e += zeros as i32;
        } else {
            self.e = 0;
        }
        // No incoming PC/RC is known. Only arithmetic exact even at PC24 is
        // accepted; no PC53/64 assumption can leak into an owner result.
        self.valid &= 128 - self.n.unsigned_abs().leading_zeros() <= 24;
        self
    }
    pub(crate) fn valid(self) -> bool {
        self.valid
    }
    fn invalid() -> Self {
        Self {
            valid: false,
            ..Self::integer(0)
        }
    }
    pub(crate) fn store(self) -> f32 {
        if !self.valid {
            return f32::NAN;
        }
        if self.n == 0 {
            return if self.negative_zero { -0. } else { 0. };
        }
        // Only exact binary32 stores are supported without a PC/RC contract.
        let magnitude = self.n.unsigned_abs();
        let bits = 128 - magnitude.leading_zeros();
        let top = self.e + bits as i32 - 1;
        if bits > 24 || !(-149..=127).contains(&top) || self.e < -149 {
            return f32::NAN;
        }
        let sign = if self.n < 0 { 0x80000000 } else { 0 };
        if top < -126 {
            f32::from_bits(sign | ((magnitude as u32) << (self.e + 149)))
        } else {
            let fraction = ((magnitude as u32) << (24 - bits)) & 0x7fffff;
            f32::from_bits(sign | (((top + 127) as u32) << 23) | fraction)
        }
    }
    pub(crate) fn b(self) -> Self {
        Self::f(self.store())
    }
    pub(crate) fn neg(mut self) -> Self {
        self.n = -self.n;
        self.negative_zero = !self.negative_zero;
        self
    }
    pub(crate) fn abs(self) -> Self {
        if self.n < 0 || (self.n == 0 && self.negative_zero) {
            self.neg()
        } else {
            self
        }
    }
    pub(crate) fn add(self, rhs: Self) -> Self {
        if !self.valid || !rhs.valid {
            return Self::invalid();
        }
        if self.n == 0 && rhs.n != 0 {
            return rhs;
        }
        if rhs.n == 0 && self.n != 0 {
            return self;
        }
        let e = self.e.min(rhs.e);
        let shift = |x: Self| {
            let s = (x.e - e) as u32;
            if s >= 126 || 128 - x.n.unsigned_abs().leading_zeros() + s > 126 {
                None
            } else {
                x.n.checked_mul(1_i128 << s)
            }
        };
        let Some(n) = shift(self).and_then(|a| shift(rhs).and_then(|b| a.checked_add(b))) else {
            return Self::invalid();
        };
        Self {
            n,
            e,
            negative_zero: self.negative_zero && rhs.negative_zero,
            valid: true,
        }
        .normalized()
    }
    pub(crate) fn sub(self, rhs: Self) -> Self {
        self.add(rhs.neg())
    }
    pub(crate) fn mul(self, rhs: Self) -> Self {
        let Some(n) = self.n.checked_mul(rhs.n) else {
            return Self::invalid();
        };
        Self {
            n,
            e: self.e + rhs.e,
            negative_zero: (self.n < 0 || self.negative_zero && self.n == 0)
                ^ (rhs.n < 0 || rhs.negative_zero && rhs.n == 0),
            valid: self.valid && rhs.valid,
        }
        .normalized()
    }
    pub(crate) fn div(self, rhs: Self) -> Self {
        if !self.valid || !rhs.valid || rhs.n == 0 || self.n % rhs.n != 0 {
            return Self::invalid();
        }
        Self {
            n: self.n / rhs.n,
            e: self.e - rhs.e,
            negative_zero: self.negative_zero ^ (rhs.n < 0),
            valid: true,
        }
        .normalized()
    }
    pub(crate) fn ge(self, rhs: Self) -> bool {
        if !self.valid || !rhs.valid {
            return false;
        }
        // Comparisons do not perform a rounded subtraction. f32 conversion
        // is exact for supported operands; zero signs compare equally.
        self.store() >= rhs.store()
    }
    pub(crate) fn trunc(self) -> i64 {
        if !self.valid {
            return i64::MIN;
        }
        let n = if self.e >= 0 {
            self.n.checked_mul(1_i128.checked_shl(self.e as u32).unwrap_or(0))
        } else {
            Some(if self.e <= -127 {
                0
            } else {
                self.n / (1_i128 << -self.e)
            })
        };
        n.and_then(|n| i64::try_from(n).ok()).unwrap_or(i64::MIN)
    }
    pub(crate) fn sqrt(self) -> Self {
        if !self.valid || self.n < 0 {
            return Self::invalid();
        }
        if self.n == 0 {
            return self;
        }
        let mut value = self;
        if value.e & 1 != 0 {
            value.n *= 2;
            value.e -= 1;
        }
        // Integer Newton iteration; accept perfect squares only.
        let n = value.n as u128;
        let mut root = 1_u128 << ((128 - n.leading_zeros() + 1) / 2);
        loop {
            let next = (root + n / root) / 2;
            if next >= root {
                break;
            }
            root = next;
        }
        if root * root != n {
            return Self::invalid();
        }
        Self {
            n: root as i128,
            e: value.e / 2,
            negative_zero: false,
            valid: true,
        }
        .normalized()
    }
    pub(crate) fn sin(self) -> Self {
        if self.valid && self.n == 0 {
            self
        } else {
            Self::invalid()
        }
    }
    pub(crate) fn cos(self) -> Self {
        if self.valid && self.n == 0 {
            Self::integer(1)
        } else {
            Self::invalid()
        }
    }
}
