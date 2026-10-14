// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The fire and police protection simulators' C ABI (fc4.h "protection",
//! docs/specs/protection.md).
//!
//! The host implements [`fc4_protection_host`] (every `ProtectionHost` method
//! of the core, plus the game's grids through [`fc4_grid_api`]); the plugin's
//! `cISC4FireProtectionSimulator` and `cISC4PoliceSimulator` (with their
//! message-target, dispatch-source and serializable sub-objects) forward their
//! slots to the `fc4_protection_*` functions. One handle is one simulator of
//! one kind (0 = fire, 1 = police).
//!
//! Tokens are opaque host identities (fc4.h): occupants (stations, jails,
//! automata) by their `cISC4Occupant` identity, departments, lots, grids, the
//! air dispatch source. 0 is the null object. The core owns one reference on
//! every token it keeps (stations, jails, departments, the air source); a
//! saved reference read back for one of the host's objects arrives with that
//! reference, and one that names a FreeCity4 object (`fc4_ref::core_handle`
//! set) is given one here (`add_ref`), so both cases end up the same.
//!
//! Re-entrancy: the core holds no borrow across a host callback. A message
//! delivered while a call is running is queued and handled when the outermost
//! call returns ([choice], as plumbing); other re-entrant calls run at once
//! and are counted (`fc4_protection_state::reentrant_calls`).
//!
//! Threads: the core is single-threaded (`RefCell`). The plugin serialises
//! every call on one object with its own lock (fc4.h).
//!
//! The grid shadow (`fc4_protection_shadow_*`) runs a handle that is never
//! initialised on a read-only host table (every write callback NULL) over the
//! ORIGINAL simulator's station list: the unadjusted grid of §5.1 rebuilt from
//! scratch in a core grid, and the adjusted grid of §2 from it (factor 1.0,
//! §16 quirk 2). The station and department tokens are borrowed for the call
//! (no reference taken or released).

use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_void};
use std::rc::Rc;

use fc4_core::grid::{GridProvider, SimGrid, TractGeometry, TractGrid};
use fc4_core::property::PropertyHolder;
use fc4_core::protection::{
    adjusted_value, query_interface, BudgetItem, CoverageOutputs, DispatchInfo, ObjRef, ProtectionHost,
    ProtectionInterface, ProtectionKind, ProtectionMessage, ProtectionRng, ProtectionSim,
};
use fc4_core::rng::FixedSeeds;
use fc4_core::stream::{InStream, RefValue, SaveObject};
use fc4_core::variant::{mem, Variant};

use crate::budget::fc4_token_callback;
use crate::guard;
use crate::hostgrid::{create_grid, fc4_grid_api, grid_token, read_property, resolve_grid, take_props};
use crate::landvalue::fc4_grid_geometry;
use crate::props::fc4_props;
use crate::stream::{fc4_istream, fc4_ostream, fc4_ref, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

/// `cISC4Occupant`: the identity stations and jails are read back with (§11).
const IID_OCCUPANT: u32 = 0x26BB_767E;

/// `GetBudgetItemForPurpose` (fc4.h `fc4_protection_budget_item`), with the
/// item's department looked up by its ID (§6.1).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_protection_budget_item {
    /// 1 when `GetDepartmentBudget(item.departmentID)` is not null.
    pub has_department: u32,
    /// `IsLocallyFundedObjectInDepartment(holder, purpose)` of that
    /// department (only meaningful with `has_department`).
    pub locally_funded: u32,
    /// `item.localFundingPercentage` (a fraction).
    pub local_funding: f32,
    pub reserved: u32,
    pub cost: i64,
    /// That department (borrowed token), 0 without one.
    pub department: u64,
}

/// `GetDispatchInfo` (fc4.h `fc4_protection_dispatch_info`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_protection_dispatch_info {
    /// The record's `+0x20`: the station the unit came from (borrowed
    /// token), whose dispatch effect draws the cone (§7.2, PR2).
    pub station: u64,
    /// Formerly the vehicle automaton; not read (0).
    pub reserved: u64,
    pub call_id: u32,
    pub active: u32,
    pub x: f32,
    pub z: f32,
}

/// A message for `fc4_protection_do_message` (fc4.h `fc4_protection_message`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_protection_message {
    pub msg_type: u32,
    pub data1: u32,
    pub data2: u32,
    pub reserved: u32,
    /// `GetVoid1()` as a `cISC4Occupant` (occupant messages), 0 otherwise.
    pub occupant: u64,
    /// `GetIGZUnknown()` (the crime message), 0 otherwise.
    pub unknown: u64,
}

/// Diagnostics (fc4.h `fc4_protection_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_protection_state {
    pub struct_size: u32,
    pub kind: u32,
    pub initialized: u8,
    pub loaded: u8,
    pub strike_flag: u8,
    pub reserved0: u8,
    pub stations: u32,
    pub tracts: u32,
    pub count_x: i32,
    pub count_z: i32,
    /// Tracts of the adjusted grid with a value > 0, the largest value and
    /// the sum of all values.
    pub covered_tracts: u32,
    pub max_coverage: u32,
    pub coverage_sum: u32,
    pub tracts_per_day: i32,
    pub zoned: i32,
    pub sum: i32,
    pub non_zero: i32,
    pub avg: f32,
    pub pct: f32,
    pub strike_date: u32,
    pub next_call: u32,
    pub jails: u32,
    pub funded_jails: u32,
    pub jailbreaks: u32,
    pub jail_capacity: i32,
    pub jail_population: i32,
    pub jailed: i32,
    pub arrests_last: i32,
    pub crimes_last: i32,
    pub reentrant_calls: u32,
    pub reserved1: u32,
    pub department: u64,
    pub jail_department: u64,
    pub adjusted_grid: u64,
    pub unadjusted_grid: u64,
    pub air_source: u64,
}

