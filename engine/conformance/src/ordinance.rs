// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/ordinance.md`: a scripted city (ordinance
//! exemplars, LTEXT, registered classes, residential population, budget
//! departments, landfill/power/police/pollution stubs, casino lots) and ops
//! that drive the ordinance simulator and ordinance objects. See README.md
//! ("Ordinance ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::{Rc, Weak};

use fc4_core::ordinance::{
    msg, prop, Ordinance, OrdinanceHost, OrdinanceKind, OrdinanceSim, StandardOrdinance, CLSID_ORDINANCE_SIMULATOR,
    ORDINANCE_EXEMPLAR_GROUP,
};
use fc4_core::property::PropertyHolder;
use fc4_core::props::{exemplar_key, PROP_EXEMPLAR_TYPE};
use fc4_core::stream::{ByteReader, RefValue, ResKey, SaveObject};
use fc4_core::variant::{mem, Elem, Variant};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, f32_of, i64_of, jf, ji, opt, str_of, u32_of, EventLog, Saved, R,
};

/// The LTEXT resource type and the group of the shipped ordinance strings (§3.7).
const LTEXT_TYPE: u32 = 0x2026_960B;
const LTEXT_GROUP: u32 = 0x6A23_1EAA;

/// A scripted foreign `cISC4Ordinance` (a DLL plugin's own class).
struct Foreign {
    token: u64,
    id: u32,
    name: Vec<u8>,
    enact: i64,
    repeal: i64,
    monthly: i64,
    conditions: Cell<bool>,
    available: Cell<bool>,
    on: Cell<bool>,
    enabled: Cell<bool>,
    adjusted: Cell<i64>,
    misc: PropertyHolder,
}

impl Ordinance for Foreign {
    fn token(&self) -> u64 {
        self.token
    }
    fn init(&self) -> bool {
        true
    }
    fn shutdown(&self) -> bool {
        true
    }
    fn current_monthly_income(&self) -> i64 {
        self.monthly
    }
    fn id(&self) -> u32 {
        self.id
    }
    fn name(&self) -> Vec<u8> {
        self.name.clone()
    }
    fn description(&self) -> Vec<u8> {
        Vec::new()
    }
    fn year_first_available(&self) -> u32 {
        0
    }
    fn chance_availability(&self) -> f32 {
        0.0
    }
    fn enactment_income(&self) -> i64 {
        self.enact
    }
    fn retracment_income(&self) -> i64 {
        self.repeal
    }
    fn monthly_constant_income(&self) -> i64 {
        self.monthly
    }
    fn monthly_income_factor(&self) -> f32 {
        0.0
    }
    fn advisor_id(&self) -> u32 {
        0
    }
    fn misc_property(&self, id: u32) -> Option<Variant> {
        self.misc.get(id).cloned()
    }
    fn is_available(&self) -> bool {
        self.available.get()
    }
    fn is_on(&self) -> bool {
        self.available.get() && self.on.get()
    }
    fn is_enabled(&self) -> bool {
        self.enabled.get()
    }
    fn monthly_adjusted_income(&self) -> i64 {
        self.adjusted.get()
    }
    fn check_conditions(&self) -> bool {
        self.enabled.get() && self.conditions.get()
    }
    fn is_income_ordinance(&self) -> bool {
        false
    }
    fn simulate(&self) -> bool {
        self.adjusted.set(self.monthly);
        true
    }
    fn set_available(&self, available: bool) -> bool {
        self.available.set(available);
        self.adjusted.set(0);
        true
    }
    fn set_on(&self, on: bool) -> bool {
        self.on.set(on);
        true
    }
    fn set_enabled(&self, enabled: bool) -> bool {
        self.enabled.set(enabled);
        true
    }
    fn force_available(&self, available: bool) -> bool {
        self.available.set(available);
        true
    }
    fn force_on(&self, on: bool) -> bool {
        self.on.set(on);
        true
    }
    fn force_monthly_adjusted_income(&self, v: i64) -> bool {
        self.adjusted.set(v);
        true
    }
    fn save_ref(self: Rc<Self>) -> RefValue {
        RefValue::Host {
            token: self.token,
            class_id: self.id,
        }
    }
}

