// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The daily simulation (§4.4), the garbage flow (§4.6), the end of cycle
//! (§4.8) and the monthly simulation (§4.9).

use crate::variant::{Elem, Variant};
use crate::x87::rhe;
use crate::x87::{ftol, ftol32};

use super::simulator::{
    fadd_i, fdiv_ii, fmul_i, p_f32, p_u32, u32_prop, PollutionState, Remaining, G_AIR_SRC, G_AIR_VAL, G_GARBAGE_SRC,
    G_RAD_SRC, G_WATER_SRC, G_WATER_VAL, L_INC, L_REC, L_WTE, L_WTP,
};
use super::tuning::Tuning;
use super::{budget, deal, factor, history, msg, prop, PollutionSim, ZONE_LANDFILL};

/// The day's ordinance factors (§4.4.3).
#[derive(Clone, Copy, Debug)]
struct Factors {
    traffic: f32,
    air: f32,
    water: f32,
    garbage: f32,
    /// Air, water, garbage multipliers per zone type 0..15.
    zones: [[f32; 3]; 16],
}

impl Factors {
    fn zone(&self, zone_type: u32) -> [f32; 3] {
        // [choice] Zone types ≥ 16 use 1.0.
        self.zones.get(zone_type as usize).copied().unwrap_or([1.0; 3])
    }
}

/// `trunc` of a float through the CRT `_ftol` (foundations §18.1): an
/// `int64` truncation whose low 32 bits are kept (NaN, ±∞ and |x| ≥ 2^63
/// give 0).
fn trunc32(v: f32) -> i32 {
    ftol32(f64::from(v))
}

/// `trunc(counter + x)` for a 64-bit counter: the integer operand enters
/// unrounded and the sum is rounded to `f32` once (foundations §18.3), then
/// `_ftol` (§18.1).
fn add_i64(counter: i64, x: f32) -> i64 {
    ftol(f64::from(fadd_i(x, counter as f64)))
}

/// The same for a 32-bit counter (the low 32 bits of `_ftol`).
fn add_u32(counter: u32, x: f32) -> u32 {
    trunc32(fadd_i(x, f64::from(counter))) as u32
}

/// A deal's daily share `rhe(f32(amount × 0.1f))` (§4.4.2): the int32
/// amount enters exactly, the product is rounded to `f32` once, and the
/// 1.5·2²³ idiom rounds it (crime.md §6 `rhe`, exact up to 2²²).
fn deal_share(amount: i32) -> i32 {
    rhe(fmul_i(f64::from(amount), 0.1))
}

/// The `Dispose` floor (§4.12): `n := rhe(f32(g − 0.5f))`, then `n + 1` when
/// `float(n + 1) = g` (an exact compare). `floor(g)` up to 2²² and a little
/// beyond; larger offers follow the `rhe` formula (5,000,000 → 4,597,152).
fn floor_rhe(g: f32) -> i32 {
    let n = rhe(g - 0.5);
    let up = n.wrapping_add(1);
    if f64::from(up) == f64::from(g) {
        up
    } else {
        n
    }
}

impl PollutionSim {
    // ---- the day (§4.4) ----------------------------------------------------------------------------------

