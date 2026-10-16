// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/demolition.md`: a scripted city
//! (occupants with types, positions, locks, costs and pole connections;
//! lots with rectangles, states, zones and flags; the zone map, the
//! terrain, view filters, the tuning exemplars) and ops that drive the
//! demolition utility. See README.md ("Demolition ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::{Rc, Weak};

use fc4_core::demolition::{
    msg, occ_type, query_interface, shipped_building_development, shipped_thresholds, DemolitionHost,
    DemolitionMessage, DemolitionUtility, Orphans, Outcome, CLSID_DEMOLITION, OCCUPANT_FLAG_IN_MANAGER,
};
use fc4_core::grid::CellRect;
use fc4_core::property::PropertyHolder;
use fc4_core::stream::{ByteReader, ByteWriter, SaveObject};
use fc4_core::zones::CellRegion;

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{boolean, field, HostRef};
use crate::scalar::{
    self, arr, bool_of, check_expect, check_state, f32_of, holder_of, i64_of, jf, ji, opt, u32_of, EventLog, R,
};

const UTILITY_TOKEN: u64 = 0xD0;

#[derive(Clone, Debug)]
struct Occ {
    ty: u32,
    pos: (f32, f32, f32),
    bounds: (f32, f32, f32, f32),
    flags: u32,
    system_lock: bool,
    player_lock: bool,
    foundation: Option<u32>,
    net_flags: Option<u32>,
    net_types: Vec<u32>,
    pole: bool,
    network: bool,
    cost: i64,
    connections: Vec<u64>,
    default_ok: bool,
    realistic_ok: bool,
    props: BTreeMap<u32, bool>,
    groups: Vec<u32>,
    extent: Option<f32>,
}

#[derive(Clone, Debug)]
struct Lot {
    rect: CellRect,
    state: u32,
    building: Option<u64>,
    occs: Vec<u64>,
    demolishable: bool,
    zone: u32,
    water: Option<bool>,
    locked: bool,
    initialized: bool,
    facing: u32,
    condition: u32,
}

pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    util: RefCell<Option<Rc<DemolitionUtility>>>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    lots: RefCell<BTreeMap<u64, Lot>>,
    zones: RefCell<HashMap<(i32, i32), u32>>,
    alt: RefCell<HashMap<(i32, i32), f32>>,
    water: RefCell<HashSet<(i32, i32)>>,
    cells: Cell<(i32, i32)>,
    bds: Cell<bool>,
    flora: Cell<bool>,
    prop_mgr: Cell<bool>,
    view_filter: Cell<Option<u64>>,
    view_level: Cell<Option<u32>>,
    rejects: RefCell<HashMap<u64, HashSet<u64>>>,
    rejects_buildings: RefCell<HashSet<u64>>,
    thresholds: RefCell<Option<Rc<PropertyHolder>>>,
    bd: RefCell<Option<Rc<PropertyHolder>>>,
    next_lot: Cell<u64>,
    record: RefCell<Vec<u8>>,
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            me: RefCell::new(Weak::new()),
            events: EventLog::default(),
            util: RefCell::new(None),
            occs: RefCell::new(BTreeMap::new()),
            lots: RefCell::new(BTreeMap::new()),
            zones: RefCell::new(HashMap::new()),
            alt: RefCell::new(HashMap::new()),
            water: RefCell::new(HashSet::new()),
            cells: Cell::new((64, 64)),
            bds: Cell::new(true),
            flora: Cell::new(true),
            prop_mgr: Cell::new(true),
            view_filter: Cell::new(None),
            view_level: Cell::new(Some(1)),
            rejects: RefCell::new(HashMap::new()),
            rejects_buildings: RefCell::new(HashSet::new()),
            thresholds: RefCell::new(Some(Rc::new(shipped_thresholds()))),
            bd: RefCell::new(Some(Rc::new(shipped_building_development()))),
            next_lot: Cell::new(0x9000),
            record: RefCell::new(Vec::new()),
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn util(&self) -> R<Rc<DemolitionUtility>> {
        self.util
            .borrow()
            .clone()
            .ok_or_else(|| "no demolition utility (demolition_create)".into())
    }

    fn ev(&self, kind: &str, fields: Vec<(&str, Json)>) {
        self.events.push(kind, fields);
    }

    fn o<T>(&self, t: u64, f: impl FnOnce(&Occ) -> T) -> T {
        f(self.occs.borrow().get(&t).expect("known occupant"))
    }

    fn l<T>(&self, t: u64, f: impl FnOnce(&Lot) -> T) -> T {
        f(self.lots.borrow().get(&t).expect("known lot"))
    }
}

fn tok(t: u64) -> Json {
    ji(t as i64)
}

fn rect_json(r: CellRect) -> Json {
    Json::Array(vec![ji(r.x1), ji(r.z1), ji(r.x2), ji(r.z2)])
}

