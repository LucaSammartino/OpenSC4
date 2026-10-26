// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The plumbing shadow (fc4.h "plumbing shadow"): tooling, no spec
//! behaviour. With the ORIGINAL plumbing simulator running, every delivery
//! of the clock to its agent (SimIdle, SimNewDay: `DoIdle`, one pass state
//! per call, §6; SimNewMonth, §7.1) is wrapped by
//! [`fc4_plumbing_shadow_before`] / [`fc4_plumbing_shadow_after`].
//!
//! # When it compares: at the end of each water pass
//!
//! A pass (§6) runs over many deliveries; nothing outside it changes the
//! water grid, so the pass is the unit that has a well-defined result. The
//! month only sets recompute and ageing and redistributes distressed pipes
//! (§7.1); it is compared on its own (below). The shadow follows the
//! original's pass from outside, without touching it:
//!
//! - **Start.** passCounter (record field 7) is written only by state 0
//!   (§6.1, PS6), so the delivery after which it moved ran state 0; the
//!   shadow reads it after each delivery through the original's own `Write`
//!   (`pass_counter`, without the references). The next `DoIdle` runs
//!   state 1; after it the shadow captures the original's record (its own `Write`
//!   into memory, §9: flag map, producers with the production of its state 1,
//!   pipes, department, totals) and starts FreeCity4's pass on it, over
//!   private copies of the two grids: states 0 and 1 at once (their results
//!   kept for the comparison), then the original's production is put back so
//!   a difference does not spread.
//! - **State by state.** States 0 to 5 each take one `DoIdle` (§6), so the
//!   shadow runs each of our states 2 to 5 just before the delivery in which
//!   the original runs the same state (state 5 falls through into 6, and our
//!   state 6 is not sliced, so our 7 follows at once). Each therefore reads
//!   the live inputs as the original's does, whatever the occupant messages
//!   delivered between the steps changed: a building's consumption at the
//!   ring flood (§6.3 step 3, §5.3 PS8) above all. The flag map, which those
//!   messages move (§5.3 step 2, §5.4), is the original's own, taken from
//!   its record before each of our states. The pass has no time limit: a
//!   loaded machine delivers few `SimIdle`, and a pass can take days; it is
//!   followed until its swap or until the counter moves again. Every input is
//!   read live, read-only, through the host table; every write stays in the
//!   shadow (deal usage, connection flags, occupant properties in overlays
//!   that later reads see; posts, history, lots, pipe watered states, budget
//!   writes are counted and dropped).
//! - **Swap** (the end of state 6; `GetWateredGrid` changes): our published
//!   grid is compared cell by cell with the original's new one, with the
//!   production, efficiency, polluted flag and water used of each producer
//!   (its record and `GetWaterProducerInfo`), the polluted count and pollution
//!   average, the deal usage, and each water connection's active and visited
//!   flags. The record captured now must show the same pass, producers, pipes
//!   and department as at the start, and the flag map our last state ran on
//!   (a change during the original's time-sliced state 6 is not seen by our
//!   unsliced one); otherwise the pass is "not exact" (the city changed while
//!   it ran: its differences are reported but not counted). Demand is not
//!   among them: no pass state reads it (§5.3, PS8). A pass restarted by a
//!   deal status (state 9, then 0) moves the counter again and is replayed
//!   from its new start. Without the `pass_counter` callback, the start shows
//!   on a probe cell of the next working grid (state 2 is its first writer and
//!   sets every cell to 0 or `PIPE`), and a restart is replayed from the
//!   swap's record.
//! - **End.** The shadow captures after each delivery until state 7 is done
//!   (the record's iterator), runs our state 8 just before the original's
//!   (its inputs, producer condition and budget items, are the ones state 8
//!   changes), and compares after it: the last totals (produced, consumed,
//!   imported, exported, as saved), the water cost, the watered and
//!   unwatered building counts (slots 21, 22), demand (23, against the
//!   record read then: the original's running sum, moved by the occupant
//!   messages delivered during the pass, PS4 / PS8), pipes, distressed and
//!   burst (18-20), sources (12), each producer's budget item cost, and,
//!   when the original ages this pass (the first state 8 after a month
//!   start, §6.7), each producer's condition. When a state 8 may have run
//!   unseen, whether the original aged shows in the conditions (an ageing
//!   changes each one whose increment is not 0, PS7). One line per pass.
//!
//! A pass whose start was not seen (the shadow started during it) is
//! replayed from the record captured at its swap and is not exact. Passes are
//! replayed at most `passes_per_day` times a game day (0: every pass); the
//! others are only followed (their swap and end), so the ageing flag stays
//! known.
//!
//! # The month
//!
//! `ApplyPipeFunding` (§7.1): the shadow reads the pipe counts (slots 18-20)
//! and the funding of the simulator's own department (the one its record
//! names, PS5) before the original's month, and the counts after it. Our
//! pipe condition gives the distressed target (PS2: the curve is its change
//! per month). The original's is not saved (PS3): a shadow made at the city
//! start (`fc4_plumbing_shadow_city_start`) knows it is 0 there and compares
//! the target exactly; one made mid-city marks it not exact. The bursts come
//! from the original's own random generator (its `GaussianFast` draw and the
//! CRT shuffle), which is not readable, so only the range our draw allows is
//! checked ("not exact").
//!
//! # Not compared
//!
//! Each pipe's watered state (`SetWateredState` has no getter), the
//! redisplayed rectangles, posted messages, history values, budget line-item
//! expenses, lot conditions and deal flags (written, never read back), the
//! month's random bursts (range only), the original's pipe condition (not
//! saved, no getter; implied by its distressed count) and its pass state and
//! flags (not saved).

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::ffi::c_void;
use std::fmt::Display;
use std::rc::Rc;

use fc4_core::grid::{CellRect, GridElem, GridProvider, ScanState, SimGrid, TractGeometry, TractGrid};
use fc4_core::julian::civil_from_julian;
use fc4_core::plumbing::{
    budget, deal, msg, pipe_funding, prop, BudgetItem, PlumbingHost, PlumbingSim, PlumbingState, Tuning,
    IID_DEPARTMENT_BUDGET, IID_NETWORK_OCCUPANT, IID_OCCUPANT, IID_SIM_GRID,
};
use fc4_core::property::PropertyHolder;
use fc4_core::stream::{ByteReader, InStream, RefValue, SaveObject};
use fc4_core::variant::Variant;

use crate::guard;
use crate::hostgrid::{collect_tokens, fc4_cell_rect, read_property, take_props};
use crate::landvalue::fc4_grid_geometry;
use crate::plumbing::{fc4_plumbing_budget_item, fc4_plumbing_host, fc4_plumbing_producer_info};
use crate::stream::fc4_bytes_sink;

type U = *mut c_void;

/// One reference of a captured record (fc4.h `fc4_plumbing_capture_ref`):
/// the object the original wrote, queried for each interface its reader asks
/// for (0 = the object does not answer it). Tokens are borrowed.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_plumbing_capture_ref {
    /// As a `cISC4SimGrid` (IID `0xC9BD939A`).
    pub grid: u64,
    /// As a `cISC4Occupant` (IID `0x26BB767E`).
    pub occupant: u64,
    /// As the pipe (IID `0x49C05B9F`, which only a pipe answers).
    pub network: u64,
    /// As a `cISC4DepartmentBudget` (IID `0x223E2B67`).
    pub department: u64,
    /// `GetGZCLSID()`.
    pub class_id: u32,
    pub reserved: u32,
}

/// A captured record (fc4.h `fc4_plumbing_capture`): the bytes the
/// original's `Write` produced (primitives little-endian, references as
/// `u32 objectID` (index + 1, 0 = null) then `u32 classID`) and its
/// references in write order.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_plumbing_capture {
    pub struct_size: u32,
    /// Non-zero when `Write` returned true and the stream stayed good.
    pub ok: u32,
    pub bytes: *const u8,
    pub len: usize,
    pub refs: *const fc4_plumbing_capture_ref,
    pub ref_count: usize,
}

impl Default for fc4_plumbing_capture {
    fn default() -> Self {
        fc4_plumbing_capture {
            struct_size: std::mem::size_of::<fc4_plumbing_capture>() as u32,
            ok: 0,
            bytes: std::ptr::null(),
            len: 0,
            refs: std::ptr::null(),
            ref_count: 0,
        }
    }
}

/// The original plumbing simulator, as the shadow reads it (fc4.h
/// `fc4_plumbing_original`). Every callback only reads.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_plumbing_original {
    pub struct_size: u32,
    pub user: *mut c_void,
    /// `GetWateredGrid()` (slot 7) as a grid token (borrowed; 0 = none).
    pub watered_grid: Option<unsafe extern "C" fn(U) -> u64>,
    /// The argument-free number slots 12-23: slots 14-17 and 23 as their
    /// signed 32-bit EAX, the others zero-extended.
    pub slot_value: Option<unsafe extern "C" fn(U, u32) -> i64>,
    /// `GetWaterProducerInfo(occupant, ...)` (slot 24); false = AL 0.
    pub producer_info: Option<unsafe extern "C" fn(U, u64, *mut fc4_plumbing_producer_info) -> bool>,
    /// `cISC4NeighborConnection::GetVisitedThisCycle()`.
    pub connection_visited: Option<unsafe extern "C" fn(U, u64) -> bool>,
    /// The original's own `Write` into memory; the view stays valid until the
    /// next call. False (or `ok` 0) when it failed.
    pub capture: Option<unsafe extern "C" fn(U, *mut fc4_plumbing_capture) -> bool>,
    /// The original's passCounter (record field 7, plumbing.md §6.1 PS6),
    /// through its own `Write` without looking at the references; false when
    /// it failed. Optional (ABI 33): without it a pass start is seen through
    /// a probe cell of the next working grid.
    pub pass_counter: Option<unsafe extern "C" fn(U, *mut i32) -> bool>,
}

