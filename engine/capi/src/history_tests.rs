// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The history warehouse C ABI: a fake game whose `send` delivers the record
//! message back to the warehouse synchronously (as the message server does),
//! the queries at both precisions, the pass through SimNewDay, persistence.

use std::cell::{Cell, RefCell};
use std::ffi::c_void;

use crate::history::*;
use crate::test_streams::{istream, ostream, In, Out};

#[derive(Default)]
struct Game {
    date: Cell<u32>,
    /// The warehouse the fake message server delivers to (null: none).
    target: Cell<usize>,
    log: RefCell<Vec<String>>,
    sends: Cell<u32>,
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}
unsafe extern "C" fn sim_date(u: *mut c_void) -> u32 {
    game(u).date.get()
}
unsafe extern "C" fn now_ms(_: *mut c_void) -> u32 {
    0
}
unsafe extern "C" fn subscribe(u: *mut c_void, m: u32) {
    game(u).log.borrow_mut().push(format!("sub {m:08X}"));
}
unsafe extern "C" fn unsubscribe(u: *mut c_void, m: u32) {
    game(u).log.borrow_mut().push(format!("unsub {m:08X}"));
}
unsafe extern "C" fn add_agent(u: *mut c_void, level: u32, name: *const u8, len: usize, flags: u32) {
    let n = std::str::from_utf8(std::slice::from_raw_parts(name, len)).unwrap();
    game(u).log.borrow_mut().push(format!("agent {level} {n} {flags}"));
}
unsafe extern "C" fn remove_agent(u: *mut c_void) {
    game(u).log.borrow_mut().push("remove_agent".into());
}
/// The message server: MessageSend delivers to the subscriber at once.
unsafe extern "C" fn send(u: *mut c_void, m: u32, d1: u32, d2: u32) {
    let g = game(u);
    g.sends.set(g.sends.get() + 1);
    let w = g.target.get() as *mut fc4_history;
    if !w.is_null() {
        assert!(fc4_history_do_message(w, m, d1, d2));
    }
}

fn host(g: &Game) -> fc4_history_host {
    fc4_history_host {
        struct_size: std::mem::size_of::<fc4_history_host>() as u32,
        user: g as *const Game as *mut c_void,
        sim_date: Some(sim_date),
        now_ms: Some(now_ms),
        subscribe: Some(subscribe),
        unsubscribe: Some(unsubscribe),
        add_agent: Some(add_agent),
        remove_agent: Some(remove_agent),
        send: Some(send),
    }
}

const K: u32 = 0xAA11_F837;
const NEW_DAY: u32 = 0x6695_6814;
const NEW_YEAR: u32 = 0x6695_6817;

fn series(w: *const fc4_history, key: u32) -> (u32, Vec<(u32, i32)>) {
    unsafe {
        let (mut max, mut n) = (0u32, 0usize);
        assert!(fc4_history_get_series(
            w,
            key,
            &mut max,
            std::ptr::null_mut(),
            0,
            &mut n
        ));
        let mut v = vec![fc4_history_point::default(); n];
        assert!(fc4_history_get_series(w, key, &mut max, v.as_mut_ptr(), n, &mut n));
        (max, v.iter().map(|p| (p.date, p.value)).collect())
    }
}

