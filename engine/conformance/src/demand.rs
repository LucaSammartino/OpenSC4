// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/demand.md`: a scripted city (demand
//! exemplars, census cells, buildings, history, ordinance and nation stubs,
//! neighbour connections, region cities, growth developers) and ops that
//! drive the demand simulator and its nodes. See README.md ("Demand ops").

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

use fc4_core::demand::{
    exemplar_prop as ep, msg, BuildingProfile, CensusLot, Connection, DemandHost, DemandNode, DemandPoster, DemandSim,
    GrowthDeveloper, CLSID_DEMAND_NODE, CLSID_DEMAND_SIMULATOR, DEMAND_EXEMPLAR_GROUP, TUNING_KEY,
};
use fc4_core::property::PropertyHolder;
use fc4_core::props::{exemplar_key, PROP_EXEMPLAR_TYPE};
use fc4_core::stream::{ByteReader, RefValue, ResKey, SaveObject};
use fc4_core::variant::{mem, Elem, Variant};

use crate::foundations::hex_decode;
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, f32_of, hex_key, i64_of, jf, ji, opt, str_of, u32_of, EventLog,
    Saved, R,
};

#[derive(Clone, Copy, Debug, Default)]
struct CellInfo {
    building_type: u32,
    wealth: u32,
    has_capacity: bool,
    desirability: i16,
    capacity: u16,
    eq: f32,
    workforce: f32,
}

#[derive(Clone, Debug, Default)]
struct Building {
    profile: Option<BuildingProfile>,
    exemplar: Option<Rc<PropertyHolder>>,
}

#[derive(Clone, Debug, Default)]
struct City {
    tax: BTreeMap<u32, f32>,
    allowable: BTreeMap<u32, i32>,
    population: BTreeMap<u32, i32>,
    extrapolated: BTreeMap<u32, i32>,
    workforce: f32,
}

#[derive(Default)]
pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    sim: RefCell<Option<Rc<DemandSim>>>,
    nodes: RefCell<BTreeMap<String, Rc<DemandNode>>>,
    next_token: Cell<u64>,
    refs: RefCell<BTreeMap<u64, i64>>,
    exemplars: RefCell<Vec<(ResKey, Rc<PropertyHolder>)>>,
    cells: Cell<(i32, i32)>,
    today: Cell<u32>,
    clock: Cell<u32>,
    clock_step: Cell<u32>,
    census: RefCell<BTreeMap<(i32, i32), CellInfo>>,
    buildings: RefCell<BTreeMap<u32, Building>>,
    dev_types: RefCell<BTreeMap<(u32, u32), u32>>,
    lots: RefCell<BTreeMap<u64, (u32, bool)>>,
    history_max: RefCell<BTreeMap<u32, f32>>,
    ordinance: RefCell<BTreeMap<u32, f32>>,
    nation: RefCell<BTreeMap<u32, f32>>,
    nation_default: Cell<f32>,
    connections: RefCell<Vec<(Connection, f32)>>,
    air: Cell<f32>,
    sea: Cell<f32>,
    cities: RefCell<BTreeMap<u64, City>>,
    all_connected: RefCell<Vec<u64>>,
    connected: RefCell<Vec<u64>>,
    self_city: Cell<u64>,
    developers: RefCell<Vec<GrowthDeveloper>>,
    last_build: RefCell<BTreeMap<u32, u32>>,
    saves: RefCell<BTreeMap<String, Saved>>,
}

/// The default `GetDeveloperType(purpose, wealth)` of the fake BDS (§2):
/// R 0x1000, CS 0x3100, CO 0x3300 (+ 0x10 · wealth); Tourism 0x3200 +
/// 0x10 · wealth; I-R/I-D/I-M/I-HT 0x4100/0x4200/0x4300/0x4400.
fn default_dev_type(purpose: u32, wealth: u32) -> u32 {
    match purpose {
        1 => 0x1000 + 0x10 * wealth,
        2 => 0x3100 + 0x10 * wealth,
        3 => 0x3300 + 0x10 * wealth,
        4 => 0x3200 + 0x10 * wealth,
        5 => 0x4100,
        6 => 0x4200,
        7 => 0x4300,
        8 => 0x4400,
        _ => 0,
    }
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            cells: Cell::new((0, 0)),
            today: Cell::new(fc4_core::julian::JULIAN_2000_01_01 as u32),
            next_token: Cell::new(0x100),
            ..Default::default()
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn token(&self) -> u64 {
        let t = self.next_token.get();
        self.next_token.set(t + 1);
        self.refs.borrow_mut().insert(t, 1);
        t
    }

    fn new_node(&self) -> Rc<DemandNode> {
        DemandNode::new(self.rc() as Rc<dyn DemandPoster>, self.token())
    }

    fn sim(&self) -> R<Rc<DemandSim>> {
        self.sim
            .borrow()
            .clone()
            .ok_or_else(|| "no demand simulator (demand_create)".into())
    }
}

impl DemandPoster for World {
    fn active_demand_changed(&self, _node: u64, key: u32, data2: i32) {
        self.events.push("post", vec![("key", ji(key)), ("data2", ji(data2))]);
    }
}

