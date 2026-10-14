// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The aura simulator's C ABI (fc4.h "aura", docs/specs/aura.md).
//!
//! The host implements [`fc4_aura_host`] (every `AuraHost` method of the
//! core, in the trait's order, with the grids of other simulators read
//! through two callbacks, and the aura's own int8 / int16 grids through the
//! shared [`fc4_grid_api`]); the plugin's `cISC4AuraSimulator` forwards its
//! slots to the `fc4_aura_*` functions.
//!
//! Tokens are opaque host identities (fc4.h): occupants (`cISC4Occupant`),
//! the lot iterator, grids. 0 is the null object.
//!
//! # Threads
//!
//! The game calls the getters (`GetMayorRating`, `GetMax/MinMayorRating`,
//! `GetAuraValue`, the four grid getters) from any thread, also while the
//! main thread is inside a pipeline step. The core keeps its state in
//! `RefCell`s, so those getters never reach it: after every main-thread call
//! the handle publishes a [`fc4_aura_ratings`] snapshot (the three ratings,
//! the two locations and the four grid tokens) behind a `Mutex`, and the
//! getters read only that ([`fc4_aura_ratings`], [`fc4_aura_grid`]).
//! `GetAuraValue` is the aura grid's own `GetCellValue` (§8.3), which the
//! plugin calls on the published grid.
//!
//! Re-entrancy (main thread): a message, transient effect or strike buster
//! delivered while a main-thread call is running is queued and handled when
//! the outermost call returns; the count is in
//! `fc4_aura_state::reentrant_calls`.

use std::ffi::{c_char, c_void, CString};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use fc4_core::aura::{AuraHost, AuraMessage, AuraRandom, AuraRng, AuraSim, IteratedLot, LotInfo, LotIterator, Strike};
use fc4_core::grid::{GridElem, GridProvider, TractGeometry, TractGrid};
use fc4_core::property::PropertyHolder;
use fc4_core::rng::FixedSeeds;
use fc4_core::stream::{RefValue, SaveObject};
use fc4_core::variant::{mem, Variant};

use crate::guard;
use crate::hostgrid::{create_grid, fc4_grid_api, grid_token, read_property, resolve_grid, take_props};
use crate::landvalue::fc4_grid_geometry;
use crate::props::fc4_props;
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

/// The grids of other simulators the pipeline reads (fc4.h
/// `FC4_AURA_INPUT_*`): the `which` of `input_grid` / `input_grid_get`.
pub const FC4_AURA_INPUT_AIR: u32 = 0;
pub const FC4_AURA_INPUT_WATER: u32 = 1;
pub const FC4_AURA_INPUT_GARBAGE: u32 = 2;
pub const FC4_AURA_INPUT_RADIATION: u32 = 3;
pub const FC4_AURA_INPUT_SCHOOL: u32 = 4;
pub const FC4_AURA_INPUT_HOSPITAL: u32 = 5;
pub const FC4_AURA_INPUT_CRIME: u32 = 6;
pub const FC4_AURA_INPUT_TRIP: u32 = 7;
pub const FC4_AURA_INPUT_CONGESTION: u32 = 8;
/// The three residential population grids (R$, R$$, R$$$): asked in this
/// order, once per state 15.
pub const FC4_AURA_INPUT_POP1: u32 = 9;
pub const FC4_AURA_INPUT_POP2: u32 = 10;
pub const FC4_AURA_INPUT_POP3: u32 = 11;
pub const FC4_AURA_INPUT_COUNT: u32 = 12;

/// The grid getters' `which` (fc4.h `FC4_AURA_GRID_*`).
pub const FC4_AURA_GRID_AURA: u32 = 0;
pub const FC4_AURA_GRID_TRANSIENT: u32 = 1;
pub const FC4_AURA_GRID_PARK: u32 = 2;
pub const FC4_AURA_GRID_LANDMARK: u32 = 3;

/// A lot as the host reads it (fc4.h `fc4_aura_lot`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_aura_lot {
    /// `GetBoundingRect`: inclusive cell rectangle.
    pub x1: i32,
    pub z1: i32,
    pub x2: i32,
    pub z2: i32,
    /// `GetZoneType()`.
    pub zone_type: u32,
    /// `GetOccupantWealth()`.
    pub occupant_wealth: i32,
    /// `GetBuilding()` as a `cISC4Occupant` token (borrowed); 0 = none.
    pub building: u64,
}