#[test]
fn create_init_record_through_the_message_server() {
    unsafe {
        let g = Game::default();
        g.date.set(2_451_545);
        let h = host(&g);
        let w = fc4_history_create(&h);
        assert!(!w.is_null());
        g.target.set(w as usize);
        assert!(fc4_history_init(w));
        assert_eq!(
            *g.log.borrow(),
            vec![
                "sub 89EFA536".to_string(),
                "sub 89EFA537".into(),
                "agent 4 cSC4HistoryWarehouse 3".into(),
                "agent 7 cSC4HistoryWarehouse 3".into(),
            ]
        );
        // RecordHistoryData goes through send, which re-enters the warehouse.
        assert!(fc4_history_record(w, K, 5));
        assert!(fc4_history_record(w, K, 6)); // same day: overwrites
        g.date.set(2_451_546);
        assert!(fc4_history_record(w, K, -7));
        assert_eq!(g.sends.get(), 3);
        assert_eq!(series(w, K), (0, vec![(2_451_545, 6), (2_451_546, -7)]));
        assert!(fc4_history_set_queue_length(w, K, 1));
        assert_eq!(series(w, K), (1, vec![(2_451_546, -7)]));

        let mut st = fc4_history_state::default();
        assert!(fc4_history_get_state(w, &mut st));
        assert_eq!(st.struct_size as usize, std::mem::size_of::<fc4_history_state>());
        assert_eq!((st.initialized, st.series_count, st.total_points), (1, 1, 1));
        let mut keys = [0u32; 4];
        assert_eq!(fc4_history_keys(w, keys.as_mut_ptr(), 4), 1);
        assert_eq!(keys[0], K);

        assert!(fc4_history_shutdown(w));
        assert_eq!(g.log.borrow().len(), 7);
        fc4_history_destroy(w);
    }
}

#[test]
fn queries_and_precision() {
    unsafe {
        let g = Game::default();
        let h = host(&g);
        let w = fc4_history_create(&h);
        for (d, v) in [(7000u32, 1), (7001, 2), (7002, 4)] {
            g.date.set(d);
            assert!(fc4_history_do_message(w, 0x89EF_A536, K, v as u32));
        }
        let mut n = -1.0f32;
        assert!(fc4_history_get_count(w, K, 7000, 7002, &mut n));
        assert_eq!(n, 3.0);
        let (mut s24, mut i24, mut s53, mut i53) = (0f32, 0f32, 0f32, 0f32);
        assert!(fc4_history_get_trend(w, K, 7000, 7002, 24, &mut s24, &mut i24));
        assert!(fc4_history_get_trend(w, K, 7000, 7002, 53, &mut s53, &mut i53));
        assert!((s24 - 1.5).abs() < 1e-6 && (i24 - 5.0 / 6.0).abs() < 1e-6);
        assert_ne!((s24, i24), (s53, i53));
        let mut p = 0f32;
        assert!(fc4_history_get_point(w, K, 7001, 24, &mut p));
        assert!((p - 7.0 / 3.0).abs() < 1e-6);
        let (mut a, mut b, mut c) = (0f32, 0f32, 0f32);
        assert!(fc4_history_get_min_max_average(
            w, K, 7000, 7002, 24, &mut a, &mut b, &mut c
        ));
        assert_eq!((a, b), (1.0, 4.0));
        assert_eq!(c, 7.0f32 / 3.0);
        // Missing key: false, outputs untouched.
        let mut untouched = 42.0f32;
        assert!(!fc4_history_get_point(w, 0x99, 7001, 24, &mut untouched));
        assert!(!fc4_history_get_count(w, 0x99, 0, 1, &mut untouched));
        assert!(!fc4_history_get_trend(
            w,
            0x99,
            0,
            1,
            24,
            &mut untouched,
            std::ptr::null_mut()
        ));
        assert_eq!(untouched, 42.0);
        // NULL outputs are allowed.
        assert!(fc4_history_get_min_max_average(
            w,
            K,
            7000,
            7002,
            53,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut()
        ));
        // Coalesce / Dispose.
        assert!(!fc4_history_coalesce(w, 0x99, 0, 1, 0));
        assert!(fc4_history_dispose_range(w, K, 7000, 7001));
        assert_eq!(series(w, K).1, vec![(7001, 2), (7002, 4)]);
        assert!(fc4_history_dispose(w, K));
        assert!(series(w, K).1.is_empty());
        assert!(!fc4_history_dispose(w, 0x99));
        fc4_history_destroy(w);
    }
}

