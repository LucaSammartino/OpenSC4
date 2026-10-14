// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The water pathfinder (`cSC4WaterPathFinder`, §4.10): the depth map
//! (§4.10.1), the A* `FindPath` (§4.10.2) and the compressed water route
//! format (§4.10.3). In the game this is an internal object of the traffic
//! simulator, not a GZCOM class; FreeCity4 keeps it private to the model
//! (§9).

use std::collections::HashMap;

use crate::x87::{self, rhe};

use super::model::{union_rect, Cx, NetworkModel};
use super::{side, CellRect, NetworkHost, DX, DZ};

/// The step-cost static `0.1` at `0x00B0D490` (§4.10.2 step 4), never
/// written.
const STEP_SCALE: f32 = 0.1;

/// The depth byte of a cell (§4.10.1): 0 when not navigable, `w ≤ 0` or NaN;
/// otherwise `ceil(r24(min(w, 40.0) × 5.0))` in 1…200, with the ceiling from
/// the `1.5·2²³` idiom: `r = round-to-even(r24(0.5 + p))`, and `r − 1` when
/// `p = r − 1` exactly.
pub fn depth_byte(navigable: bool, w: f32) -> u8 {
    if !navigable || w.is_nan() || w <= 0.0 {
        return 0;
    }
    // min takes 40.0 only when 40.0 < w.
    let m = if 40.0 < w { 40.0f32 } else { w };
    let p = x87::r24(f64::from(m) * 5.0) as f32;
    let r = rhe(x87::r24(0.5 + f64::from(p)) as f32);
    let byte = if p == (r - 1) as f32 { r - 1 } else { r };
    byte.clamp(0, 255) as u8
}

/// The constant `0.2` at `0x00AB34F4` (§4.10.2 step 4), a float32 memory
/// operand (`0x3E4CCCCD`, TN14), not the double 0.2.
const DEPTH_SCALE: f32 = 0.2;

/// The A* step into a cell of depth byte `depth` from accumulated `g`
/// (§4.10.2 step 4):
/// `g' = r24(r24(r24(r24(40.0 − r24(depth × 0.2)) × 0.1) + g) + 1.0)`, every
/// constant a float32 (TN14).
pub fn step_cost(depth: u8, g: f32) -> f32 {
    let a = x87::r24(f64::from(depth) * f64::from(DEPTH_SCALE));
    let b = x87::r24(40.0 - a);
    let c = x87::r24(b * f64::from(STEP_SCALE));
    let d = x87::r24(c + f64::from(g));
    x87::r24(d + 1.0) as f32
}

/// The A* heuristic `r24(√ (float)(dx² + dz²))` with integer (dx, dz)
/// (§4.10.2).
pub fn heuristic(dx: i32, dz: i32) -> f32 {
    let sq = x87::r24(f64::from(dx) * f64::from(dx) + f64::from(dz) * f64::from(dz));
    x87::r24(sq.sqrt()) as f32
}

/// The compressed water route (§4.10.3): `[0, x, z]` of the first cell, then
/// runs `[0x40, count, ⌈count/4⌉ bytes]` of unit-step codes (0 = −x, 1 = −z,
/// 2 = +x, 3 = +z, four per byte, least significant first). A run holds at
/// most 255 steps; the 256th starts a new run.
pub fn encode_route(path: &[(i32, i32)]) -> Vec<u8> {
    let mut out = Vec::new();
    let Some(&(x0, z0)) = path.first() else {
        return out;
    };
    out.extend_from_slice(&[0, x0 as u8, z0 as u8]);
    let codes: Vec<u8> = path
        .windows(2)
        .map(|w| {
            let ((ax, az), (bx, bz)) = (w[0], w[1]);
            if bx < ax {
                0
            } else if bz < az {
                1
            } else if bx > ax {
                2
            } else {
                3
            }
        })
        .collect();
    for run in codes.chunks(255) {
        out.push(0x40);
        out.push(run.len() as u8);
        for quad in run.chunks(4) {
            out.push(quad.iter().enumerate().fold(0u8, |b, (i, &c)| b | ((c & 3) << (2 * i))));
        }
    }
    out
}

/// A successful `FindPath` (§4.10.2).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WaterPath {
    /// The number of nodes on the path, both ends included.
    pub cells: u32,
    /// The target node's `g` (diagnostic; the original keeps it in the
    /// node).
    pub g: f32,
    /// The route start → target.
    pub forward: Vec<u8>,
    /// The route target → start.
    pub backward: Vec<u8>,
}

/// One A* node (§4.10.2 step 3).
#[derive(Clone, Copy, Debug)]
struct Node {
    cell: (i32, i32),
    pred: Option<usize>,
    g: f32,
    f: f32,
    open: bool,
    closed: bool,
}

