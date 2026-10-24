// SPDX-License-Identifier: GPL-3.0-or-later
//! QV14's finite binary arithmetic, with explicit caller PC/RC, independent of host FP.
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RingPrecision {
    #[default]
    Bits24,
    Bits53,
    Bits64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RingRounding {
    #[default]
    Nearest,
    Down,
    Up,
    TowardZero,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RingContext {
    pub precision: RingPrecision,
    pub rounding: RingRounding,
}

/// Exact signed dyadic value; each arithmetic result has at most PC significant bits.
#[derive(Clone, Copy, Debug)]
pub(super) struct F {
    sig: u64,
    exp: i32,
    neg: bool,
}
impl F {
    pub fn float(v: f32) -> Self {
        let b = v.to_bits();
        let e = ((b >> 23) & 255) as i32;
        Self {
            sig: u64::from((b & 0x7fffff) | if e == 0 { 0 } else { 1 << 23 }),
            exp: if e == 0 { -149 } else { e - 150 },
            neg: b >> 31 != 0,
        }
    }
    pub fn int(v: i32) -> Self {
        Self {
            sig: i64::from(v).unsigned_abs(),
            exp: 0,
            neg: v < 0,
        }
    }
    pub fn abs(mut self) -> Self {
        self.neg = false;
        self
    }
    pub fn negative(self) -> bool {
        self.neg && self.sig != 0
    }
    pub fn cmp(self, rhs: Self) -> Ordering {
        if self.sig == 0 && rhs.sig == 0 {
            return Ordering::Equal;
        }
        if self.negative() != rhs.negative() {
            return if self.negative() {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        let bits = |v: Self| 64 - v.sig.leading_zeros() as i32;
        let order = if self.sig == 0 {
            Ordering::Less
        } else if rhs.sig == 0 {
            Ordering::Greater
        } else {
            let a = bits(self);
            let b = bits(rhs);
            (self.exp + a).cmp(&(rhs.exp + b)).then_with(|| {
                let n = a.max(b);
                (u128::from(self.sig) << (n - a)).cmp(&(u128::from(rhs.sig) << (n - b)))
            })
        };
        if self.negative() {
            order.reverse()
        } else {
            order
        }
    }
}

impl RingContext {
    fn bits(self) -> u32 {
        match self.precision {
            RingPrecision::Bits24 => 24,
            RingPrecision::Bits53 => 53,
            RingPrecision::Bits64 => 64,
        }
    }
    fn round(self, raw: u128, exp: i32, neg: bool, sticky: bool, precision: u32) -> F {
        let bits = 128 - raw.leading_zeros();
        let shift = bits.saturating_sub(precision);
        let (mut sig, lost, half) = if shift == 0 {
            (raw, false, Ordering::Less)
        } else {
            let mask = (1u128 << shift) - 1;
            let remainder = raw & mask;
            (raw >> shift, remainder != 0, remainder.cmp(&(1u128 << (shift - 1))))
        };
        let discarded = lost || sticky;
        let increment = match self.rounding {
            RingRounding::Nearest => half == Ordering::Greater || (half == Ordering::Equal && (sticky || sig & 1 != 0)),
            RingRounding::Down => neg && discarded,
            RingRounding::Up => !neg && discarded,
            RingRounding::TowardZero => false,
        };
        sig += u128::from(increment);
        let carry = u32::from(sig > u128::from(u64::MAX));
        F {
            sig: (sig >> carry) as u64,
            exp: exp + shift as i32 + carry as i32,
            neg,
        }
    }
    pub(super) fn add(self, a: F, b: F) -> F {
        if a.sig == 0 {
            return self.round(u128::from(b.sig), b.exp, b.neg, false, self.bits());
        }
        if b.sig == 0 {
            return self.round(u128::from(a.sig), a.exp, a.neg, false, self.bits());
        }
        let top = |v: F| v.exp + 64 - v.sig.leading_zeros() as i32;
        let exp = top(a).max(top(b)) - 100;
        let units = |v: F| {
            let shift = v.exp - exp;
            if shift >= 0 {
                u128::from(v.sig) << shift
            } else {
                let s = (-shift) as u32;
                if s >= 64 {
                    1
                } else {
                    u128::from(v.sig >> s) | u128::from(v.sig & ((1u64 << s) - 1) != 0)
                }
            }
        };
        let (x, y) = (units(a), units(b));
        let (raw, neg) = if a.neg == b.neg {
            (x + y, a.neg)
        } else if x >= y {
            (x - y, a.neg)
        } else {
            (y - x, b.neg)
        };
        self.round(raw, exp, neg, false, self.bits())
    }
    pub(super) fn sub(self, a: F, mut b: F) -> F {
        b.neg = !b.neg;
        self.add(a, b)
    }
    pub(super) fn mul(self, a: F, b: F) -> F {
        self.round(
            u128::from(a.sig) * u128::from(b.sig),
            a.exp + b.exp,
            a.neg ^ b.neg,
            false,
            self.bits(),
        )
    }
    pub(super) fn sqrt(self, v: F) -> F {
        if v.sig == 0 {
            return v;
        }
        let mut sig = u128::from(v.sig);
        let mut exp = v.exp;
        if exp & 1 != 0 {
            sig <<= 1;
            exp -= 1;
        }
        let shift = sig.leading_zeros() / 2 * 2;
        let n = sig << shift;
        let (mut lo, mut hi) = (0u128, 1u128 << 64);
        while lo + 1 < hi {
            let mid = (lo + hi) / 2;
            if mid <= n / mid {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        let rem = n - lo * lo;
        let exp = exp / 2 - shift as i32 / 2;
        if self.bits() == 64 {
            let increment = match self.rounding {
                RingRounding::Nearest => rem > lo,
                RingRounding::Up => rem != 0,
                _ => false,
            };
            self.round(lo + u128::from(increment), exp, false, false, 64)
        } else {
            self.round(lo, exp, false, rem != 0, self.bits())
        }
    }
    pub(super) fn store(self, v: F) -> F {
        let rounded = self.round(u128::from(v.sig), v.exp, v.neg, false, 24);
        // Ordinary bitmap geometry fits binary32's exponent range (QV14 boundary).
        let value = (rounded.sig as f64) * 2f64.powi(rounded.exp);
        F::float(if rounded.neg { -(value as f32) } else { value as f32 })
    }
    pub(super) fn integer(self, z: F, ceil: bool) -> i32 {
        let bias = F::float(if ceil { 0.5 } else { -0.5 });
        let value = self.store(self.add(self.add(z, bias), F::int(12582912)));
        let f = (value.sig as f64) * 2f64.powi(value.exp) * if value.neg { -1.0 } else { 1.0 };
        let n = (f as f32).to_bits().wrapping_add(0xb4c00000) as i32;
        let corrected = if ceil { n.wrapping_sub(1) } else { n.wrapping_add(1) };
        if z.cmp(F::int(corrected)) == Ordering::Equal {
            corrected
        } else {
            n
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inherited_precision_and_rounding_apply_to_signed_halfway_sums() {
        for (precision, bits) in [
            (RingPrecision::Bits24, 24),
            (RingPrecision::Bits53, 53),
            (RingPrecision::Bits64, 64),
        ] {
            for rounding in [
                RingRounding::Nearest,
                RingRounding::Down,
                RingRounding::Up,
                RingRounding::TowardZero,
            ] {
                let context = RingContext { precision, rounding };
                for neg in [false, true] {
                    // At magnitude one, 2^-PC is exactly half an ulp. The even
                    // endpoint is one; directed rounding depends on the sign.
                    let half = F {
                        sig: 1,
                        exp: -bits,
                        neg,
                    };
                    let one = F::int(if neg { -1 } else { 1 });
                    let increment = matches!((rounding, neg), (RingRounding::Down, true) | (RingRounding::Up, false));
                    let expected = F {
                        sig: (1u64 << (bits - 1)) + u64::from(increment),
                        exp: 1 - bits,
                        neg,
                    };
                    assert_eq!(
                        context.add(one, half).cmp(expected),
                        Ordering::Equal,
                        "{context:?}, negative={neg}"
                    );

                    // The other tie, one ulp farther on, rounds to the even
                    // endpoint two ulps away when RC is nearest.
                    let odd = F {
                        sig: (1u64 << (bits - 1)) + 1,
                        exp: 1 - bits,
                        neg,
                    };
                    let increment = increment || rounding == RingRounding::Nearest;
                    let expected = F {
                        sig: odd.sig + u64::from(increment),
                        ..odd
                    };
                    assert_eq!(
                        context.add(odd, half).cmp(expected),
                        Ordering::Equal,
                        "{context:?}, odd negative={neg}"
                    );
                }
            }
        }
    }

    #[test]
    fn square_roots_round_at_inherited_precision_without_a_binary32_spill() {
        // Exact lower dyadic bounds for sqrt(2), independently of host FP.
        for (precision, lower, exp, nearest_up) in [
            (RingPrecision::Bits24, 0xb504f3, -23, false),
            (RingPrecision::Bits53, 0x16a09e667f3bcc, -52, true),
            (RingPrecision::Bits64, 0xb504f333f9de6484, -63, false),
        ] {
            for rounding in [
                RingRounding::Nearest,
                RingRounding::Down,
                RingRounding::Up,
                RingRounding::TowardZero,
            ] {
                let context = RingContext { precision, rounding };
                let increment = rounding == RingRounding::Up || (rounding == RingRounding::Nearest && nearest_up);
                let expected = F {
                    sig: lower + u64::from(increment),
                    exp,
                    neg: false,
                };
                assert_eq!(context.sqrt(F::int(2)).cmp(expected), Ordering::Equal, "{context:?}");
                assert_eq!(
                    context.sqrt(F::int(4)).cmp(F::int(2)),
                    Ordering::Equal,
                    "exact square: {context:?}"
                );
            }
        }
    }
}
