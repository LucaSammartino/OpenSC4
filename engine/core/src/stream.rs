// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Serialization streams (`docs/specs/foundations.md` §2).
//!
//! Records are written and read through the [`OutStream`] / [`InStream`]
//! traits, so the same record code runs against:
//! - [`ByteWriter`] / [`ByteReader`]: byte-exact, standalone implementations
//!   (tests, the class-object store of [`crate::store`], standalone mode);
//! - host adapters (the GZCOM shim wraps the game's own streams).
//!
//! Both sides follow the stream model of §2.1: a single sticky "good" flag;
//! every operation fails without effect once it is clear; reads are atomic
//! (a read needing more bytes than remain fails, consumes nothing and leaves
//! the destination unchanged).

use std::any::Any;
use std::collections::HashMap;
use std::rc::Rc;

/// Maximum string length in bytes / code units (§2.3).
pub const MAX_STRING_LEN: u32 = 200_000;

/// A serializable object (one with a class ID, written into the class-object
/// store and referred to by references, §2.7).
pub trait SaveObject: Any {
    /// The class ID (`GetGZCLSID`).
    fn class_id(&self) -> u32;
    /// Writes the object's payload. Returns false only on a stream error (§3.5).
    fn write(&self, out: &mut dyn OutStream) -> bool;
    /// Reads the object's payload. Interior mutability: objects are shared.
    fn read(&self, inp: &mut dyn InStream) -> bool;
    /// For downcasting a shared object back to its concrete type.
    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any>;
    /// Host identity of the object (the GZCOM shim's wrapper), 0 if none.
    fn host_token(&self) -> u64 {
        0
    }
}

/// A reference as seen by record code (§2.7).
#[derive(Clone)]
pub enum RefValue {
    Null,
    /// An object implemented in the core.
    Object(Rc<dyn SaveObject>),
    /// An object owned by the host (e.g. a game property holder). The token
    /// is opaque to the core; `class_id` is used when writing.
    Host {
        token: u64,
        class_id: u32,
    },
}

impl RefValue {
    pub fn is_null(&self) -> bool {
        matches!(self, RefValue::Null)
    }

    /// Downcasts an `Object` reference to a concrete core type.
    pub fn object<T: SaveObject>(&self) -> Option<Rc<T>> {
        match self {
            RefValue::Object(o) => o.clone().as_any_rc().downcast::<T>().ok(),
            _ => None,
        }
    }
}

impl std::fmt::Debug for RefValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefValue::Null => write!(f, "Null"),
            RefValue::Object(o) => write!(f, "Object(class 0x{:08X})", o.class_id()),
            RefValue::Host { token, class_id } => write!(f, "Host(token {token:#x}, class 0x{class_id:08X})"),
        }
    }
}

/// A three-part resource key (§2.5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ResKey {
    pub type_id: u32,
    pub group: u32,
    pub instance: u32,
}

/// Write side of a stream (`cIGZOStream` plus the segment-stream additions).
pub trait OutStream {
    fn put_u8(&mut self, v: u8) -> bool;
    fn put_u16(&mut self, v: u16) -> bool;
    fn put_u32(&mut self, v: u32) -> bool;
    fn put_u64(&mut self, v: u64) -> bool;
    fn put_f32(&mut self, v: f32) -> bool;
    fn put_f64(&mut self, v: f64) -> bool;
    /// `SetVoid`: raw bytes, no length prefix.
    fn put_bytes(&mut self, v: &[u8]) -> bool;
    /// `SetGZStr` (§2.3): u32 byte length (top byte 0) + bytes.
    fn put_str(&mut self, v: &[u8]) -> bool;
    /// `WriteGZSerializable` (§2.7).
    fn put_ref(&mut self, r: &RefValue) -> bool;
    /// 0 while good, non-zero after a failure (sticky).
    fn error(&self) -> i32;
    /// Puts the stream into its (sticky) error state, for writers that fail
    /// after emitting part of a value (variants, foundations §9.2). Host
    /// streams without such a call ignore it.
    fn set_error(&mut self) {}

