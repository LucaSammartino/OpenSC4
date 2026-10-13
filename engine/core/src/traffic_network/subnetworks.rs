// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Subnetworks (§4.11, `UpdateSubnetworks`): connected components of the car
//! graph, labelled per (cell, entry side), and the subnetwork info (§4.12,
//! `UpdateSubnetworkInfo`): per-label population and destination capacity.

use std::collections::VecDeque;

use super::model::{NetworkModel, NetworkState, SubnetworkInfo};
use super::{net, side, travel, wormhole, NetworkHost};

/// One state of the explicit `Fill` stack (§4.11): (cell, entry e, next k).
#[derive(Clone, Copy, Debug)]
struct FillState {
    x: i32,
    z: i32,
    e: usize,
    k: usize,
}

impl NetworkModel {
    /// `UpdateSubnetworks` clear step (§4.11): the subnetwork counter := 1 and
    /// every record's `subnet[0…3]` := 0.
    pub fn clear_subnetworks(&self) {
        self.with(|s| {
            s.subnet_counter = 1;
            for list in &mut s.info {
                for r in list {
                    r.subnet = [0; 4];
                }
            }
        });
    }

    /// `UpdateSubnetworks` (§4.11): the forward and backward seed passes over
    /// the car graph. Runs in one call (work-cycle state 12).
    pub fn update_subnetworks(&self) {
        self.with(|s| {
            // Forward pass: x = 0 … X−2 (outer), z = 0 … Z−2 (inner), entry
            // sides 0 and 1.
            for x in 0..(s.count_x - 1) {
                for z in 0..(s.count_z - 1) {
                    if car_record(s, x, z) {
                        if subnet_of(s, x, z, 0) == 0 {
                            fill(s, x, z, 0);
                        }
                        if subnet_of(s, x, z, 1) == 0 {
                            fill(s, x, z, 1);
                        }
                    }
                }
            }
            // Backward pass: x = X−2 … 0, z = Z−2 … 0, entry sides 2 and 3.
            for x in (0..(s.count_x - 1)).rev() {
                for z in (0..(s.count_z - 1)).rev() {
                    if car_record(s, x, z) {
                        if subnet_of(s, x, z, 2) == 0 {
                            fill(s, x, z, 2);
                        }
                        if subnet_of(s, x, z, 3) == 0 {
                            fill(s, x, z, 3);
                        }
                    }
                }
            }
            s.subnetworks_stale = false;
        });
    }

    /// `UpdateSubnetworkInfo` (§4.12): builds the working subnetwork-info
    /// vector from the lots' ring cells and the neighbour connections, then
    /// publishes it. The work is time-sliced in the original; slicing changes
    /// only when the result is published, not the result, so FreeCity4 runs it
    /// whole *[choice]*.
    pub fn update_subnetwork_info(&self) {
        let h = self.host.clone();
        self.with(|s| {
            // Step 1: clear the working vector and resize to `counter`.
            s.subnet_working = vec![SubnetworkInfo::default(); s.subnet_counter as usize];
            // Step 2: for each cell with a lot (anchor only).
            for z in 0..s.count_z {
                for x in 0..s.count_x {
                    let Some(lot) = h.lot_at(x, z, true) else { continue };
                    accumulate_lot(s, &*h, lot, x, z);
                }
            }
            // The neighbour connections.
            accumulate_neighbours(s, &*h);
            // Publish.
            s.subnet_published = s.subnet_working.clone();
        });
    }
}

/// Whether the cell has a car record (§4.11 seed condition).
fn car_record(s: &NetworkState, x: i32, z: i32) -> bool {
    s.car_info(x, z).is_some()
}

/// The `subnet[e]` of the cell's first car record (0 without one).
fn subnet_of(s: &NetworkState, x: i32, z: i32, e: usize) -> u16 {
    s.car_info(x, z).map_or(0, |r| r.subnet[e])
}

/// Sets `subnet[e]` of the cell's first car record.
fn set_subnet(s: &mut NetworkState, x: i32, z: i32, e: usize, label: u16) {
    if let Some(r) = s.network_info_mut(net::CAR, x, z) {
        r.subnet[e] = label;
    }
}

