// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The compressed route (`GetCompressedRoute`, §4.15) and
//! `WasTripReversible` (§4.16).

use crate::traffic_network::{wormhole, NetworkState};

use super::{edge_to, Pathfinder};

/// One node of a route as the encoder sees it: the node's cell (off-map for
/// an exit node) and its travel type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteNode {
    pub x: i32,
    pub z: i32,
    pub t: u8,
}

/// What the route encoder reads about cells (§4.15).
pub trait RouteMap {
    /// Whether the cell is in the city.
    fn in_city(&self, x: i32, z: i32) -> bool;
    /// Whether the cell has the ferry wormhole bit.
    fn has_ferry(&self, x: i32, z: i32) -> bool;
    /// `GetNeighborConnectionInnerCell(x, z)`: the in-city cell of the
    /// neighbour connection at an off-map point.
    fn inner_cell(&self, x: i32, z: i32) -> Option<(i32, i32)>;
}

impl RouteMap for NetworkState {
    fn in_city(&self, x: i32, z: i32) -> bool {
        NetworkState::in_city(self, x, z)
    }

    fn has_ferry(&self, x: i32, z: i32) -> bool {
        self.wormhole_at(x, z) & wormhole::FERRY != 0
    }

    fn inner_cell(&self, x: i32, z: i32) -> Option<(i32, i32)> {
        self.neighbor_connection_inner_cell(x, z).map(|(c, _)| c)
    }
}

/// The largest run of unit steps (§4.15).
const MAX_RUN: usize = 255;
/// The run record flag.
const RUN_FLAG: u8 = 0x40;
/// The jump record flag for a jump from an off-map or ferry cell.
const FERRY_FLAG: u8 = 0x80;

/// The cell written for a node: an off-map node is written as its
/// connection's in-city cell (without one, its own cell *[choice]*).
fn written_cell(map: &dyn RouteMap, n: &RouteNode) -> (u8, u8) {
    let (x, z) = if map.in_city(n.x, n.z) {
        (n.x, n.z)
    } else {
        map.inner_cell(n.x, n.z).unwrap_or((n.x, n.z))
    };
    (x as u8, z as u8)
}

/// Writes a pending run `[t | 0x40, count, ⌈count/4⌉ packed bytes]`: codes
/// four per byte, least significant bits first.
fn flush(out: &mut Vec<u8>, run: &mut Option<(u8, Vec<u8>)>) {
    if let Some((t, steps)) = run.take() {
        out.push(t | RUN_FLAG);
        out.push(steps.len() as u8);
        for chunk in steps.chunks(4) {
            let b = chunk.iter().enumerate().fold(0u8, |b, (i, &c)| b | (c << (2 * i)));
            out.push(b);
        }
    }
}

/// Encodes a route root → result (§4.15) *[compat]*. Returns the bytes and
/// the mask, the OR of `1 << t` over all nodes.
///
/// - Header: the root's type (no flags), x and z.
/// - Unit steps (one cell along x or z, code = direction 0 −x, 1 −z, 2 +x,
///   3 +z) are collected into runs of one travel type (the type of the node
///   stepped to). A run is flushed when the next unit step has another type,
///   when it holds 255 steps, before a jump and at the end.
/// - Any other move is a jump: flush, then `[t | f, x, z]` of the new node,
///   `f = 0x80` when the previous cell is off-map or has the ferry bit.
pub fn encode_route(nodes: &[RouteNode], map: &dyn RouteMap) -> (Vec<u8>, u32) {
    let mut out = Vec::new();
    let Some(root) = nodes.first() else {
        return (out, 0);
    };
    let mask = nodes
        .iter()
        .fold(0u32, |m, n| m | 1u32.checked_shl(u32::from(n.t)).unwrap_or(0));
    let (hx, hz) = written_cell(map, root);
    out.extend([root.t, hx, hz]);
    let mut run: Option<(u8, Vec<u8>)> = None;
    for w in nodes.windows(2) {
        let (prev, cur) = (&w[0], &w[1]);
        let code = match (cur.x.wrapping_sub(prev.x), cur.z.wrapping_sub(prev.z)) {
            (-1, 0) => Some(0u8),
            (0, -1) => Some(1),
            (1, 0) => Some(2),
            (0, 1) => Some(3),
            _ => None,
        };
        match code {
            Some(c) => {
                let restart = run
                    .as_ref()
                    .is_some_and(|(rt, steps)| *rt != cur.t || steps.len() >= MAX_RUN);
                if restart {
                    flush(&mut out, &mut run);
                }
                run.get_or_insert_with(|| (cur.t, Vec::new())).1.push(c);
            }
            None => {
                flush(&mut out, &mut run);
                let f = if !map.in_city(prev.x, prev.z) || map.has_ferry(prev.x, prev.z) {
                    FERRY_FLAG
                } else {
                    0
                };
                let (x, z) = written_cell(map, cur);
                out.extend([cur.t | f, x, z]);
            }
        }
    }
    flush(&mut out, &mut run);
    (out, mask)
}

