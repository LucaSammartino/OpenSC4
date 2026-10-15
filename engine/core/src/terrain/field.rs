// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The height field and the water array (terrain.md §2-§8).

use std::collections::BTreeMap;

use crate::property::PropertyHolder;
use crate::props;
use crate::x87::{ftol32, r24};

use super::brush::BrushDescription;
use super::{flag, msg, tuning_prop as tp, CELL_WIDTH};

/// `0.0625` (`1 / 16`), the cell scale of §4.1.
const INV_CELL: f64 = 0.0625;

/// The largest city side the core accepts, in cells ([choice]: the game
/// uses 64, 128 or 256; anything up to this is accepted).
pub const MAX_CELLS: u32 = 4096;

/// One queued terrain message (§8): `MessagePost(msg, false)` with these four
/// data words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Post {
    pub msg: u32,
    pub data1: u32,
    pub data2: u32,
    pub data3: u32,
    pub data4: u32,
}

/// An inclusive vertex rectangle `(x0, z0, x1, z1)` (left, top, right,
/// bottom), as `RedisplayTerrain` and the rectangle slots take it (§8.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VertRect {
    pub x0: i32,
    pub z0: i32,
    pub x1: i32,
    pub z1: i32,
}

impl VertRect {
    pub const fn new(x0: i32, z0: i32, x1: i32, z1: i32) -> VertRect {
        VertRect { x0, z0, x1, z1 }
    }

    /// The single vertex `(x, z)`.
    pub const fn point(x: i32, z: i32) -> VertRect {
        VertRect {
            x0: x,
            z0: z,
            x1: x,
            z1: z,
        }
    }

    /// Grows the rectangle to include `(x, z)`.
    pub fn include(&mut self, x: i32, z: i32) {
        self.x0 = self.x0.min(x);
        self.z0 = self.z0.min(z);
        self.x1 = self.x1.max(x);
        self.z1 = self.z1.max(z);
    }

    /// The `(x0 << 16) + x1`, `(z0 << 16) + z1` packing of §8.1 (32-bit
    /// wrapping arithmetic).
    pub fn packed(&self) -> (u32, u32) {
        let p = |a: i32, b: i32| (a.wrapping_shl(16)).wrapping_add(b) as u32;
        (p(self.x0, self.x1), p(self.z0, self.z1))
    }
}

/// One surface-water vertex record (§2.2; `0x24` bytes in the original).
/// Only the fields the simulation reads are kept.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WaterVertex {
    /// Metres relative to the ground: water level = altitude + depth.
    /// Negative on dry land; "water" means `depth > 0`.
    pub depth: f32,
    /// `+0x10` (§2.2).
    pub velocity: [f32; 3],
    /// `+0x1C` (§2.2; `GetSedimentValue`, slot 55).
    pub sediment: f32,
}

/// The tuning of §2.4 ("Terrain Properties").
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainTuning {
    /// `0x48CD7B26`, slot 24, the raise clamp.
    pub max_height: f32,
    /// `0x48CD7B27`, slot 23, the lower clamp.
    pub min_height: f32,
    /// `0x48CD7B21`, the sea level of a new terrain.
    pub new_sea_level: f32,
    /// `0x48CD7B2A`, the flat height of a new terrain.
    pub new_flat_height: f32,
    /// `0x48CD7B28`.
    pub image_height_scale: f32,
    /// `0x48CD7B70`, `SetSeaLevel`'s minimum.
    pub sea_level_min: f32,
    /// `0x48CD7B71`, `SetSeaLevel`'s maximum.
    pub sea_level_max: f32,
    /// `0x48CD7B72`, `0x48CD7B73` (editor values).
    pub sea_level_editor: [f32; 2],
    /// `0x48CD7B65` (render).
    pub cliff_slope: f32,
    /// `0x48CD7B20`, stored at `+0x78` and saved.
    pub saved_78: f32,
}

impl TerrainTuning {
    /// The shipped exemplar's values (§2.4).
    pub const SHIPPED: TerrainTuning = TerrainTuning {
        max_height: 2500.0,
        min_height: 20.0,
        new_sea_level: 250.0,
        new_flat_height: 300.0,
        image_height_scale: 3.0,
        sea_level_min: -10.0,
        sea_level_max: 200.0,
        sea_level_editor: [5.0, 40.0],
        cliff_slope: 0.5,
        saved_78: 308.0,
    };

    /// Reads the tuning exemplar. Missing maximum and minimum heights take the
    /// code defaults 200 and 5 (§2.4). [choice] The other properties have no
    /// code default in the spec; a missing one takes the shipped value.
    pub fn from_holder(h: Option<&PropertyHolder>) -> TerrainTuning {
        let mut t = TerrainTuning::SHIPPED;
        t.max_height = 200.0;
        t.min_height = 5.0;
        let Some(h) = h else {
            return t;
        };
        let f = |id: u32, v: &mut f32| {
            if let Some(x) = props::f32_of(h, id) {
                *v = x;
            }
        };
        f(tp::MAX_HEIGHT, &mut t.max_height);
        f(tp::MIN_HEIGHT, &mut t.min_height);
        f(tp::NEW_SEA_LEVEL, &mut t.new_sea_level);
        f(tp::NEW_FLAT_HEIGHT, &mut t.new_flat_height);
        f(tp::IMAGE_HEIGHT_SCALE, &mut t.image_height_scale);
        f(tp::CLIFF_SLOPE, &mut t.cliff_slope);
        f(tp::SAVED_78, &mut t.saved_78);
        if let Some(v) = props::f32s(h, tp::SEA_LEVEL_RANGE) {
            let mut all = [
                t.sea_level_min,
                t.sea_level_max,
                t.sea_level_editor[0],
                t.sea_level_editor[1],
            ];
            for (dst, x) in all.iter_mut().zip(v) {
                *dst = x;
            }
            [
                t.sea_level_min,
                t.sea_level_max,
                t.sea_level_editor[0],
                t.sea_level_editor[1],
            ] = all;
        }
        // §2.4: the new-terrain sea level and flat height are clamped to at
        // least the minimum allowed height.
        if t.new_sea_level < t.min_height {
            t.new_sea_level = t.min_height;
        }
        if t.new_flat_height < t.min_height {
            t.new_flat_height = t.min_height;
        }
        t
    }
}

