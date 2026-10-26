// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The pollution grid shadow (fc4.h "pollution shadow"): tooling, no spec
//! behaviour. With the ORIGINAL pollution simulator running, each of its day
//! and month deliveries is wrapped:
//!
//! 1. before: the plugin captures the original's §6 record (its own `Write`
//!    into a capture stream: the bytes and the references, each reference
//!    already queried for the grid, occupant and department interfaces);
//!    [`fc4_pollution_shadow_begin`] copies the live grids the record refers
//!    to (read through the grid table), loads the record into a private core
//!    simulator over those copies and runs FreeCity4's day or month. Every
//!    input is read through the host table as it stands before the original
//!    runs (and remembered); every write is kept inside the shadow (occupant
//!    properties and deal usage in overlays that later reads see; posts,
//!    history, budget and lot calls are only counted; no visual effect is
//!    created; references are not counted);
//! 2. the original's delivery, unchanged;
//! 3. after: the plugin captures the record again;
//!    [`fc4_pollution_shadow_finish`] compares, cell by cell, our eight grids
//!    with the live ones, the saved scalars and lists with the record, and the
//!    occupant properties and deal usage we wrote or read with their live
//!    values. When our run reached the end of a cycle (which reshuffles the
//!    tract iterator with the CRT `rand()`, process-wide in the original) it
//!    runs again from the same start with the original's new order taken from
//!    the record, reading the remembered inputs.
//!
//! Not readable, hence not compared: `radiationSum` (not saved and not
//! touched by `Read`, §6 / PG7; our run reads the record into a fresh object,
//! which starts it at 0, so the radiation history of an end of cycle
//! differs), the posted messages, history and budget values, the visual
//! effects and the worst tracts (not saved; the month recomputes them).

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::grid::{CityDims, GridElem, GridProvider, SimGrid, TractGeometry, TractGrid};
use fc4_core::julian::civil_from_julian;
use fc4_core::pollution::{
    deal, msg, prop, BudgetItem, Deal, PollutionHost, PollutionSim, PollutionState, IID_DEPARTMENT_BUDGET,
    IID_OCCUPANT, IID_SIM_GRID,
};
use fc4_core::property::PropertyHolder;
use fc4_core::rng::FixedSeeds;
use fc4_core::stream::{ByteReader, InStream, RefValue, SaveObject};
use fc4_core::variant::Variant;

use crate::guard;
use crate::hostgrid::{collect_tokens, fc4_grid_api, read_property, take_props};
use crate::landvalue::fc4_grid_geometry;
use crate::pollution::{fc4_pollution_budget_item, fc4_pollution_host};
use crate::stream::fc4_bytes_sink;

/// One reference of a captured record (fc4.h `fc4_pollution_capture_ref`):
/// the object the original wrote, queried for each interface its reader asks
/// for (0 = the object does not answer it). Tokens are borrowed.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_pollution_capture_ref {
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

/// A captured record (fc4.h `fc4_pollution_capture`): the bytes the
/// original's `Write` produced (primitives little-endian, references as
/// `u32 objectID` (index + 1, 0 = null) then `u32 classID`) and its
/// references in write order.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_pollution_capture {
    pub struct_size: u32,
    /// Non-zero when `Write` returned true and the stream stayed good.
    pub ok: u32,
    pub bytes: *const u8,
    pub len: usize,
    pub refs: *const fc4_pollution_capture_ref,
    pub ref_count: usize,
}

/// Totals since creation (fc4.h `fc4_pollution_shadow_stats`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_pollution_shadow_stats {
    pub struct_size: u32,
    /// Deliveries finished (compared or not).
    pub deliveries: u32,
    /// ... compared without a difference.
    pub clean: u32,
    /// ... not compared (a capture failed, our run failed).
    pub incomplete: u32,
    /// Runs repeated with the original's tract order.
    pub reruns: u32,
    /// The last delivery's difference count (-1: not compared).
    pub last_mismatches: i32,
    /// Inputs a repeated run had not read before (read live, after the
    /// original's update).
    pub uncached_reads: u32,
    pub reserved: u32,
    pub cells: u64,
    pub cell_mismatches: u64,
    pub field_mismatches: u64,
    pub prop_mismatches: u64,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_pollution_capture_ref>() == 32);
    assert!(std::mem::size_of::<fc4_pollution_capture>() == if cfg!(target_pointer_width = "32") { 24 } else { 40 });
    assert!(std::mem::size_of::<fc4_pollution_shadow_stats>() == 64);
};

const GRID_NAMES: [&str; 8] = [
    "airSrc",
    "waterSrc",
    "garbageSrc",
    "radSrc",
    "airVal",
    "waterVal",
    "garbageVal",
    "radVal",
];

/// Occupant properties the day and month write (§4.4.5, §4.9.3).
const WRITTEN_PROPS: [u32; 8] = [
    prop::WTP_USED,
    prop::INC_USED,
    prop::WTE_USED,
    prop::WTP_USED_LAST,
    prop::INC_USED_LAST,
    prop::WTE_USED_LAST,
    prop::WTE_OUTPUT,
    prop::CONDITION,
];

/// Day detail lines per month (the month line counts every day).
const DETAIL_DAYS_PER_MONTH: u32 = 8;

// ---- the shadow's host -----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// Before the original's update: live reads, remembered.
    Live,
    /// A repeated run: remembered reads first.
    Replay,
    /// Loading the record captured after the original's update: no grids.
    Parse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Key {
    CellCounts,
    CellWidths,
    PositionToCell(u32, u32),
    Tuning,
    Altitude(u32, u32),
    Zone(i32, i32),
    Traffic(i32, i32),
    Factor(u32, u32),
    Deals(u32),
    Powered(u64),
    Population,
    Landfill,
    Tiles,
    Prop(u64, u32),
    Position(u64),
    BoundingBox(u64),
    Funding(u64),
    BudgetItem(u64, u32),
}

