// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The power C ABI against a fake game: the game's grid through
//! `fc4_grid_api`, occupants and their properties, the reference protocol,
//! persistence through host streams and re-entrant messages.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::{c_char, c_void};

use fc4_core::power::{budget, msg, prop, shipped_tuning, state, GRID_CLSID, GRID_INSTANCE};
use fc4_core::variant::mem;

use crate::budget::fc4_token_callback;
use crate::hostgrid::{fc4_cell_rect, fc4_grid_api};
use crate::landvalue::fc4_grid_geometry;
use crate::power::*;
use crate::props::{fc4_props, fc4_props_set_typed};
use crate::test_streams::{istream, ostream, In, Out};

const CELLS: i32 = 32;
const PLANT: u64 = 10;
const CONSUMER: u64 = 11;
const DEPT: u64 = 0xD0;
const OCCUPANT_BUILDING: u32 = 0x2781_28A0;

struct Grid {
    data: Vec<u8>,
    refs: i32,
    shut: bool,
    instance: u32,
}

/// (element type, array flag, bytes, count).
type Written = (u16, bool, Vec<u8>, usize);

#[derive(Default)]
struct Game {
    grids: RefCell<HashMap<u64, Grid>>,
    next_grid: Cell<u64>,
    refs: RefCell<HashMap<u64, i32>>,
    agents: RefCell<Vec<(u32, String, u32)>>,
    subs: RefCell<Vec<u32>>,
    posts: RefCell<Vec<fc4_power_post>>,
    dept: Cell<u64>,
    services: Cell<i32>,
    /// Properties the simulator wrote: (occupant, id) → (type, array, bytes, count).
    written: RefCell<HashMap<(u64, u32), Written>>,
    /// Re-entrancy test: post() delivers this message to `power`.
    power: Cell<usize>,
    reenter: Cell<u32>,
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

impl Game {
    fn refs_of(&self, t: u64) -> i32 {
        self.refs.borrow().get(&t).copied().unwrap_or(0)
    }
}

// ---- grids --------------------------------------------------------------------------------------------

unsafe extern "C" fn g_create(u: *mut c_void, clsid: u32, instance: u32, ts: i32) -> u64 {
    assert_eq!((clsid, ts), (GRID_CLSID, 1));
    let g = game(u);
    let t = g.next_grid.get() + 0x7000;
    g.next_grid.set(g.next_grid.get() + 0x10);
    g.grids.borrow_mut().insert(
        t,
        Grid {
            data: vec![0; (CELLS * CELLS) as usize],
            refs: 1,
            shut: false,
            instance,
        },
    );
    t
}
unsafe extern "C" fn g_class(u: *mut c_void, t: u64) -> u32 {
    if game(u).grids.borrow().contains_key(&t) {
        GRID_CLSID
    } else {
        0
    }
}
unsafe extern "C" fn g_geometry(_: *mut c_void, _t: u64, out: *mut fc4_grid_geometry) {
    *out = fc4_grid_geometry {
        tract_size: 1,
        shift: 0,
        count_x: CELLS,
        count_z: CELLS,
        width_x: 16.0,
        width_z: 16.0,
        inv_width_x: 1.0 / 16.0,
        inv_width_z: 1.0 / 16.0,
    };
}
unsafe extern "C" fn g_get(u: *mut c_void, t: u64, kind: u32, x: i32, z: i32) -> f64 {
    assert_eq!(kind, GRID_CLSID);
    f64::from(game(u).grids.borrow()[&t].data[(x * CELLS + z) as usize])
}
unsafe extern "C" fn g_set(u: *mut c_void, t: u64, _kind: u32, x: i32, z: i32, v: f64) {
    game(u).grids.borrow_mut().get_mut(&t).unwrap().data[(x * CELLS + z) as usize] = v as u8;
}
unsafe extern "C" fn g_fill(u: *mut c_void, t: u64, _kind: u32, v: f64) {
    game(u).grids.borrow_mut().get_mut(&t).unwrap().data.fill(v as u8);
}
unsafe extern "C" fn g_shutdown(u: *mut c_void, t: u64) {
    game(u).grids.borrow_mut().get_mut(&t).unwrap().shut = true;
}
unsafe extern "C" fn g_release(u: *mut c_void, t: u64) {
    game(u).grids.borrow_mut().get_mut(&t).unwrap().refs -= 1;
}

// ---- city, simulator, messages ---------------------------------------------------------------------------

unsafe extern "C" fn cell_counts(_: *mut c_void, x: *mut i32, z: *mut i32) {
    *x = CELLS;
    *z = CELLS;
}
unsafe extern "C" fn cell_widths(_: *mut c_void, x: *mut f32, z: *mut f32) {
    *x = 16.0;
    *z = 16.0;
}
unsafe extern "C" fn tuning(_: *mut c_void) -> *mut fc4_props {
    Box::into_raw(Box::new(shipped_tuning())) as *mut fc4_props
}
unsafe extern "C" fn acquire(u: *mut c_void) {
    game(u).services.set(game(u).services.get() + 1);
}
unsafe extern "C" fn release_services(u: *mut c_void) {
    game(u).services.set(game(u).services.get() - 1);
}
unsafe extern "C" fn add_agent(u: *mut c_void, level: u32, name: *const c_char, len: usize, flags: u32) {
    let name = String::from_utf8(std::slice::from_raw_parts(name as *const u8, len).to_vec()).unwrap();
    game(u).agents.borrow_mut().push((level, name, flags));
}
unsafe extern "C" fn remove_agent(u: *mut c_void) {
    game(u).agents.borrow_mut().clear();
}
unsafe extern "C" fn subscribe(u: *mut c_void, ids: *const u32, n: usize) {
    game(u)
        .subs
        .borrow_mut()
        .extend_from_slice(std::slice::from_raw_parts(ids, n));
}
unsafe extern "C" fn unsubscribe(u: *mut c_void, ids: *const u32, n: usize) {
    let ids = std::slice::from_raw_parts(ids, n);
    game(u).subs.borrow_mut().retain(|i| !ids.contains(i));
}
unsafe extern "C" fn post(u: *mut c_void, p: *const fc4_power_post) {
    let g = game(u);
    g.posts.borrow_mut().push(*p);
    // A synchronous delivery back into the simulator (re-entrancy test).
    let target = g.power.get();
    if target != 0 && g.reenter.get() == 0 {
        g.reenter.set(1);
        let m = fc4_power_message {
            msg_type: msg::OCCUPANT_INSERTED,
            void1: CONSUMER,
            ..Default::default()
        };
        assert!(fc4_power_do_message(target as *mut fc4_power, &m));
    }
}
unsafe extern "C" fn add_ref(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_insert(0) += 1;
}
unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_insert(0) -= 1;
}
unsafe extern "C" fn resolve_ref(_: *mut c_void, t: u64, _iid: u32) -> u64 {
    t
}

