// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests for the traffic simulator. Expected values are the ones
//! `docs/specs/traffic-simulator.md` states (section and §11 test numbers in
//! the comments), not values read back from the code. Where a test goes
//! beyond §11, the expected values are derived by hand from the cited
//! section.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use super::*;
use crate::grid::{CityDims, CoreGrids, GridProvider, TractGrid};
use crate::property::PropertyHolder;
use crate::rng::{FixedSeeds, RzRandomSuper};
use crate::stream::{ByteReader, ByteWriter, RefResolver, RegisteredObject, SaveRegistry};
use crate::traffic_network::{net_type, side, travel, NetworkHost, PathInfo};
use crate::variant::{mem, Elem, Variant};

// ---------------------------------------------------------------------------
// The stub game
// ---------------------------------------------------------------------------

/// A scripted lot.
#[derive(Clone, Debug)]
struct Lot {
    rect: CellRect,
    holder: u64,
    building: Option<u64>,
    zone: u32,
    livable: bool,
    road_access: bool,
    link: Option<u64>,
    /// developer type → travel population.
    pops: Vec<(u32, u32)>,
    strategy_byte: u8,
}

impl Lot {
    /// A lot of zone 0 (no trip generation) with a building `1000 + holder`.
    fn new(rect: CellRect, holder: u64) -> Lot {
        Lot {
            rect,
            holder,
            building: Some(1000 + holder),
            zone: 0,
            livable: true,
            road_access: true,
            link: None,
            pops: Vec::new(),
            strategy_byte: 0,
        }
    }
}

/// A scripted neighbour connection: `conn_type` `None` answers
/// `FindConnection` for any type (and reports type 0); `other_side` is
/// `FindConnectionOnOtherSide`; `city` the established city's position.
#[derive(Clone, Debug, Default)]
struct Conn {
    cell: (i32, i32),
    holder: u64,
    dirty: bool,
    conn_type: Option<u32>,
    other_side: Option<u64>,
    city: Option<(i32, i32)>,
}

/// A scripted network occupant (§3.6): the flag word (bit `1 << t` per
/// network type, as `IsOfType`; bit 31 the subway skip), the position
/// (occupant slot 8) and the network occupant's own cell (slot 59).
#[derive(Clone, Copy, Debug)]
struct Occ {
    flags: u32,
    pos: [f32; 3],
    cell: (i32, i32),
}

/// The department tokens are `DEPT_BASE + i`.
const DEPT_BASE: u64 = 0xD000;
/// The class ID of the host references written by the stub.
const HOST_CLASS: u32 = 0x1234_5678;

#[derive(Default)]
struct World {
    cells: Cell<(i32, i32)>,
    lots: RefCell<Vec<(u64, Lot)>>,
    conns: RefCell<Vec<(u64, Conn)>>,
    holders: RefCell<HashMap<u64, BTreeMap<u32, Variant>>>,
    trips: RefCell<BTreeMap<Source, Vec<TripData>>>,
    posted: RefCell<Vec<SimMessage>>,
    agents: RefCell<Vec<(u32, String, u32)>>,
    subs: RefCell<Vec<u32>>,
    exemplar: RefCell<Option<Rc<PropertyHolder>>>,
    day: Cell<u32>,
    date: Cell<u32>,
    clock: Cell<u64>,
    departments: RefCell<Vec<(u32, f32)>>,
    dept_base: Cell<u64>,
    funding: RefCell<HashMap<u64, f32>>,
    incomes: RefCell<Vec<(u64, u32, i64)>>,
    history: RefCell<Vec<(u32, u32)>>,
    workforce: Cell<f32>,
    dev_pop: RefCell<HashMap<(u32, i32, i32), i16>>,
    carpool: Cell<bool>,
    view: Cell<Option<CellRect>>,
    accidents: RefCell<Vec<(i32, i32, i64)>>,
    refs: RefCell<HashMap<u64, i32>>,
    released_departments: RefCell<Vec<u64>>,
    /// `create_line_item` calls (department, item, only if missing) (TG7).
    line_items: RefCell<Vec<(u64, dept::LineItem, bool)>>,
    occs: RefCell<HashMap<u64, Occ>>,
    /// `UpdateRoadsBudget` (false) / `UpdateMassTransitBudget` (true) calls
    /// with the cell counts they priced.
    costs: RefCell<Vec<(bool, [u32; 13])>>,
    /// The established neighbouring cities (traffic-network.md §2.7.4).
    cities: RefCell<Vec<NeighbourCity>>,
    /// The regional demand per wealth (§2.7.4 step 1).
    demand: RefCell<[Option<f32>; 3]>,
    /// The cost of a deactivated lot's `0xCA550301` budget item (§4.1.3).
    item_cost: Cell<i64>,
    /// The `0xCA550301` budget item of a building, `(department ID, cost)`
    /// (§4.16).
    query_items: RefCell<HashMap<u64, (u32, i64)>>,
    /// The language drives on the left (§4.4, TS17).
    drive_left: Cell<bool>,
}

fn inside(r: CellRect, x: i32, z: i32) -> bool {
    x >= r.x1 && x <= r.x2 && z >= r.z1 && z <= r.z2
}

impl World {
    /// An 8 × 8 city with the shipped tuning (§11 preamble).
    fn new() -> Rc<World> {
        let w = World::default();
        w.cells.set((8, 8));
        *w.exemplar.borrow_mut() = Some(Rc::new(SimTuning::spec_exemplar()));
        w.day.set(5);
        w.date.set(1);
        w.workforce.set(1.0);
        w.dept_base.set(DEPT_BASE);
        Rc::new(w)
    }

    fn add_lot(&self, id: u64, lot: Lot) {
        self.holders.borrow_mut().entry(lot.holder).or_default();
        self.lots.borrow_mut().push((id, lot));
    }

    fn lot(&self, id: u64) -> Option<Lot> {
        self.lots
            .borrow()
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, l)| l.clone())
    }

    fn prop(&self, holder: u64, id: u32) -> Option<Variant> {
        self.holders.borrow().get(&holder).and_then(|m| m.get(&id).cloned())
    }

    fn set_prop(&self, holder: u64, id: u32, v: Variant) {
        self.holders.borrow_mut().entry(holder).or_default().insert(id, v);
    }

    fn set_trips(&self, src: Source, trips: Vec<TripData>) {
        self.trips.borrow_mut().insert(src, trips);
    }

    fn get_trips(&self, src: Source) -> Vec<TripData> {
        self.trips.borrow().get(&src).cloned().unwrap_or_default()
    }

    /// A network occupant with `flags`, positioned in the middle of `cell`,
    /// whose own cell is `own` (§3.6).
    fn add_occ(&self, id: u64, flags: u32, cell: (i32, i32), own: (i32, i32)) {
        let pos = [cell.0 as f32 * 16.0 + 8.0, 0.0, cell.1 as f32 * 16.0 + 8.0];
        self.occs.borrow_mut().insert(id, Occ { flags, pos, cell: own });
    }

    fn occ_flags(&self, occ: u64) -> u32 {
        self.occs.borrow().get(&occ).map_or(0, |o| o.flags)
    }

    fn dims(&self) -> CityDims {
        let (x, z) = self.cells.get();
        CityDims {
            cells_x: x,
            cells_z: z,
            ..CityDims::square(1)
        }
    }
}

impl GridProvider<u32> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u32>>> {
        GridProvider::<u32>::create_grid(&CoreGrids { city: self.dims() }, instance, tract_size)
    }
}

impl GridProvider<u8> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        GridProvider::<u8>::create_grid(&CoreGrids { city: self.dims() }, instance, tract_size)
    }
}

impl GridProvider<f32> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<f32>>> {
        GridProvider::<f32>::create_grid(&CoreGrids { city: self.dims() }, instance, tract_size)
    }
}

impl NetworkHost for World {
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
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        self.occs.borrow().get(&occ).map_or([0.0; 3], |o| o.pos)
    }
    fn occupant_bounding_cells(&self, _occ: u64) -> CellRect {
        CellRect::new(0, 0, 0, 0)
    }
    fn prop_type(&self, _occ: u64) -> u32 {
        0
    }
    fn occupant_property(&self, _occ: u64, _id: u32) -> Option<Variant> {
        None
    }
    fn has_network_flag(&self, occ: u64, flag: u32) -> bool {
        self.occ_flags(occ) & flag == flag && self.occs.borrow().contains_key(&occ)
    }
    fn has_any_network_flag(&self, occ: u64, mask: u32) -> bool {
        self.occ_flags(occ) & mask != 0
    }
    fn is_of_type(&self, occ: u64, network_type: u32) -> bool {
        network_type < 32 && self.occ_flags(occ) & (1 << network_type) != 0
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
    fn lot_at(&self, x: i32, z: i32, anchor_only: bool) -> Option<u64> {
        self.lots
            .borrow()
            .iter()
            .find(|(_, l)| {
                if anchor_only {
                    (l.rect.x1, l.rect.z1) == (x, z)
                } else {
                    inside(l.rect, x, z)
                }
            })
            .map(|(id, _)| *id)
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        self.lot(lot).map_or(CellRect::new(0, 0, -1, -1), |l| l.rect)
    }
    fn lot_location(&self, lot: u64) -> (i32, i32) {
        self.lot(lot).map_or((0, 0), |l| (l.rect.x1, l.rect.z1))
    }
    fn lot_facing(&self, _lot: u64) -> u32 {
        0
    }
    fn lot_building(&self, lot: u64) -> Option<u64> {
        self.lot(lot).and_then(|l| l.building)
    }
    fn lot_populations(&self, _lot: u64) -> Vec<Vec<(u32, u32)>> {
        Vec::new()
    }
    fn lot_holder(&self, lot: u64) -> u64 {
        self.lot(lot).map_or(0, |l| l.holder)
    }
    fn holder_property(&self, holder: u64, id: u32) -> Option<Variant> {
        self.prop(holder, id)
    }
    fn set_holder_property(&self, holder: u64, id: u32, value: Variant) {
        self.set_prop(holder, id, value);
    }
    fn remove_holder_property(&self, holder: u64, id: u32) -> bool {
        self.holders
            .borrow_mut()
            .get_mut(&holder)
            .is_some_and(|m| m.remove(&id).is_some())
    }
    fn max_trip_capacity(&self, _holder: u64, _k: u32) -> u32 {
        panic!("the simulator answers max_trip_capacity itself")
    }
    fn neighbor_pool_holder(&self, _x: i32, _z: i32) -> Option<u64> {
        panic!("the simulator answers neighbor_pool_holder itself")
    }
    fn is_road_damaged(&self, _x: i32, _z: i32) -> bool {
        panic!("the simulator answers is_road_damaged itself")
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
        self.conns.borrow().iter().map(|(id, _)| *id).collect()
    }
    fn connection_cell(&self, conn: u64) -> (i32, i32) {
        self.conns
            .borrow()
            .iter()
            .find(|(id, _)| *id == conn)
            .map_or((0, 0), |(_, c)| c.cell)
    }
    fn post(&self, _msg_type: u32) {}
    fn add_ref(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_default() += 1;
    }
    fn release(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_default() -= 1;
    }
}

impl TrafficSimHost for World {
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        self.agents.borrow_mut().push((level, name.to_string(), flags));
    }
    fn subscribe(&self, msg_type: u32) {
        self.subs.borrow_mut().push(msg_type);
    }
    fn post_message(&self, msg: SimMessage) {
        self.posted.borrow_mut().push(msg);
    }
    fn object_ref(&self, token: u64) -> RefValue {
        RefValue::Host {
            token,
            class_id: HOST_CLASS,
        }
    }
    fn resolve_ref(&self, r: &RefValue, _iid: u32) -> Option<u64> {
        match r {
            RefValue::Host { token, .. } => Some(*token),
            _ => None,
        }
    }
    fn cell_width(&self) -> f32 {
        16.0
    }
    fn tuning_exemplar(&self) -> Option<Rc<PropertyHolder>> {
        self.exemplar.borrow().clone()
    }
    fn sim_day_of_month(&self) -> u32 {
        self.day.get()
    }
    fn sim_date_number(&self) -> u32 {
        self.date.get()
    }
    fn sim_clock_ms(&self) -> u64 {
        self.clock.get()
    }
    fn create_department(&self, id: u32, max_funding: f32) -> Option<u64> {
        let mut d = self.departments.borrow_mut();
        d.push((id, max_funding));
        Some(self.dept_base.get() + d.len() as u64 - 1)
    }
    fn release_department(&self, department: u64) {
        self.released_departments.borrow_mut().push(department);
    }
    fn create_line_item(&self, department: u64, item: dept::LineItem, only_if_missing: bool) {
        self.line_items.borrow_mut().push((department, item, only_if_missing));
    }
    /// Fixture stand-in (TS34): an unset department is at 100 % funding,
    /// as a department `CreateDepartmentBudget` makes (budget.md §6).
    fn department_funding(&self, department: u64) -> f32 {
        self.funding.borrow().get(&department).copied().unwrap_or(1.0)
    }
    fn set_line_item_income(&self, department: u64, item: u32, income: i64) {
        self.incomes.borrow_mut().push((department, item, income));
    }
    fn update_network_costs(&self, _department: u64, mass_transit: bool, cells: &[u32; 13], _costs: &[f32; 13]) {
        self.costs.borrow_mut().push((mass_transit, *cells));
    }
    fn activate_budget_items(&self, _building: u64, _lot: Option<u64>) -> i64 {
        0
    }
    fn deactivate_budget_items(&self, _holder: u64) -> i64 {
        self.item_cost.get()
    }
    fn budget_item_for_purpose(&self, building: u64, purpose: u32) -> Option<(u32, i64)> {
        if purpose != prop::BUDGET_ITEM {
            return None;
        }
        self.query_items.borrow().get(&building).copied()
    }
    fn department_budget(&self, department_id: u32) -> Option<u64> {
        let d = self.departments.borrow();
        d.iter()
            .rposition(|e| e.0 == department_id)
            .map(|i| self.dept_base.get() + i as u64)
    }
    fn record_history(&self, series: u32, value: u32) {
        self.history.borrow_mut().push((series, value));
    }
    fn lots(&self) -> Vec<u64> {
        self.lots.borrow().iter().map(|(id, _)| *id).collect()
    }
    fn as_lot(&self, object: u64) -> Option<u64> {
        self.lot(object).map(|_| object)
    }
    fn lot_zone(&self, lot: u64) -> u32 {
        self.lot(lot).map_or(0, |l| l.zone)
    }
    fn lot_is_livable(&self, lot: u64) -> bool {
        self.lot(lot).is_some_and(|l| l.livable)
    }
    fn lot_has_road_access(&self, lot: u64) -> bool {
        self.lot(lot).is_some_and(|l| l.road_access)
    }
    fn lot_link(&self, lot: u64) -> Option<u64> {
        self.lot(lot).and_then(|l| l.link)
    }
    fn lot_travel_population(&self, lot: u64, developer_type: u32) -> u32 {
        self.lot(lot)
            .and_then(|l| l.pops.iter().find(|p| p.0 == developer_type).map(|p| p.1))
            .unwrap_or(0)
    }
    fn lot_strategy_byte(&self, lot: u64) -> u8 {
        self.lot(lot).map_or(0, |l| l.strategy_byte)
    }
    fn clear_lot_flag(&self, _lot: u64, _flag: u32) {}
    fn trip_list(&self, source: Source) -> Vec<TripData> {
        self.get_trips(source)
    }
    fn set_trip_list(&self, source: Source, trips: Vec<TripData>) {
        self.set_trips(source, trips);
    }
    fn connection_holder(&self, connection: u64) -> u64 {
        self.conns
            .borrow()
            .iter()
            .find(|(id, _)| *id == connection)
            .map_or(0, |(_, c)| c.holder)
    }
    fn take_connection_volume_dirty(&self, connection: u64) -> bool {
        let mut conns = self.conns.borrow_mut();
        match conns.iter_mut().find(|(id, _)| *id == connection) {
            Some((_, c)) => std::mem::replace(&mut c.dirty, false),
            None => false,
        }
    }
    fn find_connection(&self, connection_type: u32, x: i32, z: i32) -> Option<u64> {
        self.conns
            .borrow()
            .iter()
            .find(|(_, c)| c.cell == (x, z) && c.conn_type.map_or(true, |t| t == connection_type))
            .map(|(id, _)| *id)
    }
    fn connection_type(&self, connection: u64) -> u32 {
        self.conns
            .borrow()
            .iter()
            .find(|(id, _)| *id == connection)
            .and_then(|(_, c)| c.conn_type)
            .unwrap_or(0)
    }
    fn connection_on_other_side(&self, connection: u64) -> Option<u64> {
        self.conns
            .borrow()
            .iter()
            .find(|(id, _)| *id == connection)
            .and_then(|(_, c)| c.other_side)
    }
    fn established_city_position(&self, connection: u64) -> Option<(i32, i32)> {
        self.conns
            .borrow()
            .iter()
            .find(|(id, _)| *id == connection)
            .and_then(|(_, c)| c.city)
    }
    fn connected_cities(&self) -> Vec<NeighbourCity> {
        self.cities.borrow().clone()
    }
    fn regional_demand(&self, w: u32) -> Option<f32> {
        self.demand.borrow().get(w as usize).copied().flatten()
    }
    fn workforce_percent(&self, _x: i32, _z: i32) -> f32 {
        self.workforce.get()
    }
    fn developer_population(&self, developer_type: u32, x: i32, z: i32) -> i16 {
        self.dev_pop.borrow().get(&(developer_type, x, z)).copied().unwrap_or(0)
    }
    fn is_ordinance_on(&self, id: u32) -> bool {
        id == CARPOOL_ORDINANCE && self.carpool.get()
    }
    fn ordinances_on(&self) -> Vec<u32> {
        Vec::new()
    }
    fn ordinance_property(&self, _id: u32, _prop: u32) -> Option<Variant> {
        None
    }
    fn as_network_occupant(&self, object: u64) -> Option<u64> {
        self.occs.borrow().contains_key(&object).then_some(object)
    }
    fn network_occupant_cell(&self, occ: u64) -> (i32, i32) {
        self.occs.borrow().get(&occ).map_or((0, 0), |o| o.cell)
    }
    fn occupants_in_cells(&self, _rect: CellRect) -> Vec<u64> {
        Vec::new()
    }
    fn occupant_filter_includes(&self, _filter: u64, _occupant: u64) -> bool {
        false
    }
    /// Fixture stand-in (TS34): every cell reports a network occupant.
    fn cell_has_network_occupant(&self, _x: i32, _z: i32) -> bool {
        true
    }
    fn drives_on_right(&self) -> bool {
        !self.drive_left.get()
    }
    fn view_cell_rect(&self) -> Option<CellRect> {
        self.view.get()
    }
    fn create_accident(&self, x: i32, z: i32, duration_ms: i64) {
        self.accidents.borrow_mut().push((x, z, duration_ms));
    }
}

/// An initialised simulator over `w`.
fn sim(w: &Rc<World>) -> Rc<TrafficSimulator> {
    let s = TrafficSimulator::new(w.clone(), 0);
    assert!(s.init());
    s
}

/// The §11 test 10 street: road info `{invCapacity 0.0625, type Street}` at
/// (0,3)…(4,3), connection count 3.
fn street(s: &TrafficSimulator) {
    s.network().update_state(|n| {
        for x in 0..=4 {
            let r = n.append_info(1, x, 3).unwrap();
            r.inv_cap = 0.0625;
            r.net_type = net_type::STREET as u8;
            r.conn_count = 3;
        }
    });
}

/// The trip capacity property `0x8A0B4100` of a holder (12 × u16).
fn set_capacity(w: &World, holder: u64, vals: &[u16]) {
    let mut v = vals.to_vec();
    v.resize(12, 0);
    w.set_prop(
        holder,
        prop::TRIP_CAPACITY,
        Variant::array(mem::UINT16, v.into_iter().map(Elem::U16).collect()),
    );
}

fn capacity(w: &World, holder: u64, i: usize) -> u32 {
    adapters::capacity_element(w, holder, i)
}

fn volume(s: &TrafficSimulator, n: usize, x: i32, z: i32) -> f32 {
    s.volume_grid(n).unwrap().get(x, z)
}

fn u8_prop(w: &World, holder: u64, id: u32) -> Option<u8> {
    w.prop(holder, id)
        .and_then(|v| v.items().first().map(|e| e.as_i64() as u8))
}

const ROUTE_10: [u8; 6] = [0x01, 0x02, 0x03, 0x41, 0x02, 0x0A];

fn bits(v: f32) -> u32 {
    v.to_bits()
}

// ---------------------------------------------------------------------------
// §11 conformance tests
// ---------------------------------------------------------------------------

/// §11 test 1: the cycle order, with and without the subnetwork rebuild,
/// and the days of `DoDailySimulation`.
#[test]
fn t01_cycle_order() {
    for stale in [false, true] {
        let w = World::new();
        let s = sim(&w);
        assert_eq!(s.cycle_state(), 0);
        let mut seen = vec![s.cycle_state()];
        loop {
            s.network().update_state(|n| n.subnetworks_stale = stale);
            assert!(
                s.work_on_sim_cycle(),
                "every phase of an empty city finishes in one call"
            );
            seen.push(s.cycle_state());
            if s.cycle_state() == 0 {
                break;
            }
        }
        let mut want = vec![0, 16, 17, 18, 15, 2, 4, 5, 11, 3, 7, 6, 10, 12, 13, 19, 8, 0];
        if !stale {
            want.retain(|&x| x != 12);
        }
        assert_eq!(seen, want);
    }
    // DoDailySimulation returns true on entering 17, 18, 2 and 11.
    let w = World::new();
    let s = sim(&w);
    s.network().update_state(|n| n.subnetworks_stale = false);
    let mut ends = Vec::new();
    for _ in 0..5 {
        assert!(s.do_daily_simulation());
        ends.push(s.cycle_state());
    }
    assert_eq!(ends, vec![17, 18, 2, 11, 17]);
}

