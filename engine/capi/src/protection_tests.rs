// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The protection C ABI against a fake game: the game's grids through
//! `fc4_grid_api`, occupants and their properties, departments, the
//! dispatch manager, the reference protocol, persistence through host
//! streams, re-entrant messages and the grid shadow.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::ffi::{c_char, c_void};

use fc4_core::protection::{msg, prop, shipped_tuning, ProtectionKind, AGENT_NAME, IID_DISPATCH_SOURCE};
use fc4_core::variant::mem;

use crate::hostgrid::fc4_grid_api;
use crate::landvalue::fc4_grid_geometry;
use crate::props::{fc4_props, fc4_props_set_typed};
use crate::protection::*;
use crate::test_streams::{istream, ostream, In, Out};

const TRACTS: i32 = 16;
const STATION: u64 = 0x5_0001;
const STATION2: u64 = 0x5_0002;
const JAIL: u64 = 0x5_0003;
const HOUSE: u64 = 0x5_0004;
const DEPT: u64 = 0xD0;
const JAIL_DEPT: u64 = 0xD1;
const AIR: u64 = 0xA1;
const AUTOMATON: u64 = 0x7_0001;
const BUILDING: u32 = 0x2781_28A0;
const U8_GRID: u32 = 0x49B9_E602;
const I16_GRID: u32 = 0x49B9_E605;

struct Grid {
    kind: u32,
    data: Vec<f64>,
    refs: i32,
    shut: bool,
    instance: u32,
}

/// (element type, array flag, bytes, count).
type Prop = (u16, bool, Vec<u8>, usize);
/// (occupant, id, array flag, values) of a property write.
type Write = (u64, u32, bool, Vec<u32>);
/// (id, group, name group, name instance, max funding) of a created department.
type Department = (u32, u32, u32, u32, f32);

