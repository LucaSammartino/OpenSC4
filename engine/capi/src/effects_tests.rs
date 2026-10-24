// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The effects manager through the C ABI (docs/specs/effects.md): a fake
//! game behind `fc4_effects_host`.

use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_void, CString};

use crate::effects::*;
use crate::terrain::fc4_brush_params;

#[test]
fn effects_visual_capture_refuses_missing_callback_and_bad_resource() {
    unsafe extern "C" fn unreadable(_: *mut c_void, _: u32, _: *mut u8, _: usize) -> bool {
        false
    }
    let bytes = fc4_core::effects::shipped::volcano_directory().to_bytes();
    let mut counts = [99; 3];
    unsafe {
        for (length, callback) in [
            (bytes.len(), None),
            (1, Some(unreadable as unsafe extern "C" fn(_, _, _, _) -> _)),
            (bytes.len(), Some(unreadable)),
        ] {
            assert!(!fc4_effects_verify_visuals(
                bytes.as_ptr(),
                length,
                0x100,
                callback,
                std::ptr::null_mut(),
                counts.as_mut_ptr()
            ));
            assert_eq!(counts, [0; 3]);
        }
    }
}

#[test]
fn effects_record_names_native_case_membership_and_refusals() {
    unsafe extern "C" fn native(user: *mut c_void, name: *const c_char) -> u32 {
        let name = std::ffi::CStr::from_ptr(name).to_str().unwrap();
        let inject_difference = !user.is_null() && name == "MiXeD";
        u32::from(
            !inject_difference && (name.eq_ignore_ascii_case("mixed") || name == "disabled" || name == "DISABLED"),
        )
    }
    let mut directory = fc4_core::effects::shipped::volcano_directory();
    directory.names = vec![("MiXeD".into(), 0), ("disabled".into(), 0)];
    // HasVisualEffect means name existence even if priority disqualifies it.
    directory.effects[0].priority = 0;
    directory.reindex();
    let bytes = directory.to_bytes();
    let mut counts = [99; 3];
    unsafe {
        assert!(fc4_effects_verify_names(
            bytes.as_ptr(),
            bytes.len(),
            Some(native),
            std::ptr::null_mut(),
            counts.as_mut_ptr()
        ));
        assert_eq!(counts, [2, 9, 0]);
        assert!(fc4_effects_verify_names(
            bytes.as_ptr(),
            bytes.len(),
            Some(native),
            counts.as_mut_ptr().cast(),
            counts.as_mut_ptr()
        ));
        assert_eq!(counts, [2, 9, 1]);
        assert!(!fc4_effects_verify_names(
            bytes.as_ptr(),
            1,
            Some(native),
            std::ptr::null_mut(),
            counts.as_mut_ptr()
        ));
        assert_eq!(counts, [0; 3]);
        assert!(!fc4_effects_verify_names(
            bytes.as_ptr(),
            bytes.len(),
            None,
            std::ptr::null_mut(),
            counts.as_mut_ptr()
        ));
        assert_eq!(counts, [0; 3]);
    }
}

#[derive(Default)]
struct Game {
    log: RefCell<Vec<String>>,
    brushes: Cell<u32>,
    no_sim: Cell<bool>,
    fx: Cell<usize>,
    reentrant: Cell<Option<bool>>,
}

impl Game {
    fn user(&self) -> *mut c_void {
        self as *const Game as *mut c_void
    }
    fn log(&self, s: impl Into<String>) {
        self.log.borrow_mut().push(s.into());
    }
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
}

unsafe fn g<'a>(u: *mut c_void) -> &'a Game {
    &*(u as *const Game)
}

