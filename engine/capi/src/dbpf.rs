// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

//! DBPF packages and exemplars read by `fc4_formats` (data-formats §2, §4),
//! for the plug-in's formats shadow: it opens a shipped package itself and
//! compares each exemplar's own properties, as the core parses them, with the
//! ones the game's resource manager loaded for the same key.

use fc4_formats::dbpf::{FileSource, Package};
use fc4_formats::exemplar::{Exemplar, Values};
use fc4_formats::tgi::Tgi;

use crate::props::{fc4_props, fc4_props_create, fc4_props_destroy, fc4_props_set_typed};
use crate::{guard, slice_arg};

/// An open DBPF package (`fc4_formats::dbpf::Package` over a file).
#[repr(C)]
pub struct fc4_dbpf {
    _private: [u8; 0],
}

unsafe fn pkg_ref<'a>(p: *const fc4_dbpf) -> Option<&'a Package<FileSource>> {
    (p as *const Package<FileSource>).as_ref()
}

/// Opens the package at `path` (UTF-8 bytes, no terminator). NULL when the
/// file cannot be read or its header or index fail the checks of
/// data-formats §2.1-§2.3.
#[no_mangle]
pub unsafe extern "C" fn fc4_dbpf_open(path: *const u8, len: usize) -> *mut fc4_dbpf {
    let Some(bytes) = slice_arg(path, len) else {
        return std::ptr::null_mut();
    };
    let Ok(path) = std::str::from_utf8(bytes) else {
        return std::ptr::null_mut();
    };
    guard(std::ptr::null_mut(), || match Package::<FileSource>::open(path) {
        Ok(p) => Box::into_raw(Box::new(p)) as *mut fc4_dbpf,
        Err(_) => std::ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_dbpf_close(p: *mut fc4_dbpf) {
    if !p.is_null() {
        guard((), || drop(Box::from_raw(p as *mut Package<FileSource>)));
    }
}

/// How many index entries of type `type_id` the package holds (duplicates
/// included, index order).
#[no_mangle]
pub unsafe extern "C" fn fc4_dbpf_count_type(p: *const fc4_dbpf, type_id: u32) -> usize {
    let Some(pkg) = pkg_ref(p) else { return 0 };
    guard(0, || pkg.entries_of_type(type_id).count())
}

/// The group and instance of the `index`-th entry of type `type_id` (index
/// order). False when out of range.
#[no_mangle]
pub unsafe extern "C" fn fc4_dbpf_key_of_type(
    p: *const fc4_dbpf,
    type_id: u32,
    index: usize,
    group: *mut u32,
    instance: *mut u32,
) -> bool {
    let Some(pkg) = pkg_ref(p) else { return false };
    let Some(e) = guard(None, || pkg.entries_of_type(type_id).nth(index).map(|e| e.tgi)) else {
        return false;
    };
    if let Some(g) = group.as_mut() {
        *g = e.group;
    }
    if let Some(i) = instance.as_mut() {
        *i = e.instance;
    }
    true
}

/// Native-endian bytes of a slice of plain numbers.
fn ne_bytes<T: Copy, const N: usize>(v: &[T], f: impl Fn(T) -> [u8; N]) -> Vec<u8> {
    v.iter().flat_map(|&x| f(x)).collect()
}

/// The element memory type and element bytes of `values`, as
/// `fc4_props_set_typed` takes them; `None` for the values the game holds as
/// no data or an object (Empty, a kind-1 resource): those are not compared
/// (the host's CopyToCore skips them as well).
fn typed(values: &Values) -> Option<(u16, Vec<u8>, usize)> {
    use fc4_core::variant::mem;
    Some(match values {
        Values::Empty | Values::Resource(_) => return None,
        Values::Bool(v) => (mem::BOOL, v.iter().map(|&b| u8::from(b)).collect(), v.len()),
        Values::Uint8(v) => (mem::UINT8, v.clone(), v.len()),
        Values::Sint8(v) => (mem::SINT8, v.iter().map(|&x| x as u8).collect(), v.len()),
        Values::Uint16(v) => (mem::UINT16, ne_bytes(v, u16::to_ne_bytes), v.len()),
        Values::Sint16(v) => (mem::SINT16, ne_bytes(v, i16::to_ne_bytes), v.len()),
        Values::Uint32(v) => (mem::UINT32, ne_bytes(v, u32::to_ne_bytes), v.len()),
        Values::Sint32(v) => (mem::SINT32, ne_bytes(v, i32::to_ne_bytes), v.len()),
        Values::Uint64(v) => (mem::UINT64, ne_bytes(v, u64::to_ne_bytes), v.len()),
        Values::Sint64(v) => (mem::SINT64, ne_bytes(v, i64::to_ne_bytes), v.len()),
        Values::Float32(v) => (mem::FLOAT32, ne_bytes(v, f32::to_ne_bytes), v.len()),
        Values::Float64(v) => (mem::FLOAT64, ne_bytes(v, f64::to_ne_bytes), v.len()),
        Values::String(v) => (mem::RZ_CHAR, v.clone(), v.len()),
        Values::Char(v) => (mem::CHAR, v.clone(), v.len()),
        Values::UnicodeChar(v) => (mem::UNICODE_CHAR, ne_bytes(v, u16::to_ne_bytes), v.len()),
        // The void array: memory type 0 with the array flag (fc4_props_set_typed, NG37).
        Values::Void(v) => (mem::EMPTY, v.clone(), v.len()),
    })
}

/// Reads entry (`type_id`, `group`, `instance`) (the first one in index
/// order, data-formats Q3) and parses it as an exemplar or cohort
/// (data-formats §4, `Exemplar::parse_record`): a new fc4_props with its OWN
/// properties, each with the variant type and shape the game holds
/// (`fc4_props_set_typed`), Empty and kind-1 resource properties left out.
/// `parent` (may be NULL) receives the parent cohort key (type, group,
/// instance; all zero = none). NULL when the entry is absent, fails to
/// decompress, or is not a valid exemplar of that type. The caller destroys
/// the result.
#[no_mangle]
pub unsafe extern "C" fn fc4_dbpf_read_exemplar(
    p: *const fc4_dbpf,
    type_id: u32,
    group: u32,
    instance: u32,
    parent: *mut u32,
) -> *mut fc4_props {
    let Some(pkg) = pkg_ref(p) else {
        return std::ptr::null_mut();
    };
    let tgi = Tgi { type_id, group, instance };
    let Some(ex) = guard(None, || {
        let data = pkg.read_tgi(&tgi).ok()?;
        Exemplar::parse_record(type_id, &data).ok()
    }) else {
        return std::ptr::null_mut();
    };
    if let Some(out) = slice_arg(parent as *const u32, 3).map(|_| std::slice::from_raw_parts_mut(parent, 3)) {
        out.copy_from_slice(&[ex.parent.type_id, ex.parent.group, ex.parent.instance]);
    }
    let props = fc4_props_create();
    if props.is_null() {
        return props;
    }
    for prop in ex.properties() {
        let Some((mem_type, bytes, count)) = typed(&prop.values) else { continue };
        if !fc4_props_set_typed(props, prop.id, mem_type, prop.stored_as_array(), bytes.as_ptr().cast(), count) {
            fc4_props_destroy(props);
            return std::ptr::null_mut();
        }
    }
    props
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::props::props_ref;
    use fc4_core::variant::{Elem, Variant};
    use fc4_formats::dbpf::{Compression, PackageBuilder};
    use fc4_formats::exemplar::{Kind, Property};

    #[test]
    fn opens_a_package_and_reads_own_properties() {
        let mut ex = Exemplar::new(Kind::Exemplar);
        ex.parent = Tgi { type_id: 0x0534_2861, group: 1, instance: 2 };
        ex.set(Property::single(0x10, Values::Uint32(vec![7])));
        ex.set(Property::array(0x20, Values::Float32(vec![1.5, 2.5])));
        ex.set(Property::single(0x30, Values::Empty));
        let key = Tgi { type_id: 0x6534_284A, group: 3, instance: 4 };
        let mut b = PackageBuilder::new();
        b.add(key, &ex.to_binary(), Compression::Always).unwrap();
        let dir = std::env::temp_dir().join(format!("fc4_dbpf_capi_{}.dat", std::process::id()));
        std::fs::write(&dir, b.build().unwrap()).unwrap();
        let path = dir.to_str().unwrap().as_bytes();
        unsafe {
            let p = fc4_dbpf_open(path.as_ptr(), path.len());
            assert!(!p.is_null());
            assert_eq!(fc4_dbpf_count_type(p, key.type_id), 1);
            let (mut g, mut i) = (0, 0);
            assert!(fc4_dbpf_key_of_type(p, key.type_id, 0, &mut g, &mut i));
            assert_eq!((g, i), (3, 4));
            assert!(!fc4_dbpf_key_of_type(p, key.type_id, 1, &mut g, &mut i));
            let mut parent = [0u32; 3];
            let props = fc4_dbpf_read_exemplar(p, key.type_id, 3, 4, parent.as_mut_ptr());
            assert!(!props.is_null());
            assert_eq!(parent, [0x0534_2861, 1, 2]);
            let h = props_ref(props).unwrap();
            assert_eq!(h.get(0x10), Some(&Variant::Scalar(Elem::U32(7))));
            assert!(matches!(h.get(0x20), Some(Variant::Array { items, .. }) if items.len() == 2));
            assert!(h.get(0x30).is_none());
            fc4_props_destroy(props);
            assert!(fc4_dbpf_read_exemplar(p, key.type_id, 9, 9, std::ptr::null_mut()).is_null());
            fc4_dbpf_close(p);
        }
        let _ = std::fs::remove_file(dir);
        assert!(unsafe { fc4_dbpf_open(b"no/such.dat".as_ptr(), 11) }.is_null());
    }
}
