// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Tests that drive the library only through its C ABI, the way the C++ shim does.

use super::*;
use std::cell::{Cell, RefCell};

#[derive(Default)]
struct Recorder {
    now: Cell<u32>,
    step: Cell<u32>,
    deliveries: RefCell<Vec<(fc4_agent, fc4_message)>>,
    posts: RefCell<Vec<fc4_message>>,
    refs: RefCell<Vec<(char, fc4_agent)>>,
    /// Agent that removes itself (all levels) on its first delivery.
    self_removing: Cell<fc4_agent>,
}

unsafe extern "C" fn now_ms(user: *mut c_void) -> u32 {
    let r = &*(user as *const Recorder);
    let t = r.now.get();
    r.now.set(t.wrapping_add(r.step.get()));
    t
}

unsafe extern "C" fn deliver(user: *mut c_void, sim: *mut fc4_sim, agent: fc4_agent, msg: *const fc4_message) -> bool {
    let r = &*(user as *const Recorder);
    r.deliveries.borrow_mut().push((agent, *msg));
    if agent == r.self_removing.get() {
        // Re-enter the simulator from inside a delivery.
        assert!(fc4_sim_remove_agent_all_levels(sim, agent));
    }
    true
}

unsafe extern "C" fn add_ref(user: *mut c_void, agent: fc4_agent) {
    (*(user as *const Recorder)).refs.borrow_mut().push(('+', agent));
}

unsafe extern "C" fn release(user: *mut c_void, agent: fc4_agent) {
    (*(user as *const Recorder)).refs.borrow_mut().push(('-', agent));
}

unsafe extern "C" fn post(user: *mut c_void, msg: *const fc4_message) {
    (*(user as *const Recorder)).posts.borrow_mut().push(*msg);
}

fn make(rec: &Recorder) -> *mut fc4_sim {
    let host = fc4_host {
        struct_size: std::mem::size_of::<fc4_host>() as u32,
        user: rec as *const Recorder as *mut c_void,
        now_ms: Some(now_ms),
        deliver: Some(deliver),
        add_ref: Some(add_ref),
        release: Some(release),
        post: Some(post),
    };
    let sim = unsafe { fc4_sim_create(&host) };
    assert!(!sim.is_null());
    sim
}

#[test]
fn abi_version_and_layout() {
    assert_eq!(fc4_abi_version(), FC4_ABI_VERSION);
    let mut widths: *const u8 = std::ptr::null();
    let n = unsafe { fc4_record_layout(4, &mut widths) };
    assert_eq!(n, 30);
    let total: usize = unsafe { std::slice::from_raw_parts(widths, n) }
        .iter()
        .map(|&w| w as usize)
        .sum();
    assert_eq!(total, 79);
    assert_eq!(unsafe { fc4_record_layout(5, &mut widths) }, 0);
}

#[test]
fn create_rejects_bad_host() {
    unsafe {
        assert!(fc4_sim_create(std::ptr::null()).is_null());
        let mut host = fc4_host {
            struct_size: 4,
            user: std::ptr::null_mut(),
            now_ms: Some(now_ms),
            deliver: None,
            add_ref: None,
            release: None,
            post: None,
        };
        assert!(fc4_sim_create(&host).is_null());
        host.struct_size = std::mem::size_of::<fc4_host>() as u32;
        host.now_ms = None;
        assert!(fc4_sim_create(&host).is_null());
    }
}

#[test]
fn null_sim_is_tolerated() {
    unsafe {
        let null = std::ptr::null_mut();
        assert!(!fc4_sim_pause(null));
        assert!(fc4_sim_on_tick(null));
        assert_eq!(fc4_sim_load(null, [0u8].as_ptr(), 1), FC4_ERR_INVALID_ARGUMENT);
        assert_eq!(fc4_sim_save(null, std::ptr::null_mut(), 0), 0);
        fc4_sim_destroy(null);
    }
}

