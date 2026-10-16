// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The prop manager over the C ABI (fc4.h "prop manager",
//! docs/specs/prop-lotconfig.md Part A).
//!
//! The host implements [`fc4_prop_host`]; the plugin's `cISC4PropManager`
//! (with its message-target and serializable sub-objects) forwards its slots
//! here. Props, buildings, lots, the occupant manager, the lot manager, the
//! resource manager, the view filter and text decals are host objects, named
//! by tokens (0 = none).
//!
//! **The catalogue** (§A2) is process-wide: the host creates one
//! [`fc4_prop_catalogue`] per process and hands it to every manager it
//! creates; the first `Init` that finds it unbuilt builds it.
//! `fc4_prop_catalogue_app_shutdown` is the application shutdown callback.
//!
//! **Lists** (§A7.2): the query slots take the caller's list (`in`, `n_in`),
//! may filter what was there before (quirk 7), and hand the resulting list
//! back element by element through a token callback. Every token handed back
//! that was not in `in` carries one reference for the caller; tokens of `in`
//! that were dropped were released.
//!
//! Threads: the core is single-threaded (`RefCell`). The plugin serialises
//! every call on one object (fc4.h).

use std::ffi::{c_char, c_void};
use std::rc::Rc;

use fc4_core::prop_manager::{
    query_interface, BuildingProps, BuiltinProp, CommandArg, PropCatalogue, PropHost, PropManager, PropMessage,
    PropProfile, PropService, SharedPropCatalogue,
};
use fc4_core::property::PropertyHolder;
use fc4_core::stream::{RefValue, ResKey, SaveObject};

use crate::budget::fc4_token_callback;
use crate::development::fc4_res_key;
use crate::guard;
use crate::hostgrid::{collect_tokens, fc4_cell_rect, take_props};
use crate::ordinance::fc4_key_visitor;
use crate::props::fc4_props;
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

/// `fc4_prop_host.acquire_service` services.
pub const FC4_PROP_SERVICE_OCCUPANT_MANAGER: u32 = 0;
pub const FC4_PROP_SERVICE_RESOURCE_MANAGER: u32 = 1;
pub const FC4_PROP_SERVICE_LOT_MANAGER: u32 = 2;

/// `fc4_prop_host.create_builtin_prop` kinds (§A6.1 step 3).
pub const FC4_PROP_BUILTIN_PLAIN: u32 = 0;
pub const FC4_PROP_BUILTIN_TERRAIN_DECAL: u32 = 1;
pub const FC4_PROP_BUILTIN_VISUAL_EFFECT: u32 = 2;

/// `fc4_prop_host.building_props` results (§A7.1 step 3).
pub const FC4_PROP_BUILDING_NO_HOLDER: u32 = 0;
pub const FC4_PROP_BUILDING_ABSENT: u32 = 1;
pub const FC4_PROP_BUILDING_PRESENT: u32 = 2;

#[repr(C)]
pub struct fc4_prop_catalogue {
    _private: [u8; 0],
}

#[repr(C)]
pub struct fc4_prop_manager {
    _private: [u8; 0],
}

