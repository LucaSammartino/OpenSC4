// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Trip generation (§4.1.1, §4.1.4), the three trip phases ConfirmTrips,
//! ReRouteTrips and FullTrips (§4.3, §4.4), `SetupTrip`,
//! `PerformPathSearch`, `CompleteTripSourceCalculations` (§4.6) and
//! FinishTripMap (§4.7).

use crate::grid::CellRect;
use crate::pathfinder::{filter, flag, result, Pathfinder, NO_PARK_AND_RIDE};
use crate::variant::{Elem, Variant};
use crate::x87::{r24, rhe};

use crate::traffic_network::{NetworkHost, NetworkState};

use super::cycle::Run;
use super::route::{confirm_route, mode};
use super::{prop, status, trip_flag, Source, TripData, FREIGHT};

/// The neighbour-connection types of a divided road (§4.4, TS17): Highway
/// (3), Avenue (8) and GroundHighway (12) (traffic-network.md §2.7.1).
pub const DIVIDED_ROAD_MASK: u16 = 0x1108;

/// `FindHighwayReturnTile(x, z, back)` (`0x0070DD30`, §4.4, TS17): moves an
/// off-map cell one step along its map edge, so a car or bus that left by a
/// divided road comes back on the other carriageway. `right` is the
/// language's driving side, inverted when `back`; then the first matching
/// row: `x < 0` → `z ± 1`, `x > X − 1` → `z ∓ 1`, `z < 0` → `x ∓ 1`,
/// `z > Z − 1` → `x ± 1` (upper signs when `right`). An in-city cell is
/// unchanged; the new cell is not checked.
pub fn find_highway_return_tile(x: i32, z: i32, back: bool, drives_on_right: bool, size: (i32, i32)) -> (i32, i32) {
    let right = drives_on_right != back;
    let d = if right { 1 } else { -1 };
    if x < 0 {
        (x, z + d)
    } else if x > size.0 - 1 {
        (x, z - d)
    } else if z < 0 {
        (x - d, z)
    } else if z > size.1 - 1 {
        (x + d, z)
    } else {
        (x, z)
    }
}

/// The travel types a commuter may use (§4.4 step 5): walk, car, bus,
/// passenger train, subway, el-train, monorail.
pub const COMMUTER_TYPES: u16 = 0x1CF;
/// Walk or car: the start types of a commuter without a forced start.
pub const COMMUTER_START: u16 = 0x3;
/// Truck and freight train (§4.4 step 6).
pub const FREIGHT_TYPES: u16 = 0x30;

/// The developer types of the residential travel population, by wealth
/// (§4.1.1).
const RES_DEVELOPERS: [u32; 3] = [0x1010, 0x1020, 0x1030];
/// The industrial developer types, in the order they are summed (§4.1.1).
const IND_DEVELOPERS: [u32; 4] = [0x4400, 0x4300, 0x4200, 0x4100];

/// `float(u32)`: the value rounded to f32.
fn f_u32(v: u32) -> f64 {
    r24(f64::from(v))
}

/// The floor idiom of §4.1.1: `r := round(r24(n − 0.5))`, then `r + 1` if
/// `float(r + 1) = n` (u32 arithmetic) *[compat]*.
pub fn floor_idiom(n: f32) -> u32 {
    let r = rhe(r24(f64::from(n) - 0.5) as f32) as u32;
    if f_u32(r.wrapping_add(1)) == f64::from(n) {
        r.wrapping_add(1)
    } else {
        r
    }
}

/// The ceil idiom of §4.1.1: `r := round(r24(n + 0.5))`, then `r − 1` if
/// `float(r − 1) = n`.
pub fn ceil_idiom(n: f32) -> u32 {
    let r = rhe(r24(f64::from(n) + 0.5) as f32) as u32;
    if f_u32(r.wrapping_sub(1)) == f64::from(n) {
        r.wrapping_sub(1)
    } else {
        r
    }
}

/// The residential commuters of a lot (§4.1.1): `max(1, floor(r24(pop ×
/// scale)))`.
pub fn residential_commuters(pop: u32, scale: f32) -> u32 {
    let n = r24(f_u32(pop) * f64::from(scale)) as f32;
    floor_idiom(n).max(1)
}

/// The split of a lot's commuters over the three strategies (§4.1.1, tests
/// 7, 8): below 50 all go to the first strategy whose threshold exceeds the
/// lot's strategy byte (none when the byte is above every threshold
/// *[compat]*); otherwise `round(r24(float(commuters) × pct[s])) & 0xFFFF`.
pub fn residential_split(commuters: u32, pct: [f32; 3], strategy_byte: u8) -> [u16; 3] {
    let mut w = [0u16; 3];
    if commuters < 50 {
        let mut c: u32 = 1;
        for (s, &p) in pct.iter().enumerate() {
            let t = rhe(r24(f64::from(p) * 100.0) as f32) as u32 & 0xFF;
            c = c.wrapping_add(t);
            if u32::from(strategy_byte) < c {
                w[s] = commuters as u16;
                break;
            }
        }
    } else {
        for (s, &p) in pct.iter().enumerate() {
            let v = rhe(r24(f_u32(commuters) * f64::from(p)) as f32) as u32;
            w[s] = (v & 0xFFFF) as u16;
        }
    }
    w
}

/// The freight trips of an industrial lot (§4.1.1): `ceil(r24(freightScaling
/// × float(jobs)))`, kept to 16 bits.
pub fn industrial_trips(jobs: u32, freight_scaling: f32) -> u16 {
    let n = r24(f64::from(freight_scaling) * f_u32(jobs)) as f32;
    ceil_idiom(n) as u16
}

/// Removes `excess` from the trips matching `pred`, in list order; a trip
/// whose count reaches 0 is erased. Returns what is left of `excess`.
fn reduce(trips: &mut Vec<TripData>, mut excess: u32, pred: impl Fn(&TripData) -> bool) -> u32 {
    let mut i = 0;
    while i < trips.len() && excess > 0 {
        if pred(&trips[i]) {
            let take = trips[i].count.min(excess);
            trips[i].count -= take;
            excess -= take;
            if trips[i].count == 0 {
                trips.remove(i);
                continue;
            }
        }
        i += 1;
    }
    excess
}

