// SPDX-License-Identifier: GPL-3.0-or-later
//! FN16: integer emulation of the x87 product, retaining the extended exponent range.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Precision {
    #[default]
    Pc24,
    Pc53,
    Pc64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Rounding {
    #[default]
    NearestEven,
    Down,
    Up,
    TowardZero,
}

/// Incoming thread control, supplied as environment state, never as a computed advance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct X87Control {
    pub precision: Precision,
    pub rounding: Rounding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConversionError {
    NonFinite,
    IntegerOverflow,
}

impl X87Control {
    /// Round `integer * scale` to PC significant bits, multiply by exact 2^binary_scale,
    /// then truncate to i64. FN16's inputs have at most 56 product bits, so PC64 is exact.
    /// The power-of-two multiplication neither loses bits nor exceeds x87's exponent range,
    /// even for f32 subnormals and maximum finite f32. No f32/f64 storage rounding occurs.
    pub fn product(self, integer: i32, scale: f32, binary_scale: i32) -> Result<i64, ConversionError> {
        let bits = scale.to_bits();
        let biased = (bits >> 23) & 255;
        if biased == 255 {
            return Err(ConversionError::NonFinite);
        }
        let negative = (integer < 0) ^ (bits >> 31 != 0);
        let mantissa = (bits & 0x7FFFFF) | if biased == 0 { 0 } else { 1 << 23 };
        let mut significand = u64::from(integer.unsigned_abs()) * u64::from(mantissa);
        if significand == 0 {
            return Ok(0);
        }
        let mut exponent = if biased == 0 { -149 } else { biased as i32 - 150 } + binary_scale;
        let precision = match self.precision {
            Precision::Pc24 => 24,
            Precision::Pc53 => 53,
            Precision::Pc64 => 64,
        };
        let discarded = (64 - significand.leading_zeros()).saturating_sub(precision);
        if discarded != 0 {
            let remainder = significand & ((1 << discarded) - 1);
            significand >>= discarded;
            let increment = match self.rounding {
                Rounding::NearestEven => {
                    let half = 1 << (discarded - 1);
                    remainder > half || (remainder == half && significand & 1 != 0)
                }
                Rounding::Down => negative && remainder != 0,
                Rounding::Up => !negative && remainder != 0,
                Rounding::TowardZero => false,
            };
            significand += u64::from(increment);
            exponent += discarded as i32;
        }
        let magnitude = if exponent < 0 {
            significand.checked_shr(exponent.unsigned_abs()).unwrap_or(0)
        } else {
            if exponent as u32 + (64 - significand.leading_zeros()) > 64 {
                return Err(ConversionError::IntegerOverflow);
            }
            significand << exponent
        };
        if magnitude > i64::MAX as u64 + u64::from(negative) {
            return Err(ConversionError::IntegerOverflow);
        }
        Ok(if negative {
            magnitude.wrapping_neg() as i64
        } else {
            magnitude as i64
        })
    }
}

/// Stored s16 advance (§7). Zero-width characters override every branch.
pub fn scaled_advance(adv: i32, scale: f32, ch: u32, control: X87Control) -> Result<i16, ConversionError> {
    if ch == 0x200B || ch == 0xFEFF {
        return Ok(0);
    }
    let narrow_band = scale > f32::from_bits(0x3F7AE148) && scale < 1.0;
    if scale == 1.0 || (narrow_band && ch != 0x20) {
        return Ok((adv / 65536) as i16);
    }
    let mut v = control.product(adv, scale, -16)? as i16;
    if narrow_band && i32::from(v) + 1 > adv / 65536 {
        v = v.wrapping_sub(1);
    }
    Ok(v)
}
