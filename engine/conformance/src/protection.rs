// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/protection.md`: a scripted city
//! (occupants with properties, budget items, lots, zones, terrain, the
//! dispatch manager, the police coverage the fire end of cycle reads), a
//! scriptable generator, and ops that drive the fire and police simulators.
//! See README.md ("Protection ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::rc::{Rc, Weak};

use fc4_core::grid::{create_for_clsid, CityDims, GridProvider, SimGrid, TractGrid};
use fc4_core::property::PropertyHolder;
use fc4_core::protection::{
    adjusted_value, choose_base_call_number, combine_strike_chance, dispatch_from_station_mask, escapees,
    jail_capacity, jailed_percentage, msg, occupant_coverage, query_interface, ratio_share, rehabilitation,
    reset_unjailed, revoke_dispatch_mask, scale_effect, shipped_tuning, unit_mask, BudgetItem, CoverageOutputs,
    DispatchInfo, Effect, ObjRef, ProtectionHost, ProtectionInterface, ProtectionKind, ProtectionMessage,
    ProtectionRandom, ProtectionRng, ProtectionSim, SlotReturn, CLSID_ADJUSTED_GRID, CLSID_UNADJUSTED_GRID, FIRE_SLOTS,
    OCCUPANT_TYPE_BUILDING, POLICE_SLOTS,
};
use fc4_core::rng::FixedSeeds;
use fc4_core::store::{save_store, Package};
use fc4_core::stream::{ByteReader, ByteWriter, OutStream, RefResolver, RefValue, SaveObject, SaveRegistry};
use fc4_core::variant::Variant;

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, f32_of, i64_of, jf, ji, opt, str_of, u32_of, EventLog, Saved, R,
};

/// The class ID the fake world gives its occupants (stations, jails,
/// automata) in saves.
const OCCUPANT_CLASS: u32 = 0x0CC0_0001;
/// The class ID of the fake departments.
const DEPARTMENT_CLASS: u32 = 0x0DE0_0001;
/// Department tokens are `DEPARTMENT_TOKEN + department ID`.
const DEPARTMENT_TOKEN: u64 = 1 << 32;
/// Air dispatch source tokens: fire, police.
const AIR_TOKENS: [u64; 2] = [0xA000_0001, 0xA000_0002];

/// A scripted generator (as the crime ops): queued values, each queue
/// repeating its last value (default 0.5). `range` returns `a + u·(b − a)`.
struct Stub {
    world: Weak<World>,
    uniform: VecDeque<f64>,
    range: VecDeque<f64>,
    last: (f64, f64),
}

impl Stub {
    fn log(&self, fields: Vec<(&str, Json)>) {
        if let Some(w) = self.world.upgrade() {
            w.events.push("rng", fields);
        }
    }
}

impl ProtectionRandom for Stub {
    fn uniform_f64(&mut self) -> f64 {
        let u = self.uniform.pop_front().unwrap_or(self.last.0);
        self.last.0 = u;
        self.log(vec![("kind", Json::Str("uniform".into())), ("value", Json::Float(u))]);
        u
    }
    fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        let u = self.range.pop_front().unwrap_or(self.last.1);
        self.last.1 = u;
        let v = a + u * (b - a);
        self.log(vec![
            ("kind", Json::Str("range".into())),
            ("lo", Json::Float(a)),
            ("hi", Json::Float(b)),
            ("value", Json::Float(v)),
        ]);
        v
    }
}

#[derive(Clone, Default)]
struct Occupant {
    occ_type: u32,
    pos: [f32; 3],
    prop: bool,
    /// `None`: no lot; else `HasRoadAccess`.
    road: Option<bool>,
    props: BTreeMap<u32, Variant>,
    /// Budget items by purpose: (locally funded, local funding, cost,
    /// whether the item's department exists).
    items: BTreeMap<u32, (bool, f32, i64, bool)>,
}

#[derive(Clone)]
struct ProtSaved {
    saved: Saved,
    hosts: BTreeMap<(u32, u32), u64>,
}

pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    sim: RefCell<Option<Rc<ProtectionSim>>>,
    kind: Cell<ProtectionKind>,
    next_token: Cell<u64>,
    saves: RefCell<BTreeMap<String, ProtSaved>>,
    city: Cell<CityDims>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    occupants: RefCell<BTreeMap<u64, Occupant>>,
    dept_funding: RefCell<BTreeMap<u32, f32>>,
    departments_fail: Cell<bool>,
    day: Cell<u32>,
    zones: RefCell<BTreeMap<(i32, i32), u32>>,
    lots: RefCell<BTreeMap<u64, (i32, i32)>>,
    dispatches: RefCell<Vec<(u32, DispatchInfo)>>,
    police_pct: Cell<Option<f32>>,
    sea_level: Cell<f32>,
    altitude: Cell<f32>,
    disaster: Cell<bool>,
    refs: RefCell<BTreeMap<u64, i64>>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            me: RefCell::new(Weak::new()),
            events: EventLog::default(),
            sim: RefCell::new(None),
            kind: Cell::new(ProtectionKind::Fire),
            next_token: Cell::new(0x5100),
            saves: RefCell::new(BTreeMap::new()),
            city: Cell::new(CityDims::square(64)),
            tuning: RefCell::new(None),
            occupants: RefCell::new(BTreeMap::new()),
            dept_funding: RefCell::new(BTreeMap::new()),
            departments_fail: Cell::new(false),
            day: Cell::new(1000),
            zones: RefCell::new(BTreeMap::new()),
            lots: RefCell::new(BTreeMap::new()),
            dispatches: RefCell::new(Vec::new()),
            police_pct: Cell::new(None),
            sea_level: Cell::new(250.0),
            altitude: Cell::new(300.0),
            disaster: Cell::new(false),
            refs: RefCell::new(BTreeMap::new()),
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn sim(&self) -> R<Rc<ProtectionSim>> {
        self.sim
            .borrow()
            .clone()
            .ok_or_else(|| "no protection simulator (protection_create)".into())
    }

    fn token(&self) -> u64 {
        let t = self.next_token.get();
        self.next_token.set(t + 1);
        t
    }

    /// `rng`: `{seed}` (a real `cRZRandom`) or `{stub: {uniform, range}}`;
    /// none → an empty stub.
    fn rng(&self, spec: Option<&Json>) -> R<Box<dyn ProtectionRandom>> {
        if let Some(s) = spec.and_then(|s| opt(s, "seed")) {
            let seeds = FixedSeeds {
                wall_clock: 1,
                process_clock: 1,
            };
            return Ok(Box::new(ProtectionRng::with_seed(u32_of(s, "seed")?, &seeds)));
        }
        let stub = spec.and_then(|s| opt(s, "stub"));
        let floats = |k: &str| -> R<VecDeque<f64>> {
            match stub.and_then(|s| opt(s, k)) {
                None => Ok(VecDeque::new()),
                Some(v) => arr(v, k)?
                    .iter()
                    .map(|x| scalar::num(x).ok_or_else(|| format!("{k}: number")))
                    .collect(),
            }
        };
        Ok(Box::new(Stub {
            world: self.me.borrow().clone(),
            uniform: floats("uniform")?,
            range: floats("range")?,
            last: (0.5, 0.5),
        }))
    }

    fn obj(&self, token: u64) -> ObjRef {
        ObjRef {
            token,
            class_id: OCCUPANT_CLASS,
        }
    }

    fn grid_u8(&self) -> R<Rc<dyn TractGrid<u8>>> {
        self.sim()?.coverage_grid().ok_or_else(|| "no adjusted grid".into())
    }

    fn grid_i16(&self) -> R<Rc<dyn TractGrid<i16>>> {
        self.sim()?.unadjusted_grid().ok_or_else(|| "no unadjusted grid".into())
    }
}

fn kind_name(k: ProtectionKind) -> &'static str {
    match k {
        ProtectionKind::Fire => "fire",
        ProtectionKind::Police => "police",
    }
}

fn kind_of(j: &Json) -> R<ProtectionKind> {
    match str_of(j, "kind")? {
        "fire" => Ok(ProtectionKind::Fire),
        "police" => Ok(ProtectionKind::Police),
        other => Err(format!("unknown kind '{other}' (fire, police)")),
    }
}

