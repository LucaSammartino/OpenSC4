// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cRZIStream` and `cRZNullIStream` (§5).

use super::com::{StreamCore, IID_GZ_ISTREAM};
use super::GzSerializable;

/// The `cIGZIStream` slots 3-18 plus access to the shared object (§5).
pub trait GzIStream {
    fn core(&self) -> &StreamCore;
    fn core_mut(&mut self) -> &mut StreamCore;
    fn skip(&mut self, n: u32) -> bool;
    fn get_sint8(&mut self, out: &mut i8) -> bool;
    fn get_uint8(&mut self, out: &mut u8) -> bool;
    fn get_sint16(&mut self, out: &mut i16) -> bool;
    fn get_uint16(&mut self, out: &mut u16) -> bool;
    fn get_sint32(&mut self, out: &mut i32) -> bool;
    fn get_uint32(&mut self, out: &mut u32) -> bool;
    fn get_sint64(&mut self, out: &mut i64) -> bool;
    fn get_uint64(&mut self, out: &mut u64) -> bool;
    fn get_float32(&mut self, out: &mut f32) -> bool;
    fn get_float64(&mut self, out: &mut f64) -> bool;
    /// `GetRZCharStr(buf, max)`: see [`base_get_rz_char_str`].
    fn get_rz_char_str(&mut self, buf: &mut [u8], max: u32) -> bool;
    /// `GetGZStr(out)`: `out` holds the string's bytes.
    fn get_gz_str(&mut self, out: &mut Vec<u8>) -> bool;
    fn get_void(&mut self, out: &mut [u8]) -> bool;
    fn get_error(&self) -> bool;
    /// Slot 16: `o.Read(*this)`, its result.
    fn get_gz_serializable(&mut self, o: &mut dyn GzSerializable) -> bool
    where
        Self: Sized,
    {
        o.read(self)
    }
}

/// Slots 6-11: `GetVoid(&out, N)` straight into `out`'s bytes; on success
/// reversed in place when the flag is false. On failure `out` keeps whatever
/// `GetVoid` left in it.
fn base_get_int<const N: usize>(s: &mut dyn GzIStream, cur: [u8; N]) -> (bool, [u8; N]) {
    let mut b = cur;
    if s.get_void(&mut b) {
        if !s.core().little_endian() {
            b.reverse();
        }
        (true, b)
    } else {
        (false, b)
    }
}

/// The length decoder (§5): bytes through the virtual `GetUint8` while bit 7
/// is set, seven bits each, least significant group first.
///
/// A failed `GetUint8` leaves its byte unwritten; we start each byte at 0, so
/// a failed read ends the loop (OS5). Groups past the eighth byte overran the
/// original's 8-byte buffer *[quirk]*; here they are consumed and ignored, as
/// are bits beyond 32.
pub fn decode_string_length(s: &mut dyn GzIStream) -> u32 {
    let mut len: u64 = 0;
    let mut i = 0u32;
    loop {
        let mut b = 0u8;
        s.get_uint8(&mut b);
        if i < 8 {
            len |= u64::from(b & 0x7F) << (7 * i);
        }
        i += 1;
        if b & 0x80 == 0 {
            return len as u32;
        }
    }
}

/// `GetRZCharStr(buf, max)` (§5): if `GetError()` or length 0, `buf[0] := 0`;
/// else read `k = min(length, max)` bytes into `buf`, `buf[k] := 0` (one past
/// `max` when truncated *[quirk]*), and `Skip(length - k)` if no error, also
/// when that is 0 (literal reading, OS6). Returns `GetError() == 0`.
///
/// In C a `buf` shorter than `k + 1` is overrun; here the bytes that do not
/// fit are read and dropped.
pub fn base_get_rz_char_str(s: &mut dyn GzIStream, buf: &mut [u8], max: u32) -> bool {
    let len = decode_string_length(s);
    if s.get_error() || len == 0 {
        if let Some(b) = buf.first_mut() {
            *b = 0;
        }
        return !s.get_error();
    }
    let k = len.min(max) as usize;
    let mut tmp = vec![0u8; k];
    let fit = k.min(buf.len());
    tmp[..fit].copy_from_slice(&buf[..fit]);
    s.get_void(&mut tmp);
    buf[..fit].copy_from_slice(&tmp[..fit]);
    if let Some(b) = buf.get_mut(k) {
        *b = 0;
    }
    if !s.get_error() {
        s.skip(len - k as u32);
    }
    !s.get_error()
}

