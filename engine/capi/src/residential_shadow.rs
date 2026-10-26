// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The residential shadow (fc4.h "residential shadow"): tooling, no spec
//! behaviour. With the ORIGINAL residential simulator running, each SimNewDay
//! delivery to its agent is wrapped:
//!
//! 1. before: the plugin captures the original's §9 record (its own `Write`
//!    into a capture stream: the bytes and the references, each reference
//!    already queried for the grid, occupant and department interfaces);
//!    [`fc4_residential_shadow_begin`] copies the 25 live grids the record
//!    refers to (read through the grid table), loads the record into a
//!    private core simulator over those copies, restores what `Init` sets and
//!    the record does not hold (the simulator tuning, the ordinance effects)
//!    from the game as it stands, and runs FreeCity4's daily state. Every
//!    write stays inside the shadow (occupant properties and local funding in
//!    overlays that later reads see; history, posts, automata and budget line
//!    calls are only counted; no visual effect is created; references are not
//!    counted);
//! 2. the original's delivery, unchanged;
//! 3. after: the plugin captures the record again and reads the original's
//!    population, global EQ / HQ and LE through its own getters;
//!    [`fc4_residential_shadow_finish`] compares, cell by cell, our 25 grids
//!    with the live ones, the saved fields with the record, the occupant
//!    properties and local funding we wrote (or read, for the properties the
//!    daily states write) with their live values, and the four scalars.
//!
//! **Why per delivery.** The original runs one state of its monthly cycle per
//! day (§5.1), and a time-sliced state (1, 2, 4) can stop part-way (§5.2).
//! Only a wrap of each delivery sees the inputs that state read and the exact
//! point where it stopped; a month-level comparison could not tell a wrong
//! state from a state that read different inputs on a later day.
//!
//! **When ours runs.** States 1, 2 and 4 read nothing the original's delivery
//! can change (the BDS building map, the pollution grids, the growth
//! developers' population maps, the tuning, the date; their grids are our
//! copies), so they run after the delivery, when the original's progress is
//! known: a state 1 / 2 the original did not finish runs with a work budget of
//! 0, and a state 4 the original stopped part-way runs exactly as many tracts
//! as the original's tract iterator advanced. Every other state runs before
//! the delivery, reading the game as the original reads it.
//!
//! **Not readable, hence not compared** ([`NOT_COMPARED`], logged once at
//! start-up): the temporary proximity grid (not saved: a delivery in states
//! 1-3 re-seeds it from the building map as it stands; RG23: the seed writes
//! only the z = 0 edge and the original's grid is 0 everywhere else, as a
//! fresh grid is, so the re-seed is exact unless that edge of the building
//! map changed since the seed day); the age histogram,
//! the coverage counters and the grade totals (not saved; history inputs
//! only); the history values and the posted strike messages; the state-0
//! strike rolls (the original's clock-seeded `cRZRandom`), so on state-0 days
//! the strike flags, strike start days and the buildings' strike days; the
//! automata controllers, budget line items and locally funded objects; the
//! visual effects (buzzards; RG1: our run counts them as created, both draws
//! and the flag, as the original with shipped data); the ordinance effects
//! before the shadow's first state 0 (read from the ordinances as they stand;
//! RG7: afterwards the runs keep the lists of our last state 0, as the
//! original keeps those of its last state 0); the dirty flag (saved as
//! `dirty ∨ state ≠ 10`, so a record taken during a cycle does not hold it).
//! RG6 (answered, no original behaviour): every draw of the class is a
//! double draw that uses only the generator's `mul` word, so a shadow that
//! read that word before state 0 could reproduce the strike rolls; this one
//! does not, and does not compare them. RG13: the local funding we set is
//! kept and compared as the item reads it back, float32(float32(q × 100.0f)
//! × 0.01f).

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::budget::{local_funding_fraction, local_funding_percent};
use fc4_core::grid::{
    CityDims, GridElem, GridProvider, ScanIterator, ScanState, SimGrid, TractGeometry, TractGrid, MAX_TRACTS,
};
use fc4_core::julian::civil_from_julian;
use fc4_core::property::PropertyHolder;
use fc4_core::residential::{
    msg, state as st, BudgetItem, CreatedEffect, EducationHealthHost, Grids, OrdinanceEffect, ResidentialHost,
    ResidentialMessage, ResidentialRng, ResidentialSim, ResidentialState, SimulatorTuning, HOSPITAL,
    IID_DEPARTMENT_BUDGET, IID_OCCUPANT, IID_SIM_GRID, MAX_LIST_COUNT, PROP_AUTO_BUDGET, PROP_STRIKE_DAY, SCHOOL,
};
use fc4_core::rng::FixedSeeds;
use fc4_core::stream::{ByteReader, InStream, RefValue, SaveObject};
use fc4_core::variant::{Elem, Variant};

use crate::guard;
use crate::hostgrid::{read_property, take_props};
use crate::landvalue::fc4_grid_geometry;
use crate::residential::{fc4_residential_budget_item, fc4_residential_host, BorrowedGrid};
use crate::stream::fc4_bytes_sink;

/// One reference of a captured record (fc4.h
/// `fc4_residential_capture_ref`): the object the original wrote, queried for
/// each interface its reader asks for (0 = the object does not answer it).
/// Tokens are borrowed.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_residential_capture_ref {
    /// As a `cISC4SimGrid` (IID `0xC9BD939A`).
    pub grid: u64,
    /// As a `cISC4Occupant` (IID `0x26BB767E`).
    pub occupant: u64,
    /// As a `cISC4DepartmentBudget` (IID `0x223E2B67`).
    pub department: u64,
    /// `GetGZCLSID()`.
    pub class_id: u32,
    pub reserved: u32,
}

/// A captured record (fc4.h `fc4_residential_capture`): the bytes the
/// original's `Write` produced (primitives little-endian, references as
/// `u32 objectID` (index + 1, 0 = null) then `u32 classID`) and its
/// references in write order.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_residential_capture {
    pub struct_size: u32,
    /// Non-zero when `Write` returned true and the stream stayed good.
    pub ok: u32,
    pub bytes: *const u8,
    pub len: usize,
    pub refs: *const fc4_residential_capture_ref,
    pub ref_count: usize,
}

/// The original's getters after its delivery (fc4.h
/// `fc4_residential_shadow_scalars`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_residential_shadow_scalars {
    pub struct_size: u32,
    /// Slot 16 `GetPopulation`.
    pub population: i32,
    /// Slots 32 / 33.
    pub global_eq: f32,
    pub global_hq: f32,
    /// Slot 34 (AL, zero-extended).
    pub global_le: u32,
    pub reserved: u32,
}

/// Totals since creation (fc4.h `fc4_residential_shadow_stats`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_residential_shadow_stats {
    pub struct_size: u32,
    /// Deliveries finished (compared or not), state-10 days left out.
    pub deliveries: u32,
    pub compared: u32,
    /// ... compared without a difference.
    pub clean: u32,
    /// ... not compared (a capture failed, our run failed).
    pub incomplete: u32,
    /// State-10 days (the original does nothing; not compared).
    pub idle: u32,
    /// Monthly cycles seen to end (state 9 → 10).
    pub cycles: u32,
    /// The last delivery's difference count (-1: not compared).
    pub last_mismatches: i32,
    pub cells: u64,
    pub cell_mismatches: u64,
    pub field_mismatches: u64,
    /// Occupant properties and local funding.
    pub prop_mismatches: u64,
    pub scalar_mismatches: u64,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_residential_capture_ref>() == 32);
    assert!(std::mem::size_of::<fc4_residential_capture>() == if cfg!(target_pointer_width = "32") { 24 } else { 40 });
    assert!(std::mem::size_of::<fc4_residential_shadow_scalars>() == 24);
    assert!(std::mem::size_of::<fc4_residential_shadow_stats>() == 72);
};

/// What the shadow cannot read, logged once at start-up
/// ([`fc4_residential_shadow_describe`]).
pub const NOT_COMPARED: &str =
    "the temporary proximity grid (not saved: a delivery in states 1-3 re-seeds it from the \
     building map as it stands; exact unless the map's z = 0 edge changed since the seed day, RG23); the age \
     histogram, the coverage counters and the grade totals (not saved; history \
     inputs only); history values and posted strike messages; the state-0 strike rolls (the original's clock-seeded \
     cRZRandom), so on state-0 days the strike flags, strike start days and buildings' strike days; automata \
     controllers, budget line items and locally funded objects; visual effects (buzzards; counted as created, RG1); \
     the ordinance effects before the shadow's first state 0 (read as they stand; afterwards those of our last state \
     0, RG7); the dirty flag (saved as dirty or state != 10, so a record taken during a cycle does not hold it)";

/// The saved grids in record order (§9.1).
const GRID_COUNT: usize = 25;
const GRID_NAMES: [&str; GRID_COUNT] = [
    "proxR$",
    "proxR$$",
    "proxR$$$",
    "popR$",
    "popR$$",
    "popR$$$",
    "age",
    "schoolCov0",
    "schoolCov1",
    "schoolCov2",
    "schoolCov3",
    "schoolCov4",
    "schoolCov5",
    "schoolCov6",
    "hospitalCov0",
    "hospitalCov1",
    "hospitalCov2",
    "schoolGrade",
    "hospitalGrade",
    "eqR$",
    "eqR$$",
    "eqR$$$",
    "hqR$",
    "hqR$$",
    "hqR$$$",
];

