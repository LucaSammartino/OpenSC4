// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The land value grid shadow (fc4.h "land value shadow"): with the
//! ORIGINAL land value simulator running, FreeCity4's pass (states 0-8,
//! docs/specs/landvalue.md §6, the core code our simulator runs) runs on the
//! original's live inputs and is compared with the original's grids and
//! scalars (tooling; no spec behaviour).
//!
//! # When
//!
//! At the end of each of the original's passes. LandValueMapChanged
//! (`0x87CC75B3`, §9) is posted at the end of state 8, so when it is
//! delivered the original has finished a pass (state 9) and its three grids
//! and its average belong to that pass. A month boundary is not a pass
//! boundary: a pass starts only at a SimNewMonth after a change (§5), runs
//! over several SimIdle / SimNewDay calls, and writes the slope and
//! intrinsic grids in place while it runs (§6), so at a month boundary the
//! grids can be a mix of two passes, or of none recent. The pass end is the
//! one moment at which every published value belongs to one pass.
//!
//! # What
//!
//! Our pass runs on local grids: the read-only host table answers the
//! terrain, the zones, the lot developer's area slope, the growth developers
//! and their desirability maps live; every grid write, release, history
//! record and post goes to the shadow's own copies or nowhere. The local
//! grids take the original's geometries, and the intrinsic grid starts as a
//! copy of the original's (state 4 leaves it untouched when the altitude
//! curve is absent, §6 [quirk]). Compared, after our pass:
//!
//! - the intrinsic, total and slope grids, tract by tract (the slope
//!   bitwise);
//! - `GetLandValueType` (slot 5) at the first cell of every total tract,
//!   against our thresholds applied to the original's value there (§7.6;
//!   *[compat]* LV4: the type is uniform over a tract, so this is exact);
//! - the average (slot 10), the mode altitude (slot 11, bitwise) and the
//!   mean altitude (the record, §10: it has no getter; bitwise).
//!
//! Not compared (no getter, not saved): the scratch grid, the histogram,
//! min/max altitude, pivot and ranges (they show through the intrinsic
//! grid), the pipeline state and the dirty flag (only their OR is in the
//! record: logged as `settled`).
//!
//! # Limits
//!
//! The inputs are read when the pass has ended, not at the moment each
//! state read them. A change during the pass sets the original's dirty flag,
//! so the record's byte 5 is 1 (`settled=0`) and a difference may be such a
//! change. To tell such a change from a formula difference, each SimNewDay
//! after the first pass end also runs our pass on that day's live inputs;
//! when a pass end's intrinsic grid or altitude scalars differ, the line
//! ends with `inputs:`, naming the first of those days that reproduces them
//! bitwise, or saying none does (diagnostic only: the counts are those of
//! the pass-end inputs). The growth developers' desirability maps are not covered by the
//! dirty flag: they are republished about once a month, near the month
//! start, where passes start too. *[compat]* (§7.4, LV2): a developer's
//! published map changes only in its state-17 call, in one piece, and at
//! most once between the original's state 7 and the comparison, so there
//! are exactly two candidates: the map before the developer's last state-17
//! copy, and the live one. The shadow snapshots each listed developer's map
//! at each SimNewDay ([`fc4_landvalue_shadow_note_day`]) and after each
//! comparison, but only while the developer's slot 10
//! (`GrowthAndDeclineListsAreValid`, `state < 17`) is true, when the map
//! cannot change; the last such snapshot before a change is the map from
//! before the copy (with the host's slot-10 callback NULL, every note
//! snapshots). When the total grid differs and the other candidate differs
//! from the live map, our pass runs again with it and the better result is
//! kept (`desirability=previous`).
//!
//! *[compat]* (§7.4, LV1): the original applies the listed developers in
//! ascending unsigned order of their pointers (a map keyed by the pointer),
//! not in tuning order; our pass is told to do the same
//! ([`LandValueSim::order_developers_by_address`]), the tokens being the
//! pointers `GetDeveloper(ID)` returns.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::fmt::Display;
use std::rc::Rc;

use fc4_core::julian::civil_from_julian;
use fc4_core::landvalue::{
    GridKind, LandValueHost, LandValueSim, TuningInput, INSTANCE_INTRINSIC, INSTANCE_SLOPE, INSTANCE_TOTAL,
    STATE_FINISHED,
};
use fc4_core::simgrid::GridGeometry;

use crate::guard;
use crate::landvalue::{fc4_grid_geometry, fc4_landvalue_host, fc4_landvalue_tuning, tuning_of, CHost};
use crate::stream::fc4_bytes_sink;

/// Opaque handle (fc4.h `fc4_landvalue_shadow`).
#[repr(C)]
pub struct fc4_landvalue_shadow {
    _private: [u8; 0],
}

