// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The disaster layer through the C ABI (docs/specs/disasters.md): a fake
//! game behind `fc4_disasters_host`.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{c_char, c_void, CStr};

use crate::budget::fc4_token_callback;
use crate::disasters::*;
use crate::hostgrid::fc4_cell_rect;
use crate::props::{fc4_props_create, fc4_props_destroy, fc4_props_set_u32s};

const FIRE: u32 = 0x621C_DA33;
const RIOT: u32 = 0xCA38_0B06;
const TOXIC: u32 = 0xEBFB_0404;
const PIPE: u32 = 0x4A13_32E8;
const EFFECT: u32 = 0x6A7B_4397;
const BUILDING: u32 = 0x2781_28A0;
const FLAMMABILITY: u32 = 0x2924_4DB5;
const BURNING: u32 = 0x69DB_EBE1;
const FLAME: u32 = 0xC9B6_B73D;
const D3: u32 = 0xA9B6_ECD3;

/// An occupant: (cell, in the occupant manager, burning).
type Occ = ((i32, i32), bool, bool);

#[derive(Default)]
struct Game {
    log: RefCell<Vec<String>>,
    now: Cell<u64>,
    emergency: Cell<i32>,
    saves_off: Cell<i32>,
    next: Cell<u64>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    refs: RefCell<BTreeMap<u64, i32>>,
    roads: RefCell<BTreeSet<(i32, i32)>>,
    leaders: RefCell<BTreeMap<u64, (i32, i32)>>,
    pipes: RefCell<BTreeMap<(i32, i32), u64>>,
    neighbours: RefCell<Vec<(i32, i32)>>,
    neighbour_calls: Cell<u32>,
}