#[derive(Default)]
pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    sim: RefCell<Option<Rc<OrdinanceSim>>>,
    objects: RefCell<BTreeMap<String, Rc<dyn Ordinance>>>,
    standard: RefCell<BTreeMap<String, Rc<StandardOrdinance>>>,
    next_token: Cell<u64>,
    refs: RefCell<BTreeMap<u64, i64>>,
    exemplars: RefCell<Vec<(ResKey, Rc<PropertyHolder>)>>,
    ltext: RefCell<BTreeMap<(u32, u32, u32), Vec<u8>>>,
    /// Registered ordinance classes; `None` = all standard classes.
    classes: RefCell<Option<BTreeSet<u32>>>,
    year: Cell<Option<u32>>,
    population: Cell<i32>,
    budget: Cell<bool>,
    departments: RefCell<BTreeMap<u32, i64>>,
    lines: RefCell<BTreeMap<(u32, u32), i64>>,
    landfill: Cell<u32>,
    power_plants: Cell<i32>,
    police: Cell<u32>,
    recycling: Cell<f32>,
    lots: RefCell<Vec<(u64, u32)>>,
    view_control: Cell<u32>,
    saves: RefCell<BTreeMap<String, Saved>>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            next_token: Cell::new(0x100),
            year: Cell::new(Some(2000)),
            budget: Cell::new(true),
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

    fn sim(&self) -> R<Rc<OrdinanceSim>> {
        self.sim
            .borrow()
            .clone()
            .ok_or_else(|| "no ordinance simulator (ordinance_create)".into())
    }

    fn make(&self, class_id: u32) -> Option<Rc<StandardOrdinance>> {
        StandardOrdinance::new(self.rc() as Rc<dyn OrdinanceHost>, class_id, self.token())
    }
}

