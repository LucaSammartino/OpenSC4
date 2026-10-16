// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! terrain.md §18 conformance tests (numbered as there) and the quirks of §16.

use super::*;

const SEA: f32 = 250.0;

/// A `cells × cells` terrain at `h` everywhere, sea 250, view ready.
fn flat(cells: u32, h: f32) -> Terrain {
    let n = ((cells + 1) * (cells + 1)) as usize;
    let mut t = Terrain::from_altitudes(cells, cells, &vec![h; n], SEA, TerrainTuning::SHIPPED).unwrap();
    t.post_city_init();
    t
}

/// The §4.4 cell: h00 = 100, h10 = 104, h01 = 108, h11 = 120 on a 2 × 2
/// city; the other vertices 100, except (2, 2) = 140.
fn worked() -> Terrain {
    let mut a = vec![100.0f32; 9];
    a[1] = 104.0; // (1, 0)
    a[3] = 108.0; // (0, 1)
    a[4] = 120.0; // (1, 1)
    a[8] = 140.0; // (2, 2)
    Terrain::from_altitudes(2, 2, &a, SEA, TerrainTuning::SHIPPED).unwrap()
}

fn no_rand() -> impl FnMut(u32) -> u32 {
    |_| 1
}

// ---- 1, 2: GetAltitude ----------------------------------------------------------------------------

#[test]
fn t01_get_altitude_worked_example() {
    let t = worked();
    assert_eq!(t.altitude(4.0, 12.0), 109.0);
    assert_eq!(t.altitude(12.0, 4.0), 107.0);
    assert_eq!(t.altitude(8.0, 8.0), 110.0);
    assert_eq!(t.altitude(0.0, 0.0), 100.0);
    assert_eq!(t.altitude(-0.001, 0.0), 0.0);
    assert_eq!(t.altitude(32.0, 32.0), 140.0);
    assert_eq!(t.altitude(32.001, 0.0), 0.0);
    assert_eq!(t.altitude(f32::NAN, 0.0), 0.0);
}

#[test]
fn t02_floor_at_cell_boundaries() {
    assert_eq!(Terrain::cell_floor(16.0), 1);
    assert_eq!(Terrain::cell_floor(15.999), 0);
    assert_eq!(Terrain::cell_floor(0.0), 0);
    // x = sizeX uses the last cell with fx = 16: the corner vertex.
    let t = worked();
    assert_eq!(t.altitude(32.0, 0.0), t.altitude_at_vertex(2, 0));
    // Continuity across the boundary.
    assert_eq!(t.altitude(16.0, 0.0), 104.0);
    assert!((t.altitude(15.999, 0.0) - 104.0).abs() < 0.01);
}

#[test]
fn quirk1_flip_flags_are_ignored() {
    let mut t = worked();
    let before = [t.altitude(12.0, 4.0), t.altitude(4.0, 12.0)];
    t.set_vertex_flags(0, 0, flag::FLIPPED_MASK | flag::FLIPPED_BY_CLIFF);
    assert!(t.is_triangulation_flipped(0, 0));
    assert_eq!([t.altitude(12.0, 4.0), t.altitude(4.0, 12.0)], before);
}

// ---- 3: GetFaceNormal ------------------------------------------------------------------------------

fn close3(got: [f32; 3], want: [f64; 3]) {
    for k in 0..3 {
        assert!((f64::from(got[k]) - want[k]).abs() < 1e-6, "got {got:?}, want {want:?}");
    }
}

#[test]
fn t03_face_normal() {
    let t = worked();
    let s = 1.0 / (4096.0f64 + 65536.0 + 65536.0).sqrt();
    close3(t.face_normal(12.0, 4.0).unwrap(), [-64.0 * s, 256.0 * s, -256.0 * s]);
    let s = 1.0 / (192.0f64 * 192.0 + 128.0 * 128.0 + 65536.0).sqrt();
    close3(t.face_normal(4.0, 12.0).unwrap(), [-192.0 * s, 256.0 * s, -128.0 * s]);
    assert_eq!(t.face_normal(32.0, 4.0), None);
    assert_eq!(t.face_normal(4.0, 32.0), None);
    assert_eq!(t.face_normal(-0.5, 4.0), None);
    // Flat ground: straight up.
    close3(flat(2, 300.0).face_normal(3.0, 5.0).unwrap(), [0.0, 1.0, 0.0]);
}

#[test]
fn other_height_reads() {
    let t = worked();
    assert_eq!(t.altitude_at_index(4), 120.0);
    assert_eq!(t.altitude_at_index(9), 0.0);
    assert_eq!(t.altitude_at_vertex(1, 1), 120.0);
    assert_eq!(t.altitude_at_vertex(-1, 0), 0.0);
    assert_eq!(t.altitude_at_vertex(3, 0), 0.0);
    assert_eq!(t.cell_corner_altitudes(0, 0), Some([100.0, 108.0, 120.0, 104.0]));
    assert_eq!(t.cell_corner_altitudes(2, 0), None);
    // Nearest grid, ties to even: 8 / 16 = 0.5 → 0; 24 / 16 = 1.5 → 2.
    assert_eq!(t.altitude_at_nearest_grid(8.0, 8.0), 100.0);
    assert_eq!(t.altitude_at_nearest_grid(24.0, 24.0), 140.0);
    assert_eq!(t.altitude_at_nearest_grid(17.0, 15.0), 120.0);
    assert_eq!(t.altitude_at_nearest_grid(-1.0, 0.0), 0.0);
    assert_eq!(t.altitude_checksum(VertRect::new(0, 0, 1, 1)), 432.0);
    assert_eq!(t.altitude_checksum(VertRect::new(1, 0, 0, 1)), 0.0);
    assert_eq!(t.altitude_checksum(VertRect::new(0, 0, 3, 1)), 0.0);
    assert_eq!(t.altitudes_in(VertRect::new(0, 0, 1, 1)), [100.0, 104.0, 108.0, 120.0]);
    assert_eq!(t.vert_index(1, 2), 7);
    assert_eq!(t.vertex_of(7), (1, 2));
    assert!(t.location_is_in_bounds(32.0, 0.0));
    assert!(!t.location_is_in_bounds(0.0, -0.0001));
}

// ---- 4: CellHasWater / CellHasLand -------------------------------------------------------------------

