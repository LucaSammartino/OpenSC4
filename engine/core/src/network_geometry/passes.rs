// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The validation passes (§4): `IdentifyStartingConditions` (§4.2), the
//! tunnels (§6), `CheckForReservedTiles` (§5), the highlight pass (host),
//! the bridges (§7, with the bridge internals of NG2 left to the host),
//! `CheckForGoodTerrain` (§4.5) and the land bridges (host, NG4).

use super::drag::{Step, StepKind};
use super::params::{is_underground, TypeInfo};
use super::{GeometryHost, GeometryTerrain, Run};
use crate::x87::r24;

/// The failure codes (§14).
pub mod code {
    pub const BAD_TERRAIN: u32 = 0xF000_0001;
    pub const RESERVED: u32 = 0xF000_0002;
    pub const BRIDGE: u32 = 0xF000_0003;
    /// Never set (§6.3) *[quirk]*.
    pub const TUNNEL: u32 = 0xF000_0004;
    pub const INTERSECTION: u32 = 0xF000_0005;
    pub const SMOOTHING: u32 = 0xF000_0006;
    pub const UNSOLVED: u32 = 0xF000_0007;
    pub const TERRAIN_SUPPORT: u32 = 0xF000_0008;
    pub const POWER_ZONED: u32 = 0xF000_0011;
    pub const POWER_BUILDING: u32 = 0xF000_0012;
    pub const POWER_ROAD: u32 = 0xF000_0013;
}

/// What a cell's occupant is, for the reserved-tile test (§5.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellOccupant {
    /// A landfill occupant (`0x6A11BD2B`).
    Landfill,
    /// A network occupant (`0xC772BF98`): immovable (network occupant slot
    /// 17), a connection node (flag `0x200000`), its type (slot 22).
    Network {
        immovable: bool,
        connection_node: bool,
        type_id: u32,
    },
    /// A building (`0x278128A0`): skipped here (the highlight pass decides).
    Building,
    /// Anything else: not reserved.
    Other,
}

/// An extra cell (`+0xF0`, 20 bytes; who fills the list is NG3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExtraCell {
    pub x: i32,
    pub z: i32,
    pub type_id: u32,
}

/// A recorded tunnel: its portal steps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tunnel {
    pub start: u32,
    pub end: u32,
}

/// A recorded bridge: its abutment steps and the host's bridge type.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Bridge {
    pub start: u32,
    pub end: u32,
    pub bridge_type: u32,
}

/// A bridge type that `FindBestBridgeType` chose (NG2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BridgeChoice {
    /// The host's index of the bridge type.
    pub bridge_type: u32,
    /// Bridge type `+4`: the minimum span.
    pub min_span: u32,
}

/// The tool's per-variant switches (§1.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ToolFlags {
    /// Slot 61.
    pub allow_bridges: bool,
    /// Slot 62.
    pub allow_tunnels: bool,
    /// Slot 63.
    pub stopped_by_water: bool,
    /// Slot 68 is the base pass (false: the underground no-op).
    pub reserved_pass: bool,
    /// Slot 69 is the base pass (false: any terrain is good).
    pub terrain_pass: bool,
    /// Slot 59 is the base highlight pass (false: returns its argument).
    pub highlight_pass: bool,
    /// Slot 56 is the base terrain-support test (false: returns 1).
    pub support_test: bool,
    /// Slot 57 is the base grading (false: no-op).
    pub grading: bool,
    /// Tool `+0x354`: land bridges.
    pub land_bridges: bool,
}

impl ToolFlags {
    /// The base tool, or the underground tools' overrides for pipe and
    /// subway (§1.2). The power line uses the base slots listed here.
    pub fn for_type(type_id: u32) -> ToolFlags {
        let base = !is_underground(type_id);
        ToolFlags {
            allow_bridges: base,
            allow_tunnels: base,
            stopped_by_water: base,
            reserved_pass: base,
            terrain_pass: base,
            highlight_pass: base,
            support_test: base,
            grading: base,
            land_bridges: false,
        }
    }
}

/// `IsTerrainOK(x, z)` (§4.1): no water, or water under a connection node.
pub fn is_terrain_ok(t: &dyn GeometryTerrain, host: &dyn GeometryHost, x: i32, z: i32) -> bool {
    !t.cell_has_water(x, z) || host.is_connection_node(x, z)
}

