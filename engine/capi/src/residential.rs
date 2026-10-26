// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The residential simulator's C ABI (fc4.h "residential",
//! docs/specs/residential.md and docs/specs/residential-education-health.md).
//!
//! The host implements [`fc4_residential_host`] (one callback per
//! `ResidentialHost` / `EducationHealthHost` method of the core, in the
//! traits' order, plus `hand_out`); the residential grids are the game's own
//! `cSC4SimGrid<T>` through the shared [`fc4_grid_api`] (uint8 `0x49B9E602`,
//! uint16 `0x49B9E604`, float `0x49B9E60A`: residential.md §11.3, the aura,
//! crime and growth developer read them raw). The building map and the growth
//! developers' population maps are BORROWED tokens read through the same
//! table. The plugin's `cISC4ResidentialSimulator` forwards its slots to the
//! `fc4_residential_*` functions.
//!
//! Tokens are opaque host identities: occupants (`cISC4Occupant`),
//! departments (`cISC4DepartmentBudget`), line items, visual effects, grids.
//! 0 is the null object.
//!
//! # Threads
//!
//! The core keeps its state in `RefCell`s and is main-thread only (module
//! docs of `fc4_core::residential`). The game calls the getters from any
//! thread, also while the main thread is inside a day. So:
//!
//! - after every outermost main-thread call the handle publishes a
//!   [`fc4_residential_snapshot`] (flags, counts and grid tokens) and a
//!   *detached* copy of the state (the grids replaced by their tokens) behind
//!   locks;
//! - a getter on the main thread with no call running uses the core
//!   directly; anywhere else (another thread, or a call from inside a host
//!   callback) it runs on a **view**: a throw-away core built from the
//!   detached copy, its grids borrowed views of the same game grids (writes
//!   ignored), its host the same callback table. A view never touches the
//!   main core's `RefCell`s;
//! - a call that changes state (a message, a strike end, a funding or
//!   auto-budget setter) made anywhere but the main thread with no call
//!   running is queued, with its occupant held, and handled when the next
//!   outermost main-thread call returns (`fc4_residential_state`'s
//!   `reentrant_calls` / `deferred_calls`).

use std::cell::Cell;
use std::ffi::{c_char, c_void, CString};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use std::thread::ThreadId;

use fc4_core::grid::{GridElem, GridProvider, TractGeometry, TractGrid, MAX_TRACTS};
use fc4_core::property::PropertyHolder;
use fc4_core::residential::{
    BudgetItem, CreatedEffect, EducationHealthHost, Grids, QueryData, ResidentialHost, ResidentialMessage,
    ResidentialRandom, ResidentialRng, ResidentialSim, ResidentialState,
};
use fc4_core::rng::FixedSeeds;
use fc4_core::stream::{RefValue, SaveObject};
use fc4_core::variant::Variant;

use crate::budget::fc4_token_callback;
use crate::demand::fc4_u32_callback;
use crate::hostgrid::{create_grid, fc4_grid_api, grid_token, read_property, resolve_grid, take_props};
use crate::landvalue::fc4_grid_geometry;
use crate::props::fc4_props;
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};
use crate::{guard, slice_arg};

type U = *mut c_void;

/// `which` of [`fc4_residential_buildings`] (slots 47, 49, 61).
pub const FC4_RESIDENTIAL_LIST_SCHOOL: u32 = 0;
pub const FC4_RESIDENTIAL_LIST_HOSPITAL: u32 = 1;
pub const FC4_RESIDENTIAL_LIST_HEALTH_AFFECTING: u32 = 2;

/// `GetBudgetItemForPurpose`'s item (fc4.h `fc4_residential_budget_item`,
/// budget.md §2).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_residential_budget_item {
    pub department: u32,
    /// 0 = "use the building type".
    pub line: u32,
    pub purpose: u32,
    pub index: u32,
    pub cost: i64,
    /// A fraction (1.0 = 100 %).
    pub local_funding: f32,
    pub reserved: u32,
}

/// The query record of slots 14, 15, 26 and 27 (fc4.h
/// `fc4_residential_query_data`; REH §4.11): the fields after the game
/// record's leading `cRZString`, its +0x14 .. +0x38 in order, the two costs
/// widened to 64 bits.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_residential_query_data {
    pub capacity: u32,
    pub effective_capacity: u32,
    pub capacity_per_staff: u32,
    pub effective_per_staff: u32,
    pub served: u32,
    pub grade: f32,
    pub staff_funding: f32,
    pub reserved0: u32,
    pub staff_cost: i64,
    pub vehicle_funding: f32,
    pub reserved1: u32,
    pub vehicle_cost: i64,
}

impl From<&fc4_residential_query_data> for QueryData {
    fn from(q: &fc4_residential_query_data) -> QueryData {
        QueryData {
            capacity: q.capacity,
            effective_capacity: q.effective_capacity,
            capacity_per_staff: q.capacity_per_staff,
            effective_per_staff: q.effective_per_staff,
            served: q.served,
            grade: q.grade,
            staff_funding: q.staff_funding,
            staff_cost: q.staff_cost,
            vehicle_funding: q.vehicle_funding,
            vehicle_cost: q.vehicle_cost,
        }
    }
}

impl From<&QueryData> for fc4_residential_query_data {
    fn from(q: &QueryData) -> fc4_residential_query_data {
        fc4_residential_query_data {
            capacity: q.capacity,
            effective_capacity: q.effective_capacity,
            capacity_per_staff: q.capacity_per_staff,
            effective_per_staff: q.effective_per_staff,
            served: q.served,
            grade: q.grade,
            staff_funding: q.staff_funding,
            reserved0: 0,
            staff_cost: q.staff_cost,
            vehicle_funding: q.vehicle_funding,
            reserved1: 0,
            vehicle_cost: q.vehicle_cost,
        }
    }
}

/// Slots 43 / 44 (fc4.h `fc4_residential_min_max`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_residential_min_max {
    pub min_x: u32,
    pub min_z: u32,
    pub max_x: u32,
    pub max_z: u32,
    pub min: f32,
    pub max: f32,
}

/// What any thread may read (fc4.h `fc4_residential_snapshot`): published
/// after every outermost main-thread call. Grid tokens are borrowed (0 =
/// none).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_residential_snapshot {
    pub struct_size: u32,
    pub initialized: u8,
    /// Slots 6 / 7.
    pub school_strike: u8,
    pub health_strike: u8,
    /// Slots 52 / 54.
    pub school_auto_budget: u8,
    pub hospital_auto_budget: u8,
    pub pad: [u8; 3],
    /// The daily state machine (§5.1).
    pub state: u32,
    /// Slot 16.
    pub population: i32,
    /// Slots 46 / 48.
    pub school_building_count: i32,
    pub hospital_building_count: i32,
    pub population_size: i32,
    pub eq_size: i32,
    pub reserved: u32,
    /// Slot 5 (R$, R$$, R$$$).
    pub proximity: [u64; 3],
    /// Slot 25.
    pub population_grids: [u64; 3],
    /// Slot 41.
    pub age: u64,
    /// Slots 39 / 40.
    pub school_grade: u64,
    pub hospital_grade: u64,
    /// Slots 23 / 24.
    pub eq: [u64; 3],
    pub hq: [u64; 3],
}

