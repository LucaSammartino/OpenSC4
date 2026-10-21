// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The layer-only disaster layer (disasters.md §2.2-§2.7).

use std::ffi::c_void;

use crate::disaster_layer::*;

#[derive(Default)]
struct Rec {
    now: u64,
    log: Vec<String>,
    refs: i64,
    follow: bool,
    in_manager: bool,
    cell: (i32, i32),
    counts: Option<(u32, u32)>,
    reenter: usize,
}

unsafe fn r(u: *mut c_void) -> &'static mut Rec {
    &mut *(u as *mut Rec)
}
unsafe extern "C" fn now(u: *mut c_void) -> u64 {
    r(u).now
}
unsafe extern "C" fn pause(u: *mut c_void) {
    r(u).log.push("pause".into());
    let d = r(u).reenter as *mut fc4_dlayer;
    if !d.is_null() {
        // A re-entrant call is refused, not deadlocked.
        assert_eq!(fc4_dlayer_ongoing(d), 0);
    }
}
unsafe extern "C" fn resume(u: *mut c_void) {
    r(u).log.push("resume".into());
}
unsafe extern "C" fn save_off(u: *mut c_void) {
    r(u).log.push("save_off".into());
}
unsafe extern "C" fn save_on(u: *mut c_void) {
    r(u).log.push("save_on".into());
}
unsafe extern "C" fn started(u: *mut c_void, n: u32, x: i32, z: i32, i: u64) {
    r(u).log.push(format!("started {n} {x} {z} {i}"));
}
unsafe extern "C" fn ended(u: *mut c_void) {
    r(u).log.push("ended".into());
}
unsafe extern "C" fn sub(u: *mut c_void, on: bool) {
    r(u).log.push(format!("sub {on}"));
}
unsafe extern "C" fn add_ref(u: *mut c_void, _: u64) {
    r(u).refs += 1;
}
unsafe extern "C" fn release(u: *mut c_void, _: u64) {
    r(u).refs -= 1;
}
unsafe extern "C" fn follow(u: *mut c_void) -> bool {
    r(u).follow
}
unsafe extern "C" fn go_to(u: *mut c_void, x: i32, z: i32, zoom: u32) {
    r(u).log.push(format!("go {x} {z} {zoom}"));
}
unsafe extern "C" fn in_mgr(u: *mut c_void, _: u64) -> bool {
    r(u).in_manager
}
unsafe extern "C" fn occ_cell(u: *mut c_void, _: u64, x: *mut i32, z: *mut i32) -> bool {
    *x = r(u).cell.0;
    *z = r(u).cell.1;
    true
}
unsafe extern "C" fn inst_loc(_: *mut c_void, i: u64, x: *mut i32, z: *mut i32) -> bool {
    *x = i as i32 * 10;
    *z = i as i32 * 10 + 1;
    i != 99
}
unsafe extern "C" fn counts(u: *mut c_void, f: *mut u32, p: *mut u32) -> bool {
    match r(u).counts {
        Some((a, b)) => {
            *f = a;
            *p = b;
            true
        }
        None => false,
    }
}

fn make(rec: &mut Rec) -> *mut fc4_dlayer {
    let h = fc4_dlayer_host {
        struct_size: std::mem::size_of::<fc4_dlayer_host>() as u32,
        user: rec as *mut Rec as *mut c_void,
        now_ms: Some(now),
        emergency_pause: Some(pause),
        emergency_resume: Some(resume),
        disable_save: Some(save_off),
        enable_save: Some(save_on),
        post_started: Some(started),
        post_ended: Some(ended),
        subscribe: Some(sub),
        add_ref: Some(add_ref),
        release: Some(release),
        follow_disasters: Some(follow),
        go_to: Some(go_to),
        occupant_in_manager: Some(in_mgr),
        occupant_cell: Some(occ_cell),
        instance_location: Some(inst_loc),
        dispatch_counts: Some(counts),
    };
    unsafe { fc4_dlayer_create(&h) }
}

