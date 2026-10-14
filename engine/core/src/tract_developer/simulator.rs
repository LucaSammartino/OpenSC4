// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The tract developer's state and behaviour (§1–§6).

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use super::tuning::Tuning;
use super::{
    lot_state, network, BuildingProfile, Cap, CellRect, Cheat, ConstructionProfile, GrowthProfile, Outcome,
    TractDeveloperHost, ABANDON_DECLINE, ABANDON_KICK_OUT, AGENT_LEVELS, ANCHOR_GROUP, MSG_YEAR,
};
use crate::development::{building_fits_in_zone, dev_type, growth_stage_fits_in_zone};
use crate::grid::CellIterator;
use crate::rng::{CrtRand, FixedSeeds, RzRandom, SeedSource};
use crate::stl::{stl_sort, StlHashSet};
use crate::stream::{InStream, OutStream};
use crate::x87::{ftol32, r24, rhe};

/// Save record version (§6).
pub const RECORD_VERSION: u16 = 3;
/// Histogram size (§4.1): 13 rows × 28 codes.
pub const HIST_ROWS: usize = 13;
pub const HIST_COLUMNS: usize = 28;
/// [choice] The largest style list a save read allocates.
pub const MAX_SAVED_STYLES: usize = 1 << 16;
/// [choice] The longest neighbour list `BuildFarm` builds.
pub const MAX_FARM_NEIGHBOURS: usize = 1 << 16;

/// Histogram rows (§4.1).
mod row {
    pub const GROW: usize = 0;
    pub const TAKE_OVER: usize = 1;
    pub const LIST_CONFIGURATIONS: usize = 2;
    pub const EXISTING: usize = 3;
    pub const AGGREGATION: usize = 4;
    pub const SUBDIVISION: usize = 5;
    pub const BUILD: usize = 6;
    pub const CONFIGURATION_SUITABLE: usize = 7;
    pub const LOT_SUITABLE: usize = 8;
    pub const AGGREGATION_LEGAL: usize = 9;
    pub const PICK_BUILDING: usize = 10;
    pub const CONFIGURATION_COMPATIBLE: usize = 11;
    pub const BUILDING_COMPATIBLE: usize = 12;
}

/// Outcome codes as integers (§4.1).
mod code {
    pub const UNKNOWN: u32 = 0;
    pub const NOTHING_TO_REOCCUPY: u32 = 1;
    pub const MONOPOLY: u32 = 2;
    pub const NO_LOT: u32 = 3;
    pub const LOT_NOT_READY: u32 = 4;
    pub const NO_CONFIGURATION: u32 = 5;
    pub const WRONG_ZONE: u32 = 6;
    pub const WRONG_PURPOSE: u32 = 7;
    pub const WRONG_WEALTH: u32 = 8;
    pub const WRONG_STYLE: u32 = 9;
    pub const TOO_STEEP: u32 = 10;
    pub const BUILDING_TOO_SMALL: u32 = 11;
    pub const STAGE_TOO_SMALL: u32 = 12;
    pub const BUILDING_TOO_BIG: u32 = 13;
    pub const STAGE_TOO_BIG: u32 = 14;
    pub const DIFFERENT_PURPOSE: u32 = 15;
    pub const DIFFERENT_WEALTH: u32 = 16;
    pub const NOT_BIGGER: u32 = 17;
    pub const HISTORICAL: u32 = 18;
    pub const JUST_BUILT: u32 = 19;
    pub const NO_RESOURCES: u32 = 20;
    pub const POOR_ROADS: u32 = 21;
    pub const WRONG_LAND_VALUE: u32 = 22;
    pub const NO_FIRE: u32 = 23;
    pub const NO_PARK: u32 = 24;
    pub const NO_WATER: u32 = 25;
    pub const NO_POWER: u32 = 26;
    pub const SUCCESS: u32 = 27;
}

/// Which random draw of §4.22 a [`Draw`] record is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawKind {
    /// `ListExistingLots` iterator reset: one `rand()`.
    R1,
    /// `ScoreCandidateLots` industrial term: `RandomDoubleUniform`.
    R2,
    /// `PickBuilding` draw A: `RandomUint32Uniform(100)`.
    R3,
    /// `PickBuilding` draw B: `RandomUint32Uniform(|S_cfg|)`.
    R4,
    /// `PickBuilding` draw C: `RandomUint32Uniform(|S27|)`.
    R5,
    /// `BuildFarm` field type: `RandomUint32Uniform(count)`.
    R6,
    /// `UpdatePopulation`: `RandomDoubleRangeUniform` of the temporary
    /// generator.
    R7,
    /// `Decline` iterator reset: one `rand()`.
    R8,
    /// `CreateLotIfPossible` with facing −1 (`PerformAggregation`,
    /// §4.15.3): `RandomUint32Uniform(4)` on the zone developer's
    /// generator, inside the host. Recorded by the host, never by the core.
    R9,
}

impl DrawKind {
    pub fn name(self) -> &'static str {
        match self {
            DrawKind::R1 => "R1",
            DrawKind::R2 => "R2",
            DrawKind::R3 => "R3",
            DrawKind::R4 => "R4",
            DrawKind::R5 => "R5",
            DrawKind::R6 => "R6",
            DrawKind::R7 => "R7",
            DrawKind::R8 => "R8",
            DrawKind::R9 => "R9",
        }
    }
}

/// One random draw (the debug trace of [`TractDeveloperStatics`]): the
/// argument (`n` of `RandomUint32Uniform(n)`, the cell count of an
/// iterator reset, 0 otherwise) and the value drawn (the `rand()` result
/// for R1 / R8).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Draw {
    pub kind: DrawKind,
    pub arg: u32,
    pub value: f64,
}

/// The class-wide statics (§9.1) shared by every tract developer.
#[derive(Clone, Debug)]
pub struct TractDeveloperStatics {
    /// The file-local `cRZRandom` (`0x00B4AFD8`, §4.22).
    pub rng: RzRandom,
    /// The C runtime `rand()` the iterator resets draw from.
    pub crt: CrtRand,
    /// The random cell iterator of `ListExistingLots` (`0x00B4B098`).
    pub list_iterator: CellIterator,
    /// The random cell iterator of `Decline` (`0x00B4B0CC`).
    pub decline_iterator: CellIterator,
    /// Every draw, when tracing is on (debug and conformance).
    pub trace: Option<Vec<Draw>>,
}

/// The statics as one object handed to every instance.
pub type SharedStatics = Rc<RefCell<TractDeveloperStatics>>;

impl TractDeveloperStatics {
    /// The statics with the file-local generator seeded with `seed`
    /// (supplied by the host: the core does not derive it, §4.22) and
    /// `rand()` at its process-start state 1. The original's seed is the
    /// local wall clock XOR [`super::RNG_SEED_XOR`]; a clock value of 0 gives
    /// `0x4C09C113`.
    pub fn new(seed: u32, seeds: &dyn SeedSource) -> SharedStatics {
        Rc::new(RefCell::new(TractDeveloperStatics {
            rng: RzRandom::new(seed, seeds),
            crt: CrtRand::default(),
            list_iterator: CellIterator::new(0, 0, -1, -1),
            decline_iterator: CellIterator::new(0, 0, -1, -1),
            trace: None,
        }))
    }

    /// The statics with the file-local generator seeded from the session
    /// seed (foundations §6.3 *[choice]*): `derive_seed(session,
    /// 0x4C09C113)`, the original's clock XOR [`super::RNG_SEED_XOR`] with
    /// the session seed in place of the clock. `rand()` stays at state 1
    /// (the game never seeds it, foundations §12).
    pub fn from_session(session: u32, seeds: &dyn SeedSource) -> SharedStatics {
        TractDeveloperStatics::new(crate::rng::derive_seed(session, super::RNG_SEED_XOR), seeds)
    }

    /// Appends a draw to the trace (when tracing is on). The core records
    /// R1–R8; a host may record R9, the zone developer's draw inside
    /// `CreateLotIfPossible` (§4.15.3), to show where it falls.
    pub fn record(&mut self, kind: DrawKind, arg: u32, value: f64) {
        if let Some(t) = self.trace.as_mut() {
            t.push(Draw { kind, arg, value });
        }
    }
}

/// A candidate record (§2.3).
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    /// 1 reoccupy, 2 existing lot, 3 aggregation, 4 subdivision.
    pub kind: u8,
    pub lot: Option<u64>,
    pub x: i32,
    pub z: i32,
    /// −1: take facing, size, location and rectangle from the lot.
    pub facing: i32,
    pub w: u8,
    pub h: u8,
    pub lot_set: BTreeSet<u64>,
    pub existing: i32,
    pub abandoned: i32,
    pub stage: u32,
    pub wealth: u32,
    /// A float32 value.
    pub score: f64,
    pub rect: CellRect,
    pub n: i32,
    pub street: [i32; 4],
    pub road: [i32; 4],
    pub avenue: [i32; 4],
    pub rail: [i32; 4],
    pub corner: bool,
    pub capacity: i32,
}

impl Default for Candidate {
    /// The template (§2.3, `0x00707680`).
    fn default() -> Candidate {
        Candidate {
            kind: 0,
            lot: None,
            x: -1,
            z: -1,
            facing: -1,
            w: 0,
            h: 0,
            lot_set: BTreeSet::new(),
            existing: -1,
            abandoned: -1,
            stage: 0,
            wealth: 0,
            score: -1.0,
            rect: CellRect::new(-1, -1, -1, -1),
            n: 0,
            street: [0; 4],
            road: [0; 4],
            avenue: [0; 4],
            rail: [0; 4],
            corner: false,
            capacity: 0,
        }
    }
}