impl Game {
    fn new() -> Box<Game> {
        let g = Box::new(Game::default());
        g.next.set(0x1000);
        g
    }
    fn user(&self) -> *mut c_void {
        self as *const Game as *mut c_void
    }
    fn log(&self, s: impl Into<String>) {
        self.log.borrow_mut().push(s.into());
    }
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
    fn token(&self) -> u64 {
        let t = self.next.get();
        self.next.set(t + 1);
        t
    }
    fn building(&self, o: u64, x: i32, z: i32) {
        self.occs.borrow_mut().insert(o, ((x, z), true, false));
    }
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

unsafe extern "C" fn now_ms(u: *mut c_void) -> u64 {
    game(u).now.get()
}
unsafe extern "C" fn emergency_pause(u: *mut c_void) {
    let g = game(u);
    g.emergency.set(g.emergency.get() + 1);
    g.log("pause");
}
unsafe extern "C" fn emergency_resume(u: *mut c_void) {
    let g = game(u);
    g.emergency.set(g.emergency.get() - 1);
    g.log("resume");
}
unsafe extern "C" fn disable_save(u: *mut c_void) {
    let g = game(u);
    g.saves_off.set(g.saves_off.get() + 1);
}
unsafe extern "C" fn enable_save(u: *mut c_void) {
    let g = game(u);
    g.saves_off.set(g.saves_off.get() - 1);
}
unsafe extern "C" fn sim_paused(u: *mut c_void) -> bool {
    game(u).emergency.get() > 0
}
unsafe extern "C" fn sim_emergency_paused(u: *mut c_void) -> bool {
    game(u).emergency.get() > 0
}
unsafe extern "C" fn city_established(_: *mut c_void) -> bool {
    true
}
unsafe extern "C" fn city_cells(_: *mut c_void, x: *mut u32, z: *mut u32) {
    *x = 64;
    *z = 64;
}
unsafe extern "C" fn cell_width(_: *mut c_void) -> f32 {
    16.0
}
unsafe extern "C" fn post(u: *mut c_void, p: *const fc4_disaster_post) {
    let p = &*p;
    game(u).log(format!(
        "post {:08X} {} {} {} {:X} {}",
        p.msg_type, p.data1, p.data2, p.data3, p.data4, p.void1
    ));
}
unsafe extern "C" fn subscribe(u: *mut c_void, kind: u32, id: u32, m: u32) {
    game(u).log(format!("subscribe {kind} {id:X} {m:08X}"));
}
unsafe extern "C" fn register_tick(u: *mut c_void, i: u32, p: u32) {
    game(u).log(format!("tick {i} {p:X}"));
}
unsafe extern "C" fn record_history(u: *mut c_void, s: u32, v: i32) {
    game(u).log(format!("history {s:08X} {v}"));
}
unsafe extern "C" fn add_ref(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_default() += 1;
}
unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_default() -= 1;
}
unsafe extern "C" fn set_fire_dispatch_effectiveness(u: *mut c_void, v: f32) {
    game(u).log(format!("fire_effectiveness {v}"));
}
unsafe extern "C" fn occupant_type(u: *mut c_void, o: u64) -> u32 {
    if game(u).occs.borrow().contains_key(&o) {
        BUILDING
    } else {
        0
    }
}
unsafe extern "C" fn occupant_in_manager(u: *mut c_void, o: u64) -> bool {
    game(u).occs.borrow().get(&o).is_some_and(|x| x.1)
}
unsafe extern "C" fn occupant_cell(u: *mut c_void, o: u64, x: *mut i32, z: *mut i32) -> bool {
    match game(u).occs.borrow().get(&o) {
        Some(((cx, cz), true, _)) => {
            *x = *cx;
            *z = *cz;
            true
        }
        _ => false,
    }
}
unsafe extern "C" fn occupant_cell_rect(u: *mut c_void, o: u64, out: *mut fc4_cell_rect) {
    if let Some(((x, z), _, _)) = game(u).occs.borrow().get(&o) {
        *out = fc4_cell_rect {
            x1: *x,
            z1: *z,
            x2: *x,
            z2: *z,
        };
    }
}
unsafe extern "C" fn occupant_prop_f32(u: *mut c_void, o: u64, id: u32, out: *mut f32) -> bool {
    if id == FLAMMABILITY && game(u).occs.borrow().contains_key(&o) {
        *out = 100.0;
        return true;
    }
    false
}
unsafe extern "C" fn occupant_prop_bool(u: *mut c_void, o: u64, id: u32, out: *mut bool) -> bool {
    match game(u).occs.borrow().get(&o) {
        Some((_, _, burning)) if id == BURNING => {
            *out = *burning;
            true
        }
        _ => false,
    }
}
unsafe extern "C" fn occupants_in_rect(
    u: *mut c_void,
    r: *const fc4_cell_rect,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) {
    let r = &*r;
    let list: Vec<u64> = game(u)
        .occs
        .borrow()
        .iter()
        .filter(|(_, ((x, z), m, _))| *m && *x >= r.x1 && *x <= r.x2 && *z >= r.z1 && *z <= r.z2)
        .map(|(t, _)| *t)
        .collect();
    if let Some(f) = cb {
        for t in list {
            f(ctx, t);
        }
    }
}
unsafe extern "C" fn occupant_lot(_: *mut c_void, o: u64) -> u64 {
    o + 0x10_0000
}
unsafe extern "C" fn create_fire_occupant(u: *mut c_void, i: u32, target: u64, id: u32, top: u32, side: u32) -> u64 {
    let g = game(u);
    let t = g.token();
    if let Some(x) = g.occs.borrow_mut().get_mut(&target) {
        x.2 = true;
    }
    g.log(format!("create_fire {i} {t:X} {target:X} {id:X} {top} {side}"));
    t
}
unsafe extern "C" fn remove_fire_occupant(u: *mut c_void, fire: u64, target: u64) {
    let g = game(u);
    if let Some(x) = g.occs.borrow_mut().get_mut(&target) {
        x.2 = false;
    }
    g.log(format!("remove_fire {fire:X} {target:X}"));
}
unsafe extern "C" fn demolish_lot(u: *mut c_void, o: u64, lot: u64, flags: u32) {
    game(u).log(format!("demolish {o:X} {lot:X} {flags:X}"));
}
unsafe extern "C" fn play_effect_at(u: *mut c_void, name: *const c_char, _x: f32, _z: f32) {
    let n = CStr::from_ptr(name).to_string_lossy().into_owned();
    game(u).log(format!("effect_at {n}"));
}
unsafe extern "C" fn is_road_cell(u: *mut c_void, x: i32, z: i32) -> bool {
    game(u).roads.borrow().contains(&(x, z))
}
unsafe extern "C" fn cell_corner_position(_: *mut c_void, x: i32, z: i32, wx: *mut f32, wz: *mut f32) {
    *wx = x as f32 * 16.0;
    *wz = z as f32 * 16.0;
}
unsafe extern "C" fn create_riot_leader(u: *mut c_void, x: f32, z: f32) -> u64 {
    let g = game(u);
    let t = g.token();
    g.leaders.borrow_mut().insert(t, ((x / 16.0) as i32, (z / 16.0) as i32));
    g.log(format!("leader {t:X} {x} {z}"));
    t
}
unsafe extern "C" fn network_occupant_at(u: *mut c_void, x: i32, z: i32) -> u64 {
    if game(u).roads.borrow().contains(&(x, z)) {
        0x7000 + x as u64
    } else {
        0
    }
}
unsafe extern "C" fn unset_removal_flags(u: *mut c_void, o: u64, flags: u32) {
    game(u).log(format!("unset_removal {o:X} {flags}"));
}
unsafe extern "C" fn set_removal_flags(u: *mut c_void, o: u64, flags: u32) {
    game(u).log(format!("set_removal {o:X} {flags}"));
}
unsafe extern "C" fn automaton_cell(u: *mut c_void, a: u64, x: *mut i32, z: *mut i32) -> bool {
    match game(u).leaders.borrow().get(&a) {
        Some((cx, cz)) => {
            *x = *cx;
            *z = *cz;
            true
        }
        None => false,
    }
}
unsafe extern "C" fn create_toxic_occupant(u: *mut c_void, i: u32, x: i32, z: i32, amount: i32) -> u64 {
    let g = game(u);
    let t = g.token();
    g.log(format!("toxic {i} {t:X} {x} {z} {amount}"));
    t
}
unsafe extern "C" fn remove_toxic_occupant(u: *mut c_void, o: u64) {
    game(u).log(format!("remove_toxic {o:X}"));
}
unsafe extern "C" fn find_watered_pipe(u: *mut c_void, x: i32, z: i32) -> u64 {
    game(u).pipes.borrow().get(&(x, z)).copied().unwrap_or(0)
}
unsafe extern "C" fn pipe_cell(u: *mut c_void, p: u64, x: *mut i32, z: *mut i32) {
    if let Some((c, _)) = game(u).pipes.borrow().iter().find(|(_, t)| **t == p) {
        *x = c.0;
        *z = c.1;
    }
}
unsafe extern "C" fn view_level(_: *mut c_void) -> u32 {
    1
}
unsafe extern "C" fn create_leak_effect(_: *mut c_void, _p: u64, _level: u32) -> bool {
    true
}
unsafe extern "C" fn connected_pipe_cells(u: *mut c_void, _p: u64, out: *mut i32, cap: usize) -> usize {
    let g = game(u);
    g.neighbour_calls.set(g.neighbour_calls.get() + 1);
    let n = g.neighbours.borrow();
    for (i, (x, z)) in n.iter().take(cap).enumerate() {
        *out.add(2 * i) = *x;
        *out.add(2 * i + 1) = *z;
    }
    n.len()
}
unsafe extern "C" fn demolish_pipe(u: *mut c_void, p: u64, flags: u32) {
    let g = game(u);
    g.log(format!("demolish_pipe {p:X} {flags:X}"));
    g.pipes.borrow_mut().retain(|_, t| *t != p);
}

