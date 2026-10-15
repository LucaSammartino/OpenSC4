// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The §17 tests and the behaviours of `docs/specs/advisors.md`.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use crate::julian::julian_from_civil;
use crate::stream::{ByteReader, ByteWriter, OutStream, RefResolver, RefValue, SaveObject, SaveRegistry};

use super::scripted::{HostCall, Occupant, ScriptedHost};
use super::*;

const ID: i32 = 23;

fn ae(guid: u32, trigger: &str) -> AdviceEntry {
    AdviceEntry {
        guid,
        class_id: clsid::ADVICE,
        advice_type: 1,
        priority: 50,
        frequency: 35,
        timeout: 45,
        trigger: Some(trigger.as_bytes().to_vec()),
        ..AdviceEntry::default()
    }
}

fn advisor_entry(class_id: u32, id: i32) -> AdvisorEntry {
    AdvisorEntry {
        class_id,
        id,
        caption: b"Planning|tail".to_vec(),
        name: b"Bob".to_vec(),
        advice_type: 1,
    }
}

struct F {
    h: Rc<ScriptedHost>,
    sys: Rc<AdvisorSystem>,
}

impl F {
    fn new(class_id: u32, advices: Vec<AdviceEntry>) -> F {
        let h = ScriptedHost::new();
        *h.advisors.borrow_mut() = Some(vec![advisor_entry(class_id, ID)]);
        *h.advices.borrow_mut() = advices;
        let sys = AdvisorSystem::new(h.clone(), 0);
        assert!(sys.init());
        F { h, sys }
    }
    fn plain(advices: Vec<AdviceEntry>) -> F {
        F::new(clsid::ADVISOR, advices)
    }
    fn adv(&self) -> Rc<Advisor> {
        self.sys.get_advisor(ID).expect("advisor")
    }
    fn advice(&self, guid: u32) -> Rc<Advice> {
        self.adv().advice_by_guid(guid).expect("advice")
    }
    fn day(&self, day: i32) {
        self.h.set_day(day);
        self.adv().simulate(0);
    }
    fn info(&self, guid: u32) -> AdviceInfo {
        let adv = self.adv();
        let a = self.advice(guid);
        adv.state()
            .infos
            .into_iter()
            .find(|i| Rc::ptr_eq(&i.advice, &a))
            .unwrap()
    }
    fn set_month(&self, m: i32) {
        self.adv().with_state(|s| s.month_counter = m);
    }
    fn log(&self) -> Vec<HostCall> {
        self.h.take_log()
    }
}

fn posts(log: &[HostCall], msg_type: u32) -> Vec<HostCall> {
    log.iter()
        .filter(|c| matches!(c, HostCall::Post { msg, .. } | HostCall::Send { msg, .. } if *msg == msg_type))
        .cloned()
        .collect()
}

// ================================================================ §17.1

#[test]
fn t01_batch_size() {
    for (n, b) in [(0, 1), (1, 1), (29, 1), (30, 2), (59, 2), (60, 3), (100, 4), (250, 9)] {
        assert_eq!(batch_size(n), b, "n = {n}");
    }
}

// ================================================================ §17.2, §17.3

/// An advisor in month 5 on day 1000 with one advice created then.
fn gate_fixture(once: bool) -> F {
    let f = F::plain(vec![]);
    let adv = f.adv();
    adv.with_state(|s| {
        s.month_counter = 5;
        s.current_day = 1000;
    });
    *f.h.advices.borrow_mut() = vec![AdviceEntry {
        once,
        ..ae(0x100, "true")
    }];
    assert!(adv.init(1));
    let i = f.info(0x100);
    assert_eq!((i.next_day, i.expiry, i.last_month), (1000, 0, 4));
    f
}

#[test]
fn t02_evaluation_gate() {
    let f = gate_fixture(false);
    f.log();
    f.day(1000);
    let a = f.advice(0x100);
    assert!(a.is_active());
    assert_eq!(f.info(0x100).expiry, 1045);
    assert_eq!(f.info(0x100).last_month, 5);
    let log = f.log();
    assert_eq!(posts(&log, msg::ADVICE_ADDED).len(), 1);
    // Day 1045: timed out.
    f.day(1045);
    assert!(!a.is_active());
    assert_eq!(f.info(0x100).next_day, 1080);
    assert_eq!(f.info(0x100).expiry, 0);
    let log = f.log();
    assert_eq!(
        posts(&log, msg::ADVICE_REMOVED),
        vec![HostCall::Send {
            msg: msg::ADVICE_REMOVED,
            data1: 0,
            obj: "advice:0x100".into()
        }]
    );
    // Day 1080, month 6, lastMonth 6: not evaluated.
    f.set_month(6);
    f.adv().with_state(|s| s.infos[0].last_month = 6);
    f.day(1080);
    assert!(!a.is_active());
    // After the next monthly step: evaluated and activated.
    f.adv().simulate(1);
    f.day(1080);
    assert!(a.is_active());
}

#[test]
fn t03_once() {
    let f = gate_fixture(true);
    f.day(1000);
    f.day(1045);
    assert_eq!(f.info(0x100).next_day, NEVER);
    for m in 6..10 {
        f.set_month(m);
        f.day(1100 + m * 40);
        assert!(!f.advice(0x100).is_active());
    }
}

// ================================================================ §17.4

#[test]
fn t04_persist_and_no_timeout() {
    let f = F::plain(vec![
        AdviceEntry {
            persist: true,
            ..ae(0x1, "t1")
        },
        AdviceEntry {
            no_timeout: true,
            ..ae(0x2, "t2")
        },
        AdviceEntry {
            persist: true,
            no_timeout: true,
            ..ae(0x3, "t3")
        },
    ]);
    for t in ["t1", "t2", "t3"] {
        f.h.set_trigger(t.as_bytes(), LuaResult::Bool(true));
    }
    f.set_month(1);
    // Batch size 1: three days to visit all three.
    f.day(1000);
    f.day(1001);
    f.day(1002);
    for g in [1, 2, 3] {
        assert!(f.advice(g).is_active(), "{g}");
    }
    for t in ["t1", "t2", "t3"] {
        f.h.set_trigger(t.as_bytes(), LuaResult::Bool(false));
    }
    f.day(1003);
    assert!(f.advice(1).is_active());
    assert!(!f.advice(2).is_active());
    assert!(f.advice(3).is_active());
    // Advice 1 expires at 1000 + 45.
    f.day(1045);
    assert!(!f.advice(1).is_active());
    f.day(5000);
    assert!(f.advice(3).is_active());
}

// ================================================================ §17.5

#[test]
fn t05_trigger_values() {
    let f = F::plain(vec![ae(0x1, "x")]);
    let a = f.advice(0x1);
    for (r, want) in [
        (LuaResult::Bool(true), true),
        (LuaResult::Number(1.0), true),
        (LuaResult::Number(0.0), false),
        (LuaResult::Number(f64::NAN), true),
        (LuaResult::Str(b"1".to_vec()), false),
        (LuaResult::Other, false),
        (LuaResult::Error, false),
    ] {
        f.h.set_trigger(b"x", r.clone());
        assert_eq!(a.get_trigger(), want, "{r:?}");
    }
    f.h.set_trigger(b"x", LuaResult::Bool(false));
    a.set_flags(flags::FORCED_TRIGGER, true);
    assert!(a.get_trigger());
    a.set_flags(flags::FORCED_TRIGGER, false);
    f.h.set_trigger(b"x", LuaResult::Bool(true));
    a.set_flags(flags::NO_TEXT, true);
    assert!(!a.get_trigger());
    // The literal readings of the scripted host.
    assert!(matches!(scripted::literal_result(b"0/0"), LuaResult::Number(n) if n.is_nan()));
    assert_eq!(scripted::literal_result(b"\"1\""), LuaResult::Str(b"1".to_vec()));
}

#[test]
fn trigger_without_compiled_function_is_false() {
    let h = ScriptedHost::new();
    h.bad_triggers.borrow_mut().push(b"bad".to_vec());
    *h.advisors.borrow_mut() = Some(vec![advisor_entry(clsid::ADVISOR, ID)]);
    *h.advices.borrow_mut() = vec![
        ae(1, "bad"),
        AdviceEntry {
            trigger: None,
            ..ae(2, "")
        },
    ];
    let sys = AdvisorSystem::new(h.clone(), 0);
    sys.init();
    let adv = sys.get_advisor(ID).unwrap();
    let a = adv.advice_by_guid(1).unwrap();
    assert_eq!(a.state().trigger, NO_TRIGGER);
    assert!(!a.get_trigger());
    // A missing trigger compiles "0".
    let b = adv.advice_by_guid(2).unwrap();
    assert_eq!(b.state().trigger_source, b"0");
    assert!(!b.get_trigger());
}

#[test]
fn trigger_pushes_the_context_advice() {
    let f = F::plain(vec![ae(0x1, "ctx")]);
    let sys = f.sys.clone();
    let seen = Rc::new(RefCell::new(None));
    let s2 = seen.clone();
    *f.h.trigger_hook.borrow_mut() = Some(Box::new(move |_| {
        *s2.borrow_mut() = sys.context_advice().map(|a| a.guid());
    }));
    f.advice(1).get_trigger();
    assert_eq!(*seen.borrow(), Some(1));
    assert!(f.sys.context_advice().is_none());
}

// ================================================================ §17.6

