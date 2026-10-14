// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The traffic unit's C ABI against a fake game: the identity, `Init` on a
//! new city (agents, subscriptions, grids, departments, the units per
//! slice), `Shutdown` and the reference audit, the save record's round trip
//! in the load order (`Read` before `Init`, the loaded grids and departments
//! kept), re-entrant calls (snapshot answers, queued messages), the work
//! cycle over days and a month, the query slots (edge density record,
//! strategy table, capacities through a holder, the statistics block), and
//! slot 6's pathfinders (FindPath, the route, the compressed route, the
//! flag-2 fallback).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;

use fc4_core::traffic_network::{travel, EDGE_DENSITY_ZERO};
use fc4_core::traffic_simulator::{self as ts, tuning::DEFAULT_UNITS_PER_SLICE, SimTuning};

use crate::budget::fc4_token_callback;
use crate::hostgrid::{fc4_cell_rect, fc4_grid_api};
use crate::landvalue::fc4_grid_geometry;
use crate::props::{fc4_props, fc4_props_set_typed};
use crate::stream::fc4_ref;
use crate::test_streams::{istream, ostream, In, Out};
use crate::traffic::*;

const GRID_BASE: u64 = 0x6000;
const DEPT_BASE: u64 = 0xD000;

struct GridData {
    kind: u32,
    instance: u32,
    tract: i32,
    nx: i32,
    nz: i32,
    v: Vec<f64>,
}

/// A holder property as the game keeps it: (mem type, array, element bytes,
/// count).
type Prop = (u16, bool, Vec<u8>, usize);

