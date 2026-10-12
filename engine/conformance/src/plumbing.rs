// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/plumbing.md`: a scripted city (cells,
//! terrain water, water pollution values, funding and the ordinance factor;
//! building and pipe occupants with properties, lots, network flags and
//! connections; neighbour deals and connections; the budget with
//! departments, line items and budget items) and ops that drive the plumbing
//! simulator. See README.md ("Plumbing ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::rc::{Rc, Weak};

use fc4_core::grid::{create_for_clsid, CityDims, GridProvider, ScanState, SimGrid, TractGrid};
use fc4_core::plumbing::{
    answers_iid, deal as dealk, eax_of, msg, nearest_water_ring, network, occupant_type, pipe_cost, trunc_i64,
    BudgetItem, CellRect, GridRc, Message, PlumbingHost, PlumbingSim, PlumbingState, Tuning, CLSID_PLUMBING_SIMULATOR,
    IID_DEPARTMENT_BUDGET, IID_NETWORK_OCCUPANT, IID_OCCUPANT, SLOT_RETURNS,
};
use fc4_core::property::PropertyHolder;
use fc4_core::rng::{FixedSeeds, RzRandom};
use fc4_core::store::{load_store, save_store, LoadRegistry, Package};
use fc4_core::stream::{ByteReader, ByteWriter, InStream, OutStream, RefValue, SaveObject, SaveRegistry};
use fc4_core::variant::{Elem, Variant};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, f32_of, i64_of, jf, ji, opt, str_of, u32_of, EventLog, R,
};

/// Test-only class IDs of the host objects a save refers to.
const OCCUPANT_CLASS: u32 = 0x0CC0_0011;
const DEPARTMENT_CLASS: u32 = 0x0CC0_0012;

#[derive(Default)]
struct Occ {
    building: bool,
    network: bool,
    props: PropertyHolder,
    /// The anchor cell (position = its centre).
    cell: (i32, i32),
    /// Bounding cells (inclusive); the bounding box covers them exactly.
    rect: CellRect,
    building_type: u32,
    net_flags: u32,
    connects: Vec<u32>,
    budget: Option<BudgetItem>,
    local_funding: Option<f32>,
}

#[derive(Clone, Copy, Default)]
struct Lot {
    rect: CellRect,
    needs_power: bool,
    has_power: bool,
}

#[derive(Clone, Copy, Default)]
struct Deal {
    t: u32,
    amount: i32,
    used: i32,
    city: u64,
    flags: u32,
}

#[derive(Clone, Copy, Default)]
struct Conn {
    cell: (i32, i32),
    city: u64,
    active: bool,
    visited: bool,
    other: Option<u64>,
}

#[derive(Clone, Copy, Default)]
struct LineItem {
    dept: u64,
    line: u32,
    expenses: i64,
    secondary: i64,
    display: bool,
}

/// One saved city plus the host objects its records refer to.
#[derive(Clone)]
struct SavedCity {
    pkg: Package,
    root: Vec<u8>,
    hosts: HashMap<u32, u64>,
}

#[derive(Default)]
pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    sim: RefCell<Option<Rc<PlumbingSim>>>,
    next_token: Cell<u64>,
    saves: RefCell<BTreeMap<String, SavedCity>>,
    cells: Cell<(i32, i32)>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    wall_seed: Cell<u32>,
    funding: Cell<f32>,
    factor: Cell<Option<f32>>,
    no_terrain: Cell<bool>,
    no_disaster: Cell<bool>,
    no_department: Cell<bool>,
    no_grids: Cell<bool>,
    water: RefCell<BTreeMap<(i32, i32), bool>>,
    water_values: RefCell<BTreeMap<(i32, i32), i32>>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    lots: RefCell<BTreeMap<u64, Lot>>,
    deals: RefCell<BTreeMap<u64, Deal>>,
    conns: RefCell<BTreeMap<u64, Conn>>,
    /// Department token → ID.
    depts: RefCell<BTreeMap<u64, u32>>,
    line_items: RefCell<BTreeMap<u64, LineItem>>,
    labels: RefCell<BTreeMap<String, u64>>,
    names: RefCell<BTreeMap<u64, String>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    grids: RefCell<Vec<Rc<SimGrid<u8>>>>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            next_token: Cell::new(0x100),
            funding: Cell::new(1.0),
            factor: Cell::new(Some(1.0)),
            wall_seed: Cell::new(1),
            cells: Cell::new((64, 64)),
            ..Default::default()
        });
        *w.tuning.borrow_mut() = Some(Rc::new(Tuning::shipped_exemplar()));
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn sim(&self) -> R<Rc<PlumbingSim>> {
        self.sim
            .borrow()
            .clone()
            .ok_or_else(|| "no plumbing simulator (plumbing_create)".into())
    }

    /// A new token named `label` (the label also finds it).
    fn token(&self, label: &str) -> u64 {
        let t = self.next_token.get();
        self.next_token.set(t + 1);
        self.names.borrow_mut().insert(t, label.to_owned());
        self.labels.borrow_mut().insert(label.to_owned(), t);
        t
    }

    fn name(&self, t: u64) -> Json {
        Json::Str(self.names.borrow().get(&t).cloned().unwrap_or_else(|| format!("#{t}")))
    }

    fn opt_name(&self, t: Option<u64>) -> Json {
        t.map_or(Json::Null, |t| self.name(t))
    }

    fn find(&self, j: &Json) -> R<u64> {
        let l = str_of(j, "label")?;
        self.labels
            .borrow()
            .get(l)
            .copied()
            .ok_or_else(|| format!("unknown object '{l}'"))
    }

    /// The city token of a label, created on first use.
    fn city(&self, j: &Json) -> R<u64> {
        let l = str_of(j, "city")?;
        let found = self.labels.borrow().get(l).copied();
        Ok(found.unwrap_or_else(|| self.token(l)))
    }

    fn dims(&self) -> CityDims {
        let (x, z) = self.cells.get();
        CityDims {
            cells_x: x,
            cells_z: z,
            ..CityDims::square(1)
        }
    }

    fn grid_name(&self, g: &Option<GridRc>) -> Json {
        let Some(g) = g else { return Json::Null };
        let p = Rc::as_ptr(g) as *const ();
        self.grids
            .borrow()
            .iter()
            .find(|s| Rc::as_ptr(s) as *const () == p)
            .map_or(Json::Str("?".into()), |s| {
                Json::Str(format!("0x{:08X}", s.instance_id()))
            })
    }

    fn inside(r: CellRect, (x, z): (i32, i32)) -> bool {
        x >= r.x1 && x <= r.x2 && z >= r.z1 && z <= r.z2
    }

    fn line_of(&self, li: u64) -> Json {
        self.line_items.borrow().get(&li).map_or(Json::Null, |l| ji(l.line))
    }
}

impl GridProvider<u8> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        self.events.push(
            "create_grid",
            vec![("instance", ji(instance)), ("size", ji(tract_size))],
        );
        if self.no_grids.get() {
            return None;
        }
        let g = SimGrid::<u8>::create(&self.dims(), instance, tract_size);
        self.grids.borrow_mut().push(g.clone());
        Some(g as Rc<dyn TractGrid<u8>>)
    }

    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u8>>> {
        let g = r.object::<SimGrid<u8>>()?;
        self.grids.borrow_mut().push(g.clone());
        Some(g as Rc<dyn TractGrid<u8>>)
    }
}

fn hex(v: u32) -> Json {
    Json::Str(format!("0x{v:08X}"))
}

fn cell_json((x, z): (i32, i32)) -> Json {
    Json::Array(vec![ji(x), ji(z)])
}

