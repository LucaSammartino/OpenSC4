// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/dispatch.md`: scripted dispatch sources
//! (types, stations with unit masks), vehicles, pylons and references, and
//! ops that drive the dispatch manager. See README.md ("Dispatch ops").

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

use fc4_core::dispatch::{
    msg, query_interface, DispatchHost, DispatchInfo, DispatchManager, DispatchMessage, StationUnits,
    CLSID_DISPATCH_MANAGER,
};
use fc4_core::stream::{ByteReader, ByteWriter, RefResolver, RefValue, SaveObject, SaveRegistry};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{boolean, field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, f32_of, i64_of, jf, ji, opt, str_of, u32_of, EventLog, R,
};

const MANAGER_TOKEN: u64 = 0xD15;
const VIEW_TOKEN: u64 = 0x7E;
/// The class ID of host objects in written records.
const HOST_CLASS: u32 = 0xC1A5_5000;

#[derive(Clone, Debug, Default)]
struct Source {
    types: Vec<u32>,
    can: bool,
    all_busy: bool,
    stations: Vec<StationUnits>,
    nearest: bool,
}

#[derive(Clone, Debug, Default)]
struct Pylon {
    source: Option<u64>,
    call: u32,
    dtype: u32,
    dstate: u32,
    ui: u32,
}

pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    mgr: RefCell<Option<Rc<DispatchManager>>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    sources: RefCell<BTreeMap<u64, Source>>,
    pylons: RefCell<BTreeMap<u64, Pylon>>,
    next: Cell<u64>,
    vehicles_fail: Cell<bool>,
    next_handle: Cell<u32>,
    record: RefCell<Vec<u8>>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            me: RefCell::new(Weak::new()),
            events: EventLog::default(),
            mgr: RefCell::new(None),
            refs: RefCell::new(BTreeMap::new()),
            sources: RefCell::new(BTreeMap::new()),
            pylons: RefCell::new(BTreeMap::new()),
            next: Cell::new(0x1000),
            vehicles_fail: Cell::new(false),
            next_handle: Cell::new(1),
            record: RefCell::new(Vec::new()),
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn mgr(&self) -> R<Rc<DispatchManager>> {
        self.mgr
            .borrow()
            .clone()
            .ok_or_else(|| "no dispatch manager (dispatch_create)".into())
    }

    fn ev(&self, kind: &str, fields: Vec<(&str, Json)>) {
        self.events.push(kind, fields);
    }

    /// A new host object (one reference for the core).
    fn fresh(&self) -> u64 {
        let t = self.next.get();
        self.next.set(t + 1);
        *self.refs.borrow_mut().entry(t).or_insert(0) += 1;
        t
    }

    /// Marks the unit `call` of `src`'s station `station` busy or free.
    fn unit(&self, src: u64, station: Option<u64>, call: u32, free: bool) {
        let mut s = self.sources.borrow_mut();
        let Some(s) = s.get_mut(&src) else {
            return;
        };
        for st in &mut s.stations {
            if station.is_some_and(|t| t != st.station) || call < st.base || call >= st.base + st.count {
                continue;
            }
            let bit = 1u32 << (call - st.base);
            if free {
                st.mask |= bit;
            } else {
                st.mask &= !bit;
            }
            return;
        }
    }

    fn pylon_mut(&self, p: u64, f: impl FnOnce(&mut Pylon)) {
        if let Some(x) = self.pylons.borrow_mut().get_mut(&p) {
            f(x);
        }
    }
}

fn tok(t: u64) -> Json {
    ji(t as i64)
}

fn opt_tok(t: Option<u64>) -> Json {
    t.map_or(Json::Null, tok)
}