#[derive(Default)]
struct Game {
    cx: i32,
    cz: i32,
    grids: RefCell<HashMap<u64, GridData>>,
    next_grid: Cell<u64>,
    grid_shutdowns: Cell<u32>,
    refs: RefCell<HashMap<u64, i32>>,
    agents: RefCell<Vec<(u32, String, u32)>>,
    subs: RefCell<Vec<u32>>,
    posts: RefCell<Vec<fc4_traffic_post>>,
    depts: RefCell<Vec<(u32, u64, f32)>>,
    released_depts: RefCell<Vec<u64>>,
    history: RefCell<Vec<(u32, u32)>>,
    day: Cell<u32>,
    date: Cell<u32>,
    holders: RefCell<HashMap<(u64, u32), Prop>>,
    /// Re-entrancy: the handle `add_agent` calls back into, once.
    reenter: Cell<usize>,
    reentrant_pct: RefCell<Vec<[f32; 3]>>,
    reentrant_scale: RefCell<Vec<f32>>,
    reentrant_msg: RefCell<Vec<bool>>,
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

impl Game {
    fn new(cx: i32, cz: i32) -> Box<Game> {
        let g = Box::new(Game {
            cx,
            cz,
            ..Default::default()
        });
        g.next_grid.set(GRID_BASE);
        g.date.set(1000);
        g
    }
    fn user(&self) -> *mut c_void {
        self as *const Game as *mut c_void
    }
    fn refs_of(&self, t: u64) -> i32 {
        self.refs.borrow().get(&t).copied().unwrap_or(0)
    }
    fn bump(&self, t: u64, d: i32) {
        *self.refs.borrow_mut().entry(t).or_insert(0) += d;
    }
    fn grids_of(&self, kind: u32) -> usize {
        self.grids.borrow().values().filter(|g| g.kind == kind).count()
    }
    fn live_refs(&self) -> Vec<(u64, i32)> {
        let mut v: Vec<(u64, i32)> = self
            .refs
            .borrow()
            .iter()
            .filter(|e| *e.1 != 0)
            .map(|(&t, &n)| (t, n))
            .collect();
        v.sort();
        v
    }
}

// ---- grids ------------------------------------------------------------------------------------------

unsafe extern "C" fn g_create(u: *mut c_void, kind: u32, instance: u32, tract: i32) -> u64 {
    let g = game(u);
    assert!(
        matches!(kind, 0x49B9_E602 | 0x49B9_E606 | 0x49B9_E60A),
        "u8, u32 or f32"
    );
    let t = g.next_grid.get();
    g.next_grid.set(t + 1);
    let tract = tract.max(1);
    let (nx, nz) = ((g.cx + tract - 1) / tract, (g.cz + tract - 1) / tract);
    g.grids.borrow_mut().insert(
        t,
        GridData {
            kind,
            instance,
            tract,
            nx,
            nz,
            v: vec![0.0; (nx * nz) as usize],
        },
    );
    g.bump(t, 1);
    t
}
unsafe extern "C" fn g_class(u: *mut c_void, t: u64) -> u32 {
    game(u).grids.borrow().get(&t).map_or(0, |g| g.kind)
}
unsafe extern "C" fn g_geometry(u: *mut c_void, t: u64, out: *mut fc4_grid_geometry) {
    let grids = game(u).grids.borrow();
    let Some(g) = grids.get(&t) else { return };
    let w = 16.0 * g.tract as f32;
    *out = fc4_grid_geometry {
        tract_size: g.tract,
        shift: g.tract.trailing_zeros() as i32,
        count_x: g.nx,
        count_z: g.nz,
        width_x: w,
        width_z: w,
        inv_width_x: 1.0 / w,
        inv_width_z: 1.0 / w,
    };
}
unsafe extern "C" fn g_get(u: *mut c_void, t: u64, _k: u32, x: i32, z: i32) -> f64 {
    let grids = game(u).grids.borrow();
    let g = &grids[&t];
    g.v[(x * g.nz + z) as usize]
}
unsafe extern "C" fn g_set(u: *mut c_void, t: u64, _k: u32, x: i32, z: i32, v: f64) {
    let mut grids = game(u).grids.borrow_mut();
    let g = grids.get_mut(&t).unwrap();
    let nz = g.nz;
    g.v[(x * nz + z) as usize] = v;
}
unsafe extern "C" fn g_fill(u: *mut c_void, t: u64, _k: u32, v: f64) {
    game(u).grids.borrow_mut().get_mut(&t).unwrap().v.fill(v);
}
unsafe extern "C" fn g_shutdown(u: *mut c_void, _t: u64) {
    let g = game(u);
    g.grid_shutdowns.set(g.grid_shutdowns.get() + 1);
}
unsafe extern "C" fn g_release(u: *mut c_void, t: u64) {
    game(u).bump(t, -1);
}

fn grid_api(g: &Game) -> fc4_grid_api {
    fc4_grid_api {
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
    }
}

// ---- the host ---------------------------------------------------------------------------------------

unsafe extern "C" fn add_agent(u: *mut c_void, level: u32, name: *const u8, len: usize, flags: u32) {
    let g = game(u);
    let n = String::from_utf8_lossy(std::slice::from_raw_parts(name, len)).into_owned();
    g.agents.borrow_mut().push((level, n, flags));
    let p = g.reenter.replace(0) as *mut fc4_traffic;
    if !p.is_null() {
        let mut pct = [9.0f32; 3];
        fc4_traffic_travel_strategy_percentages(p, 1, pct.as_mut_ptr());
        g.reentrant_pct.borrow_mut().push(pct);
        g.reentrant_scale.borrow_mut().push(fc4_traffic_scalar(p, 0));
        let r = fc4_traffic_do_message(p, ts::msg::SIM_BEGIN, 0, 0, 0, 0);
        g.reentrant_msg.borrow_mut().push(r);
    }
}
unsafe extern "C" fn subscribe(u: *mut c_void, m: u32) {
    game(u).subs.borrow_mut().push(m);
}
unsafe extern "C" fn post_message(u: *mut c_void, m: *const fc4_traffic_post) {
    game(u).posts.borrow_mut().push(*m);
}
unsafe extern "C" fn add_ref(u: *mut c_void, t: u64) {
    game(u).bump(t, 1);
}
unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    game(u).bump(t, -1);
}
unsafe extern "C" fn cell_counts(u: *mut c_void, x: *mut i32, z: *mut i32) {
    *x = game(u).cx;
    *z = game(u).cz;
}
unsafe extern "C" fn position_to_cell(_: *mut c_void, x: f32, z: f32, cx: *mut i32, cz: *mut i32) {
    *cx = (x / 16.0).floor() as i32;
    *cz = (z / 16.0).floor() as i32;
}
unsafe extern "C" fn cell_width(_: *mut c_void) -> f32 {
    16.0
}
unsafe extern "C" fn tuning(_: *mut c_void) -> *mut fc4_props {
    Box::into_raw(Box::new(SimTuning::spec_exemplar())) as *mut fc4_props
}
unsafe extern "C" fn day_of_month(u: *mut c_void) -> u32 {
    game(u).day.get()
}
unsafe extern "C" fn date_number(u: *mut c_void) -> u32 {
    game(u).date.get()
}
unsafe extern "C" fn clock_ms(u: *mut c_void) -> u64 {
    u64::from(game(u).date.get()) * 1000
}
unsafe extern "C" fn create_department(u: *mut c_void, id: u32, max: f32) -> u64 {
    let g = game(u);
    let t = DEPT_BASE + g.depts.borrow().len() as u64;
    g.depts.borrow_mut().push((id, t, max));
    g.bump(t, 1);
    t
}
unsafe extern "C" fn release_department(u: *mut c_void, t: u64) {
    let g = game(u);
    g.released_depts.borrow_mut().push(t);
    g.bump(t, -1);
}
unsafe extern "C" fn department_funding(_: *mut c_void, _t: u64) -> f32 {
    1.0
}
unsafe extern "C" fn record_history(u: *mut c_void, id: u32, v: u32) {
    game(u).history.borrow_mut().push((id, v));
}
unsafe extern "C" fn no_lots(_: *mut c_void, _cb: fc4_token_callback, _ctx: *mut c_void) {}
unsafe extern "C" fn holder_property(u: *mut c_void, h: u64, id: u32, out: *mut fc4_props) -> bool {
    let holders = game(u).holders.borrow();
    let Some((mem, arr, bytes, n)) = holders.get(&(h, id)) else {
        return false;
    };
    fc4_props_set_typed(out, id, *mem, *arr, bytes.as_ptr() as *const c_void, *n)
}
unsafe extern "C" fn set_holder_property(
    u: *mut c_void,
    h: u64,
    id: u32,
    mem: u16,
    arr: bool,
    data: *const c_void,
    n: usize,
) {
    let size = match mem {
        0x01..=0x03 => 1,
        0x04 | 0x05 => 2,
        0x06 | 0x07 | 0x0A => 4,
        _ => 8,
    };
    let bytes = std::slice::from_raw_parts(data as *const u8, size * n).to_vec();
    game(u).holders.borrow_mut().insert((h, id), (mem, arr, bytes, n));
}