impl PlumbingHost for World {
    fn cell_counts(&self) -> (i32, i32) {
        self.cells.get()
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        ((x / 16.0).floor() as i32, (z / 16.0).floor() as i32)
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn wall_clock_seed(&self) -> u32 {
        self.wall_seed.get()
    }
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        self.events.push(
            "agent",
            vec![
                ("level", ji(level)),
                ("name", Json::Str(name.into())),
                ("flags", ji(flags)),
            ],
        );
    }
    fn remove_agent(&self) {
        self.events.push("remove_agent", vec![]);
    }
    fn subscribe(&self, t: &[u32]) {
        self.events.push(
            "subscribe",
            vec![("types", Json::Array(t.iter().map(|&v| hex(v)).collect()))],
        );
    }
    fn unsubscribe(&self, t: &[u32]) {
        self.events.push(
            "unsubscribe",
            vec![("types", Json::Array(t.iter().map(|&v| hex(v)).collect()))],
        );
    }
    fn post(&self, msg_type: u32, data1: u32) {
        self.events
            .push("post", vec![("type", hex(msg_type)), ("data1", ji(data1))]);
    }
    fn record_history(&self, id: u32, value: u32) {
        self.events.push("history", vec![("id", hex(id)), ("value", ji(value))]);
    }
    fn cumulative_factor(&self, _id: u32, _index: u32) -> Option<f32> {
        self.factor.get()
    }
    fn water_value(&self, x: i32, z: i32) -> i32 {
        self.water_values.borrow().get(&(x, z)).copied().unwrap_or(0)
    }
    fn occupant_type(&self, occ: u64) -> u32 {
        match self.occs.borrow().get(&occ) {
            Some(o) if o.building => occupant_type::BUILDING,
            Some(o) if o.network => occupant_type::NETWORK,
            _ => 0,
        }
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant> {
        self.occs.borrow().get(&occ)?.props.get(id).cloned()
    }
    fn set_occupant_property(&self, occ: u64, id: u32, value: Variant) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.props.add(id, value);
        }
    }
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        let (x, z) = self.occs.borrow().get(&occ).map_or((0, 0), |o| o.cell);
        [x as f32 * 16.0 + 8.0, 0.0, z as f32 * 16.0 + 8.0]
    }
    fn occupant_bounding_box(&self, occ: u64) -> ([f32; 3], [f32; 3]) {
        let r = self.occs.borrow().get(&occ).map_or(CellRect::default(), |o| o.rect);
        (
            [r.x1 as f32 * 16.0, 0.0, r.z1 as f32 * 16.0],
            [(r.x2 + 1) as f32 * 16.0, 0.0, (r.z2 + 1) as f32 * 16.0],
        )
    }
    fn occupant_bounding_cells(&self, occ: u64) -> CellRect {
        self.occs
            .borrow()
            .get(&occ)
            .map_or(CellRect::new(0, 0, -1, -1), |o| o.rect)
    }
    fn building_type(&self, occ: u64) -> Option<u32> {
        self.occs
            .borrow()
            .get(&occ)
            .filter(|o| o.building)
            .map(|o| o.building_type)
    }
    fn buildings_in_cell(&self, x: i32, z: i32) -> Vec<u64> {
        self.occs
            .borrow()
            .iter()
            .filter(|(_, o)| o.building && World::inside(o.rect, (x, z)))
            .map(|(&k, _)| k)
            .collect()
    }
    fn is_network_occupant(&self, occ: u64) -> bool {
        self.occs.borrow().get(&occ).is_some_and(|o| o.network)
    }
    fn has_network_flag(&self, occ: u64, flag: u32) -> bool {
        self.network_flags(occ) & flag != 0
    }
    fn network_flags(&self, occ: u64) -> u32 {
        self.occs.borrow().get(&occ).map_or(0, |o| o.net_flags)
    }
    fn set_network_flag(&self, occ: u64, flag: u32) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.net_flags |= flag;
        }
    }
    fn clear_network_flag(&self, occ: u64, flag: u32) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.net_flags &= !flag;
        }
    }
    fn connects_to_neighbor(&self, occ: u64, dir: u32, network_type: u32) -> bool {
        network_type == network::PIPE_TYPE && self.occs.borrow().get(&occ).is_some_and(|o| o.connects.contains(&dir))
    }
    fn occupied_cell(&self, occ: u64) -> (i32, i32) {
        self.occs.borrow().get(&occ).map_or((0, 0), |o| o.cell)
    }
    fn set_pipe_watered(&self, occ: u64, watered: bool) {
        self.events.push(
            "pipe_watered",
            vec![("pipe", self.name(occ)), ("watered", Json::Bool(watered))],
        );
    }
    fn lot_at(&self, x: i32, z: i32) -> Option<u64> {
        self.lots
            .borrow()
            .iter()
            .find(|(_, l)| World::inside(l.rect, (x, z)))
            .map(|(&k, _)| k)
    }
    fn occupant_lot(&self, occ: u64) -> Option<u64> {
        let c = self.occs.borrow().get(&occ)?.cell;
        self.lot_at(c.0, c.1)
    }
    fn lot_rect(&self, lot: u64) -> CellRect {
        self.lots.borrow().get(&lot).map_or(CellRect::default(), |l| l.rect)
    }
    fn lot_needs_power(&self, lot: u64) -> bool {
        self.lots.borrow().get(&lot).is_some_and(|l| l.needs_power)
    }
    fn lot_has_power(&self, lot: u64) -> bool {
        self.lots.borrow().get(&lot).is_some_and(|l| l.has_power)
    }
    fn set_lot_condition(&self, lot: u64, condition: u8) {
        self.events.push(
            "lot_condition",
            vec![("lot", self.name(lot)), ("condition", ji(condition))],
        );
    }
    fn terrain_cell_counts(&self) -> Option<(i32, i32)> {
        (!self.no_terrain.get()).then(|| self.cells.get())
    }
    fn cell_has_water(&self, x: i32, z: i32) -> bool {
        self.water.borrow().contains_key(&(x, z))
    }
    fn redisplay_terrain(&self, min: (i32, i32), max: (i32, i32)) {
        self.events
            .push("redisplay", vec![("min", cell_json(min)), ("max", cell_json(max))]);
    }
    fn attempt_pipe_burst(&self, x: i32, z: i32) -> bool {
        let manager = !self.no_disaster.get();
        self.events.push(
            "burst",
            vec![("x", ji(x)), ("z", ji(z)), ("manager", Json::Bool(manager))],
        );
        manager
    }
    fn find_deals(&self, deal_type: u32) -> Vec<u64> {
        self.deals
            .borrow()
            .iter()
            .filter(|(_, d)| d.t == deal_type)
            .map(|(&k, _)| k)
            .collect()
    }
    fn deal_type(&self, deal: u64) -> u32 {
        self.deals.borrow().get(&deal).map_or(0, |d| d.t)
    }
    fn deal_city(&self, deal: u64) -> u64 {
        self.deals.borrow().get(&deal).map_or(0, |d| d.city)
    }
    fn deal_amount(&self, deal: u64) -> i32 {
        self.deals.borrow().get(&deal).map_or(0, |d| d.amount)
    }
    fn deal_amount_used(&self, deal: u64) -> i32 {
        self.deals.borrow().get(&deal).map_or(0, |d| d.used)
    }
    fn set_deal_amount_used(&self, deal: u64, used: i32) {
        if let Some(d) = self.deals.borrow_mut().get_mut(&deal) {
            d.used = used;
        }
        self.events
            .push("set_used", vec![("deal", self.name(deal)), ("used", ji(used))]);
    }
    fn set_deal_flags(&self, deal: u64, flags: u32, on: bool) {
        if let Some(d) = self.deals.borrow_mut().get_mut(&deal) {
            if on {
                d.flags |= flags;
            } else {
                d.flags &= !flags;
            }
        }
        self.events.push(
            "deal_flags",
            vec![("deal", self.name(deal)), ("flags", ji(flags)), ("on", Json::Bool(on))],
        );
    }
    fn connections_of_type(&self, t: u32) -> Vec<u64> {
        if t != dealk::WATER_CONNECTION {
            return Vec::new();
        }
        self.conns.borrow().keys().copied().collect()
    }
    fn connections_of_type_to_city(&self, t: u32, city: u64) -> Vec<u64> {
        let all = self.connections_of_type(t);
        let conns = self.conns.borrow();
        all.into_iter().filter(|c| conns[c].city == city).collect()
    }
    fn find_unvisited_connection(&self, t: u32, city: u64) -> Option<u64> {
        let all = self.connections_of_type_to_city(t, city);
        let conns = self.conns.borrow();
        all.into_iter().find(|c| !conns[c].visited)
    }
    fn connection_on_other_side(&self, conn: u64) -> Option<u64> {
        self.conns.borrow().get(&conn)?.other
    }
    fn connection_cell(&self, conn: u64) -> (i32, i32) {
        self.conns.borrow().get(&conn).map_or((0, 0), |c| c.cell)
    }
    fn set_connection_visited(&self, conn: u64, visited: bool) {
        if let Some(c) = self.conns.borrow_mut().get_mut(&conn) {
            c.visited = visited;
        }
    }
    fn connection_is_active(&self, conn: u64) -> bool {
        self.conns.borrow().get(&conn).is_some_and(|c| c.active)
    }
    fn set_connection_active(&self, conn: u64, active: bool) {
        if let Some(c) = self.conns.borrow_mut().get_mut(&conn) {
            c.active = active;
        }
    }
    fn mark_deals_dirty(&self) {
        self.events.push("deals_dirty", vec![]);
    }
    fn create_department(&self, id: u32, group: u32) -> Option<u64> {
        self.events
            .push("create_department", vec![("id", hex(id)), ("group", hex(group))]);
        if self.no_department.get() {
            return None;
        }
        let d = self.token("department");
        self.depts.borrow_mut().insert(d, id);
        Some(d)
    }
    fn set_department_name(&self, _dept: u64, group: u32, instance: u32) {
        self.events.push(
            "department_name",
            vec![("group", hex(group)), ("instance", hex(instance))],
        );
    }
    fn set_department_max_funding(&self, _dept: u64, fraction: f32) {
        self.events.push("max_funding", vec![("value", jf(fraction))]);
    }
    fn create_line_item(&self, dept: u64, line: u32, locally_funded: bool) -> Option<u64> {
        self.events.push(
            "create_line_item",
            vec![("line", ji(line)), ("locally_funded", Json::Bool(locally_funded))],
        );
        let li = self.token(&format!("line{line}"));
        self.line_items.borrow_mut().insert(
            li,
            LineItem {
                dept,
                line,
                ..LineItem::default()
            },
        );
        Some(li)
    }
    fn set_line_item_name(&self, li: u64, group: u32, instance: u32) {
        self.events.push(
            "line_item_name",
            vec![
                ("line", self.line_of(li)),
                ("group", hex(group)),
                ("instance", hex(instance)),
            ],
        );
    }
    fn remove_line_item(&self, dept: u64, line: u32) {
        self.events.push("remove_line_item", vec![("line", ji(line))]);
        self.line_items
            .borrow_mut()
            .retain(|_, l| !(l.dept == dept && l.line == line));
    }
    fn department_funding(&self, _dept: u64) -> f32 {
        self.funding.get()
    }
    fn department_by_id(&self, id: u32) -> Option<u64> {
        self.depts.borrow().iter().find(|(_, &i)| i == id).map(|(&t, _)| t)
    }
    fn line_item(&self, dept: u64, line: u32) -> Option<u64> {
        self.line_items
            .borrow()
            .iter()
            .find(|(_, l)| l.dept == dept && l.line == line)
            .map(|(&t, _)| t)
    }
    fn create_line_item_for_building_type(&self, dept: u64, line: u32, locally_funded: bool) -> Option<u64> {
        self.events.push(
            "create_building_line_item",
            vec![("line", ji(line)), ("locally_funded", Json::Bool(locally_funded))],
        );
        let li = self.token(&format!("line{line}"));
        self.line_items.borrow_mut().insert(
            li,
            LineItem {
                dept,
                line,
                ..LineItem::default()
            },
        );
        Some(li)
    }
    fn set_line_item_display_flag(&self, li: u64, flag: u32, on: bool) {
        if let Some(l) = self.line_items.borrow_mut().get_mut(&li) {
            l.display = on;
        }
        self.events.push(
            "display_flag",
            vec![("line", self.line_of(li)), ("flag", ji(flag)), ("on", Json::Bool(on))],
        );
    }
    fn add_to_line_item_expenses(&self, li: u64, delta: i64) {
        if let Some(l) = self.line_items.borrow_mut().get_mut(&li) {
            l.expenses = l.expenses.wrapping_add(delta);
        }
        self.events
            .push("add_expenses", vec![("line", self.line_of(li)), ("delta", ji(delta))]);
    }
    fn set_line_item_expenses(&self, li: u64, value: i64) {
        if let Some(l) = self.line_items.borrow_mut().get_mut(&li) {
            l.expenses = value;
        }
        self.events
            .push("set_expenses", vec![("line", self.line_of(li)), ("value", ji(value))]);
    }
    fn line_item_secondary_info(&self, li: u64) -> i64 {
        self.line_items.borrow().get(&li).map_or(0, |l| l.secondary)
    }
    fn set_line_item_secondary_info(&self, li: u64, value: i64) {
        if let Some(l) = self.line_items.borrow_mut().get_mut(&li) {
            l.secondary = value;
        }
        self.events
            .push("set_secondary", vec![("line", self.line_of(li)), ("value", ji(value))]);
    }
    fn budget_item_for_purpose(&self, occ: u64, _purpose: u32) -> Option<BudgetItem> {
        self.occs.borrow().get(&occ)?.budget
    }
    fn change_budget_item_line(&self, occ: u64, index: u32, line: u32) {
        if let Some(b) = self.occs.borrow_mut().get_mut(&occ).and_then(|o| o.budget.as_mut()) {
            b.line_item = line;
        }
        self.events.push(
            "change_budget_line",
            vec![("occupant", self.name(occ)), ("index", ji(index)), ("line", ji(line))],
        );
    }
    fn change_budget_item_cost(&self, occ: u64, index: u32, cost: i64) {
        if let Some(b) = self.occs.borrow_mut().get_mut(&occ).and_then(|o| o.budget.as_mut()) {
            b.cost = cost;
        }
        self.events.push(
            "change_budget_cost",
            vec![("occupant", self.name(occ)), ("index", ji(index)), ("cost", ji(cost))],
        );
    }
    fn line_item_from_budget_item(&self, item: &BudgetItem) -> Option<u64> {
        let d = self.department_by_id(item.department_id)?;
        self.line_item(d, item.line_item)
    }
    fn local_funding(&self, _dept: u64, occ: u64) -> Option<f32> {
        self.occs.borrow().get(&occ)?.local_funding
    }
    fn add_ref(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) += 1;
    }
    fn release(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) -= 1;
    }
    fn object_ref(&self, token: u64) -> RefValue {
        let class_id = if self.depts.borrow().contains_key(&token) {
            DEPARTMENT_CLASS
        } else {
            OCCUPANT_CLASS
        };
        RefValue::Host { token, class_id }
    }
    fn resolve_ref(&self, r: &RefValue, iid: u32) -> Option<u64> {
        let RefValue::Host { token, class_id } = r else {
            return None;
        };
        let ok = match (*class_id, iid) {
            (OCCUPANT_CLASS, IID_OCCUPANT) => true,
            (OCCUPANT_CLASS, IID_NETWORK_OCCUPANT) => self.is_network_occupant(*token),
            (DEPARTMENT_CLASS, IID_DEPARTMENT_BUDGET) => true,
            _ => false,
        };
        if ok {
            Some(*token)
        } else {
            // The stream's reference is dropped here.
            self.release(*token);
            None
        }
    }
    fn grid_shutdown(&self, grid: &GridRc) {
        let name = self.grid_name(&Some(grid.clone()));
        self.events.push("grid_shutdown", vec![("grid", name)]);
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "plumbing", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("plumbing_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "tuning" => tuning(&w, op),
        "building" => building(&w, op),
        "pipe" => pipe(&w, op),
        "pipe_row" => pipe_row(&w, op),
        "pipes" => pipes(&w, op),
        "occupant" => occupant(&w, op),
        "lot" => lot(&w, op),
        "deal" => deal(&w, op),
        "connection" => connection(&w, op),
        "create" => create(&w, op),
        "init" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.init()))),
        "shutdown" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.shutdown()))),
        "rng" => rng(&w, op),
        "gaussian" => gaussian(op),
        "slice" => slice(&w, op),
        "message" => message(&w, op),
        "insert" => insert_remove(&w, op, msg::INSERT_OCCUPANT),
        "remove" => insert_remove(&w, op, msg::REMOVE_OCCUPANT),
        "idle" => idle(&w, op),
        "pass" => pass(&w, op),
        "month" => w
            .sim()
            .and_then(|s| check_expect(op, Json::Bool(s.do_message(&Message::of_type(msg::SIM_NEW_MONTH))))),
        "call" => call(&w, op),
        "set" => set(&w, op),
        "expect" => expect(&w, op),
        "expect_tuning" => expect_tuning(&w, op),
        "expect_flags" => expect_flags(&w, op),
        "expect_grid" => expect_grid(&w, op),
        "expect_occupant" => expect_occupant(&w, op),
        "expect_deal" => expect_deal(&w, op),
        "expect_connection" => expect_connection(&w, op),
        "expect_line_item" => expect_line_item(&w, op),
        "expect_refs" => expect_refs(&w, op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        "save" => save(&w, op),
        "load" => load(&w, op),
        "expect_save" => expect_save(&w, op),
        "patch" => patch(&w, op),
        "expect_payload" => expect_payload(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    })
}

