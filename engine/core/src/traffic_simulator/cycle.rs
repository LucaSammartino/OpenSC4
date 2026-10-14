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
use crate::traffic_network::{inverse_max_speed, Cx, NetworkState};
use crate::variant::{Elem, Variant};
use crate::x87::{ftol, r24};

use super::adapters::{capacity_element, PfAdapter};
use super::state::{DamageState, State, PASS_COUNTERS, PC_REROUTE_BUCKET};
use super::{
    cell_key16, cycle_state as cs, dept, grid_id, msg, prop, Message, Source, TrafficSimHost, TrafficSimulator,
    AGENT_FLAGS, AGENT_LEVELS, AGENT_NAME,
};

/// The network-occupant type of the occupant messages (§3.1).
const NETWORK_OCCUPANT_TYPE: u32 = crate::traffic_network::occupant_type::NETWORK;

/// The network types whose occupants change the road damage set, and the
/// rail one (traffic-network.md §3: DoRoadChanged for 6, 0, 10, 3, 2, 12;
/// DoRailChanged for 1).
const ROAD_TYPES: [u32; 6] = [6, 0, 10, 3, 2, 12];
const RAIL_TYPE: u32 = 1;

/// The context of every operation that touches both the simulator state and
/// the network state (see the module documentation of `mod.rs`).
pub(crate) struct Run<'r, 'c> {
    pub(crate) h: &'r dyn TrafficSimHost,
    pub(crate) st: &'r mut State,
    pub(crate) cx: &'r mut Cx<'c>,
    pub(crate) dmg: &'r RefCell<DamageState>,
}

impl Run<'_, '_> {
    // ---- pathfinders ----

    /// `CreatePathFinder` (slot 6): a pathfinder initialised against the
    /// simulator (pathfinder.md §1).
    pub(crate) fn create_path_finder(&mut self) -> Pathfinder {
        let host = PfAdapter {
            h: self.h,
            st: &*self.st,
        };
        Pathfinder::create(&host, self.st.shared.clone())
    }

    /// `FindPath` of a pathfinder over the simulator's network and state.
    pub(crate) fn find_path_with(&mut self, pf: &mut Pathfinder) -> u32 {
        let host = PfAdapter {
            h: self.h,
            st: &*self.st,
        };
        pf.find_path(&mut PathCx {
            net: &mut *self.cx.s,
            host: &host,
        })
    }

    pub(crate) fn net(&self) -> &NetworkState {
        self.cx.s
    }

    // ---- §3.3 DoDailySimulation ----

    /// `DoDailySimulation` (§3.3) with the work budget of §3.4.
    pub(crate) fn do_daily_simulation(&mut self) -> bool {
        // Step 2: a new day of month.
        let day = self.h.sim_day_of_month() as i32;
        if day != self.st.last_day {
            self.st.last_day = day;
            if day == 1 {
                self.pay_monthly_income();
            }
            self.strategy_redraw();
        }
        // Step 3.
        loop {
            let finished = self.work_on_sim_cycle();
            let s = self.st.cycle_state;
            if finished {
                match s {
                    cs::CONFIRM_TRIPS => {
                        self.start_pacing(self.st.pass_total, 20);
                        continue;
                    }
                    cs::REROUTE_TRIPS => {
                        self.start_pacing(self.st.to_reroute, 18);
                        return true;
                    }
                    cs::FULL_TRIPS => {
                        self.start_pacing(self.st.to_search, 18);
                        self.st.pass_total = 0;
                        return true;
                    }
                    cs::CREATE_MAPS | cs::UPDATE_NEIGHBOURS => return true,
                    _ => continue,
                }
            }
            if (cs::CONFIRM_TRIPS..=cs::FULL_TRIPS).contains(&s) {
                let share =
                    (self.st.total as u32).wrapping_mul(self.st.day_index.wrapping_add(1)) / self.st.days.max(1);
                if self.st.processed as u32 >= share {
                    self.st.day_index = self.st.day_index.wrapping_add(1);
                    return true;
                }
            }
            // Not done for today: the simulator calls again within the day.
            return false;
        }
    }

    /// The pacing reset of §3.3: `total`, `days := max(1, total div k)`, day
    /// index and processed 0, the pass counters 0. *[question]* `rerouteBucket`
    /// is pass counter 2 (§6.1 row 15) and §4.3 needs it to survive the
    /// cycle, so it is kept; the other nine are zeroed.
    fn start_pacing(&mut self, total: i32, k: i32) {
        self.st.total = total;
        self.st.days = (total / k).max(1) as u32;
        self.st.day_index = 0;
        self.st.processed = 0;
        self.reset_pass_counters();
    }