/// A table with every callback NULL except the header.
fn empty_host(g: &Game) -> fc4_disasters_host {
    // SAFETY: an all-zero table is valid (every callback None, user NULL).
    let mut h: fc4_disasters_host = unsafe { std::mem::zeroed() };
    h.struct_size = std::mem::size_of::<fc4_disasters_host>() as u32;
    h.user = g.user();
    h
}

fn host(g: &Game) -> fc4_disasters_host {
    let mut h = empty_host(g);
    h.now_ms = Some(now_ms);
    h.emergency_pause = Some(emergency_pause);
    h.emergency_resume = Some(emergency_resume);
    h.disable_save = Some(disable_save);
    h.enable_save = Some(enable_save);
    h.sim_paused = Some(sim_paused);
    h.sim_emergency_paused = Some(sim_emergency_paused);
    h.city_established = Some(city_established);
    h.city_cells = Some(city_cells);
    h.cell_width = Some(cell_width);
    h.post = Some(post);
    h.subscribe = Some(subscribe);
    h.register_tick = Some(register_tick);
    h.record_history = Some(record_history);
    h.add_ref = Some(add_ref);
    h.release = Some(release);
    h.set_fire_dispatch_effectiveness = Some(set_fire_dispatch_effectiveness);
    h.occupant_type = Some(occupant_type);
    h.occupant_in_manager = Some(occupant_in_manager);
    h.occupant_cell = Some(occupant_cell);
    h.occupant_cell_rect = Some(occupant_cell_rect);
    h.occupant_prop_f32 = Some(occupant_prop_f32);
    h.occupant_prop_bool = Some(occupant_prop_bool);
    h.occupants_in_rect = Some(occupants_in_rect);
    h.occupant_lot = Some(occupant_lot);
    h.create_fire_occupant = Some(create_fire_occupant);
    h.remove_fire_occupant = Some(remove_fire_occupant);
    h.demolish_lot = Some(demolish_lot);
    h.play_effect_at = Some(play_effect_at);
    h.is_road_cell = Some(is_road_cell);
    h.cell_corner_position = Some(cell_corner_position);
    h.create_riot_leader = Some(create_riot_leader);
    h.automaton_cell = Some(automaton_cell);
    h.network_occupant_at = Some(network_occupant_at);
    h.unset_removal_flags = Some(unset_removal_flags);
    h.set_removal_flags = Some(set_removal_flags);
    h.create_toxic_occupant = Some(create_toxic_occupant);
    h.remove_toxic_occupant = Some(remove_toxic_occupant);
    h.find_watered_pipe = Some(find_watered_pipe);
    h.pipe_cell = Some(pipe_cell);
    h.view_level = Some(view_level);
    h.create_leak_effect = Some(create_leak_effect);
    h.connected_pipe_cells = Some(connected_pipe_cells);
    h.demolish_pipe = Some(demolish_pipe);
    h
}

