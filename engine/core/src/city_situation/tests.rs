// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests of the city situation manager against a fake game: the §12
//! tests of `docs/specs/city-situation.md` (named `cs12_01_…` to
//! `cs12_07_…`; test 8 is the C ABI's), the lifecycle, tracking, the
//! mission state machine and the messages.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use super::*;
use crate::stream::{ByteReader, ByteWriter, SaveObject};

#[derive(Clone, Debug, Default)]
struct Auto {
    flags: u32,
    atype: u32,
    pos: (f32, f32),
}

struct World {
    log: RefCell<Vec<String>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    debug: Cell<bool>,
    advices: RefCell<BTreeMap<u64, AdviceInfo>>,
    autos: RefCell<BTreeMap<u64, Auto>>,
    /// Candidate vehicles per advice.
    candidates: RefCell<BTreeMap<u64, Vec<u64>>>,
    rewards: RefCell<Vec<u32>>,
    targets: RefCell<BTreeMap<u64, u64>>,
    status_answer: Cell<u32>,
    button: Cell<u32>,
    time_limit: Cell<f32>,
    funds: Cell<i64>,
    next_handle: Cell<u32>,
}

impl World {
    fn new() -> Rc<World> {
        Rc::new(World {
            log: RefCell::new(Vec::new()),
            refs: RefCell::new(BTreeMap::new()),
            debug: Cell::new(false),
            advices: RefCell::new(BTreeMap::new()),
            autos: RefCell::new(BTreeMap::new()),
            candidates: RefCell::new(BTreeMap::new()),
            rewards: RefCell::new(vec![0; REWARD_SLOTS]),
            targets: RefCell::new(BTreeMap::new()),
            status_answer: Cell::new(status::RUNNING),
            button: Cell::new(0),
            time_limit: Cell::new(0.0),
            funds: Cell::new(100_000),
            next_handle: Cell::new(1),
        })
    }
    fn ev(&self, s: impl Into<String>) {
        self.log.borrow_mut().push(s.into());
    }
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
    fn has(&self, s: &str) -> bool {
        self.log.borrow().iter().any(|l| l == s)
    }
    fn refs(&self, t: u64) -> i64 {
        self.refs.borrow().get(&t).copied().unwrap_or(0)
    }
    /// Adds an advice; mission type 0 in a fixture stands for type 2 (no
    /// target).
    fn advice(&self, token: u64, mut info: AdviceInfo) {
        if info.mission_type == 0 {
            info.mission_type = MISSION_TYPE_NO_TARGET;
        }
        self.advices.borrow_mut().insert(token, info);
    }
    fn auto(&self, token: u64, flags: u32, pos: (f32, f32)) {
        self.autos.borrow_mut().insert(token, Auto { flags, atype: 1, pos });
    }
}