/// Diagnostics (fc4.h `fc4_residential_state`). Main thread.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_residential_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub city_started: u8,
    pub dirty: u8,
    pub tuning_loaded: u8,
    pub school_strike: u8,
    pub health_strike: u8,
    pub school_auto_budget: u8,
    pub hospital_auto_budget: u8,
    pub state: u32,
    pub population: i32,
    pub running_population: i32,
    /// Σ over each population grid's tracts (R$, R$$, R$$$).
    pub population_by_wealth: [u32; 3],
    /// Slot 34.
    pub global_le: u32,
    pub le_sum: u64,
    /// Slots 32 / 33.
    pub global_eq: f32,
    pub global_hq: f32,
    /// The coverage counters of the last coverage states (REH §4.4).
    pub school_covered: i32,
    pub hospital_covered: i32,
    /// Slots 46 / 48.
    pub school_buildings: i32,
    pub hospital_buildings: i32,
    pub pending_school: u32,
    pub pending_hospital: u32,
    pub health_affecting: u32,
    pub other: u32,
    pub population_size: i32,
    pub eq_size: i32,
    /// The population R$ grid's tract counts.
    pub tracts_x: i32,
    pub tracts_z: i32,
    /// State 4's scan position.
    pub iterator_x: i32,
    pub iterator_z: i32,
    pub school_strike_start: i32,
    pub health_strike_start: i32,
    /// Departments held (of 4) and grids held (of 25).
    pub departments: u32,
    pub grids: u32,
    pub reentrant_calls: u32,
    pub deferred_calls: u32,
    pub view_calls: u32,
    pub reserved: u32,
}

/// Everything the residential simulator asks of the game (fc4.h
/// `fc4_residential_host`): one callback per `ResidentialHost` /
/// `EducationHealthHost` method other than the grids, in the traits' order.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_residential_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    /// The residential grids, and reads of the borrowed building map and
    /// population maps.
    pub grids: fc4_grid_api,
    // lifecycle (§4.1, §4.2)
    pub acquire_services: Option<unsafe extern "C" fn(U)>,
    pub acquire_developers: Option<unsafe extern "C" fn(U)>,
    pub acquire_demand_node: Option<unsafe extern "C" fn(U) -> bool>,
    pub acquire_message_server: Option<unsafe extern "C" fn(U) -> bool>,
    pub acquire_budget_simulator: Option<unsafe extern "C" fn(U) -> bool>,
    pub has_simulator: Option<unsafe extern "C" fn(U) -> bool>,
    pub add_agent: Option<unsafe extern "C" fn(U, u32, *const c_char, u32)>,
    pub remove_agent: Option<unsafe extern "C" fn(U)>,
    pub subscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub release_message_server_and_budget: Option<unsafe extern "C" fn(U)>,
    pub release_services: Option<unsafe extern "C" fn(U)>,
    pub set_history_queue_length: Option<unsafe extern "C" fn(U, u32, u32)>,
    /// `CreateDepartmentBudget`, `AddRef`, name, max and initial funding (and
    /// the secondary departments' local line item): a token holding one
    /// reference, 0 = none.
    pub create_department: Option<unsafe extern "C" fn(U, u32, u32, f32, f32) -> u64>,
    pub release_department: Option<unsafe extern "C" fn(U, u64)>,
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    /// `AddRef` for a reference handed to the game (slots 21, 22, 47, 49,
    /// 61). NULL = `add_ref`.
    pub hand_out: Option<unsafe extern "C" fn(U, u64)>,
    // resources (§7)
    /// A NEW fc4_props (the core destroys it), NULL if the exemplar is missing.
    pub tuning: Option<unsafe extern "C" fn(U) -> *mut fc4_props>,
    // the monthly pass's inputs (§5.3, §5.5)
    /// BDS slot 41: a BORROWED `cISC4SimGrid<uint8>` token, 0 = none.
    pub building_map: Option<unsafe extern "C" fn(U) -> u64>,
    pub building_map_key: Option<unsafe extern "C" fn(U, u32, u32) -> u8>,
    /// Growth developer slot 15 of the developer type: a BORROWED
    /// `cISC4SimGrid<int16>` token, 0 = none.
    pub developer_population_map: Option<unsafe extern "C" fn(U, u32) -> u64>,
    pub pollution_value: Option<unsafe extern "C" fn(U, u32, i32, i32) -> u16>,
    pub sim_month: Option<unsafe extern "C" fn(U) -> u32>,
    pub sim_day_number: Option<unsafe extern "C" fn(U) -> u32>,
    pub altitude: Option<unsafe extern "C" fn(U, f32, f32) -> f32>,
    // visual effects (§5.5 step 2, RG1): the effect (one reference), 0 = the
    // create failed, FC4_EFFECT_NOT_SHOWN = the host cannot show effects;
    // NULL = cannot show effects.
    pub create_visual_effect: Option<unsafe extern "C" fn(U, *const c_char) -> u64>,
    pub set_effect_transform: Option<unsafe extern "C" fn(U, u64, *const f32)>,
    pub start_and_release_effect: Option<unsafe extern "C" fn(U, u64)>,
    // messages (§5.7, §8, REH §5)
    pub record_history: Option<unsafe extern "C" fn(U, u32, i32)>,
    pub post_strike_message: Option<unsafe extern "C" fn(U, u32, u64, u32, u32)>,
    // occupants (§4.3, §6.6)
    pub occupant_as_building: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub is_occupant_group: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub occupant_bool_property: Option<unsafe extern "C" fn(U, u64, u32, *mut bool) -> bool>,
    pub set_occupant_bool_property: Option<unsafe extern "C" fn(U, u64, u32, bool) -> bool>,
    /// A reference read with `iid` (one reference handed over): the token
    /// the core keeps, 0 = refused.
    pub resolve_ref: Option<unsafe extern "C" fn(U, u64, u32) -> u64>,
    // education and health: budget (REH §4.1, §4.2)
    pub budget_item: Option<unsafe extern "C" fn(U, u64, u32, *mut fc4_residential_budget_item) -> bool>,
    pub change_budget_item_line: Option<unsafe extern "C" fn(U, u64, u32, u32)>,
    /// `GetDepartmentBudget(id)`: borrowed, 0 = none.
    pub department_budget: Option<unsafe extern "C" fn(U, u32) -> u64>,
    /// `*GetFundingPercentage()`; NaN = cannot be read (RG10: the output is
    /// left alone).
    pub department_funding: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub is_locally_funded: Option<unsafe extern "C" fn(U, u64, u64, u32) -> bool>,
    pub add_locally_funded: Option<unsafe extern "C" fn(U, u64, u64, u32)>,
    pub remove_locally_funded: Option<unsafe extern "C" fn(U, u64, u64, u32)>,
    pub set_local_funding: Option<unsafe extern "C" fn(U, u64, u64, f32, u32)>,
    /// Borrowed, 0 = none.
    pub line_item: Option<unsafe extern "C" fn(U, u64, u32) -> u64>,
    pub create_line_item_for_building_type: Option<unsafe extern "C" fn(U, u64, u32, bool) -> u64>,
    pub remove_line_item: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub line_item_count: Option<unsafe extern "C" fn(U, u64) -> i64>,
    pub set_line_item_count: Option<unsafe extern "C" fn(U, u64, i64)>,
    pub set_line_item_display_flag: Option<unsafe extern "C" fn(U, u64, u32, bool)>,
    // education and health: occupants (REH §2, §4.1, §4.4-§4.6)
    pub occupant_property: Option<unsafe extern "C" fn(U, u64, u32, *mut fc4_props) -> bool>,
    pub set_occupant_u32: Option<unsafe extern "C" fn(U, u64, u32, u32)>,
    pub set_occupant_f32: Option<unsafe extern "C" fn(U, u64, u32, f32)>,
    pub building_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub building_slot11: Option<unsafe extern "C" fn(U, u64) -> u32>,
    /// (x, z) of the occupant's lot centre; false = no lot.
    pub occupant_lot_center: Option<unsafe extern "C" fn(U, u64, *mut f32) -> bool>,
    /// (x, z) of `GetPosition`.
    pub occupant_position: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    // ordinances (REH §4.3)
    pub ordinances_on: Option<unsafe extern "C" fn(U, fc4_u32_callback, *mut c_void)>,
    /// A NEW fc4_props of the ordinance's misc properties, NULL = skip it.
    pub ordinance_properties: Option<unsafe extern "C" fn(U, u32) -> *mut fc4_props>,
    // automata (REH §4.9)
    pub add_controllers: Option<unsafe extern "C" fn(U, *const c_char, u64)>,
    pub remove_controllers: Option<unsafe extern "C" fn(U, *const c_char, u64)>,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_residential_budget_item>() == 32);
    assert!(std::mem::size_of::<fc4_residential_query_data>() == 56);
    assert!(std::mem::size_of::<fc4_residential_min_max>() == 24);
    assert!(std::mem::size_of::<fc4_residential_snapshot>() == 160);
    assert!(std::mem::size_of::<fc4_residential_state>() == 144);
    assert!(std::mem::size_of::<fc4_residential_host>() == if cfg!(target_pointer_width = "32") { 292 } else { 584 });
};

