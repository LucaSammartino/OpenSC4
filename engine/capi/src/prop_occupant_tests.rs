// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The prop occupant through the C ABI (docs/specs/prop-occupant.md): a fake
//! game behind `fc4_prop_occupant_host` and `fc4_prop_effect_host`.

use std::cell::{Cell, RefCell};
use std::ffi::c_void;

use crate::prop_occupant::*;

/// The fake game every callback sees.
struct Game {
    in_city: Cell<bool>,
    model_state_property: Cell<i32>,
    messages: RefCell<Vec<(u32, u32)>>,
    model_states: RefCell<Vec<u8>>,
    clock: RefCell<Vec<(u8, u8)>>,
    draws: RefCell<Vec<u8>>,
    draws_made: Cell<u32>,
    removed: Cell<u32>,
    effect_handles: RefCell<Vec<u64>>,
    effect_calls: RefCell<Vec<u64>>,
    no_effect: Cell<bool>,
}

impl Default for Game {
    fn default() -> Self {
        Game {
            in_city: Cell::new(false),
            model_state_property: Cell::new(-1),
            messages: RefCell::new(Vec::new()),
            model_states: RefCell::new(Vec::new()),
            clock: RefCell::new(Vec::new()),
            draws: RefCell::new(Vec::new()),
            draws_made: Cell::new(0),
            removed: Cell::new(0),
            effect_handles: RefCell::new(Vec::new()),
            effect_calls: RefCell::new(Vec::new()),
            no_effect: Cell::new(false),
        }
    }
}

fn game<'a>(user: *mut c_void) -> &'a Game {
    // SAFETY: every callback is given `&Game` as its user pointer.
    unsafe { &*(user as *const Game) }
}

unsafe extern "C" fn cb_is_in_city(user: *mut c_void) -> i32 {
    i32::from(game(user).in_city.get())
}

unsafe extern "C" fn cb_post(user: *mut c_void, t: u32, d: u32) {
    game(user).messages.borrow_mut().push((t, d));
}

unsafe extern "C" fn cb_model_property(user: *mut c_void) -> i32 {
    game(user).model_state_property.get()
}

unsafe extern "C" fn cb_set_model_state(user: *mut c_void, s: u8) {
    game(user).model_states.borrow_mut().push(s);
}

unsafe extern "C" fn cb_set_clock(user: *mut c_void, a: u8, b: u8) {
    let g = game(user);
    let mut c = g.clock.borrow_mut();
    if a == 0 && b == 0 {
        c.clear();
    } else {
        c.push((a, b));
    }
}

unsafe extern "C" fn cb_draw(user: *mut c_void) -> u8 {
    let g = game(user);
    g.draws_made.set(g.draws_made.get() + 1);
    let mut d = g.draws.borrow_mut();
    if d.is_empty() {
        50
    } else {
        d.remove(0)
    }
}

unsafe extern "C" fn cb_remove(user: *mut c_void) -> i32 {
    let g = game(user);
    if g.in_city.get() {
        g.in_city.set(false);
        g.removed.set(g.removed.get() + 1);
        1
    } else {
        0
    }
}

unsafe extern "C" fn cb_restart_effect(user: *mut c_void, _id: u32, _n: *const u8, _len: u32) -> u64 {
    let g = game(user);
    if g.no_effect.get() {
        return 0;
    }
    let h = 0xE000 + g.effect_handles.borrow().len() as u64 + 1;
    g.effect_handles.borrow_mut().push(h);
    h
}

unsafe extern "C" fn cb_effect_slot3(user: *mut c_void, h: u64) {
    game(user).effect_calls.borrow_mut().push(h);
}

unsafe extern "C" fn cb_effect_slot4(user: *mut c_void, h: u64) {
    game(user).effect_calls.borrow_mut().push(h);
}

unsafe extern "C" fn cb_flag2(user: *mut c_void) -> i32 {
    i32::from(game(user).in_city.get())
}

fn host(user: *mut c_void) -> fc4_prop_occupant_host {
    fc4_prop_occupant_host {
        struct_size: std::mem::size_of::<fc4_prop_occupant_host>() as u32,
        user,
        is_in_city: Some(cb_is_in_city),
        post_occupant_message: Some(cb_post),
        has_model_state_property: Some(cb_model_property),
        set_model_state: Some(cb_set_model_state),
        set_clock_entry: Some(cb_set_clock),
        draw_chance: Some(cb_draw),
        remove_from_city: Some(cb_remove),
    }
}

