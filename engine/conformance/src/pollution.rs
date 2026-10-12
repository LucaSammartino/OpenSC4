// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/pollution.md`: a scripted city (zones,
//! traffic, ordinance factors, deals, landfills, occupants, budget, visual
//! effects) and ops that drive the pollution simulator. See README.md
//! ("Pollution ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap};
use std::rc::{Rc, Weak};

use fc4_core::grid::{create_for_clsid, CityDims, CoreGrids};
use fc4_core::pollution::{
    shipped_tuning, BudgetItem, Deal, GridProvider, PollutionHost, PollutionSim, PollutionState, Remaining, TractGrid,
    WorstTract, CLSID_POLLUTION_SIMULATOR, IID_DEPARTMENT_BUDGET, IID_OCCUPANT,
};
use fc4_core::property::PropertyHolder;
use fc4_core::rng::{CrtRand, FixedSeeds, RzRandomSuper};
use fc4_core::store::{load_store, save_store, LoadRegistry, Package};
use fc4_core::stream::{ByteReader, ByteWriter, InStream, OutStream, RefResolver, RefValue, SaveObject, SaveRegistry};
use fc4_core::variant::{Elem, Variant};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{boolean, field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, f32_of, i64_of, jf, ji, opt, str_of, u32_of, EventLog, R,
};

/// Test-only class IDs of the host objects a save refers to.
const OCCUPANT_CLASS: u32 = 0x0CC0_0001;
const DEPARTMENT_CLASS: u32 = 0x0CC0_0002;

const GRID_NAMES: [&str; 8] = [
    "air_src",
    "water_src",
    "garbage_src",
    "rad_src",
    "air_val",
    "water_val",
    "garbage_val",
    "rad_val",
];

#[derive(Default)]
struct Occupant {
    label: String,
    props: PropertyHolder,
    position: [f32; 3],
    bbox: ([f32; 3], [f32; 3]),
    powered: bool,
    budget: Option<BudgetItem>,
    /// How many times the simulator wrote each property.
    writes: BTreeMap<u32, u32>,
}

#[derive(Default)]
struct Effect {
    started: bool,
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
    sim: RefCell<Option<Rc<PollutionSim>>>,
    next_token: Cell<u64>,
    saves: RefCell<BTreeMap<String, SavedCity>>,
    city: Cell<Option<CityDims>>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    altitude: Cell<f32>,
    zone_default: Cell<u32>,
    zones: RefCell<BTreeMap<(i32, i32), u32>>,
    traffic: RefCell<BTreeMap<(i32, i32), u8>>,
    factors: RefCell<BTreeMap<(u32, u32), f32>>,
    deals: RefCell<Vec<(u32, String, Deal)>>,
    population: Cell<i32>,
    landfill: Cell<(u32, u32)>,
    landfill_tiles: Cell<u32>,
    no_department: Cell<bool>,
    funding: Cell<f32>,
    no_effects: Cell<bool>,
    occupants: RefCell<BTreeMap<u64, Occupant>>,
    labels: RefCell<BTreeMap<String, u64>>,
    names: RefCell<BTreeMap<u64, String>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    effects: RefCell<BTreeMap<u64, Effect>>,
    effect_count: Cell<u32>,
    crt_mark: Cell<Option<CrtRand>>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            next_token: Cell::new(0x100),
            funding: Cell::new(1.0),
            ..Default::default()
        });
        *w.tuning.borrow_mut() = Some(Rc::new(shipped_tuning()));
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn sim(&self) -> R<Rc<PollutionSim>> {
        self.sim
            .borrow()
            .clone()
            .ok_or_else(|| "no pollution simulator (pollution_create)".into())
    }

    fn dims(&self) -> CityDims {
        self.city.get().unwrap_or(CityDims::square(128))
    }

    fn token(&self, label: &str) -> u64 {
        let t = self.next_token.get();
        self.next_token.set(t + 1);
        self.names.borrow_mut().insert(t, label.to_owned());
        t
    }

    fn name(&self, t: u64) -> Json {
        Json::Str(self.names.borrow().get(&t).cloned().unwrap_or_else(|| format!("#{t}")))
    }

    fn occupant(&self, j: &Json) -> R<u64> {
        let l = str_of(j, "occupant")?;
        self.labels
            .borrow()
            .get(l)
            .copied()
            .ok_or_else(|| format!("unknown occupant '{l}'"))
    }

    fn with_occ<T>(&self, occ: u64, f: impl FnOnce(&mut Occupant) -> T) -> Option<T> {
        self.occupants.borrow_mut().get_mut(&occ).map(f)
    }

    fn zone(&self, x: i32, z: i32) -> u32 {
        self.zones
            .borrow()
            .get(&(x, z))
            .copied()
            .unwrap_or(self.zone_default.get())
    }
}

impl GridProvider<i16> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<i16>>> {
        CoreGrids { city: self.dims() }.create_grid(instance, tract_size)
    }
}

fn post_name(t: u32) -> Json {
    use fc4_core::pollution::msg;
    Json::Str(
        match t {
            msg::RADIATION_CHANGED => "RadiationChanged",
            msg::AIR_CHANGED => "AirChanged",
            msg::WATER_CHANGED => "WaterChanged",
            msg::GARBAGE_CHANGED => "GarbageChanged",
            msg::DEAL_STATUS => "DealStatus",
            _ => return ji(t),
        }
        .into(),
    )
}

fn types_json(t: &[u32]) -> Json {
    Json::Array(t.iter().map(|&v| ji(v)).collect())
}

fn vec3(v: [f32; 3]) -> Json {
    Json::Array(v.iter().map(|&x| jf(x)).collect())
}

