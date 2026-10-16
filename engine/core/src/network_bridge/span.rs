// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The water pass's step test (§4.2), edge test (§4.3), span selection
//! (`SelectBridgeStartAndEnd`, §5.1) and span adjustment
//! (`AdjustBridgeSpan`, §5.2).

use crate::network_geometry::smoothing::CellInfo;
use crate::network_geometry::Run;
use crate::x87::r24;

use super::{regular3, step_cells_of, step_rect, BridgeHost, BridgeTerrain};

/// The tool's OR filter mask (§4.2): `0x1F4F` (road, rail, highway, street,
/// avenue, light rail, monorail, one-way road, dirt road and ground highway)
/// for those types, and only the type's own bit (`1 << type`) for pipe, power
/// line and subway.
pub fn occupant_filter_mask(type_id: u32) -> u32 {
    match type_id {
        4 | 5 | 7 => 1u32 << type_id,
        _ => 0x1F4F,
    }
}

/// §4.2: a step is **clear** when the occupant manager finds no occupant
/// passing the tool's OR filter in the rectangle of the step's first and last
/// cells, grown by one cell on every side. A failed query also counts as
/// clear.
pub fn is_step_good_for_bridge(run: &Run, step: u32, type_id: u32, host: &dyn BridgeHost) -> bool {
    let Some(rect) = step_rect(run, step, 1) else {
        return true;
    };
    !host.occupant_present(rect, Some(occupant_filter_mask(type_id)))
}

/// The allowed edge mask of §4.3, by pen direction and by the cell's place in
/// the step.
pub struct EdgeMask;

impl EdgeMask {
    /// The allowed mask for a cell of a step (§4.3). `index` is the cell's
    /// place: 0 = first, `n − 1` = last, anything else = middle. A one-cell
    /// step uses the one-cell row.
    pub fn allowed(pen: u8, index: usize, n: usize) -> u32 {
        let along_x = pen == 0 || pen == 4;
        if n <= 1 {
            return if along_x { 0x0002_0002 } else { 0x0200_0200 };
        }
        match (along_x, index) {
            (true, 0) => 0x0402_0002,
            (true, i) if i + 1 == n => 0x0002_0402,
            (true, _) => 0x0402_0402,
            (false, 0) => 0x0204_0200,
            (false, i) if i + 1 == n => 0x0200_0204,
            (false, _) => 0x0204_0204,
        }
    }
}

/// The cell infos the edge test and the height solve read, by cell.
pub type InfoMap<'a> = &'a dyn Fn(i32, i32) -> Option<CellInfo>;

/// §4.3 `CheckBridgeStepEdgeTypes(step, type)`: for every cell of the step
/// that already has a cell info, the step fails when the cell info's type
/// mask has a bit of the low 13 other than `1 << type`, or its edge word for
/// this type has a bit outside the allowed mask. Cells without a cell info
/// pass.
pub fn bridge_step_edge_types_ok(
    run: &Run,
    step: u32,
    type_id: u32,
    infos: InfoMap<'_>,
) -> bool {
    let cells = step_cells_of(run, step);
    let n = cells.len();
    for (k, &(x, z)) in cells.iter().enumerate() {
        let Some(ci) = infos(x, z) else { continue };
        // The type mask (`+0x08`): a bit of the low 13 other than `1 << type`.
        let want = 1u32 << type_id;
        if ci.flags & 0x1FFF & !want != 0 {
            return false;
        }
        // The edge word for this type (`+0x18 + 4 × type`).
        let word = ci.edges.get(type_id as usize).copied().unwrap_or(0);
        if word & !EdgeMask::allowed(run.pen_direction, k, n) != 0 {
            return false;
        }
    }
    true
}

