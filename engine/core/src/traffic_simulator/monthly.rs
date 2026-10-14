// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! StartRecalc and FinalCalculations (§4.12), `DoMonthlySimulation` with
//! the funding damage, the potholes, the strike, the accident check and the
//! history (§4.12, §4.13).

use crate::grid::CellIterator;
use crate::pathfinder::rect_contains;
use crate::x87::{ftol, r24};

use super::cycle::Run;
use super::state::DamageBlock;
use super::{history, msg, SimMessage, TrafficSimulator, PC_REROUTE_BUCKET};

/// The strike constants (§4.12 step 2, code only).
pub const STRIKE_START: f32 = 0.7;
pub const STRIKE_KEEP: f32 = 1.0;
pub const STRIKE_MAX_MONTHS: i32 = 24;
pub const STRIKE_SCALE: f32 = 1.0;
/// The commute history scale (`0x00AB32C0`).
const COMMUTE_HISTORY_SCALE: f32 = 25.0;

/// The damage kinds of `ApplyPotholeChanges` (§4.12).
pub mod kind {
    pub const ROAD: u32 = 1;
    pub const RAIL: u32 = 2;
}

/// The damage-rate update of `ApplyRoad/MassTransitFunding` (§4.12 step 1):
/// `d := r24(curve(r24(f × 100)) + d)`, then clamped to 0…1 (NaN passes);
/// returns the new rate and the target `low32(ftol(r24(float(cellCount) ×
/// d)))`.
pub fn damage_target(curve: &crate::curve::ResponseCurve, f: f32, d: f32, cell_count: u32) -> (f32, u32) {
    let x = r24(f64::from(f) * 100.0) as f32;
    // NaN passes the clamp.
    let d = (r24(f64::from(curve.eval(x)) + f64::from(d)) as f32).clamp(0.0, 1.0);
    let t = r24(r24(f64::from(cell_count)) * f64::from(d));
    (d, ftol(t) as u32)
}

impl Run<'_, '_> {
    // ---- §4.12 StartRecalc and FinalCalculations ----

    /// StartRecalc (state 0, `0x00717C90`). `RecalculateNeighborConnection
    /// Capacities` is not specified by traffic-network.md *[question]*.
    pub(crate) fn start_recalc(&mut self) {
        let st = &mut *self.st;
        st.cycle_start_date = self.h.sim_date_number();
        st.income_acc.set(0.0);
        st.fee_acc = 0.0;
        st.job_capacity = [0; 3];
        st.attempted = [0; 3];
        st.routed = [0; 3];
        st.pass_counters[PC_REROUTE_BUCKET] = st.pass_counters[PC_REROUTE_BUCKET].wrapping_add(1) & 3;
        st.car_len_sum = 0.0;
        st.car_len_count = 0;
        st.freight_len_sum = 0.0;
        st.freight_count = 0;
    }

    /// FinalCalculations (state 8, `0x0070E7D0`).
    pub(crate) fn final_calculations(&mut self) {
        let st = &mut *self.st;
        st.transit_income = st.income_acc.get() as f32;
        st.toll_income = st.fee_acc as f32;
        // No zero check *[quirk]*: 0/0 = NaN.
        st.avg_car_commute = r24(f64::from(st.car_len_sum) / r24(f64::from(st.car_len_count))) as f32;
        st.freight_byte = if st.freight_count != 0 {
            let a = r24(f64::from(st.freight_len_sum) / r24(f64::from(st.freight_count)));
            let b = r24(a / f64::from(st.tuning.commute_max_time));
            let c = r24(b * 255.0);
            ftol(c) as u8
        } else {
            0
        };
    }

    // ---- §4.12 DoMonthlySimulation ----

    pub(crate) fn do_monthly_simulation(&mut self) {
        // 1. The funding damage, road then rail. A missing department counts
        // as full funding *[choice]*.
        let road_f = self.st.roads.map_or(1.0, |d| self.h.department_funding(d));
        let transit_f = self.st.mass_transit.map_or(1.0, |d| self.h.department_funding(d));
        self.apply_funding(kind::ROAD, road_f);
        self.apply_funding(kind::RAIL, transit_f);
        // 2.
        self.update_strikes(transit_f);
        // 3.
        self.check_for_accident();
        // 4.
        self.record_history();
    }

    /// `ApplyRoadFunding` / `ApplyMassTransitFunding` (§4.12 step 1).
    fn apply_funding(&mut self, kind: u32, f: f32) {
        let (rate, count, damaged) = {
            let d = self.dmg.borrow();
            let b = if kind == kind::ROAD { &d.road } else { &d.rail };
            (b.rate, b.cell_count, b.damaged_count)
        };
        let (rate, target) = damage_target(&self.st.tuning.damage_curve, f, rate, count);
        {
            let mut d = self.dmg.borrow_mut();
            let b = if kind == kind::ROAD { &mut d.road } else { &mut d.rail };
            b.rate = rate;
        }
        if target != damaged {
            self.apply_pothole_changes(kind, target.wrapping_sub(damaged) as i32);
            let mut d = self.dmg.borrow_mut();
            let b = if kind == kind::ROAD { &mut d.road } else { &mut d.rail };
            b.damaged_count = target;
        }
    }