impl DemandHost for World {
    fn exemplar_keys(&self) -> Vec<ResKey> {
        self.exemplars.borrow().iter().map(|(k, _)| *k).collect()
    }
    fn exemplar(&self, key: ResKey) -> Option<Rc<PropertyHolder>> {
        // The last exemplar with the key wins (a later plugin overrides).
        self.exemplars
            .borrow()
            .iter()
            .rev()
            .find(|(k, _)| *k == key)
            .map(|(_, h)| h.clone())
    }
    fn warn(&self, text: &str) {
        self.events.push("warn", vec![("text", Json::Str(text.to_owned()))]);
    }
    fn create_node(&self) -> Option<Rc<DemandNode>> {
        Some(self.new_node())
    }
    fn add_ref(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) += 1;
    }
    fn release(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) -= 1;
    }
    fn cell_counts(&self) -> (i32, i32) {
        self.cells.get()
    }
    fn today(&self) -> u32 {
        self.today.get()
    }
    fn now_ms(&self) -> u32 {
        let t = self.clock.get();
        self.clock.set(t.wrapping_add(self.clock_step.get()));
        t
    }
    fn census_lot(&self, x: i32, z: i32) -> Option<CensusLot> {
        let c = *self.census.borrow().get(&(x, z))?;
        (c.has_capacity && c.building_type != 0).then_some(CensusLot {
            building_type: c.building_type,
            wealth: c.wealth,
        })
    }
    fn building_profile(&self, building_type: u32) -> Option<BuildingProfile> {
        self.buildings.borrow().get(&building_type)?.profile.clone()
    }
    fn developer_type(&self, purpose: u32, wealth: u32) -> u32 {
        self.dev_types
            .borrow()
            .get(&(purpose, wealth))
            .copied()
            .unwrap_or_else(|| default_dev_type(purpose, wealth))
    }
    fn desirability(&self, _dev_type: u32, x: i32, z: i32) -> i16 {
        self.census.borrow().get(&(x, z)).map_or(0, |c| c.desirability)
    }
    fn lot_capacity(&self, x: i32, z: i32, _dev_type: u32) -> u16 {
        self.census.borrow().get(&(x, z)).map_or(0, |c| c.capacity)
    }
    fn cell_eq(&self, x: i32, z: i32) -> f32 {
        self.census.borrow().get(&(x, z)).map_or(0.0, |c| c.eq)
    }
    fn cell_workforce_percent(&self, x: i32, z: i32) -> f32 {
        self.census.borrow().get(&(x, z)).map_or(0.0, |c| c.workforce)
    }
    fn lot_building_type(&self, lot: u64) -> u32 {
        self.lots.borrow().get(&lot).map_or(0, |l| l.0)
    }
    fn lot_excluded(&self, lot: u64) -> bool {
        self.lots.borrow().get(&lot).is_some_and(|l| l.1)
    }
    fn all_building_types(&self) -> Vec<u32> {
        self.buildings.borrow().keys().copied().collect()
    }
    fn building_exemplar(&self, building_type: u32) -> Option<Rc<PropertyHolder>> {
        self.buildings.borrow().get(&building_type)?.exemplar.clone()
    }
    fn history_max(&self, series: u32, _from_day: u32, _to_day: u32) -> Option<f32> {
        self.history_max.borrow().get(&series).copied()
    }
    fn ordinance_factor(&self, prop: u32, _index: u32) -> f32 {
        self.ordinance.borrow().get(&prop).copied().unwrap_or(1.0)
    }
    fn nation_economy_modifier(&self, demand_id: u32) -> f32 {
        self.nation
            .borrow()
            .get(&demand_id)
            .copied()
            .unwrap_or(self.nation_default.get())
    }
    fn connections(&self) -> Vec<Connection> {
        self.connections.borrow().iter().map(|c| c.0).collect()
    }
    fn traffic_arrived(&self, connection: u64) -> f32 {
        self.connections
            .borrow()
            .iter()
            .find(|c| c.0.token == connection)
            .map_or(0.0, |c| c.1)
    }
    fn airport_trips(&self) -> f32 {
        self.air.get()
    }
    fn seaport_trips(&self) -> f32 {
        self.sea.get()
    }
    fn all_connected_cities(&self) -> Vec<u64> {
        self.all_connected.borrow().clone()
    }
    fn connected_cities(&self) -> Vec<u64> {
        self.connected.borrow().clone()
    }
    fn own_regional_city(&self) -> u64 {
        self.self_city.get()
    }
    fn city_population(&self, city: u64, pop_type: u32) -> i32 {
        self.cities
            .borrow()
            .get(&city)
            .and_then(|c| c.population.get(&pop_type).copied())
            .unwrap_or(0)
    }
    fn city_extrapolated_population(&self, city: u64, pop_type: u32) -> i32 {
        self.cities
            .borrow()
            .get(&city)
            .and_then(|c| c.extrapolated.get(&pop_type).copied())
            .unwrap_or(0)
    }
    fn city_allowable_extrapolation(&self, city: u64, pop_type: u32) -> i32 {
        self.cities
            .borrow()
            .get(&city)
            .and_then(|c| c.allowable.get(&pop_type).copied())
            .unwrap_or(0)
    }
    fn city_tax_rate(&self, city: u64, tax_type: u32) -> f32 {
        self.cities
            .borrow()
            .get(&city)
            .and_then(|c| c.tax.get(&tax_type).copied())
            .unwrap_or(0.0)
    }
    fn city_workforce_percentage(&self, city: u64) -> f32 {
        self.cities.borrow().get(&city).map_or(0.0, |c| c.workforce)
    }
    fn city_extrapolate_growth(&self, city: u64, pop_type: u32, amount: f32) {
        self.events.push(
            "extrapolate",
            vec![
                ("city", ji(city as i64)),
                ("type", ji(pop_type)),
                ("amount", jf(amount)),
            ],
        );
    }
    fn growth_developers(&self) -> Vec<GrowthDeveloper> {
        self.developers.borrow().clone()
    }
    fn day_of_last_successful_build(&self, dev_type: u32) -> u32 {
        self.last_build.borrow().get(&dev_type).copied().unwrap_or(0)
    }
    fn record_history(&self, series: u32, value: i32) {
        self.events
            .push("history", vec![("series", ji(series)), ("value", ji(value))]);
    }
    fn set_history_queue_length(&self, series: u32, length: u32) {
        self.events
            .push("history_queue", vec![("series", ji(series)), ("length", ji(length))]);
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "demand", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("demand_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "exemplar" => add_exemplar(&w, op),
        "tuning" => tuning(&w, op),
        "building" => building(&w, op),
        "cells" => cells(&w, op),
        "lot" => lot(&w, op),
        "connections" => connections(&w, op),
        "cities" => cities(&w, op),
        "developers" => developers(&w, op),
        "create" => {
            let t = w.token();
            *w.sim.borrow_mut() = Some(DemandSim::new(w.rc() as Rc<dyn DemandHost>, t));
            Ok(())
        }
        "init" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.init()))),
        "shutdown" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.shutdown()))),
        "message" => message(&w, op),
        "state_changed" => state_changed(&w, op),
        "call" => call(&w, op),
        "new_node" => {
            let label = str_of(field(op, "node").unwrap_or(&Json::Null), "node").map(str::to_owned);
            label.map(|l| {
                let n = w.new_node();
                w.nodes.borrow_mut().insert(l, n);
            })
        }
        "node" => node_call(&w, op),
        "expect_node" => expect_node(&w, op),
        "expect" => expect_sim(&w, op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        "expect_refs" => expect_refs(&w, op),
        "save" => save(&w, op),
        "load" => load(&w, op),
        "expect_save" => expect_save(&w, op),
        "patch" => patch(&w, op),
        "expect_payload" => expect_payload(&w, op),
        "node_record" => node_record(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    })
}

