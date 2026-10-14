// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The flammability grid shadow (fc4.h "flammability shadow"): with the
//! ORIGINAL flammability simulator running, FreeCity4's simulator (the core
//! code our subsystem runs) follows the original's messages on the
//! original's live inputs and is compared with the original's grids and
//! scalars at the end of every cycle (tooling; no spec behaviour; section
//! references point at docs/specs/flammability.md).
//!
//! # When
//!
//! At the end of each of the original's cycles: SimCycleComplete
//! (`0x08F415FE`, §9) is posted by `EndOfCycle` (§6.6), after the average and
//! before the hotspot list is rebuilt, and delivered (queued) later, when the
//! average and the hotspots of the cycle that just ended stand and stay until
//! the next cycle ends. Every tract was visited exactly once in that cycle
//! (§6.5), so the whole adjusted grid is that cycle's. A day or a month is not
//! a cycle boundary: at a day boundary most of the grid belongs to the
//! previous cycle, and the average and the hotspots exist only per cycle.
//!
//! # What
//!
//! A mirror simulator (ours, on its own grids, never on the game's) gets
//! every message the original subscribes to (§5: the three occupant
//! messages, season change, ordinance updated, condition changed) from the
//! plugin, in the same delivery as the original's, and reads the same live
//! inputs through a read-only host table. *[compat]* (§5, FL4): one send
//! delivers to the subscribers in subscription order, so the original, which
//! subscribes in its `Init`, handles each message just before the later
//! mirror, and the handlers read nothing the subscribers in between write.
//! It does not run the daily refresh:
//! the original's start tract is a `rand()` draw (§7) that cannot be read.
//! At each cycle end:
//!
//! - the **raw grid** must equal the mirror's exactly (the raw grid changes
//!   only through the occupant messages, §6.3);
//! - the **adjusted grid**: every tract must hold a value the cycle could
//!   have written: its daily value (§6.5) from the raw values, multipliers
//!   and watered states it had in the cycle, a value the mirror's occupant
//!   updates wrote, or its previous value. A tract whose inputs did not
//!   change in the cycle must hold its daily value exactly (or its previous
//!   value: tracts visited on the wrap day before the previous comparison,
//!   counted as "unvisited", at most 2 × tractsPerDay; *[compat]* §9, FL1:
//!   SimCycleComplete is delivered at the next frame's message-server tick,
//!   by when at most the rest of the wrap day and, at speed 2, one more day
//!   are visited, so the bound is right for shipped tuning);
//! - the **average** must lie in the range those values and the zoning (at
//!   both ends of the cycle) allow; exact when nothing changed;
//! - the **multiplier**, bitwise (*[compat]* §6.4, FL2: only the
//!   constructor, `Read` and the SeasonChange / OrdinanceUpdated branch write
//!   it, so the mirror takes the original's at the baseline and recomputes it
//!   on the same two messages);
//! - the **hotspots**: the hot tracts in visiting order are a rotation of the
//!   fixed prime-step order (§6.5); every rotation is tried with our
//!   `ComputeHotspotList` (§6.7) and one must give the original's list. When
//!   tracts may or may not have been hot (their inputs changed), a miss is
//!   reported but not counted;
//! - the getters, on the mirror resynchronised to the original: slot 6 per
//!   cell, slot 7, slot 8.
//!
//! Then the mirror takes the original's grids, multiplier, average and
//! hotspots, so one difference is reported once.
//!
//! **The watered state is the one at the visit** *[compat]* (§6.5, FL5,
//! FL6): the day visit applies the no-water factor (1.25 shipped) from
//! plumbing slot 11 as it answers that day, and that answer changes only
//! when a plumbing pass ends and swaps its published grid in. Every shadow
//! call (each forwarded message, including the plumbing pass end
//! WaterMapChanged `0x6946DC34`, and the cycle end) first polls the plumbing
//! simulator's `GetWateredGrid()` (slot 7, host `watered_grid`): the
//! published grid's object alternates at every pass, so a changed pointer
//! means a pass ended; the watered states are then read again and the
//! tracts whose state changed are marked. A marked tract (or one whose
//! state differs between the two cycle ends) may hold its daily value under
//! either state; every other tract is checked under its one state, as
//! before. Two passes between polls would bring the pointer back unseen;
//! the pass-end message gives one poll per pass. With `watered_grid` NULL
//! only the two cycle ends are compared.
//!
//! A tract size other than 1 is reported `INCOMPLETE` (§2, FL3: no original
//! behaviour to match; the implementer's choice stands).
//!
//! Not compared: the fire roll (a clock-seeded generator), the hotspot
//! shuffle and the iterator start (`rand()`), the random lot, the candidate
//! list and the zoned sum and count (no getter, not saved, §10).

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::flammability::{
    compute_hotspots, daily_value, msg, CellIterator, CellRect, CrtRand, FlammabilityHost, FlammabilityRandom,
    FlammabilitySim, Hotspot, Message, ADJUSTED_GRID_INSTANCE, RAW_GRID_INSTANCE,
};
use fc4_core::grid::{GridProvider, TractGrid};
use fc4_core::property::PropertyHolder;
use fc4_core::simgrid::GridGeometry;
use fc4_core::stream::RefValue;
use fc4_core::variant::Variant;

use crate::flammability::{fc4_flammability_host, fc4_flammability_message, CHost};
use crate::guard;
use crate::landvalue::fc4_grid_geometry;
use crate::landvalue_shadow::{date, emit, Diff};
use crate::stream::fc4_bytes_sink;

/// Opaque handle (fc4.h `fc4_flammability_shadow`).
#[repr(C)]
pub struct fc4_flammability_shadow {
    _private: [u8; 0],
}

/// One hotspot of the original's record (§10).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_flammability_shadow_hotspot {
    pub x: i32,
    pub z: i32,
    pub weight: i32,
}

