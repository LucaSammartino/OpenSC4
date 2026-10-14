// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Neighbour connection edge records (§2.7). Four arrays of 8-byte records,
//! one per map edge, indexed by the coordinate along the edge, named by the
//! side `d` of pathfinder.md §4.1 (also the order the loops visit them). The
//! graph uses the records through `HasMapExit` (§2.7), which tests mask bit 9
//! (a water/ferry neighbour connection); the pathfinder reads the cost
//! through `GetTrafficNeighborConnection` (§2.7.1, pathfinder.md §4.13).
//!
//! The record-level parts of §2.7.1–§2.7.4 are here: the lookups, the record
//! writes of Add and Remove, the pool IDs the sweep keeps and the records the
//! edge pass visits. The parts that need other objects stay with the traffic
//! simulator (PF6: "host methods of the traffic simulator"): asking the
//! neighbours simulator for the connection and the other side's average trip
//! length (Add steps 1 and 3), choosing or creating the pool (Add step 5), the
//! pool map and its property holders (§2.7.3), the capacities (§2.7.4 steps
//! 1–5), the destination bits (Remove step 3, §2.7.4 step 6) and the neighbour
//! roll (§2.7.5).

use std::collections::BTreeSet;

use super::model::{NetworkModel, NetworkState};
use super::NET_OF_TRAVEL;

/// Which map edge a neighbour connection is on (§2.7), numbered by the side
/// `d` of pathfinder.md §4.1. The four arrays are stored in
/// `NetworkState::neighbour_edges` in this order, the order in which the pool
/// sweep (§2.7.3) and the edge pass (§2.7.4 step 6) visit them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    /// Side 0: x < 0, indexed by z (Z records).
    West = 0,
    /// Side 1: z < 0, indexed by x (X records).
    North = 1,
    /// Side 2: x > X−1, indexed by z (Z records).
    East = 2,
    /// Side 3: z > Z−1, indexed by x (X records).
    South = 3,
}

impl Edge {
    /// The four edges in side order 0…3.
    pub const ALL: [Edge; 4] = [Edge::West, Edge::North, Edge::East, Edge::South];

    /// The off-map cell of record `j` of this edge in an `X × Z` city:
    /// (−1, j), (j, −1), (X, j) or (j, Z) (§2.7.4 step 6).
    pub fn off_map_cell(self, j: i32, count_x: i32, count_z: i32) -> (i32, i32) {
        match self {
            Edge::West => (-1, j),
            Edge::North => (j, -1),
            Edge::East => (count_x, j),
            Edge::South => (j, count_z),
        }
    }
}

/// `connMask(n)` (`0x00AB322C`, u32[6]): the neighbour-connection types a
/// trip on transit network `n` can use (§2.7.1) *[compat]*. Types: 1 Rail,
/// 2 Road, 3 Highway, 6 Subway, 7 LightRail, 8 Avenue, 9 Ferry, 10 DirtRoad,
/// 11 Monorail, 12 GroundHighway.
pub const CONN_MASK: [u16; 6] = [0x704, 0x170C, 0x2, 0x40, 0x80, 0x800];

/// The highest neighbour-connection type `FindNeighborConnection` asks
/// about (§2.7.1: `c` = 0…12).
pub const MAX_NEIGHBOUR_CONNECTION_TYPE: u32 = 12;

/// The neighbour-connection type bit that gives a pool holder to every
/// travel type and counts in `HasMapExit` (Ferry, §2.7, §2.7.1).
pub const FERRY_CONNECTION_BIT: u16 = 1 << 9;

/// "All travel types" for [`NetworkState::neighbor_pool_id`] (the original's
/// `t = −1`, §2.7.1). Only the pool lookup gives it all bits; to
/// [`NetworkState::find_neighbor_connection`] it is a travel type outside
/// 0…8 and wants none (TN27 *[choice]*).
pub const ANY_TRAVEL_TYPE: u32 = u32::MAX;

/// One neighbour connection edge record (§2.7).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NeighbourEdge {
    /// A u16 mask of neighbour-connection types (bit = type, see
    /// [`CONN_MASK`]). Bit 9 is a water/ferry connection.
    pub mask: u16,
    /// A u16 neighbour pool ID; 0 is the default pool.
    pub pool: u16,
    /// An f32 cost: the extra trip length of a trip that ends at the off-map
    /// cell (pathfinder.md §4.13).
    pub cost: f32,
}

