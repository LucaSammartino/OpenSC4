// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cISC4AuraSimulator`: state (§3), lifecycle (§4), messages (§5),
//! occupant cones (§7), interface methods (§8) and the save record (§11).
//! The monthly pipeline (§6) is in `pipeline.rs`.

use std::any::Any;
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::grid::{add_radius_effect, GridProvider, TractGrid, IID_SIM_GRID};
use crate::pacing::{self, Pacing};
use crate::stream::{InStream, OutStream, RefValue, SaveObject};
use crate::variant::Elem;

use super::{
    grid_id, ids, msg, AuraHost, AuraMessage, AuraRandom, AuraTuning, LotIterator, Strike, AGENT_FLAGS, AGENT_LEVELS,
    AGENT_NAME, AURA_RECORD_VERSION, CLSID_AURA_SIMULATOR, DAY_STEP_MS,
};

/// [choice] The most lots one `DoIdle` call processes (§6.5), whatever the
/// host's timer says.
pub const MAX_LOTS_PER_CALL: u32 = 1 << 20;

/// One hotspot (§6.16), in tract units.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Hotspot {
    pub x: i32,
    pub z: i32,
    pub weight: i32,
}

/// The adaptive step of the time-slice helper (§6.5). In the original it
/// is a process-wide static; FreeCity4 keeps it per simulator ([choice],
/// simulator §5.5). `Init`, `Shutdown` and a load do not reset it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SliceStep {
    /// Added to the call's counter per iteration; the clock is read when the
    /// counter is a multiple of 1024 (1..=1024, a power of two).
    pub step: u32,
    /// The signed miss counter.
    pub misses: i32,
}

impl Default for SliceStep {
    fn default() -> Self {
        SliceStep { step: 1024, misses: 0 }
    }
}

impl SliceStep {
    /// The adaptation at the end of a state-5 call, with `n = count >> 10`
    /// (the number of clock reads, §6.5): `n > 20` counts a miss (at 16 the
    /// step halves), `1 ≤ n < 5` counts down (at −4 the step doubles), and
    /// `n = 0` or `5 ≤ n ≤ 20` changes nothing.
    pub fn adapt(&mut self, n: u32) {
        if n > 20 {
            self.misses = self.misses.saturating_add(1);
            if self.misses >= 16 {
                self.misses = 0;
                if self.step > 1 {
                    self.step /= 2;
                }
            }
        } else if (1..5).contains(&n) {
            self.misses = self.misses.saturating_sub(1);
            if self.misses <= -4 {
                self.misses = 0;
                if self.step < 1024 {
                    self.step *= 2;
                }
            }
        }
    }
}

/// The plain fields of §3 (grids are held separately).
#[derive(Clone, Debug, PartialEq)]
pub struct AuraData {
    pub initialized: bool,
    /// Pipeline state 0..17 (§6).
    pub state: i32,
    pub cycle_start_day: u32,
    /// `None` = −1 (§6.3).
    pub department_cursor: Option<u32>,
    pub mayor_rating: i32,
    pub unused_rating: i32,
    pub global_effect: i32,
    pub previous_tax_rate: [f32; 12],
    pub effective_base_aura: [i32; 3],
    pub tuning: AuraTuning,
    pub max_rating: i16,
    pub max_location: (i32, i32),
    pub min_rating: i16,
    pub min_location: (i32, i32),
    /// (tract x, z, 1) in order.
    pub low_tracts: Vec<(i32, i32, i32)>,
    pub hotspots: Vec<Hotspot>,
    pub riot_roll: f32,
    pub re_add_landmarks: bool,
}

impl Default for AuraData {
    /// The constructor values (§3).
    fn default() -> Self {
        AuraData {
            initialized: false,
            state: 0,
            cycle_start_day: 0,
            department_cursor: None,
            mayor_rating: 0,
            unused_rating: 0,
            global_effect: 0,
            previous_tax_rate: [-1.0; 12],
            effective_base_aura: [0; 3],
            tuning: AuraTuning::default(),
            max_rating: 0,
            max_location: (0, 0),
            min_rating: 0,
            min_location: (0, 0),
            low_tracts: Vec::new(),
            hotspots: Vec::new(),
            riot_roll: 0.0,
            re_add_landmarks: false,
        }
    }
}