/// Totals since creation (fc4.h `fc4_plumbing_shadow_stats`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_plumbing_shadow_stats {
    pub struct_size: u32,
    /// Deliveries wrapped (before / after pairs).
    pub deliveries: u32,
    /// Passes whose swap was seen.
    pub passes_seen: u32,
    /// Pass lines (passes replayed and compared at least at their swap).
    pub passes_compared: u32,
    /// ... exact, compared to their end, without a difference.
    pub passes_clean: u32,
    /// ... not exact (differences reported, not counted).
    pub passes_not_exact: u32,
    /// Passes not compared (INCOMPLETE lines) or whose end was not seen.
    pub passes_incomplete: u32,
    pub months: u32,
    pub months_clean: u32,
    /// The last comparison's counted differences (-1: none yet).
    pub last_mismatches: i32,
    /// Records captured.
    pub captures: u32,
    pub reserved: u32,
    pub cells: u64,
    pub cell_mismatches: u64,
    pub scalar_mismatches: u64,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_plumbing_capture_ref>() == 40);
    assert!(std::mem::size_of::<fc4_plumbing_capture>() == if cfg!(target_pointer_width = "32") { 24 } else { 40 });
    assert!(std::mem::size_of::<fc4_plumbing_original>() == if cfg!(target_pointer_width = "32") { 32 } else { 64 });
    assert!(std::mem::size_of::<fc4_plumbing_shadow_stats>() == 72);
};

/// `DoIdle` calls our replay may take to reach state 8 (states 3 and 4 run
/// once per network).
const MAX_REPLAY_STEPS: u32 = 1_000_000;
/// Deliveries after a swap before the end is given up.
const MAX_AFTER_SWAP: u32 = 256;
/// INCOMPLETE lines per month (the month line counts them all).
const INCOMPLETE_LINES_PER_MONTH: u32 = 8;
/// Producers, deals and connections listed per line (beyond `max_cells`).
const LIST_MIN: usize = 4;

// ---- the shadow's host -----------------------------------------------------------------------------

/// The shadow's `PlumbingHost`: reads go to the C host (live, read-only);
/// writes stay here.
struct ShadowHost {
    c: fc4_plumbing_host,
    o: fc4_plumbing_original,
    /// Loading a record only: no grids.
    parse: Cell<bool>,
    tuning: RefCell<Option<Option<Rc<PropertyHolder>>>>,
    /// Our occupant property writes.
    props: RefCell<HashMap<(u64, u32), Variant>>,
    /// Our `SetAmountUsed` values.
    deals_used: RefCell<BTreeMap<u64, i32>>,
    /// Our `SetVisitedThisCycle` / `SetIsActive` values.
    visited: RefCell<BTreeMap<u64, bool>>,
    active: RefCell<BTreeMap<u64, bool>>,
    /// The budget item cost each producer should have after state 8: the
    /// cost read, or the one `ChangeBudgetItemCost` set.
    maintenance: RefCell<BTreeMap<u64, i64>>,
    /// The current run's grid copies by token.
    grids: RefCell<HashMap<u64, Rc<SimGrid<u8>>>>,
    /// Calls kept here: posts, history, lots, pipes, budget writes.
    swallowed: Cell<u32>,
}

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the shadow promised valid callbacks.
        match $s.c.$f { Some(f) => unsafe { f($s.c.user $(, $arg)*) }, None => $default }
    };
}

macro_rules! ocb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the shadow promised valid callbacks.
        match $s.o.$f { Some(f) => unsafe { f($s.o.user $(, $arg)*) }, None => $default }
    };
}

fn opt(t: u64) -> Option<u64> {
    (t != 0).then_some(t)
}

impl ShadowHost {
    fn swallow(&self) {
        self.swallowed.set(self.swallowed.get().wrapping_add(1));
    }

    fn live_prop(&self, occ: u64, id: u32) -> Option<Variant> {
        let f = self.c.occupant_property?;
        // SAFETY: valid callback; the scratch holder outlives the call.
        read_property(id, |p| unsafe { f(self.c.user, occ, id, p) })
    }

    fn live_deals(&self, deal_type: u32) -> Vec<u64> {
        collect_tokens(|sink, ctx| cb!(self, find_deals, (), deal_type, sink, ctx))
    }

    fn live_deal_used(&self, d: u64) -> i32 {
        cb!(self, deal_amount_used, 0, d)
    }

    fn live_connections(&self) -> Vec<u64> {
        collect_tokens(|sink, ctx| cb!(self, connections_of_type, (), deal::WATER_CONNECTION, sink, ctx))
    }

    fn live_active(&self, conn: u64) -> bool {
        cb!(self, connection_is_active, false, conn)
    }

    /// The original's passCounter (PS6), if the table can read it.
    fn orig_pass_counter(&self) -> Option<i32> {
        let f = self.o.pass_counter?;
        let mut v = 0i32;
        // SAFETY: valid callback; `v` outlives the call.
        unsafe { f(self.o.user, &mut v) }.then_some(v)
    }

    /// The original's `GetVisitedThisCycle`, if the table has it.
    fn orig_visited(&self, conn: u64) -> Option<bool> {
        let f = self.o.connection_visited?;
        // SAFETY: valid callback.
        Some(unsafe { f(self.o.user, conn) })
    }

    fn live_budget_item(&self, occ: u64, purpose: u32) -> Option<BudgetItem> {
        let mut i = fc4_plumbing_budget_item::default();
        cb!(self, budget_item_for_purpose, false, occ, purpose, &mut i).then_some(BudgetItem {
            department_id: i.department,
            line_item: i.line,
            cost: i.cost,
            index: i.index,
        })
    }

    /// Starts a replay: every overlay and grid copy is dropped.
    fn begin_run(&self) {
        self.props.borrow_mut().clear();
        self.deals_used.borrow_mut().clear();
        self.visited.borrow_mut().clear();
        self.active.borrow_mut().clear();
        self.maintenance.borrow_mut().clear();
        self.grids.borrow_mut().clear();
    }
}

