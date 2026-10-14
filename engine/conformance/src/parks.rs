// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/parks.md`: a scripted city (services,
//! the budget's departments, budget items and line items, building
//! occupants with their groups and lots, the tuning exemplar, an automata
//! controller manager, a scripted or the default generator) and ops that
//! drive the park manager. See README.md ("Park ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::{Rc, Weak};

use fc4_core::budget::BudgetItem;
use fc4_core::parks::{
    escape_probability, msg, query_interface, shipped_tuning, ParkHost, ParkManager, ParkMessage, ParkRandom,
    CLSID_PARK_MANAGER, OCCUPANT_CLASS, OCCUPANT_TYPE_BUILDING, PROP_PREVIEW_OCCUPANT, PURPOSE_PARK,
};
use fc4_core::property::PropertyHolder;
use fc4_core::stream::{ByteReader, ByteWriter, RefResolver, RefValue, RegisteredObject, SaveObject, SaveRegistry};
use fc4_core::variant::{Elem, Variant};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{boolean, field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, f32_of, holder_of, i64_of, jf, ji, opt, str_of, u32_of,
    EventLog, R,
};

/// The manager's own token (void1 of the escape message).
const MANAGER_TOKEN: u64 = 0xA000;

#[derive(Clone)]
struct Record {
    bytes: Vec<u8>,
    entries: Vec<RegisteredObject>,
}

#[derive(Clone, Default)]
struct Occ {
    occ_type: u32,
    building: bool,
    building_type: u32,
    groups: Vec<u32>,
    preview: bool,
    lot: Option<u64>,
}

#[derive(Clone, Default)]
struct Line {
    expenses: i64,
    secondary: i64,
    display: BTreeSet<u32>,
}

struct Services {
    city: Cell<bool>,
    budget: Cell<bool>,
    lot_manager: Cell<bool>,
    message_server: Cell<bool>,
}

pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    mgr: RefCell<Option<Rc<ParkManager>>>,
    session_seed: Cell<u32>,
    services: Services,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    /// Department ID → token (the token is the ID).
    departments: RefCell<BTreeMap<u32, u64>>,
    create_departments: Cell<bool>,
    funding: RefCell<BTreeMap<u64, f32>>,
    lines: RefCell<BTreeMap<(u64, u32), Line>>,
    create_lines: Cell<bool>,
    refs: RefCell<BTreeMap<u64, i64>>,
    occupants: RefCell<BTreeMap<u64, Occ>>,
    items: RefCell<BTreeMap<(u64, u32), BudgetItem>>,
    conditions: RefCell<BTreeMap<u64, u32>>,
    automata: Cell<bool>,
    stub: Cell<bool>,
    doubles: RefCell<VecDeque<f64>>,
    u32s: RefCell<VecDeque<u32>>,
    records: RefCell<BTreeMap<String, Record>>,
}

/// A line item's token: the department in the high word, the line below.
fn line_token(dept: u64, line: u32) -> u64 {
    (dept << 32) | u64::from(line)
}

/// The scripted generator: queued doubles (then 1.0) and u32s (then
/// n − 1); every draw is an event.
struct Stub(Weak<World>);

impl ParkRandom for Stub {
    fn uniform_f64(&mut self) -> f64 {
        let w = self.0.upgrade().expect("world alive");
        w.events.push("double", vec![]);
        let v = w.doubles.borrow_mut().pop_front();
        v.unwrap_or(1.0)
    }
    fn uniform_u32(&mut self, n: u32) -> u32 {
        let w = self.0.upgrade().expect("world alive");
        w.events.push("u32", vec![("n", ji(n))]);
        let v = w.u32s.borrow_mut().pop_front();
        v.unwrap_or(n.saturating_sub(1))
    }
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            me: RefCell::new(Weak::new()),
            events: EventLog::default(),
            mgr: RefCell::new(None),
            session_seed: Cell::new(0x1234_5678),
            services: Services {
                city: Cell::new(true),
                budget: Cell::new(true),
                lot_manager: Cell::new(true),
                message_server: Cell::new(true),
            },
            tuning: RefCell::new(None),
            departments: RefCell::new(BTreeMap::new()),
            create_departments: Cell::new(true),
            funding: RefCell::new(BTreeMap::new()),
            lines: RefCell::new(BTreeMap::new()),
            create_lines: Cell::new(true),
            refs: RefCell::new(BTreeMap::new()),
            occupants: RefCell::new(BTreeMap::new()),
            items: RefCell::new(BTreeMap::new()),
            conditions: RefCell::new(BTreeMap::new()),
            automata: Cell::new(true),
            stub: Cell::new(true),
            doubles: RefCell::new(VecDeque::new()),
            u32s: RefCell::new(VecDeque::new()),
            records: RefCell::new(BTreeMap::new()),
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn mgr(&self) -> R<Rc<ParkManager>> {
        self.mgr
            .borrow()
            .clone()
            .ok_or_else(|| "no park manager (park_create)".into())
    }