impl PollutionHost for World {
    fn cell_counts(&self) -> (i32, i32) {
        let d = self.dims();
        (d.cells_x, d.cells_z)
    }
    fn cell_widths(&self) -> (f32, f32) {
        let d = self.dims();
        (d.cell_width_x, d.cell_width_z)
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        let d = self.dims();
        ((x / d.cell_width_x).floor() as i32, (z / d.cell_width_z).floor() as i32)
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn altitude(&self, _x: f32, _z: f32) -> f32 {
        self.altitude.get()
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
    fn remove_agent(&self) {
        self.events.push("remove_agent", vec![]);
    }
    fn subscribe(&self, t: &[u32]) {
        self.events.push("subscribe", vec![("types", types_json(t))]);
    }
    fn unsubscribe(&self, t: &[u32]) {
        self.events.push("unsubscribe", vec![("types", types_json(t))]);
    }
    fn post(&self, msg_type: u32, data1: u32) {
        self.events
            .push("post", vec![("type", post_name(msg_type)), ("data1", ji(data1))]);
    }
    fn record_history(&self, series: u32, value: i32) {
        self.events
            .push("history", vec![("series", ji(series)), ("value", ji(value))]);
    }
    fn set_history_queue_length(&self, series: u32, length: u32) {
        self.events
            .push("history_queue", vec![("series", ji(series)), ("length", ji(length))]);
    }
    fn add_prop_requester(&self) {
        self.events.push("add_prop_requester", vec![]);
    }
    fn remove_prop_requester(&self) {
        self.events.push("remove_prop_requester", vec![]);
    }
    fn zone_type(&self, x: i32, z: i32) -> u32 {
        self.zone(x, z)
    }
    fn air_traffic(&self, x: i32, z: i32) -> u8 {
        self.traffic.borrow().get(&(x, z)).copied().unwrap_or(0)
    }
    fn cumulative_factor(&self, id: u32, index: u32) -> f32 {
        self.factors.borrow().get(&(id, index)).copied().unwrap_or(1.0)
    }
    fn find_deals(&self, deal_type: u32) -> Vec<Deal> {
        self.deals
            .borrow()
            .iter()
            .filter(|d| d.0 == deal_type)
            .map(|d| d.2)
            .collect()
    }
    fn set_deal_amount_used(&self, deal: u64, used: i32) {
        let mut deals = self.deals.borrow_mut();
        if let Some(d) = deals.iter_mut().find(|d| d.2.token == deal) {
            d.2.amount_used = used;
            self.events
                .push("deal_used", vec![("deal", Json::Str(d.1.clone())), ("used", ji(used))]);
        }
    }
    fn mark_deals_dirty(&self) {
        self.events.push("mark_deals_dirty", vec![]);
    }
    fn is_occupant_powered(&self, occ: u64) -> bool {
        self.occupants.borrow().get(&occ).is_some_and(|o| o.powered)
    }
    fn population(&self) -> i32 {
        self.population.get()
    }
    fn landfill_usage(&self) -> (u32, u32) {
        self.landfill.get()
    }
    fn landfill_tile_count(&self) -> u32 {
        self.landfill_tiles.get()
    }
    fn set_lot_condition(&self, cell_x: i32, cell_z: i32, condition: u32) {
        self.events.push(
            "lot_condition",
            vec![("x", ji(cell_x)), ("z", ji(cell_z)), ("condition", ji(condition))],
        );
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant> {
        self.occupants.borrow().get(&occ)?.props.get(id).cloned()
    }
    fn set_occupant_property(&self, occ: u64, id: u32, value: Variant) {
        self.with_occ(occ, |o| {
            o.props.add(id, value);
            *o.writes.entry(id).or_insert(0) += 1;
        });
    }
    fn remove_occupant_property(&self, occ: u64, id: u32) {
        self.with_occ(occ, |o| {
            let mut h = PropertyHolder::new();
            for i in o.props.ids() {
                if i != id {
                    if let Some(v) = o.props.get(i) {
                        h.add(i, v.clone());
                    }
                }
            }
            o.props = h;
        });
    }
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        self.occupants.borrow().get(&occ).map_or([0.0; 3], |o| o.position)
    }
    fn occupant_bounding_box(&self, occ: u64) -> ([f32; 3], [f32; 3]) {
        self.occupants
            .borrow()
            .get(&occ)
            .map_or(([0.0; 3], [0.0; 3]), |o| o.bbox)
    }
    fn add_ref(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) += 1;
    }
    fn release(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) -= 1;
        if self.effects.borrow().contains_key(&token) {
            self.effects.borrow_mut().remove(&token);
            self.events.push("effect_release", vec![("effect", self.name(token))]);
        }
    }
    fn object_ref(&self, token: u64) -> RefValue {
        let class_id = if self.occupants.borrow().contains_key(&token) {
            OCCUPANT_CLASS
        } else {
            DEPARTMENT_CLASS
        };
        RefValue::Host { token, class_id }
    }
    fn resolve_ref(&self, r: &RefValue, iid: u32) -> Option<u64> {
        let RefValue::Host { token, class_id } = r else {
            return None;
        };
        let ok = matches!(
            (*class_id, iid),
            (OCCUPANT_CLASS, IID_OCCUPANT) | (DEPARTMENT_CLASS, IID_DEPARTMENT_BUDGET)
        );
        if ok {
            Some(*token)
        } else {
            // The stream's reference is dropped here.
            self.release(*token);
            None
        }
    }
    fn create_department(&self, id: u32, group: u32) -> Option<u64> {
        if self.no_department.get() {
            return None;
        }
        let d = self.token("department");
        self.labels.borrow_mut().insert("department".into(), d);
        self.events
            .push("create_department", vec![("id", ji(id)), ("group", ji(group))]);
        Some(d)
    }
    fn set_department_name(&self, _dept: u64, group: u32, instance: u32) {
        self.events.push(
            "department_name",
            vec![("group", ji(group)), ("instance", ji(instance))],
        );
    }
    fn set_department_max_funding(&self, _dept: u64, fraction: f32) {
        self.events.push("max_funding", vec![("value", jf(fraction))]);
    }
    fn create_line_item(&self, _dept: u64, line: u32, locally_funded: bool, name_group: u32, name_instance: u32) {
        self.events.push(
            "line_item",
            vec![
                ("line", ji(line)),
                ("local", Json::Bool(locally_funded)),
                ("group", ji(name_group)),
                ("instance", ji(name_instance)),
            ],
        );
    }
    fn department_funding(&self, _dept: u64) -> f32 {
        self.funding.get()
    }
    fn set_line_item_full_expenses(&self, _dept: u64, line: u32, value: i64) {
        self.events
            .push("full_expenses", vec![("line", ji(line)), ("value", ji(value))]);
    }
    fn budget_item_for_purpose(&self, occ: u64, purpose: u32) -> Option<BudgetItem> {
        self.occupants
            .borrow()
            .get(&occ)?
            .budget
            .filter(|b| b.purpose == purpose)
    }
    fn add_budget_item_to_budget(&self, occ: u64, index: u32) {
        self.events
            .push("budget_add", vec![("occupant", self.name(occ)), ("index", ji(index))]);
    }
    fn remove_budget_item_from_budget(&self, occ: u64, index: u32) {
        self.events.push(
            "budget_remove",
            vec![("occupant", self.name(occ)), ("index", ji(index))],
        );
    }
    fn change_budget_item_cost(&self, occ: u64, index: u32, cost: i64) {
        self.with_occ(occ, |o| {
            if let Some(b) = o.budget.as_mut() {
                b.cost = cost;
            }
        });
        self.events.push(
            "budget_cost",
            vec![("occupant", self.name(occ)), ("index", ji(index)), ("cost", ji(cost))],
        );
    }
    fn add_to_budget_item_line_expenses(&self, item: &BudgetItem, delta: i64) {
        self.events.push(
            "line_expenses",
            vec![("line", ji(item.line_item)), ("delta", ji(delta))],
        );
    }
    fn create_visual_effect(&self, id: u32, kind: u32) -> Option<u64> {
        if self.no_effects.get() {
            return None;
        }
        let n = self.effect_count.get() + 1;
        self.effect_count.set(n);
        let label = format!("effect{n}");
        let e = self.token(&label);
        self.refs.borrow_mut().insert(e, 1);
        self.effects.borrow_mut().insert(e, Effect { started: false });
        self.events.push(
            "effect_create",
            vec![("effect", Json::Str(label)), ("id", ji(id)), ("kind", ji(kind))],
        );
        Some(e)
    }
    fn set_effect_position(&self, effect: u64, position: [f32; 3]) {
        self.events.push(
            "effect_position",
            vec![("effect", self.name(effect)), ("position", vec3(position))],
        );
    }
    fn set_effect_param(&self, effect: u64, param: u32, value: f32) {
        self.events.push(
            "effect_param",
            vec![
                ("effect", self.name(effect)),
                ("param", ji(param)),
                ("value", jf(value)),
            ],
        );
    }
    fn start_effect(&self, effect: u64, t: u32) {
        if let Some(e) = self.effects.borrow_mut().get_mut(&effect) {
            e.started = true;
        }
        self.events
            .push("effect_start", vec![("effect", self.name(effect)), ("t", ji(t))]);
    }
    fn stop_effect(&self, effect: u64, t: u32) {
        if let Some(e) = self.effects.borrow_mut().get_mut(&effect) {
            e.started = false;
        }
        self.events
            .push("effect_stop", vec![("effect", self.name(effect)), ("t", ji(t))]);
    }
    fn is_effect_started(&self, effect: u64) -> bool {
        self.effects.borrow().get(&effect).is_some_and(|e| e.started)
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "pollution", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("pollution_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "tuning" => tuning(&w, op),
        "occupant" => occupant(&w, op),
        "create" => create(&w, op),
        "init" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.init()))),
        "shutdown" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.shutdown()))),
        "message" => message(&w, op),
        "call" => call(&w, op),
        "set" => set(&w, op),
        "expect" => expect(&w, op),
        "set_grid" => set_grid(&w, op),
        "expect_grid" => expect_grid(&w, op),
        "expect_occupant" => expect_occupant(&w, op),
        "expect_refs" => expect_refs(&w, op),
        "expect_effects" => expect_effects(&w, op),
        "crt" => crt(&w, op),
        "expect_fx" => expect_fx(&w, op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        "save" => save(&w, op),
        "load" => load(&w, op),
        "read" => read(&w, op),
        "expect_save" => expect_save(&w, op),
        "patch" => patch(&w, op),
        "expect_payload" => expect_payload(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    })
}