impl GridProvider<u8> for ShadowHost {
    fn create_grid(&self, _instance: u32, _tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        None
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u8>>> {
        if self.parse.get() {
            return None;
        }
        match *r {
            RefValue::Host { token, .. } => self
                .grids
                .borrow()
                .get(&token)
                .map(|g| g.clone() as Rc<dyn TractGrid<u8>>),
            _ => None,
        }
    }
}

impl PlumbingHost for ShadowHost {
    fn cell_counts(&self) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        cb!(self, cell_counts, (), &mut x, &mut z);
        (x, z)
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        let (mut cx, mut cz) = (0, 0);
        cb!(self, position_to_cell, (), x, z, &mut cx, &mut cz);
        (cx, cz)
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        // Read once per shadow: the exemplar is the game's data.
        if let Some(t) = self.tuning.borrow().as_ref() {
            return t.clone();
        }
        let p = cb!(self, tuning, std::ptr::null_mut());
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        let t = unsafe { take_props(p) };
        *self.tuning.borrow_mut() = Some(t.clone());
        t
    }
    fn session_seed(&self) -> u32 {
        0
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
    fn record_history(&self, _id: u32, _value: u32) {
        self.swallow();
    }
    fn cumulative_factor(&self, id: u32, index: u32) -> Option<f32> {
        let mut v = 0.0f32;
        cb!(self, cumulative_factor, false, id, index, &mut v).then_some(v)
    }
    fn water_value(&self, x: i32, z: i32) -> i32 {
        cb!(self, water_value, 0, x, z)
    }
    fn occupant_type(&self, occ: u64) -> u32 {
        cb!(self, occupant_type, 0, occ)
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant> {
        if let Some(v) = self.props.borrow().get(&(occ, id)) {
            return Some(v.clone());
        }
        self.live_prop(occ, id)
    }
    fn set_occupant_property(&self, occ: u64, id: u32, value: Variant) {
        self.props.borrow_mut().insert((occ, id), value);
    }
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        let mut p = [0.0f32; 3];
        cb!(self, occupant_position, (), occ, p.as_mut_ptr());
        p
    }
    fn occupant_bounding_box(&self, occ: u64) -> ([f32; 3], [f32; 3]) {
        let (mut a, mut b) = ([0.0f32; 3], [0.0f32; 3]);
        cb!(self, occupant_bounding_box, (), occ, a.as_mut_ptr(), b.as_mut_ptr());
        (a, b)
    }
    fn occupant_bounding_cells(&self, occ: u64) -> CellRect {
        let mut r = fc4_cell_rect::default();
        cb!(self, occupant_bounding_cells, (), occ, &mut r);
        r.into()
    }
    fn building_type(&self, occ: u64) -> Option<u32> {
        let mut t = 0;
        cb!(self, building_type, false, occ, &mut t).then_some(t)
    }
    fn buildings_in_cell(&self, x: i32, z: i32) -> Vec<u64> {
        collect_tokens(|sink, ctx| cb!(self, buildings_in_cell, (), x, z, sink, ctx))
    }
    fn is_network_occupant(&self, occ: u64) -> bool {
        cb!(self, is_network_occupant, false, occ)
    }
    fn has_network_flag(&self, occ: u64, flag: u32) -> bool {
        cb!(self, has_network_flag, false, occ, flag)
    }
    fn network_flags(&self, occ: u64) -> u32 {
        cb!(self, network_flags, 0, occ)
    }
    fn set_network_flag(&self, _occ: u64, _flag: u32) {
        self.swallow();
    }
    fn clear_network_flag(&self, _occ: u64, _flag: u32) {
        self.swallow();
    }
    fn connects_to_neighbor(&self, occ: u64, dir: u32, network_type: u32) -> bool {
        cb!(self, connects_to_neighbor, false, occ, dir, network_type)
    }
    fn occupied_cell(&self, occ: u64) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        cb!(self, occupied_cell, (), occ, &mut x, &mut z);
        (x, z)
    }
    fn set_pipe_watered(&self, _occ: u64, _watered: bool) {
        self.swallow();
    }
    fn lot_at(&self, x: i32, z: i32) -> Option<u64> {
        opt(cb!(self, lot_at, 0, x, z))
    }
    fn occupant_lot(&self, occ: u64) -> Option<u64> {
        opt(cb!(self, occupant_lot, 0, occ))
    }
    fn lot_rect(&self, lot: u64) -> CellRect {
        let mut r = fc4_cell_rect::default();
        cb!(self, lot_rect, (), lot, &mut r);
        r.into()
    }
    fn lot_needs_power(&self, lot: u64) -> bool {
        cb!(self, lot_needs_power, false, lot)
    }
    fn lot_has_power(&self, lot: u64) -> bool {
        cb!(self, lot_has_power, false, lot)
    }
    fn set_lot_condition(&self, _lot: u64, _condition: u8) {
        self.swallow();
    }
    fn terrain_cell_counts(&self) -> Option<(i32, i32)> {
        let (mut x, mut z) = (0, 0);
        cb!(self, terrain_cell_counts, false, &mut x, &mut z).then_some((x, z))
    }
    fn cell_has_water(&self, x: i32, z: i32) -> bool {
        cb!(self, cell_has_water, false, x, z)
    }
    fn redisplay_terrain(&self, _min: (i32, i32), _max: (i32, i32)) {
        self.swallow();
    }
    fn attempt_pipe_burst(&self, _x: i32, _z: i32) -> bool {
        self.swallow();
        false
    }
    fn find_deals(&self, deal_type: u32) -> Vec<u64> {
        self.live_deals(deal_type)
    }
    fn deal_type(&self, d: u64) -> u32 {
        cb!(self, deal_type, 0, d)
    }
    fn deal_city(&self, d: u64) -> u64 {
        cb!(self, deal_city, 0, d)
    }
    fn deal_amount(&self, d: u64) -> i32 {
        cb!(self, deal_amount, 0, d)
    }
    fn deal_amount_used(&self, d: u64) -> i32 {
        if let Some(&v) = self.deals_used.borrow().get(&d) {
            return v;
        }
        self.live_deal_used(d)
    }
    fn set_deal_amount_used(&self, d: u64, used: i32) {
        self.deals_used.borrow_mut().insert(d, used);
    }
    fn set_deal_flags(&self, _d: u64, _flags: u32, _on: bool) {
        self.swallow();
    }
    fn connections_of_type(&self, connection_type: u32) -> Vec<u64> {
        collect_tokens(|sink, ctx| cb!(self, connections_of_type, (), connection_type, sink, ctx))
    }
    fn connections_of_type_to_city(&self, connection_type: u32, city: u64) -> Vec<u64> {
        collect_tokens(|sink, ctx| cb!(self, connections_of_type_to_city, (), connection_type, city, sink, ctx))
    }
    /// `FindUnvisitedConnection(t, city)` over our visited flags: the
    /// connection list walked from its head in append order, the first of
    /// type t to `city` whose visited bit is clear (plumbing.md §6.2, PS1;
    /// power.md §6.5, PW1), which is the order `GetConnectionsOfTypeToCity`
    /// lists them in. The original's answer reads its own flags, which the
    /// shadow must not change.
    fn find_unvisited_connection(&self, connection_type: u32, city: u64) -> Option<u64> {
        self.connections_of_type_to_city(connection_type, city)
            .into_iter()
            .find(|&c| {
                let v = self.visited.borrow().get(&c).copied();
                !v.unwrap_or_else(|| self.orig_visited(c).unwrap_or(false))
            })
    }
    fn connection_on_other_side(&self, conn: u64) -> Option<u64> {
        opt(cb!(self, connection_on_other_side, 0, conn))
    }
    fn connection_cell(&self, conn: u64) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        cb!(self, connection_cell, (), conn, &mut x, &mut z);
        (x, z)
    }
    fn set_connection_visited(&self, conn: u64, visited: bool) {
        self.visited.borrow_mut().insert(conn, visited);
    }
    fn connection_is_active(&self, conn: u64) -> bool {
        if let Some(&v) = self.active.borrow().get(&conn) {
            return v;
        }
        self.live_active(conn)
    }
    fn set_connection_active(&self, conn: u64, active: bool) {
        self.active.borrow_mut().insert(conn, active);
    }
    fn mark_deals_dirty(&self) {
        self.swallow();
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
    fn create_line_item(&self, _dept: u64, _line: u32, _locally_funded: bool) -> Option<u64> {
        None
    }
    fn set_line_item_name(&self, _line_item: u64, _group: u32, _instance: u32) {
        self.swallow();
    }
    fn remove_line_item(&self, _dept: u64, _line: u32) {
        self.swallow();
    }
    fn department_funding(&self, dept: u64) -> f32 {
        cb!(self, department_funding, 1.0, dept)
    }
    fn department_by_id(&self, id: u32) -> Option<u64> {
        opt(cb!(self, department_by_id, 0, id))
    }
    fn line_item(&self, dept: u64, line: u32) -> Option<u64> {
        opt(cb!(self, line_item, 0, dept, line))
    }
    fn create_line_item_for_building_type(&self, _dept: u64, _line: u32, _locally_funded: bool) -> Option<u64> {
        None
    }
    fn set_line_item_display_flag(&self, _line_item: u64, _flag: u32, _on: bool) {
        self.swallow();
    }
    fn add_to_line_item_expenses(&self, _line_item: u64, _delta: i64) {
        self.swallow();
    }
    fn set_line_item_expenses(&self, _line_item: u64, _value: i64) {
        self.swallow();
    }
    fn line_item_secondary_info(&self, line_item: u64) -> i64 {
        cb!(self, line_item_secondary_info, 0, line_item)
    }
    fn set_line_item_secondary_info(&self, _line_item: u64, _value: i64) {
        self.swallow();
    }
    fn budget_item_for_purpose(&self, occ: u64, purpose: u32) -> Option<BudgetItem> {
        let item = self.live_budget_item(occ, purpose)?;
        self.maintenance.borrow_mut().entry(occ).or_insert(item.cost);
        Some(item)
    }
    fn change_budget_item_line(&self, _occ: u64, _index: u32, _line: u32) {
        self.swallow();
    }
    fn change_budget_item_cost(&self, occ: u64, _index: u32, cost: i64) {
        self.maintenance.borrow_mut().insert(occ, cost);
        self.swallow();
    }
    fn line_item_from_budget_item(&self, _item: &BudgetItem) -> Option<u64> {
        // Only state 8's expense delta asks: a write, dropped.
        None
    }
    fn local_funding(&self, dept: u64, occ: u64) -> Option<f32> {
        let mut f = 0.0f32;
        cb!(self, local_funding, false, dept, occ, &mut f).then_some(f)
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
    fn grid_shutdown(&self, _grid: &Rc<dyn TractGrid<u8>>) {}
}

// ---- the captured record ---------------------------------------------------------------------------

/// A captured record as a core input stream: references are answered with
/// the token of the interface the reader asks for.
struct CaptureReader<'a> {
    r: ByteReader<'a>,
    refs: &'a [fc4_plumbing_capture_ref],
    /// The grid references in read order (0 = null or not a grid).
    grid_tokens: Vec<u64>,
}

impl<'a> CaptureReader<'a> {
    fn new(bytes: &'a [u8], refs: &'a [fc4_plumbing_capture_ref]) -> Self {
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
        let mut token = 0;
        let mut class_id = 0;
        if id != 0 {
            class_id = self.r.get_u32()?;
            let Some(e) = self.refs.get(id as usize - 1) else {
                self.r.set_error();
                return None;
            };
            token = match iid {
                IID_SIM_GRID => e.grid,
                IID_OCCUPANT => e.occupant,
                // The plumbing glue knows a pipe by its cISC4Occupant identity
                // (cFC4Plumbing.cpp ResolveRef); only a pipe answers the IID.
                IID_NETWORK_OCCUPANT if e.network != 0 => {
                    if e.occupant != 0 {
                        e.occupant
                    } else {
                        e.network
                    }
                }
                IID_DEPARTMENT_BUDGET => e.department,
                _ => 0,
            };
        }
        if iid == IID_SIM_GRID {
            self.grid_tokens.push(token);
        }
        if token == 0 {
            // §2.7 helper semantics: an object without the interface reads as null.
            return Some(RefValue::Null);
        }
        Some(RefValue::Host { token, class_id })
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
    refs: Vec<fc4_plumbing_capture_ref>,
}

/// A loaded record: the state as our `Read` sees it (no grids) and the two
/// grid tokens it refers to (working, published).
struct Rec {
    state: PlumbingState,
    grids: [u64; 2],
}

impl Rec {
    fn iterator_done(&self) -> bool {
        self.state.iterator.as_ref().is_some_and(|i| i.state == ScanState::Done)
    }
}

/// What must not change while a pass runs for its replay to be exact: the
/// producers, pipes and department since its start (`start`), and the flag
/// map since our last state ran on it (`flags_seen`: each of our states 2 to
/// 5 runs on the original's flag map as it stands then, so only a change
/// during the original's time-sliced state 6, after our unsliced one,
/// remains). Demand is not among them: no pass state reads it, and the
/// occupant messages delivered between the pass's steps move it (plumbing.md
/// §5.3, PS8); it is compared with the record read at the comparison.
fn changed_inputs(start: &PlumbingState, flags_seen: &[u8], now: &PlumbingState) -> Vec<&'static str> {
    let mut v = Vec::new();
    if flags_seen != now.flags.as_slice() || (start.count_x, start.count_z) != (now.count_x, now.count_z) {
        v.push("flag map changed during state 6");
    }
    let prod = |s: &PlumbingState| {
        s.producers
            .iter()
            .map(|p| (p.occupant, p.seed_x, p.seed_z, p.rect, p.max_capacity, p.production))
            .collect::<Vec<_>>()
    };
    if prod(start) != prod(now) {
        v.push("producers changed during the pass");
    }
    if start.pipes != now.pipes {
        v.push("pipes changed during the pass");
    }
    if start.department != now.department {
        v.push("department changed during the pass");
    }
    v
}

/// Marks a run not exact, once per reason.
fn note_inexact(run: &mut Run, why: String) {
    if !run.why_inexact.contains(&why) {
        run.why_inexact.push(why);
    }
}

