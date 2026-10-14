// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! What the lot developer's `AttemptAreaLevelling` (slot 21) does when the
//! landfill's `LevelRect(rect, altitude, false)` calls it (§4.5.3).
//!
//! The function belongs to the lot developer, whose core leaves slot 21 to its
//! host. The landfill spec is where its behaviour was pinned (§4.5.3 *[compat]*,
//! conformance test 22), so it lives here as a pure function over the three
//! lot-developer slots it uses ([`LevellingSlots`]). A landfill host may call
//! the game's own slot 21 instead, or this function with its own slots; the
//! landfill core only sees [`super::LandfillHost::attempt_area_levelling`].

use crate::grid::CellRect;

/// `FLT_MAX` (`0x7F7FFFFF`, the constant at `0x00AB1B00`): the
/// `maxAreaSlope` that means "use the tuning" (§4.5.3 step 2).
pub const FLT_MAX: f32 = f32::MAX;
/// The slope limit at or above which the slope test is skipped (§4.5.3
/// step 3, the constant at `0x00A8FAF0`).
pub const SLOPE_TEST_LIMIT: f32 = 1000.0;
/// The reason `ModifyTerrainHeight` receives (§4.5.3 step 4); the terrain
/// passes it to `RedisplayTerrain`, which posts `0x278E6003` with
/// `data4 = 5`, a value the landfill ignores (§3.2).
pub const LEVELLING_REASON: u32 = 5;
/// The lot developer's tuning `0xE900EA2A` `maxAreaSlopeAllowedForLevelling`,
/// shipped value (§4.5.3 step 2).
pub const SHIPPED_MAX_AREA_SLOPE: f32 = 40.0;

/// The arguments of `cISC4Demolition::ModifyTerrainHeight` (demolition
/// slot 9) as `AttemptAreaLevelling` passes them (§4.5.3 step 4, conformance
/// test 22): `(1, true, x1, z1, x2 + 1, z2 + 1, avg, lot, 5, null)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModifyTerrainHeight {
    /// The first argument: the literal 1 of test 22.
    pub first: bool,
    /// The second argument: `true` in test 22. It is `AttemptAreaLevelling`'s
    /// fourth argument, `waterFlag` (LF1, `0x006C44F0`); the third,
    /// `withWater`, feeds the vertex average and the slope test. The landfill
    /// passes literal 1 for both, so the order is unobservable from it.
    pub second: bool,
    /// The rect arguments: `x1, z1, x2 + 1, z2 + 1` of the caller's rect.
    pub x1: i32,
    pub z1: i32,
    pub x2: i32,
    pub z2: i32,
    /// The target altitude: the **vertex** average of step 1.
    pub altitude: f32,
    /// The lot (`null` = 0 from the landfill).
    pub lot: u64,
    /// The reason: always [`LEVELLING_REASON`].
    pub reason: u32,
    /// The cost output pointer: always null.
    pub cost_out: Option<i64>,
}

/// The lot-developer and demolition slots `AttemptAreaLevelling` uses
/// (§4.5.3).
pub trait LevellingSlots {
    /// Lot developer slot 18 `GetAverageAreaAltitude(rect, withWater)`: the
    /// mean of `GetTerrainAltitudeByVertex` over the vertices `x1 … x2+1`,
    /// `z1 … z2+1`.
    fn average_area_altitude(&self, rect: CellRect, with_water: bool) -> f32;
    /// Lot developer slot 29 `GetAreaSlope(rect, withWater)` (lot-developer
    /// §11.4).
    fn area_slope(&self, rect: CellRect, with_water: bool) -> f32;
    /// The lot developer's tuning `0xE900EA2A` (field `+0xE8`, shipped
    /// [`SHIPPED_MAX_AREA_SLOPE`]).
    fn max_area_slope_allowed_for_levelling(&self) -> f32;
    /// `cISC4Demolition::ModifyTerrainHeight` (slot 9, `+0x24`, the object at
    /// lot developer `+0x20`); its result is `AttemptAreaLevelling`'s.
    fn modify_terrain_height(&self, call: ModifyTerrainHeight) -> bool;
}

/// `AttemptAreaLevelling(rect, f, withWater, waterFlag, maxAreaSlope, lot)`
/// (lot developer slot 21, `0x006C44F0`), as §4.5.3 describes it *[compat]*:
///
/// 1. `avg := GetAverageAreaAltitude(rect, withWater)`.
/// 2. `m := maxAreaSlope`; if `m` compares equal to `FLT_MAX` **or is
///    unordered** (a NaN), `m := maxAreaSlopeAllowedForLevelling`.
/// 3. If `m < 1000.0` (a NaN `m` skips this test): `s := GetAreaSlope(rect,
///    withWater)`; if `s > m` — strictly, both ordered (a NaN `s` does not
///    block) — return false and change nothing.
/// 4. Otherwise return `ModifyTerrainHeight(1, waterFlag, x1, z1, x2 + 1,
///    z2 + 1, avg, lot, 5, null)`. The terrain is levelled to `avg`.
///
/// *[quirk]* `f` is **never read**: the target is always the vertex average.
pub fn attempt_area_levelling(
    slots: &dyn LevellingSlots,
    rect: CellRect,
    f: f32,
    with_water: bool,
    water_flag: bool,
    max_area_slope: f32,
    lot: u64,
) -> bool {
    // The float argument is never read (§4.5.3 *[quirk]*).
    let _ = f;
    // 1.
    let avg = slots.average_area_altitude(rect, with_water);
    // 2. `FUCOMPP` against FLT_MAX: equal or unordered takes the branch.
    let mut m = max_area_slope;
    if m == FLT_MAX || m.is_nan() {
        m = slots.max_area_slope_allowed_for_levelling();
    }
    // 3. Rust's `<` and `>` are false for NaN, which is the spec's reading of
    //    both compares.
    if m < SLOPE_TEST_LIMIT {
        let s = slots.area_slope(rect, with_water);
        if s > m {
            return false;
        }
    }
    // 4.
    slots.modify_terrain_height(ModifyTerrainHeight {
        first: true,
        second: water_flag,
        x1: rect.x1,
        z1: rect.z1,
        x2: rect.x2 + 1,
        z2: rect.z2 + 1,
        altitude: avg,
        lot,
        reason: LEVELLING_REASON,
        cost_out: None,
    })
}