/// What the plugin read from the original at the end of its pass (fc4.h
/// `fc4_landvalue_original`). Grids are x outer (`count_x × count_z`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_landvalue_original {
    pub struct_size: u32,
    /// Non-zero: the record (the original's own Write) parsed (version 1).
    pub has_record: u32,
    /// `GetIntrinsicLandValueMap` (slot 7): its geometry and every tract.
    pub intrinsic_geometry: fc4_grid_geometry,
    pub intrinsic: *const u8,
    /// `GetTotalLandValueMap` (slot 8).
    pub total_geometry: fc4_grid_geometry,
    pub total: *const u8,
    /// `GetSlopeMap` (slot 9).
    pub slope_geometry: fc4_grid_geometry,
    pub slope: *const f32,
    /// `GetLandValueType` (slot 5) at the first cell of every total tract;
    /// NULL = not read.
    pub types: *const u8,
    /// `GetAverageLandValue` (slot 10).
    pub average: u8,
    /// The record's average (field 6) and its byte 5 (dirty or a pass running).
    pub record_average: u8,
    pub record_flag: u8,
    pub reserved: u8,
    /// `GetModeAltitude` (slot 11).
    pub mode_altitude: f32,
    /// The record's mean altitude (field 7).
    pub mean_altitude: f32,
}

/// Counters (fc4.h `fc4_landvalue_shadow_stats`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_landvalue_shadow_stats {
    pub struct_size: u32,
    /// Comparison calls, those that compared, and those without a difference.
    pub passes: u32,
    pub compared: u32,
    pub clean: u32,
    /// The last call's difference count; -1 = not compared.
    pub last_mismatches: i32,
    /// The last compared pass, by kind.
    pub intrinsic_mismatches: u32,
    pub total_mismatches: u32,
    pub slope_mismatches: u32,
    pub type_mismatches: u32,
    pub scalar_mismatches: u32,
    /// Passes compared with a developer's previous desirability map.
    pub desirability_reruns: u32,
    /// Snapshot changes (`note_day` and after each comparison; LV2).
    pub desirability_changes: u32,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_landvalue_shadow_stats>() == 48);
    assert!(std::mem::size_of::<fc4_landvalue_original>() == if cfg!(target_pointer_width = "32") { 132 } else { 152 });
};

/// Local grid tokens (host tokens are pointers, far below this tag).
const LOCAL_TAG: u64 = 0x4C56_0000_0000_0000;
const LOCAL_MASK: u64 = 0xFFFF_0000_0000_0000;
/// The largest tract count per axis accepted from the original.
const MAX_COUNT: i32 = 4096;
/// Step calls allowed for one pass (10 + one per developer, §6).
const MAX_CALLS: u32 = 4096;
const DEFAULT_CELLS: usize = 4;
const MAX_CELLS: usize = 64;

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

fn valid(g: &GridGeometry) -> bool {
    g.tract_size >= 1 && (1..=MAX_COUNT).contains(&g.count_x) && (1..=MAX_COUNT).contains(&g.count_z)
}

fn same_counts(a: &GridGeometry, b: &GridGeometry) -> bool {
    a.count_x == b.count_x && a.count_z == b.count_z && a.tract_size == b.tract_size
}

// ---- the read-only host ---------------------------------------------------------------------------

enum LocalData {
    U8(Vec<u8>),
    F32(Vec<f32>),
}

struct Local {
    geom: GridGeometry,
    data: LocalData,
}

/// The geometries (and the intrinsic grid's values) local grids are made with.
#[derive(Default)]
struct Seeds {
    intrinsic: Option<(GridGeometry, Vec<u8>)>,
    total: Option<GridGeometry>,
    slope: Option<GridGeometry>,
}

/// The game's read callbacks, with every grid our pass creates kept local
/// and every write, release, history record and post swallowed.
struct ShadowHost {
    c: CHost,
    locals: RefCell<Vec<Local>>,
    seeds: RefCell<Seeds>,
    /// Desirability maps served from a snapshot (the rerun): grid token →
    /// its geometry and values.
    overrides: RefCell<BTreeMap<u64, (GridGeometry, Vec<i16>)>>,
    /// Grids our Init asked for with a tract size other than the original's.
    size_mismatches: Cell<u32>,
    /// Writes our pass aimed at a grid that is not local (never expected).
    foreign_writes: Cell<u32>,
    posts: Cell<u32>,
    history: Cell<Option<i32>>,
}

impl ShadowHost {
    fn local(&self, t: u64) -> Option<usize> {
        if t & LOCAL_MASK != LOCAL_TAG {
            return None;
        }
        let i = (t & !LOCAL_MASK) as usize;
        (i < self.locals.borrow().len()).then_some(i)
    }

    fn local_u8(&self, t: u64) -> Option<Vec<u8>> {
        let i = self.local(t)?;
        match &self.locals.borrow()[i].data {
            LocalData::U8(v) => Some(v.clone()),
            LocalData::F32(_) => None,
        }
    }

    fn local_f32(&self, t: u64) -> Option<Vec<f32>> {
        let i = self.local(t)?;
        match &self.locals.borrow()[i].data {
            LocalData::F32(v) => Some(v.clone()),
            LocalData::U8(_) => None,
        }
    }
}

struct HostRef(Rc<ShadowHost>);

