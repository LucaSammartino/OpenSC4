// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Variants (`cIGZVariant`) and their on-disk encoding, `docs/specs/foundations.md`
//! §9. The memory type numbers (§9.1) are what callers see; the disk tags
//! (§9.2) are a different numbering used only in streams.

use crate::stream::{InStream, OutStream, RefValue};

/// Marker that selects the object path (§9.2).
pub const OBJECT_MARKER: u32 = 0xA9BE_E9AC;

/// Array bit of memory types and disk tags.
pub const ARRAY: u16 = 0x8000;

/// cIGZVariant::GetCount metadata, separate from logical storage length.
/// Published data-formats §5.2: scalar 0, array element count (including 0/1).
/// `array_count` comes from the owned adapter's storage, never a native oracle.
pub fn native_count(mem_type: u16, array_count: usize) -> usize {
    if mem_type & ARRAY != 0 {
        array_count
    } else {
        0
    }
}

/// Memory types (§9.1).
pub mod mem {
    pub const EMPTY: u16 = 0x00;
    pub const BOOL: u16 = 0x01;
    pub const UINT8: u16 = 0x02;
    pub const SINT8: u16 = 0x03;
    pub const UINT16: u16 = 0x04;
    pub const SINT16: u16 = 0x05;
    pub const UINT32: u16 = 0x06;
    pub const SINT32: u16 = 0x07;
    pub const UINT64: u16 = 0x08;
    pub const SINT64: u16 = 0x09;
    pub const FLOAT32: u16 = 0x0A;
    pub const FLOAT64: u16 = 0x0B;
    pub const CHAR: u16 = 0x0C;
    pub const UNICODE_CHAR: u16 = 0x0D;
    pub const RZ_CHAR: u16 = 0x0E;
    pub const VOID_PTR: u16 = 0x0F;
    pub const IGZ_UNKNOWN: u16 = 0x10;
}

/// One element of a variant. The memory type follows from the variant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Elem {
    Bool(bool),
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    F32(f32),
    F64(f64),
    Char(u8),
    UnicodeChar(u16),
    RzChar(u8),
    /// A pointer value (never saveable).
    VoidPtr(u64),
}

impl Elem {
    pub fn mem_type(&self) -> u16 {
        match self {
            Elem::Bool(_) => mem::BOOL,
            Elem::U8(_) => mem::UINT8,
            Elem::I8(_) => mem::SINT8,
            Elem::U16(_) => mem::UINT16,
            Elem::I16(_) => mem::SINT16,
            Elem::U32(_) => mem::UINT32,
            Elem::I32(_) => mem::SINT32,
            Elem::U64(_) => mem::UINT64,
            Elem::I64(_) => mem::SINT64,
            Elem::F32(_) => mem::FLOAT32,
            Elem::F64(_) => mem::FLOAT64,
            Elem::Char(_) => mem::CHAR,
            Elem::UnicodeChar(_) => mem::UNICODE_CHAR,
            Elem::RzChar(_) => mem::RZ_CHAR,
            Elem::VoidPtr(_) => mem::VOID_PTR,
        }
    }

    /// The value as a 64-bit integer, for property readers: the int64 step of
    /// the converting getters (`docs/specs/data-formats.md` §5.1). Unsigned
    /// types are zero-extended, signed types sign-extended, Char and RZChar
    /// **sign-extended** as signed bytes (EX4), RZUnicodeChar zero-extended.
    /// Floats are truncated toward zero by `_ftol`, which does not saturate:
    /// NaN, ±∞ and magnitudes ≥ 2⁶³ give `0x8000000000000000` (EX3), so a
    /// caller keeping the low 32 bits gets 0, and larger values wrap.
    pub fn as_i64(&self) -> i64 {
        match *self {
            Elem::Bool(b) => i64::from(b),
            Elem::U8(v) => i64::from(v),
            Elem::Char(v) | Elem::RzChar(v) => i64::from(v as i8),
            Elem::I8(v) => i64::from(v),
            Elem::U16(v) | Elem::UnicodeChar(v) => i64::from(v),
            Elem::I16(v) => i64::from(v),
            Elem::U32(v) => i64::from(v),
            Elem::I32(v) => i64::from(v),
            Elem::U64(v) | Elem::VoidPtr(v) => v as i64,
            Elem::I64(v) => v,
            Elem::F32(v) => crate::x87::ftol(f64::from(v)),
            Elem::F64(v) => crate::x87::ftol(v),
        }
    }

