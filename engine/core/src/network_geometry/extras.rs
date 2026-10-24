// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The small rules of §10, §11 and §13 that §18's tests cover: the
//! occupant flags (§10.1), the support level (§10.2), the cost of a
//! solution (network-tools.md §8.1, §11 NT2), the power-line cost (§11 NT3)
//! and the removal choice of `DestroyOccupantsAndResolve` (§13 NT1).

use std::collections::BTreeSet;

use super::params::TYPE_COUNT;
use crate::x87::{ftol32, r24, rhe};

/// Network flag: a draft (preview) occupant (§10.3).
pub const FLAG_DRAFT: u32 = 0x8000_0000;
/// Network flag: elevated, with support structures (§10.2).
pub const FLAG_SUPPORTS: u32 = 0x4000_0000;
/// The second flag set with supports (§10.2).
pub const FLAG_SUPPORTS_2: u32 = 0x0800_0000;
/// A tunnel occupant is made usable (§10.1).
pub const FLAG_USABLE: u32 = 0x1000_0000;
/// Set on every occupant but a tunnel occupant (§10.1).
pub const FLAG_4000: u32 = 0x4000;
/// The bridge and portal bits (§10.1).
pub const FLAG_BRIDGE_PORTAL: u32 = 0xC0000;
/// The 13 network type bits.
pub const TYPE_BITS: u32 = (1 << TYPE_COUNT) - 1;

/// `CreateNetworkOccupant` (slot 54, §10.1): the new occupant's network
/// flags from the cell info's flags. In draft mode only the bridge and
/// portal bits of the extra bits survive [inferred: the type bits are kept,
/// every other bit is an "extra" bit; Q NG15] and `0x80000000` is set; in
/// real mode `0x80000000` is cleared.
pub fn occupant_flags(cell_flags: u32, real: bool, tunnel_occupant: bool) -> u32 {
    let mut f = if real {
        cell_flags & !FLAG_DRAFT
    } else {
        (cell_flags & (TYPE_BITS | FLAG_BRIDGE_PORTAL)) | FLAG_DRAFT
    };
    if tunnel_occupant {
        f = (f | FLAG_USABLE) & !FLAG_4000;
    } else {
        f |= FLAG_4000;
    }
    f
}

/// The support level of a piece (§10.2): the maximum over the 4 corners of
/// 0 (d ≤ 0.01), 1 (d < threshold) or 2, `d = r24(corner − terrain)`.
pub fn support_level(corners: [f32; 4], terrain: [f32; 4], threshold: f32) -> u32 {
    (0..4)
        .map(|k| {
            let d = r24(f64::from(corners[k]) - f64::from(terrain[k])) as f32;
            if d <= 0.01 {
                0
            } else if d < threshold {
                1
            } else {
                2
            }
        })
        .max()
        .unwrap_or(0)
}

/// The flags §10.2 sets for a support level.
pub fn support_flags(level: u32) -> u32 {
    if level > 0 {
        FLAG_SUPPORTS | FLAG_SUPPORTS_2
    } else {
        0
    }
}

/// One solved cell for the price (network-tools.md §8.1).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CostCell {
    /// Part of the run (cell record `+0x10`).
    pub in_run: bool,
    /// Intersection marker (`+0x52 ≠ 0`); distinct from placed state (NT29).
    pub prebuilt: bool,
    /// The cell's network type bits.
    pub type_mask: u32,
    pub corners: [f32; 4],
    /// The terrain altitudes at the corners (terrain slot 12).
    pub terrain: [f32; 4],
}

/// A bridge for the price: `(end − start) × (per_cell + +0x320) + fixed`
/// (bridge record `+0x154`, `+0x150`). [inferred] Integers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CostBridge {
    pub start: u32,
    pub end: u32,
    pub per_cell: i32,
    pub fixed: i32,
}