fn effect_host(user: *mut c_void) -> fc4_prop_effect_host {
    fc4_prop_effect_host {
        struct_size: std::mem::size_of::<fc4_prop_effect_host>() as u32,
        user,
        restart_effect: Some(cb_restart_effect),
        effect_slot3: Some(cb_effect_slot3),
        effect_slot4: Some(cb_effect_slot4),
        flag2: Some(cb_flag2),
    }
}

#[test]
fn create_rejects_a_short_or_missing_table() {
    let g = Game::default();
    let user = &g as *const Game as *mut c_void;
    let mut h = host(user);
    h.struct_size = 4;
    // SAFETY: the table is deliberately too small.
    unsafe {
        assert!(fc4_prop_occupant_create(&h, std::ptr::null()).is_null());
        assert!(fc4_prop_occupant_create(std::ptr::null(), std::ptr::null()).is_null());
    }
}

#[test]
fn the_type_table_is_process_wide_and_interns_unknown_types() {
    // SAFETY: a fresh table is always valid to destroy once.
    unsafe {
        let t = fc4_prop_type_table_create();
        assert!(!t.is_null());
        assert_eq!(fc4_prop_type_table_len(t), 0);
        assert_eq!(fc4_prop_type_table_intern(t, 0x1234_5678), 0);
        assert_eq!(fc4_prop_type_table_intern(t, 0x1234_5678), 0, "same type, same index");
        assert_eq!(fc4_prop_type_table_intern(t, 0x9ABC_DEF0), 1);
        assert_eq!(fc4_prop_type_table_len(t), 2);
        assert_eq!(fc4_prop_type_table_id_at(t, 0), 0x1234_5678);
        assert_eq!(fc4_prop_type_table_id_at(t, 1), 0x9ABC_DEF0);
        fc4_prop_type_table_destroy(t);
    }
}

#[test]
fn constructor_defaults_and_the_measured_channels() {
    let g = Game::default();
    let user = &g as *const Game as *mut c_void;
    let h = host(user);
    // SAFETY: one create per destroy.
    unsafe {
        let p = fc4_prop_occupant_create(&h, std::ptr::null());
        assert!(!p.is_null());
        assert_eq!(fc4_prop_occupant_get_state(p), 0);
        assert_eq!(fc4_prop_occupant_get_random_chance(p), 100);
        assert_eq!(fc4_prop_occupant_lot_tag(p), 0xFFFF_FFFF);
        assert_eq!(fc4_prop_occupant_has_lot_tag(p), 0);
        assert_eq!(fc4_prop_occupant_prop_type_index(p), 0);
        assert_eq!(fc4_prop_occupant_refcount(p), 0);
        assert_eq!(fc4_prop_class_id(), 0x2977_AA47);
        assert_eq!(fc4_prop_occupant_type(), 0xA823_821E);
        assert_eq!(fc4_prop_no_lot_tag(), 0xFFFF_FFFF);
        assert_eq!(fc4_prop_object_size(0x2977_AA47), 0x68);
        assert_eq!(fc4_prop_object_size(0x2977_AA48), 0xA0);
        assert_eq!(fc4_prop_object_size(0x2977_AA49), 0x94);
        assert_eq!(fc4_prop_object_size(0xC9EF_B016), 0xD8);
        assert_eq!(fc4_prop_object_size(0x1), 0);
        fc4_prop_occupant_destroy(p);
    }
    // The measured channels of section 4.1.
    assert_eq!(fc4_prop_occupant_slot_count(), 24);
    assert_eq!(fc4_prop_occupant_slot_channel(12), FC4_RET_EAX as i32);
    assert_eq!(fc4_prop_occupant_slot_channel(18), FC4_RET_AL as i32);
    assert_eq!(fc4_prop_occupant_slot_channel(14), FC4_RET_VOID as i32);
    assert_eq!(fc4_prop_occupant_slot_channel(1), FC4_RET_VIA_CALL as i32);
    assert_eq!(fc4_prop_occupant_slot_channel(8), FC4_RET_EAX_OR_VOID as i32);
    assert_eq!(fc4_prop_occupant_slot_channel(24), -1);
    assert_eq!(fc4_prop_occupant_query_interface(0xE979_3A65), 0);
    assert_eq!(fc4_prop_occupant_query_interface(0x4522_94AA), 4);
    assert_eq!(fc4_prop_occupant_query_interface(0xDEAD_BEEF), 8);
}

