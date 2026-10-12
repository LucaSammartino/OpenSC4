// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The demand C ABI against a fake game: the graph built from demand
//! exemplars through fc4_props with nodes created by the host, propagation and
//! ActiveDemandChanged posts, the interface queries, an occupancy message, and
//! persistence of the simulator (node references) and its nodes.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::c_void;

use crate::budget::fc4_token_callback;
use crate::demand::*;
use crate::ordinance::fc4_key_visitor;
use crate::props::*;
use crate::stream::fc4_ref;
use crate::test_streams::{istream, ostream, In, Out};

const EXEMPLAR: u32 = 0x6534_284A;
const GROUP: u32 = 0xC7BB_4816;
const NODE_CLASS: u32 = 0x8990_C12D;
const PRE_SAVE: u32 = 0x26C6_3343;
const ACTIVE_DEMAND_CHANGED_KEYS: [u32; 2] = [0x2010, 0x3121];
const BUILDING: u32 = 0xBEEF;

/// (instance, [(property, u32 values)]).
type Exemplar = (u32, Vec<(u32, Vec<u32>)>);

#[derive(Default)]
struct Game {
    host: Cell<Option<fc4_demand_host>>,
    exemplars: RefCell<Vec<Exemplar>>,
    /// token → (core node, references).
    nodes: RefCell<BTreeMap<u64, (usize, i64)>>,
    next: Cell<u64>,
    posts: RefCell<Vec<(u64, u32, i32)>>,
    history: RefCell<Vec<(u32, i32)>>,
    queue_lengths: Cell<u32>,
    warnings: Cell<u32>,
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

fn holder(props: &[(u32, Vec<u32>)]) -> *mut fc4_props {
    let p = fc4_props_create();
    for (id, v) in props {
        assert!(unsafe { fc4_props_set_u32s(p, *id, v.as_ptr(), v.len()) });
    }
    p
}

unsafe extern "C" fn for_each_key(u: *mut c_void, visit: fc4_key_visitor, ctx: *mut c_void) {
    let keys: Vec<u32> = game(u).exemplars.borrow().iter().map(|e| e.0).collect();
    for i in keys {
        visit.unwrap()(ctx, EXEMPLAR, GROUP, i);
    }
    visit.unwrap()(ctx, 0x1234_5678, GROUP, 1); // not an exemplar: skipped by the core
}

unsafe extern "C" fn exemplar(u: *mut c_void, t: u32, g: u32, i: u32) -> *mut fc4_props {
    if t != EXEMPLAR || g != GROUP {
        return std::ptr::null_mut();
    }
    let ex = game(u).exemplars.borrow();
    match ex.iter().find(|e| e.0 == i) {
        Some((_, props)) => holder(props),
        None => std::ptr::null_mut(),
    }
}

unsafe extern "C" fn warn(u: *mut c_void, _: *const u8, _: usize) {
    game(u).warnings.set(game(u).warnings.get() + 1);
}

unsafe extern "C" fn create_node(u: *mut c_void, token: *mut u64) -> *mut fc4_demand_node {
    let g = game(u);
    g.next.set(g.next.get() + 1);
    let t = 0x100 + g.next.get();
    let host = g.host.get().unwrap();
    let n = fc4_demand_node_create(&host, t);
    assert!(!n.is_null());
    g.nodes.borrow_mut().insert(t, (n as usize, 1));
    *token = t;
    n
}

unsafe extern "C" fn add_ref(u: *mut c_void, t: u64) {
    if let Some(e) = game(u).nodes.borrow_mut().get_mut(&t) {
        e.1 += 1;
    }
}

unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    if let Some(e) = game(u).nodes.borrow_mut().get_mut(&t) {
        e.1 -= 1;
    }
}

unsafe extern "C" fn post(u: *mut c_void, node: u64, key: u32, data2: i32) {
    game(u).posts.borrow_mut().push((node, key, data2));
}

unsafe extern "C" fn cell_counts(_: *mut c_void, x: *mut i32, z: *mut i32) {
    *x = 4;
    *z = 4;
}

unsafe extern "C" fn all_building_types(_: *mut c_void, cb: fc4_u32_callback, ctx: *mut c_void) {
    cb.unwrap()(ctx, BUILDING);
}

unsafe extern "C" fn building_exemplar(_: *mut c_void, bt: u32) -> *mut fc4_props {
    // 0x27812841: creates 5 demand on 0x2010; a pair for a missing node is dropped.
    if bt == BUILDING {
        holder(&[(0x2781_2841, vec![0x2010, 5, 0x9999, 1])])
    } else {
        std::ptr::null_mut()
    }
}

unsafe extern "C" fn connected(_: *mut c_void, cb: fc4_token_callback, ctx: *mut c_void) {
    cb.unwrap()(ctx, 0xC17);
}