fn host(g: &Game) -> fc4_traffic_host {
    // SAFETY: every field is an integer, a pointer or an Option of a
    // function pointer: all-zero is NULL / None.
    let mut h: fc4_traffic_host = unsafe { std::mem::zeroed() };
    h.struct_size = std::mem::size_of::<fc4_traffic_host>() as u32;
    h.user = g.user();
    h.grids = grid_api(g);
    h.add_agent = Some(add_agent);
    h.subscribe = Some(subscribe);
    h.post_message = Some(post_message);
    h.add_ref = Some(add_ref);
    h.release = Some(release);
    h.cell_counts = Some(cell_counts);
    h.position_to_cell = Some(position_to_cell);
    h.cell_width = Some(cell_width);
    h.tuning = Some(tuning);
    h.sim_day_of_month = Some(day_of_month);
    h.sim_date_number = Some(date_number);
    h.sim_clock_ms = Some(clock_ms);
    h.create_department = Some(create_department);
    h.release_department = Some(release_department);
    h.department_funding = Some(department_funding);
    h.record_history = Some(record_history);
    h.lots = Some(no_lots);
    h.all_connections = Some(no_lots);
    h.holder_property = Some(holder_property);
    h.set_holder_property = Some(set_holder_property);
    h
}

fn state(p: *const fc4_traffic) -> fc4_traffic_state {
    let mut s = fc4_traffic_state::default();
    assert!(unsafe { fc4_traffic_get_state(p, &mut s) });
    s
}

fn new_city(g: &Game) -> *mut fc4_traffic {
    let h = host(g);
    let p = unsafe { fc4_traffic_create(&h, 7) };
    assert!(!p.is_null());
    p
}

fn write(p: *const fc4_traffic) -> Out {
    let mut o = Out::default();
    let os = ostream(&mut o);
    assert!(unsafe { fc4_traffic_write(p, &os) });
    o
}