impl DemolitionHost for World {
    fn subscribe(&self, m: u32) {
        self.ev("subscribe", vec![("type", ji(m))]);
    }
    fn unsubscribe(&self, m: u32) {
        self.ev("unsubscribe", vec![("type", ji(m))]);
    }
    fn acquire_city(&self) -> bool {
        self.ev("acquire", vec![("service", Json::Str("city".into()))]);
        true
    }
    fn release_city(&self) {
        self.ev("release_service", vec![("service", Json::Str("city".into()))]);
    }
    fn city_cell_counts(&self) -> (i32, i32) {
        self.cells.get()
    }
    fn acquire_occupant_manager(&self) -> bool {
        self.ev("acquire", vec![("service", Json::Str("occupant_manager".into()))]);
        true
    }
    fn release_occupant_manager(&self) {
        self.ev(
            "release_service",
            vec![("service", Json::Str("occupant_manager".into()))],
        );
    }
    fn acquire_lot_developer(&self) -> bool {
        self.ev("acquire", vec![("service", Json::Str("lot_developer".into()))]);
        true
    }
    fn release_lot_developer(&self) {
        self.ev("release_service", vec![("service", Json::Str("lot_developer".into()))]);
    }
    fn acquire_terrain(&self) -> bool {
        self.ev("acquire", vec![("service", Json::Str("terrain".into()))]);
        true
    }
    fn release_terrain(&self) {
        self.ev("release_service", vec![("service", Json::Str("terrain".into()))]);
    }
    fn thresholds_exemplar(&self) -> Option<Rc<PropertyHolder>> {
        self.thresholds.borrow().clone()
    }
    fn building_development_exemplar(&self) -> Option<Rc<PropertyHolder>> {
        self.bd.borrow().clone()
    }
    fn release(&self, t: u64) {
        self.ev("release", vec![("token", tok(t))]);
    }
    fn acquire_view_filter(&self) -> Option<u64> {
        self.view_filter.get()
    }
    fn view_level(&self) -> Option<u32> {
        self.view_level.get()
    }
    fn filter_includes(&self, f: u64, o: u64) -> bool {
        !self.rejects.borrow().get(&f).is_some_and(|s| s.contains(&o))
    }
    fn filter_includes_type(&self, f: u64, t: u32) -> bool {
        !(t == occ_type::BUILDING && self.rejects_buildings.borrow().contains(&f))
    }
    fn default_filter_includes(&self, o: u64) -> bool {
        self.o(o, |o| o.default_ok)
    }
    fn realistic_filter_includes(&self, o: u64) -> bool {
        self.o(o, |o| o.realistic_ok)
    }
    fn occupant_type(&self, o: u64) -> u32 {
        self.o(o, |o| o.ty)
    }
    fn occupant_position(&self, o: u64) -> (f32, f32, f32) {
        self.o(o, |o| o.pos)
    }
    fn can_remove(&self, o: u64, level: u32) -> bool {
        self.o(o, |o| {
            if level & 1 != 0 {
                !o.system_lock
            } else if level & 2 != 0 {
                !o.player_lock
            } else {
                true
            }
        })
    }
    fn occupant_flag_set(&self, o: u64, f: u32) -> bool {
        self.o(o, |o| o.flags & f != 0)
    }
    fn occupant_flags(&self, o: u64) -> u32 {
        self.o(o, |o| o.flags)
    }
    fn is_occupant_group(&self, o: u64, g: u32) -> bool {
        self.o(o, |o| o.groups.contains(&g))
    }
    fn occupant_bool_property(&self, o: u64, id: u32) -> Option<bool> {
        self.o(o, |o| o.props.get(&id).copied())
    }
    fn occupant_bounds(&self, o: u64) -> (f32, f32, f32, f32) {
        self.o(o, |o| o.bounds)
    }
    fn foundation_slot9(&self, o: u64) -> Option<u32> {
        self.o(o, |o| o.foundation)
    }
    fn network_has_any_flag(&self, o: u64, mask: u32) -> Option<bool> {
        self.o(o, |o| o.net_flags.map(|f| f & mask != 0))
    }
    fn network_has_type(&self, o: u64, t: u32) -> bool {
        self.o(o, |o| o.net_types.contains(&t))
    }
    fn occupant_shutdown(&self, o: u64) {
        self.ev("shutdown", vec![("occupant", tok(o))]);
    }
    fn remove_occupant(&self, o: u64, b: bool) -> bool {
        {
            let mut occs = self.occs.borrow_mut();
            let x = occs.get_mut(&o).expect("known occupant");
            if x.flags & OCCUPANT_FLAG_IN_MANAGER == 0 {
                return false;
            }
            x.flags &= !OCCUPANT_FLAG_IN_MANAGER;
        }
        self.ev("remove", vec![("occupant", tok(o)), ("flag", Json::Bool(b))]);
        true
    }
    fn occupants_in_box(&self, x0: f32, z0: f32, x1: f32, z1: f32) -> Vec<u64> {
        self.occs
            .borrow()
            .iter()
            .filter(|(_, o)| o.bounds.0 <= x1 && o.bounds.2 >= x0 && o.bounds.1 <= z1 && o.bounds.3 >= z0)
            .map(|(&t, _)| t)
            .collect()
    }
    fn occupants_in_cell_rect(&self, r: CellRect) -> Vec<u64> {
        self.occupants_in_box(
            16.0 * r.x1 as f32,
            16.0 * r.z1 as f32,
            16.0 * (r.x2 + 1) as f32,
            16.0 * (r.z2 + 1) as f32,
        )
    }
    fn building_demolition_cost(&self, b: u64) -> Option<i64> {
        self.bds.get().then(|| self.o(b, |o| o.cost))
    }
    fn flora_demolition_cost(&self, o: u64) -> Option<i64> {
        self.flora.get().then(|| self.o(o, |o| o.cost))
    }
    fn is_network_occupant(&self, o: u64) -> bool {
        self.o(o, |o| o.network)
    }
    fn network_demolition_cost(&self, o: u64) -> i64 {
        self.o(o, |o| o.cost)
    }
    fn is_power_pole(&self, o: u64) -> bool {
        self.o(o, |o| o.pole)
    }
    fn pole_demolition_cost(&self, p: u64) -> i64 {
        self.o(p, |o| o.cost)
    }
    fn pole_connections(&self, p: u64) -> Vec<u64> {
        self.o(p, |o| o.connections.clone())
    }
    fn pole_connection_count(&self, p: u64) -> u32 {
        self.o(p, |o| o.connections.len() as u32)
    }
    fn pole_break_all_connections(&self, p: u64) {
        let conns = self.o(p, |o| o.connections.clone());
        {
            let mut occs = self.occs.borrow_mut();
            for c in conns {
                if let Some(x) = occs.get_mut(&c) {
                    x.connections.retain(|&y| y != p);
                }
            }
            occs.get_mut(&p).expect("known pole").connections.clear();
        }
        self.ev("break", vec![("pole", tok(p))]);
    }
    fn send_pre_demolish(&self, o: u64) {
        self.ev("pre_demolish", vec![("occupant", tok(o))]);
    }
    fn has_prop_manager(&self) -> bool {
        self.prop_mgr.get()
    }
    fn remove_prop(&self, o: u64) -> bool {
        self.ev("remove_prop", vec![("occupant", tok(o))]);
        true
    }
    fn list_append(&self, list: u64, o: u64) {
        self.ev("append", vec![("list", tok(list)), ("occupant", tok(o))]);
    }
    fn set_insert(&self, set: u64, o: u64) {
        self.ev("insert", vec![("set", tok(set)), ("occupant", tok(o))]);
    }
    fn queue_removals(&self, on: bool) {
        self.ev("queue", vec![("on", Json::Bool(on))]);
    }
    fn delete_occupant_connection(&self, o: u64) {
        self.ev("delete_connection", vec![("occupant", tok(o))]);
    }
    fn destroy_network_occupants(&self, list: &[u64]) {
        self.ev(
            "destroy",
            vec![("occupants", Json::Array(list.iter().map(|&t| tok(t)).collect()))],
        );
    }
    fn cell_corner_to_position(&self, x: i32, z: i32) -> (f32, f32) {
        (16.0 * x as f32, 16.0 * z as f32)
    }
    fn position_to_cell(&self, x: f32, z: f32) -> Option<(i32, i32)> {
        Some(((x / 16.0).floor() as i32, (z / 16.0).floor() as i32))
    }
    fn lot_at(&self, x: i32, z: i32) -> Option<u64> {
        self.lots
            .borrow()
            .iter()
            .find(|(_, l)| x >= l.rect.x1 && x <= l.rect.x2 && z >= l.rect.z1 && z <= l.rect.z2)
            .map(|(&t, _)| t)
    }
    fn lots_in_cell_rect(&self, r: CellRect) -> Vec<u64> {
        self.lots
            .borrow()
            .iter()
            .filter(|(_, l)| l.rect.x1 <= r.x2 && l.rect.x2 >= r.x1 && l.rect.z1 <= r.z2 && l.rect.z2 >= r.z1)
            .map(|(&t, _)| t)
            .collect()
    }
    fn lot_building(&self, l: u64) -> Option<u64> {
        self.l(l, |l| l.building)
    }
    fn lot_state(&self, l: u64) -> u32 {
        self.l(l, |l| l.state)
    }
    fn lot_demolishable(&self, l: u64) -> bool {
        self.l(l, |l| l.demolishable)
    }
    fn lot_occupants(&self, l: u64) -> Vec<u64> {
        self.l(l, |l| l.occs.clone())
    }
    fn lot_size(&self, l: u64) -> (u8, u8) {
        self.l(l, |l| {
            ((l.rect.x2 - l.rect.x1 + 1) as u8, (l.rect.z2 - l.rect.z1 + 1) as u8)
        })
    }
    fn lot_bounding_rect(&self, l: u64) -> CellRect {
        self.l(l, |l| l.rect)
    }
    fn lot_facing(&self, l: u64) -> u32 {
        self.l(l, |l| l.facing)
    }
    fn lot_zone_type(&self, l: u64) -> u32 {
        self.l(l, |l| l.zone)
    }
    fn lot_water_compatible(&self, l: u64) -> Option<bool> {
        self.l(l, |l| l.water)
    }
    fn lot_initialized(&self, l: u64) -> bool {
        self.l(l, |l| l.initialized)
    }
    fn lot_vertices_locked(&self, l: u64) -> bool {
        self.l(l, |l| l.locked)
    }
    fn lot_condition(&self, l: u64) -> u32 {
        self.l(l, |l| l.condition)
    }
    fn explode_lot(&self, l: u64, b: bool) {
        self.ev("explode", vec![("lot", tok(l)), ("flag", Json::Bool(b))]);
    }
    fn start_demolish_lot(&self, l: u64) {
        self.ev("start", vec![("lot", tok(l))]);
    }
    fn end_demolish_lot(&self, l: u64) {
        if let Some(lot) = self.lots.borrow_mut().get_mut(&l) {
            lot.state = 1;
            lot.building = None;
        }
        self.ev("end", vec![("lot", tok(l))]);
    }
    fn delete_lot(&self, l: u64) {
        self.lots.borrow_mut().remove(&l);
        self.ev("delete", vec![("lot", tok(l))]);
    }
    fn create_lot_if_possible(&self, x: i32, z: i32, facing: u32) -> bool {
        let zone = self.zone_at(x, z);
        let t = self.next_lot.get();
        self.next_lot.set(t + 1);
        self.lots.borrow_mut().insert(
            t,
            Lot {
                rect: CellRect::new(x, z, x, z),
                state: 1,
                building: None,
                occs: Vec::new(),
                demolishable: true,
                zone,
                water: Some(false),
                locked: false,
                initialized: true,
                facing,
                condition: 0,
            },
        );
        self.ev("create", vec![("x", ji(x)), ("z", ji(z)), ("facing", ji(facing))]);
        true
    }
    fn zone_at(&self, x: i32, z: i32) -> u32 {
        self.zones.borrow().get(&(x, z)).copied().unwrap_or(0)
    }
    fn dezone(&self, r: CellRect) {
        for x in r.x1..=r.x2 {
            for z in r.z1..=r.z2 {
                self.zones.borrow_mut().insert((x, z), 0);
            }
        }
        self.ev("dezone", vec![("rect", rect_json(r))]);
    }
    fn altitude_at_vertex(&self, x: i32, z: i32) -> f32 {
        self.alt.borrow().get(&(x, z)).copied().unwrap_or(0.0)
    }
    fn is_water_vertex(&self, x: i32, z: i32) -> bool {
        self.water.borrow().contains(&(x, z))
    }
    fn set_altitude_rect(&self, r: CellRect, h: f32, reason: u32) {
        self.ev(
            "set_rect",
            vec![("rect", rect_json(r)), ("height", jf(h)), ("reason", ji(reason))],
        );
    }
    fn set_altitude_vertex(&self, x: i32, z: i32, h: f32) {
        self.alt.borrow_mut().insert((x, z), h);
        self.ev("set_vertex", vec![("x", ji(x)), ("z", ji(z)), ("height", jf(h))]);
    }
    fn redisplay_terrain(&self, r: CellRect, reason: u32) {
        self.ev("redisplay", vec![("rect", rect_json(r)), ("reason", ji(reason))]);
    }
    fn effect_extent(&self, o: u64) -> Option<f32> {
        self.o(o, |o| o.extent)
    }
    fn play_occupant_sound(&self, o: u64) {
        self.ev("sound", vec![("occupant", tok(o))]);
    }
    fn start_occupant_effect(&self, o: u64, keys: &[u32], ray: Option<(i32, i32)>, grey: bool) {
        self.ev(
            "effect",
            vec![
                ("occupant", tok(o)),
                ("keys", Json::Array(keys.iter().map(|&k| ji(k)).collect())),
                ("ray", ray.map_or(Json::Null, |(x, y)| Json::Array(vec![ji(x), ji(y)]))),
                ("grey", Json::Bool(grey)),
            ],
        );
    }
    fn start_group_effect(&self, instance: u32, x: i32, z: i32) {
        self.ev("group", vec![("instance", ji(instance)), ("x", ji(x)), ("z", ji(z))]);
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "demolition", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("demolition_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "occupant" => occupant(&w, op),
        "lot" => lot(&w, op),
        "create" => {
            let u = DemolitionUtility::new(w.rc() as Rc<dyn DemolitionHost>, UTILITY_TOKEN);
            *w.util.borrow_mut() = Some(u);
            Ok(())
        }
        "init" => w.util().and_then(|u| check_expect(op, Json::Bool(u.init()))),
        "shutdown" => w.util().and_then(|u| check_expect(op, Json::Bool(u.shutdown()))),
        "add_ref" => w.util().and_then(|u| check_expect(op, ji(u.add_ref()))),
        "release" => w.util().and_then(|u| check_expect(op, ji(u.release()))),
        "identity" => identity(&w, op),
        "message" => message(&w, op),
        "lots" => demolish_lots(&w, op),
        "region" => demolish_region(&w, op),
        "demolish_occupant" => demolish_occupant(&w, op),
        "terrain" => terrain(&w, op),
        "expect" => expect(&w, op),
        "expect_lot" => expect_lot(&w, op),
        "expect_zones" => expect_zones(&w, op),
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

// ---- environment ----------------------------------------------------------------------------------

fn token_of(j: &Json, what: &str) -> R<u64> {
    Ok(i64_of(j, what)? as u64)
}

fn opt_token(op: &Json, key: &str) -> R<Option<u64>> {
    opt(op, key).map(|v| token_of(v, key)).transpose()
}

fn tokens(j: &Json, what: &str) -> R<Vec<u64>> {
    arr(j, what)?.iter().map(|v| token_of(v, what)).collect()
}

fn ints(j: &Json, what: &str) -> R<Vec<i32>> {
    arr(j, what)?.iter().map(|v| Ok(i64_of(v, what)? as i32)).collect()
}

fn rect_of(j: &Json, what: &str) -> R<CellRect> {
    let v = ints(j, what)?;
    if v.len() != 4 {
        return Err(format!("'{what}' must be [x1, z1, x2, z2]"));
    }
    Ok(CellRect::new(v[0], v[1], v[2], v[3]))
}

fn cell_of(j: &Json, what: &str) -> R<(i32, i32)> {
    let v = ints(j, what)?;
    if v.len() != 2 {
        return Err(format!("'{what}' must be [x, z]"));
    }
    Ok((v[0], v[1]))
}

/// An exemplar: "shipped", null (missing) or `{"props": {...}}`.
fn exemplar_of(v: &Json, shipped: fn() -> PropertyHolder) -> R<Option<Rc<PropertyHolder>>> {
    Ok(match v {
        Json::Null => None,
        Json::Str(s) if s == "shipped" => Some(Rc::new(shipped())),
        other => Some(Rc::new(holder_of(field(other, "props")?)?)),
    })
}

/// `demolition_env`: `cells` ([x, z], default [64, 64]), `building_development_simulator`,
/// `flora_simulator`, `prop_manager` (bools, default true), `view_filter` (token or null),
/// `view_level` (int or null; default 1), `rejects` (`{"filter": [occupants]}`),
/// `rejects_buildings` (filters refusing the building type), `thresholds` /
/// `building_development` (exemplars), `water` ([[x, z], …] water vertices), `altitudes`
/// ([[x, z, h], …]), `zones` ([[x1, z1, x2, z2, zone], …]).
fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "cells") {
        w.cells.set(cell_of(v, "cells")?);
    }
    if let Some(v) = opt(op, "building_development_simulator") {
        w.bds.set(bool_of(v, "building_development_simulator")?);
    }
    if let Some(v) = opt(op, "flora_simulator") {
        w.flora.set(bool_of(v, "flora_simulator")?);
    }
    if let Some(v) = opt(op, "prop_manager") {
        w.prop_mgr.set(bool_of(v, "prop_manager")?);
    }
    if let Some(v) = op.get("view_filter") {
        w.view_filter.set(match v {
            Json::Null => None,
            t => Some(token_of(t, "view_filter")?),
        });
    }
    if let Some(v) = op.get("view_level") {
        w.view_level.set(match v {
            Json::Null => None,
            t => Some(u32_of(t, "view_level")?),
        });
    }
    if let Some(v) = opt(op, "rejects") {
        for (k, list) in v.as_object().ok_or("rejects must be an object")? {
            let f = token_of(&Json::Str(k.clone()), "filter")?;
            w.rejects
                .borrow_mut()
                .insert(f, tokens(list, "rejects")?.into_iter().collect());
        }
    }
    if let Some(v) = opt(op, "rejects_buildings") {
        *w.rejects_buildings.borrow_mut() = tokens(v, "rejects_buildings")?.into_iter().collect();
    }
    if let Some(v) = op.get("thresholds") {
        *w.thresholds.borrow_mut() = exemplar_of(v, shipped_thresholds)?;
    }
    if let Some(v) = op.get("building_development") {
        *w.bd.borrow_mut() = exemplar_of(v, shipped_building_development)?;
    }
    if let Some(v) = opt(op, "water") {
        for c in arr(v, "water")? {
            w.water.borrow_mut().insert(cell_of(c, "water")?);
        }
    }
    if let Some(v) = opt(op, "altitudes") {
        for c in arr(v, "altitudes")? {
            let a = arr(c, "altitudes")?;
            if a.len() != 3 {
                return Err("altitudes entries are [x, z, h]".into());
            }
            let x = i64_of(&a[0], "x")? as i32;
            let z = i64_of(&a[1], "z")? as i32;
            w.alt.borrow_mut().insert((x, z), f32_of(&a[2], "h")?);
        }
    }
    if let Some(v) = opt(op, "zones") {
        for c in arr(v, "zones")? {
            let a = ints(c, "zones")?;
            if a.len() != 5 {
                return Err("zones entries are [x1, z1, x2, z2, zone]".into());
            }
            for x in a[0]..=a[2] {
                for z in a[1]..=a[3] {
                    w.zones.borrow_mut().insert((x, z), a[4] as u32);
                }
            }
        }
    }
    Ok(())
}

fn type_of(j: &Json) -> R<u32> {
    Ok(match j.as_str() {
        Some("foundation") => occ_type::FOUNDATION,
        Some("building") => occ_type::BUILDING,
        Some("pole") => occ_type::POWER_POLE,
        Some("construction") => occ_type::CONSTRUCTION,
        Some("prop") => occ_type::PROP,
        Some("network") => occ_type::NETWORK,
        Some("flora") => occ_type::FLORA,
        _ => u32_of(j, "type")?,
    })
}

/// `demolition_occupant`: `occupant`, `type` (foundation, building, pole, construction, prop,
/// network, flora or an ID), `cell` ([x, z]: centred in that cell, an 8 m box), `bounds`
/// ([x0, z0, x1, z1] metres), `flags` (default 1 = in the occupant manager), `system_lock`,
/// `player_lock`, `foundation` (slot 9's answer; absent = no foundation interface),
/// `network_flags` (absent = no network interface), `network_types`, `pole` / `network`
/// (the interfaces; default by type), `cost`, `connections`, `default_ok`, `realistic_ok`
/// (default true), `props` (`{"0xID": bool}`), `groups`, `extent` (screen extent; null =
/// below the cutoff; default 10). Updating an existing occupant changes only the given
/// fields.
fn occupant(w: &World, op: &Json) -> R<()> {
    let t = token_of(field(op, "occupant")?, "occupant")?;
    let existing = w.occs.borrow().get(&t).cloned();
    let mut o = match existing {
        Some(o) => o,
        None => {
            let ty = type_of(field(op, "type")?)?;
            Occ {
                ty,
                pos: (8.0, 0.0, 8.0),
                bounds: (4.0, 4.0, 12.0, 12.0),
                flags: OCCUPANT_FLAG_IN_MANAGER,
                system_lock: false,
                player_lock: false,
                foundation: None,
                net_flags: None,
                net_types: Vec::new(),
                pole: ty == occ_type::POWER_POLE,
                network: ty == occ_type::NETWORK,
                cost: 0,
                connections: Vec::new(),
                default_ok: true,
                realistic_ok: true,
                props: BTreeMap::new(),
                groups: Vec::new(),
                extent: Some(10.0),
            }
        }
    };
    if let Some(v) = opt(op, "type") {
        o.ty = type_of(v)?;
    }
    if let Some(v) = opt(op, "cell") {
        let (x, z) = cell_of(v, "cell")?;
        let (cx, cz) = (16.0 * x as f32 + 8.0, 16.0 * z as f32 + 8.0);
        o.pos = (cx, 0.0, cz);
        o.bounds = (cx - 4.0, cz - 4.0, cx + 4.0, cz + 4.0);
    }
    if let Some(v) = opt(op, "bounds") {
        let b = arr(v, "bounds")?
            .iter()
            .map(|e| f32_of(e, "bounds"))
            .collect::<R<Vec<f32>>>()?;
        if b.len() != 4 {
            return Err("bounds is [x0, z0, x1, z1]".into());
        }
        o.bounds = (b[0], b[1], b[2], b[3]);
    }
    if let Some(v) = opt(op, "flags") {
        o.flags = u32_of(v, "flags")?;
    }
    if let Some(v) = opt(op, "system_lock") {
        o.system_lock = bool_of(v, "system_lock")?;
    }
    if let Some(v) = opt(op, "player_lock") {
        o.player_lock = bool_of(v, "player_lock")?;
    }
    if let Some(v) = op.get("foundation") {
        o.foundation = opt(op, "foundation").map(|_| u32_of(v, "foundation")).transpose()?;
    }
    if let Some(v) = op.get("network_flags") {
        o.net_flags = opt(op, "network_flags")
            .map(|_| u32_of(v, "network_flags"))
            .transpose()?;
    }
    if let Some(v) = opt(op, "network_types") {
        o.net_types = arr(v, "network_types")?
            .iter()
            .map(|e| u32_of(e, "network_types"))
            .collect::<R<_>>()?;
    }
    if let Some(v) = opt(op, "pole") {
        o.pole = bool_of(v, "pole")?;
    }
    if let Some(v) = opt(op, "network") {
        o.network = bool_of(v, "network")?;
    }
    if let Some(v) = opt(op, "cost") {
        o.cost = i64_of(v, "cost")?;
    }
    if let Some(v) = opt(op, "connections") {
        o.connections = tokens(v, "connections")?;
    }
    if let Some(v) = opt(op, "default_ok") {
        o.default_ok = bool_of(v, "default_ok")?;
    }
    if let Some(v) = opt(op, "realistic_ok") {
        o.realistic_ok = bool_of(v, "realistic_ok")?;
    }
    if let Some(v) = opt(op, "props") {
        for (k, b) in v.as_object().ok_or("props must be an object")? {
            o.props.insert(u32_of(&Json::Str(k.clone()), "prop")?, bool_of(b, k)?);
        }
    }
    if let Some(v) = opt(op, "groups") {
        o.groups = arr(v, "groups")?
            .iter()
            .map(|e| u32_of(e, "groups"))
            .collect::<R<_>>()?;
    }
    if let Some(v) = op.get("extent") {
        o.extent = opt(op, "extent").map(|_| f32_of(v, "extent")).transpose()?;
    }
    w.occs.borrow_mut().insert(t, o);
    Ok(())
}

/// `demolition_lot`: `lot`, `rect` ([x1, z1, x2, z2] cells, inclusive), `state` (default 1),
/// `building`, `occupants`, `demolishable` (default true), `zone` (also written to the zone
/// map over the rectangle), `water` (water compatibility; null = no configuration; default
/// false), `locked` (vertices locked), `initialized` (default true), `facing`, `condition`.
/// Updating an existing lot changes only the given fields (the zone map only with `zone`).
fn lot(w: &World, op: &Json) -> R<()> {
    let t = token_of(field(op, "lot")?, "lot")?;
    let existing = w.lots.borrow().get(&t).cloned();
    let mut l = match existing {
        Some(l) => l,
        None => Lot {
            rect: rect_of(field(op, "rect")?, "rect")?,
            state: 1,
            building: None,
            occs: Vec::new(),
            demolishable: true,
            zone: 0,
            water: Some(false),
            locked: false,
            initialized: true,
            facing: 0,
            condition: 0,
        },
    };
    if let Some(v) = opt(op, "rect") {
        l.rect = rect_of(v, "rect")?;
    }
    if let Some(v) = opt(op, "state") {
        l.state = u32_of(v, "state")?;
    }
    if let Some(v) = opt(op, "building") {
        l.building = Some(token_of(v, "building")?);
    }
    if let Some(v) = opt(op, "occupants") {
        l.occs = tokens(v, "occupants")?;
    }
    if let Some(v) = opt(op, "demolishable") {
        l.demolishable = bool_of(v, "demolishable")?;
    }
    if let Some(v) = op.get("water") {
        l.water = opt(op, "water").map(|_| bool_of(v, "water")).transpose()?;
    }
    if let Some(v) = opt(op, "locked") {
        l.locked = bool_of(v, "locked")?;
    }
    if let Some(v) = opt(op, "initialized") {
        l.initialized = bool_of(v, "initialized")?;
    }
    if let Some(v) = opt(op, "facing") {
        l.facing = u32_of(v, "facing")?;
    }
    if let Some(v) = opt(op, "condition") {
        l.condition = u32_of(v, "condition")?;
    }
    if let Some(v) = opt(op, "zone") {
        l.zone = u32_of(v, "zone")?;
        for x in l.rect.x1..=l.rect.x2 {
            for z in l.rect.z1..=l.rect.z2 {
                w.zones.borrow_mut().insert((x, z), l.zone);
            }
        }
    }
    w.lots.borrow_mut().insert(t, l);
    Ok(())
}

// ---- calls ----------------------------------------------------------------------------------------

/// `demolition_identity`: `iids` (`{"0xIID": offset or null}`) and `class`.
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
        let got = w.util().map_or(CLSID_DEMOLITION, |u| u.class_id());
        if got != want {
            return Err(format!("class 0x{got:08X}, expected 0x{want:08X}"));
        }
    }
    Ok(())
}

