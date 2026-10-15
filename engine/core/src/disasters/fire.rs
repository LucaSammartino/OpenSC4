// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The fire (§5): one persistent fire instance owned by the fire manager;
//! every fire in the city is an entry in it.

use std::collections::{BTreeSet, VecDeque};

use super::layer::Timer;
use super::system::{Core, InstanceState};
use super::{
    history, instance_class, kind, msg, occupant, prop, zoom_of, CellRect, DisasterPost, BURN_DOWN_FLAGS,
    DEFAULT_MAX_STAGE, FIRE_PERIOD_MS, TICK_PRIORITY, TOAST_PER_UPDATE,
};
use crate::curve::ResponseCurve;
use crate::property::PropertyHolder;
use crate::x87::rhe;

/// Tuning properties of the "Flame" exemplar (§5.1).
pub mod fire_prop {

    pub const D3_DURATIONS: u32 = 0xA9B6_ECD3;
    pub const D4_SPREAD_DIVISOR: u32 = 0xA9B6_ECD4;
    pub const D5_SPREAD_RADIUS: u32 = 0xA9B6_ECD5;
    pub const D6_WIND_FACTOR: u32 = 0xA9B6_ECD6;
    pub const D7_TOP_FLAMES: u32 = 0xA9B6_ECD7;
    pub const D8_SIDE_FLAMES: u32 = 0xA9B6_ECD8;
    pub const D9_COVERAGE: u32 = 0xA9B6_ECD9;
    pub const DA_IGNITION: u32 = 0xA9B6_ECDA;
}

/// The code defaults of §5.1.
pub const DEFAULT_D3: [u32; 5] = [10; 5];
pub const DEFAULT_D4: [u32; 5] = [10; 5];
pub const DEFAULT_D5: [u32; 5] = [0, 3, 5, 7, 9];
pub const DEFAULT_ZOOM: u32 = 4;
/// The shipped "Flame" values (§5.1).
pub const SHIPPED_D3: [u32; 5] = [30, 25, 20, 20, 25];
pub const SHIPPED_D4: [u32; 5] = [4, 3, 2, 2, 1];
pub const SHIPPED_D5: [u32; 5] = [32, 48, 48, 80, 80];
pub const SHIPPED_D6: f32 = 1.0;
pub const SHIPPED_D7: [(f32, f32); 2] = [(0.0, 1.0), (10000.0, 5.0)];
pub const SHIPPED_D8: [(f32, f32); 3] = [(0.0, 0.0), (10000.0, 6.0), (50000.0, 8.0)];
pub const SHIPPED_D9: [(f32, f32); 7] = [
    (0.0, 0.0),
    (0.25, 0.1),
    (50.0, 0.5),
    (75.0, 1.5),
    (100.0, 4.0),
    (150.0, 8.0),
    (200.0, 10.0),
];
pub const SHIPPED_DA: [(f32, f32); 8] = [
    (0.0, 0.0),
    (30.0, 5.0),
    (50.0, 10.0),
    (75.0, 20.0),
    (90.0, 30.0),
    (115.0, 40.0),
    (160.0, 50.0),
    (200.0, 50.0),
];
pub const SHIPPED_ZOOM: u32 = 3;

/// The fire tuning as read (§5.1).
#[derive(Clone, Debug, PartialEq)]
pub struct FireTuning {
    pub d3: [u32; 5],
    /// The cumulative stage thresholds (`D3cum`).
    pub d3cum: [u32; 5],
    pub d4: [u32; 5],
    pub d5: [u32; 5],
    pub d6: f32,
    pub d7: ResponseCurve,
    pub d8: ResponseCurve,
    pub d9: ResponseCurve,
    pub da: ResponseCurve,
    pub zoom: u32,
}

impl Default for FireTuning {
    fn default() -> FireTuning {
        FireTuning::from_holder(None)
    }
}

/// Five `Uint32` values: missing ones keep their default, extra ones are
/// ignored [choice, Q14].
fn five(h: Option<&PropertyHolder>, id: u32, default: [u32; 5]) -> [u32; 5] {
    let mut out = default;
    if let Some(v) = h.and_then(|h| crate::props::u32s(h, id)) {
        for (o, x) in out.iter_mut().zip(v) {
            *o = x;
        }
    }
    out
}