/// [inferred] The target zone type a zone type converts to (§4.4): 1–3 →
/// 1, 4–6 → 2, 7–9 → 3, others → 0.
pub fn zone_family(zone: i32) -> i32 {
    match zone {
        1..=3 => 1,
        4..=6 => 2,
        7..=9 => 3,
        _ => 0,
    }
}

/// The configuration filter's `IsLotConfigurationIncluded(cfg)` (§4.6,
/// §7.4): compatible with the purpose (slot 17) **and** the wealth (slot
/// 16), in that order.
pub fn configuration_filter_includes(host: &dyn TractDeveloperHost, cfg: u64, wealth: u32, purpose: u32) -> bool {
    host.cfg_compatible_purpose(cfg, purpose) && host.cfg_compatible_wealth(cfg, wealth)
}

/// The per-attempt working state (§2.2).
#[derive(Clone, Debug, Default)]
struct Work {
    dev: u64,
    tx: i32,
    tz: i32,
    max_stage: u32,
    purpose: u32,
    info: ConstructionProfile,
    wealth: u32,
    target_zone: i32,
    sub_type: u32,
    existing_capacity: i32,
    desired: i32,
    desired_max: i32,
    built: i32,
    relaxed: bool,
    out_flag: bool,
    lots: BTreeSet<u64>,
    cfgs: BTreeSet<u64>,
    candidates: Vec<Candidate>,
    suitability: BTreeMap<u64, u32>,
    chosen_cfg: Option<u64>,
    chosen_building: u32,
}

/// One tract developer (§1).
pub struct TractDeveloper {
    host: Rc<dyn TractDeveloperHost>,
    statics: SharedStatics,
    initialised: bool,
    configured: bool,
    tuning: Tuning,
    active_styles: Vec<u32>,
    current_style_index: i32,
    years_since_style_change: i32,
    single_style: bool,
    years_between_style_changes: i32,
    filters: BTreeMap<(u32, u32), u64>,
    histogram_enabled: bool,
    histogram: Vec<[u32; HIST_COLUMNS]>,
    w: Work,
    /// Candidates scored by the last `grow` (§4.14; diagnostics).
    scored: Cell<u32>,
}

fn rect_wh(r: CellRect) -> (i32, i32) {
    (
        r.x2.wrapping_sub(r.x1).wrapping_add(1),
        r.z2.wrapping_sub(r.z1).wrapping_add(1),
    )
}

fn rect_overlaps(a: CellRect, b: CellRect) -> bool {
    a.x1 <= b.x2 && b.x1 <= a.x2 && a.z1 <= b.z2 && b.z1 <= a.z2
}

fn rect_inside(inner: CellRect, outer: CellRect) -> bool {
    inner.x1 >= outer.x1 && inner.x2 <= outer.x2 && inner.z1 >= outer.z1 && inner.z2 <= outer.z2
}