    fn new_mgr(&self) -> Rc<ParkManager> {
        let m = ParkManager::new(self.rc() as Rc<dyn ParkHost>, MANAGER_TOKEN, self.session_seed.get());
        if self.stub.get() {
            m.set_random(Box::new(Stub(self.me.borrow().clone())));
        }
        *self.mgr.borrow_mut() = Some(m.clone());
        m
    }

    fn line_mut(&self, token: u64, f: impl FnOnce(&mut Line)) {
        if let Some(l) = self.lines.borrow_mut().get_mut(&(token >> 32, token as u32)) {
            f(l);
        }
    }

    fn acquire(&self, service: &str, ok: bool) -> bool {
        self.events.push(
            "acquire",
            vec![("service", Json::Str(service.into())), ("ok", Json::Bool(ok))],
        );
        ok
    }

    fn release_service(&self, service: &str) {
        self.events
            .push("release_service", vec![("service", Json::Str(service.into()))]);
    }
}

impl ParkHost for World {
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
    fn acquire_message_server(&self) -> bool {
        self.acquire("message_server", self.services.message_server.get())
    }
    fn subscribe(&self, msg_type: u32) {
        self.events.push("subscribe", vec![("type", ji(msg_type))]);
    }
    fn unsubscribe(&self, msg_type: u32) {
        self.events.push("unsubscribe", vec![("type", ji(msg_type))]);
    }
    fn release_message_server(&self) {
        self.release_service("message_server");
    }
    fn acquire_city(&self) -> bool {
        self.acquire("city", self.services.city.get())
    }
    fn release_city(&self) {
        self.release_service("city");
    }
    fn acquire_budget(&self) -> bool {
        self.acquire("budget", self.services.budget.get())
    }
    fn release_budget(&self) {
        self.release_service("budget");
    }
    fn acquire_lot_manager(&self) -> bool {
        self.acquire("lot_manager", self.services.lot_manager.get())
    }
    fn release_lot_manager(&self) {
        self.release_service("lot_manager");
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.events.push("tuning", vec![]);
        self.tuning.borrow().clone()
    }
    fn add_ref(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) += 1;
        self.events.push("add_ref", vec![("token", ji(token as i64))]);
    }
    fn release(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) -= 1;
        self.events.push("release", vec![("token", ji(token as i64))]);
    }
    fn department_budget(&self, id: u32) -> Option<u64> {
        self.departments.borrow().get(&id).copied()
    }
    fn create_department_budget(&self, id: u32, group: u32) -> Option<u64> {
        self.events
            .push("create_department", vec![("id", ji(id)), ("group", ji(group))]);
        if !self.create_departments.get() {
            return None;
        }
        let t = u64::from(id);
        self.departments.borrow_mut().insert(id, t);
        Some(t)
    }
    fn set_department_name(&self, dept: u64, group: u32, instance: u32) {
        self.events.push(
            "department_name",
            vec![
                ("department", ji(dept as i64)),
                ("group", ji(group)),
                ("instance", ji(instance)),
            ],
        );
    }
    fn set_max_funding(&self, dept: u64, max: f32) {
        self.events
            .push("max_funding", vec![("department", ji(dept as i64)), ("max", jf(max))]);
    }
    fn department_funding(&self, dept: u64) -> Option<f32> {
        self.funding.borrow().get(&dept).copied()
    }
    fn budget_item(&self, occupant: u64, purpose: u32) -> Option<BudgetItem> {
        self.items.borrow().get(&(occupant, purpose)).copied()
    }
    fn change_budget_item_line(&self, occupant: u64, index: i32, line: u32) {
        self.events.push(
            "change_line",
            vec![
                ("occupant", ji(occupant as i64)),
                ("index", ji(index)),
                ("line", ji(line)),
            ],
        );
        for ((o, _), i) in self.items.borrow_mut().iter_mut() {
            if *o == occupant && i.index == index {
                i.line_item = line;
            }
        }
    }
    fn line_item_from_budget_item(&self, item: &BudgetItem) -> Option<u64> {
        let dept = self.department_budget(item.department_id)?;
        self.line_item(dept, item.line_item)
    }
    fn line_item(&self, dept: u64, line: u32) -> Option<u64> {
        self.lines
            .borrow()
            .contains_key(&(dept, line))
            .then(|| line_token(dept, line))
    }
    fn create_line_item(&self, dept: u64, line: u32, local: bool) -> Option<u64> {
        self.events.push(
            "create_line",
            vec![
                ("department", ji(dept as i64)),
                ("line", ji(line)),
                ("local", Json::Bool(local)),
            ],
        );
        if !self.create_lines.get() {
            return None;
        }
        self.lines.borrow_mut().insert((dept, line), Line::default());
        Some(line_token(dept, line))
    }
    fn set_line_display_flag(&self, line: u64, flag: u32, on: bool) {
        self.events.push(
            "display_flag",
            vec![("line", ji(line as u32)), ("flag", ji(flag)), ("on", Json::Bool(on))],
        );
        self.line_mut(line, |l| {
            if on {
                l.display.insert(flag);
            } else {
                l.display.remove(&flag);
            }
        });
    }
    fn add_to_full_expenses(&self, line: u64, amount: i64) {
        self.line_mut(line, |l| l.expenses = l.expenses.wrapping_add(amount));
    }
    fn line_secondary(&self, line: u64) -> i64 {
        let mut v = 0;
        self.line_mut(line, |l| v = l.secondary);
        v
    }
    fn set_line_secondary(&self, line: u64, value: i64) {
        self.line_mut(line, |l| l.secondary = value);
    }
    fn occupant_type(&self, occupant: u64) -> u32 {
        self.occupants.borrow().get(&occupant).map_or(0, |o| o.occ_type)
    }
    fn occupant_has_property(&self, occupant: u64, id: u32) -> bool {
        id == PROP_PREVIEW_OCCUPANT && self.occupants.borrow().get(&occupant).is_some_and(|o| o.preview)
    }
    fn is_occupant_group(&self, occupant: u64, group: u32) -> bool {
        self.occupants
            .borrow()
            .get(&occupant)
            .is_some_and(|o| o.groups.contains(&group))
    }
    fn is_building_occupant(&self, occupant: u64) -> bool {
        self.occupants.borrow().get(&occupant).is_some_and(|o| o.building)
    }
    fn building_type(&self, building: u64) -> u32 {
        self.occupants.borrow().get(&building).map_or(0, |o| o.building_type)
    }
    fn occupant_position_lot(&self, occupant: u64) -> Option<u64> {
        self.occupants.borrow().get(&occupant)?.lot
    }
    fn set_lot_condition(&self, lot: u64, condition: u32) {
        self.events.push(
            "set_condition",
            vec![("lot", ji(lot as i64)), ("condition", ji(condition))],
        );
        self.conditions.borrow_mut().insert(lot, condition);
    }
    fn post_zoo_escape(&self, manager: u64, zoo: u64) {
        self.events.push(
            "post",
            vec![
                ("type", ji(msg::ZOO_ESCAPE)),
                ("void1", ji(manager as i64)),
                ("unknown", ji(zoo as i64)),
            ],
        );
    }
    fn add_controllers(&self, name: &str, occupant: u64) {
        if self.automata.get() {
            self.events.push(
                "controllers",
                vec![("name", Json::Str(name.into())), ("occupant", ji(occupant as i64))],
            );
        }
    }
    fn object_ref(&self, token: u64) -> RefValue {
        RefValue::Host {
            token,
            class_id: OCCUPANT_CLASS,
        }
    }
    /// A known occupant resolves to its token; anything else does not, and
    /// the reference the stream handed over is dropped.
    fn resolve_ref(&self, r: &RefValue, _iid: u32) -> Option<u64> {
        let RefValue::Host { token, .. } = *r else {
            return None;
        };
        if token != 0 && self.occupants.borrow().contains_key(&token) {
            return Some(token);
        }
        *self.refs.borrow_mut().entry(token).or_insert(0) -= 1;
        None
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "parks", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("park_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "building" => building(&w, op),
        "item" => item(&w, op),
        "line" => line(&w, op),
        "create" => {
            if let Some(v) = opt(op, "session_seed") {
                match u32_of(v, "session_seed") {
                    Ok(s) => w.session_seed.set(s),
                    Err(e) => return Some(Err(e)),
                }
            }
            w.new_mgr();
            Ok(())
        }
        "init" => w.mgr().and_then(|m| check_expect(op, Json::Bool(m.init()))),
        "shutdown" => w.mgr().and_then(|m| check_expect(op, Json::Bool(m.shutdown()))),
        "add_ref" => w.mgr().and_then(|m| check_expect(op, ji(m.add_ref()))),
        "release" => w.mgr().and_then(|m| check_expect(op, ji(m.release()))),
        "identity" => identity(&w, op),
        "message" => message(&w, op),
        "call" => call(&w, op),
        "set" => set(&w, op),
        "expect" => expect(&w, op),
        "expect_line" => expect_line(&w, op),
        "expect_item" => expect_item(&w, op),
        "expect_conditions" => expect_conditions(&w, op),
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

// ---- environment ----------------------------------------------------------------------------------

fn key_of(k: &str) -> R<u64> {
    Json::Str(k.to_owned())
        .as_i64()
        .map(|v| v as u64)
        .ok_or_else(|| format!("bad key '{k}'"))
}

fn token_of(j: &Json, what: &str) -> R<u64> {
    Ok(i64_of(j, what)? as u64)
}

fn funding_of(v: &Json) -> R<f32> {
    match v.as_str() {
        Some("nan") | Some("NaN") => Ok(f32::NAN),
        _ => f32_of(v, "funding"),
    }
}

/// The tuning exemplar: "shipped", "no_curve" (the shipped threshold
/// only), null (no exemplar) or `{"props": {...}}`.
fn tuning_of(v: &Json) -> R<Option<PropertyHolder>> {
    Ok(match v {
        Json::Null => None,
        Json::Str(s) if s == "shipped" => Some(shipped_tuning()),
        Json::Str(s) if s == "no_curve" => {
            let mut h = PropertyHolder::new();
            h.add(
                fc4_core::parks::prop::DISTRESS_THRESHOLD,
                Variant::Scalar(Elem::F32(fc4_core::parks::SHIPPED_THRESHOLD)),
            );
            Some(h)
        }
        other => Some(holder_of(field(other, "props")?)?),
    })
}

/// `park_env`: `services` (`{"city", "budget", "lot_manager",
/// "message_server": bool}`, all present by default), `departments` (IDs
/// that already exist), `create_departments` / `create_lines` (whether
/// creation succeeds, default true), `funding` (`{"0xID": fraction, "nan"
/// or null}`; null or absent = unreadable), `tuning`, `automata` (the automata
/// controller manager exists), `random` ("stub" or "default"), `doubles` /
/// `u32s` (the stub generator's queues).
fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "services") {
        for (k, on) in v.as_object().ok_or("services must be an object")? {
            let on = bool_of(on, k)?;
            match k.as_str() {
                "city" => w.services.city.set(on),
                "budget" => w.services.budget.set(on),
                "lot_manager" => w.services.lot_manager.set(on),
                "message_server" => w.services.message_server.set(on),
                other => return Err(format!("unknown service '{other}'")),
            }
        }
    }
    if let Some(v) = opt(op, "departments") {
        for d in arr(v, "departments")? {
            let id = u32_of(d, "department")?;
            w.departments.borrow_mut().insert(id, u64::from(id));
        }
    }
    if let Some(v) = opt(op, "create_departments") {
        w.create_departments.set(bool_of(v, "create_departments")?);
    }
    if let Some(v) = opt(op, "create_lines") {
        w.create_lines.set(bool_of(v, "create_lines")?);
    }
    if let Some(v) = opt(op, "funding") {
        for (k, f) in v.as_object().ok_or("funding must be an object")? {
            if *f == Json::Null {
                w.funding.borrow_mut().remove(&key_of(k)?);
            } else {
                w.funding.borrow_mut().insert(key_of(k)?, funding_of(f)?);
            }
        }
    }
    if let Some(v) = op.get("tuning") {
        *w.tuning.borrow_mut() = tuning_of(v)?.map(Rc::new);
    }
    if let Some(v) = opt(op, "automata") {
        w.automata.set(bool_of(v, "automata")?);
    }
    if let Some(v) = opt(op, "random") {
        match str_of(v, "random")? {
            "stub" => w.stub.set(true),
            "default" => w.stub.set(false),
            other => return Err(format!("unknown generator '{other}'")),
        }
    }
    if let Some(v) = opt(op, "doubles") {
        for d in arr(v, "doubles")? {
            w.doubles
                .borrow_mut()
                .push_back(scalar::num(d).ok_or("doubles must be numbers")?);
        }
    }
    if let Some(v) = opt(op, "u32s") {
        for d in arr(v, "u32s")? {
            w.u32s.borrow_mut().push_back(u32_of(d, "u32")?);
        }
    }
    Ok(())
}