impl CitySituationHost for World {
    fn debug_enabled(&self) -> bool {
        self.debug.get()
    }
    fn register_cheat(&self) -> bool {
        self.ev("register_cheat");
        true
    }
    fn unregister_cheat(&self) {
        self.ev("unregister_cheat");
    }
    fn subscribe(&self, m: u32) {
        self.ev(format!("subscribe {m:#x}"));
    }
    fn unsubscribe(&self, m: u32) {
        self.ev(format!("unsubscribe {m:#x}"));
    }
    fn acquire_indicator_view(&self) -> bool {
        self.ev("acquire_view");
        true
    }
    fn release_indicator_view(&self) {
        self.ev("release_view");
    }
    fn register_animation_tick(&self) {
        self.ev("register_tick");
    }
    fn unregister_animation_tick(&self) {
        self.ev("unregister_tick");
    }
    fn acquire_automata_managers(&self) -> Vec<u64> {
        let v = vec![43, 45, 44, 46];
        for &m in &v {
            *self.refs.borrow_mut().entry(m).or_insert(0) += 1;
        }
        v
    }
    fn acquire_lot_manager(&self) -> bool {
        true
    }
    fn release_lot_manager(&self) {
        self.ev("release_lot");
    }
    fn acquire_mysim(&self) -> bool {
        true
    }
    fn release_mysim(&self) {
        self.ev("release_mysim");
    }
    fn acquire_lua(&self) -> bool {
        true
    }
    fn release_lua(&self) {
        self.ev("release_lua");
    }
    fn create_lua_table(&self) {
        self.ev("create_lua");
    }
    fn remove_lua_table(&self) {
        self.ev("remove_lua");
    }
    fn release_mission_resources(&self) {
        self.ev("release_mission_resources");
    }
    fn show_all_indicators(&self, kind: u32, visible: bool) {
        self.ev(format!("show_all {kind} {visible}"));
    }
    fn preferences_rewards(&self) -> Vec<u32> {
        self.rewards.borrow().clone()
    }
    fn set_preferences_reward(&self, i: usize, g: u32) {
        self.rewards.borrow_mut()[i] = g;
    }
    fn save_preferences(&self) {
        self.ev("save_prefs");
    }
    fn add_ref(&self, t: u64) {
        *self.refs.borrow_mut().entry(t).or_insert(0) += 1;
    }
    fn release(&self, t: u64) {
        *self.refs.borrow_mut().entry(t).or_insert(0) -= 1;
    }
    fn is_situation_advice(&self, o: u64) -> bool {
        self.advices.borrow().contains_key(&o)
    }
    fn advisor_advices(&self) -> Vec<u64> {
        self.advices.borrow().keys().copied().collect()
    }
    fn find_advice(&self, guid: u32) -> Option<u64> {
        self.advices
            .borrow()
            .iter()
            .find(|(_, i)| i.guid == guid)
            .map(|(&t, _)| t)
    }
    fn advice_info(&self, a: u64) -> AdviceInfo {
        self.advices.borrow().get(&a).copied().unwrap_or_default()
    }
    fn retire_advice(&self, a: u64) {
        self.ev(format!("retire {a}"));
    }
    fn run_script(&self, a: u64, s: Script) {
        self.ev(format!("script {a} {s:?}"));
    }
    fn time_limit(&self, _a: u64) -> f32 {
        self.time_limit.get()
    }
    fn bind_mission_script(&self, a: u64, v: Option<u64>) {
        self.ev(format!("bind {a} {v:?}"));
    }
    fn build_coverage_cells(&self, a: u64) {
        self.ev(format!("coverage {a}"));
    }
    fn first_target(&self, a: u64) -> Option<u64> {
        self.targets.borrow().get(&a).copied()
    }
    fn target_count(&self, _a: u64) -> u32 {
        3
    }
    fn target_name(&self, _a: Option<u64>, t: Option<u64>) -> String {
        format!("{t:?}")
    }
    fn is_automaton(&self, o: u64) -> bool {
        self.autos.borrow().contains_key(&o)
    }
    fn occupant_has_property(&self, _o: u64, _id: u32) -> bool {
        false
    }
    fn candidate_vehicles(&self, a: u64) -> Vec<u64> {
        self.candidates.borrow().get(&a).cloned().unwrap_or_default()
    }
    fn automaton_flags(&self, a: u64) -> u32 {
        self.autos.borrow().get(&a).map_or(0, |x| x.flags)
    }
    fn automaton_type(&self, a: u64) -> u32 {
        self.autos.borrow().get(&a).map_or(0, |x| x.atype)
    }
    fn track_indicator(&self, a: u64) -> u32 {
        let h = self.next_handle.get();
        self.next_handle.set(h + 1);
        self.ev(format!("track {a} -> {h}"));
        h
    }
    fn untrack_indicator(&self, h: u32) {
        self.ev(format!("untrack {h}"));
    }
    fn add_listener(&self, a: u64) {
        self.ev(format!("listen {a}"));
    }
    fn remove_listener(&self, a: u64) {
        self.ev(format!("unlisten {a}"));
    }
    fn freeze_automata(&self, m: u64, on: bool) {
        self.ev(format!("freeze {m} {on}"));
    }
    fn play_sound(&self, a: u64, s: u32) {
        self.ev(format!("sound {a} {s:#x}"));
    }
    fn position(&self, o: u64) -> (f32, f32) {
        self.autos.borrow().get(&o).map_or((0.0, 0.0), |x| x.pos)
    }
    fn check_for_success(&self, _a: u64, _t: f32) -> u32 {
        self.status_answer.get()
    }
    fn run_pursuit(&self, _a: u64, _t: f32) {}
    fn start_countdown_effect(&self) {
        self.ev("countdown_start");
    }
    fn end_countdown_effect(&self) {
        self.ev("countdown_end");
    }
    fn outcome_effect(&self, a: u64, success: bool) {
        self.ev(format!("effect {a} {success}"));
    }
    fn player_drive_mode(&self, v: Option<u64>, on: bool) {
        self.ev(format!("drive {v:?} {on}"));
    }
    fn start_joyride(&self, v: Option<u64>) {
        self.ev(format!("joyride {v:?}"));
    }
    fn add_transient_effect(&self, x: f32, z: f32, r: f32, m: f32) {
        self.ev(format!("aura {x} {z} {r} {m}"));
    }
    fn deposit_funds(&self, amount: i64) {
        self.ev(format!("deposit {amount}"));
        // budget.md §6: a withdrawal below zero funds is refused.
        let f = self.funds.get();
        if amount >= 0 || f + amount >= 0 {
            self.funds.set(f + amount);
        }
    }
    fn send_activation(&self, a: u64, on: bool) {
        self.ev(format!("activation {a} {on}"));
    }
    fn post_show_popup(&self, a: Option<u64>, v: Option<u64>) {
        self.ev(format!("post_popup {a:?} {v:?}"));
    }
    fn post_popup_closed(&self, a: u64) {
        self.ev(format!("post_closed {a}"));
    }
    fn post_initiate(&self, g: u32, b: bool) {
        self.ev(format!("post_initiate {g:#x} {b}"));
    }
    fn hide_mission_status_panel(&self) {
        self.ev("hide_panel");
    }
    fn end_mysim_dialog(&self) {
        self.ev("end_mysim");
    }
    fn show_popup(&self, p: Popup, a: Option<u64>, v: Option<u64>) -> u32 {
        self.ev(format!("popup {p:?} {a:?} {v:?}"));
        self.button.get()
    }
}