    fn put_i8(&mut self, v: i8) -> bool {
        self.put_u8(v as u8)
    }
    fn put_i16(&mut self, v: i16) -> bool {
        self.put_u16(v as u16)
    }
    fn put_i32(&mut self, v: i32) -> bool {
        self.put_u32(v as u32)
    }
    fn put_i64(&mut self, v: i64) -> bool {
        self.put_u64(v as u64)
    }
    /// Booleans are written as `SetUint8` 0/1 (§2.2).
    fn put_bool(&mut self, v: bool) -> bool {
        self.put_u8(u8::from(v))
    }
    /// `WriteResKey`: type, group, instance (§2.5).
    fn put_res_key(&mut self, k: ResKey) -> bool {
        self.put_u32(k.type_id) && self.put_u32(k.group) && self.put_u32(k.instance)
    }
    /// `SetGZSerializable`: the object writes itself inline (§2.6).
    fn put_inline(&mut self, obj: &dyn SaveObject) -> bool
    where
        Self: Sized,
    {
        obj.write(self) && self.error() == 0
    }
}

/// Read side of a stream (`cIGZIStream` plus the segment-stream additions).
/// Every getter returns `None` on failure (sticky).
pub trait InStream {
    fn get_u8(&mut self) -> Option<u8>;
    fn get_u16(&mut self) -> Option<u16>;
    fn get_u32(&mut self) -> Option<u32>;
    fn get_u64(&mut self) -> Option<u64>;
    fn get_f32(&mut self) -> Option<f32>;
    fn get_f64(&mut self) -> Option<f64>;
    /// `GetVoid`: exactly `n` raw bytes.
    fn get_bytes(&mut self, n: usize) -> Option<Vec<u8>>;
    /// `GetGZStr` (§2.3), as UTF-8 bytes.
    fn get_str(&mut self) -> Option<Vec<u8>>;
    /// `ReadGZSerializable` (§2.7).
    fn get_ref(&mut self) -> Option<RefValue>;
    /// A reference read with the interface the caller needs (the "usual
    /// helper semantics" of §2.7): a host object is `QueryInterface`d to
    /// `iid`. A non-null object without that interface reads as `Null` (the
    /// stream stays good). Host objects come with one reference owned by the
    /// caller.
    fn get_ref_as(&mut self, iid: u32) -> Option<RefValue> {
        let _ = iid;
        self.get_ref()
    }
    /// `Skip(n)`.
    fn skip(&mut self, n: usize) -> bool;
    fn error(&self) -> i32;
    /// Puts the stream into its (sticky) error state (foundations §9.2:
    /// unreadable variant tags). Host streams without such a call ignore it.
    fn set_error(&mut self) {}

    fn get_i8(&mut self) -> Option<i8> {
        self.get_u8().map(|v| v as i8)
    }
    fn get_i16(&mut self) -> Option<i16> {
        self.get_u16().map(|v| v as i16)
    }
    fn get_i32(&mut self) -> Option<i32> {
        self.get_u32().map(|v| v as i32)
    }
    fn get_i64(&mut self) -> Option<i64> {
        self.get_u64().map(|v| v as i64)
    }
    /// [choice] any non-zero byte reads as true (§2.2).
    fn get_bool(&mut self) -> Option<bool> {
        self.get_u8().map(|v| v != 0)
    }
    fn get_res_key(&mut self) -> Option<ResKey> {
        Some(ResKey {
            type_id: self.get_u32()?,
            group: self.get_u32()?,
            instance: self.get_u32()?,
        })
    }
}

// ---- references: save-side registry (§2.7) ----------------------------------------------------

/// One registration: (class ID, object ID, the object if it is a core object,
/// the host token if it is a host object).
pub type RegisteredObject = (u32, u32, Option<Rc<dyn SaveObject>>, u64);

