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

use crate::{guard, slice_arg};

#[repr(C)]
pub struct fc4_props {
    _private: [u8; 0],
}

pub(crate) unsafe fn props_ref<'a>(p: *const fc4_props) -> Option<&'a PropertyHolder> {
    (p as *const PropertyHolder).as_ref()
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

/// One value is stored as a scalar, several (or none) as an array.
fn variant(elem: u16, items: Vec<Elem>) -> Variant {
    if items.len() == 1 {
        Variant::Scalar(items[0])
    } else {
        Variant::Array { elem, items }
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
        /// Sets property `id` (replacing it). `values` may be NULL when
        /// `count` is 0 (an empty array).
        #[no_mangle]
        pub unsafe extern "C" fn $name(p: *mut fc4_props, id: u32, values: *const $t, count: usize) -> bool {
            let items: Vec<Elem> = match slice_arg(values, count) {
                Some(s) => s.iter().map(|&v| $elem(v)).collect(),
                None if count == 0 => Vec::new(),
                None => return false,
            };
            with_props(p, |h| h.add(id, variant($mem, items)))
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
/// (bool: one byte). A scalar (`is_array` false) needs `count` 1. False for
/// other types (empty, pointers, objects) or a bad count.
#[no_mangle]
pub unsafe extern "C" fn fc4_props_set_typed(
    p: *mut fc4_props,
    id: u32,
    mem_type: u16,
    is_array: bool,
    data: *const std::ffi::c_void,
    count: usize,
) -> bool {
    let Some(size) = elem_size(mem_type) else { return false };
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
    let items: Vec<Elem> = bytes.chunks_exact(size).map(|c| decode_elem(mem_type, c)).collect();
    let v = if is_array {
        Variant::Array { elem: mem_type, items }
    } else {
        Variant::Scalar(items[0])
    };
    with_props(p, |h| h.add(id, v))
}