/// Everything the protection simulators ask of the game (fc4.h
/// `fc4_protection_host`; one callback per `ProtectionHost` method, in the
/// trait's order). `kind` arguments are 0 = fire, 1 = police.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_protection_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    pub grids: fc4_grid_api,
    // lifecycle
    pub acquire_services: Option<unsafe extern "C" fn(U, u32)>,
    pub release_services: Option<unsafe extern "C" fn(U, u32)>,
    /// `AddAgent(target, level, name, flags)` once per level, in order.
    pub add_agent: Option<unsafe extern "C" fn(U, *const c_char, usize, *const u32, usize, u32)>,
    pub remove_agent: Option<unsafe extern "C" fn(U)>,
    pub subscribe: Option<unsafe extern "C" fn(U, *const u32, usize)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, *const u32, usize)>,
    pub register_dispatch_source: Option<unsafe extern "C" fn(U)>,
    pub unregister_dispatch_source: Option<unsafe extern "C" fn(U)>,
    /// `SetInstanceID(instance)` on a grid token.
    pub set_grid_instance: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    // resources
    /// A NEW fc4_props (the core destroys it), NULL if the exemplar is missing.
    pub tuning: Option<unsafe extern "C" fn(U, u32) -> *mut fc4_props>,
    // budget
    /// (id, group, name group, name instance, max funding): the department
    /// with one reference for the core; 0 = none.
    pub create_department: Option<unsafe extern "C" fn(U, u32, u32, u32, u32, f32) -> u64>,
    pub department_id: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub department_funding: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub add_locally_funded: Option<unsafe extern "C" fn(U, u64, u64, u32)>,
    pub remove_locally_funded: Option<unsafe extern "C" fn(U, u64, u64, u32)>,
    pub set_local_funding: Option<unsafe extern "C" fn(U, u64, u64, f32, u32) -> bool>,
    pub budget_item: Option<unsafe extern "C" fn(U, u64, u32, *mut fc4_protection_budget_item) -> bool>,
    // occupants
    pub occupant_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub occupant_property: Option<unsafe extern "C" fn(U, u64, u32, *mut fc4_props) -> bool>,
    /// (occupant, id, element type `mem::*`, array, data, count).
    pub set_occupant_property: Option<unsafe extern "C" fn(U, u64, u32, u16, bool, *const c_void, usize)>,
    /// World x, y, z.
    pub occupant_position: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    pub is_prop_occupant: Option<unsafe extern "C" fn(U, u64) -> bool>,
    /// -1 = no lot, 0 = no road access, 1 = road access.
    pub lot_road_access: Option<unsafe extern "C" fn(U, u64) -> i32>,
    // city, simulator, zones, terrain, disasters
    pub sim_day: Option<unsafe extern "C" fn(U) -> u32>,
    pub lot_location: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32) -> bool>,
    pub cell_center_to_position: Option<unsafe extern "C" fn(U, i32, i32, *mut f32, *mut f32)>,
    pub zone_value: Option<unsafe extern "C" fn(U, i32, i32) -> u32>,
    pub sea_level: Option<unsafe extern "C" fn(U) -> f32>,
    pub altitude: Option<unsafe extern "C" fn(U, f32, f32) -> f32>,
    pub disaster_ongoing: Option<unsafe extern "C" fn(U) -> bool>,
    // dispatch manager
    pub dispatch_count: Option<unsafe extern "C" fn(U, u32) -> u32>,
    /// (type, buffer, capacity): the number of IDs written (≤ capacity).
    pub dispatch_ids: Option<unsafe extern "C" fn(U, u32, *mut u32, u32) -> u32>,
    pub dispatch_info: Option<unsafe extern "C" fn(U, u32, *mut fc4_protection_dispatch_info) -> bool>,
    // automata controllers
    pub add_controllers: Option<unsafe extern "C" fn(U, *const c_char, usize, u64)>,
    pub remove_controllers: Option<unsafe extern "C" fn(U, *const c_char, usize, u64)>,
    // other simulators, history, messages
    pub police_percent_coverage: Option<unsafe extern "C" fn(U, *mut f32) -> bool>,
    pub record_history: Option<unsafe extern "C" fn(U, u32, i32)>,
    /// (message type, subject token or 0).
    pub post: Option<unsafe extern "C" fn(U, u32, u64)>,
    // air dispatch sources
    /// (kind, class ID out): the source with one reference for the core; 0 =
    /// none.
    pub create_air_source: Option<unsafe extern "C" fn(U, u32, *mut u32) -> u64>,
    pub air_source_init: Option<unsafe extern "C" fn(U, u64)>,
    pub air_source_shutdown: Option<unsafe extern "C" fn(U, u64)>,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_protection_budget_item>() == 32);
    assert!(std::mem::size_of::<fc4_protection_dispatch_info>() == 32);
    assert!(std::mem::size_of::<fc4_protection_message>() == 32);
    assert!(std::mem::size_of::<fc4_protection_state>() == 152);
    assert!(std::mem::size_of::<fc4_protection_host>() == if cfg!(target_pointer_width = "32") { 220 } else { 440 });
};

fn kind_of(k: u32) -> Option<ProtectionKind> {
    match k {
        0 => Some(ProtectionKind::Fire),
        1 => Some(ProtectionKind::Police),
        _ => None,
    }
}

fn kind_code(k: ProtectionKind) -> u32 {
    match k {
        ProtectionKind::Fire => 0,
        ProtectionKind::Police => 1,
    }
}

// ---- host adapter -----------------------------------------------------------------------------------

struct CHost(fc4_protection_host);

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.0.$f { Some(f) => unsafe { f($s.0.user $(, $arg)*) }, None => $default }
    };
}

fn opt(t: u64) -> Option<u64> {
    (t != 0).then_some(t)
}

impl CHost {
    fn set_property(&self, occ: u64, id: u32, is_array: bool, values: &[u32]) {
        cb!(
            self,
            set_occupant_property,
            (),
            occ,
            id,
            mem::UINT32,
            is_array,
            values.as_ptr() as *const c_void,
            values.len()
        )
    }
}

impl GridProvider<u8> for CHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        create_grid::<u8>(&self.0.grids, instance, tract_size, true)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u8>>> {
        resolve_grid::<u8>(&self.0.grids, r, true)
    }
}

impl GridProvider<i16> for CHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<i16>>> {
        create_grid::<i16>(&self.0.grids, instance, tract_size, true)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<i16>>> {
        resolve_grid::<i16>(&self.0.grids, r, true)
    }
}