// ---- occupants -----------------------------------------------------------------------------------------

fn cells_of(occ: u64) -> fc4_cell_rect {
    match occ {
        PLANT => fc4_cell_rect {
            x1: 8,
            z1: 0,
            x2: 9,
            z2: 1,
        },
        _ => fc4_cell_rect {
            x1: 12,
            z1: 0,
            x2: 13,
            z2: 1,
        },
    }
}
unsafe extern "C" fn occupant_type(_: *mut c_void, occ: u64) -> u32 {
    if occ == PLANT || occ == CONSUMER {
        OCCUPANT_BUILDING
    } else {
        0
    }
}
unsafe fn scalar<T>(p: *mut fc4_props, id: u32, mem_type: u16, v: T) -> bool {
    fc4_props_set_typed(p, id, mem_type, false, &v as *const T as *const c_void, 1)
}
unsafe extern "C" fn occupant_property(u: *mut c_void, occ: u64, id: u32, p: *mut fc4_props) -> bool {
    if let Some((t, a, b, n)) = game(u).written.borrow().get(&(occ, id)) {
        return fc4_props_set_typed(p, id, *t, *a, b.as_ptr() as *const c_void, *n);
    }
    match (occ, id) {
        (PLANT, prop::CAPACITY) => scalar(p, id, mem::UINT32, 1000u32),
        (PLANT, prop::PLANT_TYPE) => scalar(p, id, mem::SINT32, 1i32),
        (PLANT, prop::CONDITION_CURVE) => {
            let c = [0.0f32, 1.0, 1.0, 1.0];
            fc4_props_set_typed(p, id, mem::FLOAT32, true, c.as_ptr() as *const c_void, c.len())
        }
        (CONSUMER, prop::REQUIREMENT) => scalar(p, id, mem::UINT32, 500u32),
        _ => false,
    }
}
unsafe extern "C" fn set_occupant_property(
    u: *mut c_void,
    occ: u64,
    id: u32,
    mem_type: u16,
    is_array: bool,
    data: *const c_void,
    count: usize,
) {
    let size = match mem_type {
        mem::BOOL | mem::UINT8 | mem::SINT8 | mem::CHAR | mem::RZ_CHAR => 1,
        mem::UINT16 | mem::SINT16 | mem::UNICODE_CHAR => 2,
        mem::UINT64 | mem::SINT64 | mem::FLOAT64 => 8,
        _ => 4,
    };
    let bytes = std::slice::from_raw_parts(data as *const u8, size * count).to_vec();
    game(u)
        .written
        .borrow_mut()
        .insert((occ, id), (mem_type, is_array, bytes, count));
}
unsafe extern "C" fn bounding_box(_: *mut c_void, occ: u64, a: *mut f32, b: *mut f32) {
    let r = cells_of(occ);
    let (a, b) = (
        std::slice::from_raw_parts_mut(a, 3),
        std::slice::from_raw_parts_mut(b, 3),
    );
    a.copy_from_slice(&[r.x1 as f32 * 16.0, 0.0, r.z1 as f32 * 16.0]);
    b.copy_from_slice(&[(r.x2 + 1) as f32 * 16.0, 10.0, (r.z2 + 1) as f32 * 16.0]);
}
unsafe extern "C" fn position(_: *mut c_void, occ: u64, out: *mut f32) {
    let r = cells_of(occ);
    let o = std::slice::from_raw_parts_mut(out, 3);
    o.copy_from_slice(&[(r.x1 + r.x2 + 1) as f32 * 8.0, 0.0, (r.z1 + r.z2 + 1) as f32 * 8.0]);
}
unsafe extern "C" fn bounding_cells(_: *mut c_void, occ: u64, out: *mut fc4_cell_rect) {
    *out = cells_of(occ);
}
unsafe extern "C" fn in_manager(_: *mut c_void, _occ: u64) -> bool {
    true
}
unsafe extern "C" fn no_tokens(_: *mut c_void, _r: *const fc4_cell_rect, _cb: fc4_token_callback, _ctx: *mut c_void) {}