fn group_of(j: &Json) -> R<u32> {
    match j.as_str() {
        Some("park") => Ok(fc4_core::parks::GROUP_PARK),
        Some("zoo") => Ok(fc4_core::parks::GROUP_ZOO),
        _ => u32_of(j, "group"),
    }
}

/// `park_building`: an occupant (`occupant`) with `occupant_type` (default
/// the building type 0x278128A0), `building` (the building interface,
/// default true), `type` (the building type), `groups` ("park", "zoo" or
/// IDs), `preview` (property 0x029244DB present), `lot` (the lot at its
/// position, default occupant + 1; null = none) and `condition` (the lot's,
/// default 0).
fn building(w: &World, op: &Json) -> R<()> {
    let occ = token_of(field(op, "occupant")?, "occupant")?;
    let mut o = Occ {
        occ_type: opt(op, "occupant_type").map_or(Ok(OCCUPANT_TYPE_BUILDING), |v| u32_of(v, "occupant_type"))?,
        building: opt(op, "building").map_or(Ok(true), |v| bool_of(v, "building"))?,
        building_type: opt(op, "type").map_or(Ok(0), |v| u32_of(v, "type"))?,
        preview: opt(op, "preview").map_or(Ok(false), |v| bool_of(v, "preview"))?,
        ..Occ::default()
    };
    if let Some(v) = opt(op, "groups") {
        o.groups = arr(v, "groups")?.iter().map(group_of).collect::<R<_>>()?;
    }
    o.lot = match op.get("lot") {
        Some(Json::Null) => None,
        Some(v) => Some(token_of(v, "lot")?),
        None => Some(occ + 1),
    };
    if let Some(l) = o.lot {
        let c = opt(op, "condition").map_or(Ok(0), |v| u32_of(v, "condition"))?;
        w.conditions.borrow_mut().insert(l, c);
    }
    w.occupants.borrow_mut().insert(occ, o);
    Ok(())
}