/// A (developer type, count) pair of `GetPopulations()`.
pub type fc4_aura_population_sink = Option<unsafe extern "C" fn(*mut c_void, u32, i32)>;

/// What any thread may read (fc4.h `fc4_aura_ratings`): published after
/// every main-thread call.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_aura_ratings {
    pub struct_size: u32,
    /// `GetMayorRating()` (the low 8 bits), `GetMaxMayorRating`,
    /// `GetMinMayorRating` (§8.3).
    pub mayor_rating: i8,
    pub max_rating: i8,
    pub min_rating: i8,
    pub pad: i8,
    pub max_x: i32,
    pub max_z: i32,
    pub min_x: i32,
    pub min_z: i32,
    pub reserved: [u32; 2],
    /// The four grid getters (borrowed tokens; 0 = none).
    pub aura_grid: u64,
    pub transient_grid: u64,
    pub park_map: u64,
    pub landmark_map: u64,
}

/// Diagnostics (fc4.h `fc4_aura_state`). Main thread.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_aura_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub re_add_landmarks: u8,
    pub has_lot_iterator: u8,
    pub pad0: u8,
    pub state: i32,
    pub cycle_start_day: u32,
    pub mayor_rating: i32,
    pub unused_rating: i32,
    pub global_effect: i32,
    pub max_rating: i16,
    pub min_rating: i16,
    pub max_x: i32,
    pub max_z: i32,
    pub min_x: i32,
    pub min_z: i32,
    pub effective_base_aura: [i32; 3],
    pub hotspots: u32,
    pub low_tracts: u32,
    pub departments: u32,
    pub riot_roll: f32,
    pub slice_step: u32,
    pub slice_miss: i32,
    /// Over the aura grid's tracts: count, sum, min, max (0 when no grid).
    pub aura_tracts: u32,
    pub aura_sum: i32,
    pub aura_min: i32,
    pub aura_max: i32,
    /// Base tracts with data (≠ −128).
    pub base_tracts: u32,
    pub reentrant_calls: u32,
    pub long_decay: f32,
    pub short_decay: f32,
    pub reserved: u32,
}

