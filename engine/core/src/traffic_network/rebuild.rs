// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `DoConnectionsChanged` (§4.2): the rebuild of the cell graph over a
//! rectangle, and the lane continuity it maintains (§4.4). This is the network
//! model's core operation: the occupant-insert messages (§3), the transit
//! switch and ferry phases, and the tunnel bits all funnel through it.

use crate::grid::CellRect;

use super::model::{NetworkModel, NetworkState};
use super::switches::set_connection;
use super::tunnels::InvMaxSpeed;
use super::{net, net_type, side, travel, NetworkHost, PATH_TYPE_OF_NET, TAKES_PART, TRANSIT_MASK};

impl NetworkModel {
    /// `DoConnectionsChanged(x1, z1, x2, z2)` (§4.2): rebuilds the graph over
    /// a rectangle. `inv` is the simulator's inverse max-speed table (§4.8
    /// step 5), threaded through for the tunnel phase; the ferry phase needs
    /// no speeds.
    ///
    /// The whole rebuild runs inside one [`NetworkModel::with`] section: it
    /// releases the state `RefCell` borrow for the duration (foundations §19.3
    /// rule 4: no borrow is held across a host callback), so the host calls
    /// below are safe and a re-entrant call sees an empty state.
    pub fn do_connections_changed(&self, rect: CellRect, inv: &InvMaxSpeed) {
        let h = self.host.clone();
        self.with(|s| {
            if s.count_x <= 0 || s.count_z <= 0 {
                return;
            }
            // Step 1: R = the rectangle grown by 2 on every side, clamped to
            // the city. Add R to the intersection dirty rectangle (union) and
            // set the dirty flag (§4.6).
            let r = super::model::grow_clamp(rect, 2, s.count_x, s.count_z);
            if r.x1 > r.x2 || r.z1 > r.z2 {
                return;
            }
            s.intersection_rect = if s.intersection_dirty {
                super::model::union_rect(s.intersection_rect, r)
            } else {
                r
            };
            s.intersection_dirty = true;

            // Step 2: reset every cell of R (delete its network-info records,
            // reset conn/switch/priorCar/wormhole).
            for x in r.x1..=r.x2 {
                for z in r.z1..=r.z2 {
                    if let Some(list) = s.info_mut(x, z) {
                        list.clear();
                    }
                    if let Some(c) = s.conn_mut(x, z) {
                        c.reset();
                    }
                }
            }

            // Step 3: the network occupants of R, in list order.
            let occupants = h.network_occupants_in_rect(r);
            for occ in occupants {
                apply_occupant(s, &*h, occ);
            }

            // Step 4: transit switches over R1 = R grown by 1 more (the
            // original rectangle + 3), clamped. Step 5: tunnels over R1.
            // Step 6: ferries over R1. None of the three phases calls the
            // host, so they run in this same section.
            let r1 = super::model::grow_clamp(rect, 3, s.count_x, s.count_z);
            self.apply_switches(s, r1);
            self.apply_tunnels(s, r1);
            self.apply_ferries(s, r1);

            // Step 7: set the "subnetworks stale" and "network changed" flags.
            s.subnetworks_stale = true;
            s.network_changed = true;
        });
    }
}

/// One network occupant's contribution to a cell (§4.2 step 3).
fn apply_occupant(s: &mut NetworkState, h: &dyn NetworkHost, occ: u64) {
    // Skip an occupant with network flag 0x200000 (excluded from the graph),
    // or with no taking-part network type.
    if h.has_network_flag(occ, super::network_flag::EXCLUDED) {
        return;
    }
    if !takes_part(h, occ) {
        return;
    }
    let p = h.occupant_position(occ);
    let (cx, cz) = h.position_to_cell(p[0], p[2]);
    if !s.in_city(cx, cz) {
        return;
    }
    let has_record_flag = h.has_network_flag(occ, super::network_flag::RECORD_FLAG);

    // For n = 0…5 (in this order), with p = pt(n) and count = 0.
    for n in 0..net::COUNT as u32 {
        let path_type = PATH_TYPE_OF_NET[n as usize];
        let mut count: u32 = 0;
        // For each entry side e = 0…3.
        for e in 0..side::COUNT {
            let mut found_exit = false;
            // For each exit x = 0…3.
            for x in 0..side::COUNT {
                if h.path_connection(occ, path_type, e as u32, x as u32) {
                    found_exit = true;
                    count += 1;
                    // For every travel type t with net(t) = n:
                    // conn[t] |= 1 << (x + 4e).
                    for t in 0..travel::COUNT {
                        if super::NET_OF_TRAVEL[t] == n {
                            if let Some(c) = s.conn_mut(cx, cz) {
                                c.conn[t] |= 1 << (x + 4 * e);
                            }
                            // When t = 1 (Car): LaneContinuity(c, occ, e, x).
                            if t == travel::CAR {
                                lane_continuity(s, h, cx, cz, occ, e, x);
                            }
                        }
                    }
                }
            }
            // If no exit was found for e: ask the path info for a turn-around.
            if !found_exit {
                if let Some(pi) = h.path_info(occ) {
                    if h.path_info_turn_around(pi, path_type, e as u32) {
                        count += 1;
                        // SetConnection(c, e, e, t) for every t on n.
                        for t in 0..travel::COUNT as u16 {
                            if super::NET_OF_TRAVEL[t as usize] == n {
                                set_connection(s, cx, cz, e, e, t);
                            }
                        }
                    }
                }
            }
        }
        // If count ≠ 0 (as u16).
        if count as u16 != 0 {
            if let Some(idx) = s.get_or_create_info(n, cx, cz) {
                if let Some(list) = s.info_mut(cx, cz) {
                    let r = &mut list[idx];
                    // connCount = count (u8).
                    r.conn_count = (count & 0xFF) as u8;
                    // extraCost = the damaged-road extra step cost if n = 1 and
                    // IsRoadDamaged(c), else 0.0.
                    r.extra_cost = if n == net::CAR && h.is_road_damaged(cx, cz) {
                        s.tuning.damaged_road_cost
                    } else {
                        0.0
                    };
                    // If the occupant has network flag 0x40000: flags |= 1.
                    if has_record_flag {
                        r.flags |= 1;
                    }
                    // Choose the network type (§4.2 step 3).
                    let (ty, inv) = choose_type(s, h, occ, n);
                    r.net_type = ty;
                    r.inv_cap = inv;
                }
            }
            // Create the edge-density entries of the cell for every t on n, if
            // missing (traffic-simulator.md §4.8).
            for t in 0..travel::COUNT as u32 {
                if super::NET_OF_TRAVEL[t as usize] == n {
                    for evening in [false, true] {
                        s.edge_density_mut(cx, cz, t, evening);
                    }
                }
            }
        }
    }
}

