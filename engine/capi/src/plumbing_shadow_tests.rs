// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The plumbing shadow (plumbing_shadow.rs) on one fake game, with
//! FreeCity4's own plumbing simulator (fc4_plumbing) standing in for the
//! original: a producer, two consumers, 100 pipes, an import deal over one
//! water connection.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::c_void;

use fc4_core::plumbing::{deal, msg, occupant_type, prop, Tuning};
use fc4_core::variant::mem;

use crate::budget::fc4_token_callback;
use crate::hostgrid::{fc4_cell_rect, fc4_grid_api};
use crate::landvalue::fc4_grid_geometry;
use crate::plumbing::*;
use crate::plumbing_shadow::*;
use crate::props::{fc4_props, fc4_props_set_typed};
use crate::stream::{fc4_ostream, fc4_ref};

const CELLS: i32 = 32;
const U8_GRID: u32 = 0x49B9_E602;
const PRODUCER: u64 = 0x100;
const CONSUMER: u64 = 0x200;
const CONSUMER2: u64 = 0x201;
const PIPE0: u64 = 0x1000;
const PIPES: u64 = 100;
const LOT_P: u64 = 0x500;
const LOT_C: u64 = 0x501;
const LOT_C2: u64 = 0x502;
const DEPT: u64 = 0xD0;
const IMPORT_DEAL: u64 = 0xDE;
const CITY2: u64 = 0xC2;
const CONN: u64 = 0xCC;
const CONN_OTHER: u64 = 0xCD;
/// 2000-01-01.
const JULIAN: i32 = 2_451_545;

struct Grid {
    data: Vec<u8>,
    refs: i32,
    shut: bool,
}

/// (element type, array flag, bytes, count).
type Written = (u16, bool, Vec<u8>, usize);

/// The original's record as the plugin's capture stream produces it.
#[derive(Default)]
struct Capture {
    world: usize,
    bytes: Vec<u8>,
    refs: Vec<fc4_plumbing_capture_ref>,
}

#[derive(Default)]
struct World {
    grids: RefCell<BTreeMap<u64, Grid>>,
    next_grid: Cell<u64>,
    refs: RefCell<BTreeMap<u64, i32>>,
    props: RefCell<BTreeMap<(u64, u32), Written>>,
    net_flags: RefCell<BTreeMap<u64, u32>>,
    deal_used: RefCell<BTreeMap<u64, i32>>,
    deal_flags: RefCell<BTreeMap<u64, (u32, bool)>>,
    visited: RefCell<BTreeMap<u64, bool>>,
    active: RefCell<BTreeMap<u64, bool>>,
    watered_pipes: RefCell<BTreeMap<u64, bool>>,
    item_cost: Cell<i64>,
    funding: Cell<f32>,
    posts: RefCell<Vec<(u32, u32)>>,
    history: RefCell<Vec<(u32, u32)>>,
    lot_conditions: RefCell<Vec<(u64, u8)>>,
    redisplays: Cell<u32>,
    bursts: Cell<u32>,
    dept: Cell<u64>,
    /// The stand-in original.
    a: Cell<usize>,
    capture: RefCell<Box<Capture>>,
    fail_capture: Cell<bool>,
}

fn world(u: *mut c_void) -> &'static World {
    unsafe { &*(u as *const World) }
}

impl World {
    fn new() -> Box<World> {
        let w = Box::<World>::default();
        w.item_cost.set(100);
        w.funding.set(1.0);
        w.set(PRODUCER, prop::CAPACITY, mem::UINT32, false, &[1000u32]);
        w.set(PRODUCER, prop::SOURCE_TYPE, mem::UINT8, false, &[2u8]);
        w.set(PRODUCER, prop::COST_PER_UNIT, mem::FLOAT32, false, &[0.5f32]);
        w.set(
            PRODUCER,
            prop::MAINTENANCE_CURVE,
            mem::FLOAT32,
            true,
            &[0.0f32, 1.0, 100.0, 2.0],
        );
        w.set(PRODUCER, prop::AGING_RATE, mem::FLOAT32, false, &[2.0f32]);
        w.set(PRODUCER, prop::AGING_T1, mem::FLOAT32, false, &[50.0f32]);
        w.set(PRODUCER, prop::AGING_T2, mem::FLOAT32, false, &[80.0f32]);
        w.set(CONSUMER, prop::CONSUMPTION, mem::SINT32, false, &[100i32]);
        w.set(CONSUMER2, prop::CONSUMPTION, mem::SINT32, false, &[30i32]);
        w.active.borrow_mut().insert(CONN_OTHER, true);
        w
    }

    fn set<T: Copy>(&self, occ: u64, id: u32, t: u16, array: bool, v: &[T]) {
        let bytes = unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }.to_vec();
        self.props.borrow_mut().insert((occ, id), (t, array, bytes, v.len()));
    }

    fn user(&self) -> *mut c_void {
        self as *const World as *mut c_void
    }

    fn a(&self) -> *mut fc4_plumbing {
        self.a.get() as *mut fc4_plumbing
    }

    /// Everything the shadow could change by mistake.
    fn fingerprint(&self) -> String {
        let grids: Vec<(u64, Vec<u8>, i32, bool)> = self
            .grids
            .borrow()
            .iter()
            .map(|(t, g)| (*t, g.data.clone(), g.refs, g.shut))
            .collect();
        format!(
            "{grids:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {:?} {} {} {:?} {:?} {:?} {} {}",
            self.refs.borrow(),
            self.props.borrow(),
            self.net_flags.borrow(),
            self.deal_used.borrow(),
            self.deal_flags.borrow(),
            self.visited.borrow(),
            self.active.borrow(),
            self.watered_pipes.borrow(),
            self.item_cost.get(),
            self.funding.get(),
            self.posts.borrow(),
            self.history.borrow(),
            self.lot_conditions.borrow(),
            self.redisplays.get(),
            self.bursts.get()
        )
    }
}

fn is_pipe(occ: u64) -> bool {
    (PIPE0..PIPE0 + PIPES).contains(&occ)
}

/// The cells an occupant covers (inclusive).
fn rect_of(occ: u64) -> Option<(i32, i32, i32, i32)> {
    match occ {
        PRODUCER => Some((3, 3, 5, 5)),
        CONSUMER => Some((6, 4, 6, 4)),
        CONSUMER2 => Some((5, 6, 5, 6)),
        p if is_pipe(p) => {
            let i = (p - PIPE0) as i32;
            Some((i % CELLS, 20 + i / CELLS, i % CELLS, 20 + i / CELLS))
        }
        _ => None,
    }
}

fn anchor(occ: u64) -> (i32, i32) {
    match occ {
        PRODUCER => (4, 4),
        _ => rect_of(occ).map_or((0, 0), |r| (r.0, r.1)),
    }
}

fn lot_of(occ: u64) -> u64 {
    match occ {
        PRODUCER => LOT_P,
        CONSUMER => LOT_C,
        CONSUMER2 => LOT_C2,
        _ => 0,
    }
}

fn lot_rect_of(lot: u64) -> Option<(i32, i32, i32, i32)> {
    match lot {
        LOT_P => rect_of(PRODUCER),
        LOT_C => rect_of(CONSUMER),
        LOT_C2 => rect_of(CONSUMER2),
        _ => None,
    }
}