/// The water pathfinder's state (§4.10.1): the depth map (x outer) and its
/// lazy dirty rectangle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaterPathFinder {
    pub count_x: i32,
    pub count_z: i32,
    /// The depth bytes, `x · count_z + z`.
    pub depth: Vec<u8>,
    pub dirty: bool,
    pub dirty_rect: CellRect,
}

impl Default for WaterPathFinder {
    fn default() -> Self {
        WaterPathFinder {
            count_x: 0,
            count_z: 0,
            depth: Vec::new(),
            dirty: false,
            dirty_rect: CellRect::new(0, 0, -1, -1),
        }
    }
}

impl WaterPathFinder {
    /// `Init` (§1 step 6): sizes the depth map (all 0, clean).
    pub fn init(&mut self, count_x: i32, count_z: i32) {
        *self = WaterPathFinder {
            count_x,
            count_z,
            depth: vec![0; (count_x.max(0) as usize) * (count_z.max(0) as usize)],
            ..WaterPathFinder::default()
        };
    }

    fn index(&self, x: i32, z: i32) -> Option<usize> {
        (x >= 0 && z >= 0 && x < self.count_x && z < self.count_z)
            .then(|| x as usize * self.count_z as usize + z as usize)
    }

    /// The depth byte of a cell (0 off-map).
    pub fn depth_at(&self, x: i32, z: i32) -> u8 {
        self.index(x, z).map_or(0, |i| self.depth[i])
    }

    /// Adds a rectangle to the dirty rectangle (§4.10.1), clamped to the map.
    pub fn mark_dirty(&mut self, rect: CellRect) {
        let r = super::grow_clamp(rect, 0, self.count_x, self.count_z);
        if r.x1 > r.x2 || r.z1 > r.z2 {
            return;
        }
        self.union_dirty(r);
    }

    /// `dirty ∪= r` (§4.10.1) with no growing and no clamping: dirty := r
    /// when the flag is clear, else the union; the flag is set.
    pub fn union_dirty(&mut self, r: CellRect) {
        self.dirty_rect = if self.dirty { union_rect(self.dirty_rect, r) } else { r };
        self.dirty = true;
    }

    /// Updates the dirty rectangle from the surface water (§4.10.1), then
    /// clears the flag. *[choice]* (TN24) Only the in-map cells of the
    /// rectangle are asked and written; an unclamped rectangle (TN15) can
    /// reach past the map. The original asks about every cell of it
    /// (`IsNavigable` is false off the map) and writes 0 for each off-map
    /// cell through the raw rows, an out-of-bounds heap write; in-map cells
    /// get the same values either way.
    pub fn update(&mut self, host: &dyn NetworkHost) {
        if !self.dirty {
            return;
        }
        let r = self.dirty_rect;
        for x in r.x1..=r.x2 {
            for z in r.z1..=r.z2 {
                let Some(i) = self.index(x, z) else {
                    continue;
                };
                let navigable = host.is_navigable(x, z);
                let w = if navigable { host.water_depth(x, z) } else { 0.0 };
                self.depth[i] = depth_byte(navigable, w);
            }
        }
        self.dirty = false;
    }

