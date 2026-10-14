// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The residential C ABI (fc4.h "residential") against a fake game: the
//! residential grids through `fc4_grid_api`, the borrowed building map and
//! population maps, departments, occupants and their properties, the month,
//! persistence, getters on another thread (views), queued state changes and
//! the references handed to the game.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::{c_char, c_void, CStr};
use std::sync::mpsc;
use std::time::Duration;

use fc4_core::residential::{
    grid_class, history, instance, msg, shipped_tuning, AGENT_LEVELS, AGENT_NAME, OCCUPANT_TYPE_BUILDING,
};
use fc4_core::variant::mem;

use crate::budget::fc4_token_callback;
use crate::demand::fc4_u32_callback;
use crate::hostgrid::fc4_grid_api;
use crate::landvalue::fc4_grid_geometry;
use crate::props::{fc4_props, fc4_props_set_typed};
use crate::residential::*;
use crate::stream::fc4_ref;
use crate::test_streams::{istream, ostream, In, Out};

const CELLS: i32 = 32;
const BUILDING_MAP: u64 = 0x800;
const POP_MAP: [u64; 3] = [0x810, 0x820, 0x830];
const SCHOOL: u64 = 0x5C;
const SCHOOL_GROUP: u32 = 0x1503;

struct Grid {
    data: Vec<f64>,
    clsid: u32,
    tract_size: i32,
    refs: i32,
    shut: bool,
    instance: u32,
    input: bool,
}

impl Grid {
    fn count(&self) -> i32 {
        CELLS / self.tract_size
    }
}

type Prop = (u16, bool, Vec<u8>, usize);

#[derive(Default)]
struct Game {
    grids: RefCell<BTreeMap<u64, Grid>>,
    next_grid: Cell<u64>,
    agents: RefCell<Vec<(u32, String, u32)>>,
    subs: RefCell<Vec<u32>>,
    unsubs: RefCell<Vec<u32>>,
    removed_agent: Cell<bool>,
    queue_lengths: Cell<u32>,
    departments: RefCell<Vec<(u32, u32, f32, f32)>>,
    /// Occupant and department references (net).
    refs: RefCell<BTreeMap<u64, i32>>,
    hand_outs: Cell<u32>,
    history: RefCell<Vec<(u32, i32)>>,
    props: RefCell<BTreeMap<(u64, u32), Prop>>,
    /// A handle probed from another thread inside `record_history`.
    probe: Cell<usize>,
    probes: Cell<u32>,
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

impl Game {
    fn new() -> Box<Game> {
        let g = Box::<Game>::default();
        // The BDS building map: the R$ key (purpose 1: (1 << 2) + 1 + 1) in cell (0, 0).
        g.input(BUILDING_MAP, grid_class::U8, 1, 0.0);
        g.grids.borrow_mut().get_mut(&BUILDING_MAP).unwrap().data[0] = 6.0;
        // The growth developers' population maps: 100 R$ residents in tract (0, 0).
        for (i, t) in POP_MAP.iter().enumerate() {
            g.input(*t, grid_class::I16, 4, 0.0);
            if i == 0 {
                g.grids.borrow_mut().get_mut(t).unwrap().data[0] = 100.0;
            }
        }
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
                input: true,
            },
        );
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

    /// The residential grids (not the inputs, not the temporary grid).
    fn own_grids(&self) -> Vec<u64> {
        self.grids
            .borrow()
            .iter()
            .filter(|(_, g)| !g.input && g.instance != 0)
            .map(|(t, _)| *t)
            .collect()
    }

    fn grid_by_instance(&self, inst: u32) -> u64 {
        *self
            .grids
            .borrow()
            .iter()
            .find(|(_, g)| g.instance == inst)
            .expect("grid")
            .0
    }

    fn probe_now(&self) {
        let p = self.probe.get();
        if p == 0 {
            return;
        }
        self.probes.set(self.probes.get() + 1);
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let p = p as *const fc4_residential;
            let mut s = fc4_residential_snapshot::default();
            let ok = unsafe { fc4_residential_get_snapshot(p, &mut s) };
            let eq = unsafe { fc4_residential_global_quotient(p, true) };
            let _ = tx.send((ok, s.population_grids[0], eq));
        });
        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok((ok, pop, eq)) => assert!(ok && pop != 0 && eq.is_finite(), "the getters answer from a view"),
            Err(_) => panic!("a getter on another thread blocked during a main-thread host call"),
        }
    }
}