/// §5.2's two step measures over a step's cells.
pub trait StepMeasures: BridgeTerrain {
    /// `lowOf(step)`: the largest, over the step's cells, of the cell's
    /// **lowest** terrain corner (terrain slot 17), but at least 0.
    fn low_of(&self, run: &Run, step: u32) -> f32 {
        let mut m = 0.0f32;
        for &(x, z) in step_cells_of(run, step) {
            m = m.max(self.lowest_corner(x, z));
        }
        m.max(0.0)
    }
    /// `surfaceOf(step)`: the largest, over the step's cells, of the cell's
    /// surface level (terrain slot 18), but at least 0.
    fn surface_of(&self, run: &Run, step: u32) -> f32 {
        let mut m = 0.0f32;
        for &(x, z) in step_cells_of(run, step) {
            m = m.max(self.surface_level(x, z));
        }
        m.max(0.0)
    }
}

/// Everything the water pass needs from the tool and the world.
pub struct SpanWorld<'a> {
    /// The dragged run.
    pub run: &'a Run,
    /// The tool's `+0x78`, the valid step count.
    pub valid: u32,
    /// The tool's `+0x74`, the straight step count.
    pub straight: u32,
    /// The tool's `+0x35C`, the network type.
    pub type_id: u32,
    /// The terrain (§5.2's measures, and the wet test).
    pub terrain: &'a dyn StepMeasures,
    /// The occupants and the exemplars.
    pub host: &'a dyn BridgeHost,
    /// The cell infos.
    pub infos: InfoMap<'a>,
}

impl SpanWorld<'_> {
    /// §4.3 over this world.
    pub fn edge_ok(&self, step: u32) -> bool {
        bridge_step_edge_types_ok(self.run, step, self.type_id, self.infos)
    }
    /// §4.2 over this world.
    pub fn step_clear(&self, step: u32) -> bool {
        is_step_good_for_bridge(self.run, step, self.type_id, self.host)
    }
    /// Whether any cell of the step has water (terrain slot 34).
    pub fn step_wet(&self, step: u32) -> bool {
        step_cells_of(self.run, step)
            .iter()
            .any(|&(x, z)| self.terrain.cell_has_water(x, z))
    }
}

/// `AdjustBridgeSpan`'s outputs (§5.2).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AdjustedSpan {
    pub start: u32,
    pub end: u32,
    /// The clearance (§5.2's last step).
    pub clearance: f32,
    /// The reference elevation `+0x14`.
    pub reference: f32,
}

