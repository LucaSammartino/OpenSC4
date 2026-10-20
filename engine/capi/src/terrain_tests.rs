// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The terrain C ABI: life cycle, queries, setters, the message hand-over,
//! brushes, navigability through the host, the bodies of water, the lava's
//! fire callbacks, re-entrancy and the records.

use std::cell::RefCell;
use std::ffi::c_void;

use crate::terrain::*;
use crate::{FC4_ERR_TRUNCATED, FC4_OK};

#[derive(Default)]
struct Game {
    posts: RefCell<Vec<fc4_terrain_post>>,
    calls: RefCell<Vec<String>>,
    /// The terrain, for callbacks that call back in.
    terrain: RefCell<Option<*mut fc4_terrain>>,
    /// What a re-entrant read returned.
    reentrant: RefCell<Vec<f32>>,
    bridge: Option<(i32, i32, f32)>,
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

unsafe extern "C" fn post(u: *mut c_void, p: *const fc4_terrain_post) {
    let g = game(u);
    g.posts.borrow_mut().push(*p);
    // The terrain is released before the posts are handed over: a read works.
    if let Some(t) = *g.terrain.borrow() {
        g.reentrant.borrow_mut().push(fc4_terrain_altitude(t, 0.0, 0.0));
    }
}
unsafe extern "C" fn random_uniform(u: *mut c_void, n: u32) -> u32 {
    let g = game(u);
    g.calls.borrow_mut().push(format!("random {n}"));
    // Called while the terrain is being changed: a read is refused.
    if let Some(t) = *g.terrain.borrow() {
        g.reentrant.borrow_mut().push(fc4_terrain_altitude(t, 0.0, 0.0));
    }
    0
}
unsafe extern "C" fn bridge_at(u: *mut c_void, x: i32, z: i32, deck: *mut f32) -> i32 {
    match game(u).bridge {
        Some((bx, bz, d)) if (bx, bz) == (x, z) => {
            *deck = d;
            FC4_TERRAIN_BRIDGE_DECK
        }
        _ => FC4_TERRAIN_BRIDGE_NONE,
    }
}
unsafe extern "C" fn pole(_: *mut c_void, x: i32, z: i32) -> bool {
    (x, z) == (3, 3)
}
unsafe extern "C" fn flammable(_: *mut c_void, _x: i32, _z: i32) -> bool {
    true
}
unsafe extern "C" fn ignite(u: *mut c_void, x: i32, z: i32) {
    game(u).calls.borrow_mut().push(format!("ignite {x} {z}"));
}
unsafe extern "C" fn quick_burn(u: *mut c_void, x: i32, z: i32) {
    game(u).calls.borrow_mut().push(format!("burn {x} {z}"));
}
unsafe extern "C" fn cause(u: *mut c_void, c: u32) {
    game(u).calls.borrow_mut().push(format!("cause {c}"));
}

fn host(g: &Game, with_post: bool) -> fc4_terrain_host {
    fc4_terrain_host {
        struct_size: std::mem::size_of::<fc4_terrain_host>() as u32,
        user: g as *const Game as *mut c_void,
        post: if with_post { Some(post) } else { None },
        random_uniform: Some(random_uniform),
        lot_at_cell: None,
        bridge_at: Some(bridge_at),
        has_power_pole: Some(pole),
        is_cell_flammable: Some(flammable),
        ignite_cell: Some(ignite),
        quick_burn_cell: Some(quick_burn),
        set_cause: Some(cause),
        next_burn_cell: None,
    }
}

fn flat(g: &Game, with_post: bool, cells: u32, h: f32) -> *mut fc4_terrain {
    let n = ((cells + 1) * (cells + 1)) as usize;
    let a = vec![h; n];
    let hs = host(g, with_post);
    unsafe {
        let t = fc4_terrain_create_from_altitudes(&hs, std::ptr::null(), cells, cells, a.as_ptr(), n, 250.0);
        assert!(!t.is_null());
        assert!(fc4_terrain_post_city_init(t));
        t
    }
}

#[test]
fn create_and_query() {
    let g = Game::default();
    let hs = host(&g, true);
    unsafe {
        let t = fc4_terrain_create(&hs, std::ptr::null(), 64, 64);
        assert!(!t.is_null());
        assert_eq!(fc4_terrain_cell_count(t, 0), 64);
        assert_eq!(fc4_terrain_altitude(t, 512.0, 512.0), 300.0);
        assert_eq!(fc4_terrain_sea_level(t), 250.0);
        assert_eq!(fc4_terrain_water_depth_at_vertex(t, 0, 0), -50.0);
        assert!(fc4_terrain_cell_has_land(t, 0, 0) && !fc4_terrain_cell_has_water(t, 0, 0));
        assert_eq!(fc4_terrain_allowed_height(t, 1), 2500.0);
        let mut n = [0f32; 3];
        assert!(fc4_terrain_face_normal(t, 8.0, 8.0, n.as_mut_ptr()));
        assert_eq!(n, [0.0, 1.0, 0.0]);
        assert!(!fc4_terrain_face_normal(t, 1024.0, 8.0, n.as_mut_ptr()));
        let mut c = [0f32; 4];
        assert!(fc4_terrain_cell_corner_altitudes(t, 1, 1, c.as_mut_ptr()));
        assert!(!fc4_terrain_cell_corner_altitudes(t, 64, 1, c.as_mut_ptr()));
        let mut st = fc4_terrain_state::default();
        assert!(fc4_terrain_get_state(t, &mut st));
        assert_eq!((st.verts_x, st.view_ready, st.saved_78), (65, 0, 308.0));
        fc4_terrain_destroy(t);
        // Bad host table, bad size.
        let mut bad = hs;
        bad.struct_size = 4;
        assert!(fc4_terrain_create(&bad, std::ptr::null(), 64, 64).is_null());
        assert!(fc4_terrain_create(&hs, std::ptr::null(), 0, 64).is_null());
        assert!(fc4_terrain_create(std::ptr::null(), std::ptr::null(), 64, 64).is_null());
        // NULL handle: fallbacks.
        assert_eq!(fc4_terrain_altitude(std::ptr::null(), 1.0, 1.0), 0.0);
        assert!(!fc4_terrain_set_sea_level(std::ptr::null_mut(), 100.0));
    }
}

#[test]
fn posts_go_to_the_host_after_the_terrain_is_released() {
    let g = Game::default();
    unsafe {
        let t = flat(&g, true, 16, 300.0);
        *g.terrain.borrow_mut() = Some(t);
        let r = fc4_terrain_rect {
            x0: 3,
            z0: 5,
            x1: 10,
            z1: 12,
        };
        assert!(fc4_terrain_redisplay(t, true, true, r, 4));
        let p = g.posts.borrow().clone();
        assert_eq!(p.len(), 3);
        assert_eq!(
            p[0],
            fc4_terrain_post {
                msg_type: 0x278E_6003,
                data1: 0x0003_000A,
                data2: 0x0005_000C,
                data3: 0,
                data4: 4
            }
        );
        assert_eq!((p[1].msg_type, p[1].data1, p[1].data4), (0x678E_6E3A, 0x0003_000A, 4));
        assert_eq!(p[2].msg_type, 0x870B_C918);
        // Each post could read the terrain.
        assert_eq!(*g.reentrant.borrow(), [300.0; 3]);
        // Nothing waits for take_posts.
        let mut buf = [fc4_terrain_post::default(); 4];
        assert_eq!(fc4_terrain_take_posts(t, buf.as_mut_ptr(), 4), 0);
        *g.terrain.borrow_mut() = None;
        fc4_terrain_destroy(t);
    }
}

#[test]
fn take_posts_without_a_post_callback() {
    let g = Game::default();
    unsafe {
        let t = flat(&g, false, 4, 300.0);
        assert!(!fc4_terrain_set_sea_level(t, 201.0));
        assert!(fc4_terrain_set_sea_level(t, 100.0));
        assert!(fc4_terrain_begin_long_operation(t));
        assert!(fc4_terrain_end_long_operation(t));
        let mut st = fc4_terrain_state::default();
        fc4_terrain_get_state(t, &mut st);
        assert_eq!(st.posts_pending, 6);
        let mut buf = [fc4_terrain_post::default(); 8];
        assert_eq!(fc4_terrain_take_posts(t, buf.as_mut_ptr(), 2), 2);
        assert_eq!((buf[0].msg_type, buf[1].msg_type), (0x8917_1496, 0x278E_6003));
        assert_eq!((buf[1].data1, buf[1].data2, buf[1].data4), (4, 4, 0));
        assert_eq!(fc4_terrain_take_posts(t, buf.as_mut_ptr(), 8), 4);
        let m: Vec<u32> = buf[..4].iter().map(|p| p.msg_type).collect();
        assert_eq!(m, [0x678E_6E3A, 0x870B_C918, 0xEA3B_0AF1, 0x6A3B_0AFD]);
        assert!(g.posts.borrow().is_empty());
        assert_eq!(fc4_terrain_water_depth_at_vertex(t, 0, 0), -200.0);
        fc4_terrain_destroy(t);
    }
}

#[test]
fn setters_and_the_stale_cache() {
    let g = Game::default();
    unsafe {
        let t = flat(&g, true, 4, 300.0);
        assert!(fc4_terrain_set_view_ready(t, false));
        assert!(fc4_terrain_set_altitude(t, 1, 1, 3000.0));
        assert_eq!(fc4_terrain_altitude_at_vertex(t, 1, 1), 2500.0);
        assert_eq!(fc4_terrain_water_depth_at_vertex(t, 1, 1), -2250.0);
        assert_eq!(fc4_terrain_cached_height(t, 1), 300.0);
        let i = fc4_terrain_vert_index(t, 2, 2);
        assert!(fc4_terrain_set_altitude_at_vertex(t, i, 240.0));
        assert_eq!(fc4_terrain_water_depth_at_index(t, i), 10.0);
        assert!(fc4_terrain_lock_altitude(
            t,
            fc4_terrain_rect {
                x0: 2,
                z0: 2,
                x1: 2,
                z1: 2
            },
            true
        ));
        assert!(fc4_terrain_raise_vertex(t, i, 5.0));
        assert_eq!(fc4_terrain_altitude_at_index(t, i), 240.0);
        assert_eq!(fc4_terrain_vertex_flags(t, 2, 2), 0x4000);
        assert!(g.posts.borrow().is_empty());
        assert!(fc4_terrain_post_city_init(t));
        assert!(fc4_terrain_redisplay(t, true, false, fc4_terrain_rect::default(), 0));
        assert_eq!(fc4_terrain_cached_height(t, 1), 2500.0);
        let mut a = vec![0f32; 25];
        assert_eq!(fc4_terrain_altitudes(t, a.as_mut_ptr(), 25), 25);
        assert_eq!(a[6], 2500.0);
        assert_eq!(fc4_terrain_altitudes(t, std::ptr::null_mut(), 0), 25);
        fc4_terrain_destroy(t);
    }
}

#[test]
fn water_queries_and_the_display_state_abi65() {
    let g = Game::default();
    unsafe {
        let t = flat(&g, true, 8, 200.0); // 50 m of sea everywhere
        assert_eq!(fc4_terrain_lerped_water_level(t, 40.0, 40.0), 250.0);
        assert_eq!(fc4_terrain_flooding_altitude(t, 2, 2), 250.0);
        let mut v = [1f32; 3];
        assert!(fc4_terrain_water_velocity(t, 2, 2, v.as_mut_ptr()));
        assert_eq!(v, [0.0; 3]);
        assert!(!fc4_terrain_water_velocity(t, 2, 2, std::ptr::null_mut()));
        assert_eq!(fc4_terrain_sediment_at(t, 40.0, 40.0), 0.0);
        assert_eq!(fc4_terrain_sediment_at(t, -1.0, 40.0), 0.0);
        // TR14: the display state posts only when it changes.
        let s = fc4_terrain_water_display_state(t);
        g.posts.borrow_mut().clear();
        assert!(fc4_terrain_set_water_display_state(t, s));
        assert!(g.posts.borrow().is_empty());
        assert!(fc4_terrain_set_water_display_state(t, s ^ 1));
        assert!(fc4_terrain_set_water_display_state(t, s ^ 1));
        assert_eq!(g.posts.borrow().len(), 1);
        assert_eq!(g.posts.borrow()[0].msg_type, 0x6AB9_7795);
        assert_eq!(fc4_terrain_water_display_state(t), s ^ 1);
        // NULL handle: fallbacks.
        assert_eq!(fc4_terrain_lerped_water_level(std::ptr::null(), 1.0, 1.0), 0.0);
        assert_eq!(fc4_terrain_flooding_altitude(std::ptr::null(), 1, 1), 0.0);
        assert!(!fc4_terrain_water_velocity(std::ptr::null(), 1, 1, v.as_mut_ptr()));
        assert_eq!(fc4_terrain_talus_angle(std::ptr::null()), 0.0);
        fc4_terrain_destroy(t);
    }
}

#[test]
fn sinkhole_brush_and_reentrancy() {
    let g = Game::default();
    unsafe {
        let t = flat(&g, true, 20, 300.0);
        *g.terrain.borrow_mut() = Some(t);
        let px = [0u8; 16];
        assert!(fc4_terrain_add_shipped_brush(t, 0x14, 4, 4, px.as_ptr()));
        assert!(!fc4_terrain_add_shipped_brush(t, 0x99, 4, 4, px.as_ptr()));
        let mut p = fc4_brush_params {
            from_x: 160.0,
            from_z: 160.0,
            to_x: 160.0,
            to_z: 160.0,
            level: 0,
            redisplay: 1,
            strength_scale: 1.0,
            radius_scale: 1.0,
            data4: 3,
            target_height: 100.0,
            ..Default::default()
        };
        assert!(fc4_terrain_apply_brush(t, 0x14, &mut p));
        assert_eq!(p.out_valid, 1);
        assert_eq!(
            p.out,
            fc4_terrain_rect {
                x0: 10,
                z0: 10,
                x1: 10,
                z1: 10
            }
        );
        assert!(fc4_terrain_altitude_at_vertex(t, 10, 10) < 300.0);
        let first = g.posts.borrow()[0];
        assert_eq!(
            (first.msg_type, first.data1, first.data4),
            (0x278E_6003, 0x000A_000A, 3)
        );
        // An unknown brush: false, the out rectangle cleared.
        assert!(!fc4_terrain_apply_brush(t, 0x1234_5678, &mut p));
        assert_eq!(p.out_valid, 0);
        // A random-walk brush calls random_uniform while the terrain is being
        // changed: the re-entrant read is refused.
        g.reentrant.borrow_mut().clear();
        let comp = fc4_brush_component {
            kind: 0,
            radius: [16.0; 5],
            strength: [0.01; 5],
            oriented: 0,
            neutral: 255,
            _pad: [0; 2],
            bitmap_width: 4,
            bitmap_height: 4,
            pixels: px.as_ptr(),
        };
        assert!(fc4_terrain_add_brush(t, 7, &comp, 1, 1.0, 0.5));
        g.posts.borrow_mut().clear();
        g.reentrant.borrow_mut().clear();
        assert!(fc4_terrain_apply_brush(t, 7, &mut p));
        assert_eq!(g.calls.borrow()[0], "random 5");
        let re = g.reentrant.borrow().clone();
        assert_eq!(re[0], 0.0, "refused while changing");
        // The shipped 0x14 walks too (0.5 degree step, TR8): its application
        // above drew once and its re-entrant read was refused as well.
        assert_eq!(g.calls.borrow().as_slice(), ["random 5", "random 5"]);
        let mut st = fc4_terrain_state::default();
        fc4_terrain_get_state(t, &mut st);
        assert_eq!((st.reentrant_refused, st.brush_count), (2, 2));
        let (mut walk, mut stroke) = (0.0, 1.0);
        assert!(fc4_terrain_brush_angles(t, 7, &mut walk, &mut stroke));
        assert!((f64::from(walk) - (0.5 - 1.0 + 2.0 * std::f64::consts::PI)).abs() < 1e-5);
        assert_eq!(stroke, 0.0);
        let mut angle = 1.0;
        assert!(fc4_terrain_brush_angle(t, 7, &mut angle));
        assert_eq!(angle, 0.0, "brush_angle is the stroke angle");
        assert!(fc4_terrain_set_brush_angles(t, 7, 1.25, 2.5));
        assert!(fc4_terrain_brush_angles(t, 7, &mut walk, std::ptr::null_mut()));
        assert_eq!(walk, 1.25);
        assert!(!fc4_terrain_set_brush_angles(t, 0x99, 0.0, 0.0));
        assert!(!fc4_terrain_brush_angles(t, 0x99, &mut walk, &mut stroke));
        assert_eq!(fc4_terrain_talus_angle(t), 0.79);
        assert!(fc4_terrain_set_talus_angle(t, 0.5));
        assert_eq!(fc4_terrain_talus_angle(t), 0.5);
        // A kind above 11 is rejected.
        let bad = fc4_brush_component { kind: 12, ..comp };
        assert!(!fc4_terrain_add_brush(t, 8, &bad, 1, 0.0, 0.0));
        *g.terrain.borrow_mut() = None;
        fc4_terrain_destroy(t);
    }
}

#[test]
fn navigability_and_bodies_through_the_host() {
    let g = Game {
        bridge: Some((6, 6, 400.0)),
        ..Game::default()
    };
    unsafe {
        let t = flat(&g, true, 8, 200.0); // 50 m of sea everywhere
        assert!(fc4_terrain_has_draft_clearance(t, 1, 1));
        assert_eq!(fc4_terrain_water_depth_for_cell(t, 1, 1), 50.0);
        assert_eq!(fc4_terrain_water_type(t, 1, 1), 3);
        assert!(fc4_terrain_is_navigable(t, 1, 1, 0, 0));
        assert!(!fc4_terrain_is_navigable(t, 3, 3, 0, 0), "a power pole");
        assert!(fc4_terrain_is_navigable(t, 6, 6, 0, 0), "a high bridge");
        assert!(fc4_terrain_update_bodies_of_water(t));
        let mut map = vec![0u16; 64];
        assert_eq!(fc4_terrain_bodies_map(t, map.as_mut_ptr(), 64), 64);
        assert_eq!(map[3 + 3 * 8], 0);
        assert_eq!(fc4_terrain_body_of_water(t, 0, 0), 1);
        assert!(map.iter().all(|&v| v <= 1));
        assert!(!fc4_terrain_bodies_dirty(t));
        assert!(fc4_terrain_bodies_note_power_pole(t, 2, 2));
        assert!(fc4_terrain_bodies_dirty(t));
        assert!(fc4_terrain_bodies_on_new_day(t));
        assert!(!fc4_terrain_bodies_on_new_day(t));
        fc4_terrain_destroy(t);
    }
}

#[test]
fn lava_flow_calls_the_fire_side() {
    let g = Game::default();
    unsafe {
        let t = flat(&g, true, 10, 300.0);
        assert!(!fc4_terrain_lava_flow(t, 33333.0));
        assert!(fc4_terrain_lava_start(t, 80.0, 80.0, 20.0, std::ptr::null()));
        let (mut s, mut a, mut b) = Default::default();
        assert!(fc4_terrain_lava_rects(t, &mut s, &mut a, &mut b));
        assert_eq!(
            s,
            fc4_terrain_rect {
                x0: 4,
                z0: 4,
                x1: 6,
                z1: 6
            }
        );
        assert_eq!(
            b,
            fc4_terrain_rect {
                x0: 0,
                z0: 0,
                x1: 9,
                z1: 9
            }
        );
        // Â§18 test 13 through the ABI: the west neighbour crosses 1.5.
        for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1), (-1, -1), (1, -1), (-1, 1), (1, 1)] {
            assert!(fc4_terrain_lava_set_level(t, 5 + dx, 5 + dz, 1.4));
        }
        assert!(fc4_terrain_lava_set_level(t, 5, 5, 10.0));
        assert!(fc4_terrain_lava_flow(t, 33333.0));
        let calls = g.calls.borrow().clone();
        assert_eq!(&calls[..2], ["ignite 3 4", "cause 3"]);
        assert!(fc4_terrain_lava_level(t, 4, 5) > 1.5);
        // No production: burning draws nothing.
        g.calls.borrow_mut().clear();
        assert!(fc4_terrain_lava_burn(t));
        assert!(g.calls.borrow().is_empty());
        assert!(fc4_terrain_lava_set_rates(t, 1e-4, 0.0));
        assert!(fc4_terrain_lava_burn(t)); // next_burn_cell missing: every draw lost
        assert!(g.calls.borrow().is_empty());
        assert!(fc4_terrain_lava_stop(t));
        assert!(!fc4_terrain_lava_stop(t));
        assert_eq!(fc4_terrain_lava_level(t, 5, 5), 0.0);
        fc4_terrain_destroy(t);
    }
}

