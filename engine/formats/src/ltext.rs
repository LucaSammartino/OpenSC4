// SPDX-License-Identifier: GPL-3.0-or-later
//! LTEXT (localized text) entries, type [`crate::tgi::TYPE_LTEXT`].
//!
//! The string resource reads the payload with the foundations §2.3 string reader (`GetGZStr`), so the record is
//! (data-formats §6.1):
//!
//! ```text
//! uint32  header      encoding = header >> 24, count = header & 0x00FF_FFFF
//!         encoding 0x10: count UTF-16LE code units (converted to UTF-8)
//!         encoding 0x00: count bytes of 8-bit text
//! ```
//!
//! The usual description "`uint16` count, `uint16` marker `0x1000`/`0x0000`" is the same bytes for counts below
//! 65,536; the original reads a **24-bit** count. All §2.3 rules apply: count over 200,000 fails, and **bytes
//! after the string are ignored** (the length need not match the record size).
//!
//! The type ID is shared with other data: the shipped `Sound.dat` stores audio clips (XA and RIFF/WAV) and some
//! bare text under `0x2026960B`, and one locale entry is bare 8-bit text without a header. Use [`is_ltext`] to
//! tell records this reader accepts apart before parsing (the game never reads the audio as text: the caller
//! decides, data-formats §6.1).
//!
//! Choices *\[choice\]* (foundations §2.3): the §2.3 read quirks are not reproduced — the "second byte 0 means
//! UTF-16" rule does not exist (the header decides), an encoding byte other than 0 and 0x10 is an error rather
//! than the legacy raw-bytes path, and a record with fewer than 4 bytes is a truncation error. An unpaired
//! surrogate becomes U+FFFD. 8-bit text is decoded as UTF-8 when valid, Latin-1 otherwise.

use crate::error::Result;
use crate::reader::Reader;

/// The encoding byte of a UTF-16 LTEXT record (top byte of the §2.3 header).
pub const UTF16_ENCODING: u8 = 0x10;
/// The encoding byte of an 8-bit LTEXT record.
pub const BYTE_ENCODING: u8 = 0x00;

/// Decodes an LTEXT record with the §2.3 string reader. Trailing bytes after the string are ignored.
pub fn parse(data: &[u8]) -> Result<String> {
    let bytes = Reader::new(data, "LTEXT").gzstr()?;
    Ok(match std::str::from_utf8(&bytes) {
        Ok(s) => s.to_owned(),
        // 8-bit text that is not valid UTF-8: Latin-1 (one byte per character), as the game's internal text.
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    })
}

/// True if `data` is an LTEXT record this reader accepts: a 4-byte header with encoding 0x00 or 0x10, a count
/// of at most 200,000, and enough bytes for the string. Bytes after the string do not matter.
pub fn is_ltext(data: &[u8]) -> bool {
    if data.len() < 4 {
        return false;
    }
    let header = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let encoding = (header >> 24) as u8;
    let count = (header & 0x00FF_FFFF) as usize;
    if count > 200_000 {
        return false;
    }
    match encoding {
        BYTE_ENCODING => data.len() >= 4 + count,
        UTF16_ENCODING => data.len() >= 4 + 2 * count,
        _ => false,
    }
}

