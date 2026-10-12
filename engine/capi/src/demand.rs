// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The demand subsystem (`docs/specs/demand.md`) over the C ABI: the demand
//! simulator (`fc4_demand_sim`) and demand nodes (`fc4_demand_node`). See the
//! "demand" section of `include/fc4.h`.
//!
//! Every GZCOM demand node of the shim owns one `fc4_demand_node`; the
//! simulator's map holds the core objects of the nodes it created or read
//! back from a save (all of them ours: foreign nodes never enter the map, see
//! `fc4_core::demand`).

use std::collections::BTreeMap;
use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::demand::{
    BuildingProfile, CensusLot, Connection, DemandHost, DemandNode, DemandPoster, DemandSim, GrowthDeveloper,
    CLSID_DEMAND_NODE,
};
use fc4_core::property::PropertyHolder;
use fc4_core::stream::{ResKey, SaveObject};

use crate::budget::fc4_token_callback;
use crate::guard;
use crate::ordinance::fc4_key_visitor;
use crate::props::fc4_props;
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

#[repr(C)]
pub struct fc4_demand_sim {
    _private: [u8; 0],
}

#[repr(C)]
pub struct fc4_demand_node {
    _private: [u8; 0],
}

type U = *mut c_void;

/// One neighbour connection (§8.3).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_demand_connection {
    /// The connection (its `cISC4NeighborConnection*`).
    pub token: u64,
    /// `GetCityOnOtherSideOfConnection()` (0 = none).
    pub city: u64,
    /// `GetType()`.
    pub conn_type: u32,
    pub reserved: u32,
}

/// One growth developer (§9.2).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_demand_growth_developer {
    pub sub_type: u32,
    /// `GetType()`: 1 R, 2 C, 3 I.
    pub kind: u32,
    /// 0 = `GetSourceDemand()` returned no node.
    pub has_source: u32,
    /// The source node's committed supply and demand.
    pub supply: f32,
    pub demand: f32,
    pub reserved: u32,
}

/// Diagnostics (`fc4_demand_sim_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_demand_sim_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub loaded_from_save: u8,
    pub census_state: u8,
    pub _pad: u8,
    pub node_count: u32,
    pub link_count: u32,
    pub cursor_x: i32,
    pub cursor_z: i32,
    pub co_economy_bonus: f32,
    pub i_economy_bonus: f32,
    pub commercial_relief: f32,
    pub industrial_relief: f32,
    pub export_city_count: u32,
}

pub type fc4_capacity_visitor = Option<unsafe extern "C" fn(ctx: *mut c_void, map_index: u32, dev_type: u32, cap: u16)>;
pub type fc4_u32_callback = Option<unsafe extern "C" fn(ctx: *mut c_void, value: u32)>;
pub type fc4_connection_callback = Option<unsafe extern "C" fn(ctx: *mut c_void, c: *const fc4_demand_connection)>;
pub type fc4_growth_developer_callback =
    Option<unsafe extern "C" fn(ctx: *mut c_void, g: *const fc4_demand_growth_developer)>;
pub type fc4_population_visitor = Option<unsafe extern "C" fn(ctx: *mut c_void, key: u32, value: i32)>;