pub(crate) type G8 = Rc<dyn TractGrid<i8>>;
pub(crate) type G16 = Rc<dyn TractGrid<i16>>;

/// The grids of §2.
#[derive(Default, Clone)]
pub(crate) struct Grids {
    pub(crate) aura: Option<G8>,
    pub(crate) base: Option<G8>,
    pub(crate) transient: Option<G8>,
    pub(crate) work: Option<G8>,
    pub(crate) building_effect: Option<G8>,
    pub(crate) park_map: Option<G16>,
    pub(crate) raw_park: Option<G16>,
    pub(crate) landmark_map: Option<G16>,
    pub(crate) landmark_work: Option<G16>,
    /// Department ID → landmark grid, ascending.
    pub(crate) departments: BTreeMap<u32, G16>,
}

/// The aura simulator.
pub struct AuraSim {
    pub(crate) host: Rc<dyn AuraHost>,
    token: Cell<u64>,
    pub(crate) d: RefCell<AuraData>,
    pub(crate) g: RefCell<Grids>,
    pub(crate) rng: RefCell<Box<dyn AuraRandom>>,
    pub(crate) lots: RefCell<Option<Box<dyn LotIterator>>>,
    pub(crate) slice: Cell<SliceStep>,
    /// simulator.md §5.7: captured from [`pacing::process`] at construction.
    pub(crate) pacing: Cell<Pacing>,
}

impl AuraSim {
    /// A new simulator with constructor values (§3) and the given generator
    /// (normally a wall-clock seeded [`super::AuraRng`]).
    pub fn new(host: Rc<dyn AuraHost>, token: u64, rng: Box<dyn AuraRandom>) -> Rc<AuraSim> {
        Rc::new(AuraSim {
            host,
            token: Cell::new(token),
            d: RefCell::new(AuraData::default()),
            g: RefCell::new(Grids::default()),
            rng: RefCell::new(rng),
            lots: RefCell::new(None),
            slice: Cell::new(SliceStep::default()),
            pacing: Cell::new(pacing::process()),
        })
    }

    /// The pacing this simulator runs (simulator.md §5.7).
    pub fn pacing(&self) -> Pacing {
        self.pacing.get()
    }

    /// Shadows and tests: [`Pacing::Original`] keeps the clock-bound steps.
    pub fn set_pacing(&self, p: Pacing) {
        self.pacing.set(p);
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn set_random(&self, rng: Box<dyn AuraRandom>) {
        *self.rng.borrow_mut() = rng;
    }

    /// The plain state.
    pub fn data(&self) -> Ref<'_, AuraData> {
        self.d.borrow()
    }

