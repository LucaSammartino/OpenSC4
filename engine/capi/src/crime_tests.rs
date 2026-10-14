// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The crime C ABI against a fake game: the game's grids through
//! `fc4_grid_api` (uint8 crime grids, uint16 population and float EQ grids,
//! the police grid), lots and their references, the agent and subscription
//! glue, the posts, persistence through host streams, re-entrant messages,
//! the any-thread getters (called from a second thread while the main thread
//! is inside a call) and the shadow's replay.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::ffi::c_void;

use fc4_core::crime::{msg, shipped_tuning, shipped_type_exemplars, CRIME_TYPE_GROUP, DEFAULT_NAN};

use crate::crime::*;
use crate::hostgrid::{fc4_cell_rect, fc4_grid_api};
use crate::landvalue::fc4_grid_geometry;
use crate::ordinance::fc4_key_visitor;
use crate::props::fc4_props;
use crate::stream::fc4_bytes_sink;
use crate::test_streams::{istream, ostream, In, Out};

const CELLS: i32 = 32;
const CLS_U8: u32 = 0x49B9_E602;
const CLS_U16: u32 = 0x49B9_E604;
const CLS_F32: u32 = 0x49B9_E60A;
const CRIME_GRID: u32 = 0x89D5_B895;
const EFFECT_GRID: u32 = 0x2A5B_A1B3;

struct Grid {
    kind: u32,
    tract_size: i32,
    data: Vec<f64>,
    refs: i32,
    shut: bool,
    instance: u32,
}

impl Grid {
    fn count(&self) -> i32 {
        CELLS / self.tract_size
    }
}

#[derive(Clone, Copy, Default)]
struct Lot {
    rect: [i32; 4],
    zone: u32,
    condition: u8,
    /// (token, purpose, wealth)
    building: Option<(u64, u32, u32)>,
}