/// The inputs of `GetCostOfSolution` (slot 20, network-tools.md §8.1).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CostInputs {
    /// `+0x220`: the demolition cost.
    pub demolition: i32,
    pub cells: Vec<CostCell>,
    /// The per-type cell cost (p+0x8C).
    pub cell_costs: [i32; TYPE_COUNT],
    /// `+0x328`: earthwork cost per metre.
    pub earthwork_cost: f32,
    /// `+0x320`.
    pub bridge_cost: i32,
    pub bridges: Vec<CostBridge>,
    /// `+0x324`.
    pub tunnel_cost: i32,
    /// (start, end) steps.
    pub tunnels: Vec<(u32, u32)>,
    /// The intersections' rule costs.
    pub intersections: Vec<i32>,
}

/// `GetCostOfSolution` (network-tools.md §8.1).
pub fn cost_of_solution(c: &CostInputs) -> i32 {
    // Earthwork: Σ |corner − altitude| in a float, then the 1.5·2²³ idiom.
    let mut s = 0.0f32;
    for cell in &c.cells {
        for k in 0..4 {
            let d = r24(f64::from(cell.corners[k]) - f64::from(cell.terrain[k])) as f32;
            s = r24(f64::from(s) + f64::from(d.abs())) as f32;
        }
    }
    let earthwork = rhe(r24(f64::from(s) * f64::from(c.earthwork_cost)) as f32);
    let mut total = c.demolition.wrapping_add(earthwork);
    for cell in &c.cells {
        if !cell.in_run || cell.prebuilt {
            continue;
        }
        let best = (0..TYPE_COUNT)
            .filter(|&t| cell.type_mask & (1 << t) != 0)
            .map(|t| c.cell_costs[t])
            .max();
        if let Some(b) = best {
            total = total.wrapping_add(b);
        }
    }
    for b in &c.bridges {
        let len = b.end.wrapping_sub(b.start) as i32;
        total = total.wrapping_add(
            len.wrapping_mul(b.per_cell.wrapping_add(c.bridge_cost))
                .wrapping_add(b.fixed),
        );
    }
    for &(s, e) in &c.tunnels {
        total = total.wrapping_add((e.wrapping_sub(s) as i32).wrapping_mul(c.tunnel_cost));
    }
    for &i in &c.intersections {
        total = total.wrapping_add(i);
    }
    total
}

/// `cSC4PowerLineTool::GetCostOfSolution` (`0x0064F0F0`, §11): 0 for at
/// most one pole; else `trunc((uint32)wire × per_wire) + per_pole × poles`.
/// NT31: the multiply remains in x87 precision until truncation, with no
/// intervening float32 store (network-tools §8.3.1).
pub fn power_line_cost(poles: u32, wire_cells: u32, per_pole: i32, per_wire: f32) -> i32 {
    if poles <= 1 {
        return 0;
    }
    let w = ftol32(f64::from(wire_cells) * f64::from(per_wire));
    w.wrapping_add(per_pole.wrapping_mul(poles as i32))
}

/// The cell key of `DestroyOccupantsAndResolve`: `(x << 16) + z` (§13; the
/// tool's own sets use `(z << 16) + x`, NG7).
pub fn destroy_key(x: i32, z: i32) -> i32 {
    (x << 16).wrapping_add(z)
}

/// How `DestroyOccupantsAndResolve` removes one occupant (§13).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Removal {
    /// The cell is a destroyed cell: `RemoveOccupant(o, 0, 0)`.
    Remove,
    /// A piece pulled in by the closure: `RemoveOrReplaceOccupant`.
    RemoveOrReplace,
}

/// §13: the removal for an occupant in cell `key` given the destroyed
/// cells.
pub fn removal_for(key: i32, destroyed: &BTreeSet<i32>) -> Removal {
    if destroyed.contains(&key) {
        Removal::Remove
    } else {
        Removal::RemoveOrReplace
    }
}

/// §13 / network-tools.md §10 step 5.4: a replaced piece's cell goes into
/// the re-solve set; otherwise the cell is erased from it.
pub fn update_resolve_set(resolve: &mut BTreeSet<i32>, key: i32, replaced: bool) {
    if replaced {
        resolve.insert(key);
    } else {
        resolve.remove(&key);
    }
}