fn ids_json(ids: &[u32]) -> Json {
    Json::Array(ids.iter().map(|&i| ji(i)).collect())
}

fn tokens_json(list: &[ObjRef]) -> Json {
    Json::Array(list.iter().map(|o| ji(o.token as i64)).collect())
}

fn opt_token(o: Option<ObjRef>) -> Json {
    o.map_or(Json::Null, |o| ji(o.token as i64))
}

impl GridProvider<u8> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        self.events.push(
            "create_grid",
            vec![
                ("type", Json::Str("u8".into())),
                ("instance", ji(instance)),
                ("size", ji(tract_size)),
            ],
        );
        Some(SimGrid::<u8>::create(&self.city.get(), instance, tract_size))
    }
}

impl GridProvider<i16> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<i16>>> {
        self.events.push(
            "create_grid",
            vec![
                ("type", Json::Str("i16".into())),
                ("instance", ji(instance)),
                ("size", ji(tract_size)),
            ],
        );
        Some(SimGrid::<i16>::create(&self.city.get(), instance, tract_size))
    }
}

impl ProtectionHost for World {
    fn acquire_services(&self, kind: ProtectionKind) {
        self.events
            .push("acquire_services", vec![("kind", Json::Str(kind_name(kind).into()))]);
    }
    fn release_services(&self, kind: ProtectionKind) {
        self.events
            .push("release_services", vec![("kind", Json::Str(kind_name(kind).into()))]);
    }
    fn add_agent(&self, name: &str, levels: &[u32], flags: u32) {
        for &l in levels {
            self.events.push(
                "add_agent",
                vec![("level", ji(l)), ("name", Json::Str(name.into())), ("flags", ji(flags))],
            );
        }
    }
    fn remove_agent(&self) {
        self.events.push("remove_agent", vec![]);
    }
    fn subscribe(&self, t: &[u32]) {
        self.events.push("subscribe", vec![("types", ids_json(t))]);
    }
    fn unsubscribe(&self, t: &[u32]) {
        self.events.push("unsubscribe", vec![("types", ids_json(t))]);
    }
    fn register_dispatch_source(&self) {
        self.events.push("register_dispatch", vec![]);
    }
    fn unregister_dispatch_source(&self) {
        self.events.push("unregister_dispatch", vec![]);
    }
    fn set_adjusted_instance(&self, g: &Rc<dyn TractGrid<u8>>, instance: u32) {
        if let Some(s) = g.save_ref().object::<SimGrid<u8>>() {
            s.set_instance_id(instance);
        }
        self.events.push(
            "set_instance",
            vec![("type", Json::Str("u8".into())), ("instance", ji(instance))],
        );
    }
    fn set_unadjusted_instance(&self, g: &Rc<dyn TractGrid<i16>>, instance: u32) {
        if let Some(s) = g.save_ref().object::<SimGrid<i16>>() {
            s.set_instance_id(instance);
        }
        self.events.push(
            "set_instance",
            vec![("type", Json::Str("i16".into())), ("instance", ji(instance))],
        );
    }
    fn shutdown_adjusted(&self, _g: &Rc<dyn TractGrid<u8>>) {
        self.events
            .push("shutdown_grid", vec![("type", Json::Str("u8".into()))]);
    }
    fn shutdown_unadjusted(&self, _g: &Rc<dyn TractGrid<i16>>) {
        self.events
            .push("shutdown_grid", vec![("type", Json::Str("i16".into()))]);
    }
    fn add_ref(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_default() += 1;
    }
    fn release(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_default() -= 1;
    }
    fn tuning(&self, _kind: ProtectionKind) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn create_department(&self, id: u32, group: u32, name: (u32, u32), max_funding: f32) -> Option<ObjRef> {
        self.events.push(
            "department",
            vec![
                ("id", ji(id)),
                ("group", ji(group)),
                ("name", ids_json(&[name.0, name.1])),
                ("max", jf(max_funding)),
            ],
        );
        if self.departments_fail.get() {
            return None;
        }
        let token = DEPARTMENT_TOKEN + u64::from(id);
        self.add_ref(token);
        Some(ObjRef {
            token,
            class_id: DEPARTMENT_CLASS,
        })
    }
    fn department_id(&self, dept: u64) -> u32 {
        dept.wrapping_sub(DEPARTMENT_TOKEN) as u32
    }
    fn department_funding(&self, dept: u64) -> f32 {
        self.dept_funding
            .borrow()
            .get(&self.department_id(dept))
            .copied()
            .unwrap_or(1.0)
    }
    fn add_locally_funded(&self, dept: u64, occupant: u64, purpose: u32) {
        self.events.push(
            "add_funded",
            vec![
                ("dept", ji(self.department_id(dept))),
                ("occupant", ji(occupant as i64)),
                ("purpose", ji(purpose)),
            ],
        );
    }
    fn remove_locally_funded(&self, dept: u64, occupant: u64, purpose: u32) {
        self.events.push(
            "remove_funded",
            vec![
                ("dept", ji(self.department_id(dept))),
                ("occupant", ji(occupant as i64)),
                ("purpose", ji(purpose)),
            ],
        );
    }
    fn set_local_funding(&self, dept: u64, occupant: u64, p: f32, purpose: u32) -> bool {
        self.events.push(
            "set_funding",
            vec![
                ("dept", ji(self.department_id(dept))),
                ("occupant", ji(occupant as i64)),
                ("p", jf(p)),
                ("purpose", ji(purpose)),
            ],
        );
        if let Some(o) = self.occupants.borrow_mut().get_mut(&occupant) {
            if let Some(i) = o.items.get_mut(&purpose) {
                i.1 = p;
            }
        }
        true
    }
    fn budget_item(&self, occupant: u64, purpose: u32) -> Option<BudgetItem> {
        let occ = self.occupants.borrow();
        let &(locally_funded, local_funding, cost, dept) = occ.get(&occupant)?.items.get(&purpose)?;
        let dept_id = match purpose {
            0xEA56_768A => fc4_core::protection::JAIL_DEPARTMENT,
            _ => self.kind.get().params().department,
        };
        Some(BudgetItem {
            locally_funded,
            local_funding,
            cost,
            department_funding: dept.then(|| self.dept_funding.borrow().get(&dept_id).copied().unwrap_or(1.0)),
        })
    }
    fn occupant_type(&self, occupant: u64) -> u32 {
        self.occupants.borrow().get(&occupant).map_or(0, |o| o.occ_type)
    }
    fn occupant_property(&self, occupant: u64, id: u32) -> Option<Variant> {
        self.occupants.borrow().get(&occupant)?.props.get(&id).cloned()
    }
    fn set_occupant_u32(&self, occupant: u64, id: u32, value: u32) {
        self.occupants
            .borrow_mut()
            .entry(occupant)
            .or_default()
            .props
            .insert(id, Variant::Scalar(fc4_core::variant::Elem::U32(value)));
    }
    fn set_occupant_u32s(&self, occupant: u64, id: u32, values: &[u32]) {
        let v = Variant::array(
            fc4_core::variant::mem::UINT32,
            values.iter().map(|&x| fc4_core::variant::Elem::U32(x)).collect(),
        );
        self.occupants
            .borrow_mut()
            .entry(occupant)
            .or_default()
            .props
            .insert(id, v);
    }
    fn occupant_position(&self, occupant: u64) -> [f32; 3] {
        self.occupants.borrow().get(&occupant).map_or([0.0; 3], |o| o.pos)
    }
    fn is_prop_occupant(&self, occupant: u64) -> bool {
        self.occupants.borrow().get(&occupant).is_some_and(|o| o.prop)
    }
    fn lot_road_access(&self, occupant: u64) -> Option<bool> {
        self.occupants.borrow().get(&occupant).and_then(|o| o.road)
    }
    fn sim_day(&self) -> u32 {
        self.day.get()
    }
    fn lot_location(&self, unknown: u64) -> Option<(i32, i32)> {
        self.lots.borrow().get(&unknown).copied()
    }
    fn cell_center_to_position(&self, cell_x: i32, cell_z: i32) -> (f32, f32) {
        let c = self.city.get();
        (
            (cell_x as f32 + 0.5) * c.cell_width_x,
            (cell_z as f32 + 0.5) * c.cell_width_z,
        )
    }
    fn zone_value(&self, cell_x: i32, cell_z: i32) -> u32 {
        self.zones.borrow().get(&(cell_x, cell_z)).copied().unwrap_or(0)
    }
    fn sea_level(&self) -> f32 {
        self.sea_level.get()
    }
    fn altitude(&self, _x: f32, _z: f32) -> f32 {
        self.altitude.get()
    }
    fn disaster_ongoing(&self) -> bool {
        self.disaster.get()
    }
    fn dispatch_count(&self, _t: u32) -> u32 {
        self.dispatches.borrow().len() as u32
    }
    fn dispatch_ids(&self, _t: u32) -> Vec<u32> {
        self.dispatches.borrow().iter().map(|d| d.0).collect()
    }
    fn dispatch_info(&self, id: u32) -> Option<DispatchInfo> {
        self.dispatches.borrow().iter().find(|d| d.0 == id).map(|d| d.1)
    }
    fn add_controllers(&self, group: &str, occupant: u64) {
        self.events.push(
            "add_controllers",
            vec![("group", Json::Str(group.into())), ("occupant", ji(occupant as i64))],
        );
    }
    fn remove_controllers(&self, group: &str, occupant: u64) {
        self.events.push(
            "remove_controllers",
            vec![("group", Json::Str(group.into())), ("occupant", ji(occupant as i64))],
        );
    }
    fn police_percent_coverage(&self) -> Option<f32> {
        self.police_pct.get()
    }
    fn record_history(&self, id: u32, value: i32) {
        self.events
            .push("history", vec![("series", ji(id)), ("value", ji(value))]);
    }
    fn post(&self, msg_type: u32, subject: Option<u64>) {
        let mut f = vec![("type", ji(msg_type))];
        if let Some(s) = subject {
            f.push(("subject", ji(s as i64)));
        }
        self.events.push("post", f);
    }
    fn create_air_source(&self, kind: ProtectionKind) -> Option<ObjRef> {
        let token = AIR_TOKENS[kind as usize];
        self.add_ref(token);
        self.events.push("air_create", vec![]);
        Some(ObjRef {
            token,
            class_id: kind.params().air_source_clsid,
        })
    }
    fn air_source_init(&self, source: u64) {
        self.events.push("air_init", vec![("source", ji(source as i64))]);
    }
    fn air_source_shutdown(&self, source: u64) {
        self.events.push("air_shutdown", vec![("source", ji(source as i64))]);
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "protection", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("protection_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "occupants" => occupants(&w, op),
        "create" => create(&w, op),
        "init" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.init()))),
        "shutdown" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.shutdown()))),
        "message" => message(&w, op),
        "call" => call(&w, op),
        "eval" => eval(op),
        "set" => set(&w, op),
        "expect" => expect(&w, op),
        "expect_occupant" => expect_occupant(&w, op),
        "expect_refs" => expect_refs(&w, op),
        "grid_set" => grid_set(&w, op),
        "expect_grid" => expect_grid(&w, op),
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

