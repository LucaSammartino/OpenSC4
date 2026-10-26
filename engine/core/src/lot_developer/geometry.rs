// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Pure formulas of the lot developer: `GetAreaSlope` and
//! `GetLotDevelopmentCost` (§11.4) and the rubble count of
//! `AddLotRubbleProps` (§7.3). Every x87 operation is one `r24`
//! (foundations §18).

use super::CellRect;
use crate::curve::ResponseCurve;
use crate::x87::{ftol, ftol32, r24};

/// [choice] The most rubble props one explosion places (a hostile bounding
/// box would otherwise ask for billions).
pub const MAX_RUBBLE_PROPS: i64 = 1 << 16;

/// `GetAreaSlope(rect, withWater)` (slot 29, §11.4) over the vertex
/// altitudes `alt(x, z)` for `x = x1 ..= x2 + 1` (outer) and
/// `z = z1 ..= z2 + 1` (inner). The first strict minimum and maximum give
/// `lo` / `hi` and their vertices; the distance between those vertices is
/// scaled in cascade (quirk 12).
pub fn area_slope_of(rect: CellRect, alt: &mut dyn FnMut(i32, i32) -> f32) -> f32 {
    area_slope_with_precision(rect, alt, crate::x87::Precision::Bits24)
}

/// Foundations §18.4: distance and cascade stay in registers until the
/// caller's float store, including during PC53 native re-entry.
pub fn area_slope_with_precision(
    rect: CellRect,
    alt: &mut dyn FnMut(i32, i32) -> f32,
    pc: crate::x87::Precision,
) -> f32 {
    let mut lo = f32::MAX;
    let mut hi = -f32::MAX;
    let (mut lo_pos, mut hi_pos) = ((rect.x1, rect.z1), (rect.x1, rect.z1));
    let x_end = i64::from(rect.x2) + 1;
    let z_end = i64::from(rect.z2) + 1;
    let mut x = i64::from(rect.x1);
    while x <= x_end {
        let mut z = i64::from(rect.z1);
        while z <= z_end {
            let a = alt(x as i32, z as i32);
            if a < lo {
                lo = a;
                lo_pos = (x as i32, z as i32);
            }
            if a > hi {
                hi = a;
                hi_pos = (x as i32, z as i32);
            }
            z += 1;
        }
        x += 1;
    }
    let dx = pc.round(f64::from(hi_pos.0) - f64::from(lo_pos.0));
    let dz = pc.round(f64::from(hi_pos.1) - f64::from(lo_pos.1));
    let mut d = pc.round(pc.round(pc.round(dx * dx) + pc.round(dz * dz)).sqrt());
    if d == 0.0 {
        return 0.0;
    }
    let rise = pc.round(f64::from(hi) - f64::from(lo));
    if d > 6.0 {
        d = pc.round(d * 0.25);
    }
    if d > 5.0 {
        d = pc.round(d * f64::from(0.3f32));
    }
    if d > 4.0 {
        d = pc.round(d * f64::from(0.35f32));
        return pc.round(rise / d) as f32;
    }
    if d > 3.0 {
        d = pc.round(d * f64::from(0.4f32));
    }
    pc.round(rise / d) as f32
}

/// `GetLotDevelopmentCost(rect)` (slot 32, §11.4) from the area slope
/// (`GetAreaSlope(rect, true)`): `s := r24(slope · 0.0625)`, then
/// `trunc64(r24(r24(curve(s) · w) · h))` with `w`, `h` the rectangle's
/// cell counts (`fimul`). An empty curve evaluates to 0 (§3.5 [choice]).
pub fn development_cost(rect: CellRect, slope: f32, curve: &ResponseCurve) -> i64 {
    let w = i64::from(rect.x2) - i64::from(rect.x1) + 1;
    let h = i64::from(rect.z2) - i64::from(rect.z1) + 1;
    let s = r24(f64::from(slope) * 0.0625) as f32;
    let c = f64::from(curve.eval(s));
    ftol(r24(r24(c * (w as i32) as f64) * (h as i32) as f64))
}

/// The shape of a rubble box (§7.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RubbleShape {
    /// Number of indices (corners 0–3, then interiors).
    pub count: i64,
    /// `dx < 10 or dz < 10`: corners get nothing.
    pub small: bool,
    /// `dx > 12 and dz > 12`: corners draw a type.
    pub large: bool,
}

/// The rubble count of `AddLotRubbleProps` (§7.3): `dx := r24(max.x −
/// min.x)`, `dz` likewise, `area := r24(dz · dx)`, `count :=
/// trunc(r24(area · 2^-7)) + 4` (at most [`MAX_RUBBLE_PROPS`]).
pub fn rubble_count(min: [f32; 3], max: [f32; 3]) -> RubbleShape {
    let dx = r24(f64::from(max[0]) - f64::from(min[0]));
    let dz = r24(f64::from(max[2]) - f64::from(min[2]));
    let area = r24(dz * dx);
    let small = dx < 10.0 || dz < 10.0;
    let large = dx > 12.0 && dz > 12.0;
    let count = (i64::from(ftol32(r24(area * 0.0078125))) + 4).min(MAX_RUBBLE_PROPS);
    RubbleShape { count, small, large }
}
