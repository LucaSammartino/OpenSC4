// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/occupant-manager.md`: a scripted world
//! of occupants (bounding box, type, flags, stored manager box, the base
//! `Init` / `Shutdown` / `PostOccupantMessage` of §5.6), caller filters, and
//! ops that drive the occupant manager. See README.md ("Occupant manager
//! ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::{Rc, Weak};

use fc4_core::occupant_manager::{
    bounding_cells, msg, occupant_manager_cell_to_position, occupant_manager_cell_to_world_cell,
    position_to_occupant_manager_cell, position_to_standard_city_cell, position_to_world_cell, query_interface,
    standard_city_cell_to_position, standard_city_cell_to_world_cell, world_cell_to_occupant_manager_cell,
    world_cell_to_position, world_cell_to_standard_city_cell, BBox, CellBox, Filter, Interface, OccupantHost,
    OccupantManager, Vec3, CLSID_OCCUPANT_MANAGER,
};
use fc4_core::stream::{ByteReader, ByteWriter, RefResolver, RefValue, RegisteredObject, SaveObject, SaveRegistry};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{boolean, field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, f32_of, i64_of, jf, ji, opt, str_of, u32_of, EventLog, R,
};

/// The class written for occupants in saved records.
const OCCUPANT_CLASS: u32 = 0x0CC0_0CC0;

#[derive(Clone, Debug, Default)]
struct Occ {
    bbox: BBox,
    holder: bool,
    occ_type: u32,
    flags: u32,
    om: Option<CellBox>,
    initialized: bool,
    /// Bit 1 of `+0x2E`: `Init` calls occupant slot 17 instead of posting
    /// Create.
    quiet: bool,
    /// `+0x30` is set: Move is sent synchronously.
    sync: bool,
}

#[derive(Clone)]
struct Record {
    bytes: Vec<u8>,
    entries: Vec<RegisteredObject>,
}

pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    mgr: RefCell<Option<Rc<OccupantManager>>>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    /// Caller filters: token → occupants excluded.
    filters: RefCell<BTreeMap<u64, BTreeSet<u64>>>,
    trace_locks: Cell<bool>,
    depth: Cell<i64>,
    /// Occupant references logged as events.
    trace_refs: Cell<bool>,
    records: RefCell<BTreeMap<String, Record>>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            me: RefCell::new(Weak::new()),
            events: EventLog::default(),
            mgr: RefCell::new(None),
            occs: RefCell::new(BTreeMap::new()),
            refs: RefCell::new(BTreeMap::new()),
            filters: RefCell::new(BTreeMap::new()),
            trace_locks: Cell::new(false),
            depth: Cell::new(0),
            trace_refs: Cell::new(false),
            records: RefCell::new(BTreeMap::new()),
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn mgr(&self) -> R<Rc<OccupantManager>> {
        self.mgr
            .borrow()
            .clone()
            .ok_or_else(|| "no occupant manager (om_create)".into())
    }

    fn new_mgr(&self) -> Rc<OccupantManager> {
        let m = Rc::new(OccupantManager::new(self.rc() as Rc<dyn OccupantHost>));
        *self.mgr.borrow_mut() = Some(m.clone());
        m
    }

    fn occ(&self, t: u64) -> Occ {
        self.occs.borrow().get(&t).cloned().unwrap_or_default()
    }

    fn with(&self, t: u64, f: impl FnOnce(&mut Occ)) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&t) {
            f(o);
        }
    }

    fn post(&self, occ: u64, msg_type: u32, data4: u32, sync: bool) {
        let o = self.occ(occ);
        self.events.push(
            "post",
            vec![
                ("type", ji(msg_type)),
                ("occupant", ji(occ as i64)),
                ("data2", Json::Bool(o.flags & 4 != 0)),
                ("data3", ji(o.occ_type)),
                ("data4", ji(data4)),
                ("sync", Json::Bool(sync)),
                ("inserted", Json::Bool(o.flags & 1 != 0)),
            ],
        );
    }

    fn count_ref(&self, t: u64, d: i64, kind: &str) {
        *self.refs.borrow_mut().entry(t).or_insert(0) += d;
        if self.trace_refs.get() {
            self.events.push(kind, vec![("occupant", ji(t as i64))]);
        }
    }
}

