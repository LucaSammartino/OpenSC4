// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The aura grid shadow (fc4.h "aura shadow"): with the ORIGINAL aura
//! simulator running, FreeCity4's pipeline (states 1-17, docs/specs/aura.md
//! §6) is run once per original cycle on the original's own record and
//! compared with it cell by cell (tooling; no spec behaviour).
//!
//! # When
//!
//! The only externally observable cycle boundary is the original's
//! cycle-completed message (`0x2A09B21A`, §11.1), posted (queued) at the end
//! of state 17; when it is delivered the original is idle in state 0 until
//! the next month. The plugin captures the original's record there (its own
//! `cIGZSerializable::Write` into memory) and calls
//! [`fc4_aura_shadow_cycle`]; at city load (PostCityInit) it captures the
//! loaded record as the first baseline ([`fc4_aura_shadow_baseline`]).
//!
//! # What
//!
//! Record `k` (end of cycle `k`) holds the base grid after state 14 and the
//! transient grid after state 17's decay. Our cycle `k` starts from record
//! `k`'s grids (raw park, department landmark grids, building effects:
//! the original's current state) with the base and transient grids, and the
//! best / worst locations, of record `k − 1`, and reads every other input
//! live through a read-only host table (the grids of the other simulators,
//! the lots, land value, funding, tax rates, ordinances, tuning). Nothing is
//! written to the game: our grids are core copies, and posts, history,
//! disasters, strikes, agents, subscriptions and the registry are swallowed.
//!
//! Compared, tract by tract: the base grid and the aura grid; also the park
//! and landmark maps and the transient grid (a transient tract that differs
//! got a transient effect during the cycle, an input we cannot see: aura
//! differences there are counted apart); the mayor rating, the best and
//! worst rating and location, and the hotspots. Not compared: the riot roll
//! (the original's generator is seeded from the wall clock, §9), and the
//! posts, history and disasters (writes).
//!
//! Limits: the inputs are read when the cycle has ended, not at the moment
//! of each state (a city that changed during the cycle's few days shows as
//! a difference); the tax-change baseline (§6.5.2) is the tax rates of our
//! first compared cycle, not of the original's first cycle after the load.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::aura::{msg, AuraHost, AuraMessage, AuraRng, AuraSim, LotInfo, LotIterator, Strike};
use fc4_core::grid::{CityDims, GridElem, GridProvider, SimGrid, TractGeometry, TractGrid, IID_SIM_GRID};
use fc4_core::julian::civil_from_julian;
use fc4_core::property::PropertyHolder;
use fc4_core::rng::FixedSeeds;
use fc4_core::stream::{ByteReader, InStream, RefValue, SaveObject};

use crate::aura::{fc4_aura_host, CHost};
use crate::guard;
use crate::landvalue::fc4_grid_geometry;
use crate::stream::fc4_bytes_sink;

/// One reference of a captured record (fc4.h `fc4_aura_capture_ref`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_aura_capture_ref {
    /// The object as a `cISC4SimGrid` (IID `0xC9BD939A`), borrowed; 0 = not a grid.
    pub grid: u64,
    /// `GetGZCLSID()`.
    pub class_id: u32,
    pub reserved: u32,
}

/// A captured record (fc4.h `fc4_aura_capture`): the bytes the original's
/// `Write` produced (primitives little-endian, references as `u32 objectID`
/// (index + 1, 0 = null) then `u32 classID`) and its references in write
/// order.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_aura_capture {
    pub struct_size: u32,
    /// Non-zero when `Write` returned true and the stream stayed good.
    pub ok: u32,
    pub bytes: *const u8,
    pub len: usize,
    pub refs: *const fc4_aura_capture_ref,
    pub ref_count: usize,
}

