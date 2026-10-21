// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The land pass (§7): `IdentifyLandBridgeCells` (§7.2) and
//! `CreateLandBridge` (§7.3).

use crate::network_geometry::Run;
use crate::x87::r24;

use super::select::find_best_bridge_type_recording;
use super::span::SpanWorld;
use super::{step_cells_of, BridgeRecord, BridgeTerrain, BridgeTool};

/// The two terrain vertex altitudes §7.2 reads at the first cell of a step.
///
/// The scan reads `z × +0x248 + x` plus `+0x250[d]` and plus `+0x270[d]`,
/// with `d := pen & 3`. So `d` is 0 for pen 0 and 4 and 2 for pen 2 and 6:
/// the scan always reads the cell's +x edge (x drags) or +z edge (z drags),
/// whichever way the drag runs (quirk 6). `hi` and `lo` are the larger and
/// smaller of the two.
pub fn step_edge_pair(t: &dyn BridgeTerrain, run: &Run, step: u32, city_x: i32) -> (f32, f32) {
    let Some(&(x, z)) = step_cells_of(run, step).first() else {
        return (0.0, 0.0);
    };
    let stride = city_x + 1;
    let base = z * stride + x;
    let d = (run.pen_direction & 3) as usize;
    // `+0x250` and `+0x270` (network-geometry.md §6.1), as the geometry
    // core's `edge_offsets` reads them: the two ends of the cell's +x edge
    // for d = 0, and of its +z edge for d = 2.
    let (o1, o2) = match d {
        0 => (stride, 1),     // (x, z+1) and (x+1, z+1): the +x edge
        2 => (1, stride + 1), // (x+1, z) and (x+1, z+1): the +z edge
        _ => (stride, 1),
    };
    let h1 = t.vertex_altitude(base + o1);
    let h2 = t.vertex_altitude(base + o2);
    (h1.max(h2), h1.min(h2))
}

/// The state of the §7.2 scan.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LandScan {
    pub in_span: bool,
    /// The low and high altitudes of the last step seen outside a span.
    pub a: f32,
    pub b: f32,
    /// The span's first step.
    pub span_start: u32,
    /// The near bank's height (the deck).
    pub d: f32,
    /// The height the ground has to drop below to stay in the span.
    pub e: f32,
    /// The lowest ground under the span. The first step of a span never
    /// lowers it (§7.2).
    pub c: f32,
}

/// `IdentifyLandBridgeCells` (`0x00632AD0`, §7.2). `v` is passed by value and
/// never changed; the pass sets no failure code.
///
/// `limit := min(+0x74, v) − 1`. `max_height` is p+0x3C (`+0x2CC`, 55 for
/// road, one-way and dirt road, 30 for the others) and `bank_tolerance` is
/// p+0x40 (`+0x2D0`, 20 for every shipped type).
#[allow(clippy::too_many_arguments)]
pub fn identify_land_bridge_cells(
    tool: &mut BridgeTool,
    w: &mut SpanWorld<'_>,
    v: u32,
    max_height: f32,
    bank_tolerance: f32,
    city_x: i32,
) {
    let limit = w.straight.min(v).saturating_sub(1);
    if limit < 4 {
        return;
    }
    let run: &Run = w.run;
    let mut st = LandScan {
        c: f32::MAX,
        ..LandScan::default()
    };
    for i in 1..limit {
        // 1. Not kind 0 or fails the edge test: `inSpan := false`; A and B
        // are not updated.
        if !(super::regular3(run, i) && w.edge_ok(i)) {
            st.in_span = false;
            continue;
        }
        let (hi, lo) = step_edge_pair(w.terrain, run, i, city_x);
        if !st.in_span {
            // 2.
            if hi < st.a {
                st.span_start = i.saturating_sub(1);
                st.d = st.b;
                st.e = hi;
                st.c = f32::MAX;
                st.in_span = true;
            } else {
                st.a = lo;
                st.b = hi;
            }
            continue;
        }
        // 3. In a span, `hi < D` (still below the deck).
        if hi < st.d {
            if st.b < st.e || hi < st.e {
                st.c = st.c.min(lo);
                continue;
            }
            st.span_start += 1;
            st.d = st.e;
        }
        // 4. The check. In every case `inSpan := false`.
        if i.saturating_sub(st.span_start) + 1 >= 4
            && r24(f64::from(st.d) - f64::from(st.c)) > f64::from(max_height)
            && (r24(f64::from(st.d) - f64::from(hi))).abs() <= f64::from(bank_tolerance)
        {
            let _ = create_land_bridge(tool, w, st.span_start, i, st.d, city_x);
        }
        st.in_span = false;
    }
}

/// `CreateLandBridge` (`0x00632580`, §7.3). The return value is not used by
/// the scan.
///
/// The recorded span is returned so the caller can mark its steps kind 1 with
/// [`super::mark_bridge_steps`] once it holds the run mutably; the span
/// selection itself only reads the run.
pub fn create_land_bridge(
    tool: &mut BridgeTool,
    w: &SpanWorld<'_>,
    start: u32,
    end: u32,
    elev: f32,
    city_x: i32,
) -> Option<(u32, u32)> {
    let _ = city_x;
    let (mut s, mut e) = (start, end);
    // 1. `FindBestBridgeType(&start, &end, water = 0, h = 0.0, type)`.
    let index = find_best_bridge_type_recording(tool, w, &mut s, &mut e, false, 0.0)?;
    // 2. Any occupant at all blocks a land bridge (unlike the water pass's
    // filtered test) *[compat]*.
    let rect = land_rect(w.run, s, e);
    if w.host.occupant_present(rect, None) {
        tool.candidates.clear();
        return None;
    }
    // 3. Mark the steps and append the record. `+0x10` and `+0x14` are both
    // the near bank's height `D`: `SelectBridgeElevation` is **not** run, so
    // neither `+0x5C` nor `+0x78` is added.
    tool.append_record(BridgeRecord {
        type_id: w.type_id,
        start: s,
        end: e,
        bridge_type: index,
        deck: elev,
        reference: elev,
        land: 1,
        pieces: Vec::new(),
    });
    // 4. As in §4.4 step 4.4: the length map, then the specialised or the
    // parametric model. The caller builds the model.
    Some((s, e))
}

/// The rectangle spanned by the first cell of step `start` and the last cell
/// of step `end`, grown by one cell on every side (§7.3 step 2).
pub fn land_rect(run: &Run, start: u32, end: u32) -> ((i32, i32), (i32, i32)) {
    let a = step_cells_of(run, start).first().copied().unwrap_or((0, 0));
    let b = step_cells_of(run, end).last().copied().unwrap_or(a);
    (
        (a.0.min(b.0) - 1, a.1.min(b.1) - 1),
        (a.0.max(b.0) + 1, a.1.max(b.1) + 1),
    )
}