/// Our `DoIdle` until `target` is the next state (one state per call, §6;
/// our states 6 and 7 are not sliced).
fn run_to(sim: &PlumbingSim, target: u32) -> Result<(), String> {
    let mut steps = 0u32;
    while sim.with_state(|s| s.state) != target {
        if steps >= MAX_REPLAY_STEPS || sim.with_state(|s| s.state) > 8 {
            return Err(format!("our pass did not reach state {target}"));
        }
        sim.do_idle(0);
        steps += 1;
    }
    Ok(())
}

// ---- comparisons and lines ---------------------------------------------------------------------------

fn date(julian: i32) -> String {
    let d = civil_from_julian(julian);
    format!("{:04}-{:02}-{:02}", d.year, d.month, d.day)
}

fn first_number(v: &Option<Variant>) -> Option<f64> {
    v.as_ref().and_then(|v| v.items().first().map(|e| e.as_f64()))
}

fn same_f32(a: f32, b: f32) -> bool {
    a == b || a.to_bits() == b.to_bits()
}

fn mark(bad: bool) -> &'static str {
    if bad {
        "!"
    } else {
        ""
    }
}

/// One comparison's parts and differences.
#[derive(Default)]
struct Cmp {
    diffs: u64,
    cell_diffs: u64,
    parts: Vec<String>,
}

impl Cmp {
    fn scalar<T: PartialEq + Display>(&mut self, name: &str, orig: T, ours: T) {
        let bad = orig != ours;
        if bad {
            self.diffs += 1;
        }
        self.parts.push(format!("{name} orig/ours={orig}/{ours}{}", mark(bad)));
    }

    /// Per-object pairs: `name orig/ours=[a/b, c/d!]`, at most `max` listed.
    fn list(&mut self, name: &str, items: &[(String, String, bool)], max: usize) {
        let bad = items.iter().filter(|i| i.2).count();
        self.diffs += bad as u64;
        let mut shown: Vec<String> = items
            .iter()
            .take(max)
            .map(|(a, b, x)| format!("{a}/{b}{}", mark(*x)))
            .collect();
        if items.len() > max {
            shown.push(format!("+{} more ({} of them differing)", items.len() - max, {
                items[max..].iter().filter(|i| i.2).count()
            }));
        }
        self.parts.push(format!("{name} orig/ours=[{}]", shown.join(", ")));
    }
}

// ---- the shadow -------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_plumbing_shadow`).
#[repr(C)]
pub struct fc4_plumbing_shadow {
    _private: [u8; 0],
}

/// The cell of the original's next working grid that shows when its state 2
/// ran (see the module notes).
#[derive(Clone, Copy, Debug)]
struct Probe {
    token: u64,
    x: i32,
    z: i32,
    value: u8,
}

/// One pass of ours, replayed on the original's record.
struct Run {
    aligned_start: bool,
    why_inexact: Vec<String>,
    /// The record the replay started from.
    start: PlumbingState,
    /// The game day it started on (the pass line's `days`).
    start_julian: i32,
    /// The original's flag map our last state ran on.
    flags_seen: Vec<u8>,
    sim: Rc<PlumbingSim>,
    /// Our state 1: (occupant, production, efficiency, polluted).
    ours1: Vec<(u64, u32, f32, bool)>,
    avg1: u32,
    polluted1: u32,
    /// The comparison so far (the swap's).
    cmp: Cmp,
    /// The ageing flag our state 8 ran with (None: unknown, not aged).
    ageing: Option<bool>,
    /// With the flag unknown: each producer's condition just before the
    /// original's state 8, to see whether it aged (PS7).
    cond_before: Option<Vec<(u64, Option<Variant>)>>,
}

/// A pass the shadow follows: its pass counter, and our replay if it has one.
struct Track {
    id: i32,
    run: Option<Box<Run>>,
    after_swap: u32,
    deliveries: u32,
}

enum Phase {
    /// Between passes (or not known).
    Idle,
    /// The passCounter moved: that delivery ran state 0 (PS6). The next
    /// `DoIdle` runs state 1, after which the record holds this pass's
    /// production: the replay starts there.
    Counted { julian: i32, id: i32 },
    /// A pass started (state 1 or 2 seen); our states run one per delivery
    /// (`Shadow::step`) until its swap, however many days that takes.
    Started {
        julian: i32,
        deliveries: u32,
        run: Option<Box<Run>>,
        id: i32,
    },
    /// Swapped; state 7 running.
    Swapped(Track),
    /// State 7 done: the next `DoIdle` runs state 8.
    EndPending(Track),
}

/// The pipe counts and funding read before the original's month.
#[derive(Clone)]
struct MonthBefore {
    pipes: u32,
    distressed: u32,
    burst: u32,
    funding: f32,
    /// Why the funding is not the simulator's own department's (PS5).
    funding_note: Option<String>,
}

/// The delivery between before and after.
struct Cur {
    julian: i32,
    pub_before: u64,
    month: Option<MonthBefore>,
}

/// Counts since the last month line.
#[derive(Default)]
struct Period {
    seen: u32,
    compared: u32,
    clean: u32,
    not_exact: u32,
    incomplete: u32,
    incomplete_lines: u32,
    differing: u64,
}

struct Shadow {
    host: Rc<ShadowHost>,
    max_cells: usize,
    passes_per_day: u32,
    cur: Option<Cur>,
    phase: Phase,
    probe: Option<Probe>,
    probe_cursor: usize,
    /// The original's ageing flag as far as known (§6.7 step 3, §7.1).
    ageing: Option<bool>,
    /// A state 8 may have run (or may still run) unseen.
    maybe_end: bool,
    day: i32,
    day_replays: u32,
    /// Our pipe condition (§7.1), from the shadow's creation.
    tracked_condition: f32,
    /// Made at the city start (`fc4_plumbing_shadow_city_start`): the
    /// original's pipe condition and ageing flag are its constructor's, 0
    /// and false (PS3), so `tracked_condition` is its condition.
    condition_known: bool,
    /// The original's passCounter at the last read (PS6).
    last_counter: Option<i32>,
    tuning: Option<Tuning>,
    period: Period,
    stats: fc4_plumbing_shadow_stats,
}

impl Shadow {
    fn slot(&self, slot: u32) -> i64 {
        ocb!(self.host, slot_value, 0, slot)
    }

    fn watered_grid(&self) -> u64 {
        ocb!(self.host, watered_grid, 0)
    }