    /// The value as a double, for property readers: the to-double step of the
    /// converting getters (data-formats §5.1). Signed types and a Uint32 below
    /// 2³¹ are exact (a Sint64 rounded to 53 bits); Char and RZChar are signed
    /// bytes (EX4); a Uint32 from 2³¹ and **every** Uint64 keep only 24
    /// significant bits, because the unsigned correction is an x87 add at the
    /// main thread's precision (EX2): Uint64 16,777,217 gives 16,777,216.0.
    pub fn as_f64(&self) -> f64 {
        match *self {
            Elem::F32(v) => f64::from(v),
            Elem::F64(v) => v,
            // One rounding of the exact value to 24 bits (nearest, ties to even).
            Elem::U64(v) => f64::from(v as f32),
            Elem::U32(v) => crate::x87::u32_to_float(v as i32),
            Elem::I64(v) => v as f64,
            _ => self.as_i64() as f64,
        }
    }
}

/// A variant value.
#[derive(Clone, Debug)]
pub enum Variant {
    Empty,
    /// `IGZUnknown` (memory type 0x10): an object reference, possibly null.
    Object(RefValue),
    Scalar(Elem),
    /// An array of memory type `0x8000 | elem`. `elem == 0` is the raw "void
    /// array" (its items are `U8`); `elem == RZ_CHAR` is a string.
    Array {
        elem: u16,
        items: Vec<Elem>,
    },
}

impl PartialEq for Variant {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Variant::Empty, Variant::Empty) => true,
            (Variant::Object(a), Variant::Object(b)) => match (a, b) {
                (RefValue::Null, RefValue::Null) => true,
                (
                    RefValue::Host {
                        token: t1,
                        class_id: c1,
                    },
                    RefValue::Host {
                        token: t2,
                        class_id: c2,
                    },
                ) => t1 == t2 && c1 == c2,
                (RefValue::Object(x), RefValue::Object(y)) => std::rc::Rc::ptr_eq(x, y),
                _ => false,
            },
            (Variant::Scalar(a), Variant::Scalar(b)) => a == b,
            (Variant::Array { elem: e1, items: i1 }, Variant::Array { elem: e2, items: i2 }) => e1 == e2 && i1 == i2,
            _ => false,
        }
    }
}

impl Variant {
    /// A string (an `RZChar` array, §9.1).
    pub fn string(bytes: &[u8]) -> Variant {
        Variant::Array {
            elem: mem::RZ_CHAR,
            items: bytes.iter().map(|&b| Elem::RzChar(b)).collect(),
        }
    }

    pub fn array(elem: u16, items: Vec<Elem>) -> Variant {
        Variant::Array { elem, items }
    }

    /// `GetType()` (§9.1).
    pub fn mem_type(&self) -> u16 {
        match self {
            Variant::Empty => mem::EMPTY,
            Variant::Object(_) => mem::IGZ_UNKNOWN,
            Variant::Scalar(e) => e.mem_type(),
            Variant::Array { elem, .. } => ARRAY | elem,
        }
    }

    /// Logical storage element count: one for scalars/objects. This is NOT
    /// cIGZVariant::GetCount; callers consuming value bytes still need one.
    pub fn count(&self) -> usize {
        match self {
            Variant::Empty => 0,
            Variant::Object(_) | Variant::Scalar(_) => 1,
            Variant::Array { items, .. } => items.len(),
        }
    }

