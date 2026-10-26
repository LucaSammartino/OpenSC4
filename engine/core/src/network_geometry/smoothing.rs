// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The cell infos (§8.1) and the height solver around the satisfier (§9.2
//! to §9.6): the per-cell slope and smoothness constraints, the per-cell
//! range constraints, `ComputeInitialSmoothingSet`, `SmoothenNetwork`, the
//! smoothing loop and `CanTilesBeSupportedOnTerrain`.

use std::collections::{BTreeMap, BTreeSet};

use super::drag::StepKind;
use super::params::{TypeInfo, TYPE_COUNT};
use super::passes::code;
use super::satisfier::VertexHtConstraintSatisfier;
use super::{GeometryHost, GeometryTerrain, Run};
use crate::x87::r24;

/// The vertex-id offset of a connection node's own vertices (§9.2, §9.3).
pub const CONNECTION_NODE_OFFSET: i32 = 100_000;
/// The unbounded preferred range of a new cell (§8.1).
pub const UNBOUNDED: f32 = 4.25e37;
/// Network flag: a piece on a bridge step (§10.3).
pub const FLAG_BRIDGE: u32 = 0x40000;
/// Network flag: a tunnel portal piece (§10.3).
pub const FLAG_PORTAL: u32 = 0x80000;
/// Network flag: an intersection piece.
pub const FLAG_INTERSECTION: u32 = 0x100000;
/// Network flag: a connection node.
pub const FLAG_CONNECTION_NODE: u32 = 0x200000;
/// The edge patterns of §9.2 that keep a four-sided single-type cell from
/// being levelled.
pub const FOUR_SIDED_PATTERNS: [u32; 2] = [0x0301_0301, 0x0103_0103];
/// "Diagonal or special" edge values (§9.2).
pub const DIAGONAL_EDGE_MASK: u32 = 0xF8F8_F8F8;

/// A cell key `(z << 16) + x` (§2.1).
pub fn cell_key(x: i32, z: i32) -> i32 {
    (z << 16).wrapping_add(x)
}

/// `cSC4NetworkCellInfo` (§8.1), the fields the height solver reads.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CellInfo {
    pub x: i32,
    pub z: i32,
    /// `+0x08`: the piece's network flags.
    pub flags: u32,
    /// `+0x10`: part of the run.
    pub in_run: bool,
    /// `+0x14`: the step index (−1 off the run).
    pub step: i32,
    /// `+0x18`–`+0x48`: per-type edge masks, one byte per side. [inferred]
    /// Byte k is the side in direction 2k (0 = −x, 1 = −z, 2 = +x, 3 = +z).
    pub edges: [u32; TYPE_COUNT],
    /// `+0x4C`: the union of the edge masks.
    pub edge_union: u32,
    /// `+0x51`: an existing piece.
    pub placed: bool,
    /// `+0x52`: an intersection piece.
    pub intersection: bool,
    /// `+0x53`: fixed (an immovable piece, or a step of kind ≠ 0).
    pub fixed: bool,
    /// `+0x55`: tunnel interior.
    pub tunnel_interior: bool,
    /// `+0x56`: connection node.
    pub connection_node: bool,
    /// `+0x58`: corner heights, in the order (x, z), (x, z+1), (x+1, z+1),
    /// (x+1, z).
    pub corners: [f32; 4],
    /// `+0x68`: allowed ranges.
    pub allowed: [(f32, f32); 4],
    /// `+0x88`: preferred ranges.
    pub preferred: [(f32, f32); 4],
    /// `+0xA8`: vertex ids (without the connection-node offset).
    pub vertices: [i32; 4],
    /// The piece has support structures (`GetSupportStructureInfo`, §9.6).
    pub has_supports: bool,
    /// A connection node's fixed vertices (ids with the offset) and heights
    /// (the lot's heights and `kEdgesToFixVertex`, §9.3). Empty: the four
    /// corners at their corner heights [choice].
    pub node_fixed: Vec<(i32, f32)>,
}