// ---- host adapter -----------------------------------------------------------------------------------

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.t.$f { Some(f) => unsafe { f($s.t.user $(, $arg)*) }, None => $default }
    };
}

/// The C host as the core's `ResidentialHost`. `handing_out` routes the
/// core's `add_ref` to `hand_out` while a getter collects references for the
/// game (slots 47, 49, 61).
pub(crate) struct CHost {
    pub(crate) t: fc4_residential_host,
    handing_out: Cell<bool>,
}

impl CHost {
    pub(crate) fn new(t: fc4_residential_host) -> CHost {
        CHost {
            t,
            handing_out: Cell::new(false),
        }
    }

    /// An `AddRef` for a reference the game will own.
    pub(crate) fn hand_out(&self, token: u64) {
        if token == 0 {
            return;
        }
        match self.t.hand_out {
            // SAFETY: valid callback (fc4_residential_create's precondition).
            Some(f) => unsafe { f(self.t.user, token) },
            None => cb!(self, add_ref, (), token),
        }
    }

    fn shutdown_grid(&self, r: RefValue) {
        if let RefValue::Host { token, .. } = r {
            if token != 0 {
                if let Some(f) = self.t.grids.shutdown {
                    // SAFETY: valid callback; the token is a grid we hold.
                    unsafe { f(self.t.grids.user, token) };
                }
            }
        }
    }
}

/// While alive, the host's `add_ref` is a hand-out.
pub(crate) struct HandingOut<'a>(&'a CHost);

impl<'a> HandingOut<'a> {
    pub(crate) fn new(h: &'a CHost) -> HandingOut<'a> {
        h.handing_out.set(true);
        HandingOut(h)
    }
}

impl Drop for HandingOut<'_> {
    fn drop(&mut self) {
        self.0.handing_out.set(false);
    }
}

fn opt(t: u64) -> Option<u64> {
    (t != 0).then_some(t)
}

/// `create_visual_effect`'s "the host cannot show effects" (fc4.h, RG1):
/// treated as created ([`CreatedEffect::NotShown`]).
pub const FC4_EFFECT_NOT_SHOWN: u64 = u64::MAX;

/// A `create_visual_effect` result as the core sees it (RG1).
pub(crate) fn effect_of(t: u64) -> CreatedEffect {
    match t {
        0 => CreatedEffect::Failed,
        FC4_EFFECT_NOT_SHOWN => CreatedEffect::NotShown,
        e => CreatedEffect::Effect(e),
    }
}

/// A game grid read through the grid table without holding a reference:
/// another simulator's grid (the building map, a population map) or, in a
/// view, one of the residential grids the main core holds. Writes are
/// ignored. The geometry is asked until it is non-empty, then kept
/// (`HostGrid`'s rule).
pub(crate) struct BorrowedGrid<T: GridElem> {
    api: fc4_grid_api,
    token: u64,
    geom: Cell<Option<TractGeometry>>,
    _t: PhantomData<T>,
}

impl<T: GridElem> BorrowedGrid<T> {
    pub(crate) fn wrap(api: fc4_grid_api, token: u64) -> Option<Rc<dyn TractGrid<T>>> {
        (token != 0).then(|| {
            Rc::new(BorrowedGrid::<T> {
                api,
                token,
                geom: Cell::new(None),
                _t: PhantomData,
            }) as Rc<dyn TractGrid<T>>
        })
    }
}

impl<T: GridElem> TractGrid<T> for BorrowedGrid<T> {
    fn geometry(&self) -> TractGeometry {
        if let Some(g) = self.geom.get() {
            return g;
        }
        let mut g = fc4_grid_geometry::default();
        if let Some(f) = self.api.geometry {
            // SAFETY: valid callback; `g` outlives the call.
            unsafe { f(self.api.user, self.token, &mut g) };
        }
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
        if geom.count_x > 0 && geom.count_z > 0 {
            self.geom.set(Some(geom));
        }
        geom
    }
    fn get(&self, tx: i32, tz: i32) -> T {
        if !self.geometry().in_bounds(tx, tz) {
            return T::default();
        }
        match self.api.get {
            // SAFETY: valid callback; the owner keeps the grid alive for the call.
            Some(f) => T::from_f64(unsafe { f(self.api.user, self.token, T::CLSID, tx, tz) }),
            None => T::default(),
        }
    }
    fn set(&self, _: i32, _: i32, _: T) {}
    fn fill(&self, _: T) {}
    fn save_ref(&self) -> RefValue {
        RefValue::Host {
            token: self.token,
            class_id: T::CLSID,
        }
    }
}

// [choice] The grids the core creates are not shut down when their handle
// drops: the core calls `shutdown_grid_*` itself at `Shutdown` (§4.2), and
// the temporary grid is only released (§5.3).
impl GridProvider<u8> for CHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        create_grid::<u8>(&self.t.grids, instance, tract_size, false)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u8>>> {
        resolve_grid::<u8>(&self.t.grids, r, false)
    }
}

impl GridProvider<u16> for CHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u16>>> {
        create_grid::<u16>(&self.t.grids, instance, tract_size, false)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u16>>> {
        resolve_grid::<u16>(&self.t.grids, r, false)
    }
}

impl GridProvider<f32> for CHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<f32>>> {
        create_grid::<f32>(&self.t.grids, instance, tract_size, false)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<f32>>> {
        resolve_grid::<f32>(&self.t.grids, r, false)
    }
}

unsafe extern "C" fn push_u32(ctx: *mut c_void, v: u32) {
    if let Some(out) = (ctx as *mut Vec<u32>).as_mut() {
        out.push(v);
    }
}

fn c_name(name: &str) -> CString {
    CString::new(name).unwrap_or_default()
}

impl ResidentialHost for CHost {
    fn invocation_precision(&self) -> fc4_core::x87::Precision {
        crate::invocation::precision()
    }

