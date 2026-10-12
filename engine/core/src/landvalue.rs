// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. This program is distributed WITHOUT ANY WARRANTY; see
// the GNU General Public License for more details. You should have received a
// copy of the license along with this program (see LICENSE at the repository
// root); if not, see <https://www.gnu.org/licenses/>.

//! The land value simulator (`docs/specs/landvalue.md`), section references
//! `§n` point at that spec.
//!
//! The three published grids (intrinsic, total, slope) are the host's
//! objects (in the game: the game's own `cSC4SimGrid`, simgrid.md §6.3),
//! referred to by token. The core keeps working copies and writes a grid
//! back when the pipeline state that computes it completes [choice]: other
//! code never sees a half-written grid. The scratch grid is core memory
//! only [choice] (it is neither saved nor handed out, §2).
//!
//! Precision: f32 throughout, in the spec's operation order; see `[precision]`
//! tags and simgrid.rs.

use crate::curve::ResponseCurve;
use crate::simgrid::{
    add_distance_effect, process_two_grids, GridGeometry, ScanIterator, CLSID_GRID_F32, CLSID_GRID_U8, IID_SIM_GRID,
};
use crate::stream::{InStream, OutStream, RefValue};

pub const CLSID_LAND_VALUE_SIMULATOR: u32 = 0xE990_C18E;
pub const IID_LAND_VALUE_SIMULATOR: u32 = 0xE778_1D8C;
pub const RECORD_VERSION: u16 = 1;

/// Grid instance IDs (§2).
pub const INSTANCE_INTRINSIC: u32 = 0x89D5_BB39;
pub const INSTANCE_TOTAL: u32 = 0x89D5_BB51;
pub const INSTANCE_SCRATCH: u32 = 0x89D5_BB60;
pub const INSTANCE_SLOPE: u32 = 0x49D5_BB6F;
/// The slope grid's fixed tract size (§2).
pub const SLOPE_TRACT_SIZE: i32 = 8;

/// History series (§7.5).
pub const HISTORY_AVERAGE_LAND_VALUE: u32 = 0x6A66_3AC8;
/// Finished pipeline state (§6).
pub const STATE_FINISHED: u32 = 9;
/// [choice] Tracts handled per millisecond of budget (§6: any deterministic
/// slicing). A 5 ms day step covers a 128 × 128 grid in one call.
pub const TRACTS_PER_MS: u32 = 4096;

/// Messages (§5, §9).
pub mod msg {
    pub const SIM_BEGIN: u32 = 0x6695_6810;
    pub const SIM_NEW_DAY: u32 = 0x6695_6814;
    pub const SIM_NEW_MONTH: u32 = 0x6695_6816;
    pub const SIM_IDLE: u32 = 0x6695_681A;
    pub const INSERT_OCCUPANT: u32 = 0x99EF_1142;
    pub const REMOVE_OCCUPANT: u32 = 0x99EF_1143;
    pub const WATER_HEIGHTS_CHANGED: u32 = 0x678E_6E3A;
    pub const TERRAIN_HEIGHTS_CHANGED: u32 = 0x278E_6003;
    pub const ZONING_CHANGE: u32 = 0x0769_9455;
    /// The five change messages land value subscribes to (§4.1 step 9).
    pub const CHANGES: [u32; 5] = [
        INSERT_OCCUPANT,
        REMOVE_OCCUPANT,
        WATER_HEIGHTS_CHANGED,
        TERRAIN_HEIGHTS_CHANGED,
        ZONING_CHANGE,
    ];
    /// LandValueMapChanged (§9).
    pub const LAND_VALUE_MAP_CHANGED: u32 = 0x87CC_75B3;
    /// Day step budget (§5).
    pub const DAY_BUDGET_MS: u32 = 5;
}

/// Tuning exemplar (§8).
pub mod tuning {
    pub const TYPE: u32 = 0x6534_284A;
    pub const GROUP: u32 = 0xE7E2_C2DB;
    pub const INSTANCE: u32 = 0xE7E2_C8D8;
    pub const TRACT_SIZE: u32 = 0x47E2_C200;
    pub const INTRINSIC: u32 = 0x47E2_C300;
    pub const WEALTH: u32 = 0x47E2_C301;
    pub const WATER: u32 = 0x47E2_C310;
    pub const ALTITUDE_CURVE: u32 = 0x47E2_C320;
    pub const DEVELOPER_IDS: u32 = 0x47E2_C330;
    pub const DEVELOPER_WEIGHTS: u32 = 0x47E2_C331;
    pub const DESIRABILITY_CURVE: u32 = 0x47E2_C332;
}