unsafe extern "C" fn extrapolated(_: *mut c_void, city: u64, pop_type: u32) -> i32 {
    if city == 0xC17 && pop_type == 0x3121 {
        250
    } else {
        0
    }
}

unsafe extern "C" fn record_history(u: *mut c_void, s: u32, v: i32) {
    game(u).history.borrow_mut().push((s, v));
}

unsafe extern "C" fn queue_length(u: *mut c_void, _: u32, _: u32) {
    game(u).queue_lengths.set(game(u).queue_lengths.get() + 1);
}

fn make_game() -> Box<Game> {
    let g = Box::new(Game::default());
    let host = fc4_demand_host {
        struct_size: std::mem::size_of::<fc4_demand_host>() as u32,
        user: &*g as *const Game as *mut c_void,
        for_each_exemplar_key: Some(for_each_key),
        exemplar: Some(exemplar),
        warn: Some(warn),
        create_node: Some(create_node),
        add_ref: Some(add_ref),
        release: Some(release),
        post_active_demand: Some(post),
        cell_counts: Some(cell_counts),
        today: None,
        now_ms: None,
        census_lot: None,
        building_profile: None,
        developer_type: None,
        desirability: None,
        lot_capacity: None,
        cell_eq: None,
        cell_workforce_percent: None,
        lot_building_type: None,
        lot_excluded: None,
        all_building_types: Some(all_building_types),
        building_exemplar: Some(building_exemplar),
        history_max: None,
        ordinance_factor: None,
        nation_economy_modifier: None,
        connections: None,
        traffic_arrived: None,
        airport_trips: None,
        seaport_trips: None,
        all_connected_cities: Some(connected),
        connected_cities: Some(connected),
        own_regional_city: None,
        city_population: None,
        city_extrapolated_population: Some(extrapolated),
        city_allowable_extrapolation: None,
        city_tax_rate: None,
        city_workforce_percentage: None,
        city_extrapolate_growth: None,
        growth_developers: None,
        day_of_last_successful_build: None,
        record_history: Some(record_history),
        set_history_queue_length: Some(queue_length),
    };
    g.host.set(Some(host));
    // 0x3121 (census, quantity in supply) drives 68 % into 0x2010; 0x2010 is
    // bounded ±6000; 0x13121 is 0x3121's regional node.
    *g.exemplars.borrow_mut() = vec![
        (
            0x3121,
            vec![(0x10, vec![3]), (0x47BB_3F30, vec![0x2010, 68]), (0x47BB_3F60, vec![1])],
        ),
        (
            0x2010,
            vec![
                (0x10, vec![3]),
                (0x47BB_3F10, vec![(-6000i32) as u32]),
                (0x47BB_3F11, vec![6000]),
            ],
        ),
        (0x13121, vec![(0x10, vec![3])]),
        (0x5555, vec![(0x10, vec![14])]), // another exemplar type: not a node
    ];
    g
}

fn node_handle(g: &Game, token: u64) -> *mut fc4_demand_node {
    g.nodes
        .borrow()
        .get(&token)
        .map_or(std::ptr::null_mut(), |e| e.0 as *mut fc4_demand_node)
}

fn state(s: *const fc4_demand_sim) -> fc4_demand_sim_state {
    let mut st = fc4_demand_sim_state {
        struct_size: std::mem::size_of::<fc4_demand_sim_state>() as u32,
        ..Default::default()
    };
    assert!(unsafe { fc4_demand_sim_get_state(s, &mut st) });
    st
}

fn destroy_nodes(g: &Game) {
    for (h, _) in g.nodes.borrow_mut().values() {
        unsafe { fc4_demand_node_destroy(*h as *mut fc4_demand_node) };
    }
}

