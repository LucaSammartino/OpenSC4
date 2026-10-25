// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The lot configuration manager over the C ABI (fc4.h "lot configuration
//! manager", docs/specs/prop-lotconfig.md Part B).
//!
//! The host implements [`fc4_lot_config_host`]; the plugin's
//! `cISC4LotConfigurationManager` (with its serializable sub-object)
//! forwards its slots here. Lot configurations, filters, the building
//! development simulator, the flora simulator and the prop manager are host
//! objects, named by tokens (0 = none). Configuration objects cross the ABI
//! as [`fc4_lotdev_cfg_object`] (the lot developer's record): `object_type`
//! is the type byte, `byte3` the mirror flag, `footprint_max` / `footprint_min`
//! the (x, z) corners.
//!
//! **The catalogue** (§B2) is process-wide: the host creates one
//! [`fc4_lot_config_catalogue`] per process and hands it to every manager.
//! [`fc4_lot_config_object_array`] is the configuration class's object
//! reader (§B3.3), in the core so that every host builds the same arrays.
//!
//! Threads: the core is single-threaded (`RefCell`). The plugin serialises
//! every call on one object (fc4.h).

use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::lot_config_manager::{
    lot_object_array, query_interface, BuildingInfo, LcmService, LotConfigCatalogue, LotConfigHost, LotConfigManager,
    LotConfigObject, SharedLotConfigCatalogue,
};
use fc4_core::property::PropertyHolder;
use fc4_core::stl::StlHashSet;
use fc4_core::stream::{RefValue, ResKey, SaveObject};

use crate::budget::fc4_token_callback;
use crate::guard;
use crate::hostgrid::take_props;
use crate::lotdev::{fc4_lotdev_cfg_object, fc4_lotdev_cfg_object_callback, fc4_u32s_sink};
use crate::ordinance::fc4_key_visitor;
use crate::props::{fc4_props, props_ref};
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

/// `fc4_lot_config_host.acquire_service` services.
pub const FC4_LCM_SERVICE_BUILDING_DEVELOPMENT: u32 = 0;
pub const FC4_LCM_SERVICE_FLORA: u32 = 1;
pub const FC4_LCM_SERVICE_PROP_MANAGER: u32 = 2;

#[repr(C)]
pub struct fc4_lot_config_catalogue {
    _private: [u8; 0],
}

#[repr(C)]
pub struct fc4_lot_config_manager {
    _private: [u8; 0],
}

/// What `0x0048AD40` stores in a configuration (§B4.3).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_lot_config_building_info {
    /// +0x3E.
    pub min_capacity: u16,
    /// +0x40.
    pub max_capacity: u16,
    /// +0x8C.
    pub non_monopoly: u8,
    pub reserved: [u8; 3],
    /// +0x80: the occupant groups, ascending.
    pub n_groups: u32,
    pub groups: *const u32,
}

