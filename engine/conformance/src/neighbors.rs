// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops `neighbors_…` for `fc4_core::neighbors`
//! (`docs/specs/neighbors.md`): a scripted region (regional cities with
//! positions, sizes, statistics, budgets, their own connections and local
//! deals), the city's figures, terrain heights, marker props and occupants,
//! all behind [`NeighborsHost`]; an event log of the host calls; a queue of
//! the status reports the simulator posts (delivered by `neighbors_deliver`;
//! sent reports are delivered at once unless `deliver_sent` is false); and
//! ops that drive the simulator, its deals and connections. Deals are named
//! by integer IDs: the vector's own (`neighbors_deal`), or 1000, 1001, … in
//! the order the runner first sees a deal the core created. See README.md
//! ("Neighbours ops").

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

use fc4_core::curve::ResponseCurve;
use fc4_core::neighbors::{
    conn_type, connection_inner_cell, deal_cost, deal_max_amount, deal_type, inner_cell, is_valid_connection_cell, msg,
    neighbour_connection_cell, neighbouring_region_cell, shipped_tuning, side_of, CityHandle, CityId, CommodityStats,
    ConnectionRequest, DealData, ForeignConnection, NeighborConnection, NeighborDeal, NeighborsHost, NeighborsMessage,
    NeighborsSimulator, PriceInputs, Side, Stat, UtilityFigures, CLSID_NEIGHBORS_SIMULATOR, CLSID_NEIGHBOR_CONNECTION,
    DEAL_PRICE_CURVE, GARBAGE_COST_CURVE,
};
use fc4_core::property::PropertyHolder;
use fc4_core::stream::{ByteReader, ByteWriter, RefResolver, RefValue, SaveObject};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arr, bool_of, check_expect, check_state, f32_of, holder_of, i64_of, jf, ji, opt, show, str_of, u32_of,
    EventLog, Saved, R,
};

/// Cells are 16 m (`CellCenterToPosition` = `16·x + 8`).
const CELL: f32 = 16.0;
/// The first ID given to a deal the core created.
const FIRST_AUTO_ID: i64 = 1000;

const STATS: [(&str, Stat); 5] = [
    ("exported", Stat::Exported),
    ("imported", Stat::Imported),
    ("produced", Stat::Produced),
    ("demanded", Stat::Demanded),
    ("cost_per_unit", Stat::CostPerUnit),
];

// ---- world -------------------------------------------------------------------------------------

#[derive(Clone, Default)]
struct City {
    pos: (i32, i32),
    size: (i32, i32),
    established: bool,
    budget: f64,
    income: f64,
    /// Per statistic, commodities 1..=3.
    stats: BTreeMap<&'static str, [f64; 3]>,
    /// The city's own connections: (type, cell in that city, active).
    conns: Vec<(u32, (i32, i32), bool)>,
    /// `GetAllConnections` as (type, other-side region cell).
    links: Vec<(u32, (i32, i32))>,
    local: Vec<DealData>,
}

#[derive(Clone, Default)]
struct Occ {
    kind: u32,
    networks: Vec<u32>,
    cell: (i32, i32),
    pole_flag: bool,
}

pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    sim: RefCell<Option<Rc<NeighborsSimulator>>>,
    next_token: Cell<u64>,
    session: Cell<u32>,
    cities: RefCell<BTreeMap<CityId, City>>,
    current: Cell<Option<CityId>>,
    /// Whether the current city object ([`CITY_OBJECT`]) exists (§8.2, NB12).
    city_object: Cell<bool>,
    cells: Cell<(i32, i32)>,
    today: Cell<u32>,
    figures: RefCell<UtilityFigures>,
    landfill: Cell<(u32, u32)>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    props: RefCell<BTreeMap<u64, [f32; 3]>>,
    next_prop: Cell<u64>,
    occupants: RefCell<BTreeMap<u64, Occ>>,
    altitude: Cell<f32>,
    /// `GetAltitude(px, pz)` at given world positions (§6.3, NB3); any
    /// other position reads `altitude`.
    altitude_at: RefCell<Vec<([f32; 2], f32)>>,
    surface: Cell<f32>,
    deliver_sent: Cell<bool>,
    /// Posted status reports `(deal, data1)`, delivered by `neighbors_deliver`.
    posted: RefCell<Vec<(Option<Rc<NeighborDeal>>, u32)>>,
    /// Deal IDs.
    deals: RefCell<Vec<(i64, Rc<NeighborDeal>)>>,
    next_id: Cell<i64>,
    saves: RefCell<BTreeMap<String, Saved>>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            me: RefCell::new(Weak::new()),
            events: EventLog::default(),
            sim: RefCell::new(None),
            next_token: Cell::new(0x51),
            session: Cell::new(0x1234_5678),
            cities: RefCell::new(BTreeMap::new()),
            current: Cell::new(None),
            city_object: Cell::new(true),
            cells: Cell::new((64, 64)),
            today: Cell::new(1000),
            figures: RefCell::new(UtilityFigures::default()),
            landfill: Cell::new((0, 0)),
            tuning: RefCell::new(Some(Rc::new(shipped_tuning()))),
            props: RefCell::new(BTreeMap::new()),
            next_prop: Cell::new(0x100),
            occupants: RefCell::new(BTreeMap::new()),
            altitude: Cell::new(270.0),
            altitude_at: RefCell::new(Vec::new()),
            surface: Cell::new(280.0),
            deliver_sent: Cell::new(true),
            posted: RefCell::new(Vec::new()),
            deals: RefCell::new(Vec::new()),
            next_id: Cell::new(FIRST_AUTO_ID),
            saves: RefCell::new(BTreeMap::new()),
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> R<Rc<World>> {
        self.me.borrow().upgrade().ok_or_else(|| "world dropped".into())
    }

    fn sim(&self) -> R<Rc<NeighborsSimulator>> {
        self.sim
            .borrow()
            .clone()
            .ok_or_else(|| "no simulator (neighbors_create first)".into())
    }

    fn with_city<T>(&self, id: CityId, f: impl FnOnce(&mut City) -> T) -> Option<T> {
        self.cities.borrow_mut().get_mut(&id).map(f)
    }

    fn city_pos(&self, id: CityId) -> R<(i32, i32)> {
        self.with_city(id, |c| c.pos).ok_or_else(|| format!("no city {id}"))
    }

    /// The statistic's name and commodity index (1..=3 → 0..=2).
    fn stat_slot(stat: Stat, commodity: i32) -> Option<(usize, &'static str)> {
        let name = STATS.iter().find(|(_, s)| *s == stat)?.0;
        let i = usize::try_from(commodity - 1).ok().filter(|&i| i < 3)?;
        Some((i, name))
    }

    /// The deal's ID, assigning the next automatic one on first sight.
    fn deal_id(&self, d: &Rc<NeighborDeal>) -> i64 {
        if let Some((id, _)) = self.deals.borrow().iter().find(|(_, x)| Rc::ptr_eq(x, d)) {
            return *id;
        }
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        self.deals.borrow_mut().push((id, d.clone()));
        id
    }

    fn register(&self, id: i64, d: &Rc<NeighborDeal>) {
        let mut deals = self.deals.borrow_mut();
        deals.retain(|(i, x)| *i != id && !Rc::ptr_eq(x, d));
        deals.push((id, d.clone()));
    }

    fn deal(&self, id: i64) -> R<Rc<NeighborDeal>> {
        self.deals
            .borrow()
            .iter()
            .find(|(i, _)| *i == id)
            .map(|(_, d)| d.clone())
            .ok_or_else(|| format!("no deal {id}"))
    }

    fn ids(&self, deals: &[Rc<NeighborDeal>]) -> Json {
        Json::Array(deals.iter().map(|d| ji(self.deal_id(d))).collect())
    }

    fn cell_of(pos: [f32; 3]) -> (i32, i32) {
        ((pos[0] / CELL).floor() as i32, (pos[2] / CELL).floor() as i32)
    }
}