impl OrdinanceHost for World {
    fn exemplar_keys(&self) -> Vec<ResKey> {
        self.exemplars.borrow().iter().map(|(k, _)| *k).collect()
    }
    fn exemplar(&self, key: ResKey) -> Option<Rc<PropertyHolder>> {
        self.exemplars
            .borrow()
            .iter()
            .rev()
            .find(|(k, _)| *k == key)
            .map(|(_, h)| h.clone())
    }
    fn ltext(&self, key: ResKey) -> Option<Vec<u8>> {
        self.ltext
            .borrow()
            .get(&(key.type_id, key.group, key.instance))
            .cloned()
    }
    fn create_ordinance(&self, class_id: u32) -> Option<Rc<StandardOrdinance>> {
        if let Some(set) = &*self.classes.borrow() {
            if !set.contains(&class_id) {
                return None;
            }
        }
        self.make(class_id)
    }
    fn add_ref(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) += 1;
    }
    fn release(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) -= 1;
    }
    fn foreign_ordinance(&self, _token: u64) -> Option<Rc<dyn Ordinance>> {
        None
    }
    fn post(&self, msg_type: u32, data2: u32, data3: u32) {
        let name = match msg_type {
            msg::ORDINANCE_UPDATED => "OrdinanceUpdated",
            msg::AVAILABILITY_CHANGED => "AvailabilityChanged",
            msg::MONTHLY_DONE => "MonthlyDone",
            _ => "other",
        };
        self.events.push(
            "post",
            vec![
                ("type", Json::Str(name.into())),
                ("id", ji(data2)),
                ("value", ji(data3)),
            ],
        );
    }
    fn sim_year(&self) -> Option<u32> {
        self.year.get()
    }
    fn residential_population(&self) -> i32 {
        self.population.get()
    }
    fn has_budget_simulator(&self) -> bool {
        self.budget.get()
    }
    fn department_total_expenses(&self, dept: u32) -> Option<i64> {
        if !self.budget.get() {
            return None;
        }
        self.departments.borrow().get(&dept).copied()
    }
    fn line_item_full_expenses(&self, dept: u32, line: u32) -> Option<i64> {
        if !self.budget.get() || !self.departments.borrow().contains_key(&dept) {
            return None;
        }
        self.lines.borrow().get(&(dept, line)).copied()
    }
    fn landfill_capacity(&self) -> u32 {
        self.landfill.get()
    }
    fn power_plant_count(&self, power_type: i32) -> i32 {
        if power_type == 3 {
            self.power_plants.get()
        } else {
            0
        }
    }
    fn police_station_count(&self) -> u32 {
        self.police.get()
    }
    fn recycling_center_effect(&self) -> f32 {
        self.recycling.get()
    }
    fn lots_with_building_type(&self, building_type: u32) -> Vec<u64> {
        self.lots
            .borrow()
            .iter()
            .filter(|l| l.1 == building_type)
            .map(|l| l.0)
            .collect()
    }
    fn demolish_lots(&self, lots: &[u64]) {
        self.events.push(
            "demolish",
            vec![("lots", Json::Array(lots.iter().map(|&l| ji(l as i64)).collect()))],
        );
        self.lots.borrow_mut().retain(|l| !lots.contains(&l.0));
    }
    fn hide_conditional_building(&self, building_type: u32) {
        self.events.push("hide", vec![("building_type", ji(building_type))]);
    }
    fn remove_view_input_control(&self, control_id: u32) {
        if self.view_control.get() == control_id {
            self.view_control.set(0);
            self.events.push("remove_control", vec![("id", ji(control_id))]);
        }
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "ordinance", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("ordinance_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "exemplar" => add_exemplar(&w, op),
        "create" => {
            let t = w.token();
            *w.sim.borrow_mut() = Some(OrdinanceSim::new(w.rc() as Rc<dyn OrdinanceHost>, t));
            Ok(())
        }
        "init" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.init()))),
        "shutdown" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.shutdown()))),
        "message" => message(&w, op),
        "call" => call(&w, op),
        "new" => new_object(&w, op),
        "foreign" => foreign(&w, op),
        "object" => object_call(&w, op),
        "expect_object" => expect_object(&w, op),
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
        "record" => record(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    })
}

// ---- environment ----------------------------------------------------------------------------------

fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = op.get("year") {
        w.year.set(match v {
            Json::Null => None,
            v => Some(u32_of(v, "year")?),
        });
    }
    if let Some(v) = opt(op, "population") {
        w.population.set(i64_of(v, "population")? as i32);
    }
    if let Some(v) = opt(op, "budget") {
        w.budget.set(bool_of(v, "budget")?);
    }
    if let Some(v) = opt(op, "departments") {
        let mut m = BTreeMap::new();
        for (k, x) in v.as_object().ok_or("departments must be an object")? {
            m.insert(key_of(k)?, i64_of(x, k)?);
        }
        *w.departments.borrow_mut() = m;
    }
    if let Some(v) = opt(op, "line_items") {
        let mut m = BTreeMap::new();
        for (dk, lines) in v.as_object().ok_or("line_items must be an object")? {
            for (lk, x) in lines.as_object().ok_or("line_items entries must be objects")? {
                m.insert((key_of(dk)?, key_of(lk)?), i64_of(x, lk)?);
            }
        }
        *w.lines.borrow_mut() = m;
    }
    if let Some(v) = opt(op, "landfill_capacity") {
        w.landfill.set(u32_of(v, "landfill_capacity")?);
    }
    if let Some(v) = opt(op, "power_plants") {
        w.power_plants.set(i64_of(v, "power_plants")? as i32);
    }
    if let Some(v) = opt(op, "police") {
        w.police.set(u32_of(v, "police")?);
    }
    if let Some(v) = opt(op, "recycling") {
        w.recycling.set(f32_of(v, "recycling")?);
    }
    if let Some(v) = opt(op, "lots") {
        let mut l = Vec::new();
        for e in arr(v, "lots")? {
            l.push((
                i64_of(field(e, "lot")?, "lot")? as u64,
                u32_of(field(e, "building_type")?, "building_type")?,
            ));
        }
        *w.lots.borrow_mut() = l;
    }
    if let Some(v) = opt(op, "view_control") {
        w.view_control.set(u32_of(v, "view_control")?);
    }
    if let Some(v) = opt(op, "ltext") {
        for e in arr(v, "ltext")? {
            let ty = opt(e, "type").map_or(Ok(LTEXT_TYPE), |x| u32_of(x, "type"))?;
            let group = opt(e, "group").map_or(Ok(LTEXT_GROUP), |x| u32_of(x, "group"))?;
            let inst = u32_of(field(e, "instance")?, "instance")?;
            w.ltext.borrow_mut().insert(
                (ty, group, inst),
                str_of(field(e, "text")?, "text")?.as_bytes().to_vec(),
            );
        }
    }
    if let Some(v) = op.get("classes") {
        *w.classes.borrow_mut() = match v {
            Json::Null => None,
            v => Some(
                arr(v, "classes")?
                    .iter()
                    .map(|c| u32_of(c, "class"))
                    .collect::<R<BTreeSet<u32>>>()?,
            ),
        };
    }
    Ok(())
}

