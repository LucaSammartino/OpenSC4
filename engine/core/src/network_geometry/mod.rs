// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The network tool's geometry (`docs/specs/network-geometry.md`): the
//! geometry half of `PlaceNetwork` (network-tools.md §5). Section
//! references (`§n`) point at network-geometry.md.
//!
//! # Structure
//!
//! - [`params`]: the network types, the placement parameters (§2.4, the
//!   exemplars of group `0x084344E0`, a host input) and the per-type limits
//!   (§2.3).
//! - [`drag`]: `BreakIntoStraightAndDiagSegments` and `ComputeDraggedCells`
//!   (§3).
//! - [`passes`]: the validation passes and their failure codes (§4–§7).
//! - [`satisfier`]: the vertex-height constraint satisfier (§9.1).
//! - [`smoothing`]: the cell infos (§8.1), the per-cell constraints (§9.2,
//!   §9.3), `SmoothenNetwork` (§9.4) and `CanTilesBeSupportedOnTerrain`
//!   (§9.6).
//! - [`grading`]: `UpdateTerrainHeights` (§9.7).
//! - [`extras`]: the occupant flags, support levels, costs and the removal
//!   choice that §18's tests 14, 16–19 cover (§10, §11, §13).
//! - [`NetworkGeometry`]: the tool's geometry state tying them together.
//!
//! The terrain is read and graded through [`GeometryTerrain`] and
//! [`GradingTerrain`], both implemented for [`crate::terrain::Terrain`].
//! Everything else the tool asks the world (occupants, connection nodes,
//! the bridge internals of NG2, the piece solver's constraints of NG5) goes
//! through [`GeometryHost`], whose defaults are an empty world.
//!
//! # Precision (foundations §18)
//!
//! Every float operation is computed in `f64` from `f32` operands and
//! rounded once with [`crate::x87::r24`].
//!
//! # Choices (`[choice]`) and inferences (`[inferred]`)
//!
//! Each is tagged in the code and listed in the lane report as an NG
//! question. The satisfier's pick tie-break (NG1) is the vertices'
//! insertion order [choice].

pub mod drag;
pub mod extras;
pub mod grading;
pub mod params;
pub mod passes;
pub mod satisfier;
pub mod smoothing;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

pub use drag::{break_into_segments, compute_dragged_cells, direction_offset, Segments, Step, StepKind};
pub use grading::{update_terrain_heights, Grading, GRADING_THRESHOLD};
pub use params::{PlacementParams, TypeInfo, TypeTable, TYPE_COUNT};
pub use passes::{code, Bridge, BridgeChoice, CellOccupant, ExtraCell, PassResult, ToolFlags, Tunnel};
pub use satisfier::{Constraint, VertexHtConstraintSatisfier};
pub use smoothing::{cell_key, CellInfo, ExistingPiece, SmoothingSet, TerrainVertex};

use crate::terrain::{Terrain, VertRect};

/// The terrain as the geometry reads it.
pub trait GeometryTerrain {
    /// The city size in cells (X, Z); the vertex row stride is X + 1.
    fn cells(&self) -> (u32, u32);
    /// Terrain slot 12 `GetAltitude(vertex id)`.
    fn vertex_altitude(&self, v: i32) -> f32;
    /// Terrain slot 34 `CellHasWater`.
    fn cell_has_water(&self, x: i32, z: i32) -> bool;
    /// The surface-water level at a vertex (§8.1); NaN = none.
    fn surface_water_level(&self, v: i32) -> f32;
    /// Terrain slot 24 `MaxAllowedHeight`.
    fn max_allowed_height(&self) -> f32;
}

// Static-record lifecycle is distinct from the degree-valued geometry input.
pub mod type_info;
#[cfg(test)]
mod type_info_tests;

