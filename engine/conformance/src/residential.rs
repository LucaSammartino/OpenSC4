// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/residential.md`: a scripted city (the
//! growth developers' population maps, the BDS building map, pollution
//! source values, the month and day, terrain, the effects manager, occupants
//! with groups, properties, lots and budget items, departments, line items,
//! ordinances), a scripted generator, and ops that drive the residential
//! simulator (`fc4_core::residential`), its education and health half (REH)
//! and its save record. See README.md ("Residential ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::{Rc, Weak};

use fc4_core::grid::{CityDims, CoreGrids, GridElem, GridProvider, ScanState, SimGrid, TractGeometry, TractGrid};
use fc4_core::property::PropertyHolder;
use fc4_core::residential::{
    age_bucket, msg, ordinance_modifiers, pct, query_interface, shipped_tuning, tracts_in_radius, BudgetItem,
    CreatedEffect, DepartmentBlock, EducationHealthHost, OccupantLists, QueryData, ResidentialHost, ResidentialMessage,
    ResidentialRandom, ResidentialRng, ResidentialSim, ResidentialState, SlotReturn, DEVELOPERS, GRID_TRIPLE_RESULT,
    HOSPITAL, OCCUPANT_TYPE_BUILDING, SCHOOL, SLOTS,
};
use fc4_core::rng::FixedSeeds;
use fc4_core::stream::{ByteReader, ByteWriter, RefResolver, RefValue, RegisteredObject, SaveObject, SaveRegistry};
use fc4_core::variant::{Elem, Variant};
use fc4_core::x87::rhe;

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{boolean, field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, f32_of, i64_of, jf, ji, opt, str_of, u32_of, EventLog, R,
};

/// The class ID the world writes for its objects (occupants, departments).
const HOST_CLASS: u32 = 0x0CC0_0002;

/// The class's generator, scripted: each `RandomDoubleRangeUniform(a, b)`
/// returns the next listed draw verbatim, or `b` when the list is empty
/// (which never starts an effect); with `session`, the session-seeded
/// `cRZRandom` instead. Every draw is logged as an `rng` event.
struct Stub {
    world: Weak<World>,
    draws: VecDeque<f64>,
    real: Option<ResidentialRng>,
}

impl ResidentialRandom for Stub {
    fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        let v = match &mut self.real {
            Some(r) => r.range_f64(a, b),
            None => self.draws.pop_front().unwrap_or(b),
        };
        if let Some(w) = self.world.upgrade() {
            w.events.push(
                "rng",
                vec![
                    ("lo", Json::Float(a)),
                    ("hi", Json::Float(b)),
                    ("value", Json::Float(v)),
                ],
            );
        }
        v
    }
}

/// One saved record: the bytes and the registry entries its references
/// point at.
#[derive(Clone)]
struct Record {
    bytes: Vec<u8>,
    entries: Vec<RegisteredObject>,
}

pub(crate) struct World {
    precision: Cell<fc4_core::x87::Precision>,
    me: RefCell<Weak<World>>,
    events: EventLog,
    sim: RefCell<Option<Rc<ResidentialSim>>>,
    city: Cell<CityDims>,
    dev_size: Cell<i32>,
    developers: RefCell<Vec<Rc<SimGrid<i16>>>>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    month: Cell<u32>,
    today: Cell<u32>,
    demand_node: Cell<bool>,
    message_server: Cell<bool>,
    simulator: Cell<bool>,
    pollution: Cell<[u16; 3]>,
    building_map: RefCell<Option<Rc<SimGrid<u8>>>>,
    effect: Cell<Option<u64>>,
    altitude: Cell<f32>,
    groups: RefCell<BTreeMap<u64, Vec<u32>>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    bool_props: RefCell<BTreeMap<(u64, u32), bool>>,
    next_department: Cell<u64>,
    /// Occupant properties other than the bool ones.
    props: RefCell<BTreeMap<(u64, u32), Variant>>,
    building_types: RefCell<BTreeMap<u64, u32>>,
    slot11: RefCell<BTreeMap<u64, u32>>,
    lots: RefCell<BTreeMap<u64, (f32, f32)>>,
    positions: RefCell<BTreeMap<u64, (f32, f32)>>,
    /// Budget items by (occupant, purpose).
    items: RefCell<BTreeMap<(u64, u32), BudgetItem>>,
    /// `GetDepartmentBudget`: department ID → token.
    departments: RefCell<BTreeMap<u32, u64>>,
    funding: RefCell<BTreeMap<u64, f32>>,
    locally_funded: RefCell<BTreeSet<(u64, u64, u32)>>,
    /// Line items by (department token, line), and their counts.
    line_items: RefCell<BTreeMap<(u64, u32), u64>>,
    line_counts: RefCell<BTreeMap<u64, i64>>,
    next_line_item: Cell<u64>,
    /// The ordinances that are on, by ID, with their misc properties.
    ordinances: RefCell<BTreeMap<u32, Rc<PropertyHolder>>>,
    /// The `served` fixture, applied after the next successful `Init`.
    served: Cell<Option<(u32, u32)>>,
    records: RefCell<BTreeMap<String, Record>>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            precision: Cell::new(fc4_core::x87::Precision::Bits24),
            me: RefCell::new(Weak::new()),
            events: EventLog::default(),
            sim: RefCell::new(None),
            city: Cell::new(CityDims::square(16)),
            dev_size: Cell::new(4),
            developers: RefCell::new(Vec::new()),
            tuning: RefCell::new(Some(Rc::new(shipped_tuning()))),
            month: Cell::new(6),
            today: Cell::new(1000),
            demand_node: Cell::new(true),
            message_server: Cell::new(true),
            simulator: Cell::new(true),
            pollution: Cell::new([0; 3]),
            building_map: RefCell::new(None),
            effect: Cell::new(Some(500)),
            altitude: Cell::new(0.0),
            groups: RefCell::new(BTreeMap::new()),
            refs: RefCell::new(BTreeMap::new()),
            bool_props: RefCell::new(BTreeMap::new()),
            next_department: Cell::new(900),
            props: RefCell::new(BTreeMap::new()),
            building_types: RefCell::new(BTreeMap::new()),
            slot11: RefCell::new(BTreeMap::new()),
            lots: RefCell::new(BTreeMap::new()),
            positions: RefCell::new(BTreeMap::new()),
            items: RefCell::new(BTreeMap::new()),
            departments: RefCell::new(BTreeMap::new()),
            funding: RefCell::new(BTreeMap::new()),
            locally_funded: RefCell::new(BTreeSet::new()),
            line_items: RefCell::new(BTreeMap::new()),
            line_counts: RefCell::new(BTreeMap::new()),
            next_line_item: Cell::new(700),
            ordinances: RefCell::new(BTreeMap::new()),
            served: Cell::new(None),
            records: RefCell::new(BTreeMap::new()),
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn sim(&self) -> R<Rc<ResidentialSim>> {
        self.sim
            .borrow()
            .clone()
            .ok_or_else(|| "no simulator (residential_create first)".to_string())
    }

    fn ev(&self, kind: &str, fields: Vec<(&str, Json)>) {
        self.events.push(kind, fields);
    }

    /// The three developers' population maps (`int16`, tract size
    /// `dev_size`), created on first use.
    fn developer(&self, w: usize) -> Rc<SimGrid<i16>> {
        let mut d = self.developers.borrow_mut();
        if d.is_empty() {
            let city = self.city.get();
            d.extend((0..3).map(|_| SimGrid::<i16>::create(&city, 0, self.dev_size.get())));
        }
        d[w].clone()
    }

    fn rng(&self, spec: Option<&Json>) -> R<Box<dyn ResidentialRandom>> {
        let real = match spec.and_then(|s| opt(s, "session")) {
            Some(s) => {
                let seeds = FixedSeeds {
                    wall_clock: 1,
                    process_clock: 1,
                };
                Some(ResidentialRng::from_session(u32_of(s, "session")?, &seeds))
            }
            None => None,
        };
        let draws: VecDeque<f64> = match spec.and_then(|s| opt(s, "draws")) {
            None => VecDeque::new(),
            Some(v) => arr(v, "draws")?
                .iter()
                .map(|x| scalar::num(x).ok_or_else(|| "draws: numbers".to_string()))
                .collect::<R<_>>()?,
        };
        Ok(Box::new(Stub {
            world: self.me.borrow().clone(),
            draws,
            real,
        }))
    }

    fn new_sim(&self, rng: Box<dyn ResidentialRandom>) -> Rc<ResidentialSim> {
        ResidentialSim::new(self.rc() as Rc<dyn ResidentialHost>, 77, rng)
    }

    /// The `served: [s, h]` fixture: `s` schools (occupants 9001…) and `h`
    /// hospitals (9101…) of capacity 100 (units 0 with the shipped chunks)
    /// and radius 10 m at lot centre (32, 32), each in its service's first
    /// group list with one reference, and population (1, 0, 0) on tract
    /// (0, 0): each building counts that tract once in its coverage state
    /// (REH §4.4 [quirk]), so the states serve `s` and `h`.
    fn apply_served(&self, s: &ResidentialSim) -> R<()> {
        let Some((schools, hospitals)) = self.served.take() else {
            return Ok(());
        };
        for (svc, base, n) in [(&SCHOOL, 9001u64, schools), (&HOSPITAL, 9101u64, hospitals)] {
            for occ in base..base + u64::from(n) {
                self.groups.borrow_mut().insert(occ, vec![svc.groups[0]]);
                self.props
                    .borrow_mut()
                    .insert((occ, svc.capacity), Variant::Scalar(Elem::U32(100)));
                self.props
                    .borrow_mut()
                    .insert((occ, svc.radius), Variant::Scalar(Elem::F32(10.0)));
                self.lots.borrow_mut().insert(occ, (32.0, 32.0));
                *self.refs.borrow_mut().entry(occ).or_insert(0) += 1;
                s.with_state(|st| {
                    let l = if svc.school {
                        &mut st.lists.school_groups[0]
                    } else {
                        &mut st.lists.hospital_groups[0]
                    };
                    l.push(Some(occ));
                });
            }
        }
        let g = s.snapshot().grids;
        g.population[0].as_ref().ok_or("no population grid")?.set(0, 0, 1);
        Ok(())
    }
}

