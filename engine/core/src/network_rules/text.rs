// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The number readers of §1.6, a small `sscanf` for the patterns the rule
//! readers use, and the INI reader for the rule resources.

/// The C locale's `isspace`.
fn is_blank(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\x0B' | b'\x0C' | b'\r')
}

fn skip_blanks(s: &[u8], mut i: usize) -> usize {
    while i < s.len() && is_blank(s[i]) {
        i += 1;
    }
    i
}

/// `atol`: optional leading blanks and sign, then decimal digits; anything
/// else ends the number (empty gives 0). An overflow wraps *[choice]*.
pub fn atol(s: &str) -> i32 {
    let b = s.as_bytes();
    let mut i = skip_blanks(b, 0);
    let mut neg = false;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        neg = b[i] == b'-';
        i += 1;
    }
    let mut v: i32 = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        v = v.wrapping_mul(10).wrapping_add(i32::from(b[i] - b'0'));
        i += 1;
    }
    if neg {
        v.wrapping_neg()
    } else {
        v
    }
}

/// `RZAtoUint32HexA` (`0x0090FCEF`): leading blanks skipped, an optional
/// `0x` / `0X`, then at most 8 hex digits (either case); the first non-hex
/// character ends it.
pub fn rz_atou32_hex(s: &str) -> u32 {
    let b = s.as_bytes();
    let mut i = skip_blanks(b, 0);
    if i + 1 < b.len() && b[i] == b'0' && (b[i + 1] == b'x' || b[i + 1] == b'X') {
        i += 2;
    }
    let mut v: u32 = 0;
    let mut n = 0;
    while i < b.len() && n < 8 {
        let Some(d) = (b[i] as char).to_digit(16) else { break };
        v = (v << 4) | d;
        i += 1;
        n += 1;
    }
    v
}

/// `RZConvertToUint32A`: hex when the text starts with `0x` (or `0X`
/// *[choice]*), else decimal (read as [`atol`]). Hex digits accumulate
/// without a limit, wrapping *[choice]*.
pub fn rz_convert_u32(s: &str) -> u32 {
    let b = s.as_bytes();
    if b.len() >= 2 && b[0] == b'0' && (b[1] == b'x' || b[1] == b'X') {
        let mut v: u32 = 0;
        for &c in &b[2..] {
            let Some(d) = (c as char).to_digit(16) else { break };
            v = v.wrapping_shl(4) | d;
        }
        v
    } else {
        atol(s) as u32
    }
}

/// One value read by [`scan`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScanValue {
    Int(i64),
    Float(f32),
}

impl ScanValue {
    /// The value as a C `int` (a float truncated).
    pub fn as_i32(self) -> i32 {
        match self {
            ScanValue::Int(v) => v as i32,
            ScanValue::Float(f) => f as i32,
        }
    }

    pub fn as_f32(self) -> f32 {
        match self {
            ScanValue::Int(v) => v as f32,
            ScanValue::Float(f) => f,
        }
    }
}

fn digits(s: &[u8], mut i: usize, radix: u32) -> (usize, Option<i64>) {
    let start = i;
    let mut v: i64 = 0;
    while i < s.len() {
        let Some(d) = (s[i] as char).to_digit(radix) else { break };
        v = v.wrapping_mul(i64::from(radix)).wrapping_add(i64::from(d));
        i += 1;
    }
    (i, (i > start).then_some(v))
}