// ---- environment ----------------------------------------------------------------------------------

/// `{x, z, w?, d?}` → the cells (or tracts) of the rectangle.
fn rect_of(e: &Json) -> R<Vec<(i32, i32)>> {
    let x = i64_of(field(e, "x")?, "x")? as i32;
    let z = i64_of(field(e, "z")?, "z")? as i32;
    let w = opt(e, "w").map_or(Ok(1), |v| i64_of(v, "w"))? as i32;
    let d = opt(e, "d").map_or(Ok(1), |v| i64_of(v, "d"))? as i32;
    if !(0..=4096).contains(&w) || !(0..=4096).contains(&d) {
        return Err("rectangle too large".into());
    }
    Ok((x..x + w).flat_map(|i| (z..z + d).map(move |j| (i, j))).collect())
}

fn pair_of(v: &Json, k: &str) -> R<(i64, i64)> {
    let a = arr(v, k)?;
    if a.len() != 2 {
        return Err(format!("'{k}' must have 2 elements"));
    }
    Ok((i64_of(&a[0], k)?, i64_of(&a[1], k)?))
}

fn f3(v: &Json, k: &str) -> R<[f32; 3]> {
    let a = arr(v, k)?;
    if a.len() != 3 {
        return Err(format!("'{k}' must have 3 elements"));
    }
    Ok([f32_of(&a[0], k)?, f32_of(&a[1], k)?, f32_of(&a[2], k)?])
}

fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "cells") {
        let (x, z) = pair_of(v, "cells")?;
        let mut d = w.dims();
        d.cells_x = x as i32;
        d.cells_z = z as i32;
        w.city.set(Some(d));
    }
    if let Some(v) = opt(op, "cell_width") {
        let a = arr(v, "cell_width")?;
        let mut d = w.dims();
        d.cell_width_x = f32_of(&a[0], "cell_width")?;
        d.cell_width_z = f32_of(&a[1], "cell_width")?;
        w.city.set(Some(d));
    }
    if let Some(v) = opt(op, "altitude") {
        w.altitude.set(f32_of(v, "altitude")?);
    }
    if let Some(v) = opt(op, "zone_default") {
        w.zone_default.set(u32_of(v, "zone_default")?);
        w.zones.borrow_mut().clear();
    }
    if let Some(v) = opt(op, "zones") {
        for e in arr(v, "zones")? {
            let zt = u32_of(field(e, "zone")?, "zone")?;
            for c in rect_of(e)? {
                w.zones.borrow_mut().insert(c, zt);
            }
        }
    }
    if let Some(v) = opt(op, "traffic") {
        for e in arr(v, "traffic")? {
            let t = u32_of(field(e, "v")?, "v")? as u8;
            for c in rect_of(e)? {
                w.traffic.borrow_mut().insert(c, t);
            }
        }
    }
    if let Some(v) = opt(op, "factors") {
        for e in arr(v, "factors")? {
            let id = u32_of(field(e, "id")?, "id")?;
            let value = f32_of(field(e, "value")?, "value")?;
            match opt(e, "index") {
                Some(i) => {
                    w.factors.borrow_mut().insert((id, u32_of(i, "index")?), value);
                }
                None => {
                    for i in 0..16 {
                        w.factors.borrow_mut().insert((id, i), value);
                    }
                }
            }
        }
    }
    if let Some(v) = op.get("deals") {
        let mut list = Vec::new();
        if !matches!(v, Json::Null) {
            for e in arr(v, "deals")? {
                let label = str_of(field(e, "deal")?, "deal")?;
                let t = match field(e, "type")?.as_str() {
                    Some("import") => fc4_core::pollution::deal::IMPORT,
                    Some("export") => fc4_core::pollution::deal::EXPORT,
                    _ => u32_of(field(e, "type")?, "type")?,
                };
                let d = Deal {
                    token: w.token(label),
                    amount: i64_of(field(e, "amount")?, "amount")? as i32,
                    amount_used: opt(e, "used").map_or(Ok(0), |u| i64_of(u, "used"))? as i32,
                };
                list.push((t, label.to_owned(), d));
            }
        }
        *w.deals.borrow_mut() = list;
    }
    if let Some(v) = opt(op, "population") {
        w.population.set(i64_of(v, "population")? as i32);
    }
    if let Some(v) = opt(op, "landfill") {
        let (u, c) = pair_of(v, "landfill")?;
        w.landfill.set((u as u32, c as u32));
    }
    if let Some(v) = opt(op, "landfill_tiles") {
        w.landfill_tiles.set(u32_of(v, "landfill_tiles")?);
    }
    if let Some(v) = opt(op, "department") {
        w.no_department.set(!bool_of(v, "department")?);
    }
    if let Some(v) = opt(op, "funding") {
        w.funding.set(f32_of(v, "funding")?);
    }
    if let Some(v) = opt(op, "effects") {
        w.no_effects.set(!bool_of(v, "effects")?);
    }
    Ok(())
}

/// `pollution_tuning`: `shipped` (default true), `props` (merged over it),
/// `remove` (IDs), or `none`.
fn tuning(w: &World, op: &Json) -> R<()> {
    if opt(op, "none").and_then(boolean).unwrap_or(false) {
        *w.tuning.borrow_mut() = None;
        return Ok(());
    }
    let mut h = if opt(op, "shipped").and_then(boolean).unwrap_or(true) {
        shipped_tuning()
    } else {
        PropertyHolder::new()
    };
    if let Some(p) = opt(op, "props") {
        let extra = scalar::holder_of(p)?;
        for id in extra.ids() {
            if let Some(v) = extra.get(id) {
                h.add(id, v.clone());
            }
        }
    }
    if let Some(r) = opt(op, "remove") {
        let gone: Vec<u32> = arr(r, "remove")?
            .iter()
            .map(|v| u32_of(v, "remove"))
            .collect::<R<_>>()?;
        let mut k = PropertyHolder::new();
        for id in h.ids() {
            if !gone.contains(&id) {
                if let Some(v) = h.get(id) {
                    k.add(id, v.clone());
                }
            }
        }
        h = k;
    }
    *w.tuning.borrow_mut() = Some(Rc::new(h));
    Ok(())
}

