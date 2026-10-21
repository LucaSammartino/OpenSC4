// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The `cSC4MySimDispatch` tests: the §4.7 job and commute behaviours of
//! `docs/specs/mysims.md` (the parts the simulation side names) over a real
//! traffic network and pathfinder, and the `cSC4MySimPool` behaviours of
//! §1.1.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use crate::grid::{CellRect, CityDims, CoreGrids, GridProvider};
use crate::pathfinder::{shared_work_check, PathfinderHost, TravelTables};
use crate::traffic_network::{net_type, travel, NetworkHost, NetworkModel, PathInfo, Tuning};

use super::dispatch::{
    destination_flag_for_wealth, set_current_commute_destination, JobSearchState, JobSearchWorld,
    RecordingJobPathfinder, FLAG_HAS_HOME, STATE_STORE_UNVALIDATED,
};
use super::dispatch::{DispatchHost, DispatchLocation, DispatchState, LotAt, MySimDispatch};
use super::{dispatch_note, JobDestination, DISPATCH_CLSID, DISPATCH_IID};

/// holder → (max[4], used[4]).
type Cap = ([u32; 4], [u32; 4]);

/// An 8 × 8 city whose street runs (0,3)…(4,3) (pathfinder.md §11
/// preamble): the MySim's home at (2,2), jobs along the street.
#[derive(Default)]
struct DispatchWorld {
    /// The city cells.
    cells: Cell<(i32, i32)>,
    tables: Cell<TravelTables>,
    max_distance: Cell<u32>,
    attractiveness: Cell<f32>,
    strike: Cell<u8>,
    trip_scale: Cell<f32>,
    carpool: Cell<bool>,
    volumes: RefCell<HashMap<(u32, i32, i32), f32>>,
    dest: RefCell<HashMap<(i32, i32), u8>>,
    free: Cell<[u32; 4]>,
    /// holder → (max[4], used[4]).
    caps: RefCell<HashMap<u64, Cap>>,
    /// (lot token, bounding rect, holder).
    lots: RefCell<Vec<(u64, CellRect, u64)>>,
    pools: RefCell<HashMap<(i32, i32), u64>>,
    income: Cell<f64>,
    deadline_reached: Cell<bool>,
    /// (building, lot rect): the dispatch's building answers.
    buildings: RefCell<HashMap<u64, CellRect>>,
    /// The neighbour cells of the map edges.
    neighbors: RefCell<Vec<(i32, i32)>>,
    notifications: RefCell<Vec<(i32, u32, u32, u32)>>,
    wealth: Cell<u32>,
}

impl DispatchWorld {
    /// The §11 preamble: 8 × 8, car travel, MaxDistanceOriginToNetwork 1.
    fn new() -> Rc<DispatchWorld> {
        let w = DispatchWorld::default();
        w.cells.set((8, 8));
        let mut t = TravelTables::default();
        t.affected_by_traffic[travel::CAR] = true;
        t.can_reach_destination[travel::CAR] = true;
        t.generates_traffic[travel::CAR] = true;
        w.tables.set(t);
        w.max_distance.set(1);
        w.attractiveness.set(1.0);
        w.trip_scale.set(1.0);
        Rc::new(w)
    }

    /// A street cell at (x, z) (pathfinder tests' `street_cell`).
    fn street(&self, net: &mut crate::traffic_network::NetworkState, x: i32, z: i32) {
        if let Some(c) = net.conn_mut(x, z) {
            c.conn[travel::CAR] = 0x0104;
            c.switch[travel::CAR] = 0xFFFF;
            c.prior_car = [0xFFFF; 4];
            c.wormhole = 0;
        }
        if let Some(r) = net.append_info(1, x, z) {
            r.inv_cap = 0.0625;
            r.net_type = net_type::STREET as u8;
            r.extra_cost = 0.0;
        }
    }

    fn lot(&self, token: u64, rect: CellRect) {
        self.lots.borrow_mut().push((token, rect, token + 0x1_0000));
        self.buildings.borrow_mut().insert(token, rect);
    }

    fn take_notifications(&self) -> Vec<(i32, u32, u32, u32)> {
        std::mem::take(&mut self.notifications.borrow_mut())
    }
}

