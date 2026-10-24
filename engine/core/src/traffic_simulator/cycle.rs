// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Init, Shutdown, `DoMessage` (§1, §3.1), `DoDailySimulation` (§3.3),
//! `WorkOnSimCycle` with its next-state table and entry actions (§3.2), and
//! lot activation (§4.1.3).

use std::cell::RefCell;

use crate::grid::{CellRect, GridProvider, ScanIterator};
use crate::pathfinder::{PathCx, Pathfinder};
use crate::traffic_network::{inverse_max_speed, Cx, NetworkHost, NetworkState};
use crate::variant::{Elem, Variant};
use crate::x87::{ftol, r24};

use super::adapters::{capacity_element, set_capacity_element, Holders, PfAdapter};
use super::pools::{pool_token, Pools};
use super::state::{DamageBlock, DamageState, StatRow, State, PASS_COUNTERS, PC_REROUTE_BUCKET, PC_REROUTE_COUNTER};
use super::{
    cell_key16, cycle_state as cs, dept, grid_id, msg, prop, GridF32, GridU8, Message, TrafficSimHost,
    TrafficSimulator, AGENT_FLAGS, AGENT_LEVELS, AGENT_NAME,
};

/// The network-occupant type of the occupant messages (§3.1).
const NETWORK_OCCUPANT_TYPE: u32 = crate::traffic_network::occupant_type::NETWORK;

/// The network types of the bookkeeping calls (§3.6, TS4): Avenue and Road
/// each, then the first of OneWayRoad, Street, Highway, GroundHighway; Rail;
/// Subway.
const AVENUE: u32 = 6;
const ROAD: u32 = 0;
const ROAD_LIKE: [u32; 4] = [10, 3, 2, 12];
const RAIL: u32 = 1;
const SUBWAY: u32 = 7;
/// Highway and GroundHighway are not in the pothole sets (§3.6).
const HIGHWAYS: [u32; 2] = [2, 12];
/// The network flag that keeps a subway occupant out of the count (§3.6
/// step 5, slot 8).
const SUBWAY_SKIP_FLAG: u32 = 0x8000_0000;
/// The level-crossing test of DoRailChanged (slot 9, any of Road | Street).
const LEVEL_CROSSING_MASK: u32 = 9;

/// The context of every operation that touches both the simulator state and
/// the network state (see the module documentation of `mod.rs`).
pub(crate) struct Run<'r, 'c> {
    pub(crate) h: &'r dyn TrafficSimHost,
    pub(crate) st: &'r mut State,
    pub(crate) cx: &'r mut Cx<'c>,
    pub(crate) dmg: &'r RefCell<DamageState>,
    pub(crate) pools: &'r RefCell<Pools>,
}

