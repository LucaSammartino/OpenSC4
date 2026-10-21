// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The pieces and their pillars (§11): `InsertBridgePieces` (§11.1),
//! `InsertBridgePiece` (§11.2) and the support columns (§11.3).

use std::collections::BTreeMap;

use crate::network_geometry::smoothing::{cell_key, CellInfo};
use crate::x87::r24;

use super::model::{PieceKind, PieceRecord};
use super::types::{BridgeType, Footprint};
use super::{BridgeRecord, BridgeTool};

/// The bridge occupant class (§11.2 step 1).
pub const BRIDGE_OCCUPANT_CLASS: u32 = 0x49CC_1BCD;

/// The cell size (§11.2 step 1), the tool's `+0x238`.
pub const CELL_SIZE: f32 = 16.0;

/// One support column (§11.3 step 5): four corners from `bottom` up to `top`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PillarColumn {
    /// The four (x, z) corners of the footprint, around the occupant's
    /// position.
    pub corners: [Footprint; 4],
    /// The ground the column starts at.
    pub bottom: f32,
    /// The column's top.
    pub top: f32,
    /// True for the part of a column split at the water level (§11.3 step 6).
    pub underwater: bool,
}

/// The occupant position of §11.2 step 2: x and z are the means of the four
/// corner points' x and z (summed in corner order, then × 0.25), and the
/// **height is the highest of the four corner heights** (a corner replaces
/// the running value, which starts at −FLT_MAX, only when strictly greater).
///
/// [compat] The height is not the model height `Y` (NB3).
pub fn occupant_position(corners: &[f32; 4], x: i32, z: i32, cell: f32) -> (f32, f32, f32) {
    // `(x·s, c0, z·s)`, `(x·s, c1, z·s + s)`, `(x·s + s, c2, z·s + s)`,
    // `(x·s + s, c3, z·s)`.
    let (xs, zs) = (f64::from(x) * f64::from(cell), f64::from(z) * f64::from(cell));
    let s = f64::from(cell);
    let pts = [(xs, zs), (xs, zs + s), (xs + s, zs + s), (xs + s, zs)];
    let sx: f64 = pts.iter().map(|p| p.0).sum();
    let sz: f64 = pts.iter().map(|p| p.1).sum();
    let mut h = -f32::MAX;
    for &c in corners {
        if c > h {
            h = c;
        }
    }
    (r24(sx * 0.25) as f32, r24(sz * 0.25) as f32, h)
}

/// §11.2 step 2's model offset: `dy := r24(Y − height)`, so the model's
/// origin is at `Y`.
pub fn model_offset(y: f32, height: f32) -> f32 {
    r24(f64::from(y) - f64::from(height)) as f32
}

/// §11.3: the support column of a piece, or none.
///
/// 1. Nothing when the depth is < 0.
/// 2. `top := r24(height − depth)`, where `height` is the occupant's position
///    height (the **highest corner** of the piece, not its model height `Y`).
/// 3. `bottom :=` the lowest of the terrain altitudes at the cell info's four
///    vertex ids.
/// 4. If `top < bottom`, the model already reaches the ground: no column.
/// 5. The column's four corners are the footprint `+0x8C` for a pier piece
///    (kind 1), else `+0x80`, in the variant for the piece's rotation, around
///    the occupant's position, from `bottom` up to `top`.
/// 6. If the mean of the four bottom heights (× 0.25) is below the water
///    level, the column is split at `water − 0.01` into an underwater part
///    and the part above.
pub fn pillar_columns(
    rec: &PieceRecord,
    bridge: &BridgeType,
    height: f32,
    vertex_altitudes: [f32; 4],
    water_level: f32,
) -> Vec<PillarColumn> {
    if rec.depth < 0.0 {
        return Vec::new();
    }
    let top = r24(f64::from(height) - f64::from(rec.depth)) as f32;
    let bottom = vertex_altitudes.iter().copied().fold(f32::MAX, f32::min);
    if top < bottom {
        return Vec::new();
    }
    let variants = if rec.kind == PieceKind::Pier {
        bridge.pier_footprint
    } else {
        bridge.ramp_footprint
    };
    let corners = variants[(rec.rotation & 3) as usize];
    let mean = vertex_altitudes.iter().map(|&v| f64::from(v)).sum::<f64>() * 0.25;
    if mean < f64::from(water_level) {
        let split = r24(f64::from(water_level) - 0.01) as f32;
        vec![
            PillarColumn {
                corners,
                bottom,
                top: split,
                underwater: true,
            },
            PillarColumn {
                corners,
                bottom: split,
                top,
                underwater: false,
            },
        ]
    } else {
        vec![PillarColumn {
            corners,
            bottom,
            top,
            underwater: false,
        }]
    }
}