fn i32_of(j: &Json, what: &str) -> R<i32> {
    Ok(i64_of(j, what)? as i32)
}

fn pair(j: &Json, what: &str) -> R<(i32, i32)> {
    let a = arr(j, what)?;
    if a.len() != 2 {
        return Err(format!("'{what}' must be [x, z]"));
    }
    Ok((i32_of(&a[0], what)?, i32_of(&a[1], what)?))
}

fn vec3(j: &Json, what: &str) -> R<[f32; 3]> {
    let a = arr(j, what)?;
    if a.len() != 3 {
        return Err(format!("'{what}' must be [x, y, z]"));
    }
    Ok([f32_of(&a[0], what)?, f32_of(&a[1], what)?, f32_of(&a[2], what)?])
}

/// `[[x, z, v], …]`.
fn triples(j: &Json, what: &str) -> R<Vec<(i32, i32, f64)>> {
    arr(j, what)?
        .iter()
        .map(|e| {
            let a = arr(e, what)?;
            if a.len() != 3 {
                return Err(format!("'{what}' entries are [x, z, v]"));
            }
            let v = scalar::num(&a[2]).ok_or_else(|| format!("{what}: number"))?;
            Ok((i32_of(&a[0], what)?, i32_of(&a[1], what)?, v))
        })
        .collect()
}

fn key_id(k: &str) -> R<u64> {
    Json::Str(k.to_owned())
        .as_i64()
        .map(|v| v as u64)
        .ok_or_else(|| format!("bad ID '{k}'"))
}

fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "cells") {
        let (x, z) = pair(v, "cells")?;
        w.city.set(CityDims {
            cells_x: x,
            cells_z: z,
            ..CityDims::square(1)
        });
    }
    if let Some(v) = opt(op, "day") {
        w.day.set(u32_of(v, "day")?);
    }
    if let Some(v) = op.get("police_pct") {
        w.police_pct.set(match v {
            Json::Null => None,
            v => Some(f32_of(v, "police_pct")?),
        });
    }
    if let Some(v) = opt(op, "sea_level") {
        w.sea_level.set(f32_of(v, "sea_level")?);
    }
    if let Some(v) = opt(op, "altitude") {
        w.altitude.set(f32_of(v, "altitude")?);
    }
    if let Some(v) = opt(op, "disaster") {
        w.disaster.set(bool_of(v, "disaster")?);
    }
    if let Some(v) = opt(op, "departments_fail") {
        w.departments_fail.set(bool_of(v, "departments_fail")?);
    }
    if let Some(v) = opt(op, "dept_funding") {
        for (k, f) in v.as_object().ok_or("dept_funding must be an object")? {
            w.dept_funding
                .borrow_mut()
                .insert(key_id(k)? as u32, f32_of(f, "dept_funding")?);
        }
    }
    if let Some(v) = op.get("tuning") {
        let h = match v {
            Json::Null => None,
            Json::Str(s) if s == "shipped" => Some(shipped_tuning(w.kind.get())),
            Json::Str(s) if s == "empty" => Some(PropertyHolder::new()),
            _ => return Err("tuning is \"shipped\", \"empty\" or null".into()),
        };
        *w.tuning.borrow_mut() = h.map(Rc::new);
    }
    if let Some(p) = opt(op, "tuning_props") {
        let extra = scalar::holder_of(p)?;
        let mut h = w.tuning.borrow().as_deref().cloned().unwrap_or_default();
        for id in extra.ids() {
            if let Some(v) = extra.get(id) {
                h.add(id, v.clone());
            }
        }
        *w.tuning.borrow_mut() = Some(Rc::new(h));
    }
    if let Some(v) = opt(op, "zones") {
        w.zones.borrow_mut().clear();
        for e in arr(v, "zones")? {
            let x = i32_of(field(e, "x")?, "x")?;
            let z = i32_of(field(e, "z")?, "z")?;
            let wd = opt(e, "w").map_or(Ok(1), |v| i32_of(v, "w"))?;
            let d = opt(e, "d").map_or(Ok(1), |v| i32_of(v, "d"))?;
            if !(0..=4096).contains(&wd) || !(0..=4096).contains(&d) {
                return Err("rectangle too large".into());
            }
            let t = opt(e, "v").map_or(Ok(1), |v| u32_of(v, "v"))?;
            for i in x..x + wd {
                for j in z..z + d {
                    w.zones.borrow_mut().insert((i, j), t);
                }
            }
        }
    }
    if let Some(v) = opt(op, "lots") {
        for (k, l) in v.as_object().ok_or("lots must be an object")? {
            let id = key_id(k)?;
            match l {
                Json::Null => {
                    w.lots.borrow_mut().remove(&id);
                }
                l => {
                    w.lots.borrow_mut().insert(id, pair(l, "lot")?);
                }
            }
        }
    }
    if let Some(v) = opt(op, "dispatches") {
        let mut list = Vec::new();
        for e in arr(v, "dispatches")? {
            let id = u32_of(field(e, "id")?, "id")?;
            let pos = opt(e, "pos").map_or(Ok((0.0, 0.0)), |p| {
                let a = arr(p, "pos")?;
                if a.len() != 2 {
                    return Err("pos is [x, z]".to_string());
                }
                Ok((f32_of(&a[0], "pos")?, f32_of(&a[1], "pos")?))
            })?;
            list.push((
                id,
                DispatchInfo {
                    station: opt(e, "station").map_or(Ok(0), |v| u32_of(v, "station"))? as u64,
                    call_id: opt(e, "call").map_or(Ok(id), |v| u32_of(v, "call"))?,
                    active: opt(e, "active").map_or(Ok(true), |v| bool_of(v, "active"))?,
                    automaton: opt(e, "automaton").map_or(Ok(0), |v| u32_of(v, "automaton"))? as u64,
                    x: pos.0,
                    z: pos.1,
                },
            ));
        }
        *w.dispatches.borrow_mut() = list;
    }
    Ok(())
}