/// The four corner vertex ids of cell (x, z) with the row stride `s`.
pub fn corner_vertices(x: i32, z: i32, s: i32) -> [i32; 4] {
    [z * s + x, (z + 1) * s + x, (z + 1) * s + x + 1, z * s + x + 1]
}

/// An existing network piece in a cell, from the host (§8.1).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExistingPiece {
    pub flags: u32,
    pub immovable: bool,
    pub corners: [f32; 4],
    pub edges: [u32; TYPE_COUNT],
    pub has_supports: bool,
}

fn add(a: f32, b: f32) -> f32 {
    r24(f64::from(a) + f64::from(b)) as f32
}
fn sub(a: f32, b: f32) -> f32 {
    r24(f64::from(a) - f64::from(b)) as f32
}

impl CellInfo {
    fn with_edges(mut self, edges: [u32; TYPE_COUNT]) -> CellInfo {
        self.edges = edges;
        self.edge_union = edges.iter().fold(0, |a, e| a | e);
        self.intersection = self.flags & FLAG_INTERSECTION != 0;
        self.connection_node = self.flags & FLAG_CONNECTION_NODE != 0;
        self
    }

    /// The number of sides with a non-zero edge byte.
    pub fn connected_sides(&self) -> u32 {
        (0..4).filter(|k| (self.edge_union >> (8 * k)) & 0xFF != 0).count() as u32
    }

    /// The number of network types with edges.
    pub fn type_count(&self) -> u32 {
        self.edges.iter().filter(|&&e| e != 0).count() as u32
    }
}

/// `CreateCellInfosForCurrentPlacementTiles` (slot 71, `0x00633250`, §8.1)
/// for the dragged cells of the valid steps.
pub fn create_cell_infos(
    run: &Run,
    info: &TypeInfo,
    valid: u32,
    t: &dyn GeometryTerrain,
    host: &dyn GeometryHost,
) -> BTreeMap<i32, CellInfo> {
    let mut out = BTreeMap::new();
    let s = t.cells().0 as i32 + 1;
    let type_bit = 1u32 << run.type_id;
    for (si, step) in run.steps.iter().enumerate().take(valid as usize) {
        for c in step.first..=step.last {
            let (x, z) = run.cells[c as usize];
            let vertices = corner_vertices(x, z, s);
            let kind_bits = match step.kind {
                StepKind::Bridge => FLAG_BRIDGE,
                StepKind::Portal => FLAG_PORTAL,
                _ => 0,
            };
            let mut ci = match host.existing_piece(x, z) {
                None => {
                    let corners = vertices.map(|v| t.vertex_altitude(v));
                    let tol = info.params.new_cell_tolerance;
                    CellInfo {
                        flags: type_bit | kind_bits,
                        corners,
                        allowed: corners.map(|h| (sub(h, tol), add(h, tol))),
                        preferred: [(-UNBOUNDED, UNBOUNDED); 4],
                        fixed: step.kind != StepKind::Regular,
                        ..CellInfo::default()
                    }
                    .with_edges([0; TYPE_COUNT])
                }
                Some(p) => {
                    let tol = info.params.resolve_tolerance;
                    CellInfo {
                        flags: p.flags | kind_bits,
                        corners: p.corners,
                        allowed: p.corners.map(|h| (sub(h, tol), add(h, tol))),
                        preferred: p.corners.map(|h| (h, h)),
                        placed: true,
                        fixed: p.immovable || step.kind != StepKind::Regular,
                        has_supports: p.has_supports,
                        ..CellInfo::default()
                    }
                    .with_edges(p.edges)
                }
            };
            ci.x = x;
            ci.z = z;
            ci.in_run = true;
            ci.step = si as i32;
            ci.vertices = vertices;
            ci.tunnel_interior = step.kind == StepKind::TunnelInterior;
            // A network never sinks below the water surface.
            for (k, &v) in vertices.iter().enumerate() {
                let w = t.surface_water_level(v);
                if w.is_nan() {
                    continue;
                }
                if w > ci.allowed[k].0 {
                    ci.allowed[k].0 = w;
                }
                if w > ci.preferred[k].0 {
                    ci.preferred[k].0 = w;
                }
            }
            out.insert(cell_key(x, z), ci);
        }
    }
    out
}