fn opt_city(c: Option<CityId>) -> Json {
    c.map_or(Json::Null, |c| ji(c as i64))
}

impl NeighborsHost for World {
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        self.events.push(
            "add_agent",
            vec![
                ("level", ji(level)),
                ("name", Json::Str(name.into())),
                ("flags", ji(flags)),
            ],
        );
    }
    fn remove_agents(&self) {
        self.events.push("remove_agents", vec![]);
    }
    fn subscribe(&self, msg_type: u32) {
        self.events.push("subscribe", vec![("msg", ji(msg_type))]);
    }
    fn unsubscribe(&self, msg_type: u32) {
        self.events.push("unsubscribe", vec![("msg", ji(msg_type))]);
    }
    fn cell_counts(&self) -> (i32, i32) {
        self.cells.get()
    }
    fn today(&self) -> u32 {
        self.today.get()
    }
    fn utility_figures(&self) -> UtilityFigures {
        *self.figures.borrow()
    }
    fn landfill_usage(&self) -> (u32, u32) {
        self.landfill.get()
    }
    fn send_history(&self, series: u32, value: i32) {
        self.events
            .push("history", vec![("series", ji(series)), ("value", ji(value))]);
    }
    fn current_city(&self) -> Option<CityId> {
        self.current.get()
    }
    fn current_city_object(&self) -> Option<CityHandle> {
        self.city_object.get().then_some(CITY_OBJECT)
    }
    fn city_at(&self, rx: i32, rz: i32) -> Option<CityId> {
        self.cities
            .borrow()
            .iter()
            .find(|(_, c)| rx >= c.pos.0 && rx < c.pos.0 + c.size.0 && rz >= c.pos.1 && rz < c.pos.1 + c.size.1)
            .map(|(&id, _)| id)
    }
    fn city_position(&self, city: CityId) -> (i32, i32) {
        self.with_city(city, |c| c.pos).unwrap_or_default()
    }
    fn city_size(&self, city: CityId) -> (i32, i32) {
        self.with_city(city, |c| c.size).unwrap_or_default()
    }
    fn city_established(&self, city: CityId) -> bool {
        self.with_city(city, |c| c.established).unwrap_or(false)
    }
    fn city_stat(&self, city: CityId, stat: Stat, commodity: i32) -> f64 {
        let Some((i, name)) = World::stat_slot(stat, commodity) else {
            return 0.0;
        };
        self.with_city(city, |c| c.stats.get(name).map_or(0.0, |v| v[i]))
            .unwrap_or(0.0)
    }
    fn set_city_stat(&self, city: CityId, stat: Stat, commodity: i32, value: f64) {
        if let Some((i, name)) = World::stat_slot(stat, commodity) {
            self.with_city(city, |c| c.stats.entry(name).or_insert([0.0; 3])[i] = value);
        }
    }
    fn city_budget(&self, city: CityId) -> f64 {
        self.with_city(city, |c| c.budget).unwrap_or(0.0)
    }
    fn set_city_budget(&self, city: CityId, value: f64) {
        self.with_city(city, |c| c.budget = value);
    }
    fn set_city_income(&self, city: CityId, value: f64) {
        self.with_city(city, |c| c.income = value);
    }
    fn city_find_connection(&self, city: CityId, t: u32, x: i32, z: i32) -> Option<ForeignConnection> {
        self.with_city(city, |c| {
            c.conns
                .iter()
                .position(|&(ct, cell, _)| ct == t && cell == (x, z))
                .map(|i| ForeignConnection {
                    token: 0xF000 + i as u64,
                    active: c.conns[i].2,
                })
        })
        .flatten()
    }
    fn city_connections(&self, city: CityId) -> Vec<(u32, (i32, i32))> {
        self.with_city(city, |c| c.links.clone()).unwrap_or_default()
    }
    fn change_symmetric_connection(&self, city: CityId, conn: &Rc<NeighborConnection>, add: bool) {
        let (x, z) = conn.cell();
        self.events.push(
            "symmetric",
            vec![
                ("city", ji(city as i64)),
                ("type", ji(conn.conn_type())),
                ("x", ji(x)),
                ("z", ji(z)),
                ("add", Json::Bool(add)),
            ],
        );
    }
    fn reload_city(&self, city: CityId) {
        self.events.push("reload_city", vec![("city", ji(city as i64))]);
    }
    fn update_local_deals(&self, city: CityId) {
        self.events.push("update_local_deals", vec![("city", ji(city as i64))]);
    }
    fn local_deals(&self, city: CityId) -> Vec<DealData> {
        self.with_city(city, |c| c.local.clone()).unwrap_or_default()
    }
    fn set_local_deals(&self, city: CityId, deals: &mut Vec<Rc<NeighborDeal>>) {
        // foundations §11.3 [quirk]: deals with a missing region city leave
        // the caller's list.
        deals.retain(|d| {
            let (ox, oz) = d.other_side();
            let (cx, cz) = d.city_side();
            self.city_at(ox, oz).is_some() && self.city_at(cx, cz).is_some()
        });
        let ids = self.ids(deals);
        self.events
            .push("set_local_deals", vec![("city", ji(city as i64)), ("deals", ids)]);
    }
    fn update_import_export(&self, city: CityId) {
        self.events
            .push("update_import_export", vec![("city", ji(city as i64))]);
    }
    fn post_status(&self, deal: Option<&Rc<NeighborDeal>>, status: u32) {
        let id = deal.map_or(Json::Null, |d| ji(self.deal_id(d)));
        let flags = deal.map_or(0, |d| d.flags());
        self.events.push(
            "status",
            vec![
                ("deal", id),
                ("status", ji(status)),
                ("flags", ji(flags)),
                ("sent", Json::Bool(false)),
            ],
        );
        self.posted.borrow_mut().push((deal.cloned(), status));
    }
    fn send_status(&self, deal: &Rc<NeighborDeal>, status: u32) {
        self.events.push(
            "status",
            vec![
                ("deal", ji(self.deal_id(deal))),
                ("status", ji(status)),
                ("flags", ji(deal.flags())),
                ("sent", Json::Bool(true)),
            ],
        );
        if !self.deliver_sent.get() {
            return;
        }
        let sim = self.sim.borrow().clone();
        if let Some(sim) = sim {
            sim.do_message(&NeighborsMessage {
                msg_type: msg::DEAL_STATUS,
                data1: status,
                ..NeighborsMessage::default()
            });
        }
    }
    fn post_connection_message(&self, msg_type: u32, t: u32, x: i32, z: i32) {
        let kind = if msg_type == msg::CONNECTION_CREATED {
            "connection_created"
        } else {
            "connection_deleted"
        };
        self.events
            .push(kind, vec![("type", ji(t)), ("x", ji(x)), ("z", ji(z))]);
    }
    fn cell_center(&self, x: i32, z: i32) -> (f32, f32) {
        (x as f32 * CELL + 8.0, z as f32 * CELL + 8.0)
    }
    fn marker_altitude(&self, px: f32, pz: f32) -> f32 {
        let at = self.altitude_at.borrow();
        at.iter()
            .find(|(p, _)| *p == [px, pz])
            .map_or(self.altitude.get(), |&(_, y)| y)
    }
    fn max_surface_level(&self, _x: i32, _z: i32) -> f32 {
        self.surface.get()
    }
    fn add_marker_prop(&self, position: [f32; 3], orientation: i32, bool_props: &[u32]) -> Option<u64> {
        let token = self.next_prop.get();
        self.next_prop.set(token + 1);
        self.props.borrow_mut().insert(token, position);
        self.events.push(
            "prop_add",
            vec![
                ("prop", ji(token as i64)),
                ("pos", Json::Array(position.iter().map(|&v| jf(v)).collect())),
                ("orientation", ji(orientation)),
                ("props", Json::Array(bool_props.iter().map(|&p| ji(p)).collect())),
            ],
        );
        Some(token)
    }
    fn marker_props(&self, x: i32, z: i32) -> Vec<u64> {
        self.props
            .borrow()
            .iter()
            .filter(|(_, p)| World::cell_of(**p) == (x, z))
            .map(|(&t, _)| t)
            .collect()
    }
    fn remove_prop(&self, prop: u64) {
        self.props.borrow_mut().remove(&prop);
        self.events.push("prop_remove", vec![("prop", ji(prop as i64))]);
    }
    fn move_prop(&self, prop: u64, position: [f32; 3]) {
        if let Some(p) = self.props.borrow_mut().get_mut(&prop) {
            *p = position;
        }
        self.events.push(
            "prop_move",
            vec![
                ("prop", ji(prop as i64)),
                ("pos", Json::Array(position.iter().map(|&v| jf(v)).collect())),
            ],
        );
    }
    fn occupant_type(&self, occupant: u64) -> u32 {
        self.occupants.borrow().get(&occupant).map_or(0, |o| o.kind)
    }
    fn occupant_networks(&self, occupant: u64) -> Vec<u32> {
        self.occupants
            .borrow()
            .get(&occupant)
            .map_or(Vec::new(), |o| o.networks.clone())
    }
    fn occupant_cell(&self, occupant: u64) -> (i32, i32) {
        self.occupants.borrow().get(&occupant).map_or((0, 0), |o| o.cell)
    }
    fn power_pole_flag(&self, occupant: u64) -> bool {
        self.occupants.borrow().get(&occupant).is_some_and(|o| o.pole_flag)
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "neighbors", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("neighbors_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "city" => city(&w, op),
        "occupant" => occupant(&w, op),
        "create" => create(&w, op),
        "message" => message(&w, op),
        "month" => month(&w, op),
        "deliver" => deliver(&w, op),
        "deal" => deal(&w, op),
        "deal_set" => deal_set(&w, op),
        "deal_call" => deal_call(&w, op),
        "connection_set" => connection_set(&w, op),
        "call" => call(&w, op),
        "geometry" => geometry(op),
        "price" => price(op),
        "expect" => expect(&w, op),
        "record" => record(&w, op),
        "deal_record" => deal_record(&w, op),
        "connection_record" => connection_record(&w, op),
        "save" => save(&w, op),
        "load" => load(&w, op),
        "expect_save" => expect_save(&w, op),
        "patch" => patch(&w, op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        _ => Err(format!("unknown op '{name}'")),
    })
}