impl Default for TerrainTuning {
    fn default() -> Self {
        TerrainTuning::SHIPPED
    }
}

/// The terrain (§2.3) with the surface water's depth array (§2.2).
#[derive(Clone, Debug)]
pub struct Terrain {
    pub(super) cells_x: u32,
    pub(super) cells_z: u32,
    pub(super) verts_x: u32,
    pub(super) verts_z: u32,
    pub(super) alt: Vec<f32>,
    pub(super) flags: Vec<u16>,
    pub(super) water: Vec<WaterVertex>,
    /// `+0x70`, `+0x74`: the cached minimum and maximum (§4.6).
    pub(super) min_cache: f32,
    pub(super) max_cache: f32,
    /// `+0x7C`.
    pub(super) sea_level: f32,
    /// `+0x18`.
    pub(super) terrain_type: i32,
    /// `+0x78`.
    pub(super) saved_78: f32,
    pub(super) tuning: TerrainTuning,
    /// `+0xEC`, set by `DoPostCityInit` (§8.1).
    pub(super) view_ready: bool,
    /// `+0xDD` (slots 83, 84).
    pub(super) water_display_state: u8,
    /// `+0xBC`, the brush level used for a negative `params.level` (§9.2).
    pub(super) default_brush_level: i32,
    /// The brush descriptions (§9.1), by brush ID. [choice] The original
    /// keeps one process-wide map; here each terrain has its own, so a brush
    /// angle does not survive a city switch.
    pub(super) brushes: BTreeMap<u32, BrushDescription>,
    pub(super) posts: Vec<Post>,
}

impl Terrain {
    /// `Create` (a new terrain, §4.6, §10.1): every vertex at the tuning's
    /// flat height, the sea at the tuning's sea level (both already clamped
    /// to at least the minimum allowed height), the water reset to the sea
    /// level and the minimum and maximum computed. `None` for a side of 0
    /// cells or more than [`MAX_CELLS`].
    pub fn create(cells_x: u32, cells_z: u32, tuning: TerrainTuning) -> Option<Terrain> {
        let flat = tuning.new_flat_height.max(tuning.min_height);
        let mut t = Terrain::blank(cells_x, cells_z, tuning, flat)?;
        t.sea_level = tuning.new_sea_level.max(tuning.min_height);
        t.reset_water_to_sea_level();
        t.recompute_min_max();
        Some(t)
    }

    /// A terrain from an altitude array in index order (`x + z × vertsX`),
    /// with the sea at `sea_level`, the water reset to it and the minimum and
    /// maximum computed. For tests and tools.
    pub fn from_altitudes(
        cells_x: u32,
        cells_z: u32,
        altitudes: &[f32],
        sea_level: f32,
        tuning: TerrainTuning,
    ) -> Option<Terrain> {
        let mut t = Terrain::blank(cells_x, cells_z, tuning, 0.0)?;
        if altitudes.len() != t.alt.len() {
            return None;
        }
        t.alt.copy_from_slice(altitudes);
        t.sea_level = sea_level;
        t.reset_water_to_sea_level();
        t.recompute_min_max();
        Some(t)
    }

    pub(super) fn blank(cells_x: u32, cells_z: u32, tuning: TerrainTuning, height: f32) -> Option<Terrain> {
        if cells_x == 0 || cells_z == 0 || cells_x > MAX_CELLS || cells_z > MAX_CELLS {
            return None;
        }
        let (vx, vz) = (cells_x + 1, cells_z + 1);
        let n = (vx * vz) as usize;
        Some(Terrain {
            cells_x,
            cells_z,
            verts_x: vx,
            verts_z: vz,
            alt: vec![height; n],
            flags: vec![0; n],
            water: vec![WaterVertex::default(); n],
            min_cache: height,
            max_cache: height,
            sea_level: tuning.new_sea_level,
            terrain_type: 0,
            saved_78: tuning.saved_78,
            tuning,
            view_ready: false,
            water_display_state: 0,
            default_brush_level: 0,
            brushes: BTreeMap::new(),
            posts: Vec::new(),
        })
    }

    // ---- grid (§2.1) ----------------------------------------------------------------------------

    /// `GetCellCountX` (slot 7).
    pub fn cells_x(&self) -> u32 {
        self.cells_x
    }
    /// `GetCellCountZ` (slot 8).
    pub fn cells_z(&self) -> u32 {
        self.cells_z
    }
    pub fn verts_x(&self) -> u32 {
        self.verts_x
    }
    pub fn verts_z(&self) -> u32 {
        self.verts_z
    }
    pub fn vertex_count(&self) -> u32 {
        self.verts_x * self.verts_z
    }
    /// `sizeX = 16 × cellsX` metres.
    pub fn size_x(&self) -> f32 {
        r24(f64::from(CELL_WIDTH) * f64::from(self.cells_x)) as f32
    }
    pub fn size_z(&self) -> f32 {
        r24(f64::from(CELL_WIDTH) * f64::from(self.cells_z)) as f32
    }
    /// The whole-city vertex rectangle `(0, 0, cellsX, cellsZ)` (`+0x40`).
    pub fn whole_city(&self) -> VertRect {
        VertRect::new(0, 0, self.cells_x as i32, self.cells_z as i32)
    }
    pub fn tuning(&self) -> &TerrainTuning {
        &self.tuning
    }