impl DispatchHost for World {
    fn acquire_message_server(&self) -> bool {
        self.ev("acquire", vec![("service", Json::Str("message_server".into()))]);
        true
    }
    fn subscribe(&self, m: u32) {
        self.ev("subscribe", vec![("type", ji(m))]);
    }
    fn unsubscribe(&self, m: u32) {
        self.ev("unsubscribe", vec![("type", ji(m))]);
    }
    fn release_message_server(&self) {
        self.ev("release_service", vec![("service", Json::Str("message_server".into()))]);
    }
    fn post(&self, m: u32, t: u32, id: u32) {
        self.ev("post", vec![("type", ji(m)), ("dispatch_type", ji(t)), ("id", ji(id))]);
    }
    fn acquire_city(&self) -> bool {
        self.ev("acquire", vec![("service", Json::Str("city".into()))]);
        true
    }
    fn release_city(&self) {
        self.ev("release_service", vec![("service", Json::Str("city".into()))]);
    }
    fn create_vehicle_view(&self) -> Option<u64> {
        self.ev("create_view", vec![]);
        Some(VIEW_TOKEN)
    }
    fn destroy_vehicle_view(&self, v: u64) {
        self.ev("destroy_view", vec![("view", tok(v))]);
    }
    fn refresh_vehicle_view(&self, v: Option<u64>) {
        self.ev("refresh_view", vec![("view", opt_tok(v))]);
    }
    fn track_vehicle(&self, a: u64, kind: u32) -> u32 {
        let h = self.next_handle.get();
        self.next_handle.set(h + 1);
        self.ev(
            "track",
            vec![("automaton", tok(a)), ("kind", ji(kind)), ("handle", ji(h))],
        );
        h
    }
    fn untrack_vehicle(&self, h: u32) {
        self.ev("untrack", vec![("handle", ji(h))]);
    }
    fn add_ref(&self, t: u64) {
        *self.refs.borrow_mut().entry(t).or_insert(0) += 1;
    }
    fn release(&self, t: u64) {
        *self.refs.borrow_mut().entry(t).or_insert(0) -= 1;
    }
    fn altitude(&self, x: f32, z: f32) -> f32 {
        (x + z) / 100.0
    }
    fn source_can_handle(&self, src: u64, t: u32) -> bool {
        self.sources.borrow().get(&src).is_some_and(|s| s.types.contains(&t))
    }
    fn source_can_dispatch(&self, src: u64, _x: f32, _z: f32) -> (bool, bool) {
        self.sources
            .borrow()
            .get(&src)
            .map_or((false, false), |s| (s.can, s.all_busy))
    }
    fn source_dispatch_from_nearest_station(&self, src: u64, _x: f32, _z: f32) -> Option<(u64, u32)> {
        let (station, call) = {
            let s = self.sources.borrow();
            let s = s.get(&src)?;
            if !s.nearest {
                return None;
            }
            s.stations.iter().find_map(|st| {
                (0..st.count)
                    .find(|i| st.mask & (1 << i) != 0)
                    .map(|i| (st.station, st.base + i))
            })?
        };
        self.unit(src, Some(station), call, false);
        self.ev("nearest", vec![("station", tok(station)), ("call", ji(call))]);
        Some((station, call))
    }
    fn source_dispatch_from_station(&self, src: u64, station: Option<u64>, call: &mut u32) -> bool {
        self.unit(src, station, *call, false);
        self.ev("from_station", vec![("station", opt_tok(station)), ("call", ji(*call))]);
        true
    }
    fn source_revoke_dispatch_from_station(&self, src: u64, station: Option<u64>, call: u32) {
        self.unit(src, station, call, true);
        self.ev(
            "revoke_from_station",
            vec![("station", opt_tok(station)), ("call", ji(call))],
        );
    }
    fn source_stations(&self, src: u64) -> Vec<StationUnits> {
        self.sources
            .borrow()
            .get(&src)
            .map_or_else(Vec::new, |s| s.stations.clone())
    }
    fn begin_vehicle(&self, _src: u64, info: &DispatchInfo) -> Option<u64> {
        if self.vehicles_fail.get() {
            self.ev("vehicle_failed", vec![("id", ji(info.id))]);
            return None;
        }
        let a = info.automaton.unwrap_or_else(|| self.fresh());
        self.ev(
            "vehicle",
            vec![("automaton", tok(a)), ("id", ji(info.id)), ("call", ji(info.call_id))],
        );
        Some(a)
    }
    fn automaton_despawn(&self, a: u64) {
        self.ev("despawn", vec![("automaton", tok(a))]);
    }
    fn automaton_go_home(&self, a: u64) {
        self.ev("go_home", vec![("automaton", tok(a))]);
    }
    fn create_pylon(&self) -> Option<u64> {
        let p = self.fresh();
        self.pylons.borrow_mut().insert(p, Pylon::default());
        self.ev("create_pylon", vec![("pylon", tok(p))]);
        Some(p)
    }
    fn pylon_source(&self, p: u64) -> Option<u64> {
        self.pylons.borrow().get(&p).and_then(|x| x.source)
    }
    fn pylon_set_source(&self, p: u64, st: u64) {
        self.pylon_mut(p, |x| x.source = Some(st));
    }
    fn pylon_call_number(&self, p: u64) -> u32 {
        self.pylons.borrow().get(&p).map_or(0, |x| x.call)
    }
    fn pylon_set_call_number(&self, p: u64, n: u32) {
        self.pylon_mut(p, |x| x.call = n);
    }
    fn pylon_dispatch_type(&self, p: u64) -> u32 {
        self.pylons.borrow().get(&p).map_or(0, |x| x.dtype)
    }
    fn pylon_set_dispatch_type(&self, p: u64, t: u32) {
        self.pylon_mut(p, |x| x.dtype = t);
    }
    fn pylon_set_dispatch_state(&self, p: u64, s: u32) {
        self.pylon_mut(p, |x| x.dstate = s);
    }
    fn pylon_set_ui_state(&self, p: u64, s: u32) {
        self.pylon_mut(p, |x| x.ui = s);
    }
    fn pylon_move_to(&self, p: u64, x: f32, y: f32, z: f32) {
        self.ev(
            "move",
            vec![("pylon", tok(p)), ("x", jf(x)), ("y", jf(y)), ("z", jf(z))],
        );
    }
    fn pylon_place_at_unit(&self, p: u64, st: u64, i: u32, n: u32) {
        self.ev(
            "place",
            vec![
                ("pylon", tok(p)),
                ("station", tok(st)),
                ("index", ji(i)),
                ("count", ji(n)),
            ],
        );
    }
    fn pylon_return_to_station(&self, p: u64, st: u64) {
        self.ev("park", vec![("pylon", tok(p)), ("station", tok(st))]);
    }
    fn pylon_shutdown(&self, p: u64) {
        self.ev("pylon_shutdown", vec![("pylon", tok(p))]);
    }
    fn object_ref(&self, t: u64) -> RefValue {
        RefValue::Host {
            token: t,
            class_id: HOST_CLASS,
        }
    }
    fn resolve_ref(&self, r: &RefValue, _iid: u32) -> Option<u64> {
        match *r {
            RefValue::Host { token, .. } => {
                self.add_ref(token);
                Some(token)
            }
            _ => None,
        }
    }
}