/// What the demand subsystem needs from the game (fc4.h: `fc4_demand_host`).
/// Missing callbacks read as "nothing / 0 / false".
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_demand_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    // resources
    pub for_each_exemplar_key: Option<unsafe extern "C" fn(U, fc4_key_visitor, *mut c_void)>,
    pub exemplar: Option<unsafe extern "C" fn(U, u32, u32, u32) -> *mut fc4_props>,
    pub warn: Option<unsafe extern "C" fn(U, *const u8, usize)>,
    // nodes
    pub create_node: Option<unsafe extern "C" fn(U, *mut u64) -> *mut fc4_demand_node>,
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    pub post_active_demand: Option<unsafe extern "C" fn(U, u64, u32, i32)>,
    // city and clock
    pub cell_counts: Option<unsafe extern "C" fn(U, *mut i32, *mut i32)>,
    pub today: Option<unsafe extern "C" fn(U) -> u32>,
    pub now_ms: Option<unsafe extern "C" fn(U) -> u32>,
    // census
    pub census_lot: Option<unsafe extern "C" fn(U, i32, i32, *mut u32, *mut u32) -> bool>,
    pub building_profile: Option<unsafe extern "C" fn(U, u32, *mut u32, fc4_capacity_visitor, *mut c_void) -> bool>,
    pub developer_type: Option<unsafe extern "C" fn(U, u32, u32) -> u32>,
    pub desirability: Option<unsafe extern "C" fn(U, u32, i32, i32) -> i16>,
    pub lot_capacity: Option<unsafe extern "C" fn(U, i32, i32, u32) -> u16>,
    pub cell_eq: Option<unsafe extern "C" fn(U, i32, i32) -> f32>,
    pub cell_workforce_percent: Option<unsafe extern "C" fn(U, i32, i32) -> f32>,
    // occupancy
    pub lot_building_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub lot_excluded: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub all_building_types: Option<unsafe extern "C" fn(U, fc4_u32_callback, *mut c_void)>,
    pub building_exemplar: Option<unsafe extern "C" fn(U, u32) -> *mut fc4_props>,
    // modifiers
    pub history_max: Option<unsafe extern "C" fn(U, u32, u32, u32, *mut f32) -> bool>,
    pub ordinance_factor: Option<unsafe extern "C" fn(U, u32, u32) -> f32>,
    pub nation_economy_modifier: Option<unsafe extern "C" fn(U, u32) -> f32>,
    // cap relief
    pub connections: Option<unsafe extern "C" fn(U, fc4_connection_callback, *mut c_void)>,
    pub traffic_arrived: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub airport_trips: Option<unsafe extern "C" fn(U) -> f32>,
    pub seaport_trips: Option<unsafe extern "C" fn(U) -> f32>,
    // region
    pub all_connected_cities: Option<unsafe extern "C" fn(U, fc4_token_callback, *mut c_void)>,
    pub connected_cities: Option<unsafe extern "C" fn(U, fc4_token_callback, *mut c_void)>,
    pub own_regional_city: Option<unsafe extern "C" fn(U) -> u64>,
    pub city_population: Option<unsafe extern "C" fn(U, u64, u32) -> i32>,
    pub city_extrapolated_population: Option<unsafe extern "C" fn(U, u64, u32) -> i32>,
    pub city_allowable_extrapolation: Option<unsafe extern "C" fn(U, u64, u32) -> i32>,
    pub city_tax_rate: Option<unsafe extern "C" fn(U, u64, u32) -> f32>,
    pub city_workforce_percentage: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub city_extrapolate_growth: Option<unsafe extern "C" fn(U, u64, u32, f32)>,
    pub growth_developers: Option<unsafe extern "C" fn(U, fc4_growth_developer_callback, *mut c_void)>,
    pub day_of_last_successful_build: Option<unsafe extern "C" fn(U, u32) -> u32>,
    // history
    pub record_history: Option<unsafe extern "C" fn(U, u32, i32)>,
    pub set_history_queue_length: Option<unsafe extern "C" fn(U, u32, u32)>,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_demand_connection>() == 24);
    assert!(std::mem::size_of::<fc4_demand_growth_developer>() == 24);
    assert!(std::mem::size_of::<fc4_demand_sim_state>() == 44);
    assert!(std::mem::size_of::<fc4_demand_host>() == if cfg!(target_pointer_width = "32") { 172 } else { 344 });
};

/// Node values (`fc4_demand_node_get` / `fc4_demand_node_set`, fc4.h
/// `FC4_DEMAND_*`).
pub mod value {
    pub const SUPPLY: u32 = 0;
    pub const DEMAND: u32 = 1;
    pub const NEW_SUPPLY: u32 = 2;
    pub const NEW_DEMAND: u32 = 3;
    pub const ACTIVE_DEMAND: u32 = 4;
    pub const ECONOMY_MODIFIER: u32 = 5;
    pub const ACTIVE_MAX: u32 = 6;
    pub const ACTIVE_MIN: u32 = 7;
    pub const TAX_MODIFIER: u32 = 8;
    pub const DEMAND_CAP: u32 = 9;
}

// ---- host adapter -----------------------------------------------------------------------------------

macro_rules! cb {
    ($h:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $h.$f { Some(f) => unsafe { f($h.user $(, $arg)*) }, None => $default }
    };
}

unsafe extern "C" fn collect_key(ctx: *mut c_void, type_id: u32, group: u32, instance: u32) {
    (*(ctx as *mut Vec<ResKey>)).push(ResKey {
        type_id,
        group,
        instance,
    });
}

