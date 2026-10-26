// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The terrain, cost and geometry slots of §11 / §17 over a snapshot of
//! the values they read from the lot developer: the vertex row length
//! `vertsX`, the city extents and cell counts `Init` stores (§1.3, §2.1),
//! the tuning (§3.5) and whether the boundary map exists (§17.7). None of
//! these slots changes the lot developer's own state, so a host that must
//! answer one while the lot developer itself is busy (another subsystem
//! asking for a lot slope in the middle of `StartConstruction`, the BDS
//! reading slot 17 while slot 55 creates a foundation) keeps a
//! [`TerrainView`] from [`super::LotDeveloper::terrain_view`] and calls it
//! directly; the results are the same as the lot developer's own.
//!
//! This file has the altitude helpers (§17.2, slots 15–18 and 22) and the
//! slope and cost slots (§11.4, 25–32); `levelling.rs` (19–21),
//! `positions.rs` (39, 40), `boundary.rs` (42, 43) and `foundation.rs`
//! (48–61) add the rest of §17 to the same type.

use super::geometry::{area_slope_with_precision, development_cost};
use super::tuning::Tuning;
use super::{CellRect, LotDeveloperHost};
use crate::x87::{ftol32, r24};

/// The values the terrain and geometry slots read (see the module docs).
#[derive(Clone, Debug, PartialEq)]
pub struct TerrainView {
    /// `vertsX` (`+0x40`: terrain cell count X + 1, §1.3 step 5 / §13).
    pub verts_x: i32,
    /// `extentX`, `extentZ` (`+0x44`, `+0x48`): the city size in metres
    /// (§1.3 step 5 / §13); slot 16 clamps to them.
    pub extent_x: f32,
    pub extent_z: f32,
    /// The cell counts `Init` stores (`+0x50` X, `+0x4C` Z, §1.3 step 4);
    /// slot 43 skips cells outside them.
    pub cell_count_x: i32,
    pub cell_count_z: i32,
    /// The tuning (§3.5).
    pub tuning: Tuning,
    /// Whether `InitBoundaryRules` has filled the boundary map (§1.3 step
    /// 11; cleared by `Shutdown`, §1.4).
    pub boundary_ready: bool,
}

impl Default for TerrainView {
    /// The constructor's values: no terrain yet, the code-default tuning, no
    /// boundary map.
    fn default() -> TerrainView {
        TerrainView {
            verts_x: 0,
            extent_x: 0.0,
            extent_z: 0.0,
            cell_count_x: 0,
            cell_count_z: 0,
            tuning: Tuning::default(),
            boundary_ready: false,
        }
    }
}

/// `FLT_MAX`, the start value of the lowest / highest searches (§17.2).
pub const FLT_MAX: f32 = f32::MAX;
/// The edge step of slot 16 in metres (§17.2 step 3).
pub const EDGE_STEP: f64 = 4.0;
/// [choice] The most samples one edge sweep of slot 16 takes. With the
/// clamps of step 1 a sweep never gets near it; it only bounds a city whose
/// extents are not finite.
pub const MAX_EDGE_SAMPLES: u32 = 1 << 20;

/// The vertices `x1 ..= x2 + 1` (outer), `z1 ..= z2 + 1` (inner).
pub(crate) fn for_each_vertex(rect: CellRect, mut f: impl FnMut(i32, i32)) {
    let x_end = i64::from(rect.x2) + 1;
    let z_end = i64::from(rect.z2) + 1;
    let mut x = i64::from(rect.x1);
    while x <= x_end {
        let mut z = i64::from(rect.z1);
        while z <= z_end {
            f(x as i32, z as i32);
            z += 1;
        }
        x += 1;
    }
}

/// Keeps the lowest (or highest) of the samples: strict, so a NaN sample
/// never replaces the accumulator (§17.2 slot 17).
fn keep(acc: &mut f32, v: f32, lowest: bool) {
    if (lowest && v < *acc) || (!lowest && v > *acc) {
        *acc = v;
    }
}