/// `protection_occupants`: `{"id": {...} or null}`; fields `type?`
/// (building), `pos?` ([x, y, z]), `prop?`, `road?` (true / false / null =
/// no lot), `props?` (a property map merged into the occupant's; a null
/// value removes the property), `items?` (`{"0xPURPOSE": {local, funded?
/// (true), cost? (1000), dept? (true)} or null}`), `station?` (a shorthand
/// `[kind, centre, outer, radius, units]` writing the kind's effect and unit
/// properties).
fn occupants(w: &World, op: &Json) -> R<()> {
    for (k, v) in op.as_object().ok_or("op must be an object")? {
        if k == "op" {
            continue;
        }
        let id = key_id(k)?;
        if matches!(v, Json::Null) {
            w.occupants.borrow_mut().remove(&id);
            continue;
        }
        let mut occ = w.occupants.borrow().get(&id).cloned().unwrap_or(Occupant {
            occ_type: OCCUPANT_TYPE_BUILDING,
            ..Occupant::default()
        });
        if let Some(t) = opt(v, "type") {
            occ.occ_type = u32_of(t, "type")?;
        }
        if let Some(p) = opt(v, "pos") {
            occ.pos = vec3(p, "pos")?;
        }
        if let Some(p) = opt(v, "prop") {
            occ.prop = bool_of(p, "prop")?;
        }
        if let Some(r) = v.get("road") {
            occ.road = match r {
                Json::Null => None,
                r => Some(bool_of(r, "road")?),
            };
        }
        if let Some(s) = opt(v, "station") {
            let a = arr(s, "station")?;
            if a.len() != 5 {
                return Err("station is [kind, centre, outer, radius, units]".into());
            }
            let p = kind_of(&a[0])?.params();
            use fc4_core::variant::Elem;
            occ.props
                .insert(p.effect_props[0], Variant::Scalar(Elem::I32(i32_of(&a[1], "centre")?)));
            occ.props
                .insert(p.effect_props[1], Variant::Scalar(Elem::I32(i32_of(&a[2], "outer")?)));
            occ.props
                .insert(p.effect_props[2], Variant::Scalar(Elem::F32(f32_of(&a[3], "radius")?)));
            occ.props
                .insert(p.unit_count_prop, Variant::Scalar(Elem::U32(u32_of(&a[4], "units")?)));
        }
        if let Some(p) = opt(v, "props") {
            for (pk, pv) in p.as_object().ok_or("props must be an object")? {
                let pid = key_id(pk)? as u32;
                match pv {
                    Json::Null => {
                        occ.props.remove(&pid);
                    }
                    pv => {
                        occ.props.insert(pid, scalar::variant_of(pv)?);
                    }
                }
            }
        }
        if let Some(items) = opt(v, "items") {
            for (ik, iv) in items.as_object().ok_or("items must be an object")? {
                let purpose = key_id(ik)? as u32;
                match iv {
                    Json::Null => {
                        occ.items.remove(&purpose);
                    }
                    iv => {
                        let local = f32_of(field(iv, "local")?, "local")?;
                        let funded = opt(iv, "funded").map_or(Ok(true), |v| bool_of(v, "funded"))?;
                        let cost = opt(iv, "cost").map_or(Ok(1000), |v| i64_of(v, "cost"))?;
                        let dept = opt(iv, "dept").map_or(Ok(true), |v| bool_of(v, "dept"))?;
                        occ.items.insert(purpose, (funded, local, cost, dept));
                    }
                }
            }
        }
        w.occupants.borrow_mut().insert(id, occ);
    }
    Ok(())
}

/// `protection_create`: `kind` (`fire` / `police`), `rng?`.
fn create(w: &World, op: &Json) -> R<()> {
    let kind = kind_of(field(op, "kind")?)?;
    w.kind.set(kind);
    if opt(op, "tuning").is_none() && w.tuning.borrow().is_none() {
        *w.tuning.borrow_mut() = Some(Rc::new(shipped_tuning(kind)));
    }
    let rng = w.rng(opt(op, "rng"))?;
    *w.sim.borrow_mut() = Some(ProtectionSim::new(
        w.rc() as Rc<dyn ProtectionHost>,
        kind,
        w.token(),
        rng,
    ));
    Ok(())
}

// ---- calls ------------------------------------------------------------------------------------------

fn message(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let t = match field(op, "type")?.as_str() {
        Some("SimBegin") => msg::SIM_BEGIN,
        Some("SimNewDay") => msg::SIM_NEW_DAY,
        Some("SimNewMonth") => msg::SIM_NEW_MONTH,
        Some("OccupantInserted") => msg::OCCUPANT_INSERTED,
        Some("OccupantRemoved") => msg::OCCUPANT_REMOVED,
        Some("OccupantChanged") => msg::OCCUPANT_STATE_CHANGED,
        Some("FundingChanged") => msg::FUNDING_CHANGED,
        Some("DispatchBegin") => msg::DISPATCH_BEGIN,
        Some("DispatchActive") => msg::DISPATCH_ACTIVE,
        Some("DispatchInactive") => msg::DISPATCH_INACTIVE,
        Some("DispatchEnd") => msg::DISPATCH_END,
        Some("ReloadTuning") => msg::RELOAD_TUNING,
        Some("Crime") => msg::CRIME,
        _ => u32_of(field(op, "type")?, "type")?,
    };
    let count = opt(op, "count").map_or(Ok(1), |v| i64_of(v, "count"))?;
    if !(0..=10_000).contains(&count) {
        return Err("count is 0..10000".into());
    }
    let m = ProtectionMessage {
        msg_type: t,
        data1: opt(op, "data1").map_or(Ok(0), |v| u32_of(v, "data1"))?,
        data2: opt(op, "data2").map_or(Ok(0), |v| u32_of(v, "data2"))?,
        occupant: opt(op, "occupant")
            .map(|v| u32_of(v, "occupant").map(|o| w.obj(u64::from(o))))
            .transpose()?,
        unknown: opt(op, "unknown")
            .map(|v| u32_of(v, "unknown").map(u64::from))
            .transpose()?,
    };
    for _ in 0..count {
        check_expect(op, Json::Bool(s.handle_message(m)))?;
    }
    Ok(())
}

fn a_u64(op: &Json, i: usize) -> R<u64> {
    Ok(u64::from(u32_of(arg(op, i)?, "argument")?))
}

fn a_u32(op: &Json, i: usize) -> R<u32> {
    u32_of(arg(op, i)?, "argument")
}

fn a_f32(op: &Json, i: usize) -> R<f32> {
    f32_of(arg(op, i)?, "argument")
}

fn a_i32(op: &Json, i: usize) -> R<i32> {
    i32_of(arg(op, i)?, "argument")
}

fn effect_json(ok: bool, e: &Effect) -> Json {
    Json::Object(vec![
        ("ok".into(), Json::Bool(ok)),
        ("x".into(), jf(e.x)),
        ("z".into(), jf(e.z)),
        ("radius".into(), jf(e.radius)),
        ("centre".into(), ji(e.centre)),
        ("outer".into(), ji(e.outer)),
        ("original_centre".into(), ji(e.original_centre)),
    ])
}