    /// SimNewDay (level 4).
    pub fn simulate_day(&self) {
        let t = self.tuning.borrow().clone();
        let h = self.host.clone();
        self.edit(|s| {
            // 4.4.1 Disposal capacities.
            let (wtp_unused, _) = self.list_unused(s, L_WTP, prop::WATER_TREATMENT, prop::WTP_USED);
            let (inc_unused, inc_total) = self.list_unused(s, L_INC, prop::INCINERATOR, prop::INC_USED);
            s.incinerator_capacity = inc_total as i32;
            let (wte_unused, wte_total) = self.list_unused(s, L_WTE, prop::WASTE_TO_ENERGY, prop::WTE_USED);
            s.wte_capacity = wte_total as i32;
            let mut rem = Remaining {
                wtp: wtp_unused,
                inc: inc_unused,
                wte: wte_unused,
            };
            // 4.4.2 Neighbour deals.
            let mut import_total = 0i64;
            for d in h.find_deals(deal::IMPORT) {
                if d.amount.wrapping_sub(d.amount_used) > 0 {
                    let n = deal_share(d.amount);
                    import_total = import_total.wrapping_add(i64::from(n));
                    h.set_deal_amount_used(d.token, d.amount_used.wrapping_add(n));
                    s.month.imported = s.month.imported.wrapping_add(i64::from(n));
                }
            }
            for d in h.find_deals(deal::EXPORT) {
                let room = d.amount.wrapping_sub(d.amount_used);
                if room > 0 {
                    let n = deal_share(d.amount).min(room);
                    h.set_deal_amount_used(d.token, d.amount_used.wrapping_add(n));
                    s.export_capacity = s.export_capacity.wrapping_add(n);
                }
            }
            let offered = import_total as f32;
            let eaten = self.dispose_in(s, &t, offered, &mut rem);
            s.add_to_garbage_supply(trunc32(offered - eaten));
            // 4.4.3 Factors.
            let mut f = Factors {
                traffic: h.cumulative_factor(factor::TRAFFIC, 0),
                air: h.cumulative_factor(factor::AIR, 0),
                water: h.cumulative_factor(factor::WATER, 0),
                garbage: h.cumulative_factor(factor::GARBAGE, 0),
                zones: [[1.0; 3]; 16],
            };
            for (z, m) in f.zones.iter_mut().enumerate() {
                m[0] = h.cumulative_factor(factor::ZONE_AIR, z as u32);
                m[1] = h.cumulative_factor(factor::ZONE_WATER, z as u32);
                m[2] = h.cumulative_factor(factor::ZONE_GARBAGE, z as u32);
            }
            // 4.4.4 The visits ([choice] at most one cycle's worth).
            let (tx, tz) = s.tract_counts(G_AIR_SRC);
            let visits = u64::from(s.tracts_per_day).min(tx as u64 * tz as u64);
            for _ in 0..visits {
                self.visit_tract(s, &t, &f, &mut rem);
            }
            // 4.4.5 After the visits: the valve.
            let room = i64::from(s.garbage_capacity) - i64::from(s.garbage_supply);
            if room < 0 {
                let excess = -room;
                let mut e = i64::from(trunc32(self.dispose_in(s, &t, excess as f32, &mut rem)));
                let left = excess.wrapping_sub(e);
                if left as u32 > t.max_garbage_on_valve {
                    e += left - i64::from(t.max_garbage_on_valve);
                }
                s.add_to_garbage_supply(e.wrapping_neg() as i32);
            }
            // Usage bookkeeping.
            let lists = [
                (L_WTP, prop::WATER_TREATMENT, prop::WTP_USED, wtp_unused, rem.wtp),
                (L_INC, prop::INCINERATOR, prop::INC_USED, inc_unused, rem.inc),
                (L_WTE, prop::WASTE_TO_ENERGY, prop::WTE_USED, wte_unused, rem.wte),
            ];
            for (list, cap, used_prop, unused, remaining) in lists {
                self.distribute_used(s, list, cap, used_prop, unused, unused.wrapping_sub(remaining));
            }
            // Maintenance costs (not waste-to-energy).
            for list in [L_WTP, L_INC, L_REC] {
                for occ in s.lists[list].clone() {
                    self.update_maintenance_cost(occ);
                }
            }
        });
    }

    /// Σ max(0, current − used) and Σ current over a list (§4.4.1); an
    /// occupant whose query or property fails adds 0.
    fn list_unused(&self, s: &PollutionState, list: usize, cap: u32, used_prop: u32) -> (u32, u32) {
        let (mut unused, mut total) = (0u32, 0u32);
        for &occ in &s.lists[list] {
            let (ok, cur) = self.current_capacity_in(s, cap, occ);
            if !ok {
                continue;
            }
            total = total.wrapping_add(cur);
            if let Some(used) = p_u32(&*self.host, occ, used_prop) {
                unused = unused.wrapping_add(cur.saturating_sub(used));
            }
        }
        (unused, total)
    }