/// `Fill(x, z, e)` (§4.11): labels the (cell, entry side) states reachable
/// along directed car edges, using travel type Car and an explicit stack.
fn fill(s: &mut NetworkState, x0: i32, z0: i32, e0: usize) {
    // Step 1: start only if the cell has a car record with subnet[e] = 0 and
    // connCount ≤ 2, and is not a tunnel cell.
    let Some(info) = s.car_info(x0, z0) else { return };
    if info.subnet[e0] != 0 || info.conn_count > 2 {
        return;
    }
    if worm(s, x0, z0) & wormhole::TUNNEL != 0 {
        return;
    }
    let counter = s.subnet_counter;
    let mut labelled_any = false;
    // The explicit stack (a deque, allocator B, internal).
    let mut stack: VecDeque<FillState> = VecDeque::new();
    let mut cur = FillState { x: x0, z: z0, e: e0, k: 0 };
    loop {
        // Step 2: for k = 0…3, if conn[1] has bit k + 4e.
        let mut advanced = false;
        while cur.k < side::COUNT {
            let k = cur.k;
            let has = s
                .conn_at(cur.x, cur.z)
                .is_some_and(|c| c.conn[travel::CAR] & (1 << (k + 4 * cur.e)) != 0);
            if !has {
                cur.k += 1;
                continue;
            }
            // Label this cell's entry side.
            if subnet_of(s, cur.x, cur.z, cur.e) == 0 {
                set_subnet(s, cur.x, cur.z, cur.e, counter);
                labelled_any = true;
            }
            // Find the next state.
            match next_state(s, cur, k) {
                Some(next) => {
                    // Push the current state with k + 1 and move to next (k = 0).
                    stack.push_back(FillState { k: k + 1, ..cur });
                    cur = next;
                    advanced = true;
                    break;
                }
                None => {
                    cur.k += 1;
                }
            }
        }
        if advanced {
            continue;
        }
        // Step 3: after k = 3, a tunnel cell whose record allows Car.
        if worm(s, cur.x, cur.z) & wormhole::TUNNEL != 0 {
            if let Some(other) = tunnel_car_other(s, cur.x, cur.z) {
                if subnet_of(s, cur.x, cur.z, cur.e) == 0 {
                    set_subnet(s, cur.x, cur.z, cur.e, counter);
                    labelled_any = true;
                }
                // The other portal's car record with subnet[e] = 0 (the same
                // entry side): push and continue there with entry e.
                if s.car_info(other.0, other.1).is_some() && subnet_of(s, other.0, other.1, cur.e) == 0 {
                    stack.push_back(cur);
                    cur = FillState { x: other.0, z: other.1, e: cur.e, k: 0 };
                    continue;
                }
            }
        }
        // Step 4: pop and resume.
        match stack.pop_back() {
            Some(next) => cur = next,
            None => break,
        }
    }
    // When the stack is empty, counter += 1 if anything was labelled.
    if labelled_any {
        s.subnet_counter = counter + 1;
    }
}

/// The wormhole byte at a cell.
fn worm(s: &NetworkState, x: i32, z: i32) -> u8 {
    s.conn_at(x, z).map_or(0, |c| c.wormhole)
}

