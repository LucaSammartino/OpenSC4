// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The recompute cycle (§6: `DoIdle`, `UpdatePowerGrid`, publishing,
//! statistics) and plants and poles (§7: output, costs, ageing, the month,
//! pole decay).

use crate::grid::{CellIterator, CellRect};
use crate::variant::{Elem, Variant};

use super::simulator::{Cx, FoundPlants};
use super::{
    budget, deal, div_i, div_ii, ftol32, ftol64, history, msg, mul_i, network_flag, occupant_type, prop, state,
    AGED_WEAR, DEFAULT_T1, DEFAULT_T2, DESTROY_EXTRA_FLAGS, GROW_ITERATIONS, MAX_CONNECTIONS_PER_DEAL,
    ORDINANCE_FACTOR_ID, PLANT_TYPES, PLANT_TYPE_NUCLEAR, PRODUCTION_RATIO,
};

/// The 8 × 8 block rectangle of block (bx, bz).
fn block_rect(bx: i32, bz: i32) -> CellRect {
    let (x, z) = (bx.saturating_mul(8), bz.saturating_mul(8));
    CellRect::new(x, z, x.saturating_add(7), z.saturating_add(7))
}

impl Cx<'_> {
    /// A cell clamped to the map.
    fn clamp_cell(&self, (x, z): (i32, i32)) -> (i32, i32) {
        let (cx, cz) = self.cells();
        (x.clamp(0, (cx - 1).max(0)), z.clamp(0, (cz - 1).max(0)))
    }

    fn department_funding(&self) -> f32 {
        self.i.s.department.map_or(1.0, |d| self.h.department_funding(d))
    }

    // ---- output (§7.1) ------------------------------------------------------------------------------

    /// Slot 46 / condition: the wear → condition curve at the plant's wear,
    /// 0 without the curve.
    pub(crate) fn condition(&self, occ: u64) -> f32 {
        match self.prop_curve(occ, prop::CONDITION_CURVE) {
            Some(c) => c.eval(self.prop_f32(occ, prop::WEAR).unwrap_or(0.0)),
            None => 0.0,
        }
    }

    /// Slot 45 / efficiency (§7.1).
    pub(crate) fn efficiency(&self, occ: u64) -> f32 {
        if occ == 0 {
            return 0.0;
        }
        let funded = match self.h.budget_item_for_purpose(occ, budget::PURPOSE) {
            Some(item) => self.i.s.tuning.funding_efficiency.eval(item.local_funding * 100.0) * 0.01f32,
            None => 0.0,
        };
        funded * self.condition(occ)
    }

    /// `GetCurrentPowerProduced` (§7.1).
    pub(crate) fn current_power_produced(&self, occ: u64) -> u32 {
        let Some(cap) = self.prop_u32(occ, prop::CAPACITY) else {
            return 0;
        };
        ftol64(mul_i(f64::from(cap), self.efficiency(occ))) as u32
    }

    // ---- DoIdle (§6) --------------------------------------------------------------------------------

    pub(crate) fn do_idle(&mut self) {
        if !self.i.s.enabled {
            return;
        }
        if self.i.s.wire_dirty {
            let r = self.i.s.dirty_rect;
            self.i.s.wire.rect_put(r, false);
            self.update_wire_bits(r);
            self.i.s.wire_dirty = false;
        }
        match self.i.s.state {
            state::IDLE => {
                if self.i.s.recompute {
                    self.i.s.recompute = false;
                    self.i.s.state = state::RESET;
                }
            }
            state::RESET => {
                self.reset();
                self.i.s.state = state::PLANTS;
            }
            state::PLANTS => {
                self.plants_step();
                self.i.s.state = state::NEIGHBOURS;
            }
            state::NEIGHBOURS => {
                self.neighbours_step();
                self.i.s.state = state::STATISTICS;
            }
            state::STATISTICS => self.statistics(),
            _ => self.i.s.state = state::IDLE,
        }
    }

    /// `UpdateWireBits(rect)` (§5.3 steps 1–3).
    fn update_wire_bits(&mut self, rect: CellRect) {
        let radius = self.i.s.tuning.radius;
        let big = self.expand(rect, radius.saturating_mul(2));
        for occ in self.h.occupants_in_cells(big) {
            let r = match self.h.occupant_type(occ) {
                occupant_type::BUILDING => Some(self.building_rect(occ)),
                occupant_type::NETWORK => self.h.network_occupant_cell(occ).and_then(|(x, z)| {
                    (self.h.network_flags(occ) & network_flag::CONDUCTS != 0).then(|| CellRect::new(x, z, x, z))
                }),
                occupant_type::POWER_POLE => self.h.power_pole(occ).map(|p| self.neighbourhood(p.cell.0, p.cell.1)),
                occupant_type::POWER_LINE => self.h.power_line(occ).map(|l| self.neighbourhood(l.cell.0, l.cell.1)),
                _ => None,
            };
            if let Some(r) = r {
                self.wire_rect(r);
            }
        }
        let (cx, cz) = self.cells();
        let (x1, z1) = (rect.x1.max(0), rect.z1.max(0));
        let (x2, z2) = (rect.x2.min(cx - 1), rect.z2.min(cz - 1));
        for x in x1..=x2 {
            for z in z1..=z2 {
                if self.h.zone_type(x, z) != 0 {
                    let n = self.neighbourhood(x, z);
                    self.wire_rect(n);
                }
            }
        }
        for x in x1..=x2 {
            for z in z1..=z2 {
                self.write_grid_cell(x, z);
            }
        }
    }

    /// State 0 (§6.1).
    fn reset(&mut self) {
        let s = &mut self.i.s;
        s.ring_index = s.ring_index.wrapping_add(1).rem_euclid(12);
        let row = s.ring_row();
        for t in 0..PLANT_TYPES {
            s.annual[t] = s.annual[t].wrapping_sub(s.ring[row][t]);
            s.ring[row][t] = 0;
        }
        for t in [deal::BUY_POWER, deal::SELL_POWER] {
            for d in self.h.find_deals(t) {
                self.h.set_deal_amount_used(d.token, 0);
            }
        }
        for c in self.h.connections_of_type(deal::CONNECTION_POWER_POLE) {
            self.h.set_connection_visited(c, false);
        }
        let s = &mut self.i.s;
        s.production = 0;
        s.consumption = 0;
        s.export = 0;
        s.import = 0;
        s.demand = 0;
        s.powered = 0;
        s.unpowered = 0;
        for p in &mut s.plants {
            p.visited = 0;
        }
        s.powered_next.clear();
        s.changed = false;
    }

    /// State 1 (§6.2).
    fn plants_step(&mut self) {
        let mut total = self.i.s.cost_base;
        let n = self.i.s.plants.len();
        for i in 0..n {
            let Some(node) = self.i.s.plants.get(i).copied() else {
                break;
            };
            if node.visited == 0 {
                let p = self.h.occupant_position(node.occupant);
                let (x, z) = self.position_to_cell(p[0], p[2]);
                self.update_power_grid(x, z);
            }
            total = total.wrapping_add(self.plant_costs(node.occupant));
        }
        if let Some(d) = self.i.s.department {
            if let Some(li) = self.h.line_item(d, budget::LINE_POLES) {
                self.h.set_line_item_full_expenses(li, total);
            }
        }
        if self.i.s.age_plants {
            let s = &mut self.i.s;
            s.overworked = 0;
            s.aged = 0;
            s.distressed = 0;
            s.age_plants = false;
            let plants: Vec<u64> = s.plants.iter().map(|p| p.occupant).collect();
            for occ in plants {
                self.age_plant(occ);
            }
        }
    }

    /// The flood fill of §6.3 step 1; returns the clamped `(rowMin, rowMax)`.
    pub(crate) fn flood(&mut self, x: i32, z: i32) -> (i32, i32) {
        let (_, cz) = self.cells();
        let s = &mut self.i.s;
        s.mark.clear();
        s.mark.set(x, z);
        let (lo, hi) = s.mark.selection_grow(&s.wire, GROW_ITERATIONS, z, z.saturating_add(1));
        let hi = hi.min(cz - 1);
        let mut lo = lo.max(0);
        if lo > hi {
            lo = hi;
        }
        if hi < 0 {
            return (0, 0);
        }
        (lo, hi)
    }

    /// `IsNeighborConnected(city, markVisited, light)` (§6.4).
    fn is_neighbor_connected(&mut self, city: u64, mark_visited: bool, light: bool) -> bool {
        for conn in self.h.connections_to_city(deal::CONNECTION_POWER_POLE, city) {
            let (x, z) = self.clamp_cell(self.h.connection_cell(conn));
            if self.i.s.mark.test(x, z) {
                if mark_visited {
                    self.h.set_connection_visited(conn, true);
                }
                if light {
                    self.i.s.powered_next.set(x, z);
                }
                return true;
            }
        }
        false
    }

    /// `UpdatePowerGrid(x, z)` (§6.3).
    pub(crate) fn update_power_grid(&mut self, x: i32, z: i32) {
        // 1. The connected network.
        let (row_min, row_max) = self.flood(x, z);

        // 2. Supply.
        let mut supply = 0u32;
        for i in 0..self.i.s.plants.len() {
            let node = self.i.s.plants[i];
            if node.visited != 0 {
                continue;
            }
            let (min, max) = self.h.occupant_bounding_box(node.occupant);
            let (x1, z1) = self.position_to_cell(min[0], min[2]);
            let (x2, z2) = self.position_to_cell(max[0], max[2]);
            if self.i.s.mark.rect_any(CellRect::new(x1, z1, x2, z2)) {
                supply = supply.wrapping_add(self.current_power_produced(node.occupant));
                self.i.s.plants[i].visited = 1;
            }
        }
        self.i.s.production = self.i.s.production.wrapping_add(supply);

        // 3. Import capacity.
        let buys = self.h.find_deals(deal::BUY_POWER);
        let mut imp = 0u32;
        for d in &buys {
            let used = self.h.deal_amount_used(d.token);
            if used < d.amount && self.is_neighbor_connected(d.city, true, false) {
                imp = imp.wrapping_add(d.amount.wrapping_sub(used) as u32);
            }
        }

        // 4. Availability.
        let factor = self.i.s.factor;
        let scaled = ftol64(mul_i(f64::from(supply), -PRODUCTION_RATIO)) as u32;
        let total = imp.wrapping_sub(scaled);
        let mut avail = ftol64(div_i(f64::from(total), factor)) as u32;

        // 5. Exports.
        for d in self.h.find_deals(deal::SELL_POWER) {
            let used = self.h.deal_amount_used(d.token);
            if used < d.amount && self.is_neighbor_connected(d.city, false, (avail as i32) > 0) {
                let give = d.amount.wrapping_sub(used).min(avail as i32);
                if give > 0 {
                    self.i.s.export = self.i.s.export.wrapping_add(give as u32);
                    avail = avail.wrapping_sub(give as u32);
                    self.h.set_deal_amount_used(d.token, used.wrapping_add(give));
                } else {
                    self.h.set_deal_flags(d.token, deal::FLAG_UNSERVED, true);
                }
            }
        }

        // 6. Blocks in random order (one rand() call).
        let (cx, _) = self.cells();
        let bz0 = row_min >> 3;
        let mut it = CellIterator::new(0, bz0, (cx / 8 - 1).max(0), bz0.max(row_max >> 3));
        it.reset(true, &mut self.i.crt);
        let mut shortfall = false;
        while let Some((bx, bz)) = it.next_cell() {
            let r = block_rect(bx, bz);
            let s = &mut self.i.s;
            let (mut need, mut n) = (0u32, 0u32);
            for x in r.x1..=r.x2 {
                for z in r.z1..=r.z2 {
                    if s.mark.test(x, z) {
                        let c = s.consumption_at(x, z);
                        if c > 0 {
                            need = need.wrapping_add(c as u32);
                            n += 1;
                        }
                    }
                }
            }
            s.demand = s.demand.wrapping_add(need);
            if avail != 0 && need <= avail {
                s.powered_next.rect_or(&s.mark, r);
                s.powered = s.powered.wrapping_add(n);
                avail -= need;
            } else {
                shortfall = true;
            }
        }

        // 7. What was used.
        if shortfall {
            avail = 0;
        }
        let leftover = ftol64(mul_i(f64::from(avail), factor)) as u32;
        let used = total.wrapping_sub(leftover);
        self.i.s.consumption = self.i.s.consumption.wrapping_add(used);

        // 8. Imports.
        let mut remaining = used as i32;
        for d in &buys {
            let used_d = self.h.deal_amount_used(d.token);
            if used_d < d.amount && self.is_neighbor_connected(d.city, false, false) {
                let take = d.amount.wrapping_sub(used_d).min(remaining);
                self.i.s.import = self.i.s.import.wrapping_add(take as u32);
                remaining = remaining.wrapping_sub(take);
                self.h.set_deal_amount_used(d.token, used_d.wrapping_add(take));
            }
        }

        // 9. The plants of this network.
        self.mark_plants_visited(remaining as u32, supply.max(remaining as u32));
    }

    /// `MarkPowerPlantsAsVisited(a, b)` (§6.4).
    fn mark_plants_visited(&mut self, a: u32, b: u32) {
        let ratio = if (self.i.s.production as i32) > 0 {
            div_ii(f64::from(a), f64::from(b))
        } else {
            0.0
        };
        let row = self.i.s.ring_row();
        for i in 0..self.i.s.plants.len() {
            let node = self.i.s.plants[i];
            if node.visited != 1 {
                continue;
            }
            let p = self.current_power_produced(node.occupant);
            let used = ftol32(mul_i(f64::from(p), ratio));
            self.h
                .set_occupant_property(node.occupant, prop::USED, Variant::Scalar(Elem::I32(used)));
            let t = self.plant_type(node.occupant);
            let s = &mut self.i.s;
            if let Some(t) = Cx::type_index(t) {
                s.ring[row][t] = s.ring[row][t].wrapping_add(used);
            }
            s.plants[i].visited = 2;
        }
    }

    /// State 2 (§6.5).
    fn neighbours_step(&mut self) {
        for d in self.h.find_deals(deal::BUY_POWER) {
            if self.h.deal_amount_used(d.token) >= d.amount {
                continue;
            }
            for _ in 0..MAX_CONNECTIONS_PER_DEAL {
                let Some(conn) = self.h.find_unvisited_connection(deal::CONNECTION_POWER_POLE, d.city) else {
                    break;
                };
                self.h.set_connection_visited(conn, true);
                if let Some(other) = self.h.connection_on_other_side(conn) {
                    if self.h.connection_is_active(other) {
                        let (x, z) = self.clamp_cell(self.h.connection_cell(conn));
                        self.update_power_grid(x, z);
                    }
                }
            }
        }
        let s = &mut self.i.s;
        std::mem::swap(&mut s.powered_current, &mut s.powered_next);
        self.update_powered_lots();
    }

    /// `UpdatePoweredLots` (§6.5).
    fn update_powered_lots(&mut self) {
        let s = &mut self.i.s;
        s.powered_next.xor(&s.powered_current);
        let (cx, _) = self.cells();
        let blocks = cx >> 3;
        for bz in 0..blocks {
            for bx in 0..blocks {
                let r = block_rect(bx, bz);
                if !self.i.s.powered_next.rect_any(r) {
                    continue;
                }
                for x in r.x1..=r.x2 {
                    for z in r.z1..=r.z2 {
                        if self.i.s.powered_next.test(x, z) {
                            self.write_grid_cell(x, z);
                        }
                    }
                }
                self.post(
                    msg::POWER_BLOCK_CHANGED,
                    [r.x1 as u32, r.z1 as u32, r.x2 as u32, r.z2 as u32],
                    None,
                );
                self.i.s.changed = true;
            }
        }
        if self.i.s.changed {
            self.post(msg::POWER_CHANGED, [0; 4], None);
        }
    }

    /// State 3 (§6.6).
    fn statistics(&mut self) {
        let s = &mut self.i.s;
        s.last_production = s.production;
        s.last_consumption = s.consumption;
        s.last_import = s.import;
        s.last_export = s.export;
        s.last_demand = s.demand;
        let (p, c) = (s.production as i32, s.consumption as i32);
        let (imp, exp, dem) = (s.import as i32, s.export as i32, s.demand as i32);
        let factor = s.factor;
        let h = self.h;
        h.record_history(history::PRODUCTION, p);
        h.record_history(history::IMPORT, imp);
        h.record_history(history::EXPORT, exp);
        h.record_history(
            history::CONSUMPTION_NET,
            ftol32(mul_i(f64::from(c.wrapping_sub(exp)), factor)),
        );
        h.record_history(history::DEMAND, ftol32(mul_i(f64::from(dem), factor)));
        h.record_history(history::SURPLUS, p.wrapping_sub(c));
        for d in h.find_deals(deal::SELL_POWER) {
            if h.deal_amount_used(d.token) < d.amount {
                h.set_deal_flags(d.token, deal::FLAG_UNSERVED, true);
            }
        }
        h.mark_deals_dirty();
        if c > 0 {
            let funding = self.i.s.department.map_or(0, |d| h.budget_funding(d));
            h.record_history(history::COST_PER_UNIT, funding.wrapping_div(i64::from(c)) as i32);
        }
        let s = &mut self.i.s;
        let (consumers, powered) = (s.consumers as i32, s.powered as i32);
        let unpowered_pct = if consumers > 0 {
            let q = div_ii(f64::from(powered), f64::from(consumers));
            ftol32((1.0f32 - q) * 100.0f32)
        } else {
            0
        };
        h.record_history(history::UNPOWERED_PERCENT, unpowered_pct);
        let row = s.ring_row();
        for (t, &id) in history::PER_TYPE.iter().enumerate() {
            h.record_history(id, s.ring[row][t + 1]);
        }
        for t in 0..PLANT_TYPES {
            s.annual[t] = s.annual[t].wrapping_add(s.ring[row][t]);
        }
        s.deficit = if p > c { deficit(p, c) } else { 0.0 };
        s.unpowered = consumers.wrapping_sub(powered).max(0) as u32;
        for conn in h.connections_of_type(deal::CONNECTION_POWER_POLE) {
            let (x, z) = self.clamp_cell(h.connection_cell(conn));
            h.set_connection_active(conn, self.is_powered(x, z));
        }
        self.i.s.state = state::IDLE;
        self.post(msg::DEAL_STATUS, [msg::DEAL_STATUS_DATA1, 0, 0, 0], None);
    }

    // ---- costs (§7.3) -------------------------------------------------------------------------------

    /// The fuel cost of a plant (added to line item 1), after updating its
    /// maintenance cost (§7.3).
    fn plant_costs(&mut self, occ: u64) -> i64 {
        let used = self.prop_i32(occ, prop::USED).unwrap_or(0);
        let per_unit = self.prop_f32(occ, prop::FUEL_COST).unwrap_or(0.0);
        let fuel = ftol64(mul_i(f64::from(used), per_unit));
        if let Some(curve) = self.prop_curve(occ, prop::MAINTENANCE_CURVE) {
            let base = self.prop_i64(occ, prop::BASE_MAINTENANCE).unwrap_or(0);
            let wear = self.prop_f32(occ, prop::WEAR).unwrap_or(0.0);
            let maint = ftol64(mul_i(base as f64, curve.eval(wear)));
            if let Some(item) = self.h.budget_item_for_purpose(occ, budget::PURPOSE) {
                if maint != item.cost {
                    if let Some(dept) = self.h.department(item.department) {
                        self.h.set_local_full_funding(dept, occ, maint, budget::PURPOSE);
                    }
                    self.h.change_budget_item_cost(occ, item.index, maint);
                }
            }
        }
        fuel
    }

    // ---- ageing (§7.4) ------------------------------------------------------------------------------

    /// `AgePowerPlant(occ)` (§7.4).
    pub(crate) fn age_plant(&mut self, occ: u64) {
        let wear0 = self.prop_f32(occ, prop::WEAR).unwrap_or(0.0);
        let rate = self.prop_f32(occ, prop::AGEING_RATE).unwrap_or(0.0);
        let capacity = self.prop_i32(occ, prop::CAPACITY).unwrap_or(0);
        let used = self.prop_i32(occ, prop::USED).unwrap_or(0);
        let t1 = self.prop_f32(occ, prop::AGEING_T1).unwrap_or(DEFAULT_T1);
        let t2 = self.prop_f32(occ, prop::AGEING_T2).unwrap_or(DEFAULT_T2);
        let destroy = self.prop_u32(occ, prop::DESTROY_FLAGS).unwrap_or(0);

        let eff_cap = ftol32(mul_i(f64::from(capacity), self.efficiency(occ)));
        let usage = if eff_cap > 0 && used > 0 {
            div_ii(f64::from(used), f64::from(eff_cap))
        } else {
            0.0
        };
        let funding = self.department_funding();
        let tuning = &self.i.s.tuning;
        let wear = wear0 + tuning.usage_decay.eval(usage) * tuning.funding_decay.eval(funding) * rate;
        self.h
            .set_occupant_property(occ, prop::WEAR, Variant::Scalar(Elem::F32(wear)));
        if usage > 1.0 {
            self.i.s.overworked = self.i.s.overworked.wrapping_add(1);
        }
        if wear >= AGED_WEAR {
            self.i.s.aged = self.i.s.aged.wrapping_add(1);
        }
        if !(wear >= t2 || wear >= t1) {
            return;
        }
        let position = self.h.occupant_position(occ);
        let (x, z) = self.position_to_cell(position[0], position[2]);
        let Some(lot) = self.h.lot_at(x, z) else { return };
        if wear >= t2 && destroy > 0 {
            self.h.demolish_lots(&[lot], destroy | DESTROY_EXTRA_FLAGS);
            let radius = self.prop_f32(occ, prop::RADIATION_RADIUS).unwrap_or(0.0);
            if radius > 0.0 {
                let strength = self.prop_f32(occ, prop::RADIATION_STRENGTH).unwrap_or(0.0);
                self.h.add_radiation(position, strength, radius);
            }
            if self.plant_type(occ) == PLANT_TYPE_NUCLEAR {
                self.post(msg::NUCLEAR_MELTDOWN, [0; 4], Some(occ));
            }
        } else if wear >= t1 {
            self.i.s.distressed = self.i.s.distressed.wrapping_add(1);
            self.h.set_lot_condition(lot, 2);
        }
    }

    // ---- the month (§7.5) and pole decay (§7.6) -----------------------------------------------------

    /// `DoMonthlySimulation` (§7.5).
    pub(crate) fn monthly(&mut self) {
        self.i.s.factor = self.h.cumulative_factor(ORDINANCE_FACTOR_ID, 0);
        let s = &mut self.i.s;
        s.ring_index = s.ring_index.wrapping_add(1).rem_euclid(12);
        let row = s.ring_row();
        for t in 0..PLANT_TYPES {
            s.annual[t] = s.annual[t].wrapping_sub(s.ring[row][t]);
            s.ring[row][t] = 0;
            s.annual[t] = s.annual[t].wrapping_add(s.ring[row][t]);
        }
        let (p, c) = (s.production as i32, s.consumption as i32);
        s.deficit = if c > p && p > 0 { deficit(p, c) } else { 0.0 };
        s.recompute = true;
        s.age_plants = true;
        let funding = self.department_funding();
        self.apply_pole_funding(funding);
    }

    /// `ApplyPoleFunding(pct)` (§7.6); `pct` is a fraction.
    pub(crate) fn apply_pole_funding(&mut self, pct: f32) {
        let s = &mut self.i.s;
        let f = s.broken_fraction - s.tuning.pole_decay.eval(pct * 100.0);
        s.broken_fraction = if f.is_nan() { 0.0 } else { f.clamp(0.0, 1.0) };
        let target = ftol64(mul_i(f64::from(s.pole_count), s.broken_fraction)) as u32;
        if target == s.broken_count {
            return;
        }
        let mut list = s.poles.clone();
        self.i.crt.random_shuffle(&mut list);
        for pole in list {
            let s = &mut self.i.s;
            if s.broken_count == target {
                break;
            }
            let distressed = self.h.pole_is_distressed(pole);
            if distressed && s.broken_count > target {
                self.h.set_pole_distressed(pole, false);
                s.broken_count -= 1;
            } else if !distressed && s.broken_count < target {
                self.h.set_pole_distressed(pole, true);
                s.broken_count += 1;
            }
        }
    }

    /// `FindPowerPlants` (slot 30).
    pub(crate) fn find_power_plants(&self) -> FoundPlants {
        let mut found = FoundPlants::default();
        let (mut lowest, mut ratio) = (2.0f32, 0.0f32);
        for node in &self.i.s.plants {
            let occ = node.occupant;
            let Some(item) = self.h.budget_item_for_purpose(occ, budget::PURPOSE) else {
                continue;
            };
            let Some(dept) = self.h.department(item.department) else {
                continue;
            };
            if !self.h.is_locally_funded_object(dept, occ, budget::PURPOSE) {
                continue;
            }
            if item.local_funding < lowest {
                lowest = item.local_funding;
                found.lowest_funded = Some((occ, lowest));
            }
            let wear = self.prop_f32(occ, prop::WEAR).unwrap_or(0.0);
            let t1 = self.prop_f32(occ, prop::AGEING_T1).unwrap_or(DEFAULT_T1);
            let t2 = self.prop_f32(occ, prop::AGEING_T2).unwrap_or(DEFAULT_T2);
            let r = wear / t1.max(t2);
            if r > ratio {
                ratio = r;
                found.most_worn = Some((occ, ftol32(r * 100.0)));
            }
        }
        found
    }
}

/// `f32(int((production − consumption) / production))` (§6.6 step 7,
/// §7.5 step 3); 0 when production is 0 ([choice]).
fn deficit(p: i32, c: i32) -> f32 {
    if p == 0 {
        0.0
    } else {
        p.wrapping_sub(c).wrapping_div(p) as f32
    }
}
