// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The pollution C ABI (fc4.h "pollution" and "pollution shadow") against a
//! fake game: the game's grids through `fc4_grid_api`, occupants and their
//! properties, the reference protocol, persistence, getters from a second
//! thread during a main-thread call, and the shadow over a stand-in original.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::{c_char, c_void};
use std::sync::mpsc;
use std::time::Duration;

use fc4_core::pollution::{msg, prop, shipped_tuning, CLSID_POLLUTION_SIMULATOR, GRID_INSTANCES};
use fc4_core::variant::mem;

use crate::hostgrid::fc4_grid_api;
use crate::landvalue::fc4_grid_geometry;
use crate::pollution::*;
use crate::pollution_shadow::*;
use crate::props::{fc4_props, fc4_props_set_typed};
use crate::stream::{fc4_ostream, fc4_ref};
use crate::test_streams::{istream, ostream, In, Out};

const CELLS: i32 = 32;
const INT16_GRID: u32 = 0x49B9_E605;
const POLLUTER: u64 = 5;
const INCINERATOR: u64 = 7;
const DEPT: u64 = 0xD0;
/// 2000-01-01.
const JULIAN: i32 = 2_451_545;

struct Grid {
    data: Vec<i16>,
    tract_size: i32,
    refs: i32,
    shut: bool,
    instance: u32,
}

impl Grid {
    fn count(&self) -> i32 {
        CELLS / self.tract_size
    }
}

/// (element type, array flag, bytes, count).
type Written = (u16, bool, Vec<u8>, usize);

#[derive(Default)]
struct Game {
    grids: RefCell<BTreeMap<u64, Grid>>,
    next_grid: Cell<u64>,
    refs: RefCell<BTreeMap<u64, i32>>,
    agents: RefCell<Vec<(u32, String, u32)>>,
    subs: RefCell<Vec<u32>>,
    posts: RefCell<Vec<(u32, u32)>>,
    history: RefCell<Vec<(u32, i32)>>,
    requesters: Cell<i32>,
    dept: Cell<u64>,
    /// Occupant properties (set up by the test or written by the simulator).
    props: RefCell<BTreeMap<(u64, u32), Written>>,
    /// Second-thread test: the simulator to read from another thread at
    /// every host write.
    probe: Cell<usize>,
    probes: Cell<u32>,
    probe_failures: Cell<u32>,
    /// Every grid is still an empty object the game made for a saved
    /// reference (its own record not read yet): geometry 0 × 0.
    unread: Cell<bool>,
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

impl Game {
    fn new() -> Box<Game> {
        let g = Box::<Game>::default();
        g.set(POLLUTER, prop::CENTRE, mem::SINT32, true, &[100i32, 50, 40, 0]);
        g.set(INCINERATOR, prop::INCINERATOR, mem::UINT32, false, &[1000u32]);
        g
    }

    fn set<T: Copy>(&self, occ: u64, id: u32, t: u16, array: bool, v: &[T]) {
        let bytes = unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, std::mem::size_of_val(v)) }.to_vec();
        self.props.borrow_mut().insert((occ, id), (t, array, bytes, v.len()));
    }

    fn refs_of(&self, t: u64) -> i32 {
        self.refs.borrow().get(&t).copied().unwrap_or(0)
    }

    fn user(&self) -> *mut c_void {
        self as *const Game as *mut c_void
    }

    /// Everything a delivery could change.
    fn fingerprint(&self) -> String {
        let grids: Vec<(u64, Vec<i16>, i32, bool)> = self
            .grids
            .borrow()
            .iter()
            .map(|(t, g)| (*t, g.data.clone(), g.refs, g.shut))
            .collect();
        format!(
            "{grids:?} {:?} {:?} {:?} {:?} {:?} {} {}",
            self.refs.borrow(),
            self.agents.borrow(),
            self.subs.borrow(),
            self.posts.borrow(),
            self.history.borrow(),
            self.requesters.get(),
            self.props.borrow().len()
        ) + &format!("{:?}", self.props.borrow())
    }

    /// Reads the simulator from another thread while this (main) thread is
    /// inside a host call; a getter blocked on the state would time out.
    fn probe(&self) {
        let p = self.probe.get();
        if p == 0 {
            return;
        }
        self.probes.set(self.probes.get() + 1);
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let p = p as *mut fc4_pollution;
            let r = unsafe {
                (
                    fc4_pollution_slot_value(p, 10),
                    fc4_pollution_slot_value(p, 42),
                    fc4_pollution_slot_cell(p, 7, 10, 10),
                    fc4_pollution_value(p, 0, 10, 10),
                    fc4_pollution_grid(p, 0),
                    fc4_pollution_grid_by_index(p, 0),
                    fc4_pollution_garbage_buildings(p, None, std::ptr::null_mut()),
                    fc4_pollution_slot_cell(p, 13, 10, 10),
                )
            };
            let _ = tx.send(r);
        });
        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok(r) => {
                if r.4 == 0 || r.4 == r.5 || r.5 == 0 || r.6 != 1 {
                    self.probe_failures.set(self.probe_failures.get() + 1);
                }
            }
            Err(_) => panic!("a getter on another thread blocked during a main-thread host call"),
        }
    }
}