#[derive(Default)]
struct Game {
    grids: RefCell<HashMap<u64, Grid>>,
    next_grid: Cell<u64>,
    refs: RefCell<HashMap<u64, i32>>,
    props: RefCell<HashMap<(u64, u32), Prop>>,
    writes: RefCell<Vec<Write>>,
    types: RefCell<HashMap<u64, u32>>,
    pos: RefCell<HashMap<u64, [f32; 3]>>,
    agents: RefCell<Vec<(String, Vec<u32>, u32)>>,
    subs: RefCell<Vec<u32>>,
    posts: RefCell<Vec<(u32, u64)>>,
    history: RefCell<Vec<(u32, i32)>>,
    departments: RefCell<Vec<Department>>,
    funded: RefCell<Vec<(u64, u64, u32, bool)>>,
    services: Cell<i32>,
    registered: Cell<i32>,
    air: RefCell<Vec<&'static str>>,
    dispatches: RefCell<Vec<(u32, fc4_protection_dispatch_info)>>,
    capacities: RefCell<Vec<u32>>,
    /// Grids the game made for a saved reference whose own record it has not
    /// read yet: their geometry is still 0 × 0 (the in-game fire grid at
    /// fire's `Read`).
    unread: RefCell<HashSet<u64>>,
    /// Re-entrancy test: post() delivers a day message to this handle.
    target: Cell<usize>,
    reenter: Cell<u32>,
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

impl Game {
    fn refs_of(&self, t: u64) -> i32 {
        self.refs.borrow().get(&t).copied().unwrap_or(0)
    }
    fn set_raw<T: Copy>(&self, o: u64, id: u32, mem_type: u16, v: T) {
        let bytes =
            unsafe { std::slice::from_raw_parts(&v as *const T as *const u8, std::mem::size_of::<T>()) }.to_vec();
        self.props.borrow_mut().insert((o, id), (mem_type, false, bytes, 1));
    }
    /// A station building: centre, outer, radius (metres), position, units.
    #[allow(clippy::too_many_arguments)]
    fn station(&self, o: u64, kind: ProtectionKind, centre: i32, outer: i32, radius: f32, at: [f32; 3], units: u32) {
        let p = kind.params();
        self.types.borrow_mut().insert(o, BUILDING);
        self.set_raw(o, p.effect_props[0], mem::SINT32, centre);
        self.set_raw(o, p.effect_props[1], mem::SINT32, outer);
        self.set_raw(o, p.effect_props[2], mem::FLOAT32, radius);
        self.set_raw(o, p.unit_count_prop, mem::UINT32, units);
        self.pos.borrow_mut().insert(o, at);
    }
    fn prop_u32(&self, o: u64, id: u32) -> Option<u32> {
        self.props
            .borrow()
            .get(&(o, id))
            .map(|(_, _, b, _)| u32::from_ne_bytes(b[..4].try_into().unwrap()))
    }
    fn grid_values(&self, t: u64) -> Vec<f64> {
        self.grids.borrow()[&t].data.clone()
    }
}

// ---- grids --------------------------------------------------------------------------------------------

unsafe extern "C" fn g_create(u: *mut c_void, kind: u32, instance: u32, ts: i32) -> u64 {
    assert!(kind == U8_GRID || kind == I16_GRID);
    assert_eq!(ts, 2);
    let g = game(u);
    let t = g.next_grid.get() + 0x7000;
    g.next_grid.set(g.next_grid.get() + 0x10);
    g.grids.borrow_mut().insert(
        t,
        Grid {
            kind,
            data: vec![0.0; (TRACTS * TRACTS) as usize],
            refs: 1,
            shut: false,
            instance,
        },
    );
    t
}
unsafe extern "C" fn g_class(u: *mut c_void, t: u64) -> u32 {
    game(u).grids.borrow().get(&t).map_or(0, |g| g.kind)
}
fn geometry() -> fc4_grid_geometry {
    fc4_grid_geometry {
        tract_size: 2,
        shift: 1,
        count_x: TRACTS,
        count_z: TRACTS,
        width_x: 32.0,
        width_z: 32.0,
        inv_width_x: 1.0 / 32.0,
        inv_width_z: 1.0 / 32.0,
    }
}
unsafe extern "C" fn g_geometry(u: *mut c_void, t: u64, out: *mut fc4_grid_geometry) {
    *out = if game(u).unread.borrow().contains(&t) {
        fc4_grid_geometry::default()
    } else {
        geometry()
    };
}
unsafe extern "C" fn g_get(u: *mut c_void, t: u64, kind: u32, x: i32, z: i32) -> f64 {
    let grids = game(u).grids.borrow();
    assert_eq!(grids[&t].kind, kind);
    grids[&t].data[(x * TRACTS + z) as usize]
}
unsafe extern "C" fn g_set(u: *mut c_void, t: u64, _kind: u32, x: i32, z: i32, v: f64) {
    game(u).grids.borrow_mut().get_mut(&t).unwrap().data[(x * TRACTS + z) as usize] = v;
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

// ---- lifecycle, references -------------------------------------------------------------------------------

unsafe extern "C" fn acquire(u: *mut c_void, _kind: u32) {
    game(u).services.set(game(u).services.get() + 1);
}
unsafe extern "C" fn release_services(u: *mut c_void, _kind: u32) {
    game(u).services.set(game(u).services.get() - 1);
}
unsafe extern "C" fn add_agent(u: *mut c_void, name: *const c_char, len: usize, levels: *const u32, n: usize, f: u32) {
    let name = String::from_utf8(std::slice::from_raw_parts(name as *const u8, len).to_vec()).unwrap();
    let levels = std::slice::from_raw_parts(levels, n).to_vec();
    game(u).agents.borrow_mut().push((name, levels, f));
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
unsafe extern "C" fn register_source(u: *mut c_void) {
    game(u).registered.set(game(u).registered.get() + 1);
}
unsafe extern "C" fn unregister_source(u: *mut c_void) {
    game(u).registered.set(game(u).registered.get() - 1);
}
unsafe extern "C" fn set_grid_instance(u: *mut c_void, t: u64, instance: u32) {
    game(u).grids.borrow_mut().get_mut(&t).unwrap().instance = instance;
}
unsafe extern "C" fn add_ref(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_insert(0) += 1;
}
unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_insert(0) -= 1;
}
unsafe extern "C" fn tuning(_: *mut c_void, kind: u32) -> *mut fc4_props {
    let k = if kind == 0 {
        ProtectionKind::Fire
    } else {
        ProtectionKind::Police
    };
    Box::into_raw(Box::new(shipped_tuning(k))) as *mut fc4_props
}

// ---- budget ------------------------------------------------------------------------------------------------

unsafe extern "C" fn create_department(u: *mut c_void, id: u32, group: u32, ng: u32, ni: u32, max: f32) -> u64 {
    let g = game(u);
    g.departments.borrow_mut().push((id, group, ng, ni, max));
    let t = if id == ProtectionKind::Police.params().department || id == ProtectionKind::Fire.params().department {
        DEPT
    } else {
        JAIL_DEPT
    };
    *g.refs.borrow_mut().entry(t).or_insert(0) += 1;
    t
}
unsafe extern "C" fn department_id(_: *mut c_void, d: u64) -> u32 {
    match d {
        DEPT => 1,
        JAIL_DEPT => 2,
        _ => 0,
    }
}
unsafe extern "C" fn department_funding(_: *mut c_void, _d: u64) -> f32 {
    1.0
}
unsafe extern "C" fn add_funded(u: *mut c_void, d: u64, o: u64, purpose: u32) {
    game(u).funded.borrow_mut().push((d, o, purpose, true));
}
unsafe extern "C" fn remove_funded(u: *mut c_void, d: u64, o: u64, purpose: u32) {
    game(u).funded.borrow_mut().push((d, o, purpose, false));
}
unsafe extern "C" fn budget_item(_: *mut c_void, o: u64, _purpose: u32, out: *mut fc4_protection_budget_item) -> bool {
    if o != STATION && o != STATION2 && o != JAIL {
        return false;
    }
    *out = fc4_protection_budget_item {
        has_department: 1,
        locally_funded: 0,
        local_funding: 1.0,
        reserved: 0,
        cost: 500,
        department: DEPT,
    };
    true
}

// ---- occupants, city ---------------------------------------------------------------------------------------

unsafe extern "C" fn occupant_type(u: *mut c_void, o: u64) -> u32 {
    game(u).types.borrow().get(&o).copied().unwrap_or(0)
}
unsafe extern "C" fn occupant_property(u: *mut c_void, o: u64, id: u32, p: *mut fc4_props) -> bool {
    match game(u).props.borrow().get(&(o, id)) {
        Some((t, a, b, n)) => fc4_props_set_typed(p, id, *t, *a, b.as_ptr() as *const c_void, *n),
        None => false,
    }
}
unsafe extern "C" fn set_occupant_property(
    u: *mut c_void,
    o: u64,
    id: u32,
    mem_type: u16,
    is_array: bool,
    data: *const c_void,
    count: usize,
) {
    assert_eq!(mem_type, mem::UINT32);
    let values = std::slice::from_raw_parts(data as *const u32, count).to_vec();
    let bytes = std::slice::from_raw_parts(data as *const u8, 4 * count).to_vec();
    let g = game(u);
    g.writes.borrow_mut().push((o, id, is_array, values));
    g.props.borrow_mut().insert((o, id), (mem_type, is_array, bytes, count));
}
unsafe extern "C" fn position(u: *mut c_void, o: u64, out: *mut f32) {
    let p = game(u).pos.borrow().get(&o).copied().unwrap_or([0.0; 3]);
    std::slice::from_raw_parts_mut(out, 3).copy_from_slice(&p);
}
unsafe extern "C" fn road_access(_: *mut c_void, _o: u64) -> i32 {
    1
}
unsafe extern "C" fn sim_day(_: *mut c_void) -> u32 {
    1000
}
unsafe extern "C" fn sea_level(_: *mut c_void) -> f32 {
    250.0
}
unsafe extern "C" fn altitude(_: *mut c_void, _x: f32, _z: f32) -> f32 {
    270.0
}
unsafe extern "C" fn zone_value(_: *mut c_void, _x: i32, _z: i32) -> u32 {
    1
}
unsafe extern "C" fn dispatch_count(u: *mut c_void, t: u32) -> u32 {
    game(u).dispatches.borrow().iter().filter(|(k, _)| *k == t).count() as u32
}
unsafe extern "C" fn dispatch_ids(u: *mut c_void, t: u32, out: *mut u32, cap: u32) -> u32 {
    let g = game(u);
    g.capacities.borrow_mut().push(cap);
    let ids: Vec<u32> = g
        .dispatches
        .borrow()
        .iter()
        .filter(|(k, _)| *k == t)
        .map(|(_, i)| i.call_id)
        .collect();
    let n = ids.len().min(cap as usize);
    std::ptr::copy_nonoverlapping(ids.as_ptr(), out, n);
    n as u32
}
unsafe extern "C" fn dispatch_info(u: *mut c_void, id: u32, out: *mut fc4_protection_dispatch_info) -> bool {
    match game(u).dispatches.borrow().iter().find(|(_, i)| i.call_id == id) {
        Some((_, i)) => {
            *out = *i;
            true
        }
        None => false,
    }
}
unsafe extern "C" fn record_history(u: *mut c_void, id: u32, v: i32) {
    game(u).history.borrow_mut().push((id, v));
}
unsafe extern "C" fn post(u: *mut c_void, t: u32, subject: u64) {
    let g = game(u);
    g.posts.borrow_mut().push((t, subject));
    // A synchronous delivery back into the simulator (re-entrancy test).
    let target = g.target.get();
    if target != 0 && g.reenter.get() == 0 {
        g.reenter.set(1);
        let m = fc4_protection_message {
            msg_type: msg::OCCUPANT_INSERTED,
            occupant: STATION2,
            ..Default::default()
        };
        assert!(fc4_protection_do_message(target as *mut fc4_protection, &m));
    }
}
unsafe extern "C" fn create_air(u: *mut c_void, _kind: u32, class_id: *mut u32) -> u64 {
    let g = game(u);
    g.air.borrow_mut().push("create");
    *g.refs.borrow_mut().entry(AIR).or_insert(0) += 1;
    *class_id = 0;
    AIR
}
unsafe extern "C" fn air_init(u: *mut c_void, _s: u64) {
    game(u).air.borrow_mut().push("init");
}
unsafe extern "C" fn air_shutdown(u: *mut c_void, _s: u64) {
    game(u).air.borrow_mut().push("shutdown");
}

fn grid_api(user: *mut c_void) -> fc4_grid_api {
    fc4_grid_api {
        struct_size: std::mem::size_of::<fc4_grid_api>() as u32,
        user,
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

/// The read-only part of the table (what the grid shadow gets).
fn read_only_host(g: &Game) -> fc4_protection_host {
    let mut h: fc4_protection_host = unsafe { std::mem::zeroed() };
    h.struct_size = std::mem::size_of::<fc4_protection_host>() as u32;
    h.user = g as *const Game as *mut c_void;
    h.tuning = Some(tuning);
    h.department_id = Some(department_id);
    h.department_funding = Some(department_funding);
    h.budget_item = Some(budget_item);
    h.occupant_type = Some(occupant_type);
    h.occupant_property = Some(occupant_property);
    h.occupant_position = Some(position);
    h.lot_road_access = Some(road_access);
    h.sim_day = Some(sim_day);
    h.zone_value = Some(zone_value);
    h.sea_level = Some(sea_level);
    h.altitude = Some(altitude);
    h.dispatch_count = Some(dispatch_count);
    h.dispatch_ids = Some(dispatch_ids);
    h.dispatch_info = Some(dispatch_info);
    h
}

fn host(g: &Game) -> fc4_protection_host {
    let mut h = read_only_host(g);
    h.grids = grid_api(h.user);
    h.acquire_services = Some(acquire);
    h.release_services = Some(release_services);
    h.add_agent = Some(add_agent);
    h.remove_agent = Some(remove_agent);
    h.subscribe = Some(subscribe);
    h.unsubscribe = Some(unsubscribe);
    h.register_dispatch_source = Some(register_source);
    h.unregister_dispatch_source = Some(unregister_source);
    h.set_grid_instance = Some(set_grid_instance);
    h.add_ref = Some(add_ref);
    h.release = Some(release);
    h.create_department = Some(create_department);
    h.add_locally_funded = Some(add_funded);
    h.remove_locally_funded = Some(remove_funded);
    h.set_occupant_property = Some(set_occupant_property);
    h.record_history = Some(record_history);
    h.post = Some(post);
    h.create_air_source = Some(create_air);
    h.air_source_init = Some(air_init);
    h.air_source_shutdown = Some(air_shutdown);
    h
}

fn create(g: &Game, kind: u32) -> *mut fc4_protection {
    let h = host(g);
    let p = unsafe { fc4_protection_create(&h, kind, 0x51A, 7, 9) };
    assert!(!p.is_null());
    p
}

fn insert(p: *mut fc4_protection, occ: u64) {
    let m = fc4_protection_message {
        msg_type: msg::OCCUPANT_INSERTED,
        occupant: occ,
        ..Default::default()
    };
    assert!(unsafe { fc4_protection_do_message(p, &m) });
}

fn state(p: *const fc4_protection) -> fc4_protection_state {
    let mut s = fc4_protection_state::default();
    assert!(unsafe { fc4_protection_get_state(p, &mut s) });
    s
}

unsafe extern "C" fn collect(ctx: *mut c_void, t: u64) {
    (*(ctx as *mut Vec<u64>)).push(t);
}

fn list(f: impl FnOnce(crate::budget::fc4_token_callback, *mut c_void) -> u32) -> (u32, Vec<u64>) {
    let mut v: Vec<u64> = Vec::new();
    let n = f(Some(collect), &mut v as *mut Vec<u64> as *mut c_void);
    (n, v)
}

// ---- tests ------------------------------------------------------------------------------------------------

#[test]
fn create_rejects_bad_tables_and_kinds() {
    let g = Game::default();
    unsafe {
        assert!(fc4_protection_create(std::ptr::null(), 0, 1, 0, 0).is_null());
        let mut h = host(&g);
        assert!(fc4_protection_create(&h, 2, 1, 0, 0).is_null());
        h.grids.struct_size = 8;
        assert!(fc4_protection_create(&h, 0, 1, 0, 0).is_null());
        let mut h = host(&g);
        h.struct_size = 8;
        assert!(fc4_protection_create(&h, 0, 1, 0, 0).is_null());
        assert!(fc4_protection_shadow_create(&h, 0).is_null());
        let h = host(&g);
        assert!(fc4_protection_shadow_create(&h, 5).is_null());
    }
}

#[test]
fn query_interface_follows_section_1() {
    assert_eq!(fc4_protection_query_interface(0, 1), 1);
    assert_eq!(fc4_protection_query_interface(0, 0x88F5_4C7B), 1);
    assert_eq!(fc4_protection_query_interface(0, 0x6765_9895), 0);
    assert_eq!(fc4_protection_query_interface(1, 0x6765_9895), 1);
    assert_eq!(fc4_protection_query_interface(1, 0x4522_94AA), 2);
    assert_eq!(fc4_protection_query_interface(0, IID_DISPATCH_SOURCE), 3);
    assert_eq!(fc4_protection_query_interface(1, 0xE4FD_A3D4), 4);
    assert_eq!(fc4_protection_query_interface(0, 0xE98B_2F57), 0);
    assert_eq!(fc4_protection_query_interface(7, 1), 0);
}

#[test]
fn null_handle_gives_the_neutral_answers() {
    let null = std::ptr::null_mut::<fc4_protection>();
    unsafe {
        assert!(!fc4_protection_init(null));
        assert!(!fc4_protection_shutdown(null));
        assert_eq!(fc4_protection_grid(null), 0);
        assert_eq!(fc4_protection_station_count(null), 0);
        assert_eq!(fc4_protection_strike_length(null), u32::MAX);
        assert_eq!(fc4_protection_average_coverage(null), 0.0);
        assert!(!fc4_protection_is_station(null, STATION));
        let mut busy = true;
        assert!(!fc4_protection_can_dispatch(null, 0.0, 0.0, &mut busy));
        assert!(busy);
        let (n, v) = list(|cb, ctx| fc4_protection_stations(null, cb, ctx));
        assert_eq!((n, v.len()), (0, 0));
        assert!(!fc4_protection_get_state(null, &mut fc4_protection_state::default()));
        assert!(!fc4_protection_shadow_run(
            null,
            &geometry(),
            std::ptr::null(),
            0,
            0,
            false
        ));
        assert_eq!(fc4_protection_shadow_grid(null, std::ptr::null_mut(), 0), 0);
        assert_eq!(fc4_protection_shadow_unadjusted_grid(null, std::ptr::null_mut(), 0), 0);
        fc4_protection_destroy(null);
    }
}

#[test]
fn fire_init_and_shutdown_hold_and_release_everything() {
    let g = Game::default();
    let p = create(&g, 0);
    unsafe {
        assert!(fc4_protection_init(p));
        let fire = ProtectionKind::Fire.params();
        // Two game grids with the §2 instances, the department, the agent
        // (levels 4 and 6), nine subscriptions, the dispatch source and the
        // air source (created and initialised).
        let s = state(p);
        assert_eq!(s.initialized, 1);
        assert_eq!(
            (s.count_x, s.count_z, s.tracts),
            (TRACTS, TRACTS, (TRACTS * TRACTS) as u32)
        );
        let adj = s.adjusted_grid;
        let unadj = s.unadjusted_grid;
        assert_eq!(g.grids.borrow()[&adj].instance, fire.adjusted_instance);
        assert_eq!(g.grids.borrow()[&unadj].instance, fire.unadjusted_instance);
        assert_eq!(fc4_protection_grid(p), adj);
        assert_eq!(fc4_protection_department(p), DEPT);
        assert_eq!(g.departments.borrow().len(), 1);
        assert_eq!(g.departments.borrow()[0].0, fire.department);
        assert_eq!(*g.agents.borrow(), vec![(AGENT_NAME.to_string(), vec![4, 6], 2)]);
        assert_eq!(g.subs.borrow().len(), msg::SUBSCRIPTIONS.len());
        assert_eq!(g.registered.get(), 1);
        assert_eq!(*g.air.borrow(), vec!["create", "init"]);
        assert_eq!(s.air_source, AIR);
        assert_eq!(s.tracts_per_day, TRACTS * TRACTS / fire.period);
        assert_eq!(g.services.get(), 1);
        // A second Init changes nothing.
        assert!(fc4_protection_init(p));
        assert_eq!(g.departments.borrow().len(), 1);

        assert!(fc4_protection_shutdown(p));
        assert_eq!(g.services.get(), 0);
        assert_eq!(g.registered.get(), 0);
        assert!(g.agents.borrow().is_empty());
        assert!(g.subs.borrow().is_empty());
        assert_eq!(*g.air.borrow(), vec!["create", "init", "shutdown"]);
        assert_eq!(g.refs_of(AIR), 0);
        assert_eq!(g.refs_of(DEPT), 0);
        for t in [adj, unadj] {
            let grids = g.grids.borrow();
            assert!(grids[&t].shut);
            assert_eq!(grids[&t].refs, 0);
        }
        assert_eq!(fc4_protection_grid(p), 0);
        fc4_protection_destroy(p);
    }
}

#[test]
fn a_station_builds_coverage_and_answers_the_slots() {
    let g = Game::default();
    g.station(STATION, ProtectionKind::Fire, 100, 20, 96.0, [176.0, 0.0, 176.0], 3);
    g.types.borrow_mut().insert(HOUSE, BUILDING);
    let p = create(&g, 0);
    unsafe {
        assert!(fc4_protection_init(p));
        insert(p, STATION);
        insert(p, HOUSE); // no station effect: ignored
        assert_eq!(g.refs_of(STATION), 1);
        assert_eq!(g.refs_of(HOUSE), 0);
        assert_eq!(fc4_protection_station_count(p), 1);
        assert!(fc4_protection_is_station(p, STATION));
        assert!(!fc4_protection_is_station(p, HOUSE));
        assert!(!fc4_protection_is_station(p, 0));
        let (n, v) = list(|cb, ctx| fc4_protection_stations(p, cb, ctx));
        assert_eq!((n, v), (1, vec![STATION]));
        // §5.2: the availability mask (3 units) and base call number 1, both
        // u32 scalars; the station booked as locally funded.
        let fire = ProtectionKind::Fire.params();
        assert_eq!(g.prop_u32(STATION, fire.mask_prop), Some(0b111));
        assert_eq!(g.prop_u32(STATION, fire.base_prop), Some(1));
        assert!(g.writes.borrow().iter().all(|w| !w.2));
        assert_eq!(*g.funded.borrow(), vec![(DEPT, STATION, fire.purpose, true)]);
        // The cone is in the adjusted grid around tract (5, 5).
        let s = state(p);
        assert!(s.covered_tracts > 0);
        assert!(s.max_coverage > 0);
        let adj = g.grid_values(s.adjusted_grid);
        assert!(adj[(5 * TRACTS + 5) as usize] > 0.0);
        assert_eq!(adj[0], 0.0);
        // Slots.
        let (mut cost, mut pct) = (0i32, 0.0f32);
        assert!(fc4_protection_station_funding(p, STATION, &mut cost, &mut pct));
        assert_eq!((cost, pct), (500, 1.0));
        assert!(fc4_protection_station_funding(
            p,
            STATION,
            std::ptr::null_mut(),
            std::ptr::null_mut()
        ));
        assert!(!fc4_protection_station_funding(p, HOUSE, &mut cost, &mut pct));
        assert!(fc4_protection_station_efficiency(p, STATION) > 0.0);
        assert!(!fc4_protection_is_on_strike(p));
        assert!(!fc4_protection_is_station_on_strike(p, STATION));
        assert_eq!(fc4_protection_strike_length(p), u32::MAX);
        assert_eq!(fc4_protection_total_dispatch_count(p), 3);
        let (mut cover, mut station) = (-1.0f32, -1.0f32);
        assert!(fc4_protection_occupant_coverage(
            p,
            STATION,
            0.5,
            &mut cover,
            &mut station
        ));
        assert!(cover > 0.0 && station > 0.0);
        // Fire zeroes the outputs first, even when the effect fails.
        let (mut cover, mut station) = (-1.0f32, -1.0f32);
        assert!(!fc4_protection_occupant_coverage(
            p,
            HOUSE,
            0.5,
            &mut cover,
            &mut station
        ));
        assert_eq!((cover, station), (0.0, 0.0));
        // A strike begins once; the station is appended by the striking list
        // only when its property says so.
        assert!(fc4_protection_begin_strike(p, STATION));
        assert!(!fc4_protection_begin_strike(p, STATION));
        assert_eq!(fc4_protection_strike_start_date(p), 1000);
        assert_eq!(fc4_protection_strike_length(p), 0);
        assert_eq!(*g.posts.borrow().last().unwrap(), (fire.msg_strike_begin, STATION));
        let (n, v) = list(|cb, ctx| fc4_protection_striking_stations(p, cb, ctx) as u32);
        assert_eq!((n, v.len()), (0, 0));
        g.set_raw(STATION, prop::STRIKE_START, mem::UINT32, 990u32);
        let (n, v) = list(|cb, ctx| fc4_protection_striking_stations(p, cb, ctx) as u32);
        assert_eq!((n, v), (1, vec![STATION]));
        assert!(fc4_protection_end_strike(p));

        // Removal releases the station.
        let m = fc4_protection_message {
            msg_type: msg::OCCUPANT_REMOVED,
            occupant: STATION,
            ..Default::default()
        };
        assert!(fc4_protection_do_message(p, &m));
        assert_eq!(g.refs_of(STATION), 0);
        assert_eq!(fc4_protection_station_count(p), 0);
        assert!(fc4_protection_shutdown(p));
        fc4_protection_destroy(p);
    }
}

#[test]
fn the_dispatch_source_slots() {
    let g = Game::default();
    g.station(STATION, ProtectionKind::Police, 100, 20, 96.0, [176.0, 0.0, 176.0], 2);
    let p = create(&g, 1);
    unsafe {
        assert!(fc4_protection_init(p));
        insert(p, STATION);
        assert!(fc4_protection_can_handle_dispatch_type(p, 0));
        assert!(!fc4_protection_can_handle_dispatch_type(p, 1));
        assert_eq!(fc4_protection_max_possible_dispatch_count(p), 2);
        let (mut flags, mut base, mut count) = (0u32, 0u32, 0u32);
        assert!(fc4_protection_dispatches_at_station(
            p, STATION, &mut flags, &mut base, &mut count
        ));
        assert_eq!((flags, base, count), (0b11, 1, 2));
        let mut count = 0;
        assert!(fc4_protection_dispatches_at_station(
            p,
            STATION,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut count
        ));
        assert_eq!(count, 2);
        assert!(!fc4_protection_dispatches_at_station(
            p, HOUSE, &mut flags, &mut base, &mut count
        ));
        // CanDispatch: units 2, nothing busy, land above sea level.
        let mut busy = true;
        assert!(fc4_protection_can_dispatch(p, 10.0, 10.0, &mut busy));
        assert!(!busy);
        // Dispatch the nearest station's first free unit (call ID 1), then
        // one by ID, then revoke.
        let mut id = 77u32;
        assert_eq!(
            fc4_protection_dispatch_from_nearest_station(p, 0.0, 0.0, &mut id),
            STATION
        );
        assert_eq!(id, 1);
        let mut id = 2u32;
        assert!(fc4_protection_dispatch_from_station(p, STATION, &mut id));
        assert_eq!(id, 2);
        let mask = ProtectionKind::Police.params().mask_prop;
        assert_eq!(g.prop_u32(STATION, mask), Some(0));
        assert!(fc4_protection_revoke_dispatch_from_station(p, STATION, 1));
        assert!(!fc4_protection_revoke_dispatch_from_station(p, STATION, 1));
        assert_eq!(g.prop_u32(STATION, mask), Some(0b01));
        let (mut b, mut c, mut d) = (0u32, [9.0f32; 3], 0u32);
        assert!(fc4_protection_dispatch_automata_properties(
            p,
            &mut b,
            c.as_mut_ptr(),
            &mut d
        ));
        assert_eq!((b, c, d), (0x4228, [0.0; 3], 0x11882));
        assert!(fc4_protection_dispatch_automata_properties(
            p,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut()
        ));
        // An active dispatch adds the cone of its source station's dispatch
        // effect (§7.2, PR2); the ID buffer is sized from the count (never a
        // capacity of 0). The reserved field (an occupant without the
        // properties) is not read.
        let pd = ProtectionKind::Police.params();
        g.set_raw(STATION, pd.dispatch_effect_props[0], mem::SINT32, 50i32);
        g.set_raw(STATION, pd.dispatch_effect_props[1], mem::SINT32, 10i32);
        g.set_raw(STATION, pd.dispatch_effect_props[2], mem::FLOAT32, 64.0f32);
        g.dispatches.borrow_mut().push((
            0,
            fc4_protection_dispatch_info {
                station: STATION,
                reserved: AUTOMATON,
                call_id: 1,
                active: 1,
                x: 400.0,
                z: 400.0,
            },
        ));
        let m = fc4_protection_message {
            msg_type: msg::DISPATCH_ACTIVE,
            data1: 0,
            data2: 1,
            ..Default::default()
        };
        assert!(fc4_protection_do_message(p, &m));
        assert!(g.capacities.borrow().iter().all(|&c| c > 0));
        let adj = g.grid_values(state(p).adjusted_grid);
        assert!(adj[(12 * TRACTS + 12) as usize] > 0.0);
        assert_eq!(fc4_protection_station_dispatch_radius(p, STATION), 64.0);
        assert_eq!(fc4_protection_station_dispatch_radius(p, AUTOMATON), 0.0);
        assert!(fc4_protection_shutdown(p));
        fc4_protection_destroy(p);
    }
}

#[test]
fn police_jails_and_their_lists() {
    let g = Game::default();
    g.types.borrow_mut().insert(JAIL, BUILDING);
    g.set_raw(JAIL, prop::JAIL_CAPACITY, mem::UINT32, 500u32);
    let p = create(&g, 1);
    unsafe {
        assert!(fc4_protection_init(p));
        // Police: the station department and the jail department, and the
        // crime subscription on top of the nine.
        assert_eq!(g.departments.borrow().len(), 2);
        assert_eq!(fc4_protection_jail_department(p), JAIL_DEPT);
        assert!(g.subs.borrow().contains(&msg::CRIME));
        insert(p, JAIL);
        assert!(fc4_protection_is_jail(p, JAIL));
        assert_eq!(fc4_protection_jail_count(p), 1);
        let (n, v) = list(|cb, ctx| fc4_protection_jails(p, cb, ctx));
        assert_eq!((n, v), (1, vec![JAIL]));
        assert_eq!(
            fc4_protection_jail_capacity_of(p, JAIL),
            fc4_protection_jail_capacity(p)
        );
        // Jail list + locally funded jail list.
        assert_eq!(g.refs_of(JAIL), 2);
        // No jailbreak: 0 and no callback (§16 quirk 7).
        let (n, v) = list(|cb, ctx| fc4_protection_jailbreak_buildings(p, cb, ctx) as u32);
        assert_eq!((n, v.len()), (0, 0));
        assert!(!fc4_protection_is_in_jailbreak(p, JAIL));
        assert!(fc4_protection_begin_jailbreak(p, JAIL));
        assert!(fc4_protection_is_in_jailbreak(p, JAIL));
        let (n, v) = list(|cb, ctx| fc4_protection_jailbreak_buildings(p, cb, ctx) as u32);
        assert_eq!((n, v), (1, vec![JAIL]));
        assert!(!fc4_protection_end_jailbreak(p, JAIL)); // AL = 0 always
        assert!(fc4_protection_process_monthly_crime_stats(p));
        let _ = fc4_protection_reset_unjailed_criminals(p);
        assert!(fc4_protection_shutdown(p));
        assert_eq!(g.refs_of(JAIL), 0);
        assert_eq!(g.refs_of(JAIL_DEPT), 0);
        assert_eq!(g.refs_of(DEPT), 0);
        fc4_protection_destroy(p);
    }
}

#[test]
fn save_round_trip_and_read_before_init() {
    let g = Game::default();
    g.station(STATION, ProtectionKind::Police, 100, 20, 96.0, [176.0, 0.0, 176.0], 2);
    g.types.borrow_mut().insert(JAIL, BUILDING);
    g.set_raw(JAIL, prop::JAIL_CAPACITY, mem::UINT32, 500u32);
    let p = create(&g, 1);
    let mut out = Out::default();
    unsafe {
        assert!(fc4_protection_init(p));
        insert(p, STATION);
        insert(p, JAIL);
        let before = state(p);
        let os = ostream(&mut out);
        assert!(fc4_protection_write(p, &os));
        assert!(out.refs.iter().any(|&(t, _)| t == STATION));
        assert!(out.refs.iter().any(|&(t, _)| t == before.adjusted_grid));
        assert!(fc4_protection_shutdown(p));
        fc4_protection_destroy(p);
    }
    // The stream hands each host reference over with one reference (the
    // grids are still alive in the fake game: give them that reference).
    let adj_unadj: Vec<u64> = g.grids.borrow().keys().copied().collect();
    for t in &adj_unadj {
        g.grids.borrow_mut().get_mut(t).unwrap().refs += 1;
    }
    for t in [JAIL, DEPT, JAIL_DEPT, AIR] {
        let n = out.refs.iter().filter(|&&(r, _)| r == t).count() as i32;
        *g.refs.borrow_mut().entry(t).or_insert(0) += n;
    }
    // The station's ref says "one of FreeCity4's objects" (core_handle set):
    // it arrives without a reference and the core takes one through add_ref.
    let station_refs = g.refs_of(STATION);
    assert_eq!(station_refs, 0);
    let mark = |r: &mut crate::stream::fc4_ref| {
        if r.token == STATION {
            r.core_handle = 1 as *mut c_void;
        }
    };
    let mut inp = In::new(&out.bytes);
    inp.resolve = Some(&mark);
    let q = create(&g, 1);
    let grids_before = g.grids.borrow().len();
    unsafe {
        let is = istream(&mut inp);
        assert!(fc4_protection_read(q, &is));
        // Read creates nothing and sizes nothing from the city.
        assert_eq!(g.grids.borrow().len(), grids_before);
        let s = state(q);
        assert_eq!(s.loaded, 1);
        assert_eq!(s.initialized, 0);
        assert_eq!(s.stations, 1);
        assert_eq!(s.jails, 1);
        assert_eq!(s.department, DEPT);
        assert_eq!(s.jail_department, JAIL_DEPT);
        assert_eq!(s.air_source, AIR);
        assert_eq!(g.refs_of(STATION), station_refs + 1);
        // Destroyed without Init: everything it holds is released.
        fc4_protection_destroy(q);
    }
    assert_eq!(g.refs_of(STATION), station_refs);
    for t in [JAIL, DEPT, JAIL_DEPT, AIR] {
        assert_eq!(g.refs_of(t), 0, "token {t:#x}");
    }
    for t in &adj_unadj {
        assert_eq!(g.grids.borrow()[t].refs, 0);
    }
}

/// A loaded city whose grid records come after the simulator's own record
/// (the in-game fire case: `Read (fire): tracts=0x0` with a non-null grid,
/// then `Init (fire): tracts=0x0 tractsPerDay=0` and no daily cycle for the
/// whole run). At `Read` both grids are still the empty objects the game made
/// for the refs; the game reads their records before `Init` (HG1, foundations
/// §3.7: every saved object is read in ascending class ID before
/// `cSC4City::Init`, and fire `0x2990C05A` comes before the grids
/// `0x49B9E602`). `Init` takes the geometry as it is then (§4.1 base init
/// steps 2 and 6, §11 "recomputed on load"), for fire and police, and does
/// not rebuild the adjusted grid (§11).
#[test]
fn init_takes_the_geometry_of_grids_read_after_the_simulator() {
    for kind in [0u32, 1] {
        let pk = if kind == 0 {
            ProtectionKind::Fire
        } else {
            ProtectionKind::Police
        };
        let g = Game::default();
        g.station(STATION, pk, 100, 20, 96.0, [176.0, 0.0, 176.0], 2);
        let p = create(&g, kind);
        let mut out = Out::default();
        let (adj, unadj) = unsafe {
            assert!(fc4_protection_init(p));
            insert(p, STATION);
            let s = state(p);
            let os = ostream(&mut out);
            assert!(fc4_protection_write(p, &os));
            assert!(fc4_protection_shutdown(p));
            fc4_protection_destroy(p);
            (s.adjusted_grid, s.unadjusted_grid)
        };
        let saved = g.grid_values(adj);
        assert!(saved.iter().any(|&v| v > 0.0));
        // The stream hands each reference over with one reference. Both
        // grids are the game's empty objects until their records are read.
        for &(t, _) in &out.refs {
            if t == adj || t == unadj {
                g.grids.borrow_mut().get_mut(&t).unwrap().refs += 1;
            } else if t != 0 {
                *g.refs.borrow_mut().entry(t).or_insert(0) += 1;
            }
        }
        g.unread.borrow_mut().extend([adj, unadj]);

        let q = create(&g, kind);
        let mut inp = In::new(&out.bytes);
        unsafe {
            let is = istream(&mut inp);
            assert!(fc4_protection_read(q, &is));
            // What the plugin's Read status line sees (and the core must not
            // keep): the grid is there, its geometry not yet.
            let s = state(q);
            assert_eq!(s.loaded, 1, "kind {kind}");
            assert_eq!(s.adjusted_grid, adj, "kind {kind}");
            assert_eq!((s.count_x, s.count_z, s.tracts), (0, 0, 0), "kind {kind}");
            assert_eq!(s.tracts_per_day, 0, "kind {kind}");

            // The game reads the grids' records, then calls Init.
            g.unread.borrow_mut().clear();
            assert!(fc4_protection_init(q));
            let s = state(q);
            assert_eq!(s.initialized, 1, "kind {kind}");
            assert_eq!(
                (s.count_x, s.count_z, s.tracts),
                (TRACTS, TRACTS, (TRACTS * TRACTS) as u32),
                "kind {kind}"
            );
            // 16 × 16 tracts: fire 256 / 50 = 5, police 256 / 40 = 6.
            assert_eq!(s.tracts_per_day, TRACTS * TRACTS / pk.params().period, "kind {kind}");
            assert!(s.tracts_per_day > 0, "kind {kind}");
            // §11: the adjusted grid is whatever was saved.
            assert_eq!(g.grid_values(adj), saved, "kind {kind}");
            assert!(s.covered_tracts > 0, "kind {kind}");

            // The daily cycle runs. The saved iterator was never reset, so
            // the first day ends a cycle at once (§4.1 base init step 2),
            // then visits tracts-per-day tracts (every cell is zoned here).
            let posts = g.posts.borrow().len();
            let day = fc4_protection_message {
                msg_type: msg::SIM_NEW_DAY,
                ..Default::default()
            };
            assert!(fc4_protection_do_message(q, &day));
            let s = state(q);
            assert_eq!(s.zoned, s.tracts_per_day, "kind {kind}");
            assert_eq!(
                g.posts.borrow()[posts..].to_vec(),
                vec![(pk.params().msg_cycle, 0)],
                "kind {kind}"
            );
            assert!(fc4_protection_shutdown(q));
            fc4_protection_destroy(q);
        }
    }
}

#[test]
fn a_message_during_a_call_is_queued() {
    let g = Game::default();
    g.station(STATION, ProtectionKind::Fire, 100, 20, 96.0, [176.0, 0.0, 176.0], 1);
    g.station(STATION2, ProtectionKind::Fire, 100, 20, 96.0, [300.0, 0.0, 300.0], 1);
    let p = create(&g, 0);
    unsafe {
        assert!(fc4_protection_init(p));
        insert(p, STATION);
        g.target.set(p as usize);
        // BeginStrike posts; the post delivers an insert of STATION2, which
        // runs after BeginStrike returns.
        assert!(fc4_protection_begin_strike(p, STATION));
        assert_eq!(g.reenter.get(), 1);
        assert_eq!(fc4_protection_station_count(p), 2);
        assert_eq!(state(p).reentrant_calls, 0);
        g.target.set(0);
        assert!(fc4_protection_shutdown(p));
        assert_eq!(g.refs_of(STATION2), 0);
        fc4_protection_destroy(p);
    }
}

#[test]
fn the_shadow_rebuilds_the_replacements_grid_read_only() {
    let g = Game::default();
    g.station(STATION, ProtectionKind::Fire, 100, 20, 96.0, [176.0, 0.0, 176.0], 3);
    g.station(STATION2, ProtectionKind::Fire, 80, 10, 128.0, [300.0, 0.0, 120.0], 2);
    let p = create(&g, 0);
    unsafe {
        assert!(fc4_protection_init(p));
        insert(p, STATION);
        insert(p, STATION2);
        let ours = g.grid_values(state(p).adjusted_grid);
        let refs = (g.refs_of(STATION), g.refs_of(STATION2), g.refs_of(DEPT));
        let writes = g.writes.borrow().len();
        // Not a shadow handle.
        assert!(!fc4_protection_shadow_run(
            p,
            &geometry(),
            [STATION].as_ptr(),
            1,
            DEPT,
            false
        ));

        let ro = read_only_host(&g);
        let s = fc4_protection_shadow_create(&ro, 0);
        assert!(!s.is_null());
        assert_eq!(fc4_protection_shadow_grid(s, std::ptr::null_mut(), 0), 0);
        assert_eq!(fc4_protection_shadow_unadjusted_grid(s, std::ptr::null_mut(), 0), 0);
        let mut bad = geometry();
        bad.count_x = 0;
        assert!(!fc4_protection_shadow_run(s, &bad, std::ptr::null(), 0, 0, false));
        let stations = [STATION, STATION2];
        assert!(fc4_protection_shadow_run(
            s,
            &geometry(),
            stations.as_ptr(),
            2,
            DEPT,
            false
        ));
        let mut grid = vec![0u8; (TRACTS * TRACTS) as usize];
        assert_eq!(fc4_protection_shadow_grid(s, grid.as_mut_ptr(), grid.len()), grid.len());
        let theirs: Vec<f64> = grid.iter().map(|&v| f64::from(v)).collect();
        assert_eq!(theirs, ours);
        assert!(grid.iter().any(|&v| v > 0));
        // PR5: the unadjusted grid too (the replacement's int16 grid).
        let ours_unadj = g.grid_values(state(p).unadjusted_grid);
        let mut unadj = vec![0i16; (TRACTS * TRACTS) as usize];
        assert_eq!(
            fc4_protection_shadow_unadjusted_grid(s, unadj.as_mut_ptr(), unadj.len()),
            unadj.len()
        );
        let theirs: Vec<f64> = unadj.iter().map(|&v| f64::from(v)).collect();
        assert_eq!(theirs, ours_unadj);
        assert!(unadj.iter().any(|&v| v > 0));
        let mut short16 = [0i16; 3];
        assert_eq!(
            fc4_protection_shadow_unadjusted_grid(s, short16.as_mut_ptr(), 3),
            unadj.len()
        );
        assert_eq!(short16[..], unadj[..3]);
        // Queries on the shadow use the same inputs.
        assert_eq!(
            fc4_protection_station_efficiency(s, STATION),
            fc4_protection_station_efficiency(p, STATION)
        );
        assert_eq!(fc4_protection_chance_of_strike(s), fc4_protection_chance_of_strike(p));
        assert_eq!(fc4_protection_total_dispatch_count(s), 5);
        let (mut a, mut b) = (0.0f32, 0.0f32);
        let (mut c, mut d) = (0.0f32, 0.0f32);
        assert!(fc4_protection_occupant_coverage(s, STATION2, 0.5, &mut a, &mut b));
        assert!(fc4_protection_occupant_coverage(p, STATION2, 0.5, &mut c, &mut d));
        assert_eq!((a, b), (c, d));
        // A short buffer gets the first values only.
        let mut short = [0u8; 4];
        assert_eq!(fc4_protection_shadow_grid(s, short.as_mut_ptr(), 4), grid.len());
        assert_eq!(short[..], grid[..4]);
        // Read-only: no reference taken or released, nothing written.
        fc4_protection_destroy(s);
        assert_eq!((g.refs_of(STATION), g.refs_of(STATION2), g.refs_of(DEPT)), refs);
        assert_eq!(g.writes.borrow().len(), writes);
        assert!(fc4_protection_shutdown(p));
        fc4_protection_destroy(p);
    }
}