/// Whether an occupant is of at least one taking-part network type (§4.2
/// step 3, §4.1 table `0x00AB3164`).
fn takes_part(h: &dyn NetworkHost, occ: u64) -> bool {
    (0..net_type::COUNT as u32).any(|k| TAKES_PART[k as usize] && h.is_of_type(occ, k))
}

/// Chooses the network type of a record (§4.2 step 3): start "none"; for
/// k = 0…12 where `mask[k]` has bit n and the occupant `IsOfType(k)`, set
/// `type = k`, with two exceptions: Street (3) is taken only while still
/// "none", and Road (0) does not replace Highway (2). The last match wins. If
/// still "none": type 0 and `invCap = 1.0`, else `invCap = invCapTable[type]`.
fn choose_type(s: &NetworkState, h: &dyn NetworkHost, occ: u64, n: u32) -> (u8, f32) {
    let mut chosen: Option<u32> = None;
    for k in 0..net_type::COUNT as u32 {
        if TRANSIT_MASK[k as usize] & (1 << n) == 0 {
            continue;
        }
        if !h.is_of_type(occ, k) {
            continue;
        }
        match k {
            net_type::STREET => {
                // Street is taken only while still "none".
                if chosen.is_none() {
                    chosen = Some(k);
                }
            }
            net_type::ROAD => {
                // Road does not replace Highway.
                if chosen != Some(net_type::HIGHWAY) {
                    chosen = Some(k);
                }
            }
            _ => chosen = Some(k),
        }
    }
    match chosen {
        None => (0, 1.0),
        Some(k) => (k as u8, s.tuning.inv_cap_table[k as usize]),
    }
}

/// `LaneContinuity` (§4.4, `RemoveBadElevatedPaths`): called for each car
/// connection (e → x) of an occupant at cell c. Restricts nibble x of
/// `priorCar[e]` at c from the flagged neighbours of the cell through side e.
fn lane_continuity(s: &mut NetworkState, h: &dyn NetworkHost, cx: i32, cz: i32, occ: u64, e: usize, x: usize) {
    // Step 1: from the occupant's path info, get the car paths (path type 1)
    // that start at entry e; keep those whose exit byte equals x; collect the
    // first vertex of each. If none, nothing changes.
    let Some(pi) = h.path_info(occ) else { return };
    let vertices: Vec<u64> = h
        .path_info_exits_from_entry(pi, travel::CAR, e as u32)
        .into_iter()
        .filter(|(exit, _)| *exit as usize == x)
        .map(|(_, v)| v)
        .collect();
    if vertices.is_empty() {
        return;
    }
    // Step 2: P = the neighbour through side e (no bounds check; the query
    // returns nothing off-map).
    let (px, pz) = (cx + super::DX[e], cz + super::DZ[e]);
    if !s.in_city(px, pz) {
        return;
    }
    let back = super::OPPOSITE_SIDE[e];
    let mut m: u16 = 0;
    let mut any_flagged = false;
    for nocc in h.network_occupants_in_rect(CellRect::new(px, pz, px, pz)) {
        if !h.has_any_network_flag(nocc, super::network_flag::LANE_CONTINUITY) {
            continue;
        }
        let Some(npi) = h.path_info(nocc) else { continue };
        any_flagged = true;
        // M = 0 for this occupant; the last qualifying occupant decides
        // *[quirk]*.
        let mut this_m: u16 = 0;
        for &v in &vertices {
            for entry in h.path_info_entries_near(npi, travel::CAR, v, back as u32, 4.0) {
                this_m |= 1 << entry;
            }
        }
        m = this_m;
    }
    // With no flagged occupant in P, the word keeps 0xFFFF from the reset.
    if !any_flagged {
        return;
    }
    // Write nibble x of priorCar[e] at c := M.
    if let Some(c) = s.conn_mut(cx, cz) {
        c.prior_car[e] = (c.prior_car[e] & !(0xF << (4 * x))) | ((m & 0xF) << (4 * x));
    }
}