impl NetworkState {
    /// Init (§2.7): all four arrays with every record `(0, 0, 0.0)`, Z
    /// records on sides 0 and 2, X records on sides 1 and 3. The records are
    /// not saved; DoPostCityInit rebuilds them (§2.7.2).
    pub fn init_neighbour_edges(&mut self) {
        let x = self.count_x.max(0) as usize;
        let z = self.count_z.max(0) as usize;
        let rec = NeighbourEdge::default();
        self.neighbour_edges = [vec![rec; z], vec![rec; x], vec![rec; z], vec![rec; x]];
    }

    /// The map edge and index of an off-map cell (x, z) next to the map, in
    /// `GetTrafficNeighborConnection`'s order (§2.7.1): x < 0 → side 0 at z;
    /// x > X−1 → side 2 at z; z < 0 → side 1 at x; otherwise side 3 at x.
    /// `None` when the cell is not next to exactly one edge (a corner or an
    /// in-city cell).
    pub fn neighbour_edge_index(&self, x: i32, z: i32) -> Option<(Edge, usize)> {
        let x_off = x < 0 || x >= self.count_x;
        let z_off = z < 0 || z >= self.count_z;
        if x_off == z_off {
            return None;
        }
        if x < 0 {
            Some((Edge::West, z as usize))
        } else if x >= self.count_x {
            Some((Edge::East, z as usize))
        } else if z < 0 {
            Some((Edge::North, x as usize))
        } else {
            Some((Edge::South, x as usize))
        }
    }

    /// The neighbour edge record at an off-map cell (x, z), if the array has
    /// one at that index.
    pub fn neighbour_edge_at(&self, x: i32, z: i32) -> Option<&NeighbourEdge> {
        let (edge, i) = self.neighbour_edge_index(x, z)?;
        self.neighbour_edges[edge as usize].get(i)
    }

    /// The record at an off-map cell for writing, growing its array to hold
    /// it; `None` for a corner or in-city cell.
    fn neighbour_edge_mut(&mut self, x: i32, z: i32) -> Option<&mut NeighbourEdge> {
        let (edge, i) = self.neighbour_edge_index(x, z)?;
        let arr = &mut self.neighbour_edges[edge as usize];
        if arr.len() <= i {
            arr.resize(i + 1, NeighbourEdge::default());
        }
        Some(&mut arr[i])
    }

    /// Sets (or grows to hold) the neighbour edge record at an off-map cell.
    pub fn set_neighbour_edge(&mut self, x: i32, z: i32, rec: NeighbourEdge) {
        if let Some(r) = self.neighbour_edge_mut(x, z) {
            *r = rec;
        }
    }

    /// `GetTrafficNeighborConnection(x, z)` (§2.7.1, `0x0070F970`): the
    /// record of an off-map cell. The original never fails and checks no
    /// range (a corner cell reads index −1 or Z of the side-0 or side-2
    /// array, heap memory; an in-city cell gets side-3 record x, the record
    /// of (x, Z), which it aliases); every game caller passes an off-map edge
    /// cell. *[choice]* (TN28, kept: never observable) Any other cell, or an
    /// index past its array, gives an empty record (mask 0, pool 0, cost
    /// 0.0), and writes to it are discarded.
    pub fn traffic_neighbor_connection(&self, x: i32, z: i32) -> NeighbourEdge {
        self.neighbour_edge_at(x, z).copied().unwrap_or_default()
    }

