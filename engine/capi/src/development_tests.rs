// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The building development simulator's and the growth developer's C ABI
//! against a fake game: the catalogue load from host exemplars, the
//! developers as host objects, the construction node and the building map as
//! game objects, the tract developer, the reference protocol, persistence
//! through host streams and re-entrant messages.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::{c_char, c_void};

use fc4_core::development_simulator::{msg, BUILDING_MAP_INSTANCE};
use fc4_core::growth_developer::shipped_exemplar;
use fc4_core::property::PropertyHolder;
use fc4_core::variant::{mem, Elem, Variant};

use crate::budget::fc4_token_callback;
use crate::development::*;
use crate::growth::*;
use crate::hostgrid::fc4_grid_api;
use crate::landvalue::fc4_grid_geometry;
use crate::props::fc4_props;
use crate::test_streams::{istream, ostream, In, Out};

const CELLS: i32 = 16;
const NODE: u64 = 0xE000;
const DEV_BASE: u64 = 0xD000;
const SOURCE_BASE: u64 = 0x5000;
const LOT: u64 = 0x8000;
const OCC: u64 = 0x7001;
const BUILDING: u32 = 0x1234_0001;
const FAMILY: u32 = 0x9000_0001;
const FOUNDATION: u32 = 0x2222_0001;
const EXEMPLAR: u32 = 0x6534_284A;

struct Grid {
    kind: u32,
    instance: u32,
    tract_size: i32,
    data: Vec<f64>,
    refs: i32,
    shut: bool,
}

#[derive(Default)]
struct Dev {
    key: (u32, u32, u32),
    inits: u32,
    shutdowns: u32,
    sub_type: u32,
    active: f32,
    growth: Vec<fc4_growth_item>,
    started: u32,
}