// ---- identity -------------------------------------------------------------------------------------

#[test]
fn identity_and_query_interface() {
    for iid in [1, 0x6802_7F37, 0x4522_94AA, 0xE4FD_A3D4] {
        assert!(fc4_traffic_query_interface(iid), "{iid:#x}");
    }
    for iid in [0, 0xAA0D_B766, 0x6802_7F36, 0x8802_7F0D] {
        assert!(!fc4_traffic_query_interface(iid), "{iid:#x}");
    }
    assert_eq!(fc4_traffic_class_id(), 0x6990_C1AA);
    assert_eq!(fc4_traffic_simulator_type(), 0x8802_7F0D);
    assert_eq!(fc4_traffic_default_units_per_slice(), DEFAULT_UNITS_PER_SLICE);
}

#[test]
fn incomplete_host_is_refused() {
    let g = Game::new(8, 8);
    let mut h = host(&g);
    h.struct_size = 8;
    assert!(unsafe { fc4_traffic_create(&h, 0) }.is_null());
    let mut h = host(&g);
    h.grids.struct_size = 8;
    assert!(unsafe { fc4_traffic_create(&h, 0) }.is_null());
    assert!(unsafe { fc4_traffic_create(std::ptr::null(), 0) }.is_null());
    // NULL handles answer the fallbacks.
    unsafe {
        assert!(!fc4_traffic_init(std::ptr::null_mut(), 0));
        assert!(fc4_traffic_do_message(std::ptr::null_mut(), 1, 0, 0, 0, 0));
        assert_eq!(fc4_traffic_scalar(std::ptr::null(), 0), 0.0);
        assert!(fc4_traffic_pf_create(std::ptr::null_mut()).is_null());
        fc4_traffic_destroy(std::ptr::null_mut());
    }
}

// ---- Init, Shutdown, references ----------------------------------------------------------------------

#[test]
fn init_registers_agents_grids_and_departments() {
    let g = Game::new(16, 16);
    let p = new_city(&g);
    unsafe {
        assert_eq!(state(p).initialized, 0);
        assert!(fc4_traffic_init(p, 0));
        let agents = g.agents.borrow().clone();
        assert_eq!(
            agents.iter().map(|a| a.0).collect::<Vec<_>>(),
            ts::AGENT_LEVELS.to_vec()
        );
        assert!(agents.iter().all(|a| a.1 == ts::AGENT_NAME && a.2 == ts::AGENT_FLAGS));
        assert_eq!(*g.subs.borrow(), ts::msg::SUBSCRIPTIONS.to_vec());
        // u8: air, commercial, commercial work, unused, trip length,
        // congestion and the network's switch mode grid; f32: peak volume,
        // six volumes, float, peak ratio; u32: the switch ID grid.
        assert_eq!(g.grids_of(0x49B9_E602), 7);
        assert_eq!(g.grids_of(0x49B9_E60A), 9);
        assert_eq!(g.grids_of(0x49B9_E606), 1);
        let depts: Vec<u32> = g.depts.borrow().iter().map(|d| d.0).collect();
        assert_eq!(depts, vec![ts::dept::ROADS, ts::dept::MASS_TRANSIT]);
        let s = state(p);
        assert_eq!(s.initialized, 1);
        assert_eq!(s.units_per_slice, DEFAULT_UNITS_PER_SLICE);
        assert_eq!((s.roads, s.mass_transit), (DEPT_BASE, DEPT_BASE + 1));
        for which in 0..4 {
            assert_ne!(fc4_traffic_map(p, which), 0);
        }
        assert_eq!(fc4_traffic_map(p, 4), 0);
        assert_eq!(fc4_traffic_map(p, FC4_TRAFFIC_MAP_CONGESTION), s.congestion);
        // The spec tuning (§4.14): commuteMaxTime 6, display multiplier 25.
        assert_eq!(fc4_traffic_scalar(p, 0), 6.0);
        assert_eq!(fc4_traffic_scalar(p, 1), 25.0);
        // A second Init changes nothing.
        assert!(fc4_traffic_init(p, 0));
        assert_eq!(g.grids_of(0x49B9_E602), 7);
        assert_eq!(g.depts.borrow().len(), 2);

        assert!(fc4_traffic_shutdown(p));
        assert_eq!(state(p).initialized, 0);
        assert_eq!(*g.released_depts.borrow(), vec![DEPT_BASE, DEPT_BASE + 1]);
        assert_eq!(g.grid_shutdowns.get(), 17);
        assert_eq!(fc4_traffic_map(p, FC4_TRAFFIC_MAP_AIR), 0);
        fc4_traffic_destroy(p);
    }
    assert_eq!(g.live_refs(), vec![], "every grid and department given back");
}