/// `park_item`: the budget item of `occupant` for `purpose` (default the
/// park purpose 0xCA639989): `department`, `line` (default 0), `cost`,
/// `index` (default 0). `remove: true` deletes it.
fn item(w: &World, op: &Json) -> R<()> {
    let occ = token_of(field(op, "occupant")?, "occupant")?;
    let purpose = opt(op, "purpose").map_or(Ok(PURPOSE_PARK), |v| u32_of(v, "purpose"))?;
    if opt(op, "remove").and_then(boolean).unwrap_or(false) {
        w.items.borrow_mut().remove(&(occ, purpose));
        return Ok(());
    }
    let it = BudgetItem {
        department_id: u32_of(field(op, "department")?, "department")?,
        line_item: opt(op, "line").map_or(Ok(0), |v| u32_of(v, "line"))?,
        purpose,
        cost: i64_of(field(op, "cost")?, "cost")?,
        index: opt(op, "index").map_or(Ok(0), |v| i64_of(v, "index"))? as i32,
        local_funding: 1.0,
    };
    w.items.borrow_mut().insert((occ, purpose), it);
    Ok(())
}

/// `park_line`: creates (or updates) the line `line` of `department` with
/// `expenses` and `secondary`.
fn line(w: &World, op: &Json) -> R<()> {
    let d = u64::from(u32_of(field(op, "department")?, "department")?);
    let line = u32_of(field(op, "line")?, "line")?;
    let mut lines = w.lines.borrow_mut();
    let l = lines.entry((d, line)).or_default();
    if let Some(v) = opt(op, "expenses") {
        l.expenses = i64_of(v, "expenses")?;
    }
    if let Some(v) = opt(op, "secondary") {
        l.secondary = i64_of(v, "secondary")?;
    }
    Ok(())
}

