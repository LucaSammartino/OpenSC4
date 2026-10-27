// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 FreeCity4 contributors

use crate::map_view::ring_float::F;
use crate::map_view::RingContext;

/// Lower and exclusive upper vertex bounds before grid clamping. The camera
/// and intersection producers are inputs, not approximated here (§15.5.15.6).
pub fn coordinate_bounds(coordinate: f32, context: RingContext) -> Option<(i32, i32)> {
    // SPEC-GAP: nonfinite camera intersections are rejected instead of emulating unchecked native conversions.
    if !coordinate.is_finite() {
        return None;
    }
    let t = context.mul(F::float(coordinate), F::float(0.0625));
    Some((context.integer(t, false), context.integer(t, true).wrapping_add(1)))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ViewColumnInterval {
    pub left: i32,
    pub right: i32,
}

impl ViewColumnInterval {
    /// The upper clamp compares unsigned: a negative upper result becomes W.
    pub fn clamped(lower: i32, upper_exclusive: i32, verts_x: u32) -> Self {
        Self {
            left: lower.max(0).min(verts_x as i32),
            right: (upper_exclusive as u32).min(verts_x) as i32,
        }
    }
}

/// Ascending, so each union uses the next row's original interval. This must
/// not recursively propagate the last row's interval to all preceding rows.
pub fn widen_row_intervals(rows: &mut [ViewColumnInterval]) {
    for row in 0..rows.len().saturating_sub(1) {
        rows[row].left = rows[row].left.min(rows[row + 1].left);
        rows[row].right = rows[row].right.max(rows[row + 1].right);
    }
}

/// Finite row-culling inputs after the camera has produced H (§15.5.15.6).
/// Water equal to the land minimum is invisible, even at the visible plane.
pub fn row_visibility(nonempty: bool, visible_min: f32, land_min: f32, land_max: f32, water_max: f32) -> [bool; 2] {
    [
        nonempty && land_max >= visible_min,
        nonempty && water_max >= visible_min && water_max > land_min,
    ]
}
