// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The two models that turn a span into pieces (§9, §10): the specialised
//! model (`ConstructSpecializedBridge`) and the parametric model
//! (`ConstructParametricBridge`).

use crate::x87::r24;

use super::specialize::{Specialization, SupportKind};
use super::types::BridgeType;
use super::BridgeRecord;

/// The orientation both models share (§9.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Orientation {
    /// `R := (pen / 2 + 3) & 3`: pen 0 (−x) → 3, pen 2 (−z) → 0, pen 4 (+x)
    /// → 1, pen 6 (+z) → 2.
    pub r: u8,
    /// The corner base `t := [1, 0, 3, 2][R]`.
    pub t: u8,
    /// The corner base of a mirrored ramp, `t` taken with `(R − 2) & 3`
    /// (§10.3).
    pub t_mirror: u8,
    /// True when the cells of a step are taken first to last (`R` is 0 or
    /// 1); otherwise last to first. The first cell taken is **lane** 0.
    pub lanes_forward: bool,
}

/// The corner-base table at `0x00AA8960` and `0x00AA8970` (equal, §9.1).
pub const CORNER_BASE: [u8; 4] = [1, 0, 3, 2];

impl Orientation {
    /// The orientation of a pen direction (§9.1).
    pub fn of(pen: u8) -> Orientation {
        let r = ((pen / 2) + 3) & 3;
        Orientation {
            r,
            t: CORNER_BASE[r as usize],
            t_mirror: CORNER_BASE[((r + 2) & 3) as usize],
            lanes_forward: r == 0 || r == 1,
        }
    }

    /// A piece's rotation (§9.3, §10.3): `(rotation + R) & 3`.
    pub fn rotate(&self, rotation: u8) -> u8 {
        (rotation.wrapping_add(self.r)) & 3
    }

    /// The four corner indices of one edge across the drag: `t` and
    /// `(t + 1) & 3`, and the other edge `(t − 1) & 3` and `(t − 2) & 3`.
    pub fn edges(&self, t: u8) -> ([usize; 2], [usize; 2]) {
        (
            [t as usize, ((t + 1) & 3) as usize],
            [((t.wrapping_sub(1)) & 3) as usize, ((t.wrapping_sub(2)) & 3) as usize],
        )
    }
}

/// The piece kind of a piece record (§9.2 `+0x00`). The specialised model
/// writes 0 for every piece.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum PieceKind {
    /// 0: a ramp piece.
    #[default]
    Ramp = 0,
    /// 1: a pier piece.
    Pier = 1,
    /// 2: a deck piece.
    Deck = 2,
}

/// One piece record (§9.2, `0x50` bytes).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PieceRecord {
    /// `+0x00`: the kind (the specialised model writes 0 for every piece).
    pub kind: PieceKind,
    /// `+0x04`, `+0x08`: the cell x, z.
    pub x: i32,
    pub z: i32,
    /// `+0x0C`: the piece (exemplar instance).
    pub instance: u32,
    /// `+0x10`: the rotation, 0–3.
    pub rotation: u8,
    /// `+0x14`–`+0x20`: the four corner heights, in the terrain corner order
    /// of terrain.md slot 17.
    pub corners: [f32; 4],
    /// `+0x24`: the model height Y.
    pub y: f32,
    /// `+0x28`: the model depth below the origin (−1.0 = never a column).
    pub depth: f32,
    /// `+0x2C`–`+0x38`: the four suspension weights (1.0 when supported).
    pub weights: [f32; 4],
    /// `+0x3C`: the bridge type `+0xA4` when supported.
    pub support_value: f32,
    /// `+0x40`: the support flags (§9.3, §10.4).
    pub flags: u8,
    /// `+0x44`–`+0x4C`: the cable pieces (§11.4). Drawing only.
    pub cables: Vec<u32>,
}

impl PieceRecord {
    /// A supported piece (§9.3): the weights 1.0, `+0x3C := +0xA4`, and the
    /// flags.
    pub fn mark_supported(&mut self, orientation: &Orientation, lane: usize, width: u32, pylon: bool) {
        self.weights = [1.0; 4];
        self.flags = support_flags(orientation, lane, width, pylon);
    }
}