/// The terrain as the grading writes it (§9.7).
pub trait GradingTerrain: GeometryTerrain {
    /// Terrain slot 50 `SetAltitudeAtVertex`.
    fn set_altitude_at_vertex(&mut self, v: i32, h: f32);
    /// Terrain slot 87.
    fn begin_long_terrain_operation(&mut self);
    /// Terrain slot 68 `RedisplayTerrain(1, 1, rect, kind)`.
    fn redisplay_terrain(&mut self, rect: VertRect, kind: u32);
    /// Terrain slot 88.
    fn end_long_terrain_operation(&mut self);
}

impl GeometryTerrain for Terrain {
    fn cells(&self) -> (u32, u32) {
        (self.cells_x(), self.cells_z())
    }
    fn vertex_altitude(&self, v: i32) -> f32 {
        if v < 0 {
            0.0
        } else {
            self.altitude_at_index(v as u32)
        }
    }
    fn cell_has_water(&self, x: i32, z: i32) -> bool {
        Terrain::cell_has_water(self, x, z)
    }
    /// [inferred] The water level (`altitude + depth`, terrain slot 31); a
    /// dry vertex's level is at or below its altitude.
    fn surface_water_level(&self, v: i32) -> f32 {
        if v < 0 || v as u32 >= self.vertex_count() {
            f32::NAN
        } else {
            self.water_level_at_index(v as u32)
        }
    }
    fn max_allowed_height(&self) -> f32 {
        Terrain::max_allowed_height(self)
    }
}

impl GradingTerrain for Terrain {
    fn set_altitude_at_vertex(&mut self, v: i32, h: f32) {
        if v >= 0 {
            Terrain::set_altitude_at_vertex(self, v as u32, h);
        }
    }
    fn begin_long_terrain_operation(&mut self) {
        Terrain::begin_long_terrain_operation(self);
    }
    fn redisplay_terrain(&mut self, rect: VertRect, kind: u32) {
        Terrain::redisplay_terrain(self, true, true, rect, kind);
    }
    fn end_long_terrain_operation(&mut self) {
        Terrain::end_long_terrain_operation(self);
    }
}

/// What the geometry asks the rest of the world. The defaults are an empty
/// world: no occupants, no connection nodes, no bridge types.
pub trait GeometryHost {
    /// The cell has a cell info that is a connection node (`+0x56`, §4.1).
    fn is_connection_node(&self, _x: i32, _z: i32) -> bool {
        false
    }
    /// The occupants of a cell (occupant manager slot 40), for §5.1.
    fn cell_occupants(&self, _x: i32, _z: i32) -> Vec<CellOccupant> {
        Vec::new()
    }
    /// The existing network piece in a cell (§8.1).
    fn existing_piece(&self, _x: i32, _z: i32) -> Option<ExistingPiece> {
        None
    }
    /// §3.3 step 3: the cell holds an auto-connect occupant (slots 65 and
    /// 67) or, for road, a cell info with edge bits.
    fn extends_diagonal_end(&self, _x: i32, _z: i32, _type_id: u32) -> bool {
        false
    }
    /// Slot 59 `UpdateOccupantHighlights` (network-tools.md §6): the new
    /// valid count (at most `v`).
    fn update_occupant_highlights(&mut self, _run: &Run, v: u32) -> u32 {
        v
    }
    /// The number of bridge types of the network type (type info `+0x30`).
    fn bridge_type_count(&self, _type_id: u32) -> u32 {
        0
    }
    /// `IsStepGoodForBridge` (NG2).
    fn is_step_good_for_bridge(&self, _run: &Run, _step: u32) -> bool {
        true
    }
    /// `CheckBridgeStepEdgeTypes` (NG2).
    fn bridge_step_edge_types_ok(&self, _run: &Run, _step: u32) -> bool {
        true
    }
    /// `AdjustBridgeSpan` (NG2); `None` fails the span.
    fn adjust_bridge_span(&mut self, _run: &Run, start: u32, end: u32) -> Option<(u32, u32)> {
        Some((start, end))
    }
    /// `FindBestBridgeType` (NG2); `None` = no bridge type fits.
    fn find_best_bridge_type(&mut self, _run: &Run, _start: u32, _end: u32) -> Option<BridgeChoice> {
        None
    }
    /// `IdentifyLandBridgeCells` (NG4); never lowers `v`.
    fn identify_land_bridge_cells(&mut self, _run: &mut Run, _v: u32) {}
    /// The bridge and tunnel elevation constraints, slot 77's plane
    /// constraint, slot 78 and the intersection constraints (§9.4 steps 2
    /// and 4; NG2, NG5), added to the network satisfier before `Solve`.
    fn extra_network_constraints(&mut self, _sat: &mut VertexHtConstraintSatisfier, _level: i32) {}
    /// The intersection constraints of §9.6 step 4 (NG5).
    fn extra_terrain_constraints(&mut self, _sat: &mut VertexHtConstraintSatisfier) {}
}