#[test]
fn units_per_slice_override_and_destroy_without_shutdown() {
    let g = Game::new(8, 8);
    let p = new_city(&g);
    unsafe {
        assert!(fc4_traffic_init(p, 5));
        assert_eq!(state(p).units_per_slice, 5);
        // Destroy shuts down what is still held.
        fc4_traffic_destroy(p);
    }
    assert_eq!(g.live_refs(), vec![]);
}

// ---- re-entrancy ----------------------------------------------------------------------------------

#[test]
fn reentrant_calls_use_the_snapshot_and_queue_messages() {
    let g = Game::new(8, 8);
    let p = new_city(&g);
    unsafe {
        g.reenter.set(p as usize);
        assert!(fc4_traffic_init(p, 0));
        // Before Init's tuning: the constructor's values (no panic, no
        // RefCell conflict).
        assert_eq!(g.reentrant_pct.borrow().len(), 1);
        assert_eq!(*g.reentrant_msg.borrow(), vec![true]);
        let s = state(p);
        assert!(s.reentrant_calls >= 2, "{}", s.reentrant_calls);
        assert_eq!(s.queued_messages, 1);
        // Outside a call the core answers.
        let mut pct = [0f32; 3];
        fc4_traffic_travel_strategy_percentages(p, 1, pct.as_mut_ptr());
        assert_eq!(pct, core_of(p).travel_strategy_percentages(1));
        let mut none = [7f32; 3];
        fc4_traffic_travel_strategy_percentages(p, 9, none.as_mut_ptr());
        assert_eq!(none, [0.0; 3]);
        fc4_traffic_destroy(p);
    }
    assert_eq!(g.live_refs(), vec![]);
}

// ---- the work cycle -------------------------------------------------------------------------------

#[test]
fn days_run_the_cycle_and_a_month_reports() {
    let g = Game::new(16, 16);
    let p = new_city(&g);
    unsafe {
        assert!(fc4_traffic_init(p, 0));
        assert!(fc4_traffic_do_message(p, ts::msg::SIM_BEGIN, 0, 0, 0, 0));
        assert!(fc4_traffic_do_message(p, ts::msg::POST_CITY_INIT, 0, 0, 0, 0));
        let mut seen = vec![state(p).cycle_state];
        for d in 0..120u32 {
            g.day.set(d % 30 + 1);
            g.date.set(1000 + d);
            fc4_traffic_do_message(p, ts::msg::DAY, 0, 0, 0, 0);
            let c = state(p).cycle_state;
            if seen.last() != Some(&c) {
                seen.push(c);
            }
        }
        assert!(seen.len() > 3, "the cycle moves: {seen:?}");
        assert!(
            g.posts
                .borrow()
                .iter()
                .any(|m| m.msg_type == ts::msg::TRAFFIC_MAP_CHANGED),
            "CreateFinalMap publishes the maps"
        );
        let before = g.history.borrow().len();
        assert!(fc4_traffic_do_message(p, ts::msg::MONTH, 0, 0, 0, 0));
        assert!(g.history.borrow().len() > before, "the month records history");
        assert!(fc4_traffic_shutdown(p));
        fc4_traffic_destroy(p);
    }
    assert_eq!(g.live_refs(), vec![]);
}

// ---- the save record ------------------------------------------------------------------------------