impl LandValueHost for HostRef {
    fn terrain_cells(&self) -> (i32, i32) {
        self.0.c.terrain_cells()
    }
    fn is_land_vertex(&self, vx: u32, vz: u32) -> bool {
        self.0.c.is_land_vertex(vx, vz)
    }
    fn altitude(&self, px: f32, pz: f32) -> f32 {
        self.0.c.altitude(px, pz)
    }
    fn min_allowed_height(&self) -> f32 {
        self.0.c.min_allowed_height()
    }
    fn max_allowed_height(&self) -> f32 {
        self.0.c.max_allowed_height()
    }
    fn zone_type(&self, x: i32, z: i32) -> u32 {
        self.0.c.zone_type(x, z)
    }
    fn area_slope(&self, x1: i32, z1: i32, x2: i32, z2: i32) -> f32 {
        self.0.c.area_slope(x1, z1, x2, z2)
    }
    fn developer(&self, developer_type: u32) -> u64 {
        self.0.c.developer(developer_type)
    }
    fn desirability_map(&self, developer: u64) -> u64 {
        self.0.c.desirability_map(developer)
    }
    fn create_grid(&self, kind: GridKind, tract_size: i32, instance_id: u32) -> u64 {
        let h = &self.0;
        let (geom, data) = {
            let s = h.seeds.borrow();
            match (instance_id, kind) {
                (INSTANCE_INTRINSIC, GridKind::U8) => match &s.intrinsic {
                    Some((g, v)) => (*g, LocalData::U8(v.clone())),
                    None => return 0,
                },
                (INSTANCE_TOTAL, GridKind::U8) => match s.total {
                    Some(g) => (g, LocalData::U8(vec![0; g.len()])),
                    None => return 0,
                },
                (INSTANCE_SLOPE, GridKind::F32) => match s.slope {
                    Some(g) => (g, LocalData::F32(vec![0.0; g.len()])),
                    None => return 0,
                },
                _ => return 0,
            }
        };
        if geom.tract_size != tract_size {
            h.size_mismatches.set(h.size_mismatches.get() + 1);
        }
        let mut l = h.locals.borrow_mut();
        l.push(Local { geom, data });
        LOCAL_TAG | (l.len() as u64 - 1)
    }
    fn grid_geometry(&self, grid: u64) -> GridGeometry {
        match self.0.local(grid) {
            Some(i) => self.0.locals.borrow()[i].geom,
            None => self.0.c.grid_geometry(grid),
        }
    }
    fn grid_set_tract_size(&self, _grid: u64, _tract_size: i32) {
        // Only asked for a restored grid (our grids are all new): never
        // forwarded (read-only).
    }
    fn grid_read_u8(&self, grid: u64, out: &mut [u8]) {
        match self.0.local(grid) {
            Some(i) => {
                if let LocalData::U8(v) = &self.0.locals.borrow()[i].data {
                    let n = out.len().min(v.len());
                    out[..n].copy_from_slice(&v[..n]);
                }
            }
            None => self.0.c.grid_read_u8(grid, out),
        }
    }
    fn grid_read_i16(&self, grid: u64, out: &mut [i16]) {
        if let Some((_, v)) = self.0.overrides.borrow().get(&grid) {
            let n = out.len().min(v.len());
            out[..n].copy_from_slice(&v[..n]);
            return;
        }
        self.0.c.grid_read_i16(grid, out)
    }
    fn grid_write_u8(&self, grid: u64, values: &[u8]) {
        match self.0.local(grid) {
            Some(i) => {
                if let LocalData::U8(v) = &mut self.0.locals.borrow_mut()[i].data {
                    let n = values.len().min(v.len());
                    v[..n].copy_from_slice(&values[..n]);
                }
            }
            None => self.0.foreign_writes.set(self.0.foreign_writes.get() + 1),
        }
    }
    fn grid_write_f32(&self, grid: u64, values: &[f32]) {
        match self.0.local(grid) {
            Some(i) => {
                if let LocalData::F32(v) = &mut self.0.locals.borrow_mut()[i].data {
                    let n = values.len().min(v.len());
                    v[..n].copy_from_slice(&values[..n]);
                }
            }
            None => self.0.foreign_writes.set(self.0.foreign_writes.get() + 1),
        }
    }
    fn grid_shutdown(&self, _grid: u64) {}
    fn release(&self, _token: u64) {}
    fn set_history_queue_length(&self, _series: u32, _length: u32) {}
    fn record_history(&self, _series: u32, value: i32) {
        self.0.history.set(Some(value));
    }
    fn post(&self, _msg_type: u32) {
        self.0.posts.set(self.0.posts.get() + 1);
    }
}

// ---- the original's results ---------------------------------------------------------------------

#[derive(Clone)]
struct Record {
    flag: u8,
    average: u8,
    mean: f32,
}

#[derive(Clone)]
struct Orig {
    intrinsic: (GridGeometry, Vec<u8>),
    total: (GridGeometry, Vec<u8>),
    slope: (GridGeometry, Vec<f32>),
    types: Option<Vec<u8>>,
    average: u8,
    mode: f32,
    record: Option<Record>,
}