/// An empty world.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoHost;
impl GeometryHost for NoHost {}

/// The dragged run: the tool's fields `+0x54` to `+0x80`, the extra cells,
/// and the tunnels and bridges the passes recorded.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Run {
    pub type_id: u32,
    pub start: (i32, i32),
    /// The end after `BreakIntoStraightAndDiagSegments`.
    pub end: (i32, i32),
    /// `+0x60`.
    pub cells: Vec<(i32, i32)>,
    /// `+0x54`.
    pub steps: Vec<Step>,
    /// `+0x70`.
    pub dragged_count: u32,
    /// `+0x74`.
    pub straight_count: u32,
    /// `+0x6C` and the second axis output.
    pub axis: (u8, u8),
    /// `+0x7C`.
    pub pen_direction: u8,
    /// `+0x80`.
    pub end_direction: u8,
    /// `+0xF0` (NG3).
    pub extra_cells: Vec<ExtraCell>,
    pub tunnels: Vec<Tunnel>,
    pub bridges: Vec<Bridge>,
    /// The start steps `FindTunnelEnd` was run from (a trace for tests).
    pub tunnel_searches: Vec<u32>,
    /// §6.2 step 1's start height.
    pub tunnel_start_height: Option<f32>,
}

/// `DrawNetworkLine(&start, &end, snap, type)` (slot 18, §3.1) into `run`.
pub fn draw_network_line(
    run: &mut Run,
    info: &TypeInfo,
    start: (i32, i32),
    end: (i32, i32),
    snap: bool,
    cells: (u32, u32),
    host: &dyn GeometryHost,
) -> bool {
    // 1. Clear the steps and the cells.
    *run = Run {
        type_id: info.type_id,
        start,
        end,
        ..Run::default()
    };
    // 2. BreakIntoStraightAndDiagSegments.
    let (w1, w2) = info.widths;
    let seg = break_into_segments(start, end, snap, info.params.straight_only, w1, w2);
    run.end = seg.end;
    run.dragged_count = seg.dragged_count;
    run.straight_count = seg.straight_count;
    run.axis = seg.axis;
    run.pen_direction = seg.pen_direction;
    run.end_direction = seg.end_direction;
    // 3. Straight only.
    if info.params.straight_only && seg.dragged_count != seg.straight_count {
        return false;
    }
    // 4. Ordered staircase rows, including turn cells (network-tools NT28).
    let (c, s) = drag::compute_dragged_rows(start, seg.end, &seg, w1, w2);
    run.cells = c;
    run.steps = s;
    // §3.3 step 3: [inferred] "whose end is diagonal" is read as the end
    // direction (always diagonal), so the next cell is the last cell plus
    // the end direction's offset (Q NG9).
    let n = run.steps.len() as u32;
    if w1 == 1 && (n >= 3 || (n == 2 && seg.straight_count == n)) {
        if let Some(&(x, z)) = run.cells.last() {
            let (dx, dz) = direction_offset(seg.end_direction);
            if host.extends_diagonal_end(x + dx, z + dz, info.type_id) {
                let i = run.cells.len() as u32;
                run.cells.push((x + dx, z + dz));
                run.steps.push(Step {
                    first: i,
                    last: i,
                    kind: StepKind::Regular,
                });
            }
        }
    }
    run.dragged_count = run.steps.len() as u32;
    // 5. A type wider than one cell must stay inside the city.
    if w1 > 1
        && run
            .cells
            .iter()
            .any(|&(x, z)| x as u32 >= cells.0 || z as u32 >= cells.1)
    {
        run.cells.clear();
        run.steps.clear();
        run.dragged_count = 0;
        return false;
    }
    true
}

