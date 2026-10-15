// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Levelling, slots 19, 20 and 21 (§17.4). Every terrain write goes through
//! the demolition utility's `ModifyTerrainHeight` (§17.3), a host call
//! ([`LotDeveloperHost::modify_terrain_height`]); the lot developer always
//! passes `doIt = 1`, `reason = 5` and a null cost.

use super::terrain::{TerrainView, FLT_MAX};
use super::{CellRect, LotDeveloperHost};
use crate::x87::r24;

/// `AttemptAreaLevelling` skips the slope test from this maximum slope on
/// (§17.4 step 3: `m < 1000`).
pub const NO_SLOPE_TEST_FROM: f32 = 1000.0;

impl TerrainView {
    /// Slot 19 `LevelAreaInternalVertices(rect, target, skipWater, force,
    /// lot)` (§17.4). Returns the number of `ModifyTerrainHeight` calls (EAX;
    /// as a bool, AL is its low byte).
    ///
    /// A negative target (strict, NaN no) becomes the vertex average
    /// (slot 18 with water). Only the interior vertices `x1 + 1 ..= x2`,
    /// `z1 + 1 ..= z2` are visited; without `force` a vertex is levelled
    /// only when it lies more than `lotAutoLevelInternalAltitudeDelta`
    /// (`0xE900EA29`) above the target.
    pub fn level_area_internal_vertices(
        &self,
        host: &dyn LotDeveloperHost,
        rect: CellRect,
        target: f32,
        skip_water: bool,
        force: bool,
        lot: Option<u64>,
    ) -> i32 {
        let mut target = target;
        if target < 0.0 {
            target = r24(f64::from(self.average_area_altitude(host, rect, true))) as f32;
        }
        let delta = self.tuning.auto_level_internal_altitude_delta;
        let mut count = 0i32;
        let mut x = i64::from(rect.x1) + 1;
        while x <= i64::from(rect.x2) {
            let mut z = i64::from(rect.z1) + 1;
            while z <= i64::from(rect.z2) {
                let (vx, vz) = (x as i32, z as i32);
                z += 1;
                if skip_water && host.terrain_is_water_vertex(vx, vz) {
                    continue;
                }
                let a = self.altitude_by_vertex(host, vx, vz, true);
                if force || (r24(f64::from(a) - f64::from(target)) as f32) > delta {
                    host.modify_terrain_height(true, vx, vz, vx, vz, target, lot);
                    count = count.wrapping_add(1);
                }
            }
            x += 1;
        }
        count
    }

    /// Slot 20 `LevelBorderingVertices(rect, h, unused, lot)` (§17.4):
    /// `ModifyTerrainHeight(1, 1, x1, z1, x2 + 1, z2 + 1, h, lot, 5, null)`
    /// over **every** vertex of the rectangle; returns its EAX.
    pub fn level_bordering_vertices(
        &self,
        host: &dyn LotDeveloperHost,
        rect: CellRect,
        h: f32,
        lot: Option<u64>,
    ) -> i32 {
        host.modify_terrain_height(
            true,
            rect.x1,
            rect.z1,
            rect.x2.wrapping_add(1),
            rect.z2.wrapping_add(1),
            h,
            lot,
        )
    }

    /// Slot 21 `AttemptAreaLevelling(rect, unused, withWater, perVertex,
    /// maxAreaSlope, lot)` (§17.4): levels every vertex to the vertex
    /// average unless the area is too steep. Returns 0 or
    /// `ModifyTerrainHeight`'s EAX.
    pub fn attempt_area_levelling(
        &self,
        host: &dyn LotDeveloperHost,
        rect: CellRect,
        with_water: bool,
        per_vertex: bool,
        max_area_slope: f32,
        lot: Option<u64>,
    ) -> i32 {
        let avg = self.average_area_altitude(host, rect, with_water);
        let mut m = max_area_slope;
        if m == FLT_MAX || m.is_nan() {
            m = self.tuning.max_area_slope_for_levelling;
        }
        if m < NO_SLOPE_TEST_FROM && self.area_slope(host, rect, with_water) > m {
            return 0;
        }
        host.modify_terrain_height(
            per_vertex,
            rect.x1,
            rect.z1,
            rect.x2.wrapping_add(1),
            rect.z2.wrapping_add(1),
            avg,
            lot,
        )
    }
}