/// What the prop manager needs from the game (fc4.h `fc4_prop_host`). A
/// missing callback reads as 0 / false / none.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_prop_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    // the catalogue (§A3)
    pub for_each_exemplar_key: Option<unsafe extern "C" fn(U, fc4_key_visitor, *mut c_void) -> bool>,
    pub load_exemplar: Option<unsafe extern "C" fn(U, u32, u32, u32) -> *mut fc4_props>,
    pub register_shutdown_callback: Option<unsafe extern "C" fn(U)>,
    pub post_exemplar_data_updated: Option<unsafe extern "C" fn(U, u64, u32)>,
    // lifecycle (§A4)
    pub register_command: Option<unsafe extern "C" fn(U, u32)>,
    pub unregister_command: Option<unsafe extern "C" fn(U, u32)>,
    pub subscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub acquire_service: Option<unsafe extern "C" fn(U, u32) -> u64>,
    pub view_filter: Option<unsafe extern "C" fn(U) -> u64>,
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    // creating props (§A6)
    pub create_prop_instance: Option<unsafe extern "C" fn(U, u32) -> u64>,
    pub create_builtin_prop: Option<unsafe extern "C" fn(U, u32) -> u64>,
    pub set_prop_type: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub attach_exemplar: Option<unsafe extern "C" fn(U, u64, u32, u32, u32, bool) -> *mut fc4_props>,
    pub set_bounding_box: Option<unsafe extern "C" fn(U, u64, *const f32, *const f32)>,
    // occupants (§A7)
    pub occupant_flag: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub occupant_init: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub occupant_shutdown: Option<unsafe extern "C" fn(U, u64)>,
    pub set_prop_usage: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub set_position: Option<unsafe extern "C" fn(U, u64, *const f32)>,
    pub set_orientation: Option<unsafe extern "C" fn(U, u64, i32)>,
    pub position: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    pub hide: Option<unsafe extern "C" fn(U, u64)>,
    // the occupant manager
    pub insert_occupant: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub remove_occupant: Option<unsafe extern "C" fn(U, u64)>,
    pub occupants_in_box:
        Option<unsafe extern "C" fn(U, *const f32, *const f32, u32, fc4_token_callback, *mut c_void) -> bool>,
    pub occupants_in_cell_rect:
        Option<unsafe extern "C" fn(U, *const fc4_cell_rect, u32, fc4_token_callback, *mut c_void) -> bool>,
    pub building_at_cell: Option<unsafe extern "C" fn(U, i32, i32) -> u64>,
    // lots and the city
    pub lot_at_cell: Option<unsafe extern "C" fn(U, i32, i32) -> u64>,
    pub lot_cell_rect: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    pub cell_to_world: Option<unsafe extern "C" fn(U, i32, i32, *mut f32, *mut f32)>,
    pub world_to_cell: Option<unsafe extern "C" fn(U, f32, f32, *mut i32, *mut i32)>,
    // the building-prop property (§A7.1 step 3)
    pub building_props: Option<unsafe extern "C" fn(U, u64, fc4_token_callback, *mut c_void) -> u32>,
    pub set_building_props: Option<unsafe extern "C" fn(U, u64, *const u64, usize)>,
    pub add_building_props_property: Option<unsafe extern "C" fn(U, u64, *const u64, usize)>,
    pub remove_building_props_property: Option<unsafe extern "C" fn(U, u64)>,
    // the CreateTextDecal command (§A4.3)
    pub create_text_decal: Option<unsafe extern "C" fn(U) -> u64>,
    pub terrain_altitude: Option<unsafe extern "C" fn(U, f32, f32) -> f32>,
    pub decal_set_text: Option<unsafe extern "C" fn(U, u64, *const c_char, usize)>,
    pub decal_slot8: Option<unsafe extern "C" fn(U, u64, *const f32)>,
    pub decal_slot10: Option<unsafe extern "C" fn(U, u64, f32)>,
    pub decal_slot6: Option<unsafe extern "C" fn(U, u64, f32)>,
    // save references (§A8)
    pub resolve_ref: Option<unsafe extern "C" fn(U, u64, u32) -> u64>,
}

/// The 0x1C-byte prop profile (§A3.2) in the original's layout: the bytes
/// are copied as the catalogue holds them (OccupantSize values past the
/// third land in the following fields, quirk 8).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_prop_profile {
    /// +0x00.
    pub class_id: u32,
    /// +0x04, +0x08, +0x0C.
    pub size: [f32; 3],
    /// +0x10.
    pub satisfaction: u32,
    /// +0x14.
    pub wealth: u8,
    /// +0x15 .. +0x17 (not written by the profile reader).
    pub wealth_pad: [u8; 3],
    /// +0x18.
    pub crime_effect: u32,
}

/// One argument of the `CreateTextDecal` command: its text and its value as
/// a float.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_prop_command_arg {
    pub text: *const c_char,
    pub text_len: u32,
    pub value: f32,
}

/// Diagnostics (`fc4_prop_manager_get_state`), every instance field of §A2.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_prop_manager_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub destroyed: u8,
    /// The filter object exists.
    pub filter: u8,
    pub reserved0: u8,
    pub ref_count: u32,
    /// The filter's usage.
    pub filter_usage: u32,
    pub occupant_manager: u64,
    pub resource_manager: u64,
    pub lot_manager: u64,
    pub view_filter: u64,
}

/// The process catalogue (`fc4_prop_catalogue_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_prop_catalogue_state {
    pub struct_size: u32,
    pub built: u8,
    pub callback_registered: u8,
    pub reserved0: u16,
    /// data2 of the catalogue message.
    pub data2: u32,
    pub types: u32,
    pub families: u32,
    pub profiles: u32,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_prop_profile>() == 0x1C);
    assert!(std::mem::size_of::<fc4_prop_manager_state>() == 48);
    assert!(std::mem::size_of::<fc4_prop_catalogue_state>() == 24);
    assert!(std::mem::size_of::<fc4_prop_command_arg>() == if cfg!(target_pointer_width = "32") { 12 } else { 16 });
    assert!(std::mem::size_of::<fc4_prop_host>() == if cfg!(target_pointer_width = "32") { 188 } else { 376 });
};

// ---- host adapter -----------------------------------------------------------------------------------

struct CHost {
    h: fc4_prop_host,
}

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.h.$f { Some(f) => unsafe { f($s.h.user $(, $arg)*) }, None => $default }
    };
}

fn opt(t: u64) -> Option<u64> {
    (t != 0).then_some(t)
}

