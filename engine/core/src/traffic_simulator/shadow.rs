// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The traffic shadow (tooling, no spec behaviour): a same-process
//! differential of the traffic simulator. FreeCity4's core runs as a private
//! copy next to the ORIGINAL simulator, on the original's live inputs, read
//! only, and is compared bitwise after every delivery the original gets from
//! the simulator's agents (day, month, one-shot).
//!
//! # The method
//!
//! The original's work per day depends on wall-clock time (§3.3: a 4 ms
//! deadline per delivery), so the copy cannot run before the original: it
//! cannot know where the original will stop. It runs **after** it, on the
//! inputs as they stood **before**:
//!
//! 1. [`TrafficShadow::before`], just before the original's delivery: the
//!    original's record `R0` (its own `Write`, captured by the host) is
//!    compared with the copy's; when they differ (something between two
//!    deliveries changed one side only) the copy reads `R0` and resumes as a
//!    load does (a **resync**). The inputs the original's delivery may change
//!    and the copy reads are snapshotted: every source's trip list, the trip
//!    capacity array of every lot and connection holder and the connection
//!    arrays (the **pre-image**); the original's grids are copied into
//!    private grids when the copy's differ.
//! 2. The original's delivery, unchanged.
//! 3. [`TrafficShadow::after`]: the original's record `R1` gives where it
//!    stopped (the cycle state and the phase's progress, [`progress`]). The
//!    copy steps one unit at a time ([`State::shadow_lazy`], one unit per
//!    slice) to the same point, takes the original's day step when the
//!    original ended its day on its share (§3.3), and runs on to the next
//!    trip to examine where the original's deadline test would have stopped
//!    it (states 17, 18). Its reads see the pre-image; its writes stay in an
//!    overlay ([`ShadowHost`]: nothing reaches the game). Then the copy's
//!    record is compared byte for byte with `R1` (the differing fields are
//!    named by the marks of `write_record`), every trip list and holder
//!    property with the live ones, and the copy's grids with the original's.
//!
//! The network model is the copy's own: built once by `DoPostCityInit` on
//! the live city when the shadow starts, then kept current by the messages
//! the original receives ([`TrafficShadow::message`]).
//!
//! # Not compared
//!
//! - The draws (§4.15): the original's generator is not readable, so the
//!   values of property `0x4A611773` (never read, §4.1.1) are not compared;
//!   the pothole moves (draw 6) and the accident check (draw 3) can differ
//!   in which cells they pick.
//! - What the original sends outside its record and the holders: posted
//!   messages, history values, line-item incomes and network costs, budget
//!   items, accidents. The copy's are counted.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use crate::grid::{CellRect, CityDims, GridElem, GridProvider, ScanState, SimGrid, TractGeometry, TractGrid};
use crate::julian::civil_from_julian;
use crate::property::PropertyHolder;
use crate::stl::{StlHashMap, StlHashSet, STL_DEFAULT_HINT, STL_PRIMES};
use crate::stream::{ByteReader, InStream, OutStream, RefValue};
use crate::traffic_network::{NetworkHost, NetworkState, PathInfo};
use crate::variant::Variant;

use super::cycle_state as cs;
use super::record::{parse_record, write_trip};
use super::state::{State, PC_CAPACITY_SEND_BACKS, PC_SOURCE_COUNTER};
use super::{
    msg, prop, ref_iid, Message, NeighbourCity, SimMessage, Source, TrafficSimHost, TrafficSimulator, TripData,
};

/// The units a trip phase has done since its pacing reset (§3.4): state 16
/// the sources confirmed (pass counter 3), state 17 the searches
/// (`processed`) plus the capacity send-backs (pass counter 4), state 18
/// the searches (`processed`); 0 in any other state.
pub fn progress(st: &State) -> u64 {
    match st.cycle_state {
        cs::CONFIRM_TRIPS => u64::from(st.pass_counters[PC_SOURCE_COUNTER]),
        cs::REROUTE_TRIPS => u64::from(st.processed as u32) + u64::from(st.pass_counters[PC_CAPACITY_SEND_BACKS]),
        cs::FULL_TRIPS => u64::from(st.processed as u32),
        _ => 0,
    }
}

/// The class IDs of the grids the simulator refers to (simgrid §2).
pub mod grid_class {
    pub const BOOL: u32 = 0x49B9_E601;
    pub const U8: u32 = 0x49B9_E602;
    pub const U32: u32 = 0x49B9_E606;
    pub const F32: u32 = 0x49B9_E60A;
}

// ---- the captured record -------------------------------------------------------------------------

/// One reference of a captured record: the object's identities for the
/// three interfaces the record's readers ask (0 = it does not answer) and
/// its class ID.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CapturedRef {
    /// `cISC4SimGrid` (`0xC9BD939A`).
    pub grid: u64,
    /// `cISC4Occupant` (`0x26BB767E`): a switch building.
    pub occupant: u64,
    /// `cISC4DepartmentBudget` (`0x223E2B67`).
    pub department: u64,
    pub class_id: u32,
}

impl CapturedRef {
    /// The identity a reader asking `iid` gets (0: the object does not
    /// answer, the reference reads as null).
    pub fn token(&self, iid: u32) -> u64 {
        match iid {
            ref_iid::GRID => self.grid,
            ref_iid::BUILDING => self.occupant,
            ref_iid::DEPARTMENT => self.department,
            _ => [self.grid, self.occupant, self.department]
                .into_iter()
                .find(|&t| t != 0)
                .unwrap_or(0),
        }
    }
}

/// The original's record read through its own `Write` into memory:
/// primitives little-endian, a reference as a u32 object number (1-based
/// index into `refs`, 0 = null) followed, when not null, by the u32 class
/// ID; each object is listed once, at its first reference.
#[derive(Clone, Debug, Default)]
pub struct Capture {
    pub bytes: Vec<u8>,
    pub refs: Vec<CapturedRef>,
}

/// A [`Capture`] as an input stream: a reference reads as the identity of
/// the interface the reader asks.
pub struct CaptureReader<'a> {
    r: ByteReader<'a>,
    refs: &'a [CapturedRef],
}