impl Run<'_, '_> {
    /// The property holders as the simulator sees them (pools included).
    pub(crate) fn holders(&self) -> Holders<'_> {
        Holders {
            h: self.h,
            pools: self.pools,
        }
    }

    // ---- pathfinders ----

    /// `CreatePathFinder` (slot 6): a pathfinder initialised against the
    /// simulator (pathfinder.md §1).
    pub(crate) fn create_path_finder(&mut self) -> Pathfinder {
        let host = PfAdapter {
            h: self.h,
            st: &*self.st,
            pools: self.pools,
        };
        Pathfinder::create(&host, self.st.shared.clone())
    }

    /// `FindPath` of a pathfinder over the simulator's network and state.
    pub(crate) fn find_path_with(&mut self, pf: &mut Pathfinder) -> u32 {
        let host = PfAdapter {
            h: self.h,
            st: &*self.st,
            pools: self.pools,
        };
        pf.find_path(&mut PathCx {
            net: &mut *self.cx.s,
            host: &host,
        })
    }

    /// `Init` (pathfinder slot 3) of a pathfinder (pathfinder.md §1).
    pub(crate) fn init_path_finder(&mut self, pf: &mut Pathfinder) -> bool {
        let host = PfAdapter {
            h: self.h,
            st: &*self.st,
            pools: self.pools,
        };
        pf.init(&host)
    }

    /// `ApplyTraffic(style)` (pathfinder slot 29, pathfinder.md §4.17) of a
    /// pathfinder over the simulator's network and state.
    pub(crate) fn apply_path_finder_traffic(&mut self, pf: &mut Pathfinder, style: u32) {
        let host = PfAdapter {
            h: self.h,
            st: &*self.st,
            pools: self.pools,
        };
        pf.apply_traffic(
            &mut PathCx {
                net: &mut *self.cx.s,
                host: &host,
            },
            style,
        );
    }

    pub(crate) fn net(&self) -> &NetworkState {
        self.cx.s
    }

    // ---- §3.3 DoDailySimulation ----

    /// `DoDailySimulation` (§3.3) with the work budget of §3.4.
    pub(crate) fn do_daily_simulation(&mut self) -> bool {
        self.day_start();
        // Step 3.
        loop {
            let finished = self.work_on_sim_cycle();
            if let Some(r) = self.after_slice(finished, true) {
                return r;
            }
        }
    }

    /// Step 2 of §3.3: on a new day of month, day 1's incomes and the
    /// strategy re-draw.
    pub(crate) fn day_start(&mut self) {
        let day = self.h.sim_day_of_month() as i32;
        if day != self.st.last_day {
            self.st.last_day = day;
            if day == 1 {
                self.pay_monthly_income();
            }
            self.strategy_redraw();
        }
    }

    /// Step 3 of §3.3 after one `WorkOnSimCycle` that `finished` or not:
    /// `None` runs another slice within the same call, `Some(r)` ends the
    /// call with `r`. Without `share` (the traffic shadow's stepping) the
    /// share test is skipped: an unfinished slice ends the call with false.
    pub(crate) fn after_slice(&mut self, finished: bool, share: bool) -> Option<bool> {
        let s = self.st.cycle_state;
        if finished {
            return match s {
                cs::CONFIRM_TRIPS => {
                    // `toReRoute` and `toSearch` are zeroed here and only
                    // here (`0x0072184B`, TS21), with every pass counter
                    // except 1 (the re-route counter) and 2
                    // (`rerouteBucket`): the source counter (3) restarts
                    // at 0 every cycle (TS1).
                    self.start_pacing(self.st.pass_total, 20);
                    self.st.to_reroute = 0;
                    self.st.to_search = 0;
                    for i in 0..PASS_COUNTERS {
                        if i != PC_REROUTE_COUNTER && i != PC_REROUTE_BUCKET {
                            self.st.pass_counters[i] = 0;
                        }
                    }
                    None
                }
                cs::REROUTE_TRIPS => {
                    self.start_pacing(self.st.to_reroute, 18);
                    // Counter 1 is zeroed on entering 17 (`0x007218F2`).
                    self.st.pass_counters[PC_REROUTE_COUNTER] = 0;
                    Some(true)
                }
                cs::FULL_TRIPS => {
                    // `0x007217BF`–`0x007217FD`: day index, processed
                    // and `passTotal`, **not** pass counter 1 (TS35): it
                    // keeps state 17's re-route count through state 18.
                    self.start_pacing(self.st.to_search, 18);
                    self.st.pass_total = 0;
                    Some(true)
                }
                cs::CREATE_MAPS | cs::UPDATE_NEIGHBOURS => Some(true),
                _ => None,
            };
        }
        // The share test (`0x0072191E`, all unsigned), with the
        // `dayIndex < days` guard: once the day index has reached `days`
        // (more trips than the last cycle, or `days = 0` after a load,
        // §3.5 step 13) no share ends the day.
        if share && (cs::CONFIRM_TRIPS..=cs::FULL_TRIPS).contains(&s) && self.st.day_index < self.st.days {
            let share = (self.st.total as u32).wrapping_mul(self.st.day_index.wrapping_add(1)) / self.st.days;
            if self.st.processed as u32 >= share {
                self.st.day_index = self.st.day_index.wrapping_add(1);
                return Some(true);
            }
        }
        // Not done for today: the simulator calls again within the day.
        Some(false)
    }

    /// The pacing reset of §3.3: `total`, `days := max(1, total div k)`, day
    /// index and processed 0. The pass counters are the caller's (TS1).
    fn start_pacing(&mut self, total: i32, k: i32) {
        self.st.total = total;
        self.st.days = (total / k).max(1) as u32;
        self.st.day_index = 0;
        self.st.processed = 0;
    }

    /// Day 1 (§3.3): the incomes into the line items and the last monthly
    /// income. A missing department is skipped *[choice]*.
    fn pay_monthly_income(&mut self) {
        if let Some(d) = self.st.mass_transit {
            self.h
                .set_line_item_income(d, dept::TRANSIT_INCOME_ITEM, ftol(f64::from(self.st.transit_income)));
        }
        if let Some(d) = self.st.roads {
            self.h
                .set_line_item_income(d, dept::TOLL_INCOME_ITEM, ftol(f64::from(self.st.toll_income)));
        }
        self.st.last_monthly_income = r24(f64::from(self.st.toll_income) + f64::from(self.st.transit_income)) as f32;
    }

    /// The strategy re-draw (§3.3, draw 1 of §4.15). A tuning with 0 buckets
    /// or `period / buckets = 0` divides by zero in the original; FreeCity4
    /// skips the re-draw *[choice]*.
    fn strategy_redraw(&mut self) {
        let buckets = self.st.tuning.strategy_buckets;
        if buckets == 0 {
            return;
        }
        let q = self.st.tuning.strategy_period / buckets;
        if q == 0 || self.h.sim_date_number() % q != 0 {
            return;
        }
        let cells = self
            .st
            .buckets
            .get(&self.st.strategy_bucket)
            .cloned()
            .unwrap_or_default();
        for (x, z) in cells {
            if let Some(lot) = self.h.lot_at(x, z, false) {
                let v = self.st.rng.uniform_u32(100);
                let holder = self.h.lot_holder(lot);
                if holder != 0 {
                    self.h
                        .set_holder_property(holder, prop::STRATEGY_DRAW, Variant::Scalar(Elem::U8(v as u8)));
                }
            }
        }
        self.st.strategy_bucket = (self.st.strategy_bucket.wrapping_add(1)) % buckets;
    }

    // ---- §3.2 WorkOnSimCycle ----

    /// `WorkOnSimCycle` (§3.2): the dirty intersection data first, then the
    /// current phase; on finishing, the next state and its entry action.
    pub(crate) fn work_on_sim_cycle(&mut self) -> bool {
        self.cx.s.update_intersection_data();
        let finished = match self.st.cycle_state {
            cs::START_RECALC => {
                self.start_recalc();
                true
            }
            cs::CONFIRM_TRIPS => self.confirm_trips_slice(),
            cs::REROUTE_TRIPS => self.trip_slice(false),
            cs::FULL_TRIPS => {
                let done = self.trip_slice(true);
                if done {
                    // End of FullTrips: the published and building sets are
                    // swapped (TS56, confirmed; the traffic_shadow_hypotheses
                    // bits were retired once it was).
                    std::mem::swap(&mut self.st.type_count_published, &mut self.st.type_count_building);
                }
                done
            }
            cs::FINISH_TRIP_MAP => self.finish_trip_map(),
            cs::CREATE_MAPS => self.create_maps(),
            cs::CREATE_FINAL_MAP => self.create_final_map(),
            cs::CLEAR_DESTINATIONS => self.clear_trip_destinations(),
            cs::UPDATE_NEIGHBOURS => {
                self.update_neighbor_connections();
                true
            }
            cs::REDUCE_SWITCHES => {
                self.reduce_transit_switches();
                true
            }
            cs::DUMP_EDGES => {
                self.dump_edge_densities();
                true
            }
            cs::REDUCE_EDGES => self.reduce_edge_densities(),
            cs::REFRESH_COMMUTE => self.refresh_commute_map(),
            cs::UPDATE_SUBNETWORKS => {
                // traffic-network.md §4.11, in one call; the stale flag is
                // consumed here *[choice]*.
                self.cx.s.clear_subnetworks();
                self.cx.s.update_subnetworks();
                self.cx.s.subnetworks_stale = false;
                true
            }
            cs::SUBNETWORK_INFO => self.update_subnetwork_info(),
            cs::FERRY_ROUTES => {
                while !self.cx.ferry_pass_step() {}
                true
            }
            cs::FINAL_CALCULATIONS => {
                self.final_calculations();
                true
            }
            // 1, 9, 14 and anything else finish at once.
            _ => true,
        };
        if finished {
            let s = usize::from(self.st.cycle_state);
            let mut next = cs::NEXT.get(s).copied().unwrap_or(cs::START_RECALC);
            if next == cs::UPDATE_SUBNETWORKS && !self.cx.s.subnetworks_stale {
                next = cs::SUBNETWORK_INFO;
            }
            self.st.cycle_state = next;
            self.enter_state(next);
        }
        finished
    }

    /// The entry actions of §3.2.
    pub(crate) fn enter_state(&mut self, s: u8) {
        match s {
            cs::CONFIRM_TRIPS | cs::REROUTE_TRIPS | cs::FULL_TRIPS => {
                // The traffic-source iterator's `Reset` (§4.1, TS20): the
                // connections a snapshot, the lots walked live.
                self.reset_sources();
                self.st.source_pos = 0;
                self.st.trip_pos = 0;
                self.st.continue_search = false;
                // The saved source cell, the two s16 phase positions of row
                // 14, := (0, 0) (§3.2, TS44: the table's "pass counters
                // reset" was these two).
                self.st.extras.phase_positions = [0, 0];
                // The pass counters, `toReRoute` and `toSearch` are reset by
                // the day agent's pacing (§3.3, TS1, TS21), not here.
                if s == cs::FULL_TRIPS {
                    self.st.type_count_building = [0; 9];
                }
            }
            cs::CREATE_MAPS => {
                self.cx.reset_subnetwork_working();
                self.reset_scan();
                // The statistics reset (§4.9, TS40): the five fields of
                // every working row and the record counts 0, the separate
                // per-slot comparison value −1.0.
                let w = &mut self.st.stats_work;
                w.rows = [StatRow::RESET; 13];
                w.bridge = StatRow::RESET;
                w.records = [0; 13];
                w.bridge_records = 0;
                self.st.stat_counts = [0; 14];
                self.st.stat_max_cmp = [-1.0; 14];
                if let Some(g) = &self.st.commercial_work {
                    g.fill(0);
                }
                self.st.accidents.clear();
            }
            cs::CREATE_FINAL_MAP => {
                self.reset_scan();
                if let (Some(src), Some(dst)) = (&self.st.commercial_work, &self.st.commercial) {
                    for tx in 0..dst.count_x() {
                        for tz in 0..dst.count_z() {
                            dst.set(tx, tz, src.get(tx, tz));
                        }
                    }
                }
            }
            cs::CLEAR_DESTINATIONS | cs::REDUCE_EDGES => self.reset_scan(),
            cs::REFRESH_COMMUTE => {
                for k in 0..4 {
                    self.st.set_free(k, 0);
                }
                self.reset_scan();
            }
            cs::SUBNETWORK_INFO => {
                // [TS64] Entering 13 resets the scanning iterator (the member
                // `+0x818`, the one saved in §6.1 row 14) to state 0 at
                // (0, 0) with its rectangle and step kept (state 2 at once if
                // (0, 0) is outside the rectangle). Entering 12 does not touch
                // it, and UpdateSubnetworks (state 12) does not use it.
                self.reset_scan();
                self.cx.begin_subnetwork_info();
            }
            cs::FERRY_ROUTES => {
                let riders = self.cx.begin_ferry_pass();
                self.st.ferry_total = self.st.ferry_total.wrapping_add(riders);
            }
            // FinishTripMap (15) and the lot sweep (1) share one entry arm
            // (§3.2 rows 15 and 1, TS67): the lot iterator reset (the walk
            // starts at the first lot, so no resume point is kept) and the
            // two s16 phase positions := (0, 0), so a stop before the first
            // lot leaves (0, 0), not the last trip-phase source's cell. The
            // "take the next source" flag `+0x838` := 1 has no core field
            // (the trip phases take a source whenever `current` is none, and
            // the 16/17/18 entry resets that). The re-route context x/z are
            // not touched.
            cs::FINISH_TRIP_MAP | cs::LOT_SWEEP => {
                self.st.finish_resume = None;
                self.st.extras.phase_positions = [0, 0];
            }
            _ => {}
        }
    }

    /// The scanning iterator's next cell; `None` at the end of the scan, or
    /// at the traffic shadow's stop ([`State::shadow_scan_stop`], tooling),
    /// which [`Self::scan_stopped`] tells apart.
    pub(crate) fn next_scan_cell(&mut self) -> Option<(i32, i32)> {
        if self.scan_stopped() {
            return None;
        }
        self.st.scan.next_cell()
    }

    /// The scan rests at the traffic shadow's stop (never without the
    /// shadow).
    pub(crate) fn scan_stopped(&self) -> bool {
        let it = &self.st.scan;
        it.state == crate::grid::ScanState::Running
            && self.st.shadow_scan_stop == Some((self.st.cycle_state, it.x, it.z))
    }

    /// Resets the scanning iterator to (0, 0), keeping its rectangle and step
    /// (§6.1 "The scanning iterator in row 14", TS31: every scanning state
    /// resets the iterator to (0, 0) on entry; TS64 gives the same for
    /// entering 13). The state is fresh at (0, 0), or done at once when
    /// (0, 0) is outside the rectangle (§7.4).
    pub(crate) fn reset_scan(&mut self) {
        self.st.scan.reset(0, 0);
        self.st.extras.scan_state_byte = None;
    }

    /// `UpdateSubnetworkInfo` (state 13, `0x00716180`; work cycle
    /// `WorkOnUpdateSubnetworkInfo` `0x0071DC50`, §4.12). [TS64] The deadline
    /// test is **before each step** of the scanning iterator
    /// (`0x00716212`), so a call can stop between two cells: the iterator then
    /// reads state 1 (running) at the last cell visited and the next call
    /// continues from it. After the last cell the neighbour connections are
    /// added and the result published in the same call, with no further
    /// deadline test, and the iterator reads state 2 at `(x0, z1 + 1)` (for a
    /// 128-cell city, (0, 128)), where it stays until the next scanning state
    /// resets it. DoPostCityInit (§1 step 7) runs the same work on a **local**
    /// iterator instead, so the member keeps its loaded value there
    /// ([`NetworkModel::update_subnetwork_info`]).
    fn update_subnetwork_info(&mut self) -> bool {
        while let Some((x, z)) = self.next_scan_cell() {
            self.cx.subnetwork_info_cell(x, z);
        }
        if self.scan_stopped() {
            return false;
        }
        self.cx.finish_subnetwork_info();
        true
    }

    // ---- §4.8 the small edge and switch phases ----

    /// `DumpEdgeDensities` (state 7): `last := current`, `usage := min(0xFFFF,
    /// Σ current)` for every record.
    fn dump_edge_densities(&mut self) {
        for key in self.cx.s.edge_density.keys() {
            if let Some(r) = self.cx.s.edge_density.get_mut(key) {
                r.last = r.current;
                let sum: u32 = r.current.iter().map(|&c| u32::from(c)).sum();
                r.usage = sum.min(0xFFFF) as u16;
            }
        }
    }

    /// `ReduceEdgeDensities` (state 6, `0x007110B0`, §4.8, TS22): `current
    /// := 0` for the 9 travel types and both evening values at every cell
    /// with network info or a transit switch (the scan), then at each ferry
    /// terminal's arrival cell, in list order (the departure cell has
    /// network info from the ferry phase of every rebuild). Missing records
    /// are not created.
    fn reduce_edge_densities(&mut self) -> bool {
        let mut cells = Vec::new();
        while let Some((x, z)) = self.next_scan_cell() {
            if !self.cx.s.info_at(x, z).is_empty() || self.cx.s.switch_id_at(x, z) != 0 {
                cells.push((x, z));
            }
        }
        let stopped = self.scan_stopped();
        if !stopped {
            for t in &self.cx.s.ferries {
                cells.push((i32::from(t.arr_x), i32::from(t.arr_z)));
            }
        }
        for (x, z) in cells {
            for evening in [false, true] {
                for t in 0..9u32 {
                    if let Some(r) = self.cx.s.edge_density.get_mut(NetworkState::edge_key(x, z, t, evening)) {
                        r.current = [0; 16];
                    }
                }
            }
        }
        !stopped
    }

    /// `ReduceTransitSwitches` (state 3, `0x00713750`, §4.8, TS3): for every
    /// switch in table order, `load := r24(densityReduction × load)` with the
    /// volume-reset factor `0x29136788`, nothing else. With the shipped 0 the
    /// load restarts from 0 (`0 × NaN` and `0 × ±∞` give NaN, a negative load
    /// becomes −0.0); a non-zero factor decays it. The two counters keep what
    /// CreateMaps wrote (§4.9 step 4).
    fn reduce_transit_switches(&mut self) {
        let f = f64::from(self.st.tuning.density_reduction);
        for id in self.cx.s.switches.keys() {
            if let Some(sw) = self.cx.s.switches.get_mut(id) {
                sw.load = r24(f * f64::from(sw.load)) as f32;
            }
        }
    }

    // ---- §3.1 messages that need both states ----

    /// The occupant messages (§3.1, traffic-network.md §3): the bookkeeping
    /// of §3.6 first (`0x00724334`–`0x0072442E`, TS4), then the network part.
    fn occupant_message(&mut self, object: u64, removed: bool) {
        let Some(occ) = self.h.as_network_occupant(object) else {
            // A failed cast still marks the occupants as changed.
            self.cx.s.occupants_changed = true;
            return;
        };
        let ins = !removed;
        let has = |t: u32| self.h.is_of_type(occ, t);
        let (avenue, road) = (has(AVENUE), has(ROAD));
        let road_like = ROAD_LIKE.into_iter().find(|&t| has(t));
        let rail = has(RAIL);
        let subway = has(SUBWAY) && !self.h.has_network_flag(occ, SUBWAY_SKIP_FLAG);
        if avenue {
            self.do_road_changed(occ, ins, AVENUE);
        }
        if road {
            self.do_road_changed(occ, ins, ROAD);
        }
        if let Some(t) = road_like {
            self.do_road_changed(occ, ins, t);
        }
        if rail {
            self.do_rail_changed(occ, ins);
        }
        if subway {
            self.do_subway_changed(ins);
        }
        self.cx.occupant_changed(occ, removed);
    }

    /// `bump(t)` (§3.6): `cells[t] += 1` on insert; on remove `cells[t] −= 1`
    /// only when it is above 0.
    fn bump(&mut self, t: u32, ins: bool) {
        let Some(c) = self.st.cells_per_type.get_mut(t as usize) else {
            return;
        };
        if ins {
            *c = c.wrapping_add(1);
        } else if *c > 0 {
            *c -= 1;
        }
    }

    /// `DoRoadChanged(occ, ins, t)` (`0x007155D0`, §3.6).
    fn do_road_changed(&mut self, occ: u64, ins: bool, t: u32) {
        let p = self.h.occupant_position(occ);
        let (x, z) = self.h.position_to_cell(p[0], p[2]);
        self.bump(t, ins);
        self.update_roads_budget();
        if HIGHWAYS.contains(&t) {
            return;
        }
        let repaired = {
            let mut d = self.dmg.borrow_mut();
            change_cell(&mut d.road, cell_key16(x, z), ins)
        };
        if repaired {
            self.repaired_cell(x, z);
        }
    }

    /// `DoRailChanged(occ, ins)` (`0x00715790`, §3.6): a level crossing (Road
    /// or Street) skips the rail sets.
    fn do_rail_changed(&mut self, occ: u64, ins: bool) {
        self.bump(RAIL, ins);
        if !self.h.has_any_network_flag(occ, LEVEL_CROSSING_MASK) {
            let (x, z) = self.h.network_occupant_cell(occ);
            let repaired = {
                let mut d = self.dmg.borrow_mut();
                change_cell(&mut d.rail, cell_key16(x, z), ins)
            };
            if repaired {
                // The road pothole count, also for rail *[quirk]*.
                self.repaired_cell(x, z);
            }
        }
        self.update_mass_transit_budget();
    }

    /// `DoSubwayChanged(ins)` (`0x0070EA70`, §3.6).
    fn do_subway_changed(&mut self, ins: bool) {
        self.bump(SUBWAY, ins);
        self.update_mass_transit_budget();
    }

    /// A damaged cell repaired by removal (§3.6): the published pothole
    /// count −= 1 (wrapping) and `0x69247DC8 (x, z, 0)` is posted.
    fn repaired_cell(&mut self, x: i32, z: i32) {
        let p = &mut self.st.stats_last.potholes;
        *p = p.wrapping_sub(1);
        self.h.post_message(super::SimMessage {
            msg_type: msg::CELL_DAMAGE,
            data1: x as u32,
            data2: z as u32,
            data3: 0,
        });
    }

    // ---- §3.5 DoPostCityInit ----

    /// DoPostCityInit step 5 (`0x00720660`–`0x007206B9`, traffic-network.md
    /// §1): every cell of the ring around the map (corners included) whose
    /// `HasMapExit` is false gets destination byte 0; `freeDest` is not
    /// changed.
    pub(crate) fn destination_boundary_pass(&mut self) {
        let r = self.cx.s.city_rect();
        for x in r.x1 - 1..=r.x2 + 1 {
            for z in r.z1 - 1..=r.z2 + 1 {
                let ring = x < r.x1 || x > r.x2 || z < r.z1 || z > r.z2;
                if ring && !self.cx.s.has_map_exit(x, z) {
                    self.st.set_dest_byte(x, z, 0);
                }
            }
        }
    }

    /// DoPostCityInit step 11: resume the interrupted phase, by the loaded
    /// cycle state.
    pub(crate) fn resume_loaded_phase(&mut self) {
        match self.st.cycle_state {
            cs::CONFIRM_TRIPS | cs::REROUTE_TRIPS | cs::FULL_TRIPS => {
                // The re-route context's search state := idle.
                for pf in [self.st.pf.as_mut(), self.st.pf2.as_mut()].into_iter().flatten() {
                    pf.reset();
                }
                self.st.continue_search = false;
                // The iterator advanced to the source at the saved cell (the
                // two s16 phase positions, TS44), which becomes the held
                // current source; none: reset again. x and z are compared
                // as 16-bit values (`0x00720C17`–`0x00720C25`).
                let want = self.st.extras.phase_positions;
                self.reset_sources();
                let mut taken = 0;
                let mut held = None;
                while let Some(src) = self.next_source() {
                    taken += 1;
                    let (x, z) = self.source_cell(src);
                    if [x as i16, z as i16] == want {
                        held = Some(src);
                        break;
                    }
                }
                match held {
                    Some(src) => {
                        self.hold_source(Some(src));
                        self.st.source_pos = taken - 1;
                    }
                    None => {
                        self.reset_sources();
                        self.st.source_pos = 0;
                    }
                }
                self.st.trip_pos = 0;
            }
            cs::FINISH_TRIP_MAP => {
                // The lot iterator reset, then past the lot at the saved
                // cell when there is one: the same two phase positions,
                // which FinishTripMap sets to each lot's location as it
                // takes it (TS45, `0x00720B2C`–`0x00720B46`); a lot the walk
                // never meets restarts it from the first lot.
                let [px, pz] = self.st.extras.phase_positions;
                self.st.finish_resume = self
                    .h
                    .lot_at(i32::from(px), i32::from(pz), false)
                    .filter(|l| self.h.lots().contains(l));
            }
            cs::FERRY_ROUTES => self.cx.post_city_init_ferries(),
            cs::LOT_SWEEP | 14 => {
                // Never saved by 1.1.641 (§3.2).
                for k in 0..4 {
                    self.st.set_free(k, 0);
                }
                self.reset_scan();
                self.clear_trip_destinations();
                self.reset_scan();
                self.refresh_commute_map();
                self.st.cycle_state = cs::START_RECALC;
            }
            _ => {}
        }
    }

    /// DoPostCityInit step 12: the bucket multimap rebuilt from the lots, in
    /// list order, each lot with `0x691C863C` under that value with its
    /// location (lot slot 21).
    pub(crate) fn rebuild_strategy_buckets(&mut self) {
        self.st.buckets.clear();
        for lot in self.h.lots() {
            let holder = self.h.lot_holder(lot);
            if holder == 0 {
                continue;
            }
            let Some(b) = self
                .h
                .holder_property(holder, prop::STRATEGY_BUCKET)
                .and_then(|v| v.items().first().map(|e| e.as_i64() as u32))
            else {
                continue;
            };
            let at = self.h.lot_location(lot);
            self.st.buckets.entry(b).or_default().push(at);
        }
    }

    /// `DoLotActivated` (§4.1.3; the network part is traffic-network.md
    /// §4.7.1). `lot` is the message's copy of the lot.
    fn lot_activated(&mut self, lot: u64) {
        let outcome = self.cx.lot_activated(lot);
        if !outcome.continues() {
            // No building, or the switch step returned (TN1): none of the
            // steps below, not even the draw.
            return;
        }
        let Some(building) = self.h.lot_building(lot) else {
            return;
        };
        let rect = self.h.lot_bounding_rect(lot);
        let real = self.h.lot_at(rect.x1, rect.z1, false);
        if let Some(real) = real {
            // Step 1: the strategy bucket.
            let b = self.st.strategy_bucket_counter;
            let n = self.st.tuning.strategy_buckets;
            self.st.strategy_bucket_counter = if n == 0 { 0 } else { b.wrapping_add(1) % n };
            let holder = self.h.lot_holder(real);
            if holder != 0 {
                self.h
                    .set_holder_property(holder, prop::STRATEGY_BUCKET, Variant::Scalar(Elem::U32(b)));
            }
            // The lot cell: its location (lot slot 21), as the deactivation
            // and the load rebuild use it (§4.1.3, §3.5 step 12).
            let at = self.h.lot_location(real);
            self.st.buckets.entry(b).or_default().push(at);
            // Step 2: the draw.
            let v = self.st.rng.uniform_u32(100);
            if holder != 0 {
                self.h
                    .set_holder_property(holder, prop::STRATEGY_DRAW, Variant::Scalar(Elem::U8(v as u8)));
            }
        }
        // Step 3: the budget items.
        let e = self.h.activate_budget_items(building, real);
        self.st.budget_total = self.st.budget_total.wrapping_add(e);
        // Step 4: the destination bits, stored over the lot rectangle.
        if self.st.cycle_state != cs::REFRESH_COMMUTE {
            let holder = self.h.lot_holder(lot);
            let mut bits = 0u8;
            for k in 0..4usize {
                if capacity_element(self.h, holder, k) > 0 {
                    bits |= 1 << k;
                    self.st.set_free(k, self.st.free(k).wrapping_add(1));
                }
            }
            for x in rect.x1..=rect.x2 {
                for z in rect.z1..=rect.z2 {
                    self.st.set_dest_byte(x, z, bits);
                }
            }
        }
    }

    /// `DoLotDeactivated(lot)` (`0x0071E2F0`, §4.1.3, TS23). The network
    /// part (the switch, step 2, and the ferry terminal, step 5) runs as one
    /// call; none of the other steps reads what it changes.
    fn lot_deactivated(&mut self, lot: u64) {
        let holder = self.h.lot_holder(lot);
        let rect = self.h.lot_bounding_rect(lot);
        // 1. The one bucket entry under the lot's `0x691C863C` at its
        // location.
        if holder != 0 {
            if let Some(b) = self
                .h
                .holder_property(holder, prop::STRATEGY_BUCKET)
                .and_then(|v| v.items().first().map(|e| e.as_i64() as u32))
            {
                let at = self.h.lot_location(lot);
                if let Some(cells) = self.st.buckets.get_mut(&b) {
                    if let Some(i) = cells.iter().position(|&c| c == at) {
                        cells.remove(i);
                    }
                }
            }
        }
        // 2, 5.
        self.cx.lot_deactivated(lot);
        // 3. The budget items: `E −= cost` of the `0xCA550301` item.
        let cost = self.h.deactivate_budget_items(holder);
        self.st.budget_total = self.st.budget_total.wrapping_sub(cost);
        // 4. Unless in state 10: `freeDest[k] −= 1` per bit set at the NW
        // cell.
        if self.st.cycle_state != cs::REFRESH_COMMUTE {
            let b = self.st.dest_byte(rect.x1, rect.z1);
            for k in 0..4 {
                if b & (1 << k) != 0 {
                    self.st.set_free(k, self.st.free(k).wrapping_sub(1));
                }
            }
        }
        // 6. The bytes over the rect, bit 3 included.
        for x in rect.x1..=rect.x2 {
            for z in rect.z1..=rect.z2 {
                self.st.set_dest_byte(x, z, 0);
            }
        }
    }
}