fn key_of(k: &str) -> R<u32> {
    Json::Str(k.to_owned())
        .as_i64()
        .map(|v| v as u32)
        .ok_or_else(|| format!("bad key '{k}'"))
}

/// `ordinance_exemplar`: an ordinance exemplar written with friendly names.
fn add_exemplar(w: &World, op: &Json) -> R<()> {
    let instance = u32_of(field(op, "instance")?, "instance")?;
    let group = opt(op, "group").map_or(Ok(ORDINANCE_EXEMPLAR_GROUP), |g| u32_of(g, "group"))?;
    let mut h = match opt(op, "props") {
        Some(p) => scalar::holder_of(p)?,
        None => PropertyHolder::new(),
    };
    let ty = opt(op, "exemplar_type").map_or(Ok(14), |t| u32_of(t, "exemplar_type"))?;
    h.add(PROP_EXEMPLAR_TYPE, Variant::Scalar(Elem::U32(ty)));
    match op.get("id") {
        Some(Json::Null) => {}
        Some(v) => h.add(prop::ID, Variant::Scalar(Elem::U32(u32_of(v, "id")?))),
        None => h.add(prop::ID, Variant::Scalar(Elem::U32(instance))),
    }
    let s32 = |v: &Json, k: &str| -> R<Variant> { Ok(Variant::Scalar(Elem::I32(i64_of(v, k)? as i32))) };
    for (name, id) in [
        ("enact", prop::ENACTMENT_INCOME),
        ("repeal", prop::RETRACMENT_INCOME),
        ("constant", prop::MONTHLY_CONSTANT_INCOME),
    ] {
        if let Some(v) = opt(op, name) {
            h.add(id, s32(v, name)?);
        }
    }
    for (name, id) in [
        ("factor", prop::MONTHLY_INCOME_FACTOR),
        ("chance", prop::CHANCE_AVAILABILITY),
    ] {
        if let Some(v) = opt(op, name) {
            h.add(id, Variant::Scalar(Elem::F32(f32_of(v, name)?)));
        }
    }
    for (name, id) in [("year", prop::YEAR_FIRST_AVAILABLE), ("advisor", prop::ADVISOR_ID)] {
        if let Some(v) = opt(op, name) {
            h.add(id, Variant::Scalar(Elem::U32(u32_of(v, name)?)));
        }
    }
    if let Some(v) = opt(op, "income") {
        h.add(
            prop::IS_INCOME_ORDINANCE,
            Variant::Scalar(Elem::Bool(bool_of(v, "income")?)),
        );
    }
    for (name, id) in [("name", prop::NAME), ("description", prop::DESCRIPTION)] {
        if let Some(v) = opt(op, name) {
            h.add(id, Variant::string(str_of(v, name)?.as_bytes()));
        }
    }
    // LTEXT keys: true = the shipped (0x2026960B, 0x6A231EAA, instance), or [type, group, instance].
    for (name, id) in [
        ("name_ltext", prop::NAME_KEY),
        ("description_ltext", prop::DESCRIPTION_KEY),
    ] {
        if let Some(v) = opt(op, name) {
            let words: Vec<u32> = match v {
                Json::Bool(true) => vec![LTEXT_TYPE, LTEXT_GROUP, instance],
                v => arr(v, name)?.iter().map(|x| u32_of(x, name)).collect::<R<_>>()?,
            };
            h.add(
                id,
                Variant::array(mem::UINT32, words.into_iter().map(Elem::U32).collect()),
            );
        }
    }
    let key = exemplar_key(group, instance);
    if opt(op, "replace").and_then(crate::runner::boolean).unwrap_or(false) {
        w.exemplars.borrow_mut().retain(|(k, _)| *k != key);
    }
    if opt(op, "remove").and_then(crate::runner::boolean).unwrap_or(false) {
        w.exemplars.borrow_mut().retain(|(k, _)| *k != key);
        return Ok(());
    }
    w.exemplars.borrow_mut().push((key, Rc::new(h)));
    Ok(())
}