/// Everything the aura simulator asks of the game (fc4.h `fc4_aura_host`;
/// one callback per `AuraHost` method other than the aura's own grids, in
/// the trait's order).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_aura_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    /// The aura's own grids (int8 `0x49B9E603`, int16 `0x49B9E605`).
    pub grids: fc4_grid_api,
    // city and simulator
    pub cell_width: Option<unsafe extern "C" fn(U, *mut f32, *mut f32)>,
    pub position_to_cell: Option<unsafe extern "C" fn(U, f32, f32, *mut i32, *mut i32)>,
    pub sim_date_number: Option<unsafe extern "C" fn(U) -> u32>,
    /// A NEW fc4_props (the core destroys it), NULL if the exemplar is missing.
    pub tuning: Option<unsafe extern "C" fn(U) -> *mut fc4_props>,
    /// The three base auras (R$, R$$, R$$$).
    pub publish_base_aura: Option<unsafe extern "C" fn(U, *const i32)>,
    // lots
    pub lot_count: Option<unsafe extern "C" fn(U) -> u32>,
    /// A fresh lot iterator token (0 = none: no lots).
    pub lot_iterator: Option<unsafe extern "C" fn(U) -> u64>,
    /// The iterator's next lot and its population pairs; false at the end.
    pub lot_next:
        Option<unsafe extern "C" fn(U, u64, *mut fc4_aura_lot, fc4_aura_population_sink, *mut c_void) -> bool>,
    pub lot_iterator_release: Option<unsafe extern "C" fn(U, u64)>,
    /// `GetLot(x, z, false)`: false = no lot (`building` is not needed).
    pub lot_at: Option<unsafe extern "C" fn(U, i32, i32, *mut fc4_aura_lot) -> bool>,
    /// `GetLandValueType(x, z)`; negative = the city has no land value
    /// simulator (§6.5.1, AU10: the value/wealth term is skipped). A NULL
    /// callback reads as none.
    pub land_value_type: Option<unsafe extern "C" fn(U, i32, i32) -> i32>,
    // occupants
    pub occupant_has_property: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub occupant_property: Option<unsafe extern "C" fn(U, u64, u32, *mut fc4_props) -> bool>,
    pub occupant_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub occupant_position: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    pub budget_item_department: Option<unsafe extern "C" fn(U, u64, u32, *mut u32) -> bool>,
    // budget, ordinances
    pub department_funding: Option<unsafe extern "C" fn(U, u32, *mut f32) -> bool>,
    pub tax_rate: Option<unsafe extern "C" fn(U, i32) -> f32>,
    pub cumulative_offset: Option<unsafe extern "C" fn(U, u32, u32) -> i32>,
    // grids of other simulators (read only)
    /// Input `which` (`FC4_AURA_INPUT_*`) as a BORROWED grid token, read
    /// through `grids.geometry` / `grids.get` and valid until the outermost
    /// call into the core returns; 0 = not available.
    pub input_grid: Option<unsafe extern "C" fn(U, u32) -> u64>,
    // effects on the game
    pub record_history: Option<unsafe extern "C" fn(U, u32, i32)>,
    pub attempt_disaster_start: Option<unsafe extern "C" fn(U, u32, i32, i32)>,
    /// 0 police, 1 fire, 2 health, 3 school (§8.4).
    pub end_strike: Option<unsafe extern "C" fn(U, u32)>,
    pub post: Option<unsafe extern "C" fn(U, u32)>,
    // CPU-cycle limit timer
    pub limit_timer_start: Option<unsafe extern "C" fn(U, u32)>,
    pub limit_timer_expired: Option<unsafe extern "C" fn(U) -> bool>,
    // framework plumbing
    pub subscribe: Option<unsafe extern "C" fn(U, *const u32, u32)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, *const u32, u32)>,
    pub add_agent: Option<unsafe extern "C" fn(U, u32, *const c_char, u32)>,
    pub remove_agent: Option<unsafe extern "C" fn(U)>,
    pub registry_add: Option<unsafe extern "C" fn(U) -> bool>,
    pub registry_remove: Option<unsafe extern "C" fn(U)>,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_aura_lot>() == 32);
    assert!(std::mem::size_of::<fc4_aura_ratings>() == 64);
    assert!(std::mem::size_of::<fc4_aura_state>() == 120);
    assert!(std::mem::size_of::<fc4_aura_host>() == if cfg!(target_pointer_width = "32") { 176 } else { 352 });
};

// ---- host adapter -----------------------------------------------------------------------------------

/// The C host as the core's `AuraHost`.
pub(crate) struct CHost(pub(crate) fc4_aura_host);

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.0.$f { Some(f) => unsafe { f($s.0.user $(, $arg)*) }, None => $default }
    };
}

fn geometry_of(g: &fc4_grid_geometry) -> TractGeometry {
    TractGeometry {
        tract_size: g.tract_size,
        shift: g.shift,
        count_x: g.count_x,
        count_z: g.count_z,
        width_x: g.width_x,
        width_z: g.width_z,
        inv_width_x: g.inv_width_x,
        inv_width_z: g.inv_width_z,
    }
}

/// A grid of another simulator: a borrowed token read through the grid
/// table (no reference is held or released). [choice] Writes are ignored:
/// the aura never writes these grids.
struct InputGrid<T: GridElem> {
    api: fc4_grid_api,
    token: u64,
    geom: TractGeometry,
    _t: std::marker::PhantomData<T>,
}

impl<T: GridElem> InputGrid<T> {
    fn new(api: fc4_grid_api, token: u64) -> InputGrid<T> {
        let mut g = fc4_grid_geometry::default();
        if let Some(f) = api.geometry {
            // SAFETY: valid callback (fc4_aura_create's precondition); `g` outlives the call.
            unsafe { f(api.user, token, &mut g) };
        }
        InputGrid {
            api,
            token,
            geom: geometry_of(&g),
            _t: std::marker::PhantomData,
        }
    }
}

impl<T: GridElem> TractGrid<T> for InputGrid<T> {
    fn geometry(&self) -> TractGeometry {
        self.geom
    }
    fn get(&self, tx: i32, tz: i32) -> T {
        if !self.geom.in_bounds(tx, tz) {
            return T::default();
        }
        match self.api.get {
            // SAFETY: valid callback; the host keeps the grid alive for the call.
            Some(f) => T::from_f64(unsafe { f(self.api.user, self.token, T::CLSID, tx, tz) }),
            None => T::default(),
        }
    }
    fn set(&self, _: i32, _: i32, _: T) {}
    fn fill(&self, _: T) {}
    fn save_ref(&self) -> RefValue {
        RefValue::Null
    }
}

