// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The network tool's bridges (`docs/specs/network-bridges.md`): the bridge
//! types, the water and land passes, the elevations, the two models, the
//! pieces and their pillars, the cost and the tool's bridge getters.
//! Section references (`§n`) point at network-bridges.md.
//!
//! # Structure
//!
//! - [`types`]: the bridge type record (§2.3), its defaults, the ramp
//!   heights (§2.4) and the shipped columns (§2.6).
//! - [`load`]: the INI sections (§2.1), one entry (§2.2) and the water and
//!   land lists with the land-bridge flag (§2.5).
//! - [`specialize`]: the specialisation grammar (§3).
//! - [`span`]: the step test (§4.2), the edge test (§4.3),
//!   `SelectBridgeStartAndEnd` (§5.1) and `AdjustBridgeSpan` (§5.2).
//! - [`select`]: `FindBestBridgeType` (§6.1) and
//!   `PadStepsToMakeMinBridgeSpan` (§6.2).
//! - [`land`]: `IdentifyLandBridgeCells` (§7.2) and `CreateLandBridge` (§7.3).
//! - [`elev`]: `SelectBridgeElevation` (§8.2) and the bridge's four places in
//!   the height solve (§8.3, the answer to NB1).
//! - [`model`]: the orientation (§9.1), the piece record (§9.2),
//!   `ConstructSpecializedBridge` (§9.3) and `ConstructParametricBridge`
//!   (§10).
//! - [`pieces`]: `InsertBridgePieces` (§11.1), `InsertBridgePiece` (§11.2)
//!   and the pillars (§11.3).
//! - [`cost`]: `GetCostOfSolution`'s bridge term (§12) and the failure codes
//!   (§13).
//! - [`getters`]: the tool's bridge slots (§1.1).
//! - [`BridgeTool`]: the tool's bridge state, tying them together.
//!
//! The geometry the bridge sits on — the run, the steps, the cell infos, the
//! height satisfier and the terrain — is the network geometry core
//! ([`crate::network_geometry`]); this module reuses its types and never
//! duplicates them. The bridge types come in through [`load::BridgeSource`].
//!
//! # Precision (foundations §18)
//!
//! Every float operation is computed in `f64` from `f32` operands and rounded
//! once with [`crate::x87::r24`].
//!
//! # Choices (`[choice]`) and inferences (`[inferred]`)
//!
//! Each is tagged in the code and listed in the lane report as a numbered
//! question. The suspension profile and the cable plotting (§11.4) are
//! drawing only and are not implemented: their formulas are not in the spec.

pub mod cost;
pub mod elev;
pub mod getters;
pub mod land;
pub mod load;
pub mod model;
pub mod pieces;
pub mod select;
pub mod span;
pub mod specialize;
pub mod types;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

pub use cost::{bridge_cost, Failure};
pub use elev::{
    bridge_elevation_constraints, remove_bridge_pieces_from_solve_set, select_bridge_elevation,
    BridgeConstraintEntry,
};
pub use getters::ToolBridgeGetters;
pub use land::{create_land_bridge, identify_land_bridge_cells, LandScan};
pub use load::{
    bridge_type_lists, land_bridge_flag, load_bridge_types, BridgeSource, IniEntry, PropValue,
};
pub use model::{
    construct_parametric_bridge, construct_specialized_bridge, lane_cells, support_flags,
    Orientation, PieceKind, PieceRecord, CORNER_BASE,
};
pub use pieces::{insert_bridge_piece, insert_bridge_pieces, pillar_columns, PillarColumn};
pub use select::{find_best_bridge_type, pad_steps_to_make_min_bridge_span, BridgeCandidate};
pub use span::{
    adjust_bridge_span, bridge_step_edge_types_ok, is_step_good_for_bridge,
    select_bridge_start_and_end, AdjustedSpan, EdgeMask,
};
pub use specialize::{parse_specializations, Specialization, SpecialPiece, SupportKind};
pub use types::{
    ramp_heights, BridgeType, PieceDef, ShippedBridgeType, DEFAULT_MAX_CLEARANCE, DEFAULT_MAX_SPAN,
    DEFAULT_MAX_PIER_COUNT,
};