#[test]
fn save_round_trip_in_load_order() {
    let g = Game::new(16, 16);
    let a = new_city(&g);
    let (bytes, refs) = unsafe {
        assert!(fc4_traffic_init(a, 0));
        assert!(fc4_traffic_do_message(a, ts::msg::POST_CITY_INIT, 0, 0, 0, 0));
        for d in 0..10u32 {
            g.day.set(d + 1);
            fc4_traffic_do_message(a, ts::msg::DAY, 0, 0, 0, 0);
        }
        let o = write(a);
        (o.bytes, o.refs)
    };
    assert_eq!(bytes[..2], ts::RECORD_VERSION.to_le_bytes());
    // Every grid and both departments go out by reference.
    let grid_refs = refs.iter().filter(|r| g.grids.borrow().contains_key(&r.0)).count();
    assert_eq!(grid_refs, 16, "the simulator's 16 grids");
    assert!(refs.iter().any(|r| r.0 == DEPT_BASE));
    assert!(refs.iter().any(|r| r.0 == DEPT_BASE + 1));
    let made = g.grids.borrow().len();

    // Load: a new object, Read before Init (the reference handed over by
    // the stream is the core's).
    let b = new_city(&g);
    unsafe {
        let resolve = |r: &mut fc4_ref| {
            if r.token != 0 {
                g.bump(r.token, 1);
            }
        };
        let mut i = In::new(&bytes);
        i.resolve = Some(&resolve);
        let is = istream(&mut i);
        assert!(fc4_traffic_read(b, &is));
        assert_eq!(i.pos, bytes.len(), "the whole record is read");
        let s = state(b);
        assert_eq!((s.initialized, s.loaded), (0, 1));
        assert!(fc4_traffic_init(b, 0));
        // The loaded grids are kept (HG1); only the one grid the record
        // does not hold (the unused u8 grid `0x69D5C3B5`) is Init's.
        let fresh: Vec<(u32, u32)> = g
            .grids
            .borrow()
            .iter()
            .filter(|e| *e.0 >= GRID_BASE + made as u64)
            .map(|e| (e.1.kind, e.1.instance))
            .collect();
        assert_eq!(fresh, vec![(0x49B9_E602, ts::grid_id::UNUSED)]);
        assert_eq!(g.depts.borrow().len(), 2, "the loaded departments are kept");
        let again = write(b);
        assert_eq!(again.bytes, bytes);
        // A truncated record changes nothing and fails.
        let c = new_city(&g);
        let mut i = In::new(&bytes[..40]);
        i.resolve = Some(&resolve);
        let is = istream(&mut i);
        assert!(!fc4_traffic_read(c, &is));
        assert_eq!(state(c).loaded, 0);
        fc4_traffic_destroy(c);

        fc4_traffic_destroy(a);
        assert!(fc4_traffic_shutdown(b));
        assert_eq!(g.refs_of(DEPT_BASE), 0, "the loaded department went back at Shutdown");
        fc4_traffic_destroy(b);
    }
    // Every reference the reads took (the truncated one included) is given
    // back once, and nothing is released twice.
    assert_eq!(g.live_refs(), vec![]);
}

// ---- query slots ----------------------------------------------------------------------------------

#[test]
fn edge_density_record_layout() {
    let g = Game::new(8, 8);
    let p = new_city(&g);
    unsafe {
        assert!(fc4_traffic_init(p, 0));
        let key = fc4_core::traffic_network::NetworkState::edge_key(2, 3, 1, true);
        core_of(p).network().update_state(|n| {
            let r = n.edge_density.entry(key, || EDGE_DENSITY_ZERO);
            r.last = std::array::from_fn(|i| i as u16);
            r.usage = 77;
            r.current = std::array::from_fn(|i| 100 + i as u16);
        });
        let mut out = [0xFFFFu16; 33];
        assert!(fc4_traffic_edge_density(p, 2, 3, 1, true, out.as_mut_ptr()));
        assert_eq!(out[0], 0);
        assert_eq!(out[15], 15);
        assert_eq!(out[16], 77);
        assert_eq!(out[17], 100);
        assert_eq!(out[32], 115);
        let mut zero = [0xFFFFu16; 33];
        assert!(!fc4_traffic_edge_density(p, 2, 3, 1, false, zero.as_mut_ptr()));
        assert_eq!(zero, [0; 33]);
        assert_eq!(state(p).edge_records, 1);
        fc4_traffic_destroy(p);
    }
}