impl<'a> CaptureReader<'a> {
    pub fn new(c: &'a Capture) -> CaptureReader<'a> {
        CaptureReader {
            r: ByteReader::new(&c.bytes),
            refs: &c.refs,
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
        let token = e.token(iid);
        if token == 0 {
            // An object without the interface reads as null (foundations
            // §2.7, the helper semantics).
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

/// A record written in the [`Capture`] format (references numbered by first
/// appearance), with the field marks of `write_record`: the copy's record as
/// the game's capture stream would hold it, so the two compare byte for
/// byte.
#[derive(Default)]
pub struct CanonicalWriter {
    pub bytes: Vec<u8>,
    /// (offset, field) at each mark, in order.
    pub marks: Vec<(usize, &'static str)>,
    ids: Vec<u64>,
    /// References that are not host objects (none expected: every grid of
    /// the copy is a [`ShadowGrid`]).
    pub foreign_refs: u32,
    good: bool,
}

impl CanonicalWriter {
    pub fn new() -> CanonicalWriter {
        CanonicalWriter {
            good: true,
            ..CanonicalWriter::default()
        }
    }

    fn emit(&mut self, b: &[u8]) -> bool {
        if self.good {
            self.bytes.extend_from_slice(b);
        }
        self.good
    }

    /// The field the byte at `offset` belongs to.
    pub fn field_at(&self, offset: usize) -> &'static str {
        let i = self.marks.partition_point(|(o, _)| *o <= offset);
        if i == 0 {
            "(before the first field)"
        } else {
            self.marks[i - 1].1
        }
    }

    /// The fields as (name, start, end) byte ranges.
    pub fn fields(&self) -> Vec<(&'static str, usize, usize)> {
        let mut v = Vec::with_capacity(self.marks.len());
        for (i, &(o, name)) in self.marks.iter().enumerate() {
            let end = self.marks.get(i + 1).map_or(self.bytes.len(), |m| m.0);
            v.push((name, o, end));
        }
        v
    }
}

impl OutStream for CanonicalWriter {
    fn put_u8(&mut self, v: u8) -> bool {
        self.emit(&[v])
    }
    fn put_u16(&mut self, v: u16) -> bool {
        self.emit(&v.to_le_bytes())
    }
    fn put_u32(&mut self, v: u32) -> bool {
        self.emit(&v.to_le_bytes())
    }
    fn put_u64(&mut self, v: u64) -> bool {
        self.emit(&v.to_le_bytes())
    }
    fn put_f32(&mut self, v: f32) -> bool {
        self.emit(&v.to_bits().to_le_bytes())
    }
    fn put_f64(&mut self, v: f64) -> bool {
        self.emit(&v.to_bits().to_le_bytes())
    }
    fn put_bytes(&mut self, v: &[u8]) -> bool {
        self.emit(v)
    }
    fn put_str(&mut self, v: &[u8]) -> bool {
        self.put_u32(v.len() as u32);
        self.emit(v)
    }
    fn put_ref(&mut self, r: &RefValue) -> bool {
        match r {
            RefValue::Null => self.put_u32(0),
            RefValue::Host { token, class_id } => {
                let i = match self.ids.iter().position(|t| t == token) {
                    Some(i) => i,
                    None => {
                        self.ids.push(*token);
                        self.ids.len() - 1
                    }
                };
                self.put_u32(i as u32 + 1);
                self.put_u32(*class_id)
            }
            RefValue::Object(o) => {
                self.foreign_refs += 1;
                self.put_u32(u32::MAX);
                self.put_u32(o.class_id())
            }
        }
    }
    fn error(&self) -> i32 {
        i32::from(!self.good)
    }
    fn set_error(&mut self) {
        self.good = false;
    }
    fn mark(&mut self, label: &'static str) {
        self.marks.push((self.bytes.len(), label));
    }
}

// ---- private grids ----------------------------------------------------------------------------------

/// A private copy of one of the original's grids: its values live here, its
/// saved reference is the original grid's (so the copy's record names the
/// original's grids, as the original's own record does).
pub struct ShadowGrid<T: GridElem> {
    pub data: Rc<SimGrid<T>>,
    pub token: u64,
    pub class_id: u32,
}

impl<T: GridElem> TractGrid<T> for ShadowGrid<T> {
    fn geometry(&self) -> TractGeometry {
        self.data.geometry()
    }
    fn get(&self, tx: i32, tz: i32) -> T {
        self.data.get(tx, tz)
    }
    fn set(&self, tx: i32, tz: i32, v: T) {
        self.data.set(tx, tz, v)
    }
    fn fill(&self, v: T) {
        self.data.fill(v)
    }
    fn save_ref(&self) -> RefValue {
        RefValue::Host {
            token: self.token,
            class_id: self.class_id,
        }
    }
}

#[derive(Clone)]
enum GridSlot {
    U8(Rc<ShadowGrid<u8>>),
    U32(Rc<ShadowGrid<u32>>),
    F32(Rc<ShadowGrid<f32>>),
}

impl GridSlot {
    fn geometry(&self) -> TractGeometry {
        match self {
            GridSlot::U8(g) => g.geometry(),
            GridSlot::U32(g) => g.geometry(),
            GridSlot::F32(g) => g.geometry(),
        }
    }
    /// The values as f64, x outer.
    fn values(&self) -> Vec<f64> {
        match self {
            GridSlot::U8(g) => g.data.values().into_iter().map(|v| v.to_f64()).collect(),
            GridSlot::U32(g) => g.data.values().into_iter().map(|v| v.to_f64()).collect(),
            GridSlot::F32(g) => g.data.values().into_iter().map(|v| v.to_f64()).collect(),
        }
    }
}

/// The original's grids as the host reads them (borrowed, never owned).
pub trait LiveGrids {
    /// The grid's geometry; `None` when it cannot be read.
    fn geometry(&self, token: u64) -> Option<TractGeometry>;
    /// `GetTractValue(tx, tz)` of a grid of class `class_id`, as a double.
    fn value(&self, token: u64, class_id: u32, tx: i32, tz: i32) -> f64;
}

fn live_values(live: &dyn LiveGrids, token: u64, class_id: u32, g: &TractGeometry) -> Vec<f64> {
    let (nx, nz) = (g.count_x.clamp(0, 4096), g.count_z.clamp(0, 4096));
    let mut v = Vec::with_capacity((nx * nz) as usize);
    for x in 0..nx {
        for z in 0..nz {
            v.push(live.value(token, class_id, x, z));
        }
    }
    v
}

/// Equal as the grid's element type stores them.
fn same_value(class_id: u32, a: f64, b: f64) -> bool {
    match class_id {
        grid_class::F32 => (a as f32).to_bits() == (b as f32).to_bits(),
        grid_class::U32 => a as u32 == b as u32,
        _ => a as u8 == b as u8,
    }
}

// ---- the shadow's host -----------------------------------------------------------------------------

/// The properties the original's delivery may change and the copy reads
/// (the pre-image): on lot holders, and on connection holders.
const LOT_PROPS: [u32; 1] = [prop::TRIP_CAPACITY];
const CONNECTION_PROPS: [u32; 8] = [
    prop::TRIP_CAPACITY,
    prop::POOL_ARRIVALS,
    prop::POOL_ARRIVALS + 1,
    prop::POOL_ARRIVALS + 2,
    prop::NEIGHBOUR_TARGETS,
    prop::NEIGHBOUR_TARGETS + 1,
    prop::NEIGHBOUR_TARGETS + 2,
    prop::CONNECTION_TRIP_LENGTH,
];
/// Written by the copy, not compared: the values come from the original's
/// generator, which the shadow cannot read (§4.15 draws 1, 2).
const UNCOMPARED_PROPS: [u32; 1] = [prop::STRATEGY_DRAW];

/// Calls the copy made that the shadow keeps (none reaches the game).
#[derive(Clone, Copy, Debug, Default)]
pub struct Kept {
    pub posts: u32,
    pub history: u32,
    pub incomes: u32,
    pub network_costs: u32,
    pub budget_items: u32,
    pub accidents: u32,
    pub lot_flags: u32,
    pub agents: u32,
    pub refs: u32,
}

/// The copy's `TrafficSimHost`: reads go to the live host (the pre-image
/// first for what the original's delivery may change), writes stay here.
pub struct ShadowHost {
    inner: Rc<dyn TrafficSimHost>,
    /// The original's grids by token, as private copies.
    grids: RefCell<HashMap<u64, GridSlot>>,
    /// Class IDs of the original's objects (grids, buildings, departments),
    /// from the captures: the copy's record names them as the original's.
    classes: RefCell<HashMap<u64, u32>>,
    next_fresh: Cell<u64>,
    /// The pre-image (see the module notes).
    trips_pre: RefCell<HashMap<Source, Vec<TripData>>>,
    props_pre: RefCell<HashMap<(u64, u32), Option<Variant>>>,
    /// The copy's writes, in first-write order.
    trips_ours: RefCell<HashMap<Source, Vec<TripData>>>,
    trips_order: RefCell<Vec<Source>>,
    props_ours: RefCell<HashMap<(u64, u32), Option<Variant>>>,
    props_order: RefCell<Vec<(u64, u32)>>,
    /// The original's lots for this delivery (one lot walk).
    lots_cache: RefCell<Option<Rc<[u64]>>>,
    /// References actually acquired from the live host, including the
    /// current traffic source across daily deliveries (§4.1).
    held: RefCell<HashMap<u64, u32>>,
    pub kept: Cell<Kept>,
    pub posted: RefCell<Vec<SimMessage>>,
}

/// Synthetic tokens of the grids the copy creates itself (only before its
/// first `Read`).
const FRESH_GRID_BASE: u64 = 0x000F_C4E0_0000_0000;

impl ShadowHost {
    pub fn new(inner: Rc<dyn TrafficSimHost>) -> Rc<ShadowHost> {
        Rc::new(ShadowHost {
            inner,
            grids: RefCell::new(HashMap::new()),
            classes: RefCell::new(HashMap::new()),
            next_fresh: Cell::new(FRESH_GRID_BASE),
            trips_pre: RefCell::new(HashMap::new()),
            props_pre: RefCell::new(HashMap::new()),
            trips_ours: RefCell::new(HashMap::new()),
            trips_order: RefCell::new(Vec::new()),
            props_ours: RefCell::new(HashMap::new()),
            props_order: RefCell::new(Vec::new()),
            lots_cache: RefCell::new(None),
            held: RefCell::new(HashMap::new()),
            kept: Cell::new(Kept::default()),
            posted: RefCell::new(Vec::new()),
        })
    }

    /// The live host.
    pub fn inner(&self) -> &Rc<dyn TrafficSimHost> {
        &self.inner
    }

    fn keep(&self, f: impl FnOnce(&mut Kept)) {
        let mut k = self.kept.get();
        f(&mut k);
        self.kept.set(k);
    }

    /// The class IDs of a capture's objects.
    pub fn learn_classes(&self, c: &Capture) {
        let mut m = self.classes.borrow_mut();
        for r in &c.refs {
            for t in [r.grid, r.occupant, r.department] {
                if t != 0 {
                    m.insert(t, r.class_id);
                }
            }
        }
    }

    /// The private copy of grid `token` (class `class_id`) holds the live
    /// values: made when missing or of another geometry, refreshed
    /// otherwise. False when the grid cannot be read.
    pub fn load_grid(&self, live: &dyn LiveGrids, token: u64, class_id: u32) -> bool {
        let Some(geom) = live.geometry(token) else {
            return false;
        };
        let values = live_values(live, token, class_id, &geom);
        let mut grids = self.grids.borrow_mut();
        let keep = grids.get(&token).is_some_and(|g| g.geometry() == geom);
        macro_rules! fill {
            ($t:ty, $variant:ident) => {{
                let vals: Vec<$t> = values.iter().map(|&v| <$t>::from_f64(v)).collect();
                match grids.get(&token) {
                    Some(GridSlot::$variant(g)) if keep => g.data.set_values(&vals),
                    _ => {
                        let g = Rc::new(ShadowGrid {
                            data: SimGrid::<$t>::from_values(geom, &vals),
                            token,
                            class_id,
                        });
                        grids.insert(token, GridSlot::$variant(g));
                    }
                }
            }};
        }
        match class_id {
            grid_class::BOOL | grid_class::U8 => fill!(u8, U8),
            grid_class::U32 => fill!(u32, U32),
            grid_class::F32 => fill!(f32, F32),
            _ => return false,
        }
        true
    }

    /// Compares the private copy of every grid in `tokens` with the live
    /// one: (token, cells compared, cells differing, first differing cells
    /// as (tx, tz, original, ours)).
    pub fn compare_grids(&self, live: &dyn LiveGrids, tokens: &[u64], max_cells: usize) -> Vec<GridDiff> {
        let grids = self.grids.borrow();
        let classes = self.classes.borrow();
        let mut out = Vec::new();
        for &token in tokens {
            let Some(g) = grids.get(&token) else { continue };
            let class_id = classes.get(&token).copied().unwrap_or(0);
            let geom = g.geometry();
            let mut d = GridDiff {
                token,
                ..GridDiff::default()
            };
            match live.geometry(token) {
                Some(lg) if lg == geom => {}
                _ => {
                    d.geometry_differs = true;
                    out.push(d);
                    continue;
                }
            }
            let ours = g.values();
            let orig = live_values(live, token, class_id, &geom);
            let nz = geom.count_z.max(1);
            for (i, (&o, &b)) in orig.iter().zip(&ours).enumerate() {
                d.cells += 1;
                if !same_value(class_id, o, b) {
                    d.differing += 1;
                    if d.first.len() < max_cells {
                        d.first.push((i as i32 / nz, i as i32 % nz, o, b));
                    }
                }
            }
            out.push(d);
        }
        out
    }

    /// The pre-image: every source's trip list and the holder properties of
    /// [`LOT_PROPS`] and [`CONNECTION_PROPS`], as they are now; the copy's
    /// writes are forgotten.
    pub fn snapshot_inputs(&self, current: Option<Source>) {
        let h = &*self.inner;
        self.lots_cache.borrow_mut().take();
        let mut trips = HashMap::new();
        let mut props = HashMap::new();
        for lot in h.lots() {
            trips.insert(Source::Lot(lot), h.trip_list(Source::Lot(lot)));
            let holder = h.lot_holder(lot);
            if holder != 0 {
                for id in LOT_PROPS {
                    props.insert((holder, id), h.holder_property(holder, id));
                }
            }
        }
        for c in h.all_connections() {
            trips.insert(Source::Connection(c), h.trip_list(Source::Connection(c)));
            let holder = h.connection_holder(c);
            if holder != 0 {
                for id in CONNECTION_PROPS {
                    props.insert((holder, id), h.holder_property(holder, id));
                }
            }
        }
        // A removed current source is absent from the city walk but still
        // owned by both simulators (§4.1). Capture its pre-delivery list
        // too; falling through to a post-delivery live read is not a snapshot.
        if let Some(s) = current {
            trips.entry(s).or_insert_with(|| h.trip_list(s));
            let (holder, ids) = match s {
                Source::Lot(l) => (h.lot_holder(l), LOT_PROPS.as_slice()),
                Source::Connection(c) => (h.connection_holder(c), CONNECTION_PROPS.as_slice()),
            };
            if holder != 0 {
                for &id in ids {
                    props
                        .entry((holder, id))
                        .or_insert_with(|| h.holder_property(holder, id));
                }
            }
        }
        *self.trips_pre.borrow_mut() = trips;
        *self.props_pre.borrow_mut() = props;
        self.forget_writes();
    }

    /// Forgets the copy's writes (trip lists, holder properties, posts).
    pub fn forget_writes(&self) {
        self.trips_ours.borrow_mut().clear();
        self.trips_order.borrow_mut().clear();
        self.props_ours.borrow_mut().clear();
        self.props_order.borrow_mut().clear();
        self.posted.borrow_mut().clear();
    }

    /// The copy's trip-list writes and the pre-image sources.
    fn sources(&self) -> Vec<Source> {
        let mut seen: HashSet<Source> = HashSet::new();
        let mut v = Vec::new();
        for s in self.trips_order.borrow().iter() {
            if seen.insert(*s) {
                v.push(*s);
            }
        }
        let mut pre: Vec<Source> = self.trips_pre.borrow().keys().copied().collect();
        pre.sort();
        for s in pre {
            if seen.insert(s) {
                v.push(s);
            }
        }
        v
    }

    /// The trip list the copy left (its write, else the pre-image).
    fn expected_trips(&self, s: Source) -> Option<Vec<TripData>> {
        if let Some(t) = self.trips_ours.borrow().get(&s) {
            return Some(t.clone());
        }
        self.trips_pre.borrow().get(&s).cloned()
    }

    fn written_trips(&self, s: Source) -> bool {
        self.trips_ours.borrow().contains_key(&s)
    }

    /// The (holder, property) pairs to compare: the copy's writes, then the
    /// pre-image.
    fn holder_props(&self) -> Vec<(u64, u32)> {
        let mut seen: HashSet<(u64, u32)> = HashSet::new();
        let mut v = Vec::new();
        for k in self.props_order.borrow().iter() {
            if !UNCOMPARED_PROPS.contains(&k.1) && seen.insert(*k) {
                v.push(*k);
            }
        }
        let mut pre: Vec<(u64, u32)> = self.props_pre.borrow().keys().copied().collect();
        pre.sort();
        for k in pre {
            if seen.insert(k) {
                v.push(k);
            }
        }
        v
    }

    fn expected_prop(&self, k: (u64, u32)) -> Option<Variant> {
        if let Some(v) = self.props_ours.borrow().get(&k) {
            return v.clone();
        }
        if let Some(v) = self.props_pre.borrow().get(&k) {
            return v.clone();
        }
        self.inner.holder_property(k.0, k.1)
    }

    fn fresh_grid<T: GridElem>(&self, instance: u32, tract_size: i32) -> Rc<ShadowGrid<T>> {
        let (cells_x, cells_z) = self.inner.cell_counts();
        let w = self.inner.cell_width();
        let city = CityDims {
            cells_x,
            cells_z,
            cell_width_x: w,
            cell_width_z: w,
        };
        let token = self.next_fresh.get();
        self.next_fresh.set(token + 1);
        Rc::new(ShadowGrid {
            data: SimGrid::<T>::create(&city, instance, tract_size),
            token,
            class_id: T::CLSID,
        })
    }
}

/// One grid's comparison.
#[derive(Clone, Debug, Default)]
pub struct GridDiff {
    pub token: u64,
    pub geometry_differs: bool,
    pub cells: u64,
    pub differing: u64,
    /// (tx, tz, original, ours).
    pub first: Vec<(i32, i32, f64, f64)>,
}

impl GridProvider<u8> for ShadowHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        Some(self.fresh_grid::<u8>(instance, tract_size) as Rc<dyn TractGrid<u8>>)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u8>>> {
        let RefValue::Host { token, .. } = *r else { return None };
        match self.grids.borrow().get(&token) {
            Some(GridSlot::U8(g)) => Some(g.clone() as Rc<dyn TractGrid<u8>>),
            _ => None,
        }
    }
}

impl GridProvider<u32> for ShadowHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u32>>> {
        Some(self.fresh_grid::<u32>(instance, tract_size) as Rc<dyn TractGrid<u32>>)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u32>>> {
        let RefValue::Host { token, .. } = *r else { return None };
        match self.grids.borrow().get(&token) {
            Some(GridSlot::U32(g)) => Some(g.clone() as Rc<dyn TractGrid<u32>>),
            _ => None,
        }
    }
}

impl GridProvider<f32> for ShadowHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<f32>>> {
        Some(self.fresh_grid::<f32>(instance, tract_size) as Rc<dyn TractGrid<f32>>)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<f32>>> {
        let RefValue::Host { token, .. } = *r else { return None };
        match self.grids.borrow().get(&token) {
            Some(GridSlot::F32(g)) => Some(g.clone() as Rc<dyn TractGrid<f32>>),
            _ => None,
        }
    }
}

impl NetworkHost for ShadowHost {
    fn cell_counts(&self) -> (i32, i32) {
        self.inner.cell_counts()
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        self.inner.position_to_cell(x, z)
    }
    fn network_occupants_in_rect(&self, rect: CellRect) -> Vec<u64> {
        self.inner.network_occupants_in_rect(rect)
    }
    fn subway_occupants_in_rect(&self, rect: CellRect) -> Vec<u64> {
        self.inner.subway_occupants_in_rect(rect)
    }
    fn prop_occupants_in_rect(&self, rect: CellRect) -> Vec<u64> {
        self.inner.prop_occupants_in_rect(rect)
    }
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        self.inner.occupant_position(occ)
    }
    fn occupant_bounding_cells(&self, occ: u64) -> CellRect {
        self.inner.occupant_bounding_cells(occ)
    }
    fn prop_type(&self, occ: u64) -> u32 {
        self.inner.prop_type(occ)
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant> {
        self.inner.occupant_property(occ, id)
    }
    fn has_network_flag(&self, occ: u64, flag: u32) -> bool {
        self.inner.has_network_flag(occ, flag)
    }
    fn has_any_network_flag(&self, occ: u64, mask: u32) -> bool {
        self.inner.has_any_network_flag(occ, mask)
    }
    fn is_of_type(&self, occ: u64, network_type: u32) -> bool {
        self.inner.is_of_type(occ, network_type)
    }
    fn path_connection(&self, occ: u64, path_type: u32, entry: u32, exit: u32) -> bool {
        self.inner.path_connection(occ, path_type, entry, exit)
    }
    fn path_info(&self, occ: u64) -> Option<PathInfo> {
        self.inner.path_info(occ)
    }
    fn path_info_turn_around(&self, pi: PathInfo, path_type: u32, entry: u32) -> bool {
        self.inner.path_info_turn_around(pi, path_type, entry)
    }
    fn path_info_exits_from_entry(&self, pi: PathInfo, path_type: u32, entry: u32) -> Vec<(u8, u64)> {
        self.inner.path_info_exits_from_entry(pi, path_type, entry)
    }
    fn path_info_entries_near(&self, pi: PathInfo, path_type: u32, vertex: u64, exit: u32, radius: f32) -> Vec<u32> {
        self.inner.path_info_entries_near(pi, path_type, vertex, exit, radius)
    }
    fn is_tunnel(&self, occ: u64) -> bool {
        self.inner.is_tunnel(occ)
    }
    fn tunnel_paired_portal(&self, occ: u64) -> Option<u64> {
        self.inner.tunnel_paired_portal(occ)
    }
    fn lot_at(&self, x: i32, z: i32, anchor_only: bool) -> Option<u64> {
        self.inner.lot_at(x, z, anchor_only)
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        self.inner.lot_bounding_rect(lot)
    }
    fn lot_location(&self, lot: u64) -> (i32, i32) {
        self.inner.lot_location(lot)
    }
    fn lot_facing(&self, lot: u64) -> u32 {
        self.inner.lot_facing(lot)
    }
    fn lot_building(&self, lot: u64) -> Option<u64> {
        self.inner.lot_building(lot)
    }
    fn lot_populations(&self, lot: u64) -> Vec<Vec<(u32, u32)>> {
        self.inner.lot_populations(lot)
    }
    fn lot_holder(&self, lot: u64) -> u64 {
        self.inner.lot_holder(lot)
    }
    fn holder_property(&self, holder: u64, id: u32) -> Option<Variant> {
        if let Some(v) = self.props_ours.borrow().get(&(holder, id)) {
            return v.clone();
        }
        if let Some(v) = self.props_pre.borrow().get(&(holder, id)) {
            return v.clone();
        }
        self.inner.holder_property(holder, id)
    }
    fn set_holder_property(&self, holder: u64, id: u32, value: Variant) {
        if self.props_ours.borrow_mut().insert((holder, id), Some(value)).is_none() {
            self.props_order.borrow_mut().push((holder, id));
        }
    }
    fn remove_holder_property(&self, holder: u64, id: u32) -> bool {
        let had = self.holder_property(holder, id).is_some();
        if self.props_ours.borrow_mut().insert((holder, id), None).is_none() {
            self.props_order.borrow_mut().push((holder, id));
        }
        had
    }
    fn max_trip_capacity(&self, holder: u64, k: u32) -> u32 {
        super::adapters::capacity_element(self, holder, k as usize)
    }
    fn neighbor_pool_holder(&self, x: i32, z: i32) -> Option<u64> {
        self.inner.neighbor_pool_holder(x, z)
    }
    fn neighbor_pool_holder_in(&self, s: &mut NetworkState, x: i32, z: i32) -> Option<u64> {
        self.inner.neighbor_pool_holder_in(s, x, z)
    }
    fn is_road_damaged(&self, x: i32, z: i32) -> bool {
        self.inner.is_road_damaged(x, z)
    }
    fn cell_under_water(&self, x: i32, z: i32) -> bool {
        self.inner.cell_under_water(x, z)
    }
    fn is_navigable(&self, x: i32, z: i32) -> bool {
        self.inner.is_navigable(x, z)
    }
    fn water_depth(&self, x: i32, z: i32) -> f32 {
        self.inner.water_depth(x, z)
    }
    fn body_of_water(&self, x: i32, z: i32) -> Option<u16> {
        self.inner.body_of_water(x, z)
    }
    fn connections_of_type(&self, connection_type: u32) -> Vec<u64> {
        self.inner.connections_of_type(connection_type)
    }
    fn all_connections(&self) -> Vec<u64> {
        self.inner.all_connections()
    }
    fn connection_cell(&self, conn: u64) -> (i32, i32) {
        self.inner.connection_cell(conn)
    }
    fn post(&self, msg_type: u32) {
        self.keep(|k| k.posts += 1);
        self.posted.borrow_mut().push(SimMessage::of_type(msg_type));
    }
    fn add_ref(&self, token: u64) {
        self.keep(|k| k.refs += 1);
        self.inner.add_ref(token);
        *self.held.borrow_mut().entry(token).or_default() += 1;
    }
    fn release(&self, token: u64) {
        self.keep(|k| k.refs += 1);
        // Read restores borrowed record tokens, not acquired references.
        // Never release one unless this shadow took a reference itself.
        let mut held = self.held.borrow_mut();
        if let Some(n) = held.get_mut(&token) {
            *n -= 1;
            self.inner.release(token);
            if *n == 0 {
                held.remove(&token);
            }
        }
    }
}

impl Drop for ShadowHost {
    fn drop(&mut self) {
        for (token, count) in self.held.get_mut().drain() {
            for _ in 0..count {
                self.inner.release(token);
            }
        }
    }
}

impl TrafficSimHost for ShadowHost {
    fn add_agent(&self, _level: u32, _name: &str, _flags: u32) {
        self.keep(|k| k.agents += 1);
    }
    fn subscribe(&self, _msg_type: u32) {
        self.keep(|k| k.agents += 1);
    }
    fn post_message(&self, msg: SimMessage) {
        self.keep(|k| k.posts += 1);
        self.posted.borrow_mut().push(msg);
    }
    fn object_ref(&self, token: u64) -> RefValue {
        if token == 0 {
            return RefValue::Null;
        }
        let class_id = self.classes.borrow().get(&token).copied().unwrap_or(0);
        RefValue::Host { token, class_id }
    }
    fn resolve_ref(&self, r: &RefValue, _iid: u32) -> Option<u64> {
        match *r {
            RefValue::Host { token, .. } if token != 0 => Some(token),
            _ => None,
        }
    }
    fn cell_width(&self) -> f32 {
        self.inner.cell_width()
    }
    fn tuning_exemplar(&self) -> Option<Rc<PropertyHolder>> {
        self.inner.tuning_exemplar()
    }
    fn sim_day_of_month(&self) -> u32 {
        self.inner.sim_day_of_month()
    }
    fn sim_date_number(&self) -> u32 {
        self.inner.sim_date_number()
    }
    fn sim_clock_ms(&self) -> u64 {
        self.inner.sim_clock_ms()
    }
    /// The original's department, borrowed (the copy creates nothing).
    fn create_department(&self, id: u32, _max_funding: f32) -> Option<u64> {
        self.inner.department_budget(id)
    }
    fn release_department(&self, _department: u64) {}
    /// The original's departments are the original's: nothing is created.
    fn create_line_item(&self, _department: u64, _item: super::dept::LineItem, _only_if_missing: bool) {}
    fn department_funding(&self, department: u64) -> f32 {
        self.inner.department_funding(department)
    }
    fn set_line_item_income(&self, _department: u64, _item: u32, _income: i64) {
        self.keep(|k| k.incomes += 1);
    }
    fn update_network_costs(&self, _d: u64, _mass_transit: bool, _cells: &[u32; 13], _costs: &[f32; 13]) {
        self.keep(|k| k.network_costs += 1);
    }
    /// The cost the original's budget would have booked, read without
    /// booking it (budget simulator slot 51 on the building, the
    /// `0xCA550301` purpose: §4.1.3 step 3 adds only that path's cost).
    fn activate_budget_items(&self, building: u64, _lot: Option<u64>) -> i64 {
        self.keep(|k| k.budget_items += 1);
        self.inner
            .budget_item_for_purpose(building, prop::BUDGET_ITEM)
            .map_or(0, |(_, cost)| cost)
    }
    /// The deactivation's cost (§4.1.3, TS23), read without removing
    /// anything: the shadow's host table gives `deactivate_budget_items` as
    /// a lookup on the lot's holder (slot 51, nothing removed; fc4.h traffic
    /// shadow). The original's own handler may have removed the item first
    /// (it gets the lot message before the copy): the cost is 0 then and
    /// the next record check resyncs the copy's total.
    fn deactivate_budget_items(&self, holder: u64) -> i64 {
        self.keep(|k| k.budget_items += 1);
        self.inner.deactivate_budget_items(holder)
    }
    fn budget_item_for_purpose(&self, building: u64, purpose: u32) -> Option<(u32, i64)> {
        self.inner.budget_item_for_purpose(building, purpose)
    }
    fn department_budget(&self, department_id: u32) -> Option<u64> {
        self.inner.department_budget(department_id)
    }
    fn record_history(&self, _series: u32, _value: u32) {
        self.keep(|k| k.history += 1);
    }
    fn lots(&self) -> Vec<u64> {
        self.lots_shared().to_vec()
    }
    /// One lot walk per shadow call (the lots cannot change inside it).
    fn lots_shared(&self) -> Rc<[u64]> {
        if let Some(l) = self.lots_cache.borrow().as_ref() {
            return l.clone();
        }
        let l = self.inner.lots_shared();
        *self.lots_cache.borrow_mut() = Some(l.clone());
        l
    }
    fn as_lot(&self, object: u64) -> Option<u64> {
        self.inner.as_lot(object)
    }
    fn lot_zone(&self, lot: u64) -> u32 {
        self.inner.lot_zone(lot)
    }
    fn lot_is_livable(&self, lot: u64) -> bool {
        self.inner.lot_is_livable(lot)
    }
    fn lot_has_road_access(&self, lot: u64) -> bool {
        self.inner.lot_has_road_access(lot)
    }
    fn lot_link(&self, lot: u64) -> Option<u64> {
        self.inner.lot_link(lot)
    }
    fn lot_travel_population(&self, lot: u64, developer_type: u32) -> u32 {
        self.inner.lot_travel_population(lot, developer_type)
    }
    fn lot_strategy_byte(&self, lot: u64) -> u8 {
        self.inner.lot_strategy_byte(lot)
    }
    fn clear_lot_flag(&self, _lot: u64, _flag: u32) {
        self.keep(|k| k.lot_flags += 1);
    }
    /// The copy computes every source's volume itself (§4.1.1, §4.1.4): the
    /// source's own method would write the game's list.
    fn native_update_traffic_volume(&self, _source: Source) -> bool {
        false
    }
    fn trip_list(&self, source: Source) -> Vec<TripData> {
        if let Some(t) = self.trips_ours.borrow().get(&source) {
            return t.clone();
        }
        if let Some(t) = self.trips_pre.borrow().get(&source) {
            return t.clone();
        }
        self.inner.trip_list(source)
    }
    fn set_trip_list(&self, source: Source, trips: Vec<TripData>) {
        if self.trips_ours.borrow_mut().insert(source, trips).is_none() {
            self.trips_order.borrow_mut().push(source);
        }
    }
    fn connection_holder(&self, connection: u64) -> u64 {
        self.inner.connection_holder(connection)
    }
    fn take_connection_volume_dirty(&self, connection: u64) -> bool {
        self.inner.take_connection_volume_dirty(connection)
    }
    fn find_connection(&self, connection_type: u32, x: i32, z: i32) -> Option<u64> {
        self.inner.find_connection(connection_type, x, z)
    }
    fn connection_type(&self, connection: u64) -> u32 {
        self.inner.connection_type(connection)
    }
    fn connection_on_other_side(&self, connection: u64) -> Option<u64> {
        self.inner.connection_on_other_side(connection)
    }
    fn established_city_position(&self, connection: u64) -> Option<(i32, i32)> {
        self.inner.established_city_position(connection)
    }
    fn connected_cities(&self) -> Vec<NeighbourCity> {
        self.inner.connected_cities()
    }
    fn regional_demand(&self, w: u32) -> Option<f32> {
        self.inner.regional_demand(w)
    }
    fn workforce_percent(&self, x: i32, z: i32) -> f32 {
        self.inner.workforce_percent(x, z)
    }
    fn developer_population(&self, developer_type: u32, x: i32, z: i32) -> i16 {
        self.inner.developer_population(developer_type, x, z)
    }
    fn is_ordinance_on(&self, id: u32) -> bool {
        self.inner.is_ordinance_on(id)
    }
    fn ordinances_on(&self) -> Vec<u32> {
        self.inner.ordinances_on()
    }
    fn ordinance_property(&self, id: u32, p: u32) -> Option<Variant> {
        self.inner.ordinance_property(id, p)
    }
    fn as_network_occupant(&self, object: u64) -> Option<u64> {
        self.inner.as_network_occupant(object)
    }
    fn network_occupant_cell(&self, occ: u64) -> (i32, i32) {
        self.inner.network_occupant_cell(occ)
    }
    fn occupants_in_cells(&self, rect: CellRect) -> Vec<u64> {
        self.inner.occupants_in_cells(rect)
    }
    fn occupant_filter_includes(&self, filter: u64, occupant: u64) -> bool {
        self.inner.occupant_filter_includes(filter, occupant)
    }
    fn cell_has_network_occupant(&self, x: i32, z: i32) -> bool {
        self.inner.cell_has_network_occupant(x, z)
    }
    fn drives_on_right(&self) -> bool {
        self.inner.drives_on_right()
    }
    fn view_cell_rect(&self) -> Option<CellRect> {
        self.inner.view_cell_rect()
    }
    fn create_accident(&self, _x: i32, _z: i32, _duration_ms: i64) {
        self.keep(|k| k.accidents += 1);
    }
}

// ---- comparisons ------------------------------------------------------------------------------------

/// An edge-density record in row 7: u32 key, 16 + 1 + 16 u16.
const EDGE_RECORD_BYTES: usize = 4 + 2 * 33;

/// The keys of a damage key list (u32 n, n × u16).
fn damage_keys(b: &[u8]) -> Option<Vec<u32>> {
    let n = u32::from_le_bytes(b.get(..4)?.try_into().ok()?) as usize;
    if b.len() != 4 + 2 * n {
        return None;
    }
    Some(
        b[4..]
            .chunks_exact(2)
            .map(|c| u32::from(u16::from_le_bytes([c[0], c[1]])))
            .collect(),
    )
}

/// The leading u32 key of each `size`-byte record.
fn record_keys(b: &[u8], size: usize) -> Option<Vec<u32>> {
    if b.len() % size != 0 {
        return None;
    }
    Some(
        b.chunks_exact(size)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
    )
}

/// Switch keys in capture order (row 9). References and rule lists make
/// these records variable-sized; reject an incomplete field unchanged.
fn switch_keys(b: &[u8]) -> Option<Vec<u32>> {
    let mut r = ByteReader::new(b);
    let mut keys = Vec::new();
    while r.remaining() != 0 {
        let id = r.get_u32()?;
        if r.get_u32()? != 0 && !r.skip(4) {
            return None;
        }
        if !r.skip(23) {
            return None;
        }
        let rules = r.get_u32()? as usize;
        if !r.skip(rules.checked_mul(9)?.checked_add(5)?) {
            return None;
        }
        keys.push(id);
    }
    Some(keys)
}

/// The smallest listed bucket count, at least the default (193) and the key
/// count, under which `keys` is in bucket-ascending order (a table never
/// shrinks, so a larger count than this is possible; the first few
/// candidates are tried).
fn order_bucket_count(keys: &[u32]) -> Option<usize> {
    let min = keys.len().max(STL_DEFAULT_HINT);
    STL_PRIMES
        .iter()
        .map(|&p| p as usize)
        .filter(|&p| p >= min)
        .take(6)
        .find(|&p| keys.windows(2).all(|w| w[0] as usize % p <= w[1] as usize % p))
}

/// A set whose iteration order is `keys` (see `restore_orders`).
fn ordered_set(keys: &[u32]) -> Option<StlHashSet> {
    let p = order_bucket_count(keys)?;
    let mut s = StlHashSet::with_hint(p);
    for &k in keys.iter().rev() {
        s.insert(k);
    }
    (s.iter_order() == keys).then_some(s)
}

fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02X}"));
    }
    s
}

/// A field's two values as text: short fields in hex (4 bytes also as f32
/// and u32), long ones by their first differing byte.
fn describe_field(orig: &[u8], ours: &[u8]) -> String {
    if orig.len() <= 16 && ours.len() <= 16 {
        let mut s = format!("orig {} ours {}", hex(orig), hex(ours));
        if orig.len() == 4 && ours.len() == 4 {
            let o = [orig[0], orig[1], orig[2], orig[3]];
            let b = [ours[0], ours[1], ours[2], ours[3]];
            s.push_str(&format!(
                " (u32 {}/{}, f32 {:?}/{:?})",
                u32::from_le_bytes(o),
                u32::from_le_bytes(b),
                f32::from_le_bytes(o),
                f32::from_le_bytes(b)
            ));
        }
        return s;
    }
    if orig.len() == ours.len() && orig.len() % 4 == 0 && orig.len() <= 80 {
        // A short table of words: each differing u32.
        let words: Vec<String> = orig
            .chunks_exact(4)
            .zip(ours.chunks_exact(4))
            .enumerate()
            .filter(|(_, (o, b))| o != b)
            .map(|(i, (o, b))| {
                let o = u32::from_le_bytes([o[0], o[1], o[2], o[3]]);
                let b = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                format!("[{i}] {o}/{b}")
            })
            .collect();
        return format!("u32 words orig/ours: {}", words.join(" "));
    }
    let n = orig.len().min(ours.len());
    let differing = (0..n).filter(|&i| orig[i] != ours[i]).count() + orig.len().max(ours.len()) - n;
    let first = (0..n).find(|&i| orig[i] != ours[i]).unwrap_or(n);
    let o = &orig[first.min(orig.len())..(first + 8).min(orig.len())];
    let b = &ours[first.min(ours.len())..(first + 8).min(ours.len())];
    format!(
        "{differing} of {} bytes differ (lengths {}/{}), first at +{first}: orig {} ours {}",
        orig.len().max(ours.len()),
        orig.len(),
        ours.len(),
        hex(o),
        hex(b)
    )
}

/// The fields of `ours` whose bytes differ from the original's record
/// `orig`, as (field, description). When the lengths differ, the fields are
/// compared up to the first difference only (after it they are not
/// aligned).
pub fn compare_record(orig: &[u8], ours: &CanonicalWriter) -> Vec<(&'static str, String)> {
    if orig == ours.bytes.as_slice() {
        return Vec::new();
    }
    let aligned = orig.len() == ours.bytes.len();
    let mut out = Vec::new();
    for (name, s, e) in ours.fields() {
        let o = &orig[s.min(orig.len())..e.min(orig.len())];
        let b = &ours.bytes[s..e];
        if o != b {
            out.push((name, describe_field(o, b)));
            if !aligned {
                out.push((
                    "(record length)",
                    format!(
                        "orig {} bytes, ours {}: the fields after {name} are not compared",
                        orig.len(),
                        ours.bytes.len()
                    ),
                ));
                break;
            }
        }
    }
    if out.is_empty() {
        out.push((
            "(record length)",
            format!("orig {} bytes, ours {}", orig.len(), ours.bytes.len()),
        ));
    }
    out
}

/// A record-field difference of a known kind (not counted): a field of rows
/// 14 and 15 the spec leaves unnamed (labels with `~`: the lot iterator
/// position, the re-route context but its cell, the context table and
/// flags, the three unused pacing words), which the copy keeps as read. The
/// kind is the field's own label, so the month line names it.
///
/// The pass counters (TS59), the re-route context cell and `dayIndex`
/// (TS60) and a statistics set's last byte (`freightByte`, TS61) are
/// modelled and compared exactly.
fn known_record_difference(name: &'static str, orig: &[u8], ours: &[u8]) -> Option<&'static str> {
    (orig.len() == ours.len() && name.starts_with('~')).then_some(name)
}

/// [`compare_record`] with the differences of a known kind
/// ([`known_record_difference`]) set apart: (the others, the known kinds).
pub fn compare_record_split(orig: &[u8], ours: &CanonicalWriter) -> (Vec<(&'static str, String)>, Vec<&'static str>) {
    let all = compare_record(orig, ours);
    if orig.len() != ours.bytes.len() {
        return (all, Vec::new());
    }
    let mut known = Vec::new();
    let mut rest = Vec::new();
    let fields = ours.fields();
    for (name, text) in all {
        let k = fields
            .iter()
            .find(|f| f.0 == name)
            .and_then(|&(_, s, e)| known_record_difference(name, &orig[s..e], &ours.bytes[s..e]));
        match k {
            Some(k) => known.push(k),
            None => rest.push((name, text)),
        }
    }
    (rest, known)
}

/// A trip list as its saved bytes (§6.3): count, then each trip.
fn trip_bytes(t: &[TripData]) -> Vec<u8> {
    let mut w = CanonicalWriter::new();
    w.put_u32(t.len() as u32);
    for trip in t {
        write_trip(&mut w, trip);
    }
    w.bytes
}

/// A trip-list difference of a known kind (not counted). The lists agree but
/// for, on each differing trip, either
/// - what a search the original's deadline suspended leaves in a status-0
///   trip: the destination, written as soon as the outbound leg is found
///   (`0x0071C752`), and on the two-leg path flags bit 0 cleared
///   (`0x0071C858`), nothing else (§4.4, TS65); the copy's searches never
///   suspend (§3.4 *[choice]*, accepted in TS65); or
/// - the destination and travel-type fields of a trip appended by the
///   reconcile (§4.1.1, §4.1.4), undefined in the original *[quirk]*, 0 in
///   ours *[choice]*: ours unrouted with (0, 0) and mask 0.
///
/// A reversible trip's return route is no longer a known kind: both sides
/// keep it (TS58).
fn known_trip_difference(orig: &[TripData], ours: &[TripData]) -> Option<&'static str> {
    const UNDEFINED: &str = "a new trip's undefined destination and travel types (§4.1.1 quirk)";
    const SUSPENDED: &str =
        "a status-0 or status-1 trip's destination and flags bit 0 (the original's suspended search, TS65, §3.4 [choice])";
    if orig.len() != ours.len() {
        return None;
    }
    let mut kind = None;
    for (o, b) in orig.iter().zip(ours) {
        if o == b {
            continue;
        }
        let mut o4 = o.clone();
        o4.dest_x = b.dest_x;
        o4.dest_z = b.dest_z;
        // Bit 0 cleared by the original's suspended two-leg search.
        if o4.flags & 1 == 0 && b.flags & 1 != 0 {
            o4.flags |= 1;
        }
        // A trip FullTrips (status 0) or ReRouteTrips (status 1) searches
        // (TS65: a suspended return leg leaves the new destination).
        let searched = matches!(o.status, super::status::SEARCH | super::status::REROUTE);
        let k = if searched && o.status == b.status && o4 == *b {
            // Read by nothing before the trip's search.
            SUSPENDED
        } else {
            let fresh = b.route.is_empty() && b.dest_x == 0 && b.dest_z == 0 && b.travel_types_used == 0;
            let mut o3 = o.clone();
            o3.dest_x = 0;
            o3.dest_z = 0;
            o3.travel_types_used = 0;
            if fresh && o3 == *b {
                UNDEFINED
            } else {
                return None;
            }
        };
        kind.get_or_insert(k);
    }
    kind
}

fn show_trip(t: &TripData) -> String {
    format!(
        "{{count {} type {} strategy {} flags {} status {} dest ({}, {}) length {:?} mask 0x{:X} route {}B return {}B}}",
        t.count,
        t.trip_type,
        t.strategy,
        t.flags,
        t.status,
        t.dest_x,
        t.dest_z,
        t.trip_length,
        t.travel_types_used,
        t.route.len(),
        t.return_route.len()
    )
}

fn show_variant(v: &Option<Variant>) -> String {
    match v {
        None => "absent".to_string(),
        Some(v) => {
            let items: Vec<String> = v.items().iter().map(|e| format!("{}", e.as_f64())).collect();
            format!("type 0x{:04X} [{}]", v.mem_type(), items.join(","))
        }
    }
}

fn date(julian: i32) -> String {
    if julian <= 0 {
        return "(no date yet)".to_string();
    }
    let d = civil_from_julian(julian);
    format!("{:04}-{:02}-{:02}", d.year, d.month, d.day)
}

/// The cell-scanning phases (§3.2), which the original's deadline can stop
/// mid-scan. [TS64] State 13 is one of them: its deadline test is before each
/// step of the scanning iterator, so a stopped call leaves the iterator at the
/// last cell visited and the copy must stop there too. State 12 is not: it
/// does not use the scanning iterator.
/// mid-scan; UpdateSubnetworkInfo (13) among them, on the same scanning
/// iterator (traffic-network.md §4.12, TS64).
fn is_scan_phase(s: u8) -> bool {
    matches!(
        s,
        cs::CREATE_MAPS
            | cs::CREATE_FINAL_MAP
            | cs::CLEAR_DESTINATIONS
            | cs::REDUCE_EDGES
            | cs::REFRESH_COMMUTE
            | cs::SUBNETWORK_INFO
    )
}

fn is_trip_phase(s: u8) -> bool {
    matches!(s, cs::CONFIRM_TRIPS | cs::REROUTE_TRIPS | cs::FULL_TRIPS)
}

fn delivery_kind(t: u32) -> &'static str {
    match t {
        msg::DAY => "day",
        msg::MONTH => "month",
        msg::SIM_BEGIN => "begin",
        _ => "message",
    }
}

/// The states a day can pass through from `from` to `to` (both included),
/// along the next-state table (12 and 13 both: the subnetwork step may be
/// skipped); every state when `to` is not reached within one cycle.
fn chain(from: u8, to: u8) -> HashSet<u8> {
    let mut set = HashSet::new();
    let mut s = from;
    set.insert(s);
    for _ in 0..24 {
        if s == to {
            return set;
        }
        s = cs::NEXT.get(usize::from(s)).copied().unwrap_or(cs::START_RECALC);
        set.insert(s);
        if s == cs::UPDATE_SUBNETWORKS {
            set.insert(cs::SUBNETWORK_INFO);
        }
    }
    (0..20).collect()
}

// ---- the driver ---------------------------------------------------------------------------------------

/// A delivery to the original's agent.
#[derive(Clone, Copy, Debug, Default)]
pub struct Delivery {
    pub msg_type: u32,
    pub data1: u32,
    pub data2: u32,
    pub data3: u32,
    /// The game date (Julian day number), for the lines.
    pub julian: i32,
}

/// Counters over the shadow's life (diagnostics, the capi stats).
#[derive(Clone, Copy, Debug, Default)]
pub struct ShadowStats {
    pub deliveries: u32,
    pub compared: u32,
    pub clean: u32,
    pub incomplete: u32,
    pub resyncs: u32,
    pub record_fields: u64,
    pub trip_lists: u64,
    pub trip_mismatches: u64,
    pub props: u64,
    pub prop_mismatches: u64,
    pub grid_cells: u64,
    pub grid_mismatches: u64,
    pub units: u64,
    pub last_mismatches: i32,
}

/// The month's sums (one month line).
#[derive(Default)]
struct MonthAcc {
    deliveries: u32,
    days: u32,
    months: u32,
    others: u32,
    compared: u32,
    clean: u32,
    incomplete: u32,
    drifts: u32,
    record_fields: u64,
    fields: BTreeMap<&'static str, u32>,
    trip_lists: u64,
    trip_mismatches: u64,
    props: u64,
    prop_mismatches: u64,
    grid_cells: u64,
    grid_mismatches: u64,
    units: u64,
    run_ons: u32,
    list_ends: u32,
    day_steps: u32,
    detail_lines: u32,
    first: Vec<String>,
    reasons: BTreeMap<String, u32>,
    /// Differences of a known kind (a spec question, not counted).
    known: BTreeMap<&'static str, u64>,
}

/// The delivery between [`TrafficShadow::before`] and
/// [`TrafficShadow::after`].
struct Pending {
    d: Delivery,
    /// Why nothing is compared (the copy could not start or read).
    failed: Option<String>,
    /// The record fields that differed just before the delivery (the copy
    /// was resynced).
    drift: Vec<(&'static str, String)>,
    from_state: u8,
}

/// One delivery's result.
#[derive(Clone, Debug, Default)]
pub struct Outcome {
    /// The difference count; -1 when nothing was compared.
    pub mismatches: i32,
    /// Log lines (a delivery line when something differs, the month line
    /// after a month delivery).
    pub lines: Vec<String>,
}

/// The traffic shadow (see the module notes).
pub struct TrafficShadow {
    host: Rc<ShadowHost>,
    copy: Rc<TrafficSimulator>,
    started: bool,
    /// The copy's grids hold the original's values (as of the last
    /// comparison).
    grids_synced: bool,
    /// The previous delivery's target (cycle state, progress), for the
    /// original's advance within a phase.
    last_target: Option<(u8, u64)>,
    max_diffs: usize,
    detail_lines_per_month: u32,
    pending: Option<Pending>,
    month: MonthAcc,
    pub stats: ShadowStats,
}

/// At most this many single steps per delivery (a guard; a day of the
/// original is far smaller).
const MAX_STEPS: u32 = 4_000_000;

impl TrafficShadow {
    /// A shadow over the live host `inner`; `max_diffs` differences are
    /// named per line, at most `detail_lines` delivery lines per month.
    pub fn new(inner: Rc<dyn TrafficSimHost>, session_seed: u32, max_diffs: usize, detail_lines: u32) -> TrafficShadow {
        let host = ShadowHost::new(inner);
        let copy = TrafficSimulator::new(host.clone() as Rc<dyn TrafficSimHost>, session_seed);
        TrafficShadow {
            host,
            copy,
            started: false,
            grids_synced: false,
            last_target: None,
            max_diffs: max_diffs.max(1),
            detail_lines_per_month: detail_lines,
            pending: None,
            month: MonthAcc::default(),
            stats: ShadowStats {
                last_mismatches: -1,
                ..ShadowStats::default()
            },
        }
    }