/// The support flags (§9.3, §10.4):
///
/// - 2 if `R` is 1 or 3, else 1 (the drag axis);
/// - `| 4` when the piece is a pylon / the step `s = placed`;
/// - only when W > 1: `| 8` or `| 0x10` by lane. Lane 0 gets 8 when `R` is 0
///   or 3, else 0x10; the other lane gets the other value.
pub fn support_flags(o: &Orientation, lane: usize, width: u32, pylon: bool) -> u8 {
    let mut f = if o.r == 1 || o.r == 3 { 2u8 } else { 1u8 };
    if pylon {
        f |= 4;
    }
    if width > 1 {
        let lane0_is_8 = o.r == 0 || o.r == 3;
        let (a, b) = if lane0_is_8 { (8u8, 0x10u8) } else { (0x10u8, 8u8) };
        f |= if lane == 0 { a } else { b };
    }
    f
}

/// The lane order of a step's cells (§9.1): first to last when `R` is 0 or 1,
/// else last to first. The first cell taken is **lane** 0.
pub fn lane_cells(o: &Orientation, cells: &[(i32, i32)]) -> Vec<(i32, i32)> {
    if o.lanes_forward {
        cells.to_vec()
    } else {
        cells.iter().rev().copied().collect()
    }
}

/// `ConstructSpecializedBridge` (`0x00632060`, §9.3).
///
/// The entry holds the piece list and the step heights `h[]` (§3.2). For each
/// step `s` from `start` to `end`, with `k := s − start`, and each cell of the
/// step in lane order, take the **next** piece of the list (one counter over
/// the whole bridge; the list's length is not checked against the cell count,
/// quirk 9) and append a record.
pub fn construct_specialized_bridge(
    record: &mut BridgeRecord,
    entry: &Specialization,
    bridge: &BridgeType,
    o: &Orientation,
    cells_of: &dyn Fn(u32) -> Vec<(i32, i32)>,
) {
    let elev = record.deck;
    let d4 = bridge.model_height_offset;
    let mut next = 0usize;
    for s in record.start..=record.end {
        let k = (s - record.start) as usize;
        let cells = lane_cells(o, &cells_of(s));
        let mut step_support: Vec<(usize, bool)> = Vec::new();
        for (lane, &(x, z)) in cells.iter().enumerate() {
            let Some(p) = entry.pieces.get(next) else {
                continue;
            };
            next += 1;
            let mut rec = PieceRecord {
                kind: PieceKind::Ramp,
                x,
                z,
                instance: p.instance,
                rotation: o.rotate(p.rotation),
                depth: p.depth,
                ..PieceRecord::default()
            };
            // `Y := r24(r24(elev + P.Y) + +0xD4)`.
            rec.y = r24(r24(f64::from(elev) + f64::from(p.y)) + f64::from(d4)) as f32;
            // Corners `t` and `t + 1`: `r24(h[k] + elev)`; corners `t − 1`
            // and `t − 2`: `r24(h[k + 1] + elev)`.
            let (outer, inner) = o.edges(o.t);
            let hk = entry.heights.get(k).copied().unwrap_or(0.0);
            let hk1 = entry.heights.get(k + 1).copied().unwrap_or(0.0);
            let ho = r24(f64::from(hk) + f64::from(elev)) as f32;
            let hi = r24(f64::from(hk1) + f64::from(elev)) as f32;
            for &c in &outer {
                rec.corners[c] = ho;
            }
            for &c in &inner {
                rec.corners[c] = hi;
            }
            // A piece is supported when its support kind is not 0 or a
            // suspended section is open.
            let supported = p.support != SupportKind::None;
            if supported {
                rec.support_value = bridge.support_value;
                rec.mark_supported(o, lane, bridge.width, p.support == SupportKind::Pylon);
            }
            step_support.push((rec.flags as usize, p.support != SupportKind::None));
            record.pieces.push(rec);
        }
        let _ = &step_support;
    }
}

