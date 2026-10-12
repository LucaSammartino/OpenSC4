// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cISC4FlammabilitySimulator` (§3–§11).

use std::any::Any;
use std::cell::{Cell, RefCell, RefMut};
use std::rc::Rc;

use crate::curve::ResponseCurve;
use crate::grid::{CellIterator, TractGrid, IID_SIM_GRID, MAX_TRACTS};
use crate::property::PropertyHolder;
use crate::props::{self, round_half_even_f32};
use crate::rng::CrtRand;
use crate::stream::{InStream, OutStream, RefValue, SaveObject};
use crate::variant::{Elem, Variant};

use super::{
    msg, occupant_type, prop, FlammabilityHost, FlammabilityRandom, Message, OccupantId, ABANDONED_CONDITION,
    ADJUSTED_GRID_INSTANCE, CLSID_FLAMMABILITY_SIMULATOR, FIRE_CAUSE, HISTORY_AVERAGE, RAW_GRID_INSTANCE, SUMMER,
};

/// The record version (§10); every other version is rejected.
pub const FLAMMABILITY_RECORD_VERSION: u16 = 2;

/// A hot tract (a candidate) or a cluster of them (a hotspot), §3.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hotspot {
    pub x: i32,
    pub z: i32,
    pub weight: i32,
}

impl Hotspot {
    pub fn new(x: i32, z: i32, weight: i32) -> Hotspot {
        Hotspot { x, z, weight }
    }
}

/// The tuning fields (§3, §8).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Tuning {
    pub summer_multiplier: f32,
    pub abandonment_multiplier: f32,
    pub no_water_multiplier: f32,
    /// Always 1 after a tuning read (§2).
    pub tract_size: i32,
    /// Always 40 after a tuning read (§8).
    pub cycle_period: i32,
    pub hotspot_threshold: i32,
    pub hotspot_radius: i32,
    pub hotspot_min_weight: i32,
    /// % per cycle.
    pub random_fire_probability: f32,
    /// Weight → % per cycle; `None` = no hotspot fires.
    pub hotspot_fire_curve: Option<ResponseCurve>,
}

impl Tuning {
    /// The code defaults (§8), set before every read.
    pub fn code_defaults() -> Tuning {
        Tuning {
            summer_multiplier: 1.2,
            abandonment_multiplier: 1.1,
            no_water_multiplier: 1.1,
            tract_size: 1,
            cycle_period: 40,
            hotspot_threshold: 196,
            hotspot_radius: 8,
            hotspot_min_weight: 16,
            random_fire_probability: 0.01,
            hotspot_fire_curve: None,
        }
    }

    /// The tuning read (§8): code defaults first, then every present
    /// property; a missing exemplar keeps all defaults. The curve follows
    /// the "delete" pattern (foundations §4.1).
    pub fn read(h: Option<&PropertyHolder>) -> Tuning {
        let mut t = Tuning::code_defaults();
        let Some(h) = h else { return t };
        if let Some(v) = props::f32_of(h, prop::SUMMER_MULTIPLIER) {
            t.summer_multiplier = v;
        }
        if let Some(v) = props::f32_of(h, prop::ABANDONMENT_MULTIPLIER) {
            t.abandonment_multiplier = v;
        }
        if let Some(v) = props::f32_of(h, prop::NO_WATER_MULTIPLIER) {
            t.no_water_multiplier = v;
        }
        if let Some(v) = props::u32s(h, prop::HOTSPOT_SETTINGS) {
            if let &[threshold, radius, min_weight] = v.as_slice() {
                t.hotspot_threshold = threshold as i32;
                t.hotspot_radius = radius as i32;
                t.hotspot_min_weight = min_weight as i32;
            }
        }
        t.hotspot_fire_curve = props::f32s(h, prop::HOTSPOT_FIRE_CURVE).and_then(|v| ResponseCurve::from_flat(&v));
        if let Some(v) = props::f32_of(h, prop::RANDOM_FIRE_PROBABILITY) {
            t.random_fire_probability = v;
        }
        t
    }