// ---- grids --------------------------------------------------------------------------------------------

unsafe extern "C" fn g_create(u: *mut c_void, clsid: u32, inst: u32, ts: i32) -> u64 {
    assert!(clsid == grid_class::U8 || clsid == grid_class::U16 || clsid == grid_class::F32);
    assert!(ts == 1 || ts == 4, "tract size {ts}");
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
            instance: inst,
            input: false,
        },
    );
    t
}
unsafe extern "C" fn g_class(u: *mut c_void, t: u64) -> u32 {
    game(u).grids.borrow().get(&t).map_or(0, |g| g.clsid)
}
unsafe extern "C" fn g_geometry(u: *mut c_void, t: u64, out: *mut fc4_grid_geometry) {
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
unsafe extern "C" fn g_get(u: *mut c_void, t: u64, kind: u32, x: i32, z: i32) -> f64 {
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
    assert!(!g.input, "another simulator's grid is never written");
    let n = g.count();
    if x >= 0 && z >= 0 && x < n && z < n {
        g.data[(x * n + z) as usize] = v;
    }
}
unsafe extern "C" fn g_fill(u: *mut c_void, t: u64, _kind: u32, v: f64) {
    let mut grids = game(u).grids.borrow_mut();
    let g = grids.get_mut(&t).unwrap();
    assert!(!g.input);
    g.data.fill(v);
}
unsafe extern "C" fn g_shutdown(u: *mut c_void, t: u64) {
    game(u).grids.borrow_mut().get_mut(&t).unwrap().shut = true;
}
unsafe extern "C" fn g_release(u: *mut c_void, t: u64) {
    game(u).grids.borrow_mut().get_mut(&t).unwrap().refs -= 1;
}

// ---- the rest of the game -------------------------------------------------------------------------------

unsafe extern "C" fn yes(_: *mut c_void) -> bool {
    true
}
unsafe extern "C" fn nothing(_: *mut c_void) {}
unsafe extern "C" fn add_agent(u: *mut c_void, level: u32, name: *const c_char, flags: u32) {
    let n = CStr::from_ptr(name).to_string_lossy().into_owned();
    game(u).agents.borrow_mut().push((level, n, flags));
}
unsafe extern "C" fn remove_agent(u: *mut c_void) {
    game(u).removed_agent.set(true);
}
unsafe extern "C" fn subscribe(u: *mut c_void, m: u32) {
    game(u).subs.borrow_mut().push(m);
}
unsafe extern "C" fn unsubscribe(u: *mut c_void, m: u32) {
    game(u).unsubs.borrow_mut().push(m);
}
unsafe extern "C" fn queue_length(u: *mut c_void, _series: u32, n: u32) {
    assert_eq!(n, history::QUEUE_LENGTH);
    let g = game(u);
    g.queue_lengths.set(g.queue_lengths.get() + 1);
}
unsafe extern "C" fn create_department(u: *mut c_void, id: u32, name: u32, max: f32, init: f32) -> u64 {
    let g = game(u);
    g.departments.borrow_mut().push((id, name, max, init));
    let t = 0xD00 + g.departments.borrow().len() as u64;
    g.bump(t, 1);
    t
}
unsafe extern "C" fn release_department(u: *mut c_void, t: u64) {
    game(u).bump(t, -1);
}
unsafe extern "C" fn add_ref(u: *mut c_void, t: u64) {
    game(u).bump(t, 1);
}
unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    game(u).bump(t, -1);
}
unsafe extern "C" fn hand_out(u: *mut c_void, t: u64) {
    let g = game(u);
    g.hand_outs.set(g.hand_outs.get() + 1);
    if let Some(gr) = g.grids.borrow_mut().get_mut(&t) {
        gr.refs += 1;
        return;
    }
    g.bump(t, 1);
}
unsafe extern "C" fn tuning(_: *mut c_void) -> *mut fc4_props {
    Box::into_raw(Box::new(shipped_tuning())) as *mut fc4_props
}
unsafe extern "C" fn building_map(_: *mut c_void) -> u64 {
    BUILDING_MAP
}
unsafe extern "C" fn building_map_key(_: *mut c_void, purpose: u32, wealth: u32) -> u8 {
    ((purpose << 2) + wealth + 1) as u8
}
unsafe extern "C" fn developer_population_map(_: *mut c_void, dev_type: u32) -> u64 {
    match dev_type {
        0x1010 => POP_MAP[0],
        0x1020 => POP_MAP[1],
        0x1030 => POP_MAP[2],
        _ => 0,
    }
}
unsafe extern "C" fn sim_month(_: *mut c_void) -> u32 {
    3
}
unsafe extern "C" fn sim_day_number(_: *mut c_void) -> u32 {
    1000
}
unsafe extern "C" fn record_history(u: *mut c_void, series: u32, value: i32) {
    let g = game(u);
    g.probe_now();
    g.history.borrow_mut().push((series, value));
}
unsafe extern "C" fn occupant_as_building(_: *mut c_void, occ: u64) -> u64 {
    occ
}
/// The school is recognised by group 0x1503 (REH §4.1) and belongs to the
/// K-6 group list (0x150F).
unsafe extern "C" fn is_occupant_group(_: *mut c_void, occ: u64, group: u32) -> bool {
    occ == SCHOOL && (group == SCHOOL_GROUP || group == 0x150F)
}
unsafe extern "C" fn resolve_ref(_: *mut c_void, t: u64, _iid: u32) -> u64 {
    t
}
unsafe extern "C" fn occupant_property(u: *mut c_void, occ: u64, id: u32, out: *mut fc4_props) -> bool {
    match game(u).props.borrow().get(&(occ, id)) {
        Some((t, a, b, n)) => fc4_props_set_typed(out, id, *t, *a, b.as_ptr() as *const c_void, *n),
        None => false,
    }
}
unsafe extern "C" fn set_u32(u: *mut c_void, occ: u64, id: u32, v: u32) {
    let b = v.to_ne_bytes().to_vec();
    game(u).props.borrow_mut().insert((occ, id), (mem::UINT32, false, b, 1));
}
unsafe extern "C" fn set_f32(u: *mut c_void, occ: u64, id: u32, v: f32) {
    let b = v.to_ne_bytes().to_vec();
    game(u)
        .props
        .borrow_mut()
        .insert((occ, id), (mem::FLOAT32, false, b, 1));
}
unsafe extern "C" fn ordinances_on(_: *mut c_void, _cb: fc4_u32_callback, _ctx: *mut c_void) {}