/// One edge sweep of slot 16 (§17.2 step 3): `start, start + 4, …`
/// (`r24(t + 4)`) while below `end`, the step that would reach or pass the
/// end replaced by the end itself. The caller only sweeps when `start <
/// end`.
fn sweep(start: f32, end: f32, mut sample: impl FnMut(f32)) {
    let mut t = start;
    let mut n = 0u32;
    loop {
        sample(t);
        n += 1;
        // NaN stops the sweep too.
        let below = t.partial_cmp(&end) == Some(std::cmp::Ordering::Less);
        if !below || n >= MAX_EDGE_SAMPLES {
            return;
        }
        let next = r24(f64::from(t) + EDGE_STEP) as f32;
        if next >= end || next.is_nan() {
            sample(end);
            return;
        }
        t = next;
    }
}

impl TerrainView {
    /// Slot 15 `GetTerrainAltitudeByVertex(x, z, withWater)` (§17.2): the
    /// terrain's altitude at vertex index `x + z · vertsX`; with `withWater`
    /// the water level at the same index when that is larger (strict, NaN
    /// keeps the altitude).
    pub fn altitude_by_vertex(&self, host: &dyn LotDeveloperHost, x: i32, z: i32, with_water: bool) -> f32 {
        let index = x.wrapping_add(z.wrapping_mul(self.verts_x));
        let alt = host.terrain_altitude(index);
        if !with_water {
            return alt;
        }
        let w = host.terrain_water_level(index);
        if w > alt {
            w
        } else {
            alt
        }
    }

    /// `GetCornerAltitudes(rect, out, withWater)` (§17.2, private): slot 15
    /// at `(x1, z1)`, `(x2+1, z1)`, `(x2+1, z2+1)`, `(x1, z2+1)`.
    pub fn corner_altitudes(&self, host: &dyn LotDeveloperHost, r: CellRect, with_water: bool) -> [f32; 4] {
        let x2 = r.x2.wrapping_add(1);
        let z2 = r.z2.wrapping_add(1);
        [
            self.altitude_by_vertex(host, r.x1, r.z1, with_water),
            self.altitude_by_vertex(host, x2, r.z1, with_water),
            self.altitude_by_vertex(host, x2, z2, with_water),
            self.altitude_by_vertex(host, r.x1, z2, with_water),
        ]
    }

    /// Slot 17 `GetLowestOrHighestAreaAltitude(SC4Rect<int32>, lowest,
    /// withWater)` (§17.2): the minimum (start `+FLT_MAX`) or maximum (start
    /// `−FLT_MAX`) of slot 15 over the vertices; `±FLT_MAX` for an empty
    /// rectangle.
    pub fn lowest_or_highest_area_altitude(
        &self,
        host: &dyn LotDeveloperHost,
        rect: CellRect,
        lowest: bool,
        with_water: bool,
    ) -> f32 {
        let mut acc = if lowest { FLT_MAX } else { -FLT_MAX };
        for_each_vertex(rect, |x, z| {
            let v = self.altitude_by_vertex(host, x, z, with_water);
            keep(&mut acc, v, lowest);
        });
        acc
    }