/// Steps 5–6 of DoRoadChanged on one damage block (§3.6). Insert: the key
/// joins the intact set unless already there (the damaged set is not looked
/// at), and `cellCount += 1` in every case. Remove: the key leaves the
/// intact set, or else the damaged set (`damagedCount −= 1`; the result is
/// true); `cellCount −= 1` in every case (u32, wrapping *[compat]*).
fn change_cell(b: &mut DamageBlock, key: u16, ins: bool) -> bool {
    let k = u32::from(key);
    if ins {
        b.intact.insert(k);
        b.cell_count = b.cell_count.wrapping_add(1);
        return false;
    }
    let mut repaired = false;
    if !b.intact.remove(k) && b.damaged.remove(k) {
        b.damaged_count = b.damaged_count.wrapping_sub(1);
        repaired = true;
    }
    b.cell_count = b.cell_count.wrapping_sub(1);
    repaired
}

impl TrafficSimulator {
    // ---- slots 3, 4 ----

    /// `Init` (slot 3, §1), only when not yet initialised.
    pub fn init(&self) -> bool {
        if self.read_state(|s| s.initialised) {
            return true;
        }
        // Step 1: the tuning.
        self.read_tunable_values();
        let h = &*self.host;
        // Steps 2, 3.
        for level in AGENT_LEVELS {
            h.add_agent(level, AGENT_NAME, AGENT_FLAGS);
        }
        for m in msg::SUBSCRIPTIONS {
            h.subscribe(m);
        }
        // Step 4: the grids and the destination map. A grid, the
        // destination map and the scanning iterator a committed `Read`
        // brought are kept (`Read` runs before `Init` on a load, foundations
        // §3.7): the spec creates them unconditionally, which would drop the
        // saved maps of §6.2 *[choice, question]*.
        let (w, hgt) = h.cell_counts();
        self.with_state(|h, s| {
            let tract = s.tuning.tract_size;
            let g8 = |slot: &mut Option<GridU8>, id: u32, t: i32| {
                if slot.is_none() {
                    *slot = GridProvider::<u8>::create_grid(h, id, t);
                }
            };
            let gf = |slot: &mut Option<GridF32>, id: u32, t: i32| {
                if slot.is_none() {
                    *slot = GridProvider::<f32>::create_grid(h, id, t);
                }
            };
            g8(&mut s.air, grid_id::AIR_POLLUTING, tract);
            gf(&mut s.peak_volume, grid_id::PEAK_VOLUME, tract);
            g8(&mut s.commercial, grid_id::COMMERCIAL, 4);
            g8(&mut s.commercial_work, grid_id::COMMERCIAL_WORK, 4);
            g8(&mut s.unused, grid_id::UNUSED, 1);
            g8(&mut s.trip_length, grid_id::TRIP_LENGTH, 1);
            s.volume.resize(6, None);
            for (n, slot) in s.volume.iter_mut().enumerate() {
                gf(slot, grid_id::VOLUME + n as u32, 1);
            }
            gf(&mut s.float_grid, grid_id::FLOAT, 1);
            g8(&mut s.congestion, grid_id::CONGESTION, 1);
            gf(&mut s.peak_ratio, grid_id::PEAK_RATIO, 1);
            let keep_dest = s.loaded && s.dest_rows == w.max(0) + 2 && s.dest_cols == hgt.max(0) + 2;
            if !keep_dest {
                s.size_dest(w, hgt);
            }
            // Step 5: the scanning iterator over the city rect.
            if !s.loaded {
                s.scan = ScanIterator::new(0, 0, w - 1, hgt - 1);
                s.extras.scan_state_byte = None;
            }
        });
        // Step 5: the network model (cell connection array, switch grids,
        // water pathfinder), outside any operation.
        self.net.init();
        let (roads_max, transit_max, loaded_roads, loaded_transit) = self.read_state(|s| {
            (
                s.tuning.max_roads_funding,
                s.tuning.max_transit_funding,
                s.roads,
                s.mass_transit,
            )
        });
        // A department a `Read` resolved is kept, with its saved maximum
        // funding; only a null member is created, with its name and line
        // items 1-3 (§9.1 TG7).
        let create = |id: u32, max: f32, lines: &[dept::LineItem; 3]| {
            let d = h.create_department(id, max)?;
            for &item in lines {
                h.create_line_item(d, item, false);
            }
            Some(d)
        };
        let roads = loaded_roads.or_else(|| create(dept::ROADS, roads_max, &dept::ROADS_LINES));
        let transit = loaded_transit.or_else(|| create(dept::MASS_TRANSIT, transit_max, &dept::MASS_TRANSIT_LINES));
        // Then, on every Init, lines 4-6 of both when missing, in the
        // original's order (§9.1 TG7).
        for (mass_transit, item) in dept::INIT_LINES {
            if let Some(d) = if mass_transit { transit } else { roads } {
                h.create_line_item(d, item, true);
            }
        }
        self.update_state(|s| {
            s.roads = roads;
            s.mass_transit = transit;
        });
        // Step 7 (here, before the pathfinders): the default pool,
        // `SetMaxTripCapacity(default, 3, 0xFFFFFFFF)` (traffic-network.md
        // §2.7.3). The pools by ID a committed `Read` brought are kept (row
        // 19 of §6.1; `Read` runs before `Init`) *[choice]*: only the
        // default pool is Init's.
        self.pools.borrow_mut().default = Pools::default().default;
        set_capacity_element(&self.holders(), pool_token(0), 3, u32::MAX);
        // Step 6: the two pathfinders.
        self.run(|r| {
            let pf = r.create_path_finder();
            let pf2 = r.create_path_finder();
            r.st.pf = Some(pf);
            r.st.pf2 = Some(pf2);
        });
        // Step 7: the accident timer starts with its next check at 0, so the
        // first month checks *[choice]*.
        self.update_state(|s| s.initialised = true);
        true
    }