struct Fx {
    g: Box<Game>,
    d: *mut fc4_disasters,
}

impl Drop for Fx {
    fn drop(&mut self) {
        unsafe { fc4_disasters_destroy(self.d) };
    }
}

fn fx() -> Fx {
    let g = Game::new();
    let h = host(&g);
    let d = unsafe { fc4_disasters_create(&h, 0x1234_5678) };
    assert!(!d.is_null());
    Fx { g, d }
}

fn state(d: *const fc4_disasters) -> fc4_disasters_state {
    let mut s = fc4_disasters_state::default();
    assert!(unsafe { fc4_disasters_get_state(d, &mut s) });
    s
}

#[test]
fn create_checks_the_table_and_null_handles_fall_back() {
    let g = Game::new();
    unsafe {
        assert!(fc4_disasters_create(std::ptr::null(), 1).is_null());
        let mut h = host(&g);
        h.struct_size -= 1;
        assert!(fc4_disasters_create(&h, 1).is_null());
        fc4_disasters_destroy(std::ptr::null_mut());
        let null = std::ptr::null_mut();
        assert!(!fc4_disasters_init(null));
        assert!(!fc4_disasters_enabled(null));
        assert_eq!(fc4_disasters_attempt_start(null, FIRE, 1, 1), 0);
        assert_eq!(fc4_disasters_riot_leaders(null, 1, std::ptr::null_mut(), 0), 0);
        let mut s = fc4_disasters_state::default();
        assert!(!fc4_disasters_get_state(null, &mut s));
    }
    // An empty table works: every callback reads as 0 / false / none.
    let h = empty_host(&g);
    unsafe {
        let d = fc4_disasters_create(&h, 1);
        assert!(fc4_disasters_init(d));
        assert!(fc4_disasters_has_manager(d, FIRE));
        assert!(!fc4_disasters_ignite(d, 0x10, true, 0, false));
        assert!(fc4_disasters_shutdown(d));
        fc4_disasters_destroy(d);
    }
}