impl ProtectionHost for CHost {
    fn acquire_services(&self, kind: ProtectionKind) {
        cb!(self, acquire_services, (), kind_code(kind))
    }
    fn release_services(&self, kind: ProtectionKind) {
        cb!(self, release_services, (), kind_code(kind))
    }
    fn add_agent(&self, name: &str, levels: &[u32], flags: u32) {
        cb!(
            self,
            add_agent,
            (),
            name.as_ptr() as *const c_char,
            name.len(),
            levels.as_ptr(),
            levels.len(),
            flags
        )
    }
    fn remove_agent(&self) {
        cb!(self, remove_agent, ())
    }
    fn subscribe(&self, msg_types: &[u32]) {
        cb!(self, subscribe, (), msg_types.as_ptr(), msg_types.len())
    }
    fn unsubscribe(&self, msg_types: &[u32]) {
        cb!(self, unsubscribe, (), msg_types.as_ptr(), msg_types.len())
    }
    fn register_dispatch_source(&self) {
        cb!(self, register_dispatch_source, ())
    }
    fn unregister_dispatch_source(&self) {
        cb!(self, unregister_dispatch_source, ())
    }
    fn set_adjusted_instance(&self, grid: &Rc<dyn TractGrid<u8>>, instance: u32) {
        let t = grid_token(&Some(grid.clone()));
        if t != 0 {
            cb!(self, set_grid_instance, (), t, instance)
        }
    }
    fn set_unadjusted_instance(&self, grid: &Rc<dyn TractGrid<i16>>, instance: u32) {
        let t = grid_token(&Some(grid.clone()));
        if t != 0 {
            cb!(self, set_grid_instance, (), t, instance)
        }
    }
    // The host grid shuts the game grid down and releases it when the core
    // drops it (HostGrid, shutdown_on_drop), which is the §4.2 order.
    fn shutdown_adjusted(&self, _grid: &Rc<dyn TractGrid<u8>>) {}
    fn shutdown_unadjusted(&self, _grid: &Rc<dyn TractGrid<i16>>) {}
    fn add_ref(&self, token: u64) {
        if token != 0 {
            cb!(self, add_ref, (), token)
        }
    }
    fn release(&self, token: u64) {
        if token != 0 {
            cb!(self, release, (), token)
        }
    }
    fn tuning(&self, kind: ProtectionKind) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, tuning, std::ptr::null_mut(), kind_code(kind));
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }
    fn create_department(&self, id: u32, group: u32, name: (u32, u32), max_funding: f32) -> Option<ObjRef> {
        opt(cb!(self, create_department, 0, id, group, name.0, name.1, max_funding))
            .map(|token| ObjRef { token, class_id: 0 })
    }
    fn department_id(&self, dept: u64) -> u32 {
        cb!(self, department_id, 0, dept)
    }
    fn department_funding(&self, dept: u64) -> f32 {
        cb!(self, department_funding, 1.0, dept)
    }
    fn add_locally_funded(&self, dept: u64, occupant: u64, purpose: u32) {
        cb!(self, add_locally_funded, (), dept, occupant, purpose)
    }
    fn remove_locally_funded(&self, dept: u64, occupant: u64, purpose: u32) {
        cb!(self, remove_locally_funded, (), dept, occupant, purpose)
    }
    fn set_local_funding(&self, dept: u64, occupant: u64, p: f32, purpose: u32) -> bool {
        cb!(self, set_local_funding, false, dept, occupant, p, purpose)
    }
    fn budget_item(&self, occupant: u64, purpose: u32) -> Option<BudgetItem> {
        let mut i = fc4_protection_budget_item::default();
        cb!(self, budget_item, false, occupant, purpose, &mut i).then_some(BudgetItem {
            locally_funded: i.locally_funded != 0,
            local_funding: i.local_funding,
            cost: i.cost,
            department: if i.has_department != 0 {
                Some(i.department)
            } else {
                None
            },
        })
    }
    fn occupant_type(&self, occupant: u64) -> u32 {
        cb!(self, occupant_type, 0, occupant)
    }
    fn occupant_property(&self, occupant: u64, id: u32) -> Option<Variant> {
        let f = self.0.occupant_property?;
        // SAFETY: valid callback; the scratch holder outlives the call.
        read_property(id, |p| unsafe { f(self.0.user, occupant, id, p) })
    }
    // [choice] (§14 "property helpers"): the spec names the game's set helper
    // without saying whether a single u32 is stored as a scalar or as a
    // one-element vector; a scalar is the literal reading (spec question).
    fn set_occupant_u32(&self, occupant: u64, id: u32, value: u32) {
        self.set_property(occupant, id, false, &[value])
    }
    fn set_occupant_u32s(&self, occupant: u64, id: u32, values: &[u32]) {
        self.set_property(occupant, id, true, values)
    }
    fn occupant_position(&self, occupant: u64) -> [f32; 3] {
        let mut p = [0.0f32; 3];
        cb!(self, occupant_position, (), occupant, p.as_mut_ptr());
        p
    }
    fn is_prop_occupant(&self, occupant: u64) -> bool {
        cb!(self, is_prop_occupant, false, occupant)
    }
    fn lot_road_access(&self, occupant: u64) -> Option<bool> {
        match cb!(self, lot_road_access, -1, occupant) {
            r if r < 0 => None,
            r => Some(r != 0),
        }
    }
    fn sim_day(&self) -> u32 {
        cb!(self, sim_day, 0)
    }
    fn lot_location(&self, unknown: u64) -> Option<(i32, i32)> {
        let (mut x, mut z) = (0, 0);
        cb!(self, lot_location, false, unknown, &mut x, &mut z).then_some((x, z))
    }
    fn cell_center_to_position(&self, cell_x: i32, cell_z: i32) -> (f32, f32) {
        let (mut x, mut z) = (0.0f32, 0.0f32);
        cb!(self, cell_center_to_position, (), cell_x, cell_z, &mut x, &mut z);
        (x, z)
    }
    fn zone_value(&self, cell_x: i32, cell_z: i32) -> u32 {
        cb!(self, zone_value, 0, cell_x, cell_z)
    }
    fn sea_level(&self) -> f32 {
        cb!(self, sea_level, 0.0)
    }
    fn altitude(&self, x: f32, z: f32) -> f32 {
        cb!(self, altitude, 0.0, x, z)
    }
    fn disaster_ongoing(&self) -> bool {
        cb!(self, disaster_ongoing, false)
    }
    fn dispatch_count(&self, dispatch_type: u32) -> u32 {
        cb!(self, dispatch_count, 0, dispatch_type)
    }
    // The buffer is sized from GetDispatchCount: the host never sees a
    // capacity of 0 (which the game reads as "no limit", docs/abi).
    fn dispatch_ids(&self, dispatch_type: u32) -> Vec<u32> {
        let n = cb!(self, dispatch_count, 0, dispatch_type).min(1 << 16);
        if n == 0 {
            return Vec::new();
        }
        let mut v = vec![0u32; n as usize];
        let written = cb!(self, dispatch_ids, 0, dispatch_type, v.as_mut_ptr(), n);
        v.truncate(written.min(n) as usize);
        v
    }
    fn dispatch_info(&self, id: u32) -> Option<DispatchInfo> {
        let mut i = fc4_protection_dispatch_info::default();
        cb!(self, dispatch_info, false, id, &mut i).then_some(DispatchInfo {
            station: i.station,
            call_id: i.call_id,
            active: i.active != 0,
            x: i.x,
            z: i.z,
        })
    }
    fn add_controllers(&self, group: &str, occupant: u64) {
        cb!(
            self,
            add_controllers,
            (),
            group.as_ptr() as *const c_char,
            group.len(),
            occupant
        )
    }
    fn remove_controllers(&self, group: &str, occupant: u64) {
        cb!(
            self,
            remove_controllers,
            (),
            group.as_ptr() as *const c_char,
            group.len(),
            occupant
        )
    }
    fn police_percent_coverage(&self) -> Option<f32> {
        let mut v = 0.0f32;
        cb!(self, police_percent_coverage, false, &mut v).then_some(v)
    }
    fn record_history(&self, id: u32, value: i32) {
        cb!(self, record_history, (), id, value)
    }
    fn post(&self, msg_type: u32, subject: Option<u64>) {
        cb!(self, post, (), msg_type, subject.unwrap_or(0))
    }
    fn create_air_source(&self, kind: ProtectionKind) -> Option<ObjRef> {
        let mut class_id = 0u32;
        let token = opt(cb!(self, create_air_source, 0, kind_code(kind), &mut class_id))?;
        if class_id == 0 {
            class_id = kind.params().air_source_clsid;
        }
        Some(ObjRef { token, class_id })
    }
    fn air_source_init(&self, source: u64) {
        cb!(self, air_source_init, (), source)
    }
    fn air_source_shutdown(&self, source: u64) {
        cb!(self, air_source_shutdown, (), source)
    }
}