    /// The copy (tests, diagnostics).
    pub fn copy(&self) -> &Rc<TrafficSimulator> {
        &self.copy
    }

    /// The copy's host.
    pub fn host(&self) -> &Rc<ShadowHost> {
        &self.host
    }

    /// Whether the copy has started (the first delivery read the original's
    /// record).
    pub fn started(&self) -> bool {
        self.started
    }

    /// The copy's record in the capture format.
    pub fn record(&self) -> CanonicalWriter {
        let mut w = CanonicalWriter::new();
        self.copy.write(&mut w);
        w
    }

    fn load_grids(&self, c: &Capture, live: &dyn LiveGrids) {
        for r in &c.refs {
            if r.grid != 0 {
                self.host.load_grid(live, r.grid, r.class_id);
            }
        }
    }

    fn grid_tokens(c: &Capture) -> Vec<u64> {
        let mut seen = HashSet::new();
        c.refs
            .iter()
            .map(|r| r.grid)
            .filter(|&t| t != 0 && seen.insert(t))
            .collect()
    }

    /// Reads the original's record into the copy and resumes as a load does
    /// (§3.5 step 11 for the trip phases, [`TrafficSimulator::shadow_resume`]).
    fn resync(&self, c: &Capture) -> bool {
        if !self.copy.read(&mut CaptureReader::new(c)) {
            return false;
        }
        self.restore_orders(c);
        self.copy.shadow_resume();
        self.copy.shadow_apply_damage_costs();
        true
    }