fn inside(r: (i32, i32, i32, i32), x: i32, z: i32) -> bool {
    x >= r.0 && z >= r.1 && x <= r.2 && z <= r.3
}

// ---- grids --------------------------------------------------------------------------------------------

unsafe extern "C" fn g_create(u: *mut c_void, clsid: u32, _instance: u32, ts: i32) -> u64 {
    assert_eq!(clsid, U8_GRID);
    assert_eq!(ts, 1);
    let w = world(u);
    let t = w.next_grid.get() + 0x7000;
    w.next_grid.set(w.next_grid.get() + 0x10);
    w.grids.borrow_mut().insert(
        t,
        Grid {
            data: vec![0; (CELLS * CELLS) as usize],
            refs: 1,
            shut: false,
        },
    );
    t
}
unsafe extern "C" fn g_class(u: *mut c_void, t: u64) -> u32 {
    if world(u).grids.borrow().contains_key(&t) {
        U8_GRID
    } else {
        0
    }
}
unsafe extern "C" fn g_geometry(_u: *mut c_void, _t: u64, out: *mut fc4_grid_geometry) {
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
    assert_eq!(kind, U8_GRID);
    let grids = world(u).grids.borrow();
    let g = &grids[&t];
    if x < 0 || z < 0 || x >= CELLS || z >= CELLS {
        return 0.0;
    }
    f64::from(g.data[(x * CELLS + z) as usize])
}
unsafe extern "C" fn g_set(u: *mut c_void, t: u64, _kind: u32, x: i32, z: i32, v: f64) {
    let mut grids = world(u).grids.borrow_mut();
    let g = grids.get_mut(&t).unwrap();
    if x >= 0 && z >= 0 && x < CELLS && z < CELLS {
        g.data[(x * CELLS + z) as usize] = v as u8;
    }
}
unsafe extern "C" fn g_fill(u: *mut c_void, t: u64, _kind: u32, v: f64) {
    world(u).grids.borrow_mut().get_mut(&t).unwrap().data.fill(v as u8);
}
unsafe extern "C" fn g_shutdown(u: *mut c_void, t: u64) {
    world(u).grids.borrow_mut().get_mut(&t).unwrap().shut = true;
}
unsafe extern "C" fn g_release(u: *mut c_void, t: u64) {
    world(u).grids.borrow_mut().get_mut(&t).unwrap().refs -= 1;
}

fn set_cell(w: &World, grid: u64, x: i32, z: i32, v: u8) {
    w.grids.borrow_mut().get_mut(&grid).unwrap().data[(x * CELLS + z) as usize] = v;
}

fn cell(w: &World, grid: u64, x: i32, z: i32) -> u8 {
    w.grids.borrow()[&grid].data[(x * CELLS + z) as usize]
}

// ---- city, simulator, history -------------------------------------------------------------------------

unsafe extern "C" fn cell_counts(_: *mut c_void, x: *mut i32, z: *mut i32) {
    *x = CELLS;
    *z = CELLS;
}
unsafe extern "C" fn position_to_cell(_: *mut c_void, x: f32, z: f32, cx: *mut i32, cz: *mut i32) {
    *cx = (x / 16.0).floor() as i32;
    *cz = (z / 16.0).floor() as i32;
}
unsafe extern "C" fn tuning(_: *mut c_void) -> *mut fc4_props {
    Box::into_raw(Box::new(Tuning::shipped_exemplar())) as *mut fc4_props
}
unsafe extern "C" fn session_seed(_: *mut c_void) -> u32 {
    0x1234_5678
}
unsafe extern "C" fn post(u: *mut c_void, m: u32, d: u32) {
    world(u).posts.borrow_mut().push((m, d));
}
unsafe extern "C" fn record_history(u: *mut c_void, id: u32, v: u32) {
    world(u).history.borrow_mut().push((id, v));
}

// ---- occupants ------------------------------------------------------------------------------------------

unsafe extern "C" fn occupant_type(_: *mut c_void, occ: u64) -> u32 {
    if is_pipe(occ) {
        occupant_type::NETWORK
    } else {
        occupant_type::BUILDING
    }
}
unsafe extern "C" fn occupant_property(u: *mut c_void, occ: u64, id: u32, p: *mut fc4_props) -> bool {
    match world(u).props.borrow().get(&(occ, id)) {
        Some((t, a, b, n)) => fc4_props_set_typed(p, id, *t, *a, b.as_ptr() as *const c_void, *n),
        None => false,
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
        mem::BOOL | mem::UINT8 | mem::SINT8 => 1,
        mem::UINT16 | mem::SINT16 => 2,
        mem::UINT64 | mem::SINT64 | mem::FLOAT64 => 8,
        _ => 4,
    };
    let bytes = std::slice::from_raw_parts(data as *const u8, size * count).to_vec();
    world(u)
        .props
        .borrow_mut()
        .insert((occ, id), (mem_type, is_array, bytes, count));
}
unsafe extern "C" fn occupant_position(_: *mut c_void, occ: u64, out: *mut f32) {
    let (x, z) = anchor(occ);
    let p = [(x as f32 + 0.5) * 16.0, 0.0, (z as f32 + 0.5) * 16.0];
    std::slice::from_raw_parts_mut(out, 3).copy_from_slice(&p);
}
unsafe extern "C" fn occupant_bounding_box(_: *mut c_void, occ: u64, a: *mut f32, b: *mut f32) {
    let r = rect_of(occ).unwrap_or((0, 0, 0, 0));
    let lo = [r.0 as f32 * 16.0 + 0.5, 0.0, r.1 as f32 * 16.0 + 0.5];
    let hi = [(r.2 + 1) as f32 * 16.0 - 0.5, 10.0, (r.3 + 1) as f32 * 16.0 - 0.5];
    std::slice::from_raw_parts_mut(a, 3).copy_from_slice(&lo);
    std::slice::from_raw_parts_mut(b, 3).copy_from_slice(&hi);
}
unsafe extern "C" fn occupant_bounding_cells(_: *mut c_void, occ: u64, out: *mut fc4_cell_rect) {
    let r = rect_of(occ).unwrap_or((0, 0, 0, 0));
    *out = fc4_cell_rect {
        x1: r.0,
        z1: r.1,
        x2: r.2,
        z2: r.3,
    };
}
unsafe extern "C" fn building_type(_: *mut c_void, occ: u64, out: *mut u32) -> bool {
    if is_pipe(occ) {
        return false;
    }
    *out = 1;
    true
}
unsafe extern "C" fn buildings_in_cell(_: *mut c_void, x: i32, z: i32, cb: fc4_token_callback, ctx: *mut c_void) {
    for occ in [PRODUCER, CONSUMER, CONSUMER2] {
        if rect_of(occ).is_some_and(|r| inside(r, x, z)) {
            cb.unwrap()(ctx, occ);
        }
    }
}

// ---- pipes ------------------------------------------------------------------------------------------------