// ---- argument helpers ---------------------------------------------------------------------------------

fn pair(j: &Json, what: &str) -> R<(i32, i32)> {
    let a = arr(j, what)?;
    if a.len() != 2 {
        return Err(format!("'{what}' must be [x, z]"));
    }
    Ok((i64_of(&a[0], what)? as i32, i64_of(&a[1], what)? as i32))
}

fn jpair(p: (i32, i32)) -> Json {
    Json::Array(vec![ji(p.0), ji(p.1)])
}

fn num(j: &Json, what: &str) -> R<f64> {
    scalar::num(j).ok_or_else(|| format!("'{what}' must be a number"))
}

/// A deal type: a number, or `"E0"` … `"E5"` (§3).
fn deal_type_of(j: &Json) -> R<u32> {
    if let Some(s) = j.as_str() {
        if let Some(i) = s.strip_prefix('E').and_then(|d| d.parse::<u32>().ok()) {
            if i < 6 {
                return Ok(deal_type::BUY_POWER + i);
            }
        }
    }
    u32_of(j, "deal type")
}

/// A connection type: a number or the SDK name in lower case.
fn conn_type_of(j: &Json) -> R<u32> {
    if let Some(s) = j.as_str() {
        let t = match s {
            "rail" => conn_type::RAIL,
            "road" => conn_type::ROAD,
            "highway" => conn_type::HIGHWAY,
            "power_pole" => conn_type::POWER_POLE,
            "pipe" => conn_type::PIPE,
            "subway" => conn_type::SUBWAY,
            "light_rail" => conn_type::LIGHT_RAIL,
            "avenue" => conn_type::AVENUE,
            "ferry" => conn_type::FERRY,
            "dirt_road" => conn_type::DIRT_ROAD,
            "monorail" => conn_type::MONORAIL,
            "ground_highway" => conn_type::GROUND_HIGHWAY,
            _ => return u32_of(j, "connection type"),
        };
        return Ok(t);
    }
    u32_of(j, "connection type")
}

/// `[type, x, z]`.
fn conn_key(j: &Json) -> R<(u32, i32, i32)> {
    let a = arr(j, "connection")?;
    if a.len() != 3 {
        return Err("a connection is [type, x, z]".into());
    }
    Ok((
        conn_type_of(&a[0])?,
        i64_of(&a[1], "x")? as i32,
        i64_of(&a[2], "z")? as i32,
    ))
}

fn jconn(c: &NeighborConnection) -> Json {
    let (x, z) = c.cell();
    Json::Array(vec![ji(c.conn_type()), ji(x), ji(z)])
}

fn city_of(j: &Json) -> R<Option<CityId>> {
    match j {
        Json::Null => Ok(None),
        v => Ok(Some(i64_of(v, "city")? as CityId)),
    }
}

/// The city object token of the current city (`cISC4City`, §8.2 NB12),
/// distinct from every regional city ID a vector uses.
const CITY_OBJECT: CityHandle = 0xC17_0000;

/// A city object argument (§8.2, NB12): `"city"` is the current city object
/// ([`CITY_OBJECT`]), a number any other handle (a regional city's ID
/// included: a different object), null none.
fn city_object_of(j: &Json) -> R<Option<CityHandle>> {
    match j {
        Json::Null => Ok(None),
        Json::Str(s) if s == "city" => Ok(Some(CITY_OBJECT)),
        v => Ok(Some(i64_of(v, "city object")? as CityHandle)),
    }
}

/// A deal spec merged into `d`: `type`, `other` (a city: its position) or
/// `other_side`, `city` or `city_side`, the amounts, `cost`, `flags`,
/// `start`, `expiration`, `current`, `suspension`.
fn deal_data(w: &World, j: &Json, mut d: DealData) -> R<DealData> {
    if let Some(v) = opt(j, "type") {
        d.deal_type = deal_type_of(v)?;
    }
    if let Some(v) = opt(j, "other") {
        d.other_side = w.city_pos(i64_of(v, "other")? as CityId)?;
    }
    if let Some(v) = opt(j, "other_side") {
        d.other_side = pair(v, "other_side")?;
    }
    if let Some(v) = opt(j, "city") {
        d.city_side = w.city_pos(i64_of(v, "city")? as CityId)?;
    }
    if let Some(v) = opt(j, "city_side") {
        d.city_side = pair(v, "city_side")?;
    }
    for (k, slot) in [
        ("amount", &mut d.amount),
        ("inc_amount", &mut d.inc_amount),
        ("inc_amount2", &mut d.inc_amount2),
        ("last_amount", &mut d.last_amount),
        ("last_inc_amount", &mut d.last_inc_amount),
        ("amount_used", &mut d.amount_used),
    ] {
        if let Some(v) = opt(j, k) {
            *slot = i64_of(v, k)? as i32;
        }
    }
    if let Some(v) = opt(j, "cost") {
        d.cost = i64_of(v, "cost")?;
    }
    for (k, slot) in [
        ("flags", &mut d.flags),
        ("start", &mut d.start_date),
        ("expiration", &mut d.expiration_date),
        ("current", &mut d.current_date),
        ("suspension", &mut d.suspension_date),
    ] {
        if let Some(v) = opt(j, k) {
            *slot = u32_of(v, k)?;
        }
    }
    if let Some(v) = opt(j, "properties") {
        d.props = holder_of(v)?;
    }
    Ok(d)
}

