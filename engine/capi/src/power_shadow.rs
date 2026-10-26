// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The power shadow (fc4.h "power shadow"): tooling, no spec behaviour. With
//! the ORIGINAL power simulator running, each of its SimNewMonth deliveries
//! and each SimNewDay / SimIdle delivery that does work (a `DoIdle` step,
//! power.md §6) is wrapped:
//!
//! 1. before: the plugin captures the original's §9.1 record (its own
//!    `Write` into a capture stream; every reference already queried for the
//!    occupant, pole, grid and department interfaces) and its unsaved plant
//!    counts (slots 27–29); [`fc4_power_shadow_begin`] loads the record into a
//!    private core simulator, copies the live powered grid it refers to and
//!    runs FreeCity4's step (`DoIdle` or `DoMonthlySimulation`). Every input
//!    is read through the host table as it stands before the original runs
//!    (and remembered); every write stays inside the shadow (occupant
//!    properties, deal usage, connection flags, budget item costs in overlays
//!    that later reads see; posts, history, demolitions, lots and budget lines
//!    are only counted; references are not counted). A month also rebuilds
//!    the bookkeeping from scratch (§5.1 for every live occupant, a wire
//!    rebuild of the whole map) for the bookkeeping line;
//! 2. the original's delivery, unchanged;
//! 3. after: the record and the counts again; [`fc4_power_shadow_finish`]
//!    compares. A step with an order-dependent shortfall network (§6.3 step
//!    6: one `rand()` per network, process-wide in the original) runs again
//!    with the original's powered bits as the block probe's target, which
//!    recovers the original's draw when FreeCity4 computes the same blocks.
//!    Repeated runs read the remembered inputs.
//!
//! The unsaved state the step needs is mirrored. The plugin creates the
//! shadow at the city's PostCityInit ([`FC4_POWER_SHADOW_CITY_START`]), when
//! the original was just constructed and initialised; a shadow created later
//! (switched on mid-city) does not know these values until the original shows
//! them:
//! - the ordinance factor (§8, PW7): the original reads it in `Init`, on a
//!   tuning reload and at each month start, never on an ordinance change. At
//!   a city start the shadow reads it then, on each forwarded tuning reload
//!   and at each month; mid-city it is unknown until the first month, and the
//!   differences of the steps that use it (states 1 and 2) are not exact;
//! - the pass import / export / demand (§9.1, PW3): 0 at a city start (the
//!   constructor's), then our previous step's; mid-city unknown until a
//!   state 0 is observed (the state 3 totals are not exact until then);
//! - `brokenFraction` (§7.6, PW4): 0 at a city start, then ours; mid-city it
//!   is never known, and the months' brokenCount is not exact;
//! - the dirty rectangle (§5.3, PW5): merged from the original's messages,
//!   which the plugin forwards ([`fc4_power_shadow_message`]) with
//!   [`PowerSim::dirty_rect_for`], the rectangle FreeCity4's `DoMessage`
//!   merges [choice, power.md §5.3]; consumed by each enabled step. Mid-city
//!   it is unknown until the first compared step, which runs again with the
//!   bounding box of the changed wire bits when its wire bits changed;
//! - `enabled` (latched once the original's `DoIdle` is seen working).
//!
//! The month's bookkeeping line (§5.2, PW8) sets apart what the original's
//! running counts inherited from the save: a difference equal to the
//! baseline's is inherited (case 1), one that changed since the last line is
//! in flight (case 2, occupant messages not yet delivered), and only one that
//! stays without being inherited counts as a mismatch [choice, power.md
//! §5.2]. The baseline is the first line equal to the line before it (the
//! first line can have messages in flight too), and it is never edited (an
//! entry masked by a difference in flight comes back as inherited). The wire
//! bits are listed and not counted: a whole-map rebuild differs from the
//! original's rectangle-by-rectangle bitmap by design (§5.3, PW5), and each
//! rebuild is compared on the cycle lines; see `Shadow::bookkeeping_line`.
//!
//! Not readable, hence not compared: the posted messages, history values,
//! budget line items, demolitions and lot conditions, the poles the month's
//! `random_shuffle` picks (the set depends on the original's `rand()`; the
//! broken count is compared), the deal flags.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::grid::{Bitmap, CellRect, CityDims, GridElem, GridProvider, SimGrid, TractGeometry, TractGrid};
use fc4_core::julian::civil_from_julian;
use fc4_core::power::{
    deal, msg, prop, state, BlockDraw, BudgetItem, Deal, LineInfo, PoleInfo, Posted, PowerHost, PowerMessage, PowerSim,
    PowerState, GRID_CLSID, IID_DEPARTMENT_BUDGET, IID_OCCUPANT, IID_POWER_POLE, IID_SIM_GRID, ORDINANCE_FACTOR_ID,
    PLANT_TYPES,
};
use fc4_core::property::PropertyHolder;
use fc4_core::stream::{ByteReader, InStream, RefValue, SaveObject};
use fc4_core::variant::Variant;

use crate::guard;
use crate::landvalue::fc4_grid_geometry;
use crate::power::{fc4_power_host, fc4_power_message, CHost};
use crate::stream::fc4_bytes_sink;

// ---- C types ---------------------------------------------------------------------------------------

/// One reference of a captured record (fc4.h `fc4_power_capture_ref`): the
/// object the original wrote, queried for each interface its reader asks
/// for (0 = the object does not answer it). Tokens are borrowed.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_power_capture_ref {
    /// As a `cISC4Occupant` (IID `0x26BB767E`): plants, and poles.
    pub occupant: u64,
    /// As a `cISC4SimGrid` (IID `0xC9BD939A`).
    pub grid: u64,
    /// As a `cISC4DepartmentBudget` (IID `0x223E2B67`).
    pub department: u64,
    /// `GetGZCLSID()`.
    pub class_id: u32,
    /// [`FC4_POWER_CAPTURE_POLE`]: the object answers `cISC4PowerPoleOccupant`.
    pub flags: u32,
}

/// `fc4_power_capture_ref::flags`: the object answers
/// `cISC4PowerPoleOccupant` (`0xA8C86142`); the pole is known by `occupant`.
pub const FC4_POWER_CAPTURE_POLE: u32 = 1;

/// A captured record (fc4.h `fc4_power_capture`): the bytes the original's
/// `Write` produced (primitives little-endian, references as `u32 objectID`
/// (index + 1, 0 = null) then `u32 classID`) and its references in write
/// order.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_power_capture {
    pub struct_size: u32,
    /// Non-zero when `Write` returned true and the stream stayed good.
    pub ok: u32,
    pub bytes: *const u8,
    pub len: usize,
    pub refs: *const fc4_power_capture_ref,
    pub ref_count: usize,
}

/// The original's unsaved plant counts (fc4.h `fc4_power_shadow_counts`):
/// slots 27, 28, 29.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_power_shadow_counts {
    pub struct_size: u32,
    /// Non-zero when read.
    pub ok: u32,
    pub overworked: u32,
    pub aged: u32,
    pub distressed: u32,
    pub reserved: u32,
}

/// Creation options (fc4.h `fc4_power_shadow_options`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_power_shadow_options {
    pub struct_size: u32,
    /// Differing cells listed per line (0 → 8, at most 64).
    pub max_cells: u32,
    /// Cycle lines per month besides the first cycle after the month and
    /// the clean ones (0 → 32; `u32::MAX`: every cycle).
    pub max_lines: u32,
    /// [`FC4_POWER_SHADOW_CITY_START`].
    pub flags: u32,
    /// `cISC4NeighborConnection::GetVisitedThisCycle` (slot 11) of a
    /// connection token, called with the host table's `user`. The shadow
    /// emulates `FindUnvisitedConnection` with it (its own visited flags
    /// stay in the shadow). NULL: the live `FindUnvisitedConnection`.
    pub connection_visited: Option<unsafe extern "C" fn(*mut c_void, u64) -> bool>,
}

/// `fc4_power_shadow_options::flags`: the shadow is created at the city's
/// PostCityInit (a load or a new city), when the original was just
/// constructed and initialised: its unsaved values are known (the module
/// notes). The host table must answer the ordinance factor at creation.
pub const FC4_POWER_SHADOW_CITY_START: u32 = 1;

/// Totals since creation (fc4.h `fc4_power_shadow_stats`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_power_shadow_stats {
    pub struct_size: u32,
    /// Deliveries wrapped (compared or not).
    pub deliveries: u32,
    /// SimNewDay / SimIdle deliveries passed through: the original was idle
    /// (state 4, no recompute) and no message had arrived since.
    pub idle_skipped: u32,
    /// `DoIdle` steps compared.
    pub steps: u32,
    /// Recompute cycles whose last step (state 3) was compared.
    pub cycles: u32,
    /// Months compared.
    pub months: u32,
    /// Deliveries compared without a difference.
    pub clean: u32,
    /// ... not compared (a capture or our run failed).
    pub incomplete: u32,
    /// Runs repeated (wire rebuild, block order).
    pub reruns: u32,
    /// `DoIdle` deliveries the original did not act on before it was seen
    /// working (`enabled` comes with ConnectionsReady, §5).
    pub not_enabled: u32,
    /// The last delivery's difference count (-1: not compared).
    pub last_mismatches: i32,
    /// Inputs a repeated run had not read before (read live, after the
    /// original's update).
    pub uncached_reads: u32,
    pub mismatches: u64,
    /// Differences set apart as not exact (block orders no start
    /// reproduces, pass totals not observed).
    pub not_exact: u64,
    pub bookkeeping_mismatches: u64,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_power_capture_ref>() == 32);
    assert!(std::mem::size_of::<fc4_power_capture>() == if cfg!(target_pointer_width = "32") { 24 } else { 40 });
    assert!(std::mem::size_of::<fc4_power_shadow_counts>() == 24);
    assert!(std::mem::size_of::<fc4_power_shadow_options>() == if cfg!(target_pointer_width = "32") { 20 } else { 24 });
    assert!(std::mem::size_of::<fc4_power_shadow_stats>() == 72);
};

/// Clean cycle lines per month (the first one after the month included).
const CLEAN_CYCLE_LINES: u32 = 4;
/// Occupant properties the cycle writes (§6.4, §7.4) or the insertion
/// writes (§5.1).
const WRITTEN_PROPS: [u32; 3] = [prop::USED, prop::WEAR, prop::BASE_MAINTENANCE];

// ---- the shadow's host -----------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    /// Before the original's update: live reads, remembered.
    Live,
    /// A repeated run: remembered reads first.
    Replay,
    /// Loading the record captured after the original's update: no grid.
    Parse,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Key {
    Factor(u32, u32),
    SimTime,
    OccType(u64),
    Prop(u64, u32),
    Position(u64),
    BBox(u64),
    BCells(u64),
    InManager(u64),
    BuildingType(u64),
    Pole(u64),
    PoleDistressed(u64),
    Line(u64),
    LinePoles(u64),
    NetCell(u64),
    HasNetFlag(u64, u32),
    NetFlags(u64),
    OccsIn(i32, i32, i32, i32),
    Zone(i32, i32),
    LotAt(i32, i32),
    OccLot(u64),
    AsLot(u64),
    LotRect(u64),
    LotState(u64),
    LotNeeds(u64),
    LotHas(u64),
    ViewControl,
    Deals(u32),
    DealUsed(u64),
    DealType(u64),
    ConnsOfType(u32),
    ConnsToCity(u32, u64),
    Visited(u64),
    FindUnvisited(u32, u64),
    OtherSide(u64),
    ConnCell(u64),
    ConnActive(u64),
    Department(u32),
    DeptFunding(u64),
    BudgetFunding(u64),
    LineItem(u64, u32),
    SecondaryInfo(u64),
    BudgetItem(u64, u32),
    LocallyFunded(u64, u64, u32),
}

#[derive(Clone)]
enum Val {
    U32(u32),
    I32(i32),
    I64(i64),
    F32(f32),
    B(bool),
    I2((i32, i32)),
    Prop(Option<Variant>),
    V3([f32; 3]),
    Box(([f32; 3], [f32; 3])),
    Rect(CellRect),
    OptU32(Option<u32>),
    OptU64(Option<u64>),
    Pole(Option<PoleInfo>),
    Line(Option<LineInfo>),
    Cell(Option<(i32, i32)>),
    Pair(Option<(u64, u64)>),
    Tokens(Vec<u64>),
    Deals(Vec<Deal>),
    Item(Option<BudgetItem>),
}

