// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/automata.md`: record round trips for every
//! class of the layer, the controller manager with a script set (templates,
//! occupant groups, `PostCityInit`), an automata manager's slots, and a
//! scripted city for the riot and fire hooks. See README.md ("Automata
//! ops").

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use fc4_core::automata::{
    AutomataManager, Automaton, Controller, ControllerKind, ControllerManager, ControllerManagerHost,
    ControllerTemplate, ControllerTime, CreateArgs, FireHook, FireHost, ManagerKind, ManagerPart, OccupantGroup,
    RiotHost, RiotLeaders, ScriptSystem, SetStateArgs, TemplateKind, Vec3, CLSID_CONTROLLER_MANAGER, CLSID_PEDESTRIAN,
    CLSID_SCRIPT_SYSTEM, OCCUPANT_TYPE_BUILDING,
};
use fc4_core::stream::{ByteReader, ByteWriter, OutStream, RefResolver, RefValue, SaveObject};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{boolean, field, HostRef};
use crate::scalar::{self, arr, i64_of, matches, opt, show, str_of, u32_of, R};

/// The class of an occupant given by number (a building, `0xA9BD882D`).
const OCCUPANT_CLASS: u32 = 0xA9BD_882D;

// ---- streams ------------------------------------------------------------------------------------------------

/// Resolves references to host tokens `class << 32 | id`; for the controller
/// classes (when `controllers` is set) it builds core controllers instead,
/// with the object ID as their host token.
struct Resolver<'a> {
    controllers: Option<&'a RefCell<BTreeMap<u32, Rc<Controller>>>>,
}

impl RefResolver for Resolver<'_> {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        if let Some(map) = self.controllers {
            if let Some(c) = Controller::for_class(class_id) {
                c.set_host_token(u64::from(object_id));
                map.borrow_mut().insert(object_id, c.clone());
                return Some(RefValue::Object(c));
            }
        }
        Some(RefValue::Host {
            token: (u64::from(class_id) << 32) | u64::from(object_id),
            class_id,
        })
    }
}

/// Writes references as object ID and class: host references from their
/// token, core objects from their host token.
struct RawOut(ByteWriter);

impl OutStream for RawOut {
    fn put_u8(&mut self, v: u8) -> bool {
        self.0.put_u8(v)
    }
    fn put_u16(&mut self, v: u16) -> bool {
        self.0.put_u16(v)
    }
    fn put_u32(&mut self, v: u32) -> bool {
        self.0.put_u32(v)
    }
    fn put_u64(&mut self, v: u64) -> bool {
        self.0.put_u64(v)
    }
    fn put_f32(&mut self, v: f32) -> bool {
        self.0.put_f32(v)
    }
    fn put_f64(&mut self, v: f64) -> bool {
        self.0.put_f64(v)
    }
    fn put_bytes(&mut self, v: &[u8]) -> bool {
        self.0.put_bytes(v)
    }
    fn put_str(&mut self, v: &[u8]) -> bool {
        self.0.put_str(v)
    }
    fn put_ref(&mut self, r: &RefValue) -> bool {
        match r {
            RefValue::Null => self.0.put_u32(0),
            RefValue::Host { token, class_id } => self.0.put_u32(*token as u32) && self.0.put_u32(*class_id),
            RefValue::Object(o) => self.0.put_u32(o.host_token() as u32) && self.0.put_u32(o.class_id()),
        }
    }
    fn error(&self) -> i32 {
        self.0.error()
    }
}

fn raw_out() -> RawOut {
    RawOut(ByteWriter::new())
}

// ---- the world ----------------------------------------------------------------------------------------------

struct LogHost(Rc<RefCell<Vec<String>>>);

impl ControllerManagerHost for LogHost {
    fn log(&self, text: &str) {
        self.0.borrow_mut().push(format!("log {text}"));
    }
    fn release(&self, token: u64) {
        self.0.borrow_mut().push(format!("release {:#x}", token as u32));
    }
    fn add_ref(&self, token: u64) {
        self.0.borrow_mut().push(format!("add_ref {:#x}", token as u32));
    }
}