#[test]
fn t06_event_advice() {
    let f = F::plain(vec![AdviceEntry {
        event: event::NEW_CITY,
        frequency: 1000,
        timeout: 1,
        ..ae(0x10, "true")
    }]);
    let a = f.advice(0x10);
    assert!(a.check_flags(flags::EVENT));
    // The advisor subscribed to the event.
    assert_eq!(f.sys.subscribers(event::NEW_CITY).len(), 1);
    f.set_month(3);
    f.day(1000);
    assert!(!a.is_active(), "event advices are not batch-evaluated");
    let s = f.sys.new_subject();
    s.set_data1(7);
    f.sys.push_event(event::NEW_CITY, Some(s.clone()));
    f.day(1001);
    assert!(a.is_active());
    assert!(Rc::ptr_eq(&a.event_subject().unwrap(), &s));
    // Timeout 1: deactivated next day (nextDay = 1002 + 1000).
    f.day(1002);
    assert!(!a.is_active());
    assert_eq!(f.info(0x10).next_day, 2002);
    f.sys.push_event(event::NEW_CITY, None);
    f.day(1003);
    assert!(a.is_active(), "re-activated regardless of nextDay");
}

#[test]
fn event_on_active_advice_refreshes_subject() {
    let f = F::plain(vec![AdviceEntry {
        event: 0x77,
        ..ae(0x10, "true")
    }]);
    let a = f.advice(0x10);
    f.sys.push_event(0x77, None);
    f.day(1000);
    assert!(a.is_active());
    let s = f.sys.new_subject();
    f.sys.push_event(0x77, Some(s.clone()));
    f.day(1001);
    assert!(Rc::ptr_eq(&a.event_subject().unwrap(), &s));
}

#[test]
fn events_pushed_during_the_event_pass_wait() {
    let f = F::plain(vec![
        AdviceEntry {
            event: 0x77,
            ..ae(0x10, "first")
        },
        AdviceEntry {
            event: 0x78,
            ..ae(0x11, "true")
        },
    ]);
    let sys = f.sys.clone();
    *f.h.trigger_hook.borrow_mut() = Some(Box::new(move |t| {
        if t == b"first" {
            sys.push_event(0x78, None);
        }
    }));
    f.h.set_trigger(b"first", LuaResult::Bool(false));
    f.sys.push_event(0x77, None);
    f.day(1000);
    assert!(!f.advice(0x11).is_active());
    f.day(1001);
    assert!(f.advice(0x11).is_active());
}

// ================================================================ §17.7

#[test]
fn t07_activation_messages() {
    let f = F::plain(vec![AdviceEntry {
        command: 0x1234,
        ..ae(0x5, "true")
    }]);
    f.set_month(1);
    f.log();
    f.day(1000);
    let log = f.log();
    let msgs: Vec<HostCall> = log
        .iter()
        .filter(|c| matches!(c, HostCall::Post { .. } | HostCall::Send { .. }))
        .cloned()
        .collect();
    assert_eq!(
        msgs,
        vec![
            HostCall::Post {
                msg: msg::ADVICE_ADDED,
                data1: 0,
                obj: "advice:0x5".into()
            },
            HostCall::Post {
                msg: msg::ADVICE_COMMAND,
                data1: 0x1234,
                obj: format!("advisor:{ID}")
            },
        ]
    );
    f.advice(5).set_flags(flags::EXPIRE_REQUEST, true);
    f.day(1001);
    let log = f.log();
    assert_eq!(
        posts(&log, msg::ADVICE_REMOVED),
        vec![HostCall::Send {
            msg: msg::ADVICE_REMOVED,
            data1: 0,
            obj: "advice:0x5".into()
        }]
    );
}

#[test]
fn active_dirty_text_is_rebuilt_and_changed_sent() {
    let f = F::plain(vec![ae(0x5, "true")]);
    f.set_month(1);
    f.day(1000);
    let a = f.advice(5);
    a.set_flags(flags::TEXT_DIRTY, true);
    f.log();
    f.day(1001);
    let log = f.log();
    assert!(log.contains(&HostCall::Rebuild { guid: 5 }));
    assert_eq!(posts(&log, msg::ADVICE_CHANGED).len(), 1);
    assert!(!a.check_flags(flags::TEXT_DIRTY));
}

#[test]
fn activation_asserts_and_deactivation_retracts_effects() {
    let f = F::plain(vec![AdviceEntry {
        effects: vec![0xE1, 0xE2],
        ..ae(0x5, "true")
    }]);
    f.set_month(1);
    f.log();
    f.day(1000);
    let log = f.log();
    let eff: Vec<_> = log.iter().filter(|c| matches!(c, HostCall::Effect { .. })).collect();
    // The list was appended by both Inits of the system's Init [quirk].
    assert_eq!(f.advice(5).state().effects, vec![0xE1, 0xE2, 0xE1, 0xE2]);
    assert_eq!(eff.len(), 4);
    assert!(matches!(eff[0], HostCall::Effect { id: 0xE1, assert: true }));
    f.adv().retire_advice(Some(&f.advice(5)));
    let log = f.log();
    assert!(log.contains(&HostCall::Effect {
        id: 0xE2,
        assert: false
    }));
}

// ================================================================ §17.8, §17.9

fn date_day(y: i32, m: i32, d: i32) -> i32 {
    julian_from_civil(y, m, d)
}

#[test]
fn t08_ticker_monthly_then_daily() {
    let f = F::plain(vec![]);
    let adv = f.adv();
    f.h.set_day(date_day(2001, 3, 27));
    f.sys.on_ticker();
    assert_eq!(adv.month_counter(), 0);
    f.log();
    f.h.set_day(date_day(2001, 3, 28));
    f.sys.on_ticker();
    assert_eq!(adv.month_counter(), 1);
    assert_eq!(adv.current_day(), date_day(2001, 3, 28));
    assert_eq!(f.h.global("g_day"), Some(28.0));
    assert_eq!(f.h.global("g_month"), Some(3.0));
    assert_eq!(f.h.global("g_year"), Some(2001.0));
    assert_eq!(f.h.global("g_year_count"), Some(1.0));
    assert_eq!(f.h.global("g_date"), Some(f64::from(date_day(2001, 3, 28))));
    let log = f.log();
    assert_eq!(log.first(), Some(&HostCall::CollectGarbage));
    assert_eq!(log.last(), Some(&HostCall::Ticker));
    // 27 → 29: no monthly step.
    f.h.set_day(date_day(2001, 4, 27));
    f.sys.on_ticker();
    f.h.set_day(date_day(2001, 4, 29));
    f.sys.on_ticker();
    assert_eq!(adv.month_counter(), 1);
    // Not established: the chain stops.
    f.h.established.set(false);
    f.log();
    f.h.set_day(date_day(2001, 4, 30));
    f.sys.on_ticker();
    assert!(f.log().is_empty());
}

#[test]
fn t09_paused_updates() {
    let f = F::plain(vec![]);
    f.h.now_ms.set(1000);
    f.h.set_day(date_day(2001, 3, 10));
    f.sys.on_ticker();
    assert_eq!(f.sys.state().idle_start, Some(1000));
    f.h.paused.set(true);
    f.h.now_ms.set(1300);
    f.sys.on_ticker();
    assert_eq!(f.sys.state().idle_start, Some(1000), "300 ms: none");
    f.h.now_ms.set(1600);
    f.sys.on_ticker();
    assert_eq!(f.sys.state().idle_start, Some(1600), "600 ms: one daily step");
    // Exactly 500 ms is not enough.
    f.h.now_ms.set(2100);
    f.sys.on_ticker();
    assert_eq!(f.sys.state().idle_start, Some(1600));
    // Not paused: nothing.
    f.h.paused.set(false);
    f.h.now_ms.set(9000);
    f.sys.on_ticker();
    assert_eq!(f.sys.state().idle_start, Some(1600));
}

#[test]
fn ticker_reload_pending_reloads_before_the_daily_step() {
    let f = F::plain(vec![]);
    f.sys.set_reload_pending();
    f.log();
    f.h.set_day(1000);
    f.sys.on_ticker();
    let log = f.log();
    assert!(log.contains(&HostCall::Lua("reload_scripts".into())));
    assert!(!f.sys.state().reload_pending);
}

// ================================================================ §17.10

#[test]
fn t10_mayor_rating() {
    let f = F::new(clsid::CITY_PLANNING, vec![]);
    f.h.set_value(Src::new(SrcKind::MayorRating), Val::int(-5));
    f.h.set_value(
        Src::new(SrcKind::MinMayorRating),
        Val {
            i: -20,
            cell: Some((14.0, 22.0)),
            ..Val::default()
        },
    );
    f.h.set_value(
        Src::new(SrcKind::MaxMayorRating),
        Val {
            i: 40,
            cell: Some((3.0, 5.0)),
            ..Val::default()
        },
    );
    f.adv().update_variables();
    assert_eq!(f.h.global("ga_mayor_rating"), Some(-5.0));
    let (v, s) = f.h.local("l_mayor_rating_l").unwrap();
    assert_eq!(v, -20.0);
    assert_eq!(s.unwrap().location, Some((14.0, 22.0)));
    let (v, s) = f.h.local("l_mayor_rating_h").unwrap();
    assert_eq!(v, 40.0);
    assert_eq!(s.unwrap().location, Some((3.0, 5.0)));
    // Missing sources: not written.
    assert_eq!(f.h.global("g_tax_rate_neutral"), None);
}

