// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The [`ToolHost`] seam: everything the network tool asks of the game
//! beyond its own state. The defaults are an empty world (like
//! `fc4_core::network_geometry::GeometryHost`); a glue host binds the
//! geometry core for the solve and the occupant manager for the inserts.

use super::resolve::{Listed, UndergroundOutcome};
use super::state::ToolState;

/// One run of `DrawNetworkLine`/`PlaceNetwork`: the drag inputs and the
/// counters the spec names (`+0x54`-`+0x80`), plus the bridges and tunnels
/// the passes recorded and the re-solve cells of §10.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ToolRun {
    /// The run's network type.
    pub type_id: u32,
    /// The clamped anchors (§3.1 step 1 / §3.3 step 5.4).
    pub start: (i32, i32),
    pub end: (i32, i32),
    /// `+0x70`: dragged step count.
    pub dragged_count: u32,
    /// `+0x74`: straight step count.
    pub straight_count: u32,
    /// `+0x78`: **valid** step count (lowered by the passes, §4).
    pub valid_count: u32,
    /// The dragged cells (§3.3 of the geometry).
    pub cells: Vec<(i32, i32)>,
    /// The bridge records of this run (§4.5 of the bridges).
    pub bridges: Vec<(u32, u32)>,
    /// The tunnel records (§6.2 of the geometry).
    pub tunnels: Vec<(u32, u32)>,
    /// §10: the cells to re-solve after a destroy (`(z << 16) + x` keys
    /// decoded into cells by the caller).
    pub resolve_cells: Vec<(i32, i32)>,
    /// The tool's `+0x208` at the moment of the insert (§6.2 step 5: the
    /// listeners see `0x99EF1142` with that `data4`). The core copies it
    /// from the state before every insert call.
    pub data4: u32,
}

impl ToolRun {
    pub fn new(type_id: u32, start: (i32, i32), end: (i32, i32)) -> ToolRun {
        ToolRun {
            type_id,
            start,
            end,
            ..ToolRun::default()
        }
    }
}

/// The pass result the host returns from `validate_and_solve` (§5.1 steps
/// 4-5): the valid count after the passes/solve and the failure code, if any
/// (§14 of the geometry: any code refuses the whole run).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PassResult {
    /// `+0x78` after the passes/solve.
    pub valid_count: u32,
    /// The failure code (`0xF0000001`-`0xF0000008`, `0xF0000011`-`13`), when
    /// a pass lowered the count or the solve failed.
    pub failure: Option<u32>,
}

/// A neighbour found while closing the destroy set (§10 step 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Neighbour {
    pub occ: u64,
    pub x: i32,
    pub z: i32,
    /// Network flag `0x400000`: an immovable prebuilt piece.
    pub immovable: bool,
    /// Network flag `0x100000`: part of an intersection/complex piece.
    pub intersection: bool,
}

/// Everything the tool asks of the game. Defaults: an empty world.
pub trait ToolHost {
    // ---- the drag and the pipeline (§3.2, §5) ----
    /// Slot 18 `DrawNetworkLine(&start, &end, snap, type)` over `run` (the
    /// geometry core's `draw_network_line`; the host binds it).
    fn draw_network_line(&mut self, _run: &mut ToolRun, _snap: bool) -> bool {
        false
    }
    /// Slot 53 `DoNetworkSpecificPlacement(arg3, neighbourMode)`: the base
    /// returns false; the power-line tool's override returns true (§9) and
    /// `PlaceNetwork` returns true at once.
    fn do_network_specific_placement(
        &mut self,
        _run: &mut ToolRun,
        _arg3: u32,
        _neighbour_mode: u32,
        _real: bool,
    ) -> bool {
        false
    }
    /// §5.1 steps 4-5: the validation passes and the solve (through the
    /// geometry core). Returns the valid count and the failure code.
    fn validate_and_solve(&mut self, _run: &mut ToolRun, _stop_on_failure: bool, _real: bool) -> PassResult {
        PassResult::default()
    }
    /// §5.1 step 7: `AttemptNeighborConnections` (true = one was found).
    fn attempt_neighbor_connections(&mut self, _run: &mut ToolRun) -> bool {
        false
    }
    /// §5.1 step 7: `PlaceNeighborConnections` (real mode only).
    fn place_neighbor_connections(&mut self, _run: &mut ToolRun) {}

    // ---- §3.3: the extension probes ----
    /// `GetNetworkOccupant(x, z)` with the OR filter on `mask` (occupant
    /// manager slot 32 + QI `0xA821EF94`): `Some(type)` = the network
    /// occupant's type when `IsOfType` matters (the caller compares with the
    /// run's type); `None` = no occupant or outside the city.
    fn get_network_occupant(&self, _x: i32, _z: i32, _mask: u32) -> Option<u32> {
        None
    }
    /// §3.3 step 6.2: the callback tool's `PlaceNetworkOccupants` (one-cell
    /// join in the callback tool's own type).
    fn callback_place(&mut self, _callback: u32, _ax: i32, _az: i32, _bx: i32, _bz: i32, _real: bool) {}