/// TS64 (traffic-network.md §4.12, traffic-simulator.md §3.2): entering state
/// 13 resets the scanning iterator (the member `+0x818`, the one saved in §6.1
/// row 14) to state 0 at (0, 0) with its rectangle and step kept; entering 12
/// does not touch it; `UpdateSubnetworkInfo` tests the deadline **before each
/// cell**, so a stopped call leaves the iterator running at the last cell
/// visited; a finished pass rests in state 2 at `(x0, z1 + 1)`.
#[test]
fn ts64_state_13_scanning_iterator() {
    use crate::grid::ScanState;

    let w = World::new();
    let s = sim(&w);
    let (x0, z0, x1, z1) = s.read_state(|st| (st.scan.x0, st.scan.z0, st.scan.x1, st.scan.z1));
    assert_eq!((x0, z0, x1, z1), (0, 0, 7, 7), "an 8x8 city scans its whole rect");

    // Mid-scan, as a save taken mid-scan would leave it.
    s.update_state(|st| {
        st.scan.x = 3;
        st.scan.z = 5;
        st.scan.state = ScanState::Running;
    });
    // TS64: entering 12 does not touch the iterator (12 does not use it).
    s.run(|r| r.enter_state(cycle_state::UPDATE_SUBNETWORKS));
    assert_eq!(
        s.read_state(|st| (st.scan.x, st.scan.z, st.scan.state)),
        (3, 5, ScanState::Running),
        "TS64: entering 12 leaves the scanning iterator alone"
    );
    // TS64: entering 13 resets it to state 0 at (0, 0), rectangle and step
    // kept.
    s.run(|r| r.enter_state(cycle_state::SUBNETWORK_INFO));
    assert_eq!(
        s.read_state(|st| (st.scan.x, st.scan.z, st.scan.state, st.scan.x1, st.scan.z1)),
        (0, 0, ScanState::Fresh, x1, z1),
        "TS64: entering 13 resets the iterator to (0, 0)"
    );
    // A whole pass: the iterator ends in state 2 at (x0, z1 + 1).
    s.update_state(|st| st.cycle_state = cycle_state::SUBNETWORK_INFO);
    assert!(s.work_on_sim_cycle(), "an empty city's state 13 finishes in one call");
    assert_eq!(
        s.read_state(|st| (st.scan.x, st.scan.z, st.scan.state)),
        (x0, z1 + 1, ScanState::Done),
        "TS64: a finished pass rests at (x0, z1 + 1)"
    );

    // TS64: the deadline test is before each step, so a call stops between two
    // cells and the iterator reads state 1 at the last cell visited.
    s.run(|r| r.enter_state(cycle_state::SUBNETWORK_INFO));
    s.update_state(|st| {
        st.cycle_state = cycle_state::SUBNETWORK_INFO;
        st.shadow_scan_stop = Some((cycle_state::SUBNETWORK_INFO, 3, 5));
    });
    assert!(!s.work_on_sim_cycle(), "the deadline stop ends the call unfinished");
    assert_eq!(
        s.read_state(|st| (st.scan.x, st.scan.z, st.scan.state)),
        (3, 5, ScanState::Running),
        "TS64: a stopped call leaves the iterator running at the last cell visited"
    );
    // The next call continues from it.
    s.update_state(|st| st.shadow_scan_stop = None);
    assert!(s.work_on_sim_cycle(), "the next call finishes the pass");
    assert_eq!(
        s.read_state(|st| (st.scan.x, st.scan.z, st.scan.state)),
        (x0, z1 + 1, ScanState::Done),
        "TS64: the next call continues from the iterator and rests at (x0, z1 + 1)"
    );
}

/// Ten zone-0 lots (no trip generation) with four status-0 trips each.
fn ten_sources(w: &World) {
    for i in 0..10u64 {
        let id = 100 + i;
        w.add_lot(id, Lot::new(CellRect::new(i as i32 % 8, 6, i as i32 % 8, 6), 500 + i));
        w.set_trips(Source::Lot(id), vec![TripData::new(1, 0, 0, 0, status::SEARCH); 4]);
    }
}

/// §11 test 2: the day pacing.
#[test]
fn t02_day_pacing() {
    // N = 3: day 1 = sources 1–3 (12 < 20) and 4–6 (24 ≥ 20); day 2 =
    // sources 7–9 (36 < 40) and source 10, then 17 is entered.
    let w = World::new();
    ten_sources(&w);
    let s = sim(&w);
    s.set_traffic_units_per_slice(3);
    s.update_state(|st| st.pass_total = 100);
    assert!(!s.do_daily_simulation());
    assert_eq!(
        s.read_state(|st| (st.total, st.days, st.processed, st.source_pos)),
        (100, 5, 12, 3)
    );
    assert!(s.do_daily_simulation());
    assert_eq!(
        s.read_state(|st| (st.processed, st.day_index, st.source_pos)),
        (24, 1, 6)
    );
    w.day.set(6);
    assert!(!s.do_daily_simulation());
    assert_eq!(s.read_state(|st| (st.processed, st.source_pos)), (36, 9));
    assert!(s.do_daily_simulation());
    assert_eq!(s.cycle_state(), cycle_state::REROUTE_TRIPS);
    // toReRoute = 0 (no routed trips) → days 1.
    assert_eq!(s.read_state(|st| (st.total, st.days, st.to_search)), (0, 1, 40));

    // N = 1: day 1 ends after source 5 (processed 20).
    let w = World::new();
    ten_sources(&w);
    let s = sim(&w);
    s.set_traffic_units_per_slice(1);
    s.update_state(|st| st.pass_total = 100);
    let mut calls = 0;
    while !s.do_daily_simulation() {
        calls += 1;
    }
    assert_eq!(calls, 4);
    assert_eq!(
        s.read_state(|st| (st.processed, st.source_pos, st.day_index)),
        (20, 5, 1)
    );

    // toReRoute = 17 → days = 1.
    let w = World::new();
    let s = sim(&w);
    s.update_state(|st| {
        st.cycle_state = cycle_state::CONFIRM_TRIPS;
        st.to_reroute = 17;
    });
    assert!(s.do_daily_simulation());
    assert_eq!(s.read_state(|st| (st.total, st.days)), (17, 1));
}

/// §11 test 3: the shipped congestion curve after Init.
#[test]
fn t03_congestion_scale() {
    let w = World::new();
    let s = sim(&w);
    let (pts, scale) = s.read_state(|st| (st.tuning.congestion_curve.points(), st.tuning.congestion_scale));
    let pts: Vec<(u32, u32)> = pts.iter().map(|&(x, y)| (bits(x), bits(y))).collect();
    assert_eq!(
        pts,
        vec![
            (bits(0.0), bits(1.0)),
            (bits(1.0), bits(1.0)),
            (bits(2.0), 0x3FC4_EC4F),
            (bits(3.0), 0x4055_5555)
        ]
    );
    assert_eq!(bits(scale), 0x42DA_924A);
}

/// §11 test 4: the congestion map values.
#[test]
fn t04_congestion_map() {
    let t = {
        let w = World::new();
        let s = sim(&w);
        s.read_state(|st| st.tuning.clone())
    };
    let c = |d: f32| maps::congestion_byte(1.0, d, &t.congestion_curve, t.congestion_scale);
    assert_eq!(bits(t.congestion_curve.eval(1.5)), 0x3FA2_7628);
    assert_eq!(bits(t.congestion_curve.eval(2.5)), 0x401B_E5BE);
    assert_eq!(
        [c(0.5), c(1.0), c(1.5), c(2.0), c(2.5), c(3.0), c(4.0)],
        [1, 1, 29, 59, 157, 255, 255]
    );
    assert_eq!(
        maps::congestion_byte(0.0, 2.5, &t.congestion_curve, t.congestion_scale),
        0
    );
}

/// §11 test 5: the air-polluting map values.
#[test]
fn t05_air_polluting_map() {
    let a = maps::air_polluting_byte;
    assert_eq!(
        [a(150.0), a(15.0), a(25.0), a(2.0), a(3000.0), a(0.0)],
        [15, 2, 2, 0, 255, 0]
    );
}

/// §11 test 6: the trip-length map values, and a failed trip.
#[test]
fn t06_trip_length_map() {
    let b = |avg: f32| trips::trip_length_byte(avg, 6.0);
    assert_eq!([b(3.0), b(0.6), b(1.5), b(6.0), b(7.0)], [128, 26, 64, 255, 255]);
    // A lot with any failed trip → 0.
    let w = World::new();
    w.add_lot(1, Lot::new(CellRect::new(2, 2, 2, 2), 11));
    let mut ok = TripData::new(10, 0, 1, 0, status::ROUTED);
    ok.trip_length = 3.0;
    let failed = TripData::new(5, 0, 1, 0, status::FAILED);
    w.set_trips(Source::Lot(1), vec![ok, failed]);
    let s = sim(&w);
    s.trip_length_map().unwrap().set(2, 2, 77);
    s.complete_trip_source_calculations(Source::Lot(1));
    assert_eq!(s.trip_length_map().unwrap().get(2, 2), 0);
}

/// §11 test 7: the strategy percentages and the small-lot thresholds.
#[test]
fn t07_strategy_percentages() {
    let w = World::new();
    let s = sim(&w);
    s.update_state(|st| st.tuning.pct[1] = [80, 20, 0]);
    let p = s.travel_strategy_percentages(1);
    assert_eq!(p.map(bits), [0x3F4C_CCCC, 0x3E4C_CCCC, 0]);
    assert_eq!(trips::residential_split(30, p, 80), [30, 0, 0]);
    assert_eq!(trips::residential_split(30, p, 81), [0, 30, 0]);
    let q = s.update_state(|st| {
        st.tuning.pct[1] = [0, 50, 50];
        st.tuning.strategy_percentages(1)
    });
    assert_eq!(trips::residential_split(30, q, 0), [30, 0, 0]);
}

/// §11 test 8: the large-lot split.
#[test]
fn t08_large_lot_split() {
    let t = SimTuning {
        pct: [[0; 3], [80, 20, 0], [30, 0, 70], [0; 3]],
        ..SimTuning::default()
    };
    assert_eq!(
        trips::residential_split(200, t.strategy_percentages(1), 0),
        [160, 40, 0]
    );
    assert_eq!(trips::residential_split(75, t.strategy_percentages(2), 0), [22, 0, 52]);
}

/// §11 test 9: the carpool factor on the car volume.
#[test]
fn t09_carpool() {
    for (count, q) in [(10u32, 8.0f32), (7, 6.0), (3, 3.0), (1, 1.0)] {
        let w = World::new();
        w.carpool.set(true);
        let s = sim(&w);
        street(&s);
        s.apply_route(&ROUTE_10, count, CellRect::new(2, 2, 2, 2), (4, 4), 0, 0);
        assert_eq!(bits(volume(&s, 1, 2, 3)), bits(q), "count {count}");
    }
}

/// §11 test 10 city: the street, lot A at (2,2) and job lot B at (4,4) with
/// `max[0] = 100`.
fn test10() -> (Rc<World>, Rc<TrafficSimulator>) {
    let w = World::new();
    w.add_lot(1, Lot::new(CellRect::new(2, 2, 2, 2), 11));
    w.add_lot(2, Lot::new(CellRect::new(4, 4, 4, 4), 12));
    set_capacity(&w, 12, &[100]);
    let s = sim(&w);
    street(&s);
    (w, s)
}

fn current(s: &TrafficSimulator, x: u8, z: u8, evening: bool) -> [u16; 16] {
    s.traffic_edge_density(x, z, 1, evening).current
}

fn one(i: usize, v: u16) -> [u16; 16] {
    let mut c = [0; 16];
    c[i] = v;
    c
}

/// §11 test 10: the hand-checkable network.
#[test]
fn t10_hand_checkable_network() {
    let (w, s) = test10();
    s.apply_route(&ROUTE_10, 40, CellRect::new(2, 2, 2, 2), (4, 4), 0, 0);
    for x in 2..=4 {
        assert_eq!(bits(volume(&s, 1, x, 3)), 0x4220_0000);
    }
    assert_eq!(current(&s, 2, 3, false), one(4 + 2, 40));
    assert_eq!(current(&s, 3, 3, false), one(2, 40));
    assert_eq!(current(&s, 4, 3, false), one(3, 40));
    assert_eq!(current(&s, 4, 3, true), [0; 16]);
    assert_eq!(capacity(&w, 12, 4), 40);
    assert_eq!(s.read_state(|st| st.income_acc.get()), 0.0);

    // Mode 2.
    let (_, s2) = test10();
    s2.apply_route(&ROUTE_10, 40, CellRect::new(2, 2, 2, 2), (4, 4), 2, 0);
    for x in 2..=4 {
        assert_eq!(volume(&s2, 1, x, 3), 80.0);
    }
    assert_eq!(current(&s2, 2, 3, false), one(4 + 2, 40));
    assert_eq!(current(&s2, 3, 3, false), one(2, 40));
    assert_eq!(current(&s2, 4, 3, false), one(3, 40));
    assert_eq!(current(&s2, 2, 3, true), one(4 * 2 + 1, 40));
    assert_eq!(current(&s2, 3, 3, true), one(4 * 2, 40));
    assert_eq!(current(&s2, 4, 3, true), one(4 * 3, 40));

    // CreateMaps then CreateFinalMap.
    s.enter_cycle_state(cycle_state::CREATE_MAPS);
    assert!(s.work_on_sim_cycle());
    assert_eq!(s.cycle_state(), cycle_state::CREATE_FINAL_MAP);
    assert_eq!(s.peak_ratio_grid().unwrap().get(3, 3), 2.5);
    assert_eq!(s.peak_volume_grid().unwrap().get(3, 3), 40.0);
    assert_eq!(volume(&s, 1, 3, 3), 0.0);
    // The statistics (§4.9, TS9): all five street records count; r = 2.5
    // > 1 on the three route cells (congested 3), r = 0 on (0, 3) and
    // (1, 3); the average is 7.5 / 5 = 1.5.
    let (records, row) = s.read_state(|st| {
        let t = net_type::STREET as usize;
        (st.stats_last.records[t], st.stats_last.rows[t])
    });
    assert_eq!(records, 5);
    assert_eq!((row.sum, row.congested, row.max), (1.5, 3, 2.5));
    assert!(s.work_on_sim_cycle());
    assert_eq!(s.congestion_map().unwrap().get(3, 3), 157);
    assert_eq!(s.air_polluting_traffic_map().unwrap().get(3, 3), 4);
    assert!(w
        .posted
        .borrow()
        .contains(&SimMessage::of_type(msg::TRAFFIC_MAP_CHANGED)));

    // CompleteTripSourceCalculations for A.
    let mut t = TripData::new(40, 0, 1, 0, status::ROUTED);
    t.trip_length = 3.0;
    t.travel_types_used = 0x2;
    w.set_trips(Source::Lot(1), vec![t]);
    s.complete_trip_source_calculations(Source::Lot(1));
    assert_eq!(s.trip_length_map().unwrap().get(2, 2), 128);
    s.read_state(|st| {
        assert_eq!((st.attempted[0], st.routed[0]), (40, 40));
        assert_eq!((st.car_len_sum, st.car_len_count), (120.0, 40));
        assert_eq!(st.type_count_building[1], 40);
        assert_eq!(st.pass_total, 1);
    });
    assert_eq!(w.get_trips(Source::Lot(1))[0].flags, trip_flag::ROUTED_LAST_PASS);
    s.update_state(|st| st.cycle_state = cycle_state::FINAL_CALCULATIONS);
    assert!(s.work_on_sim_cycle());
    assert_eq!(s.read_state(|st| st.avg_car_commute), 3.0);
    s.do_monthly_simulation();
    assert!(w.history.borrow().contains(&(history::COMMUTE, 75)));
}

/// §11 test 11: the background traffic.
#[test]
fn t11_background_traffic() {
    let (w, s) = test10();
    w.dev_pop.borrow_mut().insert((0x1010, 2, 3), 100);
    w.dev_pop.borrow_mut().insert((0x3110, 2, 3), 10);
    assert_eq!(bits(s.background_traffic(2, 3)), 0x40E0_0000);
    s.enter_cycle_state(cycle_state::CREATE_MAPS);
    assert!(s.work_on_sim_cycle());
    assert_eq!(volume(&s, 1, 2, 3), 7.0);
    assert_eq!(volume(&s, 1, 3, 3), 0.0);
    s.enter_cycle_state(cycle_state::CREATE_MAPS);
    assert!(s.work_on_sim_cycle());
    assert_eq!(s.peak_volume_grid().unwrap().get(2, 3), 7.0);
    // A road cell without traffic still has peak volume 1.
    assert_eq!(s.peak_volume_grid().unwrap().get(3, 3), 1.0);
}

fn seed(s: &TrafficSimulator, v: u32) {
    let seeds = FixedSeeds {
        wall_clock: v,
        process_clock: v,
    };
    s.update_state(|st| st.rng = RzRandomSuper::new(v, &seeds));
}

/// §11 test 12: the seeded draws.
#[test]
fn t12_seeded_draws() {
    let w = World::new();
    for (i, x) in [1, 2, 3].into_iter().enumerate() {
        w.add_lot(10 + i as u64, Lot::new(CellRect::new(x, 1, x, 1), 20 + i as u64));
    }
    w.add_lot(13, Lot::new(CellRect::new(5, 5, 5, 5), 23));
    let s = sim(&w);
    seed(&s, 1);
    s.update_state(|st| {
        st.buckets.insert(0, vec![(1, 1), (2, 1), (3, 1)]);
        st.strategy_bucket = 0;
    });
    // Date number 2: 2 mod (365 / 128) = 0 → the re-draw.
    w.date.set(2);
    s.do_daily_simulation();
    let drawn: Vec<Option<u8>> = (20..23).map(|h| u8_prop(&w, h, prop::STRATEGY_DRAW)).collect();
    assert_eq!(drawn, vec![Some(82), Some(93), Some(5)]);
    assert_eq!(s.read_state(|st| st.strategy_bucket), 1);
    // The generator continues 13, 78, 34.
    let more: Vec<u32> = s.update_state(|st| (0..3).map(|_| st.rng.uniform_u32(100)).collect());
    assert_eq!(more, vec![13, 78, 34]);

    // Lot activation draws from the same generator (§4.1.3 step 2).
    seed(&s, 1);
    s.update_state(|st| {
        for _ in 0..3 {
            st.rng.uniform_u32(100);
        }
    });
    s.do_message(&Message {
        msg_type: msg::LOT_STATE_CHANGED,
        data2: 3,
        object: 13,
        ..Message::default()
    });
    assert_eq!(u8_prop(&w, 23, prop::STRATEGY_DRAW), Some(13));

    // Seed 12345.
    seed(&s, 12345);
    let v: Vec<u32> = s.update_state(|st| (0..6).map(|_| st.rng.uniform_u32(100)).collect());
    assert_eq!(v, vec![81, 12, 25, 77, 2, 59]);

    // RandomDoubleUniform and CheckRailAccident.
    seed(&s, 1);
    let u: Vec<u32> = s.update_state(|st| (0..3).map(|_| bits(st.rng.uniform_f64() as f32)).collect());
    assert_eq!(u, vec![0x3EA7_ED74, 0x3EDE_5083, 0x3F0D_363A]);
    s.update_damage(|d| d.rail.damaged.insert(u32::from(cell_key16(3, 3))));
    for (factor, want) in [(1.0f32, [true, true, true]), (0.5, [true, true, false])] {
        seed(&s, 1);
        s.update_state(|st| st.tuning.rail_damage_accident_factor = factor);
        let got: Vec<bool> = (0..3).map(|_| s.check_rail_accident(3, 3)).collect();
        assert_eq!(got, want.to_vec(), "factor {factor}");
    }
    // An undamaged cell draws nothing.
    seed(&s, 1);
    assert!(!s.check_rail_accident(4, 4));
    assert_eq!(s.update_state(|st| st.rng.next_u32()), 0xD3F6_B9E5);
}

/// §11 test 13: the destination capacity.
#[test]
fn t13_capacity() {
    let w = World::new();
    w.add_lot(1, Lot::new(CellRect::new(4, 4, 5, 5), 12));
    set_capacity(&w, 12, &[0, 30, 0, 0, 0, 25]);
    let s = sim(&w);
    let mut t = TripData::new(5, 1, 0, 0, status::ROUTED);
    t.dest_x = 4;
    t.dest_z = 4;
    assert!(s.capacity_still_available(&t));
    t.count = 6;
    assert!(!s.capacity_still_available(&t));
    s.update_state(|st| {
        for x in 4..=5 {
            for z in 4..=5 {
                st.set_dest_byte(x, z, 0x3);
            }
        }
        st.set_free(1, 7);
    });
    s.apply_traffic_to_destination(5, 5, 5, 1, 1);
    assert_eq!(capacity(&w, 12, 5), 30);
    for x in 4..=5 {
        for z in 4..=5 {
            assert_eq!(s.destination_byte(x, z), 0x1);
        }
    }
    assert_eq!(s.free_destinations()[1], 6);
}