/// §11.2 step 3's occupant flags: clear `0x2000000` and `0x4000000`, set
/// `0x8000000`. If the occupant is usable (`0x10000000`), clear `0x4000`,
/// else set it (network-manager.md §5).
pub fn occupant_flags(existing: u32, usable: bool) -> u32 {
    let mut f = existing & !0x0200_0000 & !0x0400_0000;
    f |= 0x0800_0000;
    if usable {
        f &= !0x0000_4000;
    } else {
        f |= 0x0000_4000;
    }
    f
}

/// §11.2 step 4: the model bound height, `occupant slot 68 :=` the lowest
/// corner of the cell (`0x0062DB90`) `− +0xD0`, when the piece's exemplar
/// lacks property `0xABE1B5A3`.
pub fn model_bound(corners: &[f32; 4], model_bound_offset: f32) -> f32 {
    let lowest = corners.iter().copied().fold(f32::MAX, f32::min);
    r24(f64::from(lowest) - f64::from(model_bound_offset)) as f32
}

/// What `InsertBridgePiece` hands to the occupant manager.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InsertedPiece {
    pub x: i32,
    pub z: i32,
    pub instance: u32,
    pub rotation: u8,
    pub corners: [f32; 4],
    /// The occupant position (§11.2 step 2).
    pub position: (f32, f32, f32),
    /// The model offset `(0, dy, 0)`.
    pub dy: f32,
    pub flags: u32,
    /// §11.2 step 4's occupant slot 68.
    pub model_bound: f32,
    /// The support columns (§11.3).
    pub columns: Vec<PillarColumn>,
}

/// `InsertBridgePiece` (`0x0062EEB0`, §11.2) for one piece record.
///
/// `vertex_altitudes` are the terrain altitudes (slot 12) at the cell info's
/// four vertex ids; `water_level` is terrain slot 30.
pub fn insert_bridge_piece(
    rec: &PieceRecord,
    bridge: &BridgeType,
    ci: &CellInfo,
    vertex_altitudes: [f32; 4],
    water_level: f32,
    usable: bool,
) -> InsertedPiece {
    let (px, pz, height) = occupant_position(&rec.corners, rec.x, rec.z, CELL_SIZE);
    let dy = model_offset(rec.y, height);
    let columns = pillar_columns(rec, bridge, height, vertex_altitudes, water_level);
    InsertedPiece {
        x: rec.x,
        z: rec.z,
        instance: rec.instance,
        rotation: rec.rotation,
        corners: rec.corners,
        position: (px, pz, height),
        dy,
        flags: occupant_flags(0, usable),
        model_bound: model_bound(&ci.corners, bridge.model_bound_offset),
        columns,
    }
}

/// `InsertBridgePieces` (`0x0062F610`, §11.1).
///
/// For each bridge record in order, stopping at the first one whose `end ≥
/// valid`: for each piece record, look up the cell's info (key
/// `(z << 16) + x`; it must exist), set its byte `+0x51 := 1` (a bridge
/// cell), and insert the piece.
///
/// [quirk 8] The walk stops at the first record past `valid` instead of
/// skipping it.
pub fn insert_bridge_pieces(
    tool: &BridgeTool,
    valid: u32,
    infos: &mut BTreeMap<i32, CellInfo>,
    mut insert: impl FnMut(&PieceRecord, &BridgeType, &CellInfo),
) -> u32 {
    let mut n = 0u32;
    for r in &tool.records {
        if r.end >= valid {
            break;
        }
        let Some(bt) = tool.type_of_record(r) else {
            continue;
        };
        for p in &r.pieces {
            let Some(ci) = infos.get_mut(&cell_key(p.x, p.z)) else {
                continue;
            };
            // `+0x51 := 1`, a bridge cell.
            ci.placed = true;
            let snapshot = ci.clone();
            insert(p, bt, &snapshot);
            n += 1;
        }
    }
    n
}

/// The pieces of a record as `InsertedPiece`s, in order (the conformance
/// vectors' view of §11).
pub fn insert_all(
    record: &BridgeRecord,
    bridge: &BridgeType,
    infos: &dyn Fn(i32, i32) -> Option<CellInfo>,
    vertex_altitudes: &dyn Fn(i32, i32) -> [f32; 4],
    water_level: &dyn Fn(i32, i32) -> f32,
    usable: bool,
) -> Vec<InsertedPiece> {
    record
        .pieces
        .iter()
        .map(|p| {
            let ci = infos(p.x, p.z).unwrap_or_else(|| CellInfo {
                x: p.x,
                z: p.z,
                corners: p.corners,
                ..CellInfo::default()
            });
            insert_bridge_piece(
                p,
                bridge,
                &ci,
                vertex_altitudes(p.x, p.z),
                water_level(p.x, p.z),
                usable,
            )
        })
        .collect()
}