/// Calls the shadow kept to itself (not readable in the game, so not
/// compared).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Swallowed {
    posts: u32,
    history: u32,
    demolitions: u32,
    lot_writes: u32,
    budget_writes: u32,
    other: u32,
}

/// Our writes, seen by our later reads and compared after the update.
#[derive(Clone, Default)]
struct Overlay {
    props: HashMap<(u64, u32), Option<Variant>>,
    prop_order: Vec<(u64, u32)>,
    deals: BTreeMap<u64, i32>,
    visited: BTreeMap<u64, bool>,
    active: BTreeMap<u64, bool>,
    distressed: BTreeMap<u64, bool>,
    lot_power: BTreeMap<u64, bool>,
    /// (occupant, item index) → cost / line.
    item_cost: BTreeMap<(u64, u32), i64>,
    item_line: BTreeMap<(u64, u32), u32>,
    secondary: BTreeMap<u64, i64>,
    swallowed: Swallowed,
}

/// A live grid as it stood before the original's update.
#[derive(Clone)]
struct BaseGrid {
    geom: TractGeometry,
    values: Vec<u8>,
}

/// The shadow's `PowerHost`: reads go to the C host (remembered), writes
/// stay here.
struct ShadowHost {
    live: CHost,
    connection_visited: Option<unsafe extern "C" fn(*mut c_void, u64) -> bool>,
    mode: Cell<Mode>,
    cache: RefCell<HashMap<Key, Val>>,
    /// Read once per shadow.
    cells: Cell<Option<(i32, i32)>>,
    widths: Cell<Option<(f32, f32)>>,
    tuning: RefCell<Option<Option<Rc<PropertyHolder>>>>,
    overlay: RefCell<Overlay>,
    /// The powered grids as they stood before the update (by token) and the
    /// current run's working copies.
    base: RefCell<HashMap<u64, BaseGrid>>,
    grids: RefCell<HashMap<u64, Rc<SimGrid<u8>>>>,
    uncached: Cell<u32>,
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

    fn swallow(&self, f: impl FnOnce(&mut Swallowed)) {
        f(&mut self.overlay.borrow_mut().swallowed);
    }

    /// Starts a run: the overlays and working grids are emptied; a live run
    /// forgets the remembered reads.
    fn begin_run(&self, mode: Mode) {
        self.mode.set(mode);
        if mode == Mode::Live {
            self.cache.borrow_mut().clear();
            self.base.borrow_mut().clear();
        }
        *self.overlay.borrow_mut() = Overlay::default();
        self.grids.borrow_mut().clear();
    }

    fn live_grid_value(&self, token: u64, x: i32, z: i32) -> u8 {
        let api = &self.live.0.grids;
        match api.get {
            // SAFETY: valid callback on a live game grid.
            Some(f) => u8::from_f64(unsafe { f(api.user, token, u8::CLSID, x, z) }),
            None => 0,
        }
    }

    fn live_geometry(&self, token: u64) -> TractGeometry {
        let api = &self.live.0.grids;
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

    /// The live grid's values, x outer.
    fn copy_live(&self, token: u64) -> BaseGrid {
        let geom = self.live_geometry(token);
        let (nx, nz) = (geom.count_x.clamp(0, 4096), geom.count_z.clamp(0, 4096));
        let mut values = Vec::with_capacity((nx * nz) as usize);
        for x in 0..nx {
            for z in 0..nz {
                values.push(self.live_grid_value(token, x, z));
            }
        }
        BaseGrid { geom, values }
    }

    fn city(&self) -> CityDims {
        let (cells_x, cells_z) = self.cell_counts();
        let (cell_width_x, cell_width_z) = self.cell_widths();
        CityDims {
            cells_x,
            cells_z,
            cell_width_x,
            cell_width_z,
        }
    }

    /// `FindUnvisitedConnection` emulated: the first connection of
    /// `GetConnectionsOfTypeToCity(type, city)` whose visited flag (ours if
    /// we set it, else the live `GetVisitedThisCycle`) is false. power.md
    /// §6.5 (PW1): the game's walks the connection list from its head in
    /// append order and returns the first of type `type`, visited bit clear,
    /// other side `city`: the same connections in the same order.
    fn emulated_unvisited(&self, f: unsafe extern "C" fn(*mut c_void, u64) -> bool, t: u32, city: u64) -> Option<u64> {
        for c in self.connections_to_city(t, city) {
            let ours = self.overlay.borrow().visited.get(&c).copied();
            let visited = match ours {
                Some(v) => v,
                // SAFETY: valid callback on a live connection.
                None => cached!(self, Key::Visited(c), B, unsafe { f(self.live.0.user, c) }),
            };
            if !visited {
                return Some(c);
            }
        }
        None
    }
}

macro_rules! cached {
    ($s:expr, $key:expr, $var:ident, $live:expr) => {
        $s.read(
            $key,
            Val::$var,
            |v| match v {
                Val::$var(x) => Some(x.clone()),
                _ => None,
            },
            || $live,
        )
    };
}
use cached;

impl GridProvider<u8> for ShadowHost {
    /// A working grid of the city's size (the bookkeeping rebuild's `Init`).
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        Some(SimGrid::<u8>::create(&self.city(), instance, tract_size) as Rc<dyn TractGrid<u8>>)
    }
    /// The run's copy of the live grid the record refers to (none while
    /// parsing the record captured after the update).
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u8>>> {
        let RefValue::Host { token, .. } = *r else {
            return None;
        };
        if token == 0 || self.mode.get() == Mode::Parse {
            return None;
        }
        if let Some(g) = self.grids.borrow().get(&token) {
            return Some(g.clone() as Rc<dyn TractGrid<u8>>);
        }
        if !self.base.borrow().contains_key(&token) {
            let b = self.copy_live(token);
            self.base.borrow_mut().insert(token, b);
        }
        let b = self.base.borrow()[&token].clone();
        let g = SimGrid::from_values(b.geom, &b.values);
        self.grids.borrow_mut().insert(token, g.clone());
        Some(g as Rc<dyn TractGrid<u8>>)
    }
}