/// Slot 67, one cell (§5.1): a landfill, or an immovable network occupant
/// that is not a connection node or not of this type.
pub fn is_cell_reserved(host: &dyn GeometryHost, x: i32, z: i32, type_id: u32) -> bool {
    host.cell_occupants(x, z).iter().any(|o| match *o {
        CellOccupant::Landfill => true,
        CellOccupant::Network {
            immovable,
            connection_node,
            type_id: t,
        } => immovable && (!connection_node || t != type_id),
        CellOccupant::Building | CellOccupant::Other => false,
    })
}

impl Run {
    fn step_cells(&self, i: u32) -> &[(i32, i32)] {
        let s = self.steps[i as usize];
        &self.cells[s.first as usize..=s.last as usize]
    }

    fn kind(&self, i: u32) -> StepKind {
        self.steps[i as usize].kind
    }

    fn regular(&self, i: u32) -> bool {
        self.kind(i) == StepKind::Regular
    }

    fn step_has_water(&self, t: &dyn GeometryTerrain, i: u32) -> bool {
        self.step_cells(i).iter().any(|&(x, z)| t.cell_has_water(x, z))
    }

    /// A step's mean height on the edge given by direction `d` (§6.1): the
    /// mean of the edge's two corners on the first cell and on the last
    /// cell. [inferred] Summed in that order, then divided by 4.
    pub fn step_mean(&self, t: &dyn GeometryTerrain, i: u32, d: u8) -> f32 {
        let s = self.steps[i as usize];
        let stride = t.cells().0 as i32 + 1;
        let (o1, o2) = edge_offsets(stride, d);
        let mut sum = 0.0f64;
        for c in [s.first, s.last] {
            let (x, z) = self.cells[c as usize];
            let base = z * stride + x;
            for o in [o1, o2] {
                sum = r24(sum + f64::from(t.vertex_altitude(base + o)));
            }
        }
        r24(sum / 4.0) as f32
    }
}

/// The per-direction vertex offset tables `+0x250` and `+0x270` (§6.1), for
/// the vertex row stride `s`.
pub fn edge_offsets(s: i32, d: u8) -> (i32, i32) {
    const A: [(i32, i32); 8] = [(1, 1), (1, 0), (1, 0), (0, 0), (0, 0), (0, 1), (0, 1), (1, 1)];
    const B: [(i32, i32); 8] = [(0, 1), (0, 1), (1, 1), (1, 1), (1, 0), (1, 0), (0, 0), (0, 0)];
    // (stride multiples, units): +0x250 = S+1, S, S, 0, 0, 1, 1, S+1;
    // +0x270 = 1, 1, S+1, S+1, S, S, 0, 0.
    let d = (d & 7) as usize;
    let v = |(m, u): (i32, i32)| m * s + u;
    (v(A[d]), v(B[d]))
}

/// The result of the passes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PassResult {
    /// The valid step count after the passes (`v`); 0 when refused.
    pub valid: u32,
    /// The failure code of the pass that lowered `v`, if any (§4). [inferred]
    /// When several passes lower it, the last one's code is kept.
    pub failure: Option<u32>,
}

/// §4.2 `IdentifyStartingConditions` (slot 66).
pub fn identify_starting_conditions(
    run: &Run,
    flags: &ToolFlags,
    t: &dyn GeometryTerrain,
    host: &dyn GeometryHost,
    v: &mut u32,
) {
    if flags.stopped_by_water {
        if let Some(&(x, z)) = run.cells.first() {
            if !is_terrain_ok(t, host, x, z) {
                *v = 0;
            }
        }
    }
}