    /// The shipped exemplar (§8, `SimCity_1.dat`), for hosts and tests
    /// without the game's data files.
    pub fn shipped_exemplar() -> PropertyHolder {
        let mut h = PropertyHolder::new();
        let f = |v: f32| Variant::Scalar(Elem::F32(v));
        h.add(prop::SUMMER_MULTIPLIER, f(1.0));
        h.add(prop::ABANDONMENT_MULTIPLIER, f(1.5));
        h.add(prop::NO_WATER_MULTIPLIER, f(1.25));
        h.add(
            prop::HOTSPOT_SETTINGS,
            Variant::array(
                crate::variant::mem::UINT32,
                vec![Elem::U32(54), Elem::U32(8), Elem::U32(40)],
            ),
        );
        let curve = [0.0, 0.0, 39.0, 0.0, 40.0, 7.0, 50.0, 15.0, 70.0, 20.0, 100.0, 25.0];
        h.add(
            prop::HOTSPOT_FIRE_CURVE,
            Variant::array(
                crate::variant::mem::FLOAT32,
                curve.iter().map(|&v| Elem::F32(v)).collect(),
            ),
        );
        h.add(prop::RANDOM_FIRE_PROBABILITY, f(5.0));
        h
    }
}

/// The simulator state (§3) apart from the grids and the generators.
#[derive(Clone, Debug, PartialEq)]
pub struct FlammabilityData {
    pub initialized: bool,
    pub ref_count: u32,
    /// Set when `Release` destroyed the object (§4.3).
    pub destroyed: bool,
    pub iterator: CellIterator,
    /// The city's cell counts (bounds checks).
    pub count_x: i32,
    pub count_z: i32,
    pub average: u8,
    pub global_multiplier: f32,
    pub zoned_sum: i32,
    pub zoned_count: i32,
    pub candidates: Vec<Hotspot>,
    pub hotspots: Vec<Hotspot>,
    pub tuning: Tuning,
    pub tracts_per_day: i32,
}

impl Default for FlammabilityData {
    /// The constructor values (§3).
    fn default() -> Self {
        FlammabilityData {
            initialized: false,
            ref_count: 0,
            destroyed: false,
            iterator: CellIterator::constructed(),
            count_x: 0,
            count_z: 0,
            average: 0,
            global_multiplier: 0.0,
            zoned_sum: 0,
            zoned_count: 0,
            candidates: Vec::new(),
            hotspots: Vec::new(),
            tuning: Tuning::default(),
            tracts_per_day: 0,
        }
    }
}

// ---- arithmetic (§6) ------------------------------------------------------------------------------

/// A value clamped to 0..255; NaN reads as 0 ([choice]).
fn to_byte(x: f32) -> u8 {
    if x.is_nan() {
        0
    } else {
        x.clamp(0.0, 255.0) as u8
    }
}

/// One x87 multiply at 24-bit precision (foundations §18): the exact
/// product of an integer operand (not pre-rounded) and a `float32`, rounded
/// once to `float32`.
fn mul24(a: f64, m: f32) -> f32 {
    (a * f64::from(m)) as f32
}

/// `roundHE(a × m)`: the product rounded to `float32`, then to an integer,
/// ties to even (the `1.5 × 2^23` trick at 24-bit precision, §15.5).
fn round_product(a: f64, m: f32) -> f32 {
    round_half_even_f32(mul24(a, m))
}

/// `clamp(roundHE(p × m), 0, 255)` (§6.1).
pub fn occupant_raw_value(p: i64, m: f32) -> u8 {
    to_byte(round_product(p as f64, m))
}

/// `clamp(roundHE(w × g), 0, 255)`: the adjusted value written by an
/// occupant update (§6.3).
pub fn adjusted_value(w: u8, g: f32) -> u8 {
    to_byte(round_product(f64::from(w), g))
}

/// The daily value of a tract with raw value `r > 0` (§6.5):
/// `roundHE(r × g)`, then `roundHE(v × noWater)` when not watered, at most
/// 255 ([choice]: and at least 0).
pub fn daily_value(r: u8, g: f32, watered: bool, no_water: f32) -> u8 {
    let mut v = round_product(f64::from(r), g);
    if !watered {
        v = round_product(f64::from(v), no_water);
    }
    to_byte(v)
}