// ---- the save stream --------------------------------------------------------------------------------

/// The record reader: delegates to [`CIn`], except that references come back
/// as host tokens the core owns one reference on. Station and jail lists are
/// read with `cISC4Occupant` (the identity the occupant messages use).
struct PIn<'a> {
    inner: CIn<'a>,
    host: &'a CHost,
}

impl InStream for PIn<'_> {
    fn get_u8(&mut self) -> Option<u8> {
        self.inner.get_u8()
    }
    fn get_u16(&mut self) -> Option<u16> {
        self.inner.get_u16()
    }
    fn get_u32(&mut self) -> Option<u32> {
        self.inner.get_u32()
    }
    fn get_u64(&mut self) -> Option<u64> {
        self.inner.get_u64()
    }
    fn get_f32(&mut self) -> Option<f32> {
        self.inner.get_f32()
    }
    fn get_f64(&mut self) -> Option<f64> {
        self.inner.get_f64()
    }
    fn get_bytes(&mut self, n: usize) -> Option<Vec<u8>> {
        self.inner.get_bytes(n)
    }
    fn get_str(&mut self) -> Option<Vec<u8>> {
        self.inner.get_str()
    }
    fn get_ref(&mut self) -> Option<RefValue> {
        self.get_ref_as(IID_OCCUPANT)
    }
    fn get_ref_as(&mut self, iid: u32) -> Option<RefValue> {
        let f = self.inner.stream.get_ref?;
        let mut out = fc4_ref::default();
        // SAFETY: valid callback; `out` outlives the call.
        if !unsafe { f(self.inner.stream.user, iid, &mut out) } {
            return None;
        }
        if out.token == 0 {
            return Some(RefValue::Null);
        }
        // One of FreeCity4's own objects comes without a reference (fc4.h
        // fc4_ref): take the one the core will own.
        if !out.core_handle.is_null() {
            self.host.add_ref(out.token);
        }
        Some(RefValue::Host {
            token: out.token,
            class_id: out.class_id,
        })
    }
    fn skip(&mut self, n: usize) -> bool {
        self.inner.skip(n)
    }
    fn error(&self) -> i32 {
        self.inner.error()
    }
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

// ---- handles ----------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_protection`).
#[repr(C)]
pub struct fc4_protection {
    _private: [u8; 0],
}

struct Handle {
    host: Rc<CHost>,
    sim: Rc<ProtectionSim>,
    depth: Cell<u32>,
    queue: RefCell<Vec<ProtectionMessage>>,
    reentrant: Cell<u32>,
    /// A grid-shadow handle: its station and department tokens are borrowed.
    shadow: bool,
}

impl Handle {
    fn new(host: Rc<CHost>, kind: ProtectionKind, token: u64, seeds: FixedSeeds, shadow: bool) -> Handle {
        let rng = ProtectionRng::new(&seeds);
        let sim = ProtectionSim::new(host.clone(), kind, token, Box::new(rng));
        Handle {
            host,
            sim,
            depth: Cell::new(0),
            queue: RefCell::new(Vec::new()),
            reentrant: Cell::new(0),
            shadow,
        }
    }

    /// Runs `f` with the call depth counted; at the end of the outermost
    /// call, the queued messages are handled.
    fn call<R>(&self, f: impl FnOnce(&ProtectionSim) -> R) -> R {
        if self.depth.get() > 0 {
            self.reentrant.set(self.reentrant.get().wrapping_add(1));
        }
        self.depth.set(self.depth.get() + 1);
        let r = f(&self.sim);
        if self.depth.get() == 1 {
            loop {
                let next = {
                    let mut q = self.queue.borrow_mut();
                    if q.is_empty() {
                        None
                    } else {
                        Some(q.remove(0))
                    }
                };
                let Some(m) = next else { break };
                self.sim.handle_message(m);
            }
        }
        self.depth.set(self.depth.get() - 1);
        r
    }
}

/// Runs `f` on the handle; the fallback for NULL or a panic.
fn with_handle<R>(p: *const fc4_protection, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_protection*.
    let Some(h) = (unsafe { (p as *const Handle).as_ref() }) else {
        return fallback;
    };
    guard(fallback, || f(h))
}

fn sim<R>(p: *const fc4_protection, fallback: R, f: impl FnOnce(&ProtectionSim) -> R) -> R {
    with_handle(p, fallback, |h| h.call(f))
}

fn host_ok(host: *const fc4_protection_host) -> Option<fc4_protection_host> {
    // SAFETY: documented precondition: NULL or a valid host table.
    let h = unsafe { host.as_ref() }?;
    ((h.struct_size as usize) >= std::mem::size_of::<fc4_protection_host>()).then_some(*h)
}