#[test]
fn enable_nests_and_init_once() {
    let mut rec = Rec::default();
    let d = make(&mut rec);
    unsafe {
        assert!(fc4_dlayer_init(d));
        assert!(!fc4_dlayer_init(d));
        assert!(!fc4_dlayer_enable(d, false));
        assert!(!fc4_dlayer_enable(d, false));
        assert!(!fc4_dlayer_enable(d, true));
        assert!(fc4_dlayer_enable(d, true));
        assert!(fc4_dlayer_enable(d, true)); // not below 0
        assert!(fc4_dlayer_enabled(d));
        fc4_dlayer_destroy(d);
    }
}

#[test]
fn begin_end_period_and_assessment() {
    let mut rec = Rec {
        counts: Some((4, 2)),
        ..Default::default()
    };
    let d = make(&mut rec);
    unsafe {
        fc4_dlayer_init(d);
        assert!(!fc4_dlayer_ended(d)); // nothing ongoing
        fc4_dlayer_register(d, 7, 3, 4, 0, 3, false);
        rec.now = 1000;
        fc4_dlayer_begun(d, 7);
        fc4_dlayer_begun(d, 8);
        assert_eq!(fc4_dlayer_ongoing(d), 2);
        // Instance 7 resolves through GetLocation (70, 71); 8 has no entry.
        assert_eq!(
            rec.log,
            ["pause", "sub true", "save_off", "started 1 70 71 7", "started 2 0 0 8"]
        );
        assert!(fc4_dlayer_occupant_removed(d, FC4_DLAYER_OCC_BUILDING, true, 500, 0x1 | 0x20 | 0x400));
        assert!(fc4_dlayer_occupant_removed(d, FC4_DLAYER_OCC_NETWORK, false, 0, 0));
        assert!(fc4_dlayer_occupant_removed(d, FC4_DLAYER_OCC_OTHER, true, 7, 0));
        // Fire dispatch 11 runs 2 s, police 12 gets only a stop (0 s).
        fc4_dlayer_dispatch(d, 0xE946_71EC, 1, 11);
        rec.now = 3000;
        fc4_dlayer_dispatch(d, 0xE946_71EB, 5, 11);
        fc4_dlayer_dispatch(d, 0x8946_71F6, 0, 12);
        assert!(!fc4_dlayer_dispatch(d, 0xE946_71EC, 2, 13));
        rec.now = 5000;
        rec.log.clear();
        fc4_dlayer_ended(d);
        assert!(rec.log.is_empty());
        fc4_dlayer_ended(d);
        assert_eq!(rec.log, ["save_on", "resume", "sub false", "ended"]);
        let mut out = fc4_dlayer_record::default();
        assert!(fc4_dlayer_record(d, &mut out));
        assert_eq!(out.building_count, 1);
        assert_eq!(out.total_cost, 500 + 10 + 7);
        assert_eq!(out.kind_count, [1, 0, 0, 1]);
        assert_eq!(out.kind_cost, [500, 0, 0, 500]);
        assert_eq!(out.special_count, 1);
        // 2 s / (4 x 4 s)
        assert_eq!(out.fire_effectiveness, 2.0 / 16.0);
        assert_eq!(out.police_effectiveness, 0.0);
        // Not assessing: ignored.
        assert!(!fc4_dlayer_occupant_removed(d, FC4_DLAYER_OCC_BUILDING, true, 1, 0));
        assert!(fc4_dlayer_reset_cost(d));
        fc4_dlayer_record(d, &mut out);
        assert_eq!((out.building_count, out.total_cost, out.special_count), (0, 0, 0));
        assert_eq!(out.fire_effectiveness, 2.0 / 16.0);
        fc4_dlayer_destroy(d);
    }
}

#[test]
fn missing_simulator_writes_neither_value() {
    let mut rec = Rec::default();
    let d = make(&mut rec);
    unsafe {
        fc4_dlayer_init(d);
        fc4_dlayer_begun(d, 1);
        fc4_dlayer_dispatch(d, 0xE946_71EC, 1, 1);
        rec.now = 1000;
        fc4_dlayer_ended(d);
        let mut out = fc4_dlayer_record::default();
        fc4_dlayer_record(d, &mut out);
        assert_eq!((out.fire_effectiveness, out.police_effectiveness), (0.0, 0.0));
        fc4_dlayer_destroy(d);
    }
}