    /// Native metadata count (data-formats §5.2, §10.5 item 42).
    pub fn raw_count(&self) -> usize {
        native_count(self.mem_type(), self.count())
    }

    /// The values (one for a scalar).
    pub fn items(&self) -> Vec<Elem> {
        match self {
            Variant::Scalar(e) => vec![*e],
            Variant::Array { items, .. } => items.clone(),
            _ => Vec::new(),
        }
    }

    /// The bytes of a string or of any 8-bit array.
    pub fn bytes(&self) -> Option<Vec<u8>> {
        match self {
            Variant::Array { items, .. } => items
                .iter()
                .map(|e| match *e {
                    Elem::U8(v) | Elem::Char(v) | Elem::RzChar(v) => Some(v),
                    _ => None,
                })
                .collect(),
            _ => None,
        }
    }

    /// `WriteVariant` (§9.2). Returns false (and leaves the stream in its
    /// error state) for types that cannot be saved.
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        if let Variant::Object(r) = self {
            return out.put_u32(OBJECT_MARKER) && out.put_ref(r) && out.error() == 0;
        }
        let ty = self.mem_type();
        let Some(tag) = disk_tag(ty) else {
            // IGZUnknown arrays have a tag but cannot be written; nothing else lands here.
            out.set_error();
            return false;
        };
        out.put_u32(0);
        out.put_u32(u32::from(tag));
        let ok = match self {
            Variant::Empty => true,
            Variant::Scalar(e) => put_elem(out, e),
            Variant::Array { elem, items } => {
                if *elem == mem::VOID_PTR || *elem == mem::IGZ_UNKNOWN {
                    false
                } else if *elem == mem::RZ_CHAR {
                    // §2.3 string: the header is the count; 8-bit text.
                    let bytes: Vec<u8> = items.iter().map(|e| e.as_i64() as u8).collect();
                    out.put_u32(bytes.len() as u32) && out.put_bytes(&bytes)
                } else {
                    out.put_u32(items.len() as u32) && items.iter().all(|e| put_elem(out, e))
                }
            }
            Variant::Object(_) => unreachable!(),
        };
        if !ok {
            out.set_error();
            return false;
        }
        out.error() == 0
    }

    /// `ReadVariant` (§9.2) into `dest`. On failure the stream is in its error
    /// state and `dest` is unchanged. A null object reference also leaves
    /// `dest` unchanged (and succeeds).
    pub fn read_into(inp: &mut dyn InStream, dest: &mut Variant) -> bool {
        match read_variant(inp) {
            Some(Read::Value(v)) => {
                *dest = v;
                true
            }
            Some(Read::NullObject) => true,
            None => {
                inp.set_error();
                false
            }
        }
    }

    /// Reads a variant; a null object reference reads as `Object(Null)`. Used
    /// by property readers, where the destination is fresh (§10.2).
    pub fn read(inp: &mut dyn InStream) -> Option<Variant> {
        match read_variant(inp) {
            Some(Read::Value(v)) => Some(v),
            Some(Read::NullObject) => Some(Variant::Object(RefValue::Null)),
            None => {
                inp.set_error();
                None
            }
        }
    }
}

enum Read {
    Value(Variant),
    NullObject,
}

/// Disk tag of a scalar memory type (§9.2); arrays add 0x8000. `None` for
/// memory types that have no tag.
fn scalar_disk_tag(t: u16) -> Option<u16> {
    Some(match t {
        mem::EMPTY => 0x00,
        mem::BOOL => 0x0B,
        mem::UINT8 => 0x01,
        mem::SINT8 => 0x05,
        mem::UINT16 => 0x02,
        mem::SINT16 => 0x06,
        mem::UINT32 => 0x03,
        mem::SINT32 => 0x07,
        mem::UINT64 => 0x04,
        mem::SINT64 => 0x08,
        mem::FLOAT32 => 0x09,
        mem::FLOAT64 => 0x0A,
        mem::CHAR => 0x11,
        mem::UNICODE_CHAR => 0x10,
        mem::RZ_CHAR => 0x0C,
        mem::VOID_PTR => 0x12,
        _ => return None,
    })
}

