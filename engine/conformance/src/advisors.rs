// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/advisors.md`: the scripted advisor host
//! of the core (`fc4_core::advisors::scripted::ScriptedHost`, Lua replaced by
//! literal trigger texts) and ops that drive the advisor system, its
//! advisors, advices and subjects. The MySim ops (`mysims.rs`) share this
//! world. See README.md ("Advisor ops").

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use fc4_core::advisors::scripted::{HostCall, Occupant, ScriptedHost};
use fc4_core::advisors::Advice;
use fc4_core::advisors::{
    batch_size, clsid, AdviceEntry, AdviceSubject, Advisor, AdvisorEntry, AdvisorMessage, AdvisorSystem,
    ConditionalStatus, Item, ListKind, ListSrc, LuaResult, LuaValue, Src, SrcKind, SubjectInit, SubjectState, Val,
};
use fc4_core::julian::julian_from_civil;
use fc4_core::mysims::scripted::{MsCall, ScriptedMySimHost};
use fc4_core::mysims::MySimAgent;
use fc4_core::stream::{ByteReader, ByteWriter, RefResolver, RefValue, SaveObject, SaveRegistry};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arr, bool_of, check_expect, check_state, f32_of, i64_of, jf, ji, opt, str_of, u32_of, EventLog, Saved, R,
};

/// The MySim part of the world (`mysim_create`).
pub(crate) struct MsPart {
    pub(crate) h: Rc<ScriptedMySimHost>,
    pub(crate) agent: Rc<MySimAgent>,
}

/// A core object a vector names.
#[derive(Clone)]
pub(crate) enum Obj {
    System(Rc<AdvisorSystem>),
    Advisor(Rc<Advisor>),
    Advice(Rc<Advice>),
    Subject(Rc<AdviceSubject>),
}

pub(crate) struct World {
    pub(crate) h: Rc<ScriptedHost>,
    pub(crate) sys: RefCell<Option<Rc<AdvisorSystem>>>,
    pub(crate) ms: RefCell<Option<MsPart>>,
    pub(crate) seed: Cell<u32>,
    pub(crate) named: RefCell<BTreeMap<String, Obj>>,
    pub(crate) records: RefCell<BTreeMap<String, Vec<u8>>>,
    saves: RefCell<BTreeMap<String, Saved>>,
    pub(crate) events: EventLog,
}

impl World {
    fn new() -> Rc<World> {
        Rc::new(World {
            h: ScriptedHost::new(),
            sys: RefCell::new(None),
            ms: RefCell::new(None),
            seed: Cell::new(0),
            named: RefCell::new(BTreeMap::new()),
            records: RefCell::new(BTreeMap::new()),
            saves: RefCell::new(BTreeMap::new()),
            events: EventLog::default(),
        })
    }

    pub(crate) fn sys(&self) -> R<Rc<AdvisorSystem>> {
        self.sys
            .borrow()
            .clone()
            .ok_or_else(|| "no advisor system (adv_create)".into())
    }

    /// A new system over the world's host (the previous one is dropped).
    pub(crate) fn new_sys(&self) -> Rc<AdvisorSystem> {
        let s = AdvisorSystem::new(self.h.clone(), self.seed.get());
        *self.sys.borrow_mut() = Some(s.clone());
        s
    }

    /// Moves the hosts' logs into the event log.
    pub(crate) fn sync(&self) {
        for c in self.h.take_log() {
            let (k, f) = host_call(&c);
            self.events.push(k, f);
        }
        if let Some(ms) = &*self.ms.borrow() {
            for c in ms.h.take_log() {
                let (k, f) = ms_call(&c);
                self.events.push(k, f);
            }
        }
    }

    pub(crate) fn advisor(&self, id: i32) -> R<Rc<Advisor>> {
        self.sys()?.get_advisor(id).ok_or_else(|| format!("no advisor {id}"))
    }
}

pub(crate) fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "advisors", World::new)
}

// ---- JSON forms -------------------------------------------------------------------------------------

pub(crate) fn jstr(b: &[u8]) -> Json {
    Json::Str(String::from_utf8_lossy(b).into_owned())
}

fn jnum(v: f64) -> Json {
    if v.is_nan() {
        Json::Str("nan".into())
    } else {
        Json::Float(v)
    }
}

pub(crate) fn subject_json(s: &SubjectState) -> Json {
    Json::Object(vec![
        (
            "location".into(),
            s.location.map_or(Json::Null, |(x, z)| Json::Array(vec![jf(x), jf(z)])),
        ),
        ("object".into(), s.object.map_or(Json::Null, |o| ji(o as i64))),
        ("data1".into(), ji(s.data1)),
        ("data2".into(), ji(s.data2)),
    ])
}

fn obj_text(s: &str) -> Json {
    Json::Str(s.to_owned())
}

/// One advisor-host call as an event.
fn host_call(c: &HostCall) -> (&'static str, Vec<(&'static str, Json)>) {
    match c {
        HostCall::Post { msg, data1, obj } => (
            "post",
            vec![("type", ji(*msg)), ("data1", ji(*data1)), ("object", obj_text(obj))],
        ),
        HostCall::Send { msg, data1, obj } => (
            "send",
            vec![("type", ji(*msg)), ("data1", ji(*data1)), ("object", obj_text(obj))],
        ),
        HostCall::UpdateSelf { advisor, data1, data2 } => (
            "update_self",
            vec![("advisor", ji(*advisor)), ("data1", ji(*data1)), ("data2", ji(*data2))],
        ),
        HostCall::Subscribe { target, msg } => ("subscribe", vec![("target", obj_text(target)), ("type", ji(*msg))]),
        HostCall::Unsubscribe { target, msg } => {
            ("unsubscribe", vec![("target", obj_text(target)), ("type", ji(*msg))])
        }
        HostCall::Global { name, value } => ("global", vec![("name", obj_text(name)), ("value", jnum(*value))]),
        HostCall::GlobalString { name, value } => {
            ("global_string", vec![("name", obj_text(name)), ("value", jstr(value))])
        }
        HostCall::Local { name, value, subject } => (
            "local",
            vec![
                ("name", obj_text(name)),
                ("value", jnum(*value)),
                ("subject", subject.as_ref().map_or(Json::Null, subject_json)),
            ],
        ),
        HostCall::Effect { id, assert } => ("effect", vec![("id", ji(*id)), ("assert", Json::Bool(*assert))]),
        HostCall::Rebuild { guid } => ("rebuild", vec![("guid", ji(*guid))]),
        HostCall::History { history_type, value } => {
            ("history", vec![("type", ji(*history_type)), ("value", ji(*value))])
        }
        HostCall::Status { guid, status } => (
            "status",
            vec![
                ("guid", ji(*guid)),
                ("status", ji(status.status)),
                ("count", ji(status.building_count)),
                ("tooltip", jstr(&status.tooltip)),
            ],
        ),
        HostCall::Cheat { id, on } => ("cheat", vec![("id", ji(*id)), ("on", Json::Bool(*on))]),
        HostCall::Window { start } => ("window", vec![("start", Json::Bool(*start))]),
        HostCall::Ticker => ("ticker", vec![]),
        HostCall::CollectGarbage => ("collect_garbage", vec![]),
        HostCall::Lua(s) => ("lua", vec![("call", obj_text(s))]),
        HostCall::CancelDeal { object } => ("cancel_deal", vec![("object", ji(*object as i64))]),
        HostCall::SetOrdinance { id, on } => ("set_ordinance", vec![("id", ji(*id)), ("on", Json::Bool(*on))]),
    }
}

