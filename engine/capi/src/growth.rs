// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The growth developer's C ABI (fc4.h "growth developer",
//! docs/specs/growth-developer.md).
//!
//! The host implements [`fc4_growth_host`] (every `GrowthDeveloperHost`
//! method of the core, plus the game's grids through [`fc4_grid_api`]); the
//! plugin's `cISC4GrowthDeveloper` (with its message-target and serializable
//! sub-objects) forwards its slots to the `fc4_growth_*` functions. One handle
//! is one developer. The process-wide state every developer shares (the
//! file-local generator of §7 and the time-slice step of §5.2) is one
//! [`fc4_growth_statics`] the host creates once per process.
//!
//! The developer's six grids are game grids (`fc4_grid_api`); the source
//! maps (§4.3) are game grids the host hands over with one reference
//! (`source_map`), released when the core is done with them. Tokens are
//! opaque host identities; the core owns one reference on every special
//! building and source demand node token it keeps.
//!
//! Re-entrancy and threads: as the BDS (fc4.h "development"): a message
//! delivered while a call is running is queued until the outermost call
//! returns; the plugin serialises every call with the development lock.

use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_void};
use std::rc::Rc;

use fc4_core::development::GrowthProfile;
use fc4_core::grid::{GridElem, GridProvider, TractGrid};
use fc4_core::growth_developer::{
    query_interface, DevGrid, GridHandle, GrowthDeveloper, GrowthDeveloperHost, GrowthInterface, GrowthMessage,
    GrowthStatics, Service, SourceMap,
};
use fc4_core::property::PropertyHolder;
use fc4_core::rng::FixedSeeds;
use fc4_core::stream::{InStream, RefValue, SaveObject};
use fc4_core::variant::Variant;

use crate::budget::fc4_token_callback;
use crate::development::{fc4_construction_profile, fc4_growth_item, fc4_growth_profile};
use crate::guard;
use crate::hostgrid::{
    collect_tokens, create_grid, fc4_cell_rect, fc4_grid_api, grid_token, read_property, resolve_grid, take_props,
    HostGrid,
};
use crate::props::fc4_props;
use crate::stream::{fc4_istream, fc4_ostream, fc4_ref, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

/// A message for `fc4_growth_do_message` (fc4.h `fc4_growth_message`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_growth_message {
    pub msg_type: u32,
    pub data1: u32,
    pub data2: u32,
    pub data3: u32,
    pub data4: u32,
    /// ZoningChange: the first 16 bytes of void2 `{x1, z1, x2, z2}`.
    pub rect: [i32; 4],
    pub reserved: u32,
    /// StateChanged: `GetIGZUnknown()` as a lot (borrowed), 0 = none.
    pub unknown: u64,
}

/// Diagnostics (fc4.h `fc4_growth_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_growth_state {
    pub struct_size: u32,
    pub initialized: u8,
    /// 0 idle, 1..20 (§3.2).
    pub update_state: u8,
    pub has_salt: u8,
    pub message_server: u8,
    pub developer_type: u32,
    pub purpose: u32,
    pub wealth: u32,
    pub target_zone: i32,
    pub growth_items: u32,
    pub decline_items: u32,
    pub total_capacity: i32,
    pub last_phantom_sum: i32,
    /// The city size of the last state 20 (§5.5).
    pub last_city_size: f32,
    pub last_salt: u32,
    pub start_day: u32,
    pub specials: u32,
    pub services: u32,
    pub tract_size: u32,
    pub reentrant_calls: u32,
    pub reserved: u32,
    /// Salts drawn from the shared generator (all developers).
    pub draws: u64,
    pub source_demand: u64,
    pub desirability_map: u64,
    pub capacity_map: u64,
    pub population_map: u64,
    pub max_capacity_map: u64,
    pub zoned_map: u64,
    pub working_map: u64,
}

/// One (key, float) pair.
pub type fc4_key_float_callback = Option<unsafe extern "C" fn(ctx: *mut c_void, key: u32, value: f32)>;

