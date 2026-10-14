// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cISC4PlumbingSimulator`: state (§3), lifecycle (§4), messages and
//! occupant changes (§5), the pass (§6), the month and production (§7),
//! persistence (§9) and the interface slots (§10).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

use crate::curve::ResponseCurve;
use crate::grid::{Bitmap, ScanIterator, ScanState, MAX_TRACTS};
use crate::props::{self, round_half_even_f32};
use crate::rng::{CrtRand, FixedSeeds, RzRandom, CLOCK_SEED};
use crate::stream::{InStream, OutStream, RefValue, SaveObject};
use crate::variant::{Elem, Variant};
use crate::x87;

use super::tuning::Tuning;
use super::{
    budget, deal, flag, history, msg, network, occupant_type, prop, CellRect, GridRc, Message, PlumbingHost,
    AGENT_FLAGS, AGENT_LEVELS, AGENT_NAME, CLSID_PLUMBING_SIMULATOR, IID_DEPARTMENT_BUDGET, IID_NETWORK_OCCUPANT,
    IID_OCCUPANT, IID_SIM_GRID, NOTIFICATIONS, ORDINANCE_FACTOR, PUBLISHED_GRID_INSTANCE, WORKING_GRID_INSTANCE,
};

/// The record version (§9); every other version is rejected.
pub const PLUMBING_RECORD_VERSION: u16 = 4;

/// A water producer record (§5.3).
#[derive(Clone, Debug, PartialEq)]
pub struct Producer {
    pub seed_x: i32,
    pub seed_z: i32,
    /// The lot footprint (inclusive cells).
    pub rect: CellRect,
    pub max_capacity: u32,
    pub production: u32,
    /// Not saved.
    pub efficiency: f32,
    /// Not saved.
    pub polluted: bool,
    /// Pass scratch.
    pub visited: bool,
    pub assigned: bool,
    pub occupant: u64,
}

/// Produced, consumed, imported, exported (§3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Totals {
    pub produced: i64,
    pub consumed: i64,
    pub imported: i64,
    pub exported: i64,
}

/// The deterministic part of `ApplyPipeFunding(f)` (§7.1 steps 1–3).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PipeFunding {
    /// The new pipe condition (the distress fraction: 0 = no pipe distressed).
    pub condition: f32,
    /// The number of distressed pipes after the month.
    pub target: u32,
    /// The burst draw's upper bound: the draw lies in `[0, burst_max)`, and
    /// is 0 when `burst_max ≤ 0`.
    pub burst_max: i32,
}

/// `ApplyPipeFunding(f)` steps 1–3 (§7.1): the condition from the previous
/// one and the funding fraction `f`, the distressed target and the bound of
/// the burst draw (the draw itself is random).
pub fn pipe_funding(tuning: &Tuning, condition: f32, num_pipes: u32, f: f32) -> PipeFunding {
    let mut cond = condition - tuning.pipe_decay_curve.eval(f * 100.0);
    cond = if cond.is_nan() { 0.0 } else { cond.clamp(0.0, 1.0) };
    if num_pipes == 0 {
        cond = 0.0;
    }
    let target = trunc_i64(fmul_i(f64::from(num_pipes), cond)) as u32;
    let twice = (2.0 * f64::from(target as i32)) as f32;
    let burst_max = trunc_i64(twice * tuning.burst_probability) as i32;
    PipeFunding {
        condition: cond,
        target,
        burst_max,
    }
}

/// `GetWaterProducerInfo` outputs (§10.1 slot 24).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ProducerInfo {
    pub production: u32,
    pub max_capacity: u32,
    pub efficiency: f32,
    pub polluted: bool,
    /// Property `0x08D9763D` (0 if missing).
    pub used: u32,
}

/// Every field of §3 (persisted or not), the grids and the generators.
#[derive(Clone)]
pub struct PlumbingState {
    pub tuning: Tuning,
    pub working: Option<GridRc>,
    pub published: Option<GridRc>,
    pub count_x: i32,
    pub count_z: i32,
    /// `countX · countZ` bytes, index `z · countX + x` (§2).
    pub flags: Vec<u8>,
    pub visited: Bitmap,
    /// Flood queues: A (to process) and B (network cells, then the rings).
    pub queue_a: VecDeque<(i32, i32)>,
    pub queue_b: VecDeque<(i32, i32)>,
    pub iterator: Option<ScanIterator>,
    pub producers: Vec<Producer>,
    /// The network being flooded, by occupant.
    pub current: Option<u64>,
    pub pipes: Vec<u64>,
    pub num_pipes: u32,
    pub num_distressed: u32,
    pub num_burst: u32,
    pub pipe_condition: f32,
    pub demand: i64,
    pub department: Option<u64>,
    pub pass_counter: i32,
    pub average_water_pollution: u32,
    pub total_water_cost: i64,
    pub num_polluted: u32,
    pub last: Totals,
    pub pass: Totals,
    pub building_cells: u32,
    pub watered_cells: u32,
    pub last_building_cells: u32,
    pub last_watered_cells: u32,
    pub factor: f32,
    pub supply: i64,
    pub consumed: i64,
    pub import_owed: i64,
    pub ring: i64,
    pub ring_remaining: i64,
    pub last_pass_pipe_count: u32,
    pub unused_counter: u32,
    /// 0..9; 9 = idle.
    pub state: u32,
    pub recompute: bool,
    pub ageing: bool,
    pub enabled: bool,
    pub initialized: bool,
    pub grids_initialized: bool,
    /// The message server is held (notifications subscribed).
    pub server_held: bool,
    /// The simulator's own `cRZRandom` (created at `Init`).
    pub rng: Option<RzRandom>,
    /// The `rand()` generator of the pipe shuffle (foundations §12.6).
    pub crt: CrtRand,
    pub ref_count: u32,
    pub destroyed: bool,
    /// [choice] Cells per call in states 6 and 7 (`None` = the whole state).
    pub slice_cells: Option<u32>,
    /// Occupants whose reference leaked (§5.3 step 5.2 [quirk]).
    pub leaked: Vec<u64>,
}

impl Default for PlumbingState {
    /// The constructor values (§3).
    fn default() -> Self {
        PlumbingState {
            tuning: Tuning::default(),
            working: None,
            published: None,
            count_x: 0,
            count_z: 0,
            flags: Vec::new(),
            visited: Bitmap::default(),
            queue_a: VecDeque::new(),
            queue_b: VecDeque::new(),
            iterator: None,
            producers: Vec::new(),
            current: None,
            pipes: Vec::new(),
            num_pipes: 0,
            num_distressed: 0,
            num_burst: 0,
            pipe_condition: 0.0,
            demand: 0,
            department: None,
            pass_counter: 0,
            average_water_pollution: 0,
            total_water_cost: 0,
            num_polluted: 0,
            last: Totals::default(),
            pass: Totals::default(),
            building_cells: 0,
            watered_cells: 0,
            last_building_cells: 0,
            last_watered_cells: 0,
            factor: 0.0,
            supply: 0,
            consumed: 0,
            import_owed: 0,
            ring: 0,
            ring_remaining: 0,
            last_pass_pipe_count: 0,
            unused_counter: 0,
            state: 9,
            recompute: false,
            ageing: false,
            enabled: false,
            initialized: false,
            grids_initialized: false,
            server_held: false,
            rng: None,
            crt: CrtRand::default(),
            ref_count: 0,
            destroyed: false,
            slice_cells: None,
            leaked: Vec::new(),
        }
    }
}

impl PlumbingState {
    /// The flag map index of a city cell.
    pub fn index(&self, x: i32, z: i32) -> Option<usize> {
        if x < 0 || z < 0 || x >= self.count_x || z >= self.count_z {
            return None;
        }
        let i = z as usize * self.count_x as usize + x as usize;
        (i < self.flags.len()).then_some(i)
    }

    /// The flag byte of a cell (0 outside).
    pub fn flag(&self, x: i32, z: i32) -> u8 {
        self.index(x, z).map_or(0, |i| self.flags[i])
    }

    /// `flags[x, z] := f(flags[x, z])` (ignored outside).
    pub fn update_flag(&mut self, x: i32, z: i32, f: impl FnOnce(u8) -> u8) {
        if let Some(i) = self.index(x, z) {
            self.flags[i] = f(self.flags[i]);
        }
    }

    /// The working grid's value (0 without it).
    pub fn working_value(&self, x: i32, z: i32) -> u8 {
        self.working.as_ref().map_or(0, |g| g.get_cell_value(x, z))
    }

    fn working_or(&self, x: i32, z: i32, v: u8) {
        if let Some(g) = &self.working {
            g.set(x, z, g.get(x, z) | v);
        }
    }