/// One MySim-host call as an event.
fn ms_call(c: &MsCall) -> (&'static str, Vec<(&'static str, Json)>) {
    match c {
        MsCall::CreateTable(s) => ("create_table", vec![("sim", ji(*s))]),
        MsCall::Activate(s) => ("activate", vec![("sim", ji(*s))]),
        MsCall::AgentLua(on) => ("agent_lua", vec![("on", Json::Bool(*on))]),
        MsCall::Post(p) => (
            "mysim_post",
            vec![
                ("type", ji(p.msg_type)),
                ("data1", ji(p.data1)),
                ("data2", ji(p.data2)),
                ("void1", p.void1.map_or(Json::Null, |v| ji(v as i64))),
                ("sim", p.sim.map_or(Json::Null, ji)),
            ],
        ),
        MsCall::Dispatch(s, what) => ("dispatch", vec![("sim", ji(*s)), ("what", obj_text(what))]),
        MsCall::Trigger(name, p) => (
            "trigger",
            vec![
                ("name", obj_text(name)),
                ("position", Json::Array(p.iter().map(|v| jf(*v)).collect())),
            ],
        ),
        MsCall::Subscribe(m, on) => ("mysim_subscribe", vec![("type", ji(*m)), ("on", Json::Bool(*on))]),
    }
}

// ---- parsing helpers ------------------------------------------------------------------------------

fn bytes_of(j: &Json, what: &str) -> R<Vec<u8>> {
    Ok(str_of(j, what)?.as_bytes().to_vec())
}

fn opt_u32(op: &Json, k: &str, d: u32) -> R<u32> {
    opt(op, k).map_or(Ok(d), |v| u32_of(v, k))
}

fn opt_i32(op: &Json, k: &str, d: i32) -> R<i32> {
    opt(op, k).map_or(Ok(d), |v| i64_of(v, k).map(|x| x as i32))
}

fn opt_bool(op: &Json, k: &str, d: bool) -> R<bool> {
    opt(op, k).map_or(Ok(d), |v| bool_of(v, k))
}

fn key_of(k: &str) -> R<i64> {
    Json::Str(k.to_owned()).as_i64().ok_or_else(|| format!("bad key '{k}'"))
}

fn pair_f32(j: &Json, what: &str) -> R<(f32, f32)> {
    let a = arr(j, what)?;
    if a.len() != 2 {
        return Err(format!("'{what}' must be [x, z]"));
    }
    Ok((f32_of(&a[0], what)?, f32_of(&a[1], what)?))
}

fn f64_of(j: &Json, what: &str) -> R<f64> {
    if j.as_str().is_some_and(|s| s.eq_ignore_ascii_case("nan")) {
        return Ok(f64::NAN);
    }
    scalar::num(j).ok_or_else(|| format!("'{what}' must be a number"))
}

/// A Lua call result: `true` / `false`, a number (`"nan"`), `"nil"`,
/// `"error"`, or `{"str": text}`.
fn lua_result_of(j: &Json) -> R<LuaResult> {
    Ok(match j {
        Json::Bool(b) => LuaResult::Bool(*b),
        Json::Null => LuaResult::Other,
        Json::Str(s) if s == "nil" => LuaResult::Other,
        Json::Str(s) if s == "error" => LuaResult::Error,
        Json::Object(_) => LuaResult::Str(bytes_of(field(j, "str")?, "str")?),
        _ => LuaResult::Number(f64_of(j, "result")?),
    })
}

fn lua_value_json(v: &LuaValue) -> Json {
    match v {
        LuaValue::Nil => Json::Null,
        LuaValue::Bool(b) => Json::Bool(*b),
        LuaValue::Number(n) => jnum(*n),
        LuaValue::Str(s) => Json::Object(vec![("str".into(), jstr(s))]),
        LuaValue::Subject(s) => Json::Object(vec![(
            "subject".into(),
            s.as_ref().map_or(Json::Null, |s| subject_json(&s.state())),
        )]),
    }
}

/// A message type: a name or an ID.
pub(crate) fn msg_type_of(j: &Json) -> R<u32> {
    use fc4_core::advisors::msg;
    Ok(match j.as_str() {
        Some("PostCityInit") => msg::POST_CITY_INIT,
        Some("CityEstablished") => msg::CITY_ESTABLISHED,
        Some("Ticker") => msg::TICKER,
        Some("Cheat") => msg::CHEAT,
        Some("InsertOccupant") | Some("OccupantInserted") => msg::INSERT_OCCUPANT,
        Some("RemoveOccupant") | Some("OccupantRemoved") => msg::REMOVE_OCCUPANT,
        Some("PostCityInitComplete") => msg::POST_CITY_INIT_COMPLETE,
        Some("OrdinanceUpdated") => msg::ORDINANCE_UPDATED,
        Some("UpdateSelf") => msg::UPDATE_SELF,
        Some("DealStatus") => msg::DEAL_STATUS,
        Some("AdviceCommand") => msg::ADVICE_COMMAND,
        Some("DisasterStarted") => msg::DISASTER_STARTED,
        _ => u32_of(j, "type")?,
    })
}

/// An `AdvisorMessage` from `type`, `data1`–`data4`, `void1`, `unknown`.
pub(crate) fn message_of(op: &Json) -> R<AdvisorMessage> {
    let tok = |k: &str| -> R<Option<u64>> { opt(op, k).map(|v| i64_of(v, k).map(|x| x as u64)).transpose() };
    Ok(AdvisorMessage {
        msg_type: msg_type_of(field(op, "type")?)?,
        data1: opt_u32(op, "data1", 0)?,
        data2: opt_u32(op, "data2", 0)?,
        data3: opt_u32(op, "data3", 0)?,
        data4: opt_u32(op, "data4", 0)?,
        void1: tok("void1")?,
        unknown: tok("unknown")?,
    })
}

/// A class: a name or an ID.
pub(crate) fn class_of(j: &Json) -> R<u32> {
    Ok(match j.as_str() {
        Some("Advisor") => clsid::ADVISOR,
        Some("AdvisorBase") => clsid::ADVISOR_BASE,
        Some("CityPlanning") => clsid::CITY_PLANNING,
        Some("CitySituations") => clsid::CITY_SITUATIONS,
        Some("Finance") => clsid::FINANCE,
        Some("HQ") => clsid::HQ,
        Some("Safety") => clsid::SAFETY,
        Some("Traffic") => clsid::TRAFFIC,
        Some("Utility") => clsid::UTILITY,
        Some("Advice") => clsid::ADVICE,
        Some("RewardAdvice") => clsid::REWARD_ADVICE,
        Some("NeighborDealAdvice") => clsid::NEIGHBOR_DEAL_ADVICE,
        Some("Subject") => clsid::SUBJECT,
        Some("MySim") => clsid::MYSIM,
        Some("MyStreetSim") => clsid::MY_STREET_SIM,
        _ => u32_of(j, "class")?,
    })
}

/// An object spec: `"system"`, `{"advisor": id}`, `{"advice": guid,
/// "advisor"?: id}`, `{"mysim": index}`, or a name (`{"named": n}` or the
/// bare string).
pub(crate) fn obj_of(w: &World, j: &Json) -> R<Obj> {
    if j.as_str() == Some("system") {
        return Ok(Obj::System(w.sys()?));
    }
    if let Some(n) = j.as_str() {
        return w
            .named
            .borrow()
            .get(n)
            .cloned()
            .ok_or_else(|| format!("no object named '{n}'"));
    }
    if let Some(n) = j.get("named") {
        return obj_of(w, n);
    }
    if let Some(i) = j.get("mysim") {
        let i = i64_of(i, "mysim")? as usize;
        let ms = w.ms.borrow();
        let ms = ms.as_ref().ok_or("no MySims (mysim_create)")?;
        return ms
            .agent
            .sim(i)
            .map(Obj::Advisor)
            .ok_or_else(|| format!("no MySim at index {i}"));
    }
    if let Some(g) = j.get("advice") {
        let guid = u32_of(g, "advice")?;
        let advisors: Vec<Rc<Advisor>> = match j.get("advisor") {
            Some(a) => vec![w.advisor(i64_of(a, "advisor")? as i32)?],
            None => w.sys()?.advisors().into_iter().map(|(_, a)| a).collect(),
        };
        return advisors
            .iter()
            .find_map(|a| a.advice_by_guid(guid))
            .map(Obj::Advice)
            .ok_or_else(|| format!("no advice 0x{guid:X}"));
    }
    if let Some(a) = j.get("advisor") {
        return Ok(Obj::Advisor(w.advisor(i64_of(a, "advisor")? as i32)?));
    }
    Err(format!("bad object spec {}", scalar::show(j)))
}