#[test]
fn layer_lifecycle_enable_counter_and_messages() {
    let f = fx();
    unsafe {
        assert!(fc4_disasters_init(f.d));
        let s = state(f.d);
        assert_eq!(s.struct_size as usize, std::mem::size_of::<fc4_disasters_state>());
        assert_eq!((s.initialised, s.enabled, s.ongoing), (1, 1, 0));
        for t in [FIRE, RIOT, TOXIC, PIPE, EFFECT] {
            assert!(fc4_disasters_has_manager(f.d, t));
        }
        // §18 test 6.
        assert!(!fc4_disasters_enable(f.d, false));
        assert!(!fc4_disasters_enable(f.d, false));
        assert!(!fc4_disasters_enable(f.d, true));
        assert!(fc4_disasters_enable(f.d, true));
        assert_eq!(state(f.d).disable_counter, 0);
        // Unknown target kind or NULL message: false.
        let m = fc4_disaster_message {
            msg_type: 0x0BFB_41BA,
            data1: 5,
            data2: 6,
            data3: 40,
            ..Default::default()
        };
        assert!(!fc4_disasters_do_message(f.d, 7, 0, &m));
        assert!(!fc4_disasters_do_message(
            f.d,
            FC4_DISASTER_TARGET_LAYER,
            0,
            std::ptr::null()
        ));
        // The toxic cheat through the manager target.
        assert!(fc4_disasters_do_message(f.d, FC4_DISASTER_TARGET_MANAGER, TOXIC, &m));
        let s = state(f.d);
        assert_eq!((s.toxic_spills, s.ongoing), (1, 1));
        assert_ne!(s.toxic_instance, 0);
        assert!(fc4_disasters_is_ongoing(f.d));
        assert!(fc4_disasters_shutdown(f.d));
        assert!(f.g.take().iter().any(|l| l.starts_with("remove_toxic")));
        assert_eq!(state(f.d).initialised, 0);
    }
}

#[test]
fn fire_burns_down_and_ends_its_episode() {
    let f = fx();
    let g = &f.g;
    g.building(0x10, 5, 5);
    unsafe {
        assert!(fc4_disasters_init(f.d));
        g.take();
        assert!(fc4_disasters_ignite(f.d, 0x10, false, 0, false));
        let fire = fc4_disasters_fire_instance(f.d);
        assert_ne!(fire, 0);
        let log = g.take();
        assert!(log.contains(&format!("post CA1B1E64 1 5 5 621CDA33 {fire}")), "{log:?}");
        assert_eq!(g.emergency.get(), 1);
        assert_eq!(g.saves_off.get(), 1);
        let mut e = fc4_disaster_fire::default();
        assert!(fc4_disasters_get_fire(f.d, 0, &mut e));
        assert_eq!((e.target, e.counter, e.max_stage, e.extinguishing), (0x10, 0, 2, 0));
        assert!(!fc4_disasters_get_fire(f.d, 1, &mut e));
        // No exemplar: the default D3 (cumulative 10, 20, ...) burns it down
        // at stage 2, the 20th update.
        fc4_disasters_fire_update(f.d);
        let mut n = 1;
        while fc4_disasters_fire_count(f.d) > 0 && n < 100 {
            g.now.set(g.now.get() + 1000);
            fc4_disasters_fire_update(f.d);
            n += 1;
        }
        assert_eq!(n, 20);
        let log = g.take();
        assert!(log.contains(&"demolish 10 100010 300".to_string()), "{log:?}");
        assert!(log.iter().any(|l| l.starts_with("post 49E3DB1D")));
        // The next update ends the episode.
        fc4_disasters_fire_update(f.d);
        let log = g.take();
        assert!(log.contains(&"resume".to_string()));
        assert!(log.contains(&"history 49D15B74 1".to_string()));
        assert_eq!(g.emergency.get(), 0);
        assert_eq!(g.saves_off.get(), 0);
        assert_eq!(state(f.d).fire_phase, 3);
    }
}

