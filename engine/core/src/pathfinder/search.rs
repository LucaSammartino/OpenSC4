// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `FindPath` (§4.3): per-call preparation (§4.4), the standard destination
//! search (§4.5), start nodes (§4.6), the main loop (§4.8), AddTripNode
//! (§4.9), the goal test (§4.10), the work-time limit (§4.11), the exhausted
//! open set (§4.12), recording a trip (§4.13) and RecalculateNodeScores
//! (§4.18).

use crate::traffic_network::{
    boundary_cells, cell_key, grow_clamp, net, travel, wormhole, NetworkInfo, NetworkModel, NetworkState, DX, DZ,
    OPPOSITE_SIDE,
};
use crate::x87::r24;

use super::nodes::{node_key, Node, EXIT_KEY_BIT};
use super::{
    edge_to, filter, flag, rect_contains, result, CellRect, PathCx, Pathfinder, PathfinderHost, WorkCheck,
    LEGAL_ENDING_SUBTYPE, LEGAL_STARTING_SUBTYPE, NETWORK_OF_TRAVEL, NO_EXCLUDED_RECT, NO_PARK_AND_RIDE,
    OVER_BUDGET_LIMIT, PROHIBIT_SHARP_TURNS, SEARCH_RECT_MARGIN, START_KEY_PREFIX, STRIKE_ON, STRIKE_SWITCH_MASK,
    WORK_CHECK_INTERVAL,
};

/// One AddTripNode request (§4.9): the passed cell (for an exit node the
/// in-city cell it is reached from), the entry side (for an exit node the
/// direction of the move), the travel type, the step cost, the budget and the
/// exit flag.
#[derive(Clone, Copy, Debug)]
pub(super) struct Step {
    pub cell: (i32, i32),
    pub side: usize,
    pub t: usize,
    pub cost: f32,
    pub max_cost: f32,
    pub exit: bool,
}

/// `DistanceToRectangle(x, z, rect)` (§4.12): 0.0 inside; with `x` in the x
/// range the integer z distance to the nearer z edge; with `z` in the z range
/// the integer x distance; otherwise the Euclidean length to the nearest
/// corner (`LengthOfLine`, float: the integer squares and their sum are exact,
/// the square root is rounded once to `f32`).
pub fn distance_to_rect(x: i32, z: i32, r: CellRect) -> f32 {
    let in_x = x >= r.x1 && x <= r.x2;
    let in_z = z >= r.z1 && z <= r.z2;
    let dz = if z < r.z1 { r.z1 - z } else { z - r.z2 };
    let dx = if x < r.x1 { r.x1 - x } else { x - r.x2 };
    match (in_x, in_z) {
        (true, true) => 0.0,
        (true, false) => dz as f32,
        (false, true) => dx as f32,
        (false, false) => {
            let (dx, dz) = (f64::from(dx), f64::from(dz));
            r24((dx * dx + dz * dz).sqrt()) as f32
        }
    }
}

/// The cells of a start ring that §4.6 uses: the ring's cells in boundary
/// order (`BoundaryIterator`), keeping those with `x` inside the start rect's
/// x range or `z` inside its z range (cells diagonal to the start rect are
/// skipped).
pub fn start_ring_cells(start: CellRect, ring: CellRect) -> Vec<(i32, i32)> {
    boundary_cells(ring)
        .into_iter()
        .filter(|&(x, z)| (x >= start.x1 && x <= start.x2) || (z >= start.z1 && z <= start.z2))
        .collect()
}

/// Manhattan distance from a cell to a rectangle (per axis the distance to
/// the rect's range, 0 inside it; §4.5 step 4).
fn manhattan_to_rect(x: i32, z: i32, r: CellRect) -> i64 {
    let axis = |v: i32, lo: i32, hi: i32| -> i64 {
        if v < lo {
            i64::from(lo) - i64::from(v)
        } else if v > hi {
            i64::from(v) - i64::from(hi)
        } else {
            0
        }
    };
    axis(x, r.x1, r.x2) + axis(z, r.z1, r.z2)
}

/// The distance-table formula of §4.4 for one axis: `e0 − i` before the
/// range, 0 inside, `i − e1` after it.
fn axis_distance(i: i32, e0: i32, e1: i32) -> i32 {
    if i < e0 {
        e0.wrapping_sub(i)
    } else if i <= e1 {
        0
    } else {
        i.wrapping_sub(e1)
    }
}

impl Pathfinder {
    /// Whether (x, z) is in the city (§4.1): `0 ≤ x < W`, `0 ≤ z < H`.
    pub(super) fn in_city(&self, x: i32, z: i32) -> bool {
        x >= 0 && z >= 0 && x < self.count_x && z < self.count_z
    }

