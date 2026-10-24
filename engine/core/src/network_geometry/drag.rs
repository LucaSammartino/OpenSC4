// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! From a drag to steps and cells (§3): `BreakIntoStraightAndDiagSegments`
//! (§3.2), `ComputeDraggedCells` (§3.3) and `DrawNetworkLine` (§3.1).

/// A step's kind (§2.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum StepKind {
    #[default]
    Regular = 0,
    /// A bridge step, the two abutments included.
    Bridge = 1,
    /// A tunnel portal.
    Portal = 2,
    /// A tunnel interior step.
    TunnelInterior = 3,
}

impl StepKind {
    pub fn from_u32(v: u32) -> Option<StepKind> {
        Some(match v {
            0 => StepKind::Regular,
            1 => StepKind::Bridge,
            2 => StepKind::Portal,
            3 => StepKind::TunnelInterior,
            _ => return None,
        })
    }
}

/// `tDraggedStep` (§2.2): the first and last index into the dragged cells and
/// the kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Step {
    pub first: u32,
    pub last: u32,
    pub kind: StepKind,
}

/// The outputs of `BreakIntoStraightAndDiagSegments` (§3.2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Segments {
    /// The end after the adjustments.
    pub end: (i32, i32),
    /// Tool `+0x70`.
    pub dragged_count: u32,
    /// Tool `+0x74`.
    pub straight_count: u32,
    /// The two axis outputs: `(0, 1)` when dx > dz, else `(1, 0)`; the first
    /// is the major-axis flag `+0x6C`.
    pub axis: (u8, u8),
    /// Tool `+0x7C`: 0 = −x, 2 = −z, 4 = +x, 6 = +z.
    pub pen_direction: u8,
    /// Tool `+0x80`: 1, 3, 5 or 7.
    pub end_direction: u8,
}

/// The unit offset of a direction `d` (0..7): even = the axes of §2.1,
/// odd = the diagonals between them (1 = −x−z, 3 = +x−z, 5 = +x+z,
/// 7 = −x+z, as §3.2 step 6 picks them).
pub fn direction_offset(d: u8) -> (i32, i32) {
    match d & 7 {
        0 => (-1, 0),
        1 => (-1, -1),
        2 => (0, -1),
        3 => (1, -1),
        4 => (1, 0),
        5 => (1, 1),
        6 => (0, 1),
        _ => (-1, 1),
    }
}

/// `BreakIntoStraightAndDiagSegments` (`0x00637EB0`, §3.2) *[compat]*.
/// `w1`, `w2` are the type's widths (type info `+0x1C`, `+0x20`).
pub fn break_into_segments(
    start: (i32, i32),
    end: (i32, i32),
    snap: bool,
    straight_only: bool,
    w1: u32,
    w2: u32,
) -> Segments {
    let (x1, z1) = start;
    let (ox2, oz2) = end;
    let (mut x2, mut z2) = end;
    let mut dx = (x2 - x1).abs();
    let mut dz = (z2 - z1).abs();

    // 1. Pre-adjust: the first matching rule.
    if dx == 1 && dz > 2 {
        x2 = x1;
        dx = 0;
    } else if dz == 1 && dx > 2 {
        z2 = z1;
        dz = 0;
    } else if dx == dz + 2 && dz != 0 {
        x2 -= (x2 - x1).signum();
        dx = dz + 1;
    } else if dz == dx + 2 && dx != 0 {
        z2 -= (z2 - z1).signum();
        dz = dx + 1;
    }

    if straight_only {
        // 2. Straight only.
        if dz < dx {
            z2 = z1;
            dz = 0;
        } else {
            x2 = x1;
            dx = 0;
        }
    } else if snap {
        // 3. Snap.
        if 2 * dz < dx {
            z2 = z1;
            dz = 0;
        } else if 2 * dx < dz {
            x2 = x1;
            dx = 0;
        } else {
            if dx > dz {
                dx = if w1 == 1 { dz + 1 } else { dz };
            } else if dz > dx {
                dz = if w1 == 1 { dx + 1 } else { dx };
            }
            // Each sign points from the start to the original end.
            x2 = x1 + (ox2 - x1).signum() * dx;
            z2 = z1 + (oz2 - z1).signum() * dz;
        }
    }

    // 4. Counts.
    let (dragged_count, axis) = if dx > dz { (dx + 1, (0, 1)) } else { (dz + 1, (1, 0)) };
    // 5, 6. The directions use the end as passed in, and the adjusted deltas.
    let pen_direction = if dz < dx {
        if ox2 <= x1 {
            0
        } else {
            4
        }
    } else if z1 < oz2 {
        6
    } else {
        2
    };
    let end_direction = if ox2 <= x1 {
        if z1 < oz2 {
            7
        } else {
            1
        }
    } else if z1 < oz2 {
        5
    } else {
        3
    };
    // 7. Straight count.
    let straight_count = if dx == 0 {
        dz + 1
    } else if dz == 0 {
        dx + 1
    } else {
        let d = (dx - dz).abs();
        if w2 == 1 && d == 1 && dragged_count > 1 {
            0
        } else {
            d
        }
    };
    Segments {
        end: (x2, z2),
        dragged_count: dragged_count as u32,
        straight_count: straight_count as u32,
        axis,
        pen_direction,
        end_direction,
    }
}