/// The names of the daily states (§5.1).
const STATE_NAMES: [&str; 16] = [
    "strikes and lists",
    "proximity R$",
    "proximity R$$",
    "proximity R$$$",
    "population",
    "school coverage",
    "school grading",
    "hospital coverage",
    "hospital grading",
    "history",
    "idle",
    "histogram reset",
    "post population",
    "seed R$",
    "seed R$$",
    "seed R$$$",
];

/// The occupant properties the daily states write (REH §2, §4.5, §4.9, §6.6):
/// a value the original changed and ours left is a difference too.
fn written_props() -> [u32; 10] {
    [
        SCHOOL.served,
        SCHOOL.effective_capacity,
        SCHOOL.grade,
        SCHOOL.staff_ratio,
        HOSPITAL.served,
        HOSPITAL.effective_capacity,
        HOSPITAL.grade,
        HOSPITAL.staff_ratio,
        PROP_STRIKE_DAY,
        PROP_AUTO_BUDGET,
    ]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    U8,
    U16,
    F32,
}

fn kind_of(i: usize) -> Kind {
    match i {
        3..=5 => Kind::U16,
        17..=24 => Kind::F32,
        _ => Kind::U8,
    }
}

// ---- the shadow's host -------------------------------------------------------------------------------

/// Reads the original's delivery can change (occupant properties, budget
/// items, funding): remembered as read before it, for the comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Key {
    Prop(u64, u32),
    BoolProp(u64, u32),
    BudgetItem(u64, u32),
    Funding(u64),
    LocallyFunded(u64, u64, u32),
    LineItemCount(u64),
}

#[derive(Clone)]
enum Val {
    Prop(Option<Variant>),
    Bool(Option<bool>),
    Item(Option<BudgetItem>),
    F(f32),
    B(bool),
    I64(i64),
}

/// A growth developer's population map (a borrowed view), or none.
type DevMap = Option<Rc<dyn TractGrid<i16>>>;

/// The shadow's `ResidentialHost`: reads go to the C host, writes stay here.
struct ShadowHost {
    c: fc4_residential_host,
    /// The volatile reads of this delivery, as they stood when first read.
    cache: RefCell<HashMap<Key, Val>>,
    /// Our occupant property writes, in first-write order.
    overlay: RefCell<HashMap<(u64, u32), Variant>>,
    overlay_order: RefCell<Vec<(u64, u32)>>,
    /// Our `SetLocalFundingPercent` values by (occupant, purpose).
    funding: RefCell<BTreeMap<(u64, u32), f32>>,
    /// The current run's grid copies by the original's token.
    grids_u8: RefCell<HashMap<u64, Rc<SimGrid<u8>>>>,
    grids_u16: RefCell<HashMap<u64, Rc<SimGrid<u16>>>>,
    grids_f32: RefCell<HashMap<u64, Rc<SimGrid<f32>>>>,
    /// The growth developers' population maps (borrowed views), per run.
    dev_maps: RefCell<HashMap<u32, DevMap>>,
    /// The city as the proximity grid gives it (for the temporary grid).
    dims: Cell<CityDims>,
    /// Calls kept here: lifecycle, history, posts, automata, line items.
    swallowed: Cell<u32>,
}

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the shadow promised valid callbacks.
        match $s.c.$f { Some(f) => unsafe { f($s.c.user $(, $arg)*) }, None => $default }
    };
}

impl ShadowHost {
    fn new(c: fc4_residential_host) -> ShadowHost {
        ShadowHost {
            c,
            cache: RefCell::new(HashMap::new()),
            overlay: RefCell::new(HashMap::new()),
            overlay_order: RefCell::new(Vec::new()),
            funding: RefCell::new(BTreeMap::new()),
            grids_u8: RefCell::new(HashMap::new()),
            grids_u16: RefCell::new(HashMap::new()),
            grids_f32: RefCell::new(HashMap::new()),
            dev_maps: RefCell::new(HashMap::new()),
            dims: Cell::new(CityDims::default()),
            swallowed: Cell::new(0),
        }
    }

    fn read<T: Clone>(
        &self,
        key: Key,
        wrap: impl FnOnce(T) -> Val,
        unwrap: impl Fn(&Val) -> Option<T>,
        live: impl FnOnce() -> T,
    ) -> T {
        if let Some(v) = self.cache.borrow().get(&key).and_then(&unwrap) {
            return v;
        }
        let v = live();
        self.cache.borrow_mut().insert(key, wrap(v.clone()));
        v
    }

    fn swallow(&self) {
        self.swallowed.set(self.swallowed.get().wrapping_add(1));
    }

    /// A live occupant property (not remembered).
    fn live_prop(&self, occ: u64, id: u32) -> Option<Variant> {
        let f = self.c.occupant_property?;
        // SAFETY: valid callback; the scratch holder outlives the call.
        read_property(id, |p| unsafe { f(self.c.user, occ, id, p) })
    }

    fn live_bool_prop(&self, occ: u64, id: u32) -> Option<bool> {
        let mut v = false;
        cb!(self, occupant_bool_property, false, occ, id, &mut v).then_some(v)
    }

    fn live_budget_item(&self, occ: u64, purpose: u32) -> Option<BudgetItem> {
        let mut i = fc4_residential_budget_item {
            purpose,
            ..Default::default()
        };
        cb!(self, budget_item, false, occ, purpose, &mut i).then_some(BudgetItem {
            department: i.department,
            line: i.line,
            index: i.index,
            cost: i.cost,
            local_funding: i.local_funding,
        })
    }

    fn set_overlay(&self, occ: u64, id: u32, v: Variant) {
        if self.overlay.borrow_mut().insert((occ, id), v).is_none() {
            self.overlay_order.borrow_mut().push((occ, id));
        }
    }

    /// Starts a delivery: everything remembered is forgotten.
    fn begin_delivery(&self) {
        self.cache.borrow_mut().clear();
        self.begin_run(self.dims.get());
    }

    /// Starts a run: the overlays and the grid copies are emptied.
    fn begin_run(&self, dims: CityDims) {
        self.overlay.borrow_mut().clear();
        self.overlay_order.borrow_mut().clear();
        self.funding.borrow_mut().clear();
        self.grids_u8.borrow_mut().clear();
        self.grids_u16.borrow_mut().clear();
        self.grids_f32.borrow_mut().clear();
        self.dev_maps.borrow_mut().clear();
        self.dims.set(dims);
    }
}

macro_rules! provider {
    ($t:ty, $map:ident) => {
        impl GridProvider<$t> for ShadowHost {
            fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<$t>>> {
                Some(SimGrid::<$t>::create(&self.dims.get(), instance, tract_size) as Rc<dyn TractGrid<$t>>)
            }
            fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<$t>>> {
                match *r {
                    RefValue::Host { token, .. } => self
                        .$map
                        .borrow()
                        .get(&token)
                        .map(|g| g.clone() as Rc<dyn TractGrid<$t>>),
                    _ => None,
                }
            }
        }
    };
}
provider!(u8, grids_u8);
provider!(u16, grids_u16);
provider!(f32, grids_f32);

impl ResidentialHost for ShadowHost {
    fn invocation_precision(&self) -> fc4_core::x87::Precision {
        crate::invocation::precision()
    }

    fn acquire_services(&self) {
        self.swallow();
    }
    fn acquire_developers(&self) {
        self.swallow();
    }
    fn acquire_demand_node(&self) -> bool {
        self.swallow();
        true
    }
    fn acquire_message_server(&self) -> bool {
        self.swallow();
        true
    }
    fn acquire_budget_simulator(&self) -> bool {
        self.swallow();
        true
    }
    fn has_simulator(&self) -> bool {
        self.swallow();
        true
    }
    fn add_agent(&self, _level: u32, _name: &str, _flags: u32) {
        self.swallow();
    }
    fn remove_agent(&self) {
        self.swallow();
    }
    fn subscribe(&self, _msg_type: u32) {
        self.swallow();
    }
    fn unsubscribe(&self, _msg_type: u32) {
        self.swallow();
    }
    fn release_message_server_and_budget(&self) {
        self.swallow();
    }
    fn release_services(&self) {
        self.swallow();
    }
    fn set_history_queue_length(&self, _series: u32, _length: u32) {
        self.swallow();
    }
    fn create_department(&self, _id: u32, _name: u32, _max: f32, _init: f32) -> Option<u64> {
        self.swallow();
        None
    }
    fn release_department(&self, _department: u64) {}
    fn shutdown_grid_u8(&self, _grid: &Rc<dyn TractGrid<u8>>) {}
    fn shutdown_grid_u16(&self, _grid: &Rc<dyn TractGrid<u16>>) {}
    fn shutdown_grid_f32(&self, _grid: &Rc<dyn TractGrid<f32>>) {}
    fn add_ref(&self, _token: u64) {}
    fn release(&self, _token: u64) {}
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, tuning, std::ptr::null_mut());
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }
    fn building_map(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        BorrowedGrid::<u8>::wrap(self.c.grids, cb!(self, building_map, 0))
    }
    fn building_map_key(&self, purpose: u32, wealth: u32) -> u8 {
        cb!(self, building_map_key, 0, purpose, wealth)
    }
    fn developer_population_map(&self, dev_type: u32) -> Option<Rc<dyn TractGrid<i16>>> {
        if let Some(m) = self.dev_maps.borrow().get(&dev_type) {
            return m.clone();
        }
        let m = BorrowedGrid::<i16>::wrap(self.c.grids, cb!(self, developer_population_map, 0, dev_type));
        self.dev_maps.borrow_mut().insert(dev_type, m.clone());
        m
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
    fn create_visual_effect(&self, _name: &str) -> CreatedEffect {
        // RG1 [choice]: read-only, so nothing is shown; treated as created
        // (both draws and the flag), as the original with shipped data. RG27
        // (answered; matches): after a create only the second draw and the
        // flag change state, so "not shown" counts as created.
        CreatedEffect::NotShown
    }
    fn set_effect_transform(&self, _effect: u64, _translation: [f32; 3]) {}
    fn start_and_release_effect(&self, _effect: u64) {}
    fn record_history(&self, _series: u32, _value: i32) {
        self.swallow();
    }
    fn post_strike_message(&self, _msg_type: u32, _occupant: Option<u64>, _data2: u32, _data3: u32) {
        self.swallow();
    }
    fn occupant_as_building(&self, occupant: u64) -> Option<u64> {
        let b = cb!(self, occupant_as_building, 0, occupant);
        (b != 0).then_some(b)
    }
    fn is_occupant_group(&self, occupant: u64, group: u32) -> bool {
        cb!(self, is_occupant_group, false, occupant, group)
    }
    fn occupant_bool_property(&self, occupant: u64, id: u32) -> Option<bool> {
        if let Some(Variant::Scalar(Elem::Bool(b))) = self.overlay.borrow().get(&(occupant, id)) {
            return Some(*b);
        }
        self.read(
            Key::BoolProp(occupant, id),
            Val::Bool,
            |v| match v {
                Val::Bool(b) => Some(*b),
                _ => None,
            },
            || self.live_bool_prop(occupant, id),
        )
    }
    fn set_occupant_bool_property(&self, occupant: u64, id: u32, value: bool) -> bool {
        self.set_overlay(occupant, id, Variant::Scalar(Elem::Bool(value)));
        true
    }
    fn object_ref(&self, token: u64) -> RefValue {
        if token == 0 {
            RefValue::Null
        } else {
            RefValue::Host { token, class_id: 0 }
        }
    }
    fn resolve_ref(&self, r: &RefValue, _iid: u32) -> Option<u64> {
        match *r {
            RefValue::Host { token, .. } if token != 0 => Some(token),
            _ => None,
        }
    }
}