    fn acquire_services(&self) {
        cb!(self, acquire_services, ())
    }
    fn acquire_developers(&self) {
        cb!(self, acquire_developers, ())
    }
    fn acquire_demand_node(&self) -> bool {
        cb!(self, acquire_demand_node, false)
    }
    fn acquire_message_server(&self) -> bool {
        cb!(self, acquire_message_server, false)
    }
    fn acquire_budget_simulator(&self) -> bool {
        cb!(self, acquire_budget_simulator, false)
    }
    fn has_simulator(&self) -> bool {
        cb!(self, has_simulator, false)
    }
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        let c = c_name(name);
        cb!(self, add_agent, (), level, c.as_ptr(), flags)
    }
    fn remove_agent(&self) {
        cb!(self, remove_agent, ())
    }
    fn subscribe(&self, msg_type: u32) {
        cb!(self, subscribe, (), msg_type)
    }
    fn unsubscribe(&self, msg_type: u32) {
        cb!(self, unsubscribe, (), msg_type)
    }
    fn release_message_server_and_budget(&self) {
        cb!(self, release_message_server_and_budget, ())
    }
    fn release_services(&self) {
        cb!(self, release_services, ())
    }
    fn set_history_queue_length(&self, series: u32, length: u32) {
        cb!(self, set_history_queue_length, (), series, length)
    }
    fn create_department(&self, id: u32, name: u32, max_funding: f32, init_funding: f32) -> Option<u64> {
        opt(cb!(self, create_department, 0, id, name, max_funding, init_funding))
    }
    fn release_department(&self, department: u64) {
        if department != 0 {
            cb!(self, release_department, (), department)
        }
    }
    fn shutdown_grid_u8(&self, grid: &Rc<dyn TractGrid<u8>>) {
        self.shutdown_grid(grid.save_ref())
    }
    fn shutdown_grid_u16(&self, grid: &Rc<dyn TractGrid<u16>>) {
        self.shutdown_grid(grid.save_ref())
    }
    fn shutdown_grid_f32(&self, grid: &Rc<dyn TractGrid<f32>>) {
        self.shutdown_grid(grid.save_ref())
    }
    fn add_ref(&self, token: u64) {
        if token == 0 {
            return;
        }
        if self.handing_out.get() {
            self.hand_out(token);
        } else {
            cb!(self, add_ref, (), token)
        }
    }
    fn release(&self, token: u64) {
        if token != 0 {
            cb!(self, release, (), token)
        }
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, tuning, std::ptr::null_mut());
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }
    fn building_map(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        BorrowedGrid::<u8>::wrap(self.t.grids, cb!(self, building_map, 0))
    }
    fn building_map_key(&self, purpose: u32, wealth: u32) -> u8 {
        cb!(self, building_map_key, 0, purpose, wealth)
    }
    fn developer_population_map(&self, dev_type: u32) -> Option<Rc<dyn TractGrid<i16>>> {
        BorrowedGrid::<i16>::wrap(self.t.grids, cb!(self, developer_population_map, 0, dev_type))
    }
    fn pollution_value(&self, kind: u32, cell_x: i32, cell_z: i32) -> u16 {
        cb!(self, pollution_value, 0, kind, cell_x, cell_z)
    }
    fn sim_month(&self) -> u32 {
        cb!(self, sim_month, 0)
    }
    fn sim_day_number(&self) -> u32 {
        cb!(self, sim_day_number, 0)
    }
    fn altitude(&self, x: f32, z: f32) -> f32 {
        cb!(self, altitude, 0.0, x, z)
    }
    fn create_visual_effect(&self, name: &str) -> CreatedEffect {
        let c = c_name(name);
        effect_of(cb!(self, create_visual_effect, FC4_EFFECT_NOT_SHOWN, c.as_ptr()))
    }
    fn set_effect_transform(&self, effect: u64, translation: [f32; 3]) {
        cb!(self, set_effect_transform, (), effect, translation.as_ptr())
    }
    fn start_and_release_effect(&self, effect: u64) {
        cb!(self, start_and_release_effect, (), effect)
    }
    fn record_history(&self, series: u32, value: i32) {
        cb!(self, record_history, (), series, value)
    }
    fn post_strike_message(&self, msg_type: u32, occupant: Option<u64>, data2: u32, data3: u32) {
        cb!(
            self,
            post_strike_message,
            (),
            msg_type,
            occupant.unwrap_or(0),
            data2,
            data3
        )
    }
    fn occupant_as_building(&self, occupant: u64) -> Option<u64> {
        opt(cb!(self, occupant_as_building, 0, occupant))
    }
    fn is_occupant_group(&self, occupant: u64, group: u32) -> bool {
        cb!(self, is_occupant_group, false, occupant, group)
    }
    fn occupant_bool_property(&self, occupant: u64, id: u32) -> Option<bool> {
        let mut v = false;
        cb!(self, occupant_bool_property, false, occupant, id, &mut v).then_some(v)
    }
    fn set_occupant_bool_property(&self, occupant: u64, id: u32, value: bool) -> bool {
        cb!(self, set_occupant_bool_property, false, occupant, id, value)
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

impl EducationHealthHost for CHost {
    fn budget_item(&self, occupant: u64, purpose: u32) -> Option<BudgetItem> {
        let mut i = fc4_residential_budget_item {
            purpose,
            ..Default::default()
        };
        cb!(self, budget_item, false, occupant, purpose, &mut i).then_some(BudgetItem {
            department: i.department,
            line: i.line,
            index: i.index,
            cost: i.cost,
            local_funding: i.local_funding,
        })
    }
    fn change_budget_item_line(&self, occupant: u64, index: u32, line: u32) {
        cb!(self, change_budget_item_line, (), occupant, index, line)
    }
    fn department_budget(&self, department_id: u32) -> Option<u64> {
        opt(cb!(self, department_budget, 0, department_id))
    }
    fn department_funding(&self, department: u64) -> Option<f32> {
        // RG10 (REH §4.2): no callback, or a NaN from it, is "missing" (the
        // output keeps the caller's value), never 1.0.
        let f = cb!(self, department_funding, f32::NAN, department);
        (!f.is_nan()).then_some(f)
    }
    fn is_locally_funded(&self, department: u64, occupant: u64, purpose: u32) -> bool {
        cb!(self, is_locally_funded, false, department, occupant, purpose)
    }
    fn add_locally_funded(&self, department: u64, occupant: u64, purpose: u32) {
        cb!(self, add_locally_funded, (), department, occupant, purpose)
    }
    fn remove_locally_funded(&self, department: u64, occupant: u64, purpose: u32) {
        cb!(self, remove_locally_funded, (), department, occupant, purpose)
    }
    fn set_local_funding(&self, department: u64, occupant: u64, fraction: f32, purpose: u32) {
        cb!(self, set_local_funding, (), department, occupant, fraction, purpose)
    }
    fn line_item(&self, department: u64, line: u32) -> Option<u64> {
        opt(cb!(self, line_item, 0, department, line))
    }
    fn create_line_item_for_building_type(&self, department: u64, line: u32, local: bool) -> Option<u64> {
        opt(cb!(
            self,
            create_line_item_for_building_type,
            0,
            department,
            line,
            local
        ))
    }
    fn remove_line_item(&self, department: u64, line: u32) {
        cb!(self, remove_line_item, (), department, line)
    }
    fn line_item_count(&self, line_item: u64) -> i64 {
        cb!(self, line_item_count, 0, line_item)
    }
    fn set_line_item_count(&self, line_item: u64, count: i64) {
        cb!(self, set_line_item_count, (), line_item, count)
    }
    fn set_line_item_display_flag(&self, line_item: u64, flag: u32, on: bool) {
        cb!(self, set_line_item_display_flag, (), line_item, flag, on)
    }
    fn occupant_property(&self, occupant: u64, id: u32) -> Option<Variant> {
        let f = self.t.occupant_property?;
        // SAFETY: valid callback; the scratch holder outlives the call.
        read_property(id, |p| unsafe { f(self.t.user, occupant, id, p) })
    }
    fn set_occupant_u32(&self, occupant: u64, id: u32, value: u32) {
        cb!(self, set_occupant_u32, (), occupant, id, value)
    }
    fn set_occupant_f32(&self, occupant: u64, id: u32, value: f32) {
        cb!(self, set_occupant_f32, (), occupant, id, value)
    }
    fn building_type(&self, building: u64) -> u32 {
        cb!(self, building_type, 0, building)
    }
    fn building_slot11(&self, building: u64) -> u32 {
        cb!(self, building_slot11, 0, building)
    }
    fn occupant_lot_center(&self, occupant: u64) -> Option<(f32, f32)> {
        let mut xz = [0.0f32; 2];
        cb!(self, occupant_lot_center, false, occupant, xz.as_mut_ptr()).then_some((xz[0], xz[1]))
    }
    fn occupant_position(&self, occupant: u64) -> (f32, f32) {
        let mut xz = [0.0f32; 2];
        cb!(self, occupant_position, (), occupant, xz.as_mut_ptr());
        (xz[0], xz[1])
    }
    fn ordinances_on(&self) -> Vec<u32> {
        let mut v: Vec<u32> = Vec::new();
        cb!(
            self,
            ordinances_on,
            (),
            Some(push_u32),
            &mut v as *mut Vec<u32> as *mut c_void
        );
        v
    }
    fn ordinance_properties(&self, id: u32) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, ordinance_properties, std::ptr::null_mut(), id);
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }
    fn add_controllers(&self, name: &str, occupant: u64) {
        let c = c_name(name);
        cb!(self, add_controllers, (), c.as_ptr(), occupant)
    }
    fn remove_controllers(&self, name: &str, occupant: u64) {
        let c = c_name(name);
        cb!(self, remove_controllers, (), c.as_ptr(), occupant)
    }
}

