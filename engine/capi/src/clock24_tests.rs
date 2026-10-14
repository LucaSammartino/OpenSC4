// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The 24-hour clock C ABI: a fake framework with a settable microsecond
//! clock, the tuning struct, the idle step, the cheats, the getters and the
//! 26-byte record.

use std::cell::{Cell, RefCell};
use std::ffi::c_void;

use crate::clock24::*;
use crate::test_streams::{istream, ostream, In, Out};

#[derive(Default)]
struct Game {
    now: Cell<u64>,
    posts: RefCell<Vec<(u32, u32, u32)>>,
    calls: RefCell<Vec<String>>,
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}
unsafe extern "C" fn now(u: *mut c_void) -> u64 {
    game(u).now.get()
}
unsafe extern "C" fn post(u: *mut c_void, m: u32, d2: u32, d3: u32) {
    game(u).posts.borrow_mut().push((m, d2, d3));
}
unsafe extern "C" fn add_service(u: *mut c_void) {
    game(u).calls.borrow_mut().push("add_service".into());
}
unsafe extern "C" fn remove_service(u: *mut c_void) {
    game(u).calls.borrow_mut().push("remove_service".into());
}
unsafe extern "C" fn add_idle(u: *mut c_void) {
    game(u).calls.borrow_mut().push("add_idle".into());
}
unsafe extern "C" fn remove_idle(u: *mut c_void) {
    game(u).calls.borrow_mut().push("remove_idle".into());
}
unsafe extern "C" fn yes(_: *mut c_void) -> bool {
    true
}
unsafe extern "C" fn register(u: *mut c_void, id: u32, name: *const u8, len: usize) {
    let n = std::str::from_utf8(std::slice::from_raw_parts(name, len)).unwrap();
    game(u).calls.borrow_mut().push(format!("register {id:08X} {n}"));
}
unsafe extern "C" fn notify(u: *mut c_void, id: u32) {
    game(u).calls.borrow_mut().push(format!("notify {id:08X}"));
}
unsafe extern "C" fn unnotify(u: *mut c_void, id: u32) {
    game(u).calls.borrow_mut().push(format!("unnotify {id:08X}"));
}
unsafe extern "C" fn unregister(u: *mut c_void, id: u32) {
    game(u).calls.borrow_mut().push(format!("unregister {id:08X}"));
}

fn host(g: &Game) -> fc4_clock24_host {
    fc4_clock24_host {
        struct_size: std::mem::size_of::<fc4_clock24_host>() as u32,
        user: g as *const Game as *mut c_void,
        now_micros: Some(now),
        post: Some(post),
        add_system_service: Some(add_service),
        remove_system_service: Some(remove_service),
        add_to_on_idle: Some(add_idle),
        remove_from_on_idle: Some(remove_idle),
        has_cheat_manager: Some(yes),
        register_cheat: Some(register),
        add_cheat_notification: Some(notify),
        remove_cheat_notification: Some(unnotify),
        unregister_cheat: Some(unregister),
    }
}

fn shipped() -> fc4_clock24_tuning {
    fc4_clock24_tuning {
        struct_size: std::mem::size_of::<fc4_clock24_tuning>() as u32,
        has_magnification: 1,
        has_night_start: 1,
        has_night_end: 1,
        _pad: 0,
        magnification: 135.0,
        night_start: 20,
        night_end: 1,
    }
}

fn state(c: *const fc4_clock24) -> fc4_clock24_state {
    let mut s = fc4_clock24_state::default();
    assert!(unsafe { fc4_clock24_get_state(c, &mut s) });
    s
}

