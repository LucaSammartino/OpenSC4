// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cRZTextOStream` (§2): numbers as text through the derived `SetRZCharStr`,
//! raw bytes as a hex dump.

use super::c_str;
use super::com::{StreamCore, IID_GZ_OSTREAM};
use super::ostream::GzOStream;

/// `%g` of the MSVC 7 C runtime (§2.2): 6 significant digits, trailing zeros
/// and point removed, exponent form below 1e-4 or from 1e6 on, exponent with a
/// sign and at least three digits. `decimal_point` is the runtime locale's
/// (`b'.'` in the C locale) *[compat-limit]*.
///
/// Rounding: the spec's model (§2.2, OS3): the value's 17 significant digits
/// correctly rounded, then half-up on the 7th digit. The 17-digit step uses
/// Rust's exact `{:.16e}` as a generic correctly-rounded formatter (the native
/// class delegates to its C runtime at this boundary). *[compat-limit]* The
/// runtime's 12-byte power-of-ten scaling may differ from the model where its
/// 18th digit sits next to 5: unmeasured, open as OS7.
/// Infinities and NaNs use the runtime's `1.#INF` / `1.#QNAN` / `1.#IND` /
/// `1.#SNAN` forms *[inferred]*; -0.0 prints `-0` *[inferred]*.
pub fn format_g(d: f64, decimal_point: u8) -> String {
    let dp = decimal_point as char;
    let neg = d.is_sign_negative();
    let sign = if neg { "-" } else { "" };
    if d.is_infinite() {
        return format!("{sign}1{dp}#INF");
    }
    if d.is_nan() {
        let bits = d.to_bits();
        let mantissa = bits & 0x000F_FFFF_FFFF_FFFF;
        let quiet = mantissa & 0x0008_0000_0000_0000 != 0;
        // The x87 "indefinite" is the negative quiet NaN with no other bit.
        if neg && mantissa == 0x0008_0000_0000_0000 {
            return format!("-1{dp}#IND");
        }
        let kind = if quiet { "QNAN" } else { "SNAN" };
        return format!("{sign}1{dp}#{kind}");
    }
    if d == 0.0 {
        return format!("{sign}0");
    }
    let a = d.abs();
    let e17 = format!("{a:.16e}");
    let (mant, exp) = e17.split_once('e').expect("exponent form");
    let mut exp: i32 = exp.parse().expect("exponent");
    let all: Vec<u8> = mant.bytes().filter(u8::is_ascii_digit).map(|b| b - b'0').collect();
    let mut digits: Vec<u8> = all[..6].to_vec();
    if all[6] >= 5 {
        let mut i = 5;
        loop {
            if digits[i] == 9 {
                digits[i] = 0;
                if i == 0 {
                    digits.insert(0, 1);
                    digits.truncate(6);
                    exp += 1;
                    break;
                }
                i -= 1;
            } else {
                digits[i] += 1;
                break;
            }
        }
    }
    let ch = |v: &u8| (b'0' + v) as char;
    let strip = |frac: &str| -> String {
        let t = frac.trim_end_matches('0');
        if t.is_empty() {
            String::new()
        } else {
            format!("{dp}{t}")
        }
    };
    if !(-4..6).contains(&exp) {
        let first = ch(&digits[0]);
        let frac: String = digits[1..].iter().map(ch).collect();
        let esign = if exp < 0 { '-' } else { '+' };
        format!("{sign}{first}{}e{esign}{:03}", strip(&frac), exp.unsigned_abs())
    } else if exp >= 0 {
        let k = exp as usize + 1;
        let int: String = digits[..k].iter().map(ch).collect();
        let frac: String = digits[k..].iter().map(ch).collect();
        format!("{sign}{int}{}", strip(&frac))
    } else {
        let zeros = "0".repeat((-exp - 1) as usize);
        let frac: String = digits.iter().map(ch).collect();
        format!("{sign}0{}", strip(&format!("{zeros}{frac}")))
    }
}

// --- the text bodies (§2.2-§2.4), shared with the file stream in text mode -

pub(crate) fn text_set(s: &mut dyn GzOStream, text: String) -> bool {
    s.set_rz_char_str(text.as_bytes())
}