impl CHost {
    fn input<T: GridElem>(&self, which: u32) -> Option<Rc<dyn TractGrid<T>>> {
        let token = cb!(self, input_grid, 0, which);
        (token != 0).then(|| Rc::new(InputGrid::<T>::new(self.0.grids, token)) as Rc<dyn TractGrid<T>>)
    }

    fn lot_info(l: &fc4_aura_lot) -> LotInfo {
        LotInfo {
            x1: l.x1,
            z1: l.z1,
            x2: l.x2,
            z2: l.z2,
            zone_type: l.zone_type,
        }
    }
}

/// The host's lot iterator; dropping it releases the host's token.
struct HostLots {
    host: fc4_aura_host,
    token: u64,
}

unsafe extern "C" fn push_pair(ctx: *mut c_void, t: u32, n: i32) {
    if let Some(v) = (ctx as *mut Vec<(u32, i32)>).as_mut() {
        v.push((t, n));
    }
}

impl LotIterator for HostLots {
    fn next_lot(&mut self) -> Option<IteratedLot> {
        if self.token == 0 {
            return None;
        }
        let f = self.host.lot_next?;
        let mut l = fc4_aura_lot::default();
        let mut pops: Vec<(u32, i32)> = Vec::new();
        // SAFETY: valid callback; `l` and `pops` outlive the call.
        let more = unsafe {
            f(
                self.host.user,
                self.token,
                &mut l,
                Some(push_pair),
                &mut pops as *mut Vec<(u32, i32)> as *mut c_void,
            )
        };
        more.then(|| IteratedLot {
            info: CHost::lot_info(&l),
            building: (l.building != 0).then_some(l.building),
            occupant_wealth: l.occupant_wealth,
            populations: pops,
        })
    }
}

impl Drop for HostLots {
    fn drop(&mut self) {
        if self.token == 0 {
            return;
        }
        if let Some(f) = self.host.lot_iterator_release {
            // SAFETY: valid callback; the token came from lot_iterator.
            unsafe { f(self.host.user, self.token) };
        }
    }
}