    /// `Shutdown` (slot 4): releases the pathfinders, grids and departments.
    pub fn shutdown(&self) -> bool {
        self.run(|r| r.hold_source(None));
        let depts = self.update_state(|s| {
            s.pf = None;
            s.pf2 = None;
            s.air = None;
            s.peak_volume = None;
            s.commercial = None;
            s.commercial_work = None;
            s.unused = None;
            s.trip_length = None;
            s.volume = vec![None; 6];
            s.float_grid = None;
            s.congestion = None;
            s.peak_ratio = None;
            s.initialised = false;
            s.loaded = false;
            (vec![s.roads.take(), s.mass_transit.take()], std::mem::take(&mut s.iter))
        });
        let (depts, iter) = depts;
        // The source iterator's connection references (§9.1 TG19) *[choice]*:
        // given back here, before the connections go with the city.
        for c in iter.conns {
            NetworkHost::release(&*self.host, c);
        }
        // The network's two switch grids go with the others *[choice]*.
        self.net.update_state(|n| {
            n.switch_mode = None;
            n.switch_id = None;
        });
        for d in depts.into_iter().flatten() {
            self.host.release_department(d);
        }
        true
    }

    /// `ReadSimulatorTunableValues` (§4.14) and the network's copies (the
    /// capacity tuning, the inverse speed table, the congestion curve), then
    /// `UpdateTravelStrategies`.
    pub fn read_tunable_values(&self) {
        let ex = self.host.tuning_exemplar();
        let cw = self.host.cell_width();
        let (net_tuning, inv, curve) = self.update_state(|s| {
            s.tuning.read(ex.as_deref());
            (
                s.tuning.network.clone(),
                inverse_max_speed(&s.tuning.max_speed, cw),
                s.tuning.congestion_curve.clone(),
            )
        });
        self.net.update_state(|n| {
            n.tuning = net_tuning;
            n.inv_max_speed = inv;
            n.congestion_curve = curve;
        });
        self.update_travel_strategies();
    }