/// Everything a growth developer asks of the game (fc4.h `fc4_growth_host`;
/// one callback per `GrowthDeveloperHost` method, in the trait's order).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_growth_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    pub grids: fc4_grid_api,
    // lifecycle
    /// Takes a reference to service `s` (0 city, 1 BDS, 2 occupant manager, 3
    /// zone manager, 4 land value, 5 traffic); false when it is null.
    pub acquire_service: Option<unsafe extern "C" fn(U, u32) -> bool>,
    pub release_service: Option<unsafe extern "C" fn(U, u32)>,
    pub acquire_message_server: Option<unsafe extern "C" fn(U) -> bool>,
    pub release_message_server: Option<unsafe extern "C" fn(U)>,
    pub subscribe: Option<unsafe extern "C" fn(U, *const u32, usize)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, *const u32, usize)>,
    pub simulator_present: Option<unsafe extern "C" fn(U) -> bool>,
    /// `AddAgent(target, level, name, flags)`.
    pub add_agent: Option<unsafe extern "C" fn(U, u32, *const c_char, usize, u32)>,
    pub remove_agent: Option<unsafe extern "C" fn(U)>,
    pub sim_date_number: Option<unsafe extern "C" fn(U) -> u32>,
    /// `SetTractSize(S)` on one of the developer's grid tokens.
    pub grid_set_tract_size: Option<unsafe extern "C" fn(U, u64, i32)>,
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    // demand and neighbours
    /// `GetDemand(id, index)` (borrowed; 0 = none).
    pub get_demand: Option<unsafe extern "C" fn(U, u32, u32) -> u64>,
    pub query_supply: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub add_to_demand_value: Option<unsafe extern "C" fn(U, u64, f32)>,
    pub active_demand: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub connected_cities: Option<unsafe extern "C" fn(U) -> u32>,
    pub all_connected_cities: Option<unsafe extern "C" fn(U) -> u32>,
    // the BDS
    pub stage_population_multiplier: Option<unsafe extern "C" fn(U) -> f32>,
    /// `GetTractDeveloper()` (borrowed; 0 = none).
    pub tract_developer: Option<unsafe extern "C" fn(U) -> u64>,
    /// `GetSpecialBuildings()`: `cb` once per occupant in list order
    /// (borrowed).
    pub special_buildings: Option<unsafe extern "C" fn(U, fc4_token_callback, *mut c_void)>,
    pub max_capacity: Option<unsafe extern "C" fn(U, i32, u32, u32) -> f32>,
    pub building_purpose: Option<unsafe extern "C" fn(U, u32) -> u32>,
    // the tract developer
    pub update_population: Option<unsafe extern "C" fn(U, u64, *const fc4_growth_profile)>,
    // zones, lots, occupants, city
    pub zone_type: Option<unsafe extern "C" fn(U, i32, i32) -> i8>,
    pub lot_building_type: Option<unsafe extern "C" fn(U, u64, bool) -> u32>,
    pub lot_capacity: Option<unsafe extern "C" fn(U, u64, u32, bool) -> u16>,
    /// Width, depth into `out[2]`.
    pub lot_size: Option<unsafe extern "C" fn(U, u64, *mut u8)>,
    pub lot_bounding_rect: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    /// `GetLocation()` into `out[2]`.
    pub lot_location: Option<unsafe extern "C" fn(U, u64, *mut i32)>,
    /// Fills the scratch holder with property `id` of the occupant; false
    /// when absent.
    pub occupant_property: Option<unsafe extern "C" fn(U, u64, u32, *mut fc4_props) -> bool>,
    pub occupant_position: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    pub position_to_cell: Option<unsafe extern "C" fn(U, f32, f32, *mut i32)>,
    // source maps
    /// A source map (fc4.h `FC4_GROWTH_SOURCE_*` with its argument) as a grid
    /// token with one reference for the core; 0 = null.
    pub source_map: Option<unsafe extern "C" fn(U, u32, u32) -> u64>,
    // time slicing and seeds
    pub limit_timer_start: Option<unsafe extern "C" fn(U, u32)>,
    pub limit_timer_expired: Option<unsafe extern "C" fn(U) -> bool>,
    pub wall_clock_seed: Option<unsafe extern "C" fn(U) -> u32>,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_growth_message>() == 48);
    assert!(std::mem::size_of::<fc4_growth_state>() == 136);
    assert!(std::mem::size_of::<fc4_growth_host>() == if cfg!(target_pointer_width = "32") { 200 } else { 400 });
};