/// What the plugin read from the original at its cycle end (fc4.h
/// `fc4_flammability_original`). Grids are x outer.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_flammability_original {
    pub struct_size: u32,
    /// Non-zero: the record (the original's own Write) parsed (version 2).
    pub has_record: u32,
    /// The original simulator's identity (a new one restarts the shadow).
    pub simulator: u64,
    /// The record's adjusted and raw grids: geometry and every tract.
    pub geometry: fc4_grid_geometry,
    pub raw_geometry: fc4_grid_geometry,
    pub adjusted: *const u8,
    pub raw: *const u8,
    /// The record's hotspots, in list order.
    pub hotspots: *const fc4_flammability_shadow_hotspot,
    /// `IsCellFlammable` (slot 6) per city cell, x outer; NULL = not read.
    pub flammable: *const u8,
    pub hotspot_count: u32,
    /// The record's multiplier and average.
    pub global_multiplier: f32,
    pub average: u8,
    /// `GetAverageFlammability` (slot 8).
    pub average_getter: u8,
    /// `GetFlammabilityHotspot` (slot 7): its result and outputs.
    pub hotspot_found: u8,
    pub hotspot_value: u8,
    pub hotspot_x: i32,
    pub hotspot_z: i32,
    /// The city's cell counts (the `flammable` array).
    pub cells_x: i32,
    pub cells_z: i32,
}

/// Counters (fc4.h `fc4_flammability_shadow_stats`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_flammability_shadow_stats {
    pub struct_size: u32,
    /// Cycle calls, those that compared, and those without a difference.
    pub cycles: u32,
    pub compared: u32,
    pub clean: u32,
    /// The last call's difference count; -1 = not compared.
    pub last_mismatches: i32,
    /// The last compared cycle, by kind.
    pub raw_mismatches: u32,
    pub adjusted_mismatches: u32,
    pub unvisited: u32,
    pub scalar_mismatches: u32,
    pub hotspot_mismatches: u32,
    pub getter_mismatches: u32,
    /// Messages mirrored since the shadow started.
    pub events: u32,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_flammability_shadow_hotspot>() == 12);
    assert!(std::mem::size_of::<fc4_flammability_shadow_stats>() == 48);
    assert!(
        std::mem::size_of::<fc4_flammability_original>() == if cfg!(target_pointer_width = "32") { 128 } else { 144 }
    );
};

const DEFAULT_CELLS: usize = 4;
const MAX_CELLS: usize = 64;
/// The largest tract count per axis accepted from the original.
const MAX_COUNT: i32 = 4096;
/// Distance checks allowed per cycle for the hotspot rotations.
const HOTSPOT_BUDGET: u64 = 60_000_000;

