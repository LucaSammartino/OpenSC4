// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Conformance ops for `docs/specs/network-manager.md`: a scripted city
//! (network occupants, the terrain, the tools, zones, lots, network lot
//! configurations, the riot manager's removal counters, a scripted or the
//! default generator) and ops that drive the network manager, the network
//! lot manager and the riot lock. See README.md ("Network manager ops").

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::rc::{Rc, Weak};

use fc4_core::grid::CellRect;
use fc4_core::network_manager::{
    clear_network_flag, corner_mismatch, flag, lot_key, lot_manager_query_interface, manager_query_interface, msg,
    object_position, pothole_offset, rect_is_empty, road_texture, road_variation, set_network_flag, terrain_rect,
    LotEntry, NetworkLotConfig, NetworkLotHost, NetworkLotManager, NetworkLotMessage, NetworkLotObject,
    NetworkLotRandom, NetworkManager, NetworkManagerHost, NetworkMessage, Position, RemovalCounters, RiotHost,
    RiotLock, CLSID_NETWORK_LOT_MANAGER, CLSID_NETWORK_MANAGER, OBJECT_FLORA, OBJECT_PROP,
};
use fc4_core::stream::{ByteReader, ByteWriter, RefResolver, RefValue, RegisteredObject, SaveObject, SaveRegistry};

use crate::foundations::{hex_decode, hex_encode};
use crate::json::Json;
use crate::runner::{boolean, field, HostRef};
use crate::scalar::{
    self, arg, arr, bool_of, check_expect, check_state, i64_of, jf, ji, opt, str_of, u32_of, EventLog, R,
};

/// The managers' own tokens.
const MANAGER_TOKEN: u64 = 0xA000;
const LOT_MANAGER_TOKEN: u64 = 0xB000;
/// The class every saved host object is written with.
const OCCUPANT_CLASS: u32 = 0xA821_EF94;
const CITY: (i32, i32) = (64, 64);

#[derive(Clone)]
struct Record {
    bytes: Vec<u8>,
    entries: Vec<RegisteredObject>,
}

#[derive(Clone)]
struct Occ {
    flags: u32,
    in_city: bool,
    managed: bool,
    lit: bool,
    cell: (i32, i32),
    /// Height above the terrain at every corner.
    height: f32,
    creation: u64,
    retrofit: bool,
    flag4: bool,
    removable: bool,
    piece: u32,
    rotation: i32,
    flip: i32,
    intersection: bool,
    edges6: u32,
    active_edges: u32,
}

#[derive(Clone, Copy, Default)]
struct Lot {
    wealth: i32,
    livable: bool,
    building_type: u32,
    rect: (i32, i32, i32, i32),
}

pub(crate) struct World {
    me: RefCell<Weak<World>>,
    events: EventLog,
    mgr: RefCell<Option<Rc<NetworkManager>>>,
    lots_mgr: RefCell<Option<Rc<NetworkLotManager>>>,
    session_seed: Cell<u32>,
    stub: Cell<bool>,
    u32s: RefCell<VecDeque<u32>>,
    night: Cell<bool>,
    uk_paths: Cell<bool>,
    terrain: Cell<f32>,
    slope: Cell<f32>,
    traffic_map: Cell<bool>,
    crews_unregister: Cell<bool>,
    refs: RefCell<BTreeMap<u64, i64>>,
    occupants: RefCell<BTreeMap<u64, Occ>>,
    next_tool: Cell<u64>,
    tool_types: RefCell<BTreeMap<u64, u32>>,
    next_object: Cell<u64>,
    zones: RefCell<BTreeMap<(i32, i32), i32>>,
    lot_cells: RefCell<BTreeMap<(i32, i32), u64>>,
    lot_data: RefCell<BTreeMap<u64, Lot>>,
    profiles: RefCell<BTreeMap<u32, i32>>,
    configs: RefCell<Vec<NetworkLotConfig>>,
    object_ids: RefCell<BTreeMap<(u64, usize), Vec<u32>>>,
    riot: RefCell<RiotLock>,
    counters: RefCell<BTreeMap<u64, RemovalCounters>>,
    records: RefCell<BTreeMap<String, Record>>,
}

/// The scripted generator: queued values (then 0); every draw is an event.
struct Stub(Weak<World>);

impl NetworkLotRandom for Stub {
    fn uniform_u32(&mut self, n: u32) -> u32 {
        let w = self.0.upgrade().expect("world alive");
        w.events.push("draw", vec![("n", ji(n))]);
        let v = w.u32s.borrow_mut().pop_front();
        v.unwrap_or(0)
    }
}

impl World {
    fn new() -> Rc<World> {
        let w = Rc::new(World {
            me: RefCell::new(Weak::new()),
            events: EventLog::default(),
            mgr: RefCell::new(None),
            lots_mgr: RefCell::new(None),
            session_seed: Cell::new(0x1234_5678),
            stub: Cell::new(true),
            u32s: RefCell::new(VecDeque::new()),
            night: Cell::new(false),
            uk_paths: Cell::new(false),
            terrain: Cell::new(0.0),
            slope: Cell::new(0.0),
            traffic_map: Cell::new(true),
            crews_unregister: Cell::new(true),
            refs: RefCell::new(BTreeMap::new()),
            occupants: RefCell::new(BTreeMap::new()),
            next_tool: Cell::new(0x7000),
            tool_types: RefCell::new(BTreeMap::new()),
            next_object: Cell::new(0x5000),
            zones: RefCell::new(BTreeMap::new()),
            lot_cells: RefCell::new(BTreeMap::new()),
            lot_data: RefCell::new(BTreeMap::new()),
            profiles: RefCell::new(BTreeMap::new()),
            configs: RefCell::new(Vec::new()),
            object_ids: RefCell::new(BTreeMap::new()),
            riot: RefCell::new(RiotLock::new()),
            counters: RefCell::new(BTreeMap::new()),
            records: RefCell::new(BTreeMap::new()),
        });
        *w.me.borrow_mut() = Rc::downgrade(&w);
        w
    }

    fn rc(&self) -> Rc<World> {
        self.me.borrow().upgrade().expect("world alive")
    }

    fn mgr(&self) -> R<Rc<NetworkManager>> {
        self.mgr
            .borrow()
            .clone()
            .ok_or_else(|| "no network manager (net_create)".into())
    }

    fn lots(&self) -> R<Rc<NetworkLotManager>> {
        self.lots_mgr
            .borrow()
            .clone()
            .ok_or_else(|| "no network lot manager (net_create)".into())
    }

    fn new_mgr(&self) -> Rc<NetworkManager> {
        let m = NetworkManager::new(self.rc() as Rc<dyn NetworkManagerHost>, MANAGER_TOKEN);
        *self.mgr.borrow_mut() = Some(m.clone());
        m
    }

    fn new_lots(&self) -> Rc<NetworkLotManager> {
        let m = NetworkLotManager::new(
            self.rc() as Rc<dyn NetworkLotHost>,
            LOT_MANAGER_TOKEN,
            self.session_seed.get(),
        );
        if self.stub.get() {
            m.set_random(Box::new(Stub(self.me.borrow().clone())));
        }
        *self.lots_mgr.borrow_mut() = Some(m.clone());
        m
    }

    fn bump(&self, t: u64, d: i64) {
        *self.refs.borrow_mut().entry(t).or_insert(0) += d;
    }

    fn occ(&self, t: u64) -> Option<Occ> {
        self.occupants.borrow().get(&t).cloned()
    }

    fn flags_of(&self, t: u64) -> u32 {
        self.occ(t).map_or(0, |o| o.flags)
    }

    fn tool_type(&self, tool: u64) -> Json {
        self.tool_types.borrow().get(&tool).map_or(ji(-1), |&t| ji(t))
    }