#[test]
fn graph_propagation_queries_and_occupancy() {
    let g = make_game();
    unsafe {
        let host = g.host.get().unwrap();
        let s = fc4_demand_sim_create(&host, 0x5000);
        assert!(!s.is_null());
        assert!(fc4_demand_sim_init(s));
        let st = state(s);
        assert_eq!(
            (st.initialized, st.node_count, st.link_count, st.census_state),
            (1, 3, 1, 3)
        );
        assert_eq!(g.queue_lengths.get(), 12); // the 12 daily series (§10.4)

        // GetDemand: tokens, no reference added.
        let t2010 = fc4_demand_sim_get_demand(s, 0x2010, 0);
        let t3121 = fc4_demand_sim_get_demand(s, 0x3121, 0);
        let t13121 = fc4_demand_sim_get_demand(s, 0x3121, 0x10000);
        assert!(t2010 != 0 && t3121 != 0 && t13121 != 0);
        assert_eq!(fc4_demand_sim_get_demand(s, 0x4242, 0), 0);
        assert!(g.nodes.borrow().values().all(|e| e.1 == 1));
        let (n2010, n3121, n13121) = (node_handle(&g, t2010), node_handle(&g, t3121), node_handle(&g, t13121));
        assert_eq!(fc4_demand_node_id(n2010), 0x2010);
        assert_eq!(fc4_demand_node_get(n2010, value::ACTIVE_MAX), 6000.0);
        assert_eq!(fc4_demand_node_region_use(n3121), 1);

        // ResetRegionalDemands(true): the neighbour's extrapolated 0x3121 on 0x13121
        // (folded by Init's propagation).
        assert_eq!(fc4_demand_node_get(n13121, value::SUPPLY), 250.0);

        // Inject supply, propagate (PreSave runs DoDemandIndexLinks twice).
        assert!(fc4_demand_node_add(n3121, value::SUPPLY, 1000.0));
        assert_eq!(fc4_demand_node_get(n3121, value::NEW_SUPPLY), 1000.0);
        g.posts.borrow_mut().clear();
        assert!(fc4_demand_sim_handle_message(s, PRE_SAVE, 0, 0, 0));
        assert_eq!(fc4_demand_node_get(n3121, value::SUPPLY), 1000.0);
        assert_eq!(fc4_demand_node_get(n2010, value::DEMAND), 680.0);
        // ActiveDemandChanged from both nodes, with their tokens; 680 of 6000 -> 113.
        for key in ACTIVE_DEMAND_CHANGED_KEYS {
            assert!(g.posts.borrow().iter().any(|p| p.1 == key));
        }
        assert!(g.posts.borrow().contains(&(t2010, 0x2010, 113)));

        // Interface queries (§5).
        let mut j = [9.0f32; 3];
        fc4_demand_sim_jobs_per_unit(s, 0x3121, 0, j.as_mut_ptr());
        assert_eq!(j, [0.68, 0.0, 0.0]);
        assert_eq!(fc4_demand_sim_jobs_by_census(s, 0x3121), 680);
        assert_eq!(fc4_demand_sim_jobs_by_census(s, 0x1011), 0);
        assert_eq!(fc4_demand_sim_neutral_tax_rate(s), 0.0);
        let mut summary: Vec<(u32, i32)> = Vec::new();
        unsafe extern "C" fn visit(ctx: *mut c_void, k: u32, v: i32) {
            (*(ctx as *mut Vec<(u32, i32)>)).push((k, v));
        }
        let n = fc4_demand_sim_local_population_summary(s, Some(visit), &mut summary as *mut _ as *mut c_void);
        assert_eq!((n, summary), (1, vec![(0x3121, 1000)]));

        // Occupancy (§7): 1 -> 3 adds the building's effects; 3 -> 2 does nothing.
        fc4_demand_sim_state_changed(s, 0x77, 1, 3, BUILDING, 1);
        assert_eq!(fc4_demand_node_get(n2010, value::NEW_DEMAND), 5.0);
        fc4_demand_sim_state_changed(s, 0x77, 3, 2, BUILDING, 1);
        assert_eq!(fc4_demand_node_get(n2010, value::NEW_DEMAND), 5.0);
        fc4_demand_sim_update_occupant_effects(s, BUILDING, 1.0, 0.5);
        assert_eq!(fc4_demand_node_get(n2010, value::NEW_DEMAND), 2.5);

        // EndOfCycle folds and reports pending values.
        assert!(fc4_demand_node_end_of_cycle(n2010));
        assert!(!fc4_demand_node_end_of_cycle(n2010));
        assert_eq!(fc4_demand_node_get(n2010, value::DEMAND), 682.5);
        assert!(fc4_demand_node_set(n2010, value::DEMAND_CAP, 0.5));
        assert!(!fc4_demand_node_set(n2010, value::NEW_SUPPLY, 1.0));
        assert_eq!(fc4_demand_node_get(n2010, value::DEMAND_CAP), 0.5);

        // Shutdown releases the map's references (the nodes were created with one).
        assert!(fc4_demand_sim_shutdown(s));
        assert!(g.nodes.borrow().values().all(|e| e.1 == 0));
        assert_eq!(state(s).node_count, 0);
        fc4_demand_sim_destroy(s);
        destroy_nodes(&g);
    }
}

