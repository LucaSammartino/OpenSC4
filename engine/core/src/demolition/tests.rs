// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests of the demolition utility against a fake city: the §12 tests
//! of `docs/specs/demolition.md` (named `dm12_01_…` to `dm12_13_…`), the
//! §4.10 worked examples, the filter, the lifecycle and the record.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use super::*;
use crate::stream::{ByteReader, ByteWriter, SaveObject};

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

impl Occ {
    /// An occupant of `ty` centred in cell (x, z), in the occupant manager.
    fn at(ty: u32, x: i32, z: i32) -> Occ {
        let (cx, cz) = (16.0 * x as f32 + 8.0, 16.0 * z as f32 + 8.0);
        Occ {
            ty,
            pos: (cx, 0.0, cz),
            bounds: (cx - 4.0, cz - 4.0, cx + 4.0, cz + 4.0),
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

impl Lot {
    fn new(x1: i32, z1: i32, x2: i32, z2: i32, zone: u32) -> Lot {
        Lot {
            rect: CellRect::new(x1, z1, x2, z2),
            state: 1,
            building: None,
            occs: Vec::new(),
            demolishable: true,
            zone,
            water: Some(false),
            locked: false,
            initialized: true,
            facing: 0,
            condition: 0,
        }
    }
}

struct World {
    log: RefCell<Vec<String>>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    lots: RefCell<BTreeMap<u64, Lot>>,
    zones: RefCell<HashMap<(i32, i32), u32>>,
    alt: RefCell<HashMap<(i32, i32), f32>>,
    water: RefCell<HashSet<(i32, i32)>>,
    bds: Cell<bool>,
    flora: Cell<bool>,
    prop_mgr: Cell<bool>,
    view_filter: Cell<Option<u64>>,
    view_level: Cell<Option<u32>>,
    /// Filter token → the occupants it rejects.
    rejects: RefCell<HashMap<u64, HashSet<u64>>>,
    rejects_building_type: RefCell<HashSet<u64>>,
    thresholds: RefCell<Option<Rc<PropertyHolder>>>,
    bd: RefCell<Option<Rc<PropertyHolder>>>,
    next_lot: Cell<u64>,
}

impl World {
    fn new() -> Rc<World> {
        Rc::new(World {
            log: RefCell::new(Vec::new()),
            occs: RefCell::new(BTreeMap::new()),
            lots: RefCell::new(BTreeMap::new()),
            zones: RefCell::new(HashMap::new()),
            alt: RefCell::new(HashMap::new()),
            water: RefCell::new(HashSet::new()),
            bds: Cell::new(true),
            flora: Cell::new(true),
            prop_mgr: Cell::new(true),
            view_filter: Cell::new(None),
            view_level: Cell::new(Some(1)),
            rejects: RefCell::new(HashMap::new()),
            rejects_building_type: RefCell::new(HashSet::new()),
            thresholds: RefCell::new(Some(Rc::new(shipped_thresholds()))),
            bd: RefCell::new(Some(Rc::new(shipped_building_development()))),
            next_lot: Cell::new(0x9000),
        })
    }
    fn ev(&self, s: impl Into<String>) {
        self.log.borrow_mut().push(s.into());
    }
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
    fn occ(&self, t: u64, o: Occ) {
        self.occs.borrow_mut().insert(t, o);
    }
    fn lot(&self, t: u64, l: Lot) {
        for x in l.rect.x1..=l.rect.x2 {
            for z in l.rect.z1..=l.rect.z2 {
                self.zones.borrow_mut().insert((x, z), l.zone);
            }
        }
        self.lots.borrow_mut().insert(t, l);
    }
    fn o<R>(&self, t: u64, f: impl FnOnce(&Occ) -> R) -> R {
        f(&self.occs.borrow()[&t])
    }
    fn l<R>(&self, t: u64, f: impl FnOnce(&Lot) -> R) -> R {
        f(&self.lots.borrow()[&t])
    }
}

impl DemolitionHost for World {
    fn subscribe(&self, m: u32) {
        self.ev(format!("subscribe {m:08X}"));
    }
    fn unsubscribe(&self, m: u32) {
        self.ev(format!("unsubscribe {m:08X}"));
    }
    fn acquire_city(&self) -> bool {
        self.ev("acquire city");
        true
    }
    fn release_city(&self) {
        self.ev("release city");
    }
    fn city_cell_counts(&self) -> (i32, i32) {
        (64, 64)
    }
    fn acquire_occupant_manager(&self) -> bool {
        self.ev("acquire occupant manager");
        true
    }
    fn release_occupant_manager(&self) {
        self.ev("release occupant manager");
    }
    fn acquire_lot_developer(&self) -> bool {
        self.ev("acquire lot developer");
        true
    }
    fn release_lot_developer(&self) {
        self.ev("release lot developer");
    }
    fn acquire_terrain(&self) -> bool {
        self.ev("acquire terrain");
        true
    }
    fn release_terrain(&self) {
        self.ev("release terrain");
    }
    fn thresholds_exemplar(&self) -> Option<Rc<PropertyHolder>> {
        self.thresholds.borrow().clone()
    }
    fn building_development_exemplar(&self) -> Option<Rc<PropertyHolder>> {
        self.bd.borrow().clone()
    }
    fn release(&self, t: u64) {
        self.ev(format!("release {t:#x}"));
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
        !(t == occ_type::BUILDING && self.rejects_building_type.borrow().contains(&f))
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
        self.ev(format!("shutdown {o:#x}"));
    }
    fn remove_occupant(&self, o: u64, b: bool) -> bool {
        let mut occs = self.occs.borrow_mut();
        let x = occs.get_mut(&o).unwrap();
        if x.flags & OCCUPANT_FLAG_IN_MANAGER == 0 {
            return false;
        }
        x.flags &= !OCCUPANT_FLAG_IN_MANAGER;
        drop(occs);
        self.ev(format!("remove {o:#x} {b}"));
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
        let (x0, z0, x1, z1) = (
            16.0 * r.x1 as f32,
            16.0 * r.z1 as f32,
            16.0 * (r.x2 + 1) as f32,
            16.0 * (r.z2 + 1) as f32,
        );
        self.occupants_in_box(x0, z0, x1, z1)
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
        let mut occs = self.occs.borrow_mut();
        for c in conns {
            occs.get_mut(&c).unwrap().connections.retain(|&x| x != p);
        }
        occs.get_mut(&p).unwrap().connections.clear();
        drop(occs);
        self.ev(format!("break {p:#x}"));
    }
    fn send_pre_demolish(&self, o: u64) {
        self.ev(format!("pre_demolish {o:#x}"));
    }
    fn has_prop_manager(&self) -> bool {
        self.prop_mgr.get()
    }
    fn remove_prop(&self, o: u64) -> bool {
        self.ev(format!("remove_prop {o:#x}"));
        true
    }
    fn list_append(&self, list: u64, o: u64) {
        self.ev(format!("append {list:#x} {o:#x}"));
    }
    fn set_insert(&self, set: u64, o: u64) {
        self.ev(format!("insert {set:#x} {o:#x}"));
    }
    fn queue_removals(&self, on: bool) {
        self.ev(format!("queue {on}"));
    }
    fn delete_occupant_connection(&self, o: u64) {
        self.ev(format!("delete_connection {o:#x}"));
    }
    fn destroy_network_occupants(&self, list: &[u64]) {
        self.ev(format!("destroy {list:x?}"));
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
        self.ev(format!("explode {l:#x} {b}"));
    }
    fn start_demolish_lot(&self, l: u64) {
        self.ev(format!("start {l:#x}"));
    }
    fn end_demolish_lot(&self, l: u64) {
        let mut lots = self.lots.borrow_mut();
        let lot = lots.get_mut(&l).unwrap();
        lot.state = 1;
        lot.building = None;
        drop(lots);
        self.ev(format!("end {l:#x}"));
    }
    fn delete_lot(&self, l: u64) {
        self.lots.borrow_mut().remove(&l);
        self.ev(format!("delete {l:#x}"));
    }
    fn create_lot_if_possible(&self, x: i32, z: i32, facing: u32) -> bool {
        let zone = self.zone_at(x, z);
        let t = self.next_lot.get();
        self.next_lot.set(t + 1);
        let mut l = Lot::new(x, z, x, z, zone);
        l.facing = facing;
        self.lots.borrow_mut().insert(t, l);
        self.ev(format!("create {x} {z} {facing}"));
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
        self.ev(format!("dezone {} {} {} {}", r.x1, r.z1, r.x2, r.z2));
    }
    fn altitude_at_vertex(&self, x: i32, z: i32) -> f32 {
        self.alt.borrow().get(&(x, z)).copied().unwrap_or(0.0)
    }
    fn is_water_vertex(&self, x: i32, z: i32) -> bool {
        self.water.borrow().contains(&(x, z))
    }
    fn set_altitude_rect(&self, r: CellRect, h: f32, reason: u32) {
        self.ev(format!("set_rect {} {} {} {} {h} {reason}", r.x1, r.z1, r.x2, r.z2));
    }
    fn set_altitude_vertex(&self, x: i32, z: i32, h: f32) {
        self.alt.borrow_mut().insert((x, z), h);
        self.ev(format!("set_vertex {x} {z} {h}"));
    }
    fn redisplay_terrain(&self, r: CellRect, reason: u32) {
        self.ev(format!("redisplay {} {} {} {} {reason}", r.x1, r.z1, r.x2, r.z2));
    }
    fn effect_extent(&self, o: u64) -> Option<f32> {
        self.o(o, |o| o.extent)
    }
    fn play_occupant_sound(&self, o: u64) {
        self.ev(format!("sound {o:#x}"));
    }
    fn start_occupant_effect(&self, o: u64, keys: &[u32], ray: Option<(i32, i32)>, grey: bool) {
        self.ev(format!("effect {o:#x} {keys:x?} {ray:?} {grey}"));
    }
    fn start_group_effect(&self, instance: u32, x: i32, z: i32) {
        self.ev(format!("group {instance:#x} {x} {z}"));
    }
}

fn setup() -> (Rc<World>, Rc<DemolitionUtility>) {
    let w = World::new();
    let d = DemolitionUtility::new(w.clone(), 0xD0);
    assert!(d.init());
    w.take();
    (w, d)
}

fn region(x1: i32, z1: i32, x2: i32, z2: i32) -> CellRegion {
    CellRegion::new(CellRect::new(x1, z1, x2, z2), true)
}

// ---- identity and lifecycle -------------------------------------------------------------------

#[test]
fn identity_and_lifecycle() {
    assert_eq!(query_interface(1), Some(0));
    assert_eq!(query_interface(IID_DEMOLITION), Some(0));
    assert_eq!(query_interface(IID_MESSAGE_TARGET2), Some(4));
    assert_eq!(query_interface(IID_SERIALIZABLE), Some(8));
    assert_eq!(query_interface(0xE98B_2F57), None);
    let w = World::new();
    let d = DemolitionUtility::new(w.clone(), 0xD0);
    assert_eq!(d.class_id(), CLSID_DEMOLITION);
    assert!(d.shutdown());
    assert!(w.take().is_empty());
    assert!(d.init());
    assert_eq!(
        w.take(),
        [
            "subscribe C53D10AA",
            "subscribe 26D31EC1",
            "subscribe A9DED179",
            "acquire city",
            "acquire occupant manager",
            "acquire lot developer",
            "acquire terrain"
        ]
    );
    let s = d.snapshot();
    assert!(s.initialized && s.filter_created);
    assert_eq!((s.cells_x, s.cells_z), (64, 64));
    assert_eq!(s.tuning, DemolitionTuning::SHIPPED);
    assert!(d.init());
    assert!(w.take().is_empty());
    assert_eq!(d.add_ref(), 1);
    assert_eq!(d.add_ref(), 2);
    assert_eq!(d.release(), 1);
    assert_eq!(d.release(), 0);
    assert_eq!(
        w.take(),
        [
            "release terrain",
            "release lot developer",
            "release occupant manager",
            "release city",
            "unsubscribe C53D10AA",
            "unsubscribe 26D31EC1",
            "unsubscribe A9DED179"
        ]
    );
    assert!(d.snapshot().destroyed);
}

#[test]
fn tuning_defaults_and_exemplar_changed() {
    let w = World::new();
    *w.thresholds.borrow_mut() = None;
    *w.bd.borrow_mut() = None;
    let d = DemolitionUtility::new(w.clone(), 0xD0);
    d.init();
    assert_eq!(d.snapshot().tuning, DemolitionTuning::DEFAULT);
    // [quirk] Any exemplar re-reads the tuning.
    *w.bd.borrow_mut() = Some(Rc::new(shipped_building_development()));
    assert!(d.do_message(&DemolitionMessage {
        msg_type: msg::EXEMPLAR_CHANGED,
        data1: 1,
        data2: 2,
        ..Default::default()
    }));
    let t = d.snapshot().tuning;
    assert_eq!(t.destroyed_tile_cost, 2);
    assert_eq!(t.small_medium_model, 64);
    assert!(d.do_message(&DemolitionMessage::default_of(msg::POST_CITY_INIT)));
}

impl DemolitionMessage {
    fn default_of(t: u32) -> DemolitionMessage {
        DemolitionMessage {
            msg_type: t,
            ..Default::default()
        }
    }
}

#[test]
fn deferred_lot_demolition() {
    let (w, d) = setup();
    let mut l = Lot::new(3, 4, 4, 4, 15);
    l.state = 2;
    w.lot(0x50, l);
    assert!(d.do_message(&DemolitionMessage {
        msg_type: msg::DEFERRED_LOT_DEMOLITION,
        lot: Some(0x50),
        dezone: true,
        ..Default::default()
    }));
    assert_eq!(w.take(), ["end 0x50", "dezone 3 4 4 4", "release 0x50"]);
    w.lots.borrow_mut().get_mut(&0x50).unwrap().initialized = false;
    d.deferred_lot_demolition(Some(0x50), true);
    assert_eq!(w.take(), ["release 0x50"]);
    d.deferred_lot_demolition(None, true);
    assert!(w.take().is_empty());
}

// ---- §12 --------------------------------------------------------------------------------------

#[test]
fn dm12_01_record() {
    let (_, d) = setup();
    let mut out = ByteWriter::new();
    assert!(d.write(&mut out));
    assert_eq!(out.bytes(), [0x01, 0x00]);
    assert!(d.read(&mut ByteReader::new(&[0x01, 0x00])));
    assert!(!d.read(&mut ByteReader::new(&[0x02, 0x00])));
    assert!(!d.read(&mut ByteReader::new(&[0x01])));
}

#[test]
fn dm12_02_empty_list() {
    let (w, d) = setup();
    let r = d.demolish_lots(true, &[], 0, 1, false, None, None);
    assert_eq!(r, Outcome { ok: true, cost: None });
    let r = d.demolish_lots(true, &[None], 0, 1, false, None, None);
    assert_eq!(r, Outcome { ok: true, cost: None });
    assert!(w.take().is_empty());
}

#[test]
fn dm12_03_building_price() {
    let (w, d) = setup();
    let mut b = Occ::at(occ_type::BUILDING, 5, 5);
    b.cost = 1234;
    w.occ(0x10, b);
    let mut l = Lot::new(5, 5, 5, 5, 1);
    l.state = 3;
    l.building = Some(0x10);
    l.occs = vec![0x10];
    w.lot(0x50, l);
    let r = d.demolish_lots(false, &[Some(0x50)], 0, 1, false, None, None);
    assert_eq!(
        r,
        Outcome {
            ok: true,
            cost: Some(1234)
        }
    );
    assert!(w.take().is_empty(), "nothing removed");
    // No building-development simulator: the building fails, the region does not.
    w.bds.set(false);
    let r = d.demolish_lots(false, &[Some(0x50)], 0, 1, false, None, None);
    assert_eq!(
        r,
        Outcome {
            ok: true,
            cost: Some(0)
        }
    );
}

#[test]
fn dm12_04_destroyed_lot() {
    let (w, d) = setup();
    let mut l = Lot::new(10, 10, 12, 11, 1);
    l.state = LOT_STATE_DESTROYED;
    w.lot(0x50, l);
    let r = d.demolish_lots(false, &[Some(0x50)], 0, 1, false, None, None);
    assert_eq!(
        r,
        Outcome {
            ok: true,
            cost: Some(12)
        }
    );
    assert!(w.take().is_empty());
    let r = d.demolish_lots(true, &[Some(0x50)], 1, 1, false, None, None);
    assert_eq!(
        r,
        Outcome {
            ok: true,
            cost: Some(12)
        }
    );
    assert_eq!(w.take(), ["queue true", "start 0x50", "end 0x50", "queue false"]);
}

#[test]
fn dm12_05_not_demolishable() {
    let (w, d) = setup();
    let mut b = Occ::at(occ_type::BUILDING, 5, 5);
    b.cost = 99;
    w.occ(0x10, b);
    let mut l = Lot::new(5, 5, 5, 5, 1);
    l.building = Some(0x10);
    l.occs = vec![0x10];
    l.demolishable = false;
    w.lot(0x50, l);
    let r = d.demolish_lots(true, &[Some(0x50)], 0, 1, false, None, None);
    assert_eq!(
        r,
        Outcome {
            ok: true,
            cost: Some(0)
        }
    );
    assert_eq!(w.take(), ["queue true", "queue false"]);
}

#[test]
fn dm12_06_blocked_lot() {
    let (w, d) = setup();
    let mut b = Occ::at(occ_type::BUILDING, 5, 5);
    b.player_lock = true;
    w.occ(0x10, b);
    let mut l = Lot::new(5, 5, 5, 5, 1);
    l.state = 3;
    l.building = Some(0x10);
    l.occs = vec![0x10];
    w.lot(0x50, l);
    w.lot(0x51, Lot::new(6, 5, 6, 5, 1));
    let r = d.demolish_region(true, &region(5, 5, 6, 5), 1, 1, false, None, None, None, 0, 0);
    assert!(!r.ok);
    let log = w.take();
    assert!(log.contains(&"start 0x51".to_string()));
    assert!(!log.contains(&"start 0x50".to_string()));
    // With the system privilege the player lock does not count.
    let r = d.demolish_region(false, &region(5, 5, 5, 5), 0, 1, false, None, None, None, 0, 0);
    assert!(r.ok);
    // A prop with the system lock blocks the whole lot for the system.
    let mut p = Occ::at(occ_type::PROP, 6, 5);
    p.system_lock = true;
    w.occ(0x11, p);
    w.lots.borrow_mut().get_mut(&0x51).unwrap().occs = vec![0x11];
    let r = d.demolish_region(true, &region(6, 5, 6, 5), 0, 1, false, None, None, None, 0, 0);
    assert!(!r.ok);
    assert_eq!(w.take(), ["queue true", "queue false"]);
}

#[test]
fn dm12_07_reparcel() {
    let (w, d) = setup();
    let mut l = Lot::new(20, 30, 21, 31, 8);
    l.facing = 2;
    w.lot(0x50, l);
    let r = d.demolish_region(true, &region(20, 30, 21, 31), 1, 1, false, None, None, None, 0, 0);
    assert!(r.ok);
    assert_eq!(
        w.take(),
        [
            "queue true",
            "start 0x50",
            "end 0x50",
            "delete 0x50",
            "create 20 30 2",
            "create 20 31 2",
            "create 21 30 2",
            "create 21 31 2",
            "queue false"
        ]
    );
    // 0x801: the lot stays (state 1).
    w.lot(0x60, Lot::new(40, 40, 41, 41, 8));
    let r = d.demolish_region(true, &region(40, 40, 41, 41), 1, 0x801, false, None, None, None, 0, 0);
    assert!(r.ok);
    let log = w.take();
    assert!(!log.iter().any(|e| e.starts_with("delete") || e.starts_with("create")));
    assert_eq!(w.lot_state(0x60), 1);
    // §4.10 example 4: only the cells that still hold zone 7 are re-parcelled.
    w.lot(0x70, Lot::new(50, 50, 52, 50, 7));
    w.zones.borrow_mut().insert((50, 50), 1);
    d.demolish_lots(true, &[Some(0x70)], 0, 0x2000, false, None, None);
    let creates: Vec<String> = w.take().into_iter().filter(|e| e.starts_with("create")).collect();
    assert_eq!(creates, ["create 51 50 0", "create 52 50 0"]);
}

#[test]
fn dm12_08_dezone() {
    let (w, d) = setup();
    w.lot(0x50, Lot::new(1, 1, 2, 2, 1));
    d.demolish_region(true, &region(1, 1, 2, 2), 1, 1, true, None, None, None, 0, 0);
    assert!(w.take().contains(&"dezone 1 1 2 2".to_string()));
    assert_eq!(w.zone_at(2, 2), 0);
    for zone in [ZONE_AIRPORT, ZONE_SEAPORT, ZONE_PLOPPED] {
        w.lot(0x60, Lot::new(5, 5, 5, 5, zone));
        d.demolish_region(true, &region(5, 5, 5, 5), 1, 1, false, None, None, None, 0, 0);
        assert!(w.take().contains(&"dezone 5 5 5 5".to_string()));
        assert_eq!(w.zone_at(5, 5), 0);
    }
}

#[test]
fn dm12_09_explode() {
    let (w, d) = setup();
    w.lot(0x50, Lot::new(1, 1, 1, 1, 1));
    d.demolish_lots(true, &[Some(0x50)], 0, 0x303, false, None, None);
    let log = w.take();
    assert!(log.contains(&"explode 0x50 true".to_string()));
    assert!(!log.iter().any(|e| e.starts_with("start")));
    d.demolish_lots(true, &[Some(0x50)], 0, 0x1303, false, None, None);
    assert!(w.take().contains(&"explode 0x50 false".to_string()));
    // No configuration: also exploded; a water-compatible one is cleared.
    w.lots.borrow_mut().get_mut(&0x50).unwrap().water = None;
    d.demolish_lots(true, &[Some(0x50)], 0, 0x300, false, None, None);
    assert!(w.take().contains(&"explode 0x50 true".to_string()));
    w.lots.borrow_mut().get_mut(&0x50).unwrap().water = Some(true);
    d.demolish_lots(true, &[Some(0x50)], 0, 0x300, false, None, None);
    let log = w.take();
    assert!(log.contains(&"start 0x50".to_string()) && !log.iter().any(|e| e.starts_with("explode")));
}

#[test]
fn dm12_10_power_pole() {
    let (w, d) = setup();
    let mut a = Occ::at(occ_type::POWER_POLE, 3, 3);
    a.cost = 10;
    a.connections = vec![0x21, 0x22];
    let mut b = Occ::at(occ_type::POWER_POLE, 9, 9);
    b.connections = vec![0x20];
    let mut c = Occ::at(occ_type::POWER_POLE, 9, 10);
    c.connections = vec![0x20, 0x23];
    let mut e = Occ::at(occ_type::POWER_POLE, 9, 11);
    e.connections = vec![0x22];
    w.occ(0x20, a);
    w.occ(0x21, b);
    w.occ(0x22, c);
    w.occ(0x23, e);
    let r = d.demolish_region(true, &region(3, 3, 3, 3), 1, 0, false, None, Some(0x5E7), None, 0, 0);
    assert_eq!(
        r,
        Outcome {
            ok: true,
            cost: Some(10)
        }
    );
    assert_eq!(
        w.take(),
        [
            "pre_demolish 0x20",
            "remove 0x20 false",
            "break 0x20",
            "shutdown 0x20",
            "insert 0x5e7 0x20",
            "queue true",
            "insert 0x5e7 0x21",
            "pre_demolish 0x21",
            "remove 0x21 true",
            "queue false"
        ]
    );
    // A dry run sends nothing.
    w.occ(0x24, Occ::at(occ_type::POWER_POLE, 30, 30));
    let r = d.demolish_occupant(false, 0x24, 1, 1, false, &mut Orphans::Host(0x1157));
    assert_eq!(
        r,
        Outcome {
            ok: true,
            cost: Some(0)
        }
    );
    assert!(w.take().is_empty());
    // Directly, with a caller list.
    let mut p = Occ::at(occ_type::POWER_POLE, 31, 31);
    p.connections = vec![0x26];
    let mut q = Occ::at(occ_type::POWER_POLE, 32, 31);
    q.connections = vec![0x25];
    w.occ(0x25, p);
    w.occ(0x26, q);
    let r = d.demolish_occupant(true, 0x25, 1, 1, false, &mut Orphans::Host(0x1157));
    assert!(r.ok);
    assert_eq!(
        w.take(),
        [
            "pre_demolish 0x25",
            "remove 0x25 false",
            "break 0x25",
            "shutdown 0x25",
            "append 0x1157 0x26"
        ]
    );
    // A pole without the pole interface is "anything else".
    let mut np = Occ::at(occ_type::POWER_POLE, 33, 33);
    np.pole = false;
    w.occ(0x27, np);
    assert_eq!(
        d.demolish_occupant(true, 0x27, 1, 1, false, &mut Orphans::None),
        Outcome {
            ok: false,
            cost: Some(0)
        }
    );
}

#[test]
fn dm12_11_networks_one_batch() {
    let (w, d) = setup();
    for (i, x) in [(0x30u64, 1), (0x31, 2), (0x32, 3)] {
        let mut r = Occ::at(occ_type::NETWORK, x, 7);
        r.cost = 5;
        w.occ(i, r);
    }
    let r = d.demolish_region(true, &region(1, 7, 3, 7), 1, 0, false, None, None, None, 0, 0);
    assert_eq!(
        r,
        Outcome {
            ok: true,
            cost: Some(15)
        }
    );
    assert_eq!(
        w.take(),
        [
            "queue true",
            "delete_connection 0x30",
            "delete_connection 0x31",
            "delete_connection 0x32",
            "destroy [30, 31, 32]",
            "queue false"
        ]
    );
    // Directly, without skipNetworkUpdate: one network at once.
    w.occ(0x33, Occ::at(occ_type::NETWORK, 9, 9));
    d.demolish_occupant(true, 0x33, 1, 0x2001, false, &mut Orphans::None);
    assert_eq!(w.take(), ["delete_connection 0x33", "destroy [33]"]);
}

#[test]
fn dm12_14_float_tolerance() {
    // DM6: |alt − h| in f32 against 0.05f. Exactly 0.05f: area mode does not
    // count it (a widened f64 compare with 0.05 would).
    let (w, d) = setup();
    let next_up = f32::from_bits(0.05f32.to_bits() + 1);
    let next_down = f32::from_bits(0.05f32.to_bits() - 1);
    w.alt.borrow_mut().insert((1, 1), 0.05);
    w.alt.borrow_mut().insert((3, 3), next_up);
    w.alt.borrow_mut().insert((5, 5), next_down);
    assert_eq!(d.modify_terrain_height(false, false, 1, 1, 1, 1, 0.0, None, 5), 0);
    assert_eq!(d.modify_terrain_height(false, false, 3, 3, 3, 3, 0.0, None, 5), 1);
    assert_eq!(d.modify_terrain_height(false, true, 5, 5, 5, 5, 0.0, None, 5), 0);
    assert_eq!(d.modify_terrain_height(false, true, 3, 3, 3, 3, 0.0, None, 5), 1);
    // [question DM15] Per-vertex mode skips `< 0.05f` (§4.7), so exactly
    // 0.05f is set; §12 test 14 says "counted by neither".
    assert_eq!(d.modify_terrain_height(false, true, 1, 1, 1, 1, 0.0, None, 5), 1);
    // The subtraction rounds to f32: 10.05f − 10 is not 0.05f.
    w.alt.borrow_mut().insert((7, 7), 10.05);
    let diff = (10.05f32 - 10.0f32).abs();
    let expect = i32::from(diff > 0.05f32);
    assert_eq!(d.modify_terrain_height(false, false, 7, 7, 7, 7, 10.0, None, 5), expect);
}

#[test]
fn dm12_12_terrain_per_vertex() {
    let (w, d) = setup();
    // A building whose box covers vertex (5, 5) ± 14.5 m.
    let mut b = Occ::at(occ_type::BUILDING, 4, 4);
    b.bounds = (70.0, 70.0, 75.0, 75.0);
    w.occ(0x10, b);
    // A locked lot on cell (8, 5): vertices (8..9, 5..6).
    let mut l = Lot::new(8, 5, 8, 5, 1);
    l.locked = true;
    w.lot(0x50, l);
    // A water vertex and one already at the height.
    w.water.borrow_mut().insert((6, 5));
    w.alt.borrow_mut().insert((7, 5), 10.04);
    let n = d.modify_terrain_height(true, true, 5, 5, 9, 5, 10.0, None, 5);
    // (5,5) under the building, (6,5) water, (7,5) within 0.05,
    // (8,5) and (9,5) on the locked lot: none.
    assert_eq!(n, 0);
    assert!(w.take().is_empty());
    let n = d.modify_terrain_height(true, true, 5, 5, 9, 6, 10.0, None, 5);
    // Row z = 6: (5, 6)'s square starts at z = 81.5, past the box: free;
    // (6, 6) and (7, 6) are free; (8, 6) and (9, 6) touch the locked lot.
    assert_eq!(n, 3);
    assert_eq!(
        w.take(),
        [
            "set_vertex 5 6 10",
            "set_vertex 6 6 10",
            "set_vertex 7 6 10",
            "redisplay 4 4 10 7 5"
        ]
    );
    // The exempt lot does not lock; a dry run counts and redisplays.
    let n = d.modify_terrain_height(false, true, 8, 5, 8, 5, 10.0, Some(0x50), 5);
    assert_eq!(n, 1);
    assert_eq!(w.take(), ["redisplay 7 4 9 6 5"]);
    // A movable building does not block.
    w.occs
        .borrow_mut()
        .get_mut(&0x10)
        .unwrap()
        .props
        .insert(PROP_MOVABLE, true);
    assert_eq!(d.modify_terrain_height(false, true, 5, 5, 5, 5, 10.0, None, 5), 1);
}

#[test]
fn terrain_area_mode() {
    let (w, d) = setup();
    w.alt.borrow_mut().insert((2, 2), 3.0);
    // n = 8 vertices of (1..3, 1..3) differ from 3.0 (all but (2, 2)).
    assert_eq!(d.modify_terrain_height(true, false, 1, 1, 3, 3, 3.0, None, 5), 8);
    assert_eq!(w.take(), ["set_rect 1 1 3 3 3 5", "redisplay 1 1 3 3 5"]);
    // No difference: 0, nothing.
    assert_eq!(d.modify_terrain_height(true, false, 2, 2, 2, 2, 3.0, None, 5), 0);
    // A locked lot in Q: n, nothing set.
    let mut l = Lot::new(0, 0, 0, 0, 1);
    l.locked = true;
    w.lot(0x50, l);
    assert_eq!(d.modify_terrain_height(true, false, 1, 1, 1, 1, 3.0, None, 5), 1);
    assert!(w.take().is_empty());
    assert_eq!(d.modify_terrain_height(true, false, 1, 1, 1, 1, 3.0, Some(0x50), 5), 1);
    assert_eq!(w.take().len(), 2);
    // An immovable occupant in Q: 0.
    w.occ(0x10, Occ::at(occ_type::PROP, 0, 1));
    assert_eq!(d.modify_terrain_height(true, false, 1, 1, 1, 1, 3.0, Some(0x50), 5), 0);
    assert!(w.take().is_empty());
    // The rectangle is clamped to the city.
    w.occs.borrow_mut().clear();
    assert_eq!(
        d.modify_terrain_height(true, false, 62, 62, 70, 64, 3.0, Some(0x50), 5),
        27
    );
    assert_eq!(w.take(), ["set_rect 62 62 64 64 3 5", "redisplay 62 62 64 64 5"]);
}

#[test]
fn dm12_13_effects() {
    let (w, d) = setup();
    let mut t = Occ::at(occ_type::FLORA, 4, 4);
    t.cost = 5;
    w.occ(0x40, t);
    // Cause byte 0: no effect, real removal.
    d.demolish_region(true, &region(4, 4, 4, 4), 1, 0, false, None, None, None, 0, 0);
    let log = w.take();
    assert!(!log.iter().any(|e| e.starts_with("effect") || e.starts_with("group")));
    assert!(log.contains(&"remove 0x40 true".to_string()));
    // A dry run with a cause: none.
    w.occs.borrow_mut().get_mut(&0x40).unwrap().flags = OCCUPANT_FLAG_IN_MANAGER;
    d.demolish_region(false, &region(4, 4, 4, 4), 1, 1, false, None, None, None, 0, 0);
    assert!(w.take().is_empty());
    // Committed with cause 1: the occupant effect, the removal, the group effect.
    let r = d.demolish_region(true, &region(4, 4, 4, 4), 1, 1, false, None, None, Some(0x40), 7, 8);
    assert_eq!(r.cost, Some(5));
    assert_eq!(
        w.take(),
        [
            "sound 0x40",
            "effect 0x40 [120] Some((7, 8)) false",
            "remove 0x40 true",
            "group 0x100 4 4",
            "queue true",
            "queue false"
        ]
    );
}

#[test]
fn effect_keys_and_sizes() {
    assert_eq!(effect_key_candidates(0x0312), vec![0x0312, 0x0311, 0x0310]);
    assert_eq!(effect_key_candidates(0x0130), vec![0x0130]);
    let t = DemolitionTuning::SHIPPED;
    assert_eq!(model_size_class(20.0, &t), 0);
    assert_eq!(model_size_class(20.5, &t), 1);
    assert_eq!(model_size_class(64.5, &t), 2);
    assert_eq!(group_size_class(0, &t), 0);
    assert_eq!(group_size_class(21, &t), 1);
    assert_eq!(group_size_class(101, &t), 2);
    // DM8: unsigned compares, so a negative n is the large variant.
    assert_eq!(group_size_class(-1, &t), 2);
    let (w, d) = setup();
    // A large building on a run-down lot: 0x0312 → grey, keys down to 0x0310.
    let mut b = Occ::at(occ_type::BUILDING, 1, 1);
    b.extent = Some(70.0);
    w.occ(0x10, b);
    let mut l = Lot::new(1, 1, 1, 1, 1);
    l.condition = 2;
    w.lot(0x50, l);
    d.do_demolish_occupant_effect(0x10, 0x303, Some(0x50), false, 0, 0);
    assert_eq!(w.take(), ["sound 0x10", "effect 0x10 [312, 311, 310] None true"]);
    // Props: no effect without 0x8A5E5DB8; 0x20 in group 0x5004, else 0x30.
    let mut p = Occ::at(occ_type::PROP, 1, 1);
    p.extent = Some(30.0);
    w.occ(0x11, p);
    d.do_demolish_occupant_effect(0x11, 1, None, false, 0, 0);
    assert!(w.take().is_empty());
    w.occs
        .borrow_mut()
        .get_mut(&0x11)
        .unwrap()
        .props
        .insert(PROP_EFFECT_PROP, true);
    d.do_demolish_occupant_effect(0x11, 1, None, false, 0, 0);
    assert_eq!(w.take(), ["sound 0x11", "effect 0x11 [131, 130] None false"]);
    w.occs
        .borrow_mut()
        .get_mut(&0x11)
        .unwrap()
        .groups
        .push(GROUP_PROP_FLORA_LIKE);
    d.do_demolish_occupant_effect(0x11, 1, None, false, 0, 0);
    assert_eq!(w.take(), ["sound 0x11", "effect 0x11 [121, 120] None false"]);
    // Networks: 0x50 for type 1 without type 2, else 0x40.
    let mut n = Occ::at(occ_type::NETWORK, 1, 1);
    n.net_types = vec![1];
    w.occ(0x12, n);
    d.do_demolish_occupant_effect(0x12, 2, None, true, 3, 4);
    assert_eq!(w.take(), ["sound 0x12", "effect 0x12 [250] Some((3, 4)) false"]);
    w.occs.borrow_mut().get_mut(&0x12).unwrap().net_types.push(2);
    d.do_demolish_occupant_effect(0x12, 2, None, false, 0, 0);
    assert_eq!(w.take(), ["sound 0x12", "effect 0x12 [240] None false"]);
    // Below the screen cutoff: nothing.
    w.occs.borrow_mut().get_mut(&0x12).unwrap().extent = None;
    d.do_demolish_occupant_effect(0x12, 2, None, false, 0, 0);
    assert!(w.take().is_empty());
    // The group effect: size from n, centre cell.
    d.do_demolish_group_effect(0x2, 150, 0, CellRect::new(10, 20, 13, 25));
    assert_eq!(w.take(), ["group 0x202 11 22"]);
}

// ---- §4.10 worked examples and the filter -----------------------------------------------------

#[test]
fn worked_example_1_bulldozer_preview() {
    let (w, d) = setup();
    // A 2×1 empty RCI lot with a base texture and a prop; a tree on (12, 20).
    let mut l = Lot::new(10, 20, 11, 20, 1);
    l.occs = vec![0x60, 0x61];
    w.lot(0x50, l);
    w.occ(0x60, Occ::at(0x1234_5678, 10, 20)); // base texture: not in the type list
    w.occ(0x61, Occ::at(occ_type::PROP, 11, 20));
    let mut tree = Occ::at(occ_type::FLORA, 12, 20);
    tree.cost = 5;
    w.occ(0x62, tree);
    let r = d.demolish_region(false, &region(10, 20, 12, 20), 1, 1, false, None, None, None, 0, 0);
    assert_eq!(
        r,
        Outcome {
            ok: true,
            cost: Some(5)
        }
    );
    assert!(w.take().is_empty());
    assert_eq!(d.snapshot().region, None);
    assert_eq!(d.snapshot().filter, Filter::default());
}

#[test]
fn flag_40_leaves_cost_untouched() {
    let (w, d) = setup();
    let mut t = Occ::at(occ_type::FLORA, 1, 1);
    t.flags |= OCCUPANT_FLAG_SKIPPABLE;
    t.cost = 3;
    w.occ(0x40, t);
    assert_eq!(
        d.demolish_occupant(true, 0x40, 1, 0x101, false, &mut Orphans::None),
        Outcome { ok: false, cost: None }
    );
    assert_eq!(
        d.demolish_occupant(false, 0x40, 1, 0x1, false, &mut Orphans::None),
        Outcome {
            ok: true,
            cost: Some(3)
        }
    );
    // Missing simulators: false with cost 0.
    w.flora.set(false);
    assert_eq!(
        d.demolish_occupant(true, 0x40, 1, 1, false, &mut Orphans::None),
        Outcome {
            ok: false,
            cost: Some(0)
        }
    );
    w.occ(0x41, Occ::at(occ_type::PROP, 1, 1));
    w.prop_mgr.set(false);
    assert_eq!(
        d.demolish_occupant(false, 0x41, 1, 1, false, &mut Orphans::None),
        Outcome {
            ok: false,
            cost: Some(0)
        }
    );
    assert!(w.take().is_empty());
    // Foundations and constructions.
    let mut f = Occ::at(occ_type::FOUNDATION, 1, 1);
    f.flags = 0;
    w.occ(0x42, f);
    assert!(d.demolish_occupant(true, 0x42, 1, 1, false, &mut Orphans::None).ok);
    assert!(w.take().is_empty());
    w.occ(0x43, Occ::at(occ_type::CONSTRUCTION, 1, 1));
    assert!(d.demolish_occupant(true, 0x43, 1, 1, false, &mut Orphans::None).ok);
    assert!(!d.demolish_occupant(true, 0x43, 1, 1, false, &mut Orphans::None).ok);
    assert_eq!(w.take(), ["remove 0x43 true"]);
}

#[test]
fn filter_steps() {
    let (w, d) = setup();
    let mut b = Occ::at(occ_type::BUILDING, 1, 1);
    w.occ(0x10, b.clone());
    d.with_state(|s| s.filter.level = Some(2));
    assert!(d.filter_includes(0x10));
    // 1. Realism: rejected only when the default filter accepts.
    b.realistic_ok = false;
    w.occ(0x10, b.clone());
    assert!(!d.filter_includes(0x10));
    b.default_ok = false;
    w.occ(0x10, b.clone());
    assert!(d.filter_includes(0x10));
    // 3. Bounds (inclusive).
    d.with_state(|s| s.filter.bounds = Some((24.0, 24.0, 40.0, 40.0)));
    assert!(d.filter_includes(0x10));
    d.with_state(|s| s.filter.bounds = Some((24.5, 0.0, 40.0, 40.0)));
    assert!(!d.filter_includes(0x10));
    d.with_state(|s| s.filter.bounds = None);
    // 4. Privilege.
    b.player_lock = true;
    w.occ(0x10, b.clone());
    assert!(!d.filter_includes(0x10));
    d.with_state(|s| s.filter.level = Some(1));
    assert!(d.filter_includes(0x10));
    // 5. Foundations: slot 9 must answer 1; other types are refused.
    let mut f = Occ::at(occ_type::FOUNDATION, 1, 1);
    w.occ(0x11, f.clone());
    assert!(!d.filter_includes(0x11));
    f.foundation = Some(1);
    w.occ(0x11, f);
    assert!(d.filter_includes(0x11));
    w.occ(0x12, Occ::at(0x0BAD_0000, 1, 1));
    assert!(!d.filter_includes(0x12));
    // 6. The view filter (not applied to an accepted foundation).
    w.rejects.borrow_mut().insert(0xF1, [0x10, 0x11].into_iter().collect());
    d.with_state(|s| s.filter.view = Some(0xF1));
    assert!(!d.filter_includes(0x10));
    assert!(d.filter_includes(0x11));
    d.with_state(|s| s.filter.view = None);
    // 7. Networks underground.
    let mut n = Occ::at(occ_type::NETWORK, 1, 1);
    n.net_flags = Some(0x1);
    w.occ(0x13, n);
    assert!(d.filter_includes(0x13));
    w.view_level.set(Some(2));
    assert!(!d.filter_includes(0x13));
    w.occs.borrow_mut().get_mut(&0x13).unwrap().net_flags = Some(0x20);
    assert!(d.filter_includes(0x13));
    w.view_level.set(None);
    w.occs.borrow_mut().get_mut(&0x13).unwrap().net_flags = Some(0x1);
    assert!(d.filter_includes(0x13));
}

#[test]
fn region_view_filter_and_loose_occupants() {
    let (w, d) = setup();
    // An empty lot with a prop; a tree on the lot's cell outside the lot list.
    let mut l = Lot::new(1, 1, 1, 1, 1);
    l.occs = vec![0x61];
    w.lot(0x50, l);
    w.occ(0x61, Occ::at(occ_type::PROP, 1, 1));
    let mut tree = Occ::at(occ_type::FLORA, 1, 1);
    tree.cost = 7;
    w.occ(0x62, tree);
    // The tree is accepted by the default filter: on a lot cell it is not
    // added by the per-cell pass.
    let r = d.demolish_region(false, &region(1, 1, 1, 1), 1, 1, false, None, None, None, 0, 0);
    assert_eq!(r.cost, Some(0));
    // Not accepted by the default filter (and realistic): it is added, unless 0x2000.
    w.occs.borrow_mut().get_mut(&0x62).unwrap().default_ok = false;
    let r = d.demolish_region(false, &region(1, 1, 1, 1), 1, 1, false, None, None, None, 0, 0);
    assert_eq!(r.cost, Some(7));
    let r = d.demolish_region(false, &region(1, 1, 1, 1), 1, 0x2001, false, None, None, None, 0, 0);
    assert_eq!(r.cost, Some(0));
    // A view filter that rejects buildings: an empty lot does not qualify.
    w.view_filter.set(Some(0xF1));
    w.rejects_building_type.borrow_mut().insert(0xF1);
    let r = d.demolish_region(
        true,
        &region(1, 1, 1, 1),
        1,
        0x2000,
        false,
        None,
        Some(0x5E7),
        None,
        0,
        0,
    );
    assert!(r.ok);
    let log = w.take();
    assert!(!log.iter().any(|e| e.starts_with("start")));
    assert_eq!(log.last().map(String::as_str), Some("release 0xf1"));
    // A caller filter rejecting the prop: nothing taken from the lot (prop on
    // the lot rejected → blocked).
    w.view_filter.set(None);
    w.rejects.borrow_mut().insert(0xC1, [0x61].into_iter().collect());
    let r = d.demolish_region(
        true,
        &region(1, 1, 1, 1),
        1,
        0x2000,
        false,
        Some(0xC1),
        None,
        None,
        0,
        0,
    );
    assert!(!r.ok);
}