fn sum_counts<'a>(trips: impl Iterator<Item = &'a TripData>) -> u32 {
    trips.fold(0u32, |a, t| a.wrapping_add(t.count))
}

/// The lot reconcile of §4.1.1: the trip list against the targets `w` of
/// trip type `ty` (all zero = the probe).
pub fn reconcile_lot(trips: &mut Vec<TripData>, w: [u16; 3], ty: u8) {
    let total: u32 = w.iter().map(|&v| u32::from(v)).sum();
    if total == 0 {
        if trips.first().is_some_and(TripData::is_probe) {
            return;
        }
        trips.clear();
        trips.push(TripData::new(1, ty, 0, trip_flag::PROBE, status::SEARCH));
        return;
    }
    trips.retain(|t| !t.is_probe());
    for (s, &want) in w.iter().enumerate() {
        let s = s as u8;
        let want = u32::from(want);
        trips.retain(|t| t.trip_type == ty);
        let have = sum_counts(trips.iter().filter(|t| t.strategy == s));
        if have < want {
            let d = want - have;
            if let Some(t) = trips
                .iter_mut()
                .find(|t| t.strategy == s && (t.status == status::NEW || t.status == status::FAILED))
            {
                t.count = t.count.wrapping_add(d);
            } else {
                let exists = trips.iter().any(|t| t.strategy == s);
                let st = if exists { status::NEW } else { status::SEARCH };
                trips.push(TripData::new(d, ty, s, 0, st));
            }
        } else if have > want {
            let excess = reduce(trips, have - want, |t| t.strategy == s && t.status == status::NEW);
            reduce(trips, excess, |t| t.strategy == s);
        }
    }
}

/// The neighbour-connection reconcile of §4.1.4: `targets[w][s]` for trip
/// type `w` = 0…2 and travel type `s` = 0…8.
pub fn reconcile_connection(trips: &mut Vec<TripData>, targets: &[[u32; 9]; 3]) {
    for (w, row) in targets.iter().enumerate() {
        let w = w as u8;
        for (s, &target) in row.iter().enumerate() {
            let s = s as u8;
            let matches = |t: &TripData| t.trip_type == w && t.strategy == s;
            let have = sum_counts(trips.iter().filter(|t| matches(t)));
            if have < target {
                // The 16-bit deficit *[quirk]*.
                let d = (target - have) & 0xFFFF;
                if let Some(t) = trips.iter_mut().find(|t| matches(t) && t.status == status::FAILED) {
                    t.count = t.count.wrapping_add(d);
                } else {
                    trips.push(TripData::new(d, w, s, trip_flag::FORCED_START, status::SEARCH));
                }
            } else if have > target {
                let excess = reduce(trips, have - target, |t| matches(t) && t.status == status::FAILED);
                reduce(trips, excess, matches);
            }
        }
    }
}

/// The ConfirmTrips merge (`0x0071D983`–`0x0071DA18`, §4.3, TS55): each
/// trip that is routed (status 3) when the walk reaches it absorbs every
/// later trip of the same type and strategy, whatever that trip's status,
/// and becomes status 0 **only when it absorbed at least one** (the store
/// is inside the match, `0x0071D9B6`); a routed trip with no partner stays
/// routed. Returns the number of absorbing trips (pass counter 5,
/// `0x0071DA0D`).
pub fn merge_routed(trips: &mut Vec<TripData>) -> u32 {
    let mut merges = 0;
    let mut i = 0;
    while i < trips.len() {
        if trips[i].status == status::ROUTED && absorb_later(trips, i) > 0 {
            trips[i].status = status::SEARCH;
            merges += 1;
        }
        i += 1;
    }
    merges
}

/// The ReRouteTrips merge (`0x0071D2B2`–`0x0071D344`, §4.4, TS57): each
/// status-0 trip absorbs every later trip of the same type and strategy,
/// whatever its status (0, 1, 3, 4 or 5): the test meant to skip routed
/// trips reads the absorbing trip's status (`0x0071D2D7`), always 0 there
/// *[quirk]*. The absorbing trip stays status 0; no counter moves.
pub fn merge_search(trips: &mut Vec<TripData>) {
    let mut i = 0;
    while i < trips.len() {
        if trips[i].status == status::SEARCH {
            absorb_later(trips, i);
        }
        i += 1;
    }
}

/// Every later trip of trip `i`'s type and strategy into trip `i`; the
/// number of trips absorbed.
fn absorb_later(trips: &mut Vec<TripData>, i: usize) -> usize {
    let (ty, st) = (trips[i].trip_type, trips[i].strategy);
    let mut absorbed = 0;
    let mut j = i + 1;
    while j < trips.len() {
        if trips[j].trip_type == ty && trips[j].strategy == st {
            let c = trips[j].count;
            trips[i].count = trips[i].count.wrapping_add(c);
            trips.remove(j);
            absorbed += 1;
        } else {
            j += 1;
        }
    }
    absorbed
}

/// The result of a trip's search.
struct Searched {
    found: bool,
    trip_length: f32,
    arrived: u32,
}

impl Run<'_, '_> {
    /// `GetSourceCells` (§4.1): the lot's cell rect or the connection's cell.
    pub(crate) fn source_rect(&self, src: Source) -> CellRect {
        match src {
            Source::Lot(l) => self.h.lot_bounding_rect(l),
            Source::Connection(c) => {
                let (x, z) = self.h.connection_cell(c);
                CellRect::new(x, z, x, z)
            }
        }
    }

    /// The destination rectangle of a cell (§4.4): the lot's bounding rect
    /// when the cell is in the city and holds a lot (`GetLot(x, z, false)`,
    /// any cell of the lot, TS19), else the cell.
    fn dest_rect(&self, x: i32, z: i32) -> CellRect {
        if self.net().in_city(x, z) {
            if let Some(l) = self.h.lot_at(x, z, false) {
                return self.h.lot_bounding_rect(l);
            }
        }
        CellRect::new(x, z, x, z)
    }