/// Assigns object IDs during one save: the same object always gets the same
/// ID; [choice] IDs are 1, 2, 3, … in registration order (§2.7).
#[derive(Default)]
pub struct SaveRegistry {
    next_id: u32,
    by_identity: HashMap<usize, u32>,
    /// (class, id, object) in registration order; host objects have `None`.
    pub entries: Vec<RegisteredObject>,
}

impl SaveRegistry {
    pub fn new() -> Self {
        SaveRegistry {
            next_id: 1,
            ..Default::default()
        }
    }

    fn identity(r: &RefValue) -> usize {
        match r {
            RefValue::Object(o) => Rc::as_ptr(o) as *const () as usize,
            RefValue::Host { token, .. } => (*token as usize) ^ (1usize << (usize::BITS - 1)),
            RefValue::Null => 0,
        }
    }

    /// Registers the referenced object (once) and returns (id, class).
    pub fn register(&mut self, r: &RefValue) -> Option<(u32, u32)> {
        let class = match r {
            RefValue::Null => return None,
            RefValue::Object(o) => o.class_id(),
            RefValue::Host { class_id, .. } => *class_id,
        };
        let key = Self::identity(r);
        if let Some(&id) = self.by_identity.get(&key) {
            return Some((id, class));
        }
        let id = self.next_id;
        self.next_id += 1;
        self.by_identity.insert(key, id);
        let (obj, token) = match r {
            RefValue::Object(o) => (Some(o.clone()), 0),
            RefValue::Host { token, .. } => (None, *token),
            RefValue::Null => (None, 0),
        };
        self.entries.push((class, id, obj, token));
        Some((id, class))
    }
}

/// Resolves `(classID, objectID)` while loading (§2.7 "Reading").
pub trait RefResolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue>;
}

// ---- byte writer ------------------------------------------------------------------------------------

/// A byte-exact `cIGZOStream` (§2.2). References need a [`SaveRegistry`].
pub struct ByteWriter {
    buf: Vec<u8>,
    good: bool,
    registry: Option<Rc<std::cell::RefCell<SaveRegistry>>>,
}

impl Default for ByteWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteWriter {
    pub fn new() -> Self {
        ByteWriter {
            buf: Vec::new(),
            good: true,
            registry: None,
        }
    }

    pub fn with_registry(registry: Rc<std::cell::RefCell<SaveRegistry>>) -> Self {
        ByteWriter {
            buf: Vec::new(),
            good: true,
            registry: Some(registry),
        }
    }

    pub fn bytes(&self) -> &[u8] {
        &self.buf
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Store support: truncate to `len` (discarding an entry, §3.5).
    pub fn truncate(&mut self, len: usize) {
        self.buf.truncate(len);
    }

    /// Store support: overwrite 4 bytes at `pos`.
    pub fn patch_u32(&mut self, pos: usize, v: u32) {
        self.buf[pos..pos + 4].copy_from_slice(&v.to_le_bytes());
    }

    pub fn is_good(&self) -> bool {
        self.good
    }

    fn emit(&mut self, data: &[u8]) -> bool {
        if !self.good {
            return false;
        }
        self.buf.extend_from_slice(data);
        true
    }
}

impl OutStream for ByteWriter {
    fn put_u8(&mut self, v: u8) -> bool {
        self.emit(&[v])
    }
    fn put_u16(&mut self, v: u16) -> bool {
        self.emit(&v.to_le_bytes())
    }
    fn put_u32(&mut self, v: u32) -> bool {
        self.emit(&v.to_le_bytes())
    }
    fn put_u64(&mut self, v: u64) -> bool {
        self.emit(&v.to_le_bytes())
    }
    fn put_f32(&mut self, v: f32) -> bool {
        self.emit(&v.to_bits().to_le_bytes())
    }
    fn put_f64(&mut self, v: f64) -> bool {
        self.emit(&v.to_bits().to_le_bytes())
    }
    fn put_bytes(&mut self, v: &[u8]) -> bool {
        self.emit(v)
    }
    fn put_str(&mut self, v: &[u8]) -> bool {
        // Writers must keep strings NUL-free and at most 200,000 bytes (§2.3).
        // [choice] enforce it by cutting at the first NUL and at the limit.
        let end = v
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(v.len())
            .min(MAX_STRING_LEN as usize);
        let s = &v[..end];
        self.put_u32(s.len() as u32) && self.emit(s)
    }
    fn put_ref(&mut self, r: &RefValue) -> bool {
        if !self.good {
            return false;
        }
        if r.is_null() {
            return self.put_u32(0);
        }
        let Some(reg) = &self.registry else {
            // A reference needs the class-object store (segment stream).
            self.good = false;
            return false;
        };
        let Some((id, class)) = reg.borrow_mut().register(r) else {
            return self.put_u32(0);
        };
        self.put_u32(id) && self.put_u32(class)
    }
    fn error(&self) -> i32 {
        i32::from(!self.good)
    }
    fn set_error(&mut self) {
        self.good = false;
    }
}

// ---- byte reader --------------------------------------------------------------------------------------

/// A byte-exact `cIGZIStream` over one record (§2.1, §2.2, §2.3).
///
/// Standalone-mode semantics [choice] (§1, §2.3): the read quirks of the
/// original are errors (short string header, legacy string encodings) and the
/// "second byte is 0 means UTF-16" quirk does not exist.
pub struct ByteReader<'a> {
    data: &'a [u8],
    pos: usize,
    good: bool,
    resolver: Option<&'a mut dyn RefResolver>,
}