/// Disk tag of any memory type except `IGZUnknown` scalars (marker path).
pub fn disk_tag(mem_type: u16) -> Option<u16> {
    if mem_type & ARRAY != 0 {
        let t = mem_type & !ARRAY;
        if t == mem::IGZ_UNKNOWN {
            return Some(ARRAY | 0x0D);
        }
        return scalar_disk_tag(t).map(|d| ARRAY | d);
    }
    if mem_type == mem::IGZ_UNKNOWN {
        return None;
    }
    scalar_disk_tag(mem_type)
}

/// Memory type of a disk tag the reader accepts (§9.2 "Reader tag set").
fn readable_mem_type(tag: u32) -> Option<u16> {
    let scalar = |d: u32| -> Option<u16> {
        Some(match d {
            0x00 => mem::EMPTY,
            0x01 => mem::UINT8,
            0x02 => mem::UINT16,
            0x03 => mem::UINT32,
            0x04 => mem::UINT64,
            0x05 => mem::SINT8,
            0x06 => mem::SINT16,
            0x07 => mem::SINT32,
            0x08 => mem::SINT64,
            0x09 => mem::FLOAT32,
            0x0A => mem::FLOAT64,
            0x0B => mem::BOOL,
            0x0C => mem::RZ_CHAR,
            _ => return None,
        })
    };
    if tag & 0x8000 != 0 {
        let d = tag & !0x8000;
        if tag > 0xFFFF {
            return None;
        }
        return match d {
            0x10 => Some(ARRAY | mem::UNICODE_CHAR),
            0x11 => Some(ARRAY | mem::CHAR),
            _ => scalar(d).map(|t| ARRAY | t),
        };
    }
    scalar(tag)
}

fn elem_size(t: u16) -> usize {
    match t {
        mem::EMPTY | mem::BOOL | mem::UINT8 | mem::SINT8 | mem::CHAR | mem::RZ_CHAR => 1,
        mem::UINT16 | mem::SINT16 | mem::UNICODE_CHAR => 2,
        mem::UINT32 | mem::SINT32 | mem::FLOAT32 => 4,
        _ => 8,
    }
}

fn put_elem(out: &mut dyn OutStream, e: &Elem) -> bool {
    match *e {
        Elem::Bool(b) => out.put_bool(b),
        Elem::U8(v) | Elem::Char(v) | Elem::RzChar(v) => out.put_u8(v),
        Elem::I8(v) => out.put_i8(v),
        Elem::U16(v) | Elem::UnicodeChar(v) => out.put_u16(v),
        Elem::I16(v) => out.put_i16(v),
        Elem::U32(v) => out.put_u32(v),
        Elem::I32(v) => out.put_i32(v),
        Elem::U64(v) => out.put_u64(v),
        Elem::I64(v) => out.put_i64(v),
        Elem::F32(v) => out.put_f32(v),
        Elem::F64(v) => out.put_f64(v),
        Elem::VoidPtr(_) => false,
    }
}

fn decode(t: u16, b: &[u8]) -> Elem {
    let u = |n: usize| -> u64 {
        let mut x = [0u8; 8];
        x[..n].copy_from_slice(&b[..n]);
        u64::from_le_bytes(x)
    };
    match t {
        mem::BOOL => Elem::Bool(b[0] != 0),
        mem::UINT8 | mem::EMPTY => Elem::U8(b[0]),
        mem::SINT8 => Elem::I8(b[0] as i8),
        mem::CHAR => Elem::Char(b[0]),
        mem::RZ_CHAR => Elem::RzChar(b[0]),
        mem::UINT16 => Elem::U16(u(2) as u16),
        mem::SINT16 => Elem::I16(u(2) as u16 as i16),
        mem::UNICODE_CHAR => Elem::UnicodeChar(u(2) as u16),
        mem::UINT32 => Elem::U32(u(4) as u32),
        mem::SINT32 => Elem::I32(u(4) as u32 as i32),
        mem::FLOAT32 => Elem::F32(f32::from_bits(u(4) as u32)),
        mem::UINT64 => Elem::U64(u(8)),
        mem::SINT64 => Elem::I64(u(8) as i64),
        _ => Elem::F64(f64::from_bits(u(8))),
    }
}