impl EducationHealthHost for ShadowHost {
    fn budget_item(&self, occupant: u64, purpose: u32) -> Option<BudgetItem> {
        let item = self.read(
            Key::BudgetItem(occupant, purpose),
            Val::Item,
            |v| match v {
                Val::Item(i) => Some(*i),
                _ => None,
            },
            || self.live_budget_item(occupant, purpose),
        );
        // SetLocalFundingPercent sets the item's local funding (RG13,
        // answered): `set_local_funding` below keeps the value the item reads
        // back, not the fraction written.
        item.map(|mut i| {
            if let Some(&f) = self.funding.borrow().get(&(occupant, purpose)) {
                i.local_funding = f;
            }
            i
        })
    }
    fn change_budget_item_line(&self, _occupant: u64, _index: u32, _line: u32) {
        self.swallow();
    }
    fn department_budget(&self, department_id: u32) -> Option<u64> {
        let d = cb!(self, department_budget, 0, department_id);
        (d != 0).then_some(d)
    }
    fn department_funding(&self, department: u64) -> Option<f32> {
        // RG10: no callback, or NaN, is "missing" (remembered as NaN).
        let f = self.read(
            Key::Funding(department),
            Val::F,
            |v| match v {
                Val::F(f) => Some(*f),
                _ => None,
            },
            || cb!(self, department_funding, f32::NAN, department),
        );
        (!f.is_nan()).then_some(f)
    }
    fn is_locally_funded(&self, department: u64, occupant: u64, purpose: u32) -> bool {
        self.read(
            Key::LocallyFunded(department, occupant, purpose),
            Val::B,
            |v| match v {
                Val::B(b) => Some(*b),
                _ => None,
            },
            || cb!(self, is_locally_funded, false, department, occupant, purpose),
        )
    }
    fn add_locally_funded(&self, _department: u64, _occupant: u64, _purpose: u32) {
        self.swallow();
    }
    fn remove_locally_funded(&self, _department: u64, _occupant: u64, _purpose: u32) {
        self.swallow();
    }
    fn set_local_funding(&self, _department: u64, occupant: u64, fraction: f32, purpose: u32) {
        // REH §4.2 (RG13, answered; the bracket needed a correction): the
        // item stores float32(q × 100.0f) and reads back float32(pct ×
        // 0.01f), an inexact round trip (0.43 → 0.42999998), so the shadow
        // keeps and compares the read-back value.
        let back = local_funding_fraction(local_funding_percent(fraction));
        self.funding.borrow_mut().insert((occupant, purpose), back);
    }
    fn line_item(&self, department: u64, line: u32) -> Option<u64> {
        let l = cb!(self, line_item, 0, department, line);
        (l != 0).then_some(l)
    }
    fn create_line_item_for_building_type(&self, _department: u64, _line: u32, _local: bool) -> Option<u64> {
        self.swallow();
        None
    }
    fn remove_line_item(&self, _department: u64, _line: u32) {
        self.swallow();
    }
    fn line_item_count(&self, line_item: u64) -> i64 {
        self.read(
            Key::LineItemCount(line_item),
            Val::I64,
            |v| match v {
                Val::I64(n) => Some(*n),
                _ => None,
            },
            || cb!(self, line_item_count, 0, line_item),
        )
    }
    fn set_line_item_count(&self, _line_item: u64, _count: i64) {
        self.swallow();
    }
    fn set_line_item_display_flag(&self, _line_item: u64, _flag: u32, _on: bool) {
        self.swallow();
    }
    fn occupant_property(&self, occupant: u64, id: u32) -> Option<Variant> {
        if let Some(v) = self.overlay.borrow().get(&(occupant, id)) {
            return Some(v.clone());
        }
        self.read(
            Key::Prop(occupant, id),
            Val::Prop,
            |v| match v {
                Val::Prop(p) => Some(p.clone()),
                _ => None,
            },
            || self.live_prop(occupant, id),
        )
    }
    fn set_occupant_u32(&self, occupant: u64, id: u32, value: u32) {
        self.set_overlay(occupant, id, Variant::Scalar(Elem::U32(value)));
    }
    fn set_occupant_f32(&self, occupant: u64, id: u32, value: f32) {
        self.set_overlay(occupant, id, Variant::Scalar(Elem::F32(value)));
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
        unsafe extern "C" fn push(ctx: *mut c_void, v: u32) {
            if let Some(out) = (ctx as *mut Vec<u32>).as_mut() {
                out.push(v);
            }
        }
        let mut v: Vec<u32> = Vec::new();
        cb!(
            self,
            ordinances_on,
            (),
            Some(push),
            &mut v as *mut Vec<u32> as *mut c_void
        );
        v
    }
    fn ordinance_properties(&self, id: u32) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, ordinance_properties, std::ptr::null_mut(), id);
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }
    fn add_controllers(&self, _name: &str, _occupant: u64) {
        self.swallow();
    }
    fn remove_controllers(&self, _name: &str, _occupant: u64) {
        self.swallow();
    }
}

// ---- the captured record -------------------------------------------------------------------------------

/// A captured record as a core input stream: references are answered with
/// the token of the interface the reader asks for.
struct CaptureReader<'a> {
    r: ByteReader<'a>,
    refs: &'a [fc4_residential_capture_ref],
}

impl<'a> CaptureReader<'a> {
    fn new(bytes: &'a [u8], refs: &'a [fc4_residential_capture_ref]) -> Self {
        CaptureReader {
            r: ByteReader::new(bytes),
            refs,
        }
    }
}

impl InStream for CaptureReader<'_> {
    fn get_u8(&mut self) -> Option<u8> {
        self.r.get_u8()
    }
    fn get_u16(&mut self) -> Option<u16> {
        self.r.get_u16()
    }
    fn get_u32(&mut self) -> Option<u32> {
        self.r.get_u32()
    }
    fn get_u64(&mut self) -> Option<u64> {
        self.r.get_u64()
    }
    fn get_f32(&mut self) -> Option<f32> {
        self.r.get_f32()
    }
    fn get_f64(&mut self) -> Option<f64> {
        self.r.get_f64()
    }
    fn get_bytes(&mut self, n: usize) -> Option<Vec<u8>> {
        self.r.get_bytes(n)
    }
    fn get_str(&mut self) -> Option<Vec<u8>> {
        self.r.get_str()
    }
    fn get_ref(&mut self) -> Option<RefValue> {
        self.get_ref_as(0)
    }
    fn get_ref_as(&mut self, iid: u32) -> Option<RefValue> {
        let id = self.r.get_u32()?;
        if id == 0 {
            return Some(RefValue::Null);
        }
        let class_id = self.r.get_u32()?;
        let Some(e) = self.refs.get(id as usize - 1) else {
            self.r.set_error();
            return None;
        };
        let t = pick(e, iid);
        // §2.7 helper semantics: an object without the interface reads as null.
        Some(if t != 0 {
            RefValue::Host { token: t, class_id }
        } else {
            RefValue::Null
        })
    }
    fn skip(&mut self, n: usize) -> bool {
        self.r.skip(n)
    }
    fn error(&self) -> i32 {
        self.r.error()
    }
    fn set_error(&mut self) {
        self.r.set_error();
    }
}

fn pick(e: &fc4_residential_capture_ref, iid: u32) -> u64 {
    match iid {
        IID_SIM_GRID => e.grid,
        IID_OCCUPANT => e.occupant,
        IID_DEPARTMENT_BUDGET => e.department,
        _ => 0,
    }
}

/// A capture copied out of the caller's buffers.
#[derive(Clone, Default)]
struct Capture {
    bytes: Vec<u8>,
    refs: Vec<fc4_residential_capture_ref>,
}