/// A purpose: a name (`school_staff`, `school_vehicle`, `hospital_staff`,
/// `hospital_vehicle`) or an ID.
fn purpose_of(j: &Json) -> R<u32> {
    let named = |s: &str| match s {
        "school_staff" => Some(SCHOOL.staff_purpose),
        "school_vehicle" => Some(SCHOOL.vehicle_purpose),
        "hospital_staff" => Some(HOSPITAL.staff_purpose),
        "hospital_vehicle" => Some(HOSPITAL.vehicle_purpose),
        _ => None,
    };
    match j.as_str().and_then(named) {
        Some(p) => Ok(p),
        None => u32_of(j, "purpose"),
    }
}

/// A map key as an ID (decimal or `"0x…"`).
fn key_id(k: &str) -> R<i64> {
    Json::Str(k.to_string()).as_i64().ok_or_else(|| format!("bad ID '{k}'"))
}

/// A purpose map key (a name or an ID).
fn key_purpose(k: &str) -> R<u32> {
    purpose_of(&Json::Str(k.to_string()))
}

/// A property value as the world reports it: a u32 / i32 / f32 scalar as a
/// number, a bool as a boolean, an array as an array of those.
fn variant_json(v: &Variant) -> Json {
    fn elem(e: &Elem) -> Json {
        match e {
            Elem::F32(x) => jf(*x),
            Elem::Bool(b) => Json::Bool(*b),
            other => ji(other.as_i64()),
        }
    }
    match v {
        Variant::Scalar(e) => elem(e),
        v => Json::Array(v.items().iter().map(elem).collect()),
    }
}

fn query_json(q: &QueryData) -> Json {
    Json::Object(vec![
        ("capacity".into(), ji(q.capacity)),
        ("effective_capacity".into(), ji(q.effective_capacity)),
        ("capacity_per_staff".into(), ji(q.capacity_per_staff)),
        ("effective_per_staff".into(), ji(q.effective_per_staff)),
        ("served".into(), ji(q.served)),
        ("grade".into(), jf(q.grade)),
        ("staff_funding".into(), jf(q.staff_funding)),
        ("staff_cost".into(), ji(q.staff_cost)),
        ("vehicle_funding".into(), jf(q.vehicle_funding)),
        ("vehicle_cost".into(), ji(q.vehicle_cost)),
    ])
}

fn instance_of<T: GridElem>(g: &Rc<dyn TractGrid<T>>) -> u32 {
    g.save_ref()
        .object::<SimGrid<T>>()
        .map_or(u32::MAX, |g| g.instance_id())
}

impl GridProvider<u8> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        GridProvider::<u8>::create_grid(&CoreGrids { city: self.city.get() }, instance, tract_size)
    }
}

impl GridProvider<u16> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u16>>> {
        GridProvider::<u16>::create_grid(&CoreGrids { city: self.city.get() }, instance, tract_size)
    }
}

impl GridProvider<f32> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<f32>>> {
        GridProvider::<f32>::create_grid(&CoreGrids { city: self.city.get() }, instance, tract_size)
    }
}

impl ResidentialHost for World {
    fn invocation_precision(&self) -> fc4_core::x87::Precision {
        self.precision.get()
    }

    fn acquire_services(&self) {
        self.ev("acquire_services", vec![]);
    }
    fn acquire_developers(&self) {
        self.ev("acquire_developers", vec![]);
    }
    fn acquire_demand_node(&self) -> bool {
        self.ev("demand_node", vec![]);
        self.demand_node.get()
    }
    fn acquire_message_server(&self) -> bool {
        self.ev("message_server", vec![]);
        self.message_server.get()
    }
    fn acquire_budget_simulator(&self) -> bool {
        self.ev("budget_simulator", vec![]);
        true
    }
    fn has_simulator(&self) -> bool {
        self.ev("simulator", vec![]);
        self.simulator.get()
    }
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        self.ev(
            "add_agent",
            vec![
                ("level", ji(level)),
                ("name", Json::Str(name.into())),
                ("flags", ji(flags)),
            ],
        );
    }
    fn remove_agent(&self) {
        self.ev("remove_agent", vec![]);
    }
    fn subscribe(&self, msg_type: u32) {
        self.ev("subscribe", vec![("type", ji(msg_type))]);
    }
    fn unsubscribe(&self, msg_type: u32) {
        self.ev("unsubscribe", vec![("type", ji(msg_type))]);
    }
    fn release_message_server_and_budget(&self) {
        self.ev("release_message_server_and_budget", vec![]);
    }
    fn release_services(&self) {
        self.ev("release_services", vec![]);
    }
    fn set_history_queue_length(&self, series: u32, length: u32) {
        self.ev("queue_length", vec![("series", ji(series)), ("length", ji(length))]);
    }
    fn create_department(&self, id: u32, name: u32, max_funding: f32, init_funding: f32) -> Option<u64> {
        let t = self.next_department.get();
        self.next_department.set(t + 1);
        self.ev(
            "department",
            vec![
                ("id", ji(id)),
                ("name", ji(name)),
                ("max", jf(max_funding)),
                ("init", jf(init_funding)),
                ("token", ji(t as i64)),
            ],
        );
        Some(t)
    }
    fn release_department(&self, department: u64) {
        self.ev("release_department", vec![("token", ji(department as i64))]);
        *self.refs.borrow_mut().entry(department).or_insert(0) -= 1;
    }
    fn shutdown_grid_u8(&self, grid: &Rc<dyn TractGrid<u8>>) {
        self.ev("shutdown_grid", vec![("instance", ji(instance_of(grid)))]);
    }
    fn shutdown_grid_u16(&self, grid: &Rc<dyn TractGrid<u16>>) {
        self.ev("shutdown_grid", vec![("instance", ji(instance_of(grid)))]);
    }
    fn shutdown_grid_f32(&self, grid: &Rc<dyn TractGrid<f32>>) {
        self.ev("shutdown_grid", vec![("instance", ji(instance_of(grid)))]);
    }
    fn add_ref(&self, token: u64) {
        self.ev("add_ref", vec![("id", ji(token as i64))]);
        *self.refs.borrow_mut().entry(token).or_insert(0) += 1;
    }
    fn release(&self, token: u64) {
        self.ev("release", vec![("id", ji(token as i64))]);
        *self.refs.borrow_mut().entry(token).or_insert(0) -= 1;
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn building_map(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        self.building_map.borrow().clone().map(|g| g as Rc<dyn TractGrid<u8>>)
    }
    fn building_map_key(&self, purpose: u32, wealth: u32) -> u8 {
        // BDS slot 42 (§5.3): (purpose << 2) + wealth + 1.
        ((purpose << 2) + wealth + 1) as u8
    }
    fn developer_population_map(&self, dev_type: u32) -> Option<Rc<dyn TractGrid<i16>>> {
        let w = DEVELOPERS.iter().position(|&d| d == dev_type)?;
        Some(self.developer(w) as Rc<dyn TractGrid<i16>>)
    }
    fn pollution_value(&self, kind: u32, _cell_x: i32, _cell_z: i32) -> u16 {
        self.pollution.get().get(kind as usize).copied().unwrap_or(0)
    }
    fn sim_month(&self) -> u32 {
        self.ev("get_month", vec![]);
        self.month.get()
    }
    fn sim_day_number(&self) -> u32 {
        self.today.get()
    }
    fn altitude(&self, _x: f32, _z: f32) -> f32 {
        self.altitude.get()
    }
    fn create_visual_effect(&self, name: &str) -> CreatedEffect {
        self.ev("effect", vec![("name", Json::Str(name.into()))]);
        // RG1: null = the create failed; an ID = the effect.
        match self.effect.get() {
            Some(e) => CreatedEffect::Effect(e),
            None => CreatedEffect::Failed,
        }
    }
    fn set_effect_transform(&self, effect: u64, translation: [f32; 3]) {
        self.ev(
            "transform",
            vec![
                ("effect", ji(effect as i64)),
                ("pos", Json::Array(translation.iter().map(|&v| jf(v)).collect())),
            ],
        );
    }
    fn start_and_release_effect(&self, effect: u64) {
        self.ev("start_effect", vec![("effect", ji(effect as i64))]);
    }
    fn record_history(&self, series: u32, value: i32) {
        self.ev("history", vec![("series", ji(series)), ("value", ji(value))]);
    }
    fn post_strike_message(&self, msg_type: u32, occupant: Option<u64>, data2: u32, data3: u32) {
        self.ev(
            "strike",
            vec![
                ("type", ji(msg_type)),
                ("occupant", occupant.map_or(Json::Null, |o| ji(o as i64))),
                ("data2", ji(data2)),
                ("data3", ji(data3)),
            ],
        );
    }
    fn occupant_as_building(&self, occupant: u64) -> Option<u64> {
        self.groups.borrow().contains_key(&occupant).then_some(occupant)
    }
    fn is_occupant_group(&self, occupant: u64, group: u32) -> bool {
        self.groups.borrow().get(&occupant).is_some_and(|g| g.contains(&group))
    }
    fn occupant_bool_property(&self, occupant: u64, id: u32) -> Option<bool> {
        self.bool_props.borrow().get(&(occupant, id)).copied()
    }
    fn set_occupant_bool_property(&self, occupant: u64, id: u32, value: bool) -> bool {
        self.bool_props.borrow_mut().insert((occupant, id), value);
        true
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
}

/// The budget simulator, the occupants' property holders and lots, the
/// ordinance simulator and the automata controllers, as scripted by
/// `residential_env`; every change the simulator makes is logged.
impl EducationHealthHost for World {
    fn budget_item(&self, occupant: u64, purpose: u32) -> Option<BudgetItem> {
        self.items.borrow().get(&(occupant, purpose)).copied()
    }
    fn change_budget_item_line(&self, occupant: u64, index: u32, line: u32) {
        self.ev(
            "change_line",
            vec![
                ("occupant", ji(occupant as i64)),
                ("index", ji(index)),
                ("line", ji(line)),
            ],
        );
        for ((o, _), item) in self.items.borrow_mut().iter_mut() {
            if *o == occupant && item.index == index {
                item.line = line;
            }
        }
    }
    fn department_budget(&self, department_id: u32) -> Option<u64> {
        self.departments.borrow().get(&department_id).copied()
    }
    fn department_funding(&self, department: u64) -> Option<f32> {
        // RG10: NaN marks a department whose funding cannot be read.
        let f = self.funding.borrow().get(&department).copied().unwrap_or(1.0);
        (!f.is_nan()).then_some(f)
    }
    fn is_locally_funded(&self, department: u64, occupant: u64, purpose: u32) -> bool {
        self.locally_funded.borrow().contains(&(department, occupant, purpose))
    }
    fn add_locally_funded(&self, department: u64, occupant: u64, purpose: u32) {
        self.ev(
            "add_local",
            vec![
                ("department", ji(department as i64)),
                ("occupant", ji(occupant as i64)),
                ("purpose", ji(purpose)),
            ],
        );
        self.locally_funded.borrow_mut().insert((department, occupant, purpose));
    }
    fn remove_locally_funded(&self, department: u64, occupant: u64, purpose: u32) {
        self.ev(
            "remove_local",
            vec![
                ("department", ji(department as i64)),
                ("occupant", ji(occupant as i64)),
                ("purpose", ji(purpose)),
            ],
        );
        self.locally_funded
            .borrow_mut()
            .remove(&(department, occupant, purpose));
    }
    fn set_local_funding(&self, department: u64, occupant: u64, fraction: f32, purpose: u32) {
        self.ev(
            "set_local",
            vec![
                ("department", ji(department as i64)),
                ("occupant", ji(occupant as i64)),
                ("fraction", jf(fraction)),
                ("purpose", ji(purpose)),
            ],
        );
        if let Some(item) = self.items.borrow_mut().get_mut(&(occupant, purpose)) {
            item.local_funding = fraction;
        }
    }
    fn line_item(&self, department: u64, line: u32) -> Option<u64> {
        self.line_items.borrow().get(&(department, line)).copied()
    }
    fn create_line_item_for_building_type(&self, department: u64, line: u32, local: bool) -> Option<u64> {
        let t = self.next_line_item.get();
        self.next_line_item.set(t + 1);
        self.ev(
            "create_line",
            vec![
                ("department", ji(department as i64)),
                ("line", ji(line)),
                ("local", Json::Bool(local)),
                ("token", ji(t as i64)),
            ],
        );
        self.line_items.borrow_mut().insert((department, line), t);
        Some(t)
    }
    fn remove_line_item(&self, department: u64, line: u32) {
        self.ev(
            "remove_line",
            vec![("department", ji(department as i64)), ("line", ji(line))],
        );
        self.line_items.borrow_mut().remove(&(department, line));
    }
    fn line_item_count(&self, line_item: u64) -> i64 {
        self.line_counts.borrow().get(&line_item).copied().unwrap_or(0)
    }
    fn set_line_item_count(&self, line_item: u64, count: i64) {
        self.ev(
            "line_count",
            vec![("line_item", ji(line_item as i64)), ("count", ji(count))],
        );
        self.line_counts.borrow_mut().insert(line_item, count);
    }
    fn set_line_item_display_flag(&self, line_item: u64, flag: u32, on: bool) {
        self.ev(
            "display_flag",
            vec![
                ("line_item", ji(line_item as i64)),
                ("flag", ji(flag)),
                ("on", Json::Bool(on)),
            ],
        );
    }
    fn occupant_property(&self, occupant: u64, id: u32) -> Option<Variant> {
        self.props.borrow().get(&(occupant, id)).cloned()
    }
    fn set_occupant_u32(&self, occupant: u64, id: u32, value: u32) {
        self.props
            .borrow_mut()
            .insert((occupant, id), Variant::Scalar(Elem::U32(value)));
    }
    fn set_occupant_f32(&self, occupant: u64, id: u32, value: f32) {
        self.props
            .borrow_mut()
            .insert((occupant, id), Variant::Scalar(Elem::F32(value)));
    }
    fn building_type(&self, building: u64) -> u32 {
        self.building_types.borrow().get(&building).copied().unwrap_or(0)
    }
    fn building_slot11(&self, building: u64) -> u32 {
        self.slot11.borrow().get(&building).copied().unwrap_or(0)
    }
    fn occupant_lot_center(&self, occupant: u64) -> Option<(f32, f32)> {
        self.lots.borrow().get(&occupant).copied()
    }
    fn occupant_position(&self, occupant: u64) -> (f32, f32) {
        self.positions.borrow().get(&occupant).copied().unwrap_or((0.0, 0.0))
    }
    fn ordinances_on(&self) -> Vec<u32> {
        self.ev("ordinance_effects", vec![]);
        self.ordinances.borrow().keys().copied().collect()
    }
    fn ordinance_properties(&self, id: u32) -> Option<Rc<PropertyHolder>> {
        self.ordinances.borrow().get(&id).cloned()
    }
    fn add_controllers(&self, name: &str, occupant: u64) {
        self.ev(
            "controllers",
            vec![
                ("name", Json::Str(name.into())),
                ("occupant", ji(occupant as i64)),
                ("added", Json::Bool(true)),
            ],
        );
    }
    fn remove_controllers(&self, name: &str, occupant: u64) {
        self.ev(
            "controllers",
            vec![
                ("name", Json::Str(name.into())),
                ("occupant", ji(occupant as i64)),
                ("added", Json::Bool(false)),
            ],
        );
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "residential", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("residential_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "create" => create(&w, op),
        "init" => w.sim().and_then(|s| {
            let ok = s.init();
            if ok {
                w.apply_served(&s)?;
            }
            check_expect(op, Json::Bool(ok))
        }),
        "expect_props" => expect_props(&w, op),
        "shutdown" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.shutdown()))),
        "message" => message(&w, op),
        "run_to" => run_to(&w, op),
        "tract" => tract(&w, op),
        "expect_tract" => expect_tract(&w, op),
        "call" => call(&w, op),
        "set" => set(&w, op),
        "expect" => expect(&w, op),
        "grid_set" => grid_set(&w, op),
        "expect_grid" => expect_grid(&w, op),
        "expect_refs" => expect_refs(&w, op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        "write" => write(&w, op),
        "read" => read(&w, op),
        "expect_record" => expect_record(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    })
}

