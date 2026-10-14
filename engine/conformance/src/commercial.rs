// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/commercial-industrial.md`: a scripted
//! city (demand nodes, a building map, building purposes) and ops that drive
//! the commercial simulator and the industrial stub. See README.md
//! ("Commercial and industrial ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::{Rc, Weak};

use fc4_core::commercial::{
    msg, query_interface, CommercialHost, CommercialMessage, CommercialSim, IndustrialSim, CLSID_COMMERCIAL_SIMULATOR,
    CLSID_INDUSTRIAL_SIMULATOR, OCCUPANT_TYPE_BUILDING,
};
use fc4_core::grid::{CityDims, GridProvider, SimGrid, TractGrid};
use fc4_core::stream::{ByteReader, ByteWriter, RefResolver, RefValue, RegisteredObject, SaveObject, SaveRegistry};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{boolean, field, HostRef};
use crate::scalar::{self, arr, bool_of, check_expect, check_state, i64_of, ji, opt, str_of, u32_of, EventLog, R};

#[derive(Clone)]
struct Record {
    bytes: Vec<u8>,
    entries: Vec<RegisteredObject>,
}

pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    city: Cell<CityDims>,
    nodes: RefCell<BTreeSet<u32>>,
    demand: Cell<bool>,
    map: RefCell<Option<Rc<SimGrid<u8>>>>,
    /// (purpose, wealth) → building-map key; absent: `(purpose << 2) + wealth + 1`.
    keys: RefCell<BTreeMap<(u32, u32), u8>>,
    purposes: RefCell<BTreeMap<u64, u32>>,
    grids: RefCell<Vec<Rc<SimGrid<u8>>>>,
    sim: RefCell<Option<Rc<CommercialSim>>>,
    industrial: RefCell<Option<Rc<IndustrialSim>>>,
    records: RefCell<BTreeMap<String, Record>>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            me: RefCell::new(Weak::new()),
            events: EventLog::default(),
            city: Cell::new(CityDims::square(8)),
            nodes: RefCell::new([0x3000, 0x3800].into_iter().collect()),
            demand: Cell::new(true),
            map: RefCell::new(None),
            keys: RefCell::new(BTreeMap::new()),
            purposes: RefCell::new(BTreeMap::new()),
            grids: RefCell::new(Vec::new()),
            sim: RefCell::new(None),
            industrial: RefCell::new(None),
            records: RefCell::new(BTreeMap::new()),
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn sim(&self) -> R<Rc<CommercialSim>> {
        self.sim
            .borrow()
            .clone()
            .ok_or_else(|| "no commercial simulator (commercial_create)".into())
    }

    fn industrial(&self) -> R<Rc<IndustrialSim>> {
        self.industrial
            .borrow()
            .clone()
            .ok_or_else(|| "no industrial simulator (industrial_create)".into())
    }

    fn new_sim(&self) -> Rc<CommercialSim> {
        let s = CommercialSim::new(self.rc() as Rc<dyn CommercialHost>, 0xC0);
        *self.sim.borrow_mut() = Some(s.clone());
        s
    }

    /// The instance ID of a grid the world made or loaded (0 = unknown).
    fn instance_of(&self, g: &Rc<dyn TractGrid<u8>>) -> u32 {
        let p = Rc::as_ptr(g) as *const ();
        self.grids
            .borrow()
            .iter()
            .find(|c| Rc::as_ptr(c) as *const () == p)
            .map_or(0, |c| c.instance_id())
    }
}

impl GridProvider<u8> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        self.events.push(
            "create_grid",
            vec![("instance", ji(instance)), ("tract_size", ji(tract_size))],
        );
        let g = SimGrid::<u8>::create(&self.city.get(), instance, tract_size);
        self.grids.borrow_mut().push(g.clone());
        Some(g)
    }

    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u8>>> {
        let g = r.object::<SimGrid<u8>>()?;
        self.grids.borrow_mut().push(g.clone());
        Some(g)
    }
}