#[test]
fn save_round_trip() {
    let g = make_game();
    unsafe {
        let host = g.host.get().unwrap();
        let s = fc4_demand_sim_create(&host, 0x5000);
        assert!(fc4_demand_sim_init(s));
        fc4_demand_sim_set_economy_bonuses(s, 0.25, 0.5);
        let t3121 = fc4_demand_sim_get_demand(s, 0x3121, 0);
        assert!(fc4_demand_node_add(node_handle(&g, t3121), value::SUPPLY, 40.0));

        let mut out = Out::default();
        assert!(fc4_demand_sim_write(s, &ostream(&mut out)));
        // version, 4 floats, count, 3 × (key, ref), census count 24 × (key, value).
        assert_eq!(out.refs.len(), 3);
        assert!(out.refs.iter().all(|r| r.1 == NODE_CLASS));
        assert_eq!(out.bytes.len(), 2 + 16 + 4 + 3 * (4 + 12) + 4 + 24 * 8);

        // Node records: write -> read -> write is byte-identical.
        let mut node_out = Out::default();
        assert!(fc4_demand_node_write(node_handle(&g, t3121), &ostream(&mut node_out)));
        assert_eq!(node_out.bytes.len(), 55);
        let copy = fc4_demand_node_create(&host, 0x9000);
        let mut i = In::new(&node_out.bytes);
        assert!(fc4_demand_node_read(copy, &istream(&mut i)));
        let mut again = Out::default();
        assert!(fc4_demand_node_write(copy, &ostream(&mut again)));
        assert_eq!(again.bytes, node_out.bytes);
        assert_eq!(fc4_demand_node_get(copy, value::NEW_SUPPLY), 40.0);
        fc4_demand_node_destroy(copy);

        // The simulator record into a fresh simulator: references resolve to the
        // same (our) nodes; the map takes one reference each; a foreign object is
        // released and skipped.
        let handles: BTreeMap<u64, usize> = g.nodes.borrow().iter().map(|(t, e)| (*t, e.0)).collect();
        let resolve = |r: &mut fc4_ref| {
            if let Some(h) = handles.get(&r.token) {
                r.core_handle = *h as *mut c_void;
            }
        };
        let s2 = fc4_demand_sim_create(&host, 0x6000);
        let mut i = In::new(&out.bytes);
        i.resolve = Some(&resolve);
        assert!(fc4_demand_sim_read(s2, &istream(&mut i)));
        let st = state(s2);
        assert_eq!((st.loaded_from_save, st.node_count), (1, 3));
        assert_eq!((st.co_economy_bonus, st.i_economy_bonus), (0.25, 0.5));
        assert!(g.nodes.borrow().values().all(|e| e.1 == 2));
        let mut out2 = Out::default();
        assert!(fc4_demand_sim_write(s2, &ostream(&mut out2)));
        assert_eq!(out2.bytes, out.bytes);

        // Version 3 is rejected.
        let mut bad = out.bytes.clone();
        bad[0] = 3;
        let s3 = fc4_demand_sim_create(&host, 0x7000);
        let mut i = In::new(&bad);
        assert!(!fc4_demand_sim_read(s3, &istream(&mut i)));
        fc4_demand_sim_destroy(s3);

        fc4_demand_sim_destroy(s2);
        fc4_demand_sim_destroy(s);
        assert!(g.nodes.borrow().values().all(|e| e.1 == 0));
        destroy_nodes(&g);
    }
}

#[test]
fn null_handles_and_short_tables() {
    unsafe {
        let mut h: fc4_demand_host = std::mem::zeroed();
        h.struct_size = 8;
        assert!(fc4_demand_sim_create(&h, 1).is_null());
        assert!(fc4_demand_node_create(&h, 1).is_null());
        assert!(fc4_demand_sim_create(std::ptr::null(), 1).is_null());
        assert!(!fc4_demand_sim_init(std::ptr::null_mut()));
        assert_eq!(fc4_demand_sim_get_demand(std::ptr::null_mut(), 1, 0), 0);
        assert!(fc4_demand_sim_handle_message(std::ptr::null_mut(), PRE_SAVE, 0, 0, 0));
        assert_eq!(fc4_demand_node_get(std::ptr::null(), value::SUPPLY), 0.0);
        assert!(!fc4_demand_node_end_of_cycle(std::ptr::null_mut()));
        let mut j = [1.0f32; 3];
        fc4_demand_sim_jobs_per_unit(std::ptr::null(), 1, 0, j.as_mut_ptr());
        assert_eq!(j, [0.0; 3]);
        // A node without a simulator works on its own (the class store creates nodes so).
        h.struct_size = std::mem::size_of::<fc4_demand_host>() as u32;
        let n = fc4_demand_node_create(&h, 0x42);
        assert!(fc4_demand_node_init(n));
        assert!(fc4_demand_node_set_id(n, 0x1010));
        assert!(fc4_demand_node_set(n, value::ACTIVE_MAX, 100.0));
        assert!(fc4_demand_node_set(n, value::ACTIVE_MIN, 200.0));
        assert_eq!(fc4_demand_node_get(n, value::ACTIVE_MIN), 100.0);
        assert!(fc4_demand_node_simulation_begin(n)); // posts nowhere: no callback
        fc4_demand_node_destroy(n);
    }
}