// ---- environment ----------------------------------------------------------------------------------

fn map_f32(j: &Json) -> R<BTreeMap<u32, f32>> {
    let mut m = BTreeMap::new();
    for (k, v) in j.as_object().ok_or("expected an object")? {
        let id = Json::Str(k.clone()).as_i64().ok_or_else(|| format!("bad key '{k}'"))? as u32;
        m.insert(id, f32_of(v, k)?);
    }
    Ok(m)
}

fn map_i32(j: Option<&Json>) -> R<BTreeMap<u32, i32>> {
    let mut m = BTreeMap::new();
    if let Some(j) = j {
        for (k, v) in j.as_object().ok_or("expected an object")? {
            let id = Json::Str(k.clone()).as_i64().ok_or_else(|| format!("bad key '{k}'"))? as u32;
            m.insert(id, i64_of(v, k)? as i32);
        }
    }
    Ok(m)
}

fn ids(j: &Json) -> R<Vec<u64>> {
    arr(j, "list")?
        .iter()
        .map(|v| i64_of(v, "city").map(|x| x as u64))
        .collect()
}

fn env(w: &World, op: &Json) -> R<()> {
    if let Some(c) = opt(op, "cells") {
        let a = arr(c, "cells")?;
        w.cells.set((
            i64_of(a.first().ok_or("cells")?, "x")? as i32,
            i64_of(a.get(1).ok_or("cells")?, "z")? as i32,
        ));
    }
    if let Some(t) = opt(op, "today") {
        w.today.set(u32_of(t, "today")?);
    }
    if let Some(d) = opt(op, "date") {
        let j = fc4_core::julian::julian_from_civil(
            i64_of(field(d, "year")?, "year")? as i32,
            i64_of(field(d, "month")?, "month")? as i32,
            i64_of(field(d, "day")?, "day")? as i32,
        );
        w.today.set(j as u32);
    }
    if let Some(v) = opt(op, "advance_days") {
        w.today.set(w.today.get().wrapping_add(u32_of(v, "advance_days")?));
    }
    if let Some(v) = opt(op, "clock_step") {
        w.clock_step.set(u32_of(v, "clock_step")?);
    }
    if let Some(v) = opt(op, "air") {
        w.air.set(f32_of(v, "air")?);
    }
    if let Some(v) = opt(op, "sea") {
        w.sea.set(f32_of(v, "sea")?);
    }
    if let Some(v) = opt(op, "history_max") {
        *w.history_max.borrow_mut() = map_f32(v)?;
    }
    if let Some(v) = opt(op, "ordinance") {
        *w.ordinance.borrow_mut() = map_f32(v)?;
    }
    if let Some(v) = opt(op, "nation") {
        *w.nation.borrow_mut() = map_f32(v)?;
    }
    if let Some(v) = opt(op, "nation_default") {
        w.nation_default.set(f32_of(v, "nation_default")?);
    }
    if let Some(v) = opt(op, "dev_types") {
        let mut m = BTreeMap::new();
        for e in arr(v, "dev_types")? {
            m.insert(
                (
                    u32_of(field(e, "purpose")?, "purpose")?,
                    u32_of(field(e, "wealth")?, "wealth")?,
                ),
                u32_of(field(e, "dev")?, "dev")?,
            );
        }
        *w.dev_types.borrow_mut() = m;
    }
    Ok(())
}