unsafe extern "C" fn is_network_occupant(_: *mut c_void, occ: u64) -> bool {
    is_pipe(occ)
}
unsafe extern "C" fn has_network_flag(u: *mut c_void, occ: u64, f: u32) -> bool {
    world(u).net_flags.borrow().get(&occ).copied().unwrap_or(0) & f != 0
}
unsafe extern "C" fn network_flags(u: *mut c_void, occ: u64) -> u32 {
    world(u).net_flags.borrow().get(&occ).copied().unwrap_or(0)
}
unsafe extern "C" fn set_network_flag(u: *mut c_void, occ: u64, f: u32) {
    *world(u).net_flags.borrow_mut().entry(occ).or_insert(0) |= f;
}
unsafe extern "C" fn clear_network_flag(u: *mut c_void, occ: u64, f: u32) {
    *world(u).net_flags.borrow_mut().entry(occ).or_insert(0) &= !f;
}
unsafe extern "C" fn occupied_cell(_: *mut c_void, occ: u64, x: *mut i32, z: *mut i32) {
    let (a, b) = anchor(occ);
    *x = a;
    *z = b;
}
unsafe extern "C" fn set_pipe_watered(u: *mut c_void, occ: u64, on: bool) {
    world(u).watered_pipes.borrow_mut().insert(occ, on);
}

// ---- lots ------------------------------------------------------------------------------------------------

unsafe extern "C" fn lot_at(_: *mut c_void, x: i32, z: i32) -> u64 {
    [LOT_P, LOT_C, LOT_C2]
        .into_iter()
        .find(|&l| lot_rect_of(l).is_some_and(|r| inside(r, x, z)))
        .unwrap_or(0)
}
unsafe extern "C" fn occupant_lot(_: *mut c_void, occ: u64) -> u64 {
    lot_of(occ)
}
unsafe extern "C" fn lot_rect(_: *mut c_void, lot: u64, out: *mut fc4_cell_rect) {
    let r = lot_rect_of(lot).unwrap_or((0, 0, 0, 0));
    *out = fc4_cell_rect {
        x1: r.0,
        z1: r.1,
        x2: r.2,
        z2: r.3,
    };
}
unsafe extern "C" fn lot_has_power(_: *mut c_void, _lot: u64) -> bool {
    true
}
unsafe extern "C" fn set_lot_condition(u: *mut c_void, lot: u64, c: u8) {
    world(u).lot_conditions.borrow_mut().push((lot, c));
}
unsafe extern "C" fn redisplay_terrain(u: *mut c_void, _a: i32, _b: i32, _c: i32, _d: i32) {
    world(u).redisplays.set(world(u).redisplays.get() + 1);
}
unsafe extern "C" fn attempt_pipe_burst(u: *mut c_void, _x: i32, _z: i32) -> bool {
    world(u).bursts.set(world(u).bursts.get() + 1);
    true
}

// ---- neighbours ------------------------------------------------------------------------------------------

unsafe extern "C" fn find_deals(_: *mut c_void, t: u32, cb: fc4_token_callback, ctx: *mut c_void) {
    if t == deal::IMPORT {
        cb.unwrap()(ctx, IMPORT_DEAL);
    }
}
unsafe extern "C" fn deal_type(_: *mut c_void, _d: u64) -> u32 {
    deal::IMPORT
}
unsafe extern "C" fn deal_city(_: *mut c_void, _d: u64) -> u64 {
    CITY2
}
unsafe extern "C" fn deal_amount(_: *mut c_void, _d: u64) -> i32 {
    500
}
unsafe extern "C" fn deal_amount_used(u: *mut c_void, d: u64) -> i32 {
    world(u).deal_used.borrow().get(&d).copied().unwrap_or(0)
}
unsafe extern "C" fn set_deal_amount_used(u: *mut c_void, d: u64, v: i32) {
    world(u).deal_used.borrow_mut().insert(d, v);
}
unsafe extern "C" fn set_deal_flags(u: *mut c_void, d: u64, f: u32, on: bool) {
    world(u).deal_flags.borrow_mut().insert(d, (f, on));
}
unsafe extern "C" fn connections_of_type(_: *mut c_void, t: u32, cb: fc4_token_callback, ctx: *mut c_void) {
    if t == deal::WATER_CONNECTION {
        cb.unwrap()(ctx, CONN);
    }
}
unsafe extern "C" fn connections_of_type_to_city(
    _: *mut c_void,
    t: u32,
    city: u64,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) {
    if t == deal::WATER_CONNECTION && city == CITY2 {
        cb.unwrap()(ctx, CONN);
    }
}
unsafe extern "C" fn find_unvisited_connection(u: *mut c_void, t: u32, city: u64) -> u64 {
    let visited = world(u).visited.borrow().get(&CONN).copied().unwrap_or(false);
    if t == deal::WATER_CONNECTION && city == CITY2 && !visited {
        CONN
    } else {
        0
    }
}
unsafe extern "C" fn connection_on_other_side(_: *mut c_void, c: u64) -> u64 {
    if c == CONN {
        CONN_OTHER
    } else {
        0
    }
}
unsafe extern "C" fn connection_cell(_: *mut c_void, _c: u64, x: *mut i32, z: *mut i32) {
    *x = 31;
    *z = 31;
}
unsafe extern "C" fn set_connection_visited(u: *mut c_void, c: u64, v: bool) {
    world(u).visited.borrow_mut().insert(c, v);
}
unsafe extern "C" fn connection_is_active(u: *mut c_void, c: u64) -> bool {
    world(u).active.borrow().get(&c).copied().unwrap_or(false)
}
unsafe extern "C" fn set_connection_active(u: *mut c_void, c: u64, v: bool) {
    world(u).active.borrow_mut().insert(c, v);
}

// ---- budget ------------------------------------------------------------------------------------------------

unsafe extern "C" fn create_department(u: *mut c_void, _id: u32, _group: u32) -> u64 {
    world(u).dept.set(DEPT);
    DEPT
}
unsafe extern "C" fn department_funding(u: *mut c_void, _d: u64) -> f32 {
    world(u).funding.get()
}
unsafe extern "C" fn department_by_id(u: *mut c_void, _id: u32) -> u64 {
    world(u).dept.get()
}
unsafe extern "C" fn budget_item_for_purpose(
    u: *mut c_void,
    occ: u64,
    _purpose: u32,
    out: *mut fc4_plumbing_budget_item,
) -> bool {
    if occ != PRODUCER {
        return false;
    }
    *out = fc4_plumbing_budget_item {
        department: 0xAA40_14B4,
        line: 7,
        cost: world(u).item_cost.get(),
        index: 0,
        reserved: 0,
    };
    true
}
unsafe extern "C" fn change_budget_item_cost(u: *mut c_void, _occ: u64, _index: u32, cost: i64) {
    world(u).item_cost.set(cost);
}

// ---- references --------------------------------------------------------------------------------------------

unsafe extern "C" fn add_ref(u: *mut c_void, t: u64) {
    *world(u).refs.borrow_mut().entry(t).or_insert(0) += 1;
}
unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    *world(u).refs.borrow_mut().entry(t).or_insert(0) -= 1;
}
unsafe extern "C" fn resolve_ref(_: *mut c_void, t: u64, _iid: u32) -> u64 {
    t
}