/// What the lot configuration manager needs from the game (fc4.h
/// `fc4_lot_config_host`). A missing callback reads as 0 / false / none.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_lot_config_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    // the catalogue (§B3)
    pub detail_level: Option<unsafe extern "C" fn(U) -> u32>,
    pub for_each_exemplar_key: Option<unsafe extern "C" fn(U, fc4_key_visitor, *mut c_void) -> bool>,
    pub register_shutdown_callback: Option<unsafe extern "C" fn(U)>,
    pub unregister_shutdown_callback: Option<unsafe extern "C" fn(U)>,
    pub create_configuration: Option<unsafe extern "C" fn(U) -> u64>,
    pub load_configuration: Option<unsafe extern "C" fn(U, u64, u32, u32, u32, u32) -> bool>,
    // configuration slots
    pub config_id: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub config_belongs_to_family: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub config_families: Option<unsafe extern "C" fn(U, u64, fc4_u32s_sink, *mut c_void)>,
    pub config_size: Option<unsafe extern "C" fn(U, u64, *mut u32, *mut u32)>,
    pub config_objects: Option<unsafe extern "C" fn(U, u64, fc4_lotdev_cfg_object_callback, *mut c_void)>,
    pub set_config_objects: Option<unsafe extern "C" fn(U, u64, *const fc4_lotdev_cfg_object, usize)>,
    pub config_primary_building:
        Option<unsafe extern "C" fn(U, u64, fc4_lotdev_cfg_object_callback, *mut c_void) -> bool>,
    pub set_config_building_info: Option<unsafe extern "C" fn(U, u64, *const fc4_lot_config_building_info)>,
    pub config_retaining_walls: Option<unsafe extern "C" fn(U, u64, fc4_u32s_sink, *mut c_void)>,
    pub set_config_retaining_walls: Option<unsafe extern "C" fn(U, u64, *const u32, usize)>,
    pub is_retaining_wall_valid: Option<unsafe extern "C" fn(U, u32) -> bool>,
    pub create_generic_configuration:
        Option<unsafe extern "C" fn(U, u32, u32, u32, *const fc4_lotdev_cfg_object, usize) -> u64>,
    // lifecycle and references
    pub acquire_service: Option<unsafe extern "C" fn(U, u32) -> u64>,
    pub subscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    // filters
    pub filter_matches: Option<unsafe extern "C" fn(U, u64, u64) -> bool>,
    // the building development simulator
    pub building_exemplar: Option<unsafe extern "C" fn(U, u32) -> *mut fc4_props>,
    pub is_building_type_valid: Option<unsafe extern "C" fn(U, u32) -> bool>,
    pub is_building_family_type_valid: Option<unsafe extern "C" fn(U, u32) -> bool>,
    pub building_types_by_family: Option<unsafe extern "C" fn(U, u32, fc4_u32s_sink, *mut c_void) -> u32>,
    pub building_types_into_set: Option<unsafe extern "C" fn(U, u32, fc4_u32s_sink, *mut c_void) -> u32>,
    pub is_building_in_family: Option<unsafe extern "C" fn(U, u32, u32) -> bool>,
    pub building_capacity: Option<unsafe extern "C" fn(U, u32) -> u16>,
    pub building_footprint: Option<unsafe extern "C" fn(U, u32, *mut f32, *mut f32) -> bool>,
    // the flora simulator (global)
    pub flora_radius: Option<unsafe extern "C" fn(U, u32, *mut f32) -> bool>,
    pub is_flora_type_valid: Option<unsafe extern "C" fn(U, u32) -> bool>,
    pub flora_types_into_set: Option<unsafe extern "C" fn(U, u32, fc4_u32s_sink, *mut c_void) -> u32>,
    // the prop manager (global)
    pub is_prop_type_valid: Option<unsafe extern "C" fn(U, u32) -> bool>,
    pub prop_types_by_family: Option<unsafe extern "C" fn(U, u32, fc4_u32s_sink, *mut c_void) -> u32>,
    pub is_prop_family_type_valid: Option<unsafe extern "C" fn(U, u32) -> bool>,
    pub prop_size: Option<unsafe extern "C" fn(U, u32, *mut f32, *mut f32) -> bool>,
    // save references (§B6)
    pub resolve_ref: Option<unsafe extern "C" fn(U, u64, u32) -> u64>,
}

/// Diagnostics (`fc4_lot_config_get_state`), the instance fields of §B2.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_lot_config_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub destroyed: u8,
    pub reserved0: u16,
    pub ref_count: u32,
    /// Registered filters.
    pub filters: u32,
    pub building_development: u64,
    pub flora: u64,
    pub prop_manager: u64,
}

/// The process catalogue (`fc4_lot_config_catalogue_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_lot_config_catalogue_state {
    pub struct_size: u32,
    pub loaded: u8,
    pub callback_registered: u8,
    pub reserved0: u16,
    /// The detail level the catalogue was built for.
    pub detail: u32,
    pub configurations: u32,
    pub families: u32,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_lot_config_state>() == 40);
    assert!(std::mem::size_of::<fc4_lot_config_catalogue_state>() == 20);
    assert!(
        std::mem::size_of::<fc4_lot_config_building_info>() == if cfg!(target_pointer_width = "32") { 16 } else { 24 }
    );
    assert!(std::mem::size_of::<fc4_lot_config_host>() == if cfg!(target_pointer_width = "32") { 168 } else { 336 });
};

// ---- objects across the ABI -------------------------------------------------------------------------

/// SAFETY: `o` valid; its `ids` NULL or `n_ids` values.
unsafe fn object_in(o: &fc4_lotdev_cfg_object) -> LotConfigObject {
    LotConfigObject {
        kind: o.object_type as u8,
        flags: o.flags,
        orientation: o.orientation,
        mirror: o.byte3,
        offset: o.offset,
        max_x: o.footprint_max[0],
        max_z: o.footprint_max[1],
        min_x: o.footprint_min[0],
        min_z: o.footprint_min[1],
        foundation: o.foundation,
        tag: o.tag,
        ids: slice_in(o.ids, o.n_ids as usize).to_vec(),
    }
}

/// The C view of `o`; its `ids` borrow `o.ids`.
fn object_out(o: &LotConfigObject) -> fc4_lotdev_cfg_object {
    fc4_lotdev_cfg_object {
        object_type: u32::from(o.kind),
        flags: o.flags,
        orientation: o.orientation,
        byte3: o.mirror,
        offset: o.offset,
        footprint_max: [o.max_x, o.max_z],
        footprint_min: [o.min_x, o.min_z],
        foundation: o.foundation,
        tag: o.tag,
        n_ids: o.ids.len() as u32,
        ids: if o.ids.is_empty() {
            std::ptr::null()
        } else {
            o.ids.as_ptr()
        },
    }
}