    /// Direct state access, for hosts that restore state and for tests.
    pub fn data_mut(&self) -> RefMut<'_, AuraData> {
        self.d.borrow_mut()
    }

    pub fn slice_step(&self) -> SliceStep {
        self.slice.get()
    }

    pub fn set_slice_step(&self, s: SliceStep) {
        self.slice.set(s);
    }

    /// Whether a lot iterator is held (§6.4, §6.5).
    pub fn has_lot_iterator(&self) -> bool {
        self.lots.borrow().is_some()
    }

    // ---- grid accessors ----------------------------------------------------------------------------

    /// `GetAuraGrid()` (no reference added).
    pub fn aura_grid(&self) -> Option<G8> {
        self.g.borrow().aura.clone()
    }
    /// `GetTransientAuraGrid()`.
    pub fn transient_grid(&self) -> Option<G8> {
        self.g.borrow().transient.clone()
    }
    /// `GetParkMap()`.
    pub fn park_map(&self) -> Option<G16> {
        self.g.borrow().park_map.clone()
    }
    /// `GetLandmarkMap()`.
    pub fn landmark_map(&self) -> Option<G16> {
        self.g.borrow().landmark_map.clone()
    }
    pub fn base_grid(&self) -> Option<G8> {
        self.g.borrow().base.clone()
    }
    pub fn work_grid(&self) -> Option<G8> {
        self.g.borrow().work.clone()
    }
    pub fn building_effect_grid(&self) -> Option<G8> {
        self.g.borrow().building_effect.clone()
    }
    pub fn raw_park_grid(&self) -> Option<G16> {
        self.g.borrow().raw_park.clone()
    }
    pub fn landmark_work_grid(&self) -> Option<G16> {
        self.g.borrow().landmark_work.clone()
    }
    pub fn department_grid(&self, dept: u32) -> Option<G16> {
        self.g.borrow().departments.get(&dept).cloned()
    }
    /// The department map's keys, ascending.
    pub fn department_ids(&self) -> Vec<u32> {
        self.g.borrow().departments.keys().copied().collect()
    }

    fn new_i8(&self, instance: u32, size: i32) -> Option<G8> {
        GridProvider::<i8>::create_grid(&*self.host, instance, size)
    }

    fn new_i16(&self, instance: u32, size: i32) -> Option<G16> {
        GridProvider::<i16>::create_grid(&*self.host, instance, size)
    }

    // ---- lifecycle (§4) ----------------------------------------------------------------------------

    /// `Init()` (§4.1): always true.
    pub fn init(&self) -> bool {
        if self.d.borrow().initialized {
            return true;
        }
        {
            let mut d = self.d.borrow_mut();
            d.tuning.pollution = [None, None, None, None];
            d.state = 0;
        }
        *self.lots.borrow_mut() = None;
        self.create_missing_grids();
        self.d.borrow_mut().state = 1;
        self.read_tuning();
        self.host.subscribe(&msg::SUBSCRIPTIONS);
        for level in AGENT_LEVELS {
            self.host.add_agent(level, AGENT_NAME, AGENT_FLAGS);
        }
        // [choice] A registry failure is not an error (§4.3).
        let _ = self.host.registry_add();
        self.d.borrow_mut().initialized = true;
        true
    }

    /// §4.1 step 3: every grid of §2 that is still missing.
    fn create_missing_grids(&self) {
        let s = grid_id::TRACT_SIZE;
        let mut g = self.g.borrow().clone();
        let fill8 = |grid: &Option<G8>, v: Option<i8>| {
            if let (Some(grid), Some(v)) = (grid, v) {
                grid.fill(v);
            }
        };
        let fill16 = |grid: &Option<G16>, v: Option<i16>| {
            if let (Some(grid), Some(v)) = (grid, v) {
                grid.fill(v);
            }
        };
        macro_rules! ensure {
            ($field:ident, $make:ident, $inst:expr, $size:expr, $fill:ident, $v:expr) => {
                if g.$field.is_none() {
                    g.$field = self.$make($inst, $size);
                    $fill(&g.$field, $v);
                }
            };
        }
        ensure!(aura, new_i8, grid_id::AURA, s, fill8, Some(0));
        ensure!(base, new_i8, grid_id::BASE, s, fill8, Some(-128));
        ensure!(transient, new_i8, grid_id::TRANSIENT, s, fill8, Some(0));
        ensure!(work, new_i8, grid_id::WORK, grid_id::WORK_TRACT_SIZE, fill8, None);
        ensure!(building_effect, new_i8, grid_id::BUILDING_EFFECT, s, fill8, Some(0));
        ensure!(park_map, new_i16, grid_id::PARK_MAP, s, fill16, None);
        ensure!(raw_park, new_i16, grid_id::RAW_PARK, s, fill16, Some(0));
        ensure!(landmark_map, new_i16, grid_id::LANDMARK_MAP, s, fill16, Some(0));
        ensure!(landmark_work, new_i16, grid_id::LANDMARK_MAP, s, fill16, Some(0));
        *self.g.borrow_mut() = g;
    }

    /// `Shutdown()` (§4.2): always true.
    pub fn shutdown(&self) -> bool {
        if !self.d.borrow().initialized {
            return true;
        }
        self.d.borrow_mut().initialized = false;
        self.host.registry_remove();
        self.d.borrow_mut().tuning.riot = None;
        self.host.remove_agent();
        self.host.unsubscribe(&msg::SUBSCRIPTIONS);
        // Dropping the handles is each grid's Shutdown + Release.
        let old = std::mem::take(&mut *self.g.borrow_mut());
        drop(old);
        *self.lots.borrow_mut() = None;
        self.d.borrow_mut().tuning.delete_curves();
        true
    }

    /// Reads the tuning exemplar (§10); nothing changes if it is missing.
    pub fn read_tuning(&self) {
        let Some(h) = self.host.tuning() else { return };
        let base = {
            let mut d = self.d.borrow_mut();
            d.tuning.read(&h);
            d.tuning.base_aura
        };
        self.host.publish_base_aura(base);
    }

    // ---- messages (§5) -----------------------------------------------------------------------------

    /// `DoMessage` (§5): true except for a late SimNewDay.
    pub fn do_message(&self, m: &AuraMessage) -> bool {
        match m.msg_type {
            msg::SIM_BEGIN => true,
            msg::SIM_NEW_DAY => {
                self.do_idle(DAY_STEP_MS);
                let state = self.d.borrow().state;
                let elapsed = self
                    .host
                    .sim_date_number()
                    .wrapping_sub(self.d.borrow().cycle_start_day);
                !(state != 0 && (state as u32) < elapsed)
            }
            msg::SIM_NEW_MONTH => {
                let mut d = self.d.borrow_mut();
                if d.state == 0 {
                    d.state = 1;
                }
                true
            }
            msg::SIM_IDLE => {
                if self.pacing.get().is_deterministic() {
                    self.settle_cycle();
                } else {
                    self.do_idle(m.data3);
                }
                true
            }
            msg::POST_CITY_INIT => {
                self.post_city_init();
                true
            }
            msg::OCCUPANT_INSERTED | msg::OCCUPANT_REMOVED => {
                if !self.host.occupant_has_property(m.void1, ids::PROP_PREVIEW_OCCUPANT) {
                    self.occupant_changed(m.void1, m.msg_type == msg::OCCUPANT_INSERTED);
                }
                true
            }
            msg::ORDINANCE_UPDATED => {
                let v = self.host.cumulative_offset(ids::ORDINANCE_AURA_EFFECT, 0);
                self.d.borrow_mut().global_effect = v;
                true
            }
            msg::EXEMPLAR_CHANGED => {
                if m.data1 == super::TUNING_KEY.group && m.data2 == super::TUNING_KEY.instance {
                    self.read_tuning();
                }
                true
            }
            _ => true,
        }
    }

    /// PostCityInit (§5.2): re-stamps every landmark after a version-8 load.
    /// The flag is never cleared [quirk].
    fn post_city_init(&self) {
        if !self.d.borrow().re_add_landmarks {
            return;
        }
        let mut it = self.host.lot_iterator();
        for _ in 0..MAX_LOTS_PER_CALL {
            let Some(lot) = it.next_lot() else { break };
            let Some(occ) = lot.building else { continue };
            if self.host.occupant_has_property(occ, ids::PROP_LANDMARK_EFFECT) {
                self.occupant_changed(occ, false);
                self.occupant_changed(occ, true);
            }
        }
    }

    // ---- occupant effects (§7) ---------------------------------------------------------------------

    /// `OccupantChanged(occupant, add)` (§7).
    pub fn occupant_changed(&self, occ: u64, add: bool) {
        let sign: i64 = if add { 1 } else { -1 };
        // [choice] AU4: fewer than two values is absent (no cone).
        let values = |prop: u32| match self.host.occupant_sint32s(occ, prop) {
            Some(v) if v.len() >= 2 => Some((v[0], v[1])),
            _ => None,
        };
        let park = values(ids::PROP_PARK_EFFECT);
        let landmark = values(ids::PROP_LANDMARK_EFFECT);
        let building = values(ids::PROP_MAYOR_RATING_EFFECT);
        let network = add && self.host.occupant_type(occ) == ids::OCCUPANT_TYPE_NETWORK;
        // §14.3.1's native helper capture has no position read for a no-op
        // occupant. Do not request an input that no cone consumes.
        if !network && park.is_none() && landmark.is_none() && building.is_none() {
            return;
        }
        let pos = self.host.occupant_position(occ);
        if network {
            self.add_transient_effect_type(pos[0], pos[2], 3);
        }
        let (cx, cz) = self.centre_cell(pos[0], pos[2]);
        let cone16 = |grid: &G16, mag: i32, radius: i32| {
            let geo = grid.geometry();
            let (tx, tz) = geo.cell_to_tract(cx, cz);
            let r = (f64::from(radius) / f64::from(geo.tract_size)) as f32;
            let c = (sign * i64::from(mag)) as f32;
            add_radius_effect(&**grid, geo.count_x, geo.count_z, tx, tz, r, c, 0.0);
        };
        if let Some((mag, radius)) = park {
            if let Some(g) = self.raw_park_grid() {
                cone16(&g, mag, radius);
            }
        }
        if let Some((mag, radius)) = landmark {
            let dept = self
                .host
                .budget_item_department(occ, ids::PURPOSE_LANDMARK)
                .unwrap_or(0);
            let grid = match self.department_grid(dept) {
                Some(g) => Some(g),
                None if add => {
                    let g = self.new_i16(dept, grid_id::TRACT_SIZE);
                    if let Some(g) = &g {
                        g.fill(0);
                        self.g.borrow_mut().departments.insert(dept, g.clone());
                    }
                    g
                }
                None => None,
            };
            if let Some(g) = grid {
                cone16(&g, mag, radius);
            }
        }
        if let Some((mag, radius)) = building {
            if let Some(g) = self.building_effect_grid() {
                let geo = g.geometry();
                let (tx, tz) = geo.position_to_tract(pos[0], pos[2]);
                let r = (f64::from(radius) / f64::from(geo.tract_size)) as f32;
                let c = (sign * i64::from(mag)) as f32;
                add_radius_effect(&*g, geo.count_x, geo.count_z, tx, tz, r, c, 0.0);
            }
        }
    }

    /// The centre cell of §7 step 2: the position's cell, or the centre of
    /// the lot on it.
    pub fn centre_cell(&self, x: f32, z: f32) -> (i32, i32) {
        let (cx, cz) = self.host.position_to_cell(x, z);
        let Some(lot) = self.host.lot_at(cx, cz) else {
            return (cx, cz);
        };
        let (wx, wz) = self.host.cell_width();
        (axis_centre(lot.x1, lot.x2, x, wx), axis_centre(lot.z1, lot.z2, z, wz))
    }

    // ---- interface methods (§8) --------------------------------------------------------------------

    /// `AddTransientEffect(x, z, type)` (§8.1); `type ≥ 4` is ignored
    /// [choice].
    pub fn add_transient_effect_type(&self, x: f32, z: f32, kind: u32) {
        if kind >= 4 {
            return;
        }
        let (mag, radius) = {
            let t = &self.d.borrow().tuning;
            (t.transient_magnitude[kind as usize], t.transient_radius[kind as usize])
        };
        if mag != 0.0 && radius != 0.0 {
            self.add_transient_effect(x, z, radius, mag);
        }
    }

    /// `AddTransientEffect(x, z, radius, magnitude)` (§8.2): world units.
    pub fn add_transient_effect(&self, x: f32, z: f32, radius: f32, magnitude: f32) {
        let Some(g) = self.transient_grid() else { return };
        let geo = g.geometry();
        let (tx, tz) = geo.position_to_tract(x, z);
        add_radius_effect(
            &*g,
            geo.count_x,
            geo.count_z,
            tx,
            tz,
            radius / geo.width_x,
            magnitude,
            0.0,
        );
    }

    /// `GetAuraValue(x, z)` (cells).
    pub fn aura_value(&self, x: i32, z: i32) -> i8 {
        self.aura_grid().map_or(0, |g| g.get_cell_value(x, z))
    }

    /// `GetMayorRating()`: the low 8 bits.
    pub fn mayor_rating(&self) -> i8 {
        self.d.borrow().mayor_rating as i8
    }

    /// `GetMaxMayorRating(&p)`: (rating in AL, location in cells).
    pub fn max_mayor_rating(&self) -> (i8, (i32, i32)) {
        let d = self.d.borrow();
        (d.max_rating as i8, d.max_location)
    }

    /// `GetMinMayorRating(&p)`.
    pub fn min_mayor_rating(&self) -> (i8, (i32, i32)) {
        let d = self.d.borrow();
        (d.min_rating as i8, d.min_location)
    }

    /// `BustStrike(type)` (§8.4).
    pub fn bust_strike(&self, kind: u32) {
        let order = [
            (1, Strike::Police),
            (2, Strike::Fire),
            (3, Strike::Health),
            (4, Strike::School),
        ];
        for (k, s) in order {
            if kind == k || kind == 0 {
                self.host.end_strike(s);
            }
        }
    }

    /// The registry command `add_transient_effect(x, z, radius, magnitude)`
    /// (§4.3): true.
    pub fn registry_add_transient_effect(&self, params: &[Elem]) -> bool {
        let f = |i: usize| params.get(i).map_or(0.0, |e| e.as_f64() as f32);
        self.add_transient_effect(f(0), f(1), f(2), f(3));
        true
    }

    /// The registry command `end_strike(type)` (§4.3): true.
    pub fn registry_end_strike(&self, params: &[Elem]) -> bool {
        self.bust_strike(params.first().map_or(0, |e| e.as_i64() as u32));
        true
    }
}