// ---- budget ---------------------------------------------------------------------------------------------

unsafe extern "C" fn department(u: *mut c_void, id: u32) -> u64 {
    if id == budget::DEPARTMENT {
        game(u).dept.get()
    } else {
        0
    }
}
unsafe extern "C" fn create_department(u: *mut c_void, id: u32, _group: u32) -> u64 {
    assert_eq!(id, budget::DEPARTMENT);
    game(u).dept.set(DEPT);
    DEPT
}
unsafe extern "C" fn department_funding(_: *mut c_void, _d: u64) -> f32 {
    1.0
}
unsafe extern "C" fn create_line_item(_: *mut c_void, _d: u64, line: u32, _f: bool) -> u64 {
    0x1000 + u64::from(line)
}
unsafe extern "C" fn budget_item(_: *mut c_void, occ: u64, purpose: u32, out: *mut fc4_power_budget_item) -> bool {
    if occ != PLANT || purpose != budget::PURPOSE {
        return false;
    }
    *out = fc4_power_budget_item {
        department: budget::DEPARTMENT,
        line: 7,
        cost: 250,
        index: 0,
        local_funding: 1.0,
    };
    true
}

fn host(g: &Game) -> fc4_power_host {
    let mut h: fc4_power_host = unsafe { std::mem::zeroed() };
    h.struct_size = std::mem::size_of::<fc4_power_host>() as u32;
    h.user = g as *const Game as *mut c_void;
    h.grids = fc4_grid_api {
        struct_size: std::mem::size_of::<fc4_grid_api>() as u32,
        user: h.user,
        create: Some(g_create),
        class_of: Some(g_class),
        geometry: Some(g_geometry),
        get: Some(g_get),
        set: Some(g_set),
        fill: Some(g_fill),
        shutdown: Some(g_shutdown),
        release: Some(g_release),
    };
    h.cell_counts = Some(cell_counts);
    h.cell_widths = Some(cell_widths);
    h.tuning = Some(tuning);
    h.acquire_services = Some(acquire);
    h.release_services = Some(release_services);
    h.add_agent = Some(add_agent);
    h.remove_agent = Some(remove_agent);
    h.subscribe = Some(subscribe);
    h.unsubscribe = Some(unsubscribe);
    h.post = Some(post);
    h.add_ref = Some(add_ref);
    h.release = Some(release);
    h.resolve_ref = Some(resolve_ref);
    h.occupant_type = Some(occupant_type);
    h.occupant_property = Some(occupant_property);
    h.set_occupant_property = Some(set_occupant_property);
    h.occupant_position = Some(position);
    h.occupant_bounding_box = Some(bounding_box);
    h.occupant_bounding_cells = Some(bounding_cells);
    h.occupant_in_manager = Some(in_manager);
    h.occupants_in_cells = Some(no_tokens);
    h.department = Some(department);
    h.create_department = Some(create_department);
    h.department_funding = Some(department_funding);
    h.create_line_item = Some(create_line_item);
    h.create_line_item_for_building_type = Some(create_line_item);
    h.budget_item_for_purpose = Some(budget_item);
    h
}