#[test]
fn city_planning_populations_and_extrapolation() {
    let f = F::new(clsid::CITY_PLANNING, vec![]);
    let h = &f.h;
    h.set_value(Src::ab(SrcKind::DemandSupply, 0x1001, 0), Val::float(0.1));
    h.set_value(Src::ab(SrcKind::DemandSupply, 0x3001, 0), Val::float(0.2));
    h.set_value(Src::ab(SrcKind::DemandSupply, 0x4001, 0), Val::float(0.3));
    h.set_value(Src::a(SrcKind::ExtrapolatedPopulation, 0x1010), Val::int(500));
    h.set_value(Src::a(SrcKind::RegionPopulation, 0x1010), Val::int(200));
    h.set_value(Src::a(SrcKind::ExtrapolatedPopulation, 0x1020), Val::int(10));
    h.set_value(Src::a(SrcKind::RegionPopulation, 0x1020), Val::int(30));
    h.set_value(Src::new(SrcKind::CitySizeX), Val::float(1024.0));
    h.set_value(Src::new(SrcKind::CitySizeZ), Val::float(1024.0));
    h.set_value(Src::ab(SrcKind::DemandCap, 0x4200, variables::REGION), Val::float(0.5));
    f.adv().update_variables();
    let rci = ((0.3f32 + 0.2f32) + 0.1f32) as f64;
    assert_eq!(h.global("g_city_rci_population"), Some(rci));
    assert_eq!(h.global("g_city_r_population"), Some(f64::from(0.1f32)));
    assert_eq!(h.global("g_r1_demand_extrap"), Some(300.0));
    assert_eq!(h.global("g_r2_demand_extrap"), Some(-20.0));
    assert_eq!(h.global("g_r_demand_extrap"), Some(280.0));
    assert_eq!(h.global("g_c_demand_extrap"), None);
    assert_eq!(h.global("g_city_tile_size"), Some(1_048_576.0));
    assert_eq!(h.global("g_current_id_cap"), Some(50.0));
    // The occupant variables follow.
    assert_eq!(h.global("g_num_buildings"), Some(0.0));
}

// ================================================================ §17.11

#[test]
fn t11_subject_clamp() {
    let f = F::plain(vec![]);
    let s = f.sys.new_subject();
    assert!(!s.has_location());
    assert_eq!(s.location_x(), 0.0);
    s.set_location(70.0, -3.0);
    assert_eq!((s.location_x(), s.location_z()), (64.0, 0.0));
    assert!(s.has_location());
}

#[test]
fn subject_object_refcounts_and_clone() {
    let f = F::plain(vec![]);
    let s = f.sys.new_subject();
    s.set_object(Some(0x99));
    assert_eq!(f.h.refs.borrow()[&0x99], 1);
    let c = s.clone_subject();
    assert_eq!(f.h.refs.borrow()[&0x99], 2);
    assert_eq!(c.state(), s.state());
    assert!(c.serial() != s.serial());
    s.set_object(Some(0x98));
    assert_eq!(f.h.refs.borrow()[&0x99], 1);
    drop(c);
    drop(s);
    assert_eq!(f.h.refs.borrow()[&0x99], 0);
    assert_eq!(f.h.refs.borrow()[&0x98], 0);
}

// ================================================================ §17.12

#[test]
fn t12_reward_condition() {
    let f = F::new(
        clsid::CITY_PLANNING,
        vec![AdviceEntry {
            class_id: clsid::REWARD_ADVICE,
            ..ae(0xAB, "x")
        }],
    );
    let a = f.advice(0xAB);
    assert_eq!(a.kind(), AdviceKind::Reward);
    assert!(a.check_flags(flags::REWARD));
    f.h.reward_subjects.borrow_mut().insert(
        0xAB,
        SubjectInit {
            location: Some((5.0, 6.0)),
            ..SubjectInit::default()
        },
    );
    // 1 with status 0 → 6, subject set, trigger true.
    f.h.conditions.borrow_mut().insert(1, LuaResult::Number(1.0));
    assert!(a.get_trigger());
    assert_eq!(f.h.statuses.borrow()[&0xAB].status, 6);
    assert_eq!(a.event_subject().unwrap().state().location, Some((5.0, 6.0)));
    // 1 again with bit 4: no new trigger.
    assert!(!a.get_trigger());
    // 0 with status 6 → 5.
    f.h.conditions.borrow_mut().insert(1, LuaResult::Number(0.0));
    a.get_trigger();
    assert_eq!(f.h.statuses.borrow()[&0xAB].status, 5);
    // A string with status 6 → 4 and the tooltip.
    f.h.statuses.borrow_mut().get_mut(&0xAB).unwrap().status = 6;
    f.h.conditions
        .borrow_mut()
        .insert(1, LuaResult::Str(b"need 5k".to_vec()));
    assert!(!a.get_trigger());
    let st = f.h.statuses.borrow()[&0xAB].clone();
    assert_eq!(st.status, 4);
    assert_eq!(st.tooltip, b"detok:need 5k");
    // Anything else: the status is written back unchanged.
    f.h.conditions.borrow_mut().insert(1, LuaResult::Other);
    a.get_trigger();
    assert_eq!(f.h.statuses.borrow()[&0xAB].status, 4);
}

#[test]
fn update_self_expires_unavailable_reward() {
    let f = F::new(
        clsid::CITY_PLANNING,
        vec![AdviceEntry {
            class_id: clsid::REWARD_ADVICE,
            ..ae(0xAB, "x")
        }],
    );
    let adv = f.adv();
    adv.with_state(|s| s.dirty = false);
    let a = f.advice(0xAB);
    f.h.conditions.borrow_mut().insert(1, LuaResult::Number(0.0));
    let mut m = AdvisorMessage::of_type(msg::UPDATE_SELF);
    m.data1 = 1;
    m.data2 = 0xAB;
    adv.do_message(&m);
    assert!(a.check_flags(flags::EXPIRE_REQUEST));
    assert!(adv.state().dirty);
}

#[test]
fn conditional_building_insert_posts_update_self() {
    let f = F::new(clsid::FINANCE, vec![]);
    f.h.occupants.borrow_mut().insert(
        7,
        Occupant {
            occupant_type: OCCUPANT_TYPE_BUILDING,
            conditional: Some(true),
            key_instance: Some(0xAB),
            groups: vec![],
        },
    );
    f.log();
    let m = AdvisorMessage {
        void1: Some(7),
        ..AdvisorMessage::of_type(msg::INSERT_OCCUPANT)
    };
    f.adv().do_message(&m);
    let log = f.log();
    // Finance first posts UpdateSelf(0, 0) (pending), then the base (1, key).
    assert_eq!(
        log,
        vec![
            HostCall::UpdateSelf {
                advisor: ID,
                data1: 0,
                data2: 0
            },
            HostCall::UpdateSelf {
                advisor: ID,
                data1: 1,
                data2: 0xAB
            }
        ]
    );
    // A second insert does not post (0, 0) again.
    f.adv().do_message(&m);
    assert_eq!(f.log().len(), 1);
}

#[test]
fn ordinance_update_and_post_city_init_complete() {
    let f = F::new(
        clsid::FINANCE,
        vec![AdviceEntry {
            class_id: clsid::REWARD_ADVICE,
            ..ae(0xAB, "x")
        }],
    );
    f.log();
    f.adv().do_message(&AdvisorMessage::of_type(msg::ORDINANCE_UPDATED));
    assert_eq!(
        f.log(),
        vec![HostCall::UpdateSelf {
            advisor: ID,
            data1: 2,
            data2: 0
        }]
    );
    f.adv()
        .do_message(&AdvisorMessage::of_type(msg::POST_CITY_INIT_COMPLETE));
    // The reward trigger ran (status written).
    assert!(f.log().iter().any(|c| matches!(c, HostCall::Status { guid: 0xAB, .. })));
}

// ================================================================ §17.13

struct Store {
    reg: Rc<RefCell<SaveRegistry>>,
    /// (class, id) → record bytes.
    records: BTreeMap<(u32, u32), Vec<u8>>,
}

/// Saves `root` and everything it references.
fn save(root: RefValue) -> (Store, Vec<u8>) {
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(reg.clone());
    w.put_ref(&root);
    let root_bytes = w.into_bytes();
    let mut records = BTreeMap::new();
    let mut done = 0;
    loop {
        let next = reg.borrow().entries.get(done).cloned();
        let Some((class, id, obj, _)) = next else { break };
        done += 1;
        if let Some(o) = obj {
            let mut w = ByteWriter::with_registry(reg.clone());
            assert!(o.write(&mut w));
            records.insert((class, id), w.into_bytes());
        }
    }
    (Store { reg, records }, root_bytes)
}

struct Loader<'a> {
    sys: &'a Rc<AdvisorSystem>,
    made: BTreeMap<(u32, u32), RefValue>,
}

impl RefResolver for Loader<'_> {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        if let Some(r) = self.made.get(&(class_id, object_id)) {
            return Some(r.clone());
        }
        let r = self.sys.create_object(class_id).unwrap_or(RefValue::Host {
            token: u64::from(object_id),
            class_id,
        });
        self.made.insert((class_id, object_id), r.clone());
        Some(r)
    }
}

/// Loads a store into `sys`: creates every object, reads every record.
fn load(sys: &Rc<AdvisorSystem>, st: &Store, root_bytes: &[u8]) -> RefValue {
    let mut l = Loader {
        sys,
        made: BTreeMap::new(),
    };
    for (class, id) in st.records.keys() {
        l.resolve(*class, *id);
    }
    for ((class, id), bytes) in &st.records {
        let obj = l.made[&(*class, *id)].clone();
        if let RefValue::Object(o) = obj {
            let mut r = ByteReader::with_resolver(bytes, &mut l);
            assert!(o.read(&mut r), "read {class:#x}");
        }
    }
    let mut r = ByteReader::with_resolver(root_bytes, &mut l);
    crate::stream::InStream::get_ref(&mut r).unwrap()
}

struct B(Vec<u8>);
impl B {
    fn u16(mut self, v: u16) -> B {
        self.0.extend(v.to_le_bytes());
        self
    }
    fn i32(mut self, v: i32) -> B {
        self.0.extend(v.to_le_bytes());
        self
    }
    fn u32(mut self, v: u32) -> B {
        self.0.extend(v.to_le_bytes());
        self
    }
    fn i8(mut self, v: i8) -> B {
        self.0.push(v as u8);
        self
    }
    fn f32(mut self, v: f32) -> B {
        self.0.extend(v.to_le_bytes());
        self
    }
    fn str(self, s: &[u8]) -> B {
        let mut b = self.u32(s.len() as u32);
        b.0.extend_from_slice(s);
        b
    }
    fn rf(self, id: u32, class: u32) -> B {
        self.u32(id).u32(class)
    }
}