/// Lot centre along one axis (§7 step 2).
fn axis_centre(x1: i32, x2: i32, p: f32, cell_width: f32) -> i32 {
    let width = i64::from(x2) - i64::from(x1) + 1;
    let half = width / 2;
    let mid = i64::from(x1) + half;
    let r = if width % 2 != 0 {
        mid
    } else {
        let near = x1 as f32 * cell_width;
        let far = (i64::from(x1) + width) as f32 * cell_width;
        if (far - p).abs() <= (p - near).abs() {
            mid
        } else {
            mid - 1
        }
    };
    r as i32
}

// ---- persistence (§11) --------------------------------------------------------------------------------

fn put_grid<T: crate::grid::GridElem>(out: &mut dyn OutStream, g: &Option<Rc<dyn TractGrid<T>>>) {
    out.put_ref(&g.as_ref().map_or(RefValue::Null, |g| g.save_ref()));
}

impl AuraSim {
    fn get8(&self, inp: &mut dyn InStream) -> Option<Option<G8>> {
        let r = inp.get_ref_as(IID_SIM_GRID)?;
        Some(GridProvider::<i8>::resolve_grid(&*self.host, &r))
    }

    fn get16(&self, inp: &mut dyn InStream) -> Option<Option<G16>> {
        let r = inp.get_ref_as(IID_SIM_GRID)?;
        Some(GridProvider::<i16>::resolve_grid(&*self.host, &r))
    }