/// Resolves every reference to its object ID as the token (the core's
/// reference is taken by `resolve_ref`).
struct Ids;

impl RefResolver for Ids {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        Some(RefValue::Host {
            token: u64::from(object_id),
            class_id,
        })
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "dispatch", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("dispatch_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "source" => source(&w, op),
        "create" => {
            let m = DispatchManager::new(w.rc() as Rc<dyn DispatchHost>, MANAGER_TOKEN);
            *w.mgr.borrow_mut() = Some(m);
            Ok(())
        }
        "init" => w.mgr().and_then(|m| check_expect(op, Json::Bool(m.init()))),
        "shutdown" => w.mgr().and_then(|m| check_expect(op, Json::Bool(m.shutdown()))),
        "add_ref" => w.mgr().and_then(|m| check_expect(op, ji(m.add_ref()))),
        "release" => w.mgr().and_then(|m| check_expect(op, ji(m.release()))),
        "identity" => identity(&w, op),
        "message" => message(&w, op),
        "call" => call(&w, op),
        "expect" => expect(&w, op),
        "expect_units" => expect_units(&w, op),
        "expect_refs" => expect_refs(&w, op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        "write" => write(&w, op),
        "read" => read(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    })
}

fn token_of(j: &Json, what: &str) -> R<u64> {
    Ok(i64_of(j, what)? as u64)
}

/// `dispatch_env`: `vehicles_fail` (BeginVehicle fails).
fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "vehicles_fail") {
        w.vehicles_fail.set(bool_of(v, "vehicles_fail")?);
    }
    Ok(())
}

