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
    destination_flag_for_wealth, set_current_commute_destination, set_state, world_to_cell, JobSearchState,
    JobSearchWorld, RecordingJobPathfinder, StateNote, FLAG_HAS_HOME, NEIGHBOR_VALID_CONNECTION_TYPES,
    STATE_STORE_UNVALIDATED,
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
    fn occupant_position(&self, occupant: u64) -> (f32, f32) {
        // The lot's first cell centre; an unknown occupant sits at (−1, −1).
        self.buildings
            .borrow()
            .get(&occupant)
            .map_or((-1.0, -1.0), |r| (r.x1 as f32 * 16.0 + 8.0, r.z1 as f32 * 16.0 + 8.0))
    }
    fn region_city(&self, x: i32, z: i32) -> Option<u64> {
        ((x, z) == (-1, 0)).then_some(77)
    }
    fn connections_of_type_to_city(&self, connection_type: u32, city: u64) -> usize {
        usize::from(city == 77 && connection_type == 2)
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
    /// Occupant positions (`GetPosition` x, z).
    positions: RefCell<Vec<(u64, (f32, f32))>>,
    /// Region cities: (position, handle).
    region: RefCell<Vec<((i32, i32), u64)>>,
    /// Neighbour connections: (type, city handle).
    connections: RefCell<Vec<(u32, u64)>>,
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
    fn occupant_position(&self, occupant: u64) -> (f32, f32) {
        let p = self.positions.borrow();
        p.iter().find(|(o, _)| *o == occupant).map_or((0.0, 0.0), |(_, xz)| *xz)
    }
    fn region_city(&self, x: i32, z: i32) -> Option<u64> {
        self.region.borrow().iter().find(|(p, _)| *p == (x, z)).map(|(_, c)| *c)
    }
    fn connections_of_type_to_city(&self, connection_type: u32, city: u64) -> usize {
        let c = self.connections.borrow();
        c.iter().filter(|(t, h)| *t == connection_type && *h == city).count()
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
        home: DispatchLocation::by_occupant(Some((0x500, (168.0, 168.0)))),
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
/// The found building's location comes from `SetByOccupant` (position
/// (175.9, 32.0) → cell (10, 2)) with byCommute restored to 1.
#[test]
fn t30_job_search_setup() {
    let w = jw(2);
    w.positions.borrow_mut().push((0x900, (175.9, 32.0)));
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
    let d = st.commute_destination;
    assert_eq!((d.occupant, d.in_city, d.by_commute), (Some(0x900), true, true));
    assert_eq!((d.world, d.cell, d.neighbor_city), ((175.9, 32.0), (10, 2), (-1, -1)));
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
    assert_eq!((d.occupant, d.in_city, d.cell), (None, false, (0, 37)));
    assert_eq!((d.neighbor_city, d.by_commute), ((3, 4), true));
    assert_eq!(d.world, (0.0, 592.0));
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
    let loc = DispatchLocation::by_occupant(Some((0x42, (48.0, 64.0))));
    assert_eq!(loc.cell, (3, 4));
    assert_eq!(
        set_current_commute_destination(&mut st, &mut pf, &w, loc, false),
        (true, false)
    );
    assert!(pf.calls.is_empty());
    assert_eq!(st.commute_destination, loc);
    // An occupant whose cells are out of the city is invalid (the occupant
    // is not consulted).
    let off = DispatchLocation::by_occupant(Some((0x42, (-1.0, 64.0))));
    assert_eq!(
        set_current_commute_destination(&mut st, &mut pf, &w, off, false),
        (false, false)
    );
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

/// §4.7.1 location contract: the default location, `IsValid` (in-city
/// bounds without the occupant; neighbour city through the connection
/// types, Ferry excluded) and `SetByOccupant`.
#[test]
fn t40_location_contract() {
    let w = jw(2);
    let d = DispatchLocation::default();
    assert_eq!(d.occupant, None);
    assert_eq!((d.world, d.cell, d.neighbor_city), ((-16.0, -16.0), (-1, -1), (-1, -1)));
    assert_eq!((d.in_city, d.by_commute, d.reserved), (true, false, (0, 0)));
    assert!(!d.is_valid(&w));
    let mut c = DispatchLocation::default();
    c.set_cell_position(3, 4);
    assert!(c.is_valid(&w));
    c.set_cell_position(64, 0);
    assert!(!c.is_valid(&w));
    c.set_cell_position(0, 63);
    assert!(c.is_valid(&w));
    // Out of the city: a region city with only a Ferry connection (type 5)
    // or power/pipe is invalid; with a Subway (6) one valid; no city → invalid.
    let mut n = DispatchLocation {
        in_city: false,
        neighbor_city: (2, 9),
        ..DispatchLocation::default()
    };
    assert!(!n.is_valid(&w));
    w.region.borrow_mut().push(((2, 9), 0xC1));
    w.connections.borrow_mut().push((5, 0xC1));
    w.connections.borrow_mut().push((4, 0xC1));
    w.connections.borrow_mut().push((6, 0xC2));
    assert!(!n.is_valid(&w));
    w.connections.borrow_mut().push((6, 0xC1));
    assert!(n.is_valid(&w));
    for t in NEIGHBOR_VALID_CONNECTION_TYPES {
        w.connections.borrow_mut().clear();
        w.connections.borrow_mut().push((t, 0xC1));
        assert!(n.is_valid(&w), "type {t}");
    }
    n.neighbor_city = (2, 8);
    assert!(!n.is_valid(&w));
    assert_eq!(NEIGHBOR_VALID_CONNECTION_TYPES, [3, 12, 8, 2, 10, 1, 7, 11, 6]);
    // SetByOccupant resets every field, incl. byCommute and the neighbour
    // city; a null occupant leaves cells −1.
    let mut s = DispatchLocation {
        by_commute: true,
        in_city: false,
        neighbor_city: (5, 5),
        reserved: (1, 2),
        ..DispatchLocation::default()
    };
    s.set_by_occupant(Some((0x77, (175.9, 32.0))));
    assert_eq!((s.occupant, s.world, s.cell), (Some(0x77), (175.9, 32.0), (10, 2)));
    assert_eq!(
        (s.in_city, s.by_commute, s.neighbor_city, s.reserved),
        (true, false, (-1, -1), (0, 0))
    );
    s.set_by_occupant(None);
    assert_eq!(s, DispatchLocation::default());
    assert!(!s.is_valid(&w));
    // floor, not truncation, below 0.
    assert_eq!(
        (world_to_cell(-0.5), world_to_cell(15.99), world_to_cell(16.0)),
        (-1, 0, 1)
    );
}

/// `setState` (§4.7.1): `+0x40`/`+0x44`, flag 2 at state 3, the
/// notifications in order.
#[test]
fn t41_set_state() {
    let mut st = JobSearchState::default();
    assert_eq!(
        set_state(&mut st, 3),
        vec![
            StateNote::Flags { old: 0, new: 2 },
            StateNote::TripState { old: 0, new: 3 }
        ]
    );
    assert_eq!((st.state, st.prev_state, st.flags), (3, 0, 2));
    assert_eq!(set_state(&mut st, 3), vec![]);
    assert_eq!(st.prev_state, 3);
    assert_eq!(set_state(&mut st, 2), vec![StateNote::TripState { old: 3, new: 2 }]);
    assert_eq!((st.state, st.prev_state, st.flags), (2, 3, 2));
    // Flag already set: only the state change.
    assert_eq!(set_state(&mut st, 3), vec![StateNote::TripState { old: 2, new: 3 }]);
}

// ================================================================ §4.7.1 (dispatch)

/// The dispatch object: the long-lived pathfinder accumulates; no home →
/// false and no `0xEBA06BF4`; `ChangeJob` stores a building in the city
/// without a search and notifies (example "ChangeJob(b)").
#[test]
fn t36_dispatch_find_new_job() {
    let f = D::new();
    f.d.init(1);
    // No home (flag 0x80 clear).
    assert!(!f.d.find_new_job(1));
    assert!(f.take().is_empty());
    // Home lot 1 at (2,2); wealth 1 then 2 on the same dispatch.
    f.w.lot(1, CellRect::new(2, 2, 2, 2));
    f.d.set_default_destination(1, DispatchLocation::by_occupant(Some((1, (40.0, 40.0)))));
    f.w.wealth.set(1);
    f.d.find_new_job(1);
    f.w.wealth.set(2);
    f.d.find_new_job(1);
    let config = f.d.job_pathfinder_config(1).unwrap();
    let (flags, filter, dest, density, start, allowed, starting) = config;
    assert_eq!((flags, filter, dest, density), (0x11, 0, 0x3, 1));
    assert_eq!(start, CellRect::new(2, 2, 2, 2));
    assert_eq!((allowed, starting), (0x1CF, 0x1CF));
    // ChangeJob: building 7 at (4,4) → stored, no pathfinder call, one
    // 0xEBA06BF4, true; at any trip state.
    f.w.lot(7, CellRect::new(4, 4, 4, 4));
    for s in [0, 2, 3] {
        f.d.set_state(1, s);
        f.take();
        assert!(f.d.change_job(1, Some(7)));
        assert_eq!(f.take(), vec![(1, dispatch_note::JOB_FOUND, 0, 0)]);
        assert_eq!(f.d.job_pathfinder_config(1).unwrap(), config);
    }
    let d = f.d.get_state(1).search.commute_destination;
    assert_eq!(
        (d.occupant, d.cell, d.in_city, d.by_commute),
        (Some(7), (4, 4), true, true)
    );
    assert_eq!(
        f.d.job_destination(1),
        Some(JobDestination::Lot {
            building: 7,
            cell: (4, 4)
        })
    );
    // A null occupant: invalid → the normal job search runs, which needs a
    // home (sim 2 has none: false, no notification); a building in the
    // city needs no home.
    f.d.init(2);
    assert!(!f.d.change_job(2, None));
    assert!(f.take().is_empty());
    assert!(f.d.change_job(2, Some(7)));
    assert_eq!(f.take(), vec![(2, dispatch_note::JOB_FOUND, 0, 0)]);
    // Shutdown drops the pathfinder.
    f.d.shutdown(1);
    assert!(f.d.job_pathfinder_config(1).is_none());
}

/// The default destination stored unvalidated at trip state 2 (the
/// `FailCommute` / `FindNewJob` location, §4.7.1 callers table) reports no
/// job destination. (`CheckCommute`'s pathfinder is not the dispatch's:
/// MS15.)
#[test]
fn t37_default_destination_at_state_2() {
    let f = D::new();
    f.d.init(1);
    f.w.lot(2, CellRect::new(4, 4, 4, 4));
    assert!(f.d.change_job(1, Some(2)));
    f.d.set_state(1, 2);
    assert!(f.d.find_new_job(1));
    assert_eq!(f.d.job_destination(1), None);
}

/// The notifications (§3.3): `setState` posts the flag and trip-state
/// changes; arrival and sim state only notify (they write no trip state).
#[test]
fn t38_notifications() {
    let f = D::new();
    f.d.init(1);
    f.d.set_state(1, 3);
    assert_eq!(f.d.get_state(1).search.state, 3);
    f.d.report_flags(1, 0x11, 0x2);
    assert_eq!(f.d.get_state(1).search.flags, 0x11);
    f.d.report_arrival(1, true);
    f.d.report_sim_state(1, 5);
    assert_eq!(f.d.get_state(1).search.state, 3);
    assert_eq!(
        f.take(),
        vec![
            (1, dispatch_note::TRIP_STATE_A, 0, 2),
            (1, dispatch_note::TRIP_STATE_B, 0, 3),
            (1, dispatch_note::TRIP_STATE_A, 2, 0x11),
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

#[test]
fn t42_ms13_arithmetic_and_home_rect() {
    let cases = [
        (0x432F_E666, 10),
        (0xC100_0000, -1),
        (0xC180_0000, -1),
        (0x8000_0000, 0),
        (0x8DA2_4260, 0),
        (0x477F_FFFD, 4095),
        (0x4C80_0000, 4_194_304),
        (0x4CBE_BC20, 5_222_152),
        (0xCCBE_BC20, -8_305_697),
        (0x7FC0_0000, 880_803_840),
        (0xFFC0_0000, -1_266_679_808),
        (0x7F80_0001, 880_803_841),
        (0x7F80_0000, 876_609_536),
        (0xFF80_0000, -1_270_874_112),
    ];
    for (bits, cell) in cases {
        assert_eq!(world_to_cell(f32::from_bits(bits)), cell, "{bits:08x}");
    }
    let mut loc = DispatchLocation {
        occupant: Some(77),
        by_commute: true,
        in_city: false,
        neighbor_city: (2, 9),
        reserved: (3, 4),
        ..DispatchLocation::default()
    };
    let old = loc;
    loc.set_cell_position(3, 37);
    assert_eq!(loc.world, (48.0, 592.0));
    assert_eq!(
        loc,
        DispatchLocation {
            cell: (3, 37),
            world: (48.0, 592.0),
            ..old
        }
    );
    loc.set_cell_position(0x1000_0001, 0);
    assert_eq!((loc.cell, loc.world), ((0x1000_0001, 0), (16.0, 0.0)));
    let w = jw(2);
    // Different occupant's lot covers the home cell: never fall back to it
    // when a home occupant exists but its own lot is absent (MS13 c).
    w.lots.borrow_mut()[0].0 = 0x999;
    let mut st = home_state();
    let mut pf = found_pf((1, 1));
    set_current_commute_destination(&mut st, &mut pf, &w, find_new_job_loc(), true);
    assert_eq!(pf.start_rect, Some(CellRect::new(10, 10, 10, 10)));
    st.home.occupant = None;
    set_current_commute_destination(&mut st, &mut pf, &w, find_new_job_loc(), true);
    assert_eq!(pf.start_rect, Some(CellRect::new(10, 10, 12, 11)));
}