impl<'a> ByteReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        ByteReader {
            data,
            pos: 0,
            good: true,
            resolver: None,
        }
    }

    pub fn with_resolver(data: &'a [u8], resolver: &'a mut dyn RefResolver) -> Self {
        ByteReader {
            data,
            pos: 0,
            good: true,
            resolver: Some(resolver),
        }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.pos
    }

    /// Store support: re-synchronise at an entry boundary (§3.5).
    pub fn seek(&mut self, pos: usize) {
        self.pos = pos.min(self.data.len());
    }

    pub fn is_good(&self) -> bool {
        self.good
    }

    /// Takes `n` bytes atomically: on shortage nothing is consumed and the
    /// stream enters the error state.
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if !self.good {
            return None;
        }
        if self.remaining() < n {
            self.good = false;
            return None;
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Some(s)
    }

    fn take_array<const N: usize>(&mut self) -> Option<[u8; N]> {
        let s = self.take(N)?;
        let mut a = [0u8; N];
        a.copy_from_slice(s);
        Some(a)
    }

    /// `GetRZCharStr(buf, maxBytes)` (§2.4): reads a string and copies
    /// `min(len + 1, max_bytes)` bytes including the NUL if it fits; a
    /// truncated result is not NUL-terminated. Returns the copied bytes.
    pub fn get_rz_char_str(&mut self, max_bytes: usize) -> Option<Vec<u8>> {
        let mut s = self.get_str()?;
        s.push(0);
        s.truncate(max_bytes);
        Some(s)
    }
}