fn corners(t: &mut Terrain, d: [f32; 4]) {
    // (0,0), (1,0), (0,1), (1,1)
    t.set_water_depth(0, 0, d[0]);
    t.set_water_depth(1, 0, d[1]);
    t.set_water_depth(0, 1, d[2]);
    t.set_water_depth(1, 1, d[3]);
}

#[test]
fn t04_cell_has_water_and_land() {
    let mut t = flat(2, 300.0);
    corners(&mut t, [-1.0, -1.0, -1.0, 0.5]);
    assert!(t.cell_has_water(0, 0) && t.cell_has_land(0, 0));
    corners(&mut t, [0.0; 4]);
    assert!(!t.cell_has_water(0, 0) && t.cell_has_land(0, 0));
    corners(&mut t, [f32::NAN, -1.0, -1.0, -1.0]);
    assert!(!t.cell_has_water(0, 0) && t.cell_has_land(0, 0));
    corners(&mut t, [f32::NAN; 4]);
    assert!(!t.cell_has_water(0, 0) && !t.cell_has_land(0, 0));
    assert!(!t.cell_has_water(2, 0) && !t.cell_has_land(2, 0));
    assert!(!t.cell_has_land(-1, 0));
}

#[test]
fn vertex_water_reads() {
    let mut t = worked();
    // Sea 250 over ground at 100-140: everything is sea water.
    assert_eq!(t.water_depth_at_vertex(1, 1), 130.0);
    assert_eq!(t.water_level(20.0, 20.0), 250.0);
    assert_eq!(t.water_level_at_index(8), 250.0);
    assert!(t.is_water_vertex(0, 0) && !t.is_land_vertex(0, 0) && t.is_sea_vertex(0, 0));
    assert!(!t.is_water_vertex(3, 0) && !t.is_land_vertex(3, 0) && !t.is_sea_vertex(3, 0));
    assert_eq!(t.number_of_water_vertices(VertRect::new(0, 0, 2, 2)), 9);
    t.set_water_depth(1, 1, -2.0);
    assert_eq!(t.number_of_water_vertices(VertRect::new(-5, -5, 9, 9)), 8);
    // Max surface: the corners' altitude + max(depth, 0).
    assert_eq!(t.max_surface_level(0, 0), 250.0);
    assert_eq!(t.max_surface_level(5, 0), 0.0);
    let mut f = flat(2, 300.0);
    f.set_water_depth(1, 1, 3.0);
    assert_eq!(f.max_surface_level(0, 0), 303.0);
    assert_eq!(f.max_surface_level(1, 1), 303.0);
}

// ---- 5, 6: surface water cell queries ----------------------------------------------------------------

fn cell_depths(t: &mut Terrain, d: [f32; 4]) {
    // i, i + 1, i + 1 + vertsX, i + vertsX for the cell (0, 0)
    t.set_water_depth(0, 0, d[0]);
    t.set_water_depth(1, 0, d[1]);
    t.set_water_depth(1, 1, d[2]);
    t.set_water_depth(0, 1, d[3]);
}

#[test]
fn t05_water_depth_for_cell() {
    let mut t = flat(2, 300.0);
    cell_depths(&mut t, [2.0, 3.0, 0.5, 4.0]);
    assert_eq!(t.water_depth_for_cell(0, 0), 0.5);
    cell_depths(&mut t, [2.0, -1.0, 3.0, 3.0]);
    assert_eq!(t.water_depth_for_cell(0, 0), 0.0);
    cell_depths(&mut t, [6.0, 6.0, 6.0, 5.0]);
    assert!(!t.has_draft_clearance(0, 0));
    cell_depths(&mut t, [6.0, 6.0, 6.0, 5.001]);
    assert!(t.has_draft_clearance(0, 0));
    // A NaN candidate never replaces the minimum; a NaN first value stays.
    cell_depths(&mut t, [6.0, f32::NAN, 7.0, 8.0]);
    assert_eq!(t.water_depth_for_cell(0, 0), 6.0);
    cell_depths(&mut t, [f32::NAN, 1.0, 7.0, 8.0]);
    assert_eq!(t.water_depth_for_cell(0, 0), 0.0);
}

#[test]
fn t06_water_type() {
    let mut t = flat(2, 300.0);
    t.set_water_depth(0, 0, -2.0);
    assert_eq!(t.water_type(0, 0), water_type::LAND);
    let mut t = Terrain::from_altitudes(1, 1, &[247.0, 260.0, 0.0, 0.0], SEA, TerrainTuning::SHIPPED).unwrap();
    t.set_water_depth(0, 0, 3.0);
    t.set_water_depth(1, 0, 3.0);
    assert_eq!(t.water_type(0, 0), water_type::SEA);
    assert_eq!(t.water_type(1, 0), water_type::FRESH);
    // A NaN depth is not `≤ 0`, and its NaN level is "above the sea" (§10.2).
    t.set_water_depth(1, 0, f32::NAN);
    assert_eq!(t.water_type(1, 0), water_type::FRESH);
}

// ---- 7, 8: setters ------------------------------------------------------------------------------------

/// A one-cell terrain whose vertex 0 is at `alt` with `depth`.
fn one(alt: f32, depth: f32) -> Terrain {
    let mut t = Terrain::from_altitudes(1, 1, &[alt, 300.0, 300.0, 300.0], SEA, TerrainTuning::SHIPPED).unwrap();
    t.set_water_depth(0, 0, depth);
    t
}

#[test]
fn t07_set_altitude_at_vertex() {
    let mut t = one(260.0, -10.0);
    t.set_altitude_at_vertex(0, 270.0);
    assert_eq!((t.altitude_at_index(0), t.water_depth_at_index(0)), (270.0, -10.0));
    let mut t = one(260.0, -10.0);
    t.set_altitude_at_vertex(0, 240.0);
    assert_eq!((t.altitude_at_index(0), t.water_depth_at_index(0)), (240.0, 10.0));
    let mut t = one(240.0, 10.0);
    t.set_altitude_at_vertex(0, 245.0);
    assert_eq!(t.water_depth_at_index(0), 5.0);
    let mut t = one(240.0, 10.0);
    t.set_altitude_at_vertex(0, 255.0);
    assert_eq!(t.water_depth_at_index(0), -5.0);
    // No clamp.
    let mut t = one(260.0, -10.0);
    t.set_altitude_at_vertex(0, 5000.0);
    assert_eq!(t.altitude_at_index(0), 5000.0);
    // Locked: nothing.
    let mut t = one(240.0, 10.0);
    t.lock_altitude(VertRect::point(0, 0), true);
    t.set_altitude_at_vertex(0, 255.0);
    assert_eq!((t.altitude_at_index(0), t.water_depth_at_index(0)), (240.0, 10.0));
}