/// Calls `cb(ctx, token)` for each object; the number of objects.
unsafe fn emit(objs: &[ObjRef], cb: fc4_token_callback, ctx: *mut c_void) -> usize {
    if let Some(f) = cb {
        for o in objs {
            f(ctx, o.token);
        }
    }
    objs.len()
}

/// Which sub-object `QueryInterface(iid)` answers with (§1): 0 = none (AL =
/// 0), 1 = the primary interface (also `cIGZUnknown`), 2 =
/// `cIGZMessageTarget2`, 3 = the dispatch source, 4 = `cIGZSerializable`.
/// Any `kind` other than 0 / 1 answers 0.
#[no_mangle]
pub extern "C" fn fc4_protection_query_interface(kind: u32, iid: u32) -> u32 {
    let Some(k) = kind_of(kind) else { return 0 };
    match query_interface(k, iid) {
        None => 0,
        Some(ProtectionInterface::Primary) => 1,
        Some(ProtectionInterface::MessageTarget) => 2,
        Some(ProtectionInterface::DispatchSource) => 3,
        Some(ProtectionInterface::Serializable) => 4,
    }
}

/// Creates the core object (constructor values, §3). The host table is
/// copied. `kind` 0 = fire, 1 = police; `token` is the host identity of the
/// simulator itself. The object's own `cRZRandom` is seeded from the clock
/// (§10): `wall_clock_seed` and `process_clock` are those clock values.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_create(
    host: *const fc4_protection_host,
    kind: u32,
    token: u64,
    wall_clock_seed: u32,
    process_clock: u32,
) -> *mut fc4_protection {
    guard(std::ptr::null_mut(), || {
        let (Some(h), Some(k)) = (host_ok(host), kind_of(kind)) else {
            return std::ptr::null_mut();
        };
        if !h.grids.complete() {
            return std::ptr::null_mut();
        }
        let seeds = FixedSeeds {
            wall_clock: wall_clock_seed,
            process_clock,
        };
        Box::into_raw(Box::new(Handle::new(Rc::new(CHost(h)), k, token, seeds, false))) as *mut fc4_protection
    })
}

/// Destroys the core object. The references it still holds (a loaded object
/// that was never initialised) are released; a shadow handle releases
/// nothing.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_destroy(p: *mut fc4_protection) {
    if p.is_null() {
        return;
    }
    guard((), || {
        let b = Box::from_raw(p as *mut Handle);
        let held: Vec<ObjRef> = {
            let mut d = b.sim.data_mut();
            let d = &mut *d;
            let mut held = Vec::new();
            held.extend(d.department.take());
            held.append(&mut d.stations);
            held.extend(d.air_source.take());
            held.extend(d.jail.jail_department.take());
            held.append(&mut d.jail.funded_jails);
            held.append(&mut d.jail.jails);
            held.append(&mut d.jail.jailbreaks);
            held
        };
        // An initialised object released everything at Shutdown (the plugin
        // shuts it down first), so `held` is empty then.
        if !b.shadow {
            for o in &held {
                b.host.release(o.token);
            }
        }
        b.sim.set_grids(None, None);
        drop(b);
    });
}

/// `Init` (§4.1): always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_init(p: *mut fc4_protection) -> bool {
    sim(p, false, ProtectionSim::init)
}

/// `Shutdown` (§4.2): always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_shutdown(p: *mut fc4_protection) -> bool {
    sim(p, false, ProtectionSim::shutdown)
}

/// `DoMessage` (§5): always true. A message delivered during another call is
/// queued (the host keeps its tokens alive until the outermost call returns).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_do_message(p: *mut fc4_protection, m: *const fc4_protection_message) -> bool {
    let Some(m) = m.as_ref() else { return true };
    let msg = ProtectionMessage {
        msg_type: m.msg_type,
        data1: m.data1,
        data2: m.data2,
        occupant: opt(m.occupant).map(|token| ObjRef { token, class_id: 0 }),
        unknown: opt(m.unknown),
    };
    with_handle(p, true, |h| {
        if h.depth.get() > 0 {
            h.queue.borrow_mut().push(msg);
            return true;
        }
        h.call(|s| s.handle_message(msg))
    })
}

// ---- the primary interfaces (§15.1, §15.2) ---------------------------------------------------------

/// Slot 5: the adjusted grid's token (borrowed; 0 = none).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_grid(p: *const fc4_protection) -> u64 {
    sim(p, 0, |s| grid_token(&s.coverage_grid()))
}

/// Slot 6 `GetDepartmentBudget` (borrowed; 0 = none).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_department(p: *const fc4_protection) -> u64 {
    sim(p, 0, |s| s.department().map_or(0, |o| o.token))
}

/// Police slot 9 `GetJailDepartmentBudget` (borrowed; 0 = none).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_jail_department(p: *const fc4_protection) -> u64 {
    sim(p, 0, |s| s.jail_department().map_or(0, |o| o.token))
}

/// Fire slot 7 `IsFireStation` / police slot 22 `IsPoliceStation`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_is_station(p: *const fc4_protection, occ: u64) -> bool {
    sim(p, false, |s| s.is_station(occ))
}

/// Fire slot 8 / police slot 7 `GetStationFunding(occ, int32*, pct*)` (§6.6):
/// each output written only when its pointer is not NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_station_funding(
    p: *const fc4_protection,
    occ: u64,
    cost: *mut i32,
    pct: *mut f32,
) -> bool {
    let Some((c, f)) = sim(p, None, |s| s.station_funding(occ)) else {
        return false;
    };
    if let Some(o) = cost.as_mut() {
        *o = c;
    }
    if let Some(o) = pct.as_mut() {
        *o = f;
    }
    true
}

/// Fire slot 9 / police slot 8 `SetStationFunding(occ, pct)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_set_station_funding(p: *mut fc4_protection, occ: u64, pct: f32) -> bool {
    sim(p, false, |s| s.set_station_funding(occ, pct))
}

/// Message-target slot 11 `ProtectionGetStationFunding(occ, float*)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_protection_station_funding(
    p: *const fc4_protection,
    occ: u64,
    out: *mut f32,
) -> bool {
    let Some(v) = sim(p, None, |s| s.protection_station_funding(occ)) else {
        return false;
    };
    if let Some(o) = out.as_mut() {
        *o = v;
    }
    true
}

/// Fire slot 10 / police slot 25 `GetStationEfficiency(occ)` (a fraction).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_station_efficiency(p: *const fc4_protection, occ: u64) -> f32 {
    sim(p, 0.0, |s| s.station_efficiency(occ))
}

/// Fire slot 11 / police slot 10 `IsOnStrike`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_is_on_strike(p: *const fc4_protection) -> bool {
    sim(p, false, ProtectionSim::is_on_strike)
}