    /// `H(x, z)` (§4.4): 0.0 inside the search rect, else the float of the
    /// integer `dX[x] + dZ[z]`.
    pub fn heuristic(&self, x: i32, z: i32) -> f32 {
        if rect_contains(self.search_rect, x, z) {
            return 0.0;
        }
        let e = self.end_rect;
        let dx = usize::try_from(x.wrapping_add(1))
            .ok()
            .and_then(|i| self.dist_x.get(i).copied())
            .unwrap_or_else(|| axis_distance(x, e.x1, e.x2));
        let dz = usize::try_from(z.wrapping_add(1))
            .ok()
            .and_then(|i| self.dist_z.get(i).copied())
            .unwrap_or_else(|| axis_distance(z, e.z1, e.z2));
        dx.wrapping_add(dz) as f32
    }

    /// The score `f = r24(r24(H(x, z) × w) + g)` (§4.4).
    pub fn score(&self, x: i32, z: i32, g: f32) -> f32 {
        let hw = r24(f64::from(self.heuristic(x, z)) * f64::from(self.weight));
        r24(hw + f64::from(g)) as f32
    }

    /// Builds the distance tables from the end rect (§4.4), i = −1…W and
    /// −1…H.
    fn build_distance_tables(&mut self) {
        let e = self.end_rect;
        self.dist_x = (-1..=self.count_x).map(|i| axis_distance(i, e.x1, e.x2)).collect();
        self.dist_z = (-1..=self.count_z).map(|i| axis_distance(i, e.z1, e.z2)).collect();
    }

    // ---- FindPath (§4.3, §4.4) ----

    /// `FindPath()` (slot 23, §4.3): 1 found, 0 not found, 2 suspended.
    pub fn find_path(&mut self, cx: &mut PathCx<'_>) -> u32 {
        if !self.initialised {
            // The original reads through null distance tables [choice].
            return result::NOT_FOUND;
        }
        // Step 1: the process-wide statics, first call in the process.
        let mut wc = self.shared.get();
        if !wc.initialised {
            wc = WorkCheck {
                initialised: true,
                multiplier: WORK_CHECK_INTERVAL,
                adaptation: 0,
            };
            self.shared.set(wc);
        }
        self.work_step = wc.multiplier;
        self.work_counter = 0;
        // Step 2: the work timer; traffic arrived = 0.
        self.deadline = cx.host.work_deadline(self.max_work_time);
        self.arrived = 0;
        // Step 3.
        if self.flags & flag::PARK_AND_RIDE == 0 {
            self.park_ride = NO_PARK_AND_RIDE;
        }
        // Step 4: maxCost, also when resuming.
        self.tables = cx.host.travel_tables();
        let start_costs = self.start_costs.unwrap_or(self.tables.default_start_cost);
        for (m, &c) in self.max_cost.iter_mut().zip(&start_costs) {
            *m = r24(f64::from(c) + f64::from(self.trip_length)) as f32;
        }
        // Step 5.
        if !self.suspended {
            if self.flags & flag::CONTINUE == 0 {
                self.excluded_rect = NO_EXCLUDED_RECT;
                self.release_path_nodes();
            }
            self.result = None;
            self.over_budget = [0; 9];
            self.disabled = 0;
        }
        // Step 6: the strike reduces the mask of switch transitions only.
        self.strike = cx.host.strike_state() == STRIKE_ON;
        self.switch_mask = if self.strike {
            self.allowed & STRIKE_SWITCH_MASK
        } else {
            self.allowed
        };
        // Step 7.
        if !self.suspended {
            let s = self.start_rect;
            let m = SEARCH_RECT_MARGIN;
            self.search_rect = CellRect::new(
                s.x1.saturating_sub(m),
                s.z1.saturating_sub(m),
                s.x2.saturating_add(m),
                s.z2.saturating_add(m),
            );
            match self.filter {
                filter::STANDARD => {
                    self.weight = cx.host.nearest_destination_attractiveness();
                    if !self.find_nearest_standard_dest(cx) {
                        return result::NOT_FOUND;
                    }
                    self.build_distance_tables();
                    if self.flags & flag::CONTINUE != 0 {
                        self.recalculate_node_scores();
                    }
                }
                filter::END_RECT => {
                    // No score recomputation, even with flag 4 *[quirk]*.
                    self.build_distance_tables();
                    self.weight = cx.host.nearest_destination_attractiveness();
                }
                _ => {
                    self.build_distance_tables();
                    self.weight = 1.0;
                }
            }
            if self.starting == 0 {
                return result::NOT_FOUND;
            }
            if self.flags & flag::CONTINUE == 0 && self.create_start_nodes(cx, &start_costs) == 0 {
                return result::NOT_FOUND;
            }
        }
        // Step 8: the volume grids are read through the host; clear the
        // suspended state and run the main loop.
        self.suspended = false;
        self.main_loop(cx)
    }