fn advisor_of(w: &World, j: &Json) -> R<Rc<Advisor>> {
    match obj_of(w, j)? {
        Obj::Advisor(a) => Ok(a),
        _ => Err("not an advisor".into()),
    }
}

fn advice_of(w: &World, j: &Json) -> R<Rc<Advice>> {
    match obj_of(w, j)? {
        Obj::Advice(a) => Ok(a),
        _ => Err("not an advice".into()),
    }
}

fn subject_of(w: &World, j: &Json) -> R<Rc<AdviceSubject>> {
    match obj_of(w, j)? {
        Obj::Subject(s) => Ok(s),
        _ => Err("not a subject".into()),
    }
}

fn save_obj(o: &Obj) -> Rc<dyn SaveObject> {
    match o {
        Obj::System(s) => s.clone(),
        Obj::Advisor(a) => a.clone(),
        Obj::Advice(a) => a.clone(),
        Obj::Subject(s) => s.clone(),
    }
}

// ---- states ------------------------------------------------------------------------------------------

fn guids(v: &[Rc<Advice>]) -> Json {
    Json::Array(v.iter().map(|a| ji(a.guid())).collect())
}

fn system_state(s: &AdvisorSystem) -> Vec<(String, Json)> {
    let st = s.state();
    vec![
        (
            "advisors".into(),
            Json::Array(st.advisors.keys().map(|k| ji(*k)).collect()),
        ),
        ("initialized".into(), Json::Bool(st.initialized)),
        ("loaded_from_save".into(), Json::Bool(st.loaded_from_save)),
        ("reload_pending".into(), Json::Bool(st.reload_pending)),
        (
            "thresholds".into(),
            Json::Array(st.thresholds.iter().map(|v| ji(*v)).collect()),
        ),
        ("tags".into(), Json::Array(vec![ji(st.tags.0), ji(st.tags.1)])),
        ("last_day".into(), ji(st.last_day)),
        ("idle_start".into(), st.idle_start.map_or(Json::Null, ji)),
        (
            "context".into(),
            Json::Array(st.context.iter().map(|a| ji(a.guid())).collect()),
        ),
        ("dolly_llama".into(), Json::Bool(st.dolly_llama)),
        ("popup_threshold".into(), ji(s.popup_threshold())),
    ]
}

fn advisor_state(a: &Advisor) -> Vec<(String, Json)> {
    let s = a.state();
    let active: Vec<Rc<Advice>> = a.current_advices().into_iter().map(|(x, _)| x).collect();
    vec![
        ("id".into(), ji(s.id)),
        ("class".into(), ji(SaveObject::class_id(a))),
        ("kind".into(), Json::Str(format!("{:?}", a.kind()))),
        ("caption".into(), jstr(&s.caption)),
        ("name".into(), jstr(&s.name)),
        ("advice_type".into(), ji(s.advice_type)),
        ("table_index".into(), ji(s.table_index)),
        ("month_counter".into(), ji(s.month_counter)),
        ("current_day".into(), ji(s.current_day)),
        ("cursor".into(), ji(s.cursor)),
        ("advices".into(), guids(&a.advices())),
        ("active".into(), guids(&active)),
        (
            "events".into(),
            Json::Object(
                s.events
                    .iter()
                    .map(|(k, v)| {
                        (
                            format!("0x{k:X}"),
                            Json::Array(v.iter().map(|i| ji(*i as i64)).collect()),
                        )
                    })
                    .collect(),
            ),
        ),
        (
            "queue".into(),
            Json::Array(
                s.queue
                    .iter()
                    .map(|(e, sub)| {
                        Json::Array(vec![
                            ji(*e),
                            sub.as_ref().map_or(Json::Null, |x| subject_json(&x.state())),
                        ])
                    })
                    .collect(),
            ),
        ),
        ("dirty".into(), Json::Bool(s.dirty)),
        ("pending".into(), ji(s.pending)),
        ("base_subscribed".into(), Json::Bool(s.base_subscribed)),
        ("own_subscribed".into(), Json::Bool(s.own_subscribed)),
        ("debug_subscribed".into(), Json::Bool(s.debug_subscribed)),
        ("building_count".into(), ji(s.building_count)),
        ("park_count".into(), ji(s.park_count)),
        ("recreation_count".into(), ji(s.recreation_count)),
        (
            "landmarks".into(),
            Json::Object(s.landmarks.iter().map(|(k, v)| (format!("{k}"), ji(*v))).collect()),
        ),
        ("fire_active".into(), Json::Bool(s.fire_active)),
        ("riot_active".into(), Json::Bool(s.riot_active)),
        ("user_disaster".into(), Json::Bool(s.user_disaster)),
    ]
}

fn advice_state(w: &World, a: &Rc<Advice>) -> Vec<(String, Json)> {
    let s = a.state();
    let mut v = vec![
        ("guid".into(), ji(s.guid)),
        ("class".into(), ji(a.class_id())),
        ("kind".into(), Json::Str(format!("{:?}", a.kind()))),
        ("flags".into(), ji(s.flags)),
        ("active".into(), Json::Bool(a.is_active())),
        ("advisor".into(), ji(s.advisor_id)),
        ("table_index".into(), ji(s.table_index)),
        ("priority".into(), ji(s.priority)),
        ("frequency".into(), ji(s.frequency)),
        ("timeout".into(), ji(s.timeout)),
        ("mood".into(), ji(s.mood)),
        ("event".into(), ji(s.event_id)),
        ("command".into(), ji(s.command_id)),
        (
            "effects".into(),
            Json::Array(s.effects.iter().map(|e| ji(*e)).collect()),
        ),
        ("trigger_source".into(), jstr(&s.trigger_source)),
        (
            "has_trigger".into(),
            Json::Bool(s.trigger != fc4_core::advisors::NO_TRIGGER),
        ),
        ("loaded".into(), Json::Bool(s.loaded)),
        ("nd_type".into(), ji(s.nd_type)),
        (
            "subject".into(),
            s.subject.as_ref().map_or(Json::Null, |x| subject_json(&x.state())),
        ),
    ];
    // The info record of the advisor holding it.
    let info = w.sys().ok().and_then(|sys| {
        sys.advisors()
            .into_iter()
            .find_map(|(_, adv)| adv.state().infos.into_iter().find(|i| Rc::ptr_eq(&i.advice, a)))
    });
    if let Some(i) = info {
        v.push(("next_day".into(), ji(i.next_day)));
        v.push(("expiry".into(), ji(i.expiry)));
        v.push(("last_month".into(), ji(i.last_month)));
    }
    v
}

fn subject_state(s: &AdviceSubject) -> Vec<(String, Json)> {
    let st = s.state();
    match subject_json(&st) {
        Json::Object(kv) => {
            let mut kv = kv;
            kv.push(("has_location".into(), Json::Bool(s.has_location())));
            kv.push(("x".into(), jf(s.location_x())));
            kv.push(("z".into(), jf(s.location_z())));
            kv
        }
        _ => unreachable!(),
    }
}

pub(crate) fn obj_state(w: &World, o: &Obj) -> Vec<(String, Json)> {
    match o {
        Obj::System(s) => system_state(s),
        Obj::Advisor(a) => advisor_state(a),
        Obj::Advice(a) => advice_state(w, a),
        Obj::Subject(s) => subject_state(s),
    }
}