/// The source map codes of `source_map` (fc4.h `FC4_GROWTH_SOURCE_*`).
pub mod source {
    pub const INTRINSIC_LAND_VALUE: u32 = 0;
    pub const TOTAL_LAND_VALUE: u32 = 1;
    pub const SLOPE: u32 = 2;
    pub const RESIDENTIAL_PROXIMITY: u32 = 3;
    pub const COMMERCIAL_PROXIMITY: u32 = 4;
    pub const PARK: u32 = 5;
    pub const POLLUTION: u32 = 6;
    pub const SCHOOL_GRADE: u32 = 7;
    pub const HOSPITAL_GRADE: u32 = 8;
    pub const CRIME: u32 = 9;
    pub const TRIP_LENGTH: u32 = 10;
    pub const COMMERCIAL_TRAFFIC: u32 = 11;
    pub const LANDMARK: u32 = 12;
    pub const TRANSIENT_AURA: u32 = 13;
}

/// The code and argument of a source map.
pub fn source_code(m: SourceMap) -> (u32, u32) {
    use source::*;
    match m {
        SourceMap::IntrinsicLandValue => (INTRINSIC_LAND_VALUE, 0),
        SourceMap::TotalLandValue => (TOTAL_LAND_VALUE, 0),
        SourceMap::Slope => (SLOPE, 0),
        SourceMap::ResidentialProximity(w) => (RESIDENTIAL_PROXIMITY, w),
        SourceMap::CommercialProximity(t) => (COMMERCIAL_PROXIMITY, t),
        SourceMap::Park => (PARK, 0),
        SourceMap::Pollution(i) => (POLLUTION, i),
        SourceMap::SchoolGrade => (SCHOOL_GRADE, 0),
        SourceMap::HospitalGrade => (HOSPITAL_GRADE, 0),
        SourceMap::Crime => (CRIME, 0),
        SourceMap::TripLength => (TRIP_LENGTH, 0),
        SourceMap::CommercialTraffic => (COMMERCIAL_TRAFFIC, 0),
        SourceMap::Landmark => (LANDMARK, 0),
        SourceMap::TransientAura => (TRANSIENT_AURA, 0),
    }
}

/// The service code of `acquire_service` / `release_service`.
pub fn service_code(s: Service) -> u32 {
    match s {
        Service::City => 0,
        Service::Bds => 1,
        Service::OccupantManager => 2,
        Service::ZoneManager => 3,
        Service::LandValue => 4,
        Service::Traffic => 5,
    }
}

// ---- host adapter -----------------------------------------------------------------------------------

struct GHost(fc4_growth_host);

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.0.$f { Some(f) => unsafe { f($s.0.user $(, $arg)*) }, None => $default }
    };
}

fn opt(t: u64) -> Option<u64> {
    (t != 0).then_some(t)
}

impl GHost {
    fn source<T: GridElem>(&self, m: SourceMap) -> Option<Rc<dyn TractGrid<T>>> {
        let (code, arg) = source_code(m);
        let token = opt(cb!(self, source_map, 0, code, arg))?;
        // The core only reads a source map: released, never shut down.
        Some(HostGrid::<T>::wrap(self.0.grids, token, false) as Rc<dyn TractGrid<T>>)
    }

    fn release_token(&self, token: u64) {
        if token != 0 {
            cb!(self, release, (), token)
        }
    }
}

impl GridProvider<u8> for GHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        create_grid::<u8>(&self.0.grids, instance, tract_size, true)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u8>>> {
        resolve_grid::<u8>(&self.0.grids, r, true)
    }
}

impl GridProvider<i16> for GHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<i16>>> {
        create_grid::<i16>(&self.0.grids, instance, tract_size, true)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<i16>>> {
        resolve_grid::<i16>(&self.0.grids, r, true)
    }
}

impl GridProvider<u16> for GHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u16>>> {
        create_grid::<u16>(&self.0.grids, instance, tract_size, true)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u16>>> {
        resolve_grid::<u16>(&self.0.grids, r, true)
    }
}

fn handle_token(h: GridHandle<'_>) -> u64 {
    match h {
        GridHandle::U8(g) => grid_token(&Some(g.clone())),
        GridHandle::I16(g) => grid_token(&Some(g.clone())),
        GridHandle::U16(g) => grid_token(&Some(g.clone())),
    }
}