    fn new_object(&self) -> u64 {
        let t = self.next_object.get();
        self.next_object.set(t + 1);
        t
    }

    /// The network occupant at a cell (any type), for the riot.
    fn tile(&self, x: i32, z: i32) -> Option<u64> {
        self.occupants
            .borrow()
            .iter()
            .find(|(_, o)| o.cell == (x, z) && o.flags & flag::ALL_TYPES != 0)
            .map(|(t, _)| *t)
    }

    fn rc_mgr_for_crew(&self) -> Option<Rc<NetworkManager>> {
        self.mgr.borrow().clone()
    }
}

fn tok(t: u64) -> Json {
    ji(t as i64)
}

fn cell_json(x: i32, z: i32) -> Json {
    Json::Array(vec![ji(x), ji(z)])
}

fn pos_json(p: Position) -> Json {
    Json::Array(p.iter().map(|&v| jf(v)).collect())
}

fn rect_json(r: &CellRect) -> Json {
    Json::Array(vec![ji(r.x1), ji(r.z1), ji(r.x2), ji(r.z2)])
}

fn tokens_json(v: &[u64]) -> Json {
    Json::Array(v.iter().map(|&t| tok(t)).collect())
}

fn opt_tokens_json(v: &[Option<u64>]) -> Json {
    Json::Array(v.iter().map(|e| e.map_or(Json::Null, tok)).collect())
}

impl NetworkManagerHost for World {
    fn subscribe(&self, msg_type: u32) {
        self.events.push("subscribe", vec![("type", ji(msg_type))]);
    }
    fn unsubscribe(&self, msg_type: u32) {
        self.events.push("unsubscribe", vec![("type", ji(msg_type))]);
    }
    fn uk_paths(&self) -> bool {
        self.uk_paths.get()
    }
    fn city_size(&self) -> (i32, i32) {
        CITY
    }
    fn is_night(&self) -> bool {
        self.night.get()
    }
    fn view_rotation_changed(&self) {
        self.events.push("view_rotation", vec![]);
    }
    fn add_ref(&self, token: u64) {
        self.bump(token, 1);
    }
    fn release(&self, token: u64) {
        self.bump(token, -1);
    }
    fn create_tool(&self, network_type: u32) -> Option<u64> {
        let t = self.next_tool.get();
        self.next_tool.set(t + 1);
        self.tool_types.borrow_mut().insert(t, network_type);
        self.bump(t, 1);
        self.events
            .push("create_tool", vec![("type", ji(network_type)), ("tool", tok(t))]);
        Some(t)
    }
    fn tool_init(&self, tool: u64) {
        self.events.push("tool_init", vec![("type", self.tool_type(tool))]);
    }
    fn tool_shutdown(&self, tool: u64) {
        self.events.push("tool_shutdown", vec![("type", self.tool_type(tool))]);
    }
    fn tool_destroy_occupants(&self, tool: u64, occupants: &[u64]) {
        self.events.push(
            "destroy",
            vec![("type", self.tool_type(tool)), ("occupants", tokens_json(occupants))],
        );
    }
    fn tool_update_underground_heights(&self, tool: u64, cells: &[(i32, i32)], occupants: &[u64], network_type: u32) {
        self.events.push(
            "underground",
            vec![
                ("type", self.tool_type(tool)),
                ("network_type", ji(network_type)),
                (
                    "cells",
                    Json::Array(cells.iter().map(|&(x, z)| cell_json(x, z)).collect()),
                ),
                ("occupants", tokens_json(occupants)),
            ],
        );
    }
    fn tool_cell_info(&self, tool: u64, packed: u32) -> u64 {
        self.events.push(
            "cell_info",
            vec![("type", self.tool_type(tool)), ("packed", ji(packed))],
        );
        u64::from(packed)
    }
    fn tool_reinsert_piece(&self, tool: u64, info: u64) {
        self.events
            .push("reinsert", vec![("type", self.tool_type(tool)), ("info", tok(info))]);
    }
    fn query_network_occupant(&self, occupant: u64) -> Option<u64> {
        if NetworkManagerHost::is_network_occupant(self, occupant) {
            self.bump(occupant, 1);
            Some(occupant)
        } else {
            None
        }
    }
    fn is_network_occupant(&self, occupant: u64) -> bool {
        self.flags_of(occupant) & flag::ALL_TYPES != 0
    }
    fn network_flags(&self, n: u64) -> u32 {
        self.flags_of(n)
    }
    fn is_of_type(&self, n: u64, network_type: u32) -> bool {
        network_type < 32 && self.flags_of(n) & (1 << network_type) != 0
    }
    fn occupant_flags(&self, occupant: u64) -> u32 {
        u32::from(self.occ(occupant).is_some_and(|o| o.in_city))
    }
    fn occupant_flag_set(&self, occupant: u64, f: u32) -> bool {
        f == 4 && self.occ(occupant).is_some_and(|o| o.flag4)
    }
    fn occupied_cell(&self, n: u64) -> (i32, i32) {
        self.occ(n).map_or((0, 0), |o| o.cell)
    }
    fn occupant_altitude(&self, n: u64, _x: f32, _z: f32) -> f32 {
        self.terrain.get() + self.occ(n).map_or(0.0, |o| o.height)
    }
    fn is_managed_globally(&self, n: u64) -> bool {
        self.occ(n).is_some_and(|o| o.managed)
    }
    fn can_be_lit(&self, n: u64) -> bool {
        self.occ(n).is_some_and(|o| o.lit)
    }
    fn set_lit(&self, n: u64, lit: bool) {
        self.events
            .push("set_lit", vec![("occupant", tok(n)), ("lit", Json::Bool(lit))]);
    }
    fn check_for_retrofit(&self, n: u64) -> bool {
        self.occ(n).is_some_and(|o| o.retrofit)
    }
    fn retrofit(&self, n: u64) {
        self.events.push("retrofit", vec![("occupant", tok(n))]);
    }
    fn creation_number(&self, occupant: u64) -> u64 {
        self.occ(occupant).map_or(occupant, |o| o.creation)
    }
    fn remove_occupant(&self, occupant: u64) -> bool {
        self.events.push("remove_occupant", vec![("occupant", tok(occupant))]);
        self.occ(occupant).is_some_and(|o| o.removable)
    }
    fn occupants_in_rect(&self, rect: CellRect, mask: u32) -> Vec<u64> {
        self.events
            .push("query", vec![("rect", rect_json(&rect)), ("mask", ji(mask))]);
        self.occupants
            .borrow()
            .iter()
            .filter(|(_, o)| {
                let (x, z) = o.cell;
                o.flags & mask != 0 && x >= rect.x1 && x <= rect.x2 && z >= rect.z1 && z <= rect.z2
            })
            .map(|(t, _)| *t)
            .collect()
    }
    fn network_occupants(&self) -> Vec<u64> {
        self.occupants.borrow().keys().copied().collect()
    }
    fn post_refresh(&self, n: u64) {
        self.events.push("refresh", vec![("occupant", tok(n))]);
    }
    fn terrain_altitude(&self, _x: f32, _z: f32) -> f32 {
        self.terrain.get()
    }
    fn crew_shutdown(&self, crew: u64) {
        self.events.push("crew_shutdown", vec![("crew", tok(crew))]);
        if self.crews_unregister.get() {
            if let Some(m) = self.rc_mgr_for_crew() {
                m.unregister_construction_crew(crew);
            }
        }
    }
    fn object_ref(&self, token: u64) -> RefValue {
        RefValue::Host {
            token,
            class_id: OCCUPANT_CLASS,
        }
    }
    fn resolve_ref(&self, r: &RefValue, _iid: u32) -> Option<u64> {
        resolve(self, r)
    }
}

