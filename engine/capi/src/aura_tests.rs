// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The aura C ABI (fc4.h "aura") against a fake game: the aura's int8 /
//! int16 grids and the other simulators' input grids through `fc4_grid_api`,
//! a lot iterator, occupants and their properties, persistence, getters from
//! a second thread during a main-thread call, and re-entrant calls.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::{c_char, c_void, CStr};
use std::sync::mpsc;
use std::time::Duration;

use fc4_core::aura::{grid_id, ids, msg, shipped_tuning, AGENT_LEVELS, AGENT_NAME};
use fc4_core::variant::mem;

use crate::aura::*;
use crate::hostgrid::fc4_grid_api;
use crate::landvalue::fc4_grid_geometry;
use crate::props::{fc4_props, fc4_props_set_typed};
use crate::test_streams::{istream, ostream, In, Out};

pub(crate) const CELLS: i32 = 32;
const UINT8_GRID: u32 = 0x49B9_E602;
const UINT16_GRID: u32 = 0x49B9_E604;
pub(crate) const LOT_BUILDING: u64 = 0x55;
pub(crate) const PARK: u64 = 0x66;
const ITERATOR: u64 = 0x900;
pub(crate) const CRIME: u64 = 0x800;
pub(crate) const POP: [u64; 3] = [0x810, 0x820, 0x830];

pub(crate) struct Grid {
    pub(crate) data: Vec<f64>,
    pub(crate) clsid: u32,
    pub(crate) tract_size: i32,
    pub(crate) refs: i32,
    pub(crate) shut: bool,
    pub(crate) instance: u32,
}

impl Grid {
    fn count(&self) -> i32 {
        CELLS / self.tract_size
    }
}

/// (element type, array flag, bytes, count).
type Prop = (u16, bool, Vec<u8>, usize);

#[derive(Default)]
pub(crate) struct Game {
    pub(crate) grids: RefCell<BTreeMap<u64, Grid>>,
    next_grid: Cell<u64>,
    pub(crate) agents: RefCell<Vec<(u32, String, u32)>>,
    pub(crate) subs: RefCell<Vec<u32>>,
    pub(crate) unsubs: RefCell<Vec<u32>>,
    pub(crate) posts: RefCell<Vec<u32>>,
    pub(crate) history: RefCell<Vec<(u32, i32)>>,
    pub(crate) strikes: RefCell<Vec<u32>>,
    pub(crate) disasters: RefCell<Vec<(u32, i32, i32)>>,
    pub(crate) base_aura: Cell<[i32; 3]>,
    pub(crate) props: RefCell<BTreeMap<(u64, u32), Prop>>,
    /// Lot iterators handed out and not yet released.
    pub(crate) iterators: Cell<i32>,
    pub(crate) iterated: Cell<u32>,
    pub(crate) no_lots: Cell<bool>,
    pub(crate) funding: Cell<f32>,
    pub(crate) inputs_asked: RefCell<Vec<u32>>,
    /// Second-thread test: the handle to read from another thread.
    pub(crate) probe: Cell<usize>,
    pub(crate) probes: Cell<u32>,
    /// Re-entrancy test: the handle to call back into.
    pub(crate) reenter: Cell<usize>,
}

pub(crate) fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

impl Game {
    pub(crate) fn new() -> Box<Game> {
        let g = Box::<Game>::default();
        g.funding.set(0.5);
        // The other simulators' grids: crime 0 everywhere, R$ population 100 in tract (0, 0).
        g.input(CRIME, UINT8_GRID, 2, 0.0);
        for (i, t) in POP.iter().enumerate() {
            g.input(*t, UINT16_GRID, 4, 0.0);
            if i == 0 {
                g.grids.borrow_mut().get_mut(t).unwrap().data[0] = 100.0;
            }
        }
        // The park: effect {20, 8} (magnitude, radius in cells).
        g.set(PARK, ids::PROP_PARK_EFFECT, mem::SINT32, true, &[20i32, 8]);
        g
    }

    fn input(&self, t: u64, clsid: u32, tract_size: i32, v: f64) {
        let n = (CELLS / tract_size) as usize;
        self.grids.borrow_mut().insert(
            t,
            Grid {
                data: vec![v; n * n],
                clsid,
                tract_size,
                refs: 1,
                shut: false,
                instance: 0,
            },
        );
    }

    pub(crate) fn set<T: Copy>(&self, occ: u64, id: u32, t: u16, array: bool, v: &[T]) {
        let bytes = unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }.to_vec();
        self.props.borrow_mut().insert((occ, id), (t, array, bytes, v.len()));
    }

    pub(crate) fn user(&self) -> *mut c_void {
        self as *const Game as *mut c_void
    }

    pub(crate) fn value(&self, t: u64, x: i32, z: i32) -> f64 {
        let grids = self.grids.borrow();
        let g = &grids[&t];
        g.data[(x * g.count() + z) as usize]
    }

    /// The token of the aura's grid with this instance (the first one).
    pub(crate) fn grid_by_instance(&self, instance: u32, tract_size: i32) -> u64 {
        *self
            .grids
            .borrow()
            .iter()
            .find(|(_, g)| g.instance == instance && g.tract_size == tract_size)
            .expect("grid")
            .0
    }

    /// Reads the handle from another thread while this (main) thread is
    /// inside a host call; a getter blocked on the state would time out.
    fn probe_now(&self) {
        let p = self.probe.get();
        if p == 0 {
            return;
        }
        self.probes.set(self.probes.get() + 1);
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let p = p as *const fc4_aura;
            let mut r = fc4_aura_ratings::default();
            let ok = unsafe { fc4_aura_get_ratings(p, &mut r) };
            let g = unsafe { fc4_aura_grid(p, FC4_AURA_GRID_AURA) };
            let _ = tx.send((ok, r.aura_grid, g));
        });
        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok((ok, a, g)) => assert!(ok && a != 0 && a == g, "the getters answer from the snapshot"),
            Err(_) => panic!("a getter on another thread blocked during a main-thread host call"),
        }
    }
}