/// Fire slot 12 / police slot 11 `GetChanceOfStrike` (a fraction).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_chance_of_strike(p: *const fc4_protection) -> f32 {
    sim(p, 0.0, ProtectionSim::chance_of_strike)
}

/// Fire slot 13 / police slot 12 `BeginStrike(occ)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_begin_strike(p: *mut fc4_protection, occ: u64) -> bool {
    sim(p, false, |s| s.begin_strike(occ))
}

/// Fire slot 14 / police slot 13 `EndStrike`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_end_strike(p: *mut fc4_protection) -> bool {
    sim(p, false, ProtectionSim::end_strike)
}

/// Fire slot 15 / police slot 14 `GetStrikeStartDate`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_strike_start_date(p: *const fc4_protection) -> u32 {
    sim(p, 0, ProtectionSim::strike_start_date)
}

/// Fire slot 16 / police slot 15 `GetStrikeLength` (`0xFFFFFFFF` without a
/// date).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_strike_length(p: *const fc4_protection) -> u32 {
    sim(p, u32::MAX, ProtectionSim::strike_length)
}

/// Fire slot 17 / police slot 16 `GetStrikingStations`: `cb` once per
/// striking station (borrowed tokens; the caller AddRefs each one it appends,
/// and does not clear its list). Returns the number appended.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_striking_stations(
    p: *const fc4_protection,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) -> i32 {
    let mut v = Vec::new();
    let n = sim(p, 0, |s| s.striking_stations(&mut v));
    emit(&v, cb, ctx);
    n
}

/// Fire slot 18 / police slot 17 `IsStationOnStrike(occ)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_is_station_on_strike(p: *const fc4_protection, occ: u64) -> bool {
    sim(p, false, |s| s.is_station_on_strike(occ))
}

/// Fire slot 19 / police slot 18 `GetAverageCoverage`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_average_coverage(p: *const fc4_protection) -> f32 {
    sim(p, 0.0, ProtectionSim::average_coverage)
}

/// Fire slot 20 / police slot 19 `GetPercentCoverage`: the value behind the
/// returned pointer (a fraction).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_percent_coverage(p: *const fc4_protection) -> f32 {
    sim(p, 0.0, ProtectionSim::percent_coverage)
}

/// Fire slot 21 / police slot 20 `GetOccupantCoverage(occ, p, cover&,
/// station&)` (§7.3): fire zeroes both outputs first; police leaves them as
/// they are when the station effect fails. NULL outputs are scratch.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_occupant_coverage(
    p: *const fc4_protection,
    occ: u64,
    pct: f32,
    cover_radius: *mut f32,
    station_radius: *mut f32,
) -> bool {
    let mut out = CoverageOutputs {
        cover_radius: cover_radius.as_ref().copied().unwrap_or(0.0),
        station_radius: station_radius.as_ref().copied().unwrap_or(0.0),
    };
    let ok = sim(p, false, |s| s.occupant_coverage(occ, pct, &mut out));
    if let Some(o) = cover_radius.as_mut() {
        *o = out.cover_radius;
    }
    if let Some(o) = station_radius.as_mut() {
        *o = out.station_radius;
    }
    ok
}

/// Fire slot 22 / police slot 21 `GetFireStations` / `GetPoliceStations` and
/// dispatch-source slot 6 `GetStations`: `cb` once per station in list order
/// (borrowed tokens; the caller clears its list first and AddRefs each one).
/// Returns the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_stations(
    p: *const fc4_protection,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) -> u32 {
    let v = sim(p, Vec::new(), ProtectionSim::stations);
    emit(&v, cb, ctx) as u32
}

/// Fire slot 23 / police slot 24 the station count.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_station_count(p: *const fc4_protection) -> u32 {
    sim(p, 0, ProtectionSim::station_count)
}

/// Fire slot 24 / police slot 23 `GetStationDispatchRadius(occ)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_station_dispatch_radius(p: *const fc4_protection, occ: u64) -> f32 {
    sim(p, 0.0, |s| s.station_dispatch_radius(occ))
}

/// Fire slot 25 / police slot 26 `GetTotalDispatchCount`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_total_dispatch_count(p: *const fc4_protection) -> i32 {
    sim(p, 0, ProtectionSim::total_dispatch_count)
}

// ---- police jails (§9, §15.2) -------------------------------------------------------------------------

/// Police slot 27 `ResetUnjailedCriminals` (§9.3).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_reset_unjailed_criminals(p: *mut fc4_protection) -> i32 {
    sim(p, 0, ProtectionSim::reset_unjailed_criminals)
}

/// Police slot 28 `GetJailCapacity(occ)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_jail_capacity_of(p: *const fc4_protection, occ: u64) -> u32 {
    sim(p, 0, |s| s.jail_capacity_of(occ))
}

/// Police slot 29 `GetJailCapacity()` (total).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_jail_capacity(p: *const fc4_protection) -> u32 {
    sim(p, 0, ProtectionSim::jail_capacity)
}

/// Police slot 30 `GetJailPopulation`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_jail_population(p: *const fc4_protection) -> u32 {
    sim(p, 0, ProtectionSim::jail_population)
}

/// Police slot 31 `GetCriminalCount` (jailed).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_criminal_count(p: *const fc4_protection) -> u32 {
    sim(p, 0, ProtectionSim::criminal_count)
}

/// Police slot 32 `GetJails`: as [`fc4_protection_stations`] over the jail
/// list.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_jails(
    p: *const fc4_protection,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) -> u32 {
    let v = sim(p, Vec::new(), ProtectionSim::jails);
    emit(&v, cb, ctx) as u32
}

/// Police slot 33 `IsJail(occ)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_is_jail(p: *const fc4_protection, occ: u64) -> bool {
    sim(p, false, |s| s.is_jail(occ))
}

/// Police slot 34 `GetArrestCount` (last month).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_arrest_count(p: *const fc4_protection) -> u32 {
    sim(p, 0, ProtectionSim::arrest_count)
}

/// Police slot 35 `GetJailCount`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_jail_count(p: *const fc4_protection) -> u32 {
    sim(p, 0, ProtectionSim::jail_count)
}

/// Police slot 36 `GetJailedPercentage` (a fraction; NaN when jailed and
/// capacity are both 0, §16 quirk 10).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_jailed_percentage(p: *const fc4_protection) -> f32 {
    sim(p, 0.0, ProtectionSim::jailed_percentage)
}

