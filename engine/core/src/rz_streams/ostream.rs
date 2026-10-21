// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cRZOStream` (§1) and `cRZNullOStream` (§3).

use super::com::{StreamCore, IID_GZ_OSTREAM};
use super::{c_str, GzSerializable};

/// The `cIGZOStream` slots 3-18 plus access to the shared object (§1.3).
/// String arguments are byte slices: `set_rz_char_str` takes a C string (it
/// stops at the first NUL, like `strlen`), `set_gz_str` takes the string's
/// `Strlen()` bytes as `ToChar()` returns them (embedded NULs included).
pub trait GzOStream {
    fn core(&self) -> &StreamCore;
    fn core_mut(&mut self) -> &mut StreamCore;
    fn flush(&mut self);
    fn set_sint8(&mut self, v: i8) -> bool;
    fn set_uint8(&mut self, v: u8) -> bool;
    fn set_sint16(&mut self, v: i16) -> bool;
    fn set_uint16(&mut self, v: u16) -> bool;
    fn set_sint32(&mut self, v: i32) -> bool;
    fn set_uint32(&mut self, v: u32) -> bool;
    fn set_sint64(&mut self, v: i64) -> bool;
    fn set_uint64(&mut self, v: u64) -> bool;
    fn set_float32(&mut self, f: f32) -> bool;
    fn set_float64(&mut self, d: f64) -> bool;
    fn set_rz_char_str(&mut self, s: &[u8]) -> bool;
    fn set_gz_str(&mut self, s: &[u8]) -> bool;
    fn set_void(&mut self, p: &[u8]) -> bool;
    fn get_error(&self) -> bool;
    /// Slot 16 (shared by every class here): `o.Write(*this)`, its result.
    fn set_gz_serializable(&mut self, o: &mut dyn GzSerializable) -> bool
    where
        Self: Sized,
    {
        o.write(self)
    }
}

/// The value's little-endian bytes, reversed when the flag is false (§1.4).
pub(crate) fn ordered<const N: usize>(mut le: [u8; N], little_endian: bool) -> [u8; N] {
    if !little_endian {
        le.reverse();
    }
    le
}

// --- the base bodies (§1.3-§1.5), shared by every binary owner -----------

pub(crate) fn base_set_int<const N: usize>(s: &mut dyn GzOStream, le: [u8; N]) -> bool {
    let b = ordered(le, s.core().little_endian());
    s.set_void(&b)
}

/// `EncodeStringLength` (§1.5): unsigned LEB128 through the virtual
/// `SetUint8`; the bytes' results are ignored *[quirk]*.
pub(crate) fn encode_string_length(s: &mut dyn GzOStream, mut len: u32) {
    loop {
        let group = (len & 0x7F) as u8;
        len >>= 7;
        if len == 0 {
            s.set_uint8(group);
            return;
        }
        s.set_uint8(group | 0x80);
    }
}

/// Base `SetRZCharStr` / `SetGZStr` after the length is known (§1.5): an
/// empty string returns true whatever the sink did; else the final
/// `SetVoid`'s result alone.
pub(crate) fn base_set_string(s: &mut dyn GzOStream, bytes: &[u8]) -> bool {
    encode_string_length(s, bytes.len() as u32);
    if bytes.is_empty() {
        true
    } else {
        s.set_void(bytes)
    }
}

/// The derived class's pure slots of `cRZOStream`: `Flush` (3), `SetVoid`
/// (17), `GetError` (18).
pub trait ByteSink {
    fn flush(&mut self);
    fn set_void(&mut self, p: &[u8]) -> bool;
    fn get_error(&self) -> bool;
}

/// `cRZOStream` over a typed sink.
pub struct RzOStream<S: ByteSink> {
    core: StreamCore,
    sink: S,
}

impl<S: ByteSink> RzOStream<S> {
    /// `cRZOStream(bool littleEndian)` (§1.2).
    pub fn new(little_endian: bool, sink: S) -> Self {
        RzOStream {
            core: StreamCore::new(little_endian, IID_GZ_OSTREAM),
            sink,
        }
    }
    pub fn sink(&self) -> &S {
        &self.sink
    }
    pub fn sink_mut(&mut self) -> &mut S {
        &mut self.sink
    }
}

impl<S: ByteSink> GzOStream for RzOStream<S> {
    fn core(&self) -> &StreamCore {
        &self.core
    }
    fn core_mut(&mut self) -> &mut StreamCore {
        &mut self.core
    }
    fn flush(&mut self) {
        self.sink.flush()
    }
    fn set_sint8(&mut self, v: i8) -> bool {
        self.set_void(&[v as u8])
    }
    fn set_uint8(&mut self, v: u8) -> bool {
        self.set_void(&[v])
    }
    fn set_sint16(&mut self, v: i16) -> bool {
        base_set_int(self, v.to_le_bytes())
    }
    fn set_uint16(&mut self, v: u16) -> bool {
        base_set_int(self, v.to_le_bytes())
    }
    fn set_sint32(&mut self, v: i32) -> bool {
        base_set_int(self, v.to_le_bytes())
    }
    fn set_uint32(&mut self, v: u32) -> bool {
        base_set_int(self, v.to_le_bytes())
    }
    fn set_sint64(&mut self, v: i64) -> bool {
        base_set_int(self, v.to_le_bytes())
    }
    fn set_uint64(&mut self, v: u64) -> bool {
        base_set_int(self, v.to_le_bytes())
    }
    /// Never reversed *[quirk]* (§1.4).
    fn set_float32(&mut self, f: f32) -> bool {
        self.set_void(&f.to_le_bytes())
    }
    fn set_float64(&mut self, d: f64) -> bool {
        self.set_void(&d.to_le_bytes())
    }
    fn set_rz_char_str(&mut self, s: &[u8]) -> bool {
        base_set_string(self, c_str(s))
    }
    fn set_gz_str(&mut self, s: &[u8]) -> bool {
        base_set_string(self, s)
    }
    fn set_void(&mut self, p: &[u8]) -> bool {
        self.sink.set_void(p)
    }
    fn get_error(&self) -> bool {
        self.sink.get_error()
    }
}

/// `cRZNullOStream`'s slots (§3): `Flush` nothing, `SetVoid` true,
/// `GetError` false.
#[derive(Default, Clone, Copy, Debug)]
pub struct NullSink;

impl ByteSink for NullSink {
    fn flush(&mut self) {}
    fn set_void(&mut self, _p: &[u8]) -> bool {
        true
    }
    fn get_error(&self) -> bool {
        false
    }
}

/// `cRZNullOStream` (§3).
pub type RzNullOStream = RzOStream<NullSink>;

impl RzNullOStream {
    /// The constructor: count 0, no user data, `littleEndian` **false**.
    pub fn new_null() -> Self {
        RzOStream::new(false, NullSink)
    }
}
