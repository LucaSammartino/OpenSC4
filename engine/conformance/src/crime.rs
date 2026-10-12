// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/crime.md`: a scripted city (zones, lots,
//! occupants, props, residential grids, demand, police, ordinances), a
//! scriptable crime generator, and ops that drive the crime simulator. See
//! README.md ("Crime ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::rc::{Rc, Weak};

use fc4_core::crime::{
    census_bucket, choose_candidate, decay_value, jobless_factor, msg, parse_mask, rhe, shipped_tuning,
    shipped_type_exemplars, BuildingInfo, CrimeEvent, CrimeHost, CrimeMessage, CrimeRandom, CrimeShared, CrimeSim,
    DemandValues, GridProvider, Incident, LotBits, LotRef, NearbyOccupant, NearestKind, RzRandomSuper, TractGrid,
    CLSID_CRIME_SIMULATOR,
};
use fc4_core::grid::{create_for_clsid, CityDims, SimGrid};
use fc4_core::property::PropertyHolder;
use fc4_core::rng::FixedSeeds;
use fc4_core::store::{save_store, Package};
use fc4_core::stream::{ByteReader, ByteWriter, OutStream, RefResolver, RefValue, ResKey, SaveObject, SaveRegistry};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, f32_of, i64_of, jf, ji, opt, str_of, u32_of, EventLog, Saved, R,
};

/// The class ID the fake world gives its lots (a stand-in; in game the
/// lots are the game's own objects).
const LOT_CLASS: u32 = 0x1234_5678;
/// The class ID of `cSC4SimGrid<uint8>` (simgrid §2).
const GRID_U8_CLASS: u32 = 0x49B9_E602;

/// A scripted generator: queued values, each queue repeating its last
/// value (defaults: uniform 0.5, range u 0.5, uint 0). `range` returns
/// `lo + u·(hi − lo)`; `uint` returns the queued value.
struct Stub {
    world: Weak<World>,
    uniform: VecDeque<f64>,
    range: VecDeque<f64>,
    uint: VecDeque<u32>,
    last: (f64, f64, u32),
}

impl Stub {
    fn log(&self, fields: Vec<(&str, Json)>) {
        if let Some(w) = self.world.upgrade() {
            w.events.push("rng", fields);
        }
    }
}

impl CrimeRandom for Stub {
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
    fn uniform_u32(&mut self, n: u32) -> u32 {
        let v = self.uint.pop_front().unwrap_or(self.last.2);
        self.last.2 = v;
        self.log(vec![("kind", Json::Str("uint".into())), ("n", ji(n)), ("value", ji(v))]);
        v
    }
}

#[derive(Clone, Default)]
struct Lot {
    rect: [i32; 4],
    location: Option<(i32, i32)>,
    condition: u8,
    building: Option<BuildingInfo>,
    center: (i32, i32),
}

#[derive(Clone, Default)]
struct Occupant {
    preview: bool,
    effect: Option<Vec<u8>>,
    pos: [f32; 3],
}

/// A save plus the host objects (lots) its registry assigned IDs to.
#[derive(Clone)]
struct CrimeSaved {
    saved: Saved,
    hosts: BTreeMap<(u32, u32), u64>,
}

pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    sim: RefCell<Option<Rc<CrimeSim>>>,
    shared: RefCell<Option<Rc<CrimeShared>>>,
    next_token: Cell<u64>,
    saves: RefCell<BTreeMap<String, CrimeSaved>>,
    city: Cell<CityDims>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    types: RefCell<Vec<(u32, Rc<PropertyHolder>)>>,
    ltext: RefCell<BTreeMap<(u32, u32, u32), String>>,
    pops: RefCell<[Option<Rc<SimGrid<u16>>>; 3]>,
    eqs: RefCell<[Option<Rc<SimGrid<f32>>>; 3]>,
    demand: RefCell<BTreeMap<u32, DemandValues>>,
    police: RefCell<Option<Rc<SimGrid<u8>>>>,
    jail: Cell<u32>,
    released: Cell<i32>,
    ordinance: Cell<Option<f32>>,
    disaster_layer: Cell<bool>,
    lots: RefCell<BTreeMap<u64, Lot>>,
    random: RefCell<VecDeque<Option<u64>>>,
    zones: RefCell<BTreeMap<(i32, i32), u32>>,
    occupants: RefCell<BTreeMap<u64, Occupant>>,
    nearby: RefCell<Vec<NearbyOccupant>>,
    prop_ids: RefCell<BTreeMap<u32, u32>>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            me: RefCell::new(Weak::new()),
            events: EventLog::default(),
            sim: RefCell::new(None),
            shared: RefCell::new(None),
            next_token: Cell::new(0x100),
            saves: RefCell::new(BTreeMap::new()),
            city: Cell::new(CityDims::square(64)),
            tuning: RefCell::new(Some(Rc::new(shipped_tuning()))),
            types: RefCell::new(shipped_types()),
            ltext: RefCell::new(BTreeMap::new()),
            pops: RefCell::new([None, None, None]),
            eqs: RefCell::new([None, None, None]),
            demand: RefCell::new(BTreeMap::new()),
            police: RefCell::new(None),
            jail: Cell::new(0),
            released: Cell::new(0),
            ordinance: Cell::new(Some(1.0)),
            disaster_layer: Cell::new(true),
            lots: RefCell::new(BTreeMap::new()),
            random: RefCell::new(VecDeque::new()),
            zones: RefCell::new(BTreeMap::new()),
            occupants: RefCell::new(BTreeMap::new()),
            nearby: RefCell::new(Vec::new()),
            prop_ids: RefCell::new(BTreeMap::new()),
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn sim(&self) -> R<Rc<CrimeSim>> {
        self.sim
            .borrow()
            .clone()
            .ok_or_else(|| "no crime simulator (crime_create)".into())
    }

    fn token(&self) -> u64 {
        let t = self.next_token.get();
        self.next_token.set(t + 1);
        t
    }

    fn lot_ref(&self, token: u64) -> LotRef {
        LotRef {
            token,
            class_id: LOT_CLASS,
        }
    }

    /// `rng`: `{seed}` (a real `cRZRandomSuper`) or `{stub: {uniform, range,
    /// uint}}`; none → an empty stub.
    fn rng(&self, spec: Option<&Json>) -> R<Box<dyn CrimeRandom>> {
        if let Some(s) = spec.and_then(|s| opt(s, "seed")) {
            let seeds = FixedSeeds {
                wall_clock: 1,
                process_clock: 1,
            };
            return Ok(Box::new(RzRandomSuper::new(u32_of(s, "seed")?, &seeds)));
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
        let uint = match stub.and_then(|s| opt(s, "uint")) {
            None => VecDeque::new(),
            Some(v) => arr(v, "uint")?.iter().map(|x| u32_of(x, "uint")).collect::<R<_>>()?,
        };
        Ok(Box::new(Stub {
            world: self.me.borrow().clone(),
            uniform: floats("uniform")?,
            range: floats("range")?,
            uint,
            last: (0.5, 0.5, 0),
        }))
    }

    fn effect_of(&self, kind: &str) -> R<Rc<dyn TractGrid<u8>>> {
        let s = self.sim()?;
        match kind {
            "crime" => s.crime_grid(),
            "effect" => s.effect_grid(),
            other => return Err(format!("unknown grid '{other}' (crime, effect)")),
        }
        .ok_or_else(|| format!("the simulator has no {kind} grid"))
    }
}

