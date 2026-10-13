// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The water pathfinder (`cSC4WaterPathFinder`, §4.10): the depth map
//! (§4.10.1), the A* `FindPath` (§4.10.2) and the compressed water route
//! format (§4.10.3). In the game this is an internal object of the traffic
//! simulator, not a GZCOM class; FreeCity4 keeps it private to the model
//! (§9: "a replacement may keep it private").

use crate::grid::CellRect;
use crate::x87::{self, rhe};

use super::model::NetworkState;
use super::{net, side, NetworkHost, DX, DZ};

/// The A* node-store bucket count (§4.10.2): `(key × 0x9E3779B1) mod 3079`.
const NODE_BUCKETS: usize = 3079;
/// The step-cost static `0.1` (§4.10.2 step 4).
const STEP_SCALE: f32 = 0.1;

/// The depth byte of a water depth (§4.10.1): 0 when not navigable, `w ≤ 0`
/// or NaN; otherwise `ceil(r24(min(w, 40.0) × 5.0))` in 1…200, with the
/// ceiling from the `1.5·2²³` idiom.
pub fn depth_byte(navigable: bool, w: f32) -> u8 {
    if !navigable || !(w > 0.0) {
        return 0; // w <= 0 or NaN, or not navigable
    }
    // min takes 40.0 only when 40.0 < w.
    let m = if 40.0 < w { 40.0f32 } else { w };
    let p = x87::r24(f64::from(m) * 5.0) as f32;
    // r = round-to-even(r24(0.5 + p)); r − 1 is used when p = r − 1 exactly.
    let q = x87::r24(0.5 + f64::from(p)) as f32;
    let r = rhe(q);
    let byte = if p == (r - 1) as f32 { r - 1 } else { r };
    byte.clamp(0, 255) as u8
}

/// The A* step cost of a cell of depth byte `d` and accumulated `g`
/// (§4.10.2 step 4):
/// `g' = r24(r24(r24(r24(40.0 − r24(d × 0.2)) × 0.1) + g) + 1.0)`.
pub fn step_cost(depth: u8, g: f32) -> f32 {
    let a = x87::r24(f64::from(depth) * 0.2) as f32;
    let b = x87::r24(40.0 - f64::from(a)) as f32;
    let c = x87::r24(f64::from(b) * f64::from(STEP_SCALE)) as f32;
    let d = x87::r24(f64::from(c) + f64::from(g)) as f32;
    x87::r24(f64::from(d) + 1.0) as f32
}

/// The A* heuristic distance `r24(√ (float)(dx² + dz²))` from a cell to the
/// target, with integer (dx, dz) (§4.10.2).
pub fn heuristic(dx: i32, dz: i32) -> f32 {
    let sq = f64::from(dx) * f64::from(dx) + f64::from(dz) * f64::from(dz);
    x87::r24(sq.sqrt()) as f32
}

/// One A* node (§4.10.2 step 3).
#[derive(Clone, Copy, Debug)]
struct Node {
    key: u32,
    cell: (i32, i32),
    pred: u32,
    g: f32,
    f: f32,
    open: bool,
    closed: bool,
}

/// The water pathfinder's private state (§4.10.1): the depth map and its
/// dirty rectangle.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WaterPathFinder {
    pub count_x: i32,
    pub count_z: i32,
    /// The depth bytes, x outer (`x · count_z + z`).
    pub depth: Vec<u8>,
    pub dirty: bool,
    pub dirty_rect: CellRect,
}

impl WaterPathFinder {
    /// `Init` (§1 step 6): sizes the depth grid.
    pub fn init(&mut self, count_x: i32, count_z: i32) {
        self.count_x = count_x;
        self.count_z = count_z;
        self.depth = vec![0; (count_x.max(0) * count_z.max(0)) as usize];
        self.dirty = false;
        self.dirty_rect = CellRect::new(0, 0, -1, -1);
    }

    fn index(&self, x: i32, z: i32) -> Option<usize> {
        if x >= 0 && z >= 0 && x < self.count_x && z < self.count_z {
            Some(x as usize * self.count_z as usize + z as usize)
        } else {
            None
        }
    }

    /// The depth byte at an in-city cell (0 off-map).
    pub fn depth_at(&self, x: i32, z: i32) -> u8 {
        self.index(x, z).map_or(0, |i| self.depth[i])
    }