// ---- calls ------------------------------------------------------------------------------------------

/// `park_identity`: `iids` (`{"0xIID": bool}`) and `class`.
fn identity(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "iids") {
        for (k, want) in v.as_object().ok_or("iids must be an object")? {
            let iid = key_of(k)? as u32;
            if query_interface(iid) != bool_of(want, k)? {
                return Err(format!("QueryInterface({k}) = {}", query_interface(iid)));
            }
        }
    }
    if let Some(v) = opt(op, "class") {
        let want = u32_of(v, "class")?;
        let got = w.mgr().map_or(CLSID_PARK_MANAGER, |m| m.class_id());
        if got != want {
            return Err(format!("class 0x{got:08X}, expected 0x{want:08X}"));
        }
    }
    Ok(())
}

/// `park_message`: `type` (SimNewMonth, OccupantInserted, OccupantRemoved
/// or an ID) and `occupant` (void1).
fn message(w: &World, op: &Json) -> R<()> {
    let m = w.mgr()?;
    let t = match field(op, "type")?.as_str() {
        Some("SimNewMonth") => msg::SIM_NEW_MONTH,
        Some("OccupantInserted") => msg::OCCUPANT_INSERTED,
        Some("OccupantRemoved") => msg::OCCUPANT_REMOVED,
        _ => u32_of(field(op, "type")?, "type")?,
    };
    let pm = ParkMessage {
        msg_type: t,
        void1: opt(op, "occupant").map(|v| token_of(v, "occupant")).transpose()?,
        ..ParkMessage::default()
    };
    check_expect(op, Json::Bool(m.do_message(&pm)))
}