fn message(p: *mut fc4_power, msg_type: u32, void1: u64) {
    let m = fc4_power_message {
        msg_type,
        void1,
        ..Default::default()
    };
    assert!(unsafe { fc4_power_do_message(p, &m) });
}

fn state_of(p: *const fc4_power) -> fc4_power_state {
    let mut s = fc4_power_state::default();
    assert!(unsafe { fc4_power_get_state(p, &mut s) });
    s
}

/// Idle steps until the state machine is back at 4 after leaving it.
fn cycle(p: *mut fc4_power) {
    let mut left = false;
    for _ in 0..16 {
        message(p, msg::SIM_IDLE, 0);
        let st = state_of(p).state;
        if st != state::IDLE {
            left = true;
        } else if left {
            return;
        }
    }
    panic!("the cycle did not finish");
}

fn started(g: &Game) -> *mut fc4_power {
    let h = host(g);
    let p = unsafe { fc4_power_create(&h, 0x51) };
    assert!(!p.is_null());
    assert!(unsafe { fc4_power_init(p) });
    message(p, msg::CONNECTIONS_READY, 0);
    p
}

#[test]
fn create_rejects_short_tables() {
    let g = Game::default();
    let mut h = host(&g);
    h.grids.struct_size = 8;
    assert!(unsafe { fc4_power_create(&h, 0) }.is_null());
    let mut h = host(&g);
    h.struct_size = 8;
    assert!(unsafe { fc4_power_create(&h, 0) }.is_null());
    assert!(unsafe { fc4_power_create(std::ptr::null(), 0) }.is_null());
    // NULL handles answer the fallbacks.
    unsafe {
        assert!(!fc4_power_init(std::ptr::null_mut()));
        assert_eq!(fc4_power_slot_value(std::ptr::null(), 24), 0);
        fc4_power_destroy(std::ptr::null_mut());
    }
}

#[test]
fn init_registers_and_creates_the_game_grid() {
    let g = Game::default();
    let p = started(&g);
    let names: Vec<(u32, u32)> = g.agents.borrow().iter().map(|(l, _, f)| (*l, *f)).collect();
    assert_eq!(names, vec![(6, 2), (4, 2), (10, 2)]);
    assert!(g.agents.borrow().iter().all(|(_, n, _)| n == "cSC4PowerSimulator"));
    assert_eq!(*g.subs.borrow(), msg::SUBSCRIPTIONS.to_vec());
    assert_eq!(g.services.get(), 1);
    let grid = unsafe { fc4_power_powered_grid(p) };
    assert_ne!(grid, 0);
    assert_eq!(g.grids.borrow()[&grid].instance, GRID_INSTANCE);
    assert_eq!(unsafe { fc4_power_department(p) }, DEPT);
    assert_eq!(g.refs_of(DEPT), 1);

    assert!(unsafe { fc4_power_shutdown(p) });
    assert_eq!(g.services.get(), 0);
    assert!(g.agents.borrow().is_empty() && g.subs.borrow().is_empty());
    assert_eq!(g.refs_of(DEPT), 0);
    let gr = &g.grids.borrow()[&grid];
    assert!(gr.shut && gr.refs == 0, "Shutdown shuts the grid down and releases it");
    unsafe { fc4_power_destroy(p) };
}