    fn geometry(&self, token: u64) -> TractGeometry {
        let api = &self.host.c.grids;
        let mut g = fc4_grid_geometry::default();
        if let Some(f) = api.geometry {
            // SAFETY: valid callback; `g` outlives the call.
            unsafe { f(api.user, token, &mut g) };
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

    fn live_cell(&self, token: u64, x: i32, z: i32) -> u8 {
        let api = &self.host.c.grids;
        match api.get {
            // SAFETY: valid callback on a live game grid.
            Some(f) => u8::from_f64(unsafe { f(api.user, token, u8::CLSID, x, z) }),
            None => 0,
        }
    }

    fn tuning(&mut self) -> Tuning {
        if self.tuning.is_none() {
            let mut t = Tuning::default();
            t.reload(self.host.tuning().as_deref());
            self.tuning = Some(t);
        }
        self.tuning.clone().unwrap_or_default()
    }

    /// The original's record, through its own `Write`.
    fn capture(&mut self) -> Result<Capture, String> {
        let Some(f) = self.host.o.capture else {
            return Err("no capture callback".to_string());
        };
        let mut v = fc4_plumbing_capture::default();
        // SAFETY: valid callback; `v` outlives the call.
        let ok = unsafe { f(self.host.o.user, &mut v) };
        self.stats.captures = self.stats.captures.wrapping_add(1);
        if !ok || v.ok == 0 {
            return Err("the original's Write failed".to_string());
        }
        if v.bytes.is_null() || v.len == 0 {
            return Err("empty record".to_string());
        }
        // SAFETY: the callback's view stays valid until the next call.
        let bytes = unsafe { std::slice::from_raw_parts(v.bytes, v.len) }.to_vec();
        let refs = if v.refs.is_null() || v.ref_count == 0 {
            Vec::new()
        } else {
            // SAFETY: as above.
            unsafe { std::slice::from_raw_parts(v.refs, v.ref_count) }.to_vec()
        };
        Ok(Capture { bytes, refs })
    }

    fn parse(&self, c: &Capture) -> Result<Rec, String> {
        let h = &self.host;
        h.parse.set(true);
        let sim = PlumbingSim::new(h.clone() as Rc<dyn PlumbingHost>, 0);
        // Shadows replay the original's pacing (simulator.md §5.7).
        sim.set_pacing(fc4_core::pacing::Pacing::Original);
        let mut r = CaptureReader::new(&c.bytes, &c.refs);
        let ok = sim.read(&mut r);
        h.parse.set(false);
        if !ok {
            return Err("the record is not a version-4 plumbing record".to_string());
        }
        if r.error() != 0 || r.grid_tokens.len() != 2 {
            return Err("the record is truncated or refers to an unknown object".to_string());
        }
        Ok(Rec {
            state: sim.snapshot(),
            grids: [r.grid_tokens[0], r.grid_tokens[1]],
        })
    }

    fn capture_rec(&mut self) -> Result<(Capture, Rec), String> {
        let c = self.capture()?;
        let r = self.parse(&c)?;
        Ok((c, r))
    }

    /// FreeCity4's pass on the original's record: states 0 and 1, then the
    /// original's production put back, then on until `pending` is the next
    /// state (2: the original has run state 1; 3: state 2, seen on the probe
    /// cell; 8: the swap, the rest of the pass at once).
    fn replay(&self, cap: &Capture, rec: &Rec, julian: i32, pending: u32) -> Result<Box<Run>, String> {
        let aligned_start = pending != 8;
        let h = self.host.clone();
        h.begin_run();
        for &t in &rec.grids {
            if t == 0 || h.grids.borrow().contains_key(&t) {
                continue;
            }
            let geom = self.geometry(t);
            if geom.count_x <= 0 || geom.count_z <= 0 || geom.count_x > 4096 || geom.count_z > 4096 {
                return Err(format!(
                    "a grid of the original is {}x{} tracts",
                    geom.count_x, geom.count_z
                ));
            }
            h.grids.borrow_mut().insert(t, SimGrid::from_values(geom, &[]));
        }
        let sim = PlumbingSim::new(h.clone() as Rc<dyn PlumbingHost>, 0);
        // Shadows replay the original's pacing (simulator.md §5.7).
        sim.set_pacing(fc4_core::pacing::Pacing::Original);
        sim.read_tuning();
        let mut r = CaptureReader::new(&cap.bytes, &cap.refs);
        if !sim.read(&mut r) || r.error() != 0 {
            return Err("our Read of the original's record failed".to_string());
        }
        sim.with_state(|s| {
            s.enabled = true;
            s.state = 0;
            s.recompute = false;
            s.ageing = false;
            s.slice_cells = None;
            // Our state 0 counts the pass again.
            s.pass_counter = s.pass_counter.wrapping_sub(1);
        });
        sim.do_idle(0); // state 0
        sim.do_idle(0); // state 1
        let s1 = sim.snapshot();
        if s1.state != 2 {
            return Err(format!("our pass is in state {} after states 0 and 1", s1.state));
        }
        let ours1 = s1
            .producers
            .iter()
            .map(|p| (p.occupant, p.production, p.efficiency, p.polluted))
            .collect();
        // The flood runs on the original's production (compared on its own),
        // so a difference there does not spread to the grid and totals.
        let orig: HashMap<u64, u32> = rec.state.producers.iter().map(|p| (p.occupant, p.production)).collect();
        sim.with_state(|s| {
            for p in &mut s.producers {
                if let Some(&v) = orig.get(&p.occupant) {
                    p.production = v;
                }
            }
        });
        run_to(&sim, pending)?;
        Ok(Box::new(Run {
            aligned_start,
            why_inexact: Vec::new(),
            start: rec.state.clone(),
            start_julian: julian,
            flags_seen: rec.state.flags.clone(),
            sim,
            ours1,
            avg1: s1.average_water_pollution,
            polluted1: s1.num_polluted,
            cmp: Cmp::default(),
            ageing: None,
            cond_before: None,
        }))
    }

    /// Remembers a probe cell of `token` (the original's next working grid).
    /// A cell of the new working grid whose value state 2 will change: one
    /// that differs from what state 2 writes there (the pipe value where the
    /// swap record's flag map has a pipe, else 0). Without the record, any
    /// value that is neither.
    fn choose_probe(&mut self, token: u64, at_swap: Option<&PlumbingState>) {
        self.probe = None;
        if token == 0 {
            return;
        }
        let g = self.geometry(token);
        let (nx, nz) = (g.count_x.clamp(0, 4096), g.count_z.clamp(0, 4096));
        let n = (nx as usize) * (nz as usize);
        if n == 0 {
            return;
        }
        let start = self.probe_cursor % n;
        for k in 0..n {
            let i = (start + k) % n;
            let (x, z) = ((i % nx as usize) as i32, (i / nx as usize) as i32);
            let v = self.live_cell(token, x, z);
            let differs = match at_swap {
                Some(st) => {
                    let pipe = st.flag(x, z) & fc4_core::plumbing::flag::PIPE != 0;
                    v != if pipe { fc4_core::plumbing::value::PIPE } else { 0 }
                }
                None => v != 0 && v != fc4_core::plumbing::value::PIPE,
            };
            if differs {
                self.probe = Some(Probe { token, x, z, value: v });
                // The next probe starts after this one, so one cell is not
                // read over and over.
                self.probe_cursor = i + 1;
                return;
            }
        }
    }

    fn probe_changed(&self) -> bool {
        self.probe.is_some_and(|p| self.live_cell(p.token, p.x, p.z) != p.value)
    }

    /// Whether today's replay budget allows one more (counted).
    fn budget_allows(&mut self, julian: i32) -> bool {
        if julian != self.day {
            self.day = julian;
            self.day_replays = 0;
        }
        if self.passes_per_day != 0 && self.day_replays >= self.passes_per_day {
            return false;
        }
        self.day_replays += 1;
        true
    }

    /// Before a `DoIdle` of the original during its pass: our next state,
    /// the one the original is about to run (states 0 to 5 take one call
    /// each, §6), on the inputs as they stand now. The flag map first: the
    /// original's, from its record, which the occupant messages delivered
    /// since our last state may have moved (§5.3 step 2, §5.4; PS8). After
    /// our state 5 the rest of our pass runs at once (state 5 falls through
    /// into 6, which ours does not slice; 7 only redisplays).
    fn step(&mut self, run: &mut Run) {
        let state = run.sim.with_state(|s| s.state);
        if !(2..=5).contains(&state) {
            return;
        }
        match self.capture_rec() {
            Ok((_, r)) if (r.state.count_x, r.state.count_z) == (run.start.count_x, run.start.count_z) => {
                run.sim.with_state(|s| s.flags.clone_from(&r.state.flags));
                run.flags_seen = r.state.flags;
            }
            Ok(_) => note_inexact(run, "the city's size changed during the pass".to_string()),
            Err(e) => note_inexact(run, format!("capture before our state {state}: {e}")),
        }
        run.sim.do_idle(0);
        if run.sim.with_state(|s| s.state) >= 6 {
            if let Err(e) = run_to(&run.sim, 8) {
                note_inexact(run, e);
            }
        }
    }

    fn note_observed_end(&mut self) {
        self.ageing = Some(false);
        self.maybe_end = false;
    }

    fn note_unobserved_end(&mut self) {
        self.maybe_end = true;
        if self.ageing == Some(true) {
            self.ageing = None;
        }
    }

    fn incomplete(&mut self, julian: i32, id: Option<i32>, why: &str, lines: &mut Vec<String>) {
        self.stats.passes_incomplete = self.stats.passes_incomplete.wrapping_add(1);
        self.period.incomplete += 1;
        if self.period.incomplete_lines < INCOMPLETE_LINES_PER_MONTH {
            self.period.incomplete_lines += 1;
            let pass = id.map_or(String::new(), |i| format!(" pass {i}"));
            lines.push(format!(
                "plumbing shadow {}{pass}: INCOMPLETE ({why}); not compared",
                date(julian)
            ));
        }
    }

    // ---- delivery -----------------------------------------------------------------------------------

    /// The funding `ApplyPipeFunding` reads (§7.1, PS5): the simulator's own
    /// department, the one its record names. [choice] When the record cannot
    /// be read or names none (the original has no null test), the
    /// department `0xAA4014B4` by ID, said in the month line.
    fn month_funding(&mut self) -> (f32, Option<String>) {
        let why = match self.capture_rec() {
            Ok((_, r)) => match r.state.department {
                Some(d) => return (self.host.department_funding(d), None),
                None => "the record names no department".to_string(),
            },
            Err(e) => e,
        };
        let f = self
            .host
            .department_by_id(budget::DEPARTMENT)
            .map_or(1.0, |d| self.host.department_funding(d));
        (
            f,
            Some(format!(
                "funding from department {:#010x} by ID ({why})",
                budget::DEPARTMENT
            )),
        )
    }

    fn before(&mut self, msg_type: u32, julian: i32) -> i32 {
        let pub_before = self.watered_grid();
        let mut month = None;
        if msg_type == msg::SIM_NEW_MONTH {
            let (funding, funding_note) = self.month_funding();
            month = Some(MonthBefore {
                pipes: self.slot(18) as u32,
                distressed: self.slot(19) as u32,
                burst: self.slot(20) as u32,
                funding,
                funding_note,
            });
            // §7.1: the month sets the ageing flag the next state 8 uses
            // (unknown when a state 8 may run unseen around it).
            self.ageing = if self.maybe_end { None } else { Some(true) };
        } else {
            if self.last_counter.is_none() {
                // The counter before this delivery, so a state 0 in it shows.
                self.last_counter = self.host.orig_pass_counter();
            }
            if let Phase::Started { run: Some(_), .. } = &self.phase {
                let mut phase = std::mem::replace(&mut self.phase, Phase::Idle);
                if let Phase::Started { run: Some(run), .. } = &mut phase {
                    self.step(run);
                }
                self.phase = phase;
            }
            // Occupant insert/remove notifications can arrive after the swap
            // and before state 8 (plumbing.md §5.3/§5.4). These lists and
            // counts are message-owned inputs, not outputs of the pass. Read
            // them at the same boundary as state 8, just as we read the flag
            // map before the flood; keep the pass's computed grids/totals.
            let end_inputs = if matches!(self.phase, Phase::EndPending(_)) {
                Some(self.capture_rec())
            } else {
                None
            };
            if let Phase::EndPending(t) = &mut self.phase {
                // Our state 8, on the inputs the original's state 8 changes
                // (condition, budget items) as they stand before it.
                if let Some(run) = t.run.as_mut() {
                    match end_inputs {
                        Some(Ok((_, r))) => run.sim.with_state(|s| {
                            s.pipes = r.state.pipes;
                            s.num_pipes = r.state.num_pipes;
                            s.num_distressed = r.state.num_distressed;
                            s.num_burst = r.state.num_burst;
                            s.producers = r.state.producers;
                            s.department = r.state.department;
                        }),
                        Some(Err(e)) => note_inexact(run, format!("capture before state 8: {e}")),
                        None => unreachable!(),
                    }
                    let ageing = self.ageing;
                    run.ageing = ageing;
                    if ageing.is_none() {
                        // PS7: whether the original ages shows in the
                        // producers' conditions after its state 8.
                        let occs: Vec<u64> = run.sim.snapshot().producers.iter().map(|p| p.occupant).collect();
                        run.cond_before = Some(
                            occs.into_iter()
                                .map(|o| (o, self.host.live_prop(o, prop::CONDITION)))
                                .collect(),
                        );
                    }
                    self.host.maintenance.borrow_mut().clear();
                    run.sim.with_state(|s| s.ageing = ageing == Some(true));
                    run.sim.do_idle(0);
                }
            }
        }
        self.cur = Some(Cur {
            julian,
            pub_before,
            month,
        });
        0
    }

    fn after(&mut self, lines: &mut Vec<String>) -> i32 {
        let Some(cur) = self.cur.take() else {
            return -1;
        };
        self.stats.deliveries = self.stats.deliveries.wrapping_add(1);
        if let Some(mb) = cur.month {
            return self.month_after(cur.julian, mb, lines);
        }
        let pub_after = self.watered_grid();
        let swapped = cur.pub_before != 0 && pub_after != 0 && pub_after != cur.pub_before;
        // PS6 (§6.1): passCounter is written only by state 0 (and the
        // constructor and Read), so it moved iff this delivery ran state 0.
        let by_counter = self.host.o.pass_counter.is_some();
        let counter = if by_counter {
            self.host.orig_pass_counter()
        } else {
            None
        };
        let started_now = matches!((self.last_counter, counter), (Some(a), Some(b)) if a != b);
        self.last_counter = counter;
        let counted = |julian: i32| Phase::Counted {
            julian,
            id: counter.unwrap_or(0),
        };
        let phase = std::mem::replace(&mut self.phase, Phase::Idle);
        let mut result = -1;
        self.phase = match phase {
            Phase::Idle => {
                if swapped {
                    // A pass we did not see start.
                    self.maybe_end = false;
                    self.on_swap(None, cur.julian, cur.pub_before, pub_after, 0, lines)
                } else if by_counter {
                    if started_now {
                        // The previous pass has ended (or was abandoned).
                        self.maybe_end = false;
                        counted(cur.julian)
                    } else {
                        Phase::Idle
                    }
                } else if self.probe_changed() {
                    // This delivery ran state 2 (its first writer).
                    self.on_start(cur.julian, 3, lines)
                } else {
                    Phase::Idle
                }
            }
            Phase::Counted { julian, id } => {
                if started_now {
                    // State 0 again (a deal status restarted the pass).
                    counted(cur.julian)
                } else if swapped {
                    // Not after state 1 (one state per DoIdle, §6).
                    self.on_swap(Some((None, id)), cur.julian, cur.pub_before, pub_after, 1, lines)
                } else {
                    // This delivery ran state 1.
                    self.on_start(julian, 2, lines)
                }
            }
            Phase::Started {
                julian,
                deliveries,
                run,
                id,
            } => {
                if swapped {
                    self.on_swap(
                        Some((run, id)),
                        cur.julian,
                        cur.pub_before,
                        pub_after,
                        deliveries + 1,
                        lines,
                    )
                } else if started_now {
                    // A deal status restarted the pass (state 9, then 0):
                    // the replay starts again with it.
                    counted(cur.julian)
                } else {
                    // No time limit: the original's states 6 and 7 are
                    // time-sliced and a loaded game delivers few SimIdle, so
                    // a pass can take days (r6b: four). It ends at its swap,
                    // or at the next counter move (a restart).
                    Phase::Started {
                        julian,
                        deliveries: deliveries + 1,
                        run,
                        id,
                    }
                }
            }
            Phase::Swapped(mut t) => {
                t.after_swap += 1;
                t.deliveries += 1;
                match self.capture_rec() {
                    Err(e) => {
                        result = self.end_line(t, cur.julian, Err(format!("capture after the swap: {e}")), lines);
                        self.note_unobserved_end();
                        Phase::Idle
                    }
                    Ok((_, r)) if r.state.pass_counter != t.id => {
                        // Restarted (a deal status) before state 8: no end.
                        result = self.end_line(t, cur.julian, Err("restarted before its end".to_string()), lines);
                        Phase::Idle
                    }
                    Ok((_, r)) if r.iterator_done() => Phase::EndPending(t),
                    Ok(_) if t.after_swap > MAX_AFTER_SWAP => {
                        result = self.end_line(t, cur.julian, Err("state 7 did not finish".to_string()), lines);
                        self.note_unobserved_end();
                        Phase::Idle
                    }
                    Ok(_) => Phase::Swapped(t),
                }
            }
            Phase::EndPending(mut t) => {
                t.deliveries += 1;
                match self.capture_rec() {
                    Err(e) => {
                        result = self.end_line(t, cur.julian, Err(format!("capture after state 8: {e}")), lines);
                        self.note_unobserved_end();
                    }
                    Ok((_, r)) if r.state.pass_counter != t.id => {
                        result = self.end_line(t, cur.julian, Err("restarted before state 8".to_string()), lines);
                    }
                    Ok((_, r)) => {
                        if let Some(run) = t.run.as_mut() {
                            self.compare_end(run, &r);
                        }
                        result = self.end_line(t, cur.julian, Ok(()), lines);
                        self.note_observed_end();
                    }
                }
                Phase::Idle
            }
        };
        if started_now && matches!(self.phase, Phase::Idle) {
            // A restart seen after the swap ("restarted before ..."): the
            // new pass's state 0 ran in this delivery.
            self.phase = counted(cur.julian);
        }
        if result >= 0 {
            self.stats.last_mismatches = result;
        }
        result
    }

    /// A pass started: the original has run state 1 (`pending` 2, seen
    /// through its counter) or state 2 (`pending` 3, seen on the probe cell).
    fn on_start(&mut self, julian: i32, pending: u32, lines: &mut Vec<String>) -> Phase {
        self.probe = None;
        // The previous pass has ended (or was abandoned) by now.
        self.maybe_end = false;
        let replay = self.budget_allows(julian);
        match self.capture_rec() {
            Err(e) => {
                self.incomplete(julian, None, &format!("capture at the start: {e}"), lines);
                Phase::Started {
                    julian,
                    deliveries: 0,
                    run: None,
                    id: 0,
                }
            }
            Ok((c, r)) => {
                let id = r.state.pass_counter;
                let run = if replay {
                    match self.replay(&c, &r, julian, pending) {
                        Ok(run) => Some(run),
                        Err(e) => {
                            self.incomplete(julian, Some(id), &e, lines);
                            None
                        }
                    }
                } else {
                    None
                };
                Phase::Started {
                    julian,
                    deliveries: 0,
                    run,
                    id,
                }
            }
        }
    }

    /// The original swapped its grids (the end of its state 6).
    fn on_swap(
        &mut self,
        started: Option<(Option<Box<Run>>, i32)>,
        julian: i32,
        old_published: u64,
        published: u64,
        deliveries: u32,
        lines: &mut Vec<String>,
    ) -> Phase {
        self.stats.passes_seen = self.stats.passes_seen.wrapping_add(1);
        self.period.seen += 1;
        let probe_on = old_published;
        let captured = self.capture_rec();
        let at_swap = captured.as_ref().ok().map(|(_, r)| r.state.clone());
        let phase = match captured {
            Err(e) => {
                let id = started.as_ref().map(|s| s.1);
                self.incomplete(julian, id, &format!("capture at the swap: {e}"), lines);
                self.note_unobserved_end();
                Phase::Idle
            }
            Ok((c, r)) => {
                let id = r.state.pass_counter;
                let mut run: Option<Box<Run>> = None;
                match started {
                    Some((Some(mut prev), start_id)) => {
                        if start_id != id {
                            // A deal status restarted the pass: replay its
                            // last run from this record.
                            match self.replay(&c, &r, julian, 8) {
                                Ok(mut again) => {
                                    again.why_inexact.push("restarted during the pass".to_string());
                                    run = Some(again);
                                }
                                Err(e) => self.incomplete(julian, Some(id), &e, lines),
                            }
                        } else {
                            let behind = prev.sim.with_state(|s| s.state);
                            if behind < 6 {
                                // The original ran more states than the
                                // deliveries we saw: ours finish now, later
                                // than its.
                                note_inexact(
                                    &mut prev,
                                    format!("our pass was at state {behind} at the original's swap"),
                                );
                            }
                            if let Err(e) = run_to(&prev.sim, 8) {
                                note_inexact(&mut prev, e);
                            }
                            let changed = changed_inputs(&prev.start, &prev.flags_seen, &r.state);
                            if !changed.is_empty() {
                                prev.why_inexact.push(changed.join(", "));
                            }
                            run = Some(prev);
                        }
                    }
                    Some((None, _)) => {}
                    None => {
                        if self.budget_allows(julian) {
                            match self.replay(&c, &r, julian, 8) {
                                Ok(mut again) => {
                                    again.why_inexact.push("start not seen".to_string());
                                    run = Some(again);
                                }
                                Err(e) => self.incomplete(julian, Some(id), &e, lines),
                            }
                        }
                    }
                }
                if let Some(run) = run.as_mut() {
                    self.compare_swap(run, published);
                }
                Phase::Swapped(Track {
                    id,
                    run,
                    after_swap: 0,
                    deliveries,
                })
            }
        };
        if self.host.o.pass_counter.is_none() {
            // Without the counter (PS6), the next start shows on a probe cell.
            self.choose_probe(probe_on, at_swap.as_ref());
        }
        phase
    }

    // ---- comparisons --------------------------------------------------------------------------------

    fn compare_swap(&mut self, run: &mut Run, published: u64) {
        let mut c = Cmp::default();
        let max = self.max_cells.max(LIST_MIN);
        // The grid, cell by cell.
        let geom = self.geometry(published);
        let (nx, nz) = (geom.count_x.clamp(0, 4096), geom.count_z.clamp(0, 4096));
        let ours = run.sim.watered_grid();
        let (mut differing, mut max_diff) = (0u64, 0i32);
        let mut firsts: Vec<String> = Vec::new();
        for z in 0..nz {
            for x in 0..nx {
                let o = self.live_cell(published, x, z);
                let b = ours.as_ref().map_or(0, |g| g.get(x, z));
                if o != b {
                    differing += 1;
                    max_diff = max_diff.max((i32::from(o) - i32::from(b)).abs());
                    if firsts.len() < self.max_cells {
                        firsts.push(format!("({x},{z}) {o}/{b}"));
                    }
                }
            }
        }
        let cells = (nx as u64) * (nz as u64);
        self.stats.cells = self.stats.cells.wrapping_add(cells);
        self.stats.cell_mismatches = self.stats.cell_mismatches.wrapping_add(differing);
        c.diffs += differing;
        c.cell_diffs = differing;
        let first = if firsts.is_empty() {
            String::new()
        } else {
            format!(" first (x,z) orig/ours: {}", firsts.join(", "))
        };
        c.parts.push(format!(
            "grid {nx}x{nz} cells={cells} differing={differing} max|diff|={max_diff}{first}"
        ));
        // Producers: production (the record), efficiency, polluted, used.
        let h = self.host.clone();
        let (mut prod, mut eff, mut pol, mut used) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for p in &run.start.producers {
            let occ = p.occupant;
            let ours1 = run.ours1.iter().find(|o| o.0 == occ);
            let (op, oe, opol) = ours1.map_or((None, None, None), |o| (Some(o.1), Some(o.2), Some(o.3)));
            prod.push((
                p.production.to_string(),
                op.map_or("absent".to_string(), |v| v.to_string()),
                op != Some(p.production),
            ));
            let mut info = fc4_plumbing_producer_info::default();
            let known = ocb!(h, producer_info, false, occ, &mut info);
            let ours_used = first_number(&h.props.borrow().get(&(occ, prop::WATER_USED)).cloned()).map(|v| v as u32);
            if known {
                eff.push((
                    info.efficiency.to_string(),
                    oe.map_or("absent".to_string(), |v| v.to_string()),
                    !oe.is_some_and(|v| same_f32(v, info.efficiency)),
                ));
                pol.push((
                    u8::from(info.polluted).to_string(),
                    opol.map_or("absent".to_string(), |v| u8::from(v).to_string()),
                    opol != Some(info.polluted),
                ));
                used.push((
                    info.used.to_string(),
                    ours_used.map_or("unset".to_string(), |v| v.to_string()),
                    ours_used != Some(info.used),
                ));
            } else {
                for l in [&mut eff, &mut pol, &mut used] {
                    l.push(("unknown".to_string(), "?".to_string(), true));
                }
            }
        }
        c.parts.push(format!("producers {}", run.start.producers.len()));
        c.list("production", &prod, max);
        c.list("efficiency", &eff, max);
        c.list("polluted", &pol, max);
        c.list("used", &used, max);
        c.scalar("numPolluted", run.start.num_polluted, run.polluted1);
        c.scalar("averagePollution", run.start.average_water_pollution, run.avg1);
        // Deal usage.
        let mut deals = Vec::new();
        for t in [deal::IMPORT, deal::EXPORT] {
            for d in h.live_deals(t) {
                let o = h.live_deal_used(d);
                let b = h.deals_used.borrow().get(&d).copied();
                deals.push((
                    format!("{d:#x} {o}"),
                    b.map_or("unset".to_string(), |v| v.to_string()),
                    b != Some(o),
                ));
            }
        }
        let bad: Vec<String> = deals
            .iter()
            .filter(|d| d.2)
            .take(max)
            .map(|(a, b, _)| format!("{a}/{b}"))
            .collect();
        let nbad = deals.iter().filter(|d| d.2).count() as u64;
        c.diffs += nbad;
        c.parts.push(format!(
            "deals compared={} differing={nbad}{}",
            deals.len(),
            if bad.is_empty() {
                String::new()
            } else {
                format!(" [used orig/ours: {}]", bad.join(", "))
            }
        ));
        // Water connections: active (state 5) and visited (states 0 and 3).
        let conns = h.live_connections();
        let (mut active_bad, mut visited_bad, mut visited_compared) = (Vec::new(), 0u64, 0u64);
        for &cn in &conns {
            let o = h.live_active(cn);
            let b = h.active.borrow().get(&cn).copied();
            if b != Some(o) {
                active_bad.push(format!(
                    "{cn:#x} {}/{}",
                    u8::from(o),
                    b.map_or("unset".into(), |v| u8::from(v).to_string())
                ));
            }
            if let Some(ov) = h.orig_visited(cn) {
                visited_compared += 1;
                if h.visited.borrow().get(&cn).copied() != Some(ov) {
                    visited_bad += 1;
                }
            }
        }
        c.diffs += active_bad.len() as u64 + visited_bad;
        let shown: Vec<String> = active_bad.iter().take(max).cloned().collect();
        c.parts.push(format!(
            "connections compared={} active differing={}{} visited compared={visited_compared} differing={visited_bad}",
            conns.len(),
            active_bad.len(),
            if shown.is_empty() {
                String::new()
            } else {
                format!(" [active orig/ours: {}]", shown.join(", "))
            }
        ));
        run.cmp = c;
    }

    fn compare_end(&mut self, run: &mut Run, end: &Rec) {
        let ours = run.sim.snapshot();
        let h = self.host.clone();
        let c = &mut run.cmp;
        c.scalar("produced", end.state.last.produced, ours.last.produced);
        c.scalar("consumed", end.state.last.consumed, ours.last.consumed);
        c.scalar("imported", end.state.last.imported, ours.last.imported);
        c.scalar("exported", end.state.last.exported, ours.last.exported);
        c.scalar("wateredCells", self.slot(21) as u32, ours.last_watered_cells);
        c.scalar(
            "unwateredCells",
            self.slot(22) as u32,
            ours.last_building_cells.wrapping_sub(ours.last_watered_cells),
        );
        c.scalar("cost", end.state.total_water_cost, ours.total_water_cost);
        // PS4 / PS8 (§5.3): demand is the original's running sum, moved by
        // the occupant messages delivered between the pass's steps and never
        // recomputed; ours carries it, taken from the record read now. The
        // getter (slot 23, low 32 bits) must agree with it.
        c.scalar("demand", self.slot(23) as i32, end.state.demand as i32);
        c.scalar("pipes", self.slot(18) as u32, ours.num_pipes);
        c.scalar("distressed", self.slot(19) as u32, ours.num_distressed);
        c.scalar("burst", self.slot(20) as u32, ours.num_burst);
        c.scalar("sources", self.slot(12) as u32, ours.producers.len() as u32);
        // Each producer's budget item after state 8 (§6.7 step 2).
        let maintenance: Vec<(u64, i64)> = h.maintenance.borrow().iter().map(|(k, v)| (*k, *v)).collect();
        let mut bad = Vec::new();
        for &(occ, expected) in &maintenance {
            let live = h.live_budget_item(occ, budget::PURPOSE).map(|i| i.cost);
            if live != Some(expected) {
                bad.push(format!(
                    "{occ:#x} {}/{expected}",
                    live.map_or("none".to_string(), |v| v.to_string())
                ));
            }
        }
        c.diffs += bad.len() as u64;
        c.parts.push(format!(
            "maintenance compared={} differing={}{}",
            maintenance.len(),
            bad.len(),
            if bad.is_empty() {
                String::new()
            } else {
                format!(" [cost orig/ours: {}]", bad.join(", "))
            }
        ));
        // Ageing (§6.7 step 3, §7.3): each producer's condition. With the
        // flag unknown, PS7: exactly the first state 8 after a month start
        // ages, and an ageing changes a producer's condition whenever its
        // increment is not 0, so a changed condition shows that the original
        // aged; none changed is compared as not aged (the same values
        // either way when every increment is 0).
        let mut ageing = run.ageing;
        let mut inferred = "";
        if ageing.is_none() {
            if let Some(before) = run.cond_before.take() {
                let num = |v: &Option<Variant>| first_number(v).map(|v| (v as f32).to_bits());
                let aged = before
                    .iter()
                    .any(|(o, v)| num(&h.live_prop(*o, prop::CONDITION)) != num(v));
                if aged {
                    // Our ageing from the conditions as they were before the
                    // original's state 8 (state 8 reads them before ageing).
                    for (o, v) in &before {
                        if let Some(v) = v {
                            h.props
                                .borrow_mut()
                                .entry((*o, prop::CONDITION))
                                .or_insert_with(|| v.clone());
                        }
                    }
                    run.sim.age_water_producers();
                    inferred = " (inferred: a producer's condition changed; plumbing.md §6.7, PS7)";
                } else {
                    inferred = " (inferred: no producer's condition changed; plumbing.md §6.7, PS7)";
                }
                ageing = Some(aged);
            }
        }
        match ageing {
            Some(true) => {
                let mut items = Vec::new();
                for p in &ours.producers {
                    let o = first_number(&h.live_prop(p.occupant, prop::CONDITION));
                    let b = first_number(&h.props.borrow().get(&(p.occupant, prop::CONDITION)).cloned());
                    let bad = match (o, b) {
                        (Some(a), Some(b)) => !same_f32(a as f32, b as f32),
                        (a, b) => a != b,
                    };
                    let show = |v: Option<f64>| v.map_or("absent".to_string(), |v| (v as f32).to_string());
                    items.push((show(o), show(b), bad));
                }
                c.parts.push(format!("ageing=yes{inferred}"));
                c.list("condition", &items, self.max_cells.max(LIST_MIN));
            }
            Some(false) => c.parts.push(format!("ageing=no{inferred}")),
            None => c.parts.push("ageing=unknown (condition not compared)".to_string()),
        }
    }

    /// The pass line (for a replayed pass); the end comparison is `Ok`, or
    /// why the end was not seen. Returns the counted differences (-1 for a
    /// pass that was only followed).
    fn end_line(&mut self, t: Track, julian: i32, end: Result<(), String>, lines: &mut Vec<String>) -> i32 {
        let Some(run) = t.run else {
            return -1;
        };
        let exact = run.why_inexact.is_empty();
        let total = run.cmp.diffs;
        let counted = if exact { total } else { 0 };
        self.stats.passes_compared = self.stats.passes_compared.wrapping_add(1);
        self.period.compared += 1;
        if exact {
            self.stats.scalar_mismatches = self
                .stats
                .scalar_mismatches
                .wrapping_add(total.saturating_sub(run.cmp.cell_diffs));
            self.period.differing += total;
        } else {
            self.stats.passes_not_exact = self.stats.passes_not_exact.wrapping_add(1);
            self.period.not_exact += 1;
        }
        let end_ok = end.is_ok();
        if !end_ok {
            self.stats.passes_incomplete = self.stats.passes_incomplete.wrapping_add(1);
            self.period.incomplete += 1;
        }
        if exact && end_ok && total == 0 {
            self.stats.passes_clean = self.stats.passes_clean.wrapping_add(1);
            self.period.clean += 1;
        }
        let mut parts = run.cmp.parts;
        if let Err(e) = &end {
            parts.push(format!("end not seen ({e}): totals not compared"));
        }
        let not_exact = if exact || total == 0 {
            String::new()
        } else {
            format!(" (not exact: {total} differences not counted)")
        };
        lines.push(format!(
            "plumbing shadow {} pass {}: aligned={} exact={}{}; {}; deliveries={} days={}; mismatches={counted}{not_exact}; \
             passes compared={} clean={}",
            date(julian),
            t.id,
            if run.aligned_start { "start" } else { "swap" },
            u8::from(exact),
            if exact {
                String::new()
            } else {
                format!(" ({})", run.why_inexact.join("; "))
            },
            parts.join("; "),
            t.deliveries,
            julian.wrapping_sub(run.start_julian),
            self.stats.passes_compared,
            self.stats.passes_clean
        ));
        counted.min(i32::MAX as u64) as i32
    }

    fn month_after(&mut self, julian: i32, mb: MonthBefore, lines: &mut Vec<String>) -> i32 {
        let (pipes, distressed, burst) = (self.slot(18) as u32, self.slot(19) as u32, self.slot(20) as u32);
        let t = self.tuning();
        let ours = pipe_funding(&t, self.tracked_condition, mb.pipes, mb.funding);
        let mut c = Cmp::default();
        c.parts.push(format!(
            "pipes={}{} distressedBefore={} burstBefore={}",
            mb.pipes,
            if pipes != mb.pipes {
                format!(" (now {pipes})")
            } else {
                String::new()
            },
            mb.distressed,
            mb.burst
        ));
        let new_bursts = i64::from(burst) - i64::from(mb.burst);
        let allowed = i64::from(ours.burst_max.saturating_sub(1).max(0));
        if !self.condition_known {
            // PS3: the original's condition is not saved and has no getter;
            // only a shadow made at the city start knows it (0).
            c.parts.push(format!(
                "distressed orig/ours={distressed}/{} burst +{new_bursts} (ours allows 0..{allowed}) (not exact: \
                 the original's pipe condition is not known, the shadow started mid-city; plumbing.md §7.1 PS3)",
                ours.target
            ));
        } else {
            c.scalar("distressed", distressed, ours.target);
            if new_bursts < 0 || new_bursts > allowed {
                c.diffs += 1;
                c.parts.push(format!("burst +{new_bursts} (ours allows 0..{allowed})!"));
            } else if allowed == 0 {
                c.parts.push(format!("burst orig/ours=+{new_bursts}/+0"));
            } else {
                c.parts.push(format!(
                    "burst +{new_bursts} (not exact: the original's own random draw; ours allows 0..{allowed})"
                ));
            }
        }
        let implied = if mb.pipes == 0 {
            "0 (no pipes)".to_string()
        } else {
            format!(
                "{:.4}..{:.4}",
                f64::from(distressed) / f64::from(mb.pipes),
                f64::from(distressed + 1) / f64::from(mb.pipes)
            )
        };
        c.parts.push(format!(
            "pipeCondition ours={:.4} (orig not readable; its distressed count implies {implied})",
            ours.condition
        ));
        if let Some(n) = &mb.funding_note {
            c.parts.push(n.clone());
        }
        self.tracked_condition = ours.condition;
        let n = c.diffs;
        self.stats.months = self.stats.months.wrapping_add(1);
        if n == 0 {
            self.stats.months_clean = self.stats.months_clean.wrapping_add(1);
        }
        self.stats.scalar_mismatches = self.stats.scalar_mismatches.wrapping_add(n);
        let p = std::mem::take(&mut self.period);
        lines.push(format!(
            "plumbing shadow {} month: {}; funding={:.3}; mismatches={n}; months={} clean={}; passes since the last \
             month line: seen={} compared={} clean={} notExact={} incomplete={} differing={}",
            date(julian),
            c.parts.join("; "),
            mb.funding,
            self.stats.months,
            self.stats.months_clean,
            p.seen,
            p.compared,
            p.clean,
            p.not_exact,
            p.incomplete,
            p.differing
        ));
        let r = n.min(i32::MAX as u64) as i32;
        self.stats.last_mismatches = r;
        r
    }
}

fn with_shadow<R>(s: *mut fc4_plumbing_shadow, fallback: R, f: impl FnOnce(&mut Shadow) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_plumbing_shadow*.
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

/// Creates a shadow. `host`: a plumbing host table of which only the read
/// callbacks are called (city geometry, tuning, ordinance factor, water
/// pollution, occupant reads, buildings in a cell, pipes, lots, terrain,
/// deals and connections, department funding and lookup, budget items) and
/// the grid table's `geometry` / `get`; the write callbacks may be NULL.
/// `orig`: the original simulator's getters and its capture; `watered_grid`,
/// `slot_value` and `capture` are required (`pass_counter` is optional, see
/// the module notes). The shadow starts as made mid-city; call
/// `fc4_plumbing_shadow_city_start` at once for one made at PostCityInit.
/// `max_cells`: differing cells
/// listed per line (0 → 8, at most 64). `passes_per_day`: passes replayed per
/// game day (0: every pass). NULL when a table is missing or short.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_shadow_create(
    host: *const fc4_plumbing_host,
    orig: *const fc4_plumbing_original,
    max_cells: u32,
    passes_per_day: u32,
) -> *mut fc4_plumbing_shadow {
    guard(std::ptr::null_mut(), || {
        let (Some(h), Some(o)) = (host.as_ref(), orig.as_ref()) else {
            return std::ptr::null_mut();
        };
        if (h.struct_size as usize) < std::mem::size_of::<fc4_plumbing_host>()
            || !h.grids.complete()
            || (o.struct_size as usize) < std::mem::size_of::<fc4_plumbing_original>()
            || o.watered_grid.is_none()
            || o.slot_value.is_none()
            || o.capture.is_none()
        {
            return std::ptr::null_mut();
        }
        let host = Rc::new(ShadowHost {
            c: *h,
            o: *o,
            parse: Cell::new(false),
            tuning: RefCell::new(None),
            props: RefCell::new(HashMap::new()),
            deals_used: RefCell::new(BTreeMap::new()),
            visited: RefCell::new(BTreeMap::new()),
            active: RefCell::new(BTreeMap::new()),
            maintenance: RefCell::new(BTreeMap::new()),
            grids: RefCell::new(HashMap::new()),
            swallowed: Cell::new(0),
        });
        let max_cells = match max_cells {
            0 => 8,
            n => n.min(64) as usize,
        };
        Box::into_raw(Box::new(Shadow {
            host,
            max_cells,
            passes_per_day,
            cur: None,
            phase: Phase::Idle,
            probe: None,
            probe_cursor: 0,
            ageing: None,
            // At creation the original may be anywhere in a pass.
            maybe_end: true,
            day: i32::MIN,
            day_replays: 0,
            // §3/§9: not saved, 0 after a load; the original's only when the
            // shadow is made at the city start (fc4_plumbing_shadow_city_start).
            tracked_condition: 0.0,
            condition_known: false,
            last_counter: None,
            tuning: None,
            period: Period::default(),
            stats: fc4_plumbing_shadow_stats {
                struct_size: std::mem::size_of::<fc4_plumbing_shadow_stats>() as u32,
                last_mismatches: -1,
                ..Default::default()
            },
        })) as *mut fc4_plumbing_shadow
    })
}

/// Marks a shadow made at the city start (PostCityInit), before its first
/// delivery: the original's pipe condition and ageing flag are its
/// constructor's, 0 and false, after every load (plumbing.md §7.1, PS3; the
/// ageing flag is not saved either), and no state 8 is pending. Without it
/// the shadow started mid-city: the month's distressed count is not exact and
/// the ageing is unknown until a month start. False for NULL or after a
/// delivery. (ABI 33)
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_shadow_city_start(s: *mut fc4_plumbing_shadow) -> bool {
    with_shadow(s, false, |sh| {
        if sh.stats.deliveries != 0 || sh.cur.is_some() {
            return false;
        }
        sh.condition_known = true;
        sh.tracked_condition = 0.0;
        sh.ageing = Some(false);
        sh.maybe_end = false;
        true
    })
}

/// Destroys a shadow (NULL is ignored).
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_shadow_destroy(s: *mut fc4_plumbing_shadow) {
    if s.is_null() {
        return;
    }
    guard((), || drop(Box::from_raw(s as *mut Shadow)));
}

/// Before the original's delivery of `msg_type` (SimIdle `0x6695681A` and
/// SimNewDay `0x66956814`: one `DoIdle`; SimNewMonth `0x66956816`; `julian` =
/// the message's data1). 0 = taken; -2 for any other type (nothing done).
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_shadow_before(s: *mut fc4_plumbing_shadow, msg_type: u32, julian: i32) -> i32 {
    if msg_type != msg::SIM_IDLE && msg_type != msg::SIM_NEW_DAY && msg_type != msg::SIM_NEW_MONTH {
        return -2;
    }
    with_shadow(s, -1, |sh| sh.before(msg_type, julian))
}

/// After the original's delivery: follows its pass and hands the log lines
/// to `sink`, one call per line (a pass line at the end of a replayed pass,
/// INCOMPLETE lines, the month line). Returns the counted differences of a
/// comparison finished in this delivery (a pass end or a month), -1 when none
/// finished.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_shadow_after(
    s: *mut fc4_plumbing_shadow,
    sink: fc4_bytes_sink,
    ctx: *mut c_void,
) -> i32 {
    let mut lines = Vec::new();
    let r = with_shadow(s, -1, |sh| sh.after(&mut lines));
    emit(&lines, sink, ctx);
    r
}

/// Totals since creation.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_shadow_get_stats(
    s: *mut fc4_plumbing_shadow,
    out: *mut fc4_plumbing_shadow_stats,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_shadow(s, false, |sh| {
        *o = sh.stats;
        true
    })
}
