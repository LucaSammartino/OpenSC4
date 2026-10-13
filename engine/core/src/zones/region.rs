// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `SC4CellRegion<long>` (§11.4): an inclusive rectangle plus one bit per
//! cell, and the region helpers of the zone developer (`CompressRegion`,
//! `ExtractConnectedCells`, §7.2).

use super::CellRect;

/// [choice] The largest side a region stores bits for. A rectangle wider or
/// deeper than this keeps its coordinates, but the cells beyond the first
/// `MAX_REGION_SIDE` columns / rows are never included (hostile rectangles
/// cannot make the core allocate more than 16 M cells).
pub const MAX_REGION_SIDE: i64 = 4096;

/// A cell region: `rect` (inclusive) and a bit per cell of the rectangle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellRegion {
    pub rect: CellRect,
    width: i32,
    height: i32,
    /// Column-major like `cRZCellMap<bool>`: bit `(dx, dz)` at
    /// `dx · height + dz`.
    bits: Vec<bool>,
}

fn side(lo: i32, hi: i32) -> i32 {
    (i64::from(hi) - i64::from(lo) + 1).clamp(0, MAX_REGION_SIDE) as i32
}

impl CellRegion {
    /// A region over `rect` with every bit set to `fill` (constructor
    /// `0x00442640(rect, fill)`).
    pub fn new(rect: CellRect, fill: bool) -> CellRegion {
        let (width, height) = (side(rect.x1, rect.x2), side(rect.z1, rect.z2));
        CellRegion {
            rect,
            width,
            height,
            bits: vec![fill; width as usize * height as usize],
        }
    }

    /// [choice] The region with no cells (`{0, 0, −1, −1}`), used where the
    /// original makes a region from an empty set (§7.2 step 5).
    pub fn empty() -> CellRegion {
        CellRegion::new(CellRect::new(0, 0, -1, -1), false)
    }

    /// A region over `rect` with the listed cells set (cells outside the
    /// rectangle are ignored).
    pub fn from_cells(rect: CellRect, cells: &[(i32, i32)]) -> CellRegion {
        let mut r = CellRegion::new(rect, false);
        for &(x, z) in cells {
            r.set(x, z, true);
        }
        r
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    fn index(&self, x: i32, z: i32) -> Option<usize> {
        // `IncludesCell` (0x00466610): unsigned offset test.
        let dx = (i64::from(x) - i64::from(self.rect.x1)) as u64;
        let dz = (i64::from(z) - i64::from(self.rect.z1)) as u64;
        if dx < self.width as u64 && dz < self.height as u64 {
            Some(dx as usize * self.height as usize + dz as usize)
        } else {
            None
        }
    }

    /// Whether `(x, z)` lies in the rectangle and its bit is set.
    pub fn includes(&self, x: i32, z: i32) -> bool {
        self.index(x, z)
            .and_then(|i| self.bits.get(i))
            .copied()
            .unwrap_or(false)
    }

    /// Sets or clears one bit (ignored outside the rectangle).
    pub fn set(&mut self, x: i32, z: i32, v: bool) {
        if let Some(i) = self.index(x, z) {
            if let Some(b) = self.bits.get_mut(i) {
                *b = v;
            }
        }
    }

    /// The set cells, x outer, z inner.
    pub fn cells(&self) -> Vec<(i32, i32)> {
        let mut out = Vec::new();
        for dx in 0..self.width {
            for dz in 0..self.height {
                if self.bits[dx as usize * self.height as usize + dz as usize] {
                    out.push((self.rect.x1.wrapping_add(dx), self.rect.z1.wrapping_add(dz)));
                }
            }
        }
        out
    }

    /// The number of set cells.
    pub fn count(&self) -> usize {
        self.bits.iter().filter(|&&b| b).count()
    }

    pub fn is_empty(&self) -> bool {
        !self.bits.iter().any(|&b| b)
    }

    /// The bounding box of the set cells, `None` when none is set.
    pub fn bounding_box(&self) -> Option<CellRect> {
        let cells = self.cells();
        let first = cells.first()?;
        let mut r = CellRect::new(first.0, first.1, first.0, first.1);
        for &(x, z) in &cells {
            r.x1 = r.x1.min(x);
            r.z1 = r.z1.min(z);
            r.x2 = r.x2.max(x);
            r.z2 = r.z2.max(z);
        }
        Some(r)
    }

    /// `CompressRegion` (`0x0072AF30`): a new region cropped to the bounding
    /// box of the set cells. [choice] No set cell gives [`CellRegion::empty`].
    pub fn compress(&self) -> CellRegion {
        match self.bounding_box() {
            Some(bb) => CellRegion::from_cells(bb, &self.cells()),
            None => CellRegion::empty(),
        }
    }

    /// `ExtractConnectedCells` (`0x0072CDA0`) applied until no cell is left:
    /// the 4-connected components, each cropped. [choice] Components are
    /// seeded in x-outer, z-inner order of their first cell (the region's
    /// storage order) and flood-filled; no component gives no entry (the
    /// caller adds the empty region).
    pub fn components(&self) -> Vec<CellRegion> {
        let mut left = self.clone();
        let mut out = Vec::new();
        for (sx, sz) in self.cells() {
            if !left.includes(sx, sz) {
                continue;
            }
            let mut comp = Vec::new();
            let mut stack = vec![(sx, sz)];
            left.set(sx, sz, false);
            while let Some((x, z)) = stack.pop() {
                comp.push((x, z));
                for (nx, nz) in [
                    (x.wrapping_sub(1), z),
                    (x.wrapping_add(1), z),
                    (x, z.wrapping_sub(1)),
                    (x, z.wrapping_add(1)),
                ] {
                    if left.includes(nx, nz) {
                        left.set(nx, nz, false);
                        stack.push((nx, nz));
                    }
                }
            }
            let mut r = CellRect::new(sx, sz, sx, sz);
            for &(x, z) in &comp {
                r.x1 = r.x1.min(x);
                r.z1 = r.z1.min(z);
                r.x2 = r.x2.max(x);
                r.z2 = r.z2.max(z);
            }
            out.push(CellRegion::from_cells(r, &comp));
        }
        out
    }
}