unsafe extern "C" fn collect_token(ctx: *mut c_void, token: u64) {
    (*(ctx as *mut Vec<u64>)).push(token);
}

unsafe extern "C" fn collect_u32(ctx: *mut c_void, v: u32) {
    (*(ctx as *mut Vec<u32>)).push(v);
}

unsafe extern "C" fn collect_capacity(ctx: *mut c_void, map_index: u32, dev_type: u32, cap: u16) {
    let maps = &mut *(ctx as *mut Vec<Vec<(u32, u16)>>);
    let i = map_index as usize;
    if maps.len() <= i {
        maps.resize(i + 1, Vec::new());
    }
    maps[i].push((dev_type, cap));
}

unsafe extern "C" fn collect_connection(ctx: *mut c_void, c: *const fc4_demand_connection) {
    if let Some(c) = c.as_ref() {
        (*(ctx as *mut Vec<Connection>)).push(Connection {
            token: c.token,
            conn_type: c.conn_type,
            city: c.city,
        });
    }
}

unsafe extern "C" fn collect_developer(ctx: *mut c_void, g: *const fc4_demand_growth_developer) {
    if let Some(g) = g.as_ref() {
        (*(ctx as *mut Vec<GrowthDeveloper>)).push(GrowthDeveloper {
            sub_type: g.sub_type,
            kind: g.kind,
            source: (g.has_source != 0).then_some((g.supply, g.demand)),
        });
    }
}

/// Takes ownership of a holder the host built.
unsafe fn take_props(p: *mut fc4_props) -> Option<PropertyHolder> {
    (!p.is_null()).then(|| *Box::from_raw(p as *mut PropertyHolder))
}

struct CHost(fc4_demand_host);

impl DemandPoster for CHost {
    fn active_demand_changed(&self, node: u64, key: u32, data2: i32) {
        cb!(self.0, post_active_demand, (), node, key, data2)
    }
}

impl CHost {
    fn tokens(&self, f: Option<unsafe extern "C" fn(U, fc4_token_callback, *mut c_void)>) -> Vec<u64> {
        let mut v: Vec<u64> = Vec::new();
        if let Some(f) = f {
            // SAFETY: valid callback (precondition).
            unsafe { f(self.0.user, Some(collect_token), &mut v as *mut Vec<u64> as *mut c_void) };
        }
        v
    }
}

