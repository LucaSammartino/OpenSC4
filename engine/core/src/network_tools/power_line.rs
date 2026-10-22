// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The power-line tool `cSC4PowerLineTool` (network-tools.md §9,
//! network-geometry.md §11, network-occupants.md §4.5/§5.2): the tool's own
//! state (the pole and line lists, the wire-cell count, the exemplar
//! parameters), slot 53 `DoNetworkSpecificPlacement` (`0x00652980`), slot 81
//! `CheckForReservedTiles` (`0x0064FED0`), `GetCostOfSolution`
//! (`0x0064F0F0`), `ComputePolePosition` (`0x0064EFE0`) and the reuse rule of
//! `CreatePowerPole` (`0x00650140`).
//!
//! # Not specified (owned by the host, questions NT18-NT20)
//!
//! The specs name `DeterminePolePositions` (`0x00650840`), `PlacePoles`
//! (`0x00651560`), `AttemptNeighborConnections`, `PlaceNeighborConnections`
//! and the slot 82 preview refresh but do not give their bodies, nor where
//! slot 81 is called from on this path; they are [`PowerLineHost`] hooks.
//! Slots 3, 4, 10 and 16 (`Init`, `Shutdown`, `UpdateOnZoneChange`,
//! `0x0064F3F0`) are listed by network-tools.md §1 without behaviour and are
//! not implemented here.

use crate::network_geometry::extras::power_line_cost;
use crate::network_occupants::power_line::pole_position_axis;

use super::host::ToolRun;
use super::state::ToolState;

/// §5.3 / §9: a cell is zoned.
pub const FAIL_ZONED: u32 = 0xF000_0011;
/// §5.3 / §9: a cell has a building.
pub const FAIL_BUILDING: u32 = 0xF000_0012;
/// §5.3 / §9: a cell has a road.
pub const FAIL_ROAD: u32 = 0xF000_0013;

/// The size of one pole record (`+0x36C`..`+0x370` vector stride, §11).
pub const POLE_RECORD_BYTES: u32 = 0x30;

/// The exemplar the tool's parameters come from (network-geometry.md §11):
/// type `0x6534284A`, group `0xE7E2C2DB`, instance `0xC911E35B`.
pub const PARAM_EXEMPLAR: (u32, u32, u32) = (0x6534_284A, 0xE7E2_C2DB, 0xC911_E35B);
/// Cost per pole, `+0x3AC` (shipped 15).
pub const PROP_COST_PER_POLE: u32 = 0x4A5F_5953;
/// Cost per wire cell, `+0x3B0` (shipped 2.0).
pub const PROP_COST_PER_WIRE: u32 = 0x0A5F_596B;
/// `+0x3A4` (shipped 10, default 20).
pub const PROP_3A4: u32 = 0x098B_25C8;
/// `+0x3A8` (shipped 7).
pub const PROP_3A8: u32 = 0x098B_27D0;

/// The tool's exemplar parameters (network-geometry.md §11).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowerLineParams {
    /// `+0x3A4`: spacing in dragged steps at pivots, default 20.
    pub v_3a4: u32,
    /// `+0x3A8`: terrain-ray clearance, default 7.0.
    pub v_3a8: f32,
    /// `+0x3AC`: cost per pole (`0x4A5F5953`).
    pub cost_per_pole: i32,
    /// `+0x3B0`: cost per wire cell (`0x0A5F596B`).
    pub cost_per_wire: f32,
}

impl Default for PowerLineParams {
    /// Fresh Init values, power-line-tool §1 (NT19).
    fn default() -> Self {
        PowerLineParams {
            v_3a4: 20,
            v_3a8: 7.0,
            cost_per_pole: 0,
            cost_per_wire: 0.0,
        }
    }
}

impl PowerLineParams {
    /// Reads the parameters from the exemplar: each property that
    /// `uint`/`float` answers replaces its default.
    pub fn load(uint: impl Fn(u32) -> Option<u32>, float: impl Fn(u32) -> Option<f32>) -> Self {
        let d = PowerLineParams::default();
        PowerLineParams {
            v_3a4: uint(PROP_3A4).unwrap_or(d.v_3a4),
            v_3a8: float(PROP_3A8).unwrap_or(d.v_3a8),
            cost_per_pole: uint(PROP_COST_PER_POLE).map_or(d.cost_per_pole, |v| v as i32),
            cost_per_wire: float(PROP_COST_PER_WIRE).unwrap_or(d.cost_per_wire),
        }
    }
}