impl CommercialHost for World {
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
        self.events.push("acquire_message_server", vec![]);
        true
    }
    fn subscribe(&self, msg_type: u32) {
        self.events.push("subscribe", vec![("type", ji(msg_type))]);
    }
    fn unsubscribe(&self, msg_type: u32) {
        self.events.push("unsubscribe", vec![("type", ji(msg_type))]);
    }
    fn release_message_server(&self) {
        self.events.push("release_message_server", vec![]);
    }
    fn acquire_city(&self) -> bool {
        self.events.push("acquire_city", vec![]);
        true
    }
    fn release_city(&self) {
        self.events.push("release_city", vec![]);
    }
    fn acquire_bds(&self) -> bool {
        self.events.push("acquire_bds", vec![]);
        true
    }
    fn release_bds(&self) {
        self.events.push("release_bds", vec![]);
    }
    fn has_demand_simulator(&self) -> bool {
        self.demand.get()
    }
    fn demand_node(&self, demand_id: u32) -> Option<u64> {
        self.events.push("demand_node", vec![("id", ji(demand_id))]);
        self.nodes.borrow().contains(&demand_id).then_some(u64::from(demand_id))
    }
    fn release_demand_node(&self, node: u64) {
        self.events.push("release_demand_node", vec![("node", ji(node as i64))]);
    }
    fn shutdown_grid(&self, grid: &Rc<dyn TractGrid<u8>>) {
        self.events
            .push("shutdown_grid", vec![("instance", ji(self.instance_of(grid)))]);
    }
    fn building_map(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        self.map.borrow().clone().map(|m| m as Rc<dyn TractGrid<u8>>)
    }
    fn building_map_key(&self, purpose: u32, wealth: u32) -> u8 {
        match self.keys.borrow().get(&(purpose, wealth)) {
            Some(&k) => k,
            None => ((purpose << 2) + wealth + 1) as u8,
        }
    }
    fn building_purpose(&self, occupant: u64) -> Option<u32> {
        self.purposes.borrow().get(&occupant).copied()
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "commercial", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    if let Some(rest) = name.strip_prefix("industrial_") {
        let w = world(host);
        return Some(industrial(&w, rest, op));
    }
    let rest = name.strip_prefix("commercial_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "create" => {
            w.new_sim();
            Ok(())
        }
        "init" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.init()))),
        "shutdown" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.shutdown()))),
        "add_ref" => w.sim().and_then(|s| check_expect(op, ji(s.add_ref()))),
        "release" => w.sim().and_then(|s| check_expect(op, ji(s.release()))),
        "identity" => identity(op, query_interface, CLSID_COMMERCIAL_SIMULATOR, w.sim().map(|s| s.class_id())),
        "message" => message(&w, op),
        "idle" => idle(&w, op),
        "set" => set(&w, op),
        "expect" => expect(&w, op),
        "set_grid" => set_grid(&w, op),
        "expect_grid" => expect_grid(&w, op),
        "proximity_map" => proximity_map(&w, op),
        "expect_events" => w.events.expect_events(op),
        "expect_event_count" => w.events.expect_count(op),
        "clear_events" => {
            w.events.clear();
            Ok(())
        }
        "write" => write(&w, op, false),
        "read" => read(&w, op, false),
        "expect_record" => expect_record(&w, op),
        _ => Err(format!("unknown op '{name}'")),
    })
}

fn industrial(w: &World, rest: &str, op: &Json) -> R<()> {
    match rest {
        "create" => {
            *w.industrial.borrow_mut() = Some(IndustrialSim::new(0x1D));
            Ok(())
        }
        "init" => w.industrial().and_then(|s| check_expect(op, Json::Bool(s.init()))),
        "shutdown" => w.industrial().and_then(|s| check_expect(op, Json::Bool(s.shutdown()))),
        "message" => {
            let t = msg_type(field(op, "type")?)?;
            w.industrial().and_then(|s| check_expect(op, Json::Bool(s.do_message(t))))
        }
        "add_ref" => w.industrial().and_then(|s| check_expect(op, ji(s.add_ref()))),
        "release" => w.industrial().and_then(|s| check_expect(op, ji(s.release()))),
        "identity" => identity(
            op,
            IndustrialSim::query_interface,
            CLSID_INDUSTRIAL_SIMULATOR,
            w.industrial().map(|s| s.class_id()),
        ),
        "expect" => {
            let s = w.industrial()?.snapshot();
            let got = vec![
                ("initialized".to_owned(), Json::Bool(s.initialized)),
                ("ref_count".to_owned(), ji(s.ref_count)),
                ("destroyed".to_owned(), Json::Bool(s.destroyed)),
            ];
            check_state(&got, field(op, "state")?)
        }
        "write" => write(w, op, true),
        "read" => read(w, op, true),
        other => Err(format!("unknown op 'industrial_{other}'")),
    }
}

// ---- environment ----------------------------------------------------------------------------------

