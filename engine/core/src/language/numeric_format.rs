//! Numeric runtime assembly from rz-strings §5.1. No host printf is used.
//!
//! The 12-byte extended scaling procedure is not specified sufficiently to
//! reproduce its final intermediate digit (RZS7). That single operation remains
//! an explicit provider; decimal rounding, layout and special values live here.

use super::{FormatPrimitives, StringFault};

/// Unsigned significant ASCII digits, with the decimal point `position` places
/// from the first digit. A provider must return the legacy rounded 17-digit
/// intermediate, not a shortest spelling or an exact binary64 expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecimalIntermediate {
    pub digits: Vec<u8>,
    pub position: i32,
}

pub trait LegacyDecimal {
    /// Input is finite, positive and nonzero. Trailing zeros may be omitted.
    fn convert(&self, magnitude: f64) -> Result<DecimalIntermediate, StringFault>;
}

impl LegacyDecimal for () {
    fn convert(&self, _: f64) -> Result<DecimalIntermediate, StringFault> {
        Err(StringFault::Unsupported("RZS7"))
    }
}

/// Implements §5.1 around the legacy decimal intermediate. With `()` as the
/// provider, finite nonzero conversion is explicitly Unsupported (RZS7).
#[derive(Debug, Default)]
pub struct NumericPrimitives<D = ()> {
    pub decimal: D,
}

// The first discarded byte is compared as ASCII, including special-value text.
fn round(digits: &mut Vec<u8>, position: &mut i32, keep: i32) {
    if keep < 0 {
        *digits = vec![b'0'];
        *position = 1;
        return;
    }
    let keep = keep as usize;
    let carry = digits.get(keep).is_some_and(|&guard| guard >= b'5');
    digits.truncate(keep);
    if carry {
        let mut i = digits.len();
        loop {
            if i == 0 {
                digits.insert(0, b'1');
                *position += 1;
                break;
            }
            i -= 1;
            if digits[i] != b'9' {
                digits[i] += 1;
                break;
            }
            digits[i] = b'0';
        }
    }
    if digits.is_empty() || digits.iter().all(|&c| c == b'0') {
        *digits = vec![b'0'];
        *position = 1;
    }
}

fn digit(digits: &[u8], index: i32) -> u8 {
    if index < 0 {
        b'0'
    } else {
        digits.get(index as usize).copied().unwrap_or(b'0')
    }
}

impl<D: LegacyDecimal> FormatPrimitives for NumericPrimitives<D> {
    fn float_digits(
        &self,
        conversion: u8,
        magnitude: f64,
        precision: Option<u32>,
        alternate: bool,
    ) -> Result<Vec<u8>, StringFault> {
        let general = matches!(conversion, b'g' | b'G');
        if !matches!(conversion, b'f' | b'e' | b'E' | b'g' | b'G') || magnitude < 0.0 {
            return Err(StringFault::Argument);
        }
        let mut precision = precision.unwrap_or(6).min(512);
        if general && precision == 0 {
            precision = 1;
        }
        // The spec assumes successful allocation. Unlike the native runtime's
        // private allocation, this bounded core representation needs no separate
        // precision>163 allocation/fallback policy.
        let (mut intermediate, negative_nan) = if magnitude == 0.0 {
            (
                DecimalIntermediate {
                    digits: vec![b'0'],
                    position: 1,
                },
                false,
            )
        } else if magnitude.is_infinite() {
            (
                DecimalIntermediate {
                    digits: b"1#INF".to_vec(),
                    position: 1,
                },
                false,
            )
        } else if magnitude.is_nan() {
            let marker = if magnitude.to_bits() == 0xfff8_0000_0000_0000 {
                b"1#IND".as_slice()
            } else {
                // The owner's double load/comparison may quiet signaling NaNs;
                // payload-preserving stringification is expressly not promised.
                b"1#QNAN".as_slice()
            };
            (
                DecimalIntermediate {
                    digits: marker.to_vec(),
                    position: 1,
                },
                magnitude.is_sign_negative(),
            )
        } else {
            let intermediate = self.decimal.convert(magnitude)?;
            if intermediate.digits.is_empty()
                || intermediate.digits.len() > 17
                || intermediate.digits.first() == Some(&b'0')
                || !intermediate.digits.iter().all(u8::is_ascii_digit)
                || !(-323..=309).contains(&intermediate.position)
            {
                return Err(StringFault::Argument);
            }
            (intermediate, false)
        };
        let keep = if general {
            precision as i32
        } else if conversion == b'f' {
            intermediate.position + precision as i32
        } else {
            precision as i32 + 1
        };
        round(&mut intermediate.digits, &mut intermediate.position, keep);
        let DecimalIntermediate { digits, position } = intermediate;
        let exponent = position - 1;
        let exponential = conversion != b'f' && (!general || exponent < -4 || exponent >= precision as i32);
        let fractional = if general {
            if exponential {
                precision as i32 - 1
            } else {
                (precision as i32 - position).max(0)
            }
        } else {
            precision as i32
        };
        let mut output = Vec::new();
        if negative_nan {
            output.push(b'-');
        }
        if exponential {
            output.push(digit(&digits, 0));
        } else {
            for i in 0..position.max(1) {
                output.push(if position <= 0 { b'0' } else { digit(&digits, i) });
            }
        }
        if fractional > 0 || alternate {
            output.push(b'.');
        }
        for i in 0..fractional {
            output.push(digit(&digits, if exponential { i + 1 } else { position + i }));
        }
        if general && !alternate && fractional > 0 {
            while output.last() == Some(&b'0') {
                output.pop();
            }
            if output.last() == Some(&b'.') {
                output.pop();
            }
        }
        if exponential {
            output.push(if matches!(conversion, b'E' | b'G') { b'E' } else { b'e' });
            output.push(if exponent < 0 { b'-' } else { b'+' });
            let exponent = exponent.unsigned_abs();
            output.extend([
                b'0' + (exponent / 100) as u8,
                b'0' + (exponent / 10 % 10) as u8,
                b'0' + (exponent % 10) as u8,
            ]);
        }
        Ok(output)
    }

    fn pointer_digits(&self, pointer: u32, _: Option<u32>) -> Result<Vec<u8>, StringFault> {
        Ok((0..8)
            .rev()
            .map(|i| b"0123456789ABCDEF"[((pointer >> (i * 4)) & 15) as usize])
            .collect())
    }
}