fn u32_list(j: &Json, what: &str) -> R<Vec<Elem>> {
    let mut out = Vec::new();
    for v in arr(j, what)? {
        match v {
            Json::Array(pair) => {
                for x in pair {
                    out.push(Elem::U32(u32_of(x, what)?));
                }
            }
            x => out.push(Elem::U32(u32_of(x, what)?)),
        }
    }
    Ok(out)
}

/// `demand_exemplar`: a demand exemplar written with friendly names.
fn add_exemplar(w: &World, op: &Json) -> R<()> {
    let instance = u32_of(field(op, "instance")?, "instance")?;
    let group = opt(op, "group").map_or(Ok(DEMAND_EXEMPLAR_GROUP), |g| u32_of(g, "group"))?;
    let mut h = match opt(op, "props") {
        Some(p) => scalar::holder_of(p)?,
        None => PropertyHolder::new(),
    };
    let ty = opt(op, "exemplar_type").map_or(Ok(3), |t| u32_of(t, "exemplar_type"))?;
    h.add(PROP_EXEMPLAR_TYPE, Variant::Scalar(Elem::U32(ty)));
    if let Some(k) = opt(op, "key") {
        h.add(ep::KEY, Variant::Scalar(Elem::U32(u32_of(k, "key")?)));
    }
    for (name, id) in [
        ("min", ep::MIN),
        ("max", ep::MAX),
        ("supply", ep::INITIAL_SUPPLY),
        ("demand", ep::INITIAL_DEMAND),
    ] {
        if let Some(v) = opt(op, name) {
            h.add(id, Variant::Scalar(Elem::I32(i64_of(v, name)? as i32)));
        }
    }
    for (name, id) in [
        ("drives", ep::DRIVES),
        ("satisfies", ep::SATISFIES),
        ("contributes", ep::CONTRIBUTES),
        ("caps", ep::CAPS),
        ("sums", ep::SUMS),
    ] {
        if let Some(v) = opt(op, name) {
            h.add(id, Variant::array(mem::UINT32, u32_list(v, name)?));
        }
    }
    if let Some(v) = opt(op, "region_use") {
        h.add(
            ep::REGION_USE,
            Variant::Scalar(Elem::U8(u32_of(v, "region_use")? as u8)),
        );
    }
    w.exemplars
        .borrow_mut()
        .push((exemplar_key(group, instance), Rc::new(h)));
    Ok(())
}

/// `demand_tuning`: the Demand Simulator exemplar (`props`), or `null` to
/// remove it.
fn tuning(w: &World, op: &Json) -> R<()> {
    w.exemplars.borrow_mut().retain(|(k, _)| *k != TUNING_KEY);
    if let Some(p) = opt(op, "props") {
        w.exemplars
            .borrow_mut()
            .push((TUNING_KEY, Rc::new(scalar::holder_of(p)?)));
    }
    Ok(())
}

fn pairs_variant(j: &Json, what: &str) -> R<Variant> {
    Ok(Variant::array(mem::UINT32, u32_list(j, what)?))
}

/// `demand_building`: a building type's profile and exemplar.
fn building(w: &World, op: &Json) -> R<()> {
    let bt = u32_of(field(op, "type")?, "type")?;
    let purpose = opt(op, "purpose").map_or(Ok(0), |p| u32_of(p, "purpose"))?;
    let mut capacities = Vec::new();
    if let Some(c) = opt(op, "capacities") {
        for map in arr(c, "capacities")? {
            let mut m = Vec::new();
            for e in arr(map, "capacity map")? {
                let pair = arr(e, "capacity entry")?;
                m.push((
                    u32_of(pair.first().ok_or("dev")?, "dev")?,
                    u32_of(pair.get(1).ok_or("cap")?, "cap")? as u16,
                ));
            }
            capacities.push(m);
        }
    }
    let mut ex = match opt(op, "props") {
        Some(p) => scalar::holder_of(p)?,
        None => PropertyHolder::new(),
    };
    use fc4_core::demand::building_prop as bp;
    if let Some(v) = opt(op, "creates") {
        ex.add(bp::CREATED, pairs_variant(v, "creates")?);
    }
    if let Some(v) = opt(op, "satisfies") {
        ex.add(bp::SATISFIED, pairs_variant(v, "satisfies")?);
    }
    for (name, id) in [
        ("creates_amounts", bp::CREATED_AMOUNTS),
        ("satisfies_amounts", bp::SATISFIED_AMOUNTS),
    ] {
        if let Some(v) = opt(op, name) {
            let items = arr(v, name)?
                .iter()
                .map(|x| f32_of(x, name).map(Elem::F32))
                .collect::<R<Vec<_>>>()?;
            ex.add(id, Variant::array(mem::FLOAT32, items));
        }
    }
    let profile = (purpose != 0 || !capacities.is_empty()).then_some(BuildingProfile { purpose, capacities });
    w.buildings.borrow_mut().insert(
        bt,
        Building {
            profile,
            exemplar: Some(Rc::new(ex)),
        },
    );
    Ok(())
}