fn read_variant(inp: &mut dyn InStream) -> Option<Read> {
    let marker = inp.get_u32()?;
    // [quirk] any marker other than the object marker reads like 0.
    if marker == OBJECT_MARKER {
        let r = inp.get_ref()?;
        return Some(if r.is_null() {
            Read::NullObject
        } else {
            Read::Value(Variant::Object(r))
        });
    }
    let tag = inp.get_u32()?;
    let t = readable_mem_type(tag)?;
    if t == mem::EMPTY {
        return Some(Read::Value(Variant::Empty));
    }
    if t & ARRAY == 0 {
        let b = inp.get_bytes(elem_size(t))?;
        return Some(Read::Value(Variant::Scalar(decode(t, &b))));
    }
    let elem = t & !ARRAY;
    if elem == mem::RZ_CHAR {
        // String: §2.3 in full (including the UTF-16 header).
        let s = inp.get_str()?;
        return Some(Read::Value(Variant::string(&s)));
    }
    let count = inp.get_u32()? as usize;
    let size = elem_size(elem);
    // [choice] a count whose elements would pass the end of the record fails:
    // the block read is all-or-nothing.
    let bytes = inp.get_bytes(count.checked_mul(size)?)?;
    let items = bytes.chunks(size).map(|c| decode(elem, c)).collect();
    Some(Read::Value(Variant::Array { elem, items }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_count_distinguishes_scalar_and_array_shapes() {
        use super::{mem, Elem, Variant};
        let scalar = Variant::Scalar(Elem::U32(7));
        assert_eq!(scalar.count(), 1);
        assert_eq!(scalar.raw_count(), 0);
        assert_eq!(Variant::Empty.raw_count(), 0);
        for n in [0, 1, 3] {
            let v = Variant::array(mem::UINT32, vec![Elem::U32(7); n]);
            assert_eq!(v.count(), n);
            assert_eq!(v.raw_count(), n);
        }
    }
    use super::*;
    use crate::stream::{ByteReader, ByteWriter};

    fn enc(v: &Variant) -> (bool, Vec<u8>) {
        let mut w = ByteWriter::new();
        let ok = v.write(&mut w);
        (ok, w.bytes().to_vec())
    }

    #[test]
    fn worked_examples() {
        assert_eq!(
            enc(&Variant::Scalar(Elem::U32(7))).1,
            [0, 0, 0, 0, 3, 0, 0, 0, 7, 0, 0, 0]
        );
        assert_eq!(
            enc(&Variant::Scalar(Elem::Bool(true))).1,
            [0, 0, 0, 0, 0x0B, 0, 0, 0, 1]
        );
        assert_eq!(
            enc(&Variant::string(b"Abc")).1,
            [0, 0, 0, 0, 0x0C, 0x80, 0, 0, 3, 0, 0, 0, 0x41, 0x62, 0x63]
        );
        assert_eq!(enc(&Variant::Empty).1, [0; 8]);
        let (ok, b) = enc(&Variant::Scalar(Elem::VoidPtr(1)));
        assert!(!ok);
        assert_eq!(b, [0, 0, 0, 0, 0x12, 0, 0, 0]);
    }

    #[test]
    fn readable_tags_round_trip() {
        let samples = [
            Variant::Scalar(Elem::I8(-2)),
            Variant::Scalar(Elem::F64(1.5)),
            Variant::array(mem::UINT16, vec![]),
            Variant::array(mem::CHAR, vec![Elem::Char(1), Elem::Char(2)]),
            Variant::array(mem::UNICODE_CHAR, vec![Elem::UnicodeChar(0x263A)]),
            Variant::array(0, vec![Elem::U8(9)]),
            Variant::array(mem::BOOL, vec![Elem::Bool(true), Elem::Bool(false)]),
        ];
        for v in &samples {
            let (ok, b) = enc(v);
            assert!(ok, "{v:?}");
            let mut r = ByteReader::new(&b);
            let back = Variant::read(&mut r).expect("readable");
            assert_eq!(&back, v);
            assert_eq!(r.position(), b.len());
        }
    }

    #[test]
    fn quirks() {
        // Char scalars are written but not readable.
        let (ok, b) = enc(&Variant::Scalar(Elem::Char(65)));
        assert!(ok);
        let mut r = ByteReader::new(&b);
        let mut dest = Variant::Scalar(Elem::U32(3));
        assert!(!Variant::read_into(&mut r, &mut dest));
        assert_eq!(dest, Variant::Scalar(Elem::U32(3)));
        assert_ne!(r.error(), 0);
        // A null object leaves the destination unchanged.
        let mut r = ByteReader::new(&[0xAC, 0xE9, 0xBE, 0xA9, 0, 0, 0, 0]);
        assert!(Variant::read_into(&mut r, &mut dest));
        assert_eq!(dest, Variant::Scalar(Elem::U32(3)));
        // Marker 1 reads like 0.
        let mut r = ByteReader::new(&[1, 0, 0, 0, 3, 0, 0, 0, 9, 0, 0, 0]);
        assert_eq!(Variant::read(&mut r), Some(Variant::Scalar(Elem::U32(9))));
    }

    /// data-formats §5.1 worked examples (EX2–EX4).
    #[test]
    fn conversions() {
        // Floats to integers: _ftol truncation, the low bits kept (EX3).
        assert_eq!(Elem::F32(300.7).as_i64() as u8, 44);
        assert_eq!(Elem::F32(3e9).as_i64() as i32, -1_294_967_296);
        assert_eq!(Elem::F32(-1.5).as_i64() as u32, 0xFFFF_FFFF);
        assert_eq!(Elem::F32(2.9).as_i64(), 2);
        for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 1e19, -1e19] {
            assert_eq!(Elem::F64(x).as_i64(), i64::MIN, "{x}");
            assert_eq!(Elem::F64(x).as_i64() as u32, 0, "{x}");
            assert_ne!(Elem::F64(x).as_i64(), 0, "{x}: true as Bool");
        }
        assert_eq!(Elem::F32(0.5).as_i64(), 0, "false as Bool");
        // Char and RZChar are signed bytes, RZUnicodeChar unsigned (EX4).
        for e in [Elem::RzChar(0xE9), Elem::Char(0xE9)] {
            assert_eq!(e.as_i64() as u32, 0xFFFF_FFE9);
            assert_eq!(e.as_i64() as i16, -23);
            assert_eq!(e.as_i64() as u8, 0xE9);
            assert_eq!(e.as_f64() as f32, -23.0);
        }
        assert_eq!(Elem::UnicodeChar(0xFFE9).as_i64() as u32, 65_513);
        // Unsigned values to double: 24 significant bits for a Uint64 and a Uint32 from 2^31 (EX2).
        assert_eq!(Elem::U64(16_777_217).as_f64(), 16_777_216.0);
        assert_eq!(Elem::I64(16_777_217).as_f64(), 16_777_217.0);
        assert_eq!(Elem::U32(16_777_217).as_f64(), 16_777_217.0);
        assert_eq!(Elem::U32(0x8000_0001).as_f64(), 2_147_483_648.0);
        assert_eq!(Elem::U32(0xFFFF_FFFF).as_f64(), 4_294_967_296.0);
        assert_eq!(Elem::U64(u64::MAX).as_f64(), 18_446_744_073_709_551_616.0);
        assert_eq!(Elem::I64((1 << 53) + 1).as_f64(), 9_007_199_254_740_992.0);
        assert_eq!(Elem::I32(-3).as_f64(), -3.0);
    }
}