    /// Marks a cell in the visited bitmap: true when it was not marked and
    /// the bitmap holds it ([choice]: a cell the bitmap cannot hold, which
    /// only a city whose cell count is not a multiple of 32 has, is never
    /// pushed, so every flood ends).
    fn mark(&mut self, x: i32, z: i32) -> bool {
        if self.visited.test(x, z) {
            return false;
        }
        self.visited.set(x, z);
        self.visited.test(x, z)
    }

    fn in_city(&self, x: i32, z: i32) -> bool {
        x >= 0 && z >= 0 && x < self.count_x && z < self.count_z
    }

    /// A cell clamped to `[0, countX − 1] × [0, countZ − 1]`.
    pub fn clamp_cell(&self, (x, z): (i32, i32)) -> (i32, i32) {
        (x.min(self.count_x - 1).max(0), z.min(self.count_z - 1).max(0))
    }

    /// A rectangle clipped to the city (`None` when nothing is left).
    fn clip(&self, r: CellRect) -> Option<CellRect> {
        let c = CellRect::new(
            r.x1.max(0),
            r.z1.max(0),
            r.x2.min(self.count_x - 1),
            r.z2.min(self.count_z - 1),
        );
        (c.x1 <= c.x2 && c.z1 <= c.z2).then_some(c)
    }

    fn producer_index(&self, occ: u64) -> Option<usize> {
        self.producers.iter().position(|p| p.occupant == occ)
    }

    /// A scanning iterator over the city, reset to (0, 0) (§6.2 "nothing
    /// left", §6.5 end). [choice] One whose rectangle is not the city's cell
    /// range (only a hostile save makes one) is recreated over the city.
    fn reset_iterator(&mut self) {
        let city = ScanIterator::for_counts(self.count_x, self.count_z);
        let it = self.iterator.get_or_insert_with(|| city.clone());
        if (it.x0, it.z0, it.x1, it.z1) != (city.x0, city.z0, city.x1, city.z1) {
            *it = city;
        }
        it.reset(0, 0);
    }
}

// ---- arithmetic (foundations §18) -------------------------------------------------------------------

/// `_ftol` (foundations §18.1): truncation toward zero as an `int64`; NaN,
/// ±∞ and magnitudes ≥ 2^63 give `0x8000000000000000` ([`x87::ftol`]).
pub fn trunc_i64(x: f32) -> i64 {
    x87::ftol(f64::from(x))
}

/// `low32(trunc(x))`, sign-extended: NaN and out-of-range values give 0.
fn low32(x: f32) -> i64 {
    i64::from(x87::ftol32(f64::from(x)))
}

/// `int × float`: the integer enters unrounded, one 24-bit rounding.
fn fmul_i(i: f64, f: f32) -> f32 {
    x87::r24(i * f64::from(f)) as f32
}

/// `int / int` in floating point: one 24-bit rounding.
fn fdiv_ii(a: f64, b: f64) -> f32 {
    x87::r24(a / b) as f32
}

/// The pipe line item's expenses (§5.4): `round(float(u32 numPipes) ×
/// costPerPipe)`, the product rounded to `f32`, then half to even.
pub fn pipe_cost(num_pipes: u32, cost_per_pipe: f32) -> i64 {
    trunc_i64(round_half_even_f32(fmul_i(f64::from(num_pipes), cost_per_pipe)))
}

/// One axis of `NearestWater`'s box (§7.2 step 2, int32 arithmetic):
/// `[max(0, c − reach), min(c + reach, count − 1)]`, an upper bound below
/// the lower one raised to it.
fn water_box_axis(c: i32, reach: i32, count: i32) -> (i32, i32) {
    let lo = c.wrapping_sub(reach).max(0);
    let hi = c.wrapping_add(reach).min(count - 1).max(lo);
    (lo, hi)
}

/// `NearestWater`'s spiral search (§7.2 steps 2–5): the box
/// `[max(0, x − reach), min(x + reach, terrainX − 1)] × (same in z)`; −1 when
/// `(x, z)` lies outside it; otherwise the spiral from `(x, z)`, ring by ring
/// clockwise (ring r: the column `x + r` from `z − r + 1` up to `z + r`,
/// the row `z + r` back to `x − r`, the column `x − r` back to `z − r`, the
/// row `z − r` to `x + r`), testing only the cells inside the box, each once,
/// in that order. The result is the ring (the Chebyshev distance) of the
/// first water cell, or −1 after the ring that passes the last box edge.
pub fn nearest_water_ring(
    x: i32,
    z: i32,
    reach: u32,
    terrain: (i32, i32),
    has_water: &mut dyn FnMut(i32, i32) -> bool,
) -> i32 {
    // [choice] Terrain counts are clamped to 0 ..= MAX_TRACTS (bounded work).
    let (tx, tz) = (terrain.0.clamp(0, MAX_TRACTS), terrain.1.clamp(0, MAX_TRACTS));
    let reach = reach as i32;
    let (x0, x1) = water_box_axis(x, reach, tx);
    let (z0, z1) = water_box_axis(z, reach, tz);
    if x < x0 || x > x1 || z < z0 || z > z1 {
        return -1;
    }
    let [x, z, x0, x1, z0, z1] = [x, z, x0, x1, z0, z1].map(i64::from);
    let mut test = |cx: i64, cz: i64| has_water(cx as i32, cz as i32);
    if test(x, z) {
        return 0;
    }
    let last = (x - x0).max(x1 - x).max(z - z0).max(z1 - z);
    for d in 1..=last {
        let found = (x + d <= x1 && ((z - d + 1).max(z0)..=(z + d).min(z1)).any(|cz| test(x + d, cz)))
            || (z + d <= z1 && ((x - d).max(x0)..=(x + d - 1).min(x1)).rev().any(|cx| test(cx, z + d)))
            || (x - d >= x0 && ((z - d).max(z0)..=(z + d - 1).min(z1)).rev().any(|cz| test(x - d, cz)))
            || (z - d >= z0 && ((x - d + 1).max(x0)..=(x + d).min(x1)).any(|cx| test(cx, z - d)));
        if found {
            return d as i32;
        }
    }
    -1
}

/// `GetNext` of the scanning iterator (simgrid §7.4). A finished scan rests
/// at `(x0, z + step)`, i.e. `(0, countZ)` for the city rectangle, as the
/// original's `0x0060C8D0` leaves it (§6.6, PL4); the position is set here
/// explicitly so it does not depend on where the shared iterator stops.
fn scan_next(it: &mut ScanIterator) -> Option<(i32, i32)> {
    let (running, z) = (it.state == ScanState::Running, it.z);
    let r = it.next_cell();
    if r.is_none() && running {
        it.x = it.x0;
        it.z = z.wrapping_add(it.step.max(1));
    }
    r
}

// ---- occupant property readers ----------------------------------------------------------------------

fn numbers(v: &Variant) -> Vec<Elem> {
    match v {
        Variant::Array { elem, .. } if *elem == crate::variant::mem::RZ_CHAR || *elem == 0 => Vec::new(),
        _ => v.items(),
    }
}

fn elem_f32(e: &Elem) -> f32 {
    match *e {
        Elem::F32(x) => x,
        other => other.as_f64() as f32,
    }
}

fn p_i64(h: &dyn PlumbingHost, occ: u64, id: u32) -> Option<i64> {
    let v = h.occupant_property(occ, id)?;
    numbers(&v).first().map(Elem::as_i64)
}

fn p_f32(h: &dyn PlumbingHost, occ: u64, id: u32) -> Option<f32> {
    let v = h.occupant_property(occ, id)?;
    numbers(&v).first().map(elem_f32)
}

/// A per-occupant curve, built for each call (foundations §4.1 [choice]: a
/// non-numeric or single-value property is absent, an empty array is an
/// empty curve evaluating to 0).
fn p_curve(h: &dyn PlumbingHost, occ: u64, id: u32) -> Option<ResponseCurve> {
    props::curve_of_variant(&h.occupant_property(occ, id)?)
}

fn var_u32(v: u32) -> Variant {
    Variant::Scalar(Elem::U32(v))
}

// ---- the simulator --------------------------------------------------------------------------------

/// The plumbing simulator.
pub struct PlumbingSim {
    host: Rc<dyn PlumbingHost>,
    token: Cell<u64>,
    s: RefCell<PlumbingState>,
}