// ---- ops ------------------------------------------------------------------------------------------------

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("adv_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "advisor" => advisor_entry(&w, op),
        "advice" => advice_entry(&w, op),
        "trigger" => trigger(&w, op),
        "condition" => condition(&w, op),
        "value" => value(&w, op),
        "list" => list(&w, op),
        "occupant" => occupant(&w, op),
        "reward_subject" => reward_subject(&w, op),
        "status" => status(&w, op),
        "deal" => deal(&w, op),
        "history" => history(&w, op),
        "create" => {
            if let Some(v) = opt(op, "session_seed") {
                match u32_of(v, "session_seed") {
                    Ok(s) => w.seed.set(s),
                    Err(e) => return Some(Err(e)),
                }
            }
            w.new_sys();
            Ok(())
        }
        "init" => w.sys().and_then(|s| check_expect(op, Json::Bool(s.init()))),
        "shutdown" => w.sys().and_then(|s| check_expect(op, Json::Bool(s.shutdown()))),
        "reload_data" => w.sys().and_then(|s| check_expect(op, Json::Bool(s.reload_data()))),
        "ticker" => w.sys().map(|s| s.on_ticker()),
        "message" => message(&w, op),
        "day" => day(&w, op),
        "simulate" => simulate(&w, op),
        "update_variables" => field(op, "advisor")
            .and_then(|a| advisor_of(&w, a))
            .map(|a| a.update_variables()),
        "push_event" => push_event(&w, op),
        "subject" => subject(&w, op),
        "set" => set(&w, op),
        "flags" => set_flags(&w, op),
        "get_trigger" => field(op, "advice")
            .and_then(|a| advice_of(&w, a))
            .and_then(|a| check_expect(op, Json::Bool(a.get_trigger()))),
        "retire" => retire(&w, op),
        "command_would_trigger" => command_would_trigger(&w, op),
        "context" => context(&w, op),
        "lua" => lua(&w, op),
        "rng" => rng(&w, op),
        "batch_size" => batch(op),
        "popup" => popup(&w, op),
        "add_advisor" => add_advisor(&w, op),
        "advisor_init" => advisor_init(&w, op),
        "create_object" => create_object(&w, op),
        "write" => write(&w, op),
        "read" => read(&w, op),
        "expect_record" => expect_record(&w, op),
        "save" => save(&w, op),
        "load" => load(&w, op),
        "expect_entry" => expect_entry(&w, op),
        "expect" => expect(&w, op),
        "expect_globals" => expect_globals(&w, op),
        "expect_local" => expect_local(&w, op),
        "expect_status" => expect_status(&w, op),
        "expect_refs" => expect_refs(&w, op),
        "expect_history_days" => expect_history_days(&w, op),
        "expect_events" => {
            w.sync();
            w.events.expect_events(op)
        }
        "expect_event_count" => {
            w.sync();
            w.events.expect_count(op)
        }
        "clear_events" => {
            w.sync();
            w.events.clear();
            Ok(())
        }
        _ => Err(format!("unknown op '{name}'")),
    })
}

// ---- setup ----------------------------------------------------------------------------------------------

/// `adv_env`: `has_city`, `established`, `difficulty` (null: unreadable),
/// `cells` ([x, z]), `day` (a day number) or `date` ([y, m, d]),
/// `start_year`, `paused`, `now_ms`, `debug`, `popup` (`{"low": v, …}` or
/// null: no table), `tutorial`, `advisors_table` (false: `advisors` is not
/// a table), `tags` ([subject, string]).
fn env(w: &World, op: &Json) -> R<()> {
    let h = &w.h;
    if let Some(v) = opt(op, "has_city") {
        h.has_city.set(bool_of(v, "has_city")?);
    }
    if let Some(v) = opt(op, "established") {
        h.established.set(bool_of(v, "established")?);
    }
    if let Some(v) = op.get("difficulty") {
        h.difficulty.set(match v {
            Json::Null => None,
            _ => Some(i64_of(v, "difficulty")? as i32),
        });
    }
    if let Some(v) = opt(op, "cells") {
        let a = arr(v, "cells")?;
        h.cells.set((u32_of(&a[0], "cells")?, u32_of(&a[1], "cells")?));
    }
    if let Some(v) = opt(op, "day") {
        h.set_day(i64_of(v, "day")? as i32);
    }
    if let Some(v) = opt(op, "date") {
        let a = arr(v, "date")?;
        let g = |i: usize| {
            a.get(i)
                .ok_or_else(|| "date: [y, m, d]".to_string())
                .and_then(|x| i64_of(x, "date"))
        };
        h.set_day(julian_from_civil(g(0)? as i32, g(1)? as i32, g(2)? as i32));
    }
    if let Some(v) = opt(op, "start_year") {
        h.start_year.set(i64_of(v, "start_year")? as i32);
    }
    if let Some(v) = opt(op, "paused") {
        h.paused.set(bool_of(v, "paused")?);
    }
    if let Some(v) = opt(op, "now_ms") {
        h.now_ms.set(u32_of(v, "now_ms")?);
    }
    if let Some(v) = opt(op, "debug") {
        h.debug.set(bool_of(v, "debug")?);
    }
    if let Some(v) = op.get("popup") {
        *h.popup.borrow_mut() = match v {
            Json::Null => None,
            _ => Some(
                v.as_object()
                    .ok_or("popup must be an object")?
                    .iter()
                    .map(|(k, x)| Ok((k.clone(), f64_of(x, k)?)))
                    .collect::<R<_>>()?,
            ),
        };
    }
    if let Some(v) = op.get("tutorial") {
        h.tutorial.set(match v {
            Json::Null => None,
            _ => Some(u32_of(v, "tutorial")?),
        });
    }
    if let Some(v) = opt(op, "advisors_table") {
        if !bool_of(v, "advisors_table")? {
            *h.advisors.borrow_mut() = None;
        } else if h.advisors.borrow().is_none() {
            *h.advisors.borrow_mut() = Some(Vec::new());
        }
    }
    if let Some(v) = opt(op, "tags") {
        let a = arr(v, "tags")?;
        h.tags
            .set((i64_of(&a[0], "tags")? as i32, i64_of(&a[1], "tags")? as i32));
    }
    Ok(())
}

/// `adv_advisor`: appends an `advisors` entry: `class`, `id`, `caption?`
/// ("Planning|tail"), `name?` ("Bob"), `advice_type?` (1).
fn advisor_entry(w: &World, op: &Json) -> R<()> {
    let e = AdvisorEntry {
        class_id: class_of(field(op, "class")?)?,
        id: i64_of(field(op, "id")?, "id")? as i32,
        caption: opt(op, "caption").map_or(Ok(b"Planning|tail".to_vec()), |v| bytes_of(v, "caption"))?,
        name: opt(op, "name").map_or(Ok(b"Bob".to_vec()), |v| bytes_of(v, "name"))?,
        advice_type: opt_i32(op, "advice_type", 1)?,
    };
    w.h.advisors.borrow_mut().get_or_insert_with(Vec::new).push(e);
    Ok(())
}

/// `adv_advice`: appends an `advices` entry: `guid`, `class?` (Advice),
/// `type?` (1), `priority?` (50), `mood?`, `frequency?` (35), `timeout?`
/// (45), `trigger?` (text; null: none), `once`, `news_only`, `event`,
/// `command`, `persist`, `no_timeout`, `effects`, `nd_type`. With
/// `replace: true` the table is emptied first.
fn advice_entry(w: &World, op: &Json) -> R<()> {
    let e = AdviceEntry {
        guid: u32_of(field(op, "guid")?, "guid")?,
        class_id: opt(op, "class").map_or(Ok(clsid::ADVICE), class_of)?,
        advice_type: opt_i32(op, "type", 1)?,
        priority: opt_i32(op, "priority", 50)?,
        mood: opt_i32(op, "mood", 0)?,
        frequency: opt_i32(op, "frequency", 35)?,
        timeout: opt_i32(op, "timeout", 45)?,
        trigger: match op.get("trigger") {
            None => Some(b"true".to_vec()),
            Some(Json::Null) => None,
            Some(v) => Some(bytes_of(v, "trigger")?),
        },
        once: opt_bool(op, "once", false)?,
        news_only: opt_bool(op, "news_only", false)?,
        event: opt_u32(op, "event", 0)?,
        command: opt_u32(op, "command", 0)?,
        persist: opt_bool(op, "persist", false)?,
        no_timeout: opt_bool(op, "no_timeout", false)?,
        effects: match opt(op, "effects") {
            Some(v) => arr(v, "effects")?
                .iter()
                .map(|x| u32_of(x, "effect"))
                .collect::<R<_>>()?,
            None => Vec::new(),
        },
        nd_type: opt_i32(op, "nd_type", 0)?,
    };
    if opt_bool(op, "replace", false)? {
        w.h.advices.borrow_mut().clear();
    }
    w.h.advices.borrow_mut().push(e);
    Ok(())
}