fn host(g: &Game) -> fc4_residential_host {
    fc4_residential_host {
        struct_size: std::mem::size_of::<fc4_residential_host>() as u32,
        user: g.user(),
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
        acquire_services: Some(nothing),
        acquire_developers: Some(nothing),
        acquire_demand_node: Some(yes),
        acquire_message_server: Some(yes),
        acquire_budget_simulator: Some(yes),
        has_simulator: Some(yes),
        add_agent: Some(add_agent),
        remove_agent: Some(remove_agent),
        subscribe: Some(subscribe),
        unsubscribe: Some(unsubscribe),
        release_message_server_and_budget: Some(nothing),
        release_services: Some(nothing),
        set_history_queue_length: Some(queue_length),
        create_department: Some(create_department),
        release_department: Some(release_department),
        add_ref: Some(add_ref),
        release: Some(release),
        hand_out: Some(hand_out),
        tuning: Some(tuning),
        building_map: Some(building_map),
        building_map_key: Some(building_map_key),
        developer_population_map: Some(developer_population_map),
        pollution_value: None,
        sim_month: Some(sim_month),
        sim_day_number: Some(sim_day_number),
        altitude: None,
        create_visual_effect: None,
        set_effect_transform: None,
        start_and_release_effect: None,
        record_history: Some(record_history),
        post_strike_message: None,
        occupant_as_building: Some(occupant_as_building),
        is_occupant_group: Some(is_occupant_group),
        occupant_bool_property: None,
        set_occupant_bool_property: None,
        resolve_ref: Some(resolve_ref),
        budget_item: None,
        change_budget_item_line: None,
        department_budget: None,
        department_funding: None,
        is_locally_funded: None,
        add_locally_funded: None,
        remove_locally_funded: None,
        set_local_funding: None,
        line_item: None,
        create_line_item_for_building_type: None,
        remove_line_item: None,
        line_item_count: None,
        set_line_item_count: None,
        set_line_item_display_flag: None,
        occupant_property: Some(occupant_property),
        set_occupant_u32: Some(set_u32),
        set_occupant_f32: Some(set_f32),
        building_type: None,
        building_slot11: None,
        occupant_lot_center: None,
        occupant_position: None,
        ordinances_on: Some(ordinances_on),
        ordinance_properties: None,
        add_controllers: None,
        remove_controllers: None,
    }
}