// ---- environment ----------------------------------------------------------------------------------

fn int_field(e: &Json, k: &str) -> R<i32> {
    Ok(i64_of(field(e, k)?, k)? as i32)
}

fn opt_i32(e: &Json, k: &str, default: i32) -> R<i32> {
    opt(e, k).map_or(Ok(default), |v| Ok(i64_of(v, k)? as i32))
}

fn opt_bool(e: &Json, k: &str, default: bool) -> R<bool> {
    opt(e, k).map_or(Ok(default), |v| bool_of(v, k))
}

/// `{x, z, w?, d?}` → the inclusive cell rectangle.
fn rect_of(e: &Json) -> R<CellRect> {
    let x = int_field(e, "x")?;
    let z = int_field(e, "z")?;
    let w = opt_i32(e, "w", 1)?;
    let d = opt_i32(e, "d", 1)?;
    if !(1..=4096).contains(&w) || !(1..=4096).contains(&d) {
        return Err("rectangle size must be 1..=4096".into());
    }
    Ok(CellRect::new(x, z, x + w - 1, z + d - 1))
}

fn pair_of(v: &Json, k: &str) -> R<(i32, i32)> {
    let a = arr(v, k)?;
    if a.len() != 2 {
        return Err(format!("'{k}' must have 2 elements"));
    }
    Ok((i64_of(&a[0], k)? as i32, i64_of(&a[1], k)? as i32))
}