/// A C `sscanf` for the conversions the rule readers use: `%d`, `%i` (`0x`
/// hex, a leading `0` octal), `%x` (an optional `0x`), `%f`; a blank in the
/// pattern matches any run of blanks (none included); any other pattern
/// character must match itself. Returns the values assigned before the first
/// failure (their count is `sscanf`'s result).
pub fn scan(pattern: &str, input: &str) -> Vec<ScanValue> {
    let p = pattern.as_bytes();
    let s = input.as_bytes();
    let mut out = Vec::new();
    let (mut pi, mut si) = (0usize, 0usize);
    while pi < p.len() {
        let c = p[pi];
        if is_blank(c) {
            si = skip_blanks(s, si);
            pi += 1;
            continue;
        }
        if c != b'%' {
            if si < s.len() && s[si] == c {
                si += 1;
                pi += 1;
                continue;
            }
            return out;
        }
        let Some(&conv) = p.get(pi + 1) else { return out };
        pi += 2;
        si = skip_blanks(s, si);
        let mut neg = false;
        if conv != b'f' && si < s.len() && (s[si] == b'+' || s[si] == b'-') {
            neg = s[si] == b'-';
            si += 1;
        }
        let parsed = match conv {
            b'd' => digits(s, si, 10),
            b'x' => {
                if si + 1 < s.len() && s[si] == b'0' && (s[si + 1] == b'x' || s[si + 1] == b'X') {
                    let (j, v) = digits(s, si + 2, 16);
                    if v.is_some() {
                        (j, v)
                    } else {
                        (si + 1, Some(0))
                    }
                } else {
                    digits(s, si, 16)
                }
            }
            b'i' => {
                if si + 1 < s.len() && s[si] == b'0' && (s[si + 1] == b'x' || s[si + 1] == b'X') {
                    let (j, v) = digits(s, si + 2, 16);
                    if v.is_some() {
                        (j, v)
                    } else {
                        (si + 1, Some(0))
                    }
                } else if si < s.len() && s[si] == b'0' {
                    digits(s, si, 8)
                } else {
                    digits(s, si, 10)
                }
            }
            b'f' => {
                let start = si;
                let mut j = si;
                if j < s.len() && (s[j] == b'+' || s[j] == b'-') {
                    j += 1;
                }
                let int_start = j;
                while j < s.len() && s[j].is_ascii_digit() {
                    j += 1;
                }
                let mut mantissa = j > int_start;
                if j < s.len() && s[j] == b'.' {
                    let f0 = j + 1;
                    j += 1;
                    while j < s.len() && s[j].is_ascii_digit() {
                        j += 1;
                    }
                    mantissa |= j > f0;
                }
                if !mantissa {
                    return out;
                }
                if j < s.len() && (s[j] == b'e' || s[j] == b'E') {
                    let mut k = j + 1;
                    if k < s.len() && (s[k] == b'+' || s[k] == b'-') {
                        k += 1;
                    }
                    let e0 = k;
                    while k < s.len() && s[k].is_ascii_digit() {
                        k += 1;
                    }
                    if k > e0 {
                        j = k;
                    }
                }
                let text = String::from_utf8_lossy(&s[start..j]);
                let text = text.strip_suffix('.').map_or(text.to_string(), |t| format!("{t}.0"));
                let text = if text.starts_with('.') || text.starts_with("-.") || text.starts_with("+.") {
                    text.replacen('.', "0.", 1)
                } else {
                    text
                };
                match text.parse::<f32>() {
                    Ok(f) => {
                        out.push(ScanValue::Float(f));
                        si = j;
                        continue;
                    }
                    Err(_) => return out,
                }
            }
            _ => return out,
        };
        match parsed {
            (j, Some(v)) => {
                out.push(ScanValue::Int(if neg { v.wrapping_neg() } else { v }));
                si = j;
            }
            _ => return out,
        }
    }
    out
}

/// One INI section: its name as written and its `key = value` entries in
/// file order (keys may repeat).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IniSection {
    pub name: String,
    pub entries: Vec<(String, String)>,
}

/// The rule resources' INI reader *[choice: the grammar of the game's INI
/// reader is not in the specs]*: lines end at `\n` (a trailing `\r` is
/// dropped); blanks around a line are ignored; a line starting with `;` is a
/// comment; `[name]` starts a section; `key = value` splits at the first `=`,
/// both sides trimmed; other lines, and entries before the first section,
/// are ignored. A repeated section name starts a section of its own here;
/// [`find_section`] merges them in file order.
pub fn ini_sections(bytes: &[u8]) -> Vec<IniSection> {
    let text = String::from_utf8_lossy(bytes);
    let mut out: Vec<IniSection> = Vec::new();
    for raw in text.split('\n') {
        let line = raw.trim_matches(|c: char| c.is_ascii_whitespace());
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[') {
            if let Some(end) = rest.find(']') {
                out.push(IniSection {
                    name: rest[..end].trim().to_string(),
                    entries: Vec::new(),
                });
                continue;
            }
        }
        let Some(eq) = line.find('=') else { continue };
        if let Some(sec) = out.last_mut() {
            sec.entries
                .push((line[..eq].trim().to_string(), line[eq + 1..].trim().to_string()));
        }
    }
    out
}

/// The entries of every section named `name` (compared lower-cased), in file
/// order.
pub fn find_section<'a>(sections: &'a [IniSection], name: &str) -> Vec<&'a (String, String)> {
    let want = name.to_ascii_lowercase();
    sections
        .iter()
        .filter(|s| s.name.to_ascii_lowercase() == want)
        .flat_map(|s| s.entries.iter())
        .collect()
}