/// `park_call`: `method` (monthly, zoo_escape, escape_probability [f]) with
/// `args`; `expect` checks the result, `expect_bits` (hex) the exact bits
/// of a double result.
fn call(w: &World, op: &Json) -> R<()> {
    let m = w.mgr()?;
    let got = match str_of(field(op, "method")?, "method")? {
        "monthly" => {
            m.monthly();
            Json::Null
        }
        "zoo_escape" => {
            m.zoo_escape();
            Json::Null
        }
        "escape_probability" => {
            let f = funding_of(arg(op, 0)?)?;
            let p = escape_probability(&m.snapshot().escape_curve, f);
            if let Some(b) = opt(op, "expect_bits") {
                let want = u64_bits(b)?;
                if p.to_bits() != want {
                    return Err(format!("p = {p:e} (0x{:016X}), expected 0x{want:016X}", p.to_bits()));
                }
            }
            Json::Float(p)
        }
        other => return Err(format!("unknown park method '{other}'")),
    };
    check_expect(op, got)
}

fn u64_bits(j: &Json) -> R<u64> {
    let s = str_of(j, "bits")?;
    u64::from_str_radix(s.trim_start_matches("0x"), 16).map_err(|e| format!("bad bits '{s}': {e}"))
}

fn tokens(j: &Json, what: &str) -> R<Vec<Option<u64>>> {
    arr(j, what)?
        .iter()
        .map(|e| match e {
            Json::Null => Ok(None),
            v => token_of(v, what).map(Some),
        })
        .collect()
}