#[test]
fn t13_persistence_round_trip() {
    let f = F::plain(vec![
        AdviceEntry {
            frequency: 10,
            timeout: 45,
            ..ae(0x100, "true")
        },
        AdviceEntry {
            event: 0x77,
            ..ae(0x200, "false")
        },
    ]);
    let adv = f.adv();
    adv.with_state(|s| {
        s.month_counter = 5;
        s.current_day = 1000;
    });
    f.day(1000);
    adv.with_state(|s| s.infos[0].last_month = 5);
    let a = f.advice(0x100);
    assert!(a.is_active());
    assert_eq!(f.info(0x100).expiry, 1045);
    assert_eq!(f.info(0x100).next_day, 0);
    let subj = f.sys.new_subject();
    adv.push_event(0x77, Some(subj));
    let (st, _) = save(RefValue::Object(adv.clone()));
    // The advisor is object 1; advices 2 and 3; the subject 4.
    let rec = &st.records[&(clsid::ADVISOR, 1)];
    let want = B(Vec::new())
        .u16(3)
        .i32(1)
        .str(b"Planning")
        .str(b"Bob")
        .i32(5)
        .i32(1000)
        .i32(ID)
        .i32(1)
        .u32(2)
        .rf(2, clsid::ADVICE)
        .i32(1045)
        .i32(5)
        .i32(0)
        .i8(1)
        .i32(0)
        .rf(3, clsid::ADVICE)
        .i32(0)
        .i32(-1)
        .i32(0)
        .i8(0)
        .i32(0x77)
        .u32(1)
        .i32(0x77)
        .rf(4, clsid::SUBJECT)
        .0;
    assert_eq!(rec, &want);
    let _ = &st.reg;
    // Load into a fresh system, then Init.
    let h2 = ScriptedHost::new();
    *h2.advisors.borrow_mut() = f.h.advisors.borrow().clone();
    *h2.advices.borrow_mut() = f.h.advices.borrow().clone();
    let sys2 = AdvisorSystem::new(h2.clone(), 0);
    let root_bytes = {
        let reg = st.reg.clone();
        let mut w = ByteWriter::with_registry(reg);
        w.put_ref(&RefValue::Object(adv.clone()));
        w.into_bytes()
    };
    let r = load(&sys2, &st, &root_bytes);
    let adv2 = r.object::<Advisor>().unwrap();
    assert_eq!(adv2.current_advices().len(), 1);
    assert_eq!(adv2.state().queue.len(), 1);
    assert_eq!(adv2.state().events.get(&0x77), Some(&vec![1]));
    sys2.add_advisor(Some(adv2.clone()));
    assert!(adv2.init(1));
    assert!(adv2.current_advices().is_empty());
    let a2 = adv2.advice_by_guid(0x100).unwrap();
    assert!(!a2.is_active());
    assert_eq!(adv2.state().infos[0].expiry, 1045);
    // The loaded advice kept its record and was re-initialised in place.
    assert_eq!(a2.state().guid, 0x100);
    assert!(!a2.state().loaded);
}

// ================================================================ §17.14, §17.15

fn advice_record(version: u16, effects: Option<&[u32]>) -> Vec<u8> {
    let mut b = B(Vec::new())
        .u16(version)
        .i32(3)
        .u32(0xABC)
        .u32(flags::ACTIVE)
        .i32(ID)
        .i32(10)
        .i32(20)
        .i32(30)
        .i32(-1);
    if let Some(e) = effects {
        b = b.u32(e.len() as u32);
        for x in e {
            b = b.u32(*x);
        }
    }
    b.u32(0).0
}

#[test]
fn t14_advice_record_versions() {
    let f = F::plain(vec![]);
    let a = f.sys.create_advice(clsid::ADVICE).unwrap();
    let mut l = Loader {
        sys: &f.sys,
        made: BTreeMap::new(),
    };
    let rec = advice_record(3, None);
    assert!(a.read(&mut ByteReader::with_resolver(&rec, &mut l)));
    let s = a.state();
    assert_eq!((s.guid, s.mood, s.priority), (0xABC, -1, 10));
    assert!(s.effects.is_empty());
    assert!(s.subject.is_none());
    assert!(s.loaded);
    assert!(a.check_flags(flags::TEXT_DIRTY | flags::ACTIVE));
    // Version 4: effects appended to the current list [quirk].
    let rec = advice_record(4, Some(&[7, 8]));
    assert!(a.read(&mut ByteReader::with_resolver(&rec, &mut l)));
    let rec = advice_record(4, Some(&[9]));
    assert!(a.read(&mut ByteReader::with_resolver(&rec, &mut l)));
    assert_eq!(a.state().effects, vec![7, 8, 9]);
    // A truncated record fails.
    let rec = advice_record(4, Some(&[9]));
    assert!(!a.read(&mut ByteReader::with_resolver(&rec[..rec.len() - 2], &mut l)));
}

#[test]
fn neighbor_deal_advice_record() {
    let f = F::plain(vec![]);
    let a = f.sys.create_advice(clsid::NEIGHBOR_DEAL_ADVICE).unwrap();
    a.with_state(|s| {
        s.guid = 0x55;
        s.nd_type = 0x4773_FFE2u32 as i32;
    });
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(reg);
    assert!(a.write(&mut w));
    let bytes = w.into_bytes();
    assert_eq!(&bytes[..4], &[2, 0, 4, 0]);
    assert_eq!(&bytes[bytes.len() - 4..], &0x4773_FFE2u32.to_le_bytes());
    let b = f.sys.create_advice(clsid::NEIGHBOR_DEAL_ADVICE).unwrap();
    let mut l = Loader {
        sys: &f.sys,
        made: BTreeMap::new(),
    };
    assert!(b.read(&mut ByteReader::with_resolver(&bytes, &mut l)));
    assert_eq!(b.nd_type(), 0x4773_FFE2u32 as i32);
    let mut bad = bytes.clone();
    bad[0] = 1;
    assert!(!b.read(&mut ByteReader::with_resolver(&bad, &mut l)));
    assert!(b.query_interface(iid::NEIGHBOR_DEAL_ADVICE));
    assert!(!a.clone_advice().is_active());
}

#[test]
fn t15_subject_record() {
    let f = F::plain(vec![]);
    let s = f.sys.new_subject();
    s.set_data1(5);
    s.set_data2(6);
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(reg.clone());
    assert!(s.write(&mut w));
    let want = B(Vec::new()).u16(4).i8(0).u32(5).u32(6).u32(0).0;
    assert_eq!(w.into_bytes(), want);
    s.set_location(1.5, 2.5);
    s.set_object(Some(0x42));
    let mut w = ByteWriter::with_registry(reg);
    assert!(s.write(&mut w));
    let bytes = w.into_bytes();
    let want = B(Vec::new())
        .u16(4)
        .i8(1)
        .f32(1.5)
        .f32(2.5)
        .u32(5)
        .u32(6)
        .rf(1, 0x1234_5678)
        .0;
    assert_eq!(bytes, want);
    let t = f.sys.new_subject();
    let mut l = Loader {
        sys: &f.sys,
        made: BTreeMap::new(),
    };
    assert!(t.read(&mut ByteReader::with_resolver(&bytes, &mut l)));
    assert_eq!(t.state().location, Some((1.5, 2.5)));
    assert_eq!(t.object(), Some(1));
    let mut v3 = bytes.clone();
    v3[0] = 3;
    assert!(!t.read(&mut ByteReader::with_resolver(&v3, &mut l)));
}

// ================================================================ §17.16

#[test]
fn t16_system_record() {
    let h = ScriptedHost::new();
    *h.advisors.borrow_mut() = Some(vec![
        advisor_entry(clsid::ADVISOR, 26),
        advisor_entry(clsid::ADVISOR, 23),
    ]);
    let sys = AdvisorSystem::new(h.clone(), 0);
    sys.init();
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(reg);
    assert!(sys.write(&mut w));
    let want = B(Vec::new())
        .u16(1)
        .u32(2)
        .rf(1, clsid::ADVISOR)
        .i32(23)
        .rf(2, clsid::ADVISOR)
        .i32(26)
        .0;
    assert_eq!(w.into_bytes(), want);
    // Round trip: loadedFromSave, then Init does not reload the table.
    let (st, root) = save(RefValue::Object(sys.clone()));
    let h2 = ScriptedHost::new();
    *h2.advisors.borrow_mut() = h.advisors.borrow().clone();
    let sys2 = AdvisorSystem::new(h2.clone(), 0);
    let mut l = Loader {
        sys: &sys2,
        made: BTreeMap::new(),
    };
    // The system object of the store is sys2 itself.
    l.made.insert((clsid::SYSTEM, 1), RefValue::Object(sys2.clone()));
    for ((class, id), bytes) in &st.records {
        let obj = l.resolve(*class, *id).unwrap();
        if let RefValue::Object(o) = obj {
            assert!(o.read(&mut ByteReader::with_resolver(bytes, &mut l)));
        }
    }
    let _ = root;
    assert!(sys2.state().loaded_from_save);
    assert_eq!(sys2.advisors().len(), 2);
    let a23 = sys2.get_advisor(23).unwrap();
    sys2.init();
    assert!(Rc::ptr_eq(&a23, &sys2.get_advisor(23).unwrap()), "kept, not reloaded");
    let mut bad = B(Vec::new()).u16(2).u32(0).0;
    assert!(!sys2.read(&mut ByteReader::with_resolver(&bad, &mut l)));
    bad[0] = 1;
    assert!(sys2.read(&mut ByteReader::with_resolver(&bad, &mut l)));
}

// ================================================================ §17.17