/// The next `Fill` state from (cur, k) (§4.11 step 2), or `None`.
fn next_state(s: &NetworkState, cur: FillState, k: usize) -> Option<FillState> {
    let (nx, nz) = (cur.x + super::DX[k], cur.z + super::DZ[k]);
    // Off-map neighbour: if the cell also has bit e + 4k (the connection from
    // k back to e) and subnet[k] = 0, continue in the same cell with entry k.
    if !s.in_city(nx, nz) {
        let back = s
            .conn_at(cur.x, cur.z)
            .is_some_and(|c| c.conn[travel::CAR] & (1 << (cur.e + 4 * k)) != 0);
        if back && subnet_of(s, cur.x, cur.z, k) == 0 {
            return Some(FillState { x: cur.x, z: cur.z, e: k, k: 0 });
        }
        return None;
    }
    // In-city neighbour N with a car record, where subnet[(k+2) mod 4] = 0:
    // continue at N with entry (k + 2) mod 4.
    let back_e = super::OPPOSITE_SIDE[k];
    if s.car_info(nx, nz).is_some() {
        if subnet_of(s, nx, nz, back_e) == 0 {
            return Some(FillState { x: nx, z: nz, e: back_e, k: 0 });
        }
        return None;
    }
    // N without a car record but with switch ID s ≠ 0: step further in
    // direction k while the cells stay in the city and carry ID s. At the
    // first cell with another ID: if it has a car record with
    // subnet[(k+2) mod 4] = 0, continue there.
    let sid = s.switch_id_at(nx, nz);
    if sid == 0 {
        return None;
    }
    let (mut cx, mut cz) = (nx, nz);
    loop {
        let (fx, fz) = (cx + super::DX[k], cz + super::DZ[k]);
        if !s.in_city(fx, fz) || s.switch_id_at(fx, fz) != sid {
            // The first cell with another ID.
            if s.car_info(fx, fz).is_some() && subnet_of(s, fx, fz, back_e) == 0 {
                return Some(FillState { x: fx, z: fz, e: back_e, k: 0 });
            }
            return None;
        }
        cx = fx;
        cz = fz;
    }
}

/// The other portal of a tunnel cell when its tunnel record allows Car
/// (§4.11 step 3), as a cell.
fn tunnel_car_other(s: &NetworkState, x: i32, z: i32) -> Option<(i32, i32)> {
    let key = u32::from(super::cell_key(x, z));
    let t = s.tunnels.get(key)?;
    // The mask always has Car (0x1FE has bit 1).
    if t.mask & (1 << travel::CAR) == 0 {
        return None;
    }
    Some(t.other)
}

/// §4.12 step 4: accumulates one lot's populations and capacities into the
/// working subnetwork info for the labels on its ring.
fn accumulate_lot(s: &mut NetworkState, h: &dyn NetworkHost, lot: u64, x: i32, z: i32) {
    let _ = (x, z);
    let r = h.lot_bounding_rect(lot);
    // Ring = the boundary of r grown by 1 and clamped to the city, skipping
    // the four corners (cells whose x is outside [r.x1, r.x2] AND whose z is
    // outside [r.z1, r.z2]).
    let ring = super::model::grow_clamp(r, 1, s.count_x, s.count_z);
    let ids = ring_labels(s, r, ring);
    let holder = h.lot_holder(lot);
    if ids.is_empty() {
        // Step 3: remove the property.
        if holder != 0 {
            h.remove_holder_property(holder, super::prop::LOT_SUBNETWORKS);
        }
        return;
    }
    // Step 4: set the property.
    if holder != 0 {
        let items: Vec<crate::variant::Elem> = ids.iter().map(|&id| crate::variant::Elem::U32(id)).collect();
        h.set_holder_property(
            holder,
            super::prop::LOT_SUBNETWORKS,
            crate::variant::Variant::array(crate::variant::mem::UINT32, items),
        );
    }
    // Populations: for every (type, count) in every map of lot.GetPopulations().
    for map in h.lot_populations(lot) {
        for (dev_type, count) in map {
            for &id in &ids {
                if let Some(info) = s.subnet_working.get_mut(id as usize) {
                    *info.population.entry(dev_type).or_insert(0) += count;
                }
            }
        }
    }
    // Capacities: for k = 0…3, info[id].capacity[k] += GetMaxTripCapacity.
    let caps = max_trip_capacities(h, holder);
    for &id in &ids {
        if let Some(info) = s.subnet_working.get_mut(id as usize) {
            for k in 0..4 {
                info.capacity[k] += caps[k];
            }
        }
    }
}