/// `pollution_occupant`: defines (or redefines) an occupant.
fn occupant(w: &World, op: &Json) -> R<()> {
    let label = str_of(field(op, "occupant")?, "occupant")?.to_owned();
    let d = w.dims();
    let (cw, cwz) = (d.cell_width_x, d.cell_width_z);
    let mut o = Occupant {
        label: label.clone(),
        powered: true,
        ..Default::default()
    };
    if let Some(p) = opt(op, "props") {
        o.props = scalar::holder_of(p)?;
    }
    if let Some(v) = opt(op, "tract") {
        let (tx, tz) = pair_of(v, "tract")?;
        o.position = [(tx as f32 * 2.0 + 1.0) * cw, 0.0, (tz as f32 * 2.0 + 1.0) * cwz];
    }
    if let Some(v) = opt(op, "position") {
        o.position = f3(v, "position")?;
    }
    // `cells`: [x0, z0, x1, z1] inclusive footprint → a bounding box inside it.
    if let Some(v) = opt(op, "cells") {
        let a: Vec<f32> = arr(v, "cells")?.iter().map(|x| f32_of(x, "cells")).collect::<R<_>>()?;
        if a.len() != 4 {
            return Err("'cells' must have 4 elements".into());
        }
        o.bbox = (
            [a[0] * cw + 1.0, 0.0, a[1] * cwz + 1.0],
            [a[2] * cw + cw - 1.0, 10.0, a[3] * cwz + cwz - 1.0],
        );
    } else {
        o.bbox = (o.position, o.position);
    }
    if let Some(v) = opt(op, "bbox") {
        let a = arr(v, "bbox")?;
        o.bbox = (f3(&a[0], "bbox")?, f3(&a[1], "bbox")?);
    }
    if let Some(v) = opt(op, "powered") {
        o.powered = bool_of(v, "powered")?;
    }
    if let Some(b) = opt(op, "budget") {
        o.budget = Some(BudgetItem {
            department_id: fc4_core::pollution::budget::DEPARTMENT,
            line_item: u32_of(field(b, "line")?, "line")?,
            purpose: opt(b, "purpose").map_or(Ok(fc4_core::pollution::budget::PURPOSE), |v| u32_of(v, "purpose"))?,
            cost: opt(b, "cost").map_or(Ok(0), |v| i64_of(v, "cost"))?,
            index: opt(b, "index").map_or(Ok(0), |v| u32_of(v, "index"))?,
            local_funding: 1.0,
        });
    }
    let existing = w.labels.borrow().get(&label).copied();
    let t = match existing {
        Some(t) => t,
        None => {
            let t = w.token(&label);
            w.labels.borrow_mut().insert(label, t);
            w.refs.borrow_mut().insert(t, 1);
            t
        }
    };
    w.occupants.borrow_mut().insert(t, o);
    Ok(())
}

fn create(w: &World, op: &Json) -> R<()> {
    let seed = opt(op, "fx_seed").map_or(Ok(1), |v| u32_of(v, "fx_seed"))?;
    let me = w.rc();
    let sim = PollutionSim::new(
        me.clone() as Rc<dyn PollutionHost>,
        w.token("pollution"),
        seed,
        &seeds(),
    );
    *w.sim.borrow_mut() = Some(sim);
    Ok(())
}

fn seeds() -> FixedSeeds {
    FixedSeeds {
        wall_clock: 1,
        process_clock: 1,
    }
}

// ---- calls ------------------------------------------------------------------------------------------

fn message(w: &World, op: &Json) -> R<()> {
    use fc4_core::pollution::msg;
    let s = w.sim()?;
    let t = match field(op, "type")?.as_str() {
        Some("SimBegin") => msg::SIM_BEGIN,
        Some("SimNewDay") => msg::SIM_NEW_DAY,
        Some("SimNewMonth") => msg::SIM_NEW_MONTH,
        Some("OccupantInserted") => msg::OCCUPANT_INSERTED,
        Some("OccupantRemoved") => msg::OCCUPANT_REMOVED,
        Some("PostCityInit") => msg::POST_CITY_INIT,
        Some("TuningChanged") => msg::TUNING_CHANGED,
        Some("PreViewLevelChange") => msg::PRE_VIEW_LEVEL_CHANGE,
        _ => u32_of(field(op, "type")?, "type")?,
    };
    let occ = match opt(op, "occupant") {
        Some(o) => w.occupant(o)?,
        None => 0,
    };
    let view = opt(op, "view_level").map_or(Ok(0), |v| u32_of(v, "view_level"))?;
    let count = opt(op, "count").map_or(Ok(1), |v| i64_of(v, "count"))?;
    if !(1..=100_000).contains(&count) {
        return Err("'count' must be 1..=100000".into());
    }
    let mut r = true;
    for _ in 0..count {
        r &= s.do_message(t, occ, view);
    }
    check_expect(op, Json::Bool(r))
}

fn a_i32(op: &Json, i: usize) -> R<i32> {
    Ok(i64_of(arg(op, i)?, "argument")? as i32)
}

fn a_u32(op: &Json, i: usize) -> R<u32> {
    u32_of(arg(op, i)?, "argument")
}

fn a_f32(op: &Json, i: usize) -> R<f32> {
    f32_of(arg(op, i)?, "argument")
}

fn a_occ(w: &World, op: &Json, i: usize) -> R<u64> {
    w.occupant(arg(op, i)?)
}

fn worst(t: WorstTract) -> Json {
    Json::Array(vec![ji(t.x), ji(t.z), ji(t.value)])
}

fn found_u(r: (bool, u32)) -> Json {
    Json::Array(vec![Json::Bool(r.0), ji(r.1)])
}

fn found_f(r: (bool, f32)) -> Json {
    Json::Array(vec![Json::Bool(r.0), jf(r.1)])
}

fn grid_name(s: &PollutionState, g: &Option<Rc<dyn TractGrid<i16>>>) -> Json {
    let Some(g) = g else { return Json::Null };
    for (i, mine) in s.grids.iter().enumerate() {
        if let Some(m) = mine {
            if Rc::ptr_eq(m, g) {
                return Json::Str(GRID_NAMES[i].into());
            }
        }
    }
    Json::Str("foreign".into())
}