    // ---- §4.1 UpdateTrafficVolume ----

    /// `UpdateTrafficVolume` of a source (§4.1.1, §4.1.4).
    pub(crate) fn update_traffic_volume(&mut self, src: Source) {
        if self.h.native_update_traffic_volume(src) {
            return;
        }
        match src {
            Source::Lot(l) => self.update_lot_volume(l),
            Source::Connection(c) => self.update_connection_volume(c),
        }
    }

    /// `cSC4Lot::UpdateTrafficVolume` (§4.1.1).
    fn update_lot_volume(&mut self, lot: u64) {
        let h = self.h;
        let zone = h.lot_zone(lot);
        let (w, ty) = match zone {
            1..=3 => {
                if !h.lot_is_livable(lot) {
                    ([0; 3], 0)
                } else {
                    let found = RES_DEVELOPERS
                        .iter()
                        .enumerate()
                        .map(|(i, &d)| (i as u8, h.lot_travel_population(lot, d)))
                        .find(|&(_, p)| p != 0);
                    match found {
                        None => ([0; 3], 0),
                        Some((wealth, pop)) => {
                            let (x, z) = h.lot_location(lot);
                            let scale = h.workforce_percent(x, z);
                            let commuters = residential_commuters(pop, scale);
                            let pct = self.st.tuning.strategy_percentages(u32::from(wealth) + 1);
                            let byte = h.lot_strategy_byte(lot);
                            (residential_split(commuters, pct, byte), wealth)
                        }
                    }
                }
            }
            7..=9 => {
                if !h.lot_has_road_access(lot) || h.lot_link(lot).is_some() {
                    return;
                }
                if !h.lot_is_livable(lot) {
                    ([0; 3], FREIGHT)
                } else {
                    let jobs = IND_DEVELOPERS
                        .iter()
                        .fold(0u32, |a, &d| a.wrapping_add(h.lot_travel_population(lot, d)));
                    ([0, 0, industrial_trips(jobs, self.st.tuning.freight_scaling)], FREIGHT)
                }
            }
            _ => return,
        };
        let src = Source::Lot(lot);
        let mut trips = h.trip_list(src);
        reconcile_lot(&mut trips, w, ty);
        h.set_trip_list(src, trips);
    }

    /// The neighbour connection's `UpdateTrafficVolume` (§4.1.4).
    fn update_connection_volume(&mut self, conn: u64) {
        let h = self.h;
        if !h.take_connection_volume_dirty(conn) {
            return;
        }
        let holder = h.connection_holder(conn);
        let mut targets = [[0u32; 9]; 3];
        for (w, row) in targets.iter_mut().enumerate() {
            if let Some(v) = h.holder_property(holder, prop::NEIGHBOUR_TARGETS + w as u32) {
                for (d, e) in row.iter_mut().zip(v.items()) {
                    *d = e.as_i64() as u32;
                }
            }
        }
        let src = Source::Connection(conn);
        let mut trips = h.trip_list(src);
        reconcile_connection(&mut trips, &targets);
        h.set_trip_list(src, trips);
    }

    // ---- §4.1 the traffic-source iterator ----

    /// `Reset` of the traffic-source iterator (§4.1, TS20): the neighbour
    /// connections listed now, the lot walk rewound; no source held. The
    /// snapshot holds a counted reference on each connection, so one removed
    /// during the phase is still visited until the next `Reset` (§9.1 TG19);
    /// the previous snapshot's references are given back.
    pub(crate) fn reset_sources(&mut self) {
        let conns = self.h.all_connections();
        for &c in &conns {
            NetworkHost::add_ref(self.h, c);
        }
        let old = std::mem::take(&mut self.st.iter.conns);
        self.st.iter.reset(conns);
        self.hold_source(None);
        for c in old {
            NetworkHost::release(self.h, c);
        }
    }

    /// `GetNext` of the traffic-source iterator (§4.1, TS20): the next lot
    /// of the live walk, then the connections of the snapshot.
    pub(crate) fn next_source(&mut self) -> Option<Source> {
        if !self.st.iter.lots_done {
            let lots = self.h.lots_shared();
            if let Some(l) = self.st.iter.next_lot(lots) {
                return Some(Source::Lot(l));
            }
        }
        self.st.iter.next_connection().map(Source::Connection)
    }

    /// The location the iterator yields with a source (§4.1): a lot's
    /// location (lot slot 21), a connection's cell.
    pub(crate) fn source_cell(&self, src: Source) -> (i32, i32) {
        match src {
            Source::Lot(l) => self.h.lot_location(l),
            Source::Connection(c) => self.h.connection_cell(c),
        }
    }

    /// The native iterator hands out a counted source (§4.1). Keep it
    /// usable if the city removes it between slices, until its list ends.
    pub(crate) fn hold_source(&mut self, source: Option<Source>) {
        if source == self.st.current {
            return;
        }
        let token = |s| match s {
            Source::Lot(t) | Source::Connection(t) => t,
        };
        if let Some(s) = source {
            NetworkHost::add_ref(self.h, token(s));
        }
        if let Some(s) = std::mem::replace(&mut self.st.current, source) {
            NetworkHost::release(self.h, token(s));
        }
    }

    /// A trip phase takes `src`: its cell is stored in the two s16 phase
    /// positions for the load resume (§3.5 step 11, §6.1 row 14, TS44;
    /// `0x0071E276`, `0x0071D0FD`, `0x0071D51C`).
    /// The re-route context cell (`+0x83C`, `+0x840`) is written too, the
    /// two s16 widened (`0x0071E26D`, `0x0071D104`, `0x0071D523`; §6.1 row
    /// 14, TS60).
    fn take_source(&mut self, src: Source) {
        let (x, z) = self.source_cell(src);
        let (px, pz) = (x as i16, z as i16);
        let e = &mut self.st.extras;
        e.phase_positions = [px, pz];
        e.context_x = i32::from(px);
        e.context_z = i32::from(pz);
    }