/// §11 test 14: the neighbour reconcile and SetupTrip of a forced start.
#[test]
fn t14_neighbour_reconcile() {
    let w = World::new();
    w.conns.borrow_mut().push((
        77,
        Conn {
            cell: (-1, 3),
            holder: 70,
            dirty: true,
            ..Conn::default()
        },
    ));
    w.set_prop(
        70,
        prop::NEIGHBOUR_TARGETS,
        Variant::array(mem::UINT32, [0, 50, 0, 0, 0, 0, 0, 0, 0].map(Elem::U32).to_vec()),
    );
    let s = sim(&w);
    s.update_traffic_volume(Source::Connection(77));
    let t = w.get_trips(Source::Connection(77));
    assert_eq!(t, vec![TripData::new(50, 0, 1, 4, 0)]);
    // The flag was consumed: a second update does nothing.
    w.set_trips(Source::Connection(77), Vec::new());
    s.update_traffic_volume(Source::Connection(77));
    assert!(w.get_trips(Source::Connection(77)).is_empty());
    let mut pf = s.create_path_finder();
    let mut pf2 = s.create_path_finder();
    s.setup_trip(&mut pf, &mut pf2, &t[0], Source::Connection(77));
    assert_eq!(pf.allowed_travel_types(), 0x1CF);
    assert_eq!(pf.starting_travel_types(), 0x2);
    assert_eq!(pf.start_rect(), CellRect::new(-1, 3, -1, 3));
    assert_eq!(pf.traffic_density(), 50);
    assert_eq!(pf.destination_flags(), 0x1);
}

/// The registry entries of a save, resolved back for the reader.
struct Resolver(Vec<RegisteredObject>, Rc<World>);

impl RefResolver for Resolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        let e = self.0.iter().find(|e| e.0 == class_id && e.1 == object_id)?;
        Some(match &e.2 {
            Some(o) => RefValue::Object(o.clone()),
            None => {
                NetworkHost::add_ref(&*self.1, e.3);
                RefValue::Host { token: e.3, class_id }
            }
        })
    }
}

fn record(s: &TrafficSimulator) -> (Vec<u8>, Vec<RegisteredObject>) {
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut out = ByteWriter::with_registry(reg.clone());
    assert!(s.write(&mut out));
    let entries = reg.borrow().entries.clone();
    (out.into_bytes(), entries)
}

/// §11 test 15: the save round trip, the header and the trip record.
#[test]
fn t15_save_round_trip() {
    let w = World::new();
    let s = sim(&w);
    let (bytes, entries) = record(&s);
    assert_eq!(
        bytes[..18],
        [0x29, 0, 0x0D, 0, 0, 0, 1, 0, 0, 0, 0x0D, 0, 0, 0, 9, 0, 0, 0]
    );
    let w2 = World::new();
    w2.dept_base.set(0xE000);
    let s2 = sim(&w2);
    let mut res = Resolver(entries.clone(), w2.clone());
    assert!(s2.read(&mut ByteReader::with_resolver(&bytes, &mut res)));
    let (again, _) = record(&s2);
    assert_eq!(again, bytes);
    // The record's departments replaced the reader's own.
    assert_eq!(*w2.released_departments.borrow(), vec![0xE000, 0xE001]);
    assert_eq!(
        s2.read_state(|st| (st.roads, st.mass_transit)),
        (Some(DEPT_BASE), Some(DEPT_BASE + 1))
    );

    // The trip record with empty routes.
    let t = TripData {
        count: 7,
        trip_type: 1,
        strategy: 2,
        flags: 3,
        status: 4,
        dest_x: 5,
        dest_z: -1,
        trip_length: 1.5,
        travel_types_used: 0x12,
        ..TripData::default()
    };
    let mut out = ByteWriter::new();
    assert!(write_trip(&mut out, &t));
    let mut want = vec![
        2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 1, 2, 3, 4, 5, 0, 0xFF, 0xFF,
    ];
    want.extend(1.5f32.to_le_bytes());
    want.extend([0x12, 0, 0, 0]);
    assert_eq!(out.bytes(), &want[..]);
    let mut back = TripData::default();
    assert!(read_trip(&mut ByteReader::new(&want), &mut back));
    assert_eq!(back, t);
}

// ---------------------------------------------------------------------------
// Beyond §11
// ---------------------------------------------------------------------------

/// §6.3: a version-1 trip record has no mask; a vector count at or above
/// 1,000,000 keeps the previous contents and reads nothing.
#[test]
fn trip_record_version_1_and_long_vector() {
    let mut b = vec![1, 0, 0, 0];
    b.extend(1_000_000u32.to_le_bytes());
    b.extend([1, 0, 0, 0, 0xAB]);
    b.extend([9, 0, 0, 0, 0, 1, 0, 3, 2, 0, 3, 0]);
    b.extend(2.0f32.to_le_bytes());
    let mut t = TripData {
        route: vec![1, 2, 3],
        travel_types_used: 0xFF,
        ..TripData::default()
    };
    assert!(read_trip(&mut ByteReader::new(&b), &mut t));
    assert_eq!(t.route, vec![1, 2, 3]);
    assert_eq!(t.return_route, vec![0xAB]);
    assert_eq!((t.count, t.strategy, t.status, t.dest_x, t.dest_z), (9, 1, 3, 2, 3));
    assert_eq!(t.trip_length, 2.0);
    assert_eq!(t.travel_types_used, 0);
    // A short record fails.
    assert!(!read_trip(&mut ByteReader::new(&b[..10]), &mut t));
}

/// Save-records §7.1.1: records older than version 19 are refused, and a
/// failed read leaves the simulator unchanged.
#[test]
fn read_rejects_old_and_truncated_records() {
    let w = World::new();
    let s = sim(&w);
    s.update_state(|st| st.budget_total = 1234);
    let (bytes, entries) = record(&s);
    let w2 = World::new();
    let s2 = sim(&w2);
    s2.update_state(|st| st.budget_total = 99);
    let mut old = bytes.clone();
    old[..2].copy_from_slice(&18u16.to_le_bytes());
    let mut res = Resolver(entries.clone(), w2.clone());
    assert!(!s2.read(&mut ByteReader::with_resolver(&old, &mut res)));
    let mut res = Resolver(entries.clone(), w2.clone());
    assert!(!s2.read(&mut ByteReader::with_resolver(&bytes[..bytes.len() - 1], &mut res)));
    assert_eq!(s2.read_state(|st| st.budget_total), 99);
    let mut res = Resolver(entries, w2.clone());
    assert!(s2.read(&mut ByteReader::with_resolver(&bytes, &mut res)));
    assert_eq!(s2.read_state(|st| st.budget_total), 1234);
}

/// Save-records §7.1.1: the edge densities of a v < 20 record are floats,
/// rounded half to even, out of range → 65,535.
#[test]
fn legacy_edge_density_values() {
    let l = record::legacy_density;
    assert_eq!(
        [l(2.5), l(3.5), l(-1.0), l(70_000.0), l(65_535.0)],
        [2, 4, 65_535, 65_535, 65_535]
    );
}

/// Read's mask and flags of a switch (traffic-network.md §4.7.2 after a
/// load).
#[test]
fn read_mask_and_flags_of_rules() {
    use crate::traffic_network::Rule;
    let r = |kind: u8, from: u8, to: u8| Rule {
        kind,
        side: [false; 4],
        from,
        to,
    };
    let (mask, flags) = record::read_mask_and_flags(&[r(1, 1, 2), r(0, 6, 7), r(1, 2, 5)]);
    assert_eq!(mask, 2 | 5);
    assert_eq!(flags, 0x01 | 0x08 | 0x40 | 0x20);
}

/// §4.12 step 1: the funding damage target and the potholes.
#[test]
fn funding_damage_and_potholes() {
    let w = World::new();
    let s = sim(&w);
    street(&s);
    let curve = s.read_state(|st| st.tuning.damage_curve.clone());
    // 100 % funding: curve(100) = −0.01 → the rate stays 0.
    assert_eq!(monthly::damage_target(&curve, 1.0, 0.0, 10), (0.0, 0));
    // 0 % funding: +0.02 per month; 50 cells → 1.
    let (d, n) = monthly::damage_target(&curve, 0.0, 0.0, 50);
    assert_eq!((bits(d), n), (bits(0.02), 1));
    // Potholes: five intact road cells, three damaged.
    s.update_damage(|d| {
        for x in 0..=4 {
            d.road.insert_cell(cell_key16(x, 3));
        }
    });
    s.apply_pothole_changes(monthly::kind::ROAD, 3);
    let (intact, damaged) = { (s.damage().road.intact.len(), s.damage().road.damaged.len()) };
    assert_eq!((intact, damaged), (2, 3));
    let cost = s.network().state().tuning.damaged_road_cost;
    let mut seen = 0;
    for x in 0..=4 {
        let info = s.network().network_info(1, x, 3).unwrap();
        if s.is_road_damaged(x, 3) {
            assert_eq!(info.extra_cost, cost);
            seen += 1;
        } else {
            assert_eq!(info.extra_cost, 0.0);
        }
    }
    assert_eq!(seen, 3);
    let posts = w
        .posted
        .borrow()
        .iter()
        .filter(|m| m.msg_type == msg::CELL_DAMAGE && m.data3 == 1)
        .count();
    assert_eq!(posts, 3);
    // Repairs.
    s.apply_pothole_changes(monthly::kind::ROAD, -10);
    assert_eq!(s.damage().road.damaged.len(), 0);
    assert_eq!(s.damage().road.intact.len(), 5);
}

/// §4.12 step 2: the strike states.
#[test]
fn strike_transitions() {
    let w = World::new();
    let s = sim(&w);
    let transit = s.read_state(|st| st.mass_transit).unwrap();
    // Funding 50 %, E > 0: p = 0.2; the first word of seed 1 gives u =
    // 0.328 > 0.2: no strike.
    w.funding.borrow_mut().insert(transit, 0.5);
    s.update_state(|st| st.budget_total = 5);
    seed(&s, 1);
    s.do_monthly_simulation();
    assert_eq!(s.read_state(|st| st.strike_state), 2);
    // Funding 0 %: p = 0.7 > 0.434 (the second word): the strike starts.
    w.funding.borrow_mut().insert(transit, 0.0);
    s.do_monthly_simulation();
    assert_eq!(s.read_state(|st| st.strike_state), 0);
    // Next month: on strike.
    s.do_monthly_simulation();
    assert_eq!(
        s.read_state(|st| (st.strike_state, st.strike_months, st.strike_flag)),
        (1, 0, true)
    );
    s.do_monthly_simulation();
    assert_eq!(s.read_state(|st| st.strike_months), 1);
    // Full funding ends it.
    w.funding.borrow_mut().insert(transit, 1.0);
    s.do_monthly_simulation();
    assert_eq!(s.read_state(|st| (st.strike_state, st.strike_flag)), (2, false));
    let strikes: Vec<u32> = w
        .posted
        .borrow()
        .iter()
        .map(|m| m.msg_type)
        .filter(|&t| (msg::STRIKE_OVER_A..=msg::STRIKE_STARTS).contains(&t))
        .collect();
    assert_eq!(strikes, vec![msg::STRIKE_STARTS, msg::STRIKE_ON, msg::STRIKE_OVER_B]);
}

/// §4.13: no view → no accident check; the first check is due at clock 0.
#[test]
fn accident_check_needs_a_view() {
    let w = World::new();
    let s = sim(&w);
    s.update_state(|st| {
        st.accident_check = vec![AccidentEntry { x: 3, z: 3, p: 1.0 }];
    });
    s.do_monthly_simulation();
    assert!(w.accidents.borrow().is_empty());
    assert_eq!(s.read_state(|st| st.next_accident_check), 60_000);
    // With a view, after the period: the accident (p = 1 > any draw).
    w.view.set(Some(CellRect::new(0, 0, 7, 7)));
    w.clock.set(60_000);
    s.do_monthly_simulation();
    assert_eq!(*w.accidents.borrow(), vec![(3, 3, 20_000)]);
    assert!(w.posted.borrow().contains(&SimMessage {
        msg_type: msg::TRAFFIC_ACCIDENT,
        data1: 3,
        data2: 3,
        data3: 0
    }));
}

/// §4.1.1: lot reconcile, the probe, and the industrial trips.
#[test]
fn lot_reconcile_and_probe() {
    let mut t = Vec::new();
    trips::reconcile_lot(&mut t, [0; 3], 0);
    assert_eq!(t, vec![TripData::new(1, 0, 0, trip_flag::PROBE, status::SEARCH)]);
    trips::reconcile_lot(&mut t, [10, 5, 0], 1);
    assert_eq!(
        t,
        vec![
            TripData::new(10, 1, 0, 0, status::SEARCH),
            TripData::new(5, 1, 1, 0, status::SEARCH)
        ]
    );
    // Growth goes to a new trip (status 4) once one of the strategy exists.
    trips::reconcile_lot(&mut t, [12, 5, 0], 1);
    assert_eq!(t[2], TripData::new(2, 1, 0, 0, status::NEW));
    // Shrinking takes the status-4 trips first.
    trips::reconcile_lot(&mut t, [9, 5, 0], 1);
    assert_eq!(t.iter().map(|t| t.count).collect::<Vec<_>>(), vec![9, 5]);
    // Freight: ceil(r24(0.05 × 30)) = 2.
    assert_eq!(trips::industrial_trips(30, 0.05), 2);
    assert_eq!(trips::industrial_trips(40, 0.05), 2);
    assert_eq!(trips::residential_commuters(0, 1.0), 1);
}

/// §4.1.3: lot activation stores the destination bits over the lot (TN1:
/// stored, not ORed) and counts the free destinations; deactivation clears
/// them.
#[test]
fn lot_activation_destination_bits() {
    let w = World::new();
    w.add_lot(5, Lot::new(CellRect::new(1, 1, 2, 2), 50));
    set_capacity(&w, 50, &[0, 4, 0, 9]);
    let s = sim(&w);
    s.update_state(|st| st.set_dest_byte(1, 1, 0x80));
    let m = Message {
        msg_type: msg::LOT_STATE_CHANGED,
        data2: 3,
        object: 5,
        ..Message::default()
    };
    assert!(s.do_message(&m));
    for (x, z) in [(1, 1), (2, 1), (1, 2), (2, 2)] {
        assert_eq!(s.destination_byte(x, z), 0x0A);
    }
    assert_eq!(s.free_destinations(), [0, 1, 0, 1]);
    assert_eq!(
        w.prop(50, prop::STRATEGY_BUCKET).map(|v| v.items()[0].as_i64()),
        Some(0)
    );
    assert_eq!(s.read_state(|st| st.strategy_bucket_counter), 1);
    s.do_message(&Message {
        data1: 3,
        data2: 0,
        ..m
    });
    assert_eq!(s.destination_byte(2, 2), 0);
    // A lot without a building stops at once: no draw, no bucket.
    w.add_lot(
        6,
        Lot {
            building: None,
            ..Lot::new(CellRect::new(4, 4, 4, 4), 60)
        },
    );
    s.do_message(&Message { object: 6, ..m });
    assert_eq!(s.read_state(|st| st.strategy_bucket_counter), 1);
    assert!(w.prop(60, prop::STRATEGY_DRAW).is_none());
}

/// §3.3: day 1 pays the incomes into the line items.
#[test]
fn day_one_income() {
    let w = World::new();
    let s = sim(&w);
    s.update_state(|st| {
        st.transit_income = 12.7;
        st.toll_income = 3.2;
    });
    w.day.set(1);
    s.do_daily_simulation();
    let (roads, transit) = s.read_state(|st| (st.roads.unwrap(), st.mass_transit.unwrap()));
    assert_eq!(
        *w.incomes.borrow(),
        vec![
            (transit, dept::TRANSIT_INCOME_ITEM, 12),
            (roads, dept::TOLL_INCOME_ITEM, 3)
        ]
    );
    assert_eq!(s.last_monthly_income(), r24f(3.2 + 12.7));
}

fn r24f(v: f64) -> f32 {
    crate::x87::r24(f64::from(v as f32)) as f32
}

/// §9.1 TG7: a department made at Init gets its line items 1-3 (mass-transit
/// line 2 Income); then, on every Init, lines 4-6 of both when missing, in
/// the original's order (roads 5, the tolls, Income). A department a `Read`
/// resolved is not created again: only the conditional lines are asked for.
#[test]
fn tg7_department_line_items() {
    let w = World::new();
    let s = sim(&w);
    let (r, m) = (DEPT_BASE, DEPT_BASE + 1);
    let mut want: Vec<(u64, dept::LineItem, bool)> = dept::ROADS_LINES.iter().map(|&i| (r, i, false)).collect();
    want.extend(dept::MASS_TRANSIT_LINES.iter().map(|&i| (m, i, false)));
    want.extend(
        dept::INIT_LINES
            .iter()
            .map(|&(mt, i)| (if mt { m } else { r }, i, true)),
    );
    assert_eq!(*w.line_items.borrow(), want);
    let order: Vec<(u64, u32, u32, bool)> = w.line_items.borrow()[6..]
        .iter()
        .map(|&(d, i, _)| (d, i.line, i.name, i.income))
        .collect();
    assert_eq!(
        order,
        vec![
            (m, 4, 0x2AB7_EA93, false),
            (m, 5, 0x8BCE_33E0, false),
            (r, 4, 0xCBCE_33D5, false),
            (m, 6, 0x8BCE_33E1, false),
            (r, 5, 0x0C3B_F3C4, true),
            (r, 6, 0xEA5F_96E7, false),
        ]
    );
    let income: Vec<(u64, u32)> = w.line_items.borrow()[..6]
        .iter()
        .filter(|e| e.1.income)
        .map(|e| (e.0, e.1.line))
        .collect();
    assert_eq!(income, vec![(m, 2)]);
    assert_eq!((dept::GROUP, dept::NAME_GROUP), (0xAA36_9059, 0x6A3F_F01C));
    // A load: Read before Init keeps the saved departments.
    let (bytes, entries) = record(&s);
    let w2 = World::new();
    w2.dept_base.set(0xE000);
    let s2 = TrafficSimulator::new(w2.clone(), 0);
    let mut res = Resolver(entries, w2.clone());
    assert!(s2.read(&mut ByteReader::with_resolver(&bytes, &mut res)));
    assert!(s2.init());
    assert!(w2.departments.borrow().is_empty());
    let loaded: Vec<(u64, dept::LineItem, bool)> = dept::INIT_LINES
        .iter()
        .map(|&(mt, i)| (if mt { m } else { r }, i, true))
        .collect();
    assert_eq!(*w2.line_items.borrow(), loaded);
}

/// §1: Init registers the agents and the subscriptions in order, and
/// creates the grids.
#[test]
fn init_registers_agents_and_grids() {
    let w = World::new();
    let s = sim(&w);
    let agents: Vec<u32> = w.agents.borrow().iter().map(|a| a.0).collect();
    assert_eq!(agents, AGENT_LEVELS.to_vec());
    assert_eq!(*w.subs.borrow(), msg::SUBSCRIPTIONS.to_vec());
    assert_eq!(s.commercial_traffic_map().unwrap().count_x(), 2);
    assert_eq!(s.congestion_map().unwrap().count_x(), 8);
    assert_eq!(
        w.departments.borrow().iter().map(|d| d.0).collect::<Vec<_>>(),
        vec![dept::ROADS, dept::MASS_TRANSIT]
    );
    // Shipped 120 % → r24(120 × 0.01f) = 0x3F999999, one ulp below 1.2
    // (§4.14 "Funding percents", TS29).
    assert_eq!(w.departments.borrow()[0].1.to_bits(), 0x3F99_9999);
    assert_eq!(w.departments.borrow()[1].1.to_bits(), 0x3F99_9999);
    assert!(s.query_interface(IID_TRAFFIC_SIMULATOR));
    assert_eq!(s.simulator_type(), SIMULATOR_TYPE);
    assert_eq!(s.trip_scale(), 6.0);
    assert_eq!(s.trip_scale_for_display(), 25.0);
    assert!(!s.desired_lot_insertion_point());
    assert!(s.shutdown());
    assert_eq!(w.released_departments.borrow().len(), 2);
    assert!(s.congestion_map().is_none());
}

/// §4.8: DumpEdgeDensities and ReduceEdgeDensities.
#[test]
fn edge_density_dump_and_reduce() {
    let (_, s) = test10();
    s.apply_route(&ROUTE_10, 40, CellRect::new(2, 2, 2, 2), (4, 4), 0, 0);
    s.enter_cycle_state(cycle_state::DUMP_EDGES);
    assert!(s.work_on_sim_cycle());
    let d = s.traffic_edge_density(3, 3, 1, false);
    assert_eq!((d.last, d.usage), (one(2, 40), 40));
    assert_eq!(s.cycle_state(), cycle_state::REDUCE_EDGES);
    assert!(s.work_on_sim_cycle());
    let d = s.traffic_edge_density(3, 3, 1, false);
    assert_eq!((d.current, d.last, d.usage), ([0; 16], one(2, 40), 40));
    // GetTravelTimeRatio: usage 40 × 0.0625 = 2.5 → the curve at 2.5.
    let c = s.read_state(|st| st.tuning.congestion_curve.eval(2.5));
    assert_eq!(bits(s.travel_time_ratio(3, 3, 1)), bits(c));
}

// ---------------------------------------------------------------------------
// §3.6 occupant bookkeeping (TS4), §4.9 the window (TS8) and the statistics
// (TS9), §4.12 the pothole count
// ---------------------------------------------------------------------------

fn bit(t: u32) -> u32 {
    1 << t
}

fn occupant_msg(s: &TrafficSimulator, id: u64, ins: bool) {
    s.do_message(&Message {
        msg_type: if ins {
            msg::OCCUPANT_INSERTED
        } else {
            msg::OCCUPANT_REMOVED
        },
        data3: crate::traffic_network::occupant_type::NETWORK,
        object: id,
        ..Message::default()
    });
}