/// The distinct non-zero `subnet[0…3]` of the first car record of each ring
/// cell (corners excluded), in order of discovery (§4.12 step 2).
fn ring_labels(s: &NetworkState, lot: crate::grid::CellRect, ring: crate::grid::CellRect) -> Vec<u32> {
    let mut ids: Vec<u32> = Vec::new();
    for x in ring.x1..=ring.x2 {
        for z in ring.z1..=ring.z2 {
            // Only the boundary of the grown rectangle.
            let on_boundary = x == ring.x1 || x == ring.x2 || z == ring.z1 || z == ring.z2;
            if !on_boundary {
                continue;
            }
            // Skip the four corners: x outside [lot.x1, lot.x2] AND z outside
            // [lot.z1, lot.z2].
            let x_out = x < lot.x1 || x > lot.x2;
            let z_out = z < lot.z1 || z > lot.z2;
            if x_out && z_out {
                continue;
            }
            if !s.in_city(x, z) {
                continue;
            }
            if let Some(r) = s.car_info(x, z) {
                for &id in &r.subnet {
                    if id != 0 && !ids.contains(&u32::from(id)) {
                        ids.push(u32::from(id));
                    }
                }
            }
        }
    }
    ids
}

/// `GetMaxTripCapacity(holder, k)` for k = 0…3 from property `0x8A0B4100`
/// (traffic-simulator.md §4.2): element k of the 12-u16 array, 0 if missing.
fn max_trip_capacities(h: &dyn NetworkHost, holder: u64) -> [u32; 4] {
    let mut caps = [0u32; 4];
    if holder == 0 {
        return caps;
    }
    if let Some(v) = h.holder_property(holder, super::prop::TRIP_CAPACITY) {
        let items = v.items();
        for k in 0..4 {
            caps[k] = items.get(k).map_or(0, |e| e.as_i64() as u32);
        }
    }
    caps
}

/// §4.12: after the last cell, the neighbour connections are added.
fn accumulate_neighbours(s: &mut NetworkState, h: &dyn NetworkHost) {
    let conns = h.all_connections();
    for conn in conns {
        let (x, z) = h.connection_cell(conn);
        // Map (x, z) to the in-city cell: x = −1 → 0; x = X → X−1; else
        // z = −1 → 0; z = Z → Z−1.
        let ix = if x < 0 { 0 } else if x >= s.count_x { s.count_x - 1 } else { x };
        let iz = if z < 0 { 0 } else if z >= s.count_z { s.count_z - 1 } else { z };
        if s.car_info(ix, iz).is_none() {
            continue;
        }
        // GetNeighborPoolPropertyHolder(1, x, z); FreeCity4 exposes it through
        // the host's neighbour pool lookup, folded into connection_cell's
        // owner. Without a holder, skip.
        let caps = neighbour_pool_capacities(h, ix, iz);
        let Some(caps) = caps else { continue };
        // Add c to info[subnet[e]].capacity for each e = 0…3 with subnet[e] ≠ 0.
        // Not deduplicated: a label shared by two entry sides counts twice
        // *[quirk]*.
        let Some(info) = s.car_info(ix, iz).copied() else { continue };
        for e in 0..4 {
            let id = info.subnet[e];
            if id == 0 {
                continue;
            }
            if let Some(slot) = s.subnet_working.get_mut(id as usize) {
                for k in 0..4 {
                    slot.capacity[k] += caps[k];
                }
            }
        }
    }
}

/// The neighbour pool's `GetMaxTripCapacity` for k = 0…3, if there is a pool
/// property holder at the in-city cell (traffic-simulator.md §4.12,
/// `GetNeighborPoolPropertyHolder`). FreeCity4 reads the pool through the
/// host's connection property; `None` without one.
fn neighbour_pool_capacities(h: &dyn NetworkHost, _x: i32, _z: i32) -> Option<[u32; 4]> {
    // The neighbour pool holder is not a network-model concern; the host
    // exposes it through the connection's own property holder. This is a
    // QUESTION for the coordinator (see docs/specs/QUESTIONS.md): the exact
    // host callback for GetNeighborPoolPropertyHolder. Until then, no pool.
    let _ = h;
    None
}