/// `commercial_env`: `cells` (a square city, default 8), `nodes` (the demand
/// node IDs the demand simulator answers, default both), `demand` (whether
/// the city has a demand simulator), `map` (`{"key", "cells": [[x, z], …]}`
/// or `{"fill", "cells": [{"x", "z", "v"}]}`, null = no building map),
/// `keys` (`[{"purpose", "wealth", "key"}]`), `purposes` (`{"token":
/// purpose}`: building occupants).
fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "cells") {
        w.city.set(CityDims::square(i64_of(v, "cells")? as i32));
    }
    if let Some(v) = opt(op, "nodes") {
        *w.nodes.borrow_mut() = arr(v, "nodes")?.iter().map(|x| u32_of(x, "node")).collect::<R<_>>()?;
    }
    if let Some(v) = opt(op, "demand") {
        w.demand.set(bool_of(v, "demand")?);
    }
    if let Some(v) = op.get("map") {
        *w.map.borrow_mut() = match v {
            Json::Null => None,
            m => {
                let g = SimGrid::<u8>::create(&w.city.get(), 0x1234, 1);
                if let Some(f) = opt(m, "fill") {
                    g.fill(u32_of(f, "fill")? as u8);
                }
                let key = opt(m, "key").map(|k| u32_of(k, "key")).transpose()?;
                for c in arr(field(m, "cells")?, "cells")? {
                    match (key, c) {
                        (Some(k), Json::Array(xz)) if xz.len() == 2 => {
                            g.set(i64_of(&xz[0], "x")? as i32, i64_of(&xz[1], "z")? as i32, k as u8)
                        }
                        _ => g.set(
                            i64_of(field(c, "x")?, "x")? as i32,
                            i64_of(field(c, "z")?, "z")? as i32,
                            u32_of(field(c, "v")?, "v")? as u8,
                        ),
                    }
                }
                Some(g)
            }
        };
    }
    if let Some(v) = opt(op, "keys") {
        for e in arr(v, "keys")? {
            w.keys.borrow_mut().insert(
                (u32_of(field(e, "purpose")?, "purpose")?, u32_of(field(e, "wealth")?, "wealth")?),
                u32_of(field(e, "key")?, "key")? as u8,
            );
        }
    }
    if let Some(v) = opt(op, "purposes") {
        for (k, p) in v.as_object().ok_or("purposes must be an object")? {
            let t = Json::Str(k.clone()).as_i64().ok_or("bad occupant token")? as u64;
            w.purposes.borrow_mut().insert(t, u32_of(p, "purpose")?);
        }
    }
    Ok(())
}

// ---- calls ------------------------------------------------------------------------------------------

/// `*_identity`: `iids` (`{"0xIID": bool}`) answered by QueryInterface and
/// `class` (GetGZCLSID).
fn identity(op: &Json, qi: fn(u32) -> bool, class: u32, live: R<u32>) -> R<()> {
    if let Some(v) = opt(op, "iids") {
        for (k, want) in v.as_object().ok_or("iids must be an object")? {
            let iid = Json::Str(k.clone()).as_i64().ok_or("bad IID")? as u32;
            if qi(iid) != bool_of(want, k)? {
                return Err(format!("QueryInterface({k}) = {}", qi(iid)));
            }
        }
    }
    if let Some(v) = opt(op, "class") {
        let want = u32_of(v, "class")?;
        let got = live.unwrap_or(class);
        if got != want {
            return Err(format!("class 0x{got:08X}, expected 0x{want:08X}"));
        }
    }
    Ok(())
}

fn msg_type(j: &Json) -> R<u32> {
    Ok(match j.as_str() {
        Some("SimBegin") => msg::SIM_BEGIN,
        Some("SimNewDay") => msg::SIM_NEW_DAY,
        Some("SimNewMonth") => msg::SIM_NEW_MONTH,
        Some("SimIdle") => msg::SIM_IDLE,
        Some("OccupantInserted") => msg::OCCUPANT_INSERTED,
        Some("OccupantRemoved") => msg::OCCUPANT_REMOVED,
        _ => u32_of(j, "type")?,
    })
}