/// The tool's geometry state: the type table, the run, the valid count and
/// failure code, the cell infos, the smoothing sets and the two satisfiers
/// (`+0x124` network, `+0x188` terrain).
#[derive(Clone, Debug, Default)]
pub struct NetworkGeometry {
    pub types: TypeTable,
    /// Tool `+0x354` per type (NG4); false unless the host sets it.
    pub land_bridges: [bool; TYPE_COUNT],
    pub run: Run,
    /// `+0x78`.
    pub valid: u32,
    /// `+0x20C`: the failure code, 0 = none.
    pub failure: u32,
    /// The cell infos by cell key (`GetCellInfo`).
    pub cells: BTreeMap<i32, CellInfo>,
    /// The solved cells (the piece solver's, NG5). [choice] Until it
    /// exists, the dragged cells of the valid steps.
    pub solved: Vec<(i32, i32)>,
    pub smoothing: SmoothingSet,
    pub network: VertexHtConstraintSatisfier,
    pub terrain: VertexHtConstraintSatisfier,
    /// The level the smoothing loop succeeded at.
    pub level: Option<i32>,
}

impl NetworkGeometry {
    pub fn new(types: TypeTable) -> NetworkGeometry {
        NetworkGeometry {
            types,
            ..NetworkGeometry::default()
        }
    }

    fn info(&self, type_id: u32) -> TypeInfo {
        self.types
            .get(type_id)
            .cloned()
            .unwrap_or_else(|| TypeInfo::new(type_id, PlacementParams::default()))
    }

    /// The tool flags of the run's type.
    pub fn flags(&self) -> ToolFlags {
        let t = self.run.type_id;
        let mut f = ToolFlags::for_type(t);
        f.land_bridges = self.land_bridges.get(t as usize).copied().unwrap_or(false);
        f
    }

    /// Clears the run and the solve state (`Reset`).
    pub fn reset(&mut self) {
        self.run = Run::default();
        self.valid = 0;
        self.failure = 0;
        self.cells.clear();
        self.solved.clear();
        self.smoothing = SmoothingSet::default();
        self.network.shutdown();
        self.terrain.shutdown();
        self.level = None;
    }

    /// `DrawNetworkLine` (§3.1).
    pub fn draw_network_line(
        &mut self,
        type_id: u32,
        start: (i32, i32),
        end: (i32, i32),
        snap: bool,
        t: &dyn GeometryTerrain,
        host: &dyn GeometryHost,
    ) -> bool {
        let info = self.info(type_id);
        self.valid = 0;
        let ok = draw_network_line(&mut self.run, &info, start, end, snap, t.cells(), host);
        self.valid = self.run.dragged_count;
        ok
    }

    /// The validation passes (§4). Sets `valid` and the failure code.
    pub fn validate(&mut self, t: &dyn GeometryTerrain, host: &mut dyn GeometryHost) -> PassResult {
        let info = self.info(self.run.type_id);
        let flags = self.flags();
        let r = passes::run_passes(&mut self.run, &flags, &info, t, host);
        self.valid = r.valid;
        if let Some(c) = r.failure {
            self.failure = c;
        }
        r
    }

    /// The cell infos of the valid steps (slot 71, §8.1), the run's edges
    /// ([choice], until the piece solver of NG5) and the solved cells.
    pub fn create_cell_infos(&mut self, t: &dyn GeometryTerrain, host: &dyn GeometryHost) {
        let info = self.info(self.run.type_id);
        self.cells = smoothing::create_cell_infos(&self.run, &info, self.valid, t, host);
        smoothing::assign_run_edges(&self.run, self.valid, &mut self.cells);
        self.solved = self
            .run
            .steps
            .iter()
            .take(self.valid as usize)
            .flat_map(|s| self.run.cells[s.first as usize..=s.last as usize].iter().copied())
            .collect();
    }