/// Copies a capture; `Err` with the reason when it is not usable.
unsafe fn capture_of(c: *const fc4_residential_capture) -> Result<Capture, &'static str> {
    let Some(c) = c.as_ref() else {
        return Err("no capture");
    };
    if (c.struct_size as usize) < std::mem::size_of::<fc4_residential_capture>() {
        return Err("capture struct too small");
    }
    if c.ok == 0 {
        return Err("the original's Write failed");
    }
    if c.bytes.is_null() || c.len == 0 {
        return Err("empty record");
    }
    let bytes = std::slice::from_raw_parts(c.bytes, c.len).to_vec();
    let refs = if c.refs.is_null() || c.ref_count == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(c.refs, c.ref_count).to_vec()
    };
    Ok(Capture { bytes, refs })
}

/// One education or health block of the record, as written.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct RawBlock {
    f0: f32,
    max_funding: f32,
    strike_efficiency: f32,
    strike_duration: i32,
    min_grade: f32,
    strike_flag: u8,
    strike_start: i32,
    departments: [u64; 2],
    init_funding: f32,
    hundred: f32,
    id: u32,
}

/// The tract iterator as the record lays it out (§9.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RawIterator {
    state: i8,
    x: i32,
    z: i32,
    step: u32,
    x0: i32,
    z0: i32,
    x1: i32,
    z1: i32,
}

impl RawIterator {
    fn of(it: &ScanIterator) -> RawIterator {
        RawIterator {
            state: match it.state {
                ScanState::Fresh => 0,
                ScanState::Running => 1,
                ScanState::Done => 2,
            },
            x: it.x,
            z: it.z,
            step: it.step as u32,
            x0: it.x0,
            z0: it.z0,
            x1: it.x1,
            z1: it.z1,
        }
    }
}

/// The version-7 record of §9.1 as written: no fix-ups (state 1 stays 1),
/// references as the tokens the original wrote. Null list entries are
/// dropped (as `Read` drops them, RS12).
#[derive(Clone, Debug, Default)]
struct RawRecord {
    proximity_size: i32,
    population_size: i32,
    eq_size: i32,
    grids: [u64; GRID_COUNT],
    unused: [u32; 2],
    state: u32,
    population: i32,
    /// Health, then education (record order).
    blocks: [RawBlock; 2],
    /// The 14 occupant lists in record order.
    lists: Vec<Vec<u64>>,
    iterator: RawIterator,
    le_sum: u64,
    running_population: i32,
    auto_budget: [u8; 2],
}

struct RawParser<'a> {
    r: ByteReader<'a>,
    refs: &'a [fc4_residential_capture_ref],
}

impl RawParser<'_> {
    fn token(&mut self, iid: u32) -> Option<u64> {
        let id = self.r.get_u32()?;
        if id == 0 {
            return Some(0);
        }
        self.r.get_u32()?;
        self.refs.get(id as usize - 1).map(|e| pick(e, iid))
    }

    fn block(&mut self) -> Option<RawBlock> {
        let r = &mut self.r;
        let mut b = RawBlock {
            f0: r.get_f32()?,
            max_funding: r.get_f32()?,
            strike_efficiency: r.get_f32()?,
            strike_duration: r.get_i32()?,
            min_grade: r.get_f32()?,
            strike_flag: r.get_u8()?,
            strike_start: r.get_i32()?,
            ..RawBlock::default()
        };
        b.departments = [self.token(IID_DEPARTMENT_BUDGET)?, self.token(IID_DEPARTMENT_BUDGET)?];
        b.init_funding = self.r.get_f32()?;
        b.hundred = self.r.get_f32()?;
        b.id = self.r.get_u32()?;
        Some(b)
    }

    fn list(&mut self) -> Option<Vec<u64>> {
        let n = self.r.get_u32()?;
        let mut v = Vec::new();
        if n == 0 || n >= MAX_LIST_COUNT {
            return Some(v);
        }
        for _ in 0..n {
            let t = self.token(IID_OCCUPANT)?;
            if t != 0 {
                v.push(t);
            }
        }
        Some(v)
    }

    fn record(&mut self) -> Result<RawRecord, String> {
        let bad = || "the record ends early or is damaged".to_string();
        let version = self.r.get_u16().ok_or_else(bad)?;
        if version != 7 {
            return Err(format!("record version {version}, expected 7"));
        }
        let mut rec = RawRecord {
            proximity_size: self.r.get_i32().ok_or_else(bad)?,
            ..RawRecord::default()
        };
        let mut grids = [0u64; GRID_COUNT];
        let mut next = 0usize;
        let mut take = |p: &mut Self, n: usize| -> Option<()> {
            for _ in 0..n {
                grids[next] = p.token(IID_SIM_GRID)?;
                next += 1;
            }
            Some(())
        };
        take(self, 3).ok_or_else(bad)?;
        rec.population_size = self.r.get_i32().ok_or_else(bad)?;
        take(self, 4).ok_or_else(bad)?; // population ×3, age
        if self.r.get_u32() != Some(7) {
            return Err(bad());
        }
        take(self, 7).ok_or_else(bad)?;
        if self.r.get_u32() != Some(3) {
            return Err(bad());
        }
        take(self, 3).ok_or_else(bad)?;
        take(self, 2).ok_or_else(bad)?; // school / hospital grade
        rec.eq_size = self.r.get_i32().ok_or_else(bad)?;
        take(self, 6).ok_or_else(bad)?; // EQ ×3, HQ ×3
        rec.grids = grids;
        rec.unused = [self.r.get_u32().ok_or_else(bad)?, self.r.get_u32().ok_or_else(bad)?];
        rec.state = self.r.get_u32().ok_or_else(bad)?;
        self.r.get_u8().ok_or_else(bad)?; // dirty: not compared
        rec.population = self.r.get_i32().ok_or_else(bad)?;
        rec.blocks = [self.block().ok_or_else(bad)?, self.block().ok_or_else(bad)?];
        for _ in 0..3 {
            let l = self.list().ok_or_else(bad)?;
            rec.lists.push(l);
        }
        for n in [7u32, 3] {
            if self.r.get_u32() != Some(n) {
                return Err(bad());
            }
            for _ in 0..n {
                let l = self.list().ok_or_else(bad)?;
                rec.lists.push(l);
            }
        }
        let l = self.list().ok_or_else(bad)?;
        rec.lists.push(l);
        let r = &mut self.r;
        rec.iterator = RawIterator {
            state: r.get_i8().ok_or_else(bad)?,
            x: r.get_i32().ok_or_else(bad)?,
            z: r.get_i32().ok_or_else(bad)?,
            step: r.get_u32().ok_or_else(bad)?,
            x0: r.get_i32().ok_or_else(bad)?,
            z0: r.get_i32().ok_or_else(bad)?,
            x1: r.get_i32().ok_or_else(bad)?,
            z1: r.get_i32().ok_or_else(bad)?,
        };
        rec.le_sum = r.get_u64().ok_or_else(bad)?;
        rec.running_population = r.get_i32().ok_or_else(bad)?;
        rec.auto_budget = [r.get_u8().ok_or_else(bad)?, r.get_u8().ok_or_else(bad)?];
        Ok(rec)
    }
}

fn raw_record(c: &Capture) -> Result<RawRecord, String> {
    RawParser {
        r: ByteReader::new(&c.bytes),
        refs: &c.refs,
    }
    .record()
}

/// The record's iterator as a scan iterator (`Read`'s conversions: a state
/// byte other than 0 / 1 reads as done, a step below 1 as 1).
fn scan_of(r: &RawIterator) -> ScanIterator {
    let mut it = ScanIterator::new(r.x0, r.z0, r.x1, r.z1);
    it.x = r.x;
    it.z = r.z;
    it.set_step(r.step as i32);
    it.state = match r.state {
        0 => ScanState::Fresh,
        1 => ScanState::Running,
        _ => ScanState::Done,
    };
    it
}

/// The tracts a scan visits from `from` until it stands where `to` stands,
/// `None` if it never does (within the rectangle's tract count).
fn tracts_between(from: &RawIterator, to: &RawIterator) -> Option<u32> {
    let mut it = scan_of(from);
    let span = |a: i32, b: i32| (i64::from(b) - i64::from(a) + 1).clamp(0, i64::from(MAX_TRACTS)) as u64;
    let limit = span(it.x0, it.x1) * span(it.z0, it.z1) + 1;
    let mut n = 0u64;
    loop {
        if RawIterator::of(&it) == *to {
            return u32::try_from(n).ok();
        }
        if n >= limit || it.next_cell().is_none() {
            return None;
        }
        n += 1;
    }
}

// ---- the shadow ---------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_residential_shadow`).
#[repr(C)]
pub struct fc4_residential_shadow {
    _private: [u8; 0],
}

enum BaseValues {
    U8(Vec<u8>),
    U16(Vec<u16>),
    F32(Vec<f32>),
}

/// A live grid as it stood before the original's delivery.
struct BaseGrid {
    token: u64,
    geom: TractGeometry,
    values: BaseValues,
}

/// How our delivery runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Policy {
    /// The default (unbounded) work budget.
    Unbounded,
    /// A work budget of `n` units: a state the original stopped part-way.
    Budget(u32),
}

/// One run of ours.
struct Run {
    sim: Rc<ResidentialSim>,
    overlay: Vec<((u64, u32), Variant)>,
    funding: BTreeMap<(u64, u32), f32>,
    notes: Vec<String>,
}

