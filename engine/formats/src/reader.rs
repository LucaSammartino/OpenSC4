// SPDX-License-Identifier: GPL-3.0-or-later
//! Bounds-checked little-endian cursor over a byte slice.

use crate::error::{Error, Result};

pub(crate) struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    what: &'static str,
}

impl<'a> Reader<'a> {
    pub(crate) fn new(data: &'a [u8], what: &'static str) -> Self {
        Reader { data, pos: 0, what }
    }

    pub(crate) fn pos(&self) -> usize {
        self.pos
    }

    pub(crate) fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    pub(crate) fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        if n > self.remaining() {
            return Err(Error::Truncated {
                what: self.what,
                needed: (self.pos as u64) + n as u64,
                available: self.data.len() as u64,
            });
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut a = [0u8; N];
        a.copy_from_slice(self.bytes(N)?);
        Ok(a)
    }

    pub(crate) fn u8(&mut self) -> Result<u8> {
        Ok(self.array::<1>()?[0])
    }
    pub(crate) fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.array()?))
    }
    pub(crate) fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    pub(crate) fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    pub(crate) fn i16(&mut self) -> Result<i16> {
        Ok(i16::from_le_bytes(self.array()?))
    }
    pub(crate) fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.array()?))
    }

    /// Checks that `count` elements of `elem` bytes fit in the rest of the data, before anything is allocated.
    pub(crate) fn ensure(&self, count: u64, elem: u64) -> Result<()> {
        let need = count.saturating_mul(elem);
        if need > self.remaining() as u64 {
            return Err(Error::Truncated {
                what: self.what,
                needed: (self.pos as u64).saturating_add(need),
                available: self.data.len() as u64,
            });
        }
        Ok(())
    }

    /// Reads a `GetGZStr` string (foundations §2.3), returning its bytes.
    ///
    /// ```text
    /// uint32  header      encoding = header >> 24, count = header & 0x00FF_FFFF
    ///         encoding 0x00: count bytes of 8-bit text (returned unchanged)
    ///         encoding 0x10: count UTF-16LE code units, converted to UTF-8
    /// ```
    ///
    /// Bytes after the string are left in the reader (the length need not match the record size). Standalone
    /// *\[choice\]* (foundations §2.3): the §2.3 quirks are not reproduced — a header with fewer than 4 bytes
    /// left is a truncation error (quirk 2), an encoding byte other than 0 and 0x10 is an error rather than the
    /// legacy raw-bytes path (quirk 3), and the "second byte 0 means UTF-16" rule does not exist (quirk 1: the
    /// header decides). An unpaired UTF-16 surrogate becomes U+FFFD. `count` over 200,000 fails.
    pub(crate) fn gzstr(&mut self) -> Result<Vec<u8>> {
        /// Largest string the original reader accepts (foundations §2.3).
        const MAX_COUNT: usize = 200_000;
        let header = self.u32()?;
        let encoding = header >> 24;
        let count = (header & 0x00FF_FFFF) as usize;
        if count > MAX_COUNT {
            return Err(Error::Corrupt {
                what: self.what,
                detail: format!("string count {count} exceeds {MAX_COUNT}"),
            });
        }
        match encoding {
            0x00 => Ok(self.bytes(count)?.to_vec()),
            0x10 => {
                let raw = self.bytes(count.checked_mul(2).ok_or_else(|| Error::Corrupt {
                    what: self.what,
                    detail: format!("UTF-16 count {count} overflows"),
                })?)?;
                let units = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]));
                Ok(char::decode_utf16(units)
                    .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
                    .collect::<String>()
                    .into_bytes())
            }
            other => Err(Error::Corrupt {
                what: self.what,
                detail: format!("string encoding byte {other:#04X} (only 0x00 and 0x10 are read)"),
            }),
        }
    }
}

pub(crate) fn le_u32(d: &[u8], off: usize) -> u32 {
    let mut a = [0u8; 4];
    a.copy_from_slice(&d[off..off + 4]);
    u32::from_le_bytes(a)
}