#[test]
fn tuning_reaches_the_fire() {
    let f = fx();
    let g = &f.g;
    g.building(0x10, 5, 5);
    unsafe {
        let p = fc4_props_create();
        let d3 = [1u32, 1, 1, 1, 1];
        assert!(fc4_props_set_u32s(p, D3, d3.as_ptr(), d3.len()));
        assert!(fc4_disasters_set_tuning(f.d, FLAME, p));
        fc4_props_destroy(p);
        assert!(fc4_disasters_init(f.d));
        assert!(fc4_disasters_ignite(f.d, 0x10, false, 0, false));
        // D3cum [1, 2, 3, 4, 5]: maxStage 2 is reached at counter 2.
        fc4_disasters_fire_update(f.d);
        assert_eq!(fc4_disasters_fire_count(f.d), 1);
        g.now.set(1000);
        fc4_disasters_fire_update(f.d);
        assert_eq!(fc4_disasters_fire_count(f.d), 0);
        assert!(!fc4_disasters_set_tuning(std::ptr::null_mut(), FLAME, std::ptr::null()));
        assert!(fc4_disasters_set_tuning(f.d, FLAME, std::ptr::null()));
    }
}

#[test]
fn hosed_fire_and_dispatch_effectiveness() {
    let f = fx();
    let g = &f.g;
    g.building(0x10, 5, 5);
    unsafe {
        assert!(fc4_disasters_init(f.d));
        assert!(fc4_disasters_ignite(f.d, 0x10, false, 0, false));
        let mut e = fc4_disaster_fire::default();
        assert!(fc4_disasters_get_fire(f.d, 0, &mut e));
        assert!(fc4_disasters_fire_set_hosed(f.d, e.occupant, 1));
        assert!(!fc4_disasters_fire_set_hosed(f.d, 0x9999, 1));
        let msg = |t: u32, ty: i64, key: i64| fc4_disaster_message {
            msg_type: t,
            data1: ty,
            data2: key,
            ..Default::default()
        };
        let begin = msg(0xE946_71EB, 1, 1);
        let active = msg(0xE946_71EC, 1, 1);
        let end = msg(0x8946_71F6, 1, 1);
        assert!(fc4_disasters_do_message(f.d, FC4_DISASTER_TARGET_LAYER, 0, &begin));
        assert!(fc4_disasters_do_message(f.d, FC4_DISASTER_TARGET_LAYER, 0, &active));
        g.now.set(20_000);
        assert!(fc4_disasters_do_message(f.d, FC4_DISASTER_TARGET_LAYER, 0, &end));
        assert_eq!(state(f.d).fire_timers, 1);
        fc4_disasters_extinguish(f.d, 0x10);
        assert!(fc4_disasters_get_fire(f.d, 0, &mut e));
        assert_eq!(e.extinguishing, 1);
        g.now.set(40_000);
        fc4_disasters_fire_update(f.d);
        assert_eq!(fc4_disasters_fire_count(f.d), 0);
        g.take();
        fc4_disasters_fire_update(f.d);
        // One timer of 20 s over 40 s: 0.5.
        assert!(g.take().contains(&"fire_effectiveness 0.5".to_string()));
        assert_eq!(state(f.d).fire_effectiveness, 0.5);
    }
}