/// `ComputeGlobalMultiplier` (§6.4) from the ordinance factor (`None` = no
/// ordinance simulator: 1.0) and the season (`None` = no weather).
pub fn global_multiplier(factor: Option<f32>, season: Option<i32>, summer: f32) -> f32 {
    let f = factor.unwrap_or(1.0);
    if season == Some(SUMMER) {
        f * summer
    } else {
        f
    }
}

/// `ComputeHotspotList` (§6.7): merges the candidates, in discovery order,
/// into the first hotspot closer than `radius` (Manhattan, exclusive), then
/// drops hotspots lighter than `min_weight`.
pub fn compute_hotspots(candidates: &[Hotspot], radius: i32, min_weight: i32) -> Vec<Hotspot> {
    let mut hotspots: Vec<Hotspot> = Vec::new();
    for c in candidates {
        let near = if radius <= 0 {
            None // no distance is < radius
        } else {
            hotspots.iter_mut().find(|h| {
                let d = (i64::from(h.x) - i64::from(c.x)).abs() + (i64::from(h.z) - i64::from(c.z)).abs();
                d < i64::from(radius)
            })
        };
        match near {
            Some(h) => {
                let w = h.weight;
                let n = w.wrapping_add(1);
                h.x = h.x.wrapping_mul(w).wrapping_add(c.x).checked_div(n).unwrap_or(h.x);
                h.z = h.z.wrapping_mul(w).wrapping_add(c.z).checked_div(n).unwrap_or(h.z);
                h.weight = n;
            }
            None => hotspots.push(*c),
        }
    }
    hotspots.retain(|h| h.weight >= min_weight);
    hotspots
}

/// The integer value of a property (its first element; 0 if absent).
fn int_value(v: Option<Variant>) -> i64 {
    v.and_then(|v| v.items().first().map(Elem::as_i64)).unwrap_or(0)
}

type GridRc = Rc<dyn TractGrid<u8>>;

/// The flammability simulator.
pub struct FlammabilitySim {
    host: Rc<dyn FlammabilityHost>,
    token: Cell<u64>,
    d: RefCell<FlammabilityData>,
    adjusted: RefCell<Option<GridRc>>,
    raw: RefCell<Option<GridRc>>,
    rng: RefCell<Box<dyn FlammabilityRandom>>,
    crt: RefCell<CrtRand>,
}

impl FlammabilitySim {
    /// A new simulator with the constructor values of §3 and the given fire
    /// roll generator (normally a [`super::FlammabilityRng`]).
    pub fn new(host: Rc<dyn FlammabilityHost>, token: u64, rng: Box<dyn FlammabilityRandom>) -> Rc<FlammabilitySim> {
        Rc::new(FlammabilitySim {
            host,
            token: Cell::new(token),
            d: RefCell::new(FlammabilityData::default()),
            adjusted: RefCell::new(None),
            raw: RefCell::new(None),
            rng: RefCell::new(rng),
            crt: RefCell::new(CrtRand::default()),
        })
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    /// Replaces the fire roll generator.
    pub fn set_random(&self, rng: Box<dyn FlammabilityRandom>) {
        *self.rng.borrow_mut() = rng;
    }

    /// The `rand()` generator (iterator start, hotspot shuffle).
    pub fn crt(&self) -> CrtRand {
        *self.crt.borrow()
    }

    pub fn set_crt(&self, crt: CrtRand) {
        *self.crt.borrow_mut() = crt;
    }

    pub fn snapshot(&self) -> FlammabilityData {
        self.d.borrow().clone()
    }

    /// Direct state access, for hosts that restore state and for tests.
    pub fn data_mut(&self) -> RefMut<'_, FlammabilityData> {
        self.d.borrow_mut()
    }

    /// `GetFlammabilityGrid()` (slot 5): the adjusted grid, no new reference.
    pub fn adjusted_grid(&self) -> Option<GridRc> {
        self.adjusted.borrow().clone()
    }

    /// The raw grid (not exposed through the interface).
    pub fn raw_grid(&self) -> Option<GridRc> {
        self.raw.borrow().clone()
    }

    fn raw_get(&self, x: i32, z: i32) -> u8 {
        self.raw_grid().map_or(0, |g| g.get(x, z))
    }

