// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The city situation manager through the C ABI
//! (docs/specs/city-situation.md): a fake game behind `fc4_citysit_host`,
//! including §12 item 8 (slots 13 to 15 return `float`).

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::c_void;

use crate::budget::fc4_token_callback;
use crate::city_situation::*;
use crate::stream::fc4_bytes_sink;
use crate::test_streams::{istream, ostream, In, Out};

const ADV: u64 = 0x500;
const CAR: u64 = 0x700;
const G: u32 = 0x8C15_1EFD;
const MANAGERS: [u64; 4] = [43, 45, 44, 46];
const DRIVABLE: u32 = 2;
const SUCCESS: u32 = 1;
const FAILURE: u32 = 2;
const RUNNING: u32 = 3;
const IDLE: u32 = 4;
const ANIMATION_TICK: u32 = 0x86AD_10EE;
const ADVICE_ADDED: u32 = 0x8A09_FA64;
const SHOW_POPUP: u32 = 0xEBB6_7EF7;
const CHEAT: u32 = 0x230E_27AC;
const CHEAT_ID: u32 = 0x8C41_36DB;

#[derive(Default)]
struct Game {
    log: RefCell<Vec<String>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    debug: Cell<bool>,
    advices: RefCell<BTreeMap<u64, fc4_citysit_advice_info>>,
    /// Automata: flags and position.
    autos: RefCell<BTreeMap<u64, (u32, f32, f32)>>,
    candidates: RefCell<BTreeMap<u64, Vec<u64>>>,
    rewards: RefCell<Vec<u32>>,
    status_answer: Cell<u32>,
    button: Cell<u32>,
    time_limit: Cell<f32>,
    next_handle: Cell<u32>,
}

