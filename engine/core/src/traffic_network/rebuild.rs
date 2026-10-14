// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `DoConnectionsChanged` (§4.2), `SetConnection` (§4.3, on
//! [`NetworkState`]), lane continuity (§4.4), the network occupant messages
//! (§3) and `DoPostCityInit` (§1). The occupant messages, the transit-switch
//! and ferry activations and the full rebuild all funnel through
//! `DoConnectionsChanged`.

use super::model::{union_rect, Cx, NetworkModel, NetworkState};
use super::{
    net, net_type, network_flag, side, travel, CellRect, NetworkHost, DX, DZ, NET_OF_TRAVEL, OPPOSITE_SIDE,
    PATH_TYPE_CAR, PATH_TYPE_OF_NET, TAKES_PART, TRANSIT_MASK,
};

/// The radius of the lane-continuity vertex query (§4.4 step 2).
const LANE_RADIUS: f32 = 4.0;

/// Whether an occupant is of at least one taking-part network type (§4.1
/// table `0x00AB3164`).
fn takes_part(h: &dyn NetworkHost, occ: u64) -> bool {
    (0..net_type::COUNT as u32).any(|k| TAKES_PART[k as usize] && h.is_of_type(occ, k))
}

/// The travel types on transit network `n`.
fn travel_types_on(n: u32) -> impl Iterator<Item = usize> {
    (0..travel::COUNT).filter(move |&t| NET_OF_TRAVEL[t] == n)
}

/// The network type of a record (§4.2 step 3): start "none"; for k = 0…12
/// where `mask[k]` has bit n and the occupant `IsOfType(k)`, set `type = k`,
/// except that Street (3) is taken only while still "none" and Road (0) does
/// not replace Highway (2). The last match wins. "None" gives type 0 and
/// `invCap = 1.0`, else `invCap = invCapTable[type]`.
fn choose_type(s: &NetworkState, h: &dyn NetworkHost, occ: u64, n: u32) -> (u8, f32) {
    let mut chosen: Option<u32> = None;
    for k in 0..net_type::COUNT as u32 {
        if TRANSIT_MASK[k as usize] & (1 << n) == 0 || !h.is_of_type(occ, k) {
            continue;
        }
        match k {
            net_type::STREET if chosen.is_some() => {}
            net_type::ROAD if chosen == Some(net_type::HIGHWAY) => {}
            _ => chosen = Some(k),
        }
    }
    match chosen {
        None => (0, 1.0),
        Some(k) => (k as u8, s.tuning.inv_cap_table[k as usize]),
    }
}

