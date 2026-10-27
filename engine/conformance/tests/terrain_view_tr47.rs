// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

//! terrain.md §15.5.14 and §15.5.15.5–7's isolated worked vectors.
//! No vector here is a captured native frame or an installed host bridge.

use fc4_core::map_view::{RingContext, RingPrecision, RingRounding};
use fc4_core::terrain::{
    coordinate_bounds, edge_triangle_indices, plumbing_calls, row_visibility, widen_row_intervals, ExtensionBuffer,
    ExtensionFlushCall as Flush, PlumbingCall as Plumb, Terrain, TerrainTuning, TerrainViewLifecycle,
    TerrainViewShared, TextureCoordinateCache, ViewColumnInterval as Interval, ViewVertex,
};

fn pc64() -> RingContext {
    RingContext {
        precision: RingPrecision::Bits64,
        rounding: RingRounding::Nearest,
    }
}

fn source() -> Terrain {
    Terrain::from_altitudes(4, 3, &[300.0; 20], 250.0, TerrainTuning::SHIPPED).unwrap()
}

#[test]
fn edge_words_are_fixed_and_distinct_from_hidden_terrain_words() {
    let mut terrain = source();
    terrain.set_vertex_flags(2, 1, 0x1220);
    let words = edge_triangle_indices(5);
    assert_eq!(words.len(), 24);
    let bytes: Vec<_> = words[6..18].iter().flat_map(|i| i.to_le_bytes()).collect();
    assert_eq!(
        bytes,
        [1, 0, 6, 0, 7, 0, 1, 0, 7, 0, 2, 0, 2, 0, 7, 0, 8, 0, 2, 0, 8, 0, 3, 0]
    );
    let mut view = TerrainViewLifecycle::default();
    assert!(view.init(&terrain, 0, &mut TerrainViewShared::default()));
    assert_eq!(&view.indices().unwrap().words()[36..42], &[2, 2, 2, 8, 7, 3]);
    assert_eq!(&words[12..18], &[2, 7, 8, 2, 8, 3]);
    assert_eq!(edge_triangle_indices(0), []);
    assert_eq!(edge_triangle_indices(1), []);
    // Storage is u16, even when W itself is not representable there.
    assert_eq!(&edge_triangle_indices(65_537)[..6], &[0, 1, 2, 0, 2, 1]);
}

#[test]
fn uv_vectors_use_each_buffers_positions_and_preserve_other_bytes() {
    let uv_bits = [
        [0x3F800000u32, 0x40000000],
        [0x40000000, 0x40400000],
        [0x40400000, 0x3F800000],
        [0x3F800000, 0x3F800000],
    ];
    for (rotation, expected) in uv_bits.into_iter().enumerate() {
        let mut cache = TextureCoordinateCache::default();
        let mut land = [ViewVertex::new(16.0, 10.0, 32.0)];
        let mut water = [ViewVertex::new(16.0, 20.0, 32.0)];
        let before_land = land[0].clone();
        let before_water = water[0].clone();
        assert!(cache.update(
            pc64(),
            2,
            rotation as u32,
            [0.25, 0.25],
            [64.0, 48.0],
            &mut land,
            &mut water
        ));
        for (vertex, before) in [(&land[0], &before_land), (&water[0], &before_water)] {
            assert_eq!(
                &vertex.0[16..24],
                &expected.into_iter().flat_map(u32::to_le_bytes).collect::<Vec<_>>()
            );
            assert_eq!(&vertex.0[..16], &before.0[..16]);
            assert_eq!(&vertex.0[24..], &before.0[24..]);
        }
    }
    let mut cache = TextureCoordinateCache::default();
    let mut land = [ViewVertex::new(16.0, 10.0, 32.0)];
    let mut water = [ViewVertex::new(32.0, 20.0, 16.0)];
    assert!(cache.update(pc64(), 0, 0, [0.25, 0.25], [64.0, 48.0], &mut land, &mut water));
    assert_eq!(&land[0].0[16..24], &[0, 0, 128, 63, 0, 0, 0, 64]); // (1,2)
    assert_eq!(&water[0].0[16..24], &[0, 0, 0, 64, 0, 0, 128, 63]); // (2,1)
    let previous = land.clone();
    // A tuning-only change must not refresh UVs or the stored factor.
    assert!(!cache.update(pc64(), 0, 0, [4.0, 4.0], [64.0, 48.0], &mut land, &mut water));
    assert_eq!(land, previous);
    assert_eq!(cache.factor(), 1.0 / 16.0);
    // An invalid rotation refreshes keys/factor but writes no vertex bytes.
    assert!(cache.update(pc64(), 0, 9, [4.0, 4.0], [64.0, 48.0], &mut land, &mut water));
    assert_eq!(cache.factor(), 16.0);
    assert_eq!(land, previous);
}