    /// `UpdateTravelStrategies` (§4.1.2).
    pub fn update_travel_strategies(&self) {
        let ex = self.host.tuning_exemplar();
        let offsets: Vec<Vec<i64>> = self
            .host
            .ordinances_on()
            .into_iter()
            .map(|id| {
                self.host
                    .ordinance_property(id, prop::ORDINANCE_STRATEGY)
                    .map(|v| v.items().iter().map(|e| e.as_i64()).collect())
                    .unwrap_or_default()
            })
            .collect();
        self.update_state(|s| {
            if let Some(ex) = ex.as_deref() {
                s.tuning.read_strategies(ex);
            }
            for o in &offsets {
                s.tuning.add_ordinance_offsets(o);
            }
        });
    }

    /// The transit-switch cost multiplier from the mass-transit funding
    /// (§3.1): `curve 0xCA5F7821(r24(f × 100))`. Without the department
    /// nothing changes *[choice]*.
    fn update_transit_multiplier(&self) {
        let Some(d) = self.read_state(|s| s.mass_transit) else {
            return;
        };
        let f = self.host.department_funding(d);
        let m = self.read_state(|s| s.tuning.switch_curve.eval(r24(f64::from(f) * 100.0) as f32));
        self.net.update_state(|n| n.transit_multiplier = m);
    }

