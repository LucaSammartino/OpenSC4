// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Subnetworks (§4.11, `UpdateSubnetworks`): "first reached" labels of the
//! car graph per (cell, entry side), and the subnetwork info (§4.12,
//! `UpdateSubnetworkInfo`): per-label population and destination capacity.

use crate::variant::{mem, Elem, Variant};

use super::model::{boundary_cells, key32, Cx, NetworkModel, NetworkState, SubnetworkInfo};
use super::{prop, side, travel, wormhole, DX, DZ, OPPOSITE_SIDE};

/// The `k` of a `Fill` state that resumes after step 3 (§4.11).
const AFTER_TUNNEL: usize = side::COUNT + 1;

/// One state of the explicit `Fill` stack (§4.11): (cell, entry e, next k).
#[derive(Clone, Copy, Debug)]
struct FillState {
    x: i32,
    z: i32,
    e: usize,
    k: usize,
}

impl NetworkState {
    /// The clear step (`0x0070FCE0`, §4.11): counter := 1, every record's
    /// `subnet[0…3]` := 0.
    pub fn clear_subnetworks(&mut self) {
        self.subnet_counter = 1;
        for r in self.info.iter_mut().flatten() {
            r.subnet = [0; 4];
        }
    }

    /// `UpdateSubnetworks` (§4.11, work-cycle state 12): the forward seeds
    /// (x = 0 … X−2 outer, z = 0 … Z−2 inner, entry sides 0 then 1) and the
    /// backward seeds (x = X−2 … 0, z = Z−2 … 0, entry sides 2 then 3). The
    /// last row and column are never seeds *[quirk]*.
    pub fn update_subnetworks(&mut self) {
        let (xs, zs) = (self.count_x - 1, self.count_z - 1);
        for x in 0..xs {
            for z in 0..zs {
                self.seed(x, z, [0, 1]);
            }
        }
        for x in (0..xs).rev() {
            for z in (0..zs).rev() {
                self.seed(x, z, [2, 3]);
            }
        }
    }

    fn seed(&mut self, x: i32, z: i32, sides: [usize; 2]) {
        for e in sides {
            if self.car_info(x, z).is_some_and(|r| r.subnet[e] == 0) {
                self.fill(x, z, e);
            }
        }
    }

    fn subnet_of(&self, x: i32, z: i32, e: usize) -> Option<u16> {
        self.car_info(x, z).map(|r| r.subnet[e])
    }

    /// Whether (x, z) has a car record with `subnet[e] = 0`.
    fn unlabelled(&self, x: i32, z: i32, e: usize) -> bool {
        self.subnet_of(x, z, e) == Some(0)
    }

    fn set_subnet(&mut self, x: i32, z: i32, e: usize, label: u16) {
        if let Some(r) = self.network_info_mut(super::net::CAR, x, z) {
            r.subnet[e] = label;
        }
    }

    fn car_bit(&self, x: i32, z: i32, bit: usize) -> bool {
        self.conn_at(x, z)
            .is_some_and(|c| c.conn[travel::CAR] & (1 << bit) != 0)
    }