// ---- calls ------------------------------------------------------------------------------------------

fn message(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let t = match field(op, "type")?.as_str() {
        Some("SimNewMonth") => msg::SIM_NEW_MONTH,
        Some("SimBegin") => msg::SIM_BEGIN,
        _ => u32_of(field(op, "type")?, "type")?,
    };
    check_expect(op, Json::Bool(s.handle_message(t)))
}

fn a_u32(op: &Json, i: usize) -> R<u32> {
    u32_of(arg(op, i)?, "argument")
}

fn a_bool(op: &Json, i: usize) -> R<bool> {
    bool_of(arg(op, i)?, "argument")
}

fn ids_json(r: (Vec<u32>, u32)) -> Json {
    Json::Object(vec![
        ("ids".into(), Json::Array(r.0.into_iter().map(ji).collect())),
        ("total".into(), ji(r.1)),
    ])
}

fn object_by_label(w: &World, label: &str) -> R<Rc<dyn Ordinance>> {
    w.objects
        .borrow()
        .get(label)
        .cloned()
        .ok_or_else(|| format!("no ordinance object '{label}'"))
}

fn call(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let m = str_of(field(op, "method")?, "method")?;
    let b = Json::Bool;
    let opt_id = |o: Option<Rc<dyn Ordinance>>| o.map_or(Json::Null, |o| ji(o.id()));
    let got = match m {
        "add_ordinance" => b(s.add_ordinance(object_by_label(w, str_of(arg(op, 0)?, "label")?)?)),
        "remove_ordinance" => {
            let o = object_by_label(w, str_of(arg(op, 0)?, "label")?)?;
            b(s.remove_ordinance(&*o))
        }
        "is_ordinance_available" => b(s.is_ordinance_available(a_u32(op, 0)?)),
        "is_ordinance_on" => b(s.is_ordinance_on(a_u32(op, 0)?)),
        "is_ordinance_available_but_off" => b(s.is_ordinance_available_but_off(a_u32(op, 0)?)),
        "set_ordinance_available" => b(s.set_ordinance_available(a_u32(op, 0)?, a_bool(op, 1)?)),
        "set_ordinance_on" => b(s.set_ordinance_on(a_u32(op, 0)?, a_bool(op, 1)?)),
        "available_count" => ji(s.available_count()),
        "available_array" => ids_json(s.available_array(a_u32(op, 0)? as usize)),
        "on_count" => ji(s.on_count()),
        "on_array" => ids_json(s.on_array(a_u32(op, 0)? as usize)),
        "is_ordinance_enabled" => b(s.is_ordinance_enabled(a_u32(op, 0)?)),
        "set_ordinance_enabled" => b(s.set_ordinance_enabled(a_u32(op, 0)?, a_bool(op, 1)?)),
        "id_array" => ids_json(s.id_array(a_u32(op, 0)? as usize)),
        "ordinance_id_by_name" => ji(s.ordinance_id_by_name(str_of(arg(op, 0)?, "name")?.as_bytes())),
        "ordinance_by_id" => opt_id(s.ordinance_by_id(a_u32(op, 0)?)),
        "ordinance_by_name" => opt_id(s.ordinance_by_name(str_of(arg(op, 0)?, "name")?.as_bytes())),
        "monthly_expense" => ji(s.monthly_expense()),
        "monthly_income" => ji(s.monthly_income()),
        "cumulative_factor" => jf(s.cumulative_factor(a_u32(op, 0)?, a_u32(op, 1)?)),
        "cumulative_offset" => ji(s.cumulative_offset(a_u32(op, 0)?, a_u32(op, 1)?)),
        "do_monthly_simulation" => {
            s.do_monthly_simulation();
            Json::Null
        }
        "simulation_begin" => {
            s.simulation_begin();
            Json::Null
        }
        other => return Err(format!("unknown ordinance simulator method '{other}'")),
    };
    check_expect(op, got)
}

