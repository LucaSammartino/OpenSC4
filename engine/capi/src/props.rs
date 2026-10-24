// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Property holders built by the host (`fc4_props`, fc4.h "property
//! holders"): the shim copies the properties a subsystem reads from a game
//! exemplar into one of these and hands it to the core.

use fc4_core::property::PropertyHolder;
use fc4_core::variant::{mem, Elem, Variant};

use crate::stream::fc4_bytes_sink;
use crate::{guard, slice_arg};

#[repr(C)]
pub struct fc4_props {
    _private: [u8; 0],
}

pub(crate) unsafe fn props_ref<'a>(p: *const fc4_props) -> Option<&'a PropertyHolder> {
    (p as *const PropertyHolder).as_ref()
}

/// Tooling snapshots: a new independent holder, or NULL for NULL input.
#[no_mangle]
pub unsafe extern "C" fn fc4_props_clone(p: *const fc4_props) -> *mut fc4_props {
    guard(std::ptr::null_mut(), || {
        props_ref(p).map_or(std::ptr::null_mut(), |h| {
            Box::into_raw(Box::new(h.clone())) as *mut fc4_props
        })
    })
}

fn with_props(p: *mut fc4_props, f: impl FnOnce(&mut PropertyHolder)) -> bool {
    // SAFETY: documented precondition: NULL or a live fc4_props*.
    match unsafe { (p as *mut PropertyHolder).as_mut() } {
        Some(h) => guard(false, || {
            f(h);
            true
        }),
        None => false,
    }
}