#[test]
fn locations_resolve_cursor_and_release() {
    let mut rec = Rec {
        in_manager: true,
        cell: (5, 6),
        ..Default::default()
    };
    let d = make(&mut rec);
    unsafe {
        fc4_dlayer_init(d);
        fc4_dlayer_register(d, 1, 0, 0, 100, 3, false);
        fc4_dlayer_register(d, 2, 1, 2, 0, 2, false);
        fc4_dlayer_register(d, 1, 9, 9, 101, 3, false);
        assert_eq!(rec.refs, 2);
        assert_eq!(fc4_dlayer_location_count(d), 3);
        let mut l = fc4_dlayer_location::default();
        assert!(fc4_dlayer_location_at(d, 0, &mut l));
        assert_eq!((l.instance, l.occupant, l.x, l.z, l.zoom), (1, 100, 0, 0, 3));
        assert!(!fc4_dlayer_location_at(d, 3, &mut l));
        // Slot 12 resolves the newest entry of instance 1 (index 2).
        let (mut x, mut z) = (0, 0);
        assert!(fc4_dlayer_most_recent(d, 1, &mut x, &mut z));
        assert_eq!((x, z), (5, 6));
        fc4_dlayer_location_at(d, 2, &mut l);
        assert_eq!((l.x, l.z), (5, 6));
        // Gone occupant: unchanged.
        rec.in_manager = false;
        rec.cell = (7, 7);
        assert!(fc4_dlayer_most_recent(d, 1, &mut x, &mut z));
        assert_eq!((x, z), (5, 6));
        assert!(!fc4_dlayer_most_recent(d, 3, &mut x, &mut z));
        // The cursor walks back from the newest, wrapping.
        rec.log.clear();
        fc4_dlayer_go_to_next(d);
        fc4_dlayer_go_to_next(d);
        fc4_dlayer_go_to_next(d);
        fc4_dlayer_go_to_next(d);
        assert_eq!(rec.log, ["go 5 6 3", "go 20 21 2", "go 0 0 3", "go 5 6 3"]);
        // Focus with the preference: GoToNextDisaster on the new entry.
        rec.follow = true;
        rec.log.clear();
        fc4_dlayer_register(d, 99, 4, 4, 0, 1, true);
        assert_eq!(rec.log, ["go 4 4 1"]);
        fc4_dlayer_unregister_by_occupant(d, 1, 101);
        assert_eq!(rec.refs, 1);
        fc4_dlayer_unregister(d, 2);
        assert_eq!(fc4_dlayer_location_count(d), 2);
        fc4_dlayer_begun(d, 1);
        assert!(fc4_dlayer_assessing(d));
        rec.log.clear();
        assert!(fc4_dlayer_shutdown(d));
        assert_eq!(rec.refs, 0);
        assert_eq!(fc4_dlayer_location_count(d), 0);
        assert_eq!(rec.log, ["sub false"]);
        assert!(!fc4_dlayer_initialised(d));
        fc4_dlayer_destroy(d);
    }
}

#[test]
fn reentrant_call_is_refused() {
    let mut rec = Rec::default();
    let d = make(&mut rec);
    rec.reenter = d as usize;
    unsafe {
        fc4_dlayer_init(d);
        fc4_dlayer_begun(d, 1);
        assert_eq!(fc4_dlayer_refused(d), 1);
        assert_eq!(fc4_dlayer_ongoing(d), 1);
        fc4_dlayer_destroy(d);
    }
}

#[test]
fn null_and_short_host() {
    unsafe {
        assert!(fc4_dlayer_create(std::ptr::null()).is_null());
        let mut rec = Rec::default();
        let d = make(&mut rec);
        assert!(!fc4_dlayer_record(d, std::ptr::null_mut()));
        fc4_dlayer_destroy(d);
        assert_eq!(fc4_dlayer_ongoing(std::ptr::null_mut()), 0);
    }
}