#[test]
fn refcounting_lives_at_plus_0x14_and_the_slots_round_trip() {
    let g = Game::default();
    let user = &g as *const Game as *mut c_void;
    let h = host(user);
    // SAFETY: one create per destroy.
    unsafe {
        let p = fc4_prop_occupant_create(&h, std::ptr::null());
        assert_eq!(fc4_prop_occupant_add_ref(p), 1);
        assert_eq!(fc4_prop_occupant_add_ref(p), 2);
        assert_eq!(fc4_prop_occupant_release(p), 1);
        assert_eq!(fc4_prop_occupant_refcount(p), 1);

        fc4_prop_occupant_set_prop_type(p, 0x1111_2222);
        assert_eq!(fc4_prop_occupant_get_prop_type(p), 0x1111_2222);
        // The C return of slot 10 is the al flag, always 1.
        assert_eq!(fc4_prop_occupant_set_orientation(p, 7), 1);
        assert_eq!(fc4_prop_occupant_get_orientation(p), 3, "clamped to 0..3");
        assert_eq!(fc4_prop_occupant_set_orientation(p, -1), 1);
        assert_eq!(fc4_prop_occupant_get_orientation(p), 0);

        fc4_prop_occupant_set_random_chance(p, 37);
        assert_eq!(fc4_prop_occupant_get_random_chance(p), 37);
        fc4_prop_occupant_set_prop_usage(p, 2);
        assert_eq!(fc4_prop_occupant_get_prop_usage(p), 2);
        assert_eq!(fc4_prop_add_usage(1), 1);
        assert_eq!(fc4_prop_add_usage(2), 2);
        assert_eq!(fc4_prop_add_usage(3), 3);
        assert_eq!(fc4_prop_add_usage(4), 4);
        assert_eq!(fc4_prop_add_usage(9), 0);

        fc4_prop_occupant_set_lot_tag(p, 0x1234);
        assert_eq!(fc4_prop_occupant_lot_tag(p), 0x1234);
        assert_eq!(fc4_prop_occupant_has_lot_tag(p), 1);

        let lo = [0.5f32, 1.5, 2.5];
        let hi = [3.5f32, 4.5, 5.5];
        fc4_prop_occupant_set_bounding_box(p, lo.as_ptr(), hi.as_ptr());
        let mut glo = [0.0f32; 3];
        let mut ghi = [0.0f32; 3];
        fc4_prop_occupant_get_bounding_box(p, glo.as_mut_ptr(), ghi.as_mut_ptr());
        assert_eq!(glo, lo);
        assert_eq!(ghi, hi);

        fc4_prop_occupant_set_time_of_day_range(p, 18.0, 8.0);
        let (mut a, mut b) = (0.0f32, 0.0f32);
        fc4_prop_occupant_get_time_of_day_range(p, &mut a, &mut b);
        assert!((a - 18.0).abs() < 1e-5 && (b - 8.0).abs() < 1e-5);
        assert_eq!(g.clock.borrow().as_slice(), &[(180, 80)]);
        fc4_prop_occupant_set_time_of_day_range(p, 0.0, 0.0);
        assert!(g.clock.borrow().is_empty(), "(0, 0) registers no clock entry");

        fc4_prop_occupant_destroy(p);
    }
}