    // ---- §4.3 ConfirmTrips ----

    /// One slice of ConfirmTrips (state 16, §3.4): at most N sources.
    pub(crate) fn confirm_trips_slice(&mut self) -> bool {
        let n = self.st.tuning.traffic_units_per_slice.max(1);
        let mut units = 0;
        loop {
            if units >= n {
                return false;
            }
            let Some(src) = self.next_source() else {
                return true;
            };
            self.take_source(src);
            self.st.source_pos += 1;
            self.update_traffic_volume(src);
            self.confirm_source(src);
            units += 1;
        }
    }

    /// `ConfirmTrips(source)` (`0x0071D920`, §4.3), with the pass counters
    /// of §10 Q4 (TS59): 3 per source, 7 the status-0 trips met before the
    /// merge, 5 per merge, 0 per route confirmed, 6 per return leg not
    /// confirmed, 4 per full destination, 8 per failure retried.
    pub(crate) fn confirm_source(&mut self, src: Source) {
        use super::state::{PC_CAPACITY_SEND_BACKS, PC_CONFIRMED, PC_FAILURES_RETRIED, PC_MERGES, PC_SEARCH_TRIPS_MET};
        let pc = &mut self.st.pass_counters;
        pc[super::PC_SOURCE_COUNTER] = pc[super::PC_SOURCE_COUNTER].wrapping_add(1);
        let reroute = (pc[super::PC_SOURCE_COUNTER] & 3) == pc[super::PC_REROUTE_BUCKET];
        let mut trips = self.h.trip_list(src);
        // Counter 7, every source, before anything else (`0x0071D95D`).
        let met = trips.iter().filter(|t| t.status == status::SEARCH).count() as u32;
        pc[PC_SEARCH_TRIPS_MET] = pc[PC_SEARCH_TRIPS_MET].wrapping_add(met);
        if reroute {
            let merges = merge_routed(&mut trips);
            pc[PC_MERGES] = pc[PC_MERGES].wrapping_add(merges);
        }
        let bump = |st: &mut super::State, i: usize| st.pass_counters[i] = st.pass_counters[i].wrapping_add(1);
        let rect = self.source_rect(src);
        for slot in trips.iter_mut() {
            self.st.processed = self.st.processed.wrapping_add(1);
            let mut t = slot.clone();
            match t.status {
                status::ROUTED => {
                    if self.capacity_still_available(&t) {
                        t.status = if t.is_freight() {
                            status::SEARCH
                        } else {
                            status::REROUTE
                        };
                        if !reroute {
                            self.confirm_old_route(&mut t, rect);
                        }
                    } else {
                        t.status = status::SEARCH;
                        bump(self.st, PC_CAPACITY_SEND_BACKS);
                    }
                }
                status::FAILED => {
                    t.status = status::SEARCH;
                    bump(self.st, PC_FAILURES_RETRIED);
                }
                _ => {}
            }
            if t.status == status::ROUTED {
                // Only `confirm_old_route` leaves a routed trip routed here.
                bump(self.st, PC_CONFIRMED);
            }
            match t.status {
                status::REROUTE => self.st.to_reroute = self.st.to_reroute.wrapping_add(1),
                status::SEARCH => self.st.to_search = self.st.to_search.wrapping_add(1),
                _ => {}
            }
            *slot = t;
        }
        self.h.set_trip_list(src, trips);
    }

    /// The confirmation of an old route without a search (§4.3).
    fn confirm_old_route(&mut self, t: &mut TripData, rect: CellRect) {
        let strike = self.st.strike_state;
        if !confirm_route(self.net(), strike, &t.route) {
            return;
        }
        let dest = (i32::from(t.dest_x), i32::from(t.dest_z));
        let probe = t.is_probe();
        if t.is_freight() {
            if !probe {
                self.apply_route(&t.route, t.count, rect, dest, mode::OUTBOUND, t.trip_type);
            }
        } else if t.flags & trip_flag::REVERSIBLE != 0 {
            if !probe {
                self.apply_route(&t.route, t.count, rect, dest, mode::BOTH, t.trip_type);
            }
        } else {
            if !confirm_route(self.net(), strike, &t.return_route) {
                // The status stays 1; pass counter 6 (`0x0071DBA0`, TS59).
                let c = &mut self.st.pass_counters[super::state::PC_RETURN_NOT_CONFIRMED];
                *c = c.wrapping_add(1);
                return;
            }
            if !probe {
                self.apply_route(&t.route, t.count, rect, dest, mode::OUTBOUND, t.trip_type);
                self.apply_route(&t.return_route, t.count, rect, dest, mode::RETURN, t.trip_type);
            }
        }
        t.status = status::ROUTED;
    }

    // ---- §4.4 ReRouteTrips and FullTrips ----

    /// One slice of ReRouteTrips (`full` false, the status-1 trips) or
    /// FullTrips (`full` true, the status-0 trips), §3.4: at most N trips
    /// examined. A unit is a trip of the phase's status *[choice]*.
    pub(crate) fn trip_slice(&mut self, full: bool) -> bool {
        let n = self.st.tuning.traffic_units_per_slice.max(1);
        let lazy = self.st.shadow_lazy;
        self.trip_units(full, n, lazy)
    }