impl GridProvider<i8> for CHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<i8>>> {
        create_grid::<i8>(&self.0.grids, instance, tract_size, true)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<i8>>> {
        resolve_grid::<i8>(&self.0.grids, r, true)
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

/// `RefSint32()` of a property (§7 "How the values are read"). [choice]
/// (AU4) Only a Sint32 or Uint32 **array** (a Uint32 one bit for bit, as the
/// original's raw read gives) of at least two values is read; anything else
/// (a scalar, one value, another element type) reads as absent: no cone.
/// The original reads the word at variant + 4 as an int32 array whatever
/// the type and count; the shipped properties are Sint32 arrays of 2.
pub(crate) fn sint32s(v: &Variant) -> Option<Vec<i32>> {
    match v {
        Variant::Array { elem, items } if (*elem == mem::SINT32 || *elem == mem::UINT32) && items.len() >= 2 => {
            Some(items.iter().map(|e| e.as_i64() as i32).collect())
        }
        _ => None,
    }
}

impl AuraHost for CHost {
    fn cell_width(&self) -> (f32, f32) {
        let (mut x, mut z) = (0.0f32, 0.0f32);
        cb!(self, cell_width, (), &mut x, &mut z);
        (x, z)
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        let (mut cx, mut cz) = (0, 0);
        cb!(self, position_to_cell, (), x, z, &mut cx, &mut cz);
        (cx, cz)
    }
    fn sim_date_number(&self) -> u32 {
        cb!(self, sim_date_number, 0)
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, tuning, std::ptr::null_mut());
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }
    fn publish_base_aura(&self, base_aura: [i32; 3]) {
        cb!(self, publish_base_aura, (), base_aura.as_ptr())
    }
    fn lot_count(&self) -> u32 {
        cb!(self, lot_count, 0)
    }
    fn lot_iterator(&self) -> Box<dyn LotIterator> {
        let token = cb!(self, lot_iterator, 0);
        Box::new(HostLots { host: self.0, token })
    }
    fn lot_at(&self, x: i32, z: i32) -> Option<LotInfo> {
        let mut l = fc4_aura_lot::default();
        cb!(self, lot_at, false, x, z, &mut l).then(|| CHost::lot_info(&l))
    }
    fn land_value_type(&self, x: i32, z: i32) -> Option<i32> {
        let t = cb!(self, land_value_type, -1, x, z);
        (t >= 0).then_some(t)
    }
    fn occupant_has_property(&self, occupant: u64, prop: u32) -> bool {
        cb!(self, occupant_has_property, false, occupant, prop)
    }
    fn occupant_sint32s(&self, occupant: u64, prop: u32) -> Option<Vec<i32>> {
        let f = self.0.occupant_property?;
        // SAFETY: valid callback; the scratch holder outlives the call.
        let v = read_property(prop, |p| unsafe { f(self.0.user, occupant, prop, p) })?;
        sint32s(&v)
    }
    fn occupant_type(&self, occupant: u64) -> u32 {
        cb!(self, occupant_type, 0, occupant)
    }
    fn occupant_position(&self, occupant: u64) -> [f32; 3] {
        let mut p = [0.0f32; 3];
        cb!(self, occupant_position, (), occupant, p.as_mut_ptr());
        p
    }
    fn budget_item_department(&self, occupant: u64, purpose: u32) -> Option<u32> {
        let mut d = 0u32;
        cb!(self, budget_item_department, false, occupant, purpose, &mut d).then_some(d)
    }
    fn department_funding(&self, dept: u32) -> Option<f32> {
        let mut f = 0.0f32;
        cb!(self, department_funding, false, dept, &mut f).then_some(f)
    }
    fn tax_rate(&self, index: i32) -> f32 {
        cb!(self, tax_rate, 0.0, index)
    }
    fn cumulative_offset(&self, prop: u32, index: u32) -> i32 {
        cb!(self, cumulative_offset, 0, prop, index)
    }
    fn pollution_grid(&self, kind: i32) -> Option<Rc<dyn TractGrid<i16>>> {
        if !(0..4).contains(&kind) {
            return None;
        }
        self.input::<i16>(FC4_AURA_INPUT_AIR + kind as u32)
    }
    fn school_grade_map(&self) -> Option<Rc<dyn TractGrid<f32>>> {
        self.input::<f32>(FC4_AURA_INPUT_SCHOOL)
    }
    fn hospital_grade_map(&self) -> Option<Rc<dyn TractGrid<f32>>> {
        self.input::<f32>(FC4_AURA_INPUT_HOSPITAL)
    }
    fn crime_grid(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        self.input::<u8>(FC4_AURA_INPUT_CRIME)
    }
    fn trip_length_map(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        self.input::<u8>(FC4_AURA_INPUT_TRIP)
    }
    fn congestion_map(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        self.input::<u8>(FC4_AURA_INPUT_CONGESTION)
    }
    fn population_grids(&self) -> Option<[Rc<dyn TractGrid<u16>>; 3]> {
        let a = self.input::<u16>(FC4_AURA_INPUT_POP1)?;
        let b = self.input::<u16>(FC4_AURA_INPUT_POP2)?;
        let c = self.input::<u16>(FC4_AURA_INPUT_POP3)?;
        Some([a, b, c])
    }
    fn record_history(&self, series: u32, value: i32) {
        cb!(self, record_history, (), series, value)
    }
    fn attempt_disaster_start(&self, manager: u32, x: i32, z: i32) {
        cb!(self, attempt_disaster_start, (), manager, x, z)
    }
    fn end_strike(&self, strike: Strike) {
        let k = match strike {
            Strike::Police => 0,
            Strike::Fire => 1,
            Strike::Health => 2,
            Strike::School => 3,
        };
        cb!(self, end_strike, (), k)
    }
    fn post(&self, msg_type: u32) {
        cb!(self, post, (), msg_type)
    }
    fn limit_timer_start(&self, ms: u32) {
        cb!(self, limit_timer_start, (), ms)
    }
    fn limit_timer_expired(&self) -> bool {
        // No callback: the budget never runs out (one call does the whole walk,
        // up to the core's cap).
        cb!(self, limit_timer_expired, false)
    }
    fn subscribe(&self, msgs: &[u32]) {
        cb!(self, subscribe, (), msgs.as_ptr(), msgs.len() as u32)
    }
    fn unsubscribe(&self, msgs: &[u32]) {
        cb!(self, unsubscribe, (), msgs.as_ptr(), msgs.len() as u32)
    }
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        let c = CString::new(name).unwrap_or_default();
        cb!(self, add_agent, (), level, c.as_ptr(), flags)
    }
    fn remove_agent(&self) {
        cb!(self, remove_agent, ())
    }
    fn registry_add(&self) -> bool {
        cb!(self, registry_add, false)
    }
    fn registry_remove(&self) {
        cb!(self, registry_remove, ())
    }
}

