//! rz-strings §5.1.1: software extended arithmetic, not host floating point.
use super::{DecimalIntermediate, StringFault};

#[derive(Clone, Copy)]
struct Extended {
    significand: u128,
    exponent: i32,
}

impl Extended {
    fn multiply(self, rhs: Self) -> Self {
        // Fifteen limb products only: discarded low products contribute no carry.
        let mut product = 0_u128;
        for i in 0..5 {
            for j in (4 - i)..5 {
                let a = (self.significand >> (16 * i)) & 0xffff;
                let b = (rhs.significand >> (16 * j)) & 0xffff;
                product += (a * b) << (16 * (i + j - 4));
            }
        }
        let mut exponent = self.exponent + rhs.exponent - 16382;
        while product & (1 << 95) == 0 {
            product <<= 1;
            exponent -= 1;
        }
        let mut significand = product >> 16;
        let remainder = product & 0xffff;
        if remainder > 0x8000 || (remainder == 0x8000 && significand & 1 != 0) {
            significand += 1;
            if significand == 1 << 80 {
                significand = 1 << 79;
                exponent += 1;
            }
        }
        Self { significand, exponent }
    }
}

// Raw stored entries, grouped by base-eight digit. Each tuple is (E, M).
const POSITIVE: [(i32, u128); 19] = [
    (0x4002, 0xA0000000000000000000),
    (0x4005, 0xC8000000000000000000),
    (0x4008, 0xFA000000000000000000),
    (0x400C, 0x9C400000000000000000),
    (0x400F, 0xC3500000000000000000),
    (0x4012, 0xF4240000000000000000),
    (0x4016, 0x98968000000000000000),
    (0x4019, 0xBEBC2000000000000000),
    (0x4034, 0x8E1BC9BF040000000000),
    (0x404E, 0xD3C21BCECCEDA1000000),
    (0x4069, 0x9DC5ADA82B70B59EF020),
    (0x4083, 0xEB194F8E1AE525FD5DD0),
    (0x409E, 0xAF298D050E4395D79671),
    (0x40B9, 0x82818F1281ED44A0BFF9),
    (0x40D3, 0xC2781F49FFCFA6D53CBF),
    (0x41A8, 0x93BA47C980E98CE0C66F),
    (0x427C, 0xE070F78D3927556B85BC),
    (0x4351, 0xAA7EEBFB9DF9DE8EDDBC),
    (0x4426, 0x81842F29F2CCE376E6A1),
];
const NEGATIVE: [(i32, u128); 19] = [
    (0x3FFB, 0xCCCCCCCCCCCCCCCDCCCD),
    (0x3FF8, 0xA3D70A3D70A3D70A3D71),
    (0x3FF5, 0x83126E978D4FDF3B645A),
    (0x3FF1, 0xD1B71758E219652CD3C3),
    (0x3FEE, 0xA7C5AC471B4784230FD0),
    (0x3FEB, 0x8637BD05AF6C69B6A640),
    (0x3FE7, 0xD6BF94D5E57A42BC3D33),
    (0x3FE4, 0xABCC77118461CEFDFDC2),
    (0x3FC9, 0xE69594BEC44DE15B4C2F),
    (0x3FAF, 0x9ABE14CD44753B53C492),
    (0x3F94, 0xCFB11EAD453994BA67DE),
    (0x3F7A, 0x8B61313BBABCE2C62324),
    (0x3F5F, 0xBB127C53B17EC1595561),
    (0x3F44, 0xFB158592BE068D2FEED7),
    (0x3F2A, 0xA87FEA27A539E9A53F24),
    (0x3E55, 0xDDD0467C64BCE4A1AC7D),
    (0x3D81, 0x91FF83775423CC067B63),
    (0x3CAC, 0xC0314325637A193AFA91),
    (0x3BD7, 0xFD00B897478238D18921),
];

fn extract(magnitude: f64) -> Result<([u8; 18], i32), StringFault> {
    if !magnitude.is_finite() || magnitude <= 0.0 {
        return Err(StringFault::Argument);
    }
    let bits = magnitude.to_bits();
    let b = ((bits >> 52) & 0x7ff) as i32;
    let fraction = u128::from(bits & ((1_u64 << 52) - 1));
    let mut value = if b == 0 {
        let length = 128 - fraction.leading_zeros();
        Extended {
            significand: fraction << (80 - length),
            exponent: 15308 + length as i32,
        }
    } else {
        Extended {
            significand: ((1 << 52) + fraction) << 27,
            exponent: b + 15360,
        }
    };
    let h = (value.significand >> 72) as i32;
    let e = value.exponent;
    let mut position = (e * 0x4D10 + 0x4D * (e / 256 + 2 * h) - 0x134312F4) >> 16;
    let table = if position > 0 { &NEGATIVE[..] } else { &POSITIVE[..] };
    let mut power = position.unsigned_abs();
    let mut group = 0;
    while power != 0 {
        let digit = (power & 7) as usize;
        if digit != 0 {
            let (exponent, mut significand) = table[group * 7 + digit - 1];
            if significand & 0xffff >= 0x8000 {
                significand -= 0x10000;
            }
            value = value.multiply(Extended { significand, exponent });
        }
        power >>= 3;
        group += 1;
    }
    if value.exponent >= 0x3fff {
        position += 1;
        value = value.multiply(Extended {
            significand: 0xCCCCCCCCCCCCCCCCCCCC,
            exponent: 0x3ffb,
        });
    }
    let mut residue = (value.significand << 8) >> (16382 - value.exponent);
    let mut digits = [b'0'; 18];
    for digit in &mut digits {
        residue *= 10;
        *digit += (residue >> 88) as u8;
        residue &= (1_u128 << 88) - 1;
    }
    Ok((digits, position))
}

pub(super) fn convert(magnitude: f64) -> Result<DecimalIntermediate, StringFault> {
    let (raw, mut position) = extract(magnitude)?;
    let mut digits = raw[..17].to_vec();
    if raw[17] >= b'5' {
        let mut index = digits.len();
        loop {
            if index == 0 {
                digits[0] = b'1';
                position += 1;
                break;
            }
            index -= 1;
            if digits[index] != b'9' {
                digits[index] += 1;
                break;
            }
            digits[index] = b'0';
        }
    }
    while digits.last() == Some(&b'0') {
        digits.pop();
    }
    Ok(DecimalIntermediate { digits, position })
}

#[cfg(test)]
pub(super) fn raw_digits(magnitude: f64) -> [u8; 18] {
    extract(magnitude).unwrap().0
}