impl PowerHost for ShadowHost {
    fn cell_counts(&self) -> (i32, i32) {
        if let Some(c) = self.cells.get() {
            return c;
        }
        let c = self.live.cell_counts();
        self.cells.set(Some(c));
        c
    }
    fn cell_widths(&self) -> (f32, f32) {
        if let Some(w) = self.widths.get() {
            return w;
        }
        let w = self.live.cell_widths();
        self.widths.set(Some(w));
        w
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        if let Some(t) = self.tuning.borrow().as_ref() {
            return t.clone();
        }
        let t = self.live.tuning();
        *self.tuning.borrow_mut() = Some(t.clone());
        t
    }
    fn cumulative_factor(&self, id: u32, index: u32) -> f32 {
        cached!(
            self,
            Key::Factor(id, index),
            F32,
            self.live.cumulative_factor(id, index)
        )
    }
    fn sim_time(&self) -> u32 {
        cached!(self, Key::SimTime, U32, self.live.sim_time())
    }
    fn acquire_services(&self) {}
    fn release_services(&self) {}
    fn add_agent(&self, _level: u32, _name: &str, _flags: u32) {}
    fn remove_agent(&self) {}
    fn subscribe(&self, _msg_types: &[u32]) {}
    fn unsubscribe(&self, _msg_types: &[u32]) {}
    fn post(&self, _msg: &Posted) {
        self.swallow(|s| s.posts += 1);
    }
    fn record_history(&self, _series: u32, _value: i32) {
        self.swallow(|s| s.history += 1);
    }
    fn set_history_queue_length(&self, _series: u32, _length: u32) {}
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
    fn occupant_type(&self, occ: u64) -> u32 {
        cached!(self, Key::OccType(occ), U32, self.live.occupant_type(occ))
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant> {
        if let Some(v) = self.overlay.borrow().props.get(&(occ, id)) {
            return v.clone();
        }
        cached!(self, Key::Prop(occ, id), Prop, self.live.occupant_property(occ, id))
    }
    fn set_occupant_property(&self, occ: u64, id: u32, value: Variant) {
        let mut o = self.overlay.borrow_mut();
        if o.props.insert((occ, id), Some(value)).is_none() {
            o.prop_order.push((occ, id));
        }
    }
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        cached!(self, Key::Position(occ), V3, self.live.occupant_position(occ))
    }
    fn occupant_bounding_box(&self, occ: u64) -> ([f32; 3], [f32; 3]) {
        cached!(self, Key::BBox(occ), Box, self.live.occupant_bounding_box(occ))
    }
    fn occupant_bounding_cells(&self, occ: u64) -> CellRect {
        cached!(self, Key::BCells(occ), Rect, self.live.occupant_bounding_cells(occ))
    }
    fn occupant_in_manager(&self, occ: u64) -> bool {
        cached!(self, Key::InManager(occ), B, self.live.occupant_in_manager(occ))
    }
    fn building_type(&self, occ: u64) -> Option<u32> {
        cached!(self, Key::BuildingType(occ), OptU32, self.live.building_type(occ))
    }
    fn power_pole(&self, occ: u64) -> Option<PoleInfo> {
        cached!(self, Key::Pole(occ), Pole, self.live.power_pole(occ))
    }
    fn break_all_connections(&self, _pole: u64, _flag: bool) {
        self.swallow(|s| s.other += 1);
    }
    fn pole_is_distressed(&self, pole: u64) -> bool {
        if let Some(&v) = self.overlay.borrow().distressed.get(&pole) {
            return v;
        }
        cached!(self, Key::PoleDistressed(pole), B, self.live.pole_is_distressed(pole))
    }
    fn set_pole_distressed(&self, pole: u64, on: bool) {
        self.overlay.borrow_mut().distressed.insert(pole, on);
    }
    fn power_line(&self, occ: u64) -> Option<LineInfo> {
        cached!(self, Key::Line(occ), Line, self.live.power_line(occ))
    }
    fn line_connected_poles(&self, line: u64) -> Option<(u64, u64)> {
        cached!(self, Key::LinePoles(line), Pair, self.live.line_connected_poles(line))
    }
    fn remove_pole_connection(&self, _first: u64, _second: u64) {
        self.swallow(|s| s.other += 1);
    }
    fn network_occupant_cell(&self, occ: u64) -> Option<(i32, i32)> {
        cached!(self, Key::NetCell(occ), Cell, self.live.network_occupant_cell(occ))
    }
    fn has_network_flag(&self, occ: u64, flag: u32) -> bool {
        cached!(
            self,
            Key::HasNetFlag(occ, flag),
            B,
            self.live.has_network_flag(occ, flag)
        )
    }
    fn network_flags(&self, occ: u64) -> u32 {
        cached!(self, Key::NetFlags(occ), U32, self.live.network_flags(occ))
    }
    fn occupants_in_cells(&self, rect: CellRect) -> Vec<u64> {
        cached!(
            self,
            Key::OccsIn(rect.x1, rect.z1, rect.x2, rect.z2),
            Tokens,
            self.live.occupants_in_cells(rect)
        )
    }
    fn remove_occupant(&self, _occ: u64) {
        self.swallow(|s| s.other += 1);
    }
    fn zone_type(&self, x: i32, z: i32) -> u32 {
        cached!(self, Key::Zone(x, z), U32, self.live.zone_type(x, z))
    }
    fn power_line_tool_zone_change(&self, _rect: CellRect, _data3: u32) {
        self.swallow(|s| s.other += 1);
    }
    fn lot_at(&self, x: i32, z: i32) -> Option<u64> {
        cached!(self, Key::LotAt(x, z), OptU64, self.live.lot_at(x, z))
    }
    fn occupant_lot(&self, occ: u64) -> Option<u64> {
        cached!(self, Key::OccLot(occ), OptU64, self.live.occupant_lot(occ))
    }
    fn as_lot(&self, token: u64) -> Option<u64> {
        cached!(self, Key::AsLot(token), OptU64, self.live.as_lot(token))
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        cached!(self, Key::LotRect(lot), Rect, self.live.lot_bounding_rect(lot))
    }
    fn lot_state(&self, lot: u64) -> u32 {
        cached!(self, Key::LotState(lot), U32, self.live.lot_state(lot))
    }
    fn lot_needs_power(&self, lot: u64) -> bool {
        cached!(self, Key::LotNeeds(lot), B, self.live.lot_needs_power(lot))
    }
    fn lot_has_power(&self, lot: u64) -> bool {
        if let Some(&v) = self.overlay.borrow().lot_power.get(&lot) {
            return v;
        }
        cached!(self, Key::LotHas(lot), B, self.live.lot_has_power(lot))
    }
    fn set_lot_has_power(&self, lot: u64, on: bool) {
        self.overlay.borrow_mut().lot_power.insert(lot, on);
    }
    fn set_lot_condition(&self, _lot: u64, _condition: u32) {
        self.swallow(|s| s.lot_writes += 1);
    }
    fn demolish_lots(&self, _lots: &[u64], _flags: u32) {
        self.swallow(|s| s.demolitions += 1);
    }
    fn add_radiation(&self, _position: [f32; 3], _strength: f32, _radius: f32) {
        self.swallow(|s| s.other += 1);
    }
    fn current_view_input_control(&self) -> Option<u32> {
        cached!(self, Key::ViewControl, OptU32, self.live.current_view_input_control())
    }
    fn remove_current_view_input_control(&self) {
        self.swallow(|s| s.other += 1);
    }
    fn find_deals(&self, deal_type: u32) -> Vec<Deal> {
        cached!(self, Key::Deals(deal_type), Deals, self.live.find_deals(deal_type))
    }
    fn deal_amount_used(&self, d: u64) -> i32 {
        if let Some(&v) = self.overlay.borrow().deals.get(&d) {
            return v;
        }
        cached!(self, Key::DealUsed(d), I32, self.live.deal_amount_used(d))
    }
    fn set_deal_amount_used(&self, d: u64, used: i32) {
        self.overlay.borrow_mut().deals.insert(d, used);
    }
    fn set_deal_flags(&self, _deal: u64, _flags: u32, _on: bool) {
        self.swallow(|s| s.other += 1);
    }
    fn deal_type(&self, token: u64) -> Option<u32> {
        cached!(self, Key::DealType(token), OptU32, self.live.deal_type(token))
    }
    fn mark_deals_dirty(&self) {
        self.swallow(|s| s.other += 1);
    }
    fn connections_of_type(&self, t: u32) -> Vec<u64> {
        cached!(self, Key::ConnsOfType(t), Tokens, self.live.connections_of_type(t))
    }
    fn connections_to_city(&self, t: u32, city: u64) -> Vec<u64> {
        cached!(
            self,
            Key::ConnsToCity(t, city),
            Tokens,
            self.live.connections_to_city(t, city)
        )
    }
    fn find_unvisited_connection(&self, t: u32, city: u64) -> Option<u64> {
        match self.connection_visited {
            Some(f) => self.emulated_unvisited(f, t, city),
            None => {
                // Without the visited getter: the live answer, which does not
                // see our own flags; a connection we already visited ends the
                // loop instead of repeating it.
                let c = cached!(
                    self,
                    Key::FindUnvisited(t, city),
                    OptU64,
                    self.live.find_unvisited_connection(t, city)
                )?;
                (!self.overlay.borrow().visited.get(&c).copied().unwrap_or(false)).then_some(c)
            }
        }
    }
    fn connection_on_other_side(&self, conn: u64) -> Option<u64> {
        cached!(
            self,
            Key::OtherSide(conn),
            OptU64,
            self.live.connection_on_other_side(conn)
        )
    }
    fn connection_cell(&self, conn: u64) -> (i32, i32) {
        cached!(self, Key::ConnCell(conn), I2, self.live.connection_cell(conn))
    }
    fn set_connection_visited(&self, conn: u64, on: bool) {
        self.overlay.borrow_mut().visited.insert(conn, on);
    }
    fn connection_is_active(&self, conn: u64) -> bool {
        if let Some(&v) = self.overlay.borrow().active.get(&conn) {
            return v;
        }
        cached!(self, Key::ConnActive(conn), B, self.live.connection_is_active(conn))
    }
    fn set_connection_active(&self, conn: u64, on: bool) {
        self.overlay.borrow_mut().active.insert(conn, on);
    }
    fn department(&self, id: u32) -> Option<u64> {
        cached!(self, Key::Department(id), OptU64, self.live.department(id))
    }
    fn create_department(&self, _id: u32, _group: u32) -> Option<u64> {
        self.swallow(|s| s.budget_writes += 1);
        None
    }
    fn set_department_name(&self, _dept: u64, _group: u32, _instance: u32) {
        self.swallow(|s| s.budget_writes += 1);
    }
    fn set_department_max_funding(&self, _dept: u64, _fraction: f32) {
        self.swallow(|s| s.budget_writes += 1);
    }
    fn department_funding(&self, dept: u64) -> f32 {
        cached!(self, Key::DeptFunding(dept), F32, self.live.department_funding(dept))
    }
    fn budget_funding(&self, dept: u64) -> i64 {
        cached!(self, Key::BudgetFunding(dept), I64, self.live.budget_funding(dept))
    }
    fn create_line_item(&self, _dept: u64, _line: u32, _locally_funded: bool) -> Option<u64> {
        self.swallow(|s| s.budget_writes += 1);
        None
    }
    fn create_line_item_for_building_type(&self, _dept: u64, _line: u32, _flag: bool) -> Option<u64> {
        self.swallow(|s| s.budget_writes += 1);
        None
    }
    fn line_item(&self, dept: u64, line: u32) -> Option<u64> {
        cached!(self, Key::LineItem(dept, line), OptU64, self.live.line_item(dept, line))
    }
    fn set_line_item_name(&self, _item: u64, _group: u32, _instance: u32) {
        self.swallow(|s| s.budget_writes += 1);
    }
    fn set_line_item_display_flag(&self, _item: u64, _flag: u32, _on: bool) {
        self.swallow(|s| s.budget_writes += 1);
    }
    fn line_item_secondary_info(&self, item: u64) -> i64 {
        if let Some(&v) = self.overlay.borrow().secondary.get(&item) {
            return v;
        }
        cached!(
            self,
            Key::SecondaryInfo(item),
            I64,
            self.live.line_item_secondary_info(item)
        )
    }
    fn set_line_item_secondary_info(&self, item: u64, value: i64) {
        self.overlay.borrow_mut().secondary.insert(item, value);
    }
    fn set_line_item_full_expenses(&self, _item: u64, _value: i64) {
        self.swallow(|s| s.budget_writes += 1);
    }
    fn budget_item_for_purpose(&self, occ: u64, purpose: u32) -> Option<BudgetItem> {
        let item = cached!(
            self,
            Key::BudgetItem(occ, purpose),
            Item,
            self.live.budget_item_for_purpose(occ, purpose)
        );
        item.map(|mut i| {
            let o = self.overlay.borrow();
            if let Some(&c) = o.item_cost.get(&(occ, i.index)) {
                i.cost = c;
            }
            if let Some(&l) = o.item_line.get(&(occ, i.index)) {
                i.line = l;
            }
            i
        })
    }
    fn change_budget_item_line(&self, occ: u64, index: u32, line: u32) {
        self.overlay.borrow_mut().item_line.insert((occ, index), line);
    }
    fn change_budget_item_cost(&self, occ: u64, index: u32, cost: i64) {
        self.overlay.borrow_mut().item_cost.insert((occ, index), cost);
    }
    fn add_locally_funded_object(&self, _dept: u64, _occ: u64, _purpose: u32) {
        self.swallow(|s| s.budget_writes += 1);
    }
    fn remove_locally_funded_object(&self, _dept: u64, _occ: u64, _purpose: u32) {
        self.swallow(|s| s.budget_writes += 1);
    }
    fn is_locally_funded_object(&self, dept: u64, occ: u64, purpose: u32) -> bool {
        cached!(
            self,
            Key::LocallyFunded(dept, occ, purpose),
            B,
            self.live.is_locally_funded_object(dept, occ, purpose)
        )
    }
    fn set_local_funding_percent(&self, _dept: u64, _occ: u64, _fraction: f32, _purpose: u32) -> bool {
        self.swallow(|s| s.budget_writes += 1);
        true
    }
    fn set_local_full_funding(&self, _dept: u64, _occ: u64, _amount: i64, _purpose: u32) {
        self.swallow(|s| s.budget_writes += 1);
    }
}

// ---- the captured record ---------------------------------------------------------------------------

/// A captured record as a core input stream: references are answered with
/// the token of the interface the reader asks for.
struct CaptureReader<'a> {
    r: ByteReader<'a>,
    refs: &'a [fc4_power_capture_ref],
    /// The grid references in read order (0 = null or not a grid).
    grid_tokens: Vec<u64>,
}