impl DemandHost for CHost {
    fn exemplar_keys(&self) -> Vec<ResKey> {
        let mut v: Vec<ResKey> = Vec::new();
        cb!(
            self.0,
            for_each_exemplar_key,
            (),
            Some(collect_key),
            &mut v as *mut Vec<ResKey> as *mut c_void
        );
        v
    }
    fn exemplar(&self, key: ResKey) -> Option<Rc<PropertyHolder>> {
        let p = cb!(
            self.0,
            exemplar,
            std::ptr::null_mut(),
            key.type_id,
            key.group,
            key.instance
        );
        // SAFETY: the callback returns NULL or a new fc4_props it hands over.
        unsafe { take_props(p) }.map(Rc::new)
    }
    fn warn(&self, text: &str) {
        cb!(self.0, warn, (), text.as_ptr(), text.len())
    }
    fn create_node(&self) -> Option<Rc<DemandNode>> {
        let mut token = 0u64;
        let h = cb!(self.0, create_node, std::ptr::null_mut(), &mut token);
        // SAFETY: NULL or a live fc4_demand_node* (the host's new object).
        let n = unsafe { node_of(h) }?.clone();
        if token != 0 {
            n.set_token(token);
        }
        Some(n)
    }
    fn add_ref(&self, token: u64) {
        cb!(self.0, add_ref, (), token)
    }
    fn release(&self, token: u64) {
        cb!(self.0, release, (), token)
    }
    fn cell_counts(&self) -> (i32, i32) {
        let (mut x, mut z) = (0i32, 0i32);
        cb!(self.0, cell_counts, (), &mut x, &mut z);
        (x, z)
    }
    fn today(&self) -> u32 {
        cb!(self.0, today, 0)
    }
    fn now_ms(&self) -> u32 {
        cb!(self.0, now_ms, 0)
    }
    fn census_lot(&self, x: i32, z: i32) -> Option<CensusLot> {
        let (mut bt, mut wealth) = (0u32, 0u32);
        cb!(self.0, census_lot, false, x, z, &mut bt, &mut wealth).then_some(CensusLot {
            building_type: bt,
            wealth,
        })
    }
    fn building_profile(&self, building_type: u32) -> Option<BuildingProfile> {
        let mut purpose = 0u32;
        let mut maps: Vec<Vec<(u32, u16)>> = Vec::new();
        let found = cb!(
            self.0,
            building_profile,
            false,
            building_type,
            &mut purpose,
            Some(collect_capacity),
            &mut maps as *mut Vec<Vec<(u32, u16)>> as *mut c_void
        );
        found.then_some(BuildingProfile {
            purpose,
            capacities: maps,
        })
    }
    fn developer_type(&self, purpose: u32, wealth: u32) -> u32 {
        cb!(self.0, developer_type, 0, purpose, wealth)
    }
    fn desirability(&self, dev_type: u32, x: i32, z: i32) -> i16 {
        cb!(self.0, desirability, 0, dev_type, x, z)
    }
    fn lot_capacity(&self, x: i32, z: i32, dev_type: u32) -> u16 {
        cb!(self.0, lot_capacity, 0, x, z, dev_type)
    }
    fn cell_eq(&self, x: i32, z: i32) -> f32 {
        cb!(self.0, cell_eq, 0.0, x, z)
    }
    fn cell_workforce_percent(&self, x: i32, z: i32) -> f32 {
        cb!(self.0, cell_workforce_percent, 0.0, x, z)
    }
    fn lot_building_type(&self, lot: u64) -> u32 {
        cb!(self.0, lot_building_type, 0, lot)
    }
    fn lot_excluded(&self, lot: u64) -> bool {
        cb!(self.0, lot_excluded, false, lot)
    }
    fn all_building_types(&self) -> Vec<u32> {
        let mut v: Vec<u32> = Vec::new();
        cb!(
            self.0,
            all_building_types,
            (),
            Some(collect_u32),
            &mut v as *mut Vec<u32> as *mut c_void
        );
        v
    }
    fn building_exemplar(&self, building_type: u32) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self.0, building_exemplar, std::ptr::null_mut(), building_type);
        // SAFETY: as for `exemplar`.
        unsafe { take_props(p) }.map(Rc::new)
    }
    fn history_max(&self, series: u32, from_day: u32, to_day: u32) -> Option<f32> {
        let mut v = 0.0f32;
        cb!(self.0, history_max, false, series, from_day, to_day, &mut v).then_some(v)
    }
    fn ordinance_factor(&self, prop: u32, index: u32) -> f32 {
        cb!(self.0, ordinance_factor, 1.0, prop, index)
    }
    fn nation_economy_modifier(&self, demand_id: u32) -> f32 {
        cb!(self.0, nation_economy_modifier, 0.0, demand_id)
    }
    fn connections(&self) -> Vec<Connection> {
        let mut v: Vec<Connection> = Vec::new();
        cb!(
            self.0,
            connections,
            (),
            Some(collect_connection),
            &mut v as *mut Vec<Connection> as *mut c_void
        );
        v
    }
    fn traffic_arrived(&self, connection: u64) -> f32 {
        cb!(self.0, traffic_arrived, 0.0, connection)
    }
    fn airport_trips(&self) -> f32 {
        cb!(self.0, airport_trips, 0.0)
    }
    fn seaport_trips(&self) -> f32 {
        cb!(self.0, seaport_trips, 0.0)
    }
    fn all_connected_cities(&self) -> Vec<u64> {
        self.tokens(self.0.all_connected_cities)
    }
    fn connected_cities(&self) -> Vec<u64> {
        self.tokens(self.0.connected_cities)
    }
    fn own_regional_city(&self) -> u64 {
        cb!(self.0, own_regional_city, 0)
    }
    fn city_population(&self, city: u64, pop_type: u32) -> i32 {
        cb!(self.0, city_population, 0, city, pop_type)
    }
    fn city_extrapolated_population(&self, city: u64, pop_type: u32) -> i32 {
        cb!(self.0, city_extrapolated_population, 0, city, pop_type)
    }
    fn city_allowable_extrapolation(&self, city: u64, pop_type: u32) -> i32 {
        cb!(self.0, city_allowable_extrapolation, 0, city, pop_type)
    }
    fn city_tax_rate(&self, city: u64, tax_type: u32) -> f32 {
        cb!(self.0, city_tax_rate, 0.0, city, tax_type)
    }
    fn city_workforce_percentage(&self, city: u64) -> f32 {
        cb!(self.0, city_workforce_percentage, 0.0, city)
    }
    fn city_extrapolate_growth(&self, city: u64, pop_type: u32, amount: f32) {
        cb!(self.0, city_extrapolate_growth, (), city, pop_type, amount)
    }
    fn growth_developers(&self) -> Vec<GrowthDeveloper> {
        let mut v: Vec<GrowthDeveloper> = Vec::new();
        cb!(
            self.0,
            growth_developers,
            (),
            Some(collect_developer),
            &mut v as *mut Vec<GrowthDeveloper> as *mut c_void
        );
        v
    }
    fn day_of_last_successful_build(&self, dev_type: u32) -> u32 {
        cb!(self.0, day_of_last_successful_build, 0, dev_type)
    }
    fn record_history(&self, series: u32, value: i32) {
        cb!(self.0, record_history, (), series, value)
    }
    fn set_history_queue_length(&self, series: u32, length: u32) {
        cb!(self.0, set_history_queue_length, (), series, length)
    }
}