impl FireTuning {
    /// `ReadTunableValues` (§5.1): defaults where a property is missing; a
    /// missing curve is empty and evaluates to 0.
    pub fn from_holder(h: Option<&PropertyHolder>) -> FireTuning {
        let d3 = five(h, fire_prop::D3_DURATIONS, DEFAULT_D3);
        let mut d3cum = [0u32; 5];
        let mut acc = 0u32;
        for (c, d) in d3cum.iter_mut().zip(d3) {
            acc = acc.wrapping_add(d);
            *c = acc;
        }
        let curve = |id| h.and_then(|h| crate::props::curve(h, id)).unwrap_or_default();
        FireTuning {
            d3,
            d3cum,
            d4: five(h, fire_prop::D4_SPREAD_DIVISOR, DEFAULT_D4),
            d5: five(h, fire_prop::D5_SPREAD_RADIUS, DEFAULT_D5),
            d6: h
                .and_then(|h| crate::props::f32_of(h, fire_prop::D6_WIND_FACTOR))
                .unwrap_or(0.0),
            d7: curve(fire_prop::D7_TOP_FLAMES),
            d8: curve(fire_prop::D8_SIDE_FLAMES),
            d9: curve(fire_prop::D9_COVERAGE),
            da: curve(fire_prop::DA_IGNITION),
            zoom: zoom_of(h, DEFAULT_ZOOM),
        }
    }

    /// The stage of a counter (§5.10 step 7): the first `i` in 0..4 with
    /// `counter < D3cum[i]`, else 5.
    pub fn stage_of(&self, counter: i32) -> u32 {
        self.d3cum
            .iter()
            .position(|&t| i64::from(counter) < i64::from(t))
            .map_or(5, |i| i as u32)
    }

    /// `round(D9(coverage))`, half to even (§5.4 step 4).
    pub fn resistance(&self, coverage: u8) -> i32 {
        rhe(self.d9.eval(f32::from(coverage)))
    }
}

/// The shipped "Flame" exemplar (§5.1), for tests and tools.
pub fn shipped_flame() -> PropertyHolder {
    use crate::variant::{mem, Elem, Variant};
    let mut h = PropertyHolder::new();
    let u32s = |v: &[u32]| Variant::Array {
        elem: mem::UINT32,
        items: v.iter().map(|&x| Elem::U32(x)).collect(),
    };
    let curve = |v: &[(f32, f32)]| Variant::Array {
        elem: mem::FLOAT32,
        items: v.iter().flat_map(|&(x, y)| [Elem::F32(x), Elem::F32(y)]).collect(),
    };
    h.add(fire_prop::D3_DURATIONS, u32s(&SHIPPED_D3));
    h.add(fire_prop::D4_SPREAD_DIVISOR, u32s(&SHIPPED_D4));
    h.add(fire_prop::D5_SPREAD_RADIUS, u32s(&SHIPPED_D5));
    h.add(fire_prop::D6_WIND_FACTOR, Variant::Scalar(Elem::F32(SHIPPED_D6)));
    h.add(fire_prop::D7_TOP_FLAMES, curve(&SHIPPED_D7));
    h.add(fire_prop::D8_SIDE_FLAMES, curve(&SHIPPED_D8));
    h.add(fire_prop::D9_COVERAGE, curve(&SHIPPED_D9));
    h.add(fire_prop::DA_IGNITION, curve(&SHIPPED_DA));
    h.add(prop::ZOOM_LEVEL, Variant::Scalar(Elem::U8(SHIPPED_ZOOM as u8)));
    h
}

/// One fire (a fire occupant, §5.11).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FireEntry {
    /// The fire occupant (host token).
    pub occupant: u64,
    pub target: u64,
    pub counter: i32,
    pub stage: u32,
    pub max_stage: u32,
    pub fire_id: u32,
    pub hosed: u32,
    /// The extinguish time on the instance's timer (ms).
    pub extinguish_at: Option<u64>,
    pub top_flames: u32,
    pub side_flames: u32,
}

/// The fire instance's state (§5.2).
#[derive(Clone, Debug, Default)]
pub(crate) struct FireState {
    pub initialised: bool,
    pub fires: Vec<FireEntry>,
    pub fire_ids: BTreeSet<u32>,
    pub toast: VecDeque<u64>,
    pub timer: Timer,
    pub next_ms: u64,
    pub cause: u32,
    pub ignited_buildings: u32,
    pub ignited_flora: u32,
    pub no_burn_down: bool,
    pub city_cells: (u32, u32),
    pub toasted: u32,
}