/// `plumbing_env`: `cells? [x, z]`, `funding?`, `factor?` (null: no
/// ordinance simulator), `water? [[x, z], …]`, `water_rects? [{x, z, w, d}]`,
/// `clear_water?`, `water_values? [{x, z, v}]`, `no_terrain?`,
/// `no_disaster?`, `no_department?`, `no_grids?`, `wall_seed?`,
/// `departments? [{label, id}]`.
fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "cells") {
        let (x, z) = pair_of(v, "cells")?;
        if !(0..=4096).contains(&x) || !(0..=4096).contains(&z) {
            return Err("cells must be 0..=4096".into());
        }
        w.cells.set((x, z));
    }
    if let Some(v) = opt(op, "funding") {
        w.funding.set(f32_of(v, "funding")?);
    }
    if let Some(v) = op.get("factor") {
        w.factor.set(match v {
            Json::Null => None,
            v => Some(f32_of(v, "factor")?),
        });
    }
    if opt_bool(op, "clear_water", false)? {
        w.water.borrow_mut().clear();
    }
    if let Some(v) = opt(op, "water") {
        for c in arr(v, "water")? {
            w.water.borrow_mut().insert(pair_of(c, "water")?, true);
        }
    }
    if let Some(v) = opt(op, "water_rects") {
        for e in arr(v, "water_rects")? {
            let r = rect_of(e)?;
            for z in r.z1..=r.z2 {
                for x in r.x1..=r.x2 {
                    w.water.borrow_mut().insert((x, z), true);
                }
            }
        }
    }
    if let Some(v) = opt(op, "water_values") {
        for e in arr(v, "water_values")? {
            let c = (int_field(e, "x")?, int_field(e, "z")?);
            w.water_values.borrow_mut().insert(c, int_field(e, "v")?);
        }
    }
    for (k, c) in [
        ("no_terrain", &w.no_terrain),
        ("no_disaster", &w.no_disaster),
        ("no_department", &w.no_department),
        ("no_grids", &w.no_grids),
    ] {
        if let Some(v) = opt(op, k) {
            c.set(bool_of(v, k)?);
        }
    }
    if let Some(v) = opt(op, "wall_seed") {
        w.wall_seed.set(u32_of(v, "wall_seed")?);
    }
    if let Some(v) = opt(op, "departments") {
        for e in arr(v, "departments")? {
            let t = w.token(str_of(field(e, "label")?, "label")?);
            w.depts.borrow_mut().insert(t, u32_of(field(e, "id")?, "id")?);
        }
    }
    Ok(())
}

/// `plumbing_tuning`: `none?` (no exemplar), `code?` (an empty exemplar),
/// otherwise the shipped exemplar plus `props?` (`{"0xID": value}`;
/// `{"remove": true}` removes one). `reload?` sends the tuning message.
fn tuning(w: &World, op: &Json) -> R<()> {
    if opt_bool(op, "none", false)? {
        *w.tuning.borrow_mut() = None;
    } else {
        let mut h = if opt_bool(op, "code", false)? {
            PropertyHolder::new()
        } else {
            Tuning::shipped_exemplar()
        };
        if let Some(p) = opt(op, "props") {
            for (k, v) in p.as_object().ok_or("props must be an object")? {
                let id = Json::Str(k.clone()).as_i64().ok_or("bad property ID")? as u32;
                if v.get("remove").is_some() {
                    h.remove(id);
                } else {
                    h.add(id, scalar::variant_of(v).map_err(|e| format!("property {k}: {e}"))?);
                }
            }
        }
        *w.tuning.borrow_mut() = Some(Rc::new(h));
    }
    if opt_bool(op, "reload", false)? {
        let s = w.sim()?;
        s.do_message(&Message::of_type(msg::TUNING_RELOAD));
    }
    Ok(())
}

fn props_into(h: &mut PropertyHolder, op: &Json) -> R<()> {
    if let Some(p) = opt(op, "props") {
        for (k, v) in p.as_object().ok_or("props must be an object")? {
            let id = Json::Str(k.clone()).as_i64().ok_or("bad property ID")? as u32;
            if v.get("remove").is_some() {
                h.remove(id);
            } else {
                h.add(id, scalar::variant_of(v).map_err(|e| format!("property {k}: {e}"))?);
            }
        }
    }
    Ok(())
}

fn budget_of(v: &Json) -> R<BudgetItem> {
    Ok(BudgetItem {
        department_id: u32_of(field(v, "department")?, "department")?,
        line_item: u32_of(field(v, "line")?, "line")?,
        cost: i64_of(field(v, "cost")?, "cost")?,
        index: opt(v, "index").map_or(Ok(0), |x| u32_of(x, "index"))?,
    })
}

fn insert_occupant(w: &World, t: u64) -> R<()> {
    let s = w.sim()?;
    s.do_message(&Message {
        msg_type: msg::INSERT_OCCUPANT,
        occupant: Some(t),
        ..Message::default()
    });
    Ok(())
}

/// `plumbing_building`: `label`, the rectangle `{x, z, w?, d?}`, `anchor?
/// [x, z]` (default the first cell), `props?`, `lot?` (default true: a lot
/// `<label>_lot` over the rectangle), `lot_rect?`, `needs_power?`,
/// `has_power?`, `building_type?`, `budget? {department, line, cost,
/// index?}`, `local_funding?`, `insert?` (default true: InsertOccupant).
fn building(w: &World, op: &Json) -> R<()> {
    let label = str_of(field(op, "label")?, "label")?;
    let rect = rect_of(op)?;
    let anchor = opt(op, "anchor").map_or(Ok((rect.x1, rect.z1)), |v| pair_of(v, "anchor"))?;
    let mut o = Occ {
        building: true,
        cell: anchor,
        rect,
        building_type: opt(op, "building_type").map_or(Ok(0x1234), |v| u32_of(v, "building_type"))?,
        budget: opt(op, "budget").map(budget_of).transpose()?,
        local_funding: opt(op, "local_funding")
            .map(|v| f32_of(v, "local_funding"))
            .transpose()?,
        ..Occ::default()
    };
    props_into(&mut o.props, op)?;
    let t = w.token(label);
    w.occs.borrow_mut().insert(t, o);
    if opt_bool(op, "lot", true)? {
        let lr = match opt(op, "lot_rect") {
            Some(v) => rect_of(v)?,
            None => rect,
        };
        let l = w.token(&format!("{label}_lot"));
        w.lots.borrow_mut().insert(
            l,
            Lot {
                rect: lr,
                needs_power: opt_bool(op, "needs_power", false)?,
                has_power: opt_bool(op, "has_power", false)?,
            },
        );
    }
    if opt_bool(op, "insert", true)? {
        insert_occupant(w, t)?;
    }
    Ok(())
}

fn dirs_of(op: &Json) -> R<Vec<u32>> {
    match opt(op, "dirs") {
        Some(v) => arr(v, "dirs")?.iter().map(|d| u32_of(d, "dirs")).collect(),
        None => Ok(Vec::new()),
    }
}

fn add_pipe(w: &World, label: &str, rect: CellRect, cell: (i32, i32), dirs: Vec<u32>, flags: u32) -> u64 {
    let t = w.token(label);
    w.occs.borrow_mut().insert(
        t,
        Occ {
            network: true,
            cell,
            rect,
            connects: dirs,
            net_flags: flags,
            ..Occ::default()
        },
    );
    t
}

/// `plumbing_pipe`: `label`, `x`, `z` (or a rectangle with `w`, `d`),
/// `dirs?` (directions `ConnectsToNeighbor` answers), `flags?` (network
/// flags), `insert?` (default true).
fn pipe(w: &World, op: &Json) -> R<()> {
    let label = str_of(field(op, "label")?, "label")?;
    let rect = rect_of(op)?;
    let flags = opt(op, "flags").map_or(Ok(0), |v| u32_of(v, "flags"))?;
    let t = add_pipe(w, label, rect, (rect.x1, rect.z1), dirs_of(op)?, flags);
    if opt_bool(op, "insert", true)? {
        insert_occupant(w, t)?;
    }
    Ok(())
}

/// `plumbing_pipe_row`: `prefix`, from `from [x, z]` to `to [x, z]` (a row
/// or a column), connected to each other; labels `<prefix>0`, `<prefix>1`, …
fn pipe_row(w: &World, op: &Json) -> R<()> {
    let prefix = str_of(field(op, "prefix")?, "prefix")?;
    let (x0, z0) = pair_of(field(op, "from")?, "from")?;
    let (x1, z1) = pair_of(field(op, "to")?, "to")?;
    if x0 != x1 && z0 != z1 {
        return Err("a pipe row is a row or a column".into());
    }
    let n = (x1 - x0).abs().max((z1 - z0).abs());
    if n > 4096 {
        return Err("pipe row too long".into());
    }
    let (dx, dz) = ((x1 - x0).signum(), (z1 - z0).signum());
    for i in 0..=n {
        let (x, z) = (x0 + i * dx, z0 + i * dz);
        let mut dirs = Vec::new();
        // Directions: 0 = x−1, 1 = z−1, 2 = x+1, 3 = z+1.
        let back = match (dx, dz) {
            (1, _) => 0,
            (-1, _) => 2,
            (_, 1) => 1,
            _ => 3,
        };
        if i > 0 {
            dirs.push(back);
        }
        if i < n {
            dirs.push((back + 2) % 4);
        }
        let t = add_pipe(w, &format!("{prefix}{i}"), CellRect::new(x, z, x, z), (x, z), dirs, 0);
        insert_occupant(w, t)?;
    }
    Ok(())
}

/// `plumbing_pipes`: `prefix`, `count` unconnected pipes at
/// `(x0 + i mod width, z0 + i div width)`.
fn pipes(w: &World, op: &Json) -> R<()> {
    let prefix = str_of(field(op, "prefix")?, "prefix")?;
    let count = int_field(op, "count")?;
    let width = opt_i32(op, "width", 64)?.max(1);
    let (x0, z0) = (opt_i32(op, "x0", 0)?, opt_i32(op, "z0", 0)?);
    if !(0..=100_000).contains(&count) {
        return Err("count must be 0..=100000".into());
    }
    for i in 0..count {
        let (x, z) = (x0 + i % width, z0 + i / width);
        let t = add_pipe(
            w,
            &format!("{prefix}{i}"),
            CellRect::new(x, z, x, z),
            (x, z),
            Vec::new(),
            0,
        );
        insert_occupant(w, t)?;
    }
    Ok(())
}