fn call(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let m = str_of(field(op, "method")?, "method")?;
    let got = match m {
        "simulate_day" => {
            s.simulate_day();
            Json::Null
        }
        "simulate_month" => {
            s.simulate_month();
            Json::Null
        }
        "end_of_cycle" => {
            s.run_end_of_cycle();
            Json::Null
        }
        "update_visual_effects" => {
            s.update_visual_effects();
            Json::Null
        }
        "reload_tuning" => {
            s.reload_tuning();
            Json::Null
        }
        "add_radiation_to_city" => {
            s.add_radiation_to_city(
                [a_f32(op, 0)?, a_f32(op, 1)?, a_f32(op, 2)?],
                a_f32(op, 3)?,
                a_f32(op, 4)?,
            );
            Json::Null
        }
        "dispose" => {
            let mut rem = Remaining::default();
            if let Some(r) = opt(op, "remaining") {
                let g = |k: &str| opt(r, k).map_or(Ok(0), |v| u32_of(v, k));
                rem = Remaining {
                    wtp: g("wtp")?,
                    inc: g("inc")?,
                    wte: g("wte")?,
                };
            }
            let taken = s.dispose(a_f32(op, 0)?, &mut rem);
            Json::Object(vec![
                ("taken".into(), jf(taken)),
                (
                    "remaining".into(),
                    Json::Object(vec![
                        ("wtp".into(), ji(rem.wtp)),
                        ("inc".into(), ji(rem.inc)),
                        ("wte".into(), ji(rem.wte)),
                    ]),
                ),
            ])
        }
        "pollution_grid" => {
            let g = s.pollution_grid(a_u32(op, 0)?);
            grid_name(&s.snapshot(), &g)
        }
        "pollution_value" => ji(s.pollution_value(a_u32(op, 0)?, a_i32(op, 1)?, a_i32(op, 2)?)),
        "air_value" => ji(s.air_value(a_i32(op, 0)?, a_i32(op, 1)?)),
        "water_value" => ji(s.water_value(a_i32(op, 0)?, a_i32(op, 1)?)),
        "garbage_value" => ji(s.garbage_value(a_i32(op, 0)?, a_i32(op, 1)?)),
        "average_air_value" => ji(s.average_air_value()),
        "average_water_value" => ji(s.average_water_value()),
        "average_garbage_value" => ji(s.average_garbage_value()),
        "is_air_polluted" => Json::Bool(s.is_air_polluted(a_i32(op, 0)?, a_i32(op, 1)?)),
        "is_water_polluted" => Json::Bool(s.is_water_polluted(a_i32(op, 0)?, a_i32(op, 1)?)),
        "is_garbage_accumulated" => Json::Bool(s.is_garbage_accumulated(a_i32(op, 0)?, a_i32(op, 1)?)),
        "is_radioactive" => Json::Bool(s.is_radioactive(a_i32(op, 0)?, a_i32(op, 1)?)),
        "worst_air_tract" => worst(s.worst_air_tract()),
        "worst_water_tract" => worst(s.worst_water_tract()),
        "worst_garbage_tract" => worst(s.worst_garbage_tract()),
        "garbage_supply" => ji(s.garbage_supply()),
        "landfill_tile_capacity" => ji(s.landfill_tile_capacity()),
        "garbage_capacity" => ji(s.garbage_capacity()),
        "is_garbage_uncollected" => Json::Bool(s.is_garbage_uncollected()),
        "total_garbage_uncollected" => ji(s.total_garbage_uncollected()),
        "set_is_radioactive" => {
            s.set_is_radioactive(a_i32(op, 0)?, a_i32(op, 1)?, bool_of(arg(op, 2)?, "on")?);
            Json::Null
        }
        "set_air_value" => {
            s.set_air_value(a_i32(op, 0)?, a_i32(op, 1)?, a_i32(op, 2)? as i16);
            Json::Null
        }
        "set_water_value" => {
            s.set_water_value(a_i32(op, 0)?, a_i32(op, 1)?, a_i32(op, 2)? as i16);
            Json::Null
        }
        "set_garbage_value" => {
            s.set_garbage_value(a_i32(op, 0)?, a_i32(op, 1)?, a_i32(op, 2)? as i16);
            Json::Null
        }
        "add_to_garbage_supply" => ji(s.add_to_garbage_supply(a_i32(op, 0)?)),
        "set_garbage_capacity" => ji(s.set_garbage_capacity(a_i32(op, 0)?)),
        "monthly_incinerator_capacity" => ji(s.monthly_incinerator_capacity()),
        "monthly_wte_capacity" => ji(s.monthly_wte_capacity()),
        "recycling_center_effect" => jf(s.recycling_center_effect()),
        "garbage_abducted_prev_month" => ji(s.garbage_abducted_prev_month()),
        "garbage_produced_last_month" => ji(s.last_month().produced),
        "garbage_imported_last_month" => ji(s.last_month().imported),
        "garbage_exported_last_month" => ji(s.last_month().exported),
        "garbage_recycled_last_month" => ji(s.garbage_recycled_last_month()),
        "garbage_incinerated_last_month" => ji(s.last_month().incinerated),
        "garbage_wte_last_month" => ji(s.last_month().wte),
        "garbage_landfilled_last_month" => ji(s.last_month().landfilled),
        "incinerator_count" => ji(s.incinerator_count()),
        "wte_building_count" => ji(s.wte_building_count()),
        "recycling_center_count" => ji(s.recycling_center_count()),
        "water_treatment_plant_count" => ji(s.water_treatment_plant_count()),
        "garbage_buildings" => Json::Array(s.garbage_buildings().into_iter().map(|o| w.name(o)).collect()),
        "building_garbage_capacity" => ji(s.building_garbage_capacity(a_occ(w, op, 0)?)),
        "garbage_scaling_factor" => jf(s.garbage_scaling_factor()),
        "is_active_landfill" => Json::Bool(s.is_active_landfill(a_i32(op, 0)?, a_i32(op, 1)?)),
        "set_is_active_landfill" => {
            s.set_is_active_landfill(a_i32(op, 0)?, a_i32(op, 1)?, bool_of(arg(op, 2)?, "on")?);
            Json::Null
        }
        "garbage_put_in_landfill" => ji(s.garbage_put_in_landfill(a_i32(op, 0)?)),
        "max_capacity" => found_u(s.max_capacity(a_u32(op, 0)?, a_occ(w, op, 1)?)),
        "current_capacity" => found_u(s.current_capacity(a_u32(op, 0)?, a_occ(w, op, 1)?)),
        "recycling_center_max_capacity" => found_f(s.recycling_center_max_capacity(a_occ(w, op, 0)?)),
        "recycling_center_current_capacity" => found_f(s.recycling_center_current_capacity(a_occ(w, op, 0)?)),
        "recycling_center_current_capacity_bits" => {
            let (ok, p) = s.recycling_center_current_capacity(a_occ(w, op, 0)?);
            Json::Array(vec![Json::Bool(ok), ji(p.to_bits())])
        }
        "set_occupant_list_used_capacity" => {
            // args: [list (0 water treatment, 1 incinerators, 2 waste-to-energy), total, used]
            s.set_occupant_list_used_capacity(a_u32(op, 0)? as usize, a_u32(op, 1)?, a_u32(op, 2)?);
            Json::Null
        }
        "current_occupant_condition" => found_f(s.current_occupant_condition(a_occ(w, op, 0)?)),
        "current_occupant_efficiency" => found_f(s.current_occupant_efficiency(a_occ(w, op, 0)?)),
        "incinerator_used_capacity" => found_u(s.incinerator_used_capacity(a_occ(w, op, 0)?)),
        "wte_used_capacity" => found_u(s.wte_used_capacity(a_occ(w, op, 0)?)),
        "incinerator_current_capacity" => found_u(s.incinerator_current_capacity(a_occ(w, op, 0)?)),
        "wte_current_capacity" => found_u(s.wte_current_capacity(a_occ(w, op, 0)?)),
        "water_treatment_max_capacity" => found_u(s.water_treatment_max_capacity(a_occ(w, op, 0)?)),
        "water_treatment_current_capacity" => found_u(s.water_treatment_current_capacity(a_occ(w, op, 0)?)),
        "water_treatment_used_capacity" => found_u(s.water_treatment_used_capacity(a_occ(w, op, 0)?)),
        "calc_occupant_maintenance_cost" => {
            let (ok, c) = s.calc_occupant_maintenance_cost(a_occ(w, op, 0)?);
            Json::Array(vec![Json::Bool(ok), ji(c)])
        }
        "max_level_for_ui" => ji(s.max_level_for_ui(a_u32(op, 0)?)),
        "requested_share" => ji(s.requested_share(a_i32(op, 0)?, a_i32(op, 1)?, a_i32(op, 2)?, a_i32(op, 3)?)),
        "requester_satisfaction_id" => ji(s.requester_satisfaction_id()),
        "uses_custom_prop_selector" => Json::Bool(s.uses_custom_prop_selector()),
        "select_prop" => ji(s.select_prop(&[], 0, 0)),
        other => return Err(format!("unknown pollution method '{other}'")),
    };
    check_expect(op, got)
}