#[test]
fn construction_init_and_getters() {
    unsafe {
        let g = Game::default();
        g.now.set(5_000_000);
        let h = host(&g);
        let c = fc4_clock24_create(&h);
        assert!(!c.is_null());
        let s = state(c);
        assert_eq!(s.struct_size as usize, std::mem::size_of::<fc4_clock24_state>());
        assert_eq!((s.seconds, s.visual_seconds, s.magnification), (43_200, 43_200, 100.0));
        assert_eq!((s.night_start, s.night_end, s.day_night_notifications), (20, 4, 1));
        let mut p = 0f32;
        fc4_clock24_get_time_percentage(c, &mut p);
        assert_eq!(p, 0.5);
        assert_eq!(fc4_clock24_get_hour(c), 12);

        let t = shipped();
        assert!(fc4_clock24_init(c, &t));
        assert_eq!(
            *g.calls.borrow(),
            vec![
                "add_service".to_string(),
                "add_idle".into(),
                "register 281F7858 WhatTimeIzIt".into(),
                "notify 281F7858".into(),
                "register C9B73FBB Stopwatch".into(),
                "notify C9B73FBB".into(),
            ]
        );
        let mut m = 0f32;
        assert_eq!(fc4_clock24_get_time_magnification(c, &mut m), 135.0f32.to_bits());
        assert_eq!(m, 135.0);
        let (mut lo, mut hi) = (0f32, 0f32);
        assert_eq!(
            fc4_clock24_get_time_magnification_min_max(c, &mut lo, &mut hi),
            2000.0f32.to_bits()
        );
        assert_eq!((lo.to_bits(), hi), (0x3D4C_CCCD, 2000.0));
        assert_eq!(
            (fc4_clock24_get_event_hour(c, 0), fc4_clock24_get_event_hour(c, 1)),
            (20, 1)
        );
        assert_eq!(fc4_clock24_get_event_hour(c, 2), 0);
        assert!(state(c).timer_running != 0);

        // The step threshold at 135: 451,851 µs nothing, 451,852 µs 61 seconds.
        g.now.set(5_000_000 + 451_851);
        assert!(fc4_clock24_on_idle(c, 0));
        assert_eq!(state(c).seconds, 43_200);
        g.now.set(5_000_000 + 451_852);
        assert!(fc4_clock24_on_idle(c, 0));
        let s = state(c);
        assert_eq!((s.seconds, s.visual_seconds, s.timer_elapsed_us), (43_261, 43_261, 0));
        assert_eq!(
            (
                fc4_clock24_get_minute(c),
                fc4_clock24_get_seconds(c),
                fc4_clock24_get_visual_seconds(c)
            ),
            (1, 1, 1)
        );

        // Setters.
        fc4_clock24_set_hour(c, 25);
        assert_eq!(fc4_clock24_get_hour(c), 1);
        assert_eq!(fc4_clock24_get_visual_hour(c), 12);
        assert_eq!(fc4_clock24_set_minute(c, 75), 4500);
        assert_eq!(fc4_clock24_set_seconds(c, 61), 15);
        assert_eq!(state(c).seconds, 4501);
        fc4_clock24_set_time_percentage(c, -0.5);
        assert_eq!(state(c).seconds, 66_496);
        assert!(fc4_clock24_set_time_magnification(c, 5000.0));
        fc4_clock24_get_time_magnification(c, &mut m);
        assert_eq!(m, 2000.0);
        assert!(fc4_clock24_set_time_magnification_min_max(c, 10.0, 5.0));
        fc4_clock24_get_time_magnification_min_max(c, &mut lo, &mut hi);
        assert_eq!((lo, hi), (10.0, 10.0));

        // Modes and night.
        fc4_clock24_set_visual_time_mode(c, 2);
        assert_eq!(fc4_clock24_get_visual_time_mode(c), 2);
        assert!(fc4_clock24_is_night(c));
        fc4_clock24_get_visual_time_percentage(c, &mut p);
        assert_eq!(p, 0.0);
        assert_eq!(fc4_clock24_get_visual_minute(c), 0);
        g.posts.borrow_mut().clear();
        fc4_clock24_enable_day_night_notifications(c, false);
        fc4_clock24_enable_day_night_notifications(c, true);
        assert_eq!(*g.posts.borrow(), vec![(0x4477_8234, 0, 0)]);

        assert!(fc4_clock24_shutdown(c));
        let s = state(c);
        assert_eq!((s.initialized, s.seconds, s.timer_running), (0, 0, 0));
        assert_eq!(g.calls.borrow().len(), 12);
        fc4_clock24_destroy(c);
    }
}

#[test]
fn pause_and_cheats() {
    unsafe {
        let g = Game::default();
        let h = host(&g);
        let c = fc4_clock24_create(&h);
        assert!(fc4_clock24_init(c, std::ptr::null()));
        assert_eq!(state(c).magnification, 100.0);
        assert!(fc4_clock24_hidden_pause(c, true));
        assert!(fc4_clock24_is_hidden_paused(c));
        assert!(fc4_clock24_hidden_pause(c, false));
        let text = b"Stopwatch PAUSE";
        assert!(fc4_clock24_do_message(
            c,
            0x230E_27AC,
            0xC9B7_3FBB,
            text.as_ptr(),
            text.len()
        ));
        assert!(fc4_clock24_is_paused(c));
        assert!(fc4_clock24_pause(c, false));
        assert!(!fc4_clock24_is_paused(c));
        let text = b"WhatTimeIzIt 7";
        assert!(fc4_clock24_do_message(
            c,
            0x230E_27AC,
            0x281F_7858,
            text.as_ptr(),
            text.len()
        ));
        assert_eq!(fc4_clock24_get_hour(c), 7);
        assert!(fc4_clock24_do_message(c, 0x230E_27AC, 0x281F_7858, std::ptr::null(), 0));
        assert_eq!(
            *g.posts.borrow(),
            vec![
                (0x4477_8236, 1, 0),
                (0x4477_8236, 0, 0),
                (0x4477_8235, 1, 0),
                (0x4477_8235, 0, 0),
                (0x4477_8230, 7, 0),
                (0x4477_8231, 7, 0),
            ]
        );
        fc4_clock24_destroy(c);
    }
}

