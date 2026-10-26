// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The land value C ABI against a fake game: grids as host tokens, a full
//! pass, persistence through host streams, and the reference protocol.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;

use fc4_core::simgrid::{CityCells, GridGeometry};

use crate::landvalue::*;
use crate::stream::{fc4_bytes_sink, fc4_istream, fc4_ostream, fc4_ref};

enum Data {
    U8(Vec<u8>),
    F32(Vec<f32>),
    I16(Vec<i16>),
}

struct Grid {
    geom: GridGeometry,
    data: Data,
    refs: i32,
}

#[derive(Default)]
struct Game {
    grids: RefCell<HashMap<u64, Grid>>,
    next: RefCell<u64>,
    log: RefCell<Vec<String>>,
    live_altitude: Cell<f32>,
    live_slope: Cell<f32>,
}

impl Game {
    fn add(&self, s: i32, data: impl FnOnce(usize) -> Data) -> u64 {
        let geom = GridGeometry::for_city(&CITY, s);
        let mut n = self.next.borrow_mut();
        *n += 0x10;
        self.grids.borrow_mut().insert(*n, Grid { geom, data: data(geom.len()), refs: 1 });
        *n
    }
}

const CITY: CityCells = CityCells {
    cells_x: 16,
    cells_z: 8,
    cell_width_x: 16.0,
    cell_width_z: 16.0,
};
const DEV: u64 = 0xDE5;

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}
unsafe extern "C" fn terrain_cells(_: *mut c_void, x: *mut i32, z: *mut i32) {
    *x = CITY.cells_x;
    *z = CITY.cells_z;
}
unsafe extern "C" fn is_land_vertex(_: *mut c_void, x: u32, _z: u32) -> bool {
    x > 2 // the first tract column is water
}
unsafe extern "C" fn altitude(_: *mut c_void, _x: f32, _z: f32) -> f32 {
    100.0
}
unsafe extern "C" fn min_h(_: *mut c_void) -> f32 {
    20.0
}
unsafe extern "C" fn max_h(_: *mut c_void) -> f32 {
    2500.0
}
unsafe extern "C" fn zone_type(_: *mut c_void, x: i32, z: i32) -> u32 {
    u32::from(x < 8 && z < 2)
}
unsafe extern "C" fn area_slope(_: *mut c_void, x1: i32, z1: i32, _x2: i32, _z2: i32) -> f32 {
    x1 as f32 + z1 as f32 / 100.0
}
unsafe extern "C" fn developer(_: *mut c_void, t: u32) -> u64 {
    if t == 0x1030 {
        DEV
    } else {
        0
    }
}
unsafe extern "C" fn desirability_map(u: *mut c_void, dev: u64) -> u64 {
    if dev != DEV {
        return 0;
    }
    let g = game(u);
    // Created once, kept by the "developer".
    let existing = g
        .grids
        .borrow()
        .iter()
        .find(|(_, v)| matches!(v.data, Data::I16(_)))
        .map(|(k, _)| *k);
    existing.unwrap_or_else(|| g.add(2, |n| Data::I16(vec![40; n])))
}
unsafe extern "C" fn create_grid(u: *mut c_void, kind: u32, s: i32, id: u32) -> u64 {
    let g = game(u);
    g.log.borrow_mut().push(format!("create {kind} {s} {id:08X}"));
    g.add(s, |n| if kind == FC4_GRID_U8 { Data::U8(vec![0; n]) } else { Data::F32(vec![0.0; n]) })
}
unsafe extern "C" fn grid_geometry(u: *mut c_void, t: u64, out: *mut fc4_grid_geometry) {
    if let Some(g) = game(u).grids.borrow().get(&t) {
        let x = g.geom;
        *out = fc4_grid_geometry {
            tract_size: x.tract_size,
            shift: x.shift,
            count_x: x.count_x,
            count_z: x.count_z,
            width_x: x.width_x,
            width_z: x.width_z,
            inv_width_x: x.inv_width_x,
            inv_width_z: x.inv_width_z,
        };
    }
}
unsafe extern "C" fn set_tract_size(u: *mut c_void, t: u64, s: i32) {
    game(u).log.borrow_mut().push(format!("resize {t:X} {s}"));
}
unsafe extern "C" fn read_u8(u: *mut c_void, t: u64, out: *mut u8, n: usize) {
    if let Some(Grid { data: Data::U8(v), .. }) = game(u).grids.borrow().get(&t) {
        std::ptr::copy_nonoverlapping(v.as_ptr(), out, n.min(v.len()));
    }
}
unsafe extern "C" fn read_i16(u: *mut c_void, t: u64, out: *mut i16, n: usize) {
    if let Some(Grid { data: Data::I16(v), .. }) = game(u).grids.borrow().get(&t) {
        std::ptr::copy_nonoverlapping(v.as_ptr(), out, n.min(v.len()));
    }
}
unsafe extern "C" fn write_u8(u: *mut c_void, t: u64, p: *const u8, n: usize) {
    if let Some(Grid { data: Data::U8(v), .. }) = game(u).grids.borrow_mut().get_mut(&t) {
        v.copy_from_slice(std::slice::from_raw_parts(p, n));
    }
}
unsafe extern "C" fn write_f32(u: *mut c_void, t: u64, p: *const f32, n: usize) {
    if let Some(Grid { data: Data::F32(v), .. }) = game(u).grids.borrow_mut().get_mut(&t) {
        v.copy_from_slice(std::slice::from_raw_parts(p, n));
    }
}
unsafe extern "C" fn grid_shutdown(u: *mut c_void, t: u64) {
    game(u).log.borrow_mut().push(format!("shutdown {t:X}"));
}
unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    let g = game(u);
    g.log.borrow_mut().push(format!("release {t:X}"));
    let mut grids = g.grids.borrow_mut();
    let x = grids.get_mut(&t).expect("release of an unknown grid");
    x.refs -= 1;
    assert!(x.refs >= 0, "grid {t:X} over-released");
}
unsafe extern "C" fn history_len(u: *mut c_void, s: u32, n: u32) {
    game(u).log.borrow_mut().push(format!("history_length {s:08X} {n}"));
}
unsafe extern "C" fn history(u: *mut c_void, s: u32, v: i32) {
    game(u).log.borrow_mut().push(format!("history {s:08X} {v}"));
}
unsafe extern "C" fn post(u: *mut c_void, m: u32) {
    game(u).log.borrow_mut().push(format!("post {m:08X}"));
}