impl PlumbingSim {
    /// A new simulator with the constructor values of §3.
    pub fn new(host: Rc<dyn PlumbingHost>, token: u64) -> Rc<PlumbingSim> {
        Rc::new(PlumbingSim {
            host,
            token: Cell::new(token),
            s: RefCell::new(PlumbingState::default()),
        })
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    /// A copy of the state.
    pub fn snapshot(&self) -> PlumbingState {
        self.s.borrow().clone()
    }

    /// Runs `f` on the state (hosts restoring state, tests).
    pub fn with_state<R>(&self, f: impl FnOnce(&mut PlumbingState) -> R) -> R {
        self.with(f)
    }

    /// Takes the state out for the duration of `f`, so no borrow is held
    /// while `f` calls the host (a re-entrant call sees an empty state).
    fn with<R>(&self, f: impl FnOnce(&mut PlumbingState) -> R) -> R {
        let mut st = std::mem::take(&mut *self.s.borrow_mut());
        let r = f(&mut st);
        *self.s.borrow_mut() = st;
        r
    }

    /// Replaces the simulator's `cRZRandom` (tests; `Init` creates a new
    /// one from the wall clock).
    pub fn set_random(&self, rng: RzRandom) {
        self.s.borrow_mut().rng = Some(rng);
    }

    pub fn random(&self) -> Option<RzRandom> {
        self.s.borrow().rng.clone()
    }

    /// The `rand()` generator (pipe shuffle).
    pub fn crt(&self) -> CrtRand {
        self.s.borrow().crt
    }

    pub fn set_crt(&self, crt: CrtRand) {
        self.s.borrow_mut().crt = crt;
    }

    /// [choice] Cells per call in the time-sliced states 6 and 7 (at least
    /// 1); `None` runs each state whole.
    pub fn set_slice_cells(&self, n: Option<u32>) {
        self.s.borrow_mut().slice_cells = n.map(|v| v.max(1));
    }

    pub fn tuning(&self) -> Tuning {
        self.s.borrow().tuning.clone()
    }

    /// The working grid (rebuilt by the pass).
    pub fn working_grid(&self) -> Option<GridRc> {
        self.s.borrow().working.clone()
    }

    /// `GetWateredGrid()` / `GetSupplyValueMapPtr` (slots 7, 6): the
    /// published grid, no new reference.
    pub fn watered_grid(&self) -> Option<GridRc> {
        self.s.borrow().published.clone()
    }

    fn funding(&self, s: &PlumbingState) -> f32 {
        s.department.map_or(1.0, |d| self.host.department_funding(d))
    }

    // ---- lifecycle (§4) --------------------------------------------------------------------------

    /// `Init()` (§4.1). Always true.
    pub fn init(&self) -> bool {
        let h = self.host.clone();
        self.with(|s| {
            let seeds = FixedSeeds {
                wall_clock: h.wall_clock_seed(),
                process_clock: 0,
            };
            // Step 2: only what is not already held (PL2).
            if s.rng.is_none() {
                s.rng = Some(RzRandom::new(CLOCK_SEED, &seeds));
            }
            if !s.initialized {
                s.initialized = true;
                s.tuning.reload(h.tuning().as_deref());
                for level in AGENT_LEVELS {
                    h.add_agent(level, AGENT_NAME, AGENT_FLAGS);
                }
                if !s.server_held {
                    s.server_held = true;
                    h.subscribe(&NOTIFICATIONS);
                }
                match s.department {
                    None => {
                        if let Some(d) = h.create_department(budget::DEPARTMENT, budget::DEPARTMENT_GROUP) {
                            h.add_ref(d);
                            h.set_department_name(d, budget::NAME_GROUP, budget::DEPARTMENT_NAME);
                            h.set_department_max_funding(d, s.tuning.max_funding);
                            if let Some(li) = h.create_line_item(d, budget::PIPE_LINE, false) {
                                h.set_line_item_name(li, budget::NAME_GROUP, budget::PIPE_LINE_NAME);
                            }
                            s.department = Some(d);
                        }
                    }
                    Some(d) => h.remove_line_item(d, budget::REMOVED_LINE),
                }
            }
            if !s.grids_initialized {
                s.grids_initialized = true;
                for (slot, instance) in [
                    (&mut s.working, WORKING_GRID_INSTANCE),
                    (&mut s.published, PUBLISHED_GRID_INSTANCE),
                ] {
                    if slot.is_none() {
                        if let Some(g) = h.create_grid(instance, 1) {
                            g.fill(0);
                            *slot = Some(g);
                        }
                    }
                }
                let (cx, cz) = h.cell_counts();
                s.count_x = cx.clamp(0, MAX_TRACTS);
                s.count_z = cz.clamp(0, MAX_TRACTS);
                s.flags = vec![0; s.count_x as usize * s.count_z as usize];
                let side = s.count_x.max(s.count_z);
                s.visited = Bitmap::new(side, side);
                if s.iterator.is_none() {
                    s.iterator = Some(ScanIterator::for_counts(s.count_x, s.count_z));
                }
                s.state = 0;
                s.recompute = true;
            }
        });
        true
    }

    /// `Shutdown()` (§4.2). Always true.
    pub fn shutdown(&self) -> bool {
        let h = self.host.clone();
        self.with(|s| {
            if !s.initialized {
                return;
            }
            s.initialized = false;
            s.rng = None;
            if let Some(d) = s.department.take() {
                h.release(d);
            }
            if s.server_held {
                s.server_held = false;
                h.unsubscribe(&NOTIFICATIONS);
            }
            h.remove_agent();
            for slot in [&mut s.working, &mut s.published] {
                if let Some(g) = slot.take() {
                    h.grid_shutdown(&g);
                }
            }
            s.flags = Vec::new();
            s.visited = Bitmap::default();
            for p in std::mem::take(&mut s.producers) {
                h.release(p.occupant);
            }
            s.iterator = None;
            s.current = None;
            // [choice] (§4.2, PL2): the original keeps the grids-initialized
            // flag, so its next `Init` leaves the grids null and crashes;
            // clearing it makes that `Init` rebuild them as on a new city.
            s.grids_initialized = false;
            for p in std::mem::take(&mut s.pipes) {
                h.release(p);
            }
            s.num_pipes = 0;
            s.num_distressed = 0;
            s.num_burst = 0;
            s.num_polluted = 0;
        });
        true
    }

    pub fn is_initialized(&self) -> bool {
        self.s.borrow().initialized
    }

    /// `AddRef()` (§1).
    pub fn add_ref(&self) -> u32 {
        let mut s = self.s.borrow_mut();
        s.ref_count = s.ref_count.wrapping_add(1);
        s.ref_count
    }

    /// `Release()` (§1): above 1 decrements; otherwise `Shutdown()` and
    /// destruction, returning 0.
    pub fn release(&self) -> u32 {
        {
            let mut s = self.s.borrow_mut();
            if s.ref_count > 1 {
                s.ref_count -= 1;
                return s.ref_count;
            }
        }
        self.shutdown();
        self.s.borrow_mut().destroyed = true;
        0
    }

    /// `ReadSimulatorTunableValues()` (§8; `Init` and `0xCA1B151C`).
    pub fn read_tuning(&self) {
        let ex = self.host.tuning();
        self.s.borrow_mut().tuning.reload(ex.as_deref());
    }

    // ---- messages (§5) ---------------------------------------------------------------------------

    /// `DoMessage(msg)` (§5.1): true, except the day and idle messages,
    /// which return `DoIdle`'s result.
    pub fn do_message(&self, m: &Message) -> bool {
        match m.msg_type {
            msg::SIM_NEW_MONTH => self.monthly_simulation(),
            msg::SIM_NEW_DAY => return self.do_idle(5),
            msg::SIM_IDLE => return self.do_idle(m.data3),
            msg::CONNECTIONS_READY => self.s.borrow_mut().enabled = true,
            msg::INSERT_OCCUPANT | msg::REMOVE_OCCUPANT => {
                if let Some(occ) = m.occupant {
                    if self.host.occupant_property(occ, prop::PREVIEW).is_none() {
                        self.occupant_changed(occ, m.msg_type == msg::INSERT_OCCUPANT);
                    }
                }
            }
            msg::TUNING_RELOAD => self.read_tuning(),
            msg::PIPE_BURST => self.pipe_burst(m.data1 as i32, m.data2 as i32),
            msg::DEAL_STATUS if m.data1 == 0 => {
                if let Some(d) = m.deal {
                    let t = self.host.deal_type(d);
                    if t == deal::EXPORT || t == deal::IMPORT {
                        let mut s = self.s.borrow_mut();
                        s.state = 9;
                        s.recompute = true;
                    }
                }
            }
            _ => {}
        }
        true
    }

    /// `0xEA1F0DCD` (§5.1): `flags[z · countX + x] &= 0xE0` at the raw
    /// index, then recompute.
    pub fn pipe_burst(&self, x: i32, z: i32) {
        let mut s = self.s.borrow_mut();
        let i = i64::from(z) * i64::from(s.count_x) + i64::from(x);
        if i >= 0 && (i as u64) < s.flags.len() as u64 {
            s.flags[i as usize] &= flag::KEEP_ON_CLEAR;
        }
        s.recompute = true;
    }

    /// `DoOccupantInserted / Removed(occ)` (§5.2).
    pub fn occupant_changed(&self, occ: u64, insert: bool) {
        self.with(|s| match self.host.occupant_type(occ) {
            occupant_type::BUILDING => self.building_changed(s, occ, insert),
            occupant_type::NETWORK
                if self.host.is_network_occupant(occ) && !self.host.has_network_flag(occ, network::PREVIEW) =>
            {
                self.pipe_changed(s, occ, insert);
            }
            _ => {}
        });
    }

    /// `DoBuildingChanged(occ, insert)` (§5.3).
    fn building_changed(&self, s: &mut PlumbingState, occ: u64, insert: bool) {
        let h = &*self.host;
        let p = h.occupant_position(occ);
        let (x, z) = match &s.working {
            Some(g) => g.geometry().position_to_tract(p[0], p[2]),
            None => h.position_to_cell(p[0], p[2]),
        };
        // 2. Consumer.
        if let Some(c) = p_i64(h, occ, prop::CONSUMPTION) {
            let c = i64::from(c as i32);
            if insert {
                s.update_flag(x, z, |f| f | flag::CONSUMER);
                s.demand = s.demand.wrapping_add(c);
            } else {
                s.update_flag(x, z, |f| f & !flag::CONSUMER);
                s.demand = s.demand.wrapping_sub(c);
            }
        }
        // 3. Building occupants only.
        let Some(building_type) = h.building_type(occ) else {
            return;
        };
        // 4. Budget.
        if let Some(mut item) = h.budget_item_for_purpose(occ, budget::PURPOSE) {
            let dept = h.department_by_id(item.department_id);
            let mut ok = true;
            if item.line_item == 0 {
                h.change_budget_item_line(occ, item.index, building_type);
                match h.budget_item_for_purpose(occ, budget::PURPOSE) {
                    Some(i) if i.line_item != 0 => item = i,
                    _ => ok = false,
                }
            }
            if ok {
                let sign: i64 = if insert { 1 } else { -1 };
                if let Some(d) = dept {
                    let li = h.line_item(d, item.line_item).or_else(|| {
                        let li = h.create_line_item_for_building_type(d, item.line_item, false);
                        if let Some(li) = li {
                            h.set_line_item_display_flag(li, budget::DISPLAY_FLAG, true);
                        }
                        li
                    });
                    if let Some(li) = li {
                        h.add_to_line_item_expenses(li, sign.wrapping_mul(item.cost));
                        let n = h.line_item_secondary_info(li);
                        h.set_line_item_secondary_info(li, n.wrapping_add(sign));
                    }
                }
                h.set_occupant_property(occ, prop::BASE_COST, Variant::Scalar(Elem::I64(item.cost)));
            }
        }
        // 5. Producer.
        if insert {
            if let Some(cap) = h.occupant_property(occ, prop::CAPACITY) {
                h.add_ref(occ);
                let Some(lot) = h.lot_at(x, z) else {
                    // [quirk] the record and the reference leak; no recompute.
                    s.leaked.push(occ);
                    return;
                };
                let rect = h.lot_rect(lot);
                let max_capacity = numbers(&cap).first().map_or(0, |e| e.as_i64() as u32);
                s.producers.insert(
                    0,
                    Producer {
                        seed_x: x,
                        seed_z: z,
                        rect,
                        max_capacity,
                        production: 0,
                        efficiency: 0.0,
                        polluted: false,
                        visited: false,
                        assigned: false,
                        occupant: occ,
                    },
                );
                self.or_flags(s, rect, flag::PRODUCER, true);
                h.set_occupant_property(occ, prop::CONDITION, Variant::Scalar(Elem::F32(0.0)));
            }
        } else if let Some(i) = s.producer_index(occ) {
            let p = s.producers.remove(i);
            self.or_flags(s, p.rect, flag::PRODUCER, false);
            if s.current == Some(occ) {
                s.current = None;
            }
            h.release(occ);
        }
        s.recompute = true;
    }

    /// Sets (`on`) or clears `bit` over a rectangle (clipped to the city).
    fn or_flags(&self, s: &mut PlumbingState, r: CellRect, bit: u8, on: bool) {
        let Some(r) = s.clip(r) else { return };
        for z in r.z1..=r.z2 {
            for x in r.x1..=r.x2 {
                s.update_flag(x, z, |f| if on { f | bit } else { f & !bit });
            }
        }
    }

    /// `DoPipeChanged(netOcc, insert)` (§5.4).
    fn pipe_changed(&self, s: &mut PlumbingState, occ: u64, insert: bool) {
        let h = &*self.host;
        if let Some(r) = s.clip(h.occupant_bounding_cells(occ)) {
            for x in r.x1..=r.x2 {
                for z in r.z1..=r.z2 {
                    if insert {
                        let mut f = s.flag(x, z) | flag::PIPE;
                        for (dir, bit) in [(2, flag::EAST), (1, flag::NORTH), (0, flag::WEST), (3, flag::SOUTH)] {
                            if h.connects_to_neighbor(occ, dir, network::PIPE_TYPE) {
                                f |= bit;
                            }
                        }
                        s.update_flag(x, z, |_| f);
                    } else {
                        s.update_flag(x, z, |f| f & flag::KEEP_ON_CLEAR);
                    }
                }
            }
        }
        if insert {
            if !s.pipes.contains(&occ) {
                h.add_ref(occ);
                s.pipes.push(occ);
                s.num_pipes = s.num_pipes.wrapping_add(1);
            }
        } else if let Some(i) = s.pipes.iter().position(|&p| p == occ) {
            let f = h.network_flags(occ);
            if f & network::DISTRESSED != 0 {
                s.num_distressed = s.num_distressed.wrapping_sub(1);
            }
            if f & network::BURST != 0 {
                s.num_burst = s.num_burst.wrapping_sub(1);
            }
            s.pipes.remove(i);
            h.release(occ);
            s.num_pipes = s.num_pipes.wrapping_sub(1);
        }
        if let Some(d) = s.department {
            if let Some(li) = h.line_item(d, budget::PIPE_LINE) {
                h.set_line_item_expenses(li, pipe_cost(s.num_pipes, s.tuning.cost_per_pipe));
            }
        }
        s.recompute = true;
    }

    // ---- the pass (§6) ---------------------------------------------------------------------------

    /// `DoIdle(budgetMs)` (§6): "finished". The budget is not used
    /// ([choice], see [`PlumbingSim::set_slice_cells`]).
    pub fn do_idle(&self, _budget_ms: u32) -> bool {
        self.with(|s| {
            if !s.enabled {
                return true;
            }
            if s.state == 9 && s.recompute {
                s.recompute = false;
                s.state = 0;
            }
            match s.state {
                0 => {
                    self.start_pass(s);
                    s.state = 1;
                    true
                }
                1 => {
                    self.update_production(s);
                    s.state = 2;
                    true
                }
                2 => {
                    self.reset_working(s);
                    s.state = 3;
                    true
                }
                3 => {
                    self.seed_and_flood(s);
                    true
                }
                4 => {
                    self.ring_flood(s);
                    s.state = 3;
                    true
                }
                5 => {
                    self.pipes_and_connections(s);
                    s.state = 6;
                    self.lot_fill(s)
                }
                6 => self.lot_fill(s),
                7 => self.redisplay(s),
                8 => {
                    self.end_of_pass(s);
                    true
                }
                _ => true,
            }
        })
    }

    /// State 0 (§6.1).
    fn start_pass(&self, s: &mut PlumbingState) {
        let h = &*self.host;
        s.pass_counter = s.pass_counter.wrapping_add(1);
        s.factor = h.cumulative_factor(ORDINANCE_FACTOR, 0).unwrap_or(1.0);
        for t in [deal::IMPORT, deal::EXPORT] {
            for d in h.find_deals(t) {
                h.set_deal_amount_used(d, 0);
            }
        }
        for c in h.connections_of_type(deal::WATER_CONNECTION) {
            h.set_connection_visited(c, false);
        }
        s.pass = Totals::default();
        s.building_cells = 0;
        s.watered_cells = 0;
    }

    /// `UpdateWaterProduction()` (§7.2), the pass's state 1.
    pub fn update_water_production(&self) {
        self.with(|s| self.update_production(s));
    }

    fn update_production(&self, s: &mut PlumbingState) {
        let h = &*self.host;
        s.average_water_pollution = 0;
        s.num_polluted = 0;
        let (mut sum, mut count) = (0u32, 0u32);
        let f = self.funding(s);
        let eff0 = s.tuning.efficiency_curve.eval(f * 100.0) * 0.01f32;
        let t = s.tuning.clone();
        for i in 0..s.producers.len() {
            let (occ, sx, sz, max_capacity) = {
                let p = &s.producers[i];
                (p.occupant, p.seed_x, p.seed_z, p.max_capacity)
            };
            let mut cap = max_capacity;
            let mut eff = eff0;
            let v = h.water_value(sx, sz);
            sum = sum.wrapping_add(v as u32);
            count = count.wrapping_add(1);
            if let Some(lot) = h.occupant_lot(occ) {
                if h.lot_needs_power(lot) && !h.lot_has_power(lot) {
                    let p = &mut s.producers[i];
                    p.production = 0;
                    p.efficiency = 0.0;
                    continue;
                }
            }
            match p_i64(h, occ, prop::SOURCE_TYPE).map_or(0, |v| v as u8) {
                0 => {
                    let r = self.nearest_water(sx, sz, 0, t.fresh_reach);
                    if r == -1 {
                        cap = 0;
                    } else if r > 1 {
                        cap = t.distance_loss.wrapping_mul(max_capacity) / 100;
                    }
                }
                1 if self.nearest_water(sx, sz, 1, t.salt_reach) == -1 => cap = 0,
                _ => {}
            }
            let mut polluted = false;
            if v as u32 > t.max_pumped_pollution && (max_capacity as i32) > 0 {
                polluted = true;
                s.num_polluted = s.num_polluted.wrapping_add(1);
                eff = 0.0;
            }
            if let Some(curve) = p_curve(h, occ, prop::CONDITION_CURVE) {
                eff *= curve.eval(p_f32(h, occ, prop::CONDITION).unwrap_or(0.0));
            }
            let p = &mut s.producers[i];
            p.polluted = polluted;
            p.production = low32(fmul_i(f64::from(cap as i32), eff)) as u32;
            p.efficiency = eff;
        }
        if let Some(avg) = sum.checked_div(count) {
            s.average_water_pollution = avg;
        }
    }

    /// `NearestWater(x, z, type, reach)` (§7.2).
    pub fn nearest_water(&self, x: i32, z: i32, water_type: u8, reach: u32) -> i32 {
        if water_type == 2 {
            return 0;
        }
        let h = &*self.host;
        let Some(terrain) = h.terrain_cell_counts() else {
            return -1;
        };
        nearest_water_ring(x, z, reach, terrain, &mut |cx, cz| h.cell_has_water(cx, cz))
    }

    /// State 2 (§6): the scanning iterator is reset to (0, 0) and runs over
    /// every cell, so it ends done at (0, countZ) (§6.6).
    fn reset_working(&self, s: &mut PlumbingState) {
        s.reset_iterator();
        let mut it = s
            .iterator
            .take()
            .unwrap_or_else(|| ScanIterator::for_counts(s.count_x, s.count_z));
        let g = s.working.clone();
        while let Some((x, z)) = scan_next(&mut it) {
            if let Some(g) = &g {
                let v = if s.flag(x, z) & flag::PIPE != 0 {
                    value_pipe()
                } else {
                    0
                };
                g.set(x, z, v);
            }
        }
        s.iterator = Some(it);
        for p in &mut s.producers {
            p.visited = false;
            p.assigned = false;
        }
        s.current = None;
    }

    /// State 3 (§6.2): seed one network and flood its pipes, or go to state
    /// 5 when nothing is left.
    fn seed_and_flood(&self, s: &mut PlumbingState) {
        let h = &*self.host;
        s.import_owed = 0;
        // The current producer is always null here (PL3: states 2 and 4 and
        // every other writer null it), so the original's "network in
        // progress" branch is dead; [choice] FreeCity4 always seeds.
        let mut seed = None;
        for d in h.find_deals(deal::IMPORT) {
            let amount = i64::from(h.deal_amount(d));
            let used = i64::from(h.deal_amount_used(d));
            if used >= amount {
                continue;
            }
            let Some(c) = h.find_unvisited_connection(deal::WATER_CONNECTION, h.deal_city(d)) else {
                continue;
            };
            h.set_connection_visited(c, true);
            if h.connection_on_other_side(c).is_some_and(|o| h.connection_is_active(o)) {
                seed = Some(s.clamp_cell(h.connection_cell(c)));
                s.supply = amount - used;
                s.import_owed = s.supply;
                break;
            }
        }
        if seed.is_none() {
            if let Some(p) = s.producers.iter_mut().find(|p| !p.visited) {
                p.visited = true;
                s.current = Some(p.occupant);
                seed = Some((p.seed_x, p.seed_z));
                s.supply = i64::from(p.production);
            }
        }
        let Some((sx, sz)) = seed else {
            s.reset_iterator();
            s.state = 5;
            return;
        };
        s.queue_a.clear();
        s.queue_b.clear();
        s.visited.clear();
        s.queue_a.push_back((sx, sz));
        s.queue_b.push_back((sx, sz));
        s.visited.set(sx, sz);
        s.consumed = 0;
        while let Some((x, z)) = s.queue_a.pop_front() {
            let f = s.flag(x, z);
            if f & flag::PRODUCER != 0 {
                if let Some(p) = s.producers.iter_mut().find(|p| (p.seed_x, p.seed_z) == (x, z)) {
                    if !p.visited {
                        if p.production != 0 {
                            s.supply = s.supply.wrapping_add(i64::from(p.production));
                        }
                        p.visited = true;
                    }
                }
            }
            let steps = [
                (flag::WEST, x > 0, (x - 1, z)),
                (flag::EAST, x < s.count_x - 1, (x + 1, z)),
                (flag::NORTH, z > 0, (x, z - 1)),
                (flag::SOUTH, z < s.count_z - 1, (x, z + 1)),
            ];
            for (bit, inside, (nx, nz)) in steps {
                if f & bit != 0 && inside && s.flag(nx, nz) & flag::PIPE != 0 && s.mark(nx, nz) {
                    s.queue_a.push_back((nx, nz));
                    s.queue_b.push_back((nx, nz));
                }
            }
        }
        s.ring_remaining = s.queue_b.len() as i64;
        s.ring = 0;
        s.state = 4;
    }

    /// `IsNeighborConnectedToCurrentNetwork(city, _)` (§6.3).
    fn connected(&self, s: &PlumbingState, city: u64) -> bool {
        let h = &*self.host;
        h.connections_of_type_to_city(deal::WATER_CONNECTION, city)
            .into_iter()
            .any(|c| {
                let (x, z) = s.clamp_cell(h.connection_cell(c));
                s.visited.test(x, z)
            })
    }

    /// `IsNeighborConnectedToCurrentNetwork` on the current bitmap.
    pub fn is_connected_to_current_network(&self, city: u64) -> bool {
        self.with(|s| self.connected(s, city))
    }

    /// `BuildingConsumption(x, z)` (§6.3): the u32 sum of `0xC8ED2D84` over
    /// the buildings in the cell.
    pub fn building_consumption(&self, x: i32, z: i32) -> u32 {
        let h = &*self.host;
        h.buildings_in_cell(x, z)
            .into_iter()
            .map(|b| p_i64(h, b, prop::CONSUMPTION).map_or(0, |v| v as u32))
            .fold(0u32, u32::wrapping_add)
    }

    /// State 4 (§6.3).
    fn ring_flood(&self, s: &mut PlumbingState) {
        let h = &*self.host;
        let own = s.supply.wrapping_sub(s.import_owed);
        s.pass.produced = s.pass.produced.wrapping_add(own);
        // Exports.
        let mut export_taken = 0i64;
        for d in h.find_deals(deal::EXPORT) {
            let amount = i64::from(h.deal_amount(d));
            let used = i64::from(h.deal_amount_used(d));
            let take = (amount - used).min(s.supply);
            if take <= 0 {
                continue;
            }
            if self.connected(s, h.deal_city(d)) {
                s.pass.exported = s.pass.exported.wrapping_add(take);
                s.supply -= take;
                h.set_deal_amount_used(d, (used + take) as i32);
                export_taken += take;
            } else {
                h.set_deal_flags(d, deal::FLAG_NO_CONNECTION, true);
            }
        }
        // Ring flood.
        if s.supply > 0 || export_taken > 0 {
            let radius = i64::from(s.tuning.radius);
            let half = i64::from(s.tuning.radius >> 1);
            while !s.queue_b.is_empty() {
                if s.ring_remaining <= 0 {
                    s.ring_remaining = s.queue_b.len() as i64;
                    s.ring += 1;
                    if s.ring >= radius {
                        break;
                    }
                }
                s.ring_remaining -= 1;
                let Some((x, z)) = s.queue_b.pop_front() else { break };
                if s.flag(x, z) & flag::CONSUMER != 0 {
                    let bc = self.building_consumption(x, z);
                    let c = low32(fmul_i(f64::from(bc as i32), s.factor));
                    s.consumed = s.consumed.wrapping_add(c);
                    if s.consumed > s.supply {
                        s.consumed = s.supply;
                        break;
                    }
                }
                s.working_or(x, z, super::value::WATERED);
                let push = |s: &mut PlumbingState, nx: i32, nz: i32| {
                    if s.in_city(nx, nz) && s.mark(nx, nz) {
                        s.queue_b.push_back((nx, nz));
                    }
                };
                for (nx, nz) in [(x - 1, z), (x + 1, z), (x, z - 1), (x, z + 1)] {
                    push(s, nx, nz);
                }
                if s.ring < half {
                    for (nx, nz) in [(x - 1, z - 1), (x + 1, z - 1), (x - 1, z + 1), (x + 1, z + 1)] {
                        push(s, nx, nz);
                    }
                }
            }
        }
        s.pass.consumed = s.pass.consumed.wrapping_add(s.consumed);
        s.consumed = s.consumed.wrapping_add(export_taken);
        // Imports.
        if s.import_owed > 0 {
            for d in h.find_deals(deal::IMPORT) {
                let amount = i64::from(h.deal_amount(d));
                let used = i64::from(h.deal_amount_used(d));
                let take = (amount - used).min(s.consumed);
                if take <= 0 {
                    continue;
                }
                if self.connected(s, h.deal_city(d)) {
                    s.pass.imported = s.pass.imported.wrapping_add(take);
                    s.consumed -= take;
                    h.set_deal_amount_used(d, (used + take) as i32);
                }
            }
        }
        // Water used.
        let ratio = if own > 0 {
            fdiv_ii(s.consumed as f64, own as f64)
        } else {
            0.0
        };
        for p in &mut s.producers {
            if p.visited && !p.assigned {
                let used = trunc_i64(fmul_i(f64::from(p.production), ratio)) as u32;
                h.set_occupant_property(p.occupant, prop::WATER_USED, var_u32(used));
                p.assigned = true;
            }
        }
        s.queue_b.clear();
        s.current = None;
    }

    /// State 5 (§6.4).
    fn pipes_and_connections(&self, s: &mut PlumbingState) {
        let h = &*self.host;
        s.last_pass_pipe_count = 0;
        s.unused_counter = 0;
        for &p in &s.pipes {
            let (x, z) = h.occupied_cell(p);
            h.set_pipe_watered(p, s.working_value(x, z) & 1 != 0);
            s.last_pass_pipe_count = s.last_pass_pipe_count.wrapping_add(1);
        }
        for c in h.connections_of_type(deal::WATER_CONNECTION) {
            let (x, z) = s.clamp_cell(h.connection_cell(c));
            h.set_connection_active(c, s.working_value(x, z) & 1 != 0);
        }
    }

    /// State 6 (§6.5), time-sliced.
    fn lot_fill(&self, s: &mut PlumbingState) -> bool {
        let h = &*self.host;
        let mut it = s
            .iterator
            .take()
            .unwrap_or_else(|| ScanIterator::for_counts(s.count_x, s.count_z));
        let mut n = 0u32;
        let finished = loop {
            if s.slice_cells.is_some_and(|l| n >= l) {
                break false;
            }
            let Some((x, z)) = scan_next(&mut it) else { break true };
            n += 1;
            let f = s.flag(x, z);
            if f & (flag::PRODUCER | flag::CONSUMER) == 0 {
                continue;
            }
            let mut v = (s.working_value(x, z) & 1) | super::value::LOT;
            if f & flag::CONSUMER != 0 {
                s.building_cells = s.building_cells.wrapping_add(1);
                if v & 1 != 0 {
                    s.watered_cells = s.watered_cells.wrapping_add(1);
                }
            } else {
                v |= super::value::PRODUCER_ONLY;
            }
            for occ in h.buildings_in_cell(x, z) {
                self.fill_at_occupant(s, occ, v);
            }
        };
        s.iterator = Some(it);
        if finished {
            s.reset_iterator();
            std::mem::swap(&mut s.working, &mut s.published);
            s.state = 7;
        }
        finished
    }

    /// `FillGridAtOccupant(working, occ, v)` (§6.5).
    fn fill_at_occupant(&self, s: &PlumbingState, occ: u64, v: u8) {
        let h = &*self.host;
        let Some(g) = &s.working else { return };
        let geo = g.geometry();
        let (lo, hi) = h.occupant_bounding_box(occ);
        let (tx, tz) = geo.position_to_tract(lo[0], lo[2]);
        let rect = match h.lot_at(tx, tz) {
            Some(lot) => h.lot_rect(lot),
            None => {
                let (ux, uz) = geo.position_to_tract(hi[0], hi[2]);
                CellRect::new(tx, tz, ux.wrapping_sub(1), uz.wrapping_sub(1))
            }
        };
        let (x1, z1) = (rect.x1.max(0), rect.z1.max(0));
        let (x2, z2) = (rect.x2.min(geo.count_x - 1), rect.z2.min(geo.count_z - 1));
        for z in z1..=z2 {
            for x in x1..=x2 {
                g.set(x, z, g.get(x, z) | v);
            }
        }
    }

    /// State 7 (§6.6), time-sliced.
    fn redisplay(&self, s: &mut PlumbingState) -> bool {
        let h = &*self.host;
        let mut it = s
            .iterator
            .take()
            .unwrap_or_else(|| ScanIterator::for_counts(s.count_x, s.count_z));
        let (mut min, mut max) = ((s.count_x, s.count_z), (0, 0));
        let mut n = 0u32;
        let finished = loop {
            if s.slice_cells.is_some_and(|l| n >= l) {
                break false;
            }
            let Some((x, z)) = scan_next(&mut it) else { break true };
            n += 1;
            let new = s.published.as_ref().map_or(0, |g| g.get(x, z));
            let old = s.working.as_ref().map_or(0, |g| g.get(x, z));
            if new != old {
                min = (min.0.min(x), min.1.min(z));
                max = (max.0.max(x), max.1.max(z));
            }
        };
        s.iterator = Some(it);
        if min.0 <= max.0 {
            h.redisplay_terrain(min, max);
        }
        if finished {
            s.state = 8;
        }
        finished
    }

    /// State 8 (§6.7).
    fn end_of_pass(&self, s: &mut PlumbingState) {
        let h = &*self.host;
        for i in 0..s.producers.len() {
            let occ = s.producers[i].occupant;
            let used = p_i64(h, occ, prop::WATER_USED).unwrap_or(0) as i32;
            let per_unit = p_f32(h, occ, prop::COST_PER_UNIT).unwrap_or(0.0);
            s.total_water_cost = s
                .total_water_cost
                .wrapping_add(trunc_i64(fmul_i(f64::from(used), per_unit)));
            if let Some(curve) = p_curve(h, occ, prop::MAINTENANCE_CURVE) {
                let cond = p_f32(h, occ, prop::CONDITION).unwrap_or(0.0);
                let base = p_i64(h, occ, prop::BASE_COST).unwrap_or(0);
                let cost = trunc_i64(fmul_i(base as f64, curve.eval(cond)));
                if let Some(item) = h.budget_item_for_purpose(occ, budget::PURPOSE) {
                    if item.cost != cost {
                        if let Some(li) = h.line_item_from_budget_item(&item) {
                            h.add_to_line_item_expenses(li, cost.wrapping_sub(item.cost));
                        }
                        h.change_budget_item_cost(occ, item.index, cost);
                    }
                }
            }
        }
        s.last_building_cells = s.building_cells;
        s.last_watered_cells = s.watered_cells;
        if s.ageing {
            for i in 0..s.producers.len() {
                self.age_producer(s, i);
            }
            s.ageing = false;
        }
        h.post(msg::WATER_MAP_CHANGED, 0);
        s.last = s.pass;
        let records = [
            (history::PRODUCED, s.last.produced as u32),
            (history::CONSUMED, s.last.consumed as u32),
            (history::IMPORTED, s.last.imported as u32),
            (history::EXPORTED, s.last.exported as u32),
            (history::PIPES, s.last_pass_pipe_count),
            (history::UNUSED, s.unused_counter),
            (history::WATERED_BUILDINGS, s.last_watered_cells),
            (
                history::UNWATERED_BUILDINGS,
                s.last_building_cells.wrapping_sub(s.last_watered_cells),
            ),
        ];
        for (id, v) in records {
            h.record_history(id, v);
        }
        for d in h.find_deals(deal::EXPORT) {
            if h.deal_amount_used(d) < h.deal_amount(d) {
                h.set_deal_flags(d, deal::FLAG_NOT_DELIVERED, true);
            }
        }
        h.mark_deals_dirty();
        s.state = 9;
        h.post(msg::DEAL_STATUS, 8);
    }

    /// `AgeWaterProducer` (§7.3) for producer `i`.
    fn age_producer(&self, s: &PlumbingState, i: usize) {
        let h = &*self.host;
        let Some(p) = s.producers.get(i) else { return };
        let occ = p.occupant;
        let used = p_i64(h, occ, prop::WATER_USED).unwrap_or(0) as i32;
        let production = p.production as i32;
        let usage = if used > 0 && production > 0 {
            fdiv_ii(f64::from(used), f64::from(production))
        } else {
            0.0
        };
        let f = self.funding(s);
        let mut condition = p_f32(h, occ, prop::CONDITION).unwrap_or(0.0);
        let k = p_f32(h, occ, prop::AGING_RATE).unwrap_or(0.0);
        let t1 = p_f32(h, occ, prop::AGING_T1).unwrap_or(99.0);
        let t2 = p_f32(h, occ, prop::AGING_T2).unwrap_or(99.0);
        condition += s.tuning.usage_decay_curve.eval(usage) * s.tuning.funding_decay_curve.eval(f) * k;
        h.set_occupant_property(occ, prop::CONDITION, Variant::Scalar(Elem::F32(condition)));
        let c = if condition >= t2 {
            4
        } else if condition >= t1 {
            2
        } else {
            return;
        };
        let pos = h.occupant_position(occ);
        let (cx, cz) = h.position_to_cell(pos[0], pos[2]);
        if let Some(lot) = h.lot_at(cx, cz) {
            h.set_lot_condition(lot, c);
        }
    }

    /// `AgeWaterProducer` for every producer (§6.7 step 3, without the
    /// ageing flag).
    pub fn age_water_producers(&self) {
        self.with(|s| {
            for i in 0..s.producers.len() {
                self.age_producer(s, i);
            }
        });
    }

    // ---- the month (§7.1) ------------------------------------------------------------------------

    /// `DoMonthlySimulation` (§7.1): runs even when not enabled.
    pub fn monthly_simulation(&self) {
        self.with(|s| {
            s.recompute = true;
            s.ageing = true;
            let f = self.funding(s);
            self.apply_pipe_funding(s, f);
        });
    }

    /// `ApplyPipeFunding(f)` (§7.1).
    fn apply_pipe_funding(&self, s: &mut PlumbingState, f: f32) {
        let h = &*self.host;
        let PipeFunding {
            condition,
            target,
            burst_max: hi,
        } = pipe_funding(&s.tuning, s.pipe_condition, s.num_pipes, f);
        s.pipe_condition = condition;
        let bursts = match &mut s.rng {
            Some(r) => {
                // Always four draws; `lo` when the range is empty (§7.1).
                let b = r.range_i32_gaussian_fast(0, hi);
                if hi <= 0 {
                    0
                } else {
                    b
                }
            }
            None => 0,
        };
        if target == s.num_distressed && bursts == 0 {
            return;
        }
        let mut list = s.pipes.clone();
        s.crt.random_shuffle(&mut list);
        if target > s.num_distressed {
            for &p in &list {
                if s.num_distressed == target {
                    break;
                }
                if !h.has_network_flag(p, network::DISTRESSED) {
                    h.set_network_flag(p, network::DISTRESSED);
                    s.num_distressed += 1;
                }
            }
        }
        if target < s.num_distressed {
            for &p in &list {
                if s.num_distressed == target {
                    break;
                }
                if h.has_network_flag(p, network::DISTRESSED) {
                    h.clear_network_flag(p, network::DISTRESSED);
                    s.num_distressed -= 1;
                }
            }
        }
        let mut left = bursts;
        for &p in list.iter().rev() {
            if left <= 0 {
                break;
            }
            if h.has_network_flag(p, network::DISTRESSED) && !h.has_network_flag(p, network::BURST) {
                let (x, z) = h.occupied_cell(p);
                h.attempt_pipe_burst(x, z);
                h.set_network_flag(p, network::BURST);
                s.num_burst = s.num_burst.wrapping_add(1);
                left -= 1;
            }
        }
    }

    // ---- interface slots (§10) -------------------------------------------------------------------

    /// Slot 5 `GetSupplyValue(x, z)`: the published value.
    pub fn supply_value(&self, x: i32, z: i32) -> u8 {
        self.s.borrow().published.as_ref().map_or(0, |g| g.get(x, z))
    }

    /// Slot 8 `AreWateredBuildingsInCell(x, z)`: `(v & 3) == 3`.
    pub fn are_watered_buildings_in_cell(&self, x: i32, z: i32) -> bool {
        self.supply_value(x, z) & 3 == 3
    }

    /// Slot 9 `IsBuildingWatered(occ)`.
    pub fn is_building_watered(&self, occ: u64) -> bool {
        let p = self.host.occupant_position(occ);
        let Some(g) = self.watered_grid() else { return false };
        let (x, z) = g.geometry().position_to_tract(p[0], p[2]);
        self.are_watered_buildings_in_cell(x, z)
    }

    /// Slot 10 `GetWaterProduction(occ)`.
    pub fn water_production(&self, occ: u64) -> u32 {
        let s = self.s.borrow();
        s.producer_index(occ).map_or(0, |i| s.producers[i].production)
    }

    /// Slot 11 `IsCellWatered(x, z)`.
    pub fn is_cell_watered(&self, x: i32, z: i32) -> bool {
        self.s
            .borrow()
            .published
            .as_ref()
            .is_some_and(|g| g.get_cell_value(x, z) & 1 != 0)
    }

    /// Slot 12.
    pub fn num_water_sources(&self) -> u32 {
        self.s.borrow().producers.len() as u32
    }

    /// Slot 13.
    pub fn num_polluted_water_sources(&self) -> u32 {
        self.s.borrow().num_polluted
    }

    /// Slots 14–17: last month's totals (the plugin returns the low 32 bits
    /// in EAX, [`super::eax_of`]).
    pub fn last_month(&self) -> Totals {
        self.s.borrow().last
    }

    /// Slots 18–20.
    pub fn pipe_counts(&self) -> (u32, u32, u32) {
        let s = self.s.borrow();
        (s.num_pipes, s.num_distressed, s.num_burst)
    }

    /// Slot 21 `GetNumWateredBuildings` (cells).
    pub fn num_watered_buildings(&self) -> u32 {
        self.s.borrow().last_watered_cells
    }

    /// Slot 22 `GetNumUnwateredBuildings` (cells).
    pub fn num_unwatered_buildings(&self) -> u32 {
        let s = self.s.borrow();
        s.last_building_cells.wrapping_sub(s.last_watered_cells)
    }

    /// Slot 23 `GetWaterDemand` (64-bit; EAX gets the low 32 bits).
    pub fn water_demand(&self) -> i64 {
        self.s.borrow().demand
    }

    /// Slot 24 `GetWaterProducerInfo(occ, …)`: `None` = AL 0.
    pub fn water_producer_info(&self, occ: u64) -> Option<ProducerInfo> {
        let p = {
            let s = self.s.borrow();
            s.producers[s.producer_index(occ)?].clone()
        };
        let used = p_i64(&*self.host, occ, prop::WATER_USED).map_or(0, |v| v as u32);
        Some(ProducerInfo {
            production: p.production,
            max_capacity: p.max_capacity,
            efficiency: p.efficiency,
            polluted: p.polluted,
            used,
        })
    }

    /// Slot 25 `GetWaterProducerConditionPercent(occ, &pct)`: `None` when
    /// the condition or the curve `0x8A1E07EE` is absent.
    pub fn water_producer_condition_percent(&self, occ: u64) -> Option<f32> {
        let h = &*self.host;
        let cond = p_f32(h, occ, prop::CONDITION)?;
        let curve = p_curve(h, occ, prop::CONDITION_CURVE)?;
        Some(curve.eval(cond))
    }

    /// Slot 26 `GetOccupantMonthlyUpkeepCost(occ, &cost)`.
    pub fn occupant_monthly_upkeep_cost(&self, occ: u64) -> Option<i64> {
        self.host.budget_item_for_purpose(occ, budget::PURPOSE).map(|i| i.cost)
    }

    /// Slot 27 `FindWaterBuildings`: (lowest locally funded producer and its
    /// funding, worst-condition producer and its percentage). The plugin
    /// `AddRef`s each occupant it writes to a non-null pointer.
    #[allow(clippy::type_complexity)]
    pub fn find_water_buildings(&self) -> (Option<(u64, f32)>, Option<(u64, i32)>) {
        let h = &*self.host;
        let (producers, dept) = {
            let s = self.s.borrow();
            (s.producers.clone(), s.department)
        };
        let mut lowest: Option<(u64, f32)> = None;
        let mut best_f = 2.0f32;
        let mut worst: Option<(u64, f32)> = None;
        let mut best_r = 0.0f32;
        for p in &producers {
            let occ = p.occupant;
            if let Some(f) = dept.and_then(|d| h.local_funding(d, occ)) {
                if f < best_f {
                    best_f = f;
                    lowest = Some((occ, f));
                }
            }
            let cond = p_f32(h, occ, prop::CONDITION).unwrap_or(0.0);
            let t1 = p_f32(h, occ, prop::AGING_T1).unwrap_or(99.0);
            let t2 = p_f32(h, occ, prop::AGING_T2).unwrap_or(99.0);
            let ratio = cond / t1.max(t2);
            if ratio > best_r {
                best_r = ratio;
                worst = Some((occ, ratio));
            }
        }
        (lowest, worst.map(|(o, r)| (o, trunc_i64(r * 100.0) as i32)))
    }

    /// Slot 28 `GetWaterProducers`: the producer occupants in list order
    /// (the plugin `AddRef`s each one it appends).
    pub fn water_producers(&self) -> Vec<u64> {
        self.s.borrow().producers.iter().map(|p| p.occupant).collect()
    }

    // ---- persistence helpers (§9) ----------------------------------------------------------------

    fn write_record(&self, out: &mut dyn OutStream, p: Option<&Producer>) {
        match p {
            Some(p) => {
                for v in [p.seed_x, p.seed_z, p.rect.x1, p.rect.z1, p.rect.x2, p.rect.z2] {
                    out.put_i32(v);
                }
                out.put_u32(p.max_capacity);
                out.put_u32(p.production);
                out.put_ref(&self.host.object_ref(p.occupant));
            }
            None => {
                for _ in 0..8 {
                    out.put_u32(0);
                }
                out.put_ref(&RefValue::Null);
            }
        }
    }

    /// One producer record (outer `None`: the stream failed); the inner
    /// `None` for a null or unresolvable occupant reference.
    fn read_record(&self, inp: &mut dyn InStream) -> Option<Option<Producer>> {
        let mut v = [0i32; 6];
        for slot in &mut v {
            *slot = inp.get_i32()?;
        }
        let max_capacity = inp.get_u32()?;
        let production = inp.get_u32()?;
        let r = inp.get_ref_as(IID_OCCUPANT)?;
        if r.is_null() {
            return Some(None);
        }
        let Some(occupant) = self.host.resolve_ref(&r, IID_OCCUPANT) else {
            return Some(None);
        };
        Some(Some(Producer {
            seed_x: v[0],
            seed_z: v[1],
            rect: CellRect::new(v[2], v[3], v[4], v[5]),
            max_capacity,
            production,
            efficiency: 0.0,
            polluted: false,
            visited: false,
            assigned: false,
            occupant,
        }))
    }

    fn read_body(&self, s: &mut PlumbingState, inp: &mut dyn InStream) -> Option<()> {
        let h = &*self.host;
        for slot in [&mut s.working, &mut s.published] {
            let r = inp.get_ref_as(IID_SIM_GRID)?;
            *slot = h.resolve_grid(&r);
        }
        let cx = inp.get_u32()?;
        let cz = inp.get_u32()?;
        if cx > MAX_TRACTS as u32 || cz > MAX_TRACTS as u32 {
            return None;
        }
        s.count_x = cx as i32;
        s.count_z = cz as i32;
        s.flags = vec![0; cx as usize * cz as usize];
        let side = s.count_x.max(s.count_z);
        s.visited = Bitmap::new(side, side);
        let n = s.flags.len();
        s.flags = inp.get_bytes(n)?;
        s.flags.resize(n, 0);
        s.pass_counter = inp.get_i32()?;
        s.average_water_pollution = inp.get_u32()?;
        s.total_water_cost = inp.get_i64()?;
        let count = inp.get_u32()?;
        s.num_polluted = inp.get_u32()?;
        for _ in 0..count {
            if let Some(p) = self.read_record(inp)? {
                s.producers.push(p);
            }
        }
        if let Some(cur) = self.read_record(inp)? {
            if s.producer_index(cur.occupant).is_some() {
                s.current = Some(cur.occupant);
            }
            h.release(cur.occupant);
        }
        let state = match inp.get_u8()? {
            0 => ScanState::Fresh,
            1 => ScanState::Running,
            _ => ScanState::Done,
        };
        let (x, z) = (inp.get_i32()?, inp.get_i32()?);
        let step = inp.get_u32()?;
        let (x0, z0, x1, z1) = (inp.get_i32()?, inp.get_i32()?, inp.get_i32()?, inp.get_i32()?);
        s.iterator = Some(ScanIterator {
            x0,
            z0,
            x1,
            z1,
            step: step as i32,
            x,
            z,
            state,
        });
        s.demand = inp.get_i64()?;
        let pipes = inp.get_u32()?;
        s.num_distressed = inp.get_u32()?;
        s.num_burst = inp.get_u32()?;
        let mut ok = true;
        for _ in 0..pipes {
            let Some(r) = inp.get_ref_as(IID_NETWORK_OCCUPANT) else {
                ok = false;
                break;
            };
            if !r.is_null() {
                if let Some(t) = h.resolve_ref(&r, IID_NETWORK_OCCUPANT) {
                    s.pipes.push(t);
                }
            }
        }
        s.num_pipes = s.pipes.len() as u32;
        if !ok {
            return None;
        }
        let r = inp.get_ref_as(IID_DEPARTMENT_BUDGET)?;
        let d = if r.is_null() {
            None
        } else {
            h.resolve_ref(&r, IID_DEPARTMENT_BUDGET)
        };
        if let Some(old) = std::mem::replace(&mut s.department, d) {
            h.release(old);
        }
        s.last = Totals {
            produced: inp.get_i64()?,
            consumed: inp.get_i64()?,
            imported: inp.get_i64()?,
            exported: inp.get_i64()?,
        };
        Some(())
    }
}

/// The pipe bit of a working grid value (§2).
fn value_pipe() -> u8 {
    super::value::PIPE
}

// ---- persistence (§9) -------------------------------------------------------------------------------

impl SaveObject for PlumbingSim {
    fn class_id(&self) -> u32 {
        CLSID_PLUMBING_SIMULATOR
    }