/// `demolition_message`: `type` (ExemplarChanged, PostCityInit, DeferredLotDemolition or an
/// ID), `data1`, `data2`, `lot`, `dezone`.
fn message(w: &World, op: &Json) -> R<()> {
    let u = w.util()?;
    let t = match field(op, "type")?.as_str() {
        Some("ExemplarChanged") => msg::EXEMPLAR_CHANGED,
        Some("PostCityInit") => msg::POST_CITY_INIT,
        Some("DeferredLotDemolition") => msg::DEFERRED_LOT_DEMOLITION,
        _ => u32_of(field(op, "type")?, "type")?,
    };
    let m = DemolitionMessage {
        msg_type: t,
        data1: opt(op, "data1").map_or(Ok(0), |v| u32_of(v, "data1"))?,
        data2: opt(op, "data2").map_or(Ok(0), |v| u32_of(v, "data2"))?,
        lot: opt_token(op, "lot")?,
        dezone: opt(op, "dezone").and_then(boolean).unwrap_or(false),
    };
    check_expect(op, Json::Bool(u.do_message(&m)))
}

fn outcome_json(o: Outcome) -> Json {
    Json::Object(vec![
        ("ok".to_owned(), Json::Bool(o.ok)),
        ("cost".to_owned(), o.cost.map_or(Json::Null, ji)),
    ])
}