    /// After a Read: gives the copy's SGI hash containers (the damage key
    /// sets, row 13, edge densities, row 7, and switches, row 9) the iteration order of
    /// the record they were read from. Our Read, like the original's (§6.2,
    /// TS54), inserts the keys one by one at the head of their chains, so it
    /// reverses each bucket's chain: right for a load, but the shadow reads
    /// the record the original WROTE from its live containers, whose order
    /// a Read does not give back (a record written after it would list the
    /// keys of a shared bucket in the other order, and the pothole picks,
    /// §4.12, would take other cells). The tooling *[choice]* rebuilds each
    /// container with the smallest listed
    /// bucket count (at least the default 193 and the key count) under which
    /// the record's order is bucket-ascending, inserting the keys last first;
    /// a container whose order no such count explains is left as read.
    /// Returns the containers rebuilt.
    fn restore_orders(&self, c: &Capture) -> u32 {
        let ours = self.record();
        if ours.bytes.len() != c.bytes.len() {
            return 0;
        }
        let field = |label: &str| -> Option<&[u8]> {
            ours.fields()
                .into_iter()
                .find(|f| f.0 == label)
                .map(|(_, a, b)| &c.bytes[a..b])
        };
        let mut done = 0;
        for (kind, labels) in super::record::DAMAGE_LABELS.iter().enumerate() {
            for (set_index, label) in labels[1..].iter().enumerate() {
                let Some(keys) = field(label).and_then(damage_keys) else {
                    continue;
                };
                let Some(set) = ordered_set(&keys) else { continue };
                self.copy.update_damage(|d| {
                    let b = if kind == 0 { &mut d.road } else { &mut d.rail };
                    let target = if set_index == 0 { &mut b.intact } else { &mut b.damaged };
                    if target.len() == set.len() && keys.iter().all(|&k| target.contains(k)) {
                        *target = set;
                        done += 1;
                    }
                });
            }
        }
        if let Some(keys) = field("edge densities").and_then(|b| record_keys(b, EDGE_RECORD_BYTES)) {
            self.copy.network().update_state(|n| {
                if keys.len() != n.edge_density.len() {
                    return;
                }
                let Some(p) = order_bucket_count(&keys) else { return };
                let mut m = StlHashMap::with_hint(p);
                for &k in keys.iter().rev() {
                    let Some(v) = n.edge_density.get(k).copied() else {
                        return;
                    };
                    m.entry(k, || v);
                }
                if m.keys() == keys {
                    n.edge_density = m;
                    done += 1;
                }
            });
        }
        if let Some(keys) = field("switches").and_then(switch_keys) {
            self.copy.network().update_state(|n| {
                if keys.len() != n.switches.len() {
                    return;
                }
                let Some(p) = order_bucket_count(&keys) else { return };
                let mut m = StlHashMap::with_hint(p);
                for &k in keys.iter().rev() {
                    let Some(v) = n.switches.get(k).cloned() else { return };
                    m.entry(k, || v);
                }
                if m.keys() == keys {
                    n.switches = m;
                    done += 1;
                }
            });
        }
        done
    }