fn host(g: &Game) -> fc4_landvalue_host {
    fc4_landvalue_host {
        struct_size: std::mem::size_of::<fc4_landvalue_host>() as u32,
        user: g as *const Game as *mut c_void,
        terrain_cells: Some(terrain_cells),
        is_land_vertex: Some(is_land_vertex),
        altitude: Some(altitude),
        min_allowed_height: Some(min_h),
        max_allowed_height: Some(max_h),
        zone_type: Some(zone_type),
        area_slope: Some(area_slope),
        developer: Some(developer),
        desirability_map: Some(desirability_map),
        create_grid: Some(create_grid),
        grid_geometry: Some(grid_geometry),
        grid_set_tract_size: Some(set_tract_size),
        grid_read_u8: Some(read_u8),
        grid_read_i16: Some(read_i16),
        grid_write_u8: Some(write_u8),
        grid_write_f32: Some(write_f32),
        grid_shutdown: Some(grid_shutdown),
        release: Some(release),
        set_history_queue_length: Some(history_len),
        record_history: Some(history),
        post: Some(post),
        developer_lists_valid: None,
    }
}

struct Shipped {
    intrinsic: [i64; 2],
    wealth: [i64; 2],
    water: [i64; 2],
    altitude: [f32; 14],
    ids: [u32; 1],
    weights: [f32; 1],
    desirability: [f32; 4],
}