// ---- grids --------------------------------------------------------------------------------------------

unsafe extern "C" fn g_create(u: *mut c_void, clsid: u32, instance: u32, ts: i32) -> u64 {
    assert_eq!(clsid, INT16_GRID);
    assert!(ts == 1 || ts == 2);
    let g = game(u);
    let t = g.next_grid.get() + 0x7000;
    g.next_grid.set(g.next_grid.get() + 0x10);
    let n = (CELLS / ts) as usize;
    g.grids.borrow_mut().insert(
        t,
        Grid {
            data: vec![0; n * n],
            tract_size: ts,
            refs: 1,
            shut: false,
            instance,
        },
    );
    t
}
unsafe extern "C" fn g_class(u: *mut c_void, t: u64) -> u32 {
    if game(u).grids.borrow().contains_key(&t) {
        INT16_GRID
    } else {
        0
    }
}
unsafe extern "C" fn g_geometry(u: *mut c_void, t: u64, out: *mut fc4_grid_geometry) {
    if game(u).unread.get() {
        *out = fc4_grid_geometry::default();
        return;
    }
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
    assert_eq!(kind, INT16_GRID);
    let grids = game(u).grids.borrow();
    let g = &grids[&t];
    let n = g.count();
    if x < 0 || z < 0 || x >= n || z >= n {
        return 0.0;
    }
    f64::from(g.data[(x * n + z) as usize])
}
unsafe extern "C" fn g_set(u: *mut c_void, t: u64, _kind: u32, x: i32, z: i32, v: f64) {
    let mut grids = game(u).grids.borrow_mut();
    let g = grids.get_mut(&t).unwrap();
    let n = g.count();
    if x >= 0 && z >= 0 && x < n && z < n {
        g.data[(x * n + z) as usize] = v as i16;
    }
}
unsafe extern "C" fn g_fill(u: *mut c_void, t: u64, _kind: u32, v: f64) {
    game(u).grids.borrow_mut().get_mut(&t).unwrap().data.fill(v as i16);
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
unsafe extern "C" fn position_to_cell(_: *mut c_void, x: f32, z: f32, cx: *mut i32, cz: *mut i32) {
    *cx = (x / 16.0).floor() as i32;
    *cz = (z / 16.0).floor() as i32;
}
unsafe extern "C" fn tuning(_: *mut c_void) -> *mut fc4_props {
    Box::into_raw(Box::new(shipped_tuning())) as *mut fc4_props
}
unsafe extern "C" fn altitude(_: *mut c_void, _x: f32, _z: f32) -> f32 {
    0.0
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
unsafe extern "C" fn post(u: *mut c_void, msg_type: u32, data1: u32) {
    game(u).posts.borrow_mut().push((msg_type, data1));
    game(u).probe();
}
unsafe extern "C" fn record_history(u: *mut c_void, series: u32, value: i32) {
    game(u).history.borrow_mut().push((series, value));
    game(u).probe();
}
unsafe extern "C" fn add_requester(u: *mut c_void) {
    game(u).requesters.set(game(u).requesters.get() + 1);
}
unsafe extern "C" fn remove_requester(u: *mut c_void) {
    game(u).requesters.set(game(u).requesters.get() - 1);
}
unsafe extern "C" fn zone_type(_: *mut c_void, x: i32, z: i32) -> u32 {
    // A residential block around the polluter.
    u32::from((8..16).contains(&x) && (8..16).contains(&z))
}
unsafe extern "C" fn powered(_: *mut c_void, _occ: u64) -> bool {
    true
}
unsafe extern "C" fn population(_: *mut c_void) -> i32 {
    5000
}
unsafe extern "C" fn factor(_: *mut c_void, _id: u32, _index: u32) -> f32 {
    1.0
}

// ---- occupants -----------------------------------------------------------------------------------------

unsafe extern "C" fn occupant_property(u: *mut c_void, occ: u64, id: u32, p: *mut fc4_props) -> bool {
    match game(u).props.borrow().get(&(occ, id)) {
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
        mem::BOOL | mem::UINT8 | mem::SINT8 | mem::CHAR | mem::RZ_CHAR => 1,
        mem::UINT16 | mem::SINT16 | mem::UNICODE_CHAR => 2,
        mem::UINT64 | mem::SINT64 | mem::FLOAT64 => 8,
        _ => 4,
    };
    let bytes = std::slice::from_raw_parts(data as *const u8, size * count).to_vec();
    game(u)
        .props
        .borrow_mut()
        .insert((occ, id), (mem_type, is_array, bytes, count));
    game(u).probe();
}
unsafe extern "C" fn remove_occupant_property(u: *mut c_void, occ: u64, id: u32) {
    game(u).props.borrow_mut().remove(&(occ, id));
}
fn centre(occ: u64) -> [f32; 3] {
    match occ {
        POLLUTER => [10.5 * 16.0, 0.0, 10.5 * 16.0],
        _ => [20.5 * 16.0, 0.0, 20.5 * 16.0],
    }
}
unsafe extern "C" fn position(_: *mut c_void, occ: u64, out: *mut f32) {
    std::slice::from_raw_parts_mut(out, 3).copy_from_slice(&centre(occ));
}
unsafe extern "C" fn bounding_box(_: *mut c_void, occ: u64, a: *mut f32, b: *mut f32) {
    let c = centre(occ);
    std::slice::from_raw_parts_mut(a, 3).copy_from_slice(&[c[0] - 8.0, 0.0, c[2] - 8.0]);
    std::slice::from_raw_parts_mut(b, 3).copy_from_slice(&[c[0] + 7.0, 10.0, c[2] + 7.0]);
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

// ---- budget ---------------------------------------------------------------------------------------------

unsafe extern "C" fn create_department(u: *mut c_void, _id: u32, _group: u32) -> u64 {
    // The budget simulator keeps the new department; the caller adds its
    // own reference (§4.1).
    game(u).dept.set(DEPT);
    DEPT
}
unsafe extern "C" fn department_funding(_: *mut c_void, _d: u64) -> f32 {
    1.0
}
unsafe extern "C" fn full_expenses(u: *mut c_void, _d: u64, _line: u32, _v: i64) {
    game(u).probe();
}

fn host(g: &Game) -> fc4_pollution_host {
    let mut h: fc4_pollution_host = unsafe { std::mem::zeroed() };
    h.struct_size = std::mem::size_of::<fc4_pollution_host>() as u32;
    h.user = g.user();
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
    h.position_to_cell = Some(position_to_cell);
    h.tuning = Some(tuning);
    h.altitude = Some(altitude);
    h.add_agent = Some(add_agent);
    h.remove_agent = Some(remove_agent);
    h.subscribe = Some(subscribe);
    h.unsubscribe = Some(unsubscribe);
    h.post = Some(post);
    h.record_history = Some(record_history);
    h.add_prop_requester = Some(add_requester);
    h.remove_prop_requester = Some(remove_requester);
    h.zone_type = Some(zone_type);
    h.cumulative_factor = Some(factor);
    h.is_occupant_powered = Some(powered);
    h.population = Some(population);
    h.occupant_property = Some(occupant_property);
    h.set_occupant_property = Some(set_occupant_property);
    h.remove_occupant_property = Some(remove_occupant_property);
    h.occupant_position = Some(position);
    h.occupant_bounding_box = Some(bounding_box);
    h.add_ref = Some(add_ref);
    h.release = Some(release);
    h.resolve_ref = Some(resolve_ref);
    h.create_department = Some(create_department);
    h.department_funding = Some(department_funding);
    h.set_line_item_full_expenses = Some(full_expenses);
    h
}

fn message(p: *mut fc4_pollution, msg_type: u32, occupant: u64) {
    assert!(unsafe { fc4_pollution_do_message(p, msg_type, occupant, 0) });
}

fn state_of(p: *const fc4_pollution) -> fc4_pollution_state {
    let mut s = fc4_pollution_state::default();
    assert!(unsafe { fc4_pollution_get_state(p, &mut s) });
    s
}

fn started(g: &Game) -> *mut fc4_pollution {
    let h = host(g);
    let p = unsafe { fc4_pollution_create(&h, 0x51, 7) };
    assert!(!p.is_null());
    assert!(unsafe { fc4_pollution_init(p) });
    p
}

fn stop(p: *mut fc4_pollution) {
    unsafe {
        assert!(fc4_pollution_shutdown(p));
        fc4_pollution_destroy(p);
    }
}

#[test]
fn create_rejects_short_tables_and_null_handles_answer_fallbacks() {
    let g = Game::new();
    let mut h = host(&g);
    h.grids.struct_size = 8;
    assert!(unsafe { fc4_pollution_create(&h, 0, 0) }.is_null());
    let mut h = host(&g);
    h.struct_size = 8;
    assert!(unsafe { fc4_pollution_create(&h, 0, 0) }.is_null());
    let null = std::ptr::null_mut::<fc4_pollution>();
    unsafe {
        assert!(fc4_pollution_create(std::ptr::null(), 0, 0).is_null());
        assert!(!fc4_pollution_init(null));
        assert!(!fc4_pollution_shutdown(null));
        assert!(fc4_pollution_do_message(null, msg::SIM_NEW_DAY, 0, 0));
        assert_eq!(fc4_pollution_slot_value(null, 10), 0);
        assert_eq!(fc4_pollution_slot_cell(null, 7, 0, 0), 0);
        assert_eq!(fc4_pollution_grid(null, 0), 0);
        assert_eq!(fc4_pollution_value(null, 0, 0, 0), 0);
        assert!(!fc4_pollution_worst_tract(
            null,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut()
        ));
        assert!(!fc4_pollution_set_cell(null, 26, 0, 0, 1));
        assert_eq!(fc4_pollution_add_to_garbage_supply(null, 5), 0);
        assert_eq!(fc4_pollution_set_garbage_capacity(null, 5), 5);
        assert_eq!(fc4_pollution_garbage_put_in_landfill(null, 5), 0);
        assert_eq!(fc4_pollution_recycling_center_effect(null), 0.0);
        assert_eq!(fc4_pollution_garbage_scaling_factor(null), 0.0);
        assert_eq!(fc4_pollution_garbage_buildings(null, None, std::ptr::null_mut()), 0);
        assert_eq!(fc4_pollution_building_garbage_capacity(null, 1), 0);
        let mut v = 9u32;
        assert!(!fc4_pollution_occupant_u32(null, 52, 1, 0, &mut v));
        let mut f = 9.0f32;
        assert!(!fc4_pollution_occupant_percent(null, 54, 1, &mut f));
        let mut c = 9i64;
        assert!(!fc4_pollution_maintenance_cost(null, 1, &mut c));
        fc4_pollution_add_radiation(null, 0.0, 0.0, 0.0, 1.0, 1.0);
        assert_eq!(fc4_pollution_requested_share(null, 0, 0, 1, 1), 0);
        assert_eq!(fc4_pollution_prop_requester_slot(null, 4), 0);
        let mut s = fc4_pollution_state::default();
        assert!(!fc4_pollution_get_state(null, &mut s));
        assert_eq!(fc4_pollution_grid_by_index(null, 0), 0);
        assert!(!fc4_pollution_write(null, std::ptr::null()));
        assert!(!fc4_pollution_read(null, std::ptr::null()));
        fc4_pollution_destroy(null);
        // The shadow likewise.
        assert!(fc4_pollution_shadow_create(std::ptr::null(), 0).is_null());
        let s = std::ptr::null_mut::<fc4_pollution_shadow>();
        assert_eq!(
            fc4_pollution_shadow_begin(s, msg::SIM_NEW_DAY, JULIAN, std::ptr::null()),
            -1
        );
        assert_eq!(
            fc4_pollution_shadow_finish(s, std::ptr::null(), None, std::ptr::null_mut()),
            -1
        );
        let mut st = fc4_pollution_shadow_stats::default();
        assert!(!fc4_pollution_shadow_get_stats(s, &mut st));
        fc4_pollution_shadow_destroy(s);
    }
}

#[test]
fn init_registers_creates_the_grids_and_the_department() {
    let g = Game::new();
    let p = started(&g);
    assert!(g.agents.borrow().iter().all(|(_, n, _)| n == "cSC4PollutionSimulator"));
    assert!(!g.agents.borrow().is_empty());
    assert_eq!(*g.subs.borrow(), msg::SUBSCRIPTIONS.to_vec());
    assert_eq!(g.requesters.get(), 1);
    assert_eq!(g.grids.borrow().len(), 8);
    for (i, &instance) in GRID_INSTANCES.iter().enumerate() {
        let t = unsafe { fc4_pollution_grid_by_index(p, i as u32) };
        assert_eq!(g.grids.borrow()[&t].instance, instance);
    }
    assert_eq!(unsafe { fc4_pollution_grid_by_index(p, 8) }, 0);
    let st = state_of(p);
    assert_eq!(st.struct_size as usize, std::mem::size_of::<fc4_pollution_state>());
    assert_eq!((st.initialized, st.loaded), (1, 0));
    assert_eq!(st.department, DEPT);
    assert_eq!(st.tracts_per_day, 16 * 16 / 15);
    assert_eq!(st.grids[0].tracts, 256);
    assert_eq!(g.refs_of(DEPT), 1);
    unsafe {
        assert_eq!(fc4_pollution_prop_requester_slot(p, 5), 0);
        assert_ne!(fc4_pollution_prop_requester_slot(p, 4), 0);
    }

    assert!(unsafe { fc4_pollution_shutdown(p) });
    assert!(g.agents.borrow().is_empty() && g.subs.borrow().is_empty());
    assert_eq!(g.requesters.get(), 0);
    assert_eq!(g.refs_of(DEPT), 0);
    for gr in g.grids.borrow().values() {
        assert!(
            gr.shut && gr.refs == 0,
            "Shutdown shuts the grids down and releases them"
        );
    }
    unsafe { fc4_pollution_destroy(p) };
}

#[test]
fn buildings_pollute_the_game_grids() {
    let g = Game::new();
    let p = started(&g);
    message(p, msg::OCCUPANT_INSERTED, POLLUTER);
    message(p, msg::OCCUPANT_INSERTED, INCINERATOR);
    assert_eq!(g.refs_of(INCINERATOR), 1, "the incinerator list holds one reference");
    // Slot 5 hands out the value grids.
    assert_eq!(unsafe { fc4_pollution_grid(p, 0) }, unsafe {
        fc4_pollution_grid_by_index(p, 4)
    });
    let air = unsafe { fc4_pollution_grid_by_index(p, 0) };
    let tract = (10 / 2) * (CELLS / 2) + 10 / 2;
    assert_eq!(
        g.grids.borrow()[&air].data[tract as usize],
        100,
        "the centre of the air cone"
    );
    unsafe {
        assert_eq!(fc4_pollution_slot_value(p, 42), 1);
        // Slot 46: borrowed tokens; the caller adds its own reference.
        let mut got: Vec<u64> = Vec::new();
        unsafe extern "C" fn push(ctx: *mut c_void, t: u64) {
            (*(ctx as *mut Vec<u64>)).push(t);
        }
        let n = fc4_pollution_garbage_buildings(p, Some(push), &mut got as *mut Vec<u64> as *mut c_void);
        assert_eq!((n, got), (1, vec![INCINERATOR]));
        assert_eq!(g.refs_of(INCINERATOR), 1);
        let mut cap = 0u32;
        assert!(fc4_pollution_occupant_u32(
            p,
            52,
            INCINERATOR,
            prop::INCINERATOR,
            &mut cap
        ));
        assert_eq!(cap, 1000);
        // The per-cell setters write through to the game grid.
        assert!(fc4_pollution_set_cell(p, 26, 0, 0, 77));
        assert!(!fc4_pollution_set_cell(p, 99, 0, 0, 77));
    }
    // A month of days: the cones spread into the value grids, garbage goes
    // to the incinerator.
    for _ in 0..30 {
        message(p, msg::SIM_NEW_DAY, 0);
    }
    message(p, msg::SIM_NEW_MONTH, 0);
    let st = state_of(p);
    assert!(st.cycle_resets >= 1);
    assert!(st.grids[4].nonzero > 0, "air values: {:?}", st.grids[4]);
    assert!(st.last_month[0] > 0, "garbage produced: {:?}", st.last_month);
    assert!(g.props.borrow().contains_key(&(INCINERATOR, prop::INC_USED_LAST)));

    message(p, msg::OCCUPANT_REMOVED, INCINERATOR);
    assert_eq!(g.refs_of(INCINERATOR), 0);
    stop(p);
}

/// Requirement: the game reads the getters from a second thread during
/// OnTick. Each host write of a day and a month (properties, posts,
/// history, budget) reads the simulator from another thread and waits.
#[test]
fn getters_answer_from_another_thread_during_a_main_thread_call() {
    let g = Game::new();
    let p = started(&g);
    message(p, msg::OCCUPANT_INSERTED, POLLUTER);
    message(p, msg::OCCUPANT_INSERTED, INCINERATOR);
    g.probe.set(p as usize);
    for _ in 0..20 {
        message(p, msg::SIM_NEW_DAY, 0);
    }
    message(p, msg::SIM_NEW_MONTH, 0);
    g.probe.set(0);
    assert!(g.probes.get() > 0, "no host write happened");
    assert_eq!(g.probe_failures.get(), 0);
    stop(p);
}

/// A message delivered from inside a call (a host callback) is queued.
#[test]
fn messages_during_a_call_are_queued() {
    thread_local! {
        static TARGET: Cell<usize> = const { Cell::new(0) };
    }
    unsafe extern "C" fn reentrant_history(u: *mut c_void, series: u32, value: i32) {
        record_history(u, series, value);
        let t = TARGET.with(|t| t.replace(0));
        if t != 0 {
            message(t as *mut fc4_pollution, msg::OCCUPANT_INSERTED, INCINERATOR);
        }
    }
    let g = Game::new();
    let mut h = host(&g);
    h.record_history = Some(reentrant_history);
    let p = unsafe { fc4_pollution_create(&h, 0x51, 7) };
    assert!(unsafe { fc4_pollution_init(p) });
    TARGET.with(|t| t.set(p as usize));
    message(p, msg::SIM_NEW_MONTH, 0);
    assert_eq!(TARGET.with(Cell::get), 0, "the month recorded history");
    let st = state_of(p);
    assert_eq!(st.reentrant_calls, 1);
    assert_eq!(st.lists[1], 1, "the queued insertion was handled after the call");
    stop(p);
}

#[test]
fn record_round_trip_through_host_streams() {
    let g = Game::new();
    let p = started(&g);
    message(p, msg::OCCUPANT_INSERTED, POLLUTER);
    message(p, msg::OCCUPANT_INSERTED, INCINERATOR);
    for _ in 0..5 {
        message(p, msg::SIM_NEW_DAY, 0);
    }
    let before = state_of(p);

    let mut out = Out::default();
    let os = ostream(&mut out);
    assert!(unsafe { fc4_pollution_write(p, &os) });
    let air = unsafe { fc4_pollution_grid_by_index(p, 0) };
    assert!(out.refs.contains(&(air, INT16_GRID)));
    assert!(out.refs.iter().any(|r| r.0 == INCINERATOR) && out.refs.iter().any(|r| r.0 == DEPT));

    // Read runs before Init: the reader takes the references the stream hands over.
    let h = host(&g);
    let q = unsafe { fc4_pollution_create(&h, 0x52, 7) };
    let gp: *const Game = &*g;
    let take = move |r: &mut fc4_ref| unsafe {
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
    assert!(unsafe { fc4_pollution_read(q, &is) });
    let after = state_of(q);
    assert_eq!((after.initialized, after.loaded), (0, 1));
    let strip = |mut s: fc4_pollution_state| {
        s.initialized = 0;
        s.loaded = 0;
        s.cycle_resets = 0;
        s
    };
    assert_eq!(strip(after), strip(before));
    assert_eq!(g.refs_of(INCINERATOR), 2);
    assert_eq!(g.refs_of(DEPT), 2);

    // Destroying a loaded, never-initialised object gives its references back.
    unsafe { fc4_pollution_destroy(q) };
    assert_eq!(g.refs_of(INCINERATOR), 1);
    assert_eq!(g.refs_of(DEPT), 1);
    assert_eq!(g.grids.borrow()[&air].refs, 1);

    // A version above 4 fails and changes nothing.
    let bad = 5u16.to_le_bytes();
    let q = unsafe { fc4_pollution_create(&h, 0x53, 7) };
    let mut inp = In::new(&bad);
    let is = istream(&mut inp);
    assert!(!unsafe { fc4_pollution_read(q, &is) });
    assert_eq!(state_of(q).loaded, 0);
    unsafe { fc4_pollution_destroy(q) };
    stop(p);
    assert_eq!(g.refs_of(INCINERATOR), 0);
    assert_eq!(g.refs_of(DEPT), 0);
}

/// A record read while its eight grids are still the empty objects the game
/// made for the refs (their own records come later in the save). `Read`
/// (which warms the grids' geometry) and the plugin's Read status line must
/// not fix it at 0 × 0: once the game has read the grids, `Init` and the
/// days see the saved geometry (the protection in-game fault, run on
/// pollution's Read-time geometry reads).
#[test]
fn grids_read_after_the_record_get_their_geometry_before_init() {
    let g = Game::new();
    let p = started(&g);
    message(p, msg::OCCUPANT_INSERTED, POLLUTER);
    for _ in 0..5 {
        message(p, msg::SIM_NEW_DAY, 0);
    }
    let before = state_of(p);
    assert!(before.grids[0].tracts > 0);
    let mut out = Out::default();
    let os = ostream(&mut out);
    assert!(unsafe { fc4_pollution_write(p, &os) });

    let h = host(&g);
    let q = unsafe { fc4_pollution_create(&h, 0x52, 7) };
    let gp: *const Game = &*g;
    let take = move |r: &mut fc4_ref| unsafe {
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
    g.unread.set(true);
    assert!(unsafe { fc4_pollution_read(q, &is) });
    // What the plugin's Read status line sees: the grids, not their geometry.
    let at_read = state_of(q);
    assert_eq!(at_read.loaded, 1);
    assert!(at_read.grids.iter().all(|s| s.tracts == 0));
    assert!((0..8).all(|i| unsafe { fc4_pollution_grid_by_index(q, i) } != 0));

    // The game reads the grids' records, then calls Init.
    g.unread.set(false);
    assert!(unsafe { fc4_pollution_init(q) });
    let st = state_of(q);
    for i in 0..8 {
        assert_eq!(st.grids[i].tracts, before.grids[i].tracts, "grid {i}");
    }
    assert_eq!(st.tracts_per_day, before.tracts_per_day);
    // A day visits tracts again: the tract cursor moves.
    message(q, msg::SIM_NEW_DAY, 0);
    let day = state_of(q);
    assert_ne!((day.cell_x, day.cell_z), (st.cell_x, st.cell_z));
    stop(q);
    stop(p);
}

// ---- the shadow ---------------------------------------------------------------------------------------

/// What the plugin's capture stream produces: the original's `Write`, with
/// references as `u32 index + 1, u32 class` and each one queried for the
/// grid, occupant and department interfaces.
struct Capture {
    game: *const Game,
    bytes: Vec<u8>,
    refs: Vec<fc4_pollution_capture_ref>,
}

impl Capture {
    fn view(&self) -> fc4_pollution_capture {
        fc4_pollution_capture {
            struct_size: std::mem::size_of::<fc4_pollution_capture>() as u32,
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
    let g = &*c.game;
    let is_grid = g.grids.borrow().contains_key(&r.token);
    c.refs.push(fc4_pollution_capture_ref {
        grid: if is_grid { r.token } else { 0 },
        occupant: if !is_grid && r.token != DEPT { r.token } else { 0 },
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

fn capture(p: *const fc4_pollution, g: &Game) -> Box<Capture> {
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
    assert!(unsafe { fc4_pollution_write(p, &os) });
    c
}

unsafe extern "C" fn lines_sink(ctx: *mut c_void, data: *const u8, len: usize) {
    let s = String::from_utf8(std::slice::from_raw_parts(data, len).to_vec()).unwrap();
    (*(ctx as *mut Vec<String>)).push(s);
}

/// One shadowed delivery to the stand-in original `a`; `meddle` runs after
/// the original and before the after-capture.
fn shadowed(
    s: *mut fc4_pollution_shadow,
    a: *mut fc4_pollution,
    g: &Game,
    msg_type: u32,
    julian: i32,
    lines: &mut Vec<String>,
    meddle: impl FnOnce(),
) -> i32 {
    let before = capture(a, g);
    let fp = g.fingerprint();
    let r = unsafe { fc4_pollution_shadow_begin(s, msg_type, julian, &before.view()) };
    assert_eq!(r, 0, "{lines:?}");
    assert_eq!(fp, g.fingerprint(), "the shadow's run changed the game");
    message(a, msg_type, 0);
    meddle();
    let after = capture(a, g);
    unsafe {
        fc4_pollution_shadow_finish(
            s,
            &after.view(),
            Some(lines_sink),
            lines as *mut Vec<String> as *mut c_void,
        )
    }
}

fn shadow_stats(s: *mut fc4_pollution_shadow) -> fc4_pollution_shadow_stats {
    let mut st = fc4_pollution_shadow_stats::default();
    assert!(unsafe { fc4_pollution_shadow_get_stats(s, &mut st) });
    st
}

/// FreeCity4's simulator stands in for the original: the shadow, run on its
/// records and its game grids, must agree with it to the cell, through two
/// ends of a cycle (each replayed with the original's tract order) and a
/// month.
#[test]
fn shadow_agrees_with_an_identical_original() {
    let g = Game::new();
    let a = started(&g);
    message(a, msg::OCCUPANT_INSERTED, POLLUTER);
    message(a, msg::OCCUPANT_INSERTED, INCINERATOR);
    let h = host(&g);
    let s = unsafe { fc4_pollution_shadow_create(&h, 0) };
    assert!(!s.is_null());
    let mut lines: Vec<String> = Vec::new();
    for day in 0..31 {
        let r = shadowed(s, a, &g, msg::SIM_NEW_DAY, JULIAN + day, &mut lines, || {});
        assert_eq!(r, 0, "day {day}: {lines:#?}");
    }
    let r = shadowed(s, a, &g, msg::SIM_NEW_MONTH, JULIAN + 31, &mut lines, || {});
    assert_eq!(r, 0, "{lines:#?}");
    let st = shadow_stats(s);
    assert_eq!((st.deliveries, st.clean, st.incomplete), (32, 32, 0));
    assert!(st.reruns >= 2, "{st:?}");
    assert_eq!(st.last_mismatches, 0);
    assert_eq!(st.cells, 32 * (7 * 256 + 1024));
    assert_eq!((st.cell_mismatches, st.field_mismatches, st.prop_mismatches), (0, 0, 0));
    let month = lines.last().unwrap();
    assert!(
        month.starts_with("pollution shadow month to 2000-02-01: deliveries=32 (days 31, months 1) clean=32"),
        "{month}"
    );
    assert!(
        month.contains("mismatches=0;") && month.contains("first differing cells: none"),
        "{month}"
    );
    assert!(month.contains("airVal 0/"), "{month}");
    // Days at the end of a cycle say they were replayed.
    assert!(
        lines.iter().any(|l| l.contains("day: clean; end of cycle: replayed")),
        "{lines:#?}"
    );

    // A cell the original computes differently is reported with its place
    // and both values.
    let air = unsafe { fc4_pollution_grid_by_index(a, 0) };
    let mut more: Vec<String> = Vec::new();
    let r = shadowed(s, a, &g, msg::SIM_NEW_DAY, JULIAN + 32, &mut more, || {
        g.grids.borrow_mut().get_mut(&air).unwrap().data[3 * 16 + 4] += 7;
    });
    assert_eq!(r, 1, "{more:#?}");
    assert!(
        more[0].starts_with("pollution shadow 2000-02-02 day: mismatches=1; airSrc 1/256 max|d|=7"),
        "{more:#?}"
    );
    assert!(more[0].contains("t(3,4)"), "{more:#?}");
    let st = shadow_stats(s);
    assert_eq!((st.cell_mismatches, st.last_mismatches), (1, 1));

    // Not compared: a failed capture, and a delivery that is not a day or a month.
    unsafe {
        assert_eq!(
            fc4_pollution_shadow_begin(s, msg::SIM_BEGIN, JULIAN, std::ptr::null()),
            -2
        );
        assert_eq!(
            fc4_pollution_shadow_begin(s, msg::SIM_NEW_DAY, JULIAN + 33, std::ptr::null()),
            -1
        );
        let r = fc4_pollution_shadow_finish(
            s,
            std::ptr::null(),
            Some(lines_sink),
            &mut more as *mut Vec<String> as *mut c_void,
        );
        assert_eq!(r, -1);
        // Finish without a begin: nothing to compare, nothing counted.
        assert_eq!(
            fc4_pollution_shadow_finish(s, std::ptr::null(), None, std::ptr::null_mut()),
            -1
        );
    }
    assert!(
        more.last().unwrap().contains("INCOMPLETE (before: no capture)"),
        "{more:#?}"
    );
    let r = shadowed(s, a, &g, msg::SIM_NEW_MONTH, JULIAN + 34, &mut more, || {});
    assert_eq!(r, 0, "{more:#?}");
    let month = more.last().unwrap();
    assert!(
        month.starts_with("pollution shadow month to 2000-02-04: deliveries=3 (days 2, months 1) clean=1 incomplete=1"),
        "{month}"
    );
    assert!(
        month.contains("mismatches=1;") && month.contains("airSrc 1/512 max|d|=7"),
        "{month}"
    );
    assert!(
        month.contains("first differing cells: airSrc 2000-02-02 t(3,4) orig/ours="),
        "{month}"
    );
    assert!(month.contains("incomplete because: before: no capture x1"), "{month}");
    let st = shadow_stats(s);
    assert_eq!((st.deliveries, st.incomplete), (35, 1));

    unsafe { fc4_pollution_shadow_destroy(s) };
    stop(a);
    assert_eq!(g.refs_of(INCINERATOR), 0);
    assert_eq!(g.refs_of(DEPT), 0);
}

#[test]
fn host_trace_replays_pollution_cycle_and_original_mismatch() {
    let dir = crate::trace::test_record("pollution-roundtrip", || {
        let g = Game::new();
        let a = started(&g);
        message(a, msg::OCCUPANT_INSERTED, POLLUTER);
        message(a, msg::OCCUPANT_INSERTED, INCINERATOR);
        let h = host(&g);
        let s = unsafe { fc4_pollution_shadow_create(&h, 0) };
        let mut lines = Vec::new();
        for day in 0..31 {
            assert_eq!(shadowed(s, a, &g, msg::SIM_NEW_DAY, JULIAN + day, &mut lines, || {}), 0);
        }
        assert_eq!(
            shadowed(s, a, &g, msg::SIM_NEW_MONTH, JULIAN + 31, &mut lines, || {}),
            0
        );
        unsafe {
            fc4_pollution_shadow_destroy(s);
            fc4_pollution_destroy(a);
        }
    });
    assert!(crate::trace::run(&dir.join("manifest.tsv"), true, 2).unwrap());
}

#[test]
fn shadow_compares_occupant_properties_and_saved_fields() {
    let g = Game::new();
    let a = started(&g);
    message(a, msg::OCCUPANT_INSERTED, POLLUTER);
    message(a, msg::OCCUPANT_INSERTED, INCINERATOR);
    let h = host(&g);
    let s = unsafe { fc4_pollution_shadow_create(&h, 2) };
    let mut lines: Vec<String> = Vec::new();
    // A property the original's day wrote differently.
    let mut seen_prop = false;
    for day in 0..15 {
        let r = shadowed(s, a, &g, msg::SIM_NEW_DAY, JULIAN + day, &mut lines, || {
            if g.props.borrow().contains_key(&(INCINERATOR, prop::INC_USED)) && !seen_prop {
                seen_prop = true;
                g.set(INCINERATOR, prop::INC_USED, mem::UINT32, false, &[123_456u32]);
            }
        });
        if seen_prop {
            assert_eq!(r, 1, "{lines:#?}");
            assert!(
                lines.last().unwrap().contains(&format!(
                    "props 1/1 orig/ours [occ 0x7 prop {:#010X} 123456/",
                    prop::INC_USED
                )),
                "{lines:#?}"
            );
            break;
        }
        assert_eq!(r, 0, "{lines:#?}");
    }
    assert!(seen_prop, "the incinerator never took garbage");
    // A saved field: the original's garbage supply moved by another caller.
    let r = shadowed(s, a, &g, msg::SIM_NEW_DAY, JULIAN + 20, &mut lines, || unsafe {
        fc4_pollution_add_to_garbage_supply(a, 3);
    });
    assert!(r >= 1, "{lines:#?}");
    assert!(
        lines.last().unwrap().contains("fields orig/ours: garbageSupply="),
        "{lines:#?}"
    );
    let st = shadow_stats(s);
    assert!(st.prop_mismatches >= 1 && st.field_mismatches >= 1, "{st:?}");
    unsafe { fc4_pollution_shadow_destroy(s) };
    stop(a);
}

#[test]
fn shadow_struct_layouts() {
    assert_eq!(std::mem::size_of::<fc4_pollution_capture_ref>(), 32);
    assert_eq!(std::mem::size_of::<fc4_pollution_shadow_stats>(), 64);
    assert_eq!(std::mem::size_of::<fc4_pollution_state>(), 408);
    assert_eq!(std::mem::size_of::<fc4_pollution_budget_item>(), 32);
    assert_eq!(CLSID_POLLUTION_SIMULATOR, 0x8990_C065);
}