/// One pole record (48 bytes in the original). The spec names only the
/// cell; the remaining bytes are the host's (question NT18).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PoleInfo {
    pub x: i32,
    pub z: i32,
}

/// The power-line tool's own fields. Its `+0x36C`/`+0x370` are the pole
/// vector's begin/end (§9 step 3, §11), not the base tool's underground
/// depth/cover fields at the same offsets (power line is a surface tool).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PowerLineTool {
    /// Ordered input anchors; distinct from generated pole records (§2).
    pub anchors: Vec<(i32, i32)>,
    /// `+0x36C`..`+0x370`: the pole records.
    pub poles: Vec<PoleInfo>,
    /// The line list (cells of the line occupants `PlacePoles` made).
    pub lines: Vec<(i32, i32)>,
    /// `+0x384`: counted up by `DeterminePolePositions` *[inferred: the
    /// number of wire cells]*.
    pub wire_cells: u32,
    /// `+0x3A4`..`+0x3B0`.
    pub params: PowerLineParams,
}

impl PowerLineTool {
    pub fn new(params: PowerLineParams) -> PowerLineTool {
        PowerLineTool {
            params,
            ..PowerLineTool::default()
        }
    }

    /// `(+0x370 − +0x36C) / 48`: the number of pole records.
    pub fn pole_count(&self) -> u32 {
        self.poles.len() as u32
    }

    /// `GetCostOfSolution` (`0x0064F0F0`): `+0x388` := the pole count; 0 for
    /// at most one pole, else `trunc(r24((float)wire × +0x3B0)) + +0x3AC ×
    /// poles`.
    pub fn cost_of_solution(&self) -> i32 {
        power_line_cost(
            self.pole_count(),
            self.wire_cells,
            self.params.cost_per_pole,
            self.params.cost_per_wire,
        )
    }
}

/// The hooks the specs name without a body (questions NT18, NT20).
pub trait PowerLineHost {
    /// `DeterminePolePositions` (`0x00650840`): fills `tool.poles` and counts
    /// `tool.wire_cells`. It may lower `run.valid_count` and set `failure`
    /// (slot 81, [`check_for_reserved_tiles`], is the only source of the
    /// power-line codes the spec names; where it is called is NT18).
    fn determine_pole_positions(&mut self, run: &mut ToolRun, tool: &mut PowerLineTool, failure: &mut u32);
    /// `PlacePoles(arg)` (`0x00651560`); §9 step 2 passes 1.
    fn place_poles(&mut self, run: &mut ToolRun, tool: &mut PowerLineTool, arg: u32);
    /// `AttemptNeighborConnections`.
    fn attempt_neighbor_connections(&mut self, run: &mut ToolRun, tool: &mut PowerLineTool) -> bool;
    /// `PlaceNeighborConnections`.
    fn place_neighbor_connections(&mut self, run: &mut ToolRun, tool: &mut PowerLineTool);
    /// Slot 82 with argument `arg` (§9 step 5 passes 0): the preview refresh.
    fn refresh_preview(&mut self, arg: u32);
    /// §9 step 6: release the pole infos.
    fn release_pole_infos(&mut self, tool: &mut PowerLineTool);
}

