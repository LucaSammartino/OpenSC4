// SPDX-License-Identifier: GPL-3.0-or-later
use super::{parse_error, Error, Result, ScriptToken};
use std::sync::atomic::{AtomicU8, Ordering};

/// Portable process-state model of foundations §18.2. A native adapter must
/// mirror this transition to its floating-point environment; never restore it.
pub static EXPRESSION_PRECISION: AtomicU8 = AtomicU8::new(24);

fn white(b: u8) -> bool { matches!(b, b' ' | b'\t' | b'\n' | b'\r') }

pub fn next_token(input: &[u8], cursor: &mut usize) -> Result<Option<Vec<u8>>> {
    while input.get(*cursor).copied().is_some_and(white) { *cursor += 1; }
    let Some(&first) = input.get(*cursor) else { return Ok(None) };
    if first == b'#' { *cursor = input.len(); return Ok(None); }
    let close = match first { b'\'' | b'"' => Some(first), b'[' => Some(b']'), b'(' => Some(b')'), b'{' => Some(b'}'), _ => None };
    let start = *cursor + usize::from(close.is_some());
    *cursor = start;
    if let Some(end) = close {
        while let Some(&b) = input.get(*cursor) {
            if b == end {
                let token = input[start..*cursor].to_vec();
                *cursor += 1;
                return Ok(Some(token));
            }
            // SCR1 explicitly leaves mixed nesting and escapes unsettled.
            if b == b'\\' || (first != b'\'' && first != b'"' && matches!(b, b'[' | b'(' | b'{' | b'\'' | b'"')) {
                return Err(Error::Unsupported("SCR1: nested delimiters/escapes"));
            }
            *cursor += 1;
        }
        return Err(parse_error(if matches!(first, b'\'' | b'"') { 5 } else { 6 }, "Unterminated argument"));
    }
    while let Some(&b) = input.get(*cursor) {
        if white(b) || b == b'#' { break; }
        if matches!(b, b'\\' | b'\'' | b'"' | b'[' | b'(' | b'{') {
            return Err(Error::Unsupported("SCR1: embedded grouping/escapes"));
        }
        *cursor += 1;
    }
    Ok(Some(input[start..*cursor].to_vec()))
}
pub fn split(input: &[u8]) -> Result<Vec<ScriptToken>> {
    let mut result = Vec::new();
    let mut cursor = 0;
    while let Some(token) = next_token(input, &mut cursor)? {
        result.push(ScriptToken::new(result.len() as u32, token));
    }
    Ok(result)
}
pub fn count_arguments(input: &[u8]) -> Result<usize> { Ok(split(input)?.len()) }

struct Expression<'a> { bytes: &'a [u8], at: usize }
impl Expression<'_> {
    fn space(&mut self) { while self.bytes.get(self.at).copied().is_some_and(white) { self.at += 1; } }
    fn term(&mut self) -> Result<f64> {
        self.space();
        let start = self.at;
        if matches!(self.bytes.get(self.at), Some(b'+' | b'-')) { self.at += 1; }
        let mut digits = 0;
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) { self.at += 1; digits += 1; }
        if self.bytes.get(self.at) == Some(&b'.') {
            self.at += 1;
            while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) { self.at += 1; digits += 1; }
        }
        if digits == 0 { return Err(parse_error(0, "Invalid numeric term (SCR1 leaves extended grammar open)")); }
        if matches!(self.bytes.get(self.at), Some(b'e' | b'E')) {
            self.at += 1;
            if matches!(self.bytes.get(self.at), Some(b'+' | b'-')) { self.at += 1; }
            let exponent = self.at;
            while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) { self.at += 1; }
            if exponent == self.at { return Err(parse_error(0, "Invalid exponent")); }
        }
        // Literal decimal operands are the minimal reading needed by §9.
        std::str::from_utf8(&self.bytes[start..self.at]).ok().and_then(|s| s.parse().ok()).ok_or_else(|| parse_error(0, "Invalid number"))
    }
    fn product(&mut self) -> Result<f64> {
        let mut n = self.term()?;
        loop {
            self.space();
            let Some(&op) = self.bytes.get(self.at) else { break };
            if !matches!(op, b'*' | b'/' | b'%') { break; }
            self.at += 1;
            if op == b'%' { return Err(Error::Unsupported("SCR1: remainder operation")); }
            let r = self.term()?;
            n = if op == b'*' { n * r } else { n / r };
        }
        Ok(n)
    }
    fn sum(&mut self) -> Result<f64> {
        EXPRESSION_PRECISION.store(53, Ordering::SeqCst);
        let mut n = self.product()?;
        loop {
            self.space();
            let Some(&op) = self.bytes.get(self.at) else { break };
            if !matches!(op, b'+' | b'-') { break; }
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
    if e.at != input.len() { return Err(parse_error(0, "Trailing expression text")); }
    Ok(n)
}
pub fn parse_int(input: &[u8]) -> Result<i32> {
    Ok(((parse_float(input)? + 12_582_912.0) as f32).to_bits().wrapping_sub(0x4b40_0000) as i32)
}
pub fn parse_uint(input: &[u8]) -> Result<u32> { Ok(crate::x87::ftol(parse_float(input)?) as u32) }
pub fn parse_bool(input: &[u8]) -> Result<bool> {
    match input.to_ascii_lowercase().as_slice() {
        b"true" | b"on" => Ok(true), b"false" | b"off" | b"" => Ok(false), _ => Ok(parse_uint(input)? != 0),
    }
}
pub fn parse_enum(input: &[u8], primary: &[(&[u8], u32)], secondary: &[(&[u8], u32)]) -> Result<u32> {
    primary.iter().chain(secondary).find(|(name, _)| *name == input).map(|(_, value)| *value).ok_or_else(|| parse_error(4, "Unknown enum"))
}
fn components<const N: usize>(input: &[u8], minimum: usize, defaults: [f32; N]) -> Result<[f32; N]> {
    let tokens = split(input)?;
    if tokens.len() < minimum || tokens.len() > N { return Err(parse_error(0, "Wrong component count")); }
    let mut result = defaults;
    for (i, t) in tokens.iter().enumerate() { result[i] = parse_float(&t.bytes)? as f32; }
    Ok(result)
}
pub fn color(input: &[u8]) -> Result<[f32; 3]> { components(input, 3, [0.; 3]) }
pub fn color_alpha(input: &[u8]) -> Result<[f32; 4]> { components(input, 3, [0., 0., 0., 1.]) }
pub fn vector2(input: &[u8]) -> Result<[f32; 2]> { components(input, 0, [0.; 2]) }
pub fn vector3(input: &[u8]) -> Result<[f32; 3]> { components(input, 0, [0.; 3]) }