// ---- grids --------------------------------------------------------------------------------------------

unsafe extern "C" fn g_create(u: *mut c_void, clsid: u32, instance: u32, ts: i32) -> u64 {
    assert!(clsid == grid_id::CLSID_INT8 || clsid == grid_id::CLSID_INT16);
    assert!(ts == 1 || ts == 4);
    let g = game(u);
    let t = g.next_grid.get() + 0x7000;
    g.next_grid.set(g.next_grid.get() + 0x10);
    let n = (CELLS / ts) as usize;
    g.grids.borrow_mut().insert(
        t,
        Grid {
            data: vec![0.0; n * n],
            clsid,
            tract_size: ts,
            refs: 1,
            shut: false,
            instance,
        },
    );
    t
}
pub(crate) unsafe extern "C" fn g_class(u: *mut c_void, t: u64) -> u32 {
    game(u).grids.borrow().get(&t).map_or(0, |g| g.clsid)
}
pub(crate) unsafe extern "C" fn g_geometry(u: *mut c_void, t: u64, out: *mut fc4_grid_geometry) {
    let ts = game(u).grids.borrow().get(&t).map_or(1, |g| g.tract_size);
    let w = 16.0 * ts as f32;
    *out = fc4_grid_geometry {
        tract_size: ts,
        shift: ts.trailing_zeros() as i32,
        count_x: CELLS / ts,
        count_z: CELLS / ts,
        width_x: w,
        width_z: w,
        inv_width_x: 1.0 / w,
        inv_width_z: 1.0 / w,
    };
}
pub(crate) unsafe extern "C" fn g_get(u: *mut c_void, t: u64, kind: u32, x: i32, z: i32) -> f64 {
    let grids = game(u).grids.borrow();
    let g = &grids[&t];
    assert_eq!(kind, g.clsid, "read with the grid's own element type");
    let n = g.count();
    if x < 0 || z < 0 || x >= n || z >= n {
        return 0.0;
    }
    g.data[(x * n + z) as usize]
}
unsafe extern "C" fn g_set(u: *mut c_void, t: u64, kind: u32, x: i32, z: i32, v: f64) {
    let mut grids = game(u).grids.borrow_mut();
    let g = grids.get_mut(&t).unwrap();
    assert_eq!(kind, g.clsid);
    assert!(g.instance != 0, "an input grid is never written");
    let n = g.count();
    if x >= 0 && z >= 0 && x < n && z < n {
        g.data[(x * n + z) as usize] = v;
    }
}
unsafe extern "C" fn g_fill(u: *mut c_void, t: u64, _kind: u32, v: f64) {
    game(u).grids.borrow_mut().get_mut(&t).unwrap().data.fill(v);
}
unsafe extern "C" fn g_shutdown(u: *mut c_void, t: u64) {
    game(u).grids.borrow_mut().get_mut(&t).unwrap().shut = true;
}
unsafe extern "C" fn g_release(u: *mut c_void, t: u64) {
    game(u).grids.borrow_mut().get_mut(&t).unwrap().refs -= 1;
}

// ---- city, lots, occupants --------------------------------------------------------------------------------