const ADV: u64 = 0x500;
const CAR: u64 = 0x700;
const G: u32 = 0x8C15_1EFD;

fn setup() -> (Rc<World>, Rc<CitySituationManager>) {
    let w = World::new();
    let m = CitySituationManager::new(w.clone(), 0x99, 7);
    m.add_ref();
    (w, m)
}

fn running(w: &Rc<World>, m: &Rc<CitySituationManager>, info: AdviceInfo) {
    w.advice(ADV, info);
    w.auto(CAR, AUTO_FLAG_DRIVABLE, (100.0, 200.0));
    m.init();
    assert!(m.initiate_situation(ADV, Some(CAR), false));
    m.activate_situation();
    // The countdown: freeze, 4000 ms, unfreeze.
    m.tick(16.0);
    m.tick(4000.0);
    assert_eq!(m.current_status(), status::RUNNING);
    assert!(!m.countdown_in_progress());
    w.take();
}

fn tick_msg(t: f32) -> SituationMessage {
    SituationMessage {
        msg_type: msg::ANIMATION_TICK,
        data3: t.to_bits(),
        ..Default::default()
    }
}

#[test]
fn identity_and_lifecycle() {
    assert_eq!(query_interface(IID_CITY_SITUATION), Some(0));
    assert_eq!(query_interface(1), Some(0xC));
    assert_eq!(query_interface(IID_SERIALIZABLE), Some(0xC));
    assert_eq!(query_interface(IID_MESSAGE_TARGET2), Some(4));
    assert_eq!(query_interface(IID_MESSAGE_TARGET), Some(8));
    assert_eq!(query_interface(0x1234), None);

    let (w, m) = setup();
    w.debug.set(true);
    let s = m.snapshot();
    assert_eq!(s.status, status::IDLE);
    assert!(s.indicators_visible);
    assert!(m.init());
    let log = w.take();
    assert_eq!(log[0], "register_cheat");
    for (i, mt) in msg::SUBSCRIPTIONS.iter().enumerate() {
        assert_eq!(log[1 + i], format!("subscribe {mt:#x}"));
    }
    assert!(log.contains(&"show_all 4 true".to_string()));
    assert_eq!(m.snapshot().automata_managers, vec![43, 45, 44, 46]);
    assert!(m.init());
    assert!(w.take().is_empty());

    m.add_ref();
    assert_eq!(m.release(), 1);
    assert_eq!(m.release(), 0);
    let s = m.snapshot();
    assert!(s.destroyed && !s.initialized);
    let log = w.take();
    assert!(log.contains(&"unregister_cheat".to_string()));
    assert!(log.contains(&"release_view".to_string()));
    assert!(log.contains(&"remove_lua".to_string()));
    assert_eq!(w.refs(43), 0);
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
    for p in payloads {
        let (_w, m) = setup();
        let mut r = ByteReader::new(p);
        assert!(m.read(&mut r));
        assert_eq!(r.position(), p.len());
        assert!(m.snapshot().loaded_from_save);
        let mut out = ByteWriter::new();
        assert!(m.write(&mut out));
        assert_eq!(out.bytes(), p);
    }
    let (_w, m) = setup();
    assert!(m.read(&mut ByteReader::new(payloads[2])));
    assert_eq!(m.success_count(G), 1);

    // Version 3 is rejected before anything changes.
    let (_w, m) = setup();
    m.with_state(|s| {
        s.success_counts.insert(5, 9);
    });
    assert!(!m.read(&mut ByteReader::new(&[0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00])));
    assert_eq!(m.success_count(5), 9);
    assert!(!m.snapshot().loaded_from_save);

    // Version 1: the map, no visible byte (stays 1).
    let (_w, m) = setup();
    let v1 = [0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x03, 0x00];
    let mut r = ByteReader::new(&v1);
    assert!(m.read(&mut r));
    assert_eq!(r.position(), v1.len());
    assert_eq!(m.success_count(5), 3);
    assert!(m.csi_visible());
}