    /// [`trip_slice`](Self::trip_slice) with `n` units: a slice normally
    /// ends when it finds the trip after its n-th unit (finishing the
    /// current source and taking the next one on the way); `lazy` (the
    /// traffic shadow) ends right after the n-th unit. `n` = 0 only runs on
    /// to the next trip to examine.
    pub(crate) fn trip_units(&mut self, full: bool, n: u32, lazy: bool) -> bool {
        if self.cx.s.network_changed {
            // A network change aborts a suspended search (§4.4).
            for pf in [self.st.pf.as_mut(), self.st.pf2.as_mut()].into_iter().flatten() {
                pf.reset();
            }
            self.st.continue_search = false;
            self.cx.s.network_changed = false;
        }
        let want = if full { status::SEARCH } else { status::REROUTE };
        let mut units = 0;
        loop {
            let src = match self.st.current {
                Some(s) => s,
                None => {
                    let Some(s) = self.next_source() else {
                        return true;
                    };
                    self.take_source(s);
                    self.hold_source(Some(s));
                    s
                }
            };
            let mut trips = self.h.trip_list(src);
            let next = (self.st.trip_pos..trips.len()).find(|&i| trips[i].status == want);
            let Some(i) = next else {
                self.end_source(full, src, trips);
                continue;
            };
            if units >= n {
                return false;
            }
            units += 1;
            let rect = self.source_rect(src);
            let mut t = trips[i].clone();
            let pc = |st: &mut super::State, i: usize| st.pass_counters[i] = st.pass_counters[i].wrapping_add(1);
            if full {
                // Every search adds 1 to `processed` (none suspends, §3.4).
                let r = self.search_trip(&mut t, rect);
                self.st.processed = self.st.processed.wrapping_add(1);
                if r.found {
                    t.status = status::ROUTED;
                    t.trip_length = r.trip_length;
                    if r.arrived < t.count {
                        // The split (§4.4): +1 to `toSearch`, to the running
                        // phase's `total` and to pass counter 9
                        // (`0x0071D75A`, TS21).
                        let mut copy = t.clone();
                        copy.count = t.count - r.arrived;
                        copy.status = status::SEARCH;
                        t.count = r.arrived;
                        trips.insert(i + 1, copy);
                        self.st.continue_search = true;
                        self.st.to_search = self.st.to_search.wrapping_add(1);
                        self.st.total = self.st.total.wrapping_add(1);
                        pc(self.st, super::state::PC_SPLITS);
                    } else {
                        self.st.continue_search = false;
                    }
                } else {
                    t.status = status::FAILED;
                    self.st.continue_search = false;
                }
            } else if !self.capacity_still_available(&t) {
                // Sent back for lack of capacity: +1 to `toSearch` and to
                // pass counter 4, not to `processed` (`0x0071D20D`, TS21).
                t.status = status::SEARCH;
                self.st.to_search = self.st.to_search.wrapping_add(1);
                pc(self.st, super::state::PC_CAPACITY_SEND_BACKS);
            } else {
                let r = self.search_trip(&mut t, rect);
                self.st.processed = self.st.processed.wrapping_add(1);
                if r.found {
                    t.status = status::ROUTED;
                    t.trip_length = r.trip_length;
                    // A found re-route: +1 to the re-route counter (TS21).
                    pc(self.st, super::state::PC_REROUTE_COUNTER);
                } else {
                    t.status = status::FAILED;
                }
            }
            trips[i] = t;
            self.st.trip_pos = i + 1;
            self.h.set_trip_list(src, trips);
            if lazy && units >= n {
                return false;
            }
        }
    }

    /// The end of a source's list in ReRouteTrips (the merge of its status-0
    /// trips) or FullTrips (`CompleteTripSourceCalculations`); the next
    /// source is taken by the caller.
    fn end_source(&mut self, full: bool, src: Source, mut trips: Vec<TripData>) {
        if full {
            let rect = self.source_rect(src);
            self.complete_trip_source_calculations(src, rect, &mut trips);
        } else {
            merge_search(&mut trips);
        }
        self.h.set_trip_list(src, trips);
        self.hold_source(None);
        self.st.source_pos += 1;
        self.st.trip_pos = 0;
    }

    /// The traffic shadow's run-on (tooling): after the copy's units of a
    /// day, the original's deadline may have stopped it at any test before
    /// its next unit. In game it stops before a trip to examine, or at the
    /// end of a list (after the merge or `CompleteTripSourceCalculations`,
    /// before the next source is taken), which §3.4 does not name (TS
    /// question in the lane report). The copy runs on without examining a
    /// trip: past every list with no trip left to examine, until the source
    /// at `target` (the original's saved source cell, §6.1 row 14) is held
    /// (its list's end is left to the shadow, [`Self::end_pending_list`]) or
    /// a trip to examine is next. True when the phase finished on the way.
    pub(crate) fn trip_run_on(&mut self, full: bool, target: [i16; 2]) -> bool {
        let want = if full { status::SEARCH } else { status::REROUTE };
        loop {
            let src = match self.st.current {
                Some(s) => s,
                None => {
                    let Some(s) = self.next_source() else {
                        return true;
                    };
                    self.take_source(s);
                    self.hold_source(Some(s));
                    s
                }
            };
            let trips = self.h.trip_list(src);
            if trips[self.st.trip_pos.min(trips.len())..]
                .iter()
                .any(|t| t.status == want)
            {
                return false;
            }
            if self.st.extras.phase_positions == target {
                return false;
            }
            self.end_source(full, src, trips);
        }
    }

    /// The traffic shadow (tooling): the source held with no trip left to
    /// examine, its list's end not taken yet.
    pub(crate) fn pending_list_end(&self, full: bool) -> Option<Source> {
        let want = if full { status::SEARCH } else { status::REROUTE };
        let src = self.st.current?;
        let trips = self.h.trip_list(src);
        let left = trips[self.st.trip_pos.min(trips.len())..]
            .iter()
            .any(|t| t.status == want);
        (!left).then_some(src)
    }

    /// The traffic shadow (tooling): takes the list end of
    /// [`Self::pending_list_end`]'s source (the merge or
    /// `CompleteTripSourceCalculations`); the next source is not taken.
    pub(crate) fn end_pending_list(&mut self, full: bool) -> bool {
        let Some(src) = self.pending_list_end(full) else {
            return false;
        };
        let trips = self.h.trip_list(src);
        self.end_source(full, src, trips);
        true
    }

