// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The monthly pipeline (§6): `DoIdle` executes one step of the current
//! state and moves to the next.

use crate::curve::ResponseCurve;
use crate::grid::{process_two_grids, GridElem, TractGrid, MAX_TRACTS};

use super::simulator::{AuraSim, Hotspot, MAX_LOTS_PER_CALL};
use super::{clamp_aura, ids, msg, trunc, IteratedLot};

/// `trunc` to `i16`, saturating ([choice]).
fn to_i16(v: f32) -> i16 {
    trunc(f64::from(v)).clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16
}

/// The residential developer types' wealth index (§6.5.1).
fn wealth_of(developer_type: u32) -> Option<usize> {
    ids::DEVELOPER_RESIDENTIAL.iter().position(|&t| t == developer_type)
}

/// Tract counts clamped for loops.
fn counts<T: GridElem>(g: &dyn TractGrid<T>) -> (i32, i32) {
    let geo = g.geometry();
    (geo.count_x.clamp(0, MAX_TRACTS), geo.count_z.clamp(0, MAX_TRACTS))
}

impl AuraSim {
    fn set_state(&self, s: i32) {
        self.d.borrow_mut().state = s;
    }

    /// `DoIdle(ms)`: one pipeline step (§6).
    pub fn do_idle(&self, ms: u32) {
        let state = self.d.borrow().state;
        match state {
            0 => {}
            1 => {
                if self.d.borrow().initialized {
                    let day = self.host.sim_date_number();
                    let mut d = self.d.borrow_mut();
                    d.cycle_start_day = day;
                    d.state = 2;
                }
            }
            2 => {
                self.step_park_map();
                self.set_state(3);
            }
            3 => self.step_landmarks(),
            4 => self.step_prepare(),
            5 => self.step_lots(ms),
            6 => {
                // [choice] A missing park curve still advances (§6.6).
                let curve = self.d.borrow().tuning.park.clone();
                if let (Some(c), Some(src)) = (curve, self.park_map()) {
                    self.combine(&*src, Some(&c), false, f32::from);
                }
                self.set_state(7);
            }
            7 => {
                let curves = self.d.borrow().tuning.pollution.clone();
                for (i, c) in curves.iter().enumerate() {
                    if let Some(c) = c {
                        if let Some(src) = self.host.pollution_grid(i as i32) {
                            self.combine(&*src, Some(c), false, f32::from);
                        }
                    }
                }
                self.set_state(8);
            }
            8 | 9 => {
                let (curve, src) = if state == 8 {
                    (self.d.borrow().tuning.school.clone(), self.host_school())
                } else {
                    (self.d.borrow().tuning.hospital.clone(), self.host_hospital())
                };
                if let (Some(c), Some(src)) = (curve, src) {
                    self.combine(&*src, Some(&c), true, |v| v);
                }
                self.set_state(state + 1);
            }
            10 => {
                let curve = self.d.borrow().tuning.crime.clone();
                if let Some(c) = curve {
                    if let Some(src) = self.host.crime_grid() {
                        self.combine(&*src, Some(&c), false, f32::from);
                    }
                }
                self.set_state(11);
            }
            11 => {
                let curve = self.d.borrow().tuning.trip.clone();
                if let Some(c) = curve {
                    if let Some(src) = self.host.trip_length_map() {
                        // [quirk] A trip length of 0 reads as 255.
                        self.combine(&*src, Some(&c), true, |v| if v == 0 { 255.0 } else { f32::from(v) });
                    }
                }
                self.set_state(12);
            }
            12 => {
                let curve = self.d.borrow().tuning.congestion.clone();
                if let Some(c) = curve {
                    if let Some(src) = self.host.congestion_map() {
                        self.combine(&*src, Some(&c), true, f32::from);
                    }
                }
                self.set_state(13);
            }
            13 => {
                if let Some(src) = self.building_effect_grid() {
                    self.combine(&*src, None, false, f32::from);
                }
                self.set_state(14);
            }
            14 => {
                self.step_base();
                self.set_state(15);
            }
            15 => {
                self.step_rating();
                self.set_state(16);
            }
            16 => {
                self.step_hotspots();
                self.set_state(17);
            }
            17 => {
                self.step_end();
                self.set_state(0);
            }
            // [choice] Unknown states end the cycle.
            _ => self.set_state(0),
        }
    }

    fn host_school(&self) -> Option<std::rc::Rc<dyn TractGrid<f32>>> {
        self.host.school_grade_map()
    }

    fn host_hospital(&self) -> Option<std::rc::Rc<dyn TractGrid<f32>>> {
        self.host.hospital_grade_map()
    }