unsafe extern "C" fn push_object(ctx: *mut c_void, o: *const fc4_lotdev_cfg_object) {
    if let (Some(v), Some(o)) = ((ctx as *mut Vec<LotConfigObject>).as_mut(), o.as_ref()) {
        v.push(object_in(o));
    }
}

unsafe fn slice_in<'a, T>(p: *const T, n: usize) -> &'a [T] {
    if p.is_null() || n == 0 {
        &[]
    } else {
        std::slice::from_raw_parts(p, n)
    }
}

/// A sink that records whether it was called and the values.
#[derive(Default)]
struct Assigned {
    called: bool,
    values: Vec<u32>,
}

unsafe extern "C" fn assign_u32s(ctx: *mut c_void, values: *const u32, n: usize) {
    if let Some(a) = (ctx as *mut Assigned).as_mut() {
        a.called = true;
        a.values.extend_from_slice(slice_in(values, n));
    }
}

fn collect<R>(f: impl FnOnce(fc4_u32s_sink, *mut c_void) -> R) -> (R, Assigned) {
    let mut a = Assigned::default();
    let r = f(Some(assign_u32s), &mut a as *mut Assigned as *mut c_void);
    (r, a)
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

// ---- host adapter -----------------------------------------------------------------------------------

struct CHost {
    h: fc4_lot_config_host,
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

impl CHost {
    fn objects_of(&self, f: impl FnOnce(fc4_lotdev_cfg_object_callback, *mut c_void)) -> Vec<LotConfigObject> {
        let mut v: Vec<LotConfigObject> = Vec::new();
        f(Some(push_object), &mut v as *mut Vec<LotConfigObject> as *mut c_void);
        v
    }
}

impl LotConfigHost for CHost {
    fn detail_level(&self) -> u32 {
        cb!(self, detail_level, 0)
    }
    fn exemplar_keys(&self) -> Option<Vec<ResKey>> {
        let mut keys: Vec<ResKey> = Vec::new();
        let ctx = &mut keys as *mut Vec<ResKey> as *mut c_void;
        cb!(self, for_each_exemplar_key, false, Some(collect_key), ctx).then_some(keys)
    }
    fn register_shutdown_callback(&self) {
        cb!(self, register_shutdown_callback, ())
    }
    fn unregister_shutdown_callback(&self) {
        cb!(self, unregister_shutdown_callback, ())
    }
    fn create_configuration(&self) -> u64 {
        cb!(self, create_configuration, 0)
    }
    fn load_configuration(&self, cfg: u64, key: ResKey, detail: u32) -> bool {
        cb!(
            self,
            load_configuration,
            false,
            cfg,
            key.type_id,
            key.group,
            key.instance,
            detail
        )
    }
    fn config_id(&self, cfg: u64) -> u32 {
        cb!(self, config_id, 0, cfg)
    }
    fn config_belongs_to_family(&self, cfg: u64, family: u32) -> bool {
        cb!(self, config_belongs_to_family, false, cfg, family)
    }
    fn config_families(&self, cfg: u64) -> Vec<u32> {
        collect(|f, ctx| cb!(self, config_families, (), cfg, f, ctx)).1.values
    }
    fn config_size(&self, cfg: u64) -> (u32, u32) {
        let (mut x, mut z) = (0u32, 0u32);
        cb!(self, config_size, (), cfg, &mut x, &mut z);
        (x, z)
    }
    fn config_objects(&self, cfg: u64) -> Vec<LotConfigObject> {
        self.objects_of(|f, ctx| cb!(self, config_objects, (), cfg, f, ctx))
    }
    fn set_config_objects(&self, cfg: u64, objects: Vec<LotConfigObject>) {
        let c: Vec<fc4_lotdev_cfg_object> = objects.iter().map(object_out).collect();
        cb!(self, set_config_objects, (), cfg, c.as_ptr(), c.len())
    }
    fn config_primary_building(&self, cfg: u64) -> Option<LotConfigObject> {
        let mut found = false;
        let mut v = self.objects_of(|f, ctx| found = cb!(self, config_primary_building, false, cfg, f, ctx));
        if found {
            v.pop()
        } else {
            None
        }
    }
    fn set_config_building_info(&self, cfg: u64, info: &BuildingInfo) {
        let c = fc4_lot_config_building_info {
            min_capacity: info.min_capacity,
            max_capacity: info.max_capacity,
            non_monopoly: info.non_monopoly.into(),
            reserved: [0; 3],
            n_groups: info.groups.len() as u32,
            groups: if info.groups.is_empty() {
                std::ptr::null()
            } else {
                info.groups.as_ptr()
            },
        };
        cb!(self, set_config_building_info, (), cfg, &c)
    }
    fn config_retaining_walls(&self, cfg: u64) -> Vec<u32> {
        collect(|f, ctx| cb!(self, config_retaining_walls, (), cfg, f, ctx))
            .1
            .values
    }
    fn set_config_retaining_walls(&self, cfg: u64, ids: Vec<u32>) {
        cb!(self, set_config_retaining_walls, (), cfg, ids.as_ptr(), ids.len())
    }
    fn is_retaining_wall_valid(&self, id: u32) -> bool {
        cb!(self, is_retaining_wall_valid, false, id)
    }
    fn create_generic_configuration(&self, ty: u32, size: (u32, u32), objects: Vec<LotConfigObject>) -> Option<u64> {
        let c: Vec<fc4_lotdev_cfg_object> = objects.iter().map(object_out).collect();
        opt(cb!(
            self,
            create_generic_configuration,
            0,
            ty,
            size.0,
            size.1,
            c.as_ptr(),
            c.len()
        ))
    }
    fn acquire_service(&self, service: LcmService) -> Option<u64> {
        let s = match service {
            LcmService::BuildingDevelopment => FC4_LCM_SERVICE_BUILDING_DEVELOPMENT,
            LcmService::Flora => FC4_LCM_SERVICE_FLORA,
            LcmService::PropManager => FC4_LCM_SERVICE_PROP_MANAGER,
        };
        opt(cb!(self, acquire_service, 0, s))
    }
    fn subscribe(&self, msg_type: u32) {
        cb!(self, subscribe, (), msg_type)
    }
    fn unsubscribe(&self, msg_type: u32) {
        cb!(self, unsubscribe, (), msg_type)
    }
    fn add_ref(&self, token: u64) {
        cb!(self, add_ref, (), token)
    }
    fn release(&self, token: u64) {
        cb!(self, release, (), token)
    }
    fn filter_matches(&self, filter: u64, cfg: u64) -> bool {
        cb!(self, filter_matches, false, filter, cfg)
    }
    fn building_exemplar(&self, ty: u32) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, building_exemplar, std::ptr::null_mut(), ty);
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }
    fn is_building_type_valid(&self, id: u32) -> bool {
        cb!(self, is_building_type_valid, false, id)
    }
    fn is_building_family_type_valid(&self, id: u32) -> bool {
        cb!(self, is_building_family_type_valid, false, id)
    }
    fn building_types_by_family(&self, vec: &mut Vec<u32>, id: u32) -> u32 {
        let (r, a) = collect(|f, ctx| cb!(self, building_types_by_family, 0, id, f, ctx));
        if a.called {
            *vec = a.values;
        }
        r
    }
    fn building_types_into_set(&self, set: &mut StlHashSet, id: u32) -> u32 {
        let (r, a) = collect(|f, ctx| cb!(self, building_types_into_set, 0, id, f, ctx));
        for v in a.values {
            set.insert(v);
        }
        r
    }
    fn is_building_in_family(&self, ty: u32, family: u32) -> bool {
        cb!(self, is_building_in_family, false, ty, family)
    }
    fn building_capacity(&self, ty: u32) -> u16 {
        cb!(self, building_capacity, 0, ty)
    }
    fn building_footprint(&self, ty: u32) -> Option<(f32, f32)> {
        let (mut w, mut d) = (0.0f32, 0.0f32);
        cb!(self, building_footprint, false, ty, &mut w, &mut d).then_some((w, d))
    }
    fn flora_radius(&self, id: u32) -> Option<f32> {
        let mut r = 0.0f32;
        cb!(self, flora_radius, false, id, &mut r).then_some(r)
    }
    fn is_flora_type_valid(&self, id: u32) -> bool {
        cb!(self, is_flora_type_valid, false, id)
    }
    fn flora_types_into_set(&self, set: &mut StlHashSet, id: u32) -> u32 {
        let (r, a) = collect(|f, ctx| cb!(self, flora_types_into_set, 0, id, f, ctx));
        for v in a.values {
            set.insert(v);
        }
        r
    }
    fn is_prop_type_valid(&self, id: u32) -> bool {
        cb!(self, is_prop_type_valid, false, id)
    }
    fn prop_types_by_family(&self, vec: &mut Vec<u32>, id: u32) -> u32 {
        let (r, a) = collect(|f, ctx| cb!(self, prop_types_by_family, 0, id, f, ctx));
        if a.called {
            *vec = a.values;
        }
        r
    }
    fn is_prop_family_type_valid(&self, id: u32) -> bool {
        cb!(self, is_prop_family_type_valid, false, id)
    }
    fn prop_size(&self, id: u32) -> Option<(f32, f32)> {
        let (mut x, mut z) = (0.0f32, 0.0f32);
        cb!(self, prop_size, false, id, &mut x, &mut z).then_some((x, z))
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

// ---- the object reader (§B3.3) ----------------------------------------------------------------------

/// `ConvertPropertiesToObjectArray` (§B3.3): the object array of a lot
/// configuration exemplar at detail `detail`, each object to `visit` in
/// order (its `ids` valid during the call). `next_serial` (may be NULL: a
/// counter from 0) is the generated tag of a configuration below version 2,
/// advanced once per kept object. Returns the number of objects.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_object_array(
    exemplar: *const fc4_props,
    detail: u32,
    next_serial: *mut u32,
    visit: fc4_lotdev_cfg_object_callback,
    ctx: *mut c_void,
) -> usize {
    let Some(h) = props_ref(exemplar) else { return 0 };
    let mut serial = next_serial.as_ref().copied().unwrap_or(0);
    let objects = guard(Vec::new(), || lot_object_array(h, detail, &mut serial));
    if let Some(p) = next_serial.as_mut() {
        *p = serial;
    }
    if let Some(f) = visit {
        for o in &objects {
            let c = object_out(o);
            f(ctx, &c);
        }
    }
    objects.len()
}

// ---- the process catalogue --------------------------------------------------------------------------

fn with_cat<R>(c: *const fc4_lot_config_catalogue, fallback: R, f: impl FnOnce(&SharedLotConfigCatalogue) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_lot_config_catalogue*.
    match unsafe { (c as *const SharedLotConfigCatalogue).as_ref() } {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

/// A new, unloaded process catalogue (§B2).
#[no_mangle]
pub extern "C" fn fc4_lot_config_catalogue_create() -> *mut fc4_lot_config_catalogue {
    guard(std::ptr::null_mut(), || {
        Box::into_raw(Box::new(LotConfigCatalogue::shared())) as *mut fc4_lot_config_catalogue
    })
}

/// Drops the host's handle; managers created with it keep the catalogue.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_catalogue_destroy(c: *mut fc4_lot_config_catalogue) {
    if !c.is_null() {
        guard((), || drop(Box::from_raw(c as *mut SharedLotConfigCatalogue)));
    }
}

#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_catalogue_get_state(
    c: *const fc4_lot_config_catalogue,
    out: *mut fc4_lot_config_catalogue_state,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_cat(c, false, |h| {
        let cat = h.borrow();
        *o = fc4_lot_config_catalogue_state {
            struct_size: std::mem::size_of::<fc4_lot_config_catalogue_state>() as u32,
            loaded: cat.is_loaded().into(),
            callback_registered: cat.callback_registered().into(),
            reserved0: 0,
            detail: cat.detail(),
            configurations: cat.len() as u32,
            families: cat.family_set().len() as u32,
        };
        true
    })
}

// ---- handles ----------------------------------------------------------------------------------------

struct Handle {
    m: Rc<LotConfigManager>,
}

fn with_m<R>(m: *const fc4_lot_config_manager, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_lot_config_manager*.
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

unsafe fn emit(list: &[u64], out: fc4_token_callback, ctx: *mut c_void) {
    if let Some(f) = out {
        for &t in list {
            f(ctx, t);
        }
    }
}

/// Creates a constructed manager (§B1: reference count 0) sharing
/// `catalogue` (NULL: a private catalogue of its own). The host table is
/// copied (NULL for a NULL or short table).
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_create(
    host: *const fc4_lot_config_host,
    catalogue: *const fc4_lot_config_catalogue,
    token: u64,
) -> *mut fc4_lot_config_manager {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_lot_config_host>() => {
            let cat = match (catalogue as *const SharedLotConfigCatalogue).as_ref() {
                Some(c) => c.clone(),
                None => LotConfigCatalogue::shared(),
            };
            let m = LotConfigManager::new(Rc::new(CHost { h: *h }), cat, token);
            Box::into_raw(Box::new(Handle { m })) as *mut fc4_lot_config_manager
        }
        _ => std::ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_destroy(m: *mut fc4_lot_config_manager) {
    if !m.is_null() {
        guard((), || drop(Box::from_raw(m as *mut Handle)));
    }
}

/// `QueryInterface` (§B1): `1`, `0xC8EDBEC0` and `cIGZSerializable`.
#[no_mangle]
pub extern "C" fn fc4_lot_config_query_interface(iid: u32) -> bool {
    query_interface(iid)
}

#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_add_ref(m: *mut fc4_lot_config_manager) -> u32 {
    with_m(m, 0, |h| h.m.add_ref())
}

/// `Release`: the last one runs `Shutdown` and marks the object destroyed
/// (0); the handle stays valid until [`fc4_lot_config_destroy`].
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_release(m: *mut fc4_lot_config_manager) -> u32 {
    with_m(m, 0, |h| h.m.release())
}

/// Slot 3 `Init` (§B4.1): builds the catalogue, or rebuilds it when the
/// detail preference changed (`StaticInit`).
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_init(m: *mut fc4_lot_config_manager) -> bool {
    with_m(m, false, |h| h.m.init())
}

/// Slot 4 `Shutdown` (§B4.2). The catalogue stays.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_shutdown(m: *mut fc4_lot_config_manager) -> bool {
    with_m(m, false, |h| h.m.shutdown())
}

/// The process unload callback (`0x0048A240`): releases every
/// configuration and empties the map; the family set stays (quirk 10).
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_unload(m: *mut fc4_lot_config_manager) {
    with_m(m, (), |h| h.m.unload())
}

/// `DoMessage` (§B4.3): PostCityInit fills every configuration's building
/// information. Always true.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_do_message(m: *mut fc4_lot_config_manager, msg_type: u32) -> bool {
    with_m(m, true, |h| h.m.do_message(msg_type))
}