    fn reset_pass_counters(&mut self) {
        for i in 0..PASS_COUNTERS {
            if i != PC_REROUTE_BUCKET {
                self.st.pass_counters[i] = 0;
            }
        }
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
                    // End of FullTrips: the counts built in this pass are
                    // published (§4.4).
                    self.st.type_count_published = self.st.type_count_building;
                }
                done
            }
            cs::FINISH_TRIP_MAP => {
                self.finish_trip_map();
                true
            }
            cs::CREATE_MAPS => {
                self.create_maps();
                true
            }
            cs::CREATE_FINAL_MAP => {
                self.create_final_map();
                true
            }
            cs::CLEAR_DESTINATIONS => {
                self.clear_trip_destinations();
                true
            }
            // UpdateNeighborConnections (`0x00714BE0`): traffic-network.md
            // does not specify it *[question]*; nothing to do.
            cs::UPDATE_NEIGHBOURS => true,
            cs::REDUCE_SWITCHES => {
                self.reduce_transit_switches();
                true
            }
            cs::DUMP_EDGES => {
                self.dump_edge_densities();
                true
            }
            cs::REDUCE_EDGES => {
                self.reduce_edge_densities();
                true
            }
            cs::REFRESH_COMMUTE => {
                self.refresh_commute_map();
                true
            }
            cs::UPDATE_SUBNETWORKS => {
                // traffic-network.md §4.11, in one call; the stale flag is
                // consumed here *[choice]*.
                self.cx.s.clear_subnetworks();
                self.cx.s.update_subnetworks();
                self.cx.s.subnetworks_stale = false;
                true
            }
            cs::SUBNETWORK_INFO => self.cx.work_on_subnetwork_info(usize::MAX),
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
    fn enter_state(&mut self, s: u8) {
        match s {
            cs::CONFIRM_TRIPS | cs::REROUTE_TRIPS | cs::FULL_TRIPS => {
                // The traffic-source iterator (§4.1): lots, then neighbour
                // connections, as listed now *[choice: a snapshot]*.
                let mut sources: Vec<Source> = self.h.lots().into_iter().map(Source::Lot).collect();
                sources.extend(self.h.all_connections().into_iter().map(Source::Connection));
                self.st.sources = sources;
                self.st.source_pos = 0;
                self.st.trip_pos = 0;
                self.st.continue_search = false;
                self.reset_pass_counters();
                if s == cs::CONFIRM_TRIPS {
                    // The counts ConfirmTrips builds *[choice: zeroed here]*.
                    self.st.to_reroute = 0;
                    self.st.to_search = 0;
                }
                if s == cs::FULL_TRIPS {
                    self.st.type_count_building = [0; 9];
                }
            }
            cs::CREATE_MAPS => {
                self.reset_scan();
                let w = &mut self.st.stats_work;
                w.rows = Default::default();
                w.aggregate = Default::default();
                w.over = [0; 13];
                w.aggregate_over = 0;
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
            cs::SUBNETWORK_INFO => self.cx.begin_subnetwork_info(),
            cs::FERRY_ROUTES => {
                let riders = self.cx.begin_ferry_pass();
                self.st.ferry_total = self.st.ferry_total.wrapping_add(riders);
            }
            _ => {}
        }
    }

    /// Resets the scanning iterator to the first cell of the city.
    pub(crate) fn reset_scan(&mut self) {
        let (x0, z0) = (self.st.scan.x0, self.st.scan.z0);
        self.st.scan.reset(x0, z0);
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

    /// `ReduceEdgeDensities` (state 6): `current := 0` for the 9 travel types
    /// and both evening values at every cell with network info or a transit
    /// switch, and at the ferry terminals' departure and arrival cells
    /// *[choice: the terminal cells]*. Missing records are not created.
    fn reduce_edge_densities(&mut self) {
        let mut cells = Vec::new();
        while let Some((x, z)) = self.st.scan.next_cell() {
            if !self.cx.s.info_at(x, z).is_empty() || self.cx.s.switch_id_at(x, z) != 0 {
                cells.push((x, z));
            }
        }
        for t in &self.cx.s.ferries {
            cells.push((i32::from(t.dep_x), i32::from(t.dep_z)));
            cells.push((i32::from(t.arr_x), i32::from(t.arr_z)));
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
    }

    /// `ReduceTransitSwitches` (state 3): rolls the usage for the next cycle
    /// *[question: traffic-network.md does not specify it]*: the load
    /// restarts from 0 (CreateMaps keeps the previous one in counter 1).
    fn reduce_transit_switches(&mut self) {
        for id in self.cx.s.switches.keys() {
            if let Some(sw) = self.cx.s.switches.get_mut(id) {
                sw.load = 0.0;
            }
        }
    }

    // ---- §3.1 messages that need both states ----

    /// The occupant messages (§3.1, traffic-network.md §3).
    fn occupant_message(&mut self, object: u64, removed: bool) {
        let Some(occ) = self.h.as_network_occupant(object) else {
            // A failed cast still marks the occupants as changed.
            self.cx.s.occupants_changed = true;
            return;
        };
        // The simulator's bookkeeping first (DoRoadChanged / DoRailChanged):
        // FreeCity4 keeps the damage blocks' intact sets
        // *[question: the functions are not specified]*.
        let road = ROAD_TYPES.iter().any(|&t| self.h.is_of_type(occ, t));
        let rail = self.h.is_of_type(occ, RAIL_TYPE);
        if road || rail {
            let p = self.h.occupant_position(occ);
            let (x, z) = self.h.position_to_cell(p[0], p[2]);
            let key = cell_key16(x, z);
            let mut guard = self.dmg.borrow_mut();
            let d = &mut *guard;
            for (hit, block) in [(road, &mut d.road), (rail, &mut d.rail)] {
                if hit {
                    if removed {
                        block.remove_cell(key);
                    } else {
                        block.insert_cell(key);
                    }
                }
            }
        }
        self.cx.occupant_changed(occ, removed);
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
            // The lot cell: the rectangle's corner *[choice]*.
            self.st.buckets.entry(b).or_default().push((rect.x1, rect.z1));
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

    /// `DoLotDeactivated` (§4.1.3 *[inferred]*): the network part, then the
    /// destination bytes over the lot rectangle := 0 and the lot's bucket
    /// entries removed *[choice: not traced; `freeDest` is not changed]*.
    fn lot_deactivated(&mut self, lot: u64) {
        self.cx.lot_deactivated(lot);
        let rect = self.h.lot_bounding_rect(lot);
        for x in rect.x1..=rect.x2 {
            for z in rect.z1..=rect.z2 {
                self.st.set_dest_byte(x, z, 0);
            }
        }
        for cells in self.st.buckets.values_mut() {
            cells.retain(|&c| c != (rect.x1, rect.z1));
        }
    }
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
        // Step 4: the grids and the destination map.
        let (w, hgt) = h.cell_counts();
        self.with_state(|h, s| {
            let tract = s.tuning.tract_size;
            let g8 = |id: u32, t: i32| GridProvider::<u8>::create_grid(h, id, t);
            let gf = |id: u32, t: i32| GridProvider::<f32>::create_grid(h, id, t);
            s.air = g8(grid_id::AIR_POLLUTING, tract);
            s.peak_volume = gf(grid_id::PEAK_VOLUME, tract);
            s.commercial = g8(grid_id::COMMERCIAL, 4);
            s.commercial_work = g8(grid_id::COMMERCIAL_WORK, 4);
            s.unused = g8(grid_id::UNUSED, 1);
            s.trip_length = g8(grid_id::TRIP_LENGTH, 1);
            s.volume = (0..6).map(|n| gf(grid_id::VOLUME + n, 1)).collect();
            s.float_grid = gf(grid_id::FLOAT, 1);
            s.congestion = g8(grid_id::CONGESTION, 1);
            s.peak_ratio = gf(grid_id::PEAK_RATIO, 1);
            s.size_dest(w, hgt);
            // Step 5: the scanning iterator over the city rect.
            s.scan = ScanIterator::new(0, 0, w - 1, hgt - 1);
        });
        // Step 5: the network model (cell connection array, switch grids,
        // water pathfinder), outside any operation.
        self.net.init();
        let (roads_max, transit_max) = self.read_state(|s| (s.tuning.max_roads_funding, s.tuning.max_transit_funding));
        let roads = h.create_department(dept::ROADS, roads_max);
        let transit = h.create_department(dept::MASS_TRANSIT, transit_max);
        self.update_state(|s| {
            s.roads = roads;
            s.mass_transit = transit;
        });
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
            vec![s.roads.take(), s.mass_transit.take()]
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
                // "Marks the network as changed" *[question: which flag, and
                // who rebuilds]*: the flag the trip phases consume.
                self.net.update_state(|n| n.network_changed = true);
            }
            msg::DAY => return self.do_daily_simulation(),
            msg::MONTH => self.do_monthly_simulation(),
            msg::POST_CITY_INIT => self.do_post_city_init(),
            msg::BUDGET_CHANGED => {
                if m.data2 == dept::MASS_TRANSIT {
                    self.update_transit_multiplier();
                }
            }
            // Adding / removing a neighbour connection: neither spec says
            // what the network part does *[question]*.
            msg::NEIGHBOUR_ADDED | msg::NEIGHBOUR_REMOVED => {}
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

    /// `DoPostCityInit` (`0x00720430`, traffic-network.md §1): the network
    /// rebuild; steps 4 (the neighbour connections) and 5 (the destination
    /// map boundary pass) are not specified *[question]*; then the transit
    /// multiplier.
    pub fn do_post_city_init(&self) {
        self.run(|r| {
            r.cx.post_city_init_rebuild();
            r.cx.post_city_init_finish();
        });
        self.update_transit_multiplier();
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
}