/// `GetGZStr(out)` (§5): `out := ""`; the length; then chunks of at most 255
/// bytes while no error. A chunk whose read leaves an error is not appended
/// (our reading of "appended while no error", OS5). Returns `GetError() == 0`.
pub fn base_get_gz_str(s: &mut dyn GzIStream, out: &mut Vec<u8>) -> bool {
    out.clear();
    let mut rem = decode_string_length(s);
    while rem > 0 && !s.get_error() {
        let c = rem.min(255);
        let mut tmp = vec![0u8; c as usize];
        s.get_void(&mut tmp);
        if s.get_error() {
            break;
        }
        out.extend_from_slice(&tmp);
        rem -= c;
    }
    !s.get_error()
}

/// The derived class's pure slots of `cRZIStream`: `Skip` (3), `GetVoid`
/// (17), `GetError` (18). The transfer contract (atomic or partial) is the
/// source's.
pub trait ByteSource {
    fn skip(&mut self, n: u32) -> bool;
    fn get_void(&mut self, out: &mut [u8]) -> bool;
    fn get_error(&self) -> bool;
}

/// `cRZIStream` over a typed source.
pub struct RzIStream<S: ByteSource> {
    core: StreamCore,
    source: S,
}

impl<S: ByteSource> RzIStream<S> {
    /// `cRZIStream(bool littleEndian)`.
    pub fn new(little_endian: bool, source: S) -> Self {
        RzIStream {
            core: StreamCore::new(little_endian, IID_GZ_ISTREAM),
            source,
        }
    }
    pub fn source(&self) -> &S {
        &self.source
    }
    pub fn source_mut(&mut self) -> &mut S {
        &mut self.source
    }
}

macro_rules! get_int {
    ($name:ident, $t:ty) => {
        fn $name(&mut self, out: &mut $t) -> bool {
            let (ok, b) = base_get_int(self, out.to_le_bytes());
            *out = <$t>::from_le_bytes(b);
            ok
        }
    };
}

impl<S: ByteSource> GzIStream for RzIStream<S> {
    fn core(&self) -> &StreamCore {
        &self.core
    }
    fn core_mut(&mut self) -> &mut StreamCore {
        &mut self.core
    }
    fn skip(&mut self, n: u32) -> bool {
        self.source.skip(n)
    }
    fn get_sint8(&mut self, out: &mut i8) -> bool {
        let mut b = [*out as u8];
        let ok = self.get_void(&mut b);
        *out = b[0] as i8;
        ok
    }
    fn get_uint8(&mut self, out: &mut u8) -> bool {
        self.get_void(std::slice::from_mut(out))
    }
    get_int!(get_sint16, i16);
    get_int!(get_uint16, u16);
    get_int!(get_sint32, i32);
    get_int!(get_uint32, u32);
    get_int!(get_sint64, i64);
    get_int!(get_uint64, u64);
    /// Never reversed.
    fn get_float32(&mut self, out: &mut f32) -> bool {
        let mut b = out.to_le_bytes();
        let ok = self.get_void(&mut b);
        *out = f32::from_le_bytes(b);
        ok
    }
    fn get_float64(&mut self, out: &mut f64) -> bool {
        let mut b = out.to_le_bytes();
        let ok = self.get_void(&mut b);
        *out = f64::from_le_bytes(b);
        ok
    }
    fn get_rz_char_str(&mut self, buf: &mut [u8], max: u32) -> bool {
        base_get_rz_char_str(self, buf, max)
    }
    fn get_gz_str(&mut self, out: &mut Vec<u8>) -> bool {
        base_get_gz_str(self, out)
    }
    fn get_void(&mut self, out: &mut [u8]) -> bool {
        self.source.get_void(out)
    }
    fn get_error(&self) -> bool {
        self.source.get_error()
    }
}

/// `cRZNullIStream`'s slots: an error byte (0 at construction) that `Skip`
/// and `GetVoid` set to 1, returning false; `GetError` returns it.
#[derive(Default, Clone, Copy, Debug)]
pub struct NullSource {
    error: bool,
}

impl ByteSource for NullSource {
    fn skip(&mut self, _n: u32) -> bool {
        self.error = true;
        false
    }
    fn get_void(&mut self, _out: &mut [u8]) -> bool {
        self.error = true;
        false
    }
    fn get_error(&self) -> bool {
        self.error
    }
}

/// `cRZNullIStream` (§5).
pub type RzNullIStream = RzIStream<NullSource>;

impl RzNullIStream {
    /// The constructor. §5 does not name its byte order; it inherits
    /// `cRZIStream(bool)` and we pass false as `cRZNullOStream` does (OS6).
    pub fn new_null() -> Self {
        RzIStream::new(false, NullSource::default())
    }
}