    /// Adds or replaces a cell info (placeholders, existing occupants).
    pub fn insert_cell_info(&mut self, ci: CellInfo) {
        self.cells.insert(cell_key(ci.x, ci.z), ci);
    }

    /// `SmoothenNetwork(level)` (§9.4).
    pub fn smoothen(&mut self, level: i32, t: &dyn GeometryTerrain, host: &mut dyn GeometryHost) -> bool {
        let info = self.info(self.run.type_id);
        self.smoothing =
            smoothing::compute_initial_smoothing_set(&self.run, &self.solved, &self.cells, level, t.cells());
        smoothing::smoothen_network(
            &mut self.network,
            &self.smoothing,
            &self.cells,
            &info,
            &self.types.infos,
            t,
            host,
            level,
        )
    }

    /// Slot 56 `CanTilesBeSupportedOnTerrain` (§9.6): always true for the
    /// underground tools.
    pub fn can_tiles_be_supported(&mut self, t: &dyn GeometryTerrain, host: &mut dyn GeometryHost) -> bool {
        if !self.flags().support_test {
            return true;
        }
        let info = self.info(self.run.type_id);
        smoothing::can_tiles_be_supported(
            &mut self.terrain,
            &self.network,
            &self.smoothing,
            &self.cells,
            &info,
            t,
            host,
        )
    }

    /// The smoothing loop (§9.4). On failure the code is stored and `valid`
    /// is 0.
    pub fn smooth(&mut self, t: &dyn GeometryTerrain, host: &mut dyn GeometryHost) -> Result<i32, u32> {
        let has_bridge = !self.run.bridges.is_empty();
        let r = smoothing::smoothing_loop(has_bridge, |level| {
            if !self.smoothen(level, t, host) {
                return (false, false);
            }
            (true, self.can_tiles_be_supported(t, host))
        });
        match r {
            Ok(level) => {
                self.level = Some(level);
                // Retain the final network corners for pricing/insertion.
                // NT29: a later getter reads live terrain, never re-solves
                // these corners from the new terrain or satisfier targets.
                for ci in self.cells.values_mut() {
                    let off = if ci.connection_node {
                        smoothing::CONNECTION_NODE_OFFSET
                    } else {
                        0
                    };
                    ci.corners = std::array::from_fn(|k| {
                        self.network
                            .height(ci.vertices[k] + off)
                            .unwrap_or_else(|| t.vertex_altitude(ci.vertices[k]))
                    });
                }
            }
            Err(c) => {
                self.failure = c;
                self.valid = 0;
                self.level = None;
            }
        }
        r
    }

    /// The geometry of `PlaceNetwork` (network-tools.md §5.1 steps 4 and 5
    /// without the piece solver): the passes, the cell infos and the
    /// smoothing loop. Returns the valid count (0 = refused; see `failure`).
    pub fn place(&mut self, t: &dyn GeometryTerrain, host: &mut dyn GeometryHost) -> u32 {
        if self.run.dragged_count == 0 {
            return 0;
        }
        self.validate(t, host);
        if self.valid == 0 {
            return 0;
        }
        self.create_cell_infos(t, host);
        if self.smooth(t, host).is_err() {
            // Failure retains the public footprint, but no solved inventory
            // is priced or inserted (network-tools §5.2).
            self.solved.clear();
            return 0;
        }
        self.valid
    }