fn clamp_i32(v: i64) -> i32 {
    v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// The tract's cell rectangle for a tract size (§4.5).
fn tract_rect(tx: i32, tz: i32, size: i32) -> CellRect {
    let s = i64::from(size);
    let x1 = i64::from(tx) * s;
    let z1 = i64::from(tz) * s;
    CellRect::new(
        clamp_i32(x1),
        clamp_i32(z1),
        clamp_i32(x1 + s - 1),
        clamp_i32(z1 + s - 1),
    )
}

impl TractDeveloper {
    /// The constructor's state (§2.1): no styles, index 0, single style,
    /// years 0, the constructor's tuning.
    pub fn new(host: Rc<dyn TractDeveloperHost>, statics: SharedStatics) -> TractDeveloper {
        TractDeveloper {
            host,
            statics,
            initialised: false,
            configured: false,
            tuning: Tuning::constructor(),
            active_styles: Vec::new(),
            current_style_index: 0,
            years_since_style_change: 0,
            single_style: true,
            years_between_style_changes: 0,
            filters: BTreeMap::new(),
            histogram_enabled: false,
            histogram: vec![[0; HIST_COLUMNS]; HIST_ROWS],
            w: Work::default(),
            scored: Cell::new(0),
        }
    }

    pub fn statics(&self) -> SharedStatics {
        self.statics.clone()
    }

    pub fn tuning(&self) -> &Tuning {
        &self.tuning
    }

    /// Replaces the tuning (tests and hosts that read it themselves).
    pub fn set_tuning(&mut self, t: Tuning) {
        self.tuning = t;
    }

    pub fn is_initialised(&self) -> bool {
        self.initialised
    }

    /// Whether `ReadTuning` ran (the class-wide "configured" flag).
    pub fn is_configured(&self) -> bool {
        self.configured
    }

    /// The registered configuration filters, `(wealth, purpose) → filter`.
    pub fn filters(&self) -> &BTreeMap<(u32, u32), u64> {
        &self.filters
    }

    // ---- histogram (§4.1) ----

    /// The debug switch of the histogram [choice].
    pub fn set_histogram_enabled(&mut self, on: bool) {
        self.histogram_enabled = on;
    }

    /// The histogram of the last `grow` (13 rows of 28 counters).
    pub fn histogram(&self) -> &[[u32; HIST_COLUMNS]] {
        &self.histogram
    }

    fn hist(&mut self, row: usize, code: u32) {
        if !self.histogram_enabled {
            return;
        }
        if let Some(c) = self.histogram.get_mut(row).and_then(|r| r.get_mut(code as usize)) {
            *c = c.wrapping_add(1);
        }
    }

    // ---- lifecycle (§1.3, §1.4, §3) ----

    /// `Init` (§1.3).
    pub fn init(&mut self) -> bool {
        if self.initialised {
            return true;
        }
        self.configured = true;
        self.read_tuning();
        if !self.host.acquire_services() {
            return false;
        }
        for (level, msg) in AGENT_LEVELS {
            if !self.host.add_agent(level, msg) {
                return false;
            }
        }
        if !self.host.acquire_message_server() {
            return false;
        }
        self.initialised = true;
        true
    }

    /// `ReadTuning` (§4.21) and the style fix-ups that follow it.
    pub fn read_tuning(&mut self) {
        let h = self.host.tuning();
        self.tuning.read(h.as_deref(), &mut self.years_between_style_changes);
        if self.active_styles.is_empty() {
            self.current_style_index = 0;
            self.active_styles = self.tuning.available_styles.clone();
        }
        if self.current_style_index as u32 as usize >= self.active_styles.len() {
            self.current_style_index = 0;
        }
    }

    /// `Shutdown` (§1.4): returns true. The style state is kept.
    pub fn shutdown(&mut self) -> bool {
        if self.initialised {
            self.initialised = false;
            self.host.release_message_server();
            self.host.remove_agent();
            for (_, f) in std::mem::take(&mut self.filters) {
                self.host.unregister_configuration_filter(f);
            }
            self.host.release_services();
            self.w = Work::default();
        }
        true
    }

    /// `DoMessage` (§3): Year runs `NewYear`; always true.
    pub fn do_message(&mut self, msg: u32) -> bool {
        if msg == MSG_YEAR {
            self.new_year();
        }
        true
    }

    // ---- styles (§4.20) ----

    /// `NewYear` (§4.20).
    pub fn new_year(&mut self) {
        self.years_since_style_change = self.years_since_style_change.wrapping_add(1);
        if self.years_since_style_change >= self.years_between_style_changes {
            self.current_style_index = self.current_style_index.wrapping_add(1);
            self.years_since_style_change = 0;
            if self.current_style_index as u32 as usize >= self.active_styles.len() {
                self.current_style_index = 0;
            }
        }
    }

    fn style_at_index(&self) -> Option<u32> {
        usize::try_from(self.current_style_index)
            .ok()
            .and_then(|i| self.active_styles.get(i).copied())
    }

    /// `GetCurrentStyle` (slot 8).
    pub fn current_style(&self) -> u32 {
        if self.active_styles.is_empty() {
            0
        } else {
            self.style_at_index().unwrap_or(0)
        }
    }

    /// `GetActiveStyles` (slot 9).
    pub fn active_styles(&self) -> &[u32] {
        &self.active_styles
    }

    /// `SetActiveStyles` (slot 10, §4.20).
    pub fn set_active_styles(&mut self, v: &[u32]) {
        let mut idx = if (self.current_style_index as u32 as usize) < v.len() {
            self.current_style_index
        } else {
            0
        };
        if let Some(cur) = self.style_at_index() {
            if let Some(i) = v.iter().rposition(|&s| s == cur) {
                idx = i as i32;
            }
        }
        self.active_styles = v.to_vec();
        self.current_style_index = idx;
    }

    /// Assigns the style list as it stands, without the index search of
    /// `SetActiveStyles` (what a save read does; hosts and tests).
    pub fn assign_active_styles(&mut self, v: Vec<u32>) {
        self.active_styles = v;
    }

    pub fn current_style_index(&self) -> i32 {
        self.current_style_index
    }

    pub fn set_current_style_index(&mut self, i: i32) {
        self.current_style_index = i;
    }

    pub fn years_since_style_change(&self) -> i32 {
        self.years_since_style_change
    }

    pub fn set_years_since_style_change(&mut self, n: i32) {
        self.years_since_style_change = n;
    }

    /// `UseAllStylesAtOnce(f)` (slot 12).
    pub fn use_all_styles_at_once(&mut self, f: bool) {
        self.single_style = !f;
    }

    /// `IsUsingAllStylesAtOnce` (slot 11).
    pub fn is_using_all_styles_at_once(&self) -> bool {
        !self.single_style
    }

    /// `GetYearsBetweenStyleChanges` (slot 13).
    pub fn years_between_style_changes(&self) -> i32 {
        self.years_between_style_changes
    }

    /// `SetYearsBetweenStyleChanges` (slot 14).
    pub fn set_years_between_style_changes(&mut self, n: i32) {
        self.years_between_style_changes = n;
    }

    // ---- persistence (§6) ----

    /// `Write` (§6). Returns whether the stream has no error.
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        let mut ok = out.put_u16(RECORD_VERSION);
        ok &= out.put_i32(self.current_style_index);
        ok &= out.put_i32(self.years_since_style_change);
        ok &= out.put_u8(u8::from(self.single_style));
        ok &= out.put_u32(self.active_styles.len() as u32);
        for &s in &self.active_styles {
            ok &= out.put_u32(s);
        }
        ok &= out.put_i32(self.years_between_style_changes);
        ok
    }

    /// `Read` (§6). Version > 3 → false (nothing changes). Fields are
    /// stored as they are read; a failed read returns false.
    pub fn read(&mut self, inp: &mut dyn InStream) -> bool {
        let Some(version) = inp.get_u16() else {
            return false;
        };
        if version > RECORD_VERSION {
            return false;
        }
        let mut ok = true;
        match inp.get_i32() {
            Some(v) => self.current_style_index = v,
            None => ok = false,
        }
        match inp.get_i32() {
            Some(v) => self.years_since_style_change = v,
            None => ok = false,
        }
        if version >= 2 {
            match inp.get_u8() {
                Some(b) => self.single_style = b != 0,
                None => ok = false,
            }
            let count = inp.get_u32().unwrap_or_else(|| {
                ok = false;
                0
            }) as usize;
            let mut styles = Vec::new();
            for _ in 0..count.min(MAX_SAVED_STYLES) {
                match inp.get_u32() {
                    Some(v) => styles.push(v),
                    None => {
                        ok = false;
                        break;
                    }
                }
            }
            styles.resize(count.min(MAX_SAVED_STYLES), 0);
            self.active_styles = styles;
        } else {
            self.single_style = true;
            self.active_styles = vec![0x2000, 0x2001, 0x2002];
        }
        if version >= 3 {
            match inp.get_i32() {
                Some(v) => self.years_between_style_changes = v,
                None => ok = false,
            }
        }
        ok
    }

    // ---- shared helpers ----

    fn host(&self) -> Rc<dyn TractDeveloperHost> {
        self.host.clone()
    }

    fn draw_u32(&self, kind: DrawKind, n: u32) -> u32 {
        let mut s = self.statics.borrow_mut();
        let v = s.rng.uniform_u32(n);
        s.record(kind, n, f64::from(v));
        v
    }

    fn draw_double(&self, kind: DrawKind) -> f64 {
        let mut s = self.statics.borrow_mut();
        let v = s.rng.uniform_f64();
        s.record(kind, 0, v);
        v
    }

    /// The developer cap checks (§4.2) for stage `s` at `(x, z)`; `pw` is
    /// the developer whose power and water caps apply (`None`: those checks
    /// pass), `fp` the one for fire and park. Returns the failure code.
    fn caps_fail(&self, s: u32, x: i32, z: i32, pw: Option<u64>, fp: u64) -> Option<u32> {
        let h = &self.host;
        if h.cheat(Cheat::IgnoreCaps) {
            return None;
        }
        if let Some(d) = pw {
            if s > u32::from(h.dev_cap(d, Cap::Power)) && !h.cheat(Cheat::IgnorePower) && h.powered_value(x, z) < 2 {
                return Some(code::NO_POWER);
            }
            if s > u32::from(h.dev_cap(d, Cap::Water)) && !h.cheat(Cheat::IgnorePlumbing) && !h.is_cell_watered(x, z) {
                return Some(code::NO_WATER);
            }
        }
        if s > u32::from(h.dev_cap(fp, Cap::Fire)) && h.fire_coverage(x, z) == 0 {
            return Some(code::NO_FIRE);
        }
        if s > u32::from(h.dev_cap(fp, Cap::Park)) && h.park_value(x, z) == 0 {
            return Some(code::NO_PARK);
        }
        None
    }

    /// The style a style-checked step compares against: purposes 1–3 with
    /// a non-empty active list.
    fn styles_apply(&self) -> bool {
        (1..=3).contains(&self.w.purpose) && !self.active_styles.is_empty()
    }

    // ---- Grow (§4.3) ----

    /// `Grow`'s setup (§4.3 steps 1–2): the working state of §2.2 for
    /// `profile`, read from the developer.
    fn setup(&mut self, profile: &GrowthProfile) {
        let h = self.host();
        let dev = profile.developer;
        let mut w = Work {
            dev,
            tx: profile.x,
            tz: profile.z,
            max_stage: u32::from(profile.stage as u8),
            ..Work::default()
        };
        w.purpose = h.dev_purpose(dev);
        w.info = h.dev_construction_info(dev);
        w.wealth = w.info.wealth;
        w.existing_capacity = (h.dev_capacity_value(dev, profile.x, profile.z) & 0xFFFF) as i32;
        let demand = h.dev_source_demand(dev).map_or(0.0, f64::from);
        w.desired = ftol32(demand).max(1);
        w.desired_max = ftol32(r24(f64::from(w.desired) * f64::from(self.tuning.overbuild)));
        w.target_zone = h.dev_target_zone(dev);
        w.sub_type = h.dev_subtype(dev);
        self.w = w;
    }

    /// `Grow(profile, outcomes&, outFlag&)` (slot 5, §4.3): returns the
    /// capacity started (possibly negative). Appends one code per build
    /// attempt and per pass without candidates; never clears `outcomes`.
    pub fn grow(&mut self, profile: &GrowthProfile, outcomes: &mut Vec<Outcome>, out_flag: &mut bool) -> i32 {
        let h = self.host();
        let dev = profile.developer;
        self.setup(profile);
        self.scored.set(0);
        for r in self.histogram.iter_mut() {
            *r = [0; HIST_COLUMNS];
        }
        let size = i64::from(h.dev_desirability_tract_size(dev));
        let mut passes_left = size.saturating_mul(size).saturating_add(1);
        let mut first = true;
        loop {
            // Pass setup.
            self.w.lots.clear();
            self.w.cfgs.clear();
            self.w.candidates.clear();
            self.w.relaxed = !first;
            self.w.chosen_cfg = None;
            self.w.chosen_building = 0;
            self.w.suitability.clear();
            let mut best = 0;
            let mut stop = false;

            self.list_existing_lots(size as i32);
            let r = self.take_over_abandoned();
            best = best.max(r);
            if r == code::SUCCESS {
                outcomes.push(Outcome::Success);
                *out_flag = self.w.out_flag;
                return self.w.built;
            }
            if self.list_possible_lot_configurations() == code::SUCCESS {
                best = best.max(self.list_candidate_lots_existing());
                best = best.max(self.list_candidate_lots_aggregation());
                if self.w.purpose != 5 {
                    best = best.max(self.list_candidate_lots_subdivision());
                }
                if self.w.candidates.is_empty() {
                    if best == code::SUCCESS {
                        best = code::NO_LOT;
                    }
                    outcomes.push(Outcome::from_code(best).unwrap_or(Outcome::UnknownFailure));
                    self.hist(row::GROW, best);
                } else {
                    let mut cands = std::mem::take(&mut self.w.candidates);
                    self.score_candidates(&mut cands);
                    sort_candidates(&mut cands);
                    for c in cands.iter_mut() {
                        let r = if self.w.purpose == 5 {
                            self.build_farm(c)
                        } else {
                            self.build(c)
                        };
                        best = r;
                        outcomes.push(Outcome::from_code(r).unwrap_or(Outcome::UnknownFailure));
                        if r == code::NO_RESOURCES {
                            stop = self.w.relaxed;
                            break;
                        }
                        if self.w.built >= self.w.desired {
                            stop = true;
                            break;
                        }
                        if r == code::SUCCESS {
                            if (5..=8).contains(&self.w.purpose) {
                                stop = true;
                            }
                            break;
                        }
                    }
                }
            }
            if !first && best != code::SUCCESS {
                stop = true;
            }
            passes_left -= 1;
            first = false;
            if passes_left < 0 || stop {
                break;
            }
        }
        self.w.candidates.clear();
        *out_flag = self.w.out_flag;
        self.w.built
    }

    /// `ListExistingLots` (§4.5): one `rand()` (R1), then the lots of the
    /// tract's cells into the lot set.
    fn list_existing_lots(&mut self, tract_size: i32) {
        let rect = tract_rect(self.w.tx, self.w.tz, tract_size);
        {
            let mut s = self.statics.borrow_mut();
            let s = &mut *s;
            s.list_iterator.set_rect(rect.x1, rect.z1, rect.x2, rect.z2);
            let before = s.crt;
            s.list_iterator.reset(true, &mut s.crt);
            if s.crt != before {
                let v = f64::from(s.list_iterator.current);
                let n = s.list_iterator.count as u32;
                s.record(DrawKind::R1, n, v);
            }
        }
        // The cells are walked one at a time (no cell list), so a hostile
        // tract size costs time, not memory.
        loop {
            let cell = self.statics.borrow_mut().list_iterator.next_cell();
            let Some((x, z)) = cell else { break };
            if let Some(l) = self.host.get_lot(x, z) {
                self.w.lots.insert(l);
            }
        }
    }

    /// `TakeOverAbandoned` (§4.4): always ≥ 1.
    fn take_over_abandoned(&mut self) -> u32 {
        let h = self.host();
        let dev_type = self.w.sub_type;
        let t0 = ftol32(r24(
            f64::from(self.w.desired.wrapping_sub(self.w.built)) * f64::from(self.tuning.reoccupy[0])
        ));
        let t1 = ftol32(r24(
            f64::from(self.w.desired_max.wrapping_sub(self.w.built)) * f64::from(self.tuning.reoccupy[1])
        ));
        let mut best = code::NOTHING_TO_REOCCUPY;
        let mut sum: i32 = 0;
        let mut cands = Vec::new();
        let lots: Vec<u64> = self.w.lots.iter().copied().collect();
        for l in lots {
            let state = h.lot_state(l);
            let eligible = state == lot_state::ABANDONED
                || (self.tuning.kick_out && state == lot_state::OCCUPIED && h.lot_occupant_wealth(l) < self.w.wealth);
            if !eligible {
                continue;
            }
            if zone_family(h.lot_zone_type(l)) != self.w.target_zone {
                continue;
            }
            if h.lot_capacity(l, dev_type, true) == 0 {
                best = best.max(code::DIFFERENT_PURPOSE);
                continue;
            }
            let r = self.is_lot_suitable(l, false);
            if r != code::SUCCESS && r != code::HISTORICAL {
                best = best.max(r);
                continue;
            }
            let (x, z) = h.lot_location(l);
            let mut c = Candidate {
                kind: 1,
                lot: Some(l),
                x,
                z,
                ..Candidate::default()
            };
            self.find_existing_capacity(&mut c);
            let mut cap = h.lot_available_capacity(l, dev_type, false);
            if cap == 0 {
                cap = h.lot_capacity(l, dev_type, false);
            }
            let bt = h.lot_building_type(l);
            let p = h.building_profile(bt);
            let pw = h.developer_for(u32::from(p.purpose), u32::from(p.wealth));
            if let Some(f) = self.caps_fail(c.stage, c.x, c.z, pw, self.w.dev) {
                best = best.max(f);
                continue;
            }
            c.capacity = i32::from(cap);
            sum = sum.wrapping_add(i32::from(cap));
            cands.push(c);
        }
        if !cands.is_empty() && sum > t0 {
            self.score_candidates(&mut cands);
            sort_candidates(&mut cands);
            let mut taken: i32 = 0;
            for c in &cands {
                let Some(l) = c.lot else { continue };
                if h.lot_state(l) == lot_state::OCCUPIED {
                    h.abandon_building(l, ABANDON_KICK_OUT);
                }
                h.reoccupy_building(l, dev_type);
                self.w.built = self.w.built.wrapping_add(c.capacity);
                taken = taken.wrapping_add(c.capacity);
                if taken > t1 {
                    break;
                }
            }
            self.hist(row::TAKE_OVER, code::SUCCESS);
            return code::SUCCESS;
        }
        if best == code::NOTHING_TO_REOCCUPY {
            self.hist(row::TAKE_OVER, code::NOTHING_TO_REOCCUPY);
        }
        best
    }

    /// `ListPossibleLotConfigurations` (§4.6).
    fn list_possible_lot_configurations(&mut self) -> u32 {
        let key = (self.w.wealth, self.w.purpose);
        let filter = match self.filters.get(&key) {
            Some(&f) => f,
            None => {
                let f = self.host.register_configuration_filter(key.0, key.1);
                self.filters.insert(key, f);
                f
            }
        };
        let cfgs = self.host.configurations_by_filter(filter);
        if cfgs.is_empty() {
            return code::NO_CONFIGURATION;
        }
        let mut best = 0;
        for cfg in cfgs {
            let r = self.is_lot_configuration_suitable(cfg);
            self.hist(row::LIST_CONFIGURATIONS, r);
            best = best.max(r);
            if r == code::SUCCESS {
                self.w.cfgs.insert(cfg);
            }
        }
        best
    }

    /// `IsLotConfigurationSuitable` (§4.7).
    fn is_lot_configuration_suitable(&mut self, cfg: u64) -> u32 {
        let r = self.lot_configuration_suitable(cfg);
        self.hist(row::CONFIGURATION_SUITABLE, r);
        r
    }

    fn lot_configuration_suitable(&self, cfg: u64) -> u32 {
        let h = &self.host;
        if !h.cfg_compatible_purpose(cfg, self.w.purpose) {
            return code::WRONG_PURPOSE;
        }
        if !h.cfg_compatible_wealth(cfg, self.w.wealth) {
            return code::WRONG_WEALTH;
        }
        let stage = u32::from(h.cfg_growth_stage(cfg));
        if stage > self.w.max_stage {
            return code::STAGE_TOO_BIG;
        }
        if self.styles_apply() {
            let ok = if self.single_style {
                h.cfg_compatible_style(cfg, self.style_at_index().unwrap_or(0))
            } else {
                self.active_styles.iter().any(|&s| h.cfg_compatible_style(cfg, s))
            };
            if !ok {
                return code::WRONG_STYLE;
            }
        }
        if h.cfg_primary_building_ids(cfg).is_none() {
            return code::UNKNOWN;
        }
        let min = if stage == 1 {
            0
        } else {
            i32::from(h.cfg_min_capacity(cfg))
        };
        let max = i32::from(h.cfg_max_capacity(cfg));
        if !self.w.relaxed && max < self.w.desired && stage < self.w.max_stage {
            return code::BUILDING_TOO_SMALL;
        }
        if min > self.w.desired_max.wrapping_add(self.w.existing_capacity) {
            return code::BUILDING_TOO_BIG;
        }
        code::SUCCESS
    }

    /// `IsLotSuitable(lot, redevelop)` (§4.8).
    fn is_lot_suitable(&mut self, lot: u64, redevelop: bool) -> u32 {
        if redevelop {
            if let Some(&r) = self.w.suitability.get(&lot) {
                return r;
            }
        }
        let r = self.lot_suitable(lot, redevelop);
        if redevelop {
            self.w.suitability.insert(lot, r);
        }
        self.hist(row::LOT_SUITABLE, r);
        r
    }

    fn lot_suitable(&self, lot: u64, redevelop: bool) -> u32 {
        let h = &self.host;
        let (x, z) = h.lot_location(lot);
        if !building_fits_in_zone(self.w.info.zone, h.zone_at(x, z)) {
            return code::WRONG_ZONE;
        }
        if redevelop && h.lot_is_livable(lot) && h.lot_days_since_state_change(lot) < 180 {
            return code::JUST_BUILT;
        }
        let (lv_min, lv_max) = (self.w.info.land_value_min, self.w.info.land_value_max);
        if lv_min != 0 || lv_max != 0 {
            let lv = h.land_value(x, z);
            if lv < lv_min || lv > lv_max {
                return code::WRONG_LAND_VALUE;
            }
        }
        let state = h.lot_state(lot);
        if state == lot_state::CONSTRUCTING || state >= 5 {
            return code::LOT_NOT_READY;
        }
        let rect = h.lot_bounding_rect(lot);
        let (rw, rh) = rect_wh(rect);
        match h.can_create_lot(rect.x1, rect.z1, rw, rh) {
            7 | 8 => {}
            1 => return code::LOT_NOT_READY,
            2 => return code::WRONG_ZONE,
            3 => return code::NO_LOT,
            _ => return code::UNKNOWN,
        }
        if h.lot_has_capacity(lot) {
            let t = h.lot_building_type(lot);
            if t == 0 {
                return code::LOT_NOT_READY;
            }
            let p = h.building_profile(t);
            if u32::from(p.purpose) == 5 || (u32::from(p.purpose) == 7 && self.w.purpose == 6) {
                return code::DIFFERENT_PURPOSE;
            }
            if !self.tuning.kick_out && u32::from(p.wealth) != self.w.wealth {
                return code::DIFFERENT_WEALTH;
            }
            if self.w.wealth < u32::from(p.wealth) {
                return code::DIFFERENT_WEALTH;
            }
            if u32::from(p.wealth) == self.w.wealth && u32::from(p.purpose) != self.w.purpose {
                return code::DIFFERENT_WEALTH;
            }
            if h.lot_is_historical(lot) {
                return code::HISTORICAL;
            }
            let s = u32::from(h.lot_configuration_stage(lot).unwrap_or(0));
            let ok = if u32::from(p.wealth) == self.w.wealth {
                self.w.max_stage > s
            } else {
                self.w.max_stage >= s
            };
            if !ok {
                return code::NOT_BIGGER;
            }
        }
        let access = match self.w.purpose {
            1 => h.road_access_value(x, z) != 0,
            6..=8 => h.lot_has_organic_road_access(lot, self.w.purpose),
            _ => h.lot_has_road_access(lot),
        };
        if !access {
            return code::POOR_ROADS;
        }
        code::SUCCESS
    }

    // ---- candidate listing (§4.9) ----

    fn new_lot_candidate(&self, kind: u8, lot: u64, w: u32, hh: u32) -> Candidate {
        let (x, z) = self.host.lot_location(lot);
        Candidate {
            kind,
            lot: Some(lot),
            x,
            z,
            facing: self.host.lot_facing(lot),
            w: w as u8,
            h: hh as u8,
            ..Candidate::default()
        }
    }

    /// `ListCandidateLots_Existing` (§4.9.1).
    fn list_candidate_lots_existing(&mut self) -> u32 {
        let h = self.host();
        let mut best = 0;
        let lots: Vec<u64> = self.w.lots.iter().copied().collect();
        let cfgs: Vec<u64> = self.w.cfgs.iter().copied().collect();
        for l in lots {
            let r = self.is_lot_suitable(l, true);
            if h.lot_state(l) != lot_state::OCCUPIED && r != code::WRONG_ZONE {
                self.w.out_flag = true;
            }
            if r != code::SUCCESS {
                best = best.max(r);
                continue;
            }
            for &cfg in &cfgs {
                let (lw, lh) = h.lot_size(l);
                if h.cfg_size(cfg) != (lw, lh) {
                    self.hist(row::EXISTING, code::NO_CONFIGURATION);
                    best = best.max(code::NO_CONFIGURATION);
                    continue;
                }
                if !growth_stage_fits_in_zone(u32::from(h.cfg_growth_stage(cfg)), h.lot_zone_type(l)) {
                    self.hist(row::EXISTING, code::WRONG_ZONE);
                    best = best.max(code::WRONG_ZONE);
                    continue;
                }
                let slope = h.lot_slope(l);
                let (lo, hi) = (h.cfg_min_slope(cfg), h.cfg_max_slope(cfg));
                if !(slope >= lo && slope <= hi) {
                    self.hist(row::EXISTING, code::TOO_STEEP);
                    best = best.max(code::TOO_STEEP);
                    continue;
                }
                let mut c = self.new_lot_candidate(2, l, lw, lh);
                self.find_networks(&mut c);
                if self.is_lot_configuration_compatible(cfg, &mut c, false) == code::SUCCESS {
                    self.w.candidates.push(c);
                    self.hist(row::EXISTING, code::SUCCESS);
                    best = code::SUCCESS;
                    break;
                }
            }
        }
        best
    }

    /// `ListCandidateLots_Aggregation` (§4.9.2).
    fn list_candidate_lots_aggregation(&mut self) -> u32 {
        let h = self.host();
        let mut best = 0;
        let lots: Vec<u64> = self.w.lots.iter().copied().collect();
        let cfgs: Vec<u64> = self.w.cfgs.iter().copied().collect();
        for l in lots {
            for &cfg in &cfgs {
                let (lw, lh) = h.lot_size(l);
                let (cw, ch) = h.cfg_size(cfg);
                if cw < lw || ch < lh || (cw == lw && ch == lh) {
                    self.hist(row::AGGREGATION, code::NO_CONFIGURATION);
                    best = best.max(code::NO_CONFIGURATION);
                    continue;
                }
                if !growth_stage_fits_in_zone(u32::from(h.cfg_growth_stage(cfg)), h.lot_zone_type(l)) {
                    self.hist(row::AGGREGATION, code::WRONG_ZONE);
                    best = best.max(code::NO_CONFIGURATION);
                    continue;
                }
                let slope = h.lot_slope(l);
                if slope < h.cfg_min_slope(cfg) || slope > h.cfg_max_slope(cfg) {
                    self.hist(row::AGGREGATION, code::TOO_STEEP);
                    best = best.max(code::NO_CONFIGURATION);
                    continue;
                }
                let (x, z) = h.lot_location(l);
                let facing = h.lot_facing(l);
                let d = (cw as i32).wrapping_sub(lw as i32);
                let shifted = match facing.rem_euclid(4) {
                    0 => (x.wrapping_add(d), z),
                    1 => (x, z.wrapping_add(d)),
                    2 => (x.wrapping_sub(d), z),
                    _ => (x, z.wrapping_sub(d)),
                };
                for (ax, az) in [(x, z), shifted] {
                    let mut c = Candidate {
                        kind: 3,
                        x: ax,
                        z: az,
                        facing,
                        w: cw as u8,
                        h: ch as u8,
                        ..Candidate::default()
                    };
                    self.find_networks(&mut c);
                    if self.is_aggregation_legal(&mut c)
                        && self.is_lot_configuration_compatible(cfg, &mut c, false) == code::SUCCESS
                    {
                        self.w.candidates.push(c);
                        self.hist(row::AGGREGATION, code::SUCCESS);
                        best = code::SUCCESS;
                    } else {
                        best = best.max(code::NO_CONFIGURATION);
                    }
                }
            }
        }
        best
    }

    /// `ListCandidateLots_Subdivision` (§4.9.3).
    fn list_candidate_lots_subdivision(&mut self) -> u32 {
        let h = self.host();
        let mut best = 0;
        let lots: Vec<u64> = self.w.lots.iter().copied().collect();
        let cfgs: Vec<u64> = self.w.cfgs.iter().copied().collect();
        for l in lots {
            let r = self.is_lot_suitable(l, true);
            if r != code::SUCCESS {
                best = best.max(r);
                continue;
            }
            for &cfg in &cfgs {
                let (lw, lh) = h.lot_size(l);
                let (cw, ch) = h.cfg_size(cfg);
                if lw < cw || lh < ch || (lw == cw && lh == ch) {
                    self.hist(row::SUBDIVISION, code::NO_LOT);
                    best = best.max(code::NO_LOT);
                    continue;
                }
                // cw = 0 would divide by zero in the original; here it
                // fails the check.
                if cw == 0 || lw % cw != 0 || lh != ch {
                    self.hist(row::SUBDIVISION, code::NO_LOT);
                    best = best.max(code::NO_LOT);
                    continue;
                }
                if !growth_stage_fits_in_zone(u32::from(h.cfg_growth_stage(cfg)), h.lot_zone_type(l)) {
                    self.hist(row::SUBDIVISION, code::WRONG_ZONE);
                    best = best.max(code::WRONG_ZONE);
                    continue;
                }
                let slope = h.lot_slope(l);
                if slope < h.cfg_min_slope(cfg) || slope > h.cfg_max_slope(cfg) {
                    self.hist(row::SUBDIVISION, code::TOO_STEEP);
                    best = best.max(code::TOO_STEEP);
                    continue;
                }
                let mut c = self.new_lot_candidate(4, l, cw, ch);
                c.n = (lw / cw) as i32;
                self.find_networks(&mut c);
                if self.is_lot_configuration_compatible(cfg, &mut c, false) == code::SUCCESS {
                    self.w.candidates.push(c);
                    self.hist(row::SUBDIVISION, code::SUCCESS);
                    best = code::SUCCESS;
                }
            }
        }
        best
    }

    // ---- geometry (§4.10) ----

    /// `GetBoundingRect(c)` (§4.10.1).
    fn bounding_rect(&self, c: &mut Candidate) -> CellRect {
        let r = c.rect;
        if r.x1 >= 0 && r.z1 >= 0 && r.x2 >= 0 && r.z2 >= 0 {
            return r;
        }
        if c.facing == -1 {
            if let Some(l) = c.lot {
                let h = &self.host;
                c.facing = h.lot_facing(l);
                let (w, hh) = h.lot_size(l);
                c.w = w as u8;
                c.h = hh as u8;
                let (x, z) = h.lot_location(l);
                c.x = x;
                c.z = z;
                c.rect = h.lot_bounding_rect(l);
            }
            return c.rect;
        }
        c.rect = candidate_rect(c.x, c.z, i32::from(c.w), i32::from(c.h), c.facing);
        c.rect
    }

    /// `FindNetworks(c)` (§4.10.2).
    fn find_networks(&self, c: &mut Candidate) {
        let h = &self.host;
        let rect = match (c.kind, c.lot) {
            (4, Some(l)) => h.lot_bounding_rect(l),
            _ => self.bounding_rect(c),
        };
        let (cx, cz) = h.city_cell_counts();
        // (side, fixed coordinate, true for a row): the cells are walked
        // lazily, so a hostile rectangle costs time, not memory.
        let mut sides: Vec<(usize, i32, bool)> = Vec::with_capacity(4);
        if rect.z1 > 0 {
            sides.push((0, rect.z1 - 1, true));
        }
        if rect.x2 < cx.wrapping_sub(1) {
            sides.push((1, rect.x2 + 1, false));
        }
        if rect.z2 < cz.wrapping_sub(1) {
            sides.push((2, rect.z2 + 1, true));
        }
        if rect.x1 > 0 {
            sides.push((3, rect.x1 - 1, false));
        }
        for (side, fixed, row) in sides {
            let (lo, hi) = if row { (rect.x1, rect.x2) } else { (rect.z1, rect.z2) };
            for v in lo..=hi {
                let (x, z) = if row { (v, fixed) } else { (fixed, v) };
                let Some(tile) = h.find_network_tile(x, z) else {
                    continue;
                };
                if h.network_is_of_type(tile, network::STREET) || h.network_is_of_type(tile, network::DIRT_ROAD) {
                    c.street[side] = c.street[side].wrapping_add(1);
                }
                if h.network_is_of_type(tile, network::ROAD) || h.network_is_of_type(tile, network::ONE_WAY_ROAD) {
                    c.road[side] = c.road[side].wrapping_add(1);
                }
                if h.network_is_of_type(tile, network::AVENUE) {
                    c.avenue[side] = c.avenue[side].wrapping_add(1);
                }
                if h.network_is_of_type(tile, network::RAIL) {
                    c.rail[side] = c.rail[side].wrapping_add(1);
                }
            }
        }
    }

    /// `FindExistingCapacity(c)` (§4.11).
    fn find_existing_capacity(&self, c: &mut Candidate) {
        if c.existing >= 0 && c.abandoned >= 0 {
            return;
        }
        c.existing = 0;
        c.abandoned = 0;
        c.stage = 0;
        c.wealth = 0;
        let lots: Vec<u64> = if c.kind == 3 {
            c.lot_set.iter().copied().collect()
        } else {
            c.lot.into_iter().collect()
        };
        let h = &self.host;
        for l in lots {
            let cap = i32::from(h.lot_capacity(l, self.w.sub_type, true));
            if h.lot_state(l) == lot_state::ABANDONED {
                c.abandoned = c.abandoned.wrapping_add(cap);
            } else {
                c.existing = c.existing.wrapping_add(cap);
            }
            c.stage = c.stage.max(u32::from(h.lot_configuration_stage(l).unwrap_or(0)));
            let t = h.lot_building_type(l);
            if t != 0 {
                c.wealth = c.wealth.max(u32::from(h.building_profile(t).wealth));
            }
        }
    }

    /// `IsLotConfigurationCompatible(cfg, c, requireRoads)` (§4.12).
    fn is_lot_configuration_compatible(&mut self, cfg: u64, c: &mut Candidate, require_roads: bool) -> u32 {
        let r = self.lot_configuration_compatible(cfg, c, require_roads);
        self.hist(row::CONFIGURATION_COMPATIBLE, r);
        r
    }

    fn lot_configuration_compatible(&self, cfg: u64, c: &mut Candidate, require_roads: bool) -> u32 {
        self.find_existing_capacity(c);
        let h = &self.host;
        if h.cfg_size(cfg) != (u32::from(c.w), u32::from(c.h)) {
            return code::UNKNOWN;
        }
        if !h.cfg_compatible_zone(cfg, h.zone_at(c.x, c.z)) {
            return code::WRONG_ZONE;
        }
        let s = u32::from(h.cfg_growth_stage(cfg));
        let ok = if h.cfg_compatible_wealth(cfg, c.wealth) {
            s > c.stage
        } else {
            s >= c.stage
        };
        if !ok {
            return code::STAGE_TOO_SMALL;
        }
        let slope = match c.lot {
            Some(l) => h.lot_slope(l),
            None => h.area_slope(c.rect),
        };
        if slope < h.cfg_min_slope(cfg) || slope > h.cfg_max_slope(cfg) {
            return code::TOO_STEEP;
        }
        if let Some(f) = self.caps_fail(s, c.x, c.z, Some(self.w.dev), self.w.dev) {
            return f;
        }
        if self.w.target_zone != 3 {
            let bits = h.cfg_required_roads(cfg);
            match roads_check(&bits, c, require_roads) {
                None => return code::POOR_ROADS,
                Some(true) => c.corner = true,
                Some(false) => {}
            }
        }
        code::SUCCESS
    }

    /// `IsAggregationLegal(c)` (§4.13).
    fn is_aggregation_legal(&mut self, c: &mut Candidate) -> bool {
        let rect = self.bounding_rect(c);
        let (cx, cz) = self.host.city_cell_counts();
        if rect.x1 < 0 || rect.z1 < 0 || rect.x2 >= cx || rect.z2 >= cz || rect.x1 > rect.x2 || rect.z1 > rect.z2 {
            return false;
        }
        let h = self.host();
        let mut same_facing = false;
        let mut zone: Option<i32> = None;
        for z in rect.z1..=rect.z2 {
            for x in rect.x1..=rect.x2 {
                let Some(l) = h.get_lot(x, z) else { return false };
                let r = self.is_lot_suitable(l, true);
                if r == code::SUCCESS {
                    if h.lot_facing(l) == c.facing {
                        same_facing = true;
                    }
                } else if r != code::POOR_ROADS {
                    return false;
                }
                let zt = h.lot_zone_type(l);
                match zone {
                    None => zone = Some(zt),
                    Some(z0) if z0 != zt => {
                        self.hist(row::AGGREGATION_LEGAL, code::WRONG_ZONE);
                        return false;
                    }
                    _ => {}
                }
                c.lot_set.insert(l);
            }
        }
        if !same_facing {
            self.hist(row::AGGREGATION_LEGAL, code::POOR_ROADS);
            return false;
        }
        if c.lot_set.len() < 2 {
            self.hist(row::AGGREGATION_LEGAL, code::NO_LOT);
            return false;
        }
        let lots: Vec<u64> = c.lot_set.iter().copied().collect();
        if lots.iter().any(|&l| !rect_inside(h.lot_bounding_rect(l), rect)) {
            self.hist(row::AGGREGATION_LEGAL, code::NO_LOT);
            return false;
        }
        self.hist(row::AGGREGATION_LEGAL, code::SUCCESS);
        true
    }

    // ---- scoring (§4.14) ----

    /// `ScoreCandidateLots` (§4.14): every operation is float32.
    fn score_candidates(&self, cands: &mut [Candidate]) {
        self.scored.set(self.scored.get().wrapping_add(cands.len() as u32));
        for c in cands.iter_mut() {
            if c.kind != 1 {
                self.find_existing_capacity(c);
            }
            let mut draw = || self.draw_double(DrawKind::R2);
            c.score = score_candidate(c, self.w.target_zone, &self.tuning, &mut draw);
        }
    }

    // ---- building (§4.15) ----

    fn profile(&self, b: u32) -> BuildingProfile {
        self.host.building_profile(b)
    }

    /// `Build(c)` (§4.15.1).
    fn build(&mut self, c: &mut Candidate) -> u32 {
        let r = self.pick_building(c);
        if r != code::SUCCESS {
            return r;
        }
        let h = self.host();
        let cap = self.profile(self.w.chosen_building).capacity;
        if !h.claim_construction_resources(cap) {
            self.hist(row::BUILD, code::NO_RESOURCES);
            return code::NO_RESOURCES;
        }
        let lot = match c.kind {
            4 => {
                h.release_construction_resources(cap);
                self.perform_subdivision(c);
                return code::SUCCESS;
            }
            3 => self.perform_aggregation(c),
            2 => match c.lot {
                Some(l) => {
                    self.w.suitability.remove(&l);
                    h.demolish_lot(l);
                    self.w.built = self.w.built.wrapping_sub(c.existing);
                    Some(l)
                }
                None => None,
            },
            _ => None,
        };
        let (Some(lot), Some(cfg)) = (lot, self.w.chosen_cfg) else {
            // The claimed resources are not released [quirk].
            self.hist(row::BUILD, code::UNKNOWN);
            return code::UNKNOWN;
        };
        h.start_construction(lot, cfg, self.w.chosen_building, true);
        self.w.built = self.w.built.wrapping_add(i32::from(cap));
        code::SUCCESS
    }

    /// `BuildFarm(c)` (§4.15.2).
    fn build_farm(&mut self, c: &mut Candidate) -> u32 {
        let h = self.host();
        let rect = c.rect;
        let mut near: Vec<u64> = Vec::new();
        for f in [1, 2, 4, 8] {
            near.extend(h.lots_surrounding_rect(rect, f));
        }
        let mut fields: Vec<u64> = Vec::new();
        let mut i = 0;
        while i < near.len() {
            let l = near[i];
            i += 1;
            if rect_overlaps(h.lot_bounding_rect(l), rect) {
                continue;
            }
            let r = self.is_lot_suitable(l, true);
            if (r == code::SUCCESS || r == code::POOR_ROADS) && !fields.contains(&l) {
                fields.push(l);
                for f in [1, 2, 4, 8] {
                    if near.len() < MAX_FARM_NEIGHBOURS {
                        near.extend(h.lots_surrounding_lot(l, f));
                    }
                }
                near.truncate(MAX_FARM_NEIGHBOURS);
            }
        }
        let n = fields.len();
        if n < 15 {
            return code::UNKNOWN;
        }
        if !h.claim_construction_resources((n + 30) as u16) {
            return code::UNKNOWN;
        }
        h.release_construction_resources(30);
        let r = self.build(c);
        if r != code::SUCCESS {
            h.release_construction_resources(n as u16);
            return r;
        }
        let id = match h.building_field_ids(self.w.chosen_building) {
            Some(ids) => {
                let k = self.draw_u32(DrawKind::R6, ids.len() as u32);
                ids.get(k as usize).copied().unwrap_or(0)
            }
            None => 0,
        };
        let field_cap = self.profile(id).capacity;
        match h.get_lot_configuration(id) {
            None => h.release_construction_resources(n as u16),
            Some(cfg) => {
                for &l in &fields {
                    h.lot_set_facing(l, c.facing);
                    h.start_construction(l, cfg, id, false);
                    self.w.suitability.remove(&l);
                    h.lot_set_state(l, lot_state::OCCUPIED, 0, 0);
                    h.lot_set_population(l, dev_type::I_AGRICULTURE, field_cap);
                }
            }
        }
        r
    }

    /// `PerformAggregation(c, lot&)` (§4.15.3).
    fn perform_aggregation(&mut self, c: &mut Candidate) -> Option<u64> {
        let h = self.host();
        let lots: Vec<u64> = c.lot_set.iter().copied().collect();
        for l in lots {
            self.w.suitability.remove(&l);
            h.demolish_lot(l);
            h.delete_lot(l);
        }
        self.w.built = self.w.built.wrapping_sub(c.existing);
        let mut facing = c.facing;
        if (5..=8).contains(&self.w.purpose) {
            if let Some(cfg) = self.w.chosen_cfg {
                let (cw, ch) = h.cfg_size(cfg);
                if cw == ch {
                    facing = -1;
                }
            }
        }
        let rect = c.rect;
        let (rw, rh) = rect_wh(rect);
        let lot = h.create_lot_if_possible(rect.x1, rect.z1, rw, rh, facing);
        h.redisplay_terrain(rect);
        lot
    }

    /// `PerformSubdivision(c)` (§4.15.4).
    fn perform_subdivision(&mut self, c: &mut Candidate) {
        let h = self.host();
        let (cw, ch) = (i32::from(c.w), i32::from(c.h));
        let (mut x, mut z) = (c.x, c.z);
        let (pw, ph, dx, dz) = match c.facing.rem_euclid(4) {
            0 => {
                x = x.wrapping_add(1 - cw);
                (cw, ch, -cw, 0)
            }
            1 => {
                x = x.wrapping_add(1 - ch);
                z = z.wrapping_add(1 - cw);
                (ch, cw, 0, -cw)
            }
            2 => {
                z = z.wrapping_add(1 - ch);
                (cw, ch, cw, 0)
            }
            _ => (ch, cw, 0, cw),
        };
        if let Some(parent) = c.lot {
            self.w.suitability.remove(&parent);
            h.demolish_lot(parent);
            self.w.built = self.w.built.wrapping_sub(c.existing);
            h.delete_lot(parent);
        } else {
            self.w.built = self.w.built.wrapping_sub(c.existing);
        }
        let mut target = self.w.desired_max.wrapping_sub(self.w.built).wrapping_add(c.existing);
        if target == 0 {
            target = 1;
        }
        for i in 0..c.n.max(0) {
            let lot = h.create_lot_if_possible(x, z, pw, ph, c.facing);
            if let Some(lot) = lot {
                if self.w.built < target {
                    let ok = i == 0 || self.pick_building(c) == code::SUCCESS;
                    if ok {
                        let cap = self.profile(self.w.chosen_building).capacity;
                        if h.claim_construction_resources(cap) {
                            if let Some(cfg) = self.w.chosen_cfg {
                                h.start_construction(lot, cfg, self.w.chosen_building, true);
                            }
                            self.w.built = self.w.built.wrapping_add(i32::from(cap));
                        }
                    }
                }
            }
            x = x.wrapping_add(dx);
            z = z.wrapping_add(dz);
        }
    }

    /// `PickBuilding(c)` (§4.16).
    fn pick_building(&mut self, c: &mut Candidate) -> u32 {
        let h = self.host();
        self.find_existing_capacity(c);
        let mut group = 0;
        if h.needs_anchor_building(c.rect, self.w.purpose) {
            group = ANCHOR_GROUP;
        } else {
            let pcts = h.dev_style_percentages(self.w.dev);
            if !pcts.is_empty() {
                let r = self.draw_u32(DrawKind::R3, 100);
                group = style_group(&pcts, r);
            }
        }
        let mut s_cfg: BTreeSet<u64> = BTreeSet::new();
        let cfgs: Vec<u64> = self.w.cfgs.iter().copied().collect();
        for cfg in cfgs {
            if group != 0 && !h.cfg_compatible_style(cfg, group) {
                self.hist(row::PICK_BUILDING, code::WRONG_STYLE);
                continue;
            }
            let corner = c.corner;
            if self.is_lot_configuration_compatible(cfg, c, corner) == code::SUCCESS {
                s_cfg.insert(cfg);
            }
        }
        if s_cfg.is_empty() {
            return code::UNKNOWN;
        }
        let k = self.draw_u32(DrawKind::R4, s_cfg.len() as u32);
        let Some(&chosen) = s_cfg.iter().nth(k as usize) else {
            return code::UNKNOWN;
        };
        self.w.chosen_cfg = Some(chosen);
        let (lo, hi) = capacity_window(c, self.w.desired_max, self.w.built);
        let mut s27 = StlHashSet::with_hint(100);
        let mut s2 = StlHashSet::with_hint(100);
        let mut last = code::UNKNOWN;
        for id in h.cfg_primary_building_ids(chosen).unwrap_or_default() {
            let mut t = StlHashSet::with_hint(100);
            for b in h.building_types_by_family(id) {
                t.insert(b);
            }
            for b in t.iter_order() {
                if group != 0 && !h.building_in_occupant_group(b, group) {
                    self.hist(row::PICK_BUILDING, code::WRONG_STYLE);
                    continue;
                }
                let r = self.is_building_compatible(b, lo, hi);
                last = r;
                if r == code::SUCCESS {
                    s27.insert(b);
                } else if r == code::MONOPOLY && h.dev_monopoly_use(self.w.dev) == 1 {
                    s2.insert(b);
                }
            }
        }
        if s27.is_empty() {
            if s2.is_empty() {
                return last;
            }
            s27 = s2;
        }
        let order = s27.iter_order();
        let m = self.draw_u32(DrawKind::R5, order.len() as u32);
        self.w.chosen_building = order.get(m as usize).copied().unwrap_or(0);
        code::SUCCESS
    }

    /// `IsBuildingCompatible(b, c)` (§4.17), with the capacity window of
    /// §4.16 step 7.
    fn is_building_compatible(&mut self, b: u32, lo: i32, hi: i32) -> u32 {
        let r = self.building_compatible(b, lo, hi);
        self.hist(row::BUILDING_COMPATIBLE, r);
        r
    }

    fn building_compatible(&self, b: u32, lo: i32, hi: i32) -> u32 {
        let h = &self.host;
        let p = h.building_profile(b);
        if u32::from(p.purpose) != self.w.purpose {
            return code::WRONG_PURPOSE;
        }
        if u32::from(p.wealth) != self.w.wealth {
            return code::WRONG_WEALTH;
        }
        if self.styles_apply() && !h.building_in_occupant_group(b, self.style_at_index().unwrap_or(0)) {
            return code::WRONG_STYLE;
        }
        let cap = i32::from(p.capacity);
        if cap < lo {
            return code::BUILDING_TOO_SMALL;
        }
        if cap > hi {
            return code::BUILDING_TOO_BIG;
        }
        let monopoly = h.building_is_monopoly(b);
        let used = h.dev_monopoly_use(self.w.dev);
        if used < 0 {
            return code::SUCCESS;
        }
        if ((used == 0 || used == 1) && monopoly) || (used == 2 && !monopoly) {
            return code::MONOPOLY;
        }
        code::SUCCESS
    }

    // ---- UpdatePopulation (§4.18) ----

    /// `UpdatePopulation(profile)` (slot 7, §4.18).
    pub fn update_population(&mut self, profile: &GrowthProfile) {
        let h = self.host();
        let dev = profile.developer;
        let (tx, tz) = (profile.x, profile.z);
        let s = h.dev_desirability_shift(dev);
        let rect = CellRect::new(
            tx.wrapping_shl(s),
            tz.wrapping_shl(s),
            tx.wrapping_add(1).wrapping_shl(s).wrapping_sub(1),
            tz.wrapping_add(1).wrapping_shl(s).wrapping_sub(1),
        );
        let lots = h.lots_in_cell_rect(rect);
        if lots.is_empty() {
            return;
        }
        let d = h.dev_desirability_value(dev, tx, tz);
        let ds = r24(f64::from(d) * f64::from(0.01f32));
        let dev_type = h.dev_subtype(dev);
        let wealth = h.dev_wealth(dev);
        // §4.18 [choice]: tract (0, 0) seeds 0 (a clock reseed in the
        // original); it uses the fixed substitute 1.
        let seed = if tx == 0 && tz == 0 {
            1
        } else {
            (tx.wrapping_shl(16).wrapping_add(tz)).wrapping_shl(4) as u32
        };
        let mut g = RzRandom::new(
            seed,
            &FixedSeeds {
                wall_clock: 1,
                process_clock: 1,
            },
        );
        let var = f64::from(self.tuning.occupancy_variation);
        let mv = [
            f64::from(self.tuning.min_vacancy[0]),
            f64::from(self.tuning.min_vacancy[1]),
        ];
        for lot in lots {
            if !(h.lot_has_capacity(lot) && h.lot_occupant_wealth(lot) == wealth) {
                continue;
            }
            let max_cap = h.lot_available_capacity(lot, dev_type, false);
            if max_cap == 0 {
                continue;
            }
            let j = g.range_f64(-var, var);
            self.statics.borrow_mut().record(DrawKind::R7, 0, j);
            let mut occ = r24(r24(j + 1.0) * ds);
            if occ < 0.0 {
                occ = 0.0;
            } else if occ >= 1.0 {
                // A NaN occupancy compares as "< 1" (§4.18): no second
                // draw, and its population is the low half of the NaN
                // pattern, 0.
                let v = g.range_f64(mv[0], mv[1]);
                self.statics.borrow_mut().record(DrawKind::R7, 0, v);
                occ = r24(1.0 - v);
            }
            let pop = rhe(r24(f64::from(max_cap) * occ) as f32) as u16;
            h.lot_set_population(lot, dev_type, pop);
        }
    }

    // ---- Decline (§4.19) ----

    /// `Decline(profile)` (slot 6, §4.19): the capacity abandoned.
    pub fn decline(&mut self, profile: &GrowthProfile) -> i32 {
        let h = self.host();
        let dev = profile.developer;
        let size = h.dev_desirability_tract_size(dev);
        let rect = tract_rect(profile.x, profile.z, size);
        {
            let mut s = self.statics.borrow_mut();
            let s = &mut *s;
            s.decline_iterator.set_rect(rect.x1, rect.z1, rect.x2, rect.z2);
            let before = s.crt;
            s.decline_iterator.reset(true, &mut s.crt);
            if s.crt != before {
                let v = f64::from(s.decline_iterator.current);
                let n = s.decline_iterator.count as u32;
                s.record(DrawKind::R8, n, v);
            }
        }
        let purpose = h.dev_purpose(dev);
        let wealth = h.dev_wealth(dev);
        let dev_type = h.dev_subtype(dev);
        let mut sum: i32 = 0;
        loop {
            let cell = self.statics.borrow_mut().decline_iterator.next_cell();
            let Some((x, z)) = cell else { break };
            let Some(l) = h.get_lot(x, z) else { continue };
            let qualifies = h.lot_state(l) == lot_state::OCCUPIED
                && h.lot_days_since_state_change(l) >= 60
                && {
                    let t = h.lot_building_type(l);
                    t != 0 && u32::from(h.building_profile(t).purpose) == purpose
                }
                && h.lot_occupant_wealth(l) == wealth;
            if qualifies {
                sum = sum.wrapping_add(i32::from(h.lot_capacity(l, dev_type, false)));
                h.abandon_building(l, ABANDON_DECLINE);
            }
        }
        sum
    }

    // ---- read-only views for tests and conformance ----

    /// `desired` and `desiredMax` of the last `grow` (§4.2).
    pub fn desired(&self) -> (i32, i32) {
        (self.w.desired, self.w.desired_max)
    }

    /// The configuration chosen by the last `PickBuilding`.
    pub fn chosen(&self) -> (Option<u64>, u32) {
        (self.w.chosen_cfg, self.w.chosen_building)
    }

    /// The candidates the last `grow` scored (§4.14), takeover candidates
    /// included (diagnostics).
    pub fn last_scored(&self) -> u32 {
        self.scored.get()
    }

    // ---- read-only probes (tooling: the shadow) ----

    /// `Grow`'s setup (§4.3 steps 1–2) for `profile` without a pass: the
    /// working state the predicates below read, with `relaxed` as given. Only
    /// host reads; no draw, no histogram.
    pub fn probe_setup(&mut self, profile: &GrowthProfile, relaxed: bool) {
        self.setup(profile);
        self.w.relaxed = relaxed;
    }

    /// `IsLotConfigurationSuitable(cfg)` (§4.7) on the probe's working state,
    /// not counted.
    pub fn probe_configuration_suitable(&self, cfg: u64) -> u32 {
        self.lot_configuration_suitable(cfg)
    }

    /// `IsLotSuitable(lot, false)` (§4.8) on the probe's working state, not
    /// counted and not cached.
    pub fn probe_lot_suitable(&self, lot: u64) -> u32 {
        self.lot_suitable(lot, false)
    }

    /// `IsBuildingCompatible(b, c)` (§4.17) with the capacity window
    /// `[lo, hi]`, on the probe's working state, not counted.
    pub fn probe_building_compatible(&self, b: u32, lo: i32, hi: i32) -> u32 {
        self.building_compatible(b, lo, hi)
    }
}