    /// `SetupTrip` then `PerformPathSearch` with the two pathfinders.
    fn search_trip(&mut self, t: &mut TripData, rect: CellRect) -> Searched {
        let none = Searched {
            found: false,
            trip_length: 0.0,
            arrived: 0,
        };
        let Some(mut pf) = self.st.pf.take() else { return none };
        let Some(mut pf2) = self.st.pf2.take() else {
            self.st.pf = Some(pf);
            return none;
        };
        self.setup_trip(&mut pf, &mut pf2, t, rect);
        let found = self.perform_path_search(&mut pf, &mut pf2, t, rect);
        let r = Searched {
            found,
            trip_length: pf.get_trip_length(),
            arrived: pf.get_traffic_arrived(),
        };
        self.st.pf = Some(pf);
        self.st.pf2 = Some(pf2);
        r
    }

    /// `SetupTrip(trip)` (`0x0070EC40`, §4.4).
    pub(crate) fn setup_trip(&mut self, pf: &mut Pathfinder, pf2: &mut Pathfinder, t: &TripData, rect: CellRect) {
        // 1.
        pf.set_traffic_density(t.count);
        // 2.
        if t.status == status::REROUTE {
            pf.set_trip_destination_filter(filter::END_RECT);
            pf.set_end_rect(self.dest_rect(i32::from(t.dest_x), i32::from(t.dest_z)));
        } else {
            pf.set_trip_destination_filter(filter::STANDARD);
            pf.clear_all_destination_flags();
            let bits = if t.is_probe() && t.trip_type == 0 {
                7
            } else {
                1u8.checked_shl(u32::from(t.trip_type)).unwrap_or(0)
            };
            pf.set_destination_flag(bits);
        }
        // 3. Searches never suspend (§3.4).
        pf.set_max_work_time(0);
        pf2.set_max_work_time(0);
        pf.set_start_rect(rect);
        // 4.
        pf.clear_all_flags();
        pf.clear_all_travel_type_flags();
        pf.set_starting_cost_array(None);
        pf.set_max_trip_length(self.st.tuning.commute_max_time);
        if t.trip_type <= 2 {
            // 5.
            if self.st.continue_search {
                pf.set_flag(flag::CONTINUE);
            }
            pf.set_travel_type_flag(COMMUTER_TYPES, false);
            if t.flags & trip_flag::FORCED_START != 0 {
                pf.set_travel_type_flag(1u16.checked_shl(u32::from(t.strategy)).unwrap_or(0), true);
            } else {
                pf.set_travel_type_flag(COMMUTER_START, true);
                let costs = self.strategy_costs(t.strategy);
                if costs.is_some() {
                    pf.set_starting_cost_array(costs);
                }
            }
        } else if t.trip_type == FREIGHT {
            // 6.
            pf.set_flag(flag::NO_BUDGET);
            pf.set_travel_type_flag(FREIGHT_TYPES, true);
        }
    }

    /// The starting cost array of a strategy (§4.4 step 5): 0 mass transit,
    /// 1 car, 2 neutral; none for another strategy *[choice]*.
    fn strategy_costs(&self, strategy: u8) -> Option<[f32; 9]> {
        let tu = &self.st.tuning;
        match strategy {
            0 => Some(tu.start_mass_transit),
            1 => Some(tu.start_car),
            2 => Some(tu.start_neutral),
            _ => None,
        }
    }

    /// `SetupPathFinderForLot(pf, lot)` (slot 7, `0x00711610`, §4.4, TS10).
    /// Unlike SetupTrip it never sets the traffic density, the work-time
    /// limit, flag 4, a forced start type or a re-route end rect.
    pub(crate) fn setup_path_finder_for_lot(&mut self, pf: &mut Pathfinder, lot: u64) -> bool {
        // 1.
        if lot == 0 {
            return false;
        }
        // 2. An empty list: UpdateTrafficVolume once, then look again.
        let src = Source::Lot(lot);
        let mut trips = self.h.trip_list(src);
        if trips.is_empty() {
            self.update_traffic_volume(src);
            trips = self.h.trip_list(src);
        }
        let Some(first) = trips.first() else {
            return false;
        };
        // 3. The dominant strategy (unsigned, strict: a tie keeps the lower
        // index). A strategy above 2 is not summed *[choice]* (TS47: the
        // original adds it past its 12-byte heap array of sums,
        // `0x00711680`–`0x00711687`; only a damaged save has one). The type
        // is the first trip's.
        let mut sum = [0u32; 3];
        for t in &trips {
            if let Some(s) = sum.get_mut(usize::from(t.strategy)) {
                *s = s.wrapping_add(t.count);
            }
        }
        let mut best = 0;
        if sum[1] > sum[0] {
            best = 1;
        }
        if sum[2] > sum[best] {
            best = 2;
        }
        let ty = first.trip_type;
        // 4.
        pf.set_start_rect(self.source_rect(src));
        pf.clear_all_flags();
        pf.clear_all_travel_type_flags();
        pf.set_starting_cost_array(None);
        pf.set_max_trip_length(self.st.tuning.commute_max_time);
        // 5.
        pf.set_trip_destination_filter(filter::STANDARD);
        pf.clear_all_destination_flags();
        pf.set_destination_flag(1u8.checked_shl(u32::from(ty)).unwrap_or(0));
        // 6.
        if ty <= 2 {
            pf.set_travel_type_flag(COMMUTER_TYPES, false);
            pf.set_travel_type_flag(COMMUTER_START, true);
            pf.set_starting_cost_array(self.strategy_costs(best as u8));
            true
        } else if ty == FREIGHT {
            pf.set_flag(flag::NO_BUDGET);
            pf.set_travel_type_flag(FREIGHT_TYPES, true);
            true
        } else {
            false
        }
    }

