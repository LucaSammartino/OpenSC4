// SPDX-License-Identifier: GPL-3.0-or-later
//! LTEXT (localized text) entries, type [`crate::tgi::TYPE_LTEXT`].
//!
//! ```text
//! u16 LE   character count
//! u16 LE   encoding marker: 0x1000 = UTF-16LE text follows, 0x0000 = 8-bit text follows
//! ...      count UTF-16LE units (marker 0x1000) or count bytes (marker 0x0000), no terminator
//! ```
//!
//! The public documentation describes the UTF-16 form. The 8-bit form occurs in real data: each
//! `SimCityLocale.DAT` has one, and city saves store names such as `Temp.png` this way.
//!
//! The type ID is shared with other data: the shipped `Sound.dat` stores audio clips (XA and RIFF/WAV) and
//! some bare text under `0x2026960B`, and one locale entry is bare 8-bit text without the header. Use
//! [`is_ltext`] to tell real LTEXT records apart before parsing.
//!
//! Choices *\[choice\]*: an unpaired surrogate becomes U+FFFD; bytes after the UTF-16 text are ignored; the
//! 8-bit form must be exactly `count` bytes long (otherwise audio whose first bytes happen to read as a
//! count and a zero marker would pass) and is decoded as UTF-8 when valid, Latin-1 otherwise.

use crate::error::{Error, Result};

/// The encoding marker of a UTF-16 LTEXT record.
pub const UTF16_MARKER: u16 = 0x1000;
/// The encoding marker of an 8-bit LTEXT record.
pub const BYTE_MARKER: u16 = 0x0000;

/// Decodes an LTEXT record.
pub fn parse(data: &[u8]) -> Result<String> {
    if data.len() < 4 {
        return Err(Error::Truncated {
            what: "LTEXT",
            needed: 4,
            available: data.len() as u64,
        });
    }
    let count = u16::from_le_bytes([data[0], data[1]]) as usize;
    let marker = u16::from_le_bytes([data[2], data[3]]);
    let body = &data[4..];
    match marker {
        UTF16_MARKER => {
            if body.len() < count * 2 {
                return Err(Error::Truncated {
                    what: "LTEXT",
                    needed: (4 + count * 2) as u64,
                    available: data.len() as u64,
                });
            }
            let units = body[..count * 2]
                .chunks_exact(2)
                .map(|c| u16::from_le_bytes([c[0], c[1]]));
            Ok(char::decode_utf16(units)
                .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
                .collect())
        }
        BYTE_MARKER if body.len() == count => Ok(match std::str::from_utf8(body) {
            Ok(s) => s.to_owned(),
            Err(_) => body.iter().map(|&b| b as char).collect(),
        }),
        _ => Err(Error::BadMagic {
            what: "LTEXT",
            found: data[..4].to_vec(),
        }),
    }
}

/// True if `data` is an LTEXT record in either form (see the module documentation).
pub fn is_ltext(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }
    let count = u16::from_le_bytes([data[0], data[1]]) as usize;
    match u16::from_le_bytes([data[2], data[3]]) {
        UTF16_MARKER => 4 + 2 * count <= data.len(),
        BYTE_MARKER => 4 + count == data.len(),
        _ => false,
    }
}

/// Encodes a string as a UTF-16 LTEXT record. Fails if it has more than 65535 UTF-16 units.
pub fn encode(text: &str) -> Result<Vec<u8>> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let n = u16::try_from(units.len()).map_err(|_| Error::Unsupported {
        what: "LTEXT",
        detail: format!("{} UTF-16 units", units.len()),
    })?;
    let mut out = Vec::with_capacity(4 + units.len() * 2);
    out.extend_from_slice(&n.to_le_bytes());
    out.extend_from_slice(&UTF16_MARKER.to_le_bytes());
    for u in units {
        out.extend_from_slice(&u.to_le_bytes());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        for s in ["", "Budget", "Département – ünïcödé", "ラーメン", "𝄞 clef"] {
            let e = encode(s).unwrap();
            assert!(is_ltext(&e));
            assert_eq!(parse(&e).unwrap(), s);
        }
    }

    #[test]
    fn layout() {
        assert_eq!(encode("Hi").unwrap(), [2, 0, 0x00, 0x10, b'H', 0, b'i', 0]);
    }

    #[test]
    fn quirks() {
        assert!(parse(&[1, 0]).is_err());
        assert!(matches!(parse(&[3, 0, 0, 0x10, b'a', 0]), Err(Error::Truncated { .. })));
        // trailing bytes ignored
        assert_eq!(parse(&[1, 0, 0, 0x10, b'a', 0, 0, 0]).unwrap(), "a");
        // lone surrogate
        assert_eq!(parse(&[1, 0, 0, 0x10, 0x00, 0xD8]).unwrap(), "\u{FFFD}");
        // other data under the same type: not LTEXT
        let xa = [b'X', b'A', 0, 0, 1, 2];
        assert!(matches!(parse(&xa), Err(Error::BadMagic { .. })));
        assert!(!is_ltext(&xa));
        assert!(!is_ltext(&[3, 0, 0, 0x10, b'a', 0]));
        // 8-bit form, exact length only
        let mut temp = vec![8, 0, 0, 0];
        temp.extend_from_slice(b"Temp.png");
        assert!(is_ltext(&temp));
        assert_eq!(parse(&temp).unwrap(), "Temp.png");
        assert!(!is_ltext(&[2, 0, 0, 0, b'a', b'b', b'c']));
        assert!(parse(&[2, 0, 0, 0, b'a', b'b', b'c']).is_err());
        assert_eq!(parse(&[2, 0, 0, 0, 0xE9, b't']).unwrap(), "\u{e9}t");
    }
}