unsafe extern "C" fn collect_key(ctx: *mut c_void, type_id: u32, group: u32, instance: u32) {
    if let Some(v) = (ctx as *mut Vec<ResKey>).as_mut() {
        v.push(ResKey {
            type_id,
            group,
            instance,
        });
    }
}

fn rect_c(r: (i32, i32, i32, i32)) -> fc4_cell_rect {
    fc4_cell_rect {
        x1: r.0,
        z1: r.1,
        x2: r.2,
        z2: r.3,
    }
}

impl PropHost for CHost {
    fn exemplar_keys(&self) -> Option<Vec<ResKey>> {
        let mut keys: Vec<ResKey> = Vec::new();
        let ctx = &mut keys as *mut Vec<ResKey> as *mut c_void;
        cb!(self, for_each_exemplar_key, false, Some(collect_key), ctx).then_some(keys)
    }
    fn load_exemplar(&self, key: ResKey) -> Option<Rc<PropertyHolder>> {
        let p = cb!(
            self,
            load_exemplar,
            std::ptr::null_mut(),
            key.type_id,
            key.group,
            key.instance
        );
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }
    fn register_shutdown_callback(&self) {
        cb!(self, register_shutdown_callback, ())
    }
    fn post_exemplar_data_updated(&self, manager: u64, data2: u32) {
        cb!(self, post_exemplar_data_updated, (), manager, data2)
    }
    fn register_command(&self, id: u32) {
        cb!(self, register_command, (), id)
    }
    fn unregister_command(&self, id: u32) {
        cb!(self, unregister_command, (), id)
    }
    fn subscribe(&self, msg_type: u32) {
        cb!(self, subscribe, (), msg_type)
    }
    fn unsubscribe(&self, msg_type: u32) {
        cb!(self, unsubscribe, (), msg_type)
    }
    fn acquire_service(&self, service: PropService) -> Option<u64> {
        let s = match service {
            PropService::OccupantManager => FC4_PROP_SERVICE_OCCUPANT_MANAGER,
            PropService::ResourceManager => FC4_PROP_SERVICE_RESOURCE_MANAGER,
            PropService::LotManager => FC4_PROP_SERVICE_LOT_MANAGER,
        };
        opt(cb!(self, acquire_service, 0, s))
    }
    fn view_filter(&self) -> Option<u64> {
        opt(cb!(self, view_filter, 0))
    }
    fn add_ref(&self, token: u64) {
        cb!(self, add_ref, (), token)
    }
    fn release(&self, token: u64) {
        cb!(self, release, (), token)
    }
    fn create_prop_instance(&self, class_id: u32) -> Option<u64> {
        opt(cb!(self, create_prop_instance, 0, class_id))
    }
    fn create_builtin_prop(&self, kind: BuiltinProp) -> u64 {
        let k = match kind {
            BuiltinProp::Plain => FC4_PROP_BUILTIN_PLAIN,
            BuiltinProp::TerrainDecal => FC4_PROP_BUILTIN_TERRAIN_DECAL,
            BuiltinProp::VisualEffect => FC4_PROP_BUILTIN_VISUAL_EFFECT,
        };
        cb!(self, create_builtin_prop, 0, k)
    }
    fn set_prop_type(&self, prop: u64, id: u32) {
        cb!(self, set_prop_type, (), prop, id)
    }
    fn attach_exemplar(&self, prop: u64, key: ResKey, global: bool) -> Option<Rc<PropertyHolder>> {
        let p = cb!(
            self,
            attach_exemplar,
            std::ptr::null_mut(),
            prop,
            key.type_id,
            key.group,
            key.instance,
            global
        );
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }
    fn set_bounding_box(&self, prop: u64, min: [f32; 3], max: [f32; 3]) {
        cb!(self, set_bounding_box, (), prop, min.as_ptr(), max.as_ptr())
    }
    fn occupant_flag(&self, occ: u64, flag: u32) -> bool {
        cb!(self, occupant_flag, false, occ, flag)
    }
    fn occupant_init(&self, occ: u64) -> bool {
        cb!(self, occupant_init, false, occ)
    }
    fn occupant_shutdown(&self, occ: u64) {
        cb!(self, occupant_shutdown, (), occ)
    }
    fn set_prop_usage(&self, occ: u64, usage: u32) {
        cb!(self, set_prop_usage, (), occ, usage)
    }
    fn set_position(&self, occ: u64, pos: [f32; 3]) {
        cb!(self, set_position, (), occ, pos.as_ptr())
    }
    fn set_orientation(&self, occ: u64, orientation: i32) {
        cb!(self, set_orientation, (), occ, orientation)
    }
    fn position(&self, occ: u64) -> [f32; 3] {
        let mut p = [0.0f32; 3];
        cb!(self, position, (), occ, p.as_mut_ptr());
        p
    }
    fn hide(&self, occ: u64) {
        cb!(self, hide, (), occ)
    }
    fn insert_occupant(&self, occ: u64) -> bool {
        cb!(self, insert_occupant, false, occ)
    }
    fn remove_occupant(&self, occ: u64) {
        cb!(self, remove_occupant, (), occ)
    }
    fn occupants_in_box(&self, min: [f32; 3], max: [f32; 3], usage: u32) -> Option<Vec<u64>> {
        let mut ok = false;
        let v =
            collect_tokens(|f, ctx| ok = cb!(self, occupants_in_box, false, min.as_ptr(), max.as_ptr(), usage, f, ctx));
        ok.then_some(v)
    }
    fn occupants_in_cell_rect(&self, rect: (i32, i32, i32, i32), usage: u32) -> Option<Vec<u64>> {
        let r = rect_c(rect);
        let mut ok = false;
        let v = collect_tokens(|f, ctx| ok = cb!(self, occupants_in_cell_rect, false, &r, usage, f, ctx));
        ok.then_some(v)
    }
    fn building_at_cell(&self, x: i32, z: i32) -> Option<u64> {
        opt(cb!(self, building_at_cell, 0, x, z))
    }
    fn lot_at_cell(&self, x: i32, z: i32) -> Option<u64> {
        opt(cb!(self, lot_at_cell, 0, x, z))
    }
    fn lot_cell_rect(&self, lot: u64) -> (i32, i32, i32, i32) {
        let mut r = fc4_cell_rect::default();
        cb!(self, lot_cell_rect, (), lot, &mut r);
        (r.x1, r.z1, r.x2, r.z2)
    }
    fn cell_to_world(&self, x: i32, z: i32) -> (f32, f32) {
        let (mut wx, mut wz) = (0.0f32, 0.0f32);
        cb!(self, cell_to_world, (), x, z, &mut wx, &mut wz);
        (wx, wz)
    }
    fn world_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        let (mut cx, mut cz) = (0i32, 0i32);
        cb!(self, world_to_cell, (), x, z, &mut cx, &mut cz);
        (cx, cz)
    }
    fn building_props(&self, building: u64) -> BuildingProps {
        let mut r = FC4_PROP_BUILDING_NO_HOLDER;
        let v = collect_tokens(|f, ctx| r = cb!(self, building_props, FC4_PROP_BUILDING_NO_HOLDER, building, f, ctx));
        match r {
            FC4_PROP_BUILDING_PRESENT => BuildingProps::Present(v),
            FC4_PROP_BUILDING_ABSENT => BuildingProps::Absent,
            _ => BuildingProps::NoHolder,
        }
    }
    fn set_building_props(&self, building: u64, props: &[u64]) {
        cb!(self, set_building_props, (), building, props.as_ptr(), props.len())
    }
    fn add_building_props_property(&self, building: u64, props: &[u64]) {
        cb!(
            self,
            add_building_props_property,
            (),
            building,
            props.as_ptr(),
            props.len()
        )
    }
    fn remove_building_props_property(&self, building: u64) {
        cb!(self, remove_building_props_property, (), building)
    }
    fn create_text_decal(&self) -> Option<u64> {
        opt(cb!(self, create_text_decal, 0))
    }
    fn terrain_altitude(&self, x: f32, z: f32) -> f32 {
        cb!(self, terrain_altitude, 0.0, x, z)
    }
    fn decal_set_text(&self, decal: u64, text: &str) {
        cb!(
            self,
            decal_set_text,
            (),
            decal,
            text.as_ptr() as *const c_char,
            text.len()
        )
    }
    fn decal_slot8(&self, decal: u64, values: [f32; 3]) {
        cb!(self, decal_slot8, (), decal, values.as_ptr())
    }
    fn decal_slot10(&self, decal: u64, value: f32) {
        cb!(self, decal_slot10, (), decal, value)
    }
    fn decal_slot6(&self, decal: u64, value: f32) {
        cb!(self, decal_slot6, (), decal, value)
    }
    fn object_ref(&self, token: u64) -> RefValue {
        if token == 0 {
            RefValue::Null
        } else {
            RefValue::Host { token, class_id: 0 }
        }
    }
    fn resolve_ref(&self, r: &RefValue, iid: u32) -> Option<u64> {
        match *r {
            RefValue::Host { token, .. } if token != 0 => opt(cb!(self, resolve_ref, 0, token, iid)),
            _ => None,
        }
    }
}