// ---- state ------------------------------------------------------------------------------------------

fn state_json(w: &World, s: &PollutionState) -> Vec<(String, Json)> {
    let i = |k: &str, v: i64| (k.to_owned(), ji(v));
    let b = |k: &str, v: bool| (k.to_owned(), Json::Bool(v));
    let names = |l: &[u64]| Json::Array(l.iter().map(|&o| w.name(o)).collect());
    let it = &s.iterator;
    vec![
        b("initialized", s.initialized),
        b("loaded", s.loaded),
        i("cell_x", s.cell_x.into()),
        i("cell_z", s.cell_z.into()),
        i("tracts_per_day", s.tracts_per_day.into()),
        (
            "iterator".into(),
            Json::Object(vec![
                ("mode".into(), ji(it.mode)),
                (
                    "rect".into(),
                    Json::Array(vec![ji(it.x0), ji(it.z0), ji(it.x1), ji(it.z1)]),
                ),
                ("count".into(), ji(it.count)),
                ("index".into(), ji(it.index)),
                ("table_len".into(), ji(it.table.len() as i64)),
                ("exhausted".into(), Json::Bool(it.is_exhausted())),
            ]),
        ),
        i("recycling_count", s.recycling_count.into()),
        i("avg_air", s.avg_air.into()),
        i("avg_water", s.avg_water.into()),
        i("avg_garbage", s.avg_garbage.into()),
        i("air_sum", s.air_sum.into()),
        i("water_sum_treated", s.water_sum_treated.into()),
        i("water_sum", s.water_sum.into()),
        i("treated_sum", s.treated_sum.into()),
        i("last_water_sum", s.last_water_sum.into()),
        i("last_treated_sum", s.last_treated_sum.into()),
        i("developed_tracts", s.developed_tracts.into()),
        i("radiation_sum", s.radiation_sum.into()),
        i("incinerator_capacity", s.incinerator_capacity.into()),
        i("wte_capacity", s.wte_capacity.into()),
        ("recycling_effect".into(), jf(s.recycling_effect)),
        b("garbage_uncollected", s.garbage_uncollected),
        i("total_uncollected", s.total_uncollected.into()),
        i("produced", s.month.produced),
        i("imported", s.month.imported),
        i("exported", s.month.exported),
        ("recycled".into(), Json::Float(s.month.recycled)),
        i("incinerated", s.month.incinerated),
        i("wte", s.month.wte),
        i("landfilled", s.month.landfilled),
        i("last_produced", s.last_month.produced),
        i("last_imported", s.last_month.imported),
        i("last_exported", s.last_month.exported),
        ("last_recycled".into(), Json::Float(s.last_month.recycled)),
        i("last_incinerated", s.last_month.incinerated),
        i("last_wte", s.last_month.wte),
        i("last_landfilled", s.last_month.landfilled),
        i("export_capacity", s.export_capacity.into()),
        i("decomposed", s.decomposed.into()),
        i("decomposed_last_month", s.decomposed_last_month.into()),
        i("active_landfill_cells", s.active_landfill_cells.into()),
        i("unused64", s.unused64),
        i("last_month_cost", s.last_month_cost),
        b("dirty_radiation", s.dirty_radiation),
        b("dirty_air", s.dirty_air),
        b("dirty_water", s.dirty_water),
        b("dirty_garbage", s.dirty_garbage),
        b("effects_visible", s.effects_visible),
        ("worst_air".into(), worst(s.worst_air)),
        ("worst_water".into(), worst(s.worst_water)),
        ("worst_garbage".into(), worst(s.worst_garbage)),
        ("efficiency".into(), jf(s.efficiency)),
        i("efficiency_bits", s.efficiency.to_bits().into()),
        i("garbage_supply", s.garbage_supply.into()),
        i("garbage_capacity", s.garbage_capacity.into()),
        ("department".into(), s.department.map_or(Json::Null, |d| w.name(d))),
        ("wtp".into(), names(&s.lists[0])),
        ("incinerators".into(), names(&s.lists[1])),
        ("wte_plants".into(), names(&s.lists[2])),
        ("recycling".into(), names(&s.lists[3])),
        i("blocks_x", s.blocks_x.into()),
        i("blocks_z", s.blocks_z.into()),
        (
            "grids".into(),
            Json::Array(s.grids.iter().map(|g| Json::Bool(g.is_some())).collect()),
        ),
    ]
}

fn expect(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?.snapshot();
    let got = state_json(w, &s);
    let want = field(op, "state")?;
    let (nested, flat): (Vec<_>, Vec<_>) = want
        .as_object()
        .ok_or("'state' must be an object")?
        .iter()
        .cloned()
        .partition(|(k, _)| k == "iterator");
    for (k, v) in nested {
        let g = got
            .iter()
            .find(|(gk, _)| *gk == k)
            .map(|e| &e.1)
            .ok_or("no such field")?;
        if !scalar::matches(g, &v, true) {
            return Err(format!("{k}: got {}, expected {}", scalar::show(g), scalar::show(&v)));
        }
    }
    check_state(&got, &Json::Object(flat))
}