// ---- handles ----------------------------------------------------------------------------------------

unsafe fn sim_of<'a>(s: *const fc4_demand_sim) -> Option<&'a Rc<DemandSim>> {
    (s as *const Rc<DemandSim>).as_ref()
}

unsafe fn node_of<'a>(n: *const fc4_demand_node) -> Option<&'a Rc<DemandNode>> {
    (n as *const Rc<DemandNode>).as_ref()
}

fn with_sim<R>(s: *const fc4_demand_sim, fallback: R, f: impl FnOnce(&Rc<DemandSim>) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_demand_sim*.
    match unsafe { sim_of(s) } {
        Some(x) => guard(fallback, || f(x)),
        None => fallback,
    }
}

fn with_node<R>(n: *const fc4_demand_node, fallback: R, f: impl FnOnce(&Rc<DemandNode>) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_demand_node*.
    match unsafe { node_of(n) } {
        Some(x) => guard(fallback, || f(x)),
        None => fallback,
    }
}

fn host_of(host: *const fc4_demand_host) -> Option<Rc<CHost>> {
    // SAFETY: NULL or a valid struct (precondition).
    let h = unsafe { host.as_ref() }?;
    ((h.struct_size as usize) >= std::mem::size_of::<fc4_demand_host>()).then(|| Rc::new(CHost(*h)))
}

/// Turns the core handle of one of our demand nodes into its object.
fn decoder(class_id: u32, handle: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    if class_id != CLSID_DEMAND_NODE {
        return None;
    }
    // SAFETY: the host's reference reads only report core handles of our own
    // objects, and only demand nodes have this class ID.
    unsafe { node_of(handle as *const fc4_demand_node) }.map(|n| n.clone() as Rc<dyn SaveObject>)
}

// ---- simulator ----------------------------------------------------------------------------------------

/// Creates the simulator's core for the GZCOM object `token`. The host table
/// is copied.
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_sim_create(host: *const fc4_demand_host, token: u64) -> *mut fc4_demand_sim {
    guard(std::ptr::null_mut(), || match host_of(host) {
        Some(h) => Box::into_raw(Box::new(DemandSim::new(h, token))) as *mut fc4_demand_sim,
        None => std::ptr::null_mut(),
    })
}

/// Destroys the core; every node in the map is released.
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_sim_destroy(s: *mut fc4_demand_sim) {
    if !s.is_null() {
        guard((), || drop(Box::from_raw(s as *mut Rc<DemandSim>)));
    }
}

/// `Init` (§13, core part: subscriptions and agents are the shim's).
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_sim_init(s: *mut fc4_demand_sim) -> bool {
    with_sim(s, false, |x| x.init())
}

/// `Shutdown` (§13, core part).
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_sim_shutdown(s: *mut fc4_demand_sim) -> bool {
    with_sim(s, false, |x| x.shutdown())
}

/// `GetDemand(demandID, demandIndex)` (§5): initializes first if needed.
/// The node's token (no reference added), 0 if none.
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_sim_get_demand(s: *mut fc4_demand_sim, demand_id: u32, demand_index: u32) -> u64 {
    with_sim(s, 0, |x| x.get_demand(demand_id, demand_index).map_or(0, |n| n.token()))
}