fn slot_json(rows: &[fc4_core::protection::SlotRow], slot: u32) -> Json {
    match rows.iter().find(|r| r.0 == slot) {
        None => Json::Bool(false),
        Some(&(_, name, pops, ret)) => Json::Object(vec![
            ("name".into(), Json::Str(name.into())),
            ("pops".into(), ji(pops)),
            (
                "ret".into(),
                Json::Str(
                    match ret {
                        SlotReturn::Al => "al",
                        SlotReturn::Eax => "eax",
                        SlotReturn::St0 => "st0",
                        SlotReturn::HiddenPtr => "hidden_ptr",
                        SlotReturn::EdxEax => "edx_eax",
                        SlotReturn::None => "none",
                    }
                    .into(),
                ),
            ),
        ]),
    }
}

fn marked(op: &Json, i: usize) -> R<Vec<ObjRef>> {
    // A caller list of `n` marker entries (token 0xFFFFFFFF), to show what is
    // left alone.
    let n = op
        .get("args")
        .and_then(|a| a.as_array())
        .and_then(|a| a.get(i))
        .map_or(Ok(0), |v| u32_of(v, "length"))?;
    Ok(vec![
        ObjRef {
            token: 0xFFFF_FFFF,
            class_id: 0
        };
        n.min(4096) as usize
    ])
}

fn call(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let m = str_of(field(op, "method")?, "method")?;
    let unit = |f: &dyn Fn()| {
        f();
        Json::Null
    };
    let b = Json::Bool;
    let got = match m {
        "add_ref" => ji(s.add_ref()),
        "release" => ji(s.release()),
        "query_interface" => match query_interface(s.kind(), a_u32(op, 0)?) {
            None => b(false),
            Some(i) => Json::Str(
                match i {
                    ProtectionInterface::Primary => "primary",
                    ProtectionInterface::MessageTarget => "message_target",
                    ProtectionInterface::DispatchSource => "dispatch_source",
                    ProtectionInterface::Serializable => "serializable",
                }
                .into(),
            ),
        },
        "class_id" => ji(s.class_id()),
        "slot" => slot_json(
            match s.kind() {
                ProtectionKind::Fire => &FIRE_SLOTS,
                ProtectionKind::Police => &POLICE_SLOTS,
            },
            a_u32(op, 0)?,
        ),
        "slot_count" => ji(match s.kind() {
            ProtectionKind::Fire => FIRE_SLOTS.len() as i64,
            ProtectionKind::Police => POLICE_SLOTS.len() as i64,
        }),
        "read_tuning" => unit(&|| s.read_tuning()),
        "curve" => {
            let t = s.tuning();
            let x = a_f32(op, 1)?;
            jf(match str_of(arg(op, 0)?, "curve")? {
                "funding" => t.funding_curve.eval(x),
                "strike" => t.strike_curve.eval(x),
                "jail" => t.jail_curve.eval(x),
                "jailbreak" => t.jailbreak_curve.eval(x),
                other => return Err(format!("unknown curve '{other}'")),
            })
        }
        "tuning" => {
            let t = s.tuning();
            Json::Object(vec![
                ("max_funding".into(), jf(t.max_funding)),
                ("strike_efficiency".into(), jf(t.strike_efficiency)),
                ("min_funding_to_end".into(), jf(t.min_funding_to_end)),
                ("max_strike_duration".into(), ji(t.max_strike_duration)),
                ("no_road_multiplier".into(), jf(t.no_road_multiplier)),
                ("rehab_rate".into(), jf(t.rehab_rate)),
                ("arrest_threshold".into(), ji(t.arrest_threshold)),
                (
                    "escapee_range".into(),
                    Json::Array(vec![jf(t.escapee_range.0), jf(t.escapee_range.1)]),
                ),
                ("end_jailbreak_capacity".into(), jf(t.end_jailbreak_capacity)),
                ("overflow_factor".into(), jf(t.overflow_factor)),
                ("funding_curve_empty".into(), b(t.funding_curve.is_empty())),
            ])
        }
        // effects, efficiency, coverage
        "funding_of" => jf(s.funding_of(a_u64(op, 0)?, a_u32(op, 1)?)),
        "station_efficiency" => jf(s.station_efficiency(a_u64(op, 0)?)),
        "station_effect" => {
            let (ok, e) = s.station_effect(a_u64(op, 0)?);
            effect_json(ok, &e)
        }
        "dispatch_effect" => {
            let (ok, e) = s.dispatch_effect(a_u64(op, 0)?);
            effect_json(ok, &e)
        }
        "occupant_coverage" => {
            // args: [occupant, p, cover_before?, station_before?]
            let get = |i: usize, d: f32| -> R<f32> {
                match op.get("args").and_then(|a| a.as_array()).and_then(|a| a.get(i)) {
                    None => Ok(d),
                    Some(v) => f32_of(v, "argument"),
                }
            };
            let mut out = CoverageOutputs {
                cover_radius: get(2, -1.0)?,
                station_radius: get(3, -1.0)?,
            };
            let ok = s.occupant_coverage(a_u64(op, 0)?, a_f32(op, 1)?, &mut out);
            Json::Object(vec![
                ("result".into(), b(ok)),
                ("cover".into(), jf(out.cover_radius)),
                ("station".into(), jf(out.station_radius)),
            ])
        }
        "station_dispatch_radius" => jf(s.station_dispatch_radius(a_u64(op, 0)?)),
        "average_coverage" => jf(s.average_coverage()),
        "percent_coverage" => jf(s.percent_coverage()),
        "is_station" => b(s.is_station(a_u64(op, 0)?)),
        "stations" => tokens_json(&s.stations()),
        "station_count" => ji(s.station_count()),
        // strikes
        "is_station_on_strike" => b(s.is_station_on_strike(a_u64(op, 0)?)),
        "update_striking_state" => b(s.update_striking_state(a_u64(op, 0)?)),
        "chance_of_strike" => jf(s.chance_of_strike()),
        "strike_pass" => unit(&|| s.strike_pass()),
        "begin_strike" => b(s.begin_strike(a_u64(op, 0)?)),
        "end_strike" => b(s.end_strike()),
        "is_on_strike" => b(s.is_on_strike()),
        "strike_start_date" => ji(s.strike_start_date()),
        "strike_length" => ji(s.strike_length()),
        "striking_stations" => {
            let mut list = marked(op, 0)?;
            let n = s.striking_stations(&mut list);
            Json::Object(vec![("count".into(), ji(n)), ("list".into(), tokens_json(&list))])
        }
        // budget
        "station_funding" => match s.station_funding(a_u64(op, 0)?) {
            None => b(false),
            Some((cost, p)) => Json::Array(vec![ji(cost), jf(p)]),
        },
        "set_station_funding" => b(s.set_station_funding(a_u64(op, 0)?, a_f32(op, 1)?)),
        "protection_station_funding" => s.protection_station_funding(a_u64(op, 0)?).map_or(b(false), jf),
        "department" => opt_token(s.department()),
        "jail_department" => opt_token(s.jail_department()),
        // dispatch source
        "dispatch_type" => ji(s.dispatch_type()),
        "can_handle_dispatch_type" => b(s.can_handle_dispatch_type(a_u32(op, 0)?)),
        "dispatches_at_station" => {
            let flags = op
                .get("args")
                .and_then(|a| a.as_array())
                .and_then(|a| a.get(1))
                .map_or(Ok(true), |v| bool_of(v, "flags"))?;
            let base = op
                .get("args")
                .and_then(|a| a.as_array())
                .and_then(|a| a.get(2))
                .map_or(Ok(true), |v| bool_of(v, "base"))?;
            match s.dispatches_at_station(a_u64(op, 0)?, flags, base) {
                None => b(false),
                Some((f, bs, c)) => Json::Array(vec![ji(f), ji(bs), ji(c)]),
            }
        }
        "dispatch_from_station" => s
            .dispatch_from_station(a_u64(op, 0)?, a_u32(op, 1)?)
            .map_or(b(false), ji),
        "revoke_dispatch_from_station" => b(s.revoke_dispatch_from_station(a_u64(op, 0)?, a_u32(op, 1)?)),
        "dispatch_from_nearest_station" => match s.dispatch_from_nearest_station(a_f32(op, 0)?, a_f32(op, 1)?) {
            None => Json::Null,
            Some((st, id)) => Json::Object(vec![("station".into(), ji(st.token as i64)), ("id".into(), ji(id))]),
        },
        "can_dispatch" => match s.can_dispatch(a_f32(op, 0)?, a_f32(op, 1)?) {
            None => b(false),
            Some(busy) => Json::Object(vec![("all_busy".into(), b(busy))]),
        },
        "total_dispatch_count" => ji(s.total_dispatch_count()),
        "max_possible_dispatch_count" => ji(s.max_possible_dispatch_count()),
        "dispatch_automata_properties" => {
            let (bv, c, d) = s.dispatch_automata_properties();
            Json::Array(vec![ji(bv), Json::Array(c.iter().map(|&v| jf(v)).collect()), ji(d)])
        }
        "set_station_availability_flags" => unit(&|| {
            s.set_station_availability_flags(a_u64(op, 0).unwrap_or_default(), a_u32(op, 1).unwrap_or_default())
        }),
        "set_station_base_call_number" => {
            unit(&|| s.set_station_base_call_number(a_u64(op, 0).unwrap_or_default(), a_u32(op, 1).unwrap_or_default()))
        }
        // grids and cycles
        "update_unadjusted" => unit(&|| s.update_unadjusted()),
        "update_both_grids_by_position" => {
            let (x, z, r) = (a_f32(op, 0)?, a_f32(op, 1)?, a_f32(op, 2)?);
            unit(&|| s.update_both_grids_by_position(x, z, r))
        }
        "do_daily" => {
            let n = op
                .get("args")
                .and_then(|a| a.as_array())
                .and_then(|a| a.first())
                .map_or(Ok(1), |v| u32_of(v, "days"))?;
            for _ in 0..n.min(100_000) {
                s.do_daily();
            }
            Json::Null
        }
        "do_monthly" => unit(&|| s.do_monthly()),
        "end_of_cycle" => unit(&|| s.end_of_cycle()),
        "process_cell" => unit(&|| s.process_cell(0, 0)),
        "coverage_grid_is_adjusted" => {
            let a = s.coverage_grid();
            let p = s.police_grid();
            b(match (a, p) {
                (Some(a), Some(p)) => Rc::ptr_eq(&a, &p),
                _ => false,
            })
        }
        // jails
        "is_jail" => b(s.is_jail(a_u64(op, 0)?)),
        "jail_capacity_of" => ji(s.jail_capacity_of(a_u64(op, 0)?)),
        "recalculate" => unit(&|| s.recalculate()),
        "jailed_percentage" => jf(s.jailed_percentage()),
        "jail_capacity" => ji(s.jail_capacity()),
        "jail_population" => ji(s.jail_population()),
        "criminal_count" => ji(s.criminal_count()),
        "arrest_count" => ji(s.arrest_count()),
        "jail_count" => ji(s.jail_count()),
        "jails" => tokens_json(&s.jails()),
        "jailbreak_buildings" => {
            let mut list = marked(op, 0)?;
            let n = s.jailbreak_buildings(&mut list);
            Json::Object(vec![("count".into(), ji(n)), ("list".into(), tokens_json(&list))])
        }
        "is_in_jailbreak" => b(s.is_in_jailbreak(a_u64(op, 0)?)),
        "chance_of_jailbreak" => jf(s.chance_of_jailbreak(a_u64(op, 0)?)),
        "begin_jailbreak" => b(s.begin_jailbreak(w.obj(a_u64(op, 0)?))),
        "end_jailbreak" => b(s.end_jailbreak(a_u64(op, 0)?)),
        "jail_upkeep_cost" => ji(s.jail_upkeep_cost(a_u64(op, 0)?)),
        "process_monthly_crime_stats" => b(s.process_monthly_crime_stats()),
        "reset_unjailed_criminals" => ji(s.reset_unjailed_criminals()),
        "station_arrest_count" => ji(s.station_arrest_count(a_u64(op, 0)?)),
        "station_crime_count" => ji(s.station_crime_count(a_u64(op, 0)?)),
        "station_stats" => Json::Array(s.station_stats(a_u64(op, 0)?).iter().map(|&v| ji(v)).collect()),
        other => return Err(format!("unknown protection method '{other}'")),
    };
    check_expect(op, got)
}