/// `SetVoid` as a hex dump (§2.4): one `SetRZCharStr` per piece, stopping at
/// the first that fails (returns false); true after the final `"\n"`.
pub(crate) fn hex_dump(s: &mut dyn GzOStream, p: &[u8]) -> bool {
    if !s.set_rz_char_str(b"\nHEXDUMP:\n") {
        return false;
    }
    for (i, &b) in p.iter().enumerate() {
        // Sign-extended *[quirk]*: 0x80-0xFF print as eight digits.
        let piece = format!("{:02x} ", b as i8 as i32 as u32);
        if !s.set_rz_char_str(piece.as_bytes()) {
            return false;
        }
        if (i + 1) % 25 == 0 && !s.set_rz_char_str(b"\n") {
            return false;
        }
    }
    s.set_rz_char_str(b"\n")
}

/// The derived class's pure slots of `cRZTextOStream`: `Flush` (3) and the
/// text sink `SetRZCharStr` (14). The sink gets a C string: the bytes before
/// the first NUL.
pub trait TextSink {
    fn flush(&mut self);
    fn set_rz_char_str(&mut self, s: &[u8]) -> bool;
}

/// `cRZTextOStream` over a typed text sink (§2.1). Unreachable in the shipped
/// game (§2.5) but kept for plugins and conformance.
pub struct RzTextOStream<T: TextSink> {
    core: StreamCore,
    sink: T,
    /// The C runtime's decimal point (§2.2) *[compat-limit]*.
    pub decimal_point: u8,
}

impl<T: TextSink> RzTextOStream<T> {
    /// No constructor of its own: the derived class sets the base fields; the
    /// byte-order flag is kept but ignored.
    pub fn new(little_endian: bool, sink: T) -> Self {
        RzTextOStream {
            core: StreamCore::new(little_endian, IID_GZ_OSTREAM),
            sink,
            decimal_point: b'.',
        }
    }
    pub fn sink(&self) -> &T {
        &self.sink
    }
    pub fn sink_mut(&mut self) -> &mut T {
        &mut self.sink
    }
}

impl<T: TextSink> GzOStream for RzTextOStream<T> {
    fn core(&self) -> &StreamCore {
        &self.core
    }
    fn core_mut(&mut self) -> &mut StreamCore {
        &mut self.core
    }
    fn flush(&mut self) {
        self.sink.flush()
    }
    fn set_sint8(&mut self, v: i8) -> bool {
        text_set(self, v.to_string())
    }
    fn set_uint8(&mut self, v: u8) -> bool {
        text_set(self, v.to_string())
    }
    fn set_sint16(&mut self, v: i16) -> bool {
        text_set(self, v.to_string())
    }
    fn set_uint16(&mut self, v: u16) -> bool {
        text_set(self, v.to_string())
    }
    fn set_sint32(&mut self, v: i32) -> bool {
        text_set(self, v.to_string())
    }
    fn set_uint32(&mut self, v: u32) -> bool {
        text_set(self, v.to_string())
    }
    fn set_sint64(&mut self, v: i64) -> bool {
        text_set(self, v.to_string())
    }
    fn set_uint64(&mut self, v: u64) -> bool {
        text_set(self, v.to_string())
    }
    fn set_float32(&mut self, f: f32) -> bool {
        let dp = self.decimal_point;
        text_set(self, format_g(f as f64, dp))
    }
    fn set_float64(&mut self, d: f64) -> bool {
        let dp = self.decimal_point;
        text_set(self, format_g(d, dp))
    }
    fn set_rz_char_str(&mut self, s: &[u8]) -> bool {
        self.sink.set_rz_char_str(c_str(s))
    }
    /// `SetRZCharStr(s.ToChar())`: stops at the first NUL *[quirk]* (§2.3).
    fn set_gz_str(&mut self, s: &[u8]) -> bool {
        self.set_rz_char_str(c_str(s))
    }
    fn set_void(&mut self, p: &[u8]) -> bool {
        hex_dump(self, p)
    }
    /// Always "error" (§2.1) *[quirk]*.
    fn get_error(&self) -> bool {
        true
    }
}