    /// `GetVertIndex` (slot 9): `x + z × vertsX`, no bounds check (32-bit
    /// wrapping).
    pub fn vert_index(&self, x: i32, z: i32) -> u32 {
        (x as u32).wrapping_add((z as u32).wrapping_mul(self.verts_x))
    }

    /// `GetVertex` (slot 10): `(index mod vertsX, index div vertsX)`, unsigned.
    pub fn vertex_of(&self, index: u32) -> (u32, u32) {
        (index % self.verts_x, index / self.verts_x)
    }

    /// The vertex bounds of §3 (unsigned compares).
    pub fn vertex_in_bounds(&self, x: i32, z: i32) -> bool {
        (x as u32) < self.verts_x && (z as u32) < self.verts_z
    }

    /// The cell bounds of §3 (unsigned compares).
    pub fn cell_in_bounds(&self, x: i32, z: i32) -> bool {
        (x as u32) < self.cells_x && (z as u32) < self.cells_z
    }

    /// The array index of an in-bounds vertex.
    pub(super) fn vi(&self, x: i32, z: i32) -> Option<usize> {
        self.vertex_in_bounds(x, z)
            .then(|| x as usize + z as usize * self.verts_x as usize)
    }

    /// An index into the arrays, `None` past their end ([choice]: the
    /// original does not check the slots that take an index).
    pub(super) fn idx(&self, i: u32) -> Option<usize> {
        ((i as usize) < self.alt.len()).then_some(i as usize)
    }

    /// An unchecked `x + z × vertsX` (the slots with no bounds check), as an
    /// array index if it lands inside the arrays.
    pub(super) fn raw_index(&self, x: i64, z: i64) -> Option<usize> {
        let i = x + z * i64::from(self.verts_x);
        (0..self.alt.len() as i64).contains(&i).then_some(i as usize)
    }

    // ---- bounds (§3) ----------------------------------------------------------------------------

    /// `LocationIsInBounds` (slot 11): `0 ≤ x ≤ sizeX`, `0 ≤ z ≤ sizeZ`; NaN
    /// is out.
    pub fn location_is_in_bounds(&self, x: f32, z: f32) -> bool {
        (0.0..=self.size_x()).contains(&x) && (0.0..=self.size_z()).contains(&z)
    }

    // ---- height queries (§4) --------------------------------------------------------------------

    /// `floor(r24(t × 0.0625))` (§4.1 step 2). The original's 1.5 × 2^23
    /// idiom gives exactly `floor` for every in-bounds `t`.
    pub(super) fn cell_floor(t: f32) -> i32 {
        r24(f64::from(t) * INV_CELL).floor() as i32
    }

    /// The altitude of an in-bounds vertex.
    pub(super) fn h(&self, x: i32, z: i32) -> f32 {
        self.alt[x as usize + z as usize * self.verts_x as usize]
    }

    /// `GetAltitude(x, z)` (slot 13, §4.1): linear interpolation on the
    /// triangle of the cell, always across the diagonal `(ix, iz)–(ix + 1,
    /// iz + 1)` *[quirk: the flip flags are ignored]*.
    pub fn altitude(&self, x: f32, z: f32) -> f32 {
        if !self.location_is_in_bounds(x, z) {
            return 0.0;
        }
        let ix = Self::cell_floor(x).min(self.cells_x as i32 - 1);
        let iz = Self::cell_floor(z).min(self.cells_z as i32 - 1);
        let fx = r24(f64::from(x) - 16.0 * f64::from(ix));
        let fz = r24(f64::from(z) - 16.0 * f64::from(iz));
        let h00 = f64::from(self.h(ix, iz));
        let h10 = f64::from(self.h(ix + 1, iz));
        let h01 = f64::from(self.h(ix, iz + 1));
        let h11 = f64::from(self.h(ix + 1, iz + 1));
        let sub = |a: f64, b: f64| r24(a - b);
        let s = if fx <= fz {
            // The triangle h00, h01, h11.
            let a = r24(sub(h11, h01) * fx);
            let b = r24(sub(h01, h00) * fz);
            r24(r24(a + b) * INV_CELL)
        } else {
            // fx > fz (or a NaN compare): the triangle h00, h10, h11.
            let a = r24(sub(h11, h10) * fz);
            let b = r24(sub(h10, h00) * fx);
            r24(r24(a + b) * INV_CELL)
        };
        r24(h00 + s) as f32
    }

    /// `GetAltitude(index)` (slot 12): past the vertex count → 0.0.
    pub fn altitude_at_index(&self, index: u32) -> f32 {
        self.idx(index).map_or(0.0, |i| self.alt[i])
    }

    /// `GetAltitudeAtVertex(x, z)` (slot 14): out of bounds → 0.0.
    pub fn altitude_at_vertex(&self, x: i32, z: i32) -> f32 {
        self.vi(x, z).map_or(0.0, |i| self.alt[i])
    }