    /// `FindPath` on a network kept in a [`NetworkModel`]: the network state
    /// is taken out of the model for the call, so no borrow of it is held
    /// across a host callback.
    pub fn find_path_on(&mut self, model: &NetworkModel, host: &dyn PathfinderHost) -> u32 {
        model.run(|cx| self.find_path(&mut PathCx { net: cx.s, host }))
    }

    // ---- §4.5 FindNearestStandardDest ----

    /// `FindNearestStandardDest` (§4.5): chooses the end rect before the
    /// search. The caller's end rect is overwritten *[quirk]*.
    fn find_nearest_standard_dest(&mut self, cx: &mut PathCx<'_>) -> bool {
        let host = cx.host;
        // Step 1.
        let wanted = (0..4u32).any(|k| self.dest_flags & (1 << k) != 0 && host.free_destinations(k) != 0);
        if !wanted {
            return false;
        }
        // Step 2: the search bounds; an off-map start excludes its edge.
        let (w, h) = (self.count_x, self.count_z);
        let mut b = CellRect::new(-1, -1, w, h);
        let s = self.start_rect;
        if s.x1 < 0 {
            b.x1 = 0;
            self.excluded_rect = CellRect::new(-1, -1, -1, h);
        } else if s.x2 == w {
            b.x2 = w - 1;
            self.excluded_rect = CellRect::new(w, -1, w, h);
        } else if s.z1 < 0 {
            b.z1 = 0;
            self.excluded_rect = CellRect::new(-1, -1, w, -1);
        } else if s.z2 == h {
            b.z2 = h - 1;
            self.excluded_rect = CellRect::new(-1, h, w, h);
        }
        // Step 3: the ring search.
        let mut r = s;
        loop {
            let left = r.x1.checked_sub(1).is_some_and(|v| v >= b.x1);
            let right = r.x2.checked_add(1).is_some_and(|v| v <= b.x2);
            let top = r.z1.checked_sub(1).is_some_and(|v| v >= b.z1);
            let bottom = r.z2.checked_add(1).is_some_and(|v| v <= b.z2);
            if !(left || right || top || bottom) {
                return false;
            }
            if left {
                r.x1 -= 1;
            }
            if right {
                r.x2 += 1;
            }
            if top {
                r.z1 -= 1;
            }
            if bottom {
                r.z2 += 1;
            }
            let mut cells = Vec::new();
            if left {
                cells.extend((r.z1..=r.z2).map(|z| (r.x1, z)));
            }
            if right {
                cells.extend((r.z1..=r.z2).map(|z| (r.x2, z)));
            }
            if top {
                cells.extend((r.x1..=r.x2).map(|x| (x, r.z1)));
            }
            if bottom {
                cells.extend((r.x1..=r.x2).map(|x| (x, r.z2)));
            }
            let candidates: Vec<(i32, i32)> = cells
                .into_iter()
                .filter(|&(x, z)| {
                    host.destination_byte(x, z) & self.dest_flags != 0 && !rect_contains(self.excluded_rect, x, z)
                })
                .collect();
            if candidates.is_empty() {
                continue;
            }
            // Step 4: in-city candidates first; the first with the smallest
            // Manhattan distance (strict less-than).
            let any_in_city = candidates.iter().any(|&(x, z)| self.in_city(x, z));
            let mut best: Option<((i32, i32), i64)> = None;
            for &(x, z) in &candidates {
                if any_in_city && !self.in_city(x, z) {
                    continue;
                }
                let d = manhattan_to_rect(x, z, s);
                if best.map_or(true, |(_, bd)| d < bd) {
                    best = Some(((x, z), d));
                }
            }
            let Some(((x, z), _)) = best else {
                return false;
            };
            // Step 5.
            if self.in_city(x, z) {
                let Some(lot) = host.lot_at(x, z) else {
                    return false;
                };
                self.end_rect = host.lot_bounding_rect(lot);
            } else {
                self.end_rect = CellRect::new(x, z, x, z);
            }
            return true;
        }
    }

    // ---- §4.6 start nodes ----

    /// Creates or fetches a start node, sets it up and pushes it (§4.6).
    fn push_start_node(&mut self, key: u32, cell: (i32, i32), t: usize, side: usize, g: f32) {
        let (id, _) = self
            .store
            .find_or_create(key, || Node::new(key, cell, t as u8, side as u8));
        let f = self.score(cell.0, cell.1, g);
        let n = &mut self.store.nodes[id as usize];
        n.g = g;
        n.f = f;
        n.parent = None;
        n.side = side as u8;
        n.open = true;
        self.open.push(&mut self.store.nodes, id);
    }