impl GrowthDeveloperHost for GHost {
    fn acquire_service(&self, s: Service) -> bool {
        cb!(self, acquire_service, false, service_code(s))
    }
    fn release_service(&self, s: Service) {
        cb!(self, release_service, (), service_code(s))
    }
    fn acquire_message_server(&self) -> bool {
        cb!(self, acquire_message_server, false)
    }
    fn release_message_server(&self) {
        cb!(self, release_message_server, ())
    }
    fn subscribe(&self, msg_types: &[u32]) {
        cb!(self, subscribe, (), msg_types.as_ptr(), msg_types.len())
    }
    fn unsubscribe(&self, msg_types: &[u32]) {
        cb!(self, unsubscribe, (), msg_types.as_ptr(), msg_types.len())
    }
    fn simulator_present(&self) -> bool {
        cb!(self, simulator_present, false)
    }
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        cb!(
            self,
            add_agent,
            (),
            level,
            name.as_ptr() as *const c_char,
            name.len(),
            flags
        )
    }
    fn remove_agent(&self) {
        cb!(self, remove_agent, ())
    }
    fn sim_date_number(&self) -> u32 {
        cb!(self, sim_date_number, 0)
    }
    // [choice] The tract size is set only while `Init` creates or re-reads
    // the grids, before anything asked a grid for its geometry (the host
    // grid keeps the first non-empty geometry it sees).
    fn grid_set_tract_size(&self, _which: DevGrid, grid: GridHandle<'_>, tract_size: i32) {
        let t = handle_token(grid);
        if t != 0 {
            cb!(self, grid_set_tract_size, (), t, tract_size)
        }
    }
    // The host grid shuts the game grid down and releases it when the core
    // drops it (HostGrid, shutdown_on_drop), which is the §1.3 order.
    fn grid_shutdown(&self, _which: DevGrid, _grid: GridHandle<'_>) {}
    fn add_ref(&self, token: u64) {
        if token != 0 {
            cb!(self, add_ref, (), token)
        }
    }
    fn release(&self, token: u64) {
        self.release_token(token)
    }

    fn get_demand(&self, id: u32, index: u32) -> Option<u64> {
        opt(cb!(self, get_demand, 0, id, index))
    }
    fn query_supply(&self, node: u64) -> f32 {
        cb!(self, query_supply, 0.0, node)
    }
    fn add_to_demand_value(&self, node: u64, v: f32) {
        cb!(self, add_to_demand_value, (), node, v)
    }
    fn active_demand(&self, node: u64) -> f32 {
        cb!(self, active_demand, 0.0, node)
    }
    fn connected_cities(&self) -> u32 {
        cb!(self, connected_cities, 0)
    }
    fn all_connected_cities(&self) -> u32 {
        cb!(self, all_connected_cities, 0)
    }

    fn stage_population_multiplier(&self) -> f32 {
        cb!(self, stage_population_multiplier, 1.0)
    }
    fn tract_developer(&self) -> Option<u64> {
        opt(cb!(self, tract_developer, 0))
    }
    fn special_buildings(&self) -> Vec<u64> {
        collect_tokens(|f, ctx| cb!(self, special_buildings, (), f, ctx))
            .into_iter()
            .filter(|&t| t != 0)
            .collect()
    }
    fn max_capacity(&self, zone_type: i32, purpose: u32, wealth: u32) -> f32 {
        cb!(self, max_capacity, 0.0, zone_type, purpose, wealth)
    }
    fn building_purpose(&self, building_type: u32) -> u32 {
        cb!(self, building_purpose, 0, building_type)
    }

    fn update_population(&self, tract_developer: u64, profile: &GrowthProfile) {
        let p = fc4_growth_profile::from(profile);
        cb!(self, update_population, (), tract_developer, &p)
    }

    fn zone_type(&self, cell_x: i32, cell_z: i32) -> i8 {
        cb!(self, zone_type, 0, cell_x, cell_z)
    }
    fn lot_building_type(&self, lot: u64, flag: bool) -> u32 {
        cb!(self, lot_building_type, 0, lot, flag)
    }
    fn lot_capacity(&self, lot: u64, developer_type: u32, flag: bool) -> u16 {
        cb!(self, lot_capacity, 0, lot, developer_type, flag)
    }
    fn lot_size(&self, lot: u64) -> (u8, u8) {
        let mut v = [0u8; 2];
        cb!(self, lot_size, (), lot, v.as_mut_ptr());
        (v[0], v[1])
    }
    fn lot_bounding_rect(&self, lot: u64) -> [i32; 4] {
        let mut r = fc4_cell_rect::default();
        cb!(self, lot_bounding_rect, (), lot, &mut r);
        [r.x1, r.z1, r.x2, r.z2]
    }
    fn lot_location(&self, lot: u64) -> (i32, i32) {
        let mut v = [0i32; 2];
        cb!(self, lot_location, (), lot, v.as_mut_ptr());
        (v[0], v[1])
    }
    fn occupant_property(&self, occupant: u64, id: u32) -> Option<Variant> {
        let f = self.0.occupant_property?;
        // SAFETY: valid callback; the scratch holder outlives the call.
        read_property(id, |p| unsafe { f(self.0.user, occupant, id, p) })
    }
    fn occupant_position(&self, occupant: u64) -> [f32; 3] {
        let mut v = [0.0f32; 3];
        cb!(self, occupant_position, (), occupant, v.as_mut_ptr());
        v
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        let mut v = [0i32; 2];
        cb!(self, position_to_cell, (), x, z, v.as_mut_ptr());
        (v[0], v[1])
    }

    fn source_u8(&self, map: SourceMap) -> Option<Rc<dyn TractGrid<u8>>> {
        self.source::<u8>(map)
    }
    fn source_i8(&self, map: SourceMap) -> Option<Rc<dyn TractGrid<i8>>> {
        self.source::<i8>(map)
    }
    fn source_i16(&self, map: SourceMap) -> Option<Rc<dyn TractGrid<i16>>> {
        self.source::<i16>(map)
    }
    fn source_f32(&self, map: SourceMap) -> Option<Rc<dyn TractGrid<f32>>> {
        self.source::<f32>(map)
    }

    fn limit_timer_start(&self, ms: u32) {
        cb!(self, limit_timer_start, (), ms)
    }
    fn limit_timer_expired(&self) -> bool {
        cb!(self, limit_timer_expired, true)
    }
    fn wall_clock_seed(&self) -> u32 {
        cb!(self, wall_clock_seed, 0)
    }
}