#[test]
fn record_example_and_load_order() {
    unsafe {
        let g = Game::default();
        let h = host(&g);
        let c = fc4_clock24_create(&h);
        let t = shipped();
        fc4_clock24_init(c, &t);
        let mut out = Out::default();
        let os = ostream(&mut out);
        assert!(fc4_clock24_write(c, &os));
        assert_eq!(
            out.bytes,
            [
                0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x07, 0x43, 0x00, 0x00, 0xFA, 0x44, 0xC0, 0xA8, 0x00,
                0x00, 0x14, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00
            ]
        );
        // A saved city with magnification 50, max 1500, night 22 / 5, version 1.
        let mut rec = vec![1u8, 0, 0, 0, 0, 0];
        rec.extend_from_slice(&50.0f32.to_le_bytes());
        rec.extend_from_slice(&1500.0f32.to_le_bytes());
        rec.extend_from_slice(&75_600u32.to_le_bytes());
        rec.extend_from_slice(&[2, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        rec.extend_from_slice(&22u32.to_le_bytes());
        rec.extend_from_slice(&5u32.to_le_bytes());
        assert_eq!(rec.len(), 36);
        let c2 = fc4_clock24_create(&h);
        let mut inp = In::new(&rec);
        let is = istream(&mut inp);
        assert!(fc4_clock24_read(c2, &is));
        assert_eq!(inp.pos, 36);
        assert!(fc4_clock24_init(c2, &t));
        let s = state(c2);
        assert_eq!((s.magnification, s.max_magnification), (135.0, 1500.0));
        assert_eq!(
            (s.night_start, s.night_end, s.seconds, s.visual_seconds),
            (20, 1, 75_600, 43_200)
        );
        // A truncated record: false.
        let mut inp = In::new(&rec[..30]);
        let is = istream(&mut inp);
        assert!(!fc4_clock24_read(c2, &is));
        for x in [c, c2] {
            fc4_clock24_destroy(x);
        }
    }
}

#[test]
fn null_and_bad_arguments() {
    unsafe {
        let null = std::ptr::null_mut::<fc4_clock24>();
        assert!(fc4_clock24_create(std::ptr::null()).is_null());
        let g = Game::default();
        let mut h = host(&g);
        h.struct_size = 8;
        assert!(fc4_clock24_create(&h).is_null());
        assert!(!fc4_clock24_init(null, std::ptr::null()));
        assert!(fc4_clock24_on_idle(null, 0));
        assert_eq!(fc4_clock24_get_hour(null), 0);
        let mut f = 3.0f32;
        fc4_clock24_get_time_percentage(null, &mut f);
        assert_eq!(f, 3.0);
        assert!(!fc4_clock24_get_state(null, &mut fc4_clock24_state::default()));
        fc4_clock24_destroy(null);
        // A too-small tuning struct.
        let h = host(&g);
        let c = fc4_clock24_create(&h);
        let mut t = shipped();
        t.struct_size = 4;
        assert!(!fc4_clock24_init(c, &t));
        assert_eq!(state(c).initialized, 0);
        // Missing properties keep the values (the magnification is clamped).
        let t = fc4_clock24_tuning {
            struct_size: std::mem::size_of::<fc4_clock24_tuning>() as u32,
            ..Default::default()
        };
        assert!(fc4_clock24_set_time_magnification_min_max(c, 1.0, 50.0));
        assert!(fc4_clock24_init(c, &t));
        let s = state(c);
        assert_eq!((s.magnification, s.night_start, s.night_end), (50.0, 20, 4));
        fc4_clock24_destroy(c);
        assert!(fc4_clock24_answers_iid(0x287F_B697));
        assert!(!fc4_clock24_answers_iid(0));
    }
}
