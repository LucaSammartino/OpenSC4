// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The network geometry C ABI: the parameters and pure helpers, the drag,
//! the passes over host callbacks, placing and grading over the host's
//! terrain and over a bound core terrain, the standalone satisfier, NULL
//! handling and re-entrancy.

use std::cell::RefCell;
use std::ffi::c_void;

use crate::network_geometry::*;
use crate::terrain::{
    fc4_terrain, fc4_terrain_altitude_at_index, fc4_terrain_create_from_altitudes, fc4_terrain_destroy,
    fc4_terrain_host, fc4_terrain_post, fc4_terrain_rect, fc4_terrain_set_view_ready,
};

const ROAD: u32 = 0;
const STREET: u32 = 3;
const PIPE: u32 = 4;
const AVENUE: u32 = 6;

/// A 64 x 64 city of vertex altitudes, sea at 250, as the host sees it.
struct Game {
    alt: RefCell<Vec<f32>>,
    sea: f32,
    landfill: Option<(i32, i32)>,
    calls: RefCell<Vec<String>>,
    geo: RefCell<Option<*mut fc4_netgeom>>,
    reentrant: RefCell<Vec<u32>>,
}

const N: i32 = 64;
const S: i32 = N + 1;

impl Game {
    fn new(f: impl Fn(i32, i32) -> f32) -> Game {
        let mut alt = Vec::new();
        for z in 0..S {
            for x in 0..S {
                alt.push(f(x, z));
            }
        }
        Game {
            alt: RefCell::new(alt),
            sea: 250.0,
            landfill: None,
            calls: RefCell::new(Vec::new()),
            geo: RefCell::new(None),
            reentrant: RefCell::new(Vec::new()),
        }
    }
    fn alt(&self, v: i32) -> f32 {
        self.alt.borrow().get(v as usize).copied().unwrap_or(0.0)
    }
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

unsafe extern "C" fn cells(_: *mut c_void, x: *mut u32, z: *mut u32) {
    *x = N as u32;
    *z = N as u32;
}
unsafe extern "C" fn vertex_altitude(u: *mut c_void, v: i32) -> f32 {
    game(u).alt(v)
}
unsafe extern "C" fn cell_has_water(u: *mut c_void, x: i32, z: i32) -> bool {
    let g = game(u);
    [(x, z), (x, z + 1), (x + 1, z + 1), (x + 1, z)]
        .iter()
        .any(|&(vx, vz)| g.alt(vz * S + vx) < g.sea)
}
unsafe extern "C" fn surface_water_level(u: *mut c_void, _v: i32) -> f32 {
    game(u).sea
}
unsafe extern "C" fn max_allowed_height(_: *mut c_void) -> f32 {
    5000.0
}
unsafe extern "C" fn set_altitude(u: *mut c_void, v: i32, h: f32) {
    game(u).alt.borrow_mut()[v as usize] = h;
}
unsafe extern "C" fn begin(u: *mut c_void) {
    game(u).calls.borrow_mut().push("begin".into());
}
unsafe extern "C" fn redisplay(u: *mut c_void, r: *const fc4_terrain_rect, kind: u32) {
    let r = *r;
    game(u)
        .calls
        .borrow_mut()
        .push(format!("redisplay {} {} {} {} {kind}", r.x0, r.z0, r.x1, r.z1));
}
unsafe extern "C" fn end(u: *mut c_void) {
    game(u).calls.borrow_mut().push("end".into());
}
unsafe extern "C" fn occupants(u: *mut c_void, x: i32, z: i32, out: *mut fc4_netgeom_occupant, cap: u32) -> u32 {
    let g = game(u);
    // A callback that calls back into the geometry is refused.
    if let Some(geo) = *g.geo.borrow() {
        g.reentrant.borrow_mut().push(fc4_netgeom_validate(geo));
    }
    if g.landfill == Some((x, z)) {
        if cap >= 1 {
            *out = fc4_netgeom_occupant {
                kind: FC4_NETGEOM_OCC_LANDFILL,
                ..fc4_netgeom_occupant::default()
            };
        }
        1
    } else {
        0
    }
}

fn host(g: &Game) -> fc4_netgeom_host {
    fc4_netgeom_host {
        struct_size: std::mem::size_of::<fc4_netgeom_host>() as u32,
        user: g as *const Game as *mut c_void,
        cells: Some(cells),
        vertex_altitude: Some(vertex_altitude),
        cell_has_water: Some(cell_has_water),
        surface_water_level: Some(surface_water_level),
        max_allowed_height: Some(max_allowed_height),
        set_altitude_at_vertex: Some(set_altitude),
        begin_long_terrain_operation: Some(begin),
        redisplay_terrain: Some(redisplay),
        end_long_terrain_operation: Some(end),
        is_connection_node: None,
        cell_occupants: Some(occupants),
        existing_piece: None,
        extends_diagonal_end: None,
        update_occupant_highlights: None,
        bridge_type_count: None,
        is_step_good_for_bridge: None,
        bridge_step_edge_types_ok: None,
        adjust_bridge_span: None,
        find_best_bridge_type: None,
    }
}

fn state(g: *mut fc4_netgeom) -> fc4_netgeom_state {
    let mut s = fc4_netgeom_state::default();
    assert!(unsafe { fc4_netgeom_get_state(g, &mut s) });
    s
}

#[test]
fn params_and_type_info() {
    unsafe {
        let mut p = fc4_netgeom_params::default();
        assert!(fc4_netgeom_params_shipped(ROAD, &mut p));
        assert_eq!(p.struct_size, 76);
        assert_eq!((p.max_slope, p.support_threshold, p.cell_cost), (35.0, 45.0, 10));
        assert!(!fc4_netgeom_params_shipped(13, &mut p));
        let mut i = fc4_netgeom_type_info::default();
        assert!(fc4_netgeom_type_info(ROAD, std::ptr::null(), &mut i));
        assert!((i.slope_rise - 11.2033).abs() < 1e-4);
        assert!((i.diagonal_slope_rise - 7.9219).abs() < 1e-4);
        assert_eq!((i.width1, i.width2), (1, 1));
        assert!(fc4_netgeom_type_info(AVENUE, std::ptr::null(), &mut i));
        assert_eq!((i.width1, i.width2), (2, 2));
        // Explicit parameters: pipe's 45 degrees.
        assert!(fc4_netgeom_params_shipped(PIPE, &mut p));
        assert!(fc4_netgeom_type_info(PIPE, &p, &mut i));
        assert!((i.slope_rise - 16.0).abs() < 1e-4);
        // A short struct_size is refused.
        p.struct_size = 8;
        assert!(!fc4_netgeom_type_info(PIPE, &p, &mut i));
        // No properties: street is still straight only; the costs keep their defaults.
        assert!(fc4_netgeom_params_from_props(STREET, std::ptr::null(), &mut p));
        assert_eq!(
            (p.straight_only, p.max_slope, p.cell_cost, p.tunnel_clearance),
            (1, 0.0, 1, 10.0)
        );
    }
}

#[test]
fn segments_and_helpers() {
    unsafe {
        let mut s = fc4_netgeom_segments::default();
        assert!(fc4_netgeom_break_into_segments(0, 0, 6, 4, true, false, 1, 1, &mut s));
        assert_eq!((s.end_x, s.end_z, s.dragged_count, s.straight_count), (5, 4, 6, 0));
        assert_eq!((s.axis0, s.axis1, s.pen_direction, s.end_direction), (0, 1, 4, 5));
        assert!(!fc4_netgeom_break_into_segments(
            0,
            0,
            6,
            4,
            true,
            false,
            1,
            1,
            std::ptr::null_mut()
        ));

        let t = [100.0f32; 4];
        assert_eq!(fc4_netgeom_support_level([130.0; 4].as_ptr(), t.as_ptr(), 45.0), 1);
        assert_eq!(fc4_netgeom_support_level([150.0; 4].as_ptr(), t.as_ptr(), 45.0), 2);
        assert_eq!(fc4_netgeom_support_level(std::ptr::null(), t.as_ptr(), 45.0), 0);
        assert_eq!(fc4_netgeom_support_flags(1), 0x4800_0000);
        assert_eq!(fc4_netgeom_occupant_flags(1, false, false) & 0x8000_0000, 0x8000_0000);
        assert_eq!(
            fc4_netgeom_occupant_flags(1 | 0x8000_0000, true, false) & 0x8000_0000,
            0
        );
        assert_eq!(fc4_netgeom_power_line_cost(3, 20, 15, 2.0), 85);
        assert_eq!(fc4_netgeom_power_line_cost(1, 20, 15, 2.0), 0);
        assert_eq!(fc4_netgeom_destroy_key(21, 20), (21 << 16) + 20);

        let (mut lo, mut hi, mut pr) = (0.0f32, 0.0f32, 0.0f32);
        let k = fc4_netgeom_terrain_vertex(
            300.0,
            300.0,
            false,
            false,
            ROAD,
            std::ptr::null(),
            &mut lo,
            &mut hi,
            &mut pr,
        );
        assert_ne!(k, FC4_NETGEOM_TV_INVALID);
        let k = fc4_netgeom_terrain_vertex(
            300.0,
            300.0,
            false,
            false,
            99,
            std::ptr::null(),
            &mut lo,
            &mut hi,
            &mut pr,
        );
        assert_eq!(k, FC4_NETGEOM_TV_INVALID);
    }
}

#[test]
fn standalone_satisfier_slope() {
    unsafe {
        let mut i = fc4_netgeom_type_info::default();
        assert!(fc4_netgeom_type_info(ROAD, std::ptr::null(), &mut i));
        let s_rise = i.slope_rise;
        let run = |c: f32| {
            let s = fc4_vhcs_create();
            assert!(fc4_vhcs_init(s, 0.0, 1000.0));
            fc4_vhcs_insert_fixed(s, 1, 0.0);
            fc4_vhcs_insert_fixed(s, 3, c);
            assert!(fc4_vhcs_add_vertex(s, 2, 0.0));
            assert!(!fc4_vhcs_add_vertex(s, 2, 0.0));
            fc4_vhcs_insert_slope(s, 1, 2, s_rise);
            fc4_vhcs_insert_slope(s, 2, 3, s_rise);
            (fc4_vhcs_solve(s), s)
        };
        let (ok, s) = run(30.0);
        assert!(!ok);
        fc4_vhcs_destroy(s);
        let (ok, s) = run(20.0);
        assert!(ok);
        let mut h = 0.0f32;
        assert!(fc4_vhcs_height(s, 2, &mut h));
        assert_eq!(h, (20.0f64 - f64::from(s_rise)) as f32);
        let n = fc4_vhcs_solution(s, std::ptr::null_mut(), 0);
        assert_eq!(n, 3);
        let mut out = vec![fc4_netgeom_vertex_height::default(); n as usize];
        assert_eq!(fc4_vhcs_solution(s, out.as_mut_ptr(), n), 3);
        assert!(out.iter().any(|v| v.vertex == 3 && v.height == 20.0));
        assert!(!fc4_vhcs_height(s, 77, &mut h));
        fc4_vhcs_destroy(s);
        // NULL handles.
        assert!(!fc4_vhcs_solve(std::ptr::null_mut()));
        fc4_vhcs_destroy(std::ptr::null_mut());
    }
}

#[test]
fn place_over_the_host_terrain() {
    let g = Game::new(|_, _| 300.0);
    let h = host(&g);
    unsafe {
        let geo = fc4_netgeom_create(&h, std::ptr::null(), 0);
        assert!(!geo.is_null());
        assert!(fc4_netgeom_draw_line(geo, ROAD, 10, 10, 20, 10, true));
        let st = state(geo);
        assert_eq!(
            (st.dragged_count, st.straight_count, st.pen_direction, st.end_direction),
            (11, 11, 4, 3)
        );
        let n = fc4_netgeom_get_cells(geo, std::ptr::null_mut(), 0);
        let mut c = vec![fc4_netgeom_cell::default(); n as usize];
        assert_eq!(fc4_netgeom_get_cells(geo, c.as_mut_ptr(), n), 11);
        assert_eq!((c[0].x, c[0].z, c[10].x, c[10].z), (10, 10, 20, 10));
        let mut steps = vec![fc4_netgeom_step::default(); 11];
        assert_eq!(fc4_netgeom_get_steps(geo, steps.as_mut_ptr(), 11), 11);
        assert!(steps.iter().all(|s| s.first == s.last && s.kind == 0));

        assert_eq!(fc4_netgeom_place(geo), 11);
        let st = state(geo);
        assert_eq!((st.valid, st.failure, st.level), (11, 0, 0));
        assert_eq!(st.solved_count, 11);
        assert_eq!(fc4_netgeom_cost(geo, 0), 110);
        let mut ci = fc4_netgeom_cell_info::default();
        assert!(fc4_netgeom_get_cell_info(geo, 12, 10, &mut ci));
        assert_eq!((ci.x, ci.z, ci.in_run, ci.step), (12, 10, 1, 2));
        assert_eq!(ci.vertices[0], 10 * S + 12);
        let mut hgt = 0.0f32;
        assert!(fc4_netgeom_sat_height(
            geo,
            FC4_NETGEOM_SAT_NETWORK,
            10 * S + 12,
            &mut hgt
        ));
        assert_eq!(hgt, 300.0);
        assert!(!fc4_netgeom_sat_height(geo, 7, 10 * S + 12, &mut hgt));
        // A flat road grades nothing: no long operation.
        let mut r = fc4_terrain_rect::default();
        assert_eq!(fc4_netgeom_update_terrain_heights(geo, &mut r), 0);
        assert!(g.calls.borrow().is_empty());

        // A dip is graded through the host: Begin, the vertices, RedisplayTerrain kind 4, End.
        g.alt.borrow_mut().iter_mut().enumerate().for_each(|(v, a)| {
            if v as i32 % S == 15 {
                *a = 294.0;
            }
        });
        assert!(fc4_netgeom_reset(geo));
        assert!(fc4_netgeom_draw_line(geo, ROAD, 10, 10, 20, 10, true));
        assert_eq!(fc4_netgeom_place(geo), 11);
        let changed = fc4_netgeom_update_terrain_heights(geo, &mut r);
        assert!(changed > 0);
        let calls = g.calls.borrow().clone();
        assert_eq!(calls.first().map(String::as_str), Some("begin"));
        assert_eq!(calls.last().map(String::as_str), Some("end"));
        assert_eq!(calls[1], format!("redisplay {} {} {} {} 4", r.x0, r.z0, r.x1, r.z1));
        let n = fc4_netgeom_sat_solution(geo, FC4_NETGEOM_SAT_TERRAIN, std::ptr::null_mut(), 0);
        let mut sol = vec![fc4_netgeom_vertex_height::default(); n as usize];
        fc4_netgeom_sat_solution(geo, FC4_NETGEOM_SAT_TERRAIN, sol.as_mut_ptr(), n);
        for v in &sol {
            assert_eq!(g.alt(v.vertex), v.height);
        }
        fc4_netgeom_destroy(geo);
    }
}

#[test]
fn failure_codes_through_the_host() {
    // A river at vertex columns 9..=12: the road starts in water.
    let mut g = Game::new(|x, _| if (9..=12).contains(&x) { 200.0 } else { 300.0 });
    g.landfill = None;
    let h = host(&g);
    unsafe {
        let geo = fc4_netgeom_create(&h, std::ptr::null(), 0);
        assert!(fc4_netgeom_draw_line(geo, ROAD, 10, 10, 20, 10, true));
        assert_eq!(fc4_netgeom_place(geo), 0);
        assert_eq!(state(geo).failure, 0xF000_0001);
        // The pipe is not stopped by water.
        assert!(fc4_netgeom_reset(geo));
        assert!(fc4_netgeom_draw_line(geo, PIPE, 10, 10, 20, 10, true));
        assert_eq!(fc4_netgeom_validate(geo), 11);
        assert_eq!(state(geo).failure, 0);
        fc4_netgeom_destroy(geo);
    }

    // A landfill at (12, 10): reserved.
    let mut g = Game::new(|_, _| 300.0);
    g.landfill = Some((12, 10));
    let h = host(&g);
    unsafe {
        let geo = fc4_netgeom_create(&h, std::ptr::null(), 0);
        assert!(fc4_netgeom_draw_line(geo, ROAD, 10, 10, 15, 10, true));
        assert_eq!(fc4_netgeom_place(geo), 0);
        assert_eq!(state(geo).failure, 0xF000_0002);
        // Re-entering the geometry from a callback is refused.
        *g.geo.borrow_mut() = Some(geo);
        assert!(fc4_netgeom_reset(geo));
        assert!(fc4_netgeom_draw_line(geo, ROAD, 10, 10, 15, 10, true));
        fc4_netgeom_validate(geo);
        *g.geo.borrow_mut() = None;
        assert!(!g.reentrant.borrow().is_empty());
        assert!(g.reentrant.borrow().iter().all(|&v| v == 0));
        assert!(state(geo).reentrant_refused > 0);
        fc4_netgeom_destroy(geo);
    }
}

#[test]
fn street_params_and_cell_infos() {
    let g = Game::new(|_, _| 300.0);
    let h = host(&g);
    unsafe {
        // An explicit table: only road and rail given; street (type 3) gets zero params.
        let mut p = [fc4_netgeom_params::default(); 2];
        assert!(fc4_netgeom_params_shipped(0, &mut p[0]));
        assert!(fc4_netgeom_params_shipped(1, &mut p[1]));
        let geo = fc4_netgeom_create(&h, p.as_ptr(), 2);
        assert!(!geo.is_null());
        let mut i = fc4_netgeom_type_info::default();
        assert!(fc4_netgeom_get_type_info(geo, STREET, &mut i));
        assert_eq!(i.slope_rise, 0.0);
        assert!(!fc4_netgeom_get_type_info(geo, 13, &mut i));
        fc4_netgeom_destroy(geo);
        // A short params entry is refused.
        p[1].struct_size = 4;
        assert!(fc4_netgeom_create(&h, p.as_ptr(), 2).is_null());

        let geo = fc4_netgeom_create(&h, std::ptr::null(), 0);
        // Street is straight only.
        assert!(fc4_netgeom_draw_line(geo, STREET, 0, 0, 5, 4, false));
        let st = state(geo);
        assert_eq!((st.end_x, st.end_z, st.dragged_count, st.straight_count), (5, 0, 6, 6));
        // A cell info round trip.
        let mut ci = fc4_netgeom_cell_info {
            struct_size: std::mem::size_of::<fc4_netgeom_cell_info>() as u32,
            x: 30,
            z: 30,
            step: -1,
            placed: 1,
            corners: [310.0, 311.0, 312.0, 313.0],
            allowed: [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
            vertices: [30 * S + 30, 31 * S + 30, 31 * S + 31, 30 * S + 31],
            ..fc4_netgeom_cell_info::default()
        };
        assert!(fc4_netgeom_insert_cell_info(geo, &ci));
        let mut back = fc4_netgeom_cell_info::default();
        assert!(fc4_netgeom_get_cell_info(geo, 30, 30, &mut back));
        assert_eq!(back, ci);
        ci.struct_size = 0;
        assert!(!fc4_netgeom_insert_cell_info(geo, &ci));
        assert!(!fc4_netgeom_get_cell_info(geo, 31, 30, &mut back));
        assert!(fc4_netgeom_set_land_bridges(geo, ROAD, true));
        assert!(!fc4_netgeom_set_land_bridges(geo, 13, true));
        fc4_netgeom_destroy(geo);
    }
}

unsafe extern "C" fn tpost(u: *mut c_void, p: *const fc4_terrain_post) {
    (*(u as *const RefCell<Vec<fc4_terrain_post>>)).borrow_mut().push(*p);
}

#[test]
fn grading_a_bound_core_terrain() {
    let posts: RefCell<Vec<fc4_terrain_post>> = RefCell::new(Vec::new());
    let mut th: fc4_terrain_host = unsafe { std::mem::zeroed() };
    th.struct_size = std::mem::size_of::<fc4_terrain_host>() as u32;
    th.user = &posts as *const _ as *mut c_void;
    th.post = Some(tpost);
    let mut alt = Vec::new();
    for _z in 0..S {
        for x in 0..S {
            alt.push(if x == 15 { 294.0f32 } else { 300.0 });
        }
    }
    // The geometry's host has no terrain callbacks at all: everything comes from the core terrain.
    let g = Game::new(|_, _| 0.0);
    let mut h = host(&g);
    h.cells = None;
    h.vertex_altitude = None;
    h.cell_has_water = None;
    h.surface_water_level = None;
    h.max_allowed_height = None;
    unsafe {
        let t: *mut fc4_terrain = fc4_terrain_create_from_altitudes(
            &th,
            std::ptr::null(),
            N as u32,
            N as u32,
            alt.as_ptr(),
            alt.len(),
            250.0,
        );
        assert!(!t.is_null());
        assert!(fc4_terrain_set_view_ready(t, true));
        posts.borrow_mut().clear();
        let geo = fc4_netgeom_create(&h, std::ptr::null(), 0);
        assert!(fc4_netgeom_set_terrain(geo, t));
        assert_eq!(state(geo).terrain_bound, 1);
        assert!(fc4_netgeom_draw_line(geo, ROAD, 10, 10, 20, 10, true));
        assert_eq!(fc4_netgeom_place(geo), 11);
        let mut r = fc4_terrain_rect::default();
        let changed = fc4_netgeom_update_terrain_heights(geo, &mut r);
        assert!(changed > 0);
        // The host's grading callbacks are not used; the terrain posted its messages.
        assert!(g.calls.borrow().is_empty());
        let kinds: Vec<u32> = posts.borrow().iter().map(|p| p.msg_type).collect();
        assert_eq!(kinds.first(), Some(&0xEA3B_0AF1));
        assert_eq!(kinds.last(), Some(&0x6A3B_0AFD));
        let ch = posts
            .borrow()
            .iter()
            .copied()
            .find(|p| p.msg_type == 0x278E_6003)
            .unwrap();
        assert_eq!(ch.data4, 4);
        assert_eq!(
            (ch.data1, ch.data2),
            (((r.x0 << 16) + r.x1) as u32, ((r.z0 << 16) + r.z1) as u32)
        );
        let n = fc4_netgeom_sat_solution(geo, FC4_NETGEOM_SAT_TERRAIN, std::ptr::null_mut(), 0);
        let mut sol = vec![fc4_netgeom_vertex_height::default(); n as usize];
        fc4_netgeom_sat_solution(geo, FC4_NETGEOM_SAT_TERRAIN, sol.as_mut_ptr(), n);
        for v in &sol {
            let a = fc4_terrain_altitude_at_index(t, v.vertex as u32);
            assert!((a - v.height).abs() <= 0.001, "vertex {} {} {}", v.vertex, a, v.height);
        }
        // Unbinding goes back to the host table (whose terrain is empty: nothing to draw on).
        assert!(fc4_netgeom_set_terrain(geo, std::ptr::null_mut()));
        assert_eq!(state(geo).terrain_bound, 0);
        fc4_netgeom_destroy(geo);
        fc4_terrain_destroy(t);
    }
}

#[test]
fn null_and_bad_arguments() {
    unsafe {
        assert!(fc4_netgeom_create(std::ptr::null(), std::ptr::null(), 0).is_null());
        let g = Game::new(|_, _| 300.0);
        let mut h = host(&g);
        h.struct_size = 8;
        assert!(fc4_netgeom_create(&h, std::ptr::null(), 0).is_null());
        let null = std::ptr::null_mut();
        assert!(!fc4_netgeom_draw_line(null, ROAD, 0, 0, 1, 0, true));
        assert_eq!(fc4_netgeom_place(null), 0);
        assert_eq!(fc4_netgeom_update_terrain_heights(null, std::ptr::null_mut()), 0);
        assert_eq!(fc4_netgeom_smooth(null, std::ptr::null_mut()), u32::MAX);
        let mut s = fc4_netgeom_state::default();
        assert!(!fc4_netgeom_get_state(null, &mut s));
        assert_eq!(fc4_netgeom_get_cells(null, std::ptr::null_mut(), 0), 0);
        fc4_netgeom_destroy(null);
    }
}