#[test]
fn t17_city_planning_record() {
    let f = F::new(clsid::CITY_PLANNING, vec![]);
    let adv = f.adv();
    adv.with_state(|s| {
        s.building_count = 10;
        s.park_count = 2;
        s.recreation_count = 2;
        s.landmarks.insert(0x1234, 1);
    });
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(reg);
    assert!(adv.write(&mut w));
    let bytes = w.into_bytes();
    let head = B(Vec::new())
        .u16(3)
        .u16(3)
        .i32(1)
        .str(b"Planning")
        .str(b"Bob")
        .i32(0)
        .i32(0)
        .i32(ID)
        .i32(0)
        .u32(0)
        .u32(0)
        .0;
    let tail = B(Vec::new()).i32(10).i32(2).i32(2).u32(1).i32(0x1234).i32(1).0;
    assert_eq!(bytes, [head.clone(), tail].concat());
    assert_eq!(adv.class_id(), clsid::CITY_PLANNING);
    // Version 2: the counters are read; the landmark map is kept.
    let other = f.sys.create_advisor(clsid::CITY_PLANNING).unwrap();
    other.with_state(|s| {
        s.landmarks.insert(0x99, 4);
    });
    let mut v2 = head.clone();
    v2[0] = 2;
    v2.extend(B(Vec::new()).i32(7).i32(1).i32(1).0);
    let mut l = Loader {
        sys: &f.sys,
        made: BTreeMap::new(),
    };
    assert!(other.read(&mut ByteReader::with_resolver(&v2, &mut l)));
    let s = other.state();
    assert_eq!((s.building_count, s.park_count, s.recreation_count), (7, 1, 1));
    assert_eq!(s.landmarks.get(&0x99), Some(&4));
    // Version 3 replaces it.
    assert!(other.read(&mut ByteReader::with_resolver(&bytes, &mut l)));
    assert_eq!(other.state().landmarks, BTreeMap::from([(0x1234, 1)]));
}

#[test]
fn base_class_advisors_save_as_plain_and_load_both_classes() {
    let f = F::new(clsid::ADVISOR_BASE, vec![]);
    let adv = f.adv();
    assert_eq!(adv.kind(), AdvisorKind::Base);
    assert_eq!(adv.class_id(), clsid::ADVISOR);
    for c in [clsid::ADVISOR, clsid::ADVISOR_BASE] {
        let r = f.sys.create_object(c).unwrap();
        assert_eq!(r.object::<Advisor>().unwrap().kind(), AdvisorKind::Plain);
    }
}

// ================================================================ §17.18

#[test]
fn t18_random_chance() {
    let f = F::plain(vec![]);
    let set = || f.sys.with_random(|g| g.set_state(g.lcg_state(), 1, None));
    set();
    assert_eq!(
        f.sys.call_lua("random_chance", &[LuaValue::Number(70.0)]),
        Some(vec![LuaValue::Number(1.0)])
    );
    assert_eq!(f.sys.with_random(|g| g.mul_state()), 0x278D_DE6D);
    set();
    assert_eq!(
        f.sys.call_lua("random_chance", &[LuaValue::Number(60.0)]),
        Some(vec![LuaValue::Nil])
    );
    set();
    let r = f.sys.with_random(|g| g.range_f64(0.0, 100.0));
    assert_eq!(r as f32, "65.450844".parse::<f32>().unwrap());
}

// ================================================================ §17.19

#[test]
fn t19_utility_counts() {
    let f = F::new(clsid::UTILITY, vec![]);
    f.h.set_value(Src::new(SrcKind::IncineratorCapacity), Val::int(1000));
    f.h.set_value(Src::a(SrcKind::RegionExported, 3), Val::float(5.0));
    f.h.set_value(Src::a(SrcKind::RegionImported, 3), Val::float(2.0));
    f.h.set_value(
        Src::new(SrcKind::LandfillUsage),
        Val {
            i: 30,
            n: 100,
            ..Val::default()
        },
    );
    f.h.set_value(Src::new(SrcKind::AirPollution), Val::int(-4));
    f.adv().update_variables();
    assert_eq!(f.h.global("g_incinerator_capacity_daily"), Some(30000.0));
    assert_eq!(f.h.global("g_garbage_imported"), Some(5.0));
    assert_eq!(f.h.global("g_garbage_exported"), Some(2.0));
    assert_eq!(f.h.global("g_landfill_capacity"), Some(100.0));
    assert_eq!(f.h.global("g_available_landfill_capacity"), Some(70.0));
    assert_eq!(f.h.global("ga_air_pollution"), Some(0.0));
}

#[test]
fn utility_deal_variables() {
    let f = F::new(clsid::UTILITY, vec![]);
    f.h.lists.borrow_mut().insert(
        ListSrc {
            kind: Some(ListKind::Connections),
            a: 0,
            groups: vec![],
        },
        vec![Item {
            kind: 1,
            ..Item::default()
        }],
    );
    f.h.lists.borrow_mut().insert(
        ListSrc {
            kind: Some(ListKind::DealProposals),
            a: 0,
            groups: vec![],
        },
        vec![Item {
            kind: 0x4773_FFE0,
            ..Item::default()
        }],
    );
    f.h.set_value(Src::a(SrcKind::DealCount, 0x4773_FFE2), Val::int(2));
    f.adv().update_variables();
    assert_eq!(f.h.global("g_nd_connection_present_garbage"), Some(1.0));
    assert_eq!(f.h.global("g_nd_connection_present_power"), Some(0.0));
    assert_eq!(f.h.global("g_nd_can_buy_power"), Some(1.0));
    assert_eq!(f.h.global("g_nd_can_sell_power"), Some(0.0));
    assert_eq!(f.h.global("g_nd_count_buy_water"), Some(2.0));
}

#[test]
fn utility_deal_events_match_type_and_clone() {
    let f = F::new(
        clsid::UTILITY,
        vec![AdviceEntry {
            class_id: clsid::NEIGHBOR_DEAL_ADVICE,
            event: event::DEAL_EVENTS[0],
            nd_type: 0x4773_FFE2u32 as i32,
            timeout: 100,
            ..ae(0x60, "true")
        }],
    );
    let adv = f.adv();
    f.h.deals.borrow_mut().insert(0xD1, 0x4773_FFE2);
    f.h.deals.borrow_mut().insert(0xD2, 0x4773_FFE2);
    f.h.deals.borrow_mut().insert(0xD3, 0x4773_FFE0);
    let report = |deal: u64| {
        adv.do_message(&AdvisorMessage {
            data1: 3,
            unknown: Some(deal),
            ..AdvisorMessage::of_type(msg::DEAL_STATUS)
        })
    };
    report(0xD3);
    f.day(1000);
    let a = f.advice(0x60);
    assert!(!a.is_active(), "other deal type");
    report(0xD1);
    f.day(1001);
    assert!(a.is_active());
    assert_eq!(a.event_subject().unwrap().object(), Some(0xD1));
    // Another deal of the same type: a clone.
    report(0xD2);
    f.day(1002);
    let infos = adv.state().infos;
    assert_eq!(infos.len(), 2);
    let c = infos[1].advice.clone();
    assert!(c.is_active());
    assert_eq!(c.event_subject().unwrap().object(), Some(0xD2));
    assert_eq!(adv.state().events[&event::DEAL_EVENTS[0]], vec![0, 1]);
}

#[test]
fn utility_power_strike_and_pending() {
    let f = F::new(clsid::UTILITY, vec![]);
    f.log();
    f.adv().do_message(&AdvisorMessage::of_type(msg::POWER_STRIKE_BEGIN));
    assert_eq!(f.h.global("g_power_strike"), Some(1.0));
    // A network insert marks bit 2 and posts UpdateSelf(0,0).
    f.h.occupants.borrow_mut().insert(
        3,
        Occupant {
            occupant_type: OCCUPANT_TYPE_NETWORK,
            ..Occupant::default()
        },
    );
    f.log();
    f.adv().do_message(&AdvisorMessage {
        void1: Some(3),
        ..AdvisorMessage::of_type(msg::INSERT_OCCUPANT)
    });
    assert_eq!(f.adv().state().pending, pending::NETWORKS);
    f.adv().do_message(&AdvisorMessage::of_type(msg::UPDATE_SELF));
    assert_eq!(f.adv().state().pending, 0);
}

// ================================================================ §17.20

#[test]
fn t20_hq_empty_lists() {
    let f = F::new(clsid::HQ, vec![]);
    f.h.lists.borrow_mut().insert(
        ListSrc {
            kind: Some(ListKind::Hospitals),
            a: 0,
            groups: vec![0x1513],
        },
        vec![],
    );
    f.adv().update_variables();
    let (v, s) = f.h.local("l_hospital_funding_l").unwrap();
    assert!(v.is_finite() && (v / 3.402_823_4e40 - 1.0).abs() < 1e-7);
    assert_eq!(v, pct100(f32::MAX));
    assert!(s.is_none());
    assert_eq!(f.h.local("l_hospital_grade_l").unwrap().0, f64::from(f32::MAX));
    assert_eq!(f.h.global("g_num_hospitals"), Some(0.0));
    assert_eq!(f.h.global("g_num_treated_patients"), Some(0.0));
}

#[test]
fn hq_minimum_walks_back_to_front() {
    let f = F::new(clsid::HQ, vec![]);
    let it = |o: u64, f0: f32, f1: f32| Item {
        object: o,
        f0,
        f1,
        ..Item::default()
    };
    f.h.lists.borrow_mut().insert(
        ListSrc {
            kind: Some(ListKind::Schools),
            a: 0,
            groups: vec![0x1506],
        },
        vec![it(1, 0.5, 0.9), it(2, 0.5, 0.2), it(3, 0.7, 0.3)],
    );
    f.h.set_value(
        Src::a(SrcKind::SchoolTotals, 0x1506),
        Val {
            f: 1.5,
            m: 3,
            n: 40,
            ..Val::default()
        },
    );
    f.adv().update_variables();
    let (v, s) = f.h.local("l_museum_funding_l").unwrap();
    assert_eq!(v, 50.0);
    // Back to front with a strict `<`: the tie keeps building 2.
    assert_eq!(s.unwrap().object, Some(2));
    assert_eq!(f.h.local("l_museum_grade_l").unwrap().0, f64::from(20.0f32));
    assert_eq!(f.h.global("ga_museum_grade"), Some(50.0));
    assert_eq!(f.h.global("g_num_museum_served"), Some(40.0));
}