unsafe extern "C" fn simulator(u: *mut c_void, d: *mut f32) -> bool {
    *d = 1.0;
    !g(u).no_sim.get()
}
unsafe extern "C" fn hidden_pause(u: *mut c_void) {
    g(u).log("hidden_pause");
}
unsafe extern "C" fn hidden_resume(u: *mut c_void) {
    g(u).log("hidden_resume");
}
unsafe extern "C" fn city_cells(_: *mut c_void, x: *mut i32, z: *mut i32) {
    *x = 64;
    *z = 64;
}
unsafe extern "C" fn in_bounds(_: *mut c_void, x: f32, z: f32) -> bool {
    (0.0..1024.0).contains(&x) && (0.0..1024.0).contains(&z)
}
unsafe extern "C" fn apply_brush(u: *mut c_void, id: u32, p: *mut fc4_brush_params) -> bool {
    let game = g(u);
    game.brushes.set(game.brushes.get() + 1);
    let p = &*p;
    game.log(format!(
        "brush {id:08X} {} {} L{} s{} r{} d{} red{}",
        p.from_x, p.from_z, p.level, p.strength_scale, p.radius_scale, p.data4, p.redisplay
    ));
    // A call back into the manager from a callback is refused.
    if game.fx.get() != 0 && game.reentrant.get().is_none() {
        game.reentrant
            .set(Some(fc4_effects_is_started(game.fx.get() as *const fc4_effects, 1)));
    }
    true
}
unsafe extern "C" fn begin_long(u: *mut c_void) {
    g(u).log("begin_long");
}
unsafe extern "C" fn end_long(u: *mut c_void) {
    g(u).log("end_long");
}
unsafe extern "C" fn ignite(u: *mut c_void, x: i32, z: i32, r0: i32, r1: i32) {
    g(u).log(format!("ignite {x} {z} {r0} {r1}"));
}
unsafe extern "C" fn cause(u: *mut c_void, c: u32) {
    g(u).log(format!("cause {c}"));
}
unsafe extern "C" fn demolish(
    u: *mut c_void,
    r: *const fc4_effects_rect,
    flags: u32,
    dezone: bool,
    f: *const fc4_effects_demolish_filter,
) {
    let (r, f) = (&*r, &*f);
    g(u).log(format!(
        "demolish {} {} {} {} {flags} {dezone} {} {}",
        r.x0, r.z0, r.x1, r.z1, f.no_networks, f.no_flora
    ));
}
unsafe extern "C" fn post(u: *mut c_void, m: *const fc4_effects_post) {
    let m = &*m;
    g(u).log(format!("post {:08X} {} {} {}", m.msg_type, m.data1, m.data2, m.data3));
}

fn host(game: &Game) -> fc4_effects_host {
    fc4_effects_host {
        struct_size: std::mem::size_of::<fc4_effects_host>() as u32,
        user: game.user(),
        simulator: Some(simulator),
        sim_pause: None,
        sim_resume: None,
        sim_hidden_pause: Some(hidden_pause),
        sim_hidden_resume: Some(hidden_resume),
        clock_pause: None,
        city_cells: Some(city_cells),
        location_in_bounds: Some(in_bounds),
        apply_brush: Some(apply_brush),
        begin_long_terrain_operation: Some(begin_long),
        end_long_terrain_operation: Some(end_long),
        ignite_cell_radius: Some(ignite),
        set_fire_cause: Some(cause),
        toxic_attempt_start: None,
        demolish_region: Some(demolish),
        extinguish_cell: None,
        post: Some(post),
        render_component_active: None,
    }
}

fn cs(s: &str) -> CString {
    CString::new(s).unwrap()
}

struct Fx(*mut fc4_effects, *mut fc4_effects_directory);

impl Drop for Fx {
    fn drop(&mut self) {
        unsafe {
            fc4_effects_destroy(self.0);
            fc4_effects_directory_destroy(self.1);
        }
    }
}

fn setup(game: &Game) -> Fx {
    unsafe {
        let h = host(game);
        let fx = fc4_effects_create(&h, 0x1234_5678);
        assert!(!fx.is_null());
        let d = fc4_effects_directory_volcano();
        assert!(!d.is_null());
        assert!(fc4_effects_set_directory(fx, d));
        Fx(fx, d)
    }
}

#[test]
fn create_rejects_a_short_host() {
    let game = Game::default();
    let mut h = host(&game);
    h.struct_size -= 4;
    unsafe {
        assert!(fc4_effects_create(&h, 1).is_null());
        assert!(fc4_effects_create(std::ptr::null(), 1).is_null());
        // NULL handles are harmless.
        fc4_effects_tick(std::ptr::null_mut(), 1000);
        assert!(!fc4_effects_finish_hook(std::ptr::null_mut()));
        fc4_effects_destroy(std::ptr::null_mut());
        fc4_effects_directory_destroy(std::ptr::null_mut());
    }
}