/// `park_set` (a test hook): `parks`, `zoos` (token lists, null entries
/// allowed; no references are taken) and `underfunded` (the latch).
fn set(w: &World, op: &Json) -> R<()> {
    let parks = opt(op, "parks").map(|v| tokens(v, "parks")).transpose()?;
    let zoos = opt(op, "zoos").map(|v| tokens(v, "zoos")).transpose()?;
    let latch = opt(op, "underfunded").map(|v| bool_of(v, "underfunded")).transpose()?;
    w.mgr()?.with_state(|s| {
        if let Some(v) = parks {
            s.parks = v;
        }
        if let Some(v) = zoos {
            s.zoos = v;
        }
        if let Some(v) = latch {
            s.underfunded = v;
        }
    });
    Ok(())
}

fn list(v: &[Option<u64>]) -> Json {
    Json::Array(v.iter().map(|e| e.map_or(Json::Null, |t| ji(t as i64))).collect())
}

fn expect(w: &World, op: &Json) -> R<()> {
    let s = w.mgr()?.snapshot();
    let curve = s.escape_curve.points();
    let got = vec![
        ("initialized".to_owned(), Json::Bool(s.initialized)),
        ("destroyed".to_owned(), Json::Bool(s.destroyed)),
        ("ref_count".to_owned(), ji(s.ref_count)),
        ("tuning_read".to_owned(), Json::Bool(s.tuning_read)),
        ("underfunded".to_owned(), Json::Bool(s.underfunded)),
        ("parks".to_owned(), list(&s.parks)),
        ("zoos".to_owned(), list(&s.zoos)),
        ("city_held".to_owned(), Json::Bool(s.city_held)),
        ("budget_held".to_owned(), Json::Bool(s.budget_held)),
        ("message_server_held".to_owned(), Json::Bool(s.message_server_held)),
        ("lot_manager_held".to_owned(), Json::Bool(s.lot_manager_held)),
        (
            "department".to_owned(),
            s.department.map_or(Json::Null, |t| ji(t as i64)),
        ),
        ("threshold".to_owned(), jf(s.distress_threshold)),
        ("curve_points".to_owned(), ji(curve.len() as i64)),
        (
            "curve".to_owned(),
            Json::Array(curve.iter().map(|&(x, y)| Json::Array(vec![jf(x), jf(y)])).collect()),
        ),
    ];
    check_state(&got, field(op, "state")?)
}

/// `park_expect_line`: the line `line` of `department`: `state`
/// (`expenses`, `secondary`, `display` = the display flags set) or
/// `absent: true`.
fn expect_line(w: &World, op: &Json) -> R<()> {
    let d = u64::from(u32_of(field(op, "department")?, "department")?);
    let line = u32_of(field(op, "line")?, "line")?;
    let l = w.lines.borrow().get(&(d, line)).cloned();
    if opt(op, "absent").and_then(boolean).unwrap_or(false) {
        return match l {
            None => Ok(()),
            Some(_) => Err(format!("line 0x{line:08X} exists")),
        };
    }
    let l = l.ok_or_else(|| format!("no line 0x{line:08X} in department 0x{d:08X}"))?;
    let got = vec![
        ("expenses".to_owned(), ji(l.expenses)),
        ("secondary".to_owned(), ji(l.secondary)),
        (
            "display".to_owned(),
            Json::Array(l.display.iter().map(|&f| ji(f)).collect()),
        ),
    ];
    check_state(&got, field(op, "state")?)
}