#[test]
fn t08_set_altitude_xz() {
    let mut t = one(260.0, -10.0);
    t.set_altitude(0, 0, 3000.0);
    assert_eq!((t.altitude_at_index(0), t.water_depth_at_index(0)), (2500.0, -2250.0));
    let mut t = one(30.0, 220.0);
    t.set_altitude(0, 0, 10.0);
    assert_eq!((t.altitude_at_index(0), t.water_depth_at_index(0)), (20.0, 230.0));
    let mut t = one(260.0, -10.0);
    t.lock_altitude(VertRect::new(0, 0, 1, 1), true);
    t.set_altitude(0, 0, 3000.0);
    t.set_altitude(0, 0, 10.0);
    assert_eq!((t.altitude_at_index(0), t.water_depth_at_index(0)), (260.0, -10.0));
    t.lock_altitude(VertRect::new(0, 0, 1, 1), false);
    t.set_altitude(0, 0, f32::NAN);
    t.set_altitude(0, 0, 260.0);
    assert_eq!((t.altitude_at_index(0), t.water_depth_at_index(0)), (260.0, -10.0));
    // (cellsX + 1) × z + x with no bounds check: (2, 0) is the vertex (0, 1).
    t.set_altitude(2, 0, 280.0);
    assert_eq!(t.altitude_at_vertex(0, 1), 280.0);
    t.set_altitude(-1, 0, 280.0); // outside the arrays: nothing
    t.set_altitude(0, 9, 280.0);
}

#[test]
fn raise_and_lower_vertex() {
    let mut t = one(300.0, 7.0); // a lake vertex (water above the sea)
    t.raise_vertex(0, 0.0);
    // [quirk 5] d = 0 still erases the lake water.
    assert_eq!(t.water_depth_at_index(0), -50.0);
    t.raise_vertex(0, 5000.0);
    assert_eq!(t.altitude_at_index(0), 2500.0);
    t.lower_vertex(0, 5000.0);
    assert_eq!((t.altitude_at_index(0), t.water_depth_at_index(0)), (20.0, 230.0));
    t.raise_vertex(99, 1.0); // past the arrays: nothing
    t.lower_vertex(99, 1.0);
}

#[test]
fn set_altitude_rect_resets_water_and_redisplays() {
    let mut t = flat(4, 300.0);
    t.set_water_depth(1, 1, 3.0); // a lake vertex
    t.set_altitude_rect(VertRect::new(1, 1, 2, 2), 200.0, true, kind::LEVELLING);
    assert_eq!(t.altitude_at_vertex(2, 2), 200.0);
    // Below the sea: depth = sea − alt; the lake vertex too.
    assert_eq!(t.water_depth_at_vertex(1, 1), 50.0);
    let p = t.take_posts();
    assert_eq!(p.len(), 3);
    assert_eq!(
        (p[0].msg, p[0].data1, p[0].data2, p[0].data4),
        (msg::TERRAIN_CHANGED, 0x0001_0002, 0x0001_0002, 5)
    );
    assert_eq!(p[1].msg, msg::WATER_CHANGED);
    // Above the sea after the reset: depth 0 (not sea − alt).
    t.set_altitude_rect(VertRect::new(1, 1, 1, 1), 280.0, true, kind::LEVELLING);
    assert_eq!(t.water_depth_at_vertex(1, 1), 0.0);
    // Without redisplay: no water reset, no message; the land quirk applies.
    let mut t = flat(4, 300.0);
    t.set_altitude_rect(VertRect::new(0, 0, 4, 4), 320.0, false, 0);
    assert_eq!(t.water_depth_at_vertex(3, 3), -50.0);
    assert!(t.take_posts().is_empty());
}

// ---- 9, 10: messages ------------------------------------------------------------------------------

#[test]
fn t09_set_sea_level() {
    let mut t = flat(4, 300.0);
    assert!(!t.set_sea_level(201.0));
    assert!(!t.set_sea_level(-10.5));
    assert!(!t.set_sea_level(f32::NAN));
    assert!(t.take_posts().is_empty());
    assert_eq!(t.sea_level(), SEA);
    t.set_water_depth(0, 0, 9.0);
    t.set_vertex_flags(0, 0, 0);
    assert!(t.set_sea_level(100.0));
    let p = t.take_posts();
    assert_eq!(
        p[0],
        Post {
            msg: msg::SEA_LEVEL_CHANGED,
            ..Post::default()
        }
    );
    assert_eq!(
        p[1],
        Post {
            msg: msg::TERRAIN_CHANGED,
            data1: 4,
            data2: 4,
            data3: 0,
            data4: 0
        }
    );
    assert_eq!(p[2].msg, msg::WATER_CHANGED);
    assert_eq!(p[3].msg, msg::TEXTURE_REDISPLAY);
    assert_eq!(p.len(), 4);
    assert_eq!(t.water_depth_at_vertex(0, 0), -200.0);
    assert!(t.set_sea_level(-10.0) && t.set_sea_level(200.0));
}

#[test]
fn t10_redisplay_packing() {
    let mut t = flat(16, 300.0);
    t.set_view_ready(false);
    t.redisplay_terrain(true, true, VertRect::new(3, 5, 10, 12), 4);
    assert!(t.take_posts().is_empty());
    t.post_city_init();
    t.redisplay_terrain(true, true, VertRect::new(3, 5, 10, 12), 4);
    let p = t.take_posts();
    let want = |m| Post {
        msg: m,
        data1: 0x0003_000A,
        data2: 0x0005_000C,
        data3: 0,
        data4: 4,
    };
    assert_eq!(p[0], want(msg::TERRAIN_CHANGED));
    assert_eq!(p[1], want(msg::WATER_CHANGED));
    assert_eq!(p[2].msg, msg::TEXTURE_REDISPLAY);
    assert_eq!(p.len(), 3);
    // Water only; texture only.
    t.redisplay_terrain(false, true, VertRect::new(0, 0, 1, 1), 3);
    t.redisplay_terrain(false, false, VertRect::new(0, 0, 1, 1), 0);
    let m: Vec<u32> = t.take_posts().iter().map(|p| p.msg).collect();
    assert_eq!(m, [msg::WATER_CHANGED, msg::TEXTURE_REDISPLAY, msg::TEXTURE_REDISPLAY]);
}

