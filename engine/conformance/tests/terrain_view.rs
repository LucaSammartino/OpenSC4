// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

//! terrain.md §15.5.13.4 and §15.5.15.4 worked vectors. These compare only
//! verified index words and row subsequences, not complete rendered frames.

use fc4_core::terrain::{cell_triangle_indices, PlainViewRow, Terrain, TerrainTuning, TerrainViewIndices};
use fc4_core::terrain::{ViewRowCall as Call, ViewVertexBuffer as Buffer};

#[test]
fn all_split_winding_and_hide_vectors() {
    // W=5, x=2: independently listed words for all sixteen cases.
    let cases = [
        (0x0000, [2, 7, 8, 2, 8, 3]),
        (0x0020, [2, 2, 2, 2, 8, 3]),
        (0x0040, [2, 7, 8, 2, 2, 2]),
        (0x0060, [2, 2, 2, 2, 2, 2]),
        (0x0200, [8, 7, 2, 3, 8, 2]),
        (0x0220, [2, 2, 2, 3, 8, 2]),
        (0x0240, [8, 7, 2, 2, 2, 2]),
        (0x0260, [2, 2, 2, 2, 2, 2]),
        (0x1000, [2, 7, 3, 3, 7, 8]),
        (0x1020, [2, 2, 2, 3, 7, 8]),
        (0x1040, [2, 7, 3, 2, 2, 2]),
        (0x1060, [2, 2, 2, 2, 2, 2]),
        (0x1200, [3, 7, 2, 8, 7, 3]),
        (0x1220, [2, 2, 2, 8, 7, 3]),
        (0x1240, [3, 7, 2, 2, 2, 2]),
        (0x1260, [2, 2, 2, 2, 2, 2]),
    ];
    for (flags, expected) in cases {
        assert_eq!(cell_triangle_indices(5, 2, flags), expected, "flags {flags:#06x}");
        // All other flag bits are irrelevant to this helper.
        assert_eq!(cell_triangle_indices(5, 2, flags | 0xCD9F), expected);
        if flags & 0x1000 != 0 {
            for split in [0x2000, 0x3000] {
                assert_eq!(cell_triangle_indices(5, 2, (flags & !0x1000) | split), expected);
            }
        }
    }
}

fn terrain() -> Terrain {
    Terrain::from_altitudes(4, 3, &[300.0; 20], 250.0, TerrainTuning::SHIPPED).unwrap()
}

#[test]
fn init_and_cell_rewrite_use_row_relative_values_at_row_specific_offsets() {
    let mut source = terrain();
    source.set_vertex_flags(2, 1, 0x1220);
    // These last-row/column vertices are not cells and produce no indices.
    source.set_vertex_flags(4, 0, 0x0060);
    source.set_vertex_flags(0, 3, 0x0060);
    let mut indices = TerrainViewIndices::new(&source);
    assert_eq!(indices.row_stride(), 24);
    assert_eq!(indices.words().len(), 72);
    assert_eq!(&indices.words()[36..42], &[2, 2, 2, 8, 7, 3]); // byte offset 72
    assert_eq!(&indices.words()[12..18], &[2, 7, 8, 2, 8, 3]);
    assert_eq!(&indices.words()[60..66], &[2, 7, 8, 2, 8, 3]);
    let before = indices.words().to_vec();
    assert!(indices.rewrite_cell(2, 1, 0x1260));
    assert_eq!(&indices.words()[36..42], &[2; 6]);
    assert_eq!(&indices.words()[..36], &before[..36]);
    assert_eq!(&indices.words()[42..], &before[42..]);
    let before = indices.words().to_vec();
    assert!(!indices.rewrite_cell(4, 1, 0));
    assert!(!indices.rewrite_cell(0, 3, 0));
    assert_eq!(indices.words(), before);
}

fn row() -> PlainViewRow {
    PlainViewRow {
        row: 1,
        left: 1,
        right: 4,
        bounding_rows_visible: [true, false],
    }
}

#[test]
fn worked_floor_and_water_subsequences_preserve_order_counts_and_offsets() {
    let land = Call::SetVertexBuffer {
        buffer: Buffer::Land,
        format: 11,
        count: 10,
        byte_offset: 160,
    };
    let water = Call::SetVertexBuffer {
        buffer: Buffer::Water,
        format: 11,
        count: 5,
        byte_offset: 160,
    };
    let index = Call::SetIndexBuffer {
        count: 24,
        byte_offset: 48,
    };
    let draw = Call::DrawPrimsIndexed {
        primitive: 0,
        first: 6,
        count: 12,
    };
    assert_eq!(row().floor_calls(5, 24), Some([land, index, draw]));
    assert_eq!(row().water_calls(5, 24), Some([index, water, draw]));
    let mut source = terrain();
    source.set_vertex_flags(2, 1, 0x1220);
    let indices = TerrainViewIndices::new(&source);
    // The draw starts six words into row 1, not at city-wide vertex 6.
    assert_eq!(&indices.words()[30..42], &[1, 6, 7, 1, 7, 2, 2, 2, 2, 8, 7, 3]);
}

#[test]
fn visibility_accepts_either_bounding_row_and_floor_retains_zero_count_draw() {
    for visible in [[true, false], [false, true], [true, true]] {
        let r = PlainViewRow {
            bounding_rows_visible: visible,
            ..row()
        };
        assert!(r.floor_calls(5, 24).is_some());
        assert!(r.water_calls(5, 24).is_some());
    }
    let invisible = PlainViewRow {
        bounding_rows_visible: [false, false],
        ..row()
    };
    assert_eq!(invisible.floor_calls(5, 24), None);
    assert_eq!(invisible.water_calls(5, 24), None);
    let empty = PlainViewRow { right: 2, ..row() };
    assert_eq!(
        empty.floor_calls(5, 24).unwrap()[2],
        Call::DrawPrimsIndexed {
            primitive: 0,
            first: 6,
            count: 0
        }
    );
    assert_eq!(empty.water_calls(5, 24), None);
}