/// Kind of a grid the core asks the host to create.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridKind {
    U8,
    F32,
}

/// What land value needs from the game (§12). Tokens are host identities.
pub trait LandValueHost {
    /// Terrain `CellCountX()` / `CellCountZ()`; vertices run 0..=cells.
    fn terrain_cells(&self) -> (i32, i32);
    fn is_land_vertex(&self, vx: u32, vz: u32) -> bool;
    fn altitude(&self, px: f32, pz: f32) -> f32;
    fn min_allowed_height(&self) -> f32;
    fn max_allowed_height(&self) -> f32;
    /// Zone manager `GetZoneType(x, z)` (full 32 bits).
    fn zone_type(&self, x: i32, z: i32) -> u32;
    /// Lot developer `GetAreaSlope({x1, z1, x2, z2}, true)` (inclusive).
    fn area_slope(&self, x1: i32, z1: i32, x2: i32, z2: i32) -> f32;
    /// `GetDeveloper(type)`; 0 = none.
    fn developer(&self, developer_type: u32) -> u64;
    /// The developer's `GetDesirabilityMap()` (an int16 grid); 0 = none.
    fn desirability_map(&self, developer: u64) -> u64;

    /// Creates a grid (`AddRef`, `Init`, `SetTractSize`, `SetInstanceID`);
    /// the core owns the returned reference. 0 on failure.
    fn create_grid(&self, kind: GridKind, tract_size: i32, instance_id: u32) -> u64;
    fn grid_geometry(&self, grid: u64) -> GridGeometry;
    fn grid_set_tract_size(&self, grid: u64, tract_size: i32);
    /// Tract values, x outer (`count_x × count_z` of the grid's geometry).
    fn grid_read_u8(&self, grid: u64, out: &mut [u8]);
    fn grid_read_i16(&self, grid: u64, out: &mut [i16]);
    fn grid_write_u8(&self, grid: u64, values: &[u8]);
    fn grid_write_f32(&self, grid: u64, values: &[f32]);
    /// `Shutdown()` on a grid.
    fn grid_shutdown(&self, grid: u64);
    fn release(&self, token: u64);

    fn set_history_queue_length(&self, series: u32, length: u32);
    fn record_history(&self, series: u32, value: i32);
    /// `MessagePost` of a type-only message (§9).
    fn post(&self, msg_type: u32);
}

/// The tuning exemplar's properties (§8). `None` = absent.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TuningInput {
    pub tract_size: Option<i64>,
    pub intrinsic: Option<Vec<i64>>,
    pub wealth: Option<Vec<i64>>,
    pub water: Option<Vec<i64>>,
    pub altitude_curve: Option<Vec<f32>>,
    pub developer_ids: Option<Vec<u32>>,
    pub developer_weights: Option<Vec<f32>>,
    pub desirability_curve: Option<Vec<f32>>,
}

/// The tuning in effect (§8), code defaults first.
#[derive(Clone, Debug, PartialEq)]
pub struct Tuning {
    pub tract_size: u8,
    pub base: u8,
    pub intrinsic_max: u8,
    pub wealth: [u8; 2],
    /// [near value, radius in tracts].
    pub water: [u8; 2],
    pub altitude_curve: Option<ResponseCurve>,
    pub desirability_curve: Option<ResponseCurve>,
}

impl Default for Tuning {
    fn default() -> Self {
        Tuning {
            tract_size: 2,
            base: 20,
            intrinsic_max: 120,
            wealth: [50, 75],
            water: [100, 5],
            altitude_curve: None,
            desirability_curve: None,
        }
    }
}

/// Altitude-effect parameters (§7.3).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AltitudeParams {
    pub min_alt: f32,
    pub pivot: f32,
    pub low_range: f32,
    pub high_range: f32,
    pub top_index: i32,
}