// ---- handles ----------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_aura`).
#[repr(C)]
pub struct fc4_aura {
    _private: [u8; 0],
}

/// A main-thread call that arrived while another one was running.
#[derive(Clone, Copy, Debug)]
enum Deferred {
    Message(AuraMessage),
    TransientType(f32, f32, u32),
    Transient(f32, f32, f32, f32),
    BustStrike(u32),
}

/// The handle. `depth`, `reentrant` and `queue` belong to the main thread's
/// calls; a getter on another thread only reaches `snap`.
struct Handle {
    sim: Rc<AuraSim>,
    depth: AtomicU32,
    reentrant: AtomicU32,
    queue: Mutex<Vec<Deferred>>,
    snap: Mutex<fc4_aura_ratings>,
}

/// Decrements the call depth when the call ends (also on a panic).
struct Depth<'a>(&'a AtomicU32);

impl Drop for Depth<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

fn ratings_of(sim: &AuraSim) -> fc4_aura_ratings {
    let (max, (max_x, max_z)) = sim.max_mayor_rating();
    let (min, (min_x, min_z)) = sim.min_mayor_rating();
    fc4_aura_ratings {
        struct_size: std::mem::size_of::<fc4_aura_ratings>() as u32,
        mayor_rating: sim.mayor_rating(),
        max_rating: max,
        min_rating: min,
        pad: 0,
        max_x,
        max_z,
        min_x,
        min_z,
        reserved: [0; 2],
        aura_grid: grid_token(&sim.aura_grid()),
        transient_grid: grid_token(&sim.transient_grid()),
        park_map: grid_token(&sim.park_map()),
        landmark_map: grid_token(&sim.landmark_map()),
    }
}

impl Handle {
    fn publish(&self) {
        let r = ratings_of(&self.sim);
        *self.snap.lock().unwrap_or_else(|e| e.into_inner()) = r;
    }

    fn run(&self, d: Deferred) {
        match d {
            Deferred::Message(m) => {
                self.sim.do_message(&m);
            }
            Deferred::TransientType(x, z, k) => self.sim.add_transient_effect_type(x, z, k),
            Deferred::Transient(x, z, r, m) => self.sim.add_transient_effect(x, z, r, m),
            Deferred::BustStrike(k) => self.sim.bust_strike(k),
        }
    }

    /// Runs `f` with the call depth counted (main thread); at the end of the
    /// outermost call, the queued calls run and the snapshot is published.
    fn call<R>(&self, f: impl FnOnce(&AuraSim) -> R) -> R {
        if self.depth.load(Ordering::Relaxed) > 0 {
            self.reentrant.fetch_add(1, Ordering::Relaxed);
        }
        self.depth.fetch_add(1, Ordering::Relaxed);
        let _depth = Depth(&self.depth);
        let r = f(&self.sim);
        if self.depth.load(Ordering::Relaxed) == 1 {
            loop {
                let next = {
                    let mut q = self.queue.lock().unwrap_or_else(|e| e.into_inner());
                    if q.is_empty() {
                        None
                    } else {
                        Some(q.remove(0))
                    }
                };
                let Some(d) = next else { break };
                self.run(d);
            }
            self.publish();
        }
        r
    }

    /// A main-thread call that changes state: queued while another call
    /// runs, else run now.
    fn defer_or_run(&self, d: Deferred) {
        if self.depth.load(Ordering::Relaxed) > 0 {
            self.reentrant.fetch_add(1, Ordering::Relaxed);
            self.queue.lock().unwrap_or_else(|e| e.into_inner()).push(d);
            return;
        }
        self.call(|_| self.run(d));
    }
}

fn handle<'a>(p: *const fc4_aura) -> Option<&'a Handle> {
    // SAFETY: documented precondition: NULL or a live fc4_aura*.
    unsafe { (p as *const Handle).as_ref() }
}