/// `0x0048AD40` (§B4.3) on one configuration.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_fill_building_information(m: *mut fc4_lot_config_manager, cfg: u64) {
    with_m(m, (), |h| h.m.fill_building_information(cfg))
}

// ---- the add / remove table (§B3.2, §B3.4) ----

/// Slot 5 `AddLotConfiguration(key)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_add_by_key(
    m: *mut fc4_lot_config_manager,
    type_id: u32,
    group: u32,
    instance: u32,
) -> bool {
    let key = ResKey {
        type_id,
        group,
        instance,
    };
    with_m(m, false, |h| h.m.add_by_key(key))
}

/// Slot 6 `AddLotConfiguration(cfg, add)`. Always true.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_add_configuration(m: *mut fc4_lot_config_manager, cfg: u64, add: bool) -> bool {
    with_m(m, false, |h| h.m.add_configuration(cfg, add))
}

/// Slot 7 `RemoveLotConfiguration(id)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_remove_configuration(m: *mut fc4_lot_config_manager, id: u32) -> bool {
    with_m(m, false, |h| h.m.remove_configuration(id))
}

/// Slot 8 `GetLotConfiguration(id)`: no reference is added; 0 for none.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_configuration(m: *const fc4_lot_config_manager, id: u32) -> u64 {
    with_m(m, 0, |h| h.m.configuration(id).unwrap_or(0))
}