#[test]
fn cs12_02_duplicate_guid() {
    let (_w, m) = setup();
    let rec = [
        0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x02, 0x00, 0x07, 0x00, 0x00, 0x00, 0x05, 0x00,
        0x01,
    ];
    assert!(m.read(&mut ByteReader::new(&rec)));
    assert_eq!(m.success_count(7), 2);
    assert_eq!(m.snapshot().success_counts.len(), 1);
}

#[test]
fn cs12_03_profile_rewards() {
    let (w, m) = setup();
    let rec = [
        0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x07, 0x00, 0x08, 0x00, 0x00, 0x00, 0x04, 0x00,
        0x01,
    ];
    assert!(m.read(&mut ByteReader::new(&rec)));
    {
        let mut r = w.rewards.borrow_mut();
        r[0] = 7;
        r[1] = 9;
        r[3] = 8; // after the first zero: ignored
    }
    m.init();
    assert_eq!(m.success_count(7), 1);
    assert_eq!(m.success_count(9), 1);
    assert_eq!(m.success_count(8), 4);
}

#[test]
fn cs12_04_idle_tick() {
    let (w, m) = setup();
    w.advice(
        ADV,
        AdviceInfo {
            guid: G,
            ..Default::default()
        },
    );
    m.init();
    w.take();
    let before = m.snapshot();
    let draw = m.peek_random();
    for _ in 0..10_000 {
        m.do_message(&tick_msg(33.0));
    }
    assert!(w.take().is_empty());
    assert_eq!(m.snapshot(), before);
    assert_eq!(m.peek_random(), draw);
}

#[test]
fn cs12_05_success_payout() {
    let (w, m) = setup();
    running(
        &w,
        &m,
        AdviceInfo {
            guid: G,
            success_money: 10_000,
            success_aura: (256.0, 50.0),
            ..Default::default()
        },
    );
    w.status_answer.set(status::SUCCESS);
    m.tick(16.0);
    assert_eq!(m.current_status(), status::SUCCESS);
    assert_eq!(m.snapshot().end_delay, 1.0);
    w.take();
    for _ in 0..4 {
        m.tick(250.0);
    }
    let log = w.take();
    assert_eq!(
        log,
        vec![
            format!("effect {ADV} true"),
            "aura 100 200 256 50".to_string(),
            "deposit 10000".to_string(),
            format!("script {ADV} Success"),
            format!("retire {ADV}"),
            format!("post_popup Some({ADV}) Some({CAR})"),
        ]
    );
    assert_eq!(m.success_count(G), 1);
    // Further ticks do nothing more.
    for _ in 0..10 {
        m.tick(250.0);
    }
    assert!(w.take().is_empty());
}

#[test]
fn cs12_06_failure_penalty_with_no_money() {
    let (w, m) = setup();
    w.funds.set(1_000);
    running(
        &w,
        &m,
        AdviceInfo {
            guid: G,
            failure_money: -3000,
            ..Default::default()
        },
    );
    w.status_answer.set(status::FAILURE);
    m.tick(16.0);
    for _ in 0..4 {
        m.tick(250.0);
    }
    let log = w.take();
    assert!(log.contains(&"deposit -3000".to_string()));
    assert_eq!(w.funds.get(), 1_000);
    assert_eq!(log.last().unwrap(), &format!("post_popup Some({ADV}) Some({CAR})"));
    assert_eq!(m.success_count(G), 0);
}

#[test]
fn cs12_07_no_city_end() {
    let (w, m) = setup();
    w.funds.set(-150_000);
    running(
        &w,
        &m,
        AdviceInfo {
            guid: G,
            failure_money: -3000,
            ..Default::default()
        },
    );
    for _ in 0..100 {
        m.tick(100.0);
    }
    w.status_answer.set(status::FAILURE);
    for _ in 0..20 {
        m.tick(100.0);
    }
    // The host has no close / region request at all; the log holds only
    // the mission's own calls.
    for l in w.take() {
        assert!(!l.contains("0xaa5cf1e3") && !l.contains("region"), "{l}");
    }
}