impl AltitudeParams {
    /// Setup at the end of state 3 (§7.3). "not (x > 1)" on purpose: NaN
    /// takes the fallback, as the spec states.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    pub fn setup(mode_alt: f32, mean_alt: f32, min_alt: f32, max_alt: f32, max_allowed: f32) -> AltitudeParams {
        let pivot = if mode_alt < mean_alt { mode_alt } else { mean_alt };
        let mut low_range = pivot - min_alt;
        if !(low_range > 1.0) {
            low_range = 1.0;
        }
        let mut high_range = max_alt - pivot;
        if !(high_range > 1.0) {
            high_range = 1.0;
        }
        let mut top_index = 100;
        if high_range > 100.0 {
            // [precision] the original rounds only (highRange − 100) to f32
            // and evaluates the rest in extended precision.
            let t = (high_range - 100.0) / (max_allowed - 100.0) * 100.0 + 100.0;
            top_index = (t as i32).min(150);
        }
        AltitudeParams {
            min_alt,
            pivot,
            low_range,
            high_range,
            top_index,
        }
    }

    /// `AltitudeEffect(a)` (§7.3); an absent curve reads as 0.
    pub fn effect(&self, curve: Option<&ResponseCurve>, a: f32) -> u8 {
        if a < self.min_alt {
            return 0;
        }
        // [precision] each operation rounded to f32.
        let x = if a >= self.pivot {
            (self.top_index - 50) as f32 / self.high_range * (a - self.pivot) + 50.0
        } else {
            (a - self.min_alt) / self.low_range * 50.0
        };
        let i = x as i32;
        let y = curve.map_or(0.0, |c| c.eval(i as f32));
        (y as i32) as u8 // low 8 bits [quirk]
    }
}

/// One desirability application (§7.4): `clamp(trunc(v + e × weight), 1, 255)`.
pub fn desirability_term(v: u8, d: i16, weight: f32, curve: Option<&ResponseCurve>) -> u8 {
    let e = curve.map_or(f32::from(d), |c| c.eval(f32::from(d)));
    // [precision] e × weight and the sum each rounded to f32.
    let s = f32::from(v) + e * weight;
    (s as i64).clamp(1, 255) as u8
}

/// Diagnostics and conformance view (§3).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LandValueState {
    pub initialized: bool,
    pub dirty: bool,
    pub pipeline_state: u32,
    pub sum: i32,
    pub count: i32,
    pub ignore_zoning: bool,
    pub histogram: Vec<i32>,
    pub hist_shift: i32,
    pub min_alt: f32,
    pub max_alt: f32,
    pub mean_alt: f32,
    pub mode_alt: f32,
    pub altitude: AltitudeParams,
    pub average_land_value: u8,
    pub developers: Vec<(u64, f32)>,
    pub intrinsic: u64,
    pub total: u64,
    pub slope: u64,
}

/// `cSC4LandValueSimulator` (§1).
pub struct LandValueSim {
    host: Box<dyn LandValueHost>,
    initialized: bool,
    dirty: bool,
    state: u32,
    intrinsic: u64,
    total: u64,
    slope: u64,
    /// Geometry of the intrinsic grid (and of scratch) for the current pass.
    geom: GridGeometry,
    slope_geom: GridGeometry,
    scratch: Vec<Vec<u8>>,
    intrinsic_work: Vec<Vec<u8>>,
    slope_work: Vec<Vec<f32>>,
    /// The last total written (state 7); used by state 8.
    total_work: Vec<Vec<u8>>,
    tract_iter: ScanIterator,
    slope_iter: ScanIterator,
    sum: i32,
    count: i32,
    ignore_zoning: bool,
    histogram: Vec<i32>,
    hist_shift: i32,
    min_alt: f32,
    max_alt: f32,
    mean_alt: f32,
    mode_alt: f32,
    alt: AltitudeParams,
    average: u8,
    tuning: Tuning,
    /// Growth developer → weight, in tuning order [choice] (§7.4).
    developers: Vec<(u64, f32)>,
    cursor: usize,
}

fn grid_from(flat: &[u8], g: &GridGeometry) -> Vec<Vec<u8>> {
    let cz = g.count_z.max(0) as usize;
    if cz == 0 {
        return vec![Vec::new(); g.count_x.max(0) as usize];
    }
    flat.chunks(cz).map(<[u8]>::to_vec).collect()
}

fn flatten<T: Copy>(rows: &[Vec<T>]) -> Vec<T> {
    rows.iter().flat_map(|r| r.iter().copied()).collect()
}

fn u8_at(values: &Option<Vec<i64>>, i: usize) -> Option<u8> {
    values.as_ref().and_then(|v| v.get(i)).map(|&x| x as u8)
}