#[test]
fn directory_counts_find_and_round_trip() {
    unsafe {
        let d = fc4_effects_directory_volcano();
        let mut c = fc4_effects_directory_counts::default();
        assert!(fc4_effects_directory_get_counts(d, &mut c));
        assert_eq!((c.brushes, c.games), (25, 57));
        assert!(c.effects >= 15);
        assert_eq!(fc4_effects_directory_find(d, cs("VOLCANO_CONTROL").as_ptr(), 5), 9);
        assert_eq!(fc4_effects_directory_find(d, cs("volcano_control").as_ptr(), 0), -1);
        assert_eq!(
            fc4_effects_directory_find(d, cs("nothing").as_ptr(), 5),
            FC4_EFFECTS_UNKNOWN
        );
        let n = fc4_effects_directory_write(d, std::ptr::null_mut(), 0);
        assert!(n > 0);
        let mut buf = vec![0u8; n];
        assert_eq!(fc4_effects_directory_write(d, buf.as_mut_ptr(), n), n);
        let e = fc4_effects_directory_parse(buf.as_ptr(), buf.len());
        assert!(!e.is_null());
        let mut c2 = fc4_effects_directory_counts::default();
        assert!(fc4_effects_directory_get_counts(e, &mut c2));
        assert_eq!(c, c2);
        assert!(fc4_effects_directory_parse(buf.as_ptr(), 10).is_null());
        fc4_effects_directory_destroy(e);
        fc4_effects_directory_destroy(d);
    }
}

#[test]
fn one_shot_crater_through_the_host() {
    let game = Game::default();
    let fx = setup(&game);
    unsafe {
        let mut id = 0;
        assert!(fc4_effects_create_effect(fx.0, cs("meteorcrater_b").as_ptr(), &mut id));
        assert!(id >= 1);
        assert!(fc4_effects_set_position(fx.0, id, 320.0, 0.0, 480.0));
        assert!(fc4_effects_start(fx.0, id, 0));
        game.take();
        fc4_effects_tick(fx.0, 33_333);
        assert_eq!(
            game.take(),
            [
                "brush 8A33E17A 320 480 L3 s6 r0.7 d3 red0",
                "brush 8A33E17A 320 480 L3 s6 r0.7 d3 red1"
            ]
        );
        let mut b = fc4_effects_brush_state::default();
        assert!(fc4_effects_get_brush(fx.0, id, 0, &mut b));
        assert!(!b.active && !b.long_op && b.one_shot);
        assert_eq!(b.applications, 2);
        let mut name = [0 as c_char; 8];
        assert_eq!(fc4_effects_get_name(fx.0, id, name.as_mut_ptr(), name.len()), 14);
        assert_eq!(std::ffi::CStr::from_ptr(name.as_ptr()).to_str().unwrap(), "meteorc");
    }
}

#[test]
fn volcano_ends_through_the_hook_with_one_post() {
    let game = Game::default();
    let fx = setup(&game);
    unsafe {
        let mut id = 0;
        assert!(fc4_effects_create_effect(fx.0, cs("volcano_control").as_ptr(), &mut id));
        let mut child = 0;
        assert!(fc4_effects_get_child(fx.0, id, 0, &mut child));
        let mut name = [0 as c_char; 32];
        fc4_effects_get_name(fx.0, child, name.as_mut_ptr(), name.len());
        assert_eq!(
            std::ffi::CStr::from_ptr(name.as_ptr()).to_str().unwrap(),
            "volcanocreator4a"
        );
        assert!(fc4_effects_set_end_message(fx.0, id, 0xCA49_3C1D, 0x77));
        assert!(fc4_effects_set_position(fx.0, id, 500.0, 0.0, 500.0));
        assert!(fc4_effects_start(fx.0, id, 0));
        game.fx.set(fx.0 as usize);
        let mut frames = 0;
        while !game.log.borrow().iter().any(|l| l.starts_with("post")) && frames < 200 {
            fc4_effects_tick(fx.0, 33_333);
            assert!(!fc4_effects_draw_pass(fx.0));
            assert!(fc4_effects_finish_hook(fx.0));
            frames += 1;
        }
        assert!(frames < 60, "{frames}");
        // The callback's call back into the manager was refused.
        assert_eq!(game.reentrant.get(), Some(false));
        let log = game.take();
        assert_eq!(log.iter().filter(|l| *l == "begin_long").count(), 1);
        assert_eq!(log.iter().filter(|l| *l == "end_long").count(), 2);
        assert_eq!(
            log.iter().filter(|l| l.starts_with("post")).collect::<Vec<_>>(),
            ["post CA493C1D 119 0 0"]
        );
        assert!(!fc4_effects_is_started(fx.0, id));
        let mut s = fc4_effects_state::default();
        assert!(fc4_effects_get_state(fx.0, &mut s));
        assert_eq!(
            (s.select_draws, s.end_posts, s.finish_mode),
            (1, 1, FC4_EFFECTS_FINISH_HOOK)
        );
        assert!(fc4_effects_release(fx.0, id));
        fc4_effects_finish_hook(fx.0);
        assert!(fc4_effects_get_state(fx.0, &mut s));
        assert_eq!(s.effect_count, 0);
        let mut info = fc4_effect_info::default();
        assert!(!fc4_effects_get_effect(fx.0, id, &mut info));
    }
}