/// `desired` and `desiredMax` for a demand value and overbuild (§4.2).
pub fn desired_of(demand: Option<f32>, overbuild: f32) -> (i32, i32) {
    let desired = ftol32(demand.map_or(0.0, f64::from)).max(1);
    (desired, ftol32(r24(f64::from(desired) * f64::from(overbuild))))
}

/// The capacity window of §4.16 step 7: `(lo, hi)`.
pub fn capacity_window(c: &Candidate, desired_max: i32, built: i32) -> (i32, i32) {
    let mut lo = c.existing.checked_add(1).unwrap_or(c.existing);
    if c.kind == 4 && c.n != 0 {
        lo = lo.wrapping_div(c.n);
    }
    let hi = desired_max
        .wrapping_sub(built)
        .wrapping_add(c.existing)
        .max(lo.wrapping_add(1));
    (lo, hi)
}

/// The cell rectangle of a candidate anchored at `(x, z)` (§4.10.1).
pub fn candidate_rect(x: i32, z: i32, w: i32, h: i32, facing: i32) -> CellRect {
    match facing.rem_euclid(4) {
        0 => CellRect::new(
            x.wrapping_sub(w).wrapping_add(1),
            z,
            x,
            z.wrapping_add(h).wrapping_sub(1),
        ),
        1 => CellRect::new(
            x.wrapping_sub(h).wrapping_add(1),
            z.wrapping_sub(w).wrapping_add(1),
            x,
            z,
        ),
        2 => CellRect::new(
            x,
            z.wrapping_sub(h).wrapping_add(1),
            x.wrapping_add(w).wrapping_sub(1),
            z,
        ),
        _ => CellRect::new(
            x,
            z,
            x.wrapping_add(h).wrapping_sub(1),
            z.wrapping_add(w).wrapping_sub(1),
        ),
    }
}