#[test]
fn redisplay_cells_converts_to_vertices() {
    let mut t = flat(8, 300.0);
    t.redisplay_terrain_cells(true, false, (5, 2), (3, 7), 1);
    let p = t.take_posts();
    assert_eq!((p[0].data1, p[0].data2, p[0].data4), (0x0003_0006, 0x0002_0008, 1));
    t.redisplay_terrain_cells(true, false, (7, 7), (7, 7), 1);
    assert_eq!(t.take_posts()[0].data1, 0x0007_0008);
}

#[test]
fn long_operations_terrain_type_and_display_state() {
    let mut t = flat(2, 300.0);
    t.set_view_ready(false);
    t.begin_long_terrain_operation();
    t.begin_long_terrain_operation();
    t.end_long_terrain_operation();
    let m: Vec<u32> = t.take_posts().iter().map(|p| p.msg).collect();
    assert_eq!(
        m,
        [
            msg::BEGIN_LONG_OPERATION,
            msg::BEGIN_LONG_OPERATION,
            msg::END_LONG_OPERATION
        ]
    );
    t.post_city_init();
    t.set_terrain_type(0);
    assert!(t.take_posts().is_empty());
    t.set_terrain_type(2);
    let p = t.take_posts();
    assert_eq!((p[0].msg, p[0].data1), (msg::TERRAIN_TYPE_CHANGED, 2));
    assert_eq!(p[1].msg, msg::TEXTURE_REDISPLAY);
    assert_eq!(p.len(), 2);
    t.set_water_display_state(1);
    assert_eq!(
        t.take_posts()[0],
        Post {
            msg: msg::WATER_DISPLAY_STATE,
            data1: 1,
            ..Post::default()
        }
    );
    assert_eq!(t.water_display_state(), 1);
}

#[test]
fn quirk2_min_max_cache_goes_stale() {
    let mut t = flat(2, 300.0);
    t.set_view_ready(false);
    t.set_altitude(1, 1, 500.0);
    assert_eq!((t.min_height(), t.max_height()), (300.0, 300.0));
    // Before the view is ready, RedisplayTerrain does not recompute either.
    t.redisplay_terrain(true, true, t.whole_city(), 0);
    assert_eq!(t.max_height(), 300.0);
    t.post_city_init();
    t.redisplay_terrain(false, true, t.whole_city(), 0);
    assert_eq!(t.max_height(), 300.0);
    t.redisplay_terrain(true, false, t.whole_city(), 0);
    assert_eq!(t.max_height(), 500.0);
}

#[test]
fn cell_flags_pack_the_vertex_bits() {
    let mut t = flat(2, 300.0);
    t.set_vertex_flags(1, 1, 0x0260 | flag::CLIFF);
    assert_eq!(t.cell_flags(1, 1), 1 | 2 | 4);
    t.set_cell_flags(1, 1, 8);
    assert_eq!(t.vertex_flags(1, 1), flag::FLIPPED_EXTERNAL | flag::CLIFF);
    assert!(t.is_triangulation_flipped(1, 1));
    assert_eq!(t.cell_flags(2, 1), 0);
}

// ---- 11, 12: brushes ------------------------------------------------------------------------------

fn sinkhole_params() -> BrushParams {
    BrushParams {
        from_x: 160.0,
        from_z: 160.0,
        to_x: 160.0,
        to_z: 160.0,
        level: 0,
        redisplay: true,
        strength_scale: 1.0,
        radius_scale: 1.0,
        data4: kind::DISASTER,
        out: None,
        target_height: 100.0,
    }
}

/// A grey test image: `v × 7 + u × 3` mod 256.
fn pattern(w: u32, h: u32) -> BrushBitmap {
    let px = (0..h)
        .flat_map(|v| (0..w).map(move |u| ((v * 7 + u * 3) % 256) as u8))
        .collect();
    BrushBitmap::new(w, h, px).unwrap()
}

#[test]
fn t11_pipe_burst_sinkhole() {
    for (w, h) in [(16, 16), (64, 64), (8, 32), (2, 2)] {
        let mut t = flat(20, 300.0);
        let bmp = pattern(w, h);
        assert!(t.add_brush(
            brush_id::PIPE_SINK_HOLE,
            shipped_brush(brush_id::PIPE_SINK_HOLE, Some(bmp)).unwrap()
        ));
        let mut p = sinkhole_params();
        assert!(t.apply_brush(brush_id::PIPE_SINK_HOLE, &mut p, &mut no_rand()));
        let out = p.out.expect("touched");
        let floor = 300.0 - 255.0 * 0.01;
        for z in 0..=20 {
            for x in 0..=20 {
                let a = t.altitude_at_vertex(x, z);
                assert!(a <= 300.0, "raised ({x}, {z})");
                assert!(a >= floor - 1e-4, "too deep ({x}, {z}): {a}");
                if a != 300.0 {
                    // Inside the footprint centred on (160, 160): 16 m wide
                    // and 16·H/W m tall (§9.3).
                    let half_z = (8 * h / w) as i32;
                    assert!((16 * x - 160).abs() <= 8 && (16 * z - 160).abs() <= half_z);
                    assert!(out.x0 <= x && x <= out.x1 && out.z0 <= z && z <= out.z1);
                }
            }
        }
        if w == h {
            // A square 16 m footprint holds only the centre vertex.
            assert_eq!(out, VertRect::point(10, 10));
        }
        let p = t.take_posts();
        let changed: Vec<&Post> = p.iter().filter(|p| p.msg == msg::TERRAIN_CHANGED).collect();
        assert_eq!(changed.len(), 1);
        let (d1, d2) = out.packed();
        assert_eq!((changed[0].data1, changed[0].data2, changed[0].data4), (d1, d2, 3));
        if w == h {
            assert_eq!((d1, d2), (0x000A_000A, 0x000A_000A));
        }
    }
}