#[test]
fn a_plant_powers_its_consumer_through_the_game_grid() {
    let g = Game::default();
    let p = started(&g);
    message(p, msg::OCCUPANT_INSERTED, PLANT);
    message(p, msg::OCCUPANT_INSERTED, CONSUMER);
    assert_eq!(g.refs_of(PLANT), 1, "the plant list holds one reference");
    unsafe {
        assert_eq!(fc4_power_plant_efficiency(p, PLANT), 1.0);
        assert_eq!(fc4_power_plant_condition(p, PLANT), 1.0);
    }
    cycle(p);
    let st = state_of(p);
    assert_eq!((st.last_production, st.last_consumption), (1000, 500));
    assert_eq!((st.powered, st.unpowered, st.consumers, st.plant_count), (1, 0, 1, 1));
    unsafe {
        assert!(fc4_power_is_powered(p, 13, 1) && fc4_power_is_wired(p, 13, 1));
        assert!(!fc4_power_is_powered(p, 20, 1));
        assert_eq!(fc4_power_slot_value(p, 24), 1);
        assert_eq!(fc4_power_slot_value(p, 12), 1000);
        assert_eq!(fc4_power_plant_count_of_type(p, 1), 1);
        let (mut prod, mut used) = (0, 0);
        assert!(fc4_power_occupant_produced(p, PLANT, &mut prod, &mut used));
        assert_eq!((prod, used), (1000, 500));
        let w = &g.written.borrow()[&(PLANT, prop::USED)];
        assert_eq!(
            (w.0, w.1, w.3),
            (mem::SINT32, false, 1),
            "USED crosses as the core's int32 scalar"
        );
        let mut cons = 0;
        assert!(fc4_power_occupant_consumed(p, CONSUMER, &mut cons));
        assert_eq!(cons, 500);
        assert!(!fc4_power_occupant_consumed(p, 99, &mut cons));
    }
    let grid = st.grid;
    assert_eq!(g.grids.borrow()[&grid].data[(13 * CELLS + 1) as usize], 2);
    assert!(g.posts.borrow().iter().any(|m| m.msg_type == msg::POWER_BLOCK_CHANGED));

    // GetPowerPlants hands out one reference per plant.
    let mut got: Vec<u64> = Vec::new();
    unsafe extern "C" fn push(ctx: *mut c_void, t: u64) {
        (*(ctx as *mut Vec<u64>)).push(t);
    }
    let n = unsafe { fc4_power_plants(p, Some(push), &mut got as *mut Vec<u64> as *mut c_void) };
    assert_eq!((n, got.clone()), (1, vec![PLANT]));
    assert_eq!(g.refs_of(PLANT), 2);
    unsafe { release(&g as *const Game as *mut c_void, PLANT) };
    // Without a callback the references go straight back.
    assert_eq!(unsafe { fc4_power_plants(p, None, std::ptr::null_mut()) }, 1);
    assert_eq!(g.refs_of(PLANT), 1);

    // The recompute report: off by default; switched on, the next cycle's
    // text (a removal and reinsertion asks for the recompute).
    unsafe extern "C" fn text(ctx: *mut c_void, data: *const u8, len: usize) {
        (*(ctx as *mut Vec<u8>)).extend_from_slice(std::slice::from_raw_parts(data, len));
    }
    let mut t: Vec<u8> = Vec::new();
    let ctx = &mut t as *mut Vec<u8> as *mut c_void;
    unsafe {
        assert!(!fc4_power_take_report(p, Some(text), ctx));
        fc4_power_set_report(p, true);
    }
    message(p, msg::OCCUPANT_REMOVED, CONSUMER);
    message(p, msg::OCCUPANT_INSERTED, CONSUMER);
    cycle(p);
    unsafe {
        assert!(fc4_power_take_report(p, Some(text), ctx));
        assert!(!fc4_power_take_report(p, Some(text), ctx));
        assert!(!fc4_power_take_report(std::ptr::null(), None, std::ptr::null_mut()));
        fc4_power_set_report(p, false);
    }
    let t = String::from_utf8(t).unwrap();
    assert!(
        t.starts_with("power recompute 1: ") && t.contains("production 1000"),
        "{t}"
    );

    unsafe {
        fc4_power_shutdown(p);
        fc4_power_destroy(p);
    }
    assert_eq!(g.refs_of(PLANT), 0);
}