impl GridProvider<u32> for DispatchWorld {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn crate::grid::TractGrid<u32>>> {
        GridProvider::<u32>::create_grid(
            &CoreGrids {
                city: CityDims {
                    cells_x: self.cells.get().0,
                    cells_z: self.cells.get().1,
                    ..CityDims::square(1)
                },
            },
            instance,
            tract_size,
        )
    }
}

impl GridProvider<u8> for DispatchWorld {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn crate::grid::TractGrid<u8>>> {
        GridProvider::<u8>::create_grid(
            &CoreGrids {
                city: CityDims {
                    cells_x: self.cells.get().0,
                    cells_z: self.cells.get().1,
                    ..CityDims::square(1)
                },
            },
            instance,
            tract_size,
        )
    }
}

impl NetworkHost for DispatchWorld {
    fn cell_counts(&self) -> (i32, i32) {
        self.cells.get()
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        ((x / 16.0).floor() as i32, (z / 16.0).floor() as i32)
    }
    fn network_occupants_in_rect(&self, _rect: CellRect) -> Vec<u64> {
        Vec::new()
    }
    fn subway_occupants_in_rect(&self, _rect: CellRect) -> Vec<u64> {
        Vec::new()
    }
    fn prop_occupants_in_rect(&self, _rect: CellRect) -> Vec<u64> {
        Vec::new()
    }
    fn occupant_position(&self, _occ: u64) -> [f32; 3] {
        [0.0; 3]
    }
    fn occupant_bounding_cells(&self, _occ: u64) -> CellRect {
        CellRect::new(0, 0, -1, -1)
    }
    fn prop_type(&self, _occ: u64) -> u32 {
        0
    }
    fn occupant_property(&self, _occ: u64, _id: u32) -> Option<crate::variant::Variant> {
        None
    }
    fn has_network_flag(&self, _occ: u64, _flag: u32) -> bool {
        false
    }
    fn has_any_network_flag(&self, _occ: u64, _mask: u32) -> bool {
        false
    }
    fn is_of_type(&self, _occ: u64, _network_type: u32) -> bool {
        false
    }
    fn path_connection(&self, _occ: u64, _path_type: u32, _entry: u32, _exit: u32) -> bool {
        false
    }
    fn path_info(&self, _occ: u64) -> Option<PathInfo> {
        None
    }
    fn path_info_turn_around(&self, _pi: PathInfo, _path_type: u32, _entry: u32) -> bool {
        false
    }
    fn path_info_exits_from_entry(&self, _pi: PathInfo, _path_type: u32, _entry: u32) -> Vec<(u8, u64)> {
        Vec::new()
    }
    fn path_info_entries_near(
        &self,
        _pi: PathInfo,
        _path_type: u32,
        _vertex: u64,
        _exit: u32,
        _radius: f32,
    ) -> Vec<u32> {
        Vec::new()
    }
    fn is_tunnel(&self, _occ: u64) -> bool {
        false
    }
    fn tunnel_paired_portal(&self, _occ: u64) -> Option<u64> {
        None
    }
    fn lot_at(&self, x: i32, z: i32, _anchor_only: bool) -> Option<u64> {
        self.lots
            .borrow()
            .iter()
            .find(|(_, r, _)| (r.x1..=r.x2).contains(&x) && (r.z1..=r.z2).contains(&z))
            .map(|l| l.0)
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        self.lots
            .borrow()
            .iter()
            .find(|l| l.0 == lot)
            .map_or(CellRect::new(0, 0, -1, -1), |l| l.1)
    }
    fn lot_location(&self, lot: u64) -> (i32, i32) {
        self.lots
            .borrow()
            .iter()
            .find(|l| l.0 == lot)
            .map_or((0, 0), |l| (l.1.x1, l.1.z1))
    }
    fn lot_facing(&self, _lot: u64) -> u32 {
        0
    }
    fn lot_building(&self, _lot: u64) -> Option<u64> {
        None
    }
    fn lot_populations(&self, _lot: u64) -> Vec<Vec<(u32, u32)>> {
        Vec::new()
    }
    fn lot_holder(&self, lot: u64) -> u64 {
        self.lots.borrow().iter().find(|l| l.0 == lot).map_or(0, |l| l.2)
    }
    fn holder_property(&self, _holder: u64, _id: u32) -> Option<crate::variant::Variant> {
        None
    }
    fn set_holder_property(&self, _holder: u64, _id: u32, _value: crate::variant::Variant) {}
    fn remove_holder_property(&self, _holder: u64, _id: u32) -> bool {
        false
    }
    fn max_trip_capacity(&self, holder: u64, k: u32) -> u32 {
        self.caps.borrow().get(&holder).map_or(0, |c| c.0[k as usize & 3])
    }
    fn neighbor_pool_holder(&self, _x: i32, _z: i32) -> Option<u64> {
        None
    }
    fn is_road_damaged(&self, _x: i32, _z: i32) -> bool {
        false
    }
    fn cell_under_water(&self, _x: i32, _z: i32) -> bool {
        false
    }
    fn is_navigable(&self, _x: i32, _z: i32) -> bool {
        false
    }
    fn water_depth(&self, _x: i32, _z: i32) -> f32 {
        0.0
    }
    fn body_of_water(&self, _x: i32, _z: i32) -> Option<u16> {
        None
    }
    fn connections_of_type(&self, _connection_type: u32) -> Vec<u64> {
        Vec::new()
    }
    fn all_connections(&self) -> Vec<u64> {
        Vec::new()
    }
    fn connection_cell(&self, _conn: u64) -> (i32, i32) {
        (0, 0)
    }
    fn post(&self, _msg_type: u32) {}
    fn add_ref(&self, _token: u64) {}
    fn release(&self, _token: u64) {}
}