#[test]
fn yearly_pass_through_messages() {
    unsafe {
        let g = Game::default();
        let h = host(&g);
        let w = fc4_history_create(&h);
        let d = |y, m, dd| fc4_core::julian::julian_from_civil(y, m, dd) as u32;
        for (date, v) in [(d(2025, 10, 5), 1), (d(2025, 10, 20), 2), (d(2025, 11, 3), 3)] {
            g.date.set(date);
            fc4_history_do_message(w, 0x89EF_A536, K, v);
        }
        g.date.set(d(2026, 1, 1));
        assert!(fc4_history_do_message(w, NEW_YEAR, 0, 0));
        let mut st = fc4_history_state::default();
        fc4_history_get_state(w, &mut st);
        assert_eq!(st.pass_state, 1);
        assert!(fc4_history_do_message(w, NEW_DAY, 0, 0));
        assert_eq!(series(w, K).1, vec![(d(2025, 10, 1), 1), (d(2025, 11, 3), 3)]);
        assert!(!fc4_history_do_message(w, NEW_DAY, 0, 0)); // the end: once false
        fc4_history_get_state(w, &mut st);
        assert_eq!(st.pass_state, 0);
        fc4_history_destroy(w);
    }
}

#[test]
fn save_round_trip() {
    unsafe {
        let g = Game::default();
        let h = host(&g);
        let w = fc4_history_create(&h);
        fc4_history_set_queue_length(w, 0x20, 365);
        for (date, v) in [(100u32, -5), (101, 7)] {
            g.date.set(date);
            fc4_history_do_message(w, 0x89EF_A536, 0x20, v as u32);
        }
        let mut out = Out::default();
        let os = ostream(&mut out);
        assert!(fc4_history_write(w, &os));
        let mut want = vec![2u8, 0, 1, 0, 0, 0];
        for v in [0x20u32, 365, 2, 100, (-5i32) as u32, 101, 7] {
            want.extend_from_slice(&v.to_le_bytes());
        }
        assert_eq!(out.bytes, want);

        let w2 = fc4_history_create(&h);
        let mut inp = In::new(&out.bytes);
        let is = istream(&mut inp);
        assert!(fc4_history_read(w2, &is));
        assert_eq!(series(w2, 0x20), (365, vec![(100, -5), (101, 7)]));
        // Version 3: false.
        let mut v3 = out.bytes.clone();
        v3[0] = 3;
        let w3 = fc4_history_create(&h);
        let mut inp = In::new(&v3);
        let is = istream(&mut inp);
        assert!(!fc4_history_read(w3, &is));
        // A truncated record: false.
        let mut inp = In::new(&out.bytes[..out.bytes.len() - 1]);
        let is = istream(&mut inp);
        assert!(!fc4_history_read(w3, &is));
        for x in [w, w2, w3] {
            fc4_history_destroy(x);
        }
    }
}

#[test]
fn null_and_bad_arguments() {
    unsafe {
        let null = std::ptr::null_mut::<fc4_history>();
        assert!(fc4_history_create(std::ptr::null()).is_null());
        let g = Game::default();
        let mut h = host(&g);
        h.struct_size = 8;
        assert!(fc4_history_create(&h).is_null());
        assert!(!fc4_history_init(null));
        assert!(!fc4_history_record(null, 1, 1));
        assert!(fc4_history_do_message(null, 0, 0, 0));
        assert_eq!(fc4_history_keys(null, std::ptr::null_mut(), 0), 0);
        let mut f = 0f32;
        assert!(!fc4_history_get_count(null, 1, 0, 1, &mut f));
        assert!(!fc4_history_get_state(null, &mut fc4_history_state::default()));
        fc4_history_destroy(null);
        assert!(fc4_history_answers_iid(0x69F4_A803));
        assert!(!fc4_history_answers_iid(0x4522_94AA));
        // A host without callbacks: date 0, sends go nowhere.
        let empty = fc4_history_host {
            struct_size: std::mem::size_of::<fc4_history_host>() as u32,
            user: std::ptr::null_mut(),
            sim_date: None,
            now_ms: None,
            subscribe: None,
            unsubscribe: None,
            add_agent: None,
            remove_agent: None,
            send: None,
        };
        let w = fc4_history_create(&empty);
        assert!(fc4_history_init(w));
        assert!(fc4_history_record(w, 1, 1));
        assert_eq!(fc4_history_keys(w, std::ptr::null_mut(), 0), 0);
        fc4_history_destroy(w);
    }
}