// ================================================================ §17.21

#[test]
fn t21_popup_thresholds() {
    let f = F::plain(vec![]);
    let shipped: BTreeMap<String, f64> = [("low", 60.0), ("medium", 80.0), ("high", 100.0)]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    *f.h.popup.borrow_mut() = Some(shipped);
    f.sys.init_popup_thresholds();
    f.h.difficulty.set(Some(1));
    assert_eq!(f.sys.popup_threshold(), 80);
    f.h.difficulty.set(Some(7));
    assert_eq!(f.sys.popup_threshold(), 60);
    f.h.difficulty.set(Some(2));
    assert_eq!(f.sys.popup_threshold(), 100);
    *f.h.popup.borrow_mut() = None;
    f.sys.init_popup_thresholds();
    f.h.difficulty.set(Some(1));
    assert_eq!(f.sys.popup_threshold(), 100);
}

// ================================================================ §17.22

#[test]
fn t22_lua_functions() {
    let f = F::plain(vec![]);
    let n = |v: f64| LuaValue::Number(v);
    let call = |name: &str, args: &[LuaValue]| f.sys.call_lua(name, args).unwrap();
    assert_eq!(
        call(
            "_combine_guids",
            &[n(f64::from(0x0F0F_0000u32)), n(f64::from(0x00F0_F0F0u32))]
        ),
        vec![n(268_431_600.0)]
    );
    assert_eq!(
        call("_combine_guids", &[n(f64::from(0x8000_0000u32)), n(0.0)]),
        vec![n(-2_147_483_648.0)]
    );
    assert_eq!(call("_combine_guids", &[n(1.0)]), vec![n(-1.0)]);
    f.h.set_day(10_000);
    f.h.history.borrow_mut().insert(0x5, (12.0, 1.0, 3.0));
    let r = call("trend_slope", &[n(5.0), n(2.0)]);
    assert_eq!(r, vec![n(f64::from(49.999_863f32))]);
    assert_eq!(f.h.history_days.borrow()[0], (5, 10_000 - 60, 10_000));
    assert_eq!(call("trend_value", &[n(5.0), n(1.0)]), vec![n(12.0)]);
    assert_eq!(call("trend_count", &[n(5.0), n(1.0)]), vec![n(3.0)]);
    assert_eq!(f.h.history_days.borrow()[2], (5, 10_000 - 30 + 1, 10_000));
    assert_eq!(call("trend_value", &[n(6.0), n(1.0)]), vec![n(0.0)]);
    assert!(f.sys.call_lua("no_such_function", &[]).is_none());
}

#[test]
fn lua_context_functions() {
    let f = F::plain(vec![AdviceEntry {
        event: 0x77,
        ..ae(0x10, "true")
    }]);
    let a = f.advice(0x10);
    let s = f.sys.new_subject();
    s.set_object(Some(0x42));
    s.set_data1(9);
    a.set_event_subject(Some(s.clone()));
    // No context: nothing.
    assert_eq!(f.sys.call_lua("advice_subject", &[]).unwrap(), vec![LuaValue::Nil]);
    f.sys.push_context_advice(a.clone());
    let r = f.sys.call_lua("advice_subject", &[]).unwrap();
    let LuaValue::Subject(Some(c)) = &r[0] else { panic!() };
    assert!(!Rc::ptr_eq(c, &s));
    assert_eq!(c.object(), Some(0x42));
    f.log();
    f.sys.call_lua("event_feedback", &[LuaValue::Number(3.0)]);
    assert_eq!(
        f.log(),
        vec![HostCall::Post {
            msg: 0x77,
            data1: 3,
            obj: "host:0x42".into()
        }]
    );
    assert_eq!(
        f.sys.call_lua("event_disaster_distance", &[]).unwrap(),
        vec![LuaValue::Number(9.0)]
    );
    assert_eq!(
        f.sys.call_lua("event_building_distance", &[]).unwrap(),
        vec![LuaValue::Nil]
    );
    f.sys.call_lua("expire_advice", &[]);
    assert!(a.check_flags(flags::EXPIRE_REQUEST));
    // retire_advice: only an active advice.
    f.adv().activate(0);
    assert!(a.is_active());
    f.sys.call_lua("retire_advice", &[]);
    assert!(!a.is_active());
    f.sys.pop_context_advice();
    assert!(f.sys.context_advice().is_none());
}

#[test]
fn lua_reward_and_ordinance_functions() {
    let f = F::new(clsid::CITY_PLANNING, vec![]);
    let n = |v: f64| LuaValue::Number(v);
    let call = |name: &str, args: &[LuaValue]| f.sys.call_lua(name, args).unwrap();
    assert_eq!(call("get_reward_building_state", &[n(7.0)]), vec![n(1.0)]);
    f.h.statuses.borrow_mut().insert(
        7,
        ConditionalStatus {
            status: 1,
            building_count: 3,
            tooltip: vec![],
        },
    );
    // Neither missing nor bit 2: its argument [quirk].
    assert_eq!(call("get_reward_building_state", &[n(7.0)]), vec![n(7.0)]);
    call("unlock_reward_building", &[n(7.0)]);
    assert_eq!(f.h.statuses.borrow()[&7].status, 2);
    assert_eq!(call("get_reward_building_state", &[n(7.0)]), vec![n(2.0)]);
    assert_eq!(call("reward_instance_count", &[n(7.0)]), vec![n(3.0)]);
    f.adv().with_state(|s| {
        s.landmarks.insert(8, 2);
    });
    assert_eq!(call("reward_instance_count", &[n(8.0)]), vec![n(2.0)]);
    f.h.ordinances.borrow_mut().insert(
        4,
        scripted::Ordinance {
            on: false,
            available: true,
            amounts: [100, 200, -5],
            name: b"Smoke".to_vec(),
            description: b"d".to_vec(),
        },
    );
    assert_eq!(call("ordinance_is_on", &[n(4.0)]), vec![LuaValue::Nil]);
    assert_eq!(call("ordinance_is_available", &[n(4.0)]), vec![n(1.0)]);
    call("ordinance_set_on", &[n(4.0)]);
    assert_eq!(call("ordinance_is_on", &[n(4.0)]), vec![n(1.0)]);
    assert_eq!(call("ordinance_monthly_cost", &[n(4.0)]), vec![n(-5.0)]);
    assert_eq!(
        call("ordinance_name", &[n(4.0)]),
        vec![LuaValue::Str(b"Smoke".to_vec())]
    );
    assert_eq!(call("ordinance_name", &[n(5.0)]), vec![LuaValue::Nil]);
    assert_eq!(call("ordinance_enact_fee", &[n(5.0)]), vec![n(0.0)]);
    f.h.difficulty.set(None);
    assert_eq!(call("difficulty_level", &[]), vec![n(-1.0)]);
    f.h.tutorial.set(Some(0x33));
    assert_eq!(call("is_tutorial", &[n(f64::from(0x33))]), vec![n(1.0)]);
    assert_eq!(call("is_tutorial", &[n(1.0)]), vec![LuaValue::Nil]);
    // trigger_event pushes to the subscribers.
    f.sys.call_lua("trigger_event", &[n(f64::from(0x9))]);
}

// ================================================================ §4, §6.5

#[test]
fn system_init_sequence_and_shutdown() {
    let h = ScriptedHost::new();
    h.debug.set(true);
    *h.advisors.borrow_mut() = Some(vec![
        advisor_entry(clsid::ADVISOR, 0),
        advisor_entry(0x1234_5678, 30),
        advisor_entry(clsid::FINANCE, 23),
    ]);
    let sys = AdvisorSystem::new(h.clone(), 0);
    assert!(sys.init());
    let log = h.take_log();
    assert_eq!(
        &log[..4],
        &[
            HostCall::Lua("register_api(true)".into()),
            HostCall::Lua("create_tags".into()),
            HostCall::Lua("register_detokenizer(true)".into()),
            HostCall::Lua("reload_scripts".into()),
        ]
    );
    assert!(log.contains(&HostCall::Cheat {
        id: cheat::ADV,
        on: true
    }));
    assert_eq!(log.last(), Some(&HostCall::Window { start: true }));
    // ID 0 and unknown classes make no advisor.
    assert_eq!(sys.advisors().len(), 1);
    assert_eq!(sys.data_tag(0), 1);
    assert_eq!(sys.data_tag(1), 2);
    assert_eq!(sys.data_tag(5), 0);
    // A second Init only restarts the window.
    assert!(sys.init());
    assert_eq!(h.take_log(), vec![HostCall::Window { start: true }]);
    assert!(sys.shutdown());
    let log = h.take_log();
    assert!(log.contains(&HostCall::Send {
        msg: msg::ADVISOR_REMOVED_ALL,
        data1: 0,
        obj: "none".into()
    }));
    assert!(log.contains(&HostCall::Lua("close_lua".into())));
    assert!(sys.advisors().is_empty());
    assert!(!sys.state().initialized);
}

#[test]
fn system_without_advisors_table() {
    let h = ScriptedHost::new();
    *h.advisors.borrow_mut() = None;
    let sys = AdvisorSystem::new(h.clone(), 0);
    assert!(!sys.load_advisors());
    assert!(sys.init());
    assert!(sys.advisors().is_empty());
}