#[test]
fn floor_textured_and_following_floor_draw_bytes_match_the_worked_record() {
    let mut cache = TextureCoordinateCache::default();
    let mut vertex = [ViewVertex::new(16.0, 10.0, 32.0)];
    cache.update(pc64(), 0, 1, [0.25, 0.25], [64.0, 48.0], &mut vertex, &mut []);
    vertex[0].set_rgb([0x11, 0x22, 0x33]);
    assert_eq!(
        vertex[0].0,
        [
            0, 0, 128, 65, 0, 0, 32, 65, 0, 0, 0, 66, 0x33, 0x22, 0x11, 0xFF, 0, 0, 0, 64, 0, 0, 64, 64, 0, 0, 0, 0, 0,
            0, 0, 0,
        ]
    );
    let floor = vertex[0].clone();
    vertex[0].set_rgb([0x44, 0x55, 0x66]);
    vertex[0].set_layer_alpha(7, [(3, 0xFFFF), (7, 0x01AB), (0, 0)]);
    assert_eq!(&vertex[0].0[12..16], &[0x66, 0x55, 0x44, 0xAB]);
    assert_eq!(&vertex[0].0[..12], &floor.0[..12]);
    assert_eq!(&vertex[0].0[16..], &floor.0[16..]);
    vertex[0].set_rgb([0x11, 0x22, 0x33]);
    assert_eq!(&vertex[0].0[12..16], &[0x33, 0x22, 0x11, 0xAB]);
    vertex[0].set_layer_alpha(7, [(3, 0), (0, 0), (7, 0xFF)]);
    assert_eq!(vertex[0].0[15], 0); // A zero ID stops the scan.
}

#[test]
fn qualified_boundary_vectors_keep_live_precision_until_the_destination_store() {
    for (bits, bounds) in [(0x417FFFFF, (0, 2)), (0x41800000, (1, 2)), (0x41800001, (1, 3))] {
        // No rotation branch exists in this conversion.
        for _rotation in 0..4 {
            assert_eq!(coordinate_bounds(f32::from_bits(bits), pc64()), Some(bounds));
        }
    }
    let pc24 = RingContext::default();
    assert_eq!(coordinate_bounds(f32::from_bits(0x417FFFFF), pc24), Some((0, 3)));
    assert_eq!(coordinate_bounds(f32::NAN, pc64()), None);
    assert_eq!(Interval::clamped(-2, -1, 5), Interval { left: 0, right: 5 });
    let mut intervals = [
        Interval { left: 2, right: 3 },
        Interval { left: 1, right: 4 },
        Interval { left: 0, right: 5 },
    ];
    widen_row_intervals(&mut intervals);
    assert_eq!(
        intervals,
        [
            Interval { left: 1, right: 4 },
            Interval { left: 0, right: 5 },
            Interval { left: 0, right: 5 }
        ]
    );
}

#[test]
fn water_equality_is_rejected_independently_of_land_visibility() {
    for (a, b, q, expected) in [
        (8.0, 8.0, 8.0, [true, false]),
        (4.0, 8.0, 8.0, [true, true]),
        (4.0, 7.0, 8.0, [false, true]),
        (4.0, 8.0, 7.0, [true, false]),
    ] {
        assert_eq!(row_visibility(true, 8.0, a, b, q), expected);
        assert_eq!(row_visibility(false, 8.0, a, b, q), [false, false]);
    }
    assert_eq!(
        row_visibility(true, f32::from_bits(0x41000001), 8.0, 8.0, 8.0),
        [false, false]
    );
}

