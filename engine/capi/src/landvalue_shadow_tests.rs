// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The land value shadow against a fake game whose "original" is our own
//! simulator (fc4_landvalue_*): a clean pass, one wrong tract, one wrong
//! scalar, a desirability map changed after the pass, the read-only
//! contract, the INCOMPLETE paths and the stats getter inside a pass.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;

use fc4_core::simgrid::{CityCells, GridGeometry};

use crate::landvalue::*;
use crate::landvalue_shadow::*;

const CITY: CityCells = CityCells {
    cells_x: 16,
    cells_z: 8,
    cell_width_x: 16.0,
    cell_width_z: 16.0,
};
const DEV: u64 = 0xDE5;

enum Data {
    U8(Vec<u8>),
    F32(Vec<f32>),
    I16(Vec<i16>),
}

struct Grid {
    geom: GridGeometry,
    data: Data,
}

#[derive(Default)]
struct Game {
    grids: RefCell<HashMap<u64, Grid>>,
    next: RefCell<u64>,
    dmap: RefCell<u64>,
    /// Every call that changes the game (create, write, resize, shutdown,
    /// release, history, post).
    changes: RefCell<Vec<String>>,
}

impl Game {
    fn add(&self, s: i32, data: impl FnOnce(usize) -> Data) -> u64 {
        let geom = GridGeometry::for_city(&CITY, s);
        let mut n = self.next.borrow_mut();
        *n += 0x10;
        self.grids.borrow_mut().insert(
            *n,
            Grid {
                geom,
                data: data(geom.len()),
            },
        );
        *n
    }
    fn u8s(&self, t: u64) -> Vec<u8> {
        match &self.grids.borrow()[&t].data {
            Data::U8(v) => v.clone(),
            _ => panic!("not u8"),
        }
    }
    fn f32s(&self, t: u64) -> Vec<f32> {
        match &self.grids.borrow()[&t].data {
            Data::F32(v) => v.clone(),
            _ => panic!("not f32"),
        }
    }
    fn geometry(&self, t: u64) -> fc4_grid_geometry {
        let x = self.grids.borrow()[&t].geom;
        fc4_grid_geometry {
            tract_size: x.tract_size,
            shift: x.shift,
            count_x: x.count_x,
            count_z: x.count_z,
            width_x: x.width_x,
            width_z: x.width_z,
            inv_width_x: x.inv_width_x,
            inv_width_z: x.inv_width_z,
        }
    }
    fn note(&self, s: String) {
        self.changes.borrow_mut().push(s);
    }
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}
unsafe extern "C" fn terrain_cells(_: *mut c_void, x: *mut i32, z: *mut i32) {
    *x = CITY.cells_x;
    *z = CITY.cells_z;
}
unsafe extern "C" fn is_land_vertex(_: *mut c_void, x: u32, _z: u32) -> bool {
    x > 2
}
unsafe extern "C" fn altitude(_: *mut c_void, x: f32, z: f32) -> f32 {
    60.0 + x / 4.0 + z / 16.0
}
unsafe extern "C" fn min_h(_: *mut c_void) -> f32 {
    20.0
}
unsafe extern "C" fn max_h(_: *mut c_void) -> f32 {
    2500.0
}
unsafe extern "C" fn zone_type(_: *mut c_void, x: i32, z: i32) -> u32 {
    u32::from(x < 10 && z < 4)
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
    let existing = *g.dmap.borrow();
    if existing != 0 {
        return existing;
    }
    let t = g.add(2, |n| Data::I16((0..n).map(|i| (i as i16 * 7) % 90 - 20).collect()));
    *g.dmap.borrow_mut() = t;
    t
}
unsafe extern "C" fn create_grid(u: *mut c_void, kind: u32, s: i32, id: u32) -> u64 {
    let g = game(u);
    g.note(format!("create {id:08X}"));
    g.add(s, |n| {
        if kind == FC4_GRID_U8 {
            Data::U8(vec![0; n])
        } else {
            Data::F32(vec![0.0; n])
        }
    })
}
unsafe extern "C" fn grid_geometry(u: *mut c_void, t: u64, out: *mut fc4_grid_geometry) {
    if game(u).grids.borrow().contains_key(&t) {
        *out = game(u).geometry(t);
    }
}
unsafe extern "C" fn set_tract_size(u: *mut c_void, t: u64, s: i32) {
    game(u).note(format!("resize {t:X} {s}"));
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
    game(u).note(format!("write {t:X}"));
    if let Some(Grid { data: Data::U8(v), .. }) = game(u).grids.borrow_mut().get_mut(&t) {
        v.copy_from_slice(std::slice::from_raw_parts(p, n));
    }
}
unsafe extern "C" fn write_f32(u: *mut c_void, t: u64, p: *const f32, n: usize) {
    game(u).note(format!("write {t:X}"));
    if let Some(Grid { data: Data::F32(v), .. }) = game(u).grids.borrow_mut().get_mut(&t) {
        v.copy_from_slice(std::slice::from_raw_parts(p, n));
    }
}
unsafe extern "C" fn grid_shutdown(u: *mut c_void, t: u64) {
    game(u).note(format!("shutdown {t:X}"));
}
unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    game(u).note(format!("release {t:X}"));
}
unsafe extern "C" fn history_len(u: *mut c_void, s: u32, n: u32) {
    game(u).note(format!("history_length {s:08X} {n}"));
}
unsafe extern "C" fn history(u: *mut c_void, s: u32, v: i32) {
    game(u).note(format!("history {s:08X} {v}"));
}
unsafe extern "C" fn post(u: *mut c_void, m: u32) {
    game(u).note(format!("post {m:08X}"));
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
    }
}