    /// `FindNeighborConnection(t, x, z)` (§2.7.1, `0x007104A0`): the
    /// neighbour connection a trip of travel type `t` uses at an off-map
    /// cell. None for an in-city cell. Otherwise, for connection type `c` =
    /// 0…12 ascending with bit `c` in both `connMask(net(t))` and the cell's
    /// record mask, `find(c)` asks the neighbours simulator's
    /// `FindConnection(c, x, z)` (slot 11); the first connection found is the
    /// result. *[choice]* (TN27) A travel type outside 0…8, [`ANY_TRAVEL_TYPE`]
    /// included, wants no connection type, so the result is none; the
    /// original indexes its tables unchecked (t = 9…26 read the pedestrian
    /// mask), but every caller passes 0…8.
    pub fn find_neighbor_connection<T>(
        &self,
        t: u32,
        x: i32,
        z: i32,
        mut find: impl FnMut(u32) -> Option<T>,
    ) -> Option<T> {
        if self.in_city(x, z) {
            return None;
        }
        let wanted = wanted_connection_types(t);
        let rec = self.traffic_neighbor_connection(x, z);
        (0..=MAX_NEIGHBOUR_CONNECTION_TYPE)
            .filter(|&c| wanted & rec.mask & (1 << c) != 0)
            .find_map(&mut find)
    }

    /// The pool part of `GetNeighborPoolPropertyHolder(t, x, z)` (§2.7.1,
    /// `0x007114E0`): `None` (a null holder) when the record's mask has none
    /// of the wanted connection types (all for [`ANY_TRAVEL_TYPE`], else
    /// `connMask(net(t))`, none for another t outside 0…8, TN27 *[choice]*)
    /// and no Ferry bit; otherwise the pool ID, 0 for the default pool. A
    /// record whose pool ID names no pool (`pool_exists` false) is reset to
    /// pool 0 *[compat]* and gives the default pool. The caller maps the ID
    /// to its pool's property holder. A corner or in-city cell has the empty
    /// record (TN28), so the result is `None`.
    pub fn neighbor_pool_id(&mut self, t: u32, x: i32, z: i32, pool_exists: impl Fn(u16) -> bool) -> Option<u16> {
        let wanted = if t == ANY_TRAVEL_TYPE {
            u16::MAX
        } else {
            wanted_connection_types(t)
        };
        let rec = self.traffic_neighbor_connection(x, z);
        if rec.mask & wanted == 0 && rec.mask & FERRY_CONNECTION_BIT == 0 {
            return None;
        }
        if rec.pool == 0 {
            return Some(0);
        }
        if pool_exists(rec.pool) {
            return Some(rec.pool);
        }
        if let Some(r) = self.neighbour_edge_mut(x, z) {
            r.pool = 0;
        }
        Some(0)
    }

    /// The record writes of `AddNeighborConnection(type, x, z)` (§2.7.2,
    /// `0x007153C0`), once the caller has found the connection (step 1; no
    /// connection → no call), chosen the cost candidate `f` (step 3:
    /// `commuteMaxTime`, or the other side's `0xCA947077`) and the pool ID
    /// (step 5: 0 without an established other-side city): `mask |= 1 <<
    /// type`; `cost := f` if `cost < f` (ordered: a NaN on either side
    /// changes nothing, and the cost never goes down *[quirk]*); `pool :=
    /// pool`. A corner or in-city cell changes nothing *[choice]* (TN28: the
    /// original would write heap memory or the aliased side-3 record).
    pub fn add_neighbor_connection_record(&mut self, conn_type: u32, x: i32, z: i32, cost: f32, pool: u16) {
        let Some(rec) = self.neighbour_edge_mut(x, z) else {
            return;
        };
        rec.mask |= 1u16.checked_shl(conn_type).unwrap_or(0);
        if rec.cost < cost {
            rec.cost = cost;
        }
        rec.pool = pool;
    }

    /// The record writes of `RemoveNeighborConnection(type, x, z)` (§2.7.2,
    /// `0x00713AB0`, steps 1–2): `mask &= ~(1 << type)`; when the mask is
    /// then 0, `pool := 0` and the result is true: the caller runs the pool
    /// sweep (§2.7.3) and clears the cell's destination byte (step 3). The
    /// cost is kept.
    pub fn remove_neighbor_connection_record(&mut self, conn_type: u32, x: i32, z: i32) -> bool {
        let Some(rec) = self.neighbour_edge_mut(x, z) else {
            return false;
        };
        rec.mask &= !1u16.checked_shl(conn_type).unwrap_or(0);
        if rec.mask != 0 {
            return false;
        }
        rec.pool = 0;
        true
    }