/// `SC4ReadGZSerializable`: a host object known to the world comes back
/// with one reference.
fn resolve(w: &World, r: &RefValue) -> Option<u64> {
    match *r {
        RefValue::Host { token, .. }
            if w.occupants.borrow().contains_key(&token) || (0x5000..0x7000).contains(&token) =>
        {
            w.bump(token, 1);
            Some(token)
        }
        _ => None,
    }
}

impl NetworkLotHost for World {
    fn subscribe(&self, msg_type: u32) {
        self.events.push("subscribe", vec![("type", ji(msg_type))]);
    }
    fn unsubscribe(&self, msg_type: u32) {
        self.events.push("unsubscribe", vec![("type", ji(msg_type))]);
    }
    fn load_configurations(&self) -> Vec<NetworkLotConfig> {
        self.configs.borrow().clone()
    }
    fn add_ref(&self, token: u64) {
        self.bump(token, 1);
    }
    fn release(&self, token: u64) {
        self.bump(token, -1);
    }
    fn query_network_occupant(&self, unknown: u64) -> Option<u64> {
        NetworkManagerHost::query_network_occupant(self, unknown)
    }
    fn network_flags(&self, n: u64) -> u32 {
        self.flags_of(n)
    }
    fn piece_id(&self, n: u64) -> u32 {
        self.occ(n).map_or(0, |o| o.piece)
    }
    fn rotation(&self, n: u64) -> i32 {
        self.occ(n).map_or(0, |o| o.rotation)
    }
    fn flip(&self, n: u64) -> i32 {
        self.occ(n).map_or(0, |o| o.flip)
    }
    fn position_cell(&self, n: u64) -> (i32, i32) {
        self.occ(n).map_or((0, 0), |o| o.cell)
    }
    fn occupied_cell(&self, n: u64) -> (i32, i32) {
        self.occ(n).map_or((0, 0), |o| o.cell)
    }
    fn occupant_altitude(&self, n: u64, x: f32, z: f32) -> f32 {
        NetworkManagerHost::occupant_altitude(self, n, x, z)
    }
    fn is_intersection(&self, n: u64) -> bool {
        self.occ(n).is_some_and(|o| o.intersection)
    }
    fn edge_types_flag(&self, n: u64, _kind: u32) -> u32 {
        self.occ(n).map_or(0, |o| o.edges6)
    }
    fn active_edges(&self, n: u64) -> u32 {
        self.occ(n).map_or(0, |o| o.active_edges)
    }
    fn set_network_flag(&self, n: u64, bits: u32) {
        self.events
            .push("set_flag", vec![("occupant", tok(n)), ("bits", ji(bits))]);
        if let Some(o) = self.occupants.borrow_mut().get_mut(&n) {
            o.flags |= bits;
        }
    }
    fn set_under_texture(&self, n: u64, texture: u32) {
        self.events
            .push("texture", vec![("occupant", tok(n)), ("texture", ji(texture))]);
    }
    fn set_variation(&self, n: u64, variation: u32) {
        self.events
            .push("variation", vec![("occupant", tok(n)), ("variation", ji(variation))]);
    }
    fn traffic_map_exists(&self) -> bool {
        self.traffic_map.get()
    }
    fn find_network_tile(&self, x: i32, z: i32, mask: u32) -> Option<u64> {
        self.occupants
            .borrow()
            .iter()
            .find(|(_, o)| o.cell == (x, z) && o.flags & mask != 0)
            .map(|(t, _)| *t)
    }
    fn update_intersection(&self, n: u64) {
        self.events.push("intersection", vec![("occupant", tok(n))]);
    }
    fn city_size(&self) -> (i32, i32) {
        CITY
    }
    fn cell_center(&self, x: i32, z: i32) -> (f32, f32) {
        (x as f32 * 16.0 + 8.0, z as f32 * 16.0 + 8.0)
    }
    fn terrain_altitude(&self, _x: f32, _z: f32) -> f32 {
        self.terrain.get()
    }
    fn zone_type(&self, x: i32, z: i32) -> i32 {
        self.zones.borrow().get(&(x, z)).copied().unwrap_or(0)
    }
    fn lot_at(&self, x: i32, z: i32) -> Option<u64> {
        self.lot_cells.borrow().get(&(x, z)).copied()
    }
    fn lot_wealth(&self, lot: u64) -> i32 {
        self.lot_data.borrow().get(&lot).map_or(0, |l| l.wealth)
    }
    fn lot_is_livable(&self, lot: u64) -> bool {
        self.lot_data.borrow().get(&lot).is_some_and(|l| l.livable)
    }
    fn lot_building_type(&self, lot: u64) -> u32 {
        self.lot_data.borrow().get(&lot).map_or(0, |l| l.building_type)
    }
    fn building_profile_wealth(&self, building_type: u32) -> Option<i32> {
        self.profiles.borrow().get(&building_type).copied()
    }
    fn query_lot(&self, unknown: u64) -> Option<u64> {
        if self.lot_data.borrow().contains_key(&unknown) {
            self.bump(unknown, 1);
            Some(unknown)
        } else {
            None
        }
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        let r = self.lot_data.borrow().get(&lot).map_or((0, 0, -1, -1), |l| l.rect);
        CellRect {
            x1: r.0,
            z1: r.1,
            x2: r.2,
            z2: r.3,
        }
    }
    fn area_slope(&self, _x: i32, _z: i32) -> f32 {
        self.slope.get()
    }
    fn remove_street_props(&self, x: i32, z: i32) {
        self.events.push("remove_props", vec![("cell", cell_json(x, z))]);
    }
    fn add_street_prop(&self, id: u32, pos: Position, orientation: u32) -> Option<u64> {
        let t = self.new_object();
        self.events.push(
            "prop",
            vec![
                ("id", ji(id)),
                ("pos", pos_json(pos)),
                ("orientation", ji(orientation)),
                ("token", tok(t)),
            ],
        );
        Some(t)
    }
    fn street_prop_slot28(&self, prop: u64, value: u32) {
        self.events
            .push("prop28", vec![("prop", tok(prop)), ("value", ji(value))]);
    }
    fn street_prop_slot29(&self, prop: u64, value: u32) {
        self.events
            .push("prop29", vec![("prop", tok(prop)), ("value", ji(value))]);
    }
    fn demolish_flora(&self, x: i32, z: i32) {
        self.events.push("demolish_flora", vec![("cell", cell_json(x, z))]);
    }
    fn add_flora(&self, id: u32, pos: Position) -> Option<u64> {
        let t = self.new_object();
        self.events
            .push("flora", vec![("id", ji(id)), ("pos", pos_json(pos)), ("token", tok(t))]);
        Some(t)
    }
    fn flora_slot28(&self, flora: u64, index: u32) {
        self.events
            .push("flora28", vec![("flora", tok(flora)), ("index", ji(index))]);
    }
    fn object_ids(&self, config: u64, index: usize) -> Vec<u32> {
        self.object_ids
            .borrow()
            .get(&(config, index))
            .cloned()
            .unwrap_or_default()
    }
    fn create_effect_prop(&self) -> Option<u64> {
        let t = self.new_object();
        self.bump(t, 1);
        self.events.push("effect", vec![("token", tok(t))]);
        Some(t)
    }
    fn effect_set_name(&self, prop: u64, name: &str) {
        self.events.push(
            "effect_name",
            vec![("prop", tok(prop)), ("name", Json::Str(name.into()))],
        );
    }
    fn effect_start(&self, prop: u64) {
        self.events.push("effect_start", vec![("prop", tok(prop))]);
    }
    fn effect_stop(&self, prop: u64) {
        self.events.push("effect_stop", vec![("prop", tok(prop))]);
    }
    fn set_position(&self, prop: u64, pos: Position) {
        self.events
            .push("position", vec![("prop", tok(prop)), ("pos", pos_json(pos))]);
    }
    fn insert_occupant(&self, occupant: u64) {
        self.events.push("insert", vec![("occupant", tok(occupant))]);
    }
    fn remove_occupant(&self, occupant: u64) {
        self.events.push("remove", vec![("occupant", tok(occupant))]);
    }
    fn first_network_occupant_at(&self, x: i32, z: i32, mask: u32) -> Option<u64> {
        self.find_network_tile(x, z, mask)
    }
    fn object_ref(&self, token: u64) -> RefValue {
        RefValue::Host {
            token,
            class_id: OCCUPANT_CLASS,
        }
    }
    fn resolve_ref(&self, r: &RefValue, _iid: u32) -> Option<u64> {
        resolve(self, r)
    }
}

