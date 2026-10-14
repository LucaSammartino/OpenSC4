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
use crate::traffic_network::{net_type, NetworkHost, PathInfo};
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

/// A scripted neighbour connection.
#[derive(Clone, Debug)]
struct Conn {
    cell: (i32, i32),
    holder: u64,
    dirty: bool,
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
    fn occupant_position(&self, _occ: u64) -> [f32; 3] {
        [0.0; 3]
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
    /// Fixture stand-in (TS34): an unset department is at 100 % funding,
    /// as a department `CreateDepartmentBudget` makes (budget.md §6).
    fn department_funding(&self, department: u64) -> f32 {
        self.funding.borrow().get(&department).copied().unwrap_or(1.0)
    }
    fn set_line_item_income(&self, department: u64, item: u32, income: i64) {
        self.incomes.borrow_mut().push((department, item, income));
    }
    fn update_network_costs(&self, _department: u64, _mass_transit: bool, _cells: &[u32; 13], _costs: &[f32; 13]) {}
    fn activate_budget_items(&self, _building: u64, _lot: Option<u64>) -> i64 {
        0
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
    fn find_connection(&self, _connection_type: u32, x: i32, z: i32) -> Option<u64> {
        self.conns
            .borrow()
            .iter()
            .find(|(_, c)| c.cell == (x, z))
            .map(|(id, _)| *id)
    }
    fn neighbor_pool_property_holder(&self, _t: u32, _x: i32, _z: i32) -> Option<u64> {
        None
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
    fn as_network_occupant(&self, _object: u64) -> Option<u64> {
        None
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
    // r = 2.5 > 1: one over-capacity street cell per route cell.
    assert_eq!(s.read_state(|st| st.stats_last.over[net_type::STREET as usize]), 3);
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