/// A delivery ready to compare.
struct Ready {
    julian: i32,
    before: Capture,
    raw: RawRecord,
    base: Vec<BaseGrid>,
    dims: CityDims,
    /// `None`: deferred to the finish (states 1, 2 and 4).
    run: Option<Result<Run, String>>,
}

/// The delivery between begin and finish.
enum Pending {
    Failed {
        julian: i32,
        state: Option<u32>,
        reason: String,
    },
    Ready(Box<Ready>),
}

#[derive(Default, Clone, Copy)]
struct GridAcc {
    cells: u64,
    mismatches: u64,
}

/// Totals of the current monthly cycle.
#[derive(Default)]
struct CycleAcc {
    deliveries: u32,
    compared: u32,
    clean: u32,
    incomplete: u32,
    grids: [GridAcc; GRID_COUNT],
    fields: BTreeMap<String, u32>,
    props: (u64, u64),
    funding: (u64, u64),
    scalars: (u64, u64),
    reasons: BTreeMap<String, u32>,
}

struct Shadow {
    host: Rc<ShadowHost>,
    max_cells: usize,
    pending: Option<Pending>,
    /// The ordinance-effect lists (health, school) our run read in the last
    /// state 0 (REH §4.3, RG7): the original reads them only in `Init` and in
    /// state 0, so the runs in between use these. `None` until the shadow has
    /// run a state 0; before it the shadow reads them as they stand, which
    /// matches the original's `Init` lists while no ordinance has changed
    /// since `Init` (RG28, answered: acceptable; the [choice] of reading them
    /// at `Init` too is not taken).
    effects: Option<(Vec<OrdinanceEffect>, Vec<OrdinanceEffect>)>,
    cycle: CycleAcc,
    stats: fc4_residential_shadow_stats,
}

fn date(julian: i32) -> String {
    let d = civil_from_julian(julian);
    format!("{:04}-{:02}-{:02}", d.year, d.month, d.day)
}

fn state_name(s: u32) -> &'static str {
    STATE_NAMES.get(s as usize).copied().unwrap_or("unknown")
}

/// Equal, or both NaN.
fn same_f32(a: f32, b: f32) -> bool {
    a == b || (a.is_nan() && b.is_nan())
}

fn same_elem<T: GridElem>(a: T, b: T) -> bool {
    a == b || (T::FLOAT && a.to_f64().is_nan() && b.to_f64().is_nan())
}

/// The numbers of a variant, for comparing values whose element types may
/// differ between our write and the game's property.
fn numbers(v: &Option<Variant>) -> Option<Vec<f64>> {
    v.as_ref().map(|v| v.items().iter().map(|e| e.as_f64()).collect())
}

fn same_value(a: &Option<Variant>, b: &Option<Variant>) -> bool {
    match (numbers(a), numbers(b)) {
        (None, None) => true,
        (Some(x), Some(y)) => x.len() == y.len() && x.iter().zip(&y).all(|(p, q)| p == q || (p.is_nan() && q.is_nan())),
        _ => false,
    }
}

fn show(v: &Option<Variant>) -> String {
    match numbers(v) {
        None => "absent".to_string(),
        Some(n) if n.len() == 1 => format!("{}", n[0]),
        Some(n) => format!("{n:?}"),
    }
}

enum GridRef {
    U8(Option<Rc<dyn TractGrid<u8>>>),
    U16(Option<Rc<dyn TractGrid<u16>>>),
    F32(Option<Rc<dyn TractGrid<f32>>>),
}

/// Our grid at record position `i`.
fn grid_at(g: &Grids, i: usize) -> GridRef {
    match i {
        0..=2 => GridRef::U8(g.proximity[i].clone()),
        3..=5 => GridRef::U16(g.population[i - 3].clone()),
        6 => GridRef::U8(g.age.clone()),
        7..=13 => GridRef::U8(g.school_coverage[i - 7].clone()),
        14..=16 => GridRef::U8(g.hospital_coverage[i - 14].clone()),
        17 => GridRef::F32(g.school_grade.clone()),
        18 => GridRef::F32(g.hospital_grade.clone()),
        19..=21 => GridRef::F32(g.eq[i - 19].clone()),
        _ => GridRef::F32(g.hq[i - 22].clone()),
    }
}

const LIST_NAMES: [&str; 14] = [
    "pendingSchool",
    "pendingHospital",
    "other",
    "school0",
    "school1",
    "school2",
    "school3",
    "school4",
    "school5",
    "school6",
    "hospital0",
    "hospital1",
    "hospital2",
    "healthAffecting",
];

/// The saved fields that differ: (name, original, ours). On a state-0 day
/// the strike flags and start days are left out (the strike rolls).
fn field_diffs(o: &RawRecord, b: &ResidentialState, strikes_excluded: bool) -> Vec<(String, String, String)> {
    let mut v: Vec<(String, String, String)> = Vec::new();
    let mut f = |name: &str, orig: String, ours: String| {
        if orig != ours {
            v.push((name.to_string(), orig, ours));
        }
    };
    let bits = |x: f32| format!("{x:?}");
    f(
        "proximitySize",
        o.proximity_size.to_string(),
        b.proximity_size.to_string(),
    );
    f(
        "populationSize",
        o.population_size.to_string(),
        b.population_size.to_string(),
    );
    f("eqSize", o.eq_size.to_string(), b.eq_size.to_string());
    f(
        "unused1",
        format!("{:#x}", o.unused[0]),
        format!("{:#x}", b.unused_float1.to_bits()),
    );
    f(
        "unused2",
        format!("{:#x}", o.unused[1]),
        format!("{:#x}", b.unused_float2.to_bits()),
    );
    f("state", o.state.to_string(), b.state.to_string());
    // Not `dirty`: saved as `dirty ∨ state ≠ 10`, the record of a running
    // cycle does not hold it (NOT_COMPARED). RG14 (answered: the choice
    // stands): no slot returns the flag and nothing in the class reads it.
    f("population", o.population.to_string(), b.population.to_string());
    for (name, ob, bb) in [
        ("health", &o.blocks[0], &b.health),
        ("education", &o.blocks[1], &b.education),
    ] {
        let fl = |x: f32, y: f32| {
            if same_f32(x, y) {
                (String::new(), String::new())
            } else {
                (bits(x), bits(y))
            }
        };
        for (field, (x, y)) in [
            ("f0", fl(ob.f0, bb.f0)),
            ("maxFunding", fl(ob.max_funding, bb.max_funding)),
            ("strikeEfficiency", fl(ob.strike_efficiency, bb.strike_efficiency)),
            ("minGrade", fl(ob.min_grade, bb.min_grade)),
            ("initFunding", fl(ob.init_funding, bb.init_funding)),
            ("hundred", fl(ob.hundred, bb.hundred)),
        ] {
            f(&format!("{name}.{field}"), x, y);
        }
        f(
            &format!("{name}.strikeDuration"),
            ob.strike_duration.to_string(),
            bb.strike_duration.to_string(),
        );
        f(&format!("{name}.id"), format!("{:#x}", ob.id), format!("{:#x}", bb.id));
        if !strikes_excluded {
            f(
                &format!("{name}.strikeFlag"),
                ob.strike_flag.to_string(),
                bb.strike_flag.to_string(),
            );
            f(
                &format!("{name}.strikeStart"),
                ob.strike_start.to_string(),
                bb.strike_start.to_string(),
            );
        }
        let ours = [bb.department1.unwrap_or(0), bb.department2.unwrap_or(0)];
        f(
            &format!("{name}.departments"),
            format!("{:x?}", ob.departments),
            format!("{ours:x?}"),
        );
    }
    let lists = b.lists.save_order();
    for (i, name) in LIST_NAMES.iter().enumerate() {
        let ours: Vec<u64> = lists
            .get(i)
            .map(|l| l.iter().flatten().copied().collect())
            .unwrap_or_default();
        let orig = o.lists.get(i).cloned().unwrap_or_default();
        if ours != orig {
            f(
                name,
                format!("n={} {:x?}", orig.len(), &orig[..orig.len().min(4)]),
                format!("n={} {:x?}", ours.len(), &ours[..ours.len().min(4)]),
            );
        }
    }
    let it = RawIterator::of(&b.iterator);
    if it != o.iterator {
        let show_it = |r: &RawIterator| format!("{}@({},{})", r.state, r.x, r.z);
        f("iterator", show_it(&o.iterator), show_it(&it));
    }
    f("leSum", o.le_sum.to_string(), b.le_sum.to_string());
    f(
        "runningPopulation",
        o.running_population.to_string(),
        b.running_population.to_string(),
    );
    f(
        "autoBudget",
        format!("{:?}", [o.auto_budget[0] != 0, o.auto_budget[1] != 0]),
        format!("{:?}", [b.school_auto_budget, b.hospital_auto_budget]),
    );
    v
}

impl Shadow {
    fn api(&self) -> &crate::hostgrid::fc4_grid_api {
        &self.host.c.grids
    }