// ---- environment --------------------------------------------------------------------------------

fn pair_i(v: &Json, what: &str) -> R<(i64, i64)> {
    let a = arr(v, what)?;
    if a.len() != 2 {
        return Err(format!("'{what}' must have two entries"));
    }
    Ok((i64_of(&a[0], what)?, i64_of(&a[1], what)?))
}

fn triple_u16(v: &Json, what: &str) -> R<[u16; 3]> {
    let a = arr(v, what)?;
    if a.len() != 3 {
        return Err(format!("'{what}' must have three entries"));
    }
    Ok([
        u32_of(&a[0], what)? as u16,
        u32_of(&a[1], what)? as u16,
        u32_of(&a[2], what)? as u16,
    ])
}

/// A float argument: a number, or a `"0x…"` float32 bit pattern.
fn float_of(v: &Json, what: &str) -> R<f32> {
    match v.as_str().and_then(|s| s.strip_prefix("0x")) {
        Some(h) => u32::from_str_radix(h, 16)
            .map(f32::from_bits)
            .map_err(|e| format!("{what}: {e}")),
        None => f32_of(v, what),
    }
}

fn triple_f32(v: &Json, what: &str) -> R<[f32; 3]> {
    let a = arr(v, what)?;
    if a.len() != 3 {
        return Err(format!("'{what}' must have three entries"));
    }
    Ok([float_of(&a[0], what)?, float_of(&a[1], what)?, float_of(&a[2], what)?])
}

fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "precision_bits") {
        w.precision.set(match u32_of(v, "precision_bits")? {
            24 => fc4_core::x87::Precision::Bits24,
            53 => fc4_core::x87::Precision::Bits53,
            _ => return Err("precision_bits must be 24 or 53".into()),
        });
    }
    if let Some(v) = opt(op, "cells") {
        let (x, z) = pair_i(v, "cells")?;
        w.city.set(CityDims {
            cells_x: x as i32,
            cells_z: z as i32,
            cell_width_x: 16.0,
            cell_width_z: 16.0,
        });
        w.developers.borrow_mut().clear();
    }
    if let Some(v) = opt(op, "dev_size") {
        w.dev_size.set(i64_of(v, "dev_size")? as i32);
        w.developers.borrow_mut().clear();
    }
    if let Some(v) = opt(op, "month") {
        w.month.set(u32_of(v, "month")?);
    }
    if let Some(v) = opt(op, "today") {
        w.today.set(u32_of(v, "today")?);
    }
    if let Some(v) = op.get("tuning") {
        *w.tuning.borrow_mut() = match v {
            Json::Null => None,
            v => match str_of(v, "tuning")? {
                "shipped" => Some(Rc::new(shipped_tuning())),
                "empty" => Some(Rc::new(PropertyHolder::new())),
                other => return Err(format!("unknown tuning '{other}'")),
            },
        };
    }
    if let Some(v) = opt(op, "tuning_props") {
        let mut t = w.tuning.borrow().as_deref().cloned().unwrap_or_default();
        for (k, val) in v.as_object().ok_or("tuning_props must be an object")? {
            let id = Json::Str(k.clone())
                .as_i64()
                .ok_or_else(|| format!("bad property ID '{k}'"))? as u32;
            match val {
                Json::Null => {
                    t.remove(id);
                }
                val => t.add(id, scalar::variant_of(val)?),
            }
        }
        *w.tuning.borrow_mut() = Some(Rc::new(t));
    }
    if let Some(v) = opt(op, "pollution") {
        w.pollution.set(triple_u16(v, "pollution")?);
    }
    if let Some(v) = opt(op, "altitude") {
        w.altitude.set(f32_of(v, "altitude")?);
    }
    if let Some(v) = op.get("effect") {
        w.effect.set(match v {
            Json::Null => None,
            v => Some(i64_of(v, "effect")? as u64),
        });
    }
    if let Some(v) = op.get("building_map") {
        *w.building_map.borrow_mut() = match v {
            Json::Null => None,
            v => {
                let size = opt(v, "size").map_or(Ok(1), |s| i64_of(s, "size"))? as i32;
                let g = SimGrid::<u8>::create(&w.city.get(), 0, size);
                for c in arr(field(v, "cells")?, "cells")? {
                    let c = arr(c, "cell")?;
                    g.set(
                        i64_of(&c[0], "x")? as i32,
                        i64_of(&c[1], "z")? as i32,
                        u32_of(&c[2], "key")? as u8,
                    );
                }
                Some(g)
            }
        };
    }
    for (k, cell) in [
        ("demand_node", &w.demand_node),
        ("message_server", &w.message_server),
        ("simulator", &w.simulator),
    ] {
        if let Some(v) = opt(op, k) {
            cell.set(bool_of(v, k)?);
        }
    }
    if let Some(v) = opt(op, "served") {
        let (s, h) = pair_i(v, "served")?;
        w.served.set(Some((s as u32, h as u32)));
    }
    env_education_health(w, op)?;
    if let Some(v) = opt(op, "groups") {
        let mut g = w.groups.borrow_mut();
        for (k, list) in v.as_object().ok_or("groups must be an object")? {
            let id = Json::Str(k.clone()).as_i64().ok_or("bad occupant ID")? as u64;
            match list {
                Json::Null => {
                    g.remove(&id);
                }
                list => {
                    let ids = arr(list, "groups")?
                        .iter()
                        .map(|x| u32_of(x, "group"))
                        .collect::<R<Vec<u32>>>()?;
                    g.insert(id, ids);
                }
            }
        }
    }
    Ok(())
}