/// `ordinance_new`: a free-standing standard ordinance of `class` (the class
/// factory), optionally keyed (`instance`, `group?`: SetKey).
fn new_object(w: &World, op: &Json) -> R<()> {
    let label = str_of(field(op, "ordinance")?, "ordinance")?.to_owned();
    let class = u32_of(field(op, "class")?, "class")?;
    let o = w
        .make(class)
        .ok_or_else(|| format!("0x{class:08X} is not a standard ordinance class"))?;
    if let Some(i) = opt(op, "instance") {
        let group = opt(op, "group").map_or(Ok(ORDINANCE_EXEMPLAR_GROUP), |g| u32_of(g, "group"))?;
        o.set_key(exemplar_key(group, u32_of(i, "instance")?));
    }
    w.standard.borrow_mut().insert(label.clone(), o.clone());
    w.objects.borrow_mut().insert(label, o);
    Ok(())
}

/// `ordinance_foreign`: a scripted plugin ordinance.
fn foreign(w: &World, op: &Json) -> R<()> {
    let label = str_of(field(op, "ordinance")?, "ordinance")?.to_owned();
    let g = |k: &str| opt(op, k).map_or(Ok(0), |v| i64_of(v, k));
    let flag = |k: &str, d: bool| opt(op, k).map_or(Ok(d), |v| bool_of(v, k));
    let o = Rc::new(Foreign {
        token: w.token(),
        id: u32_of(field(op, "id")?, "id")?,
        name: opt(op, "name")
            .map_or(Ok(""), |v| str_of(v, "name"))?
            .as_bytes()
            .to_vec(),
        enact: g("enact")?,
        repeal: g("repeal")?,
        monthly: g("monthly")?,
        conditions: Cell::new(flag("conditions", true)?),
        available: Cell::new(flag("available", false)?),
        on: Cell::new(flag("on", false)?),
        enabled: Cell::new(true),
        adjusted: Cell::new(0),
        misc: match opt(op, "props") {
            Some(p) => scalar::holder_of(p)?,
            None => PropertyHolder::new(),
        },
    });
    w.objects.borrow_mut().insert(label, o);
    Ok(())
}

/// The ordinance named by `ordinance` (a label) or `id` (from the simulator).
fn target(w: &World, op: &Json) -> R<Rc<dyn Ordinance>> {
    if let Some(l) = opt(op, "ordinance") {
        return object_by_label(w, str_of(l, "ordinance")?);
    }
    let id = u32_of(field(op, "id")?, "id")?;
    w.sim()?
        .ordinance_by_id(id)
        .ok_or_else(|| format!("no ordinance 0x{id:08X}"))
}