impl InStream for ByteReader<'_> {
    fn get_u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }
    fn get_u16(&mut self) -> Option<u16> {
        self.take_array().map(u16::from_le_bytes)
    }
    fn get_u32(&mut self) -> Option<u32> {
        self.take_array().map(u32::from_le_bytes)
    }
    fn get_u64(&mut self) -> Option<u64> {
        self.take_array().map(u64::from_le_bytes)
    }
    fn get_f32(&mut self) -> Option<f32> {
        self.take_array().map(|b| f32::from_bits(u32::from_le_bytes(b)))
    }
    fn get_f64(&mut self) -> Option<f64> {
        self.take_array().map(|b| f64::from_bits(u64::from_le_bytes(b)))
    }
    fn get_bytes(&mut self, n: usize) -> Option<Vec<u8>> {
        self.take(n).map(<[u8]>::to_vec)
    }
    fn get_str(&mut self) -> Option<Vec<u8>> {
        let header = self.get_u32()?;
        let encoding = header >> 24;
        let count = header & 0x00FF_FFFF;
        if count > MAX_STRING_LEN {
            self.good = false;
            return None;
        }
        match encoding {
            0x00 => self.take(count as usize).map(<[u8]>::to_vec),
            0x10 => {
                let raw = self.take(count as usize * 2)?;
                let units: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
                Some(String::from_utf16_lossy(&units).into_bytes())
            }
            _ => {
                // [choice] legacy encodings are a read error in standalone mode.
                self.good = false;
                None
            }
        }
    }
    fn get_ref(&mut self) -> Option<RefValue> {
        let object_id = self.get_u32()?;
        if object_id == 0 {
            return Some(RefValue::Null);
        }
        let class_id = self.get_u32()?;
        let resolved = match self.resolver.as_deref_mut() {
            Some(r) => r.resolve(class_id, object_id),
            None => None,
        };
        match resolved {
            Some(v) => Some(v),
            None => {
                // Unknown class or creation failure (§2.7): stream error.
                self.good = false;
                None
            }
        }
    }
    fn skip(&mut self, n: usize) -> bool {
        self.take(n).is_some()
    }
    fn error(&self) -> i32 {
        i32::from(!self.good)
    }
    fn set_error(&mut self) {
        self.good = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives_are_little_endian() {
        let mut w = ByteWriter::new();
        assert!(w.put_u32(0x1122_3344));
        assert!(w.put_f32(1.0));
        assert!(w.put_f64(1.0));
        assert!(w.put_i16(-2));
        assert_eq!(
            w.bytes(),
            &[0x44, 0x33, 0x22, 0x11, 0x00, 0x00, 0x80, 0x3F, 0, 0, 0, 0, 0, 0, 0xF0, 0x3F, 0xFE, 0xFF]
        );
    }

    #[test]
    fn sticky_atomic_reads() {
        let data = [1, 0, 0, 0, 7, 8];
        let mut r = ByteReader::new(&data);
        assert_eq!(r.get_u32(), Some(1));
        assert_eq!(r.get_u32(), None); // only 2 bytes left: fails, consumes nothing
        assert_eq!(r.position(), 4);
        assert_ne!(r.error(), 0);
        assert_eq!(r.get_u8(), None); // sticky
    }

    #[test]
    fn strings() {
        let mut w = ByteWriter::new();
        assert!(w.put_str(b"Fire Dept."));
        assert!(w.put_str(b""));
        assert_eq!(
            w.bytes(),
            &[0x0A, 0, 0, 0, 0x46, 0x69, 0x72, 0x65, 0x20, 0x44, 0x65, 0x70, 0x74, 0x2E, 0, 0, 0, 0]
        );
        let bytes = w.into_bytes();
        let mut r = ByteReader::new(&bytes);
        assert_eq!(r.get_str().as_deref(), Some(&b"Fire Dept."[..]));
        assert_eq!(r.get_str().as_deref(), Some(&b""[..]));

        let utf16 = [0x02, 0, 0, 0x10, 0x41, 0x00, 0xE9, 0x00];
        assert_eq!(ByteReader::new(&utf16).get_str(), Some("Aé".as_bytes().to_vec()));

        let too_long = (MAX_STRING_LEN + 1).to_le_bytes();
        let mut r = ByteReader::new(&too_long);
        assert_eq!(r.get_str(), None);
        assert_ne!(r.error(), 0);

        let mut w = ByteWriter::new();
        w.put_str(b"Fire Dept.");
        let b = w.into_bytes();
        assert_eq!(ByteReader::new(&b).get_rz_char_str(4), Some(b"Fire".to_vec())); // no terminator
        assert_eq!(ByteReader::new(&b).get_rz_char_str(64), Some(b"Fire Dept.\0".to_vec()));
    }

    #[test]
    fn res_key() {
        let mut w = ByteWriter::new();
        w.put_res_key(ResKey {
            type_id: 1,
            group: 2,
            instance: 3,
        });
        assert_eq!(w.bytes(), &[1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0]);
    }
}