    /// Marks a rectangle dirty (§4.10.1): unioned into the dirty rectangle.
    pub fn mark_dirty(&mut self, rect: CellRect) {
        self.dirty_rect = if self.dirty {
            super::model::union_rect(self.dirty_rect, rect)
        } else {
            rect
        };
        self.dirty = true;
    }

    /// Marks the whole map dirty (§4.10.1, message `0x26D31EC1`).
    pub fn mark_all_dirty(&mut self) {
        self.mark_dirty(CellRect::new(0, 0, self.count_x - 1, self.count_z - 1));
    }

    /// Updates the dirty rectangle from the host's surface water and terrain
    /// (§4.10.1), then clears the flag.
    pub fn update(&mut self, host: &dyn NetworkHost) {
        if !self.dirty {
            return;
        }
        let r = clamp_rect(self.dirty_rect, self.count_x, self.count_z);
        for x in r.x1..=r.x2 {
            for z in r.z1..=r.z2 {
                let navigable = host.is_navigable(x, z, 0.0);
                let w = if navigable { host.water_depth(x, z) } else { 0.0 };
                if let Some(i) = self.index(x, z) {
                    self.depth[i] = depth_byte(navigable, w);
                }
            }
        }
        self.dirty = false;
    }

    /// `FindPath(sx, sz, tx, tz, &cells, fwd, back)` (§4.10.2). Returns
    /// `(success, cells, forward route, backward route)`. On failure the
    /// routes are untouched (empty here). `host` supplies the body-of-water
    /// map (step 2) and updates the dirty depth map (step 1).
    pub fn find_path(
        &mut self,
        host: &dyn NetworkHost,
        sx: i32,
        sz: i32,
        tx: i32,
        tz: i32,
    ) -> (bool, u32, Vec<u8>, Vec<u8>) {
        // Step 1: update the dirty map.
        self.update(host);
        // Step 2: both cells must have the same non-zero water-body ID.
        let body = |x: i32, z: i32| host.body_of_water(x, z).unwrap_or(0);
        let sb = body(sx, sz);
        let tb = body(tx, tz);
        if sb == 0 || sb != tb {
            return (false, 0, Vec::new(), Vec::new());
        }

        // Step 3: the node store (a Vec keyed by cell key) and the open list
        // (a sorted vec of node indices: insert before the first strictly
        // greater f, so equal f is FIFO).
        let mut nodes: Vec<Node> = Vec::new();
        let mut by_key: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
        let mut open: Vec<u32> = Vec::new();

        let key_of = |x: i32, z: i32| ((x as u32 & 0xFF) << 8) | (z as u32 & 0xFF);
        let mut get_or_create = |x: i32, z: i32, nodes: &mut Vec<Node>, by_key: &mut std::collections::HashMap<u32, u32>| -> u32 {
            let k = key_of(x, z);
            if let Some(&i) = by_key.get(&k) {
                return i;
            }
            let i = nodes.len() as u32;
            nodes.push(Node {
                key: k,
                cell: (x, z),
                pred: u32::MAX,
                g: 0.0,
                f: 0.0,
                open: false,
                closed: false,
            });
            by_key.insert(k, i);
            i
        };

        // The start node.
        let start = get_or_create(sx, sz, &mut nodes, &mut by_key);
        nodes[start as usize].g = 0.0;
        nodes[start as usize].f = heuristic(sx - tx, sz - tz);
        nodes[start as usize].open = true;
        insert_open(&mut open, &nodes, start);

        // Step 4: the loop.
        let mut result = u32::MAX;
        while !open.is_empty() {
            let cur = open.remove(0);
            nodes[cur as usize].open = false;
            let (cx, cz) = nodes[cur as usize].cell;
            if (cx, cz) == (tx, tz) {
                result = cur;
                break;
            }
            let cur_g = nodes[cur as usize].g;
            let cur_pred = nodes[cur as usize].pred;
            for d in 0..side::COUNT {
                let (bx, bz) = (cx + DX[d], cz + DZ[d]);
                // Skip b outside the city.
                if bx < 0 || bz < 0 || bx >= self.count_x || bz >= self.count_z {
                    continue;
                }
                let is_target = (bx, bz) == (tx, tz);
                let depth = self.depth_at(bx, bz);
                // Skip b if its depth byte is 0, unless b is the target.
                if depth == 0 && !is_target {
                    continue;
                }
                let b = get_or_create(bx, bz, &mut nodes, &mut by_key);
                // Skip b if it is the current node's predecessor.
                if cur_pred != u32::MAX && b == cur_pred {
                    continue;
                }
                let gp = step_cost(depth, cur_g);
                let was_open = nodes[b as usize].open;
                let was_closed = nodes[b as usize].closed;
                if was_open && !(gp < nodes[b as usize].g) {
                    continue;
                }
                if was_closed && !(gp < nodes[b as usize].g) {
                    continue;
                }
                if was_open {
                    remove_open(&mut open, b);
                }
                nodes[b as usize].closed = false;
                nodes[b as usize].g = gp;
                nodes[b as usize].f = x87::r24(f64::from(heuristic(bx - tx, bz - tz)) + f64::from(gp)) as f32;
                nodes[b as usize].pred = cur;
                nodes[b as usize].open = true;
                insert_open(&mut open, &nodes, b);
            }
            nodes[cur as usize].closed = true;
        }

        if result == u32::MAX {
            return (false, 0, Vec::new(), Vec::new());
        }
        // Success: walk the path root → result.
        let mut path: Vec<(i32, i32)> = Vec::new();
        let mut cur = result;
        while cur != u32::MAX {
            path.push(nodes[cur as usize].cell);
            cur = nodes[cur as usize].pred;
        }
        path.reverse();
        let cells = path.len() as u32;
        let fwd = encode_route(&path);
        let mut back: Vec<(i32, i32)> = path.iter().copied().rev().collect();
        let _ = &mut back;
        let backward = encode_route(&back);
        (true, cells, fwd, backward)
    }
}