#[derive(Default)]
struct Game {
    grids: RefCell<HashMap<u64, Grid>>,
    next: Cell<u64>,
    refs: RefCell<HashMap<u64, i32>>,
    devs: RefCell<HashMap<u64, Dev>>,
    next_dev: Cell<u64>,
    node: RefCell<(f32, f32)>,
    node_inits: Cell<u32>,
    node_shutdowns: Cell<u32>,
    subs: RefCell<Vec<u32>>,
    agents: RefCell<Vec<(u32, String, u32)>>,
    cheats: RefCell<Vec<(u32, String)>>,
    commands: RefCell<Vec<(u32, String)>>,
    posted: RefCell<Vec<u32>>,
    services: Cell<i32>,
    grows: RefCell<Vec<fc4_growth_profile>>,
    grow_result: Cell<i32>,
    day: Cell<u32>,
    census: Cell<f32>,
    /// Re-entrancy test: `tract_grow` delivers a cheat message to this handle.
    target: Cell<usize>,
    zone: Cell<i8>,
    lot_state: Cell<u32>,
    // growth developer host
    agents_g: RefCell<Vec<u32>>,
    tract_sizes: RefCell<Vec<(u64, i32)>>,
    released_services: RefCell<Vec<u32>>,
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

impl Game {
    fn refs_of(&self, t: u64) -> i32 {
        self.refs.borrow().get(&t).copied().unwrap_or(0)
    }
    fn grid_refs(&self, t: u64) -> i32 {
        self.grids.borrow().get(&t).map_or(0, |g| g.refs)
    }
    fn live_grids(&self) -> usize {
        self.grids.borrow().values().filter(|g| g.refs > 0).count()
    }
}

fn s(p: *const c_char, n: usize) -> String {
    unsafe { String::from_utf8_lossy(std::slice::from_raw_parts(p as *const u8, n)).into_owned() }
}

// ---- grids --------------------------------------------------------------------------------------------

unsafe extern "C" fn g_create(u: *mut c_void, kind: u32, instance: u32, ts: i32) -> u64 {
    let g = game(u);
    let t = 0x1_0000 + g.next.get() * 0x10;
    g.next.set(g.next.get() + 1);
    g.grids.borrow_mut().insert(
        t,
        Grid {
            kind,
            instance,
            tract_size: ts,
            data: vec![0.0; (CELLS * CELLS) as usize],
            refs: 1,
            shut: false,
        },
    );
    t
}
unsafe extern "C" fn g_class(u: *mut c_void, t: u64) -> u32 {
    game(u).grids.borrow().get(&t).map_or(0, |g| g.kind)
}
unsafe extern "C" fn g_geometry(u: *mut c_void, t: u64, out: *mut fc4_grid_geometry) {
    if game(u).grids.borrow().contains_key(&t) {
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
}
unsafe extern "C" fn g_get(u: *mut c_void, t: u64, _k: u32, x: i32, z: i32) -> f64 {
    game(u)
        .grids
        .borrow()
        .get(&t)
        .map_or(0.0, |g| g.data[(x * CELLS + z) as usize])
}
unsafe extern "C" fn g_set(u: *mut c_void, t: u64, _k: u32, x: i32, z: i32, v: f64) {
    if let Some(g) = game(u).grids.borrow_mut().get_mut(&t) {
        g.data[(x * CELLS + z) as usize] = v;
    }
}
unsafe extern "C" fn g_fill(u: *mut c_void, t: u64, _k: u32, v: f64) {
    if let Some(g) = game(u).grids.borrow_mut().get_mut(&t) {
        g.data.iter_mut().for_each(|d| *d = v);
    }
}
unsafe extern "C" fn g_shutdown(u: *mut c_void, t: u64) {
    if let Some(g) = game(u).grids.borrow_mut().get_mut(&t) {
        g.shut = true;
    }
}
unsafe extern "C" fn g_release(u: *mut c_void, t: u64) {
    if let Some(g) = game(u).grids.borrow_mut().get_mut(&t) {
        g.refs -= 1;
    }
}

fn grid_api(g: &Game) -> fc4_grid_api {
    fc4_grid_api {
        struct_size: std::mem::size_of::<fc4_grid_api>() as u32,
        user: g as *const Game as *mut c_void,
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

// ---- BDS host -----------------------------------------------------------------------------------------

fn holder(props: &[(u32, Variant)]) -> *mut fc4_props {
    let mut h = PropertyHolder::new();
    for (id, v) in props {
        h.add(*id, v.clone());
    }
    Box::into_raw(Box::new(h)) as *mut fc4_props
}

fn u32v(v: u32) -> Variant {
    Variant::Scalar(Elem::U32(v))
}

fn u32a(v: &[u32]) -> Variant {
    Variant::array(mem::UINT32, v.iter().map(|&x| Elem::U32(x)).collect())
}

fn f32a(v: &[f32]) -> Variant {
    Variant::array(mem::FLOAT32, v.iter().map(|&x| Elem::F32(x)).collect())
}

unsafe extern "C" fn b_exemplars(
    _u: *mut c_void,
    types: *const u32,
    n: usize,
    visit: fc4_development_exemplar_visitor,
    ctx: *mut c_void,
) {
    let want: Vec<u32> = if n == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(types, n).to_vec()
    };
    let visit = visit.unwrap();
    // (exemplar type, instance, group, props, localized name)
    type Exemplar = (u32, u32, u32, Vec<(u32, Variant)>, Option<&'static [u8]>);
    let all: Vec<Exemplar> = vec![
        (
            2,
            BUILDING,
            0x1111,
            vec![
                (0x20, Variant::string(b"bld_exemplar")),
                (0x2781_2832, u32v(1)),
                (0x2781_2833, u32v(1)),
                (0x2781_2834, u32a(&[0x1010, 20])),
                (0x2781_2810, f32a(&[16.0, 10.0, 16.0])),
                (0x2781_2870, u32a(&[FAMILY])),
                (0xAA1D_D396, u32a(&[0x1000, 0x150A])),
            ],
            Some(b"Localized House"),
        ),
        (0x11, FOUNDATION, 0x2222, vec![], None),
        (4, 0x1010, 0x67CD_5FA1, vec![(0x21, u32v(0x1010))], None),
        (4, 0x5555, 0x67CD_5FA1, vec![], None),
        (7, 0x7777, 0x1, vec![], None),
    ];
    for (et, inst, group, mut props, name) in all {
        if !want.is_empty() && !want.contains(&et) {
            continue;
        }
        props.push((0x10, u32v(et)));
        let key = fc4_res_key {
            type_id: EXEMPLAR,
            group,
            instance: inst,
        };
        let (np, nl) = name.map_or((std::ptr::null(), 0), |b| (b.as_ptr(), b.len()));
        visit(ctx, &key, holder(&props), np, nl, name.is_some());
    }
}

unsafe extern "C" fn b_lot_configs(_u: *mut c_void, visit: fc4_lot_config_visitor, ctx: *mut c_void) {
    let ids = [BUILDING];
    let c = fc4_lot_config {
        size_x: 1,
        size_z: 1,
        zones: 1 << 1,
        has_primary: 1,
        orientation: 0,
        x: 8.0,
        z: 8.0,
        n_ids: 1,
        ids: ids.as_ptr(),
    };
    visit.unwrap()(ctx, &c);
}

unsafe extern "C" fn b_day(u: *mut c_void) -> u32 {
    game(u).day.get()
}
unsafe extern "C" fn b_cell_widths(_u: *mut c_void, out: *mut f32) {
    *out = 16.0;
    *out.add(1) = 16.0;
}
unsafe extern "C" fn b_cell_counts(_u: *mut c_void, out: *mut i32) {
    *out = CELLS;
    *out.add(1) = CELLS;
}
unsafe extern "C" fn b_post(u: *mut c_void, n: u32) {
    game(u).posted.borrow_mut().push(n);
}
unsafe extern "C" fn b_subscribe(u: *mut c_void, t: *const u32, n: usize) {
    game(u)
        .subs
        .borrow_mut()
        .extend_from_slice(std::slice::from_raw_parts(t, n));
}
unsafe extern "C" fn b_unsubscribe(u: *mut c_void, t: *const u32, n: usize) {
    let v = std::slice::from_raw_parts(t, n);
    game(u).subs.borrow_mut().retain(|x| !v.contains(x));
}
unsafe extern "C" fn b_register_command(u: *mut c_void, id: u32, p: *const c_char, n: usize) {
    game(u).commands.borrow_mut().push((id, s(p, n)));
}
unsafe extern "C" fn b_add_agent(u: *mut c_void, level: u32, p: *const c_char, n: usize, flags: u32) {
    game(u).agents.borrow_mut().push((level, s(p, n), flags));
}
unsafe extern "C" fn b_remove_agent(u: *mut c_void) {
    game(u).agents.borrow_mut().clear();
}
unsafe extern "C" fn b_register_cheat(u: *mut c_void, id: u32, p: *const c_char, n: usize) {
    game(u).cheats.borrow_mut().push((id, s(p, n)));
}
unsafe extern "C" fn b_unregister_cheat(u: *mut c_void, id: u32) {
    game(u).cheats.borrow_mut().retain(|c| c.0 != id);
}
unsafe extern "C" fn b_acquire(u: *mut c_void) {
    game(u).services.set(game(u).services.get() + 1);
}
unsafe extern "C" fn b_release_services(u: *mut c_void) {
    game(u).services.set(game(u).services.get() - 1);
}
unsafe extern "C" fn b_census(u: *mut c_void, _id: u32, out: *mut f32) -> bool {
    *out = game(u).census.get();
    true
}
unsafe extern "C" fn b_create_node(u: *mut c_void) -> u64 {
    // One token for every node; each creation is one more reference.
    *game(u).refs.borrow_mut().entry(NODE).or_insert(0) += 1;
    NODE
}
unsafe extern "C" fn b_node_init(u: *mut c_void, _t: u64) {
    game(u).node_inits.set(game(u).node_inits.get() + 1);
}
unsafe extern "C" fn b_node_shutdown(u: *mut c_void, _t: u64) {
    game(u).node_shutdowns.set(game(u).node_shutdowns.get() + 1);
}
unsafe extern "C" fn b_node_supply(u: *mut c_void, _t: u64) -> f32 {
    game(u).node.borrow().0
}
unsafe extern "C" fn b_node_demand(u: *mut c_void, _t: u64) -> f32 {
    game(u).node.borrow().1
}
unsafe extern "C" fn b_node_set_supply(u: *mut c_void, _t: u64, v: f32) {
    game(u).node.borrow_mut().0 = v;
}
unsafe extern "C" fn b_node_set_demand(u: *mut c_void, _t: u64, v: f32) {
    game(u).node.borrow_mut().1 = v;
}
unsafe extern "C" fn b_node_add(u: *mut c_void, _t: u64, v: f32) {
    game(u).node.borrow_mut().1 += v;
}
unsafe extern "C" fn b_node_eoc(_u: *mut c_void, _t: u64) -> bool {
    true
}
unsafe extern "C" fn b_active(u: *mut c_void, node: u64) -> f32 {
    game(u)
        .devs
        .borrow()
        .get(&(node - SOURCE_BASE + DEV_BASE))
        .map_or(0.0, |d| d.active)
}
unsafe extern "C" fn b_grow(u: *mut c_void, p: *const fc4_growth_profile, _flag: *mut bool) -> i32 {
    let g = game(u);
    g.grows.borrow_mut().push(*p);
    if g.target.get() != 0 {
        // A message sent while the BDS is running: queued.
        let m = fc4_development_message {
            msg_type: msg::CHEAT,
            data1: 0x08DA_D89A,
            ..Default::default()
        };
        assert!(fc4_development_do_message(
            g.target.get() as *mut fc4_development,
            &m,
            std::ptr::null(),
            0
        ));
        assert!(
            !fc4_development_cheat(g.target.get() as *const fc4_development, 1),
            "not handled yet"
        );
        // Slots run at once.
        assert!(fc4_development_claim(g.target.get() as *mut fc4_development, 5));
    }
    g.grow_result.get()
}
unsafe extern "C" fn b_zone(u: *mut c_void, _x: i32, _z: i32) -> i8 {
    game(u).zone.get()
}
unsafe extern "C" fn b_lot_at(_u: *mut c_void, _x: i32, _z: i32) -> u64 {
    LOT
}
unsafe extern "C" fn b_lot_state(u: *mut c_void, _l: u64) -> u32 {
    game(u).lot_state.get()
}
unsafe extern "C" fn b_true2(_u: *mut c_void, _x: i32, _z: i32) -> bool {
    true
}
unsafe extern "C" fn b_lv_type(_u: *mut c_void, _x: i32, _z: i32) -> u32 {
    1
}
unsafe extern "C" fn b_add_ref(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_insert(0) += 1;
}
unsafe extern "C" fn b_release(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_insert(0) -= 1;
}
unsafe extern "C" fn b_create_dev(u: *mut c_void, key: *const fc4_res_key) -> u64 {
    let g = game(u);
    let t = DEV_BASE + g.next_dev.get();
    g.next_dev.set(g.next_dev.get() + 1);
    let k = &*key;
    g.devs.borrow_mut().insert(
        t,
        Dev {
            key: (k.type_id, k.group, k.instance),
            ..Dev::default()
        },
    );
    g.refs.borrow_mut().insert(t, 1);
    t
}
unsafe extern "C" fn b_dev_init(u: *mut c_void, d: u64, key: *const fc4_res_key) -> bool {
    let mut devs = game(u).devs.borrow_mut();
    let dev = devs.entry(d).or_default();
    dev.inits += 1;
    dev.key = ((*key).type_id, (*key).group, (*key).instance);
    dev.sub_type = (*key).instance;
    true
}
unsafe extern "C" fn b_dev_shutdown(u: *mut c_void, d: u64) -> bool {
    if let Some(dev) = game(u).devs.borrow_mut().get_mut(&d) {
        dev.shutdowns += 1;
    }
    true
}
unsafe extern "C" fn b_dev_sub_type(u: *mut c_void, d: u64) -> u32 {
    game(u).devs.borrow().get(&d).map_or(0, |x| x.sub_type)
}
unsafe extern "C" fn b_dev_purpose(_u: *mut c_void, _d: u64) -> u32 {
    1
}
unsafe extern "C" fn b_dev_valid(_u: *mut c_void, _d: u64) -> bool {
    true
}
unsafe extern "C" fn b_dev_list(u: *mut c_void, d: u64, which: u32, sink: fc4_growth_items_sink, ctx: *mut c_void) {
    let v = if which == 0 {
        game(u)
            .devs
            .borrow()
            .get(&d)
            .map(|x| x.growth.clone())
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    sink.unwrap()(ctx, v.as_ptr(), v.len());
}
unsafe extern "C" fn b_dev_source(_u: *mut c_void, d: u64) -> u64 {
    d - DEV_BASE + SOURCE_BASE
}
unsafe extern "C" fn b_dev_start(u: *mut c_void, d: u64) {
    if let Some(dev) = game(u).devs.borrow_mut().get_mut(&d) {
        dev.started += 1;
    }
}
unsafe extern "C" fn b_dev_finished(_u: *mut c_void, _d: u64) -> bool {
    true
}

fn bds_host(g: &Game) -> fc4_development_host {
    let mut h: fc4_development_host = unsafe { std::mem::zeroed() };
    h.struct_size = std::mem::size_of::<fc4_development_host>() as u32;
    h.user = g as *const Game as *mut c_void;
    h.grids = grid_api(g);
    h.sim_day = Some(b_day);
    h.cell_widths = Some(b_cell_widths);
    h.terrain_cell_counts = Some(b_cell_counts);
    h.exemplars = Some(b_exemplars);
    h.lot_configurations = Some(b_lot_configs);
    h.post_building_types_updated = Some(b_post);
    h.subscribe = Some(b_subscribe);
    h.unsubscribe = Some(b_unsubscribe);
    h.register_command = Some(b_register_command);
    h.add_agent = Some(b_add_agent);
    h.remove_agent = Some(b_remove_agent);
    h.register_cheat = Some(b_register_cheat);
    h.unregister_cheat = Some(b_unregister_cheat);
    h.acquire_services = Some(b_acquire);
    h.release_services = Some(b_release_services);
    h.census_supply = Some(b_census);
    h.create_construction_node = Some(b_create_node);
    h.node_init = Some(b_node_init);
    h.node_shutdown = Some(b_node_shutdown);
    h.node_supply = Some(b_node_supply);
    h.node_demand = Some(b_node_demand);
    h.node_set_supply = Some(b_node_set_supply);
    h.node_set_demand = Some(b_node_set_demand);
    h.node_add_to_demand = Some(b_node_add);
    h.node_end_of_cycle = Some(b_node_eoc);
    h.demand_active_value = Some(b_active);
    h.tract_grow = Some(b_grow);
    h.zone_type = Some(b_zone);
    h.lot_at = Some(b_lot_at);
    h.lot_state = Some(b_lot_state);
    h.powered = Some(b_true2);
    h.land_value_type = Some(b_lv_type);
    h.add_ref = Some(b_add_ref);
    h.release = Some(b_release);
    h.create_developer = Some(b_create_dev);
    h.developer_init = Some(b_dev_init);
    h.developer_shutdown = Some(b_dev_shutdown);
    h.developer_sub_type = Some(b_dev_sub_type);
    h.developer_purpose = Some(b_dev_purpose);
    h.developer_lists_valid = Some(b_dev_valid);
    h.developer_list = Some(b_dev_list);
    h.developer_source_demand = Some(b_dev_source);
    h.developer_start_map_update = Some(b_dev_start);
    h.developer_map_update_finished = Some(b_dev_finished);
    h
}

fn message(t: u32) -> fc4_development_message {
    fc4_development_message {
        msg_type: t,
        ..Default::default()
    }
}

unsafe fn send(p: *mut fc4_development, t: u32) {
    assert!(fc4_development_do_message(p, &message(t), std::ptr::null(), 0));
}

unsafe fn state(p: *const fc4_development) -> fc4_development_state {
    let mut st = fc4_development_state::default();
    assert!(fc4_development_get_state(p, &mut st));
    st
}

unsafe extern "C" fn push_u32_pair(ctx: *mut c_void, k: u32, v: u32) {
    (*(ctx as *mut Vec<(u32, u32)>)).push((k, v));
}
unsafe extern "C" fn push_dev(ctx: *mut c_void, k: u32, t: u64) {
    (*(ctx as *mut Vec<(u32, u64)>)).push((k, t));
}
unsafe extern "C" fn push_bytes(ctx: *mut c_void, p: *const u8, n: usize) {
    (*(ctx as *mut Vec<u8>)).extend_from_slice(std::slice::from_raw_parts(p, n));
}
unsafe extern "C" fn push_density(ctx: *mut c_void, z: u32, k: u32, e: f32, c: f32) {
    (*(ctx as *mut Vec<(u32, u32, f32, f32)>)).push((z, k, e, c));
}

/// One live BDS with its process statics.
unsafe fn new_bds(g: &Game, pp: *mut fc4_development_process, token: u64) -> *mut fc4_development {
    let host = bds_host(g);
    let p = fc4_development_create(&host, pp, token);
    assert!(!p.is_null());
    p
}

#[test]
fn init_loads_the_catalogue_once_creates_developers_and_registers() {
    unsafe {
        let g = Game::default();
        g.day.set(100);
        let pp = fc4_development_process_create(7);
        let p = new_bds(&g, pp, 0xB0);
        assert!(!fc4_development_catalogue_loaded(pp));
        assert!(fc4_development_init(p));

        // §1.3 step 1: once per process, one post with the building count.
        assert!(fc4_development_catalogue_loaded(pp));
        assert_eq!(*g.posted.borrow(), vec![1]);
        // §1.5: one developer (the exemplar without 0x21 creates nothing).
        let mut devs: Vec<(u32, u64)> = Vec::new();
        assert_eq!(
            fc4_development_developers(p, Some(push_dev), &mut devs as *mut _ as *mut c_void),
            1
        );
        assert_eq!(devs, vec![(0x1010, DEV_BASE)]);
        assert_eq!(fc4_development_developer(p, 0x1010), DEV_BASE);
        {
            let devs = g.devs.borrow();
            let d = &devs[&DEV_BASE];
            assert_eq!(d.key, (EXEMPLAR, 0x67CD_5FA1, 0x1010));
            assert_eq!(d.inits, 1);
        }
        assert_eq!(fc4_development_last_successful_build(p, 0x1010), 100);
        // Steps 6–10.
        let map = fc4_development_building_map(p);
        assert_ne!(map, 0);
        assert_eq!(g.grids.borrow()[&map].instance, BUILDING_MAP_INSTANCE);
        assert_eq!(g.grids.borrow()[&map].tract_size, 1);
        assert_eq!(g.node_inits.get(), 1);
        assert_eq!(*g.node.borrow(), (100.0, 0.0));
        assert_eq!(fc4_development_total_resources(p), 100);
        assert_eq!(*g.subs.borrow(), msg::SUBSCRIPTIONS.to_vec());
        assert_eq!(*g.commands.borrow(), vec![(0x0B99_AE81, "PlaceBuilding".to_string())]);
        let levels: Vec<u32> = g.agents.borrow().iter().map(|a| a.0).collect();
        assert_eq!(levels, vec![0, 4, 6, 10]);
        assert!(g
            .agents
            .borrow()
            .iter()
            .all(|a| a.1 == "cSC4BuildingDevelopmentSimulator" && a.2 == 2));
        assert_eq!(g.cheats.borrow().len(), 3);
        assert_eq!(g.services.get(), 1);

        // A second BDS of the same process does not load again.
        let p2 = new_bds(&g, pp, 0xB1);
        assert!(fc4_development_init(p2));
        assert_eq!(g.posted.borrow().len(), 1);
        assert!(fc4_development_shutdown(p2));
        fc4_development_destroy(p2);

        // Shutdown: everything released.
        assert!(fc4_development_shutdown(p));
        assert_eq!(g.refs_of(DEV_BASE), 0);
        assert_eq!(g.devs.borrow()[&DEV_BASE].shutdowns, 1);
        assert_eq!(g.refs_of(NODE), 0);
        assert_eq!(g.node_shutdowns.get(), 2);
        assert_eq!(g.grid_refs(map), 0);
        assert!(
            !g.grids.borrow()[&map].shut,
            "the building map is released, not shut down"
        );
        assert!(g.subs.borrow().is_empty());
        assert!(g.agents.borrow().is_empty());
        assert!(g.cheats.borrow().is_empty());
        assert_eq!(g.services.get(), 0);
        fc4_development_destroy(p);
        fc4_development_process_destroy(pp);
    }
}

#[test]
fn the_catalogue_is_exported_from_the_process_statics() {
    unsafe {
        let g = Game::default();
        let pp = fc4_development_process_create(7);
        let p = new_bds(&g, pp, 0xB0);
        assert!(fc4_development_init(p));

        let mut v = [0u32; 4];
        assert_eq!(fc4_development_catalogue_types(pp, 0, v.as_mut_ptr(), 4), 1);
        assert_eq!(v[0], BUILDING);
        assert_eq!(fc4_development_catalogue_types(pp, 1, v.as_mut_ptr(), 4), 1);
        assert_eq!(v[0], FAMILY);
        assert_eq!(fc4_development_catalogue_types(pp, 2, v.as_mut_ptr(), 4), 1);
        assert_eq!(v[0], FOUNDATION);
        assert_eq!(fc4_development_catalogue_types(pp, 3, std::ptr::null_mut(), 0), 0);
        assert_eq!(fc4_development_catalogue_family(pp, FAMILY, v.as_mut_ptr(), 4), 1);
        assert_eq!(v[0], BUILDING);
        assert_eq!(fc4_development_catalogue_family(pp, BUILDING, v.as_mut_ptr(), 4), 0);

        let mut k = fc4_res_key::default();
        assert!(fc4_development_catalogue_key(pp, 0, BUILDING, &mut k));
        assert_eq!((k.type_id, k.group, k.instance), (EXEMPLAR, 0x1111, BUILDING));
        assert!(fc4_development_catalogue_key(pp, 2, FOUNDATION, &mut k));
        assert!(!fc4_development_catalogue_key(pp, 0, 0xDEAD, &mut k));

        let mut name: Vec<u8> = Vec::new();
        assert!(fc4_development_catalogue_name(
            pp,
            1,
            BUILDING,
            Some(push_bytes),
            &mut name as *mut _ as *mut c_void
        ));
        assert_eq!(name, b"Localized House");
        name.clear();
        assert!(fc4_development_catalogue_name(
            pp,
            0,
            BUILDING,
            Some(push_bytes),
            &mut name as *mut _ as *mut c_void
        ));
        assert_eq!(name, b"bld_exemplar");
        assert!(!fc4_development_catalogue_name(
            pp,
            0,
            0xDEAD,
            None,
            std::ptr::null_mut()
        ));

        let mut prof = fc4_building_profile::default();
        let mut caps: Vec<(u32, u32)> = Vec::new();
        assert!(fc4_development_catalogue_profile(
            pp,
            BUILDING,
            &mut prof,
            Some(push_u32_pair),
            &mut caps as *mut _ as *mut c_void
        ));
        assert_eq!((prof.purpose, prof.wealth, prof.capacity), (1, 1, 20));
        assert_eq!(prof.value_499afa38, 30);
        assert_eq!(prof.category_flags, 1 | (1 << 23));
        assert_eq!(prof.occupant_size, [16.0, 10.0, 16.0]);
        assert_eq!(caps, vec![(0x1010, 20)]);

        // §4.7.4: one configuration, zone 1, key 4·1 + 1 + 1.
        let mut dens: Vec<(u32, u32, f32, f32)> = Vec::new();
        assert_eq!(
            fc4_development_catalogue_density(pp, Some(push_density), &mut dens as *mut _ as *mut c_void),
            1
        );
        assert_eq!(dens, vec![(1, 6, 20.5, 20.0)]);
        assert_eq!(fc4_development_catalogue_max_capacity(pp, 1, 1, 1), 20.0);
        assert_eq!(fc4_development_building_map_key(1, 1), 6);
        assert_eq!(fc4_development_developer_type(3, 3), 0x3330);
        assert_eq!(fc4_development_wealth_of(0x3330), 3);

        assert!(fc4_development_shutdown(p));
        fc4_development_destroy(p);
        fc4_development_process_destroy(pp);
    }
}

#[test]
fn query_interface_codes() {
    assert_eq!(fc4_development_query_interface(1), 1);
    assert_eq!(fc4_development_query_interface(0x0781_3EAE), 1);
    assert_eq!(fc4_development_query_interface(0xE98B_2F57), 2);
    assert_eq!(fc4_development_query_interface(0x4522_94AA), 3);
    assert_eq!(fc4_development_query_interface(0xE4FD_A3D4), 4);
    assert_eq!(fc4_development_query_interface(0x1234), 0);
    assert_eq!(fc4_growth_query_interface(0xE7CD_D662), 1);
    assert_eq!(fc4_growth_query_interface(0x4522_94AA), 2);
    assert_eq!(fc4_growth_query_interface(0xE4FD_A3D4), 3);
    assert_eq!(fc4_growth_query_interface(0xE98B_2F57), 0);
}

#[test]
fn monthly_planning_and_daily_growth_reach_the_tract_developer() {
    unsafe {
        let g = Game::default();
        g.day.set(10);
        g.census.set(1000.0);
        g.grow_result.set(12);
        let pp = fc4_development_process_create(7);
        let p = new_bds(&g, pp, 0xB0);
        assert!(fc4_development_init(p));
        {
            let mut devs = g.devs.borrow_mut();
            let d = devs.get_mut(&DEV_BASE).unwrap();
            d.active = 50.0;
            d.growth = vec![
                fc4_growth_item {
                    x: 3,
                    z: 4,
                    max_stage: 2,
                },
                fc4_growth_item {
                    x: 5,
                    z: 6,
                    max_stage: 1,
                },
            ];
        }
        // §4.1: SimBegin sets the supply (0.5 × 3000 clamped to 100..2000)
        // and starts every developer's map update.
        send(p, msg::SIM_BEGIN);
        assert_eq!(g.node.borrow().0, 1500.0);
        assert_eq!(g.devs.borrow()[&DEV_BASE].started, 1);
        assert_eq!(state(p).state, 1);
        send(p, msg::SIM_IDLE);
        assert_eq!(state(p).state, 4);
        send(p, msg::SIM_IDLE);
        assert_eq!(state(p).state, 0);

        // §4.4: one attempt on the first unmarked tract.
        g.day.set(11);
        send(p, msg::SIM_NEW_DAY);
        let grows = g.grows.borrow().clone();
        assert_eq!(grows.len(), 1);
        assert_eq!(
            (grows[0].developer, grows[0].x, grows[0].z, grows[0].stage),
            (DEV_BASE, 3, 4, 2)
        );
        assert_eq!(fc4_development_last_successful_build(p, 0x1010), 11);
        let st = state(p);
        assert_eq!((st.grow_calls, st.built), (1, 1));
        assert_eq!(st.built_by_wealth, [0, 1, 0, 0], "0x1010 is wealth 1");
        assert_eq!(st.built_by_stage[2], 1);
        assert_eq!((st.marked_types, st.marked_cells), (1, 1));
        // The next day: the next tract (round robin).
        send(p, msg::SIM_NEW_DAY);
        assert_eq!((g.grows.borrow()[1].x, g.grows.borrow()[1].z), (5, 6));

        // The budget (§4.4.1): 0 means unlimited.
        fc4_development_set_grow_budget(p, 0);
        assert_eq!(state(p).grow_budget, u32::MAX);
        fc4_development_set_grow_budget(p, 3);
        assert_eq!(state(p).grow_budget, 3);

        assert!(fc4_development_shutdown(p));
        fc4_development_destroy(p);
        fc4_development_process_destroy(pp);
    }
}

#[test]
fn a_message_during_a_call_is_queued_and_slots_run_at_once() {
    unsafe {
        let g = Game::default();
        g.census.set(1000.0);
        let pp = fc4_development_process_create(7);
        let p = new_bds(&g, pp, 0xB0);
        assert!(fc4_development_init(p));
        {
            let mut devs = g.devs.borrow_mut();
            let d = devs.get_mut(&DEV_BASE).unwrap();
            d.active = 50.0;
            d.growth = vec![fc4_growth_item {
                x: 1,
                z: 1,
                max_stage: 1,
            }];
        }
        g.target.set(p as usize);
        send(p, msg::SIM_NEW_DAY);
        g.target.set(0);
        // The queued cheat ran after the outer call; the claim at once.
        assert!(fc4_development_cheat(p, 1));
        assert_eq!(g.node.borrow().1, 5.0);
        assert_eq!(state(p).reentrant_calls, 2);
        assert!(fc4_development_shutdown(p));
        fc4_development_destroy(p);
        fc4_development_process_destroy(pp);
    }
}

#[test]
fn suitability_and_construction_resources() {
    unsafe {
        let g = Game::default();
        let pp = fc4_development_process_create(7);
        let p = new_bds(&g, pp, 0xB0);
        assert!(fc4_development_init(p));
        let mut prof = fc4_construction_profile {
            zone: 1,
            wealth: 1,
            purpose: 1,
            x: 2,
            z: 3,
            ..Default::default()
        };
        let mut reason = -1;
        g.zone.set(4);
        assert!(!fc4_development_is_suitable(p, &prof, &mut reason));
        assert_eq!(reason, 0);
        g.zone.set(2);
        g.lot_state.set(3);
        assert!(!fc4_development_is_suitable(p, &prof, &mut reason));
        assert_eq!(reason, 4);
        g.lot_state.set(1);
        reason = -1;
        assert!(fc4_development_is_suitable(p, &prof, &mut reason));
        assert_eq!(reason, -1, "untouched on success");
        prof.wealth = 2;
        assert!(!fc4_development_is_suitable(p, &prof, &mut reason));
        assert_eq!(reason, 2);

        // §4.6 with the node's supply 100.
        assert_eq!(fc4_development_free_resources(p), 100);
        assert!(fc4_development_claim(p, 60));
        assert!(!fc4_development_claim(p, 60));
        assert_eq!(fc4_development_free_resources(p), 40);
        assert!(fc4_development_release_resources(p, 60));
        assert_eq!(fc4_development_free_resources(p), 100);
        assert_eq!(fc4_development_stage_population_multiplier(p), 1.0);
        assert!(fc4_development_shutdown(p));
        fc4_development_destroy(p);
        fc4_development_process_destroy(pp);
    }
}

#[test]
fn save_and_load_keep_the_references() {
    unsafe {
        let g = Game::default();
        g.census.set(1000.0);
        let pp = fc4_development_process_create(7);
        // A landmark (group 0x150A) arrives.
        unsafe extern "C" fn in_group(_u: *mut c_void, _o: u64, grp: u32) -> bool {
            grp == 0x150A
        }
        let mut host = bds_host(&g);
        host.occupant_in_group = Some(in_group);
        let p_l = fc4_development_create(&host, pp, 0xB2);
        assert!(fc4_development_init(p_l));
        let m = fc4_development_message {
            msg_type: msg::OCCUPANT_INSERTED,
            occupant: OCC,
            ..Default::default()
        };
        assert!(fc4_development_do_message(p_l, &m, std::ptr::null(), 0));
        assert_eq!(g.refs_of(OCC), 1);
        let mut specials: Vec<u64> = Vec::new();
        unsafe extern "C" fn push_tok(ctx: *mut c_void, t: u64) {
            (*(ctx as *mut Vec<u64>)).push(t);
        }
        let _: fc4_token_callback = Some(push_tok);
        assert_eq!(
            fc4_development_special_buildings(p_l, Some(push_tok), &mut specials as *mut _ as *mut c_void),
            1
        );
        assert_eq!(specials, vec![OCC]);

        let mut out = Out::default();
        let os = ostream(&mut out);
        assert!(fc4_development_write(p_l, &os));
        let map = fc4_development_building_map(p_l);
        let dev = fc4_development_developer(p_l, 0x1010);
        // grid, node, developer, special building.
        assert_eq!(
            out.refs,
            vec![
                (map, 0x49B9_E602),
                (NODE, CLSID_DEMAND_NODE),
                (dev, 0x0990_C138),
                (OCC, 0)
            ]
        );
        assert!(fc4_development_shutdown(p_l));
        assert_eq!(g.refs_of(OCC), 0);

        // Read into a fresh BDS: every reference arrives with one reference
        // for the reader (the game's stream protocol).
        let bump = |r: &mut crate::stream::fc4_ref| {
            if g.grids.borrow().contains_key(&r.token) {
                g.grids.borrow_mut().get_mut(&r.token).unwrap().refs += 1;
            } else {
                *g.refs.borrow_mut().entry(r.token).or_insert(0) += 1;
            }
        };
        let mut i = In::new(&out.bytes);
        i.resolve = Some(&bump);
        let is = istream(&mut i);
        let q = fc4_development_create(&host, pp, 0xB3);
        assert!(fc4_development_read(q, &is));
        assert_eq!(i.pos, out.bytes.len());
        let st = state(q);
        assert_eq!(st.loaded, 1);
        assert_eq!((st.developers, st.special_buildings), (1, 1));
        assert_eq!(st.building_map, map);
        assert_eq!(st.construction_node, NODE);
        assert_eq!(g.refs_of(dev), 1);
        assert_eq!(g.refs_of(OCC), 1);
        assert_eq!(g.refs_of(NODE), 1);

        // A loaded object never initialised releases what it read.
        fc4_development_destroy(q);
        assert_eq!(g.refs_of(dev), 0);
        assert_eq!(g.refs_of(OCC), 0);
        assert_eq!(g.refs_of(NODE), 0);
        assert_eq!(g.grid_refs(map), 0);

        // Read again and Init: the loaded developer is re-Inited, not
        // replaced; the loaded node is only Inited.
        let mut i = In::new(&out.bytes);
        i.resolve = Some(&bump);
        let is = istream(&mut i);
        let q = fc4_development_create(&host, pp, 0xB4);
        assert!(fc4_development_read(q, &is));
        let inits = g.devs.borrow()[&dev].inits;
        let created = g.next_dev.get();
        let supply = g.node.borrow().0;
        assert!(fc4_development_init(q));
        assert_eq!(g.devs.borrow()[&dev].inits, inits + 1);
        assert_eq!(g.next_dev.get(), created);
        assert_eq!(g.node.borrow().0, supply);
        assert!(fc4_development_shutdown(q));
        fc4_development_destroy(q);
        assert_eq!(g.refs_of(dev), 0);

        fc4_development_destroy(p_l);
        fc4_development_process_destroy(pp);
        assert_eq!(g.live_grids(), 0);
    }
}

// ---- growth developer host ------------------------------------------------------------------------------

unsafe extern "C" fn gd_acquire(u: *mut c_void, _s: u32) -> bool {
    game(u).services.set(game(u).services.get() + 1);
    true
}
unsafe extern "C" fn gd_release_service(u: *mut c_void, s: u32) {
    game(u).services.set(game(u).services.get() - 1);
    game(u).released_services.borrow_mut().push(s);
}
unsafe extern "C" fn gd_true(_u: *mut c_void) -> bool {
    true
}
unsafe extern "C" fn gd_add_agent(u: *mut c_void, level: u32, _p: *const c_char, _n: usize, _f: u32) {
    game(u).agents_g.borrow_mut().push(level);
}
unsafe extern "C" fn gd_remove_agent(u: *mut c_void) {
    game(u).agents_g.borrow_mut().clear();
}
unsafe extern "C" fn gd_tract_size(u: *mut c_void, t: u64, s: i32) {
    game(u).tract_sizes.borrow_mut().push((t, s));
}
unsafe extern "C" fn gd_get_demand(_u: *mut c_void, id: u32, _i: u32) -> u64 {
    0xA000 + u64::from(id)
}
unsafe extern "C" fn gd_zone(_u: *mut c_void, _x: i32, _z: i32) -> i8 {
    1
}
unsafe extern "C" fn gd_max_capacity(_u: *mut c_void, _z: i32, _p: u32, _w: u32) -> f32 {
    10.0
}
unsafe extern "C" fn gd_expired(_u: *mut c_void) -> bool {
    false
}

fn growth_host(g: &Game) -> fc4_growth_host {
    let mut h: fc4_growth_host = unsafe { std::mem::zeroed() };
    h.struct_size = std::mem::size_of::<fc4_growth_host>() as u32;
    h.user = g as *const Game as *mut c_void;
    h.grids = grid_api(g);
    h.acquire_service = Some(gd_acquire);
    h.release_service = Some(gd_release_service);
    h.acquire_message_server = Some(gd_true);
    h.subscribe = Some(b_subscribe);
    h.unsubscribe = Some(b_unsubscribe);
    h.simulator_present = Some(gd_true);
    h.add_agent = Some(gd_add_agent);
    h.remove_agent = Some(gd_remove_agent);
    h.sim_date_number = Some(b_day);
    h.grid_set_tract_size = Some(gd_tract_size);
    h.add_ref = Some(b_add_ref);
    h.release = Some(b_release);
    h.get_demand = Some(gd_get_demand);
    h.zone_type = Some(gd_zone);
    h.max_capacity = Some(gd_max_capacity);
    h.limit_timer_expired = Some(gd_expired);
    h
}

fn exemplar_props(t: u32) -> *mut fc4_props {
    Box::into_raw(Box::new(shipped_exemplar(t).unwrap())) as *mut fc4_props
}

unsafe fn gstate(p: *const fc4_growth) -> fc4_growth_state {
    let mut st = fc4_growth_state::default();
    assert!(fc4_growth_get_state(p, &mut st));
    st
}

#[test]
fn growth_developer_init_creates_its_grids_and_registers() {
    unsafe {
        let g = Game::default();
        let host = growth_host(&g);
        let st = fc4_growth_statics_create(3);
        let d = fc4_growth_create(&host, st, 0xD1);
        assert!(!d.is_null());
        assert!(
            !fc4_growth_init(d, std::ptr::null_mut()),
            "a null holder fails (§1.2 [choice])"
        );
        assert!(fc4_growth_init(d, exemplar_props(0x1010)));
        assert_eq!(g.services.get(), 6);
        assert_eq!(*g.agents_g.borrow(), vec![10, 4]);
        assert_eq!(*g.subs.borrow(), vec![0xE8BD_A2E7, 0x0769_9455]);
        assert_eq!(g.live_grids(), 6);
        let s = gstate(d);
        assert_eq!(s.developer_type, 0x1010);
        assert_eq!((s.purpose, s.wealth, s.target_zone), (1, 1, 1));
        // Instances: developer type + the offsets of §1.2.
        let dm = fc4_growth_grid(d, 0);
        assert_eq!(g.grids.borrow()[&dm].instance, 0x1010 + 0x0200_0000);
        // The shipped tract size differs from the constructed 64: every grid
        // gets SetTractSize.
        assert_eq!(g.tract_sizes.borrow().len(), 6);
        assert_eq!(fc4_growth_sub_type(d), 0x1010);
        assert_eq!(fc4_growth_source_demand(d), 0xA000 + 0x1010);
        assert_eq!(g.refs_of(0xA000 + 0x1010), 1);
        let mut prof = fc4_construction_profile {
            building_type: 0x55,
            x: 9,
            ..Default::default()
        };
        fc4_growth_construction_info(d, &mut prof);
        assert_eq!((prof.zone, prof.wealth, prof.purpose), (1, 1, 1));
        assert_eq!((prof.building_type, prof.x), (0x55, 9), "the rest untouched");

        assert!(fc4_growth_shutdown(d));
        assert_eq!(g.services.get(), 0);
        assert_eq!(g.live_grids(), 0);
        assert!(g.grids.borrow().values().all(|x| x.shut));
        assert_eq!(g.refs_of(0xA000 + 0x1010), 0);
        fc4_growth_destroy(d);
        fc4_growth_statics_destroy(st);
    }
}

#[test]
fn growth_developer_map_update_runs_to_the_end() {
    unsafe {
        let g = Game::default();
        let host = growth_host(&g);
        let st = fc4_growth_statics_create(3);
        let d = fc4_growth_create(&host, st, 0xD1);
        assert!(fc4_growth_init(d, exemplar_props(0x1010)));
        // ZoningChange over the city: every tract zoned.
        let z = fc4_growth_message {
            msg_type: 0x0769_9455,
            rect: [0, 0, CELLS - 1, CELLS - 1],
            ..Default::default()
        };
        assert!(fc4_growth_do_message(d, &z));
        fc4_growth_start_map_update(d);
        assert!(!fc4_growth_map_update_finished(d));
        let idle = fc4_growth_message {
            msg_type: 0x6695_681A,
            data3: 5,
            ..Default::default()
        };
        for _ in 0..200 {
            if fc4_growth_map_update_finished(d) {
                break;
            }
            assert!(fc4_growth_do_message(d, &idle));
        }
        assert!(fc4_growth_map_update_finished(d));
        assert!(fc4_growth_lists_valid(d));
        // D is W published (no sources: every tract at the baseline).
        let dm = fc4_growth_grid(d, 0);
        let w = fc4_growth_grid(d, 5);
        assert_eq!(g.grids.borrow()[&dm].data, g.grids.borrow()[&w].data);
        let n = fc4_growth_list(d, 0, std::ptr::null_mut(), 0);
        let mut items = vec![fc4_growth_item::default(); n];
        assert_eq!(fc4_growth_list(d, 0, items.as_mut_ptr(), n), n);
        let s = gstate(d);
        assert_eq!(s.growth_items as usize, n);
        assert_eq!(s.update_state, 0);
        assert!(fc4_growth_shutdown(d));
        fc4_growth_destroy(d);
        fc4_growth_statics_destroy(st);
    }
}

#[test]
fn growth_developer_save_and_load_keep_the_grid_references() {
    unsafe {
        let g = Game::default();
        let host = growth_host(&g);
        let st = fc4_growth_statics_create(3);
        let d = fc4_growth_create(&host, st, 0xD1);
        assert!(fc4_growth_init(d, exemplar_props(0x1010)));
        let mut out = Out::default();
        let os = ostream(&mut out);
        assert!(fc4_growth_write(d, &os));
        assert_eq!(out.refs.len(), 5);
        let saved: Vec<u64> = out.refs.iter().map(|r| r.0).collect();
        assert!(fc4_growth_shutdown(d));
        fc4_growth_destroy(d);

        // The game keeps the grids alive for the load (its own references).
        for t in &saved {
            g.grids.borrow_mut().get_mut(t).unwrap().refs = 1;
        }
        let bump = |r: &mut crate::stream::fc4_ref| {
            if let Some(x) = g.grids.borrow_mut().get_mut(&r.token) {
                x.refs += 1;
            }
        };
        let mut i = In::new(&out.bytes);
        i.resolve = Some(&bump);
        let is = istream(&mut i);
        let e = fc4_growth_create(&host, st, 0xD2);
        assert!(fc4_growth_read(e, &is));
        assert_eq!(fc4_growth_grid(e, 0), saved[1], "D");
        for t in &saved {
            assert_eq!(g.grid_refs(*t), 2);
        }
        // Never initialised: destroying releases what it read.
        fc4_growth_destroy(e);
        for t in &saved {
            assert_eq!(g.grid_refs(*t), 1);
        }
        fc4_growth_statics_destroy(st);
    }
}