/// `protection_eval`: the pure functions of §5–§9 (`fn`, `args`, `expect`).
fn eval(op: &Json) -> R<()> {
    let f = str_of(field(op, "fn")?, "fn")?;
    let opt_pair = |v: Option<(u32, u32)>| v.map_or(Json::Bool(false), |(a, b)| Json::Array(vec![ji(a), ji(b)]));
    let got = match f {
        "adjusted_value" => ji(adjusted_value(a_i32(op, 0)? as i16, a_f32(op, 1)?)),
        "scale_effect" => {
            let e = Effect {
                centre: a_i32(op, 0)?,
                outer: a_i32(op, 1)?,
                radius: a_f32(op, 2)?,
                original_centre: a_i32(op, 0)?,
                ..Effect::default()
            };
            let s = scale_effect(e, a_f32(op, 3)?);
            Json::Array(vec![ji(s.centre), ji(s.outer), jf(s.radius), ji(s.original_centre)])
        }
        "occupant_coverage" => {
            // args: [centre, outer, radius, original_centre, p]
            let e = Effect {
                centre: a_i32(op, 0)?,
                outer: a_i32(op, 1)?,
                radius: a_f32(op, 2)?,
                original_centre: a_i32(op, 3)?,
                ..Effect::default()
            };
            let (c, r) = occupant_coverage(&e, a_f32(op, 4)?);
            Json::Array(vec![jf(c), jf(r)])
        }
        "combine_strike_chance" => {
            let c: Vec<f32> = arr(arg(op, 0)?, "chances")?
                .iter()
                .map(|v| f32_of(v, "chance"))
                .collect::<R<_>>()?;
            jf(combine_strike_chance(&c))
        }
        "unit_mask" => ji(unit_mask(a_u32(op, 0)?)),
        "dispatch_mask" => opt_pair(dispatch_from_station_mask(
            a_u32(op, 0)?,
            a_u32(op, 1)?,
            a_u32(op, 2)?,
            a_u32(op, 3)?,
        )),
        "revoke_mask" => revoke_dispatch_mask(a_u32(op, 0)?, a_u32(op, 1)?, a_u32(op, 2)?, a_u32(op, 3)?)
            .map_or(Json::Bool(false), ji),
        "base_call_number" => {
            let others: Vec<(u32, u32)> = arr(arg(op, 0)?, "others")?
                .iter()
                .map(|p| {
                    let a = arr(p, "others")?;
                    if a.len() != 2 {
                        return Err("others are [base, count]".to_string());
                    }
                    Ok((u32_of(&a[0], "base")?, u32_of(&a[1], "count")?))
                })
                .collect::<R<_>>()?;
            ji(choose_base_call_number(&others, a_u32(op, 1)?, a_u32(op, 2)?))
        }
        "rehabilitation" => {
            let (j, k) = rehabilitation(a_i32(op, 0)?, a_f32(op, 1)?);
            Json::Array(vec![ji(j), ji(k)])
        }
        "reset_unjailed" => {
            let (r, j) = reset_unjailed(a_i32(op, 0)?, a_i32(op, 1)?, a_i32(op, 2)?, a_f32(op, 3)?);
            Json::Array(vec![ji(r), ji(j)])
        }
        "jailed_percentage" => {
            let v = jailed_percentage(a_i32(op, 0)?, a_i32(op, 1)?);
            if v.is_nan() {
                Json::Str("nan".into())
            } else {
                jf(v)
            }
        }
        "ratio_share" => ji(ratio_share(a_u32(op, 0)?, a_f32(op, 1)?)),
        "escapees" => ji(escapees(a_u32(op, 0)?, scalar::num(arg(op, 1)?).ok_or("r: number")?)),
        "jail_capacity" => ji(jail_capacity(a_u32(op, 0)?, a_f32(op, 1)?)),
        other => return Err(format!("unknown protection fn '{other}'")),
    };
    check_expect(op, got)
}