/// `adv_trigger`: `text` → `result` (see `lua_result_of`); `bad: true`
/// makes the text fail to compile.
fn trigger(w: &World, op: &Json) -> R<()> {
    let text = bytes_of(field(op, "text")?, "text")?;
    if opt_bool(op, "bad", false)? {
        w.h.bad_triggers.borrow_mut().push(text);
        return Ok(());
    }
    w.h.set_trigger(&text, lua_result_of(field(op, "result")?)?);
    Ok(())
}

/// `adv_condition`: the reward condition of table index `index` returns
/// `result`.
fn condition(w: &World, op: &Json) -> R<()> {
    let i = i64_of(field(op, "index")?, "index")? as i32;
    w.h.conditions
        .borrow_mut()
        .insert(i, lua_result_of(field(op, "result")?)?);
    Ok(())
}

/// `adv_value`: `src` (a `SrcKind` name), `a?`, `b?`, `object?`, `value`
/// (`{i, f, cell: [x, z], object, object2, n, m}`; null: unreadable).
fn value(w: &World, op: &Json) -> R<()> {
    let name = str_of(field(op, "src")?, "src")?;
    let kind = SrcKind::from_name(name).ok_or_else(|| format!("unknown source '{name}'"))?;
    let src = Src {
        kind,
        a: opt_u32(op, "a", 0)?,
        b: opt_u32(op, "b", 0)?,
        object: opt(op, "object").map_or(Ok(0), |v| i64_of(v, "object"))? as u64,
    };
    let v = field(op, "value")?;
    if *v == Json::Null {
        w.h.values.borrow_mut().remove(&src);
        return Ok(());
    }
    let g = |k: &str| -> R<i64> { opt(v, k).map_or(Ok(0), |x| i64_of(x, k)) };
    let val = Val {
        i: g("i")?,
        f: opt(v, "f").map_or(Ok(0.0), |x| f64_of(x, "f"))?,
        cell: opt(v, "cell").map(|c| pair_f32(c, "cell")).transpose()?,
        object: g("object")? as u64,
        object2: g("object2")? as u64,
        n: g("n")? as u64,
        m: g("m")? as u64,
    };
    w.h.set_value(src, val);
    Ok(())
}

fn list_kind_of(j: &Json) -> R<ListKind> {
    let code = match j.as_str() {
        Some("hospitals") => 1,
        Some("schools") => 2,
        Some("fire_stations") => 3,
        Some("police_stations") => 4,
        Some("port_lots") => 5,
        Some("connections") => 6,
        Some("deal_proposals") => 7,
        _ => u32_of(j, "kind")?,
    };
    ListKind::from_code(code).ok_or_else(|| format!("unknown list kind {code}"))
}

/// `adv_list`: `kind`, `a?`, `groups?`, `items` (`[{object, f0, f1, x, z,
/// flags, kind}]`; null: unreadable).
fn list(w: &World, op: &Json) -> R<()> {
    let src = ListSrc {
        kind: Some(list_kind_of(field(op, "kind")?)?),
        a: opt_u32(op, "a", 0)?,
        groups: match opt(op, "groups") {
            Some(g) => arr(g, "groups")?.iter().map(|x| u32_of(x, "group")).collect::<R<_>>()?,
            None => Vec::new(),
        },
    };
    let items = field(op, "items")?;
    if *items == Json::Null {
        w.h.lists.borrow_mut().remove(&src);
        return Ok(());
    }
    let mut v = Vec::new();
    for it in arr(items, "items")? {
        let f = |k: &str| opt(it, k).map_or(Ok(0.0), |x| f32_of(x, k));
        v.push(Item {
            object: opt(it, "object").map_or(Ok(0), |x| i64_of(x, "object"))? as u64,
            f0: f("f0")?,
            f1: f("f1")?,
            x: f("x")?,
            z: f("z")?,
            flags: opt_u32(it, "flags", 0)?,
            kind: opt_u32(it, "kind", 0)?,
        });
    }
    w.h.lists.borrow_mut().insert(src, v);
    Ok(())
}

/// `adv_occupant`: `occupant`, `type` (the occupant type), `conditional?`
/// (property 0xEA3209F8), `key_instance?`, `groups?`.
fn occupant(w: &World, op: &Json) -> R<()> {
    let o = Occupant {
        occupant_type: opt_u32(op, "type", fc4_core::advisors::OCCUPANT_TYPE_BUILDING)?,
        conditional: opt(op, "conditional").map(|v| bool_of(v, "conditional")).transpose()?,
        key_instance: opt(op, "key_instance").map(|v| u32_of(v, "key_instance")).transpose()?,
        groups: match opt(op, "groups") {
            Some(g) => arr(g, "groups")?.iter().map(|x| u32_of(x, "group")).collect::<R<_>>()?,
            None => Vec::new(),
        },
    };
    let t = i64_of(field(op, "occupant")?, "occupant")? as u64;
    w.h.occupants.borrow_mut().insert(t, o);
    Ok(())
}

/// `adv_reward_subject`: `guid` → `{location?, object?, data1?, data2?}`.
fn reward_subject(w: &World, op: &Json) -> R<()> {
    let s = SubjectInit {
        location: opt(op, "location").map(|c| pair_f32(c, "location")).transpose()?,
        object: opt(op, "object").map_or(Ok(0), |v| i64_of(v, "object"))? as u64,
        data1: opt_u32(op, "data1", 0)?,
        data2: opt_u32(op, "data2", 0)?,
    };
    w.h.reward_subjects
        .borrow_mut()
        .insert(u32_of(field(op, "guid")?, "guid")?, s);
    Ok(())
}

/// `adv_status`: the conditional-building status of `guid`: `status`,
/// `count?`, `tooltip?`; `remove: true` deletes it.
fn status(w: &World, op: &Json) -> R<()> {
    let guid = u32_of(field(op, "guid")?, "guid")?;
    if opt_bool(op, "remove", false)? {
        w.h.statuses.borrow_mut().remove(&guid);
        return Ok(());
    }
    let s = ConditionalStatus {
        status: u32_of(field(op, "status")?, "status")? as u8,
        building_count: opt_u32(op, "count", 0)? as u16,
        tooltip: opt(op, "tooltip").map_or(Ok(Vec::new()), |v| bytes_of(v, "tooltip"))?,
    };
    w.h.statuses.borrow_mut().insert(guid, s);
    Ok(())
}

/// `adv_deal`: deal object `object` of type `type`.
fn deal(w: &World, op: &Json) -> R<()> {
    let o = i64_of(field(op, "object")?, "object")? as u64;
    w.h.deals.borrow_mut().insert(o, u32_of(field(op, "type")?, "type")?);
    Ok(())
}

/// `adv_history`: history `type` → `value`, `slope`, `count`.
fn history(w: &World, op: &Json) -> R<()> {
    let t = u32_of(field(op, "type")?, "type")?;
    let f = |k: &str| opt(op, k).map_or(Ok(0.0), |v| f64_of(v, k));
    w.h.history
        .borrow_mut()
        .insert(t, (f("value")?, f("slope")? as f32, f("count")?));
    Ok(())
}

// ---- calls ----------------------------------------------------------------------------------------------