fn started(g: &Game) -> *mut fc4_residential {
    let h = host(g);
    let p = unsafe { fc4_residential_create(&h, 0xA0, 7) };
    assert!(!p.is_null());
    assert!(unsafe { fc4_residential_init(p) });
    p
}

fn snap(p: *const fc4_residential) -> fc4_residential_snapshot {
    let mut s = fc4_residential_snapshot::default();
    assert!(unsafe { fc4_residential_get_snapshot(p, &mut s) });
    s
}

fn state_of(p: *const fc4_residential) -> fc4_residential_state {
    let mut s = fc4_residential_state::default();
    assert!(unsafe { fc4_residential_get_state(p, &mut s) });
    s
}

fn day(p: *mut fc4_residential) -> bool {
    unsafe { fc4_residential_do_message(p, msg::SIM_NEW_DAY, 0, 0, 0, 0) }
}

/// Days until the cycle reaches the idle state 10.
fn run_month(p: *mut fc4_residential) {
    for _ in 0..40 {
        if snap(p).state == 10 {
            return;
        }
        assert!(day(p), "an unbounded work budget finishes every state");
    }
    panic!("the cycle did not reach state 10");
}

fn on_thread<R: Send + 'static>(f: impl FnOnce() -> R + Send + 'static) -> R {
    std::thread::spawn(f).join().expect("thread")
}

#[test]
fn a_short_table_is_refused() {
    let g = Game::new();
    let mut h = host(&g);
    h.struct_size -= 4;
    assert!(unsafe { fc4_residential_create(&h, 1, 0) }.is_null());
    h = host(&g);
    h.grids.struct_size = 8;
    assert!(unsafe { fc4_residential_create(&h, 1, 0) }.is_null());
}

#[test]
fn init_creates_the_grids_departments_and_agents() {
    let g = Game::new();
    let p = started(&g);
    let agents = g.agents.borrow().clone();
    assert_eq!(
        agents.iter().map(|a| a.0).collect::<Vec<_>>(),
        AGENT_LEVELS.to_vec(),
        "levels 0, 10, 6, 4"
    );
    assert!(agents.iter().all(|a| a.1 == AGENT_NAME && a.2 == 2));
    assert_eq!(*g.subs.borrow(), msg::SUBSCRIPTIONS.to_vec());
    assert_eq!(g.queue_lengths.get(), 20);
    assert_eq!(g.own_grids().len(), 25, "every grid of §2");
    // The population tract size comes from the R$ population map (4); proximity is 1.
    let prox = g.grid_by_instance(instance::PROXIMITY[0]);
    let pop = g.grid_by_instance(instance::POPULATION[0]);
    assert_eq!(g.grids.borrow()[&prox].tract_size, 1);
    assert_eq!(g.grids.borrow()[&pop].tract_size, 4);
    assert_eq!(g.grids.borrow()[&pop].clsid, grid_class::U16);
    assert_eq!(
        g.grids.borrow()[&g.grid_by_instance(instance::EQ[2])].clsid,
        grid_class::F32
    );
    let s = snap(p);
    assert_eq!(s.initialized, 1);
    assert_eq!(s.proximity[0], prox);
    assert_eq!(s.population_grids[0], pop);
    assert!(s.eq.iter().chain(&s.hq).all(|&t| t != 0) && s.age != 0);
    let st = state_of(p);
    assert_eq!((st.departments, st.grids), (4, 25));
    assert_eq!(g.departments.borrow().len(), 4);

    assert!(unsafe { fc4_residential_shutdown(p) });
    assert!(g.removed_agent.get());
    assert_eq!(*g.unsubs.borrow(), msg::SUBSCRIPTIONS.to_vec());
    for t in g.own_grids() {
        let gr = &g.grids.borrow()[&t];
        assert!(gr.shut && gr.refs == 0, "grid {t:#x}: shut down and released");
    }
    assert!(g.refs.borrow().values().all(|&n| n == 0), "departments released");
    let s = snap(p);
    assert_eq!((s.initialized, s.population_grids[0]), (0, 0));
    unsafe { fc4_residential_destroy(p) };
}

