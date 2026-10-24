// SPDX-License-Identifier: GPL-3.0-or-later
//! LR12 isolated record observation: no Init and no reference resolution into live objects.
use super::Lot;
use crate::stream::{ByteReader, ByteWriter, OutStream, RefResolver, RefValue, SaveObject};

struct Identities;
impl RefResolver for Identities {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        Some(RefValue::Host {
            token: u64::from(object_id),
            class_id,
        })
    }
}

// Unlike a save registry, this writer preserves the observation's assigned IDs.
pub struct IdentityWriter(pub ByteWriter);
impl OutStream for IdentityWriter {
    fn put_u8(&mut self, v: u8) -> bool {
        self.0.put_u8(v)
    }
    fn put_u16(&mut self, v: u16) -> bool {
        self.0.put_u16(v)
    }
    fn put_u32(&mut self, v: u32) -> bool {
        self.0.put_u32(v)
    }
    fn put_u64(&mut self, v: u64) -> bool {
        self.0.put_u64(v)
    }
    fn put_f32(&mut self, v: f32) -> bool {
        self.0.put_f32(v)
    }
    fn put_f64(&mut self, v: f64) -> bool {
        self.0.put_f64(v)
    }
    fn put_bytes(&mut self, v: &[u8]) -> bool {
        self.0.put_bytes(v)
    }
    fn put_str(&mut self, v: &[u8]) -> bool {
        self.0.put_str(v)
    }
    fn put_ref(&mut self, v: &RefValue) -> bool {
        match v {
            RefValue::Null => self.0.put_u32(0),
            RefValue::Host { token, class_id } if *token <= u64::from(u32::MAX) => {
                self.0.put_u32(*token as u32) && self.0.put_u32(*class_id)
            }
            _ => {
                self.0.set_error();
                false
            }
        }
    }
    fn error(&self) -> i32 {
        self.0.error()
    }
    fn set_error(&mut self) {
        self.0.set_error();
    }
}

/// Full record re-encoding, preserving configuration metadata instead of calling
/// the live object's Write (which intentionally uses the Init configuration).
pub fn reencode(bytes: &[u8]) -> Option<Vec<u8>> {
    let lot = Lot::default();
    let mut ids = Identities;
    let mut input = ByteReader::with_resolver(bytes, &mut ids);
    if !lot.read(&mut input) || input.remaining() != 0 {
        return None;
    }
    let mut output = IdentityWriter(ByteWriter::new());
    if !lot.rec.borrow().write(&mut output) {
        return None;
    }
    Some(output.0.into_bytes())
}