/// `adv_message`: `target` ("system", default, or an object spec), the
/// message fields; `expect` checks the result.
fn message(w: &World, op: &Json) -> R<()> {
    let m = message_of(op)?;
    let got = match op.get("target") {
        None => w.sys()?.do_message(&m),
        Some(t) => match obj_of(w, t)? {
            Obj::System(s) => s.do_message(&m),
            Obj::Advisor(a) => a.do_message(&m),
            _ => return Err("a message goes to the system or an advisor".into()),
        },
    };
    check_expect(op, Json::Bool(got))
}

/// `adv_day`: sets the date (`day` or `date`), then, with `advisor` (an
/// object spec), runs its daily step (`Simulate(0)`).
fn day(w: &World, op: &Json) -> R<()> {
    env(w, op)?;
    if let Some(a) = op.get("advisor") {
        advisor_of(w, a)?.simulate(0);
    }
    Ok(())
}

/// `adv_simulate`: `advisor`, `kind` (0 daily, 1 monthly).
fn simulate(w: &World, op: &Json) -> R<()> {
    let a = advisor_of(w, field(op, "advisor")?)?;
    let k = i64_of(field(op, "kind")?, "kind")? as i32;
    check_expect(op, Json::Bool(a.simulate(k)))
}

/// `adv_push_event`: `event`, `subject?` (a named subject); to the system,
/// or to `advisor`'s queue directly.
fn push_event(w: &World, op: &Json) -> R<()> {
    let ev = u32_of(field(op, "event")?, "event")?;
    let s = opt(op, "subject").map(|v| subject_of(w, v)).transpose()?;
    let got = match op.get("advisor") {
        Some(a) => advisor_of(w, a)?.push_event(ev, s),
        None => w.sys()?.push_event(ev, s),
    };
    check_expect(op, Json::Bool(got))
}

/// `adv_subject`: `name` (a new subject unless it exists, or `clone_of`
/// another named subject), then `location` ([x, z], set through
/// `SetLocation`), `object` (null: none), `data1`, `data2`.
fn subject(w: &World, op: &Json) -> R<()> {
    let name = str_of(field(op, "name")?, "name")?.to_owned();
    let existing = w.named.borrow().get(&name).cloned();
    let s = match (existing, opt(op, "clone_of")) {
        (_, Some(c)) => subject_of(w, c)?.clone_subject(),
        (Some(Obj::Subject(s)), None) => s,
        (_, None) => w.sys()?.new_subject(),
    };
    if let Some(v) = opt(op, "location") {
        let (x, z) = pair_f32(v, "location")?;
        s.set_location(x, z);
    }
    if let Some(v) = op.get("object") {
        s.set_object(match v {
            Json::Null => None,
            _ => Some(i64_of(v, "object")? as u64),
        });
    }
    if let Some(v) = opt(op, "data1") {
        s.set_data1(u32_of(v, "data1")?);
    }
    if let Some(v) = opt(op, "data2") {
        s.set_data2(u32_of(v, "data2")?);
    }
    w.named.borrow_mut().insert(name, Obj::Subject(s));
    Ok(())
}

/// `adv_set`: pokes `object`'s state (test setup). Advisors:
/// `month_counter`, `current_day`, `cursor`, `dirty`, `pending`,
/// `building_count`, `park_count`, `recreation_count`, `landmarks`
/// (`{"key": count}`), `info` (`{guid, next_day?, expiry?, last_month?}`),
/// `name` (the personal name), `queue: []` (empties it). The system:
/// `reload_pending: true`.
fn set(w: &World, op: &Json) -> R<()> {
    match obj_of(w, field(op, "object")?)? {
        Obj::Advisor(a) => {
            let gi = |k: &str| opt(op, k).map(|v| i64_of(v, k).map(|x| x as i32)).transpose();
            let (mc, cd, cur) = (gi("month_counter")?, gi("current_day")?, gi("cursor")?);
            let (bc, pc, rc) = (gi("building_count")?, gi("park_count")?, gi("recreation_count")?);
            let dirty = opt(op, "dirty").map(|v| bool_of(v, "dirty")).transpose()?;
            let pending = opt(op, "pending").map(|v| u32_of(v, "pending")).transpose()?;
            let mut landmarks = None;
            if let Some(v) = opt(op, "landmarks") {
                let mut m = BTreeMap::new();
                for (k, c) in v.as_object().ok_or("landmarks must be an object")? {
                    m.insert(key_of(k)? as i32, i64_of(c, k)? as i32);
                }
                landmarks = Some(m);
            }
            let clear_queue = opt(op, "queue").is_some();
            a.with_state(|s| {
                if let Some(v) = mc {
                    s.month_counter = v;
                }
                if let Some(v) = cd {
                    s.current_day = v;
                }
                if let Some(v) = cur {
                    s.cursor = v;
                }
                if let Some(v) = bc {
                    s.building_count = v;
                }
                if let Some(v) = pc {
                    s.park_count = v;
                }
                if let Some(v) = rc {
                    s.recreation_count = v;
                }
                if let Some(v) = dirty {
                    s.dirty = v;
                }
                if let Some(v) = pending {
                    s.pending = v;
                }
                if let Some(m) = landmarks {
                    s.landmarks = m;
                }
                if clear_queue {
                    s.queue.clear();
                }
            });
            if let Some(n) = opt(op, "name") {
                a.set_personal_name(&bytes_of(n, "name")?);
            }
            if let Some(i) = opt(op, "info") {
                let guid = u32_of(field(i, "guid")?, "guid")?;
                let adv = a.advice_by_guid(guid).ok_or("no such advice")?;
                let g = |k: &str| opt(i, k).map(|v| i64_of(v, k).map(|x| x as i32)).transpose();
                let (nd, ex, lm) = (g("next_day")?, g("expiry")?, g("last_month")?);
                a.with_state(|s| {
                    for inf in s.infos.iter_mut().filter(|x| Rc::ptr_eq(&x.advice, &adv)) {
                        if let Some(v) = nd {
                            inf.next_day = v;
                        }
                        if let Some(v) = ex {
                            inf.expiry = v;
                        }
                        if let Some(v) = lm {
                            inf.last_month = v;
                        }
                    }
                });
            }
            Ok(())
        }
        Obj::System(s) => {
            if opt_bool(op, "reload_pending", false)? {
                s.set_reload_pending();
            }
            Ok(())
        }
        Obj::Advice(a) => {
            if let Some(v) = opt(op, "nd_type") {
                let t = i64_of(v, "nd_type")? as i32;
                a.with_state(|s| s.nd_type = t);
            }
            if let Some(v) = opt(op, "guid") {
                let g = u32_of(v, "guid")?;
                a.with_state(|s| s.guid = g);
            }
            if let Some(v) = opt(op, "advisor") {
                a.set_advisor_id(i64_of(v, "advisor")? as i32);
            }
            Ok(())
        }
        Obj::Subject(_) => Err("use adv_subject".into()),
    }
}

/// `adv_flags`: `SetFlags(mask, on)` on `advice`.
fn set_flags(w: &World, op: &Json) -> R<()> {
    let a = advice_of(w, field(op, "advice")?)?;
    let mask = u32_of(field(op, "mask")?, "mask")?;
    let on = opt_bool(op, "on", true)?;
    check_expect(op, Json::Bool(a.set_flags(mask, on)))
}

/// `adv_retire`: `RetireAdvice(advice)` on `advisor` (`advice` null: a null
/// pointer).
fn retire(w: &World, op: &Json) -> R<()> {
    let adv = advisor_of(w, field(op, "advisor")?)?;
    let a = opt(op, "advice").map(|v| advice_of(w, v)).transpose()?;
    check_expect(op, Json::Bool(adv.retire_advice(a.as_ref())))
}

/// `adv_command_would_trigger`: `advisor`, `command`.
fn command_would_trigger(w: &World, op: &Json) -> R<()> {
    let adv = advisor_of(w, field(op, "advisor")?)?;
    let c = u32_of(field(op, "command")?, "command")?;
    check_expect(op, Json::Bool(adv.command_would_trigger(c)))
}

