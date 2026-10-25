// SPDX-License-Identifier: GPL-3.0-or-later
//! Owned flora occupant state. The C++ owner serializes access with its mutex.
use std::ffi::c_void;

use fc4_core::flora::occupant::FloraInstance;

use crate::guard;
use crate::props::{fc4_props, props_ref};
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

#[repr(C)]
pub struct fc4_flora_instance {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct fc4_flora_instance_values {
    pub flora_type: u32,
    pub birth: u32,
    pub last_seed: u32,
    pub state: i32,
    pub orientation: u32,
    pub position: [f32; 3],
    pub lot_tag: u32,
    pub removal: u16,
    pub flags: u8,
    pub highlight: u8,
    pub status: u8,
    pub cells: [u8; 4],
}

#[no_mangle]
pub extern "C" fn fc4_flora_instance_create() -> *mut fc4_flora_instance {
    guard(std::ptr::null_mut(), || {
        Box::into_raw(Box::new(FloraInstance::default())).cast()
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_flora_instance_destroy(p: *mut fc4_flora_instance) {
    if !p.is_null() {
        guard((), || drop(Box::from_raw(p.cast::<FloraInstance>())));
    }
}

#[no_mangle]
pub unsafe extern "C" fn fc4_flora_instance_values_get(
    p: *const fc4_flora_instance,
    out: *mut fc4_flora_instance_values,
) -> bool {
    guard(false, || {
        let (Some(p), Some(out)) = (p.cast::<FloraInstance>().as_ref(), out.as_mut()) else {
            return false;
        };
        *out = fc4_flora_instance_values {
            flora_type: p.flora.flora_type,
            birth: p.flora.birth_date,
            last_seed: p.flora.last_seeding_date,
            state: i32::from(p.flora.state),
            orientation: u32::from(p.flora.orientation),
            position: p.position,
            lot_tag: p.lot_tag,
            removal: p.base.removal,
            flags: p.base.flags,
            highlight: p.base.highlight,
            status: p.base.status,
            cells: p.base.cell_rect,
        };
        true
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_flora_instance_values_set(
    p: *mut fc4_flora_instance,
    value: *const fc4_flora_instance_values,
) -> bool {
    guard(false, || {
        let (Some(p), Some(v)) = (p.cast::<FloraInstance>().as_mut(), value.as_ref()) else {
            return false;
        };
        p.flora.flora_type = v.flora_type;
        p.flora.birth_date = v.birth;
        p.flora.last_seeding_date = v.last_seed;
        p.flora.state = v.state as i8;
        p.flora.orientation = v.orientation as u8;
        p.position = v.position;
        p.lot_tag = v.lot_tag;
        p.base.removal = v.removal;
        p.base.flags = v.flags;
        p.base.highlight = v.highlight;
        p.base.status = v.status;
        p.base.cell_rect = v.cells;
        true
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_flora_instance_write(
    p: *const fc4_flora_instance,
    out: *const fc4_ostream,
    props: *const fc4_props,
) -> bool {
    guard(false, || {
        let (Some(p), Some(out)) = (p.cast::<FloraInstance>().as_ref(), out.as_ref()) else {
            return false;
        };
        if !ostream_ok(out) {
            return false;
        }
        let mut snapshot = p.clone();
        snapshot.base.holder.own = props_ref(props).cloned().unwrap_or_default();
        snapshot.write(&mut COut(out))
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_flora_instance_read(p: *mut fc4_flora_instance, inp: *const fc4_istream) -> bool {
    guard(false, || {
        let (Some(p), Some(inp)) = (p.cast::<FloraInstance>().as_mut(), inp.as_ref()) else {
            return false;
        };
        if !istream_ok(inp) {
            return false;
        }
        p.read(&mut CIn {
            stream: inp,
            decode: |_: u32, _: *mut c_void| None,
        })
    })
}

/// A fresh copy of the instance overrides; the caller destroys it.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_instance_props(p: *const fc4_flora_instance) -> *mut fc4_props {
    guard(std::ptr::null_mut(), || {
        p.cast::<FloraInstance>().as_ref().map_or(std::ptr::null_mut(), |p| {
            Box::into_raw(Box::new(p.base.holder.own.clone())).cast()
        })
    })
}

/// Compare the complete persisted record and the occupant's public getters.
#[no_mangle]
pub unsafe extern "C" fn fc4_flora_instance_record_shadow(
    bytes: *const u8,
    length: usize,
    expected: *const fc4_flora_instance_values,
) -> bool {
    guard(false, || {
        let Some(expected) = expected.as_ref() else {
            return false;
        };
        if bytes.is_null() || length > 16 * 1024 * 1024 {
            return false;
        }
        let bytes = std::slice::from_raw_parts(bytes, length);
        let mut reader = fc4_core::stream::ByteReader::new(bytes);
        let mut instance = FloraInstance::default();
        if !instance.read(&mut reader) || reader.position() != length {
            return false;
        }
        let mut writer = fc4_core::stream::ByteWriter::new();
        if !instance.write(&mut writer) || writer.bytes() != bytes {
            return false;
        }
        instance.flora.flora_type == expected.flora_type
            && instance.flora.birth_date == expected.birth
            && instance.flora.last_seeding_date == expected.last_seed
            && i32::from(instance.flora.state) == expected.state
            && u32::from(instance.flora.orientation) == expected.orientation
            && instance.position.map(f32::to_bits) == expected.position.map(f32::to_bits)
            && instance.lot_tag == expected.lot_tag
            && instance.base.flags == (expected.flags & 0xdd)
            && instance.base.highlight == expected.highlight
            && instance.base.occupant_type == fc4_core::flora::occupant_type::FLORA
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owns_independent_instances_and_narrows_byte_fields() {
        unsafe {
            let a = fc4_flora_instance_create();
            let b = fc4_flora_instance_create();
            assert!(!a.is_null() && !b.is_null() && a != b);
            let mut v = fc4_flora_instance_values::default();
            assert!(fc4_flora_instance_values_get(a, &mut v));
            assert_eq!(v.flags, 12);
            v.flora_type = 17;
            v.state = 255;
            v.orientation = 257;
            assert!(fc4_flora_instance_values_set(a, &v));
            assert!(fc4_flora_instance_values_get(a, &mut v));
            assert_eq!((v.flora_type, v.state, v.orientation), (17, -1, 1));
            assert!(fc4_flora_instance_values_get(b, &mut v));
            assert_eq!(v.flora_type, 0);
            fc4_flora_instance_destroy(a);
            fc4_flora_instance_destroy(b);
            fc4_flora_instance_destroy(std::ptr::null_mut());
        }
    }

    #[test]
    fn shadow_detects_getter_difference_and_extra_bytes() {
        unsafe {
            let p = fc4_flora_instance_create();
            let mut v = fc4_flora_instance_values::default();
            assert!(fc4_flora_instance_values_get(p, &mut v));
            let mut out = fc4_core::stream::ByteWriter::new();
            assert!((&*p.cast::<FloraInstance>()).write(&mut out));
            assert!(fc4_flora_instance_record_shadow(
                out.bytes().as_ptr(),
                out.bytes().len(),
                &v
            ));
            for length in 0..out.bytes().len() {
                assert!(!fc4_flora_instance_record_shadow(out.bytes().as_ptr(), length, &v));
            }
            let mut extended = out.bytes().to_vec();
            extended.push(0);
            assert!(!fc4_flora_instance_record_shadow(extended.as_ptr(), extended.len(), &v));
            v.birth += 1;
            assert!(!fc4_flora_instance_record_shadow(
                out.bytes().as_ptr(),
                out.bytes().len(),
                &v
            ));
            fc4_flora_instance_destroy(p);
        }
    }

    #[test]
    fn shadow_compares_position_bits_without_normalizing_floats() {
        unsafe {
            let p = fc4_flora_instance_create();
            let mut v = fc4_flora_instance_values::default();
            assert!(fc4_flora_instance_values_get(p, &mut v));
            v.position = [-0.0, f32::from_bits(0x7fc0_1234), 16.0];
            assert!(fc4_flora_instance_values_set(p, &v));
            let mut out = fc4_core::stream::ByteWriter::new();
            assert!((&*p.cast::<FloraInstance>()).write(&mut out));
            let exact = |expected: &fc4_flora_instance_values| {
                fc4_flora_instance_record_shadow(out.bytes().as_ptr(), out.bytes().len(), expected)
            };
            assert!(exact(&v));
            v.position[0] = 0.0;
            assert!(!exact(&v));
            v.position[0] = -0.0;
            v.position[1] = f32::from_bits(0x7fc0_5678);
            assert!(!exact(&v));
            fc4_flora_instance_destroy(p);
        }
    }
}