fn common(op: &Json) -> R<(bool, u32, u32, bool)> {
    Ok((
        bool_of(field(op, "do_it")?, "do_it")?,
        opt(op, "privilege").map_or(Ok(1), |v| u32_of(v, "privilege"))?,
        opt(op, "flags").map_or(Ok(1), |v| u32_of(v, "flags"))?,
        opt(op, "dezone").and_then(boolean).unwrap_or(false),
    ))
}

/// `demolition_lots`: `DemolishLots(do_it, lots, privilege (default 1), flags (default 1),
/// dezone, caller_filter, demolished)`; `lots` may hold nulls; `expect` is `{"ok", "cost"}`
/// (cost null = left unchanged).
fn demolish_lots(w: &World, op: &Json) -> R<()> {
    let u = w.util()?;
    let (do_it, privilege, flags, dezone) = common(op)?;
    let lots = arr(field(op, "lots")?, "lots")?
        .iter()
        .map(|v| match v {
            Json::Null => Ok(None),
            t => token_of(t, "lots").map(Some),
        })
        .collect::<R<Vec<_>>>()?;
    let r = u.demolish_lots(
        do_it,
        &lots,
        privilege,
        flags,
        dezone,
        opt_token(op, "caller_filter")?,
        opt_token(op, "demolished")?,
    );
    check_expect(op, outcome_json(r))
}