unsafe fn grid_of<T: Copy>(
    g: &fc4_grid_geometry,
    p: *const T,
    what: &'static str,
) -> Result<(GridGeometry, Vec<T>), String> {
    let geom = geometry_of(g);
    if !valid(&geom) {
        return Err(format!(
            "the original's {what} grid has geometry {}x{} (S={})",
            g.count_x, g.count_z, g.tract_size
        ));
    }
    if p.is_null() {
        return Err(format!("the original's {what} grid was not read"));
    }
    Ok((geom, std::slice::from_raw_parts(p, geom.len()).to_vec()))
}

unsafe fn orig_of(p: *const fc4_landvalue_original) -> Result<Orig, String> {
    let Some(o) = p.as_ref() else {
        return Err("nothing read from the original".into());
    };
    if (o.struct_size as usize) < std::mem::size_of::<fc4_landvalue_original>() {
        return Err("bad fc4_landvalue_original".into());
    }
    let intrinsic = grid_of(&o.intrinsic_geometry, o.intrinsic, "intrinsic")?;
    let total = grid_of(&o.total_geometry, o.total, "total")?;
    let slope = grid_of(&o.slope_geometry, o.slope, "slope")?;
    if !same_counts(&intrinsic.0, &total.0) {
        return Err("the original's intrinsic and total grids differ in geometry".into());
    }
    let types = (!o.types.is_null()).then(|| std::slice::from_raw_parts(o.types, total.0.len()).to_vec());
    Ok(Orig {
        intrinsic,
        total,
        slope,
        types,
        average: o.average,
        mode: o.mode_altitude,
        record: (o.has_record != 0).then_some(Record {
            flag: o.record_flag,
            average: o.record_average,
            mean: o.mean_altitude,
        }),
    })
}

// ---- comparison helpers -----------------------------------------------------------------------------

pub(crate) struct Diff {
    pub(crate) cells: usize,
    pub(crate) mismatched: usize,
    pub(crate) max: f64,
    pub(crate) first: Vec<String>,
}

impl Diff {
    pub(crate) fn new(cells: usize) -> Diff {
        Diff {
            cells,
            mismatched: 0,
            max: 0.0,
            first: Vec::new(),
        }
    }

    pub(crate) fn note(&mut self, x: i32, z: i32, orig: impl Display, ours: impl Display, d: f64, max_cells: usize) {
        self.mismatched += 1;
        if d.is_nan() || d > self.max {
            self.max = if d.is_nan() { f64::INFINITY } else { d };
        }
        if self.first.len() < max_cells {
            self.first.push(format!("({x},{z}) {orig}/{ours}"));
        }
    }

    pub(crate) fn summary(&self, name: &str) -> String {
        format!(
            "{name} cells={} mismatched={} max|d|={}",
            self.cells, self.mismatched, self.max
        )
    }
}

fn diff_u8(g: &GridGeometry, orig: &[u8], ours: &[u8], max_cells: usize) -> Diff {
    let mut d = Diff::new(g.len());
    for x in 0..g.count_x {
        for z in 0..g.count_z {
            let i = x as usize * g.count_z as usize + z as usize;
            let (o, m) = (orig[i], ours.get(i).copied().unwrap_or(0));
            if o != m {
                d.note(x, z, o, m, f64::from(o.abs_diff(m)), max_cells);
            }
        }
    }
    d
}

fn diff_f32(g: &GridGeometry, orig: &[f32], ours: &[f32], max_cells: usize) -> Diff {
    let mut d = Diff::new(g.len());
    for x in 0..g.count_x {
        for z in 0..g.count_z {
            let i = x as usize * g.count_z as usize + z as usize;
            let (o, m) = (orig[i], ours.get(i).copied().unwrap_or(0.0));
            if o.to_bits() != m.to_bits() {
                d.note(x, z, o, m, (f64::from(o) - f64::from(m)).abs(), max_cells);
            }
        }
    }
    d
}

pub(crate) fn date(julian: i32) -> String {
    let d = civil_from_julian(julian);
    format!("{:04}-{:02}-{:02}", d.year, d.month, d.day)
}

fn same_f32(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

#[cfg(test)]
#[path = "landvalue_recording_tests.rs"]
mod recording_tests;

fn mark(same: bool) -> &'static str {
    if same {
        ""
    } else {
        "!"
    }
}

// ---- the shadow ---------------------------------------------------------------------------------

/// One listed developer's desirability map as last snapshotted (LV2).
struct DMap {
    grid: u64,
    geom: GridGeometry,
    /// The last snapshot taken while slot 10 was true (the map cannot change
    /// then).
    latest: Vec<i16>,
    /// The snapshot before `latest` changed: the map before the developer's
    /// last state-17 copy, once `latest` has caught up with the copy.
    previous: Option<Vec<i16>>,
}

/// One pass of ours.
struct Pass {
    finished: bool,
    calls: u32,
    intrinsic: Vec<u8>,
    total: Vec<u8>,
    slope: Vec<f32>,
    /// Our `GetLandValueType` thresholds on the original's total values.
    types: Vec<u8>,
    average: u8,
    mode: f32,
    mean: f32,
    developers: usize,
    posts: u32,
    history: Option<i32>,
    size_mismatches: u32,
    foreign_writes: u32,
}