#[test]
fn a_month_publishes_the_population() {
    let g = Game::new();
    let p = started(&g);
    assert_eq!(snap(p).state, 0);
    run_month(p);
    let s = snap(p);
    assert_eq!(s.population, 100);
    let st = state_of(p);
    assert_eq!(st.population, 100);
    assert_eq!(st.population_by_wealth, [100, 0, 0]);
    assert_eq!((st.tracts_x, st.tracts_z), (8, 8));
    assert!(g.history.borrow().contains(&(history::POPULATION, 100)));
    // The temporary grid was released (state 3) and only the 25 remain held.
    let temp_held = g
        .grids
        .borrow()
        .values()
        .filter(|gr| !gr.input && gr.instance == 0 && gr.refs > 0)
        .count();
    assert_eq!(temp_held, 0);
    assert!(unsafe { fc4_residential_shutdown(p) });
    unsafe { fc4_residential_destroy(p) };
}

#[test]
fn getters_on_another_thread_run_on_a_view() {
    let g = Game::new();
    let p = started(&g);
    run_month(p);
    let main = unsafe {
        (
            fc4_residential_global_quotient(p, true),
            fc4_residential_global_quotient(p, false),
            fc4_residential_cell_quotient(p, true, 1, 1),
            fc4_residential_global_le(p),
            fc4_residential_cell_life_expectancy(p, 1, 1),
            fc4_residential_new_age_by_wealth(p, 2),
            fc4_residential_system_rating(p, true),
        )
    };
    let mut mm = fc4_residential_min_max::default();
    assert!(unsafe { fc4_residential_quotient_min_max(p, true, &mut mm) });
    let views_before = state_of(p).view_calls;
    let pp = p as usize;
    let (other, other_mm) = on_thread(move || unsafe {
        let p = pp as *mut fc4_residential;
        let mut mm = fc4_residential_min_max::default();
        let ok = fc4_residential_quotient_min_max(p, true, &mut mm);
        assert!(ok);
        (
            (
                fc4_residential_global_quotient(p, true),
                fc4_residential_global_quotient(p, false),
                fc4_residential_cell_quotient(p, true, 1, 1),
                fc4_residential_global_le(p),
                fc4_residential_cell_life_expectancy(p, 1, 1),
                fc4_residential_new_age_by_wealth(p, 2),
                fc4_residential_system_rating(p, true),
            ),
            mm,
        )
    });
    assert_eq!(other, main, "a view answers as the core does");
    assert_eq!(other_mm, mm);
    assert_eq!(state_of(p).view_calls, views_before + 8);
    assert!(main.0 > 0.0, "a populated tract has an EQ");

    // Inside a main-thread host call (state 9 records history), a getter on
    // another thread neither blocks nor reaches the core.
    g.probe.set(p as usize);
    unsafe { fc4_residential_do_message(p, msg::SIM_NEW_MONTH, 0, 0, 0, 0) };
    run_month(p);
    assert!(g.probes.get() > 0);
    g.probe.set(0);
    // The lifecycle is refused off the main thread.
    assert!(!on_thread(move || unsafe {
        fc4_residential_init(pp as *mut fc4_residential)
    }));
    assert!(unsafe { fc4_residential_shutdown(p) });
    unsafe { fc4_residential_destroy(p) };
}