// ---- the save stream --------------------------------------------------------------------------------

/// The record reader: references come back as host tokens the core owns one
/// reference on (the grid references of §9).
struct GIn<'a> {
    inner: CIn<'a>,
    host: &'a GHost,
}

impl InStream for GIn<'_> {
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
        self.get_ref_as(0)
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

// ---- the process statics -------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_growth_statics`): the state every growth
/// developer of the process shares.
#[repr(C)]
pub struct fc4_growth_statics {
    _private: [u8; 0],
}

struct Statics(Rc<GrowthStatics>);

/// The statics, the generator seeded from the session seed (foundations §6.3
/// *[choice]*: `derive_seed(session_seed, 0xAC09C0E8)`).
#[no_mangle]
pub extern "C" fn fc4_growth_statics_create(session_seed: u32) -> *mut fc4_growth_statics {
    guard(std::ptr::null_mut(), || {
        let seeds = FixedSeeds {
            wall_clock: session_seed,
            process_clock: session_seed,
        };
        Box::into_raw(Box::new(Statics(GrowthStatics::from_session(session_seed, &seeds)))) as *mut fc4_growth_statics
    })
}

/// Destroys the statics (the developers made with them keep their share).
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_statics_destroy(s: *mut fc4_growth_statics) {
    if s.is_null() {
        return;
    }
    guard((), || drop(Box::from_raw(s as *mut Statics)));
}

/// Which sub-object `QueryInterface(iid)` answers with (§1): 0 = none, 1 =
/// the primary interface (also `cIGZUnknown`), 2 = `cIGZMessageTarget2`
/// (+4), 3 = `cIGZSerializable` (+8).
#[no_mangle]
pub extern "C" fn fc4_growth_query_interface(iid: u32) -> u32 {
    match query_interface(iid) {
        None => 0,
        Some(GrowthInterface::Primary) => 1,
        Some(GrowthInterface::MessageTarget) => 2,
        Some(GrowthInterface::Serializable) => 3,
    }
}

