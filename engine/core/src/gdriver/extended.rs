// SPDX-License-Identifier: GPL-3.0-or-later
//! Round-to-nearest-even binary80 significand arithmetic for §8.6. The finite
//! exponent range required by f32 matrices and u32 viewports fits comfortably
//! inside binary80; it must not be rounded through a binary64 intermediate.
#[derive(Clone, Copy, Debug)]
enum Extended {
    Finite {
        negative: bool,
        significand: u64,
        exponent: i32,
    },
    Infinite(bool),
    Nan(u32),
}

fn rounded(value: u128, shift: u32) -> u128 {
    if shift == 0 {
        return value;
    }
    if shift > 128 {
        return 0;
    }
    let quotient = if shift == 128 { 0 } else { value >> shift };
    let half = 1u128 << (shift - 1);
    let remainder = value & (half | (half - 1));
    quotient + u128::from(remainder > half || remainder == half && quotient & 1 != 0)
}
fn jam(value: u128, shift: u32) -> u128 {
    if shift == 0 {
        value
    } else if shift >= 128 {
        u128::from(value != 0)
    } else {
        (value >> shift) | u128::from(value & ((1u128 << shift) - 1) != 0)
    }
}
impl Extended {
    fn finite(negative: bool, mut significand: u128, mut exponent: i32) -> Self {
        if significand > u64::MAX as u128 {
            significand >>= 1;
            exponent += 1;
        }
        Self::Finite {
            negative,
            significand: significand as u64,
            exponent,
        }
    }
    fn from_single(value: f32) -> Self {
        let bits = value.to_bits();
        let negative = bits >> 31 != 0;
        let fraction = bits & 0x7fffff;
        let e = (bits >> 23) & 255;
        if e == 255 {
            return if fraction == 0 {
                Self::Infinite(negative)
            } else {
                Self::Nan(bits | 0x400000)
            };
        }
        if e != 0 {
            return Self::finite(negative, ((fraction | 0x800000) as u128) << 40, e as i32 - 127);
        }
        if fraction == 0 {
            return Self::finite(negative, 0, 0);
        }
        let top = 31 - fraction.leading_zeros();
        Self::finite(negative, (fraction as u128) << (63 - top), top as i32 - 149)
    }
    fn reciprocal(value: u32, negative: bool) -> Self {
        if value == 0 {
            return Self::Infinite(negative);
        }
        let log = 31 - value.leading_zeros();
        let exponent = -(log as i32) - i32::from(!value.is_power_of_two());
        let numerator = 1u128 << (63 - exponent);
        let q = numerator / value as u128;
        let r = numerator % value as u128;
        Self::finite(
            negative,
            q + u128::from(2 * r > value as u128 || 2 * r == value as u128 && q & 1 != 0),
            exponent,
        )
    }
    fn multiply(self, other: Self) -> Self {
        use Extended::*;
        match (self, other) {
            (Nan(n), _) | (_, Nan(n)) => Nan(n),
            (Infinite(a), Infinite(b)) => Infinite(a ^ b),
            (
                Infinite(a),
                Finite {
                    negative: b,
                    significand: s,
                    ..
                },
            )
            | (
                Finite {
                    negative: b,
                    significand: s,
                    ..
                },
                Infinite(a),
            ) => {
                if s == 0 {
                    Nan(0xffc00000)
                } else {
                    Infinite(a ^ b)
                }
            }
            (
                Finite {
                    negative: a,
                    significand: x,
                    exponent: e,
                },
                Finite {
                    negative: b,
                    significand: y,
                    exponent: f,
                },
            ) => {
                let product = x as u128 * y as u128;
                if product == 0 {
                    return Self::finite(a ^ b, 0, 0);
                }
                let shift = if product >> 127 != 0 { 64 } else { 63 };
                Self::finite(a ^ b, rounded(product, shift), e + f + (shift as i32 - 63))
            }
        }
    }
    fn add(self, other: Self) -> Self {
        use Extended::*;
        match (self, other) {
            (Nan(n), _) | (_, Nan(n)) => Nan(n),
            (Infinite(a), Infinite(b)) => {
                if a == b {
                    Infinite(a)
                } else {
                    Nan(0xffc00000)
                }
            }
            (Infinite(a), _) | (_, Infinite(a)) => Infinite(a),
            (
                Finite {
                    negative: a,
                    significand: x,
                    exponent: e,
                },
                Finite {
                    negative: b,
                    significand: y,
                    exponent: f,
                },
            ) => {
                if x == 0 && y == 0 {
                    return Self::finite(a && b, 0, 0);
                }
                if x == 0 {
                    return other;
                }
                if y == 0 {
                    return self;
                }
                let (a, x, e, b, y, f) = if (e, x) >= (f, y) {
                    (a, x, e, b, y, f)
                } else {
                    (b, y, f, a, x, e)
                };
                let big = (x as u128) << 3;
                let small = jam((y as u128) << 3, (e - f) as u32);
                let mut sum = if a == b { big + small } else { big - small };
                if sum == 0 {
                    return Self::finite(false, 0, 0);
                }
                let top = 127 - sum.leading_zeros();
                let exponent = e + top as i32 - 66;
                sum = if top > 66 {
                    jam(sum, top - 66)
                } else {
                    sum << (66 - top)
                };
                Self::finite(a, rounded(sum, 3), exponent)
            }
        }
    }
    fn half(self) -> Self {
        match self {
            Self::Finite {
                negative,
                significand,
                exponent,
            } => Self::finite(negative, significand as u128, exponent - 1),
            _ => self,
        }
    }
    fn single(self) -> f32 {
        let bits = match self {
            Self::Nan(bits) => bits,
            Self::Infinite(negative) => ((negative as u32) << 31) | 0x7f800000,
            Self::Finite {
                negative,
                significand,
                mut exponent,
            } => {
                let sign = (negative as u32) << 31;
                if significand == 0 {
                    return f32::from_bits(sign);
                }
                if exponent < -126 {
                    let mantissa = rounded(significand as u128, (40 + (-126 - exponent)) as u32) as u32;
                    return f32::from_bits(sign | mantissa);
                }
                let mut mantissa = rounded(significand as u128, 40) as u32;
                if mantissa == 0x1000000 {
                    mantissa >>= 1;
                    exponent += 1;
                }
                sign | if exponent > 127 {
                    0x7f800000
                } else {
                    ((exponent + 127) as u32) << 23 | (mantissa & 0x7fffff)
                }
            }
        };
        f32::from_bits(bits)
    }
}