#[test]
fn t11b_sinkhole_depth_follows_the_pixel() {
    let mut t = flat(20, 300.0);
    let bmp = BrushBitmap::uniform(4, 4, 0);
    t.add_brush(
        brush_id::PIPE_SINK_HOLE,
        shipped_brush(brush_id::PIPE_SINK_HOLE, Some(bmp)).unwrap(),
    );
    let mut p = sinkhole_params();
    t.apply_brush(brush_id::PIPE_SINK_HOLE, &mut p, &mut no_rand());
    let want = crate::x87::r24(300.0 - crate::x87::r24(255.0 * f64::from(0.01f32))) as f32;
    assert_eq!(t.altitude_at_vertex(10, 10), want);
    assert_eq!(
        t.water_depth_at_vertex(10, 10),
        crate::x87::r24(250.0 - f64::from(want)) as f32
    );
    // Level 4 and a strength scale.
    let mut t = flat(20, 300.0);
    t.add_brush(
        brush_id::PIPE_SINK_HOLE,
        shipped_brush(brush_id::PIPE_SINK_HOLE, Some(BrushBitmap::uniform(4, 4, 55))).unwrap(),
    );
    let mut p = BrushParams {
        level: 4,
        strength_scale: 2.0,
        ..sinkhole_params()
    };
    t.apply_brush(brush_id::PIPE_SINK_HOLE, &mut p, &mut no_rand());
    assert!((t.altitude_at_vertex(10, 10) - (300.0 - 200.0 * 0.0035 * 2.0)).abs() < 1e-4);
}

#[test]
fn t12_unknown_brush() {
    let mut t = flat(20, 300.0);
    let mut p = BrushParams {
        out: Some(VertRect::new(1, 2, 3, 4)),
        ..sinkhole_params()
    };
    assert!(!t.apply_brush(0x1234_5678, &mut p, &mut no_rand()));
    assert_eq!(p.out, None);
    assert!(t.take_posts().is_empty());
}

#[test]
fn brush_edge_cases() {
    let mut t = flat(20, 300.0);
    // A kind above 11 is rejected; so are five components.
    let mut d = shipped_brush(brush_id::PIPE_SINK_HOLE, Some(BrushBitmap::uniform(4, 4, 0))).unwrap();
    d.components[0].kind = 12;
    assert!(!t.add_brush(7, d.clone()));
    d.components[0].kind = 0;
    d.components = vec![d.components[0].clone(); 5];
    assert!(!t.add_brush(7, d));
    // Centre out of the vertices: nothing applied, nothing posted, true.
    t.add_brush(
        brush_id::PIPE_SINK_HOLE,
        shipped_brush(brush_id::PIPE_SINK_HOLE, Some(BrushBitmap::uniform(4, 4, 0))).unwrap(),
    );
    let mut p = BrushParams {
        to_x: 400.0,
        to_z: 160.0,
        ..sinkhole_params()
    };
    assert!(t.apply_brush(brush_id::PIPE_SINK_HOLE, &mut p, &mut no_rand()));
    assert_eq!(p.out, None);
    assert!(t.take_posts().is_empty());
    // Neutral pixels: nothing changes.
    t.add_brush(
        brush_id::PIPE_SINK_HOLE,
        shipped_brush(brush_id::PIPE_SINK_HOLE, Some(BrushBitmap::uniform(4, 4, 255))).unwrap(),
    );
    let mut p = sinkhole_params();
    t.apply_brush(brush_id::PIPE_SINK_HOLE, &mut p, &mut no_rand());
    assert_eq!(p.out, None);
    // Redisplay off: no message even when vertices changed.
    t.add_brush(
        brush_id::PIPE_SINK_HOLE,
        shipped_brush(brush_id::PIPE_SINK_HOLE, Some(BrushBitmap::uniform(4, 4, 0))).unwrap(),
    );
    let mut p = BrushParams {
        redisplay: false,
        ..sinkhole_params()
    };
    t.apply_brush(brush_id::PIPE_SINK_HOLE, &mut p, &mut no_rand());
    assert_eq!(p.out, Some(VertRect::point(10, 10)));
    assert!(t.take_posts().is_empty());
}

#[test]
fn earthquake_fault_raises_along_the_stroke() {
    // A 64 m footprint (radius 4 × scale 16), 4 × 4 bitmap: the left half
    // above neutral, the right half below.
    let mut t = flat(20, 300.0);
    let px: Vec<u8> = (0..16).map(|i| if i % 4 < 2 { 228 } else { 28 }).collect();
    let bmp = BrushBitmap::new(4, 4, px).unwrap();
    t.add_brush(
        brush_id::EARTHQUAKE_FAULT,
        shipped_brush(brush_id::EARTHQUAKE_FAULT, Some(bmp)).unwrap(),
    );
    // A stroke along +z: angle = atan2(16, 0) + π/2 = π, so U runs against x.
    let mut p = BrushParams {
        from_x: 160.0,
        from_z: 144.0,
        to_x: 160.0,
        to_z: 160.0,
        level: 0,
        redisplay: false,
        radius_scale: 16.0,
        ..BrushParams::default()
    };
    assert!(t.apply_brush(brush_id::EARTHQUAKE_FAULT, &mut p, &mut no_rand()));
    let angle = t.brush(brush_id::EARTHQUAKE_FAULT).unwrap().angle;
    assert!((f64::from(angle) - std::f64::consts::PI).abs() < 1e-6);
    // k = 16 / (64 / 4) = 1 pixel per vertex: U = −dx + 2, V = −dz + 2.
    // dx = −1, 0 → U = 3, 2 (right half, lowered); dx = 1, 2 → U = 1, 0 (raised).
    assert!(t.altitude_at_vertex(9, 10) < 300.0);
    assert!(t.altitude_at_vertex(10, 10) < 300.0);
    assert!(t.altitude_at_vertex(11, 10) > 300.0);
    assert!(t.altitude_at_vertex(12, 10) > 300.0);
    assert_eq!(t.altitude_at_vertex(8, 10), 300.0);
    assert_eq!(t.altitude_at_vertex(13, 10), 300.0);
    // sin(π) in single precision is not 0, so V drifts by a fraction of a
    // pixel along the rows: the fringe rows z = 8 and 13 may be reached.
    let out = p.out.unwrap();
    assert!(out.x0 <= 9 && out.x1 >= 12 && out.z0 <= 9 && out.z1 >= 12, "{out:?}");
    assert!(out.x0 >= 8 && out.x1 <= 13 && out.z0 >= 8 && out.z1 <= 13, "{out:?}");
    // from = to reuses the stored angle [quirk 6].
    let mut q = BrushParams { from_z: 160.0, ..p };
    t.apply_brush(brush_id::EARTHQUAKE_FAULT, &mut q, &mut no_rand());
    assert_eq!(t.brush(brush_id::EARTHQUAKE_FAULT).unwrap().angle, angle);
}