/// `plumbing_occupant`: changes an occupant: `occupant`, `props?`,
/// `net_flags?`, `budget?` (null removes it), `local_funding?`.
fn occupant(w: &World, op: &Json) -> R<()> {
    let t = w.find(field(op, "occupant")?)?;
    let mut occs = w.occs.borrow_mut();
    let o = occs.get_mut(&t).ok_or("not an occupant")?;
    props_into(&mut o.props, op)?;
    if let Some(v) = opt(op, "net_flags") {
        o.net_flags = u32_of(v, "net_flags")?;
    }
    if let Some(v) = op.get("budget") {
        o.budget = match v {
            Json::Null => None,
            v => Some(budget_of(v)?),
        };
    }
    if let Some(v) = opt(op, "local_funding") {
        o.local_funding = Some(f32_of(v, "local_funding")?);
    }
    Ok(())
}

/// `plumbing_lot`: `lot` (label), `needs_power?`, `has_power?`, or a new
/// lot with `label` and a rectangle.
fn lot(w: &World, op: &Json) -> R<()> {
    if let Some(l) = opt(op, "lot") {
        let t = w.find(l)?;
        let mut lots = w.lots.borrow_mut();
        let lot = lots.get_mut(&t).ok_or("not a lot")?;
        lot.needs_power = opt_bool(op, "needs_power", lot.needs_power)?;
        lot.has_power = opt_bool(op, "has_power", lot.has_power)?;
        return Ok(());
    }
    let t = w.token(str_of(field(op, "label")?, "label")?);
    w.lots.borrow_mut().insert(
        t,
        Lot {
            rect: rect_of(op)?,
            needs_power: opt_bool(op, "needs_power", false)?,
            has_power: opt_bool(op, "has_power", false)?,
        },
    );
    Ok(())
}

/// `plumbing_deal`: `label` (new) or `deal` (existing), `type` ("import" /
/// "export" / a number), `amount`, `used?`, `city` (label), `flags?`.
fn deal(w: &World, op: &Json) -> R<()> {
    if let Some(d) = opt(op, "deal") {
        let t = w.find(d)?;
        let mut deals = w.deals.borrow_mut();
        let deal = deals.get_mut(&t).ok_or("not a deal")?;
        if let Some(v) = opt(op, "amount") {
            deal.amount = i64_of(v, "amount")? as i32;
        }
        if let Some(v) = opt(op, "used") {
            deal.used = i64_of(v, "used")? as i32;
        }
        if let Some(v) = opt(op, "flags") {
            deal.flags = u32_of(v, "flags")?;
        }
        return Ok(());
    }
    let t = match field(op, "type")?.as_str() {
        Some("import") => dealk::IMPORT,
        Some("export") => dealk::EXPORT,
        _ => u32_of(field(op, "type")?, "type")?,
    };
    let city = w.city(field(op, "city")?)?;
    let d = Deal {
        t,
        amount: int_field(op, "amount")?,
        used: opt_i32(op, "used", 0)?,
        city,
        flags: opt(op, "flags").map_or(Ok(0), |v| u32_of(v, "flags"))?,
    };
    let label = str_of(field(op, "label")?, "label")?;
    let token = w.token(label);
    w.deals.borrow_mut().insert(token, d);
    Ok(())
}

/// `plumbing_connection`: `label`, `x`, `z`, `city`, `active?`, `visited?`,
/// `other?` (the label of the connection on the other side).
fn connection(w: &World, op: &Json) -> R<()> {
    let c = Conn {
        cell: (int_field(op, "x")?, int_field(op, "z")?),
        city: w.city(field(op, "city")?)?,
        active: opt_bool(op, "active", false)?,
        visited: opt_bool(op, "visited", false)?,
        other: opt(op, "other").map(|v| w.find(v)).transpose()?,
    };
    let t = w.token(str_of(field(op, "label")?, "label")?);
    w.conns.borrow_mut().insert(t, c);
    Ok(())
}

// ---- driving the simulator ------------------------------------------------------------------------

fn create(w: &World, _op: &Json) -> R<()> {
    let me = w.rc();
    let t = w.token("plumbing");
    *w.sim.borrow_mut() = Some(PlumbingSim::new(me as Rc<dyn PlumbingHost>, t));
    Ok(())
}

fn seeds(w: &World) -> FixedSeeds {
    FixedSeeds {
        wall_clock: w.wall_seed.get(),
        process_clock: 0,
    }
}

/// `plumbing_rng`: replaces the simulator's generator with one seeded with
/// `seed`.
fn rng(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    s.set_random(RzRandom::new(u32_of(field(op, "seed")?, "seed")?, &seeds(w)));
    Ok(())
}

/// `plumbing_gaussian`: `seed`, `calls [{lo, hi, expect}]`: successive
/// `RandomSint32RangeGaussianFast` calls on a fresh generator; `raws?`
/// checks the next raw draws afterwards.
fn gaussian(op: &Json) -> R<()> {
    let seeds = FixedSeeds {
        wall_clock: 1,
        process_clock: 0,
    };
    let mut r = RzRandom::new(u32_of(field(op, "seed")?, "seed")?, &seeds);
    for (i, c) in arr(field(op, "calls")?, "calls")?.iter().enumerate() {
        let lo = int_field(c, "lo")?;
        let hi = int_field(c, "hi")?;
        // The method is `lo` for an empty range (always four draws).
        let v = r.range_i32_gaussian_fast(lo, hi);
        let v = if hi <= lo { lo } else { v };
        let want = i64_of(field(c, "expect")?, "expect")?;
        if i64::from(v) != want {
            return Err(format!("call {i}: got {v}, expected {want}"));
        }
    }
    if let Some(v) = opt(op, "raws") {
        for want in arr(v, "raws")? {
            let got = r.next_raw();
            if i64::from(got) != i64_of(want, "raws")? {
                return Err(format!("raw draw {got}, expected {}", scalar::show(want)));
            }
        }
    }
    Ok(())
}

/// `plumbing_slice`: `cells` per call in states 6 and 7 (null: whole).
fn slice(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let n = match op.get("cells") {
        None | Some(Json::Null) => None,
        Some(v) => Some(u32_of(v, "cells")?),
    };
    s.set_slice_cells(n);
    Ok(())
}

fn msg_type(j: &Json) -> R<u32> {
    Ok(match j.as_str() {
        Some("SimNewMonth") => msg::SIM_NEW_MONTH,
        Some("SimNewDay") => msg::SIM_NEW_DAY,
        Some("SimIdle") => msg::SIM_IDLE,
        Some("ConnectionsReady") => msg::CONNECTIONS_READY,
        Some("InsertOccupant") => msg::INSERT_OCCUPANT,
        Some("RemoveOccupant") => msg::REMOVE_OCCUPANT,
        Some("TuningReload") => msg::TUNING_RELOAD,
        Some("PipeBurst") => msg::PIPE_BURST,
        Some("DealStatus") => msg::DEAL_STATUS,
        _ => u32_of(j, "type")?,
    })
}

/// `plumbing_message`: `type`, `data1?`, `data2?`, `data3?`, `occupant?`,
/// `deal?` (labels), `expect?` (the result).
fn message(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let d = |k: &str| opt(op, k).map_or(Ok(0), |v| u32_of(v, k));
    let m = Message {
        msg_type: msg_type(field(op, "type")?)?,
        data1: d("data1")?,
        data2: d("data2")?,
        data3: d("data3")?,
        occupant: opt(op, "occupant").map(|v| w.find(v)).transpose()?,
        deal: opt(op, "deal").map(|v| w.find(v)).transpose()?,
    };
    check_expect(op, Json::Bool(s.do_message(&m)))
}

fn insert_remove(w: &World, op: &Json, t: u32) -> R<()> {
    let s = w.sim()?;
    let occ = w.find(field(op, "occupant")?)?;
    s.do_message(&Message {
        msg_type: t,
        occupant: Some(occ),
        ..Message::default()
    });
    Ok(())
}

/// `plumbing_idle`: `count?` DoIdle calls (`ms?`, default 5) through the
/// idle message; `expect?` is the last result.
fn idle(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let count = opt_i32(op, "count", 1)?;
    if !(1..=100_000).contains(&count) {
        return Err("'count' must be 1..=100000".into());
    }
    let ms = opt(op, "ms").map_or(Ok(5), |v| u32_of(v, "ms"))?;
    let mut r = true;
    for _ in 0..count {
        r = s.do_message(&Message {
            msg_type: msg::SIM_IDLE,
            data3: ms,
            ..Message::default()
        });
    }
    check_expect(op, Json::Bool(r))
}

/// `plumbing_pass`: starts a pass (sets recompute when idle) and calls
/// DoIdle until the state is 9 again; `calls?` checks the number of calls.
fn pass(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    s.with_state(|st| {
        if st.state >= 9 {
            st.recompute = true;
        }
    });
    let mut calls = 0i64;
    loop {
        s.do_idle(5);
        calls += 1;
        if s.with_state(|st| st.state) == 9 {
            break;
        }
        if calls > 1_000_000 {
            return Err("the pass did not end".into());
        }
    }
    if let Some(v) = opt(op, "calls") {
        if calls != i64_of(v, "calls")? {
            return Err(format!("the pass took {calls} calls"));
        }
    }
    Ok(())
}

fn a_i32(op: &Json, i: usize) -> R<i32> {
    Ok(i64_of(arg(op, i)?, "argument")? as i32)
}

