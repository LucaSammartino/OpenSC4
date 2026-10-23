// SPDX-License-Identifier: GPL-3.0-or-later
use super::{parse_error, Result, ScriptToken};
use std::sync::atomic::{AtomicU8, Ordering};

/// Portable process-state model of foundations §18.2. A native adapter must
/// mirror this transition to its floating-point environment; never restore it.
pub static EXPRESSION_PRECISION: AtomicU8 = AtomicU8::new(24);

fn white(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

pub fn next_token(input: &[u8], cursor: &mut usize) -> Result<Option<Vec<u8>>> {
    let input = &input[..input.iter().position(|b| *b == b'#').unwrap_or(input.len())];
    while input.get(*cursor).copied().is_some_and(white) {
        *cursor += 1;
    }
    let Some(&first) = input.get(*cursor) else {
        return Ok(None);
    };
    if first == b'#' {
        *cursor = input.len();
        return Ok(None);
    }
    let close = match first {
        b'[' => Some(b']'),
        b'(' => Some(b')'),
        b'{' => Some(b'}'),
        _ => None,
    };
    if let Some(end) = close {
        *cursor += 1;
        let start = *cursor;
        let mut depth = 1;
        while let Some(&b) = input.get(*cursor) {
            if b == first { depth += 1; }
            if b == end {
                depth -= 1;
                if depth == 0 {
                    let token = input[start..*cursor].to_vec();
                    *cursor += 1;
                    return Ok(Some(token));
                }
            }
            *cursor += 1;
        }
        return Err(parse_error(6, "Unterminated group"));
    }
    let mut token = Vec::new();
    while let Some(&b) = input.get(*cursor) {
        if white(b) { break; }
        if matches!(b, b'\'' | b'"') {
            let quote = b;
            *cursor += 1;
            let start = *cursor;
            while input.get(*cursor).is_some_and(|b| *b != quote) { *cursor += 1; }
            if *cursor == input.len() { return Err(parse_error(5, "Unterminated quote")); }
            token.extend_from_slice(&input[start..*cursor]);
                *cursor += 1;
        } else {
            token.push(b);
            *cursor += 1;
        }
    }
    Ok(Some(token))
}
pub fn split(input: &[u8]) -> Result<Vec<ScriptToken>> {
    let mut result = Vec::new();
    let mut cursor = 0;
    while let Some(token) = next_token(input, &mut cursor)? {
        result.push(ScriptToken::new(result.len() as u32, token));
    }
    Ok(result)
}
pub fn count_arguments(input: &[u8]) -> Result<usize> {
    Ok(split(input)?.len())
}

// The mantissa and exponent deliberately use the legacy helper, not strtod.
fn english_decimal(bytes: &[u8]) -> Result<f64> {
    let mut at = 0;
    let mut value = 0.0;
    while bytes.get(at).is_some_and(u8::is_ascii_digit) {
        value = value * 10.0 + f64::from(bytes[at] - b'0');
        at += 1;
    }
    if bytes.get(at) == Some(&b'.') {
        at += 1;
        let mut factor = 0.1f64;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            value += f64::from(bytes[at] - b'0') * factor;
            factor *= 0.1;
            at += 1;
        }
    }
    if matches!(bytes.get(at), Some(b'e' | b'E')) {
        at += 1;
        let minus = bytes.get(at) == Some(&b'-');
        if matches!(bytes.get(at), Some(b'+' | b'-')) { at += 1; }
        let mut exponent = 0u32;
        while bytes.get(at).is_some_and(u8::is_ascii_digit) {
            exponent = exponent.wrapping_mul(10).wrapping_add(u32::from(bytes[at] - b'0'));
            at += 1;
        }
        let signed = exponent as i32;
        let power = 10i32.wrapping_pow(signed.unsigned_abs());
        let power = if signed < 0 {
            if power == 0 { return Err(parse_error(0, "Integer reciprocal of zero")); }
            1 / power
        } else { power };
        let scale = f64::from(power);
        value *= if minus { 1.0 / scale } else { scale };
    }
    Ok(value)
}
struct Expression<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Expression<'_> {
    fn space(&mut self) {
        while self.bytes.get(self.at).copied().is_some_and(white) {
            self.at += 1;
        }
    }
    fn term(&mut self) -> Result<f64> {
        self.space();
        let mut negative = false;
        while matches!(self.bytes.get(self.at), Some(b'+' | b'-')) {
            negative ^= self.bytes[self.at] == b'-';
            self.at += 1;
            self.space();
        }
        let value = self.factor()?;
        Ok(if negative { -value } else { value })
    }
    fn punctuation(&mut self, byte: u8) -> Result<()> {
        self.space();
        if self.bytes.get(self.at) != Some(&byte) { return Err(parse_error(0, "Expected punctuation")); }
        self.at += 1;
        Ok(())
    }
    fn factor(&mut self) -> Result<f64> {
        if self.bytes.get(self.at) == Some(&b'(') {
            self.at += 1;
            let value = self.sum()?;
            self.punctuation(b')')?;
            return Ok(value);
        }
        if self.bytes.get(self.at).is_some_and(u8::is_ascii_alphabetic) {
            let start = self.at;
            while self.bytes.get(self.at).is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_') { self.at += 1; }
            let name = &self.bytes[start..self.at];
            if name == b"pi" { return Ok(f64::from(f32::from_bits(0x4049_0fdb))); }
            if name == b"e" { return Ok(1f64.exp()); }
            self.punctuation(b'(')?;
            let a = self.sum()?;
            let b = if matches!(name, b"pow" | b"atan2" | b"datan2") {
                self.punctuation(b',')?;
                self.sum()?
            } else { 0.0 };
            self.punctuation(b')')?;
            let degrees = f64::from(f32::from_bits(0x3c8e_fa35));
            let inverse = f64::from(f32::from_bits(0x4265_2ee1));
            return Ok(match name {
                b"sqrt" => a.sqrt(), b"exp" => a.exp(), b"log" => a.ln(),
                b"abs" => a.abs(), b"floor" => a.floor(), b"ceil" => a.ceil(), b"sqr" => a*a,
                b"sin" => a.sin(), b"cos" => a.cos(), b"tan" => a.tan(),
                b"asin" => a.asin(), b"acos" => a.acos(), b"atan" => a.atan(),
                b"sind" => (a*degrees).sin(), b"cosd" => (a*degrees).cos(), b"tand" => (a*degrees).tan(),
                b"dasin" => a.asin()*inverse, b"dacos" => a.acos()*inverse, b"datan" => a.atan()*inverse,
                b"pow" => a.powf(b), b"atan2" => a.atan2(b), b"datan2" => a.atan2(b)*inverse,
                _ => return Err(parse_error(0, "Unknown function")),
            });
        }
        let start = self.at;
        if self.bytes.get(self.at..self.at+2) == Some(b"0x") {
            self.at += 2;
            let mut value = 0u32;
            let begin = self.at;
            while let Some(digit) = self.bytes.get(self.at).and_then(|b| (*b as char).to_digit(16)) {
                value = value.saturating_mul(16).saturating_add(digit);
                self.at += 1;
            }
            if begin == self.at { return Err(parse_error(0, "Invalid hex")); }
            return Ok(f64::from(value));
        }
        while self.bytes.get(self.at).is_some_and(|b| b.is_ascii_digit() || *b == b'.') { self.at += 1; }
        if self.at == start { return Err(parse_error(0, "Invalid numeric term")); }
        if matches!(self.bytes.get(self.at), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.bytes.get(self.at), Some(b'+' | b'-')) {
                self.at += 1;
            }
            while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
                self.at += 1;
            }
        }
        english_decimal(&self.bytes[start..self.at])
    }
    fn power(&mut self) -> Result<f64> {
        let a = self.term()?;
        self.space();
        if self.bytes.get(self.at) == Some(&b'^') {
            self.at += 1;
            Ok(a.powf(self.power()?))
        } else { Ok(a) }
    }
    fn product(&mut self) -> Result<f64> {
        let mut n = self.power()?;
        loop {
            self.space();
            let Some(&op) = self.bytes.get(self.at) else {
                break;
            };
            if !matches!(op, b'*' | b'/' | b'%') {
                break;
            }
            self.at += 1;
            let r = self.power()?;
            n = match op { b'*' => n*r, b'/' => n/r, _ => n - (n/r).floor()*r };
        }
        Ok(n)
    }
    fn sum(&mut self) -> Result<f64> {
        EXPRESSION_PRECISION.store(53, Ordering::SeqCst);
        let mut n = self.product()?;
        loop {
            self.space();
            let Some(&op) = self.bytes.get(self.at) else {
                break;
            };
            if !matches!(op, b'+' | b'-') {
                break;
            }
            self.at += 1;
            let r = self.product()?;
            n = if op == b'+' { n + r } else { n - r };
        }
        Ok(n)
    }
}
pub fn parse_float(input: &[u8]) -> Result<f64> {
    let mut e = Expression { bytes: input, at: 0 };
    let n = e.sum()?;
    e.space();
    if e.at != input.len() {
        return Err(parse_error(0, "Trailing expression text"));
    }
    Ok(n)
}
pub fn parse_int(input: &[u8]) -> Result<i32> {
    Ok(((parse_float(input)? + 12_582_912.0) as f32)
        .to_bits()
        .wrapping_sub(0x4b40_0000) as i32)
}
pub fn parse_uint(input: &[u8]) -> Result<u32> {
    Ok(crate::x87::ftol(parse_float(input)?) as u32)
}
pub fn parse_bool(input: &[u8]) -> Result<bool> {
    match input.to_ascii_lowercase().as_slice() {
        b"true" | b"on" => Ok(true),
        b"false" | b"off" | b"" => Ok(false),
        _ => Ok(parse_uint(input)? != 0),
    }
}
pub fn parse_enum(input: &[u8], primary: &[(&[u8], u32)], secondary: &[(&[u8], u32)]) -> Result<u32> {
    primary
        .iter()
        .chain(secondary)
        .find(|(name, _)| *name == input)
        .map(|(_, value)| *value)
        .ok_or_else(|| parse_error(4, "Unknown enum"))
}
fn components<const N: usize>(input: &[u8], minimum: usize, defaults: [f32; N]) -> Result<[f32; N]> {
    let tokens = split(input)?;
    if tokens.len() < minimum || tokens.len() > N {
        return Err(parse_error(0, "Wrong component count"));
    }
    let mut result = defaults;
    for (i, t) in tokens.iter().enumerate() {
        result[i] = parse_float(&t.bytes)? as f32;
    }
    Ok(result)
}
pub fn color(input: &[u8]) -> Result<[f32; 3]> {
    components(input, 3, [0.; 3])
}
pub fn color_alpha(input: &[u8]) -> Result<[f32; 4]> {
    components(input, 3, [0., 0., 0., 1.])
}
pub fn vector2(input: &[u8]) -> Result<[f32; 2]> {
    components(input, 0, [0.; 2])
}
pub fn vector3(input: &[u8]) -> Result<[f32; 3]> {
    components(input, 0, [0.; 3])
}