#[test]
fn records_round_trip_through_load() {
    let g = Game::default();
    let hs = host(&g, false);
    unsafe {
        let t = flat(&g, false, 64, 300.0);
        assert!(fc4_terrain_set_altitude(t, 3, 4, 321.5));
        assert!(fc4_terrain_set_cached_heights(t, 1.0, 2.0));
        let rec = |r: u32| {
            let n = fc4_terrain_write_record(t, r, std::ptr::null_mut(), 0);
            let mut v = vec![0u8; n];
            assert_eq!(fc4_terrain_write_record(t, r, v.as_mut_ptr(), n), n);
            v
        };
        let (misc, alt, props) = (rec(0), rec(1), rec(2));
        assert_eq!(
            (misc.len(), alt.len(), props.len()),
            (26, 2 + 65 * 65 * 4, 2 + 65 * 65 * 2)
        );
        let edge = rec(FC4_TERRAIN_RECORD_EDGE + 3);
        assert_eq!(edge.len(), 2 + 4 + 65 * 4);
        assert_eq!(fc4_terrain_write_record(t, 7, std::ptr::null_mut(), 0), 0);
        let mut e = vec![0f32; 65];
        assert_eq!(fc4_terrain_read_edge(edge.as_ptr(), edge.len(), e.as_mut_ptr(), 65), 65);
        assert_eq!(fc4_terrain_read_edge(edge.as_ptr(), 3, e.as_mut_ptr(), 65), -1);
        let mut err = 99;
        let u = fc4_terrain_load(
            &hs,
            std::ptr::null(),
            misc.as_ptr(),
            misc.len(),
            alt.as_ptr(),
            alt.len(),
            props.as_ptr(),
            props.len(),
            &mut err,
        );
        assert!(!u.is_null());
        assert_eq!(err, FC4_OK);
        assert_eq!(fc4_terrain_altitude_at_vertex(u, 3, 4), 321.5);
        assert_eq!(
            (fc4_terrain_cached_height(u, 0), fc4_terrain_cached_height(u, 1)),
            (1.0, 2.0)
        );
        let v = fc4_terrain_load(
            &hs,
            std::ptr::null(),
            misc.as_ptr(),
            10,
            alt.as_ptr(),
            alt.len(),
            props.as_ptr(),
            props.len(),
            &mut err,
        );
        assert!(v.is_null());
        assert_eq!(err, FC4_ERR_TRUNCATED);
        fc4_terrain_destroy(u);
        fc4_terrain_destroy(t);
    }
}