/// Slot 20: the map in bucket order (`ids` and `configurations` may be
/// NULL). Returns the number of entries; copies at most `capacity`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_configuration_map(
    m: *const fc4_lot_config_manager,
    ids: *mut u32,
    configurations: *mut u64,
    capacity: usize,
) -> usize {
    with_m(m, 0, |h| {
        let map = h.m.configuration_map();
        let (i, c): (Vec<u32>, Vec<u64>) = map.into_iter().unzip();
        copy_out(&i, ids, capacity);
        copy_out(&c, configurations, capacity)
    })
}

/// Slot 21: the family set in bucket order. As the map.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_family_id_set(
    m: *const fc4_lot_config_manager,
    out: *mut u32,
    capacity: usize,
) -> usize {
    with_m(m, 0, |h| copy_out(&h.m.family_id_set(), out, capacity))
}

// ---- set queries (§B5.2) ----

/// Slots 9–11 `GetLotConfigurationsByFilter / ByFamily / BySize`: `which`
/// 0 = by filter (`a` = the filter token, 0 = null), 1 = by family (`a` =
/// the family), 2 = by size (`a` = x, `b` = z). The matches go to `out` in
/// the map's bucket order, each with a new reference for the caller.
/// Returns the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_configurations(
    m: *mut fc4_lot_config_manager,
    which: u32,
    a: u64,
    b: u32,
    out: fc4_token_callback,
    ctx: *mut c_void,
) -> u32 {
    let v = with_m(m, Vec::new(), |h| match which {
        0 => h.m.configurations_by_filter(opt(a)),
        1 => h.m.configurations_by_family(a as u32),
        2 => h.m.configurations_by_size(a as u32, b),
        _ => Vec::new(),
    });
    emit(&v, out, ctx);
    v.len() as u32
}

