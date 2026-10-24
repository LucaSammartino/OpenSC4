// SPDX-License-Identifier: GPL-3.0-or-later
//! Watercraft §1: only version 2 and the automaton base are serialized.

use crate::automata::AutomatonBase;
use crate::stream::{InStream, OutStream};

pub const CLASS_ID: u32 = 0x096E_75F5;
pub const RECORD_VERSION: u16 = 2;

#[derive(Clone, Debug, PartialEq)]
pub struct WatercraftRecord {
    pub base: AutomatonBase,
}

impl WatercraftRecord {
    pub fn write(&self, stream: &mut dyn OutStream) -> bool {
        let _ = stream.put_u16(RECORD_VERSION);
        self.base.write(stream)
    }

    /// Read never consults city geometry or allocates runtime resources.
    /// Only version 2 delegates to the base reader (watercraft §3).
    pub fn read(stream: &mut dyn InStream) -> Option<Self> {
        if stream.get_u16()? != RECORD_VERSION {
            return None;
        }
        Some(Self {
            base: AutomatonBase::read(stream)?,
        })
    }
}