    /// `FindPath(sx, sz, tx, tz, &cells, fwd, back)` (§4.10.2). `None` on
    /// failure (the caller's vectors are then untouched).
    pub fn find_path(&mut self, host: &dyn NetworkHost, sx: i32, sz: i32, tx: i32, tz: i32) -> Option<WaterPath> {
        // Step 1: update a dirty map.
        self.update(host);
        // Step 2: the same non-zero water-body ID at both cells.
        let sb = host.body_of_water(sx, sz).unwrap_or(0);
        let tb = host.body_of_water(tx, tz).unwrap_or(0);
        if sb == 0 || sb != tb {
            return None;
        }
        // Step 3: the node store (reset per call) and the sorted open list.
        let mut nodes: Vec<Node> = Vec::new();
        let mut by_cell: HashMap<(i32, i32), usize> = HashMap::new();
        let mut open: Vec<usize> = Vec::new();
        let mut node_of = |cell: (i32, i32), nodes: &mut Vec<Node>| -> usize {
            *by_cell.entry(cell).or_insert_with(|| {
                nodes.push(Node {
                    cell,
                    pred: None,
                    g: 0.0,
                    f: 0.0,
                    open: false,
                    closed: false,
                });
                nodes.len() - 1
            })
        };
        let start = node_of((sx, sz), &mut nodes);
        nodes[start].f = heuristic(sx - tx, sz - tz);
        nodes[start].open = true;
        insert_open(&mut open, &nodes, start);

        // Step 4: the loop.
        let mut reached = None;
        while !open.is_empty() {
            let cur = open.remove(0);
            nodes[cur].open = false;
            let (cx, cz) = nodes[cur].cell;
            if (cx, cz) == (tx, tz) {
                reached = Some(cur);
                break;
            }
            let (g, pred) = (nodes[cur].g, nodes[cur].pred);
            for d in 0..side::COUNT {
                let (bx, bz) = (cx + DX[d], cz + DZ[d]);
                if bx < 0 || bz < 0 || bx >= self.count_x || bz >= self.count_z {
                    continue;
                }
                let depth = self.depth_at(bx, bz);
                if depth == 0 && (bx, bz) != (tx, tz) {
                    continue;
                }
                if pred.is_some_and(|p| nodes[p].cell == (bx, bz)) {
                    continue;
                }
                let gp = step_cost(depth, g);
                let b = node_of((bx, bz), &mut nodes);
                let nb = nodes[b];
                let better = gp < nb.g; // NaN is "not less"
                if (nb.open || nb.closed) && !better {
                    continue;
                }
                let n = &mut nodes[b];
                n.closed = false;
                n.g = gp;
                n.f = x87::r24(f64::from(heuristic(bx - tx, bz - tz)) + f64::from(gp)) as f32;
                n.pred = Some(cur);
                if nb.open {
                    open.retain(|&i| i != b);
                }
                nodes[b].open = true;
                insert_open(&mut open, &nodes, b);
            }
            nodes[cur].closed = true;
        }

        // Step 5 / success.
        let g = reached.map_or(0.0, |i| nodes[i].g);
        let mut path = Vec::new();
        let mut at = reached;
        while let Some(i) = at {
            path.push(nodes[i].cell);
            at = nodes[i].pred;
        }
        if path.is_empty() {
            return None;
        }
        path.reverse();
        let forward = encode_route(&path);
        path.reverse();
        Some(WaterPath {
            cells: path.len() as u32,
            g,
            forward,
            backward: encode_route(&path),
        })
    }
}

/// Inserts `n` before the first node with a strictly greater `f` (equal `f`
/// is FIFO; a NaN `f` is "not less" and goes last).
fn insert_open(open: &mut Vec<usize>, nodes: &[Node], n: usize) {
    let f = nodes[n].f;
    let pos = open.iter().position(|&i| f < nodes[i].f).unwrap_or(open.len());
    open.insert(pos, n);
}

impl Cx<'_> {
    /// The water pathfinder's part of post city init (§4.10.1, message
    /// `0x26D31EC1`): the whole map dirty and updated at once.
    pub(crate) fn water_post_city_init(&mut self) {
        let all = self.s.city_rect();
        self.s.water.mark_dirty(all);
        self.s.water.update(self.h);
    }

    /// The water pathfinder's part of a network occupant message (§4.10.1,
    /// TN15): r = the occupant's bounding city cells; if the NW cell
    /// `(r.x1, r.z1)` is under water (the only cell tested), dirty ∪= r, not
    /// grown and not clamped (bridges).
    pub(crate) fn water_occupant_changed(&mut self, occ: u64) {
        let r = self.h.occupant_bounding_cells(occ);
        if self.h.cell_under_water(r.x1, r.z1) {
            self.s.water.union_dirty(r);
        }
    }
}

impl NetworkModel {
    /// Terrain changed (`0x278E6003` / `0x678E6E3A`, §4.10.1): dirty ∪=
    /// `(data1 >> 16) − 1 … data1 & 0xFFFF` by
    /// `(data2 >> 16) − 1 … data2 & 0xFFFF`, clamped to the map.
    pub fn water_terrain_changed(&self, data1: u32, data2: u32) {
        let r = CellRect::new(
            (data1 >> 16) as i32 - 1,
            (data2 >> 16) as i32 - 1,
            (data1 & 0xFFFF) as i32,
            (data2 & 0xFFFF) as i32,
        );
        self.s.borrow_mut().water.mark_dirty(r);
    }

    /// `0x6A3B0AFD` (§4.10.1): updates the dirty depth map now.
    pub fn water_update(&self) {
        self.run(|cx| cx.s.water.update(cx.h));
    }

    /// The water pathfinder's `FindPath` between two cells (§4.10.2).
    pub fn find_water_path(&self, sx: i32, sz: i32, tx: i32, tz: i32) -> Option<WaterPath> {
        self.run(|cx| cx.s.water.find_path(cx.h, sx, sz, tx, tz))
    }

    /// `GetWaterRoute(x1, z1, x2, z2, vector<u8>&)` (slot 33): the forward
    /// route into `route` on success; `route` is untouched on failure.
    pub fn get_water_route(&self, x1: i32, z1: i32, x2: i32, z2: i32, route: &mut Vec<u8>) -> bool {
        match self.find_water_path(x1, z1, x2, z2) {
            Some(p) => {
                *route = p.forward;
                true
            }
            None => false,
        }
    }
}