/// The fire instance's observable state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FireSnapshot {
    pub instance: u32,
    pub phase: u32,
    pub fires: Vec<FireEntry>,
    pub fire_ids: Vec<u32>,
    pub toast: Vec<u64>,
    pub elapsed_ms: u64,
    pub next_ms: u64,
    pub cause: u32,
    pub ignited_buildings: u32,
    pub ignited_flora: u32,
    pub no_burn_down: bool,
    pub toasted: u32,
    pub tick_registered: bool,
    pub tuning: FireTuning,
}

impl Core {
    fn fs(&mut self) -> Option<&mut FireState> {
        let id = self.fire_instance?;
        match &mut self.instances.get_mut(&id)?.state {
            InstanceState::Fire(f) => Some(f),
            _ => None,
        }
    }

    fn fs_ref(&self) -> Option<&FireState> {
        let id = self.fire_instance?;
        match &self.instances.get(&id)?.state {
            InstanceState::Fire(f) => Some(f),
            _ => None,
        }
    }

    fn ft(&self) -> FireTuning {
        self.fire_tuning.clone().unwrap_or_default()
    }

    fn fire_now(&mut self) -> u64 {
        let now = self.host.now_ms();
        self.fs().map_or(0, |f| f.timer.elapsed_ms(now))
    }

    fn fire_phase(&self) -> u32 {
        self.fire_instance
            .and_then(|i| self.instances.get(&i))
            .map_or(0, |i| i.phase)
    }

    fn set_fire_phase(&mut self, phase: u32) {
        if let Some(i) = self.fire_instance.and_then(|i| self.instances.get_mut(&i)) {
            i.phase = phase;
        }
    }

    /// The fire manager's instance, created and `Init`ed on first use
    /// (flammability §13.1); `None` without a fire manager.
    pub(crate) fn ensure_fire(&mut self) -> Option<u32> {
        if let Some(id) = self.fire_instance {
            return Some(id);
        }
        if !self.managers.contains_key(&kind::FIRE) {
            return None;
        }
        let id = self.new_instance(kind::FIRE, instance_class::FIRE, InstanceState::Fire(Box::default()));
        self.fire_instance = Some(id);
        self.fire_init(id);
        Some(id)
    }

    /// `Init` (§5.3).
    fn fire_init(&mut self, id: u32) {
        let host = self.host.clone();
        if self.fire_tuning.is_none() {
            let h = host.tuning(super::tuning_instance::FLAME);
            self.fire_tuning = Some(FireTuning::from_holder(h.as_deref()));
        }
        self.add_instance_to_list(kind::FIRE, id);
        let cells = host.city_cells();
        let now = host.now_ms();
        if let Some(f) = self.fs() {
            f.city_cells = cells;
            f.timer.restart(now);
            f.initialised = true;
        }
        self.subscribe(id, msg::VIEW_LEVEL_CHANGED);
        self.subscribe(id, msg::VIEW_UPDATE);
        self.fire_update();
    }

    /// `Shutdown`: every fire's occupant removed, the toast list released
    /// [choice, Q13].
    pub(crate) fn fire_shutdown(&mut self, _id: u32) {
        let host = self.host.clone();
        let Some(f) = self.fs() else { return };
        let fires = std::mem::take(&mut f.fires);
        let toast = std::mem::take(&mut f.toast);
        f.fire_ids.clear();
        f.initialised = false;
        for e in fires {
            host.remove_fire_occupant(e.occupant, e.target);
        }
        for o in toast {
            host.release(o);
        }
    }

    // ---- §5.4 ----

    /// Manager slot 4: `Ignite(o, false, 0, true)`, true for any non-null
    /// occupant *[quirk]*.
    pub(crate) fn fire_ignite_occupant(&mut self, o: u64) -> bool {
        if o == 0 {
            return false;
        }
        if self.ensure_fire().is_some() {
            self.fire_ignite(o, false, 0, true);
        }
        true
    }