    // ---- the commit (§6) ----
    /// §6.1 step 1: slot 70 (stub), slot 60, slot 57, `MarkPositionChanged…`,
    /// `MarkTerrainChanged…` (real) or `UpdatePositionChangedPreviews…`
    /// (draft).
    fn commit_pre(&mut self, _run: &mut ToolRun, _real: bool) {}
    /// §6.1 step 2.1: slot 55 `InsertOccupantsForSolvedCells(&solved, &set,
    /// previewOnly)`. The run's `data4` field carries the tool's `+0x208`
    /// (the listeners see `0x99EF1142` with that `data4`, §6.2).
    fn insert_occupants_for_solved_cells(&mut self, _run: &mut ToolRun, _preview_only: bool) {}
    /// §6.1 step 2.2: `InsertBridgePieces` (`0x0062F610`).
    fn insert_bridge_pieces(&mut self, _run: &mut ToolRun) {}
    /// §6.1 step 2.3: `InsertTunnelPieces` (`0x006287D0`).
    fn insert_tunnel_pieces(&mut self, _run: &mut ToolRun) {}
    /// §6.1 step 2.4: `InsertHighwayIntersectionPieces` (`0x0062C4B0`).
    fn insert_highway_intersection_pieces(&mut self, _run: &mut ToolRun) {}
    /// §6.1 step 3.2's test: the type's two crew parameters (type record
    /// `+0x1C`, `+0x20`) both equal 1.
    fn crew_parameters_both_one(&self, _type_id: u32) -> bool {
        false
    }
    /// §6.1 step 3.1: `crew.AddNetworkSegment(&set, …)` (`0x00606B50`).
    fn crew_add_network_segment(&mut self, _crew: u64, _run: &mut ToolRun) {}
    /// §6.3 `MarkOccupantsUsable(set, release)`: per occupant — level-1 view
    /// clears `0x4000`; `SetVisibility(true, true)`; `SetNetworkFlag(
    /// 0x10000000)` (posts `0x89D3E9F3` data1 = 1, queued); if release,
    /// Release. Runs **unconditionally** in real mode (quirk 5).
    fn mark_occupants_usable(&mut self, _run: &mut ToolRun, _release: bool) {}
    /// §6.1 step 4 (draft): the occupants go into the preview set `+0x228`;
    /// `SetTerrainHilight`.
    fn commit_draft(&mut self, _run: &mut ToolRun) {}

    // ---- §4 `PlaceNetworkOccupantById` ----
    /// Slot 72 `InsertNetworkOccupant(&cell, &request, &out)` for the
    /// single-cell placement of §4 (uses the run's `data4`, which carries
    /// the tool's `+0x208`).
    fn insert_network_occupant_by_id(
        &mut self,
        _run: &mut ToolRun,
        _x: i32,
        _z: i32,
        _piece: u32,
        _usable: bool,
        _type_id: u32,
    ) {
    }

    // ---- §6.4 / §5.1: the cell queries ----
    /// Slot 9 `OccupantExists(x, z, mask, &id)` (zones.md §8.8): the draft
    /// query against the world cache. Returns `(exists, mask_out)`.
    fn occupant_exists(&self, _st: &ToolState, _x: i32, _z: i32, _flags: u32) -> (bool, [u8; 4]) {
        (false, [0; 4])
    }

    // ---- §10 `DestroyOccupantsAndResolve` ----
    /// The occupant's cell (network occupant slot 59).
    fn occupied_cell(&self, _occ: u64) -> Option<(i32, i32)> {
        None
    }
    /// The occupant's neighbours by network type (the closure, step 3).
    fn neighbours_by_network_type(&self, _occ: u64) -> Vec<Neighbour> {
        Vec::new()
    }
    /// Flag `0x100000`: part of an intersection piece.
    fn is_intersection_piece(&self, _occ: u64) -> bool {
        false
    }
    /// Network manager slot 12 `GetAllIntersectionPieces`.
    fn intersection_structure(&self, _occ: u64) -> Vec<u64> {
        Vec::new()
    }
    /// Flag `0x80000`: a tunnel portal (network-geometry.md §10.3).
    fn is_tunnel_portal(&self, _occ: u64) -> bool {
        false
    }
    /// The portal at the other end (QI `0x8A4BD52B` slot 59).
    fn paired_portal(&self, _occ: u64) -> Option<u64> {
        None
    }
    /// §10 step 5.1: `MessageSend` of `0xAA7A014D` (synchronous).
    fn send_pre_demolish_sync(&mut self, _occ: u64) {}
    /// §10 step 5.3 (destroyed cell): `occupantManager.RemoveOccupant(o,
    /// false, 0)`.
    fn remove_occupant(&mut self, _occ: u64, _shutdown: bool) {}
    /// §10 step 5.3 (closure piece): `RemoveOrReplaceOccupant` — true when a
    /// plain piece was put back.
    fn remove_or_replace_occupant(&mut self, _occ: u64) -> bool {
        false
    }
    /// §10 step 5.4: `AsOccupant().Shutdown()`; Release.
    fn occupant_shutdown(&self, _occ: u64) {}
    /// Release of a reference the tool holds.
    fn release(&self, _occ: u64) {}
    /// §10 step 6.1: `SolveNetwork(−1, true)` over the re-solve set.
    fn solve_network_resolve(&mut self, _run: &mut ToolRun) {}
    /// §10 step 6.3 / §6.3 with the re-inserted neighbours (`data4 = 1`).
    fn mark_occupants_usable_list(&mut self, _occ: &[u64]) {}