/// Police slot 37 `GetJailCurrentUpkeepCost(jail)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_jail_upkeep_cost(p: *const fc4_protection, occ: u64) -> i64 {
    sim(p, 0, |s| s.jail_upkeep_cost(occ))
}

/// Police slot 38 `ProcessMonthlyCrimeStats` (true, [choice] of §15.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_process_monthly_crime_stats(p: *mut fc4_protection) -> bool {
    sim(p, false, ProtectionSim::process_monthly_crime_stats)
}

/// Police slot 39 `IsInJailbreak(jail)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_is_in_jailbreak(p: *const fc4_protection, occ: u64) -> bool {
    sim(p, false, |s| s.is_in_jailbreak(occ))
}

/// Police slot 40 `BeginJailbreak(jail)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_begin_jailbreak(p: *mut fc4_protection, occ: u64) -> bool {
    sim(p, false, |s| {
        s.begin_jailbreak(ObjRef {
            token: occ,
            class_id: 0,
        })
    })
}

/// Police slot 41 `EndJailbreak(jail)` (AL = 0 always, §15.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_end_jailbreak(p: *mut fc4_protection, occ: u64) -> bool {
    sim(p, false, |s| s.end_jailbreak(occ))
}

/// Police slot 42 `GetJailbreakBuildings`: 0 and no callback when the
/// jailbreak list is empty (the caller leaves its list as is, §16 quirk 7);
/// otherwise the caller clears its list and `cb` runs once per jail.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_jailbreak_buildings(
    p: *const fc4_protection,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) -> i32 {
    let mut v = Vec::new();
    let n = sim(p, 0, |s| s.jailbreak_buildings(&mut v));
    emit(&v, cb, ctx);
    n
}

/// Police slot 43 `GetPoliceStationArrestCount(occ)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_station_arrest_count(p: *const fc4_protection, occ: u64) -> u32 {
    sim(p, 0, |s| s.station_arrest_count(occ))
}

/// Police slot 44 `GetPoliceStationCrimeCount(occ)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_station_crime_count(p: *const fc4_protection, occ: u64) -> u32 {
    sim(p, 0, |s| s.station_crime_count(occ))
}

// ---- the dispatch source (§8, §15.4) -------------------------------------------------------------------

/// Dispatch-source slot 3 `CanHandleDispatchType(t)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_can_handle_dispatch_type(p: *const fc4_protection, t: u32) -> bool {
    sim(p, false, |s| s.can_handle_dispatch_type(t))
}

/// Dispatch-source slot 4 `CanDispatch(x, z, bool* allBusy)`: `*all_busy`
/// written (when not NULL) only when it returns true.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_can_dispatch(
    p: *const fc4_protection,
    x: f32,
    z: f32,
    all_busy: *mut bool,
) -> bool {
    let Some(b) = sim(p, None, |s| s.can_dispatch(x, z)) else {
        return false;
    };
    if let Some(o) = all_busy.as_mut() {
        *o = b;
    }
    true
}

/// Dispatch-source slot 5 `GetMaxPossibleDispatchCount` (§7.3, PR1).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_max_possible_dispatch_count(p: *const fc4_protection) -> i32 {
    sim(p, 0, ProtectionSim::max_possible_dispatch_count)
}

/// Dispatch-source slot 7 `GetDispatchesAtStation(occ, flags*, base*,
/// count*)` (§8.1): a NULL `flags` / `base` is not requested; each requested
/// output is written as soon as its property is read. A NULL `count` is
/// scratch.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_dispatches_at_station(
    p: *const fc4_protection,
    occ: u64,
    flags: *mut u32,
    base: *mut u32,
    count: *mut u32,
) -> bool {
    let mut scratch = 0u32;
    let count = match count.as_mut() {
        Some(c) => c,
        None => &mut scratch,
    };
    let (flags, base) = (flags.as_mut(), base.as_mut());
    sim(p, false, |s| s.dispatches_at_station_into(occ, flags, base, count))
}

/// Dispatch-source slot 8 `DispatchFromNearestStation(x, z, u32* id)`: the
/// station (borrowed; 0 = none) and `*id` (0 unless a unit was dispatched).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_dispatch_from_nearest_station(
    p: *mut fc4_protection,
    x: f32,
    z: f32,
    id: *mut u32,
) -> u64 {
    let r = sim(p, None, |s| s.dispatch_from_nearest_station(x, z));
    if let Some(o) = id.as_mut() {
        *o = r.map_or(0, |(_, call)| call);
    }
    r.map_or(0, |(s, _)| s.token)
}

/// Dispatch-source slot 9 `DispatchFromStation(occ, u32* id)` (§8.2): `*id`
/// in (0 = any free unit) and out (the call ID).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_dispatch_from_station(p: *mut fc4_protection, occ: u64, id: *mut u32) -> bool {
    let want = id.as_ref().copied().unwrap_or(0);
    let Some(call) = sim(p, None, |s| s.dispatch_from_station(occ, want)) else {
        return false;
    };
    if let Some(o) = id.as_mut() {
        *o = call;
    }
    true
}

/// Dispatch-source slot 10 `RevokeDispatchFromStation(occ, id)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_revoke_dispatch_from_station(
    p: *mut fc4_protection,
    occ: u64,
    id: u32,
) -> bool {
    sim(p, false, |s| s.revoke_dispatch_from_station(occ, id))
}

/// Dispatch-source slot 11 `GetDispatchAutomataProperties` (§8.2): `*b`,
/// `c[0..3]` and `*d` (each when not NULL); always true.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_dispatch_automata_properties(
    p: *const fc4_protection,
    b: *mut u32,
    c: *mut f32,
    d: *mut u32,
) -> bool {
    let Some((vb, vc, vd)) = sim(p, None, |s| Some(s.dispatch_automata_properties())) else {
        return false;
    };
    if let Some(o) = b.as_mut() {
        *o = vb;
    }
    if !c.is_null() {
        std::ptr::copy_nonoverlapping(vc.as_ptr(), c, 3);
    }
    if let Some(o) = d.as_mut() {
        *o = vd;
    }
    true
}

// ---- diagnostics and persistence -----------------------------------------------------------------------

