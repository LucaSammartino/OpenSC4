// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! List-returning host callbacks (fc4.h "list sinks"): the host hands a list
//! to the core by calling a sink one or more times (each call appends).
//! Used by the city detail manager, the port developers and the prop
//! developer.

use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::lot_developer::CfgObject;

/// Appends `len` tokens.
pub type fc4_u64s_sink = Option<unsafe extern "C" fn(ctx: *mut c_void, items: *const u64, len: usize)>;
/// Appends `len` u32 values.
pub type fc4_u32s_sink = Option<unsafe extern "C" fn(ctx: *mut c_void, items: *const u32, len: usize)>;

/// One lot configuration object (SDK `cSC4LotConfigurationObject`, a
/// 0x34-byte record of cfg slot 23): the fields the cores read.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_lot_cfg_object {
    /// 1 prop, 2 base texture, 4 flora, … (the type byte).
    pub object_type: u32,
    /// The object ID (record +0x24), the lot tag of the occupant made from it.
    pub tag: u32,
    /// The flag byte (bit 0: optional).
    pub flags: u8,
    pub reserved: [u8; 3],
    /// The ID list (record +0x28: prop IDs or family IDs).
    pub id_count: u32,
    pub ids: *const u32,
}

/// Appends `len` configuration objects (copied during the call).
pub type fc4_lot_cfg_objects_sink =
    Option<unsafe extern "C" fn(ctx: *mut c_void, items: *const fc4_lot_cfg_object, len: usize)>;

const _: () = {
    assert!(std::mem::size_of::<fc4_lot_cfg_object>() == if cfg!(target_pointer_width = "32") { 20 } else { 24 });
};

unsafe fn slice<'a, T>(p: *const T, n: usize) -> &'a [T] {
    if p.is_null() || n == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(p, n)
    }
}

pub(crate) unsafe extern "C" fn collect_u64s(ctx: *mut c_void, items: *const u64, len: usize) {
    (*(ctx as *mut Vec<u64>)).extend_from_slice(slice(items, len));
}

pub(crate) unsafe extern "C" fn collect_u32s(ctx: *mut c_void, items: *const u32, len: usize) {
    (*(ctx as *mut Vec<u32>)).extend_from_slice(slice(items, len));
}

pub(crate) unsafe extern "C" fn collect_cfg_objects(ctx: *mut c_void, items: *const fc4_lot_cfg_object, len: usize) {
    let v = &mut *(ctx as *mut Vec<CfgObject>);
    for o in slice(items, len) {
        v.push(CfgObject {
            object_type: o.object_type,
            flags: o.flags,
            ids: slice(o.ids, o.id_count as usize).to_vec(),
            tag: o.tag,
            ..CfgObject::default()
        });
    }
}

/// Calls `f(sink, ctx)` with a token-collecting sink.
pub(crate) fn tokens(f: impl FnOnce(fc4_u64s_sink, *mut c_void)) -> Vec<u64> {
    let mut v: Vec<u64> = Vec::new();
    f(Some(collect_u64s), &mut v as *mut Vec<u64> as *mut c_void);
    v
}

/// Calls `f(sink, ctx)` with a u32-collecting sink.
pub(crate) fn u32s(f: impl FnOnce(fc4_u32s_sink, *mut c_void)) -> Vec<u32> {
    let mut v: Vec<u32> = Vec::new();
    f(Some(collect_u32s), &mut v as *mut Vec<u32> as *mut c_void);
    v
}

/// Calls `f(sink, ctx)` with a configuration-object sink; `None` when `f`
/// returns false (no configuration).
pub(crate) fn cfg_objects(f: impl FnOnce(fc4_lot_cfg_objects_sink, *mut c_void) -> bool) -> Option<Rc<Vec<CfgObject>>> {
    let mut v: Vec<CfgObject> = Vec::new();
    f(Some(collect_cfg_objects), &mut v as *mut Vec<CfgObject> as *mut c_void).then(|| Rc::new(v))
}