fn deal_state(w: &World, d: &Rc<NeighborDeal>) -> Vec<(String, Json)> {
    let x = d.data();
    let in_list = w
        .sim()
        .map(|s| s.find_deals_all().iter().any(|y| Rc::ptr_eq(y, d)))
        .unwrap_or(false);
    vec![
        ("type".into(), ji(x.deal_type)),
        ("other_side".into(), jpair(x.other_side)),
        ("city_side".into(), jpair(x.city_side)),
        ("amount".into(), ji(x.amount)),
        ("inc_amount".into(), ji(x.inc_amount)),
        ("inc_amount2".into(), ji(x.inc_amount2)),
        ("last_amount".into(), ji(x.last_amount)),
        ("last_inc_amount".into(), ji(x.last_inc_amount)),
        ("amount_used".into(), ji(x.amount_used)),
        ("cost".into(), ji(x.cost)),
        ("flags".into(), ji(x.flags)),
        ("start".into(), ji(x.start_date)),
        ("expiration".into(), ji(x.expiration_date)),
        ("current".into(), ji(x.current_date)),
        ("suspension".into(), ji(x.suspension_date)),
        ("in_list".into(), Json::Bool(in_list)),
        ("other_city".into(), opt_city(d.other_city())),
    ]
}

fn connection_state(c: &NeighborConnection) -> Vec<(String, Json)> {
    vec![
        ("type".into(), ji(c.conn_type())),
        ("cell".into(), jpair(c.cell())),
        ("other_region_cell".into(), jpair(c.other_region_cell())),
        ("active".into(), Json::Bool(c.is_active())),
        ("visited".into(), Json::Bool(c.visited_this_cycle())),
        ("flags".into(), ji(c.flags())),
        ("needs_traffic_sync".into(), Json::Bool(c.needs_traffic_sync())),
    ]
}

// ---- set-up ops -----------------------------------------------------------------------------------------

/// `neighbors_env`: `cells` ([w, h]), `today`, `current` (a city ID or
/// null), `session`, `tuning` ("shipped", null, or a property map),
/// `tuning_remove` ([IDs] removed from the current tuning), `figures`
/// (fields of `UtilityFigures`; department expenses may be null), `landfill`
/// ([usage, capacity]), `altitude`, `altitude_at` ([[px, pz, y]]: the
/// terrain altitude at those world positions), `surface`, `deliver_sent`,
/// `city_object` (whether the current city object exists, default true).
fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "cells") {
        w.cells.set(pair(v, "cells")?);
    }
    if let Some(v) = opt(op, "today") {
        w.today.set(u32_of(v, "today")?);
    }
    if let Some(v) = op.get("current") {
        w.current.set(city_of(v)?);
    }
    if let Some(v) = opt(op, "session") {
        w.session.set(u32_of(v, "session")?);
    }
    if let Some(v) = op.get("tuning") {
        *w.tuning.borrow_mut() = match v {
            Json::Null => None,
            Json::Str(s) if s == "shipped" => Some(Rc::new(shipped_tuning())),
            other => Some(Rc::new(holder_of(other)?)),
        };
    }
    if let Some(v) = opt(op, "tuning_remove") {
        let mut h = w.tuning.borrow().as_deref().cloned().unwrap_or_default();
        for id in arr(v, "tuning_remove")? {
            h.remove(u32_of(id, "tuning_remove")?);
        }
        *w.tuning.borrow_mut() = Some(Rc::new(h));
    }
    if let Some(v) = opt(op, "figures") {
        let mut f = w.figures.borrow_mut();
        for (k, val) in v.as_object().ok_or("'figures' must be an object")? {
            let opt_num = |val: &Json| -> R<Option<f64>> {
                match val {
                    Json::Null => Ok(None),
                    v => Ok(Some(num(v, k)?)),
                }
            };
            match k.as_str() {
                "power_department_expenses" => f.power_department_expenses = opt_num(val)?,
                "water_department_expenses" => f.water_department_expenses = opt_num(val)?,
                "sanitation_department_expenses" => f.sanitation_department_expenses = opt_num(val)?,
                _ => {
                    let slot = match k.as_str() {
                        "total_funds" => &mut f.total_funds,
                        "monthly_income" => &mut f.monthly_income,
                        "power_production" => &mut f.power_production,
                        "power_requirement" => &mut f.power_requirement,
                        "water_capacity" => &mut f.water_capacity,
                        "water_demand" => &mut f.water_demand,
                        "garbage_recycled" => &mut f.garbage_recycled,
                        "waste_to_energy_capacity" => &mut f.waste_to_energy_capacity,
                        "incinerator_capacity" => &mut f.incinerator_capacity,
                        "garbage_abducted" => &mut f.garbage_abducted,
                        "garbage_produced" => &mut f.garbage_produced,
                        "waste_to_energy_expenses" => &mut f.waste_to_energy_expenses,
                        other => return Err(format!("unknown figure '{other}'")),
                    };
                    *slot = num(val, k)?;
                }
            }
        }
    }
    if let Some(v) = opt(op, "landfill") {
        let (u, c) = pair(v, "landfill")?;
        w.landfill.set((u as u32, c as u32));
    }
    if let Some(v) = opt(op, "altitude") {
        w.altitude.set(f32_of(v, "altitude")?);
    }
    if let Some(v) = opt(op, "altitude_at") {
        let mut at = Vec::new();
        for e in arr(v, "altitude_at")? {
            let e = arr(e, "altitude_at entry")?;
            if e.len() != 3 {
                return Err("altitude_at entries are [px, pz, y]".to_string());
            }
            at.push(([f32_of(&e[0], "px")?, f32_of(&e[1], "pz")?], f32_of(&e[2], "y")?));
        }
        *w.altitude_at.borrow_mut() = at;
    }
    if let Some(v) = opt(op, "surface") {
        w.surface.set(f32_of(v, "surface")?);
    }
    if let Some(v) = opt(op, "deliver_sent") {
        w.deliver_sent.set(bool_of(v, "deliver_sent")?);
    }
    if let Some(v) = opt(op, "city_object") {
        w.city_object.set(bool_of(v, "city_object")?);
    }
    Ok(())
}

/// `neighbors_city`: adds or updates regional city `id`: `pos`, `size`,
/// `established` (default true for a new city), `budget`, `income`, `stats`
/// ({name: [power, water, garbage]}), `conns` ([[type, x, z, active], …],
/// the city's own connections), `links` ([[type, rx, rz], …], its
/// `GetAllConnections`), `local_deals` ([deal specs]).
fn city(w: &World, op: &Json) -> R<()> {
    let id = i64_of(field(op, "id")?, "id")? as CityId;
    let mut c = w.with_city(id, |c| c.clone()).unwrap_or(City {
        size: (1, 1),
        established: true,
        ..City::default()
    });
    if let Some(v) = opt(op, "pos") {
        c.pos = pair(v, "pos")?;
    }
    if let Some(v) = opt(op, "size") {
        c.size = pair(v, "size")?;
    }
    if let Some(v) = opt(op, "established") {
        c.established = bool_of(v, "established")?;
    }
    if let Some(v) = opt(op, "budget") {
        c.budget = num(v, "budget")?;
    }
    if let Some(v) = opt(op, "income") {
        c.income = num(v, "income")?;
    }
    if let Some(v) = opt(op, "stats") {
        for (k, vals) in v.as_object().ok_or("'stats' must be an object")? {
            let name = STATS
                .iter()
                .find(|(n, _)| n == k)
                .ok_or_else(|| format!("unknown statistic '{k}'"))?
                .0;
            let a = arr(vals, k)?;
            if a.len() != 3 {
                return Err(format!("'{k}' must be [power, water, garbage]"));
            }
            c.stats.insert(name, [num(&a[0], k)?, num(&a[1], k)?, num(&a[2], k)?]);
        }
    }
    if let Some(v) = opt(op, "conns") {
        c.conns.clear();
        for e in arr(v, "conns")? {
            let a = arr(e, "conns")?;
            if a.len() != 4 {
                return Err("a city connection is [type, x, z, active]".into());
            }
            c.conns.push((
                conn_type_of(&a[0])?,
                (i64_of(&a[1], "x")? as i32, i64_of(&a[2], "z")? as i32),
                bool_of(&a[3], "active")?,
            ));
        }
    }
    if let Some(v) = opt(op, "links") {
        c.links.clear();
        for e in arr(v, "links")? {
            let a = arr(e, "links")?;
            if a.len() != 3 {
                return Err("a link is [type, rx, rz]".into());
            }
            c.links.push((
                conn_type_of(&a[0])?,
                (i64_of(&a[1], "rx")? as i32, i64_of(&a[2], "rz")? as i32),
            ));
        }
    }
    let local = match opt(op, "local_deals") {
        Some(v) => Some(arr(v, "local_deals")?.to_vec()),
        None => None,
    };
    w.cities.borrow_mut().insert(id, c);
    if let Some(local) = local {
        let mut out = Vec::new();
        for d in &local {
            out.push(deal_data(w, d, DealData::default())?);
        }
        w.with_city(id, |c| c.local = out);
    }
    Ok(())
}