#[test]
fn success_reward_and_saturation() {
    let (w, m) = setup();
    running(
        &w,
        &m,
        AdviceInfo {
            guid: G,
            flags: SITUATION_FLAG_REWARD,
            ..Default::default()
        },
    );
    m.with_state(|s| {
        s.success_counts.insert(G, MAX_SUCCESS_COUNT);
    });
    w.rewards.borrow_mut()[0] = 5;
    m.handle_status_changed(status::SUCCESS);
    m.tick(1000.0);
    assert_eq!(m.success_count(G), MAX_SUCCESS_COUNT);
    assert_eq!(w.rewards.borrow()[1], G);
    assert!(w.has("save_prefs"));
    assert!(!w
        .log
        .borrow()
        .iter()
        .any(|l| l.starts_with("deposit") || l.starts_with("aura")));
}

#[test]
fn long_end_delay() {
    let (w, m) = setup();
    running(
        &w,
        &m,
        AdviceInfo {
            guid: G,
            end_delay: 3.0,
            ..Default::default()
        },
    );
    m.handle_status_changed(status::FAILURE);
    assert_eq!(m.snapshot().end_delay, 1.0);
    // With the vehicle flag the advice's delay applies.
    let (w2, m2) = setup();
    running(
        &w2,
        &m2,
        AdviceInfo {
            guid: G,
            end_delay: 3.0,
            ..Default::default()
        },
    );
    w2.autos.borrow_mut().get_mut(&CAR).unwrap().flags |= AUTO_FLAG_LONG_END;
    m2.handle_status_changed(status::FAILURE);
    assert_eq!(m2.snapshot().end_delay, 3.0);
    drop(w);
}

#[test]
fn activation_and_countdown() {
    let (w, m) = setup();
    w.advice(
        ADV,
        AdviceInfo {
            guid: G,
            ..Default::default()
        },
    );
    w.time_limit.set(60.0);
    m.init();
    w.take();
    assert!(m.initiate_situation(ADV, Some(CAR), true));
    assert!(!m.initiate_situation(ADV, Some(CAR), true));
    assert_eq!(w.refs(ADV), 1);
    assert_eq!(
        w.take(),
        vec![
            format!("bind {ADV} Some({CAR})"),
            format!("post_popup Some({ADV}) Some({CAR})")
        ]
    );
    m.activate_situation();
    assert_eq!(
        w.take(),
        vec![
            format!("script {ADV} Start"),
            format!("activation {ADV} true"),
            format!("drive Some({CAR}) true"),
        ]
    );
    assert_eq!(m.time_limit_seconds(), 60.0);
    m.tick(10.0);
    let log = w.take();
    assert_eq!(log[0], "freeze 43 true");
    assert_eq!(log[4], "countdown_start");
    assert!(m.countdown_in_progress());
    m.tick(3000.0);
    assert!(w.take().is_empty());
    assert_eq!(m.situation_time(), 0.0);
    m.tick(1000.0);
    let log = w.take();
    assert_eq!(log[0], "countdown_end");
    assert_eq!(log[1], "freeze 43 false");
    assert!(!m.countdown_in_progress());
    // The mission clock ran 1 s in that tick.
    assert_eq!(m.situation_time(), 1.0);
    // 30-second warning, once (at 30 s elapsed of 60).
    w.auto(CAR, AUTO_FLAG_DRIVABLE, (0.0, 0.0));
    for _ in 0..28 {
        m.tick(1000.0);
    }
    assert!(w.take().is_empty());
    m.tick(1000.0);
    assert_eq!(w.take(), vec![format!("sound {CAR} {WARNING_SOUND:#x}")]);
    m.tick(1000.0);
    assert!(w.take().is_empty());
    // The time limit fails the mission.
    for _ in 0..29 {
        m.tick(1000.0);
    }
    assert_eq!(m.current_status(), status::FAILURE);
    assert!((m.time_remaining_seconds()).abs() < 1e-3);
    // Deactivate releases the advice.
    m.deactivate_situation();
    assert_eq!(w.refs(ADV), 0);
    assert_eq!(m.current_status(), status::IDLE);
    assert!(w.has(&format!("activation {ADV} false")));
}

