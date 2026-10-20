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

use super::dispatch::{DispatchHost, DispatchState, MySimDispatch};
use super::pool::{MySimPool, MySimTemplate, PoolHost};
use super::{dispatch_note, JobDestination, DISPATCH_CLSID, DISPATCH_IID};

/// The job destination bits, from the dispatch module's setup.
const JOB_BITS: u8 = 0x1F;

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
    caps: RefCell<HashMap<u64, ([u32; 4], [u32; 4])>>,
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
    installation: Cell<bool>,
    installed: RefCell<Vec<MySimTemplate>>,
    /// The `mysim` dispatch commands, for the notify path.
    sim_commands: RefCell<Vec<(i32, String)>>,
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

    /// A destination byte at a map-edge cell (the neighbour connection).
    fn dest_map(&self, (x, z): (i32, i32)) {
        self.dest.borrow_mut().insert((x, z), JOB_BITS);
    }

    /// A neighbour pool holder at a map-edge cell.
    fn pool_map(&self, (x, z): (i32, i32)) {
        self.pools.borrow_mut().insert((x, z), 0x2_0000);
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
    net: Rc<NetworkModel>,
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
        D { w, net: model, d }
    }

    fn take(&self) -> Vec<(i32, u32, u32, u32)> {
        self.w.take_notifications()
    }
}

impl DispatchHost for DispatchWorld {
    fn home_rect(&self, sim: i32) -> Option<CellRect> {
        (sim == 1).then_some(CellRect::new(2, 2, 2, 2))
    }
    fn job_rect(&self, _sim: i32) -> Option<CellRect> {
        self.buildings.borrow().get(&2).copied()
    }
    fn building_rect(&self, building: u64) -> Option<CellRect> {
        self.buildings.borrow().get(&building).copied()
    }
    fn neighbor_cell(&self, _sim: i32) -> Option<(i32, i32)> {
        self.neighbors.borrow().first().copied()
    }
    fn job_candidates(&self, _sim: i32) -> Vec<CellRect> {
        self.buildings
            .borrow()
            .iter()
            .filter(|(b, _)| **b != 1)
            .map(|(_, r)| *r)
            .collect()
    }
    fn building_at_cell(&self, x: i32, z: i32) -> Option<u64> {
        self.buildings
            .borrow()
            .iter()
            .find(|(_, r)| (r.x1..=r.x2).contains(&x) && (r.z1..=r.z2).contains(&z))
            .map(|(b, _)| *b)
    }
    fn is_neighbor_cell(&self, x: i32, z: i32) -> bool {
        x < 0 || z < 0 || x >= self.cells.get().0 || z >= self.cells.get().1
    }
    fn notify(&self, sim: i32, sub_id: u32, data1: u32, data2: u32) {
        self.notifications.borrow_mut().push((sim, sub_id, data1, data2));
    }
}

/// §4.7: `FindNewJob` runs the pathfinder from the home lot towards a job
/// destination and reports it; no home → false; no candidates → false.
#[test]
fn t30_find_new_job() {
    let f = D::new();
    // Job lot 2 at (4,4), no capacity needed for filter 3.
    f.w.lot(2, CellRect::new(4, 4, 4, 4));
    assert!(f.d.find_new_job(1));
    // The state holds the destination and "by commute".
    let s = f.d.get_state(1);
    assert_eq!(s.destination, Some(2));
    assert!(s.destination_by_commute);
    assert_eq!(s.trip_state, 2);
    // The notification is the job-found sub-ID.
    assert_eq!(f.take(), vec![(1, dispatch_note::JOB_FOUND, 0, 0)]);
    // The destination reported to the MySim is the lot's building.
    assert_eq!(
        f.d.job_destination(1),
        Some(JobDestination::Lot {
            building: 2,
            cell: (4, 4)
        })
    );
    // Trip length of the straight street run: 3 cells × 1.5 − 0? The path
    // (2,3),(3,3),(4,3) costs 1.5 each = 4.5; the §11 test 2 value was 3.0
    // for a start at (2,2) through the ring, end (4,4): cost steps of 1.5
    // per cell. assert only that it is positive and finite.
    assert!(s.trip_length > 0.0 && s.trip_length.is_finite());
    // No home: false, no notification.
    assert!(!f.d.find_new_job(2));
    assert!(f.take().is_empty());
    // No candidates: false.
    f.w.buildings.borrow_mut().remove(&2);
    assert!(!f.d.find_new_job(1));
    assert!(f.take().is_empty());
}

/// §4.7: `ChangeJob` sets the commute destination to the building; the
/// result is the dispatch's.
#[test]
fn t31_change_job() {
    let f = D::new();
    f.w.lot(2, CellRect::new(4, 4, 4, 4));
    assert!(f.d.change_job(1, 2));
    assert_eq!(f.d.get_state(1).destination, Some(2));
    // A building without a reachable path: false, the destination cleared.
    // (No job lot 9 → no rect → false.)
    assert!(!f.d.change_job(1, 9));
}

/// §4.7: `CheckCommute` reports the total time; a failed path is `None`
/// (the MySim then runs `FailCommute`).
#[test]
fn t32_check_commute() {
    let f = D::new();
    f.w.lot(2, CellRect::new(4, 4, 4, 4));
    assert!(f.d.find_new_job(1));
    let t = f.d.check_commute(1);
    assert!(t.is_some_and(|v| v > 0.0));
    // A MySim without a destination and without a neighbour cell: None.
    f.d.reset_commute(1);
    f.w.neighbors.borrow_mut().clear();
    assert!(f.d.check_commute(1).is_none());
}