/// `neighbors_occupant`: `id`, `kind` ("network", "power_pole" or a number),
/// `networks`, `cell`, `pole_flag`.
fn occupant(w: &World, op: &Json) -> R<()> {
    let id = i64_of(field(op, "id")?, "id")? as u64;
    let kind = match field(op, "kind")? {
        Json::Str(s) if s == "network" => fc4_core::neighbors::occupant::NETWORK,
        Json::Str(s) if s == "power_pole" => fc4_core::neighbors::occupant::POWER_POLE,
        v => u32_of(v, "kind")?,
    };
    let networks = match opt(op, "networks") {
        Some(v) => arr(v, "networks")?
            .iter()
            .map(|n| u32_of(n, "networks"))
            .collect::<R<Vec<u32>>>()?,
        None => Vec::new(),
    };
    let cell = opt(op, "cell").map_or(Ok((0, 0)), |v| pair(v, "cell"))?;
    let pole_flag = opt(op, "pole_flag").map_or(Ok(false), |v| bool_of(v, "pole_flag"))?;
    w.occupants.borrow_mut().insert(
        id,
        Occ {
            kind,
            networks,
            cell,
            pole_flag,
        },
    );
    Ok(())
}

/// `neighbors_create`: a simulator (the session seed from `neighbors_env`);
/// `Init` unless `init` is false (`expect` = its result); ConnectionsReady
/// when `ready` is true.
fn create(w: &World, op: &Json) -> R<()> {
    let token = w.next_token.get();
    w.next_token.set(token + 1);
    let s = NeighborsSimulator::new(w.rc()? as Rc<dyn NeighborsHost>, token, w.session.get());
    *w.sim.borrow_mut() = Some(s.clone());
    if opt(op, "init").map_or(Ok(true), |v| bool_of(v, "init"))? {
        check_expect(op, Json::Bool(s.init()))?;
    }
    if opt(op, "ready").map_or(Ok(false), |v| bool_of(v, "ready"))? {
        s.do_message(&NeighborsMessage::of_type(msg::CONNECTIONS_READY));
    }
    Ok(())
}

/// `neighbors_message`: `type`, `data1`…`data3`, `occupant`; `expect` =
/// DoMessage's result.
fn message(w: &World, op: &Json) -> R<()> {
    let mut m = NeighborsMessage::of_type(u32_of(field(op, "type")?, "type")?);
    for (k, slot) in [
        ("data1", &mut m.data1),
        ("data2", &mut m.data2),
        ("data3", &mut m.data3),
    ] {
        if let Some(v) = opt(op, k) {
            *slot = u32_of(v, k)?;
        }
    }
    if let Some(v) = opt(op, "occupant") {
        m.occupant = Some(i64_of(v, "occupant")? as u64);
    }
    check_expect(op, Json::Bool(w.sim()?.do_message(&m)))
}

/// `neighbors_month`: sets `today` if given, then `count` SimNewMonth
/// messages (default 1).
fn month(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "today") {
        w.today.set(u32_of(v, "today")?);
    }
    let n = opt(op, "count").map_or(Ok(1), |v| i64_of(v, "count"))?;
    let s = w.sim()?;
    for _ in 0..n {
        s.do_message(&NeighborsMessage::of_type(msg::SIM_NEW_MONTH));
    }
    Ok(())
}

/// `neighbors_deliver`: delivers the posted status reports in order (the
/// reports posted meanwhile join the queue) until none are left or `max`
/// (default 1000) were delivered; `count` = how many.
fn deliver(w: &World, op: &Json) -> R<()> {
    let max = opt(op, "max").map_or(Ok(1000), |v| i64_of(v, "max"))?;
    let s = w.sim()?;
    let mut n = 0i64;
    while n < max {
        let next = {
            let mut q = w.posted.borrow_mut();
            if q.is_empty() {
                break;
            }
            q.remove(0)
        };
        s.do_message(&NeighborsMessage {
            msg_type: msg::DEAL_STATUS,
            data1: next.1,
            ..NeighborsMessage::default()
        });
        n += 1;
    }
    if let Some(v) = opt(op, "count") {
        let want = i64_of(v, "count")?;
        if want != n {
            return Err(format!("delivered {n}, expected {want}"));
        }
    }
    Ok(())
}

/// `neighbors_deal`: a deal with vector ID `id` from a deal spec (the city
/// side defaults to the current city); added to the simulator's list
/// without a push unless `adopt` is false.
fn deal(w: &World, op: &Json) -> R<()> {
    let id = i64_of(field(op, "id")?, "id")?;
    let base = DealData {
        city_side: w.current.get().map_or(Ok((0, 0)), |c| w.city_pos(c))?,
        ..DealData::default()
    };
    let d = NeighborDeal::from_data(deal_data(w, op, base)?);
    if opt(op, "adopt").map_or(Ok(true), |v| bool_of(v, "adopt"))? {
        w.sim()?.adopt_deal(&d);
    }
    w.register(id, &d);
    Ok(())
}

/// `neighbors_deal_set`: overwrites fields of deal `deal` (no reports).
fn deal_set(w: &World, op: &Json) -> R<()> {
    let d = w.deal(i64_of(field(op, "deal")?, "deal")?)?;
    d.set_data(deal_data(w, op, d.data())?);
    Ok(())
}

/// `neighbors_deal_call`: deal `deal`'s `method` with `args`; `expect` =
/// the result.
fn deal_call(w: &World, op: &Json) -> R<()> {
    let d = w.deal(i64_of(field(op, "deal")?, "deal")?)?;
    let arg = |i: usize| scalar::arg(op, i);
    let got = match str_of(field(op, "method")?, "method")? {
        "set_amount" => {
            d.set_amount(i64_of(arg(0)?, "amount")? as i32);
            Json::Null
        }
        "set_flags" => Json::Bool(d.set_flags(u32_of(arg(0)?, "flags")?, bool_of(arg(1)?, "on")?)),
        "cancel" => {
            d.cancel();
            Json::Null
        }
        "activate" => Json::Bool(d.activate()),
        "update_cost" => {
            d.update_cost();
            Json::Null
        }
        "update_state" => {
            d.update_state();
            Json::Null
        }
        "clone" => ji(w.deal_id(&d.clone_deal(bool_of(arg(0)?, "silent")?))),
        "check_flags_and" => Json::Bool(d.check_flags_and(u32_of(arg(0)?, "flags")?)),
        "check_flags_or" => Json::Bool(d.check_flags_or(u32_of(arg(0)?, "flags")?)),
        "other_city" => opt_city(d.other_city()),
        other => return Err(format!("unknown deal method '{other}'")),
    };
    check_expect(op, got)
}