#[test]
fn could_not_start_and_targets() {
    let (w, m) = setup();
    w.advice(
        ADV,
        AdviceInfo {
            guid: G,
            mission_type: 1,
            ..Default::default()
        },
    );
    m.init();
    w.take();
    assert!(!m.initiate_situation(ADV, Some(CAR), true));
    assert_eq!(m.current_status(), status::COULD_NOT_START);
    assert!(w.has(&format!("post_popup Some({ADV}) None")));
    // Close on "could not start" cancels.
    w.button.set(button::CLOSE);
    m.show_situation_popup(Some(ADV), None);
    assert!(w.has(&format!("popup Success Some({ADV}) None")));
    assert_eq!(m.active_situation(), None);
    assert_eq!(w.refs(ADV), 0);

    w.targets.borrow_mut().insert(ADV, 0x900);
    w.take();
    assert!(m.initiate_situation(ADV, None, false));
    assert_eq!(m.current_target(), Some(0x900));
    assert_eq!(m.num_targets(), 3);
    assert_eq!(m.num_targets_remaining(), 3);
    assert_eq!(m.current_target_name(), "Some(2304)");

    // Coverage missions build their cells.
    w.advice(
        0x501,
        AdviceInfo {
            guid: 2,
            mission_type: MISSION_TYPE_COVERAGE,
            ..Default::default()
        },
    );
    assert!(m.initiate_situation(0x501, None, false));
    assert!(w.has("coverage 1281"));
    assert_eq!(w.refs(ADV), 0);
    assert_eq!(w.refs(0x501), 1);
}

#[test]
fn tracking_and_max_csi() {
    let (w, m) = setup();
    m.init();
    m.set_max_csi(1);
    w.advice(
        ADV,
        AdviceInfo {
            guid: G,
            ..Default::default()
        },
    );
    w.advice(
        0x501,
        AdviceInfo {
            guid: 2,
            ..Default::default()
        },
    );
    w.auto(CAR, AUTO_FLAG_DRIVABLE, (0.0, 0.0));
    w.auto(0x701, AUTO_FLAG_DRIVABLE | 1, (0.0, 0.0)); // blocked
    w.auto(0x702, AUTO_FLAG_DRIVABLE, (0.0, 0.0));
    w.candidates.borrow_mut().insert(ADV, vec![0x701, CAR]);
    w.candidates.borrow_mut().insert(0x501, vec![CAR, 0x702]);
    w.take();
    m.do_message(&SituationMessage {
        msg_type: msg::ADVICE_ADDED,
        object: Some(ADV),
        ..Default::default()
    });
    assert_eq!(w.take(), vec![format!("track {CAR} -> 1"), format!("listen {CAR}")]);
    assert!(m.is_situation_available(CAR));
    m.do_message(&SituationMessage {
        msg_type: msg::ADVICE_ADDED,
        object: Some(0x501),
        ..Default::default()
    });
    // Max CSI 1: the second entry has no vehicle.
    assert!(w.take().is_empty());
    assert_eq!(m.snapshot().track_list.len(), 2);
    m.set_max_csi(5);
    // An inserted automaton lets free entries look again.
    m.do_message(&SituationMessage {
        msg_type: msg::OCCUPANT_INSERTED,
        object: Some(0x702),
        ..Default::default()
    });
    assert_eq!(w.take(), vec!["track 1794 -> 2".to_string(), "listen 1794".to_string()]);
    assert_eq!(m.tracked_automata_count(), 2);
    // Removing the first advice untracks its vehicle.
    m.do_message(&SituationMessage {
        msg_type: msg::ADVICE_REMOVED,
        object: Some(ADV),
        ..Default::default()
    });
    assert_eq!(w.take(), vec!["untrack 1".to_string(), format!("unlisten {CAR}")]);
    assert_eq!(m.tracked_automata_count(), 1);
    // AdviceRemovedAll is ignored.
    assert!(m.do_message(&SituationMessage {
        msg_type: msg::ADVICE_REMOVED_ALL,
        ..Default::default()
    }));
    assert_eq!(m.tracked_automata_count(), 1);
    // A removed occupant is dropped; its entry looks for another (CAR).
    m.do_message(&SituationMessage {
        msg_type: msg::OCCUPANT_REMOVED,
        object: Some(0x702),
        ..Default::default()
    });
    assert_eq!(
        w.take(),
        vec![
            "untrack 2".to_string(),
            "unlisten 1794".to_string(),
            format!("track {CAR} -> 3"),
            format!("listen {CAR}"),
        ]
    );
    // Disabled notifications drop it as well.
    m.do_message(&SituationMessage {
        msg_type: msg::AUTOMATON_NOTIFY,
        object: Some(CAR),
        data2: notify::DISABLED_C,
        data3: 0x13,
        ..Default::default()
    });
    assert_eq!(m.tracked_automata_count(), 1);
    w.candidates.borrow_mut().clear();
    m.do_message(&SituationMessage {
        msg_type: msg::AUTOMATON_NOTIFY,
        object: Some(CAR),
        data2: notify::DISABLED_C,
        data3: notify::DISABLED_C_DATA3,
        ..Default::default()
    });
    assert_eq!(m.tracked_automata_count(), 0);
}