#[test]
fn tuning_helpers() {
    unsafe {
        let mut t = fc4_terrain_tuning::default();
        assert!(fc4_terrain_tuning_shipped(&mut t));
        assert_eq!((t.max_height, t.min_height, t.sea_level_max), (2500.0, 20.0, 200.0));
        assert!(fc4_terrain_tuning_from_props(std::ptr::null(), &mut t));
        assert_eq!((t.max_height, t.min_height), (200.0, 5.0));
        let mut l = fc4_lava_tuning::default();
        assert!(fc4_lava_tuning_shipped(&mut l));
        assert_eq!((l.max_flow_rate, l.solidify_threshold), (4e-6, 3.0));
        // A custom tuning: the clamps follow it.
        let g = Game::default();
        let hs = host(&g, false);
        t.struct_size = std::mem::size_of::<fc4_terrain_tuning>() as u32;
        let tr = fc4_terrain_create(&hs, &t, 4, 4);
        assert!(fc4_terrain_set_altitude(tr, 1, 1, 1000.0));
        assert_eq!(fc4_terrain_altitude_at_vertex(tr, 1, 1), 200.0);
        fc4_terrain_destroy(tr);
    }
}

#[test]
fn a_second_thread_reads_while_the_first_writes() {
    let g = Game::default();
    let t = flat(&g, false, 32, 300.0) as usize;
    let reader = std::thread::spawn(move || {
        let mut seen = 0;
        for _ in 0..2000 {
            let a = unsafe { fc4_terrain_altitude(t as *const fc4_terrain, 100.0, 100.0) };
            assert!(a == 300.0 || a == 310.0, "{a}");
            seen += 1;
        }
        seen
    });
    for k in 0..2000 {
        let h = if k % 2 == 0 { 310.0 } else { 300.0 };
        unsafe {
            let r = fc4_terrain_rect {
                x0: 0,
                z0: 0,
                x1: 32,
                z1: 32,
            };
            fc4_terrain_set_altitude_rect(t as *mut fc4_terrain, r, h, false, 0);
        }
    }
    assert_eq!(reader.join().unwrap(), 2000);
    unsafe { fc4_terrain_destroy(t as *mut fc4_terrain) };
}