// ---- the process catalogue --------------------------------------------------------------------------

fn with_cat<R>(c: *const fc4_prop_catalogue, fallback: R, f: impl FnOnce(&SharedPropCatalogue) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_prop_catalogue*.
    match unsafe { (c as *const SharedPropCatalogue).as_ref() } {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

/// A new, unbuilt process catalogue (§A2).
#[no_mangle]
pub extern "C" fn fc4_prop_catalogue_create() -> *mut fc4_prop_catalogue {
    guard(std::ptr::null_mut(), || {
        Box::into_raw(Box::new(PropCatalogue::shared())) as *mut fc4_prop_catalogue
    })
}

/// Drops the host's handle; managers created with it keep the catalogue.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_catalogue_destroy(c: *mut fc4_prop_catalogue) {
    if !c.is_null() {
        guard((), || drop(Box::from_raw(c as *mut SharedPropCatalogue)));
    }
}

/// The application shutdown callback (`0x004A2A30`): tears the catalogue
/// down (the next `Init` rebuilds it).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_catalogue_app_shutdown(c: *mut fc4_prop_catalogue) {
    with_cat(c, (), |h| h.borrow_mut().app_shutdown())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_prop_catalogue_get_state(
    c: *const fc4_prop_catalogue,
    out: *mut fc4_prop_catalogue_state,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_cat(c, false, |h| {
        let cat = h.borrow();
        let (types, families, profiles) = cat.counts();
        *o = fc4_prop_catalogue_state {
            struct_size: std::mem::size_of::<fc4_prop_catalogue_state>() as u32,
            built: cat.is_built().into(),
            callback_registered: cat.callback_registered().into(),
            reserved0: 0,
            data2: cat.data2(),
            types: types as u32,
            families: families as u32,
            profiles: profiles as u32,
        };
        true
    })
}

// ---- handles ----------------------------------------------------------------------------------------

struct Handle {
    m: Rc<PropManager>,
}

fn with_m<R>(m: *const fc4_prop_manager, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_prop_manager*.
    match unsafe { (m as *const Handle).as_ref() } {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

unsafe fn copy_out<T: Copy>(v: &[T], out: *mut T, capacity: usize) -> usize {
    if !out.is_null() {
        for (i, x) in v.iter().take(capacity).enumerate() {
            *out.add(i) = *x;
        }
    }
    v.len()
}

unsafe fn slice_in<'a, T>(p: *const T, n: usize) -> &'a [T] {
    if p.is_null() || n == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(p, n)
    }
}

unsafe fn emit(list: &[u64], out: fc4_token_callback, ctx: *mut c_void) {
    if let Some(f) = out {
        for &t in list {
            f(ctx, t);
        }
    }
}

/// Creates a constructed manager (§A1: reference count 0) sharing
/// `catalogue` (NULL: a private catalogue of its own). The host table is
/// copied (NULL for a NULL or short table). `token` is the manager's own
/// host identity (void1 of the catalogue message).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_create(
    host: *const fc4_prop_host,
    catalogue: *const fc4_prop_catalogue,
    token: u64,
) -> *mut fc4_prop_manager {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_prop_host>() => {
            let cat = match (catalogue as *const SharedPropCatalogue).as_ref() {
                Some(c) => c.clone(),
                None => PropCatalogue::shared(),
            };
            let m = PropManager::new(Rc::new(CHost { h: *h }), cat, token);
            Box::into_raw(Box::new(Handle { m })) as *mut fc4_prop_manager
        }
        _ => std::ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_destroy(m: *mut fc4_prop_manager) {
    if !m.is_null() {
        guard((), || drop(Box::from_raw(m as *mut Handle)));
    }
}

/// `QueryInterface` (§A1).
#[no_mangle]
pub extern "C" fn fc4_prop_manager_query_interface(iid: u32) -> bool {
    query_interface(iid)
}

#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_add_ref(m: *mut fc4_prop_manager) -> u32 {
    with_m(m, 0, |h| h.m.add_ref())
}

/// `Release`: the last one runs `Shutdown` and marks the object destroyed
/// (0); the handle stays valid until [`fc4_prop_manager_destroy`].
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_release(m: *mut fc4_prop_manager) -> u32 {
    with_m(m, 0, |h| h.m.release())
}

/// `Init` (§A4.1): builds the catalogue when unbuilt (`StaticInit`).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_init(m: *mut fc4_prop_manager) -> bool {
    with_m(m, false, |h| h.m.init())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_shutdown(m: *mut fc4_prop_manager) -> bool {
    with_m(m, false, |h| h.m.shutdown())
}

/// `DoMessage` (§A4.3): `view_filter` is `*void2` of
/// `PostViewFilterChange` (0 = null); `args` the `CreateTextDecal`
/// command's arguments (`args` may be NULL when `n_args` is 0). Always true.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_do_message(
    m: *mut fc4_prop_manager,
    msg_type: u32,
    view_filter: u64,
    args: *const fc4_prop_command_arg,
    n_args: usize,
) -> bool {
    let args: Vec<CommandArg> = slice_in(args, n_args)
        .iter()
        .map(|a| CommandArg {
            text: String::from_utf8_lossy(slice_in(a.text as *const u8, a.text_len as usize)).into_owned(),
            value: a.value,
        })
        .collect();
    let msg = PropMessage {
        msg_type,
        view_filter: opt(view_filter),
        args,
    };
    with_m(m, true, |h| h.m.do_message(&msg))
}

