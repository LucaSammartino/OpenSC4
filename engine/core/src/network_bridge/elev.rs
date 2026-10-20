// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The elevations (§8): `SelectBridgeElevation` (§8.2) and the bridge's four
//! places in the height solve (§8.3), which answers NB1.

use std::collections::BTreeMap;

use crate::network_geometry::smoothing::{cell_key, CellInfo};
use crate::network_geometry::Run;
use crate::x87::r24;

use super::span::SpanWorld;
use super::types::BridgeType;
use super::{step_cells_of, BridgeRecord, BridgeTool};

/// §8.3.3's constraint entry: `{start, end, e, e}`, where `e` is the record's
/// deck elevation minus the bridge type's total ramp rise.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BridgeConstraintEntry {
    pub start: u32,
    pub end: u32,
    /// `e := r24(deck − +0x60[0])` (§8.3.3).
    pub height: f32,
}

impl BridgeConstraintEntry {
    /// `{start, end, e, e}` as the level's constraint list holds it.
    pub fn as_tuple(&self) -> (u32, u32, f32, f32) {
        (self.start, self.end, self.height, self.height)
    }
}

/// `SelectBridgeElevation` (`0x0062EA90`, §8.2): the deck elevation `+0x10`.
///
/// The record's `+0x14`, the reference elevation, is already the bank height
/// §5.2 chose and is not touched here.
pub fn select_bridge_elevation(
    w: &SpanWorld<'_>,
    record: &mut BridgeRecord,
    bridge: &BridgeType,
    altitude_override: f32,
) {
    // 1. `+0x10 :=` the override `+0x204` (−1.0 when none).
    record.deck = altitude_override;
    // 2. For each step `k` from `start` to `end − 1` (the end step is
    // excluded): per cell `surface(c) + +0x5C` when the cell has water, else
    // `surface(c) + +0x78`. Only the **last** cell's value is kept (quirk 5).
    for k in record.start..record.end {
        let cells = step_cells_of(w.run, k);
        let Some(&(x, z)) = cells.last() else {
            continue;
        };
        let surface = w.terrain.surface_level(x, z);
        let v = if w.terrain.cell_has_water(x, z) {
            r24(f64::from(surface) + f64::from(bridge.deck_height_water)) as f32
        } else {
            r24(f64::from(surface) + f64::from(bridge.total_ramp_rise)) as f32
        };
        if v > record.deck {
            record.deck = v;
        }
    }
}

/// §8.3.1 `RemoveBridgePiecesFromSetOfCellsToBeSolved` (`0x00638B40`), the
/// last step of `BuildSetOfCellsToBeSolved`: for each bridge record in order,
/// stopping at the first whose end step is ≥ `valid` (as §11.1), the cell key
/// `(z << 16) + x` of each of its piece records is erased from the solve set
/// `+0x84`. The rule solver never chooses a piece for a bridge cell.
pub fn remove_bridge_pieces_from_solve_set(tool: &BridgeTool, valid: u32, solve_set: &mut dyn FnMut(i32, bool)) {
    // The callback is `(cell key, erase)`: §8.3.1 erases every listed key.
    for r in &tool.records {
        if r.end >= valid {
            break;
        }
        for p in &r.pieces {
            solve_set(cell_key(p.x, p.z), false);
        }
    }
}

/// The cell keys §8.3.1 erases, as a list.
pub fn bridge_solve_set_removals(tool: &BridgeTool, valid: u32) -> Vec<i32> {
    let mut out = Vec::new();
    remove_bridge_pieces_from_solve_set(tool, valid, &mut |k, _erase| out.push(k));
    out
}

/// §8.3.3 `CopyBridgeElevationConstraintsIntoCellInfo` (`0x0062FB90`), which
/// runs at **every** smoothing level (network-geometry.md §9.4 step 2).
///
/// For each bridge record in order, stopping at the first whose end step is
/// ≥ `valid`:
///
/// - it appends the entry `{start, end, e, e}` to the level's constraint
///   list, with `e := r24(deck − +0x60[0])`;
/// - it walks the steps `start … end`, and each step's cells first to last,
///   with one counter over the record's piece records starting at 0. Each
///   cell info gets the piece's four corner heights as its corner heights
///   `+0x58[i]`, index for index, and its allowed range `+0x68[i]` :=
///   `[corner, corner]`. The preferred range is not touched.
///
/// Returns the constraint entries, in order. `infos` is the mutable cell
/// info map of the smoothing level.
pub fn bridge_elevation_constraints(
    tool: &BridgeTool,
    run: &Run,
    valid: u32,
    infos: &mut BTreeMap<i32, CellInfo>,
) -> Vec<BridgeConstraintEntry> {
    let mut out = Vec::new();
    for r in &tool.records {
        if r.end >= valid {
            break;
        }
        let Some(bt) = tool.type_of_record(r) else {
            continue;
        };
        // `e := r24(deck − +0x60[0])`: the deck elevation minus the bridge
        // type's total ramp rise. For a parametric bridge that is the height
        // of the abutments' **outer** edges (§10.3); a specialised bridge's
        // outer edge is `h[0] + deck` (§9.3), which `e` ignores.
        let e = r24(f64::from(r.deck) - f64::from(bt.total_ramp_rise)) as f32;
        out.push(BridgeConstraintEntry {
            start: r.start,
            end: r.end,
            height: e,
        });
        // The piece records are in lane order (§9.1), which runs last to
        // first when `R` is 2 or 3, so on a bridge more than one cell wide
        // built with pen 0 or 6 the records of a step land on its cells in
        // reverse (quirk 10). Both models give every lane of a step the same
        // corner heights, so this has no effect.
        let mut n = 0usize;
        for k in r.start..=r.end {
            for &(x, z) in step_cells_of(run, k) {
                let Some(p) = r.pieces.get(n) else {
                    continue;
                };
                if let Some(ci) = infos.get_mut(&cell_key(x, z)) {
                    pin_cell_to_piece_corners(ci, &p.corners);
                }
                n += 1;
            }
        }
    }
    out
}

/// The per-cell half of §8.3.3: one cell info takes a piece's four corner
/// heights as its corner heights `+0x58[i]`, index for index, and its
/// allowed range `+0x68[i]` becomes `[corner, corner]`. The preferred range
/// is not touched.
pub fn pin_cell_to_piece_corners(ci: &mut CellInfo, corners: &[f32; 4]) {
    for (i, &c) in corners.iter().enumerate() {
        ci.corners[i] = c;
        ci.allowed[i] = (c, c);
    }
    ci.fixed = true;
}