#[test]
fn quirk7_low_side_half_pixel() {
    // k = 16 / (16 / 2) = 2 pixels per vertex on a 2 × 2 bitmap: U = 2·dx + 1.
    // A 45° rotation puts the vertex (−1, 0) at U = 1 − √2 ≈ −0.41: truncated
    // to column 0 and therefore inside the footprint.
    let mut t = flat(20, 300.0);
    let mut d = shipped_brush(brush_id::PIPE_SINK_HOLE, Some(BrushBitmap::uniform(2, 2, 0))).unwrap();
    d.angle = std::f32::consts::FRAC_PI_4;
    t.add_brush(brush_id::PIPE_SINK_HOLE, d);
    let mut p = sinkhole_params();
    t.apply_brush(brush_id::PIPE_SINK_HOLE, &mut p, &mut no_rand());
    // (dx, dz) = (−1, 0): U = 1 − √2·… → trunc 0; V = 1 + √2 → 2 (out). So
    // only vertices whose U and V both land in (−1, 2) count.
    let k = 2.0f64;
    let (c, s) = (std::f64::consts::FRAC_1_SQRT_2, std::f64::consts::FRAC_1_SQRT_2);
    for dz in -2..=2 {
        for dx in -2..=2 {
            let u = s * f64::from(dz) * k + c * f64::from(dx) * k + 1.0;
            let v = c * f64::from(dz) * k - s * f64::from(dx) * k + 1.0;
            let inside = u > -1.0 && u < 2.0 && v > -1.0 && v < 2.0;
            let lowered = t.altitude_at_vertex(10 + dx, 10 + dz) < 300.0;
            assert_eq!(lowered, inside, "({dx}, {dz}) u {u} v {v}");
        }
    }
}

#[test]
fn random_walk_angle() {
    let mut t = flat(20, 300.0);
    let mut d = shipped_brush(brush_id::PIPE_SINK_HOLE, Some(BrushBitmap::uniform(2, 2, 0))).unwrap();
    d.components[0].oriented = false;
    d.random_walk_step = 1.0;
    d.angle = 0.5;
    t.add_brush(1, d);
    let mut draws = vec![];
    let mut p = sinkhole_params();
    t.apply_brush(1, &mut p, &mut |n| {
        draws.push(n);
        0
    });
    // RandomUint32Uniform(5) = 0: angle −= step, wrapped to [0, 2π).
    let a = t.brush(1).unwrap().angle;
    assert_eq!(draws, [5]);
    assert!((f64::from(a) - (0.5 - 1.0 + 2.0 * std::f64::consts::PI)).abs() < 1e-5);
    // Any other draw: angle += step, wrapped back to 0.5.
    t.apply_brush(1, &mut p, &mut |_| 3);
    assert!((f64::from(t.brush(1).unwrap().angle) - 0.5).abs() < 1e-5);
}

// ---- 13: lava -----------------------------------------------------------------------------------------

#[derive(Default)]
struct Fire {
    calls: Vec<String>,
    burn_cells: Vec<Option<(i32, i32)>>,
}

impl LavaHost for Fire {
    fn is_cell_flammable(&mut self, _x: i32, _z: i32) -> bool {
        true
    }
    fn ignite_cell(&mut self, x: i32, z: i32) {
        self.calls.push(format!("ignite {x} {z}"));
    }
    fn quick_burn_cell(&mut self, x: i32, z: i32) {
        self.calls.push(format!("burn {x} {z}"));
    }
    fn set_cause(&mut self, c: u32) {
        self.calls.push(format!("cause {c}"));
    }
    fn next_burn_cell(&mut self) -> Option<(i32, i32)> {
        if self.burn_cells.is_empty() {
            None
        } else {
            self.burn_cells.remove(0)
        }
    }
}

#[test]
fn t13_lava_flow_rule() {
    let t = flat(10, 300.0);
    let dt = 33333.0f32;
    let mut lava = Lava::start(&t, 80.0, 80.0, 20.0, LavaTuning::SHIPPED);
    lava.set_level(&t, 5, 5, 10.0);
    for (dx, dz) in NEIGHBOURS {
        lava.set_level(&t, 5 + dx, 5 + dz, 1.4);
    }
    let (amount, to) = lava.flow_amount(&t, 5, 5, dt);
    assert_eq!(to, Some((4, 5)), "the first edge neighbour wins");
    assert!(
        (f64::from(amount) - 33333.0 * 4.3 * (4e-6 * 4.3 / 12.0)).abs() < 1e-4,
        "{amount}"
    );
    assert!((f64::from(amount) - 0.2054).abs() < 1e-3);
    let mut fire = Fire::default();
    let mode = lava.set_things_on_fire(&t, 4, 5, 1.4, amount, &mut fire);
    assert_eq!(mode, FireMode::Ignite);
    assert_eq!(
        fire.calls,
        [
            "ignite 3 4",
            "cause 3",
            "ignite 4 4",
            "cause 3",
            "ignite 3 5",
            "cause 3",
            "ignite 4 5",
            "cause 3"
        ]
    );
    // Neighbours at 0: amount ≈ 0.2778, nothing ignited.
    for (dx, dz) in NEIGHBOURS {
        lava.set_level(&t, 5 + dx, 5 + dz, 0.0);
    }
    let (amount, _) = lava.flow_amount(&t, 5, 5, dt);
    assert!((f64::from(amount) - 0.2778).abs() < 1e-3, "{amount}");
    let mut fire = Fire::default();
    assert_eq!(
        lava.set_things_on_fire(&t, 4, 5, 0.0, amount, &mut fire),
        FireMode::None
    );
    assert!(fire.calls.is_empty());
    // A vertex on the city edge loses dt × 1e-7 × lava.
    lava.set_level(&t, 0, 5, 10.0);
    let (amount, to) = lava.flow_amount(&t, 0, 5, dt);
    assert_eq!(to, None);
    assert!((f64::from(amount) - 0.033333).abs() < 1e-5);
}

#[test]
fn lava_diagonal_and_burn_modes() {
    let t = flat(10, 300.0);
    let mut lava = Lava::start(&t, 80.0, 80.0, 20.0, LavaTuning::SHIPPED);
    lava.set_level(&t, 5, 5, 10.0);
    for (dx, dz) in NEIGHBOURS {
        lava.set_level(&t, 5 + dx, 5 + dz, 9.0);
    }
    // Edge drop 1; a diagonal needs more than √2.
    lava.set_level(&t, 6, 6, 8.6);
    assert_eq!(lava.lowest_neighbour(&t, 5, 5), Some((4, 5)));
    lava.set_level(&t, 6, 6, 8.5);
    assert_eq!(lava.lowest_neighbour(&t, 5, 5), Some((6, 6)));
    for (dx, dz) in NEIGHBOURS {
        lava.set_level(&t, 5 + dx, 5 + dz, 10.0);
    }
    assert_eq!(lava.lowest_neighbour(&t, 5, 5), None);
    assert_eq!(lava.fire_mode(2.9, 0.2), FireMode::Burn);
    assert_eq!(lava.fire_mode(1.4, 2.0), FireMode::Ignite);
    assert_eq!(lava.fire_mode(3.0, 1.0), FireMode::None);
}