impl Cx<'_> {
    /// `DoConnectionsChanged(x1, z1, x2, z2)` (§4.2) *[compat]*.
    pub(crate) fn do_connections_changed(&mut self, rect: CellRect) {
        let h = self.h;
        // Step 1: R = the rectangle grown by 2 and clamped; the intersection
        // dirty rectangle ∪= R and the dirty flag.
        let r = self.s.grow_clamp(rect, 2);
        let s = &mut *self.s;
        s.intersection_rect = if s.intersection_dirty {
            union_rect(s.intersection_rect, r)
        } else {
            r
        };
        s.intersection_dirty = true;
        // Step 2: reset every cell of R.
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
        if r.x1 <= r.x2 && r.z1 <= r.z2 {
            for occ in h.network_occupants_in_rect(r) {
                self.apply_occupant(occ);
            }
        }
        // Steps 4–6 over R1 = R grown by 1 more and clamped.
        let r1 = self.s.grow_clamp(r, 1);
        self.apply_switches(r1);
        self.apply_tunnels(r1);
        self.s.apply_ferries(r1);
        // Step 7.
        self.s.subnetworks_stale = true;
        self.s.network_changed = true;
    }

    /// One network occupant of §4.2 step 3.
    fn apply_occupant(&mut self, occ: u64) {
        let h = self.h;
        if !takes_part(h, occ) || h.has_network_flag(occ, network_flag::EXCLUDED) {
            return;
        }
        let p = h.occupant_position(occ);
        let (cx, cz) = h.position_to_cell(p[0], p[2]);
        // The original does not check; an occupant listed in R is in the
        // city [choice].
        if !self.s.in_city(cx, cz) {
            return;
        }
        let record_flag = h.has_network_flag(occ, network_flag::RECORD_FLAG);
        for n in 0..net::COUNT as u32 {
            let pt = PATH_TYPE_OF_NET[n as usize];
            let mut count: u16 = 0;
            for e in 0..side::COUNT {
                let mut found = false;
                for x in 0..side::COUNT {
                    if !h.path_connection(occ, pt, e as u32, x as u32) {
                        continue;
                    }
                    found = true;
                    count = count.wrapping_add(1);
                    for t in travel_types_on(n) {
                        if let Some(c) = self.s.conn_mut(cx, cz) {
                            c.conn[t] |= 1 << (x + 4 * e);
                        }
                        if t == travel::CAR {
                            self.lane_continuity(cx, cz, occ, e, x);
                        }
                    }
                }
                // No exit for e: the path info's turn-around test.
                if !found {
                    if let Some(pi) = h.path_info(occ) {
                        if h.path_info_turn_around(pi, pt, e as u32) {
                            count = count.wrapping_add(1);
                            for t in travel_types_on(n) {
                                self.s.set_connection(cx, cz, e, e, t);
                            }
                        }
                    }
                }
            }
            if count == 0 {
                continue;
            }
            let damaged = n == net::CAR && h.is_road_damaged(cx, cz);
            let (ty, inv) = choose_type(self.s, h, occ, n);
            let extra = if damaged { self.s.tuning.damaged_road_cost } else { 0.0 };
            if let Some(r) = self.s.get_or_create_info(n, cx, cz) {
                r.conn_count = count as u8;
                r.extra_cost = extra;
                if record_flag {
                    r.flags |= 1;
                }
                r.net_type = ty;
                r.inv_cap = inv;
            }
            for t in travel_types_on(n) {
                self.s.ensure_edge_density(cx, cz, t as u32);
            }
        }
    }

    /// `LaneContinuity` (§4.4, `RemoveBadElevatedPaths`) for a car connection
    /// (e → x) of `occ` at cell c.
    fn lane_continuity(&mut self, cx: i32, cz: i32, occ: u64, e: usize, x: usize) {
        let h = self.h;
        // Step 1: the first vertices of the car paths from entry e whose exit
        // byte is x. None → nothing changes.
        let Some(pi) = h.path_info(occ) else {
            return;
        };
        let vertices: Vec<u64> = h
            .path_info_exits_from_entry(pi, PATH_TYPE_CAR, e as u32)
            .into_iter()
            .filter(|&(exit, _)| usize::from(exit) == x)
            .map(|(_, v)| v)
            .collect();
        if vertices.is_empty() {
            return;
        }
        // Step 2: each flagged network occupant of P (the neighbour through
        // e; the query returns nothing off-map) writes nibble x of
        // priorCar[e]; the last one decides *[quirk]*.
        let (px, pz) = (cx + DX[e], cz + DZ[e]);
        let back = OPPOSITE_SIDE[e] as u32;
        for nocc in h.network_occupants_in_rect(CellRect::new(px, pz, px, pz)) {
            if !h.has_any_network_flag(nocc, network_flag::LANE_CONTINUITY) {
                continue;
            }
            let Some(npi) = h.path_info(nocc) else {
                continue;
            };
            let mut m: u16 = 0;
            for &v in &vertices {
                for entry in h.path_info_entries_near(npi, PATH_TYPE_CAR, v, back, LANE_RADIUS) {
                    if entry < 4 {
                        m |= 1 << entry;
                    }
                }
            }
            if let Some(c) = self.s.conn_mut(cx, cz) {
                c.prior_car[e] = (c.prior_car[e] & !(0xF << (4 * x))) | (m << (4 * x));
            }
        }
    }

    /// A network occupant inserted or removed (§3, `0x99EF1142` /
    /// `0x99EF1143` with data3 = `0xC772BF98`), after the caller's
    /// `QueryInterface(0xA821EF94)` succeeded. The simulator's per-type
    /// bookkeeping (DoRoadChanged …) is traffic-simulator.md's. The water
    /// pathfinder's own subscription (§4.10.1) is handled here too.
    pub(crate) fn occupant_changed(&mut self, occ: u64, removed: bool) {
        let h = self.h;
        if takes_part(h, occ) {
            // (a) a tunnel: §4.8.
            if h.is_tunnel(occ) {
                self.tunnel_changed(occ, removed);
            }
            // (b) the cell; on remove its 18 edge-density keys go.
            let p = h.occupant_position(occ);
            let (x, z) = h.position_to_cell(p[0], p[2]);
            if removed {
                self.s.erase_cell_edge_densities(x, z);
            }
            // (c) the rebuild.
            self.do_connections_changed(CellRect::new(x, z, x, z));
        }
        self.s.occupants_changed = true;
        self.water_occupant_changed(occ);
    }

    /// `DoPostCityInit` (§1, `0x26D31EC1`), the network part, in order.
    /// Steps 4 and 5 (the neighbour connections and the destination-map
    /// boundary pass) belong to traffic-simulator.md and run in the caller
    /// between [`NetworkModel::post_city_init_rebuild`] and
    /// [`NetworkModel::post_city_init_finish`]; [`NetworkModel::do_post_city_init`]
    /// runs both halves back to back.
    pub(crate) fn post_city_init_rebuild(&mut self) {
        // Steps 1, 2: re-read the switch rules, refresh the access points.
        self.reread_switches();
        // Step 3: the whole city.
        let all = self.s.city_rect();
        self.do_connections_changed(all);
    }

    /// Steps 6–8 and the water pathfinder's handler, as the network model
    /// alone runs them (it has no work-cycle state, so step 8 always runs).
    /// The traffic simulator calls [`Self::post_city_init_subnetworks`] and
    /// runs step 8 only in state 19 (traffic-simulator.md §3.5, TS7).
    pub(crate) fn post_city_init_finish(&mut self) {
        self.post_city_init_subnetworks();
        self.post_city_init_ferries();
        // The water pathfinder's own handler of the message (§4.10.1).
        self.water_post_city_init();
    }

    /// Steps 6 and 7: clear the labels; label, then the subnetwork info
    /// synchronously, published.
    pub(crate) fn post_city_init_subnetworks(&mut self) {
        self.s.clear_subnetworks();
        self.s.update_subnetworks();
        self.begin_subnetwork_info();
        self.work_on_subnetwork_info(usize::MAX);
    }

    /// Step 8 (only when the loaded work-cycle state is 19, TS7): every
    /// terminal marked for re-routing; the pass starts at the first
    /// terminal (the neighbour-cell vector and the counters are left as they
    /// are: only entering the work-cycle state rebuilds and rotates them).
    pub(crate) fn post_city_init_ferries(&mut self) {
        for t in &mut self.s.ferries {
            t.reroute = true;
        }
        self.s.ferry_pass.running = true;
        self.s.ferry_pass.terminal = 0;
    }
}