/// `park_expect_item`: the park budget item of `occupant`: `state`
/// (`line`, `department`, `cost`, `index`).
fn expect_item(w: &World, op: &Json) -> R<()> {
    let occ = token_of(field(op, "occupant")?, "occupant")?;
    let purpose = opt(op, "purpose").map_or(Ok(PURPOSE_PARK), |v| u32_of(v, "purpose"))?;
    let i = w.items.borrow().get(&(occ, purpose)).copied().ok_or("no such item")?;
    let got = vec![
        ("line".to_owned(), ji(i.line_item)),
        ("department".to_owned(), ji(i.department_id)),
        ("cost".to_owned(), ji(i.cost)),
        ("index".to_owned(), ji(i.index)),
    ];
    check_state(&got, field(op, "state")?)
}

/// `park_expect_conditions`: `{"lot": condition}`.
fn expect_conditions(w: &World, op: &Json) -> R<()> {
    for (k, v) in field(op, "conditions")?
        .as_object()
        .ok_or("conditions must be an object")?
    {
        let lot = key_of(k)?;
        let got = w.conditions.borrow().get(&lot).copied().unwrap_or(0);
        if got != u32_of(v, k)? {
            return Err(format!("lot {k}: condition {got}"));
        }
    }
    Ok(())
}

/// `park_expect_refs`: `{"token": references}` taken by the manager.
fn expect_refs(w: &World, op: &Json) -> R<()> {
    for (k, v) in field(op, "refs")?.as_object().ok_or("refs must be an object")? {
        let t = key_of(k)?;
        let got = w.refs.borrow().get(&t).copied().unwrap_or(0);
        if got != i64_of(v, k)? {
            return Err(format!("{k}: {got} references"));
        }
    }
    Ok(())
}

// ---- the record (§7) ---------------------------------------------------------------------------------

/// Resolves references like the class-object store: a host object comes
/// back as its token (the registered one, or the object ID for a hand-made
/// record) with one reference handed to the reader.
struct Resolver(Vec<RegisteredObject>, Rc<World>);

impl RefResolver for Resolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        let token = match self.0.iter().find(|e| e.0 == class_id && e.1 == object_id) {
            Some(e) => e.3,
            None if self.0.is_empty() => u64::from(object_id),
            None => return None,
        };
        *self.1.refs.borrow_mut().entry(token).or_insert(0) += 1;
        Some(RefValue::Host { token, class_id })
    }
}

/// `park_write`: the record into `slot`; `expect`, `length`, `hex`.
fn write(w: &World, op: &Json) -> R<()> {
    let m = w.mgr()?;
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut out = ByteWriter::with_registry(reg.clone());
    check_expect(op, Json::Bool(m.write(&mut out)))?;
    let bytes = out.into_bytes();
    if let Some(v) = opt(op, "length") {
        if bytes.len() as i64 != i64_of(v, "length")? {
            return Err(format!("record is {} bytes", bytes.len()));
        }
    }
    if let Some(h) = opt(op, "hex") {
        let want = hex_decode(str_of(h, "hex")?)?;
        if bytes != want {
            return Err(format!(
                "record differs:\n    got      {}\n    expected {}",
                hex_encode(&bytes),
                hex_encode(&want)
            ));
        }
    }
    let entries = reg.borrow().entries.clone();
    if let Some(slot) = opt(op, "slot") {
        w.records
            .borrow_mut()
            .insert(str_of(slot, "slot")?.to_string(), Record { bytes, entries });
    }
    Ok(())
}

/// `park_read`: a slot's record (or `hex`, whose references resolve to
/// their object IDs), optionally with another `version` or cut to
/// `length`, into a new constructed manager (or the current one with
/// `reuse`).
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
    let reuse = opt(op, "reuse").and_then(boolean).unwrap_or(false);
    let m = if reuse { w.mgr()? } else { w.new_mgr() };
    let mut res = Resolver(rec.entries.clone(), w.rc());
    let ok = m.read(&mut ByteReader::with_resolver(&rec.bytes, &mut res));
    check_expect(op, Json::Bool(ok))
}

/// `park_expect_record`: `same_as` (another slot), `length`, `at`.
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
