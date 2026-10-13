// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Tunnels (§2.5, §4.8, `DoTunnelChanged`). The tunnel table is keyed by the
//! portal's cell key. Each portal's insert message creates its own record; a
//! tunnel works in both directions once both portals are inserted.

use crate::grid::CellRect;
use crate::x87;

use super::model::{NetworkModel, NetworkState};
use super::{cell_key, travel, NetworkHost, TUNNEL_MASK, TUNNEL_TYPE_ORDER};

/// A tunnel record (§2.5): the other portal's cell, a travel-type mask (always
/// `0x1FE`), the per-type cost and the network type.
#[derive(Clone, Copy, Debug, PartialEq)]
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

impl Default for TunnelInfo {
    fn default() -> Self {
        TunnelInfo {
            other: (0, 0),
            mask: TUNNEL_MASK,
            cost: [0.0; travel::COUNT],
            net_type: 0,
        }
    }
}

/// The inverse max-speed table the tunnel cost reads (§4.8 step 5): a class
/// static filled from the shipped "Max speed by Network" tuning. The traffic
/// simulator owns it; the model holds a copy. `[t][type]`, `FLT_MAX` when the
/// speed is ≤ 0.
pub type InvMaxSpeed = [[f32; super::net_type::COUNT]; travel::COUNT];

/// Computes the inverse max-speed table from the raw speeds (§4.8 step 5,
/// traffic-simulator.md §4.14):
/// `invMaxSpeed = r24(1 / r24(r24(v × 16.666666) / cellWidthX))`, or `FLT_MAX`
/// when `v ≤ 0`.
pub fn inverse_max_speed(speeds: &[[f32; super::net_type::COUNT]; travel::COUNT], cell_width_x: f32) -> InvMaxSpeed {
    let mut out = [[f32::MAX; super::net_type::COUNT]; travel::COUNT];
    for t in 0..travel::COUNT {
        for k in 0..super::net_type::COUNT {
            let v = speeds[t][k];
            if v > 0.0 {
                // r24(v × 16.666666f) — the float constant enters unrounded.
                let a = x87::r24(f64::from(v) * f64::from(16.666666f32)) as f32;
                // r24(a / cellWidthX).
                let b = x87::r24(f64::from(a) / f64::from(cell_width_x)) as f32;
                // r24(1 / b).
                out[t][k] = x87::r24(1.0 / f64::from(b)) as f32;
            }
        }
    }
    out
}

/// The tunnel step distance (§4.8 step 4):
/// `d = r24(√ r24(r24(r24(dz²) + r24(dy²)) + r24(dx²)))`, with (dx, dy, dz)
/// this portal's 3-D position minus the other's, in world units.
pub fn tunnel_distance(dx: f32, dy: f32, dz: f32) -> f32 {
    let dx2 = x87::r24(f64::from(dx) * f64::from(dx)) as f32;
    let dy2 = x87::r24(f64::from(dy) * f64::from(dy)) as f32;
    let dz2 = x87::r24(f64::from(dz) * f64::from(dz)) as f32;
    let sum = x87::r24(f64::from(x87::r24(f64::from(dz2) + f64::from(dy2)) as f32) + f64::from(dx2)) as f32;
    x87::r24(f64::from(sum).sqrt()) as f32
}

/// The tunnel cost per travel type (§4.8 step 5):
/// `cost[t] = r24(r24(d × invMaxSpeed[t][type]) × 0.0625)`. For types with
/// zero speed on that network the cost is +∞ (invMaxSpeed = FLT_MAX and
/// `d × FLT_MAX` overflows; the mask `0x1FE` keeps Walk out anyway).
pub fn tunnel_costs(d: f32, net_type: u8, inv: &InvMaxSpeed) -> [f32; travel::COUNT] {
    let mut cost = [0.0f32; travel::COUNT];
    for t in 0..travel::COUNT {
        let v = inv[t][net_type as usize];
        let a = x87::r24(f64::from(d) * f64::from(v)) as f32;
        cost[t] = x87::r24(f64::from(a) * f64::from(0.0625f32)) as f32;
    }
    cost
}

impl NetworkModel {
    /// `DoTunnelChanged`, remove (§4.8): erases the portal's key. No rebuild
    /// of the tunnel bit happens here; the rebuild that follows (§3 step c)
    /// clears wormhole bit 2.
    pub fn remove_tunnel(&self, x: i32, z: i32) {
        self.with(|s| {
            s.tunnels.remove(u32::from(cell_key(x, z)));
        });
    }

    /// `DoTunnelChanged`, insert (§4.8): reads the paired portal, the network
    /// type and the 3-D positions from the host, computes the cost and stores
    /// the record under this portal's cell key. Returns whether a record was
    /// created (none when the tunnel has no taking-part network type).
    ///
    /// `inv` is the simulator's inverse max-speed table (§4.8 step 5).
    pub fn insert_tunnel(&self, occ: u64, inv: &InvMaxSpeed) -> bool {
        let h = self.host.clone();
        self.with(|s| {
            let Some(other) = h.tunnel_paired_portal(occ) else {
                return false;
            };
            // Step 2: the network type, the first of the order it IsOfType.
            let net_type = TUNNEL_TYPE_ORDER.iter().copied().find(|&k| h.is_of_type(occ, k));
            let Some(net_type) = net_type else {
                return false; // None → no record
            };
            // Step 1: the other portal's cell.
            let op = h.occupant_position(other);
            let (ox, oz) = h.position_to_cell(op[0], op[2]);
            // Step 4: the distance from this portal's 3-D position minus the
            // other's.
            let p = h.occupant_position(occ);
            let (dx, dy, dz) = (p[0] - op[0], p[1] - op[1], p[2] - op[2]);
            let d = tunnel_distance(dx, dy, dz);
            // Step 5: the per-type cost.
            let cost = tunnel_costs(d, net_type as u8, inv);
            let key = u32::from(cell_key_from_position(&h, occ));
            s.tunnels.entry(key, || TunnelInfo {
                other: (ox, oz),
                // Step 3: the mask is always 0x1FE *[quirk]*.
                mask: TUNNEL_MASK,
                cost,
                net_type: net_type as u8,
            });
            true
        })
    }

    /// The tunnel phase of `DoConnectionsChanged` (§4.2 step 5): every cell
    /// of R1 that is a key of the tunnel table gets `wormhole |= 2`.
    pub(super) fn apply_tunnels(&self, s: &mut NetworkState, r1: CellRect) {
        for x in r1.x1..=r1.x2 {
            for z in r1.z1..=r1.z2 {
                let key = u32::from(cell_key(x, z));
                if s.tunnels.contains(key) {
                    if let Some(c) = s.conn_mut(x, z) {
                        c.wormhole |= super::wormhole::TUNNEL;
                    }
                }
            }
        }
    }
}

/// The cell key of an occupant from its position (§3: `PositionToCell`).
fn cell_key_from_position(h: &dyn NetworkHost, occ: u64) -> u16 {
    let p = h.occupant_position(occ);
    let (cx, cz) = h.position_to_cell(p[0], p[2]);
    cell_key(cx, cz)
}