fn pair_f(v: &Json, what: &str) -> R<(f32, f32)> {
    let a = arr(v, what)?;
    if a.len() != 2 {
        return Err(format!("'{what}' must have two entries"));
    }
    Ok((float_of(&a[0], what)?, float_of(&a[1], what)?))
}

fn obj<'a>(v: &'a Json, what: &str) -> R<&'a [(String, Json)]> {
    v.as_object().ok_or_else(|| format!("'{what}' must be an object"))
}

/// A per-occupant map `{"<occupant>": value | null}` into `map`.
fn occupant_map<T>(op: &Json, key: &str, map: &RefCell<BTreeMap<u64, T>>, value: impl Fn(&Json) -> R<T>) -> R<()> {
    if let Some(v) = opt(op, key) {
        for (k, x) in obj(v, key)? {
            let occ = key_id(k)? as u64;
            match x {
                Json::Null => {
                    map.borrow_mut().remove(&occ);
                }
                x => {
                    let t = value(x)?;
                    map.borrow_mut().insert(occ, t);
                }
            }
        }
    }
    Ok(())
}

/// The education / health part of `residential_env`: occupant properties,
/// lots, positions, building types and slot-11 values, departments, budget
/// items, the locally funded set, line items and the ordinances that are on.
fn env_education_health(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "props") {
        let mut props = w.props.borrow_mut();
        for (k, list) in obj(v, "props")? {
            let occ = key_id(k)? as u64;
            if matches!(list, Json::Null) {
                props.retain(|(o, _), _| *o != occ);
                continue;
            }
            for (id, x) in obj(list, "props")? {
                let id = key_id(id)? as u32;
                match x {
                    Json::Null => {
                        props.remove(&(occ, id));
                    }
                    x => {
                        props.insert((occ, id), scalar::variant_of(x)?);
                    }
                }
            }
        }
    }
    occupant_map(op, "lots", &w.lots, |x| pair_f(x, "lot"))?;
    occupant_map(op, "positions", &w.positions, |x| pair_f(x, "position"))?;
    occupant_map(op, "building_types", &w.building_types, |x| u32_of(x, "type"))?;
    occupant_map(op, "slot11", &w.slot11, |x| u32_of(x, "slot11"))?;
    if let Some(v) = opt(op, "departments") {
        for (k, x) in obj(v, "departments")? {
            let id = key_id(k)? as u32;
            match x {
                Json::Null => {
                    w.departments.borrow_mut().remove(&id);
                }
                x => {
                    let token = i64_of(field(x, "token")?, "token")? as u64;
                    w.departments.borrow_mut().insert(id, token);
                    if let Some(f) = opt(x, "funding") {
                        w.funding.borrow_mut().insert(token, float_of(f, "funding")?);
                    }
                }
            }
        }
    }
    if let Some(v) = opt(op, "items") {
        let mut items = w.items.borrow_mut();
        for (k, per) in obj(v, "items")? {
            let occ = key_id(k)? as u64;
            for (p, x) in obj(per, "items")? {
                let purpose = key_purpose(p)?;
                match x {
                    Json::Null => {
                        items.remove(&(occ, purpose));
                    }
                    x => {
                        let num = |key: &str, d: i64| opt(x, key).map_or(Ok(d), |v| i64_of(v, key));
                        let item = BudgetItem {
                            department: u32_of(field(x, "department")?, "department")?,
                            line: num("line", 0)? as u32,
                            index: num("index", 0)? as u32,
                            cost: num("cost", 0)?,
                            local_funding: opt(x, "local").map_or(Ok(1.0), |v| float_of(v, "local"))?,
                        };
                        items.insert((occ, purpose), item);
                    }
                }
            }
        }
    }
    if let Some(v) = opt(op, "locally_funded") {
        let mut set = w.locally_funded.borrow_mut();
        set.clear();
        for e in arr(v, "locally_funded")? {
            let e = arr(e, "locally_funded")?;
            if e.len() != 3 {
                return Err("locally_funded entries are [department, occupant, purpose]".into());
            }
            set.insert((
                i64_of(&e[0], "department")? as u64,
                i64_of(&e[1], "occupant")? as u64,
                purpose_of(&e[2])?,
            ));
        }
    }
    if let Some(v) = opt(op, "line_items") {
        for e in arr(v, "line_items")? {
            let e = arr(e, "line_items")?;
            if e.len() != 4 {
                return Err("line_items entries are [department, line, token, count]".into());
            }
            let (dept, line) = (i64_of(&e[0], "department")? as u64, u32_of(&e[1], "line")?);
            let token = i64_of(&e[2], "token")? as u64;
            w.line_items.borrow_mut().insert((dept, line), token);
            w.line_counts.borrow_mut().insert(token, i64_of(&e[3], "count")?);
        }
    }
    if let Some(v) = opt(op, "ordinances") {
        for (k, x) in obj(v, "ordinances")? {
            let id = key_id(k)? as u32;
            match x {
                Json::Null => {
                    w.ordinances.borrow_mut().remove(&id);
                }
                x => {
                    w.ordinances.borrow_mut().insert(id, Rc::new(scalar::holder_of(x)?));
                }
            }
        }
    }
    Ok(())
}