/// A main-thread call (lifecycle, messages, persistence, effects).
fn sim<R>(p: *const fc4_aura, fallback: R, f: impl FnOnce(&AuraSim) -> R) -> R {
    match handle(p) {
        Some(h) => guard(fallback, || h.call(f)),
        None => fallback,
    }
}

/// Creates the core object (constructor values, §3). The host table is
/// copied. `token` is the host identity of the simulator itself; the two
/// clock values seed its `cRZRandom` (§9: seeded from the wall clock at
/// construction).
#[no_mangle]
pub unsafe extern "C" fn fc4_aura_create(
    host: *const fc4_aura_host,
    token: u64,
    wall_clock_seed: u32,
    process_clock: u32,
) -> *mut fc4_aura {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_aura_host>() && h.grids.complete() => {
            let host = Rc::new(CHost(*h));
            let seeds = FixedSeeds {
                wall_clock: wall_clock_seed,
                process_clock,
            };
            let rng: Box<dyn AuraRandom> = Box::new(AuraRng::new(&seeds));
            let sim = AuraSim::new(host, token, rng);
            let snap = Mutex::new(ratings_of(&sim));
            Box::into_raw(Box::new(Handle {
                sim,
                depth: AtomicU32::new(0),
                reentrant: AtomicU32::new(0),
                queue: Mutex::new(Vec::new()),
                snap,
            })) as *mut fc4_aura
        }
        _ => std::ptr::null_mut(),
    })
}

/// Destroys the core object. Its grids (and a held lot iterator) are
/// released by their handles' drops (the grids are shut down first).
#[no_mangle]
pub unsafe extern "C" fn fc4_aura_destroy(p: *mut fc4_aura) {
    if p.is_null() {
        return;
    }
    guard((), || drop(Box::from_raw(p as *mut Handle)));
}

/// `Init` (§4.1): always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_aura_init(p: *mut fc4_aura) -> bool {
    sim(p, false, AuraSim::init)
}

/// `Shutdown` (§4.2): always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_aura_shutdown(p: *mut fc4_aura) -> bool {
    sim(p, false, AuraSim::shutdown)
}

/// `DoMessage` (§5): true except for a late SimNewDay. `void1` is the
/// occupant token for the occupant messages. A message delivered during
/// another main-thread call is queued (true).
#[no_mangle]
pub unsafe extern "C" fn fc4_aura_do_message(
    p: *mut fc4_aura,
    msg_type: u32,
    data1: u32,
    data2: u32,
    data3: u32,
    void1: u64,
) -> bool {
    let m = AuraMessage {
        msg_type,
        data1,
        data2,
        data3,
        void1,
    };
    match handle(p) {
        Some(h) => guard(true, || {
            if h.depth.load(Ordering::Relaxed) > 0 {
                h.reentrant.fetch_add(1, Ordering::Relaxed);
                h.queue
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .push(Deferred::Message(m));
                return true;
            }
            h.call(|s| s.do_message(&m))
        }),
        None => true,
    }
}

/// Slot 6 `AddTransientEffect(x, z, type)` (§8.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_aura_add_transient_effect_type(p: *mut fc4_aura, x: f32, z: f32, kind: u32) {
    if let Some(h) = handle(p) {
        guard((), || h.defer_or_run(Deferred::TransientType(x, z, kind)));
    }
}

/// Slot 7 `AddTransientEffect(x, z, radius, magnitude)` (§8.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_aura_add_transient_effect(p: *mut fc4_aura, x: f32, z: f32, radius: f32, magnitude: f32) {
    if let Some(h) = handle(p) {
        guard((), || h.defer_or_run(Deferred::Transient(x, z, radius, magnitude)));
    }
}

/// Slot 11 `BustStrike(type)` (§8.4).
#[no_mangle]
pub unsafe extern "C" fn fc4_aura_bust_strike(p: *mut fc4_aura, kind: u32) {
    if let Some(h) = handle(p) {
        guard((), || h.defer_or_run(Deferred::BustStrike(kind)));
    }
}

/// Slots 8-10 and the grid getters' tokens: the snapshot published after
/// the last main-thread call. Any thread. False (and `out` untouched) for
/// NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_aura_get_ratings(p: *const fc4_aura, out: *mut fc4_aura_ratings) -> bool {
    let (Some(h), Some(o)) = (handle(p), out.as_mut()) else {
        return false;
    };
    guard(false, || {
        *o = *h.snap.lock().unwrap_or_else(|e| e.into_inner());
        true
    })
}