impl RiotHost for World {
    fn tile_occupant(&self, x: i32, z: i32) -> Option<u64> {
        self.tile(x, z)
    }
    fn add_ref(&self, token: u64) {
        self.bump(token, 1);
    }
    fn release(&self, token: u64) {
        self.bump(token, -1);
    }
    fn unset_removal_flags(&self, occupant: u64, m: u32) {
        self.events
            .push("unset_removal", vec![("occupant", tok(occupant)), ("mask", ji(m))]);
        self.counters
            .borrow_mut()
            .entry(occupant)
            .or_default()
            .unset_removal_flags(m);
    }
    fn set_removal_flags(&self, occupant: u64, m: u32) {
        self.events
            .push("set_removal", vec![("occupant", tok(occupant)), ("mask", ji(m))]);
        self.counters
            .borrow_mut()
            .entry(occupant)
            .or_default()
            .set_removal_flags(m);
    }
}

fn world(host: &HostRef) -> Rc<World> {
    scalar::world(host, "network_manager", World::new)
}

pub(crate) fn exec(host: &HostRef, name: &str, op: &Json) -> Option<R<()>> {
    let rest = name.strip_prefix("net_")?;
    let w = world(host);
    Some(match rest {
        "env" => env(&w, op),
        "occupant" => occupant(&w, op),
        "zone" => zone(&w, op),
        "lot" => lot(&w, op),
        "config" => config(&w, op),
        "create" => create(&w, op),
        "init" => lifecycle(&w, op, "init"),
        "shutdown" => lifecycle(&w, op, "shutdown"),
        "add_ref" => lifecycle(&w, op, "add_ref"),
        "release" => lifecycle(&w, op, "release"),
        "identity" => identity(&w, op),
        "message" => message(&w, op),
        "call" => call(&w, op),
        "riot" => riot(&w, op),
        "expect" => expect(&w, op),
        "expect_lot" => expect_lot(&w, op),
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

// ---- parsing helpers ----------------------------------------------------------------------------------

fn token_of(j: &Json, what: &str) -> R<u64> {
    Ok(i64_of(j, what)? as u64)
}

fn i32_of(j: &Json, what: &str) -> R<i32> {
    Ok(i64_of(j, what)? as i32)
}

/// A float, or `"nan"`.
fn float_of(j: &Json, what: &str) -> R<f32> {
    match j.as_str() {
        Some("nan") | Some("NaN") => Ok(f32::NAN),
        _ => scalar::num(j)
            .map(|v| v as f32)
            .ok_or_else(|| format!("'{what}' must be a number")),
    }
}

fn cell_of(j: &Json, what: &str) -> R<(i32, i32)> {
    match arr(j, what)? {
        [x, z] => Ok((i32_of(x, what)?, i32_of(z, what)?)),
        _ => Err(format!("'{what}' must be [x, z]")),
    }
}

fn rect_of(j: &Json, what: &str) -> R<CellRect> {
    match arr(j, what)? {
        [x1, z1, x2, z2] => Ok(CellRect {
            x1: i32_of(x1, what)?,
            z1: i32_of(z1, what)?,
            x2: i32_of(x2, what)?,
            z2: i32_of(z2, what)?,
        }),
        _ => Err(format!("'{what}' must be [x1, z1, x2, z2]")),
    }
}

const FLAG_NAMES: [(&str, u32); 22] = [
    ("road", flag::ROAD),
    ("rail", flag::RAIL),
    ("highway", flag::HIGHWAY),
    ("street", flag::STREET),
    ("pipe", flag::PIPE),
    ("power_line", flag::POWER_LINE),
    ("avenue", flag::AVENUE),
    ("subway", flag::SUBWAY),
    ("light_rail", flag::LIGHT_RAIL),
    ("monorail", flag::MONORAIL),
    ("one_way_road", flag::ONE_WAY_ROAD),
    ("dirt_road", flag::DIRT_ROAD),
    ("ground_highway", flag::GROUND_HIGHWAY),
    ("translucent", flag::TRANSLUCENT),
    ("pipe_distressed", flag::PIPE_DISTRESSED),
    ("pipe_burst", flag::PIPE_BURST),
    ("flag_40000", flag::UNKNOWN_40000),
    ("flag_80000", flag::UNKNOWN_80000),
    ("intersection", flag::INTERSECTION),
    ("connection_node", flag::CONNECTION_NODE),
    ("usable", flag::USABLE),
    ("flag_80000000", flag::UNKNOWN_80000000),
];

/// Network flags: an integer or a list of names and integers.
fn flags_of(j: &Json) -> R<u32> {
    match j {
        Json::Array(items) => items.iter().try_fold(0u32, |acc, e| match e.as_str() {
            Some(s) if !s.starts_with("0x") => FLAG_NAMES
                .iter()
                .find(|(n, _)| *n == s)
                .map(|(_, b)| acc | b)
                .ok_or_else(|| format!("unknown network flag '{s}'")),
            _ => Ok(acc | u32_of(e, "flags")?),
        }),
        _ => u32_of(j, "flags"),
    }
}

fn message_type(j: &Json) -> R<u32> {
    Ok(match j.as_str() {
        Some("ViewChanged") => msg::VIEW_CHANGED,
        Some("TerrainChanged") => msg::TERRAIN_CHANGED,
        Some("LongTerrainBegin") => msg::LONG_TERRAIN_BEGIN,
        Some("LongTerrainEnd") => msg::LONG_TERRAIN_END,
        Some("WaterDisplayChanged") => msg::WATER_DISPLAY_CHANGED,
        Some("PreCityShutdown") => msg::PRE_CITY_SHUTDOWN,
        Some("PostCityInit") => msg::POST_CITY_INIT,
        Some("OccupantInserted") => msg::OCCUPANT_INSERTED,
        Some("OccupantRemoved") => msg::OCCUPANT_REMOVED,
        Some("Dawn") => msg::DAWN,
        Some("Dusk") => msg::DUSK,
        Some("UsableChanged") => msg::USABLE_CHANGED,
        Some("CreateLotObjects") => msg::CREATE_LOT_OBJECTS,
        Some("ZoningChanged") => msg::ZONING_CHANGED,
        Some("LotStateChanged") => msg::LOT_STATE_CHANGED,
        Some("RoadDamageChanged") => msg::ROAD_DAMAGE_CHANGED,
        _ => u32_of(j, "type")?,
    })
}

/// Which manager an op addresses: `of` = "manager" (default) or "lots".
fn is_lots(op: &Json) -> R<bool> {
    match opt(op, "of").map(|v| str_of(v, "of")).transpose()? {
        None | Some("manager") => Ok(false),
        Some("lots") => Ok(true),
        Some(other) => Err(format!("unknown manager '{other}'")),
    }
}

fn u32_or(op: &Json, key: &str, default: u32) -> R<u32> {
    opt(op, key).map_or(Ok(default), |v| u32_of(v, key))
}

fn bool_or(op: &Json, key: &str, default: bool) -> R<bool> {
    opt(op, key).map_or(Ok(default), |v| bool_of(v, key))
}

// ---- environment ----------------------------------------------------------------------------------

/// `net_env`: `night`, `uk_paths`, `terrain` (the flat terrain altitude),
/// `slope` (the lot developer's area slope), `traffic_map`,
/// `crews_unregister`, `random` ("stub" or "default"), `session_seed`,
/// `u32s` (the stub's queue).
fn env(w: &World, op: &Json) -> R<()> {
    if let Some(v) = opt(op, "night") {
        w.night.set(bool_of(v, "night")?);
    }
    if let Some(v) = opt(op, "uk_paths") {
        w.uk_paths.set(bool_of(v, "uk_paths")?);
    }
    if let Some(v) = opt(op, "terrain") {
        w.terrain.set(float_of(v, "terrain")?);
    }
    if let Some(v) = opt(op, "slope") {
        w.slope.set(float_of(v, "slope")?);
    }
    if let Some(v) = opt(op, "traffic_map") {
        w.traffic_map.set(bool_of(v, "traffic_map")?);
    }
    if let Some(v) = opt(op, "crews_unregister") {
        w.crews_unregister.set(bool_of(v, "crews_unregister")?);
    }
    if let Some(v) = opt(op, "random") {
        match str_of(v, "random")? {
            "stub" => w.stub.set(true),
            "default" => w.stub.set(false),
            other => return Err(format!("unknown generator '{other}'")),
        }
    }
    if let Some(v) = opt(op, "session_seed") {
        w.session_seed.set(u32_of(v, "session_seed")?);
    }
    if let Some(v) = opt(op, "u32s") {
        for d in arr(v, "u32s")? {
            w.u32s.borrow_mut().push_back(u32_of(d, "u32")?);
        }
    }
    Ok(())
}

/// `net_occupant`: `occupant`, `flags`, `cell`, `in_city` (true),
/// `managed`, `lit`, `height`, `creation` (the token), `retrofit`, `flag4`,
/// `removable` (true), `piece`, `rotation`, `flip`, `intersection`,
/// `edges6`, `active_edges`; `remove: true` deletes it. Any field left out
/// keeps an existing occupant's value.
fn occupant(w: &World, op: &Json) -> R<()> {
    let t = token_of(field(op, "occupant")?, "occupant")?;
    if opt(op, "remove").and_then(boolean).unwrap_or(false) {
        w.occupants.borrow_mut().remove(&t);
        return Ok(());
    }
    let mut o = w.occ(t).unwrap_or(Occ {
        flags: 0,
        in_city: true,
        managed: false,
        lit: false,
        cell: (0, 0),
        height: 0.0,
        creation: t,
        retrofit: false,
        flag4: false,
        removable: true,
        piece: 0,
        rotation: 0,
        flip: 0,
        intersection: false,
        edges6: 0,
        active_edges: 0,
    });
    if let Some(v) = opt(op, "flags") {
        o.flags = flags_of(v)?;
    }
    if let Some(v) = opt(op, "cell") {
        o.cell = cell_of(v, "cell")?;
    }
    o.in_city = bool_or(op, "in_city", o.in_city)?;
    o.managed = bool_or(op, "managed", o.managed)?;
    o.lit = bool_or(op, "lit", o.lit)?;
    if let Some(v) = opt(op, "height") {
        o.height = float_of(v, "height")?;
    }
    if let Some(v) = opt(op, "creation") {
        o.creation = token_of(v, "creation")?;
    }
    o.retrofit = bool_or(op, "retrofit", o.retrofit)?;
    o.flag4 = bool_or(op, "flag4", o.flag4)?;
    o.removable = bool_or(op, "removable", o.removable)?;
    o.piece = u32_or(op, "piece", o.piece)?;
    if let Some(v) = opt(op, "rotation") {
        o.rotation = i32_of(v, "rotation")?;
    }
    if let Some(v) = opt(op, "flip") {
        o.flip = i32_of(v, "flip")?;
    }
    o.intersection = bool_or(op, "intersection", o.intersection)?;
    o.edges6 = u32_or(op, "edges6", o.edges6)?;
    o.active_edges = u32_or(op, "active_edges", o.active_edges)?;
    w.occupants.borrow_mut().insert(t, o);
    Ok(())
}

/// `net_zone`: `cells` (`[[x, z, zone]]`) or `clear: true`.
fn zone(w: &World, op: &Json) -> R<()> {
    if opt(op, "clear").and_then(boolean).unwrap_or(false) {
        w.zones.borrow_mut().clear();
    }
    if let Some(v) = opt(op, "cells") {
        for c in arr(v, "cells")? {
            match arr(c, "cells")? {
                [x, z, zone] => {
                    w.zones
                        .borrow_mut()
                        .insert((i32_of(x, "x")?, i32_of(z, "z")?), i32_of(zone, "zone")?);
                }
                _ => return Err("a zone cell is [x, z, zone]".into()),
            }
        }
    }
    Ok(())
}

/// `net_lot`: a lot `lot` covering `rect` (`[x1, z1, x2, z2]`, inclusive)
/// or `cell`, with `wealth` (0), `livable`, `building_type`; `profiles`
/// (`{"0xTYPE": wealth}`) sets building profiles; `clear: true` removes
/// every lot first.
fn lot(w: &World, op: &Json) -> R<()> {
    if opt(op, "clear").and_then(boolean).unwrap_or(false) {
        w.lot_cells.borrow_mut().clear();
        w.lot_data.borrow_mut().clear();
    }
    if let Some(v) = opt(op, "profiles") {
        for (k, wealth) in v.as_object().ok_or("profiles must be an object")? {
            let t = u32_of(&Json::Str(k.clone()), "profile")?;
            w.profiles.borrow_mut().insert(t, i32_of(wealth, k)?);
        }
    }
    let Some(t) = opt(op, "lot") else {
        return Ok(());
    };
    let t = token_of(t, "lot")?;
    let rect = match (opt(op, "rect"), opt(op, "cell")) {
        (Some(r), _) => rect_of(r, "rect")?,
        (None, Some(c)) => {
            let (x, z) = cell_of(c, "cell")?;
            CellRect {
                x1: x,
                z1: z,
                x2: x,
                z2: z,
            }
        }
        _ => return Err("a lot needs 'rect' or 'cell'".into()),
    };
    let l = Lot {
        wealth: opt(op, "wealth").map_or(Ok(0), |v| i32_of(v, "wealth"))?,
        livable: bool_or(op, "livable", false)?,
        building_type: u32_or(op, "building_type", 0)?,
        rect: (rect.x1, rect.z1, rect.x2, rect.z2),
    };
    for x in rect.x1..=rect.x2 {
        for z in rect.z1..=rect.z2 {
            w.lot_cells.borrow_mut().insert((x, z), t);
        }
    }
    w.lot_data.borrow_mut().insert(t, l);
    Ok(())
}

fn bool_list<const N: usize>(j: &Json, what: &str) -> R<[bool; N]> {
    let mut out = [false; N];
    for e in arr(j, what)? {
        let i = u32_of(e, what)? as usize;
        if i < N {
            out[i] = true;
        }
    }
    Ok(out)
}

/// `net_config`: a network lot configuration loaded at the lot manager's
/// `Init`: `id`, `piece`, `token?` (the ID), `zones?` / `wealths?` (allowed
/// values; absent: all), `mask?` (4 bytes), `period?`, `min_slope?`,
/// `max_slope?`, `rotations?`, `flip_rule?`, `objects?` (`[{kind: "prop" |
/// "flora" | n, flags?, orientation?, offset?: [x, y, z], field_24?, ids?:
/// [..]}]`). `clear: true` removes every configuration first.
fn config(w: &World, op: &Json) -> R<()> {
    if opt(op, "clear").and_then(boolean).unwrap_or(false) {
        w.configs.borrow_mut().clear();
        w.object_ids.borrow_mut().clear();
    }
    let Some(id) = opt(op, "id") else {
        return Ok(());
    };
    let id = u32_of(id, "id")?;
    let token = opt(op, "token").map_or(Ok(u64::from(id)), |v| token_of(v, "token"))?;
    let mut c = NetworkLotConfig {
        token,
        piece_id: u32_of(field(op, "piece")?, "piece")?,
        id,
        zones: [true; 16],
        wealths: [true; 4],
        ..NetworkLotConfig::default()
    };
    if let Some(v) = opt(op, "zones") {
        c.zones = bool_list(v, "zones")?;
    }
    if let Some(v) = opt(op, "wealths") {
        c.wealths = bool_list(v, "wealths")?;
    }
    if let Some(v) = opt(op, "mask") {
        let m = arr(v, "mask")?;
        if m.len() != 4 {
            return Err("'mask' must have 4 bytes".into());
        }
        for (i, b) in m.iter().enumerate() {
            c.mask[i] = u32_of(b, "mask")? as u8;
        }
    }
    c.period = u32_or(op, "period", u32::from(c.period))? as u8;
    if let Some(v) = opt(op, "min_slope") {
        c.min_slope = float_of(v, "min_slope")?;
    }
    if let Some(v) = opt(op, "max_slope") {
        c.max_slope = float_of(v, "max_slope")?;
    }
    c.rotations = u32_or(op, "rotations", u32::from(c.rotations))? as u8;
    c.flip_rule = u32_or(op, "flip_rule", u32::from(c.flip_rule))? as u8;
    if let Some(v) = opt(op, "objects") {
        for (i, o) in arr(v, "objects")?.iter().enumerate() {
            let kind = match field(o, "kind")?.as_str() {
                Some("prop") => OBJECT_PROP,
                Some("flora") => OBJECT_FLORA,
                _ => u32_of(field(o, "kind")?, "kind")? as u8,
            };
            let mut offset = [0.0f32; 3];
            if let Some(off) = opt(o, "offset") {
                let a = arr(off, "offset")?;
                if a.len() != 3 {
                    return Err("'offset' must be [x, y, z]".into());
                }
                for (k, e) in a.iter().enumerate() {
                    offset[k] = float_of(e, "offset")?;
                }
            }
            c.objects.push(NetworkLotObject {
                kind,
                flags: u32_or(o, "flags", 0)? as u8,
                orientation: u32_or(o, "orientation", 0)? as u8,
                offset,
                field_24: u32_or(o, "field_24", 0)?,
            });
            if let Some(ids) = opt(o, "ids") {
                let ids = arr(ids, "ids")?
                    .iter()
                    .map(|e| u32_of(e, "ids"))
                    .collect::<R<Vec<u32>>>()?;
                w.object_ids.borrow_mut().insert((token, i), ids);
            }
        }
    }
    w.configs.borrow_mut().push(c);
    Ok(())
}

/// `net_create`: new constructed managers (`of`: "manager", "lots" or
/// "both", the default), replacing the current ones.
fn create(w: &World, op: &Json) -> R<()> {
    let which = opt(op, "of").map_or(Ok("both"), |v| str_of(v, "of"))?;
    match which {
        "manager" => {
            w.new_mgr();
        }
        "lots" => {
            w.new_lots();
        }
        "both" => {
            w.new_mgr();
            w.new_lots();
        }
        other => return Err(format!("unknown manager '{other}'")),
    }
    Ok(())
}

fn lifecycle(w: &World, op: &Json, what: &str) -> R<()> {
    let got = if is_lots(op)? {
        let m = w.lots()?;
        match what {
            "init" => Json::Bool(m.init()),
            "shutdown" => Json::Bool(m.shutdown()),
            "add_ref" => ji(m.add_ref()),
            _ => ji(m.release()),
        }
    } else {
        let m = w.mgr()?;
        match what {
            "init" => Json::Bool(m.init()),
            "shutdown" => Json::Bool(m.shutdown()),
            "add_ref" => ji(m.add_ref()),
            _ => ji(m.release()),
        }
    };
    check_expect(op, got)
}

/// `net_identity`: `iids` (`{"0xIID": bool}`) and `class`, for `of`.
fn identity(w: &World, op: &Json) -> R<()> {
    let lots = is_lots(op)?;
    if let Some(v) = opt(op, "iids") {
        for (k, want) in v.as_object().ok_or("iids must be an object")? {
            let iid = u32_of(&Json::Str(k.clone()), "iid")?;
            let got = if lots {
                lot_manager_query_interface(iid)
            } else {
                manager_query_interface(iid)
            };
            if got != bool_of(want, k)? {
                return Err(format!("QueryInterface({k}) = {got}"));
            }
        }
    }
    if let Some(v) = opt(op, "class") {
        let want = u32_of(v, "class")?;
        let got = if lots {
            w.lots().map_or(CLSID_NETWORK_LOT_MANAGER, |m| m.class_id())
        } else {
            w.mgr().map_or(CLSID_NETWORK_MANAGER, |m| m.class_id())
        };
        if got != want {
            return Err(format!("class 0x{got:08X}, expected 0x{want:08X}"));
        }
    }
    Ok(())
}

/// `net_message`: `type` (a name or an ID), `data1`..`data4`, `occupant`
/// (void1), `unknown` (the IGZUnknown), `view` (`[old, new]`), `rect`
/// (`*data2` of the zoning message), for `of`.
fn message(w: &World, op: &Json) -> R<()> {
    let t = message_type(field(op, "type")?)?;
    let d = |k: &str| u32_or(op, k, 0);
    let void1 = opt(op, "occupant").map(|v| token_of(v, "occupant")).transpose()?;
    let got = if is_lots(op)? {
        let m = w.lots()?;
        let lm = NetworkLotMessage {
            msg_type: t,
            data1: d("data1")?,
            data2: d("data2")?,
            data3: d("data3")?,
            void1,
            unknown: opt(op, "unknown").map(|v| token_of(v, "unknown")).transpose()?,
            rect: opt(op, "rect").map(|v| rect_of(v, "rect")).transpose()?,
        };
        m.do_message(&lm)
    } else {
        let m = w.mgr()?;
        let (view_old, view_new) = match opt(op, "view") {
            Some(v) => {
                let (a, b) = cell_of(v, "view")?;
                (a as u32, b as u32)
            }
            None => (0, 0),
        };
        let nm = NetworkMessage {
            msg_type: t,
            data1: d("data1")?,
            data2: d("data2")?,
            data3: d("data3")?,
            data4: d("data4")?,
            void1,
            view_old,
            view_new,
        };
        m.do_message(&nm)
    };
    check_expect(op, Json::Bool(got))
}

fn tokens_of(j: &Json, what: &str) -> R<Vec<u64>> {
    arr(j, what)?.iter().map(|e| token_of(e, what)).collect()
}

fn lot_json(cell: (i32, i32), l: &LotEntry) -> Json {
    Json::Object(vec![
        ("cell".into(), cell_json(cell.0, cell.1)),
        ("occupant".into(), l.occupant.map_or(Json::Null, tok)),
        ("config".into(), l.config.as_ref().map_or(Json::Null, |c| ji(c.id))),
        ("config_id".into(), ji(l.config_id)),
        ("locked".into(), Json::Bool(l.locked)),
    ])
}

fn config_by_id(w: &World, id: u32) -> R<NetworkLotConfig> {
    w.configs
        .borrow()
        .iter()
        .find(|c| c.id == id)
        .cloned()
        .ok_or_else(|| format!("no configuration {id}"))
}

/// `net_call`: `method` with `args`; `expect` checks the result.
fn call(w: &World, op: &Json) -> R<()> {
    let a = |i: usize| arg(op, i);
    let ai = |i: usize| -> R<i32> { i32_of(arg(op, i)?, "arg") };
    let au = |i: usize| -> R<u32> { u32_of(arg(op, i)?, "arg") };
    let method = str_of(field(op, "method")?, "method")?;
    let got = match method {
        // ---- the network manager ----
        "get_network_tool" => {
            let t = w.mgr()?.get_network_tool(au(0)?, bool_of(a(1)?, "unique")?);
            t.map_or(Json::Null, |t| {
                Json::Object(vec![("tool".into(), tok(t)), ("type".into(), w.tool_type(t))])
            })
        }
        "destroy_network_occupants" => {
            w.mgr()?.destroy_network_occupants(&tokens_of(a(0)?, "occupants")?);
            Json::Null
        }
        "queue_removals" => {
            w.mgr()?.queue_removals(bool_of(a(0)?, "on")?);
            Json::Null
        }
        "remove_or_replace_occupant" => {
            let tool = token_of(a(0)?, "tool")?;
            let occ = match a(1)? {
                Json::Null => None,
                v => Some(token_of(v, "occupant")?),
            };
            let (ok, replaced) = w.mgr()?.remove_or_replace_occupant(tool, occ);
            Json::Array(vec![Json::Bool(ok), Json::Bool(replaced)])
        }
        "register_construction_crew" => {
            w.mgr()?.register_construction_crew(token_of(a(0)?, "crew")?);
            Json::Null
        }
        "unregister_construction_crew" => {
            w.mgr()?.unregister_construction_crew(token_of(a(0)?, "crew")?);
            Json::Null
        }
        "add_to_retrofit_list" => {
            w.mgr()?.add_to_retrofit_list(token_of(a(0)?, "occupant")?);
            Json::Null
        }
        "terrain_rect" => rect_json(&terrain_rect(au(0)?, au(1)?, CITY)),
        "corner_mismatch" => Json::Bool(corner_mismatch(float_of(a(0)?, "d")?)),
        "set_network_flag" | "clear_network_flag" => {
            let flags = flags_of(a(0)?)?;
            let bits = flags_of(a(1)?)?;
            let c = if method == "set_network_flag" {
                set_network_flag(flags, bits, bool_of(a(2)?, "in_city")?)
            } else {
                clear_network_flag(flags, bits)
            };
            Json::Object(vec![
                ("flags".into(), ji(c.flags)),
                ("usable_post".into(), c.usable_post.map_or(Json::Null, ji)),
                ("translucency".into(), Json::Bool(c.translucency)),
            ])
        }
        // ---- the network lot manager ----
        "zone_type_for_network" => ji(w.lots()?.zone_type_for_network(ai(0)?, ai(1)?)),
        "wealth_for_network" => ji(w.lots()?.wealth_for_network(ai(0)?, ai(1)?)),
        "is_lot_compatible" => {
            // [config id, x, z, zone, wealth, rotation, flip]
            let c = config_by_id(w, au(0)?)?;
            Json::Bool(
                w.lots()?
                    .is_lot_compatible(&c, ai(1)?, ai(2)?, ai(3)?, ai(4)?, ai(5)?, ai(6)?),
            )
        }
        "object_position" => {
            // [[x, z], rotation, flipped, o, [ox, oy, oz]]
            let cell = cell_of(a(0)?, "cell")?;
            let flip = bool_of(a(2)?, "flip")?;
            let off = arr(a(4)?, "offset")?;
            if off.len() != 3 {
                return Err("offset must be [x, y, z]".into());
            }
            let offset = [
                float_of(&off[0], "offset")?,
                float_of(&off[1], "offset")?,
                float_of(&off[2], "offset")?,
            ];
            let terrain = w.terrain.get();
            let (p, o) = object_position(cell, ai(1)?, flip, au(3)? as u8, offset, |_, _| terrain);
            Json::Object(vec![("pos".into(), pos_json(p)), ("orientation".into(), ji(o))])
        }
        "update_roads_in_cell" => {
            w.lots()?.update_roads_in_cell(ai(0)?, ai(1)?);
            Json::Null
        }
        "update_road_style" => {
            w.lots()?
                .update_road_style(ai(0)?, ai(1)?, token_of(a(2)?, "occupant")?);
            Json::Null
        }
        "add_pending" => {
            w.lots()?.add_pending(ai(0)?, ai(1)?);
            Json::Null
        }
        "add_border_to_pending" => {
            w.lots()?.add_border_to_pending(rect_of(a(0)?, "rect")?);
            Json::Null
        }
        "process_update_list" => {
            w.lots()?.process_update_list();
            Json::Null
        }
        "set_lot_configuration_for_next_piece" => {
            let c = match a(0)? {
                Json::Null => None,
                v => Some(Rc::new(config_by_id(w, u32_of(v, "config")?)?)),
            };
            w.lots()?.set_lot_configuration_for_next_piece(c);
            Json::Null
        }
        "redevelop_lot" => {
            let m = w.lots()?;
            let (x, z) = (ai(0)?, ai(1)?);
            let l = m.get_network_lot(x, z).ok_or("no network lot there")?;
            m.redevelop_lot(x, z, &l);
            Json::Null
        }
        "do_road_damage_change" => {
            w.lots()?
                .do_road_damage_change(ai(0)?, ai(1)?, bool_of(a(2)?, "damaged")?);
            Json::Null
        }
        "get_network_lot" => {
            let (x, z) = (ai(0)?, ai(1)?);
            w.lots()?
                .get_network_lot(x, z)
                .map_or(Json::Null, |l| lot_json((x, z), &l))
        }
        "configuration_by_id" => Json::Bool(w.lots()?.configuration_by_id(au(0)?)),
        "pothole_offset" => {
            let (dx, dz) = pothole_offset(au(0)?);
            Json::Array(vec![ji(dx), ji(dz)])
        }
        "road_texture" => ji(road_texture(ai(0)?, ai(1)?)),
        "road_variation" => ji(road_variation(ai(0)?, ai(1)?)),
        other => return Err(format!("unknown network method '{other}'")),
    };
    check_expect(op, got)
}

/// `net_riot`: `action` = "add_instance", "remove_instance", "shutdown" or
/// "mark" (`cell`); `can_remove` (`{"0xOCC": [level, bool]}`) checks
/// `CanRemove` afterwards.
fn riot(w: &World, op: &Json) -> R<()> {
    if let Some(a) = opt(op, "action") {
        let mut lock = w.riot.borrow().clone();
        match str_of(a, "action")? {
            "add_instance" => lock.add_instance(),
            "remove_instance" => lock.remove_instance(w),
            "shutdown" => lock.shutdown(w),
            "mark" => {
                let (x, z) = cell_of(field(op, "cell")?, "cell")?;
                lock.mark(w, x, z);
            }
            other => return Err(format!("unknown riot action '{other}'")),
        }
        *w.riot.borrow_mut() = lock;
    }
    if let Some(v) = opt(op, "can_remove") {
        for (k, e) in v.as_object().ok_or("can_remove must be an object")? {
            let occ = token_of(&Json::Str(k.clone()), "occupant")?;
            let (level, want) = match arr(e, k)? {
                [l, b] => (u32_of(l, "level")?, bool_of(b, "can_remove")?),
                _ => return Err("can_remove entries are [level, bool]".into()),
            };
            let got = w
                .counters
                .borrow()
                .get(&occ)
                .copied()
                .unwrap_or_default()
                .can_remove(level);
            if got != want {
                return Err(format!("CanRemove({level}) of {k} = {got}"));
            }
        }
    }
    Ok(())
}

/// `net_expect`: `state` of `of`.
fn expect(w: &World, op: &Json) -> R<()> {
    let got = if is_lots(op)? {
        let s = w.lots()?.snapshot();
        let mut ids: Vec<u32> = s.configs.iter().flat_map(|(_, v)| v.iter().map(|c| c.id)).collect();
        ids.sort_unstable();
        vec![
            ("initialized".to_owned(), Json::Bool(s.initialized)),
            ("destroyed".to_owned(), Json::Bool(s.destroyed)),
            ("ref_count".to_owned(), ji(s.ref_count)),
            (
                "lots".to_owned(),
                Json::Array(
                    s.lots
                        .iter()
                        .map(|(k, l)| lot_json(fc4_core::network_manager::key_cell(*k), l))
                        .collect(),
                ),
            ),
            ("lot_count".to_owned(), ji(s.lots.len() as i64)),
            (
                "potholes".to_owned(),
                Json::Array(
                    s.potholes
                        .iter()
                        .map(|(k, p)| {
                            let (x, z) = fc4_core::network_manager::key_cell(*k);
                            Json::Object(vec![
                                ("cell".into(), cell_json(x, z)),
                                ("prop".into(), p.map_or(Json::Null, tok)),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "pending".to_owned(),
                Json::Array(
                    s.pending
                        .iter()
                        .map(|&k| {
                            let (x, z) = fc4_core::network_manager::key_cell(k);
                            cell_json(x, z)
                        })
                        .collect(),
                ),
            ),
            ("pending_count".to_owned(), ji(s.pending.len() as i64)),
            ("forced".to_owned(), s.forced.as_ref().map_or(Json::Null, |c| ji(c.id))),
            ("configs".to_owned(), Json::Array(ids.into_iter().map(ji).collect())),
        ]
    } else {
        let s = w.mgr()?.snapshot();
        vec![
            ("initialized".to_owned(), Json::Bool(s.initialized)),
            ("destroyed".to_owned(), Json::Bool(s.destroyed)),
            ("ref_count".to_owned(), ji(s.ref_count)),
            ("uk_paths".to_owned(), Json::Bool(s.uk_paths)),
            ("long_terrain_op".to_owned(), Json::Bool(s.long_terrain_op)),
            (
                "dirty".to_owned(),
                if rect_is_empty(&s.dirty) {
                    Json::Null
                } else {
                    rect_json(&s.dirty)
                },
            ),
            ("managed".to_owned(), opt_tokens_json(&s.managed)),
            ("pending".to_owned(), tokens_json(&s.pending)),
            ("queueing".to_owned(), Json::Bool(s.queueing)),
            ("retrofit".to_owned(), tokens_json(&s.retrofit)),
            ("crews".to_owned(), tokens_json(&s.crews)),
            (
                "tools".to_owned(),
                Json::Array(
                    s.tools
                        .iter()
                        .map(|t| t.map_or(Json::Null, |t| w.tool_type(t)))
                        .collect(),
                ),
            ),
            ("riot_locked".to_owned(), tokens_json(&w.riot.borrow().locked())),
            ("riot_instances".to_owned(), ji(w.riot.borrow().instances())),
        ]
    };
    check_state(&got, field(op, "state")?)
}

/// `net_expect_lot`: the network lot at `cell`: `state` (partial:
/// `occupant`, `config`, `config_id`, `locked`) or `absent: true`.
fn expect_lot(w: &World, op: &Json) -> R<()> {
    let (x, z) = cell_of(field(op, "cell")?, "cell")?;
    let l = w.lots()?.snapshot().lots.get(&lot_key(x, z)).cloned();
    if opt(op, "absent").and_then(boolean).unwrap_or(false) {
        return match l {
            None => Ok(()),
            Some(_) => Err(format!("a network lot at ({x}, {z})")),
        };
    }
    let l = l.ok_or_else(|| format!("no network lot at ({x}, {z})"))?;
    let got = vec![
        ("occupant".to_owned(), l.occupant.map_or(Json::Null, tok)),
        ("config".to_owned(), l.config.as_ref().map_or(Json::Null, |c| ji(c.id))),
        ("config_id".to_owned(), ji(l.config_id)),
        ("locked".to_owned(), Json::Bool(l.locked)),
    ];
    check_state(&got, field(op, "state")?)
}

/// `net_expect_refs`: `{"token": references held}`.
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

// ---- the records (§4) ---------------------------------------------------------------------------------

/// Resolves references like the class-object store: a registered object
/// comes back as its token, an object ID of a hand-made record as itself.
/// The reference is handed to the core's `resolve_ref`.
struct Resolver(Vec<RegisteredObject>);

impl RefResolver for Resolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        let token = match self.0.iter().find(|e| e.0 == class_id && e.1 == object_id) {
            Some(e) => e.3,
            None if self.0.is_empty() => u64::from(object_id),
            None => return None,
        };
        Some(RefValue::Host { token, class_id })
    }
}

fn save_object(w: &World, op: &Json) -> R<Rc<dyn SaveObject>> {
    Ok(if is_lots(op)? {
        w.lots()? as Rc<dyn SaveObject>
    } else {
        w.mgr()? as Rc<dyn SaveObject>
    })
}

/// `net_write`: the record of `of` into `slot`; `expect`, `length`, `hex`.
fn write(w: &World, op: &Json) -> R<()> {
    let m = save_object(w, op)?;
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut out = ByteWriter::with_registry(reg.clone());
    check_expect(op, Json::Bool(m.write(&mut out)))?;
    let bytes = out.into_bytes();
    if let Some(v) = opt(op, "length") {
        if bytes.len() as i64 != i64_of(v, "length")? {
            return Err(format!("record is {} bytes: {}", bytes.len(), hex_encode(&bytes)));
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
    if let Some(v) = opt(op, "registered") {
        let want = tokens_of(v, "registered")?;
        let got: Vec<u64> = entries.iter().map(|e| e.3).collect();
        if got != want {
            return Err(format!("registered {got:X?}, expected {want:X?}"));
        }
    }
    if let Some(slot) = opt(op, "slot") {
        w.records
            .borrow_mut()
            .insert(str_of(slot, "slot")?.to_string(), Record { bytes, entries });
    }
    Ok(())
}

/// `net_read`: a slot's record (or `hex`, whose references resolve to their
/// object IDs), optionally cut to `length`, into a new constructed manager
/// of `of` (or the current one with `reuse`); `error: true` puts the stream
/// in its error state first; `position` checks the bytes consumed.
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
    if let Some(n) = opt(op, "length") {
        rec.bytes.truncate(i64_of(n, "length")? as usize);
    }
    let reuse = opt(op, "reuse").and_then(boolean).unwrap_or(false);
    let m: Rc<dyn SaveObject> = match (is_lots(op)?, reuse) {
        (true, true) => w.lots()?,
        (true, false) => w.new_lots(),
        (false, true) => w.mgr()?,
        (false, false) => w.new_mgr(),
    };
    let mut res = Resolver(rec.entries.clone());
    let mut r = ByteReader::with_resolver(&rec.bytes, &mut res);
    if opt(op, "error").and_then(boolean).unwrap_or(false) {
        fc4_core::stream::InStream::set_error(&mut r);
    }
    let ok = m.read(&mut r);
    if let Some(p) = opt(op, "position") {
        let got = r.position() as i64;
        if got != i64_of(p, "position")? {
            return Err(format!("read {got} bytes"));
        }
    }
    check_expect(op, Json::Bool(ok))
}

/// `net_expect_record`: `same_as` (another slot), `length`.
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
    Ok(())
}