impl LandValueSim {
    pub fn new(host: Box<dyn LandValueHost>) -> Self {
        LandValueSim {
            host,
            initialized: false,
            dirty: false,
            // Not initialised by the original constructor; Init sets 0 (§3).
            state: STATE_FINISHED,
            intrinsic: 0,
            total: 0,
            slope: 0,
            geom: GridGeometry::default(),
            slope_geom: GridGeometry::default(),
            scratch: Vec::new(),
            intrinsic_work: Vec::new(),
            slope_work: Vec::new(),
            total_work: Vec::new(),
            tract_iter: ScanIterator::default(),
            slope_iter: ScanIterator::default(),
            sum: 0,
            count: 0,
            ignore_zoning: false,
            histogram: Vec::new(),
            hist_shift: 5,
            min_alt: 0.0,
            max_alt: 0.0,
            mean_alt: 0.0,
            mode_alt: 0.0,
            alt: AltitudeParams::default(),
            average: 0,
            tuning: Tuning::default(),
            developers: Vec::new(),
            cursor: 0,
        }
    }

    pub fn host(&self) -> &dyn LandValueHost {
        self.host.as_ref()
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn tuning(&self) -> &Tuning {
        &self.tuning
    }

    pub fn intrinsic_grid(&self) -> u64 {
        self.intrinsic
    }

    pub fn total_grid(&self) -> u64 {
        self.total
    }

    pub fn slope_grid(&self) -> u64 {
        self.slope
    }

    pub fn pipeline_state(&self) -> u32 {
        self.state
    }

    pub fn average_land_value(&self) -> u8 {
        self.average
    }

    pub fn mode_altitude(&self) -> f32 {
        self.mode_alt
    }

    /// The scratch grid (diagnostics and conformance).
    pub fn scratch(&self) -> &[Vec<u8>] {
        &self.scratch
    }

    pub fn snapshot(&self) -> LandValueState {
        LandValueState {
            initialized: self.initialized,
            dirty: self.dirty,
            pipeline_state: self.state,
            sum: self.sum,
            count: self.count,
            ignore_zoning: self.ignore_zoning,
            histogram: self.histogram.clone(),
            hist_shift: self.hist_shift,
            min_alt: self.min_alt,
            max_alt: self.max_alt,
            mean_alt: self.mean_alt,
            mode_alt: self.mode_alt,
            altitude: self.alt,
            average_land_value: self.average,
            developers: self.developers.clone(),
            intrinsic: self.intrinsic,
            total: self.total,
            slope: self.slope,
        }
    }

    /// Conformance and diagnostics: override statistics (§7.2/§7.3 inputs).
    pub fn set_altitude_stats(&mut self, min_alt: f32, max_alt: f32, mode_alt: f32, mean_alt: f32) {
        self.min_alt = min_alt;
        self.max_alt = max_alt;
        self.mode_alt = mode_alt;
        self.mean_alt = mean_alt;
    }

    /// Conformance and diagnostics: override persisted and pipeline fields.
    pub fn debug_set(&mut self, dirty: Option<bool>, state: Option<u32>, average: Option<u8>, mean_alt: Option<f32>) {
        if let Some(d) = dirty {
            self.dirty = d;
        }
        if let Some(s) = state {
            self.state = s;
        }
        if let Some(a) = average {
            self.average = a;
        }
        if let Some(m) = mean_alt {
            self.mean_alt = m;
        }
    }

    // ---- tuning (§8) ---------------------------------------------------------------------------

    /// Applies the exemplar (`None` = missing: nothing changes).
    pub fn apply_tuning(&mut self, t: Option<&TuningInput>) {
        let Some(t) = t else { return };
        let tu = &mut self.tuning;
        if let Some(s) = t.tract_size {
            tu.tract_size = s as u8;
        }
        // [choice] absent required properties (or short arrays) keep the
        // current values (the original dereferences them unchecked).
        if let Some(v) = u8_at(&t.intrinsic, 0) {
            tu.base = v;
        }
        if let Some(v) = u8_at(&t.intrinsic, 1) {
            tu.intrinsic_max = v;
        }
        for i in 0..2 {
            if let Some(v) = u8_at(&t.wealth, i) {
                tu.wealth[i] = v;
            }
            if let Some(v) = u8_at(&t.water, i) {
                tu.water[i] = v;
            }
        }
        // Curves: present → (re)built, absent → deleted (foundations §4.1).
        tu.altitude_curve = t.altitude_curve.as_deref().and_then(ResponseCurve::from_flat);
        tu.desirability_curve = t.desirability_curve.as_deref().and_then(ResponseCurve::from_flat);
        if let (Some(ids), Some(weights)) = (&t.developer_ids, &t.developer_weights) {
            // [choice] min(count of IDs, count of weights) entries.
            for (&id, &w) in ids.iter().zip(weights.iter()) {
                let dev = self.host.developer(id);
                // Later duplicates overwrite an earlier identical key.
                if let Some(e) = self.developers.iter_mut().find(|e| e.0 == dev) {
                    e.1 = w;
                } else {
                    self.developers.push((dev, w));
                }
            }
        }
    }

    // ---- lifecycle (§4) --------------------------------------------------------------------------

    /// The core part of `Init` (§4.1 steps 2–7, 10). The GZCOM object takes
    /// the services, registers the agents and subscribes to the messages.
    pub fn init(&mut self, tuning: Option<&TuningInput>) -> bool {
        if self.initialized {
            return true;
        }
        self.apply_tuning(tuning);
        let s = i32::from(self.tuning.tract_size);
        self.intrinsic = self.ensure_grid(self.intrinsic, GridKind::U8, s, INSTANCE_INTRINSIC);
        self.total = self.ensure_grid(self.total, GridKind::U8, s, INSTANCE_TOTAL);
        self.slope = self.ensure_grid(self.slope, GridKind::F32, SLOPE_TRACT_SIZE, INSTANCE_SLOPE);
        self.state = 0;
        self.geom = self.host.grid_geometry(self.intrinsic);
        self.slope_geom = self.host.grid_geometry(self.slope);
        self.scratch = vec![vec![0; self.geom.count_z.max(0) as usize]; self.geom.count_x.max(0) as usize];
        self.tract_iter = ScanIterator::for_grid(&self.geom);
        self.slope_iter = ScanIterator::for_grid(&self.slope_geom);
        self.hist_shift = 5;
        let bins = (self.host.max_allowed_height() as i32) >> self.hist_shift;
        self.histogram.resize(bins.max(0) as usize, 0);
        self.host.set_history_queue_length(HISTORY_AVERAGE_LAND_VALUE, 1);
        self.initialized = true;
        true
    }

    /// Creates a missing grid, or [choice] resizes a restored one whose tract
    /// size differs from the tuning (§14).
    fn ensure_grid(&self, token: u64, kind: GridKind, s: i32, instance: u32) -> u64 {
        if token == 0 {
            return self.host.create_grid(kind, s, instance);
        }
        if self.host.grid_geometry(token).tract_size != s {
            self.host.grid_set_tract_size(token, s);
        }
        token
    }

    /// The core part of `Shutdown` (§4.2): grids shut down and released,
    /// curves deleted; the developer list, dirty flag, state and statistics
    /// are kept [quirk].
    pub fn shutdown(&mut self) -> bool {
        if !self.initialized {
            return true;
        }
        self.initialized = false;
        for t in [self.intrinsic, self.total, self.slope] {
            if t != 0 {
                self.host.grid_shutdown(t);
                self.host.release(t);
            }
        }
        self.intrinsic = 0;
        self.total = 0;
        self.slope = 0;
        self.scratch.clear();
        self.tuning.altitude_curve = None;
        self.tuning.desirability_curve = None;
        true
    }

    /// Drops the grid references without the shutdown protocol (object
    /// destruction after a load that was never initialised).
    pub fn release_grids(&mut self) {
        for t in [self.intrinsic, self.total, self.slope] {
            if t != 0 {
                self.host.release(t);
            }
        }
        self.intrinsic = 0;
        self.total = 0;
        self.slope = 0;
    }

    // ---- messages (§5) ------------------------------------------------------------------------------

    /// `DoMessage`: always true.
    pub fn handle_message(&mut self, msg_type: u32, data3: i64) -> bool {
        match msg_type {
            msg::SIM_NEW_DAY => self.step(msg::DAY_BUDGET_MS),
            msg::SIM_NEW_MONTH => {
                if self.dirty && self.state == STATE_FINISHED {
                    self.dirty = false;
                    self.state = 0;
                }
            }
            msg::SIM_IDLE => self.step(data3.clamp(0, i64::from(u32::MAX)) as u32),
            t if msg::CHANGES.contains(&t) => self.dirty = true,
            _ => {}
        }
        true
    }

    // ---- predicates (§2) ------------------------------------------------------------------------------

    fn has_land(&self, tx: i32, tz: i32) -> bool {
        let s = self.geom.tract_size;
        let (cx, cz) = self.host.terrain_cells();
        for vx in tx * s..=tx * s + s {
            for vz in tz * s..=tz * s + s {
                // Vertices outside the terrain are not land.
                if vx < 0 || vz < 0 || vx > cx || vz > cz {
                    continue;
                }
                if self.host.is_land_vertex(vx as u32, vz as u32) {
                    return true;
                }
            }
        }
        false
    }

    fn is_zoned(&self, tx: i32, tz: i32) -> bool {
        let s = self.geom.tract_size;
        for x in tx * s..tx * s + s {
            for z in tz * s..tz * s + s {
                if self.host.zone_type(x, z) != 0 {
                    return true;
                }
            }
        }
        false
    }

    fn tract_altitude(&self, tx: i32, tz: i32) -> f32 {
        let (px, pz) = self.geom.tract_center(tx, tz);
        self.host.altitude(px, pz)
    }

    fn counted(&self, tx: i32, tz: i32) -> bool {
        (self.ignore_zoning || self.is_zoned(tx, tz)) && self.has_land(tx, tz)
    }

    // ---- the pipeline (§6) --------------------------------------------------------------------------

    /// `Step(budget)`: completes at most one state.
    pub fn step(&mut self, budget_ms: u32) {
        let mut work = budget_ms.saturating_mul(TRACTS_PER_MS).max(1);
        match self.state {
            0 => {
                self.geom = self.host.grid_geometry(self.intrinsic);
                self.slope_geom = self.host.grid_geometry(self.slope);
                self.tract_iter = ScanIterator::for_grid(&self.geom);
                self.slope_iter = ScanIterator::for_grid(&self.slope_geom);
                self.tract_iter.reset(0, 0);
                self.scratch = vec![vec![0; self.geom.count_z.max(0) as usize]; self.geom.count_x.max(0) as usize];
                self.state = 1;
            }
            1 => {
                let base = self.tuning.base;
                while let Some((x, z)) = self.next_tract(&mut work) {
                    let v = if self.has_land(x, z) { base } else { 0 };
                    self.scratch[x as usize][z as usize] = v;
                }
                if self.tract_iter.state == crate::simgrid::ScanState::Done {
                    self.state = 2;
                }
            }
            2 => {
                let (cx, cz) = (self.geom.count_x, self.geom.count_z);
                let radius = f32::from(self.tuning.water[1]);
                let near = f32::from(self.tuning.water[0]);
                add_distance_effect(&mut self.scratch, cx, cz, None, 0, radius, near, 0.0);
                // Deliberately inverted start values (§7.2).
                self.min_alt = self.host.max_allowed_height();
                self.max_alt = self.host.min_allowed_height();
                self.tract_iter.reset(0, 0);
                self.sum = 0;
                self.count = 0;
                self.ignore_zoning = false;
                self.histogram.fill(0);
                self.state = 3;
            }
            3 => self.state3(&mut work),
            5 => {
                let s = self.slope_geom.tract_size;
                if self.slope_work.len() != self.slope_geom.count_x.max(0) as usize
                    || self.slope_work.first().map_or(0, Vec::len) != self.slope_geom.count_z.max(0) as usize
                {
                    self.slope_work = vec![
                        vec![0.0; self.slope_geom.count_z.max(0) as usize];
                        self.slope_geom.count_x.max(0) as usize
                    ];
                }
                while work > 0 {
                    let Some((sx, sz)) = self.slope_iter.next() else { break };
                    work -= 1;
                    let v = self.host.area_slope(sx * s, sz * s, sx * s + s - 1, sz * s + s - 1);
                    if let Some(c) = self
                        .slope_work
                        .get_mut(sx as usize)
                        .and_then(|r| r.get_mut(sz as usize))
                    {
                        *c = v;
                    }
                }
                if self.slope_iter.state == crate::simgrid::ScanState::Done {
                    self.host.grid_write_f32(self.slope, &flatten(&self.slope_work));
                    self.tract_iter.reset(0, 0);
                    self.state = 4;
                }
            }
            4 => {
                if self.tuning.altitude_curve.is_none() {
                    // The intrinsic grid keeps its previous contents [quirk].
                    self.tract_iter.reset(0, 0);
                    self.state = 6;
                    return;
                }
                if self.intrinsic_work.len() != self.scratch.len()
                    || self.intrinsic_work.first().map(Vec::len) != self.scratch.first().map(Vec::len)
                {
                    self.intrinsic_work = self.scratch.clone();
                }
                while let Some((x, z)) = self.next_tract(&mut work) {
                    let a = self.tract_altitude(x, z);
                    let e = self.alt.effect(self.tuning.altitude_curve.as_ref(), a);
                    let v = (u32::from(self.scratch[x as usize][z as usize]) + u32::from(e)).min(255) as u8;
                    self.intrinsic_work[x as usize][z as usize] = v;
                }
                if self.tract_iter.state == crate::simgrid::ScanState::Done {
                    self.host.grid_write_u8(self.intrinsic, &flatten(&self.intrinsic_work));
                    self.tract_iter.reset(0, 0);
                    self.state = 6;
                }
            }
            6 => {
                // scratch := copy of intrinsic, dimensions included.
                self.geom = self.host.grid_geometry(self.intrinsic);
                let mut flat = vec![0u8; self.geom.len()];
                self.host.grid_read_u8(self.intrinsic, &mut flat);
                self.scratch = grid_from(&flat, &self.geom);
                self.cursor = 0;
                self.state = 7;
            }
            7 => {
                if self.cursor < self.developers.len() {
                    let (dev, weight) = self.developers[self.cursor];
                    self.cursor += 1;
                    self.apply_developer(dev, weight);
                    return;
                }
                self.host.grid_write_u8(self.total, &flatten(&self.scratch));
                self.total_work = self.scratch.clone();
                self.tract_iter.reset(0, 0);
                self.sum = 0;
                self.count = 0;
                self.state = 8;
            }
            8 => {
                while let Some((x, z)) = self.next_tract(&mut work) {
                    if self.counted(x, z) {
                        let v = self
                            .total_work
                            .get(x as usize)
                            .and_then(|r| r.get(z as usize))
                            .copied()
                            .unwrap_or(0);
                        self.sum = self.sum.wrapping_add(i32::from(v));
                        self.count += 1;
                    }
                }
                if self.tract_iter.state == crate::simgrid::ScanState::Done {
                    self.average = if self.count > 0 {
                        (self.sum / self.count) as u8
                    } else {
                        0
                    };
                    self.host
                        .record_history(HISTORY_AVERAGE_LAND_VALUE, i32::from(self.average));
                    self.host.post(msg::LAND_VALUE_MAP_CHANGED);
                    self.state = STATE_FINISHED;
                }
            }
            _ => {}
        }
    }

    /// The next tract of the tract iterator within the work budget.
    fn next_tract(&mut self, work: &mut u32) -> Option<(i32, i32)> {
        if *work == 0 {
            return None;
        }
        let t = self.tract_iter.next()?;
        *work -= 1;
        // Guard against grids that are smaller than their geometry claims.
        if self
            .scratch
            .get(t.0 as usize)
            .and_then(|r| r.get(t.1 as usize))
            .is_none()
        {
            return self.next_tract(work);
        }
        Some(t)
    }

    fn state3(&mut self, work: &mut u32) {
        while let Some((x, z)) = self.next_tract(work) {
            let a = self.tract_altitude(x, z);
            let land = self.has_land(x, z);
            if land {
                if a < self.min_alt {
                    self.min_alt = a;
                }
                if a > self.max_alt {
                    self.max_alt = a;
                }
            }
            if (self.ignore_zoning || self.is_zoned(x, z)) && land {
                // [precision] int32 sum added as a float and truncated: f32
                // loses integers above 2^24 (an extended-precision x87 does not).
                self.sum = (self.sum as f32 + a) as i32;
                self.count += 1;
                if !self.histogram.is_empty() {
                    let bin = ((a as i32) >> self.hist_shift).clamp(0, self.histogram.len() as i32 - 1);
                    self.histogram[bin as usize] += 1;
                }
            }
        }
        if self.tract_iter.state != crate::simgrid::ScanState::Done {
            return;
        }
        if self.count == 0 && !self.ignore_zoning {
            // Zoning fallback: recount every land tract (stay in state 3).
            self.ignore_zoning = true;
            self.tract_iter.reset(0, 0);
            self.sum = 0;
            self.count = 0;
            self.histogram.fill(0);
            return;
        }
        // [precision] f32 division of the int32 sum.
        self.mean_alt = if self.count > 0 {
            self.sum as f32 / self.count as f32
        } else {
            0.0
        };
        let mut best = 0usize;
        for (i, &n) in self.histogram.iter().enumerate() {
            if n > self.histogram[best] {
                best = i;
            }
        }
        let mut mode = ((1i32 << (self.hist_shift - 1)) + best as i32 * (1i32 << self.hist_shift)) as f32;
        if mode < self.min_alt {
            mode = self.min_alt;
        }
        if mode > self.max_alt {
            mode = self.max_alt;
        }
        self.mode_alt = mode;
        self.setup_altitude();
        self.slope_iter.reset(0, 0);
        self.state = 5;
    }

    /// The altitude-effect setup (§7.3) from the current statistics.
    pub fn setup_altitude(&mut self) {
        self.alt = AltitudeParams::setup(
            self.mode_alt,
            self.mean_alt,
            self.min_alt,
            self.max_alt,
            self.host.max_allowed_height(),
        );
    }

    /// `AltitudeEffect(a)` with the current parameters and curve.
    pub fn altitude_effect(&self, a: f32) -> u8 {
        self.alt.effect(self.tuning.altitude_curve.as_ref(), a)
    }

    fn apply_developer(&mut self, dev: u64, weight: f32) {
        // A null developer would crash the original [quirk]; skipped [choice].
        if dev == 0 {
            return;
        }
        let grid = self.host.desirability_map(dev);
        if grid == 0 {
            return;
        }
        let bg = self.host.grid_geometry(grid);
        let mut flat = vec![0i16; bg.len()];
        self.host.grid_read_i16(grid, &mut flat);
        let cz = bg.count_z.max(0) as usize;
        let b: Vec<Vec<i16>> = if cz == 0 {
            Vec::new()
        } else {
            flat.chunks(cz).map(<[i16]>::to_vec).collect()
        };
        let curve = self.tuning.desirability_curve.clone();
        let geom = self.geom;
        process_two_grids(&geom, &mut self.scratch, &bg, &b, |v, d| {
            *v = desirability_term(*v, d, weight, curve.as_ref());
        });
    }

    // ---- queries (§7.6) -------------------------------------------------------------------------------

    /// `GetLandValueType` for a land value `v`: 1 low, 2 medium, 3 high.
    pub fn land_value_type(&self, v: u8) -> u32 {
        if v < self.tuning.wealth[0] {
            1
        } else if v < self.tuning.wealth[1] {
            2
        } else {
            3
        }
    }

    // ---- persistence (§10) ------------------------------------------------------------------------------

    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        let r = |t: u64, class_id: u32| {
            if t == 0 {
                RefValue::Null
            } else {
                RefValue::Host { token: t, class_id }
            }
        };
        out.put_u16(RECORD_VERSION);
        out.put_ref(&r(self.intrinsic, CLSID_GRID_U8));
        out.put_ref(&r(self.total, CLSID_GRID_U8));
        out.put_ref(&r(self.slope, CLSID_GRID_F32));
        out.put_u8(u8::from(self.dirty || self.state != STATE_FINISHED));
        out.put_u8(self.average);
        out.put_f32(self.mean_alt);
        out.error() == 0
    }

    /// `Read`: grids arrive with one reference each; the pipeline state is
    /// left alone (the following `Init` sets 0).
    pub fn read(&mut self, inp: &mut dyn InStream) -> bool {
        if inp.get_u16() != Some(RECORD_VERSION) {
            return false;
        }
        let mut grids = [0u64; 3];
        for g in &mut grids {
            *g = match inp.get_ref_as(IID_SIM_GRID) {
                Some(RefValue::Host { token, .. }) => token,
                _ => 0,
            };
        }
        // [choice] grids held before the read are given back.
        for (old, new) in [self.intrinsic, self.total, self.slope].into_iter().zip(grids) {
            if old != 0 && old != new {
                self.host.release(old);
            }
        }
        [self.intrinsic, self.total, self.slope] = grids;
        if let Some(d) = inp.get_u8() {
            self.dirty = d != 0;
        }
        if let Some(a) = inp.get_u8() {
            self.average = a;
        }
        if let Some(m) = inp.get_f32() {
            self.mean_alt = m;
        }
        inp.error() == 0
    }
}