#[test]
fn messages_during_a_call_are_queued() {
    let g = Game::default();
    let p = started(&g);
    message(p, msg::OCCUPANT_INSERTED, PLANT);
    g.power.set(p as usize);
    // The cycle posts; the fake game delivers OCCUPANT_INSERTED from inside.
    cycle(p);
    assert_eq!(g.reenter.get(), 1);
    let st = state_of(p);
    assert_eq!(st.consumers, 1, "the queued insertion was handled after the call");
    g.power.set(0);
    unsafe {
        fc4_power_shutdown(p);
        fc4_power_destroy(p);
    }
}

#[test]
fn record_round_trip_through_host_streams() {
    let g = Game::default();
    let p = started(&g);
    message(p, msg::OCCUPANT_INSERTED, PLANT);
    message(p, msg::OCCUPANT_INSERTED, CONSUMER);
    cycle(p);
    let before = state_of(p);

    let mut out = Out::default();
    let os = ostream(&mut out);
    assert!(unsafe { fc4_power_write(p, &os) });
    assert!(out.refs.contains(&(DEPT, 0)) && out.refs.contains(&(PLANT, 0)));
    assert!(out.refs.contains(&(before.grid, GRID_CLSID)));

    // The reader takes the references the stream hands over.
    let h = host(&g);
    let q = unsafe { fc4_power_create(&h, 0x52) };
    let gp = &g as *const Game;
    let take = move |r: &mut crate::stream::fc4_ref| unsafe {
        if r.token != 0 {
            if let Some(gr) = (*gp).grids.borrow_mut().get_mut(&r.token) {
                gr.refs += 1;
                return;
            }
            add_ref(gp as *mut c_void, r.token);
        }
    };
    let mut inp = In::new(&out.bytes);
    inp.resolve = Some(&take);
    let is = istream(&mut inp);
    assert!(unsafe { fc4_power_read(q, &is) });
    let after = state_of(q);
    assert_eq!(after.running, 1);
    assert_eq!(
        (
            after.plant_count,
            after.consumers,
            after.powered,
            after.last_production,
            after.cost_base
        ),
        (
            before.plant_count,
            before.consumers,
            before.powered,
            before.last_production,
            before.cost_base
        )
    );
    assert_eq!((after.department, after.grid), (DEPT, before.grid));
    assert_eq!(g.refs_of(PLANT), 2);
    assert!(unsafe { fc4_power_is_powered(q, 13, 1) });

    // Destroying a loaded, never-initialised object gives its references back.
    unsafe { fc4_power_destroy(q) };
    assert_eq!(g.refs_of(PLANT), 1);
    assert_eq!(g.refs_of(DEPT), 1);
    assert_eq!(g.grids.borrow()[&before.grid].refs, 1);

    // A version above 6 fails.
    let bad = 7u16.to_le_bytes();
    let q = unsafe { fc4_power_create(&h, 0x53) };
    let mut inp = In::new(&bad);
    let is = istream(&mut inp);
    assert!(!unsafe { fc4_power_read(q, &is) });
    unsafe {
        fc4_power_destroy(q);
        fc4_power_shutdown(p);
        fc4_power_destroy(p);
    }
    assert_eq!(g.refs_of(PLANT), 0);
    assert_eq!(g.grids.borrow()[&before.grid].refs, 0);
}