    /// One tract visit (§4.4.4).
    fn visit_tract(&self, s: &mut PollutionState, t: &Tuning, f: &Factors, rem: &mut Remaining) {
        let h = &*self.host;
        let (x, z) = (s.cell_x as i32, s.cell_z as i32);
        // 1. Sources.
        let a = s.cell(G_AIR_SRC, x, z).max(0);
        let w = s.cell(G_WATER_SRC, x, z).max(0);
        let g = s.cell(G_GARBAGE_SRC, x, z).max(0);
        let r = s.cell(G_RAD_SRC, x, z).max(0);
        // 2. Ordinances.
        let mut a = trunc32(f32::from(a) * f.air) as i16;
        let mut w = trunc32(f32::from(w) * f.water) as i16;
        let mut g = trunc32(f32::from(g) * f.garbage) as i16;
        // 3. Zoning.
        let cells: [(i32, i32); 4] = [(x, z), (x + 1, z), (x, z + 1), (x + 1, z + 1)];
        let mut zones = [0u32; 4];
        let mut sums = [0f32; 3];
        let mut count = 0i32;
        let mut developed = false;
        for (k, &(cx, cz)) in cells.iter().enumerate() {
            let zt = h.zone_type(cx, cz);
            zones[k] = zt;
            if zt != ZONE_LANDFILL {
                let m = f.zone(zt);
                for i in 0..3 {
                    sums[i] += m[i];
                }
                count += 1;
                if zt != 0 {
                    developed = true;
                }
            }
        }
        if developed {
            s.developed_tracts = s.developed_tracts.wrapping_add(1);
        }
        if count > 0 {
            let n = count as f32;
            a = trunc32(f32::from(a) * sums[0] / n) as i16;
            w = trunc32(f32::from(w) * sums[1] / n) as i16;
            g = trunc32(f32::from(g) * sums[2] / n) as i16;
        }
        // 4. Per cell.
        let mut big_a = i32::from(a);
        for (k, &(cx, cz)) in cells.iter().enumerate() {
            let cur = s.garbage_value(cx, cz);
            let traffic = f32::from(h.air_traffic(cx, cz));
            big_a = trunc32(fadd_i(traffic * f.traffic * t.traffic_air_factor, f64::from(big_a)));
            if zones[k] == ZONE_LANDFILL {
                continue;
            }
            let mut gp = f32::from(g) / count as f32;
            s.month.produced = s.month.produced.wrapping_add(i64::from(rhe(gp)));
            if s.recycling_effect > 0.0 && gp > 0.0 {
                let rec = (gp * s.recycling_effect).min(gp);
                s.month.recycled = (s.month.recycled + f64::from(rec)) as f32 as f64;
                gp -= rec;
            }
            let total = gp + cur as f32;
            s.set_garbage_value(cx, cz, 0);
            if s.total_uncollected >= cur as u32 {
                s.total_uncollected = s.total_uncollected.wrapping_sub(cur as u32);
            }
            let left = total - self.dispose_in(s, t, total, rem);
            if left > 0.0 {
                if left > 32767.0 {
                    s.set_garbage_value(cx, cz, 32767);
                    s.total_uncollected = s.total_uncollected.wrapping_add(32767);
                    if left - 32767.0 > 0.0 {
                        s.add_to_garbage_supply(trunc32(left - 32767.0));
                    }
                } else if i64::from(s.garbage_capacity) - i64::from(s.garbage_supply) >= 0 || left >= 32767.0 {
                    s.set_garbage_value(cx, cz, trunc32(left) as i16);
                    s.total_uncollected = add_u32(s.total_uncollected, left);
                } else {
                    // The valve is closed: garbage flows back from the supply.
                    let over = i64::from(s.garbage_supply) - i64::from(s.garbage_capacity);
                    let share = i64::from(rhe(f32::from(g) / count as f32));
                    let mut take = if over <= share {
                        over
                    } else if g == 0 {
                        1
                    } else {
                        share
                    };
                    if fadd_i(left, take as f64) > 32767.0 {
                        take = i64::from(trunc32(32767.0 - left));
                    }
                    let take = take.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
                    s.add_to_garbage_supply(take.wrapping_neg());
                    s.set_garbage_value(cx, cz, trunc32(fadd_i(left, f64::from(take))) as i16);
                    s.total_uncollected = add_u32(s.total_uncollected, fadd_i(left, f64::from(take)));
                }
            }
        }
        // 5.–6. Water and its treatment.
        let mut big_w = i32::from(w);
        s.water_sum = s.water_sum.wrapping_add(big_w);
        if rem.wtp > 0 && big_w > 0 {
            let mut tr = if s.last_water_sum > 0 && s.last_treated_sum > 0 && rem.wtp <= s.last_treated_sum as u32 {
                let q = fdiv_ii(f64::from(s.last_treated_sum), f64::from(s.last_water_sum));
                let v = rhe(fmul_i(f64::from(big_w), q));
                if v == 0 {
                    1
                } else {
                    v
                }
            } else {
                rem.wtp as i32
            };
            tr = tr.min(big_w);
            let tr = (tr as u32).min(rem.wtp);
            s.treated_sum = s.treated_sum.wrapping_add(tr as i32);
            big_w = big_w.wrapping_sub(tr as i32);
            rem.wtp -= tr;
        }
        // 7. Sums.
        s.water_sum_treated = s.water_sum_treated.wrapping_add(big_w);
        s.air_sum = s.air_sum.wrapping_add(big_a);
        s.radiation_sum = s.radiation_sum.wrapping_add(r as u32);
        // 8.–10. Relaxation.
        let big_a = i64::from(big_a.clamp(0, 32767));
        let big_w = i64::from(big_w.clamp(0, 32767));
        let v = i64::from(s.cell(G_AIR_VAL, x, z));
        let nv = relax(v, big_a, i64::from(t.max_air_delta));
        if nv != v {
            s.set_air_value(x, z, nv as i16);
        }
        let v = i64::from(s.cell(G_WATER_VAL, x, z));
        let mut nv = relax(v, big_w, i64::from(t.max_water_delta));
        if v > big_w && nv == 0 {
            let (nx, nz) = s.tract_counts(G_WATER_VAL);
            let hot = |cx: i32, cz: i32| s.water_value(cx, cz) > 1;
            if (x > 0 && hot(x - 1, z))
                || (z > 0 && hot(x, z - 1))
                || (x < nx - 1 && hot(x + 1, z))
                || (z < nz - 1 && hot(x, z + 1))
            {
                nv = 1;
            }
        }
        if nv != v {
            s.set_water_value(x, z, nv as i16);
        }
        // 11. Advance.
        if s.iterator.is_exhausted() {
            self.with_crt(|crt| s.iterator.reset(true, crt));
            self.end_of_cycle(s);
        }
        if let Some((tx, tz)) = s.iterator.next_cell() {
            s.cell_x = tx.wrapping_mul(2) as u32;
            s.cell_z = tz.wrapping_mul(2) as u32;
        }
    }