/// `pollution_set`: pokes state fields (test setup).
fn set(w: &World, op: &Json) -> R<()> {
    let sim = w.sim()?;
    let fields = field(op, "state")?
        .as_object()
        .ok_or("state must be an object")?
        .to_vec();
    let mut err = None;
    sim.with_state(|s| {
        for (k, v) in &fields {
            let r: R<()> = (|| {
                let int = || i64_of(v, k);
                let u = || u32_of(v, k);
                let bl = || bool_of(v, k);
                match k.as_str() {
                    "cell_x" => s.cell_x = u()?,
                    "cell_z" => s.cell_z = u()?,
                    "tracts_per_day" => s.tracts_per_day = u()?,
                    "recycling_count" => s.recycling_count = u()?,
                    "avg_air" => s.avg_air = int()? as i32,
                    "avg_water" => s.avg_water = int()? as i32,
                    "avg_garbage" => s.avg_garbage = int()? as u16,
                    "air_sum" => s.air_sum = int()? as i32,
                    "water_sum_treated" => s.water_sum_treated = int()? as i32,
                    "water_sum" => s.water_sum = int()? as i32,
                    "treated_sum" => s.treated_sum = int()? as i32,
                    "last_water_sum" => s.last_water_sum = int()? as i32,
                    "last_treated_sum" => s.last_treated_sum = int()? as i32,
                    "developed_tracts" => s.developed_tracts = u()?,
                    "radiation_sum" => s.radiation_sum = u()?,
                    "incinerator_capacity" => s.incinerator_capacity = int()? as i32,
                    "wte_capacity" => s.wte_capacity = int()? as i32,
                    "recycling_effect" => s.recycling_effect = f32_of(v, k)?,
                    "garbage_uncollected" => s.garbage_uncollected = bl()?,
                    "total_uncollected" => s.total_uncollected = u()?,
                    "produced" => s.month.produced = int()?,
                    "imported" => s.month.imported = int()?,
                    "exported" => s.month.exported = int()?,
                    "recycled" => s.month.recycled = f32_of(v, k)?.into(),
                    "incinerated" => s.month.incinerated = int()?,
                    "wte" => s.month.wte = int()?,
                    "landfilled" => s.month.landfilled = int()?,
                    "last_produced" => s.last_month.produced = int()?,
                    "last_imported" => s.last_month.imported = int()?,
                    "last_exported" => s.last_month.exported = int()?,
                    "last_recycled" => s.last_month.recycled = f32_of(v, k)?.into(),
                    "last_incinerated" => s.last_month.incinerated = int()?,
                    "last_wte" => s.last_month.wte = int()?,
                    "last_landfilled" => s.last_month.landfilled = int()?,
                    "export_capacity" => s.export_capacity = int()? as i32,
                    "decomposed" => s.decomposed = u()?,
                    "decomposed_last_month" => s.decomposed_last_month = u()?,
                    "active_landfill_cells" => s.active_landfill_cells = u()?,
                    "unused64" => s.unused64 = int()?,
                    "last_month_cost" => s.last_month_cost = int()?,
                    "dirty_radiation" => s.dirty_radiation = bl()?,
                    "dirty_air" => s.dirty_air = bl()?,
                    "dirty_water" => s.dirty_water = bl()?,
                    "dirty_garbage" => s.dirty_garbage = bl()?,
                    "effects_visible" => s.effects_visible = bl()?,
                    "efficiency" => s.efficiency = f32_of(v, k)?,
                    "garbage_supply" => s.garbage_supply = int()? as i32,
                    "garbage_capacity" => s.garbage_capacity = int()? as i32,
                    other => return Err(format!("pollution_set: unknown field '{other}'")),
                }
                Ok(())
            })();
            if let Err(e) = r {
                err = Some(e);
                return;
            }
        }
    });
    err.map_or(Ok(()), Err)
}

fn grid_of(w: &World, op: &Json) -> R<Rc<dyn TractGrid<i16>>> {
    let name = str_of(field(op, "grid")?, "grid")?;
    let i = GRID_NAMES
        .iter()
        .position(|&n| n == name)
        .ok_or_else(|| format!("unknown grid '{name}'"))?;
    w.sim()?.snapshot().grids[i]
        .clone()
        .ok_or_else(|| format!("the simulator has no {name} grid"))
}

/// A grid element from JSON: an integer, or `"0x8064"` for a raw value.
fn elem(v: &Json, k: &str) -> R<i16> {
    Ok(i64_of(v, k)? as i16)
}

/// Tracts of `tracts` (tract coordinates) and `cells` (cell coordinates).
fn tract_entries(g: &dyn TractGrid<i16>, op: &Json) -> R<Vec<((i32, i32), i16)>> {
    let mut out = Vec::new();
    if let Some(v) = opt(op, "tracts") {
        for e in arr(v, "tracts")? {
            let val = elem(field(e, "v")?, "v")?;
            for t in rect_of(e)? {
                out.push((t, val));
            }
        }
    }
    if let Some(v) = opt(op, "cells") {
        let geo = g.geometry();
        for e in arr(v, "cells")? {
            let val = elem(field(e, "v")?, "v")?;
            for (x, z) in rect_of(e)? {
                out.push((geo.cell_to_tract(x, z), val));
            }
        }
    }
    Ok(out)
}

/// `pollution_set_grid`: `grid`, `fill?`, `tracts?` / `cells?` (`[{x, z,
/// w?, d?, v}]`). Writes the grid directly (no dirty flags).
fn set_grid(w: &World, op: &Json) -> R<()> {
    let g = grid_of(w, op)?;
    if let Some(v) = opt(op, "fill") {
        g.fill(elem(v, "fill")?);
    }
    for ((x, z), v) in tract_entries(&*g, op)? {
        g.set(x, z, v);
    }
    Ok(())
}

/// `pollution_expect_grid`: `grid`, `tracts?` / `cells?`, `others?` (the
/// value of every tract not listed), `nonzero?` (count of tracts ≠ 0),
/// `size?` (tract counts).
fn expect_grid(w: &World, op: &Json) -> R<()> {
    let g = grid_of(w, op)?;
    let name = str_of(field(op, "grid")?, "grid")?;
    let geo = g.geometry();
    let entries = tract_entries(&*g, op)?;
    for &((x, z), v) in &entries {
        let got = g.get(x, z);
        if got != v {
            return Err(format!("{name}[{x}][{z}] = {got}, expected {v}"));
        }
    }
    if let Some(v) = opt(op, "size") {
        let (x, z) = pair_of(v, "size")?;
        if (i64::from(geo.count_x), i64::from(geo.count_z)) != (x, z) {
            return Err(format!("{name}: {} x {} tracts", geo.count_x, geo.count_z));
        }
    }
    let all = || (0..geo.count_x).flat_map(|x| (0..geo.count_z).map(move |z| (x, z)));
    if let Some(v) = opt(op, "others") {
        let want = elem(v, "others")?;
        for (x, z) in all() {
            if entries.iter().any(|e| e.0 == (x, z)) {
                continue;
            }
            let got = g.get(x, z);
            if got != want {
                return Err(format!("{name}[{x}][{z}] = {got}, expected {want}"));
            }
        }
    }
    if let Some(v) = opt(op, "nonzero") {
        let n = all().filter(|&(x, z)| g.get(x, z) != 0).count() as i64;
        if n != i64_of(v, "nonzero")? {
            return Err(format!("{name}: {n} non-zero tracts"));
        }
    }
    Ok(())
}

fn prop_json(v: &Variant) -> Json {
    let items = v.items();
    let one = |e: &Elem| match *e {
        Elem::F32(f) => jf(f),
        Elem::F64(f) => Json::Float(f),
        other => ji(other.as_i64()),
    };
    match v {
        Variant::Scalar(e) => one(e),
        _ => Json::Array(items.iter().map(one).collect()),
    }
}

