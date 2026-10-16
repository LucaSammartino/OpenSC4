// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Coordinate systems and conversions (§3), the occupant box (§5.1 steps
//! 5–6), the overlap test (§8.2) and `GetBoundingCells` (§6.2).
//!
//! Every operation is single precision (`f32`), as at the game's 24-bit x87
//! precision (foundations §18): each step rounds to `float`.

use crate::x87::rhe;

use super::{CITY_ORIGIN_CELL, WORLD_CELLS};

/// A position or box corner: x, y (height), z.
pub type Vec3 = [f32; 3];

/// An occupant's `GetBoundingBox` (occupant slot 10).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BBox {
    pub min: Vec3,
    pub max: Vec3,
}

/// An occupant's manager box (§4.3): world cells, inclusive, one byte each.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CellBox {
    pub x0: u8,
    pub z0: u8,
    pub x1: u8,
    pub z1: u8,
}

impl CellBox {
    pub fn new(x0: u8, z0: u8, x1: u8, z1: u8) -> CellBox {
        CellBox { x0, z0, x1, z1 }
    }

    /// The single-cell bit of §4.2: `x1 ≤ x0` and `z1 ≤ z0`.
    pub fn is_single(&self) -> bool {
        self.x1 <= self.x0 && self.z1 <= self.z0
    }

    /// The cell `(x, z)` lies in the box (inclusive).
    pub fn contains(&self, x: i32, z: i32) -> bool {
        (i32::from(self.x0)..=i32::from(self.x1)).contains(&x) && (i32::from(self.z0)..=i32::from(self.z1)).contains(&z)
    }

    /// The box's cells, x outer, z inner. Empty when `x1 < x0` or `z1 < z0`.
    pub fn cells(&self) -> impl Iterator<Item = (i32, i32)> {
        let (z0, z1) = (i32::from(self.z0), i32::from(self.z1));
        (i32::from(self.x0)..=i32::from(self.x1)).flat_map(move |x| (z0..=z1).map(move |z| (x, z)))
    }
}

// ---- constants (§3.3, §3.4) ----

/// `4096` (`0x00A8F7C0`): world cell 0 starts at position −4096.
const WORLD_OFFSET: f32 = 4096.0;
/// `0.015625` = 1/64 (`0x00A8F7B8`).
const INV_CELL: f32 = 0.015_625;
/// `0.499` (`0x00A8F7BC`).
const BIAS: f32 = 0.499;
/// `0.0625` = 1/16 (`0x00A8E174`).
const INV_STANDARD: f32 = 0.0625;
/// The 0.1 m shrink of §5.1 step 5 and of the standard-cell boxes (§7, §8.4):
/// `float` `0x3DCCCCCD` (`0x00A8F768`), read as a 4-byte operand everywhere
/// (OM4). Each shrunk bound is a single-precision add or subtract.
pub(crate) const SHRINK: f32 = 0.1;
/// The width limit of §5.1 step 5: `float` 1.0 (`0x00A81228`, OM4).
pub(crate) const SHRINK_MIN_WIDTH: f32 = 1.0;

// ---- slots 10–19 (§3.2) ----

/// Slot 10 `WorldCellToOccupantManagerCell`: x − 64, z − 64.
pub fn world_cell_to_occupant_manager_cell(x: i32, z: i32) -> (i32, i32) {
    (x.wrapping_sub(CITY_ORIGIN_CELL), z.wrapping_sub(CITY_ORIGIN_CELL))
}

/// Slot 11 `OccupantManagerCellToWorldCell`: x + 64, z + 64.
pub fn occupant_manager_cell_to_world_cell(x: i32, z: i32) -> (i32, i32) {
    (x.wrapping_add(CITY_ORIGIN_CELL), z.wrapping_add(CITY_ORIGIN_CELL))
}

/// Slot 12 `WorldCellToStandardCityCell` *[quirk]* (§3.2, §15 quirk 1,
/// OM3): the original is wrong and FreeCity4 reproduces it. `out_z` is the
/// value already in the caller's z output. Returns (outX, outZ) = (4·z −
/// 256, out_z · 4): the x input is ignored.
pub fn world_cell_to_standard_city_cell(x: i32, z: i32, out_z: i32) -> (i32, i32) {
    let _ = x;
    (z.wrapping_mul(4).wrapping_sub(256), out_z.wrapping_mul(4))
}

/// Slot 13 `StandardCityCellToWorldCell`: x / 4 + 64, z / 4 + 64 with C
/// division (toward zero, §15 quirk 2: −1 → 64).
pub fn standard_city_cell_to_world_cell(x: i32, z: i32) -> (i32, i32) {
    (
        x.wrapping_div(4).wrapping_add(CITY_ORIGIN_CELL),
        z.wrapping_div(4).wrapping_add(CITY_ORIGIN_CELL),
    )
}

/// Slot 14 `WorldCellToPosition`: (x − 64) · 64, (z − 64) · 64, the cell's
/// low corner.
pub fn world_cell_to_position(x: i32, z: i32) -> (f32, f32) {
    let (a, b) = world_cell_to_occupant_manager_cell(x, z);
    occupant_manager_cell_to_position(a, b)
}