/// `neighbors_connection_set`: `connection` ([type, x, z]) of ours:
/// `active`, `visited`.
fn connection_set(w: &World, op: &Json) -> R<()> {
    let (t, x, z) = conn_key(field(op, "connection")?)?;
    let c = w.sim()?.find_connection(t, x, z).ok_or("no such connection")?;
    if let Some(v) = opt(op, "active") {
        c.set_active(bool_of(v, "active")?);
    }
    if let Some(v) = opt(op, "visited") {
        c.set_visited_this_cycle(bool_of(v, "visited")?);
    }
    Ok(())
}

// ---- calls ----------------------------------------------------------------------------------------------

/// `neighbors_call`: simulator `method` with `args`; `expect` = the result.
fn call(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let arg = |i: usize| scalar::arg(op, i);
    let int = |i: usize| -> R<i32> { Ok(i64_of(arg(i)?, "argument")? as i32) };
    let conns = |v: Vec<Rc<NeighborConnection>>| Json::Array(v.iter().map(|c| jconn(c)).collect());
    let cities = |v: Vec<CityId>| Json::Array(v.into_iter().map(|c| ji(c as i64)).collect());
    let got = match str_of(field(op, "method")?, "method")? {
        "init" => Json::Bool(s.init()),
        "shutdown" => Json::Bool(s.shutdown()),
        "create_connection" => Json::Bool(
            s.create_connection(conn_type_of(arg(0)?)?, int(1)?, int(2)?, bool_of(arg(3)?, "flag")?)
                .is_some(),
        ),
        "request_create_connection" => match s.request_create_connection(conn_type_of(arg(0)?)?, int(1)?, int(2)?) {
            ConnectionRequest::Exists => Json::Str("exists".into()),
            ConnectionRequest::Confirm { neighbour } => Json::Object(vec![("confirm".into(), opt_city(neighbour))]),
        },
        "delete_connection" => {
            s.delete_connection(conn_type_of(arg(0)?)?, int(1)?, int(2)?);
            Json::Null
        }
        "delete_occupant_connection" => {
            s.delete_occupant_connection(i64_of(arg(0)?, "occupant")? as u64);
            Json::Null
        }
        "delete_all_connections_of_type" => {
            s.delete_all_connections_of_type(conn_type_of(arg(0)?)?);
            Json::Null
        }
        "has_connection" => Json::Bool(s.has_connection(conn_type_of(arg(0)?)?, int(1)?, int(2)?)),
        "find_city_on_other_side_of_connection" => opt_city(s.find_city_on_other_side_of_connection(int(0)?, int(1)?)),
        "find_connection_on_other_side" => {
            match s.find_connection_on_other_side(conn_type_of(arg(0)?)?, int(1)?, int(2)?) {
                None => Json::Null,
                Some(c) => Json::Object(vec![
                    ("token".into(), ji(c.token as i64)),
                    ("active".into(), Json::Bool(c.active)),
                ]),
            }
        }
        "neighbour_cell" => match s.neighbour_cell(int(0)?, int(1)?) {
            None => Json::Null,
            Some((c, (x, z))) => Json::Array(vec![ji(c as i64), ji(x), ji(z)]),
        },
        "connections" => conns(s.all_connections()),
        "connections_of_type" => conns(s.connections_of_type(conn_type_of(arg(0)?)?)),
        "connections_to_city" => conns(s.connections_to_city(i64_of(arg(0)?, "city")? as CityId)),
        "connections_of_type_to_city" => {
            conns(s.connections_of_type_to_city(conn_type_of(arg(0)?)?, i64_of(arg(1)?, "city")? as CityId))
        }
        "find_unvisited_connection" => s
            .find_unvisited_connection(conn_type_of(arg(0)?)?, i64_of(arg(1)?, "city")? as CityId)
            .map_or(Json::Null, |c| jconn(&c)),
        "find_connection_suitable_for_deal" => s
            .find_connection_suitable_for_deal(deal_type_of(arg(0)?)?, i64_of(arg(1)?, "city")? as CityId)
            .map_or(Json::Null, |c| jconn(&c)),
        "adjacent_cities" => cities(s.adjacent_cities(bool_of(arg(0)?, "flag")?)),
        "connected_cities" => cities(s.connected_cities(bool_of(arg(0)?, "flag")?)),
        "all_connected_cities" => cities(s.all_connected_cities(bool_of(arg(0)?, "flag")?)),
        "get_deal_cost" => {
            Json::Float(s.get_deal_cost(deal_type_of(arg(0)?)?, num(arg(1)?, "amount")?, city_of(arg(2)?)?))
        }
        "get_deal_cost_trunc" => ji(s
            .get_deal_cost(deal_type_of(arg(0)?)?, num(arg(1)?, "amount")?, city_of(arg(2)?)?)
            .trunc() as i64),
        "get_deal_max_amount" => Json::Float(s.get_deal_max_amount(deal_type_of(arg(0)?)?, city_of(arg(1)?)?)),
        "update_commodity_stats" => {
            s.update_commodity_stats(city_object_of(arg(0)?)?);
            Json::Null
        }
        "total_monthly_expense" => ji(s.total_monthly_expense()),
        "total_monthly_income" => ji(s.total_monthly_income()),
        "propose_deal" => s
            .propose_deal(deal_type_of(arg(0)?)?, i64_of(arg(1)?, "city")? as CityId)
            .map_or(Json::Null, |d| ji(w.deal_id(&d))),
        "propose_deals" => w.ids(&s.propose_deals(bool_of(arg(0)?, "add")?)),
        "add_deal" => Json::Bool(s.add_deal(&w.deal(i64_of(arg(0)?, "deal")?)?)),
        "set_ui_disable_flag" => {
            s.set_ui_disable_flag(bool_of(arg(0)?, "flag")?);
            Json::Null
        }
        "mark_deals_dirty" => {
            s.mark_deals_dirty();
            Json::Null
        }
        "find_deals_all" => w.ids(&s.find_deals_all()),
        "find_deals" => w.ids(&s.find_deals(deal_type_of(arg(0)?)?)),
        "find_deals_with" => w.ids(&s.find_deals_with(deal_type_of(arg(0)?)?, i64_of(arg(1)?, "city")? as CityId)),
        "clone_deal_data" => {
            let data = deal_data(w, arg(0)?, DealData::default())?;
            ji(w.deal_id(&s.clone_deal_data(&data, bool_of(arg(1)?, "silent")?)))
        }
        "simulate_deals" => {
            s.simulate_deals();
            Json::Null
        }
        "set_two_curves" => {
            s.set_two_curves(bool_of(arg(0)?, "on")?);
            Json::Null
        }
        other => return Err(format!("unknown method '{other}'")),
    };
    check_expect(op, got)
}