/// Slot 12 `ClearLotConfigurationSet(set)`: releases every element once.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_clear_set(m: *mut fc4_lot_config_manager, set: *const u64, n: usize) {
    let s = slice_in(set, n).to_vec();
    with_m(m, (), |h| h.m.clear_set(&s))
}

/// Slots 13–15 `GetLotConfigurationIDsByFilter / ByFamily / BySize`, with
/// `which`, `a` and `b` as [`fc4_lot_config_configurations`] (the filter is
/// not cached). Returns the count; copies at most `capacity`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_ids(
    m: *const fc4_lot_config_manager,
    which: u32,
    a: u64,
    b: u32,
    out: *mut u32,
    capacity: usize,
) -> usize {
    with_m(m, 0, |h| {
        let v = match which {
            0 => h.m.ids_by_filter(opt(a)),
            1 => h.m.ids_by_family(a as u32),
            2 => h.m.ids_by_size(a as u32, b),
            _ => Vec::new(),
        };
        copy_out(&v, out, capacity)
    })
}

// ---- building tests (§B5.3) ----

/// Slots 16 / 17 `LotConfigurationHasBuildingFamily / Type(cfg, id, obj)`
/// (`by_type` false / true): the index of the matching object in the
/// configuration's array goes to `index` (may be NULL).
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_has_building(
    m: *const fc4_lot_config_manager,
    cfg: u64,
    id: u32,
    by_type: bool,
    index: *mut u32,
) -> bool {
    let r = with_m(m, None, |h| {
        if by_type {
            h.m.has_building_type(opt(cfg), id)
        } else {
            h.m.has_building_family(opt(cfg), id)
        }
    });
    match r {
        Some(i) => {
            if let Some(p) = index.as_mut() {
                *p = i as u32;
            }
            true
        }
        None => false,
    }
}