#[test]
fn trip_capacity_through_the_holder() {
    let g = Game::new(8, 8);
    let p = new_city(&g);
    unsafe {
        assert!(fc4_traffic_init(p, 0));
        const H: u64 = 0x4242;
        fc4_traffic_set_max_trip_capacity(p, H, 1, 77);
        assert_eq!(fc4_traffic_max_trip_capacity(p, H, 1), 77);
        assert_eq!(fc4_traffic_max_trip_capacity(p, H, 0), 0);
        assert_eq!(fc4_traffic_traffic_arrived(p, H, 1), 0);
        let holders = g.holders.borrow();
        let (mem, arr, _, n) = holders
            .get(&(H, ts::prop::TRIP_CAPACITY))
            .expect("written to the holder");
        assert!(*arr);
        assert_eq!(*mem, 0x04, "u16 elements");
        assert!(*n >= 4);
        drop(holders);
        fc4_traffic_destroy(p);
    }
}

#[test]
fn statistics_block_layout() {
    let g = Game::new(8, 8);
    let p = new_city(&g);
    unsafe {
        assert!(fc4_traffic_init(p, 0));
        core_of(p).update_state(|s| {
            s.stats_last.rows[3].sum = 1.5;
            s.stats_last.rows[3].congested = 3;
            s.stats_last.rows[3].max_x = 4;
            s.stats_last.rows[3].max_z = 5;
            s.stats_last.rows[3].max = 2.5;
            s.stats_last.records[3] = 5;
            s.stats_last.potholes = 9;
            s.stats_last.bridge.sum = 0.25;
        });
        let mut b = [0u8; FC4_TRAFFIC_STATS_BLOCK];
        assert_eq!(
            fc4_traffic_stats_block(p, b.as_mut_ptr(), b.len()),
            FC4_TRAFFIC_STATS_BLOCK
        );
        let f = |at: usize| f32::from_le_bytes(b[at..at + 4].try_into().unwrap());
        let u = |at: usize| u32::from_le_bytes(b[at..at + 4].try_into().unwrap());
        assert_eq!(f(3 * 0x14), 1.5);
        assert_eq!(u(3 * 0x14 + 4), 3);
        assert_eq!(u(3 * 0x14 + 8), 4);
        assert_eq!(u(3 * 0x14 + 12), 5);
        assert_eq!(f(3 * 0x14 + 16), 2.5);
        assert_eq!(f(13 * 0x14), 0.25);
        assert_eq!(u16::from_le_bytes([b[0x118 + 6], b[0x119 + 6]]), 5);
        assert_eq!(u16::from_le_bytes([b[0x134], b[0x135]]), 9);
        assert_eq!(state(p).potholes, 9);
        fc4_traffic_destroy(p);
    }
}

// ---- slot 6: pathfinders ----------------------------------------------------------------------------

/// The pathfinder §11 test 2 street, (0,3)…(4,3) car street cells.
fn street(p: *const fc4_traffic) {
    unsafe { core_of(p) }.network().update_state(|n| {
        for x in 0..=4 {
            let c = n.conn_mut(x, 3).unwrap();
            c.conn[travel::CAR] = 0x0104;
            c.switch[travel::CAR] = 0xFFFF;
            c.prior_car = [0xFFFF; 4];
            c.wormhole = 0;
            let r = n.append_info(1, x, 3).unwrap();
            r.inv_cap = 0.0625;
            r.net_type = fc4_core::traffic_network::net_type::STREET as u8;
            r.extra_cost = 0.0;
        }
    });
}

unsafe extern "C" fn push_bytes(ctx: *mut c_void, b: *const u8, n: usize) {
    let v = &mut *(ctx as *mut Vec<Vec<u8>>);
    v.push(if n == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(b, n).to_vec()
    });
}

fn configure(pf: *mut fc4_traffic_pf, end: fc4_cell_rect, flags: u32) {
    unsafe {
        assert!(fc4_traffic_pf_set(pf, 5, flags, 0));
        let start = fc4_cell_rect {
            x1: 2,
            z1: 2,
            x2: 2,
            z2: 2,
        };
        fc4_traffic_pf_set_rect(pf, false, &start);
        fc4_traffic_pf_set_rect(pf, true, &end);
        assert!(fc4_traffic_pf_set(pf, 9, 3, 0));
        assert!(fc4_traffic_pf_set(pf, 15, 0, 0));
        assert!(fc4_traffic_pf_set(pf, 13, 0x2, 1));
        fc4_traffic_pf_set_starting_cost_array(pf, [0f32; 9].as_ptr());
        fc4_traffic_pf_set_route_info(pf, true);
        assert!(fc4_traffic_pf_set(pf, 18, 10, 0));
    }
}