#[test]
fn riot_leaders_and_toxic_spill() {
    let f = fx();
    let g = &f.g;
    for x in 0..64 {
        g.roads.borrow_mut().insert((x, 20));
    }
    unsafe {
        assert!(fc4_disasters_init(f.d));
        let id = fc4_disasters_attempt_start(f.d, RIOT, 10, 20);
        assert_ne!(id, 0);
        assert_eq!(fc4_disasters_active_count(f.d, RIOT), 1);
        assert_eq!(fc4_disasters_riot_leaders(f.d, id, std::ptr::null_mut(), 0), 1);
        let mut l = [fc4_riot_leader::default(); 4];
        assert_eq!(fc4_disasters_riot_leaders(f.d, id, l.as_mut_ptr(), l.len()), 1);
        assert_eq!((l[0].x, l[0].z, l[0].counter), (10, 20, 0));
        // The removal lock (network-manager.md section 10): the roads within 3 cells.
        let locked: Vec<String> = g
            .take()
            .into_iter()
            .filter(|l| l.starts_with("unset_removal"))
            .collect();
        assert_eq!(
            locked,
            (7..=13)
                .map(|x| format!("unset_removal {:X} 2", 0x7000 + x))
                .collect::<Vec<_>>()
        );
        assert_eq!(state(f.d).riot_locked, 7);
        let mut i = fc4_disaster_instance::default();
        assert!(fc4_disasters_get_instance(f.d, id, &mut i));
        assert_eq!((i.manager_type, i.phase, i.tick_registered), (RIOT, 1, 1));
        for _ in 0..30 {
            g.now.set(g.now.get() + 1000);
            fc4_disasters_tick(f.d, id, 16_000);
        }
        // No exemplar: no leaders spawned by default.
        assert_eq!(fc4_disasters_riot_leaders(f.d, id, std::ptr::null_mut(), 0), 1);
        assert_eq!(fc4_disasters_riot_leaders(f.d, id, l.as_mut_ptr(), l.len()), 1);
        assert_eq!(l[0].stage, 1);
        // Shutdown ends the leaders and the tick; the instance stays in its
        // manager's list until the manager removes it.
        fc4_disasters_instance_shutdown(f.d, id);
        assert!(fc4_disasters_get_instance(f.d, id, &mut i));
        assert_eq!(i.tick_registered, 0);
        assert_eq!(fc4_disasters_riot_leaders(f.d, id, std::ptr::null_mut(), 0), 0);

        // §18 test 9 with one hose: 30 updates.
        let t = fc4_disasters_spill_toxic_waste(f.d, 3, 4, 120);
        assert_ne!(t, 0);
        let log = g.take();
        let line = log.iter().find(|l| l.starts_with("toxic ")).expect("toxic occupant");
        let occ = u64::from_str_radix(line.split(' ').nth(2).unwrap(), 16).unwrap();
        assert!(fc4_disasters_toxic_set_hosed(f.d, occ, 1));
        for _ in 0..30 {
            g.now.set(g.now.get() + 1000);
            fc4_disasters_tick(f.d, t, 16_000);
        }
        assert_eq!(state(f.d).toxic_instance, 0);
        assert!(g.take().contains(&format!("remove_toxic {occ:X}")));
    }
}