// ---- handles ----------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_growth`).
#[repr(C)]
pub struct fc4_growth {
    _private: [u8; 0],
}

struct Handle {
    host: Rc<GHost>,
    dev: Rc<GrowthDeveloper>,
    depth: Cell<u32>,
    queue: RefCell<Vec<GrowthMessage>>,
    reentrant: Cell<u32>,
}

impl Handle {
    fn call<R>(&self, f: impl FnOnce(&GrowthDeveloper) -> R) -> R {
        if self.depth.get() > 0 {
            self.reentrant.set(self.reentrant.get().wrapping_add(1));
        }
        self.depth.set(self.depth.get() + 1);
        let r = f(&self.dev);
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
                self.dev.handle_message(&m);
            }
        }
        self.depth.set(self.depth.get() - 1);
        r
    }
}

fn with_handle<R>(p: *const fc4_growth, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_growth*.
    let Some(h) = (unsafe { (p as *const Handle).as_ref() }) else {
        return fallback;
    };
    guard(fallback, || f(h))
}

fn dev<R>(p: *const fc4_growth, fallback: R, f: impl FnOnce(&GrowthDeveloper) -> R) -> R {
    with_handle(p, fallback, |h| h.call(f))
}

fn host_ok(host: *const fc4_growth_host) -> Option<fc4_growth_host> {
    // SAFETY: documented precondition: NULL or a valid host table.
    let h = unsafe { host.as_ref() }?;
    ((h.struct_size as usize) >= std::mem::size_of::<fc4_growth_host>()).then_some(*h)
}

/// Creates a constructed developer (§1.1, reference count 0: the plugin's
/// object keeps the count). `token` is the developer's host identity (the
/// `cISC4GrowthDeveloper*` growth profiles carry). NULL when the table or
/// the grid API is incomplete or `statics` is NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_create(
    host: *const fc4_growth_host,
    statics: *const fc4_growth_statics,
    token: u64,
) -> *mut fc4_growth {
    guard(std::ptr::null_mut(), || {
        let Some(h) = host_ok(host) else {
            return std::ptr::null_mut();
        };
        if !h.grids.complete() {
            return std::ptr::null_mut();
        }
        let Some(st) = (statics as *const Statics).as_ref().map(|s| s.0.clone()) else {
            return std::ptr::null_mut();
        };
        let host = Rc::new(GHost(h));
        let d = GrowthDeveloper::new(host.clone(), st, token);
        Box::into_raw(Box::new(Handle {
            host,
            dev: d,
            depth: Cell::new(0),
            queue: RefCell::new(Vec::new()),
            reentrant: Cell::new(0),
        })) as *mut fc4_growth
    })
}

/// Destroys the developer: the final `Release` (§1: `Shutdown` if still
/// initialised, then the special-building copy is released); grids still
/// held (a loaded developer never initialised) are released with it.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_destroy(p: *mut fc4_growth) {
    if p.is_null() {
        return;
    }
    guard((), || {
        let b = Box::from_raw(p as *mut Handle);
        b.dev.data_mut().ref_count = 1;
        b.dev.release();
        drop(b);
    });
}

/// `Init(holder)` (§1.2). `props` is a NEW fc4_props the core takes (the
/// developer exemplar); NULL is a null holder (false).
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_init(p: *mut fc4_growth, props: *mut fc4_props) -> bool {
    let holder: Option<Rc<PropertyHolder>> = take_props(props);
    dev(p, false, |d| d.init(holder))
}

/// `Shutdown` (§1.3): always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_shutdown(p: *mut fc4_growth) -> bool {
    dev(p, false, GrowthDeveloper::shutdown)
}

/// `DoMessage` (§3.1). A message delivered during another call is queued
/// (true).
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_do_message(p: *mut fc4_growth, m: *const fc4_growth_message) -> bool {
    let Some(m) = m.as_ref() else { return true };
    let msg = GrowthMessage {
        msg_type: m.msg_type,
        data1: m.data1,
        data2: m.data2,
        data3: m.data3,
        data4: m.data4,
        rect: m.rect,
        unknown: opt(m.unknown),
    };
    with_handle(p, true, |h| {
        if h.depth.get() > 0 {
            h.queue.borrow_mut().push(msg);
            return true;
        }
        h.call(|d| d.handle_message(&msg))
    })
}