/// `dispatch_source`: `source`, `types`, `can` (CanDispatch, default true), `all_busy`
/// (default false), `nearest` (DispatchFromNearestStation finds the first free unit,
/// default true), `stations` ([{"station", "base", "count", "mask" (default all free)}]),
/// `register` (default true: RegisterDispatchSource). Updating an existing source changes
/// only the given fields.
fn source(w: &World, op: &Json) -> R<()> {
    let src = token_of(field(op, "source")?, "source")?;
    let existing = w.sources.borrow().get(&src).cloned();
    let fresh = existing.is_none();
    let mut s = existing.unwrap_or(Source {
        can: true,
        nearest: true,
        ..Source::default()
    });
    if let Some(v) = opt(op, "types") {
        s.types = arr(v, "types")?.iter().map(|e| u32_of(e, "types")).collect::<R<_>>()?;
    }
    if let Some(v) = opt(op, "can") {
        s.can = bool_of(v, "can")?;
    }
    if let Some(v) = opt(op, "all_busy") {
        s.all_busy = bool_of(v, "all_busy")?;
    }
    if let Some(v) = opt(op, "nearest") {
        s.nearest = bool_of(v, "nearest")?;
    }
    if let Some(v) = opt(op, "stations") {
        s.stations = arr(v, "stations")?
            .iter()
            .map(|e| {
                let count = u32_of(field(e, "count")?, "count")?;
                Ok(StationUnits {
                    station: token_of(field(e, "station")?, "station")?,
                    base: opt(e, "base").map_or(Ok(1), |v| u32_of(v, "base"))?,
                    count,
                    mask: opt(e, "mask").map_or(Ok(((1u64 << count) - 1) as u32), |v| u32_of(v, "mask"))?,
                })
            })
            .collect::<R<_>>()?;
    }
    w.sources.borrow_mut().insert(src, s);
    if fresh && opt(op, "register").and_then(boolean).unwrap_or(true) {
        w.mgr()?.register_dispatch_source(src);
    }
    Ok(())
}

/// `dispatch_identity`: `iids` (`{"0xIID": offset or null}`) and `class`.
fn identity(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "iids") {
        for (k, want) in v.as_object().ok_or("iids must be an object")? {
            let iid = u32_of(&Json::Str(k.clone()), "iid")?;
            let got = query_interface(iid).map_or(Json::Null, ji);
            if !scalar::matches(&got, want, false) {
                return Err(format!("QueryInterface({k}) = {}", scalar::show(&got)));
            }
        }
    }
    if let Some(v) = opt(op, "class") {
        let want = u32_of(v, "class")?;
        let got = w.mgr().map_or(CLSID_DISPATCH_MANAGER, |m| m.class_id());
        if got != want {
            return Err(format!("class 0x{got:08X}, expected 0x{want:08X}"));
        }
    }
    Ok(())
}

/// The automaton of record `id` (for `automaton_of`).
fn automaton_of(w: &World, j: &Json) -> R<u64> {
    let id = u32_of(j, "automaton_of")?;
    w.mgr()?
        .get_dispatch_info(id)
        .and_then(|i| i.automaton)
        .ok_or_else(|| format!("no automaton for dispatch {id}"))
}

/// `dispatch_message`: `type` (RemoveOccupant, DisasterPeriodEnded, ViewChanged,
/// AutomatonEvent or an ID), `code` (data2), `state` (data3), `object` or `automaton_of`
/// (a dispatch ID: its record's automaton).
fn message(w: &World, op: &Json) -> R<()> {
    let m = w.mgr()?;
    let t = match field(op, "type")?.as_str() {
        Some("RemoveOccupant") => msg::REMOVE_OCCUPANT,
        Some("DisasterPeriodEnded") => msg::DISASTER_PERIOD_ENDED,
        Some("ViewChanged") => msg::VIEW_CHANGED,
        Some("AutomatonEvent") => msg::AUTOMATON_EVENT,
        _ => u32_of(field(op, "type")?, "type")?,
    };
    let object = match (opt(op, "object"), opt(op, "automaton_of")) {
        (Some(v), _) => Some(token_of(v, "object")?),
        (None, Some(v)) => Some(automaton_of(w, v)?),
        _ => None,
    };
    let dm = DispatchMessage {
        msg_type: t,
        data2: opt(op, "code").map_or(Ok(0), |v| u32_of(v, "code"))?,
        data3: opt(op, "state").map_or(Ok(0), |v| u32_of(v, "state"))?,
        object,
    };
    check_expect(op, Json::Bool(m.do_message(&dm)))
}

fn a_u32(op: &Json, i: usize) -> R<u32> {
    u32_of(arg(op, i)?, "argument")
}

fn a_f32(op: &Json, i: usize) -> R<f32> {
    f32_of(arg(op, i)?, "argument")
}