impl Game {
    fn new() -> Box<Game> {
        let g = Box::new(Game::default());
        *g.rewards.borrow_mut() = vec![0; 64];
        g.status_answer.set(RUNNING);
        g.next_handle.set(1);
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
    fn refs(&self, t: u64) -> i64 {
        self.refs.borrow().get(&t).copied().unwrap_or(0)
    }
    fn advice(&self, t: u64, mut info: fc4_citysit_advice_info) {
        if info.mission_type == 0 {
            info.mission_type = 2;
        }
        self.advices.borrow_mut().insert(t, info);
    }
    fn auto(&self, t: u64, flags: u32, x: f32, z: f32) {
        self.autos.borrow_mut().insert(t, (flags, x, z));
    }
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

unsafe fn emit(tokens: &[u64], cb: fc4_token_callback, ctx: *mut c_void) {
    for &t in tokens {
        cb.unwrap()(ctx, t);
    }
}

unsafe extern "C" fn debug_enabled(u: *mut c_void) -> bool {
    game(u).debug.get()
}
unsafe extern "C" fn register_cheat(u: *mut c_void) -> bool {
    game(u).log("register_cheat");
    true
}
unsafe extern "C" fn unregister_cheat(u: *mut c_void) {
    game(u).log("unregister_cheat");
}
unsafe extern "C" fn subscribe(u: *mut c_void, m: u32) {
    game(u).log(format!("subscribe {m:#x}"));
}
unsafe extern "C" fn unsubscribe(u: *mut c_void, m: u32) {
    game(u).log(format!("unsubscribe {m:#x}"));
}
unsafe extern "C" fn acquire_view(u: *mut c_void) -> bool {
    game(u).log("acquire_view");
    true
}
unsafe extern "C" fn release_view(u: *mut c_void) {
    game(u).log("release_view");
}
unsafe extern "C" fn register_tick(u: *mut c_void) {
    game(u).log("register_tick");
}
unsafe extern "C" fn unregister_tick(u: *mut c_void) {
    game(u).log("unregister_tick");
}
unsafe extern "C" fn acquire_automata_managers(u: *mut c_void, cb: fc4_token_callback, ctx: *mut c_void) {
    for m in MANAGERS {
        *game(u).refs.borrow_mut().entry(m).or_insert(0) += 1;
    }
    emit(&MANAGERS, cb, ctx);
}
unsafe extern "C" fn yes(_: *mut c_void) -> bool {
    true
}
unsafe extern "C" fn remove_lua(u: *mut c_void) {
    game(u).log("remove_lua");
}
unsafe extern "C" fn show_all(u: *mut c_void, kind: u32, visible: bool) {
    game(u).log(format!("show_all {kind} {visible}"));
}
unsafe extern "C" fn preferences_rewards(u: *mut c_void, out: *mut u32, cap: usize) -> usize {
    let r = game(u).rewards.borrow();
    if !out.is_null() {
        for (i, &g) in r.iter().take(cap).enumerate() {
            *out.add(i) = g;
        }
    }
    r.len()
}
unsafe extern "C" fn set_preferences_reward(u: *mut c_void, i: u32, g: u32) {
    game(u).rewards.borrow_mut()[i as usize] = g;
}
unsafe extern "C" fn save_preferences(u: *mut c_void) {
    game(u).log("save_prefs");
}
unsafe extern "C" fn add_ref(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_insert(0) += 1;
}
unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_insert(0) -= 1;
}
unsafe extern "C" fn is_situation_advice(u: *mut c_void, o: u64) -> bool {
    game(u).advices.borrow().contains_key(&o)
}
unsafe extern "C" fn advisor_advices(u: *mut c_void, cb: fc4_token_callback, ctx: *mut c_void) {
    let v: Vec<u64> = game(u).advices.borrow().keys().copied().collect();
    emit(&v, cb, ctx);
}
unsafe extern "C" fn find_advice(u: *mut c_void, guid: u32) -> u64 {
    game(u)
        .advices
        .borrow()
        .iter()
        .find(|(_, i)| i.guid == guid)
        .map_or(0, |(&t, _)| t)
}
unsafe extern "C" fn advice_info(u: *mut c_void, a: u64, out: *mut fc4_citysit_advice_info) {
    if let Some(i) = game(u).advices.borrow().get(&a) {
        *out = *i;
    }
}
unsafe extern "C" fn retire_advice(u: *mut c_void, a: u64) {
    game(u).log(format!("retire {a:#x}"));
}
unsafe extern "C" fn run_script(u: *mut c_void, a: u64, s: u32) {
    game(u).log(format!("script {a:#x} {s}"));
}
unsafe extern "C" fn time_limit(u: *mut c_void, _a: u64) -> f32 {
    game(u).time_limit.get()
}
unsafe extern "C" fn bind_mission_script(u: *mut c_void, a: u64, v: u64) {
    game(u).log(format!("bind {a:#x} {v:#x}"));
}
unsafe extern "C" fn target_name(_: *mut c_void, _a: u64, t: u64, sink: fc4_bytes_sink, ctx: *mut c_void) {
    let s = format!("target {t:#x}");
    sink.unwrap()(ctx, s.as_ptr(), s.len());
}
unsafe extern "C" fn advice_accepts_automaton(u: *mut c_void, a: u64, v: u64) -> bool {
    game(u).log(format!("accepts? {a:#x} {v:#x}"));
    false
}
unsafe extern "C" fn is_automaton(u: *mut c_void, o: u64) -> bool {
    game(u).autos.borrow().contains_key(&o)
}
unsafe extern "C" fn candidate_vehicles(u: *mut c_void, a: u64, cb: fc4_token_callback, ctx: *mut c_void) {
    let v = game(u).candidates.borrow().get(&a).cloned().unwrap_or_default();
    emit(&v, cb, ctx);
}
unsafe extern "C" fn automaton_flags(u: *mut c_void, a: u64) -> u32 {
    game(u).autos.borrow().get(&a).map_or(0, |x| x.0)
}
unsafe extern "C" fn automaton_type(_: *mut c_void, _a: u64) -> u32 {
    1
}
unsafe extern "C" fn track_indicator(u: *mut c_void, a: u64) -> u32 {
    let g = game(u);
    let h = g.next_handle.get();
    g.next_handle.set(h + 1);
    g.log(format!("track {a:#x} -> {h}"));
    h
}
unsafe extern "C" fn untrack_indicator(u: *mut c_void, h: u32) {
    game(u).log(format!("untrack {h}"));
}
unsafe extern "C" fn add_listener(u: *mut c_void, a: u64) {
    game(u).log(format!("listen {a:#x}"));
}
unsafe extern "C" fn remove_listener(u: *mut c_void, a: u64) {
    game(u).log(format!("unlisten {a:#x}"));
}
unsafe extern "C" fn freeze_automata(u: *mut c_void, m: u64, on: bool) {
    game(u).log(format!("freeze {m} {on}"));
}
unsafe extern "C" fn play_sound(u: *mut c_void, a: u64, s: u32) {
    game(u).log(format!("sound {a:#x} {s:#x}"));
}
unsafe extern "C" fn position(u: *mut c_void, o: u64, x: *mut f32, z: *mut f32) {
    if let Some(&(_, px, pz)) = game(u).autos.borrow().get(&o) {
        *x = px;
        *z = pz;
    }
}
unsafe extern "C" fn check_for_success(u: *mut c_void, _a: u64, _t: f32) -> u32 {
    game(u).status_answer.get()
}
unsafe extern "C" fn countdown_start(u: *mut c_void) {
    game(u).log("countdown_start");
}
unsafe extern "C" fn countdown_end(u: *mut c_void) {
    game(u).log("countdown_end");
}
unsafe extern "C" fn outcome_effect(u: *mut c_void, a: u64, success: bool) {
    game(u).log(format!("effect {a:#x} {success}"));
}
unsafe extern "C" fn player_drive_mode(u: *mut c_void, v: u64, on: bool) {
    game(u).log(format!("drive {v:#x} {on}"));
}
unsafe extern "C" fn start_joyride(u: *mut c_void, v: u64) {
    game(u).log(format!("joyride {v:#x}"));
}
unsafe extern "C" fn add_transient_effect(u: *mut c_void, x: f32, z: f32, r: f32, m: f32) {
    game(u).log(format!("aura {x} {z} {r} {m}"));
}
unsafe extern "C" fn deposit_funds(u: *mut c_void, amount: i64) {
    game(u).log(format!("deposit {amount}"));
}
unsafe extern "C" fn send_activation(u: *mut c_void, a: u64, on: bool) {
    game(u).log(format!("activation {a:#x} {on}"));
}
unsafe extern "C" fn post_show_popup(u: *mut c_void, a: u64, v: u64) {
    game(u).log(format!("post_popup {a:#x} {v:#x}"));
}
unsafe extern "C" fn post_popup_closed(u: *mut c_void, a: u64) {
    game(u).log(format!("post_closed {a:#x}"));
}
unsafe extern "C" fn post_initiate(u: *mut c_void, g: u32, b: bool) {
    game(u).log(format!("post_initiate {g:#x} {b}"));
}
unsafe extern "C" fn hide_panel(u: *mut c_void) {
    game(u).log("hide_panel");
}
unsafe extern "C" fn end_mysim(u: *mut c_void) {
    game(u).log("end_mysim");
}
unsafe extern "C" fn show_popup(u: *mut c_void, p: u32, a: u64, v: u64) -> u32 {
    game(u).log(format!("popup {p} {a:#x} {v:#x}"));
    game(u).button.get()
}

fn host(g: &Game) -> fc4_citysit_host {
    fc4_citysit_host {
        struct_size: std::mem::size_of::<fc4_citysit_host>() as u32,
        user: g.user(),
        debug_enabled: Some(debug_enabled),
        register_cheat: Some(register_cheat),
        unregister_cheat: Some(unregister_cheat),
        subscribe: Some(subscribe),
        unsubscribe: Some(unsubscribe),
        acquire_indicator_view: Some(acquire_view),
        release_indicator_view: Some(release_view),
        register_animation_tick: Some(register_tick),
        unregister_animation_tick: Some(unregister_tick),
        acquire_automata_managers: Some(acquire_automata_managers),
        acquire_lot_manager: Some(yes),
        release_lot_manager: None,
        acquire_mysim: Some(yes),
        release_mysim: None,
        acquire_lua: Some(yes),
        release_lua: None,
        create_lua_table: None,
        remove_lua_table: Some(remove_lua),
        release_mission_resources: None,
        show_all_indicators: Some(show_all),
        preferences_rewards: Some(preferences_rewards),
        set_preferences_reward: Some(set_preferences_reward),
        save_preferences: Some(save_preferences),
        add_ref: Some(add_ref),
        release: Some(release),
        is_situation_advice: Some(is_situation_advice),
        advisor_advices: Some(advisor_advices),
        find_advice: Some(find_advice),
        advice_info: Some(advice_info),
        retire_advice: Some(retire_advice),
        run_script: Some(run_script),
        time_limit: Some(time_limit),
        bind_mission_script: Some(bind_mission_script),
        build_coverage_cells: None,
        first_target: None,
        target_count: None,
        target_name: Some(target_name),
        advice_accepts_automaton: Some(advice_accepts_automaton),
        is_automaton: Some(is_automaton),
        occupant_has_property: None,
        candidate_vehicles: Some(candidate_vehicles),
        automaton_flags: Some(automaton_flags),
        automaton_type: Some(automaton_type),
        track_indicator: Some(track_indicator),
        untrack_indicator: Some(untrack_indicator),
        add_listener: Some(add_listener),
        remove_listener: Some(remove_listener),
        freeze_automata: Some(freeze_automata),
        play_sound: Some(play_sound),
        position: Some(position),
        check_for_success: Some(check_for_success),
        run_pursuit: None,
        start_countdown_effect: Some(countdown_start),
        end_countdown_effect: Some(countdown_end),
        outcome_effect: Some(outcome_effect),
        player_drive_mode: Some(player_drive_mode),
        start_joyride: Some(start_joyride),
        add_transient_effect: Some(add_transient_effect),
        deposit_funds: Some(deposit_funds),
        send_activation: Some(send_activation),
        post_show_popup: Some(post_show_popup),
        post_popup_closed: Some(post_popup_closed),
        post_initiate: Some(post_initiate),
        hide_mission_status_panel: Some(hide_panel),
        end_mysim_dialog: Some(end_mysim),
        show_popup: Some(show_popup),
    }
}

fn create(g: &Game) -> *mut fc4_citysit {
    let m = unsafe { fc4_citysit_create(&host(g), 7, 0x99) };
    assert!(!m.is_null());
    unsafe { fc4_citysit_add_ref(m) };
    m
}

fn state(m: *const fc4_citysit) -> fc4_citysit_state {
    let mut s = fc4_citysit_state::default();
    assert!(unsafe { fc4_citysit_get_state(m, &mut s) });
    s
}

fn tick(m: *mut fc4_citysit, t: f32) {
    unsafe { fc4_citysit_tick(m, t) };
}

fn info(guid: u32) -> fc4_citysit_advice_info {
    fc4_citysit_advice_info {
        guid,
        ..Default::default()
    }
}

/// A mission on ADV with CAR, past its countdown.
fn running(g: &Game, m: *mut fc4_citysit, i: fc4_citysit_advice_info) {
    g.advice(ADV, i);
    g.auto(CAR, DRIVABLE, 100.0, 200.0);
    unsafe {
        assert!(fc4_citysit_init(m));
        assert!(fc4_citysit_initiate_situation(m, ADV, CAR, false));
        fc4_citysit_activate_situation(m);
    }
    tick(m, 16.0);
    tick(m, 4000.0);
    assert_eq!(unsafe { fc4_citysit_current_status(m) }, RUNNING);
    g.take();
}

#[test]
fn create_rejects_bad_tables_and_identity() {
    let g = Game::new();
    unsafe {
        assert!(fc4_citysit_create(std::ptr::null(), 0, 0).is_null());
        let mut h = host(&g);
        h.struct_size -= 1;
        assert!(fc4_citysit_create(&h, 0, 0).is_null());
        let mut off = 99u32;
        for (iid, want) in [
            (0x6BB1_437A, 0),
            (1, 0xC),
            (0xE4FD_A3D4, 0xC),
            (0x4522_94AA, 4),
            (0xE98B_2F57, 8),
        ] {
            assert!(fc4_citysit_query_interface(iid, &mut off), "{iid:08X}");
            assert_eq!(off, want);
        }
        assert!(!fc4_citysit_query_interface(0x1234, &mut off));
        assert_eq!(fc4_citysit_time_limit_seconds(std::ptr::null()), 0.0);
        fc4_citysit_destroy(std::ptr::null_mut());
    }
}

#[test]
fn lifecycle() {
    let g = Game::new();
    g.debug.set(true);
    let m = create(&g);
    let s = state(m);
    assert_eq!(s.struct_size as usize, std::mem::size_of::<fc4_citysit_state>());
    assert_eq!((s.status, s.indicators_visible), (IDLE, 1));
    assert!(unsafe { fc4_citysit_init(m) });
    let log = g.take();
    assert_eq!(log[0], "register_cheat");
    assert!(log.contains(&"show_all 4 true".to_string()));
    let s = state(m);
    assert_eq!((s.initialized, s.cheat_registered, s.automata_managers), (1, 1, 4));
    unsafe {
        assert_eq!(fc4_citysit_add_ref(m), 2);
        assert_eq!(fc4_citysit_release(m), 1);
        assert_eq!(fc4_citysit_release(m), 0);
    }
    let s = state(m);
    assert_eq!((s.destroyed, s.initialized), (1, 0));
    let log = g.take();
    for e in ["unregister_cheat", "release_view", "remove_lua"] {
        assert!(log.contains(&e.to_string()), "{e}");
    }
    assert_eq!(g.refs(43), 0);
    unsafe { fc4_citysit_destroy(m) };
}

#[test]
fn cs12_01_record_round_trip() {
    let payloads: [&[u8]; 3] = [
        &[0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01],
        &[0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        &[
            0x02, 0x00, 0x01, 0x00, 0x00, 0x00, 0xFD, 0x1E, 0x15, 0x8C, 0x01, 0x00, 0x01,
        ],
    ];
    let g = Game::new();
    for p in payloads {
        let m = create(&g);
        let mut i = In::new(p);
        let is = istream(&mut i);
        assert!(unsafe { fc4_citysit_read(m, &is) });
        assert_eq!(i.pos, p.len());
        assert_eq!(state(m).loaded_from_save, 1);
        let mut o = Out::default();
        let os = ostream(&mut o);
        assert!(unsafe { fc4_citysit_write(m, &os) });
        assert_eq!(o.bytes, p);
        unsafe { fc4_citysit_destroy(m) };
    }
    let m = create(&g);
    let mut i = In::new(payloads[2]);
    let is = istream(&mut i);
    assert!(unsafe { fc4_citysit_read(m, &is) });
    assert_eq!(unsafe { fc4_citysit_get_success_count(m, G) }, 1);
    let mut c = [fc4_citysit_success_count::default(); 2];
    assert_eq!(unsafe { fc4_citysit_success_counts(m, c.as_mut_ptr(), 2) }, 1);
    assert_eq!((c[0].guid, c[0].count), (G, 1));
    // Version 3 is rejected.
    let bad = [0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    let m3 = create(&g);
    let mut i = In::new(&bad);
    let is = istream(&mut i);
    assert!(!unsafe { fc4_citysit_read(m3, &is) });
    assert_eq!(state(m3).loaded_from_save, 0);
    unsafe {
        assert!(!fc4_citysit_read(m3, std::ptr::null()));
        fc4_citysit_destroy(m);
        fc4_citysit_destroy(m3);
    }
}

#[test]
fn cs12_05_success_payout() {
    let g = Game::new();
    let m = create(&g);
    running(
        &g,
        m,
        fc4_citysit_advice_info {
            guid: G,
            success_money: 10_000,
            success_radius: 256.0,
            success_magnitude: 50.0,
            ..Default::default()
        },
    );
    unsafe {
        let (mut r, mut mag, mut money) = (0.0f32, 0.0f32, 0i64);
        assert!(fc4_citysit_situation_aura(m, &mut r, &mut mag));
        assert_eq!((r, mag), (256.0, 50.0));
        assert!(fc4_citysit_situation_money(m, &mut money));
        assert_eq!(money, 10_000);
    }
    g.status_answer.set(SUCCESS);
    tick(m, 16.0);
    assert_eq!(unsafe { fc4_citysit_current_status(m) }, SUCCESS);
    assert_eq!(state(m).end_delay, 1.0);
    g.take();
    for _ in 0..4 {
        // The tick as the message: milliseconds as float bits in data3.
        let t = 250.0f32.to_bits();
        assert!(unsafe { fc4_citysit_do_message(m, ANIMATION_TICK, 0, 0, t, 0, 0, std::ptr::null(), 0) });
    }
    assert_eq!(
        g.take(),
        [
            "effect 0x500 true",
            "aura 100 200 256 50",
            "deposit 10000",
            "script 0x500 1",
            "retire 0x500",
            "post_popup 0x500 0x700"
        ]
    );
    assert_eq!(unsafe { fc4_citysit_get_success_count(m, G) }, 1);
    unsafe { fc4_citysit_destroy(m) };
}

/// §12 item 8: slots 13 to 15 come back as `float` (seconds).
#[test]
fn cs12_08_float_getters_and_the_mission_clock() {
    let g = Game::new();
    let m = create(&g);
    g.advice(ADV, info(G));
    g.time_limit.set(60.0);
    unsafe {
        assert!(fc4_citysit_init(m));
        g.take();
        assert!(fc4_citysit_initiate_situation(m, ADV, CAR, true));
        assert!(!fc4_citysit_initiate_situation(m, ADV, CAR, true));
        assert_eq!(g.refs(ADV), 1);
        assert_eq!(fc4_citysit_active_situation(m), ADV);
        assert_eq!(g.take(), ["bind 0x500 0x700", "post_popup 0x500 0x700"]);
        fc4_citysit_activate_situation(m);
        assert_eq!(
            g.take(),
            ["script 0x500 0", "activation 0x500 true", "drive 0x700 true"]
        );
        let limit: f32 = fc4_citysit_time_limit_seconds(m);
        assert_eq!(limit, 60.0);
        tick(m, 10.0);
        assert!(fc4_citysit_countdown_in_progress(m));
        tick(m, 3000.0);
        assert_eq!(fc4_citysit_situation_time(m), 0.0);
        tick(m, 1000.0);
        assert!(!fc4_citysit_countdown_in_progress(m));
        let elapsed: f32 = fc4_citysit_situation_time(m);
        assert_eq!(elapsed, 1.0);
        let left: f32 = fc4_citysit_time_remaining_seconds(m);
        assert_eq!(left, 59.0);
        // The 30-second warning, once.
        g.auto(CAR, DRIVABLE, 0.0, 0.0);
        for _ in 0..28 {
            tick(m, 1000.0);
        }
        g.take();
        tick(m, 1000.0);
        assert_eq!(g.take(), ["sound 0x700 0x6c32871f"]);
        assert_eq!(state(m).warning_given, 1);
        // The time limit fails the mission; deactivate releases the advice.
        for _ in 0..30 {
            tick(m, 1000.0);
        }
        assert_eq!(fc4_citysit_current_status(m), FAILURE);
        assert!(fc4_citysit_time_remaining_seconds(m).abs() < 1e-3);
        fc4_citysit_deactivate_situation(m);
        assert_eq!(g.refs(ADV), 0);
        assert_eq!(fc4_citysit_current_status(m), IDLE);
        assert_eq!(fc4_citysit_active_situation(m), 0);
        fc4_citysit_destroy(m);
    }
}

#[test]
fn tracking_popup_and_cheat() {
    let g = Game::new();
    g.debug.set(true);
    let m = create(&g);
    g.advice(ADV, info(G));
    g.auto(CAR, DRIVABLE, 1.0, 2.0);
    g.candidates.borrow_mut().insert(ADV, vec![CAR]);
    unsafe {
        assert!(fc4_citysit_init(m));
        fc4_citysit_set_max_csi(m, 4);
        assert_eq!(fc4_citysit_max_csi(m), 4);
        g.take();
        // A situation advice appears: its vehicle carries an indicator.
        assert!(fc4_citysit_do_message(
            m,
            ADVICE_ADDED,
            0,
            0,
            0,
            ADV,
            0,
            std::ptr::null(),
            0
        ));
        assert!(fc4_citysit_is_situation_available(m, CAR));
        assert_eq!(fc4_citysit_tracked_automata_count(m), 1);
        let mut e = [fc4_citysit_track_entry::default(); 2];
        assert_eq!(fc4_citysit_track_list(m, e.as_mut_ptr(), 2), 1);
        assert_eq!((e[0].advice, e[0].automaton), (ADV, CAR));
        assert!(e[0].handle != 0);
        // Slot 22 starts it with a briefing.
        g.take();
        assert!(!fc4_citysit_initiate_situation_for_occupant(m, 0x1, 0));
        assert!(!fc4_citysit_initiate_situation_for_occupant(m, 0, ADV));
        assert!(fc4_citysit_initiate_situation_for_occupant(m, CAR, 0));
        assert!(g.take().contains(&"post_popup 0x500 0x700".to_string()));
        // The posted popup: closed with START activates the mission.
        g.button.set(6);
        assert!(fc4_citysit_do_message(
            m,
            SHOW_POPUP,
            0,
            0,
            0,
            ADV,
            CAR,
            std::ptr::null(),
            0
        ));
        let log = g.take();
        assert_eq!(&log[..3], ["hide_panel", "end_mysim", "popup 0 0x500 0x700"]);
        assert!(log.contains(&"activation 0x500 true".to_string()));
        fc4_citysit_end_situation(m);
        // The cheat text arrives as UTF-8 bytes.
        g.take();
        let text = b"csitrigger 8c151efd";
        assert!(fc4_citysit_do_message(
            m,
            CHEAT,
            CHEAT_ID,
            0,
            0,
            0,
            0,
            text.as_ptr(),
            text.len()
        ));
        assert_eq!(g.take(), ["post_initiate 0x8c151efd true"]);
        assert!(fc4_citysit_lua_activate_situation(m, G, false));
        assert_eq!(g.take(), ["post_initiate 0x8c151efd false"]);
        // Slot 16 through the sink.
        let mut name: Vec<u8> = Vec::new();
        assert!(fc4_citysit_current_target_name(
            m,
            Some(collect),
            &mut name as *mut Vec<u8> as *mut c_void
        ));
        assert_eq!(name, b"target 0x0");
        // Visibility.
        fc4_citysit_set_csi_visible(m, false);
        assert!(!fc4_citysit_csi_visible(m));
        assert!(g.take().contains(&"show_all 4 false".to_string()));
        fc4_citysit_destroy(m);
    }
}

unsafe extern "C" fn collect(ctx: *mut c_void, data: *const u8, len: usize) {
    (*(ctx as *mut Vec<u8>)).extend_from_slice(std::slice::from_raw_parts(data, len));
}