    /// `Write` (§9). Returns false only on a stream error.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        let s = self.snapshot();
        let grid_ref = |g: &Option<GridRc>| g.as_ref().map_or(RefValue::Null, |g| g.save_ref());
        out.put_u16(PLUMBING_RECORD_VERSION);
        out.put_ref(&grid_ref(&s.working));
        out.put_ref(&grid_ref(&s.published));
        out.put_u32(s.count_x.max(0) as u32);
        out.put_u32(s.count_z.max(0) as u32);
        let n = s.count_x.max(0) as usize * s.count_z.max(0) as usize;
        let mut map = s.flags.clone();
        map.resize(n, 0);
        out.put_bytes(&map);
        out.put_i32(s.pass_counter);
        out.put_u32(s.average_water_pollution);
        out.put_i64(s.total_water_cost);
        out.put_u32(s.producers.len() as u32);
        out.put_u32(s.num_polluted);
        for p in &s.producers {
            self.write_record(out, Some(p));
        }
        let cur = s.current.and_then(|o| s.producer_index(o)).map(|i| &s.producers[i]);
        self.write_record(out, cur);
        let it = s
            .iterator
            .clone()
            .unwrap_or_else(|| ScanIterator::for_counts(s.count_x, s.count_z));
        out.put_u8(match it.state {
            ScanState::Fresh => 0,
            ScanState::Running => 1,
            ScanState::Done => 2,
        });
        out.put_i32(it.x);
        out.put_i32(it.z);
        out.put_u32(it.step as u32);
        for v in [it.x0, it.z0, it.x1, it.z1] {
            out.put_i32(v);
        }
        out.put_i64(s.demand);
        out.put_u32(s.num_pipes);
        out.put_u32(s.num_distressed);
        out.put_u32(s.num_burst);
        for &p in &s.pipes {
            out.put_ref(&self.host.object_ref(p));
        }
        out.put_ref(&s.department.map_or(RefValue::Null, |d| self.host.object_ref(d)));
        for v in [s.last.produced, s.last.consumed, s.last.imported, s.last.exported] {
            out.put_i64(v);
        }
        out.error() == 0
    }

    /// `Read` (§9): false only for a version other than 4.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        if inp.get_u16() != Some(PLUMBING_RECORD_VERSION) {
            return false;
        }
        self.with(|s| {
            let _ = self.read_body(s, inp);
            s.grids_initialized = true;
            s.state = 0;
        });
        true
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