#[derive(Default)]
struct Game {
    grids: RefCell<HashMap<u64, Grid>>,
    next_grid: Cell<u64>,
    log: RefCell<Vec<String>>,
    lots: RefCell<HashMap<u64, Lot>>,
    lot_refs: RefCell<HashMap<u64, i32>>,
    random: RefCell<VecDeque<u64>>,
    released: Cell<i32>,
    jailed: Cell<u32>,
    police: Cell<u64>,
    pops: Cell<[u64; 3]>,
    eqs: Cell<[u64; 3]>,
    /// (type, lot, name length)
    committed: RefCell<Vec<(u32, u64, usize)>>,
    map_updates: Cell<u32>,
    prop_ids: RefCell<HashMap<u32, u32>>,
    /// Re-entrancy test: the first commit post delivers SimIdle to `crime`.
    crime: Cell<usize>,
    reenter: Cell<bool>,
    /// Concurrency test: the stats call reads the getters from a second thread.
    concurrent: Cell<bool>,
    concurrent_seen: RefCell<Option<(u64, u32)>>,
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

impl Game {
    fn push(&self, s: String) {
        self.log.borrow_mut().push(s);
    }
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
    fn new_grid(&self, kind: u32, instance: u32, tract_size: i32, refs: i32) -> u64 {
        let t = self.next_grid.get() + 0x7000;
        self.next_grid.set(self.next_grid.get() + 0x10);
        let n = (CELLS / tract_size) as usize;
        self.grids.borrow_mut().insert(
            t,
            Grid {
                kind,
                tract_size,
                data: vec![0.0; n * n],
                refs,
                shut: false,
                instance,
            },
        );
        t
    }
    fn grid_refs(&self, t: u64) -> i32 {
        self.grids.borrow()[&t].refs
    }
    fn set(&self, t: u64, x: i32, z: i32, v: f64) {
        let mut g = self.grids.borrow_mut();
        let g = g.get_mut(&t).unwrap();
        let n = g.count();
        g.data[(x * n + z) as usize] = v;
    }
    fn get(&self, t: u64, x: i32, z: i32) -> f64 {
        let g = self.grids.borrow();
        let g = &g[&t];
        g.data[(x * g.count() + z) as usize]
    }
    fn lot_refs(&self, t: u64) -> i32 {
        self.lot_refs.borrow().get(&t).copied().unwrap_or(0)
    }
}

// ---- grids --------------------------------------------------------------------------------------------

unsafe extern "C" fn g_create(u: *mut c_void, clsid: u32, instance: u32, ts: i32) -> u64 {
    assert_eq!(clsid, CLS_U8);
    game(u).new_grid(clsid, instance, ts, 1)
}
unsafe extern "C" fn g_class(u: *mut c_void, t: u64) -> u32 {
    game(u).grids.borrow().get(&t).map_or(0, |g| g.kind)
}
unsafe extern "C" fn g_geometry(u: *mut c_void, t: u64, out: *mut fc4_grid_geometry) {
    let s = game(u).grids.borrow()[&t].tract_size;
    let w = 16.0 * s as f32;
    *out = fc4_grid_geometry {
        tract_size: s,
        shift: s.trailing_zeros() as i32,
        count_x: CELLS / s,
        count_z: CELLS / s,
        width_x: w,
        width_z: w,
        inv_width_x: 1.0 / w,
        inv_width_z: 1.0 / w,
    };
}
unsafe extern "C" fn g_get(u: *mut c_void, t: u64, kind: u32, x: i32, z: i32) -> f64 {
    assert_eq!(kind, game(u).grids.borrow()[&t].kind, "read with the grid's own class");
    game(u).get(t, x, z)
}
unsafe extern "C" fn g_set(u: *mut c_void, t: u64, _kind: u32, x: i32, z: i32, v: f64) {
    game(u).set(t, x, z, v);
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

// ---- glue -----------------------------------------------------------------------------------------------

unsafe extern "C" fn add_agent(u: *mut c_void, name: *const u8, len: usize, levels: *const u32, n: u32, flags: u32) {
    let name = String::from_utf8_lossy(std::slice::from_raw_parts(name, len)).into_owned();
    let levels = std::slice::from_raw_parts(levels, n as usize);
    game(u).push(format!("agent {name} {levels:?} {flags}"));
}
unsafe extern "C" fn remove_agent(u: *mut c_void) {
    game(u).push("remove_agent".into());
}
unsafe extern "C" fn subscribe(u: *mut c_void, ids: *const u32, n: u32) {
    let ids = std::slice::from_raw_parts(ids, n as usize);
    game(u).push(format!("subscribe {ids:X?}"));
}
unsafe extern "C" fn unsubscribe(u: *mut c_void, ids: *const u32, n: u32) {
    let ids = std::slice::from_raw_parts(ids, n as usize);
    game(u).push(format!("unsubscribe {ids:X?}"));
}
unsafe extern "C" fn acquire(u: *mut c_void) {
    game(u).push("acquire".into());
}
unsafe extern "C" fn release_services(u: *mut c_void) {
    game(u).push("release_services".into());
}
unsafe extern "C" fn add_prop_requester(u: *mut c_void) {
    game(u).push("prop_requester".into());
}

fn props_of(h: fc4_core::property::PropertyHolder) -> *mut fc4_props {
    Box::into_raw(Box::new(h)) as *mut fc4_props
}
unsafe extern "C" fn tuning(_: *mut c_void) -> *mut fc4_props {
    props_of(shipped_tuning())
}
unsafe extern "C" fn for_each_type_key(_: *mut c_void, visit: fc4_key_visitor, ctx: *mut c_void) {
    let visit = visit.unwrap();
    // Reversed and with a duplicate and a foreign group: the list is still
    // in ascending instance order, once each (§7 [choice]).
    let mut keys: Vec<u32> = shipped_type_exemplars().iter().map(|(i, _)| *i).collect();
    keys.reverse();
    keys.push(0x03);
    for k in keys {
        visit(ctx, 0x6534_284A, CRIME_TYPE_GROUP, k);
    }
    visit(ctx, 0x6534_284A, 0x1234_5678, 0x99);
}
unsafe extern "C" fn exemplar(_: *mut c_void, t: u32, g: u32, i: u32) -> *mut fc4_props {
    assert_eq!((t, g), (0x6534_284A, CRIME_TYPE_GROUP));
    shipped_type_exemplars()
        .into_iter()
        .find(|(k, _)| *k == i)
        .map_or(std::ptr::null_mut(), |(_, h)| props_of(h))
}
unsafe extern "C" fn ltext(_: *mut c_void, _t: u32, _g: u32, i: u32, sink: fc4_bytes_sink, ctx: *mut c_void) -> bool {
    // Only Generic's key exists (§7).
    if i != 0xEA5E_A698 {
        return false;
    }
    let s = b"generic";
    sink.unwrap()(ctx, s.as_ptr(), s.len());
    true
}
unsafe extern "C" fn population_grids(u: *mut c_void, out: *mut u64) {
    let g = game(u);
    for (k, t) in g.pops.get().iter().enumerate() {
        if *t != 0 {
            g.grids.borrow_mut().get_mut(t).unwrap().refs += 1; // one reference each (§6.1)
        }
        *out.add(k) = *t;
    }
}
unsafe extern "C" fn eq_grids(u: *mut c_void, out: *mut u64) {
    let g = game(u);
    for (k, t) in g.eqs.get().iter().enumerate() {
        if *t != 0 {
            g.grids.borrow_mut().get_mut(t).unwrap().refs += 1;
        }
        *out.add(k) = *t;
    }
}
unsafe extern "C" fn demand(_: *mut c_void, id: u32, v: *mut f32, max: *mut f32, min: *mut f32) -> bool {
    if id != 0x4100 {
        return false;
    }
    *v = -1000.0;
    *max = 1000.0;
    *min = -1000.0;
    true
}
unsafe extern "C" fn police_grid(u: *mut c_void) -> u64 {
    let g = game(u);
    let t = g.police.get();
    if t != 0 {
        g.grids.borrow_mut().get_mut(&t).unwrap().refs += 1; // handed over with one reference
    }
    t
}
unsafe extern "C" fn jail_population(u: *mut c_void) -> u32 {
    game(u).push("jail_population".into());
    game(u).jailed.get()
}
unsafe extern "C" fn reset_unjailed(u: *mut c_void) -> i32 {
    game(u).push("reset_unjailed".into());
    game(u).released.get()
}
unsafe extern "C" fn monthly_stats(u: *mut c_void) {
    let g = game(u);
    g.push("monthly_stats".into());
    if g.concurrent.get() {
        // The main thread is inside fc4_crime_do_message: a second thread
        // reads the getters (foundations §19), which must not touch the core.
        let p = g.crime.get();
        let seen = std::thread::spawn(move || unsafe {
            let p = p as *const fc4_crime;
            let mut ev = [fc4_crime_event::default(); 8];
            let n = fc4_crime_incidents(p, ev.as_mut_ptr(), 8);
            fc4_crime_hotspot(p, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
            fc4_crime_set_visuals(p, 0, true);
            (fc4_crime_grid(p), n + fc4_crime_average(p) + fc4_crime_monthly_count(p))
        })
        .join()
        .unwrap();
        *g.concurrent_seen.borrow_mut() = Some(seen);
    }
}
unsafe extern "C" fn ordinance_factor(_: *mut c_void, f: *mut f32) -> bool {
    *f = 0.5;
    true
}
unsafe extern "C" fn random_lot(u: *mut c_void) -> u64 {
    game(u).random.borrow_mut().pop_front().unwrap_or(0)
}
unsafe extern "C" fn lot_at(u: *mut c_void, x: i32, z: i32) -> u64 {
    let lots = game(u).lots.borrow();
    lots.iter()
        .find(|(_, l)| (l.rect[0]..=l.rect[2]).contains(&x) && (l.rect[1]..=l.rect[3]).contains(&z))
        .map_or(0, |(t, _)| *t)
}
unsafe extern "C" fn lot_rect(u: *mut c_void, lot: u64, out: *mut fc4_cell_rect) {
    let r = game(u).lots.borrow()[&lot].rect;
    *out = fc4_cell_rect {
        x1: r[0],
        z1: r[1],
        x2: r[2],
        z2: r[3],
    };
}
unsafe extern "C" fn lot_location(u: *mut c_void, lot: u64, x: *mut i32, z: *mut i32) -> bool {
    let r = game(u).lots.borrow()[&lot].rect;
    *x = r[0];
    *z = r[1];
    true
}
unsafe extern "C" fn lot_condition(u: *mut c_void, lot: u64) -> u8 {
    game(u).lots.borrow()[&lot].condition
}
unsafe extern "C" fn lot_building(u: *mut c_void, lot: u64, b: *mut u64, p: *mut u32, w: *mut u32) -> bool {
    match game(u).lots.borrow()[&lot].building {
        Some((t, purpose, wealth)) => {
            *b = t;
            *p = purpose;
            *w = wealth;
            true
        }
        None => false,
    }
}
unsafe extern "C" fn lot_center(u: *mut c_void, lot: u64, x: *mut i32, z: *mut i32) {
    let r = game(u).lots.borrow()[&lot].rect;
    *x = (r[0] + r[2]) / 2;
    *z = (r[1] + r[3]) / 2;
}
unsafe extern "C" fn zone_type(u: *mut c_void, x: i32, z: i32) -> u32 {
    let lots = game(u).lots.borrow();
    lots.values()
        .find(|l| (l.rect[0]..=l.rect[2]).contains(&x) && (l.rect[1]..=l.rect[3]).contains(&z))
        .map_or(0, |l| l.zone)
}
unsafe extern "C" fn add_ref(u: *mut c_void, t: u64) {
    *game(u).lot_refs.borrow_mut().entry(t).or_insert(0) += 1;
}
unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    *game(u).lot_refs.borrow_mut().entry(t).or_insert(0) -= 1;
}
unsafe extern "C" fn has_property(_: *mut c_void, _o: u64, _id: u32) -> bool {
    false
}
unsafe extern "C" fn property_bytes(_: *mut c_void, o: u64, id: u32, sink: fc4_bytes_sink, ctx: *mut c_void) -> bool {
    if id != 0xCA5B_9306 || o != 0xE0 {
        return false;
    }
    let b = [100u8, 8];
    sink.unwrap()(ctx, b.as_ptr(), 2);
    true
}
unsafe extern "C" fn occupant_position(_: *mut c_void, _o: u64, xyz: *mut f32) {
    // The centre of tract (4, 4) of a size-2 grid (32 world units per tract).
    for (k, v) in [144.0f32, 0.0, 144.0].iter().enumerate() {
        *xyz.add(k) = *v;
    }
}
unsafe extern "C" fn prop_incident(u: *mut c_void, prop: u32) -> u32 {
    game(u).prop_ids.borrow().get(&prop).copied().unwrap_or(0)
}
unsafe extern "C" fn layer(_: *mut c_void) -> bool {
    true
}
unsafe extern "C" fn ignite(u: *mut c_void, b: u64, cause: u32) {
    game(u).push(format!("ignite {b:X} {cause}"));
}
unsafe extern "C" fn post_committed(u: *mut c_void, type_id: u32, lot: u64, _name: *const u8, len: usize) {
    let g = game(u);
    g.committed.borrow_mut().push((type_id, lot, len));
    if g.reenter.replace(false) {
        let m = fc4_crime_message {
            msg_type: msg::SIM_IDLE,
            ..Default::default()
        };
        // Queued: the month is still being committed.
        assert!(fc4_crime_do_message(g.crime.get() as *mut fc4_crime, &m));
    }
}
unsafe extern "C" fn post_map(u: *mut c_void) {
    let g = game(u);
    g.map_updates.set(g.map_updates.get() + 1);
}

fn host(g: &Game) -> fc4_crime_host {
    let mut h: fc4_crime_host = unsafe { std::mem::zeroed() };
    h.struct_size = std::mem::size_of::<fc4_crime_host>() as u32;
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
    h.add_agent = Some(add_agent);
    h.remove_agent = Some(remove_agent);
    h.subscribe = Some(subscribe);
    h.unsubscribe = Some(unsubscribe);
    h.acquire_services = Some(acquire);
    h.release_services = Some(release_services);
    h.add_prop_requester = Some(add_prop_requester);
    h.tuning = Some(tuning);
    h.for_each_type_key = Some(for_each_type_key);
    h.exemplar = Some(exemplar);
    h.ltext = Some(ltext);
    h.population_grids = Some(population_grids);
    h.eq_grids = Some(eq_grids);
    h.demand = Some(demand);
    h.police_grid = Some(police_grid);
    h.jail_population = Some(jail_population);
    h.reset_unjailed_criminals = Some(reset_unjailed);
    h.process_monthly_crime_stats = Some(monthly_stats);
    h.ordinance_crime_factor = Some(ordinance_factor);
    h.random_lot = Some(random_lot);
    h.lot_at = Some(lot_at);
    h.lot_bounding_rect = Some(lot_rect);
    h.lot_location = Some(lot_location);
    h.lot_condition = Some(lot_condition);
    h.lot_building = Some(lot_building);
    h.lot_center_cell = Some(lot_center);
    h.zone_type = Some(zone_type);
    h.add_ref = Some(add_ref);
    h.release = Some(release);
    h.occupant_has_property = Some(has_property);
    h.occupant_property_bytes = Some(property_bytes);
    h.occupant_position = Some(occupant_position);
    h.prop_crime_incident_id = Some(prop_incident);
    h.disaster_layer_present = Some(layer);
    h.ignite = Some(ignite);
    h.post_crime_committed = Some(post_committed);
    h.post_crime_map_updated = Some(post_map);
    h
}

fn message(t: u32) -> fc4_crime_message {
    fc4_crime_message {
        msg_type: t,
        ..Default::default()
    }
}

fn state(p: *const fc4_crime) -> fc4_crime_state {
    let mut s = fc4_crime_state::default();
    assert!(unsafe { fc4_crime_get_state(p, &mut s) });
    s
}

/// Three residential lots of 2×2 cells, the random lot queue for one month,
/// and `released` = 5 so that crimes = trunc(5 × 0.66) = 3 (§6.4).
fn city(g: &Game) {
    let mut lots = g.lots.borrow_mut();
    for (t, x) in [(0x100u64, 4), (0x101, 10), (0x102, 20)] {
        lots.insert(
            t,
            Lot {
                rect: [x, 6, x + 1, 7],
                zone: 1,
                condition: 1,
                building: Some((t + 0x1000, 1, 1)),
            },
        );
    }
    g.released.set(5);
}

fn month(p: *mut fc4_crime, g: &Game) {
    g.random.borrow_mut().extend([0x100u64, 0x101, 0x102]);
    unsafe {
        fc4_crime_do_message(p, &message(msg::SIM_NEW_MONTH));
        for _ in 0..6 {
            fc4_crime_do_message(p, &message(msg::SIM_IDLE));
        }
    }
}

#[test]
fn null_and_size_checks() {
    unsafe {
        assert!(fc4_crime_create(std::ptr::null(), 1, 1).is_null());
        let g = Game::default();
        let mut h = host(&g);
        h.struct_size -= 4;
        assert!(fc4_crime_create(&h, 1, 1).is_null(), "too small");
        let mut h = host(&g);
        h.grids.struct_size = 0;
        assert!(fc4_crime_create(&h, 1, 1).is_null(), "grid api required");
        let n = std::ptr::null_mut();
        assert!(!fc4_crime_init(n));
        assert!(!fc4_crime_shutdown(n));
        assert!(fc4_crime_do_message(n, &message(msg::SIM_IDLE)));
        assert_eq!(
            (fc4_crime_grid(n), fc4_crime_average(n), fc4_crime_monthly_count(n)),
            (0, 0, 0)
        );
        assert!(!fc4_crime_hotspot(
            n,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut()
        ));
        assert_eq!(fc4_crime_incidents(n, std::ptr::null_mut(), 0), 0);
        assert!(!fc4_crime_set_visuals(n, 0, true));
        assert!(!fc4_crime_get_visuals(n, 0));
        assert_eq!(fc4_crime_requested_share(n, 0, 0, 1, 1), 0);
        assert_eq!(fc4_crime_select_prop(n, std::ptr::null(), 0, 0, 0), 0);
        fc4_crime_destroy(n);
        assert!(fc4_crime_shadow_create(std::ptr::null()).is_null());
    }
}

/// §4.1, §4.2, test 18: the glue in order, Init twice, Shutdown releases.
#[test]
fn lifecycle() {
    let g = Game::default();
    let h = host(&g);
    unsafe {
        let p = fc4_crime_create(&h, 0xC0, 7);
        assert!(!p.is_null());
        assert!(fc4_crime_init(p));
        assert!(fc4_crime_init(p));
        assert_eq!(
            g.take(),
            vec![
                "agent cSC4CrimeSimulator [4, 6, 10] 2".to_string(),
                "subscribe [99EF1142, 99EF1143, C2C09554, C53D10AA]".into(),
                "acquire".into(),
                "prop_requester".into(),
            ]
        );
        let s = state(p);
        assert_eq!((s.initialized, s.pipeline_state, s.types, s.tract_size), (1, 6, 14, 2));
        assert_eq!((s.decay, s.reoffend, s.ordinance_factor), (0.8, 0.66, 1.0));
        let grid = fc4_crime_grid(p);
        assert_eq!(grid, s.crime_grid);
        assert_ne!(s.effect_grid, 0);
        let instances = {
            let gr = g.grids.borrow();
            (gr[&s.crime_grid].instance, gr[&s.effect_grid].instance)
        };
        assert_eq!(instances, (CRIME_GRID, EFFECT_GRID));
        assert!(fc4_crime_shutdown(p));
        assert!(fc4_crime_shutdown(p));
        assert_eq!(
            g.take(),
            vec![
                "release_services".to_string(),
                "unsubscribe [C2C09554, C53D10AA, 99EF1142, 99EF1143]".into(),
                "remove_agent".into(),
            ]
        );
        for t in [s.crime_grid, s.effect_grid] {
            let gr = g.grids.borrow();
            assert!(gr[&t].shut, "shut down once by Shutdown");
            assert_eq!(gr[&t].refs, 0, "released");
        }
        assert_eq!(fc4_crime_grid(p), 0, "published after Shutdown");
        fc4_crime_destroy(p);
    }
}

/// §5, §6, §10, §11: one month through messages, the posts, the snapshot,
/// re-entrancy, and the getters from a second thread mid-call.
#[test]
fn month_posts_and_snapshot() {
    let g = Game::default();
    city(&g);
    let h = host(&g);
    unsafe {
        let p = fc4_crime_create(&h, 0xC0, 0x1234);
        g.crime.set(p as usize);
        fc4_crime_init(p);
        g.take();
        // Nothing before the first month message (test 13).
        fc4_crime_do_message(p, &message(msg::SIM_IDLE));
        assert_eq!(state(p).pipeline_state, 6);
        g.reenter.set(true);
        g.concurrent.set(true);
        month(p, &g);
        g.concurrent.set(false);
        assert_eq!(
            g.take(),
            vec!["jail_population", "reset_unjailed", "monthly_stats"],
            "police calls in order, once (§6.4)"
        );
        let s = state(p);
        assert_eq!(s.pipeline_state, 6);
        assert_eq!(s.crimes_to_commit, 3, "trunc(5 × 0.66f) = 3");
        // No police grid: p = curve(0) = 0, nothing stopped (§6.6).
        assert_eq!((s.monthly_crime_count, s.incidents), (3, 3));
        assert_eq!(fc4_crime_monthly_count(p), 3);
        assert_eq!(g.map_updates.get(), 1);
        let committed = g.committed.borrow().clone();
        assert_eq!(
            committed.iter().map(|c| c.1).collect::<Vec<_>>(),
            vec![0x100, 0x101, 0x102]
        );
        for lot in [0x100, 0x101, 0x102] {
            assert_eq!(g.lot_refs(lot), 1, "one reference per incident");
        }
        assert_eq!(s.reentrant_calls, 0, "the re-entrant message was queued, not run");
        // The concurrent reader saw the state published before this call.
        assert_eq!(*g.concurrent_seen.borrow(), Some((s.crime_grid, 0)));
        // Stamps: lot 0x100 covers tracts (2, 3); severities are ≥ 40.
        let v = g.get(s.crime_grid, 2, 3);
        assert!(v >= 40.0);
        let mut value = 0u8;
        let (mut x, mut z) = (0, 0);
        assert!(fc4_crime_hotspot(p, &mut value, &mut x, &mut z));
        assert_eq!(f64::from(value), g.get(s.crime_grid, x / 2, z / 2));
        // §6.9: the zoned tracts are the three lots' tracts.
        assert!(fc4_crime_average(p) >= 40);
        // §11.4: lot centres, visuals false, icon keys.
        let mut ev = [fc4_crime_event::default(); 4];
        assert_eq!(fc4_crime_incidents(p, ev.as_mut_ptr(), 4), 3);
        assert_eq!((ev[0].cell_x, ev[0].cell_z, ev[0].has_lot), (4, 6, 1));
        assert_eq!((ev[2].cell_x, ev[2].cell_z), (20, 6));
        assert!(ev[..3]
            .iter()
            .all(|e| e.visuals_created == 0 && e.icon_type == 0x7AB5_0E44));
        assert!(fc4_crime_set_visuals(p, 1, true));
        assert!(fc4_crime_get_visuals(p, 1));
        assert!(!fc4_crime_set_visuals(p, 3, true));
        assert!(!fc4_crime_get_visuals(p, 3));
        // The flags survive a call that does not change the incidents.
        fc4_crime_do_message(p, &message(msg::SIM_IDLE));
        assert!(fc4_crime_get_visuals(p, 1));
        // A new month clears them with the incidents.
        month(p, &g);
        assert!(!fc4_crime_get_visuals(p, 1));
        fc4_crime_shutdown(p);
        for lot in [0x100, 0x101, 0x102] {
            assert_eq!(g.lot_refs(lot), 0, "released by Shutdown");
        }
        fc4_crime_destroy(p);
    }
}

/// §6.1-§6.4 through the uint16 / float grids, §5 ordinance, §6.10 effect,
/// §6.6 with the police grid; every grid reference balanced.
#[test]
fn census_police_and_effect() {
    let g = Game::default();
    city(&g);
    let pop = g.new_grid(CLS_U16, 0, 2, 1);
    let eq = g.new_grid(CLS_F32, 0, 2, 1);
    g.set(pop, 0, 0, 1000.0);
    g.set(eq, 0, 0, 49.99);
    g.set(pop, 1, 0, 234.0);
    g.set(eq, 1, 0, 50.0);
    g.pops.set([pop, 0, 0]);
    g.eqs.set([eq, 0, 0]);
    let police = g.new_grid(CLS_U8, 0, 2, 1);
    g.grids.borrow_mut().get_mut(&police).unwrap().data.fill(255.0);
    g.police.set(police);
    let h = host(&g);
    unsafe {
        let p = fc4_crime_create(&h, 0xC0, 9);
        fc4_crime_init(p);
        fc4_crime_do_message(p, &message(msg::ORDINANCE_UPDATED));
        assert_eq!(state(p).ordinance_factor, 0.5);
        month(p, &g);
        let s = state(p);
        assert_eq!(&s.census[0..4], &[1000, 234, 0, 0]);
        assert_eq!(s.jobless[0], -1.0);
        assert_eq!(s.criminal_base, 611, "0.4·1000 + 0.375·234 + 0.1·1234 = 611.15");
        for t in [pop, eq, police] {
            assert_eq!(g.grid_refs(t), 1, "the borrowed grids' references are given back");
        }
        // Occupant 0xE0 has the effect {100, 8} at tract (4, 4) (test 11).
        let mut m = message(msg::OCCUPANT_INSERTED);
        m.occupant = 0xE0;
        fc4_crime_do_message(p, &m);
        assert_eq!(g.get(s.effect_grid, 4, 4), 100.0);
        let ring = g.get(s.effect_grid, 5, 4);
        assert!(ring > 0.0 && ring < 100.0, "{ring}");
        m.msg_type = msg::OCCUPANT_REMOVED;
        fc4_crime_do_message(p, &m);
        assert_eq!(g.get(s.effect_grid, 4, 4), 0.0);
        fc4_crime_shutdown(p);
        fc4_crime_destroy(p);
    }
}

/// §11.3 and test 22 through the pure entry point.
#[test]
fn nearest_of_entry_point() {
    let o = |token: u64, pos: [f32; 3], ok: u8| fc4_crime_nearby {
        token,
        occupant_type: 0x496E_5783,
        in_crime_group: 1,
        answers_iid: ok,
        position: pos,
        ..Default::default()
    };
    let list = [
        o(1, [5.0, 0.0, 0.0], 1),
        o(2, [3.0, 0.0, 0.0], 0),
        o(3, [4.0, 0.0, 0.0], 1),
        o(9, [2109.0, 1812.0, 3693.0], 1),
    ];
    let pos = [0.0f32; 3];
    unsafe {
        assert_eq!(
            fc4_crime_nearest_of(FC4_CRIME_NEAREST_SIM, pos.as_ptr(), 10.0, list.as_ptr(), 4),
            2
        );
        assert_eq!(
            fc4_crime_nearest_of(FC4_CRIME_NEAREST_PROP, pos.as_ptr(), 10.0, list.as_ptr(), 4),
            -1
        );
        let far = &list[3..];
        assert_eq!(
            fc4_crime_nearest_of(
                FC4_CRIME_NEAREST_SIM,
                pos.as_ptr(),
                f32::from_bits(0x4590_75B4),
                far.as_ptr(),
                1
            ),
            0
        );
        assert_eq!(
            fc4_crime_nearest_of(
                FC4_CRIME_NEAREST_SIM,
                pos.as_ptr(),
                f32::from_bits(0x4590_75B3),
                far.as_ptr(),
                1
            ),
            -1
        );
        assert_eq!(fc4_crime_nearest_of(0, std::ptr::null(), 1.0, list.as_ptr(), 4), -1);
        assert_eq!(fc4_crime_nearest_of(0, pos.as_ptr(), 1.0, std::ptr::null(), 0), -1);
    }
}

/// Test 16 through the prop requester entry points.
#[test]
fn prop_requester() {
    let g = Game::default();
    city(&g);
    g.prop_ids.borrow_mut().insert(0xB, 0x11);
    let h = host(&g);
    unsafe {
        let p = fc4_crime_create(&h, 0xC0, 3);
        fc4_crime_init(p);
        let grid = fc4_crime_grid(p);
        g.grids.borrow_mut().get_mut(&grid).unwrap().data.fill(51.0);
        assert_eq!(fc4_crime_requested_share(p, 0, 0, 8, 8), 4);
        g.grids.borrow_mut().get_mut(&grid).unwrap().data.fill(255.0);
        assert_eq!(fc4_crime_requested_share(p, 0, 0, 8, 8), 10);
        let props = [0xAu32, 0xB];
        // No incident at the cell: the generic prop.
        assert_eq!(fc4_crime_select_prop(p, props.as_ptr(), 2, 4, 6), 0xA);
        assert_eq!(fc4_crime_select_prop(p, std::ptr::null(), 0, 4, 6), 0);
        fc4_crime_shutdown(p);
        fc4_crime_destroy(p);
    }
}

/// §12, test 17: write → read (before Init) → write is byte-identical; the
/// loaded grids are used and Init does not register the prop requester.
#[test]
fn save_round_trip() {
    let g = Game::default();
    city(&g);
    let h = host(&g);
    unsafe {
        let p = fc4_crime_create(&h, 0xC0, 5);
        fc4_crime_init(p);
        month(p, &g);
        let s = state(p);
        let mut out = Out::default();
        assert!(fc4_crime_write(p, &ostream(&mut out)));
        assert_eq!(out.refs.len(), 5, "two grids and three lots");
        assert_eq!(out.refs[0], (s.crime_grid, CLS_U8));

        // The stream hands over one reference per object read.
        for (t, _) in &out.refs {
            if g.grids.borrow().contains_key(t) {
                g.grids.borrow_mut().get_mut(t).unwrap().refs += 1;
            } else {
                add_ref(&g as *const Game as *mut c_void, *t);
            }
        }
        let q = fc4_crime_create(&h, 0xC1, 5);
        let mut inp = In::new(&out.bytes);
        assert!(fc4_crime_read(q, &istream(&mut inp)));
        let r = state(q);
        assert_eq!((r.loaded_from_save, r.incidents, r.crime_grid), (1, 3, s.crime_grid));
        assert_eq!(fc4_crime_incident_count(q), 3, "published by Read");
        g.take();
        fc4_crime_init(q);
        assert!(!g.take().contains(&"prop_requester".to_string()), "a loaded city");
        let r = state(q);
        assert_eq!((r.hotspot_value, r.ordinance_factor, r.pipeline_state), (0, 1.0, 6));
        let mut again = Out::default();
        assert!(fc4_crime_write(q, &ostream(&mut again)));
        // The hotspot was reset by Init (§4.1 step 9): compare the rest.
        assert_eq!(again.bytes.len(), out.bytes.len());
        let mut v6 = out.bytes.clone();
        v6[0] = 7;
        let mut bad = In::new(&v6);
        assert!(!fc4_crime_read(q, &istream(&mut bad)), "version 7 rejected");
        fc4_crime_shutdown(q);
        fc4_crime_destroy(q);
        fc4_crime_shutdown(p);
        fc4_crime_destroy(p);
    }
}

// ---- the shadow -----------------------------------------------------------------------------------------

fn shadow_host(g: &Game) -> fc4_crime_host {
    let mut h: fc4_crime_host = unsafe { std::mem::zeroed() };
    h.struct_size = std::mem::size_of::<fc4_crime_host>() as u32;
    h.user = g as *const Game as *mut c_void;
    h.tuning = Some(tuning);
    h.for_each_type_key = Some(for_each_type_key);
    h.exemplar = Some(exemplar);
    h.ltext = Some(ltext);
    h
}

#[test]
fn shadow_replay() {
    let g = Game::default();
    let h = shadow_host(&g);
    unsafe {
        let s = fc4_crime_shadow_create(&h);
        assert!(!s.is_null());
        let mut info = fc4_crime_shadow_info::default();
        assert!(fc4_crime_shadow_get_info(s, &mut info));
        assert_eq!(
            (info.types, info.tuning_found, info.tract_size, info.decay),
            (14, 1, 2, 0.8)
        );
        // Test 9: TPing (0x13) needs wealth $$ or $$$; Arson (0x05) condition 4.
        assert_eq!(fc4_crime_shadow_candidate(s, 0x13, 1, true, 1, 1, 1), 0);
        assert_eq!(fc4_crime_shadow_candidate(s, 0x13, 1, true, 1, 2, 1), 1);
        assert_eq!(fc4_crime_shadow_candidate(s, 0x05, 1, true, 1, 1, 4), 1);
        assert_eq!(fc4_crime_shadow_candidate(s, 0x05, 1, true, 1, 1, 1), 0);
        assert_eq!(fc4_crime_shadow_candidate(s, 0x77, 1, true, 1, 1, 1), -1);

        let n = 16usize;
        let mut pre = vec![0u8; n * n];
        pre[3 * n + 3] = 200; // tract (3, 3), the hotspot
        pre[6 * n + 5] = 200;
        let incidents = [
            // Test 8: cells (10, 10)-(13, 11) → tracts x 5..6, z 5; Fighting (60).
            fc4_crime_shadow_incident {
                rect: fc4_cell_rect {
                    x1: 10,
                    z1: 10,
                    x2: 13,
                    z2: 11,
                },
                type_id: 0x03,
                has_rect: 1,
                pad: [0; 3],
            },
            fc4_crime_shadow_incident {
                type_id: 0x77,
                has_rect: 1,
                ..Default::default()
            },
            fc4_crime_shadow_incident {
                type_id: 0x11,
                has_rect: 0,
                ..Default::default()
            },
        ];
        let mut zones = vec![0u32; n * n];
        zones[3 * n + 3] = 1;
        zones[5 * n + 5] = 1;
        let input = fc4_crime_shadow_month_in {
            struct_size: std::mem::size_of::<fc4_crime_shadow_month_in>() as u32,
            geometry: fc4_grid_geometry {
                tract_size: 2,
                shift: 1,
                count_x: n as i32,
                count_z: n as i32,
                width_x: 32.0,
                width_z: 32.0,
                inv_width_x: 1.0 / 32.0,
                inv_width_z: 1.0 / 32.0,
            },
            pre: pre.as_ptr(),
            hotspot_x: 3,
            hotspot_z: 3,
            hotspot_value: 200,
            incidents: incidents.as_ptr(),
            incident_count: 3,
            zones: zones.as_ptr(),
        };
        let mut post = vec![0u8; n * n];
        let mut out = fc4_crime_shadow_month_out::default();
        assert!(fc4_crime_shadow_month(s, &input, post.as_mut_ptr(), &mut out));
        assert_eq!(post[3 * n + 3], 160, "decayed at 0.8");
        assert_eq!(
            (post[5 * n + 5], post[6 * n + 5]),
            (60, 220),
            "0 + 60, 200 → 160 → + 60"
        );
        assert_eq!(
            (out.count, out.unknown_types, out.no_rect, out.severity_sum),
            (2, 1, 1, 100)
        );
        assert_eq!((out.hotspot_x, out.hotspot_z, out.hotspot_value), (6, 5, 220));
        assert_eq!(
            (out.average_computed, f32::from_bits(out.average_bits)),
            (1, (160.0 + 60.0) / 2.0)
        );
        let unzoned = vec![0u32; n * n];
        let input2 = fc4_crime_shadow_month_in {
            zones: unzoned.as_ptr(),
            ..input
        };
        assert!(fc4_crime_shadow_month(s, &input2, post.as_mut_ptr(), &mut out));
        assert_eq!(out.average_bits, DEFAULT_NAN);
        fc4_crime_shadow_destroy(s);
    }
}