    /// `GetAltitudes(rect, …)` (slot 15, *[decomp only]*): the altitudes of
    /// an inclusive vertex rectangle, z outer, x inner. [choice] The
    /// rectangle is clipped to the vertices; an empty clip gives nothing.
    pub fn altitudes_in(&self, r: VertRect) -> Vec<f32> {
        let mut out = Vec::new();
        if let Some(r) = self.clip(r) {
            for z in r.z0..=r.z1 {
                for x in r.x0..=r.x1 {
                    out.push(self.h(x, z));
                }
            }
        }
        out
    }

    /// `GetCellCornerAltitudes` (slot 17): only for an in-bounds cell, in the
    /// order `(x, z)`, `(x, z + 1)`, `(x + 1, z + 1)`, `(x + 1, z)`.
    pub fn cell_corner_altitudes(&self, x: i32, z: i32) -> Option<[f32; 4]> {
        self.cell_in_bounds(x, z)
            .then(|| [self.h(x, z), self.h(x, z + 1), self.h(x + 1, z + 1), self.h(x + 1, z)])
    }

    /// `GetAltitudeAtNearestGrid` (slot 19): in bounds, the vertex
    /// `(round(x / 16), round(z / 16))`, ties to even.
    pub fn altitude_at_nearest_grid(&self, x: f32, z: f32) -> f32 {
        if !self.location_is_in_bounds(x, z) {
            return 0.0;
        }
        let rnd = |t: f32| props::round_half_even_f64(r24(f64::from(t) / 16.0)) as i32;
        self.altitude_at_vertex(rnd(x), rnd(z))
    }

    /// `GetAltitudeCheckSum` (slot 20): the sum of the altitudes of an
    /// inclusive vertex rectangle, 0.0 for an invalid one. [choice] Invalid =
    /// reversed or not wholly inside the vertices; the sum runs z outer, x
    /// inner, each addition rounded.
    pub fn altitude_checksum(&self, r: VertRect) -> f32 {
        if !self.rect_valid(r) {
            return 0.0;
        }
        let mut s = 0.0f64;
        for z in r.z0..=r.z1 {
            for x in r.x0..=r.x1 {
                s = r24(s + f64::from(self.h(x, z)));
            }
        }
        s as f32
    }

    /// `MinHeight` (slot 21): the cached value, possibly stale (§4.6).
    pub fn min_height(&self) -> f32 {
        self.min_cache
    }
    /// `MaxHeight` (slot 22): the cached value, possibly stale (§4.6).
    pub fn max_height(&self) -> f32 {
        self.max_cache
    }
    /// `MinAllowedHeight` (slot 23).
    pub fn min_allowed_height(&self) -> f32 {
        self.tuning.min_height
    }
    /// `MaxAllowedHeight` (slot 24).
    pub fn max_allowed_height(&self) -> f32 {
        self.tuning.max_height
    }

    /// Recomputes the cached minimum and maximum over every vertex (§4.6).
    /// [choice] The scan starts from vertex 0 and replaces on a strict
    /// compare, so a NaN altitude never wins.
    pub fn recompute_min_max(&mut self) {
        let mut lo = self.alt[0];
        let mut hi = self.alt[0];
        for &a in &self.alt[1..] {
            if a < lo {
                lo = a;
            }
            if a > hi {
                hi = a;
            }
        }
        self.min_cache = lo;
        self.max_cache = hi;
    }

    /// Sets the cached minimum and maximum (the misc record's values, §13.1).
    pub fn set_min_max_cache(&mut self, min: f32, max: f32) {
        self.min_cache = min;
        self.max_cache = max;
    }

    /// `GetFaceNormal(x, z)` (slot 26, §4.3): `None` (the output untouched)
    /// unless `0 ≤ x < sizeX` and `0 ≤ z < sizeZ`.
    pub fn face_normal(&self, x: f32, z: f32) -> Option<[f32; 3]> {
        if !(0.0 <= x && x < self.size_x() && 0.0 <= z && z < self.size_z()) {
            return None;
        }
        let ix = Self::cell_floor(x);
        let iz = Self::cell_floor(z);
        if !self.cell_in_bounds(ix, iz) {
            return None;
        }
        let fx = r24(f64::from(x) - 16.0 * f64::from(ix));
        let fz = r24(f64::from(z) - 16.0 * f64::from(iz));
        let h00 = f64::from(self.h(ix, iz));
        let h10 = f64::from(self.h(ix + 1, iz));
        let h01 = f64::from(self.h(ix, iz + 1));
        let h11 = f64::from(self.h(ix + 1, iz + 1));
        let d = r24(h11 - h00);
        let zero_d = r24(0.0 * d);
        let s16 = |a: f64, b: f64| r24(16.0 * r24(a - b));
        let (nx, nz) = if fz < fx {
            let a = s16(h10, h00);
            (r24(zero_d - a), r24(a - s16(h11, h00)))
        } else {
            let a = s16(h01, h00);
            (r24(a - s16(h11, h00)), r24(zero_d - a))
        };
        let sum = r24(r24(r24(nx * nx) + r24(nz * nz)) + 65536.0);
        let s = r24(1.0 / r24(sum.sqrt()));
        Some([r24(nx * s) as f32, r24(256.0 * s) as f32, r24(nz * s) as f32])
    }

    // ---- water queries (§5) ---------------------------------------------------------------------

    /// `GetSeaLevel` (slot 29).
    pub fn sea_level(&self) -> f32 {
        self.sea_level
    }

    fn level_at(&self, i: usize) -> f32 {
        r24(f64::from(self.water[i].depth) + f64::from(self.alt[i])) as f32
    }