// ---- catalogue queries (§A5) ----

/// Slot 5: every prop type in the catalogue's bucket order. Returns the
/// count; copies at most `capacity` (`out` may be NULL).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_all_prop_types(
    m: *const fc4_prop_manager,
    out: *mut u32,
    capacity: usize,
) -> usize {
    with_m(m, 0, |h| copy_out(&h.m.all_prop_types(), out, capacity))
}

/// Slot 6: every family, as slot 5.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_all_prop_family_types(
    m: *const fc4_prop_manager,
    out: *mut u32,
    capacity: usize,
) -> usize {
    with_m(m, 0, |h| copy_out(&h.m.all_prop_family_types(), out, capacity))
}

/// Slot 7: false for an unknown type.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_prop_key_from_type(
    m: *const fc4_prop_manager,
    id: u32,
    out: *mut fc4_res_key,
) -> bool {
    with_m(m, false, |h| match h.m.prop_key_from_type(id) {
        Some(k) => {
            if let Some(o) = out.as_mut() {
                *o = k.into();
            }
            true
        }
        None => false,
    })
}

/// Slot 8.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_is_prop_type_valid(m: *const fc4_prop_manager, id: u32) -> bool {
    with_m(m, false, |h| h.m.is_prop_type_valid(id))
}

/// Slot 9 `GetPropTypesByFamily(vec, id)` (quirk 1): the caller's vector
/// (`in`, `n_in`) after the call goes to `out` once; a family assigns its
/// members and returns their count, a type assigns `[id]` and returns 0,
/// anything else leaves the vector and returns 0.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_prop_types_by_family(
    m: *const fc4_prop_manager,
    id: u32,
    in_: *const u32,
    n_in: usize,
    out: crate::lotdev::fc4_u32s_sink,
    ctx: *mut c_void,
) -> u32 {
    let mut v = slice_in(in_, n_in).to_vec();
    let r = with_m(m, 0, |h| h.m.prop_types_by_family(&mut v, id));
    if let Some(f) = out {
        f(ctx, v.as_ptr(), v.len());
    }
    r
}