    /// State 2 (§6.2): park map := raw park × Parks and Recreation funding.
    fn step_park_map(&self) {
        let (Some(park), Some(raw)) = (self.park_map(), self.raw_park_grid()) else {
            return;
        };
        // [choice] A missing department uses 1.0.
        let f = self.host.department_funding(ids::DEPARTMENT_PARKS).unwrap_or(1.0);
        let (nx, nz) = counts(&*park);
        for x in 0..nx {
            for z in 0..nz {
                let v = raw.get(x, z);
                park.set(x, z, if v > 0 { to_i16(f32::from(v) * f) } else { v });
            }
        }
    }

    /// State 3 (§6.3): one department per call; N departments take N + 1
    /// calls.
    fn step_landmarks(&self) {
        let (cursor, next) = {
            let g = self.g.borrow();
            if g.departments.is_empty() {
                drop(g);
                self.set_state(4);
                return;
            }
            let cursor = self.d.borrow().department_cursor;
            let next = match cursor {
                None => g.departments.iter().next(),
                Some(c) => {
                    if !g.departments.contains_key(&c) {
                        // [quirk] The cursor's department disappeared: no publish.
                        drop(g);
                        let mut d = self.d.borrow_mut();
                        d.department_cursor = None;
                        d.state = 4;
                        return;
                    }
                    g.departments
                        .range(c.wrapping_add(1)..)
                        .next()
                        .filter(|_| c != u32::MAX)
                }
            };
            (cursor, next.map(|(&id, grid)| (id, grid.clone())))
        };
        let Some(work) = self.landmark_work_grid() else {
            self.set_state(4);
            return;
        };
        match next {
            None => {
                if let Some(map) = self.landmark_map() {
                    let (nx, nz) = counts(&*map);
                    for x in 0..nx {
                        for z in 0..nz {
                            map.set(x, z, work.get(x, z));
                        }
                    }
                }
                let mut d = self.d.borrow_mut();
                d.department_cursor = None;
                d.state = 4;
            }
            Some((id, grid)) => {
                let f = if id == 0 {
                    1.0
                } else {
                    self.host.department_funding(id).unwrap_or(1.0)
                };
                let first = cursor.is_none();
                let (nx, nz) = counts(&*work);
                for x in 0..nx {
                    for z in 0..nz {
                        let v = grid.get(x, z);
                        let scaled = if v > 0 { f32::from(v) * f } else { f32::from(v) };
                        let w = if first {
                            to_i16(scaled)
                        } else {
                            to_i16(f32::from(work.get(x, z)) + scaled)
                        };
                        work.set(x, z, w);
                    }
                }
                self.d.borrow_mut().department_cursor = Some(id);
            }
        }
    }

    /// State 4 (§6.4): clear the work grid, take a fresh lot iterator, run
    /// the tax-change computation.
    fn step_prepare(&self) {
        if let Some(work) = self.work_grid() {
            work.fill(-128);
        }
        *self.lots.borrow_mut() = Some(self.host.lot_iterator());
        if self.host.lot_count() == 0 {
            // [quirk] The cycle ends silently.
            self.set_state(0);
            return;
        }
        self.tax_change();
        self.set_state(5);
    }

    /// The tax-change effect (§6.5.2).
    pub fn tax_change(&self) {
        let (base, curve) = {
            let d = self.d.borrow();
            (d.tuning.base_aura, d.tuning.tax_change.clone())
        };
        let mut eff = base;
        if let Some(c) = curve {
            for t in 0..12usize {
                let rate = self.host.tax_rate(t as i32);
                let mut d = self.d.borrow_mut();
                if d.previous_tax_rate[t] == -1.0 {
                    d.previous_tax_rate[t] = rate;
                } else {
                    let delta = rate - d.previous_tax_rate[t];
                    if delta != 0.0 {
                        let e = trunc(f64::from(c.eval(delta))) as i32;
                        if t < 3 {
                            eff[t] = base[t].wrapping_add(e);
                        }
                    }
                }
            }
        }
        self.d.borrow_mut().effective_base_aura = eff;
    }

