// SPDX-License-Identifier: GPL-3.0-or-later
//! Character encodings (drawing-fonts.md §6).
//!
//! Text arrives as the bytes of the font's codePage; `n` counts those bytes (for codePage 0x10 the
//! units are little-endian byte pairs).

use super::thai;
use super::CodePageConverter;

pub const CP_SYSTEM: u32 = 0;
pub const CP_SINGLE_BYTE: u32 = 8;
pub const CP_UTF16: u32 = 0x10;

/// `ConvertCharToUnicode` (`0x0089671F`): (unit, bytes consumed).
pub fn convert_char_to_unicode(conv: &dyn CodePageConverter, cp: u32, s: &[u8]) -> (u16, usize) {
    if s.is_empty() {
        return (0, 0);
    }
    if cp == CP_UTF16 {
        // FN21 provisional boundary: a lone trailing byte is read as the low half.
        let lo = s[0] as u16;
        let hi = s.get(1).copied().unwrap_or(0) as u16;
        return (lo | (hi << 8), 2);
    }
    let b = s[0];
    if b < 0x80 {
        return (b as u16, 1);
    }
    if cp == CP_SYSTEM {
        return (conv.ansi_byte_to_unicode(b), 1);
    }
    let len = conv.mb_char_len(cp, s).clamp(1, s.len());
    (conv.mb_to_unicode(cp, &s[..len]), len)
}

/// Converted text: UTF-16 units plus, per unit, the byte offset of its source character (the §11
/// map composed with the conversion).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Converted {
    pub units: Vec<u16>,
    pub src: Vec<u32>,
    pub src_len: u32,
    /// The §11 substitution ran, so the font's map vector (+0xD8) is non-empty.
    pub substituted: bool,
}

impl Converted {
    /// Source length (bytes) of the first `k` units.
    pub fn src_len_of(&self, k: usize) -> u32 {
        if k >= self.units.len() {
            self.src_len
        } else {
            self.src[k]
        }
    }
}

/// `GetTextAsUnicode`: convert the whole string, then apply the §11 substitution when the text holds
/// Thai characters. (Substitution of text without forms to change copies it unchanged, so applying
/// it to every text with a U+0E00..U+0E7F unit gives the same result as "only when needed".)
pub fn text_as_unicode(conv: &dyn CodePageConverter, cp: u32, s: &[u8]) -> Converted {
    let mut units = Vec::new();
    let mut src = Vec::new();
    let mut pos = 0usize;
    while pos < s.len() {
        let (u, len) = convert_char_to_unicode(conv, cp, &s[pos..]);
        units.push(u);
        src.push(pos as u32);
        pos += len.max(1);
    }
    let src_len = pos.min(s.len()) as u32;
    if units.iter().any(|&u| (0x0E00..=0x0E7F).contains(&u)) {
        let (out, map) = thai::make_substituted_string(&units);
        let src2 = map.iter().map(|&i| src[i as usize]).collect();
        return Converted {
            units: out,
            src: src2,
            src_len,
            substituted: true,
        };
    }
    Converted {
        units,
        src,
        src_len,
        substituted: false,
    }
}

/// Probe characters of `IsCodePageAvailable(cp)` (slot 25, switch at `0x00898A7E`).
pub fn code_page_probes(cp: u32) -> &'static [u16] {
    match cp {
        8 | 0x10 => &[],
        0 | 1252 => &[0x0061],
        874 => &[0x0E01, 0x0E2C],
        932 => &[0x3061, 0x30A4],
        936 => &[0x70A7, 0x7535],
        949 => &[0xC2B9, 0xC9D1],
        950 => &[0x6F54, 0x70B5],
        1250 => &[0x0142],
        1251 => &[0x0401],
        1253 => &[0x0370, 0x0393],
        1254 => &[0x011E],
        1255 => &[0x05D0, 0x05E2],
        1256 => &[0x0621, 0x0641],
        1257 => &[0x0160],
        _ => &[],
    }
}