/// Slot 10.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_is_prop_in_family(m: *const fc4_prop_manager, ty: u32, family: u32) -> bool {
    with_m(m, false, |h| h.m.is_prop_in_family(ty, family))
}

/// Slot 11.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_is_prop_family_type_valid(m: *const fc4_prop_manager, id: u32) -> bool {
    with_m(m, false, |h| h.m.is_prop_family_type_valid(id))
}

/// Slot 12 (quirk 3): inserts an empty name and returns it. The name is
/// copied NUL-terminated (truncated to `capacity - 1`; `out` may be NULL);
/// returns its byte length.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_prop_name(
    m: *mut fc4_prop_manager,
    id: u32,
    out: *mut c_char,
    capacity: usize,
) -> usize {
    with_m(m, 0, |h| {
        let s = h.m.prop_name(id);
        if !out.is_null() && capacity > 0 {
            let n = s.len().min(capacity - 1);
            std::ptr::copy_nonoverlapping(s.as_ptr(), out as *mut u8, n);
            *out.add(n) = 0;
        }
        s.len()
    })
}

fn profile_c(p: &PropProfile) -> fc4_prop_profile {
    let w = |i: usize| p.word(i);
    fc4_prop_profile {
        class_id: w(0),
        size: [f32::from_bits(w(1)), f32::from_bits(w(2)), f32::from_bits(w(3))],
        satisfaction: w(4),
        wealth: p.raw[0x14],
        wealth_pad: [p.raw[0x15], p.raw[0x16], p.raw[0x17]],
        crime_effect: w(6),
    }
}

/// Slot 13 (quirk 2): the profile, inserting an all-zero one for an
/// unknown ID. False only for a NULL handle or `out`.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_prop_profile(
    m: *mut fc4_prop_manager,
    id: u32,
    out: *mut fc4_prop_profile,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_m(m, false, |h| {
        *o = profile_c(&h.m.prop_profile(id));
        true
    })
}

/// Slot 14: false (nothing written) for an unknown ID.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_prop_profile_ptr(
    m: *const fc4_prop_manager,
    id: u32,
    out: *mut fc4_prop_profile,
) -> bool {
    with_m(m, false, |h| match h.m.prop_profile_ptr(id) {
        Some(p) => {
            if let Some(o) = out.as_mut() {
                *o = profile_c(&p);
            }
            true
        }
        None => false,
    })
}

// ---- creating and adding (§A6, §A7.1) ----