/// `demand_cells`: census cells (each entry may cover a rectangle with `w`, `h`).
fn cells(w: &World, op: &Json) -> R<()> {
    let mut m = w.census.borrow_mut();
    if opt(op, "replace").and_then(crate::runner::boolean).unwrap_or(true) {
        m.clear();
    }
    for c in arr(field(op, "cells")?, "cells")? {
        let x0 = i64_of(field(c, "x")?, "x")? as i32;
        let z0 = i64_of(field(c, "z")?, "z")? as i32;
        let cw = opt(c, "w").map_or(Ok(1), |v| i64_of(v, "w"))? as i32;
        let ch = opt(c, "h").map_or(Ok(1), |v| i64_of(v, "h"))? as i32;
        let info = CellInfo {
            building_type: opt(c, "building_type").map_or(Ok(1), |v| u32_of(v, "building_type"))?,
            wealth: opt(c, "wealth").map_or(Ok(1), |v| u32_of(v, "wealth"))?,
            has_capacity: opt(c, "has_capacity").map_or(Ok(true), |v| bool_of(v, "has_capacity"))?,
            desirability: opt(c, "desirability").map_or(Ok(100), |v| i64_of(v, "desirability"))? as i16,
            capacity: opt(c, "capacity").map_or(Ok(0), |v| u32_of(v, "capacity"))? as u16,
            eq: opt(c, "eq").map_or(Ok(0.0), |v| f32_of(v, "eq"))?,
            workforce: opt(c, "workforce").map_or(Ok(0.0), |v| f32_of(v, "workforce"))?,
        };
        for z in z0..z0 + ch {
            for x in x0..x0 + cw {
                m.insert((x, z), info);
            }
        }
    }
    Ok(())
}

/// `demand_lot`: a lot object for StateChanged (building type, exclusion).
fn lot(w: &World, op: &Json) -> R<()> {
    let id = u32_of(field(op, "lot")?, "lot")? as u64;
    let bt = opt(op, "building_type").map_or(Ok(0), |v| u32_of(v, "building_type"))?;
    let ex = opt(op, "excluded").map_or(Ok(false), |v| bool_of(v, "excluded"))?;
    w.lots.borrow_mut().insert(id, (bt, ex));
    Ok(())
}

fn connections(w: &World, op: &Json) -> R<()> {
    let mut v = Vec::new();
    for (i, c) in arr(field(op, "connections")?, "connections")?.iter().enumerate() {
        v.push((
            Connection {
                token: 0x9000 + i as u64,
                conn_type: u32_of(field(c, "type")?, "type")?,
                city: opt(c, "city").map_or(Ok(0), |x| i64_of(x, "city"))? as u64,
            },
            opt(c, "traffic").map_or(Ok(0.0), |x| f32_of(x, "traffic"))?,
        ));
    }
    *w.connections.borrow_mut() = v;
    Ok(())
}

/// `demand_cities`: region cities, plus the neighbour lists.
fn cities(w: &World, op: &Json) -> R<()> {
    if let Some(list) = opt(op, "cities") {
        let mut m = BTreeMap::new();
        for c in arr(list, "cities")? {
            let id = i64_of(field(c, "id")?, "id")? as u64;
            m.insert(
                id,
                City {
                    tax: opt(c, "tax").map_or(Ok(BTreeMap::new()), map_f32)?,
                    allowable: map_i32(opt(c, "allowable"))?,
                    population: map_i32(opt(c, "population"))?,
                    extrapolated: map_i32(opt(c, "extrapolated"))?,
                    workforce: opt(c, "workforce").map_or(Ok(0.0), |v| f32_of(v, "workforce"))?,
                },
            );
        }
        *w.cities.borrow_mut() = m;
    }
    if let Some(v) = opt(op, "all_connected") {
        *w.all_connected.borrow_mut() = ids(v)?;
    }
    if let Some(v) = opt(op, "connected") {
        *w.connected.borrow_mut() = ids(v)?;
    }
    if let Some(v) = opt(op, "self") {
        w.self_city.set(i64_of(v, "self")? as u64);
    }
    Ok(())
}

fn developers(w: &World, op: &Json) -> R<()> {
    let mut v = Vec::new();
    let mut last = BTreeMap::new();
    for d in arr(field(op, "developers")?, "developers")? {
        let sub = u32_of(field(d, "sub_type")?, "sub_type")?;
        let source = match opt(d, "source") {
            None => None,
            Some(s) => {
                let a = arr(s, "source")?;
                Some((
                    f32_of(a.first().ok_or("source")?, "supply")?,
                    f32_of(a.get(1).ok_or("source")?, "demand")?,
                ))
            }
        };
        v.push(GrowthDeveloper {
            sub_type: sub,
            kind: u32_of(field(d, "kind")?, "kind")?,
            source,
        });
        last.insert(sub, opt(d, "last_build").map_or(Ok(0), |x| u32_of(x, "last_build"))?);
    }
    *w.developers.borrow_mut() = v;
    *w.last_build.borrow_mut() = last;
    Ok(())
}