#[test]
fn can_player_drive_rule() {
    assert!(can_player_drive(2, 1));
    assert!(!can_player_drive(0, 1));
    assert!(!can_player_drive(3, 1));
    assert!(!can_player_drive(2 | 0x0800_0000, 1));
    assert!(!can_player_drive(2 | 0x1000_0000, 1));
    assert!(!can_player_drive(2, 0x34));
    assert!(!can_player_drive(2, 0x14));
}

#[test]
fn post_city_init_and_guid_initiation() {
    let (w, m) = setup();
    w.advice(
        ADV,
        AdviceInfo {
            guid: G,
            ..Default::default()
        },
    );
    w.auto(CAR, AUTO_FLAG_DRIVABLE, (0.0, 0.0));
    w.candidates.borrow_mut().insert(ADV, vec![CAR]);
    m.init();
    m.set_max_csi(5);
    // Not loaded: nothing.
    m.do_message(&SituationMessage {
        msg_type: msg::POST_CITY_INIT,
        ..Default::default()
    });
    assert_eq!(m.tracked_automata_count(), 0);
    m.with_state(|s| s.loaded_from_save = true);
    m.do_message(&SituationMessage {
        msg_type: msg::POST_CITY_INIT,
        ..Default::default()
    });
    assert_eq!(m.tracked_automata_count(), 1);
    w.take();
    // Without a briefing the mission is activated at once, with the entry's vehicle.
    m.do_message(&SituationMessage {
        msg_type: msg::INITIATE_BY_GUID,
        data1: G,
        data2: 0,
        ..Default::default()
    });
    assert_eq!(m.current_status(), status::RUNNING);
    assert_eq!(m.active_auto(), Some(CAR));
    assert!(!m.initiate_situation_by_guid(0x1234, true, 0));
    // The player leaves the vehicle: the situation ends.
    m.do_message(&SituationMessage {
        msg_type: msg::PLAYER_LEFT_VEHICLE,
        ..Default::default()
    });
    assert_eq!(m.active_situation(), None);
    assert_eq!(w.refs(ADV), 0);
    // Occupant slot 22.
    assert!(m.initiate_situation_for_occupant(CAR));
    assert!(!m.initiate_situation_for_occupant(0x1));
}

#[test]
fn popup_messages_and_buttons() {
    let (w, m) = setup();
    w.advice(
        ADV,
        AdviceInfo {
            guid: G,
            ..Default::default()
        },
    );
    m.init();
    assert!(m.initiate_situation(ADV, Some(CAR), true));
    w.take();
    w.button.set(button::START);
    m.do_message(&SituationMessage {
        msg_type: msg::SHOW_POPUP,
        object: Some(ADV),
        void1: Some(CAR),
        ..Default::default()
    });
    let log = w.take();
    assert_eq!(log[0], "hide_panel");
    assert_eq!(log[1], "end_mysim");
    assert_eq!(log[2], format!("popup Briefing Some({ADV}) Some({CAR})"));
    assert_eq!(m.current_status(), status::RUNNING);
    // Joyride.
    w.button.set(button::JOYRIDE);
    m.show_situation_popup(Some(ADV), Some(CAR));
    assert!(w.has(&format!("joyride Some({CAR})")));
    // A finished mission: close posts 0x6BB350E4, which ends it.
    m.handle_status_changed(status::FAILURE);
    w.button.set(button::CLOSE);
    m.show_situation_popup(Some(ADV), Some(CAR));
    assert!(w.has(&format!("popup Failure Some({ADV}) Some({CAR})")));
    assert!(w.has(&format!("post_closed {ADV}")));
    // Player left the vehicle during status 2: nothing.
    m.do_message(&SituationMessage {
        msg_type: msg::PLAYER_LEFT_VEHICLE,
        ..Default::default()
    });
    assert_eq!(m.active_situation(), Some(ADV));
    w.take();
    m.do_message(&SituationMessage {
        msg_type: msg::POPUP_CLOSED,
        object: Some(ADV),
        ..Default::default()
    });
    assert_eq!(w.take()[0], format!("drive Some({CAR}) false"));
    assert_eq!(m.active_situation(), None);
    // 0x2C16BDE7: the popup without an advice.
    w.button.set(0);
    m.do_message(&SituationMessage {
        msg_type: msg::POPUP_NO_ADVICE,
        ..Default::default()
    });
    assert!(w.has("popup Briefing None None"));
}