/// The cells of one step at pen position `(x, z)`: the row across the pen
/// axis, `width` cells. [choice] A wider network is widened towards +x / +z
/// across the pen axis (§3.3 step 2 does not say to which side; Q NG9), and
/// the alignment with an existing parallel network (the placed-network
/// filter) is not applied (Q NG9).
pub fn step_cells(x: i32, z: i32, pen_direction: u8, width: u32) -> Vec<(i32, i32)> {
    let along_x = pen_direction == 0 || pen_direction == 4;
    (0..width.max(1) as i32)
        .map(|k| if along_x { (x, z + k) } else { (x + k, z) })
        .collect()
}

/// Same-width convenience entry point for the ordered staircase (NT28).
pub fn compute_dragged_cells(
    start: (i32, i32),
    end: (i32, i32),
    _pen_direction: u8,
    width: u32,
) -> (Vec<(i32, i32)>, Vec<Step>) {
    let seg = break_into_segments(start, end, false, false, width, width);
    compute_dragged_rows(start, end, &seg, width, width)
}

/// network-tools §3.5: group every axis-aligned staircase position into
/// major-axis rows, then widen intervals and emit increasing minor values.
pub fn compute_dragged_rows(
    start: (i32, i32),
    end: (i32, i32),
    seg: &Segments,
    w1: u32,
    w2: u32,
) -> (Vec<(i32, i32)>, Vec<Step>) {
    let along_x = seg.axis.0 == 0;
    let mut rows: Vec<(i32, i32, i32)> = Vec::new();
    let (mut x, mut z) = start;
    let mut previous_x = true;
    loop {
        let (major, minor) = if along_x { (x, z) } else { (z, x) };
        if let Some(row) = rows.last_mut().filter(|r| r.0 == major) {
            row.1 = row.1.min(minor);
            row.2 = row.2.max(minor);
        } else {
            rows.push((major, minor, minor));
        }
        if (x, z) == end {
            break;
        }
        let (rx, rz) = (end.0 - x, end.1 - z);
        let move_x = rx.abs() > rz.abs() || (rx.abs() == rz.abs() && !previous_x);
        if move_x {
            x += rx.signum();
        } else {
            z += rz.signum();
        }
        previous_x = move_x;
    }
    let negative = if along_x {
        matches!(seg.end_direction, 1 | 3)
    } else {
        matches!(seg.end_direction, 1 | 7)
    };
    let mut cells = Vec::new();
    let mut steps = Vec::new();
    let count = rows.len();
    for (i, (major, mut lo, mut hi)) in rows.into_iter().enumerate() {
        let straight = i < seg.straight_count as usize;
        let first_diag = i == seg.straight_count as usize;
        let widen = straight || ((!first_diag || seg.straight_count != 0 || lo == hi) && (i + 1 < count || lo == hi));
        if widen {
            let extra = if straight { w1 } else { w2 }.max(1) as i32 - 1;
            if negative {
                lo -= extra;
            } else {
                hi += extra;
            }
        }
        let first = cells.len() as u32;
        cells.extend((lo..=hi).map(|minor| if along_x { (major, minor) } else { (minor, major) }));
        steps.push(Step {
            first,
            last: cells.len() as u32 - 1,
            kind: StepKind::Regular,
        });
    }
    (cells, steps)
}