/// Totals since creation (fc4.h `fc4_aura_shadow_stats`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_aura_shadow_stats {
    pub struct_size: u32,
    /// Cycle ends seen (compared or not).
    pub cycles: u32,
    pub compared: u32,
    /// ... compared without a base, aura or mayor-rating difference.
    pub clean: u32,
    /// ... not compared (no baseline yet, a capture or our cycle failed).
    pub incomplete: u32,
    /// The last cycle's base + aura + rating differences (-1: not compared).
    pub last_mismatches: i32,
    /// Cycles whose mayor rating differed.
    pub rating_mismatches: u32,
    pub reserved: u32,
    /// Tracts compared per grid, and the differing base / aura tracts.
    pub tracts: u64,
    pub base_mismatches: u64,
    pub aura_mismatches: u64,
    /// ... aura differences in tracts that got a transient effect.
    pub aura_transient: u64,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_aura_capture_ref>() == 16);
    assert!(std::mem::size_of::<fc4_aura_capture>() == if cfg!(target_pointer_width = "32") { 24 } else { 40 });
    assert!(std::mem::size_of::<fc4_aura_shadow_stats>() == 64);
};

/// The most pipeline steps one shadow cycle takes (17 states, one call per
/// department in state 3).
const MAX_STEPS: u32 = 100_000;
/// The largest grid side copied (tracts).
const MAX_SIDE: i32 = 4096;

// ---- the shadow's host --------------------------------------------------------------------------------

/// The shadow's `AuraHost`: reads go to the read-only C table, grids are core
/// copies, writes stay here.
struct ShadowHost {
    c: CHost,
    dims: CityDims,
    /// Inputs that answered "none" in the current cycle, in first-ask order.
    missing: RefCell<Vec<&'static str>>,
    /// Calls swallowed (posts, history, disasters, strikes, plumbing).
    swallowed: Cell<u32>,
    /// Whether the current cycle reached state 17 (its post).
    completed: Cell<bool>,
}

impl ShadowHost {
    fn swallow(&self) {
        self.swallowed.set(self.swallowed.get().wrapping_add(1));
    }

    fn note<T>(&self, name: &'static str, v: Option<T>) -> Option<T> {
        if v.is_none() {
            let mut m = self.missing.borrow_mut();
            if !m.contains(&name) {
                m.push(name);
            }
        }
        v
    }