    /// State 5 (§6.5): stamp lots until the iterator ends or the time
    /// budget is used up.
    fn step_lots(&self, ms: u32) {
        let mut slice = self.slice.get();
        let step = slice.step.max(1);
        self.host.limit_timer_start(ms);
        let Some(mut it) = self.lots.borrow_mut().take() else {
            self.set_state(6);
            return;
        };
        let mut processed = 0u32;
        let mut done = false;
        'outer: while processed < MAX_LOTS_PER_CALL {
            for _ in 0..step {
                match it.next_lot() {
                    None => {
                        done = true;
                        break 'outer;
                    }
                    Some(lot) => {
                        self.stamp_lot(&lot);
                        processed += 1;
                    }
                }
            }
            if self.host.limit_timer_expired() {
                break;
            }
        }
        slice.adapt(processed);
        self.slice.set(slice);
        if done {
            self.set_state(6);
        } else {
            *self.lots.borrow_mut() = Some(it);
        }
    }

    fn stamp_lot(&self, lot: &IteratedLot) {
        let b = self.lot_base_aura(lot);
        let Some(work) = self.work_grid() else { return };
        let (nx, nz) = counts(&*work);
        let r = &lot.info;
        for x in r.x1.max(0)..=r.x2.min(nx - 1) {
            for z in r.z1.max(0)..=r.z2.min(nz - 1) {
                if b > work.get(x, z) {
                    work.set(x, z, b);
                }
            }
        }
    }

    /// `LotBaseAura(lot)` (§6.5.1).
    pub fn lot_base_aura(&self, lot: &IteratedLot) -> i8 {
        if lot.building.is_none() {
            return -128;
        }
        let (eff, curve) = {
            let d = self.d.borrow();
            (d.effective_base_aura, d.tuning.value_wealth.clone())
        };
        let (mut sum, mut total, mut seen) = (0i32, 0i32, false);
        for &(t, count) in &lot.populations {
            if let Some(w) = wealth_of(t) {
                sum = sum.wrapping_add(eff[w].wrapping_mul(count));
                seen = true;
            }
            total = total.wrapping_add(count);
        }
        if !seen || total <= 0 {
            return -128;
        }
        let avg = clamp_aura(i64::from(sum / total));
        match curve {
            Some(c) => {
                let r = &lot.info;
                let cx = ((i64::from(r.x1) + i64::from(r.x2)) / 2) as i32;
                let cz = ((i64::from(r.z1) + i64::from(r.z2)) / 2) as i32;
                let d = self.host.land_value_type(cx, cz).wrapping_sub(lot.occupant_wealth);
                clamp_aura(i64::from(avg) + trunc(f64::from(c.eval(d as f32))))
            }
            None => avg,
        }
    }

    /// Rules A and R (§6.13): only work cells ≠ −128 change; with
    /// `residential`, only cells whose lot has zone type 1..3.
    fn combine<B: GridElem>(
        &self,
        src: &dyn TractGrid<B>,
        curve: Option<&ResponseCurve>,
        residential: bool,
        value: impl Fn(B) -> f32,
    ) {
        let Some(work) = self.work_grid() else { return };
        let host = &self.host;
        process_two_grids(&*work, src, |x, z, w: &mut i8, s| {
            if *w == -128 {
                return;
            }
            if residential && !host.lot_at(x, z).is_some_and(|l| (1..=3).contains(&l.zone_type)) {
                return;
            }
            let v = value(s);
            let add = match curve {
                Some(c) => c.eval(v),
                None => v,
            };
            *w = clamp_aura(trunc(f64::from(f32::from(*w) + add)));
        });
    }

    /// State 14 (§6.14): the long-term smoothing of the base grid.
    fn step_base(&self) {
        let global = self.host.cumulative_offset(ids::ORDINANCE_AURA_EFFECT, 0);
        let long_decay = {
            let mut d = self.d.borrow_mut();
            d.global_effect = global;
            d.tuning.long_decay
        };
        let m = (1.0f64 - f64::from(long_decay)) as f32;
        let (Some(base), Some(work)) = (self.base_grid(), self.work_grid()) else {
            return;
        };
        let geo = base.geometry();
        let shift = geo.safe_shift().min(12);
        let side = 1i64 << shift;
        let (nx, nz) = counts(&*base);
        for tx in 0..nx {
            for tz in 0..nz {
                let (mut sum, mut count) = (0i64, 0i64);
                for cx in (i64::from(tx) << shift)..(i64::from(tx) << shift) + side {
                    for cz in (i64::from(tz) << shift)..(i64::from(tz) << shift) + side {
                        let v = work.get_cell_value(cx as i32, cz as i32);
                        if v != -128 {
                            sum += i64::from(v);
                            count += 1;
                        }
                    }
                }
                if count == 0 {
                    base.set(tx, tz, -128);
                    continue;
                }
                let new = sum / count + i64::from(global);
                let old = base.get(tx, tz);
                let v = if old == -128 {
                    clamp_aura(new)
                } else {
                    let d = new - i64::from(old);
                    // One 24-bit rounding of the exact product (foundations §18).
                    let p = (d as f64 * f64::from(m)) as f32;
                    let step = if d > 0 {
                        trunc(f64::from(p.ceil()))
                    } else if d < 0 {
                        trunc(f64::from(p.floor()))
                    } else {
                        0
                    };
                    clamp_aura(i64::from(old).saturating_add(step))
                };
                base.set(tx, tz, v);
            }
        }
    }

    /// State 15 (§6.15): the aura grid, best/worst tracts, low tracts and the
    /// mayor rating.
    fn step_rating(&self) {
        let pops = self.host.population_grids();
        let (Some(aura), Some(base), Some(transient)) = (self.aura_grid(), self.base_grid(), self.transient_grid())
        else {
            return;
        };
        let geo = aura.geometry();
        let s = geo.size;
        let (nx, nz) = counts(&*aura);
        let threshold = self.d.borrow().tuning.hotspot[0];
        let mut low = Vec::new();
        let (mut max_r, mut min_r) = (-128i16, 127i16);
        let (mut max_l, mut min_l) = {
            let d = self.d.borrow();
            (d.max_location, d.min_location)
        };
        let (mut total, mut weighted) = (0i32, 0i32);
        for x in 0..nx {
            for z in 0..nz {
                let b = base.get(x, z);
                if b == -128 {
                    aura.set(x, z, 0);
                    continue;
                }
                let a = clamp_aura(i64::from(b) + i64::from(transient.get(x, z)));
                aura.set(x, z, a);
                let loc = || {
                    (
                        x.wrapping_mul(s).wrapping_add(s / 2),
                        z.wrapping_mul(s).wrapping_add(s / 2),
                    )
                };
                if i16::from(a) < min_r {
                    min_r = i16::from(a);
                    min_l = loc();
                }
                if i16::from(a) > max_r {
                    max_r = i16::from(a);
                    max_l = loc();
                }
                if i32::from(a) < threshold {
                    low.push((x, z, 1));
                }
                let pop = pops.as_ref().map_or(0i32, |p| {
                    p.iter().fold(0i32, |acc, g| acc.wrapping_add(i32::from(g.get(x, z))))
                });
                total = total.wrapping_add(pop);
                weighted = weighted.wrapping_add(i32::from(a).wrapping_mul(pop));
            }
        }
        let rating = if total > 0 { weighted / total } else { 0 };
        {
            let mut d = self.d.borrow_mut();
            d.low_tracts = low;
            d.max_rating = max_r;
            d.max_location = max_l;
            d.min_rating = min_r;
            d.min_location = min_l;
            d.mayor_rating = rating;
        }
        self.host.record_history(ids::HISTORY_MAYOR_RATING, rating);
    }

    /// State 16 (§6.16): hotspots and the riot roll.
    fn step_hotspots(&self) {
        let (low, [_, radius, min_weight], riot) = {
            let d = self.d.borrow();
            (d.low_tracts.clone(), d.tuning.hotspot, d.tuning.riot.clone())
        };
        let mut spots: Vec<Hotspot> = Vec::new();
        for &(x, z, _) in &low {
            let near = spots.iter_mut().find(|h| {
                (i64::from(h.x) - i64::from(x)).abs() + (i64::from(h.z) - i64::from(z)).abs() < i64::from(radius)
            });
            match near {
                Some(h) => {
                    let w = i64::from(h.weight);
                    let div = w + 1;
                    if div != 0 {
                        h.x = ((i64::from(h.x) * w + i64::from(x)) / div) as i32;
                        h.z = ((i64::from(h.z) * w + i64::from(z)) / div) as i32;
                    }
                    h.weight = h.weight.wrapping_add(1);
                }
                None => spots.push(Hotspot { x, z, weight: 1 }),
            }
        }
        spots.retain(|h| h.weight >= min_weight);
        let roll = self.rng.borrow_mut().range_f64(0.0, 100.0) as f32;
        let n = spots.len();
        {
            let mut d = self.d.borrow_mut();
            d.hotspots = spots.clone();
            d.riot_roll = roll;
        }
        let Some(curve) = riot else { return };
        if n == 0 {
            return;
        }
        let k = self.rng.borrow_mut().uniform_u32(n as u32) as usize;
        let Some(h) = spots.get(k) else { return };
        let p = curve.eval(h.weight as f32);
        if p > roll {
            let s = self
                .aura_grid()
                .map_or(super::grid_id::TRACT_SIZE, |g| g.geometry().size);
            let at = |t: i32| t.wrapping_mul(s).wrapping_add(s / 2);
            self.host.attempt_disaster_start(ids::DISASTER_RIOT, at(h.x), at(h.z));
        }
    }

    /// State 17 (§6.17): transient decay and the end-of-cycle message.
    fn step_end(&self) {
        let short = self.d.borrow().tuning.short_decay;
        if let Some(t) = self.transient_grid() {
            let (nx, nz) = counts(&*t);
            for x in 0..nx {
                for z in 0..nz {
                    let v = t.get(x, z);
                    if v != 0 {
                        t.set(x, z, trunc(f64::from(f32::from(v) * short)).clamp(-128, 127) as i8);
                    }
                }
            }
        }
        self.d.borrow_mut().unused_rating = 0;
        self.host.post(msg::AURA_CYCLE_COMPLETED);
    }
}