/// Slot 53 `DoNetworkSpecificPlacement` (`0x00652980`, network-tools.md §9).
/// Always returns true: the base solve never runs for power lines.
pub fn do_network_specific_placement(
    st: &mut ToolState,
    tool: &mut PowerLineTool,
    host: &mut dyn PowerLineHost,
    run: &mut ToolRun,
    neighbour_mode: u32,
    real: bool,
) -> bool {
    // §2: release saved endpoints before structural clear, including stale records.
    tool.wire_cells = 0;
    host.release_pole_infos(tool);
    tool.lines.clear();
    run.valid_count = run.dragged_count;
    // 2. With at least 2 dragged steps: DeterminePolePositions, PlacePoles(1).
    if run.dragged_count >= 2 {
        host.determine_pole_positions(run, tool, &mut st.failure);
        host.place_poles(run, tool, 1);
    }
    // 3. Pole span under 48 units (no whole record): valid count := 0.
    if tool.pole_count() == 0 {
        run.valid_count = 0;
    }
    // NT20: Attempt runs in either mode; Place requires success and real placement.
    if neighbour_mode != 0 && run.valid_count >= run.dragged_count && tool.anchors.len() >= 2 && st.failure == 0 {
        if host.attempt_neighbor_connections(run, tool) && real {
            host.place_neighbor_connections(run, tool);
        }
    }
    // 5. Draft mode: slot 82 (0) refreshes the preview.
    if !real {
        host.refresh_preview(0);
    }
    // 6. Release the pole infos; return true.
    host.release_pole_infos(tool);
    true
}

/// `CellIsOKForPlacement` (`0x0064FD00`) results (§9 table).
pub const CELL_CLEAR: u32 = 0;
pub const CELL_ZONED: u32 = 1;
pub const CELL_BUILDING: u32 = 2;
pub const CELL_ROAD: u32 = 3;

/// Slot 81 `CheckForReservedTiles` (`0x0064FED0`, network-geometry.md §11
/// NT3): walks `cells` asking `cell_ok`; returns the new valid count
/// `last + 1` and sets `failure` per result.
///
/// `uninit_prev` stands for the uninitialised "previous result" local on the
/// first cell *[quirk]*: a one-cell drag onto a road stops only when it is 0.
/// Results other than 0-3 are not specified; [literal] they leave `last`
/// alone and continue (NT20).
pub fn check_for_reserved_tiles(
    cells: &[(i32, i32)],
    mut cell_ok: impl FnMut(i32, i32) -> u32,
    uninit_prev: u32,
    failure: &mut u32,
) -> u32 {
    let count = cells.len() as i64;
    let mut last = count - 1;
    let mut prev = uninit_prev;
    for (i, &(x, z)) in cells.iter().enumerate() {
        let i = i as i64;
        let r = cell_ok(x, z);
        match r {
            CELL_CLEAR => last = i,
            CELL_ZONED => *failure = FAIL_ZONED,
            CELL_BUILDING => {
                *failure = FAIL_BUILDING;
                last = i - 1;
                break;
            }
            CELL_ROAD => {
                *failure = FAIL_ROAD;
                if i == count - 1 && prev == CELL_CLEAR {
                    last = i - 1;
                    break;
                }
            }
            _ => {}
        }
        prev = r;
    }
    (last + 1) as u32
}

/// `ComputePolePosition` (`0x0064EFE0`, network-occupants.md §5.2):
/// `x = (cell + 0.5) × 16`, `z` likewise, `y` = the terrain height.
/// "Guarded against `-FLT_MAX`": [literal] a terrain answer of `-FLT_MAX`
/// (no height) gives no position (question NO24).
pub fn compute_pole_position(x: i32, z: i32, terrain: impl Fn(f32, f32) -> f32) -> Option<[f32; 3]> {
    let (wx, wz) = (pole_position_axis(x), pole_position_axis(z));
    let y = terrain(wx, wz);
    (y != -f32::MAX).then_some([wx, y, wz])
}

/// The world the pole creation sees (network-occupants.md §4.5).
pub trait PoleWorld {
    type Pole;
    /// `FindPoleOccupant` (`0x0064FFE0`): an existing pole at the cell.
    fn find_pole(&self, x: i32, z: i32) -> Option<Self::Pole>;
    /// A new pole with the default exemplar set and `+0xB8` = `index`.
    fn new_pole(&mut self, x: i32, z: i32, exemplar: u32, index: u32) -> Self::Pole;
}

/// `CreatePowerPole` (`0x00650140`): reuse the pole at the cell, else create
/// one (`SetDefaultExemplar`; `+0xB8` := the exemplar index).
pub fn create_power_pole<W: PoleWorld>(w: &mut W, x: i32, z: i32, exemplar: u32, index: u32) -> W::Pole {
    match w.find_pole(x, z) {
        Some(p) => p,
        None => w.new_pole(x, z, exemplar, index),
    }
}