/// `pollution_expect_occupant`: `occupant`, `props` (`{"0xID": value or
/// null for absent}`).
fn expect_occupant(w: &World, op: &Json) -> R<()> {
    let occ = w.occupant(field(op, "occupant")?)?;
    let occs = w.occupants.borrow();
    let o = occs.get(&occ).ok_or("no such occupant")?;
    if let Some(v) = opt(op, "writes") {
        for (k, want) in v.as_object().ok_or("writes must be an object")? {
            let id = Json::Str(k.clone()).as_i64().ok_or("bad property ID")? as u32;
            let got = o.writes.get(&id).copied().unwrap_or(0);
            if i64::from(got) != i64_of(want, k)? {
                return Err(format!("{} property {k}: written {got} times", o.label));
            }
        }
    }
    let empty = Vec::new();
    let props = match opt(op, "props") {
        Some(v) => v.as_object().ok_or("props must be an object")?,
        None => &empty,
    };
    for (k, want) in props {
        let id = Json::Str(k.clone()).as_i64().ok_or("bad property ID")? as u32;
        let got = o.props.get(id).map_or(Json::Null, prop_json);
        if !scalar::matches(&got, want, false) {
            return Err(format!(
                "{} property {k}: got {}, expected {}",
                o.label,
                scalar::show(&got),
                scalar::show(want)
            ));
        }
    }
    Ok(())
}

/// `pollution_expect_refs`: `{"label": count}` (occupants start at 1, the
/// test's own reference).
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

/// `pollution_expect_effects`: `{water, air, radiation, garbage}` (effects
/// held by the simulator), `live?` (effect objects not yet released).
fn expect_effects(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?.snapshot();
    for (i, k) in ["water", "air", "radiation", "garbage"].iter().enumerate() {
        if let Some(v) = opt(op, k) {
            let n = s.effects[i].iter().flatten().count() as i64;
            if n != i64_of(v, k)? {
                return Err(format!("{n} {k} effects"));
            }
        }
    }
    if let Some(v) = opt(op, "live") {
        let n = w.effects.borrow().len() as i64;
        if n != i64_of(v, "live")? {
            return Err(format!("{n} live effect objects"));
        }
    }
    Ok(())
}

/// `pollution_crt`: `seed?` (re-seeds the simulator's `rand()`), `mark?`
/// (remembers the generator), `rand_calls?` (the `rand()` calls since the
/// mark).
fn crt(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    if let Some(v) = opt(op, "seed") {
        s.set_crt(CrtRand::new(u32_of(v, "seed")?));
    }
    if let Some(v) = opt(op, "rand_calls") {
        let mut probe = w.crt_mark.get().ok_or("no mark")?;
        let want = i64_of(v, "rand_calls")?;
        let now = s.crt();
        let mut n = 0i64;
        while probe != now && n < 1_000_000 {
            probe.rand();
            n += 1;
        }
        if n != want {
            return Err(format!("{n} rand() calls since the mark, expected {want}"));
        }
    }
    if opt(op, "mark").and_then(boolean).unwrap_or(false) {
        w.crt_mark.set(Some(s.crt()));
    }
    Ok(())
}

/// `pollution_expect_fx`: the visual-effect generator equals a fresh one
/// seeded `seed` after `draws` draws.
fn expect_fx(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let seed = opt(op, "seed").map_or(Ok(1), |v| u32_of(v, "seed"))?;
    let draws = i64_of(field(op, "draws")?, "draws")?;
    let mut want = RzRandomSuper::new(seed, &seeds());
    for _ in 0..draws {
        want.next_u32();
    }
    let mut got = s.effect_rng();
    for _ in 0..8 {
        if got.next_u32() != want.next_u32() {
            return Err(format!("the effect generator is not at {draws} draws"));
        }
    }
    Ok(())
}

// ---- persistence ------------------------------------------------------------------------------------

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

/// `pollution_load`: `slot`, `expect?` (clean store load), `fx_seed?`.
fn load(w: &World, op: &Json) -> R<()> {
    let saved = w.saves.borrow().get(&slot(op)?).cloned().ok_or("no such slot")?;
    let seed = opt(op, "fx_seed").map_or(Ok(1), |v| u32_of(v, "fx_seed"))?;
    let me = w.rc();
    let mut factory = |class: u32, object_id: u32| -> Option<RefValue> {
        match class {
            CLSID_POLLUTION_SIMULATOR => Some(RefValue::Object(PollutionSim::new(
                me.clone() as Rc<dyn PollutionHost>,
                me.token("pollution"),
                seed,
                &seeds(),
            ) as Rc<dyn SaveObject>)),
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
    let sim = root
        .as_ref()
        .and_then(|v| v.object::<PollutionSim>())
        .ok_or("the root did not resolve to a pollution simulator")?;
    *w.sim.borrow_mut() = Some(sim);
    Ok(())
}

/// Resolves references while `pollution_read` reads one payload: grids
/// become fresh core grids (one per object ID), host objects the tokens they
/// had when saved, each resolution handing the reader one reference.
struct PayloadResolver<'a> {
    world: &'a World,
    hosts: HashMap<u32, u64>,
    grids: BTreeMap<(u32, u32), RefValue>,
}

impl RefResolver for PayloadResolver<'_> {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        match class_id {
            OCCUPANT_CLASS | DEPARTMENT_CLASS => {
                let token = *self.hosts.get(&object_id)?;
                self.world.add_ref(token);
                Some(RefValue::Host { token, class_id })
            }
            _ => {
                if let Some(v) = self.grids.get(&(class_id, object_id)) {
                    return Some(v.clone());
                }
                let v = RefValue::Object(create_for_clsid(class_id)?);
                self.grids.insert((class_id, object_id), v.clone());
                Some(v)
            }
        }
    }
}

/// `pollution_read`: `Read` of the simulator's payload in `slot`, outside
/// the store, by the current simulator (`into: "current"`, the default) or a
/// new one (`into: "new"`, made current); checks the result (`expect`).
fn read(w: &World, op: &Json) -> R<()> {
    let saved = w.saves.borrow().get(&slot(op)?).cloned().ok_or("no such slot")?;
    let tmp = scalar::Saved {
        pkg: saved.pkg.clone(),
        root: Vec::new(),
    };
    let payload = scalar::entry_payload(&tmp, CLSID_POLLUTION_SIMULATOR, 0)?;
    let sim = match opt(op, "into").map(|v| str_of(v, "into")).transpose()? {
        None | Some("current") => w.sim()?,
        Some("new") => {
            let me = w.rc();
            PollutionSim::new(me.clone() as Rc<dyn PollutionHost>, me.token("pollution"), 1, &seeds())
        }
        Some(other) => return Err(format!("into: '{other}' (current, new)")),
    };
    let mut res = PayloadResolver {
        world: w,
        hosts: saved.hosts.clone(),
        grids: BTreeMap::new(),
    };
    let mut r = ByteReader::with_resolver(&payload, &mut res);
    let ok = sim.read(&mut r);
    check_expect(op, Json::Bool(ok))?;
    *w.sim.borrow_mut() = Some(sim);
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
    let r = scalar::patch(&mut tmp, CLSID_POLLUTION_SIMULATOR, op);
    s.pkg = tmp.pkg;
    r
}

/// `pollution_expect_payload`: `slot`, `hex?`, `at?` (`[{offset, hex}]`),
/// `length?`.
fn expect_payload(w: &World, op: &Json) -> R<()> {
    let saves = w.saves.borrow();
    let s = saves.get(&slot(op)?).ok_or("no such slot")?;
    let tmp = scalar::Saved {
        pkg: s.pkg.clone(),
        root: Vec::new(),
    };
    let got = scalar::entry_payload(&tmp, CLSID_POLLUTION_SIMULATOR, 0)?;
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