    /// The copy's start on the original's record: Read (a load), Init,
    /// DoPostCityInit on the live city (the network model), then the record
    /// again (DoPostCityInit's own resets undone).
    fn start(&mut self, c: &Capture, live: &dyn LiveGrids) -> Result<(), String> {
        self.load_grids(c, live);
        if !self.copy.read(&mut CaptureReader::new(c)) {
            return Err("our Read of the original's record failed".to_string());
        }
        if !self.copy.init() {
            return Err("our Init failed".to_string());
        }
        self.copy.do_post_city_init();
        if !self.resync(c) {
            return Err("our second Read of the original's record failed".to_string());
        }
        self.copy.set_traffic_units_per_slice(1);
        self.copy.update_state(|s| s.shadow_lazy = true);
        self.started = true;
        self.grids_synced = true;
        Ok(())
    }

    /// Just before the original's delivery `d`, with its record `r0`.
    pub fn before(&mut self, d: Delivery, r0: Option<Capture>, live: &dyn LiveGrids) {
        self.host.lots_cache.borrow_mut().take();
        let mut p = Pending {
            d,
            failed: None,
            drift: Vec::new(),
            from_state: 0,
        };
        match r0 {
            None => p.failed = Some("no record before the delivery".to_string()),
            Some(r0) => {
                self.host.learn_classes(&r0);
                if !self.started {
                    if let Err(e) = self.start(&r0, live) {
                        p.failed = Some(e);
                    }
                } else {
                    if !self.grids_synced {
                        self.load_grids(&r0, live);
                        self.grids_synced = true;
                    }
                    let ours = self.record();
                    // A known kind of difference alone does not resync:
                    // the copy keeps its own value.
                    p.drift = compare_record_split(&r0.bytes, &ours).0;
                    if !p.drift.is_empty() {
                        self.load_grids(&r0, live);
                        if !self.resync(&r0) {
                            p.failed = Some("our Read of the original's record (resync) failed".to_string());
                        }
                    }
                }
            }
        }
        p.from_state = self.copy.cycle_state();
        self.host.snapshot_inputs(self.copy.read_state(|s| s.current));
        self.pending = Some(p);
    }