    /// The inputs of `GetCostOfSolution` (network-tools.md §8.1) for the
    /// retained solved cells (NT29): retained network corners versus live
    /// terrain. The intersection marker alone excludes the cell charge;
    /// placed state does not. The bridge records' costs are NG2's (0).
    pub fn cost_inputs(&self, t: &dyn GeometryTerrain, demolition: i32) -> extras::CostInputs {
        let info = self.info(self.run.type_id);
        let mut cell_costs = [0; TYPE_COUNT];
        for (k, c) in cell_costs.iter_mut().enumerate() {
            *c = self.types.get(k as u32).map_or(1, |ti| ti.params.cell_cost);
        }
        let cells = self
            .solved
            .iter()
            .filter_map(|&(x, z)| self.cells.get(&cell_key(x, z)))
            .map(|ci| extras::CostCell {
                in_run: ci.in_run,
                prebuilt: ci.intersection,
                type_mask: ci.flags & extras::TYPE_BITS,
                corners: ci.corners,
                terrain: ci.vertices.map(|v| t.vertex_altitude(v)),
            })
            .collect();
        extras::CostInputs {
            demolition,
            cells,
            cell_costs,
            earthwork_cost: info.params.earthwork_cost,
            bridge_cost: info.params.bridge_cost,
            bridges: self
                .run
                .bridges
                .iter()
                .map(|b| extras::CostBridge {
                    start: b.start,
                    end: b.end,
                    per_cell: 0,
                    fixed: 0,
                })
                .collect(),
            tunnel_cost: info.params.tunnel_cost,
            tunnels: self.run.tunnels.iter().map(|t| (t.start, t.end)).collect(),
            intersections: Vec::new(),
        }
    }

    /// Slot 57 `UpdateTerrainHeights` (§9.7): a no-op for the underground
    /// tools.
    pub fn update_terrain_heights(&mut self, t: &mut dyn GradingTerrain) -> Grading {
        if !self.flags().grading {
            return Grading::default();
        }
        let grading = update_terrain_heights(&self.terrain, t);
        self.refresh_committed_corners(t, &grading.changed);
        grading
    }

    /// Real commit's two post-grading refreshes (network-tools §6.5, NT30).
    /// The caller supplies actual grading output order; terrain and network
    /// satisfiers remain separate. Existing solved entries keep their index.
    pub fn refresh_committed_corners<T: GeometryTerrain + ?Sized>(&mut self, t: &T, changed: &[i32]) {
        let mut solved: std::collections::BTreeSet<_> = self.solved.iter().copied().collect();
        // Preserve the supplied smoothing iteration order. Its construction
        // currently uses key order rather than the native hash order (NG5).
        for key in &self.smoothing.cells {
            let Some(ci) = self.cells.get_mut(key) else { continue };
            let off = if ci.connection_node {
                smoothing::CONNECTION_NODE_OFFSET
            } else {
                0
            };
            let corners = ci.vertices.map(|v| {
                self.network
                    .height(v + off)
                    .unwrap_or_else(|| t.vertex_altitude(v + off))
            });
            let differs = ci.corners != corners;
            ci.corners = corners;
            if differs && ci.placed && solved.insert((ci.x, ci.z)) {
                self.solved.push((ci.x, ci.z));
            }
        }
        let (cx, cz) = t.cells();
        let stride = i64::from(cx) + 1;
        for &v in changed {
            let vx = (i64::from(v) % stride) as i32;
            let vz = (i64::from(v) / stride) as i32;
            for z in (vz - 1).max(0)..=vz.min(cz as i32 - 1) {
                for x in (vx - 1).max(0)..=vx.min(cx as i32 - 1) {
                    if solved.contains(&(x, z)) {
                        continue;
                    }
                    let Some(ci) = self.cells.get_mut(&cell_key(x, z)) else {
                        continue;
                    };
                    if !ci.placed {
                        continue;
                    }
                    // Ordinary ids, direct network lookups: missing heights
                    // preserve cached corners, even at connection nodes.
                    for (corner, vertex) in ci.corners.iter_mut().zip(ci.vertices) {
                        if let Some(height) = self.network.height(vertex) {
                            *corner = height;
                        }
                    }
                    solved.insert((x, z));
                    self.solved.push((x, z));
                }
            }
        }
    }
}