const SHIPPED: Shipped = Shipped {
    intrinsic: [1, 64],
    wealth: [70, 120],
    water: [50, 10],
    altitude: [0.0, 0.0, 25.0, 5.0, 50.0, 10.0, 75.0, 21.0, 100.0, 50.0, 125.0, 30.0, 150.0, 0.0],
    ids: [0x1030],
    weights: [2.5],
    desirability: [-255.0, -255.0, 255.0, 255.0],
};

fn tuning(s: &Shipped) -> fc4_landvalue_tuning {
    fc4_landvalue_tuning {
        struct_size: std::mem::size_of::<fc4_landvalue_tuning>() as u32,
        has_tract_size: 1,
        tract_size: 2,
        intrinsic: s.intrinsic.as_ptr(),
        intrinsic_count: 2,
        wealth: s.wealth.as_ptr(),
        wealth_count: 2,
        water: s.water.as_ptr(),
        water_count: 2,
        altitude_curve: s.altitude.as_ptr(),
        altitude_curve_count: s.altitude.len(),
        developer_ids: s.ids.as_ptr(),
        developer_id_count: 1,
        developer_weights: s.weights.as_ptr(),
        developer_weight_count: 1,
        desirability_curve: s.desirability.as_ptr(),
        desirability_curve_count: 4,
    }
}

// ---- streams --------------------------------------------------------------------------------------

#[derive(Default)]
struct Out {
    bytes: Vec<u8>,
    tokens: Vec<u64>,
}
fn out_of(u: *mut c_void) -> &'static mut Out {
    unsafe { &mut *(u as *mut Out) }
}
unsafe extern "C" fn put_u8(u: *mut c_void, v: u8) -> bool {
    out_of(u).bytes.push(v);
    true
}
unsafe extern "C" fn put_u16(u: *mut c_void, v: u16) -> bool {
    out_of(u).bytes.extend_from_slice(&v.to_le_bytes());
    true
}
unsafe extern "C" fn put_u32(u: *mut c_void, v: u32) -> bool {
    out_of(u).bytes.extend_from_slice(&v.to_le_bytes());
    true
}
unsafe extern "C" fn put_f32(u: *mut c_void, v: f32) -> bool {
    out_of(u).bytes.extend_from_slice(&v.to_le_bytes());
    true
}
unsafe extern "C" fn put_ref(u: *mut c_void, r: *const fc4_ref) -> bool {
    let o = out_of(u);
    let r = &*r;
    let id = if r.token == 0 {
        0
    } else {
        o.tokens.push(r.token);
        o.tokens.len() as u32
    };
    o.bytes.extend_from_slice(&id.to_le_bytes());
    if id != 0 {
        o.bytes.extend_from_slice(&r.class_id.to_le_bytes());
    }
    true
}
unsafe extern "C" fn no_error(_: *mut c_void) -> i32 {
    0
}