/// `demolition_region`: `DemolishRegion(do_it, rect (a full region), privilege, flags,
/// dezone, caller_filter, demolished, clicked, sx, sy)`; `expect` as for `demolition_lots`.
fn demolish_region(w: &World, op: &Json) -> R<()> {
    let u = w.util()?;
    let (do_it, privilege, flags, dezone) = common(op)?;
    let region = CellRegion::new(rect_of(field(op, "rect")?, "rect")?, true);
    let r = u.demolish_region(
        do_it,
        &region,
        privilege,
        flags,
        dezone,
        opt_token(op, "caller_filter")?,
        opt_token(op, "demolished")?,
        opt_token(op, "clicked")?,
        opt(op, "sx").map_or(Ok(0), |v| i64_of(v, "sx"))? as i32,
        opt(op, "sy").map_or(Ok(0), |v| i64_of(v, "sy"))? as i32,
    );
    check_expect(op, outcome_json(r))
}

/// `demolition_demolish_occupant`: `DemolishOccupant(do_it, occupant, privilege, flags,
/// skip_network_update, orphans)` with `orphans` a list token (null = none).
fn demolish_occupant(w: &World, op: &Json) -> R<()> {
    let u = w.util()?;
    let (do_it, privilege, flags, _) = common(op)?;
    let o = token_of(field(op, "occupant")?, "occupant")?;
    let skip = opt(op, "skip_network_update").and_then(boolean).unwrap_or(false);
    let mut orphans = match opt_token(op, "orphans")? {
        Some(t) => Orphans::Host(t),
        None => Orphans::None,
    };
    let r = u.demolish_occupant(do_it, o, privilege, flags, skip, &mut orphans);
    check_expect(op, outcome_json(r))
}