#[test]
fn draw_pass_mode_and_game_record() {
    let game = Game::default();
    let fx = setup(&game);
    unsafe {
        assert!(fc4_effects_set_finish_mode(fx.0, FC4_EFFECTS_FINISH_DRAW_PASS));
        assert!(!fc4_effects_set_finish_mode(fx.0, 7));
        assert!(!fc4_effects_finish_hook(fx.0));
        assert!(fc4_effects_draw_pass(fx.0));
        // volcanocreator4a by index: its game record pauses nothing and
        // demolishes at the vent once.
        let mut id = 0;
        assert!(fc4_effects_create_effect(
            fx.0,
            cs("volcanocreator4a").as_ptr(),
            &mut id
        ));
        fc4_effects_set_position(fx.0, id, 500.0, 0.0, 500.0);
        fc4_effects_start(fx.0, id, 0);
        fc4_effects_tick(fx.0, 33_333);
        let mut gs = fc4_effects_game_state::default();
        assert!(fc4_effects_get_game(fx.0, id, 0, &mut gs));
        assert_eq!((gs.cell_x, gs.cell_z), (31, 31));
        let mut info = fc4_effect_info::default();
        assert!(fc4_effects_get_effect(fx.0, id, &mut info));
        assert!(info.started && info.refs == 2 && info.index >= 0);
        // Without a simulator the game record does not start.
        game.no_sim.set(true);
        let mut id2 = 0;
        assert!(fc4_effects_create_effect(
            fx.0,
            cs("volcanocreator4a").as_ptr(),
            &mut id2
        ));
        fc4_effects_start(fx.0, id2, 0);
        assert!(fc4_effects_get_game(fx.0, id2, 0, &mut gs));
        assert!(!gs.started);
        fc4_effects_shutdown(fx.0);
        let mut s = fc4_effects_state::default();
        assert!(fc4_effects_get_state(fx.0, &mut s));
        assert_eq!(s.effect_count, 0);
    }
}

#[test]
fn priority_zoom_and_unknown_names() {
    let game = Game::default();
    let fx = setup(&game);
    unsafe {
        assert!(fc4_effects_has(fx.0, cs("Volcano_Control").as_ptr()));
        assert!(!fc4_effects_has(fx.0, cs("nope").as_ptr()));
        let mut id = 0;
        assert!(!fc4_effects_create_effect(fx.0, cs("nope").as_ptr(), &mut id));
        fc4_effects_set_max_priority(fx.0, 0);
        assert!(fc4_effects_create_effect(fx.0, cs("volcano_control").as_ptr(), &mut id));
        let mut info = fc4_effect_info::default();
        assert!(fc4_effects_get_effect(fx.0, id, &mut info));
        assert_eq!((info.index, info.components), (-1, 0));
        fc4_effects_set_zoom(fx.0, 4, 0);
        let mut s = fc4_effects_state::default();
        assert!(fc4_effects_get_state(fx.0, &mut s));
        assert_eq!((s.level_of_detail, s.max_priority), (5, 0));
    }
}

#[test]
fn demolish_filter() {
    let f = fc4_effects_demolish_filter {
        no_networks: true,
        no_flora: false,
        _pad: [0; 2],
        min_size: 0.0,
        max_size: 40.0,
    };
    let mut o = fc4_effects_occupant_facts {
        extents: [10.0, 5.0, 10.0],
        ..Default::default()
    };
    unsafe {
        assert!(!fc4_effects_demolish_filter_skips(&f, &o));
        o.network = true;
        assert!(fc4_effects_demolish_filter_skips(&f, &o));
        o.network = false;
        o.extents = [50.0, 5.0, 10.0];
        assert!(fc4_effects_demolish_filter_skips(&f, &o));
        o.extents = [10.0, 5.0, 10.0];
        o.signpost = true;
        assert!(fc4_effects_demolish_filter_skips(&f, &o));
        assert!(!fc4_effects_demolish_filter_skips(std::ptr::null(), &o));
    }
}