#[test]
fn extension_flush_unlocks_even_when_empty_and_only_draws_whole_triangles() {
    let mut buffer = ExtensionBuffer::default();
    assert_eq!(buffer.flush(9), []);
    buffer.mapped(true);
    assert_eq!(buffer.flush(9), [Flush::Unlock { handle: 9 }]);
    assert_eq!(buffer.flush(9), []);
    buffer.mapped(true);
    let indices = vec![1, 6, 7, 1, 7, 2, 2, 7, 8, 2, 8, 3];
    assert!(buffer.append(10, &indices));
    assert_eq!(buffer.pending_counts(), (10, 12));
    assert_eq!(
        buffer.flush(9),
        [
            Flush::Unlock { handle: 9 },
            Flush::Draw {
                handle: 9,
                first: 0,
                count: 12,
                indices
            }
        ]
    );
    assert_eq!(buffer.pending_counts(), (0, 0));
    buffer.mapped(true);
    buffer.append(1, &[0]);
    assert_eq!(
        buffer.flush(9),
        [
            Flush::Unlock { handle: 9 },
            Flush::Draw {
                handle: 9,
                first: 0,
                count: 0,
                indices: vec![0]
            }
        ]
    );
    buffer.mapped(false);
    assert!(!buffer.append(10, &[0, 1, 2]));
    assert_eq!(buffer.flush(9), []); // A mapping failure does not select the plain path.
}

#[test]
fn empty_plumbing_stream_restores_offset_blend_and_depth_mask_but_not_texturing() {
    let (prefix, suffix) = plumbing_calls(7.0);
    assert_eq!(
        prefix,
        [
            Plumb::EnableBlend(true),
            Plumb::SetBlendFunc(4, 5),
            Plumb::EnableDepthMask(false),
            Plumb::EnableTexture(false, 0),
            Plumb::SetDepthOffset(-2.0)
        ]
    );
    assert_eq!(
        suffix,
        [
            Plumb::SetDepthOffset(7.0),
            Plumb::EnableBlend(false),
            Plumb::EnableDepthMask(true)
        ]
    );
}

#[test]
fn read_before_init_retains_the_process_grid_byte_and_rejects_duplicate_init() {
    let source = source();
    let mut shared = TerrainViewShared { display_grid: 0 };
    let mut loaded = TerrainViewLifecycle::default();
    assert!(loaded.read(&[1, 0, 1], &mut shared));
    assert!(loaded.indices().is_none());
    assert!(loaded.heights().is_none());
    assert!(loaded.init(&source, 0, &mut shared));
    assert_eq!(shared.display_grid, 1);
    assert_eq!(loaded.write(&shared), [1, 0, 1]);
    assert!(!loaded.init(&source, 0, &mut shared));
    assert_eq!(shared.display_grid, 1);
    // Both views serialize the same process state, not a per-instance copy.
    let mut fresh = TerrainViewLifecycle::default();
    assert!(fresh.init(&source, 0, &mut shared));
    assert_eq!(loaded.write(&shared), [1, 0, 0]);
    let mut reverse = TerrainViewLifecycle::default();
    assert!(reverse.read(&fresh.write(&shared), &mut shared));
    assert!(reverse.init(&source, 1, &mut shared));
    assert_eq!(shared.display_grid, 0);
}

#[test]
fn invalid_read_sets_the_restored_flag_without_sizing_city_caches_or_changing_grid() {
    for payload in [&[][..], &[1, 0], &[2, 0, 0]] {
        let mut shared = TerrainViewShared { display_grid: 1 };
        let mut view = TerrainViewLifecycle::default();
        assert!(!view.read(payload, &mut shared));
        assert_eq!(shared.display_grid, 1);
        assert!(view.indices().is_none());
        assert!(view.init(&source(), 0, &mut shared));
        assert_eq!(shared.display_grid, 1);
    }
}

#[test]
fn direct_cell_entry_order_altitude_refresh_and_teardown_are_distinct() {
    let mut source = source();
    let mut view = TerrainViewLifecycle::default();
    let mut shared = TerrainViewShared::default();
    assert!(!view.set_triangle_indices_for_cell(1, 2, 0x1220));
    assert!(view.init(&source, 0, &mut shared));
    assert!(view.set_triangle_indices_for_cell(1, 2, 0x1220));
    assert_eq!(&view.indices().unwrap().words()[36..42], &[2, 2, 2, 8, 7, 3]);
    let before = view.altitude_bounds();
    view.update_altitude_bounds(&source, false);
    assert_eq!(view.altitude_bounds(), before);
    source.set_raw_altitude(0, 0, 320.0);
    source.recompute_min_max();
    view.update_altitude_bounds(&source, true);
    assert_eq!(view.altitude_bounds(), [320.0, 300.0]);
    assert!(view.shutdown());
    assert!(view.indices().is_none());
    assert!(view.heights().is_none());
    assert!(!view.shutdown());
    assert!(!view.set_triangle_indices_for_cell(1, 2, 0));
}