/// The standard ordinance named by `ordinance` or `id`.
fn standard(w: &World, op: &Json) -> R<Rc<StandardOrdinance>> {
    if let Some(l) = opt(op, "ordinance") {
        let l = str_of(l, "ordinance")?;
        return w
            .standard
            .borrow()
            .get(l)
            .cloned()
            .ok_or_else(|| format!("'{l}' is not a standard ordinance"));
    }
    let o = target(w, op)?;
    o.save_ref()
        .object::<StandardOrdinance>()
        .ok_or_else(|| "not a standard ordinance".into())
}

fn text(v: Vec<u8>) -> Json {
    Json::Str(String::from_utf8_lossy(&v).into_owned())
}

fn object_call(w: &World, op: &Json) -> R<()> {
    let m = str_of(field(op, "method")?, "method")?;
    let b = Json::Bool;
    if matches!(m, "set_key" | "from_data") {
        let o = standard(w, op)?;
        let group = opt(op, "group").map_or(Ok(ORDINANCE_EXEMPLAR_GROUP), |g| u32_of(g, "group"))?;
        let key = exemplar_key(group, a_u32(op, 0)?);
        let got = if m == "set_key" {
            o.set_key(key);
            b(true)
        } else {
            b(o.from_data(key))
        };
        return check_expect(op, got);
    }
    let o = target(w, op)?;
    let got = match m {
        "init" => b(o.init()),
        "shutdown" => b(o.shutdown()),
        "current_monthly_income" => ji(o.current_monthly_income()),
        "id" => ji(o.id()),
        "name" => text(o.name()),
        "description" => text(o.description()),
        "year_first_available" => ji(o.year_first_available()),
        "chance_availability" => jf(o.chance_availability()),
        "enactment_income" => ji(o.enactment_income()),
        "retracment_income" => ji(o.retracment_income()),
        "monthly_constant_income" => ji(o.monthly_constant_income()),
        "monthly_income_factor" => jf(o.monthly_income_factor()),
        "advisor_id" => ji(o.advisor_id()),
        "is_available" => b(o.is_available()),
        "is_on" => b(o.is_on()),
        "is_enabled" => b(o.is_enabled()),
        "monthly_adjusted_income" => ji(o.monthly_adjusted_income()),
        "check_conditions" => b(o.check_conditions()),
        "is_income_ordinance" => b(o.is_income_ordinance()),
        "simulate" => b(o.simulate()),
        "set_available" => b(o.set_available(a_bool(op, 0)?)),
        "set_on" => b(o.set_on(a_bool(op, 0)?)),
        "set_enabled" => b(o.set_enabled(a_bool(op, 0)?)),
        "force_available" => b(o.force_available(a_bool(op, 0)?)),
        "force_on" => b(o.force_on(a_bool(op, 0)?)),
        "force_monthly_adjusted_income" => b(o.force_monthly_adjusted_income(i64_of(arg(op, 0)?, "v")?)),
        "has_misc_property" => b(o.misc_property(a_u32(op, 0)?).is_some()),
        other => return Err(format!("unknown ordinance method '{other}'")),
    };
    check_expect(op, got)
}

fn expect_object(w: &World, op: &Json) -> R<()> {
    let o = standard(w, op)?;
    let d = o.snapshot();
    let key = d.key.map_or(Json::Null, |k| {
        Json::Array(vec![ji(k.type_id), ji(k.group), ji(k.instance)])
    });
    let got = vec![
        ("class".to_owned(), ji(o.class_id())),
        ("plain".to_owned(), Json::Bool(o.kind() == OrdinanceKind::Plain)),
        ("initialized".to_owned(), Json::Bool(d.initialized)),
        ("key".to_owned(), key),
        ("id".to_owned(), ji(d.id)),
        ("name".to_owned(), text(d.name)),
        ("description".to_owned(), text(d.description)),
        ("year_first_available".to_owned(), ji(d.year_first_available)),
        ("chance_availability".to_owned(), jf(d.chance_availability)),
        ("enactment_income".to_owned(), ji(d.enactment_income)),
        ("retracment_income".to_owned(), ji(d.retracment_income)),
        ("monthly_constant_income".to_owned(), ji(d.monthly_constant_income)),
        ("monthly_income_factor".to_owned(), jf(d.monthly_income_factor)),
        ("advisor_id".to_owned(), ji(d.advisor_id)),
        ("is_income_ordinance".to_owned(), Json::Bool(d.is_income_ordinance)),
        ("available".to_owned(), Json::Bool(d.available)),
        ("on".to_owned(), Json::Bool(d.on)),
        ("enabled".to_owned(), Json::Bool(d.enabled)),
        ("adjusted".to_owned(), ji(d.adjusted)),
        (
            "misc_parent".to_owned(),
            d.misc.parent_key().map_or(Json::Null, |k| ji(k.instance)),
        ),
    ];
    check_state(&got, field(op, "state")?)
}