unsafe extern "C" fn cell_width(_: *mut c_void, x: *mut f32, z: *mut f32) {
    *x = 16.0;
    *z = 16.0;
}
unsafe extern "C" fn position_to_cell(_: *mut c_void, x: f32, z: f32, cx: *mut i32, cz: *mut i32) {
    *cx = (x / 16.0).floor() as i32;
    *cz = (z / 16.0).floor() as i32;
}
unsafe extern "C" fn sim_date_number(_: *mut c_void) -> u32 {
    100
}
pub(crate) unsafe extern "C" fn tuning(_: *mut c_void) -> *mut fc4_props {
    Box::into_raw(Box::new(shipped_tuning())) as *mut fc4_props
}
unsafe extern "C" fn publish_base_aura(u: *mut c_void, v: *const i32) {
    game(u).base_aura.set([*v, *v.add(1), *v.add(2)]);
}
pub(crate) unsafe extern "C" fn lot_count(u: *mut c_void) -> u32 {
    u32::from(!game(u).no_lots.get())
}
pub(crate) unsafe extern "C" fn lot_iterator(u: *mut c_void) -> u64 {
    let g = game(u);
    g.iterators.set(g.iterators.get() + 1);
    g.iterated.set(0);
    ITERATOR
}
/// One lot: cells (0..3, 0..3), R$, wealth 1, ten R$ residents and two
/// commercial workers.
pub(crate) unsafe extern "C" fn lot_next(
    u: *mut c_void,
    it: u64,
    out: *mut fc4_aura_lot,
    sink: fc4_aura_population_sink,
    ctx: *mut c_void,
) -> bool {
    assert_eq!(it, ITERATOR);
    let g = game(u);
    if g.no_lots.get() || g.iterated.get() > 0 {
        return false;
    }
    g.iterated.set(1);
    *out = fc4_aura_lot {
        x1: 0,
        z1: 0,
        x2: 3,
        z2: 3,
        zone_type: 1,
        occupant_wealth: 1,
        building: LOT_BUILDING,
    };
    if let Some(f) = sink {
        f(ctx, 0x1010, 10);
    }
    true
}
pub(crate) unsafe extern "C" fn lot_iterator_release(u: *mut c_void, it: u64) {
    assert_eq!(it, ITERATOR);
    let g = game(u);
    g.iterators.set(g.iterators.get() - 1);
}
pub(crate) unsafe extern "C" fn lot_at(u: *mut c_void, x: i32, z: i32, out: *mut fc4_aura_lot) -> bool {
    if game(u).no_lots.get() || !(0..4).contains(&x) || !(0..4).contains(&z) {
        return false;
    }
    *out = fc4_aura_lot {
        x1: 0,
        z1: 0,
        x2: 3,
        z2: 3,
        zone_type: 1,
        occupant_wealth: 1,
        building: 0,
    };
    true
}
pub(crate) unsafe extern "C" fn land_value_type(_: *mut c_void, _x: i32, _z: i32) -> i32 {
    3
}
pub(crate) unsafe extern "C" fn occupant_has_property(u: *mut c_void, occ: u64, id: u32) -> bool {
    game(u).props.borrow().contains_key(&(occ, id))
}
pub(crate) unsafe extern "C" fn occupant_property(u: *mut c_void, occ: u64, id: u32, p: *mut fc4_props) -> bool {
    match game(u).props.borrow().get(&(occ, id)) {
        Some((t, a, b, n)) => fc4_props_set_typed(p, id, *t, *a, b.as_ptr() as *const c_void, *n),
        None => false,
    }
}
pub(crate) unsafe extern "C" fn occupant_type(_: *mut c_void, _occ: u64) -> u32 {
    0x278128A0
}
pub(crate) unsafe extern "C" fn occupant_position(_: *mut c_void, _occ: u64, out: *mut f32) {
    *out = 8.0;
    *out.add(1) = 0.0;
    *out.add(2) = 8.0;
}
pub(crate) unsafe extern "C" fn department_funding(u: *mut c_void, dept: u32, out: *mut f32) -> bool {
    if dept != ids::DEPARTMENT_PARKS {
        return false;
    }
    *out = game(u).funding.get();
    true
}
pub(crate) unsafe extern "C" fn tax_rate(_: *mut c_void, _index: i32) -> f32 {
    9.0
}
pub(crate) unsafe extern "C" fn cumulative_offset(_: *mut c_void, _prop: u32, _index: u32) -> i32 {
    0
}
pub(crate) unsafe extern "C" fn input_grid(u: *mut c_void, which: u32) -> u64 {
    game(u).inputs_asked.borrow_mut().push(which);
    match which {
        FC4_AURA_INPUT_CRIME => CRIME,
        FC4_AURA_INPUT_POP1 => POP[0],
        FC4_AURA_INPUT_POP2 => POP[1],
        FC4_AURA_INPUT_POP3 => POP[2],
        _ => 0,
    }
}

// ---- effects and plumbing ---------------------------------------------------------------------------

unsafe extern "C" fn record_history(u: *mut c_void, series: u32, value: i32) {
    let g = game(u);
    g.history.borrow_mut().push((series, value));
    g.probe_now();
    let p = g.reenter.replace(0);
    if p != 0 {
        // A transient effect from inside the month's call: queued.
        fc4_aura_add_transient_effect(p as *mut fc4_aura, 8.0, 8.0, 64.0, 30.0);
    }
}
unsafe extern "C" fn attempt_disaster_start(u: *mut c_void, manager: u32, x: i32, z: i32) {
    game(u).disasters.borrow_mut().push((manager, x, z));
}
unsafe extern "C" fn end_strike(u: *mut c_void, k: u32) {
    game(u).strikes.borrow_mut().push(k);
}
unsafe extern "C" fn post(u: *mut c_void, t: u32) {
    let g = game(u);
    g.posts.borrow_mut().push(t);
    g.probe_now();
}
unsafe extern "C" fn subscribe(u: *mut c_void, ids: *const u32, n: u32) {
    game(u)
        .subs
        .borrow_mut()
        .extend_from_slice(std::slice::from_raw_parts(ids, n as usize));
}
unsafe extern "C" fn unsubscribe(u: *mut c_void, ids: *const u32, n: u32) {
    game(u)
        .unsubs
        .borrow_mut()
        .extend_from_slice(std::slice::from_raw_parts(ids, n as usize));
}
unsafe extern "C" fn add_agent(u: *mut c_void, level: u32, name: *const c_char, flags: u32) {
    let name = CStr::from_ptr(name).to_string_lossy().into_owned();
    game(u).agents.borrow_mut().push((level, name, flags));
}
unsafe extern "C" fn remove_agent(u: *mut c_void) {
    game(u).agents.borrow_mut().clear();
}