#[test]
fn advisor_init_keeps_removes_and_recreates() {
    let f = F::plain(vec![ae(1, "true"), ae(2, "true"), ae(3, "true")]);
    let adv = f.adv();
    assert_eq!(adv.caption(), b"Planning");
    assert_eq!(adv.personal_name(), b"Bob");
    let a1 = f.advice(1);
    let a2 = f.advice(2);
    f.adv().activate(0);
    assert!(a1.is_active());
    // GUID 2 leaves the table; GUID 3 changes class; GUID 4 arrives.
    *f.h.advices.borrow_mut() = vec![
        ae(1, "true"),
        AdviceEntry {
            class_id: clsid::REWARD_ADVICE,
            ..ae(3, "true")
        },
        ae(4, "true"),
        ae(4, "false"),
    ];
    f.log();
    assert!(adv.init(1));
    let guids: Vec<u32> = adv.advices().iter().map(|a| a.guid()).collect();
    assert_eq!(guids, vec![1, 3, 4]);
    assert!(Rc::ptr_eq(&adv.advices()[0], &a1), "kept");
    assert!(!a1.is_active(), "active state cleared");
    assert_eq!(adv.advices()[1].kind(), AdviceKind::Reward, "re-created");
    // The first entry of a GUID wins.
    assert_eq!(adv.advices()[2].table_index(), 3);
    assert_eq!(a2.state().trigger, NO_TRIGGER, "shut down");
    let log = f.log();
    assert_eq!(
        posts(&log, msg::ADVICE_REMOVED_ALL),
        vec![HostCall::Send {
            msg: msg::ADVICE_REMOVED_ALL,
            data1: 0,
            obj: format!("advisor:{ID}")
        }]
    );
    // A wrong ID fails.
    f.h.advisors.borrow_mut().as_mut().unwrap()[0].id = 99;
    assert!(!adv.init(1));
}

#[test]
fn unknown_advice_class_is_skipped() {
    let f = F::plain(vec![AdviceEntry {
        class_id: clsid::MYSIM_ADVICE,
        ..ae(1, "true")
    }]);
    assert!(f.adv().advices().is_empty());
    f.sys.register_advice_class(clsid::MYSIM_ADVICE, AdviceKind::Plain);
    f.adv().init(1);
    assert_eq!(f.adv().advices().len(), 1);
    assert_eq!(f.advice(1).class_id(), clsid::MYSIM_ADVICE);
}

#[test]
fn retire_and_command_would_trigger() {
    let f = F::plain(vec![
        AdviceEntry {
            command: 0x77,
            ..ae(1, "false")
        },
        AdviceEntry {
            command: 0x77,
            ..ae(2, "true")
        },
        AdviceEntry {
            command: 0x78,
            event: 5,
            ..ae(3, "true")
        },
    ]);
    let adv = f.adv();
    assert!(adv.command_would_trigger(0x77));
    assert!(!adv.command_would_trigger(0x78), "event advices are skipped");
    assert!(!adv.command_would_trigger(0x79));
    // Scanned from the last advice: advice 2 answers first.
    f.h.trigger_calls.borrow_mut().clear();
    adv.command_would_trigger(0x77);
    assert_eq!(f.h.trigger_calls.borrow().len(), 1);
    assert!(!adv.retire_advice(None));
    assert!(!adv.retire_advice(Some(&f.advice(1))), "inactive");
    adv.activate(1);
    assert!(adv.retire_advice(Some(&f.advice(2))));
    assert!(adv.current_advices().is_empty());
    assert_eq!(f.info(2).next_day, 35);
    assert_eq!(adv.advice_by_session(f.advice(2).session_id()).unwrap().guid(), 2);
}

#[test]
fn retire_during_the_active_pass_is_skipped() {
    let f = F::plain(vec![ae(1, "selfretire"), ae(2, "true")]);
    let sys = f.sys.clone();
    *f.h.trigger_hook.borrow_mut() = Some(Box::new(move |t| {
        if t == b"selfretire" {
            sys.call_lua("retire_advice", &[]);
        }
    }));
    let adv = f.adv();
    adv.activate(0);
    adv.activate(1);
    f.h.set_day(1);
    adv.simulate(0);
    assert!(!f.advice(1).is_active());
    assert!(f.advice(2).is_active());
    assert_eq!(adv.current_advices().len(), 1);
}

#[test]
fn city_situations_retire_returns_true_when_missing() {
    let f = F::new(clsid::CITY_SITUATIONS, vec![]);
    let stray = f.sys.create_advice(clsid::ADVICE).unwrap();
    stray.with_state(|s| s.flags = flags::ACTIVE);
    assert!(f.adv().retire_advice(Some(&stray)));
    let g = F::plain(vec![]);
    let stray = g.sys.create_advice(clsid::ADVICE).unwrap();
    stray.with_state(|s| s.flags = flags::ACTIVE);
    assert!(!g.adv().retire_advice(Some(&stray)));
}

#[test]
fn advisor_shutdown_clears_everything() {
    let f = F::new(
        clsid::SAFETY,
        vec![AdviceEntry {
            event: 0x77,
            ..ae(1, "true")
        }],
    );
    let adv = f.adv();
    assert_eq!(f.sys.subscribers(0x77).len(), 1);
    adv.with_state(|s| s.month_counter = 4);
    f.log();
    assert!(adv.shutdown());
    let log = f.log();
    assert!(log.contains(&HostCall::Unsubscribe {
        target: format!("advisor:{ID}"),
        msg: msg::HEALTH_STRIKE
    }));
    assert!(log.contains(&HostCall::Unsubscribe {
        target: format!("advisor:{ID}"),
        msg: msg::DEBUG_PRINT
    }));
    assert!(f.sys.subscribers(0x77).is_empty());
    assert!(adv.advices().is_empty());
    assert_eq!(adv.month_counter(), 0);
    assert!(!adv.state().dirty);
}

#[test]
fn subclass_init_subscribes_once_and_sets_dirty() {
    let f = F::new(clsid::TRAFFIC, vec![]);
    let adv = f.adv();
    adv.with_state(|s| s.dirty = false);
    f.log();
    adv.init(1);
    assert!(adv.state().dirty);
    assert!(!f.log().iter().any(|c| matches!(c, HostCall::Subscribe { .. })));
    // The first daily step publishes the variables and clears dirty.
    f.h.set_value(Src::new(SrcKind::PotholeCount), Val::int(4));
    f.day(100);
    assert_eq!(f.h.global("g_pothole_count"), Some(4.0));
    assert!(!adv.state().dirty);
}

#[test]
fn monthly_step_updates_base_family_only() {
    let f = F::new(clsid::FINANCE, vec![]);
    f.h.set_value(Src::new(SrcKind::TotalFunds), Val::int(-123_456_789_012));
    f.adv().simulate(1);
    assert_eq!(f.h.global("g_funds"), Some(-123_456_789_012.0));
    assert_eq!(f.adv().month_counter(), 1);
    assert!(f.adv().simulate(9));
}

// ================================================================ §5.5 subclasses

#[test]
fn city_planning_occupant_counts() {
    let f = F::new(clsid::CITY_PLANNING, vec![]);
    let adv = f.adv();
    let mut occ = f.h.occupants.borrow_mut();
    occ.insert(
        1,
        Occupant {
            occupant_type: OCCUPANT_TYPE_BUILDING,
            groups: vec![GROUP_PARK],
            ..Occupant::default()
        },
    );
    occ.insert(
        2,
        Occupant {
            occupant_type: OCCUPANT_TYPE_BUILDING,
            groups: vec![GROUP_LANDMARK],
            key_instance: Some(0x1234),
            ..Occupant::default()
        },
    );
    occ.insert(
        3,
        Occupant {
            occupant_type: OCCUPANT_TYPE_NETWORK,
            ..Occupant::default()
        },
    );
    drop(occ);
    let m = |t: u32, o: u64| AdvisorMessage {
        void1: Some(o),
        ..AdvisorMessage::of_type(t)
    };
    adv.do_message(&m(msg::INSERT_OCCUPANT, 1));
    adv.do_message(&m(msg::INSERT_OCCUPANT, 2));
    adv.do_message(&m(msg::INSERT_OCCUPANT, 2));
    adv.do_message(&m(msg::REMOVE_OCCUPANT, 2));
    f.log();
    adv.do_message(&m(msg::INSERT_OCCUPANT, 3));
    assert!(f.log().is_empty(), "non-buildings return early");
    let s = adv.state();
    assert_eq!((s.building_count, s.park_count, s.recreation_count), (2, 1, 1));
    assert_eq!(s.landmarks.get(&0x1234), Some(&0));
    adv.do_message(&AdvisorMessage::of_type(msg::UPDATE_SELF));
    assert_eq!(f.h.global("g_num_buildings"), Some(2.0));
    assert_eq!(f.h.global("g_num_recreation"), Some(1.0));
}

#[test]
fn traffic_zots() {
    let f = F::new(clsid::TRAFFIC, vec![]);
    let m = |d1: u32, lot: u64| AdvisorMessage {
        data1: d1,
        unknown: Some(lot),
        ..AdvisorMessage::of_type(msg::LOT_DEVELOPER_NOTIFICATION)
    };
    f.adv().do_message(&m(1, 10));
    f.adv().do_message(&m(1, 11));
    f.adv().do_message(&m(1, 11));
    assert_eq!(f.h.global("g_car_zot_count"), Some(2.0));
    f.adv().do_message(&m(2, 10));
    assert_eq!(f.h.global("g_car_zot_count"), Some(1.0));
}