fn set(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let want = field(op, "state")?.as_object().ok_or("'state' must be an object")?;
    let mut d = s.data_mut();
    for (k, v) in want {
        match k.as_str() {
            "tracts_per_day" => d.tracts_per_day = i32_of(v, k)?,
            "factor" => d.factor = f32_of(v, k)?,
            "counters" => {
                let a = arr(v, k)?;
                if a.len() != 3 {
                    return Err("counters are [zoned, sum, non_zero]".into());
                }
                d.counters.zoned = i32_of(&a[0], k)?;
                d.counters.sum = i32_of(&a[1], k)?;
                d.counters.non_zero = i32_of(&a[2], k)?;
            }
            "avg" => d.avg = f32_of(v, k)?,
            "pct" => d.pct = f32_of(v, k)?,
            "strike_flag" => d.strike_flag = bool_of(v, k)?,
            "strike_date" => d.strike_date = u32_of(v, k)?,
            "next_call" => d.next_call = u32_of(v, k)?,
            "capacity" => d.jail.capacity = i32_of(v, k)?,
            "population" => d.jail.population = i32_of(v, k)?,
            "jailed" => d.jail.jailed = i32_of(v, k)?,
            "arrests_last" => d.jail.arrests_last = i32_of(v, k)?,
            "arrests_this" => d.jail.arrests_this = i32_of(v, k)?,
            "unarrested_this" => d.jail.unarrested_this = i32_of(v, k)?,
            "crimes_last" => d.jail.crimes_last = i32_of(v, k)?,
            "crimes_this" => d.jail.crimes_this = i32_of(v, k)?,
            other => return Err(format!("unknown state field '{other}'")),
        }
    }
    Ok(())
}

fn expect(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let crt = s.crt().state();
    let d = s.data();
    let it = &d.iterator;
    let j = &d.jail;
    let f = |k: &str, v: f32| (k.to_owned(), jf(v));
    let i = |k: &str, v: i64| (k.to_owned(), ji(v));
    let bo = |k: &str, v: bool| (k.to_owned(), Json::Bool(v));
    let got = vec![
        bo("initialized", d.initialized),
        i("ref_count", d.ref_count.into()),
        bo("destroyed", d.destroyed),
        bo("loaded", d.loaded),
        ("department".into(), opt_token(d.department)),
        ("jail_department".into(), opt_token(j.jail_department)),
        ("purpose".into(), ji(d.purpose)),
        ("stations".into(), tokens_json(&d.stations)),
        i("tracts_per_day", d.tracts_per_day.into()),
        f("factor", d.factor),
        (
            "counters".into(),
            Json::Array(vec![ji(d.counters.zoned), ji(d.counters.sum), ji(d.counters.non_zero)]),
        ),
        f("avg", d.avg),
        f("pct", d.pct),
        bo("strike_flag", d.strike_flag),
        i("strike_date", d.strike_date.into()),
        i("next_call", d.next_call.into()),
        ("air_source".into(), opt_token(d.air_source)),
        ("jails".into(), tokens_json(&j.jails)),
        ("funded_jails".into(), tokens_json(&j.funded_jails)),
        ("jailbreaks".into(), tokens_json(&j.jailbreaks)),
        i("capacity", j.capacity.into()),
        i("population", j.population.into()),
        i("jailed", j.jailed.into()),
        i("arrests_last", j.arrests_last.into()),
        i("arrests_this", j.arrests_this.into()),
        i("unarrested_this", j.unarrested_this.into()),
        i("crimes_last", j.crimes_last.into()),
        i("crimes_this", j.crimes_this.into()),
        i("crt", crt.into()),
        (
            "iterator".into(),
            Json::Object(vec![
                ("state".into(), ji(it.state_raw)),
                ("mode".into(), ji(it.mode)),
                (
                    "rect".into(),
                    Json::Array(vec![ji(it.x0), ji(it.z0), ji(it.x1), ji(it.z1)]),
                ),
                ("count".into(), ji(it.count)),
                ("index".into(), ji(it.index)),
                ("current".into(), ji(it.current)),
                ("step".into(), ji(it.step)),
            ]),
        ),
    ];
    drop(d);
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

/// `protection_expect_occupant`: `id` and `props` (`{"0xID": value or
/// null}`: a number (first element), an array (all elements) or null
/// (absent)).
fn expect_occupant(w: &World, op: &Json) -> R<()> {
    let id = u64::from(u32_of(field(op, "id")?, "id")?);
    let props = field(op, "props")?.as_object().ok_or("props must be an object")?;
    for (k, want) in props {
        let pid = key_id(k)? as u32;
        let got = w.occupants.borrow().get(&id).and_then(|o| o.props.get(&pid).cloned());
        let got_json = match (&got, want) {
            (None, _) => Json::Null,
            (Some(v), Json::Array(_)) => Json::Array(v.items().iter().map(|e| Json::Float(e.as_f64())).collect()),
            (Some(v), _) => v.items().first().map_or(Json::Null, |e| Json::Float(e.as_f64())),
        };
        if !scalar::matches(&got_json, want, false) {
            return Err(format!(
                "occupant {id} property {k}: got {}, expected {}",
                scalar::show(&got_json),
                scalar::show(want)
            ));
        }
    }
    Ok(())
}

/// `protection_expect_refs`: `{"id": count}` of references the simulator
/// took and did not release (departments are `4294967296 + ID`; air
/// sources `0xA0000001` fire, `0xA0000002` police); `all_zero` checks every
/// count is 0.
fn expect_refs(w: &World, op: &Json) -> R<()> {
    let refs = w.refs.borrow();
    if opt(op, "all_zero").and_then(crate::runner::boolean).unwrap_or(false) {
        if let Some((t, n)) = refs.iter().find(|(_, &n)| n != 0) {
            return Err(format!("object {t} holds {n} references"));
        }
    }
    if let Some(v) = opt(op, "refs") {
        for (k, want) in v.as_object().ok_or("refs must be an object")? {
            let got = refs.get(&key_id(k)?).copied().unwrap_or(0);
            if got != i64_of(want, "refs")? {
                return Err(format!("object {k}: {got} references"));
            }
        }
    }
    Ok(())
}

/// `protection_grid_set`: `grid` (`adjusted`, `unadjusted`), `fill?`,
/// `cells?`.
fn grid_set(w: &World, op: &Json) -> R<()> {
    match str_of(field(op, "grid")?, "grid")? {
        "adjusted" => {
            let g = w.grid_u8()?;
            if let Some(v) = opt(op, "fill") {
                g.fill(u32_of(v, "fill")? as u8);
            }
            if let Some(v) = opt(op, "cells") {
                for (x, z, val) in triples(v, "cells")? {
                    g.set(x, z, val as u8);
                }
            }
        }
        "unadjusted" => {
            let g = w.grid_i16()?;
            if let Some(v) = opt(op, "fill") {
                g.fill(i64_of(v, "fill")? as i16);
            }
            if let Some(v) = opt(op, "cells") {
                for (x, z, val) in triples(v, "cells")? {
                    g.set(x, z, val as i16);
                }
            }
        }
        other => return Err(format!("unknown grid '{other}' (adjusted, unadjusted)")),
    }
    Ok(())
}

/// Reads one tract of a coverage grid as a number.
type CellReader = Box<dyn Fn(i32, i32) -> f64>;

/// `protection_expect_grid`: `grid` and any of `exists`, `counts`, `size`,
/// `instance`, `cells`, `others`.
fn expect_grid(w: &World, op: &Json) -> R<()> {
    let name = str_of(field(op, "grid")?, "grid")?;
    let s = w.sim()?;
    let (geo, get, instance): (_, CellReader, u32) = match name {
        "adjusted" => {
            let g = s.coverage_grid();
            if let Some(e) = opt(op, "exists") {
                if g.is_some() != bool_of(e, "exists")? {
                    return Err(format!("{name}: exists = {}", g.is_some()));
                }
            }
            let Some(g) = g else { return Ok(()) };
            let inst = g.save_ref().object::<SimGrid<u8>>().map_or(0, |s| s.instance_id());
            (g.geometry(), Box::new(move |x, z| f64::from(g.get(x, z))), inst)
        }
        "unadjusted" => {
            let g = s.unadjusted_grid();
            if let Some(e) = opt(op, "exists") {
                if g.is_some() != bool_of(e, "exists")? {
                    return Err(format!("{name}: exists = {}", g.is_some()));
                }
            }
            let Some(g) = g else { return Ok(()) };
            let inst = g.save_ref().object::<SimGrid<i16>>().map_or(0, |s| s.instance_id());
            (g.geometry(), Box::new(move |x, z| f64::from(g.get(x, z))), inst)
        }
        other => return Err(format!("unknown grid '{other}' (adjusted, unadjusted)")),
    };
    if let Some(v) = opt(op, "counts") {
        if (geo.count_x, geo.count_z) != pair(v, "counts")? {
            return Err(format!("{name}: counts ({}, {})", geo.count_x, geo.count_z));
        }
    }
    if let Some(v) = opt(op, "size") {
        if geo.tract_size != i32_of(v, "size")? {
            return Err(format!("{name}: tract size {}", geo.tract_size));
        }
    }
    if let Some(v) = opt(op, "instance") {
        if instance != u32_of(v, "instance")? {
            return Err(format!("{name}: instance 0x{instance:08X}"));
        }
    }
    let mut listed = std::collections::BTreeSet::new();
    if let Some(v) = opt(op, "cells") {
        for (x, z, want) in triples(v, "cells")? {
            listed.insert((x, z));
            let got = get(x, z);
            if got != want {
                return Err(format!("{name}[{x}][{z}] = {got}, expected {want}"));
            }
        }
    }
    if let Some(v) = opt(op, "others") {
        let want = scalar::num(v).ok_or("others: number")?;
        for x in 0..geo.count_x.clamp(0, 4096) {
            for z in 0..geo.count_z.clamp(0, 4096) {
                if !listed.contains(&(x, z)) && get(x, z) != want {
                    return Err(format!("{name}[{x}][{z}] = {}, expected {want} (unlisted)", get(x, z)));
                }
            }
        }
    }
    Ok(())
}

// ---- persistence ------------------------------------------------------------------------------------

fn slot(op: &Json) -> R<String> {
    Ok(str_of(field(op, "slot")?, "slot")?.to_owned())
}

fn save_now(w: &World) -> R<ProtSaved> {
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
        .map(|e| ((e.0, e.1), e.3))
        .collect();
    Ok(ProtSaved {
        saved: Saved {
            pkg,
            root: root.into_bytes(),
        },
        hosts,
    })
}

fn save(w: &World, op: &Json) -> R<()> {
    let s = save_now(w)?;
    w.saves.borrow_mut().insert(slot(op)?, s);
    Ok(())
}

fn is_host_class(class: u32) -> bool {
    matches!(class, OCCUPANT_CLASS | DEPARTMENT_CLASS)
        || class == ProtectionKind::Fire.params().air_source_clsid
        || class == ProtectionKind::Police.params().air_source_clsid
}

/// `protection_load`: creates the simulator (the current kind), the grids
/// (core `SimGrid`s) and the host objects (the tokens they had when saved)
/// through the store's factory, and makes the loaded simulator current.
fn load(w: &World, op: &Json) -> R<()> {
    let name = slot(op)?;
    let saved = w.saves.borrow().get(&name).cloned().ok_or("no such slot")?;
    let me = w.rc();
    let kind = w.kind.get();
    let rng = RefCell::new(Some(w.rng(opt(op, "rng"))?));
    let hosts = saved.hosts.clone();
    let mut factory = |class: u32, id: u32| -> Option<RefValue> {
        if class == kind.params().clsid {
            let r = rng.borrow_mut().take()?;
            return Some(RefValue::Object(
                ProtectionSim::new(me.clone() as Rc<dyn ProtectionHost>, kind, me.token(), r) as Rc<dyn SaveObject>,
            ));
        }
        match class {
            CLSID_ADJUSTED_GRID | CLSID_UNADJUSTED_GRID => create_for_clsid(class).map(RefValue::Object),
            c if is_host_class(c) => hosts
                .get(&(class, id))
                .map(|&token| RefValue::Host { token, class_id: class }),
            _ => None,
        }
    };
    let (ok, roots) = scalar::load(&saved.saved, &mut factory);
    let want = opt(op, "expect").map_or(Ok(true), |v| bool_of(v, "expect"))?;
    if ok != want {
        return Err(format!("store load clean = {ok}, expected {want}"));
    }
    let sim = roots
        .first()
        .and_then(|r| r.object::<ProtectionSim>())
        .ok_or("the root did not resolve to a protection simulator")?;
    *w.sim.borrow_mut() = Some(sim);
    Ok(())
}

/// Resolves references while `protection_read` reads one payload: grids
/// become fresh core grids, host objects the tokens they had when saved.
struct PayloadResolver(BTreeMap<(u32, u32), u64>, BTreeMap<(u32, u32), RefValue>);

impl RefResolver for PayloadResolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        if let Some(v) = self.1.get(&(class_id, object_id)) {
            return Some(v.clone());
        }
        let v = match class_id {
            CLSID_ADJUSTED_GRID | CLSID_UNADJUSTED_GRID => RefValue::Object(create_for_clsid(class_id)?),
            c if is_host_class(c) => RefValue::Host {
                token: *self.0.get(&(class_id, object_id))?,
                class_id,
            },
            _ => return None,
        };
        self.1.insert((class_id, object_id), v.clone());
        Some(v)
    }
}