fn a_tok(op: &Json, i: usize) -> R<Option<u64>> {
    match arg(op, i)? {
        Json::Null => Ok(None),
        v => token_of(v, "argument").map(Some),
    }
}

fn info_json(i: &DispatchInfo) -> Json {
    Json::Object(vec![
        ("type".to_owned(), ji(i.dispatch_type)),
        ("id".to_owned(), ji(i.id)),
        ("call".to_owned(), ji(i.call_id)),
        ("view_handle".to_owned(), ji(i.view_handle)),
        ("active".to_owned(), Json::Bool(i.active)),
        (
            "target".to_owned(),
            Json::Array(vec![jf(i.target.0), jf(i.target.1), jf(i.target.2)]),
        ),
        ("station".to_owned(), opt_tok(i.station)),
        ("automaton".to_owned(), opt_tok(i.automaton)),
        ("pylon".to_owned(), opt_tok(i.pylon)),
    ])
}

/// `dispatch_call`: `method` with `args`; `expect` checks the result. Methods: dispatch
/// (type, x, z), can_dispatch (type, x, z), get_dispatch_source (type), get_dispatch_info
/// (id), get_dispatch_count (type), get_dispatch_id_array (type, n), get_active_pylon,
/// get_automaton_from_pylon (pylon), get_available_pylon_count (type),
/// get_dispatch_vehicle_view, update_dispatch_state (id, state), revoke_dispatch_by_id (id),
/// revoke_dispatch_by_pylon (pylon), revoke_all_dispatches (type), begin_dispatch_tool
/// (type), end_dispatch_tool (type), set_active_pylon (pylon or null),
/// register_dispatch_source / unregister_dispatch_source (source). A record result is
/// compared on the fields `expect` gives.
fn call(w: &World, op: &Json) -> R<()> {
    let m = w.mgr()?;
    let got = match str_of(field(op, "method")?, "method")? {
        "dispatch" => ji(m.dispatch(a_u32(op, 0)?, a_f32(op, 1)?, a_f32(op, 2)?)),
        "can_dispatch" => Json::Bool(m.can_dispatch(a_u32(op, 0)?, a_f32(op, 1)?, a_f32(op, 2)?)),
        "get_dispatch_source" => opt_tok(m.get_dispatch_source(a_u32(op, 0)?)),
        "get_dispatch_info" => m.get_dispatch_info(a_u32(op, 0)?).map_or(Json::Null, |i| info_json(&i)),
        "get_dispatch_count" => ji(m.get_dispatch_count(a_u32(op, 0)?)),
        "get_dispatch_id_array" => Json::Array(
            m.get_dispatch_id_array(a_u32(op, 0)?, a_u32(op, 1)?)
                .into_iter()
                .map(ji)
                .collect(),
        ),
        "get_active_pylon" => opt_tok(m.get_active_pylon()),
        "get_automaton_from_pylon" => opt_tok(m.get_automaton_from_pylon(a_tok(op, 0)?.unwrap_or(0))),
        "get_available_pylon_count" => ji(m.get_available_pylon_count(a_u32(op, 0)?)),
        "get_dispatch_vehicle_view" => opt_tok(m.get_dispatch_vehicle_view()),
        "update_dispatch_state" => {
            m.update_dispatch_state(a_u32(op, 0)?, a_u32(op, 1)?);
            Json::Null
        }
        "revoke_dispatch_by_id" => Json::Bool(m.revoke_dispatch_by_id(a_u32(op, 0)?)),
        "revoke_dispatch_by_pylon" => Json::Bool(m.revoke_dispatch_by_pylon(a_tok(op, 0)?.unwrap_or(0))),
        "revoke_all_dispatches" => Json::Bool(m.revoke_all_dispatches(a_u32(op, 0)?)),
        "begin_dispatch_tool" => Json::Bool(m.begin_dispatch_tool(a_u32(op, 0)?)),
        "end_dispatch_tool" => Json::Bool(m.end_dispatch_tool(a_u32(op, 0)?)),
        "set_active_pylon" => {
            m.set_active_pylon(a_tok(op, 0)?);
            Json::Null
        }
        "register_dispatch_source" => {
            m.register_dispatch_source(a_tok(op, 0)?.unwrap_or(0));
            Json::Null
        }
        "unregister_dispatch_source" => {
            m.unregister_dispatch_source(a_tok(op, 0)?.unwrap_or(0));
            Json::Null
        }
        other => return Err(format!("unknown dispatch method '{other}'")),
    };
    // A record (get_dispatch_info) is compared on the fields `expect` gives.
    if let (Json::Object(_), Some(want)) = (&got, op.get("expect")) {
        if !scalar::matches(&got, want, true) {
            return Err(format!(
                "returned {}, expected {}",
                scalar::show(&got),
                scalar::show(want)
            ));
        }
        return Ok(());
    }
    check_expect(op, got)
}