/// `commercial_message`: `type`, `data3` (default the building occupant
/// type for the occupant messages), `occupant` (void1).
fn message(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let t = msg_type(field(op, "type")?)?;
    let occ_msg = matches!(t, msg::OCCUPANT_INSERTED | msg::OCCUPANT_REMOVED);
    let m = CommercialMessage {
        msg_type: t,
        data1: opt(op, "data1").map_or(Ok(0), |v| u32_of(v, "data1"))?,
        data2: opt(op, "data2").map_or(Ok(0), |v| u32_of(v, "data2"))?,
        data3: opt(op, "data3").map_or(Ok(if occ_msg { OCCUPANT_TYPE_BUILDING } else { 0 }), |v| {
            u32_of(v, "data3")
        })?,
        void1: opt(op, "occupant").map(|v| i64_of(v, "occupant").map(|t| t as u64)).transpose()?,
    };
    check_expect(op, Json::Bool(s.do_message(&m)))
}

/// `commercial_idle`: `count` DoIdle steps (default 1).
fn idle(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let n = opt(op, "count").map_or(Ok(1), |v| i64_of(v, "count"))?;
    for _ in 0..n {
        s.do_idle();
    }
    Ok(())
}

/// `commercial_set` (a test hook): `dirty` and / or `state`.
fn set(w: &World, op: &Json) -> R<()> {
    let dirty = opt(op, "dirty").map(|v| bool_of(v, "dirty")).transpose()?;
    let state = opt(op, "state").map(|v| i64_of(v, "state")).transpose()?;
    w.sim()?.with_state(|s| {
        if let Some(d) = dirty {
            s.dirty = d;
        }
        if let Some(v) = state {
            s.state = v as i32;
        }
    });
    Ok(())
}

fn expect(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?.snapshot();
    let inst = |g: &Option<Rc<dyn TractGrid<u8>>>| g.as_ref().map_or(Json::Null, |g| ji(w.instance_of(g)));
    let got = vec![
        ("initialized".to_owned(), Json::Bool(s.initialized)),
        ("dirty".to_owned(), Json::Bool(s.dirty)),
        ("state".to_owned(), ji(s.state)),
        ("flag".to_owned(), Json::Bool(s.flag())),
        ("ref_count".to_owned(), ji(s.ref_count)),
        ("destroyed".to_owned(), Json::Bool(s.destroyed)),
        ("city_held".to_owned(), Json::Bool(s.city_held)),
        ("bds_held".to_owned(), Json::Bool(s.bds_held)),
        ("message_server_held".to_owned(), Json::Bool(s.message_server_held)),
        (
            "nodes".to_owned(),
            Json::Array(
                s.demand_nodes
                    .iter()
                    .map(|n| n.map_or(Json::Null, |n| ji(n as i64)))
                    .collect(),
            ),
        ),
        ("co2".to_owned(), inst(&s.co2)),
        ("co3".to_owned(), inst(&s.co3)),
        (
            "tract_size".to_owned(),
            s.co2.as_ref().map_or(Json::Null, |g| ji(g.geometry().tract_size)),
        ),
    ];
    check_state(&got, field(op, "state")?)
}

fn grid(w: &World, op: &Json) -> R<Rc<dyn TractGrid<u8>>> {
    let s = w.sim()?.snapshot();
    let g = match str_of(field(op, "grid")?, "grid")? {
        "CO2" => s.co2,
        "CO3" => s.co3,
        other => return Err(format!("unknown grid '{other}' (CO2 or CO3)")),
    };
    g.ok_or_else(|| "that grid does not exist".into())
}

/// `commercial_set_grid`: `fill`, then `cells` (`[{"x", "z", "v"}]`).
fn set_grid(w: &World, op: &Json) -> R<()> {
    let g = grid(w, op)?;
    if let Some(f) = opt(op, "fill") {
        g.fill(u32_of(f, "fill")? as u8);
    }
    if let Some(c) = opt(op, "cells") {
        for e in arr(c, "cells")? {
            g.set(
                i64_of(field(e, "x")?, "x")? as i32,
                i64_of(field(e, "z")?, "z")? as i32,
                u32_of(field(e, "v")?, "v")? as u8,
            );
        }
    }
    Ok(())
}

