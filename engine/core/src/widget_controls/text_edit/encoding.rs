// SPDX-License-Identifier: GPL-3.0-or-later
//! Byte helpers from text-edit-encoding, deliberately independent of Unicode.
use super::super::{Result, Unsupported};

/// -1 is resolved once from the foreign system selector. -2 aliases 8 inside
/// the helpers, but does not enter the editor's pending-input gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Encoding {
    pub selector: i32,
    pub system_selector: i32,
}

impl Encoding {
    fn resolved(self) -> i32 {
        match self.selector {
            -2 => 8,
            -1 => self.system_selector,
            other => other,
        }
    }

    fn lead(self, byte: u8) -> bool {
        match self.resolved() {
            932 => (0x81..=0x9f).contains(&byte) || (0xe0..=0xfc).contains(&byte),
            936 => (0xa1..=0xfc).contains(&byte),
            949 => (0xb0..=0xc8).contains(&byte),
            950 => (0x81..=0xfe).contains(&byte),
            _ => false,
        }
    }

    /// `readable` can be longer than the logical span. The native helper may
    /// read its second byte even when logical_len is one, and may return more
    /// than logical_len. Missing readable memory is a precondition failure,
    /// never a fabricated replacement character.
    pub fn char_length(self, readable: &[u8], logical_len: usize) -> Result<usize> {
        if logical_len == 0 {
            return Ok(0);
        }
        if matches!(self.resolved(), 0 | 16) {
            return Ok(1); // Native single-byte branches do not read the buffer.
        }
        let first = *readable.first().ok_or(Unsupported::Precondition("TE1 first byte"))?;
        match self.resolved() {
            0 | 16 => Ok(1),
            8 => {
                let advertised = match first {
                    0xc0..=0xdf => 2,
                    0xe0..=0xef => 3,
                    0xf0..=0xf7 => 4,
                    0xf8..=0xfb => 5,
                    0xfc..=0xfd => 6,
                    _ => return Ok(1),
                };
                let second = *readable
                    .get(1)
                    .ok_or(Unsupported::Precondition("TE1 readable second byte"))?;
                Ok(if continuation(second) { advertised } else { 1 })
            }
            _ => Ok(if self.lead(first) { 2 } else { 1 }),
        }
    }

    pub fn previous_length(self, prefix: &[u8]) -> usize {
        if prefix.is_empty() {
            return 0;
        }
        match self.resolved() {
            0 | 16 => 1,
            8 => 1 + prefix.iter().rev().take_while(|&&b| continuation(b)).count(),
            _ => {
                let mut at = 0;
                let mut last = 1;
                while at < prefix.len() {
                    last = if self.lead(prefix[at]) && at + 1 < prefix.len() {
                        2
                    } else {
                        1
                    };
                    at += if self.lead(prefix[at]) { 2 } else { 1 };
                }
                last
            }
        }
    }

    pub fn glyph_count(self, bytes: &[u8]) -> usize {
        match self.resolved() {
            0 | 16 => bytes.len(),
            8 => bytes.iter().filter(|&&b| !continuation(b)).count(),
            _ => {
                let (mut at, mut count) = (0, 0);
                while at < bytes.len() {
                    at += if self.lead(bytes[at]) { 2 } else { 1 };
                    count += 1;
                }
                count
            }
        }
    }

    pub fn accumulates(self) -> bool {
        self.selector == 8 || self.selector >= 100
    }

    pub fn input_lead(self, byte: u8) -> bool {
        match self.resolved() {
            8 => (0xc0..=0xfd).contains(&byte),
            selector if selector >= 100 => self.lead(byte),
            _ => false,
        }
    }

    pub fn trailing_count(self, byte: u8) -> usize {
        match self.resolved() {
            8 => match byte {
                0xc2..=0xdf => 1,
                0xe0..=0xef => 2,
                0xf0..=0xf7 => 3,
                0xf8..=0xfb => 4,
                0xfc..=0xfd => 5,
                _ => 0,
            },
            selector if selector >= 100 => usize::from(self.lead(byte)),
            _ => 0,
        }
    }
}

fn continuation(byte: u8) -> bool {
    (0x80..=0xbf).contains(&byte)
}