impl<'a> CaptureReader<'a> {
    fn new(bytes: &'a [u8], refs: &'a [fc4_power_capture_ref]) -> Self {
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
                IID_POWER_POLE if e.flags & FC4_POWER_CAPTURE_POLE != 0 => e.occupant,
                IID_DEPARTMENT_BUDGET => e.department,
                _ => 0,
            };
        }
        if iid == IID_SIM_GRID {
            self.grid_tokens.push(token);
        }
        // The ref helper's semantics: an object without the interface reads
        // as null.
        Some(if token != 0 {
            RefValue::Host { token, class_id }
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

/// A capture copied out of the caller's buffers.
#[derive(Clone, Default)]
struct Capture {
    bytes: Vec<u8>,
    refs: Vec<fc4_power_capture_ref>,
}

/// Copies a capture; `Err` with the reason when it is not usable.
unsafe fn capture_of(c: *const fc4_power_capture) -> Result<Capture, &'static str> {
    let Some(c) = c.as_ref() else {
        return Err("no capture");
    };
    if (c.struct_size as usize) < std::mem::size_of::<fc4_power_capture>() {
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

/// The counts the plugin read, if it did.
unsafe fn counts_of(c: *const fc4_power_shadow_counts) -> Option<[u32; 3]> {
    let c = c.as_ref()?;
    ((c.struct_size as usize) >= std::mem::size_of::<fc4_power_shadow_counts>() && c.ok != 0).then_some([
        c.overworked,
        c.aged,
        c.distressed,
    ])
}

// ---- the shadow -------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_power_shadow`).
#[repr(C)]
pub struct fc4_power_shadow {
    _private: [u8; 0],
}

/// The dirty rectangle the original rebuilds at its next enabled `DoIdle`
/// (power.md §5.3; not saved).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dirty {
    /// Messages may have reached the original before the shadow saw them.
    Unknown,
    /// Merged from the forwarded messages (None: not dirty).
    Known(Option<CellRect>),
}

impl Dirty {
    /// §5.3's merge: the union with `r` (or `r` when not dirty).
    fn merge(self, r: CellRect) -> Dirty {
        match self {
            Dirty::Unknown => Dirty::Unknown,
            Dirty::Known(None) => Dirty::Known(Some(r)),
            Dirty::Known(Some(d)) => Dirty::Known(Some(CellRect::new(
                d.x1.min(r.x1),
                d.z1.min(r.z1),
                d.x2.max(r.x2),
                d.z2.max(r.z2),
            ))),
        }
    }

    /// `self` followed by the messages merged into `later`.
    fn then(self, later: Dirty) -> Dirty {
        match later {
            Dirty::Unknown => Dirty::Unknown,
            Dirty::Known(None) => self,
            Dirty::Known(Some(r)) => self.merge(r),
        }
    }
}

/// The unsaved state of the original that its next step needs (module
/// notes).
#[derive(Clone, Debug)]
struct Mirror {
    /// Created at the city's PostCityInit ([`FC4_POWER_SHADOW_CITY_START`]).
    city_start: bool,
    /// The ordinance factor as the original holds it, None before the first
    /// run (then read live).
    factor: Option<f32>,
    /// `factor` is the original's: read at the city start, at a tuning
    /// reload or at a month start (power.md §8, PW7).
    factor_known: bool,
    months: u32,
    broken_fraction: f32,
    /// `broken_fraction` is the original's: 0 at a city start (§7.6, PW4).
    fraction_known: bool,
    /// Our pass import, export, demand after our last step (None: unknown,
    /// until a state 0 is compared; 0 at a city start, §9.1 PW3).
    pass: Option<[u32; 3]>,
    /// The original's dirty rectangle (§5.3, PW5).
    dirty: Dirty,
    /// The last record captured after an update: state 4 without a
    /// recompute request (nothing to do until a message arrives).
    idle: bool,
    known: bool,
    /// The original's `DoIdle` was seen working.
    enabled: bool,
    last_julian: Option<i32>,
    first_julian: Option<i32>,
}

impl Default for Mirror {
    /// A shadow started mid-city: nothing unsaved is known.
    fn default() -> Self {
        Mirror {
            city_start: false,
            factor: None,
            factor_known: false,
            months: 0,
            broken_fraction: 0.0,
            fraction_known: false,
            pass: None,
            dirty: Dirty::Unknown,
            idle: false,
            known: false,
            enabled: false,
            last_julian: None,
            first_julian: None,
        }
    }
}

impl Mirror {
    /// At the city's PostCityInit: the original's constructor and `Init`
    /// left the pass values and brokenFraction at 0, no dirty rectangle
    /// (`Init` does not touch wireDirty after a load; a new city clears it)
    /// and the factor `Init` read, which is `factor` unless an ordinance
    /// changed in between (PW7: the original only reads it again at a month
    /// start or a tuning reload).
    fn at_city_start(factor: f32) -> Mirror {
        Mirror {
            city_start: true,
            factor: Some(factor),
            factor_known: true,
            fraction_known: true,
            pass: Some([0; 3]),
            dirty: Dirty::Known(None),
            ..Mirror::default()
        }
    }
}

/// One run of ours.
struct Run {
    before: PowerState,
    state: PowerState,
    /// The grid token the record refers to and our copy's values after the
    /// run (x outer).
    grid: Option<(u64, Vec<u8>)>,
    overlay: Overlay,
    draws: Vec<BlockDraw>,
}

/// What a rerun changes.
#[derive(Clone, Default)]
struct Rerun {
    wire_rect: Option<CellRect>,
    target: Option<Bitmap>,
}

/// The month's bookkeeping rebuild (§5.1 for every live occupant).
struct Bookkeeping {
    occupants: usize,
    state: PowerState,
}

/// The delivery between begin and finish.
struct Pending {
    msg_type: u32,
    julian: i32,
    before: Capture,
    counts: Option<[u32; 3]>,
    pass_known: bool,
    /// The dirty rectangle the step consumes (a month: none).
    dirty: Dirty,
    first: Result<Run, String>,
    book: Option<Result<Bookkeeping, String>>,
}

/// One difference of the bookkeeping line (§5.2, PW8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum BookKey {
    Consumers,
    Requirement,
    PlantCount,
    PoleCount,
    CostBase,
    PerType(usize),
    /// A plant (pole) held by the original and not found live (+1), or
    /// found live and not held (−1).
    Plant(u64),
    Pole(u64),
    /// A consumption-map cell.
    Cell(i32, i32),
    /// A wire bit: the original's set and ours not (+1), or the other way.
    Wire(i32, i32),
}

impl BookKey {
    fn show(self, v: i64) -> String {
        let side = |v: i64| if v > 0 { "orig only" } else { "live only" };
        match self {
            BookKey::Consumers => format!("consumers {v:+}"),
            BookKey::Requirement => format!("requirement {v:+}"),
            BookKey::PlantCount => format!("plantCount {v:+}"),
            BookKey::PoleCount => format!("poleCount {v:+}"),
            BookKey::CostBase => format!("costBase {v:+}"),
            BookKey::PerType(t) => format!("perType[{t}] {v:+}"),
            BookKey::Plant(o) => format!("plant {o:#x} {}", side(v)),
            BookKey::Pole(o) => format!("pole {o:#x} {}", side(v)),
            BookKey::Cell(x, z) => format!("cell ({x},{z}) {v:+}"),
            BookKey::Wire(x, z) => format!("wire ({x},{z}) {}", if v > 0 { "1/0" } else { "0/1" }),
        }
    }
}

/// The bookkeeping differences, original − ours (non-zero only).
type BookDiff = BTreeMap<BookKey, i64>;

/// The bookkeeping lines so far.
#[derive(Default)]
struct BookHistory {
    /// The baseline: the differences of the first line that equals the line
    /// before it (nothing in flight at either), what the save left behind
    /// (case 1). Never edited once set: an entry masked at a line by a
    /// difference in flight comes back as inherited.
    base: Option<BookDiff>,
    base_date: Option<i32>,
    /// Lines written so far.
    lines: u32,
    /// The previous line's differences, and those of the line before it.
    prev: BookDiff,
    prev2: BookDiff,
}

/// One key's place on a bookkeeping line once the baseline is set (§5.2,
/// PW8; see `Shadow::bookkeeping_line`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BookClass {
    /// The baseline's difference (case 1).
    Inherited,
    /// Changed since the last line (case 2).
    InFlight,
    /// A baseline entry whose difference is 0 at this line: masked by a
    /// difference in flight that cancels it, or in flight at the baseline.
    Absent,
    /// Not inherited and unchanged since the last line, at a line where
    /// other differences changed: counted at the next line.
    Waiting,
    /// Not inherited and unchanged since the last line at a line where
    /// nothing changed, or unchanged at the last two lines: a mismatch.
    Persistent,
}

/// Classifies one key: `v` its difference at this line, `b0` the baseline's,
/// `p1` / `p2` the last two lines', `quiet` whether no difference of the
/// line changed since the last line. None: no difference and none expected.
fn book_class(v: i64, b0: i64, p1: i64, p2: i64, quiet: bool) -> Option<BookClass> {
    if v == 0 {
        return (b0 != 0).then_some(BookClass::Absent);
    }
    Some(if v == b0 {
        BookClass::Inherited
    } else if v != p1 {
        BookClass::InFlight
    } else if quiet || p1 == p2 {
        BookClass::Persistent
    } else {
        BookClass::Waiting
    })
}

