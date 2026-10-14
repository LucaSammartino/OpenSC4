// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The commercial simulator and the industrial stub through the C ABI
//! (docs/specs/commercial-industrial.md): a fake game behind
//! `fc4_commercial_host` and its grid table.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::{c_char, c_void, CStr};

use crate::commercial::*;
use crate::hostgrid::fc4_grid_api;
use crate::landvalue::fc4_grid_geometry;
use crate::test_streams::{istream, ostream, In, Out};

const U8: u32 = 0x49B9_E602;
const CO2: u32 = 0x89D5_B788;
const CO3: u32 = 0x89D5_B6EF;
const MAP: u64 = 0x9000;
const OFFICE_KEY_CO2: f64 = 15.0;
const OFFICE_KEY_CO3: f64 = 16.0;

struct Grid {
    instance: u32,
    data: Vec<f64>,
    refs: i32,
    shut: bool,
}

#[derive(Default)]
struct Game {
    cells: i32,
    grids: RefCell<BTreeMap<u64, Grid>>,
    next: Cell<u64>,
    log: RefCell<Vec<String>>,
    nodes: RefCell<BTreeMap<u64, i32>>,
    no_node_3800: Cell<bool>,
    purposes: RefCell<BTreeMap<u64, u32>>,
}

impl Game {
    fn new(cells: i32) -> Box<Game> {
        let g = Box::new(Game {
            cells,
            ..Game::default()
        });
        g.next.set(0x100);
        g.grids.borrow_mut().insert(
            MAP,
            Grid {
                instance: 0,
                data: vec![0.0; (cells * cells) as usize],
                refs: 1,
                shut: false,
            },
        );
        g
    }
    fn user(&self) -> *mut c_void {
        self as *const Game as *mut c_void
    }
    fn log(&self, s: impl Into<String>) {
        self.log.borrow_mut().push(s.into());
    }
    fn set_map(&self, x: i32, z: i32, v: f64) {
        let n = self.cells;
        self.grids.borrow_mut().get_mut(&MAP).unwrap().data[(x * n + z) as usize] = v;
    }
    fn cells_of(&self, t: u64) -> Vec<Vec<u8>> {
        let n = self.cells;
        let g = self.grids.borrow();
        let d = &g[&t].data;
        (0..n)
            .map(|x| (0..n).map(|z| d[(x * n + z) as usize] as u8).collect())
            .collect()
    }
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

// ---- grids ------------------------------------------------------------------------------------------

unsafe extern "C" fn g_create(u: *mut c_void, clsid: u32, instance: u32, ts: i32) -> u64 {
    assert_eq!((clsid, ts), (U8, 1), "a uint8 grid with tract size 1");
    let g = game(u);
    let t = g.next.get();
    g.next.set(t + 0x10);
    g.grids.borrow_mut().insert(
        t,
        Grid {
            instance,
            data: vec![0.0; (g.cells * g.cells) as usize],
            refs: 1,
            shut: false,
        },
    );
    g.log(format!("create {instance:08X}"));
    t
}
unsafe extern "C" fn g_class(u: *mut c_void, t: u64) -> u32 {
    if game(u).grids.borrow().contains_key(&t) {
        U8
    } else {
        0
    }
}
unsafe extern "C" fn g_geometry(u: *mut c_void, _t: u64, out: *mut fc4_grid_geometry) {
    let n = game(u).cells;
    *out = fc4_grid_geometry {
        tract_size: 1,
        shift: 0,
        count_x: n,
        count_z: n,
        width_x: 16.0,
        width_z: 16.0,
        inv_width_x: 1.0 / 16.0,
        inv_width_z: 1.0 / 16.0,
    };
}
unsafe extern "C" fn g_get(u: *mut c_void, t: u64, kind: u32, x: i32, z: i32) -> f64 {
    assert_eq!(kind, U8);
    let g = game(u);
    let n = g.cells;
    if x < 0 || z < 0 || x >= n || z >= n {
        return 0.0;
    }
    g.grids.borrow()[&t].data[(x * n + z) as usize]
}
unsafe extern "C" fn g_set(u: *mut c_void, t: u64, kind: u32, x: i32, z: i32, v: f64) {
    assert_eq!(kind, U8);
    assert_ne!(t, MAP, "the building map is never written");
    let g = game(u);
    let n = g.cells;
    if x >= 0 && z >= 0 && x < n && z < n {
        g.grids.borrow_mut().get_mut(&t).unwrap().data[(x * n + z) as usize] = v;
    }
}
unsafe extern "C" fn g_fill(u: *mut c_void, t: u64, _kind: u32, v: f64) {
    game(u).grids.borrow_mut().get_mut(&t).unwrap().data.fill(v);
}
unsafe extern "C" fn g_shutdown(u: *mut c_void, t: u64) {
    let g = game(u);
    g.grids.borrow_mut().get_mut(&t).unwrap().shut = true;
    g.log(format!("grid shutdown {t:X}"));
}
unsafe extern "C" fn g_release(u: *mut c_void, t: u64) {
    let g = game(u);
    g.grids.borrow_mut().get_mut(&t).unwrap().refs -= 1;
    g.log(format!("grid release {t:X}"));
}

// ---- the rest of the host -----------------------------------------------------------------------------

unsafe extern "C" fn add_agent(u: *mut c_void, level: u32, name: *const c_char, flags: u32) {
    let n = CStr::from_ptr(name).to_string_lossy().into_owned();
    game(u).log(format!("agent {level} {n} {flags}"));
}
unsafe extern "C" fn remove_agent(u: *mut c_void) {
    game(u).log("remove agent");
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
unsafe extern "C" fn acquire_city(u: *mut c_void) -> bool {
    game(u).log("acquire city");
    true
}
unsafe extern "C" fn release_city(u: *mut c_void) {
    game(u).log("release city");
}
unsafe extern "C" fn acquire_bds(u: *mut c_void) -> bool {
    game(u).log("acquire bds");
    true
}
unsafe extern "C" fn release_bds(u: *mut c_void) {
    game(u).log("release bds");
}
unsafe extern "C" fn has_demand_simulator(_: *mut c_void) -> bool {
    true
}
unsafe extern "C" fn demand_node(u: *mut c_void, id: u32) -> u64 {
    let g = game(u);
    if id == 0x3800 && g.no_node_3800.get() {
        return 0;
    }
    let t = 0x5_0000 + u64::from(id);
    *g.nodes.borrow_mut().entry(t).or_default() += 1;
    t
}
unsafe extern "C" fn release_demand_node(u: *mut c_void, node: u64) {
    *game(u).nodes.borrow_mut().get_mut(&node).unwrap() -= 1;
}
unsafe extern "C" fn building_map(_: *mut c_void) -> u64 {
    MAP
}
unsafe extern "C" fn building_map_key(_: *mut c_void, purpose: u32, wealth: u32) -> u8 {
    assert_eq!(purpose, 3, "office");
    match wealth {
        2 => OFFICE_KEY_CO2 as u8,
        3 => OFFICE_KEY_CO3 as u8,
        _ => 0,
    }
}
unsafe extern "C" fn building_purpose(u: *mut c_void, occ: u64, out: *mut u32) -> bool {
    match game(u).purposes.borrow().get(&occ) {
        Some(&p) => {
            *out = p;
            true
        }
        None => false,
    }
}

fn host(g: &Game) -> fc4_commercial_host {
    fc4_commercial_host {
        struct_size: std::mem::size_of::<fc4_commercial_host>() as u32,
        user: g.user(),
        add_agent: Some(add_agent),
        remove_agent: Some(remove_agent),
        acquire_message_server: Some(acquire_message_server),
        subscribe: Some(subscribe),
        unsubscribe: Some(unsubscribe),
        release_message_server: Some(release_message_server),
        acquire_city: Some(acquire_city),
        release_city: Some(release_city),
        acquire_bds: Some(acquire_bds),
        release_bds: Some(release_bds),
        has_demand_simulator: Some(has_demand_simulator),
        demand_node: Some(demand_node),
        release_demand_node: Some(release_demand_node),
        building_map: Some(building_map),
        building_map_key: Some(building_map_key),
        building_purpose: Some(building_purpose),
        grids: fc4_grid_api {
            struct_size: std::mem::size_of::<fc4_grid_api>() as u32,
            user: g.user(),
            create: Some(g_create),
            class_of: Some(g_class),
            geometry: Some(g_geometry),
            get: Some(g_get),
            set: Some(g_set),
            fill: Some(g_fill),
            shutdown: Some(g_shutdown),
            release: Some(g_release),
        },
    }
}

fn state(c: *const fc4_commercial) -> fc4_commercial_state {
    let mut s = fc4_commercial_state::default();
    assert!(unsafe { fc4_commercial_get_state(c, &mut s) });
    s
}

fn write(c: *const fc4_commercial) -> Out {
    let mut o = Out::default();
    let os = ostream(&mut o);
    assert!(unsafe { fc4_commercial_write(c, &os) });
    o
}

/// The §4.4 result on an n-square map: (x, 0) = 1 except at `zero`, the
/// rest 0.
fn edge(n: i32, zero: &[i32]) -> Vec<Vec<u8>> {
    (0..n)
        .map(|x| (0..n).map(|z| u8::from(z == 0 && !zero.contains(&x))).collect())
        .collect()
}

#[test]
fn create_rejects_bad_tables() {
    let g = Game::new(8);
    unsafe {
        assert!(fc4_commercial_create(std::ptr::null()).is_null());
        let mut h = host(&g);
        h.struct_size -= 1;
        assert!(fc4_commercial_create(&h).is_null());
        let mut h = host(&g);
        h.grids.struct_size = 0;
        assert!(fc4_commercial_create(&h).is_null(), "the grid table is required");
    }
}

#[test]
fn ci13_01_identity_over_the_abi() {
    for iid in [0x0000_0001, 0x077A_BEB2, 0x4522_94AA, 0xE4FD_A3D4] {
        assert!(fc4_commercial_query_interface(iid), "{iid:08X}");
    }
    assert!(!fc4_commercial_query_interface(0x477A_9E7D));
    for iid in [0x0000_0001, 0x477A_9E7D] {
        assert!(fc4_industrial_query_interface(iid), "{iid:08X}");
    }
    assert!(!fc4_industrial_query_interface(0x077A_BEB2));
}

#[test]
fn ci13_02_init_lifecycle_and_shutdown() {
    let g = Game::new(8);
    unsafe {
        let c = fc4_commercial_create(&host(&g));
        assert!(!c.is_null());
        let s = state(c);
        assert_eq!(
            (s.initialized, s.dirty, s.state, s.flag),
            (0, 1, 3, 1),
            "constructor state"
        );
        assert!(fc4_commercial_init(c));
        let log = g.log.borrow().clone();
        assert_eq!(
            log,
            [
                "agent 0 cSC4CommercialSimulator 2",
                "agent 10 cSC4CommercialSimulator 2",
                "agent 4 cSC4CommercialSimulator 2",
                "agent 6 cSC4CommercialSimulator 2",
                "acquire ms",
                "subscribe 99EF1142",
                "subscribe 99EF1143",
                "acquire city",
                "acquire bds",
                "create 89D5B6EF",
                "create 89D5B788",
            ]
        );
        let s = state(c);
        assert_eq!(s.demand_nodes, [0x5_3000, 0x5_3800]);
        assert_ne!(s.co2_grid, 0);
        assert_ne!(s.co3_grid, 0);
        assert_eq!(g.grids.borrow()[&s.co2_grid].instance, CO2);
        assert_eq!(g.grids.borrow()[&s.co3_grid].instance, CO3);
        assert_eq!(fc4_commercial_proximity_map(c, 0x3320), s.co2_grid);
        assert_eq!(fc4_commercial_proximity_map(c, 0x3330), s.co3_grid);
        assert_eq!(fc4_commercial_proximity_map(c, 0x3310), 0);

        // Release: above 1 decrements; at 1 runs Shutdown and returns 0.
        assert_eq!(fc4_commercial_add_ref(c), 1);
        assert_eq!(fc4_commercial_add_ref(c), 2);
        assert_eq!(fc4_commercial_release(c), 1);
        g.log.borrow_mut().clear();
        assert_eq!(fc4_commercial_release(c), 0);
        let s2 = state(c);
        assert_eq!((s2.initialized, s2.destroyed), (0, 1));
        assert_eq!((s2.co2_grid, s2.co3_grid), (0, 0));
        assert!(g.nodes.borrow().values().all(|&n| n == 0), "nodes released");
        let grids = g.grids.borrow();
        for t in [s.co2_grid, s.co3_grid] {
            assert!(grids[&t].shut);
            assert_eq!(grids[&t].refs, 0);
        }
        let log = g.log.borrow();
        assert!(log.contains(&"remove agent".to_string()));
        assert!(log.contains(&"unsubscribe 99EF1142".to_string()));
        assert!(log.contains(&"release ms".to_string()));
        drop(log);
        drop(grids);
        fc4_commercial_destroy(c);
    }
}

#[test]
fn ci13_02_missing_node_fails_init() {
    let g = Game::new(8);
    g.no_node_3800.set(true);
    unsafe {
        let c = fc4_commercial_create(&host(&g));
        assert!(!fc4_commercial_init(c));
        let s = state(c);
        assert_eq!(s.initialized, 1);
        assert_eq!(s.demand_nodes, [0x5_3000, 0]);
        assert_eq!((s.co2_grid, s.co3_grid), (0, 0), "no grids");
        fc4_commercial_destroy(c);
    }
    assert_eq!(
        g.nodes.borrow()[&0x5_3000],
        1,
        "destroy without Shutdown keeps the host's count"
    );
}

#[test]
fn ci13_04_ci13_05_rebuild_through_messages() {
    let g = Game::new(8);
    g.set_map(3, 0, OFFICE_KEY_CO2);
    g.set_map(5, 5, OFFICE_KEY_CO2);
    g.set_map(6, 0, OFFICE_KEY_CO3);
    unsafe {
        let c = fc4_commercial_create(&host(&g));
        assert!(fc4_commercial_init(c));
        // Idle with dirty → 0; then CO$$; then CO$$$ and idle.
        assert!(fc4_commercial_do_message(c, 0x6695_681A, 0, 0, 0, 0));
        assert_eq!(state(c).state, 0);
        assert!(fc4_commercial_do_message(c, 0x6695_6814, 0, 0, 0, 0));
        assert_eq!(state(c).state, 2);
        fc4_commercial_do_idle(c);
        let s = state(c);
        assert_eq!((s.state, s.dirty, s.flag), (3, 0, 0));
        assert_eq!(g.cells_of(s.co2_grid), edge(8, &[3]));
        assert_eq!(g.cells_of(s.co3_grid), edge(8, &[6]));

        // An office building marks the maps dirty; another purpose does not.
        g.purposes.borrow_mut().insert(7, 3);
        g.purposes.borrow_mut().insert(8, 1);
        assert!(fc4_commercial_do_message(c, 0x99EF_1142, 0, 0, 0x2781_28A0, 8));
        assert_eq!(state(c).dirty, 0);
        assert!(fc4_commercial_do_message(c, 0x99EF_1142, 0, 0, 0x1234, 7));
        assert_eq!(state(c).dirty, 0, "not a building occupant type");
        assert!(fc4_commercial_do_message(c, 0x99EF_1143, 0, 0, 0x2781_28A0, 7));
        assert_eq!(state(c).dirty, 1);
        fc4_commercial_destroy(c);
    }
}

#[test]
fn ci13_08_record_round_trip_over_the_abi() {
    let g = Game::new(8);
    unsafe {
        let c = fc4_commercial_create(&host(&g));
        assert!(fc4_commercial_init(c));
        let s = state(c);
        let o = write(c);
        assert_eq!(o.bytes.len(), 3 + 2 * 12, "version, flag, two (token, class) test refs");
        assert_eq!(&o.bytes[..3], &[1, 0, 1]);
        assert_eq!(o.refs, [(s.co2_grid, U8), (s.co3_grid, U8)], "CO$$ then CO$$$");

        // A fresh object reads the record: the grids' references move to it.
        for t in [s.co2_grid, s.co3_grid] {
            g.grids.borrow_mut().get_mut(&t).unwrap().refs += 1;
        }
        let d = fc4_commercial_create(&host(&g));
        let mut i = In::new(&o.bytes);
        let is = istream(&mut i);
        assert!(fc4_commercial_read(d, &is));
        let r = state(d);
        assert_eq!((r.co2_grid, r.co3_grid), (s.co2_grid, s.co3_grid));
        assert_eq!((r.dirty, r.state), (1, 3));
        assert_eq!(write(d).bytes, o.bytes, "write → read → write");
        // Init keeps the loaded grids.
        g.log.borrow_mut().clear();
        assert!(fc4_commercial_init(d));
        assert!(!g.log.borrow().iter().any(|l| l.starts_with("create")));

        // Version 2 is refused.
        let mut v2 = o.bytes.clone();
        v2[0] = 2;
        let e = fc4_commercial_create(&host(&g));
        let mut i = In::new(&v2);
        let is = istream(&mut i);
        assert!(!fc4_commercial_read(e, &is));
        assert_eq!(state(e).co2_grid, 0);
        for p in [c, d, e] {
            fc4_commercial_destroy(p);
        }
    }
}

#[test]
fn ci13_08_industrial_over_the_abi() {
    unsafe {
        let p = fc4_industrial_create();
        assert!(!fc4_industrial_is_initialized(p));
        assert!(fc4_industrial_init(p));
        assert!(fc4_industrial_is_initialized(p));
        assert!(fc4_industrial_do_message(p, 0x6695_6816));
        let mut o = Out::default();
        let os = ostream(&mut o);
        assert!(fc4_industrial_write(p, &os));
        assert_eq!(o.bytes, [1, 0]);
        let mut i = In::new(&o.bytes);
        let is = istream(&mut i);
        assert!(fc4_industrial_read(p, &is));
        let bad = [2u8, 0];
        let mut i = In::new(&bad);
        let is = istream(&mut i);
        assert!(!fc4_industrial_read(p, &is));
        assert_eq!(fc4_industrial_add_ref(p), 1);
        assert_eq!(fc4_industrial_add_ref(p), 2);
        assert_eq!(fc4_industrial_release(p), 1);
        assert_eq!(fc4_industrial_release(p), 0);
        fc4_industrial_destroy(p);
    }
}

#[test]
fn null_handles_are_harmless() {
    unsafe {
        assert!(!fc4_commercial_init(std::ptr::null_mut()));
        assert_eq!(fc4_commercial_proximity_map(std::ptr::null(), 0x3320), 0);
        fc4_commercial_do_idle(std::ptr::null_mut());
        fc4_commercial_destroy(std::ptr::null_mut());
        assert!(!fc4_industrial_init(std::ptr::null_mut()));
        fc4_industrial_destroy(std::ptr::null_mut());
    }
}