    // ---- §3.1 DoMessage ----

    /// `DoMessage` (§3.1). Every handler returns true except the day agent's,
    /// which returns `DoDailySimulation`'s result.
    pub fn do_message(&self, m: &Message) -> bool {
        match m.msg_type {
            msg::SIM_BEGIN => {
                // The network-changed byte := 1 (TS5): saved and loaded, read
                // by nothing. No rebuild follows (DoPostCityInit does it).
                self.update_state(|s| s.extras.network_changed_byte = true);
            }
            msg::DAY => return self.do_daily_simulation(),
            msg::MONTH => self.do_monthly_simulation(),
            msg::POST_CITY_INIT => self.do_post_city_init(),
            msg::BUDGET_CHANGED => {
                if m.data2 == dept::MASS_TRANSIT {
                    self.update_transit_multiplier();
                }
            }
            // traffic-network.md §2.7.2 (TS6): data1 type, data2 x, data3 z.
            // Neither touches the network-changed byte.
            msg::NEIGHBOUR_ADDED => {
                self.run(|r| r.add_neighbor_connection(m.data1, m.data2 as i32, m.data3 as i32));
            }
            msg::NEIGHBOUR_REMOVED => {
                self.run(|r| r.remove_neighbor_connection(m.data1, m.data2 as i32, m.data3 as i32));
            }
            msg::TUNING_RELOAD => {
                if m.data1 == msg::TUNING_GROUP && m.data2 == msg::TUNING_INSTANCE {
                    self.read_tunable_values();
                }
            }
            msg::ORDINANCE_UPDATED => self.update_travel_strategies(),
            msg::OCCUPANT_INSERTED | msg::OCCUPANT_REMOVED => {
                if m.data3 == NETWORK_OCCUPANT_TYPE {
                    let removed = m.msg_type == msg::OCCUPANT_REMOVED;
                    self.run(|r| r.occupant_message(m.object, removed));
                    // Any such message, even a failed cast, sets the
                    // network-changed byte (§3.1, TS5).
                    self.update_state(|s| s.extras.network_changed_byte = true);
                }
            }
            msg::LOT_STATE_CHANGED => {
                if let Some(lot) = self.host.as_lot(m.object) {
                    if m.data1 == 3 {
                        self.run(|r| r.lot_deactivated(lot));
                    } else if m.data2 == 3 {
                        self.run(|r| r.lot_activated(lot));
                    }
                }
            }
            _ => {}
        }
        true
    }