    /// The department block (§11.2), **merged** into the map (§11.3, AU1:
    /// `map[departmentID] := grid`; IDs not in the record stay). [choice] A
    /// null or failed reference adds or changes nothing; a duplicate ID
    /// keeps the later grid; a grid replaced is released (the original
    /// leaks it). Nothing changes when the count cannot be read.
    fn read_departments(&self, inp: &mut dyn InStream) {
        let Some(count) = inp.get_u32() else { return };
        for _ in 0..count {
            let Some(id) = inp.get_u32() else { break };
            let Some(g) = self.get16(inp) else { break };
            if let Some(g) = g {
                self.g.borrow_mut().departments.insert(id, g);
            }
        }
    }
}

impl SaveObject for AuraSim {
    fn class_id(&self) -> u32 {
        CLSID_AURA_SIMULATOR
    }

    /// The version-9 record (§11.2); true when the stream is still good.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        let g = self.g.borrow().clone();
        out.put_u16(AURA_RECORD_VERSION);
        put_grid(out, &g.aura);
        put_grid(out, &g.park_map);
        put_grid(out, &g.raw_park);
        put_grid(out, &g.base);
        put_grid(out, &g.transient);
        put_grid(out, &g.building_effect);
        put_grid(out, &g.landmark_map);
        out.put_u32(g.departments.len() as u32);
        for (&id, grid) in &g.departments {
            out.put_u32(id);
            out.put_ref(&grid.save_ref());
        }
        let d = self.d.borrow();
        out.put_i32(d.unused_rating);
        out.put_i32(d.mayor_rating);
        out.put_i32(d.max_location.0);
        out.put_i32(d.max_location.1);
        out.put_i16(d.max_rating);
        out.put_i32(d.min_location.0);
        out.put_i32(d.min_location.1);
        out.put_i16(d.min_rating);
        out.put_u32(d.hotspots.len() as u32);
        for h in &d.hotspots {
            out.put_i32(h.x);
            out.put_i32(h.z);
            out.put_i32(h.weight);
        }
        out.error() == 0
    }

    /// §11.3: versions 8 and 9; anything else is false [choice above 9].
    /// Fields are written as they are read. A grid reference sets its slot
    /// to null first (foundations §2.7), so a reference that fails, and
    /// every later one once the stream is in error, leaves its slot empty
    /// (the previous grid is released, [choice]: the original leaks it). A
    /// plain field whose read fails keeps its value.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        let Some(version) = inp.get_u16() else { return false };
        if !(8..=AURA_RECORD_VERSION).contains(&version) {
            return false;
        }
        // Order: aura, park map, raw park, base, transient, building effect,
        // landmark map.
        let aura = self.get8(inp).flatten();
        self.g.borrow_mut().aura = aura;
        let park = self.get16(inp).flatten();
        self.g.borrow_mut().park_map = park;
        let raw = self.get16(inp).flatten();
        self.g.borrow_mut().raw_park = raw;
        let base = self.get8(inp).flatten();
        self.g.borrow_mut().base = base;
        let transient = self.get8(inp).flatten();
        self.g.borrow_mut().transient = transient;
        let building = self.get8(inp).flatten();
        self.g.borrow_mut().building_effect = building;
        let landmark = self.get16(inp).flatten();
        self.g.borrow_mut().landmark_map = landmark;
        if version == 8 {
            // The old single landmark grid: read, then Shutdown + Release.
            drop(self.get16(inp));
            self.d.borrow_mut().re_add_landmarks = true;
        } else {
            self.read_departments(inp);
        }
        {
            let mut d = self.d.borrow_mut();
            let mut set_i32 = |slot: &mut i32| {
                if let Some(v) = inp.get_i32() {
                    *slot = v;
                }
            };
            set_i32(&mut d.unused_rating);
            set_i32(&mut d.mayor_rating);
            set_i32(&mut d.max_location.0);
            set_i32(&mut d.max_location.1);
            if let Some(v) = inp.get_i16() {
                d.max_rating = v;
            }
            let mut set_i32 = |slot: &mut i32| {
                if let Some(v) = inp.get_i32() {
                    *slot = v;
                }
            };
            set_i32(&mut d.min_location.0);
            set_i32(&mut d.min_location.1);
            if let Some(v) = inp.get_i16() {
                d.min_rating = v;
            }
            // §11.3 (AU1): appended after the hotspots already held.
            if let Some(count) = inp.get_u32() {
                for _ in 0..count {
                    let (Some(x), Some(z), Some(weight)) = (inp.get_i32(), inp.get_i32(), inp.get_i32()) else {
                        break;
                    };
                    d.hotspots.push(Hotspot { x, z, weight });
                }
            }
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