/// `adv_context`: `push` (an advice spec) or `pop: true` (`expect`: the
/// popped GUID or null).
fn context(w: &World, op: &Json) -> R<()> {
    let sys = w.sys()?;
    if let Some(a) = opt(op, "push") {
        sys.push_context_advice(advice_of(w, a)?);
        return Ok(());
    }
    let got = sys.pop_context_advice().map_or(Json::Null, |a| ji(a.guid()));
    check_expect(op, got)
}

fn lua_arg(w: &World, j: &Json) -> R<LuaValue> {
    Ok(match j {
        Json::Null => LuaValue::Nil,
        Json::Bool(b) => LuaValue::Bool(*b),
        Json::Object(_) => {
            if let Some(s) = j.get("str") {
                LuaValue::Str(bytes_of(s, "str")?)
            } else {
                match j.get("subject") {
                    Some(Json::Null) => LuaValue::Subject(None),
                    Some(n) => LuaValue::Subject(Some(subject_of(w, n)?)),
                    None => return Err("a Lua argument object is {str} or {subject}".into()),
                }
            }
        }
        _ => LuaValue::Number(f64_of(j, "argument")?),
    })
}

/// `adv_lua`: calls `fn` with `args` (numbers, `"0x…"`, null, booleans,
/// `{"str": s}`, `{"subject": name}`); `expect` is the result list
/// (`null` when the function does not exist), `expect_bits` the exact bits
/// of the first number.
fn lua(w: &World, op: &Json) -> R<()> {
    let sys = w.sys()?;
    let f = str_of(field(op, "fn")?, "fn")?;
    let args = match opt(op, "args") {
        Some(a) => arr(a, "args")?.iter().map(|x| lua_arg(w, x)).collect::<R<Vec<_>>>()?,
        None => Vec::new(),
    };
    let r = sys.call_lua(f, &args);
    if let Some(b) = opt(op, "expect_bits") {
        let want = u64_bits(b)?;
        let got = match r.as_ref().and_then(|v| v.first()) {
            Some(LuaValue::Number(n)) => n.to_bits(),
            _ => return Err("no number result".into()),
        };
        if got != want {
            return Err(format!("bits 0x{got:016X}, expected 0x{want:016X}"));
        }
    }
    let got = r.map_or(Json::Null, |v| Json::Array(v.iter().map(lua_value_json).collect()));
    check_expect(op, got)
}

/// `adv_rng`: sets the system generator's multiplier state `mul` (and
/// `lcg?`); `expect_mul` checks the multiplier state.
fn rng(w: &World, op: &Json) -> R<()> {
    let sys = w.sys()?;
    if let Some(m) = opt(op, "mul") {
        let mul = u32_of(m, "mul")?;
        let lcg = opt(op, "lcg").map(|v| u32_of(v, "lcg")).transpose()?;
        sys.with_random(|g| g.set_state(lcg.unwrap_or(g.lcg_state()), mul, None));
    }
    if let Some(m) = opt(op, "expect_mul") {
        let want = u32_of(m, "expect_mul")?;
        let got = sys.with_random(|g| g.mul_state());
        if got != want {
            return Err(format!("mul 0x{got:08X}, expected 0x{want:08X}"));
        }
    }
    if let Some(m) = opt(op, "expect_range") {
        // `range_f64(0, 100)` as float32.
        let want = f32_of(m, "expect_range")?;
        let got = sys.with_random(|g| g.range_f64(0.0, 100.0)) as f32;
        if got.to_bits() != want.to_bits() {
            return Err(format!("range {got:e}, expected {want:e}"));
        }
    }
    Ok(())
}

/// `adv_batch_size`: `n` → `expect`.
fn batch(op: &Json) -> R<()> {
    let n = i64_of(field(op, "n")?, "n")? as usize;
    check_expect(op, ji(batch_size(n) as i64))
}

/// `adv_popup`: re-reads the thresholds (`init: true`), then `expect` is
/// the threshold at the current difficulty.
fn popup(w: &World, op: &Json) -> R<()> {
    let sys = w.sys()?;
    if opt_bool(op, "init", false)? {
        sys.init_popup_thresholds();
    }
    check_expect(op, ji(sys.popup_threshold()))
}

/// `adv_add_advisor`: `AddAdvisor(object)`.
fn add_advisor(w: &World, op: &Json) -> R<()> {
    let a = advisor_of(w, field(op, "object")?)?;
    check_expect(op, Json::Bool(w.sys()?.add_advisor(Some(a))))
}

/// `adv_advisor_init`: `object`'s `Init(table_index)`.
fn advisor_init(w: &World, op: &Json) -> R<()> {
    let a = advisor_of(w, field(op, "object")?)?;
    let i = i64_of(field(op, "table_index")?, "table_index")? as i32;
    check_expect(op, Json::Bool(a.init(i)))
}

/// `adv_create_object`: `CreateInstance(class)` through the system's
/// factory, named `as`; `expect: false` when the class is unknown.
fn create_object(w: &World, op: &Json) -> R<()> {
    let c = class_of(field(op, "class")?)?;
    let r = w.sys()?.create_object(c);
    let made = r.as_ref().map(|r| match r {
        RefValue::Object(o) => {
            let any = o.clone().as_any_rc();
            if let Ok(a) = any.clone().downcast::<Advisor>() {
                Some(Obj::Advisor(a))
            } else if let Ok(a) = any.clone().downcast::<Advice>() {
                Some(Obj::Advice(a))
            } else if let Ok(s) = any.downcast::<AdviceSubject>() {
                Some(Obj::Subject(s))
            } else {
                None
            }
        }
        _ => None,
    });
    if let Some(Some(o)) = made {
        w.named
            .borrow_mut()
            .insert(str_of(field(op, "as")?, "as")?.to_owned(), o);
    }
    check_expect(op, Json::Bool(r.is_some()))
}

// ---- records --------------------------------------------------------------------------------------------

/// Resolves references of a record read: core classes through the system's
/// factory (each new object named `ref<id>`), anything else as a host
/// object whose token is the object ID.
struct Resolver<'a> {
    w: &'a World,
    made: BTreeMap<(u32, u32), RefValue>,
}

impl RefResolver for Resolver<'_> {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        if let Some(r) = self.made.get(&(class_id, object_id)) {
            return Some(r.clone());
        }
        let core = self.w.sys().ok().and_then(|s| s.create_object(class_id));
        let r = match core {
            Some(r) => {
                if let RefValue::Object(o) = &r {
                    let any = o.clone().as_any_rc();
                    let named = if let Ok(a) = any.clone().downcast::<Advisor>() {
                        Some(Obj::Advisor(a))
                    } else if let Ok(a) = any.clone().downcast::<Advice>() {
                        Some(Obj::Advice(a))
                    } else {
                        any.downcast::<AdviceSubject>().ok().map(Obj::Subject)
                    };
                    if let Some(n) = named {
                        self.w.named.borrow_mut().insert(format!("ref{object_id}"), n);
                    }
                }
                r
            }
            None => RefValue::Host {
                token: u64::from(object_id),
                class_id,
            },
        };
        self.made.insert((class_id, object_id), r.clone());
        Some(r)
    }
}

/// `adv_write`: writes `object`'s record (references numbered from 1 in
/// the order written) under `name`; `expect` checks the result.
fn write(w: &World, op: &Json) -> R<()> {
    let o = obj_of(w, field(op, "object")?)?;
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut out = ByteWriter::with_registry(reg);
    let ok = save_obj(&o).write(&mut out);
    w.records
        .borrow_mut()
        .insert(str_of(field(op, "name")?, "name")?.to_owned(), out.into_bytes());
    check_expect(op, Json::Bool(ok))
}