/// `neighbors_geometry`: the static helpers of §2: `fn` = `side` ([x, z,
/// countX, countZ] → "west" | "north" | "east" | "south" | "interior"),
/// `neighbouring_region_cell` ([side, x, z, pos, size]),
/// `neighbour_connection_cell` ([side, x, z, regionCell, npos, nsize] →
/// [x, z]), `is_valid_connection_cell` ([x, z, size]),
/// `connection_inner_cell` ([x, z, countX, countZ] → [x, z] or null),
/// `inner_cell` (the same, clamped).
fn geometry(op: &Json) -> R<()> {
    let arg = |i: usize| scalar::arg(op, i);
    let int = |i: usize| -> R<i32> { Ok(i64_of(arg(i)?, "argument")? as i32) };
    let side_arg = |i: usize| -> R<Side> {
        Ok(match str_of(arg(i)?, "side")? {
            "west" => Side::West,
            "north" => Side::North,
            "east" => Side::East,
            "south" => Side::South,
            "interior" => Side::Interior,
            other => return Err(format!("unknown side '{other}'")),
        })
    };
    let got = match str_of(field(op, "fn")?, "fn")? {
        "side" => Json::Str(
            match side_of(int(0)?, int(1)?, int(2)?, int(3)?) {
                Side::West => "west",
                Side::North => "north",
                Side::East => "east",
                Side::South => "south",
                Side::Interior => "interior",
            }
            .into(),
        ),
        "neighbouring_region_cell" => jpair(neighbouring_region_cell(
            side_arg(0)?,
            int(1)?,
            int(2)?,
            pair(arg(3)?, "pos")?,
            pair(arg(4)?, "size")?,
        )),
        "neighbour_connection_cell" => jpair(neighbour_connection_cell(
            side_arg(0)?,
            int(1)?,
            int(2)?,
            pair(arg(3)?, "region cell")?,
            pair(arg(4)?, "npos")?,
            pair(arg(5)?, "nsize")?,
        )),
        "is_valid_connection_cell" => Json::Bool(is_valid_connection_cell(int(0)?, int(1)?, pair(arg(2)?, "size")?)),
        "connection_inner_cell" => connection_inner_cell(int(0)?, int(1)?, int(2)?, int(3)?).map_or(Json::Null, jpair),
        "inner_cell" => jpair(inner_cell(int(0)?, int(1)?, int(2)?, int(3)?)),
        other => return Err(format!("unknown fn '{other}'")),
    };
    check_expect(op, got)
}

fn stats_of(j: &Json) -> R<CommodityStats> {
    let g = |k: &str| opt(j, k).map_or(Ok(0.0), |v| num(v, k));
    Ok(CommodityStats {
        demanded: g("demanded")?,
        exported: g("exported")?,
        produced: g("produced")?,
        imported: g("imported")?,
        cost_per_unit: g("cost_per_unit")?,
    })
}

/// `neighbors_price`: §8.1 as pure functions. `buyer`, `seller` (statistics),
/// `inputs` ({base, scale, min_markup, max_markup, existing}; the markups
/// are f32 values widened, as the tuning holds them), `curve` ("scarcity",
/// "garbage" or a flat array). With `amount`: `GetDealCost` (`expect`,
/// `expect_unit` = cost / amount, `expect_trunc`); without:
/// `GetDealMaxAmount` (`expect`).
fn price(op: &Json) -> R<()> {
    let buyer = stats_of(field(op, "buyer")?)?;
    let seller = stats_of(field(op, "seller")?)?;
    let i = field(op, "inputs")?;
    let g = |k: &str, d: f64| opt(i, k).map_or(Ok(d), |v| num(v, k));
    let p = PriceInputs {
        base: g("base", 0.0)?,
        scale: g("scale", 1.0)?,
        min_markup: f64::from(g("min_markup", 1.0)? as f32),
        max_markup: f64::from(g("max_markup", 1.0)? as f32),
        existing: g("existing", 0.0)?,
    };
    let Some(a) = opt(op, "amount") else {
        return check_expect(op, Json::Float(deal_max_amount(&buyer, &seller, &p)));
    };
    let amount = num(a, "amount")?;
    let curve = match opt(op, "curve") {
        None => ResponseCurve::default(),
        Some(Json::Str(s)) if s == "scarcity" => ResponseCurve::from_flat(&DEAL_PRICE_CURVE).unwrap_or_default(),
        Some(Json::Str(s)) if s == "garbage" => ResponseCurve::from_flat(&GARBAGE_COST_CURVE).unwrap_or_default(),
        Some(v) => {
            let flat = arr(v, "curve")?
                .iter()
                .map(|x| f32_of(x, "curve"))
                .collect::<R<Vec<f32>>>()?;
            ResponseCurve::from_flat(&flat).ok_or("bad curve")?
        }
    };
    let cost = deal_cost(&buyer, &seller, amount, &p, &|x| curve.eval(x));
    check_expect(op, Json::Float(cost))?;
    if let Some(v) = opt(op, "expect_unit") {
        let want = num(v, "expect_unit")?;
        if !scalar::close(cost / amount, want) {
            return Err(format!("unit {}, expected {want}", cost / amount));
        }
    }
    if let Some(v) = opt(op, "expect_trunc") {
        let want = i64_of(v, "expect_trunc")?;
        if cost.trunc() as i64 != want {
            return Err(format!("truncated cost {}, expected {want}", cost.trunc()));
        }
    }
    Ok(())
}

// ---- checks ---------------------------------------------------------------------------------------------

/// `neighbors_expect`: `state` (simulator and world fields), `deal` with
/// `deal_state`, `connection` ([type, x, z]) with `connection_state`, `city`
/// with `city_state` (budget, income, stats), `deals` (the list's IDs in
/// order), `props` (the marker props' positions in creation order). See
/// README.md for the fields.
fn expect(w: &World, op: &Json) -> R<()> {
    if let Some(want) = opt(op, "state") {
        let mut got: Vec<(String, Json)> = vec![
            ("posted".into(), ji(w.posted.borrow().len() as i64)),
            ("props".into(), ji(w.props.borrow().len() as i64)),
            ("today".into(), ji(w.today.get())),
        ];
        if let Ok(s) = w.sim() {
            let st = s.state();
            got.extend([
                ("initialized".to_owned(), Json::Bool(st.initialized)),
                ("ready".to_owned(), Json::Bool(st.ready)),
                ("deals_dirty".to_owned(), Json::Bool(st.deals_dirty)),
                ("stats_dirty".to_owned(), Json::Bool(st.stats_dirty)),
                ("ui_disabled".to_owned(), Json::Bool(st.ui_disabled)),
                ("busy".to_owned(), Json::Bool(st.busy)),
                ("connections".to_owned(), ji(st.connections as i64)),
                ("deals".to_owned(), ji(st.deals as i64)),
                ("tuning".to_owned(), Json::Bool(s.tuning_exemplar().is_some())),
                (
                    "rng".to_owned(),
                    s.proposal_rng_state()
                        .map_or(Json::Null, |(a, b)| Json::Array(vec![ji(a), ji(b)])),
                ),
            ]);
        }
        check_state(&got, want)?;
    }
    if let Some(want) = opt(op, "deal_state") {
        let d = w.deal(i64_of(field(op, "deal")?, "deal")?)?;
        check_state(&deal_state(w, &d), want)?;
    }
    if let Some(want) = opt(op, "connection_state") {
        let (t, x, z) = conn_key(field(op, "connection")?)?;
        let c = w.sim()?.find_connection(t, x, z).ok_or("no such connection")?;
        check_state(&connection_state(&c), want)?;
    }
    if let Some(want) = opt(op, "city_state") {
        let id = i64_of(field(op, "city")?, "city")? as CityId;
        let c = w.with_city(id, |c| c.clone()).ok_or("no such city")?;
        let mut got = vec![
            ("budget".to_owned(), Json::Float(c.budget)),
            ("income".to_owned(), Json::Float(c.income)),
        ];
        for (name, _) in STATS {
            let v = c.stats.get(name).copied().unwrap_or([0.0; 3]);
            got.push((
                name.to_owned(),
                Json::Array(v.iter().map(|&x| Json::Float(x)).collect()),
            ));
        }
        check_state(&got, want)?;
    }
    if let Some(want) = opt(op, "deals") {
        let got = w.ids(&w.sim()?.find_deals_all());
        if !scalar::matches(&got, want, false) {
            return Err(format!("deals {}, expected {}", show(&got), show(want)));
        }
    }
    if let Some(want) = opt(op, "props") {
        let got = Json::Array(
            w.props
                .borrow()
                .values()
                .map(|p| Json::Array(p.iter().map(|&v| jf(v)).collect()))
                .collect(),
        );
        if !scalar::matches(&got, want, false) {
            return Err(format!("props {}, expected {}", show(&got), show(want)));
        }
    }
    Ok(())
}