    /// Creates the start nodes (§4.6); returns how many were pushed.
    fn create_start_nodes(&mut self, cx: &mut PathCx<'_>, start_costs: &[f32; 9]) -> usize {
        let host = cx.host;
        let s0 = self.start_rect;
        let (x0, z0) = (s0.x1, s0.z1);
        let start_in_city = self.in_city(x0, z0);
        // The start-edge table: which entry sides a start node may have.
        let sides: u8 = if start_in_city {
            0xF
        } else if x0 < 0 {
            1 << 0
        } else if x0 >= self.count_x {
            1 << 2
        } else if z0 < 0 {
            1 << 1
        } else {
            1 << 3
        };
        let lot_start = start_in_city && host.lot_at(x0, z0).is_some();
        let mut created = 0usize;
        // Rings 1 … MaxDistanceOriginToNetwork, each the previous one grown
        // by one and clamped to the city (the clamped values carry over).
        let mut ring = s0;
        for _ in 0..host.max_distance_origin_to_network() {
            ring = grow_clamp(ring, 1, self.count_x, self.count_z);
            for (x, z) in start_ring_cells(s0, ring) {
                let rec = cx.net.conn_at(x, z).copied().unwrap_or_default();
                for n in 0..net::COUNT as u32 {
                    let Some(info) = cx.net.network_info(n, x, z).copied() else {
                        continue;
                    };
                    if lot_start
                        && !LEGAL_STARTING_SUBTYPE
                            .get(usize::from(info.net_type))
                            .copied()
                            .unwrap_or(false)
                    {
                        continue;
                    }
                    for t in 0..travel::COUNT {
                        if self.starting & (1 << t) == 0 || NETWORK_OF_TRAVEL[t] != n {
                            continue;
                        }
                        for side in 0..4 {
                            if sides & (1 << side) == 0 || rec.exits(t, side) == 0 {
                                continue;
                            }
                            let key = node_key(START_KEY_PREFIX | side as u32, t as u32, x, z);
                            self.push_start_node(key, (x, z), t, side, start_costs[t]);
                            created += 1;
                        }
                    }
                }
            }
        }
        // The off-map start fallback.
        if created == 0 && !start_in_city {
            for conn in host.neighbor_connections() {
                let Some(&(_, link_cost)) = conn.links.iter().find(|(p, _)| *p == (x0, z0)) else {
                    continue;
                };
                for (t, &cost) in start_costs.iter().enumerate() {
                    if self.starting & conn.travel_mask & (1 << t) == 0 {
                        continue;
                    }
                    let (x, z) = conn.cell;
                    let key = node_key(START_KEY_PREFIX, t as u32, x, z);
                    let g = r24(f64::from(cost) + f64::from(link_cost)) as f32;
                    self.push_start_node(key, (x, z), t, 0, g);
                    created += 1;
                }
            }
        }
        created
    }

    // ---- §4.8 main loop ----

    fn main_loop(&mut self, cx: &mut PathCx<'_>) -> u32 {
        loop {
            let Some(c) = self.open.pop(&mut self.store.nodes) else {
                return self.exhausted(cx);
            };
            let node = *self.store.get(c);
            let (x, z) = (node.x, node.z);
            let t = usize::from(node.t);
            let in_city = self.in_city(x, z);
            // §4.8.1 goal test.
            let can_reach = self.tables.can_reach_destination.get(t).copied().unwrap_or(false);
            let mut cannot_end = !can_reach;
            if can_reach && in_city {
                if let Some(info) = cx.net.network_info(NETWORK_OF_TRAVEL[t], x, z) {
                    cannot_end = !LEGAL_ENDING_SUBTYPE
                        .get(usize::from(info.net_type))
                        .copied()
                        .unwrap_or(false);
                }
            }
            let skip_goal = in_city && cx.net.switch_id_at(x, z) != 0;
            if !skip_goal && self.at_goal(cx, c, cannot_end) {
                self.result = Some(c);
                // Pushed back, so a continued search sees it again.
                self.open.push(&mut self.store.nodes, c);
                self.record_trip(cx);
                self.adapt();
                return result::FOUND;
            }
            // Off-map nodes are never expanded.
            if in_city {
                self.expand(cx, c);
            }
            // §4.8.5 close and time check.
            let n = &mut self.store.nodes[c as usize];
            n.closed = true;
            n.open = false;
            if self.max_work_time != 0 {
                self.work_counter = self.work_counter.wrapping_add(self.work_step);
                if self.work_counter % WORK_CHECK_INTERVAL == 0 && cx.host.work_deadline_reached(self.deadline) {
                    self.suspended = true;
                    self.adapt();
                    return result::SUSPENDED;
                }
            }
        }
    }