fn a_u32(op: &Json, i: usize) -> R<u32> {
    u32_of(arg(op, i)?, "argument")
}

fn a_obj(w: &World, op: &Json, i: usize) -> R<u64> {
    w.find(arg(op, i)?)
}

/// `plumbing_call`: `method`, `args?`, `expect?` (the interface slots of
/// §10 and the simulator's own steps; README lists them).
fn call(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let method = str_of(field(op, "method")?, "method")?;
    let got = match method {
        "query_interface" => Json::Bool(answers_iid(a_u32(op, 0)?)),
        "add_ref" => ji(s.add_ref()),
        "release" => ji(s.release()),
        "slot_return" => {
            let slot = a_u32(op, 0)?;
            let r = SLOT_RETURNS.iter().find(|(n, _)| *n == slot).ok_or("no such slot")?;
            Json::Str(format!("{:?}", r.1))
        }
        "supply_value" => ji(s.supply_value(a_i32(op, 0)?, a_i32(op, 1)?)),
        "supply_value_map_ptr" | "watered_grid" => w.grid_name(&s.watered_grid()),
        "are_watered_buildings_in_cell" => Json::Bool(s.are_watered_buildings_in_cell(a_i32(op, 0)?, a_i32(op, 1)?)),
        "is_building_watered" => Json::Bool(s.is_building_watered(a_obj(w, op, 0)?)),
        "water_production" => ji(s.water_production(a_obj(w, op, 0)?)),
        "is_cell_watered" => Json::Bool(s.is_cell_watered(a_i32(op, 0)?, a_i32(op, 1)?)),
        "num_water_sources" => ji(s.num_water_sources()),
        "num_polluted_water_sources" => ji(s.num_polluted_water_sources()),
        "water_production_capacity" => ji(eax_of(s.last_month().produced)),
        "water_consumed" => ji(eax_of(s.last_month().consumed)),
        "water_imported" => ji(eax_of(s.last_month().imported)),
        "water_exported" => ji(eax_of(s.last_month().exported)),
        "num_pipes" => ji(s.pipe_counts().0),
        "num_distressed_pipes" => ji(s.pipe_counts().1),
        "num_burst_pipes" => ji(s.pipe_counts().2),
        "num_watered_buildings" => ji(s.num_watered_buildings()),
        "num_unwatered_buildings" => ji(s.num_unwatered_buildings()),
        "water_demand" => ji(eax_of(s.water_demand())),
        "water_demand_64" => ji(s.water_demand()),
        "water_producer_info" => match s.water_producer_info(a_obj(w, op, 0)?) {
            None => Json::Null,
            Some(i) => Json::Object(vec![
                ("production".into(), ji(i.production)),
                ("max_capacity".into(), ji(i.max_capacity)),
                ("efficiency".into(), jf(i.efficiency)),
                ("polluted".into(), Json::Bool(i.polluted)),
                ("used".into(), ji(i.used)),
            ]),
        },
        "water_producer_condition_percent" => s
            .water_producer_condition_percent(a_obj(w, op, 0)?)
            .map_or(Json::Null, jf),
        "occupant_monthly_upkeep_cost" => s.occupant_monthly_upkeep_cost(a_obj(w, op, 0)?).map_or(Json::Null, ji),
        "find_water_buildings" => {
            let (lowest, worst) = s.find_water_buildings();
            Json::Object(vec![
                ("lowest".into(), w.opt_name(lowest.map(|l| l.0))),
                ("funding".into(), lowest.map_or(Json::Null, |l| jf(l.1))),
                ("worst".into(), w.opt_name(worst.map(|l| l.0))),
                ("pct".into(), worst.map_or(Json::Null, |l| ji(l.1))),
            ])
        }
        "water_producers" => Json::Array(s.water_producers().into_iter().map(|t| w.name(t)).collect()),
        "nearest_water" => ji(s.nearest_water(a_i32(op, 0)?, a_i32(op, 1)?, a_u32(op, 2)? as u8, a_u32(op, 3)?)),
        "nearest_water_ring" => {
            let water = w.water.borrow();
            ji(nearest_water_ring(
                a_i32(op, 0)?,
                a_i32(op, 1)?,
                a_u32(op, 2)?,
                w.cells.get(),
                &mut |x, z| water.contains_key(&(x, z)),
            ))
        }
        "update_water_production" => {
            s.update_water_production();
            Json::Null
        }
        "age_water_producers" => {
            s.age_water_producers();
            Json::Null
        }
        "building_consumption" => ji(s.building_consumption(a_i32(op, 0)?, a_i32(op, 1)?)),
        "is_connected_to_current_network" => Json::Bool(s.is_connected_to_current_network(w.city(arg(op, 0)?)?)),
        "pipe_cost" => ji(pipe_cost(a_u32(op, 0)?, f32_of(arg(op, 1)?, "argument")?)),
        "trunc" => ji(trunc_i64(f32_of(arg(op, 0)?, "argument")?)),
        "eax_of" => ji(eax_of(i64_of(arg(op, 0)?, "argument")?)),
        other => return Err(format!("unknown method '{other}'")),
    };
    check_expect(op, got)
}

// ---- state ----------------------------------------------------------------------------------------

fn rect_json(r: CellRect) -> Json {
    Json::Array(vec![ji(r.x1), ji(r.z1), ji(r.x2), ji(r.z2)])
}

fn state_json(w: &World, s: &PlumbingState, sim: &PlumbingSim) -> Vec<(String, Json)> {
    let producers = s
        .producers
        .iter()
        .map(|p| {
            Json::Object(vec![
                ("occupant".into(), w.name(p.occupant)),
                ("seed".into(), cell_json((p.seed_x, p.seed_z))),
                ("rect".into(), rect_json(p.rect)),
                ("max_capacity".into(), ji(p.max_capacity)),
                ("production".into(), ji(p.production)),
                ("efficiency".into(), jf(p.efficiency)),
                ("polluted".into(), Json::Bool(p.polluted)),
                ("visited".into(), Json::Bool(p.visited)),
                ("assigned".into(), Json::Bool(p.assigned)),
            ])
        })
        .collect();
    let iterator = s.iterator.as_ref().map_or(Json::Null, |it| {
        Json::Object(vec![
            (
                "state".into(),
                ji(match it.state {
                    ScanState::Fresh => 0,
                    ScanState::Running => 1,
                    ScanState::Done => 2,
                }),
            ),
            ("x".into(), ji(it.x)),
            ("z".into(), ji(it.z)),
            (
                "rect".into(),
                Json::Array(vec![ji(it.x0), ji(it.z0), ji(it.x1), ji(it.z1)]),
            ),
        ])
    });
    let b = Json::Bool;
    vec![
        ("state".into(), ji(s.state)),
        ("recompute".into(), b(s.recompute)),
        ("ageing".into(), b(s.ageing)),
        ("enabled".into(), b(s.enabled)),
        ("initialized".into(), b(s.initialized)),
        ("grids_initialized".into(), b(s.grids_initialized)),
        ("destroyed".into(), b(s.destroyed)),
        ("ref_count".into(), ji(s.ref_count)),
        ("count_x".into(), ji(s.count_x)),
        ("count_z".into(), ji(s.count_z)),
        ("flag_map_size".into(), ji(s.flags.len() as i64)),
        ("bitmap_side".into(), ji(s.visited.width())),
        ("pass_counter".into(), ji(s.pass_counter)),
        ("average_water_pollution".into(), ji(s.average_water_pollution)),
        ("total_water_cost".into(), ji(s.total_water_cost)),
        ("num_polluted".into(), ji(s.num_polluted)),
        ("num_pipes".into(), ji(s.num_pipes)),
        ("num_distressed".into(), ji(s.num_distressed)),
        ("num_burst".into(), ji(s.num_burst)),
        ("pipe_condition".into(), jf(s.pipe_condition)),
        ("demand".into(), ji(s.demand)),
        ("factor".into(), jf(s.factor)),
        ("last_produced".into(), ji(s.last.produced)),
        ("last_consumed".into(), ji(s.last.consumed)),
        ("last_imported".into(), ji(s.last.imported)),
        ("last_exported".into(), ji(s.last.exported)),
        ("pass_produced".into(), ji(s.pass.produced)),
        ("pass_consumed".into(), ji(s.pass.consumed)),
        ("pass_imported".into(), ji(s.pass.imported)),
        ("pass_exported".into(), ji(s.pass.exported)),
        ("building_cells".into(), ji(s.building_cells)),
        ("watered_cells".into(), ji(s.watered_cells)),
        ("last_building_cells".into(), ji(s.last_building_cells)),
        ("last_watered_cells".into(), ji(s.last_watered_cells)),
        ("supply".into(), ji(s.supply)),
        ("consumed".into(), ji(s.consumed)),
        ("import_owed".into(), ji(s.import_owed)),
        ("ring".into(), ji(s.ring)),
        ("last_pass_pipe_count".into(), ji(s.last_pass_pipe_count)),
        ("unused_counter".into(), ji(s.unused_counter)),
        ("producers".into(), Json::Array(producers)),
        ("current".into(), w.opt_name(s.current)),
        (
            "pipes".into(),
            Json::Array(s.pipes.iter().map(|&p| w.name(p)).collect()),
        ),
        (
            "leaked".into(),
            Json::Array(s.leaked.iter().map(|&p| w.name(p)).collect()),
        ),
        ("department".into(), w.opt_name(s.department)),
        ("iterator".into(), iterator),
        ("working_grid".into(), w.grid_name(&s.working)),
        ("published_grid".into(), w.grid_name(&s.published)),
        ("has_rng".into(), b(s.rng.is_some())),
        (
            "rng_lcg".into(),
            s.rng.as_ref().map_or(Json::Null, |r| ji(r.lcg_state())),
        ),
        ("crt_state".into(), ji(sim.crt().state())),
    ]
}