/// §10.2's pier positions. Returns `(next, placed, spacing, last)`.
///
/// `+0x3C` (0.6 on the suspension types) moves the outer piers to about
/// `0.6 / (1.2 + count)` of the length from each end: the towers of a
/// suspension bridge.
pub fn pier_positions(t: &BridgeType, start: u32, end: u32) -> (i64, i64, u32, i64) {
    let (u34, u38) = (t.pier_spacing, t.first_pier);
    let x = t.last_column_ramp;
    let mut first = start + u38 + x;
    let mut last = end.saturating_sub(u38 + x);
    let next: i64 = -1;
    let mut spacing: u32 = 100_000;
    // 2. If M = 0: no piers.
    if t.pier_steps == 0 {
        return (next, -1, spacing, last as i64);
    }
    // 3. If `first > last`: with a minimum pier count the centre is computed
    // but no pier is placed (quirk 7).
    if first > last {
        if t.min_piers != 0 {
            let _mid = (start + end) / 2;
            // `first := last := (start + end) / 2` (truncated), but `next`
            // stays −1.
        }
        return (-1, -1, spacing, last as i64);
    }
    // 4. `span := last − first`. The loop runs at most two passes.
    let mut span = last - first;
    for pass in 0..2u32 {
        // count := span / u34 (unsigned): the number of gaps between piers.
        let mut count = if u34 > 0 { span / u34 } else { 0 };
        // If `count > +0xAC` and `+0xAC ≠ 1`: `count := +0xAC − 1` (0 if
        // `+0xAC` is 0).
        if count > t.max_piers && t.max_piers != 1 {
            count = t.max_piers.saturating_sub(1);
        }
        // If `count < +0xA8` and `+0xA8 ≠ 1`: `count := +0xA8 − 1`.
        if count < t.min_piers && t.min_piers != 1 {
            count = t.min_piers.saturating_sub(1);
        }
        // count := min(count, span / 2).
        count = count.min(span / 2);
        if count < 1 {
            spacing = 100_000;
            return (-1, -1, spacing, last as i64);
        }
        spacing = span / count;
        if pass == 1 {
            break;
        }
        // First pass only: `f := r24(r24((end − start + 1) / r24(2 × +0x3C +
        // count)) × +0x3C)` and `kk := floor(f) − (x + u38)`.
        let denom = r24(2.0 * f64::from(t.end_pier_fraction) + f64::from(count));
        let f = r24(r24(f64::from(end - start + 1) / denom) * f64::from(t.end_pier_fraction));
        let kk = f.floor() as i64 - i64::from(x + u38);
        if kk >= 1 {
            first += kk as u32;
            last = last.saturating_sub(kk as u32);
            span -= 2 * kk as u32;
        } else {
            break;
        }
    }
    // 5. `next := first`.
    (first as i64, -1, spacing, last as i64)
}