/// `protection_read`: `Read` of the simulator's payload in `slot` by a new
/// simulator of the current kind, outside the store; checks the result
/// (`expect`) and the bytes consumed (`position`), then makes the new
/// simulator current. `hex` replaces the payload.
fn read(w: &World, op: &Json) -> R<()> {
    let kind = w.kind.get();
    let (payload, hosts) = match opt(op, "hex") {
        Some(h) => (hex_decode(str_of(h, "hex")?)?, BTreeMap::new()),
        None => {
            let saved = w.saves.borrow().get(&slot(op)?).cloned().ok_or("no such slot")?;
            (
                scalar::entry_payload(&saved.saved, kind.params().clsid, 0)?,
                saved.hosts.clone(),
            )
        }
    };
    let sim = ProtectionSim::new(
        w.rc() as Rc<dyn ProtectionHost>,
        kind,
        w.token(),
        w.rng(opt(op, "rng"))?,
    );
    let mut res = PayloadResolver(hosts, BTreeMap::new());
    let mut r = ByteReader::with_resolver(&payload, &mut res);
    let ok = sim.read(&mut r);
    let pos = payload.len() - r.remaining();
    check_expect(op, Json::Bool(ok))?;
    if let Some(v) = opt(op, "position") {
        if pos as i64 != i64_of(v, "position")? {
            return Err(format!("Read consumed {pos} bytes"));
        }
    }
    *w.sim.borrow_mut() = Some(sim);
    Ok(())
}

fn expect_save(w: &World, op: &Json) -> R<()> {
    let want = w.saves.borrow().get(&slot(op)?).cloned().ok_or("no such slot")?;
    scalar::same_save(&save_now(w)?.saved, &want.saved)
}

fn patch(w: &World, op: &Json) -> R<()> {
    let clsid = w.kind.get().params().clsid;
    let mut saves = w.saves.borrow_mut();
    let s = saves.get_mut(&slot(op)?).ok_or("no such slot")?;
    scalar::patch(&mut s.saved, clsid, op)
}

fn expect_payload(w: &World, op: &Json) -> R<()> {
    let clsid = w.kind.get().params().clsid;
    let saves = w.saves.borrow();
    let s = saves.get(&slot(op)?).ok_or("no such slot")?;
    let got = scalar::entry_payload(&s.saved, clsid, 0)?;
    if let Some(h) = opt(op, "hex") {
        let want = hex_decode(str_of(h, "hex")?)?;
        if got != want {
            return Err(format!(
                "payload differs:\n    got      {}\n    expected {}",
                hex_encode(&got),
                hex_encode(&want)
            ));
        }
    }
    if let Some(v) = opt(op, "at") {
        for e in arr(v, "at")? {
            let off = i64_of(field(e, "offset")?, "offset")? as usize;
            let want = hex_decode(str_of(field(e, "hex")?, "hex")?)?;
            let slice = got
                .get(off..off.saturating_add(want.len()))
                .ok_or("slice outside the payload")?;
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