    // ---- the garbage flow (§4.6) -------------------------------------------------------------------------

    /// `Dispose(g)` with the day's remaining capacities (§4.6.1).
    pub fn dispose(&self, g: f32, rem: &mut Remaining) -> f32 {
        let t = self.tuning.borrow().clone();
        self.edit(|s| self.dispose_in(s, &t, g, rem))
    }

    pub(super) fn dispose_in(&self, s: &mut PollutionState, t: &Tuning, g: f32, rem: &mut Remaining) -> f32 {
        if g.is_nan() || g <= 0.0 {
            return 0.0;
        }
        let h = &*self.host;
        let mut g = g;
        let mut taken = 0f32;
        // Export.
        let room = s.export_capacity.wrapping_sub(s.month.exported as i32);
        if room > 0 {
            let x = if f64::from(g) < f64::from(room) { g } else { room as f32 };
            s.month.exported = add_i64(s.month.exported, x);
            g -= x;
            taken += x;
        }
        // Waste-to-energy, then incinerators.
        for (remaining, counter) in [
            (&mut rem.wte, &mut s.month.wte),
            (&mut rem.inc, &mut s.month.incinerated),
        ] {
            if *remaining != 0 {
                let x = if f64::from(*remaining) > f64::from(g) {
                    *remaining = remaining.wrapping_sub(floor_rhe(g) as u32);
                    let x = g;
                    g = 0.0;
                    x
                } else {
                    *remaining = 0;
                    0.0
                };
                *counter = add_i64(*counter, x);
                taken += x;
            }
        }
        // Landfill.
        let (usage, capacity) = h.landfill_usage();
        let room = i64::from(capacity) - i64::from(usage);
        let transport = t.transport_curve.eval(h.landfill_tile_count() as f32);
        // `rhe` leaves its exact range above 2²² (839 shipped tiles, §4.12); the
        // supply is subtracted in 32 bits.
        let limit = i64::from(rhe(transport * s.efficiency).wrapping_sub(s.garbage_supply));
        let n = limit.min(room);
        if n > 0 {
            if f64::from(g) > n as f64 {
                s.add_to_garbage_supply(n.min(i64::from(i32::MAX)) as i32);
                g = fadd_i(g, -(n as f64));
                taken = fadd_i(taken, n as f64);
            } else {
                s.add_to_garbage_supply(rhe(g));
                taken += g;
                g = 0.0;
            }
        }
        // Decomposition.
        let room = i64::from(t.decomposition_amount) - i64::from(s.decomposed);
        if room > 0 {
            let x = if f64::from(g) < room as f64 { g } else { room as f32 };
            s.decomposed = add_u32(s.decomposed, x);
            taken += x;
        }
        taken
    }