    /// `GetWaterLevel(x, z)` (slot 30): `depth + altitude` at the vertex
    /// `(floor(x / 16), floor(z / 16))`, no bounds clamp. [choice] An index
    /// outside the arrays reads 0.0.
    pub fn water_level(&self, x: f32, z: f32) -> f32 {
        let fl = |t: f32| r24(f64::from(t) / 16.0).floor();
        let (fx, fz) = (fl(x), fl(z));
        if !fx.is_finite() || !fz.is_finite() {
            return 0.0;
        }
        self.raw_index(fx as i64, fz as i64).map_or(0.0, |i| self.level_at(i))
    }

    /// `GetWaterLevel(index)` (slot 31). [choice] Past the arrays: 0.0.
    pub fn water_level_at_index(&self, index: u32) -> f32 {
        self.idx(index).map_or(0.0, |i| self.level_at(i))
    }

    /// `GetWaterDepthAtVertex(x, z)` (slot 33): the depth, possibly negative.
    /// [choice] Out of bounds: 0.0.
    pub fn water_depth_at_vertex(&self, x: i32, z: i32) -> f32 {
        self.vi(x, z).map_or(0.0, |i| self.water[i].depth)
    }

    /// `GetWaterDepthAtVertex(index)` (slot 92; surface water slot 5).
    /// [choice] Past the arrays: 0.0.
    pub fn water_depth_at_index(&self, index: u32) -> f32 {
        self.idx(index).map_or(0.0, |i| self.water[i].depth)
    }

    /// `GetMaxSurfaceLevel(x, z)` (slot 18): for an in-bounds cell, the
    /// maximum over the corners `(x, z)`, `(x, z + 1)`, `(x + 1, z + 1)`,
    /// `(x + 1, z)` of `altitude + max(depth, 0)`; else 0.0. [choice] The
    /// maximum starts from the first corner and is replaced on `>`.
    pub fn max_surface_level(&self, x: i32, z: i32) -> f32 {
        if !self.cell_in_bounds(x, z) {
            return 0.0;
        }
        let corners = [(x, z), (x, z + 1), (x + 1, z + 1), (x + 1, z)];
        let mut best = f32::NAN;
        for (k, (cx, cz)) in corners.into_iter().enumerate() {
            let i = cx as usize + cz as usize * self.verts_x as usize;
            let d = self.water[i].depth;
            let d = if d > 0.0 { d } else { 0.0 };
            let v = r24(f64::from(self.alt[i]) + f64::from(d)) as f32;
            if k == 0 || v > best {
                best = v;
            }
        }
        best
    }

    /// The depths of a cell's corners `(x, z)`, `(x + 1, z)`, `(x, z + 1)`,
    /// `(x + 1, z + 1)` (§5, slots 34 and 35).
    fn corner_depths(&self, x: i32, z: i32) -> [f32; 4] {
        let d = |cx: i32, cz: i32| self.water[cx as usize + cz as usize * self.verts_x as usize].depth;
        [d(x, z), d(x + 1, z), d(x, z + 1), d(x + 1, z + 1)]
    }

    /// `CellHasWater` (slot 34): any corner with `depth > 0`; out of bounds
    /// false. A NaN depth is not water.
    pub fn cell_has_water(&self, x: i32, z: i32) -> bool {
        self.cell_in_bounds(x, z) && self.corner_depths(x, z).iter().any(|&d| d > 0.0)
    }

    /// `CellHasLand` (slot 35): any corner with `depth ≤ 0`; out of bounds
    /// false. A NaN depth is not land.
    pub fn cell_has_land(&self, x: i32, z: i32) -> bool {
        self.cell_in_bounds(x, z) && self.corner_depths(x, z).iter().any(|&d| d <= 0.0)
    }

    /// `IsLandVertex` (slot 36).
    pub fn is_land_vertex(&self, x: i32, z: i32) -> bool {
        self.vi(x, z).is_some_and(|i| self.water[i].depth <= 0.0)
    }

    /// `IsWaterVertex` (slot 37).
    pub fn is_water_vertex(&self, x: i32, z: i32) -> bool {
        self.vi(x, z).is_some_and(|i| self.water[i].depth > 0.0)
    }

    /// `IsSeaVertex` (slot 38): `altitude < seaLevel`.
    pub fn is_sea_vertex(&self, x: i32, z: i32) -> bool {
        self.vi(x, z).is_some_and(|i| self.alt[i] < self.sea_level)
    }

    /// `NumberOfWaterVertices(rect)` (slot 53, *[decomp only]*): vertices with
    /// `depth > 0` in an inclusive rectangle. [choice] The rectangle is
    /// clipped to the vertices.
    pub fn number_of_water_vertices(&self, r: VertRect) -> u32 {
        let Some(r) = self.clip(r) else {
            return 0;
        };
        let mut n = 0;
        for z in r.z0..=r.z1 {
            for x in r.x0..=r.x1 {
                if self.water[x as usize + z as usize * self.verts_x as usize].depth > 0.0 {
                    n += 1;
                }
            }
        }
        n
    }

    /// `GetSedimentValue(x, z)` (slot 55). [choice] Out of bounds: 0.0.
    pub fn sediment(&self, x: i32, z: i32) -> f32 {
        self.vi(x, z).map_or(0.0, |i| self.water[i].sediment)
    }

    /// The water record of an in-bounds vertex.
    pub fn water_vertex(&self, x: i32, z: i32) -> Option<WaterVertex> {
        self.vi(x, z).map(|i| self.water[i])
    }

    /// Sets a vertex's depth directly (the surface water's own animation and
    /// the god-mode water brushes, which the core does not model; tests).
    pub fn set_water_depth(&mut self, x: i32, z: i32, depth: f32) -> bool {
        match self.vi(x, z) {
            Some(i) => {
                self.water[i].depth = depth;
                true
            }
            None => false,
        }
    }