/// `demolition_terrain`: `ModifyTerrainHeight(do_it, per_vertex, rect, altitude, exempt,
/// reason (default 5))`; `expect` is the count.
fn terrain(w: &World, op: &Json) -> R<()> {
    let u = w.util()?;
    let r = rect_of(field(op, "rect")?, "rect")?;
    let n = u.modify_terrain_height(
        bool_of(field(op, "do_it")?, "do_it")?,
        bool_of(field(op, "per_vertex")?, "per_vertex")?,
        r.x1,
        r.z1,
        r.x2,
        r.z2,
        f32_of(field(op, "altitude")?, "altitude")?,
        opt_token(op, "exempt")?,
        opt(op, "reason").map_or(Ok(5), |v| u32_of(v, "reason"))?,
    );
    check_expect(op, ji(n))
}

fn expect(w: &World, op: &Json) -> R<()> {
    let s = w.util()?.snapshot();
    let t = s.tuning;
    let got = vec![
        ("initialized".to_owned(), Json::Bool(s.initialized)),
        ("destroyed".to_owned(), Json::Bool(s.destroyed)),
        ("ref_count".to_owned(), ji(s.ref_count)),
        (
            "tuning".to_owned(),
            Json::Array(vec![
                ji(t.small_medium_model),
                ji(t.medium_large_model),
                ji(t.small_medium_group),
                ji(t.medium_large_group),
                ji(t.destroyed_tile_cost),
            ]),
        ),
        ("cells".to_owned(), Json::Array(vec![ji(s.cells_x), ji(s.cells_z)])),
    ];
    check_state(&got, field(op, "state")?)
}