fn host(w: &World) -> fc4_plumbing_host {
    let mut h: fc4_plumbing_host = unsafe { std::mem::zeroed() };
    h.struct_size = std::mem::size_of::<fc4_plumbing_host>() as u32;
    h.user = w.user();
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
    h.position_to_cell = Some(position_to_cell);
    h.tuning = Some(tuning);
    h.session_seed = Some(session_seed);
    h.post = Some(post);
    h.record_history = Some(record_history);
    h.occupant_type = Some(occupant_type);
    h.occupant_property = Some(occupant_property);
    h.set_occupant_property = Some(set_occupant_property);
    h.occupant_position = Some(occupant_position);
    h.occupant_bounding_box = Some(occupant_bounding_box);
    h.occupant_bounding_cells = Some(occupant_bounding_cells);
    h.building_type = Some(building_type);
    h.buildings_in_cell = Some(buildings_in_cell);
    h.is_network_occupant = Some(is_network_occupant);
    h.has_network_flag = Some(has_network_flag);
    h.network_flags = Some(network_flags);
    h.set_network_flag = Some(set_network_flag);
    h.clear_network_flag = Some(clear_network_flag);
    h.occupied_cell = Some(occupied_cell);
    h.set_pipe_watered = Some(set_pipe_watered);
    h.lot_at = Some(lot_at);
    h.occupant_lot = Some(occupant_lot);
    h.lot_rect = Some(lot_rect);
    h.lot_has_power = Some(lot_has_power);
    h.set_lot_condition = Some(set_lot_condition);
    h.redisplay_terrain = Some(redisplay_terrain);
    h.attempt_pipe_burst = Some(attempt_pipe_burst);
    h.find_deals = Some(find_deals);
    h.deal_type = Some(deal_type);
    h.deal_city = Some(deal_city);
    h.deal_amount = Some(deal_amount);
    h.deal_amount_used = Some(deal_amount_used);
    h.set_deal_amount_used = Some(set_deal_amount_used);
    h.set_deal_flags = Some(set_deal_flags);
    h.connections_of_type = Some(connections_of_type);
    h.connections_of_type_to_city = Some(connections_of_type_to_city);
    h.find_unvisited_connection = Some(find_unvisited_connection);
    h.connection_on_other_side = Some(connection_on_other_side);
    h.connection_cell = Some(connection_cell);
    h.set_connection_visited = Some(set_connection_visited);
    h.connection_is_active = Some(connection_is_active);
    h.set_connection_active = Some(set_connection_active);
    h.create_department = Some(create_department);
    h.department_funding = Some(department_funding);
    h.department_by_id = Some(department_by_id);
    h.budget_item_for_purpose = Some(budget_item_for_purpose);
    h.change_budget_item_cost = Some(change_budget_item_cost);
    h.add_ref = Some(add_ref);
    h.release = Some(release);
    h.resolve_ref = Some(resolve_ref);
    h
}