impl PathfinderHost for DispatchWorld {
    fn cell_counts(&self) -> (i32, i32) {
        self.cells.get()
    }
    fn work_deadline(&self, _ms: u32) -> u64 {
        0
    }
    fn work_deadline_reached(&self, _deadline: u64) -> bool {
        self.deadline_reached.get()
    }
    fn travel_tables(&self) -> TravelTables {
        self.tables.get()
    }
    fn max_distance_origin_to_network(&self) -> u32 {
        self.max_distance.get()
    }
    fn nearest_destination_attractiveness(&self) -> f32 {
        self.attractiveness.get()
    }
    fn strike_state(&self) -> u8 {
        self.strike.get()
    }
    fn trip_scale_for_display(&self) -> f32 {
        self.trip_scale.get()
    }
    fn volume(&self, n: u32, x: i32, z: i32) -> f32 {
        self.volumes.borrow().get(&(n, x, z)).copied().unwrap_or(0.0)
    }
    fn set_volume(&self, n: u32, x: i32, z: i32, v: f32) {
        self.volumes.borrow_mut().insert((n, x, z), v);
    }
    fn destination_byte(&self, x: i32, z: i32) -> u8 {
        self.dest.borrow().get(&(x, z)).copied().unwrap_or(0)
    }
    fn set_destination_byte(&self, x: i32, z: i32, bits: u8) {
        self.dest.borrow_mut().insert((x, z), bits);
    }
    fn free_destinations(&self, k: u32) -> u32 {
        self.free.get().get(k as usize).copied().unwrap_or(0)
    }
    fn set_free_destinations(&self, k: u32, n: u32) {
        let mut f = self.free.get();
        if let Some(slot) = f.get_mut(k as usize) {
            *slot = n;
        }
        self.free.set(f);
    }
    fn max_trip_capacity(&self, holder: u64, k: u32) -> u32 {
        self.caps.borrow().get(&holder).map_or(0, |c| c.0[k as usize & 3])
    }
    fn used_trip_capacity(&self, holder: u64, k: u32) -> u32 {
        self.caps.borrow().get(&holder).map_or(0, |c| c.1[k as usize & 3])
    }
    fn neighbor_pool_holder(&self, _t: u32, x: i32, z: i32) -> Option<u64> {
        self.pools.borrow().get(&(x, z)).copied()
    }
    fn income_accumulator(&self) -> f64 {
        self.income.get()
    }
    fn set_income_accumulator(&self, v: f64) {
        self.income.set(v);
    }
    fn lot_at(&self, x: i32, z: i32) -> Option<u64> {
        self.lots
            .borrow()
            .iter()
            .find(|(_, r, _)| (r.x1..=r.x2).contains(&x) && (r.z1..=r.z2).contains(&z))
            .map(|l| l.0)
    }
    fn lot_holder(&self, lot: u64) -> u64 {
        self.lots.borrow().iter().find(|l| l.0 == lot).map_or(0, |l| l.2)
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        self.lots
            .borrow()
            .iter()
            .find(|l| l.0 == lot)
            .map_or(CellRect::new(0, 0, -1, -1), |l| l.1)
    }
    fn occupants_in_cells(&self, _rect: CellRect) -> Vec<u64> {
        Vec::new()
    }
    fn occupant_filter_includes(&self, _filter: u64, _occupant: u64) -> bool {
        false
    }
    fn is_ordinance_on(&self, _id: u32) -> bool {
        self.carpool.get()
    }
}