/// `demolition_expect_lot`: `lot` with `state` (`state`, `building`, `zone`) or `absent`.
fn expect_lot(w: &World, op: &Json) -> R<()> {
    let t = token_of(field(op, "lot")?, "lot")?;
    let l = w.lots.borrow().get(&t).cloned();
    if opt(op, "absent").and_then(boolean).unwrap_or(false) {
        return match l {
            None => Ok(()),
            Some(_) => Err(format!("lot 0x{t:X} exists")),
        };
    }
    let l = l.ok_or_else(|| format!("no lot 0x{t:X}"))?;
    let got = vec![
        ("state".to_owned(), ji(l.state)),
        ("building".to_owned(), l.building.map_or(Json::Null, tok)),
        ("zone".to_owned(), ji(l.zone)),
        ("rect".to_owned(), rect_json(l.rect)),
    ];
    check_state(&got, field(op, "state")?)
}

/// `demolition_expect_zones`: `cells` = [[x, z, zone], …] of the zone map; `lot_count`.
fn expect_zones(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "cells") {
        for c in arr(v, "cells")? {
            let a = ints(c, "cells")?;
            let got = w.zone_at(a[0], a[1]);
            if got != a[2] as u32 {
                return Err(format!("zone at ({}, {}) is {got}, expected {}", a[0], a[1], a[2]));
            }
        }
    }
    if let Some(v) = opt(op, "lot_count") {
        let n = w.lots.borrow().len() as i64;
        if n != i64_of(v, "lot_count")? {
            return Err(format!("{n} lots"));
        }
    }
    Ok(())
}

// ---- the record (§7) ---------------------------------------------------------------------------------

/// `demolition_write`: `expect`, `hex`.
fn write(w: &World, op: &Json) -> R<()> {
    let u = w.util()?;
    let mut out = ByteWriter::new();
    check_expect(op, Json::Bool(u.write(&mut out)))?;
    let bytes = out.into_bytes();
    if let Some(h) = opt(op, "hex") {
        let want = hex_decode(scalar::str_of(h, "hex")?)?;
        if bytes != want {
            return Err(format!("record {}, expected {}", hex_encode(&bytes), hex_encode(&want)));
        }
    }
    *w.record.borrow_mut() = bytes;
    Ok(())
}

/// `demolition_read`: `hex` (or the last written record) into the current utility;
/// `expect`.
fn read(w: &World, op: &Json) -> R<()> {
    let u = w.util()?;
    let bytes = match opt(op, "hex") {
        Some(h) => hex_decode(scalar::str_of(h, "hex")?)?,
        None => w.record.borrow().clone(),
    };
    check_expect(op, Json::Bool(u.read(&mut ByteReader::new(&bytes))))
}
