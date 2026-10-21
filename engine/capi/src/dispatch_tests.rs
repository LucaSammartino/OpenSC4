// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The dispatch manager through the C ABI (docs/specs/dispatch.md): a fake
//! city behind `fc4_dispatch_host`.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::c_void;

use crate::dispatch::*;
use crate::test_streams::{istream, ostream, In, Out};

const SRC: u64 = 0x500;
const STATION: u64 = 0x600;
const VIEW: u64 = 0x7E;
const MANAGER: u64 = 0xD15;

#[derive(Clone, Default)]
struct Pylon {
    source: u64,
    call: u32,
    dtype: u32,
    dstate: u32,
    ui: u32,
}

#[derive(Default)]
struct Game {
    log: RefCell<Vec<String>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    /// The one source: its types, CanDispatch, all-busy, and its station.
    types: RefCell<Vec<u32>>,
    can: Cell<bool>,
    all_busy: Cell<bool>,
    station: Cell<fc4_station_units>,
    pylons: RefCell<BTreeMap<u64, Pylon>>,
    next: Cell<u64>,
    next_handle: Cell<u32>,
}

impl Game {
    fn new() -> Box<Game> {
        let g = Box::new(Game::default());
        g.next.set(0x1000);
        g.next_handle.set(1);
        g.can.set(true);
        *g.types.borrow_mut() = vec![1];
        g.station.set(fc4_station_units {
            station: STATION,
            mask: 0b111,
            base: 1,
            count: 3,
            reserved: 0,
        });
        g
    }
    fn user(&self) -> *mut c_void {
        self as *const Game as *mut c_void
    }
    fn log(&self, s: impl Into<String>) {
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
    fn unit(&self, call: u32, free: bool) {
        let mut st = self.station.get();
        let bit = 1u32 << (call - st.base);
        if free {
            st.mask |= bit;
        } else {
            st.mask &= !bit;
        }
        self.station.set(st);
    }
    fn pylon<R>(&self, p: u64, f: impl FnOnce(&mut Pylon) -> R) -> R {
        f(self.pylons.borrow_mut().get_mut(&p).unwrap())
    }
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

unsafe extern "C" fn acquire_message_server(u: *mut c_void) -> bool {
    game(u).log("acquire ms");
    true
}
unsafe extern "C" fn subscribe(u: *mut c_void, m: u32) {
    game(u).log(format!("subscribe {m:08X}"));
}
unsafe extern "C" fn unsubscribe(u: *mut c_void, m: u32) {
    game(u).log(format!("unsubscribe {m:08X}"));
}
unsafe extern "C" fn release_message_server(u: *mut c_void) {
    game(u).log("release ms");
}
unsafe extern "C" fn post(u: *mut c_void, m: u32, t: u32, id: u32) {
    game(u).log(format!("post {m:08X} {t} {id}"));
}
unsafe extern "C" fn acquire_city(u: *mut c_void) -> bool {
    game(u).log("acquire city");
    true
}
unsafe extern "C" fn release_city(u: *mut c_void) {
    game(u).log("release city");
}
unsafe extern "C" fn create_vehicle_view(u: *mut c_void) -> u64 {
    game(u).log("create view");
    VIEW
}
unsafe extern "C" fn destroy_vehicle_view(u: *mut c_void, v: u64) {
    game(u).log(format!("destroy view {v:#x}"));
}
unsafe extern "C" fn refresh_vehicle_view(u: *mut c_void, v: u64) {
    game(u).log(format!("refresh view {v:#x}"));
}
unsafe extern "C" fn track_vehicle(u: *mut c_void, a: u64, kind: u32) -> u32 {
    let g = game(u);
    let h = g.next_handle.get();
    g.next_handle.set(h + 1);
    g.log(format!("track {a:#x} {kind} -> {h}"));
    h
}
unsafe extern "C" fn untrack_vehicle(u: *mut c_void, h: u32) {
    game(u).log(format!("untrack {h}"));
}
unsafe extern "C" fn add_ref(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_insert(0) += 1;
}
unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_insert(0) -= 1;
}
unsafe extern "C" fn altitude(_: *mut c_void, x: f32, z: f32) -> f32 {
    (x + z) / 100.0
}
unsafe extern "C" fn source_can_handle(u: *mut c_void, src: u64, t: u32) -> bool {
    src == SRC && game(u).types.borrow().contains(&t)
}
unsafe extern "C" fn source_can_dispatch(u: *mut c_void, _src: u64, _x: f32, _z: f32, busy: *mut bool) -> bool {
    *busy = game(u).all_busy.get();
    game(u).can.get()
}
unsafe extern "C" fn source_dispatch_from_nearest_station(
    u: *mut c_void,
    _src: u64,
    _x: f32,
    _z: f32,
    call: *mut u32,
) -> u64 {
    let g = game(u);
    let st = g.station.get();
    let Some(i) = (0..st.count).find(|i| st.mask & (1 << i) != 0) else {
        return 0;
    };
    g.unit(st.base + i, false);
    *call = st.base + i;
    g.log(format!("nearest {:#x} {}", st.station, st.base + i));
    st.station
}
unsafe extern "C" fn source_dispatch_from_station(u: *mut c_void, _src: u64, station: u64, call: *mut u32) -> bool {
    game(u).unit(*call, false);
    game(u).log(format!("from_station {station:#x} {}", *call));
    true
}
unsafe extern "C" fn source_revoke_dispatch_from_station(u: *mut c_void, _src: u64, station: u64, call: u32) {
    game(u).unit(call, true);
    game(u).log(format!("revoke_from_station {station:#x} {call}"));
}
unsafe extern "C" fn source_stations(u: *mut c_void, _src: u64, out: *mut fc4_station_units, cap: usize) -> usize {
    if !out.is_null() && cap >= 1 {
        *out = game(u).station.get();
    }
    1
}
unsafe extern "C" fn begin_vehicle(u: *mut c_void, _src: u64, info: *const fc4_dispatch_info) -> u64 {
    let g = game(u);
    let i = &*info;
    let a = if i.automaton != 0 { i.automaton } else { g.fresh() };
    g.log(format!("vehicle {a:#x} id {} call {}", i.id, i.call_id));
    a
}
unsafe extern "C" fn automaton_despawn(u: *mut c_void, a: u64) {
    game(u).log(format!("despawn {a:#x}"));
}
unsafe extern "C" fn automaton_go_home(u: *mut c_void, a: u64) {
    game(u).log(format!("go_home {a:#x}"));
}
unsafe extern "C" fn create_pylon(u: *mut c_void) -> u64 {
    let g = game(u);
    let p = g.fresh();
    g.pylons.borrow_mut().insert(p, Pylon::default());
    g.log(format!("create pylon {p:#x}"));
    p
}
unsafe extern "C" fn pylon_source(u: *mut c_void, p: u64) -> u64 {
    game(u).pylon(p, |p| p.source)
}
unsafe extern "C" fn pylon_set_source(u: *mut c_void, p: u64, st: u64) {
    game(u).pylon(p, |p| p.source = st)
}
unsafe extern "C" fn pylon_call_number(u: *mut c_void, p: u64) -> u32 {
    game(u).pylon(p, |p| p.call)
}
unsafe extern "C" fn pylon_set_call_number(u: *mut c_void, p: u64, n: u32) {
    game(u).pylon(p, |p| p.call = n)
}
unsafe extern "C" fn pylon_dispatch_type(u: *mut c_void, p: u64) -> u32 {
    game(u).pylon(p, |p| p.dtype)
}
unsafe extern "C" fn pylon_set_dispatch_type(u: *mut c_void, p: u64, t: u32) {
    game(u).pylon(p, |p| p.dtype = t)
}
unsafe extern "C" fn pylon_set_dispatch_state(u: *mut c_void, p: u64, s: u32) {
    game(u).pylon(p, |p| p.dstate = s)
}
unsafe extern "C" fn pylon_set_ui_state(u: *mut c_void, p: u64, s: u32) {
    game(u).pylon(p, |p| p.ui = s)
}
unsafe extern "C" fn pylon_move_to(u: *mut c_void, p: u64, x: f32, y: f32, z: f32) {
    game(u).log(format!("move {p:#x} {x} {y} {z}"));
}
unsafe extern "C" fn pylon_place_at_unit(u: *mut c_void, p: u64, st: u64, i: u32, n: u32) {
    game(u).log(format!("place {p:#x} {st:#x} {i}/{n}"));
}
unsafe extern "C" fn pylon_return_to_station(u: *mut c_void, _src: u64, p: u64, st: u64) -> bool {
    game(u).log(format!("park {p:#x} {st:#x}"));
    true
}
/// Every vehicle of this one-source city is a fire engine (group 0x4202).
unsafe extern "C" fn vehicle_in_group(_u: *mut c_void, _a: u64, g: u32) -> bool {
    g == 0x4202
}
unsafe extern "C" fn pylon_shutdown(u: *mut c_void, p: u64) {
    game(u).log(format!("shutdown {p:#x}"));
}
unsafe extern "C" fn resolve_ref(u: *mut c_void, t: u64, _iid: u32) -> u64 {
    add_ref(u, t);
    t
}

fn host(g: &Game) -> fc4_dispatch_host {
    fc4_dispatch_host {
        struct_size: std::mem::size_of::<fc4_dispatch_host>() as u32,
        user: g.user(),
        acquire_message_server: Some(acquire_message_server),
        subscribe: Some(subscribe),
        unsubscribe: Some(unsubscribe),
        release_message_server: Some(release_message_server),
        post: Some(post),
        acquire_city: Some(acquire_city),
        release_city: Some(release_city),
        create_vehicle_view: Some(create_vehicle_view),
        destroy_vehicle_view: Some(destroy_vehicle_view),
        refresh_vehicle_view: Some(refresh_vehicle_view),
        track_vehicle: Some(track_vehicle),
        untrack_vehicle: Some(untrack_vehicle),
        add_ref: Some(add_ref),
        release: Some(release),
        altitude: Some(altitude),
        source_can_handle: Some(source_can_handle),
        source_can_dispatch: Some(source_can_dispatch),
        source_dispatch_from_nearest_station: Some(source_dispatch_from_nearest_station),
        source_dispatch_from_station: Some(source_dispatch_from_station),
        source_revoke_dispatch_from_station: Some(source_revoke_dispatch_from_station),
        source_stations: Some(source_stations),
        begin_vehicle: Some(begin_vehicle),
        automaton_despawn: Some(automaton_despawn),
        automaton_go_home: Some(automaton_go_home),
        create_pylon: Some(create_pylon),
        pylon_source: Some(pylon_source),
        pylon_set_source: Some(pylon_set_source),
        pylon_call_number: Some(pylon_call_number),
        pylon_set_call_number: Some(pylon_set_call_number),
        pylon_dispatch_type: Some(pylon_dispatch_type),
        pylon_set_dispatch_type: Some(pylon_set_dispatch_type),
        pylon_set_dispatch_state: Some(pylon_set_dispatch_state),
        pylon_set_ui_state: Some(pylon_set_ui_state),
        pylon_move_to: Some(pylon_move_to),
        pylon_place_at_unit: Some(pylon_place_at_unit),
        pylon_return_to_station: Some(pylon_return_to_station),
        pylon_shutdown: Some(pylon_shutdown),
        resolve_ref: Some(resolve_ref),
        vehicle_in_group: Some(vehicle_in_group),
    }
}

fn create(g: &Game) -> *mut fc4_dispatch {
    let d = unsafe { fc4_dispatch_create(&host(g), MANAGER) };
    assert!(!d.is_null());
    d
}

/// Initialised, the source registered, the log cleared.
fn ready(g: &Game) -> *mut fc4_dispatch {
    let d = create(g);
    unsafe {
        assert!(fc4_dispatch_init(d));
        fc4_dispatch_register_source(d, SRC);
    }
    g.take();
    d
}

fn state(d: *const fc4_dispatch) -> fc4_dispatch_state {
    let mut s = fc4_dispatch_state::default();
    assert!(unsafe { fc4_dispatch_get_state(d, &mut s) });
    s
}

fn info(d: *const fc4_dispatch, id: u32) -> fc4_dispatch_info {
    let mut i = fc4_dispatch_info::default();
    assert!(unsafe { fc4_dispatch_get_info(d, id, &mut i) });
    i
}

fn pylons(d: *const fc4_dispatch) -> Vec<u64> {
    unsafe {
        let n = fc4_dispatch_available_pylons(d, std::ptr::null_mut(), 0);
        let mut v = vec![0u64; n];
        assert_eq!(fc4_dispatch_available_pylons(d, v.as_mut_ptr(), n), n);
        v
    }
}

fn write(d: *const fc4_dispatch) -> Out {
    let mut o = Out::default();
    let os = ostream(&mut o);
    assert!(unsafe { fc4_dispatch_write(d, &os) });
    o
}

#[test]
fn create_rejects_bad_tables_and_identity() {
    let g = Game::new();
    unsafe {
        assert!(fc4_dispatch_create(std::ptr::null(), 0).is_null());
        let mut h = host(&g);
        h.struct_size -= 1;
        assert!(fc4_dispatch_create(&h, 0).is_null());
        let mut off = 99u32;
        for (iid, want) in [(1, 0), (0x6940_28EC, 0), (0x4522_94AA, 4), (0xE4FD_A3D4, 8)] {
            assert!(fc4_dispatch_query_interface(iid, &mut off), "{iid:08X}");
            assert_eq!(off, want);
        }
        assert!(!fc4_dispatch_query_interface(0xE98B_2F57, &mut off));
        assert_eq!(fc4_dispatch_dispatch(std::ptr::null_mut(), 1, 0.0, 0.0), 0);
        fc4_dispatch_destroy(std::ptr::null_mut());
    }
}

#[test]
fn lifecycle_and_sources() {
    let g = Game::new();
    let d = create(&g);
    assert_eq!(state(d).tool_type, 8);
    assert!(unsafe { fc4_dispatch_init(d) });
    assert_eq!(
        g.take(),
        [
            "acquire ms",
            "subscribe 99EF1143",
            "subscribe AA1B2002",
            "subscribe A6B79602",
            "create view",
            "acquire city"
        ]
    );
    let s = state(d);
    assert_eq!(s.struct_size as usize, std::mem::size_of::<fc4_dispatch_state>());
    assert_eq!((s.initialized, s.message_server_held, s.city_held), (1, 1, 1));
    assert_eq!(s.vehicle_view, VIEW);
    unsafe {
        assert_eq!(fc4_dispatch_get_vehicle_view(d), VIEW);
        fc4_dispatch_register_source(d, SRC);
        fc4_dispatch_register_source(d, SRC);
        assert_eq!(g.refs(SRC), 2);
        assert_eq!(fc4_dispatch_get_source(d, 1), SRC);
        assert_eq!(fc4_dispatch_get_source(d, 0), 0);
        fc4_dispatch_unregister_source(d, SRC);
        assert_eq!(g.refs(SRC), 1);
        assert_eq!(state(d).sources, 1);
        assert_eq!(fc4_dispatch_add_ref(d), 1);
        assert_eq!(fc4_dispatch_release(d), 0);
    }
    assert_eq!(
        g.take(),
        [
            "unsubscribe 99EF1143",
            "unsubscribe AA1B2002",
            "unsubscribe A6B79602",
            "release ms",
            "destroy view 0x7e",
            "release city"
        ]
    );
    assert_eq!(g.refs(SRC), 0);
    assert_eq!(state(d).destroyed, 1);
    unsafe { fc4_dispatch_destroy(d) };
}

#[test]
fn dp12_01_02_record_round_trip() {
    let g = Game::new();
    let d = create(&g);
    let o = write(d);
    // u16 2, no records, nextID 0, a null active pylon, no pylons, type 8.
    let mut want = vec![2u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    want.extend_from_slice(&[0; 12]);
    want.extend_from_slice(&[0, 0, 0, 0, 8, 0, 0, 0]);
    assert_eq!(o.bytes, want);
    // A record with nextID 1 and tool type 4 round-trips; Init resets the type.
    let mut rec = want.clone();
    rec[6] = 1;
    rec[26] = 4;
    let d2 = create(&g);
    let mut i = In::new(&rec);
    let is = istream(&mut i);
    assert!(unsafe { fc4_dispatch_read(d2, &is) });
    assert_eq!(write(d2).bytes, rec);
    let s = state(d2);
    assert_eq!((s.loaded, s.next_id, s.tool_type), (1, 1, 4));
    assert!(unsafe { fc4_dispatch_init(d2) });
    assert_eq!(state(d2).tool_type, 8);
    // dp12_03: version 3 fails with loaded set.
    let mut bad = want.clone();
    bad[0] = 3;
    let d3 = create(&g);
    let mut i = In::new(&bad);
    let is = istream(&mut i);
    assert!(!unsafe { fc4_dispatch_read(d3, &is) });
    assert_eq!(state(d3).loaded, 1);
    unsafe {
        fc4_dispatch_destroy(d);
        fc4_dispatch_destroy(d2);
        fc4_dispatch_destroy(d3);
    }
}

#[test]
fn dp12_04_dispatch_and_dp12_06_events() {
    let g = Game::new();
    let d = ready(&g);
    unsafe {
        assert!(fc4_dispatch_can_dispatch(d, 1, 10.0, 20.0));
        assert!(!fc4_dispatch_can_dispatch(d, 0, 10.0, 20.0));
        assert_eq!(fc4_dispatch_dispatch(d, 1, 10.0, 20.0), 1);
    }
    let r = info(d, 1);
    assert_eq!((r.dispatch_type, r.id, r.call_id), (1, 1, 1));
    assert_eq!((r.target_x, r.target_y, r.target_z), (10.0, 0.3, 20.0));
    assert_eq!(r.station, STATION);
    assert!(r.automaton != 0 && r.view_handle != 0 && r.pylon == 0);
    assert_eq!(g.refs(STATION), 1);
    let log = g.take();
    assert_eq!(log.iter().filter(|e| e.starts_with("post")).count(), 1);
    assert!(log.contains(&"post E94671EB 1 1".to_string()));
    unsafe {
        assert_eq!(fc4_dispatch_get_count(d, 1), 1);
        let n = fc4_dispatch_get_id_array(d, 1, 0, std::ptr::null_mut(), 0);
        let mut ids = vec![0u32; n];
        assert_eq!(fc4_dispatch_get_id_array(d, 1, 0, ids.as_mut_ptr(), n), 1);
        assert_eq!(ids, [1]);
        let mut recs = [fc4_dispatch_info::default(); 2];
        assert_eq!(fc4_dispatch_records(d, recs.as_mut_ptr(), 2), 1);
        assert_eq!(recs[0], r);
        let mut x = fc4_dispatch_info::default();
        assert!(!fc4_dispatch_get_info(d, 99, &mut x));
    }
    // The state events: 5 posts Active once, 0 ends the dispatch.
    let a = r.automaton;
    unsafe {
        assert!(fc4_dispatch_do_message(d, 0x09D9_2FC3, 7, 5, a));
        assert_eq!(g.posts(), ["post E94671EC 1 1"]);
        assert_eq!(info(d, 1).active, 1);
        assert!(fc4_dispatch_do_message(d, 0x09D9_2FC3, 7, 5, a));
        assert!(g.posts().is_empty());
        assert!(fc4_dispatch_do_message(d, 0x09D9_2FC3, 7, 0, a));
        assert_eq!(g.posts(), ["post E94671ED 1 1", "post 894671F6 1 1"]);
        assert_eq!(state(d).records, 0);
        assert_eq!(g.refs(a), 0);
        assert_eq!(g.refs(STATION), 0);
        assert!(!fc4_dispatch_do_message(d, 0x09D9_2FC3, 4, 5, a));
        // CanDispatch false: 0, no ID consumed.
        g.can.set(false);
        assert_eq!(fc4_dispatch_dispatch(d, 1, 0.0, 0.0), 0);
        assert_eq!(state(d).next_id, 1);
        fc4_dispatch_destroy(d);
    }
}

#[test]
fn dp12_07_revoke_and_dp12_09_station_removed() {
    let g = Game::new();
    let d = ready(&g);
    unsafe {
        let id = fc4_dispatch_dispatch(d, 1, 1.0, 1.0);
        fc4_dispatch_update_state(d, id, 5);
        assert_eq!(g.station.get().mask, 0b110);
        g.take();
        assert!(fc4_dispatch_revoke_by_id(d, id));
        assert_eq!(g.station.get().mask, 0b111);
        assert!(!fc4_dispatch_revoke_by_id(d, id));
        assert_eq!(g.posts(), ["post 894671F6 1 1"]);
        // A removed station takes its records with it.
        fc4_dispatch_dispatch(d, 1, 1.0, 1.0);
        fc4_dispatch_dispatch(d, 1, 2.0, 2.0);
        assert_eq!(g.refs(STATION), 2);
        assert!(fc4_dispatch_do_message(d, 0x99EF_1143, 0, 0, STATION));
        assert_eq!(state(d).records, 0);
        assert_eq!(g.refs(STATION), 0);
        // RevokeAll ignores the type.
        fc4_dispatch_dispatch(d, 1, 1.0, 1.0);
        assert!(fc4_dispatch_revoke_all(d, 0));
        assert_eq!(state(d).records, 0);
        fc4_dispatch_destroy(d);
    }
}

#[test]
fn dp12_10_tool_and_worked_example() {
    let g = Game::new();
    let d = ready(&g);
    let mut st = g.station.get();
    st.mask = 0b101;
    g.station.set(st);
    unsafe {
        assert!(fc4_dispatch_begin_tool(d, 1));
        assert_eq!(state(d).tool_type, 1);
        let ps = pylons(d);
        let calls: Vec<u32> = ps.iter().map(|&p| g.pylon(p, |p| p.call)).collect();
        assert_eq!(calls, [1, 3]);
        for &p in &ps {
            let pl = g.pylon(p, |p| p.clone());
            assert_eq!((pl.dtype, pl.dstate, pl.ui, pl.source), (1, 0, 1, STATION));
        }
        assert_eq!(fc4_dispatch_get_available_pylon_count(d, 1), 2);
        assert!(!fc4_dispatch_begin_tool(d, 0));
        assert!(fc4_dispatch_end_tool(d, 1));
        assert!(pylons(d).is_empty());
        for p in ps {
            assert_eq!(g.refs(p), 0);
        }
        // §4.7: drag pylon 2 and dispatch with it.
        st.mask = 0b111;
        g.station.set(st);
        assert!(fc4_dispatch_begin_tool(d, 1));
        let ps = pylons(d);
        let p2 = ps[1];
        fc4_dispatch_set_active_pylon(d, p2);
        assert_eq!(fc4_dispatch_get_active_pylon(d), p2);
        assert_eq!(g.pylon(p2, |p| p.ui), 2);
        g.take();
        assert_eq!(fc4_dispatch_dispatch(d, 1, 50.0, 50.0), 1);
        let r = info(d, 1);
        assert_eq!((r.call_id, r.pylon, r.station), (2, p2, STATION));
        assert_eq!(fc4_dispatch_get_automaton_from_pylon(d, p2), r.automaton);
        assert_eq!(g.posts(), ["post E94671EB 1 1"]);
        fc4_dispatch_set_active_pylon(d, 0);
        assert_eq!(fc4_dispatch_get_active_pylon(d), 0);
        assert!(fc4_dispatch_do_message(d, 0x09D9_2FC3, 7, 5, r.automaton));
        assert!(fc4_dispatch_do_message(d, 0x09D9_2FC3, 7, 0, r.automaton));
        assert_eq!(pylons(d), [ps[0], ps[2], p2]);
        assert_eq!(g.refs(p2), 1);
        // Saved: three pylon references come back through resolve_ref.
        let o = write(d);
        assert_eq!(o.refs.iter().filter(|r| r.0 != 0).count(), 3);
        let d2 = create(&g);
        let mut i = In::new(&o.bytes);
        let is = istream(&mut i);
        assert!(fc4_dispatch_read(d2, &is));
        assert_eq!(pylons(d2), [ps[0], ps[2], p2]);
        assert_eq!(g.refs(p2), 2);
        assert_eq!(state(d2).tool_type, 1);
        fc4_dispatch_destroy(d);
        fc4_dispatch_destroy(d2);
    }
}