#[test]
fn lava_start_rectangles_and_flow_step() {
    let mut t = flat(10, 300.0);
    let mut lava = Lava::start(&t, 80.0, 1000.0, 20.0, LavaTuning::SHIPPED);
    // z clamped to 159 → vertex 9; the source square 8..10 clamped to 10.
    assert_eq!(lava.source, VertRect::new(4, 8, 6, 10));
    assert_eq!(lava.burn, VertRect::new(0, 2, 9, 9));
    lava.production_rate = 1e-4;
    lava.solidification_rate = 1e-5;
    let mut fire = Fire::default();
    lava.flow(&mut t, 33333.0, &mut fire);
    assert_eq!(lava.active, VertRect::new(3, 7, 7, 10));
    // Every source vertex received 3.3333 and solidified a third of it
    // (solid = 0.33333); the factor 0 raise rewrote the depth and posted.
    let p = t.take_posts();
    assert!(p
        .iter()
        .any(|p| p.msg == msg::TERRAIN_CHANGED && p.data4 == kind::DISASTER));
    assert_eq!(t.altitude_at_vertex(5, 9), 300.0);
    // Burning: 8 draws, an exhausted draw lost.
    let mut fire = Fire {
        burn_cells: vec![Some((1, 2)), None, Some((3, 4))],
        ..Fire::default()
    };
    lava.burn_surrounding(&mut fire);
    assert_eq!(fire.calls, ["ignite 1 2", "ignite 3 4"]);
}

// ---- 14: records --------------------------------------------------------------------------------------

#[test]
fn t14_records_round_trip() {
    let mut t = flat(64, 300.0);
    t.set_altitude(3, 4, 321.5);
    t.set_vertex_flags(5, 6, 0x1808);
    t.set_min_max_cache(1.0, 2.0); // stale on purpose
    let misc = t.write_misc();
    let alt = t.write_altitudes();
    let props = t.write_properties();
    assert_eq!(misc.len(), 26);
    assert_eq!(alt.len(), 2 + 65 * 65 * 4);
    assert_eq!(props.len(), 2 + 65 * 65 * 2);
    for side in 0..4 {
        assert_eq!(t.write_edge(side).unwrap().len(), 2 + 4 + 65 * 4);
    }
    assert_eq!(t.write_edge(4), None);
    let u = Terrain::load(&misc, &alt, &props, TerrainTuning::SHIPPED).unwrap();
    assert_eq!(u.altitudes(), t.altitudes());
    assert_eq!(u.flag_words(), t.flag_words());
    assert_eq!((u.min_height(), u.max_height()), (1.0, 2.0));
    assert_eq!(u.write_misc(), misc);
    assert_eq!(u.terrain_type(), 0);
    assert_eq!(u.saved_78(), 308.0);
    assert!(!u.view_ready());
    // The water is rebuilt from the sea level, not loaded.
    assert_eq!(u.water_depth_at_vertex(3, 4), crate::x87::r24(250.0 - 321.5) as f32);
    let e = Terrain::read_edge(&t.write_edge(2).unwrap()).unwrap();
    assert_eq!(e.len(), 65);
    assert_eq!(e[0], t.altitude_at_vertex(64, 0));
}

#[test]
fn record_versions_and_truncation() {
    let t = flat(2, 300.0);
    let (misc, alt, props) = (t.write_misc(), t.write_altitudes(), t.write_properties());
    assert_eq!(
        Terrain::load(&misc[..20], &alt, &props, TerrainTuning::SHIPPED).unwrap_err(),
        RecordError::Truncated
    );
    assert_eq!(
        Terrain::load(&misc, &alt[..10], &props, TerrainTuning::SHIPPED).unwrap_err(),
        RecordError::Truncated
    );
    let mut m3 = misc.clone();
    m3[0] = 3;
    assert_eq!(
        Terrain::load(&m3, &alt, &props, TerrainTuning::SHIPPED).unwrap_err(),
        RecordError::Version(3)
    );
    // Version 1 with a terrain type ≥ 1 loads type 0; version 2 keeps it.
    let mut m1 = misc.clone();
    m1[0] = 1;
    m1[10] = 5;
    assert_eq!(
        Terrain::load(&m1, &alt, &props, TerrainTuning::SHIPPED)
            .unwrap()
            .terrain_type(),
        0
    );
    let mut m2 = misc.clone();
    m2[10] = 5;
    assert_eq!(
        Terrain::load(&m2, &alt, &props, TerrainTuning::SHIPPED)
            .unwrap()
            .terrain_type(),
        5
    );
    let mut big = misc.clone();
    big[2..6].copy_from_slice(&1u32.to_le_bytes());
    assert_eq!(
        Terrain::load(&big, &alt, &props, TerrainTuning::SHIPPED).unwrap_err(),
        RecordError::Size
    );
}

// ---- construction and tuning ----------------------------------------------------------------------

#[test]
fn create_uses_the_tuning() {
    let t = Terrain::create(64, 64, TerrainTuning::SHIPPED).unwrap();
    assert_eq!((t.verts_x(), t.vertex_count()), (65, 65 * 65));
    assert_eq!((t.size_x(), t.sea_level()), (1024.0, 250.0));
    assert_eq!(t.altitude(512.0, 512.0), 300.0);
    assert_eq!(t.water_depth_at_vertex(0, 0), -50.0);
    assert_eq!((t.min_height(), t.max_height()), (300.0, 300.0));
    assert_eq!((t.min_allowed_height(), t.max_allowed_height()), (20.0, 2500.0));
    assert!(Terrain::create(0, 64, TerrainTuning::SHIPPED).is_none());
    let code = TerrainTuning::from_holder(None);
    assert_eq!((code.max_height, code.min_height), (200.0, 5.0));
}