/// `AdjustBridgeSpan` (`0x0062FED0`, §5.2): finds the banks' heights, chooses
/// the reference elevation, then widens the span until it meets the banks.
///
/// `max_slope_rad` is the placement parameter p+0x00, the maximum slope **in
/// radians**; `altitude_override` is the tool's `+0x204` (−1.0 = none).
pub fn adjust_bridge_span(
    w: &SpanWorld<'_>,
    start: u32,
    end: u32,
    max_slope_rad: f32,
    altitude_override: f32,
) -> Option<AdjustedSpan> {
    // Two thresholds (§5.2): `c14 := r24(p0 × 14.0)` (`0x00A98F48`) and
    // `c4 := r24(p0 × 4.0)` (`0x00A8E17C`). [quirk 4] Multiplying an angle by
    // a plain constant suggests a rise per cell was meant.
    let c14 = r24(f64::from(max_slope_rad) * 14.0) as f32;
    let c4 = r24(f64::from(max_slope_rad) * 4.0) as f32;
    let (t, run) = (w.terrain, w.run);
    let mut s = start;
    let mut e = end;

    // Back bank: `A := 0`, `B := 0`, `low := 1`. For `k := start` down while
    // `k > 1`. Every step that qualifies overwrites A or B, so the farthest
    // qualifying step wins.
    let mut a = 0.0f32;
    let mut b = 0.0f32;
    let mut low = 1u32;
    let mut k = s;
    while k > 1 {
        if !(regular3(run, k) && w.step_clear(k) && w.edge_ok(k)) {
            low = k;
            break;
        }
        let d = r24(f64::from(t.surface_of(run, k - 1)) - f64::from(t.low_of(run, k))) as f32;
        if d > c14 {
            a = t.surface_of(run, k - 1);
        }
        if d > c4 {
            b = t.surface_of(run, k - 1);
        }
        k -= 1;
    }

    // Far bank: `A2 := 0`, `B2 := 0`, `far := +0x78 − 2`, `done := false`.
    let mut a2 = 0.0f32;
    let mut b2 = 0.0f32;
    let mut far = w.valid.saturating_sub(2);
    let mut done = false;
    let mut j = e;
    while j < w.valid.saturating_sub(2) {
        if !(regular3(run, j) && w.step_clear(j) && w.edge_ok(j)) {
            far = j;
            break;
        }
        if !done {
            let d = r24(f64::from(t.surface_of(run, j + 1)) - f64::from(t.low_of(run, j))) as f32;
            if d > c14 {
                a2 = t.surface_of(run, j + 1);
            }
            if d > c4 {
                b2 = t.surface_of(run, j + 1);
            } else {
                done = true;
            }
        }
        j += 1;
    }

    // Elevation: `elev := min(B, B2, max(A, A2))`.
    let mut elev = b.min(b2).min(a.max(a2));
    if altitude_override >= 0.0 {
        elev = altitude_override;
    }

    // Widen.
    while s > low && t.surface_of(run, s) < elev {
        s -= 1;
    }
    while e < far && t.surface_of(run, e) < elev {
        e += 1;
    }

    // Floor: if `elev < 0.01` (a double, `0x00A92688`), `elev` is the surface
    // level (slot 18) of the first cell of step `start`.
    if f64::from(elev) < 0.01 {
        if let Some(&(x, z)) = step_cells_of(run, s).first() {
            elev = t.surface_level(x, z);
        }
    }

    // Clearance: for each step `k` from `start` to `end − 1`, if the step has
    // a wet cell, take the first one, `c`, and keep the largest
    // `elev − surface(c)`.
    let mut clearance = 0.0f32;
    for k in s..e {
        if let Some(&(x, z)) = step_cells_of(run, k)
            .iter()
            .find(|&&(x, z)| t.cell_has_water(x, z))
        {
            let c = r24(f64::from(elev) - f64::from(t.surface_level(x, z))) as f32;
            clearance = clearance.max(c);
        }
    }

    Some(AdjustedSpan {
        start: s,
        end: e,
        clearance,
        reference: elev,
    })
}

/// `SelectBridgeStartAndEnd` (`0x00630570`, §5.1): the first span is the wet
/// run plus one dry step on each side. Returns `(start, end)`; `None` fails
/// the span.
pub fn select_bridge_start_and_end(w: &SpanWorld<'_>, i: u32) -> Option<(u32, u32)> {
    let run = w.run;
    // 1. `start := i − 1`. Fail unless `start ≥ 0`, step `start` is kind 0
    // and passes the edge test. (Step `start` is not tested for occupants.)
    let start = i.checked_sub(1)?;
    if !(start < run.steps.len() as u32 && regular3(run, start) && w.edge_ok(start)) {
        return None;
    }
    // 2. `end := i + 1`. Fail unless `end < +0x78 − 1`.
    let mut end = i + 1;
    if end >= w.valid.saturating_sub(1) {
        return None;
    }
    // 3. Loop: fail unless step `end` is kind 0, clear and passes the edge
    // test. If none of its cells has water, it is the far bank.
    loop {
        if !(regular3(run, end) && w.step_clear(end) && w.edge_ok(end)) {
            return None;
        }
        if !w.step_wet(end) {
            break;
        }
        end += 1;
        if end >= w.valid.saturating_sub(1) {
            return None;
        }
    }
    // 4. Fail if `end ≥ +0x74`.
    if end >= w.straight {
        return None;
    }
    Some((start, end))
}