#[derive(Clone)]
enum Val {
    I2(i32, i32),
    F2(f32, f32),
    Tuning(Option<Rc<PropertyHolder>>),
    F(f32),
    U(u32),
    U8(u8),
    Deals(Vec<Deal>),
    B(bool),
    I(i32),
    U2(u32, u32),
    Prop(Option<Variant>),
    V3([f32; 3]),
    Box([f32; 3], [f32; 3]),
    Item(Option<BudgetItem>),
}

/// The shadow's `PollutionHost`: reads go to the C host (remembered), writes
/// stay here.
struct ShadowHost {
    c: fc4_pollution_host,
    mode: Cell<Mode>,
    cache: RefCell<HashMap<Key, Val>>,
    /// Our occupant property writes (None = removed), in first-write order.
    overlay: RefCell<HashMap<(u64, u32), Option<Variant>>>,
    overlay_order: RefCell<Vec<(u64, u32)>>,
    /// Our `SetAmountUsed` values.
    deals: RefCell<BTreeMap<u64, i32>>,
    /// The current run's grid copies by token.
    grids: RefCell<HashMap<u64, Rc<SimGrid<i16>>>>,
    /// Calls kept here: posts, history, budget, lots, deals dirty.
    swallowed: Cell<u32>,
    uncached: Cell<u32>,
}

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the shadow promised valid callbacks.
        match $s.c.$f { Some(f) => unsafe { f($s.c.user $(, $arg)*) }, None => $default }
    };
}