/// `ConstructParametricBridge` (`0x00631300`, §10).
///
/// A parametric bridge has **ramps** at both ends (N steps each), **piers** in
/// groups of M steps, and **deck** pieces between them.
pub fn construct_parametric_bridge(
    record: &mut BridgeRecord,
    bridge: &BridgeType,
    o: &Orientation,
    cells_of: &dyn Fn(u32) -> Vec<(i32, i32)>,
) {
    let (n, m, w) = (
        bridge.ramp_steps,
        bridge.pier_steps,
        bridge.width.max(1),
    );
    let elev = record.deck;
    let (d4, c4) = (bridge.model_height_offset, bridge.cable_params[3]);
    let (next_i0, placed_i0, spacing, last_pier) =
        pier_positions(bridge, record.start, record.end);
    let mut next_i = next_i0;
    let mut placed_i = placed_i0;
    for s in record.start..=record.end {
        let cells = lane_cells(o, &cells_of(s));
        for (lane, &(x, z)) in cells.iter().enumerate() {
            let l = lane as u32;
            let (kind, index, rot_add, depth, mirrored) = if s < record.start + n {
                // 1. Ramp.
                let i = (s - record.start) * w + l;
                (
                    PieceKind::Ramp,
                    i,
                    0u8,
                    bridge.ramp_depths.get((i / w) as usize).copied().unwrap_or(-1.0),
                    false,
                )
            } else if s > record.end - n {
                // 2. Mirrored ramp: `i := (end − s + 1) × W − l − 1`.
                let i = (record.end - s + 1) * w - l - 1;
                (
                    PieceKind::Ramp,
                    i,
                    2u8,
                    bridge.ramp_depths.get((i / w) as usize).copied().unwrap_or(-1.0),
                    true,
                )
            } else if placed_i >= 0 && s as i64 - placed_i < i64::from(m) {
                // 3. Pier group.
                let d = (s as i64 - placed_i) as u32;
                (
                    PieceKind::Pier,
                    d * w + l,
                    0u8,
                    bridge.pier_depths.get(d as usize).copied().unwrap_or(-1.0),
                    false,
                )
            } else if next_i < 1 {
                // 4. Deck.
                (PieceKind::Deck, 0, 0u8, -1.0, false)
            } else {
                // 5. `d := next − s`.
                let d = next_i - i64::from(s);
                if d <= 0 {
                    // The centre pier; `placed := next`, `next := next +
                    // spacing`, and `next := −1` when it is then past `last`.
                    placed_i = next_i;
                    let mut nn = next_i + i64::from(spacing);
                    if last_pier >= 0 && nn > last_pier {
                        nn = -1;
                    }
                    next_i = nn;
                    (
                        PieceKind::Pier,
                        l,
                        0u8,
                        bridge.pier_depths.first().copied().unwrap_or(-1.0),
                        false,
                    )
                } else if d < i64::from(m) {
                    // Mirrored pier piece `d × W + l`, rotation `+ 2`.
                    (
                        PieceKind::Pier,
                        (d as u32) * w + l,
                        2u8,
                        bridge.pier_depths.get(d as usize).copied().unwrap_or(-1.0),
                        true,
                    )
                } else {
                    (PieceKind::Deck, 0, 0u8, -1.0, false)
                }
            };

            let (instance, base_rot) = match kind {
                PieceKind::Ramp => bridge
                    .ramp_pieces
                    .get(index as usize)
                    .map(|p| (p.instance, p.rotation))
                    .unwrap_or((0, 0)),
                PieceKind::Pier => bridge
                    .pier_pieces
                    .get(index as usize)
                    .map(|p| (p.instance, p.rotation))
                    .unwrap_or((0, 0)),
                PieceKind::Deck => (bridge.deck_instance, bridge.deck_rotation),
            };
            // A deck piece is `+0x2C` with rotation `(+0x30 + 2l) & 3`.
            let rot = match kind {
                PieceKind::Deck => (bridge.deck_rotation.wrapping_add(2 * l as u8)) & 3,
                PieceKind::Ramp => (base_rot.wrapping_add(rot_add)) & 3,
                PieceKind::Pier => (base_rot.wrapping_add(rot_add)) & 3,
            };

            let mut rec = PieceRecord {
                kind,
                x,
                z,
                instance,
                rotation: o.rotate(rot),
                depth,
                ..PieceRecord::default()
            };
            match kind {
                PieceKind::Deck => {
                    rec.y = r24(f64::from(elev) + f64::from(d4)) as f32;
                    rec.corners = [elev; 4];
                }
                PieceKind::Pier => {
                    rec.y = r24(r24(f64::from(elev) + f64::from(d4)) + f64::from(c4)) as f32;
                    rec.corners = [elev; 4];
                }
                PieceKind::Ramp => {
                    let u = (index / w) as usize;
                    let off = bridge.ramp_offsets.get(u).copied().unwrap_or(0.0);
                    rec.y = r24(r24(f64::from(elev) - f64::from(off)) + f64::from(d4)) as f32;
                    let t = if mirrored { o.t_mirror } else { o.t };
                    let (outer, inner) = o.edges(t);
                    let hu = bridge.ramp_drops.get(u).copied().unwrap_or(0.0);
                    let hu1 = bridge.ramp_drops.get(u + 1).copied().unwrap_or(0.0);
                    let ho = r24(f64::from(elev) - f64::from(hu)) as f32;
                    let hi = r24(f64::from(elev) - f64::from(hu1)) as f32;
                    for &c in &outer {
                        rec.corners[c] = ho;
                    }
                    for &c in &inner {
                        rec.corners[c] = hi;
                    }
                }
            }
            // §10.4: supported when a pier has been placed or is pending
            // (`placed ≠ −1` or `next ≠ −1`) **and** both cable models are
            // set.
            if (placed_i != -1 || next_i != -1) && bridge.is_cable() {
                rec.support_value = bridge.support_value;
                let pylon = kind == PieceKind::Pier && rec.flags & 4 != 0;
                rec.mark_supported(o, lane, bridge.width, pylon);
            }
            record.pieces.push(rec);
        }
    }
}
