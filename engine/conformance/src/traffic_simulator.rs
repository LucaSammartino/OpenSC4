// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/traffic-simulator.md`: a scripted game
//! implementing `TrafficSimHost` (lots with trip lists and trip capacities,
//! neighbour connections, departments, dates and the simulation clock, the
//! developer populations of the background traffic, the carpool ordinance,
//! the view, history, posted messages) over core SimGrids, and ops that drive
//! `fc4_core::traffic_simulator::TrafficSimulator` with its own network model
//! and pathfinders. See README.md ("Traffic simulator ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

use fc4_core::curve::ResponseCurve;
use fc4_core::grid::{CellRect, CityDims, CoreGrids, GridProvider, TractGrid};
use fc4_core::property::PropertyHolder;
use fc4_core::rng::{FixedSeeds, RzRandomSuper};
use fc4_core::stream::{ByteReader, ByteWriter, RefResolver, RefValue, RegisteredObject, SaveRegistry};
use fc4_core::traffic_network::{NetworkHost, PathInfo};
use fc4_core::traffic_simulator::{
    air_polluting_byte, background_value, capacity_element, cell_key16, congestion_byte, damage_target, decode_route,
    industrial_trips, invert_congestion_curve, legacy_density, read_trip, reconcile_connection, reconcile_lot,
    residential_commuters, residential_split, trip_length_byte, write_trip, AccidentEntry, DamageBlock, Message,
    SimMessage, SimTuning, Source, TrafficSimHost, TrafficSimulator, TripData, CARPOOL_ORDINANCE,
};
use fc4_core::variant::{mem, Elem, Variant};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::HostRef;
use crate::scalar::{self, arg, arr, bool_of, check_expect, i64_of, ji, matches, opt, show, str_of, u32_of, R};

// ---- the scripted game -------------------------------------------------------------------------------

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

/// A scripted neighbour connection.
#[derive(Clone, Debug)]
struct Conn {
    cell: (i32, i32),
    holder: u64,
    dirty: bool,
}

/// The department tokens: `dept_base + i` in creation order (a fixture
/// stand-in with no original counterpart, TS34).
const DEPT_BASE: u64 = 900;
/// The class ID of the host references the world writes.
const HOST_CLASS: u32 = 0x1234_5678;

/// A saved record and the registry entries its references need.
type Saved = (Vec<u8>, Vec<RegisteredObject>);

struct World {
    cells: Cell<(i32, i32)>,
    shipped: Cell<bool>,
    lots: RefCell<Vec<(u64, Lot)>>,
    conns: RefCell<Vec<(u64, Conn)>>,
    holders: RefCell<HashMap<u64, BTreeMap<u32, Variant>>>,
    trips: RefCell<BTreeMap<Source, Vec<TripData>>>,
    posted: RefCell<Vec<SimMessage>>,
    agents: RefCell<Vec<u32>>,
    subs: RefCell<Vec<u32>>,
    day: Cell<u32>,
    date: Cell<u32>,
    clock: Cell<u64>,
    departments: RefCell<Vec<(u32, f32)>>,
    dept_base: Cell<u64>,
    funding: RefCell<HashMap<u32, f32>>,
    incomes: RefCell<Vec<(u64, u32, i64)>>,
    history: RefCell<Vec<(u32, u32)>>,
    workforce: Cell<f32>,
    dev_pop: RefCell<HashMap<(u32, i32, i32), i16>>,
    carpool: Cell<bool>,
    view: Cell<Option<CellRect>>,
    accidents: RefCell<Vec<(i32, i32, i64)>>,
    refs: RefCell<BTreeMap<u64, i32>>,
    released_departments: RefCell<Vec<u64>>,
    budget_items: Cell<i64>,
    sim: RefCell<Option<Rc<TrafficSimulator>>>,
    saved: RefCell<HashMap<String, Saved>>,
}

fn inside(r: CellRect, x: i32, z: i32) -> bool {
    x >= r.x1 && x <= r.x2 && z >= r.z1 && z <= r.z2
}

impl World {
    fn new() -> World {
        World {
            cells: Cell::new((8, 8)),
            shipped: Cell::new(true),
            lots: RefCell::new(Vec::new()),
            conns: RefCell::new(Vec::new()),
            holders: RefCell::new(HashMap::new()),
            trips: RefCell::new(BTreeMap::new()),
            posted: RefCell::new(Vec::new()),
            agents: RefCell::new(Vec::new()),
            subs: RefCell::new(Vec::new()),
            day: Cell::new(5),
            date: Cell::new(1),
            clock: Cell::new(0),
            departments: RefCell::new(Vec::new()),
            dept_base: Cell::new(DEPT_BASE),
            funding: RefCell::new(HashMap::new()),
            incomes: RefCell::new(Vec::new()),
            history: RefCell::new(Vec::new()),
            workforce: Cell::new(1.0),
            dev_pop: RefCell::new(HashMap::new()),
            carpool: Cell::new(false),
            view: Cell::new(None),
            accidents: RefCell::new(Vec::new()),
            refs: RefCell::new(BTreeMap::new()),
            released_departments: RefCell::new(Vec::new()),
            budget_items: Cell::new(0),
            sim: RefCell::new(None),
            saved: RefCell::new(HashMap::new()),
        }
    }