/// Slot 15 `CreateProp(id)`: the new prop with one reference (the
/// caller's), 0 when `id` is not a prop type.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_create_prop(m: *mut fc4_prop_manager, id: u32) -> u64 {
    with_m(m, 0, |h| h.m.create_prop(id).unwrap_or(0))
}

/// Slot 16 `CreateGenericProp(key)`: 0 when the exemplar does not load.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_create_generic_prop(
    m: *mut fc4_prop_manager,
    type_id: u32,
    group: u32,
    instance: u32,
) -> u64 {
    let key = ResKey {
        type_id,
        group,
        instance,
    };
    with_m(m, 0, |h| h.m.create_generic_prop(key).unwrap_or(0))
}

/// Slot 19 `AddCityProp(occ)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_add_city_prop(m: *mut fc4_prop_manager, occ: u64) -> bool {
    with_m(m, false, |h| h.m.add_city_prop(occ))
}

/// Slot 25 `AddLotProp(lot, occ)` (the lot is ignored, quirk 6).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_add_lot_prop(m: *mut fc4_prop_manager, lot: u64, occ: u64) -> bool {
    with_m(m, false, |h| h.m.add_lot_prop(opt(lot), occ))
}

unsafe fn pos_in(p: *const f32) -> [f32; 3] {
    match slice_in(p, 3) {
        [x, y, z] => [*x, *y, *z],
        _ => [0.0; 3],
    }
}

/// Slots 20 / 26 / 32 `AddCityProp` / `AddLotProp` / `AddStreetProp(id,
/// pos, orientation)` (`usage` 1, 2 or 4): the occupant (no reference for
/// the caller), 0 when it was not added. `pos3` is three floats.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_add_prop_new(
    m: *mut fc4_prop_manager,
    usage: u32,
    id: u32,
    pos3: *const f32,
    orientation: i32,
) -> u64 {
    let pos = pos_in(pos3);
    with_m(m, 0, |h| {
        match usage {
            1 => h.m.add_city_prop_new(id, pos, orientation),
            2 => h.m.add_lot_prop_new(id, pos, orientation),
            4 => h.m.add_street_prop(id, pos, orientation),
            _ => None,
        }
        .unwrap_or(0)
    })
}

/// Slot 36 `AddBuildingProp(building, prop)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_add_building_prop(
    m: *mut fc4_prop_manager,
    building: u64,
    prop: u64,
) -> bool {
    with_m(m, false, |h| h.m.add_building_prop(building, prop))
}

// ---- queries (§A7.2) ----

/// Runs a list query on the caller's list `in` and hands the resulting list
/// to `out` (for a NULL handle: `in` unchanged).
unsafe fn query(
    m: *mut fc4_prop_manager,
    in_: *const u64,
    n_in: usize,
    out: fc4_token_callback,
    ctx: *mut c_void,
    f: impl FnOnce(&PropManager, &mut Vec<u64>) -> bool,
) -> bool {
    let mut list = slice_in(in_, n_in).to_vec();
    let fallback = list.clone();
    let mut ran = false;
    let r = with_m(m, false, |h| {
        ran = true;
        f(&h.m, &mut list)
    });
    emit(if ran { &list } else { &fallback }, out, ctx);
    r
}

/// Slot 17 `GetCityProps(list, SC4Rect<float>)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_get_city_props_world(
    m: *mut fc4_prop_manager,
    x1: f32,
    z1: f32,
    x2: f32,
    z2: f32,
    in_: *const u64,
    n_in: usize,
    out: fc4_token_callback,
    ctx: *mut c_void,
) -> bool {
    query(m, in_, n_in, out, ctx, |p, l| {
        p.get_city_props_world(l, (x1, z1, x2, z2))
    })
}

/// Slots 18 / 30 `GetCityProps` / `GetStreetProps(list, SC4Rect<int32>)`
/// (`usage` 1 or 4).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_get_props_in_cells(
    m: *mut fc4_prop_manager,
    usage: u32,
    rect: *const fc4_cell_rect,
    in_: *const u64,
    n_in: usize,
    out: fc4_token_callback,
    ctx: *mut c_void,
) -> bool {
    let r = rect.as_ref().copied().unwrap_or_default();
    let r = (r.x1, r.z1, r.x2, r.z2);
    query(m, in_, n_in, out, ctx, |p, l| match usage {
        1 => p.get_city_props_cells(l, r),
        4 => p.get_street_props_cells(l, r),
        _ => false,
    })
}

/// Slots 23 / 31 / 34 `GetLotProps` / `GetStreetProps` / `GetBuildingProps(list,
/// x, z)` (`usage` 2, 4 or 3). The street query is always false (quirk 4).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_get_props_at(
    m: *mut fc4_prop_manager,
    usage: u32,
    x: i32,
    z: i32,
    in_: *const u64,
    n_in: usize,
    out: fc4_token_callback,
    ctx: *mut c_void,
) -> bool {
    query(m, in_, n_in, out, ctx, |p, l| match usage {
        2 => p.get_lot_props_at(l, x, z),
        3 => p.get_building_props_at(l, x, z),
        4 => p.get_street_props_at(l, x, z),
        _ => false,
    })
}