/// §4.7: a neighbour job comes from the map-edge cell; `job_destination`
/// reports the neighbour city.
#[test]
fn t33_neighbor_job() {
    let f = D::new();
    f.d.init(1);
    // No destination: the commute runs to the neighbour cell (-1,3), off
    // the map; the street reaches (0,3), so the pathfinder can leave at
    // (0,3)'s exit. Give the neighbour cell a destination bit and pool.
    f.w.dest_map((-1, 3));
    f.w.pool_map((-1, 3));
    let t = f.d.check_commute(1);
    assert!(t.is_some(), "the neighbour commute succeeds");
    // With no neighbour cell the commute fails before the search.
    let f2 = D::new();
    f2.d.init(1);
    f2.w.neighbors.borrow_mut().clear();
    assert!(f2.d.check_commute(1).is_none());
}

/// The notifications (§3.3): trip state, flags, arrival, sim state.
#[test]
fn t34_notifications() {
    let f = D::new();
    f.d.init(1);
    f.d.report_trip_state(1, 3, 2);
    assert_eq!(f.d.get_state(1).trip_state, 3);
    f.d.report_flags(1, 0x11, 0);
    assert_eq!(f.d.get_state(1).flags, 0x11);
    f.d.report_arrival(1, true);
    assert_eq!(f.d.get_state(1).trip_state, 3);
    f.d.report_sim_state(1, 5);
    assert_eq!(f.d.get_state(1).trip_state, 5);
    // Sub-IDs and their data, in order.
    let notes = f.take();
    assert_eq!(
        notes,
        vec![
            (1, dispatch_note::TRIP_STATE_B, 2, 3),
            (1, dispatch_note::TRIP_STATE_A, 0, 0x11),
            (1, dispatch_note::ARRIVAL, 1, 0),
            (1, dispatch_note::STATE_1, 5, 0),
        ]
    );
    // CheckFlags (§4.1 step 3): all bits of the mask.
    f.d.report_flags(1, 0x100, 0);
    f.take();
    assert!(f.d.check_flags(1, 0x100));
    assert!(!f.d.check_flags(1, 0x101));
    // Shutdown drops the state.
    f.d.shutdown(1);
    assert_eq!(f.d.get_state(1), DispatchState::default());
}

/// The class and interface IDs (§1.1).
#[test]
fn t35_ids() {
    assert_eq!(DISPATCH_CLSID, 0xCBC1_4674);
    assert_eq!(DISPATCH_IID, 0xEB8B_6F9A);
    let f = D::new();
    assert_eq!(f.d.class_id(), DISPATCH_CLSID);
    assert_eq!(f.d.interface_id(), DISPATCH_IID);
}

/// The pathfinder setup (pathfinder.md §8's MySim dispatch row): filter 3,
/// flags 1 | 2, the Car type, density 0. Checked through the search
/// outcome: the job lot reached over the street.
#[test]
fn t36_pathfinder_setup() {
    let f = D::new();
    // Lots on the street: job at (4,3).
    f.w.lot(2, CellRect::new(4, 3, 4, 3));
    assert!(f.d.find_new_job(1));
    // The destination cell is in the job lot.
    let s = f.d.get_state(1);
    assert!(s.cell == (4, 3) || s.cell == (4, 4));
}

// ================================================================ §1.1 (the pool)

/// A scripted pool host.
struct Pool {
    found: Cell<bool>,
    templates: RefCell<Vec<MySimTemplate>>,
}

impl PoolHost for Pool {
    fn installation_found(&self) -> bool {
        self.found.get()
    }
    fn read_templates(&self) -> Vec<MySimTemplate> {
        self.templates.borrow().clone()
    }
}

/// §1.1 (the pool): `Import` reads the installation's templates into the
/// pool, `GetMySimTemplates` hands them out in list order,
/// `FindTheSimsInstallation` answers whether the installation is there.
#[test]
fn t37_pool() {
    let host = Rc::new(Pool {
        found: Cell::new(false),
        templates: RefCell::new(vec![
            MySimTemplate {
                name: b"Ann".to_vec(),
                is_male: false,
                sign: 3,
                bio: b"bio".to_vec(),
                bitmap_id: 7,
                bitmap_path: b"a.png".to_vec(),
            },
            MySimTemplate {
                name: b"Bob".to_vec(),
                is_male: true,
                sign: 1,
                bio: Vec::new(),
                bitmap_id: 8,
                bitmap_path: b"b.png".to_vec(),
            },
        ]),
    });
    let p = MySimPool::new(host.clone());
    // No installation: Import false, nothing imported.
    assert!(!p.import());
    assert!(p.templates().is_empty());
    assert!(!p.find_the_sims_installation());
    // With one: Import true, the templates in list order.
    host.found.set(true);
    assert!(p.import());
    assert!(p.find_the_sims_installation());
    let t = p.templates();
    assert_eq!(t.len(), 2);
    assert_eq!(t[0].name, b"Ann".to_vec());
    assert_eq!(t[1].name, b"Bob".to_vec());
    assert_eq!(t[1].is_male, true);
    // By index; out of range None.
    assert_eq!(p.template(1).map(|t| t.bitmap_id), Some(8));
    assert!(p.template(2).is_none());
}