struct Shadow {
    host: Rc<ShadowHost>,
    tuning: Option<TuningInput>,
    max_cells: usize,
    maps: BTreeMap<u32, DMap>,
    stats: fc4_landvalue_shadow_stats,
    /// Input provenance (diagnostic only, never counted): the original's
    /// read at the last pass end (geometries and the intrinsic seed for our
    /// day runs), and our pass at each SimNewDay since then.
    last: Option<Orig>,
    days: Vec<DayRun>,
}

/// Our pass on the live inputs of one SimNewDay between two pass ends.
struct DayRun {
    intrinsic: Vec<u8>,
    total: Vec<u8>,
    mean: f32,
    mode: f32,
}

/// Day runs kept between two pass ends (a pass spans about a month).
const MAX_DAY_RUNS: usize = 62;

impl Shadow {
    /// The developer types our Init lists (§8 [choice]: min(count of IDs,
    /// count of weights)).
    fn developer_ids(&self) -> Vec<u32> {
        match self.tuning.as_ref().map(|t| (&t.developer_ids, &t.developer_weights)) {
            Some((Some(ids), Some(w))) => ids.iter().take(w.len()).copied().collect(),
            _ => Vec::new(),
        }
    }

    /// A developer's live desirability map: the developer token, the grid
    /// token, its geometry and values.
    fn read_map(&self, id: u32) -> Option<(u64, u64, GridGeometry, Vec<i16>)> {
        let c = &self.host.c;
        let dev = c.developer(id);
        if dev == 0 {
            return None;
        }
        let grid = c.desirability_map(dev);
        if grid == 0 {
            return None;
        }
        let geom = c.grid_geometry(grid);
        if !valid(&geom) {
            return None;
        }
        let mut v = vec![0i16; geom.len()];
        c.grid_read_i16(grid, &mut v);
        Some((dev, grid, geom, v))
    }

    /// Snapshots every listed developer's map whose slot 10 is true (or
    /// unknown); returns how many snapshots changed (LV2).
    fn note(&mut self) -> u32 {
        let mut changed = 0;
        for id in self.developer_ids() {
            let Some((dev, grid, geom, v)) = self.read_map(id) else {
                self.maps.remove(&id);
                continue;
            };
            // Slot 10 false: state 17 is pending or done, the map may be
            // changing this update; keep the snapshot from before it.
            if self.host.c.developer_lists_valid(dev) == Some(false) {
                continue;
            }
            match self.maps.get_mut(&id) {
                Some(m) if m.grid == grid && same_counts(&m.geom, &geom) => {
                    if m.latest != v {
                        m.previous = Some(std::mem::replace(&mut m.latest, v));
                        changed += 1;
                    }
                }
                _ => {
                    self.maps.insert(
                        id,
                        DMap {
                            grid,
                            geom,
                            latest: v,
                            previous: None,
                        },
                    );
                }
            }
        }
        self.stats.desirability_changes = self.stats.desirability_changes.wrapping_add(changed);
        changed
    }

    /// The second candidate per developer (LV2), to try instead of the live
    /// map: the last snapshot when the live map differs from it (a copy
    /// since), else the snapshot before it (the map before the last copy).
    fn alternatives(&self) -> BTreeMap<u64, (GridGeometry, Vec<i16>)> {
        let mut out = BTreeMap::new();
        for (&id, m) in &self.maps {
            let Some((_, grid, geom, live)) = self.read_map(id) else {
                continue;
            };
            if grid != m.grid || !same_counts(&geom, &m.geom) {
                continue;
            }
            let alt = if m.latest != live {
                Some(&m.latest)
            } else {
                m.previous.as_ref()
            };
            if let Some(a) = alt.filter(|a| a.len() == live.len() && **a != live) {
                out.insert(grid, (geom, a.clone()));
            }
        }
        out
    }

    /// Our pass on the original's inputs, `overrides` replacing desirability maps.
    fn run(&self, o: &Orig, overrides: BTreeMap<u64, (GridGeometry, Vec<i16>)>) -> Pass {
        let h = &self.host;
        h.locals.borrow_mut().clear();
        *h.seeds.borrow_mut() = Seeds {
            intrinsic: Some((o.intrinsic.0, o.intrinsic.1.clone())),
            total: Some(o.total.0),
            slope: Some(o.slope.0),
        };
        *h.overrides.borrow_mut() = overrides;
        h.size_mismatches.set(0);
        h.foreign_writes.set(0);
        h.posts.set(0);
        h.history.set(None);
        let mut sim = LandValueSim::new(Box::new(HostRef(h.clone())));
        // Shadows replay the original's pacing (simulator.md §5.7).
        sim.set_pacing(fc4_core::pacing::Pacing::Original);
        sim.init(self.tuning.as_ref());
        // [compat] (§7.4, LV1): the original's order, ascending developer
        // pointer; our tokens are those pointers.
        sim.order_developers_by_address();
        let mut calls = 0;
        while sim.pipeline_state() != STATE_FINISHED && calls < MAX_CALLS {
            sim.step(u32::MAX);
            calls += 1;
        }
        let st = sim.snapshot();
        let types = o.total.1.iter().map(|&v| sim.land_value_type(v) as u8).collect();
        let pass = Pass {
            finished: sim.pipeline_state() == STATE_FINISHED,
            calls,
            intrinsic: h.local_u8(sim.intrinsic_grid()).unwrap_or_default(),
            total: h.local_u8(sim.total_grid()).unwrap_or_default(),
            slope: h.local_f32(sim.slope_grid()).unwrap_or_default(),
            types,
            average: sim.average_land_value(),
            mode: sim.mode_altitude(),
            mean: st.mean_alt,
            developers: st.developers.len(),
            posts: h.posts.get(),
            history: h.history.get(),
            size_mismatches: h.size_mismatches.get(),
            foreign_writes: h.foreign_writes.get(),
        };
        h.overrides.borrow_mut().clear();
        h.locals.borrow_mut().clear();
        pass
    }