// ---- the primary interface (§10.1) -----------------------------------------------------------------

/// Slot 5 `GetType`: the target zone.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_target_zone(p: *const fc4_growth) -> i32 {
    dev(p, 0, GrowthDeveloper::target_zone)
}

/// Slot 6 `GetSubType`: the developer type.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_sub_type(p: *const fc4_growth) -> u32 {
    dev(p, 0, GrowthDeveloper::sub_type)
}

/// Slot 7 `GetPurpose`.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_purpose(p: *const fc4_growth) -> u32 {
    dev(p, 0, GrowthDeveloper::purpose)
}

/// Slot 8 `GetWealth`.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_wealth(p: *const fc4_growth) -> u32 {
    dev(p, 0, GrowthDeveloper::wealth)
}

/// Slot 9 `GetConstructionInfo(profile&)` (§6.2): writes `zone` … the land
/// value range; the rest of `*out` is left as it was.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_construction_info(p: *const fc4_growth, out: *mut fc4_construction_profile) {
    let Some(o) = out.as_mut() else { return };
    let mut prof = o.to_core();
    dev(p, (), |d| d.construction_info(&mut prof));
    o.assign_info(&prof);
}

/// Slot 10 `GrowthAndDeclineListsAreValid`.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_lists_valid(p: *const fc4_growth) -> bool {
    dev(p, false, GrowthDeveloper::lists_valid)
}

/// Slots 11 / 12: the growth (`which` 0) or decline (1) list into `out` (at
/// most `n`); the list's length.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_list(
    p: *const fc4_growth,
    which: u32,
    out: *mut fc4_growth_item,
    n: usize,
) -> usize {
    let v = dev(p, Vec::new(), |d| match which {
        0 => d.growth_items(),
        1 => d.decline_items(),
        _ => Vec::new(),
    });
    if !out.is_null() {
        for (i, it) in v.iter().take(n).enumerate() {
            *out.add(i) = fc4_growth_item::from(*it);
        }
    }
    v.len()
}

/// Slots 13–16 and the internal grids: the grid token (borrowed; 0 = none).
/// `which`: 0 D (slot 13), 1 capacity (14), 2 population (15), 3 maximum
/// capacity (16), 4 the zoned-cell count grid, 5 W.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_grid(p: *const fc4_growth, which: u32) -> u64 {
    dev(p, 0, |d| match which {
        0 => grid_token(&d.desirability_map()),
        1 => grid_token(&d.capacity_map()),
        2 => grid_token(&d.population_map()),
        3 => grid_token(&d.max_capacity_map()),
        4 => grid_token(&d.zoned_count_map()),
        5 => grid_token(&d.working_map()),
        _ => 0,
    })
}

/// Slot 17 `GetTotalCapacity`.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_total_capacity(p: *const fc4_growth) -> i32 {
    dev(p, 0, GrowthDeveloper::total_capacity)
}

/// Slot 18 `GetSourceDemand` (borrowed; 0 = none).
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_source_demand(p: *const fc4_growth) -> u64 {
    dev(p, None, GrowthDeveloper::source_demand).unwrap_or(0)
}

/// Slots 19–22: the power, water, fire and park caps into `out[4]`.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_caps(p: *const fc4_growth, out: *mut u8) {
    let v = dev(p, [0u8; 4], GrowthDeveloper::caps);
    if !out.is_null() {
        std::ptr::copy_nonoverlapping(v.as_ptr(), out, 4);
    }
}

/// Slot 23 `GetDesirabilityThreshold`.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_desirability_threshold(p: *const fc4_growth) -> i32 {
    dev(p, 0, GrowthDeveloper::desirability_threshold)
}

/// Slot 24 `GetAbandonmentThreshold(key)` (inserts 0.0 for an unknown key).
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_abandonment_threshold(p: *mut fc4_growth, key: i32) -> f32 {
    dev(p, 0.0, |d| d.abandonment_threshold(key))
}

/// Slot 25 `GetMonopolyUse`.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_monopoly_use(p: *const fc4_growth) -> u32 {
    dev(p, 0, GrowthDeveloper::monopoly_use)
}