impl ShadowHost {
    fn read<T: Clone>(
        &self,
        key: Key,
        wrap: impl FnOnce(T) -> Val,
        unwrap: impl Fn(&Val) -> Option<T>,
        live: impl FnOnce() -> T,
    ) -> T {
        if self.mode.get() == Mode::Parse {
            return live();
        }
        if let Some(v) = self.cache.borrow().get(&key).and_then(&unwrap) {
            return v;
        }
        if self.mode.get() == Mode::Replay {
            self.uncached.set(self.uncached.get().wrapping_add(1));
            return live();
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

    fn live_deals(&self, deal_type: u32) -> Vec<Deal> {
        collect_tokens(|sink, ctx| cb!(self, find_deals, (), deal_type, sink, ctx))
            .into_iter()
            .map(|token| Deal {
                token,
                amount: cb!(self, deal_amount, 0, token),
                amount_used: cb!(self, deal_amount_used, 0, token),
            })
            .collect()
    }

    fn set_overlay(&self, occ: u64, id: u32, v: Option<Variant>) {
        if self.overlay.borrow_mut().insert((occ, id), v).is_none() {
            self.overlay_order.borrow_mut().push((occ, id));
        }
    }

    /// Starts a run: the overlays are emptied; a live run forgets the
    /// remembered reads, so every delivery reads its inputs afresh (PG9:
    /// landfill usage cannot change within one day pass but can between
    /// them, after zoning, the landfill month or a load).
    fn begin_run(&self, mode: Mode) {
        self.mode.set(mode);
        if mode == Mode::Live {
            self.cache.borrow_mut().clear();
        }
        self.overlay.borrow_mut().clear();
        self.overlay_order.borrow_mut().clear();
        self.deals.borrow_mut().clear();
        self.grids.borrow_mut().clear();
    }
}

impl GridProvider<i16> for ShadowHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<i16>>> {
        let (cells_x, cells_z) = self.cell_counts();
        let (cell_width_x, cell_width_z) = self.cell_widths();
        let city = CityDims {
            cells_x,
            cells_z,
            cell_width_x,
            cell_width_z,
        };
        Some(SimGrid::<i16>::create(&city, instance, tract_size) as Rc<dyn TractGrid<i16>>)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<i16>>> {
        if self.mode.get() == Mode::Parse {
            return None;
        }
        match *r {
            RefValue::Host { token, .. } => self
                .grids
                .borrow()
                .get(&token)
                .map(|g| g.clone() as Rc<dyn TractGrid<i16>>),
            _ => None,
        }
    }
}

impl PollutionHost for ShadowHost {
    fn cell_counts(&self) -> (i32, i32) {
        self.read(
            Key::CellCounts,
            |(a, b)| Val::I2(a, b),
            |v| match v {
                Val::I2(a, b) => Some((*a, *b)),
                _ => None,
            },
            || {
                let (mut x, mut z) = (0, 0);
                cb!(self, cell_counts, (), &mut x, &mut z);
                (x, z)
            },
        )
    }
    fn cell_widths(&self) -> (f32, f32) {
        self.read(
            Key::CellWidths,
            |(a, b)| Val::F2(a, b),
            |v| match v {
                Val::F2(a, b) => Some((*a, *b)),
                _ => None,
            },
            || {
                let (mut x, mut z) = (16.0f32, 16.0f32);
                cb!(self, cell_widths, (), &mut x, &mut z);
                (x, z)
            },
        )
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        self.read(
            Key::PositionToCell(x.to_bits(), z.to_bits()),
            |(a, b)| Val::I2(a, b),
            |v| match v {
                Val::I2(a, b) => Some((*a, *b)),
                _ => None,
            },
            || {
                let (mut cx, mut cz) = (0, 0);
                cb!(self, position_to_cell, (), x, z, &mut cx, &mut cz);
                (cx, cz)
            },
        )
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.read(
            Key::Tuning,
            Val::Tuning,
            |v| match v {
                Val::Tuning(t) => Some(t.clone()),
                _ => None,
            },
            || {
                let p = cb!(self, tuning, std::ptr::null_mut());
                // SAFETY: the host hands over a NEW fc4_props (or NULL).
                unsafe { take_props(p) }
            },
        )
    }
    fn altitude(&self, x: f32, z: f32) -> f32 {
        self.read(
            Key::Altitude(x.to_bits(), z.to_bits()),
            Val::F,
            |v| match v {
                Val::F(f) => Some(*f),
                _ => None,
            },
            || cb!(self, altitude, 0.0, x, z),
        )
    }
    fn add_agent(&self, _level: u32, _name: &str, _flags: u32) {
        self.swallow();
    }
    fn remove_agent(&self) {
        self.swallow();
    }
    fn subscribe(&self, _msg_types: &[u32]) {
        self.swallow();
    }
    fn unsubscribe(&self, _msg_types: &[u32]) {
        self.swallow();
    }
    fn post(&self, _msg_type: u32, _data1: u32) {
        self.swallow();
    }
    fn record_history(&self, _series: u32, _value: i32) {
        self.swallow();
    }
    fn set_history_queue_length(&self, _series: u32, _length: u32) {
        self.swallow();
    }
    fn add_prop_requester(&self) {
        self.swallow();
    }
    fn remove_prop_requester(&self) {
        self.swallow();
    }
    fn zone_type(&self, x: i32, z: i32) -> u32 {
        self.read(
            Key::Zone(x, z),
            Val::U,
            |v| match v {
                Val::U(u) => Some(*u),
                _ => None,
            },
            || cb!(self, zone_type, 0, x, z),
        )
    }
    fn air_traffic(&self, x: i32, z: i32) -> u8 {
        self.read(
            Key::Traffic(x, z),
            Val::U8,
            |v| match v {
                Val::U8(u) => Some(*u),
                _ => None,
            },
            || cb!(self, air_traffic, 0, x, z),
        )
    }
    fn cumulative_factor(&self, id: u32, index: u32) -> f32 {
        self.read(
            Key::Factor(id, index),
            Val::F,
            |v| match v {
                Val::F(f) => Some(*f),
                _ => None,
            },
            || cb!(self, cumulative_factor, 1.0, id, index),
        )
    }
    fn find_deals(&self, deal_type: u32) -> Vec<Deal> {
        let mut deals = self.read(
            Key::Deals(deal_type),
            Val::Deals,
            |v| match v {
                Val::Deals(d) => Some(d.clone()),
                _ => None,
            },
            || self.live_deals(deal_type),
        );
        let ours = self.deals.borrow();
        for d in deals.iter_mut() {
            if let Some(&used) = ours.get(&d.token) {
                d.amount_used = used;
            }
        }
        deals
    }
    fn set_deal_amount_used(&self, deal: u64, used: i32) {
        self.deals.borrow_mut().insert(deal, used);
    }
    fn mark_deals_dirty(&self) {
        self.swallow();
    }
    fn is_occupant_powered(&self, occ: u64) -> bool {
        self.read(
            Key::Powered(occ),
            Val::B,
            |v| match v {
                Val::B(b) => Some(*b),
                _ => None,
            },
            || cb!(self, is_occupant_powered, false, occ),
        )
    }
    fn population(&self) -> i32 {
        self.read(
            Key::Population,
            Val::I,
            |v| match v {
                Val::I(i) => Some(*i),
                _ => None,
            },
            || cb!(self, population, 0),
        )
    }
    fn landfill_usage(&self) -> (u32, u32) {
        self.read(
            Key::Landfill,
            |(a, b)| Val::U2(a, b),
            |v| match v {
                Val::U2(a, b) => Some((*a, *b)),
                _ => None,
            },
            || {
                let (mut usage, mut capacity) = (0u32, 0u32);
                cb!(self, landfill_usage, (), &mut usage, &mut capacity);
                (usage, capacity)
            },
        )
    }
    fn landfill_tile_count(&self) -> u32 {
        self.read(
            Key::Tiles,
            Val::U,
            |v| match v {
                Val::U(u) => Some(*u),
                _ => None,
            },
            || cb!(self, landfill_tile_count, 0),
        )
    }
    fn set_lot_condition(&self, _cell_x: i32, _cell_z: i32, _condition: u32) {
        self.swallow();
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant> {
        if let Some(v) = self.overlay.borrow().get(&(occ, id)) {
            return v.clone();
        }
        self.read(
            Key::Prop(occ, id),
            Val::Prop,
            |v| match v {
                Val::Prop(p) => Some(p.clone()),
                _ => None,
            },
            || self.live_prop(occ, id),
        )
    }
    fn set_occupant_property(&self, occ: u64, id: u32, value: Variant) {
        self.set_overlay(occ, id, Some(value));
    }
    fn remove_occupant_property(&self, occ: u64, id: u32) {
        self.set_overlay(occ, id, None);
    }
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        self.read(
            Key::Position(occ),
            Val::V3,
            |v| match v {
                Val::V3(p) => Some(*p),
                _ => None,
            },
            || {
                let mut p = [0.0f32; 3];
                cb!(self, occupant_position, (), occ, p.as_mut_ptr());
                p
            },
        )
    }
    fn occupant_bounding_box(&self, occ: u64) -> ([f32; 3], [f32; 3]) {
        self.read(
            Key::BoundingBox(occ),
            |(a, b)| Val::Box(a, b),
            |v| match v {
                Val::Box(a, b) => Some((*a, *b)),
                _ => None,
            },
            || {
                let (mut a, mut b) = ([0.0f32; 3], [0.0f32; 3]);
                cb!(self, occupant_bounding_box, (), occ, a.as_mut_ptr(), b.as_mut_ptr());
                (a, b)
            },
        )
    }
    fn add_ref(&self, _token: u64) {}
    fn release(&self, _token: u64) {}
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
    fn create_department(&self, _id: u32, _group: u32) -> Option<u64> {
        None
    }
    fn set_department_name(&self, _dept: u64, _group: u32, _instance: u32) {
        self.swallow();
    }
    fn set_department_max_funding(&self, _dept: u64, _fraction: f32) {
        self.swallow();
    }
    fn create_line_item(&self, _d: u64, _l: u32, _lf: bool, _g: u32, _i: u32) {
        self.swallow();
    }
    fn department_funding(&self, dept: u64) -> f32 {
        self.read(
            Key::Funding(dept),
            Val::F,
            |v| match v {
                Val::F(f) => Some(*f),
                _ => None,
            },
            || cb!(self, department_funding, 1.0, dept),
        )
    }
    fn set_line_item_full_expenses(&self, _dept: u64, _line: u32, _value: i64) {
        self.swallow();
    }
    fn budget_item_for_purpose(&self, occ: u64, purpose: u32) -> Option<BudgetItem> {
        self.read(
            Key::BudgetItem(occ, purpose),
            Val::Item,
            |v| match v {
                Val::Item(i) => Some(*i),
                _ => None,
            },
            || {
                let mut i = fc4_pollution_budget_item {
                    purpose,
                    ..Default::default()
                };
                cb!(self, budget_item_for_purpose, false, occ, purpose, &mut i).then(|| BudgetItem::from(&i))
            },
        )
    }
    fn add_budget_item_to_budget(&self, _occ: u64, _index: u32) {
        self.swallow();
    }
    fn remove_budget_item_from_budget(&self, _occ: u64, _index: u32) {
        self.swallow();
    }
    fn change_budget_item_cost(&self, _occ: u64, _index: u32, _cost: i64) {
        self.swallow();
    }
    fn add_to_budget_item_line_expenses(&self, _item: &BudgetItem, _delta: i64) {
        self.swallow();
    }
    fn create_visual_effect(&self, _id: u32, _kind: u32) -> Option<u64> {
        None
    }
    fn set_effect_position(&self, _effect: u64, _position: [f32; 3]) {}
    fn set_effect_param(&self, _effect: u64, _param: u32, _value: f32) {}
    fn start_effect(&self, _effect: u64, _t: u32) {}
    fn stop_effect(&self, _effect: u64, _t: u32) {}
    fn is_effect_started(&self, _effect: u64) -> bool {
        false
    }
}

// ---- the captured record ---------------------------------------------------------------------------

/// A captured record as a core input stream: references are answered with
/// the token of the interface the reader asks for.
struct CaptureReader<'a> {
    r: ByteReader<'a>,
    refs: &'a [fc4_pollution_capture_ref],
    /// The grid references in read order (0 = null or not a grid).
    grid_tokens: Vec<u64>,
}

impl<'a> CaptureReader<'a> {
    fn new(bytes: &'a [u8], refs: &'a [fc4_pollution_capture_ref]) -> Self {
        CaptureReader {
            r: ByteReader::new(bytes),
            refs,
            grid_tokens: Vec::new(),
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
        let token = if id == 0 {
            0
        } else {
            let class_id = self.r.get_u32()?;
            let Some(e) = self.refs.get(id as usize - 1) else {
                self.r.set_error();
                return None;
            };
            let t = match iid {
                IID_SIM_GRID => e.grid,
                IID_OCCUPANT => e.occupant,
                IID_DEPARTMENT_BUDGET => e.department,
                _ => 0,
            };
            if t != 0 {
                if iid == IID_SIM_GRID {
                    self.grid_tokens.push(t);
                }
                return Some(RefValue::Host { token: t, class_id });
            }
            0
        };
        if iid == IID_SIM_GRID {
            self.grid_tokens.push(token);
        }
        // §2.7 helper semantics: an object without the interface reads as null.
        Some(RefValue::Null)
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

/// A capture copied out of the caller's buffers.
#[derive(Clone, Default)]
struct Capture {
    bytes: Vec<u8>,
    refs: Vec<fc4_pollution_capture_ref>,
}

/// Copies a capture; `Err` with the reason when it is not usable.
unsafe fn capture_of(c: *const fc4_pollution_capture) -> Result<Capture, &'static str> {
    let Some(c) = c.as_ref() else {
        return Err("no capture");
    };
    if (c.struct_size as usize) < std::mem::size_of::<fc4_pollution_capture>() {
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

// ---- the shadow -------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_pollution_shadow`).
#[repr(C)]
pub struct fc4_pollution_shadow {
    _private: [u8; 0],
}

/// A live grid as it stood before the original's update.
struct BaseGrid {
    token: u64,
    geom: TractGeometry,
    values: Vec<i16>,
}

/// One run of ours.
struct Run {
    state: PollutionState,
    grids: Vec<Option<Rc<SimGrid<i16>>>>,
    cycle_resets: u32,
    overlay: Vec<((u64, u32), Option<Variant>)>,
    deals: BTreeMap<u64, i32>,
}

/// The delivery between begin and finish.
struct Pending {
    msg_type: u32,
    julian: i32,
    before: Capture,
    base: Vec<BaseGrid>,
    run: Result<Run, String>,
}

#[derive(Default, Clone)]
struct GridAcc {
    cells: u64,
    mismatches: u64,
    max_diff: i32,
}

#[derive(Default)]
struct MonthAcc {
    deliveries: u32,
    days: u32,
    months: u32,
    clean: u32,
    incomplete: u32,
    reruns: u32,
    uncached: u32,
    grids: [GridAcc; 8],
    first: Vec<String>,
    fields: BTreeMap<&'static str, u32>,
    props: (u64, u64),
    deals: (u64, u64),
    detail_days: u32,
    reasons: BTreeMap<String, u32>,
}

struct Shadow {
    host: Rc<ShadowHost>,
    max_cells: usize,
    pending: Option<Pending>,
    month: MonthAcc,
    stats: fc4_pollution_shadow_stats,
}

fn date(julian: i32) -> String {
    let d = civil_from_julian(julian);
    format!("{:04}-{:02}-{:02}", d.year, d.month, d.day)
}

fn kind(msg_type: u32) -> &'static str {
    if msg_type == msg::SIM_NEW_MONTH {
        "month"
    } else {
        "day"
    }
}

/// The numbers of a variant, for comparing values whose element types may
/// differ between our write and the game's property.
fn numbers(v: &Option<Variant>) -> Option<Vec<f64>> {
    v.as_ref().map(|v| v.items().iter().map(|e| e.as_f64()).collect())
}

fn show(v: &Option<Variant>) -> String {
    match numbers(v) {
        None => "absent".to_string(),
        Some(n) if n.len() == 1 => format!("{}", n[0]),
        Some(n) => format!("{n:?}"),
    }
}

/// The saved fields that differ: (name, original, ours).
fn field_diffs(o: &PollutionState, b: &PollutionState) -> Vec<(&'static str, String, String)> {
    let mut v: Vec<(&'static str, String, String)> = Vec::new();
    macro_rules! f {
        ($($name:literal => $($e:ident).+),* $(,)?) => {$(
            if o.$($e).+ != b.$($e).+ {
                v.push(($name, format!("{:?}", b.$($e).+), format!("{:?}", o.$($e).+)));
            }
        )*};
    }
    macro_rules! fb {
        ($($name:literal => $($e:ident).+),* $(,)?) => {$(
            if o.$($e).+.to_bits() != b.$($e).+.to_bits() {
                v.push(($name, format!("{:?}", b.$($e).+), format!("{:?}", o.$($e).+)));
            }
        )*};
    }
    f!(
        "cellX" => cell_x, "cellZ" => cell_z, "recyclingCount" => recycling_count, "avgAir" => avg_air,
        "avgWater" => avg_water, "airSum" => air_sum, "waterSumTreated" => water_sum_treated,
        "waterSum" => water_sum, "treatedSum" => treated_sum, "lastWaterSum" => last_water_sum,
        "lastTreatedSum" => last_treated_sum, "developedTracts" => developed_tracts, "avgGarbage" => avg_garbage,
        "incineratorCapacity" => incinerator_capacity, "wteCapacity" => wte_capacity,
        "garbageUncollected" => garbage_uncollected, "totalUncollected" => total_uncollected,
        "unused64" => unused64, "lastMonthCost" => last_month_cost, "decomposedLastMonth" => decomposed_last_month,
        "lastMonth.produced" => last_month.produced, "lastMonth.imported" => last_month.imported,
        "lastMonth.exported" => last_month.exported, "lastMonth.incinerated" => last_month.incinerated,
        "lastMonth.wte" => last_month.wte, "lastMonth.landfilled" => last_month.landfilled,
        "month.produced" => month.produced, "month.imported" => month.imported,
        "month.exported" => month.exported, "month.incinerated" => month.incinerated,
        "month.wte" => month.wte, "month.landfilled" => month.landfilled,
        "decomposed" => decomposed, "exportCapacity" => export_capacity,
        "activeLandfillCells" => active_landfill_cells, "tractsPerDay" => tracts_per_day,
        "dirtyRadiation" => dirty_radiation, "dirtyAir" => dirty_air, "dirtyWater" => dirty_water,
        "dirtyGarbage" => dirty_garbage, "garbageSupply" => garbage_supply, "garbageCapacity" => garbage_capacity,
        "department" => department, "lists" => lists,
    );
    fb!(
        "recyclingEffect" => recycling_effect, "efficiency" => efficiency,
        "lastMonth.recycled" => last_month.recycled, "month.recycled" => month.recycled,
    );
    if o.iterator != b.iterator {
        let it = |s: &PollutionState| {
            format!(
                "state {} index {} current {} count {} table {}",
                s.iterator.state_raw,
                s.iterator.index,
                s.iterator.current,
                s.iterator.count,
                if s.iterator.table == o.iterator.table {
                    "same"
                } else {
                    "differs"
                }
            )
        };
        v.push(("iterator", it(b), it(o)));
    }
    v
}

impl Shadow {
    fn api(&self) -> &fc4_grid_api {
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

    fn live_value(&self, token: u64, tx: i32, tz: i32) -> i16 {
        match self.api().get {
            // SAFETY: valid callback on a live game grid.
            Some(f) => i16::from_f64(unsafe { f(self.api().user, token, i16::CLSID, tx, tz) }),
            None => 0,
        }
    }

    /// The live grid's values, x outer.
    fn copy(&self, token: u64) -> BaseGrid {
        let geom = self.geometry(token);
        let (nx, nz) = (geom.count_x.clamp(0, 4096), geom.count_z.clamp(0, 4096));
        let mut values = Vec::with_capacity((nx * nz) as usize);
        for x in 0..nx {
            for z in 0..nz {
                values.push(self.live_value(token, x, z));
            }
        }
        BaseGrid { token, geom, values }
    }

    /// Runs ours on the record captured before the update.
    fn run(&self, p: &Pending, mode: Mode, order: Option<Vec<u16>>) -> Result<Run, String> {
        let h = &self.host;
        h.begin_run(mode);
        let grids: Vec<Option<Rc<SimGrid<i16>>>> = p
            .base
            .iter()
            .map(|b| (b.token != 0).then(|| SimGrid::from_values(b.geom, &b.values)))
            .collect();
        for (b, g) in p.base.iter().zip(&grids) {
            if let Some(g) = g {
                h.grids.borrow_mut().insert(b.token, g.clone());
            }
        }
        let seeds = FixedSeeds {
            wall_clock: 1,
            process_clock: 1,
        };
        let sim = PollutionSim::new(h.clone() as Rc<dyn PollutionHost>, 0, 1, &seeds);
        sim.reload_tuning();
        let mut r = CaptureReader::new(&p.before.bytes, &p.before.refs);
        if !sim.read(&mut r) {
            return Err("our Read of the original's record failed".to_string());
        }
        if order.is_some() {
            sim.set_shuffle_override(order);
        }
        sim.do_message(p.msg_type, 0, 0);
        let overlay = {
            let o = h.overlay.borrow();
            h.overlay_order
                .borrow()
                .iter()
                .map(|k| (*k, o.get(k).cloned().flatten()))
                .collect()
        };
        Ok(Run {
            state: sim.snapshot(),
            grids,
            cycle_resets: sim.cycle_resets(),
            overlay,
            deals: h.deals.borrow().clone(),
        })
    }

    fn begin(&mut self, msg_type: u32, julian: i32, before: Result<Capture, &'static str>) -> i32 {
        self.pending = None;
        let before = match before {
            Ok(c) => c,
            Err(e) => {
                self.pending = Some(Pending {
                    msg_type,
                    julian,
                    before: Capture::default(),
                    base: Vec::new(),
                    run: Err(format!("before: {e}")),
                });
                return -1;
            }
        };
        // The eight grid references come first (§6): their tokens, in order.
        let mut probe = CaptureReader::new(&before.bytes, &before.refs);
        let ok = probe.get_u16().is_some() && (0..8).all(|_| probe.get_ref_as(IID_SIM_GRID).is_some());
        let tokens = probe.grid_tokens.clone();
        let base: Vec<BaseGrid> = tokens
            .iter()
            .map(|&t| {
                if t == 0 {
                    BaseGrid {
                        token: 0,
                        geom: TractGeometry::default(),
                        values: Vec::new(),
                    }
                } else {
                    self.copy(t)
                }
            })
            .collect();
        let mut p = Pending {
            msg_type,
            julian,
            before,
            base,
            run: Err(String::new()),
        };
        p.run = if ok && tokens.len() == 8 {
            self.run(&p, Mode::Live, None)
        } else {
            Err("before: the record has no grid references".to_string())
        };
        let r = if p.run.is_ok() { 0 } else { -1 };
        self.pending = Some(p);
        r
    }

    /// The record captured after the update, loaded without grids, and the
    /// grid tokens it refers to.
    fn parse_after(&self, after: &Capture) -> Result<(PollutionState, Vec<u64>), String> {
        let h = &self.host;
        h.mode.set(Mode::Parse);
        let seeds = FixedSeeds {
            wall_clock: 1,
            process_clock: 1,
        };
        let sim = PollutionSim::new(h.clone() as Rc<dyn PollutionHost>, 0, 1, &seeds);
        let mut r = CaptureReader::new(&after.bytes, &after.refs);
        let ok = sim.read(&mut r);
        h.mode.set(Mode::Live);
        if !ok || r.grid_tokens.len() != 8 {
            return Err("after: our Read of the original's record failed".to_string());
        }
        Ok((sim.snapshot(), r.grid_tokens))
    }

    fn finish(&mut self, after: Result<Capture, &'static str>, lines: &mut Vec<String>) -> i32 {
        let Some(p) = self.pending.take() else {
            return -1;
        };
        self.stats.deliveries = self.stats.deliveries.wrapping_add(1);
        self.month.deliveries += 1;
        if p.msg_type == msg::SIM_NEW_MONTH {
            self.month.months += 1;
        } else {
            self.month.days += 1;
        }
        let prefix = format!("pollution shadow {} {}", date(p.julian), kind(p.msg_type));
        let result = self.compare(&p, after, &prefix, lines);
        let r = match result {
            Ok(n) => {
                if n == 0 {
                    self.stats.clean = self.stats.clean.wrapping_add(1);
                    self.month.clean += 1;
                }
                n.min(i32::MAX as u64) as i32
            }
            Err(e) => {
                self.stats.incomplete = self.stats.incomplete.wrapping_add(1);
                self.month.incomplete += 1;
                *self.month.reasons.entry(e.clone()).or_insert(0) += 1;
                if self.month.incomplete <= 3 {
                    lines.push(format!("{prefix}: INCOMPLETE ({e}); not compared"));
                }
                -1
            }
        };
        self.stats.last_mismatches = r;
        if p.msg_type == msg::SIM_NEW_MONTH {
            lines.push(self.month_line(p.julian));
            self.month = MonthAcc::default();
        }
        r
    }

    fn compare(
        &mut self,
        p: &Pending,
        after: Result<Capture, &'static str>,
        prefix: &str,
        lines: &mut Vec<String>,
    ) -> Result<u64, String> {
        if let Err(e) = &p.run {
            return Err(e.clone());
        }
        let after = after.map_err(|e| format!("after: {e}"))?;
        let (orig, after_tokens) = self.parse_after(&after)?;
        // A reshuffle at the end of a cycle: run again with the original's order.
        let mut notes: Vec<String> = Vec::new();
        let first = p.run.as_ref().map_err(|e| e.clone())?;
        let rerun;
        let ours: &Run = if first.cycle_resets > 0 {
            let before_uncached = self.host.uncached.get();
            rerun = self.run(p, Mode::Replay, Some(orig.iterator.table.clone()))?;
            self.host.mode.set(Mode::Live);
            self.stats.reruns = self.stats.reruns.wrapping_add(1);
            self.month.reruns += 1;
            let uncached = self.host.uncached.get().wrapping_sub(before_uncached);
            self.stats.uncached_reads = self.stats.uncached_reads.wrapping_add(uncached);
            self.month.uncached += uncached;
            notes.push(format!(
                "end of cycle: replayed with the original's tract order ({uncached} inputs read after its update)"
            ));
            &rerun
        } else {
            first
        };
        let mut mismatches = 0u64;
        let mut parts: Vec<String> = Vec::new();
        // Grids, cell by cell (tracts).
        for i in 0..8 {
            let ours_g = ours.grids.get(i).and_then(|g| g.as_ref());
            let live = after_tokens[i];
            match (ours_g, live) {
                (None, 0) => continue,
                (Some(g), t) if t != 0 => {
                    let geom = g.geometry();
                    let (nx, nz) = (geom.count_x.clamp(0, 4096), geom.count_z.clamp(0, 4096));
                    let values = g.values();
                    let (mut n, mut max) = (0u64, 0i32);
                    let mut firsts: Vec<String> = Vec::new();
                    for x in 0..nx {
                        for z in 0..nz {
                            let o = values[(x * nz + z) as usize];
                            let l = self.live_value(t, x, z);
                            if o != l {
                                n += 1;
                                let d = (i32::from(o) - i32::from(l)).abs();
                                max = max.max(d);
                                if firsts.len() < self.max_cells {
                                    firsts.push(format!("t({x},{z}) {l}/{o}"));
                                }
                                if self.month.first.len() < self.max_cells * 2 {
                                    self.month.first.push(format!(
                                        "{} {} t({x},{z}) orig/ours={l}/{o}",
                                        GRID_NAMES[i],
                                        date(p.julian)
                                    ));
                                }
                            }
                        }
                    }
                    let cells = (nx as u64) * (nz as u64);
                    let acc = &mut self.month.grids[i];
                    acc.cells += cells;
                    acc.mismatches += n;
                    acc.max_diff = acc.max_diff.max(max);
                    self.stats.cells = self.stats.cells.wrapping_add(cells);
                    self.stats.cell_mismatches = self.stats.cell_mismatches.wrapping_add(n);
                    if n > 0 {
                        mismatches += n;
                        parts.push(format!(
                            "{} {n}/{cells} max|d|={max} orig/ours [{}]",
                            GRID_NAMES[i],
                            firsts.join(", ")
                        ));
                    }
                }
                _ => {
                    mismatches += 1;
                    parts.push(format!(
                        "{} present orig/ours={}/{}",
                        GRID_NAMES[i],
                        live != 0,
                        ours_g.is_some()
                    ));
                }
            }
        }
        // Saved fields.
        let fields = field_diffs(&ours.state, &orig);
        if !fields.is_empty() {
            mismatches += fields.len() as u64;
            self.stats.field_mismatches = self.stats.field_mismatches.wrapping_add(fields.len() as u64);
            for (name, _, _) in &fields {
                *self.month.fields.entry(name).or_insert(0) += 1;
            }
            parts.push(format!(
                "fields orig/ours: {}",
                fields
                    .iter()
                    .map(|(n, a, b)| format!("{n}={a}/{b}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        // Occupant properties we wrote, or read and left (the original may
        // have written them), against their live values.
        let mut keys: Vec<(u64, u32)> = ours.overlay.iter().map(|(k, _)| *k).collect();
        let written: HashMap<(u64, u32), Option<Variant>> = ours.overlay.iter().cloned().collect();
        let mut read_keys: Vec<(u64, u32)> = self
            .host
            .cache
            .borrow()
            .keys()
            .filter_map(|k| match *k {
                Key::Prop(o, id) if WRITTEN_PROPS.contains(&id) && !written.contains_key(&(o, id)) => Some((o, id)),
                _ => None,
            })
            .collect();
        read_keys.sort_unstable();
        keys.extend(read_keys);
        let (mut compared, mut bad) = (0u64, Vec::new());
        for k in keys {
            let ours_v = match written.get(&k) {
                Some(v) => v.clone(),
                None => match self.host.cache.borrow().get(&Key::Prop(k.0, k.1)) {
                    Some(Val::Prop(v)) => v.clone(),
                    _ => continue,
                },
            };
            let live = self.host.live_prop(k.0, k.1);
            compared += 1;
            if numbers(&ours_v) != numbers(&live) {
                bad.push(format!(
                    "occ {:#x} prop {:#010X} {}/{}",
                    k.0,
                    k.1,
                    show(&live),
                    show(&ours_v)
                ));
            }
        }
        self.month.props.0 += compared;
        self.month.props.1 += bad.len() as u64;
        self.stats.prop_mismatches = self.stats.prop_mismatches.wrapping_add(bad.len() as u64);
        if !bad.is_empty() {
            mismatches += bad.len() as u64;
            let n = bad.len();
            bad.truncate(self.max_cells.max(1));
            parts.push(format!("props {n}/{compared} orig/ours [{}]", bad.join(", ")));
        }
        // Deal usage.
        let mut deals: BTreeMap<u64, i32> = BTreeMap::new();
        for t in [deal::IMPORT, deal::EXPORT] {
            if let Some(Val::Deals(list)) = self.host.cache.borrow().get(&Key::Deals(t)) {
                for d in list {
                    deals.insert(d.token, d.amount_used);
                }
            }
        }
        for (&t, &u) in &ours.deals {
            deals.insert(t, u);
        }
        let (mut dc, mut dbad) = (0u64, Vec::new());
        for (&t, &u) in &deals {
            let live = match self.host.c.deal_amount_used {
                // SAFETY: valid callback on a live deal.
                Some(f) => unsafe { f(self.host.c.user, t) },
                None => continue,
            };
            dc += 1;
            if live != u {
                dbad.push(format!("deal {t:#x} used {live}/{u}"));
            }
        }
        self.month.deals.0 += dc;
        self.month.deals.1 += dbad.len() as u64;
        if !dbad.is_empty() {
            mismatches += dbad.len() as u64;
            parts.push(format!("deals orig/ours [{}]", dbad.join(", ")));
        }
        if mismatches > 0 && self.month.detail_days < DETAIL_DAYS_PER_MONTH {
            self.month.detail_days += 1;
            notes.extend(parts);
            lines.push(format!("{prefix}: mismatches={mismatches}; {}", notes.join("; ")));
        } else if !notes.is_empty() && mismatches == 0 && self.month.detail_days < DETAIL_DAYS_PER_MONTH {
            self.month.detail_days += 1;
            lines.push(format!("{prefix}: clean; {}", notes.join("; ")));
        }
        Ok(mismatches)
    }

    fn month_line(&self, julian: i32) -> String {
        let m = &self.month;
        let cells: u64 = m.grids.iter().map(|g| g.cells).sum();
        let bad: u64 = m.grids.iter().map(|g| g.mismatches).sum();
        let per_grid: Vec<String> = m
            .grids
            .iter()
            .zip(GRID_NAMES)
            .map(|(g, n)| format!("{n} {}/{} max|d|={}", g.mismatches, g.cells, g.max_diff))
            .collect();
        let fields = if m.fields.is_empty() {
            "none".to_string()
        } else {
            m.fields
                .iter()
                .map(|(k, v)| format!("{k} x{v}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let first = if m.first.is_empty() {
            "none".to_string()
        } else {
            m.first.join(", ")
        };
        let mut line = format!(
            "pollution shadow month to {}: deliveries={} (days {}, months {}) clean={} incomplete={} reruns={} \
             uncached={}; cells={} mismatches={}; {}; first differing cells: {}; fields: {}; props compared={} \
             mismatched={}; deals compared={} mismatched={}",
            date(julian),
            m.deliveries,
            m.days,
            m.months,
            m.clean,
            m.incomplete,
            m.reruns,
            m.uncached,
            cells,
            bad,
            per_grid.join(", "),
            first,
            fields,
            m.props.0,
            m.props.1,
            m.deals.0,
            m.deals.1
        );
        if !m.reasons.is_empty() {
            let r: Vec<String> = m.reasons.iter().map(|(k, v)| format!("{k} x{v}")).collect();
            line.push_str(&format!("; incomplete because: {}", r.join(", ")));
        }
        line
    }
}

fn with_shadow<R>(s: *mut fc4_pollution_shadow, fallback: R, f: impl FnOnce(&mut Shadow) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_pollution_shadow*.
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

/// Creates a shadow over a read host: the shadow only calls the table's read
/// callbacks (city, tuning, terrain, zones, traffic, ordinances, deals and
/// their amounts, power, population, landfill, occupant properties, position
/// and bounding box, department funding, budget items) and the grid table's
/// `geometry` / `get`; the write callbacks may be NULL. `max_cells`: the
/// differing cells listed per grid and line (0 → 4, at most 64).
#[no_mangle]
pub unsafe extern "C" fn fc4_pollution_shadow_create(
    host: *const fc4_pollution_host,
    max_cells: u32,
) -> *mut fc4_pollution_shadow {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_pollution_host>() && h.grids.complete() => {
            let host = Rc::new(ShadowHost {
                c: crate::trace::pollution_create(*h, max_cells),
                mode: Cell::new(Mode::Live),
                cache: RefCell::new(HashMap::new()),
                overlay: RefCell::new(HashMap::new()),
                overlay_order: RefCell::new(Vec::new()),
                deals: RefCell::new(BTreeMap::new()),
                grids: RefCell::new(HashMap::new()),
                swallowed: Cell::new(0),
                uncached: Cell::new(0),
            });
            let max_cells = match max_cells {
                0 => 4,
                n => n.min(64) as usize,
            };
            Box::into_raw(Box::new(Shadow {
                host,
                max_cells,
                pending: None,
                month: MonthAcc::default(),
                stats: fc4_pollution_shadow_stats {
                    struct_size: std::mem::size_of::<fc4_pollution_shadow_stats>() as u32,
                    last_mismatches: -1,
                    ..Default::default()
                },
            })) as *mut fc4_pollution_shadow
        }
        _ => std::ptr::null_mut(),
    })
}

/// Destroys a shadow (NULL is ignored).
#[no_mangle]
pub unsafe extern "C" fn fc4_pollution_shadow_destroy(s: *mut fc4_pollution_shadow) {
    if s.is_null() {
        return;
    }
    crate::trace::will_destroy("pollution");
    guard((), || drop(Box::from_raw(s as *mut Shadow)));
    crate::trace::destroy("pollution");
}

/// Before the original's delivery of `msg_type` (SimNewDay `0x66956814` or
/// SimNewMonth `0x66956816`; `julian` = the message's data1): copies the
/// grids, loads `before` and runs ours. 0 = ran; -1 = not run (the reason is
/// reported by the finish). Other message types: -2, nothing done.
#[no_mangle]
pub unsafe extern "C" fn fc4_pollution_shadow_begin(
    s: *mut fc4_pollution_shadow,
    msg_type: u32,
    julian: i32,
    before: *const fc4_pollution_capture,
) -> i32 {
    if msg_type != msg::SIM_NEW_DAY && msg_type != msg::SIM_NEW_MONTH {
        return -2;
    }
    crate::trace::pollution_step("begin", msg_type, julian, before);
    let c = capture_of(before);
    let r = with_shadow(s, -1, |sh| sh.begin(msg_type, julian, c));
    crate::trace::step_result("pollution", "begin", r, &[]);
    r
}

/// After the original's delivery: compares (see the module notes) and hands
/// the log lines to `sink`, one call per line: a line for a delivery with a
/// difference (at most 8 per month) or a replay, and after a SimNewMonth the
/// month line. Returns the delivery's difference count, -1 when it was not
/// compared.
#[no_mangle]
pub unsafe extern "C" fn fc4_pollution_shadow_finish(
    s: *mut fc4_pollution_shadow,
    after: *const fc4_pollution_capture,
    sink: fc4_bytes_sink,
    ctx: *mut c_void,
) -> i32 {
    crate::trace::pollution_step("finish", 0, 0, after);
    let c = capture_of(after);
    let mut lines = Vec::new();
    let r = with_shadow(s, -1, |sh| sh.finish(c, &mut lines));
    crate::trace::step_result("pollution", "finish", r, &lines);
    emit(&lines, sink, ctx);
    r
}

/// Totals since creation.
#[no_mangle]
pub unsafe extern "C" fn fc4_pollution_shadow_get_stats(
    s: *mut fc4_pollution_shadow,
    out: *mut fc4_pollution_shadow_stats,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_shadow(s, false, |sh| {
        *o = sh.stats;
        true
    })
}