/// Slots 12-15: the grid's token (`FC4_AURA_GRID_*`; borrowed, 0 = none),
/// from the snapshot. Any thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_aura_grid(p: *const fc4_aura, which: u32) -> u64 {
    let Some(h) = handle(p) else { return 0 };
    guard(0, || {
        let s = *h.snap.lock().unwrap_or_else(|e| e.into_inner());
        match which {
            FC4_AURA_GRID_AURA => s.aura_grid,
            FC4_AURA_GRID_TRANSIENT => s.transient_grid,
            FC4_AURA_GRID_PARK => s.park_map,
            FC4_AURA_GRID_LANDMARK => s.landmark_map,
            _ => 0,
        }
    })
}

/// Over a grid's tracts: (count, sum, min, max); zeros without a grid.
fn grid_stats(g: &Option<Rc<dyn TractGrid<i8>>>) -> (u32, i32, i32, i32) {
    let Some(g) = g else { return (0, 0, 0, 0) };
    let geo = g.geometry();
    let (nx, nz) = (geo.count_x.clamp(0, 4096), geo.count_z.clamp(0, 4096));
    let (mut n, mut sum, mut lo, mut hi) = (0u32, 0i32, i32::MAX, i32::MIN);
    for x in 0..nx {
        for z in 0..nz {
            let v = i32::from(g.get(x, z));
            n += 1;
            sum = sum.wrapping_add(v);
            lo = lo.min(v);
            hi = hi.max(v);
        }
    }
    if n == 0 {
        (0, 0, 0, 0)
    } else {
        (n, sum, lo, hi)
    }
}

/// Diagnostics (main thread; reads the grids).
#[no_mangle]
pub unsafe extern "C" fn fc4_aura_get_state(p: *const fc4_aura, out: *mut fc4_aura_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    let Some(h) = handle(p) else { return false };
    guard(false, || {
        let s = &h.sim;
        let (aura_tracts, aura_sum, aura_min, aura_max) = grid_stats(&s.aura_grid());
        let base_tracts = s.base_grid().map_or(0, |b| {
            let geo = b.geometry();
            let mut n = 0u32;
            for x in 0..geo.count_x.clamp(0, 4096) {
                for z in 0..geo.count_z.clamp(0, 4096) {
                    if b.get(x, z) != -128 {
                        n += 1;
                    }
                }
            }
            n
        });
        let d = s.data();
        let slice = s.slice_step();
        *o = fc4_aura_state {
            struct_size: std::mem::size_of::<fc4_aura_state>() as u32,
            initialized: u8::from(d.initialized),
            re_add_landmarks: u8::from(d.re_add_landmarks),
            has_lot_iterator: u8::from(s.has_lot_iterator()),
            pad0: 0,
            state: d.state,
            cycle_start_day: d.cycle_start_day,
            mayor_rating: d.mayor_rating,
            unused_rating: d.unused_rating,
            global_effect: d.global_effect,
            max_rating: d.max_rating,
            min_rating: d.min_rating,
            max_x: d.max_location.0,
            max_z: d.max_location.1,
            min_x: d.min_location.0,
            min_z: d.min_location.1,
            effective_base_aura: d.effective_base_aura,
            hotspots: d.hotspots.len() as u32,
            low_tracts: d.low_tracts.len() as u32,
            departments: s.department_ids().len() as u32,
            riot_roll: d.riot_roll,
            slice_step: slice.step,
            slice_miss: slice.misses,
            aura_tracts,
            aura_sum,
            aura_min,
            aura_max,
            base_tracts,
            reentrant_calls: h.reentrant.load(Ordering::Relaxed),
            long_decay: d.tuning.long_decay,
            short_decay: d.tuning.short_decay,
            reserved: 0,
        };
        true
    })
}

/// `Write` (§11.2). The grid references go out as `fc4_ref`s with the token
/// and the grid CLSID.
#[no_mangle]
pub unsafe extern "C" fn fc4_aura_write(p: *const fc4_aura, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    sim(p, false, |s| s.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

/// `Read` (§11.3): false for a version below 8 or above 9, or a stream
/// error.
#[no_mangle]
pub unsafe extern "C" fn fc4_aura_read(p: *mut fc4_aura, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    sim(p, false, |s| {
        s.read(&mut CIn {
            stream: i,
            decode: no_core_objects,
        })
    })
}