/// The read-only table the plugin hands the shadow: no write callbacks.
fn read_only(g: &Game) -> fc4_landvalue_host {
    fc4_landvalue_host {
        create_grid: None,
        grid_set_tract_size: None,
        grid_write_u8: None,
        grid_write_f32: None,
        grid_shutdown: None,
        release: None,
        set_history_queue_length: None,
        record_history: None,
        post: None,
        ..host(g)
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
    altitude: [
        0.0, 0.0, 25.0, 5.0, 50.0, 10.0, 75.0, 21.0, 100.0, 50.0, 125.0, 30.0, 150.0, 0.0,
    ],
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

/// The "original": our simulator, one finished pass.
struct Original {
    lv: *mut fc4_landvalue,
}

impl Original {
    unsafe fn run(g: &Game, t: &fc4_landvalue_tuning) -> Original {
        let h = host(g);
        let lv = fc4_landvalue_create(&h);
        assert!(fc4_landvalue_init(lv, t));
        let mut st = fc4_landvalue_state::default();
        for _ in 0..100 {
            fc4_landvalue_step(lv, 1000);
            assert!(fc4_landvalue_get_state(lv, &mut st));
            if st.pipeline_state == 9 {
                break;
            }
        }
        assert_eq!(st.pipeline_state, 9);
        Original { lv }
    }
}

impl Drop for Original {
    fn drop(&mut self) {
        unsafe { fc4_landvalue_destroy(self.lv) };
    }
}

/// What the plugin reads from the original, with the arrays it points at.
struct Read {
    intrinsic: Vec<u8>,
    total: Vec<u8>,
    slope: Vec<f32>,
    types: Vec<u8>,
    o: fc4_landvalue_original,
}

unsafe fn read(g: &Game, orig: &Original) -> Read {
    let (ti, tt, ts) = (
        fc4_landvalue_grid(orig.lv, 0),
        fc4_landvalue_grid(orig.lv, 1),
        fc4_landvalue_grid(orig.lv, 2),
    );
    let total = g.u8s(tt);
    let types = total.iter().map(|&v| fc4_landvalue_type_of(orig.lv, v) as u8).collect();
    let mut st = fc4_landvalue_state::default();
    assert!(fc4_landvalue_get_state(orig.lv, &mut st));
    Read {
        intrinsic: g.u8s(ti),
        total,
        slope: g.f32s(ts),
        types,
        o: fc4_landvalue_original {
            struct_size: std::mem::size_of::<fc4_landvalue_original>() as u32,
            has_record: 1,
            intrinsic_geometry: g.geometry(ti),
            intrinsic: std::ptr::null(),
            total_geometry: g.geometry(tt),
            total: std::ptr::null(),
            slope_geometry: g.geometry(ts),
            slope: std::ptr::null(),
            types: std::ptr::null(),
            average: fc4_landvalue_average(orig.lv),
            record_average: st.average_land_value,
            record_flag: u8::from(st.dirty != 0 || st.pipeline_state != 9),
            reserved: 0,
            mode_altitude: fc4_landvalue_mode_altitude(orig.lv),
            mean_altitude: st.mean_alt,
        },
    }
}

impl Read {
    fn original(&self) -> fc4_landvalue_original {
        fc4_landvalue_original {
            intrinsic: self.intrinsic.as_ptr(),
            total: self.total.as_ptr(),
            slope: self.slope.as_ptr(),
            types: self.types.as_ptr(),
            ..self.o
        }
    }
}

unsafe extern "C" fn sink(ctx: *mut c_void, p: *const u8, n: usize) {
    let lines = &mut *(ctx as *mut Vec<String>);
    lines.push(String::from_utf8_lossy(std::slice::from_raw_parts(p, n)).into_owned());
}

unsafe fn compare(s: *mut fc4_landvalue_shadow, r: &Read, lines: &mut Vec<String>) -> i32 {
    let o = r.original();
    fc4_landvalue_shadow_compare(s, &o, 2_451_545, Some(sink), lines as *mut Vec<String> as *mut c_void)
}

#[test]
fn clean_pass_and_read_only() {
    unsafe {
        let g = Game::default();
        let t = tuning(&SHIPPED);
        let orig = Original::run(&g, &t);
        let r = read(&g, &orig);
        assert!(r.o.average > 0, "the fake city must have a non-zero average");
        let ro = read_only(&g);
        let s = fc4_landvalue_shadow_create(&ro, &t, 0);
        assert!(!s.is_null());
        g.changes.borrow_mut().clear();
        let grids_before: Vec<(u64, Vec<u8>)> = [0, 1]
            .iter()
            .map(|&w| fc4_landvalue_grid(orig.lv, w))
            .map(|t| (t, g.u8s(t)))
            .collect();
        assert_eq!(fc4_landvalue_shadow_note_day(s), 0);
        let mut lines = Vec::new();
        assert_eq!(compare(s, &r, &mut lines), 0, "{lines:?}");
        // Nothing in the game changed: no create, write, release, history, post.
        assert!(g.changes.borrow().is_empty(), "{:?}", g.changes.borrow());
        for (t, v) in grids_before {
            assert_eq!(g.u8s(t), v);
        }
        assert_eq!(lines.len(), 1);
        let l = &lines[0];
        println!("{l}");
        assert!(
            l.starts_with("landvalue shadow 2000-01-01 pass 1: compared on 8x4 tracts (S=2)"),
            "{l}"
        );
        assert!(l.contains("intrinsic cells=32 mismatched=0"), "{l}");
        assert!(l.contains("total cells=32 mismatched=0"), "{l}");
        assert!(l.contains("slope cells=2 mismatched=0"), "{l}");
        assert!(l.contains("types mismatched=0/32"), "{l}");
        assert!(
            l.contains(&format!("average orig/ours={}/{};", r.o.average, r.o.average)),
            "{l}"
        );
        assert!(l.contains("settled=1"), "{l}");
        assert!(l.contains("mismatches=0; passes=1 clean=1"), "{l}");
        assert!(!l.contains("first differing"), "{l}");
        let mut st = fc4_landvalue_shadow_stats::default();
        assert!(fc4_landvalue_shadow_get_stats(s, &mut st));
        assert_eq!((st.passes, st.compared, st.clean, st.last_mismatches), (1, 1, 1, 0));
        fc4_landvalue_shadow_destroy(s);
    }
}

#[test]
fn one_wrong_tract_and_one_wrong_scalar() {
    unsafe {
        let g = Game::default();
        let t = tuning(&SHIPPED);
        let orig = Original::run(&g, &t);
        let ro = read_only(&g);
        let s = fc4_landvalue_shadow_create(&ro, &t, 2);
        let mut lines = Vec::new();

        let mut r = read(&g, &orig);
        // Total tract (3, 1) (x outer, 4 tracts per column).
        r.total[3 * 4 + 1] ^= 0x10;
        r.types = r
            .total
            .iter()
            .map(|&v| fc4_landvalue_type_of(orig.lv, v) as u8)
            .collect();
        assert_eq!(compare(s, &r, &mut lines), 1, "{lines:?}");
        let l = &lines[0];
        assert!(l.contains("total cells=32 mismatched=1 max|d|=16"), "{l}");
        assert!(l.contains("mismatches=1;"), "{l}");
        assert!(l.contains("first differing (x,z) orig/ours: total (3,1) "), "{l}");

        let mut r = read(&g, &orig);
        r.o.average = r.o.average.wrapping_add(1);
        assert_eq!(compare(s, &r, &mut lines), 1, "{lines:?}");
        let l = &lines[1];
        assert!(
            l.contains(&format!("average orig/ours={}/{}!", r.o.average, r.o.average - 1)),
            "{l}"
        );
        assert!(l.contains("mismatches=1;") && l.contains("passes=2 clean=0"), "{l}");

        let mut r = read(&g, &orig);
        r.slope[1] += 0.5;
        r.o.mode_altitude += 1.0;
        assert_eq!(compare(s, &r, &mut lines), 2, "{lines:?}");
        println!("{}", lines[2]);
        assert!(
            lines[2].contains("slope cells=2 mismatched=1 max|d|=0.5"),
            "{}",
            lines[2]
        );
        assert!(lines[2].contains("modeAlt orig/ours="), "{}", lines[2]);
        fc4_landvalue_shadow_destroy(s);
    }
}

#[test]
fn desirability_changed_after_the_pass() {
    unsafe {
        let g = Game::default();
        let t = tuning(&SHIPPED);
        let orig = Original::run(&g, &t);
        let r = read(&g, &orig);
        let ro = read_only(&g);
        let s = fc4_landvalue_shadow_create(&ro, &t, 0);
        assert_eq!(fc4_landvalue_shadow_note_day(s), 0);
        // The developer republishes its map after the original's pass read it.
        let d = *g.dmap.borrow();
        if let Some(Grid { data: Data::I16(v), .. }) = g.grids.borrow_mut().get_mut(&d) {
            v.iter_mut().for_each(|x| *x = 90);
        }
        assert_eq!(fc4_landvalue_shadow_note_day(s), 1);
        let mut lines = Vec::new();
        assert_eq!(compare(s, &r, &mut lines), 0, "{lines:?}");
        assert!(lines[0].contains("desirability=previous map"), "{}", lines[0]);
        let mut st = fc4_landvalue_shadow_stats::default();
        assert!(fc4_landvalue_shadow_get_stats(s, &mut st));
        assert_eq!((st.desirability_changes, st.desirability_reruns), (1, 1));
        fc4_landvalue_shadow_destroy(s);
    }
}

#[test]
fn incomplete_and_null_safety() {
    unsafe {
        let g = Game::default();
        let t = tuning(&SHIPPED);
        let ro = read_only(&g);
        assert!(fc4_landvalue_shadow_create(std::ptr::null(), &t, 0).is_null());
        let bad = fc4_landvalue_tuning { struct_size: 4, ..t };
        assert!(fc4_landvalue_shadow_create(&ro, &bad, 0).is_null());
        let n = std::ptr::null_mut();
        assert_eq!(fc4_landvalue_shadow_note_day(n), 0);
        assert_eq!(fc4_landvalue_shadow_compare(n, std::ptr::null(), 0, None, n.cast()), -1);
        let mut st = fc4_landvalue_shadow_stats::default();
        assert!(!fc4_landvalue_shadow_get_stats(n, &mut st));
        fc4_landvalue_shadow_destroy(n);

        // No exemplar: code defaults, no curve, no developer.
        let s = fc4_landvalue_shadow_create(&ro, std::ptr::null(), 0);
        assert!(!s.is_null());
        let mut lines: Vec<String> = Vec::new();
        let ctx = &mut lines as *mut Vec<String> as *mut c_void;
        assert_eq!(
            fc4_landvalue_shadow_compare(s, std::ptr::null(), 0, Some(sink), ctx),
            -1
        );
        assert!(
            lines[0].contains("INCOMPLETE (nothing read from the original)"),
            "{}",
            lines[0]
        );
        assert!(!lines[0].contains("mismatches="), "{}", lines[0]);
        let orig = Original::run(&g, &t);
        let mut r = read(&g, &orig);
        r.o.total_geometry.count_x = 0;
        assert_eq!(compare(s, &r, &mut lines), -1);
        assert!(
            lines[1].contains("INCOMPLETE (the original's total grid has geometry 0x4"),
            "{}",
            lines[1]
        );
        assert!(fc4_landvalue_shadow_get_stats(s, &mut st));
        assert_eq!((st.passes, st.compared, st.last_mismatches), (2, 0, -1));
        fc4_landvalue_shadow_destroy(s);
    }
}

thread_local! {
    /// The shadow a host callback asks for its counters, and what it got:
    /// (answers, passes seen).
    static SHADOW: Cell<*mut fc4_landvalue_shadow> = const { Cell::new(std::ptr::null_mut()) };
    static INSIDE: Cell<(u32, u32)> = const { Cell::new((0, 0)) };
}

unsafe extern "C" fn slope_and_stats(u: *mut c_void, x1: i32, z1: i32, x2: i32, z2: i32) -> f32 {
    let mut st = fc4_landvalue_shadow_stats::default();
    if fc4_landvalue_shadow_get_stats(SHADOW.with(Cell::get), &mut st) {
        INSIDE.with(|c| c.set((c.get().0 + 1, st.passes)));
    }
    area_slope(u, x1, z1, x2, z2)
}

#[test]
fn stats_getter_inside_a_pass() {
    unsafe {
        let g = Game::default();
        let t = tuning(&SHIPPED);
        let orig = Original::run(&g, &t);
        let r = read(&g, &orig);
        let ro = fc4_landvalue_host {
            area_slope: Some(slope_and_stats),
            ..read_only(&g)
        };
        let s = fc4_landvalue_shadow_create(&ro, &t, 0);
        assert!(!s.is_null());
        SHADOW.with(|c| c.set(s));
        let mut lines = Vec::new();
        assert_eq!(compare(s, &r, &mut lines), 0, "{lines:?}");
        // The getter answers from inside our pass (state 5 asks for the area
        // slope), with the counters of the last call that finished.
        let (answers, passes) = INSIDE.with(Cell::get);
        assert!(answers > 0);
        assert_eq!(passes, 0);
        assert_eq!(compare(s, &r, &mut lines), 0, "{lines:?}");
        assert_eq!(INSIDE.with(Cell::get).1, 1);
        let mut st = fc4_landvalue_shadow_stats::default();
        assert!(fc4_landvalue_shadow_get_stats(s, &mut st));
        assert_eq!((st.passes, st.compared, st.clean), (2, 2, 2));
        SHADOW.with(|c| c.set(std::ptr::null_mut()));
        fc4_landvalue_shadow_destroy(s);
    }
}