/// `CreatePlaceholderCellInfo` (`0x00632EF0`, §8.1): a cell around the run
/// with a range of ± the new-cell tolerance. [inferred] Corners at the
/// terrain, the preferred range unbounded, no edges. Which cells get one is
/// not specified (Q NG13); callers add them.
pub fn placeholder_cell_info(x: i32, z: i32, info: &TypeInfo, t: &dyn GeometryTerrain) -> CellInfo {
    let s = t.cells().0 as i32 + 1;
    let vertices = corner_vertices(x, z, s);
    let corners = vertices.map(|v| t.vertex_altitude(v));
    let tol = info.params.new_cell_tolerance;
    CellInfo {
        x,
        z,
        step: -1,
        vertices,
        corners,
        allowed: corners.map(|h| (sub(h, tol), add(h, tol))),
        preferred: [(-UNBOUNDED, UNBOUNDED); 4],
        ..CellInfo::default()
    }
}

/// [choice] The run's own edges until the piece solver (NG5) exists: two
/// consecutive steps whose cells share a side get edge value 1 on that side
/// of both cells, for the run's type. Diagonal moves get no edge.
pub fn assign_run_edges(run: &Run, valid: u32, cells: &mut BTreeMap<i32, CellInfo>) {
    let t = run.type_id as usize;
    let side = |dx: i32, dz: i32| -> Option<u32> {
        match (dx, dz) {
            (-1, 0) => Some(0),
            (0, -1) => Some(1),
            (1, 0) => Some(2),
            (0, 1) => Some(3),
            _ => None,
        }
    };
    let n = (valid as usize).min(run.steps.len());
    for i in 1..n {
        let (a, b) = (run.steps[i - 1], run.steps[i]);
        for k in 0..=(a.last - a.first).min(b.last - b.first) {
            let p = run.cells[(a.first + k) as usize];
            let q = run.cells[(b.first + k) as usize];
            if let Some(sd) = side(q.0 - p.0, q.1 - p.1) {
                let back = (sd + 2) & 3;
                for (cell, s) in [(p, sd), (q, back)] {
                    if let Some(ci) = cells.get_mut(&cell_key(cell.0, cell.1)) {
                        if ci.edges[t] >> (8 * s) & 0xFF == 0 {
                            ci.edges[t] |= 1 << (8 * s);
                        }
                        ci.edge_union = ci.edges.iter().fold(0, |acc, e| acc | e);
                    }
                }
            }
        }
    }
}

/// A vertex id from vertex coordinates; outside the grid → −1 [choice: the
/// spec only says constraints with a negative id are skipped].
fn vid(vx: i32, vz: i32, cells: (u32, u32)) -> i32 {
    if vx < 0 || vz < 0 || vx > cells.0 as i32 || vz > cells.1 as i32 {
        -1
    } else {
        vz * (cells.0 as i32 + 1) + vx
    }
}

/// The two vertices (as coordinates) of side `k` of cell (x, z).
fn side_vertices(x: i32, z: i32, k: u32) -> [(i32, i32); 2] {
    match k {
        0 => [(x, z), (x, z + 1)],
        1 => [(x, z), (x + 1, z)],
        2 => [(x + 1, z), (x + 1, z + 1)],
        _ => [(x, z + 1), (x + 1, z + 1)],
    }
}

/// The cross-sections of one side (`GetCrossSections`, `0x00636360`):
/// previous (the opposite side of this cell), this (the side), next (the
/// side shifted one cell outward). [inferred] Q NG14.
// SPEC-GAP: NG15 asks for the exact cross-section vertex pairs and constraint insertion order on nonflat multi-cell runs.
pub fn cross_sections(x: i32, z: i32, side: u32, off: i32, cells: (u32, u32)) -> [[i32; 2]; 3] {
    let (dx, dz) = [(-1, 0), (0, -1), (1, 0), (0, 1)][side as usize & 3];
    let own = |vs: [(i32, i32); 2]| {
        vs.map(|(a, b)| vid(a, b, cells))
            .map(|v| if v < 0 { v } else { v + off })
    };
    let this = side_vertices(x, z, side & 3);
    let prev = side_vertices(x, z, (side + 2) & 3);
    let next = this.map(|(a, b)| vid(a + dx, b + dz, cells));
    [own(prev), own(this), next]
}