    fn raw_set(&self, x: i32, z: i32, v: u8) {
        if let Some(g) = self.raw_grid() {
            g.set(x, z, v);
        }
    }

    fn adjusted_set(&self, x: i32, z: i32, v: u8) {
        if let Some(g) = self.adjusted_grid() {
            g.set(x, z, v);
        }
    }

    // ---- lifecycle (§4) ----------------------------------------------------------------------------

    /// `Init()` (§4.1, core part: the agent and the notifications are the
    /// plugin's). Always true.
    pub fn init(&self) -> bool {
        if self.d.borrow().initialized {
            return true;
        }
        let tuning = Tuning::read(self.host.tuning().as_deref());
        let tract_size = tuning.tract_size;
        let cycle = tuning.cycle_period;
        {
            let mut d = self.d.borrow_mut();
            d.initialized = true;
            d.tuning = tuning;
        }
        let (cx, cz) = self.host.city_cell_counts();
        {
            let mut d = self.d.borrow_mut();
            d.count_x = cx.clamp(0, MAX_TRACTS);
            d.count_z = cz.clamp(0, MAX_TRACTS);
        }
        for (slot, instance) in [(&self.adjusted, ADJUSTED_GRID_INSTANCE), (&self.raw, RAW_GRID_INSTANCE)] {
            let loaded = slot.borrow().clone();
            if let Some(g) = loaded {
                if g.geometry().size != 1 {
                    // [choice] FreeCity4 supports only S = 1 (§14.4).
                    *slot.borrow_mut() = None;
                    self.host.grid_shutdown(&g);
                }
            }
            if slot.borrow().is_none() {
                if let Some(g) = self.host.create_grid(instance, tract_size) {
                    g.fill(0);
                    *slot.borrow_mut() = Some(g);
                }
            }
        }
        let (tx, tz) = self.adjusted_grid().map_or((0, 0), |g| {
            (g.count_x().clamp(0, MAX_TRACTS), g.count_z().clamp(0, MAX_TRACTS))
        });
        let mut d = self.d.borrow_mut();
        d.tracts_per_day = if cycle > 0 {
            (i64::from(tz) * i64::from(tx) / i64::from(cycle)) as i32
        } else {
            0
        };
        d.iterator.set_rect(0, 0, (tx - 1).max(0), (tz - 1).max(0));
        d.iterator.dirty = 1;
        d.zoned_sum = 0;
        d.zoned_count = 0;
        true
    }

    /// `Shutdown()` (§4.2, core part). Always true.
    pub fn shutdown(&self) -> bool {
        if !self.d.borrow().initialized {
            return true;
        }
        {
            let mut d = self.d.borrow_mut();
            d.initialized = false;
            d.tuning.hotspot_fire_curve = None;
        }
        for slot in [&self.adjusted, &self.raw] {
            let g = slot.borrow_mut().take();
            if let Some(g) = g {
                self.host.grid_shutdown(&g);
            }
        }
        true
    }

    pub fn is_initialized(&self) -> bool {
        self.d.borrow().initialized
    }

    /// `AddRef()` (§4.3).
    pub fn add_ref(&self) -> u32 {
        let mut d = self.d.borrow_mut();
        d.ref_count = d.ref_count.wrapping_add(1);
        d.ref_count
    }

    /// `Release()` (§4.3): from a count ≤ 1, `Shutdown()` and destruction
    /// (the count is not decremented); returns 0.
    pub fn release(&self) -> u32 {
        {
            let mut d = self.d.borrow_mut();
            if d.ref_count > 1 {
                d.ref_count -= 1;
                return d.ref_count;
            }
        }
        self.shutdown();
        self.d.borrow_mut().destroyed = true;
        0
    }

    // ---- messages (§5) -----------------------------------------------------------------------------