    /// The step cost onto a cell with network info `info` for travel type `t`
    /// (§4.8.2 (a)): `r24(r24(Curve(q) × v) + extraCost)` with
    /// `q = r24(volume × invCapacity)` when the type is affected by traffic,
    /// else `r24(v + extraCost)`.
    fn step_cost(&self, cx: &PathCx<'_>, t: usize, info: &NetworkInfo, x: i32, z: i32) -> f32 {
        let v = cx
            .net
            .inv_max_speed
            .get(t)
            .and_then(|row| row.get(usize::from(info.net_type)))
            .copied()
            .unwrap_or(f32::MAX);
        if self.tables.affected_by_traffic.get(t).copied().unwrap_or(false) {
            let vol = cx.host.volume(NETWORK_OF_TRAVEL[t], x, z);
            let q = r24(f64::from(vol) * f64::from(info.inv_cap)) as f32;
            let curve = cx.net.congestion_curve.eval(q);
            let a = r24(f64::from(curve) * f64::from(v));
            r24(a + f64::from(info.extra_cost)) as f32
        } else {
            r24(f64::from(v) + f64::from(info.extra_cost)) as f32
        }
    }

    /// Expands an in-city node (§4.8.2–§4.8.4).
    fn expand(&mut self, cx: &mut PathCx<'_>, c: u32) {
        let node = *self.store.get(c);
        let (x, z) = (node.x, node.z);
        let t = usize::from(node.t);
        let s = usize::from(node.side);
        let parent = node.parent.map(|p| *self.store.get(p));
        let rec = cx.net.conn_at(x, z).copied().unwrap_or_default();
        let net_t = NETWORK_OF_TRAVEL[t];

        // §4.8.2: moves along the network.
        let mut exits = rec.exits(t, s);
        if PROHIBIT_SHARP_TURNS[t] {
            if let Some(p) = parent {
                exits &= !((1u16 << s) | (1u16 << p.side));
            }
        }
        let car_turn_check = net_t == net::CAR && parent.is_some();
        for d in 0..4 {
            let (bx, bz) = (x + DX[d], z + DZ[d]);
            let s2 = OPPOSITE_SIDE[d];
            // (a) Same-type move.
            if exits & (1 << d) != 0 {
                if self.in_city(bx, bz) {
                    if let Some(info) = cx.net.network_info(net_t, bx, bz).copied() {
                        let blocked = match parent {
                            Some(p) if car_turn_check => rec.prior_car[s] & (1 << (4 * d + usize::from(p.side))) == 0,
                            _ => false,
                        };
                        if !blocked {
                            let cost = self.step_cost(cx, t, &info, bx, bz);
                            self.add_trip_node(c, self.same_type_step((bx, bz), s2, t, cost));
                        }
                    } else {
                        let id = cx.net.switch_id_at(bx, bz);
                        if id != 0 {
                            let cost = cx.net.transit_switch_entry_cost(id);
                            self.add_trip_node(c, self.same_type_step((bx, bz), s2, t, cost));
                        }
                    }
                } else {
                    // Leaving the map: the cost is the node's own heuristic
                    // term, evaluated at (x, z) *[quirk]*.
                    let cost = r24(f64::from(node.f) - f64::from(node.g)) as f32;
                    let step = Step {
                        exit: true,
                        ..self.same_type_step((x, z), d, t, cost)
                    };
                    self.add_trip_node(c, step);
                }
            }
            // (b) Switch move: no exit bit and no bounds check needed; an
            // off-map b reads "no switch, no info" (§10 Q4) *[choice]*.
            let t2 = rec.switch_to(t, d) as usize;
            if t2 != 0xF && t2 < travel::COUNT && self.switch_mask & (1 << t2) != 0 {
                let id = cx.net.switch_id_at(bx, bz);
                if id != 0 {
                    let cost = cx.net.transit_switch_entry_cost(id);
                    self.add_trip_node(c, self.same_type_step((bx, bz), s2, t2, cost));
                } else if let Some(info) = cx.net.network_info(NETWORK_OF_TRAVEL[t2], bx, bz).copied() {
                    let cost = self.step_cost(cx, t2, &info, bx, bz);
                    self.add_trip_node(c, self.same_type_step((bx, bz), s2, t2, cost));
                }
            }
        }

        // §4.8.3: wormholes.
        let wh = rec.wormhole;
        if wh & wormhole::TUNNEL != 0 {
            if let Some(tun) = cx.net.tunnels.get(u32::from(cell_key(x, z))).copied() {
                if tun.mask & (1 << t) != 0 {
                    self.add_trip_node(c, self.same_type_step(tun.other, s, t, tun.cost[t]));
                }
            }
        }
        if !self.strike {
            // Subway station: unreachable in 1.1.641 (the bit is never set).
            // The condition uses the unmasked shift of `conn[t]` by the node's
            // entry side, as written *[quirk]*; the allowed mask is not
            // checked.
            if wh & wormhole::SUBWAY != 0 {
                let t2 = if t == travel::SUBWAY {
                    travel::WALK
                } else {
                    travel::SUBWAY
                };
                for side2 in 0..4 {
                    if rec.conn[t] >> (4 * s) != 0 {
                        self.add_trip_node(c, self.same_type_step((x, z), side2, t2, 0.0));
                    }
                }
            }
            if wh & wormhole::FERRY != 0 && self.flags & flag::NO_FERRIES == 0 {
                self.ferry_moves(cx.net, c, (x, z), t);
            }
        }

        // §4.8.4: park-and-ride return.
        if (x, z) == self.park_ride {
            let (px, pz) = self.park_ride;
            let b = (px + DX[self.park_ride_side], pz + DZ[self.park_ride_side]);
            let step = Step {
                cell: b,
                side: edge_to(self.park_ride, b),
                t: travel::CAR,
                cost: 0.0,
                max_cost: self.max_cost[t],
                exit: !self.in_city(b.0, b.1),
            };
            self.add_trip_node(c, step);
        }
    }