impl NetworkModel {
    /// `DoConnectionsChanged(x1, z1, x2, z2)` (§4.2).
    pub fn do_connections_changed(&self, rect: CellRect) {
        self.run(|cx| cx.do_connections_changed(rect));
    }

    /// `SetConnection(x, z, e, x', t)` (§4.3).
    pub fn set_connection(&self, x: i32, z: i32, e: usize, exit: usize, t: usize) {
        self.s.borrow_mut().set_connection(x, z, e, exit, t);
    }

    /// A network occupant inserted (`removed = false`) or removed (§3).
    pub fn occupant_changed(&self, occ: u64, removed: bool) {
        self.run(|cx| cx.occupant_changed(occ, removed));
    }

    /// `DoTunnelChanged` alone (§4.8), without the rebuild of §3.
    pub fn tunnel_changed(&self, occ: u64, removed: bool) -> bool {
        self.run(|cx| cx.tunnel_changed(occ, removed))
    }

    /// DoPostCityInit steps 1–3 (§1).
    pub fn post_city_init_rebuild(&self) {
        self.run(|cx| cx.post_city_init_rebuild());
    }

    /// DoPostCityInit steps 6–8 (§1) and the water pathfinder's handler, as
    /// the network model alone runs them (step 8 unconditionally).
    pub fn post_city_init_finish(&self) {
        self.run(|cx| cx.post_city_init_finish());
    }

    /// DoPostCityInit (§1), the network steps back to back.
    pub fn do_post_city_init(&self) {
        self.run(|cx| {
            cx.post_city_init_rebuild();
            cx.post_city_init_finish();
        });
    }
}