/// §6.2 `IdentifyTunnelCells`: reads `v`, never writes it (§6.3).
pub fn identify_tunnel_cells(run: &mut Run, flags: &ToolFlags, info: &TypeInfo, t: &dyn GeometryTerrain, v: u32) {
    if !flags.allow_tunnels || v <= 3 {
        return;
    }
    let trailing = run.pen_direction;
    // 1. The start height: the trailing-edge mean of the first regular step.
    // [inferred] Only recorded; FindTunnelEnd gets the height of its own
    // start step (step 2), Q NG11.
    if let Some(first) = (0..v).find(|&i| run.regular(i)) {
        run.tunnel_start_height = Some(run.step_mean(t, first, trailing));
    }
    let mut i = 2;
    while i < v {
        if run.regular(i) && run.regular(i - 1) && run.regular(i - 2) && i < run.straight_count {
            // [inferred] mean(i) is the trailing-edge mean, like step 1.
            let mi = run.step_mean(t, i, trailing);
            let mp = run.step_mean(t, i - 1, trailing);
            if sub(mi, mp) > info.tunnel_trigger_rise {
                let start = i - 1;
                run.tunnel_searches.push(start);
                if let Some(end) = find_tunnel_end(run, info, t, v, start, mp) {
                    if end - start > 3 {
                        run.steps[start as usize].kind = StepKind::Portal;
                        run.steps[end as usize].kind = StepKind::Portal;
                        for k in start + 1..end {
                            run.steps[k as usize].kind = StepKind::TunnelInterior;
                        }
                        run.tunnels.push(Tunnel { start, end });
                        // [choice] The walk resumes after the tunnel.
                        i = end + 1;
                        continue;
                    }
                }
            }
        }
        i += 1;
    }
}

fn sub(a: f32, b: f32) -> f32 {
    r24(f64::from(a) - f64::from(b)) as f32
}

/// `FindTunnelEnd` (`0x00638770`, §6.2 step 3).
pub fn find_tunnel_end(
    run: &Run,
    info: &TypeInfo,
    t: &dyn GeometryTerrain,
    v: u32,
    start: u32,
    start_height: f32,
) -> Option<u32> {
    let leading = (run.pen_direction.wrapping_sub(4)) & 7;
    let clearance = info.params.tunnel_clearance.first().copied().unwrap_or(10.0);
    let ceiling = r24(f64::from(start_height) + f64::from(clearance)) as f32;
    // [inferred] The "previous step's mean" starts at the start height.
    let mut prev = start_height;
    let mut best: Option<(u32, f32, f32)> = None; // (step, |diff|, mean)
    let mut j = start + 1;
    while j + 1 < v && run.regular(j) && j < run.straight_count {
        if run.step_has_water(t, j) {
            return None;
        }
        let m = run.step_mean(t, j, leading);
        if m < prev {
            let d = sub(m, start_height).abs();
            if !matches!(best, Some((_, bd, _)) if bd <= d) {
                best = Some((j, d, m));
            }
        }
        prev = m;
        if m <= ceiling {
            break;
        }
        j += 1;
    }
    let (end, _, end_height) = best?;
    let length = end - start;
    let tol = info.tunnel_end_tolerance(length);
    (sub(end_height, start_height).abs() <= tol && length > 2).then_some(end)
}

/// §5.2 `CheckForReservedTiles(&v, type, skipSteps)` (slot 68).
pub fn check_for_reserved_tiles(run: &Run, host: &dyn GeometryHost, type_id: u32, skip_steps: bool, v: &mut u32) {
    if !skip_steps {
        'steps: for i in 0..*v {
            if run.kind(i) == StepKind::TunnelInterior {
                continue;
            }
            for &(x, z) in run.step_cells(i) {
                if is_cell_reserved(host, x, z, type_id) {
                    *v = i;
                    break 'steps;
                }
            }
        }
    }
    for e in &run.extra_cells {
        if is_cell_reserved(host, e.x, e.z, e.type_id) {
            *v = 0;
        }
    }
}

/// §7 `IdentifyBridgeCells` (`0x00632830`). The span selection follows §7
/// step 1; `IsStepGoodForBridge`, `CheckBridgeStepEdgeTypes`,
/// `AdjustBridgeSpan`, `FindBestBridgeType` and the elevation are the
/// host's (NG2).
pub fn identify_bridge_cells(
    run: &mut Run,
    flags: &ToolFlags,
    t: &dyn GeometryTerrain,
    host: &mut dyn GeometryHost,
    v: &mut u32,
) {
    if !flags.allow_bridges
        || *v <= 3
        || run.dragged_count > run.straight_count
        || host.bridge_type_count(run.type_id) == 0
    {
        return;
    }
    let mut i = 2;
    while i < *v {
        if !(run.regular(i) && run.regular(i - 1) && run.regular(i - 2)) {
            i += 1;
            continue;
        }
        let cells = run.step_cells(i).to_vec();
        if !(host.is_step_good_for_bridge(run, i)
            && cells.iter().any(|&(x, z)| t.cell_has_water(x, z))
            && host.bridge_step_edge_types_ok(run, i))
        {
            i += 1;
            continue;
        }
        // SelectBridgeStartAndEnd.
        let start = i - 1;
        let mut end = None;
        let mut j = i;
        while j < run.steps.len() as u32 {
            let dry = !run.step_has_water(t, j);
            if dry {
                end = Some(j);
                break;
            }
            if !(host.is_step_good_for_bridge(run, j) && host.bridge_step_edge_types_ok(run, j)) {
                break;
            }
            j += 1;
        }
        let span = end
            .filter(|&e| e + 1 < *v && e < run.straight_count)
            .and_then(|e| host.adjust_bridge_span(run, start, e));
        let choice = span.and_then(|(s, e)| host.find_best_bridge_type(run, s, e).map(|c| (s, e, c)));
        let Some((s, e, c)) = choice else {
            *v = i;
            return;
        };
        let len = e.saturating_sub(s);
        if len > 3 && c.min_span <= len + 1 {
            for k in s..=e {
                run.steps[k as usize].kind = StepKind::Bridge;
            }
            run.bridges.push(Bridge {
                start: s,
                end: e,
                bridge_type: c.bridge_type,
            });
            // [choice] The walk resumes after the bridge.
            i = e + 1;
            continue;
        }
        i += 1;
    }
}