// ---- calls ------------------------------------------------------------------------------------------

fn msg_type(j: &Json) -> R<u32> {
    Ok(match j.as_str() {
        Some("SimBegin") => msg::SIM_BEGIN,
        Some("SimNewDay") => msg::SIM_NEW_DAY,
        Some("SimNewMonth") => msg::SIM_NEW_MONTH,
        Some("SimIdle") => msg::SIM_IDLE,
        Some("PreSave") => msg::PRE_SAVE,
        Some("ConnectionCreated") => msg::CONNECTION_CREATED,
        Some("ConnectionDeleted") => msg::CONNECTION_DELETED,
        Some("IngredChangesMade") => msg::INGRED_CHANGES_MADE,
        _ => u32_of(j, "type")?,
    })
}

fn message(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let t = msg_type(field(op, "type")?)?;
    let d = |k| opt(op, k).map_or(Ok(0), |v| u32_of(v, k));
    let r = s.handle_message(t, d("data1")?, d("data2")?, d("data3")?);
    check_expect(op, Json::Bool(r))
}

fn state_changed(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let g = |k| opt(op, k).map_or(Ok(0), |v| u32_of(v, k));
    s.on_state_changed(
        g("lot")? as u64,
        g("old")?,
        g("new")?,
        g("building_type")?,
        g("wealth")?,
    );
    Ok(())
}

fn a_u32(op: &Json, i: usize) -> R<u32> {
    u32_of(arg(op, i)?, "argument")
}

fn a_f32(op: &Json, i: usize) -> R<f32> {
    f32_of(arg(op, i)?, "argument")
}

fn call(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let m = str_of(field(op, "method")?, "method")?;
    let got = match m {
        "get_demand" => Json::Bool(s.get_demand(a_u32(op, 0)?, a_u32(op, 1)?).is_some()),
        "simulator_type" => ji(s.simulator_type()),
        "update_occupant_effects" => {
            s.update_occupant_effects(a_u32(op, 0)?, a_f32(op, 1)?, a_f32(op, 2)?);
            Json::Null
        }
        "jobs_per_unit" => Json::Array(
            s.jobs_per_unit(a_u32(op, 0)?, a_u32(op, 1)?)
                .iter()
                .map(|&v| jf(v))
                .collect(),
        ),
        "jobs_by_census" => ji(s.jobs_by_census(a_u32(op, 0)?)),
        "neutral_tax_rate" => jf(s.neutral_tax_rate()),
        "local_population_summary" => Json::Object(
            s.local_population_summary()
                .into_iter()
                .map(|(k, v)| (hex_key(k), ji(v)))
                .collect(),
        ),
        "do_idle" => {
            s.do_idle(a_u32(op, 0)?);
            Json::Null
        }
        "propagate" => {
            s.do_demand_index_links();
            Json::Null
        }
        "simulator_begin" => {
            s.simulator_begin();
            Json::Null
        }
        "daily" => {
            s.daily();
            Json::Null
        }
        "monthly" => {
            s.monthly();
            Json::Null
        }
        "update_modifiers" => {
            s.update_modifiers();
            Json::Null
        }
        "evaluate_connection_utilization" => {
            s.evaluate_connection_utilization();
            Json::Null
        }
        "export_demand" => {
            s.export_demand();
            Json::Null
        }
        "reset_regional_demands" => {
            s.reset_regional_demands(bool_of(arg(op, 0)?, "initial")?);
            Json::Null
        }
        "perform_census" => {
            s.perform_census(i64_of(arg(op, 0)?, "x")? as i32, i64_of(arg(op, 1)?, "z")? as i32);
            Json::Null
        }
        "apply_census" => {
            s.apply_census();
            Json::Null
        }
        "record_census_history" => {
            s.record_census_history();
            Json::Null
        }
        "add_agent_effect" => {
            s.add_agent_effect(a_u32(op, 0)?, bool_of(arg(op, 1)?, "add")?, a_f32(op, 2)?);
            Json::Null
        }
        "update_capacity" => {
            s.update_capacity(a_u32(op, 0)?, a_u32(op, 1)?, bool_of(arg(op, 2)?, "occupied")?);
            Json::Null
        }
        "set_economy_bonuses" => {
            s.set_economy_bonuses(a_f32(op, 0)?, a_f32(op, 1)?);
            Json::Null
        }
        "reload_tuning" => {
            s.reload_tuning();
            Json::Null
        }
        other => return Err(format!("unknown demand method '{other}'")),
    };
    check_expect_bits(op, &got)?;
    check_expect(op, got)
}

/// `expect_bits` (a `"0x…"` string): the float32 bits of a float result,
/// exactly (the `expect` tolerance cannot see 1 ulp).
fn check_expect_bits(op: &Json, got: &Json) -> R<()> {
    let Some(want) = opt(op, "expect_bits") else {
        return Ok(());
    };
    let want = u32_of(want, "expect_bits")?;
    match *got {
        Json::Float(v) => {
            let bits = (v as f32).to_bits();
            if bits != want {
                return Err(format!("returned {v} (bits 0x{bits:08X}), expected bits 0x{want:08X}"));
            }
            Ok(())
        }
        _ => Err("expect_bits needs a float result".into()),
    }
}

