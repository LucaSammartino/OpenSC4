// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

use super::super::{flag, Terrain};

/// Six u16 words for one cell, relative to its bound pair of vertex rows.
/// terrain.md §15.5.13.4: either split bit selects the alternate diagonal;
/// winding precedes the two independent hide overrides. z affects storage
/// only, never these values. The native stores retain the low sixteen bits.
pub fn cell_triangle_indices(verts_x: u32, x: u32, flags: u16) -> [u16; 6] {
    let a = x as u16;
    let b = a.wrapping_add(1);
    let c = a.wrapping_add(verts_x as u16);
    let d = c.wrapping_add(1);
    let mut indices = match (flags & flag::FLIPPED_MASK != 0, flags & flag::CELL_FLAG_4 != 0) {
        (false, false) => [a, c, d, a, d, b],
        (false, true) => [d, c, a, b, d, a],
        (true, false) => [a, c, b, b, c, d],
        (true, true) => [b, c, a, d, c, b],
    };
    if flags & flag::EVEN_TRIANGLE_HIDDEN != 0 {
        indices[..3].fill(a);
    }
    if flags & flag::ODD_TRIANGLE_HIDDEN != 0 {
        indices[3..].fill(a);
    }
    indices
}

/// The shared land/water index grid J. Init rebuilds all cells; later native
/// per-cell notifications must synchronously call rewrite_cell. This storage
/// does not intercept those notifications (the host boundary is §15.5.14).
#[derive(Clone, Debug)]
pub struct TerrainViewIndices {
    cells_x: u32,
    cells_z: u32,
    words: Vec<u16>,
}

impl TerrainViewIndices {
    pub fn new(source: &Terrain) -> Self {
        let mut result = Self {
            cells_x: source.cells_x(),
            cells_z: source.cells_z(),
            words: vec![0; 6 * source.cells_x() as usize * source.cells_z() as usize],
        };
        for z in 0..result.cells_z {
            for x in 0..result.cells_x {
                result.rewrite_cell(x, z, source.vertex_flags(x as i32, z as i32));
            }
        }
        result
    }

    /// Stride S in u16 words, not bytes or vertices.
    pub fn row_stride(&self) -> u32 {
        6 * self.cells_x
    }

    pub fn words(&self) -> &[u16] {
        &self.words
    }

    /// Writes exactly six words at z*S+6*x, even if the flags are unchanged.
    /// This portable API takes (x,z); the native host entry takes (z,x).
    pub fn rewrite_cell(&mut self, x: u32, z: u32, flags: u16) -> bool {
        // SPEC-GAP: out-of-grid notifications are rejected without modifying storage; the native helper is unchecked.
        if x >= self.cells_x || z >= self.cells_z {
            return false;
        }
        let first = (z * self.row_stride() + 6 * x) as usize;
        self.words[first..first + 6].copy_from_slice(&cell_triangle_indices(self.cells_x + 1, x, flags));
        true
    }
}