#[test]
fn traffic_connection_maxima_keep_the_road_location() {
    let f = F::new(clsid::TRAFFIC, vec![]);
    let conns = |t: u32, cells: &[(f32, f32)]| {
        f.h.lists.borrow_mut().insert(
            ListSrc {
                kind: Some(ListKind::Connections),
                a: t,
                groups: vec![],
            },
            cells
                .iter()
                .map(|&(x, z)| Item {
                    x,
                    z,
                    ..Item::default()
                })
                .collect(),
        );
    };
    conns(2, &[(1.0, 2.0), (3.0, 4.0)]);
    conns(8, &[(5.0, 6.0)]);
    f.h.set_value(Src::ab(SrcKind::CongestionAt, 1, 2), Val::int(10));
    f.h.set_value(Src::ab(SrcKind::CongestionAt, 3, 4), Val::int(30));
    f.h.set_value(Src::ab(SrcKind::CongestionAt, 5, 6), Val::int(0));
    f.adv().update_variables();
    let (v, s) = f.h.local("l_road_connection_congestion_h").unwrap();
    assert_eq!(v, 30.0);
    assert_eq!(s.unwrap().location, Some((3.0, 4.0)));
    let (v, s) = f.h.local("l_avenue_connection_congestion_h").unwrap();
    assert_eq!(v, 0.0);
    assert_eq!(s.unwrap().location, Some((3.0, 4.0)), "[quirk]");
    // Port efficiency ×100.
    f.h.set_value(Src::a(SrcKind::PortEfficiency, 0), Val::float(1.0));
    f.adv().update_variables();
    assert_eq!(f.h.global("ga_airport_efficiency"), Some(100.0));
}

#[test]
fn safety_variables_and_crime_hotspot_fallback() {
    let f = F::new(clsid::SAFETY, vec![]);
    f.h.set_value(
        Src::new(SrcKind::FlammabilityHotspot),
        Val {
            i: 77,
            cell: Some((8.0, 9.0)),
            ..Val::default()
        },
    );
    f.h.set_value(Src::new(SrcKind::AverageCrime), Val::int(0x1FF));
    f.h.set_value(Src::new(SrcKind::FireStrikeChance), Val::float(0.25));
    f.log();
    f.adv().update_variables();
    assert_eq!(f.h.global("ga_crime"), Some(255.0));
    let (v, s) = f.h.local("l_crime_h").unwrap();
    assert_eq!(v, 77.0);
    assert_eq!(s.unwrap().location, Some((8.0, 9.0)));
    assert!(f.log().contains(&HostCall::History {
        history_type: variables::HISTORY_GA_CRIME,
        value: 255
    }));
    assert_eq!(f.h.global("g_fire_strike_chance"), Some(25.0));
}

#[test]
fn safety_stations() {
    let f = F::new(clsid::SAFETY, vec![]);
    f.h.lists.borrow_mut().insert(
        ListSrc {
            kind: Some(ListKind::FireStations),
            a: 0,
            groups: vec![],
        },
        vec![],
    );
    f.h.lists.borrow_mut().insert(
        ListSrc {
            kind: Some(ListKind::PoliceStations),
            a: 0,
            groups: vec![],
        },
        vec![
            Item {
                object: 5,
                f0: 0.8,
                flags: ITEM_LOT_EXISTS | ITEM_ROAD_ACCESS,
                ..Item::default()
            },
            Item {
                object: 6,
                f0: 0.8,
                flags: ITEM_LOT_EXISTS,
                ..Item::default()
            },
        ],
    );
    f.adv().update_variables();
    let (v, s) = f.h.local("l_fire_funding_p").unwrap();
    assert_eq!(v, 10000.0);
    assert!(s.is_none());
    assert_eq!(f.h.local("l_fire_station_no_roads").unwrap().0, 0.0);
    let (v, s) = f.h.local("l_police_funding_p").unwrap();
    assert_eq!(v, f64::from(80.0f32) + (pct100(0.8) - 80.0));
    assert_eq!(s.unwrap().object, Some(5), "front to back, strict <");
    let (v, s) = f.h.local("l_police_no_roads").unwrap();
    assert_eq!((v, s.unwrap().object), (1.0, Some(6)));
}

#[test]
fn safety_disasters_and_strikes() {
    let f = F::new(
        clsid::SAFETY,
        vec![
            AdviceEntry {
                event: event::DISASTER_BASE ^ disaster::FIRE,
                ..ae(1, "true")
            },
            AdviceEntry {
                event: event::USER_DISASTER_BASE ^ 0x1234,
                ..ae(2, "true")
            },
            AdviceEntry {
                event: event::HEALTH_STRIKE_BEGIN,
                ..ae(3, "true")
            },
        ],
    );
    let adv = f.adv();
    f.h.set_value(
        Src {
            object: 0x50,
            ..Src::new(SrcKind::DisasterFireCause)
        },
        Val::int(2),
    );
    adv.do_message(&AdvisorMessage {
        data2: 10,
        data3: 12,
        data4: disaster::FIRE,
        unknown: Some(0x50),
        ..AdvisorMessage::of_type(msg::DISASTER_STARTED)
    });
    assert_eq!(f.h.global("g_disasters_in_progress"), Some(1.0));
    assert_eq!(f.h.global("g_disasters_fire_started"), Some(1.0));
    assert_eq!(f.h.global("g_disasters_fire_cause_arson"), Some(1.0));
    let q = adv.state().queue;
    assert_eq!(q.len(), 1);
    assert_eq!(q[0].0, event::DISASTER_BASE ^ disaster::FIRE);
    assert_eq!(q[0].1.as_ref().unwrap().state().location, Some((10.0, 12.0)));
    f.day(100);
    assert!(f.advice(1).is_active());
    // Pipe bursts during a user disaster push nothing.
    adv.do_message(&AdvisorMessage {
        data4: 0x1234,
        ..AdvisorMessage::of_type(msg::DISASTER_STARTED)
    });
    assert!(adv.state().user_disaster);
    adv.do_message(&AdvisorMessage::of_type(msg::PIPE_BURST));
    assert_eq!(adv.state().queue.len(), 1);
    adv.do_message(&AdvisorMessage::of_type(msg::DISASTER_ENDED));
    assert_eq!(f.h.global("g_disasters_fire_ended"), Some(1.0));
    assert_eq!(f.h.global("g_disasters_in_progress"), Some(0.0));
    adv.do_message(&AdvisorMessage {
        data1: 1,
        ..AdvisorMessage::of_type(msg::HEALTH_STRIKE)
    });
    assert_eq!(f.h.global("g_health_strike"), Some(1.0));
    assert_eq!(adv.state().queue.last().unwrap().0, event::HEALTH_STRIKE_BEGIN);
}

// ================================================================ §5.2, §5.4 debug

#[test]
fn city_established_pushes_new_city_and_runs_the_monthly_step() {
    let f = F::plain(vec![AdviceEntry {
        event: event::NEW_CITY,
        ..ae(1, "true")
    }]);
    f.log();
    f.sys.do_message(&AdvisorMessage::of_type(msg::CITY_ESTABLISHED));
    assert_eq!(f.adv().month_counter(), 1);
    assert_eq!(f.adv().state().queue.len(), 1);
    assert!(f.log().contains(&HostCall::Ticker));
    f.sys.with_state(|s| s.loaded_from_save = true);
    f.sys.do_message(&AdvisorMessage::of_type(msg::POST_CITY_INIT));
    assert!(!f.sys.state().loaded_from_save);
}

#[test]
fn cheats_and_debug_messages() {
    let f = F::plain(vec![ae(0x10, "false"), ae(0x11, "=(0)")]);
    f.sys.cheat(cheat::DOLLY_LLAMA, "DollyLlama");
    assert!(f.sys.state().dolly_llama);
    f.log();
    f.sys.cheat(cheat::ADV, "adv trigger 10");
    assert_eq!(
        f.log(),
        vec![HostCall::Post {
            msg: msg::DEBUG_FORCE_TRIGGER,
            data1: 0x10,
            obj: "none".into()
        }]
    );
    let adv = f.adv();
    adv.do_message(&AdvisorMessage {
        data1: ID as u32,
        ..AdvisorMessage::of_type(msg::DEBUG_FORCE_TRIGGER)
    });
    assert!(f.advice(0x10).check_flags(flags::FORCED_TRIGGER));
    assert!(!f.advice(0x11).check_flags(flags::FORCED_TRIGGER), "=(0) skipped");
    adv.do_message(&AdvisorMessage {
        data1: 0x10,
        ..AdvisorMessage::of_type(msg::DEBUG_FORCE_TRIGGER)
    });
    assert!(f.advice(0x10).check_flags(flags::FORCED_TRIGGER | flags::DEBUG_GUID));
    // The forced advice activates on its next visit.
    f.set_month(1);
    f.day(500);
    assert!(f.advice(0x10).is_active());
    adv.do_message(&AdvisorMessage {
        data1: 0,
        ..AdvisorMessage::of_type(msg::DEBUG_SHOW_GUIDS)
    });
    assert!(!f.advice(0x10).check_flags(flags::DEBUG_GUID));
}

#[test]
fn query_interfaces() {
    assert!(system_query_interface(iid::ADVISOR_SYSTEM));
    assert!(!system_query_interface(iid::ADVICE));
    assert!(AdviceSubject::query_interface(iid::SUBJECT));
    let f = F::plain(vec![ae(1, "true")]);
    assert!(f.advice(1).query_interface(iid::ADVICE));
    assert!(!f.advice(1).query_interface(iid::NEIGHBOR_DEAL_ADVICE));
}

#[test]
fn src_codes_round_trip() {
    for k in SrcKind::ALL {
        assert_eq!(SrcKind::from_code(k.code()), Some(*k));
        assert_eq!(SrcKind::from_name(k.name()), Some(*k));
    }
    assert_eq!(SrcKind::from_code(0), None);
    assert_eq!(ListKind::from_code(7), Some(ListKind::DealProposals));
}

#[test]
fn rng_seed_is_derived_from_the_session() {
    let h = ScriptedHost::new();
    let sys = AdvisorSystem::new(h, 5);
    let want = crate::rng::derive_seed(5, clsid::SYSTEM);
    let mut g = crate::rng::RzRandom::new(
        want,
        &crate::rng::FixedSeeds {
            wall_clock: want,
            process_clock: want,
        },
    );
    assert_eq!(sys.with_random(|r| r.next_raw()), g.next_raw());
}