use crate::network_geometry::drag::StepKind;
use crate::network_geometry::{GeometryTerrain, Run};

/// The record the tool appends for each bridge it places (§4.5, `0x28`
/// bytes). The pieces are the models' output (§9, §10).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BridgeRecord {
    /// `+0x00`: the network type.
    pub type_id: u32,
    /// `+0x04`, `+0x08`: the start and end step (inclusive).
    pub start: u32,
    pub end: u32,
    /// `+0x0C`: the bridge type index into the network type's vector.
    pub bridge_type: u32,
    /// `+0x10`: the deck elevation (§8.2; land: §7.3).
    pub deck: f32,
    /// `+0x14`: the reference elevation (§5.2; land: the same as `deck`).
    pub reference: f32,
    /// `+0x18`: 1 = a land bridge (§7.3), 0 = a water bridge.
    pub land: u8,
    /// `+0x1C`–`+0x24`: the piece records (§9.2).
    pub pieces: Vec<PieceRecord>,
}

/// The tool's bridge state: the two bridge type lists, the candidate list and
/// the record vector (`+0x33C`, `+0x348`, `+0x1F4`, `+0xD8`), plus the two
/// fields `Reset` clears (`+0x200`, `+0x204`) and the land-bridge flag
/// (`+0x354`).
///
/// The getters of §1.1 read this from a second thread while `OnTick` runs, so
/// it is plain data behind the caller's own synchronisation: no `RefCell`
/// (HANDOFF §5.1).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BridgeTool {
    /// `+0x33C`: the bridge type indices with the water flag (§2.5).
    pub water: Vec<u32>,
    /// `+0x348`: the bridge type indices with the land flag (§2.5).
    pub land: Vec<u32>,
    /// `+0x354`: the land-bridge flag (§2.5, quirk 3).
    pub land_bridges: bool,
    /// `+0x1F4`: the candidates of the last `FindBestBridgeType` (§6.3).
    pub candidates: Vec<u32>,
    /// `+0x200`: the preferred bridge index, −1 = none (§1.1).
    pub preferred: i32,
    /// `+0x204`: the altitude override, −1.0 = none (§1.1).
    pub altitude_override: f32,
    /// `+0xD8`: the bridge records of this run (§4.5).
    pub records: Vec<BridgeRecord>,
    /// `+0x358` `+0x30`: the network type's bridge type vector (§2.3).
    pub types: Vec<BridgeType>,
    /// The specialisation length maps, by bridge type index (§3).
    pub specializations: Vec<BTreeMap<u32, Specialization>>,
}

impl BridgeTool {
    /// `Reset` (§1.1): no preference and no override; the run's records are
    /// cleared at the start of each run (network-tools.md §5.1 step 2).
    pub fn reset(&mut self) {
        self.candidates.clear();
        self.preferred = -1;
        self.altitude_override = -1.0;
        self.records.clear();
    }

    /// A tool with `types` already loaded (§2.1) and `Reset` applied.
    pub fn new(types: Vec<BridgeType>) -> BridgeTool {
        let mut t = BridgeTool {
            types,
            ..BridgeTool::default()
        };
        t.reset();
        let lists = bridge_type_lists(&t.types);
        t.water = lists.0;
        t.land = lists.1;
        t.land_bridges = land_bridge_flag(t.types.len());
        t.specializations = vec![BTreeMap::new(); t.types.len()];
        t
    }

    /// The bridge type of a record (§8.3, §12): through the tool's own type
    /// info (`+0x358`) and the record's index `+0x0C`.
    pub fn type_of_record(&self, r: &BridgeRecord) -> Option<&BridgeType> {
        self.types.get(r.bridge_type as usize)
    }

    /// The "major" bridge: the **last** record appended in the run (§1.1).
    pub fn major(&self) -> Option<&BridgeRecord> {
        self.records.last()
    }