fn ok(vs: &[i32]) -> bool {
    vs.iter().all(|&v| v >= 0) && (0..vs.len()).all(|i| (i + 1..vs.len()).all(|j| vs[i] != vs[j]))
}

/// `InsertSlopeAndSmoothnessConstraintsForCell` (`0x00636690`, §9.2).
/// `info` is the type whose limits apply ([inferred] the run's type) and
/// `elevated` whether the cell's type has a non-empty p+0xA0 list.
pub fn insert_slope_and_smoothness_constraints(
    sat: &mut VertexHtConstraintSatisfier,
    cell: &CellInfo,
    info: &TypeInfo,
    elevated: bool,
    cells: (u32, u32),
) {
    if cell.intersection || cell.fixed {
        return;
    }
    let off = if cell.connection_node {
        CONNECTION_NODE_OFFSET
    } else {
        0
    };
    let sides = cell.connected_sides();
    let union = cell.edge_union;
    let four_sided = cell.type_count() == 1 && FOUR_SIDED_PATTERNS.contains(&union);
    let diagonal = union & DIAGONAL_EDGE_MASK != 0;
    let flat = (sides != 1 && sides != 2 && !four_sided) || (elevated && sides == 2 && diagonal);
    let eq = |sat: &mut VertexHtConstraintSatisfier, a: i32, b: i32| {
        if ok(&[a, b]) {
            sat.insert_equality(a, b);
        }
    };
    if flat {
        // [inferred] Corner 0 = 1, 1 = 2, 2 = 3.
        let v = cell.vertices.map(|v| v + off);
        for k in 0..3 {
            eq(sat, v[k], v[k + 1]);
        }
        return;
    }
    let (rise, smooth) = if diagonal {
        (info.diagonal_slope_rise, info.diagonal_smoothness)
    } else {
        (info.slope_rise, info.smoothness)
    };
    for side in 0..4u32 {
        if (union >> (8 * side)) & 0xFF == 0 {
            continue;
        }
        let [prev, this, next] = cross_sections(cell.x, cell.z, side, off, cells);
        for cs in [prev, this, next] {
            eq(sat, cs[0], cs[1]);
        }
        if ok(&[prev[0], this[0]]) {
            sat.insert_slope(prev[0], this[0], rise);
        }
        if ok(&[this[0], next[0]]) {
            sat.insert_slope(this[0], next[0], rise);
        }
        if ok(&[prev[0], this[0], next[0]]) {
            sat.insert_smoothness(prev[0], this[0], next[0], smooth);
        }
    }
}

/// `InsertRangeConstraintsForCell` (slot 76, `0x006360C0`, §9.3).
/// Boundary vertices are fixed at the terrain altitude ([inferred]: §9.4
/// "held at the terrain").
pub fn insert_range_constraints(
    sat: &mut VertexHtConstraintSatisfier,
    cell: &CellInfo,
    boundary: &BTreeSet<i32>,
    t: &dyn GeometryTerrain,
) {
    if cell.connection_node {
        if cell.node_fixed.is_empty() {
            for k in 0..4 {
                sat.insert_fixed(cell.vertices[k] + CONNECTION_NODE_OFFSET, cell.corners[k]);
            }
        } else {
            for &(v, h) in &cell.node_fixed {
                sat.insert_fixed(v, h);
            }
        }
        return;
    }
    if cell.fixed {
        for k in 0..4 {
            sat.insert_fixed(cell.vertices[k], cell.corners[k]);
        }
        return;
    }
    for k in 0..4 {
        let v = cell.vertices[k];
        if boundary.contains(&v) {
            sat.insert_fixed(v, t.vertex_altitude(v));
            continue;
        }
        sat.add_vertex(v, cell.corners[k]);
        sat.insert_range(v, cell.allowed[k].0, cell.allowed[k].1);
        sat.set_preferred_range(v, cell.preferred[k].0, cell.preferred[k].1);
        if cell.placed {
            sat.set_preferred_value(v, cell.corners[k]);
        }
    }
}