#[test]
fn tuning_from_the_exemplar() {
    use crate::property::PropertyHolder;
    use crate::variant::{mem, Elem, Variant};
    let mut h = PropertyHolder::new();
    h.add(tuning_prop::MAX_HEIGHT, Variant::Scalar(Elem::F32(2500.0)));
    h.add(tuning_prop::MIN_HEIGHT, Variant::Scalar(Elem::F32(20.0)));
    h.add(tuning_prop::NEW_SEA_LEVEL, Variant::Scalar(Elem::F32(10.0)));
    h.add(
        tuning_prop::SEA_LEVEL_RANGE,
        Variant::array(
            mem::FLOAT32,
            [-5.0, 150.0, 1.0, 2.0].into_iter().map(Elem::F32).collect(),
        ),
    );
    let t = TerrainTuning::from_holder(Some(&h));
    assert_eq!((t.max_height, t.min_height, t.new_sea_level), (2500.0, 20.0, 20.0));
    assert_eq!(
        (t.sea_level_min, t.sea_level_max, t.sea_level_editor),
        (-5.0, 150.0, [1.0, 2.0])
    );
}

// ---- §10: navigability and bodies of water ---------------------------------------------------------

/// A host with a lot, a bridge and a pole at given cells.
struct Nav {
    lot: ((i32, i32), u64),
    bridge: ((i32, i32), Bridge),
    pole: (i32, i32),
}

impl NavigationHost for Nav {
    fn lot_at_cell(&self, x: i32, z: i32) -> u64 {
        if (x, z) == self.lot.0 {
            self.lot.1
        } else {
            0
        }
    }
    fn bridge_at(&self, x: i32, z: i32) -> Bridge {
        if (x, z) == self.bridge.0 {
            self.bridge.1
        } else {
            Bridge::None
        }
    }
    fn has_power_pole(&self, x: i32, z: i32) -> bool {
        (x, z) == self.pole
    }
}

/// A 10 × 10 city at 200 m under a sea at 250 (50 m deep everywhere).
fn sea() -> Terrain {
    flat(10, 200.0)
}

#[test]
fn is_navigable_rules() {
    let t = sea();
    let none = Nav {
        lot: ((-9, -9), 0),
        bridge: ((-9, -9), Bridge::None),
        pole: (-9, -9),
    };
    assert!(t.is_navigable(5, 5, 2, 0, &none));
    // The square leaves the city.
    assert!(!t.is_navigable(1, 5, 2, 0, &none));
    // A lot other than ours.
    let lot = Nav {
        lot: ((6, 6), 77),
        ..none
    };
    assert!(!t.is_navigable(5, 5, 1, 1, &lot));
    assert!(t.is_navigable(5, 5, 1, 77, &lot));
    assert!(t.is_navigable(5, 5, 1, 0, &lot));
    // A power pole.
    let pole = Nav { pole: (4, 4), ..none };
    assert!(!t.is_navigable(5, 5, 1, 0, &pole));
    // Bridges: deck 270 over a surface at 250 clears by exactly 20.
    let high = Nav {
        bridge: ((5, 5), Bridge::Deck(270.0)),
        ..none
    };
    assert!(t.is_navigable(5, 5, 0, 0, &high));
    let low = Nav {
        bridge: ((5, 5), Bridge::Deck(269.99)),
        ..none
    };
    assert!(!t.is_navigable(5, 5, 0, 0, &low));
    let unknown = Nav {
        bridge: ((5, 5), Bridge::UnknownDeck),
        ..none
    };
    assert!(!t.is_navigable(5, 5, 0, 0, &unknown));
    // [quirk 8] The high bridge at the centre accepts the square at once,
    // even with a pole further out and the square leaving the city.
    let quirk = Nav {
        bridge: ((1, 5), Bridge::Deck(400.0)),
        pole: (2, 5),
        ..none
    };
    assert!(t.is_navigable(1, 5, 2, 0, &quirk));
    // A negative r: the box is the single cell (x − r, z − r).
    let shallow = {
        let mut t = sea();
        t.set_water_depth(7, 7, 1.0);
        t
    };
    assert!(!shallow.is_navigable(5, 5, -1, 0, &none) || shallow.has_draft_clearance(6, 6));
    assert!(shallow.is_navigable(7, 7, -1, 0, &none));
    assert!(!shallow.is_navigable(6, 6, -1, 0, &none));
}

#[test]
fn bodies_of_water_flood_fill() {
    // Land at 300 everywhere, two separate pools at 200 (under the sea):
    // cells x 1..=2, z 1..=3 and x 6..=7, z 6..=7.
    let mut t = flat(10, 300.0);
    for (x0, z0, x1, z1) in [(1, 1, 3, 4), (6, 6, 8, 8)] {
        for z in z0..=z1 {
            for x in x0..=x1 {
                t.set_altitude(x, z, 200.0);
            }
        }
    }
    let mut b = BodiesOfWater::new(10, 10);
    b.update(&t, &OpenWater);
    assert_eq!(b.get(1, 1), 1);
    assert_eq!(b.get(2, 3), 1);
    assert_eq!(b.get(6, 6), 2);
    assert_eq!(b.get(7, 7), 2);
    assert_eq!(b.get(0, 0), 0);
    assert_eq!(b.get(3, 1), 0);
    assert_eq!(b.map().iter().filter(|&&v| v != 0).count(), 6 + 4);
    // A pole on a water cell marks the map dirty; SimNewDay rebuilds it.
    b.note_power_pole_changed(&t, 0, 0);
    assert!(!b.dirty());
    b.note_power_pole_changed(&t, 6, 6);
    assert!(b.dirty());
    let pole = Nav {
        lot: ((-9, -9), 0),
        bridge: ((-9, -9), Bridge::None),
        pole: (6, 6),
    };
    assert!(b.on_new_day(&t, &pole));
    assert!(!b.dirty());
    assert_eq!(b.get(6, 6), 0);
    assert_eq!(b.get(7, 6), 2);
    assert!(!b.on_new_day(&t, &pole));
}

#[test]
fn bodies_scan_x_outer() {
    // Two pools that the scan order tells apart: (5, 0) and (0, 5). With x
    // outer, z inner, (0, 5) is found first.
    let mut t = flat(8, 300.0);
    for (x, z) in [(5, 0), (0, 5)] {
        for (dx, dz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            t.set_altitude(x + dx, z + dz, 200.0);
        }
    }
    let mut b = BodiesOfWater::new(8, 8);
    b.update(&t, &OpenWater);
    assert_eq!((b.get(0, 5), b.get(5, 0)), (1, 2));
}