    /// SimNewDay: the desirability snapshots (LV2), then, once a pass end has
    /// given the geometries, our pass on today's live inputs, kept for the
    /// input provenance of the next comparison (diagnostic only).
    fn note_day(&mut self) -> u32 {
        let changed = self.note();
        if self.days.len() < MAX_DAY_RUNS {
            if let Some(o) = self.last.take() {
                let p = self.run(&o, BTreeMap::new());
                if p.finished && p.size_mismatches == 0 {
                    self.days.push(DayRun {
                        intrinsic: p.intrinsic,
                        total: p.total,
                        mean: p.mean,
                        mode: p.mode,
                    });
                }
                self.last = Some(o);
            }
        }
        changed
    }

    /// Input provenance of a pass whose intrinsic grid or altitude scalars
    /// differ (tooling, never counted; landvalue.md §6: states 1-4 read the
    /// terrain and the zones some calls before the pass end, our pass reads
    /// them at the pass end). Names the first SimNewDay since the previous
    /// pass end whose inputs give the original's intrinsic grid and mode and
    /// mean altitude bitwise, or says that none does.
    fn provenance(o: &Orig, days: &[DayRun]) -> String {
        let n = days.len();
        if n == 0 {
            return "; inputs: no SimNewDay run since the previous pass end".to_string();
        }
        let exact = |d: &DayRun| {
            d.intrinsic == o.intrinsic.1
                && same_f32(d.mode, o.mode)
                && o.record.as_ref().map_or(true, |r| same_f32(r.mean, d.mean))
        };
        let differing = |a: &[u8], b: &[u8]| {
            if a.len() == b.len() {
                a.iter().zip(b).filter(|(x, y)| x != y).count()
            } else {
                a.len().max(b.len())
            }
        };
        match days.iter().position(exact) {
            Some(k) => format!(
                "; inputs: the original's intrinsic, modeAlt and meanAlt are exact with our pass on the inputs of \
                 SimNewDay {}/{n} since the previous pass end (an input changed after the original read it), total \
                 there {}",
                k + 1,
                match differing(&days[k].total, &o.total.1) {
                    0 => "exact".to_string(),
                    t => format!("differs in {t} tract(s)"),
                }
            ),
            None => {
                let (k, best) = days
                    .iter()
                    .enumerate()
                    .map(|(k, d)| (k, differing(&d.intrinsic, &o.intrinsic.1)))
                    .min_by_key(|&(_, c)| c)
                    .unwrap_or((0, 0));
                format!(
                    "; inputs: no SimNewDay of {n} since the previous pass end reproduces the original (closest: day \
                     {}, intrinsic differs in {best} tract(s))",
                    k + 1
                )
            }
        }
    }

