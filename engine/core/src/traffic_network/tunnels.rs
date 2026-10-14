// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Tunnels (§2.5, §4.8, `DoTunnelChanged`). The tunnel table is keyed by the
//! portal's cell key. Each portal's insert message creates its own record; a
//! tunnel works in both directions once both portals are inserted.

use crate::x87;

use super::model::{key32, Cx, NetworkModel};
use super::{net_type, travel, wormhole, CellRect, TUNNEL_MASK, TUNNEL_TYPE_ORDER};

/// A tunnel record (§2.5).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TunnelInfo {
    /// The other portal's cell (x, z).
    pub other: (i32, i32),
    /// The travel-type mask (always `0x1FE`, §4.8 step 3) *[quirk]*.
    pub mask: u16,
    /// `cost[9]` (f32), §4.8 step 5.
    pub cost: [f32; travel::COUNT],
    /// The network type (§4.8 step 2).
    pub net_type: u8,
}

/// The inverse max-speed table `invMaxSpeed[t][type]` (class static
/// `0x00B4B110`, traffic-simulator.md §4.14). `FLT_MAX` where the speed is
/// ≤ 0 or missing.
pub type InvMaxSpeed = [[f32; net_type::COUNT]; travel::COUNT];

/// Fills the inverse max-speed table from the raw "Max speed by Network"
/// speeds `speeds[t][type]` (§4.8 step 5, traffic-simulator.md §4.14):
/// `invMaxSpeed = r24(1 / r24(r24(v × 16.666666) / CellWidthX))`, or
/// `FLT_MAX` when `v ≤ 0`.
pub fn inverse_max_speed(speeds: &[[f32; net_type::COUNT]; travel::COUNT], cell_width_x: f32) -> InvMaxSpeed {
    let mut out = [[f32::MAX; net_type::COUNT]; travel::COUNT];
    for (row, speeds) in out.iter_mut().zip(speeds) {
        for (slot, &v) in row.iter_mut().zip(speeds) {
            if v > 0.0 {
                let a = x87::r24(f64::from(v) * f64::from(16.666_666_f32)) as f32;
                let b = x87::r24(f64::from(a) / f64::from(cell_width_x)) as f32;
                *slot = x87::r24(1.0 / f64::from(b)) as f32;
            }
        }
    }
    out
}

/// The portal distance (§4.8 step 4):
/// `d = r24(√ r24(r24(r24(dz²) + r24(dy²)) + r24(dx²)))`.
pub fn tunnel_distance(dx: f32, dy: f32, dz: f32) -> f32 {
    let sq = |v: f32| x87::r24(f64::from(v) * f64::from(v));
    let a = x87::r24(sq(dz) + sq(dy));
    let sum = x87::r24(a + sq(dx));
    x87::r24(sum.sqrt()) as f32
}

/// The per-type cost (§4.8 step 5):
/// `cost[t] = r24(r24(d × invMaxSpeed[t][type]) × 0.0625)`. With
/// `invMaxSpeed = FLT_MAX` (no speed on that network) the product overflows
/// to +∞.
pub fn tunnel_costs(d: f32, network_type: u8, inv: &InvMaxSpeed) -> [f32; travel::COUNT] {
    let mut cost = [0.0f32; travel::COUNT];
    for (c, row) in cost.iter_mut().zip(inv) {
        let v = row.get(usize::from(network_type)).copied().unwrap_or(f32::MAX);
        let a = x87::r24(f64::from(d) * f64::from(v));
        *c = x87::r24(a * 0.0625) as f32;
    }
    cost
}

impl Cx<'_> {
    /// The portal's cell (§3: `PositionToCell` of the occupant position).
    fn occupant_cell(&self, occ: u64) -> (i32, i32) {
        let p = self.h.occupant_position(occ);
        self.h.position_to_cell(p[0], p[2])
    }

    /// `DoTunnelChanged` (§4.8). **Remove** erases the portal's key; the
    /// rebuild that follows clears wormhole bit 2. **Insert** computes the
    /// record and stores it under the portal's key, overwriting an existing
    /// record (find-or-insert, then the whole record copied over it: TN11).
    /// Returns whether a record was written.
    pub(crate) fn tunnel_changed(&mut self, occ: u64, removed: bool) -> bool {
        let (x, z) = self.occupant_cell(occ);
        let key = key32(x, z);
        if removed {
            self.s.tunnels.remove(key);
            return false;
        }
        let h = self.h;
        // Step 1: the paired portal and its cell. Without one, return: no
        // record is created or changed (TN11).
        let Some(other) = h.tunnel_paired_portal(occ) else {
            return false;
        };
        let op = h.occupant_position(other);
        let (ox, oz) = h.position_to_cell(op[0], op[2]);
        // Step 2: the first network type of the order the tunnel IsOfType.
        let Some(ty) = TUNNEL_TYPE_ORDER.iter().copied().find(|&k| h.is_of_type(occ, k)) else {
            return false;
        };
        // Step 4: this portal's 3-D position minus the other's.
        let p = h.occupant_position(occ);
        let d = tunnel_distance(p[0] - op[0], p[1] - op[1], p[2] - op[2]);
        // Steps 3 and 5.
        let rec = TunnelInfo {
            other: (ox, oz),
            mask: TUNNEL_MASK,
            cost: tunnel_costs(d, ty as u8, &self.s.inv_max_speed),
            net_type: ty as u8,
        };
        *self.s.tunnels.entry(key, TunnelInfo::default) = rec;
        true
    }

    /// §4.2 step 5: every cell of R1 that is a key of the tunnel table gets
    /// `wormhole |= 2`.
    pub(crate) fn apply_tunnels(&mut self, r1: CellRect) {
        for x in r1.x1..=r1.x2 {
            for z in r1.z1..=r1.z2 {
                if self.s.tunnels.contains(key32(x, z)) {
                    if let Some(c) = self.s.conn_mut(x, z) {
                        c.wormhole |= wormhole::TUNNEL;
                    }
                }
            }
        }
    }
}

impl NetworkModel {
    /// The tunnel record keyed by a portal cell (§2.5), if any.
    pub fn tunnel_at(&self, x: i32, z: i32) -> Option<TunnelInfo> {
        self.s.borrow().tunnels.get(key32(x, z)).copied()
    }
}