#[test]
fn tick_delivers_and_posts_days() {
    let rec = Recorder::default();
    rec.step.set(1);
    let sim = make(&rec);
    unsafe {
        let table = [2000u32, 1000, 50];
        let dil = [1.0f32, 1.0, 1.5, 3.0, 1.0];
        let tuning = fc4_tuning {
            ms_per_day: table.as_ptr(),
            ms_per_day_count: 3,
            animation_dilation: dil.as_ptr(),
            animation_dilation_count: 5,
        };
        let name = "day";
        assert!(fc4_sim_add_agent(sim, 7, 4, name.as_ptr() as _, name.len(), 3));
        assert!(fc4_sim_init(sim, &tuning));
        assert!(fc4_sim_set_sim_speed(sim, 2));
        assert!(fc4_sim_is_hidden_paused(sim));
        assert!(fc4_sim_on_post_city_init(sim));
        assert!(!fc4_sim_is_hidden_paused(sim));
        for _ in 0..40 {
            fc4_sim_on_tick(sim);
        }
        let mut st = fc4_state {
            struct_size: std::mem::size_of::<fc4_state>() as u32,
            ..Default::default()
        };
        assert!(fc4_sim_get_state(sim, &mut st));
        assert_eq!(st.ms_per_day, 50);
        assert_eq!(st.target_fps, 15);
        assert!(st.julian_day > 2_451_545);
        assert!((fc4_sim_get_animation_time_dilation(sim) - 3.0).abs() < 1e-6);

        let posts = rec.posts.borrow();
        let days: Vec<_> = posts.iter().filter(|m| m.type_ == 0x6695_6814).collect();
        assert_eq!(days.len() as i32, st.julian_day - 2_451_545);
        for (i, m) in days.iter().enumerate() {
            assert_eq!(m.data1, 2_451_546 + i as i64);
            assert_eq!(m.data3, 2 + i as i64); // day of year
        }
        let deliveries = rec.deliveries.borrow();
        assert!(deliveries.iter().all(|(a, m)| *a == 7 && m.type_ == 0x6695_6814));

        let mut dbg = fc4_debug {
            struct_size: std::mem::size_of::<fc4_debug>() as u32,
            ..Default::default()
        };
        assert!(fc4_sim_get_debug(sim, &mut dbg));
        assert_eq!(dbg.ms_per_day, 50);
        assert_eq!(dbg.one_shot_or_emergency_pending, 0);
        assert_eq!(dbg.cursors[4], -1);
        let mut info = fc4_agent_info::default();
        let mut name = [0 as c_char; 3];
        assert!(fc4_sim_get_agent_info(
            sim,
            4,
            0,
            &mut info,
            name.as_mut_ptr(),
            name.len()
        ));
        assert_eq!((info.agent, info.flags, info.enabled), (7, 3, 1));
        assert_eq!(name.map(|c| c as u8), *b"da\0"); // "day" truncated to the buffer
        assert!(!fc4_sim_get_agent_info(sim, 4, 1, &mut info, std::ptr::null_mut(), 0));
        assert!(!fc4_sim_get_agent_info(sim, 99, 0, &mut info, std::ptr::null_mut(), 0));

        let mut date = fc4_date::default();
        assert!(fc4_sim_get_date(sim, &mut date));
        assert_eq!(date.julian, st.julian_day);
        fc4_sim_destroy(sim);
    }
    // Exactly one AddRef and one Release (on destroy).
    assert_eq!(*rec.refs.borrow(), vec![('+', 7), ('-', 7)]);
}

#[test]
fn agent_may_remove_itself_during_delivery() {
    let rec = Recorder::default();
    rec.step.set(1);
    rec.self_removing.set(9);
    let sim = make(&rec);
    unsafe {
        assert!(fc4_sim_add_agent(sim, 9, 0, std::ptr::null(), 0, 1));
        assert!(fc4_sim_add_agent(sim, 10, 0, std::ptr::null(), 0, 1));
        assert!(fc4_sim_init(sim, std::ptr::null()));
        fc4_sim_simulate_agents(sim);
        let mut st = fc4_state {
            struct_size: std::mem::size_of::<fc4_state>() as u32,
            ..Default::default()
        };
        assert!(fc4_sim_get_state(sim, &mut st));
        assert_eq!(st.agent_counts[0], 0);
        fc4_sim_destroy(sim);
    }
    let d: Vec<_> = rec.deliveries.borrow().iter().map(|(a, _)| *a).collect();
    assert_eq!(d, vec![9, 10]);
    assert_eq!(*rec.refs.borrow(), vec![('+', 9), ('+', 10), ('-', 9), ('-', 10)]);
}

