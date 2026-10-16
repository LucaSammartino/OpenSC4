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
    /// Busy-wait iterations inside `now_ms`, which a caller holds the state
    /// open across: widens that window for the cross-thread test below.
    spin: Cell<u32>,
    deliveries: RefCell<Vec<(fc4_agent, fc4_message)>>,
    posts: RefCell<Vec<fc4_message>>,
    refs: RefCell<Vec<(char, fc4_agent)>>,
    /// Agent that removes itself (all levels) on its first delivery.
    self_removing: Cell<fc4_agent>,
}

unsafe extern "C" fn now_ms(user: *mut c_void) -> u32 {
    let r = &*(user as *const Recorder);
    let mut n = r.spin.get();
    while n > 0 {
        std::hint::spin_loop();
        n -= 1;
    }
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

/// The simulator handle moved to another thread.
///
/// SAFETY: the handle is a `Box<Simulator>`, not `Send` only because its host
/// is a trait object. Through this wrapper the second thread calls clock
/// getters alone (`is_initialized`, `get_sim_time`, `get_julian_day`), none of
/// which reaches a host callback or the `Recorder`; the handle is destroyed on
/// the thread that created it. Both wrappers hold the same pointer, because the
/// game shares one `cISC4Simulator` between its threads.
struct OnThread(*mut fc4_sim);
unsafe impl Send for OnThread {}

/// The game calls the `cISC4Simulator` interface from more than one thread: a
/// worker reads the clock while the main thread is inside `OnTick`. In-game
/// run 26 made that concrete — `RefCell already mutably borrowed` at
/// `core/src/simulator.rs`, a fail-fast about 0.3 s after PostCityInit, while a
/// power recompute was running on the main thread. A reader must therefore wait
/// for an open state section instead of failing; this drives the two from
/// different threads and asserts every read succeeds.
///
/// A reader that loses the race returns the ABI's fallback (`false`), so the
/// assertion below is the whole test: with a `RefCell` it fails on the first
/// contended read, and the panic it would raise is swallowed by the C ABI's
/// guard rather than reaching the panic hook.
#[test]
fn a_second_thread_may_read_the_clock_while_a_tick_runs() {
    let rec = Recorder::default();
    rec.spin.set(4000); // widen each state section so the two threads overlap
    let sim = OnThread(make(&rec));
    unsafe {
        assert!(fc4_sim_init(sim.0, std::ptr::null()));
    }
    let reader_handle = OnThread(sim.0); // the same simulator, read from another thread
    std::thread::scope(|scope| {
        let reader = scope.spawn(move || {
            // Take the whole wrapper (disjoint capture would otherwise move the
            // bare pointer field, which is not Send).
            let h = reader_handle;
            let mut failures = 0u32;
            for _ in 0..4000 {
                // A getter, the way the game's worker thread asks for the date.
                failures += u32::from(!unsafe { fc4_sim_is_initialized(h.0) });
                let _ = unsafe { fc4_sim_get_sim_time(h.0) };
                let _ = unsafe { fc4_sim_get_julian_day(h.0) };
                std::thread::yield_now();
            }
            failures
        });
        // This thread is the main thread: it ticks while the reader reads.
        for _ in 0..400 {
            unsafe {
                fc4_sim_on_tick(sim.0);
                fc4_sim_simulate_time_passage(sim.0);
            }
        }
        let failures = reader.join().expect("the reader thread must not panic");
        assert_eq!(failures, 0, "every cross-thread read of the clock must succeed");
    });
    unsafe { fc4_sim_destroy(sim.0) };
}

#[derive(Default)]
struct PacingRecorder {
    settles: RefCell<Vec<i32>>,
}

unsafe extern "C" fn paced_is_8(_user: *mut c_void, agent: fc4_agent) -> bool {
    agent == 8
}

unsafe extern "C" fn record_settle(user: *mut c_void, julian: i32) {
    (*(user as *const PacingRecorder)).settles.borrow_mut().push(julian);
}

#[test]
fn deterministic_pacing_through_the_abi() {
    // simulator.md §5.7 [choice]: the paced idle agent (8) gets exactly one
    // settle SimIdle per day (data3 = 22), never a frame idle; the other (9)
    // keeps the frame idle passes; settle() runs once per day, in order.
    assert!(!fc4_pacing_set_process(7));
    // The default; other tests in this process create cores concurrently,
    // so this test never switches the process mode.
    assert!(fc4_pacing_set_process(FC4_PACING_ORIGINAL));
    assert_eq!(fc4_pacing_process(), FC4_PACING_ORIGINAL);
    let rec = Recorder::default();
    rec.step.set(1);
    let pr = PacingRecorder::default();
    let sim = make(&rec);
    unsafe {
        let hooks = fc4_pacing_hooks {
            struct_size: std::mem::size_of::<fc4_pacing_hooks>() as u32,
            user: &pr as *const PacingRecorder as *mut c_void,
            is_paced: Some(paced_is_8),
            settle: Some(record_settle),
            idle_steps: None,
        };
        let short = fc4_pacing_hooks {
            struct_size: 4,
            ..hooks
        };
        assert!(!fc4_sim_set_pacing(sim, FC4_PACING_DETERMINISTIC, &short));
        assert!(!fc4_sim_set_pacing(sim, 9, &hooks));
        assert!(!fc4_sim_set_pacing(
            std::ptr::null_mut(),
            FC4_PACING_DETERMINISTIC,
            &hooks
        ));
        for (agent, level, name) in [(7u64, 4u32, "day"), (8, 10, "ours"), (9, 10, "theirs")] {
            assert!(fc4_sim_add_agent(sim, agent, level, name.as_ptr() as _, name.len(), 3));
        }
        assert!(fc4_sim_init(sim, std::ptr::null()));
        assert!(fc4_sim_on_post_city_init(sim));
        assert!(fc4_sim_set_pacing(sim, FC4_PACING_DETERMINISTIC, &hooks));
        let mut mode = 9u32;
        let mut settled = false;
        assert!(fc4_sim_get_pacing(sim, &mut mode, &mut settled));
        assert_eq!((mode, settled), (FC4_PACING_DETERMINISTIC, true));
        let start = fc4_sim_get_julian_day(sim);
        assert!(fc4_sim_set_turbo(sim, true, 50, 0));
        for _ in 0..20 {
            fc4_sim_on_tick(sim);
        }
        let end = fc4_sim_get_julian_day(sim);
        assert!(end > start + 2, "turbo opened days: {start}..{end}");
        let settles = pr.settles.borrow().clone();
        // Every opened day but possibly the last is settled, in order.
        let want: Vec<i32> = (start + 1..=end).collect();
        assert!(
            settles == want || settles == want[..want.len() - 1],
            "{settles:?} {want:?}"
        );
        let ours: Vec<fc4_message> = rec
            .deliveries
            .borrow()
            .iter()
            .filter(|(a, m)| *a == 8 && m.type_ == 0x6695_681A)
            .map(|(_, m)| *m)
            .collect();
        assert_eq!(ours.len(), settles.len());
        assert!(ours.iter().all(|m| m.data2 == 0 && m.data3 == 22));
        let days: Vec<i32> = ours.iter().map(|m| m.data1 as i32).collect();
        assert_eq!(days, settles);
        let theirs = rec.deliveries.borrow().iter().filter(|(a, _)| *a == 9).count();
        assert!(theirs > 0, "frame idle still reaches the original's agents");
        // Back to original: the paced agent gets frame idle again.
        assert!(fc4_sim_set_pacing(sim, FC4_PACING_ORIGINAL, std::ptr::null()));
        assert!(fc4_sim_get_pacing(sim, &mut mode, &mut settled));
        assert_eq!((mode, settled), (FC4_PACING_ORIGINAL, true));
        let before = rec.deliveries.borrow().iter().filter(|(a, _)| *a == 8).count();
        fc4_sim_on_tick(sim);
        let after = rec.deliveries.borrow().iter().filter(|(a, _)| *a == 8).count();
        assert!(after > before);
        fc4_sim_destroy(sim);
    }
}

#[test]
fn turbo_clock_through_the_abi() {
    // Tooling (fc4_core::Turbo): days open without the msPerDay wait, `until`
    // switches turbo off once and reports the day once.
    let rec = Recorder::default();
    rec.step.set(1);
    let sim = make(&rec);
    unsafe {
        let mut on = true;
        let mut frame_ms = 0u32;
        let mut until = -1i32;
        assert!(fc4_sim_get_turbo(sim, &mut on, &mut frame_ms, &mut until));
        assert_eq!((on, frame_ms, until), (false, 250, 0));
        assert!(!fc4_sim_get_turbo(std::ptr::null(), &mut on, &mut frame_ms, &mut until));
        assert!(!fc4_sim_set_turbo(std::ptr::null_mut(), true, 250, 0));
        assert_eq!(fc4_sim_take_turbo_auto_off(std::ptr::null_mut()), 0);

        let name = "day";
        assert!(fc4_sim_add_agent(sim, 7, 4, name.as_ptr() as _, name.len(), 3));
        assert!(fc4_sim_init(sim, std::ptr::null())); // speed 0: msPerDay 5000
        assert!(fc4_sim_on_post_city_init(sim));
        let until_day = 2_451_545 + 20;
        assert!(fc4_sim_set_turbo(sim, true, 100, until_day));
        assert!(fc4_sim_get_turbo(sim, &mut on, std::ptr::null_mut(), &mut until));
        assert_eq!((on, until), (true, until_day));
        for _ in 0..5 {
            fc4_sim_on_tick(sim);
        }
        assert_eq!(fc4_sim_get_julian_day(sim), until_day);
        assert_eq!(fc4_sim_take_turbo_auto_off(sim), until_day);
        assert_eq!(fc4_sim_take_turbo_auto_off(sim), 0);
        assert!(fc4_sim_get_turbo(sim, &mut on, &mut frame_ms, &mut until));
        assert_eq!((on, frame_ms, until), (false, 100, 0));
        fc4_sim_destroy(sim);
    }
}