    fn geometry(&self, token: u64) -> TractGeometry {
        let mut g = fc4_grid_geometry::default();
        if let Some(f) = self.api().geometry {
            // SAFETY: valid callback; `g` outlives the call.
            unsafe { f(self.api().user, token, &mut g) };
        }
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

    fn live<T: GridElem>(&self, token: u64, tx: i32, tz: i32) -> T {
        match self.api().get {
            // SAFETY: valid callback on a live game grid.
            Some(f) => T::from_f64(unsafe { f(self.api().user, token, T::CLSID, tx, tz) }),
            None => T::default(),
        }
    }

    /// A live grid's values, x outer.
    fn values<T: GridElem>(&self, token: u64, geom: &TractGeometry) -> Vec<T> {
        let (nx, nz) = (geom.count_x.clamp(0, MAX_TRACTS), geom.count_z.clamp(0, MAX_TRACTS));
        let mut v = Vec::with_capacity((nx as usize) * (nz as usize));
        for x in 0..nx {
            for z in 0..nz {
                v.push(self.live::<T>(token, x, z));
            }
        }
        v
    }

    fn copy(&self, i: usize, token: u64) -> BaseGrid {
        if token == 0 {
            return BaseGrid {
                token,
                geom: TractGeometry::default(),
                values: BaseValues::U8(Vec::new()),
            };
        }
        let geom = self.geometry(token);
        let values = match kind_of(i) {
            Kind::U8 => BaseValues::U8(self.values(token, &geom)),
            Kind::U16 => BaseValues::U16(self.values(token, &geom)),
            Kind::F32 => BaseValues::F32(self.values(token, &geom)),
        };
        BaseGrid { token, geom, values }
    }

    /// The city the copied proximity (else population) grids describe: cells
    /// = tracts × tract size, cell width = tract width / tract size.
    fn dims_of(base: &[BaseGrid]) -> CityDims {
        for b in base.iter().take(6) {
            let g = &b.geom;
            if b.token != 0 && g.tract_size > 0 && g.count_x > 0 && g.count_z > 0 {
                let s = g.tract_size;
                return CityDims {
                    cells_x: g.count_x.saturating_mul(s),
                    cells_z: g.count_z.saturating_mul(s),
                    cell_width_x: g.width_x / s as f32,
                    cell_width_z: g.width_z / s as f32,
                };
            }
        }
        CityDims::default()
    }

    /// Runs ours on the record captured before the delivery.
    fn run(&self, p: &Ready, policy: Policy) -> Result<Run, String> {
        let h = &self.host;
        h.begin_run(p.dims);
        for b in p.base.iter().filter(|b| b.token != 0) {
            match &b.values {
                BaseValues::U8(v) => {
                    h.grids_u8.borrow_mut().insert(b.token, SimGrid::from_values(b.geom, v));
                }
                BaseValues::U16(v) => {
                    h.grids_u16
                        .borrow_mut()
                        .insert(b.token, SimGrid::from_values(b.geom, v));
                }
                BaseValues::F32(v) => {
                    h.grids_f32
                        .borrow_mut()
                        .insert(b.token, SimGrid::from_values(b.geom, v));
                }
            }
        }
        let seeds = FixedSeeds {
            wall_clock: 1,
            process_clock: 1,
        };
        let rng = Box::new(ResidentialRng::from_session(1, &seeds));
        let sim = ResidentialSim::new(h.clone() as Rc<dyn ResidentialHost>, 0, rng);
        let mut r = CaptureReader::new(&p.before.bytes, &p.before.refs);
        if !sim.read(&mut r) {
            return Err("our Read of the original's record failed".to_string());
        }
        // What Init sets and the record does not hold (§4.1 step 7, §7.2, REH
        // §4.3). Init itself is not called (it would create grids and
        // departments); the blocks keep their saved values (the tuning read's
        // defaults go to copies). The tuning from the game as it stands; the
        // ordinance effects (RG7, answered: the original reads them only in
        // Init and in state 0) from our last state 0, and only before the
        // shadow has run one from the ordinances as they stand [choice].
        let mut notes = Vec::new();
        let exemplar = h.tuning();
        let (health_effects, school_effects) = match &self.effects {
            Some((health, school)) => (health.clone(), school.clone()),
            None => {
                if p.raw.state != st::STRIKES_AND_LISTS {
                    notes.push("ordinance effects as they stand (no state 0 run yet)".to_string());
                }
                let ordinances: Vec<Rc<PropertyHolder>> = h
                    .ordinances_on()
                    .into_iter()
                    .filter_map(|id| h.ordinance_properties(id))
                    .collect();
                (
                    ordinances
                        .iter()
                        .filter_map(|o| OrdinanceEffect::read(o, &HOSPITAL))
                        .collect(),
                    ordinances
                        .iter()
                        .filter_map(|o| OrdinanceEffect::read(o, &SCHOOL))
                        .collect(),
                )
            }
        };
        sim.with_state(|s| {
            let (mut health, mut education) = (s.health, s.education);
            s.simulator_tuning = SimulatorTuning::read(exemplar.as_deref(), &mut health, &mut education);
            s.health_effects = health_effects;
            s.school_effects = school_effects;
            s.initialized = true;
            s.message_server_held = true;
        });
        // The temporary grid is not saved (`Read` turns states 1-3 into 13-15):
        // seed it first. [compat] (RG23): the seed writes only the z = 0 edge
        // and reads only `map[x][0]`; off the edge the original's grid holds
        // 0 in every one of states 1-3 (a fresh grid, then a previous wealth's
        // passes, which leave 0 there), as our fresh grid does. So the re-seed
        // gives the original's grid unless the building map's z = 0 edge
        // changed between the seed day and this one.
        if (st::PROXIMITY_LOW..=st::PROXIMITY_HIGH).contains(&p.raw.state) {
            sim.do_daily_simulation();
            notes.push("temporary grid re-seeded from the building map as it stands".to_string());
        }
        if let Policy::Budget(n) = policy {
            sim.set_work_budget(n);
        }
        sim.do_message(&ResidentialMessage::of_type(msg::SIM_NEW_DAY));
        let overlay = {
            let o = h.overlay.borrow();
            h.overlay_order
                .borrow()
                .iter()
                .filter_map(|k| o.get(k).map(|v| (*k, v.clone())))
                .collect()
        };
        Ok(Run {
            sim,
            overlay,
            funding: h.funding.borrow().clone(),
            notes,
        })
    }

    fn begin(&mut self, julian: i32, before: Result<Capture, &'static str>) -> i32 {
        self.pending = None;
        let fail = |state: Option<u32>, reason: String| Pending::Failed { julian, state, reason };
        let before = match before {
            Ok(c) => c,
            Err(e) => {
                self.pending = Some(fail(None, format!("before: {e}")));
                return -1;
            }
        };
        let raw = match raw_record(&before) {
            Ok(r) => r,
            Err(e) => {
                self.pending = Some(fail(None, format!("before: {e}")));
                return -1;
            }
        };
        if raw.state == st::IDLE {
            // State 10 waits for SimNewMonth: the original does nothing.
            self.stats.idle = self.stats.idle.wrapping_add(1);
            return -2;
        }
        self.host.begin_delivery();
        let base: Vec<BaseGrid> = raw.grids.iter().enumerate().map(|(i, &t)| self.copy(i, t)).collect();
        let dims = Shadow::dims_of(&base);
        let mut ready = Ready {
            julian,
            before,
            raw,
            base,
            dims,
            run: None,
        };
        let deferred = matches!(
            ready.raw.state,
            st::PROXIMITY_LOW | st::PROXIMITY_MEDIUM | st::POPULATION
        );
        let r = if deferred {
            0
        } else {
            let run = self.run(&ready, Policy::Unbounded);
            let r = if run.is_ok() { 0 } else { -1 };
            if let (st::STRIKES_AND_LISTS, Ok(done)) = (ready.raw.state, &run) {
                // RG7: state 0 re-read the lists; keep them to the next.
                self.effects = Some(
                    done.sim
                        .with_state(|s| (s.health_effects.clone(), s.school_effects.clone())),
                );
            }
            ready.run = Some(run);
            r
        };
        self.pending = Some(Pending::Ready(Box::new(ready)));
        r
    }

    /// The policy of a deferred run, from the original's progress.
    fn policy(p: &Ready, after: &RawRecord) -> (Policy, Option<String>) {
        let s = p.raw.state;
        if after.state != s {
            return (Policy::Unbounded, None);
        }
        if s == st::POPULATION {
            return match tracts_between(&p.raw.iterator, &after.iterator) {
                Some(n) => (
                    Policy::Budget(n),
                    Some(format!("the original stopped after {n} tracts: ours ran {n}")),
                ),
                None => (
                    Policy::Unbounded,
                    Some("the original's tract position is not on our scan: ours ran to the end".to_string()),
                ),
            };
        }
        (
            Policy::Budget(0),
            Some("the original did not finish its passes: ours ran with a work budget of 0".to_string()),
        )
    }

    fn finish(
        &mut self,
        after: Result<Capture, &'static str>,
        scalars: Option<fc4_residential_shadow_scalars>,
        lines: &mut Vec<String>,
    ) -> i32 {
        let Some(p) = self.pending.take() else {
            return -2;
        };
        self.stats.deliveries = self.stats.deliveries.wrapping_add(1);
        self.cycle.deliveries += 1;
        let after_raw = after
            .map_err(|e| format!("after: {e}"))
            .and_then(|c| raw_record(&c).map_err(|e| format!("after: {e}")));
        let (julian, before_state) = match &p {
            Pending::Failed { julian, state, .. } => (*julian, *state),
            Pending::Ready(r) => (r.julian, Some(r.raw.state)),
        };
        let prefix = match before_state {
            Some(s) => format!("residential shadow {} state {s} ({})", date(julian), state_name(s)),
            None => format!("residential shadow {}", date(julian)),
        };
        let result = match p {
            Pending::Failed { reason, .. } => Err(reason),
            Pending::Ready(r) => match &after_raw {
                Ok(a) => self.compare(&r, a, scalars, &prefix, lines),
                Err(e) => Err(e.clone()),
            },
        };
        let r = match result {
            Ok(n) => {
                self.stats.compared = self.stats.compared.wrapping_add(1);
                self.cycle.compared += 1;
                if n == 0 {
                    self.stats.clean = self.stats.clean.wrapping_add(1);
                    self.cycle.clean += 1;
                }
                n.min(i32::MAX as u64) as i32
            }
            Err(e) => {
                self.stats.incomplete = self.stats.incomplete.wrapping_add(1);
                self.cycle.incomplete += 1;
                *self.cycle.reasons.entry(e.clone()).or_insert(0) += 1;
                lines.push(format!("{prefix}: INCOMPLETE ({e}); not compared"));
                -1
            }
        };
        self.stats.last_mismatches = r;
        if before_state == Some(st::HISTORY) && matches!(&after_raw, Ok(a) if a.state == st::IDLE) {
            self.stats.cycles = self.stats.cycles.wrapping_add(1);
            lines.push(self.cycle_line(julian));
            self.cycle = CycleAcc::default();
        }
        r
    }

    /// One grid, cell by cell: (cells, differences, the line's part).
    fn compare_grid<T: GridElem>(
        &mut self,
        i: usize,
        ours: Option<Rc<dyn TractGrid<T>>>,
        live: u64,
    ) -> (u64, u64, Option<String>) {
        match (ours, live) {
            (None, 0) => (0, 0, None),
            (Some(g), t) if t != 0 => {
                let og = g.geometry();
                let lg = self.geometry(t);
                let (nx, nz) = (og.count_x.clamp(0, MAX_TRACTS), og.count_z.clamp(0, MAX_TRACTS));
                let mut bad = 0u64;
                let mut firsts: Vec<String> = Vec::new();
                let geom = if (lg.count_x, lg.count_z) != (og.count_x, og.count_z) {
                    bad += 1;
                    format!(
                        " geometry orig/ours {}x{}/{}x{}",
                        lg.count_x, lg.count_z, og.count_x, og.count_z
                    )
                } else {
                    String::new()
                };
                for x in 0..nx {
                    for z in 0..nz {
                        let o = g.get(x, z);
                        let l = self.live::<T>(t, x, z);
                        if !same_elem(o, l) {
                            bad += 1;
                            if firsts.len() < self.max_cells {
                                firsts.push(format!("t({x},{z}) {}/{}", l.to_f64(), o.to_f64()));
                            }
                        }
                    }
                }
                let cells = (nx as u64) * (nz as u64);
                let acc = &mut self.cycle.grids[i];
                acc.cells += cells;
                acc.mismatches += bad;
                let part = (bad > 0).then(|| {
                    format!(
                        "{} {bad}/{cells} orig/ours [{}]{geom}",
                        GRID_NAMES[i],
                        firsts.join(", ")
                    )
                });
                (cells, bad, part)
            }
            (o, t) => {
                self.cycle.grids[i].mismatches += 1;
                (
                    0,
                    1,
                    Some(format!(
                        "{} present orig/ours={}/{}",
                        GRID_NAMES[i],
                        t != 0,
                        o.is_some()
                    )),
                )
            }
        }
    }

    fn compare(
        &mut self,
        p: &Ready,
        a: &RawRecord,
        scalars: Option<fc4_residential_shadow_scalars>,
        prefix: &str,
        lines: &mut Vec<String>,
    ) -> Result<u64, String> {
        let mut notes: Vec<String> = Vec::new();
        let deferred;
        let run: &Run = match &p.run {
            Some(Ok(r)) => r,
            Some(Err(e)) => return Err(e.clone()),
            None => {
                let (policy, note) = Shadow::policy(p, a);
                notes.extend(note);
                deferred = self.run(p, policy)?;
                &deferred
            }
        };
        notes.extend(run.notes.iter().cloned());
        let strikes_excluded = p.raw.state == st::STRIKES_AND_LISTS;
        if strikes_excluded {
            notes.push("strike rolls not compared".to_string());
        }
        let ours = run.sim.snapshot();
        let mut mismatches = 0u64;
        let mut parts: Vec<String> = Vec::new();

        // The 25 grids, cell by cell (tracts).
        let (mut cells, mut cell_bad) = (0u64, 0u64);
        for i in 0..GRID_COUNT {
            let live = a.grids[i];
            let (c, bad, part) = match grid_at(&ours.grids, i) {
                GridRef::U8(g) => self.compare_grid(i, g, live),
                GridRef::U16(g) => self.compare_grid(i, g, live),
                GridRef::F32(g) => self.compare_grid(i, g, live),
            };
            cells += c;
            cell_bad += bad;
            parts.extend(part);
        }
        mismatches += cell_bad;
        self.stats.cells = self.stats.cells.wrapping_add(cells);
        self.stats.cell_mismatches = self.stats.cell_mismatches.wrapping_add(cell_bad);

        // The saved fields.
        let fields = field_diffs(a, &ours, strikes_excluded);
        if !fields.is_empty() {
            mismatches += fields.len() as u64;
            self.stats.field_mismatches = self.stats.field_mismatches.wrapping_add(fields.len() as u64);
            for (name, _, _) in &fields {
                *self.cycle.fields.entry(name.clone()).or_insert(0) += 1;
            }
            parts.push(format!(
                "fields orig/ours: {}",
                fields
                    .iter()
                    .map(|(n, x, y)| format!("{n}={x}/{y}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        // Occupant properties we wrote, and those of the daily states' written
        // set we only read (the original may have written them).
        let excluded = |id: u32| strikes_excluded && id == PROP_STRIKE_DAY;
        let written: HashMap<(u64, u32), Variant> = run.overlay.iter().cloned().collect();
        let mut keys: Vec<(u64, u32, bool)> = run
            .overlay
            .iter()
            .filter(|(k, _)| !excluded(k.1))
            .map(|(k, v)| (k.0, k.1, matches!(v, Variant::Scalar(Elem::Bool(_)))))
            .collect();
        let tracked = written_props();
        let mut read_keys: Vec<(u64, u32, bool)> = self
            .host
            .cache
            .borrow()
            .keys()
            .filter_map(|k| match *k {
                Key::Prop(o, id) => Some((o, id, false)),
                Key::BoolProp(o, id) => Some((o, id, true)),
                _ => None,
            })
            .filter(|&(o, id, _)| tracked.contains(&id) && !excluded(id) && !written.contains_key(&(o, id)))
            .collect();
        read_keys.sort_unstable();
        read_keys.dedup();
        keys.extend(read_keys);
        let (mut compared, mut bad) = (0u64, Vec::new());
        for (occ, id, is_bool) in keys {
            let to_variant = |b: Option<bool>| b.map(|b| Variant::Scalar(Elem::Bool(b)));
            let ours_v = match written.get(&(occ, id)) {
                Some(v) => Some(v.clone()),
                None => match self.host.cache.borrow().get(&if is_bool {
                    Key::BoolProp(occ, id)
                } else {
                    Key::Prop(occ, id)
                }) {
                    Some(Val::Prop(v)) => v.clone(),
                    Some(Val::Bool(b)) => to_variant(*b),
                    _ => continue,
                },
            };
            let live = if is_bool {
                to_variant(self.host.live_bool_prop(occ, id))
            } else {
                self.host.live_prop(occ, id)
            };
            compared += 1;
            if !same_value(&ours_v, &live) {
                bad.push(format!(
                    "occ {occ:#x} prop {id:#010X} {}/{}",
                    show(&live),
                    show(&ours_v)
                ));
            }
        }
        self.cycle.props.0 += compared;
        self.cycle.props.1 += bad.len() as u64;
        self.stats.prop_mismatches = self.stats.prop_mismatches.wrapping_add(bad.len() as u64);
        let props_part = format!("props {}/{compared}", bad.len());
        if !bad.is_empty() {
            mismatches += bad.len() as u64;
            let n = bad.len();
            bad.truncate(self.max_cells.max(1));
            parts.push(format!("props {n}/{compared} orig/ours [{}]", bad.join(", ")));
        }

        // Local funding we set, and that of the items we read (the original's
        // auto-budget may have set it).
        let mut funding: BTreeMap<(u64, u32), f32> = BTreeMap::new();
        for (k, v) in self.host.cache.borrow().iter() {
            if let (Key::BudgetItem(o, purpose), Val::Item(Some(i))) = (k, v) {
                funding.insert((*o, *purpose), i.local_funding);
            }
        }
        for (k, v) in &run.funding {
            funding.insert(*k, *v);
        }
        let (mut fc, mut fbad) = (0u64, Vec::new());
        for (&(occ, purpose), &ours_f) in &funding {
            let Some(item) = self.host.live_budget_item(occ, purpose) else {
                // An item only read and gone since is not ours to report.
                if run.funding.contains_key(&(occ, purpose)) {
                    fc += 1;
                    fbad.push(format!("occ {occ:#x} purpose {purpose:#010X} no item/{ours_f}"));
                }
                continue;
            };
            fc += 1;
            if !same_f32(item.local_funding, ours_f) {
                fbad.push(format!(
                    "occ {occ:#x} purpose {purpose:#010X} {}/{ours_f}",
                    item.local_funding
                ));
            }
        }
        self.cycle.funding.0 += fc;
        self.cycle.funding.1 += fbad.len() as u64;
        self.stats.prop_mismatches = self.stats.prop_mismatches.wrapping_add(fbad.len() as u64);
        let funding_part = format!("funding {}/{fc}", fbad.len());
        if !fbad.is_empty() {
            mismatches += fbad.len() as u64;
            let n = fbad.len();
            fbad.truncate(self.max_cells.max(1));
            parts.push(format!("funding {n}/{fc} orig/ours [{}]", fbad.join(", ")));
        }

        // The original's getters after its delivery against ours.
        let scalar_part = match scalars {
            Some(s) => {
                let sim = &run.sim;
                let (pop, eq, hq, le) = (
                    sim.get_population(),
                    sim.get_global_eq(),
                    sim.get_global_hq(),
                    u32::from(sim.get_global_le()),
                );
                let checks = [
                    s.population == pop,
                    same_f32(s.global_eq, eq),
                    same_f32(s.global_hq, hq),
                    s.global_le == le,
                ];
                let n = checks.iter().filter(|c| !**c).count() as u64;
                self.cycle.scalars.0 += 4;
                self.cycle.scalars.1 += n;
                self.stats.scalar_mismatches = self.stats.scalar_mismatches.wrapping_add(n);
                mismatches += n;
                format!(
                    "scalars {n}/4 orig/ours population={}/{pop} EQ={}/{eq} HQ={}/{hq} LE={}/{le}",
                    s.population, s.global_eq, s.global_hq, s.global_le
                )
            }
            None => "scalars not read".to_string(),
        };

        let notes = if notes.is_empty() {
            String::new()
        } else {
            format!(" [{}]", notes.join("; "))
        };
        let mut line = format!("{prefix}{notes}: grids {cell_bad}/{cells} cells differ; ");
        for part in &parts {
            line.push_str(part);
            line.push_str("; ");
        }
        line.push_str(&format!(
            "{props_part}; {funding_part}; {scalar_part}; mismatches={mismatches}"
        ));
        lines.push(line);
        Ok(mismatches)
    }

    fn cycle_line(&self, julian: i32) -> String {
        let c = &self.cycle;
        let cells: u64 = c.grids.iter().map(|g| g.cells).sum();
        let bad: u64 = c.grids.iter().map(|g| g.mismatches).sum();
        let grids: Vec<String> = c
            .grids
            .iter()
            .zip(GRID_NAMES)
            .filter(|(g, _)| g.mismatches > 0)
            .map(|(g, n)| format!("{n} {}/{}", g.mismatches, g.cells))
            .collect();
        let fields: Vec<String> = c.fields.iter().map(|(k, v)| format!("{k} x{v}")).collect();
        let none = |v: Vec<String>| if v.is_empty() { "none".to_string() } else { v.join(", ") };
        let mut line = format!(
            "residential shadow cycle to {}: deliveries={} compared={} clean={} incomplete={}; cells={cells} \
             mismatches={bad}; grids with differences: {}; fields: {}; props compared={} mismatched={}; funding \
             compared={} mismatched={}; scalars compared={} mismatched={}",
            date(julian),
            c.deliveries,
            c.compared,
            c.clean,
            c.incomplete,
            none(grids),
            none(fields),
            c.props.0,
            c.props.1,
            c.funding.0,
            c.funding.1,
            c.scalars.0,
            c.scalars.1
        );
        if !c.reasons.is_empty() {
            let r: Vec<String> = c.reasons.iter().map(|(k, v)| format!("{k} x{v}")).collect();
            line.push_str(&format!("; incomplete because: {}", r.join(", ")));
        }
        line
    }
}

fn with_shadow<R>(s: *mut fc4_residential_shadow, fallback: R, f: impl FnOnce(&mut Shadow) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_residential_shadow*.
    match unsafe { (s as *mut Shadow).as_mut() } {
        Some(sh) => guard(fallback, || f(sh)),
        None => fallback,
    }
}

unsafe fn emit(lines: &[String], sink: fc4_bytes_sink, ctx: *mut c_void) {
    if let Some(f) = sink {
        for l in lines {
            f(ctx, l.as_ptr(), l.len());
        }
    }
}

/// Creates a shadow over a read host: the shadow calls only the table's read
/// callbacks (tuning, building map and key, the growth developers' maps,
/// pollution, date, terrain, occupants and their properties, budget items,
/// departments, funding and line items, ordinances) and the grid table's
/// `geometry` / `get`; the write callbacks may be NULL. `max_cells`: the
/// differing cells listed per grid and line (0 → 4, at most 64).
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_shadow_create(
    host: *const fc4_residential_host,
    max_cells: u32,
) -> *mut fc4_residential_shadow {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_residential_host>() && h.grids.complete() => {
            let host = Rc::new(ShadowHost::new(*h));
            let max_cells = match max_cells {
                0 => 4,
                n => n.min(64) as usize,
            };
            Box::into_raw(Box::new(Shadow {
                host,
                max_cells,
                pending: None,
                effects: None,
                cycle: CycleAcc::default(),
                stats: fc4_residential_shadow_stats {
                    struct_size: std::mem::size_of::<fc4_residential_shadow_stats>() as u32,
                    last_mismatches: -1,
                    ..Default::default()
                },
            })) as *mut fc4_residential_shadow
        }
        _ => std::ptr::null_mut(),
    })
}

/// Destroys a shadow (NULL is ignored).
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_shadow_destroy(s: *mut fc4_residential_shadow) {
    if s.is_null() {
        return;
    }
    guard((), || drop(Box::from_raw(s as *mut Shadow)));
}

/// Before the original's SimNewDay delivery (`julian` = the message's
/// data1): copies the grids, loads `before` and runs ours (or defers it to
/// the finish: states 1, 2, 4). 0 = ready; -1 = not run (the finish reports
/// why); -2 = state 10, the original does nothing: nothing to compare (a
/// finish returns -2 and logs nothing).
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_shadow_begin(
    s: *mut fc4_residential_shadow,
    julian: i32,
    before: *const fc4_residential_capture,
) -> i32 {
    let c = capture_of(before);
    with_shadow(s, -1, |sh| sh.begin(julian, c))
}

/// After the original's delivery: compares (see the module notes) and hands
/// the log lines to `sink`, one call per line: one line per delivery
/// (`residential shadow YYYY-MM-DD state N (name): ...; mismatches=N`, or
/// INCOMPLETE and why), and after the delivery that ends a monthly cycle
/// (state 9 → 10) the cycle line. `scalars`: the original's getters read
/// after its delivery (NULL: not compared). Returns the delivery's difference
/// count, -1 when it was not compared, -2 when there was nothing to compare.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_shadow_finish(
    s: *mut fc4_residential_shadow,
    after: *const fc4_residential_capture,
    scalars: *const fc4_residential_shadow_scalars,
    sink: fc4_bytes_sink,
    ctx: *mut c_void,
) -> i32 {
    let c = capture_of(after);
    let sc = scalars
        .as_ref()
        .filter(|x| (x.struct_size as usize) >= std::mem::size_of::<fc4_residential_shadow_scalars>())
        .copied();
    let mut lines = Vec::new();
    let r = with_shadow(s, -1, |sh| sh.finish(c, sc, &mut lines));
    emit(&lines, sink, ctx);
    r
}

/// Totals since creation.
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_shadow_get_stats(
    s: *mut fc4_residential_shadow,
    out: *mut fc4_residential_shadow_stats,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_shadow(s, false, |sh| {
        *o = sh.stats;
        true
    })
}

/// What the shadow cannot read and leaves out of the comparison, as one
/// line to `sink` (logged once at start-up).
#[no_mangle]
pub unsafe extern "C" fn fc4_residential_shadow_describe(sink: fc4_bytes_sink, ctx: *mut c_void) {
    let line = format!("residential shadow: not compared (not readable read-only): {NOT_COMPARED}");
    emit(&[line], sink, ctx);
}

#[cfg(test)]
mod host_tests {
    use super::*;

    unsafe extern "C" fn item(_: *mut c_void, _o: u64, purpose: u32, out: *mut fc4_residential_budget_item) -> bool {
        *out = fc4_residential_budget_item {
            department: 1,
            purpose,
            local_funding: 1.0,
            ..Default::default()
        };
        true
    }

    /// REH §4.2 (RG13, answered): the shadow keeps the local funding it sets
    /// as the item reads it back (float32(float32(q × 100.0f) × 0.01f)), so
    /// 0.43 compares as 0.42999998 and 0.5 survives.
    #[test]
    fn rg13_the_shadow_keeps_the_read_back_local_funding() {
        // SAFETY: every member is an Option of a callback, a pointer or a
        // number: all zero is "no callback".
        let mut c: fc4_residential_host = unsafe { std::mem::zeroed() };
        c.budget_item = Some(item);
        let h = ShadowHost::new(c);
        let purpose = SCHOOL.staff_purpose;
        h.set_local_funding(9, 5, f32::from_bits(0x3EDC_28F6), purpose);
        assert_eq!(h.funding.borrow()[&(5, purpose)].to_bits(), 0x3EDC_28F5);
        let i = h.budget_item(5, purpose).expect("item");
        assert_eq!(i.local_funding.to_bits(), 0x3EDC_28F5);
        h.set_local_funding(9, 5, f32::from_bits(0x3F4C_CCCC), purpose);
        assert_eq!(
            h.budget_item(5, purpose).expect("item").local_funding.to_bits(),
            0x3F4C_CCCB
        );
        h.set_local_funding(9, 5, 0.5, purpose);
        assert_eq!(h.budget_item(5, purpose).expect("item").local_funding, 0.5);
    }

    /// RG1 [choice]: the read-only shadow shows no effect and counts it as
    /// created.
    #[test]
    fn rg1_the_shadow_counts_the_buzzards_as_created() {
        // SAFETY: as above.
        let c: fc4_residential_host = unsafe { std::mem::zeroed() };
        assert_eq!(
            ShadowHost::new(c).create_visual_effect("buzzards"),
            CreatedEffect::NotShown
        );
    }
}