/// A city of one street and three lots, wired to a `MySimDispatch`.
struct D {
    w: Rc<DispatchWorld>,
    d: Rc<MySimDispatch>,
}

impl D {
    /// The city: street (0,3)…(4,3); home lot 1 at (2,2), job lots 2 and 3
    /// at (4,4) and (4,2) with commuter capacity; no neighbour pool.
    fn new() -> D {
        let w = DispatchWorld::new();
        let mut net = crate::traffic_network::NetworkState {
            count_x: 8,
            count_z: 8,
            conn: vec![crate::traffic_network::CellConnection::default(); crate::traffic_network::ARRAY_LEN],
            info: vec![Vec::new(); 64],
            ..crate::traffic_network::NetworkState::default()
        };
        net.inv_max_speed[travel::CAR][net_type::STREET as usize] = 1.5;
        net.congestion_curve = crate::curve::ResponseCurve::from_points(&[(0.0, 1.0)]);
        for x in 0..=4 {
            w.street(&mut net, x, 3);
        }
        let model = NetworkModel::new(w.clone());
        model.update_state(|s| *s = net);
        model.set_tuning(Tuning::shipped());
        *w.neighbors.borrow_mut() = vec![(-1, 3)];
        let d = MySimDispatch::new(w.clone(), model.clone(), shared_work_check());
        D { w, d }
    }

    fn take(&self) -> Vec<(i32, u32, u32, u32)> {
        self.w.take_notifications()
    }
}

impl DispatchHost for DispatchWorld {
    fn home_rect(&self, sim: i32) -> Option<CellRect> {
        (sim == 1).then_some(CellRect::new(2, 2, 2, 2))
    }
    fn building_rect(&self, building: u64) -> Option<CellRect> {
        self.buildings.borrow().get(&building).copied()
    }
    fn wealth(&self, _sim: i32) -> u32 {
        self.wealth.get()
    }
    fn occupant_lot_rect(&self, occupant: u64) -> Option<CellRect> {
        self.buildings.borrow().get(&occupant).copied()
    }
    fn lot_rect_at(&self, x: i32, z: i32) -> Option<CellRect> {
        self.buildings
            .borrow()
            .values()
            .find(|r| (r.x1..=r.x2).contains(&x) && (r.z1..=r.z2).contains(&z))
            .copied()
    }
    fn cell_is_in_bounds(&self, x: i32, z: i32) -> bool {
        x >= 0 && z >= 0 && x < self.cells.get().0 && z < self.cells.get().1
    }
    fn lot_at_cell(&self, x: i32, z: i32) -> LotAt {
        self.buildings
            .borrow()
            .iter()
            .find(|(_, r)| (r.x1..=r.x2).contains(&x) && (r.z1..=r.z2).contains(&z))
            .map_or(LotAt::NoLot, |(b, _)| LotAt::Building(*b))
    }
    fn city_across_connection(&self, x: i32, z: i32) -> Option<(i32, i32)> {
        self.neighbors.borrow().contains(&(x, z)).then_some((-1, 0))
    }
    fn notify(&self, sim: i32, sub_id: u32, data1: u32, data2: u32) {
        self.notifications.borrow_mut().push((sim, sub_id, data1, data2));
    }
}

// ================================================================ §4.7.1 (pure)

/// A typed job-search world for the §4.7.1 worked examples.
#[derive(Default)]
struct JW {
    wealth: Cell<u32>,
    /// (occupant or 0 for cell lookups, rect).
    lots: RefCell<Vec<(u64, CellRect)>>,
    cells: (i32, i32),
    lot_at: Cell<Option<LotAt>>,
    city: Cell<Option<(i32, i32)>>,
}