fn node_of(w: &World, op: &Json) -> R<Rc<DemandNode>> {
    if let Some(l) = opt(op, "node") {
        let l = str_of(l, "node")?;
        return w.nodes.borrow().get(l).cloned().ok_or_else(|| format!("no node '{l}'"));
    }
    let key = u32_of(field(op, "key")?, "key")?;
    w.sim()?.node(key).ok_or_else(|| format!("no node 0x{key:X}"))
}

fn node_call(w: &World, op: &Json) -> R<()> {
    let n = node_of(w, op)?;
    let m = str_of(field(op, "method")?, "method")?;
    let b = Json::Bool;
    let got = match m {
        "init" => b(n.init()),
        "shutdown" => b(n.shutdown()),
        "simulation_begin" => b(n.simulation_begin()),
        "get_id" => ji(n.id()),
        "set_id" => b(n.set_id(a_u32(op, 0)?)),
        "supply" => jf(n.supply()),
        "demand" => jf(n.demand()),
        "new_supply" => jf(n.new_supply()),
        "new_demand" => jf(n.new_demand()),
        "active_demand" => jf(n.active_demand()),
        "active_max" => jf(n.active_max()),
        "active_min" => jf(n.active_min()),
        "economy_modifier" => jf(n.economy_modifier()),
        "tax_modifier" => jf(n.tax_modifier()),
        "demand_cap" => jf(n.demand_cap()),
        "region_use" => ji(n.region_use()),
        "add_to_supply" => b(n.add_to_supply(a_f32(op, 0)?)),
        "add_to_demand" => b(n.add_to_demand(a_f32(op, 0)?)),
        "set_supply" => b(n.set_supply(a_f32(op, 0)?)),
        "set_demand" => b(n.set_demand(a_f32(op, 0)?)),
        "set_active_max" => b(n.set_active_max(a_f32(op, 0)?)),
        "set_active_min" => b(n.set_active_min(a_f32(op, 0)?)),
        "set_economy_modifier" => b(n.set_economy_modifier(a_f32(op, 0)?)),
        "set_tax_modifier" => b(n.set_tax_modifier(a_f32(op, 0)?)),
        "set_demand_cap" => b(n.set_demand_cap(a_f32(op, 0)?)),
        "set_region_use" => b(n.set_region_use(a_u32(op, 0)?)),
        "end_of_cycle" => b(n.end_of_cycle()),
        "calculate_active_demand" => {
            let smooth = match opt(op, "args") {
                Some(_) => bool_of(arg(op, 0)?, "smooth")?,
                None => false,
            };
            n.calculate_active_demand(smooth);
            Json::Null
        }
        "debug_is_value_locked" => b(n.debug_is_value_locked()),
        other => return Err(format!("unknown node method '{other}'")),
    };
    check_expect(op, got)
}

fn expect_node(w: &World, op: &Json) -> R<()> {
    let d = node_of(w, op)?.snapshot();
    let got = vec![
        ("id".to_owned(), ji(d.id)),
        ("supply".to_owned(), jf(d.supply)),
        ("demand".to_owned(), jf(d.demand)),
        ("new_supply".to_owned(), jf(d.new_supply)),
        ("new_demand".to_owned(), jf(d.new_demand)),
        ("active_demand".to_owned(), jf(d.active_demand)),
        ("active_min".to_owned(), jf(d.active_min)),
        ("active_max".to_owned(), jf(d.active_max)),
        ("economy_modifier".to_owned(), jf(d.economy_modifier)),
        ("tax_modifier".to_owned(), jf(d.tax_modifier)),
        ("demand_cap".to_owned(), jf(d.demand_cap)),
        ("region_use".to_owned(), ji(d.region_use)),
        ("flag".to_owned(), ji(d.flag)),
        ("loaded_from_save".to_owned(), Json::Bool(d.loaded_from_save)),
        ("initialized".to_owned(), Json::Bool(d.initialized)),
    ];
    check_state(&got, field(op, "state")?)
}

fn census_json(m: &BTreeMap<u32, f32>) -> Json {
    Json::Object(m.iter().map(|(k, v)| (hex_key(*k), jf(*v))).collect())
}

fn expect_sim(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?.snapshot();
    let got = vec![
        ("initialized".to_owned(), Json::Bool(s.initialized)),
        ("loaded_from_save".to_owned(), Json::Bool(s.loaded_from_save)),
        ("census_state".to_owned(), ji(s.census_state)),
        ("cursor".to_owned(), Json::Array(vec![ji(s.cursor.0), ji(s.cursor.1)])),
        ("co_economy_bonus".to_owned(), jf(s.co_economy_bonus)),
        ("i_economy_bonus".to_owned(), jf(s.i_economy_bonus)),
        ("commercial_relief".to_owned(), jf(s.commercial_relief)),
        ("industrial_relief".to_owned(), jf(s.industrial_relief)),
        ("current_census".to_owned(), census_json(&s.current_census)),
        ("previous_census".to_owned(), census_json(&s.previous_census)),
        (
            "node_keys".to_owned(),
            Json::Array(s.node_keys.iter().map(|&k| ji(k)).collect()),
        ),
        ("link_count".to_owned(), ji(s.link_count as i64)),
        (
            "effect_building_types".to_owned(),
            Json::Array(s.effect_building_types.iter().map(|&k| ji(k)).collect()),
        ),
        (
            "export_cities".to_owned(),
            Json::Array(s.export_cities.iter().map(|&k| ji(k as i64)).collect()),
        ),
    ];
    check_state(&got, field(op, "state")?)
}