// ---- detached state and views ---------------------------------------------------------------------

/// The grid tokens of a state (0 = none).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct GridTokens {
    pub proximity: [u64; 3],
    pub population: [u64; 3],
    pub age: u64,
    pub school_coverage: [u64; 7],
    pub hospital_coverage: [u64; 3],
    pub school_grade: u64,
    pub hospital_grade: u64,
    pub eq: [u64; 3],
    pub hq: [u64; 3],
}

impl GridTokens {
    pub(crate) fn of(g: &Grids) -> GridTokens {
        GridTokens {
            proximity: std::array::from_fn(|i| grid_token(&g.proximity[i])),
            population: std::array::from_fn(|i| grid_token(&g.population[i])),
            age: grid_token(&g.age),
            school_coverage: std::array::from_fn(|i| grid_token(&g.school_coverage[i])),
            hospital_coverage: std::array::from_fn(|i| grid_token(&g.hospital_coverage[i])),
            school_grade: grid_token(&g.school_grade),
            hospital_grade: grid_token(&g.hospital_grade),
            eq: std::array::from_fn(|i| grid_token(&g.eq[i])),
            hq: std::array::from_fn(|i| grid_token(&g.hq[i])),
        }
    }

    /// Borrowed views of the same game grids (no temporary grid).
    pub(crate) fn views(&self, api: &fc4_grid_api) -> Grids {
        let u8v = |t: u64| BorrowedGrid::<u8>::wrap(*api, t);
        let f32v = |t: u64| BorrowedGrid::<f32>::wrap(*api, t);
        Grids {
            proximity: self.proximity.map(u8v),
            population: self.population.map(|t| BorrowedGrid::<u16>::wrap(*api, t)),
            age: u8v(self.age),
            school_coverage: self.school_coverage.map(u8v),
            hospital_coverage: self.hospital_coverage.map(u8v),
            school_grade: f32v(self.school_grade),
            hospital_grade: f32v(self.hospital_grade),
            eq: self.eq.map(f32v),
            hq: self.hq.map(f32v),
            temporary: None,
        }
    }

    fn count(&self) -> u32 {
        let all = self
            .proximity
            .iter()
            .chain(&self.population)
            .chain(std::iter::once(&self.age))
            .chain(&self.school_coverage)
            .chain(&self.hospital_coverage)
            .chain([&self.school_grade, &self.hospital_grade])
            .chain(&self.eq)
            .chain(&self.hq);
        all.filter(|&&t| t != 0).count() as u32
    }
}

/// The state as of the last outermost main-thread call, its grids replaced by
/// their tokens.
#[derive(Clone)]
struct Detached {
    state: ResidentialState,
    tokens: GridTokens,
}

// SAFETY: `state.grids` is always `Grids::default()` (no `Rc`); every other
// field of `ResidentialState` is plain owned data (numbers, `Vec`s, curves),
// so the value can be cloned and dropped on any thread.
unsafe impl Send for Detached {}

impl Detached {
    fn of(mut state: ResidentialState) -> Detached {
        let tokens = GridTokens::of(&state.grids);
        state.grids = Grids::default();
        Detached { state, tokens }
    }
}

fn list_count(groups: &[Vec<Option<u64>>], pending: &[Option<u64>]) -> i32 {
    groups.iter().map(Vec::len).sum::<usize>().wrapping_add(pending.len()) as i32
}

fn snapshot_of(s: &ResidentialState, t: &GridTokens) -> fc4_residential_snapshot {
    fc4_residential_snapshot {
        struct_size: std::mem::size_of::<fc4_residential_snapshot>() as u32,
        initialized: u8::from(s.initialized),
        school_strike: u8::from(s.education.strike_flag != 0),
        health_strike: u8::from(s.health.strike_flag != 0),
        school_auto_budget: u8::from(s.school_auto_budget),
        hospital_auto_budget: u8::from(s.hospital_auto_budget),
        pad: [0; 3],
        state: s.state,
        population: s.population,
        school_building_count: list_count(&s.lists.school_groups, &s.lists.pending_school),
        hospital_building_count: list_count(&s.lists.hospital_groups, &s.lists.pending_hospital),
        population_size: s.population_size,
        eq_size: s.eq_size,
        reserved: 0,
        proximity: t.proximity,
        population_grids: t.population,
        age: t.age,
        school_grade: t.school_grade,
        hospital_grade: t.hospital_grade,
        eq: t.eq,
        hq: t.hq,
    }
}

/// A fixed generator for views (the getters draw nothing).
fn view_rng() -> Box<dyn ResidentialRandom> {
    let seeds = FixedSeeds {
        wall_clock: 1,
        process_clock: 1,
    };
    Box::new(ResidentialRng::from_session(1, &seeds))
}

// ---- handles ----------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_residential`).
#[repr(C)]
pub struct fc4_residential {
    _private: [u8; 0],
}

/// A call that changes state, made where the core cannot be reached. The
/// occupant tokens are held (`add_ref`) until it runs.
#[derive(Clone, Debug)]
enum Deferred {
    Message(ResidentialMessage),
    EndStrike(bool),
    SetFunding {
        occ: u64,
        staff: f32,
        vehicle: f32,
        school: bool,
    },
    GlobalAutoBudget {
        school: bool,
        on: bool,
    },
}

/// The handle. The core and `host` belong to the main thread; another thread
/// reaches only `snap`, `detached`, `queue` and the counters.
struct Handle {
    sim: Rc<ResidentialSim>,
    host: Rc<CHost>,
    table: fc4_residential_host,
    token: u64,
    main: ThreadId,
    depth: AtomicU32,
    reentrant: AtomicU32,
    deferred: AtomicU32,
    views: AtomicU32,
    queue: Mutex<Vec<Deferred>>,
    snap: Mutex<fc4_residential_snapshot>,
    detached: Mutex<Option<Detached>>,
}

/// Decrements the call depth when the call ends (also on a panic).
struct Depth<'a>(&'a AtomicU32);

impl<'a> Depth<'a> {
    fn enter(d: &'a AtomicU32) -> Depth<'a> {
        d.fetch_add(1, Ordering::Relaxed);
        Depth(d)
    }
}

impl Drop for Depth<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

impl Handle {
    fn on_main(&self) -> bool {
        std::thread::current().id() == self.main
    }

    /// The main thread with no call into the core running.
    fn idle_main(&self) -> bool {
        self.on_main() && self.depth.load(Ordering::Relaxed) == 0
    }

    fn publish(&self) {
        let st = self.sim.snapshot();
        let tokens = GridTokens::of(&st.grids);
        let snap = snapshot_of(&st, &tokens);
        let d = Detached::of(st);
        *self.snap.lock().unwrap_or_else(|e| e.into_inner()) = snap;
        *self.detached.lock().unwrap_or_else(|e| e.into_inner()) = Some(d);
    }

    fn run(&self, d: Deferred) {
        match d {
            Deferred::Message(m) => {
                self.sim.do_message(&m);
                if let Some(v) = m.void1 {
                    self.host.release(v);
                }
            }
            Deferred::EndStrike(school) => {
                if school {
                    self.sim.end_school_strike();
                } else {
                    self.sim.end_health_strike();
                }
            }
            Deferred::SetFunding {
                occ,
                staff,
                vehicle,
                school,
            } => {
                self.sim.set_occupant_funding(occ, staff, vehicle, school);
                self.host.release(occ);
            }
            Deferred::GlobalAutoBudget { school, on } => {
                if school {
                    self.sim.set_global_auto_budget_schools(on);
                } else {
                    self.sim.set_global_auto_budget_hospitals(on);
                }
            }
        }
    }