fn shipped_types() -> Vec<(u32, Rc<PropertyHolder>)> {
    shipped_type_exemplars()
        .into_iter()
        .map(|(i, h)| (i, Rc::new(h)))
        .collect()
}

/// The instance ID of a core grid (0 for other grids).
fn instance_of(g: &dyn TractGrid<u8>) -> u32 {
    g.save_ref().object::<SimGrid<u8>>().map_or(0, |s| s.instance_id())
}

fn ids_json(ids: &[u32]) -> Json {
    Json::Array(ids.iter().map(|&i| ji(i)).collect())
}

impl GridProvider<u8> for World {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        self.events
            .push("create", vec![("instance", ji(instance)), ("size", ji(tract_size))]);
        let g = SimGrid::<u8>::create(&self.city.get(), instance, tract_size);
        g.fill(0xEE); // Init must clear it (§2)
        Some(g)
    }
}

impl CrimeHost for World {
    fn add_agent(&self, name: &str, levels: &[u32], flags: u32) {
        self.events.push(
            "agent",
            vec![
                ("name", Json::Str(name.into())),
                ("levels", ids_json(levels)),
                ("flags", ji(flags)),
            ],
        );
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
    fn acquire_services(&self) {
        self.events.push("acquire_services", vec![]);
    }
    fn release_services(&self) {
        self.events.push("release_services", vec![]);
    }
    fn add_prop_requester(&self) {
        self.events.push("add_prop_requester", vec![]);
    }
    fn shutdown_grid(&self, g: &Rc<dyn TractGrid<u8>>) {
        self.events
            .push("grid_shutdown", vec![("instance", ji(instance_of(&**g)))]);
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn crime_type_exemplars(&self) -> Vec<(u32, Rc<PropertyHolder>)> {
        self.events.push("load_types", vec![]);
        self.types.borrow().clone()
    }
    fn ltext(&self, k: ResKey) -> Option<Vec<u8>> {
        self.ltext
            .borrow()
            .get(&(k.type_id, k.group, k.instance))
            .map(|s| s.as_bytes().to_vec())
    }
    fn population_grids(&self) -> [Option<Rc<dyn TractGrid<u16>>>; 3] {
        self.pops
            .borrow()
            .clone()
            .map(|g| g.map(|g| g as Rc<dyn TractGrid<u16>>))
    }
    fn eq_grids(&self) -> [Option<Rc<dyn TractGrid<f32>>>; 3] {
        self.eqs
            .borrow()
            .clone()
            .map(|g| g.map(|g| g as Rc<dyn TractGrid<f32>>))
    }
    fn demand(&self, id: u32) -> Option<DemandValues> {
        self.demand.borrow().get(&id).copied()
    }
    fn police_grid(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        self.police.borrow().clone().map(|g| g as Rc<dyn TractGrid<u8>>)
    }
    fn jail_population(&self) -> u32 {
        self.events.push("jail_population", vec![]);
        self.jail.get()
    }
    fn reset_unjailed_criminals(&self) -> i32 {
        self.events.push("reset_unjailed", vec![]);
        self.released.get()
    }
    fn process_monthly_crime_stats(&self) {
        self.events.push("crime_stats", vec![]);
    }
    fn ordinance_crime_factor(&self) -> Option<f32> {
        self.ordinance.get()
    }
    fn random_lot(&self) -> Option<LotRef> {
        let t = self.random.borrow_mut().pop_front().flatten()?;
        self.lots.borrow().contains_key(&t).then(|| self.lot_ref(t))
    }
    fn lot_at(&self, x: i32, z: i32) -> Option<LotRef> {
        let lots = self.lots.borrow();
        let hit = lots
            .iter()
            .find(|(_, l)| x >= l.rect[0] && x <= l.rect[2] && z >= l.rect[1] && z <= l.rect[3])
            .map(|(t, _)| *t);
        hit.map(|t| self.lot_ref(t))
    }
    fn lot_bounding_rect(&self, lot: u64) -> [i32; 4] {
        self.lots.borrow().get(&lot).map_or([0; 4], |l| l.rect)
    }
    fn lot_location(&self, lot: u64) -> Option<(i32, i32)> {
        self.lots.borrow().get(&lot).and_then(|l| l.location)
    }
    fn lot_condition(&self, lot: u64) -> u8 {
        self.lots.borrow().get(&lot).map_or(0, |l| l.condition)
    }
    fn lot_building(&self, lot: u64) -> Option<BuildingInfo> {
        self.lots.borrow().get(&lot).and_then(|l| l.building)
    }
    fn lot_center_cell(&self, lot: u64) -> (i32, i32) {
        self.lots.borrow().get(&lot).map_or((0, 0), |l| l.center)
    }
    fn zone_type(&self, x: i32, z: i32) -> u32 {
        self.zones.borrow().get(&(x, z)).copied().unwrap_or(0)
    }
    fn add_ref(&self, t: u64) {
        self.events.push("add_ref", vec![("lot", ji(t as i64))]);
    }
    fn release(&self, t: u64) {
        self.events.push("release", vec![("lot", ji(t as i64))]);
    }
    fn occupant_has_property(&self, o: u64, id: u32) -> bool {
        id == fc4_core::crime::prop::PREVIEW_OCCUPANT && self.occupants.borrow().get(&o).is_some_and(|x| x.preview)
    }
    fn occupant_property_bytes(&self, o: u64, id: u32) -> Option<Vec<u8>> {
        if id != fc4_core::crime::prop::CRIME_EFFECT {
            return None;
        }
        self.occupants.borrow().get(&o).and_then(|x| x.effect.clone())
    }
    fn occupant_position(&self, o: u64) -> [f32; 3] {
        self.occupants.borrow().get(&o).map_or([0.0; 3], |x| x.pos)
    }
    fn occupants_in_box(&self, min: [f32; 3], max: [f32; 3], _kind: NearestKind) -> Vec<NearbyOccupant> {
        // The occupant manager's box test (x and z; y is unbounded).
        self.nearby
            .borrow()
            .iter()
            .filter(|o| {
                o.position[0] >= min[0] && o.position[0] <= max[0] && o.position[2] >= min[2] && o.position[2] <= max[2]
            })
            .copied()
            .collect()
    }
    fn prop_crime_incident_id(&self, p: u32) -> u32 {
        self.prop_ids.borrow().get(&p).copied().unwrap_or(0)
    }
    fn disaster_layer_present(&self) -> bool {
        self.disaster_layer.get()
    }
    fn ignite(&self, b: u64, cause: u32) {
        self.events
            .push("ignite", vec![("building", ji(b as i64)), ("cause", ji(cause))]);
    }
    fn post_crime_committed(&self, type_id: u32, lot: LotRef, name: &[u8]) {
        self.events.push(
            "post",
            vec![
                ("type", Json::Str("CrimeCommitted".into())),
                ("crime", ji(type_id)),
                ("lot", ji(lot.token as i64)),
                ("name", Json::Str(String::from_utf8_lossy(name).into_owned())),
            ],
        );
    }
    fn post_crime_map_updated(&self) {
        self.events
            .push("post", vec![("type", Json::Str("CrimeMapUpdated".into()))]);
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "crime", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("crime_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "residential" => residential(&w, op),
        "lots" => lots(&w, op),
        "occupants" => occupants(&w, op),
        "nearby" => nearby(&w, op),
        "props" => props(&w, op),
        "types" => types(&w, op),
        "tuning" => tuning(&w, op),
        "create" => create(&w, op),
        "rng" => w.rng(Some(op)).and_then(|r| {
            w.shared
                .borrow()
                .as_ref()
                .ok_or("no process state (crime_create)")?
                .set_random(r);
            Ok(())
        }),
        "init" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.init()))),
        "shutdown" => w.sim().and_then(|s| check_expect(op, Json::Bool(s.shutdown()))),
        "message" => message(&w, op),
        "call" => call(&w, op),
        "eval" => eval(op),
        "set" => set(&w, op),
        "expect" => expect(&w, op),
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

/// `{x, z, w?, d?}` → the cells of the rectangle.
fn cells_of(e: &Json) -> R<Vec<(i32, i32)>> {
    let x = i32_of(field(e, "x")?, "x")?;
    let z = i32_of(field(e, "z")?, "z")?;
    let w = opt(e, "w").map_or(Ok(1), |v| i32_of(v, "w"))?;
    let d = opt(e, "d").map_or(Ok(1), |v| i32_of(v, "d"))?;
    if !(0..=4096).contains(&w) || !(0..=4096).contains(&d) {
        return Err("rectangle too large".into());
    }
    Ok((x..x + w).flat_map(|i| (z..z + d).map(move |j| (i, j))).collect())
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
    if opt(op, "clear_zones").and_then(crate::runner::boolean).unwrap_or(false) {
        w.zones.borrow_mut().clear();
    }
    if let Some(v) = opt(op, "zones") {
        for e in arr(v, "zones")? {
            let t = u32_of(field(e, "v")?, "v")?;
            for c in cells_of(e)? {
                w.zones.borrow_mut().insert(c, t);
            }
        }
    }
    if let Some(v) = opt(op, "demand") {
        for (k, d) in v.as_object().ok_or("demand must be an object")? {
            let id = Json::Str(k.clone()).as_i64().ok_or("bad demand ID")? as u32;
            if matches!(d, Json::Null) {
                w.demand.borrow_mut().remove(&id);
                continue;
            }
            let a = arr(d, "demand")?;
            if a.len() != 3 {
                return Err("demand values are [value, max, min]".into());
            }
            w.demand.borrow_mut().insert(
                id,
                DemandValues {
                    value: f32_of(&a[0], "value")?,
                    max: f32_of(&a[1], "max")?,
                    min: f32_of(&a[2], "min")?,
                },
            );
        }
    }
    if let Some(v) = opt(op, "jail") {
        w.jail.set(u32_of(v, "jail")?);
    }
    if let Some(v) = opt(op, "released") {
        w.released.set(i32_of(v, "released")?);
    }
    if let Some(v) = op.get("ordinance") {
        w.ordinance.set(match v {
            Json::Null => None,
            v => Some(f32_of(v, "ordinance")?),
        });
    }
    if let Some(v) = opt(op, "disaster_layer") {
        w.disaster_layer.set(bool_of(v, "disaster_layer")?);
    }
    if let Some(v) = op.get("police") {
        *w.police.borrow_mut() = match v {
            Json::Null => None,
            v => {
                let size = opt(v, "size").map_or(Ok(2), |s| i32_of(s, "size"))?;
                let g = SimGrid::<u8>::create(&w.city.get(), 0, size);
                if let Some(f) = opt(v, "fill") {
                    g.fill(u32_of(f, "fill")? as u8);
                }
                if let Some(c) = opt(v, "cells") {
                    for (x, z, val) in triples(c, "cells")? {
                        g.set(x, z, val as u8);
                    }
                }
                Some(g)
            }
        };
    }
    if let Some(v) = opt(op, "ltext") {
        for e in arr(v, "ltext")? {
            let k = arr(field(e, "key")?, "key")?;
            if k.len() != 3 {
                return Err("ltext key is [type, group, instance]".into());
            }
            let key = (u32_of(&k[0], "key")?, u32_of(&k[1], "key")?, u32_of(&k[2], "key")?);
            w.ltext
                .borrow_mut()
                .insert(key, str_of(field(e, "text")?, "text")?.to_owned());
        }
    }
    Ok(())
}

/// `crime_residential`: the population (u16) and EQ (f32) grids of one
/// wealth (`wealth` 0..2), tract size `size` (default 2), or `none`.
fn residential(w: &World, op: &Json) -> R<()> {
    let wealth = u32_of(field(op, "wealth")?, "wealth")? as usize;
    if wealth > 2 {
        return Err("wealth is 0..2".into());
    }
    if opt(op, "none").and_then(crate::runner::boolean).unwrap_or(false) {
        w.pops.borrow_mut()[wealth] = None;
        w.eqs.borrow_mut()[wealth] = None;
        return Ok(());
    }
    let size = opt(op, "size").map_or(Ok(2), |s| i32_of(s, "size"))?;
    let pop = SimGrid::<u16>::create(&w.city.get(), 0, size);
    let eq = SimGrid::<f32>::create(&w.city.get(), 0, size);
    if let Some(v) = opt(op, "pop") {
        for (x, z, val) in triples(v, "pop")? {
            pop.set(x, z, val as u16);
        }
    }
    if let Some(v) = opt(op, "eq") {
        for (x, z, val) in triples(v, "eq")? {
            eq.set(x, z, val as f32);
        }
    }
    w.pops.borrow_mut()[wealth] = Some(pop);
    w.eqs.borrow_mut()[wealth] = Some(eq);
    Ok(())
}

/// `crime_lots`: `lots` (`[{id, rect, location?, condition?, building?,
/// center?}]`, added or replaced) and `random` (the `GetRandomLot` results
/// in order, ids or null; replaces the queue).
fn lots(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "lots") {
        for e in arr(v, "lots")? {
            let id = u32_of(field(e, "id")?, "id")? as u64;
            let r = arr(field(e, "rect")?, "rect")?;
            if r.len() != 4 {
                return Err("rect is [x1, z1, x2, z2]".into());
            }
            let rect = [
                i32_of(&r[0], "rect")?,
                i32_of(&r[1], "rect")?,
                i32_of(&r[2], "rect")?,
                i32_of(&r[3], "rect")?,
            ];
            let location = match e.get("location") {
                None => Some((rect[0], rect[1])),
                Some(Json::Null) => None,
                Some(v) => Some(pair(v, "location")?),
            };
            let building = match opt(e, "building") {
                None => None,
                Some(b) => Some(BuildingInfo {
                    token: opt(b, "token").map_or(Ok(id as u32 + 1000), |t| u32_of(t, "token"))? as u64,
                    purpose: i32_of(field(b, "purpose")?, "purpose")?,
                    wealth: i32_of(field(b, "wealth")?, "wealth")?,
                }),
            };
            let center = match opt(e, "center") {
                Some(c) => pair(c, "center")?,
                None => ((rect[0] + rect[2]) / 2, (rect[1] + rect[3]) / 2),
            };
            let condition = opt(e, "condition").map_or(Ok(0), |c| u32_of(c, "condition"))? as u8;
            w.lots.borrow_mut().insert(
                id,
                Lot {
                    rect,
                    location,
                    condition,
                    building,
                    center,
                },
            );
        }
    }
    if let Some(v) = op.get("random") {
        let mut q = VecDeque::new();
        if !matches!(v, Json::Null) {
            for e in arr(v, "random")? {
                q.push_back(match e {
                    Json::Null => None,
                    e => Some(u32_of(e, "random")? as u64),
                });
            }
        }
        *w.random.borrow_mut() = q;
    }
    Ok(())
}

/// `crime_occupants`: `[{id, preview?, effect? (u8 array), pos}]`.
fn occupants(w: &World, op: &Json) -> R<()> {
    for e in arr(field(op, "occupants")?, "occupants")? {
        let id = u32_of(field(e, "id")?, "id")? as u64;
        let effect = match opt(e, "effect") {
            None => None,
            Some(v) => Some(
                arr(v, "effect")?
                    .iter()
                    .map(|b| u32_of(b, "effect").map(|b| b as u8))
                    .collect::<R<Vec<u8>>>()?,
            ),
        };
        w.occupants.borrow_mut().insert(
            id,
            Occupant {
                preview: opt(e, "preview").map_or(Ok(false), |v| bool_of(v, "preview"))?,
                effect,
                pos: opt(e, "pos").map_or(Ok([0.0; 3]), |v| vec3(v, "pos"))?,
            },
        );
    }
    Ok(())
}

/// `crime_nearby`: the occupants the bounding-box query sees
/// (`[{token, type, group?, incident_property?, pos, iid?, state?}]`).
fn nearby(w: &World, op: &Json) -> R<()> {
    let mut v = Vec::new();
    for e in arr(field(op, "occupants")?, "occupants")? {
        let b = |k: &str, d: bool| opt(e, k).map_or(Ok(d), |x| bool_of(x, k));
        v.push(NearbyOccupant {
            token: u32_of(field(e, "token")?, "token")? as u64,
            occupant_type: u32_of(field(e, "type")?, "type")?,
            in_crime_group: b("group", false)?,
            has_incident_property: b("incident_property", false)?,
            position: vec3(field(e, "pos")?, "pos")?,
            answers_iid: b("iid", true)?,
            prop_state: opt(e, "state").map_or(Ok(0), |x| u32_of(x, "state"))?,
        });
    }
    *w.nearby.borrow_mut() = v;
    Ok(())
}

/// `crime_props`: `{"0xPROP": crimeIncidentID}`.
fn props(w: &World, op: &Json) -> R<()> {
    for (k, v) in field(op, "props")?.as_object().ok_or("props must be an object")? {
        let id = Json::Str(k.clone()).as_i64().ok_or("bad prop ID")? as u32;
        w.prop_ids.borrow_mut().insert(id, u32_of(v, "incident")?);
    }
    Ok(())
}

/// `crime_types`: `shipped` (default true: the 14 shipped types first) and
/// `types` (`[{id, props}]`, property maps, in list order).
fn types(w: &World, op: &Json) -> R<()> {
    let mut list = if opt(op, "shipped").map_or(Ok(true), |v| bool_of(v, "shipped"))? {
        shipped_types()
    } else {
        Vec::new()
    };
    if let Some(v) = opt(op, "types") {
        for e in arr(v, "types")? {
            let id = u32_of(field(e, "id")?, "id")?;
            let h = match opt(e, "props") {
                Some(p) => scalar::holder_of(p)?,
                None => PropertyHolder::new(),
            };
            list.push((id, Rc::new(h)));
        }
    }
    *w.types.borrow_mut() = list;
    Ok(())
}

/// `crime_tuning`: `shipped` (the §8 exemplar), `props` (merged over it
/// when `shipped`), or `none` (no exemplar). The default is `shipped`.
fn tuning(w: &World, op: &Json) -> R<()> {
    if opt(op, "none").and_then(crate::runner::boolean).unwrap_or(false) {
        *w.tuning.borrow_mut() = None;
        return Ok(());
    }
    let mut h = if opt(op, "shipped").and_then(crate::runner::boolean).unwrap_or(false) {
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
    *w.tuning.borrow_mut() = Some(Rc::new(h));
    Ok(())
}

/// `crime_create`: a new simulator. The process state (type list and
/// generator) is shared with earlier simulators of the vector unless
/// `new_process`; `rng` replaces the generator.
fn create(w: &World, op: &Json) -> R<()> {
    let new_process = opt(op, "new_process").map_or(Ok(false), |v| bool_of(v, "new_process"))?;
    let existing = w.shared.borrow().clone();
    let shared = match existing {
        Some(s) if !new_process => {
            if let Some(r) = opt(op, "rng") {
                s.set_random(w.rng(Some(r))?);
            }
            s
        }
        _ => CrimeShared::new(w.rng(opt(op, "rng"))?),
    };
    *w.shared.borrow_mut() = Some(shared.clone());
    *w.sim.borrow_mut() = Some(CrimeSim::new(w.rc() as Rc<dyn CrimeHost>, w.token(), shared));
    Ok(())
}

// ---- calls ------------------------------------------------------------------------------------------

fn message(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let t = match field(op, "type")?.as_str() {
        Some("SimNewDay") => msg::SIM_NEW_DAY,
        Some("SimNewMonth") => msg::SIM_NEW_MONTH,
        Some("SimIdle") => msg::SIM_IDLE,
        Some("OccupantInserted") => msg::OCCUPANT_INSERTED,
        Some("OccupantRemoved") => msg::OCCUPANT_REMOVED,
        Some("OrdinanceUpdated") => msg::ORDINANCE_UPDATED,
        Some("ExemplarChanged") => msg::EXEMPLAR_CHANGED,
        _ => u32_of(field(op, "type")?, "type")?,
    };
    let count = opt(op, "count").map_or(Ok(1), |v| i64_of(v, "count"))?;
    if !(0..=10_000).contains(&count) {
        return Err("count is 0..10000".into());
    }
    let m = CrimeMessage {
        msg_type: t,
        data1: opt(op, "data1").map_or(Ok(0), |v| u32_of(v, "data1"))?,
        data2: opt(op, "data2").map_or(Ok(0), |v| u32_of(v, "data2"))?,
        void1: opt(op, "occupant").map_or(Ok(0), |v| u32_of(v, "occupant"))? as u64,
    };
    for _ in 0..count {
        check_expect(op, Json::Bool(s.handle_message(m)))?;
    }
    Ok(())
}

fn a_i32(op: &Json, i: usize) -> R<i32> {
    i32_of(arg(op, i)?, "argument")
}

fn key_json(k: ResKey) -> Json {
    Json::Array(vec![ji(k.type_id), ji(k.group), ji(k.instance)])
}

fn key_of(j: &Json) -> R<ResKey> {
    let a = arr(j, "icon")?;
    if a.len() != 3 {
        return Err("icon is [type, group, instance]".into());
    }
    Ok(ResKey {
        type_id: u32_of(&a[0], "icon")?,
        group: u32_of(&a[1], "icon")?,
        instance: u32_of(&a[2], "icon")?,
    })
}

fn bits_json(b: &LotBits) -> Json {
    Json::Object(vec![
        ("zone".into(), ji(b.zone)),
        ("wealth".into(), ji(b.wealth)),
        ("purpose".into(), ji(b.purpose)),
        ("condition".into(), ji(b.condition)),
    ])
}

fn event_json(e: &CrimeEvent) -> Json {
    Json::Object(vec![
        ("cell".into(), Json::Array(vec![ji(e.cell_x), ji(e.cell_z)])),
        ("type".into(), ji(e.type_id)),
        ("visuals".into(), Json::Bool(e.visuals_created)),
        ("icon".into(), key_json(e.icon)),
    ])
}

fn call(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let m = str_of(field(op, "method")?, "method")?;
    let unit = |f: &dyn Fn()| {
        f();
        Json::Null
    };
    let got = match m {
        "do_idle" => unit(&|| s.do_idle()),
        "do_census" => unit(&|| s.do_census()),
        "calculate_economic_factors" => unit(&|| s.calculate_economic_factors()),
        "compute_criminal_base" => unit(&|| s.compute_criminal_base()),
        "compute_crimes_to_commit" => unit(&|| s.compute_crimes_to_commit()),
        "decay_crime_grid" => unit(&|| s.decay_crime_grid()),
        "compute_average_crime" => unit(&|| s.compute_average_crime()),
        "load_types" => unit(&|| s.load_types()),
        "read_tuning" => unit(&|| s.read_tuning()),
        "add_crimes" => ji(s.add_crimes(a_i32(op, 0)?)),
        "add_crime_event" => ji(s.add_crime_event(w.lot_ref(u32_of(arg(op, 0)?, "lot")? as u64))),
        "lot_bits" => bits_json(&s.lot_bits(u32_of(arg(op, 0)?, "lot")? as u64)),
        "average_crime" => ji(s.average_crime()),
        "monthly_crime_count" => ji(s.monthly_crime_count()),
        "crime_hotspot" => match s.crime_hotspot() {
            Some((v, x, z)) => Json::Array(vec![ji(v), ji(x), ji(z)]),
            None => Json::Bool(false),
        },
        "nearest_sim" | "nearest_prop" => {
            let kind = if m == "nearest_sim" {
                NearestKind::Sim
            } else {
                NearestKind::Prop
            };
            match s.nearest(kind, vec3(arg(op, 0)?, "pos")?, f32_of(arg(op, 1)?, "radius")?) {
                Some(t) => ji(t as i64),
                None => Json::Null,
            }
        }
        "incident_array" => {
            // args[0]: the caller vector's initial length (elements marked
            // with type 0xFFFFFFFF, to show which elements are left alone).
            let n = op
                .get("args")
                .and_then(|a| a.as_array())
                .and_then(|a| a.first())
                .map_or(Ok(0), |v| u32_of(v, "length"))?;
            let mark = CrimeEvent {
                type_id: u32::MAX,
                ..CrimeEvent::default()
            };
            let mut out = vec![mark; n.min(4096) as usize];
            let ok = s.crime_incident_array(&mut out);
            Json::Object(vec![
                ("result".into(), Json::Bool(ok)),
                ("events".into(), Json::Array(out.iter().map(event_json).collect())),
            ])
        }
        "set_visuals_created" => {
            Json::Bool(s.set_visuals_created(u32_of(arg(op, 0)?, "index")?, bool_of(arg(op, 1)?, "value")?))
        }
        "visuals_created" => Json::Bool(s.visuals_created(u32_of(arg(op, 0)?, "index")?)),
        "simulator_type" => ji(s.simulator_type()),
        "requested_share" => ji(s.requested_share(a_i32(op, 0)?, a_i32(op, 1)?, a_i32(op, 2)?, a_i32(op, 3)?)),
        "requester_satisfaction_id" => ji(s.requester_satisfaction_id()),
        "uses_custom_prop_selector" => Json::Bool(s.uses_custom_prop_selector()),
        "select_prop" => {
            let props: Vec<u32> = arr(arg(op, 0)?, "props")?
                .iter()
                .map(|p| u32_of(p, "prop"))
                .collect::<R<_>>()?;
            ji(s.select_prop(&props, a_i32(op, 1)?, a_i32(op, 2)?))
        }
        "curve" => {
            // args: [name, x] or [name, x, wealth] for eq / jobless.
            let name = str_of(arg(op, 0)?, "curve")?;
            let x = f32_of(arg(op, 1)?, "x")?;
            let wealth = || -> R<usize> {
                let v = u32_of(arg(op, 2)?, "wealth")? as usize;
                if v > 2 {
                    return Err("wealth is 0..2".into());
                }
                Ok(v)
            };
            let d = s.data();
            let t = &d.tuning;
            jf(match name {
                "eq" => t.eq_curves[wealth()?].eval(x),
                "jobless" => t.jobless_curves[wealth()?].eval(x),
                "crimes" => t.crimes_curve.eval(x),
                "attenuation" => t.attenuation_curve.eval(x),
                "prop_request" => t.prop_request_curve.eval(x),
                other => return Err(format!("unknown curve '{other}'")),
            })
        }
        other => return Err(format!("unknown crime method '{other}'")),
    };
    check_expect(op, got)
}

/// `crime_eval`: the pure functions of §6 and §7 (`fn`, `args`, `expect`).
fn eval(op: &Json) -> R<()> {
    let f = str_of(field(op, "fn")?, "fn")?;
    let got = match f {
        "bucket" => match census_bucket(f32_of(arg(op, 0)?, "eq")?) {
            Some(b) => ji(b as i64),
            None => Json::Null,
        },
        "jobless" => jf(jobless_factor(
            f32_of(arg(op, 0)?, "a")?,
            f32_of(arg(op, 1)?, "b")?,
            f32_of(arg(op, 2)?, "c")?,
        )),
        "decay" => ji(decay_value(
            u32_of(arg(op, 0)?, "v")? as u8,
            f32_of(arg(op, 1)?, "decay")?,
        )),
        "mask" => {
            let bytes = match arg(op, 0)? {
                Json::Null => None,
                v => Some(
                    arr(v, "mask")?
                        .iter()
                        .map(|b| u32_of(b, "mask").map(|b| b as u8))
                        .collect::<R<Vec<u8>>>()?,
                ),
            };
            ji(parse_mask(bytes.as_deref()))
        }
        "lot_bits" => {
            let building = match arg(op, 1)? {
                Json::Null => None,
                v => {
                    let (p, w) = pair(v, "building")?;
                    Some((p, w))
                }
            };
            bits_json(&LotBits::new(
                u32_of(arg(op, 0)?, "zone")?,
                building,
                u32_of(arg(op, 2)?, "condition")? as u8,
            ))
        }
        "choose" => {
            let weights: Vec<f32> = arr(arg(op, 0)?, "weights")?
                .iter()
                .map(|v| f32_of(v, "weight"))
                .collect::<R<_>>()?;
            let r = scalar::num(arg(op, 1)?).ok_or("r: number")?;
            match choose_candidate(&weights, r) {
                Some(i) => ji(i as i64),
                None => Json::Null,
            }
        }
        "rhe" => {
            // args: [x] with x a number (taken as float32) or a hex string of
            // float32 bits (for infinities and NaN).
            let x = match arg(op, 0)? {
                v @ Json::Str(_) => f32::from_bits(u32_of(v, "bits")?),
                v => f32_of(v, "x")?,
            };
            ji(rhe(x))
        }
        other => return Err(format!("unknown crime function '{other}'")),
    };
    check_expect(op, got)
}

fn incident_of(w: &World, e: &Json) -> R<Incident> {
    Ok(Incident {
        lot: match opt(e, "lot") {
            None => None,
            Some(l) => Some(w.lot_ref(u32_of(l, "lot")? as u64)),
        },
        type_id: u32_of(field(e, "type")?, "type")?,
        name: opt(e, "name")
            .map_or(Ok(""), |n| str_of(n, "name"))?
            .as_bytes()
            .to_vec(),
        icon: opt(e, "icon").map_or(Ok(ResKey::default()), key_of)?,
        visuals_created: opt(e, "visuals").map_or(Ok(false), |v| bool_of(v, "visuals"))?,
    })
}

fn incident_json(i: &Incident) -> Json {
    Json::Object(vec![
        ("lot".into(), i.lot.map_or(Json::Null, |l| ji(l.token as i64))),
        ("type".into(), ji(i.type_id)),
        ("name".into(), Json::Str(String::from_utf8_lossy(&i.name).into_owned())),
        ("icon".into(), key_json(i.icon)),
        ("visuals".into(), Json::Bool(i.visuals_created)),
    ])
}

/// `crime_set`: pokes state fields (test setup).
fn set(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let fields = field(op, "state")?.as_object().ok_or("state must be an object")?;
    // Incidents first (they need no simulator borrow).
    let incidents = match fields.iter().find(|(k, _)| k == "incidents") {
        Some((_, v)) => Some(
            arr(v, "incidents")?
                .iter()
                .map(|e| incident_of(w, e))
                .collect::<R<Vec<_>>>()?,
        ),
        None => None,
    };
    let mut d = s.data_mut();
    if let Some(i) = incidents {
        d.incidents = i;
    }
    for (k, v) in fields {
        match k.as_str() {
            "incidents" => {}
            "hotspot" => {
                let a = arr(v, k)?;
                if a.len() != 3 {
                    return Err("hotspot is [x, z, value]".into());
                }
                d.hotspot_x = i32_of(&a[0], k)?;
                d.hotspot_z = i32_of(&a[1], k)?;
                d.hotspot_value = i32_of(&a[2], k)?;
            }
            "average_crime" => d.average_crime = f32_of(v, k)?,
            "ordinance_factor" => d.ordinance_factor = f32_of(v, k)?,
            "monthly_crime_count" => d.monthly_crime_count = u32_of(v, k)?,
            "state" => d.state = u32_of(v, k)?,
            "criminal_base" => d.criminal_base = i32_of(v, k)?,
            "crimes_to_commit" => d.crimes_to_commit = i32_of(v, k)?,
            "loaded_from_save" => d.loaded_from_save = bool_of(v, k)?,
            "jobless" => {
                let a = arr(v, k)?;
                for (i, x) in a.iter().take(3).enumerate() {
                    d.jobless[i] = f32_of(x, k)?;
                }
            }
            "census" => {
                for (wi, row) in arr(v, k)?.iter().take(3).enumerate() {
                    for (b, x) in arr(row, k)?.iter().take(4).enumerate() {
                        d.census[wi][b] = u32_of(x, k)?;
                    }
                }
            }
            other => return Err(format!("crime_set: unknown field '{other}'")),
        }
    }
    Ok(())
}

fn expect(w: &World, op: &Json) -> R<()> {
    let s = w.sim()?;
    let types = s.shared().types();
    let d = s.data();
    let it = &d.iterator;
    let f = |k: &str, v: f32| (k.to_owned(), jf(v));
    let i = |k: &str, v: i64| (k.to_owned(), ji(v));
    let b = |k: &str, v: bool| (k.to_owned(), Json::Bool(v));
    let got = vec![
        b("initialized", d.initialized),
        b("loaded_from_save", d.loaded_from_save),
        b("has_message_server", d.has_message_server),
        b("has_crime_grid", d.crime_grid.is_some()),
        b("has_effect_grid", d.effect_grid.is_some()),
        i("state", d.state.into()),
        i("criminal_base", d.criminal_base.into()),
        i("crimes_to_commit", d.crimes_to_commit.into()),
        (
            "census".into(),
            Json::Array(
                d.census
                    .iter()
                    .map(|r| Json::Array(r.iter().map(|&v| ji(v)).collect()))
                    .collect(),
            ),
        ),
        (
            "jobless".into(),
            Json::Array(d.jobless.iter().map(|&v| jf(v)).collect()),
        ),
        f("average_crime", d.average_crime),
        i("average_crime_bits", d.average_crime.to_bits().into()),
        f("ordinance_factor", d.ordinance_factor),
        i("monthly_crime_count", d.monthly_crime_count.into()),
        (
            "hotspot".into(),
            Json::Array(vec![ji(d.hotspot_x), ji(d.hotspot_z), ji(d.hotspot_value)]),
        ),
        (
            "incidents".into(),
            Json::Array(d.incidents.iter().map(incident_json).collect()),
        ),
        (
            "iterator".into(),
            Json::Object(vec![
                ("state".into(), ji(it.state_raw)),
                ("mode".into(), ji(it.mode)),
                ("dirty".into(), ji(it.dirty)),
                (
                    "rect".into(),
                    Json::Array(vec![ji(it.x0), ji(it.z0), ji(it.x1), ji(it.z1)]),
                ),
                ("count".into(), ji(it.count)),
                ("table".into(), ji(it.table.len() as i64)),
                ("index".into(), ji(it.index)),
                ("current".into(), ji(it.current)),
                ("step".into(), ji(it.step)),
            ]),
        ),
        (
            "tuning".into(),
            Json::Object(vec![
                ("tract_size".into(), ji(d.tuning.tract_size)),
                ("decay".into(), jf(d.tuning.decay)),
                ("reoffend".into(), jf(d.tuning.reoffend)),
                (
                    "prop_request_empty".into(),
                    Json::Bool(d.tuning.prop_request_curve.is_empty()),
                ),
            ]),
        ),
        (
            "types".into(),
            match types {
                None => Json::Null,
                Some(t) => Json::Array(
                    t.iter()
                        .map(|t| {
                            Json::Object(vec![
                                ("id".into(), ji(t.id)),
                                ("name".into(), Json::Str(String::from_utf8_lossy(&t.name).into_owned())),
                                ("icon".into(), key_json(t.icon)),
                                ("zone".into(), ji(t.zone_mask)),
                                ("wealth".into(), ji(t.wealth_mask)),
                                ("purpose".into(), ji(t.purpose_mask)),
                                ("condition".into(), ji(t.condition_mask)),
                                ("severity".into(), ji(t.severity)),
                                ("arson".into(), Json::Bool(t.arson)),
                                ("weight".into(), jf(t.weight)),
                            ])
                        })
                        .collect(),
                ),
            },
        ),
    ];
    drop(d);
    let want = field(op, "state")?;
    // `iterator`, `tuning`, `types` and `incidents` are matched partially
    // (only the keys given); everything else exactly.
    let nested_keys = ["iterator", "tuning", "types", "incidents"];
    let (nested, flat): (Vec<_>, Vec<_>) = want
        .as_object()
        .ok_or("'state' must be an object")?
        .iter()
        .cloned()
        .partition(|(k, _)| nested_keys.contains(&k.as_str()));
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

/// `crime_grid_set`: `grid` (`crime`, `effect`), `fill?` (first), `cells?`
/// (`[[x, z, v]]`).
fn grid_set(w: &World, op: &Json) -> R<()> {
    let g = w.effect_of(str_of(field(op, "grid")?, "grid")?)?;
    if let Some(v) = opt(op, "fill") {
        g.fill(u32_of(v, "fill")? as u8);
    }
    if let Some(v) = opt(op, "cells") {
        for (x, z, val) in triples(v, "cells")? {
            g.set(x, z, val as u8);
        }
    }
    Ok(())
}

/// `crime_expect_grid`: `grid` and any of `counts` (`[x, z]`), `size` (the
/// tract size), `instance`, `cells` (`[[x, z, v]]`), `others` (the value of
/// every unlisted tract).
fn expect_grid(w: &World, op: &Json) -> R<()> {
    let name = str_of(field(op, "grid")?, "grid")?;
    let g = w.effect_of(name)?;
    let geo = g.geometry();
    if let Some(v) = opt(op, "counts") {
        let want = pair(v, "counts")?;
        if (geo.count_x, geo.count_z) != want {
            return Err(format!("{name}: counts ({}, {})", geo.count_x, geo.count_z));
        }
    }
    if let Some(v) = opt(op, "size") {
        if geo.tract_size != i32_of(v, "size")? {
            return Err(format!("{name}: tract size {}", geo.tract_size));
        }
    }
    if let Some(v) = opt(op, "instance") {
        if instance_of(&*g) != u32_of(v, "instance")? {
            return Err(format!("{name}: instance 0x{:08X}", instance_of(&*g)));
        }
    }
    let mut listed = std::collections::BTreeSet::new();
    if let Some(v) = opt(op, "cells") {
        for (x, z, want) in triples(v, "cells")? {
            listed.insert((x, z));
            let got = g.get(x, z);
            if f64::from(got) != want {
                return Err(format!("{name}[{x}][{z}] = {got}, expected {want}"));
            }
        }
    }
    if let Some(v) = opt(op, "others") {
        let want = u32_of(v, "others")? as u8;
        for x in 0..geo.count_x.clamp(0, 4096) {
            for z in 0..geo.count_z.clamp(0, 4096) {
                if !listed.contains(&(x, z)) && g.get(x, z) != want {
                    return Err(format!(
                        "{name}[{x}][{z}] = {}, expected {want} (unlisted)",
                        g.get(x, z)
                    ));
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

/// Saves the simulator through the class-object store (a root record
/// referencing it) and remembers the host objects (lots) by object ID.
fn save_now(w: &World) -> R<CrimeSaved> {
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
    Ok(CrimeSaved {
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

/// `crime_load`: creates the simulator (sharing the process state), the
/// grids (core `SimGrid<u8>`) and the lots (host objects, the tokens they
/// had when saved) through the store's factory, and makes the loaded
/// simulator current.
fn load(w: &World, op: &Json) -> R<()> {
    let name = slot(op)?;
    let saved = w.saves.borrow().get(&name).cloned().ok_or("no such slot")?;
    let me = w.rc();
    let shared = match w.shared.borrow().clone() {
        Some(s) => s,
        None => CrimeShared::new(w.rng(None)?),
    };
    *w.shared.borrow_mut() = Some(shared.clone());
    let hosts = saved.hosts.clone();
    let mut factory = |class: u32, id: u32| -> Option<RefValue> {
        match class {
            CLSID_CRIME_SIMULATOR => {
                Some(RefValue::Object(
                    CrimeSim::new(me.clone() as Rc<dyn CrimeHost>, me.token(), shared.clone()) as Rc<dyn SaveObject>,
                ))
            }
            GRID_U8_CLASS => create_for_clsid(class).map(RefValue::Object),
            LOT_CLASS => hosts
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
        .and_then(|r| r.object::<CrimeSim>())
        .ok_or("the root did not resolve to a crime simulator")?;
    *w.sim.borrow_mut() = Some(sim);
    Ok(())
}

/// Resolves references while `crime_read` reads one payload: grids become
/// fresh core grids, lots the tokens they had when saved.
struct PayloadResolver(BTreeMap<(u32, u32), u64>, BTreeMap<(u32, u32), RefValue>);

impl RefResolver for PayloadResolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        if let Some(v) = self.1.get(&(class_id, object_id)) {
            return Some(v.clone());
        }
        let v = match class_id {
            GRID_U8_CLASS => RefValue::Object(create_for_clsid(class_id)?),
            LOT_CLASS => RefValue::Host {
                token: *self.0.get(&(class_id, object_id))?,
                class_id,
            },
            _ => return None,
        };
        self.1.insert((class_id, object_id), v.clone());
        Some(v)
    }
}

/// `crime_read`: `Read` of the simulator's payload in `slot`, outside the
/// store, by a new simulator (sharing the process state) or, with `into:
/// "current"`, by the current one; checks the result (`expect`) and the
/// bytes consumed (`position`), then makes the reading simulator current.
fn read(w: &World, op: &Json) -> R<()> {
    let saved = w.saves.borrow().get(&slot(op)?).cloned().ok_or("no such slot")?;
    let payload = scalar::entry_payload(&saved.saved, CLSID_CRIME_SIMULATOR, 0)?;
    let shared = w.shared.borrow().clone().ok_or("no process state (crime_create)")?;
    let sim = match opt(op, "into").map(|v| str_of(v, "into")).transpose()? {
        None | Some("new") => CrimeSim::new(w.rc() as Rc<dyn CrimeHost>, w.token(), shared),
        Some("current") => w.sim()?,
        Some(other) => return Err(format!("into: '{other}' (new, current)")),
    };
    let mut res = PayloadResolver(saved.hosts.clone(), BTreeMap::new());
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
    let mut saves = w.saves.borrow_mut();
    let s = saves.get_mut(&slot(op)?).ok_or("no such slot")?;
    scalar::patch(&mut s.saved, CLSID_CRIME_SIMULATOR, op)
}

fn expect_payload(w: &World, op: &Json) -> R<()> {
    let saves = w.saves.borrow();
    let s = saves.get(&slot(op)?).ok_or("no such slot")?;
    let got = scalar::entry_payload(&s.saved, CLSID_CRIME_SIMULATOR, 0)?;
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