#[test]
fn save_load_and_cheats_through_abi() {
    let rec = Recorder::default();
    let sim = make(&rec);
    unsafe {
        let text = "SimDate 2 29 2004";
        assert!(fc4_sim_handle_cheat(sim, 0x8A78_BEFF, text.as_ptr() as _, text.len()));
        assert_eq!(fc4_sim_get_julian_day(sim), fc4_julian_from_civil(2004, 2, 29));
        let size = fc4_sim_save(sim, std::ptr::null_mut(), 0);
        assert_eq!(size, 79);
        let mut buf = vec![0u8; size];
        assert_eq!(fc4_sim_save(sim, buf.as_mut_ptr(), buf.len()), 79);
        let other = make(&rec);
        assert_eq!(fc4_sim_load(other, buf.as_ptr(), buf.len()), FC4_OK);
        assert_eq!(fc4_sim_get_julian_day(other), fc4_julian_from_civil(2004, 2, 29));
        assert_eq!(fc4_sim_load(other, buf.as_ptr(), 10), FC4_ERR_TRUNCATED);
        buf[0] = 5;
        assert_eq!(
            fc4_sim_load(other, buf.as_ptr(), buf.len()),
            FC4_ERR_UNSUPPORTED_VERSION
        );
        let mut d = fc4_date::default();
        fc4_date_from_julian(2_451_545, &mut d);
        assert_eq!(
            (d.year, d.month, d.day_of_month, d.day_of_year, d.weekday),
            (2000, 1, 1, 1, 6)
        );
        fc4_sim_destroy(other);
        fc4_sim_destroy(sim);
    }
}

#[derive(Default)]
struct FakeTimers {
    qpc: Cell<u64>,
    tsc: Cell<u64>,
    published: RefCell<Vec<(f32, f32)>>,
}

unsafe extern "C" fn fake_qpc(user: *mut c_void) -> u64 {
    (*(user as *const FakeTimers)).qpc.get()
}
unsafe extern "C" fn fake_tsc(user: *mut c_void) -> u64 {
    (*(user as *const FakeTimers)).tsc.get()
}
unsafe extern "C" fn fake_publish(user: *mut c_void, a: f32, b: f32) {
    (*(user as *const FakeTimers)).published.borrow_mut().push((a, b));
}

#[test]
fn limit_timer_calibration_through_abi() {
    let rec = Recorder::default();
    let sim = make(&rec);
    let timers = FakeTimers::default();
    let mut src = fc4_timer_source {
        struct_size: std::mem::size_of::<fc4_timer_source>() as u32,
        user: &timers as *const FakeTimers as *mut c_void,
        qpc: Some(fake_qpc),
        tsc: Some(fake_tsc),
        publish: Some(fake_publish),
        qpc_frequency: 1_000_000,
    };
    unsafe {
        assert!(!fc4_sim_get_calibration(
            sim,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut()
        ));
        assert!(!fc4_sim_set_timer_source(sim, &src, 0.0));
        src.tsc = None;
        assert!(!fc4_sim_set_timer_source(sim, &src, 1e-6));
        src.tsc = Some(fake_tsc);
        assert!(fc4_sim_set_timer_source(sim, &src, 1e-6));
        assert_eq!(timers.published.borrow().len(), 1);
        // 1.1 s at 2 GHz, through OnTick (before Init).
        timers.qpc.set(1_100_000);
        timers.tsc.set(2_200_000_000);
        assert!(fc4_sim_on_tick(sim));
        let (mut c2m, mut m2c, mut n) = (0f32, 0f32, 0u64);
        assert!(fc4_sim_get_calibration(sim, &mut c2m, &mut m2c, &mut n));
        assert!(((c2m as f64 - 9.5e-7) / 9.5e-7).abs() < 1e-6);
        assert!(((m2c as f64 * 9.5e-7) - 1.0).abs() < 1e-6);
        assert_eq!(n, 1);
        assert_eq!(*timers.published.borrow().last().unwrap(), (c2m, m2c));
        fc4_sim_destroy(sim);
    }
}

#[test]
fn enumerate_names() {
    let rec = Recorder::default();
    let sim = make(&rec);
    unsafe extern "C" fn collect(ctx: *mut c_void, name: *const c_char, len: usize) {
        let v = &mut *(ctx as *mut Vec<String>);
        v.push(String::from_utf8_lossy(std::slice::from_raw_parts(name as *const u8, len)).into_owned());
    }
    unsafe {
        for (h, lvl, n) in [(1u64, 4u32, "a"), (2, 6, "b"), (3, 4, "c")] {
            assert!(fc4_sim_add_agent(sim, h, lvl, n.as_ptr() as _, n.len(), 1));
        }
        let mut names: Vec<String> = Vec::new();
        let n = fc4_sim_enumerate_agents(sim, Some(collect), &mut names as *mut _ as *mut c_void);
        assert_eq!(n, 3);
        assert_eq!(names, vec!["a", "c", "b"]);
        fc4_sim_destroy(sim);
    }
}