// ---- filters (§B5.4) ----

/// Slot 18.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_register_filter(m: *mut fc4_lot_config_manager, filter: u64) {
    with_m(m, (), |h| h.m.register_filter(filter))
}

/// Slot 19.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_unregister_filter(m: *mut fc4_lot_config_manager, filter: u64) {
    with_m(m, (), |h| h.m.unregister_filter(filter))
}

/// The flush (`0x0048A080`): drops every cached result.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_flush(m: *mut fc4_lot_config_manager) {
    with_m(m, (), |h| h.m.flush())
}

/// Diagnostics: a registered filter's cached result. False when the filter
/// is not registered; `*cached` false when there is no cached result. The
/// result's configurations (no references) are copied, at most `capacity`;
/// `*count` gets their number. Pointers may be NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_filter_cache(
    m: *const fc4_lot_config_manager,
    filter: u64,
    cached: *mut bool,
    out: *mut u64,
    capacity: usize,
    count: *mut usize,
) -> bool {
    let e = with_m(m, None, |h| h.m.snapshot().filters.get(&filter).cloned());
    let Some(e) = e else { return false };
    if let Some(p) = cached.as_mut() {
        *p = e.is_some();
    }
    let v = e.unwrap_or_default();
    let n = copy_out(&v, out, capacity);
    if let Some(p) = count.as_mut() {
        *p = n;
    }
    true
}

// ---- footprints and IDs (§B5.5, §B5.6) ----

/// Slots 24–26 `GetMaxFootprintFor{Building,Flora,Prop}Families(ids)`
/// (`kind` 0 building, 4 flora, 1 prop — the object types): the rectangle
/// `(w, d, 0, 0)` into `out4`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_max_footprint(
    m: *const fc4_lot_config_manager,
    kind: u32,
    ids: *const u32,
    n: usize,
    out4: *mut f32,
) -> bool {
    let ids = slice_in(ids, n).to_vec();
    let r = with_m(m, None, |h| match kind {
        0 => Some(h.m.footprint_buildings(&ids)),
        1 => Some(h.m.footprint_props(&ids)),
        4 => Some(h.m.footprint_flora(&ids)),
        _ => None,
    });
    match r {
        Some(r) => {
            if !out4.is_null() {
                std::ptr::copy_nonoverlapping(r.as_ptr(), out4, 4);
            }
            true
        }
        None => false,
    }
}

/// Slot 27 `GetObjectIDArray(obj, vec)`: the object's IDs with families
/// expanded (quirk 13). `obj` NULL gives an empty array. Returns the count;
/// copies at most `capacity`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_object_id_array(
    m: *const fc4_lot_config_manager,
    obj: *const fc4_lotdev_cfg_object,
    out: *mut u32,
    capacity: usize,
) -> usize {
    let o = obj.as_ref().map(|o| object_in(o));
    with_m(m, 0, |h| copy_out(&h.m.object_id_array(o.as_ref()), out, capacity))
}

// ---- editing helpers (§B5.7) ----

/// Slot 22 `CreateGenericLotConfiguration(buildingType)`: a configuration
/// not in the map with one reference for the caller, 0 when BDS rejects the
/// type (PL8).
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_create_generic(m: *mut fc4_lot_config_manager, building_type: u32) -> u64 {
    with_m(m, 0, |h| h.m.create_generic(building_type).unwrap_or(0))
}

/// Slot 23 `FixLotConfiguration(cfg)`: the code of the first problem found
/// (0 none, 1 an ID removed, 2 a record removed, 3 a footprint changed, 4
/// outside the lot, 5 a centre moved, 6 a retaining wall removed; PL9,
/// PL10).
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_fix(m: *mut fc4_lot_config_manager, cfg: u64) -> u32 {
    with_m(m, 0, |h| h.m.fix(opt(cfg)))
}

// ---- diagnostics and the record (§B6) ----

