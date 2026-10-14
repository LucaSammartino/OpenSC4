// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The terrain and cost slots of §11.1 / §11.4 (15–18, 22, 25–32) over a
//! snapshot of the three values they read from the lot developer: the
//! vertex row length `vertsX` (§2.1) and the tuning's flatness delta
//! (`0xE900EA24`) and cost curve (`0x2A2D7824`). A host that must answer
//! these slots while the lot developer itself is busy (another subsystem
//! asking for a lot slope in the middle of `StartConstruction`) keeps a
//! [`TerrainView`] from [`super::LotDeveloper::terrain_view`] and calls it
//! directly; the results are the same as the lot developer's own.

use super::geometry::{area_slope_of, development_cost};
use super::{CellRect, LotDeveloperHost};
use crate::curve::ResponseCurve;
use crate::x87::r24;

/// The values the terrain and cost slots read (see the module docs).
#[derive(Clone, Debug, PartialEq)]
pub struct TerrainView {
    /// `vertsX` (terrain cell count X + 1, §1.3 step 5 / §13).
    pub verts_x: i32,
    /// `MaxCellAltitudeDeltaForFlatness` (`0xE900EA24`, §3.5).
    pub flatness: f32,
    /// `CostVsSlope` (`0x2A2D7824`, §3.5).
    pub cost_curve: ResponseCurve,
}

impl Default for TerrainView {
    /// The constructor's values: no terrain yet, the code-default tuning.
    fn default() -> TerrainView {
        TerrainView {
            verts_x: 0,
            flatness: 1.0,
            cost_curve: ResponseCurve::default(),
        }
    }
}

/// The cell rectangle of the float overloads (slots 16 and 18,
/// `SC4Rect<float>`): §11.1 says only that "the float overload converts the
/// rectangle first". *[inferred, question LD-Q2]* Each coordinate is
/// truncated toward zero (the C cast an `SC4Rect<int32>` conversion makes);
/// NaN and out-of-range values saturate.
pub fn cell_rect_of_float(r: [f32; 4]) -> CellRect {
    let t = |v: f32| -> i32 {
        if v.is_nan() {
            0
        } else {
            v.trunc().clamp(i32::MIN as f32, i32::MAX as f32) as i32
        }
    };
    CellRect::new(t(r[0]), t(r[1]), t(r[2]), t(r[3]))
}

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

impl TerrainView {
    /// Slot 15 `GetTerrainAltitudeByVertex(x, z, withWater)`.
    pub fn altitude_by_vertex(&self, host: &dyn LotDeveloperHost, x: i32, z: i32, with_water: bool) -> f32 {
        let index = x.wrapping_add(z.wrapping_mul(self.verts_x));
        let alt = host.terrain_altitude(index);
        if !with_water {
            return alt;
        }
        let w = host.terrain_water_level(index);
        // NaN keeps the altitude.
        if w > alt {
            w
        } else {
            alt
        }
    }

    /// Slots 16 / 17 `GetLowestOrHighestAreaAltitude(rect, lowest,
    /// withWater)` over the vertices `x1 ..= x2 + 1`, `z1 ..= z2 + 1`.
    pub fn lowest_or_highest_area_altitude(
        &self,
        host: &dyn LotDeveloperHost,
        rect: CellRect,
        lowest: bool,
        with_water: bool,
    ) -> f32 {
        let mut best = if lowest { f32::MAX } else { f32::MIN };
        for_each_vertex(rect, |x, z| {
            let a = self.altitude_by_vertex(host, x, z, with_water);
            if (lowest && a < best) || (!lowest && a > best) {
                best = a;
            }
        });
        best
    }

    /// Slot 18 `GetAverageAreaAltitude(rect, withWater)`.
    pub fn average_area_altitude(&self, host: &dyn LotDeveloperHost, rect: CellRect, with_water: bool) -> f32 {
        let mut sum = 0.0f64;
        let mut n = 0i64;
        for_each_vertex(rect, |x, z| {
            sum = r24(sum + f64::from(self.altitude_by_vertex(host, x, z, with_water)));
            n += 1;
        });
        if n == 0 {
            return 0.0;
        }
        (r24(sum / n as f64)) as f32
    }