struct In<'a> {
    bytes: &'a [u8],
    pos: usize,
    game: &'a Game,
    iids: Vec<u32>,
}
fn in_of(u: *mut c_void) -> &'static mut In<'static> {
    unsafe { &mut *(u as *mut In<'static>) }
}
fn take<const N: usize>(u: *mut c_void) -> Option<[u8; N]> {
    let i = in_of(u);
    let b: [u8; N] = i.bytes.get(i.pos..i.pos + N)?.try_into().ok()?;
    i.pos += N;
    Some(b)
}
unsafe extern "C" fn get_u8(u: *mut c_void, v: *mut u8) -> bool {
    take::<1>(u).map(|b| *v = b[0]).is_some()
}
unsafe extern "C" fn get_u16(u: *mut c_void, v: *mut u16) -> bool {
    take::<2>(u).map(|b| *v = u16::from_le_bytes(b)).is_some()
}
unsafe extern "C" fn get_u32(u: *mut c_void, v: *mut u32) -> bool {
    take::<4>(u).map(|b| *v = u32::from_le_bytes(b)).is_some()
}
unsafe extern "C" fn get_f32(u: *mut c_void, v: *mut f32) -> bool {
    take::<4>(u).map(|b| *v = f32::from_le_bytes(b)).is_some()
}
unsafe extern "C" fn get_str(_: *mut c_void, _: fc4_bytes_sink, _: *mut c_void) -> bool {
    false
}
/// A loaded grid: a new host object with one reference for the reader.
unsafe extern "C" fn get_ref(u: *mut c_void, iid: u32, out: *mut fc4_ref) -> bool {
    let Some(id) = take::<4>(u).map(u32::from_le_bytes) else { return false };
    *out = fc4_ref::default();
    if id == 0 {
        return true;
    }
    let Some(class) = take::<4>(u).map(u32::from_le_bytes) else { return false };
    let i = in_of(u);
    i.iids.push(iid);
    let s = if class == 0x49B9_E60A { 8 } else { 2 };
    (*out).token = i.game.add(s, |n| if s == 8 { Data::F32(vec![0.0; n]) } else { Data::U8(vec![0; n]) });
    (*out).class_id = class;
    true
}

fn ostream(o: &mut Out) -> fc4_ostream {
    fc4_ostream {
        struct_size: std::mem::size_of::<fc4_ostream>() as u32,
        user: o as *mut Out as *mut c_void,
        put_u8: Some(put_u8),
        put_u16: Some(put_u16),
        put_u32: Some(put_u32),
        put_u64: None,
        put_f32: Some(put_f32),
        put_f64: None,
        put_bytes: None,
        put_str: None,
        put_ref: Some(put_ref),
        error: Some(no_error),
    }
}

fn istream(i: &mut In) -> fc4_istream {
    fc4_istream {
        struct_size: std::mem::size_of::<fc4_istream>() as u32,
        user: i as *mut In as *mut c_void,
        get_u8: Some(get_u8),
        get_u16: Some(get_u16),
        get_u32: Some(get_u32),
        get_u64: None,
        get_f32: Some(get_f32),
        get_f64: None,
        get_bytes: None,
        get_str: Some(get_str),
        get_ref: Some(get_ref),
        skip: None,
        error: Some(no_error),
    }
}

fn take_log(g: &Game) -> Vec<String> {
    std::mem::take(&mut *g.log.borrow_mut())
}

#[test]
fn null_safety() {
    unsafe {
        assert!(fc4_landvalue_create(std::ptr::null()).is_null());
        let g = Game::default();
        let mut h = host(&g);
        h.struct_size = 8;
        assert!(fc4_landvalue_create(&h).is_null());
        let n = std::ptr::null_mut();
        assert!(!fc4_landvalue_init(n, std::ptr::null()));
        assert!(fc4_landvalue_handle_message(n, 0x6695_6816, 0));
        fc4_landvalue_step(n, 5);
        assert_eq!(fc4_landvalue_grid(n, 0), 0);
        assert_eq!(fc4_landvalue_average(n), 0);
        fc4_landvalue_destroy(n);
    }
}

unsafe extern "C" fn edited_altitude(u: *mut c_void, _x: f32, _z: f32) -> f32 {
    game(u).live_altitude.get()
}

unsafe extern "C" fn edited_slope(u: *mut c_void, _x1: i32, _z1: i32, _x2: i32, _z2: i32) -> f32 {
    game(u).live_slope.get()
}

#[test]
fn terrain_edit_between_statistics_and_intrinsic_keeps_each_states_inputs() {
    unsafe {
        let g = Game::default();
        // gen27's terrain edit at vertex (57,61), and slope tract (7,7).
        // Use the measured inputs on a flat fake city to isolate read timing.
        g.live_altitude.set(228.07106);
        g.live_slope.set(8.655487);
        let h = fc4_landvalue_host {
            altitude: Some(edited_altitude),
            area_slope: Some(edited_slope),
            ..host(&g)
        };
        let lv = fc4_landvalue_create(&h);
        assert!(fc4_landvalue_init(lv, &tuning(&SHIPPED)));
        let mut st = fc4_landvalue_state::default();
        for _ in 0..4 {
            fc4_landvalue_step(lv, 1000);
        }
        assert!(fc4_landvalue_get_state(lv, &mut st));
        assert_eq!(st.pipeline_state, 5);
        assert_eq!(st.mean_alt, 228.0); // integer accumulation, spec section 7.2
        assert_eq!(st.min_alt, 228.07106);

        g.live_altitude.set(270.0);
        g.live_slope.set(13.074409);
        assert!(fc4_landvalue_handle_message(lv, 0x278E_6003, 0));
        assert!(fc4_landvalue_get_state(lv, &mut st));
        assert_eq!(st.pipeline_state, 5); // dirty, without restarting the pass
        assert_eq!(st.dirty, 1);
        fc4_landvalue_step(lv, 1000); // state 5 reads the new slope
        let slope = fc4_landvalue_grid(lv, 2);
        let grids = g.grids.borrow();
        let Data::F32(values) = &grids[&slope].data else { panic!() };
        assert!(values.iter().all(|&v| v == 13.074409));
        drop(grids);
        fc4_landvalue_step(lv, 1000); // state 4 reads 270 against the old statistics
        let intrinsic = fc4_landvalue_grid(lv, 0);
        let value = || {
            let grids = g.grids.borrow();
            let Data::U8(values) = &grids[&intrinsic].data else { panic!() };
            values[4] // land tract (1,0), water bonus 45 plus base 1
        };
        assert_eq!(value(), 46); // new altitude is beyond the old curve's upper endpoint
        assert!(fc4_landvalue_get_state(lv, &mut st));
        assert_eq!(st.mean_alt, 228.0);
        while st.pipeline_state != 9 {
            fc4_landvalue_step(lv, 1000);
            assert!(fc4_landvalue_get_state(lv, &mut st));
        }
        assert!(fc4_landvalue_handle_message(lv, 0x6695_6816, 0));
        for _ in 0..6 {
            fc4_landvalue_step(lv, 1000);
        }
        assert!(fc4_landvalue_get_state(lv, &mut st));
        assert_eq!(st.mean_alt, 270.0);
        assert_eq!(value(), 56); // refreshed statistics: curve(50) adds 10
        fc4_landvalue_destroy(lv);
    }
}

#[test]
fn pass_save_load() {
    unsafe {
        let g = Game::default();
        let h = host(&g);
        let lv = fc4_landvalue_create(&h);
        assert!(!lv.is_null());
        let t = tuning(&SHIPPED);
        assert!(fc4_landvalue_init(lv, &t));
        assert_eq!(
            take_log(&g),
            [
                "create 0 2 89D5BB39",
                "create 0 2 89D5BB51",
                "create 1 8 49D5BB6F",
                "history_length 6A663AC8 1"
            ]
        );
        assert_eq!(fc4_landvalue_type_of(lv, 69), 1);
        assert_eq!(fc4_landvalue_type_of(lv, 70), 2);
        assert_eq!(fc4_landvalue_type_of(lv, 120), 3);

        // One pass: 10 calls with one developer and zoned land.
        let mut calls = 0;
        let mut st = fc4_landvalue_state::default();
        loop {
            fc4_landvalue_step(lv, 1000);
            calls += 1;
            assert!(fc4_landvalue_get_state(lv, &mut st));
            if st.pipeline_state == 9 || calls > 50 {
                break;
            }
        }
        assert_eq!(calls, 10);
        // Land: base 1 plus the water bonus 50 - 5d (d = column); every tract
        // (land or not) gets the altitude effect curve(50) = 10 on flat 100 m
        // terrain; the developer adds 2.5 x 40.
        let total = fc4_landvalue_grid(lv, 1);
        let grids = g.grids.borrow();
        let Data::U8(v) = &grids[&total].data else { panic!() };
        let cz = 4;
        assert_eq!(v[0], 10 + 100);
        assert_eq!(v[cz], 46 + 10 + 100);
        assert_eq!(v[2 * cz], 41 + 10 + 100);
        drop(grids);
        // Zoned land tracts: x 1..3, z 0 (x 0 has no land).
        assert_eq!(st.count, 3);
        assert_eq!(st.average_land_value, ((156 + 151 + 146) / 3) as u8);
        assert_eq!(
            take_log(&g),
            [format!("history 6A663AC8 {}", (156 + 151 + 146) / 3), "post 87CC75B3".to_owned()]
        );
        assert_eq!(fc4_landvalue_mode_altitude(lv), 100.0);

        // Save: three references, then dirty-or-running, average, meanAlt.
        let mut o = Out::default();
        assert!(fc4_landvalue_write(lv, &ostream(&mut o)));
        assert_eq!(o.bytes.len(), 32);
        assert_eq!(&o.bytes[..2], &[1, 0]);
        assert_eq!(&o.bytes[6..10], &0x49B9_E602u32.to_le_bytes());
        assert_eq!(&o.bytes[22..26], &0x49B9_E60Au32.to_le_bytes());
        assert_eq!(o.bytes[26], 0);
        assert_eq!(o.bytes[27], st.average_land_value);
        assert_eq!(o.tokens, [fc4_landvalue_grid(lv, 0), total, fc4_landvalue_grid(lv, 2)]);

        // Shutdown: each grid shut down and released once.
        assert!(fc4_landvalue_shutdown(lv));
        assert_eq!(take_log(&g).len(), 6);
        fc4_landvalue_destroy(lv);
        assert!(take_log(&g).is_empty());

        // Load into a new object, never initialised: destroy releases the grids.
        let lv2 = fc4_landvalue_create(&h);
        let mut i = In {
            bytes: &o.bytes,
            pos: 0,
            game: &g,
            iids: Vec::new(),
        };
        assert!(fc4_landvalue_read(lv2, &istream(&mut i)));
        assert_eq!(i.pos, 32);
        assert_eq!(i.iids, [0xC9BD_939A; 3]);
        let mut st2 = fc4_landvalue_state::default();
        assert!(fc4_landvalue_get_state(lv2, &mut st2));
        assert_eq!(st2.average_land_value, st.average_land_value);
        assert_eq!(st2.mode_alt, 0.0);
        assert_eq!(st2.initialized, 0);
        fc4_landvalue_destroy(lv2);
        let log = take_log(&g);
        assert_eq!(log.len(), 3);
        assert!(log.iter().all(|l| l.starts_with("release ")));
    }
}

#[test]
fn deterministic_pacing_runs_the_pass_in_one_idle() {
    // simulator.md §5.7 [choice]: one settle SimIdle runs the whole pass,
    // whatever its budget; the result equals the step-by-step pass.
    unsafe {
        let run = |deterministic: bool| {
            let g = Game::default();
            let h = host(&g);
            let lv = fc4_landvalue_create(&h);
            let t = tuning(&SHIPPED);
            assert!(fc4_landvalue_init(lv, &t));
            let mut st = fc4_landvalue_state::default();
            if deterministic {
                crate::landvalue::test_set_pacing(lv, fc4_core::pacing::Pacing::Deterministic);
                assert!(fc4_landvalue_handle_message(lv, 0x6695_681A, 0));
            } else {
                for _ in 0..50 {
                    fc4_landvalue_step(lv, 1000);
                    assert!(fc4_landvalue_get_state(lv, &mut st));
                    if st.pipeline_state == 9 {
                        break;
                    }
                }
            }
            assert!(fc4_landvalue_get_state(lv, &mut st));
            fc4_landvalue_destroy(lv);
            (st.pipeline_state, st.average_land_value, st.count)
        };
        let original = run(false);
        assert_eq!(original.0, 9);
        assert_eq!(run(true), original, "one delivery, the same pass");
    }
}