fn geometry_of(g: &fc4_grid_geometry) -> GridGeometry {
    GridGeometry {
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

// ---- local grids and the read-only host ---------------------------------------------------------------

/// A grid of the mirror that logs every write.
struct RecGrid {
    geom: GridGeometry,
    values: RefCell<Vec<u8>>,
    log: RefCell<Vec<(usize, u8)>>,
}

impl TractGrid<u8> for RecGrid {
    fn geometry(&self) -> GridGeometry {
        self.geom
    }
    fn get(&self, tx: i32, tz: i32) -> u8 {
        self.geom
            .index(tx, tz)
            .and_then(|i| self.values.borrow().get(i).copied())
            .unwrap_or(0)
    }
    fn set(&self, tx: i32, tz: i32, v: u8) {
        if let Some(i) = self.geom.index(tx, tz) {
            if let Some(slot) = self.values.borrow_mut().get_mut(i) {
                *slot = v;
                self.log.borrow_mut().push((i, v));
            }
        }
    }
    fn save_ref(&self) -> RefValue {
        RefValue::Null
    }
}

/// The game's read callbacks; the grids are the mirror's own, and the
/// history, the fire, the random lot and the posts are never forwarded.
struct ShadowHost {
    c: CHost,
    geom: Cell<GridGeometry>,
    created: RefCell<Vec<(u32, Rc<RecGrid>)>>,
    blocked: Cell<u32>,
}

impl GridProvider<u8> for ShadowHost {
    fn create_grid(&self, instance: u32, _tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        let geom = self.geom.get();
        let g = Rc::new(RecGrid {
            geom,
            values: RefCell::new(vec![0; geom.len()]),
            log: RefCell::new(Vec::new()),
        });
        self.created.borrow_mut().push((instance, g.clone()));
        Some(g)
    }
    fn resolve_grid(&self, _r: &RefValue) -> Option<Rc<dyn TractGrid<u8>>> {
        None
    }
}

impl FlammabilityHost for ShadowHost {
    fn city_cell_counts(&self) -> (i32, i32) {
        self.c.city_cell_counts()
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        self.c.position_to_cell(x, z)
    }
    fn is_tutorial(&self) -> bool {
        // No fire roll in the mirror (it never ends a cycle either).
        true
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.c.tuning()
    }
    fn occupant_type(&self, occ: u64) -> u32 {
        self.c.occupant_type(occ)
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant> {
        self.c.occupant_property(occ, id)
    }
    fn occupant_bounds(&self, occ: u64) -> CellRect {
        self.c.occupant_bounds(occ)
    }
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        self.c.occupant_position(occ)
    }
    fn occupants_in_cells(&self, rect: CellRect) -> Vec<u64> {
        self.c.occupants_in_cells(rect)
    }
    fn occupant_lot(&self, occ: u64) -> Option<u64> {
        self.c.occupant_lot(occ)
    }
    fn lot_condition(&self, lot: u64) -> u8 {
        self.c.lot_condition(lot)
    }
    fn lot_building(&self, lot: u64) -> Option<u64> {
        self.c.lot_building(lot)
    }
    fn random_lot(&self) -> Option<u64> {
        self.blocked.set(self.blocked.get() + 1);
        None
    }
    fn cumulative_factor(&self, prop: u32, index: u32) -> Option<f32> {
        self.c.cumulative_factor(prop, index)
    }
    fn current_season(&self) -> Option<i32> {
        self.c.current_season()
    }
    fn is_cell_watered(&self, x: i32, z: i32) -> Option<bool> {
        self.c.is_cell_watered(x, z)
    }
    fn zone_type(&self, x: i32, z: i32) -> Option<u32> {
        self.c.zone_type(x, z)
    }
    fn record_history(&self, _history_type: u32, _value: i32) {
        self.blocked.set(self.blocked.get() + 1);
    }
    fn ignite_cell(&self, _x: u32, _z: u32, _use_probability: bool, _teleport: bool, _fire_iid: u32) -> bool {
        self.blocked.set(self.blocked.get() + 1);
        false
    }
    fn set_fire_cause(&self, _cause: u32) {
        self.blocked.set(self.blocked.get() + 1);
    }
    fn post(&self, _msg_type: u32) {
        self.blocked.set(self.blocked.get() + 1);
    }
    fn grid_shutdown(&self, _grid: &Rc<dyn TractGrid<u8>>) {}
}

/// The mirror never rolls (no fire roll: `is_tutorial` is true).
struct NoDraw;

impl FlammabilityRandom for NoDraw {
    fn range_f64(&mut self, a: f64, _b: f64) -> f64 {
        a
    }
}

// ---- the original's results ---------------------------------------------------------------------

struct Orig {
    simulator: u64,
    geom: GridGeometry,
    adjusted: Vec<u8>,
    raw: Vec<u8>,
    hotspots: Vec<Hotspot>,
    g: f32,
    average: u8,
    average_getter: u8,
    hotspot_getter: Option<(u8, i32, i32)>,
    flammable: Option<(i32, i32, Vec<u8>)>,
}

unsafe fn orig_of(p: *const fc4_flammability_original) -> Result<Orig, String> {
    let Some(o) = p.as_ref() else {
        return Err("nothing read from the original".into());
    };
    if (o.struct_size as usize) < std::mem::size_of::<fc4_flammability_original>() {
        return Err("bad fc4_flammability_original".into());
    }
    if o.has_record == 0 {
        return Err("the original's record (its own Write) could not be read".into());
    }
    let geom = geometry_of(&o.geometry);
    let raw_geom = geometry_of(&o.raw_geometry);
    let ok = |g: &GridGeometry| (1..=MAX_COUNT).contains(&g.count_x) && (1..=MAX_COUNT).contains(&g.count_z);
    if !ok(&geom) || !ok(&raw_geom) {
        return Err(format!(
            "the original's grids have geometries {}x{} and {}x{}",
            geom.count_x, geom.count_z, raw_geom.count_x, raw_geom.count_z
        ));
    }
    if geom.count_x != raw_geom.count_x || geom.count_z != raw_geom.count_z {
        return Err("the original's adjusted and raw grids differ in geometry".into());
    }
    // [choice] (§2, FL3): the original creates its grids with S = 1 and never
    // changes it, so another size needs a save it did not write; there is no
    // original behaviour to match, and the shadow reports it INCOMPLETE.
    if geom.tract_size != 1 || raw_geom.tract_size != 1 {
        return Err(format!(
            "the original's grids have tract size {}/{}; FreeCity4 supports only 1 (§14.4)",
            geom.tract_size, raw_geom.tract_size
        ));
    }
    if o.adjusted.is_null() || o.raw.is_null() {
        return Err("the original's grids were not read".into());
    }
    if o.hotspots.is_null() && o.hotspot_count != 0 {
        return Err("the original's hotspots were not read".into());
    }
    let n = geom.len();
    let hotspots = if o.hotspot_count == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(o.hotspots, o.hotspot_count as usize)
            .iter()
            .map(|h| Hotspot::new(h.x, h.z, h.weight))
            .collect()
    };
    let flammable = (!o.flammable.is_null() && o.cells_x > 0 && o.cells_z > 0).then(|| {
        let len = o.cells_x as usize * o.cells_z as usize;
        (
            o.cells_x,
            o.cells_z,
            std::slice::from_raw_parts(o.flammable, len).to_vec(),
        )
    });
    Ok(Orig {
        simulator: o.simulator,
        geom,
        adjusted: std::slice::from_raw_parts(o.adjusted, n).to_vec(),
        raw: std::slice::from_raw_parts(o.raw, n).to_vec(),
        hotspots,
        g: o.global_multiplier,
        average: o.average,
        average_getter: o.average_getter,
        hotspot_getter: (o.hotspot_found != 0).then_some((o.hotspot_value, o.hotspot_x, o.hotspot_z)),
        flammable,
    })
}

// ---- hotspot rotations ----------------------------------------------------------------------------

/// The inverse of `a` modulo `m` (`a`, `m` coprime).
fn mod_inverse(a: i64, m: i64) -> Option<i64> {
    let (mut r0, mut r1) = (a.rem_euclid(m), m);
    let (mut s0, mut s1) = (1i64, 0i64);
    while r1 != 0 {
        let q = r0 / r1;
        (r0, r1) = (r1, r0 - q * r1);
        (s0, s1) = (s1, s0 - q * s1);
    }
    (r0 == 1).then(|| s0.rem_euclid(m))
}

enum Search {
    /// The rotation (of how many) that gave the original's list, and that list.
    Found(usize, usize),
    /// No rotation did; the first rotation's list.
    Missed(Vec<Hotspot>),
    /// The work budget ran out.
    Budget,
}

/// Tries every rotation of the hot tracts (sorted by visiting position) as
/// the cycle's candidate list.
fn search_rotations(hot: &[Hotspot], orig: &[Hotspot], radius: i32, min_weight: i32, budget: &mut u64) -> Search {
    let h = hot.len();
    if h == 0 {
        let ours = compute_hotspots(&[], radius, min_weight);
        return if ours == orig {
            Search::Found(0, 0)
        } else {
            Search::Missed(ours)
        };
    }
    // One rotation costs about h × (hotspots before the pruning) checks.
    let unpruned = compute_hotspots(hot, radius, i32::MIN).len().max(1) as u64;
    let cost = h as u64 * unpruned;
    let mut first = None;
    let mut cands = Vec::with_capacity(h);
    for k in 0..h {
        if *budget < cost {
            return Search::Budget;
        }
        *budget -= cost;
        cands.clear();
        cands.extend_from_slice(&hot[k..]);
        cands.extend_from_slice(&hot[..k]);
        let ours = compute_hotspots(&cands, radius, min_weight);
        if ours == orig {
            return Search::Found(k, h);
        }
        if first.is_none() {
            first = Some(ours);
        }
    }
    Search::Missed(first.unwrap_or_default())
}

fn list(h: &[Hotspot], max: usize) -> String {
    let mut s: Vec<String> = h
        .iter()
        .take(max)
        .map(|h| format!("({},{},{})", h.x, h.z, h.weight))
        .collect();
    if h.len() > max {
        s.push("...".into());
    }
    format!("[{}]", s.join(" "))
}

// ---- the shadow ---------------------------------------------------------------------------------

struct Mirror {
    simulator: u64,
    geom: GridGeometry,
    sim: Rc<FlammabilitySim>,
    adjusted: Rc<RecGrid>,
    raw: Rc<RecGrid>,
    /// The original at the previous cycle end: adjusted, raw; and the inputs
    /// then: watered, zoned.
    a_prev: Vec<u8>,
    r_prev: Vec<u8>,
    w_prev: Vec<bool>,
    z_prev: Vec<bool>,
    /// The multipliers of this cycle (the first is the previous cycle end's).
    gs: Vec<f32>,
    events: u32,
    /// FL5/FL6 (§6.5): the plumbing simulator's published watered grid as
    /// last polled (`None`: not polled), the watered states read then, the
    /// tracts whose state changed at a pass seen since the previous cycle
    /// end, and how many passes were seen.
    watered_grid: Option<u64>,
    w_seen: Vec<bool>,
    w_changed: Vec<bool>,
    passes: u32,
}

struct Shadow {
    host: Rc<ShadowHost>,
    max_cells: usize,
    mirror: Option<Mirror>,
    stats: fc4_flammability_shadow_stats,
}

/// A tract's possible daily values (§6.5) over the given inputs.
fn daily_values(rs: &[u8], gs: &[f32], ws: &[bool], no_water: f32) -> BTreeSet<u8> {
    let mut out = BTreeSet::new();
    for &r in rs.iter().filter(|&&r| r > 0) {
        for &g in gs {
            for &w in ws {
                out.insert(daily_value(r, g, w, no_water));
            }
        }
    }
    out
}

impl Shadow {
    fn watered(&self, x: i32, z: i32) -> bool {
        // [choice] §6.5: no plumbing simulator reads as not watered.
        self.host.c.is_cell_watered(x, z).unwrap_or(false)
    }

    fn zoned(&self, x: i32, z: i32) -> bool {
        self.host.c.zone_type(x, z).unwrap_or(0) != 0
    }

    /// The watered state per tract now (x outer).
    fn watered_states(&self, g: &GridGeometry) -> Vec<bool> {
        let mut w = Vec::with_capacity(g.len());
        for x in 0..g.count_x {
            for z in 0..g.count_z {
                w.push(self.watered(x, z));
            }
        }
        w
    }

    /// FL5/FL6 (§6.5): when the plumbing simulator's published grid is
    /// another object than at the last poll, a pass ended: the watered
    /// states are read again and the tracts that changed are marked.
    fn poll(&mut self) {
        let Some(p) = self.host.c.watered_grid() else {
            return;
        };
        let Some(geom) = self
            .mirror
            .as_ref()
            .filter(|m| m.watered_grid != Some(p))
            .map(|m| m.geom)
        else {
            return;
        };
        let w = self.watered_states(&geom);
        let m = self.mirror.as_mut().expect("mirror");
        for (i, (&now, &seen)) in w.iter().zip(m.w_seen.iter()).enumerate() {
            if now != seen {
                m.w_changed[i] = true;
            }
        }
        m.w_seen = w;
        m.watered_grid = Some(p);
        m.passes = m.passes.wrapping_add(1);
    }

    /// The inputs per tract now: watered, zoned.
    fn inputs(&self, g: &GridGeometry) -> (Vec<bool>, Vec<bool>) {
        let n = g.len();
        let (mut w, mut z) = (Vec::with_capacity(n), Vec::with_capacity(n));
        for x in 0..g.count_x {
            for zz in 0..g.count_z {
                w.push(self.watered(x, zz));
                z.push(self.zoned(x, zz));
            }
        }
        (w, z)
    }

    /// A new mirror on the original's state.
    fn start(&self, o: &Orig) -> Result<Mirror, String> {
        let h = &self.host;
        h.geom.set(o.geom);
        h.created.borrow_mut().clear();
        let sim = FlammabilitySim::new(h.clone(), 0, Box::new(NoDraw));
        sim.init();
        let find = |instance: u32| {
            h.created
                .borrow()
                .iter()
                .find(|(i, _)| *i == instance)
                .map(|(_, g)| g.clone())
        };
        let (Some(adjusted), Some(raw)) = (find(ADJUSTED_GRID_INSTANCE), find(RAW_GRID_INSTANCE)) else {
            return Err("our simulator made no grids".into());
        };
        h.created.borrow_mut().clear();
        let mut m = Mirror {
            simulator: o.simulator,
            geom: o.geom,
            sim,
            adjusted,
            raw,
            a_prev: Vec::new(),
            r_prev: Vec::new(),
            w_prev: Vec::new(),
            z_prev: Vec::new(),
            gs: Vec::new(),
            events: 0,
            watered_grid: h.c.watered_grid(),
            w_seen: Vec::new(),
            w_changed: Vec::new(),
            passes: 0,
        };
        let (w, z) = self.inputs(&o.geom);
        Self::resync(&mut m, o, w, z);
        Ok(m)
    }

    /// The mirror takes the original's state; the cycle window restarts.
    fn resync(m: &mut Mirror, o: &Orig, w: Vec<bool>, z: Vec<bool>) {
        m.adjusted.values.borrow_mut().copy_from_slice(&o.adjusted);
        m.raw.values.borrow_mut().copy_from_slice(&o.raw);
        m.adjusted.log.borrow_mut().clear();
        m.raw.log.borrow_mut().clear();
        {
            let mut d = m.sim.data_mut();
            d.global_multiplier = o.g;
            d.average = o.average;
            d.hotspots = o.hotspots.clone();
            d.candidates.clear();
        }
        m.a_prev = o.adjusted.clone();
        m.r_prev = o.raw.clone();
        m.w_changed = vec![false; w.len()];
        m.w_seen = w.clone();
        m.w_prev = w;
        m.z_prev = z;
        m.gs = vec![o.g];
        m.events = 0;
        m.passes = 0;
    }

    fn message(&mut self, m: &fc4_flammability_message) -> bool {
        self.poll();
        let Some(mirror) = self.mirror.as_mut() else {
            return false;
        };
        let handled = [
            msg::INSERT_OCCUPANT,
            msg::REMOVE_OCCUPANT,
            msg::CHANGE_OCCUPANT,
            msg::SEASON_CHANGE,
            msg::ORDINANCE_UPDATED,
            msg::CONDITION_CHANGED,
        ];
        if !handled.contains(&m.msg_type) {
            return false;
        }
        let opt = |t: u64| (t != 0).then_some(t);
        mirror.sim.do_message(&Message {
            msg_type: m.msg_type,
            occupant: opt(m.occupant),
            lot: opt(m.lot),
        });
        if m.msg_type == msg::SEASON_CHANGE || m.msg_type == msg::ORDINANCE_UPDATED {
            let g = mirror.sim.snapshot().global_multiplier;
            if !mirror.gs.iter().any(|x| x.to_bits() == g.to_bits()) {
                mirror.gs.push(g);
            }
        }
        mirror.events = mirror.events.wrapping_add(1);
        self.stats.events = self.stats.events.wrapping_add(1);
        true
    }

    fn cycle(&mut self, o: Result<Orig, String>, julian: i32) -> (i32, String) {
        self.stats.cycles = self.stats.cycles.wrapping_add(1);
        self.stats.last_mismatches = -1;
        let head = format!("flammability shadow {} cycle {}", date(julian), self.stats.cycles);
        let o = match o {
            Ok(o) => o,
            Err(why) => {
                self.mirror = None;
                return (
                    -1,
                    format!("{head}: INCOMPLETE ({why}); not compared, the next cycle is a new baseline"),
                );
            }
        };
        let restart = match &self.mirror {
            None => Some("first cycle seen"),
            Some(m) if m.simulator != o.simulator => Some("a new original simulator"),
            Some(m) if m.geom != o.geom => Some("the grid geometry changed"),
            Some(_) => None,
        };
        if let Some(why) = restart {
            self.mirror = None;
            return match self.start(&o) {
                Ok(m) => {
                    let d = m.sim.snapshot();
                    self.mirror = Some(m);
                    (
                        -1,
                        format!(
                            "{head}: baseline ({why}): our simulator takes the original's grids ({}x{} tracts, \
                             tractsPerDay={}), multiplier {}, average {}, {} hotspot(s); compared from the next \
                             cycle",
                            o.geom.count_x,
                            o.geom.count_z,
                            d.tracts_per_day,
                            o.g,
                            o.average,
                            o.hotspots.len()
                        ),
                    )
                }
                Err(why) => (-1, format!("{head}: INCOMPLETE ({why}); not compared")),
            };
        }
        self.poll();
        let mut m = self.mirror.take().expect("mirror");
        let (n, line, w, z) = self.compare(&mut m, &o, &head);
        Self::resync(&mut m, &o, w, z);
        let line = format!("{line}{}", self.getters(&m, &o, n));
        self.mirror = Some(m);
        (self.stats.last_mismatches, line)
    }

    /// The comparison before the resync; returns the difference count so far,
    /// the line so far, and the inputs now (watered, zoned).
    fn compare(&mut self, m: &mut Mirror, o: &Orig, head: &str) -> (usize, String, Vec<bool>, Vec<bool>) {
        let g = m.geom;
        let n = g.len();
        let mc = self.max_cells;
        let snap = m.sim.snapshot();
        let tu = &snap.tuning;
        let tpd = snap.tracts_per_day.max(0) as usize;
        let (w_end, z_end) = self.inputs(&g);

        // Raw: exact.
        let ours_raw = m.raw.values.borrow().clone();
        let mut raw = Diff::new(n);
        // The cycle's raw values and update writes per tract.
        let mut rs: BTreeMap<usize, Vec<u8>> = BTreeMap::new();
        for &(i, v) in m.raw.log.borrow().iter() {
            let e = rs.entry(i).or_insert_with(|| vec![m.r_prev[i]]);
            if !e.contains(&v) {
                e.push(v);
            }
        }
        let mut us: BTreeMap<usize, Vec<u8>> = BTreeMap::new();
        for &(i, v) in m.adjusted.log.borrow().iter() {
            let e = us.entry(i).or_default();
            if !e.contains(&v) {
                e.push(v);
            }
        }

        let mut adjusted = Diff::new(n);
        let (mut exact, mut explained) = (0usize, 0usize);
        let mut unvisited = Diff::new(n);
        // Average: (visit value min, max) of tracts zoned at both ends
        // (mandatory) and of tracts zoned at one end (optional).
        let mut mandatory: Vec<(u32, u32)> = Vec::new();
        let mut optional: Vec<(u32, u32)> = Vec::new();
        // Hotspots: definite hot tracts and ambiguous ones (index, end, start, adjusted).
        let mut hot_def: Vec<usize> = Vec::new();
        let mut hot_amb: Vec<(usize, [bool; 3])> = Vec::new();
        let threshold = tu.hotspot_threshold;
        let no_water = tu.no_water_multiplier;
        let g_start = m.gs[0];
        let g_end = *m.gs.last().unwrap_or(&g_start);

        for x in 0..g.count_x {
            for z in 0..g.count_z {
                let i = x as usize * g.count_z as usize + z as usize;
                let (o_raw, a_end, a_prev, r_prev) = (o.raw[i], o.adjusted[i], m.a_prev[i], m.r_prev[i]);
                if o_raw != ours_raw[i] {
                    raw.note(x, z, o_raw, ours_raw[i], f64::from(o_raw.abs_diff(ours_raw[i])), mc);
                }
                let r_set: Vec<u8> = rs.get(&i).cloned().unwrap_or_else(|| vec![r_prev]);
                let u_set: &[u8] = us.get(&i).map_or(&[], |v| v.as_slice());
                // [compat] (§6.5, FL5/FL6): both watered states when it
                // changed at a pass in the cycle or between the cycle ends.
                let w_set: Vec<bool> = if m.w_prev[i] == w_end[i] && !m.w_changed[i] {
                    vec![w_end[i]]
                } else {
                    vec![false, true]
                };
                let certain = r_set.len() == 1 && m.gs.len() == 1 && w_set.len() == 1 && u_set.is_empty();
                let dv = daily_values(&r_set, &m.gs, &w_set, no_water);
                // The cycle's visit values.
                let mut visit: BTreeSet<u8> = dv.clone();
                if r_set.contains(&0) {
                    visit.insert(0);
                }
                if certain {
                    let r = r_set[0];
                    let expected = if r > 0 {
                        daily_value(r, g_start, w_set[0], no_water)
                    } else {
                        a_prev
                    };
                    if a_end == expected {
                        exact += 1;
                    } else if a_end == a_prev {
                        unvisited.note(x, z, a_end, expected, f64::from(a_end.abs_diff(expected)), mc);
                        if r > 0 {
                            visit.insert(a_prev);
                        }
                    } else {
                        adjusted.note(x, z, a_end, expected, f64::from(a_end.abs_diff(expected)), mc);
                    }
                } else {
                    if dv.contains(&a_end) || u_set.contains(&a_end) || a_end == a_prev {
                        explained += 1;
                    } else {
                        let nearest = dv
                            .iter()
                            .chain(u_set.iter())
                            .chain(std::iter::once(&a_prev))
                            .min_by_key(|v| v.abs_diff(a_end))
                            .copied()
                            .unwrap_or(a_prev);
                        adjusted.note(
                            x,
                            z,
                            a_end,
                            format!("{nearest}?"),
                            f64::from(a_end.abs_diff(nearest)),
                            mc,
                        );
                    }
                    if r_prev > 0 {
                        visit.insert(a_prev);
                    }
                }
                let (vmin, vmax) = (
                    u32::from(*visit.first().unwrap_or(&0)),
                    u32::from(*visit.last().unwrap_or(&0)),
                );
                match (m.z_prev[i], z_end[i]) {
                    (true, true) => mandatory.push((vmin, vmax)),
                    (false, false) => {}
                    _ => optional.push((vmin, vmax)),
                }
                let hot_all = visit.iter().all(|&v| i32::from(v) > threshold);
                let hot_any = visit.iter().any(|&v| i32::from(v) > threshold);
                if hot_all {
                    hot_def.push(i);
                } else if hot_any {
                    let at = |r: u8, g: f32, w: bool| r > 0 && i32::from(daily_value(r, g, w, no_water)) > threshold;
                    hot_amb.push((
                        i,
                        [
                            at(o_raw, g_end, w_end[i]),
                            at(r_prev, g_start, m.w_prev[i]),
                            i32::from(a_end) > threshold,
                        ],
                    ));
                }
            }
        }

        // The average's range (§6.6: the integer quotient of the zoned sum).
        let ratio = |pick_max: bool| -> u32 {
            let val = |p: &(u32, u32)| if pick_max { p.1 } else { p.0 };
            let (mut sum, mut count) = (
                mandatory.iter().map(val).map(u64::from).sum::<u64>(),
                mandatory.len() as u64,
            );
            if !pick_max && count == 0 {
                return 0; // no tract counted at all: 0
            }
            let mut opt: Vec<u32> = optional.iter().map(val).collect();
            if pick_max {
                opt.sort_unstable_by(|a, b| b.cmp(a));
            } else {
                opt.sort_unstable();
            }
            for v in opt {
                let better = count == 0
                    || if pick_max {
                        u64::from(v) * count > sum
                    } else {
                        u64::from(v) * count < sum
                    };
                if !better {
                    break;
                }
                sum += u64::from(v);
                count += 1;
            }
            sum.checked_div(count).unwrap_or(0) as u32
        };
        let (lo, hi) = (ratio(false), ratio(true));
        let average_ok = (lo..=hi).contains(&u32::from(o.average));

        // The multiplier.
        let g_ours = snap.global_multiplier;
        let g_ok = g_ours.to_bits() == o.g.to_bits();

        // Hotspots.
        let mut it = CellIterator::new(0, 0, g.count_x - 1, g.count_z - 1);
        it.reset(false, &mut CrtRand::default());
        let count = i64::from(it.count);
        let inv = if count as usize == n {
            mod_inverse(i64::from(it.step), count)
        } else {
            None
        };
        let (hot_text, hot_counted) = match inv {
            None => (
                format!(
                    "hotspots orig={} not compared (the iterator does not visit each tract once: count {} of {n})",
                    o.hotspots.len(),
                    it.count
                ),
                0,
            ),
            Some(inv) => {
                let cx = i64::from(g.count_x);
                let cz = g.count_z as usize;
                let pos = |i: usize| -> i64 {
                    let (x, z) = ((i / cz) as i64, (i % cz) as i64);
                    ((z * cx + x) % count) * inv % count
                };
                let ordered = |set: &[usize]| -> Vec<Hotspot> {
                    let mut v: Vec<(i64, usize)> = set.iter().map(|&i| (pos(i), i)).collect();
                    v.sort_unstable();
                    v.into_iter()
                        .map(|(_, i)| Hotspot::new((i / cz) as i32, (i % cz) as i32, 1))
                        .collect()
                };
                let mut tries: Vec<Vec<usize>> = Vec::new();
                // The maybe-hot tracts taken as hot by their end-state daily
                // value, their start-state daily value, their adjusted value.
                for pick in 0..3 {
                    let mut s = hot_def.clone();
                    s.extend(hot_amb.iter().filter(|a| a.1[pick]).map(|a| a.0));
                    s.sort_unstable();
                    if !tries.contains(&s) {
                        tries.push(s);
                    }
                    if hot_amb.is_empty() {
                        break;
                    }
                }
                let mut budget = HOTSPOT_BUDGET;
                let mut outcome = None;
                let mut first_miss = None;
                let mut out_of_budget = false;
                for s in &tries {
                    match search_rotations(
                        &ordered(s),
                        &o.hotspots,
                        tu.hotspot_radius,
                        tu.hotspot_min_weight,
                        &mut budget,
                    ) {
                        Search::Found(k, h) => {
                            outcome = Some((k, h));
                            break;
                        }
                        Search::Missed(l) => {
                            first_miss.get_or_insert(l);
                        }
                        Search::Budget => out_of_budget = true,
                    }
                }
                let amb = hot_amb.len();
                match outcome {
                    Some((k, h)) => (
                        format!(
                            "hotspots orig/ours={}/{} same (rotation {k} of {h} hot tracts{})",
                            o.hotspots.len(),
                            o.hotspots.len(),
                            if amb > 0 {
                                format!(", {amb} maybe hot")
                            } else {
                                String::new()
                            }
                        ),
                        0,
                    ),
                    None if out_of_budget && first_miss.is_none() => (
                        format!(
                            "hotspots orig={} not compared ({} hot tracts: over the work budget)",
                            o.hotspots.len(),
                            hot_def.len() + amb
                        ),
                        0,
                    ),
                    None => {
                        let ours = first_miss.unwrap_or_default();
                        let counted = usize::from(amb == 0 && !out_of_budget);
                        (
                            format!(
                                "hotspots orig/ours={}/{} differ{} (orig {} ours {})",
                                o.hotspots.len(),
                                ours.len(),
                                if counted == 1 {
                                    String::new()
                                } else if out_of_budget {
                                    " (not counted: work budget)".to_string()
                                } else {
                                    format!(" (not counted: {amb} tracts maybe hot)")
                                },
                                list(&o.hotspots, mc),
                                list(&ours, mc)
                            ),
                            counted,
                        )
                    }
                }
            }
        };

        let bound = 2 * tpd;
        let excess = unvisited.mismatched.saturating_sub(bound);
        let scalars = usize::from(!average_ok) + usize::from(!g_ok);
        let mismatches = raw.mismatched + adjusted.mismatched + excess + scalars + hot_counted;

        let s = &mut self.stats;
        s.raw_mismatches = raw.mismatched as u32;
        s.adjusted_mismatches = (adjusted.mismatched + excess) as u32;
        s.unvisited = unvisited.mismatched as u32;
        s.scalar_mismatches = scalars as u32;
        s.hotspot_mismatches = hot_counted as u32;

        let mut line = format!(
            "{head}: tracts={}x{} (S=1) tractsPerDay={tpd}; {}; {} (exact {exact}, explained {explained}, unvisited {} \
             of at most {bound}); average orig/ours={}/{}{}; multiplier orig/ours={}/{}{}; {hot_text}",
            g.count_x,
            g.count_z,
            raw.summary("raw"),
            adjusted.summary("adjusted"),
            unvisited.mismatched,
            o.average,
            if lo == hi {
                lo.to_string()
            } else {
                format!("{lo}..{hi}")
            },
            if average_ok { "" } else { "!" },
            o.g,
            g_ours,
            if g_ok { "" } else { "!" },
        );
        let firsts: Vec<String> = [("raw", &raw), ("adjusted", &adjusted), ("unvisited", &unvisited)]
            .iter()
            .filter(|(_, d)| !d.first.is_empty())
            .map(|(name, d)| format!("{name} {}", d.first.join(" ")))
            .collect();
        if !firsts.is_empty() {
            line += &format!("; first differing (x,z) orig/ours: {}", firsts.join("; "));
        }
        line += &format!("; events={}", m.events);
        line += &match m.watered_grid {
            Some(_) => format!(
                "; plumbing passes={} (tracts whose watered state changed: {})",
                m.passes,
                (0..n).filter(|&i| m.w_changed[i] || m.w_prev[i] != w_end[i]).count()
            ),
            None => "; plumbing passes not polled".to_string(),
        };
        (mismatches, line, w_end, z_end)
    }

    /// The getters on the resynchronised mirror; the rest of the line.
    fn getters(&mut self, m: &Mirror, o: &Orig, before: usize) -> String {
        let mc = self.max_cells;
        let ours_hot = m.sim.flammability_hotspot();
        let hot_ok = ours_hot == o.hotspot_getter;
        let show = |h: Option<(u8, i32, i32)>| match h {
            Some((v, x, z)) => format!("{v}@({x},{z})"),
            None => "none".to_string(),
        };
        let ours_avg = m.sim.average_flammability();
        let avg_ok = ours_avg == o.average_getter;
        let mut flammable = None;
        if let Some((cx, cz, v)) = &o.flammable {
            let mut d = Diff::new(v.len());
            for x in 0..*cx {
                for z in 0..*cz {
                    let orig = v[x as usize * *cz as usize + z as usize] != 0;
                    let ours = m.sim.is_cell_flammable(x, z);
                    if orig != ours {
                        d.note(x, z, u8::from(orig), u8::from(ours), 1.0, mc);
                    }
                }
            }
            flammable = Some(d);
        }
        let getters =
            usize::from(!hot_ok) + usize::from(!avg_ok) + flammable.as_ref().map_or(0, |d: &Diff| d.mismatched);
        let mismatches = before + getters;
        let s = &mut self.stats;
        s.getter_mismatches = getters as u32;
        s.compared = s.compared.wrapping_add(1);
        if mismatches == 0 {
            s.clean = s.clean.wrapping_add(1);
        }
        s.last_mismatches = mismatches.min(i32::MAX as usize) as i32;
        let mut out = format!(
            "; hotspot getter orig/ours={}/{}{}; average getter orig/ours={}/{}{}; ",
            show(o.hotspot_getter),
            show(ours_hot),
            if hot_ok { "" } else { "!" },
            o.average_getter,
            ours_avg,
            if avg_ok { "" } else { "!" },
        );
        out += &match &flammable {
            Some(d) => {
                let mut t = format!("flammable getter cells={} mismatched={}", d.cells, d.mismatched);
                if !d.first.is_empty() {
                    t += &format!(" (first (x,z) orig/ours: {})", d.first.join(" "));
                }
                t
            }
            None => "flammable getter not read".to_string(),
        };
        out += &format!(
            "; mismatches={mismatches}; cycles={} clean={}; not compared: the fire roll, the iterator start and the \
             hotspot shuffle (random draws)",
            s.compared, s.clean
        );
        out
    }
}

/// What a handle points at. The counters the stats getter reads are a copy
/// in a `Cell`, outside the shadow's `RefCell`: the getter never borrows, so
/// it answers from anywhere (a host callback, the sink), with the counters
/// of the last call that finished.
struct Handle {
    stats: Cell<fc4_flammability_shadow_stats>,
    shadow: RefCell<Shadow>,
}

fn handle<'a>(p: *const fc4_flammability_shadow) -> Option<&'a Handle> {
    // SAFETY: documented precondition: NULL or a live fc4_flammability_shadow*.
    unsafe { (p as *const Handle).as_ref() }
}

fn with_shadow<R>(p: *const fc4_flammability_shadow, fallback: R, f: impl FnOnce(&mut Shadow) -> R) -> R {
    let Some(h) = handle(p) else {
        return fallback;
    };
    let Ok(mut s) = h.shadow.try_borrow_mut() else {
        return fallback;
    };
    let r = guard(fallback, || f(&mut s));
    h.stats.set(s.stats);
    r
}

/// Creates a shadow over a read-only host table: only its read callbacks
/// (city counts, tuning, occupants, lots, ordinance factor, season, watered,
/// zone type) are called; the grid API, `record_history`, `ignite_cell`,
/// `set_fire_cause`, `random_lot` and `post` may be NULL and are never
/// called. `max_cells`: differing tracts listed per grid (0 = 4, at most 64).
#[no_mangle]
pub unsafe extern "C" fn fc4_flammability_shadow_create(
    host: *const fc4_flammability_host,
    max_cells: u32,
) -> *mut fc4_flammability_shadow {
    guard(std::ptr::null_mut(), || {
        let Some(h) = host.as_ref() else {
            return std::ptr::null_mut();
        };
        if (h.struct_size as usize) < std::mem::size_of::<fc4_flammability_host>() {
            return std::ptr::null_mut();
        }
        let max_cells = match max_cells as usize {
            0 => DEFAULT_CELLS,
            n => n.min(MAX_CELLS),
        };
        let shadow = Shadow {
            host: Rc::new(ShadowHost {
                c: CHost(*h),
                geom: Cell::new(GridGeometry::default()),
                created: RefCell::new(Vec::new()),
                blocked: Cell::new(0),
            }),
            max_cells,
            mirror: None,
            stats: fc4_flammability_shadow_stats {
                struct_size: std::mem::size_of::<fc4_flammability_shadow_stats>() as u32,
                last_mismatches: -1,
                ..Default::default()
            },
        };
        Box::into_raw(Box::new(Handle {
            stats: Cell::new(shadow.stats),
            shadow: RefCell::new(shadow),
        })) as *mut fc4_flammability_shadow
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_flammability_shadow_destroy(s: *mut fc4_flammability_shadow) {
    if s.is_null() {
        return;
    }
    guard((), || drop(Box::from_raw(s as *mut Handle)));
}

/// One of the original's six messages (§5), in the same delivery as the
/// original's (FL4: subscription order, the original first; both read the
/// same game state): the mirror handles it too. False when it was not
/// mirrored (no baseline yet, or another type). Every call, of any type
/// (the plugin also forwards the plumbing pass end), first polls the
/// plumbing simulator's published grid (FL5/FL6).
#[no_mangle]
pub unsafe extern "C" fn fc4_flammability_shadow_message(
    s: *mut fc4_flammability_shadow,
    m: *const fc4_flammability_message,
) -> bool {
    let Some(m) = m.as_ref() else { return false };
    with_shadow(s, false, |sh| sh.message(m))
}

/// The original's cycle end: the first one (or after a new simulator) is the
/// baseline; later ones are compared. One line to `sink`. Returns the
/// differences, -1 when nothing was compared.
#[no_mangle]
pub unsafe extern "C" fn fc4_flammability_shadow_cycle(
    s: *mut fc4_flammability_shadow,
    orig: *const fc4_flammability_original,
    julian: i32,
    sink: fc4_bytes_sink,
    ctx: *mut c_void,
) -> i32 {
    let o = orig_of(orig);
    let (n, line) = with_shadow(s, (-1, String::new()), |sh| sh.cycle(o, julian));
    if !line.is_empty() {
        emit(&line, sink, ctx);
    }
    n
}

#[no_mangle]
pub unsafe extern "C" fn fc4_flammability_shadow_get_stats(
    s: *const fc4_flammability_shadow,
    out: *mut fc4_flammability_shadow_stats,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    let Some(h) = handle(s) else { return false };
    *o = h.stats.get();
    true
}