/// Slots 24 / 35 `GetLotProps(list, lot)` / `GetBuildingProps(list,
/// building)` (`usage` 2 or 3; `object` 0 = null).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_get_props_of(
    m: *mut fc4_prop_manager,
    usage: u32,
    object: u64,
    in_: *const u64,
    n_in: usize,
    out: fc4_token_callback,
    ctx: *mut c_void,
) -> bool {
    query(m, in_, n_in, out, ctx, |p, l| match usage {
        2 => p.get_lot_props(l, opt(object)),
        3 => p.get_building_props(l, opt(object)),
        _ => false,
    })
}

// ---- removing (§A7.3) ----

/// Slot 21 `RemoveCityProps(SC4Rect<float>)`. True.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_remove_city_props_world(
    m: *mut fc4_prop_manager,
    x1: f32,
    z1: f32,
    x2: f32,
    z2: f32,
) -> bool {
    with_m(m, false, |h| h.m.remove_city_props_world((x1, z1, x2, z2)))
}

/// Slot 22 `RemoveCityProps(SC4Rect<int32>)`. True.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_remove_city_props_cells(
    m: *mut fc4_prop_manager,
    rect: *const fc4_cell_rect,
) -> bool {
    let r = rect.as_ref().copied().unwrap_or_default();
    with_m(m, false, |h| h.m.remove_city_props_cells((r.x1, r.z1, r.x2, r.z2)))
}

/// Slots 27 / 33 / 37 `RemoveLotProps` / `RemoveStreetPropsFromTile` /
/// `RemoveBuildingProps(x, z)` (`usage` 2, 4 or 3). Slot 33 returns nothing
/// (true here).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_remove_props_at(
    m: *mut fc4_prop_manager,
    usage: u32,
    x: i32,
    z: i32,
) -> bool {
    with_m(m, false, |h| match usage {
        2 => h.m.remove_lot_props_at(x, z),
        3 => h.m.remove_building_props_at(x, z),
        4 => {
            h.m.remove_street_props_from_tile(x, z);
            true
        }
        _ => false,
    })
}

/// Slots 28 / 38 `RemoveLotProps(lot)` / `RemoveBuildingProps(building)`
/// (`usage` 2 or 3; `object` 0 = null).
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_remove_props_of(m: *mut fc4_prop_manager, usage: u32, object: u64) -> bool {
    with_m(m, false, |h| match usage {
        2 => h.m.remove_lot_props(opt(object)),
        3 => h.m.remove_building_props(opt(object)),
        _ => false,
    })
}

/// Slot 29 `RemoveProp(occ)`. True.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_remove_prop(m: *mut fc4_prop_manager, occ: u64) -> bool {
    with_m(m, false, |h| h.m.remove_prop(occ))
}

/// Slot 39 `RemoveBuildingProp(building, prop)`: whether it was found.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_remove_building_prop(
    m: *mut fc4_prop_manager,
    building: u64,
    prop: u64,
) -> bool {
    with_m(m, false, |h| h.m.remove_building_prop(opt(building), prop))
}

// ---- diagnostics and the record (§A8) ----

#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_get_state(
    m: *const fc4_prop_manager,
    out: *mut fc4_prop_manager_state,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_m(m, false, |h| {
        let s = h.m.snapshot();
        *o = fc4_prop_manager_state {
            struct_size: std::mem::size_of::<fc4_prop_manager_state>() as u32,
            initialized: s.initialized.into(),
            destroyed: s.destroyed.into(),
            filter: s.filter.into(),
            reserved0: 0,
            ref_count: s.ref_count,
            filter_usage: s.filter_usage,
            occupant_manager: s.occupant_manager.unwrap_or(0),
            resource_manager: s.resource_manager.unwrap_or(0),
            lot_manager: s.lot_manager.unwrap_or(0),
            view_filter: s.view_filter.unwrap_or(0),
        };
        true
    })
}

/// `Write` (§A8): version 1, the occupant manager and the lot manager.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_write(m: *const fc4_prop_manager, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_m(m, false, |h| h.m.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

/// `Read` (§A8): false for any other version, and then nothing changes
/// (references read so far are released). The references come back through
/// `resolve_ref` with IIDs `0xC6BB8334` and `0x681905BD`.
#[no_mangle]
pub unsafe extern "C" fn fc4_prop_manager_read(m: *mut fc4_prop_manager, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_m(m, false, |h| {
        h.m.read(&mut CIn {
            stream: i,
            decode: no_core_objects,
        })
    })
}