/// The smoothing set (`+0xA4`, cell keys in key order, [choice] the
/// original iterates a hash set) and the boundary set (vertex ids).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SmoothingSet {
    pub cells: Vec<i32>,
    pub boundary: BTreeSet<i32>,
}

/// `ComputeInitialSmoothingSet(level)` (`0x00637720`, §9.4 step 1).
/// `solved` are the solved cells (the piece solver's, NG5).
pub fn compute_initial_smoothing_set(
    run: &Run,
    solved: &[(i32, i32)],
    infos: &BTreeMap<i32, CellInfo>,
    level: i32,
    cells: (u32, u32),
) -> SmoothingSet {
    let (cx, cz) = (cells.0 as i32, cells.1 as i32);
    // Per row z: the inclusive span of x.
    let mut rows: BTreeMap<i32, (i32, i32)> = BTreeMap::new();
    let straight = run.dragged_count == run.straight_count && run.straight_count as usize == solved.len();
    if straight {
        if let Some(&(x0, z0)) = run.cells.first() {
            let (mut a, mut b) = ((x0, z0), (x0, z0));
            for &(x, z) in &run.cells {
                a = (a.0.min(x), a.1.min(z));
                b = (b.0.max(x), b.1.max(z));
            }
            let (x0, x1) = ((a.0 - level).max(0), (b.0 + level).min(cx - 1));
            for z in (a.1 - level).max(0)..=(b.1 + level).min(cz - 1) {
                rows.insert(z, (x0, x1));
            }
        }
    } else {
        // [inferred] The union per row is kept as one span (min, max).
        for &(x, z) in solved {
            let (x0, x1) = ((x - level).max(0), (x + level).min(cx - 1));
            for r in (z - level).max(0)..=(z + level).min(cz - 1) {
                let e = rows.entry(r).or_insert((x0, x1));
                *e = (e.0.min(x0), e.1.max(x1));
            }
        }
    }
    let in_region = |x: i32, z: i32| rows.get(&z).is_some_and(|&(a, b)| a <= x && x <= b);
    let mut out = SmoothingSet::default();
    for (&z, &(x0, x1)) in &rows {
        for x in x0..=x1 {
            if let Some(ci) = infos.get(&cell_key(x, z)) {
                if !ci.tunnel_interior {
                    out.cells.push(cell_key(x, z));
                }
            }
            // The region's border vertices that are not on the city's edge.
            for (vx, vz) in [(x, z), (x, z + 1), (x + 1, z + 1), (x + 1, z)] {
                if vx == 0 || vz == 0 || vx == cx || vz == cz {
                    continue;
                }
                let interior =
                    in_region(vx - 1, vz - 1) && in_region(vx, vz - 1) && in_region(vx - 1, vz) && in_region(vx, vz);
                if !interior {
                    out.boundary.insert(vz * (cx + 1) + vx);
                }
            }
        }
    }
    out.cells.sort_unstable();
    out
}

/// `SmoothenNetwork(level)` (`0x00637A80`, §9.4). The intersection, bridge
/// and tunnel elevation and plane constraints are the host's
/// (`extra_network_constraints`, NG2, NG5). Returns `Solve()`.
#[allow(clippy::too_many_arguments)]
pub fn smoothen_network(
    sat: &mut VertexHtConstraintSatisfier,
    set: &SmoothingSet,
    infos: &BTreeMap<i32, CellInfo>,
    info: &TypeInfo,
    types: &[TypeInfo],
    t: &dyn GeometryTerrain,
    host: &mut dyn GeometryHost,
    level: i32,
) -> bool {
    sat.shutdown();
    let top = r24(f64::from(t.max_allowed_height()) + 20.0) as f32;
    sat.init(0.0, top);
    let cells = t.cells();
    for key in &set.cells {
        let Some(ci) = infos.get(key) else {
            continue;
        };
        insert_range_constraints(sat, ci, &set.boundary, t);
        let elevated =
            (0..TYPE_COUNT).any(|k| ci.edges[k] != 0 && types.get(k).is_some_and(|ti| ti.is_elevated_type()));
        insert_slope_and_smoothness_constraints(sat, ci, info, elevated, cells);
    }
    host.extra_network_constraints(sat, level);
    sat.solve()
}