    /// `Ignite(occupant, force, fireID, focus)` (slot 6).
    pub(crate) fn fire_ignite(&mut self, o: u64, force: bool, fire_id: u32, focus: bool) -> bool {
        let Some(inst) = self.fire_instance else { return false };
        let host = self.host.clone();
        if o == 0 {
            return false;
        }
        if !force && host.sim_paused() && !host.sim_emergency_paused() {
            return false;
        }
        let t = host.occupant_type(o);
        if !occupant::burnable(t) {
            return false;
        }
        if !flammable(host.as_ref(), o) {
            return false;
        }
        let tuning = self.ft();
        if !force && tuning.resistance(host.fire_coverage(o)) > 0 {
            return false;
        }
        if host.occupant_prop_bool(o, prop::BURNING).unwrap_or(false) {
            if !force {
                return false;
            }
            if let Some(e) = self.fs().and_then(|f| f.fires.iter_mut().find(|e| e.target == o)) {
                e.counter = e.counter.wrapping_add(1);
            }
            return true;
        }
        let fire_id = if fire_id == 0 { self.rng_fire.raw_u32() } else { fire_id };
        let new_id = self.fs().is_some_and(|f| !f.fire_ids.contains(&fire_id));
        if new_id {
            self.register_location(inst, 0, 0, o, tuning.zoom, focus);
            if let Some(f) = self.fs() {
                f.fire_ids.insert(fire_id);
            }
        }
        let max_stage = host.occupant_prop_u32(o, prop::MAX_STAGE).unwrap_or(DEFAULT_MAX_STAGE);
        let (top, side) = host.occupant_bbox_areas(o);
        let flames = (
            super::trunc_i32(tuning.d7.eval(top)).max(0) as u32,
            super::trunc_i32(tuning.d8.eval(side)).max(0) as u32,
        );
        let Some(fo) = host.create_fire_occupant(inst, o, fire_id, flames) else {
            return false;
        };
        let counter = if force {
            if max_stage < 2 {
                (tuning.d3cum[0] / 2) as i32
            } else {
                tuning.d3cum[0] as i32
            }
        } else {
            0
        };
        if let Some(f) = self.fs() {
            f.fires.push(FireEntry {
                occupant: fo,
                target: o,
                counter,
                stage: 0,
                max_stage,
                fire_id,
                hosed: 0,
                extinguish_at: None,
                top_flames: flames.0,
                side_flames: flames.1,
            });
        }
        host.add_fire_aura(o);
        host.post(&DisasterPost {
            msg_type: msg::FIRE_STARTED_ENDED,
            data1: fo,
            data2: 1,
            ..DisasterPost::default()
        });
        if let Some(f) = self.fs() {
            if t == occupant::FLORA {
                f.ignited_flora = f.ignited_flora.wrapping_add(1);
            } else if t == occupant::BUILDING {
                f.ignited_buildings = f.ignited_buildings.wrapping_add(1);
            }
        }
        self.fire_update_phase();
        true
    }

    // ---- §5.5 ----

    /// `IgniteCell(x, z, useProbability, teleport, fireID)` (slot 7),
    /// flammability.md §13.1: the grid and the curve on every call; with
    /// `useProbability`, `r := RandomDoubleRangeUniform(0, 100)` and return
    /// false when `r > c`; a zero ID is drawn once per call; every filtered
    /// occupant of the cell is ignited. True if any ignited.
    pub(crate) fn fire_ignite_cell(
        &mut self,
        x: i32,
        z: i32,
        use_probability: bool,
        teleport: bool,
        fire_id: u32,
    ) -> bool {
        if self.fire_instance.is_none() {
            return false;
        }
        let host = self.host.clone();
        let v = host.flammability_at(x, z);
        let c = self.ft().da.eval(f32::from(v));
        if use_probability {
            let r = self.rng_fire.range_f64(0.0, 100.0);
            if r > f64::from(c) {
                return false;
            }
        }
        let fire_id = if fire_id == 0 { self.rng_fire.raw_u32() } else { fire_id };
        let mut any = false;
        for o in host.occupants_in_rect(CellRect::cell(x, z)) {
            if occupant::burnable(host.occupant_type(o)) {
                any |= self.fire_ignite(o, teleport, fire_id, !teleport);
            }
        }
        any
    }