    /// Slot 16 `GetLowestOrHighestAreaAltitude(SC4Rect<float>, lowest,
    /// withWater)` (§17.2): the rectangle `(fx1, fz1, fx2, fz2)` in metres is
    /// clamped to the city, its interior vertices are sampled with slot 15,
    /// then its four edges every 4 m with the terrain's position getters.
    pub fn lowest_or_highest_area_altitude_f(
        &self,
        host: &dyn LotDeveloperHost,
        rect: [f32; 4],
        lowest: bool,
        with_water: bool,
    ) -> f32 {
        let [mut fx1, mut fz1, mut fx2, mut fz2] = rect;
        // 1. Clamp (NaN is never clamped).
        if fx1 < 0.0 {
            fx1 = 0.0;
        }
        if fz1 < 0.0 {
            fz1 = 0.0;
        }
        if fx2 >= self.extent_x {
            fx2 = r24(f64::from(self.extent_x) - 0.1) as f32;
        }
        if fz2 >= self.extent_z {
            fz2 = r24(f64::from(self.extent_z) - 0.1) as f32;
        }
        let mut acc = if lowest { FLT_MAX } else { -FLT_MAX };
        // 2. Interior vertices.
        let first = |v: f32| ftol32(r24(r24(f64::from(v) + 16.0) * 0.0625));
        let last = |v: f32| ftol32(r24(f64::from(v) * 0.0625));
        let (vx1, vx2) = (first(fx1), last(fx2));
        let (vz1, vz2) = (first(fz1), last(fz2));
        if vx1 <= vx2 && vz1 <= vz2 {
            for vx in vx1..=vx2 {
                for vz in vz1..=vz2 {
                    let v = self.altitude_by_vertex(host, vx, vz, with_water);
                    keep(&mut acc, v, lowest);
                }
            }
        }
        // 3. Edges.
        let mut sample = |px: f32, pz: f32| {
            let mut h = host.terrain_altitude_at(px, pz);
            if with_water {
                let w = host.terrain_water_level_at(px, pz);
                if w > h {
                    h = w;
                }
            }
            keep(&mut acc, h, lowest);
        };
        if fx1 < fx2 {
            sweep(fx1, fx2, |t| sample(t, fz1));
        }
        if fz1 < fz2 {
            sweep(fz1, fz2, |t| sample(fx2, t));
        }
        if fx1 < fx2 {
            sweep(fx1, fx2, |t| sample(t, fz2));
        }
        if fz1 < fz2 {
            sweep(fz1, fz2, |t| sample(fx1, t));
        }
        acc
    }

    /// Slot 18 `GetAverageAreaAltitude(SC4Rect<int32>, withWater)` (§17.2;
    /// the SDK's float rectangle is wrong): `r24(sum / n)` over the vertices,
    /// the sum rounded at each step. An empty rectangle gives `0 / 0` = NaN.
    pub fn average_area_altitude(&self, host: &dyn LotDeveloperHost, rect: CellRect, with_water: bool) -> f32 {
        let mut sum = 0.0f64;
        let mut n = 0u32;
        for_each_vertex(rect, |x, z| {
            sum = r24(f64::from(self.altitude_by_vertex(host, x, z, with_water)) + sum);
            n = n.wrapping_add(1);
        });
        // `fild` of the count: exact.
        (r24(sum / f64::from(n))) as f32
    }

    /// Slot 27 `GetAreaAltitudeRange(rect, withWater)`: max − min over the
    /// vertices.
    pub fn area_altitude_range(&self, host: &dyn LotDeveloperHost, rect: CellRect, with_water: bool) -> f32 {
        let hi = self.lowest_or_highest_area_altitude(host, rect, false, with_water);
        let lo = self.lowest_or_highest_area_altitude(host, rect, true, with_water);
        (r24(f64::from(hi) - f64::from(lo))) as f32
    }