/// What §9.6 step 3 does with one terrain vertex.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TerrainVertex {
    /// Fix the vertex at this height.
    Fix(f32),
    /// Insert the range and the preferred value.
    Range { lo: f32, hi: f32, preferred: f32 },
    /// The terrain cannot support the network: the test fails.
    Unsupported,
}

/// §9.6 step 3 for one vertex: network height `n`, terrain altitude `t`.
pub fn terrain_vertex_constraint(n: f32, t: f32, has_supports: bool, fixed: bool, info: &TypeInfo) -> TerrainVertex {
    let p = &info.params;
    let raise = add(t, p.max_raise);
    let mut hi = if raise < n { raise } else { n };
    let mut lo = sub(t, p.max_cut);
    if has_supports {
        let m = sub(n, p.max_support_height);
        if m > lo {
            lo = m;
        }
        if hi < lo {
            return TerrainVertex::Unsupported;
        }
    } else if !fixed {
        if hi < n {
            return TerrainVertex::Unsupported;
        }
        lo = n;
        hi = n;
    }
    if lo == hi {
        TerrainVertex::Fix(lo)
    } else {
        TerrainVertex::Range {
            lo,
            hi,
            preferred: if fixed { t } else { n },
        }
    }
}

/// `CanTilesBeSupportedOnTerrain(type)` (slot 56, `0x00644B20`, §9.6) for
/// the base tool, on the terrain satisfier `ter`. The intersection
/// constraints are the host's (NG5).
pub fn can_tiles_be_supported(
    ter: &mut VertexHtConstraintSatisfier,
    net: &VertexHtConstraintSatisfier,
    set: &SmoothingSet,
    infos: &BTreeMap<i32, CellInfo>,
    info: &TypeInfo,
    t: &dyn GeometryTerrain,
    host: &mut dyn GeometryHost,
) -> bool {
    ter.shutdown();
    ter.init(-f32::MAX, f32::MAX);
    for key in &set.cells {
        let Some(ci) = infos.get(key) else {
            continue;
        };
        if ci.connection_node {
            continue;
        }
        for k in 0..4 {
            let v = ci.vertices[k];
            let n = net.height(v).unwrap_or(ci.corners[k]);
            let a = t.vertex_altitude(v);
            ter.add_vertex(v, a);
            if set.boundary.contains(&v) || (ci.fixed && ci.placed) {
                ter.insert_fixed(v, a);
                continue;
            }
            match terrain_vertex_constraint(n, a, ci.has_supports, ci.fixed, info) {
                TerrainVertex::Unsupported => return false,
                TerrainVertex::Fix(h) => ter.insert_fixed(v, h),
                TerrainVertex::Range { lo, hi, preferred } => {
                    ter.insert_range(v, lo, hi);
                    ter.set_preferred_value(v, preferred);
                }
            }
        }
    }
    host.extra_terrain_constraints(ter);
    for &v in &set.boundary {
        if !ter.contains(v) {
            ter.insert_fixed(v, t.vertex_altitude(v));
        }
    }
    ter.solve()
}

/// The smoothing loop (in `PlaceNetwork`, §9.4): for level = (1 with a
/// bridge, else 0) to 3, `SmoothenNetwork(level)` then slot 56. `Ok(level)`
/// on success; else the **last** level's code *[compat]*.
pub fn smoothing_loop(has_bridge: bool, mut try_level: impl FnMut(i32) -> (bool, bool)) -> Result<i32, u32> {
    let mut failure = code::SMOOTHING;
    for level in i32::from(has_bridge)..=3 {
        let (smoothed, supported) = try_level(level);
        if !smoothed {
            failure = code::SMOOTHING;
        } else if !supported {
            failure = code::TERRAIN_SUPPORT;
        } else {
            return Ok(level);
        }
    }
    Err(failure)
}