fn damage_posts(w: &World) -> Vec<(u32, u32, u32)> {
    w.posted
        .borrow()
        .iter()
        .filter(|m| m.msg_type == msg::CELL_DAMAGE)
        .map(|m| (m.data1, m.data2, m.data3))
        .collect()
}

fn key(x: i32, z: i32) -> u32 {
    u32::from(cell_key16(x, z))
}

/// §3.6, the worked example: bulldozing a damaged road piece, then placing
/// a level crossing.
#[test]
fn occupant_bookkeeping_worked_example() {
    let w = World::new();
    w.cells.set((64, 64));
    let s = sim(&w);
    s.update_state(|st| {
        st.cells_per_type[0] = 100;
        st.stats_last.potholes = 3;
    });
    s.update_damage(|d| {
        for (x, z) in [(12, 34), (1, 1), (2, 2)] {
            d.road.damaged.insert(key(x, z));
        }
        d.road.cell_count = 100;
        d.road.damaged_count = 3;
    });
    w.add_occ(1, bit(net_type::ROAD), (12, 34), (12, 34));
    w.costs.borrow_mut().clear();
    occupant_msg(&s, 1, false);
    assert_eq!(
        s.read_state(|st| (st.cells_per_type[0], st.stats_last.potholes)),
        (99, 2)
    );
    {
        let d = s.damage();
        assert_eq!((d.road.damaged_count, d.road.cell_count), (2, 99));
        assert!(!d.road.damaged.contains(key(12, 34)));
    }
    assert_eq!(damage_posts(&w), vec![(12, 34, 0)]);
    let priced: Vec<(bool, u32)> = w.costs.borrow().iter().map(|c| (c.0, c.1[0])).collect();
    assert_eq!(priced, vec![(false, 99)]);

    // The level crossing at (20, 5), Road | Rail.
    let rail = s.read_state(|st| st.cells_per_type[1]);
    w.add_occ(2, bit(net_type::ROAD) | bit(net_type::RAIL), (20, 5), (20, 5));
    w.costs.borrow_mut().clear();
    occupant_msg(&s, 2, true);
    assert_eq!(
        s.read_state(|st| (st.cells_per_type[0], st.cells_per_type[1])),
        (100, rail + 1)
    );
    {
        let d = s.damage();
        assert!(d.road.intact.contains(key(20, 5)));
        assert_eq!(d.road.cell_count, 100);
        assert_eq!((d.rail.cell_count, d.rail.intact.len()), (0, 0));
    }
    // The roads budget after step 2, the mass-transit budget after step 4.
    let priced: Vec<(bool, u32, u32)> = w.costs.borrow().iter().map(|c| (c.0, c.1[0], c.1[1])).collect();
    assert_eq!(priced, vec![(false, 100, rail), (true, 100, rail + 1)]);
    assert_eq!(damage_posts(&w).len(), 1);
}

/// §3.6: the call order and the per-call counts.
#[test]
fn occupant_bookkeeping_rules() {
    let w = World::new();
    w.cells.set((64, 64));
    let s = sim(&w);
    let cells = |t: u32| s.read_state(|st| st.cells_per_type[t as usize]);
    let road_count = || s.damage().road.cell_count;

    // Avenue and Road: steps 1 and 2, two calls, one key.
    w.add_occ(1, bit(net_type::AVENUE) | bit(net_type::ROAD), (3, 3), (3, 3));
    w.costs.borrow_mut().clear();
    occupant_msg(&s, 1, true);
    assert_eq!(
        (cells(net_type::AVENUE), cells(net_type::ROAD), road_count()),
        (1, 1, 2)
    );
    assert_eq!(w.costs.borrow().len(), 2);
    // Re-inserted at a cell it holds: the count counts calls, not cells.
    occupant_msg(&s, 1, true);
    assert_eq!((road_count(), s.damage().road.intact.len()), (4, 1));

    // Step 3 takes only the first of OneWayRoad, Street, Highway,
    // GroundHighway.
    w.add_occ(2, bit(net_type::STREET) | bit(net_type::ONE_WAY_ROAD), (4, 3), (4, 3));
    occupant_msg(&s, 2, true);
    assert_eq!((cells(net_type::ONE_WAY_ROAD), cells(net_type::STREET)), (1, 0));
    assert_eq!(road_count(), 5);

    // A highway is bumped and priced but is not in the pothole sets.
    w.add_occ(3, bit(net_type::HIGHWAY), (5, 5), (5, 5));
    w.costs.borrow_mut().clear();
    occupant_msg(&s, 3, true);
    assert_eq!((cells(net_type::HIGHWAY), road_count()), (1, 5));
    assert!(!s.damage().road.intact.contains(key(5, 5)));
    assert_eq!(w.costs.borrow().len(), 1);

    // DirtRoad, LightRail and Monorail get no bookkeeping.
    w.add_occ(
        4,
        bit(net_type::DIRT_ROAD) | bit(net_type::LIGHT_RAIL) | bit(net_type::MONORAIL),
        (6, 6),
        (6, 6),
    );
    w.costs.borrow_mut().clear();
    occupant_msg(&s, 4, true);
    assert!(w.costs.borrow().is_empty());
    assert_eq!(cells(net_type::DIRT_ROAD), 0);

    // Subway: bit 31 keeps it out.
    w.add_occ(5, bit(net_type::SUBWAY) | 0x8000_0000, (7, 7), (7, 7));
    occupant_msg(&s, 5, true);
    assert_eq!(cells(net_type::SUBWAY), 0);
    w.add_occ(6, bit(net_type::SUBWAY), (7, 7), (7, 7));
    w.costs.borrow_mut().clear();
    occupant_msg(&s, 6, true);
    assert_eq!(cells(net_type::SUBWAY), 1);
    assert_eq!(w.costs.borrow().iter().map(|c| c.0).collect::<Vec<_>>(), vec![true]);

    // Remove at 0: `cells[t]` stays 0, the cell count wraps.
    s.update_damage(|d| d.road.cell_count = 0);
    w.add_occ(7, bit(net_type::STREET), (9, 9), (9, 9));
    occupant_msg(&s, 7, false);
    assert_eq!((cells(net_type::STREET), road_count()), (0, u32::MAX));
    assert!(damage_posts(&w).is_empty());

    // Rail: the occupant's own cell (slot 59), not its position. Removing a
    // damaged rail cell drops the road pothole count *[quirk]*.
    w.add_occ(8, bit(net_type::RAIL), (1, 1), (6, 6));
    occupant_msg(&s, 8, true);
    assert!(s.damage().rail.intact.contains(key(6, 6)));
    assert_eq!(s.damage().rail.cell_count, 1);
    s.update_damage(|d| {
        d.rail.intact.remove(key(6, 6));
        d.rail.damaged.insert(key(6, 6));
        d.rail.damaged_count = 1;
    });
    s.update_state(|st| st.stats_last.potholes = 0);
    occupant_msg(&s, 8, false);
    assert_eq!((s.damage().rail.damaged_count, s.damage().rail.cell_count), (0, 0));
    assert_eq!(s.read_state(|st| st.stats_last.potholes), 0xFFFF);
    assert_eq!(damage_posts(&w), vec![(6, 6, 0)]);
}

/// §4.12 and §3.6: ApplyRoadFunding re-sets the pothole count only in a
/// month when the road target changes, to the low 16 bits of the target.
#[test]
fn road_funding_sets_pothole_count() {
    let w = World::new();
    let s = sim(&w);
    let roads = s.read_state(|st| st.roads).unwrap();
    // 100 % funding, rate 0, target 0 = damaged 0: no change, the count
    // keeps its drops.
    s.update_state(|st| st.stats_last.potholes = 9);
    s.do_monthly_simulation();
    assert_eq!(s.read_state(|st| st.stats_last.potholes), 9);
    // 0 % funding: rate 0.02, 50 cells → target 1.
    w.funding.borrow_mut().insert(roads, 0.0);
    s.update_damage(|d| {
        for x in 0..5 {
            d.road.insert_cell(cell_key16(x, 1));
        }
        d.road.cell_count = 50;
    });
    s.do_monthly_simulation();
    assert_eq!(s.damage().road.damaged_count, 1);
    assert_eq!(s.read_state(|st| st.stats_last.potholes), 1);
    // Rate 1.0, 70000 cells → target 70000; the count is its low 16 bits.
    s.update_damage(|d| {
        d.road.rate = 1.0;
        d.road.cell_count = 70_000;
    });
    s.do_monthly_simulation();
    assert_eq!(s.damage().road.damaged_count, 70_000);
    assert_eq!(s.read_state(|st| st.stats_last.potholes), (70_000 & 0xFFFF) as u16);
}

/// The usage of travel type `t` at `(x, z)`.
fn set_usage(n: &mut crate::traffic_network::NetworkState, x: i32, z: i32, t: u32, evening: bool, v: u16) {
    let k = crate::traffic_network::NetworkState::edge_key(x, z, t, evening);
    n.edge_density.entry(k, Default::default).usage = v;
}

/// §4.9 step 2 (TS8), the worked example: block (2, 3).
#[test]
fn commercial_window_worked_example() {
    use super::maps::{commercial_cell_value, commercial_window};
    let mut n = crate::traffic_network::NetworkState::default();
    (n.count_x, n.count_z) = (32, 32);
    // Morning: road 300 + 40 + 100 = 440 at (13, 14); el-train 500 at
    // (9, 10).
    set_usage(&mut n, 13, 14, 1, false, 300);
    set_usage(&mut n, 13, 14, 2, false, 40);
    set_usage(&mut n, 13, 14, 4, false, 100);
    set_usage(&mut n, 9, 10, 7, false, 500);
    // Evening: monorail 380 at (6, 12), less elsewhere.
    set_usage(&mut n, 6, 12, 8, true, 380);
    set_usage(&mut n, 11, 17, 1, true, 200);
    // Outside the window: the corners, x = 14 on a block row, rows 9 and
    // 18; pedestrian and rail types add nothing.
    set_usage(&mut n, 6, 10, 7, false, 1000);
    set_usage(&mut n, 14, 13, 1, false, 1500);
    set_usage(&mut n, 9, 18, 1, false, 3000);
    set_usage(&mut n, 9, 9, 1, true, 3000);
    set_usage(&mut n, 10, 13, 0, false, 4000);
    set_usage(&mut n, 10, 13, 5, false, 4000);
    assert_eq!(commercial_cell_value(&n, 13, 14, false), 440);
    assert_eq!(commercial_window(&n, 2, 3), 880);
    // r24(880 × 0.128) = 112.64 → 113.
    let scale = SimTuning::default().commercial_scale;
    assert_eq!(crate::x87::rhe(crate::x87::r24(880.0 * f64::from(scale)) as f32), 113);
    // The road sum is u16: 65000 + 1000 wraps to 464.
    set_usage(&mut n, 0, 0, 1, false, 65_000);
    set_usage(&mut n, 0, 0, 2, false, 1000);
    assert_eq!(commercial_window(&n, 0, 0), 464);
    // A 256-wide city: the east-most block column reads x = 0 and 1 on its
    // own rows (the key's low byte, no upper clamp) *[compat]*.
    let mut n = crate::traffic_network::NetworkState::default();
    (n.count_x, n.count_z) = (256, 256);
    set_usage(&mut n, 1, 2, 7, false, 77);
    assert_eq!(commercial_window(&n, 63, 0), 77);
    set_usage(&mut n, 0, 4, 7, true, 5);
    assert_eq!(commercial_window(&n, 63, 0), 77);
}

/// §4.9 step 3 (TS9): every record counts, the pedestrian one included;
/// flag bit 0 picks the bridge slot; `max` starts at −1.0; the end of the
/// pass replaces the cell counts.
#[test]
fn statistics_slots() {
    let w = World::new();
    let s = sim(&w);
    s.network().update_state(|n| {
        let mut add = |net: u32, x: i32, ty: u32, inv: f32, flags: u8| {
            let r = n.append_info(net, x, 1).unwrap();
            r.inv_cap = inv;
            r.net_type = ty as u8;
            r.flags = flags;
        };
        add(1, 1, net_type::ROAD, 0.5, 0);
        add(0, 1, net_type::ROAD, 0.5, 0);
        add(1, 2, net_type::ROAD, 0.25, 1);
        add(1, 3, net_type::STREET, 1.0, 0);
        add(1, 4, net_type::ONE_WAY_ROAD, 1.0, 0);
    });
    let set = |n: usize, x: i32, v: f32| s.volume_grid(n).unwrap().set(x, 1, v);
    set(1, 1, 4.0);
    set(0, 1, 1.0);
    set(1, 2, 8.0);
    set(1, 3, 0.5);
    s.update_state(|st| st.cells_per_type = [7; 13]);
    s.enter_cycle_state(cycle_state::CREATE_MAPS);
    assert!(s.work_on_sim_cycle());
    let last = s.read_state(|st| st.stats_last.clone());
    let row = |sum: f32, congested: u32, (max_x, max_z): (i32, i32), max: f32| StatRow {
        sum,
        congested,
        max_x,
        max_z,
        max,
    };
    // Road: r = 2.0 (car) and 0.5 (pedestrian).
    assert_eq!(last.rows[0], row(1.25, 1, (1, 1), 2.0));
    assert_eq!(last.rows[3], row(0.5, 0, (3, 1), 0.5));
    // r = 0 still beats the starting −1.0.
    assert_eq!(last.rows[10], row(0.0, 0, (4, 1), 0.0));
    // A slot with no record publishes zeros (TS40: the reset zeroes the rows).
    assert_eq!(last.rows[1], StatRow::default());
    // The bridge: r = 2.0, not in the type's count.
    assert_eq!(last.bridge, row(2.0, 1, (2, 1), 2.0));
    let mut records = [0u16; 13];
    (records[0], records[3], records[10]) = (2, 1, 1);
    assert_eq!(last.records, records);
    assert_eq!(s.read_state(|st| st.cells_per_type), records.map(u32::from));
    // The budgets priced the new counts.
    let priced = w.costs.borrow();
    assert_eq!(
        priced[priced.len() - 2..],
        [(false, records.map(u32::from)), (true, records.map(u32::from))]
    );
}

// ---------------------------------------------------------------------------
// The neighbour pools (traffic-network.md §2.7.2–§2.7.5; TS2, TS6, TS12, TS18)
// ---------------------------------------------------------------------------

fn u16s(v: &[u32]) -> Variant {
    Variant::array(mem::UINT16, v.iter().map(|&e| Elem::U16(e as u16)).collect())
}

fn u32s(v: &[u32]) -> Variant {
    Variant::array(mem::UINT32, v.iter().map(|&e| Elem::U32(e)).collect())
}

fn neighbour_message(s: &TrafficSimulator, added: bool, conn_type: u32, x: i32, z: i32) {
    s.do_message(&Message {
        msg_type: if added {
            msg::NEIGHBOUR_ADDED
        } else {
            msg::NEIGHBOUR_REMOVED
        },
        data1: conn_type,
        data2: x as u32,
        data3: z as u32,
        ..Message::default()
    });
}

/// §4.2 (TS18): the element store creates, keeps u16 below `0xFFFF`,
/// promotes the whole array at `0xFFFF` and never goes back; a mistyped
/// property counts as missing *[choice]*.
#[test]
fn ts18_element_store() {
    let w = World::new();
    w.add_lot(1, Lot::new(CellRect::new(4, 4, 5, 5), 12));
    w.set_prop(
        12,
        prop::TRIP_CAPACITY,
        u16s(&[100, 0, 0, 0, 65530, 0, 0, 0, 7, 0, 0, 0]),
    );
    let s = sim(&w);
    // The spec's example: 65530 + 10 = 65540 promotes all twelve values.
    s.apply_traffic_to_destination(10, 4, 4, travel::CAR as u8, 0);
    assert_eq!(
        w.prop(12, prop::TRIP_CAPACITY),
        Some(u32s(&[100, 0, 0, 0, 65540, 0, 0, 0, 7, 0, 0, 0]))
    );
    // A small value keeps the u32 array.
    s.set_max_trip_capacity(12, 1, 5);
    assert_eq!(
        w.prop(12, prop::TRIP_CAPACITY),
        Some(u32s(&[100, 5, 0, 0, 65540, 0, 0, 0, 7, 0, 0, 0]))
    );
    // Missing: created as 12 u16 zeros.
    s.set_max_trip_capacity(13, 2, 9);
    assert_eq!(
        w.prop(13, prop::TRIP_CAPACITY),
        Some(u16s(&[0, 0, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0]))
    );
    // 0xFFFF itself promotes.
    s.set_max_trip_capacity(14, 0, 0xFFFF);
    assert_eq!(
        w.prop(14, prop::TRIP_CAPACITY),
        Some(u32s(&[0xFFFF, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]))
    );
    // A short array reads 0 and is replaced on a write.
    w.set_prop(15, prop::TRIP_CAPACITY, u16s(&[1, 2, 3, 4, 5]));
    assert_eq!(s.max_trip_capacity(15, 0), 0);
    s.set_max_trip_capacity(15, 1, 3);
    assert_eq!(
        w.prop(15, prop::TRIP_CAPACITY),
        Some(u16s(&[0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]))
    );
}

/// State 5 (§4.11, TS18): the roll goes through the element store, so a lot
/// without the property gets one.
#[test]
fn ts18_clear_trip_destinations_creates_the_property() {
    let w = World::new();
    w.add_lot(1, Lot::new(CellRect::new(2, 2, 2, 2), 12));
    w.add_lot(2, Lot::new(CellRect::new(4, 4, 4, 4), 13));
    w.set_prop(13, prop::TRIP_CAPACITY, u16s(&[4, 0, 0, 0, 3, 2, 1, 0, 9, 9, 9, 9]));
    let s = sim(&w);
    s.enter_cycle_state(cycle_state::CLEAR_DESTINATIONS);
    assert!(s.work_on_sim_cycle());
    assert_eq!(w.prop(12, prop::TRIP_CAPACITY), Some(u16s(&[0; 12])));
    assert_eq!(
        w.prop(13, prop::TRIP_CAPACITY),
        Some(u16s(&[4, 0, 0, 0, 0, 0, 0, 0, 3, 2, 1, 0]))
    );
}

/// traffic-network.md §2.7.2 (TS6): Add sets the mask, grows the cost and
/// finds or creates the city's pool; Remove clears the type, and the last
/// type also the pool (swept), the destination bits and `freeDest`.
#[test]
fn ts6_add_and_remove_neighbour_connections() {
    let w = World::new();
    let road = 2u32;
    let rail = 1u32;
    let push = |id: u64, c: Conn| w.conns.borrow_mut().push((id, c));
    // A's road at (−1, 5); its other side stored a trip length of 120.
    push(
        77,
        Conn {
            cell: (-1, 5),
            holder: 70,
            conn_type: Some(road),
            other_side: Some(88),
            city: Some((10, 20)),
            ..Conn::default()
        },
    );
    push(
        88,
        Conn {
            cell: (99, 99),
            holder: 80,
            conn_type: Some(road),
            ..Conn::default()
        },
    );
    w.set_prop(80, prop::CONNECTION_TRIP_LENGTH, Variant::Scalar(Elem::F32(120.0)));
    // A's rail at the same cell, no other side.
    push(
        78,
        Conn {
            cell: (-1, 5),
            holder: 71,
            conn_type: Some(rail),
            city: Some((10, 20)),
            ..Conn::default()
        },
    );
    // A's road at (3, −1), and B's road at (8, 2); C at (3, 8) is not
    // established.
    push(
        79,
        Conn {
            cell: (3, -1),
            holder: 72,
            conn_type: Some(road),
            city: Some((10, 20)),
            ..Conn::default()
        },
    );
    push(
        90,
        Conn {
            cell: (8, 2),
            holder: 73,
            conn_type: Some(road),
            city: Some((11, 20)),
            ..Conn::default()
        },
    );
    push(
        91,
        Conn {
            cell: (3, 8),
            holder: 74,
            conn_type: Some(road),
            ..Conn::default()
        },
    );
    let s = sim(&w);
    let rec = |x: i32, z: i32| {
        let r = s.network().traffic_neighbor_connection(x, z);
        (r.mask, r.pool, r.cost)
    };
    let cmt = s.read_state(|st| st.tuning.commute_max_time);
    assert!(cmt < 120.0);
    neighbour_message(&s, true, road, -1, 5);
    assert_eq!(rec(-1, 5), (1 << road, 1, 120.0));
    // The cost only grows: the rail Add offers commuteMaxTime.
    neighbour_message(&s, true, rail, -1, 5);
    assert_eq!(rec(-1, 5), (1 << road | 1 << rail, 1, 120.0));
    neighbour_message(&s, true, road, 3, -1);
    neighbour_message(&s, true, road, 8, 2);
    neighbour_message(&s, true, road, 3, 8);
    assert_eq!(rec(3, -1), (1 << road, 1, cmt));
    assert_eq!(rec(8, 2), (1 << road, 2, cmt));
    assert_eq!(rec(3, 8), (1 << road, 0, cmt));
    // No connection of that type: nothing.
    neighbour_message(&s, true, 9, 3, 8);
    assert_eq!(rec(3, 8), (1 << road, 0, cmt));
    let ids = |s: &TrafficSimulator| s.pools().map.keys().copied().collect::<Vec<u16>>();
    assert_eq!(ids(&s), vec![1, 2]);
    // The pools carry only the position.
    let pos = |id: u16, p: u32| {
        s.pools().map[&id]
            .get(p)
            .and_then(|v| v.items().first().map(|e| e.as_i64()))
    };
    assert_eq!((pos(2, POOL_CITY_X), pos(2, POOL_CITY_Z)), (Some(11), Some(20)));

    // Remove: B's last type. The pool is swept, the destination bits go
    // and `freeDest` drops once per set bit.
    s.update_state(|st| {
        st.set_dest_byte(8, 2, 0b1011);
        for k in 0..4 {
            st.set_free(k, 5);
        }
    });
    neighbour_message(&s, false, road, 8, 2);
    assert_eq!(rec(8, 2), (0, 0, cmt));
    assert_eq!(ids(&s), vec![1]);
    assert_eq!(s.destination_byte(8, 2), 0);
    assert_eq!(s.free_destinations(), [4, 4, 5, 4]);
    // Removing one of two types keeps the record's pool.
    neighbour_message(&s, false, road, -1, 5);
    assert_eq!(rec(-1, 5), (1 << rail, 1, 120.0));
    // B again: the smallest unused ID ≥ 1 is 2.
    neighbour_message(&s, true, road, 8, 2);
    assert_eq!(rec(8, 2), (1 << road, 2, cmt));
    assert_eq!(ids(&s), vec![1, 2]);
}