/// §4.5 `CheckForGoodTerrain` (slot 69).
pub fn check_for_good_terrain(
    run: &Run,
    flags: &ToolFlags,
    t: &dyn GeometryTerrain,
    host: &dyn GeometryHost,
    v: &mut u32,
) {
    if !flags.stopped_by_water || !flags.terrain_pass {
        return;
    }
    'steps: for i in 0..*v {
        if run.kind(i) != StepKind::Regular {
            continue;
        }
        for &(x, z) in run.step_cells(i) {
            if t.cell_has_water(x, z) && !host.is_connection_node(x, z) {
                *v = i;
                break 'steps;
            }
        }
    }
    // [quirk] The extra cells: the first one that is dry, or is a connection
    // node, refuses the run.
    for e in &run.extra_cells {
        if !(t.cell_has_water(e.x, e.z) && !host.is_connection_node(e.x, e.z)) {
            *v = 0;
            break;
        }
    }
}

/// Runs the passes of §4 in order on the run's valid count. A pass that
/// lowers it sets its failure code, and the run is then refused (`v := 0`).
/// [inferred] The passes run once each, in the order of §4 (network-tools.md
/// §5.1 says "repeat until stable", Q NG12).
pub fn run_passes(
    run: &mut Run,
    flags: &ToolFlags,
    info: &TypeInfo,
    t: &dyn GeometryTerrain,
    host: &mut dyn GeometryHost,
) -> PassResult {
    let mut v = run.dragged_count;
    let mut failure = None;
    let mut check = |before: u32, after: u32, c: u32| {
        if after < before {
            failure = Some(c);
        }
    };
    // 1. IdentifyStartingConditions.
    let b = v;
    identify_starting_conditions(run, flags, t, host, &mut v);
    check(b, v, code::BAD_TERRAIN);
    // 2. IdentifyTunnelCells (never lowers v).
    identify_tunnel_cells(run, flags, info, t, v);
    // 3. CheckForReservedTiles.
    if flags.reserved_pass {
        let b = v;
        check_for_reserved_tiles(run, host, run.type_id, false, &mut v);
        check(b, v, code::RESERVED);
    }
    // 4. UpdateOccupantHighlights (network-tools.md §6; the host's).
    if flags.highlight_pass {
        let b = v;
        v = host.update_occupant_highlights(run, v).min(v);
        check(b, v, code::RESERVED);
    }
    // 5. IdentifyBridgeCells.
    let b = v;
    identify_bridge_cells(run, flags, t, host, &mut v);
    check(b, v, code::BRIDGE);
    // 6. CheckForGoodTerrain.
    let b = v;
    check_for_good_terrain(run, flags, t, host, &mut v);
    check(b, v, code::BAD_TERRAIN);
    // 7. IdentifyLandBridgeCells (never lowers v; NG4).
    if flags.land_bridges {
        host.identify_land_bridge_cells(run, v);
    }
    if failure.is_some() {
        v = 0;
    }
    PassResult { valid: v, failure }
}

/// The steps' kinds as integers (for the C ABI and vectors).
pub fn kinds(steps: &[Step]) -> Vec<u32> {
    steps.iter().map(|s| s.kind as u32).collect()
}