    /// `CalculateRecyclingCenterEffect` (§4.6.3).
    fn update_recycling_effect(&self, s: &mut PollutionState) {
        let h = &*self.host;
        let pop = h.population();
        if s.recycling_count == 0 {
            s.recycling_effect = 0.0;
            return;
        }
        let mut sorted: Vec<(u64, f32)> = Vec::with_capacity(s.lists[L_REC].len());
        for occ in s.lists[L_REC].clone() {
            match self.recycling_capacity_in(s, occ) {
                (true, c) => {
                    let pos = sorted.iter().position(|e| e.1 <= c).unwrap_or(sorted.len());
                    sorted.insert(pos, (occ, c));
                }
                _ => h.release(occ),
            }
        }
        s.lists[L_REC] = sorted.iter().map(|e| e.0).collect();
        s.recycling_count = sorted.len() as u32;
        let (mut served, mut weighted) = (0f32, 0f32);
        for &(occ, cap) in &sorted {
            if served.is_nan() || f64::from(served) >= f64::from(pop) {
                break;
            }
            if let Some(pop) = p_f32(h, occ, prop::POPULATION_SERVED) {
                served += pop;
                weighted += cap * pop;
            }
        }
        s.recycling_effect = if served > 0.0 {
            // The population enters the division unrounded (§18.3).
            if f64::from(served) >= f64::from(pop) {
                weighted / served
            } else {
                fdiv_ii(f64::from(weighted), f64::from(pop))
            }
        } else {
            0.0
        };
    }

    // ---- the end of cycle (§4.8) -------------------------------------------------------------------------

    pub(super) fn end_of_cycle(&self, s: &mut PollutionState) {
        let h = &*self.host;
        self.update_recycling_effect(s);
        s.garbage_uncollected = s.total_uncollected != 0;
        let dev = s.developed_tracts;
        // Unsigned divisions; no developed tract gives 0 everywhere.
        let div = |v: u32| v.checked_div(dev).unwrap_or(0);
        s.avg_air = div(s.air_sum as u32) as i32;
        s.avg_water = div(s.water_sum_treated as u32) as i32;
        s.avg_garbage = div(s.total_uncollected) as u16;
        let rad_pct = div(s.radiation_sum.wrapping_mul(100));
        s.last_water_sum = s.water_sum;
        s.last_treated_sum = s.treated_sum;
        h.record_history(history::AIR, s.avg_air);
        h.record_history(history::WATER, s.avg_water);
        h.record_history(history::GARBAGE, i32::from(s.avg_garbage));
        h.record_history(history::RADIATION, s.radiation_sum as i32);
        h.record_history(history::AIR_PLUS_RADIATION, s.avg_air.wrapping_add(rad_pct as i32));
        s.radiation_sum = 0;
        s.developed_tracts = 0;
        s.treated_sum = 0;
        s.water_sum = 0;
        s.water_sum_treated = 0;
        s.air_sum = 0;
        for (dirty, m) in [
            (s.dirty_radiation, msg::RADIATION_CHANGED),
            (s.dirty_air, msg::AIR_CHANGED),
            (s.dirty_water, msg::WATER_CHANGED),
            (s.dirty_garbage, msg::GARBAGE_CHANGED),
        ] {
            if dirty {
                h.post(m, 0);
            }
        }
        s.dirty_radiation = false;
        s.dirty_air = false;
        s.dirty_water = false;
        s.dirty_garbage = false;
        self.update_efficiency(s);
        h.post(msg::DEAL_STATUS, 8);
    }