    /// `Fill(x, z, e)` (§4.11): labels the (cell, entry side) states reachable
    /// along directed car edges (`rep(1)` = Car) with an explicit stack.
    pub fn fill(&mut self, x0: i32, z0: i32, e0: usize) {
        // Step 1: a car record with subnet[e] = 0 and connCount ≤ 2, not a
        // tunnel cell.
        match self.car_info(x0, z0) {
            Some(r) if r.subnet[e0] == 0 && r.conn_count <= 2 => {}
            _ => return,
        }
        if self.wormhole_at(x0, z0) & wormhole::TUNNEL != 0 {
            return;
        }
        let counter = self.subnet_counter;
        let mut labelled = false;
        let mut stack: Vec<FillState> = Vec::new();
        let mut cur = FillState {
            x: x0,
            z: z0,
            e: e0,
            k: 0,
        };
        'states: loop {
            // Step 2: k = cur.k … 3.
            while cur.k < side::COUNT {
                let k = cur.k;
                if self.car_bit(cur.x, cur.z, k + 4 * cur.e) {
                    self.set_subnet(cur.x, cur.z, cur.e, counter);
                    labelled = true;
                    if let Some(next) = self.next_state(cur, k) {
                        stack.push(FillState { k: k + 1, ..cur });
                        cur = next;
                        continue 'states;
                    }
                }
                cur.k += 1;
            }
            // Step 3: after k = 3, a tunnel cell whose record allows Car. A
            // state pushed here resumes after this step (`AFTER_TUNNEL`, k =
            // 5, set before the tunnel lookup: TN16), so it never redoes it.
            if cur.k == side::COUNT && self.wormhole_at(cur.x, cur.z) & wormhole::TUNNEL != 0 {
                let other = self
                    .tunnels
                    .get(key32(cur.x, cur.z))
                    .filter(|t| t.mask & (1 << travel::CAR) != 0)
                    .map(|t| t.other);
                if let Some((ox, oz)) = other {
                    self.set_subnet(cur.x, cur.z, cur.e, counter);
                    if self.unlabelled(ox, oz, cur.e) {
                        stack.push(FillState { k: AFTER_TUNNEL, ..cur });
                        cur = FillState {
                            x: ox,
                            z: oz,
                            e: cur.e,
                            k: 0,
                        };
                        continue 'states;
                    }
                }
            }
            // Step 4: pop and resume.
            match stack.pop() {
                Some(s) => cur = s,
                None => break,
            }
        }
        if labelled {
            self.subnet_counter = counter.wrapping_add(1);
        }
    }

    /// The next `Fill` state from `cur` through exit `k` (§4.11 step 2).
    fn next_state(&self, cur: FillState, k: usize) -> Option<FillState> {
        let (nx, nz) = (cur.x + DX[k], cur.z + DZ[k]);
        if !self.in_city(nx, nz) {
            // Off-map: turn around in the same cell (entry k) if the cell
            // also connects k back to e.
            return (self.car_bit(cur.x, cur.z, cur.e + 4 * k) && self.unlabelled(cur.x, cur.z, k)).then_some(
                FillState {
                    x: cur.x,
                    z: cur.z,
                    e: k,
                    k: 0,
                },
            );
        }
        let back = OPPOSITE_SIDE[k];
        let at = |x: i32, z: i32| FillState { x, z, e: back, k: 0 };
        if let Some(sub) = self.subnet_of(nx, nz, back) {
            return (sub == 0).then_some(at(nx, nz));
        }
        // No car record: skip straight across a switch lot.
        let id = self.switch_id_at(nx, nz);
        if id == 0 {
            return None;
        }
        let (mut cx, mut cz) = (nx, nz);
        loop {
            let (fx, fz) = (cx + DX[k], cz + DZ[k]);
            if self.in_city(fx, fz) && self.switch_id_at(fx, fz) == id {
                (cx, cz) = (fx, fz);
                continue;
            }
            return self.unlabelled(fx, fz, back).then_some(at(fx, fz));
        }
    }

    /// The distinct non-zero `subnet[0…3]` of the first car record of each
    /// ring cell of a lot rectangle, in order of discovery (§4.12 steps 1,
    /// 2): the boundary of the rectangle grown by 1 and clamped, in
    /// `BoundaryIterator` order (x fastest: the first row, each middle row's
    /// two ends, the last row; TN7), without the four corner cells.
    pub fn lot_ring_labels(&self, r: super::CellRect) -> Vec<u32> {
        let ring = self.grow_clamp(r, 1);
        let mut ids = Vec::new();
        for (x, z) in boundary_cells(ring) {
            let x_out = x < r.x1 || x > r.x2;
            let z_out = z < r.z1 || z > r.z2;
            if x_out && z_out {
                continue;
            }
            if let Some(info) = self.car_info(x, z) {
                for id in info.subnet.iter().map(|&id| u32::from(id)) {
                    if id != 0 && !ids.contains(&id) {
                        ids.push(id);
                    }
                }
            }
        }
        ids
    }
}

/// `info += c` element-wise (u32, wrapping [choice]).
fn add_capacity(info: &mut SubnetworkInfo, c: &[u32; 4]) {
    for (a, b) in info.capacity.iter_mut().zip(c) {
        *a = a.wrapping_add(*b);
    }
}