/// The rounding of slot 15 / 17 (§3.3): `round-half-even(t)` through the
/// 1.5·2²³ idiom.
fn cell_of(t: f32) -> i32 {
    rhe(t)
}

/// Slot 15 `PositionToWorldCell` (§3.3): `rhe(((f + 4096) · 0.015625) −
/// 0.499)`, each step rounded to `float`. That is floor(f / 64 + 64 +
/// 0.001), give or take an ulp; no clamp.
pub fn position_to_world_cell(fx: f32, fz: f32) -> (i32, i32) {
    let one = |f: f32| cell_of((f + WORLD_OFFSET) * INV_CELL - BIAS);
    (one(fx), one(fz))
}

/// Slot 16 `OccupantManagerCellToPosition`: x · 64, z · 64.
pub fn occupant_manager_cell_to_position(x: i32, z: i32) -> (f32, f32) {
    (x as f32 * 64.0, z as f32 * 64.0)
}

/// Slot 17 `PositionToOccupantManagerCell`: slot 15 without the + 4096.
pub fn position_to_occupant_manager_cell(fx: f32, fz: f32) -> (i32, i32) {
    let one = |f: f32| cell_of(f * INV_CELL - BIAS);
    (one(fx), one(fz))
}

/// Slot 18 `StandardCityCellToPosition`: x · 16, z · 16.
pub fn standard_city_cell_to_position(x: i32, z: i32) -> (f32, f32) {
    (x as f32 * 16.0, z as f32 * 16.0)
}

/// Slot 19 `PositionToStandardCityCell` (§3.4): t = f · 0.0625, n =
/// rhe(t − 0.5); n + 1 when t == n + 1, else n. An exact floor.
pub fn position_to_standard_city_cell(fx: f32, fz: f32) -> (i32, i32) {
    let one = |f: f32| {
        let t = f * INV_STANDARD;
        let n = rhe(t - 0.5);
        let up = n.wrapping_add(1);
        if t == up as f32 {
            up
        } else {
            n
        }
    };
    (one(fx), one(fz))
}

// ---- the occupant box (§5.1 steps 5–6) ----

/// One axis of step 5: a span wider than 1.0 m (strict) loses 0.1 m on
/// each side. The width is compared without being stored, as a
/// single-precision subtraction (24-bit precision control, OM4), and each
/// bound is stored as a `float` (§18 test 24: 100.0…164.0 becomes
/// 100.09999847…163.89999389).
pub fn shrink(min: f32, max: f32) -> (f32, f32) {
    if max - min > SHRINK_MIN_WIDTH {
        (min + SHRINK, max - SHRINK)
    } else {
        (min, max)
    }
}

/// The manager box of an occupant's bounding box (§5.1 steps 5–6, used by
/// Insert and Move): x and z shrunk separately, then slot 15 of each
/// corner, each **stored as a byte with no clamp** (§15 quirk 4).
pub fn cell_box_of(b: &BBox) -> CellBox {
    let (x0, x1) = shrink(b.min[0], b.max[0]);
    let (z0, z1) = shrink(b.min[2], b.max[2]);
    let (cx0, cz0) = position_to_world_cell(x0, z0);
    let (cx1, cz1) = position_to_world_cell(x1, z1);
    CellBox::new(cx0 as u8, cz0 as u8, cx1 as u8, cz1 as u8)
}

/// A world cell clamped to the grid (0…191).
pub(crate) fn clamp_cell(v: i32) -> i32 {
    v.clamp(0, WORLD_CELLS - 1)
}

// ---- overlap (§8.2 step 6) ----

/// The strict overlap test `omin < qmax and qmin < omax` on the given axes
/// (0 = x, 1 = y, 2 = z). Touching boxes do not overlap. *[quirk]* The
/// code rejects on `omin ≥ qmax or qmin ≥ omax`, so a NaN coordinate
/// passes (§15 quirk 15).
pub fn overlaps(o: &BBox, qmin: &Vec3, qmax: &Vec3, axes: &[usize]) -> bool {
    axes.iter().all(|&a| !(o.min[a] >= qmax[a] || qmin[a] >= o.max[a]))
}

// ---- slot 26 (§6.2) ----

/// Slot 26 `GetBoundingCells(x, z)`: the neighbours of (x, z) in the order
/// (x−1, z−1), (x−1, z), (x−1, z+1), (x, z−1), (x, z+1), (x+1, z−1),
/// (x+1, z), (x+1, z+1), skipping those outside 0…191 (the fast path for
/// 1 ≤ x, z ≤ 190 writes all eight). The length is `n`.
pub fn bounding_cells(x: i32, z: i32) -> Vec<(i32, i32)> {
    const ORDER: [(i32, i32); 8] = [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)];
    let inside = |v: i32| (0..WORLD_CELLS).contains(&v);
    ORDER
        .iter()
        .map(|&(dx, dz)| (x.wrapping_add(dx), z.wrapping_add(dz)))
        .filter(|&(a, b)| inside(a) && inside(b))
        .collect()
}