impl JobSearchWorld for JW {
    fn wealth(&self) -> u32 {
        self.wealth.get()
    }
    fn occupant_lot_rect(&self, occupant: u64) -> Option<CellRect> {
        self.lots.borrow().iter().find(|(o, _)| *o == occupant).map(|(_, r)| *r)
    }
    fn lot_rect_at(&self, x: i32, z: i32) -> Option<CellRect> {
        self.lots
            .borrow()
            .iter()
            .find(|(_, r)| (r.x1..=r.x2).contains(&x) && (r.z1..=r.z2).contains(&z))
            .map(|(_, r)| *r)
    }
    fn cell_is_in_bounds(&self, x: i32, z: i32) -> bool {
        x >= 0 && z >= 0 && x < self.cells.0 && z < self.cells.1
    }
    fn cell_counts(&self) -> (i32, i32) {
        self.cells
    }
    fn lot_at(&self, _x: i32, _z: i32) -> LotAt {
        self.lot_at.get().unwrap_or(LotAt::NoLot)
    }
    fn city_across_connection(&self, _x: i32, _z: i32) -> Option<(i32, i32)> {
        self.city.get()
    }
}

fn jw(wealth: u32) -> JW {
    let w = JW {
        cells: (64, 64),
        ..JW::default()
    };
    w.wealth.set(wealth);
    w.lots.borrow_mut().push((0x500, CellRect::new(10, 10, 12, 11)));
    w.lot_at.set(Some(LotAt::Building(0x900)));
    w
}

fn home_state() -> JobSearchState {
    JobSearchState {
        flags: FLAG_HAS_HOME,
        home: DispatchLocation::by_occupant(0x500),
        ..JobSearchState::default()
    }
}

fn found_pf(end: (i32, i32)) -> RecordingJobPathfinder {
    RecordingJobPathfinder {
        find_result: 1,
        end_cell: end,
        ..RecordingJobPathfinder::default()
    }
}

fn find_new_job_loc() -> DispatchLocation {
    DispatchLocation {
        by_commute: true,
        ..DispatchLocation::default()
    }
}

/// §4.7.1 example 1: wealth 2, first search, home lot rect (10,10)–(12,11).
#[test]
fn t30_job_search_setup() {
    let w = jw(2);
    let mut st = home_state();
    let mut pf = found_pf((20, 20));
    let (ok, notify) = set_current_commute_destination(&mut st, &mut pf, &w, find_new_job_loc(), true);
    assert!(ok && notify);
    assert_eq!(pf.flags, 0x11);
    assert_eq!(pf.filter, 0);
    assert_eq!(pf.dest_flags, 0x2);
    assert_eq!(pf.density, 1);
    assert_eq!(pf.start_rect, Some(CellRect::new(10, 10, 12, 11)));
    assert_eq!((pf.allowed, pf.starting), (0x1CF, 0x1CF));
    assert_eq!(
        pf.calls,
        vec![
            "Reset",
            "ClearAllTravelTypeFlags",
            "SetRouteInfo",
            "SetFlag",
            "SetFlag",
            "SetTripDestinationFilter",
            "SetDestinationFlag",
            "SetTrafficDensity",
            "SetStartRect",
            "SetTravelTypeFlag",
            "FindPath",
        ]
    );
    assert_eq!(pf.calls.iter().filter(|c| **c == "FindPath").count(), 1);
    // The stored destination: the end lot's building, by commute.
    let d = st.commute_destination;
    assert_eq!((d.occupant, d.by_occupant, d.by_commute), (Some(0x900), true, true));
}

/// §4.7.1 examples 2 and 3: wealth 1 then 3 → `0x1` then `0x5`; wealth 0
/// → `0x1`; flags stay `0x11`.
#[test]
fn t31_job_search_accumulates() {
    let mut st = home_state();
    let mut pf = found_pf((20, 20));
    let w = jw(1);
    set_current_commute_destination(&mut st, &mut pf, &w, find_new_job_loc(), true);
    assert_eq!(pf.dest_flags, 0x1);
    w.wealth.set(3);
    set_current_commute_destination(&mut st, &mut pf, &w, find_new_job_loc(), true);
    assert_eq!(pf.dest_flags, 0x5);
    assert_eq!(pf.flags, 0x11);
    for (wealth, bits) in [(0, 1), (1, 1), (2, 2), (3, 4), (4, 1), (u32::MAX, 1)] {
        assert_eq!(destination_flag_for_wealth(wealth), bits);
    }
}