#[test]
fn state_changes_off_the_main_thread_are_queued() {
    let g = Game::new();
    let p = started(&g);
    let pp = p as usize;
    assert!(on_thread(move || unsafe {
        fc4_residential_set_global_auto_budget(pp as *mut fc4_residential, true, true)
    }));
    assert_eq!(snap(p).school_auto_budget, 0, "not yet");
    // An occupant message from another thread holds its occupant while queued.
    let msg_ok = on_thread(move || unsafe {
        fc4_residential_do_message(
            pp as *mut fc4_residential,
            msg::OCCUPANT_INSERTED,
            0,
            0,
            OCCUPANT_TYPE_BUILDING,
            SCHOOL,
        )
    });
    assert!(msg_ok);
    assert_eq!(g.refs_of(SCHOOL), 1, "held by the queue");
    assert_eq!(state_of(p).deferred_calls, 2);
    // The next main-thread call runs them.
    unsafe { fc4_residential_do_message(p, msg::SIM_IDLE, 0, 0, 0, 0) };
    let s = snap(p);
    assert_eq!(s.school_auto_budget, 1);
    assert_eq!(s.school_building_count, 1, "the school is pending");
    assert_eq!(
        g.refs_of(SCHOOL),
        1,
        "the queue's reference given back, the list's kept"
    );
    // On the idle main thread a setter runs at once.
    assert!(!unsafe { fc4_residential_set_global_auto_budget(p, false, false) });
    assert_eq!(snap(p).hospital_auto_budget, 0);
    assert!(unsafe { fc4_residential_shutdown(p) });
    assert_eq!(g.refs_of(SCHOOL), 0, "Shutdown releases the lists");
    unsafe { fc4_residential_destroy(p) };
}

unsafe extern "C" fn collect(ctx: *mut c_void, t: u64) {
    (*(ctx as *mut Vec<u64>)).push(t);
}

fn buildings(p: *const fc4_residential, which: u32, groups: Option<&[u32]>) -> Vec<u64> {
    let mut v: Vec<u64> = Vec::new();
    let (ptr, n) = groups.map_or((std::ptr::null(), 0), |g| (g.as_ptr(), g.len() as u32));
    let sink: fc4_token_callback = Some(collect);
    let count = unsafe { fc4_residential_buildings(p, which, ptr, n, sink, &mut v as *mut Vec<u64> as *mut c_void) };
    assert_eq!(count as usize, v.len());
    v
}

