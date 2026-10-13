// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Neighbour connection edge records (§2.7). Four arrays of 8-byte records,
//! one per map edge, indexed by the coordinate along the edge. The add and
//! remove messages, the pools and the costs belong to traffic-simulator.md;
//! the graph uses these records only through `HasMapExit` (§2.7), which tests
//! mask bit 9 (a water/ferry neighbour connection).

use super::model::{NetworkModel, NetworkState};

/// Which map edge a neighbour connection is on (§2.7). The four arrays are
/// stored in `NetworkState::neighbour_edges` in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    /// x < 0, indexed by z.
    West = 0,
    /// x > X−1, indexed by z.
    East = 1,
    /// z < 0, indexed by x.
    North = 2,
    /// z > Z−1, indexed by x.
    South = 3,
}

/// One neighbour connection edge record (§2.7): a mask of neighbour-connection
/// types (bit = type), a neighbour pool ID and a cost.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NeighbourEdge {
    /// A u16 mask of neighbour-connection types (bit = type). Bit 9 is a
    /// water/ferry connection (§2.7).
    pub mask: u16,
    /// A u16 neighbour pool ID.
    pub pool: u16,
    /// An f32 cost.
    pub cost: f32,
}

impl NetworkState {
    /// The map edge and index an off-map cell (x, z) next to the map belongs
    /// to, or `None` when it is not adjacent to exactly one edge.
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

    /// Sets (or grows to hold) the neighbour edge record at an off-map cell.
    pub fn set_neighbour_edge(&mut self, x: i32, z: i32, rec: NeighbourEdge) {
        let Some((edge, i)) = self.neighbour_edge_index(x, z) else {
            return;
        };
        let arr = &mut self.neighbour_edges[edge as usize];
        if arr.len() <= i {
            arr.resize(i + 1, NeighbourEdge::default());
        }
        arr[i] = rec;
    }
}

impl NetworkModel {
    /// Sets the neighbour connection edge record at an off-map cell (§2.7;
    /// the add and remove messages are traffic-simulator.md's).
    pub fn set_neighbour_edge(&self, x: i32, z: i32, rec: NeighbourEdge) {
        self.s.borrow_mut().set_neighbour_edge(x, z, rec);
    }
}