    /// `DoPostCityInit` (`0x00720430`, §3.5, TS7), in order.
    pub fn do_post_city_init(&self) {
        self.run(|r| {
            // 1–3 (traffic-network.md §1).
            r.cx.post_city_init_rebuild();
            // 4. An Add per neighbour connection, in list order, then one
            // pool sweep (traffic-network.md §2.7.2–§2.7.3).
            for conn in r.h.all_connections() {
                let (x, z) = r.h.connection_cell(conn);
                r.add_neighbor_connection(r.h.connection_type(conn), x, z);
            }
            r.pool_sweep();
            // 5. The destination-map boundary pass.
            r.destination_boundary_pass();
            // 6, 7, and the water pathfinder's own handler of the message.
            r.cx.post_city_init_subnetworks();
            r.cx.water_post_city_init();
            // 8. The subnetwork work was just done in full.
            if matches!(r.st.cycle_state, cs::UPDATE_SUBNETWORKS | cs::SUBNETWORK_INFO) {
                r.st.cycle_state = cs::FINAL_CALCULATIONS;
            }
        });
        // 9.
        self.update_transit_multiplier();
        self.run(|r| {
            // 10. The cell-count rebuild never runs in 1.1.641 *[compat]*.
            // 11.
            r.resume_loaded_phase();
            // 12.
            r.rebuild_strategy_buckets();
            // 13. No `max(1, …)`: 0 with fewer than 18 lots.
            r.st.days = (r.h.lots().len() as u32) / 18;
            // 14.
            r.update_mass_transit_budget();
            r.update_roads_budget();
        });
    }

    /// `DoDailySimulation` (§3.3): the day agent.
    pub fn do_daily_simulation(&self) -> bool {
        self.run(|r| r.do_daily_simulation())
    }

    /// One `WorkOnSimCycle` call (§3.2): whether the phase finished.
    pub fn work_on_sim_cycle(&self) -> bool {
        self.run(|r| r.work_on_sim_cycle())
    }

    /// `ApplyRoute` on the simulator (§4.5), for hosts and tests.
    #[allow(clippy::too_many_arguments)]
    pub fn apply_route(&self, route: &[u8], count: u32, source: CellRect, dest: (i32, i32), mode: u8, trip_type: u8) {
        self.run(|r| r.apply_route(route, count, source, dest, mode, trip_type));
    }

    /// `ConfirmRoute(route)` on the simulator (§4.3), with its strike state.
    pub fn confirm_route(&self, route: &[u8]) -> bool {
        self.run(|r| super::route::confirm_route(r.net(), r.st.strike_state, route))
    }
}
