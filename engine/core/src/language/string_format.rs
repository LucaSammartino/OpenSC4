//! Typed core equivalent of Sprintf (§5), not a native varargs ABI or host printf.
//! Formatting is staged so unsupported edges do not partially edit the receiver.

use super::rz_string::c_prefix;
use super::{RzString, StringFault};

#[derive(Debug, Clone, Copy)]
pub enum FormatArg<'a> {
    Signed(i64),
    Unsigned(u64),
    Float(f64),
    Bytes(&'a [u8]),
    Byte(u8),
    Pointer(u32),
}

/// Raw numeric conversion boundary. The spec names float/pointer families but
/// does not define their digit conversion, rounding, exponent or pointer form
/// (RZS6). Supply those primitives, never an arbitrary whole-format host printf.
/// These methods return digits before field padding; float input is a magnitude.
pub trait FormatPrimitives {
    fn float_digits(
        &self,
        conversion: u8,
        magnitude: f64,
        precision: Option<u32>,
        alternate: bool,
    ) -> Result<Vec<u8>, StringFault>;
    fn pointer_digits(&self, pointer: u32, precision: Option<u32>) -> Result<Vec<u8>, StringFault>;
}

#[derive(Default)]
struct Flags {
    left: bool,
    plus: bool,
    space: bool,
    zero: bool,
    alternate: bool,
}

fn next_arg<'a>(args: &[FormatArg<'a>], index: &mut usize) -> Result<FormatArg<'a>, StringFault> {
    let arg = args.get(*index).copied().ok_or(StringFault::Argument)?;
    *index += 1;
    Ok(arg)
}

fn field_number(format: &[u8], index: &mut usize) -> Result<Option<u32>, StringFault> {
    let mut number = None::<u32>;
    while let Some(&byte @ b'0'..=b'9') = format.get(*index) {
        number = Some(
            number
                .unwrap_or(0)
                .checked_mul(10)
                .and_then(|n| n.checked_add((byte - b'0') as u32))
                .ok_or(StringFault::Unsupported("RZS5"))?,
        );
        *index += 1;
    }
    Ok(number)
}