    fn drain(&self) {
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
    }

    /// A main-thread call (the caller checked `idle_main` or is the core's
    /// own lifecycle): at the end of the outermost call the queued calls run
    /// and the snapshot is published.
    fn call<R>(&self, f: impl FnOnce(&ResidentialSim) -> R) -> R {
        let _depth = Depth::enter(&self.depth);
        let r = f(&self.sim);
        if self.depth.load(Ordering::Relaxed) == 1 {
            self.drain();
            self.publish();
        }
        r
    }

    /// Queues a state change; its occupant is held until it runs.
    fn defer(&self, d: Deferred) {
        if self.on_main() {
            self.reentrant.fetch_add(1, Ordering::Relaxed);
        }
        self.deferred.fetch_add(1, Ordering::Relaxed);
        match &d {
            Deferred::Message(ResidentialMessage { void1: Some(v), .. }) => self.host.add_ref(*v),
            Deferred::SetFunding { occ, .. } => self.host.add_ref(*occ),
            _ => {}
        }
        self.queue.lock().unwrap_or_else(|e| e.into_inner()).push(d);
    }

    /// A state change: now on an idle main thread, else queued.
    fn mutate(&self, d: Deferred) {
        if self.idle_main() {
            self.call(|_| self.run(d));
        } else {
            self.defer(d);
        }
    }

    /// A throw-away core over the detached state (None before the first
    /// publish).
    fn view(&self) -> Option<(Rc<ResidentialSim>, Rc<CHost>)> {
        self.views.fetch_add(1, Ordering::Relaxed);
        let d = self.detached.lock().unwrap_or_else(|e| e.into_inner()).clone()?;
        let host = Rc::new(CHost::new(self.table));
        let grids = d.tokens.views(&self.table.grids);
        let sim = ResidentialSim::new(host.clone(), self.token, view_rng());
        let Detached { state, .. } = d;
        sim.with_state(move |s| {
            *s = state;
            s.grids = grids;
        });
        Some((sim, host))
    }

    /// A read: the core on an idle main thread, else a view. None only when
    /// no view can be made.
    fn query<R>(&self, f: impl FnOnce(&ResidentialSim, &CHost) -> R) -> Option<R> {
        if self.idle_main() {
            let r = {
                let _depth = Depth::enter(&self.depth);
                f(&self.sim, &self.host)
            };
            // A state change queued from inside (a host callback) runs now.
            let pending = !self.queue.lock().unwrap_or_else(|e| e.into_inner()).is_empty();
            if pending {
                self.call(|_| ());
            }
            return Some(r);
        }
        let (sim, host) = self.view()?;
        Some(f(&sim, &host))
    }
}

fn handle<'a>(p: *const fc4_residential) -> Option<&'a Handle> {
    // SAFETY: documented precondition: NULL or a live fc4_residential*.
    unsafe { (p as *const Handle).as_ref() }
}

/// A main-thread lifecycle call; refused (fallback) anywhere else.
fn lifecycle<R>(p: *const fc4_residential, fallback: R, f: impl FnOnce(&ResidentialSim) -> R) -> R {
    match handle(p) {
        Some(h) if h.idle_main() => guard(fallback, || h.call(f)),
        Some(h) => {
            h.reentrant.fetch_add(1, Ordering::Relaxed);
            fallback
        }
        None => fallback,
    }
}

/// A read (any thread).
fn query<R>(p: *const fc4_residential, fallback: R, f: impl FnOnce(&ResidentialSim, &CHost) -> R) -> R {
    match handle(p) {
        Some(h) => guard(None, || h.query(f)).unwrap_or(fallback),
        None => fallback,
    }
}

/// Creates the core object (constructor values, §3). The host table is
/// copied; `token` is the simulator's own identity. The class's `cRZRandom`
/// is seeded with `derive_seed(session_seed, 0x4990C013)` (foundations §6.3).
/// The calling thread becomes the main thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_create(
    host: *const fc4_residential_host,
    token: u64,
    session_seed: u32,
) -> *mut fc4_residential {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_residential_host>() && h.grids.complete() => {
            let chost = Rc::new(CHost::new(*h));
            let seeds = FixedSeeds {
                wall_clock: session_seed,
                process_clock: session_seed,
            };
            let rng: Box<dyn ResidentialRandom> = Box::new(ResidentialRng::from_session(session_seed, &seeds));
            let sim = ResidentialSim::new(chost.clone(), token, rng);
            let handle = Handle {
                sim,
                host: chost,
                table: *h,
                token,
                main: std::thread::current().id(),
                depth: AtomicU32::new(0),
                reentrant: AtomicU32::new(0),
                deferred: AtomicU32::new(0),
                views: AtomicU32::new(0),
                queue: Mutex::new(Vec::new()),
                snap: Mutex::new(fc4_residential_snapshot::default()),
                detached: Mutex::new(None),
            };
            handle.publish();
            Box::into_raw(Box::new(handle)) as *mut fc4_residential
        }
        _ => std::ptr::null_mut(),
    })
}

/// Destroys the core object. Queued calls are dropped (their occupants
/// released); the grids still held are released (`Shutdown` shuts them down
/// first).
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_destroy(p: *mut fc4_residential) {
    if p.is_null() {
        return;
    }
    guard((), || {
        let h = Box::from_raw(p as *mut Handle);
        let queued = std::mem::take(&mut *h.queue.lock().unwrap_or_else(|e| e.into_inner()));
        for d in queued {
            match d {
                Deferred::Message(ResidentialMessage { void1: Some(v), .. }) => h.host.release(v),
                Deferred::SetFunding { occ, .. } => h.host.release(occ),
                _ => {}
            }
        }
        drop(h);
    });
}

/// `Init` (§4.1). False without the demand node, the message server or the
/// simulator, or off the main thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_init(p: *mut fc4_residential) -> bool {
    lifecycle(p, false, ResidentialSim::init)
}

/// `Shutdown` (§4.2): true for a live object on the main thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_shutdown(p: *mut fc4_residential) -> bool {
    lifecycle(p, false, ResidentialSim::shutdown)
}

/// `DoMessage` (§4.3): the step's "finished" value (false only when a
/// time-sliced NewDay state ran out of budget). `void1` is the occupant token
/// of the occupant messages. A message delivered off the main thread or
/// during another call is queued (true).
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_do_message(
    p: *mut fc4_residential,
    msg_type: u32,
    data1: u32,
    data2: u32,
    data3: u32,
    void1: u64,
) -> bool {
    let m = ResidentialMessage {
        msg_type,
        data1,
        data2,
        data3,
        void1: opt(void1),
    };
    match handle(p) {
        Some(h) => guard(true, || {
            if h.idle_main() {
                h.call(|s| s.do_message(&m))
            } else {
                h.defer(Deferred::Message(m));
                true
            }
        }),
        None => true,
    }
}

/// Sets the work budget that replaces the 5 ms slicer (§5.2 [choice];
/// default unbounded). Main thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_set_work_budget(p: *mut fc4_residential, units: u32) {
    lifecycle(p, (), |s| s.set_work_budget(units))
}

/// The snapshot (any thread). False (and `out` untouched) for NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_get_snapshot(
    p: *const fc4_residential,
    out: *mut fc4_residential_snapshot,
) -> bool {
    let (Some(h), Some(o)) = (handle(p), out.as_mut()) else {
        return false;
    };
    guard(false, || {
        *o = *h.snap.lock().unwrap_or_else(|e| e.into_inner());
        true
    })
}

/// Σ over a grid's tracts (0 without a grid).
fn grid_sum<T: GridElem>(g: &Option<Rc<dyn TractGrid<T>>>) -> u32 {
    let Some(g) = g else { return 0 };
    let geo = g.geometry();
    let mut sum = 0u32;
    for x in 0..geo.count_x.clamp(0, MAX_TRACTS) {
        for z in 0..geo.count_z.clamp(0, MAX_TRACTS) {
            sum = sum.wrapping_add(g.get(x, z).sum_term() as u32);
        }
    }
    sum
}