pub(crate) fn record_input(w: &World, op: &Json) -> R<Vec<u8>> {
    match (opt(op, "hex"), opt(op, "record")) {
        (Some(h), _) => hex_decode(str_of(h, "hex")?),
        (None, Some(n)) => w
            .records
            .borrow()
            .get(str_of(n, "record")?)
            .cloned()
            .ok_or_else(|| "no such record".into()),
        _ => Err("give 'hex' or 'record'".into()),
    }
}

/// `adv_read`: reads `hex` (or the record `record`) into `object`; `expect`
/// checks the result. References resolve as `Resolver` documents.
fn read(w: &World, op: &Json) -> R<()> {
    let o = obj_of(w, field(op, "object")?)?;
    let bytes = record_input(w, op)?;
    let mut res = Resolver {
        w,
        made: BTreeMap::new(),
    };
    let mut r = ByteReader::with_resolver(&bytes, &mut res);
    let ok = save_obj(&o).read(&mut r);
    check_expect(op, Json::Bool(ok))
}

/// `adv_expect_record`: record `name` equals `hex`.
pub(crate) fn expect_record(w: &World, op: &Json) -> R<()> {
    let name = str_of(field(op, "name")?, "name")?;
    let got = w
        .records
        .borrow()
        .get(name)
        .cloned()
        .ok_or_else(|| format!("no record '{name}'"))?;
    let want = hex_decode(str_of(field(op, "hex")?, "hex")?)?;
    if got != want {
        return Err(format!(
            "record differs:\n    got      {}\n    expected {}",
            hex_encode(&got),
            hex_encode(&want)
        ));
    }
    Ok(())
}

/// `adv_save`: saves the system (and everything it references) through
/// the class-object store as `name`.
fn save(w: &World, op: &Json) -> R<()> {
    let sys = w.sys()?;
    let s = scalar::save(&[RefValue::Object(sys)])?;
    w.saves
        .borrow_mut()
        .insert(str_of(field(op, "name")?, "name")?.to_owned(), s);
    Ok(())
}

/// `adv_load`: loads save `name` into the current system (its record is
/// the system's); every other core object comes from the factory.
/// `expect` checks a clean store load.
fn load(w: &World, op: &Json) -> R<()> {
    let sys = w.sys()?;
    let saved = w
        .saves
        .borrow()
        .get(str_of(field(op, "name")?, "name")?)
        .cloned()
        .ok_or("no such save")?;
    let mut factory = |class: u32, oid: u32| -> Option<RefValue> {
        if class == clsid::SYSTEM {
            return Some(RefValue::Object(sys.clone()));
        }
        sys.create_object(class).or(Some(RefValue::Host {
            token: u64::from(oid),
            class_id: class,
        }))
    };
    let (ok, _) = scalar::load(&saved, &mut factory);
    check_expect(op, Json::Bool(ok))
}

/// `adv_expect_entry`: the payload of entry `entry` (0) of class `class`'s
/// store record in save `save` equals `hex`.
fn expect_entry(w: &World, op: &Json) -> R<()> {
    let saves = w.saves.borrow();
    let s = saves.get(str_of(field(op, "save")?, "save")?).ok_or("no such save")?;
    let class = class_of(field(op, "class")?)?;
    let entry = opt(op, "entry").map_or(Ok(0), |v| i64_of(v, "entry"))? as usize;
    let got = scalar::entry_payload(s, class, entry)?;
    let want = hex_decode(str_of(field(op, "hex")?, "hex")?)?;
    if got != want {
        return Err(format!(
            "entry differs:\n    got      {}\n    expected {}",
            hex_encode(&got),
            hex_encode(&want)
        ));
    }
    Ok(())
}

// ---- checks ---------------------------------------------------------------------------------------------

/// `adv_expect`: `object`'s partial `state` (see README), or `absent: true`
/// for an advice spec that must not resolve.
fn expect(w: &World, op: &Json) -> R<()> {
    let spec = field(op, "object")?;
    if opt_bool(op, "absent", false)? {
        return match obj_of(w, spec) {
            Ok(_) => Err("object present".into()),
            Err(_) => Ok(()),
        };
    }
    let o = obj_of(w, spec)?;
    check_state(&obj_state(w, &o), field(op, "state")?)
}

/// `adv_expect_globals`: `values` (`{"name": value}`; null: never written).
fn expect_globals(w: &World, op: &Json) -> R<()> {
    let mut errors = Vec::new();
    for (k, v) in field(op, "values")?.as_object().ok_or("values must be an object")? {
        let got = w.h.global(k).map_or(Json::Null, jnum);
        if !scalar::matches(&got, v, false) {
            errors.push(format!("{k}: got {}, expected {}", scalar::show(&got), scalar::show(v)));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

/// `adv_expect_local`: the last `set_local` of `name`: `value`, `subject`
/// (partial subject or null); `absent: true`: never set.
fn expect_local(w: &World, op: &Json) -> R<()> {
    let name = str_of(field(op, "name")?, "name")?;
    // The world's log may already have been moved to the event log: search
    // the host's log first, then the globals map for the value.
    let got = w.h.local(name);
    if opt_bool(op, "absent", false)? {
        return match got {
            None => Ok(()),
            Some(_) => Err(format!("{name} was set")),
        };
    }
    let (v, s) = got.ok_or_else(|| format!("{name} never set (since the last event check)"))?;
    let got = Json::Object(vec![
        ("value".into(), jnum(v)),
        ("subject".into(), s.as_ref().map_or(Json::Null, subject_json)),
    ]);
    let mut want = Vec::new();
    for k in ["value", "subject"] {
        if let Some(x) = op.get(k) {
            want.push((k.to_owned(), x.clone()));
        }
    }
    if !scalar::matches(&got, &Json::Object(want.clone()), true) {
        return Err(format!(
            "{name}: got {}, expected {}",
            scalar::show(&got),
            scalar::show(&Json::Object(want))
        ));
    }
    Ok(())
}

/// `adv_expect_status`: the status of `guid`: `state` (partial: `status`,
/// `count`, `tooltip`) or `absent: true`.
fn expect_status(w: &World, op: &Json) -> R<()> {
    let guid = u32_of(field(op, "guid")?, "guid")?;
    let got = w.h.statuses.borrow().get(&guid).cloned();
    if opt_bool(op, "absent", false)? {
        return if got.is_none() {
            Ok(())
        } else {
            Err("status present".into())
        };
    }
    let s = got.ok_or("no status")?;
    check_state(
        &[
            ("status".into(), ji(s.status)),
            ("count".into(), ji(s.building_count)),
            ("tooltip".into(), jstr(&s.tooltip)),
        ],
        field(op, "state")?,
    )
}

/// `adv_expect_refs`: `refs` (`{"token": count}`) of host objects.
fn expect_refs(w: &World, op: &Json) -> R<()> {
    let mut errors = Vec::new();
    for (k, v) in field(op, "refs")?.as_object().ok_or("refs must be an object")? {
        let t = key_of(k)? as u64;
        let got = w.h.refs.borrow().get(&t).copied().unwrap_or(0);
        if got != i64_of(v, k)? {
            errors.push(format!("{k}: {got}"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("references differ: {}", errors.join(", ")))
    }
}

/// `adv_expect_history_days`: the history reads so far (`[[type, from,
/// to]]`); clears them.
fn expect_history_days(w: &World, op: &Json) -> R<()> {
    let got: Vec<Json> = std::mem::take(&mut *w.h.history_days.borrow_mut())
        .into_iter()
        .map(|(t, a, b)| Json::Array(vec![ji(t), ji(a), ji(b)]))
        .collect();
    let want = field(op, "days")?;
    if !scalar::matches(&Json::Array(got.clone()), want, false) {
        return Err(format!(
            "history reads {}, expected {}",
            scalar::show(&Json::Array(got)),
            scalar::show(want)
        ));
    }
    Ok(())
}

fn u64_bits(j: &Json) -> R<u64> {
    let s = str_of(j, "bits")?;
    let h = s.strip_prefix("0x").ok_or("bits must be \"0x…\"")?;
    u64::from_str_radix(h, 16).map_err(|e| e.to_string())
}
