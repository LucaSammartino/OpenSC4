// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests of the dispatch manager against a fake city: the §12 tests of
//! `docs/specs/dispatch.md` (named `dp12_01_…` to `dp12_10_…`), the §4.7
//! worked example, the tool, the messages and the record.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use super::*;
use crate::stream::{ByteReader, ByteWriter, RefResolver, SaveObject, SaveRegistry};

#[test]
fn dropping_manager_releases_registered_sources() {
    let world = World::new();
    let manager = DispatchManager::new(world.clone(), 1);
    manager.register_dispatch_source(0x123);
    assert_eq!(world.refs(0x123), 1);
    drop(manager);
    assert_eq!(world.refs(0x123), 0);
}

#[derive(Clone, Debug, Default)]
struct Source {
    types: Vec<u32>,
    can: bool,
    all_busy: bool,
    stations: Vec<StationUnits>,
    /// `DispatchFromNearestStation` answers with the first free unit of the
    /// first station when true.
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

struct World {
    log: RefCell<Vec<String>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    sources: RefCell<BTreeMap<u64, Source>>,
    pylons: RefCell<BTreeMap<u64, Pylon>>,
    next: Cell<u64>,
    vehicles_fail: Cell<bool>,
    next_handle: Cell<u32>,
    /// Each new vehicle's occupant group (fire engines 0x4202, fire planes
    /// 0x4303, others 0x4101).
    groups: RefCell<BTreeMap<u64, u32>>,
    /// The return's unit-state call fails (§4.4).
    park_fails: Cell<bool>,
}

impl World {
    fn new() -> Rc<World> {
        Rc::new(World {
            log: RefCell::new(Vec::new()),
            refs: RefCell::new(BTreeMap::new()),
            sources: RefCell::new(BTreeMap::new()),
            pylons: RefCell::new(BTreeMap::new()),
            next: Cell::new(0x1000),
            vehicles_fail: Cell::new(false),
            next_handle: Cell::new(1),
            groups: RefCell::new(BTreeMap::new()),
            park_fails: Cell::new(false),
        })
    }
    fn ev(&self, s: impl Into<String>) {
        self.log.borrow_mut().push(s.into());
    }
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
    fn posts(&self) -> Vec<String> {
        self.take().into_iter().filter(|e| e.starts_with("post")).collect()
    }
    fn refs(&self, t: u64) -> i64 {
        self.refs.borrow().get(&t).copied().unwrap_or(0)
    }
    fn fresh(&self) -> u64 {
        let t = self.next.get();
        self.next.set(t + 1);
        *self.refs.borrow_mut().entry(t).or_insert(0) += 1;
        t
    }
    /// A source for `types` with one station `station` of `count` units
    /// (call numbers from `base`), all free.
    fn source(&self, src: u64, types: &[u32], station: u64, base: u32, count: u32) {
        self.sources.borrow_mut().insert(
            src,
            Source {
                types: types.to_vec(),
                can: true,
                all_busy: false,
                stations: vec![StationUnits {
                    station,
                    mask: (1 << count) - 1,
                    base,
                    count,
                }],
                nearest: true,
            },
        );
    }
    fn mask(&self, src: u64) -> u32 {
        self.sources.borrow()[&src].stations[0].mask
    }
    fn unit(&self, src: u64, call: u32, free: bool) {
        let mut s = self.sources.borrow_mut();
        let st = &mut s.get_mut(&src).unwrap().stations[0];
        let bit = 1u32 << (call - st.base);
        if free {
            st.mask |= bit;
        } else {
            st.mask &= !bit;
        }
    }
}

impl DispatchHost for World {
    fn acquire_message_server(&self) -> bool {
        self.ev("acquire ms");
        true
    }
    fn subscribe(&self, m: u32) {
        self.ev(format!("subscribe {m:08X}"));
    }
    fn unsubscribe(&self, m: u32) {
        self.ev(format!("unsubscribe {m:08X}"));
    }
    fn release_message_server(&self) {
        self.ev("release ms");
    }
    fn post(&self, m: u32, t: u32, id: u32) {
        self.ev(format!("post {m:08X} {t} {id}"));
    }
    fn acquire_city(&self) -> bool {
        self.ev("acquire city");
        true
    }
    fn release_city(&self) {
        self.ev("release city");
    }
    fn create_vehicle_view(&self) -> Option<u64> {
        self.ev("create view");
        Some(0x7E)
    }
    fn destroy_vehicle_view(&self, v: u64) {
        self.ev(format!("destroy view {v:#x}"));
    }
    fn refresh_vehicle_view(&self, v: Option<u64>) {
        self.ev(format!("refresh view {v:?}"));
    }
    fn track_vehicle(&self, a: u64, kind: u32) -> u32 {
        let h = self.next_handle.get();
        self.next_handle.set(h + 1);
        self.ev(format!("track {a:#x} {kind} -> {h}"));
        h
    }
    fn untrack_vehicle(&self, h: u32) {
        self.ev(format!("untrack {h}"));
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
        let s = &self.sources.borrow()[&src];
        (s.can, s.all_busy)
    }
    fn source_dispatch_from_nearest_station(&self, src: u64, _x: f32, _z: f32) -> Option<(u64, u32)> {
        let (station, call) = {
            let s = &self.sources.borrow()[&src];
            if !s.nearest {
                return None;
            }
            let st = s.stations[0];
            let i = (0..st.count).find(|i| st.mask & (1 << i) != 0)?;
            (st.station, st.base + i)
        };
        self.unit(src, call, false);
        self.ev(format!("nearest {station:#x} {call}"));
        Some((station, call))
    }
    fn source_dispatch_from_station(&self, src: u64, station: Option<u64>, call: &mut u32) -> bool {
        self.unit(src, *call, false);
        self.ev(format!("from_station {station:?} {call}"));
        true
    }
    fn source_revoke_dispatch_from_station(&self, src: u64, station: Option<u64>, call: u32) {
        self.unit(src, call, true);
        self.ev(format!("revoke_from_station {station:?} {call}"));
    }
    fn source_stations(&self, src: u64) -> Vec<StationUnits> {
        self.sources.borrow()[&src].stations.clone()
    }
    fn begin_vehicle(&self, _src: u64, info: &DispatchInfo) -> Option<u64> {
        if self.vehicles_fail.get() {
            return None;
        }
        let a = info.automaton.unwrap_or_else(|| {
            let a = self.fresh();
            let g = match info.dispatch_type {
                dispatch_type::FIRE_ENGINE => 0x4202,
                dispatch_type::FIRE_PLANE => 0x4303,
                _ => 0x4101,
            };
            self.groups.borrow_mut().insert(a, g);
            a
        });
        self.ev(format!("vehicle {a:#x} id {} call {}", info.id, info.call_id));
        Some(a)
    }
    fn automaton_despawn(&self, a: u64) {
        self.ev(format!("despawn {a:#x}"));
    }
    fn automaton_go_home(&self, a: u64) {
        self.ev(format!("go_home {a:#x}"));
    }
    fn create_pylon(&self) -> Option<u64> {
        let p = self.fresh();
        self.pylons.borrow_mut().insert(p, Pylon::default());
        self.ev(format!("create pylon {p:#x}"));
        Some(p)
    }
    fn pylon_source(&self, p: u64) -> Option<u64> {
        self.pylons.borrow()[&p].source
    }
    fn pylon_set_source(&self, p: u64, st: u64) {
        self.pylons.borrow_mut().get_mut(&p).unwrap().source = Some(st);
    }
    fn pylon_call_number(&self, p: u64) -> u32 {
        self.pylons.borrow()[&p].call
    }
    fn pylon_set_call_number(&self, p: u64, n: u32) {
        self.pylons.borrow_mut().get_mut(&p).unwrap().call = n;
    }
    fn pylon_dispatch_type(&self, p: u64) -> u32 {
        self.pylons.borrow()[&p].dtype
    }
    fn pylon_set_dispatch_type(&self, p: u64, t: u32) {
        self.pylons.borrow_mut().get_mut(&p).unwrap().dtype = t;
    }
    fn pylon_set_dispatch_state(&self, p: u64, s: u32) {
        self.pylons.borrow_mut().get_mut(&p).unwrap().dstate = s;
    }
    fn pylon_set_ui_state(&self, p: u64, s: u32) {
        self.pylons.borrow_mut().get_mut(&p).unwrap().ui = s;
    }
    fn pylon_move_to(&self, p: u64, x: f32, y: f32, z: f32) {
        self.ev(format!("move {p:#x} {x} {y} {z}"));
    }
    fn pylon_place_at_unit(&self, p: u64, st: u64, i: u32, n: u32) {
        self.ev(format!("place {p:#x} {st:#x} {i}/{n}"));
    }
    fn pylon_return_to_station(&self, _src: u64, p: u64, st: u64) -> bool {
        self.ev(format!("park {p:#x} {st:#x}"));
        !self.park_fails.get()
    }
    fn vehicle_in_group(&self, a: u64, g: u32) -> bool {
        self.groups.borrow().get(&a) == Some(&g)
    }
    fn pylon_shutdown(&self, p: u64) {
        self.ev(format!("shutdown {p:#x}"));
    }
    fn object_ref(&self, t: u64) -> RefValue {
        RefValue::Host {
            token: t,
            class_id: 0xC1A5_5000,
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

const SRC: u64 = 0x500;
const STATION: u64 = 0x600;

fn setup() -> (Rc<World>, Rc<DispatchManager>) {
    let w = World::new();
    let d = DispatchManager::new(w.clone(), 0xD15);
    assert!(d.init());
    w.source(SRC, &[dispatch_type::FIRE_ENGINE], STATION, 1, 3);
    d.register_dispatch_source(SRC);
    w.take();
    (w, d)
}

fn write(d: &DispatchManager) -> Vec<u8> {
    let mut out = ByteWriter::with_registry(Rc::new(RefCell::new(SaveRegistry::new())));
    assert!(d.write(&mut out));
    out.into_bytes()
}

/// Resolves every reference to its object ID as the token.
struct Ids;
impl RefResolver for Ids {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        Some(RefValue::Host {
            token: u64::from(object_id),
            class_id,
        })
    }
}

fn hex(s: &str) -> Vec<u8> {
    let s: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

// ---- identity and lifecycle -------------------------------------------------------------------

#[test]
fn identity_and_lifecycle() {
    assert_eq!(query_interface(1), Some(0));
    assert_eq!(query_interface(IID_DISPATCH_MANAGER), Some(0));
    assert_eq!(query_interface(IID_MESSAGE_TARGET2), Some(4));
    assert_eq!(query_interface(IID_SERIALIZABLE), Some(8));
    assert_eq!(query_interface(0xE98B_2F57), None);
    let w = World::new();
    let d = DispatchManager::new(w.clone(), 0xD15);
    assert_eq!(d.class_id(), CLSID_DISPATCH_MANAGER);
    assert!(d.init());
    assert_eq!(
        w.take(),
        [
            "acquire ms",
            "subscribe 99EF1143",
            "subscribe AA1B2002",
            "subscribe A6B79602",
            "create view",
            "acquire city"
        ]
    );
    assert_eq!(d.get_dispatch_vehicle_view(), Some(0x7E));
    assert_eq!(d.snapshot().tool_type, 8);
    // Sources: held, no duplicate check; unregister releases the first.
    d.register_dispatch_source(SRC);
    d.register_dispatch_source(SRC);
    assert_eq!(w.refs(SRC), 2);
    d.unregister_dispatch_source(SRC);
    assert_eq!(w.refs(SRC), 1);
    d.unregister_dispatch_source(0x999);
    assert_eq!(w.refs(SRC), 1);
    assert_eq!(d.add_ref(), 1);
    assert_eq!(d.release(), 0);
    assert_eq!(
        w.take(),
        [
            "unsubscribe 99EF1143",
            "unsubscribe AA1B2002",
            "unsubscribe A6B79602",
            "release ms",
            "destroy view 0x7e",
            "release city"
        ]
    );
    assert_eq!(w.refs(SRC), 0);
    assert!(d.snapshot().destroyed);
}

// ---- §12 --------------------------------------------------------------------------------------

#[test]
fn dp12_01_empty_record() {
    let w = World::new();
    let d = DispatchManager::new(w, 1);
    let b = write(&d);
    assert_eq!(b, hex("0200 00000000 00000000 00000000 00000000 08000000"));
    assert_eq!(b.len(), 22);
}

#[test]
fn dp12_02_corpus_round_trip() {
    for (next, tool) in [(0u32, 4u32), (0, 8), (1, 8), (0, 5), (0, 7)] {
        let rec = [
            hex("0200 00000000"),
            next.to_le_bytes().to_vec(),
            hex("00000000 00000000"),
            tool.to_le_bytes().to_vec(),
        ]
        .concat();
        let w = World::new();
        let d = DispatchManager::new(w.clone(), 1);
        assert!(d.read(&mut ByteReader::new(&rec)));
        assert_eq!(write(&d), rec);
        let s = d.snapshot();
        assert!(s.loaded);
        assert_eq!((s.next_id, s.tool_type), (next, tool));
        assert!(d.init());
        assert_eq!(d.snapshot().tool_type, dispatch_type::NONE);
    }
}

#[test]
fn dp12_03_wrong_versions() {
    for v in [1u16, 3] {
        let w = World::new();
        let d = DispatchManager::new(w, 1);
        let mut rec = hex("0200 00000000 00000000 00000000 00000000 08000000");
        rec[..2].copy_from_slice(&v.to_le_bytes());
        assert!(!d.read(&mut ByteReader::new(&rec)));
        assert!(d.snapshot().loaded);
    }
    // A cut record: nothing appended, references given back.
    let w = World::new();
    let d = DispatchManager::new(w.clone(), 1);
    let rec = hex("0200 00000000 05000000 00000000 01000000 07000000 11223344");
    let mut ids = Ids;
    assert!(!d.read(&mut ByteReader::with_resolver(&rec, &mut ids)));
    assert_eq!(w.refs(7), 0);
    assert!(d.snapshot().available_pylons.is_empty());
}

#[test]
fn dp12_04_dispatch() {
    let w = World::new();
    let d = DispatchManager::new(w.clone(), 1);
    d.init();
    w.take();
    assert_eq!(d.dispatch(1, 10.0, 20.0), 0);
    assert_eq!(d.snapshot().next_id, 0);
    w.source(SRC, &[1], STATION, 1, 3);
    d.register_dispatch_source(SRC);
    assert_eq!(d.dispatch(1, 10.0, 20.0), 1);
    let s = d.snapshot();
    assert_eq!(s.next_id, 1);
    assert_eq!(s.records.len(), 1);
    let r = s.records[0];
    assert_eq!((r.dispatch_type, r.id, r.call_id), (1, 1, 1));
    assert_eq!(r.target, (10.0, 0.3, 20.0));
    assert_eq!(r.station, Some(STATION));
    assert!(r.automaton.is_some() && r.view_handle != 0 && r.pylon.is_none());
    assert_eq!(w.refs(STATION), 1);
    let log = w.take();
    assert_eq!(log.iter().filter(|e| e.starts_with("post")).count(), 1);
    assert!(log.contains(&"post E94671EB 1 1".to_string()));
    // [quirk] Outside the open tool a pylon is created and leaked.
    assert!(log.iter().any(|e| e.starts_with("create pylon")));
    // CanDispatch false: 0, no ID consumed.
    w.sources.borrow_mut().get_mut(&SRC).unwrap().can = false;
    assert!(!d.can_dispatch(1, 0.0, 0.0));
    assert_eq!(d.dispatch(1, 0.0, 0.0), 0);
    assert_eq!(d.snapshot().next_id, 1);
    // All busy without an active pylon: 0.
    let src = SRC;
    w.sources.borrow_mut().get_mut(&src).unwrap().can = true;
    w.sources.borrow_mut().get_mut(&src).unwrap().all_busy = true;
    assert_eq!(d.dispatch(1, 0.0, 0.0), 0);
    assert_eq!(d.snapshot().next_id, 1);
    // A failed vehicle: revoked, 0, the ID consumed.
    w.sources.borrow_mut().get_mut(&src).unwrap().all_busy = false;
    w.vehicles_fail.set(true);
    w.take();
    assert_eq!(d.dispatch(1, 0.0, 0.0), 0);
    assert_eq!(d.snapshot().next_id, 2);
    assert_eq!(d.snapshot().records.len(), 1);
    assert_eq!(w.posts(), ["post 894671F6 1 2"]);
}

#[test]
fn dp12_05_failed_init_dispatch() {
    let (w, d) = setup();
    w.sources.borrow_mut().get_mut(&SRC).unwrap().nearest = false;
    assert_eq!(d.dispatch(1, 1.0, 1.0), 0);
    assert_eq!(d.snapshot().next_id, 1);
    assert!(d.snapshot().records.is_empty());
    assert!(w.posts().is_empty());
}

#[test]
fn dp12_06_state_events() {
    let (w, d) = setup();
    let id = d.dispatch(1, 1.0, 1.0);
    let a = d.get_dispatch_info(id).unwrap().automaton.unwrap();
    w.take();
    let ev = |code, state| DispatchMessage {
        msg_type: msg::AUTOMATON_EVENT,
        data2: code,
        data3: state,
        object: Some(a),
    };
    assert!(d.do_message(&ev(7, 5)));
    assert_eq!(w.posts(), ["post E94671EC 1 1"]);
    assert!(d.get_dispatch_info(id).unwrap().active);
    assert!(d.do_message(&ev(7, 5)));
    assert!(w.posts().is_empty());
    assert!(d.do_message(&ev(7, 3)));
    assert_eq!(w.posts(), ["post E94671ED 1 1"]);
    assert!(d.do_message(&ev(7, 5)));
    w.take();
    assert!(d.do_message(&ev(7, 0)));
    assert_eq!(w.posts(), ["post E94671ED 1 1", "post 894671F6 1 1"]);
    assert!(d.snapshot().records.is_empty());
    assert_eq!(w.refs(a), 0);
    assert_eq!(w.refs(STATION), 0);
    // Unknown automaton: true; other code or a null automaton: false.
    assert!(d.do_message(&ev(7, 5)));
    assert!(!d.do_message(&ev(4, 5)));
    assert!(!d.automaton_event(None, 3, 0));
    // Event 3: revoked.
    let id = d.dispatch(1, 1.0, 1.0);
    let a = d.get_dispatch_info(id).unwrap().automaton.unwrap();
    w.take();
    assert!(d.automaton_event(Some(a), 3, 0));
    assert_eq!(w.posts(), ["post 894671F6 1 2"]);
}

#[test]
fn dp12_07_revoke_active() {
    let (w, d) = setup();
    let id = d.dispatch(1, 1.0, 1.0);
    d.update_dispatch_state(id, 5);
    assert_eq!(w.mask(SRC), 0b110);
    w.take();
    assert!(d.revoke_dispatch_by_id(id));
    let log = w.take();
    assert_eq!(log.iter().filter(|e| e.starts_with("revoke_from_station")).count(), 1);
    let posts: Vec<&String> = log.iter().filter(|e| e.starts_with("post")).collect();
    assert_eq!(posts, ["post 894671F6 1 1"]);
    assert_eq!(w.mask(SRC), 0b111);
    assert!(!d.revoke_dispatch_by_id(id));
    // No source for the type: EndDispatch fails, the record is erased anyway.
    let id = d.dispatch(1, 1.0, 1.0);
    d.unregister_dispatch_source(SRC);
    assert!(!d.revoke_dispatch_by_id(id));
    assert!(d.snapshot().records.is_empty());
}

#[test]
fn dp12_08_revoke_all_ignores_type() {
    let (w, d) = setup();
    w.source(0x501, &[0], 0x601, 1, 2);
    d.register_dispatch_source(0x501);
    d.dispatch(1, 1.0, 1.0);
    d.dispatch(0, 2.0, 2.0);
    d.dispatch(0, 3.0, 3.0);
    assert_eq!(d.get_dispatch_count(0), 2);
    assert_eq!(d.get_dispatch_count(1), 1);
    assert!(d.revoke_all_dispatches(1));
    assert!(d.snapshot().records.is_empty());
}

#[test]
fn dp12_09_station_removed() {
    let (w, d) = setup();
    let a = d.dispatch(1, 1.0, 1.0);
    let b = d.dispatch(1, 2.0, 2.0);
    let (va, vb) = (
        d.get_dispatch_info(a).unwrap().automaton.unwrap(),
        d.get_dispatch_info(b).unwrap().automaton.unwrap(),
    );
    assert_eq!(w.refs(STATION), 2);
    w.take();
    assert!(d.do_message(&DispatchMessage {
        msg_type: msg::REMOVE_OCCUPANT,
        object: Some(STATION),
        ..Default::default()
    }));
    let log = w.take();
    assert!(d.snapshot().records.is_empty());
    assert!(!log
        .iter()
        .any(|e| e.starts_with("post") || e.starts_with("revoke_from_station")));
    assert!(log.contains(&format!("despawn {va:#x}")) && log.contains(&format!("despawn {vb:#x}")));
    assert_eq!(w.refs(STATION), 0);
    // DP7: the vehicles are despawned and released.
    assert_eq!((w.refs(va), w.refs(vb)), (0, 0));
}

#[test]
fn station_removed_available_pylon_order() {
    // DP7: an available pylon of the removed station is shut down and
    // released, then cleared from active (UI state 1, the active reference).
    let (w, d) = setup();
    d.begin_dispatch_tool(1);
    let p = d.snapshot().available_pylons[0].unwrap();
    d.set_active_pylon(Some(p));
    assert_eq!(w.refs(p), 2);
    w.take();
    d.station_removed(STATION);
    let log = w.take();
    assert_eq!(log.iter().filter(|e| **e == format!("shutdown {p:#x}")).count(), 1);
    assert_eq!(w.pylons.borrow()[&p].ui, 1);
    assert_eq!(w.refs(p), 0);
    assert_eq!(d.get_active_pylon(), None);
    assert!(d.snapshot().available_pylons.is_empty());
}

#[test]
fn begin_dispatch_tracks_new_vehicles_by_group() {
    // §4.2 step 5: kind 0 for occupant groups 0x4202 / 0x4303, else 1; every
    // new vehicle is tracked, whatever its dispatch type.
    let (w, d) = setup();
    w.source(0x501, &[dispatch_type::POLICE_CAR, 3], 0x601, 1, 3);
    d.register_dispatch_source(0x501);
    w.take();
    let a = d.dispatch(dispatch_type::FIRE_ENGINE, 1.0, 1.0);
    let b = d.dispatch(dispatch_type::POLICE_CAR, 1.0, 1.0);
    let c = d.dispatch(3, 1.0, 1.0);
    let veh = |id| d.get_dispatch_info(id).unwrap().automaton.unwrap();
    let log = w.take();
    assert!(log.contains(&format!("track {:#x} 0 -> 1", veh(a))));
    assert!(log.contains(&format!("track {:#x} 1 -> 2", veh(b))));
    assert!(log.contains(&format!("track {:#x} 1 -> 3", veh(c))));
    assert_eq!(d.get_dispatch_info(c).unwrap().view_handle, 3);
}

#[test]
fn retargeted_vehicle_is_not_tracked_again() {
    // DP9: a re-target keeps the record's handle, even 0, and tracks nothing.
    let (w, d) = setup();
    d.begin_dispatch_tool(1);
    let p1 = d.snapshot().available_pylons[0].unwrap();
    d.set_active_pylon(Some(p1));
    let id = d.dispatch(1, 10.0, 10.0);
    let i = d.snapshot().records.iter().position(|r| r.id == id).unwrap();
    d.with_state(|s| s.records[i].view_handle = 0);
    w.take();
    assert_eq!(d.dispatch(1, 30.0, 40.0), id);
    let log = w.take();
    assert!(!log.iter().any(|e| e.starts_with("track")));
    assert_eq!(d.get_dispatch_info(id).unwrap().view_handle, 0);
    assert!(log.contains(&"post E94671EB 1 1".to_string()));
}

#[test]
fn return_pylon_states_need_the_unit_state() {
    // §4.4: when the station's unit state is not available the pylon is not
    // placed and keeps its states, but still joins the available list.
    let (w, d) = setup();
    d.begin_dispatch_tool(1);
    let p1 = d.snapshot().available_pylons[0].unwrap();
    d.set_active_pylon(Some(p1));
    let id = d.dispatch(1, 10.0, 10.0);
    d.update_dispatch_state(id, 5);
    assert_eq!(w.pylons.borrow()[&p1].dstate, 5);
    w.park_fails.set(true);
    assert!(d.revoke_dispatch_by_pylon(p1));
    // Not reset to dispatch state 0 (SetActivePylon(null) gave UI state 1).
    assert_eq!(w.pylons.borrow()[&p1].dstate, 5);
    assert_eq!(
        d.snapshot().available_pylons.iter().filter(|&&e| e == Some(p1)).count(),
        1
    );
    // DP8: no source for the pylon's type: nothing at all.
    w.pylons.borrow_mut().get_mut(&p1).unwrap().dtype = 9;
    w.take();
    assert!(!d.revoke_dispatch_by_pylon(p1));
    assert!(w.take().is_empty());
}

#[test]
fn dp12_10_tool() {
    let (w, d) = setup();
    w.sources.borrow_mut().get_mut(&SRC).unwrap().stations[0].mask = 0b101;
    assert!(d.begin_dispatch_tool(1));
    let s = d.snapshot();
    assert_eq!(s.tool_type, 1);
    let calls: Vec<u32> = s
        .available_pylons
        .iter()
        .map(|p| w.pylon_call_number(p.unwrap()))
        .collect();
    assert_eq!(calls, [1, 3]);
    for p in s.available_pylons.iter().flatten() {
        let pl = w.pylons.borrow()[p].clone();
        assert_eq!((pl.dtype, pl.dstate, pl.ui, pl.source), (1, 0, 1, Some(STATION)));
    }
    assert_eq!(d.get_available_pylon_count(1), 2);
    assert_eq!(d.get_available_pylon_count(0), 0);
    assert!(d.begin_dispatch_tool(1));
    assert!(!d.begin_dispatch_tool(0));
    assert!(!d.end_dispatch_tool(0));
    let pylons: Vec<u64> = s.available_pylons.iter().flatten().copied().collect();
    assert!(d.end_dispatch_tool(1));
    assert!(d.snapshot().available_pylons.is_empty());
    assert_eq!(d.snapshot().tool_type, 8);
    for p in pylons {
        assert_eq!(w.refs(p), 0);
    }
    // Free units count only below `count`.
    w.sources.borrow_mut().get_mut(&SRC).unwrap().stations[0].mask = 0b1111_1111;
    assert_eq!(d.get_available_pylon_count(1), 3);
}

// ---- §4.7 and the tool ------------------------------------------------------------------------

#[test]
fn worked_example() {
    let (w, d) = setup();
    assert!(d.begin_dispatch_tool(1));
    let pylons: Vec<u64> = d.snapshot().available_pylons.iter().flatten().copied().collect();
    assert_eq!(pylons.len(), 3);
    let p2 = pylons[1];
    // The player drags pylon 2.
    d.set_active_pylon(Some(p2));
    assert_eq!(w.pylons.borrow()[&p2].ui, 2);
    assert!(d.can_dispatch(1, 50.0, 50.0));
    w.take();
    assert_eq!(d.dispatch(1, 50.0, 50.0), 1);
    let r = d.get_dispatch_info(1).unwrap();
    assert_eq!((r.call_id, r.pylon, r.station), (2, Some(p2), Some(STATION)));
    assert_eq!(w.mask(SRC), 0b101);
    assert_eq!(d.snapshot().available_pylons.len(), 2);
    assert_eq!(d.get_automaton_from_pylon(p2), r.automaton);
    assert_eq!(w.posts(), ["post E94671EB 1 1"]);
    d.set_active_pylon(None);
    let a = r.automaton.unwrap();
    d.automaton_event(Some(a), 7, 5);
    assert_eq!(w.posts(), ["post E94671EC 1 1"]);
    d.automaton_event(Some(a), 7, 0);
    assert_eq!(w.posts(), ["post E94671ED 1 1", "post 894671F6 1 1"]);
    assert_eq!(w.mask(SRC), 0b111);
    let s = d.snapshot();
    assert!(s.records.is_empty());
    assert_eq!(s.available_pylons, [Some(pylons[0]), Some(pylons[2]), Some(p2)]);
    assert_eq!(w.refs(p2), 1);
    let pl = w.pylons.borrow()[&p2].clone();
    assert_eq!((pl.dstate, pl.ui), (0, 1));
    // Saved now: 22 bytes plus three pylon references.
    let b = write(&d);
    assert_eq!(b.len(), 46);
    assert_eq!(&b[..18], hex("0200 00000000 01000000 00000000 03000000").as_slice());
    assert_eq!(&b[42..], hex("01000000").as_slice());
}

#[test]
fn drag_an_out_unit_retargets() {
    let (w, d) = setup();
    d.begin_dispatch_tool(1);
    let p1 = d.snapshot().available_pylons[0].unwrap();
    d.set_active_pylon(Some(p1));
    let id = d.dispatch(1, 10.0, 10.0);
    let a = d.get_dispatch_info(id).unwrap().automaton.unwrap();
    w.take();
    // Dragged again while out: the same record, re-targeted, begin again.
    assert_eq!(d.dispatch(1, 30.0, 40.0), id);
    let r = d.get_dispatch_info(id).unwrap();
    assert_eq!(r.target, (30.0, 0.7, 40.0));
    assert_eq!(r.automaton, Some(a));
    assert_eq!(d.snapshot().next_id, 1);
    assert_eq!(w.posts(), ["post E94671EB 1 1"]);
    // Dropped on its own station: revoked and parked.
    assert!(d.revoke_dispatch_by_pylon(p1));
    assert!(d.snapshot().records.is_empty());
    assert_eq!(
        d.snapshot().available_pylons.iter().filter(|&&e| e == Some(p1)).count(),
        1
    );
    assert!(!d.revoke_dispatch_by_pylon(p1));
    // EndDispatch cleared the active pylon; the list holds the only reference.
    assert_eq!(d.get_active_pylon(), None);
    assert_eq!(w.refs(p1), 1);
    // ID array.
    d.dispatch(1, 1.0, 1.0);
    d.dispatch(1, 2.0, 2.0);
    d.dispatch(1, 3.0, 3.0);
    assert_eq!(d.get_dispatch_id_array(1, 0), [2, 3, 4]);
    assert_eq!(d.get_dispatch_id_array(1, 2), [2, 3]);
    assert!(d.get_dispatch_id_array(0, 0).is_empty());
}

#[test]
fn messages_and_loaded_init() {
    let (w, d) = setup();
    let id = d.dispatch(1, 1.0, 1.0);
    let a = d.get_dispatch_info(id).unwrap().automaton.unwrap();
    w.take();
    assert!(d.do_message(&DispatchMessage {
        msg_type: msg::DISASTER_PERIOD_ENDED,
        ..Default::default()
    }));
    assert!(d.do_message(&DispatchMessage {
        msg_type: msg::VIEW_CHANGED,
        ..Default::default()
    }));
    assert_eq!(
        w.take(),
        [format!("go_home {a:#x}"), "refresh view Some(126)".to_string()]
    );
    assert_eq!(d.get_dispatch_count(1), 1);
    // Save, load into a fresh manager, Init re-tracks the vehicle.
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut out = ByteWriter::with_registry(reg.clone());
    assert!(d.write(&mut out));
    let bytes = out.into_bytes();
    let entries = reg.borrow().entries.clone();
    struct Back(Vec<crate::stream::RegisteredObject>);
    impl RefResolver for Back {
        fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
            let e = self.0.iter().find(|e| e.0 == class_id && e.1 == object_id)?;
            Some(RefValue::Host { token: e.3, class_id })
        }
    }
    let w2 = World::new();
    // The view allocates opaque handles process-wide, independently of the
    // manager's IDs or the number of records loaded (dispatch.md 4.1.3).
    w2.next_handle.set(0xF001);
    let d2 = DispatchManager::new(w2.clone(), 2);
    let mut back = Back(entries);
    assert!(d2.read(&mut ByteReader::with_resolver(&bytes, &mut back)));
    let r = d2.get_dispatch_info(id).unwrap();
    assert_eq!((r.station, r.automaton, r.view_handle), (Some(STATION), Some(a), 0));
    assert_eq!(w2.refs(STATION), 1);
    d2.init();
    assert!(w2.take().contains(&format!("track {a:#x} 0 -> {}", 0xF001)));
    assert_eq!(d2.get_dispatch_info(id).unwrap().view_handle, 0xF001);
    // Read appends.
    let mut back = Back(reg.borrow().entries.clone());
    assert!(d2.read(&mut ByteReader::with_resolver(&bytes, &mut back)));
    assert_eq!(d2.get_dispatch_count(1), 2);
}