    /// The return leg's start and end rects (§4.4, `0x0071C858`–`0x0071C987`,
    /// TS17) from pf's end cell, its result node's travel type and its start
    /// rect. Start: in the city the bounding rect of `GetLot(x, z, false)`,
    /// or `None` without a lot (pf2 keeps its previous start rect
    /// *[quirk]*); off the map the single cell, moved by
    /// [`find_highway_return_tile`] (`back` false) for a car or bus at an
    /// edge record with a divided road. End: pf's start rect, or when its
    /// `(x0, z0)` corner is off the map at such a record (any type) the
    /// single cell moved with `back` true.
    pub(crate) fn return_leg_rects(
        &self,
        end: (i32, i32),
        end_type: u8,
        pf_start: CellRect,
    ) -> (Option<CellRect>, CellRect) {
        let s: &NetworkState = self.net();
        let size = (s.count_x, s.count_z);
        let right = self.h.drives_on_right();
        let divided = |x: i32, z: i32| s.traffic_neighbor_connection(x, z).mask & DIVIDED_ROAD_MASK != 0;
        let (x, z) = end;
        let start = if s.in_city(x, z) {
            self.h.lot_at(x, z, false).map(|l| self.h.lot_bounding_rect(l))
        } else {
            let (x, z) = if (end_type == 1 || end_type == 2) && divided(x, z) {
                find_highway_return_tile(x, z, false, right, size)
            } else {
                (x, z)
            };
            Some(CellRect::new(x, z, x, z))
        };
        let (x0, z0) = (pf_start.x1, pf_start.z1);
        let end = if !s.in_city(x0, z0) && divided(x0, z0) {
            let (x, z) = find_highway_return_tile(x0, z0, true, right, size);
            CellRect::new(x, z, x, z)
        } else {
            pf_start
        };
        (start, end)
    }

    /// `PerformPathSearch(trip)` (`0x0071C6C0`, §4.4). A suspended search
    /// cannot happen (max work time 0) and counts as not found.
    fn perform_path_search(
        &mut self,
        pf: &mut Pathfinder,
        pf2: &mut Pathfinder,
        t: &mut TripData,
        rect: CellRect,
    ) -> bool {
        // 1.
        if self.find_path_with(pf) != result::FOUND {
            return false;
        }
        // 2.
        let (ex, ez) = pf.get_end_cell();
        t.dest_x = ex as i16;
        t.dest_z = ez as i16;
        let arrived = pf.get_traffic_arrived();
        let dest = (ex, ez);
        let probe = t.is_probe();
        let mut route = Vec::new();
        if t.is_freight() || pf.was_trip_reversible(self.net()) {
            let m = if t.is_freight() {
                mode::OUTBOUND
            } else {
                t.flags |= trip_flag::REVERSIBLE;
                mode::BOTH
            };
            // Only the outbound route is fetched: `returnRoute` keeps what
            // it held (an earlier two-leg result, the split's source, the
            // save) (§2, §4.4, TS58).
            t.travel_types_used = pf.get_compressed_route(self.net(), &mut route);
            t.route = route;
            if !probe {
                let r = t.route.clone();
                self.apply_route(&r, arrived, rect, dest, m, t.trip_type);
            }
            return true;
        }
        t.flags &= !trip_flag::REVERSIBLE;
        // The return leg on pf2, configured field by field (§4.4, TS17).
        let (start, end) = self.return_leg_rects((ex, ez), pf.get_last_travel_type(), pf.start_rect());
        if let Some(r) = start {
            pf2.set_start_rect(r);
        }
        pf2.set_end_rect(end);
        pf2.set_flags(flag::NO_BUDGET);
        pf2.set_traffic_density(arrived);
        pf2.set_trip_destination_filter(filter::END_RECT);
        pf2.set_allowed_travel_types(pf.allowed_travel_types());
        pf2.set_starting_travel_types(1u16.checked_shl(u32::from(pf.get_last_travel_type())).unwrap_or(0));
        pf2.set_starting_cost_array(pf.starting_cost_array());
        let (cell, side) = pf.park_and_ride();
        if cell != NO_PARK_AND_RIDE {
            pf2.set_park_and_ride(cell, side);
            pf2.set_flag(flag::PARK_AND_RIDE);
        }
        if self.find_path_with(pf2) != result::FOUND {
            return false;
        }
        let mask = pf.get_compressed_route(self.net(), &mut route);
        let mut ret = Vec::new();
        let mask2 = pf2.get_compressed_route(self.net(), &mut ret);
        t.travel_types_used = mask | mask2;
        t.route = route;
        t.return_route = ret;
        if !probe {
            let (r, b) = (t.route.clone(), t.return_route.clone());
            self.apply_route(&r, arrived, rect, dest, mode::OUTBOUND, t.trip_type);
            self.apply_route(&b, arrived, rect, dest, mode::RETURN, t.trip_type);
        }
        true
    }

    // ---- §4.6 CompleteTripSourceCalculations ----