/// Diagnostics (main thread with no call running; reads the grids and the
/// tuning). False anywhere else.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_get_state(p: *const fc4_residential, out: *mut fc4_residential_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    let Some(h) = handle(p) else { return false };
    if !h.idle_main() {
        return false;
    }
    guard(false, || {
        let _depth = Depth::enter(&h.depth);
        let s = &h.sim;
        let st = s.snapshot();
        let tokens = GridTokens::of(&st.grids);
        let (tracts_x, tracts_z) = st.grids.population[0]
            .as_ref()
            .map_or((0, 0), |g| (g.count_x(), g.count_z()));
        let departments = [
            st.health.department1,
            st.health.department2,
            st.education.department1,
            st.education.department2,
        ]
        .iter()
        .filter(|d| d.is_some())
        .count() as u32;
        *o = fc4_residential_state {
            struct_size: std::mem::size_of::<fc4_residential_state>() as u32,
            initialized: u8::from(st.initialized),
            city_started: u8::from(st.city_started),
            dirty: u8::from(st.dirty),
            tuning_loaded: u8::from(st.tuning_loaded),
            school_strike: u8::from(st.education.strike_flag != 0),
            health_strike: u8::from(st.health.strike_flag != 0),
            school_auto_budget: u8::from(st.school_auto_budget),
            hospital_auto_budget: u8::from(st.hospital_auto_budget),
            state: st.state,
            population: st.population,
            running_population: st.running_population,
            population_by_wealth: std::array::from_fn(|w| grid_sum(&st.grids.population[w])),
            global_le: u32::from(s.get_global_le()),
            le_sum: st.le_sum,
            global_eq: s.get_global_eq(),
            global_hq: s.get_global_hq(),
            school_covered: st.school_covered,
            hospital_covered: st.hospital_covered,
            school_buildings: list_count(&st.lists.school_groups, &st.lists.pending_school),
            hospital_buildings: list_count(&st.lists.hospital_groups, &st.lists.pending_hospital),
            pending_school: st.lists.pending_school.len() as u32,
            pending_hospital: st.lists.pending_hospital.len() as u32,
            health_affecting: st.lists.health_affecting.len() as u32,
            other: st.lists.other.len() as u32,
            population_size: st.population_size,
            eq_size: st.eq_size,
            tracts_x,
            tracts_z,
            iterator_x: st.iterator.x,
            iterator_z: st.iterator.z,
            school_strike_start: st.education.strike_start,
            health_strike_start: st.health.strike_start,
            departments,
            grids: tokens.count(),
            reentrant_calls: h.reentrant.load(Ordering::Relaxed),
            deferred_calls: h.deferred.load(Ordering::Relaxed),
            view_calls: h.views.load(Ordering::Relaxed),
            reserved: 0,
        };
        true
    })
}

/// `Write` (§9.2): the version-7 record; grid references go out as
/// `fc4_ref`s with the token and the grid CLSID, occupants and departments
/// with their token and class 0. Off the main thread (or during a call) the
/// detached state is written.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_write(p: *const fc4_residential, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    let Some(h) = handle(p) else { return false };
    guard(false, || {
        if h.idle_main() {
            h.call(|s| s.write(&mut COut(o)))
        } else {
            h.view().is_some_and(|(s, _)| s.write(&mut COut(o)))
        }
    })
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

/// `Read` (§9.2): false on a stream error or a damaged record (the
/// simulator is then unchanged), or off the main thread. Runs before `Init`.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_read(p: *mut fc4_residential, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    lifecycle(p, false, |s| {
        s.read(&mut CIn {
            stream: i,
            decode: no_core_objects,
        })
    })
}

// ---- slots ------------------------------------------------------------------------------------------

/// Slots 8 / 9 `EndSchoolStrike` / `EndHealthStrike`: whether a strike was
/// ended. [choice] Off the main thread (or during a call) the end is queued
/// and the answer is the published strike flag. RG9 (answered: the choice
/// stands): the original applies the call at once on the caller's thread
/// and returns AL = the city strike flag before the call, which is what the
/// published flag is.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_end_strike(p: *mut fc4_residential, school: bool) -> bool {
    let Some(h) = handle(p) else { return false };
    guard(false, || {
        if h.idle_main() {
            return h.call(|s| {
                if school {
                    s.end_school_strike()
                } else {
                    s.end_health_strike()
                }
            });
        }
        let snap = *h.snap.lock().unwrap_or_else(|e| e.into_inner());
        let on = if school {
            snap.school_strike != 0
        } else {
            snap.health_strike != 0
        };
        h.defer(Deferred::EndStrike(school));
        on
    })
}

/// Slots 10 / 11 (the hidden-pointer result's value).
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_strike_chance(p: *const fc4_residential, school: bool) -> f32 {
    query(p, 0.0, |s, _| {
        if school {
            s.chance_of_school_strike()
        } else {
            s.chance_of_health_strike()
        }
    })
}

/// Slots 12 / 13 (the hidden-pointer result's value).
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_system_rating(p: *const fc4_residential, school: bool) -> f32 {
    query(p, 0.0, |s, _| s.system_rating(school))
}

/// Slots 14 / 15 (REH §4.11, EH12): resets `totals` +0x14..+0x28 (in/out;
/// the rest untouched), adds the query fields of every building that has one
/// of `groups`, whatever its query returned, and returns their number.
/// `groups` NULL reads as no group ([choice]: the original crashes).
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_system_totals(
    p: *const fc4_residential,
    school: bool,
    groups: *const u32,
    group_count: u32,
    totals: *mut fc4_residential_query_data,
) -> u32 {
    let Some(t) = totals.as_mut() else { return 0 };
    let groups: Vec<u32> = slice_arg(groups, group_count as usize).unwrap_or(&[]).to_vec();
    let mut q = QueryData::from(&*t);
    let n = query(p, 0, |s, _| s.system_totals(school, &groups, &mut q));
    *t = fc4_residential_query_data::from(&q);
    n
}

/// Slot 19 `SetOccupantFundingPercentages`: true (§10.1 replacement value).
/// Queued off the main thread or during a call.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_set_occupant_funding(
    p: *mut fc4_residential,
    occupant: u64,
    staff: f32,
    vehicle: f32,
    school: bool,
) -> bool {
    if let Some(h) = handle(p) {
        guard((), || {
            h.mutate(Deferred::SetFunding {
                occ: occupant,
                staff,
                vehicle,
                school,
            })
        });
    }
    true
}

/// Slot 20 `GetOccupantFundingPercentages`: `staff` / `vehicle` in/out (a
/// path may leave them unchanged); true.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_get_occupant_funding(
    p: *const fc4_residential,
    occupant: u64,
    staff: *mut f32,
    vehicle: *mut f32,
    school: bool,
) -> bool {
    let (Some(a), Some(b)) = (staff.as_mut(), vehicle.as_mut()) else {
        return false;
    };
    let (mut x, mut y) = (*a, *b);
    query(p, (), |s, _| s.get_occupant_funding(occupant, &mut x, &mut y, school));
    *a = x;
    *b = y;
    true
}

/// Slots 21 / 22 `GetAverageEQGrid` / `GetAverageHQGrid`: a NEW float grid
/// per call (instance `0x69D5C351` / `0x69D5C360`), handed to the caller with
/// one reference (`hand_out`, then our reference released); 0 when none could
/// be made. `min` / `max` get the range (255 / 0 without a grid). RG8
/// (answered: the choice stands): the original builds the float grid directly
/// (global heap, inlined constructor), not through GZCOM; its callers are UI
/// code on the main thread [inferred], so nothing constrains a call from
/// another thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_average_grid(
    p: *const fc4_residential,
    is_eq: bool,
    min: *mut f32,
    max: *mut f32,
) -> u64 {
    let (token, lo, hi) = query(p, (0, 255.0, 0.0), |s, host| {
        let (g, lo, hi) = s.average_quotient_grid(is_eq);
        let token = grid_token(&g);
        host.hand_out(token);
        drop(g);
        (token, lo, hi)
    });
    if let Some(m) = min.as_mut() {
        *m = lo;
    }
    if let Some(m) = max.as_mut() {
        *m = hi;
    }
    token
}