    /// The records 0 … X−2 of each array, in side order, that the pool sweep
    /// (§2.7.3 step 1) and the edge pass (§2.7.4 step 6) look at: the bound
    /// is X−1 for all four arrays *[quirk]*, so the last record of every
    /// array is never visited. The z-indexed arrays use the x bound, the same
    /// in the game's square cities. *[choice]* (TN26) Each array stops at
    /// `min(X − 1, its length)`: with Z > X records X−1 … Z−1 of sides 0 and
    /// 2 are skipped, as in the original; with Z < X the original reads (and
    /// the edge pass writes) past their end, which this does not.
    fn swept_records(&self) -> impl Iterator<Item = (Edge, i32, &NeighbourEdge)> + '_ {
        let bound = (self.count_x - 1).max(0) as usize;
        Edge::ALL.into_iter().flat_map(move |e| {
            self.neighbour_edges[e as usize]
                .iter()
                .take(bound)
                .enumerate()
                .map(move |(j, r)| (e, j as i32, r))
        })
    }

    /// The pool sweep's step 1 (§2.7.3, `0x007138B0`): the pool IDs of
    /// records 0 … X−2 of each array. The caller releases every pool whose
    /// ID is not in the set; a pool used only by the last cell of an edge is
    /// therefore dropped while still in use *[quirk]*.
    pub fn pool_ids_in_use(&self) -> BTreeSet<u16> {
        self.swept_records()
            .map(|(_, _, r)| r.pool)
            .filter(|&p| p != 0)
            .collect()
    }

    /// The records the edge pass visits (§2.7.4 step 6, `0x00714E60`): side
    /// d = 0…3, index j = 0 … X−2, mask ≠ 0, with the off-map cell of each.
    /// The connection on the last cell of an edge is never visited, so it
    /// never gets destination bits *[quirk]*. The caller tests `HasMapExit`,
    /// picks the holder and sets the destination bits.
    pub fn edge_pass_records(&self) -> Vec<(Edge, (i32, i32), NeighbourEdge)> {
        self.swept_records()
            .filter(|(_, _, r)| r.mask != 0)
            .map(|(e, j, r)| (e, e.off_map_cell(j, self.count_x, self.count_z), *r))
            .collect()
    }
}

/// The connection types a travel type wants (§2.7.1): `connMask(net(t))`;
/// none for a travel type outside 0…8 *[choice]* (TN27). The pool lookup's
/// [`ANY_TRAVEL_TYPE`] is handled by its caller.
fn wanted_connection_types(t: u32) -> u16 {
    NET_OF_TRAVEL
        .get(t as usize)
        .and_then(|&n| CONN_MASK.get(n as usize))
        .copied()
        .unwrap_or(0)
}

impl NetworkModel {
    /// Sets the neighbour connection edge record at an off-map cell (§2.7).
    pub fn set_neighbour_edge(&self, x: i32, z: i32, rec: NeighbourEdge) {
        self.s.borrow_mut().set_neighbour_edge(x, z, rec);
    }

    /// `GetTrafficNeighborConnection(x, z)` (§2.7.1).
    pub fn traffic_neighbor_connection(&self, x: i32, z: i32) -> NeighbourEdge {
        self.s.borrow().traffic_neighbor_connection(x, z)
    }

    /// The record writes of `AddNeighborConnection` (§2.7.2).
    pub fn add_neighbor_connection_record(&self, conn_type: u32, x: i32, z: i32, cost: f32, pool: u16) {
        self.s
            .borrow_mut()
            .add_neighbor_connection_record(conn_type, x, z, cost, pool);
    }

    /// The record writes of `RemoveNeighborConnection` (§2.7.2); true when
    /// the caller must sweep the pools and clear the destination byte.
    pub fn remove_neighbor_connection_record(&self, conn_type: u32, x: i32, z: i32) -> bool {
        self.s.borrow_mut().remove_neighbor_connection_record(conn_type, x, z)
    }

    /// The pool IDs the sweep keeps (§2.7.3 step 1).
    pub fn pool_ids_in_use(&self) -> BTreeSet<u16> {
        self.s.borrow().pool_ids_in_use()
    }

    /// The records the edge pass visits (§2.7.4 step 6).
    pub fn edge_pass_records(&self) -> Vec<(Edge, (i32, i32), NeighbourEdge)> {
        self.s.borrow().edge_pass_records()
    }
}