    /// The ferry moves of a terminal cell (§4.8.3 step 4).
    fn ferry_moves(&mut self, net: &NetworkState, c: u32, (x, z): (i32, i32), t: usize) {
        let Some(term) = net
            .terminal_by_departure(cell_key(x, z))
            .and_then(|i| net.ferries.get(i))
        else {
            return;
        };
        if term.mask & (1 << t) == 0 {
            return;
        }
        for link in &term.links {
            let cell = (i32::from(link.arrival_x), i32::from(link.arrival_z));
            self.add_trip_node(c, self.same_type_step(cell, 0, t, link.cost));
        }
        for nl in &term.neighbour_links {
            let edge = (i32::from(nl.edge_x), i32::from(nl.edge_z));
            let off = (i32::from(nl.off_x), i32::from(nl.off_z));
            // The exit node at the off-map point, reached from the in-city
            // edge cell: its side is the direction from the edge cell to the
            // off-map point.
            let step = Step {
                exit: true,
                ..self.same_type_step(edge, edge_to(off, edge), t, nl.cost)
            };
            self.add_trip_node(c, step);
        }
    }

    /// A non-exit step of type `t` with the budget `maxCost[t]`.
    fn same_type_step(&self, cell: (i32, i32), side: usize, t: usize, cost: f32) -> Step {
        Step {
            cell,
            side,
            t,
            cost,
            max_cost: self.max_cost[t],
            exit: false,
        }
    }

    // ---- §4.9 AddTripNode ----

    /// `AddTripNode` (§4.9) from predecessor `c`.
    pub(super) fn add_trip_node(&mut self, c: u32, st: Step) {
        let t = st.t;
        // 1. A disabled type is ignored.
        if self.disabled & (1 << t) != 0 {
            return;
        }
        // 2. The key; no stepping straight back into the predecessor's state.
        let cn = *self.store.get(c);
        let prefix = (u32::from(cn.side) << 2) | st.side as u32;
        let mut key = node_key(prefix, t as u32, st.cell.0, st.cell.1);
        if st.exit {
            key |= EXIT_KEY_BIT;
        }
        if let Some(p) = cn.parent {
            if self.store.get(p).key == key {
                return;
            }
        }
        // 3.
        let g = r24(f64::from(st.cost) + f64::from(cn.g)) as f32;
        // 4. The budget: "not g ≤ maxCost", so a NaN g fails it *[compat]*.
        let within_budget = g <= st.max_cost;
        if self.flags & flag::NO_BUDGET == 0 && !within_budget {
            self.over_budget[t] += 1;
            if self.over_budget[t] > OVER_BUDGET_LIMIT {
                self.disabled |= 1 << t;
            }
            return;
        }
        // 5. Fetch or create; an open or closed node must get strictly
        // better. A NaN g passes (the comparison is false when unordered).
        let cell = if st.exit {
            (st.cell.0 + DX[st.side], st.cell.1 + DZ[st.side])
        } else {
            st.cell
        };
        let (id, _) = self
            .store
            .find_or_create(key, || Node::new(key, cell, t as u8, st.side as u8));
        let existing = self.store.get(id);
        if (existing.open || existing.closed) && existing.g <= g {
            return;
        }
        // 6. Predecessor, g, side, f (H at the passed cell); Update or Push.
        let f = self.score(st.cell.0, st.cell.1, g);
        let n = &mut self.store.nodes[id as usize];
        let was_open = n.open;
        n.parent = Some(c);
        n.closed = false;
        n.g = g;
        n.side = st.side as u8;
        n.f = f;
        if was_open {
            self.open.update(&mut self.store.nodes, id);
        } else {
            n.open = true;
            self.open.push(&mut self.store.nodes, id);
        }
    }