/// `TestForCornerLot` on the required-road bits (§4.10.3).
pub fn corner_ok(bits: &[bool], c: &Candidate) -> bool {
    for (i, &b) in bits.iter().take(4).enumerate() {
        if !b {
            continue;
        }
        let j = c.facing.wrapping_add(i as i32).wrapping_add(1).rem_euclid(4) as usize;
        let n = c.avenue[j].wrapping_add(c.road[j]).wrapping_add(c.street[j]);
        if n <= 0 {
            return false;
        }
    }
    true
}

/// Sorts candidates ascending by score with the game's `std::sort`
/// (§4.14; comparator `a.score < b.score`, false when unordered).
pub fn sort_candidates(cands: &mut Vec<Candidate>) {
    let scores: Vec<f64> = cands.iter().map(|c| c.score).collect();
    let mut idx: Vec<usize> = (0..cands.len()).collect();
    stl_sort(
        &mut idx,
        &mut |&a: &usize, &b: &usize| match (scores.get(a), scores.get(b)) {
            (Some(x), Some(y)) => x < y,
            _ => false,
        },
    );
    let mut old: Vec<Option<Candidate>> = std::mem::take(cands).into_iter().map(Some).collect();
    for i in idx {
        if let Some(c) = old.get_mut(i).and_then(Option::take) {
            cands.push(c);
        }
    }
}