/// `UpdateOccupantEffects(buildingType, oldPct, newPct)` (§5).
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_sim_update_occupant_effects(
    s: *mut fc4_demand_sim,
    building_type: u32,
    old_pct: f32,
    new_pct: f32,
) {
    with_sim(s, (), |x| x.update_occupant_effects(building_type, old_pct, new_pct))
}

/// `CalculateJobsPerUnitOfDemand` (§5): writes 3 floats to `out` (if not NULL).
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_sim_jobs_per_unit(
    s: *const fc4_demand_sim,
    demand_id: u32,
    demand_index: u32,
    out: *mut f32,
) {
    let j = with_sim(s, [0.0f32; 3], |x| x.jobs_per_unit(demand_id, demand_index));
    if !out.is_null() {
        std::ptr::copy_nonoverlapping(j.as_ptr(), out, 3);
    }
}

/// `GetJobsBySensus(type)` (§5).
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_sim_jobs_by_census(s: *const fc4_demand_sim, census_type: u32) -> u32 {
    with_sim(s, 0, |x| x.jobs_by_census(census_type))
}

/// `GetNeutralTaxRate()` (§5), unclamped.
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_sim_neutral_tax_rate(s: *const fc4_demand_sim) -> f32 {
    with_sim(s, 0.0, |x| x.neutral_tax_rate())
}

/// `GetLocalPopulationSummary` (§5): visits (key, value) in key order;
/// returns the number of entries.
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_sim_local_population_summary(
    s: *const fc4_demand_sim,
    visit: fc4_population_visitor,
    ctx: *mut c_void,
) -> usize {
    let map: BTreeMap<u32, i32> = with_sim(s, BTreeMap::new(), |x| x.local_population_summary());
    if let Some(f) = visit {
        for (k, v) in &map {
            f(ctx, *k, *v);
        }
    }
    map.len()
}

/// `DoMessage` (§10.2) for every message but StateChanged. Always true.
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_sim_handle_message(
    s: *mut fc4_demand_sim,
    msg_type: u32,
    data1: u32,
    data2: u32,
    data3: u32,
) -> bool {
    with_sim(s, true, |x| x.handle_message(msg_type, data1, data2, data3))
}

/// kMessageStateChanged (§7): `lot` = the message's IGZUnknown (the lot's
/// token for the host's lot callbacks), the old / new HabitationState, the
/// building type (0 = unknown) and the occupant wealth.
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_sim_state_changed(
    s: *mut fc4_demand_sim,
    lot: u64,
    old_state: u32,
    new_state: u32,
    building_type: u32,
    wealth: u32,
) {
    with_sim(s, (), |x| {
        x.on_state_changed(lot, old_state, new_state, building_type, wealth)
    })
}