    fn lot(&self, id: u64) -> Option<Lot> {
        self.lots
            .borrow()
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, l)| l.clone())
    }

    fn conn(&self, id: u64) -> Option<Conn> {
        self.conns
            .borrow()
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, c)| c.clone())
    }

    fn prop(&self, holder: u64, id: u32) -> Option<Variant> {
        self.holders.borrow().get(&holder).and_then(|m| m.get(&id).cloned())
    }

    fn set_prop(&self, holder: u64, id: u32, v: Variant) {
        self.holders.borrow_mut().entry(holder).or_default().insert(id, v);
    }

    fn dims(&self) -> CityDims {
        let (x, z) = self.cells.get();
        CityDims {
            cells_x: x,
            cells_z: z,
            ..CityDims::square(1)
        }
    }

    fn sim(&self) -> R<Rc<TrafficSimulator>> {
        self.sim
            .borrow()
            .clone()
            .ok_or_else(|| "no simulator (ts_init first)".to_string())
    }

    /// The department index of a token (0 = roads, 1 = mass transit of the
    /// first Init).
    fn dept_id(&self, token: u64) -> Option<u32> {
        let i = token.checked_sub(self.dept_base.get())? as usize;
        self.departments.borrow().get(i).map(|d| d.0)
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
    // The simulator answers these three itself (its adapter never asks).
    fn max_trip_capacity(&self, holder: u64, k: u32) -> u32 {
        capacity_element(self, holder, k as usize)
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
        self.conns.borrow().iter().map(|(id, _)| *id).collect()
    }
    fn connection_cell(&self, conn: u64) -> (i32, i32) {
        self.conn(conn).map_or((0, 0), |c| c.cell)
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
    fn add_agent(&self, level: u32, _name: &str, _flags: u32) {
        self.agents.borrow_mut().push(level);
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
        self.shipped.get().then(|| Rc::new(SimTuning::spec_exemplar()))
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
    /// Fixture stand-in (TS34): an unset department is at 100 % funding, as
    /// a department `CreateDepartmentBudget` makes (budget.md §6).
    fn department_funding(&self, department: u64) -> f32 {
        self.dept_id(department)
            .and_then(|id| self.funding.borrow().get(&id).copied())
            .unwrap_or(1.0)
    }
    fn set_line_item_income(&self, department: u64, item: u32, income: i64) {
        self.incomes.borrow_mut().push((department, item, income));
    }
    fn update_network_costs(&self, _department: u64, _mass_transit: bool, _cells: &[u32; 13], _costs: &[f32; 13]) {}
    fn activate_budget_items(&self, _building: u64, _lot: Option<u64>) -> i64 {
        self.budget_items.get()
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
        self.trips.borrow().get(&source).cloned().unwrap_or_default()
    }
    fn set_trip_list(&self, source: Source, trips: Vec<TripData>) {
        self.trips.borrow_mut().insert(source, trips);
    }
    fn connection_holder(&self, connection: u64) -> u64 {
        self.conn(connection).map_or(0, |c| c.holder)
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

/// The registry entries of a save, resolved back for the reader (host
/// objects come with a reference).
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

// ---- dispatch -----------------------------------------------------------------------------------------

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("ts_")?;
    let w = scalar::world(host, "traffic_simulator", || Rc::new(World::new()));
    Some(match rest {
        "env" => env(&w, op),
        "lot" => lot(&w, op),
        "connection" => connection(&w, op),
        "trips" => set_trips(&w, op),
        "capacity" => set_capacity(&w, op),
        "prop" => set_prop(&w, op),
        "init" => init(&w, op),
        "info" => info(&w, op),
        "street" => street(&w, op),
        "state" => state(&w, op),
        "damage" => damage(&w, op),
        "grid" => set_grid(&w, op),
        "call" => call(&w, op),
        "eval" => eval(op),
        "save" => save(&w, op),
        "load" => load(&w, op),
        "expect_same_save" => expect_same_save(&w, op),
        "expect_state" => expect_state(&w, op),
        "expect_grid" => expect_grid(&w, op),
        "expect_edge" => expect_edge(&w, op),
        "expect_world" => expect_world(&w, op),
        "expect_prop" => expect_prop(&w, op),
        "expect_trips" => expect_trips(&w, op),
        "expect_damage" => expect_damage(&w, op),
        "expect_network" => expect_network(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    })
}

// ---- parsing helpers ----------------------------------------------------------------------------------

fn field<'a>(op: &'a Json, key: &str) -> R<&'a Json> {
    opt(op, key).ok_or_else(|| format!("missing field '{key}'"))
}

fn i32_of(j: &Json, what: &str) -> R<i32> {
    Ok(i64_of(j, what)? as i32)
}

fn u64_of(j: &Json, what: &str) -> R<u64> {
    Ok(i64_of(j, what)? as u64)
}

fn pair(j: &Json, what: &str) -> R<(i32, i32)> {
    match arr(j, what)? {
        [x, z] => Ok((i32_of(x, what)?, i32_of(z, what)?)),
        _ => Err(format!("'{what}' must be [x, z]")),
    }
}

fn rect_of(j: &Json, what: &str) -> R<CellRect> {
    match arr(j, what)? {
        [a, b, c, d] => Ok(CellRect::new(
            i32_of(a, what)?,
            i32_of(b, what)?,
            i32_of(c, what)?,
            i32_of(d, what)?,
        )),
        _ => Err(format!("'{what}' must be [x1, z1, x2, z2]")),
    }
}

/// A float: a number, or its exact bit pattern as a `"0x…"` string.
fn fv(j: &Json, what: &str) -> R<f32> {
    match j.as_str() {
        Some(s) if s.starts_with("0x") || s.starts_with("0X") => u32::from_str_radix(&s[2..], 16)
            .map(f32::from_bits)
            .map_err(|_| format!("'{what}' must be a float or \"0x…\" bits")),
        _ => scalar::f32_of(j, what),
    }
}

/// A double: a number, or its bit pattern as `"0x…"` (16 hex digits).
fn dv(j: &Json, what: &str) -> R<f64> {
    match j.as_str() {
        Some(s) if s.starts_with("0x") || s.starts_with("0X") => u64::from_str_radix(&s[2..], 16)
            .map(f64::from_bits)
            .map_err(|_| format!("'{what}' must be a number or \"0x…\" bits")),
        _ => scalar::num(j).ok_or_else(|| format!("'{what}' must be a number")),
    }
}

/// A float as its bit pattern (compared with `"0x…"` strings exactly).
fn fb(v: f32) -> Json {
    ji(v.to_bits())
}

/// A double as its bit pattern.
fn db(v: f64) -> Json {
    Json::Str(format!("0x{:016X}", v.to_bits()))
}

fn jb(b: bool) -> Json {
    Json::Bool(b)
}

fn jcell((x, z): (i32, i32)) -> Json {
    Json::Array(vec![ji(x), ji(z)])
}

fn jrect(r: CellRect) -> Json {
    Json::Array(vec![ji(r.x1), ji(r.z1), ji(r.x2), ji(r.z2)])
}

fn jlist<T: Copy + Into<i64>>(v: &[T]) -> Json {
    Json::Array(v.iter().map(|&x| ji(x)).collect())
}

fn obj(fields: Vec<(&str, Json)>) -> Json {
    Json::Object(fields.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
}

fn iarg(op: &Json, i: usize) -> R<i32> {
    i32_of(arg(op, i)?, "argument")
}

fn uarg(op: &Json, i: usize) -> R<u32> {
    u32_of(arg(op, i)?, "argument")
}

fn farg(op: &Json, i: usize) -> R<f32> {
    fv(arg(op, i)?, "argument")
}

fn cells_of(op: &Json) -> R<Vec<(i32, i32)>> {
    if let Some(c) = opt(op, "cells") {
        return arr(c, "cells")?.iter().map(|c| pair(c, "cells")).collect();
    }
    if let Some(r) = opt(op, "rect") {
        let r = rect_of(r, "rect")?;
        let mut out = Vec::new();
        for z in r.z1..=r.z2 {
            for x in r.x1..=r.x2 {
                out.push((x, z));
            }
        }
        return Ok(out);
    }
    Ok(vec![pair(
        opt(op, "cell").ok_or("missing 'cell', 'cells' or 'rect'")?,
        "cell",
    )?])
}

/// A source: `["lot", id]` or `["connection", id]`.
fn source_of(j: &Json) -> R<Source> {
    match arr(j, "source")? {
        [k, id] => {
            let id = u64_of(id, "source id")?;
            match str_of(k, "source kind")? {
                "lot" => Ok(Source::Lot(id)),
                "connection" => Ok(Source::Connection(id)),
                other => Err(format!("unknown source kind '{other}'")),
            }
        }
        _ => Err("a source is [\"lot\" | \"connection\", id]".into()),
    }
}

fn hex_of(j: &Json, what: &str) -> R<Vec<u8>> {
    hex_decode(str_of(j, what)?)
}

/// A trip: `{count, type, strategy, flags, status, dest: [x, z], length,
/// mask, route, return_route}` (missing fields 0 / empty).
fn trip_of(j: &Json) -> R<TripData> {
    let n = |k: &str| -> R<i64> { opt(j, k).map_or(Ok(0), |v| i64_of(v, k)) };
    let mut t = TripData {
        count: n("count")? as u32,
        trip_type: n("type")? as u8,
        strategy: n("strategy")? as u8,
        flags: n("flags")? as u8,
        status: n("status")? as u8,
        travel_types_used: n("mask")? as u32,
        ..TripData::default()
    };
    if let Some(d) = opt(j, "dest") {
        let (x, z) = pair(d, "dest")?;
        t.dest_x = x as i16;
        t.dest_z = z as i16;
    }
    if let Some(v) = opt(j, "length") {
        t.trip_length = fv(v, "length")?;
    }
    if let Some(v) = opt(j, "route") {
        t.route = hex_of(v, "route")?;
    }
    if let Some(v) = opt(j, "return_route") {
        t.return_route = hex_of(v, "return_route")?;
    }
    Ok(t)
}

fn jtrip(t: &TripData) -> Json {
    obj(vec![
        ("count", ji(t.count)),
        ("type", ji(t.trip_type)),
        ("strategy", ji(t.strategy)),
        ("flags", ji(t.flags)),
        ("status", ji(t.status)),
        ("dest", jcell((i32::from(t.dest_x), i32::from(t.dest_z)))),
        ("length", fb(t.trip_length)),
        ("mask", ji(t.travel_types_used)),
        ("route", Json::Str(hex_encode(&t.route))),
        ("return_route", Json::Str(hex_encode(&t.return_route))),
    ])
}

fn trips_of(j: &Json) -> R<Vec<TripData>> {
    arr(j, "trips")?.iter().map(trip_of).collect()
}

fn u16_variant(vals: &[u32]) -> Variant {
    let mut v: Vec<u32> = vals.to_vec();
    v.resize(12, 0);
    Variant::array(mem::UINT16, v.into_iter().map(|x| Elem::U16(x as u16)).collect())
}

fn u32s(j: &Json, what: &str) -> R<Vec<u32>> {
    arr(j, what)?.iter().map(|v| u32_of(v, what)).collect()
}

/// Compares every key of `want` with the same key of `got`.
fn check_fields(got: &Json, want: &Json, what: &str) -> R<()> {
    let want = want.as_object().ok_or_else(|| format!("'{what}' must be an object"))?;
    let mut errors = Vec::new();
    for (k, v) in want {
        match got.get(k) {
            None => errors.push(format!("unknown {what} field '{k}'")),
            Some(g) => {
                if !matches(g, v, true) {
                    errors.push(format!("{k}: got {}, expected {}", show(g), show(v)));
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

// ---- setup ----------------------------------------------------------------------------------------------

fn env(w: &World, op: &Json) -> R<()> {
    if let Some(c) = opt(op, "cells") {
        let (x, z) = pair(c, "cells")?;
        w.cells.set((x, z));
    }
    if let Some(t) = opt(op, "tuning") {
        match str_of(t, "tuning")? {
            "shipped" => w.shipped.set(true),
            "code" => w.shipped.set(false),
            other => return Err(format!("unknown tuning '{other}'")),
        }
    }
    if let Some(v) = opt(op, "day") {
        w.day.set(u32_of(v, "day")?);
    }
    if let Some(v) = opt(op, "date") {
        w.date.set(u32_of(v, "date")?);
    }
    if let Some(v) = opt(op, "clock") {
        w.clock.set(u64_of(v, "clock")?);
    }
    if let Some(v) = opt(op, "workforce") {
        w.workforce.set(fv(v, "workforce")?);
    }
    if let Some(v) = opt(op, "carpool") {
        w.carpool.set(bool_of(v, "carpool")?);
    }
    if let Some(v) = op.get("view") {
        w.view.set(match v {
            Json::Null => None,
            r => Some(rect_of(r, "view")?),
        });
    }
    for (key, id) in [("roads_funding", 0xC921_CEEAu32), ("transit_funding", 0xC921_CEEB)] {
        if let Some(v) = opt(op, key) {
            w.funding.borrow_mut().insert(id, fv(v, key)?);
        }
    }
    if let Some(v) = opt(op, "dev_pop") {
        for e in arr(v, "dev_pop")? {
            match arr(e, "dev_pop")? {
                [t, x, z, n] => {
                    w.dev_pop.borrow_mut().insert(
                        (u32_of(t, "type")?, i32_of(x, "x")?, i32_of(z, "z")?),
                        i64_of(n, "population")? as i16,
                    );
                }
                _ => return Err("each dev_pop entry is [type, x, z, population]".into()),
            }
        }
    }
    if let Some(v) = opt(op, "dept_base") {
        w.dept_base.set(u64_of(v, "dept_base")?);
    }
    if let Some(v) = opt(op, "budget_items") {
        w.budget_items.set(i64_of(v, "budget_items")?);
    }
    Ok(())
}

fn lot(w: &World, op: &Json) -> R<()> {
    let id = u64_of(field(op, "id")?, "id")?;
    let rect = rect_of(field(op, "rect")?, "rect")?;
    let holder = opt(op, "holder").map_or(Ok(id + 0x10000), |v| u64_of(v, "holder"))?;
    let building = match op.get("building") {
        None => Some(id + 0x20000),
        Some(Json::Null) => None,
        Some(v) => Some(u64_of(v, "building")?),
    };
    let flag = |k: &str| opt(op, k).map_or(Ok(true), |v| bool_of(v, k));
    let mut pops = Vec::new();
    if let Some(p) = opt(op, "pops") {
        for e in arr(p, "pops")? {
            match arr(e, "pops")? {
                [t, n] => pops.push((u32_of(t, "type")?, u32_of(n, "count")?)),
                _ => return Err("each pops entry is [developer type, count]".into()),
            }
        }
    }
    let l = Lot {
        rect,
        holder,
        building,
        zone: opt(op, "zone").map_or(Ok(0), |v| u32_of(v, "zone"))?,
        livable: flag("livable")?,
        road_access: flag("road_access")?,
        link: opt(op, "link").map(|v| u64_of(v, "link")).transpose()?,
        pops,
        strategy_byte: opt(op, "strategy_byte").map_or(Ok(0), |v| u32_of(v, "strategy_byte"))? as u8,
    };
    w.holders.borrow_mut().entry(holder).or_default();
    {
        let mut lots = w.lots.borrow_mut();
        lots.retain(|(i, _)| *i != id);
        lots.push((id, l));
    }
    if let Some(c) = opt(op, "capacity") {
        w.set_prop(holder, 0x8A0B_4100, u16_variant(&u32s(c, "capacity")?));
    }
    if let Some(t) = opt(op, "trips") {
        w.trips.borrow_mut().insert(Source::Lot(id), trips_of(t)?);
    }
    Ok(())
}

fn connection(w: &World, op: &Json) -> R<()> {
    let id = u64_of(field(op, "id")?, "id")?;
    let holder = opt(op, "holder").map_or(Ok(id + 0x10000), |v| u64_of(v, "holder"))?;
    let c = Conn {
        cell: pair(field(op, "cell")?, "cell")?,
        holder,
        dirty: opt(op, "dirty").map_or(Ok(true), |v| bool_of(v, "dirty"))?,
    };
    {
        let mut conns = w.conns.borrow_mut();
        conns.retain(|(i, _)| *i != id);
        conns.push((id, c));
    }
    if let Some(t) = opt(op, "targets") {
        for e in arr(t, "targets")? {
            match arr(e, "targets")? {
                [ty, vals] => {
                    let v = u32s(vals, "targets")?;
                    w.set_prop(
                        holder,
                        0xAA9E_A81D + u32_of(ty, "trip type")?,
                        Variant::array(mem::UINT32, v.into_iter().map(Elem::U32).collect()),
                    );
                }
                _ => return Err("each targets entry is [trip type, [9 counts]]".into()),
            }
        }
    }
    if let Some(t) = opt(op, "trips") {
        w.trips.borrow_mut().insert(Source::Connection(id), trips_of(t)?);
    }
    Ok(())
}

fn set_trips(w: &World, op: &Json) -> R<()> {
    let src = source_of(field(op, "source")?)?;
    w.trips.borrow_mut().insert(src, trips_of(field(op, "trips")?)?);
    Ok(())
}

fn set_capacity(w: &World, op: &Json) -> R<()> {
    let holder = u64_of(field(op, "holder")?, "holder")?;
    w.set_prop(holder, 0x8A0B_4100, u16_variant(&u32s(field(op, "values")?, "values")?));
    Ok(())
}

fn set_prop(w: &World, op: &Json) -> R<()> {
    let holder = u64_of(field(op, "holder")?, "holder")?;
    let id = u32_of(field(op, "id")?, "id")?;
    w.set_prop(holder, id, scalar::variant_of(field(op, "value")?)?);
    Ok(())
}

fn init(w: &Rc<World>, op: &Json) -> R<()> {
    let seed = opt(op, "session_seed").map_or(Ok(0), |v| u32_of(v, "session_seed"))?;
    let host: Rc<dyn TrafficSimHost> = w.clone();
    let s = TrafficSimulator::new(host, seed);
    let ok = s.init();
    *w.sim.borrow_mut() = Some(s);
    check_expect(op, jb(ok))
}

/// Appends network-info records: `info` = `[[network, type, inv_cap,
/// conn_count]]` (and an optional `flags` byte as a fifth value).
fn info(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let cells = cells_of(op)?;
    let recs: Vec<(u32, u8, f32, u8, u8)> = arr(field(op, "info")?, "info")?
        .iter()
        .map(|e| match arr(e, "info")? {
            [n, t, c, k] => Ok((
                u32_of(n, "n")?,
                u32_of(t, "type")? as u8,
                fv(c, "inv_cap")?,
                u32_of(k, "count")? as u8,
                0,
            )),
            [n, t, c, k, f] => Ok((
                u32_of(n, "n")?,
                u32_of(t, "type")? as u8,
                fv(c, "inv_cap")?,
                u32_of(k, "count")? as u8,
                u32_of(f, "flags")? as u8,
            )),
            _ => Err("each info entry is [network, type, inv_cap, conn_count, flags?]".to_string()),
        })
        .collect::<R<_>>()?;
    s.network().update_state(|n| -> R<()> {
        for &(x, z) in &cells {
            for &(net, ty, cap, count, flags) in &recs {
                let r = n
                    .append_info(net, x, z)
                    .ok_or_else(|| format!("cannot add network {net} info at ({x},{z})"))?;
                r.net_type = ty;
                r.inv_cap = cap;
                r.conn_count = count;
                r.flags = flags;
            }
        }
        Ok(())
    })
}

/// The §11 test 10 street cell: road info `{0.0625, Street (3), connection
/// count 3}`.
fn street(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let cells = cells_of(op)?;
    s.network().update_state(|n| -> R<()> {
        for &(x, z) in &cells {
            let r = n
                .append_info(1, x, z)
                .ok_or_else(|| format!("cannot add road info at ({x},{z})"))?;
            r.inv_cap = 0.0625;
            r.net_type = 3;
            r.conn_count = 3;
        }
        Ok(())
    })
}

fn seed_rng(v: u32) -> RzRandomSuper {
    let seeds = FixedSeeds {
        wall_clock: v,
        process_clock: v,
    };
    RzRandomSuper::new(v, &seeds)
}

/// Sets state fields.
fn state(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let st = field(op, "state")?
        .as_object()
        .ok_or("'state' must be an object")?
        .to_vec();
    for (k, v) in st {
        let v = &v;
        match k.as_str() {
            "subnetworks_stale" => {
                let b = bool_of(v, &k)?;
                s.network().update_state(|n| n.subnetworks_stale = b);
                continue;
            }
            "network_changed" => {
                let b = bool_of(v, &k)?;
                s.network().update_state(|n| n.network_changed = b);
                continue;
            }
            _ => {}
        }
        s.update_state(|st| -> Option<R<()>> {
            Some((|| -> R<()> {
                match k.as_str() {
                    "cycle_state" => st.cycle_state = u32_of(v, &k)? as u8,
                    "pass_total" => st.pass_total = i64_of(v, &k)? as i32,
                    "to_reroute" => st.to_reroute = i64_of(v, &k)? as i32,
                    "to_search" => st.to_search = i64_of(v, &k)? as i32,
                    "units_per_slice" => st.tuning.traffic_units_per_slice = u32_of(v, &k)?.max(1),
                    "pct" => {
                        for e in arr(v, &k)? {
                            match arr(e, "pct")? {
                                [wl, p] => {
                                    let p = u32s(p, "pct")?;
                                    let row = st
                                        .tuning
                                        .pct
                                        .get_mut(u32_of(wl, "wealth")? as usize)
                                        .ok_or("wealth out of range")?;
                                    for (d, s) in row.iter_mut().zip(p) {
                                        *d = s as u8;
                                    }
                                }
                                _ => return Err("each pct entry is [wealth, [3 percentages]]".into()),
                            }
                        }
                    }
                    "budget_total" => st.budget_total = i64_of(v, &k)?,
                    "strike_state" => st.strike_state = u32_of(v, &k)? as u8,
                    "strike_months" => st.strike_months = i64_of(v, &k)? as i32,
                    "transit_income" => st.transit_income = fv(v, &k)?,
                    "toll_income" => st.toll_income = fv(v, &k)?,
                    "rail_damage_accident_factor" => st.tuning.rail_damage_accident_factor = fv(v, &k)?,
                    "commute_max_time" => st.tuning.commute_max_time = fv(v, &k)?,
                    "strategy_bucket" => st.strategy_bucket = u32_of(v, &k)?,
                    "buckets" => {
                        st.buckets.clear();
                        for e in arr(v, &k)? {
                            match arr(e, "buckets")? {
                                [b, cells] => {
                                    let cells = arr(cells, "bucket cells")?
                                        .iter()
                                        .map(|c| pair(c, "bucket cell"))
                                        .collect::<R<Vec<_>>>()?;
                                    st.buckets.insert(u32_of(b, "bucket")?, cells);
                                }
                                _ => return Err("each buckets entry is [bucket, [[x, z]]]".into()),
                            }
                        }
                    }
                    "dest" => {
                        for e in arr(v, &k)? {
                            match arr(e, "dest")? {
                                [x, z, b] => {
                                    st.set_dest_byte(i32_of(x, "x")?, i32_of(z, "z")?, u32_of(b, "bits")? as u8)
                                }
                                _ => return Err("each dest entry is [x, z, bits]".into()),
                            }
                        }
                    }
                    "free" => {
                        for (i, f) in u32s(v, &k)?.into_iter().enumerate() {
                            st.set_free(i, f);
                        }
                    }
                    "accident_check" => {
                        st.accident_check = arr(v, &k)?
                            .iter()
                            .map(|e| match arr(e, "accident_check")? {
                                [x, z, p] => Ok(AccidentEntry {
                                    x: i32_of(x, "x")?,
                                    z: i32_of(z, "z")?,
                                    p: fv(p, "p")?,
                                }),
                                _ => Err("each accident_check entry is [x, z, p]".to_string()),
                            })
                            .collect::<R<_>>()?;
                    }
                    "rng_seed" => st.rng = seed_rng(u32_of(v, &k)?),
                    "income_acc" => st.income_acc.set(dv(v, &k)?),
                    other => return Err(format!("unknown state field '{other}'")),
                }
                Ok(())
            })())
        })
        .ok_or("the simulator state is out")??;
    }
    Ok(())
}

fn block<'a>(d: &'a mut fc4_core::traffic_simulator::DamageState, kind: &str) -> R<&'a mut DamageBlock> {
    match kind {
        "road" => Ok(&mut d.road),
        "rail" => Ok(&mut d.rail),
        other => Err(format!("unknown damage kind '{other}'")),
    }
}

/// Damage sets: `kind` (`road` / `rail`), `intact?` and `damaged?` cells
/// (added through the cell bookkeeping, damaged ones moved), `rate?`.
fn damage(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let kind = str_of(field(op, "kind")?, "kind")?.to_string();
    let cells = |k: &str| -> R<Vec<(i32, i32)>> {
        opt(op, k).map_or(Ok(Vec::new()), |v| arr(v, k)?.iter().map(|c| pair(c, k)).collect())
    };
    let (intact, damaged) = (cells("intact")?, cells("damaged")?);
    let rate = opt(op, "rate").map(|v| fv(v, "rate")).transpose()?;
    s.update_damage(|d| -> R<()> {
        let b = block(d, &kind)?;
        for (x, z) in intact {
            b.insert_cell(cell_key16(x, z));
        }
        for (x, z) in damaged {
            let k = cell_key16(x, z);
            b.insert_cell(k);
            b.intact.remove(u32::from(k));
            b.damaged.insert(u32::from(k));
            b.damaged_count = b.damaged.len() as u32;
        }
        if let Some(r) = rate {
            b.rate = r;
        }
        Ok(())
    })
}

type U8Grid = Rc<dyn TractGrid<u8>>;
type F32Grid = Rc<dyn TractGrid<f32>>;

enum AnyGrid {
    U8(U8Grid),
    F32(F32Grid),
}

fn grid_of(s: &TrafficSimulator, op: &Json) -> R<AnyGrid> {
    let name = str_of(field(op, "grid")?, "grid")?;
    let g = match name {
        "air" => s.air_polluting_traffic_map().map(AnyGrid::U8),
        "congestion" => s.congestion_map().map(AnyGrid::U8),
        "trip_length" => s.trip_length_map().map(AnyGrid::U8),
        "commercial" => s.commercial_traffic_map().map(AnyGrid::U8),
        "commercial_work" => s.commercial_work_grid().map(AnyGrid::U8),
        "peak_volume" => s.peak_volume_grid().map(AnyGrid::F32),
        "peak_ratio" => s.peak_ratio_grid().map(AnyGrid::F32),
        "volume" => {
            let n = opt(op, "n").map_or(Ok(1), |v| u32_of(v, "n"))?;
            s.volume_grid(n as usize).map(AnyGrid::F32)
        }
        other => return Err(format!("unknown grid '{other}'")),
    };
    g.ok_or_else(|| format!("the simulator has no '{name}' grid"))
}

/// Writes grid values: `grid`, `n?` (volume), `values` (`[[x, z, v]]`, tract
/// coordinates; floats as numbers or bits).
fn set_grid(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let g = grid_of(&s, op)?;
    for e in arr(field(op, "values")?, "values")? {
        match arr(e, "values")? {
            [x, z, v] => {
                let (x, z) = (i32_of(x, "x")?, i32_of(z, "z")?);
                match &g {
                    AnyGrid::U8(g) => g.set(x, z, u32_of(v, "value")? as u8),
                    AnyGrid::F32(g) => g.set(x, z, fv(v, "value")?),
                }
            }
            _ => return Err("each value is [x, z, v]".into()),
        }
    }
    Ok(())
}

// ---- calls ----------------------------------------------------------------------------------------------

fn call(w: &Rc<World>, op: &Json) -> R<()> {
    let s = w.sim()?;
    let method = str_of(field(op, "method")?, "method")?;
    let got = match method {
        "init" => jb(s.init()),
        "shutdown" => jb(s.shutdown()),
        "read_tunable_values" => {
            s.read_tunable_values();
            Json::Null
        }
        "work_on_sim_cycle" => jb(s.work_on_sim_cycle()),
        "do_daily_simulation" => jb(s.do_daily_simulation()),
        "do_monthly_simulation" => {
            s.do_monthly_simulation();
            Json::Null
        }
        "do_post_city_init" => {
            s.do_post_city_init();
            Json::Null
        }
        "do_message" => {
            let m = Message {
                msg_type: uarg(op, 0)?,
                data1: uarg(op, 1)?,
                data2: uarg(op, 2)?,
                data3: uarg(op, 3)?,
                object: u64_of(arg(op, 4)?, "object")?,
            };
            jb(s.do_message(&m))
        }
        "enter_cycle_state" => {
            s.enter_cycle_state(uarg(op, 0)? as u8);
            Json::Null
        }
        "apply_route" => {
            let route = hex_of(arg(op, 0)?, "route")?;
            s.apply_route(
                &route,
                uarg(op, 1)?,
                rect_of(arg(op, 2)?, "source rect")?,
                pair(arg(op, 3)?, "destination")?,
                uarg(op, 4)? as u8,
                uarg(op, 5)? as u8,
            );
            Json::Null
        }
        "update_traffic_volume" => {
            s.update_traffic_volume(source_of(arg(op, 0)?)?);
            Json::Null
        }
        "confirm_source" => {
            s.confirm_source(source_of(arg(op, 0)?)?);
            Json::Null
        }
        "complete_trip_source_calculations" => {
            s.complete_trip_source_calculations(source_of(arg(op, 0)?)?);
            Json::Null
        }
        "capacity_still_available" => jb(s.capacity_still_available(&trip_of(arg(op, 0)?)?)),
        "apply_traffic_to_destination" => {
            s.apply_traffic_to_destination(
                uarg(op, 0)?,
                iarg(op, 1)?,
                iarg(op, 2)?,
                uarg(op, 3)? as u8,
                uarg(op, 4)? as u8,
            );
            Json::Null
        }
        "setup_trip" => {
            let t = trip_of(arg(op, 0)?)?;
            let src = source_of(arg(op, 1)?)?;
            let mut pf = s.create_path_finder();
            let mut pf2 = s.create_path_finder();
            s.setup_trip(&mut pf, &mut pf2, &t, src);
            obj(vec![
                ("density", ji(pf.traffic_density())),
                ("filter", ji(pf.trip_destination_filter())),
                ("destination_flags", ji(pf.destination_flags())),
                ("flags", ji(pf.flags())),
                ("allowed", ji(pf.allowed_travel_types())),
                ("starting", ji(pf.starting_travel_types())),
                ("start_rect", jrect(pf.start_rect())),
                ("end_rect", jrect(pf.end_rect())),
                (
                    "start_costs",
                    pf.starting_cost_array()
                        .map_or(Json::Null, |c| Json::Array(c.iter().map(|&v| fb(v)).collect())),
                ),
                ("max_work_time", ji(pf.max_work_time())),
            ])
        }
        "background_traffic" => fb(s.background_traffic(iarg(op, 0)?, iarg(op, 1)?)),
        "travel_time_ratio" => fb(s.travel_time_ratio(iarg(op, 0)?, iarg(op, 1)?, uarg(op, 2)?)),
        "check_rail_accident" => jb(s.check_rail_accident(iarg(op, 0)?, iarg(op, 1)?)),
        "is_road_damaged" => jb(s.is_road_damaged(iarg(op, 0)?, iarg(op, 1)?)),
        "apply_pothole_changes" => {
            s.apply_pothole_changes(uarg(op, 0)?, iarg(op, 1)?);
            Json::Null
        }
        "travel_strategy_percentages" => Json::Array(s.travel_strategy_percentages(uarg(op, 0)?).map(fb).to_vec()),
        "trip_scale" => fb(s.trip_scale()),
        "trip_scale_for_display" => fb(s.trip_scale_for_display()),
        "freight_scaling_factor" => fb(s.freight_scaling_factor()),
        "last_monthly_income" => fb(s.last_monthly_income()),
        "query_interface" => jb(s.query_interface(uarg(op, 0)?)),
        "simulator_type" => ji(s.simulator_type()),
        "desired_lot_insertion_point" => jb(s.desired_lot_insertion_point()),
        "max_trip_capacity" => ji(s.max_trip_capacity(u64_of(arg(op, 0)?, "holder")?, uarg(op, 1)?)),
        "set_max_trip_capacity" => {
            s.set_max_trip_capacity(u64_of(arg(op, 0)?, "holder")?, uarg(op, 1)?, uarg(op, 2)?);
            Json::Null
        }
        "traffic_arrived" => ji(s.traffic_arrived(u64_of(arg(op, 0)?, "holder")?, uarg(op, 1)?)),
        "traffic_arrived_this_cycle" => ji(s.traffic_arrived_this_cycle(u64_of(arg(op, 0)?, "holder")?, uarg(op, 1)?)),
        "destination_byte" => ji(s.destination_byte(iarg(op, 0)?, iarg(op, 1)?)),
        "rng_uniform" => {
            let (n, count) = (uarg(op, 0)?, uarg(op, 1)?);
            let v: Vec<u32> = s.update_state(|st| (0..count).map(|_| st.rng.uniform_u32(n)).collect());
            jlist(&v)
        }
        "rng_next" => {
            let count = uarg(op, 0)?;
            let v: Vec<u32> = s.update_state(|st| (0..count).map(|_| st.rng.next_u32()).collect());
            jlist(&v)
        }
        "cycle_states" => {
            // Finished phases until state 0 comes back (at most `max`),
            // `subnetworks_stale` set before each call when given.
            let stale = match op.get("args").and_then(|a| a.as_array()).and_then(|a| a.first()) {
                None | Some(Json::Null) => None,
                Some(v) => Some(bool_of(v, "stale")?),
            };
            let max = opt(op, "max").map_or(Ok(64), |v| u32_of(v, "max"))?;
            let mut seen = vec![ji(s.cycle_state())];
            for _ in 0..max {
                if let Some(b) = stale {
                    s.network().update_state(|n| n.subnetworks_stale = b);
                }
                if !s.work_on_sim_cycle() {
                    return Err(format!(
                        "a phase did not finish in one call (state {})",
                        s.cycle_state()
                    ));
                }
                seen.push(ji(s.cycle_state()));
                if s.cycle_state() == 0 {
                    break;
                }
            }
            Json::Array(seen)
        }
        "cycle_state" => ji(s.cycle_state()),
        "free_destinations" => jlist(&s.free_destinations()),
        "rng_double" => {
            let count = uarg(op, 0)?;
            let v: Vec<f64> = s.update_state(|st| (0..count).map(|_| st.rng.uniform_f64()).collect());
            Json::Array(v.into_iter().map(|u| fb(u as f32)).collect())
        }
        other => return Err(format!("unknown method '{other}'")),
    };
    check_partial(op, got)
}

/// Checks `expect` with objects compared on the keys it gives.
fn check_partial(op: &Json, got: Json) -> R<()> {
    match op.get("expect") {
        Some(w) if !matches(&got, w, true) => Err(format!("returned {}, expected {}", show(&got), show(w))),
        _ => Ok(()),
    }
}

// ---- pure functions ----------------------------------------------------------------------------------

/// The shipped tuning (§4.14), for the pure map functions.
fn shipped() -> SimTuning {
    let mut t = SimTuning::default();
    t.read(Some(&SimTuning::spec_exemplar()));
    t
}

/// `read_tuning [base, props, fields]`: `ReadSimulatorTunableValues` and the
/// strategy re-read over `base` (`"shipped"` or `"code"`) with the property
/// map `props`; returns the named fields (§4.14, §4.1.2, TS28, TS29).
fn read_tuning(op: &Json) -> R<Json> {
    let mut t = match str_of(arg(op, 0)?, "base")? {
        "shipped" => shipped(),
        "code" => SimTuning::default(),
        b => return Err(format!("unknown tuning base '{b}'")),
    };
    let h = scalar::holder_of(arg(op, 1)?)?;
    t.read(Some(&h));
    t.read_strategies(&h);
    let floats = |v: &[f32]| Json::Array(v.iter().map(|&x| fb(x)).collect());
    let mut out = Vec::new();
    for f in arr(arg(op, 2)?, "fields")? {
        let k = str_of(f, "field")?;
        let v = match k {
            "max_speed" => Json::Array(t.max_speed.iter().map(|r| floats(r)).collect()),
            "tile_costs" => floats(&t.tile_costs),
            "income_per_tile" => floats(&t.income_per_tile),
            "roads_funding" => fb(t.max_roads_funding),
            "transit_funding" => fb(t.max_transit_funding),
            "generates_traffic" => Json::Array(t.generates_traffic.iter().map(|&b| jb(b)).collect()),
            "start_neutral" => floats(&t.start_neutral),
            "start_car" => floats(&t.start_car),
            "background" => floats(&t.background),
            "pct" => Json::Array(t.pct.iter().map(|r| jlist(r)).collect()),
            _ => return Err(format!("unknown tuning field '{k}'")),
        };
        out.push((k.to_string(), v));
    }
    Ok(Json::Object(out))
}

fn eval(op: &Json) -> R<()> {
    let f = str_of(field(op, "fn")?, "fn")?;
    let got = match f {
        "congestion_byte" => {
            let t = shipped();
            ji(congestion_byte(
                farg(op, 0)?,
                farg(op, 1)?,
                &t.congestion_curve,
                t.congestion_scale,
            ))
        }
        "congestion_curve" => fb(shipped().congestion_curve.eval(farg(op, 0)?)),
        "shipped_congestion" => {
            let t = shipped();
            obj(vec![
                (
                    "points",
                    Json::Array(
                        t.congestion_curve
                            .points()
                            .into_iter()
                            .map(|(x, y)| Json::Array(vec![fb(x), fb(y)]))
                            .collect(),
                    ),
                ),
                ("scale", fb(t.congestion_scale)),
            ])
        }
        "invert_curve" => {
            let mut pts = Vec::new();
            for p in arr(arg(op, 0)?, "points")? {
                match arr(p, "point")? {
                    [x, y] => pts.push((fv(x, "x")?, fv(y, "y")?)),
                    _ => return Err("each point is [x, y]".into()),
                }
            }
            let (c, scale) = invert_congestion_curve(&ResponseCurve::from_points(&pts));
            obj(vec![
                (
                    "points",
                    Json::Array(
                        c.points()
                            .into_iter()
                            .map(|(x, y)| Json::Array(vec![fb(x), fb(y)]))
                            .collect(),
                    ),
                ),
                ("scale", fb(scale)),
            ])
        }
        "air_polluting_byte" => ji(air_polluting_byte(farg(op, 0)?)),
        "trip_length_byte" => ji(trip_length_byte(farg(op, 0)?, farg(op, 1)?)),
        "strategy_percentages" => {
            let p = u32s(arg(op, 0)?, "pct")?;
            let mut t = SimTuning::default();
            for (d, s) in t.pct[1].iter_mut().zip(p) {
                *d = s as u8;
            }
            Json::Array(t.strategy_percentages(1).map(fb).to_vec())
        }
        "read_tuning" => read_tuning(op)?,
        "residential_split" => {
            let p: Vec<f32> = arr(arg(op, 1)?, "pct")?
                .iter()
                .map(|v| fv(v, "pct"))
                .collect::<R<_>>()?;
            let p: [f32; 3] = p.try_into().map_err(|_| "pct must have 3 values".to_string())?;
            jlist(&residential_split(uarg(op, 0)?, p, uarg(op, 2)? as u8))
        }
        "residential_split_pct" => {
            let p = u32s(arg(op, 1)?, "pct")?;
            let mut t = SimTuning::default();
            for (d, s) in t.pct[1].iter_mut().zip(p) {
                *d = s as u8;
            }
            jlist(&residential_split(
                uarg(op, 0)?,
                t.strategy_percentages(1),
                uarg(op, 2)? as u8,
            ))
        }
        "residential_commuters" => ji(residential_commuters(uarg(op, 0)?, farg(op, 1)?)),
        "industrial_trips" => ji(industrial_trips(uarg(op, 0)?, farg(op, 1)?)),
        "background_value" => {
            let bg = shipped().background;
            fb(background_value(iarg(op, 0)?, iarg(op, 1)?, iarg(op, 2)?, &bg))
        }
        "damage_target" => {
            let t = shipped();
            let (d, n) = damage_target(&t.damage_curve, farg(op, 0)?, farg(op, 1)?, uarg(op, 2)?);
            Json::Array(vec![fb(d), ji(n)])
        }
        "reconcile_lot" => {
            let mut t = trips_of(arg(op, 0)?)?;
            let w = u32s(arg(op, 1)?, "targets")?;
            let w: [u32; 3] = w.try_into().map_err(|_| "targets must have 3 values".to_string())?;
            reconcile_lot(&mut t, w.map(|v| v as u16), uarg(op, 2)? as u8);
            Json::Array(t.iter().map(jtrip).collect())
        }
        "reconcile_connection" => {
            let mut t = trips_of(arg(op, 0)?)?;
            let mut targets = [[0u32; 9]; 3];
            for (row, j) in targets.iter_mut().zip(arr(arg(op, 1)?, "targets")?) {
                for (d, s) in row.iter_mut().zip(u32s(j, "targets")?) {
                    *d = s;
                }
            }
            reconcile_connection(&mut t, &targets);
            Json::Array(t.iter().map(jtrip).collect())
        }
        "decode_route" => Json::Array(
            decode_route(&hex_of(arg(op, 0)?, "route")?)
                .iter()
                .map(|c| Json::Array(vec![ji(c.x), ji(c.z), ji(c.t), jb(c.jump)]))
                .collect(),
        ),
        "legacy_density" => ji(legacy_density(farg(op, 0)?)),
        "write_trip" => {
            let mut out = ByteWriter::new();
            let ok = write_trip(&mut out, &trip_of(arg(op, 0)?)?);
            if !ok {
                return Err("write_trip failed".into());
            }
            Json::Str(hex_encode(out.bytes()))
        }
        "read_trip" => {
            let bytes = hex_of(arg(op, 0)?, "record")?;
            let mut t = match opt(op, "args").and_then(|a| a.as_array()).and_then(|a| a.get(1)) {
                Some(j) => trip_of(j)?,
                None => TripData::default(),
            };
            if read_trip(&mut ByteReader::new(&bytes), &mut t) {
                jtrip(&t)
            } else {
                Json::Null
            }
        }
        other => return Err(format!("unknown fn '{other}'")),
    };
    check_partial(op, got)
}

// ---- persistence ----------------------------------------------------------------------------------------

fn save(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let slot = str_of(field(op, "slot")?, "slot")?.to_string();
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut out = ByteWriter::with_registry(reg.clone());
    if !s.write(&mut out) {
        return Err("Write failed".into());
    }
    let bytes = out.into_bytes();
    if let Some(p) = opt(op, "prefix") {
        let want = hex_of(p, "prefix")?;
        if !bytes.starts_with(&want) {
            let n = want.len().min(bytes.len());
            return Err(format!(
                "record starts {}, expected {}",
                hex_encode(&bytes[..n]),
                hex_encode(&want)
            ));
        }
    }
    if let Some(n) = opt(op, "length") {
        let n = u32_of(n, "length")? as usize;
        if bytes.len() != n {
            return Err(format!("record length {}, expected {n}", bytes.len()));
        }
    }
    let entries = reg.borrow().entries.clone();
    w.saved.borrow_mut().insert(slot, (bytes, entries));
    Ok(())
}

/// Reads a saved record into the simulator: `slot`, `version?` (the u16
/// version patched), `truncate?` (bytes dropped from the end), `expect`.
fn load(w: &Rc<World>, op: &Json) -> R<()> {
    let s = w.sim()?;
    let slot = str_of(field(op, "slot")?, "slot")?;
    let (mut bytes, entries) = w
        .saved
        .borrow()
        .get(slot)
        .cloned()
        .ok_or_else(|| format!("no saved slot '{slot}'"))?;
    if let Some(v) = opt(op, "version") {
        let v = u32_of(v, "version")? as u16;
        bytes[..2].copy_from_slice(&v.to_le_bytes());
    }
    if let Some(n) = opt(op, "truncate") {
        let n = u32_of(n, "truncate")? as usize;
        bytes.truncate(bytes.len().saturating_sub(n));
    }
    let mut res = Resolver(entries, w.clone());
    let ok = s.read(&mut ByteReader::with_resolver(&bytes, &mut res));
    check_expect(op, jb(ok))
}

fn expect_same_save(w: &World, op: &Json) -> R<()> {
    let a = str_of(field(op, "a")?, "a")?;
    let b = str_of(field(op, "b")?, "b")?;
    let saved = w.saved.borrow();
    let x = &saved.get(a).ok_or_else(|| format!("no saved slot '{a}'"))?.0;
    let y = &saved.get(b).ok_or_else(|| format!("no saved slot '{b}'"))?.0;
    if x != y {
        let at = x
            .iter()
            .zip(y.iter())
            .position(|(p, q)| p != q)
            .unwrap_or(x.len().min(y.len()));
        return Err(format!(
            "the records differ at byte {at} (lengths {} and {})",
            x.len(),
            y.len()
        ));
    }
    Ok(())
}

// ---- checks -----------------------------------------------------------------------------------------------

fn expect_state(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let net = s.network().snapshot();
    let got = s
        .read_state(|st| {
            let rows = |set: &fc4_core::traffic_simulator::StatsSet| {
                Json::Array(
                    set.rows
                        .iter()
                        .map(|r| {
                            obj(vec![
                                ("sum", fb(r.sum)),
                                ("count", ji(r.count)),
                                ("max", fb(r.max)),
                                ("max_cell", jcell((r.max_x, r.max_z))),
                            ])
                        })
                        .collect(),
                )
            };
            Some(vec![
                ("initialised", jb(st.initialised)),
                ("cycle_state", ji(st.cycle_state)),
                ("total", ji(st.total)),
                ("days", ji(st.days)),
                ("day_index", ji(st.day_index)),
                ("processed", ji(st.processed)),
                ("pass_total", ji(st.pass_total)),
                ("to_reroute", ji(st.to_reroute)),
                ("to_search", ji(st.to_search)),
                ("source_pos", ji(st.source_pos as u32)),
                ("source_count", ji(st.sources.len() as u32)),
                ("pass_counters", jlist(&st.pass_counters)),
                ("strategy_bucket", ji(st.strategy_bucket)),
                ("strategy_bucket_counter", ji(st.strategy_bucket_counter)),
                (
                    "buckets",
                    Json::Array(
                        st.buckets
                            .iter()
                            .map(|(b, cells)| {
                                Json::Array(vec![ji(*b), Json::Array(cells.iter().map(|&c| jcell(c)).collect())])
                            })
                            .collect(),
                    ),
                ),
                ("budget_total", ji(st.budget_total)),
                ("strike_state", ji(st.strike_state)),
                ("strike_months", ji(st.strike_months)),
                ("strike_flag", jb(st.strike_flag)),
                ("strike_probability", fb(st.strike_probability)),
                ("attempted", jlist(&st.attempted)),
                ("routed", jlist(&st.routed)),
                ("job_capacity", jlist(&st.job_capacity)),
                ("car_len_sum", fb(st.car_len_sum)),
                ("car_len_count", ji(st.car_len_count)),
                ("freight_len_sum", fb(st.freight_len_sum)),
                ("freight_count", ji(st.freight_count)),
                ("freight_byte", ji(st.freight_byte)),
                ("avg_car_commute", fb(st.avg_car_commute)),
                ("transit_income", fb(st.transit_income)),
                ("toll_income", fb(st.toll_income)),
                ("last_monthly_income", fb(st.last_monthly_income)),
                ("income_acc", db(st.income_acc.get())),
                ("fee_acc", db(st.fee_acc)),
                ("free", jlist(&std::array::from_fn::<u32, 4, _>(|k| st.free(k)))),
                ("type_count_building", jlist(&st.type_count_building)),
                ("type_count_published", jlist(&st.type_count_published)),
                ("ferry_total", ji(st.ferry_total)),
                ("cells_per_type", jlist(&st.cells_per_type)),
                ("stats_over", jlist(&st.stats_last.over)),
                ("stats_rows", rows(&st.stats_last)),
                ("next_accident_check", ji(st.next_accident_check as i64)),
                (
                    "accident_check",
                    Json::Array(
                        st.accident_check
                            .iter()
                            .map(|e| Json::Array(vec![ji(e.x), ji(e.z), fb(e.p)]))
                            .collect(),
                    ),
                ),
                ("roads", st.roads.map_or(Json::Null, |d| ji(d as i64))),
                ("mass_transit", st.mass_transit.map_or(Json::Null, |d| ji(d as i64))),
                ("commute_max_time", fb(st.tuning.commute_max_time)),
                ("congestion_scale", fb(st.tuning.congestion_scale)),
                (
                    "congestion_points",
                    Json::Array(
                        st.tuning
                            .congestion_curve
                            .points()
                            .into_iter()
                            .map(|(x, y)| Json::Array(vec![fb(x), fb(y)]))
                            .collect(),
                    ),
                ),
                ("units_per_slice", ji(st.tuning.traffic_units_per_slice)),
                ("last_day", ji(st.last_day)),
                ("cycle_start_date", ji(st.cycle_start_date)),
                ("subnetworks_stale", jb(net.subnetworks_stale)),
                ("network_changed", jb(net.network_changed)),
                ("transit_multiplier", fb(net.transit_multiplier)),
            ])
        })
        .ok_or("the simulator state is out")?;
    let got: Vec<(String, Json)> = got.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
    scalar::check_state(&got, field(op, "state")?)
}

/// `grid`, `n?`, `values` (`[[x, z, v]]`: u8 values, f32 bits).
fn expect_grid(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let g = grid_of(&s, op)?;
    let mut errors = Vec::new();
    for e in arr(field(op, "values")?, "values")? {
        match arr(e, "values")? {
            [x, z, v] => {
                let (x, z) = (i32_of(x, "x")?, i32_of(z, "z")?);
                let got = match &g {
                    AnyGrid::U8(g) => ji(g.get(x, z)),
                    AnyGrid::F32(g) => fb(g.get(x, z)),
                };
                if !matches(&got, v, false) {
                    errors.push(format!("({x},{z}): got {}, expected {}", show(&got), show(v)));
                }
            }
            _ => return Err("each value is [x, z, v]".into()),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// `cell`, `t?` (1), `evening?` (false), `counts` (`[[entry, exit, value]]`:
/// the listed counters of `current`, every other one 0), `last?`, `usage?`.
fn expect_edge(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let (x, z) = pair(field(op, "cell")?, "cell")?;
    let t = opt(op, "t").map_or(Ok(1), |v| u32_of(v, "t"))?;
    let evening = opt(op, "evening").map_or(Ok(false), |v| bool_of(v, "evening"))?;
    let d = s.traffic_edge_density(x as u8, z as u8, t, evening);
    let want = |key: &str| -> R<Option<[u16; 16]>> {
        let Some(v) = opt(op, key) else { return Ok(None) };
        let mut c = [0u16; 16];
        for e in arr(v, key)? {
            match arr(e, key)? {
                [en, ex, n] => {
                    c[4 * u32_of(en, "entry")? as usize + u32_of(ex, "exit")? as usize] = u32_of(n, "value")? as u16
                }
                _ => return Err(format!("each '{key}' entry is [entry, exit, value]")),
            }
        }
        Ok(Some(c))
    };
    if let Some(c) = want("counts")? {
        if c != d.current {
            return Err(format!("current {:?}, expected {:?}", d.current, c));
        }
    }
    if let Some(c) = want("last")? {
        if c != d.last {
            return Err(format!("last {:?}, expected {:?}", d.last, c));
        }
    }
    if let Some(u) = opt(op, "usage") {
        let u = u32_of(u, "usage")? as u16;
        if u != d.usage {
            return Err(format!("usage {}, expected {u}", d.usage));
        }
    }
    Ok(())
}

fn expect_world(w: &World, op: &Json) -> R<()> {
    let posted = Json::Array(
        w.posted
            .borrow()
            .iter()
            .map(|m| Json::Array(vec![ji(m.msg_type), ji(m.data1), ji(m.data2), ji(m.data3)]))
            .collect(),
    );
    let got = vec![
        ("posted".to_string(), posted),
        (
            "posted_types".to_string(),
            Json::Array(w.posted.borrow().iter().map(|m| ji(m.msg_type)).collect()),
        ),
        (
            "history".to_string(),
            Json::Array(
                w.history
                    .borrow()
                    .iter()
                    .map(|&(s, v)| Json::Array(vec![ji(s), ji(v)]))
                    .collect(),
            ),
        ),
        (
            "incomes".to_string(),
            Json::Array(
                w.incomes
                    .borrow()
                    .iter()
                    .map(|&(d, i, v)| Json::Array(vec![ji(d as i64), ji(i), ji(v)]))
                    .collect(),
            ),
        ),
        (
            "accidents".to_string(),
            Json::Array(
                w.accidents
                    .borrow()
                    .iter()
                    .map(|&(x, z, d)| Json::Array(vec![ji(x), ji(z), ji(d)]))
                    .collect(),
            ),
        ),
        ("agents".to_string(), jlist(&w.agents.borrow())),
        ("subscriptions".to_string(), jlist(&w.subs.borrow())),
        (
            "departments".to_string(),
            Json::Array(
                w.departments
                    .borrow()
                    .iter()
                    .map(|&(id, m)| Json::Array(vec![ji(id), fb(m)]))
                    .collect(),
            ),
        ),
        (
            "released_departments".to_string(),
            Json::Array(w.released_departments.borrow().iter().map(|&d| ji(d as i64)).collect()),
        ),
        (
            "refs".to_string(),
            Json::Array(
                w.refs
                    .borrow()
                    .iter()
                    .filter(|(_, &n)| n != 0)
                    .map(|(&t, &n)| Json::Array(vec![ji(t as i64), ji(n)]))
                    .collect(),
            ),
        ),
    ];
    if let Some(want) = opt(op, "state") {
        scalar::check_state(&got, want)?;
    }
    // Entries that must be present, in any position: `[type]` or `[type,
    // data1, data2, data3]`; and history `[series, value]` pairs.
    if let Some(inc) = opt(op, "posted_includes") {
        for e in arr(inc, "posted_includes")? {
            let e = u32s(e, "posted_includes")?;
            let found = w.posted.borrow().iter().any(|m| match e.as_slice() {
                [t] => m.msg_type == *t,
                [t, a, b, c] => (m.msg_type, m.data1, m.data2, m.data3) == (*t, *a, *b, *c),
                _ => false,
            });
            if !found {
                return Err(format!("no posted message {e:?}"));
            }
        }
    }
    if let Some(inc) = opt(op, "history_includes") {
        for e in arr(inc, "history_includes")? {
            let e = u32s(e, "history_includes")?;
            let found = matches!(e.as_slice(), [s, v] if w.history.borrow().contains(&(*s, *v)));
            if !found {
                return Err(format!("no history entry {e:?}"));
            }
        }
    }
    Ok(())
}

/// `holder`, `id`, `expect` (null = absent; else the element values as a
/// list, a scalar as a one-element list).
fn expect_prop(w: &World, op: &Json) -> R<()> {
    let holder = u64_of(field(op, "holder")?, "holder")?;
    let id = u32_of(field(op, "id")?, "id")?;
    let got = match w.prop(holder, id) {
        None => Json::Null,
        Some(v) => Json::Array(
            v.items()
                .iter()
                .map(|e| match e {
                    Elem::F32(f) => fb(*f),
                    other => ji(other.as_i64()),
                })
                .collect(),
        ),
    };
    let want = op.get("expect").ok_or("missing field 'expect'")?;
    if !matches(&got, want, false) {
        return Err(format!(
            "property 0x{id:08X} of {holder}: got {}, expected {}",
            show(&got),
            show(want)
        ));
    }
    Ok(())
}

/// `source`, `trips` (the list, each trip partial).
fn expect_trips(w: &World, op: &Json) -> R<()> {
    let src = source_of(field(op, "source")?)?;
    let got = TrafficSimHost::trip_list(w, src);
    let want = arr(field(op, "trips")?, "trips")?;
    if got.len() != want.len() {
        return Err(format!(
            "{} trips, expected {}: {}",
            got.len(),
            want.len(),
            show(&Json::Array(got.iter().map(jtrip).collect()))
        ));
    }
    for (i, (g, wt)) in got.iter().zip(want).enumerate() {
        check_fields(&jtrip(g), wt, "trip").map_err(|e| format!("trip {i}: {e}"))?;
    }
    Ok(())
}

/// `kind`, `state` (partial: `cell_count`, `damaged_count`, `rate`,
/// `intact` (count), `damaged` (count), `damaged_cells` (`[[x, z]]` sorted)).
fn expect_damage(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let kind = str_of(field(op, "kind")?, "kind")?;
    let d = s.damage();
    let b = match kind {
        "road" => &d.road,
        "rail" => &d.rail,
        other => return Err(format!("unknown damage kind '{other}'")),
    };
    let mut cells: Vec<(i32, i32)> = b
        .damaged
        .iter_order()
        .into_iter()
        .map(|k| ((k >> 8) as i32 & 0xFF, (k & 0xFF) as i32))
        .collect();
    cells.sort_unstable();
    let got = vec![
        ("cell_count".to_string(), ji(b.cell_count)),
        ("damaged_count".to_string(), ji(b.damaged_count)),
        ("rate".to_string(), fb(b.rate)),
        ("intact".to_string(), ji(b.intact.len() as u32)),
        ("damaged".to_string(), ji(b.damaged.len() as u32)),
        (
            "damaged_cells".to_string(),
            Json::Array(cells.into_iter().map(jcell).collect()),
        ),
    ];
    scalar::check_state(&got, field(op, "state")?)
}

/// `cell`, `n?` (1), `record` (partial `{extra_cost, inv_cap, type,
/// conn_count}`).
fn expect_network(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let (x, z) = pair(field(op, "cell")?, "cell")?;
    let n = opt(op, "n").map_or(Ok(1), |v| u32_of(v, "n"))?;
    let got = match s.network().network_info(n, x, z) {
        None => Json::Null,
        Some(r) => obj(vec![
            ("extra_cost", fb(r.extra_cost)),
            ("inv_cap", fb(r.inv_cap)),
            ("type", ji(r.net_type)),
            ("conn_count", ji(r.conn_count)),
        ]),
    };
    let want = field(op, "record")?;
    if got == Json::Null || *want == Json::Null {
        return if matches(&got, want, false) {
            Ok(())
        } else {
            Err(format!("record {}, expected {}", show(&got), show(want)))
        };
    }
    check_fields(&got, want, "record")
}