#[test]
fn window_cheat_and_lua() {
    let (w, m) = setup();
    w.debug.set(true);
    w.advice(
        ADV,
        AdviceInfo {
            guid: G,
            flags: SITUATION_FLAG_WINDOW,
            ..Default::default()
        },
    );
    m.init();
    assert!(m.initiate_situation(ADV, None, false));
    m.do_message(&SituationMessage {
        msg_type: msg::WINDOW,
        object: Some(0x800),
        ..Default::default()
    });
    assert_eq!(w.refs(0x800), 1);
    assert_eq!(m.snapshot().window, Some(0x800));
    w.take();
    m.do_message(&SituationMessage {
        msg_type: msg::CHEAT,
        data1: CHEAT_ID,
        text: Some("csitrigger 8c151efd".into()),
        ..Default::default()
    });
    assert_eq!(w.take(), vec!["post_initiate 0x8c151efd true".to_string()]);
    assert_eq!(parse_cheat("CSITRIGGER 0x10"), Some(0x10));
    assert_eq!(parse_cheat("csitrigger"), None);
    assert_eq!(parse_cheat("other 10"), None);
    assert!(m.lua_activate_situation(G, false));
    assert_eq!(w.take(), vec!["post_initiate 0x8c151efd false".to_string()]);
    m.shutdown();
    assert_eq!(w.refs(0x800), 0);
}

#[test]
fn moved_and_removed_vehicle() {
    let (w, m) = setup();
    running(
        &w,
        &m,
        AdviceInfo {
            guid: G,
            ..Default::default()
        },
    );
    w.status_answer.set(status::SUCCESS);
    m.do_message(&SituationMessage {
        msg_type: msg::AUTOMATON_NOTIFY,
        object: Some(CAR),
        data2: notify::MOVED,
        ..Default::default()
    });
    assert_eq!(m.current_status(), status::SUCCESS);

    let (w, m) = setup();
    running(
        &w,
        &m,
        AdviceInfo {
            guid: G,
            ..Default::default()
        },
    );
    m.do_message(&SituationMessage {
        msg_type: msg::OCCUPANT_REMOVED,
        object: Some(CAR),
        ..Default::default()
    });
    assert_eq!(m.current_status(), status::FAILURE);
    assert_eq!(m.active_auto(), None);

    let (w2, m2) = setup();
    running(
        &w2,
        &m2,
        AdviceInfo {
            guid: G,
            ..Default::default()
        },
    );
    m2.do_message(&SituationMessage {
        msg_type: msg::AUTOMATON_NOTIFY,
        object: Some(CAR),
        data2: notify::REMOVED,
        ..Default::default()
    });
    assert_eq!(m2.current_status(), status::FAILURE);
    drop(w);
}

#[test]
fn getters_and_reset() {
    let (w, m) = setup();
    assert_eq!(m.situation_aura(), None);
    assert_eq!(m.situation_money(), None);
    w.advice(
        ADV,
        AdviceInfo {
            guid: G,
            success_money: 5000,
            success_aura: (32.0, 10.0),
            ..Default::default()
        },
    );
    m.init();
    m.initiate_situation(ADV, None, false);
    assert_eq!(m.situation_aura(), Some((32.0, 10.0)));
    assert_eq!(m.situation_money(), Some(5000));
    m.set_csi_visible(false);
    assert!(!m.csi_visible());
    assert!(w.has("show_all 4 false"));
    assert!(m.handle_tuning_parameters_changed());
    m.with_state(|s| {
        s.success_counts.insert(1, 4);
    });
    w.rewards.borrow_mut()[0] = 1;
    m.reset_tutorials();
    assert_eq!(m.success_count(1), 0);
    assert!(w.rewards.borrow().iter().all(|&g| g == 0));
    assert!(w.has("save_prefs"));
    // The generator draws only on request.
    let a = m.peek_random();
    let _ = m.random_uniform_u32(4);
    assert_ne!(m.peek_random(), a);
    let r = m.random_range_i32(-5, 5);
    assert!((-5..5).contains(&r));
}