    /// `ApplyPotholeChanges(kind, delta)` (`0x00715FB0`, §4.12, draw 6).
    pub(crate) fn apply_pothole_changes(&mut self, kind: u32, delta: i32) {
        let damaging = delta > 0;
        let moves = {
            let d = self.dmg.borrow();
            let b = if kind == kind::ROAD { &d.road } else { &d.rail };
            let src = if damaging { &b.intact } else { &b.damaged };
            delta.unsigned_abs().min(src.len() as u32)
        };
        let extra = if damaging {
            self.cx.s.tuning.damaged_road_cost
        } else {
            0.0
        };
        for _ in 0..moves {
            let key = {
                let d = self.dmg.borrow();
                let b = if kind == kind::ROAD { &d.road } else { &d.rail };
                let src = if damaging { &b.intact } else { &b.damaged };
                let order = src.iter_order();
                let r = self.st.rng.uniform_u32(order.len() as u32) as usize;
                order.get(r).copied()
            };
            let Some(key) = key else { break };
            let (x, z) = ((key >> 8) as i32 & 0xFF, (key & 0xFF) as i32);
            let has_info = match self.cx.s.network_info_mut(kind, x, z) {
                Some(info) => {
                    info.extra_cost = extra;
                    true
                }
                None => false,
            };
            {
                let mut d = self.dmg.borrow_mut();
                let b: &mut DamageBlock = if kind == kind::ROAD { &mut d.road } else { &mut d.rail };
                if damaging {
                    b.intact.remove(key);
                    b.damaged.insert(key);
                } else {
                    b.damaged.remove(key);
                    b.intact.insert(key);
                }
            }
            if has_info {
                self.h.post_message(SimMessage {
                    msg_type: msg::CELL_DAMAGE,
                    data1: x as u32,
                    data2: z as u32,
                    data3: u32::from(damaging),
                });
            }
        }
    }

    /// `UpdateStrikes` (`0x0070DBA0`, §4.12 step 2, draw 7).
    fn update_strikes(&mut self, f: f32) {
        let e = self.st.budget_total;
        let st = &mut *self.st;
        let mut post = None;
        match st.strike_state {
            2 => {
                if f < STRIKE_START && e > 0 {
                    let p = r24(r24(f64::from(STRIKE_START) - f64::from(f)) * f64::from(STRIKE_SCALE)) as f32;
                    st.strike_probability = p;
                    let u = st.rng.uniform_f64();
                    if f64::from(p) > u {
                        st.strike_state = 0;
                        post = Some(msg::STRIKE_STARTS);
                    }
                }
            }
            0 => {
                if f < STRIKE_KEEP && e != 0 {
                    st.strike_state = 1;
                    st.strike_months = 0;
                    st.strike_flag = true;
                    post = Some(msg::STRIKE_ON);
                } else {
                    st.strike_state = 2;
                    post = Some(msg::STRIKE_OVER_A);
                }
            }
            1 => {
                if f < STRIKE_KEEP && st.strike_months < STRIKE_MAX_MONTHS && e != 0 {
                    st.strike_months = st.strike_months.wrapping_add(1);
                } else {
                    st.strike_state = 2;
                    st.strike_flag = false;
                    post = Some(msg::STRIKE_OVER_B);
                }
            }
            // An unknown state does nothing *[choice]*.
            _ => {}
        }
        if let Some(m) = post {
            self.h.post_message(SimMessage::of_type(m));
        }
    }

    /// `CheckForAccident` (`0x007184E0`, §4.13, draw 3), on the simulation
    /// clock *[choice]*: due when the clock has reached the next check; the
    /// next check is `now + ftol(r24(period × 1000))` ms. Without a view there
    /// is no visible rect and nothing is checked *[choice]*.
    fn check_for_accident(&mut self) {
        let now = self.h.sim_clock_ms();
        if now < self.st.next_accident_check {
            return;
        }
        let period = ftol(r24(f64::from(self.st.tuning.accident_check_period) * 1000.0));
        self.st.next_accident_check = now.wrapping_add(period as u64);
        let Some(view) = self.h.view_cell_rect() else { return };
        let list = self.st.accident_check.clone();
        let mut it = CellIterator::new(0, 0, list.len() as i32 - 1, 0);
        it.reset(true, &mut self.st.crt);
        let duration = ftol(r24(f64::from(self.st.tuning.accident_duration) * 1000.0));
        while let Some((i, _)) = it.next_cell() {
            let Some(e) = list.get(i as usize) else { continue };
            let u = self.st.rng.range_f64(0.0, 1.0);
            if u < f64::from(e.p) && rect_contains(view, e.x, e.z) && self.h.cell_has_network_occupant(e.x, e.z) {
                self.h.create_accident(e.x, e.z, duration);
                self.h.post_message(SimMessage {
                    msg_type: msg::TRAFFIC_ACCIDENT,
                    data1: e.x as u32,
                    data2: e.z as u32,
                    data3: 0,
                });
                break;
            }
        }
    }

    /// The history points (§4.12 step 4).
    fn record_history(&mut self) {
        let st = &*self.st;
        let congestion = ftol(r24(f64::from(st.stats_last.rows[0].sum) * 100.0)) as u32;
        self.h.record_history(history::CONGESTION, congestion);
        let commute = ftol(r24(f64::from(st.avg_car_commute) * f64::from(COMMUTE_HISTORY_SCALE))) as u32;
        self.h.record_history(history::COMMUTE, commute);
        for (i, &c) in st.type_count_published.iter().enumerate() {
            self.h.record_history(history::TRAVEL_TYPES + i as u32, c);
        }
        self.h.record_history(history::FERRY, st.ferry_total);
    }
}

impl TrafficSimulator {
    /// `DoMonthlySimulation` (`0x00718AC0`, §4.12).
    pub fn do_monthly_simulation(&self) {
        self.run(|r| r.do_monthly_simulation());
    }

    /// `ApplyPotholeChanges(kind, delta)` (§4.12) on its own, for hosts and
    /// tests.
    pub fn apply_pothole_changes(&self, kind: u32, delta: i32) {
        self.run(|r| r.apply_pothole_changes(kind, delta));
    }
}