pub fn projection(m: [f32; 16], width: u32, height: u32) -> [f32; 16] {
    let a = Extended::reciprocal(width, true);
    let b = Extended::reciprocal(height, false);
    let mut p = m;
    for r in 0..4 {
        let i = r * 4;
        let w = Extended::from_single(m[i + 3]);
        p[i] = Extended::from_single(m[i]).add(w.multiply(a)).single();
        p[i + 1] = Extended::from_single(m[i + 1]).add(w.multiply(b)).single();
        p[i + 2] = Extended::from_single(m[i + 2]).add(w).half().single();
    }
    for v in &mut p[8..12] {
        *v = -*v;
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rational_x87_discriminator() {
        // Exact rational arithmetic, rounded to a 64-bit significand at each
        // operation, gives 0x30888889. Binary64 gives 0x30888888 (RD11).
        let mut m = [0.; 16];
        m[0] = f32::from_bits(0x3e2f59ff);
        m[3] = f32::from_bits(0x40a4645f);
        assert_eq!(projection(m, 30, 600)[0].to_bits(), 0x30888889);
    }
    #[test]
    fn single_roundtrip_and_cancellation() {
        for bits in [
            0, 0x80000000, 1, 0x80000001, 0x7fffff, 0x800000, 0x3f800000, 0x7f7fffff, 0x7f800000, 0xff800000,
            0x7fc01234,
        ] {
            assert_eq!(Extended::from_single(f32::from_bits(bits)).single().to_bits(), bits);
        }
        for value in [f32::MIN_POSITIVE, f32::MAX, 1., -2., f32::from_bits(1)] {
            assert_eq!(
                Extended::from_single(value)
                    .add(Extended::from_single(-value))
                    .single()
                    .to_bits(),
                0
            );
            assert_eq!(
                Extended::from_single(value)
                    .multiply(Extended::from_single(1.))
                    .single(),
                value
            );
        }
        assert!(projection([0.; 16], 0, 0)[0].is_nan());
    }
}