#[test]
fn set_state_posts_only_in_the_city_and_the_timer_draw_is_once() {
    let g = Game::default();
    let user = &g as *const Game as *mut c_void;
    let h = host(user);
    // SAFETY: one create per destroy.
    unsafe {
        let p = fc4_prop_occupant_create(&h, std::ptr::null());
        g.in_city.set(true);
        fc4_prop_occupant_set_state(p, 0);
        assert!(g.messages.borrow().is_empty(), "s == +0x63 does nothing");
        fc4_prop_occupant_set_state(p, 1);
        assert_eq!(g.messages.borrow().as_slice(), &[(0xE9C8_6DDB, 1)]);
        assert_eq!(fc4_prop_occupant_get_state(p), 1);

        g.in_city.set(false);
        g.messages.borrow_mut().clear();
        fc4_prop_occupant_set_state(p, 2);
        assert!(g.messages.borrow().is_empty());
        assert_eq!(fc4_prop_occupant_get_state(p), 2);

        // The timer transition draws exactly once (quirk 7).
        g.in_city.set(true);
        g.messages.borrow_mut().clear();
        g.draws.borrow_mut().push(50);
        g.draws_made.set(0);
        fc4_prop_occupant_set_timer_state(p, 0x3, 1);
        assert_eq!(g.draws_made.get(), 0, "below 0xF: no draw");
        fc4_prop_occupant_set_timer_state(p, 0xC, 1);
        assert_eq!(g.draws_made.get(), 1, "exactly one draw on the transition");
        assert_eq!(fc4_prop_occupant_get_timer_bitfield(p), 0x0F);
        assert_eq!(fc4_prop_occupant_get_state(p), 0);
        assert_eq!(fc4_prop_occupant_get_timer_state(p, 0x8), 1);
        assert_eq!(fc4_prop_occupant_get_timer_state(p, 0x10), 0);

        fc4_prop_occupant_destroy(p);
    }
}

#[test]
fn the_record_round_trips_and_the_legacy_timer_bits_apply() {
    let g = Game::default();
    let user = &g as *const Game as *mut c_void;
    let h = host(user);
    // SAFETY: buffers and handles are valid for the calls.
    unsafe {
        let p = fc4_prop_occupant_create(&h, std::ptr::null());
        fc4_prop_occupant_set_prop_type(p, 0x1111_2222);
        fc4_prop_occupant_set_random_chance(p, 50);
        fc4_prop_occupant_set_prop_usage(p, 2);
        fc4_prop_occupant_set_lot_tag(p, 0xABCD_1234);
        fc4_prop_occupant_set_timer_state(p, 0x7, 1);

        let mut buf = vec![0u8; 512];
        let mut written = 0u32;
        assert_eq!(
            fc4_prop_occupant_write_record(p, buf.as_mut_ptr(), buf.len() as u32, &mut written),
            1
        );
        assert!(written > 0);
        assert_eq!(
            fc4_prop_occupant_read_record(p, buf.as_ptr(), written),
            FC4_PROP_RECORD_OK
        );

        let mut pt = 0u32;
        let mut tb = 0u8;
        let mut tag = 0u32;
        assert_eq!(
            fc4_prop_record_read_summary(buf.as_ptr(), written, &mut pt, &mut tb, &mut tag),
            6,
            "the record's version field is 6"
        );
        assert_eq!(pt, 0x1111_2222);
        assert_eq!(tag, 0xABCD_1234);

        // Three bytes: the version reads, the base does not. `lr` §10.1 quirk:
        // a failing base read reports success without reading more.
        assert_eq!(
            fc4_prop_occupant_read_record(p, buf.as_ptr(), 3),
            FC4_PROP_RECORD_BASE_FAILED
        );
        // Cut inside the payload: the method returns false.
        assert_eq!(
            fc4_prop_occupant_read_record(p, buf.as_ptr(), written - 2),
            FC4_PROP_RECORD_TRUNCATED
        );

        // Versions.
        let mut vers = [0u16; 4];
        assert_eq!(fc4_prop_record_versions(vers.as_mut_ptr()), 1);
        assert_eq!(vers, [6, 3, 0x0F, 4]);

        fc4_prop_occupant_destroy(p);
    }
}