    /// `ResetWaterToSeaLevel` (§10.1): every vertex's `depth := sea − alt`,
    /// the other water fields 0.
    pub fn reset_water_to_sea_level(&mut self) {
        let sea = f64::from(self.sea_level);
        for (w, &a) in self.water.iter_mut().zip(&self.alt) {
            *w = WaterVertex {
                depth: r24(sea - f64::from(a)) as f32,
                ..WaterVertex::default()
            };
        }
    }

    // ---- setters (§6) ---------------------------------------------------------------------------

    fn locked(&self, i: usize) -> bool {
        self.flags[i] & flag::LOCKED != 0
    }

    fn depth_from_sea(&mut self, i: usize) {
        self.water[i].depth = r24(f64::from(self.sea_level) - f64::from(self.alt[i])) as f32;
    }

    /// `RaiseVertex(i, d)` (§6): skips a locked vertex; `alt := r24(alt +
    /// d)`, clamped to the maximum; `depth := r24(sea − alt)` *[quirk: even
    /// for d = 0, erasing lake water]*. [choice] An index past the arrays
    /// does nothing.
    pub fn raise_vertex(&mut self, index: u32, d: f32) {
        let Some(i) = self.idx(index) else {
            return;
        };
        if self.locked(i) {
            return;
        }
        let mut a = r24(f64::from(self.alt[i]) + f64::from(d)) as f32;
        if a > self.tuning.max_height {
            a = self.tuning.max_height;
        }
        self.alt[i] = a;
        self.depth_from_sea(i);
    }

    /// `LowerVertex(i, d)` (§6, exported as `RecomputeWaterLevel`): skips a
    /// locked vertex; `alt := r24(alt − d)`, clamped to the minimum; `depth
    /// := r24(sea − alt)`.
    pub fn lower_vertex(&mut self, index: u32, d: f32) {
        let Some(i) = self.idx(index) else {
            return;
        };
        if self.locked(i) {
            return;
        }
        let mut a = r24(f64::from(self.alt[i]) - f64::from(d)) as f32;
        if a < self.tuning.min_height {
            a = self.tuning.min_height;
        }
        self.alt[i] = a;
        self.depth_from_sea(i);
    }

    /// `SetAltitudeAtVertex(i, h)` (slot 50, §6): no clamp. A water vertex
    /// keeps its water level; a land vertex lowered below the sea becomes sea
    /// water; a land vertex raised to or above the sea keeps its stale depth
    /// *[quirk]*.
    pub fn set_altitude_at_vertex(&mut self, index: u32, h: f32) {
        let Some(i) = self.idx(index) else {
            return;
        };
        if self.locked(i) {
            return;
        }
        let old_depth = self.water[i].depth;
        let mut level = r24(f64::from(old_depth) + f64::from(self.alt[i]));
        self.alt[i] = h;
        if old_depth <= 0.0 {
            if self.sea_level <= h {
                return;
            }
            level = f64::from(self.sea_level);
        }
        self.water[i].depth = r24(level - f64::from(h)) as f32;
    }

    /// `SetAltitude(x, z, h)` (slot 51, §6): `i := (cellsX + 1) × z + x`, no
    /// bounds check ([choice]: an index outside the arrays does nothing).
    /// Raising goes through `alt := r24(r24(h − alt) + alt)` with the maximum
    /// clamp; lowering is `LowerVertex(i, alt − h)`; equal or NaN does
    /// nothing.
    pub fn set_altitude(&mut self, x: i32, z: i32, h: f32) {
        let Some(i) = self.raw_index(i64::from(x), i64::from(z)) else {
            return;
        };
        let a = self.alt[i];
        if a < h {
            if self.locked(i) {
                return;
            }
            let mut n = r24(r24(f64::from(h) - f64::from(a)) + f64::from(a)) as f32;
            if n > self.tuning.max_height {
                n = self.tuning.max_height;
            }
            self.alt[i] = n;
            self.depth_from_sea(i);
        } else if a > h {
            self.lower_vertex(i as u32, r24(f64::from(a) - f64::from(h)) as f32);
        }
    }

    /// `SetAltitude(rect, h, redisplay, data4)` (slot 49, §6):
    /// `SetAltitudeAtVertex` over the inclusive rectangle (z outer, x inner);
    /// with `redisplay`, the water of the rectangle is reset (`depth := sea −
    /// alt` where `alt < sea`, else 0) and `RedisplayTerrain(1, 1, rect,
    /// data4)` runs. [choice] The vertex walk is clipped to the vertices; the
    /// redisplay gets the rectangle as given.
    pub fn set_altitude_rect(&mut self, r: VertRect, h: f32, redisplay: bool, data4: u32) {
        let clip = self.clip(r);
        if let Some(c) = clip {
            for z in c.z0..=c.z1 {
                for x in c.x0..=c.x1 {
                    let i = self.vert_index(x, z);
                    self.set_altitude_at_vertex(i, h);
                }
            }
        }
        if redisplay {
            if let Some(c) = clip {
                self.reset_water_rect(c);
            }
            self.redisplay_terrain(true, true, r, data4);
        }
    }

    /// The water reset of `SetAltitude(rect)` (`0x00744CF0`): `depth := sea −
    /// alt` where `alt < sea`, else 0.
    fn reset_water_rect(&mut self, c: VertRect) {
        let sea = self.sea_level;
        for z in c.z0..=c.z1 {
            for x in c.x0..=c.x1 {
                let i = x as usize + z as usize * self.verts_x as usize;
                let a = self.alt[i];
                self.water[i].depth = if a < sea {
                    r24(f64::from(sea) - f64::from(a)) as f32
                } else {
                    0.0
                };
            }
        }
    }