/// The saved CO/I economy bonuses (§8.1, §15 Q5).
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_sim_set_economy_bonuses(s: *mut fc4_demand_sim, co: f32, i: f32) {
    with_sim(s, (), |x| x.set_economy_bonuses(co, i))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_demand_sim_get_state(s: *const fc4_demand_sim, out: *mut fc4_demand_sim_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    if (o.struct_size as usize) < std::mem::size_of::<fc4_demand_sim_state>() {
        return false;
    }
    with_sim(s, false, |x| {
        let snap = x.snapshot();
        *o = fc4_demand_sim_state {
            struct_size: o.struct_size,
            initialized: u8::from(snap.initialized),
            loaded_from_save: u8::from(snap.loaded_from_save),
            census_state: snap.census_state,
            _pad: 0,
            node_count: snap.node_keys.len() as u32,
            link_count: snap.link_count as u32,
            cursor_x: snap.cursor.0,
            cursor_z: snap.cursor.1,
            co_economy_bonus: snap.co_economy_bonus,
            i_economy_bonus: snap.i_economy_bonus,
            commercial_relief: snap.commercial_relief,
            industrial_relief: snap.industrial_relief,
            export_city_count: snap.export_cities.len() as u32,
        };
        true
    })
}

/// `Write` (§12.1). Nodes go out as references by token.
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_sim_write(s: *const fc4_demand_sim, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_sim(s, false, |x| x.write(&mut COut(o)))
}

/// `Read` (§12.1). References to our nodes arrive with their core handle
/// (no reference: the map takes one through add_ref); any other object is
/// released and skipped.
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_sim_read(s: *mut fc4_demand_sim, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_sim(s, false, |x| {
        x.read(&mut CIn {
            stream: i,
            decode: decoder,
        })
    })
}

// ---- nodes ----------------------------------------------------------------------------------------------

/// Creates a demand node's core for the GZCOM object `token` (its
/// `cISC4Demand*`); only `post_active_demand` of the host table is used.
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_node_create(host: *const fc4_demand_host, token: u64) -> *mut fc4_demand_node {
    guard(std::ptr::null_mut(), || match host_of(host) {
        Some(h) => Box::into_raw(Box::new(DemandNode::new(h, token))) as *mut fc4_demand_node,
        None => std::ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_demand_node_destroy(n: *mut fc4_demand_node) {
    if !n.is_null() {
        guard((), || drop(Box::from_raw(n as *mut Rc<DemandNode>)));
    }
}

macro_rules! node_fn {
    ($name:ident, $ret:ty, $fallback:expr, $method:ident $(, $arg:ident : $t:ty)*) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name(n: *mut fc4_demand_node $(, $arg: $t)*) -> $ret {
            with_node(n, $fallback, |x| x.$method($($arg),*))
        }
    };
}

node_fn!(fc4_demand_node_init, bool, false, init);
node_fn!(fc4_demand_node_shutdown, bool, false, shutdown);
node_fn!(fc4_demand_node_simulation_begin, bool, false, simulation_begin);
node_fn!(fc4_demand_node_id, u32, 0, id);
node_fn!(fc4_demand_node_set_id, bool, false, set_id, id: u32);
node_fn!(fc4_demand_node_region_use, u32, 0, region_use);
node_fn!(fc4_demand_node_set_region_use, bool, false, set_region_use, v: u32);
node_fn!(fc4_demand_node_end_of_cycle, bool, false, end_of_cycle);

/// One value (`FC4_DEMAND_*`): the Query*/Get* accessors.
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_node_get(n: *const fc4_demand_node, which: u32) -> f32 {
    with_node(n, 0.0, |x| match which {
        value::SUPPLY => x.supply(),
        value::DEMAND => x.demand(),
        value::NEW_SUPPLY => x.new_supply(),
        value::NEW_DEMAND => x.new_demand(),
        value::ACTIVE_DEMAND => x.active_demand(),
        value::ECONOMY_MODIFIER => x.economy_modifier(),
        value::ACTIVE_MAX => x.active_max(),
        value::ACTIVE_MIN => x.active_min(),
        value::TAX_MODIFIER => x.tax_modifier(),
        value::DEMAND_CAP => x.demand_cap(),
        _ => 0.0,
    })
}

/// The setters: SUPPLY / DEMAND (SetSupplyValue / SetDemandValue),
/// ACTIVE_MAX, ACTIVE_MIN, ECONOMY_MODIFIER, TAX_MODIFIER, DEMAND_CAP. False
/// for the other values.
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_node_set(n: *mut fc4_demand_node, which: u32, v: f32) -> bool {
    with_node(n, false, |x| match which {
        value::SUPPLY => x.set_supply(v),
        value::DEMAND => x.set_demand(v),
        value::ACTIVE_MAX => x.set_active_max(v),
        value::ACTIVE_MIN => x.set_active_min(v),
        value::ECONOMY_MODIFIER => x.set_economy_modifier(v),
        value::TAX_MODIFIER => x.set_tax_modifier(v),
        value::DEMAND_CAP => x.set_demand_cap(v),
        _ => false,
    })
}

/// AddToSupplyValue (`which` = SUPPLY) / AddToDemandValue (DEMAND).
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_node_add(n: *mut fc4_demand_node, which: u32, v: f32) -> bool {
    with_node(n, false, |x| match which {
        value::SUPPLY => x.add_to_supply(v),
        value::DEMAND => x.add_to_demand(v),
        _ => false,
    })
}

/// `Write` (§12.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_node_write(n: *const fc4_demand_node, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_node(n, false, |x| x.write(&mut COut(o)))
}

/// `Read` (§12.2): versions 0-2.
#[no_mangle]
pub unsafe extern "C" fn fc4_demand_node_read(n: *mut fc4_demand_node, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_node(n, false, |x| {
        x.read(&mut CIn {
            stream: i,
            decode: decoder,
        })
    })
}
