// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! A small, dependency-free JSON reader (RFC 8259 subset sufficient for the
//! conformance vectors: objects, arrays, strings with escapes, numbers,
//! booleans, null). Object key order is preserved.

use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    /// Integers are kept exact; everything else is a float.
    Int(i64),
    Float(f64),
    Str(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

#[derive(Debug)]
pub struct ParseError {
    pub offset: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON parse error at byte {}: {}", self.offset, self.message)
    }
}

impl Json {
    pub fn parse(text: &str) -> Result<Json, ParseError> {
        let mut p = Parser {
            s: text.as_bytes(),
            i: 0,
        };
        p.ws();
        let v = p.value()?;
        p.ws();
        if p.i != p.s.len() {
            return Err(p.err("trailing characters"));
        }
        Ok(v)
    }

    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Object(kv) => kv.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&[(String, Json)]> {
        match self {
            Json::Object(kv) => Some(kv),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Integers, or strings holding a decimal or `0x` hexadecimal integer
    /// (message IDs are easier to read in hex).
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Json::Int(i) => Some(*i),
            Json::Float(f) if f.fract() == 0.0 && f.abs() < 9.0e15 => Some(*f as i64),
            Json::Str(s) => {
                let t = s.trim();
                let (neg, t) = match t.strip_prefix('-') {
                    Some(r) => (true, r),
                    None => (false, t),
                };
                let v = if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
                    i64::from_str_radix(&h.replace('_', ""), 16).ok()?
                } else {
                    t.parse::<i64>().ok()?
                };
                Some(if neg { -v } else { v })
            }
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Json::Int(i) => Some(*i as f64),
            Json::Float(f) => Some(*f),
            _ => None,
        }
    }
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn err(&self, m: &str) -> ParseError {
        ParseError {
            offset: self.i,
            message: m.to_owned(),
        }
    }

    fn ws(&mut self) {
        while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn expect(&mut self, c: u8) -> Result<(), ParseError> {
        if self.peek() == Some(c) {
            self.i += 1;
            Ok(())
        } else {
            Err(self.err(&format!("expected '{}'", c as char)))
        }
    }

    fn literal(&mut self, word: &str, v: Json) -> Result<Json, ParseError> {
        if self.s[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            Ok(v)
        } else {
            Err(self.err("invalid literal"))
        }
    }

    fn value(&mut self) -> Result<Json, ParseError> {
        match self.peek() {
            Some(b'{') => self.object(),
            Some(b'[') => self.array(),
            Some(b'"') => Ok(Json::Str(self.string()?)),
            Some(b't') => self.literal("true", Json::Bool(true)),
            Some(b'f') => self.literal("false", Json::Bool(false)),
            Some(b'n') => self.literal("null", Json::Null),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.number(),
            _ => Err(self.err("expected a value")),
        }
    }

    fn object(&mut self) -> Result<Json, ParseError> {
        self.expect(b'{')?;
        let mut kv = Vec::new();
        self.ws();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(Json::Object(kv));
        }
        loop {
            self.ws();
            let k = self.string()?;
            self.ws();
            self.expect(b':')?;
            self.ws();
            let v = self.value()?;
            kv.push((k, v));
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Json::Object(kv));
                }
                _ => return Err(self.err("expected ',' or '}'")),
            }
        }
    }

    fn array(&mut self) -> Result<Json, ParseError> {
        self.expect(b'[')?;
        let mut a = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(Json::Array(a));
        }
        loop {
            self.ws();
            a.push(self.value()?);
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    return Ok(Json::Array(a));
                }
                _ => return Err(self.err("expected ',' or ']'")),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, ParseError> {
        let h = self
            .s
            .get(self.i..self.i + 4)
            .ok_or_else(|| self.err("short \\u escape"))?;
        let h = std::str::from_utf8(h).map_err(|_| self.err("bad \\u escape"))?;
        let v = u32::from_str_radix(h, 16).map_err(|_| self.err("bad \\u escape"))?;
        self.i += 4;
        Ok(v)
    }

    fn string(&mut self) -> Result<String, ParseError> {
        self.expect(b'"')?;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let c = self.peek().ok_or_else(|| self.err("unterminated string"))?;
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = self.peek().ok_or_else(|| self.err("bad escape"))?;
                    self.i += 1;
                    match e {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let mut cp = self.hex4()?;
                            if (0xD800..0xDC00).contains(&cp) && self.s[self.i..].starts_with(b"\\u") {
                                self.i += 2;
                                let lo = self.hex4()?;
                                cp = 0x10000 + ((cp - 0xD800) << 10) + (lo.wrapping_sub(0xDC00) & 0x3FF);
                            }
                            let ch = char::from_u32(cp).unwrap_or('\u{FFFD}');
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        _ => return Err(self.err("bad escape")),
                    }
                }
                _ => out.push(c),
            }
        }
        String::from_utf8(out).map_err(|_| self.err("invalid UTF-8 in string"))
    }

    fn number(&mut self) -> Result<Json, ParseError> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        let mut is_float = false;
        while let Some(c) = self.peek() {
            match c {
                b'0'..=b'9' => {}
                b'.' | b'e' | b'E' | b'+' | b'-' => is_float = true,
                _ => break,
            }
            self.i += 1;
        }
        let text = std::str::from_utf8(&self.s[start..self.i]).map_err(|_| self.err("bad number"))?;
        if !is_float {
            if let Ok(i) = text.parse::<i64>() {
                return Ok(Json::Int(i));
            }
        }
        text.parse::<f64>().map(Json::Float).map_err(|_| self.err("bad number"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_everything() {
        let j = Json::parse(r#"{"a": [1, -2, 3.5, "0x10", true, null], "b": {"c": "x\"é"}}"#).unwrap();
        let a = j.get("a").unwrap().as_array().unwrap();
        assert_eq!(a[0].as_i64(), Some(1));
        assert_eq!(a[1].as_i64(), Some(-2));
        assert_eq!(a[2].as_f64(), Some(3.5));
        assert_eq!(a[3].as_i64(), Some(16));
        assert_eq!(a[4].as_bool(), Some(true));
        assert_eq!(a[5], Json::Null);
        assert_eq!(j.get("b").unwrap().get("c").unwrap().as_str(), Some("x\"\u{e9}"));
        assert!(Json::parse("[1,]").is_err());
        assert!(Json::parse("{} x").is_err());
    }
}