    /// `LockAltitude(rect, lock)` (slot 52): sets or clears `0x4000` on every
    /// vertex of the inclusive rectangle ([choice]: clipped).
    pub fn lock_altitude(&mut self, r: VertRect, lock: bool) {
        if let Some(c) = self.clip(r) {
            for z in c.z0..=c.z1 {
                for x in c.x0..=c.x1 {
                    let f = &mut self.flags[x as usize + z as usize * self.verts_x as usize];
                    if lock {
                        *f |= flag::LOCKED;
                    } else {
                        *f &= !flag::LOCKED;
                    }
                }
            }
        }
    }

    /// `SetSeaLevel(h)` (slot 45): accepted only within the tuning range
    /// (`−10 ≤ h ≤ 200` shipped). Resets the water, posts `0x89171496`, then
    /// `RedisplayTerrain(1, 1, whole city, 0)`. Returns whether it was
    /// accepted.
    pub fn set_sea_level(&mut self, h: f32) -> bool {
        if !(self.tuning.sea_level_min <= h && h <= self.tuning.sea_level_max) {
            return false;
        }
        self.sea_level = h;
        self.reset_water_to_sea_level();
        self.post(msg::SEA_LEVEL_CHANGED, 0, 0, 0, 0);
        let all = self.whole_city();
        self.redisplay_terrain(true, true, all, super::kind::INTERNAL);
        true
    }

    /// Sets the sea level with no range check, message or water reset (load).
    pub fn set_sea_level_raw(&mut self, h: f32) {
        self.sea_level = h;
    }

    /// `GetTerrainType` (slot 42).
    pub fn terrain_type(&self) -> i32 {
        self.terrain_type
    }

    /// `SetTerrainType(t)` (slot 41): when `t` differs, store it, post
    /// `0xC922EB91` (data1 = t) and `RedisplayTerrain(0, 0, whole city, 0)`.
    pub fn set_terrain_type(&mut self, t: i32) {
        if t == self.terrain_type {
            return;
        }
        self.terrain_type = t;
        self.post(msg::TERRAIN_TYPE_CHANGED, t as u32, 0, 0, 0);
        let all = self.whole_city();
        self.redisplay_terrain(false, false, all, super::kind::INTERNAL);
    }

    /// The `+0x78` value (saved in the misc record).
    pub fn saved_78(&self) -> f32 {
        self.saved_78
    }

    // ---- flags (§7) -----------------------------------------------------------------------------

    /// The flag word of a vertex (0 out of bounds).
    pub fn vertex_flags(&self, x: i32, z: i32) -> u16 {
        self.vi(x, z).map_or(0, |i| self.flags[i])
    }

    /// Sets a vertex's flag word (load, tests).
    pub fn set_vertex_flags(&mut self, x: i32, z: i32, f: u16) -> bool {
        match self.vi(x, z) {
            Some(i) => {
                self.flags[i] = f;
                true
            }
            None => false,
        }
    }

    /// `IsTriangulationFlipped(x, z)` (slot 70): tests `0x3000` on the
    /// vertex `(x, z)`. [choice] Out of bounds: false.
    pub fn is_triangulation_flipped(&self, x: i32, z: i32) -> bool {
        self.vertex_flags(x, z) & flag::FLIPPED_MASK != 0
    }

    /// `GetTerrainCellFlags` (slot 82): the cell flags 1, 2, 4, 8 from the
    /// bits `0x20`, `0x40`, `0x200`, `0x2000` of the cell's `(x, z)` vertex.
    /// [choice] Out of bounds: 0.
    pub fn cell_flags(&self, x: i32, z: i32) -> u32 {
        if !self.cell_in_bounds(x, z) {
            return 0;
        }
        let f = self.vertex_flags(x, z);
        CELL_FLAG_BITS
            .iter()
            .enumerate()
            .filter(|(_, &b)| f & b != 0)
            .fold(0, |acc, (k, _)| acc | (1 << k))
    }

    /// `SetTerrainCellFlags` (slot 81). [choice] The four bits are replaced
    /// by the given cell flags; out of bounds does nothing.
    pub fn set_cell_flags(&mut self, x: i32, z: i32, cell_flags: u32) {
        if !self.cell_in_bounds(x, z) {
            return;
        }
        let i = x as usize + z as usize * self.verts_x as usize;
        for (k, &b) in CELL_FLAG_BITS.iter().enumerate() {
            if cell_flags & (1 << k) != 0 {
                self.flags[i] |= b;
            } else {
                self.flags[i] &= !b;
            }
        }
    }

    // ---- RedisplayTerrain and messages (§8) ------------------------------------------------------

    pub(super) fn post(&mut self, msg: u32, data1: u32, data2: u32, data3: u32, data4: u32) {
        self.posts.push(Post {
            msg,
            data1,
            data2,
            data3,
            data4,
        });
    }

    /// The queued messages since the last call, in posting order.
    pub fn take_posts(&mut self) -> Vec<Post> {
        std::mem::take(&mut self.posts)
    }

    /// Whether messages are waiting in the outbox.
    pub fn has_posts(&self) -> bool {
        !self.posts.is_empty()
    }

    /// `DoPostCityInit` (slot 40): sets the view-ready flag (§12.1).
    pub fn post_city_init(&mut self) {
        self.view_ready = true;
    }

    /// The view-ready flag (`+0xEC`).
    pub fn view_ready(&self) -> bool {
        self.view_ready
    }