/// Slot 26 `GetStylePercentages`: `cb(style, fraction)` in ascending style;
/// the entry count.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_styles(
    p: *const fc4_growth,
    cb: fc4_key_float_callback,
    ctx: *mut c_void,
) -> usize {
    let v: Vec<(u32, f32)> = dev(p, Vec::new(), |d| {
        d.style_percentages().iter().map(|(&k, &v)| (k, v)).collect()
    });
    if let Some(f) = cb {
        for &(k, x) in &v {
            f(ctx, k, x);
        }
    }
    v.len()
}

/// Slot 27 `StartMapUpdate`.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_start_map_update(p: *mut fc4_growth) {
    dev(p, (), GrowthDeveloper::start_map_update)
}

/// Slot 28 `IsMapUpdateFinished`.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_map_update_finished(p: *const fc4_growth) -> bool {
    dev(p, true, GrowthDeveloper::is_map_update_finished)
}

/// Slot 29 `EstimateDesirability(x, z)` (§4.6): the int16 result
/// sign-extended.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_estimate_desirability(p: *const fc4_growth, x: i32, z: i32) -> i32 {
    dev(p, 0, |d| d.estimate_desirability(x, z))
}

/// Slot 30 `GetSubjectiveThresholds(i, min&, max&)`: each output written
/// when not NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_subjective_thresholds(p: *const fc4_growth, i: i32, min: *mut i32, max: *mut i32) {
    let (a, b) = dev(p, (0, 0), |d| d.subjective_thresholds(i));
    if let Some(o) = min.as_mut() {
        *o = a;
    }
    if let Some(o) = max.as_mut() {
        *o = b;
    }
}

/// `GetSourceDemand().QueryActiveDemandValue()` (the BDS's `ad(D)`); false
/// without a source node.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_source_active_demand(p: *const fc4_growth, out: *mut f32) -> bool {
    let Some(v) = dev(p, None, GrowthDeveloper::source_active_demand) else {
        return false;
    };
    if let Some(o) = out.as_mut() {
        *o = v;
    }
    true
}

// ---- diagnostics and persistence -----------------------------------------------------------------------

/// Diagnostics.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_get_state(p: *const fc4_growth, out: *mut fc4_growth_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_handle(p, false, |h| {
        let d = &h.dev;
        let s = d.data();
        *o = fc4_growth_state {
            struct_size: std::mem::size_of::<fc4_growth_state>() as u32,
            initialized: u8::from(s.initialized),
            update_state: s.update_state.min(255) as u8,
            has_salt: u8::from(s.last_salt.is_some()),
            message_server: u8::from(s.message_server),
            developer_type: s.tuning.developer_type,
            purpose: u32::from(s.tuning.purpose),
            wealth: u32::from(s.tuning.wealth),
            target_zone: i32::from(s.tuning.target_zone),
            growth_items: s.growth.len() as u32,
            decline_items: s.decline.len() as u32,
            total_capacity: s.total_capacity,
            last_phantom_sum: s.last_phantom_sum,
            last_city_size: s.last_city_size,
            last_salt: s.last_salt.unwrap_or(0),
            start_day: s.start_day,
            specials: s.specials.len() as u32,
            services: s.services.len() as u32,
            tract_size: u32::from(s.tuning.tract_size),
            reentrant_calls: h.reentrant.get(),
            reserved: 0,
            draws: d.statics().draws(),
            source_demand: s.source_demand.unwrap_or(0),
            desirability_map: grid_token(&d.desirability_map()),
            capacity_map: grid_token(&d.capacity_map()),
            population_map: grid_token(&d.population_map()),
            max_capacity_map: grid_token(&d.max_capacity_map()),
            zoned_map: grid_token(&d.zoned_count_map()),
            working_map: grid_token(&d.working_map()),
        };
        true
    })
}

/// `Write` (§9): the v1 record.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_write(p: *const fc4_growth, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    dev(p, false, |d| d.write(&mut COut(o)))
}

/// `Read` (§9). Runs before `Init`.
#[no_mangle]
pub unsafe extern "C" fn fc4_growth_read(p: *mut fc4_growth, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_handle(p, false, |h| {
        let host = h.host.clone();
        h.call(|d| {
            d.read(&mut GIn {
                inner: CIn {
                    stream: i,
                    decode: no_core_objects,
                },
                host: &host,
            })
        })
    })
}