#[test]
fn building_lists_and_grids_are_handed_out() {
    let g = Game::new();
    let p = started(&g);
    assert!(unsafe { fc4_residential_do_message(p, msg::OCCUPANT_INSERTED, 0, 0, OCCUPANT_TYPE_BUILDING, SCHOOL) });
    assert_eq!(g.refs_of(SCHOOL), 1);
    assert_eq!(buildings(p, FC4_RESIDENTIAL_LIST_SCHOOL, None), vec![SCHOOL]);
    assert_eq!(
        (g.hand_outs.get(), g.refs_of(SCHOOL)),
        (1, 2),
        "one reference for the caller"
    );
    // The pending list always follows the selected groups.
    assert_eq!(buildings(p, FC4_RESIDENTIAL_LIST_SCHOOL, Some(&[0x1504])), vec![SCHOOL]);
    assert!(buildings(p, FC4_RESIDENTIAL_LIST_HOSPITAL, None).is_empty());
    assert!(buildings(p, FC4_RESIDENTIAL_LIST_HEALTH_AFFECTING, None).is_empty());
    let pp = p as usize;
    let other = on_thread(move || buildings(pp as *const fc4_residential, FC4_RESIDENTIAL_LIST_SCHOOL, None));
    assert_eq!(other, vec![SCHOOL]);
    assert_eq!(g.refs_of(SCHOOL), 4, "a view hands out too");
    let mut q = fc4_residential_query_data::default();
    assert!(
        !unsafe { fc4_residential_query_data(p, true, SCHOOL, &mut q) },
        "no capacity property"
    );

    run_month(p);
    let (mut lo, mut hi) = (0.0f32, 0.0f32);
    let t = unsafe { fc4_residential_average_grid(p, true, &mut lo, &mut hi) };
    assert_ne!(t, 0);
    {
        let grids = g.grids.borrow();
        let avg = &grids[&t];
        assert_eq!(
            (avg.instance, avg.clsid, avg.tract_size),
            (instance::AVERAGE_EQ, grid_class::F32, 4)
        );
        assert_eq!(avg.refs, 1, "a new grid owned by the caller");
    }
    assert!(lo <= hi);
    let t2 = on_thread(move || unsafe {
        fc4_residential_average_grid(
            pp as *const fc4_residential,
            false,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    });
    assert_ne!(t2, t);
    assert_eq!(g.grids.borrow()[&t2].instance, instance::AVERAGE_HQ);
    assert_eq!(g.grids.borrow()[&t2].refs, 1);
    assert!(unsafe { fc4_residential_shutdown(p) });
    unsafe { fc4_residential_destroy(p) };
}

#[test]
fn the_record_round_trips_and_a_view_writes_the_same() {
    let g = Game::new();
    let p = started(&g);
    unsafe { fc4_residential_do_message(p, msg::OCCUPANT_INSERTED, 0, 0, OCCUPANT_TYPE_BUILDING, SCHOOL) };
    run_month(p);
    let mut out = Out::default();
    let os = ostream(&mut out);
    assert!(unsafe { fc4_residential_write(p, &os) });
    let pop = g.grid_by_instance(instance::POPULATION[0]);
    assert!(out.refs.contains(&(pop, grid_class::U16)));
    assert!(out.refs.contains(&(SCHOOL, 0)), "occupants with their token");
    assert_eq!(
        out.refs.iter().filter(|r| r.1 != 0).count(),
        25,
        "every grid by reference"
    );

    // The same record from another thread (a view of the published state).
    let pp = p as usize;
    let (bytes, refs) = on_thread(move || {
        let mut o = Out::default();
        let os = ostream(&mut o);
        assert!(unsafe { fc4_residential_write(pp as *const fc4_residential, &os) });
        (o.bytes, o.refs)
    });
    assert_eq!((bytes, refs), (out.bytes.clone(), out.refs.clone()));

    // Read runs before Init and takes the references the stream hands over.
    let h = host(&g);
    let q = unsafe { fc4_residential_create(&h, 0xA1, 9) };
    let gp: *const Game = &*g;
    let take = move |r: &mut fc4_ref| unsafe {
        let g = &*gp;
        let mut grids = g.grids.borrow_mut();
        match grids.get_mut(&r.token) {
            Some(gr) => gr.refs += 1,
            None => {
                drop(grids);
                g.bump(r.token, 1);
            }
        }
    };
    let mut inp = In::new(&out.bytes);
    inp.resolve = Some(&take);
    let is = istream(&mut inp);
    assert!(unsafe { fc4_residential_read(q, &is) });
    let (a, b) = (snap(p), snap(q));
    assert_eq!(b.initialized, 0);
    assert_eq!(
        (b.population, b.state, b.school_building_count),
        (a.population, a.state, a.school_building_count)
    );
    assert_eq!(
        (b.population_grids, b.eq, b.hq, b.age),
        (a.population_grids, a.eq, a.hq, a.age)
    );
    unsafe { fc4_residential_destroy(q) };
    assert_eq!(
        g.grids.borrow()[&pop].refs,
        1,
        "the reader's grid references are given back"
    );

    // A truncated record fails, gives back what it took and leaves the object as it was.
    let mut bad = out.bytes.clone();
    bad.truncate(40);
    let q = unsafe { fc4_residential_create(&h, 0xA2, 9) };
    let mut inp = In::new(&bad);
    inp.resolve = Some(&take);
    let is = istream(&mut inp);
    assert!(!unsafe { fc4_residential_read(q, &is) });
    assert_eq!(snap(q).population_grids, [0; 3]);
    unsafe { fc4_residential_destroy(q) };
    assert_eq!(g.grids.borrow()[&pop].refs, 1);
    assert!(unsafe { fc4_residential_shutdown(p) });
    unsafe { fc4_residential_destroy(p) };
}