fn expect(w: &World, op: &Json) -> R<()> {
    let s = w.mgr()?.snapshot();
    let got = vec![
        ("initialized".to_owned(), Json::Bool(s.initialized)),
        ("destroyed".to_owned(), Json::Bool(s.destroyed)),
        ("ref_count".to_owned(), ji(s.ref_count)),
        ("loaded".to_owned(), Json::Bool(s.loaded)),
        ("next_id".to_owned(), ji(s.next_id)),
        ("tool_type".to_owned(), ji(s.tool_type)),
        ("record_count".to_owned(), ji(s.records.len() as i64)),
        (
            "records".to_owned(),
            Json::Array(s.records.iter().map(info_json).collect()),
        ),
        ("active_pylon".to_owned(), opt_tok(s.active_pylon)),
        (
            "available_pylons".to_owned(),
            Json::Array(s.available_pylons.iter().map(|&p| opt_tok(p)).collect()),
        ),
        (
            "available_calls".to_owned(),
            Json::Array(
                s.available_pylons
                    .iter()
                    .map(|&p| p.map_or(Json::Null, |p| ji(w.pylon_call_number(p))))
                    .collect(),
            ),
        ),
        (
            "sources".to_owned(),
            Json::Array(s.sources.iter().map(|&t| tok(t)).collect()),
        ),
        ("vehicle_view".to_owned(), opt_tok(s.vehicle_view)),
    ];
    check_state(&got, field(op, "state")?)
}

/// `dispatch_expect_units`: `source` with `masks` (one per station).
fn expect_units(w: &World, op: &Json) -> R<()> {
    let src = token_of(field(op, "source")?, "source")?;
    let got: Vec<Json> = w
        .sources
        .borrow()
        .get(&src)
        .ok_or("unknown source")?
        .stations
        .iter()
        .map(|s| ji(s.mask))
        .collect();
    check_expect(
        &Json::Object(vec![("expect".to_owned(), field(op, "masks")?.clone())]),
        Json::Array(got),
    )
}

/// `dispatch_expect_refs`: `{"token": references}`.
fn expect_refs(w: &World, op: &Json) -> R<()> {
    for (k, v) in field(op, "refs")?.as_object().ok_or("refs must be an object")? {
        let t = token_of(&Json::Str(k.clone()), "token")?;
        let got = w.refs.borrow().get(&t).copied().unwrap_or(0);
        if got != i64_of(v, k)? {
            return Err(format!("{k}: {got} references"));
        }
    }
    Ok(())
}

/// `dispatch_write`: `expect`, `hex`, `length`.
fn write(w: &World, op: &Json) -> R<()> {
    let m = w.mgr()?;
    let mut out = ByteWriter::with_registry(Rc::new(RefCell::new(SaveRegistry::new())));
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
            return Err(format!("record {}, expected {}", hex_encode(&bytes), hex_encode(&want)));
        }
    }
    *w.record.borrow_mut() = bytes;
    Ok(())
}

/// `dispatch_read`: `hex` (references resolve to their object IDs) or the last written
/// record, into a new constructed manager (or the current one with `reuse`); `expect`.
fn read(w: &World, op: &Json) -> R<()> {
    let bytes = match opt(op, "hex") {
        Some(h) => hex_decode(str_of(h, "hex")?)?,
        None => w.record.borrow().clone(),
    };
    let m = if opt(op, "reuse").and_then(boolean).unwrap_or(false) {
        w.mgr()?
    } else {
        let m = DispatchManager::new(w.rc() as Rc<dyn DispatchHost>, MANAGER_TOKEN);
        *w.mgr.borrow_mut() = Some(m.clone());
        m
    };
    let mut ids = Ids;
    let ok = m.read(&mut ByteReader::with_resolver(&bytes, &mut ids));
    check_expect(op, Json::Bool(ok))
}