/// A new, empty property holder.
#[no_mangle]
pub extern "C" fn fc4_props_create() -> *mut fc4_props {
    guard(std::ptr::null_mut(), || {
        Box::into_raw(Box::new(PropertyHolder::new())) as *mut fc4_props
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_props_destroy(p: *mut fc4_props) {
    if !p.is_null() {
        guard((), || drop(Box::from_raw(p as *mut PropertyHolder)));
    }
}

macro_rules! setter {
    ($name:ident, $t:ty, $mem:expr, $elem:expr) => {
        /// Sets property `id` (replacing it) to an **array** of `count`
        /// values, one value included (ABI 50: it was a scalar before, which
        /// the vector readers now read as empty, data-formats §5.2 EX39; a
        /// scalar is set with `fc4_props_set_typed`). `values` may be NULL
        /// when `count` is 0 (an empty array).
        #[no_mangle]
        pub unsafe extern "C" fn $name(p: *mut fc4_props, id: u32, values: *const $t, count: usize) -> bool {
            let items: Vec<Elem> = match slice_arg(values, count) {
                Some(s) => s.iter().map(|&v| $elem(v)).collect(),
                None if count == 0 => Vec::new(),
                None => return false,
            };
            with_props(p, |h| h.add(id, Variant::Array { elem: $mem, items }))
        }
    };
}

setter!(fc4_props_set_u32s, u32, mem::UINT32, Elem::U32);
setter!(fc4_props_set_i32s, i32, mem::SINT32, Elem::I32);
setter!(fc4_props_set_i64s, i64, mem::SINT64, Elem::I64);
setter!(fc4_props_set_f32s, f32, mem::FLOAT32, Elem::F32);
setter!(fc4_props_set_bools, bool, mem::BOOL, Elem::Bool);

/// Sets a string property (bytes, no terminator).
#[no_mangle]
pub unsafe extern "C" fn fc4_props_set_string(p: *mut fc4_props, id: u32, bytes: *const u8, len: usize) -> bool {
    let items: Vec<Elem> = match slice_arg(bytes, len) {
        Some(s) => s.iter().map(|&b| Elem::RzChar(b)).collect(),
        None if len == 0 => Vec::new(),
        None => return false,
    };
    with_props(p, |h| {
        h.add(
            id,
            Variant::Array {
                elem: mem::RZ_CHAR,
                items,
            },
        )
    })
}

/// Number of properties (0 for NULL).
#[no_mangle]
pub unsafe extern "C" fn fc4_props_count(p: *const fc4_props) -> usize {
    props_ref(p).map_or(0, PropertyHolder::len)
}

/// Production cIGZVariant GetCount bridge. `storage_count` is the owned
/// adapter's array length, not a serialized scalar's logical element count.
#[no_mangle]
pub extern "C" fn fc4_variant_get_count(var_type: u16, storage_count: u32) -> u32 {
    fc4_core::variant::native_count(var_type, storage_count as usize) as u32
}

/// Element size of a game variant type, or `None` for types the core does not
/// keep (empty, pointers, objects).
fn elem_size(mem_type: u16) -> Option<usize> {
    Some(match mem_type {
        mem::BOOL | mem::UINT8 | mem::SINT8 | mem::CHAR | mem::RZ_CHAR => 1,
        mem::UINT16 | mem::SINT16 | mem::UNICODE_CHAR => 2,
        mem::UINT32 | mem::SINT32 | mem::FLOAT32 => 4,
        mem::UINT64 | mem::SINT64 | mem::FLOAT64 => 8,
        _ => return None,
    })
}

fn decode_elem(mem_type: u16, b: &[u8]) -> Elem {
    let a2 = || [b[0], b[1]];
    let a4 = || [b[0], b[1], b[2], b[3]];
    let a8 = || [b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]];
    match mem_type {
        mem::BOOL => Elem::Bool(b[0] != 0),
        mem::UINT8 => Elem::U8(b[0]),
        mem::SINT8 => Elem::I8(b[0] as i8),
        mem::CHAR => Elem::Char(b[0]),
        mem::RZ_CHAR => Elem::RzChar(b[0]),
        mem::UINT16 => Elem::U16(u16::from_ne_bytes(a2())),
        mem::SINT16 => Elem::I16(i16::from_ne_bytes(a2())),
        mem::UNICODE_CHAR => Elem::UnicodeChar(u16::from_ne_bytes(a2())),
        mem::UINT32 => Elem::U32(u32::from_ne_bytes(a4())),
        mem::SINT32 => Elem::I32(i32::from_ne_bytes(a4())),
        mem::FLOAT32 => Elem::F32(f32::from_ne_bytes(a4())),
        mem::UINT64 => Elem::U64(u64::from_ne_bytes(a8())),
        mem::SINT64 => Elem::I64(i64::from_ne_bytes(a8())),
        _ => Elem::F64(f64::from_ne_bytes(a8())),
    }
}

/// Sets property `id` exactly as a game variant holds it: `mem_type` is the
/// variant's element type (the cIGZVariant type without the array bit, 0x01
/// to 0x0E), `data` holds `count` elements in that type's native layout
/// (bool: one byte). A scalar (`is_array` false) needs `count` 1. ABI 59
/// (neighbors.md §15.4 NG37): `mem_type` 0 with `is_array` is the void array
/// (raw bytes, type 0x8000), `count` bytes. False for other types (empty,
/// pointers, objects) or a bad count.
#[no_mangle]
pub unsafe extern "C" fn fc4_props_set_typed(
    p: *mut fc4_props,
    id: u32,
    mem_type: u16,
    is_array: bool,
    data: *const std::ffi::c_void,
    count: usize,
) -> bool {
    let void_array = is_array && mem_type == mem::EMPTY;
    let Some(size) = (if void_array { Some(1) } else { elem_size(mem_type) }) else {
        return false;
    };
    if !is_array && count != 1 {
        return false;
    }
    let Some(len) = size.checked_mul(count) else {
        return false;
    };
    let bytes = match slice_arg(data as *const u8, len) {
        Some(s) => s,
        None if count == 0 => &[],
        None => return false,
    };
    let items: Vec<Elem> = if void_array {
        // The void array (NG37): the core holds raw bytes as an array of
        // memory type 0 with byte elements (foundations §9.2, tag 0x8000).
        bytes.iter().map(|&b| Elem::U8(b)).collect()
    } else {
        bytes.chunks_exact(size).map(|c| decode_elem(mem_type, c)).collect()
    };
    let v = if is_array {
        Variant::Array { elem: mem_type, items }
    } else {
        Variant::Scalar(items[0])
    };
    with_props(p, |h| h.add(id, v))
}

/// `WriteVariant` of property `id` (foundations §9.2: u32 0, u32 type tag,
/// then the value) as the game's segment streams write it, handed to `sink`
/// in one call. False when the property is absent or its type cannot be
/// written (nothing is handed over then). Tooling: the traffic shadow's
/// capture stream writes the variants of a record this way.
#[no_mangle]
pub unsafe extern "C" fn fc4_props_write_variant(
    p: *const fc4_props,
    id: u32,
    sink: fc4_bytes_sink,
    ctx: *mut std::ffi::c_void,
) -> bool {
    let Some(h) = props_ref(p) else { return false };
    let Some(f) = sink else { return false };
    let bytes = guard(None, || {
        let v = h.get(id)?;
        let mut w = fc4_core::stream::ByteWriter::new();
        v.write(&mut w).then(|| w.into_bytes())
    });
    match bytes {
        Some(b) => {
            f(ctx, b.as_ptr(), b.len());
            true
        }
        None => false,
    }
}

/// `GetProperty(id, uint32&)` (holder slot 8) on one value held as a game
/// variant holds it (neighbors.md §15.4 NG44, data-formats §5.1, the
/// converting getters). `var_type` is the cIGZVariant type, the array bit
/// `0x8000` included; `data` holds `count` elements of an array, or the one
/// value of a scalar (`count` 1), in the element type's native layout (bool:
/// one byte). A Uint32 scalar is returned as is, other scalars convert by
/// number (signed values sign-extended, Char and RZChar as signed bytes,
/// RZUnicodeChar zero-extended, 64-bit values keep their low 32 bits, floats
/// truncated by `_ftol`: NaN, infinities and magnitudes of 2^63 or more give
/// 0); arrays, strings, the void array and the Empty value (type 0) take the
/// text round trip (the first element's text, a string's characters; an
/// empty character array or the void array gives the empty text, which
/// parses as 0, so true with 0). True with `*out` set; false with `*out`
/// unchanged for a pointer or object value (*[choice]*, data-formats §5.1)
/// or a bad layout.
#[no_mangle]
pub unsafe extern "C" fn fc4_variant_get_u32(
    var_type: u16,
    data: *const std::ffi::c_void,
    count: usize,
    out: *mut u32,
) -> bool {
    guard(false, || {
        let Some(v) = variant_u32(var_type, data as *const u8, count) else {
            return false;
        };
        if let Some(o) = out.as_mut() {
            *o = v;
        }
        true
    })
}

/// [`fc4_variant_get_u32`]'s value, through `fc4_formats`' converting getter.
unsafe fn variant_u32(var_type: u16, data: *const u8, count: usize) -> Option<u32> {
    match variant_value(var_type, data, count)? {
        GameValue::Prop(p) => p.get_u32(),
        // The scalar Char and RZUnicodeChar have no fc4_formats form (only
        // their arrays are readable there): by number, Char as a signed byte.
        GameValue::CharScalar(c) => Some(i32::from(c as i8) as u32),
        GameValue::UnicodeScalar(u) => Some(u32::from(u)),
    }
}

/// One game variant's value as `fc4_formats` reads it, or one of the two
/// scalar types it has no form for.
enum GameValue {
    Prop(fc4_formats::exemplar::Property),
    CharScalar(u8),
    UnicodeScalar(u16),
}

/// `GetValString` (variant slot 55, data-formats §5.1) on one value held as a
/// game variant, laid out as for [`fc4_variant_get_u32`]. The value's text
/// goes to `sink` (one call, possibly empty) and the result is 1 when the
/// getter returns true (a string, RZChar array `0x800E`) or 0 when it writes
/// the round-trip text and returns false (every other type, Empty included:
/// a Uint32 7 gives `7` and 0). -1 when nothing is written: a pointer or
/// object value (*[choice]* of §5.1, a missing property), a bad layout, no
/// sink, and the scalar Char and RZUnicodeChar, whose text §5.1 does not give
/// (question EX53; literal reading: unsupported, output unchanged).
#[no_mangle]
pub unsafe extern "C" fn fc4_variant_get_string(
    var_type: u16,
    data: *const std::ffi::c_void,
    count: usize,
    sink: fc4_bytes_sink,
    ctx: *mut std::ffi::c_void,
) -> i32 {
    let Some(f) = sink else { return -1 };
    let r = guard(None, || match variant_value(var_type, data as *const u8, count)? {
        GameValue::Prop(p) => {
            let mut out = Vec::new();
            let ok = p.get_string_into(&mut out);
            p.text().map(|_| (out, ok))
        }
        GameValue::CharScalar(_) | GameValue::UnicodeScalar(_) => None,
    });
    match r {
        Some((b, ok)) => {
            f(ctx, b.as_ptr(), b.len());
            i32::from(ok)
        }
        None => -1,
    }
}

/// The value of [`fc4_variant_get_u32`]'s arguments.
unsafe fn variant_value(var_type: u16, data: *const u8, count: usize) -> Option<GameValue> {
    use fc4_formats::exemplar::{Property, Values};
    let is_array = var_type & fc4_core::variant::ARRAY != 0;
    let base = var_type & !fc4_core::variant::ARRAY;
    if base == mem::EMPTY {
        // The Empty value (type 0) and the void array (0x8000): their text is
        // empty, which parses as 0 (data-formats §5.1). The void array's bytes
        // are never read.
        return Some(GameValue::Prop(Property::single(0, Values::Empty)));
    }
    let size = elem_size(base)?;
    if !is_array && count != 1 {
        return None;
    }
    let bytes = match slice_arg(data, size.checked_mul(count)?) {
        Some(s) => s,
        None if count == 0 => &[],
        None => return None,
    };
    if !is_array && base == mem::CHAR {
        return Some(GameValue::CharScalar(bytes[0]));
    }
    if !is_array && base == mem::UNICODE_CHAR {
        return Some(GameValue::UnicodeScalar(u16::from_ne_bytes([bytes[0], bytes[1]])));
    }
    fn each<T>(b: &[u8], n: usize, f: impl Fn(&[u8]) -> T) -> Vec<T> {
        b.chunks_exact(n).map(f).collect()
    }
    fn a4(c: &[u8]) -> [u8; 4] {
        [c[0], c[1], c[2], c[3]]
    }
    fn a8(c: &[u8]) -> [u8; 8] {
        [c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]
    }
    let values = match base {
        mem::BOOL => Values::Bool(each(bytes, 1, |c| c[0] != 0)),
        mem::UINT8 => Values::Uint8(bytes.to_vec()),
        mem::SINT8 => Values::Sint8(each(bytes, 1, |c| c[0] as i8)),
        mem::UINT16 => Values::Uint16(each(bytes, 2, |c| u16::from_ne_bytes([c[0], c[1]]))),
        mem::SINT16 => Values::Sint16(each(bytes, 2, |c| i16::from_ne_bytes([c[0], c[1]]))),
        mem::UINT32 => Values::Uint32(each(bytes, 4, |c| u32::from_ne_bytes(a4(c)))),
        mem::SINT32 => Values::Sint32(each(bytes, 4, |c| i32::from_ne_bytes(a4(c)))),
        mem::FLOAT32 => Values::Float32(each(bytes, 4, |c| f32::from_ne_bytes(a4(c)))),
        mem::UINT64 => Values::Uint64(each(bytes, 8, |c| u64::from_ne_bytes(a8(c)))),
        mem::SINT64 => Values::Sint64(each(bytes, 8, |c| i64::from_ne_bytes(a8(c)))),
        mem::FLOAT64 => Values::Float64(each(bytes, 8, |c| f64::from_ne_bytes(a8(c)))),
        mem::CHAR => Values::Char(bytes.to_vec()),
        mem::UNICODE_CHAR => Values::UnicodeChar(each(bytes, 2, |c| u16::from_ne_bytes([c[0], c[1]]))),
        mem::RZ_CHAR => Values::String(bytes.to_vec()),
        _ => return None,
    };
    let p = if is_array {
        Property::array(0, values)
    } else {
        Property::single(0, values)
    };
    Some(GameValue::Prop(p))
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_count_bridge_preserves_array_shape() {
        use super::fc4_variant_get_count;
        // Discriminate stale scalar storage metadata from a one-element array.
        assert_eq!(fc4_variant_get_count(6, 1), 0);
        assert_eq!(fc4_variant_get_count(6, 37), 0);
        for n in [0, 1, 3] {
            assert_eq!(fc4_variant_get_count(0x8006, n), n);
        }
        assert_eq!(fc4_variant_get_count(0, 0), 0);
    }
    use super::*;

    unsafe extern "C" fn collect(ctx: *mut std::ffi::c_void, data: *const u8, len: usize) {
        let v = &mut *(ctx as *mut Vec<u8>);
        v.clear();
        if len != 0 {
            v.extend_from_slice(std::slice::from_raw_parts(data, len));
        }
    }

    fn text_of(var_type: u16, bytes: &[u8], count: usize) -> (i32, Vec<u8>) {
        let mut out = b"old".to_vec();
        let r = unsafe {
            fc4_variant_get_string(
                var_type,
                bytes.as_ptr().cast(),
                count,
                Some(collect),
                (&mut out as *mut Vec<u8>).cast(),
            )
        };
        (r, out)
    }

    /// data-formats §5.1 `GetValString`: true only for a string; every other
    /// type writes its round-trip text and returns false; objects write nothing.
    #[test]
    fn get_val_string_text_and_result() {
        const A: u16 = 0x8000;
        assert_eq!(text_of(A | mem::RZ_CHAR, b"Fc4Test", 7), (1, b"Fc4Test".to_vec()));
        assert_eq!(text_of(A | mem::RZ_CHAR, b"abc", 3), (1, b"abc".to_vec()));
        assert_eq!(text_of(mem::UINT32, &7u32.to_ne_bytes(), 1), (0, b"7".to_vec()));
        assert_eq!(text_of(mem::SINT32, &(-5i32).to_ne_bytes(), 1), (0, b"-5".to_vec()));
        assert_eq!(
            text_of(A | mem::UINT32, &[5, 0, 0, 0, 9, 0, 0, 0], 2),
            (0, b"5".to_vec())
        );
        assert_eq!(text_of(A | mem::CHAR, b"12", 2), (0, b"12".to_vec()));
        assert_eq!(text_of(mem::FLOAT32, &2.75f32.to_ne_bytes(), 1), (0, b"2.75".to_vec()));
        assert_eq!(text_of(mem::EMPTY, &[], 0), (0, Vec::new()));
        // Char / RZUnicodeChar scalars (EX53, unsupported) and objects: nothing.
        assert_eq!(text_of(mem::CHAR, b"x", 1), (-1, b"old".to_vec()));
        assert_eq!(text_of(0x10, &[0; 4], 1), (-1, b"old".to_vec()));
    }

    fn u32_of(var_type: u16, bytes: &[u8], count: usize) -> Option<u32> {
        let mut out = 0xDEAD_BEEF;
        let ok = unsafe { fc4_variant_get_u32(var_type, bytes.as_ptr().cast(), count, &mut out) };
        ok.then_some(out)
    }

    /// NG44 (neighbors.md §15.4, data-formats §5.1): holder slot 8's
    /// conversions. Scalars by number; arrays through their first element's
    /// text; an empty character array gives true with 0.
    #[test]
    fn ng44_get_property_u32_conversions() {
        const A: u16 = 0x8000;
        // Scalars: Sint32 -1 and Sint8 -1 sign-extend; 64-bit keep the low bits.
        assert_eq!(u32_of(mem::SINT32, &(-1i32).to_ne_bytes(), 1), Some(0xFFFF_FFFF));
        assert_eq!(u32_of(mem::SINT8, &[0xFF], 1), Some(0xFFFF_FFFF));
        assert_eq!(
            u32_of(mem::UINT64, &0x1_2345_6789u64.to_ne_bytes(), 1),
            Some(0x2345_6789)
        );
        // Floats truncate toward zero; NaN, infinities and >= 2^63 give 0.
        assert_eq!(u32_of(mem::FLOAT32, &2.9f32.to_ne_bytes(), 1), Some(2));
        assert_eq!(u32_of(mem::FLOAT32, &(-1.5f32).to_ne_bytes(), 1), Some(0xFFFF_FFFF));
        assert_eq!(u32_of(mem::FLOAT32, &f32::NAN.to_ne_bytes(), 1), Some(0));
        assert_eq!(u32_of(mem::FLOAT64, &f64::INFINITY.to_ne_bytes(), 1), Some(0));
        assert_eq!(u32_of(mem::FLOAT32, &1e30f32.to_ne_bytes(), 1), Some(0));
        // Char and RZChar scalars are signed bytes, RZUnicodeChar unsigned.
        assert_eq!(u32_of(mem::RZ_CHAR, &[0xE9], 1), Some(0xFFFF_FFE9));
        assert_eq!(u32_of(mem::CHAR, &[0xE9], 1), Some(0xFFFF_FFE9));
        assert_eq!(u32_of(mem::UNICODE_CHAR, &0xFFE9u16.to_ne_bytes(), 1), Some(65_513));
        // Arrays: the first element's text. Signed elements give the scalar's bits.
        let trips: Vec<u8> = [30u16, 0].iter().flat_map(|x| x.to_ne_bytes()).collect();
        assert_eq!(u32_of(A | mem::UINT16, &trips, 2), Some(30));
        assert_eq!(u32_of(A | mem::SINT32, &(-5i32).to_ne_bytes(), 1), Some(0xFFFF_FFFB));
        assert_eq!(u32_of(A | mem::FLOAT32, &2.75f32.to_ne_bytes(), 1), Some(2));
        assert_eq!(u32_of(A | mem::FLOAT32, &30.0f32.to_ne_bytes(), 1), Some(30));
        // Float32 {1e30} prints all its digits, which wrap in 64 bits.
        assert_eq!(u32_of(A | mem::FLOAT32, &1e30f32.to_ne_bytes(), 1), Some(0x1905_8000));
        // A NaN element prints as 1.#QNAN: 1.
        let qnan = f32::from_bits(0x7FC0_0000);
        assert_eq!(u32_of(A | mem::FLOAT32, &qnan.to_ne_bytes(), 1), Some(1));
        // Empty arrays: "0" for numbers, the empty text for characters: true, 0.
        assert_eq!(u32_of(A | mem::UINT32, &[], 0), Some(0));
        assert_eq!(u32_of(A | mem::RZ_CHAR, &[], 0), Some(0));
        assert_eq!(u32_of(A | mem::CHAR, &[], 0), Some(0));
        // Strings parse as numbers: "12" -> 12, "0x1F" -> 31.
        assert_eq!(u32_of(A | mem::RZ_CHAR, b"12", 2), Some(12));
        assert_eq!(u32_of(A | mem::RZ_CHAR, b"0x1F", 4), Some(31));
        // The void array and the Empty value: the empty text, 0.
        assert_eq!(u32_of(0x8000, &[0x31, 0x32], 2), Some(0));
        assert_eq!(u32_of(0, &[], 0), Some(0));
        // Objects and pointers: false (the original gives an address).
        assert_eq!(u32_of(mem::IGZ_UNKNOWN, &[0; 4], 1), None);
        assert_eq!(u32_of(mem::VOID_PTR, &[0; 4], 1), None);
        // A scalar needs one element.
        assert_eq!(u32_of(mem::UINT32, &[0; 8], 2), None);
    }

    /// ABI 50 (data-formats §5.2, EX39): a slice setter stores an array even
    /// for one value, which the core's vector readers read as that value; a
    /// scalar set through `fc4_props_set_typed` reads as an empty vector and
    /// keeps its value for the converting getters.
    #[test]
    fn ex39_one_value_slices_are_arrays_and_scalars_read_empty() {
        let p = fc4_props_create();
        let one = 7u32;
        let cost = 10i64;
        unsafe {
            assert!(fc4_props_set_u32s(p, 1, &one, 1));
            assert!(fc4_props_set_typed(
                p,
                2,
                mem::SINT64,
                false,
                (&cost as *const i64).cast(),
                1
            ));
            let h = props_ref(p).unwrap();
            assert!(matches!(h.get(1), Some(Variant::Array { .. })));
            assert_eq!(fc4_core::props::u32s(h, 1), Some(vec![7]));
            assert_eq!(fc4_core::props::i64s(h, 2), Some(vec![]));
            assert_eq!(fc4_core::props::i64_of(h, 2), Some(10));
            fc4_props_destroy(p);
        }
    }

    #[test]
    fn frozen_holder_clone_survives_original_mutation_and_destruction() {
        unsafe {
            assert!(fc4_props_clone(std::ptr::null()).is_null());
            let original = fc4_props_create();
            assert!(fc4_props_set_u32s(original, 7, [11, 23, 11].as_ptr(), 3));
            let frozen = fc4_props_clone(original);
            assert!(fc4_props_set_u32s(original, 7, [99].as_ptr(), 1));
            fc4_props_destroy(original);
            assert_eq!(
                fc4_core::props::u32s(props_ref(frozen).unwrap(), 7),
                Some(vec![11, 23, 11])
            );
            fc4_props_destroy(frozen);
        }
    }
}