/// `IsLotConfigurationCompatible` check 6 (§4.12) on the required-road
/// bits: `None` fails with 21; `Some(corner)` passes, `corner` telling
/// whether the candidate becomes a corner lot (`hasReq`).
pub fn roads_check(bits: &[bool], c: &Candidate, require_roads: bool) -> Option<bool> {
    let has_req = has_required_roads(bits);
    if require_roads && !has_req {
        return None;
    }
    if !corner_ok(bits, c) {
        return None;
    }
    Some(has_req)
}

/// Whether a configuration requires roads (§4.12 check 6): any of the
/// required-road bits 0, 1, 2; bit 3 is not considered [quirk].
pub fn has_required_roads(bits: &[bool]) -> bool {
    bits.iter().take(3).any(|&b| b)
}

/// `PickBuilding` step 3 (§4.16): the occupant group for draw `r` over the
/// style percentages walked in ascending key order; 0 when none matches.
pub fn style_group(pcts: &[(u32, f32)], mut r: u32) -> u32 {
    let mut sorted = pcts.to_vec();
    sorted.sort_by_key(|&(k, _)| k);
    for (key, pct) in sorted {
        let p = style_points(pct);
        if r < p {
            return key;
        }
        r = r.wrapping_sub(p);
    }
    0
}