/// traffic-network.md §2.7.4 (TS12): the spec's example. StartRecalc shares
/// the regional jobs out by population and sets the edge cells'
/// destination bits.
#[test]
fn ts12_neighbour_capacities_example() {
    let w = World::new();
    let road = 2u32;
    w.conns.borrow_mut().extend([
        (
            77,
            Conn {
                cell: (-1, 5),
                holder: 70,
                conn_type: Some(road),
                city: Some((10, 20)),
                ..Conn::default()
            },
        ),
        (
            78,
            Conn {
                cell: (8, 2),
                holder: 71,
                conn_type: Some(road),
                city: Some((11, 20)),
                ..Conn::default()
            },
        ),
    ]);
    *w.cities.borrow_mut() = vec![
        NeighbourCity {
            position: (10, 20),
            population: [300, 1, 0],
        },
        NeighbourCity {
            position: (11, 20),
            population: [100, 2, 0],
        },
    ];
    *w.demand.borrow_mut() = [Some(800.0), Some(10.0), Some(0.0)];
    let s = sim(&w);
    // A road exit west at (0, 5): HasMapExit(−1, 5). B's cell has none.
    s.network().set_connection(0, 5, side::EAST, side::WEST, travel::CAR);
    neighbour_message(&s, true, road, -1, 5);
    neighbour_message(&s, true, road, 8, 2);
    // Arrivals this cycle are reset.
    s.set_max_trip_capacity(pool_token(1), 4, 33);
    s.set_max_trip_capacity(pool_token(0), 7, 44);
    s.enter_cycle_state(cycle_state::START_RECALC);
    assert!(s.work_on_sim_cycle());
    let caps = |id: u16| capacity_values(&s.holders(), pool_token(id)).unwrap();
    assert_eq!(caps(1), vec![600, 3, 0, u32::MAX, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(caps(2), vec![200, 6, 0, u32::MAX, 0, 0, 0, 0, 0, 0, 0, 0]);
    assert_eq!(caps(0), vec![0, 0, 0, u32::MAX, 0, 0, 0, 0, 0, 0, 0, 0]);
    // The edge pass: bits 0, 1 and 3 at (−1, 5); (8, 2) has no map exit.
    assert_eq!(s.destination_byte(-1, 5), 0b1011);
    assert_eq!(s.destination_byte(8, 2), 0);
    assert_eq!(s.free_destinations(), [1, 1, 0, 1]);
    // A missing demand node is 0 *[choice]*.
    w.demand.borrow_mut()[0] = None;
    s.enter_cycle_state(cycle_state::START_RECALC);
    assert!(s.work_on_sim_cycle());
    assert_eq!(caps(1)[..4], [0, 3, 0, u32::MAX]);
    // The count goes up again even with the bit set *[quirk]*.
    assert_eq!(s.free_destinations(), [1, 2, 0, 2]);
}

/// traffic-network.md §2.7.5 (TS2): state 11 rolls each connection's own
/// holder.
#[test]
fn ts2_neighbour_roll() {
    let w = World::new();
    w.conns.borrow_mut().extend([
        (
            77,
            Conn {
                cell: (-1, 5),
                holder: 70,
                ..Conn::default()
            },
        ),
        (
            78,
            Conn {
                cell: (8, 2),
                holder: 71,
                ..Conn::default()
            },
        ),
    ]);
    w.set_prop(70, prop::POOL_ARRIVALS + 1, u32s(&[0, 5, 0, 0, 0, 0, 0, 0, 0]));
    w.set_prop(70, prop::TRIP_CAPACITY, u16s(&[7, 7, 7, 7, 1, 2, 3, 4, 9, 9, 9, 9]));
    let s = sim(&w);
    s.enter_cycle_state(cycle_state::UPDATE_NEIGHBOURS);
    assert!(s.work_on_sim_cycle());
    assert_eq!(w.prop(70, prop::NEIGHBOUR_TARGETS), None);
    assert_eq!(
        w.prop(70, prop::NEIGHBOUR_TARGETS + 1),
        Some(u32s(&[0, 5, 0, 0, 0, 0, 0, 0, 0]))
    );
    for h in [70, 71] {
        for i in 0..3 {
            assert_eq!(w.prop(h, prop::POOL_ARRIVALS + i), Some(u32s(&[0; 9])));
        }
    }
    assert_eq!(
        w.prop(70, prop::TRIP_CAPACITY),
        Some(u16s(&[7, 7, 7, 7, 0, 0, 0, 0, 1, 2, 3, 4]))
    );
    assert_eq!(w.prop(71, prop::TRIP_CAPACITY), Some(u16s(&[0; 12])));
}

/// §4.2: off the map, the pool counts the arrivals; the connection's own
/// holder gets the per-travel-type array and the pool's new total.
#[test]
fn ts_apply_traffic_to_neighbour_destination() {
    let w = World::new();
    let road = 2u32;
    w.conns.borrow_mut().push((
        77,
        Conn {
            cell: (-1, 5),
            holder: 70,
            conn_type: Some(road),
            city: Some((10, 20)),
            ..Conn::default()
        },
    ));
    let s = sim(&w);
    neighbour_message(&s, true, road, -1, 5);
    let pool = pool_token(1);
    s.set_max_trip_capacity(pool, 0, 50);
    s.update_state(|st| {
        st.set_dest_byte(-1, 5, 0b0001);
        st.set_free(0, 3);
    });
    let car = travel::CAR as u8;
    s.apply_traffic_to_destination(30, -1, 5, car, 0);
    assert_eq!(s.traffic_arrived_this_cycle(pool, 0), 30);
    assert_eq!(
        w.prop(70, prop::POOL_ARRIVALS),
        Some(u32s(&[0, 30, 0, 0, 0, 0, 0, 0, 0]))
    );
    assert_eq!(capacity(&w, 70, 4), 30);
    assert_eq!(s.destination_byte(-1, 5), 0b0001);
    // 55 ≥ 50: the bit goes and `freeDest[0]` drops.
    s.apply_traffic_to_destination(25, -1, 5, car, 0);
    assert_eq!(s.traffic_arrived_this_cycle(pool, 0), 55);
    assert_eq!(
        w.prop(70, prop::POOL_ARRIVALS),
        Some(u32s(&[0, 55, 0, 0, 0, 0, 0, 0, 0]))
    );
    assert_eq!(s.destination_byte(-1, 5), 0);
    assert_eq!(s.free_destinations()[0], 2);
    // Freight (type 3) has no arrivals array; the connection still gets the
    // pool's total.
    s.apply_traffic_to_destination(4, -1, 5, travel::FREIGHT_TRUCK as u8, 3);
    assert_eq!(s.traffic_arrived_this_cycle(pool, 3), 4);
    assert_eq!(capacity(&w, 70, 7), 4);
    assert_eq!(w.prop(70, prop::POOL_ARRIVALS + 1), None);
    assert_eq!(w.prop(70, prop::POOL_ARRIVALS + 2), None);
    // A travel type no connection type at the cell carries: no holder.
    s.apply_traffic_to_destination(5, -1, 5, travel::PASSENGER_TRAIN as u8, 1);
    assert_eq!(s.traffic_arrived_this_cycle(pool, 1), 0);
    assert_eq!(capacity(&w, 70, 5), 0);
}

/// §6.1 row 19: the pools are saved by ID with their properties.
#[test]
fn pools_round_trip() {
    let w = World::new();
    let s = sim(&w);
    s.update_pools(|p| {
        let a = p.create(10, 20).unwrap();
        let b = p.create(-3, 7).unwrap();
        assert_eq!((a, b), (1, 2));
    });
    s.set_max_trip_capacity(pool_token(2), 0, 70_000);
    s.set_max_trip_capacity(pool_token(1), 5, 3);
    let (bytes, entries) = record(&s);
    let w2 = World::new();
    let s2 = sim(&w2);
    s2.update_pools(|p| {
        p.create(1, 1);
    });
    let mut res = Resolver(entries, w2.clone());
    assert!(s2.read(&mut ByteReader::with_resolver(&bytes, &mut res)));
    assert_eq!(s2.pools().map, s.pools().map);
    assert_eq!(s2.max_trip_capacity(pool_token(2), 0), 70_000);
    // The default pool is not saved: Init made it.
    assert_eq!(s2.max_trip_capacity(pool_token(0), 3), u32::MAX);
}

// ---------------------------------------------------------------------------
// §3.5 DoPostCityInit (TS7), §4.1 the source iterator (TS20), §4.1.3
// deactivation (TS23), §4.7 FinishTripMap (TS26), §4.8 (TS22)
// ---------------------------------------------------------------------------

/// §4.1 (TS20): the lots are walked live, the connections are a snapshot.
#[test]
fn ts20_source_iterator_walk() {
    let mut it = SourceIter::default();
    it.reset(vec![77, 78]);
    assert_eq!(it.next_lot(vec![1, 2, 3]), Some(1));
    // 2 removed before the walk reaches it, 4 appended: 3, then 4.
    assert_eq!(it.next_lot(vec![1, 3, 4]), Some(3));
    // The last lot returned is removed: its successor still comes.
    assert_eq!(it.next_lot(vec![1, 4]), Some(4));
    // Yielding 4 already advanced to the sentinel. An append now belongs
    // to the next pass, even before the caller asks for another source.
    assert!(it.lots_done);
    assert_eq!(it.next_lot(vec![1, 4, 5]), None);
    // Past the end the walk is in the connections.
    assert_eq!(it.next_lot(vec![1, 4, 5, 6]), None);
    assert_eq!(it.next_connection(), Some(77));
    assert_eq!(it.next_connection(), Some(78));
    assert_eq!(it.next_connection(), None);
}

/// §4.1 (TS20) through ConfirmTrips: a lot added during the phase is
/// visited, a removed one skipped, and a connection added after `Reset` is
/// not. Each source taken stores its cell (§3.5 step 11).
#[test]
fn ts20_confirm_trips_live_lots() {
    let w = World::new();
    w.add_lot(1, Lot::new(CellRect::new(1, 1, 1, 1), 11));
    w.add_lot(2, Lot::new(CellRect::new(2, 2, 2, 2), 12));
    w.conns.borrow_mut().push((
        77,
        Conn {
            cell: (-1, 5),
            holder: 70,
            ..Conn::default()
        },
    ));
    let s = sim(&w);
    s.set_traffic_units_per_slice(1);
    s.enter_cycle_state(cycle_state::CONFIRM_TRIPS);
    let step = |s: &TrafficSimulator| {
        let done = s.work_on_sim_cycle();
        (done, s.read_state(|st| st.extras.phase_positions))
    };
    assert_eq!(step(&s), (false, [1, 1]));
    w.lots.borrow_mut().retain(|(id, _)| *id != 2);
    w.add_lot(3, Lot::new(CellRect::new(3, 4, 3, 4), 13));
    w.conns.borrow_mut().push((
        78,
        Conn {
            cell: (8, 2),
            holder: 71,
            ..Conn::default()
        },
    ));
    assert_eq!(step(&s), (false, [3, 4]));
    assert_eq!(step(&s), (false, [-1, 5]));
    // 78 came after the snapshot: the phase is done.
    assert!(step(&s).0);
    assert_eq!(s.cycle_state(), cycle_state::REROUTE_TRIPS);
}

/// §9.1 TG19: the connection snapshot of `Reset` holds a counted reference
/// on each connection; one removed during the phase is still visited, and
/// the next `Reset` (and Shutdown) gives the references back.
#[test]
fn tg19_connection_snapshot_references() {
    let w = World::new();
    for (id, cell, holder) in [(77u64, (-1, 5), 70u64), (78, (8, 2), 71)] {
        w.conns.borrow_mut().push((
            id,
            Conn {
                cell,
                holder,
                ..Conn::default()
            },
        ));
    }
    let refs = |t: u64| w.refs.borrow().get(&t).copied().unwrap_or(0);
    let s = sim(&w);
    s.set_traffic_units_per_slice(1);
    s.enter_cycle_state(cycle_state::CONFIRM_TRIPS);
    assert_eq!((refs(77), refs(78)), (1, 1));
    // 78 leaves the city after the snapshot: still visited.
    w.conns.borrow_mut().retain(|(id, _)| *id != 78);
    let step = |s: &TrafficSimulator| {
        let done = s.work_on_sim_cycle();
        (done, s.read_state(|st| st.extras.phase_positions))
    };
    assert_eq!(step(&s), (false, [-1, 5]));
    // 78 is taken (the fixture no longer knows its cell: (0, 0)).
    assert_eq!(step(&s), (false, [0, 0]));
    assert!(step(&s).0);
    // The next Reset (entering 17): 78's reference given back.
    assert_eq!(s.cycle_state(), cycle_state::REROUTE_TRIPS);
    assert_eq!((refs(77), refs(78)), (1, 0));
    assert!(s.shutdown());
    assert_eq!((refs(77), refs(78)), (0, 0));
}

/// §3.5 (TS7): DoPostCityInit registers the connections, sweeps the pools,
/// runs the boundary pass, maps state 12/13 to 8, rebuilds the buckets,
/// sets `days` and prices both budgets.
#[test]
fn ts7_post_city_init() {
    let w = World::new();
    // A ferry connection (mask bit 9: HasMapExit true) and a road one.
    w.conns.borrow_mut().extend([
        (
            77,
            Conn {
                cell: (-1, 5),
                holder: 70,
                conn_type: Some(9),
                city: Some((10, 20)),
                ..Conn::default()
            },
        ),
        (
            78,
            Conn {
                cell: (3, 8),
                holder: 71,
                conn_type: Some(2),
                ..Conn::default()
            },
        ),
    ]);
    for i in 0..20u64 {
        let (x, z) = ((i % 4) as i32, (i / 4) as i32);
        w.add_lot(100 + i, Lot::new(CellRect::new(x, z, x, z), 200 + i));
    }
    w.set_prop(205, prop::STRATEGY_BUCKET, Variant::Scalar(Elem::U32(3)));
    w.set_prop(202, prop::STRATEGY_BUCKET, Variant::Scalar(Elem::U32(3)));
    w.set_prop(207, prop::STRATEGY_BUCKET, Variant::Scalar(Elem::U32(1)));
    let s = sim(&w);
    // A pool no record names, and stale buckets.
    s.update_pools(|p| {
        p.create(50, 50);
        p.create(51, 51);
    });
    s.update_state(|st| {
        st.cycle_state = cycle_state::SUBNETWORK_INFO;
        st.buckets.insert(9, vec![(7, 7)]);
        for x in -1..=8 {
            for z in [-1, 8] {
                st.set_dest_byte(x, z, 0xF);
                st.set_dest_byte(z, x, 0xF);
            }
        }
        st.set_free(0, 4);
        st.cells_per_type = [2; 13];
    });
    let priced = w.costs.borrow().len();
    s.do_post_city_init();
    let r = s.network().traffic_neighbor_connection(-1, 5);
    // Pools 1 and 2 were taken, so the Add created pool 3; the sweep then
    // released 1 and 2, which no record names.
    assert_eq!((r.mask, r.pool), (1 << 9, 3));
    assert_eq!(s.network().traffic_neighbor_connection(3, 8).mask, 1 << 2);
    assert_eq!(s.pools().map.keys().copied().collect::<Vec<_>>(), vec![3]);
    // The ring: only (−1, 5) has a map exit.
    assert_eq!(s.destination_byte(-1, 5), 0xF);
    assert_eq!(s.destination_byte(-1, 4), 0);
    assert_eq!(s.destination_byte(3, 8), 0);
    assert_eq!(s.destination_byte(-1, -1), 0);
    assert_eq!(s.free_destinations()[0], 4);
    assert_eq!(s.cycle_state(), cycle_state::FINAL_CALCULATIONS);
    let buckets = s.read_state(|st| st.buckets.clone());
    assert_eq!(
        buckets.into_iter().collect::<Vec<_>>(),
        vec![(1, vec![(3, 1)]), (3, vec![(2, 0), (1, 1)])]
    );
    assert_eq!(s.read_state(|st| st.days), 1);
    let costs = w.costs.borrow();
    assert_eq!(costs[priced..], [(true, [2; 13]), (false, [2; 13])]);
}

/// §3.5 step 11 (TS7): the resume by the loaded state.
#[test]
fn ts7_post_city_init_resume() {
    let w = World::new();
    w.add_lot(1, Lot::new(CellRect::new(1, 1, 1, 1), 11));
    w.add_lot(2, Lot::new(CellRect::new(2, 2, 3, 3), 12));
    w.add_lot(3, Lot::new(CellRect::new(4, 4, 4, 4), 13));
    let s = sim(&w);
    // 18: the source at the saved cell is held; the walk goes on after it.
    s.update_state(|st| {
        st.cycle_state = cycle_state::FULL_TRIPS;
        st.extras.phase_positions = [2, 2];
    });
    s.do_post_city_init();
    assert_eq!(s.read_state(|st| st.current), Some(Source::Lot(2)));
    assert_eq!(s.read_state(|st| st.days), 0);
    // No source at the saved cell: reset again, none held.
    s.update_state(|st| {
        st.cycle_state = cycle_state::CONFIRM_TRIPS;
        st.extras.phase_positions = [3, 3];
    });
    s.do_post_city_init();
    assert_eq!(s.read_state(|st| (st.current, st.iter.last_lot)), (None, None));
    // 15: past the lot at the saved cell (any cell of it).
    s.update_state(|st| {
        st.cycle_state = cycle_state::FINISH_TRIP_MAP;
        st.extras.phase_positions = [3, 3];
    });
    s.do_post_city_init();
    assert_eq!(s.read_state(|st| st.finish_resume), Some(2));
    // Ferries: only in state 19.
    let running = |s: &TrafficSimulator| s.network().update_state(|n| n.ferry_pass.running);
    s.network().update_state(|n| n.ferry_pass.running = false);
    s.update_state(|st| st.cycle_state = cycle_state::START_RECALC);
    s.do_post_city_init();
    assert!(!running(&s));
    s.update_state(|st| st.cycle_state = cycle_state::FERRY_ROUTES);
    s.do_post_city_init();
    assert!(running(&s));
    // 1 and 14: `freeDest` zeroed, then ClearTripDestinations and
    // RefreshCommuteMap over the city, then state 0.
    set_capacity(&w, 13, &[5]);
    s.update_state(|st| {
        st.cycle_state = 14;
        st.set_free(2, 9);
    });
    s.do_post_city_init();
    assert_eq!(s.cycle_state(), cycle_state::START_RECALC);
    assert_eq!(s.free_destinations(), [1, 0, 0, 0]);
    assert_eq!(s.destination_byte(4, 4), 1);
}

/// §4.7 (TS26): the spec's example, and no donor writes 0.
#[test]
fn ts26_finish_trip_map_example() {
    let w = World::new();
    w.cells.set((32, 32));
    let lot = |zone: u32, road: bool, r: CellRect, holder: u64| Lot {
        zone,
        road_access: road,
        ..Lot::new(r, holder)
    };
    w.add_lot(1, lot(8, false, CellRect::new(20, 20, 20, 21), 11));
    w.add_lot(2, lot(1, true, CellRect::new(19, 20, 19, 20), 12));
    w.add_lot(3, lot(8, false, CellRect::new(21, 20, 21, 20), 13));
    w.add_lot(4, lot(8, true, CellRect::new(22, 19, 22, 20), 14));
    // A lone lot with no donor.
    w.add_lot(5, lot(9, false, CellRect::new(5, 5, 6, 5), 15));
    let s = sim(&w);
    s.update_state(|st| {
        let g = st.trip_length.as_ref().unwrap();
        g.set(22, 20, 37);
        g.set(22, 19, 99);
        g.set(5, 5, 7);
        g.set(6, 5, 7);
    });
    s.enter_cycle_state(cycle_state::FINISH_TRIP_MAP);
    assert!(s.work_on_sim_cycle());
    let at = |x, z| s.trip_length_map().unwrap().get_cell_value(x, z);
    assert_eq!((at(20, 20), at(20, 21)), (37, 37));
    assert_eq!((at(5, 5), at(6, 5)), (0, 0));
}

/// §4.1.3 (TS23): the spec's example, deactivated in state 18, and in
/// state 10 `freeDest` is kept.
#[test]
fn ts23_lot_deactivation_example() {
    let w = World::new();
    w.cells.set((32, 32));
    w.add_lot(1, Lot::new(CellRect::new(10, 20, 11, 21), 11));
    w.add_lot(2, Lot::new(CellRect::new(14, 20, 14, 20), 12));
    w.set_prop(11, prop::STRATEGY_BUCKET, Variant::Scalar(Elem::U32(4)));
    w.item_cost.set(250);
    let s = sim(&w);
    s.update_state(|st| {
        st.cycle_state = cycle_state::FULL_TRIPS;
        for x in 10..=11 {
            for z in 20..=21 {
                st.set_dest_byte(x, z, 3);
            }
        }
        st.set_dest_byte(14, 20, 1);
        for k in 0..4 {
            st.set_free(k, 5);
        }
        st.buckets.insert(4, vec![(14, 20), (10, 20), (10, 20)]);
        st.budget_total = 1000;
    });
    let deactivate = |s: &TrafficSimulator, lot: u64| {
        s.do_message(&Message {
            msg_type: msg::LOT_STATE_CHANGED,
            data1: 3,
            object: lot,
            ..Message::default()
        })
    };
    deactivate(&s, 1);
    assert_eq!(s.free_destinations(), [4, 4, 5, 5]);
    for (x, z) in [(10, 20), (11, 20), (10, 21), (11, 21)] {
        assert_eq!(s.destination_byte(x, z), 0);
    }
    // Only the first entry at the lot's location goes.
    assert_eq!(s.read_state(|st| st.buckets[&4].clone()), vec![(14, 20), (10, 20)]);
    assert_eq!(s.read_state(|st| st.budget_total), 750);
    // State 10: the bytes go, `freeDest` stays.
    s.update_state(|st| st.cycle_state = cycle_state::REFRESH_COMMUTE);
    deactivate(&s, 2);
    assert_eq!(s.destination_byte(14, 20), 0);
    assert_eq!(s.free_destinations(), [4, 4, 5, 5]);
}

/// §4.8 (TS22): after the scan, each terminal's arrival cell is cleared; its
/// departure cell only when the scan reaches it (network info).
#[test]
fn ts22_reduce_ferry_arrival_cells() {
    let w = World::new();
    let s = sim(&w);
    s.network().update_state(|n| {
        n.ferries.push(crate::traffic_network::FerryTerminal {
            dep_x: 1,
            dep_z: 1,
            arr_x: 6,
            arr_z: 6,
            ..Default::default()
        });
        for (x, z) in [(1, 1), (6, 6)] {
            let k = crate::traffic_network::NetworkState::edge_key(x, z, 1, false);
            n.edge_density.entry(k, Default::default).current = [7; 16];
        }
    });
    s.enter_cycle_state(cycle_state::REDUCE_EDGES);
    assert!(s.work_on_sim_cycle());
    assert_eq!(s.traffic_edge_density(6, 6, 1, false).current, [0; 16]);
    assert_eq!(s.traffic_edge_density(1, 1, 1, false).current, [7; 16]);
}

// ---- §4.3 ConfirmRoute and §4.5 ApplyRoute (TS14, TS15, TS16, TS24, TS25) ----

/// Switch IDs over cells and the tolls of the switches (§4.5 step 2).
fn set_switches(s: &TrafficSimulator, cells: &[(i32, i32, u32)], tolls: &[(u32, f32)]) {
    s.network().update_state(|n| {
        let g = n.switch_id.clone().expect("switch ID grid");
        for &(x, z, id) in cells {
            g.set(x, z, id);
        }
        for &(id, toll) in tolls {
            n.switches
                .entry(id, crate::traffic_network::TransitSwitch::default)
                .simulator_value = toll.to_bits();
        }
    });
}

fn switch_load(s: &TrafficSimulator, id: u32) -> Option<f32> {
    s.network().update_state(|n| n.switches.get(id).map(|sw| sw.load))
}

/// A terminal with departure and arrival cells, links keyed by the other
/// terminals' departure cells and neighbour links `[off_x, off_z, edge_x,
/// edge_z]`.
fn terminal(
    dep: (u16, u16),
    arr: (u16, u16),
    links: &[(i32, i32)],
    neighbours: &[[i32; 4]],
) -> crate::traffic_network::FerryTerminal {
    use crate::traffic_network::{cell_key, FerryTerminal, NeighborLink, TerminalLink};
    FerryTerminal {
        dep_x: dep.0,
        dep_z: dep.1,
        arr_x: arr.0,
        arr_z: arr.1,
        links: links
            .iter()
            .map(|&(x, z)| TerminalLink {
                to_key: cell_key(x, z),
                ..TerminalLink::default()
            })
            .collect(),
        neighbour_links: neighbours
            .iter()
            .map(|&[ox, oz, ex, ez]| NeighborLink {
                off_x: ox as i16,
                off_z: oz as i16,
                edge_x: ex as u16,
                edge_z: ez as u16,
                ..NeighborLink::default()
            })
            .collect(),
        ..FerryTerminal::default()
    }
}

/// Appends terminals with both keys registered (traffic-network §2.6).
fn add_terminals(s: &TrafficSimulator, list: Vec<crate::traffic_network::FerryTerminal>) {
    s.network().update_state(|n| {
        for t in list {
            let pos = n.ferries.len() as u32;
            let (dk, ak) = (u32::from(t.departure_key()), u32::from(t.arrival_key()));
            n.ferries.push(t);
            n.ferry_departures.entry(dk, || pos);
            n.ferry_arrivals.entry(ak, || pos);
        }
    });
}

fn ferry(s: &TrafficSimulator, i: usize) -> crate::traffic_network::FerryTerminal {
    s.network().update_state(|n| n.ferries[i].clone())
}

/// The ferry fare 0.01 and no per-tile income, so `I` is the fare alone.
fn fare_only(s: &TrafficSimulator) {
    s.update_state(|st| {
        st.tuning.ferry_fare = 0.01;
        st.tuning.income_per_tile = [0.0; 9];
    });
}

fn income(s: &TrafficSimulator) -> f64 {
    s.read_state(|st| st.income_acc.get())
}

/// `r24(30 × 0.01f)` = 0.29999998 (§4.5 step 4 example).
const FARE_30: u32 = 0x3E99_9999;

/// §4.3 (TS14, TS36): the route reader's records over runs and jumps. A
/// jump to an x-neighbour has no exit and blanks the cell's entry in its
/// own record; the landing cell carries the entry the cell before the jump
/// had on arrival (TS36); a same-cell jump and `dz = −1` give `+z`; equal
/// magnitudes zero `dx`; the `0x80` flag marks the cell before (TS39).
#[test]
fn ts14_route_reader_records() {
    use super::route::{read_route, RouteStep};
    let r = |x, z, t, t_next, entry, exit, ferry_jump_next| RouteStep {
        x,
        z,
        t,
        t_next,
        entry,
        exit,
        ferry_jump_next,
    };
    assert_eq!(
        read_route(&ROUTE_10),
        vec![
            r(2, 3, 1, 1, None, Some(2), false),
            r(3, 3, 1, 1, Some(0), Some(2), false),
            r(4, 3, 1, 1, Some(0), None, false),
        ]
    );
    let route = [
        0x01, 5, 5, 0x41, 1, 0x02, 0x01, 7, 5, 0x41, 1, 0x03, 0x02, 7, 6, 0x02, 7, 4, 0x82, 4, 5, 0x02, 4, 4, 0x02, 2,
        2,
    ];
    assert_eq!(
        read_route(&route),
        vec![
            r(5, 5, 1, 1, None, Some(2), false),
            r(6, 5, 1, 1, None, None, false),
            r(7, 5, 1, 1, Some(0), Some(3), false),
            r(7, 6, 1, 2, Some(1), Some(3), false),
            r(7, 6, 2, 2, Some(1), Some(1), false),
            r(7, 4, 2, 2, Some(3), Some(0), true),
            r(4, 5, 2, 2, Some(2), Some(3), false),
            r(4, 4, 2, 2, Some(1), Some(1), false),
            r(2, 2, 2, 2, Some(3), None, false),
        ]
    );
    assert!(read_route(&[]).is_empty());
    // TS36: the entry is none only when the cell before the jump is the
    // first cell: (5, 5) jumps to (6, 5) at once.
    assert_eq!(
        read_route(&[0x01, 5, 5, 0x01, 6, 5, 0x41, 1, 0x02]),
        vec![
            r(5, 5, 1, 1, None, None, false),
            r(6, 5, 1, 1, None, Some(2), false),
            r(7, 5, 1, 1, Some(0), None, false),
        ]
    );
    // The spec example: a step +x into (5, 3), entry −x, then a jump to
    // (6, 3): (5, 3) has entry and exit none, (6, 3) entry −x.
    assert_eq!(
        read_route(&[0x01, 4, 3, 0x41, 1, 0x02, 0x01, 6, 3]),
        vec![
            r(4, 3, 1, 1, None, Some(2), false),
            r(5, 3, 1, 1, None, None, false),
            r(6, 3, 1, 1, Some(0), None, false),
        ]
    );
    // Two x-neighbour jumps in a row carry the same arrival side on.
    assert_eq!(
        read_route(&[0x01, 4, 3, 0x41, 1, 0x02, 0x01, 6, 3, 0x01, 7, 3, 0x41, 1, 0x03]),
        vec![
            r(4, 3, 1, 1, None, Some(2), false),
            r(5, 3, 1, 1, None, None, false),
            r(6, 3, 1, 1, None, None, false),
            r(7, 3, 1, 1, Some(0), Some(3), false),
            r(7, 4, 1, 1, Some(1), None, false),
        ]
    );
}

/// §4.3 worked example (TS14): ConfirmRoute over test 10's route, the
/// wormhole passes, the strike and the walk-to-bus change at (3,3).
#[test]
fn ts14_confirm_route_example() {
    use crate::traffic_network::wormhole;
    let w = World::new();
    let s = sim(&w);
    let conn = |x: i32, z: i32, t: usize, v: u16| {
        s.network().update_state(|n| n.conn_mut(x, z).unwrap().conn[t] = v);
    };
    let switch = |x: i32, z: i32, t: usize, v: u16| {
        s.network().update_state(|n| n.conn_mut(x, z).unwrap().switch[t] = v);
    };
    let worm = |x: i32, z: i32, v: u8| {
        s.network().update_state(|n| n.conn_mut(x, z).unwrap().wormhole = v);
    };
    assert!(!s.confirm_route(&[]), "an empty route");
    assert!(!s.confirm_route(&ROUTE_10));
    // (3,3) needs bit 2 + 4·0; (2,3) any entry through +x; (4,3) is never
    // looked at.
    conn(3, 3, 1, 1 << 2);
    for bit in [2, 6, 10, 14] {
        conn(2, 3, 1, 1 << bit);
        assert!(s.confirm_route(&ROUTE_10), "bit {bit}");
    }
    conn(2, 3, 1, 1 << 3);
    assert!(!s.confirm_route(&ROUTE_10), "the first cell's exit is +x");
    conn(2, 3, 1, 1 << 6);
    conn(3, 3, 1, 1 << 6);
    assert!(!s.confirm_route(&ROUTE_10), "entry −x, not −z");
    worm(3, 3, wormhole::TUNNEL);
    assert!(s.confirm_route(&ROUTE_10), "the tunnel bit");
    worm(3, 3, wormhole::FERRY);
    assert!(s.confirm_route(&ROUTE_10), "the ferry bit, no strike");
    s.update_state(|st| st.strike_state = 1);
    assert!(!s.confirm_route(&ROUTE_10), "the ferry bit during a strike");
    conn(3, 3, 1, 1 << 2);
    assert!(s.confirm_route(&ROUTE_10), "one type confirms during a strike");
    // Walk to bus at (3,3): t = 0, t' = 2, nibble 2 of switch[0].
    let route = [0x00, 2, 3, 0x40, 1, 0x02, 0x42, 1, 0x02];
    conn(2, 3, 0, 1 << 2);
    switch(3, 3, 0, 0xF2FF);
    assert!(!s.confirm_route(&route), "a type change during a strike");
    s.update_state(|st| st.strike_state = 0);
    assert!(s.confirm_route(&route));
    switch(3, 3, 0, 0xFFFF);
    assert!(!s.confirm_route(&route));
    // A type change at the first cell: the switch nibble only, no strike
    // test.
    s.update_state(|st| st.strike_state = 1);
    let route = [0x00, 2, 3, 0x42, 1, 0x02];
    switch(2, 3, 0, 0xF2FF);
    assert!(s.confirm_route(&route));
    switch(2, 3, 0, 0xF3FF);
    assert!(!s.confirm_route(&route));
}

/// §4.5 step 2 worked example (TS15): 37 commuters by car through a toll
/// booth with toll 0.1 (f32 0.100000001): fees `r24(37 × 0.1)` =
/// 3.70000005; usage 37. Mode 2 counts and charges once.
#[test]
fn ts15_toll_example() {
    let (_, s) = test10();
    set_switches(&s, &[(3, 3, 5)], &[(5, 0.1)]);
    s.apply_route(&ROUTE_10, 37, CellRect::new(2, 2, 2, 2), (4, 4), 0, 0);
    assert_eq!(s.read_state(|st| st.fee_acc), f64::from(f32::from_bits(0x406C_CCCD)));
    assert_eq!(switch_load(&s, 5), Some(37.0));
    s.apply_route(&ROUTE_10, 37, CellRect::new(2, 2, 2, 2), (4, 4), 2, 0);
    assert_eq!(s.read_state(|st| st.fee_acc), f64::from(f32::from_bits(0x40EC_CCCD)));
    assert_eq!(switch_load(&s, 5), Some(74.0));
}

/// §4.5 step 2 (TS25): `last` follows every in-city cell (a bus route over
/// IDs 0, 7, 7, 0, 7 counts switch 7 twice); the subway exception looks at
/// the next record (the last record is its own next); a missing switch is
/// inserted.
#[test]
fn ts25_switch_last_and_subway() {
    let w = World::new();
    let s = sim(&w);
    set_switches(
        &s,
        &[
            (2, 1, 7),
            (3, 1, 7),
            (5, 1, 7),
            (1, 2, 4),
            (2, 2, 9),
            (2, 3, 11),
            (1, 4, 12),
        ],
        &[(7, 0.5), (4, 0.5), (9, 0.5)],
    );
    // Bus: (1,1) +x four times to (5,1).
    s.apply_route(
        &[0x02, 1, 1, 0x42, 4, 0xAA],
        10,
        CellRect::new(0, 1, 0, 1),
        (6, 1),
        0,
        0,
    );
    assert_eq!(switch_load(&s, 7), Some(20.0));
    assert_eq!(s.read_state(|st| st.fee_acc), 10.0);
    // Subway (1,2) then (2,2), walking on to (3,2): (1,2)'s next record is
    // subway (not counted), (2,2)'s is walking (counted).
    s.apply_route(
        &[0x06, 1, 2, 0x46, 1, 0x02, 0x40, 1, 0x02],
        10,
        CellRect::new(0, 2, 0, 2),
        (4, 2),
        0,
        0,
    );
    assert_eq!(switch_load(&s, 4), Some(0.0));
    assert_eq!(switch_load(&s, 9), Some(10.0));
    assert_eq!(s.read_state(|st| st.fee_acc), 15.0);
    // A subway last cell inside switch 11: never counted.
    s.apply_route(
        &[0x06, 1, 3, 0x46, 1, 0x02],
        10,
        CellRect::new(0, 3, 0, 3),
        (3, 3),
        0,
        0,
    );
    assert_eq!(switch_load(&s, 11), None);
    // Switch 12 is not in the table: inserted, toll 0.
    s.apply_route(
        &[0x02, 1, 4, 0x42, 1, 0x02],
        10,
        CellRect::new(0, 4, 0, 4),
        (3, 4),
        0,
        0,
    );
    assert_eq!(switch_load(&s, 12), Some(10.0));
    assert_eq!(s.read_state(|st| st.fee_acc), 15.0);
}

/// §4.5 step 4 worked example (TS16), mode 2: 30 pedestrians cross from
/// terminal A to terminal B with the fare 0.01. A's riders += 60, `I` +=
/// 0.29999998, A's link to B current A += 30, B's link to A current B += 30.
/// Modes 0 and 1 count A's riders once and A's link's current A or B.
#[test]
fn ts16_ferry_leg_example() {
    // (1,2) +x (2,2) = A's departure; the 0x80 jump to (6,3) = B's arrival;
    // +x (7,3).
    let route = [0x00, 1, 2, 0x40, 1, 0x02, 0x80, 6, 3, 0x40, 1, 0x02];
    for (mode, riders, a_link, b_link) in [
        (2u8, 60u16, [0u16, 30, 0, 0], [0u16, 0, 0, 30]),
        (0, 30, [0, 30, 0, 0], [0; 4]),
        (1, 30, [0, 0, 0, 30], [0; 4]),
    ] {
        let w = World::new();
        let s = sim(&w);
        fare_only(&s);
        add_terminals(
            &s,
            vec![
                terminal((2, 2), (2, 1), &[(6, 2)], &[]),
                terminal((6, 2), (6, 3), &[(2, 2)], &[]),
            ],
        );
        s.apply_route(&route, 30, CellRect::new(0, 2, 0, 2), (7, 4), mode, 0);
        let (a, b) = (ferry(&s, 0), ferry(&s, 1));
        assert_eq!(a.riders_current, riders, "mode {mode}");
        assert_eq!(b.riders_current, 0);
        assert_eq!(a.links[0].counters, a_link, "mode {mode}");
        assert_eq!(b.links[0].counters, b_link, "mode {mode}");
        assert_eq!(income(&s), f64::from(f32::from_bits(FARE_30)), "mode {mode}");
    }
}

/// §4.5 step 4 (TS16): a crossing to a neighbour; the next record holds the
/// link's in-city edge cell. Mode 2 bumps current A and B.
#[test]
fn ts16_ferry_leg_to_neighbour() {
    let w = World::new();
    let s = sim(&w);
    fare_only(&s);
    add_terminals(&s, vec![terminal((3, 6), (3, 7), &[], &[[-1, 6, 0, 6]])]);
    // (4,6) −x (3,6), the 0x80 jump to the edge cell (0,6).
    s.apply_route(
        &[0x00, 4, 6, 0x40, 1, 0x00, 0x80, 0, 6],
        30,
        CellRect::new(5, 6, 5, 6),
        (-1, 6),
        2,
        0,
    );
    let t = ferry(&s, 0);
    assert_eq!(t.riders_current, 60);
    assert_eq!(t.neighbour_links[0].counters, [0, 30, 0, 30]);
    assert_eq!(income(&s), f64::from(f32::from_bits(FARE_30)));
}

/// §4.5 (TS16, `0x00719A4E`): the crossing in from a neighbour at the first
/// cell. The first cell is beside `R` on x only, is T's arrival cell, and
/// T's neighbour link has `R`'s first corner as its off-map cell: the link
/// is bumped by mode and `I := r24(Q × fare)`; T's riders are not counted.
/// Beside `R` on both coordinates, nothing.
#[test]
fn ts16_first_cell_neighbour_crossing() {
    let route = [0x00, 0, 5, 0x40, 1, 0x02];
    let w = World::new();
    let s = sim(&w);
    fare_only(&s);
    add_terminals(&s, vec![terminal((3, 6), (0, 5), &[], &[[-1, 5, 0, 5]])]);
    s.apply_route(&route, 30, CellRect::new(-1, 5, -1, 5), (2, 5), 0, 0);
    let t = ferry(&s, 0);
    assert_eq!(t.neighbour_links[0].counters, [0, 30, 0, 0]);
    assert_eq!(t.riders_current, 0);
    assert_eq!(income(&s), f64::from(f32::from_bits(FARE_30)));
    // R = (−1, 4): beside on x and z.
    let w = World::new();
    let s = sim(&w);
    fare_only(&s);
    add_terminals(&s, vec![terminal((3, 6), (0, 5), &[], &[[-1, 4, 0, 4]])]);
    s.apply_route(&route, 30, CellRect::new(-1, 4, -1, 4), (2, 5), 0, 0);
    assert_eq!(ferry(&s, 0).neighbour_links[0].counters, [0; 4]);
    assert_eq!(income(&s), 0.0);
}

/// §4.5 step 3 (TS24): in mode 1, `R` is the destination lot's rect (the
/// first entry faces it) and the last exit faces `sourceRect`; in mode 0 a
/// destination two cells away leaves the last cell's exit undefined (no
/// update there only).
#[test]
fn ts24_return_leg_sides() {
    let w = World::new();
    w.add_lot(1, Lot::new(CellRect::new(2, 2, 2, 2), 11));
    w.add_lot(2, Lot::new(CellRect::new(3, 4, 5, 5), 12));
    let s = sim(&w);
    street(&s);
    // (4,3) −x twice to (2,3); destination (5,5) in lot 2.
    s.apply_route(
        &[0x01, 4, 3, 0x41, 2, 0x00],
        40,
        CellRect::new(2, 2, 2, 2),
        (5, 5),
        1,
        0,
    );
    assert_eq!(current(&s, 4, 3, true), one(4 * 3, 40));
    assert_eq!(current(&s, 3, 3, true), one(4 * 2, 40));
    assert_eq!(current(&s, 2, 3, true), one(4 * 2 + 1, 40));
    for x in 2..=4 {
        assert_eq!(current(&s, x, 3, false), [0; 16]);
    }
    assert_eq!(capacity(&w, 12, 4), 0, "no destination step in mode 1");

    let (_w, s) = test10();
    s.apply_route(&ROUTE_10, 40, CellRect::new(2, 2, 2, 2), (4, 5), 0, 0);
    assert_eq!(current(&s, 2, 3, false), one(4 + 2, 40));
    assert_eq!(current(&s, 3, 3, false), one(2, 40));
    assert_eq!(current(&s, 4, 3, false), [0; 16]);
    assert_eq!(bits(volume(&s, 1, 4, 3)), 0x4220_0000);
}

/// §4.5 step 3 (TS24): an undefined side at an in-city cell before the last
/// (here the jump to the x-neighbour (4,3)) ends ApplyRoute: the volume,
/// switch usage and densities already added stay; `I`, the fees, the later
/// cells and the destination step are lost.
#[test]
fn ts24_undefined_side_ends_apply_route() {
    let (w, s) = test10();
    s.update_state(|st| st.tuning.income_per_tile = [1.0; 9]);
    set_switches(&s, &[(3, 3, 5)], &[(5, 0.5)]);
    let route = [0x01, 2, 3, 0x41, 1, 0x02, 0x01, 4, 3, 0x41, 1, 0x03];
    s.apply_route(&route, 40, CellRect::new(2, 2, 2, 2), (4, 4), 0, 0);
    assert_eq!(volume(&s, 1, 2, 3), 40.0);
    assert_eq!(volume(&s, 1, 3, 3), 40.0);
    assert_eq!(volume(&s, 1, 4, 3), 0.0);
    assert_eq!(switch_load(&s, 5), Some(40.0));
    assert_eq!(current(&s, 2, 3, false), one(4 + 2, 40));
    assert_eq!(current(&s, 3, 3, false), [0; 16]);
    assert_eq!(income(&s), 0.0);
    assert_eq!(s.read_state(|st| st.fee_acc), 0.0);
    assert_eq!(capacity(&w, 12, 4), 0);
}

/// §4.5 step 5 (`0x00719C06`): income for every cell, off-map cells
/// included; steps 1–4 for in-city cells only.
#[test]
fn apply_route_income_counts_off_map_cells() {
    let w = World::new();
    let s = sim(&w);
    s.update_state(|st| st.tuning.income_per_tile = [1.0; 9]);
    // (1,3) −x (0,3) −x (−1,3).
    s.apply_route(
        &[0x01, 1, 3, 0x41, 2, 0x00],
        40,
        CellRect::new(2, 3, 2, 3),
        (-1, 3),
        1,
        0,
    );
    assert_eq!(income(&s), 120.0);
}

// ---- §4.4 SetupPathFinderForLot and the return leg, §4.16, §4.11 (TS10, TS11, TS17, TS19) ----

/// §4.4 (TS17): FindHighwayReturnTile, the worked example (English drives
/// on the right: out by highway to (−1, 40) the return leg starts at
/// (−1, 41); in from (−1, 40) it ends at (−1, 39); UK English reversed) and
/// the other three edges; an in-city cell is unchanged.
#[test]
fn ts17_find_highway_return_tile() {
    let size = (64, 64);
    assert_eq!(find_highway_return_tile(-1, 40, false, true, size), (-1, 41));
    assert_eq!(find_highway_return_tile(-1, 40, true, true, size), (-1, 39));
    assert_eq!(find_highway_return_tile(-1, 40, false, false, size), (-1, 39));
    assert_eq!(find_highway_return_tile(-1, 40, true, false, size), (-1, 41));
    assert_eq!(find_highway_return_tile(64, 10, false, true, size), (64, 9));
    assert_eq!(find_highway_return_tile(10, -1, false, true, size), (9, -1));
    assert_eq!(find_highway_return_tile(10, 64, false, true, size), (11, 64));
    // The first matching row: a corner moves along x < 0.
    assert_eq!(find_highway_return_tile(-1, -1, false, true, size), (-1, 0));
    assert_eq!(find_highway_return_tile(5, 5, false, true, size), (5, 5));
}

/// §4.4 (TS17, TS19): the return leg's rects. Start: an off-map end cell
/// moves for a car or bus at a divided-road edge record (mask `0x1108`),
/// not for walking or at a road; an in-city end cell gives the bounding
/// rect of the lot at any of its cells, or nothing without a lot. End: pf's
/// start rect, or at an off-map divided-road corner (any type) the cell
/// moved back. Driving on the left reverses both.
#[test]
fn ts17_return_leg_rects() {
    use crate::traffic_network::NeighbourEdge;
    let w = World::new();
    w.add_lot(1, Lot::new(CellRect::new(4, 4, 5, 5), 11));
    let s = sim(&w);
    let edge = |x: i32, z: i32, mask: u16| {
        s.network().update_state(|n| {
            n.set_neighbour_edge(
                x,
                z,
                NeighbourEdge {
                    mask,
                    ..NeighbourEdge::default()
                },
            )
        });
    };
    edge(-1, 4, 1 << 3); // Highway
    edge(-1, 6, 1 << 2); // Road
    edge(-1, 2, 1 << 8); // Avenue
    edge(8, 3, 1 << 12); // GroundHighway
    let lot = CellRect::new(4, 4, 5, 5);
    let cell = |x, z| CellRect::new(x, z, x, z);
    assert_eq!(s.return_leg_rects((-1, 4), 1, lot), (Some(cell(-1, 5)), lot));
    assert_eq!(s.return_leg_rects((-1, 4), 2, lot), (Some(cell(-1, 5)), lot));
    assert_eq!(s.return_leg_rects((-1, 4), 0, lot), (Some(cell(-1, 4)), lot));
    assert_eq!(s.return_leg_rects((-1, 6), 1, lot), (Some(cell(-1, 6)), lot));
    assert_eq!(s.return_leg_rects((8, 3), 1, lot), (Some(cell(8, 2)), lot));
    // In the city: any cell of the lot; no lot, no start rect.
    assert_eq!(s.return_leg_rects((5, 5), 1, cell(-1, 6)), (Some(lot), cell(-1, 6)));
    assert_eq!(s.return_leg_rects((6, 6), 1, cell(-1, 6)), (None, cell(-1, 6)));
    // pf started off the map at an avenue: the end moves back, any type.
    assert_eq!(s.return_leg_rects((5, 5), 0, cell(-1, 2)), (Some(lot), cell(-1, 1)));
    // Driving on the left.
    w.drive_left.set(true);
    assert_eq!(
        s.return_leg_rects((-1, 4), 1, cell(-1, 2)),
        (Some(cell(-1, 3)), cell(-1, 3))
    );
}

/// §4.4 (TS10): SetupPathFinderForLot. The dominant strategy by summed
/// counts (a tie keeps the lower index), the first trip's type, the lot's
/// rect; density and work time are the caller's.
#[test]
fn ts10_setup_path_finder_for_lot() {
    let w = World::new();
    w.add_lot(1, Lot::new(CellRect::new(2, 2, 3, 3), 11));
    w.add_lot(2, Lot::new(CellRect::new(5, 5, 5, 5), 12));
    w.add_lot(3, Lot::new(CellRect::new(6, 6, 6, 6), 13));
    w.set_trips(
        Source::Lot(1),
        vec![
            TripData::new(10, 0, 0, 0, 0),
            TripData::new(30, 0, 2, 0, 0),
            TripData::new(30, 1, 1, 0, 0),
        ],
    );
    w.set_trips(Source::Lot(3), vec![TripData::new(5, 4, 0, 0, 0)]);
    let s = sim(&w);
    let tu = s.read_state(|st| st.tuning.clone());
    let mut pf = s.create_path_finder();
    assert!(!s.setup_path_finder_for_lot(&mut pf, 0));
    pf.set_traffic_density(77);
    pf.set_max_work_time(9);
    assert!(s.setup_path_finder_for_lot(&mut pf, 1));
    assert_eq!(pf.start_rect(), CellRect::new(2, 2, 3, 3));
    assert_eq!(pf.trip_destination_filter(), crate::pathfinder::filter::STANDARD);
    assert_eq!(pf.destination_flags(), 1);
    assert_eq!(pf.allowed_travel_types(), COMMUTER_TYPES);
    assert_eq!(pf.starting_travel_types(), COMMUTER_START);
    // sum = [10, 30, 30]: 1 beats 0, 2 only ties 1.
    assert_eq!(pf.starting_cost_array(), Some(tu.start_car));
    assert_eq!(pf.flags(), 0);
    assert_eq!((pf.traffic_density(), pf.max_work_time()), (77, 9));
    // No trips and none generated (zone 0): false.
    assert!(!s.setup_path_finder_for_lot(&mut pf, 2));
    // Type 4: the destination flag is set, then false.
    assert!(!s.setup_path_finder_for_lot(&mut pf, 3));
    assert_eq!(pf.destination_flags(), 1 << 4);
    // An industrial lot with no trips: UpdateTrafficVolume makes its freight.
    let mut ind = Lot::new(CellRect::new(1, 6, 1, 6), 14);
    ind.zone = 7;
    ind.pops = vec![(0x4100, 400)];
    w.add_lot(4, ind);
    let mut pf = s.create_path_finder();
    assert!(s.setup_path_finder_for_lot(&mut pf, 4));
    let trips = w.get_trips(Source::Lot(4));
    assert!(!trips.is_empty());
    assert_eq!(trips[0].trip_type, FREIGHT);
    assert_eq!(pf.flags(), crate::pathfinder::flag::NO_BUDGET);
    assert_eq!(pf.allowed_travel_types(), FREIGHT_TYPES);
    assert_eq!(pf.destination_flags(), 1 << FREIGHT);
    assert_eq!(pf.starting_cost_array(), None);
}

/// §4.16 worked example (TS11): a subway station with capacity 2500, last
/// cycle's load 3100 (ratio 1.0), a budget item of 150 in a department
/// funded at 0.9: `{2500, 2500, 1.0, 3100, 135}`; during a strike
/// `{0, 2500, 0.0, 0, 135}`. A ferry terminal in the lot at the cell:
/// riders last period and 1.0. Nothing at the cell, or a switch ID with no
/// switch: false.
#[test]
fn ts11_transit_switch_query_data_example() {
    let w = World::new();
    w.add_lot(1, Lot::new(CellRect::new(6, 6, 7, 7), 11));
    let s = sim(&w);
    let dept = w.department_budget(dept::MASS_TRANSIT).expect("mass transit");
    w.funding.borrow_mut().insert(dept, 0.9);
    w.query_items.borrow_mut().insert(900, (dept::MASS_TRANSIT, 150));
    w.query_items.borrow_mut().insert(1011, (dept::MASS_TRANSIT, 100));
    set_switches(&s, &[(3, 3, 5), (1, 1, 6)], &[(5, 0.0)]);
    s.network().update_state(|n| {
        let sw = n.switches.get_mut(5).unwrap();
        sw.building = 900;
        sw.capacity = 2500.0;
        sw.counter_a = 3100.0;
        sw.counter_b = 1.0;
        n.ferries.push(crate::traffic_network::FerryTerminal {
            lot_x: 7,
            lot_z: 6,
            riders_last: 42,
            ..Default::default()
        });
    });
    let q = |available, max_capacity, load_fraction, usage, cost| SwitchQueryData {
        available,
        max_capacity,
        load_fraction,
        usage,
        cost,
    };
    assert_eq!(s.transit_switch_query_data(3, 3), Some(q(2500, 2500, 1.0, 3100, 135)));
    // The lot's building 1000 + 11 at 0.9: r24(100 × 0.9) = 90.
    assert_eq!(s.transit_switch_query_data(7, 7), Some(q(42, 42, 1.0, 42, 90)));
    assert_eq!(s.transit_switch_query_data(2, 2), None);
    assert_eq!(s.transit_switch_query_data(1, 1), None);
    s.update_state(|st| st.strike_state = 1);
    assert_eq!(s.transit_switch_query_data(3, 3), Some(q(0, 2500, 0.0, 0, 135)));
    assert_eq!(s.transit_switch_query_data(6, 6), Some(q(0, 0, 0.0, 0, 90)));
}

/// §4.11 (TS19): ClearTripDestinations asks `GetLot(x, z, true)`, so a
/// 2 × 2 lot is rolled and counted once; RefreshCommuteMap sets the bits at
/// every cell but counts `freeDest` once.
#[test]
fn ts19_get_lot_origin_only_in_state_5_and_10() {
    let w = World::new();
    w.add_lot(1, Lot::new(CellRect::new(2, 2, 3, 3), 11));
    w.set_prop(11, prop::TRIP_CAPACITY, u16s(&[6, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0]));
    let s = sim(&w);
    s.enter_cycle_state(cycle_state::CLEAR_DESTINATIONS);
    assert!(s.work_on_sim_cycle());
    assert_eq!(
        w.prop(11, prop::TRIP_CAPACITY),
        Some(u16s(&[6, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0]))
    );
    assert_eq!(s.read_state(|st| st.job_capacity), [6, 0, 0]);
    s.update_state(|st| st.set_free(0, 0));
    s.enter_cycle_state(cycle_state::REFRESH_COMMUTE);
    assert!(s.work_on_sim_cycle());
    for (x, z) in [(2, 2), (3, 2), (2, 3), (3, 3)] {
        assert_eq!(s.destination_byte(x, z), 1);
    }
    assert_eq!(s.free_destinations()[0], 1);
}

// ---- re-answers8 (TS35-TS49) ----

/// §3.3 (TS35): entering 17 zeroes pass counter 1; entering 18 zeroes the
/// day index, processed and `passTotal`, not counter 1, which keeps the
/// re-route count of state 17 through state 18.
#[test]
fn ts35_entering_18_keeps_pass_counter_1() {
    use super::state::PC_REROUTE_COUNTER;
    let w = World::new();
    let s = sim(&w);
    s.update_state(|st| {
        st.cycle_state = cycle_state::CONFIRM_TRIPS;
        st.pass_counters[PC_REROUTE_COUNTER] = 7;
    });
    // 16 with no source finishes and enters 17.
    assert!(s.do_daily_simulation());
    assert_eq!(s.cycle_state(), cycle_state::REROUTE_TRIPS);
    assert_eq!(s.read_state(|st| st.pass_counters[PC_REROUTE_COUNTER]), 0);
    // 17 with no trip finishes and enters 18: counter 1 is kept.
    s.update_state(|st| {
        st.pass_counters[PC_REROUTE_COUNTER] = 7;
        st.pass_total = 33;
        st.to_search = 40;
        st.processed = 5;
        st.day_index = 2;
    });
    assert!(s.do_daily_simulation());
    assert_eq!(s.cycle_state(), cycle_state::FULL_TRIPS);
    assert_eq!(
        s.read_state(|st| (
            st.pass_counters[PC_REROUTE_COUNTER],
            st.pass_total,
            st.total,
            st.days,
            st.processed,
            st.day_index
        )),
        (7, 0, 40, 2, 0, 0)
    );
}

/// §3.2, §3.5 step 11 (TS44, TS45): the two s16 phase positions. The 16–18
/// entry action zeroes them; a trip phase stores the cell of the source it
/// takes; the resume compares them as 16-bit values; FinishTripMap stores
/// each lot's location as it takes it.
#[test]
fn ts44_ts45_phase_positions() {
    let w = World::new();
    w.add_lot(1, Lot::new(CellRect::new(1, 1, 1, 1), 11));
    // x = 65 538 is 2 as an s16.
    w.add_lot(2, Lot::new(CellRect::new(65_538, 2, 65_538, 2), 12));
    let s = sim(&w);
    for state in [
        cycle_state::CONFIRM_TRIPS,
        cycle_state::REROUTE_TRIPS,
        cycle_state::FULL_TRIPS,
    ] {
        s.update_state(|st| st.extras.phase_positions = [5, 6]);
        s.enter_cycle_state(state);
        assert_eq!(s.read_state(|st| st.extras.phase_positions), [0, 0], "state {state}");
    }
    // One ConfirmTrips slice of one unit takes lot 1.
    s.set_traffic_units_per_slice(1);
    s.enter_cycle_state(cycle_state::CONFIRM_TRIPS);
    assert!(!s.work_on_sim_cycle());
    assert_eq!(s.read_state(|st| st.extras.phase_positions), [1, 1]);
    // The resume finds lot 2 at (2, 2) by its 16-bit x.
    s.update_state(|st| {
        st.cycle_state = cycle_state::FULL_TRIPS;
        st.extras.phase_positions = [2, 2];
    });
    s.do_post_city_init();
    assert_eq!(s.read_state(|st| st.current), Some(Source::Lot(2)));
    // FinishTripMap: the last lot's location, as an s16 pair.
    s.update_state(|st| st.extras.phase_positions = [9, 9]);
    s.enter_cycle_state(cycle_state::FINISH_TRIP_MAP);
    assert!(s.work_on_sim_cycle());
    assert_eq!(s.cycle_state(), cycle_state::CREATE_MAPS);
    assert_eq!(s.read_state(|st| st.extras.phase_positions), [2, 2]);
}

/// §3.2 rows 16/17/18/15/1 (TS67): the phase positions are zeroed by the
/// entry action of the state entered when a phase finishes: 16 -> 17,
/// 17 -> 18 and 18 -> 15 all leave (0, 0) until the new phase takes its
/// first source or lot; entering 1 does the same. The exhausted end of a
/// trip phase writes nothing, and the re-route context is not reset.
#[test]
fn ts67_trip_phase_transitions_zero_phase_positions() {
    let w = World::new();
    w.add_lot(1, Lot::new(CellRect::new(1, 1, 1, 1), 11));
    let s = sim(&w);
    // A finished FullTrips moves to 15; its entry zeroes the last source's
    // cell (the in-game record: orig (0, 0), the cell (100, 128) before).
    s.update_state(|st| {
        st.cycle_state = cycle_state::FULL_TRIPS;
        st.extras.phase_positions = [100, 128];
    });
    assert!(s.work_on_sim_cycle());
    assert_eq!(s.cycle_state(), cycle_state::FINISH_TRIP_MAP);
    assert_eq!(s.read_state(|st| st.extras.phase_positions), [0, 0]);
    // The entry also drops a stale resume point: the walk starts at the
    // first lot and stores its location.
    s.update_state(|st| {
        st.extras.phase_positions = [7, 7];
        st.finish_resume = Some(1);
    });
    s.enter_cycle_state(cycle_state::FINISH_TRIP_MAP);
    assert_eq!(
        s.read_state(|st| (st.extras.phase_positions, st.finish_resume)),
        ([0, 0], None)
    );
    assert!(s.work_on_sim_cycle());
    assert_eq!(s.cycle_state(), cycle_state::CREATE_MAPS);
    assert_eq!(s.read_state(|st| st.extras.phase_positions), [1, 1]);
    // 16 -> 17 and 17 -> 18: each finish enters the next trip phase, whose
    // entry zeroes the positions again.
    for (from, to) in [
        (cycle_state::CONFIRM_TRIPS, cycle_state::REROUTE_TRIPS),
        (cycle_state::REROUTE_TRIPS, cycle_state::FULL_TRIPS),
    ] {
        s.update_state(|st| {
            st.cycle_state = from;
            st.current = None;
            st.extras.phase_positions = [5, 6];
        });
        s.enter_cycle_state(from);
        assert!(s.work_on_sim_cycle(), "state {from}");
        assert_eq!(s.cycle_state(), to);
        assert_eq!(s.read_state(|st| st.extras.phase_positions), [0, 0], "{from} -> {to}");
    }
    // Entering the lot sweep (1) shares the arm of 15.
    s.update_state(|st| st.extras.phase_positions = [3, 4]);
    s.enter_cycle_state(cycle_state::LOT_SWEEP);
    assert_eq!(s.read_state(|st| st.extras.phase_positions), [0, 0]);
}

/// §4.9 step 1 (TS40): the state-2 reset zeroes the working rows; a
/// separate comparison value starts at −1.0. A slot whose only record has
/// a NaN ratio never beats it, so its max fields stay 0 (not −1.0), and
/// its average is NaN.
#[test]
fn ts40_statistics_reset_and_nan_record() {
    let w = World::new();
    let s = sim(&w);
    s.network().update_state(|n| {
        let r = n.append_info(1, 5, 1).unwrap();
        r.inv_cap = 1.0;
        r.net_type = net_type::AVENUE as u8;
    });
    s.volume_grid(1).unwrap().set(5, 1, f32::NAN);
    s.update_state(|st| st.stats_work.rows[0].max = 9.0);
    s.enter_cycle_state(cycle_state::CREATE_MAPS);
    assert_eq!(s.read_state(|st| st.stats_work.rows[0]), StatRow::default());
    assert_eq!(s.read_state(|st| st.stat_max_cmp), [-1.0; 14]);
    assert!(s.work_on_sim_cycle());
    let row = s.read_state(|st| st.stats_last.rows[net_type::AVENUE as usize]);
    assert!(row.sum.is_nan());
    assert_eq!((row.congested, row.max_x, row.max_z, row.max.to_bits()), (0, 0, 0, 0));
    assert_eq!(s.read_state(|st| st.stats_last.records[net_type::AVENUE as usize]), 1);
    // The working rows of empty slots are saved as zeros.
    assert_eq!(s.read_state(|st| st.stats_work.rows[0]), StatRow::default());
}

/// traffic-network.md §2.7.2 step 5 (TS42): with IDs 1…65535 in use the
/// search wraps to key 0. The new holder is the map's own key-0 pool: the
/// record's pool is 0 (the default pool), but StartRecalc's search by city
/// position sets that holder's capacities, not the default pool's.
#[test]
fn ts42_pool_id_search_wraps_to_key_0() {
    let w = World::new();
    let road = 2u32;
    w.conns.borrow_mut().push((
        77,
        Conn {
            cell: (-1, 5),
            holder: 70,
            conn_type: Some(road),
            city: Some((10, 20)),
            ..Conn::default()
        },
    ));
    *w.cities.borrow_mut() = vec![NeighbourCity {
        position: (10, 20),
        population: [300, 1, 0],
    }];
    *w.demand.borrow_mut() = [Some(800.0), Some(10.0), Some(0.0)];
    let s = sim(&w);
    s.update_pools(|p| {
        for id in 1..=u16::MAX {
            p.map.insert(id, PropertyHolder::new());
        }
    });
    s.network().set_connection(0, 5, side::EAST, side::WEST, travel::CAR);
    neighbour_message(&s, true, road, -1, 5);
    assert!(s.pools().map.contains_key(&0));
    assert_eq!(s.pools().find(10, 20), Some(0));
    assert_eq!(s.network().traffic_neighbor_connection(-1, 5).pool, 0);
    s.enter_cycle_state(cycle_state::START_RECALC);
    assert!(s.work_on_sim_cycle());
    let caps = |t: u64| capacity_values(&s.holders(), t).unwrap();
    assert_eq!(
        caps(POOL_KEY0_TOKEN),
        vec![800, 10, 0, u32::MAX, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    assert_eq!(caps(pool_token(0)), vec![0, 0, 0, u32::MAX, 0, 0, 0, 0, 0, 0, 0, 0]);
    // The edge pass reads the default pool for record pool 0: bit 3 only.
    assert_eq!(s.destination_byte(-1, 5), 0b1000);
    // With key 0 taken too the original never ends; nothing is created.
    assert_eq!(s.update_pools(|p| p.create(1, 1)), None);
    // The sweep keeps key 0 while a swept record has pool 0.
    assert!(s.network().pool_ids_in_use().contains(&0));
}

/// §4.16 (TS46): the int64 cost is loaded exactly and the product with the
/// f32 fraction rounded once: 16,777,217 at 0.75 is 12,582,913 (12,582,912
/// when the cost is converted to f32 first).
#[test]
fn ts46_station_cost_rounds_once() {
    let w = World::new();
    let s = sim(&w);
    let dept = w.department_budget(dept::MASS_TRANSIT).expect("mass transit");
    w.funding.borrow_mut().insert(dept, 0.75);
    w.query_items.borrow_mut().insert(900, (dept::MASS_TRANSIT, 16_777_217));
    set_switches(&s, &[(3, 3, 5)], &[(5, 0.0)]);
    s.network()
        .update_state(|n| n.switches.get_mut(5).unwrap().building = 900);
    assert_eq!(s.transit_switch_query_data(3, 3).map(|q| q.cost), Some(12_582_913));
    // Below 2^24 both readings agree: 150 at 0.75 = 112.5 → 112.
    w.query_items.borrow_mut().insert(900, (dept::MASS_TRANSIT, 150));
    assert_eq!(s.transit_switch_query_data(3, 3).map(|q| q.cost), Some(112));
}

/// §4.1 (TS20): the O(1) step of an unchanged list gives the same walk as
/// the general rule. A reference walk (the rule as written, every step)
/// against the iterator over random lists: unchanged (shared or copied),
/// a lot appended, a lot removed, a lot listed twice.
#[test]
fn ts20_source_iterator_fast_path_is_the_rule() {
    use std::collections::HashSet;
    use std::rc::Rc;
    struct Reference {
        seen: Vec<u64>,
        last: Option<u64>,
        done: bool,
    }
    impl Reference {
        fn next(&mut self, lots: &[u64]) -> Option<u64> {
            if self.done {
                return None;
            }
            let next = match self.last {
                None => lots.first().copied(),
                Some(last) => {
                    let now: HashSet<u64> = lots.iter().copied().collect();
                    let seen: HashSet<u64> = self.seen.iter().copied().collect();
                    let after = match self.seen.iter().position(|&l| l == last) {
                        Some(i) => &self.seen[i + 1..],
                        None => &[][..],
                    };
                    after
                        .iter()
                        .copied()
                        .find(|l| now.contains(l))
                        .or_else(|| lots.iter().copied().find(|l| !seen.contains(l)))
                }
            };
            self.seen = lots.to_vec();
            match next {
                Some(l) => {
                    self.last = Some(l);
                    self.done = lots.iter().position(|&x| x == l) == Some(lots.len() - 1);
                }
                None => self.done = true,
            }
            next
        }
    }
    let mut seed = 0x1234_5678u32;
    let mut rand = |n: u32| {
        seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        (seed >> 8) % n
    };
    for round in 0..300 {
        let mut it = SourceIter::default();
        it.reset(Vec::new());
        let mut r = Reference {
            seen: Vec::new(),
            last: None,
            done: false,
        };
        let mut list: Vec<u64> = (1..=rand(12) as u64).collect();
        let mut fresh = 100u64;
        let mut shared: Rc<[u64]> = list.clone().into();
        for step in 0..60 {
            match rand(10) {
                0 => {
                    fresh += 1;
                    list.push(fresh);
                }
                1 if !list.is_empty() => {
                    let i = rand(list.len() as u32) as usize;
                    list.remove(i);
                }
                2 if !list.is_empty() && round % 3 == 0 => {
                    let i = rand(list.len() as u32) as usize;
                    list.push(list[i]);
                }
                _ => {}
            }
            // Mostly the same shared list, as the host hands it out.
            if *shared != *list || rand(4) == 0 {
                shared = list.clone().into();
            }
            let want = r.next(&list);
            assert_eq!(it.next_lot(shared.clone()), want, "round {round} step {step}");
            assert_eq!(
                (it.last_lot, it.lots_done),
                (r.last, r.done),
                "round {round} step {step}"
            );
            if r.done {
                break;
            }
        }
    }
}

// ---- re-answers13 (TS54-TS65) ----

/// The bytes of one named field of the record (the marks of `write_record`).
fn record_field(s: &TrafficSimulator, name: &str) -> Vec<u8> {
    let mut w = shadow::CanonicalWriter::new();
    assert!(s.write(&mut w));
    let (_, a, b) = w
        .fields()
        .into_iter()
        .find(|f| f.0 == name)
        .unwrap_or_else(|| panic!("no field '{name}'"));
    w.bytes[a..b].to_vec()
}

fn le32(v: &[i32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// A lot trip with a destination.
fn trip(count: u32, ty: u8, strategy: u8, flags: u8, st: u8, dest: (i16, i16)) -> TripData {
    let mut t = TripData::new(count, ty, strategy, flags, st);
    (t.dest_x, t.dest_z) = dest;
    t
}

/// §4.3 merge (TS55): a routed trip absorbs every later trip of its type
/// and strategy, whatever their status, and goes back to status 0 only when
/// it absorbed one; a routed trip with no partner stays routed. The return
/// value is pass counter 5's step.
#[test]
fn ts55_confirm_trips_merge() {
    let mut l = vec![
        trip(5, 0, 1, 0, status::ROUTED, (4, 4)),
        trip(2, 0, 1, 0, status::SEARCH, (0, 0)),
        trip(3, 1, 1, 0, status::ROUTED, (4, 4)),
        trip(4, 0, 1, 0, status::FAILED, (0, 0)),
        trip(6, 0, 2, 0, status::ROUTED, (4, 4)),
        trip(1, 0, 2, 0, status::NEW, (0, 0)),
    ];
    assert_eq!(merge_routed(&mut l), 2);
    let got: Vec<(u32, u8, u8, u8)> = l.iter().map(|t| (t.count, t.trip_type, t.strategy, t.status)).collect();
    assert_eq!(
        got,
        vec![
            (11, 0, 1, status::SEARCH),
            (3, 1, 1, status::ROUTED),
            (7, 0, 2, status::SEARCH)
        ]
    );
    // A status-0 or failed trip absorbs nothing in ConfirmTrips.
    let mut l = vec![
        trip(1, 0, 0, 0, status::SEARCH, (0, 0)),
        trip(2, 0, 0, 0, status::SEARCH, (0, 0)),
    ];
    assert_eq!(merge_routed(&mut l), 0);
    assert_eq!(l.len(), 2);
}

/// §4.4 merge (TS57): each status-0 trip absorbs every later trip of its
/// type and strategy whatever its status (0, 1, 3, 4, 5); a routed trip
/// before it absorbs nothing.
#[test]
fn ts57_reroute_trips_merge() {
    let mut l = vec![
        trip(9, 0, 1, 0, status::ROUTED, (4, 4)),
        trip(1, 0, 1, 0, status::SEARCH, (0, 0)),
        trip(2, 0, 1, 0, status::REROUTE, (4, 4)),
        trip(3, 0, 1, 0, status::ROUTED, (4, 4)),
        trip(4, 0, 1, 0, status::NEW, (0, 0)),
        trip(5, 0, 1, 0, status::FAILED, (0, 0)),
        trip(6, 1, 1, 0, status::ROUTED, (4, 4)),
    ];
    merge_search(&mut l);
    let got: Vec<(u32, u8, u8)> = l.iter().map(|t| (t.count, t.trip_type, t.status)).collect();
    assert_eq!(
        got,
        vec![(9, 0, status::ROUTED), (15, 0, status::SEARCH), (6, 1, status::ROUTED)]
    );
}

/// §4.3, §10 Q4 (TS59, TS55): one re-route source of ConfirmTrips. Counter
/// 3 counts the source; 7 the two status-0 trips met before the merge; 5
/// the one merge; 4 the routed trip whose destination is full; 8 the
/// failed trip retried. Every trip ends status 0.
#[test]
fn ts59_confirm_trips_counters_reroute_source() {
    use super::state::{PC_CAPACITY_SEND_BACKS, PC_FAILURES_RETRIED, PC_MERGES, PC_SEARCH_TRIPS_MET};
    let (w, s) = test10();
    // Lot 2's holder: max[1] = 2, so a type-1 trip of 3 does not fit.
    set_capacity(&w, 12, &[100, 2]);
    w.set_trips(
        Source::Lot(1),
        vec![
            trip(5, 0, 1, 0, status::ROUTED, (4, 4)),
            trip(2, 0, 1, 0, status::SEARCH, (0, 0)),
            trip(3, 1, 1, 0, status::ROUTED, (4, 4)),
            trip(4, 2, 0, 0, status::FAILED, (0, 0)),
            trip(1, 2, 0, 0, status::SEARCH, (0, 0)),
        ],
    );
    // Source counter 3 becomes 1 = rerouteBucket: a re-route source.
    s.update_state(|st| st.pass_counters[PC_REROUTE_BUCKET] = 1);
    s.confirm_source(Source::Lot(1));
    let got: Vec<(u32, u8)> = w
        .get_trips(Source::Lot(1))
        .iter()
        .map(|t| (t.count, t.status))
        .collect();
    assert_eq!(got, vec![(7, 0), (3, 0), (4, 0), (1, 0)]);
    let c = s.read_state(|st| st.pass_counters);
    assert_eq!(c[PC_SOURCE_COUNTER], 1);
    assert_eq!(c[PC_SEARCH_TRIPS_MET], 2);
    assert_eq!(c[PC_MERGES], 1);
    assert_eq!(c[PC_CAPACITY_SEND_BACKS], 1);
    assert_eq!(c[PC_FAILURES_RETRIED], 1);
    assert_eq!(c, [0, 0, 1, 1, 1, 1, 0, 2, 1, 0]);
    assert_eq!(
        s.read_state(|st| (st.to_search, st.to_reroute, st.processed)),
        (4, 0, 4)
    );
}

/// §4.3, §10 Q4 (TS59): a source that is not re-routed confirms old routes.
/// A reversible routed trip whose route confirms stays routed (counter 0);
/// a two-leg trip whose return route does not confirm stays status 1
/// (counter 6).
#[test]
fn ts59_confirm_trips_counters_0_and_6() {
    use super::state::{PC_CONFIRMED, PC_RETURN_NOT_CONFIRMED};
    let (w, s) = test10();
    s.network().update_state(|n| {
        n.conn_mut(3, 3).unwrap().conn[1] = 1 << 2;
        n.conn_mut(2, 3).unwrap().conn[1] = 1 << 2;
    });
    let mut two_leg = trip(1, 0, 1, 0, status::ROUTED, (4, 4));
    two_leg.route = ROUTE_10.to_vec();
    let mut reversible = trip(
        1,
        0,
        2,
        trip_flag::REVERSIBLE | trip_flag::PROBE,
        status::ROUTED,
        (4, 4),
    );
    reversible.route = ROUTE_10.to_vec();
    w.set_trips(Source::Lot(1), vec![two_leg, reversible]);
    s.confirm_source(Source::Lot(1));
    let got: Vec<u8> = w.get_trips(Source::Lot(1)).iter().map(|t| t.status).collect();
    assert_eq!(got, vec![status::REROUTE, status::ROUTED]);
    let c = s.read_state(|st| st.pass_counters);
    assert_eq!((c[PC_CONFIRMED], c[PC_RETURN_NOT_CONFIRMED]), (1, 1));
    assert_eq!(c, [1, 0, 0, 1, 0, 0, 1, 0, 0, 0]);
    assert_eq!(
        s.read_state(|st| (st.to_reroute, st.to_search, st.processed)),
        (1, 0, 2)
    );
}

/// §4.4, §6.1 row 19 (TS56): the end of FullTrips swaps the building and
/// published count vectors; the record holds (published, building) pairs.
/// Entering 18 zeroes the building vector only.
#[test]
fn ts56_travel_type_count_swap_and_pairs() {
    let w = World::new();
    let s = sim(&w);
    let a: [u32; 9] = std::array::from_fn(|i| i as u32 + 1);
    let b: [u32; 9] = std::array::from_fn(|i| 100 + i as u32);
    s.update_state(|st| {
        st.cycle_state = cycle_state::FULL_TRIPS;
        st.type_count_building = a;
        st.type_count_published = b;
    });
    // No source: FullTrips finishes at once.
    assert!(s.work_on_sim_cycle());
    assert_eq!(
        s.read_state(|st| (st.type_count_published, st.type_count_building)),
        (a, b)
    );
    let want: Vec<u8> = (0..9)
        .flat_map(|i| [a[i] as i32, b[i] as i32])
        .flat_map(|v| v.to_le_bytes())
        .collect();
    assert_eq!(record_field(&s, "travel type counts"), want);
    // The record reads back the same way.
    let (bytes, entries) = record(&s);
    let w2 = World::new();
    let s2 = sim(&w2);
    assert!(s2.read(&mut ByteReader::with_resolver(
        &bytes,
        &mut Resolver(entries, w2.clone())
    )));
    assert_eq!(
        s2.read_state(|st| (st.type_count_published, st.type_count_building)),
        (a, b)
    );
    s.enter_cycle_state(cycle_state::FULL_TRIPS);
    assert_eq!(
        s.read_state(|st| (st.type_count_published, st.type_count_building)),
        (a, [0; 9])
    );
}

/// §6.1 rows 14 and 15 (TS60): the day index is the s32 after `processed`,
/// then total, days and three unused u32; taking a source writes its cell
/// into the re-route context cell (the two s16 widened).
#[test]
fn ts60_day_index_and_context_cell() {
    let w = World::new();
    w.add_lot(1, Lot::new(CellRect::new(3, 4, 3, 4), 11));
    let s = sim(&w);
    s.set_traffic_units_per_slice(1);
    s.enter_cycle_state(cycle_state::CONFIRM_TRIPS);
    assert!(!s.work_on_sim_cycle());
    assert_eq!(s.read_state(|st| (st.extras.context_x, st.extras.context_z)), (3, 4));
    assert_eq!(record_field(&s, "re-route context cell"), le32(&[3, 4]));
    s.update_state(|st| {
        st.processed = 5;
        st.day_index = 2;
        st.total = 40;
        st.days = 3;
        st.extras.pacing_unused = [7, 8, 9];
    });
    assert_eq!(record_field(&s, "processed"), le32(&[5]));
    assert_eq!(record_field(&s, "day index"), le32(&[2]));
    assert_eq!(record_field(&s, "total"), le32(&[40]));
    assert_eq!(record_field(&s, "days"), le32(&[3]));
    assert_eq!(record_field(&s, "~pacing unused u32 x3"), le32(&[7, 8, 9]));
    let (bytes, entries) = record(&s);
    let w2 = World::new();
    let s2 = sim(&w2);
    assert!(s2.read(&mut ByteReader::with_resolver(
        &bytes,
        &mut Resolver(entries, w2.clone())
    )));
    assert_eq!(
        s2.read_state(|st| (
            st.processed,
            st.day_index,
            st.total,
            st.days,
            st.extras.pacing_unused,
            st.extras.context_x,
            st.extras.context_z
        )),
        (5, 2, 40, 3, [7, 8, 9], 3, 4)
    );
}

/// §4.12 FinalCalculations (TS61): `freightByte` is the last byte of the
/// published statistics set, recomputed: 3.0 / 2 = 1.5, / 6.0 = 0.25,
/// × 255 = 63.75 → 63. The working set's byte is not touched.
#[test]
fn ts61_freight_byte_is_the_published_set_byte() {
    let w = World::new();
    let s = sim(&w);
    s.update_state(|st| {
        st.cycle_state = cycle_state::FINAL_CALCULATIONS;
        st.freight_len_sum = 3.0;
        st.freight_count = 2;
        st.tuning.commute_max_time = 6.0;
        st.stats_last.byte = 0x1C;
        st.stats_work.byte = 0x1C;
    });
    assert!(s.work_on_sim_cycle());
    assert_eq!(s.read_state(|st| (st.stats_last.byte, st.stats_work.byte)), (63, 0x1C));
    assert_eq!(record_field(&s, "statistics (published)").last(), Some(&63));
    assert_eq!(record_field(&s, "statistics (working)").last(), Some(&0x1C));
    // No freight: 0.
    s.update_state(|st| {
        st.cycle_state = cycle_state::FINAL_CALCULATIONS;
        st.freight_count = 0;
    });
    assert!(s.work_on_sim_cycle());
    assert_eq!(s.read_state(|st| st.stats_last.byte), 0);
}

/// traffic-network.md §4.12, §3.2 (TS64): entering 13 resets the scanning
/// iterator to state 0 at (0, 0); a stop between two cells leaves it
/// running at the last cell visited, unpublished; the finished pass leaves
/// it done at (x0, z1 + 1) = (0, 8) for the 8 × 8 city, published.
/// Entering 12 does not touch it.
#[test]
fn ts64_subnetwork_info_scan() {
    use crate::grid::ScanState;
    let w = World::new();
    let s = sim(&w);
    let scan = |s: &TrafficSimulator| s.read_state(|st| (st.scan.state, st.scan.x, st.scan.z));
    s.update_state(|st| {
        st.scan.state = ScanState::Done;
        st.scan.x = 5;
        st.scan.z = 6;
    });
    s.enter_cycle_state(cycle_state::UPDATE_SUBNETWORKS);
    assert_eq!(scan(&s), (ScanState::Done, 5, 6));
    s.network().update_state(|n| {
        n.subnet_counter = 3;
        n.subnet_published.clear();
    });
    s.enter_cycle_state(cycle_state::SUBNETWORK_INFO);
    assert_eq!(scan(&s), (ScanState::Fresh, 0, 0));
    s.update_state(|st| st.shadow_scan_stop = Some((cycle_state::SUBNETWORK_INFO, 3, 2)));
    assert!(!s.work_on_sim_cycle());
    assert_eq!(s.cycle_state(), cycle_state::SUBNETWORK_INFO);
    assert_eq!(scan(&s), (ScanState::Running, 3, 2));
    assert!(s.network().state().subnet_published.is_empty());
    s.update_state(|st| st.shadow_scan_stop = None);
    assert!(s.work_on_sim_cycle());
    assert_eq!(s.cycle_state(), cycle_state::FERRY_ROUTES);
    assert_eq!(scan(&s), (ScanState::Done, 0, 8));
    assert_eq!(s.network().state().subnet_published.len(), 3);
}

/// §6.2 (TS54): Read inserts each key at the head of its chain in file
/// order, in a container of 193 buckets that grows before the insert that
/// would overflow it. Keys 5 and 198 share bucket 5: the writer's order
/// (198, 5; 198 inserted last) reads back as (5, 198). 194 keys grow the
/// read container to 389 buckets.
#[test]
fn ts54_hash_order_after_read() {
    use crate::traffic_network::TrafficEdgeDensity;
    let round = |s: &TrafficSimulator| {
        let (bytes, entries) = record(s);
        let w2 = World::new();
        let s2 = sim(&w2);
        assert!(s2.read(&mut ByteReader::with_resolver(
            &bytes,
            &mut Resolver(entries, w2.clone())
        )));
        s2
    };
    let w = World::new();
    let s = sim(&w);
    s.network().update_state(|n| {
        for k in [5u32, 198, 1] {
            n.edge_density.entry(k, TrafficEdgeDensity::default).usage = k as u16;
        }
    });
    s.update_damage(|d| {
        d.road.intact.insert(5);
        d.road.intact.insert(198);
    });
    assert_eq!(s.network().state().edge_density.keys(), vec![1, 198, 5]);
    assert_eq!(s.damage().road.intact.iter_order(), vec![198, 5]);
    let s2 = round(&s);
    assert_eq!(s2.network().state().edge_density.keys(), vec![1, 5, 198]);
    assert_eq!(s2.network().state().edge_density.get(198).map(|r| r.usage), Some(198));
    assert_eq!(s2.damage().road.intact.iter_order(), vec![5, 198]);
    // Growth.
    s.network().update_state(|n| {
        for k in 0..194u32 {
            n.edge_density.entry(k + 1000, TrafficEdgeDensity::default);
        }
    });
    let s3 = round(&s);
    assert_eq!(s3.network().state().edge_density.len(), 197);
    assert_eq!(s3.network().state().edge_density.bucket_count(), 389);
}

#[path = "shadow_tests.rs"]
mod shadow_tests;