/// §4.7.1 example 4: no home → only `resetPath`, false, no notify; the
/// path flags are cleared.
#[test]
fn t32_job_search_no_home() {
    let w = jw(2);
    let mut st = JobSearchState {
        flags: 0x800 | 0x1000 | 0x4000 | 0x100,
        ..JobSearchState::default()
    };
    let mut pf = found_pf((20, 20));
    assert_eq!(
        set_current_commute_destination(&mut st, &mut pf, &w, find_new_job_loc(), true),
        (false, false)
    );
    assert_eq!(pf.calls, vec!["Reset", "ClearAllTravelTypeFlags", "SetRouteInfo"]);
    assert_eq!(st.flags, 0x100);
    assert_eq!(st.commute_destination, DispatchLocation::default());
}

/// §4.7.1 example 5: end cell (−1, 37), neighbour city found, 64 × 64 →
/// cell (0, 37), no occupant, true.
#[test]
fn t33_job_search_neighbor() {
    let w = jw(2);
    w.city.set(Some((3, 4)));
    let mut st = home_state();
    let mut pf = found_pf((-1, 37));
    assert_eq!(
        set_current_commute_destination(&mut st, &mut pf, &w, find_new_job_loc(), true),
        (true, true)
    );
    let d = st.commute_destination;
    assert_eq!((d.occupant, d.by_occupant, d.cell), (None, false, (0, 37)));
    assert_eq!(d.neighbor_city, Some((3, 4)));
    // Clamped on the far side too; no city → false.
    let mut pf = found_pf((70, 64));
    set_current_commute_destination(&mut st, &mut pf, &w, find_new_job_loc(), true);
    assert_eq!(st.commute_destination.cell, (63, 63));
    w.city.set(None);
    let mut pf = found_pf((-1, 37));
    assert!(!set_current_commute_destination(&mut st, &mut pf, &w, find_new_job_loc(), true).0);
}

/// §4.7.1 example 6 and the other failures: lot without building, no lot,
/// `FindPath` ≠ 1.
#[test]
fn t34_job_search_failures() {
    let w = jw(2);
    for (lot, find) in [
        (LotAt::NoBuilding, 1),
        (LotAt::NoLot, 1),
        (LotAt::Building(9), 0),
        (LotAt::Building(9), 2),
    ] {
        w.lot_at.set(Some(lot));
        let mut st = home_state();
        let mut pf = found_pf((5, 5));
        pf.find_result = find;
        assert_eq!(
            set_current_commute_destination(&mut st, &mut pf, &w, find_new_job_loc(), true),
            (false, false)
        );
    }
}

/// §4.7.1 steps 1–2 of `SetCurrentCommuteDestination`: a valid location is
/// stored without a search; invalid + notify false → false; invalid +
/// state 2 → stored unvalidated.
#[test]
fn t35_set_current_commute_destination() {
    let w = jw(2);
    let mut st = home_state();
    let mut pf = found_pf((5, 5));
    let loc = DispatchLocation::by_occupant(0x42);
    assert_eq!(
        set_current_commute_destination(&mut st, &mut pf, &w, loc, false),
        (true, false)
    );
    assert!(pf.calls.is_empty());
    assert_eq!(st.commute_destination, loc);
    assert_eq!(
        set_current_commute_destination(&mut st, &mut pf, &w, find_new_job_loc(), false),
        (false, false)
    );
    st.state = STATE_STORE_UNVALIDATED;
    assert_eq!(
        set_current_commute_destination(&mut st, &mut pf, &w, find_new_job_loc(), true),
        (true, true)
    );
    assert!(pf.calls.is_empty());
    assert_eq!(st.commute_destination, find_new_job_loc());
    // Home by cell without a lot → the single cell as start rect.
    let mut st = JobSearchState {
        flags: FLAG_HAS_HOME,
        home: DispatchLocation::by_cell(30, 31),
        ..JobSearchState::default()
    };
    let mut pf = found_pf((5, 5));
    set_current_commute_destination(&mut st, &mut pf, &w, find_new_job_loc(), true);
    assert_eq!(pf.start_rect, Some(CellRect::new(30, 31, 30, 31)));
}