/// Encodes a string as a UTF-16 LTEXT record. Fails if it has more than 200,000 UTF-16 units.
pub fn encode(text: &str) -> Result<Vec<u8>> {
    let units: Vec<u16> = text.encode_utf16().collect();
    if units.len() > 200_000 {
        return Err(crate::error::Error::Unsupported {
            what: "LTEXT",
            detail: format!("{} UTF-16 units", units.len()),
        });
    }
    let header = (u32::from(UTF16_ENCODING) << 24) | units.len() as u32;
    let mut out = Vec::with_capacity(4 + units.len() * 2);
    out.extend_from_slice(&header.to_le_bytes());
    for u in units {
        out.extend_from_slice(&u.to_le_bytes());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Error;

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
        // 24-bit count in the low bytes, encoding 0x10 in the top byte: "Hi" = 2 units.
        assert_eq!(encode("Hi").unwrap(), [2, 0, 0, 0x10, b'H', 0, b'i', 0]);
    }

    /// §10.6 conformance vectors (data-formats §10.6, tests 44–49).
    #[test]
    fn conformance_vectors() {
        // 44: UTF-16 "Hi!!".
        assert_eq!(parse(&[0x04, 0x00, 0x00, 0x10, b'H', 0, b'i', 0, b'!', 0, b'!', 0]).unwrap(), "Hi!!");
        // 45: 8-bit "ABC"; a trailing byte is ignored (the length need not match).
        assert_eq!(parse(&[0x03, 0x00, 0x00, 0x00, b'A', b'B', b'C']).unwrap(), "ABC");
        assert_eq!(parse(&[0x03, 0x00, 0x00, 0x00, b'A', b'B', b'C', b'D']).unwrap(), "ABC");
        // 48: count 200,001 fails.
        let mut big = vec![0x00, 0x00, 0x00, 0x00];
        let count: u32 = 200_001;
        big[0..4].copy_from_slice(&count.to_le_bytes()); // encoding 0, count 200001
        assert!(matches!(parse(&big), Err(Error::Corrupt { .. })));
        assert!(!is_ltext(&big));
    }

    #[test]
    fn quirks() {
        assert!(matches!(parse(&[1, 0]), Err(Error::Truncated { .. })));
        // UTF-16 truncated: count 3 but only one unit present.
        assert!(matches!(parse(&[3, 0, 0, 0x10, b'a', 0]), Err(Error::Truncated { .. })));
        // trailing bytes ignored
        assert_eq!(parse(&[1, 0, 0, 0x10, b'a', 0, 0, 0]).unwrap(), "a");
        // lone surrogate
        assert_eq!(parse(&[1, 0, 0, 0x10, 0x00, 0xD8]).unwrap(), "\u{FFFD}");
        // [choice] quirk 1 does not exist: an 8-bit record whose second byte is 0 stays 8-bit.
        // `02 00 00 00 41 00` = 2 bytes "A\0"; the NUL is kept, not re-decoded as UTF-16.
        assert_eq!(parse(&[2, 0, 0, 0, b'A', 0]).unwrap(), "A\0");
        // §10.6 test 47: `ABCDE` has no header; its encoding byte (0x44) selects the legacy path in the
        // original. Standalone [choice] it is an error, and is_ltext rejects it. (Here the count field alone,
        // 0x434241, is over 200,000, so it fails that check first — either way it is rejected.)
        let legacy = [b'A', b'B', b'C', b'D', b'E'];
        assert!(matches!(parse(&legacy), Err(Error::Corrupt { .. })));
        assert!(!is_ltext(&legacy));
        // An encoding byte that is neither 0x00 nor 0x10, with a small count, is rejected on the encoding.
        assert!(matches!(parse(&[1, 0, 0, 0x44, b'A']), Err(Error::Corrupt { .. })));
        assert!(!is_ltext(&[1, 0, 0, 0x44, b'A']));
        // Other data under the same type (e.g. `XA` audio): `XA\0\0` reads as encoding 0, count 16728, so it is
        // a truncated 8-bit string, not LTEXT.
        let xa = [b'X', b'A', 0, 0, 1, 2];
        assert!(matches!(parse(&xa), Err(Error::Truncated { .. })));
        assert!(!is_ltext(&xa));
        assert!(!is_ltext(&[3, 0, 0, 0x10, b'a', 0]));
        // 8-bit form: trailing bytes are allowed now (the length need not match the record size).
        let mut temp = vec![8, 0, 0, 0];
        temp.extend_from_slice(b"Temp.png");
        assert!(is_ltext(&temp));
        assert_eq!(parse(&temp).unwrap(), "Temp.png");
        // A longer record than the count: still fine, the extra bytes are ignored.
        assert!(is_ltext(&[2, 0, 0, 0, b'a', b'b', b'c']));
        assert_eq!(parse(&[2, 0, 0, 0, b'a', b'b', b'c']).unwrap(), "ab");
        // 8-bit Latin-1 (0xE9 = é, not valid UTF-8 alone)
        assert_eq!(parse(&[2, 0, 0, 0, 0xE9, b't']).unwrap(), "\u{e9}t");
    }

    /// §6.1: the shipped 8-bit record (`0x2026960B`, `0x6A231EAA`, `0x0A6DF4DF`) begins "Don't localize." with
    /// header `90 00 00 00` (encoding 0, count 144). Rebuilt here as a fixture; the game-data sweep checks the
    /// real record.
    #[test]
    fn shipped_8bit_form() {
        let text = b"Don't localize. The rest of this record is padding to reach the shipped length.";
        let mut data = Vec::new();
        data.extend_from_slice(&(text.len() as u32).to_le_bytes()); // encoding 0x00, count = len
        data.extend_from_slice(text);
        assert!(is_ltext(&data));
        let parsed = parse(&data).unwrap();
        assert!(parsed.starts_with("Don't localize."));
    }
}