#[test]
fn locations_and_effects() {
    let f = fx();
    let g = &f.g;
    g.building(0x20, 7, 8);
    unsafe {
        assert!(fc4_disasters_init(f.d));
        fc4_disasters_register_location(f.d, 9, 1, 2, 0, 3, false);
        fc4_disasters_register_location(f.d, 9, 0, 0, 0x20, 3, false);
        assert_eq!(g.refs.borrow().get(&0x20).copied(), Some(1));
        let (mut x, mut z) = (0, 0);
        assert!(fc4_disasters_most_recent_location(f.d, 9, &mut x, &mut z));
        assert_eq!((x, z), (7, 8));
        assert!(!fc4_disasters_most_recent_location(f.d, 5, &mut x, &mut z));
        assert!(fc4_disasters_most_recent_location(
            f.d,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut()
        ));
        assert_eq!(fc4_disasters_location_count(f.d), 2);
        fc4_disasters_unregister_location_by_occupant(f.d, 9, 0x20);
        assert_eq!(g.refs.borrow().get(&0x20).copied(), Some(0));
        fc4_disasters_unregister_locations(f.d, 9);
        assert_eq!(fc4_disasters_location_count(f.d), 0);

        let id = fc4_disasters_start_effect(f.d, 0xE1, 160.0, 320.0);
        assert_ne!(id, 0);
        let mut i = fc4_disaster_instance::default();
        assert!(fc4_disasters_get_instance(f.d, id, &mut i));
        assert_eq!((i.manager_type, i.has_location, i.x, i.z), (EFFECT, 1, 10, 20));
        let done = fc4_disaster_message {
            msg_type: 0x2A7B_582D,
            void1: 0xE1,
            ..Default::default()
        };
        assert!(fc4_disasters_do_message(f.d, FC4_DISASTER_TARGET_INSTANCE, id, &done));
        assert!(!fc4_disasters_get_instance(f.d, id, &mut i));
        assert!(!fc4_disasters_is_ongoing(f.d));
    }
}

#[test]
fn pipe_burst_reads_many_connected_cells() {
    let f = fx();
    let g = &f.g;
    g.pipes.borrow_mut().insert((3, 3), 0x500);
    // More neighbours than the first buffer holds: the core asks again.
    *g.neighbours.borrow_mut() = (0..40).map(|i| (100 + i, 3)).collect();
    unsafe {
        assert!(fc4_disasters_init(f.d));
        assert_eq!(fc4_disasters_attempt_start(f.d, PIPE, 9, 9), 0);
        let id = fc4_disasters_attempt_start(f.d, PIPE, 3, 3);
        assert_ne!(id, 0);
        assert!(g.take().contains(&"post EA1F0DCD 3 3 0 0 0".to_string()));
        fc4_disasters_tick(f.d, id, 60_000_000);
        assert!(g.take().contains(&"demolish_pipe 500 302".to_string()));
        assert_eq!(g.neighbour_calls.get(), 2);
        for _ in 0..4 {
            fc4_disasters_tick(f.d, id, 1);
        }
        let mut i = fc4_disaster_instance::default();
        assert!(
            !fc4_disasters_get_instance(f.d, id, &mut i),
            "no watered pipe among the neighbours"
        );
        assert!(!fc4_disasters_is_ongoing(f.d));
    }
}

#[test]
fn quake_runs_through_the_abi() {
    let f = fx();
    let g = &f.g;
    unsafe {
        assert!(fc4_disasters_init(f.d));
        let id = fc4_disasters_attempt_start(f.d, 0x4296_3812, 500, 9000);
        assert_ne!(id, 0);
        assert!(g.take().contains(&"effect_at QuakeStart".to_string()));
        let mut i = fc4_disaster_instance::default();
        let mut n = 0;
        while fc4_disasters_get_instance(f.d, id, &mut i) && n < 1000 {
            fc4_disasters_tick(f.d, id, 100_000);
            n += 1;
        }
        assert!(n < 1000);
        assert_eq!(g.emergency.get(), 0);
    }
}

#[test]
fn layouts() {
    assert_eq!(std::mem::size_of::<fc4_disaster_post>(), 32);
    assert_eq!(std::mem::size_of::<fc4_disaster_message>(), 48);
    assert_eq!(std::mem::size_of::<fc4_disasters_state>(), 80);
    assert_eq!(std::mem::size_of::<fc4_disaster_fire>(), 56);
    assert_eq!(std::mem::size_of::<fc4_disaster_instance>(), 24);
    assert_eq!(std::mem::size_of::<fc4_riot_leader>(), 32);
}