    // ---- §11 the underground refit ----
    /// Terrain slot 17: the cell's current corner altitudes.
    fn corner_altitudes(&self, _x: i32, _z: i32) -> [f32; 4] {
        [0.0; 4]
    }
    /// §11.1 step 3: `SmoothenNetwork(2)` over the solved records; the
    /// per-cell solved corner heights come back through
    /// [`ToolHost::find_underground_solved`].
    fn smoothen_underground(&mut self, _run: &mut ToolRun, _level: i32) -> bool {
        false
    }
    /// §11.1 step 4: `FindUndergroundOccupant(x, z, class)` finding a piece:
    /// `(occ, [4 corner heights])` from the solved record.
    fn find_underground_solved(&self, _x: i32, _z: i32) -> Option<(u64, [f32; 4])> {
        None
    }
    /// §11.1 step 5.4: `networkManager.DestroyNetworkOccupants(destroyList)`.
    fn destroy_network_occupants(&mut self, _occ: &[u64]) {}
    /// §11.1 step 5.4: `occupantManager.ReleaseOccupantList` (slot 48).
    fn release_occupant_list(&self, _occ: &[u64]) {}
    /// §11.1 step 6: slot 86 (true), `SetUsable(false)`, slot 50,
    /// `SetUsable(true)`, Release — in that order. The usable toggle posts
    /// `0x89D3E9F3` data1 = 0 then 1.
    fn underground_rebuild(&mut self, _occ: u64, _outcome: &UndergroundOutcome) {}
}

/// An empty world (tests and hosts that bind nothing).
#[derive(Clone, Copy, Debug, Default)]
pub struct NullHost;
impl ToolHost for NullHost {}

/// §4 `PlaceNetworkOccupantById(x, z, piece, usable, type)` (slot 8): one
/// piece at one cell, without a drag or a solve.
///
/// 1. Build a cell record for `(x, z)`: the type bit from the table
///    `0x00AA835C` (`1 << type`); for each of the 4 corner vertices
///    (`z × stride + x`, `+ stride`, `+ stride + 1`, `+ 1`):
///    `h := slot74(terrain.GetAltitude(vertex))` — slot 74 is the identity
///    for surface tools and `h − depth` for underground ones. The corner's
///    allowed range is `[h − tol, h + tol]`, `tol = +0x2C4`.
/// 2. Save the real byte; `+0x50 := 1`.
/// 3. `InsertNetworkOccupant(cell, {piece, mode 2}, &out)` (slot 72, §6.2).
///    The occupant-manager insert uses the current `+0x208`, which is 1 here.
/// 4. For each occupant in `out`: if `usable`, `SetUsable(true)` and
///    `ClearNetworkFlag(0x4000)`; then Release.
/// 5. Restore the real byte.
pub fn place_network_occupant_by_id(
    st: &mut ToolState,
    host: &mut dyn ToolHost,
    x: i32,
    z: i32,
    piece: u32,
    usable: bool,
    type_arg: i32,
) {
    let type_id = if type_arg < 0 { st.type_id } else { type_arg as u32 };
    // 2. Save the real byte; `+0x50 := 1`.
    let saved_real = st.real;
    st.real = true;
    // 3. Insert (the cell record construction is the host's — it owns the
    // terrain and slot 74). The run carries the tool's `+0x208`.
    let mut run = ToolRun::new(type_id, (x, z), (x, z));
    run.data4 = st.data4;
    host.insert_network_occupant_by_id(&mut run, x, z, piece, usable, type_id);
    // 5. Restore the real byte.
    st.real = saved_real;
}

/// §10's group collection helper: the caller of
/// [`destroy_occupants_and_resolve`] filters the list per layer group
/// (network-manager.md §7.2: surface `0x1F4F` → tool 0, pipes `0x10` → tool
/// 4, subways `0x80` → tool 7).
pub fn collect_group(all: &[Listed], mask_of: impl Fn(u64) -> Option<u32>, mask: u32) -> Vec<Listed> {
    all.iter()
        .filter(|o| mask_of(o.occ).is_some_and(|f| f & mask != 0))
        .copied()
        .collect()
}