    /// The manager's `IgniteCell` (slot 5) and `AttemptDisasterStart(x, z)`:
    /// forwards (creating the instance), true whatever the instance returned.
    pub(crate) fn fire_ignite_cell_manager(&mut self, x: i32, z: i32, p: bool, teleport: bool, fire_id: u32) -> bool {
        if self.ensure_fire().is_none() {
            return false;
        }
        self.fire_ignite_cell(x, z, p, teleport, fire_id);
        true
    }

    // ---- §5.6 ----

    /// `IgniteCellRadius(x, z, r0, r1)` (slot 10).
    pub(crate) fn fire_ignite_cell_radius(&mut self, x: i32, z: i32, r0: i32, r1: i32) {
        if self.ensure_fire().is_none() {
            return;
        }
        let host = self.host.clone();
        let id = self.rng_fire.raw_u32();
        let w = host.cell_width();
        let cx = (x as f32 + 0.5) * w;
        let cz = (z as f32 + 0.5) * w;
        let rect = self.clamp_rect(CellRect {
            x0: x.saturating_sub(r1),
            z0: z.saturating_sub(r1),
            x1: x.saturating_add(r1),
            z1: z.saturating_add(r1),
        });
        let near = r0 as f32 * w;
        let far = r1 as f32 * w;
        for o in host.occupants_in_rect(rect) {
            if !occupant::burnable(host.occupant_type(o)) {
                continue;
            }
            let (px, pz) = host.occupant_position(o);
            let d = ((px - cx) * (px - cx) + (pz - cz) * (pz - cz)).sqrt();
            if d < near {
                self.toast_push(o);
            } else if d < far {
                self.fire_ignite(o, false, id, false);
            }
        }
    }

    // ---- §5.7 ----

    fn toast_push(&mut self, o: u64) {
        let host = self.host.clone();
        if let Some(f) = self.fs() {
            host.add_ref(o);
            f.toast.push_back(o);
        }
    }

    /// `ExtinguishCell(x, z, amount)` (slot 8): every fire whose target's
    /// cell rectangle holds the cell [choice, Q15].
    pub(crate) fn fire_extinguish_cell(&mut self, x: i32, z: i32, amount: i32) {
        let host = self.host.clone();
        let Some(f) = self.fs_ref() else { return };
        let hits: Vec<u64> = f
            .fires
            .iter()
            .filter(|e| host.occupant_cell_rect(e.target).contains(x, z))
            .map(|e| e.occupant)
            .collect();
        for fo in hits {
            let Some(e) = self.fs().and_then(|f| f.fires.iter_mut().find(|e| e.occupant == fo)) else {
                continue;
            };
            let out = e.counter <= amount;
            if out {
                e.counter = 0;
            } else {
                e.counter -= amount;
            }
            if out {
                self.extinguish_fire(fo);
            }
        }
    }

    /// `QuickBurnCell(x, z)` (slot 9).
    pub(crate) fn fire_quick_burn_cell(&mut self, x: i32, z: i32) {
        if self.ensure_fire().is_none() {
            return;
        }
        let host = self.host.clone();
        for o in host.occupants_in_rect(CellRect::cell(x, z)) {
            if occupant::burnable(host.occupant_type(o)) {
                self.toast_push(o);
            }
        }
    }

    /// `ProcessToastList`: at most 5 entries per `Update`; each still
    /// flammable one is burned down (each processed entry counts toward the
    /// 5) [choice].
    fn process_toast_list(&mut self) {
        let host = self.host.clone();
        for _ in 0..TOAST_PER_UPDATE {
            let Some(o) = self.fs().and_then(|f| f.toast.pop_front()) else {
                break;
            };
            if flammable(host.as_ref(), o) {
                self.toast_occupant(o);
            }
            host.release(o);
        }
    }

    /// `ExtinguishFire(fire)`: the extinguish time, once.
    fn extinguish_fire(&mut self, fo: u64) {
        let now = self.fire_now();
        let has = self
            .fs_ref()
            .and_then(|f| f.fires.iter().find(|e| e.occupant == fo))
            .map(|e| e.extinguish_at.is_some());
        if has != Some(false) {
            return;
        }
        let delay = u64::from(self.rng_fire.uniform_u32(4000));
        if let Some(e) = self.fs().and_then(|f| f.fires.iter_mut().find(|e| e.occupant == fo)) {
            e.extinguish_at = Some(now + delay);
        }
    }