/// `commercial_expect_grid`: `rows` (`rows[z][x]`, every cell), or `cells`
/// (`[{"x", "z", "v"}]`) with `rest` (every other cell) optional, or
/// `count` (`{"value": n}`: how many cells hold each value).
fn expect_grid(w: &World, op: &Json) -> R<()> {
    let g = grid(w, op)?;
    let (nx, nz) = (g.count_x(), g.count_z());
    let mut errors = Vec::new();
    if let Some(rows) = opt(op, "rows") {
        let rows = arr(rows, "rows")?;
        if rows.len() as i32 != nz {
            return Err(format!("{} rows for a grid {nz} deep", rows.len()));
        }
        for (z, row) in rows.iter().enumerate() {
            let row = arr(row, "row")?;
            if row.len() as i32 != nx {
                return Err(format!("row {z} has {} values for a grid {nx} wide", row.len()));
            }
            for (x, v) in row.iter().enumerate() {
                let got = g.get(x as i32, z as i32);
                if u32::from(got) != u32_of(v, "value")? {
                    errors.push(format!("({x}, {z}) = {got}"));
                }
            }
        }
    }
    let mut named = BTreeSet::new();
    if let Some(cells) = opt(op, "cells") {
        for e in arr(cells, "cells")? {
            let (x, z) = (i64_of(field(e, "x")?, "x")? as i32, i64_of(field(e, "z")?, "z")? as i32);
            named.insert((x, z));
            let got = g.get(x, z);
            if u32::from(got) != u32_of(field(e, "v")?, "v")? {
                errors.push(format!("({x}, {z}) = {got}"));
            }
        }
    }
    if let Some(r) = opt(op, "rest") {
        let want = u32_of(r, "rest")?;
        for x in 0..nx {
            for z in 0..nz {
                if !named.contains(&(x, z)) && u32::from(g.get(x, z)) != want {
                    errors.push(format!("({x}, {z}) = {} (rest)", g.get(x, z)));
                }
            }
        }
    }
    if let Some(c) = opt(op, "count") {
        for (k, n) in c.as_object().ok_or("count must be an object")? {
            let v = Json::Str(k.clone()).as_i64().ok_or("bad value")? as u8;
            let mut got = 0i64;
            for x in 0..nx {
                for z in 0..nz {
                    got += i64::from(g.get(x, z) == v);
                }
            }
            if got != i64_of(n, "count")? {
                errors.push(format!("{got} cells hold {v}"));
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        errors.truncate(12);
        Err(format!("grid differs: {}", errors.join("; ")))
    }
}

/// `commercial_proximity_map`: `developer` → `expect` "CO2", "CO3" or null.
fn proximity_map(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let d = u32_of(field(op, "developer")?, "developer")?;
    let snap = s.snapshot();
    let got = match s.proximity_map(d) {
        None => Json::Null,
        Some(g) => {
            let same = |o: &Option<Rc<dyn TractGrid<u8>>>| {
                o.as_ref()
                    .is_some_and(|o| Rc::as_ptr(o) as *const () == Rc::as_ptr(&g) as *const ())
            };
            if same(&snap.co2) {
                Json::Str("CO2".into())
            } else if same(&snap.co3) {
                Json::Str("CO3".into())
            } else {
                Json::Str("other".into())
            }
        }
    };
    check_expect(op, got)
}

// ---- the records (§8) -------------------------------------------------------------------------------

/// Resolves references to the grids registered while writing.
struct Resolver(Vec<RegisteredObject>);

impl RefResolver for Resolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        let e = self.0.iter().find(|e| e.0 == class_id && e.1 == object_id)?;
        e.2.clone().map(RefValue::Object)
    }
}

fn target(w: &World, industrial: bool) -> R<Rc<dyn SaveObject>> {
    Ok(if industrial {
        w.industrial()? as Rc<dyn SaveObject>
    } else {
        w.sim()? as Rc<dyn SaveObject>
    })
}

/// `*_write`: the record into `slot`; `expect` checks the result, `length`
/// the size, `hex` the bytes.
fn write(w: &World, op: &Json, industrial: bool) -> R<()> {
    let s = target(w, industrial)?;
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut out = ByteWriter::with_registry(reg.clone());
    check_expect(op, Json::Bool(s.write(&mut out)))?;
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

/// `*_read`: a slot's record (or `hex`), optionally with another `version`,
/// read into a new constructed object (or the current one with `reuse`).
fn read(w: &World, op: &Json, industrial: bool) -> R<()> {
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
    let reuse = opt(op, "reuse").and_then(boolean).unwrap_or(false);
    let s: Rc<dyn SaveObject> = match (industrial, reuse) {
        (_, true) => target(w, industrial)?,
        (true, false) => {
            let s = IndustrialSim::new(0x1D);
            *w.industrial.borrow_mut() = Some(s.clone());
            s
        }
        (false, false) => w.new_sim(),
    };
    let mut res = Resolver(rec.entries.clone());
    let ok = s.read(&mut ByteReader::with_resolver(&rec.bytes, &mut res));
    check_expect(op, Json::Bool(ok))
}

/// `commercial_expect_record`: `same_as` (another slot), `length`, `at`.
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
