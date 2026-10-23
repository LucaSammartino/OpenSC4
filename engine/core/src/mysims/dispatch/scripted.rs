// SPDX-License-Identifier: GPL-3.0-or-later
//! In-memory foreign services shared by dispatch tests and conformance.
use super::{DispatchHost, LotAt, MySimDispatch};
use crate::grid::{CellRect, CityDims, CoreGrids, GridProvider};
use crate::pathfinder::{shared_work_check, PathfinderHost, TravelTables};
use crate::traffic_network::{net_type, travel, NetworkHost, NetworkModel, PathInfo, Tuning};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

pub type OwnerCallback = Rc<dyn Fn(i32, u32, u32, u32)>;

/// holder → (max[4], used[4]).
type Cap = ([u32; 4], [u32; 4]);

/// An 8 × 8 city whose street runs (0,3)…(4,3) (pathfinder.md §11
/// preamble): the MySim's home at (2,2), jobs along the street.
#[derive(Default)]
pub struct MemoryDispatchHost {
    /// The city cells.
    pub cells: Cell<(i32, i32)>,
    pub tables: Cell<TravelTables>,
    pub max_distance: Cell<u32>,
    pub attractiveness: Cell<f32>,
    pub strike: Cell<u8>,
    pub trip_scale: Cell<f32>,
    pub carpool: Cell<bool>,
    pub volumes: RefCell<HashMap<(u32, i32, i32), f32>>,
    pub dest: RefCell<HashMap<(i32, i32), u8>>,
    pub free: Cell<[u32; 4]>,
    /// holder → (max[4], used[4]).
    pub caps: RefCell<HashMap<u64, Cap>>,
    /// (lot token, bounding rect, holder).
    pub lots: RefCell<Vec<(u64, CellRect, u64)>>,
    pub pools: RefCell<HashMap<(i32, i32), u64>>,
    pub income: Cell<f64>,
    pub deadline_reached: Cell<bool>,
    /// (building, lot rect): the dispatch's building answers.
    pub buildings: RefCell<HashMap<u64, CellRect>>,
    /// The neighbour cells of the map edges.
    pub neighbors: RefCell<Vec<(i32, i32)>>,
    pub notifications: RefCell<Vec<(i32, u32, u32, u32)>>,
    pub on_notify: RefCell<Option<OwnerCallback>>,
    pub wealth: Cell<u32>,
}

impl MemoryDispatchHost {
    /// The §11 preamble: 8 × 8, car travel, MaxDistanceOriginToNetwork 1.
    pub fn new() -> Rc<MemoryDispatchHost> {
        let w = MemoryDispatchHost::default();
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
    pub fn street(&self, net: &mut crate::traffic_network::NetworkState, x: i32, z: i32) {
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

    pub fn lot(&self, token: u64, rect: CellRect) {
        self.lots.borrow_mut().push((token, rect, token + 0x1_0000));
        self.buildings.borrow_mut().insert(token, rect);
    }

    pub fn take_notifications(&self) -> Vec<(i32, u32, u32, u32)> {
        std::mem::take(&mut self.notifications.borrow_mut())
    }
}

impl GridProvider<u32> for MemoryDispatchHost {
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

impl GridProvider<u8> for MemoryDispatchHost {
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

impl NetworkHost for MemoryDispatchHost {
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

impl PathfinderHost for MemoryDispatchHost {
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
pub struct DispatchFixture {
    pub w: Rc<MemoryDispatchHost>,
    pub d: Rc<MySimDispatch>,
}

impl DispatchFixture {
    /// The city: street (0,3)…(4,3); home lot 1 at (2,2), job lots 2 and 3
    /// at (4,4) and (4,2) with commuter capacity; no neighbour pool.
    pub fn new() -> DispatchFixture {
        let w = MemoryDispatchHost::new();
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
        DispatchFixture { w, d }
    }

    pub fn take(&self) -> Vec<(i32, u32, u32, u32)> {
        self.w.take_notifications()
    }
}

impl DispatchHost for MemoryDispatchHost {
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
        let callback = self.on_notify.borrow().clone();
        if let Some(callback) = callback {
            callback(sim, sub_id, data1, data2);
        }
    }
}
impl Default for DispatchFixture {
    fn default() -> Self {
        Self::new()
    }
}