/// The scripted city of the riot and fire hooks.
#[derive(Default)]
struct City {
    calls: Vec<String>,
    no_road: bool,
    cap: bool,
    create_null: bool,
    buildings: Vec<u64>,
}

impl RiotHost for City {
    fn nearest_road_tile(&mut self, x: i32, z: i32) -> Option<Vec3> {
        self.calls.push(format!("FindRoad {x} {z}"));
        (!self.no_road).then_some([x as f32 * 16.0 + 8.0, 0.0, z as f32 * 16.0 + 8.0])
    }
    fn leader_cap_reached(&mut self, leaders: usize) -> bool {
        self.calls.push(format!("LeaderCap {leaders}"));
        self.cap
    }
    fn create_automaton(&mut self, a: CreateArgs) -> u64 {
        self.calls.push(format!(
            "CreateAutomaton {:#x} ({}, {}, {}) {} ({}, {}, {}) {} {:#x} {}",
            a.group,
            a.position[0],
            a.position[1],
            a.position[2],
            a.mode,
            a.direction[0],
            a.direction[1],
            a.direction[2],
            a.a,
            a.b,
            a.c
        ));
        if self.create_null {
            0
        } else {
            0x500
        }
    }
    fn add_ref(&mut self, a: u64) {
        self.calls.push(format!("AddRef {a:#x}"));
    }
    fn set_state(&mut self, a: u64, t: u32, s: SetStateArgs) {
        self.calls.push(format!(
            "SetState {a:#x} {t} {} {} {} {} {} {}",
            s.controller, s.timeout_ms, s.is_final, s.anim_id, s.sfx_id, s.behaviour_template
        ));
    }
    fn set_visible(&mut self, a: u64, v: bool) {
        self.calls.push(format!("SetVisible {a:#x} {v}"));
    }
    fn as_occupant(&mut self, a: u64) -> u64 {
        a | 0x1_0000
    }
    fn register_location(&mut self, a: u64, _p: Vec3) {
        self.calls.push(format!("RegisterLocation {a:#x}"));
    }
    fn add_controller_from_template_name(&mut self, occ: u64, name: &str, extra: u32) -> bool {
        self.calls
            .push(format!("AddControllerFromTemplateName {occ:#x} {name} {extra}"));
        true
    }
    fn remove_controllers(&mut self, occ: u64) {
        self.calls.push(format!("RemoveControllers {occ:#x}"));
    }
    fn unregister_location(&mut self, occ: u64) {
        self.calls.push(format!("UnregisterLocation {occ:#x}"));
    }
    fn release(&mut self, a: u64) {
        self.calls.push(format!("Release {a:#x}"));
    }
}

impl FireHost for City {
    fn occupant_type(&mut self, occ: u64) -> u32 {
        if self.buildings.contains(&occ) {
            OCCUPANT_TYPE_BUILDING
        } else {
            0x7475_8925
        }
    }
    fn add_controllers_from_occupant_group_name(&mut self, name: &str, occ: u64, extra: u32) -> bool {
        self.calls
            .push(format!("AddControllersFromOccupantGroup {name} {occ:#x} {extra}"));
        true
    }
    fn remove_controllers(&mut self, occ: u64) {
        self.calls.push(format!("RemoveControllers {occ:#x}"));
    }
}

pub(crate) struct World {
    scripts: RefCell<ScriptSystem>,
    log: Rc<RefCell<Vec<String>>>,
    cm: RefCell<ControllerManager>,
    controllers: RefCell<BTreeMap<u32, Rc<Controller>>>,
    next_token: Cell<u32>,
    mgr: RefCell<AutomataManager>,
    riot: RefCell<RiotLeaders>,
    fire: RefCell<FireHook>,
    city: RefCell<City>,
}