pub(crate) fn grid_api(g: &Game) -> fc4_grid_api {
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

pub(crate) fn host(g: &Game) -> fc4_aura_host {
    fc4_aura_host {
        struct_size: std::mem::size_of::<fc4_aura_host>() as u32,
        user: g.user(),
        grids: grid_api(g),
        cell_width: Some(cell_width),
        position_to_cell: Some(position_to_cell),
        sim_date_number: Some(sim_date_number),
        tuning: Some(tuning),
        publish_base_aura: Some(publish_base_aura),
        lot_count: Some(lot_count),
        lot_iterator: Some(lot_iterator),
        lot_next: Some(lot_next),
        lot_iterator_release: Some(lot_iterator_release),
        lot_at: Some(lot_at),
        land_value_type: Some(land_value_type),
        occupant_has_property: Some(occupant_has_property),
        occupant_property: Some(occupant_property),
        occupant_type: Some(occupant_type),
        occupant_position: Some(occupant_position),
        budget_item_department: None,
        department_funding: Some(department_funding),
        tax_rate: Some(tax_rate),
        cumulative_offset: Some(cumulative_offset),
        input_grid: Some(input_grid),
        record_history: Some(record_history),
        attempt_disaster_start: Some(attempt_disaster_start),
        end_strike: Some(end_strike),
        post: Some(post),
        limit_timer_start: None,
        limit_timer_expired: None,
        subscribe: Some(subscribe),
        unsubscribe: Some(unsubscribe),
        add_agent: Some(add_agent),
        remove_agent: Some(remove_agent),
        registry_add: None,
        registry_remove: None,
    }
}

fn message(p: *mut fc4_aura, msg_type: u32, data3: u32, void1: u64) -> bool {
    unsafe { fc4_aura_do_message(p, msg_type, 0, 0, data3, void1) }
}

fn state_of(p: *const fc4_aura) -> fc4_aura_state {
    let mut s = fc4_aura_state::default();
    assert!(unsafe { fc4_aura_get_state(p, &mut s) });
    s
}

fn ratings_of(p: *const fc4_aura) -> fc4_aura_ratings {
    let mut r = fc4_aura_ratings::default();
    assert!(unsafe { fc4_aura_get_ratings(p, &mut r) });
    r
}

fn started(g: &Game) -> *mut fc4_aura {
    let h = host(g);
    let p = unsafe { fc4_aura_create(&h, 0xA0, 1, 2) };
    assert!(!p.is_null());
    assert!(unsafe { fc4_aura_init(p) });
    p
}

/// Idle calls until the cycle ends (at most 100).
fn run_cycle(p: *mut fc4_aura) -> u32 {
    let mut calls = 0;
    while state_of(p).state != 0 && calls < 100 {
        message(p, msg::SIM_IDLE, 5, 0);
        calls += 1;
    }
    calls
}

fn stop(p: *mut fc4_aura) {
    assert!(unsafe { fc4_aura_shutdown(p) });
    unsafe { fc4_aura_destroy(p) };
}

#[test]
fn create_rejects_short_tables_and_null_handles_answer_fallbacks() {
    let g = Game::new();
    let mut h = host(&g);
    unsafe {
        assert!(fc4_aura_create(std::ptr::null(), 1, 1, 1).is_null());
        h.struct_size -= 4;
        assert!(fc4_aura_create(&h, 1, 1, 1).is_null());
        h.struct_size += 4;
        h.grids.struct_size = 8;
        assert!(fc4_aura_create(&h, 1, 1, 1).is_null());

        let p = std::ptr::null_mut();
        assert!(!fc4_aura_init(p));
        assert!(!fc4_aura_shutdown(p));
        assert!(fc4_aura_do_message(p, msg::SIM_NEW_DAY, 0, 0, 0, 0));
        fc4_aura_add_transient_effect(p, 0.0, 0.0, 1.0, 1.0);
        fc4_aura_add_transient_effect_type(p, 0.0, 0.0, 0);
        fc4_aura_bust_strike(p, 0);
        let mut r = fc4_aura_ratings::default();
        assert!(!fc4_aura_get_ratings(p, &mut r));
        assert_eq!(fc4_aura_grid(p, 0), 0);
        let mut s = fc4_aura_state::default();
        assert!(!fc4_aura_get_state(p, &mut s));
        fc4_aura_destroy(p);
    }
}

#[test]
fn init_registers_and_creates_the_grids() {
    let g = Game::new();
    let p = started(&g);
    let agents: Vec<u32> = g.agents.borrow().iter().map(|a| a.0).collect();
    assert_eq!(agents, AGENT_LEVELS.to_vec());
    assert!(g.agents.borrow().iter().all(|a| a.1 == AGENT_NAME && a.2 == 2));
    assert_eq!(g.subs.borrow().len(), 5);
    // §2: aura, base, transient, work, building effect, park map, raw park,
    // landmark map, landmark work.
    let created = g.grids.borrow().values().filter(|gr| gr.instance != 0).count();
    assert_eq!(created, 9);
    let work = g.grid_by_instance(grid_id::WORK, 1);
    assert_eq!(g.grids.borrow()[&work].clsid, grid_id::CLSID_INT8);
    let base = g.grid_by_instance(grid_id::BASE, 4);
    assert_eq!(g.value(base, 0, 0), -128.0);
    let st = state_of(p);
    assert_eq!((st.initialized, st.state), (1, 1));
    assert_eq!(st.long_decay, 0.9);
    // The getters' snapshot names the four grids.
    let r = ratings_of(p);
    assert_eq!(r.aura_grid, g.grid_by_instance(grid_id::AURA, 4));
    assert_eq!(r.park_map, g.grid_by_instance(grid_id::PARK_MAP, 4));
    assert_eq!(unsafe { fc4_aura_grid(p, FC4_AURA_GRID_TRANSIENT) }, r.transient_grid);
    assert_eq!(unsafe { fc4_aura_grid(p, FC4_AURA_GRID_LANDMARK) }, r.landmark_map);
    assert_eq!(unsafe { fc4_aura_grid(p, 9) }, 0);
    stop(p);
    // Shutdown: every grid shut down and released, the agent and the
    // subscriptions removed.
    assert!(g
        .grids
        .borrow()
        .values()
        .filter(|gr| gr.instance != 0)
        .all(|gr| gr.shut && gr.refs == 0));
    assert!(g.agents.borrow().is_empty());
    assert_eq!(g.unsubs.borrow().len(), 5);
}

/// A cycle on the fake city: the lot's base aura (shipped tuning: base 0,
/// value curve at land value 3 − wealth 1 = −5), crime 0 (+10), the base,
/// aura and mayor rating, the history record and the end-of-cycle post.
#[test]
fn a_cycle_rates_the_lot() {
    let g = Game::new();
    let p = started(&g);
    let calls = run_cycle(p);
    assert!(calls >= 17, "one step per idle call: {calls}");
    let base = g.grid_by_instance(grid_id::BASE, 4);
    let aura = g.grid_by_instance(grid_id::AURA, 4);
    assert_eq!(g.value(base, 0, 0), 5.0);
    assert_eq!(g.value(base, 1, 0), -128.0);
    assert_eq!(g.value(aura, 0, 0), 5.0);
    assert_eq!(g.value(aura, 1, 1), 0.0);
    assert_eq!(*g.history.borrow(), vec![(ids::HISTORY_MAYOR_RATING, 5)]);
    assert_eq!(*g.posts.borrow(), vec![msg::AURA_CYCLE_COMPLETED]);
    assert_eq!(g.iterators.get(), 0, "the lot iterator is released");
    let r = ratings_of(p);
    assert_eq!((r.mayor_rating, r.max_rating, r.min_rating), (5, 5, 5));
    assert_eq!((r.max_x, r.max_z, r.min_x, r.min_z), (2, 2, 2, 2));
    let st = state_of(p);
    assert_eq!((st.aura_tracts, st.aura_sum, st.aura_min, st.aura_max), (64, 5, 0, 5));
    assert_eq!(st.base_tracts, 1);
    assert_eq!(st.effective_base_aura, [0, 0, 0]);
    // Only the inputs the cycle reads: pollution 0..3, school, hospital,
    // crime, trip, congestion, then the population grids.
    let asked = g.inputs_asked.borrow().clone();
    assert_eq!(asked, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
    // The next month starts a new cycle; SimNewDay with the state after its step.
    assert!(message(p, msg::SIM_NEW_MONTH, 0, 0));
    assert_eq!(state_of(p).state, 1);
    assert!(message(p, msg::SIM_NEW_DAY, 0, 0));
    stop(p);
}

#[test]
fn a_city_without_lots_ends_the_cycle_silently() {
    let g = Game::new();
    g.no_lots.set(true);
    let p = started(&g);
    run_cycle(p);
    assert!(g.history.borrow().is_empty());
    assert!(g.posts.borrow().is_empty());
    // State 4 took an iterator before it asked the count; it is kept until
    // the next prepare or Shutdown.
    assert_eq!(g.iterators.get(), 1);
    stop(p);
    assert_eq!(g.iterators.get(), 0);
}

/// §7 and §6.2: a park's cone in the raw park grid, scaled by the Parks
/// funding into the park map at the next cycle.
#[test]
fn occupants_stamp_parks_and_the_cycle_scales_them() {
    let g = Game::new();
    let p = started(&g);
    assert!(message(p, msg::OCCUPANT_INSERTED, 0, PARK));
    let raw = g.grid_by_instance(grid_id::RAW_PARK, 4);
    let centre = g.value(raw, 0, 0);
    assert!(centre > 0.0, "the cone's centre");
    run_cycle(p);
    let park = g.grid_by_instance(grid_id::PARK_MAP, 4);
    assert_eq!(g.value(park, 0, 0), (centre * 0.5).trunc());
    // Removing it takes the cone back out.
    assert!(message(p, msg::OCCUPANT_REMOVED, 0, PARK));
    assert_eq!(g.value(raw, 0, 0), 0.0);
    // A preview occupant is ignored.
    g.set(PARK, ids::PROP_PREVIEW_OCCUPANT, mem::UINT32, false, &[1u32]);
    assert!(message(p, msg::OCCUPANT_INSERTED, 0, PARK));
    assert_eq!(g.value(raw, 0, 0), 0.0);
    stop(p);
}

#[test]
fn strikes_and_transient_effects() {
    let g = Game::new();
    let p = started(&g);
    unsafe {
        fc4_aura_bust_strike(p, 0);
        fc4_aura_bust_strike(p, 3);
        fc4_aura_bust_strike(p, 9);
    }
    assert_eq!(*g.strikes.borrow(), vec![0, 1, 2, 3, 2]);
    let transient = g.grid_by_instance(grid_id::TRANSIENT, 4);
    // Type 2 (civic plop, shipped {25, 128 m}) at the city's first tract.
    unsafe { fc4_aura_add_transient_effect_type(p, 8.0, 8.0, 2) };
    assert_eq!(g.value(transient, 0, 0), 25.0);
    // Type 3 is {0, 0}: nothing; type 4 is out of range: nothing.
    unsafe {
        fc4_aura_add_transient_effect_type(p, 200.0, 200.0, 3);
        fc4_aura_add_transient_effect_type(p, 200.0, 200.0, 4);
    }
    let sum: f64 = g.grids.borrow()[&transient].data.iter().sum();
    unsafe { fc4_aura_add_transient_effect(p, 400.0, 400.0, 64.0, -10.0) };
    let after: f64 = g.grids.borrow()[&transient].data.iter().sum();
    assert!(after < sum);
    stop(p);
}

/// Requirement: the game reads the getters from a second thread during
/// OnTick. The history record and the post of a cycle read the handle from
/// another thread and wait.
#[test]
fn getters_answer_from_another_thread_during_a_main_thread_call() {
    let g = Game::new();
    let p = started(&g);
    g.probe.set(p as usize);
    run_cycle(p);
    g.probe.set(0);
    assert_eq!(g.probes.get(), 2);
    stop(p);
}

/// A transient effect delivered from inside a call (a host callback) is
/// queued and applied when the call returns.
#[test]
fn calls_during_a_call_are_queued() {
    let g = Game::new();
    let p = started(&g);
    g.reenter.set(p as usize);
    run_cycle(p);
    assert_eq!(g.reenter.get(), 0, "the cycle recorded history");
    let st = state_of(p);
    assert_eq!(st.reentrant_calls, 1);
    let transient = g.grid_by_instance(grid_id::TRANSIENT, 4);
    // Applied after the history call (state 15), then decayed at state 17:
    // trunc(30 × 0.75).
    assert_eq!(g.value(transient, 0, 0), 22.0);
    // The ratings were published after the queued call.
    assert_eq!(ratings_of(p).mayor_rating, 5);
    stop(p);
}

#[test]
fn sint32s_reads_integer_arrays_of_two_or_more() {
    // §7 "How the values are read", [choice] AU4: a Sint32 or Uint32 array
    // (bitwise) of at least two values; anything else is absent (no cone).
    use fc4_core::variant::{Elem, Variant};
    let a = |elem: u16, items: Vec<Elem>| Variant::array(elem, items);
    let pair = a(mem::SINT32, vec![Elem::I32(50), Elem::I32(22)]);
    assert_eq!(sint32s(&pair), Some(vec![50, 22]));
    let bits = vec![Elem::U32(0xFFFF_FF9C), Elem::U32(32), Elem::U32(7)];
    assert_eq!(sint32s(&a(mem::UINT32, bits)), Some(vec![-100, 32, 7]));
    assert_eq!(sint32s(&a(mem::SINT32, vec![Elem::I32(50)])), None);
    assert_eq!(sint32s(&a(mem::SINT32, vec![])), None);
    assert_eq!(sint32s(&Variant::Scalar(Elem::I32(50))), None);
    assert_eq!(sint32s(&a(mem::FLOAT32, vec![Elem::F32(50.0), Elem::F32(22.0)])), None);
    assert_eq!(sint32s(&a(mem::SINT16, vec![Elem::I16(50), Elem::I16(22)])), None);
}

#[test]
fn record_round_trip_through_host_streams() {
    let g = Game::new();
    let p = started(&g);
    run_cycle(p);
    let before = state_of(p);
    let mut out = Out::default();
    let os = ostream(&mut out);
    assert!(unsafe { fc4_aura_write(p, &os) });
    let aura = g.grid_by_instance(grid_id::AURA, 4);
    assert!(out.refs.contains(&(aura, grid_id::CLSID_INT8)));
    assert_eq!(out.refs.len(), 7, "seven grids, no department grid");

    // Read runs before Init: the reader takes the references the stream hands over.
    let h = host(&g);
    let q = unsafe { fc4_aura_create(&h, 0xA1, 1, 2) };
    let gp: *const Game = &*g;
    let take = move |r: &mut crate::stream::fc4_ref| unsafe {
        if let Some(gr) = (*gp).grids.borrow_mut().get_mut(&r.token) {
            gr.refs += 1;
        }
    };
    let mut inp = In::new(&out.bytes);
    inp.resolve = Some(&take);
    let is = istream(&mut inp);
    assert!(unsafe { fc4_aura_read(q, &is) });
    let after = state_of(q);
    assert_eq!(after.mayor_rating, before.mayor_rating);
    assert_eq!(
        (after.max_rating, after.max_x, after.max_z),
        (before.max_rating, before.max_x, before.max_z)
    );
    assert_eq!(after.initialized, 0);
    assert_eq!(ratings_of(q).aura_grid, aura, "the loaded grids are published");
    unsafe { fc4_aura_destroy(q) };
    assert_eq!(
        g.grids.borrow()[&aura].refs,
        1,
        "the reader's references are given back"
    );

    // A version above 9 fails.
    let bad = 10u16.to_le_bytes();
    let q = unsafe { fc4_aura_create(&h, 0xA2, 1, 2) };
    let mut inp = In::new(&bad);
    let is = istream(&mut inp);
    assert!(!unsafe { fc4_aura_read(q, &is) });
    unsafe { fc4_aura_destroy(q) };
    stop(p);
}

// ---- the shadow ---------------------------------------------------------------------------------------

use crate::aura_shadow::*;
use crate::stream::{fc4_ostream, fc4_ref};

/// What the plugin's capture stream produces: the original's `Write`, with
/// references as `u32 index + 1, u32 class` and each one queried for the
/// grid interface.
struct Capture {
    game: *const Game,
    bytes: Vec<u8>,
    refs: Vec<fc4_aura_capture_ref>,
}

impl Capture {
    fn view(&self) -> fc4_aura_capture {
        fc4_aura_capture {
            struct_size: std::mem::size_of::<fc4_aura_capture>() as u32,
            ok: 1,
            bytes: self.bytes.as_ptr(),
            len: self.bytes.len(),
            refs: self.refs.as_ptr(),
            ref_count: self.refs.len(),
        }
    }
}

fn cap(u: *mut c_void) -> &'static mut Capture {
    unsafe { &mut *(u as *mut Capture) }
}
macro_rules! cput {
    ($name:ident, $t:ty) => {
        unsafe extern "C" fn $name(u: *mut c_void, v: $t) -> bool {
            cap(u).bytes.extend_from_slice(&v.to_le_bytes());
            true
        }
    };
}
cput!(c_u8, u8);
cput!(c_u16, u16);
cput!(c_u32, u32);
cput!(c_u64, u64);
cput!(c_f32, f32);
cput!(c_f64, f64);
unsafe extern "C" fn c_bytes(u: *mut c_void, p: *const u8, n: usize) -> bool {
    if n > 0 {
        cap(u).bytes.extend_from_slice(std::slice::from_raw_parts(p, n));
    }
    true
}
unsafe extern "C" fn c_str(u: *mut c_void, p: *const u8, n: usize) -> bool {
    c_u32(u, n as u32);
    c_bytes(u, p, n)
}
unsafe extern "C" fn c_ref(u: *mut c_void, r: *const fc4_ref) -> bool {
    let c = cap(u);
    let r = &*r;
    if r.token == 0 {
        return c_u32(u, 0);
    }
    let is_grid = (*c.game).grids.borrow().contains_key(&r.token);
    c.refs.push(fc4_aura_capture_ref {
        grid: if is_grid { r.token } else { 0 },
        class_id: r.class_id,
        reserved: 0,
    });
    let id = c.refs.len() as u32;
    c_u32(u, id);
    c_u32(u, r.class_id)
}
unsafe extern "C" fn c_no_error(_: *mut c_void) -> i32 {
    0
}

fn capture(p: *const fc4_aura, g: &Game) -> Box<Capture> {
    let mut c = Box::new(Capture {
        game: g,
        bytes: Vec::new(),
        refs: Vec::new(),
    });
    let os = fc4_ostream {
        struct_size: std::mem::size_of::<fc4_ostream>() as u32,
        user: &mut *c as *mut Capture as *mut c_void,
        put_u8: Some(c_u8),
        put_u16: Some(c_u16),
        put_u32: Some(c_u32),
        put_u64: Some(c_u64),
        put_f32: Some(c_f32),
        put_f64: Some(c_f64),
        put_bytes: Some(c_bytes),
        put_str: Some(c_str),
        put_ref: Some(c_ref),
        error: Some(c_no_error),
    };
    assert!(unsafe { fc4_aura_write(p, &os) });
    c
}

unsafe extern "C" fn lines_sink(ctx: *mut c_void, data: *const u8, len: usize) {
    let s = String::from_utf8(std::slice::from_raw_parts(data, len).to_vec()).unwrap();
    (*(ctx as *mut Vec<String>)).push(s);
}

/// The plugin's read-only table: reads only, grids by geometry and get.
fn read_only_host(g: &Game) -> fc4_aura_host {
    let mut h = host(g);
    h.grids.create = None;
    h.grids.set = None;
    h.grids.fill = None;
    h.grids.shutdown = None;
    h.grids.release = None;
    h.publish_base_aura = None;
    h.record_history = None;
    h.attempt_disaster_start = None;
    h.end_strike = None;
    h.post = None;
    h.subscribe = None;
    h.unsubscribe = None;
    h.add_agent = None;
    h.remove_agent = None;
    h
}

fn shadow(g: &Game) -> *mut fc4_aura_shadow {
    let h = read_only_host(g);
    let s = unsafe { fc4_aura_shadow_create(&h, CELLS, CELLS, 3) };
    assert!(!s.is_null());
    s
}

fn baseline(s: *mut fc4_aura_shadow, a: *const fc4_aura, g: &Game) -> (bool, Vec<String>) {
    let c = capture(a, g);
    let mut lines: Vec<String> = Vec::new();
    let r = unsafe { fc4_aura_shadow_baseline(s, &c.view(), Some(lines_sink), &mut lines as *mut _ as *mut c_void) };
    (r, lines)
}

fn shadow_cycle(s: *mut fc4_aura_shadow, a: *const fc4_aura, g: &Game) -> (i32, Vec<String>) {
    let c = capture(a, g);
    let mut lines: Vec<String> = Vec::new();
    let r = unsafe {
        fc4_aura_shadow_cycle(
            s,
            &c.view(),
            2_451_576,
            Some(lines_sink),
            &mut lines as *mut _ as *mut c_void,
        )
    };
    (r, lines)
}

fn shadow_stats(s: *mut fc4_aura_shadow) -> fc4_aura_shadow_stats {
    let mut st = fc4_aura_shadow_stats::default();
    assert!(unsafe { fc4_aura_shadow_get_stats(s, &mut st) });
    st
}

/// Everything a shadow cycle could change in the game.
fn fingerprint(g: &Game) -> String {
    let grids: Vec<(u64, Vec<f64>, i32, bool)> = g
        .grids
        .borrow()
        .iter()
        .map(|(t, gr)| (*t, gr.data.clone(), gr.refs, gr.shut))
        .collect();
    format!(
        "{grids:?} {:?} {:?} {:?} {:?} {:?} {:?}",
        g.agents.borrow(),
        g.subs.borrow(),
        g.posts.borrow(),
        g.history.borrow(),
        g.strikes.borrow(),
        g.disasters.borrow()
    )
}

#[test]
fn shadow_create_checks_its_arguments() {
    let g = Game::new();
    let h = read_only_host(&g);
    unsafe {
        assert!(fc4_aura_shadow_create(std::ptr::null(), CELLS, CELLS, 0).is_null());
        assert!(fc4_aura_shadow_create(&h, 0, CELLS, 0).is_null());
        assert!(fc4_aura_shadow_create(&h, CELLS, 5000, 0).is_null());
        let mut short = h;
        short.struct_size = 8;
        assert!(fc4_aura_shadow_create(&short, CELLS, CELLS, 0).is_null());
        // NULL handles answer the fallbacks.
        let n = std::ptr::null_mut();
        assert_eq!(
            fc4_aura_shadow_cycle(n, std::ptr::null(), 0, None, std::ptr::null_mut()),
            -1
        );
        assert!(!fc4_aura_shadow_baseline(
            n,
            std::ptr::null(),
            None,
            std::ptr::null_mut()
        ));
        let mut st = fc4_aura_shadow_stats::default();
        assert!(!fc4_aura_shadow_get_stats(n, &mut st));
        fc4_aura_shadow_destroy(n);
    }
}

/// The shadow of our own simulator as the "original": every cycle matches,
/// and the shadow writes nothing to the game.
#[test]
fn shadow_of_an_identical_original_matches() {
    let g = Game::new();
    let a = started(&g);
    let s = shadow(&g);
    let (ok, lines) = baseline(s, a, &g);
    assert!(ok, "{lines:?}");
    assert!(lines[0].contains("base 0 of 64 tracts with data"), "{lines:?}");
    for month in 0..3 {
        if month > 0 {
            assert!(message(a, msg::SIM_NEW_MONTH, 0, 0));
        }
        run_cycle(a);
        let before = fingerprint(&g);
        let (r, lines) = shadow_cycle(s, a, &g);
        assert_eq!(fingerprint(&g), before, "the shadow wrote to the game");
        assert_eq!(r, 0, "{lines:?}");
        assert_eq!(lines.len(), 1);
        let l = &lines[0];
        assert!(l.starts_with("aura shadow 2000-02-01 cycle"), "{l}");
        assert!(
            l.contains("tracts=64; base mismatches=0 max|d|=0; aura mismatches=0"),
            "{l}"
        );
        assert!(l.contains("mayor rating orig/ours=5/5"), "{l}");
        assert!(l.contains("max orig/ours=5@(2,2)/5@(2,2)"), "{l}");
        assert!(l.contains("hotspots orig/ours=0/0 (same)"), "{l}");
        assert!(l.contains("riot roll not compared"), "{l}");
        assert!(
            l.contains("inputs not available (neutral): air pollution, water pollution"),
            "{l}"
        );
    }
    let st = shadow_stats(s);
    assert_eq!((st.cycles, st.compared, st.clean, st.incomplete), (3, 3, 3, 0));
    assert_eq!(st.tracts, 3 * 64);
    assert_eq!(st.last_mismatches, 0);
    unsafe { fc4_aura_shadow_destroy(s) };
    stop(a);
}

/// A base tract the pipeline did not produce, and a transient effect during
/// the cycle (counted apart).
#[test]
fn shadow_reports_differences() {
    let g = Game::new();
    let a = started(&g);
    let s = shadow(&g);
    assert!(baseline(s, a, &g).0);
    // A transient effect on the lot's tract during the cycle (after the baseline).
    unsafe { fc4_aura_add_transient_effect(a, 8.0, 8.0, 8.0, 20.0) };
    run_cycle(a);
    // The original's base grid gains a tract our pipeline does not give.
    let base = g.grid_by_instance(grid_id::BASE, 4);
    g.grids.borrow_mut().get_mut(&base).unwrap().data[5 * 8 + 5] = 50.0;
    let (r, lines) = shadow_cycle(s, a, &g);
    let l = &lines[0];
    assert!(l.contains("base mismatches=1 max|d|=178"), "{l}");
    assert!(
        l.contains("aura mismatches=1 max|d|=20 (1 in the 1 tracts that got a transient effect"),
        "{l}"
    );
    assert!(l.contains("mayor rating orig/ours=25/5"), "{l}");
    assert!(
        l.contains("first differing (x,z) orig/ours: base (5,5) 50/-128; aura (0,0) 25/5"),
        "{l}"
    );
    assert_eq!(r, 3);
    let st = shadow_stats(s);
    assert_eq!((st.compared, st.clean, st.rating_mismatches), (1, 0, 1));
    assert_eq!((st.base_mismatches, st.aura_mismatches, st.aura_transient), (1, 1, 1));
    unsafe { fc4_aura_shadow_destroy(s) };
    stop(a);
}

#[test]
fn shadow_without_a_baseline_or_a_record_compares_nothing() {
    let g = Game::new();
    let a = started(&g);
    let s = shadow(&g);
    run_cycle(a);
    // No baseline yet: this cycle takes one.
    let (r, lines) = shadow_cycle(s, a, &g);
    assert_eq!(r, -1);
    assert!(lines[0].contains("baseline taken (no previous record)"), "{lines:?}");
    // No capture: INCOMPLETE, and the baseline is dropped.
    let mut lines: Vec<String> = Vec::new();
    let r = unsafe {
        fc4_aura_shadow_cycle(
            s,
            std::ptr::null(),
            2_451_576,
            Some(lines_sink),
            &mut lines as *mut _ as *mut c_void,
        )
    };
    assert_eq!(r, -1);
    assert!(lines[0].contains("INCOMPLETE (no capture)"), "{lines:?}");
    // A record of the version alone does not parse.
    let bytes = 9u16.to_le_bytes();
    let c = fc4_aura_capture {
        struct_size: std::mem::size_of::<fc4_aura_capture>() as u32,
        ok: 1,
        bytes: bytes.as_ptr(),
        len: 2,
        refs: std::ptr::null(),
        ref_count: 0,
    };
    let mut lines: Vec<String> = Vec::new();
    let r = unsafe { fc4_aura_shadow_cycle(s, &c, 2_451_576, Some(lines_sink), &mut lines as *mut _ as *mut c_void) };
    assert_eq!(r, -1);
    assert!(lines[0].contains("INCOMPLETE"), "{lines:?}");
    let st = shadow_stats(s);
    assert_eq!(
        (st.cycles, st.compared, st.incomplete, st.last_mismatches),
        (3, 0, 3, -1)
    );
    unsafe { fc4_aura_shadow_destroy(s) };
    stop(a);
}