impl OccupantHost for World {
    fn lock(&self) {
        self.depth.set(self.depth.get() + 1);
        if self.trace_locks.get() {
            self.events.push("lock", vec![("depth", ji(self.depth.get()))]);
        }
    }
    fn unlock(&self) {
        if self.trace_locks.get() {
            self.events.push("unlock", vec![("depth", ji(self.depth.get()))]);
        }
        self.depth.set(self.depth.get() - 1);
    }
    fn add_ref(&self, occ: u64) {
        self.count_ref(occ, 1, "add_ref");
    }
    fn release(&self, occ: u64) {
        self.count_ref(occ, -1, "release");
    }
    fn init(&self, occ: u64) {
        let o = self.occ(occ);
        if o.initialized {
            return;
        }
        self.with(occ, |o| o.initialized = true);
        if o.quiet {
            self.events.push("occupant_slot17", vec![("occupant", ji(occ as i64))]);
        } else {
            self.post(occ, msg::CREATE, 0, false);
        }
    }
    fn shutdown(&self, occ: u64) {
        if self.occ(occ).initialized {
            self.with(occ, |o| o.initialized = false);
            self.post(occ, msg::DESTROY, 0, false);
        }
    }
    fn has_property_holder(&self, occ: u64) -> bool {
        self.occ(occ).holder
    }
    fn occupant_type(&self, occ: u64) -> u32 {
        self.occ(occ).occ_type
    }
    fn bounding_box(&self, occ: u64) -> BBox {
        self.occ(occ).bbox
    }
    fn post_message(&self, occ: u64, msg_type: u32, data4: u32) {
        let sync = msg_type == msg::MOVE && self.occ(occ).sync;
        self.post(occ, msg_type, data4, sync);
    }
    fn manager_bbox(&self, occ: u64) -> Option<CellBox> {
        self.occ(occ).om
    }
    fn set_manager_bbox(&self, occ: u64, b: CellBox) {
        self.with(occ, |o| o.om = Some(b));
    }
    fn set_flag(&self, occ: u64, flag: u32) {
        self.with(occ, |o| o.flags |= flag);
    }
    fn clear_flag(&self, occ: u64, flag: u32) {
        self.with(occ, |o| o.flags &= !flag);
    }
    fn is_flag_set(&self, occ: u64, flag: u32) -> bool {
        self.occ(occ).flags & flag != 0
    }
    fn filter_includes(&self, filter: u64, occ: u64) -> bool {
        !self.filters.borrow().get(&filter).is_some_and(|x| x.contains(&occ))
    }
    fn object_ref(&self, occ: u64) -> RefValue {
        RefValue::Host {
            token: occ,
            class_id: OCCUPANT_CLASS,
        }
    }
    /// A known occupant resolves to its token; anything else does not, and
    /// the reference the stream handed over is dropped.
    fn resolve_ref(&self, r: &RefValue, _iid: u32) -> Option<u64> {
        let RefValue::Host { token, .. } = *r else {
            return None;
        };
        if token != 0 && self.occs.borrow().contains_key(&token) {
            return Some(token);
        }
        *self.refs.borrow_mut().entry(token).or_insert(0) -= 1;
        None
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "occupant_manager", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("om_")?;
    let w = world(host);
    Some(match rest {
        "create" => {
            w.new_mgr();
            Ok(())
        }
        "occupant" => occupant(&w, op),
        "filter" => filter_def(&w, op),
        "trace" => trace(&w, op),
        "call" => call(&w, op),
        "expect" => expect(&w, op),
        "expect_cell" => expect_cell(&w, op),
        "expect_occupant" => expect_occupant(&w, op),
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

// ---- setup --------------------------------------------------------------------------------------------

fn token_of(j: &Json, what: &str) -> R<u64> {
    Ok(i64_of(j, what)? as u64)
}

fn vec3_of(j: &Json, what: &str) -> R<Vec3> {
    let a = arr(j, what)?;
    if a.len() != 3 {
        return Err(format!("'{what}' must have 3 numbers"));
    }
    Ok([f32_of(&a[0], what)?, f32_of(&a[1], what)?, f32_of(&a[2], what)?])
}

fn pair_of(j: &Json, what: &str) -> R<(i32, i32)> {
    let a = arr(j, what)?;
    if a.len() != 2 {
        return Err(format!("'{what}' must have 2 integers"));
    }
    Ok((i64_of(&a[0], what)? as i32, i64_of(&a[1], what)? as i32))
}

fn int3_of(j: &Json, what: &str) -> R<[i32; 3]> {
    let a = arr(j, what)?;
    if a.len() != 3 {
        return Err(format!("'{what}' must have 3 integers"));
    }
    Ok([
        i64_of(&a[0], what)? as i32,
        i64_of(&a[1], what)? as i32,
        i64_of(&a[2], what)? as i32,
    ])
}

fn cell_box_of_json(j: &Json) -> R<Option<CellBox>> {
    if *j == Json::Null {
        return Ok(None);
    }
    let a = arr(j, "om_box")?;
    if a.len() != 4 {
        return Err("'om_box' must have 4 integers".into());
    }
    let b = |i: usize| u32_of(&a[i], "om_box").map(|v| v as u8);
    Ok(Some(CellBox::new(b(0)?, b(1)?, b(2)?, b(3)?)))
}

fn cell_box_json(b: Option<CellBox>) -> Json {
    b.map_or(Json::Null, |b| {
        Json::Array(vec![ji(b.x0), ji(b.z0), ji(b.x1), ji(b.z1)])
    })
}

/// `om_occupant`: creates (or updates) `occupant` with any of `type`
/// (default 0x100), `min`, `max` (the bounding box, x y z), `holder`
/// (default true), `flags`, `om_box` ([x0, z0, x1, z1] or null),
/// `initialized`, `quiet`, `sync`.
fn occupant(w: &World, op: &Json) -> R<()> {
    let t = token_of(field(op, "occupant")?, "occupant")?;
    let mut o = w.occs.borrow().get(&t).cloned().unwrap_or(Occ {
        holder: true,
        occ_type: 0x100,
        ..Occ::default()
    });
    if let Some(v) = op.get("type") {
        o.occ_type = u32_of(v, "type")?;
    }
    if let Some(v) = opt(op, "min") {
        o.bbox.min = vec3_of(v, "min")?;
    }
    if let Some(v) = opt(op, "max") {
        o.bbox.max = vec3_of(v, "max")?;
    }
    if let Some(v) = opt(op, "holder") {
        o.holder = bool_of(v, "holder")?;
    }
    if let Some(v) = opt(op, "flags") {
        o.flags = u32_of(v, "flags")?;
    }
    if let Some(v) = op.get("om_box") {
        o.om = cell_box_of_json(v)?;
    }
    if let Some(v) = opt(op, "initialized") {
        o.initialized = bool_of(v, "initialized")?;
    }
    if let Some(v) = opt(op, "quiet") {
        o.quiet = bool_of(v, "quiet")?;
    }
    if let Some(v) = opt(op, "sync") {
        o.sync = bool_of(v, "sync")?;
    }
    w.occs.borrow_mut().insert(t, o);
    Ok(())
}

/// `om_filter`: a caller filter `filter` (a token) that excludes `exclude`.
fn filter_def(w: &World, op: &Json) -> R<()> {
    let t = token_of(field(op, "filter")?, "filter")?;
    let ex = arr(field(op, "exclude")?, "exclude")?
        .iter()
        .map(|e| token_of(e, "exclude"))
        .collect::<R<BTreeSet<u64>>>()?;
    w.filters.borrow_mut().insert(t, ex);
    Ok(())
}

/// `om_trace`: `locks` (lock / unlock events) and `refs` (add_ref /
/// release events).
fn trace(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "locks") {
        w.trace_locks.set(bool_of(v, "locks")?);
    }
    if let Some(v) = opt(op, "refs") {
        w.trace_refs.set(bool_of(v, "refs")?);
    }
    Ok(())
}

// ---- calls --------------------------------------------------------------------------------------------

/// A filter argument: null (all), a token (a caller filter) or
/// `{"type": n}` (the shared one-type filter).
fn filter_of(op: &Json) -> R<Filter> {
    match op.get("filter") {
        None | Some(Json::Null) => Ok(Filter::All),
        Some(Json::Object(_)) => Ok(Filter::Type(u32_of(field(op.get("filter").unwrap(), "type")?, "type")?)),
        Some(v) => Ok(Filter::Host(token_of(v, "filter")?)),
    }
}

fn opt_pair(op: &Json, key: &str) -> R<Option<(i32, i32)>> {
    opt(op, key).map(|v| pair_of(v, key)).transpose()
}

fn opt_vec3(op: &Json, key: &str) -> R<Option<Vec3>> {
    opt(op, key).map(|v| vec3_of(v, key)).transpose()
}

fn opt_int3(op: &Json, key: &str) -> R<Option<[i32; 3]>> {
    opt(op, key).map(|v| int3_of(v, key)).transpose()
}

fn type_of(op: &Json) -> R<u32> {
    u32_of(field(op, "type")?, "type")
}

fn pair_json(p: (i32, i32)) -> Json {
    Json::Array(vec![ji(p.0), ji(p.1)])
}

fn fpair_json(p: (f32, f32)) -> Json {
    Json::Array(vec![jf(p.0), jf(p.1)])
}

fn tokens_json(v: &[u64]) -> Json {
    Json::Array(v.iter().map(|&t| ji(t as i64)).collect())
}

fn args_f2(op: &Json) -> R<(f32, f32)> {
    Ok((f32_of(arg(op, 0)?, "arg")?, f32_of(arg(op, 1)?, "arg")?))
}

fn args_i2(op: &Json) -> R<(i32, i32)> {
    Ok((i64_of(arg(op, 0)?, "arg")? as i32, i64_of(arg(op, 1)?, "arg")? as i32))
}

/// The list of a list query: `list` (the tokens already in it, default
/// empty); the result is checked against `expect_list`, and the elements
/// appended are released afterwards unless `keep` is true.
fn list_query(w: &World, op: &Json, run: impl FnOnce(&OccupantManager, &mut Vec<u64>, i32) -> bool) -> R<Json> {
    let m = w.mgr()?;
    let mut list: Vec<u64> = match opt(op, "list") {
        Some(v) => arr(v, "list")?.iter().map(|e| token_of(e, "list")).collect::<R<_>>()?,
        None => Vec::new(),
    };
    let start = list.len();
    let max_count = opt(op, "max_count").map_or(Ok(-1), |v| i64_of(v, "max_count"))? as i32;
    let ok = run(&m, &mut list, max_count);
    if let Some(want) = op.get("expect_list") {
        let got = tokens_json(&list);
        if !scalar::matches(&got, want, false) {
            return Err(format!("list {}, expected {}", scalar::show(&got), scalar::show(want)));
        }
    }
    if !opt(op, "keep").and_then(boolean).unwrap_or(false) {
        let mut appended = list.split_off(start);
        m.release_occupant_list(&mut appended);
    }
    Ok(Json::Bool(ok))
}

/// A first-occupant result: the token (or null); the caller's reference is
/// released afterwards unless `keep` is true.
fn first(w: &World, op: &Json, r: Option<u64>) -> Json {
    if let Some(t) = r {
        if !opt(op, "keep").and_then(boolean).unwrap_or(false) {
            w.release(t);
        }
    }
    r.map_or(Json::Null, |t| ji(t as i64))
}

/// The iteration callback: logs `callback` (`occupant`), runs the
/// `actions` given for that occupant (`{"0xocc": [call ops]}`, inside the
/// callback), and returns false for the occupants in `stop_at`.
fn callback<'a>(w: &'a World, op: &'a Json) -> R<impl FnMut(u64) -> bool + 'a> {
    let stop: BTreeSet<u64> = match opt(op, "stop_at") {
        Some(v) => arr(v, "stop_at")?
            .iter()
            .map(|e| token_of(e, "stop_at"))
            .collect::<R<_>>()?,
        None => BTreeSet::new(),
    };
    let actions: BTreeMap<u64, Vec<Json>> = match opt(op, "actions") {
        Some(v) => v
            .as_object()
            .ok_or("actions must be an object")?
            .iter()
            .map(|(k, ops)| {
                let t = Json::Str(k.clone()).as_i64().ok_or(format!("bad occupant '{k}'"))? as u64;
                Ok((t, arr(ops, "actions")?.to_vec()))
            })
            .collect::<R<_>>()?,
        None => BTreeMap::new(),
    };
    Ok(move |o: u64| {
        w.events.push(
            "callback",
            vec![("occupant", ji(o as i64)), ("depth", ji(w.depth.get()))],
        );
        if let Some(ops) = actions.get(&o) {
            for a in ops {
                if let Err(e) = call(w, a) {
                    w.events.push("action_error", vec![("error", Json::Str(e))]);
                }
            }
        }
        !stop.contains(&o)
    })
}

/// `om_call`: `method` with its arguments; `expect` checks the result.
fn call(w: &World, op: &Json) -> R<()> {
    let method = str_of(field(op, "method")?, "method")?;
    let got = match method {
        // ---- conversions (§3) ----
        "position_to_world_cell" => {
            let (x, z) = args_f2(op)?;
            pair_json(position_to_world_cell(x, z))
        }
        "position_to_manager_cell" => {
            let (x, z) = args_f2(op)?;
            pair_json(position_to_occupant_manager_cell(x, z))
        }
        "position_to_standard_cell" => {
            let (x, z) = args_f2(op)?;
            pair_json(position_to_standard_city_cell(x, z))
        }
        "world_to_manager_cell" => {
            let (x, z) = args_i2(op)?;
            pair_json(world_cell_to_occupant_manager_cell(x, z))
        }
        "manager_to_world_cell" => {
            let (x, z) = args_i2(op)?;
            pair_json(occupant_manager_cell_to_world_cell(x, z))
        }
        "world_to_standard_cell" => {
            let (x, z) = args_i2(op)?;
            let out_z = i64_of(arg(op, 2)?, "out_z")? as i32;
            pair_json(world_cell_to_standard_city_cell(x, z, out_z))
        }
        "standard_to_world_cell" => {
            let (x, z) = args_i2(op)?;
            pair_json(standard_city_cell_to_world_cell(x, z))
        }
        "world_cell_to_position" => {
            let (x, z) = args_i2(op)?;
            fpair_json(world_cell_to_position(x, z))
        }
        "manager_cell_to_position" => {
            let (x, z) = args_i2(op)?;
            fpair_json(occupant_manager_cell_to_position(x, z))
        }
        "standard_cell_to_position" => {
            let (x, z) = args_i2(op)?;
            fpair_json(standard_city_cell_to_position(x, z))
        }
        "bounding_cells" => {
            let (x, z) = args_i2(op)?;
            Json::Array(bounding_cells(x, z).into_iter().map(pair_json).collect())
        }
        "query_interface" => {
            let iid = u32_of(arg(op, 0)?, "iid")?;
            match query_interface(iid) {
                None => Json::Null,
                Some(Interface::OccupantManager) => ji(0),
                Some(Interface::Serializable) => ji(4),
                Some(Interface::MessageTarget2) => ji(8),
            }
        }
        // ---- lifecycle (§2) ----
        "set_city_size" => {
            let (x, z) = args_f2(op)?;
            Json::Bool(w.mgr()?.set_city_size(x, z))
        }
        "init" => Json::Bool(w.mgr()?.init()),
        "shutdown" => Json::Bool(w.mgr()?.shutdown()),
        "add_ref" => ji(w.mgr()?.add_ref()),
        "release" => ji(w.mgr()?.release()),
        "class_id" => ji(w.mgr().map_or(CLSID_OCCUPANT_MANAGER, |m| m.class_id())),
        "cell_sizes" => fpair_json(w.mgr()?.cell_sizes()),
        "world_cell_count" => pair_json(w.mgr()?.world_cell_count()),
        "cell_count" => pair_json(w.mgr()?.cell_count()),
        "cell_bounds" => {
            let (a, b, c, d) = w.mgr()?.cell_bounds();
            Json::Array(vec![ji(a), ji(b), ji(c), ji(d)])
        }
        // ---- insert, remove, move (§5) ----
        "insert" => {
            let o = token_of(field(op, "occupant")?, "occupant")?;
            let d = opt(op, "data4").map_or(Ok(0), |v| u32_of(v, "data4"))?;
            Json::Bool(w.mgr()?.insert(o, d))
        }
        "remove" => {
            let o = token_of(field(op, "occupant")?, "occupant")?;
            let s = opt(op, "shutdown").map_or(Ok(false), |v| bool_of(v, "shutdown"))?;
            let d = opt(op, "data4").map_or(Ok(0), |v| u32_of(v, "data4"))?;
            Json::Bool(w.mgr()?.remove(o, s, d))
        }
        "remove_occupants" | "remove_occupants_typed" => {
            let s = opt(op, "shutdown").map_or(Ok(false), |v| bool_of(v, "shutdown"))?;
            let d = opt(op, "data4").map_or(Ok(0), |v| u32_of(v, "data4"))?;
            let m = w.mgr()?;
            Json::Bool(if method == "remove_occupants" {
                m.remove_occupants(filter_of(op)?, s, d)
            } else {
                m.remove_occupants_of_type(type_of(op)?, s, d)
            })
        }
        "move" => {
            let o = token_of(field(op, "occupant")?, "occupant")?;
            let p = opt(op, "post").map_or(Ok(true), |v| bool_of(v, "post"))?;
            Json::Bool(w.mgr()?.move_occupant(o, p))
        }
        // ---- cell helpers (§6) ----
        "is_cell_empty" => {
            let (x, z) = args_i2(op)?;
            Json::Bool(w.mgr()?.is_cell_empty(x, z))
        }
        "find_occupant" => {
            let o = token_of(field(op, "occupant")?, "occupant")?;
            let all = opt(op, "search_all").map_or(Ok(false), |v| bool_of(v, "search_all"))?;
            let (mut x, mut z) = opt_pair(op, "xz")?.unwrap_or((0, 0));
            let r = w.mgr()?.find_occupant(o, all, &mut x, &mut z);
            if let Some(want) = op.get("expect_xz") {
                let got = pair_json((x, z));
                if !scalar::matches(&got, want, false) {
                    return Err(format!(
                        "x, z = {}, expected {}",
                        scalar::show(&got),
                        scalar::show(want)
                    ));
                }
            }
            Json::Bool(r)
        }
        // ---- first-occupant queries (§7) ----
        "first_by_position" => {
            let (x, z) = args_f2(op)?;
            let r = w.mgr()?.first_by_position(x, z, filter_of(op)?);
            first(w, op, r)
        }
        "first_by_position_typed" => {
            let (x, z) = args_f2(op)?;
            let r = w.mgr()?.first_by_position_typed(x, z, type_of(op)?);
            first(w, op, r)
        }
        "first_in_cell" => {
            let (x, z) = args_i2(op)?;
            let r = w.mgr()?.first_in_cell(x, z, filter_of(op)?);
            first(w, op, r)
        }
        "first_in_cell_typed" => {
            let (x, z) = args_i2(op)?;
            let r = w.mgr()?.first_in_cell_typed(x, z, type_of(op)?);
            first(w, op, r)
        }
        "first_by_standard_cell" => {
            let (x, z) = args_i2(op)?;
            let r = w.mgr()?.first_by_standard_cell(x, z, filter_of(op)?);
            first(w, op, r)
        }
        "first_by_standard_cell_typed" => {
            let (x, z) = args_i2(op)?;
            let r = w.mgr()?.first_by_standard_cell_typed(x, z, type_of(op)?);
            first(w, op, r)
        }
        "first_by_standard_cells" => {
            let r = w
                .mgr()?
                .first_by_standard_cells(opt_pair(op, "min")?, opt_pair(op, "max")?, filter_of(op)?);
            first(w, op, r)
        }
        "first_by_standard_cells_typed" => {
            let r = w
                .mgr()?
                .first_by_standard_cells_typed(opt_pair(op, "min")?, opt_pair(op, "max")?, type_of(op)?);
            first(w, op, r)
        }
        // ---- list queries (§8) ----
        "by_bbox" => {
            let (a, b, f) = (opt_vec3(op, "min")?, opt_vec3(op, "max")?, filter_of(op)?);
            list_query(w, op, |m, l, n| m.occupants_by_bbox(l, a, b, f, n))?
        }
        "by_bbox_typed" => {
            let (a, b, t) = (opt_vec3(op, "min")?, opt_vec3(op, "max")?, type_of(op)?);
            list_query(w, op, |m, l, n| m.occupants_by_bbox_typed(l, a, b, t, n))?
        }
        "by_cells" => {
            let (a, b, f) = (opt_pair(op, "min")?, opt_pair(op, "max")?, filter_of(op)?);
            list_query(w, op, |m, l, n| m.occupants_by_cells(l, a, b, f, n))?
        }
        "by_cells_typed" => {
            let (a, b, t) = (opt_pair(op, "min")?, opt_pair(op, "max")?, type_of(op)?);
            list_query(w, op, |m, l, n| m.occupants_by_cells_typed(l, a, b, t, n))?
        }
        "by_standard_cells" => {
            let (a, b, f) = (opt_pair(op, "min")?, opt_pair(op, "max")?, filter_of(op)?);
            list_query(w, op, |m, l, n| m.occupants_by_standard_cells(l, a, b, f, n))?
        }
        "by_standard_cells_typed" => {
            let (a, b, t) = (opt_pair(op, "min")?, opt_pair(op, "max")?, type_of(op)?);
            list_query(w, op, |m, l, n| m.occupants_by_standard_cells_typed(l, a, b, t, n))?
        }
        // ---- iteration (§9) ----
        "iterate_by_bbox" | "iterate_by_bbox_typed" => {
            let a = vec3_of(field(op, "min")?, "min")?;
            let b = vec3_of(field(op, "max")?, "max")?;
            let m = w.mgr()?;
            let mut f = callback(w, op)?;
            Json::Bool(if method == "iterate_by_bbox" {
                m.iterate_by_bbox(&mut f, a, b, filter_of(op)?)
            } else {
                m.iterate_by_bbox_typed(&mut f, a, b, type_of(op)?)
            })
        }
        "iterate" | "iterate_typed" => {
            let (a, b) = (opt_int3(op, "min")?, opt_int3(op, "max")?);
            let m = w.mgr()?;
            let mut f = callback(w, op)?;
            Json::Bool(if method == "iterate" {
                m.iterate(&mut f, a, b, filter_of(op)?)
            } else {
                m.iterate_typed(&mut f, a, b, type_of(op)?)
            })
        }
        "iterate_by_standard_cells" | "iterate_by_standard_cells_typed" => {
            let (a, b) = (opt_pair(op, "min")?, opt_pair(op, "max")?);
            let m = w.mgr()?;
            let mut f = callback(w, op)?;
            Json::Bool(if method == "iterate_by_standard_cells" {
                m.iterate_by_standard_cells(&mut f, a, b, filter_of(op)?)
            } else {
                m.iterate_by_standard_cells_typed(&mut f, a, b, type_of(op)?)
            })
        }
        other => return Err(format!("unknown occupant manager method '{other}'")),
    };
    check_expect(op, got)
}

// ---- checks -------------------------------------------------------------------------------------------

/// `om_expect`: `state` (partial: `initialized`, `was_read`, `ref_count`,
/// `destroyed`, `size`, `counts`, `filter_type` (null = no filter), `rows`,
/// `holders`, `nodes`).
fn expect(w: &World, op: &Json) -> R<()> {
    let s = w.mgr()?.snapshot();
    let got = vec![
        ("initialized".to_owned(), Json::Bool(s.initialized)),
        ("was_read".to_owned(), Json::Bool(s.was_read)),
        ("ref_count".to_owned(), ji(s.ref_count)),
        ("destroyed".to_owned(), Json::Bool(s.destroyed)),
        ("size".to_owned(), fpair_json(s.size)),
        (
            "counts".to_owned(),
            Json::Array(s.counts.iter().map(|&c| ji(c)).collect()),
        ),
        ("filter_type".to_owned(), s.filter_type.map_or(Json::Null, ji)),
        ("rows".to_owned(), ji(s.rows as i64)),
        ("holders".to_owned(), ji(s.holders as i64)),
        ("nodes".to_owned(), ji(s.nodes as i64)),
    ];
    check_state(&got, field(op, "state")?)
}

/// `om_expect_cell`: `cell` ([x, z], world cells): `occupants` (front
/// first), `single` (the bit-0 values), or `null: true` (no holder).
fn expect_cell(w: &World, op: &Json) -> R<()> {
    let (x, z) = pair_of(field(op, "cell")?, "cell")?;
    let c = w.mgr()?.cell(x, z);
    if opt(op, "null").and_then(boolean).unwrap_or(false) {
        return match c {
            None => Ok(()),
            Some(l) => Err(format!("cell ({x}, {z}) has a holder ({} nodes)", l.len())),
        };
    }
    let l = c.ok_or_else(|| format!("cell ({x}, {z}) is null"))?;
    if let Some(want) = op.get("occupants") {
        let got = tokens_json(&l.iter().map(|n| n.occupant).collect::<Vec<_>>());
        if !scalar::matches(&got, want, false) {
            return Err(format!(
                "cell ({x}, {z}): {}, expected {}",
                scalar::show(&got),
                scalar::show(want)
            ));
        }
    }
    if let Some(want) = op.get("single") {
        let got = Json::Array(l.iter().map(|n| Json::Bool(n.single)).collect());
        if !scalar::matches(&got, want, false) {
            return Err(format!(
                "cell ({x}, {z}) bits: {}, expected {}",
                scalar::show(&got),
                scalar::show(want)
            ));
        }
    }
    Ok(())
}

/// `om_expect_occupant`: `occupant`, `state` (partial: `flags`, `om_box`,
/// `initialized`).
fn expect_occupant(w: &World, op: &Json) -> R<()> {
    let t = token_of(field(op, "occupant")?, "occupant")?;
    let o = w.occ(t);
    let got = vec![
        ("flags".to_owned(), ji(o.flags)),
        ("om_box".to_owned(), cell_box_json(o.om)),
        ("initialized".to_owned(), Json::Bool(o.initialized)),
    ];
    check_state(&got, field(op, "state")?)
}

/// `om_expect_refs`: `{"token": references}` taken (net) on occupants.
fn expect_refs(w: &World, op: &Json) -> R<()> {
    for (k, v) in field(op, "refs")?.as_object().ok_or("refs must be an object")? {
        let t = Json::Str(k.clone()).as_i64().ok_or_else(|| format!("bad key '{k}'"))? as u64;
        let got = w.refs.borrow().get(&t).copied().unwrap_or(0);
        if got != i64_of(v, k)? {
            return Err(format!("{k}: {got} references"));
        }
    }
    Ok(())
}

// ---- the record (§13) ---------------------------------------------------------------------------------

/// Resolves references like the class-object store: a host object comes
/// back as its registered token with one reference handed to the reader.
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

/// `om_write`: the record into `slot`; `expect`, `length`.
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
    let entries = reg.borrow().entries.clone();
    if let Some(slot) = opt(op, "slot") {
        w.records
            .borrow_mut()
            .insert(str_of(slot, "slot")?.to_string(), Record { bytes, entries });
    }
    Ok(())
}

/// `om_read`: a slot's record, optionally with another `version` or cut to
/// `length`, into a new constructed manager (or the current one with
/// `reuse`).
fn read(w: &World, op: &Json) -> R<()> {
    let slot = str_of(field(op, "slot")?, "slot")?;
    let mut rec = w
        .records
        .borrow()
        .get(slot)
        .cloned()
        .ok_or_else(|| format!("no record in slot '{slot}'"))?;
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

/// `om_expect_record`: `same_as` (another slot), `length`, `at` (`[{offset, hex}]`), `u32s`
/// (`[{offset, values}]`: little-endian u32 values from `offset`).
fn expect_record(w: &World, op: &Json) -> R<()> {
    let slot = str_of(field(op, "slot")?, "slot")?;
    let recs = w.records.borrow();
    let rec = recs.get(slot).ok_or_else(|| format!("no record in slot '{slot}'"))?;
    if let Some(v) = opt(op, "same_as") {
        let other = str_of(v, "same_as")?;
        let o = recs.get(other).ok_or_else(|| format!("no record in slot '{other}'"))?;
        if o.bytes != rec.bytes {
            return Err(format!("records '{slot}' and '{other}' differ"));
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
    if let Some(at) = opt(op, "u32s") {
        for e in arr(at, "u32s")? {
            let off = i64_of(field(e, "offset")?, "offset")? as usize;
            for (i, v) in arr(field(e, "values")?, "values")?.iter().enumerate() {
                let p = off + 4 * i;
                let b = rec.bytes.get(p..p + 4).ok_or("beyond the record")?;
                let got = u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
                if got != u32_of(v, "values")? {
                    return Err(format!("u32 at {p}: {got}"));
                }
            }
        }
    }
    Ok(())
}