    /// `Extinguish(o)` (slot 11).
    pub(crate) fn fire_extinguish_occupant(&mut self, o: u64) {
        if let Some(fo) = self.fire_of_target(o) {
            self.extinguish_fire(fo);
        }
    }

    /// `BurnDown(o)` (slot 12).
    pub(crate) fn fire_burn_down_occupant(&mut self, o: u64) {
        if let Some(fo) = self.fire_of_target(o) {
            self.burn_down_fire(fo);
        }
    }

    fn fire_of_target(&self, o: u64) -> Option<u64> {
        self.fs_ref()?.fires.iter().find(|e| e.target == o).map(|e| e.occupant)
    }

    pub(crate) fn fire_set_cause(&mut self, cause: u32) {
        if self.ensure_fire().is_some() {
            if let Some(f) = self.fs() {
                f.cause = cause;
            }
        }
    }

    pub(crate) fn fire_cause(&self) -> u32 {
        self.fs_ref().map_or(0, |f| f.cause)
    }

    pub(crate) fn fire_count(&self) -> u32 {
        self.fs_ref().map_or(0, |f| f.fires.len() as u32)
    }

    /// `GetFireCountInCellRect(rect)` (slot 15): fires whose target's cell
    /// rectangle meets the rectangle [choice, Q15].
    pub(crate) fn fire_count_in_rect(&self, r: CellRect) -> u32 {
        self.fs_ref().map_or(0, |f| {
            f.fires
                .iter()
                .filter(|e| self.host.occupant_cell_rect(e.target).intersects(&r))
                .count() as u32
        })
    }

    /// The fire occupant's `SetHosed(+n / −n)`, clamped at 0.
    pub(crate) fn fire_set_hosed(&mut self, fo: u64, delta: i32) -> bool {
        match self.fs().and_then(|f| f.fires.iter_mut().find(|e| e.occupant == fo)) {
            Some(e) => {
                e.hosed = (i64::from(e.hosed) + i64::from(delta)).clamp(0, i64::from(u32::MAX)) as u32;
                true
            }
            None => false,
        }
    }

    // ---- §5.8 ----

    /// `Update` (slot 4).
    pub(crate) fn fire_update(&mut self) {
        if self.fire_instance.is_none() {
            return;
        }
        let host = self.host.clone();
        self.process_toast_list();
        self.fire_update_phase();
        let now = self.fire_now();
        let due = self.fs_ref().is_some_and(|f| !f.fires.is_empty() && now >= f.next_ms);
        if due {
            self.update_all_fires();
            if let Some(f) = self.fs() {
                f.next_ms += FIRE_PERIOD_MS;
            }
        }
        let counts = self.fs().and_then(|f| {
            let c = (f.ignited_buildings, f.ignited_flora);
            f.ignited_buildings = 0;
            f.ignited_flora = 0;
            (c != (0, 0)).then_some(c)
        });
        if let Some((b, fl)) = counts {
            host.post(&DisasterPost {
                msg_type: msg::IGNITIONS,
                data1: u64::from(b),
                data2: fl,
                ..DisasterPost::default()
            });
        }
    }

    fn update_all_fires(&mut self) {
        let no_burn_down = self.host.tutorial_no_burn_down();
        let Some(f) = self.fs() else { return };
        f.no_burn_down = no_burn_down;
        if f.fires.iter().any(|e| e.target == 0) {
            self.kill_all_fires();
            return;
        }
        let list: Vec<u64> = f.fires.iter().map(|e| e.occupant).collect();
        for fo in list {
            if self.fs_ref().is_some_and(|f| f.fires.iter().any(|e| e.occupant == fo)) {
                self.update_fire(fo);
            }
        }
    }

    fn kill_all_fires(&mut self) {
        let list: Vec<u64> = self
            .fs_ref()
            .map_or(Vec::new(), |f| f.fires.iter().map(|e| e.occupant).collect());
        for fo in list {
            self.kill_fire(fo);
        }
    }

    // ---- §5.9 ----