impl Cx<'_> {
    /// Entering state 13 (§4.12): the working vector cleared and resized to
    /// `counter` default entries. The scanning region is reset by the caller:
    /// in the work cycle it is the simulator's own scanning iterator (the
    /// member `+0x818`, TS64), and DoPostCityInit uses a local one (§1 step
    /// 7).
    /// `counter` default entries, the scan reset to the whole city. This
    /// form keeps its own cell cursor: DoPostCityInit's local iterator
    /// (§4.12, TS64). The work cycle's state 13 steps the simulator's
    /// scanning iterator instead ([`Self::reset_subnetwork_working`],
    /// [`Self::subnetwork_info_cell`], [`Self::finish_subnetwork_info`]).
    pub(crate) fn begin_subnetwork_info(&mut self) {
        self.reset_subnetwork_working();
        self.s.subnet_info_cursor = Some(0);
    }

    /// §4.12, one cell of the scan: `lot = GetLot(x, z, true)` (lot manager
    /// slot 6; only the lot's anchor cell answers, so each lot is visited
    /// once).
    /// Step 1 of entering state 13 (§4.12): the working vector cleared and
    /// resized to `counter` default entries.
    pub(crate) fn reset_subnetwork_working(&mut self) {
        self.s.subnet_working = vec![SubnetworkInfo::default(); usize::from(self.s.subnet_counter)];
    }

    /// §4.12 for the cell (x, z) of the scan: the lot anchored there, if
    /// any.
    pub(crate) fn subnetwork_info_cell(&mut self, x: i32, z: i32) {
        if let Some(lot) = self.h.lot_at(x, z, true) {
            self.accumulate_lot(lot);
        }
    }

    /// §4.12 after the last cell: the neighbour connections are added and the
    /// working vector is copied to the published one. All queries read the
    /// published vector. [TS64] This runs in the same call as the last cell,
    /// with no further deadline test.
    pub(crate) fn finish_subnetwork_info(&mut self) {
        self.accumulate_neighbours();
        self.s.subnet_published = self.s.subnet_working.clone();
        self.s.subnet_info_cursor = None;
    }

    /// Works on §4.12 for at most `max_cells` cells (x inner, z outer) of
    /// the cursor of [`Self::begin_subnetwork_info`]. After the last cell,

    /// adds the neighbour connections and publishes. Returns true when the    /// pass is finished (or none is running).
    pub(crate) fn work_on_subnetwork_info(&mut self, max_cells: usize) -> bool {
        let Some(mut i) = self.s.subnet_info_cursor else {
            return true;
        };
        let total = self.s.cell_count();
        let x_count = self.s.count_x.max(1) as usize;
        let mut done = 0usize;
        while i < total && done < max_cells {
            let (x, z) = ((i % x_count) as i32, (i / x_count) as i32);
            self.subnetwork_info_cell(x, z);
            i += 1;
            done += 1;
        }
        if i < total {
            self.s.subnet_info_cursor = Some(i);
            return false;
        }
        self.finish_subnetwork_info();
        self.s.subnet_info_cursor = None;        true
    }

    /// §4.12 steps 1–4 for one lot.
    fn accumulate_lot(&mut self, lot: u64) {
        let h = self.h;
        let r = h.lot_bounding_rect(lot);
        let ids = self.s.lot_ring_labels(r);
        let holder = h.lot_holder(lot);
        if ids.is_empty() {
            if holder != 0 {
                h.remove_holder_property(holder, prop::LOT_SUBNETWORKS);
            }
            return;
        }
        if holder != 0 {
            let items = ids.iter().map(|&id| Elem::U32(id)).collect();
            h.set_holder_property(holder, prop::LOT_SUBNETWORKS, Variant::array(mem::UINT32, items));
        }
        let pops = h.lot_populations(lot);
        let caps: [u32; 4] = std::array::from_fn(|k| h.max_trip_capacity(holder, k as u32));
        for &id in &ids {
            let Some(info) = self.s.subnet_working.get_mut(id as usize) else {
                continue;
            };
            for (ty, count) in pops.iter().flatten() {
                let slot = info.population.entry(*ty).or_insert(0);
                *slot = slot.wrapping_add(*count);
            }
            add_capacity(info, &caps);
        }
    }

    /// §4.12, after the last cell: every neighbour connection's pool
    /// capacity is added to the label of each labelled entry side of its
    /// in-city cell (not deduplicated *[quirk]*).
    fn accumulate_neighbours(&mut self) {
        let h = self.h;
        for conn in h.all_connections() {
            let (cx, cz) = h.connection_cell(conn);
            // x = −1 → 0; x = X → X−1; else z = −1 → 0; z = Z → Z−1.
            let (mut x, mut z) = (cx, cz);
            if x == -1 {
                x = 0;
            } else if x == self.s.count_x {
                x = self.s.count_x - 1;
            } else if z == -1 {
                z = 0;
            } else if z == self.s.count_z {
                z = self.s.count_z - 1;
            }
            let Some(subnet) = self.s.car_info(x, z).map(|r| r.subnet) else {
                continue;
            };
            // The holder is asked with the off-map cell, the connection's
            // own, not the in-city one (TN6).
            let Some(holder) = h.neighbor_pool_holder_in(&mut *self.s, cx, cz) else {
                continue;
            };
            let c: [u32; 4] = std::array::from_fn(|k| h.max_trip_capacity(holder, k as u32));
            for id in subnet.into_iter().filter(|&id| id != 0) {
                if let Some(info) = self.s.subnet_working.get_mut(usize::from(id)) {
                    add_capacity(info, &c);
                }
            }
        }
    }
}

impl NetworkModel {
    /// The subnetwork clear step (§4.11).
    pub fn clear_subnetworks(&self) {
        self.s.borrow_mut().clear_subnetworks();
    }

    /// `UpdateSubnetworks` (§4.11).
    pub fn update_subnetworks(&self) {
        self.s.borrow_mut().update_subnetworks();
    }

    /// Enters state 13 (§4.12).
    pub fn begin_subnetwork_info(&self) {
        self.run(|cx| cx.begin_subnetwork_info());
    }

    /// Works on state 13 for at most `max_cells` cells (§4.12; the original
    /// slices by its adaptive work-unit counter, which changes only when the
    /// result is published). Returns true when published.
    pub fn work_on_subnetwork_info(&self, max_cells: usize) -> bool {
        self.run(|cx| cx.work_on_subnetwork_info(max_cells))
    }

    /// `UpdateSubnetworkInfo` run synchronously with no time limit, then
    /// published (as DoPostCityInit does, §1 step 7).
    pub fn update_subnetwork_info(&self) {
        self.run(|cx| {
            cx.begin_subnetwork_info();
            cx.work_on_subnetwork_info(usize::MAX);
        });
    }
}