    /// `DoMessage(msg)` (§5). Always true.
    pub fn do_message(&self, m: &Message) -> bool {
        match m.msg_type {
            msg::SIM_NEW_DAY => self.do_daily_simulation(),
            msg::INSERT_OCCUPANT | msg::CHANGE_OCCUPANT | msg::REMOVE_OCCUPANT => {
                if let Some(occ) = m.occupant {
                    if self.host.occupant_property(occ, prop::PREVIEW_OCCUPANT).is_none() {
                        self.occupant_update(occ, m.msg_type != msg::REMOVE_OCCUPANT);
                    }
                }
            }
            msg::SEASON_CHANGE | msg::ORDINANCE_UPDATED => {
                let g = self.compute_global_multiplier();
                self.d.borrow_mut().global_multiplier = g;
            }
            msg::CONDITION_CHANGED => {
                if let Some(b) = m.lot.and_then(|lot| self.host.lot_building(lot)) {
                    self.occupant_update(b, false);
                }
            }
            _ => {}
        }
        true
    }

    // ---- algorithms (§6) ---------------------------------------------------------------------------

    fn raw_value_of_type(&self, occ: OccupantId, t: u32) -> u8 {
        let m = if t == occupant_type::BUILDING || t == occupant_type::PROP {
            let mut m = 1.0f32;
            if let Some(lot) = self.host.occupant_lot(occ) {
                if self.host.lot_condition(lot) == ABANDONED_CONDITION {
                    m = self.d.borrow().tuning.abandonment_multiplier;
                }
            }
            m
        } else if t == occupant_type::FLORA {
            1.0
        } else {
            return 0;
        };
        let p = int_value(self.host.occupant_property(occ, prop::FLAMMABILITY));
        occupant_raw_value(p, m)
    }

    /// `GetOccupantRawFlammability(occ)` (§6.1).
    pub fn occupant_raw(&self, occ: OccupantId) -> u8 {
        let t = self.host.occupant_type(occ);
        self.raw_value_of_type(occ, t)
    }

    /// `ComputeTractRawFlammability(tx, tz)` (§6.2): building > prop >
    /// flora, a 0 counting as absent.
    pub fn tract_raw(&self, tx: i32, tz: i32) -> u8 {
        let s = self.d.borrow().tuning.tract_size;
        let x0 = tx.wrapping_mul(s);
        let z0 = tz.wrapping_mul(s);
        let rect = (
            x0,
            z0,
            x0.wrapping_add(s).wrapping_sub(1),
            z0.wrapping_add(s).wrapping_sub(1),
        );
        let (mut b, mut p, mut f) = (0u8, 0u8, 0u8);
        for occ in self.host.occupants_in_cells(rect) {
            let t = self.host.occupant_type(occ);
            let v = self.raw_value_of_type(occ, t);
            let slot = match t {
                occupant_type::BUILDING => &mut b,
                occupant_type::PROP => &mut p,
                _ => &mut f,
            };
            *slot = (*slot).max(v);
        }
        if b > 0 {
            b
        } else if p > 0 {
            p
        } else {
            f
        }
    }

    /// `OccupantUpdate(occ, add)` (§6.3).
    pub fn occupant_update(&self, occ: OccupantId, add: bool) {
        let t = self.host.occupant_type(occ);
        if t != occupant_type::BUILDING && t != occupant_type::FLORA && t != occupant_type::PROP {
            return;
        }
        let v = self.raw_value_of_type(occ, t);
        if add && v == 0 {
            return;
        }
        let (x0, z0, x1, z1) = self.host.occupant_bounds(occ);
        // [quirk §14.4] shifted LEFT by the raw grid's tract shift (0 with S = 1).
        let shift = self.raw_grid().map_or(0, |g| g.geometry().safe_shift());
        let (mut x0, mut z0, mut x1, mut z1) = (
            x0.wrapping_shl(shift),
            z0.wrapping_shl(shift),
            x1.wrapping_shl(shift),
            z1.wrapping_shl(shift),
        );
        let (cx, cz, g) = {
            let d = self.d.borrow();
            (d.count_x, d.count_z, d.global_multiplier)
        };
        if x0 < 0 || x1 >= cx || z0 < 0 || z1 >= cz {
            let r = (x0.max(0), z0.max(0), x1.min(cx - 1), z1.min(cz - 1));
            // Disjoint → (0, 0, 0, 0): tract (0, 0) is updated [quirk §14.5].
            (x0, z0, x1, z1) = if r.0 > r.2 || r.1 > r.3 { (0, 0, 0, 0) } else { r };
        }
        for x in x0..=x1 {
            for z in z0..=z1 {
                let w = if add {
                    let r = self.raw_get(x, z);
                    self.raw_set(x, z, r.max(v));
                    v // [quirk §14.3]
                } else {
                    let w = self.tract_raw(x, z);
                    self.raw_set(x, z, w);
                    w
                };
                self.adjusted_set(x, z, adjusted_value(w, g));
            }
        }
    }