    /// `UpdatePhase`.
    pub(crate) fn fire_update_phase(&mut self) {
        let Some(inst) = self.fire_instance else { return };
        let host = self.host.clone();
        let has_fires = self.fs_ref().is_some_and(|f| !f.fires.is_empty());
        let phase = self.fire_phase();
        let post = |m: u32, d1: u64, d2: u32| {
            host.post(&DisasterPost {
                msg_type: m,
                data1: d1,
                data2: d2,
                ..DisasterPost::default()
            })
        };
        if has_fires && phase != 1 {
            self.disaster_has_begun(inst);
            self.register_tick(inst, TICK_PRIORITY);
            let now = self.fire_now();
            if let Some(f) = self.fs() {
                f.next_ms = now;
            }
            self.set_fire_phase(1);
            post(msg::PHASE, u64::from(kind::FIRE), 1);
            post(msg::SEVERITY, 1, 0);
        } else if !has_fires && phase == 1 {
            post(msg::SEVERITY, 0, 0);
            self.set_fire_phase(3);
            post(msg::PHASE, u64::from(kind::FIRE), 3);
            self.unregister_locations(inst);
            self.disaster_has_ended();
            host.record_history(history::FIRE_EPISODES, 1);
            self.unregister_tick(inst);
            if let Some(f) = self.fs() {
                f.fire_ids.clear();
            }
        }
    }

    // ---- §5.10 ----

    /// `UpdateFire` for one fire.
    fn update_fire(&mut self, fo: u64) {
        let host = self.host.clone();
        let tuning = self.ft();
        let now = self.fire_now();
        let Some(e) = self
            .fs_ref()
            .and_then(|f| f.fires.iter().find(|e| e.occupant == fo))
            .copied()
        else {
            return;
        };
        let t = e.target;
        if !host.occupant_in_manager(t) {
            self.kill_fire(fo);
            return;
        }
        if let Some(at) = e.extinguish_at {
            if now >= at {
                self.kill_fire(fo);
            }
            return;
        }
        let mut k = tuning.resistance(host.fire_coverage(t));
        if e.hosed > 0 {
            k = k.wrapping_add(4i32.wrapping_mul(e.hosed as i32));
        }
        let delta = if k <= 0 {
            1
        } else {
            -(self.rng_fire.uniform_u32((k as u32).wrapping_add(1)) as i32)
        };
        let mut crossed20 = e.counter < 20 && e.counter.wrapping_add(delta) >= 20;
        let counter = e.counter.wrapping_add(delta);
        if counter < 0 {
            self.set_counter(fo, 0);
            self.extinguish_fire(fo);
            return;
        }
        self.set_counter(fo, counter);
        let s = tuning.stage_of(counter);
        if s != e.stage {
            if s >= e.max_stage {
                if !self.fs_ref().is_some_and(|f| f.no_burn_down) {
                    self.burn_down_fire(fo);
                }
                return;
            }
            if let Some(x) = self.fs().and_then(|f| f.fires.iter_mut().find(|x| x.occupant == fo)) {
                x.stage = s;
            }
            if counter >= 20 {
                crossed20 = true;
            }
        }
        let ty = host.occupant_type(t);
        if ty == occupant::BUILDING || (ty == occupant::PROP && crossed20) {
            if let Some(lot) = host.occupant_lot(t) {
                if s >= e.max_stage.wrapping_sub(1) {
                    host.set_lot_condition(lot, 6);
                } else if s >= e.max_stage.wrapping_sub(2) {
                    host.set_lot_condition(lot, 5);
                }
            }
        }
        let u = self.rng_fire.uniform_f64();
        let d4 = tuning.d4[(s as usize).min(4)];
        let p = crate::x87::r24(1.0 / f64::from(d4));
        if u < p {
            let r = tuning.d5[(s as usize).min(4)] as f32 + host.wind() * tuning.d6;
            let b = host.occupant_bbox(t);
            let x = self.rng_fire.range_gaussian(f64::from(b.x0 - r), f64::from(b.x1 + r));
            let z = self.rng_fire.range_gaussian(f64::from(b.z0 - r), f64::from(b.z1 + r));
            let (cx, cz) = self.world_to_cell(x as f32, z as f32);
            if self.inside_city(cx, cz) {
                self.fire_ignite_cell(cx, cz, true, false, e.fire_id);
            }
        }
    }

    fn set_counter(&mut self, fo: u64, counter: i32) {
        if let Some(x) = self.fs().and_then(|f| f.fires.iter_mut().find(|x| x.occupant == fo)) {
            x.counter = counter;
        }
    }