    /// `CompleteTripSourceCalculations(source)` (`0x0070F0A0`, §4.6).
    pub(crate) fn complete_trip_source_calculations(&mut self, _src: Source, rect: CellRect, trips: &mut [TripData]) {
        let max_time = self.st.tuning.commute_max_time;
        let mut acc = 0.0f32;
        let mut n: u32 = 0;
        let mut failed = false;
        for t in trips.iter_mut() {
            self.st.pass_total = self.st.pass_total.wrapping_add(1);
            let count = t.count;
            let k = usize::from(t.trip_type);
            match t.status {
                status::ROUTED => {
                    t.flags |= trip_flag::ROUTED_LAST_PASS;
                    let len = r24(f_u32(count) * f64::from(t.trip_length));
                    acc = r24(f64::from(acc) + len) as f32;
                    n = n.wrapping_add(count);
                    if !t.is_probe() {
                        if k < 3 {
                            self.st.attempted[k] = self.st.attempted[k].wrapping_add(count);
                            self.st.routed[k] = self.st.routed[k].wrapping_add(count);
                            let l2 = r24(f_u32(count) * f64::from(t.trip_length));
                            self.st.car_len_sum = r24(l2 + f64::from(self.st.car_len_sum)) as f32;
                            self.st.car_len_count = self.st.car_len_count.wrapping_add(count);
                        } else if t.is_freight() {
                            self.st.freight_len_sum = r24(f64::from(self.st.freight_len_sum) + len) as f32;
                            self.st.freight_count = self.st.freight_count.wrapping_add(count);
                        }
                        for i in 0..9 {
                            if t.travel_types_used & (1 << i) != 0 {
                                self.st.type_count_building[i] = self.st.type_count_building[i].wrapping_add(count);
                            }
                        }
                    }
                }
                status::FAILED => {
                    t.flags &= !trip_flag::ROUTED_LAST_PASS;
                    failed = true;
                    let len = r24(f_u32(count) * f64::from(max_time));
                    acc = r24(f64::from(acc) + len) as f32;
                    n = n.wrapping_add(count);
                    if k < 3 {
                        if !t.is_probe() {
                            self.st.attempted[k] = self.st.attempted[k].wrapping_add(count);
                        }
                    } else if t.is_freight() {
                        // Probe or not *[quirk]*.
                        self.st.freight_len_sum = r24(f64::from(self.st.freight_len_sum) + len) as f32;
                        self.st.freight_count = self.st.freight_count.wrapping_add(count);
                    }
                }
                _ => t.flags &= !trip_flag::ROUTED_LAST_PASS,
            }
        }
        if n == 0 {
            return;
        }
        let avg = r24(f64::from(acc) / r24(f64::from(n as i32))) as f32;
        if self.net().in_city(rect.x1, rect.z1) {
            let v = if failed { 0 } else { trip_length_byte(avg, max_time) };
            if let Some(g) = &self.st.trip_length {
                for x in rect.x1..=rect.x2 {
                    for z in rect.z1..=rect.z2 {
                        g.set(x, z, v);
                    }
                }
            }
        } else {
            for ty in 0..=12 {
                if let Some(conn) = self.h.find_connection(ty, rect.x1, rect.z1) {
                    let holder = self.h.connection_holder(conn);
                    if holder != 0 {
                        self.h.set_holder_property(
                            holder,
                            prop::CONNECTION_TRIP_LENGTH,
                            Variant::Scalar(Elem::F32(avg)),
                        );
                    }
                    break;
                }
            }
        }
    }

    // ---- §4.7 FinishTripMap ----

    /// FinishTripMap (state 15, §4.7, TS26), in one call. After a load in
    /// state 15 the lot walk starts after the lot DoPostCityInit found at
    /// the saved cell (§3.5 step 11). True when finished; false only at the
    /// traffic shadow's stop ([`State::shadow_finish_stop`], tooling: the
    /// walk resumes after that lot).
    ///
    /// [`State::shadow_finish_stop`]: super::State::shadow_finish_stop
    pub(crate) fn finish_trip_map(&mut self) -> bool {
        let h = self.h;
        let mut lots = h.lots();
        if let Some(after) = self.st.finish_resume.take() {
            if let Some(i) = lots.iter().position(|&l| l == after) {
                lots.drain(..=i);
            }
        }
        for lot in lots {
            // The lot's location into the phase positions as the walk takes
            // it (TS45, `0x0070FFD3`–`0x0070FFE7`): the state-15 resume of
            // §3.5 step 11 reads them back.
            let (lx, lz) = h.lot_location(lot);
            self.st.extras.phase_positions = [lx as i16, lz as i16];
            let stop_here = self.st.shadow_finish_stop == Some(self.st.extras.phase_positions);
            self.finish_lot(lot);
            if stop_here {
                self.st.finish_resume = Some(lot);
                return false;
            }
        }
        true
    }

    /// FinishTripMap's work on one lot (§4.7).
    fn finish_lot(&mut self, lot: u64) {
        let h = self.h;
        h.clear_lot_flag(lot, 0x80);
        let zone = h.lot_zone(lot);
        if !(zone == 8 || zone == 9) || h.lot_has_road_access(lot) {
            return;
        }
        let Some(g) = self.st.trip_length.clone() else { return };
        // `value := 0`; the linked lot's location, else the donor cell.
        let from = match h.lot_link(lot) {
            Some(link) => Some(h.lot_location(link)),
            None => self.search_trip_length_donor(lot, zone),
        };
        let v = from.map_or(0, |(sx, sz)| g.get_cell_value(sx, sz));
        // No donor writes 0 over the rect (TS26).
        let r = h.lot_bounding_rect(lot);
        for x in r.x1..=r.x2 {
            for z in r.z1..=r.z2 {
                g.set(x, z, v);
            }
        }
    }

    /// The outward search of §4.7 (`0x00710016`–`0x007101D7`, TS26) from the
    /// lot's location: distance 1, 2, …, at each distance the open
    /// directions −x, −z, +x, +z. A cell outside the city, with no lot or
    /// with a lot of another zone closes its direction; the lot itself and
    /// a same-zone lot without road access are passed over; the first
    /// same-zone lot with road access ends the search at that cell.
    fn search_trip_length_donor(&self, lot: u64, zone: u32) -> Option<(i32, i32)> {
        let (x0, z0) = self.h.lot_location(lot);
        let mut alive = [true; 4];
        let mut d = 1;
        while alive.iter().any(|&a| a) {
            for (k, a) in alive.iter_mut().enumerate() {
                if !*a {
                    continue;
                }
                let (x, z) = (
                    x0 + d * crate::traffic_network::DX[k],
                    z0 + d * crate::traffic_network::DZ[k],
                );
                if !self.net().in_city(x, z) {
                    *a = false;
                    continue;
                }
                match self.h.lot_at(x, z, false) {
                    Some(other) if self.h.lot_zone(other) == zone => {
                        if self.h.lot_has_road_access(other) {
                            return Some((x, z));
                        }
                    }
                    _ => *a = false,
                }
            }
            d += 1;
        }
        None
    }
}

/// The trip-length map value of §4.6: `clamp(round(r24(r24(avg /
/// commuteMaxTime) × 255.0f)))`, `> 255` (unsigned) → 255, 0 → 1.
pub fn trip_length_byte(avg: f32, commute_max_time: f32) -> u8 {
    let q = r24(f64::from(avg) / f64::from(commute_max_time));
    let g = rhe(r24(q * 255.0) as f32) as u32;
    if g > 255 {
        255
    } else if g == 0 {
        1
    } else {
        g as u8
    }
}