    /// Runs the end of cycle now (§4.8), for hosts and tests.
    pub fn run_end_of_cycle(&self) {
        self.edit(|s| self.end_of_cycle(s));
    }

    // ---- the month (§4.9) --------------------------------------------------------------------------------

    /// SimNewMonth (level 6).
    pub fn simulate_month(&self) {
        let t = self.tuning.borrow().clone();
        let h = self.host.clone();
        self.edit(|s| {
            // 4.9.1 Deals and cost.
            for d in h.find_deals(deal::IMPORT) {
                h.set_deal_amount_used(d.token, 0);
            }
            let mut exports = 0i64;
            for d in h.find_deals(deal::EXPORT) {
                h.set_deal_amount_used(d.token, 0);
                exports = exports.wrapping_add(i64::from(d.amount));
            }
            let m = s.month;
            let units = m
                .incinerated
                .wrapping_add(m.wte)
                .wrapping_add(m.landfilled)
                .wrapping_add(exports);
            let cost = ftol(f64::from(fmul_i(units as f64, t.cost_per_unit)));
            if let Some(dept) = s.department {
                h.set_line_item_full_expenses(dept, budget::LINE_DISPOSAL, cost);
            }
            s.last_month_cost = cost;
            // 4.9.2 Counters and history.
            s.last_month = m;
            s.decomposed_last_month = s.decomposed;
            for (series, v) in [
                (history::PRODUCED, m.produced),
                (history::IMPORTED, m.imported),
                (history::EXPORTED, m.exported),
                (history::RECYCLED, ftol(m.recycled)),
                (history::INCINERATED, m.incinerated),
                (history::WTE, m.wte),
                (history::LANDFILLED, m.landfilled),
            ] {
                h.record_history(series, v as i32);
            }
            s.month = Default::default();
            s.decomposed = 0;
            s.export_capacity = 0;
            // 4.9.3 Aging and usage.
            let funding = self.funding(s);
            self.age_list(s, &t, L_WTP, Some((prop::WTP_USED, prop::WATER_TREATMENT)), funding);
            self.age_list(s, &t, L_INC, Some((prop::INC_USED, prop::INCINERATOR)), funding);
            self.age_list(s, &t, L_REC, None, funding);
            self.roll_used(s, L_WTP, prop::WTP_USED, prop::WTP_USED_LAST);
            self.roll_used(s, L_INC, prop::INC_USED, prop::INC_USED_LAST);
            for &occ in &s.lists[L_WTE] {
                let max = p_u32(&*h, occ, prop::WTE_MAX_OUTPUT);
                let cap = p_u32(&*h, occ, prop::WASTE_TO_ENERGY);
                let used = p_u32(&*h, occ, prop::WTE_USED);
                if let (Some(max), Some(cap), Some(used)) = (max, cap, used) {
                    // [choice] A zero capacity writes nothing.
                    if let Some(out) = used.wrapping_mul(max).checked_div(cap) {
                        h.set_occupant_property(occ, prop::WTE_OUTPUT, u32_prop(out));
                    }
                }
            }
            self.roll_used(s, L_WTE, prop::WTE_USED, prop::WTE_USED_LAST);
        });
        // 4.9.5 End of month.
        self.update_visual_effects();
        h.mark_deals_dirty();
    }

    /// `AgeOccupantList` (§4.9.3).
    fn age_list(&self, s: &PollutionState, t: &Tuning, list: usize, props: Option<(u32, u32)>, funding: f32) {
        let h = &*self.host;
        for &occ in &s.lists[list] {
            let mut ratio = 0.0f32;
            match props {
                Some((used_prop, cap_prop)) => {
                    if let Some(used) = p_u32(h, occ, used_prop) {
                        let (_, cur) = self.current_capacity_in(s, cap_prop, occ);
                        if used > 0 && cur > 0 {
                            ratio = fdiv_ii(f64::from(used), f64::from(cur));
                        }
                    }
                }
                None => {
                    if s.recycling_count > 0 {
                        ratio = (s.month.recycled / f64::from(s.recycling_count)) as f32;
                    }
                }
            }
            let mut condition = p_f32(h, occ, prop::CONDITION).unwrap_or(0.0);
            let k = p_f32(h, occ, prop::AGING_RATE).unwrap_or(0.0);
            let t1 = p_f32(h, occ, prop::AGING_T1).unwrap_or(99.0);
            let t2 = p_f32(h, occ, prop::AGING_T2).unwrap_or(99.0);
            condition += t.usage_decay_curve.eval(ratio) * t.funding_decay_curve.eval(funding) * k;
            h.set_occupant_property(occ, prop::CONDITION, Variant::Scalar(Elem::F32(condition)));
            let lot_condition = if condition >= t2 {
                4
            } else if condition >= t1 {
                2
            } else {
                continue;
            };
            let pos = h.occupant_position(occ);
            let (cx, cz) = h.position_to_cell(pos[0], pos[2]);
            h.set_lot_condition(cx, cz, lot_condition);
        }
    }