    /// Appends a record (§4.4 step 4.2, §7.3 step 3). The caller marks the
    /// run's steps kind 1 with [`mark_bridge_steps`] and builds the model.
    pub fn append_record(&mut self, r: BridgeRecord) {
        self.records.push(r);
    }
}

/// Marks steps `start … end` kind 1 (§4.4 step 4.1, §7.3 step 3).
pub fn mark_bridge_steps(run: &mut Run, start: u32, end: u32) {
    let last = run.steps.len().saturating_sub(1) as u32;
    for k in start..=end.min(last) {
        if let Some(s) = run.steps.get_mut(k as usize) {
            s.kind = StepKind::Bridge;
        }
    }
}

/// The rectangle a step's cells span, grown by one cell on every side
/// (§4.2): the step's first and last cells. `None` when the step has no
/// cells.
pub fn step_rect(run: &Run, step: u32, grow: i32) -> Option<((i32, i32), (i32, i32))> {
    let s = run.steps.get(step as usize)?;
    let first = *run.cells.get(s.first as usize)?;
    let last = *run.cells.get(s.last as usize)?;
    Some((
        (first.0.min(last.0) - grow, first.1.min(last.1) - grow),
        (first.0.max(last.0) + grow, first.1.max(last.1) + grow),
    ))
}

/// The cells of a step, first to last (network-geometry.md §2.2).
pub fn step_cells_of<'a>(run: &'a Run, step: u32) -> &'a [(i32, i32)] {
    match run.steps.get(step as usize) {
        Some(s) => {
            let (a, b) = (s.first as usize, s.last as usize);
            if a <= b && b < run.cells.len() {
                &run.cells[a..=b]
            } else {
                &[]
            }
        }
        None => &[],
    }
}

/// True when every cell of the step is a regular step's (§4.4 step 1 reads
/// the kind of three steps in a row).
pub fn regular3(run: &Run, i: u32) -> bool {
    [i, i.wrapping_sub(1), i.wrapping_sub(2)].iter().all(|&k| {
        k < 0x8000_0000
            && run
                .steps
                .get(k as usize)
                .is_some_and(|s| s.kind == StepKind::Regular)
    })
}

/// The terrain measures §5.2 uses over a step's cells.
pub trait BridgeTerrain: GeometryTerrain {
    /// Terrain slot 17: the cell's **lowest** corner. [inferred] The spec
    /// reads a per-cell lowest-corner value; a cell has four corners.
    fn lowest_corner(&self, x: i32, z: i32) -> f32;
    /// Terrain slot 18: the cell's surface level (the corner altitude plus
    /// any water depth). [inferred] A cell has four corners; this core takes
    /// the same corner slot 18 reports.
    fn surface_level(&self, x: i32, z: i32) -> f32;
    /// Terrain slot 30: the water level.
    fn water_level(&self, x: i32, z: i32) -> f32;
}

/// The world the bridge pass asks about: occupants (§4.2, §7.3 step 2) and
/// the exemplar properties (§2.2). The defaults are an empty world.
pub trait BridgeHost {
    /// Occupant manager slot 40 over a rectangle, with `max_count` −1. The
    /// `mask` is the tool's OR filter (§4.2): a network occupant whose
    /// `HasAnyNetworkFlag(mask)` is true. `None` = no filter (§7.3 step 2:
    /// any occupant at all). A failed query reports `false` (§4.2).
    fn occupant_present(
        &self,
        rect: ((i32, i32), (i32, i32)),
        mask: Option<u32>,
    ) -> bool {
        let _ = (rect, mask);
        false
    }
    /// The properties of a bridge exemplar (§2.2 step 5). `None` = the lookup
    /// failed and the record is removed again.
    fn exemplar_properties(
        &self,
        _exemplar: (u32, u32, u32),
    ) -> Option<Vec<(u32, load::PropValue)>> {
        None
    }
    /// The localised string an exemplar's `0x482CD466` names (§2.2 step 6).
    fn localized_string(&self, _id: u32) -> Option<String> {
        None
    }
}

/// An empty world: nothing in the way and no exemplars.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoBridgeHost;
impl BridgeHost for NoBridgeHost {}