    fn compare(&mut self, o: Result<Orig, String>, julian: i32) -> (i32, String) {
        let days = std::mem::take(&mut self.days);
        self.stats.passes = self.stats.passes.wrapping_add(1);
        self.stats.last_mismatches = -1;
        let n = self.stats.passes;
        let head = format!("landvalue shadow {} pass {n}", date(julian));
        let o = match o {
            Ok(o) => o,
            Err(why) => return (-1, format!("{head}: INCOMPLETE ({why}); not compared")),
        };
        let mut pass = self.run(&o, BTreeMap::new());
        if !pass.finished {
            return (
                -1,
                format!(
                    "{head}: INCOMPLETE (our pass did not finish in {} steps); not compared",
                    pass.calls
                ),
            );
        }
        if pass.size_mismatches != 0 {
            return (
                -1,
                format!(
                    "{head}: INCOMPLETE (the tuning's tract size differs from the original's restored grids, §14); not \
                     compared"
                ),
            );
        }
        let mc = self.max_cells;
        let mut total = diff_u8(&o.total.0, &o.total.1, &pass.total, mc);
        let mut desirability = if pass.developers == 0 {
            "none (no developer listed)".to_string()
        } else {
            format!("live ({} developer(s))", pass.developers)
        };
        if total.mismatched > 0 {
            let alt = self.alternatives();
            if !alt.is_empty() {
                let again = self.run(&o, alt);
                let t2 = diff_u8(&o.total.0, &o.total.1, &again.total, mc);
                if again.finished && t2.mismatched < total.mismatched {
                    desirability = format!(
                        "previous map (it changed after the original's pass read it; with the live map total \
                         mismatched={})",
                        total.mismatched
                    );
                    self.stats.desirability_reruns = self.stats.desirability_reruns.wrapping_add(1);
                    total = t2;
                    pass = again;
                } else {
                    desirability = format!(
                        "live ({} developer(s); the previous map gave total mismatched={})",
                        pass.developers, t2.mismatched
                    );
                }
            }
        }
        self.note();

        let intrinsic = diff_u8(&o.intrinsic.0, &o.intrinsic.1, &pass.intrinsic, mc);
        let slope = diff_f32(&o.slope.0, &o.slope.1, &pass.slope, mc);
        let mut types = Diff::new(o.total.0.len());
        if let Some(t) = &o.types {
            let g = &o.total.0;
            for x in 0..g.count_x {
                for z in 0..g.count_z {
                    let i = x as usize * g.count_z as usize + z as usize;
                    if t[i] != pass.types[i] {
                        types.note(x, z, t[i], pass.types[i], f64::from(t[i].abs_diff(pass.types[i])), mc);
                    }
                }
            }
        }
        let average_same = o.average == pass.average;
        let mode_same = same_f32(o.mode, pass.mode);
        let mean_same = o.record.as_ref().map_or(true, |r| same_f32(r.mean, pass.mean));
        let scalars = [average_same, mode_same, mean_same].iter().filter(|s| !**s).count();
        let mismatches = intrinsic.mismatched + total.mismatched + slope.mismatched + types.mismatched + scalars;

        let s = &mut self.stats;
        s.compared = s.compared.wrapping_add(1);
        if mismatches == 0 {
            s.clean = s.clean.wrapping_add(1);
        }
        s.last_mismatches = mismatches.min(i32::MAX as usize) as i32;
        s.intrinsic_mismatches = intrinsic.mismatched as u32;
        s.total_mismatches = total.mismatched as u32;
        s.slope_mismatches = slope.mismatched as u32;
        s.type_mismatches = types.mismatched as u32;
        s.scalar_mismatches = scalars as u32;

        let g = &o.intrinsic.0;
        let sg = &o.slope.0;
        let mut line = format!(
            "{head}: compared on {}x{} tracts (S={}), slope {}x{} (S={}): {}; {}; {}; types {}",
            g.count_x,
            g.count_z,
            g.tract_size,
            sg.count_x,
            sg.count_z,
            sg.tract_size,
            intrinsic.summary("intrinsic"),
            total.summary("total"),
            slope.summary("slope"),
            if o.types.is_some() {
                format!("mismatched={}/{}", types.mismatched, types.cells)
            } else {
                "not read".to_string()
            },
        );
        line += &format!(
            "; average orig/ours={}/{}{}; modeAlt orig/ours={}/{}{}",
            o.average,
            pass.average,
            mark(average_same),
            o.mode,
            pass.mode,
            mark(mode_same)
        );
        match &o.record {
            Some(r) => {
                line += &format!("; meanAlt orig/ours={}/{}{}", r.mean, pass.mean, mark(mean_same));
                if r.average != o.average {
                    line += &format!(" (the record's average {} differs from slot 10)", r.average);
                }
                line += if r.flag == 0 {
                    "; settled=1"
                } else {
                    "; settled=0 (the original's record says dirty or a pass running)"
                };
            }
            None => line += "; meanAlt not read (no record); settled=?",
        }
        line += &format!(
            "; desirability={desirability}; mismatches={mismatches}; passes={} clean={}",
            self.stats.compared, self.stats.clean
        );
        // LV1, section 6.1: matching output does not prove matching inputs.
        // This shadow samples at pass end (and retries a previous D); it has
        // no record of the native per-Step tract/developer reads. Keep that
        // limitation on every compared line, including zero-difference lines.
        line += "; input_capture=pass-end-sample; native_step_capture=0";
        if pass.posts != 1 || pass.history != Some(i32::from(pass.average)) || pass.foreign_writes != 0 {
            line += &format!(
                "; ours: posts={} history={:?} blocked writes={} (expected 1, the average, 0)",
                pass.posts, pass.history, pass.foreign_writes
            );
        }
        let firsts: Vec<String> = [
            ("intrinsic", &intrinsic),
            ("total", &total),
            ("slope", &slope),
            ("types", &types),
        ]
        .iter()
        .filter(|(_, d)| !d.first.is_empty())
        .map(|(name, d)| format!("{name} {}", d.first.join(" ")))
        .collect();
        if !firsts.is_empty() {
            line += &format!("; first differing (x,z) orig/ours: {}", firsts.join("; "));
        }
        if intrinsic.mismatched > 0 || !mode_same || !mean_same {
            line += &Self::provenance(&o, &days);
        }
        self.last = Some(o);
        (self.stats.last_mismatches, line)
    }
}

