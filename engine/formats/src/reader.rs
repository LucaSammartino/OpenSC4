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
}

pub(crate) fn le_u32(d: &[u8], off: usize) -> u32 {
    let mut a = [0u8; 4];
    a.copy_from_slice(&d[off..off + 4]);
    u32::from_le_bytes(a)
}