/// Inserts `n` into the open list before the first node with a strictly
/// greater `f` (equal `f` is FIFO); a NaN `f` is "not less" (§4.10.2 step 3).
fn insert_open(open: &mut Vec<u32>, nodes: &[Node], n: u32) {
    let f = nodes[n as usize].f;
    let pos = open
        .iter()
        .position(|&i| f < nodes[i as usize].f)
        .unwrap_or(open.len());
    open.insert(pos, n);
}

/// Removes `n` from the open list (to reposition it).
fn remove_open(open: &mut Vec<u32>, n: u32) {
    if let Some(p) = open.iter().position(|&i| i == n) {
        open.remove(p);
    }
}

/// The compressed water route (§4.10.3): `[0, x, z]` of the first cell, then
/// runs `[0x40, count, ⌈count/4⌉ bytes]` of unit-step codes (0 = −x, 1 = −z,
/// 2 = +x, 3 = +z, four per byte, least significant first). A run holds at
/// most 255 steps; the 256th starts a new run. All water steps are travel
/// type 0, so a run breaks only at 255.
pub fn encode_route(path: &[(i32, i32)]) -> Vec<u8> {
    let mut out = Vec::new();
    let Some(&(x0, z0)) = path.first() else {
        return out;
    };
    out.extend_from_slice(&[0, (x0 & 0xFF) as u8, (z0 & 0xFF) as u8]);
    // Collect the unit-step codes.
    let mut codes: Vec<u8> = Vec::new();
    for w in path.windows(2) {
        let (ax, az) = w[0];
        let (bx, bz) = w[1];
        let code = if bx < ax {
            0
        } else if bz < az {
            1
        } else if bx > ax {
            2
        } else {
            3
        };
        codes.push(code);
    }
    // Flush in runs of at most 255.
    for run in codes.chunks(255) {
        out.push(0x40);
        out.push(run.len() as u8);
        for byte in run.chunks(4) {
            let mut packed = 0u8;
            for (i, &c) in byte.iter().enumerate() {
                packed |= (c & 3) << (2 * i);
            }
            out.push(packed);
        }
    }
    out
}

/// Clamps a rectangle to the city (`[0, count−1]` on each axis).
fn clamp_rect(r: CellRect, count_x: i32, count_z: i32) -> CellRect {
    CellRect {
        x1: r.x1.max(0),
        z1: r.z1.max(0),
        x2: r.x2.min(count_x - 1),
        z2: r.z2.min(count_z - 1),
    }
}

/// The `0x9E3779B1` node-store hash (§4.10.2), kept for reference and tests.
pub fn node_bucket(key: u32) -> usize {
    (key.wrapping_mul(0x9E37_79B1) % NODE_BUCKETS as u32) as usize
}

impl NetworkState {
    /// Whether a cell is on the car network (used by ferries and the
    /// pathfinder-facing helpers). Exposed here to keep `net` imports local.
    pub fn is_car_network(&self, x: i32, z: i32) -> bool {
        self.network_info(net::CAR, x, z).is_some()
    }
}