    fn geometry(&self, token: u64) -> TractGeometry {
        let api = &self.c.0.grids;
        let mut g = fc4_grid_geometry::default();
        if let Some(f) = api.geometry {
            // SAFETY: valid callback (fc4_aura_shadow_create's precondition).
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

    /// A core copy of a live game grid (x outer).
    fn copy<T: GridElem>(&self, token: u64) -> Option<Rc<SimGrid<T>>> {
        let api = &self.c.0.grids;
        let get = api.get?;
        let geom = self.geometry(token);
        if !(1..=MAX_SIDE).contains(&geom.count_x) || !(1..=MAX_SIDE).contains(&geom.count_z) {
            return None;
        }
        let mut values = Vec::with_capacity((geom.count_x * geom.count_z) as usize);
        for x in 0..geom.count_x {
            for z in 0..geom.count_z {
                // SAFETY: valid callback on a live game grid.
                values.push(T::from_f64(unsafe { get(api.user, token, T::CLSID, x, z) }));
            }
        }
        Some(SimGrid::from_values(geom, &values))
    }

    fn resolve<T: GridElem>(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<T>>> {
        match *r {
            RefValue::Host { token, class_id } if token != 0 && class_id == T::CLSID => {
                self.copy::<T>(token).map(|g| g as Rc<dyn TractGrid<T>>)
            }
            _ => None,
        }
    }

    fn create<T: GridElem>(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<T>>> {
        Some(SimGrid::<T>::create(&self.dims, instance, tract_size) as Rc<dyn TractGrid<T>>)
    }
}

impl GridProvider<i8> for ShadowHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<i8>>> {
        self.create(instance, tract_size)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<i8>>> {
        self.resolve(r)
    }
}

impl GridProvider<i16> for ShadowHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<i16>>> {
        self.create(instance, tract_size)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<i16>>> {
        self.resolve(r)
    }
}

const POLLUTION_NAMES: [&str; 4] = ["air pollution", "water pollution", "garbage", "radiation"];

impl AuraHost for ShadowHost {
    fn cell_width(&self) -> (f32, f32) {
        self.c.cell_width()
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        self.c.position_to_cell(x, z)
    }
    fn sim_date_number(&self) -> u32 {
        self.c.sim_date_number()
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.note("tuning", self.c.tuning())
    }
    fn publish_base_aura(&self, _: [i32; 3]) {
        self.swallow();
    }
    fn lot_count(&self) -> u32 {
        self.c.lot_count()
    }
    fn lot_iterator(&self) -> Box<dyn LotIterator> {
        self.c.lot_iterator()
    }
    fn lot_at(&self, x: i32, z: i32) -> Option<LotInfo> {
        self.c.lot_at(x, z)
    }
    fn land_value_type(&self, x: i32, z: i32) -> Option<i32> {
        self.c.land_value_type(x, z)
    }
    fn occupant_has_property(&self, occupant: u64, prop: u32) -> bool {
        self.c.occupant_has_property(occupant, prop)
    }
    fn occupant_sint32s(&self, occupant: u64, prop: u32) -> Option<Vec<i32>> {
        self.c.occupant_sint32s(occupant, prop)
    }
    fn occupant_type(&self, occupant: u64) -> u32 {
        self.c.occupant_type(occupant)
    }
    fn occupant_position(&self, occupant: u64) -> [f32; 3] {
        self.c.occupant_position(occupant)
    }
    fn budget_item_department(&self, occupant: u64, purpose: u32) -> Option<u32> {
        self.c.budget_item_department(occupant, purpose)
    }
    fn department_funding(&self, dept: u32) -> Option<f32> {
        self.c.department_funding(dept)
    }
    fn tax_rate(&self, index: i32) -> f32 {
        self.c.tax_rate(index)
    }
    fn cumulative_offset(&self, prop: u32, index: u32) -> i32 {
        self.c.cumulative_offset(prop, index)
    }
    fn pollution_grid(&self, kind: i32) -> Option<Rc<dyn TractGrid<i16>>> {
        let name = POLLUTION_NAMES.get(kind as usize).copied().unwrap_or("pollution");
        self.note(name, self.c.pollution_grid(kind))
    }
    fn school_grade_map(&self) -> Option<Rc<dyn TractGrid<f32>>> {
        self.note("school grades", self.c.school_grade_map())
    }
    fn hospital_grade_map(&self) -> Option<Rc<dyn TractGrid<f32>>> {
        self.note("hospital grades", self.c.hospital_grade_map())
    }
    fn crime_grid(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        self.note("crime", self.c.crime_grid())
    }
    fn trip_length_map(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        self.note("trip length", self.c.trip_length_map())
    }
    fn congestion_map(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        self.note("congestion", self.c.congestion_map())
    }
    fn population_grids(&self) -> Option<[Rc<dyn TractGrid<u16>>; 3]> {
        self.note("population grids", self.c.population_grids())
    }
    fn record_history(&self, _: u32, _: i32) {
        self.swallow();
    }
    fn attempt_disaster_start(&self, _: u32, _: i32, _: i32) {
        self.swallow();
    }
    fn end_strike(&self, _: Strike) {
        self.swallow();
    }
    fn post(&self, msg_type: u32) {
        if msg_type == msg::AURA_CYCLE_COMPLETED {
            self.completed.set(true);
        }
        self.swallow();
    }
    fn limit_timer_start(&self, _: u32) {}
    fn limit_timer_expired(&self) -> bool {
        false
    }
    fn subscribe(&self, _: &[u32]) {
        self.swallow();
    }
    fn unsubscribe(&self, _: &[u32]) {
        self.swallow();
    }
    fn add_agent(&self, _: u32, _: &str, _: u32) {
        self.swallow();
    }
    fn remove_agent(&self) {
        self.swallow();
    }
    fn registry_add(&self) -> bool {
        self.swallow();
        false
    }
    fn registry_remove(&self) {
        self.swallow();
    }
}

// ---- the captured record ---------------------------------------------------------------------------

/// A captured record as a core input stream: a reference is answered with
/// its grid token when the reader asks for a grid.
struct CaptureReader<'a> {
    r: ByteReader<'a>,
    refs: &'a [fc4_aura_capture_ref],
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
        if iid == IID_SIM_GRID && e.grid != 0 {
            return Some(RefValue::Host {
                token: e.grid,
                class_id,
            });
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
struct Capture {
    bytes: Vec<u8>,
    refs: Vec<fc4_aura_capture_ref>,
}

/// Copies a capture; `Err` with the reason when it is not usable.
unsafe fn capture_of(c: *const fc4_aura_capture) -> Result<Capture, &'static str> {
    let Some(c) = c.as_ref() else {
        return Err("no capture");
    };
    if (c.struct_size as usize) < std::mem::size_of::<fc4_aura_capture>() {
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

/// Opaque handle (fc4.h `fc4_aura_shadow`).
#[repr(C)]
pub struct fc4_aura_shadow {
    _private: [u8; 0],
}

/// A grid's values (x outer).
#[derive(Clone)]
struct Values<T> {
    geom: TractGeometry,
    values: Vec<T>,
}

fn values_of<T: GridElem>(g: &Rc<dyn TractGrid<T>>) -> Values<T> {
    let geom = g.geometry();
    let (nx, nz) = (geom.count_x.clamp(0, MAX_SIDE), geom.count_z.clamp(0, MAX_SIDE));
    let mut values = Vec::with_capacity((nx * nz) as usize);
    for x in 0..nx {
        for z in 0..nz {
            values.push(g.get(x, z));
        }
    }
    Values { geom, values }
}

/// What a cycle starts from: the previous record's base and transient grids
/// and best / worst locations.
struct Baseline {
    base: Values<i8>,
    transient: Values<i8>,
    max_location: (i32, i32),
    min_location: (i32, i32),
}

/// A parsed record: a never-initialised core object holding copies.
struct Parsed {
    sim: Rc<AuraSim>,
}

impl Parsed {
    fn baseline(&self) -> Option<Baseline> {
        let base = self.sim.base_grid()?;
        let transient = self.sim.transient_grid()?;
        let d = self.sim.data();
        Some(Baseline {
            base: values_of(&base),
            transient: values_of(&transient),
            max_location: d.max_location,
            min_location: d.min_location,
        })
    }
}

/// A grid comparison: tracts, differences, max |d|, the first differing
/// tracts (x, z, orig, ours).
#[derive(Default)]
struct Diff {
    tracts: u64,
    mismatches: u64,
    max_abs: i64,
    first: Vec<(i32, i32, i64, i64)>,
    /// Differences in tracts of `mask` (the transient-touched tracts).
    masked: u64,
    geometry_differs: bool,
}

fn compare<T: GridElem>(
    orig: &Option<Rc<dyn TractGrid<T>>>,
    ours: &Option<Rc<dyn TractGrid<T>>>,
    keep: usize,
    mask: Option<&BTreeSet<(i32, i32)>>,
) -> Option<Diff> {
    let (o, u) = (orig.as_ref()?, ours.as_ref()?);
    let (go, gu) = (o.geometry(), u.geometry());
    let mut d = Diff::default();
    if (go.count_x, go.count_z) != (gu.count_x, gu.count_z) {
        d.geometry_differs = true;
        return Some(d);
    }
    for x in 0..go.count_x.clamp(0, MAX_SIDE) {
        for z in 0..go.count_z.clamp(0, MAX_SIDE) {
            d.tracts += 1;
            let (a, b) = (o.get(x, z).to_f64() as i64, u.get(x, z).to_f64() as i64);
            if a != b {
                d.mismatches += 1;
                d.max_abs = d.max_abs.max((a - b).abs());
                if mask.is_some_and(|m| m.contains(&(x, z))) {
                    d.masked += 1;
                }
                if d.first.len() < keep {
                    d.first.push((x, z, a, b));
                }
            }
        }
    }
    Some(d)
}

fn show(name: &str, d: &Option<Diff>) -> String {
    match d {
        None => format!("{name} missing"),
        Some(d) if d.geometry_differs => format!("{name} geometry differs"),
        Some(d) => format!("{name} mismatches={} max|d|={}", d.mismatches, d.max_abs),
    }
}

fn first_cells(name: &str, d: &Option<Diff>) -> Option<String> {
    let d = d.as_ref()?;
    if d.first.is_empty() {
        return None;
    }
    let cells: Vec<String> = d
        .first
        .iter()
        .map(|(x, z, a, b)| format!("({x},{z}) {a}/{b}"))
        .collect();
    Some(format!("{name} {}", cells.join(" ")))
}

fn date(julian: i32) -> String {
    let d = civil_from_julian(julian);
    format!("{:04}-{:02}-{:02}", d.year, d.month, d.day)
}

struct Shadow {
    host: Rc<ShadowHost>,
    max_cells: usize,
    /// Our simulator, kept across cycles (tuning, tax baseline, slice step).
    ours: Option<Rc<AuraSim>>,
    baseline: Option<Baseline>,
    stats: fc4_aura_shadow_stats,
}

fn fixed_rng() -> Box<AuraRng> {
    Box::new(AuraRng::new(&FixedSeeds {
        wall_clock: 1,
        process_clock: 1,
    }))
}

impl Shadow {
    fn parse(&self, c: &Capture) -> Result<Parsed, &'static str> {
        let sim = AuraSim::new(self.host.clone() as Rc<dyn AuraHost>, 0, fixed_rng());
        let mut r = CaptureReader {
            r: ByteReader::new(&c.bytes),
            refs: &c.refs,
        };
        if !sim.read(&mut r) {
            return Err("the original's record did not parse");
        }
        if sim.base_grid().is_none() || sim.aura_grid().is_none() || sim.transient_grid().is_none() {
            return Err("the original's record names no base, aura or transient grid");
        }
        Ok(Parsed { sim })
    }

    fn baseline(&mut self, c: Result<Capture, &'static str>, lines: &mut Vec<String>) -> bool {
        self.ours = None; // a new city: a new simulator
        self.baseline = None;
        let parsed = c.and_then(|c| self.parse(&c));
        match parsed.as_ref().ok().and_then(Parsed::baseline) {
            Some(b) => {
                let data = b.base.values.iter().filter(|&&v| v != -128).count();
                let nonzero = b.transient.values.iter().filter(|&&v| v != 0).count();
                lines.push(format!(
                    "aura shadow: baseline from the original's record at city load: base {data} of {} tracts with \
                     data, transient {nonzero} nonzero",
                    b.base.values.len()
                ));
                self.baseline = Some(b);
                true
            }
            None => {
                let why = parsed.err().unwrap_or("the record has no base or transient grid");
                lines.push(format!(
                    "aura shadow: no baseline at city load ({why}); the first cycle only takes one"
                ));
                false
            }
        }
    }

    /// Our simulator for this record: created and initialised once (Read
    /// before Init, as on a load), else loaded with the record's grids.
    fn ours_for(&mut self, c: &Capture) -> Result<Rc<AuraSim>, &'static str> {
        let fresh = self.ours.is_none();
        let sim = match &self.ours {
            Some(s) => s.clone(),
            None => AuraSim::new(self.host.clone() as Rc<dyn AuraHost>, 0, fixed_rng()),
        };
        let mut r = CaptureReader {
            r: ByteReader::new(&c.bytes),
            refs: &c.refs,
        };
        if !sim.read(&mut r) {
            return Err("our Read of the original's record failed");
        }
        if fresh {
            sim.init();
            self.ours = Some(sim.clone());
        }
        Ok(sim)
    }

    fn cycle(&mut self, c: Result<Capture, &'static str>, julian: i32, lines: &mut Vec<String>) -> i32 {
        self.stats.cycles = self.stats.cycles.wrapping_add(1);
        let head = format!("aura shadow {} cycle {}:", date(julian), self.stats.cycles);
        let r = self.run(c, &head, lines);
        if r < 0 {
            self.stats.incomplete = self.stats.incomplete.wrapping_add(1);
        }
        self.stats.last_mismatches = r;
        r
    }

    fn run(&mut self, c: Result<Capture, &'static str>, head: &str, lines: &mut Vec<String>) -> i32 {
        let c = match c {
            Ok(c) => c,
            Err(why) => {
                self.baseline = None;
                lines.push(format!(
                    "{head} INCOMPLETE ({why}); the next cycle only takes a baseline"
                ));
                return -1;
            }
        };
        let orig = match self.parse(&c) {
            Ok(p) => p,
            Err(why) => {
                self.baseline = None;
                lines.push(format!(
                    "{head} INCOMPLETE ({why}); the next cycle only takes a baseline"
                ));
                return -1;
            }
        };
        let Some(prev) = self.baseline.take() else {
            self.baseline = orig.baseline();
            lines.push(format!("{head} baseline taken (no previous record); nothing compared"));
            return -1;
        };
        self.baseline = orig.baseline();
        let ours = match self.ours_for(&c) {
            Ok(s) => s,
            Err(why) => {
                lines.push(format!("{head} INCOMPLETE ({why})"));
                return -1;
            }
        };
        // The previous record's base and transient grids.
        let (Some(base), Some(transient)) = (ours.base_grid(), ours.transient_grid()) else {
            lines.push(format!("{head} INCOMPLETE (our base or transient grid is missing)"));
            return -1;
        };
        for (g, v) in [(&base, &prev.base), (&transient, &prev.transient)] {
            let geo = g.geometry();
            if (geo.count_x, geo.count_z) != (v.geom.count_x, v.geom.count_z) {
                lines.push(format!(
                    "{head} INCOMPLETE (the grid geometry changed since the previous record)"
                ));
                return -1;
            }
            let mut i = 0;
            for x in 0..v.geom.count_x.clamp(0, MAX_SIDE) {
                for z in 0..v.geom.count_z.clamp(0, MAX_SIDE) {
                    g.set(x, z, v.values[i]);
                    i += 1;
                }
            }
        }
        {
            let mut d = ours.data_mut();
            d.max_location = prev.max_location;
            d.min_location = prev.min_location;
            d.department_cursor = None;
            d.low_tracts.clear();
            d.state = 1;
        }
        self.host.missing.borrow_mut().clear();
        self.host.completed.set(false);
        let mut steps = 0;
        while ours.data().state != 0 && steps < MAX_STEPS {
            ours.do_message(&AuraMessage {
                msg_type: msg::SIM_IDLE,
                data3: 1000,
                ..AuraMessage::default()
            });
            steps += 1;
        }
        if !self.host.completed.get() {
            let state = ours.data().state;
            lines.push(format!(
                "{head} INCOMPLETE (our cycle ended without completing: state {state} after {steps} steps; the \
                 original completed)"
            ));
            return -1;
        }

        // Transient tracts that got an effect during the cycle.
        let mut touched = BTreeSet::new();
        if let (Some(o), Some(u)) = (orig.sim.transient_grid(), ours.transient_grid()) {
            let geo = o.geometry();
            if (geo.count_x, geo.count_z) == (u.geometry().count_x, u.geometry().count_z) {
                for x in 0..geo.count_x.clamp(0, MAX_SIDE) {
                    for z in 0..geo.count_z.clamp(0, MAX_SIDE) {
                        if o.get(x, z) != u.get(x, z) {
                            touched.insert((x, z));
                        }
                    }
                }
            }
        }
        let keep = self.max_cells;
        let base_d = compare(&orig.sim.base_grid(), &ours.base_grid(), keep, None);
        let aura_d = compare(&orig.sim.aura_grid(), &ours.aura_grid(), keep, Some(&touched));
        let park_d = compare(&orig.sim.park_map(), &ours.park_map(), 0, None);
        let landmark_d = compare(&orig.sim.landmark_map(), &ours.landmark_map(), 0, None);
        let (od, ud) = (orig.sim.data().clone(), ours.data().clone());

        let base_n = base_d.as_ref().map_or(0, |d| d.mismatches);
        let aura_n = aura_d.as_ref().map_or(0, |d| d.mismatches);
        let rating_differs = od.mayor_rating != ud.mayor_rating;
        let total = base_n + aura_n + u64::from(rating_differs);
        let st = &mut self.stats;
        st.compared = st.compared.wrapping_add(1);
        if total == 0 {
            st.clean = st.clean.wrapping_add(1);
        }
        if rating_differs {
            st.rating_mismatches = st.rating_mismatches.wrapping_add(1);
        }
        st.tracts = st.tracts.wrapping_add(base_d.as_ref().map_or(0, |d| d.tracts));
        st.base_mismatches = st.base_mismatches.wrapping_add(base_n);
        st.aura_mismatches = st.aura_mismatches.wrapping_add(aura_n);
        st.aura_transient = st.aura_transient.wrapping_add(aura_d.as_ref().map_or(0, |d| d.masked));

        let tracts = base_d.as_ref().map_or(0, |d| d.tracts);
        let masked = aura_d.as_ref().map_or(0, |d| d.masked);
        let same_spots = od.hotspots == ud.hotspots;
        let missing = self.host.missing.borrow();
        let mut line = format!(
            "{head} tracts={tracts}; {}; {} ({masked} in the {} tracts that got a transient effect during the \
             cycle); {}; {}; mayor rating orig/ours={}/{}; max orig/ours={}@({},{})/{}@({},{}); min orig/ours=\
             {}@({},{})/{}@({},{}); hotspots orig/ours={}/{}{}",
            show("base", &base_d),
            show("aura", &aura_d),
            touched.len(),
            show("park map", &park_d),
            show("landmark map", &landmark_d),
            od.mayor_rating,
            ud.mayor_rating,
            od.max_rating,
            od.max_location.0,
            od.max_location.1,
            ud.max_rating,
            ud.max_location.0,
            ud.max_location.1,
            od.min_rating,
            od.min_location.0,
            od.min_location.1,
            ud.min_rating,
            ud.min_location.0,
            ud.min_location.1,
            od.hotspots.len(),
            ud.hotspots.len(),
            if same_spots { " (same)" } else { " (differ)" },
        );
        let firsts: Vec<String> = [first_cells("base", &base_d), first_cells("aura", &aura_d)]
            .into_iter()
            .flatten()
            .collect();
        if !firsts.is_empty() {
            line += &format!("; first differing (x,z) orig/ours: {}", firsts.join("; "));
        }
        line += "; riot roll not compared (wall-clock RNG)";
        if missing.is_empty() {
            line += "; inputs: all read";
        } else {
            line += &format!("; inputs not available (neutral): {}", missing.join(", "));
        }
        lines.push(line);
        total.min(i32::MAX as u64) as i32
    }
}

fn with_shadow<R>(s: *mut fc4_aura_shadow, fallback: R, f: impl FnOnce(&mut Shadow) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_aura_shadow*.
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

/// Creates a shadow over a read-only host table: only its read callbacks
/// (city, simulator date, tuning, lots, land value, occupant reads,
/// funding, tax rates, ordinances, input grids) and the grid table's
/// `geometry` / `get` are called; the write callbacks may be NULL. `cells_x`
/// / `cells_z`: the city's cell counts (our scratch grids' geometry).
/// `max_cells`: the differing tracts listed per grid and line (0 → 4, at
/// most 64). NULL for a short table or a city outside 1..=4096 cells.
#[no_mangle]
pub unsafe extern "C" fn fc4_aura_shadow_create(
    host: *const fc4_aura_host,
    cells_x: i32,
    cells_z: i32,
    max_cells: u32,
) -> *mut fc4_aura_shadow {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h)
            if (h.struct_size as usize) >= std::mem::size_of::<fc4_aura_host>()
                && h.grids.complete()
                && (1..=MAX_SIDE).contains(&cells_x)
                && (1..=MAX_SIDE).contains(&cells_z) =>
        {
            let c = CHost(*h);
            let (cell_width_x, cell_width_z) = c.cell_width();
            let host = Rc::new(ShadowHost {
                c,
                dims: CityDims {
                    cells_x,
                    cells_z,
                    cell_width_x,
                    cell_width_z,
                },
                missing: RefCell::new(Vec::new()),
                swallowed: Cell::new(0),
                completed: Cell::new(false),
            });
            let max_cells = match max_cells {
                0 => 4,
                n => n.min(64) as usize,
            };
            Box::into_raw(Box::new(Shadow {
                host,
                max_cells,
                ours: None,
                baseline: None,
                stats: fc4_aura_shadow_stats {
                    struct_size: std::mem::size_of::<fc4_aura_shadow_stats>() as u32,
                    last_mismatches: -1,
                    ..Default::default()
                },
            })) as *mut fc4_aura_shadow
        }
        _ => std::ptr::null_mut(),
    })
}

/// Destroys a shadow (NULL is ignored). It holds no game reference.
#[no_mangle]
pub unsafe extern "C" fn fc4_aura_shadow_destroy(s: *mut fc4_aura_shadow) {
    if s.is_null() {
        return;
    }
    guard((), || drop(Box::from_raw(s as *mut Shadow)));
}

/// City load: the original's record becomes the baseline of the first
/// cycle (and our simulator starts afresh). One line to `sink`. True when a
/// baseline was taken.
#[no_mangle]
pub unsafe extern "C" fn fc4_aura_shadow_baseline(
    s: *mut fc4_aura_shadow,
    capture: *const fc4_aura_capture,
    sink: fc4_bytes_sink,
    ctx: *mut c_void,
) -> bool {
    let c = capture_of(capture);
    let mut lines = Vec::new();
    let r = with_shadow(s, false, |sh| sh.baseline(c, &mut lines));
    emit(&lines, sink, ctx);
    r
}

/// The original's cycle-completed message (`julian`: the current date, for
/// the line): our cycle on its record and the previous baseline, compared;
/// the record becomes the next baseline. One line to `sink`. Returns the
/// base + aura + mayor-rating differences, -1 when nothing was compared.
#[no_mangle]
pub unsafe extern "C" fn fc4_aura_shadow_cycle(
    s: *mut fc4_aura_shadow,
    capture: *const fc4_aura_capture,
    julian: i32,
    sink: fc4_bytes_sink,
    ctx: *mut c_void,
) -> i32 {
    let c = capture_of(capture);
    let mut lines = Vec::new();
    let r = with_shadow(s, -1, |sh| sh.cycle(c, julian, &mut lines));
    emit(&lines, sink, ctx);
    r
}

/// Totals since creation.
#[no_mangle]
pub unsafe extern "C" fn fc4_aura_shadow_get_stats(s: *mut fc4_aura_shadow, out: *mut fc4_aura_shadow_stats) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_shadow(s, false, |sh| {
        *o = sh.stats;
        true
    })
}