    // ---- §4.10 AtGoal ----

    fn at_goal(&mut self, cx: &mut PathCx<'_>, c: u32, cannot_end: bool) -> bool {
        let n = *self.store.get(c);
        match self.filter {
            filter::STANDARD => self.goal_standard(cx.host, n, cannot_end),
            filter::OCCUPANT => self.goal_occupant(cx.host, n.x, n.z),
            filter::END_RECT => self.goal_end_rect(n.x, n.z, cannot_end),
            _ => false,
        }
    }

    /// `CheckDestinationCapacity(holder, k)` (§4.10.1).
    fn check_destination_capacity(&mut self, host: &dyn PathfinderHost, holder: u64, k: u32) -> bool {
        let max = host.max_trip_capacity(holder, k);
        let used = host.used_trip_capacity(holder, k);
        if max == 0 || used >= max {
            return false;
        }
        self.arrived = (max - used).min(self.density);
        self.dest_type_reached = k as u8;
        true
    }

    /// Filter 0 (§4.10.1).
    fn goal_standard(&mut self, host: &dyn PathfinderHost, n: Node, cannot_end: bool) -> bool {
        let (x, z) = (n.x, n.z);
        let flags = self.dest_flags;
        if self.in_city(x, z) || rect_contains(self.start_rect, x, z) || rect_contains(self.excluded_rect, x, z) {
            if cannot_end {
                return false;
            }
            for d in 0..4 {
                let (nx, nz) = (x + DX[d], z + DZ[d]);
                if !self.in_city(nx, nz) || rect_contains(self.start_rect, nx, nz) {
                    continue;
                }
                let want = host.destination_byte(nx, nz) & flags;
                if want == 0 {
                    continue;
                }
                let Some(lot) = host.lot_at(nx, nz) else {
                    continue;
                };
                let holder = host.lot_holder(lot);
                for k in 0..4u32 {
                    if want & (1 << k) == 0 {
                        continue;
                    }
                    if self.check_destination_capacity(host, holder, k) {
                        self.end_cell = (nx, nz);
                        return true;
                    }
                    // A stale destination: cleared over the lot and counted
                    // down *[compat]*.
                    let r = host.lot_bounding_rect(lot);
                    for lx in r.x1..=r.x2 {
                        for lz in r.z1..=r.z2 {
                            let b = host.destination_byte(lx, lz);
                            host.set_destination_byte(lx, lz, b & !(1 << k));
                        }
                    }
                    let free = host.free_destinations(k);
                    if free != 0 {
                        host.set_free_destinations(k, free - 1);
                    }
                }
            }
            false
        } else {
            // Off-map and not in those rects: the neighbour pool; cannotEnd
            // is ignored.
            let Some(holder) = host.neighbor_pool_holder(u32::from(n.t), x, z) else {
                return false;
            };
            for k in 0..4u32 {
                let bits = host.destination_byte(x, z);
                if bits & flags & (1 << k) == 0 {
                    continue;
                }
                if self.check_destination_capacity(host, holder, k) {
                    self.end_cell = (x, z);
                    return true;
                }
                host.set_destination_byte(x, z, bits & !(1 << k));
            }
            false
        }
    }

    /// Filter 3 (§4.10.2).
    fn goal_end_rect(&mut self, x: i32, z: i32, cannot_end: bool) -> bool {
        let e = self.end_rect;
        if rect_contains(e, x, z) && (!cannot_end || !self.in_city(x, z)) {
            self.end_cell = (x, z);
            self.arrived = self.density;
            return true;
        }
        if self.in_city(e.x1, e.z1) {
            for d in 0..4 {
                let (nx, nz) = (x + DX[d], z + DZ[d]);
                if rect_contains(e, nx, nz) && (!cannot_end || !self.in_city(nx, nz)) {
                    self.end_cell = (nx, nz);
                    self.arrived = self.density;
                    return true;
                }
            }
        }
        false
    }

    /// Filter 2 (§4.10.3). A null filter never matches *[choice]*.
    fn goal_occupant(&mut self, host: &dyn PathfinderHost, x: i32, z: i32) -> bool {
        let Some(filter) = self.occupant_filter else {
            return false;
        };
        let r = grow_clamp(CellRect::new(x, z, x, z), 1, self.count_x, self.count_z);
        let found = host
            .occupants_in_cells(r)
            .into_iter()
            .any(|occ| host.occupant_filter_includes(filter, occ));
        if found {
            self.arrived = self.density;
        }
        found
    }