impl Pathfinder {
    /// The result route root → result as encoder nodes.
    fn route_nodes(&self) -> Vec<RouteNode> {
        let Some(res) = self.result else {
            return Vec::new();
        };
        let mut v: Vec<RouteNode> = self
            .walk_to_root(res)
            .into_iter()
            .map(|id| {
                let n = self.store.get(id);
                RouteNode { x: n.x, z: n.z, t: n.t }
            })
            .collect();
        v.reverse();
        v
    }

    /// `GetCompressedRoute(out, mask)` (slot 32, §4.15): `out` is cleared
    /// (its buffer kept) and the mask returned is 0 without a result node;
    /// otherwise the route is appended and the mask is the OR of `1 << t`.
    pub fn get_compressed_route(&self, net: &NetworkState, out: &mut Vec<u8>) -> u32 {
        out.clear();
        if self.result.is_none() {
            return 0;
        }
        let (bytes, mask) = encode_route(&self.route_nodes(), net);
        out.extend_from_slice(&bytes);
        mask
    }

    /// `WasTripReversible` (slot 30, §4.16): true without a result node.
    /// Otherwise the walk result → root over (node `c`, predecessor `p`),
    /// carrying `e` (the entry side `c` has in reverse, −1 unknown) and `tn`
    /// (the type of the node after `c`). Turn rules and the allowed mask are
    /// not checked.
    pub fn was_trip_reversible(&self, net: &NetworkState) -> bool {
        let Some(res) = self.result else {
            return true;
        };
        let walk = self.walk_to_root(res);
        let mut e: i32 = -1;
        let mut tn: Option<u8> = None;
        for pair in walk.windows(2) {
            let c = *self.store.get(pair[0]);
            let p = *self.store.get(pair[1]);
            let off_map = !self.in_city(c.x, c.z) || !self.in_city(p.x, p.z);
            let same = (c.x, c.z) == (p.x, p.z);
            let tunnels =
                net.wormhole_at(c.x, c.z) & wormhole::TUNNEL != 0 && net.wormhole_at(p.x, p.z) & wormhole::TUNNEL != 0;
            if off_map || same || tunnels {
                e = -1;
            } else {
                if e != -1 {
                    let rec = net.conn_at(c.x, c.z).copied().unwrap_or_default();
                    let ct = usize::from(c.t);
                    if tn != Some(c.t) {
                        if rec.switch_to(ct, e as usize) == 0xF {
                            return false;
                        }
                    } else if p.t == c.t && rec.conn[ct] & (1 << (4 * e as usize + usize::from(c.side))) == 0 {
                        return false;
                    }
                }
                // EdgeTo(c → p): the side through which a reverse trip
                // enters p.
                e = edge_to((c.x, c.z), (p.x, p.z)) as i32;
            }
            tn = Some(c.t);
        }
        true
    }
}