    /// Sets or clears the view-ready flag directly (tests, shutdown).
    pub fn set_view_ready(&mut self, ready: bool) {
        self.view_ready = ready;
    }

    /// `RedisplayTerrain(terrainFlag, waterFlag, rect, data4)` (slot 68,
    /// §8.1). Nothing before `DoPostCityInit`. With the terrain flag: the
    /// minimum and maximum are recomputed and `0x278E6003` is posted with
    /// `data1 = (x0 << 16) + x1`, `data2 = (z0 << 16) + z1`, `data3 = 0`,
    /// `data4`; with the water flag `0x678E6E3A` with the same; then always
    /// `0x870BC918` (render only).
    ///
    /// [choice] The texture message's packing is not given beyond "data1 /
    /// data2 pack a render rectangle, data3 the minimum and maximum altitude
    /// truncated to integers": here data1 / data2 pack the rectangle grown by
    /// 1 and clipped to the vertices (the normals' rectangle), data3 is
    /// `(trunc(min) << 16) | (trunc(max) & 0xFFFF)` and data4 is 0.
    pub fn redisplay_terrain(&mut self, terrain: bool, water: bool, r: VertRect, data4: u32) {
        if !self.view_ready {
            return;
        }
        let (d1, d2) = r.packed();
        if terrain {
            self.recompute_min_max();
            self.post(msg::TERRAIN_CHANGED, d1, d2, 0, data4);
        }
        if water {
            self.post(msg::WATER_CHANGED, d1, d2, 0, data4);
        }
        let g = VertRect::new(
            r.x0.saturating_sub(1).max(0),
            r.z0.saturating_sub(1).max(0),
            r.x1.saturating_add(1).min(self.cells_x as i32),
            r.z1.saturating_add(1).min(self.cells_z as i32),
        );
        let (t1, t2) = g.packed();
        let lo = ftol32(f64::from(self.min_cache)) as u32;
        let hi = ftol32(f64::from(self.max_cache)) as u32;
        self.post(msg::TEXTURE_REDISPLAY, t1, t2, (lo << 16) | (hi & 0xFFFF), 0);
    }

    /// Slot 69: the cell corners `a` and `b` are sorted, `x1 := max + 1`
    /// clamped to `cellsX` (same for z), and slot 68 runs on the vertex
    /// rectangle `(min x, min z, x1, z1)`.
    pub fn redisplay_terrain_cells(&mut self, terrain: bool, water: bool, a: (i32, i32), b: (i32, i32), data4: u32) {
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (z0, z1) = (a.1.min(b.1), a.1.max(b.1));
        let x1 = x1.saturating_add(1).min(self.cells_x as i32);
        let z1 = z1.saturating_add(1).min(self.cells_z as i32);
        self.redisplay_terrain(terrain, water, VertRect::new(x0, z0, x1, z1), data4);
    }

    /// `BeginLongTerrainOperation` (slot 87): posts `0xEA3B0AF1` (all data
    /// 0). No nesting counter (§8.3).
    pub fn begin_long_terrain_operation(&mut self) {
        self.post(msg::BEGIN_LONG_OPERATION, 0, 0, 0, 0);
    }

    /// `EndLongTerrainOperation` (slot 88): posts `0x6A3B0AFD`.
    pub fn end_long_terrain_operation(&mut self) {
        self.post(msg::END_LONG_OPERATION, 0, 0, 0, 0);
    }

    /// `GetWaterSurfaceDisplayState` (slot 83).
    pub fn water_display_state(&self) -> u8 {
        self.water_display_state
    }

    /// `SetWaterSurfaceDisplayState` (slot 84): stores the state and posts
    /// `0x6AB97795` with data1 = the state (§8.4). [choice] It posts on every
    /// call, changed or not.
    pub fn set_water_display_state(&mut self, state: u8) {
        self.water_display_state = state;
        self.post(msg::WATER_DISPLAY_STATE, u32::from(state), 0, 0, 0);
    }

    // ---- rectangles -----------------------------------------------------------------------------

    /// A rectangle that is not reversed and lies wholly inside the vertices.
    pub(super) fn rect_valid(&self, r: VertRect) -> bool {
        r.x0 <= r.x1 && r.z0 <= r.z1 && self.vertex_in_bounds(r.x0, r.z0) && self.vertex_in_bounds(r.x1, r.z1)
    }

    /// The rectangle clipped to the vertices, `None` when nothing is left.
    pub(super) fn clip(&self, r: VertRect) -> Option<VertRect> {
        let c = VertRect::new(
            r.x0.max(0),
            r.z0.max(0),
            r.x1.min(self.verts_x as i32 - 1),
            r.z1.min(self.verts_z as i32 - 1),
        );
        (c.x0 <= c.x1 && c.z0 <= c.z1).then_some(c)
    }

    /// The default brush level (`+0xBC`). [choice] It starts at 0.
    pub fn default_brush_level(&self) -> i32 {
        self.default_brush_level
    }

    pub fn set_default_brush_level(&mut self, level: i32) {
        self.default_brush_level = level;
    }

    /// The raw altitude array, in index order.
    pub fn altitudes(&self) -> &[f32] {
        &self.alt
    }

    /// The raw flag array, in index order.
    pub fn flag_words(&self) -> &[u16] {
        &self.flags
    }
}

/// The vertex bits of the cell flags 1, 2, 4, 8 (§7).
const CELL_FLAG_BITS: [u16; 4] = [
    flag::EVEN_TRIANGLE_HIDDEN,
    flag::ODD_TRIANGLE_HIDDEN,
    flag::CELL_FLAG_4,
    flag::FLIPPED_EXTERNAL,
];