    /// `KillFire(fire)`.
    fn kill_fire(&mut self, fo: u64) {
        let Some(inst) = self.fire_instance else { return };
        let host = self.host.clone();
        let Some(e) = self
            .fs_ref()
            .and_then(|f| f.fires.iter().find(|e| e.occupant == fo))
            .copied()
        else {
            return;
        };
        host.post(&DisasterPost {
            msg_type: msg::FIRE_STARTED_ENDED,
            data1: fo,
            data2: 0,
            ..DisasterPost::default()
        });
        self.unregister_location_by_occupant(inst, e.target);
        let other = self
            .fs_ref()
            .and_then(|f| f.fires.iter().find(|x| x.occupant != fo && x.fire_id == e.fire_id))
            .map(|x| x.target);
        match other {
            Some(t) => {
                let zoom = self.ft().zoom;
                self.register_location(inst, 0, 0, t, zoom, false);
            }
            None => {
                if let Some(f) = self.fs() {
                    f.fire_ids.remove(&e.fire_id);
                }
            }
        }
        if let Some(f) = self.fs() {
            f.fires.retain(|x| x.occupant != fo);
        }
        host.remove_fire_occupant(fo, e.target);
    }

    /// `BurnDownFire(fire)`: `KillFire`, then `ToastOccupant(target)`.
    fn burn_down_fire(&mut self, fo: u64) {
        let Some(t) = self
            .fs_ref()
            .and_then(|f| f.fires.iter().find(|e| e.occupant == fo))
            .map(|e| e.target)
        else {
            return;
        };
        self.kill_fire(fo);
        self.toast_occupant(t);
    }

    /// `ToastOccupant(o)`.
    fn toast_occupant(&mut self, o: u64) {
        let host = self.host.clone();
        let t = host.occupant_type(o);
        let flora = t == occupant::FLORA;
        host.play_effect(if flora { "burndown_flora" } else { "burndown_building" }, o);
        if host.occupant_prop_bool(o, prop::BURNING).unwrap_or(false) {
            self.fire_extinguish_occupant(o);
        }
        if flora {
            host.play_effect("fire_flora_scorch", o);
            host.burn_flora(o);
        } else if t == occupant::BUILDING || t == occupant::PROP {
            let demolish = host.occupant_prop_bool(o, prop::BURN_DOWN_DEMOLISHES).unwrap_or(true);
            match host.occupant_lot(o).filter(|_| demolish) {
                Some(lot) => {
                    let extra = host.occupant_prop_u32(o, prop::DEMOLITION_FLAGS).unwrap_or(0);
                    host.demolish_lot(o, lot, BURN_DOWN_FLAGS | extra);
                    let radius = host.occupant_prop_f32(o, prop::RADIATION_RADIUS).unwrap_or(0.0);
                    if radius > 0.0 {
                        let amount = host.occupant_prop_f32(o, prop::RADIATION_AMOUNT).unwrap_or(0.0);
                        host.spew_radiation(o, amount, radius);
                    }
                }
                None => host.remove_occupant(o),
            }
        }
        if let Some(f) = self.fs() {
            f.toasted = f.toasted.wrapping_add(1);
        }
    }

    pub(crate) fn fire_snapshot(&self) -> Option<FireSnapshot> {
        let id = self.fire_instance?;
        let i = self.instances.get(&id)?;
        let f = self.fs_ref()?;
        let now = self.host.now_ms();
        Some(FireSnapshot {
            instance: id,
            phase: i.phase,
            fires: f.fires.clone(),
            fire_ids: f.fire_ids.iter().copied().collect(),
            toast: f.toast.iter().copied().collect(),
            elapsed_ms: f.timer.elapsed_ms(now),
            next_ms: f.next_ms,
            cause: f.cause,
            ignited_buildings: f.ignited_buildings,
            ignited_flora: f.ignited_flora,
            no_burn_down: f.no_burn_down,
            toasted: f.toasted,
            tick_registered: i.tick_registered,
            tuning: self.ft(),
        })
    }
}

/// The flammability property `0x29244DB5` is non-zero (absent = 0).
fn flammable(host: &dyn super::DisasterHost, o: u64) -> bool {
    host.occupant_prop_f32(o, prop::FLAMMABILITY).is_some_and(|v| v != 0.0)
}