/// One comparison's findings.
#[derive(Default)]
struct Diff {
    mismatches: u64,
    not_exact: u64,
    rect_rewrites: u64,
    max_diff: i32,
    grid_cells: u64,
    parts: Vec<String>,
    first: Vec<String>,
    fields: Vec<(&'static str, String, String)>,
    props: (u64, u64),
    deals: (u64, u64),
    connections: (u64, u64),
    items: (u64, u64),
}

/// The current recompute cycle (from its 4 → 0 step to its state 3 step).
#[derive(Default)]
struct CycleAcc {
    steps: Vec<String>,
    mismatches: u64,
    not_exact: u64,
    rect_rewrites: u64,
    max_diff: i32,
    grid_cells: u64,
    first: Vec<String>,
    fields: BTreeMap<&'static str, u32>,
    notes: BTreeSet<String>,
    floods: u32,
    dependent: u32,
    matched: u32,
    unmatched: u32,
    reruns: u32,
    incomplete: u32,
    props: (u64, u64),
    deals: (u64, u64),
    connections: (u64, u64),
    items: (u64, u64),
    swallowed: Swallowed,
    started: bool,
}

/// Since the last month line.
#[derive(Default)]
struct MonthAcc {
    deliveries: u32,
    steps: u32,
    skipped: u32,
    not_enabled: u32,
    cycles: u32,
    clean_cycles: u32,
    cycle_lines: u32,
    clean_lines: u32,
    suppressed: u32,
    incomplete: u32,
    mismatches: u64,
    not_exact: u64,
    reruns: u32,
    uncached: u32,
    reasons: BTreeMap<String, u32>,
}

struct Shadow {
    host: Rc<ShadowHost>,
    max_cells: usize,
    max_lines: u32,
    pending: Option<Pending>,
    mirror: Mirror,
    book: BookHistory,
    cycle: CycleAcc,
    month: MonthAcc,
    stats: fc4_power_shadow_stats,
}

fn date(julian: i32) -> String {
    let d = civil_from_julian(julian);
    format!("{:04}-{:02}-{:02}", d.year, d.month, d.day)
}

/// The powered grid value of a cell (§2).
fn grid_formula(wire: &Bitmap, powered: &Bitmap, x: i32, z: i32) -> u8 {
    if !wire.test(x, z) {
        0
    } else if powered.test(x, z) {
        2
    } else {
        1
    }
}

/// The bounding box of the cells whose bits differ (None: equal).
fn changed_rect(a: &Bitmap, b: &Bitmap, (cx, cz): (i32, i32)) -> Option<CellRect> {
    if a.words() == b.words() {
        return None;
    }
    let mut r: Option<CellRect> = None;
    for z in 0..cz {
        for x in 0..cx {
            if a.test(x, z) != b.test(x, z) {
                r = Some(match r {
                    None => CellRect::new(x, z, x, z),
                    Some(r) => CellRect::new(r.x1.min(x), r.z1.min(z), r.x2.max(x), r.z2.max(z)),
                });
            }
        }
    }
    r
}

fn step_name(msg_type: u32, before: &PowerState) -> String {
    if msg_type == msg::SIM_NEW_MONTH {
        return "month".to_string();
    }
    match before.state {
        state::IDLE if before.recompute => "4>0".to_string(),
        state::IDLE => "4".to_string(),
        s => s.to_string(),
    }
}

/// The saved fields that differ: (name, original, ours). The consumption
/// map and the bitmaps are compared cell by cell elsewhere.
fn field_diffs(o: &PowerState, b: &PowerState) -> Vec<(&'static str, String, String)> {
    let mut v: Vec<(&'static str, String, String)> = Vec::new();
    macro_rules! f {
        ($($name:literal => $e:ident),* $(,)?) => {$(
            if o.$e != b.$e {
                v.push(($name, format!("{:?}", b.$e), format!("{:?}", o.$e)));
            }
        )*};
    }
    f!(
        "ringIndex" => ring_index, "production" => production, "consumption" => consumption,
        "consumers" => consumers, "powered" => powered, "unpowered" => unpowered, "requirement" => requirement,
        "costBase" => cost_base, "lastProduction" => last_production, "lastConsumption" => last_consumption,
        "lastImport" => last_import, "lastExport" => last_export, "lastDemand" => last_demand,
        "ring" => ring, "annual" => annual, "plantCount" => plant_count, "perType" => per_type,
        "plants" => plants, "poleCount" => pole_count, "brokenCount" => broken_count, "poles" => poles,
        "unusedFlag" => unused_flag, "strikeStart" => strike_start, "recompute" => recompute,
        "agePlants" => age_plants, "state" => state, "department" => department,
    );
    if o.deficit.to_bits() != b.deficit.to_bits() {
        v.push(("deficit", format!("{:?}", b.deficit), format!("{:?}", o.deficit)));
    }
    v
}

/// The scalars every cycle and month line shows, original / ours.
fn scalars(o: &PowerState, b: &PowerState, counts: Option<[u32; 3]>) -> String {
    let c = |i: usize| counts.map_or("?".to_string(), |c| c[i].to_string());
    format!(
        "production={}/{} consumption={}/{} import={}/{} export={}/{} demand={}/{} requirement={}/{} \
         consumers={}/{} powered={}/{} unpowered={}/{} plants={}/{} aged={}/{} distressed={}/{} \
         overworked={}/{} deficit={}/{} poles={}/{} broken={}/{}",
        o.last_production,
        b.last_production,
        o.last_consumption,
        b.last_consumption,
        o.last_import,
        b.last_import,
        o.last_export,
        b.last_export,
        o.last_demand,
        b.last_demand,
        o.requirement,
        b.requirement,
        o.consumers,
        b.consumers,
        o.powered,
        b.powered,
        o.unpowered,
        b.unpowered,
        o.plant_count,
        b.plant_count,
        c(1),
        b.aged,
        c(2),
        b.distressed,
        c(0),
        b.overworked,
        o.deficit,
        b.deficit,
        o.pole_count,
        b.pole_count,
        o.broken_count,
        b.broken_count
    )
}

impl Shadow {
    /// Another city without a PostCityInit seen: nothing unsaved is known.
    fn reset_city(&mut self) {
        self.mirror = Mirror::default();
        self.book = BookHistory::default();
        self.cycle = CycleAcc::default();
    }

    /// Runs ours on the record captured before the update.
    fn run(&self, p: &Pending, mode: Mode, rerun: Option<&Rerun>) -> Result<Run, String> {
        let h = &self.host;
        h.begin_run(mode);
        let sim = PowerSim::new(h.clone() as Rc<dyn PowerHost>, 0);
        // Shadows replay the original's pacing (simulator.md §5.7).
        sim.set_pacing(fc4_core::pacing::Pacing::Original);
        sim.read_tuning();
        let mut r = CaptureReader::new(&p.before.bytes, &p.before.refs);
        if !sim.read(&mut r) {
            return Err("our Read of the original's record failed".to_string());
        }
        let grid_token = r.grid_tokens.first().copied().unwrap_or(0);
        let m = &self.mirror;
        let factor = m.factor.unwrap_or_else(|| h.cumulative_factor(ORDINANCE_FACTOR_ID, 0));
        let pass = m.pass.unwrap_or([0; 3]);
        let counts = p.counts.unwrap_or([0; 3]);
        sim.update_state(|s| {
            s.initialized = true;
            s.enabled = true;
            s.factor = factor;
            s.broken_fraction = m.broken_fraction;
            s.import = pass[0];
            s.export = pass[1];
            s.demand = pass[2];
            s.overworked = counts[0];
            s.aged = counts[1];
            s.distressed = counts[2];
        });
        let before = sim.state().clone();
        // The original's dirty rectangle (§5.3): merged from its messages,
        // or, while unknown, the changed bits' box of a rerun.
        let dirty = match (rerun.and_then(|rr| rr.wire_rect), p.dirty) {
            (Some(r), _) => Some(r),
            (None, Dirty::Known(d)) => d,
            (None, Dirty::Unknown) => None,
        };
        if let Some(rect) = dirty {
            sim.update_state(|s| {
                s.wire_dirty = true;
                s.dirty_rect = rect;
            });
        }
        sim.set_block_probe(true, rerun.and_then(|rr| rr.target.clone()));
        sim.do_message(&PowerMessage {
            msg_type: p.msg_type,
            data1: p.julian as u32,
            ..PowerMessage::default()
        });
        let draws = sim.take_block_draws();
        let grid = h.grids.borrow().get(&grid_token).map(|g| (grid_token, g.values()));
        let overlay = h.overlay.borrow().clone();
        let state = sim.state().clone();
        Ok(Run {
            before,
            state,
            grid,
            overlay,
            draws,
        })
    }

    /// The month's bookkeeping from scratch: a new simulator, `Init`, §5.1
    /// for every live occupant of the map, then a wire rebuild of the whole
    /// map (§5.3).
    fn bookkeeping(&self) -> Result<Bookkeeping, String> {
        let h = &self.host;
        h.begin_run(Mode::Live);
        let (cx, cz) = h.cell_counts();
        if cx <= 0 || cz <= 0 {
            return Err("the city reports no cells".to_string());
        }
        let sim = PowerSim::new(h.clone() as Rc<dyn PowerHost>, 0);
        // Shadows replay the original's pacing (simulator.md §5.7).
        sim.set_pacing(fc4_core::pacing::Pacing::Original);
        sim.init();
        let all = CellRect::new(0, 0, cx - 1, cz - 1);
        let mut seen = BTreeSet::new();
        let occupants: Vec<u64> = h
            .occupants_in_cells(all)
            .into_iter()
            .filter(|&o| o != 0 && seen.insert(o))
            .collect();
        for &o in &occupants {
            sim.occupant_inserted(o);
        }
        sim.update_state(|s| {
            s.enabled = true;
            s.state = state::IDLE;
            s.recompute = false;
            s.wire_dirty = true;
            s.dirty_rect = all;
        });
        sim.do_idle();
        let state = sim.state().clone();
        Ok(Bookkeeping {
            occupants: occupants.len(),
            state,
        })
    }

    fn begin(
        &mut self,
        msg_type: u32,
        julian: i32,
        before: Result<Capture, &'static str>,
        counts: Option<[u32; 3]>,
    ) -> i32 {
        self.pending = None;
        // A date going back, or a jump of more than two months: another city
        // (the plugin also starts a new shadow at each PostCityInit).
        if let Some(last) = self.mirror.last_julian {
            if julian < last || julian > last.saturating_add(62) {
                self.reset_city();
            }
        }
        self.mirror.last_julian = Some(julian);
        self.mirror.first_julian.get_or_insert(julian);
        let mut p = Pending {
            msg_type,
            julian,
            before: Capture::default(),
            counts,
            pass_known: self.mirror.pass.is_some(),
            dirty: Dirty::Known(None),
            first: Err(String::new()),
            book: None,
        };
        if msg_type != msg::SIM_NEW_MONTH {
            // An enabled DoIdle consumes the rectangle; the messages during
            // the delivery merge into the next one.
            p.dirty = std::mem::replace(&mut self.mirror.dirty, Dirty::Known(None));
        }
        match before {
            Ok(c) => p.before = c,
            Err(e) => {
                p.first = Err(format!("before: {e}"));
                self.pending = Some(p);
                return -1;
            }
        }
        if msg_type == msg::SIM_NEW_MONTH {
            p.book = Some(self.bookkeeping());
        }
        p.first = self.run(&p, Mode::Live, None);
        if self.mirror.factor.is_none() {
            if let Ok(r) = &p.first {
                self.mirror.factor = Some(r.before.factor);
            }
        }
        let r = if p.first.is_ok() { 0 } else { -1 };
        self.pending = Some(p);
        r
    }

    /// The record captured after the update, loaded without a grid, and the
    /// grid token it refers to.
    fn parse_after(&self, after: &Capture) -> Result<(PowerState, u64), String> {
        let h = &self.host;
        h.mode.set(Mode::Parse);
        let sim = PowerSim::new(h.clone() as Rc<dyn PowerHost>, 0);
        // Shadows replay the original's pacing (simulator.md §5.7).
        sim.set_pacing(fc4_core::pacing::Pacing::Original);
        let mut r = CaptureReader::new(&after.bytes, &after.refs);
        let ok = sim.read(&mut r);
        h.mode.set(Mode::Live);
        if !ok {
            return Err("after: our Read of the original's record failed".to_string());
        }
        let token = r.grid_tokens.first().copied().unwrap_or(0);
        let s = sim.state().clone();
        Ok((s, token))
    }

    fn finish(
        &mut self,
        after: Result<Capture, &'static str>,
        counts: Option<[u32; 3]>,
        lines: &mut Vec<String>,
    ) -> i32 {
        let Some(p) = self.pending.take() else {
            return -1;
        };
        self.stats.deliveries = self.stats.deliveries.wrapping_add(1);
        self.month.deliveries += 1;
        let month = p.msg_type == msg::SIM_NEW_MONTH;
        let r = match self.compare(&p, after, counts, lines) {
            Ok(Some(n)) => n.min(i32::MAX as u64) as i32,
            Ok(None) => -1,
            Err(e) => {
                self.stats.incomplete = self.stats.incomplete.wrapping_add(1);
                self.month.incomplete += 1;
                self.cycle.incomplete += 1;
                self.mirror.known = false;
                self.mirror.pass = None;
                if !self.mirror.enabled {
                    // The original may not have consumed its rectangle.
                    self.mirror.dirty = p.dirty.then(self.mirror.dirty);
                }
                *self.month.reasons.entry(e.clone()).or_insert(0) += 1;
                if self.month.incomplete <= 3 {
                    lines.push(format!(
                        "power shadow {} {}: INCOMPLETE ({e}); not compared",
                        date(p.julian),
                        if month { "month" } else { "step" }
                    ));
                }
                -1
            }
        };
        self.stats.last_mismatches = r;
        if month {
            if let Some(b) = &p.book {
                lines.push(self.bookkeeping_line(&p, b));
            }
            lines.push(self.month_line(p.julian));
            self.month = MonthAcc::default();
        }
        r
    }

    /// Ok(Some(n)): compared with n differences; Ok(None): not compared
    /// (the original not enabled yet); Err: incomplete.
    fn compare(
        &mut self,
        p: &Pending,
        after: Result<Capture, &'static str>,
        counts_after: Option<[u32; 3]>,
        lines: &mut Vec<String>,
    ) -> Result<Option<u64>, String> {
        let first = p.first.as_ref().map_err(|e| e.clone())?;
        let after = after.map_err(|e| format!("after: {e}"))?;
        let (orig, after_grid) = self.parse_after(&after)?;
        let before = &first.before;
        let month = p.msg_type == msg::SIM_NEW_MONTH;
        let cells = self.host.cell_counts();
        let step = step_name(p.msg_type, before);

        if !month {
            let orig_acted = orig.state != before.state
                || orig.recompute != before.recompute
                || orig.wire.words() != before.wire.words();
            let ours_acted = first.state.state != before.state || first.state.recompute != before.recompute;
            if orig_acted {
                self.mirror.enabled = true;
            } else if ours_acted && !self.mirror.enabled {
                self.stats.not_enabled = self.stats.not_enabled.wrapping_add(1);
                self.month.not_enabled += 1;
                self.mirror.known = true;
                self.mirror.idle = orig.state == state::IDLE && !orig.recompute;
                // Not enabled: the original kept its dirty rectangle.
                self.mirror.dirty = p.dirty.then(self.mirror.dirty);
                return Ok(None);
            }
        }

        // Runs again with what only the update shows.
        let mut rerun = Rerun::default();
        if !month {
            if p.dirty == Dirty::Unknown {
                // §5.3: the rectangle is not saved and the messages that
                // made it came before the shadow listened.
                rerun.wire_rect = changed_rect(&before.wire, &orig.wire, cells);
            }
            if first.draws.iter().any(|d| d.order_dependent) {
                rerun.target = match before.state {
                    state::PLANTS => Some(orig.powered_next.clone()),
                    state::NEIGHBOURS => Some(orig.powered_current.clone()),
                    _ => None,
                };
            }
        }
        // Notes for the step's line (the cycle restarts at a 4>0 step).
        let mut notes = Vec::new();
        if let Dirty::Known(Some(r)) = p.dirty {
            notes.push(format!(
                "wire rebuilt over the dirty rectangle ({},{})-({},{}) merged from the original's messages",
                r.x1, r.z1, r.x2, r.z2
            ));
        }
        let again;
        let ours: &Run = if rerun.wire_rect.is_some() || rerun.target.is_some() {
            let u0 = self.host.uncached.get();
            again = self.run(p, Mode::Replay, Some(&rerun))?;
            self.host.mode.set(Mode::Live);
            let uncached = self.host.uncached.get().wrapping_sub(u0);
            self.stats.reruns = self.stats.reruns.wrapping_add(1);
            self.stats.uncached_reads = self.stats.uncached_reads.wrapping_add(uncached);
            self.month.reruns += 1;
            self.month.uncached += uncached;
            if let Some(r) = rerun.wire_rect {
                notes.push(format!(
                    "wire rebuilt by the original while its dirty rectangle is not known (the shadow started \
                     mid-city): rerun with the changed bits' box ({},{})-({},{}), which can differ through zones \
                     (power.md §5.3)",
                    r.x1, r.z1, r.x2, r.z2
                ));
            }
            if uncached > 0 {
                notes.push(format!("{uncached} inputs of a rerun read after the original's update"));
            }
            &again
        } else {
            first
        };
        let reran = !std::ptr::eq(ours, first);

        let mut d = Diff::default();
        self.diff(p, ours, &orig, after_grid, counts_after, &mut d);
        d.parts.extend(notes);
        // §8 (PW7): states 1 and 2 divide by the factor; mid-city the
        // original's is not known until its first month start.
        if !month
            && !self.mirror.factor_known
            && matches!(before.state, state::PLANTS | state::NEIGHBOURS)
            && d.mismatches > 0
        {
            d.not_exact += d.mismatches;
            d.mismatches = 0;
            d.fields.clear();
            d.parts.push(
                "NOT EXACT: the ordinance factor is not known until the first month (the shadow started mid-city; \
                 power.md §8)"
                    .to_string(),
            );
        }

        // Mirror.
        self.mirror.known = true;
        self.mirror.idle = orig.state == state::IDLE && !orig.recompute;
        let s = &ours.state;
        if month {
            self.mirror.factor = Some(s.factor);
            self.mirror.factor_known = true;
            self.mirror.broken_fraction = s.broken_fraction;
            self.mirror.months += 1;
        } else if before.state == state::RESET || self.mirror.pass.is_some() {
            self.mirror.pass = Some([s.import, s.export, s.demand]);
        }

        // Totals.
        let n = d.mismatches;
        self.stats.mismatches = self.stats.mismatches.wrapping_add(n);
        self.stats.not_exact = self.stats.not_exact.wrapping_add(d.not_exact);
        self.month.mismatches += n;
        self.month.not_exact += d.not_exact;
        if n == 0 {
            self.stats.clean = self.stats.clean.wrapping_add(1);
        }
        if month {
            self.stats.months = self.stats.months.wrapping_add(1);
            let mut line = format!(
                "power shadow {} month: {} cells compared; {}",
                date(p.julian),
                d.grid_cells,
                scalars(&orig, s, counts_after)
            );
            if !d.parts.is_empty() {
                line.push_str(&format!("; {}", d.parts.join("; ")));
            }
            if self.mirror.months == 1 {
                let start = self.mirror.first_julian.map_or("?".to_string(), date);
                line.push_str(&if self.mirror.fraction_known {
                    format!("; brokenFraction from 0 at the city start, as the original's (power.md §7.6; first step {start})")
                } else {
                    format!(
                        "; brokenFraction not known (the shadow started mid-city, {start}): brokenCount not exact \
                         (power.md §7.6)"
                    )
                });
            }
            line.push_str(&format!("; not_exact={}; mismatches={n}", d.not_exact));
            lines.push(line);
            return Ok(Some(n));
        }

        self.stats.steps = self.stats.steps.wrapping_add(1);
        self.month.steps += 1;
        if step == "4>0" {
            self.cycle = CycleAcc {
                started: true,
                ..CycleAcc::default()
            };
        }
        if reran {
            self.cycle.reruns += 1;
        }
        self.add_to_cycle(&step, &d, ours);
        if before.state == state::STATISTICS {
            self.cycle_line(p, &orig, s, counts_after, lines);
        }
        Ok(Some(n))
    }

    /// Compares one step (or month) of ours with the original's update.
    fn diff(
        &self,
        p: &Pending,
        ours: &Run,
        orig: &PowerState,
        after_grid: u64,
        counts_after: Option<[u32; 3]>,
        d: &mut Diff,
    ) {
        let before = &ours.before;
        let s = &ours.state;
        let (cx, cz) = self.host.cell_counts();
        let unmatched: Vec<&Bitmap> = ours.draws.iter().filter_map(|d| d.unmatched_mark.as_ref()).collect();
        let in_unmatched = |x: i32, z: i32| unmatched.iter().any(|m| m.test(x, z));
        let pass_unknown = !p.pass_known && before.state == state::STATISTICS;
        // §7.6 (PW4): the month's pole decay starts from brokenFraction.
        let fraction_unknown = p.msg_type == msg::SIM_NEW_MONTH && !self.mirror.fraction_known;
        let max = self.max_cells;

        // Saved fields.
        for (name, o, b) in field_diffs(s, orig) {
            let not_exact = (pass_unknown && matches!(name, "lastImport" | "lastExport" | "lastDemand"))
                || (fraction_unknown && name == "brokenCount")
                || (!unmatched.is_empty() && matches!(name, "powered" | "unpowered"));
            if not_exact {
                d.not_exact += 1;
                d.parts.push(format!("{name} {o}/{b} not exact"));
            } else {
                d.mismatches += 1;
                d.fields.push((name, o, b));
            }
        }
        if !d.fields.is_empty() {
            d.parts.push(format!(
                "fields orig/ours: {}",
                d.fields
                    .iter()
                    .map(|(n, a, b)| format!("{n}={a}/{b}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        // Unsaved plant counts (slots 27-29).
        if let Some(c) = counts_after {
            for (i, (name, v)) in [
                ("overworked", s.overworked),
                ("aged", s.aged),
                ("distressed", s.distressed),
            ]
            .into_iter()
            .enumerate()
            {
                if c[i] != v {
                    d.mismatches += 1;
                    d.fields.push((name, c[i].to_string(), v.to_string()));
                    d.parts.push(format!("{name} {}/{v}", c[i]));
                }
            }
        }
        // The consumption map (carried: the cycle does not change it).
        let mut cons = 0u64;
        let mut cons_first = Vec::new();
        for x in 0..cx {
            for z in 0..cz {
                let (o, b) = (orig.consumption_at(x, z), s.consumption_at(x, z));
                if o != b {
                    cons += 1;
                    if cons_first.len() < max {
                        cons_first.push(format!("({x},{z}) {o}/{b}"));
                    }
                }
            }
        }
        if cons > 0 {
            d.mismatches += cons;
            d.parts
                .push(format!("consumption map {cons} cells [{}]", cons_first.join(", ")));
        }
        // The bitmaps.
        for (name, a, b, order) in [
            ("mark", &orig.mark, &s.mark, false),
            ("wire", &orig.wire, &s.wire, false),
            ("current", &orig.powered_current, &s.powered_current, true),
            ("next", &orig.powered_next, &s.powered_next, true),
        ] {
            if a.words() == b.words() {
                continue;
            }
            let (mut n, mut ne) = (0u64, 0u64);
            let mut firsts = Vec::new();
            for x in 0..cx {
                for z in 0..cz {
                    let (o, u) = (a.test(x, z), b.test(x, z));
                    if o == u {
                        continue;
                    }
                    if order && in_unmatched(x, z) {
                        ne += 1;
                        continue;
                    }
                    n += 1;
                    if firsts.len() < max {
                        firsts.push(format!("({x},{z}) {}/{}", u8::from(o), u8::from(u)));
                    }
                    if d.first.len() < max {
                        d.first.push(format!("{name}({x},{z}) {}/{}", u8::from(o), u8::from(u)));
                    }
                }
            }
            if n == 0 && ne == 0 && a.words().len() != b.words().len() {
                n = 1;
                firsts.push(format!("words {}/{}", a.words().len(), b.words().len()));
            }
            d.mismatches += n;
            d.not_exact += ne;
            if n > 0 {
                d.parts
                    .push(format!("{name} {n} cells orig/ours [{}]", firsts.join(", ")));
            }
            if ne > 0 {
                d.parts.push(format!("{name} {ne} cells not exact (block order)"));
            }
        }
        // The powered grid, cell by cell.
        match &ours.grid {
            Some((token, values)) if *token == after_grid && after_grid != 0 => {
                let base = self.host.base.borrow().get(token).cloned();
                let geom = base.as_ref().map(|b| b.geom).unwrap_or_default();
                let (nx, nz) = (geom.count_x.clamp(0, 4096), geom.count_z.clamp(0, 4096));
                let mut n = 0u64;
                let mut firsts = Vec::new();
                for x in 0..nx {
                    for z in 0..nz {
                        let i = (x * nz + z) as usize;
                        let o = values.get(i).copied().unwrap_or(0);
                        let l = self.host.live_grid_value(*token, x, z);
                        if o == l {
                            continue;
                        }
                        if in_unmatched(x, z) {
                            d.not_exact += 1;
                            continue;
                        }
                        let b = base.as_ref().and_then(|b| b.values.get(i).copied()).unwrap_or(0);
                        // Rewritten by the original's §5.3 rebuild in a dirty
                        // rectangle that is not saved: the cell was stale
                        // (wired but never rewritten, §6.5 quirk), we left it
                        // and the original wrote the value of its bits.
                        if o == b
                            && l == grid_formula(&orig.wire, &orig.powered_current, x, z)
                            && b != grid_formula(&before.wire, &before.powered_current, x, z)
                        {
                            d.rect_rewrites += 1;
                            continue;
                        }
                        n += 1;
                        d.max_diff = d.max_diff.max((i32::from(o) - i32::from(l)).abs());
                        if firsts.len() < max {
                            firsts.push(format!("({x},{z}) {l}/{o}"));
                        }
                        if d.first.len() < max {
                            d.first.push(format!("grid({x},{z}) {l}/{o}"));
                        }
                    }
                }
                d.grid_cells += (nx as u64) * (nz as u64);
                d.mismatches += n;
                if n > 0 {
                    d.parts.push(format!(
                        "grid {n} cells max|d|={} orig/ours [{}]",
                        d.max_diff,
                        firsts.join(", ")
                    ));
                }
                if d.rect_rewrites > 0 {
                    d.parts.push(format!(
                        "grid {} stale cells rewritten by the original's wire rebuild (rectangle not saved)",
                        d.rect_rewrites
                    ));
                }
            }
            Some((token, _)) => {
                d.mismatches += 1;
                d.parts.push(format!("grid token orig/ours {after_grid:#x}/{token:#x}"));
            }
            None if after_grid != 0 => {
                d.mismatches += 1;
                d.parts.push("grid: ours has none".to_string());
            }
            None => {}
        }
        // Occupant properties we wrote, or read and left (the original may
        // have written them), against their live values.
        let o = &ours.overlay;
        let mut keys: Vec<(u64, u32)> = o.prop_order.clone();
        let mut read_keys: Vec<(u64, u32)> = self
            .host
            .cache
            .borrow()
            .keys()
            .filter_map(|k| match *k {
                Key::Prop(occ, id) if WRITTEN_PROPS.contains(&id) && !o.props.contains_key(&(occ, id)) => {
                    Some((occ, id))
                }
                _ => None,
            })
            .collect();
        read_keys.sort_unstable();
        keys.extend(read_keys);
        let mut bad = Vec::new();
        for k in keys {
            let ours_v = match o.props.get(&k) {
                Some(v) => v.clone(),
                None => match self.host.cache.borrow().get(&Key::Prop(k.0, k.1)) {
                    Some(Val::Prop(v)) => v.clone(),
                    _ => continue,
                },
            };
            let live = self.host.live.occupant_property(k.0, k.1);
            d.props.0 += 1;
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
        d.props.1 = bad.len() as u64;
        if !bad.is_empty() {
            d.mismatches += bad.len() as u64;
            let n = bad.len();
            bad.truncate(max.max(1));
            d.parts
                .push(format!("props {n}/{} orig/ours [{}]", d.props.0, bad.join(", ")));
        }
        // Deal usage.
        let mut deals: BTreeMap<u64, i32> = BTreeMap::new();
        for t in [deal::BUY_POWER, deal::SELL_POWER] {
            if let Some(Val::Deals(list)) = self.host.cache.borrow().get(&Key::Deals(t)) {
                for dl in list {
                    if let Some(Val::I32(u)) = self.host.cache.borrow().get(&Key::DealUsed(dl.token)) {
                        deals.insert(dl.token, *u);
                    }
                }
            }
        }
        for (&t, &u) in &o.deals {
            deals.insert(t, u);
        }
        let mut dbad = Vec::new();
        for (&t, &u) in &deals {
            let live = self.host.live.deal_amount_used(t);
            d.deals.0 += 1;
            if live != u {
                dbad.push(format!("deal {t:#x} used {live}/{u}"));
            }
        }
        d.deals.1 = dbad.len() as u64;
        if !dbad.is_empty() {
            d.mismatches += dbad.len() as u64;
            d.parts.push(format!("deals orig/ours [{}]", dbad.join(", ")));
        }
        // Connection flags we set.
        let mut cbad = Vec::new();
        if let Some(f) = self.host.connection_visited {
            for (&c, &v) in &o.visited {
                // SAFETY: valid callback on a live connection.
                let live = unsafe { f(self.host.live.0.user, c) };
                d.connections.0 += 1;
                if live != v {
                    cbad.push(format!("conn {c:#x} visited {}/{}", u8::from(live), u8::from(v)));
                }
            }
        }
        for (&c, &v) in &o.active {
            let live = self.host.live.connection_is_active(c);
            d.connections.0 += 1;
            if live != v {
                cbad.push(format!("conn {c:#x} active {}/{}", u8::from(live), u8::from(v)));
            }
        }
        d.connections.1 = cbad.len() as u64;
        if !cbad.is_empty() {
            d.mismatches += cbad.len() as u64;
            d.parts.push(format!("connections orig/ours [{}]", cbad.join(", ")));
        }
        // Budget item costs we changed (§7.3).
        let mut ibad = Vec::new();
        for (&(occ, index), &cost) in &o.item_cost {
            let live = self
                .host
                .live
                .budget_item_for_purpose(occ, fc4_core::power::budget::PURPOSE);
            d.items.0 += 1;
            match live {
                Some(i) if i.index == index && i.cost == cost => {}
                Some(i) => ibad.push(format!("occ {occ:#x} item cost {}/{cost}", i.cost)),
                None => ibad.push(format!("occ {occ:#x} item gone/{cost}")),
            }
        }
        d.items.1 = ibad.len() as u64;
        if !ibad.is_empty() {
            d.mismatches += ibad.len() as u64;
            d.parts.push(format!("budget items orig/ours [{}]", ibad.join(", ")));
        }
        if pass_unknown {
            d.parts.push(
                "pass import/export/demand not observed since the shadow started mid-city (no state 0 yet; power.md \
                 §9.1)"
                    .to_string(),
            );
        }
    }

    fn add_to_cycle(&mut self, step: &str, d: &Diff, ours: &Run) {
        let c = &mut self.cycle;
        c.steps.push(if d.mismatches > 0 {
            format!("{step}!")
        } else {
            step.to_string()
        });
        c.mismatches += d.mismatches;
        c.not_exact += d.not_exact;
        c.rect_rewrites += d.rect_rewrites;
        c.max_diff = c.max_diff.max(d.max_diff);
        c.grid_cells += d.grid_cells;
        for f in &d.first {
            if c.first.len() < self.max_cells {
                c.first.push(format!("{f} [{step}]"));
            }
        }
        for (name, _, _) in &d.fields {
            *c.fields.entry(name).or_insert(0) += 1;
        }
        for part in &d.parts {
            if c.notes.len() < 24 {
                c.notes.insert(format!("[{step}] {part}"));
            }
        }
        for dr in &ours.draws {
            c.floods += 1;
            if dr.order_dependent {
                c.dependent += 1;
                if dr.from_target {
                    c.matched += 1;
                } else if dr.unmatched_mark.is_some() {
                    c.unmatched += 1;
                }
            }
        }
        for (a, b) in [
            (&mut c.props, d.props),
            (&mut c.deals, d.deals),
            (&mut c.connections, d.connections),
            (&mut c.items, d.items),
        ] {
            a.0 += b.0;
            a.1 += b.1;
        }
        let w = ours.overlay.swallowed;
        let t = &mut c.swallowed;
        t.posts += w.posts;
        t.history += w.history;
        t.demolitions += w.demolitions;
        t.lot_writes += w.lot_writes;
        t.budget_writes += w.budget_writes;
        t.other += w.other;
    }

    fn cycle_line(
        &mut self,
        p: &Pending,
        orig: &PowerState,
        ours: &PowerState,
        counts: Option<[u32; 3]>,
        lines: &mut Vec<String>,
    ) {
        let c = std::mem::take(&mut self.cycle);
        self.stats.cycles = self.stats.cycles.wrapping_add(1);
        self.month.cycles += 1;
        let clean = c.mismatches == 0 && c.not_exact == 0;
        if c.mismatches == 0 {
            self.month.clean_cycles += 1;
        }
        let log = if clean {
            self.month.clean_lines < CLEAN_CYCLE_LINES
        } else {
            self.max_lines == u32::MAX || self.month.cycle_lines < self.max_lines
        };
        if !log {
            self.month.suppressed += 1;
            return;
        }
        if clean {
            self.month.clean_lines += 1;
        } else {
            self.month.cycle_lines += 1;
        }
        let (cx, cz) = self.host.cell_counts();
        let mut line = format!(
            "power shadow {} cycle: steps {}{} on {cx}x{cz} cells; grid {} cells compared, max|d|={}; first \
             differing cells: {}; {}; orders: floods={} order-dependent={} matched={} unmatched={}; props {}/{} \
             deals {}/{} connections {}/{} items {}/{} (compared/mismatched); not compared: posts={} history={} \
             budget={} lots={} demolitions={} other={}",
            date(p.julian),
            c.steps.join(" "),
            if c.started { "" } else { " (cycle start not observed)" },
            c.grid_cells,
            c.max_diff,
            if c.first.is_empty() {
                "none".to_string()
            } else {
                c.first.join(", ")
            },
            scalars(orig, ours, counts),
            c.floods,
            c.dependent,
            c.matched,
            c.unmatched,
            c.props.0,
            c.props.1,
            c.deals.0,
            c.deals.1,
            c.connections.0,
            c.connections.1,
            c.items.0,
            c.items.1,
            c.swallowed.posts,
            c.swallowed.history,
            c.swallowed.budget_writes,
            c.swallowed.lot_writes,
            c.swallowed.demolitions,
            c.swallowed.other
        );
        if c.rect_rewrites > 0 {
            line.push_str(&format!(
                "; {} stale grid cells rewritten by the original's wire rebuild (rectangle not saved)",
                c.rect_rewrites
            ));
        }
        if !c.notes.is_empty() {
            line.push_str(&format!("; {}", c.notes.iter().cloned().collect::<Vec<_>>().join("; ")));
        }
        if c.unmatched > 0 {
            line.push_str("; NOT EXACT: no start of the block iterator reproduces the original's powered blocks");
        }
        if c.incomplete > 0 {
            line.push_str(&format!("; {} steps not compared", c.incomplete));
        }
        line.push_str(&format!("; not_exact={}; mismatches={}", c.not_exact, c.mismatches));
        lines.push(line);
    }

    /// The month's bookkeeping line (§5.2, PW8): the original's running
    /// counts (the record before the month) against FreeCity4's insertion
    /// (§5.1) of every live occupant and a wire rebuild of the whole map.
    ///
    /// [choice, power.md §5.2] A difference equal to the baseline's is
    /// inherited from the save (case 1); one that changed since the last
    /// line is in flight (case 2: occupant messages posted and not yet
    /// delivered, foundations §5.1); only one that stays without being
    /// inherited is a mismatch. The spec takes the first line as the
    /// baseline; the occupant messages of that line's own day (and, under
    /// the turbo clock, of every day since the frame's message-server tick)
    /// can be in flight there too (runs pt-base-turbo-1 and -normal-1: the
    /// first line had consumers +3, the save's +2 and +1 in flight, and two
    /// cells in flight), so the baseline is the first line that equals the line
    /// before it [choice; QUESTIONS, power shadow baseline]: nothing changed
    /// between the two, so nothing was in flight at either (a message posted
    /// before the first is delivered at the next frame's message-server
    /// tick, flammability.md §9, which comes before the second unless one
    /// frame opens both month days). Until then nothing is counted. The
    /// baseline is never edited: a baseline entry whose difference is 0 at
    /// a line is absent (masked by a difference in flight that cancels it,
    /// run gsT1 2027-12-01: consumers +2 and two new buildings in flight,
    /// 660/660) and inherited again when it comes back; earlier versions
    /// dropped it there, and its return was then counted as persistent.
    /// A non-inherited difference unchanged since the last line counts as a
    /// mismatch when nothing else changed at this line, or when it is also
    /// unchanged since the line before (two buildings in flight at two
    /// lines can show the same count, never the same cells).
    ///
    /// The wire bits are listed apart and not counted, nor used for the
    /// baseline or the quiet test [choice; QUESTIONS, power shadow wire
    /// bits]. Ours are one rebuild of the whole map; the original's are what
    /// its insertions and its dirty-rectangle rebuilds left, and power.md §5.3
    /// makes those depend on the rectangles: a rebuild loses a bit whose only
    /// source is a zoned cell outside the rectangle (PW5), sets bits outside
    /// it that only a later rebuild clears, and wires what the insertion
    /// filters of §5.1 skip. Run pt-fix1-turbo-1 had wire 0/1 at
    /// (39..,37..38) for nine months after the rebuild over (0,37)-(44,111),
    /// run pt-fix1-turbo-2 two at (46,37..38) for eight months, with every
    /// cycle line exact; each rebuild's bits are compared exactly on the
    /// cycle lines.
    fn bookkeeping_line(&mut self, p: &Pending, b: &Result<Bookkeeping, String>) -> String {
        let head = format!("power shadow {} bookkeeping", date(p.julian));
        let b = match b {
            Ok(b) => b,
            Err(e) => return format!("{head}: INCOMPLETE ({e}); not compared"),
        };
        let o = match &p.first {
            Ok(r) => &r.before,
            Err(e) => return format!("{head}: INCOMPLETE ({e}); not compared"),
        };
        let s = &b.state;
        let max = self.max_cells;
        let (cx, cz) = self.host.cell_counts();
        let mut diff = BookDiff::new();
        let mut put = |k: BookKey, v: i64| {
            if v != 0 {
                diff.insert(k, v);
            }
        };
        let mut parts = Vec::new();
        for (k, name, a, c) in [
            (
                BookKey::Consumers,
                "consumers",
                i64::from(o.consumers),
                i64::from(s.consumers),
            ),
            (
                BookKey::Requirement,
                "requirement",
                i64::from(o.requirement),
                i64::from(s.requirement),
            ),
            (
                BookKey::PlantCount,
                "plantCount",
                i64::from(o.plant_count),
                i64::from(s.plant_count),
            ),
            (
                BookKey::PoleCount,
                "poleCount",
                i64::from(o.pole_count),
                i64::from(s.pole_count),
            ),
            (BookKey::CostBase, "costBase", o.cost_base, s.cost_base),
        ] {
            parts.push(format!("{name}={a}/{c}"));
            put(k, a.wrapping_sub(c));
        }
        for t in 0..PLANT_TYPES {
            put(BookKey::PerType(t), i64::from(o.per_type[t]) - i64::from(s.per_type[t]));
        }
        let op: BTreeSet<u64> = o.plants.iter().map(|p| p.occupant).collect();
        let sp: BTreeSet<u64> = s.plants.iter().map(|p| p.occupant).collect();
        let oq: BTreeSet<u64> = o.poles.iter().copied().collect();
        let sq: BTreeSet<u64> = s.poles.iter().copied().collect();
        for (key, a, c) in [
            (BookKey::Plant as fn(u64) -> BookKey, &op, &sp),
            (BookKey::Pole as fn(u64) -> BookKey, &oq, &sq),
        ] {
            for &t in a.difference(c) {
                put(key(t), 1);
            }
            for &t in c.difference(a) {
                put(key(t), -1);
            }
        }
        let (mut cells, mut maxd) = (0u64, 0i64);
        for x in 0..cx {
            for z in 0..cz {
                let v = i64::from(o.consumption_at(x, z)) - i64::from(s.consumption_at(x, z));
                if v != 0 {
                    cells += 1;
                    maxd = maxd.max(v.abs());
                    put(BookKey::Cell(x, z), v);
                }
            }
        }
        // The wire bits stay out of `diff`: listed, not counted (see above).
        let mut wire_bits: Vec<(BookKey, i64)> = Vec::new();
        if o.wire.words() != s.wire.words() {
            for x in 0..cx {
                for z in 0..cz {
                    let (a, c) = (o.wire.test(x, z), s.wire.test(x, z));
                    if a != c {
                        wire_bits.push((BookKey::Wire(x, z), if a { 1 } else { -1 }));
                    }
                }
            }
        }
        let wires = wire_bits.len();
        parts.push(format!(
            "consumption map {cells}/{} cells differ max|d|={maxd}; wire {wires} cells differ",
            i64::from(cx) * i64::from(cz)
        ));

        let list = |keys: &[(BookKey, i64)]| -> String {
            let mut v: Vec<String> = keys.iter().take(max).map(|&(k, v)| k.show(v)).collect();
            if keys.len() > max {
                v.push("...".to_string());
            }
            if v.is_empty() {
                String::new()
            } else {
                format!(" [{}]", v.join(", "))
            }
        };
        let since = if self.mirror.city_start {
            "since the city start"
        } else {
            "since the shadow started mid-city"
        };
        let wire_part = format!(
            "; wire bits, not counted (the original's running bitmap after its dirty-rectangle rebuilds vs one \
             rebuild of the whole map, power.md §5.3: PW5, unclipped bits, no insertion filters; each rebuild is \
             compared on the cycle lines) {wires}{}",
            list(&wire_bits)
        );
        let intro = format!(
            "{head}: the original's running counts (orig) vs FreeCity4's insertion (§5.1) of the {} live occupants \
             and a wire rebuild of the whole map (ours), orig/ours: {}",
            b.occupants,
            parts.join("; ")
        );
        let hist = &mut self.book;
        let get = |d: &BookDiff, k: &BookKey| d.get(k).copied().unwrap_or(0);
        // Nothing changed since the last line: no occupant message in flight
        // at this line that was not at the last one.
        let quiet = hist.lines > 0 && diff == hist.prev;
        hist.lines += 1;
        let Some(base) = hist.base.as_ref() else {
            let all: Vec<(BookKey, i64)> = diff.iter().map(|(&k, &v)| (k, v)).collect();
            let line = if quiet {
                hist.base = Some(diff.clone());
                hist.base_date = Some(p.julian);
                format!(
                    "{intro}; baseline (the first line equal to the one before it {since}, nothing in flight): {} \
                     differences inherited from the save (power.md §5.2 case 1){}{wire_part}; mismatches=0",
                    all.len(),
                    list(&all)
                )
            } else {
                let why = if hist.lines == 1 {
                    format!("the first line {since}")
                } else {
                    let keys: BTreeSet<&BookKey> = diff.keys().chain(hist.prev.keys()).collect();
                    let changed = keys.iter().filter(|&&k| get(&diff, k) != get(&hist.prev, k)).count();
                    format!("{changed} differences changed since the last line: in flight, power.md §5.2 case 2")
                };
                format!(
                    "{intro}; baseline not set yet ({why}; the baseline is the first line equal to the one before \
                     it, nothing counted until then): {} differences{}{wire_part}; mismatches=0",
                    all.len(),
                    list(&all)
                )
            };
            hist.prev2 = std::mem::replace(&mut hist.prev, diff);
            return line;
        };
        let keys: BTreeSet<BookKey> = diff
            .keys()
            .chain(hist.prev.keys())
            .chain(base.keys())
            .copied()
            .collect();
        let (mut inherited, mut flight, mut gone, mut absent, mut waiting, mut persistent) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for k in keys {
            let (v, b0) = (get(&diff, &k), get(base, &k));
            let (p1, p2) = (get(&hist.prev, &k), get(&hist.prev2, &k));
            match book_class(v, b0, p1, p2, quiet) {
                Some(BookClass::Inherited) => inherited.push((k, v)),
                Some(BookClass::InFlight) => flight.push((k, v)),
                Some(BookClass::Absent) => absent.push((k, b0)),
                Some(BookClass::Waiting) => waiting.push((k, v)),
                Some(BookClass::Persistent) => persistent.push((k, v)),
                None => {}
            }
            if p1 != 0 && p1 != b0 && v != p1 && (v == 0 || v == b0) {
                gone.push((k, p1));
            }
        }
        hist.prev2 = std::mem::replace(&mut hist.prev, diff);
        let base_date = hist.base_date;
        let n = persistent.len() as u64;
        self.stats.bookkeeping_mismatches = self.stats.bookkeeping_mismatches.wrapping_add(n);
        format!(
            "{intro}; inherited (as at the baseline line {}, power.md §5.2 case 1) {}{}; in flight (changed since the \
             last line, case 2) {}{}; gone since the last line {}{}; baseline entries absent (0 at this line: masked \
             by a difference in flight; kept) {}{}; unchanged while others changed (not inherited; counted at the \
             next line) {}{}; persistent (not inherited; unchanged since the last line with nothing in flight, or \
             at three lines) {}{}{wire_part}; mismatches={n}",
            base_date.map_or("?".to_string(), date),
            inherited.len(),
            list(&inherited),
            flight.len(),
            list(&flight),
            gone.len(),
            list(&gone),
            absent.len(),
            list(&absent),
            waiting.len(),
            list(&waiting),
            persistent.len(),
            list(&persistent),
        )
    }

    fn month_line(&self, julian: i32) -> String {
        let m = &self.month;
        let mut line = format!(
            "power shadow month to {}: deliveries={} steps={} idle passed through={} not enabled={} cycles={} \
             (clean {}, lines {}+{} clean, not logged {}) incomplete={} reruns={} uncached={} factor={}{} \
             pass totals {} dirty rectangle {}; not_exact={}; mismatches={}",
            date(julian),
            m.deliveries,
            m.steps,
            m.skipped,
            m.not_enabled,
            m.cycles,
            m.clean_cycles,
            m.cycle_lines,
            m.clean_lines,
            m.suppressed,
            m.incomplete,
            m.reruns,
            m.uncached,
            self.mirror.factor.unwrap_or(1.0),
            if self.mirror.factor_known { "" } else { " (not known)" },
            if self.mirror.pass.is_some() { "known" } else { "unknown" },
            if self.mirror.dirty == Dirty::Unknown {
                "unknown"
            } else {
                "known"
            },
            m.not_exact,
            m.mismatches
        );
        if !m.reasons.is_empty() {
            let r: Vec<String> = m.reasons.iter().map(|(k, v)| format!("{k} x{v}")).collect();
            line.push_str(&format!("; incomplete because: {}", r.join(", ")));
        }
        line
    }

    /// One of the original's messages (§5), as it reaches the original: the
    /// dirty rectangle it merges (PW5) and the tuning reload's factor read
    /// (PW7). Read through the live table (nothing remembered).
    fn message(&mut self, m: &PowerMessage) -> bool {
        match m.msg_type {
            msg::TUNING_RELOAD => {
                // The original re-reads its tuning and the factor now.
                *self.host.tuning.borrow_mut() = None;
                self.mirror.factor = Some(self.host.live.cumulative_factor(ORDINANCE_FACTOR_ID, 0));
                self.mirror.factor_known = true;
                true
            }
            msg::OCCUPANT_REMOVED | msg::ZONING_CHANGED => {
                let h = &self.host;
                let mode = h.mode.replace(Mode::Parse);
                let sim = PowerSim::new(h.clone() as Rc<dyn PowerHost>, 0);
                // Shadows replay the original's pacing (simulator.md §5.7).
                sim.set_pacing(fc4_core::pacing::Pacing::Original);
                sim.read_tuning();
                // The original handles a removal only once initialised, which
                // it is while the shadow compares it.
                sim.update_state(|s| s.initialized = true);
                let r = sim.dirty_rect_for(m);
                h.mode.set(mode);
                if let Some(r) = r {
                    self.mirror.dirty = self.mirror.dirty.merge(r);
                }
                true
            }
            _ => false,
        }
    }

    fn wants(&mut self, msg_type: u32, event: bool) -> bool {
        match msg_type {
            // Once a game day: also catches a recompute requested other than
            // by a message.
            msg::SIM_NEW_MONTH | msg::SIM_NEW_DAY => true,
            msg::SIM_IDLE => {
                if event || !self.mirror.known || !self.mirror.idle {
                    true
                } else {
                    self.stats.idle_skipped = self.stats.idle_skipped.wrapping_add(1);
                    self.month.skipped += 1;
                    false
                }
            }
            _ => false,
        }
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

fn with_shadow<R>(s: *mut fc4_power_shadow, fallback: R, f: impl FnOnce(&mut Shadow) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_power_shadow*.
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
/// callbacks (city, tuning, ordinance factor, occupants and their
/// properties, poles, lines, network occupants, the occupant query, zones,
/// lots, deals and connections, departments, line items, budget items) and
/// the grid table's `geometry` / `get`; the write callbacks may be NULL.
/// `options` may be NULL (defaults: a shadow started mid-city). With
/// [`FC4_POWER_SHADOW_CITY_START`] the ordinance factor is read now.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_shadow_create(
    host: *const fc4_power_host,
    options: *const fc4_power_shadow_options,
) -> *mut fc4_power_shadow {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h)
            if (h.struct_size as usize) >= std::mem::size_of::<fc4_power_host>()
                && h.grids.complete()
                && h.grids.get.is_some()
                && h.grids.geometry.is_some() =>
        {
            let o = options
                .as_ref()
                .filter(|o| (o.struct_size as usize) >= std::mem::size_of::<fc4_power_shadow_options>());
            let (traced_host, traced_options) = crate::trace::power_create(*h, o.copied());
            let h = &traced_host;
            let o = traced_options.as_ref();
            let host = Rc::new(ShadowHost {
                live: CHost(*h),
                connection_visited: o.and_then(|o| o.connection_visited),
                mode: Cell::new(Mode::Live),
                cache: RefCell::new(HashMap::new()),
                cells: Cell::new(None),
                widths: Cell::new(None),
                tuning: RefCell::new(None),
                overlay: RefCell::new(Overlay::default()),
                base: RefCell::new(HashMap::new()),
                grids: RefCell::new(HashMap::new()),
                uncached: Cell::new(0),
            });
            let max_cells = match o.map_or(0, |o| o.max_cells) {
                0 => 8,
                n => n.min(64) as usize,
            };
            let max_lines = match o.map_or(0, |o| o.max_lines) {
                0 => 32,
                n => n,
            };
            let mirror = if o.map_or(0, |o| o.flags) & FC4_POWER_SHADOW_CITY_START != 0 {
                Mirror::at_city_start(host.live.cumulative_factor(ORDINANCE_FACTOR_ID, 0))
            } else {
                Mirror::default()
            };
            Box::into_raw(Box::new(Shadow {
                host,
                max_cells,
                max_lines,
                pending: None,
                mirror,
                book: BookHistory::default(),
                cycle: CycleAcc::default(),
                month: MonthAcc::default(),
                stats: fc4_power_shadow_stats {
                    struct_size: std::mem::size_of::<fc4_power_shadow_stats>() as u32,
                    last_mismatches: -1,
                    ..Default::default()
                },
            })) as *mut fc4_power_shadow
        }
        _ => std::ptr::null_mut(),
    })
}

/// Destroys a shadow (NULL is ignored).
#[no_mangle]
pub unsafe extern "C" fn fc4_power_shadow_destroy(s: *mut fc4_power_shadow) {
    if s.is_null() {
        return;
    }
    crate::trace::will_destroy("power");
    guard((), || drop(Box::from_raw(s as *mut Shadow)));
    crate::trace::destroy("power");
}

/// Whether the delivery of `msg_type` must be wrapped: every SimNewMonth and
/// SimNewDay; a SimIdle (one `DoIdle` step) unless the original was idle
/// after its last wrapped delivery (state 4 without a recompute request:
/// its `DoIdle` does nothing) and `event` is false (`event`: one of the
/// original's messages, §5, arrived since the last wrapped delivery; it may
/// have requested a recompute). A delivery passed through is counted.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_shadow_wants(s: *mut fc4_power_shadow, msg_type: u32, event: bool) -> bool {
    crate::trace::power_wants(msg_type, event);
    let r = with_shadow(s, false, |sh| sh.wants(msg_type, event));
    crate::trace::bool_result("power", "wants", r);
    r
}

/// Before the original's delivery of `msg_type` (SimNewMonth, SimNewDay or
/// SimIdle; `julian` = the message's data1): loads `before`, copies the
/// powered grid and runs ours; a month also rebuilds the bookkeeping.
/// `counts`: the original's slots 27-29 (may be NULL). 0 = ran; -1 = not
/// run (the finish reports why); -2 = another message type, nothing done.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_shadow_begin(
    s: *mut fc4_power_shadow,
    msg_type: u32,
    julian: i32,
    before: *const fc4_power_capture,
    counts: *const fc4_power_shadow_counts,
) -> i32 {
    if msg_type != msg::SIM_NEW_DAY && msg_type != msg::SIM_NEW_MONTH && msg_type != msg::SIM_IDLE {
        return -2;
    }
    crate::trace::power_step("begin", msg_type, julian, before, counts);
    let c = capture_of(before);
    let n = counts_of(counts);
    let r = with_shadow(s, -1, |sh| sh.begin(msg_type, julian, c, n));
    crate::trace::step_result("power", "begin", r, &[]);
    r
}

/// After the original's delivery: compares (see the module notes) and hands
/// the log lines to `sink`, one call per line: at the end of a recompute
/// cycle the cycle line (every cycle with a difference up to the month's
/// limit, and the first clean ones), after a SimNewMonth the month step,
/// bookkeeping and month lines, and INCOMPLETE lines (at most 3 a month).
/// Returns the delivery's difference count, -1 when it was not compared.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_shadow_finish(
    s: *mut fc4_power_shadow,
    after: *const fc4_power_capture,
    counts: *const fc4_power_shadow_counts,
    sink: fc4_bytes_sink,
    ctx: *mut c_void,
) -> i32 {
    crate::trace::power_step("finish", 0, 0, after, counts);
    let c = capture_of(after);
    let n = counts_of(counts);
    let mut lines = Vec::new();
    let r = with_shadow(s, -1, |sh| sh.finish(c, n, &mut lines));
    crate::trace::step_result("power", "finish", r, &lines);
    emit(&lines, sink, ctx);
    r
}

/// One of the original's messages (power.md §5) as it is delivered to the
/// original, outside or during a wrapped delivery (ABI 33): a removal
/// (`void1` = the occupant's `cISC4Occupant` token) or a zoning change
/// (`rect`) merges the rectangle the original's `DoMessage` merges into the
/// dirty rectangle its next enabled `DoIdle` rebuilds (§5.3, PW5); a tuning
/// reload makes the shadow read the tuning and the ordinance factor again
/// (§8, PW7). Reads go through the host table at once. True when the
/// message is one of these.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_shadow_message(s: *mut fc4_power_shadow, m: *const fc4_power_message) -> bool {
    let Some(m) = m.as_ref() else { return false };
    crate::trace::power_message(m);
    let m = PowerMessage::from(m);
    let r = with_shadow(s, false, |sh| sh.message(&m));
    crate::trace::bool_result("power", "message", r);
    r
}

/// Totals since creation.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_shadow_get_stats(
    s: *mut fc4_power_shadow,
    out: *mut fc4_power_shadow_stats,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_shadow(s, false, |sh| {
        *o = sh.stats;
        true
    })
}

// Not used by the C ABI; keeps the plant-type count in sight of the field
// comparison (perType has PLANT_TYPES entries).
const _: () = assert!(PLANT_TYPES == 10);
const _: u32 = GRID_CLSID;