fn dynamic(args: &[FormatArg<'_>], index: &mut usize) -> Result<u32, StringFault> {
    match next_arg(args, index)? {
        FormatArg::Signed(n) if (0..=i32::MAX as i64).contains(&n) => Ok(n as u32),
        FormatArg::Signed(_) => Err(StringFault::Unsupported("RZS5")),
        _ => Err(StringFault::Argument),
    }
}

fn glyphs(bytes: &[u8]) -> Result<usize, StringFault> {
    // RZS5 leaves invalid encoding traversal open. Valid UTF-8 is traversed
    // character by character rather than splitting a multibyte character.
    Ok(std::str::from_utf8(bytes)
        .map_err(|_| StringFault::Unsupported("RZS5"))?
        .chars()
        .count())
}

fn string_precision(bytes: &[u8], precision: Option<u32>) -> Result<Vec<u8>, StringFault> {
    let bytes = c_prefix(bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| StringFault::Unsupported("RZS5"))?;
    let end = precision
        .and_then(|n| text.char_indices().nth(n as usize).map(|(i, _)| i))
        .unwrap_or(bytes.len());
    Ok(bytes[..end].to_vec())
}

fn integer_digits(mut value: u64, base: u64, upper: bool) -> Vec<u8> {
    let alphabet = if upper {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    let mut digits = Vec::new();
    loop {
        digits.push(alphabet[(value % base) as usize]);
        value /= base;
        if value == 0 {
            break;
        }
    }
    digits.reverse();
    digits
}

impl RzString {
    pub fn sprintf(
        &mut self,
        format: &[u8],
        args: &[FormatArg<'_>],
        primitives: &impl FormatPrimitives,
    ) -> Result<&mut Self, StringFault> {
        let format = c_prefix(format);
        let mut result = Vec::new();
        let mut i = 0;
        let mut argument = 0;
        while i < format.len() {
            if format[i] != b'%' {
                result.push(format[i]);
                i += 1;
                continue;
            }
            i += 1;
            if format.get(i) == Some(&b'%') {
                result.push(b'%');
                i += 1;
                continue;
            }
            let mut flags = Flags::default();
            while let Some(byte) = format.get(i) {
                match byte {
                    b'-' => flags.left = true,
                    b'+' => flags.plus = true,
                    b' ' => flags.space = true,
                    b'0' => flags.zero = true,
                    b'#' => flags.alternate = true,
                    _ => break,
                }
                i += 1;
            }
            if flags.left {
                flags.zero = false;
            }
            if flags.plus {
                flags.space = false;
            }
            let width = if format.get(i) == Some(&b'*') {
                i += 1;
                dynamic(args, &mut argument)?
            } else {
                field_number(format, &mut i)?.unwrap_or(0)
            };
            let precision = if format.get(i) == Some(&b'.') {
                i += 1;
                Some(if format.get(i) == Some(&b'*') {
                    i += 1;
                    dynamic(args, &mut argument)?
                } else {
                    field_number(format, &mut i)?.unwrap_or(0)
                })
            } else {
                None
            };
            let modifier_start = i;
            let bits = if format.get(i..i + 3) == Some(b"I64") {
                i += 3;
                64
            } else if format.get(i..i + 2) == Some(b"ll") {
                i += 2;
                64
            } else if format.get(i) == Some(&b'h') {
                i += 1;
                16
            } else if format.get(i) == Some(&b'l') {
                i += 1;
                32
            } else {
                32
            };
            let conversion = *format.get(i).ok_or(StringFault::Unsupported("RZS5"))?;
            if i != modifier_start && matches!(conversion, b's' | b'c') {
                // Wide/unsupported character conversions are explicitly RZS5.
                return Err(StringFault::Unsupported("RZS5"));
            }
            i += 1;
            if conversion == b'n' {
                return Err(StringFault::Unsupported("RZS5"));
            }
            let arg = next_arg(args, &mut argument)?;
            let mut prefix = Vec::new();
            let mut digits;
            let numeric;
            match (conversion, arg) {
                (b's', FormatArg::Bytes(bytes)) => {
                    digits = string_precision(bytes, precision)?;
                    numeric = false;
                }
                (b'c', FormatArg::Byte(byte)) if byte != 0 => {
                    digits = vec![byte];
                    numeric = false;
                }
                (b'c', FormatArg::Byte(_)) => return Err(StringFault::Unsupported("RZS5")),
                (b'd' | b'i' | b'u' | b'o' | b'x' | b'X', arg) => {
                    let signed = conversion == b'd' || conversion == b'i';
                    let raw = match arg {
                        FormatArg::Signed(n) if signed && n >= 0 => n as u64,
                        FormatArg::Signed(_) if signed => return Err(StringFault::Unsupported("RZS5")),
                        FormatArg::Unsigned(n) if !signed => n,
                        _ => return Err(StringFault::Argument),
                    };
                    let value = match bits {
                        16 => raw as u16 as u64,
                        32 => raw as u32 as u64,
                        _ => raw,
                    };
                    if signed && value >= (1u64 << (bits - 1)) {
                        return Err(StringFault::Unsupported("RZS5"));
                    }
                    let base = match conversion {
                        b'o' => 8,
                        b'x' | b'X' => 16,
                        _ => 10,
                    };
                    digits = integer_digits(value, base, conversion == b'X');
                    // Literal conventional digit/precision interpretation where
                    // §5 lists syntax but gives no zero/alternate vector (RZS6).
                    if precision == Some(0) && value == 0 {
                        digits.clear();
                    }
                    if let Some(n) = precision {
                        if n as usize > digits.len() {
                            let mut padded = vec![b'0'; n as usize - digits.len()];
                            padded.extend(digits);
                            digits = padded;
                        }
                    }
                    if signed {
                        if flags.plus {
                            prefix.push(b'+');
                        } else if flags.space {
                            prefix.push(b' ');
                        }
                    }
                    if flags.alternate {
                        if base == 16 && value != 0 {
                            prefix.extend_from_slice(if conversion == b'X' { b"0X" } else { b"0x" });
                        }
                        if base == 8 && digits.first() != Some(&b'0') {
                            prefix.push(b'0');
                        }
                    }
                    numeric = true;
                }
                (b'e' | b'E' | b'f' | b'g' | b'G', FormatArg::Float(value)) => {
                    digits = primitives.float_digits(conversion, value.abs(), precision, flags.alternate)?;
                    if value.is_sign_negative() {
                        prefix.push(b'-');
                    } else if flags.plus {
                        prefix.push(b'+');
                    } else if flags.space {
                        prefix.push(b' ');
                    }
                    numeric = true;
                }
                (b'p', FormatArg::Pointer(pointer)) => {
                    digits = primitives.pointer_digits(pointer, precision)?;
                    numeric = true;
                }
                (b's' | b'c' | b'e' | b'E' | b'f' | b'g' | b'G' | b'p', _) => return Err(StringFault::Argument),
                _ => return Err(StringFault::Unsupported("RZS5")),
            }
            let length = glyphs(&prefix)? + glyphs(&digits)?;
            let padding = (width as usize).saturating_sub(length);
            if flags.left {
                result.extend(prefix);
                result.extend(digits);
                result.resize(result.len() + padding, b' ');
            } else if flags.zero && numeric {
                result.extend(prefix);
                result.resize(result.len() + padding, b'0');
                result.extend(digits);
            } else {
                result.resize(result.len() + padding, b' ');
                result.extend(prefix);
                result.extend(digits);
            }
        }
        self.from_char(Some(&result), result.len() as u32);
        Ok(self)
    }
}