/// `rhe(r24(pct × 100))` as `uint32` (§4.16 step 3).
pub fn style_points(pct: f32) -> u32 {
    rhe(r24(f64::from(pct) * 100.0) as f32) as u32
}

/// The score of one candidate (§4.14) whose capacity sums are already
/// known; `draw` is the file-local generator's `RandomDoubleUniform`,
/// called once for industrial developers (zone type 3).
pub fn score_candidate(c: &Candidate, target_zone: i32, tuning: &Tuning, draw: &mut dyn FnMut() -> f64) -> f64 {
    let (wr, wa, wl, wx, lot_costs): (f64, f64, f64, f64, bool) = match target_zone {
        1 => (1.0, 2.0, 10.0, 0.0, true),
        2 => (-2.0, -5.0, 0.0, 0.0, true),
        3 => (-1.0, -1.0, -5.0, 20.0, false),
        _ => (0.0, 0.0, 0.0, 0.0, true),
    };
    let dem = [f64::from(tuning.demolition[0]), f64::from(tuning.demolition[1])];
    let new_lot = [f64::from(tuning.new_lot[0]), f64::from(tuning.new_lot[1])];
    let mut score = 0.0;
    if c.kind != 1 {
        let a = r24(f64::from(c.abandoned) * dem[1]);
        let e = r24(f64::from(c.existing) * dem[0]);
        score = r24(r24(a + e) + 0.0);
    }
    if lot_costs {
        match c.kind {
            3 => {
                let n = (c.lot_set.len() as u32).wrapping_sub(1);
                score = r24(score + r24(f64::from(n) * new_lot[0]));
            }
            4 => score = r24(score + new_lot[1]),
            _ => {}
        }
    }
    if wl != 0.0 {
        let f = |v: i32| f64::from(v);
        let s0 = r24(r24(r24(f(c.avenue[0]) * wa) + r24(f(c.rail[0]) * wl)) + r24(f(c.road[0]) * wr));
        score = r24(score + s0);
        for j in 1..4 {
            let t = r24(r24(r24(f(c.road[j]) * wr) + r24(f(c.avenue[j]) * wa)) + r24(f(c.rail[j]) * wl));
            score = r24(score + t);
        }
    }
    if wx != 0.0 {
        let u = draw();
        score = r24(score + r24(u * 20.0));
    }
    score
}
