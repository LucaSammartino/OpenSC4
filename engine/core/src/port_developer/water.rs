// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The seaport's sea-access map (`docs/specs/port-developers.md` §4.8).

/// A water cell.
pub const WATER: u8 = 0x80;
/// Edge bits: row z = 0, column x = 0, column x = width − 1, row
/// z = height − 1.
pub const EDGE_NORTH: u8 = 1;
pub const EDGE_WEST: u8 = 2;
pub const EDGE_EAST: u8 = 4;
pub const EDGE_SOUTH: u8 = 8;
/// The fill's segment stack (§4.8): a push beyond it is dropped.
pub const STACK_LIMIT: usize = 10_000;

/// The sea-access map: one byte per cell, row-major.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WaterMap {
    pub width: u32,
    pub height: u32,
    pub cells: Vec<u8>,
}

impl WaterMap {
    /// Builds the map (`BuildWaterAccessMap`): `0x80` for water, then the
    /// four edge scans. Returns the map and `cityHasSea`.
    pub fn build(width: u32, height: u32, has_water: impl Fn(u32, u32) -> bool) -> (WaterMap, bool) {
        let mut m = WaterMap {
            width,
            height,
            cells: vec![0; (width as usize) * (height as usize)],
        };
        for z in 0..height {
            for x in 0..width {
                if has_water(x, z) {
                    m.cells[(z * width + x) as usize] = WATER;
                }
            }
        }
        let mut sea = false;
        if width == 0 || height == 0 {
            return (m, sea);
        }
        let row = |z: u32| (0..width).map(move |x| (x, z)).collect::<Vec<_>>();
        let col = |x: u32| (0..height).map(move |z| (x, z)).collect::<Vec<_>>();
        let edges = [
            (row(0), EDGE_NORTH),
            (row(height - 1), EDGE_SOUTH),
            (col(0), EDGE_WEST),
            (col(width - 1), EDGE_EAST),
        ];
        for (cells, bit) in edges {
            let mut in_run = false;
            for (x, z) in cells {
                let water = m.get(x as i32, z as i32) & WATER != 0;
                if water && !in_run {
                    m.flood_fill(x as i32, z as i32, bit);
                    sea = true;
                }
                in_run = water;
            }
        }
        (m, sea)
    }

    /// The byte at (x, z); 0 outside the map [choice] (the original has no
    /// bounds check).
    pub fn get(&self, x: i32, z: i32) -> u8 {
        if x < 0 || z < 0 || x as u32 >= self.width || z as u32 >= self.height {
            return 0;
        }
        self.cells[(z as u32 * self.width + x as u32) as usize]
    }

    fn fillable(&self, x: i32, z: i32, bit: u8) -> bool {
        let v = self.get(x, z);
        v & WATER != 0 && v & bit == 0
    }

    /// A 4-connected scanline seed fill over water cells without `bit`,
    /// ORing it in. The seed stack holds [`STACK_LIMIT`] entries [inferred].
    pub fn flood_fill(&mut self, x: i32, z: i32, bit: u8) {
        let mut stack: Vec<(i32, i32)> = vec![(x, z)];
        let w = self.width;
        while let Some((sx, sz)) = stack.pop() {
            if !self.fillable(sx, sz, bit) {
                continue;
            }
            let mut x0 = sx;
            while self.fillable(x0 - 1, sz, bit) {
                x0 -= 1;
            }
            let mut x1 = sx;
            while self.fillable(x1 + 1, sz, bit) {
                x1 += 1;
            }
            for cx in x0..=x1 {
                self.cells[(sz as u32 * w + cx as u32) as usize] |= bit;
            }
            for nz in [sz - 1, sz + 1] {
                let mut prev = false;
                for cx in x0..=x1 {
                    let f = self.fillable(cx, nz, bit);
                    if f && !prev && stack.len() < STACK_LIMIT {
                        stack.push((cx, nz));
                    }
                    prev = f;
                }
            }
        }
    }

    /// A rectangle's access (§4.8): any cell with a non-zero low nibble.
    pub fn rect_has_access(&self, x0: i32, z0: i32, x1: i32, z1: i32) -> bool {
        (z0..=z1).any(|z| (x0..=x1).any(|x| self.get(x, z) & 0x0F != 0))
    }
}