    /// Slot 27 `GetAreaAltitudeRange(rect, withWater)`: max − min over the
    /// vertices.
    pub fn area_altitude_range(&self, host: &dyn LotDeveloperHost, rect: CellRect, with_water: bool) -> f32 {
        let hi = self.lowest_or_highest_area_altitude(host, rect, false, with_water);
        let lo = self.lowest_or_highest_area_altitude(host, rect, true, with_water);
        (r24(f64::from(hi) - f64::from(lo))) as f32
    }

    /// Slot 25 `IsAreaFlat(rect, withWater)`: the altitude range within
    /// `0xE900EA24` *[inferred: `<=`]*.
    pub fn is_area_flat(&self, host: &dyn LotDeveloperHost, rect: CellRect, with_water: bool) -> bool {
        self.area_altitude_range(host, rect, with_water) <= self.flatness
    }

    /// Slot 26 `IsLotFlat(lot, withWater)`.
    pub fn is_lot_flat(&self, host: &dyn LotDeveloperHost, lot: u64, with_water: bool) -> bool {
        self.is_area_flat(host, host.lot_bounding_rect(lot), with_water)
    }

    /// Slot 29 `GetAreaSlope(rect, withWater)` (§11.4).
    pub fn area_slope(&self, host: &dyn LotDeveloperHost, rect: CellRect, with_water: bool) -> f32 {
        let mut f = |x: i32, z: i32| self.altitude_by_vertex(host, x, z, with_water);
        area_slope_of(rect, &mut f)
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

    /// Slot 22 `GetOccupantCellCoverage(occupant, rect&)`: the bounding box
    /// shrunk by 0.05 on each side, converted with `PositionToCell`.
    pub fn occupant_cell_coverage(&self, host: &dyn LotDeveloperHost, occ: u64) -> CellRect {
        let (min, max) = host.occupant_bounding_box(occ);
        let grow = |v: f32| (r24(f64::from(v) + 0.05)) as f32;
        let shrink = |v: f32| (r24(f64::from(v) - 0.05)) as f32;
        let (_ok1, x1, z1) = host.position_to_cell(grow(min[0]), grow(min[2]));
        let (_ok2, x2, z2) = host.position_to_cell(shrink(max[0]), shrink(max[2]));
        CellRect::new(x1, z1, x2, z2)
    }

    /// Slot 22 with its return: "AL (via call)" (§11.1) is read as the
    /// **second** `PositionToCell`'s AL, the last call the slot makes
    /// *[inferred]*.
    pub fn occupant_cell_coverage_with_flag(&self, host: &dyn LotDeveloperHost, occ: u64) -> (bool, CellRect) {
        let (min, max) = host.occupant_bounding_box(occ);
        let grow = |v: f32| (r24(f64::from(v) + 0.05)) as f32;
        let shrink = |v: f32| (r24(f64::from(v) - 0.05)) as f32;
        let (_ok1, x1, z1) = host.position_to_cell(grow(min[0]), grow(min[2]));
        let (ok2, x2, z2) = host.position_to_cell(shrink(max[0]), shrink(max[2]));
        (ok2, CellRect::new(x1, z1, x2, z2))
    }

    /// Slot 28 `GetLotAltitudeRange(lot, withWater)`: slot 27 on the lot's
    /// bounding rectangle (§11.1 "ST0 (of 27)").
    pub fn lot_altitude_range(&self, host: &dyn LotDeveloperHost, lot: u64, with_water: bool) -> f32 {
        self.area_altitude_range(host, host.lot_bounding_rect(lot), with_water)
    }

    /// Slot 32 `GetLotDevelopmentCost(rect)` (§11.4).
    pub fn lot_development_cost(&self, host: &dyn LotDeveloperHost, rect: CellRect) -> i64 {
        let slope = self.area_slope(host, rect, true);
        development_cost(rect, slope, &self.cost_curve)
    }
}