    /// `ComputeGlobalMultiplier()` (§6.4); the caller stores it.
    pub fn compute_global_multiplier(&self) -> f32 {
        let factor = self.host.cumulative_factor(prop::ORDINANCE_FACTOR, 0);
        let season = self.host.current_season();
        global_multiplier(factor, season, self.d.borrow().tuning.summer_multiplier)
    }

    /// `DoDailySimulation` (§6.5).
    pub fn do_daily_simulation(&self) {
        let mut n = self.d.borrow().tracts_per_day;
        let mut wrapped = false;
        while n > 0 {
            let next = self.d.borrow_mut().iterator.next_cell();
            let Some((x, z)) = next else {
                if wrapped {
                    break; // [choice] a second wrap without a visit
                }
                wrapped = true;
                self.end_of_cycle();
                let mut crt = self.crt.borrow_mut();
                self.d.borrow_mut().iterator.reset(true, &mut crt);
                continue;
            };
            wrapped = false;
            let r = self.raw_get(x, z);
            let mut v = 0u8;
            if r > 0 {
                let (g, no_water, threshold) = {
                    let d = self.d.borrow();
                    (
                        d.global_multiplier,
                        d.tuning.no_water_multiplier,
                        d.tuning.hotspot_threshold,
                    )
                };
                let watered = self.host.is_cell_watered(x, z);
                v = daily_value(r, g, watered, no_water);
                self.adjusted_set(x, z, v);
                if i32::from(v) > threshold {
                    self.d.borrow_mut().candidates.push(Hotspot::new(x, z, 1));
                }
            }
            if self.host.zone_type(x, z) != 0 {
                let mut d = self.d.borrow_mut();
                d.zoned_count = d.zoned_count.wrapping_add(1);
                d.zoned_sum = d.zoned_sum.wrapping_add(i32::from(v));
            }
            n -= 1;
        }
    }

    /// `EndOfCycle` (§6.6).
    pub fn end_of_cycle(&self) {
        let average = {
            let mut d = self.d.borrow_mut();
            let a = if d.zoned_count != 0 {
                d.zoned_sum.wrapping_div(d.zoned_count) as u8
            } else {
                0
            };
            d.average = a;
            d.zoned_sum = 0;
            d.zoned_count = 0;
            a
        };
        self.host.post(msg::SIM_CYCLE_COMPLETE);
        self.host.record_history(HISTORY_AVERAGE, i32::from(average));
        self.compute_hotspot_list();
        self.d.borrow_mut().candidates.clear();
        self.fire_roll();
    }

    /// `ComputeHotspotList` (§6.7): rebuilds the hotspots from the
    /// candidates (which are kept; [`FlammabilitySim::end_of_cycle`] clears
    /// them).
    pub fn compute_hotspot_list(&self) {
        let mut d = self.d.borrow_mut();
        let h = compute_hotspots(&d.candidates, d.tuning.hotspot_radius, d.tuning.hotspot_min_weight);
        d.hotspots = h;
    }