/// Diagnostics. Reads the adjusted grid (one host call per tract).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_get_state(p: *const fc4_protection, out: *mut fc4_protection_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_handle(p, false, |h| {
        let s = &h.sim;
        let adj = s.coverage_grid();
        let unadj = s.unadjusted_grid();
        let (mut covered, mut max, mut total, mut cx, mut cz) = (0u32, 0u32, 0u32, 0, 0);
        if let Some(g) = &adj {
            let geom = g.geometry();
            cx = geom.count_x.max(0);
            cz = geom.count_z.max(0);
            for x in 0..cx {
                for z in 0..cz {
                    let v = u32::from(g.get(x, z));
                    if v > 0 {
                        covered += 1;
                    }
                    max = max.max(v);
                    total = total.wrapping_add(v);
                }
            }
        }
        let d = s.data();
        *o = fc4_protection_state {
            struct_size: std::mem::size_of::<fc4_protection_state>() as u32,
            kind: kind_code(s.kind()),
            initialized: u8::from(d.initialized),
            loaded: u8::from(d.loaded),
            strike_flag: u8::from(d.strike_flag),
            reserved0: 0,
            stations: d.stations.len() as u32,
            tracts: (cx as u32).wrapping_mul(cz as u32),
            count_x: cx,
            count_z: cz,
            covered_tracts: covered,
            max_coverage: max,
            coverage_sum: total,
            tracts_per_day: d.tracts_per_day,
            zoned: d.counters.zoned,
            sum: d.counters.sum,
            non_zero: d.counters.non_zero,
            avg: d.avg,
            pct: d.pct,
            strike_date: d.strike_date,
            next_call: d.next_call,
            jails: d.jail.jails.len() as u32,
            funded_jails: d.jail.funded_jails.len() as u32,
            jailbreaks: d.jail.jailbreaks.len() as u32,
            jail_capacity: d.jail.capacity,
            jail_population: d.jail.population,
            jailed: d.jail.jailed,
            arrests_last: d.jail.arrests_last,
            crimes_last: d.jail.crimes_last,
            reentrant_calls: h.reentrant.get(),
            reserved1: 0,
            department: d.department.map_or(0, |o| o.token),
            jail_department: d.jail.jail_department.map_or(0, |o| o.token),
            adjusted_grid: grid_token(&adj),
            unadjusted_grid: grid_token(&unadj),
            air_source: d.air_source.map_or(0, |o| o.token),
        };
        true
    })
}

/// `Write` (§11): the fire (v3) or police (v8) record. References go out as
/// `fc4_ref`s with the token and the class ID.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_write(p: *const fc4_protection, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    sim(p, false, |s| s.write(&mut COut(o)))
}

/// `Read` (§11). Runs before `Init`: nothing is sized from the city here.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_read(p: *mut fc4_protection, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_handle(p, false, |h| {
        let host = h.host.clone();
        h.call(|s| {
            s.read(&mut PIn {
                inner: CIn {
                    stream: i,
                    decode: no_core_objects,
                },
                host: &host,
            })
        })
    })
}

// ---- the grid shadow -------------------------------------------------------------------------------------

/// A grid-shadow handle for `kind` over a read-only host table (the grid API
/// is not used and may be empty; write callbacks should be NULL). Never
/// initialised. Destroy it with [`fc4_protection_destroy`].
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_shadow_create(
    host: *const fc4_protection_host,
    kind: u32,
) -> *mut fc4_protection {
    guard(std::ptr::null_mut(), || {
        let (Some(h), Some(k)) = (host_ok(host), kind_of(kind)) else {
            return std::ptr::null_mut();
        };
        let seeds = FixedSeeds {
            wall_clock: 0,
            process_clock: 0,
        };
        Box::into_raw(Box::new(Handle::new(Rc::new(CHost(h)), k, 0, seeds, true))) as *mut fc4_protection
    })
}

/// The largest grid the shadow builds (tracts per side).
const SHADOW_MAX_SIDE: i32 = 4096;

/// Runs the shadow: reads the tuning (§6.7), takes the original's stations
/// (in its list order), department and city strike flag (borrowed tokens, for
/// this and the following queries on the handle), rebuilds the unadjusted
/// grid from scratch in a core grid of geometry `geom` (§5.1) and derives the
/// adjusted grid (§2, factor 1.0). False for a non-shadow handle or a
/// geometry outside 1..=4096 tracts per side.
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_shadow_run(
    p: *mut fc4_protection,
    geom: *const fc4_grid_geometry,
    stations: *const u64,
    count: usize,
    department: u64,
    strike_flag: bool,
) -> bool {
    let Some(g) = geom.as_ref() else { return false };
    let side = 1..=SHADOW_MAX_SIDE;
    if !side.contains(&g.count_x) || !side.contains(&g.count_z) {
        return false;
    }
    let tokens: Vec<u64> = if stations.is_null() || count == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(stations, count).to_vec()
    };
    let geom = TractGeometry {
        tract_size: g.tract_size,
        shift: g.shift,
        count_x: g.count_x,
        count_z: g.count_z,
        width_x: g.width_x,
        width_z: g.width_z,
        inv_width_x: g.inv_width_x,
        inv_width_z: g.inv_width_z,
    };
    with_handle(p, false, |h| {
        if !h.shadow {
            return false;
        }
        let adj = SimGrid::<u8>::from_values(geom, &[]);
        let unadj = SimGrid::<i16>::from_values(geom, &[]);
        h.call(|s| {
            s.read_tuning();
            s.set_grids(Some(adj.clone()), Some(unadj.clone()));
            {
                let mut d = s.data_mut();
                d.stations = tokens
                    .iter()
                    .filter(|&&t| t != 0)
                    .map(|&token| ObjRef { token, class_id: 0 })
                    .collect();
                d.department = opt(department).map(|token| ObjRef { token, class_id: 0 });
                d.strike_flag = strike_flag;
            }
            s.update_unadjusted();
            for x in 0..geom.count_x {
                for z in 0..geom.count_z {
                    adj.set(x, z, adjusted_value(unadj.get(x, z), 1.0));
                }
            }
        });
        true
    })
}

/// Copies the shadow's adjusted grid (x outer: index `x * count_z + z`) into
/// `out` (at most `n` values); the number of tracts (0 before a run).
#[no_mangle]
pub unsafe extern "C" fn fc4_protection_shadow_grid(p: *const fc4_protection, out: *mut u8, n: usize) -> usize {
    let values = with_handle(p, Vec::new(), |h| {
        let Some(g) = h.sim.coverage_grid() else {
            return Vec::new();
        };
        let geom = g.geometry();
        let mut v = Vec::with_capacity(geom.len());
        for x in 0..geom.count_x {
            for z in 0..geom.count_z {
                v.push(g.get(x, z));
            }
        }
        v
    });
    if !out.is_null() {
        std::ptr::copy_nonoverlapping(values.as_ptr(), out, values.len().min(n));
    }
    values.len()
}