#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_get_state(
    m: *const fc4_lot_config_manager,
    out: *mut fc4_lot_config_state,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_m(m, false, |h| {
        let s = h.m.snapshot();
        *o = fc4_lot_config_state {
            struct_size: std::mem::size_of::<fc4_lot_config_state>() as u32,
            initialized: s.initialized.into(),
            destroyed: s.destroyed.into(),
            reserved0: 0,
            ref_count: s.ref_count,
            filters: s.filters.len() as u32,
            building_development: s.building_development.unwrap_or(0),
            flora: s.flora.unwrap_or(0),
            prop_manager: s.prop_manager.unwrap_or(0),
        };
        true
    })
}

/// `Write` (§B6): version 1 and the three held services.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_write(m: *const fc4_lot_config_manager, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_m(m, false, |h| h.m.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

/// Independent configuration Load projection for record-only shadows.
/// `out` must be writable; callback data is borrowed for the callback only.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_configuration_project(
    exemplar: *const fc4_props,
    detail: u32,
    out: *mut fc4_core::lot_config_manager::ConfigurationFields,
    visit: fc4_lotdev_cfg_object_callback,
    ctx: *mut c_void,
) -> bool {
    let (Some(h), Some(out)) = (props_ref(exemplar), out.as_mut()) else {
        return false;
    };
    guard(false, || {
        let Some(fields) = fc4_core::lot_config_manager::configuration_fields(h) else {
            return false;
        };
        *out = fields;
        // Version <2 generated serials are process-dependent; the live shadow
        // refuses those inputs rather than claiming independently equal tags.
        let objects = fc4_core::lot_config_manager::configuration_objects(h, detail, &mut 0);
        if let Some(f) = visit {
            for o in &objects {
                let c = object_out(o);
                f(ctx, &c);
            }
        }
        true
    })
}

/// Independent GLUE22 capacity projection, building-development §4.7.1.
/// No native profile values enter this derivation; absent exemplar means zero.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_configuration_capacity(exemplar: *const fc4_props) -> u16 {
    guard(0, || {
        props_ref(exemplar).map_or(0, |h| fc4_core::development::BuildingProfile::from_exemplar(h).capacity)
    })
}

/// Replacement configuration geometry. No RefCell: Load can overlap host reads.
#[allow(non_camel_case_types)]
pub struct fc4_lot_configuration {
    state: std::sync::Mutex<Option<fc4_core::lot_config_manager::LotConfiguration>>,
}

// SPEC-GAP: no clock seed is supplied by this ABI; legacy tags start at 1.
static LOT_CONFIGURATION_SERIAL: std::sync::Mutex<u32> = std::sync::Mutex::new(0);

#[no_mangle]
pub extern "C" fn fc4_lot_configuration_new() -> *mut fc4_lot_configuration {
    guard(std::ptr::null_mut(), || {
        Box::into_raw(Box::new(fc4_lot_configuration {
            state: std::sync::Mutex::new(None),
        }))
    })
}

/// Owner must exclude all calls before destroying the configuration.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_configuration_free(config: *mut fc4_lot_configuration) {
    if !config.is_null() {
        guard((), || drop(Box::from_raw(config)));
    }
}

/// Own an independent Load result. The exemplar is never retained.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_configuration_load(
    config: *const fc4_lot_configuration,
    exemplar: *const fc4_props,
    detail: u32,
) -> bool {
    let (Some(config), Some(h)) = (config.as_ref(), props_ref(exemplar)) else {
        return false;
    };
    guard(false, || {
        let mut serial = LOT_CONFIGURATION_SERIAL.lock().unwrap_or_else(|e| e.into_inner());
        let loaded = fc4_core::lot_config_manager::LotConfiguration::load(h, detail, &mut serial);
        let ok = loaded.is_some();
        *config.state.lock().unwrap_or_else(|e| e.into_inner()) = loaded;
        ok
    })
}

/// Copy the retained ABI view, including after the exemplar has been destroyed.
/// Callbacks execute outside the lock; their data is borrowed only for that call.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_configuration_snapshot(
    config: *const fc4_lot_configuration,
    out: *mut fc4_core::lot_config_manager::ConfigurationFields,
    visit: fc4_lotdev_cfg_object_callback,
    ctx: *mut c_void,
) -> bool {
    let (Some(config), Some(out)) = (config.as_ref(), out.as_mut()) else {
        return false;
    };
    guard(false, || {
        let loaded = config.state.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let Some(loaded) = loaded else { return false };
        *out = loaded.fields;
        if let Some(f) = visit {
            for object in &loaded.objects {
                f(ctx, &object_out(object));
            }
        }
        true
    })
}

/// `Read` (§B6): false for another version, and then nothing changes. The
/// references come back through `resolve_ref` with IIDs `0x07813EAE`,
/// `0x06C66177` and `0x28326F3E`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lot_config_read(m: *mut fc4_lot_config_manager, inp: *const fc4_istream) -> bool {
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