#[test]
fn slot6_pathfinder_finds_the_street_route() {
    let g = Game::new(8, 8);
    let p = new_city(&g);
    unsafe {
        assert!(fc4_traffic_init(p, 0));
        street(p);
        let pf = fc4_traffic_pf_create(p);
        assert!(!pf.is_null());
        let mut st = fc4_traffic_pf_state::default();
        assert!(fc4_traffic_pf_get_state(pf, &mut st));
        assert_eq!((st.initialised, st.city_x, st.city_z), (1, 8, 8));
        // Init a second time: false (already initialised).
        assert!(!fc4_traffic_pf_init(p, pf));
        configure(
            pf,
            fc4_cell_rect {
                x1: 4,
                z1: 4,
                x2: 4,
                z2: 4,
            },
            1,
        );
        assert_eq!(fc4_traffic_pf_find_path(p, pf), 1);
        assert!(fc4_traffic_pf_trip_length(pf) > 0.0);
        let (mut x, mut z) = (0, 0);
        fc4_traffic_pf_end_cell(pf, &mut x, &mut z);
        assert_eq!((x, z), (4, 4));
        assert_eq!(fc4_traffic_pf_traffic_arrived(pf), 10);
        assert_eq!(fc4_traffic_pf_last_travel_type(pf), 1);
        let scaled = fc4_traffic_pf_trip_length_for_display(p, pf);
        assert_eq!(scaled, 25.0 * fc4_traffic_pf_trip_length(pf));
        let mut nodes: Vec<Vec<u8>> = Vec::new();
        let n = fc4_traffic_pf_route(pf, Some(push_bytes), &mut nodes as *mut Vec<Vec<u8>> as *mut c_void);
        assert_eq!(n, 3);
        assert_eq!(nodes, vec![vec![2, 3, 1], vec![3, 3, 1], vec![4, 3, 1]]);
        let mut routes: Vec<Vec<u8>> = Vec::new();
        let mask =
            fc4_traffic_pf_compressed_route(p, pf, Some(push_bytes), &mut routes as *mut Vec<Vec<u8>> as *mut c_void);
        assert_eq!(mask, 1 << 1);
        // Header (type 1, x 2, z 3), then one Car run of two +x steps.
        assert_eq!(routes, vec![vec![1, 2, 3, 0x41, 2, 0b1010]]);
        assert!(fc4_traffic_pf_was_trip_reversible(p, pf));
        fc4_traffic_pf_release_path_nodes(pf);
        assert!(fc4_traffic_pf_shutdown(pf));
        fc4_traffic_pf_destroy(pf);
        fc4_traffic_destroy(p);
    }
}

#[test]
fn slot6_pathfinder_flag2_fallback() {
    let g = Game::new(8, 8);
    let p = new_city(&g);
    unsafe {
        assert!(fc4_traffic_init(p, 0));
        street(p);
        let pf = fc4_traffic_pf_create(p);
        let far = fc4_cell_rect {
            x1: 4,
            z1: 7,
            x2: 4,
            z2: 7,
        };
        // Without flag 2: not found.
        configure(pf, far, 1);
        assert_eq!(fc4_traffic_pf_find_path(p, pf), 0);
        // With flag 2, filter 3 and a route: the nearest node, nothing
        // arrived (§4.12).
        configure(pf, far, 1 | 2);
        fc4_traffic_pf_set_max_trip_length(pf, 1000.0);
        assert_eq!(fc4_traffic_pf_find_path(p, pf), 1);
        assert_eq!(fc4_traffic_pf_traffic_arrived(pf), 0);
        let mut nodes: Vec<Vec<u8>> = Vec::new();
        fc4_traffic_pf_route(pf, Some(push_bytes), &mut nodes as *mut Vec<Vec<u8>> as *mut c_void);
        assert_eq!(
            nodes.last(),
            Some(&vec![4, 3, 1]),
            "the street cell nearest the end rect"
        );
        fc4_traffic_pf_destroy(pf);
        fc4_traffic_destroy(p);
    }
}