// ================================================================ §4.7.1 (dispatch)

/// The dispatch object: the long-lived pathfinder accumulates; no home →
/// false and no `0xEBA06BF4`; a found job notifies once.
#[test]
fn t36_dispatch_find_new_job() {
    let f = D::new();
    f.d.init(1);
    // No home (flag 0x80 clear).
    assert!(!f.d.find_new_job(1));
    assert!(f.take().is_empty());
    // Home lot 1 at (2,2); wealth 1 then 2 on the same dispatch.
    f.w.lot(1, CellRect::new(2, 2, 2, 2));
    f.d.set_default_destination(1, DispatchLocation::by_occupant(1));
    f.w.wealth.set(1);
    f.d.find_new_job(1);
    f.w.wealth.set(2);
    f.d.find_new_job(1);
    let (flags, filter, dest, density, start, allowed, starting) = f.d.job_pathfinder_config(1).unwrap();
    assert_eq!((flags, filter, dest, density), (0x11, 0, 0x3, 1));
    assert_eq!(start, CellRect::new(2, 2, 2, 2));
    assert_eq!((allowed, starting), (0x1CF, 0x1CF));
    // ChangeJob: a valid building is stored without search or notify.
    f.take();
    assert!(f.d.change_job(1, 7));
    assert!(f.take().is_empty());
    assert_eq!(
        f.d.get_state(1).search.commute_destination,
        DispatchLocation::by_occupant(7)
    );
    assert_eq!(
        f.d.job_destination(1),
        Some(JobDestination::Lot {
            building: 7,
            cell: f.d.get_state(1).search.end_cell
        })
    );
    // Shutdown drops the pathfinder.
    f.d.shutdown(1);
    assert!(f.d.job_pathfinder_config(1).is_none());
}

/// `CheckCommute` (§4.7): the total time to the job lot; a reset
/// destination → None.
#[test]
fn t37_check_commute() {
    let f = D::new();
    f.d.init(1);
    f.w.lot(2, CellRect::new(4, 4, 4, 4));
    assert!(f.d.change_job(1, 2));
    let t = f.d.check_commute(1);
    assert!(t.is_some_and(|v| v > 0.0 && v.is_finite()));
    f.d.reset_commute(1);
    assert!(f.d.check_commute(1).is_none());
    assert_eq!(f.d.job_destination(1), None);
}

/// The notifications (§3.3): trip state, flags, arrival, sim state.
#[test]
fn t38_notifications() {
    let f = D::new();
    f.d.init(1);
    f.d.report_trip_state(1, 3, 2);
    assert_eq!(f.d.get_state(1).trip_state, 3);
    f.d.report_flags(1, 0x11, 0);
    assert_eq!(f.d.get_state(1).search.flags, 0x11);
    f.d.report_arrival(1, true);
    f.d.report_sim_state(1, 5);
    assert_eq!(f.d.get_state(1).trip_state, 5);
    assert_eq!(
        f.take(),
        vec![
            (1, dispatch_note::TRIP_STATE_B, 2, 3),
            (1, dispatch_note::TRIP_STATE_A, 0, 0x11),
            (1, dispatch_note::ARRIVAL, 1, 0),
            (1, dispatch_note::STATE_1, 5, 0),
        ]
    );
    f.d.report_flags(1, 0x100, 0);
    assert!(f.d.check_flags(1, 0x100));
    assert!(!f.d.check_flags(1, 0x101));
    f.d.shutdown(1);
    assert_eq!(f.d.get_state(1), DispatchState::default());
}

/// The class and interface IDs (§1.1).
#[test]
fn t39_ids() {
    assert_eq!(DISPATCH_CLSID, 0xCBC1_4674);
    assert_eq!(DISPATCH_IID, 0xEB8B_6F9A);
    let f = D::new();
    assert_eq!(f.d.class_id(), DISPATCH_CLSID);
    assert_eq!(f.d.interface_id(), DISPATCH_IID);
}