/// `props`: `{"<id>": value | null}` of one occupant (`null` = absent), a
/// u32 compared as an integer and an f32 as a float (`bits` for exact
/// float32 patterns). `locally_funded` checks the whole set.
fn expect_props(w: &World, op: &Json) -> R<()> {
    let mut errors = Vec::new();
    if let Some(want) = opt(op, "props") {
        let occ = i64_of(field(op, "occupant")?, "occupant")? as u64;
        let props = w.props.borrow();
        for (k, v) in obj(want, "props")? {
            let id = key_id(k)? as u32;
            let got = props
                .get(&(occ, id))
                .map_or(Json::Null, |p| with_bits(op, variant_json(p)));
            if !scalar::matches(&got, v, false) {
                errors.push(format!("{k}: got {}, expected {}", scalar::show(&got), scalar::show(v)));
            }
        }
    }
    if let Some(want) = opt(op, "locally_funded") {
        let got = Json::Array(
            w.locally_funded
                .borrow()
                .iter()
                .map(|&(d, o, p)| Json::Array(vec![ji(d as i64), ji(o as i64), ji(p)]))
                .collect(),
        );
        if !scalar::matches(&got, want, false) {
            errors.push(format!("locally funded: {}", scalar::show(&got)));
        }
    }
    if let Some(want) = opt(op, "line_counts") {
        for (k, v) in obj(want, "line_counts")? {
            let t = key_id(k)? as u64;
            let got = w.line_counts.borrow().get(&t).copied();
            let got = got.map_or(Json::Null, ji);
            if !scalar::matches(&got, v, false) {
                errors.push(format!("line item {k}: count {}", scalar::show(&got)));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn create(w: &World, op: &Json) -> R<()> {
    let rng = w.rng(opt(op, "rng"))?;
    let s = w.new_sim(rng);
    if let Some(v) = opt(op, "proximity_size") {
        let p = i64_of(v, "proximity_size")? as i32;
        s.with_state(|st| st.proximity_size = p);
    }
    if let Some(v) = opt(op, "work_budget") {
        s.set_work_budget(u32_of(v, "work_budget")?);
    }
    *w.sim.borrow_mut() = Some(s);
    Ok(())
}

fn message_type(j: &Json) -> R<u32> {
    if let Some(s) = j.as_str() {
        let t = match s {
            "SimBegin" => Some(msg::SIM_BEGIN),
            "SimNewDay" => Some(msg::SIM_NEW_DAY),
            "SimNewMonth" => Some(msg::SIM_NEW_MONTH),
            "SimIdle" => Some(msg::SIM_IDLE),
            "OccupantInserted" => Some(msg::OCCUPANT_INSERTED),
            "OccupantRemoved" => Some(msg::OCCUPANT_REMOVED),
            _ => None,
        };
        if let Some(t) = t {
            return Ok(t);
        }
    }
    u32_of(j, "type")
}

/// `DoMessage`, `count` times; every result is checked against `expect`.
fn message(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let m = ResidentialMessage {
        msg_type: message_type(field(op, "type")?)?,
        data1: 0,
        data2: 0,
        data3: opt(op, "data3").map_or(Ok(OCCUPANT_TYPE_BUILDING), |v| u32_of(v, "data3"))?,
        void1: opt(op, "occupant")
            .map(|v| i64_of(v, "occupant"))
            .transpose()?
            .map(|o| o as u64),
    };
    let count = opt(op, "count").map_or(Ok(1), |v| i64_of(v, "count"))?;
    for i in 0..count {
        check_expect(op, Json::Bool(s.do_message(&m))).map_err(|e| format!("message {i}: {e}"))?;
    }
    Ok(())
}

/// NewDay until the state is `state`; every step must finish. `steps`
/// checks the number of NewDays taken.
fn run_to(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let target = u32_of(field(op, "state")?, "state")?;
    let mut n = 0i64;
    while s.snapshot().state != target {
        if n >= 1000 {
            return Err(format!("state {target} not reached in 1000 days"));
        }
        if !s.do_daily_simulation() {
            return Err(format!("day {n} (state {}) did not finish", s.snapshot().state));
        }
        n += 1;
    }
    if let Some(want) = opt(op, "steps") {
        if n != i64_of(want, "steps")? {
            return Err(format!("took {n} days"));
        }
    }
    Ok(())
}

fn xz(op: &Json) -> R<(i32, i32)> {
    Ok((
        i64_of(field(op, "x")?, "x")? as i32,
        i64_of(field(op, "z")?, "z")? as i32,
    ))
}

/// Sets one population tract: `pop` (the current populations), `new` (the
/// developers' maps), `steady` (both), `eq`, `hq`, `age`.
fn tract(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let (x, z) = xz(op)?;
    let g = s.snapshot().grids;
    let steady = opt(op, "steady").map(|v| triple_u16(v, "steady")).transpose()?;
    if let Some(p) = opt(op, "pop").map(|v| triple_u16(v, "pop")).transpose()?.or(steady) {
        for (wl, v) in p.into_iter().enumerate() {
            g.population[wl].as_ref().ok_or("no population grid")?.set(x, z, v);
        }
    }
    if let Some(p) = opt(op, "new").map(|v| triple_u16(v, "new")).transpose()?.or(steady) {
        for (wl, v) in p.into_iter().enumerate() {
            w.developer(wl).set(x, z, v as i16);
        }
    }
    for (k, grids) in [("eq", &g.eq), ("hq", &g.hq)] {
        if let Some(v) = opt(op, k) {
            for (wl, q) in triple_f32(v, k)?.into_iter().enumerate() {
                grids[wl].as_ref().ok_or("no quotient grid")?.set(x, z, q);
            }
        }
    }
    if let Some(v) = opt(op, "age") {
        g.age.as_ref().ok_or("no age grid")?.set(x, z, u32_of(v, "age")? as u8);
    }
    Ok(())
}

fn bits_str(v: f32) -> Json {
    Json::Str(format!("0x{:08X}", v.to_bits()))
}

/// With `"bits": true`, every float becomes its float32 bit pattern.
fn with_bits(op: &Json, got: Json) -> Json {
    fn conv(j: Json) -> Json {
        match j {
            Json::Float(f) => bits_str(f as f32),
            Json::Array(a) => Json::Array(a.into_iter().map(conv).collect()),
            Json::Object(o) => Json::Object(o.into_iter().map(|(k, v)| (k, conv(v))).collect()),
            other => other,
        }
    }
    if opt(op, "bits").and_then(boolean).unwrap_or(false) {
        conv(got)
    } else {
        got
    }
}

fn floats(v: &[f32]) -> Json {
    Json::Array(v.iter().map(|&x| jf(x)).collect())
}

fn ints<T: Copy + Into<i64>>(v: &[T]) -> Json {
    Json::Array(v.iter().map(|&x| ji(x)).collect())
}

fn expect_tract(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let (x, z) = xz(op)?;
    let g = s.snapshot().grids;
    let pop: Vec<u16> = g
        .population
        .iter()
        .map(|p| p.as_ref().map_or(0, |p| p.get(x, z)))
        .collect();
    let eq: Vec<f32> = g.eq.iter().map(|q| q.as_ref().map_or(0.0, |q| q.get(x, z))).collect();
    let hq: Vec<f32> = g.hq.iter().map(|q| q.as_ref().map_or(0.0, |q| q.get(x, z))).collect();
    let age = g.age.as_ref().map_or(0, |a| a.get(x, z));
    let got = vec![
        ("pop".to_string(), ints(&pop)),
        ("eq".to_string(), with_bits(op, floats(&eq))),
        ("hq".to_string(), with_bits(op, floats(&hq))),
        ("age".to_string(), ji(age)),
    ];
    let want = Json::Object(
        ["pop", "eq", "hq", "age"]
            .iter()
            .filter_map(|k| op.get(k).map(|v| (k.to_string(), v.clone())))
            .collect(),
    );
    check_state(&got, &want).map_err(|e| format!("tract ({x}, {z}): {e}"))
}

// ---- calls --------------------------------------------------------------------------------------

fn kind_is_eq(v: &Json) -> R<bool> {
    match str_of(v, "kind")? {
        "eq" => Ok(true),
        "hq" => Ok(false),
        other => Err(format!("kind must be eq or hq, not '{other}'")),
    }
}

fn ids(v: &[u64]) -> Json {
    Json::Array(v.iter().map(|&x| ji(x as i64)).collect())
}

fn ret_name(r: SlotReturn) -> &'static str {
    match r {
        SlotReturn::Al => "AL",
        SlotReturn::Eax => "EAX",
        SlotReturn::St0 => "ST0",
        SlotReturn::HiddenPtr => "hidden",
        SlotReturn::None => "none",
    }
}

fn call(w: &World, op: &Json) -> R<()> {
    let method = str_of(field(op, "method")?, "method")?;
    let a = |i: usize| arg(op, i);
    let ai = |i: usize| -> R<i32> { Ok(i64_of(arg(op, i)?, "argument")? as i32) };
    // Pure helpers need no simulator.
    let pure = match method {
        "age_bucket" => Some(ji(age_bucket(ai(0)?) as i64)),
        "pct" => Some(jf(pct(float_of(a(0)?, "x")?))),
        "slot" => {
            let n = ai(0)? as usize;
            let r = SLOTS.get(n).ok_or("no such slot")?;
            Some(Json::Array(vec![
                Json::Str(r.1.into()),
                ji(r.2),
                Json::Str(ret_name(r.3).into()),
            ]))
        }
        "slot_count" => Some(ji(SLOTS.len() as i64)),
        "query_interface" => Some(Json::Bool(query_interface(u32_of(a(0)?, "iid")?))),
        "rhe" => Some(ji(rhe(float_of(a(0)?, "x")?))),
        "tracts_in_radius" => {
            let fa = |i: usize| float_of(arg(op, i)?, "argument");
            let list = tracts_in_radius(fa(0)?, fa(1)?, fa(2)?, fa(3)?, ai(4)?, ai(5)?);
            Some(Json::Array(
                list.into_iter().map(|(x, z)| Json::Array(vec![ji(x), ji(z)])).collect(),
            ))
        }
        _ => None,
    };
    if let Some(got) = pure {
        return check_call(op, with_bits(op, got));
    }
    let s = w.sim()?;
    let got = match method {
        "do_population_change" => {
            s.do_population_change(ai(0)?, ai(1)?);
            Json::Null
        }
        "hq_loss" => {
            let st = s.snapshot();
            jf(s.hq_loss(&st.population_tuning, st.population_size, ai(0)?, ai(1)?))
        }
        "tract_quotient" => {
            let g = s.snapshot().grids;
            let q = if kind_is_eq(a(0)?)? { &g.eq } else { &g.hq };
            jf(ResidentialSim::tract_quotient(q, &g.population, ai(1)?, ai(2)?))
        }
        "get_cell_eq" => jf(s.get_cell_eq(ai(0)?, ai(1)?)),
        "get_cell_hq" => jf(s.get_cell_hq(ai(0)?, ai(1)?)),
        "get_cell_eq_by_wealth" => jf(s.get_cell_eq_by_wealth(ai(0)?, ai(1)?, ai(2)? as u8)),
        "get_cell_hq_by_wealth" => jf(s.get_cell_hq_by_wealth(ai(0)?, ai(1)?, ai(2)? as u8)),
        "get_global_eq" => jf(s.get_global_eq()),
        "get_global_hq" => jf(s.get_global_hq()),
        "get_global_eq_register" => Json::Float(s.global_quotient_register(true)),
        "get_global_hq_register" => Json::Float(s.global_quotient_register(false)),
        "get_cell_life_expectancy" => ji(s.get_cell_life_expectancy(ai(0)?, ai(1)?)),
        "get_global_le" => ji(s.get_global_le()),
        "get_cell_workforce_percent" => jf(s.get_cell_workforce_percent(ai(0)?, ai(1)?)),
        "get_global_workforce_percent" => jf(s.get_global_workforce_percent()),
        "get_average_new_age_by_wealth" => ji(s.get_average_new_age_by_wealth(ai(0)? as u8)),
        "get_population" => ji(s.get_population()),
        "max_quotient" => ji(s.max_quotient()),
        "total_city_upkeep_cost" => ji(s.total_city_upkeep_cost()),
        "average_grid" => {
            let (g, min, max) = s.average_quotient_grid(kind_is_eq(a(0)?)?);
            let mut kv = vec![("min".to_string(), jf(min)), ("max".to_string(), jf(max))];
            if let Some(g) = &g {
                let geom = g.geometry();
                let mut values = Vec::new();
                for x in 0..geom.count_x {
                    for z in 0..geom.count_z {
                        values.push(jf(g.get(x, z)));
                    }
                }
                kv.push(("values".to_string(), Json::Array(values)));
                kv.push(("instance".to_string(), ji(instance_of(g))));
                kv.push(("size".to_string(), ji(geom.tract_size)));
                kv.push(("refs".to_string(), ji(Rc::strong_count(g) as i64)));
            }
            Json::Object(kv)
        }
        "average_grid_is_new" => {
            let is_eq = kind_is_eq(a(0)?)?;
            let (g1, _, _) = s.average_quotient_grid(is_eq);
            let (g2, _, _) = s.average_quotient_grid(is_eq);
            match (g1, g2) {
                (Some(g1), Some(g2)) => Json::Bool(Rc::as_ptr(&g1).cast::<()>() != Rc::as_ptr(&g2).cast::<()>()),
                _ => return Err("no grid".into()),
            }
        }
        "quotient_min_max" => match s.quotient_min_max(kind_is_eq(a(0)?)?) {
            None => Json::Null,
            Some((x0, z0, x1, z1, min, max)) => Json::Array(vec![ji(x0), ji(z0), ji(x1), ji(z1), jf(min), jf(max)]),
        },
        "grid_triple" => grid_triple(&s, str_of(a(0)?, "which")?)?,
        "school_is_on_strike" => Json::Bool(s.school_is_on_strike()),
        "health_is_on_strike" => Json::Bool(s.health_is_on_strike()),
        "end_school_strike" => Json::Bool(s.end_school_strike()),
        "end_health_strike" => Json::Bool(s.end_health_strike()),
        "chance_of_school_strike" => jf(s.chance_of_school_strike()),
        "chance_of_health_strike" => jf(s.chance_of_health_strike()),
        "school_building_count" => ji(s.school_building_count()),
        "hospital_building_count" => ji(s.hospital_building_count()),
        "school_buildings" | "hospital_buildings" => {
            let groups = match op.get("args").and_then(Json::as_array).and_then(|v| v.first()) {
                None | Some(Json::Null) => None,
                Some(g) => Some(
                    arr(g, "groups")?
                        .iter()
                        .map(|x| u32_of(x, "group"))
                        .collect::<R<Vec<u32>>>()?,
                ),
            };
            let out = if method == "school_buildings" {
                s.school_buildings(groups.as_deref())
            } else {
                s.hospital_buildings(groups.as_deref())
            };
            ids(&out)
        }
        "health_affecting_buildings" => ids(&s.health_affecting_buildings()),
        "global_auto_budget_schools" => Json::Bool(s.global_auto_budget_schools()),
        "global_auto_budget_hospitals" => Json::Bool(s.global_auto_budget_hospitals()),
        "set_global_auto_budget_schools" => Json::Bool(s.set_global_auto_budget_schools(bool_of(a(0)?, "v")?)),
        "set_global_auto_budget_hospitals" => Json::Bool(s.set_global_auto_budget_hospitals(bool_of(a(0)?, "v")?)),
        "get_auto_budget" => Json::Bool(s.get_auto_budget(i64_of(a(0)?, "occupant")? as u64)),
        "set_auto_budget" => Json::Bool(s.set_auto_budget(i64_of(a(0)?, "occupant")? as u64, bool_of(a(1)?, "v")?)),
        "set_occupant_funding" => Json::Bool(s.set_occupant_funding(
            i64_of(a(0)?, "occupant")? as u64,
            float_of(a(1)?, "staff")?,
            float_of(a(2)?, "vehicle")?,
            bool_of(a(3)?, "school")?,
        )),
        "get_occupant_funding" => {
            let start = |i: usize| op.get("args").and_then(Json::as_array).and_then(|v| v.get(i));
            let mut fa = start(2).map_or(Ok(1.0), |v| float_of(v, "a"))?;
            let mut fb = start(3).map_or(Ok(1.0), |v| float_of(v, "b"))?;
            s.get_occupant_funding(
                i64_of(a(0)?, "occupant")? as u64,
                &mut fa,
                &mut fb,
                bool_of(a(1)?, "school")?,
            );
            floats(&[fa, fb])
        }
        "efficiency_ratings" => {
            let (e1, e2) = s.efficiency_ratings(
                i64_of(a(0)?, "occupant")? as u64,
                bool_of(a(1)?, "school")?,
                bool_of(a(2)?, "ignore_strike")?,
            );
            floats(&[e1, e2])
        }
        "base_staff_efficiency" => {
            jf(s.base_staff_efficiency(i64_of(a(0)?, "occupant")? as u64, bool_of(a(1)?, "school")?))
        }
        "do_building_changed" => {
            s.do_building_changed(i64_of(a(0)?, "occupant")? as u64, bool_of(a(1)?, "added")?);
            Json::Null
        }
        "occupant_service" => match s.occupant_service(i64_of(a(0)?, "occupant")? as u64) {
            None => Json::Null,
            Some(svc) => Json::Str(if svc.school { "school" } else { "hospital" }.into()),
        },
        "ordinance_decay_products" => {
            let (e, h) = s.ordinance_decay_products();
            floats(&[e, h])
        }
        "ordinance_modifiers" => {
            let st = s.snapshot();
            let list = if bool_of(a(0)?, "school")? {
                &st.school_effects
            } else {
                &st.health_effects
            };
            let (m1, m2, m3) = ordinance_modifiers(list);
            floats(&[m1, m2, m3])
        }
        "school_system_rating" => jf(s.school_system_rating()),
        "health_system_rating" => jf(s.health_system_rating()),
        "school_system_totals" | "hospital_system_totals" => {
            let groups = arr(a(0)?, "groups")?
                .iter()
                .map(|x| u32_of(x, "group"))
                .collect::<R<Vec<u32>>>()?;
            let mut q = QueryData::default();
            let n = s.system_totals(method == "school_system_totals", &groups, &mut q);
            Json::Object(vec![("count".into(), ji(n)), ("totals".into(), query_json(&q))])
        }
        "school_query_data" | "hospital_query_data" => {
            let occ = i64_of(a(0)?, "occupant")? as u64;
            let q = if method == "school_query_data" {
                s.school_query_data(occ)
            } else {
                s.hospital_query_data(occ)
            };
            q.as_ref().map_or(Json::Null, query_json)
        }
        "estimate_occupant_capacity" => {
            let (cap, staff) = s.estimate_occupant_capacity(i64_of(a(0)?, "occupant")? as u64);
            Json::Array(vec![ji(cap), ji(staff)])
        }
        "occupant_coverage" => {
            match s.occupant_coverage(i64_of(a(0)?, "occupant")? as u64, float_of(a(1)?, "level")?) {
                None => Json::Null,
                Some((d, r)) => floats(&[d, r]),
            }
        }
        "estimate_ideal_funding" => s
            .estimate_ideal_funding(i64_of(a(0)?, "occupant")? as u64)
            .map_or(Json::Null, jf),
        "get_proximity" => match s.get_proximity_map(u32_of(a(0)?, "wealth")?) {
            None => Json::Null,
            Some(g) => ji(g.get(ai(1)?, ai(2)?)),
        },
        "add_ref" => ji(s.add_ref()),
        "release" => ji(s.release()),
        "class_id" => ji(s.class_id()),
        "tuning" => {
            let t = s.snapshot().population_tuning;
            Json::Object(vec![
                ("eq_decay".into(), jf(t.eq_decay)),
                ("hq_decay".into(), jf(t.hq_decay)),
                ("new_age".into(), ints(&t.new_age)),
                ("reset_age".into(), ji(t.reset_age)),
                ("eq_reset".into(), jf(t.eq_reset)),
                ("hq_reset".into(), jf(t.hq_reset)),
                ("new_eq".into(), floats(&t.new_eq)),
                ("new_hq".into(), floats(&t.new_hq)),
                ("deaths_door_hq".into(), jf(t.deaths_door_hq)),
                ("effect_height".into(), jf(t.effect_height)),
                ("effect_delta".into(), jf(t.effect_delta)),
                ("effect_chance".into(), jf(t.effect_chance)),
            ])
        }
        _ => return Err(format!("unknown method '{method}'")),
    };
    check_call(op, with_bits(op, got)).map_err(|e| format!("{method}: {e}"))
}

/// `expect`, with objects compared key by key when `partial` is true.
fn check_call(op: &Json, got: Json) -> R<()> {
    let partial = opt(op, "partial").and_then(boolean).unwrap_or(false);
    match op.get("expect") {
        Some(w) if partial => {
            if scalar::matches(&got, w, true) {
                Ok(())
            } else {
                Err(format!("returned {}, expected {}", scalar::show(&got), scalar::show(w)))
            }
        }
        _ => check_expect(op, got),
    }
}

/// Slots 23–25: `{result, instances, added}`, where `added` is how much
/// each grid's reference count rose for the caller.
fn grid_triple(s: &ResidentialSim, which: &str) -> R<Json> {
    fn counts<T: GridElem>(g: &[Option<Rc<dyn TractGrid<T>>>; 3]) -> Vec<usize> {
        g.iter().map(|g| g.as_ref().map_or(0, Rc::strong_count)).collect()
    }
    fn inst<T: GridElem>(g: &[Option<Rc<dyn TractGrid<T>>>; 3]) -> Json {
        Json::Array(
            g.iter()
                .map(|g| g.as_ref().map_or(Json::Null, |g| ji(instance_of(g))))
                .collect(),
        )
    }
    let (before, after, instances) = match which {
        "eq" | "hq" => {
            let get = || {
                if which == "eq" {
                    s.get_eq_grids()
                } else {
                    s.get_hq_grids()
                }
            };
            let held = get();
            let before = counts(&held);
            let more = get();
            (before, counts(&more), inst(&more))
        }
        "population" => {
            let held = s.get_population_grids();
            let before = counts(&held);
            let more = s.get_population_grids();
            (before, counts(&more), inst(&more))
        }
        other => return Err(format!("unknown grid triple '{other}'")),
    };
    let added: Vec<i64> = before.iter().zip(&after).map(|(b, a)| *a as i64 - *b as i64).collect();
    Ok(Json::Object(vec![
        ("result".into(), ji(GRID_TRIPLE_RESULT)),
        ("instances".into(), instances),
        ("added".into(), ints(&added)),
    ]))
}

// ---- state --------------------------------------------------------------------------------------

fn list_json(l: &[Option<u64>]) -> Json {
    Json::Array(l.iter().map(|o| o.map_or(Json::Null, |o| ji(o as i64))).collect())
}

fn list_of(v: &Json) -> R<Vec<Option<u64>>> {
    arr(v, "list")?
        .iter()
        .map(|x| match x {
            Json::Null => Ok(None),
            x => Ok(Some(i64_of(x, "occupant")? as u64)),
        })
        .collect()
}

fn block_json(b: &DepartmentBlock) -> Json {
    let tok = |t: Option<u64>| t.map_or(Json::Null, |t| ji(t as i64));
    Json::Object(vec![
        ("f0".into(), jf(b.f0)),
        ("max_funding".into(), jf(b.max_funding)),
        ("strike_efficiency".into(), jf(b.strike_efficiency)),
        ("strike_duration".into(), ji(b.strike_duration)),
        ("min_grade".into(), jf(b.min_grade)),
        ("strike_flag".into(), ji(b.strike_flag)),
        ("strike_start".into(), ji(b.strike_start)),
        ("init_funding".into(), jf(b.init_funding)),
        ("hundred".into(), jf(b.hundred)),
        ("id".into(), ji(b.id)),
        ("department1".into(), tok(b.department1)),
        ("department2".into(), tok(b.department2)),
    ])
}

fn set_block(b: &mut DepartmentBlock, v: &Json) -> R<()> {
    for (k, x) in v.as_object().ok_or("a department block must be an object")? {
        match k.as_str() {
            "f0" => b.f0 = float_of(x, k)?,
            "max_funding" => b.max_funding = float_of(x, k)?,
            "strike_efficiency" => b.strike_efficiency = float_of(x, k)?,
            "strike_duration" => b.strike_duration = i64_of(x, k)? as i32,
            "min_grade" => b.min_grade = float_of(x, k)?,
            "strike_flag" => b.strike_flag = u32_of(x, k)? as u8,
            "strike_start" => b.strike_start = i64_of(x, k)? as i32,
            "init_funding" => b.init_funding = float_of(x, k)?,
            "hundred" => b.hundred = float_of(x, k)?,
            other => return Err(format!("unknown block field '{other}'")),
        }
    }
    Ok(())
}

const LISTS: [&str; 6] = [
    "pending_school",
    "pending_hospital",
    "other",
    "school_groups",
    "hospital_groups",
    "health_affecting",
];

fn lists_json(l: &OccupantLists) -> Json {
    Json::Object(vec![
        ("pending_school".into(), list_json(&l.pending_school)),
        ("pending_hospital".into(), list_json(&l.pending_hospital)),
        ("other".into(), list_json(&l.other)),
        (
            "school_groups".into(),
            Json::Array(l.school_groups.iter().map(|g| list_json(g)).collect()),
        ),
        (
            "hospital_groups".into(),
            Json::Array(l.hospital_groups.iter().map(|g| list_json(g)).collect()),
        ),
        ("health_affecting".into(), list_json(&l.health_affecting)),
    ])
}

fn set_lists(l: &mut OccupantLists, v: &Json) -> R<()> {
    for (k, x) in v.as_object().ok_or("lists must be an object")? {
        match k.as_str() {
            "pending_school" => l.pending_school = list_of(x)?,
            "pending_hospital" => l.pending_hospital = list_of(x)?,
            "other" => l.other = list_of(x)?,
            "health_affecting" => l.health_affecting = list_of(x)?,
            "school_groups" | "hospital_groups" => {
                let groups = arr(x, k)?;
                let dest: &mut [Vec<Option<u64>>] = if k == "school_groups" {
                    &mut l.school_groups
                } else {
                    &mut l.hospital_groups
                };
                for (i, g) in groups.iter().enumerate() {
                    *dest.get_mut(i).ok_or("too many groups")? = list_of(g)?;
                }
            }
            other => return Err(format!("unknown list '{other}' (known: {})", LISTS.join(", "))),
        }
    }
    Ok(())
}

fn histogram_of(v: &Json, dest: &mut [i32; 20]) -> R<()> {
    match v {
        Json::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                *dest.get_mut(i).ok_or("more than 20 buckets")? = i64_of(x, "bucket")? as i32;
            }
        }
        v => {
            for (k, x) in v.as_object().ok_or("a histogram is an array or {\"bucket\": value}")? {
                let b: usize = k.parse().map_err(|_| format!("bad bucket '{k}'"))?;
                *dest.get_mut(b).ok_or("bucket out of range")? = i64_of(x, "bucket")? as i32;
            }
        }
    }
    Ok(())
}

fn iterator_json(s: &ResidentialState) -> Json {
    let it = &s.iterator;
    Json::Object(vec![
        (
            "state".into(),
            Json::Str(
                match it.state {
                    ScanState::Fresh => "fresh",
                    ScanState::Running => "running",
                    ScanState::Done => "done",
                }
                .into(),
            ),
        ),
        ("x".into(), ji(it.x)),
        ("z".into(), ji(it.z)),
        ("step".into(), ji(it.step)),
        ("rect".into(), ints(&[it.x0, it.z0, it.x1, it.z1])),
    ])
}

fn set(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    if let Some(v) = opt(op, "work_budget") {
        s.set_work_budget(u32_of(v, "work_budget")?);
    }
    let Some(want) = opt(op, "state") else {
        return Ok(());
    };
    let mut st = s.snapshot();
    for (k, v) in want.as_object().ok_or("'state' must be an object")? {
        match k.as_str() {
            "state" => st.state = u32_of(v, k)?,
            "dirty" => st.dirty = bool_of(v, k)?,
            "population" => st.population = i64_of(v, k)? as i32,
            "running_population" => st.running_population = i64_of(v, k)? as i32,
            "le_sum" => st.le_sum = i64_of(v, k)? as u64,
            "age_count" => histogram_of(v, &mut st.age_count)?,
            "age_eq_sum" => histogram_of(v, &mut st.age_eq_sum)?,
            "school_covered" => st.school_covered = i64_of(v, k)? as i32,
            "hospital_covered" => st.hospital_covered = i64_of(v, k)? as i32,
            "proximity_size" => st.proximity_size = i64_of(v, k)? as i32,
            "unused" => {
                let a = arr(v, k)?;
                st.unused_float1 = float_of(&a[0], k)?;
                st.unused_float2 = float_of(&a[1], k)?;
            }
            "health" => set_block(&mut st.health, v)?,
            "education" => set_block(&mut st.education, v)?,
            "lists" => set_lists(&mut st.lists, v)?,
            "school_auto_budget" => st.school_auto_budget = bool_of(v, k)?,
            "hospital_auto_budget" => st.hospital_auto_budget = bool_of(v, k)?,
            "iterator" => {
                let it = &mut st.iterator;
                if let Some(x) = opt(v, "step") {
                    it.set_step(i64_of(x, "step")? as i32);
                }
                if let Some(x) = opt(v, "advance") {
                    for _ in 0..i64_of(x, "advance")? {
                        it.next_cell();
                    }
                }
            }
            other => return Err(format!("unknown state field '{other}'")),
        }
    }
    s.with_state(|x| *x = st);
    Ok(())
}

fn state_fields(s: &ResidentialState) -> Vec<(String, Json)> {
    let tok = |t: Option<u64>| t.map_or(Json::Null, |t| ji(t as i64));
    vec![
        ("initialized".into(), Json::Bool(s.initialized)),
        ("city_started".into(), Json::Bool(s.city_started)),
        ("ref_count".into(), ji(s.ref_count)),
        ("destroyed".into(), Json::Bool(s.destroyed)),
        ("message_server_held".into(), Json::Bool(s.message_server_held)),
        ("proximity_size".into(), ji(s.proximity_size)),
        ("population_size".into(), ji(s.population_size)),
        ("eq_size".into(), ji(s.eq_size)),
        ("state".into(), ji(s.state)),
        ("dirty".into(), Json::Bool(s.dirty)),
        ("population".into(), ji(s.population)),
        ("running_population".into(), ji(s.running_population)),
        ("le_sum".into(), ji(s.le_sum as i64)),
        ("age_count".into(), ints(&s.age_count)),
        ("age_eq_sum".into(), ints(&s.age_eq_sum)),
        ("school_covered".into(), ji(s.school_covered)),
        ("hospital_covered".into(), ji(s.hospital_covered)),
        ("unused".into(), floats(&[s.unused_float1, s.unused_float2])),
        ("iterator".into(), iterator_json(s)),
        ("tuning_loaded".into(), Json::Bool(s.tuning_loaded)),
        ("health".into(), block_json(&s.health)),
        ("education".into(), block_json(&s.education)),
        ("lists".into(), lists_json(&s.lists)),
        ("school_auto_budget".into(), Json::Bool(s.school_auto_budget)),
        ("hospital_auto_budget".into(), Json::Bool(s.hospital_auto_budget)),
        ("temporary".into(), Json::Bool(s.grids.temporary.is_some())),
        ("school_grade_total".into(), jf(s.school_grade_total)),
        ("hospital_grade_total".into(), jf(s.hospital_grade_total)),
        ("school_effects".into(), ji(s.school_effects.len() as i64)),
        ("health_effects".into(), ji(s.health_effects.len() as i64)),
        (
            "departments".into(),
            Json::Array(vec![
                tok(s.health.department1),
                tok(s.health.department2),
                tok(s.education.department1),
                tok(s.education.department2),
            ]),
        ),
    ]
}

/// `state`: a partial object; nested objects (`health`, `education`,
/// `lists`, `iterator`) are compared key by key.
fn expect(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let got: Vec<(String, Json)> = state_fields(&s.snapshot())
        .into_iter()
        .map(|(k, v)| (k, with_bits(op, v)))
        .collect();
    let want = field(op, "state")?;
    let mut errors = Vec::new();
    for (k, v) in want.as_object().ok_or("'state' must be an object")? {
        match got.iter().find(|(gk, _)| gk == k) {
            None => errors.push(format!("unknown state field '{k}'")),
            Some((_, g)) => {
                let partial = matches!(v, Json::Object(_));
                if !scalar::matches(g, v, partial) {
                    errors.push(format!("{k}: got {}, expected {}", scalar::show(g), scalar::show(v)));
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

// ---- grids --------------------------------------------------------------------------------------

#[derive(Clone)]
enum AnyGrid {
    U8(Rc<dyn TractGrid<u8>>),
    U16(Rc<dyn TractGrid<u16>>),
    I16(Rc<dyn TractGrid<i16>>),
    F32(Rc<dyn TractGrid<f32>>),
}

impl AnyGrid {
    fn geometry(&self) -> TractGeometry {
        match self {
            AnyGrid::U8(g) => g.geometry(),
            AnyGrid::U16(g) => g.geometry(),
            AnyGrid::I16(g) => g.geometry(),
            AnyGrid::F32(g) => g.geometry(),
        }
    }
    fn instance(&self) -> u32 {
        match self {
            AnyGrid::U8(g) => instance_of(g),
            AnyGrid::U16(g) => instance_of(g),
            AnyGrid::I16(g) => instance_of(g),
            AnyGrid::F32(g) => instance_of(g),
        }
    }
    fn get(&self, x: i32, z: i32) -> Json {
        match self {
            AnyGrid::U8(g) => ji(g.get(x, z)),
            AnyGrid::U16(g) => ji(g.get(x, z)),
            AnyGrid::I16(g) => ji(g.get(x, z)),
            AnyGrid::F32(g) => jf(g.get(x, z)),
        }
    }
    fn set(&self, x: i32, z: i32, v: &Json) -> R<()> {
        match self {
            AnyGrid::U8(g) => g.set(x, z, u32_of(v, "value")? as u8),
            AnyGrid::U16(g) => g.set(x, z, u32_of(v, "value")? as u16),
            AnyGrid::I16(g) => g.set(x, z, i64_of(v, "value")? as i16),
            AnyGrid::F32(g) => g.set(x, z, float_of(v, "value")?),
        }
        Ok(())
    }
}

/// A grid by name: `proximity1`–`3`, `population1`–`3`, `eq1`–`3`,
/// `hq1`–`3`, `age`, `school_grade`, `hospital_grade`, `school_coverage0`–`6`,
/// `hospital_coverage0`–`2`, `temporary`, `developer1`–`3`, `building_map`.
fn grid_named(w: &World, name: &str) -> R<Option<AnyGrid>> {
    let s = w.sim()?;
    let g = s.snapshot().grids;
    let index = |prefix: &str, base: usize, n: usize| -> Option<R<usize>> {
        let rest = name.strip_prefix(prefix)?;
        Some(
            rest.parse::<usize>()
                .ok()
                .and_then(|i| i.checked_sub(base))
                .filter(|&i| i < n)
                .ok_or_else(|| format!("bad grid '{name}'")),
        )
    };
    Ok(match name {
        "age" => g.age.map(AnyGrid::U8),
        "school_grade" => g.school_grade.map(AnyGrid::F32),
        "hospital_grade" => g.hospital_grade.map(AnyGrid::F32),
        "temporary" => g.temporary.map(AnyGrid::U8),
        "building_map" => w
            .building_map
            .borrow()
            .clone()
            .map(|m| AnyGrid::U8(m as Rc<dyn TractGrid<u8>>)),
        _ => {
            if let Some(i) = index("school_coverage", 0, 7) {
                g.school_coverage[i?].clone().map(AnyGrid::U8)
            } else if let Some(i) = index("hospital_coverage", 0, 3) {
                g.hospital_coverage[i?].clone().map(AnyGrid::U8)
            } else if let Some(i) = index("proximity", 1, 3) {
                g.proximity[i?].clone().map(AnyGrid::U8)
            } else if let Some(i) = index("population", 1, 3) {
                g.population[i?].clone().map(AnyGrid::U16)
            } else if let Some(i) = index("developer", 1, 3) {
                Some(AnyGrid::I16(w.developer(i?) as Rc<dyn TractGrid<i16>>))
            } else if let Some(i) = index("eq", 1, 3) {
                g.eq[i?].clone().map(AnyGrid::F32)
            } else if let Some(i) = index("hq", 1, 3) {
                g.hq[i?].clone().map(AnyGrid::F32)
            } else {
                return Err(format!("unknown grid '{name}'"));
            }
        }
    })
}

fn grid_set(w: &World, op: &Json) -> R<()> {
    let name = str_of(field(op, "grid")?, "grid")?;
    let g = grid_named(w, name)?.ok_or_else(|| format!("grid '{name}' does not exist"))?;
    if let Some(v) = opt(op, "all") {
        let geom = g.geometry();
        for x in 0..geom.count_x {
            for z in 0..geom.count_z {
                g.set(x, z, v)?;
            }
        }
    }
    if let Some(cells) = opt(op, "cells") {
        for c in arr(cells, "cells")? {
            let c = arr(c, "cell")?;
            g.set(i64_of(&c[0], "x")? as i32, i64_of(&c[1], "z")? as i32, &c[2])?;
        }
    }
    Ok(())
}

/// Checks any of `exists`, `size`, `counts`, `instance`, `cells`
/// (`[[x, z, v], …]`), `all` (every tract), `others` (every tract not in
/// `cells`).
fn expect_grid(w: &World, op: &Json) -> R<()> {
    let name = str_of(field(op, "grid")?, "grid")?;
    let g = grid_named(w, name)?;
    if let Some(v) = opt(op, "exists") {
        if g.is_some() != bool_of(v, "exists")? {
            return Err(format!("grid '{name}' exists: {}", g.is_some()));
        }
    }
    let Some(g) = g else {
        return if ["size", "counts", "instance", "cells", "all", "others"]
            .iter()
            .any(|k| opt(op, k).is_some())
        {
            Err(format!("grid '{name}' does not exist"))
        } else {
            Ok(())
        };
    };
    let geom = g.geometry();
    let mut errors = Vec::new();
    if let Some(v) = opt(op, "size") {
        if i64::from(geom.tract_size) != i64_of(v, "size")? {
            errors.push(format!("size {}", geom.tract_size));
        }
    }
    if let Some(v) = opt(op, "counts") {
        let (x, z) = pair_i(v, "counts")?;
        if (i64::from(geom.count_x), i64::from(geom.count_z)) != (x, z) {
            errors.push(format!("counts ({}, {})", geom.count_x, geom.count_z));
        }
    }
    if let Some(v) = opt(op, "instance") {
        if i64::from(g.instance()) != i64_of(v, "instance")? {
            errors.push(format!("instance 0x{:08X}", g.instance()));
        }
    }
    let mut listed = Vec::new();
    if let Some(cells) = opt(op, "cells") {
        for c in arr(cells, "cells")? {
            let c = arr(c, "cell")?;
            let (x, z) = (i64_of(&c[0], "x")? as i32, i64_of(&c[1], "z")? as i32);
            listed.push((x, z));
            let got = with_bits(op, g.get(x, z));
            if !scalar::matches(&got, &c[2], false) {
                errors.push(format!(
                    "({x}, {z}) = {}, expected {}",
                    scalar::show(&got),
                    scalar::show(&c[2])
                ));
            }
        }
    }
    for (k, skip_listed) in [("all", false), ("others", true)] {
        if let Some(v) = opt(op, k) {
            for x in 0..geom.count_x {
                for z in 0..geom.count_z {
                    if skip_listed && listed.contains(&(x, z)) {
                        continue;
                    }
                    let got = with_bits(op, g.get(x, z));
                    if !scalar::matches(&got, v, false) {
                        errors.push(format!(
                            "({x}, {z}) = {}, expected {}",
                            scalar::show(&got),
                            scalar::show(v)
                        ));
                    }
                }
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("grid '{name}': {}", errors.join("; ")))
    }
}

fn expect_refs(w: &World, op: &Json) -> R<()> {
    let refs = w.refs.borrow();
    for (k, v) in field(op, "refs")?.as_object().ok_or("refs must be an object")? {
        let id = Json::Str(k.clone()).as_i64().ok_or("bad ID")? as u64;
        let got = refs.get(&id).copied().unwrap_or(0);
        if got != i64_of(v, "count")? {
            return Err(format!("object {k} holds {got} references"));
        }
    }
    Ok(())
}

// ---- the record (§9) ----------------------------------------------------------------------------

/// Resolves references like the class-object store: grids are the saved
/// core objects; host objects come with one reference for the reader.
struct Resolver(Vec<RegisteredObject>, Rc<World>);

impl RefResolver for Resolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        let e = self.0.iter().find(|e| e.0 == class_id && e.1 == object_id)?;
        Some(match &e.2 {
            Some(o) => RefValue::Object(o.clone()),
            None => {
                self.1.add_ref(e.3);
                RefValue::Host { token: e.3, class_id }
            }
        })
    }
}

/// `Write` into a slot; `expect` checks the result, `length` the size.
fn write(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut out = ByteWriter::with_registry(reg.clone());
    check_expect(op, Json::Bool(s.write(&mut out)))?;
    let bytes = out.into_bytes();
    if let Some(v) = opt(op, "length") {
        if bytes.len() as i64 != i64_of(v, "length")? {
            return Err(format!("record is {} bytes", bytes.len()));
        }
    }
    let entries = reg.borrow().entries.clone();
    let slot = str_of(field(op, "slot")?, "slot")?.to_string();
    w.records.borrow_mut().insert(slot, Record { bytes, entries });
    Ok(())
}

/// `Read` from a slot (or `hex`), optionally with another `version`, cut to
/// `length` bytes and with `patch` (`[{offset, hex}]`) applied. Without
/// `reuse` the record is read into a new constructed simulator (the class
/// store's path), which replaces the current one.
fn read(w: &World, op: &Json) -> R<()> {
    let mut rec = match opt(op, "hex") {
        Some(h) => Record {
            bytes: hex_decode(str_of(h, "hex")?)?,
            entries: Vec::new(),
        },
        None => {
            let slot = str_of(field(op, "slot")?, "slot")?;
            w.records
                .borrow()
                .get(slot)
                .cloned()
                .ok_or_else(|| format!("no record in slot '{slot}'"))?
        }
    };
    if let Some(v) = opt(op, "version") {
        let v = u32_of(v, "version")? as u16;
        rec.bytes
            .get_mut(..2)
            .ok_or("record too short for a version")?
            .copy_from_slice(&v.to_le_bytes());
    }
    if let Some(n) = opt(op, "length") {
        rec.bytes.truncate(i64_of(n, "length")? as usize);
    }
    if let Some(p) = opt(op, "patch") {
        for e in arr(p, "patch")? {
            let at = i64_of(field(e, "offset")?, "offset")? as usize;
            let bytes = hex_decode(str_of(field(e, "hex")?, "hex")?)?;
            rec.bytes
                .get_mut(at..at + bytes.len())
                .ok_or("patch beyond the record")?
                .copy_from_slice(&bytes);
        }
    }
    let reuse = opt(op, "reuse").and_then(boolean).unwrap_or(false);
    let s = if reuse {
        w.sim()?
    } else {
        let s = w.new_sim(w.rng(opt(op, "rng"))?);
        *w.sim.borrow_mut() = Some(s.clone());
        s
    };
    let mut res = Resolver(rec.entries.clone(), w.rc());
    let ok = s.read(&mut ByteReader::with_resolver(&rec.bytes, &mut res));
    check_expect(op, Json::Bool(ok))
}

/// `length`, `at` (`[{offset, hex}]`) and / or `same_as` (another slot whose
/// bytes must be identical) of a saved record.
fn expect_record(w: &World, op: &Json) -> R<()> {
    let slot = str_of(field(op, "slot")?, "slot")?;
    let recs = w.records.borrow();
    let rec = recs.get(slot).ok_or_else(|| format!("no record in slot '{slot}'"))?;
    if let Some(v) = opt(op, "same_as") {
        let other = str_of(v, "same_as")?;
        let o = recs.get(other).ok_or_else(|| format!("no record in slot '{other}'"))?;
        if o.bytes != rec.bytes {
            return Err(format!(
                "records '{slot}' and '{other}' differ:\n  {}\n  {}",
                hex_encode(&rec.bytes),
                hex_encode(&o.bytes)
            ));
        }
    }
    if let Some(v) = opt(op, "length") {
        if rec.bytes.len() as i64 != i64_of(v, "length")? {
            return Err(format!("record is {} bytes", rec.bytes.len()));
        }
    }
    if let Some(at) = opt(op, "at") {
        for e in arr(at, "at")? {
            let off = i64_of(field(e, "offset")?, "offset")? as usize;
            let want = hex_decode(str_of(field(e, "hex")?, "hex")?)?;
            let got = rec.bytes.get(off..off + want.len()).ok_or("beyond the record")?;
            if got != want.as_slice() {
                return Err(format!(
                    "bytes at {off}: {}, expected {}",
                    hex_encode(got),
                    hex_encode(&want)
                ));
            }
        }
    }
    Ok(())
}