/// `plumbing_expect`: `state` (a partial object of the fields above;
/// nested objects, such as producer records, are matched partially too).
fn expect(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let st = s.snapshot();
    let got = state_json(w, &st, &s);
    let want = field(op, "state")?.as_object().ok_or("'state' must be an object")?;
    let mut errors = Vec::new();
    for (k, v) in want {
        match got.iter().find(|(gk, _)| gk == k) {
            None => errors.push(format!("unknown state field '{k}'")),
            Some((_, g)) if !scalar::matches(g, v, true) => {
                errors.push(format!("{k}: got {}, expected {}", scalar::show(g), scalar::show(v)))
            }
            Some(_) => {}
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// `plumbing_set`: `state` with any of `state`, `recompute`, `ageing`,
/// `enabled`, `pipe_condition`, `num_pipes`, `num_distressed`, `num_burst`,
/// `demand`, `pass_counter`, `total_water_cost`, `last_produced`,
/// `last_consumed`, `last_imported`, `last_exported`; `flags? [{x, z, v}]`;
/// `production? [{occupant, production}]`; `crt?` (the `rand()` state).
fn set(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let mut prod = Vec::new();
    if let Some(v) = opt(op, "production") {
        for e in arr(v, "production")? {
            prod.push((
                w.find(field(e, "occupant")?)?,
                u32_of(field(e, "production")?, "production")?,
            ));
        }
    }
    let mut flags = Vec::new();
    if let Some(v) = opt(op, "flags") {
        for e in arr(v, "flags")? {
            flags.push((
                int_field(e, "x")?,
                int_field(e, "z")?,
                u32_of(field(e, "v")?, "v")? as u8,
            ));
        }
    }
    if let Some(v) = opt(op, "crt") {
        s.set_crt(fc4_core::rng::CrtRand::new(u32_of(v, "crt")?));
    }
    let state = opt(op, "state").and_then(Json::as_object).unwrap_or(&[]).to_vec();
    s.with_state(|st| -> R<()> {
        for (k, v) in &state {
            match k.as_str() {
                "state" => st.state = u32_of(v, k)?,
                "recompute" => st.recompute = bool_of(v, k)?,
                "ageing" => st.ageing = bool_of(v, k)?,
                "enabled" => st.enabled = bool_of(v, k)?,
                "pipe_condition" => st.pipe_condition = f32_of(v, k)?,
                "num_pipes" => st.num_pipes = u32_of(v, k)?,
                "num_distressed" => st.num_distressed = u32_of(v, k)?,
                "num_burst" => st.num_burst = u32_of(v, k)?,
                "demand" => st.demand = i64_of(v, k)?,
                "pass_counter" => st.pass_counter = i64_of(v, k)? as i32,
                "total_water_cost" => st.total_water_cost = i64_of(v, k)?,
                "last_produced" => st.last.produced = i64_of(v, k)?,
                "last_consumed" => st.last.consumed = i64_of(v, k)?,
                "last_imported" => st.last.imported = i64_of(v, k)?,
                "last_exported" => st.last.exported = i64_of(v, k)?,
                other => return Err(format!("unknown state field '{other}'")),
            }
        }
        for &(x, z, v) in &flags {
            st.update_flag(x, z, |_| v);
        }
        for &(occ, p) in &prod {
            if let Some(r) = st.producers.iter_mut().find(|r| r.occupant == occ) {
                r.production = p;
            }
        }
        Ok(())
    })
}

/// `plumbing_expect_tuning`: any of the scalar fields, `eval? [{curve, x,
/// y}]` (curves: efficiency, funding_decay, usage_decay, pipe_decay),
/// `empty? [curve]`.
fn expect_tuning(w: &World, op: &Json) -> R<()> {
    let t = w.sim()?.tuning();
    let got = vec![
        ("radius".to_string(), ji(t.radius)),
        ("fresh_reach".into(), ji(t.fresh_reach)),
        ("salt_reach".into(), ji(t.salt_reach)),
        ("distance_loss".into(), ji(t.distance_loss)),
        ("max_pumped_pollution".into(), ji(t.max_pumped_pollution)),
        ("burst_probability".into(), jf(t.burst_probability)),
        ("max_funding".into(), jf(t.max_funding)),
        ("cost_per_pipe".into(), jf(t.cost_per_pipe)),
    ];
    if let Some(v) = opt(op, "fields") {
        check_state(&got, v)?;
    }
    let curve = |n: &str| -> R<&fc4_core::curve::ResponseCurve> {
        Ok(match n {
            "efficiency" => &t.efficiency_curve,
            "funding_decay" => &t.funding_decay_curve,
            "usage_decay" => &t.usage_decay_curve,
            "pipe_decay" => &t.pipe_decay_curve,
            _ => return Err(format!("unknown curve '{n}'")),
        })
    };
    if let Some(v) = opt(op, "eval") {
        for e in arr(v, "eval")? {
            let c = curve(str_of(field(e, "curve")?, "curve")?)?;
            let y = c.eval(f32_of(field(e, "x")?, "x")?);
            if !scalar::matches(&jf(y), field(e, "y")?, false) {
                return Err(format!("{}: got {y}", scalar::show(e)));
            }
        }
    }
    if let Some(v) = opt(op, "empty") {
        for n in arr(v, "empty")? {
            if !curve(str_of(n, "empty")?)?.is_empty() {
                return Err(format!("curve {} is not empty", scalar::show(n)));
            }
        }
    }
    Ok(())
}

/// `plumbing_expect_flags`: `cells [{x, z, v}]` of the flag map.
fn expect_flags(w: &World, op: &Json) -> R<()> {
    let st = w.sim()?.snapshot();
    for e in arr(field(op, "cells")?, "cells")? {
        let (x, z) = (int_field(e, "x")?, int_field(e, "z")?);
        let got = st.flag(x, z);
        if i64::from(got) != i64_of(field(e, "v")?, "v")? {
            return Err(format!("flag ({x}, {z}) = 0x{got:02X}"));
        }
    }
    Ok(())
}

/// `plumbing_expect_grid`: `grid` ("published" or "working"), `instance?`
/// (its creation instance ID), `cells? [{x, z, v}]`, `rects? [{x, z, w, d,
/// v}]`, `nonzero?` (the number of non-zero cells).
fn expect_grid(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let g = match str_of(field(op, "grid")?, "grid")? {
        "published" => s.watered_grid(),
        "working" => s.working_grid(),
        other => return Err(format!("unknown grid '{other}'")),
    };
    if let Some(v) = op.get("instance") {
        let got = w.grid_name(&g);
        let want = match v {
            Json::Null => Json::Null,
            v => Json::Str(format!("0x{:08X}", u32_of(v, "instance")?)),
        };
        if got != want {
            return Err(format!("grid instance {}", scalar::show(&got)));
        }
    }
    let Some(g) = g else {
        return if opt(op, "cells").is_some() || opt(op, "rects").is_some() {
            Err("no grid".into())
        } else {
            Ok(())
        };
    };
    let mut want = Vec::new();
    if let Some(v) = opt(op, "cells") {
        for e in arr(v, "cells")? {
            want.push((int_field(e, "x")?, int_field(e, "z")?, i64_of(field(e, "v")?, "v")?));
        }
    }
    if let Some(v) = opt(op, "rects") {
        for e in arr(v, "rects")? {
            let r = rect_of(e)?;
            let val = i64_of(field(e, "v")?, "v")?;
            for z in r.z1..=r.z2 {
                for x in r.x1..=r.x2 {
                    want.push((x, z, val));
                }
            }
        }
    }
    for (x, z, v) in want {
        let got = g.get(x, z);
        if i64::from(got) != v {
            return Err(format!("grid ({x}, {z}) = {got}, expected {v}"));
        }
    }
    if let Some(v) = opt(op, "nonzero") {
        let geo = g.geometry();
        let mut n = 0i64;
        for z in 0..geo.count_z {
            for x in 0..geo.count_x {
                if g.get(x, z) != 0 {
                    n += 1;
                }
            }
        }
        if n != i64_of(v, "nonzero")? {
            return Err(format!("{n} non-zero cells"));
        }
    }
    Ok(())
}

fn prop_json(v: &Variant) -> Json {
    let one = |e: &Elem| match *e {
        Elem::F32(f) => jf(f),
        Elem::F64(f) => Json::Float(f),
        other => ji(other.as_i64()),
    };
    match v {
        Variant::Scalar(e) => one(e),
        _ => Json::Array(v.items().iter().map(one).collect()),
    }
}

/// `plumbing_expect_occupant`: `occupant`, `props?` (`{"0xID": value, or
/// null for absent}`), `net_flags?`, `budget_cost?`.
fn expect_occupant(w: &World, op: &Json) -> R<()> {
    let t = w.find(field(op, "occupant")?)?;
    let occs = w.occs.borrow();
    let o = occs.get(&t).ok_or("not an occupant")?;
    if let Some(p) = opt(op, "props") {
        for (k, want) in p.as_object().ok_or("props must be an object")? {
            let id = Json::Str(k.clone()).as_i64().ok_or("bad property ID")? as u32;
            let got = o.props.get(id).map_or(Json::Null, prop_json);
            if !scalar::matches(&got, want, false) {
                return Err(format!(
                    "property {k}: got {}, expected {}",
                    scalar::show(&got),
                    scalar::show(want)
                ));
            }
        }
    }
    if let Some(v) = opt(op, "net_flags") {
        if i64::from(o.net_flags) != i64_of(v, "net_flags")? {
            return Err(format!("network flags 0x{:X}", o.net_flags));
        }
    }
    if let Some(v) = opt(op, "budget_cost") {
        let got = o.budget.map(|b| b.cost);
        if got != Some(i64_of(v, "budget_cost")?) {
            return Err(format!("budget cost {got:?}"));
        }
    }
    Ok(())
}

/// `plumbing_expect_deal`: `deal`, `used?`, `flags?`.
fn expect_deal(w: &World, op: &Json) -> R<()> {
    let t = w.find(field(op, "deal")?)?;
    let d = *w.deals.borrow().get(&t).ok_or("not a deal")?;
    let got = vec![("used".to_string(), ji(d.used)), ("flags".into(), ji(d.flags))];
    let want: Vec<(String, Json)> = op
        .as_object()
        .unwrap_or(&[])
        .iter()
        .filter(|(k, _)| k == "used" || k == "flags")
        .cloned()
        .collect();
    check_state(&got, &Json::Object(want))
}

/// `plumbing_expect_connection`: `connection`, `active?`, `visited?`.
fn expect_connection(w: &World, op: &Json) -> R<()> {
    let t = w.find(field(op, "connection")?)?;
    let c = *w.conns.borrow().get(&t).ok_or("not a connection")?;
    let got = vec![
        ("active".to_string(), Json::Bool(c.active)),
        ("visited".into(), Json::Bool(c.visited)),
    ];
    let want: Vec<(String, Json)> = op
        .as_object()
        .unwrap_or(&[])
        .iter()
        .filter(|(k, _)| k == "active" || k == "visited")
        .cloned()
        .collect();
    check_state(&got, &Json::Object(want))
}

/// `plumbing_expect_line_item`: `line` (of the water department, or of
/// `department` by ID), `expenses?`, `secondary?`, `display?`, `exists?`.
fn expect_line_item(w: &World, op: &Json) -> R<()> {
    let line = u32_of(field(op, "line")?, "line")?;
    let dept_id =
        opt(op, "department").map_or(Ok(fc4_core::plumbing::budget::DEPARTMENT), |v| u32_of(v, "department"))?;
    let item = w.department_by_id(dept_id).and_then(|d| w.line_item(d, line));
    let exists = opt_bool(op, "exists", true)?;
    let Some(li) = item else {
        return if exists {
            Err(format!("no line item {line}"))
        } else {
            Ok(())
        };
    };
    if !exists {
        return Err(format!("line item {line} exists"));
    }
    let l = w.line_items.borrow()[&li];
    let got = vec![
        ("expenses".to_string(), ji(l.expenses)),
        ("secondary".into(), ji(l.secondary)),
        ("display".into(), Json::Bool(l.display)),
    ];
    let want: Vec<(String, Json)> = op
        .as_object()
        .unwrap_or(&[])
        .iter()
        .filter(|(k, _)| ["expenses", "secondary", "display"].contains(&k.as_str()))
        .cloned()
        .collect();
    check_state(&got, &Json::Object(want))
}

/// `plumbing_expect_refs`: `{"label": count}` (references the simulator
/// holds; objects start at 0).
fn expect_refs(w: &World, op: &Json) -> R<()> {
    for (k, want) in field(op, "refs")?.as_object().ok_or("refs must be an object")? {
        let t = *w
            .labels
            .borrow()
            .get(k)
            .ok_or_else(|| format!("unknown object '{k}'"))?;
        let got = w.refs.borrow().get(&t).copied().unwrap_or(0);
        if got != i64_of(want, k)? {
            return Err(format!("{k}: {got} references"));
        }
    }
    Ok(())
}

// ---- persistence ----------------------------------------------------------------------------------

fn slot(op: &Json) -> R<String> {
    Ok(str_of(field(op, "slot")?, "slot")?.to_owned())
}

fn save_now(w: &World) -> R<SavedCity> {
    let s = w.sim()?;
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut root = ByteWriter::with_registry(reg.clone());
    root.put_ref(&RefValue::Object(s as Rc<dyn SaveObject>));
    let mut pkg = Package::new();
    let rep = save_store(&reg, &mut pkg);
    if rep.aborted || rep.entries_discarded != 0 {
        return Err(format!("save failed: {rep:?}"));
    }
    let hosts = reg
        .borrow()
        .entries
        .iter()
        .filter(|e| e.2.is_none())
        .map(|e| (e.1, e.3))
        .collect();
    Ok(SavedCity {
        pkg,
        root: root.into_bytes(),
        hosts,
    })
}

fn save(w: &World, op: &Json) -> R<()> {
    let s = save_now(w)?;
    w.saves.borrow_mut().insert(slot(op)?, s);
    Ok(())
}

/// `plumbing_load`: `slot`, `expect?` (a clean store load, default true).
/// The loaded simulator replaces the current one.
fn load(w: &World, op: &Json) -> R<()> {
    let saved = w.saves.borrow().get(&slot(op)?).cloned().ok_or("no such slot")?;
    let me = w.rc();
    let mut factory = |class: u32, object_id: u32| -> Option<RefValue> {
        match class {
            CLSID_PLUMBING_SIMULATOR => {
                let t = me.token("plumbing");
                Some(RefValue::Object(
                    PlumbingSim::new(me.clone() as Rc<dyn PlumbingHost>, t) as Rc<dyn SaveObject>,
                ))
            }
            OCCUPANT_CLASS | DEPARTMENT_CLASS => {
                // The same game object comes back with a reference for the reader.
                let token = *saved.hosts.get(&object_id)?;
                me.add_ref(token);
                Some(RefValue::Host { token, class_id: class })
            }
            c => create_for_clsid(c).map(RefValue::Object),
        }
    };
    let mut reg = LoadRegistry::new(BTreeMap::new(), Box::new(&mut factory));
    let report = load_store(&saved.pkg, &mut reg);
    let ok = report.index_ok && report.failed_classes.is_empty();
    let mut r = ByteReader::with_resolver(&saved.root, &mut reg);
    let root = r.get_ref();
    let want = opt(op, "expect").map_or(Ok(true), |v| bool_of(v, "expect"))?;
    if ok != want {
        return Err(format!("store load clean = {ok}, expected {want}"));
    }
    if let Some(sim) = root.as_ref().and_then(|v| v.object::<PlumbingSim>()) {
        *w.sim.borrow_mut() = Some(sim);
    } else if want {
        return Err("the root did not resolve to a plumbing simulator".into());
    }
    Ok(())
}

fn expect_save(w: &World, op: &Json) -> R<()> {
    let want = w.saves.borrow().get(&slot(op)?).cloned().ok_or("no such slot")?;
    let got = save_now(w)?;
    scalar::same_save(
        &scalar::Saved {
            pkg: got.pkg,
            root: got.root,
        },
        &scalar::Saved {
            pkg: want.pkg,
            root: want.root,
        },
    )
}

fn patch(w: &World, op: &Json) -> R<()> {
    let mut saves = w.saves.borrow_mut();
    let s = saves.get_mut(&slot(op)?).ok_or("no such slot")?;
    let mut tmp = scalar::Saved {
        pkg: std::mem::take(&mut s.pkg),
        root: Vec::new(),
    };
    let r = scalar::patch(&mut tmp, CLSID_PLUMBING_SIMULATOR, op);
    s.pkg = tmp.pkg;
    r
}

/// `plumbing_expect_payload`: `slot`, `length?`, `hex?`, `at? [{offset,
/// hex}]`.
fn expect_payload(w: &World, op: &Json) -> R<()> {
    let saves = w.saves.borrow();
    let s = saves.get(&slot(op)?).ok_or("no such slot")?;
    let tmp = scalar::Saved {
        pkg: s.pkg.clone(),
        root: Vec::new(),
    };
    let got = scalar::entry_payload(&tmp, CLSID_PLUMBING_SIMULATOR, 0)?;
    if let Some(h) = opt(op, "hex") {
        let want = hex_decode(str_of(h, "hex")?)?;
        if got != want {
            return Err(format!("payload differs:\n    got      {}", hex_encode(&got)));
        }
    }
    if let Some(v) = opt(op, "at") {
        for e in arr(v, "at")? {
            let off = i64_of(field(e, "offset")?, "offset")? as usize;
            let want = hex_decode(str_of(field(e, "hex")?, "hex")?)?;
            let slice = got.get(off..off + want.len()).ok_or("slice outside the payload")?;
            if slice != want.as_slice() {
                return Err(format!(
                    "payload at {off}: got {}, expected {}",
                    hex_encode(slice),
                    hex_encode(&want)
                ));
            }
        }
    }
    if let Some(v) = opt(op, "length") {
        if got.len() as i64 != i64_of(v, "length")? {
            return Err(format!("payload length {}", got.len()));
        }
    }
    Ok(())
}