// ---- the original, as the plugin reads it ------------------------------------------------------------------

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
    let w = world(c.world as *mut c_void);
    let is_grid = w.grids.borrow().contains_key(&r.token);
    c.refs.push(fc4_plumbing_capture_ref {
        grid: if is_grid { r.token } else { 0 },
        occupant: if !is_grid && r.token != DEPT { r.token } else { 0 },
        network: if is_pipe(r.token) { r.token } else { 0 },
        department: if r.token == DEPT { r.token } else { 0 },
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

unsafe extern "C" fn o_capture(u: *mut c_void, out: *mut fc4_plumbing_capture) -> bool {
    let w = world(u);
    if w.fail_capture.get() {
        return false;
    }
    let mut c = Box::new(Capture {
        world: u as usize,
        ..Capture::default()
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
    let ok = fc4_plumbing_write(w.a(), &os);
    *out = fc4_plumbing_capture {
        struct_size: std::mem::size_of::<fc4_plumbing_capture>() as u32,
        ok: u32::from(ok),
        bytes: c.bytes.as_ptr(),
        len: c.bytes.len(),
        refs: c.refs.as_ptr(),
        ref_count: c.refs.len(),
    };
    *w.capture.borrow_mut() = c; // the view stays valid until the next call
    ok
}
unsafe extern "C" fn o_watered_grid(u: *mut c_void) -> u64 {
    fc4_plumbing_watered_grid(world(u).a())
}
unsafe extern "C" fn o_slot(u: *mut c_void, slot: u32) -> i64 {
    let v = fc4_plumbing_slot_value(world(u).a(), slot);
    // The EAX the game's slot returns.
    match slot {
        14..=17 | 23 => i64::from(v as i32),
        _ => i64::from(v as u32),
    }
}
unsafe extern "C" fn o_producer_info(u: *mut c_void, occ: u64, out: *mut fc4_plumbing_producer_info) -> bool {
    fc4_plumbing_get_producer_info(world(u).a(), occ, out)
}
unsafe extern "C" fn o_visited(u: *mut c_void, c: u64) -> bool {
    world(u).visited.borrow().get(&c).copied().unwrap_or(false)
}
/// The plugin reads passCounter through the original's Write (PS6); the
/// stand-in's is its state's.
unsafe extern "C" fn o_pass_counter(u: *mut c_void, out: *mut i32) -> bool {
    *out = state_of(world(u)).pass_counter;
    true
}

fn original(w: &World) -> fc4_plumbing_original {
    fc4_plumbing_original {
        struct_size: std::mem::size_of::<fc4_plumbing_original>() as u32,
        user: w.user(),
        watered_grid: Some(o_watered_grid),
        slot_value: Some(o_slot),
        producer_info: Some(o_producer_info),
        connection_visited: Some(o_visited),
        capture: Some(o_capture),
        pass_counter: Some(o_pass_counter),
    }
}

// ---- driving ----------------------------------------------------------------------------------------------

fn message(w: &World, msg_type: u32, data1: u32, data2: u32, occupant: u64) {
    let m = fc4_plumbing_message {
        msg_type,
        data1,
        data2,
        data3: 5,
        occupant,
        ..Default::default()
    };
    unsafe { fc4_plumbing_do_message(w.a(), &m) };
}

fn state_of(w: &World) -> fc4_plumbing_state {
    let mut s = fc4_plumbing_state {
        struct_size: std::mem::size_of::<fc4_plumbing_state>() as u32,
        ..Default::default()
    };
    assert!(unsafe { fc4_plumbing_get_state(w.a(), &mut s) });
    s
}

/// The stand-in original: initialised, enabled, with its buildings and pipes.
fn started() -> Box<World> {
    let w = World::new();
    let h = host(&w);
    let a = unsafe { fc4_plumbing_create(&h, 0x51) };
    assert!(!a.is_null());
    w.a.set(a as usize);
    assert!(unsafe { fc4_plumbing_init(a) });
    message(&w, msg::CONNECTIONS_READY, 0, 0, 0);
    for occ in [PRODUCER, CONSUMER, CONSUMER2] {
        message(&w, msg::INSERT_OCCUPANT, 0, 0, occ);
    }
    for p in PIPE0..PIPE0 + PIPES {
        message(&w, msg::INSERT_OCCUPANT, 0, 0, p);
    }
    w
}

fn stop(w: &World) {
    unsafe {
        assert!(fc4_plumbing_shutdown(w.a()));
        fc4_plumbing_destroy(w.a());
    }
}

unsafe extern "C" fn lines_sink(ctx: *mut c_void, data: *const u8, len: usize) {
    let s = String::from_utf8(std::slice::from_raw_parts(data, len).to_vec()).unwrap();
    (*(ctx as *mut Vec<String>)).push(s);
}

/// One shadowed delivery: `pre` runs before the shadow's before, `meddle`
/// after the original's delivery (before the shadow's after). The shadow
/// itself must leave the game unchanged.
fn deliver(
    w: &World,
    s: *mut fc4_plumbing_shadow,
    msg_type: u32,
    julian: i32,
    lines: &mut Vec<String>,
    pre: impl FnOnce(),
    meddle: impl FnOnce(),
) -> i32 {
    pre();
    let fp = w.fingerprint();
    assert_eq!(unsafe { fc4_plumbing_shadow_before(s, msg_type, julian) }, 0);
    assert_eq!(fp, w.fingerprint(), "the shadow's before changed the game");
    message(w, msg_type, julian as u32, 0, 0);
    meddle();
    let fp = w.fingerprint();
    let r = unsafe { fc4_plumbing_shadow_after(s, Some(lines_sink), lines as *mut Vec<String> as *mut c_void) };
    assert_eq!(fp, w.fingerprint(), "the shadow's after changed the game");
    r
}

/// SimIdle deliveries until the original rests (state 9, no recompute); the
/// results of the deliveries that finished a comparison.
fn idle_until_rest(w: &World, s: *mut fc4_plumbing_shadow, julian: i32, lines: &mut Vec<String>) -> Vec<i32> {
    let mut results = Vec::new();
    for _ in 0..200 {
        let r = deliver(w, s, msg::SIM_IDLE, julian, lines, || {}, || {});
        if r >= 0 {
            results.push(r);
        }
        let st = state_of(w);
        if st.state == 9 && st.recompute == 0 {
            return results;
        }
    }
    panic!("the original never came to rest: {lines:#?}");
}

fn stats(s: *mut fc4_plumbing_shadow) -> fc4_plumbing_shadow_stats {
    let mut st = fc4_plumbing_shadow_stats::default();
    assert!(unsafe { fc4_plumbing_shadow_get_stats(s, &mut st) });
    st
}

/// A shadow made at the city start (PS3), reading passCounter (PS6).
fn shadow(w: &World, passes_per_day: u32) -> *mut fc4_plumbing_shadow {
    let s = shadow_with(w, passes_per_day, true);
    assert!(unsafe { fc4_plumbing_shadow_city_start(s) });
    s
}

/// A shadow made mid-city; `counter`: with the passCounter callback, else
/// the probe cell.
fn shadow_with(w: &World, passes_per_day: u32, counter: bool) -> *mut fc4_plumbing_shadow {
    let h = host(w);
    let mut o = original(w);
    if !counter {
        o.pass_counter = None;
    }
    let s = unsafe { fc4_plumbing_shadow_create(&h, &o, 0, passes_per_day) };
    assert!(!s.is_null());
    s
}

fn pass_lines(lines: &[String]) -> Vec<&String> {
    lines
        .iter()
        .filter(|l| l.contains(" pass ") && l.contains("mismatches="))
        .collect()
}

// ---- tests --------------------------------------------------------------------------------------------------

/// FreeCity4's simulator stands in for the original: the shadow, replaying
/// each pass on its records and live inputs, must agree with it everywhere,
/// through a month with underfunded pipes and the ageing pass after it.
#[test]
fn shadow_agrees_with_an_identical_original() {
    let w = started();
    let s = shadow(&w, 0);
    let mut lines: Vec<String> = Vec::new();
    // Two passes: Init's, then the recompute the inserts set.
    let r = idle_until_rest(&w, s, JULIAN, &mut lines);
    assert_eq!(r, vec![0, 0], "{lines:#?}");
    let passes = pass_lines(&lines);
    assert_eq!(passes.len(), 2, "{lines:#?}");
    // PS6: the first delivery's state 0 moved passCounter, so the first
    // pass is seen from its start too.
    assert!(
        passes[0].starts_with("plumbing shadow 2000-01-01 pass 1: aligned=start exact=1;"),
        "{}",
        passes[0]
    );
    assert!(passes[0].contains("demand orig/ours=130/130;"), "{}", passes[0]);
    let p2 = passes[1];
    assert!(
        p2.starts_with("plumbing shadow 2000-01-01 pass 2: aligned=start exact=1;"),
        "{p2}"
    );
    assert!(p2.contains("grid 32x32 cells=1024 differing=0 max|diff|=0;"), "{p2}");
    assert!(p2.contains("producers 1; production orig/ours=[1000/1000]"), "{p2}");
    assert!(p2.contains("consumed orig/ours=130/130;"), "{p2}");
    assert!(p2.contains("produced orig/ours=1000/1000;"), "{p2}");
    assert!(p2.contains("wateredCells orig/ours=2/2;"), "{p2}");
    assert!(p2.contains("unwateredCells orig/ours=0/0;"), "{p2}");
    assert!(p2.contains("deals compared=1 differing=0"), "{p2}");
    assert!(
        p2.contains("connections compared=1 active differing=0 visited compared=1 differing=0"),
        "{p2}"
    );
    assert!(p2.contains("maintenance compared=1 differing=0"), "{p2}");
    assert!(p2.contains("ageing=no"), "{p2}");
    assert!(p2.contains("mismatches=0;"), "{p2}");
    assert!(!p2.contains('!'), "{p2}");
    let st = stats(s);
    assert_eq!(
        (
            st.passes_seen,
            st.passes_compared,
            st.passes_clean,
            st.passes_not_exact,
            st.passes_incomplete
        ),
        (2, 2, 2, 0, 0),
        "{st:?}"
    );
    assert_eq!((st.cells, st.cell_mismatches, st.scalar_mismatches), (2048, 0, 0));

    // A month without funding: the pipes' condition worsens (§7.1).
    w.funding.set(0.0);
    let r = deliver(&w, s, msg::SIM_NEW_MONTH, JULIAN + 31, &mut lines, || {}, || {});
    assert_eq!(r, 0, "{lines:#?}");
    let month = lines.last().unwrap();
    assert!(
        month.starts_with(
            "plumbing shadow 2000-02-01 month: pipes=100 distressedBefore=0 burstBefore=0; distressed orig/ours=1/1;"
        ),
        "{month}"
    );
    assert!(month.contains("pipeCondition ours=0.0100"), "{month}");
    assert!(
        month.contains("funding=0.000; mismatches=0; months=1 clean=1"),
        "{month}"
    );
    assert!(
        month.contains("passes since the last month line: seen=2 compared=2 clean=2 notExact=0"),
        "{month}"
    );
    assert_eq!(state_of(&w).num_distressed, 1);

    // The month's recompute: a pass that ages the producer (§6.7 step 3).
    w.funding.set(1.0);
    let r = idle_until_rest(&w, s, JULIAN + 31, &mut lines);
    assert_eq!(r, vec![0], "{lines:#?}");
    let p3 = pass_lines(&lines).last().unwrap().to_string();
    assert!(p3.contains("pass 3: aligned=start exact=1;"), "{p3}");
    assert!(p3.contains("ageing=yes; condition orig/ours=["), "{p3}");
    assert!(p3.contains("mismatches=0;"), "{p3}");
    assert!(!p3.contains('!'), "{p3}");
    let st = stats(s);
    assert_eq!((st.months, st.months_clean, st.passes_clean), (1, 1, 3), "{st:?}");
    // The lines, for `--nocapture`.
    println!("{}", lines.join("\n"));

    unsafe { fc4_plumbing_shadow_destroy(s) };
    stop(&w);
    // The shadow never held a reference.
    assert_eq!(w.refs.borrow().get(&PRODUCER).copied().unwrap_or(0), 0);
}

/// A published cell the original computes differently, and a producer's
/// water used changed after the original's flood: both reported and counted.
/// A consumer whose consumption changes after the pass started but before the
/// flood agrees: our flood runs just before the original's (PS8).
#[test]
fn shadow_reports_a_wrong_cell_and_a_wrong_scalar() {
    let w = started();
    let s = shadow(&w, 0);
    let mut lines: Vec<String> = Vec::new();
    assert_eq!(idle_until_rest(&w, s, JULIAN, &mut lines), vec![0, 0], "{lines:#?}");

    // A recompute that changes nothing (a burst message on an empty cell).
    message(&w, msg::PIPE_BURST, 0, 0, 0);
    let mut results = Vec::new();
    let mut meddled = false;
    for _ in 0..100 {
        let before = unsafe { fc4_plumbing_watered_grid(w.a()) };
        let r = deliver(
            &w,
            s,
            msg::SIM_IDLE,
            JULIAN + 1,
            &mut lines,
            || {},
            || {
                let now = unsafe { fc4_plumbing_watered_grid(w.a()) };
                if now != before && !meddled {
                    meddled = true;
                    let v = cell(&w, now, 10, 10);
                    set_cell(&w, now, 10, 10, v ^ 0x01);
                }
            },
        );
        if r >= 0 {
            results.push(r);
        }
        let st = state_of(&w);
        if st.state == 9 && st.recompute == 0 {
            break;
        }
    }
    assert!(meddled);
    assert_eq!(results, vec![1], "{lines:#?}");
    let p = pass_lines(&lines).last().unwrap().to_string();
    assert!(
        p.contains("grid 32x32 cells=1024 differing=1 max|diff|=1 first (x,z) orig/ours: (10,10) "),
        "{p}"
    );
    assert!(p.contains("exact=1;") && p.contains("mismatches=1;"), "{p}");
    let st = stats(s);
    assert_eq!((st.cell_mismatches, st.last_mismatches), (1, 1), "{st:?}");

    // The second consumer draws more from the delivery in which the original
    // seeds (state 3), after the pass started (state 0 and 1, and our replay's
    // start): the original's flood (state 4) reads the new consumption, and so
    // does ours, run just before it (plumbing.md §5.3 PS8; the r6b pass 339
    // difference). They agree.
    message(&w, msg::PIPE_BURST, 0, 0, 0);
    let mut results = Vec::new();
    let mut changed = false;
    for _ in 0..100 {
        let r = deliver(
            &w,
            s,
            msg::SIM_IDLE,
            JULIAN + 2,
            &mut lines,
            || {
                if !changed && state_of(&w).state == 3 {
                    changed = true;
                    w.set(CONSUMER2, prop::CONSUMPTION, mem::SINT32, false, &[40i32]);
                }
            },
            || {},
        );
        if r >= 0 {
            results.push(r);
        }
        let st = state_of(&w);
        if st.state == 9 && st.recompute == 0 {
            break;
        }
    }
    assert!(changed);
    assert_eq!(results, vec![0], "{lines:#?}");
    let p = pass_lines(&lines).last().unwrap().to_string();
    assert!(p.contains("aligned=start exact=1;"), "{p}");
    assert!(p.contains("consumed orig/ours=140/140;"), "{p}");
    assert!(p.contains("used orig/ours=[140/140]"), "{p}");
    assert!(p.contains("mismatches=0;"), "{p}");

    // The producer's water used overwritten right after the original's last
    // flood (its states 4 wrote it; nothing is left to seed, state 5 is next):
    // the used value and the water cost its state 8 adds differ, and are
    // counted.
    message(&w, msg::PIPE_BURST, 0, 0, 0);
    let mut results = Vec::new();
    let meddled = Cell::new(false);
    for _ in 0..100 {
        let r = deliver(
            &w,
            s,
            msg::SIM_IDLE,
            JULIAN + 3,
            &mut lines,
            || {},
            || {
                if !meddled.get() && state_of(&w).state == 5 {
                    meddled.set(true);
                    w.set(PRODUCER, prop::WATER_USED, mem::UINT32, false, &[999u32]);
                }
            },
        );
        if r >= 0 {
            results.push(r);
        }
        let st = state_of(&w);
        if st.state == 9 && st.recompute == 0 {
            break;
        }
    }
    assert!(meddled.get());
    assert_eq!(results, vec![2], "{lines:#?}");
    let p = pass_lines(&lines).last().unwrap().to_string();
    assert!(p.contains("used orig/ours=[999/140!]"), "{p}");
    assert!(p.contains("consumed orig/ours=140/140;"), "{p}");
    assert!(p.contains("grid 32x32 cells=1024 differing=0"), "{p}");
    assert!(p.contains("mismatches=2;"), "{p}");

    // The next pass writes the water used again and agrees.
    message(&w, msg::PIPE_BURST, 0, 0, 0);
    assert_eq!(idle_until_rest(&w, s, JULIAN + 4, &mut lines), vec![0], "{lines:#?}");
    println!("{}", lines.join("\n"));

    unsafe { fc4_plumbing_shadow_destroy(s) };
    stop(&w);
}

/// A consumer removed while a pass runs, before its flood: each of our states
/// runs on the original's flag map of the moment, so the pass stays exact and
/// agrees. Removed during the original's time-sliced state 6, after our
/// unsliced one, it makes the pass "not exact": its differences are reported,
/// not counted. A failed capture is INCOMPLETE; the daily replay budget leaves
/// the other passes followed only.
#[test]
fn shadow_flags_changes_during_a_pass_failures_and_the_budget() {
    let w = started();
    let s = shadow(&w, 0);
    let mut lines: Vec<String> = Vec::new();
    assert_eq!(idle_until_rest(&w, s, JULIAN, &mut lines), vec![0, 0], "{lines:#?}");

    // A consumer removed when the original is about to seed (state 3): the
    // flag map and demand change (no pass state reads demand: PS8); our
    // states 3 and 4 read the new flag map as the original's do.
    message(&w, msg::PIPE_BURST, 0, 0, 0);
    let mut removed = false;
    let mut results = Vec::new();
    for _ in 0..100 {
        let r = deliver(
            &w,
            s,
            msg::SIM_IDLE,
            JULIAN + 1,
            &mut lines,
            || {
                if !removed && state_of(&w).state == 3 {
                    removed = true;
                    message(&w, msg::REMOVE_OCCUPANT, 0, 0, CONSUMER);
                }
            },
            || {},
        );
        if r >= 0 {
            results.push(r);
        }
        if results.len() == 1 {
            break;
        }
    }
    assert_eq!(results, vec![0], "{lines:#?}");
    let p = pass_lines(&lines).last().unwrap().to_string();
    assert!(p.contains("aligned=start exact=1;"), "{p}");
    assert!(p.contains("consumed orig/ours=30/30;"), "{p}");
    assert!(p.contains("wateredCells orig/ours=1/1;"), "{p}");
    // Demand: the record's at the comparison, without the removed consumer.
    assert!(p.contains("demand orig/ours=30/30;"), "{p}");
    assert!(p.contains("mismatches=0;"), "{p}");
    assert_eq!(stats(s).passes_not_exact, 0);
    // The removal's recompute runs one more pass, which agrees.
    assert_eq!(idle_until_rest(&w, s, JULIAN + 1, &mut lines), vec![0], "{lines:#?}");

    // The original's states 6 and 7 sliced (as in the game, §6): the second
    // consumer removed while its state 6 runs, after ours ran whole. The flag
    // map at the swap is not the one our state 6 read: not exact.
    test_set_slice_cells(w.a(), Some(100));
    message(&w, msg::PIPE_BURST, 0, 0, 0);
    let mut removed = false;
    let mut results = Vec::new();
    for _ in 0..200 {
        let r = deliver(
            &w,
            s,
            msg::SIM_IDLE,
            JULIAN + 1,
            &mut lines,
            || {
                if !removed && state_of(&w).state == 6 {
                    removed = true;
                    message(&w, msg::REMOVE_OCCUPANT, 0, 0, CONSUMER2);
                }
            },
            || {},
        );
        if r >= 0 {
            results.push(r);
        }
        if results.len() == 1 {
            break;
        }
    }
    assert!(removed);
    assert_eq!(results, vec![0], "{lines:#?}");
    let p = pass_lines(&lines).last().unwrap().to_string();
    assert!(p.contains("exact=0 (flag map changed during state 6);"), "{p}");
    let st = stats(s);
    assert_eq!(st.passes_not_exact, 1, "{st:?}");
    // Its recompute: one more pass, sliced too, which agrees.
    assert_eq!(idle_until_rest(&w, s, JULIAN + 1, &mut lines), vec![0], "{lines:#?}");
    test_set_slice_cells(w.a(), None);
    message(&w, msg::INSERT_OCCUPANT, 0, 0, CONSUMER);
    message(&w, msg::INSERT_OCCUPANT, 0, 0, CONSUMER2);
    assert_eq!(idle_until_rest(&w, s, JULIAN + 1, &mut lines), vec![0], "{lines:#?}");

    // A capture that fails: INCOMPLETE, nothing counted.
    w.fail_capture.set(true);
    message(&w, msg::PIPE_BURST, 0, 0, 0);
    let r = idle_until_rest(&w, s, JULIAN + 2, &mut lines);
    assert!(r.is_empty(), "{r:?} {lines:#?}");
    assert!(
        lines
            .iter()
            .any(|l| l.contains("INCOMPLETE (capture at the start: the original's Write failed); not compared")),
        "{lines:#?}"
    );
    w.fail_capture.set(false);
    assert!(stats(s).passes_incomplete >= 1);
    unsafe { fc4_plumbing_shadow_destroy(s) };

    // One replay a day: the second pass of the day is only followed.
    let s = shadow(&w, 1);
    let mut lines: Vec<String> = Vec::new();
    message(&w, msg::PIPE_BURST, 0, 0, 0);
    let r = idle_until_rest(&w, s, JULIAN + 5, &mut lines);
    assert_eq!(r, vec![0], "{lines:#?}");
    message(&w, msg::PIPE_BURST, 0, 0, 0);
    let r = idle_until_rest(&w, s, JULIAN + 5, &mut lines);
    assert!(r.is_empty(), "{lines:#?}");
    message(&w, msg::PIPE_BURST, 0, 0, 0);
    let r = idle_until_rest(&w, s, JULIAN + 6, &mut lines);
    assert_eq!(r, vec![0], "{lines:#?}");
    let st = stats(s);
    assert_eq!((st.passes_seen, st.passes_compared), (3, 2), "{st:?}");
    unsafe { fc4_plumbing_shadow_destroy(s) };
    stop(&w);
}

/// Without the passCounter callback (PS6) the start shows on a probe cell:
/// the first pass, already running at the shadow's start, is replayed from
/// its swap (not exact), the next from its start. Made mid-city, the month's
/// distressed count is not exact: the original's pipe condition is not
/// known (PS3).
#[test]
fn shadow_without_the_counter_uses_the_probe_cell() {
    let w = started();
    let s = shadow_with(&w, 0, false);
    let mut lines: Vec<String> = Vec::new();
    assert_eq!(idle_until_rest(&w, s, JULIAN, &mut lines), vec![0, 0], "{lines:#?}");
    // Too late for the city start.
    assert!(!unsafe { fc4_plumbing_shadow_city_start(s) });
    let passes = pass_lines(&lines);
    assert!(
        passes[0].starts_with("plumbing shadow 2000-01-01 pass 1: aligned=swap exact=0 (start not seen);"),
        "{}",
        passes[0]
    );
    assert!(
        passes[1].starts_with("plumbing shadow 2000-01-01 pass 2: aligned=start exact=1;"),
        "{}",
        passes[1]
    );
    assert!(passes[1].contains("mismatches=0;"), "{}", passes[1]);
    w.funding.set(0.0);
    assert_eq!(
        deliver(&w, s, msg::SIM_NEW_MONTH, JULIAN + 31, &mut lines, || {}, || {}),
        0
    );
    let month = lines.last().unwrap();
    assert!(
        month.contains(
            "distressed orig/ours=1/1 burst +0 (ours allows 0..0) (not exact: the original's pipe condition is not \
             known, the shadow started mid-city; plumbing.md §7.1 PS3)"
        ),
        "{month}"
    );
    unsafe { fc4_plumbing_shadow_destroy(s) };
    stop(&w);
}

/// PS6: a deal status restarts the pass (state 9, then 0); the counter
/// moves again and the replay starts with the new pass, exactly.
#[test]
fn a_restart_moves_the_counter_and_is_replayed_from_its_start() {
    let w = started();
    let s = shadow(&w, 0);
    let mut lines: Vec<String> = Vec::new();
    assert_eq!(idle_until_rest(&w, s, JULIAN, &mut lines), vec![0, 0], "{lines:#?}");
    message(&w, msg::PIPE_BURST, 0, 0, 0);
    let mut restarted = false;
    let mut results = Vec::new();
    for _ in 0..100 {
        let r = deliver(
            &w,
            s,
            msg::SIM_IDLE,
            JULIAN + 1,
            &mut lines,
            || {
                if !restarted && state_of(&w).state == 3 {
                    restarted = true;
                    let m = fc4_plumbing_message {
                        msg_type: msg::DEAL_STATUS,
                        deal: IMPORT_DEAL,
                        ..Default::default()
                    };
                    unsafe { fc4_plumbing_do_message(w.a(), &m) };
                }
            },
            || {},
        );
        if r >= 0 {
            results.push(r);
        }
        let st = state_of(&w);
        if st.state == 9 && st.recompute == 0 {
            break;
        }
    }
    assert!(restarted);
    assert_eq!(results, vec![0], "{lines:#?}");
    let p = pass_lines(&lines).last().unwrap().to_string();
    assert!(p.contains("pass 4: aligned=start exact=1;"), "{p}");
    assert!(p.contains("mismatches=0;"), "{p}");
    unsafe { fc4_plumbing_shadow_destroy(s) };
    stop(&w);
}

/// A pass the original spreads over many game days (its states 6 and 7 are
/// time-sliced, and a loaded game delivers few SimIdle: r6b saw passes of
/// four days) is followed to its swap and compared exactly; there is no time
/// limit, and no INCOMPLETE line.
#[test]
fn a_pass_over_many_days_is_compared() {
    let w = started();
    let s = shadow(&w, 0);
    let mut lines: Vec<String> = Vec::new();
    assert_eq!(idle_until_rest(&w, s, JULIAN, &mut lines), vec![0, 0], "{lines:#?}");
    test_set_slice_cells(w.a(), Some(64));
    message(&w, msg::PIPE_BURST, 0, 0, 0);
    let mut results = Vec::new();
    for day in (JULIAN + 1..).take(200) {
        // One delivery a game day.
        let r = deliver(&w, s, msg::SIM_NEW_DAY, day, &mut lines, || {}, || {});
        if r >= 0 {
            results.push(r);
        }
        let st = state_of(&w);
        if st.state == 9 && st.recompute == 0 {
            break;
        }
    }
    assert_eq!(results, vec![0], "{lines:#?}");
    let p = pass_lines(&lines).last().unwrap().to_string();
    assert!(p.contains("pass 3: aligned=start exact=1;"), "{p}");
    assert!(p.contains("grid 32x32 cells=1024 differing=0"), "{p}");
    assert!(p.contains("mismatches=0;"), "{p}");
    let days: i32 = p
        .split("days=")
        .nth(1)
        .and_then(|t| t.split(';').next())
        .and_then(|t| t.parse().ok())
        .unwrap();
    assert!(days > 2, "{p}");
    assert!(!lines.iter().any(|l| l.contains("INCOMPLETE")), "{lines:#?}");
    assert_eq!(stats(s).passes_incomplete, 0);
    unsafe { fc4_plumbing_shadow_destroy(s) };
    stop(&w);
}

/// Pipe/producer lists can change after the grid comparison. State 8 must
/// consume the current lists, without replacing the computed pass results.
#[test]
fn removals_between_the_swap_and_state_8_are_live_inputs() {
    let w = started();
    let s = shadow(&w, 0);
    let mut lines = Vec::new();
    let mut removed = false;
    let mut results = Vec::new();
    for _ in 0..100 {
        if !removed && state_of(&w).state == 8 {
            message(&w, msg::REMOVE_OCCUPANT, 0, 0, PIPE0);
            message(&w, msg::REMOVE_OCCUPANT, 0, 0, PRODUCER);
            removed = true;
        }
        let r = deliver(&w, s, msg::SIM_IDLE, JULIAN, &mut lines, || {}, || {});
        if r >= 0 {
            results.push(r);
        }
        let st = state_of(&w);
        if st.state == 9 && st.recompute == 0 {
            break;
        }
    }
    assert!(removed);
    assert!(results.iter().all(|&r| r == 0), "{lines:#?}");
    let first = pass_lines(&lines)[0].to_string();
    assert!(first.contains("exact=1;"), "{first}");
    assert!(first.contains("pipes orig/ours=99/99;"), "{first}");
    assert!(first.contains("sources orig/ours=0/0;"), "{first}");
    assert!(first.contains("produced orig/ours=1000/1000;"), "{first}");
    unsafe { fc4_plumbing_shadow_destroy(s) };
    stop(&w);
}

/// PS7: made mid-city around a month, the shadow does not know whether the
/// next state 8 ages; the producer's condition, changed by the original's
/// state 8, shows it did, and the conditions agree.
#[test]
fn an_unknown_ageing_shows_in_the_condition() {
    let w = started();
    let s = shadow_with(&w, 0, true);
    let mut lines: Vec<String> = Vec::new();
    assert_eq!(deliver(&w, s, msg::SIM_NEW_MONTH, JULIAN, &mut lines, || {}, || {}), 0);
    let r = idle_until_rest(&w, s, JULIAN, &mut lines);
    assert!(r.iter().all(|&n| n == 0), "{lines:#?}");
    let first = pass_lines(&lines)[0].to_string();
    assert!(
        first.contains(
            "ageing=yes (inferred: a producer's condition changed; plumbing.md §6.7, PS7); condition orig/ours=["
        ),
        "{first}"
    );
    assert!(!first.contains('!'), "{first}");
    // The next pass: an observed end since, so not aged.
    let last = pass_lines(&lines).last().unwrap().to_string();
    if r.len() > 1 {
        assert!(last.contains("ageing=no;"), "{last}");
    }
    unsafe { fc4_plumbing_shadow_destroy(s) };
    stop(&w);
}

#[test]
fn shadow_rejects_bad_tables_and_answers_null_handles() {
    let w = World::new();
    let h = host(&w);
    let o = original(&w);
    unsafe {
        assert!(fc4_plumbing_shadow_create(std::ptr::null(), &o, 0, 0).is_null());
        assert!(fc4_plumbing_shadow_create(&h, std::ptr::null(), 0, 0).is_null());
        let mut short = h;
        short.struct_size = 8;
        assert!(fc4_plumbing_shadow_create(&short, &o, 0, 0).is_null());
        let mut short = h;
        short.grids.struct_size = 8;
        assert!(fc4_plumbing_shadow_create(&short, &o, 0, 0).is_null());
        let mut no_capture = o;
        no_capture.capture = None;
        assert!(fc4_plumbing_shadow_create(&h, &no_capture, 0, 0).is_null());
        let mut short = o;
        short.struct_size = 8;
        assert!(fc4_plumbing_shadow_create(&h, &short, 0, 0).is_null());

        let null = std::ptr::null_mut();
        assert_eq!(fc4_plumbing_shadow_before(null, msg::SIM_IDLE, JULIAN), -1);
        assert_eq!(fc4_plumbing_shadow_before(null, 0x1234, JULIAN), -2);
        assert_eq!(fc4_plumbing_shadow_after(null, None, null as *mut c_void), -1);
        let mut st = fc4_plumbing_shadow_stats::default();
        assert!(!fc4_plumbing_shadow_get_stats(null, &mut st));
        assert!(!fc4_plumbing_shadow_city_start(null));
        fc4_plumbing_shadow_destroy(null);

        let s = fc4_plumbing_shadow_create(&h, &o, 0, 0);
        assert!(!s.is_null());
        assert!(!fc4_plumbing_shadow_get_stats(s, std::ptr::null_mut()));
        // Not a plumbing delivery; an after without a before.
        assert_eq!(fc4_plumbing_shadow_before(s, msg::TUNING_RELOAD, JULIAN), -2);
        assert_eq!(fc4_plumbing_shadow_after(s, None, null as *mut c_void), -1);
        let st = stats(s);
        assert_eq!((st.deliveries, st.last_mismatches, st.struct_size), (0, -1, 72));
        fc4_plumbing_shadow_destroy(s);
    }
}

#[test]
fn shadow_struct_layouts() {
    assert_eq!(std::mem::size_of::<fc4_plumbing_capture_ref>(), 40);
    assert_eq!(std::mem::size_of::<fc4_plumbing_shadow_stats>(), 72);
    let ptr = std::mem::size_of::<usize>();
    assert_eq!(
        std::mem::size_of::<fc4_plumbing_capture>(),
        if ptr == 4 { 24 } else { 40 }
    );
    assert_eq!(
        std::mem::size_of::<fc4_plumbing_original>(),
        if ptr == 4 { 32 } else { 64 }
    );
}