    /// Forwards a message the original receives (§3.1) to the copy, outside
    /// the agent deliveries: the occupant, lot, neighbour, budget, tuning and
    /// ordinance messages keep the copy's network model and state current.
    /// Its writes are not compared (the original's handler is not wrapped);
    /// the next delivery's record check catches a difference.
    pub fn message(&self, m: &Message) {
        if !self.started {
            return;
        }
        self.host.lots_cache.borrow_mut().take();
        self.copy.do_message(m);
        self.host.forget_writes();
    }

    /// Steps the copy to where the original's day stopped: `t` is the
    /// original's state after its delivery. Returns (units, a note when the
    /// copy could not get there).
    fn run_day(&mut self, t: &State, orig: &[u8]) -> (u32, Option<String>) {
        self.copy.shadow_day_start();
        let s0 = self.copy.cycle_state();
        let to = t.cycle_state;
        // The original's progress in the target phase, as the copy counts:
        // - in the phase it was in before (the previous target, when no
        //   resync came between): the copy's own progress plus the
        //   original's advance;
        // - entering 17 or 18 ends the original's day (§3.3): the copy
        //   stops on entry (`entering`);
        // - otherwise the target's own count (zeroed on the phase's entry).
        let p0 = self.copy.read_state(progress);
        let entering = s0 != to && matches!(to, cs::REROUTE_TRIPS | cs::FULL_TRIPS);
        let want = match self.last_target {
            Some((ls, lp)) if ls == to && s0 == to => p0.saturating_add(progress(t).saturating_sub(lp)),
            _ => progress(t),
        };
        let allowed = chain(s0, to);
        // A scanning phase the original's deadline stopped mid-scan: the
        // copy's scan stops at the same cell.
        let stop = (is_scan_phase(to) && t.scan.state == ScanState::Running).then_some((to, t.scan.x, t.scan.z));
        self.copy.update_state(|st| st.shadow_scan_stop = stop);
        // FinishTripMap (state 15) stopped by the original's deadline: the
        // copy's lot walk stops after the lot at the original's saved cell.
        let finish_stop = (to == cs::FINISH_TRIP_MAP).then_some(t.extras.phase_positions);
        self.copy.update_state(|st| st.shadow_finish_stop = finish_stop);
        // The original entered 15 in this delivery and its saved cell reads
        // (0, 0): it stopped at the test point before the first lot (TS63),
        // having taken none. The copy stops on entry; its entry action
        // zeroed the positions as the original's does (§3.2 row 15, TS67).
        let finish_on_entry = s0 != cs::FINISH_TRIP_MAP && finish_stop == Some([0, 0]);
        let mut steps = 0u32;
        let mut note = None;
        loop {
            let (s, p, at_stop, pos) = self.copy.read_state(|st| {
                let at = st.scan.state == ScanState::Running && Some((st.cycle_state, st.scan.x, st.scan.z)) == stop;
                (st.cycle_state, progress(st), at, st.extras.phase_positions)
            });
            let reached = if is_trip_phase(s) {
                // Entering 17 or 18 ends the original's day (§3.3): the copy
                // stops on entry. Its progress there is pass counter 4 as
                // state 16 left it (17) or 0 (18), compared with the record.
                entering || p >= want
            } else if s == cs::FINISH_TRIP_MAP {
                finish_on_entry || finish_stop.map_or(true, |f| f == pos)
            } else {
                stop.is_none() || at_stop
            };
            if s == to && reached {
                if is_trip_phase(s) && !entering && p > want {
                    note = Some(format!(
                        "ours passed the original's progress in state {s}: {p} > {want}"
                    ));
                }
                break;
            }
            if !allowed.contains(&s) {
                note = Some(format!(
                    "ours ran past the original's stop (state {to}, progress {want}) into state {s}"
                ));
                break;
            }
            if steps >= MAX_STEPS {
                note = Some(format!("step limit at state {s}, progress {p}"));
                break;
            }
            self.copy.shadow_slice();
            steps += 1;
        }
        self.copy.update_state(|st| {
            st.shadow_scan_stop = None;
            st.shadow_finish_stop = None;
        });
        let s = self.copy.cycle_state();
        let target = t.extras.phase_positions;
        // The share step (§3.3): the original ended its day on its share.
        // Taken before the run-on's record check below, which compares the
        // day index exactly (TS60): a run-on examines no trip and ending a
        // list leaves `processed` alone, so the share test reads the same.
        let (di, days, total, processed) = self
            .copy
            .read_state(|st| (st.day_index, st.days, st.total, st.processed));
        if is_trip_phase(s) && s == to && t.day_index == di.wrapping_add(1) {
            let share = (total as u32)
                .wrapping_mul(di.wrapping_add(1))
                .checked_div(days)
                .unwrap_or(u32::MAX);
            if di < days && processed as u32 >= share {
                self.copy.update_state(|st| st.day_index = st.day_index.wrapping_add(1));
                self.month.day_steps += 1;
            } else if note.is_none() {
                note = Some(format!(
                    "the original took day step {di} -> {} that our share test refuses \
                     (processed {processed}, share {share}, days {days})",
                    di.wrapping_add(1)
                ));
            }
        }
        let moved = want > p0 || self.copy.read_state(|st| st.extras.phase_positions) != target;
        if note.is_none() && s == to && s0 == to && matches!(s, cs::REROUTE_TRIPS | cs::FULL_TRIPS) && moved {
            // The original's deadline stopped it after its last unit at a
            // later test (before the next trip to examine, or at a list's
            // end): run on to its saved source cell.
            self.month.run_ons += 1;
            if self.copy.shadow_advance(target) {
                note = Some(format!(
                    "state {s}: ours found no further trip to examine after progress {want}, \
                     the original stopped before one"
                ));
            } else if let Some(src) = self.copy.shadow_pending_list_end() {
                // At the original's source with nothing left to examine: the
                // original stopped before or after the list's end (merge or
                // CompleteTripSourceCalculations). Taken when the record or
                // that source's trip list says so.
                let record_differs = !compare_record_split(orig, &self.record()).0.is_empty();
                let list_differs = self.host.expected_trips(src).is_some_and(|exp| {
                    let live = self.host.inner().trip_list(src);
                    trip_bytes(&live) != trip_bytes(&exp) && known_trip_difference(&live, &exp).is_none()
                });
                if record_differs || list_differs {
                    self.copy.shadow_end_list();
                    self.month.list_ends += 1;
                }
            }
        }
        (steps, note)
    }