#[test]
fn removal_is_once_and_pv2_classifies_the_corpus_categories() {
    let g = Game::default();
    let user = &g as *const Game as *mut c_void;
    let h = host(user);
    // SAFETY: one create per destroy.
    unsafe {
        let p = fc4_prop_occupant_create(&h, std::ptr::null());
        g.in_city.set(true);
        assert_eq!(fc4_prop_occupant_shutdown(p), 1);
        assert_eq!(fc4_prop_occupant_shutdown(p), 0, "a second removal is a no-op");
        assert_eq!(g.removed.get(), 1);
        fc4_prop_occupant_destroy(p);
    }

    // PV2 (section 5.3): the four outcomes and the corpus split.
    let ids = [0x11u32, 0x22, 0x33, 0x22];
    // SAFETY: `ids` is valid for its length.
    unsafe {
        assert_eq!(
            fc4_prop_tag_classify(0x22, ids.as_ptr(), ids.len() as u32, 0),
            FC4_TAG_MATCHED
        );
        assert_eq!(
            fc4_prop_tag_classify(0x99, ids.as_ptr(), ids.len() as u32, 0),
            FC4_TAG_CONFIG_LACKS_TAG
        );
        assert_eq!(
            fc4_prop_tag_classify(0x22, std::ptr::null(), 0, 0),
            FC4_TAG_NO_CONFIGURATION
        );
        assert_eq!(
            fc4_prop_tag_classify(0x22, std::ptr::null(), 0, 1),
            FC4_TAG_UNKNOWN_CONFIGURATION
        );
        assert_eq!(
            fc4_prop_tag_classify(0xFFFF_FFFF, ids.as_ptr(), ids.len() as u32, 0),
            FC4_TAG_NONE
        );
    }
    assert_eq!(fc4_prop_tag_corpus(0), 147_162);
    assert_eq!(fc4_prop_tag_corpus(1), 40_830);
    assert_eq!(fc4_prop_tag_corpus(2), 40_794);
    assert_eq!(fc4_prop_tag_corpus(3), 36);
    assert_eq!(fc4_prop_tag_corpus(4), 0);
    assert_eq!(fc4_prop_tag_corpus(5), 2770);
    assert_eq!(fc4_prop_tag_corpus(9), 0);
}

#[test]
fn the_visual_effect_attaches_and_drops_its_name_while_live() {
    let g = Game::default();
    let user = &g as *const Game as *mut c_void;
    let h = effect_host(user);
    // SAFETY: one create per destroy; the offset buffer has two u32s.
    unsafe {
        let e = fc4_prop_effect_create(&h, std::ptr::null());
        assert!(!e.is_null());
        let name = b"pothole";
        assert_eq!(fc4_prop_effect_set_type_name(e, name.as_ptr(), name.len() as u32), 1);
        assert_eq!(fc4_prop_effect_start(e, 1), 1, "an effect object exists at +0x6C");
        assert_eq!(fc4_prop_effect_is_started(e), 1);
        assert_eq!(fc4_prop_effect_stop(e, 1), 1);
        assert_eq!(fc4_prop_effect_is_started(e), 0);
        assert_eq!(fc4_prop_effect_stop(e, 1), 0, "started is clear");
        assert_eq!(fc4_prop_effect_is_live(e), 1);
        assert_eq!(fc4_prop_effect_should_drop_name(e), 1);

        let mut offs = [0u32; 2];
        assert_eq!(fc4_prop_effect_sub_object_offsets(offs.as_mut_ptr()), 1);
        assert_eq!(offs, [4, 0x0C], "AsPropOccupant +4, AsOccupant +0x0C");

        let mut buf = vec![0u8; 512];
        let mut written = 0u32;
        assert_eq!(
            fc4_prop_effect_write_record(e, buf.as_mut_ptr(), buf.len() as u32, &mut written),
            1
        );
        let mut name_len = 0xFFFF_FFFF;
        assert_eq!(fc4_prop_effect_read_record(e, buf.as_ptr(), written, &mut name_len), 1);
        assert_eq!(name_len, 0, "the name is dropped while the effect is live");
        fc4_prop_effect_destroy(e);
    }

    // With no effect object StartEffect still sets `started` and answers 0.
    g.no_effect.set(true);
    let h2 = effect_host(user);
    // SAFETY: one create per destroy.
    unsafe {
        let e = fc4_prop_effect_create(&h2, std::ptr::null());
        let name = b"x";
        let _ = fc4_prop_effect_set_type_name(e, name.as_ptr(), name.len() as u32);
        assert_eq!(fc4_prop_effect_start(e, 1), 0);
        assert_eq!(fc4_prop_effect_is_started(e), 1);
        assert_eq!(fc4_prop_effect_stop(e, 1), 0);
        fc4_prop_effect_destroy(e);
    }
}