    /// Slot 25 `IsAreaFlat(rect, withWater)` (§17.2, LD29): **not** a range
    /// test. The reference is the first vertex `(x1, z1)` read by slot 15
    /// **always with water**, whatever `withWater` says, and stored as
    /// float32; then every vertex `x1 … x2+1` (outer) by `z1 … z2+1` (inner)
    /// is read by slot 15 with `withWater`, and `|v − ref| >
    /// kMaxCellAltitudeDeltaForFlatness` (strict, NaN no) answers false. The
    /// max − min range of a flat rectangle can therefore reach twice the
    /// tuning value. The subtraction is one x87 operation under the game's
    /// 24-bit precision control (`r24`); the absolute value is exact.
    pub fn is_area_flat(&self, host: &dyn LotDeveloperHost, rect: CellRect, with_water: bool) -> bool {
        // LD29: the first vertex, with water (the literal 1), as float32.
        let reference = f64::from(self.altitude_by_vertex(host, rect.x1, rect.z1, true));
        let limit = f64::from(self.tuning.max_cell_altitude_delta_for_flatness);
        let mut flat = true;
        // The original returns at the first vertex beyond the tuning; the
        // reads have no side effects, so stopping the walk early is the same.
        let x_end = i64::from(rect.x2) + 1;
        let z_end = i64::from(rect.z2) + 1;
        let mut x = i64::from(rect.x1);
        'walk: while x <= x_end {
            let mut z = i64::from(rect.z1);
            while z <= z_end {
                let v = f64::from(self.altitude_by_vertex(host, x as i32, z as i32, with_water));
                let d = r24(v - reference).abs();
                // Strict; a NaN difference is never above the limit.
                if d > limit {
                    flat = false;
                    break 'walk;
                }
                z += 1;
            }
            x += 1;
        }
        flat
    }

    /// Slot 26 `IsLotFlat(lot, withWater)`.
    pub fn is_lot_flat(&self, host: &dyn LotDeveloperHost, lot: u64, with_water: bool) -> bool {
        self.is_area_flat(host, host.lot_bounding_rect(lot), with_water)
    }

    /// Slot 29 `GetAreaSlope(rect, withWater)` (§11.4).
    pub fn area_slope(&self, host: &dyn LotDeveloperHost, rect: CellRect, with_water: bool) -> f32 {
        let mut f = |x: i32, z: i32| self.altitude_by_vertex(host, x, z, with_water);
        area_slope_with_precision(rect, &mut f, host.invocation_precision())
    }

    /// Slot 30 `GetLotSlope(lot, withWater)`.
    pub fn lot_slope(&self, host: &dyn LotDeveloperHost, lot: u64, with_water: bool) -> f32 {
        self.area_slope(host, host.lot_bounding_rect(lot), with_water)
    }

    /// Slot 31 `GetOccupantSlope(occupant, withWater)`: slot 29 on slot 22's
    /// rectangle.
    pub fn occupant_slope(&self, host: &dyn LotDeveloperHost, occ: u64, with_water: bool) -> f32 {
        let rect = self.occupant_cell_coverage(host, occ);
        self.area_slope(host, rect, with_water)
    }

    /// Slot 22 `GetOccupantCellCoverage(occupant, rect&)` (§17.2): the
    /// bounding box shrunk by 0.05 on each side, converted with
    /// `PositionToCell`. The slot has no defined result (LD18, LD27): the
    /// original's AL is the x87 status flags `PositionToCell` leaves, and no
    /// caller reads it, so only the rectangle is returned.
    pub fn occupant_cell_coverage(&self, host: &dyn LotDeveloperHost, occ: u64) -> CellRect {
        let (min, max) = host.occupant_bounding_box(occ);
        let grow = |v: f32| (r24(f64::from(v) + 0.05)) as f32;
        let shrink = |v: f32| (r24(f64::from(v) - 0.05)) as f32;
        let (x1, z1) = host.position_to_cell(grow(min[0]), grow(min[2]));
        let (x2, z2) = host.position_to_cell(shrink(max[0]), shrink(max[2]));
        CellRect::new(x1, z1, x2, z2)
    }

    /// Slot 28 `GetLotAltitudeRange(lot, withWater)`: slot 27 on the lot's
    /// bounding rectangle (§11.1 "ST0 (of 27)").
    pub fn lot_altitude_range(&self, host: &dyn LotDeveloperHost, lot: u64, with_water: bool) -> f32 {
        self.area_altitude_range(host, host.lot_bounding_rect(lot), with_water)
    }

    /// Slot 32 `GetLotDevelopmentCost(rect)` (§11.4).
    pub fn lot_development_cost(&self, host: &dyn LotDeveloperHost, rect: CellRect) -> i64 {
        let slope = self.area_slope(host, rect, true);
        development_cost(rect, slope, &self.tuning.cost_curve)
    }
}