    /// The fire roll (§6.8).
    pub fn fire_roll(&self) {
        if self.host.is_tutorial() {
            return;
        }
        let r = self.rng.borrow_mut().range_f64(0.0, 100.0);
        let (probability, hotspots, curve) = {
            let d = self.d.borrow();
            (
                d.tuning.random_fire_probability,
                d.hotspots.clone(),
                d.tuning.hotspot_fire_curve.clone(),
            )
        };
        let mut use_probability = true;
        let mut target = None;
        if r < f64::from(probability) {
            if let Some(b) = self.host.random_lot().and_then(|lot| self.host.lot_building(lot)) {
                let p = self.host.occupant_position(b);
                target = Some(self.host.position_to_cell(p[0], p[2]));
            }
        }
        if target.is_none() {
            let Some(curve) = curve else { return };
            if hotspots.is_empty() {
                return;
            }
            let mut order: Vec<usize> = (0..hotspots.len()).collect();
            self.crt.borrow_mut().random_shuffle(&mut order);
            let rf = r as f32;
            for i in order {
                let h = hotspots[i];
                if curve.eval(h.weight as f32) > rf {
                    let s = self.adjusted_grid().map_or(1, |g| g.geometry().size);
                    let half = s / 2;
                    target = Some((
                        h.x.wrapping_mul(s).wrapping_add(half),
                        h.z.wrapping_mul(s).wrapping_add(half),
                    ));
                    use_probability = false;
                    break;
                }
            }
        }
        let Some((cx, cz)) = target else { return };
        if self.host.ignite_cell(cx as u32, cz as u32, use_probability, false, 0) {
            self.host.set_fire_cause(FIRE_CAUSE);
        }
    }

    // ---- interface queries (§11) -------------------------------------------------------------------

    /// `IsCellFlammable(x, z)` (slot 6).
    pub fn is_cell_flammable(&self, x: i32, z: i32) -> bool {
        let (cx, cz) = {
            let d = self.d.borrow();
            (d.count_x, d.count_z)
        };
        if x < 0 || x >= cx || z < 0 || z >= cz {
            return false;
        }
        self.adjusted_grid().is_some_and(|g| g.get_cell_value(x, z) != 0)
    }

    /// `GetFlammabilityHotspot(&value, &x, &z)` (slot 7): `Some((value, x,
    /// z))` when it returns true; `None` = false and nothing written.
    pub fn flammability_hotspot(&self) -> Option<(u8, i32, i32)> {
        let best = {
            let d = self.d.borrow();
            let mut best: Option<Hotspot> = None;
            for h in &d.hotspots {
                if h.weight > best.map_or(0, |b| b.weight) {
                    best = Some(*h);
                }
            }
            best?
        };
        let g = self.adjusted_grid()?;
        let v = g.get(best.x, best.z);
        if v == 0 {
            return None;
        }
        let s = g.geometry().size;
        Some((v, s.wrapping_mul(best.x), s.wrapping_mul(best.z)))
    }

    /// `GetAverageFlammability()` (slot 8).
    pub fn average_flammability(&self) -> u8 {
        self.d.borrow().average
    }
}

// ---- persistence (§10) -------------------------------------------------------------------------------

impl SaveObject for FlammabilitySim {
    fn class_id(&self) -> u32 {
        CLSID_FLAMMABILITY_SIMULATOR
    }

    fn write(&self, out: &mut dyn OutStream) -> bool {
        let refs = |g: Option<GridRc>| g.map_or(RefValue::Null, |g| g.save_ref());
        out.put_u16(FLAMMABILITY_RECORD_VERSION);
        out.put_ref(&refs(self.adjusted_grid()));
        out.put_ref(&refs(self.raw_grid()));
        let d = self.d.borrow();
        out.put_f32(d.global_multiplier);
        out.put_u8(d.average);
        out.put_u32(d.hotspots.len() as u32);
        for h in &d.hotspots {
            out.put_i32(h.x);
            out.put_i32(h.z);
            out.put_i32(h.weight);
        }
        out.error() == 0
    }

    fn read(&self, inp: &mut dyn InStream) -> bool {
        if inp.get_u16() != Some(FLAMMABILITY_RECORD_VERSION) {
            return false;
        }
        for slot in [&self.adjusted, &self.raw] {
            let g = inp.get_ref_as(IID_SIM_GRID).and_then(|r| self.host.resolve_grid(&r));
            *slot.borrow_mut() = g;
        }
        let mut d = self.d.borrow_mut();
        if let Some(f) = inp.get_f32() {
            d.global_multiplier = f;
        }
        if let Some(a) = inp.get_u8() {
            d.average = a;
        }
        let count = inp.get_u32().unwrap_or(0);
        for _ in 0..count {
            let (Some(x), Some(z), Some(w)) = (inp.get_i32(), inp.get_i32(), inp.get_i32()) else {
                break;
            };
            d.hotspots.push(Hotspot::new(x, z, w));
        }
        inp.error() == 0
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