    // ---- §4.11 adaptation ----

    /// The check-interval adaptation, at every return after the search
    /// started (§4.11).
    fn adapt(&mut self) {
        let k = self.work_counter >> 10;
        let mut wc = self.shared.get();
        if k > 20 {
            wc.adaptation += 1;
            if wc.adaptation >= 16 {
                wc.adaptation = 0;
                if wc.multiplier > 1 {
                    wc.multiplier /= 2;
                }
            }
        } else if (1..5).contains(&k) {
            wc.adaptation -= 1;
            if wc.adaptation <= -4 {
                wc.adaptation = 0;
                if wc.multiplier < WORK_CHECK_INTERVAL {
                    wc.multiplier *= 2;
                }
            }
        }
        self.shared.set(wc);
    }

    // ---- §4.12 exhausted open set ----

    fn exhausted(&mut self, cx: &mut PathCx<'_>) -> u32 {
        if self.flags & flag::NEAREST_ON_FAILURE != 0 && self.filter == filter::END_RECT && self.route.is_some() {
            let e = self.end_rect;
            let mut best: Option<(u32, f32)> = None;
            for id in self.store.bucket_order() {
                let n = self.store.get(id);
                let d = distance_to_rect(n.x, n.z, e);
                if best.map_or(true, |(_, bd)| d < bd) {
                    best = Some((id, d));
                }
            }
            if let Some((id, _)) = best {
                self.result = Some(id);
                self.record_trip(cx);
                self.adapt();
                return result::FOUND;
            }
        }
        self.adapt();
        result::NOT_FOUND
    }

    // ---- §4.13 recording a trip ----

    /// The node indices from `from` back to the root. A predecessor cycle
    /// (only possible with NaN costs) is cut after every stored node was
    /// visited once *[choice]*.
    pub(super) fn walk_to_root(&self, from: u32) -> Vec<u32> {
        let mut out = Vec::new();
        let mut cur = Some(from);
        while let Some(id) = cur {
            if out.len() > self.store.len() {
                break;
            }
            out.push(id);
            cur = self.store.get(id).parent;
        }
        out
    }

    fn record_trip(&mut self, cx: &mut PathCx<'_>) {
        let Some(res) = self.result else {
            return;
        };
        let walk = self.walk_to_root(res);
        // 1. Park-and-ride: the last Car → other transition of the walk.
        for &id in &walk {
            let n = *self.store.get(id);
            if let Some(p) = n.parent {
                let pn = self.store.get(p);
                if usize::from(pn.t) == travel::CAR && usize::from(n.t) != travel::CAR {
                    self.park_ride = (n.x, n.z);
                    self.park_ride_side = edge_to((pn.x, pn.z), (n.x, n.z));
                }
            }
        }
        // 2. The trip length, clamped below at +0.0 (NaN kept).
        let root = *self.store.get(*walk.last().unwrap_or(&res));
        let rn = *self.store.get(res);
        let mut l = r24(f64::from(rn.g) - f64::from(root.g)) as f32;
        if l < 0.0 {
            l = 0.0;
        }
        // 3. The neighbour connection cost (Windows: filters 0 and 3).
        if (self.filter == filter::STANDARD || self.filter == filter::END_RECT)
            && self.dest_type_reached != 3
            && !self.in_city(rn.x, rn.z)
        {
            if let Some(cost) = cx.host.traffic_neighbor_connection_cost(rn.x, rn.z) {
                l = r24(f64::from(cost) + f64::from(l)) as f32;
            }
        }
        // 4.
        self.trip_length = l;
        // 5. The route deque, root → result.
        if let Some(route) = self.route.as_mut() {
            route.clear();
            for &id in &walk {
                let n = self.store.get(id);
                route.push_front(super::RouteData {
                    x: n.x as u8,
                    z: n.z as u8,
                    travel_type: n.t,
                });
            }
        }
    }

    // ---- §4.18 RecalculateNodeScores ----

    /// `RecalculateNodeScores` (filter 0 with flag 4): in allocation order,
    /// `f = r24(g)` inside the search rect, else `r24(r24(H × w) + g)`; open
    /// nodes are Updated, which only sifts up *[quirk]*.
    fn recalculate_node_scores(&mut self) {
        for id in 0..self.store.len() as u32 {
            let n = *self.store.get(id);
            let f = if rect_contains(self.search_rect, n.x, n.z) {
                n.g
            } else {
                self.score(n.x, n.z, n.g)
            };
            self.store.nodes[id as usize].f = f;
            if n.open {
                self.open.update(&mut self.store.nodes, id);
            }
        }
    }
}