impl World {
    fn new() -> Rc<World> {
        let log = Rc::new(RefCell::new(Vec::new()));
        Rc::new(World {
            scripts: RefCell::new(ScriptSystem::new()),
            cm: RefCell::new(ControllerManager::new(Box::new(LogHost(log.clone())))),
            log,
            controllers: RefCell::new(BTreeMap::new()),
            next_token: Cell::new(0x1000),
            mgr: RefCell::new(AutomataManager::new(ManagerKind::Pedestrian)),
            riot: RefCell::new(RiotLeaders::new()),
            fire: RefCell::new(FireHook::new()),
            city: RefCell::new(City::default()),
        })
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "automata", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("automata_")?;
    let w = world(host);
    Some(match rest {
        "record" => record(op),
        "scripts" => scripts(&w, op),
        "cm_new" => {
            *w.cm.borrow_mut() = ControllerManager::new(Box::new(LogHost(w.log.clone())));
            w.controllers.borrow_mut().clear();
            Ok(())
        }
        "cm_read" => cm_read(&w, op),
        "cm_add" => cm_add(&w, op),
        "cm_add_group" => cm_add_group(&w, op),
        "cm_remove" => {
            let occ = occupant(op);
            occ.and_then(|o| check(op, Json::Int(w.cm.borrow_mut().remove_controllers(&o) as i64)))
        }
        "cm_remove_group" => cm_remove_group(&w, op),
        "cm_post_city_init" => {
            let dropped = w.cm.borrow_mut().post_city_init(&*w.scripts.borrow());
            check(op, Json::Array(dropped.into_iter().map(Json::Str).collect()))
        }
        "cm_expect" => cm_expect(&w, op),
        "cm_write" => {
            let mut o = raw_out();
            let ok = w.cm.borrow().write(&mut o);
            expect_bytes(op, ok, &o.0.into_bytes())
        }
        "expect_log" => expect_lines(op, "log", &w.log.borrow()),
        "clear_log" => {
            w.log.borrow_mut().clear();
            Ok(())
        }
        "mgr_new" => mgr_new(&w, op),
        "mgr_max_count" => mgr_max_count(&w, op),
        "mgr_driving_mode" => {
            let view = op.get("view").and_then(Json::as_i64).unwrap_or(0) as u32;
            check(op, Json::Bool(w.mgr.borrow().is_driving_mode(view)))
        }
        "automaton_controller" => automaton_controller(op),
        "city" => city(&w, op),
        "riot_add" => riot_add(&w, op),
        "riot_stage" => riot_stage(&w, op),
        "riot_end" => {
            let leader = op.get("leader").and_then(Json::as_i64).unwrap_or(0) as usize;
            let mut city = w.city.borrow_mut();
            check(op, Json::Bool(w.riot.borrow_mut().end(&mut *city, leader)))
        }
        "fire_target" => {
            let t = op.get("target").and_then(Json::as_i64).unwrap_or(0) as u64;
            let mut city = w.city.borrow_mut();
            check(op, Json::Bool(w.fire.borrow_mut().set_target(&mut *city, t)))
        }
        "fire_shutdown" => {
            let mut city = w.city.borrow_mut();
            check(op, Json::Bool(w.fire.borrow_mut().shutdown(&mut *city)))
        }
        "expect_calls" => expect_lines(op, "calls", &w.city.borrow().calls),
        "clear_calls" => {
            w.city.borrow_mut().calls.clear();
            Ok(())
        }
        _ => Err(format!("unknown op '{name}'")),
    })
}

/// Checks `expect` (when given) against `got`.
fn check(op: &Json, got: Json) -> R<()> {
    scalar::check_expect(op, got)
}

fn expect_lines(op: &Json, key: &str, got: &[String]) -> R<()> {
    let want: Vec<&str> = arr(field(op, key)?, key)?
        .iter()
        .map(|j| j.as_str().ok_or_else(|| format!("'{key}' entries must be strings")))
        .collect::<R<_>>()?;
    if got != want.as_slice() {
        return Err(format!("{key}: got {got:?}, expected {want:?}"));
    }
    Ok(())
}

fn expect_bytes(op: &Json, ok: bool, got: &[u8]) -> R<()> {
    if !ok {
        return Err("Write returned false".into());
    }
    if let Some(h) = opt(op, "hex") {
        let want = hex_decode(str_of(h, "hex")?)?;
        if want != got {
            return Err(format!("wrote {}, expected {}", hex_encode(got), hex_encode(&want)));
        }
    }
    Ok(())
}

// ---- records ------------------------------------------------------------------------------------------------

enum Obj {
    Script(ScriptSystem),
    Cm(ControllerManager),
    Mgr(AutomataManager),
    Saved(Rc<dyn SaveObject>),
}

fn object_for(class: u32) -> R<Obj> {
    if class == CLSID_SCRIPT_SYSTEM {
        return Ok(Obj::Script(ScriptSystem::new()));
    }
    if class == CLSID_CONTROLLER_MANAGER {
        return Ok(Obj::Cm(ControllerManager::new(Box::new(LogHost(Rc::default())))));
    }
    if let Some(k) = ManagerKind::from_class(class) {
        return Ok(Obj::Mgr(AutomataManager::new(k)));
    }
    if let Some(c) = Controller::for_class(class) {
        return Ok(Obj::Saved(c));
    }
    Automaton::for_class(class)
        .map(|a| Obj::Saved(a as Rc<dyn SaveObject>))
        .ok_or_else(|| format!("no record for class {class:#010x}"))
}

fn jref(r: &RefValue) -> Json {
    match r {
        RefValue::Null => Json::Null,
        RefValue::Host { token, class_id } => Json::Array(vec![
            Json::Int(i64::from(*token as u32)),
            Json::Int(i64::from(*class_id)),
        ]),
        RefValue::Object(o) => Json::Array(vec![
            Json::Int(i64::from(o.host_token() as u32)),
            Json::Int(i64::from(o.class_id())),
        ]),
    }
}

fn jf(v: f32) -> Json {
    Json::Float(f64::from(v))
}

fn jtime(t: &ControllerTime) -> Json {
    Json::Array(vec![jf(t.hour), Json::Int(i64::from(t.day))])
}

fn jobj(pairs: Vec<(&str, Json)>) -> Json {
    Json::Object(pairs.into_iter().map(|(k, v)| (k.to_owned(), v)).collect())
}

/// The decoded fields a vector may check (README "Automata ops").
fn fields(obj: &Obj) -> Json {
    match obj {
        Obj::Script(_) => jobj(vec![("version", Json::Int(1))]),
        Obj::Cm(c) => {
            let r = c.record();
            jobj(vec![
                ("version", Json::Int(i64::from(r.version))),
                ("clock", jref(&r.clock)),
                ("time_a", jtime(&r.time_a)),
                ("time_b", jtime(&r.time_b)),
                ("entries", Json::Int(r.entries.len() as i64)),
                ("sets", Json::Int(r.sets.len() as i64)),
                ("loaded", Json::Bool(c.is_loaded())),
            ])
        }
        Obj::Mgr(m) => {
            let r = m.record();
            let mut v = vec![
                ("version", Json::Int(i64::from(r.version))),
                ("base_version", Json::Int(i64::from(r.base.version))),
                ("max_count", Json::Int(i64::from(r.base.max_count))),
                ("active", Json::Bool(r.base.active)),
                ("time_scaling", Json::Bool(r.base.time_scaling)),
                ("automata", Json::Int(r.base.automata.len() as i64)),
                (
                    "group_counts",
                    Json::Array(
                        r.base
                            .group_counts
                            .iter()
                            .map(|&(g, n)| Json::Array(vec![Json::Int(i64::from(g)), Json::Int(i64::from(n))]))
                            .collect(),
                    ),
                ),
                ("old_version", Json::Bool(m.is_old_version())),
            ];
            match &r.part {
                ManagerPart::Traffic(t) => {
                    v[0] = ("version", Json::Int(i64::from(t.version)));
                    v.push(("tag", Json::Int(i64::from(t.tag))));
                    v.push(("cursor", Json::Int(i64::from(t.cursor))));
                    v.push(("incidents", Json::Int(t.incidents.len() as i64)));
                }
                ManagerPart::Aircraft { pairs, spots } => {
                    v.push(("pairs", Json::Int(pairs.len() as i64)));
                    v.push(("spots", Json::Int(spots.len() as i64)));
                }
                ManagerPart::Watercraft {
                    lots,
                    occupants,
                    ferries,
                } => {
                    v.push(("lots", Json::Int(lots.len() as i64)));
                    v.push(("occupants", Json::Int(occupants.len() as i64)));
                    v.push(("ferries", Json::Int(ferries.len() as i64)));
                }
            }
            jobj(v)
        }
        Obj::Saved(o) => {
            let any = o.clone().as_any_rc();
            if let Ok(c) = any.clone().downcast::<Controller>() {
                let r = c.record();
                let mut v = vec![
                    ("base_version", Json::Int(i64::from(r.base.version))),
                    ("occupant", jref(&r.base.occupant)),
                    ("template", Json::Str(c.template_name())),
                    (
                        "position",
                        Json::Array(r.base.position.iter().map(|x| jf(*x)).collect()),
                    ),
                    ("flags", Json::Int(i64::from(r.base.flags))),
                    ("automata", Json::Int(r.base.automata.len() as i64)),
                ];
                match &r.kind {
                    ControllerKind::Generator {
                        period,
                        second_period,
                        day_stamp,
                        counter,
                    } => {
                        v.push(("period", jtime(period)));
                        v.push(("second_period", jtime(second_period)));
                        v.push(("day_stamp", Json::Int(i64::from(*day_stamp))));
                        v.push(("counter", Json::Int(i64::from(*counter))));
                    }
                    ControllerKind::Attractor { tag, curve } => {
                        v.push(("tag", Json::Int(i64::from(*tag))));
                        v.push((
                            "curve",
                            Json::Array(curve.iter().map(|&(x, y)| Json::Array(vec![jf(x), jf(y)])).collect()),
                        ));
                    }
                }
                return jobj(v);
            }
            if let Ok(a) = any.downcast::<Automaton>() {
                let r = a.record();
                let b = r.base();
                return jobj(vec![
                    ("class", Json::Int(i64::from(r.class_id()))),
                    ("base_version", Json::Int(i64::from(b.version))),
                    ("occupant_type", Json::Int(i64::from(b.occupant.occupant_type))),
                    ("controller", jref(&b.controller)),
                    (
                        "states",
                        Json::Array(b.states.iter().map(|s| Json::Int(i64::from(s.state_type))).collect()),
                    ),
                ]);
            }
            Json::Null
        }
    }
}

/// `automata_record`: read a record of `class` into a fresh object, check
/// the result, the bytes left, the fields; write it back and compare.
fn record(op: &Json) -> R<()> {
    let class = u32_of(field(op, "class")?, "class")?;
    let bytes = hex_decode(str_of(field(op, "hex")?, "hex")?)?;
    let mut obj = object_for(class)?;
    let mut resolver = Resolver { controllers: None };
    let mut r = ByteReader::with_resolver(&bytes, &mut resolver);
    let ok = match &mut obj {
        Obj::Script(s) => s.read(&mut r),
        Obj::Cm(c) => c.read(&mut r),
        Obj::Mgr(m) => m.read(&mut r),
        Obj::Saved(o) => o.read(&mut r),
    };
    let left = r.remaining();
    let want_ok = opt(op, "expect").and_then(boolean).unwrap_or(true);
    if ok != want_ok {
        return Err(format!("Read returned {ok}, expected {want_ok} ({left} bytes left)"));
    }
    if !ok {
        return Ok(());
    }
    let want_left = opt(op, "left").map(|j| i64_of(j, "left")).transpose()?.unwrap_or(0);
    if left as i64 != want_left {
        return Err(format!("{left} bytes left after Read, expected {want_left}"));
    }
    if let Some(want) = opt(op, "fields") {
        let got = fields(&obj);
        if !matches(&got, want, true) {
            return Err(format!("fields {}, expected {}", show(&got), show(want)));
        }
    }
    let mut o = raw_out();
    let wrote = match &obj {
        Obj::Script(s) => s.write(&mut o),
        Obj::Cm(c) => c.write(&mut o),
        Obj::Mgr(m) => m.write(&mut o),
        Obj::Saved(s) => s.write(&mut o),
    };
    let out = o.0.into_bytes();
    match opt(op, "written") {
        Some(h) => expect_bytes(&jobj(vec![("hex", h.clone())]), wrote, &out),
        None => expect_bytes(&jobj(vec![]), wrote, &out).and_then(|_| {
            if out == bytes[..bytes.len() - want_left as usize] {
                Ok(())
            } else {
                Err(format!(
                    "the record does not re-encode: wrote {}, read {}",
                    hex_encode(&out),
                    hex_encode(&bytes)
                ))
            }
        }),
    }
}

// ---- the controller manager ---------------------------------------------------------------------------------

fn kind_of(j: Option<&Json>) -> R<TemplateKind> {
    match j.and_then(Json::as_str).unwrap_or("generator") {
        "generator" => Ok(TemplateKind::Generator),
        "attractor" => Ok(TemplateKind::Attractor),
        other => Err(format!("bad template kind '{other}'")),
    }
}

/// `automata_scripts`: replaces the script set.
fn scripts(w: &World, op: &Json) -> R<()> {
    let mut s = ScriptSystem::new();
    s.init();
    if let Some(ts) = opt(op, "templates") {
        for t in arr(ts, "templates")? {
            let name = str_of(field(t, "name")?, "name")?;
            let mut tpl = ControllerTemplate::new(name, kind_of(t.get("kind"))?);
            if let Some(f) = t.get("flags") {
                tpl.flags = u32_of(f, "flags")?;
            }
            s.define_template(tpl);
        }
    }
    if let Some(gs) = opt(op, "groups") {
        for g in arr(gs, "groups")? {
            s.define_occupant_group(OccupantGroup {
                id: u32_of(field(g, "id")?, "id")?,
                name: str_of(field(g, "name")?, "name")?.to_owned(),
                templates: arr(field(g, "templates")?, "templates")?
                    .iter()
                    .map(|t| str_of(t, "templates").map(str::to_owned))
                    .collect::<R<_>>()?,
            });
        }
    }
    *w.scripts.borrow_mut() = s;
    Ok(())
}

fn occupant_of(j: &Json) -> R<RefValue> {
    let id = u32_of(j, "occupant")?;
    Ok(RefValue::Host {
        token: (u64::from(OCCUPANT_CLASS) << 32) | u64::from(id),
        class_id: OCCUPANT_CLASS,
    })
}

fn occupant(op: &Json) -> R<RefValue> {
    occupant_of(field(op, "occupant")?)
}

/// Gives the controllers the manager made a host token each (written as
/// their object ID).
fn number_controllers(w: &World) {
    for c in w.cm.borrow().all_controllers() {
        if let RefValue::Object(o) = &c {
            if let Ok(ctl) = o.clone().as_any_rc().downcast::<Controller>() {
                if ctl.host_token() == 0 {
                    let t = w.next_token.get();
                    w.next_token.set(t + 1);
                    ctl.set_host_token(u64::from(t));
                }
            }
        }
    }
}

/// `automata_cm_read`: reads a controller manager record; controller
/// references become core controllers, whose template names `templates`
/// gives by object ID (their own records live in their class records).
fn cm_read(w: &World, op: &Json) -> R<()> {
    let bytes = hex_decode(str_of(field(op, "hex")?, "hex")?)?;
    let mut resolver = Resolver {
        controllers: Some(&w.controllers),
    };
    let mut r = ByteReader::with_resolver(&bytes, &mut resolver);
    let ok = w.cm.borrow_mut().read(&mut r);
    if let Some(t) = opt(op, "templates") {
        for (k, v) in t.as_object().ok_or("'templates' must be an object")? {
            let id = Json::Str(k.clone()).as_i64().ok_or("bad controller ID")? as u32;
            let name = str_of(v, "template")?;
            let map = w.controllers.borrow();
            let c = map.get(&id).ok_or_else(|| format!("no controller {id:#x} was read"))?;
            c.with_record(|r| r.base.template_name = name.as_bytes().to_vec());
        }
    }
    check(op, Json::Bool(ok))
}

fn cm_add(w: &World, op: &Json) -> R<()> {
    let occ = occupant(op)?;
    let name = str_of(field(op, "template")?, "template")?;
    let made = {
        let scripts = w.scripts.borrow();
        w.cm.borrow_mut()
            .add_controller_from_template_name(&*scripts, &occ, name, 0)
    };
    number_controllers(w);
    check(op, Json::Bool(made.is_some()))
}

fn cm_add_group(w: &World, op: &Json) -> R<()> {
    let occ = occupant(op)?;
    let use_scripts = opt(op, "no_scripts").and_then(boolean) != Some(true);
    let scripts = w.scripts.borrow();
    let s: Option<&dyn fc4_core::automata::ScriptSource> = if use_scripts { Some(&*scripts) } else { None };
    let ok = if let Some(id) = opt(op, "id") {
        w.cm.borrow_mut()
            .add_controllers_from_occupant_group_id(s, u32_of(id, "id")?, &occ, 0)
    } else {
        let name = str_of(field(op, "name")?, "name")?;
        w.cm.borrow_mut()
            .add_controllers_from_occupant_group_name(s, name, &occ, 0)
    };
    drop(scripts);
    number_controllers(w);
    check(op, Json::Bool(ok))
}

fn cm_remove_group(w: &World, op: &Json) -> R<()> {
    let occ = occupant(op)?;
    let scripts = w.scripts.borrow();
    let s: Option<&dyn fc4_core::automata::ScriptSource> = Some(&*scripts);
    let ok = if let Some(id) = opt(op, "id") {
        w.cm.borrow_mut()
            .remove_controllers_from_occupant_group_id(s, u32_of(id, "id")?, &occ)
    } else {
        let name = str_of(field(op, "name")?, "name")?;
        w.cm.borrow_mut()
            .remove_controllers_from_occupant_group_name(s, name, &occ)
    };
    check(op, Json::Bool(ok))
}

/// `automata_cm_expect`: `occupants` maps occupant numbers to their
/// controllers' template names (all listed occupants, in order); `loaded`;
/// `flags` maps template names to flags.
fn cm_expect(w: &World, op: &Json) -> R<()> {
    let cm = w.cm.borrow();
    if let Some(want) = opt(op, "occupants") {
        let mut got = Vec::new();
        for e in &cm.record().entries {
            let id = match &e.occupant {
                RefValue::Host { token, .. } => *token as u32,
                _ => 0,
            };
            let names: Vec<Json> = e
                .controllers
                .iter()
                .map(|c| match c {
                    RefValue::Object(o) => o
                        .clone()
                        .as_any_rc()
                        .downcast::<Controller>()
                        .map(|c| Json::Str(c.template_name()))
                        .unwrap_or(Json::Null),
                    _ => Json::Null,
                })
                .collect();
            got.push((id.to_string(), Json::Array(names)));
        }
        let got = Json::Object(got);
        if !matches(&got, want, false) {
            return Err(format!("occupants {}, expected {}", show(&got), show(want)));
        }
    }
    if let Some(l) = opt(op, "loaded").and_then(boolean) {
        if cm.is_loaded() != l {
            return Err(format!("loaded {}, expected {l}", cm.is_loaded()));
        }
    }
    if let Some(want) = opt(op, "bound") {
        let got: Vec<Json> = cm
            .all_controllers()
            .iter()
            .map(|c| match c {
                RefValue::Object(o) => o
                    .clone()
                    .as_any_rc()
                    .downcast::<Controller>()
                    .map(|c| Json::Bool(c.template_bound() && c.is_initialized() && c.in_grid()))
                    .unwrap_or(Json::Null),
                _ => Json::Null,
            })
            .collect();
        let got = Json::Array(got);
        if !matches(&got, want, false) {
            return Err(format!("bound {}, expected {}", show(&got), show(want)));
        }
    }
    if let Some(want) = opt(op, "shut_down") {
        let got: Vec<Json> = w
            .controllers
            .borrow()
            .iter()
            .filter(|(_, c)| c.is_shut_down())
            .map(|(id, _)| Json::Int(i64::from(*id)))
            .collect();
        let got = Json::Array(got);
        if !matches(&got, want, false) {
            return Err(format!("shut down {}, expected {}", show(&got), show(want)));
        }
    }
    Ok(())
}

// ---- managers and automata ----------------------------------------------------------------------------------

fn manager_kind(j: &Json) -> R<ManagerKind> {
    Ok(match str_of(j, "kind")? {
        "vehicle" => ManagerKind::Vehicle,
        "pedestrian" => ManagerKind::Pedestrian,
        "aircraft" => ManagerKind::Aircraft,
        "watercraft" => ManagerKind::Watercraft,
        other => return Err(format!("bad manager kind '{other}'")),
    })
}

fn mgr_new(w: &World, op: &Json) -> R<()> {
    let mut m = AutomataManager::new(manager_kind(field(op, "kind")?)?);
    if let Some(n) = opt(op, "automata") {
        for i in 0..i64_of(n, "automata")? {
            let a = Automaton::for_class(CLSID_PEDESTRIAN).expect("pedestrian class");
            a.set_host_token(i as u64 + 1);
            m.add_automaton(RefValue::Object(a));
        }
    }
    *w.mgr.borrow_mut() = m;
    Ok(())
}

/// `automata_mgr_max_count`: `SetMaxCount(n)`; checks `max`, `hard_limit`,
/// `removed`, `count`.
fn mgr_max_count(w: &World, op: &Json) -> R<()> {
    let n = i64_of(field(op, "n")?, "n")? as i32;
    let mut m = w.mgr.borrow_mut();
    let removed = m.set_max_count(n);
    let got = jobj(vec![
        ("max", Json::Int(i64::from(m.max_count()))),
        ("hard_limit", Json::Int(i64::from(m.hard_limit()))),
        ("removed", Json::Int(removed.len() as i64)),
        ("count", Json::Int(m.automata_count() as i64)),
    ]);
    if let Some(want) = opt(op, "state") {
        if !matches(&got, want, true) {
            return Err(format!("{}, expected {}", show(&got), show(want)));
        }
    }
    Ok(())
}

/// `automata_automaton_controller`: a pedestrian whose controller has
/// `flags` is written; `null` says whether its record equals the one with
/// a null controller (flag 8, §6.1 field 5).
fn automaton_controller(op: &Json) -> R<()> {
    let flags = u32_of(field(op, "flags")?, "flags")?;
    let c = Controller::for_class(fc4_core::automata::CLSID_GENERATOR).expect("generator");
    c.set_flags(flags);
    c.set_host_token(0x77);
    let a = Automaton::for_class(CLSID_PEDESTRIAN).expect("pedestrian");
    a.set_controller(RefValue::Object(c));
    let mut with = raw_out();
    a.write(&mut with);
    a.set_controller(RefValue::Null);
    let mut without = raw_out();
    a.write(&mut without);
    let null = with.0.bytes() == without.0.bytes();
    let want = boolean(field(op, "null")?).ok_or("'null' must be a boolean")?;
    if null != want {
        return Err(format!("controller written as null: {null}, expected {want}"));
    }
    Ok(())
}

// ---- the hooks ----------------------------------------------------------------------------------------------

/// `automata_city`: the scripted city's answers.
fn city(w: &World, op: &Json) -> R<()> {
    let mut c = w.city.borrow_mut();
    if let Some(v) = opt(op, "road").and_then(boolean) {
        c.no_road = !v;
    }
    if let Some(v) = opt(op, "cap").and_then(boolean) {
        c.cap = v;
    }
    if let Some(v) = opt(op, "create_null").and_then(boolean) {
        c.create_null = v;
    }
    if let Some(b) = opt(op, "buildings") {
        c.buildings = arr(b, "buildings")?
            .iter()
            .map(|j| i64_of(j, "buildings").map(|v| v as u64))
            .collect::<R<_>>()?;
    }
    Ok(())
}

fn riot_add(w: &World, op: &Json) -> R<()> {
    let x = i64_of(field(op, "x")?, "x")? as i32;
    let z = i64_of(field(op, "z")?, "z")? as i32;
    let mut city = w.city.borrow_mut();
    let got = w.riot.borrow_mut().add(&mut *city, x, z);
    check(op, got.map(|i| Json::Int(i as i64)).unwrap_or(Json::Null))
}

fn riot_stage(w: &World, op: &Json) -> R<()> {
    let leader = op.get("leader").and_then(Json::as_i64).unwrap_or(0) as usize;
    let stage = i64_of(field(op, "stage")?, "stage")? as i32;
    let mut city = w.city.borrow_mut();
    check(op, Json::Bool(w.riot.borrow_mut().set_stage(&mut *city, leader, stage)))
}