/// Slots 26 / 27: the query record (in/out, written field by field); false
/// at the first missing field, with the fields read before it written
/// (EH10).
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_get_query_data(
    p: *const fc4_residential,
    school: bool,
    occupant: u64,
    data: *mut fc4_residential_query_data,
) -> bool {
    let Some(d) = data.as_mut() else { return false };
    let mut q = QueryData::from(&*d);
    let ok = query(p, false, |s, _| s.query_data_into(school, occupant, &mut q));
    *d = fc4_residential_query_data::from(&q);
    ok
}

/// Slot 28 `EstimateCurrentOccupantCapacity`: true (§10.1 replacement value).
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_estimate_capacity(
    p: *const fc4_residential,
    occupant: u64,
    capacity: *mut u32,
    staff: *mut u32,
) -> bool {
    let (c, st) = query(p, (0, 0), |s, _| s.estimate_occupant_capacity(occupant));
    if let Some(o) = capacity.as_mut() {
        *o = c;
    }
    if let Some(o) = staff.as_mut() {
        *o = st;
    }
    true
}

/// Slot 29 `GetCellLifeExpectancy` (AL).
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_cell_life_expectancy(p: *const fc4_residential, x: i32, z: i32) -> u8 {
    query(p, 0, |s, _| s.get_cell_life_expectancy(x, z))
}

/// Slot 30 `GetCellWorkforcePercent` (the hidden-pointer result's value).
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_cell_workforce(p: *const fc4_residential, x: i32, z: i32) -> f32 {
    query(p, 0.0, |s, _| s.get_cell_workforce_percent(x, z))
}

/// Slot 31 `GetGlobalWorkforcePercent`.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_global_workforce(p: *const fc4_residential) -> f32 {
    query(p, 0.0, |s, _| s.get_global_workforce_percent())
}

/// Slots 32 / 33 `GetGlobalEQ` / `GetGlobalHQ`.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_global_quotient(p: *const fc4_residential, is_eq: bool) -> f32 {
    query(p, 0.0, |s, _| if is_eq { s.get_global_eq() } else { s.get_global_hq() })
}

/// Slot 34 `GetGlobalLE` (AL).
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_global_le(p: *const fc4_residential) -> u8 {
    query(p, 0, |s, _| s.get_global_le())
}

/// Slots 35 / 36 `GetCellEQ` / `GetCellHQ`.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_cell_quotient(p: *const fc4_residential, is_eq: bool, x: i32, z: i32) -> f32 {
    query(p, 0.0, |s, _| {
        if is_eq {
            s.get_cell_eq(x, z)
        } else {
            s.get_cell_hq(x, z)
        }
    })
}

/// Slots 37 / 38 `GetCellEQByWealth` / `GetCellHQByWealth` (w = 1..3).
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_cell_quotient_by_wealth(
    p: *const fc4_residential,
    is_eq: bool,
    x: i32,
    z: i32,
    wealth: u8,
) -> f32 {
    query(p, 0.0, |s, _| {
        if is_eq {
            s.get_cell_eq_by_wealth(x, z, wealth)
        } else {
            s.get_cell_hq_by_wealth(x, z, wealth)
        }
    })
}

/// Slot 42 `GetAverageNewAgeByWealth` (AL).
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_new_age_by_wealth(p: *const fc4_residential, wealth: u8) -> u8 {
    query(p, 0, |s, _| s.get_average_new_age_by_wealth(wealth))
}

/// Slots 43 / 44: false (AL 0, all six outputs 0) when the population is 0.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_quotient_min_max(
    p: *const fc4_residential,
    is_eq: bool,
    out: *mut fc4_residential_min_max,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    let r = query(p, None, |s, _| s.quotient_min_max(is_eq));
    match r {
        Some((min_x, min_z, max_x, max_z, min, max)) => {
            *o = fc4_residential_min_max {
                min_x,
                min_z,
                max_x,
                max_z,
                min,
                max,
            };
            true
        }
        None => {
            *o = fc4_residential_min_max::default();
            false
        }
    }
}

/// Slot 45 `GetOccupantCoverage`: false (outputs untouched) for a building
/// that is neither a school nor a hospital. RG12 (answered; the bracket
/// matches): AL = 0 only when the building has neither capacity, and that
/// path leaves `dist` and `radius` unwritten.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_occupant_coverage(
    p: *const fc4_residential,
    occupant: u64,
    level: f32,
    distance: *mut f32,
    radius: *mut f32,
) -> bool {
    match query(p, None, |s, _| s.occupant_coverage(occupant, level)) {
        Some((d, r)) => {
            if let Some(o) = distance.as_mut() {
                *o = d;
            }
            if let Some(o) = radius.as_mut() {
                *o = r;
            }
            true
        }
        None => false,
    }
}

/// Slots 47 / 49 / 61 (`FC4_RESIDENTIAL_LIST_*`): each occupant is passed to
/// `sink` with one reference handed out (`hand_out`) for the caller's list,
/// which the glue appends at the tail with allocator A (RG17: as the
/// original, for all three slots).
/// `groups` NULL = every group (`has_groups` false); the pending list always
/// follows (REH §4.1). Slot 61 ignores `groups`. Returns the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_buildings(
    p: *const fc4_residential,
    which: u32,
    groups: *const u32,
    group_count: u32,
    sink: fc4_token_callback,
    ctx: *mut c_void,
) -> u32 {
    let Some(sink) = sink else { return 0 };
    let groups: Option<Vec<u32>> = slice_arg(groups, group_count as usize).map(<[u32]>::to_vec);
    let list = query(p, Vec::new(), |s, host| {
        let _handing = HandingOut::new(host);
        match which {
            FC4_RESIDENTIAL_LIST_SCHOOL => s.school_buildings(groups.as_deref()),
            FC4_RESIDENTIAL_LIST_HOSPITAL => s.hospital_buildings(groups.as_deref()),
            FC4_RESIDENTIAL_LIST_HEALTH_AFFECTING => s.health_affecting_buildings(),
            _ => Vec::new(),
        }
    });
    for &t in &list {
        sink(ctx, t);
    }
    list.len() as u32
}

/// Slots 53 / 55: the value. Queued off the main thread or during a call.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_set_global_auto_budget(
    p: *mut fc4_residential,
    school: bool,
    on: bool,
) -> bool {
    if let Some(h) = handle(p) {
        guard((), || h.mutate(Deferred::GlobalAutoBudget { school, on }));
    }
    on
}

/// Slot 56 `GetAutoBudget(occupant)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_auto_budget(p: *const fc4_residential, occupant: u64) -> bool {
    query(p, false, |s, _| s.get_auto_budget(occupant))
}

/// Slot 57 `SetAutoBudget(occupant, on)`: the property helper's result. The
/// core keeps no state for it, so it runs on any thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_set_auto_budget(p: *const fc4_residential, occupant: u64, on: bool) -> bool {
    query(p, false, |s, _| s.set_auto_budget(occupant, on))
}

/// Slot 58 `EstimateIdealFunding`: false (`out` untouched) when the query
/// fails or the building is not graded yet.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_ideal_funding(
    p: *const fc4_residential,
    occupant: u64,
    out: *mut f32,
) -> bool {
    match query(p, None, |s, _| s.estimate_ideal_funding(occupant)) {
        Some(f) => {
            if let Some(o) = out.as_mut() {
                *o = f;
            }
            true
        }
        None => false,
    }
}