/// `demand_expect_refs`: reference counts of the nodes held by the simulator
/// (by key): `{"0x21010": 1}`.
fn expect_refs(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    for (k, v) in field(op, "refs")?.as_object().ok_or("refs must be an object")? {
        let key = Json::Str(k.clone()).as_i64().ok_or("bad key")? as u32;
        let n = s.node(key).ok_or_else(|| format!("no node {k}"))?;
        let got = w.refs.borrow().get(&n.token()).copied().unwrap_or(0);
        if got != i64_of(v, k)? {
            return Err(format!("node {k}: {got} references"));
        }
    }
    Ok(())
}

// ---- persistence ------------------------------------------------------------------------------------

fn slot(op: &Json) -> R<String> {
    Ok(str_of(field(op, "slot")?, "slot")?.to_owned())
}

fn class_arg(j: &Json) -> R<u32> {
    match j.as_str() {
        Some("simulator") => Ok(CLSID_DEMAND_SIMULATOR),
        Some("node") => Ok(CLSID_DEMAND_NODE),
        _ => u32_of(j, "class"),
    }
}

fn save_now(w: &World) -> R<Saved> {
    let s = w.sim()?;
    scalar::save(&[RefValue::Object(s as Rc<dyn SaveObject>)])
}

fn save(w: &World, op: &Json) -> R<()> {
    let s = save_now(w)?;
    w.saves.borrow_mut().insert(slot(op)?, s);
    Ok(())
}

fn load(w: &World, op: &Json) -> R<()> {
    let name = slot(op)?;
    let saved = w.saves.borrow().get(&name).cloned().ok_or("no such slot")?;
    let me = w.rc();
    let mut factory = |class: u32, _id: u32| -> Option<RefValue> {
        match class {
            CLSID_DEMAND_SIMULATOR => Some(RefValue::Object(
                DemandSim::new(me.clone() as Rc<dyn DemandHost>, me.token()) as Rc<dyn SaveObject>,
            )),
            CLSID_DEMAND_NODE => Some(RefValue::Object(me.new_node() as Rc<dyn SaveObject>)),
            _ => None,
        }
    };
    let (ok, roots) = scalar::load(&saved, &mut factory);
    let want = opt(op, "expect").map_or(Ok(true), |v| bool_of(v, "expect"))?;
    if ok != want {
        return Err(format!("store load clean = {ok}, expected {want}"));
    }
    if !want {
        return Ok(()); // a failed load leaves the current simulator in place
    }
    let sim = roots
        .first()
        .and_then(|r| r.object::<DemandSim>())
        .ok_or("the root did not resolve to a demand simulator")?;
    *w.sim.borrow_mut() = Some(sim);
    Ok(())
}

fn expect_save(w: &World, op: &Json) -> R<()> {
    let want = w.saves.borrow().get(&slot(op)?).cloned().ok_or("no such slot")?;
    scalar::same_save(&save_now(w)?, &want)
}

fn patch(w: &World, op: &Json) -> R<()> {
    let class = class_arg(field(op, "class")?)?;
    let mut saves = w.saves.borrow_mut();
    let s = saves.get_mut(&slot(op)?).ok_or("no such slot")?;
    scalar::patch(s, class, op)
}

fn expect_payload(w: &World, op: &Json) -> R<()> {
    let class = class_arg(field(op, "class")?)?;
    let entry = opt(op, "entry").map_or(Ok(0), |v| i64_of(v, "entry"))? as usize;
    let saves = w.saves.borrow();
    let s = saves.get(&slot(op)?).ok_or("no such slot")?;
    let got = scalar::entry_payload(s, class, entry)?;
    let want = hex_decode(str_of(field(op, "hex")?, "hex")?)?;
    if got != want {
        return Err(format!(
            "payload differs:\n    got      {}\n    expected {}",
            crate::foundations::hex_encode(&got),
            crate::foundations::hex_encode(&want)
        ));
    }
    Ok(())
}

/// `demand_node_record`: with `hex` and `read: true`, reads the bytes into
/// the node (`expect` = the result); otherwise the node's record must equal
/// `hex`.
fn node_record(w: &World, op: &Json) -> R<()> {
    let n = node_of(w, op)?;
    let bytes = hex_decode(str_of(field(op, "hex")?, "hex")?)?;
    if opt(op, "read").and_then(crate::runner::boolean).unwrap_or(false) {
        let mut r = ByteReader::new(&bytes);
        return check_expect(op, Json::Bool(n.read(&mut r)));
    }
    let got = scalar::record_bytes(&*n);
    if got != bytes {
        return Err(format!(
            "node record differs:\n    got      {}\n    expected {}",
            crate::foundations::hex_encode(&got),
            crate::foundations::hex_encode(&bytes)
        ));
    }
    Ok(())
}