/// What a handle points at. The counters the stats getter reads are a copy
/// in a `Cell`, outside the shadow's `RefCell`: the getter never borrows, so
/// it answers from anywhere (a host callback, the sink), with the counters
/// of the last call that finished.
struct Handle {
    stats: Cell<fc4_landvalue_shadow_stats>,
    shadow: RefCell<Shadow>,
}

fn handle<'a>(p: *const fc4_landvalue_shadow) -> Option<&'a Handle> {
    // SAFETY: documented precondition: NULL or a live fc4_landvalue_shadow*.
    unsafe { (p as *const Handle).as_ref() }
}

fn with_shadow<R>(p: *const fc4_landvalue_shadow, fallback: R, f: impl FnOnce(&mut Shadow) -> R) -> R {
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

pub(crate) unsafe fn emit(line: &str, sink: fc4_bytes_sink, ctx: *mut c_void) {
    if let Some(f) = sink {
        f(ctx, line.as_ptr(), line.len());
    }
}

/// Creates a shadow over a read-only host table: only its read callbacks
/// (terrain, zones, area slope, developers, desirability maps, grid geometry
/// and reads) are called; `create_grid`, the writes, `release`, the history
/// and `post` may be NULL and are never called. `tuning` (copied; NULL = the
/// exemplar is missing) is what our Init reads; `max_cells`: differing
/// tracts listed per grid (0 = 4, at most 64).
#[no_mangle]
pub unsafe extern "C" fn fc4_landvalue_shadow_create(
    host: *const fc4_landvalue_host,
    tuning: *const fc4_landvalue_tuning,
    max_cells: u32,
) -> *mut fc4_landvalue_shadow {
    guard(std::ptr::null_mut(), || {
        let Some(h) = host.as_ref() else {
            return std::ptr::null_mut();
        };
        if (h.struct_size as usize) < std::mem::size_of::<fc4_landvalue_host>() {
            return std::ptr::null_mut();
        }
        let tuning = match tuning.as_ref() {
            Some(t) if (t.struct_size as usize) >= std::mem::size_of::<fc4_landvalue_tuning>() => Some(tuning_of(t)),
            Some(_) => return std::ptr::null_mut(),
            None => None,
        };
        let max_cells = match max_cells as usize {
            0 => DEFAULT_CELLS,
            n => n.min(MAX_CELLS),
        };
        let shadow = Shadow {
            host: Rc::new(ShadowHost {
                c: CHost(*h),
                locals: RefCell::new(Vec::new()),
                seeds: RefCell::new(Seeds::default()),
                overrides: RefCell::new(BTreeMap::new()),
                size_mismatches: Cell::new(0),
                foreign_writes: Cell::new(0),
                posts: Cell::new(0),
                history: Cell::new(None),
            }),
            tuning,
            max_cells,
            maps: BTreeMap::new(),
            last: None,
            days: Vec::new(),
            stats: fc4_landvalue_shadow_stats {
                struct_size: std::mem::size_of::<fc4_landvalue_shadow_stats>() as u32,
                last_mismatches: -1,
                ..Default::default()
            },
        };
        Box::into_raw(Box::new(Handle {
            stats: Cell::new(shadow.stats),
            shadow: RefCell::new(shadow),
        })) as *mut fc4_landvalue_shadow
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_landvalue_shadow_destroy(s: *mut fc4_landvalue_shadow) {
    if s.is_null() {
        return;
    }
    guard((), || drop(Box::from_raw(s as *mut Handle)));
}

/// SimNewDay: snapshots every listed developer's desirability map
/// (read-only) while its slot 10 is true or unknown (LV2). Returns how many
/// snapshots changed. After the first pass end it also runs our pass on the
/// day's live inputs, read-only, for the input provenance the next
/// comparison logs when its intrinsic grid or altitude scalars differ.
#[no_mangle]
pub unsafe extern "C" fn fc4_landvalue_shadow_note_day(s: *mut fc4_landvalue_shadow) -> u32 {
    with_shadow(s, 0, Shadow::note_day)
}

/// The end of one of the original's passes: our pass on its live inputs,
/// compared with `orig`; one line to `sink`. Returns the differences, -1
/// when nothing was compared (INCOMPLETE).
#[no_mangle]
pub unsafe extern "C" fn fc4_landvalue_shadow_compare(
    s: *mut fc4_landvalue_shadow,
    orig: *const fc4_landvalue_original,
    julian: i32,
    sink: fc4_bytes_sink,
    ctx: *mut c_void,
) -> i32 {
    let o = orig_of(orig);
    let (n, line) = with_shadow(s, (-1, String::new()), |sh| sh.compare(o, julian));
    if !line.is_empty() {
        emit(&line, sink, ctx);
    }
    n
}

#[no_mangle]
pub unsafe extern "C" fn fc4_landvalue_shadow_get_stats(
    s: *const fc4_landvalue_shadow,
    out: *mut fc4_landvalue_shadow_stats,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    let Some(h) = handle(s) else { return false };
    *o = h.stats.get();
    true
}