    fn incomplete(&mut self, d: Delivery, why: String, lines: &mut Vec<String>) {
        self.stats.incomplete += 1;
        self.month.incomplete += 1;
        *self.month.reasons.entry(why.clone()).or_default() += 1;
        if self.month.detail_lines < self.detail_lines_per_month {
            self.month.detail_lines += 1;
            lines.push(format!(
                "traffic shadow {} {}: INCOMPLETE ({why}); not compared",
                date(d.julian),
                delivery_kind(d.msg_type)
            ));
        }
        self.stats.last_mismatches = -1;
        self.host.forget_writes();
        self.month_line(d, lines);
    }

    /// Just after the original's delivery, with its record `r1`: the copy's
    /// run and the comparison.
    pub fn after(&mut self, r1: Option<Capture>, live: &dyn LiveGrids) -> Outcome {
        self.host.lots_cache.borrow_mut().take();
        let Some(p) = self.pending.take() else {
            return Outcome {
                mismatches: -1,
                lines: Vec::new(),
            };
        };
        self.stats.deliveries += 1;
        let d = p.d;
        let kind = delivery_kind(d.msg_type);
        self.month.deliveries += 1;
        match d.msg_type {
            msg::DAY => self.month.days += 1,
            msg::MONTH => self.month.months += 1,
            _ => self.month.others += 1,
        }
        let mut lines = Vec::new();
        let Some(r1) = r1 else {
            self.incomplete(d, "no record after the delivery".to_string(), &mut lines);
            return Outcome { mismatches: -1, lines };
        };
        self.host.learn_classes(&r1);
        if let Some(why) = p.failed {
            self.incomplete(d, why, &mut lines);
            return Outcome { mismatches: -1, lines };
        }
        // The original's state after its delivery.
        let target = self
            .copy
            .snapshot()
            .and_then(|b| parse_record(&*self.host, &b, &mut CaptureReader::new(&r1)));
        let Some(target) = target else {
            self.incomplete(
                d,
                "the original's record after the delivery does not parse".to_string(),
                &mut lines,
            );
            return Outcome { mismatches: -1, lines };
        };
        let t = target.state;
        let drifted = !p.drift.is_empty();
        if drifted {
            self.last_target = None;
        }
        // Ours.
        let (units, note) = if d.msg_type == msg::DAY {
            // WorkOnSimCycle rebuilds intersections before the phase's
            // first deadline test (§3.2; traffic-network §4.6). The
            // original can therefore clear the flag with zero progress.
            // Replay that prelude when its stop is after it, just as the
            // scan/source positions bound the phase replay below. This
            // also rebuilds capacities; copying the flag would lose them.
            if !target.intersection_dirty {
                self.copy.network().update_intersection_data();
            }
            self.run_day(&t, &r1.bytes)
        } else {
            self.copy.do_message(&Message {
                msg_type: d.msg_type,
                data1: d.data1,
                data2: d.data2,
                data3: d.data3,
                object: 0,
            });
            (0, None)
        };
        self.month.units += u64::from(units);
        self.stats.units += u64::from(units);
        self.last_target = Some((t.cycle_state, progress(&t)));

        // The comparison.
        let mut diffs: Vec<String> = Vec::new();
        let mut count = 0i64;
        if let Some(n) = &note {
            diffs.push(format!("run: {n}"));
            count += 1;
        }
        for (field, text) in &p.drift {
            *self.month.fields.entry(field).or_default() += 1;
            diffs.push(format!("between deliveries {field}: {text}"));
            count += 1;
        }
        if !p.drift.is_empty() {
            self.stats.resyncs += 1;
            self.month.drifts += 1;
        }
        let ours = self.record();
        let (rec, known) = compare_record_split(&r1.bytes, &ours);
        for k in known {
            *self.month.known.entry(k).or_default() += 1;
        }
        self.month.record_fields += ours.marks.len() as u64;
        self.stats.record_fields += rec.len() as u64;
        for (field, text) in &rec {
            *self.month.fields.entry(field).or_default() += 1;
            diffs.push(format!("record {field}: {text}"));
            count += 1;
        }
        if ours.foreign_refs != 0 {
            diffs.push(format!("record: {} references to objects of ours", ours.foreign_refs));
            count += 1;
        }
        // Trip lists: every live source.
        let h = self.host.inner().clone();
        let live_lots: HashSet<u64> = h.lots().into_iter().collect();
        let live_conns: HashSet<u64> = h.all_connections().into_iter().collect();
        let mut trip_compared = 0u64;
        let mut trip_bad = 0u64;
        for s in self.host.sources() {
            let alive = match s {
                Source::Lot(l) => live_lots.contains(&l),
                Source::Connection(c) => live_conns.contains(&c),
            };
            if !alive {
                continue;
            }
            let Some(exp) = self.host.expected_trips(s) else {
                continue;
            };
            let orig = h.trip_list(s);
            trip_compared += 1;
            if trip_bytes(&orig) != trip_bytes(&exp) {
                if let Some(k) = known_trip_difference(&orig, &exp) {
                    *self.month.known.entry(k).or_default() += 1;
                    continue;
                }
                trip_bad += 1;
                if diffs.len() < self.max_diffs * 4 {
                    let (what, (x, z)) = match s {
                        Source::Lot(l) => ("lot", h.lot_location(l)),
                        Source::Connection(c) => ("connection", h.connection_cell(c)),
                    };
                    let i = (0..orig.len().max(exp.len()))
                        .find(|&i| orig.get(i) != exp.get(i))
                        .unwrap_or(0);
                    let show = |v: &[TripData]| {
                        if orig.len() != exp.len() {
                            // Different lengths: the whole lists (compact).
                            let all: Vec<String> = v
                                .iter()
                                .take(10)
                                .map(|t| {
                                    format!(
                                        "{}x t{} s{} f{} st{}",
                                        t.count, t.trip_type, t.strategy, t.flags, t.status
                                    )
                                })
                                .collect();
                            format!("[{}{}]", all.join(", "), if v.len() > 10 { ", ..." } else { "" })
                        } else {
                            v.get(i).map_or("none".to_string(), show_trip)
                        }
                    };
                    let pre = if orig.len() != exp.len() {
                        let p = self.host.trips_pre.borrow().get(&s).cloned().unwrap_or_default();
                        format!(" (before the delivery {})", show(&p))
                    } else {
                        String::new()
                    };
                    diffs.push(format!(
                        "trips of {what} ({x}, {z}){}: orig {} trips, ours {}; trip {i}: orig {} ours {}{pre}",
                        if self.host.written_trips(s) {
                            ""
                        } else {
                            " (not written by ours)"
                        },
                        orig.len(),
                        exp.len(),
                        show(&orig),
                        show(&exp)
                    ));
                }
            }
        }
        count += trip_bad as i64;
        // Holder properties.
        let mut prop_compared = 0u64;
        let mut prop_bad = 0u64;
        for k in self.host.holder_props() {
            let exp = self.host.expected_prop(k);
            let orig = h.holder_property(k.0, k.1);
            prop_compared += 1;
            if orig != exp {
                prop_bad += 1;
                if diffs.len() < self.max_diffs * 4 {
                    diffs.push(format!(
                        "holder {:#x} property 0x{:08X}: orig {} ours {}",
                        k.0,
                        k.1,
                        show_variant(&orig),
                        show_variant(&exp)
                    ));
                }
            }
        }
        count += prop_bad as i64;
        // Grids.
        let tokens = Self::grid_tokens(&r1);
        let names = self.grid_names();
        let mut cells = 0u64;
        let mut cells_bad = 0u64;
        for g in self.host.compare_grids(live, &tokens, 4) {
            cells += g.cells;
            let name = names.get(&g.token).copied().unwrap_or("grid");
            if g.geometry_differs {
                diffs.push(format!("grid {name}: geometry differs"));
                cells_bad += 1;
                continue;
            }
            if g.differing > 0 {
                cells_bad += g.differing;
                let first: Vec<String> = g
                    .first
                    .iter()
                    .map(|(x, z, o, b)| format!("({x}, {z}) orig {o} ours {b}"))
                    .collect();
                diffs.push(format!(
                    "grid {name}: {} of {} tracts differ: {}",
                    g.differing,
                    g.cells,
                    first.join(", ")
                ));
            }
        }
        if cells_bad > 0 {
            self.grids_synced = false;
        }
        count += cells_bad as i64;

        // Accounting.
        self.stats.compared += 1;
        self.month.compared += 1;
        self.stats.trip_lists += trip_compared;
        self.stats.trip_mismatches += trip_bad;
        self.stats.props += prop_compared;
        self.stats.prop_mismatches += prop_bad;
        self.stats.grid_cells += cells;
        self.stats.grid_mismatches += cells_bad;
        self.month.trip_lists += trip_compared;
        self.month.trip_mismatches += trip_bad;
        self.month.props += prop_compared;
        self.month.prop_mismatches += prop_bad;
        self.month.grid_cells += cells;
        self.month.grid_mismatches += cells_bad;
        let mismatches = count.min(i64::from(i32::MAX)) as i32;
        self.stats.last_mismatches = mismatches;
        if mismatches == 0 {
            self.stats.clean += 1;
            self.month.clean += 1;
        } else {
            if self.month.first.len() < self.max_diffs {
                let room = self.max_diffs - self.month.first.len();
                for x in diffs.iter().take(room) {
                    self.month.first.push(format!("{} {kind}: {x}", date(d.julian)));
                }
            }
            if self.month.detail_lines < self.detail_lines_per_month {
                self.month.detail_lines += 1;
                let shown: Vec<&str> = diffs.iter().take(self.max_diffs).map(|s| s.as_str()).collect();
                let more = if diffs.len() > shown.len() {
                    format!("; (+{} more)", diffs.len() - shown.len())
                } else {
                    String::new()
                };
                lines.push(format!(
                    "traffic shadow {} {kind} (state {} -> {}, progress {}, units {units}): mismatches={mismatches}; {}{more}",
                    date(d.julian),
                    p.from_state,
                    t.cycle_state,
                    progress(&t),
                    shown.join("; "),
                ));
            }
        }
        // A difference left in the copy is resynced at the next delivery
        // (its record check); its writes are forgotten.
        self.host.forget_writes();
        self.month_line(d, &mut lines);
        Outcome { mismatches, lines }
    }