fn compare_hex(what: &str, got: &[u8], op: &Json) -> R<()> {
    if let Some(v) = opt(op, "hex") {
        let want = hex_decode(str_of(v, "hex")?)?;
        if got != want {
            return Err(format!(
                "{what} differs:\n    got      {}\n    expected {}",
                hex_encode(got),
                hex_encode(&want)
            ));
        }
    }
    Ok(())
}

/// Resolves references while reading a simulator record: a fresh connection
/// per connection object ID.
#[derive(Default)]
struct Resolver(Vec<(u32, Rc<NeighborConnection>)>);

impl RefResolver for Resolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        if class_id != CLSID_NEIGHBOR_CONNECTION {
            return None;
        }
        if let Some((_, c)) = self.0.iter().find(|(id, _)| *id == object_id) {
            return Some(RefValue::Object(c.clone()));
        }
        let c = NeighborConnection::new();
        self.0.push((object_id, c.clone()));
        Some(RefValue::Object(c))
    }
}

/// `neighbors_record`: with `read` (hex), reads a fresh simulator (each
/// reference a fresh connection; `expect` = the result, `connections` = its
/// list length); otherwise the simulator's record (§13) must equal `hex`
/// (references numbered 1, 2, … in list order).
fn record(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "read") {
        let bytes = hex_decode(str_of(v, "read")?)?;
        let s = NeighborsSimulator::new(w.rc()? as Rc<dyn NeighborsHost>, 0, w.session.get());
        let mut res = Resolver::default();
        let ok = s.read(&mut ByteReader::with_resolver(&bytes, &mut res));
        check_expect(op, Json::Bool(ok))?;
        if let Some(v) = opt(op, "connections") {
            let want = i64_of(v, "connections")?;
            let got = s.all_connections().len() as i64;
            if got != want {
                return Err(format!("{got} connections, expected {want}"));
            }
        }
        return Ok(());
    }
    let got = scalar::record_bytes(&*w.sim()?);
    compare_hex("record", &got, op)
}

/// `neighbors_deal_record`: with `read` (hex), reads a fresh deal (`expect`
/// = the result; `id` registers it; `rewrite` checks write reproduces the
/// bytes; `deal_state` checks it, `remaining` the unread bytes); otherwise
/// deal `deal`'s record must equal `hex`.
fn deal_record(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "read") {
        let bytes = hex_decode(str_of(v, "read")?)?;
        let d = NeighborDeal::new();
        let mut r = ByteReader::new(&bytes);
        check_expect(op, Json::Bool(d.read(&mut r)))?;
        if let Some(v) = opt(op, "remaining") {
            let want = i64_of(v, "remaining")?;
            if r.remaining() as i64 != want {
                return Err(format!("{} bytes left, expected {want}", r.remaining()));
            }
        }
        if let Some(v) = opt(op, "id") {
            w.register(i64_of(v, "id")?, &d);
        }
        if opt(op, "rewrite").map_or(Ok(false), |v| bool_of(v, "rewrite"))? {
            let mut out = ByteWriter::new();
            d.write(&mut out);
            if out.bytes() != &bytes[..] {
                return Err(format!(
                    "rewrite differs:\n    got      {}\n    expected {}",
                    hex_encode(out.bytes()),
                    hex_encode(&bytes)
                ));
            }
        }
        if let Some(want) = opt(op, "deal_state") {
            check_state(&deal_state(w, &d), want)?;
        }
        return Ok(());
    }
    let d = w.deal(i64_of(field(op, "deal")?, "deal")?)?;
    let mut out = ByteWriter::new();
    d.write(&mut out);
    compare_hex("deal record", out.bytes(), op)
}

/// `neighbors_connection_record`: with `read` (hex), reads a fresh
/// connection (`expect`, `rewrite`, `connection_state`); otherwise our
/// connection `connection` ([type, x, z]) must write `hex`.
fn connection_record(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "read") {
        let bytes = hex_decode(str_of(v, "read")?)?;
        let c = NeighborConnection::new();
        check_expect(op, Json::Bool(c.read(&mut ByteReader::new(&bytes))))?;
        if opt(op, "rewrite").map_or(Ok(false), |v| bool_of(v, "rewrite"))? {
            let mut out = ByteWriter::new();
            c.write(&mut out);
            if out.bytes() != &bytes[..] {
                return Err(format!(
                    "rewrite differs:\n    got      {}\n    expected {}",
                    hex_encode(out.bytes()),
                    hex_encode(&bytes)
                ));
            }
        }
        if let Some(want) = opt(op, "connection_state") {
            check_state(&connection_state(&c), want)?;
        }
        return Ok(());
    }
    let (t, x, z) = conn_key(field(op, "connection")?)?;
    let c = w.sim()?.find_connection(t, x, z).ok_or("no such connection")?;
    let mut out = ByteWriter::new();
    c.write(&mut out);
    compare_hex("connection record", out.bytes(), op)
}

// ---- the class-object store -----------------------------------------------------------------------------

fn slot(op: &Json) -> R<String> {
    Ok(str_of(field(op, "slot")?, "slot")?.to_owned())
}

fn class_arg(j: &Json) -> R<u32> {
    match j.as_str() {
        Some("simulator") => Ok(CLSID_NEIGHBORS_SIMULATOR),
        Some("connection") => Ok(CLSID_NEIGHBOR_CONNECTION),
        _ => u32_of(j, "class"),
    }
}

fn save_now(w: &World) -> R<Saved> {
    scalar::save(&[RefValue::Object(w.sim()? as Rc<dyn SaveObject>)])
}

/// `neighbors_save`: saves the simulator (and its connections) into `slot`.
fn save(w: &World, op: &Json) -> R<()> {
    let s = save_now(w)?;
    w.saves.borrow_mut().insert(slot(op)?, s);
    Ok(())
}

/// `neighbors_load`: loads `slot` into a new simulator (no `Init`; `expect`
/// = whether the store loaded cleanly, default true).
fn load(w: &World, op: &Json) -> R<()> {
    let saved = w.saves.borrow().get(&slot(op)?).cloned().ok_or("no such slot")?;
    let me = w.rc()?;
    let mut factory = |class: u32, _id: u32| -> Option<RefValue> {
        match class {
            CLSID_NEIGHBORS_SIMULATOR => {
                let token = me.next_token.get();
                me.next_token.set(token + 1);
                let s = NeighborsSimulator::new(me.clone() as Rc<dyn NeighborsHost>, token, me.session.get());
                Some(RefValue::Object(s as Rc<dyn SaveObject>))
            }
            CLSID_NEIGHBOR_CONNECTION => Some(RefValue::Object(NeighborConnection::new() as Rc<dyn SaveObject>)),
            _ => None,
        }
    };
    let (ok, roots) = scalar::load(&saved, &mut factory);
    let want = opt(op, "expect").map_or(Ok(true), |v| bool_of(v, "expect"))?;
    if ok != want {
        return Err(format!("store load clean = {ok}, expected {want}"));
    }
    if let Some(s) = roots.first().and_then(|r| r.object::<NeighborsSimulator>()) {
        *w.sim.borrow_mut() = Some(s);
    } else if want {
        return Err("the root did not resolve to a neighbours simulator".into());
    }
    Ok(())
}

/// `neighbors_expect_save`: saving now reproduces `slot` byte for byte.
fn expect_save(w: &World, op: &Json) -> R<()> {
    let want = w.saves.borrow().get(&slot(op)?).cloned().ok_or("no such slot")?;
    scalar::same_save(&save_now(w)?, &want)
}

/// `neighbors_patch`: patches an entry of `class` ("simulator",
/// "connection" or a CLSID) in `slot` (see the store ops in README.md).
fn patch(w: &World, op: &Json) -> R<()> {
    let class = class_arg(field(op, "class")?)?;
    let mut saves = w.saves.borrow_mut();
    let s = saves.get_mut(&slot(op)?).ok_or("no such slot")?;
    scalar::patch(s, class, op)
}