fn expect_sim(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?.snapshot();
    let got = vec![
        ("initialized".to_owned(), Json::Bool(s.initialized)),
        ("loaded_from_save".to_owned(), Json::Bool(s.loaded_from_save)),
        ("keys".to_owned(), Json::Array(s.keys.iter().map(|&k| ji(k)).collect())),
        ("monthly_expense".to_owned(), ji(s.monthly_expense)),
        ("monthly_income".to_owned(), ji(s.monthly_income)),
        ("pending".to_owned(), ji(s.pending)),
    ];
    check_state(&got, field(op, "state")?)
}

/// `ordinance_expect_refs`: `{"label" or "0xID": count}` reference counts.
fn expect_refs(w: &World, op: &Json) -> R<()> {
    for (k, v) in field(op, "refs")?.as_object().ok_or("refs must be an object")? {
        let token = match w.objects.borrow().get(k) {
            Some(o) => o.token(),
            None => {
                let id = key_of(k)?;
                w.sim()?
                    .ordinance_by_id(id)
                    .ok_or_else(|| format!("no ordinance {k}"))?
                    .token()
            }
        };
        let got = w.refs.borrow().get(&token).copied().unwrap_or(0);
        if got != i64_of(v, k)? {
            return Err(format!("{k}: {got} references, expected {}", i64_of(v, k)?));
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
        Some("simulator") => Ok(CLSID_ORDINANCE_SIMULATOR),
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
        if class == CLSID_ORDINANCE_SIMULATOR {
            return Some(RefValue::Object(
                OrdinanceSim::new(me.clone() as Rc<dyn OrdinanceHost>, me.token()) as Rc<dyn SaveObject>,
            ));
        }
        me.make(class).map(|o| RefValue::Object(o as Rc<dyn SaveObject>))
    };
    let (ok, roots) = scalar::load(&saved, &mut factory);
    let want = opt(op, "expect").map_or(Ok(true), |v| bool_of(v, "expect"))?;
    if ok != want {
        return Err(format!("store load clean = {ok}, expected {want}"));
    }
    let sim = roots
        .first()
        .and_then(|r| r.object::<OrdinanceSim>())
        .ok_or("the root did not resolve to an ordinance simulator")?;
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
            hex_encode(&got),
            hex_encode(&want)
        ));
    }
    Ok(())
}

/// `ordinance_record`: with `read: true`, reads `hex` into the ordinance
/// (`expect` = the result); otherwise the ordinance's record must equal
/// `hex`.
fn record(w: &World, op: &Json) -> R<()> {
    let o = standard(w, op)?;
    if opt(op, "read").and_then(crate::runner::boolean).unwrap_or(false) {
        let bytes = hex_decode(str_of(field(op, "hex")?, "hex")?)?;
        let mut r = ByteReader::new(&bytes);
        return check_expect(op, Json::Bool(o.read(&mut r)));
    }
    let got = scalar::record_bytes(&*o);
    let want = hex_decode(str_of(field(op, "hex")?, "hex")?)?;
    if got != want {
        return Err(format!(
            "ordinance record differs:\n    got      {}\n    expected {}",
            hex_encode(&got),
            hex_encode(&want)
        ));
    }
    Ok(())
}