    /// `lastMonthUsed := used; used := 0` for a list.
    fn roll_used(&self, s: &PollutionState, list: usize, used_prop: u32, last_prop: u32) {
        let h = &*self.host;
        for &occ in &s.lists[list] {
            let used = p_u32(h, occ, used_prop).unwrap_or(0);
            h.set_occupant_property(occ, last_prop, u32_prop(used));
            h.set_occupant_property(occ, used_prop, u32_prop(0));
        }
    }

    /// `SetOccupantListUsedCapacity` (§4.9.3) on the water-treatment
    /// ([`L_WTP`]), incinerator ([`L_INC`]) or waste-to-energy ([`L_WTE`])
    /// list with its capacity and used properties; other lists do nothing.
    pub fn set_occupant_list_used_capacity(&self, list: usize, total: u32, used: u32) {
        let (cap, used_prop) = match list {
            L_WTP => (prop::WATER_TREATMENT, prop::WTP_USED),
            L_INC => (prop::INCINERATOR, prop::INC_USED),
            L_WTE => (prop::WASTE_TO_ENERGY, prop::WTE_USED),
            _ => return,
        };
        let s = self.snapshot();
        self.distribute_used(&s, list, cap, used_prop, total, used);
    }

    /// `SetOccupantListUsedCapacity` (§4.9.3) *[compat]*: `total` is the
    /// day's unused total of the list and `used = total − remaining`. All
    /// arithmetic is unsigned 32-bit; remainders are dropped, a capped
    /// occupant recomputes the share from the `used` before its take (so the
    /// next ones can over-allocate and `used` can wrap), every visited
    /// occupant counts `k` down and gets `usedProp` written, even with 0.
    pub(super) fn distribute_used(
        &self,
        s: &PollutionState,
        list: usize,
        cap_prop: u32,
        used_prop: u32,
        total: u32,
        used: u32,
    ) {
        let h = &*self.host;
        let occs = &s.lists[list];
        let n = occs.len() as u32;
        if total == 0 || n == 0 || used == 0 {
            return;
        }
        let mut used = used;
        let mut share = total.min(used) / n;
        let mut k = n;
        for &occ in occs {
            if used == 0 {
                break;
            }
            k -= 1;
            let (ok, cur) = self.current_capacity_in(s, cap_prop, occ);
            if !ok {
                continue;
            }
            let Some(already) = p_u32(h, occ, used_prop) else {
                continue;
            };
            let headroom = cur.saturating_sub(already);
            let take = if share as i32 <= headroom as i32 {
                share
            } else {
                // k = 0 keeps the share.
                share = used.checked_div(k).unwrap_or(share);
                headroom
            };
            used = used.wrapping_sub(take);
            h.set_occupant_property(occ, used_prop, u32_prop(already.wrapping_add(take)));
        }
    }

    /// The maintenance cost update of one occupant (§4.9.4).
    fn update_maintenance_cost(&self, occ: u64) {
        let h = &*self.host;
        let (ok, cost) = self.calc_occupant_maintenance_cost(occ);
        if !ok {
            return;
        }
        if let Some(item) = h.budget_item_for_purpose(occ, budget::PURPOSE) {
            if item.cost != cost {
                h.add_to_budget_item_line_expenses(&item, cost.wrapping_sub(item.cost));
                h.change_budget_item_cost(occ, item.index, cost);
            }
        }
    }
}

/// One relaxation step toward `target` by at most `delta` (§4.4.4 step 9).
fn relax(v: i64, target: i64, delta: i64) -> i64 {
    if v < target {
        v + (target - v).min(delta)
    } else if v > target {
        v - (v - target).min(delta)
    } else {
        v
    }
}