    /// The grid tokens by name (the copy's state).
    fn grid_names(&self) -> HashMap<u64, &'static str> {
        fn tok<T: GridElem>(g: &Option<Rc<dyn TractGrid<T>>>) -> u64 {
            match g.as_ref().map(|g| g.save_ref()) {
                Some(RefValue::Host { token, .. }) => token,
                _ => 0,
            }
        }
        const VOLUME: [&str; 6] = ["volume 0", "volume 1", "volume 2", "volume 3", "volume 4", "volume 5"];
        let mut m: HashMap<u64, &'static str> = HashMap::new();
        let names: Vec<(u64, &'static str)> = self.copy.read_state(|s| {
            let mut v = vec![
                (tok(&s.air), "air-polluting"),
                (tok(&s.peak_volume), "peak volume"),
                (tok(&s.commercial), "commercial"),
                (tok(&s.commercial_work), "commercial work"),
                (tok(&s.unused), "unused"),
                (tok(&s.trip_length), "trip length"),
                (tok(&s.float_grid), "float"),
                (tok(&s.congestion), "congestion"),
                (tok(&s.peak_ratio), "peak ratio"),
            ];
            for (i, g) in s.volume.iter().enumerate() {
                v.push((tok(g), VOLUME.get(i).copied().unwrap_or("volume")));
            }
            v
        });
        for (t, n) in names {
            m.insert(t, n);
        }
        let (mode, id) = {
            let net = self.copy.network();
            let n = net.state();
            (tok(&n.switch_mode), tok(&n.switch_id))
        };
        m.insert(mode, "switch mode");
        m.insert(id, "switch ID");
        m.remove(&0);
        m
    }

    /// After a month delivery: the month line, and the sums restart.
    fn month_line(&mut self, d: Delivery, lines: &mut Vec<String>) {
        if d.msg_type != msg::MONTH {
            return;
        }
        let m = std::mem::take(&mut self.month);
        let mismatches = m.compared - m.clean;
        let head = if m.incomplete > 0 && m.compared == 0 {
            "INCOMPLETE".to_string()
        } else if mismatches == 0 {
            "clean".to_string()
        } else {
            format!("mismatches={mismatches}")
        };
        let fields: Vec<String> = m.fields.iter().map(|(k, v)| format!("{k} x{v}")).collect();
        let reasons: Vec<String> = m.reasons.iter().map(|(k, v)| format!("{k} x{v}")).collect();
        let kept = self.host.kept.get();
        let reasons = if reasons.is_empty() {
            String::new()
        } else {
            format!(" ({})", reasons.join(", "))
        };
        let field_list = if fields.is_empty() {
            String::new()
        } else {
            format!(" ({})", fields.join(", "))
        };
        let first = if m.first.is_empty() {
            "none".to_string()
        } else {
            m.first.join(" | ")
        };
        let known = if m.known.is_empty() {
            String::new()
        } else {
            let k: Vec<String> = m.known.iter().map(|(k, v)| format!("{k} x{v}")).collect();
            format!("; known differences (not counted): {}", k.join(", "))
        };
        lines.push(format!(
            "traffic shadow month to {}: {head}; deliveries={} (day {}, month {}, other {}) compared={} clean={} \
             incomplete={}{reasons}; record fields compared={} mismatched={}{field_list}; trip lists compared={} \
             mismatched={}; holder properties compared={} mismatched={}; grid tracts compared={} mismatched={}{known}; \
             between deliveries: resyncs={}; ours: units={} run-ons={} (list ends {}) day steps={}, kept posts={} history={} \
             incomes={} network costs={} budget items={} accidents={}; first differences: {first}",
            date(d.julian),
            m.deliveries,
            m.days,
            m.months,
            m.others,
            m.compared,
            m.clean,
            m.incomplete,
            m.record_fields,
            m.fields.values().map(|&v| u64::from(v)).sum::<u64>(),
            m.trip_lists,
            m.trip_mismatches,
            m.props,
            m.prop_mismatches,
            m.grid_cells,
            m.grid_mismatches,
            m.drifts,
            m.units,
            m.run_ons,
            m.list_ends,
            m.day_steps,
            kept.posts,
            kept.history,
            kept.incomes,
            kept.network_costs,
            kept.budget_items,
            kept.accidents,
        ));
    }
}

#[cfg(test)]
mod known_difference_tests {
    use super::*;

    /// A status-0 trip differing only in its destination is the original's
    /// suspended search (known); a routed one, or one differing elsewhere,
    /// is a mismatch.
    #[test]
    fn a_status_0_destination_is_known_only_alone() {
        let ours = TripData {
            count: 7,
            status: super::super::status::SEARCH,
            dest_x: 18,
            dest_z: 40,
            trip_length: 4.0,
            ..TripData::default()
        };
        let mut orig = ours.clone();
        orig.dest_x = 28;
        orig.dest_z = 37;
        let k = known_trip_difference(std::slice::from_ref(&orig), std::slice::from_ref(&ours));
        assert!(k.is_some_and(|k| k.contains("suspended")));
        let mut longer = orig.clone();
        longer.trip_length = 5.0;
        assert_eq!(known_trip_difference(&[longer], std::slice::from_ref(&ours)), None);
        let mut routed = ours.clone();
        routed.status = super::super::status::ROUTED;
        let mut routed_orig = routed.clone();
        routed_orig.dest_x = 28;
        assert_eq!(known_trip_difference(&[routed_orig], &[routed]), None);
    }

    /// TS65: the original's suspended return leg leaves the new destination
    /// and flags bit 0 cleared on a status-0 trip (known); bit 0 set on the
    /// original's side only, or any other flag, is a mismatch.
    #[test]
    fn a_suspended_return_leg_clears_bit_0() {
        let ours = TripData {
            count: 15,
            status: super::super::status::SEARCH,
            flags: 1,
            dest_x: 18,
            dest_z: 40,
            ..TripData::default()
        };
        let mut orig = ours.clone();
        orig.flags = 0;
        orig.dest_x = 28;
        orig.dest_z = 37;
        let k = known_trip_difference(std::slice::from_ref(&orig), std::slice::from_ref(&ours));
        assert!(k.is_some_and(|k| k.contains("TS65")));
        assert_eq!(
            known_trip_difference(std::slice::from_ref(&ours), std::slice::from_ref(&orig)),
            None
        );
        let mut other = orig.clone();
        other.flags = 4;
        assert_eq!(known_trip_difference(&[other], std::slice::from_ref(&ours)), None);
        // ReRouteTrips' trips (status 1) too; a status change is a mismatch.
        let (mut o1, mut b1) = (orig.clone(), ours.clone());
        o1.status = super::super::status::REROUTE;
        b1.status = super::super::status::REROUTE;
        assert!(known_trip_difference(std::slice::from_ref(&o1), std::slice::from_ref(&b1)).is_some());
        assert_eq!(
            known_trip_difference(std::slice::from_ref(&o1), std::slice::from_ref(&ours)),
            None
        );
    }
}
