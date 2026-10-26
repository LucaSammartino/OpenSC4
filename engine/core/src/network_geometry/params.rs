// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The network types, their widths, the placement parameters (§2.4, the
//! exemplars of group `0x084344E0`) and the per-type limits derived from
//! them (§2.3).

use crate::property::PropertyHolder;
use crate::props;
use crate::x87::r24;

/// The 13 network types (network-manager.md §3.1).
pub const TYPE_COUNT: usize = 13;

pub const ROAD: u32 = 0;
pub const RAIL: u32 = 1;
pub const HIGHWAY: u32 = 2;
pub const STREET: u32 = 3;
pub const PIPE: u32 = 4;
pub const POWER_LINE: u32 = 5;
pub const AVENUE: u32 = 6;
pub const SUBWAY: u32 = 7;
pub const LIGHT_RAIL: u32 = 8;
pub const MONORAIL: u32 = 9;
pub const ONE_WAY_ROAD: u32 = 10;
pub const DIRT_ROAD: u32 = 11;
pub const GROUND_HIGHWAY: u32 = 12;

/// The placement exemplars (§2.4): type and group.
pub const PLACEMENT_EXEMPLAR_TYPE: u32 = 0x6534_284A;
pub const PLACEMENT_EXEMPLAR_GROUP: u32 = 0x0843_44E0;

/// The placement exemplar's instance per network type (§2.4); power line has
/// none (its parameters stay at the defaults).
pub const PLACEMENT_EXEMPLAR_INSTANCES: [Option<u32>; TYPE_COUNT] = [
    Some(0x0781_214F),
    Some(0x0814_E612),
    Some(0xA843_3F6B),
    Some(0x492A_03A3),
    Some(0x0951_17DB),
    None,
    Some(0x2B73_0A17),
    Some(0x0A36_7CA7),
    Some(0xCB79_E089),
    Some(0xEBE0_8882),
    Some(0x8BDA_0C2B),
    Some(0xEBE0_8CDA),
    Some(0x0BE0_8C87),
];

/// The placement exemplar's property IDs (§2.4).
pub mod prop {
    pub const MAX_SLOPE: u32 = 0x879D_12E7;
    pub const SLOPE_04: u32 = 0x279D_1707;
    pub const MAX_SLOPE_CHANGE: u32 = 0x07B3_6C71;
    pub const MAX_RAISE: u32 = 0xC7B3_6CA4;
    pub const MAX_CUT: u32 = 0xC7B3_6CA5;
    pub const RESOLVE_TOLERANCE: u32 = 0xC804_DD86;
    pub const TUNNEL_TRIGGER_SLOPE: u32 = 0xC804_DD87;
    pub const TUNNEL_END_SLOPE: u32 = 0xC804_DD88;
    pub const PLANE_TOLERANCE: u32 = 0xC804_DD89;
    pub const NEW_CELL_TOLERANCE: u32 = 0xC804_DD93;
    pub const SUPPORT_THRESHOLD: u32 = 0xC804_DD94;
    pub const MAX_SUPPORT_HEIGHT: u32 = 0xC804_DD95;
    pub const TUNNEL_CLEARANCE: u32 = 0xE974_EAA6;
    pub const CELL_COST: u32 = 0x4A2D_49B9;
    pub const BRIDGE_COST: u32 = 0x8A2D_49EA;
    pub const TUNNEL_COST: u32 = 0x8A2D_49F0;
    pub const EARTHWORK_COST: u32 = 0x6A2D_49F4;
    pub const ELEVATED_IDS: u32 = 0xE974_EAA7;
}

/// Degrees to radians: the float constant 0.017453292 (`0x00AA8C60`, §2.4).
pub const DEG_TO_RAD: f32 = 0.017_453_292;
/// The cell size (terrain slot 6; tool `+0x238`).
pub const CELL_SIZE: f32 = 16.0;
/// `0x00AA8C50`: the diagonal factor of §2.3.
pub const DIAGONAL_FACTOR: f32 = 0.707_106_77;

/// The type's two widths in cells (type info `+0x1C`, `+0x20`, §2.3).
pub fn widths(type_id: u32) -> (u32, u32) {
    match type_id {
        HIGHWAY | AVENUE | GROUND_HIGHWAY => (2, 2),
        POWER_LINE => (1, 2),
        _ => (1, 1),
    }
}

/// Underground tools (pipe and subway, §1.2): no bridges, no tunnels, not
/// stopped by water, no reserved-tile pass, any terrain is good, no terrain
/// support test and no grading.
pub fn is_underground(type_id: u32) -> bool {
    type_id == PIPE || type_id == SUBWAY
}

/// One type's placement parameters as the exemplar gives them (§2.4): angles
/// in degrees, heights in metres. Host input.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacementParams {
    /// p+0x00, `0x879D12E7`: max slope (degrees).
    pub max_slope: f32,
    /// p+0x04, `0x279D1707` (degrees), feeds type info `+0x110` (NG6).
    pub slope_04: f32,
    /// p+0x08, `0x07B36C71`: max change of slope (degrees).
    pub max_slope_change: f32,
    /// p+0x0C, `0xC7B36CA4`: max terrain raise under the network (m).
    pub max_raise: f32,
    /// p+0x10, `0xC7B36CA5`: max terrain cut under the network (m).
    pub max_cut: f32,
    /// p+0x14, `0xC804DD86`: re-solve tolerance of an existing cell (m).
    pub resolve_tolerance: f32,
    /// p+0x18, `0xC804DD87`: tunnel trigger slope (degrees).
    pub tunnel_trigger_slope: f32,
    /// p+0x1C, `0xC804DD88`: tunnel end slope (degrees).
    pub tunnel_end_slope: f32,
    /// p+0x20: regular-cell plane tolerance in metres (section 9.8).
    pub plane_tolerance: f32,
    /// p+0x25: straight only (not read from the exemplar; 1 for street).
    pub straight_only: bool,
    /// p+0x34, `0xC804DD93`: new-cell tolerance (m).
    pub new_cell_tolerance: f32,
    /// p+0x38, `0xC804DD94`: support height threshold (m).
    pub support_threshold: f32,
    /// p+0x3C, `0xC804DD95`: max height above the graded terrain with
    /// supports (m).
    pub max_support_height: f32,
    /// p+0x6C, `0xE974EAA6`: the tunnel clearance (default `[10.0]`).
    pub tunnel_clearance: Vec<f32>,
    /// p+0x8C, `0x4A2D49B9`: cost per cell (default 1).
    pub cell_cost: i32,
    /// p+0x90, `0x8A2D49EA`: bridge cost per cell (default 10).
    pub bridge_cost: i32,
    /// p+0x94, `0x8A2D49F0`: tunnel cost per cell (default 20).
    pub tunnel_cost: i32,
    /// p+0x98, `0x6A2D49F4`: earthwork cost per metre (default 0.1).
    pub earthwork_cost: f32,
    /// p+0xA0, `0xE974EAA7`: the id list; non-empty only for highway, light
    /// rail and monorail (§9.2's flat-cell test reads only whether it is
    /// empty).
    pub elevated_ids: Vec<u32>,
}

impl Default for PlacementParams {
    /// The defaults that apply when the exemplar or a property is missing.
    /// §2.4 gives the defaults of the tunnel clearance and the four costs.
    /// [choice] The other fields have no stated default (Q NG8): 0.
    fn default() -> Self {
        PlacementParams {
            max_slope: 0.0,
            slope_04: 0.0,
            max_slope_change: 0.0,
            max_raise: 0.0,
            max_cut: 0.0,
            resolve_tolerance: 0.0,
            tunnel_trigger_slope: 0.0,
            tunnel_end_slope: 0.0,
            plane_tolerance: 0.0,
            straight_only: false,
            new_cell_tolerance: 0.0,
            support_threshold: 0.0,
            max_support_height: 0.0,
            tunnel_clearance: vec![10.0],
            cell_cost: 1,
            bridge_cost: 10,
            tunnel_cost: 20,
            earthwork_cost: 0.1,
            elevated_ids: Vec::new(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn shipped_row(
    slopes: [f32; 3],
    raise_cut: [f32; 2],
    resolve: f32,
    tunnel: [f32; 2],
    tolerances: [f32; 2],
    max_support_height: f32,
    clearance: &[f32],
    costs: [i32; 3],
) -> PlacementParams {
    let mut p = PlacementParams {
        max_slope: slopes[0],
        slope_04: slopes[1],
        max_slope_change: slopes[2],
        max_raise: raise_cut[0],
        max_cut: raise_cut[1],
        resolve_tolerance: resolve,
        tunnel_trigger_slope: tunnel[0],
        tunnel_end_slope: tunnel[1],
        plane_tolerance: 15.0,
        new_cell_tolerance: tolerances[0],
        support_threshold: tolerances[1],
        max_support_height,
        cell_cost: costs[0],
        bridge_cost: costs[1],
        tunnel_cost: costs[2],
        ..PlacementParams::default()
    };
    if !clearance.is_empty() {
        p.tunnel_clearance = clearance.to_vec();
    }
    p
}

impl PlacementParams {
    /// The shipped exemplar values of §2.4 (tests and tools; the game reads
    /// the exemplars). Absent properties take the defaults. [choice] The
    /// highway's p+0xA0 list is "1 id" with no value given: a single 0 stands
    /// for it (only its non-emptiness is read, §9.2).
    pub fn shipped(type_id: u32) -> PlacementParams {
        let road = || {
            let mut p = shipped_row(
                [35.0, 40.0, 35.0],
                [25.0, 10000.0],
                10.0,
                [35.0, 40.0],
                [500.0, 45.0],
                55.0,
                &[13.8],
                [10, 100, 20],
            );
            p.earthwork_cost = 0.1;
            p.plane_tolerance = 35.0;
            p
        };
        let rail = |cell_cost: i32, tunnel_cost: i32, clearance: f32, trigger: f32| {
            shipped_row(
                [25.0, 30.0, 25.0],
                [25.0, 10000.0],
                101.0,
                [trigger, 25.0],
                [180.0, 10.0],
                30.0,
                &[clearance],
                [cell_cost, 100, tunnel_cost],
            )
        };
        let highway = |cell_cost: i32, tunnel_cost: i32, clearance: f32| {
            shipped_row(
                [20.0, 30.0, 20.0],
                [50.0, 10000.0],
                101.0,
                [20.0, 20.0],
                [180.0, 10.0],
                30.0,
                &[clearance, clearance],
                [cell_cost, 1000, tunnel_cost],
            )
        };
        match type_id {
            ROAD | ONE_WAY_ROAD | DIRT_ROAD => road(),
            RAIL => rail(8, 20, 7.25, 25.0),
            HIGHWAY => {
                let mut p = highway(300, 400, 23.5);
                p.elevated_ids = vec![0];
                p
            }
            STREET => {
                let mut p = shipped_row(
                    [40.0, 40.0, 40.0],
                    [50000.0, 10000.0],
                    500.0,
                    [89.5, 40.0],
                    [50000.0, 10.0],
                    30.0,
                    &[],
                    [5, 10, 20],
                );
                p.straight_only = true;
                p
            }
            PIPE => shipped_row(
                [45.0, 40.0, 45.0],
                [0.0, 45.0],
                500.0,
                [89.5, 45.0],
                [40.0, 10.0],
                30.0,
                &[],
                [5, 10, 20],
            ),
            AVENUE => shipped_row(
                [40.0, 30.0, 40.0],
                [50.0, 10000.0],
                101.0,
                [40.0, 40.0],
                [180.0, 10.0],
                30.0,
                &[7.5, 7.5],
                [30, 1000, 20],
            ),
            SUBWAY => shipped_row(
                [35.0, 40.0, 35.0],
                [0.0, 35.0],
                300.0,
                [89.5, 35.0],
                [40.0, 10.0],
                30.0,
                &[],
                [150, 10, 20],
            ),
            LIGHT_RAIL => {
                let mut p = rail(50, 150, 7.25, 89.0);
                p.elevated_ids = vec![0];
                p
            }
            MONORAIL => {
                let mut p = rail(100, 150, 22.4, 25.0);
                p.elevated_ids = vec![0];
                p
            }
            GROUND_HIGHWAY => highway(100, 200, 11.25),
            _ => PlacementParams::default(),
        }
    }

    /// Reads one type's placement exemplar (§2.4; no parent cohort). A
    /// missing exemplar or property keeps the default. p+0x25 is not read:
    /// it is 1 only for street.
    pub fn from_holder(type_id: u32, h: Option<&PropertyHolder>) -> PlacementParams {
        let mut p = PlacementParams {
            straight_only: type_id == STREET,
            ..PlacementParams::default()
        };
        let Some(h) = h else {
            return p;
        };
        let f = |id: u32, v: &mut f32| {
            if let Some(x) = props::f32_of(h, id) {
                *v = x;
            }
        };
        f(prop::MAX_SLOPE, &mut p.max_slope);
        f(prop::SLOPE_04, &mut p.slope_04);
        f(prop::MAX_SLOPE_CHANGE, &mut p.max_slope_change);
        f(prop::MAX_RAISE, &mut p.max_raise);
        f(prop::MAX_CUT, &mut p.max_cut);
        f(prop::RESOLVE_TOLERANCE, &mut p.resolve_tolerance);
        f(prop::TUNNEL_TRIGGER_SLOPE, &mut p.tunnel_trigger_slope);
        f(prop::TUNNEL_END_SLOPE, &mut p.tunnel_end_slope);
        f(prop::PLANE_TOLERANCE, &mut p.plane_tolerance);
        f(prop::NEW_CELL_TOLERANCE, &mut p.new_cell_tolerance);
        f(prop::SUPPORT_THRESHOLD, &mut p.support_threshold);
        f(prop::MAX_SUPPORT_HEIGHT, &mut p.max_support_height);
        f(prop::EARTHWORK_COST, &mut p.earthwork_cost);
        if let Some(v) = props::f32s(h, prop::TUNNEL_CLEARANCE) {
            if !v.is_empty() {
                p.tunnel_clearance = v;
            }
        }
        let i = |id: u32, v: &mut i32| {
            if let Some(x) = props::u32_of(h, id) {
                *v = x as i32;
            }
        };
        i(prop::CELL_COST, &mut p.cell_cost);
        i(prop::BRIDGE_COST, &mut p.bridge_cost);
        i(prop::TUNNEL_COST, &mut p.tunnel_cost);
        if let Some(v) = props::u32s(h, prop::ELEVATED_IDS) {
            p.elevated_ids = v;
        }
        p
    }
}

/// `r24(deg × 0.017453292)`: an angle as the original stores it (radians).
pub fn radians(deg: f32) -> f32 {
    r24(f64::from(deg) * f64::from(DEG_TO_RAD)) as f32
}

/// `r24(tan(rad) × 16)` (`0x0063D460`, §2.3): `fptan` on the stored radians
/// (transcendentals are not narrowed by the precision control), then one
/// rounded product.
pub fn rise_per_cell(rad: f32) -> f32 {
    r24(f64::from(rad).tan() * f64::from(CELL_SIZE)) as f32
}

/// The per-type limits derived from the placement parameters (§2.3) and the
/// tool's per-type copies (§2.1). Every value is a float as stored.
#[derive(Clone, Debug, PartialEq)]
pub struct TypeInfo {
    pub type_id: u32,
    /// Type info `+0x1C`, `+0x20`.
    pub widths: (u32, u32),
    pub params: PlacementParams,
    /// `+0x100`: `r24(tan(p+0x00) × 16)`, the straight slope rise per cell.
    pub slope_rise: f32,
    /// `+0x104`: `r24(+0x100 × 0.70710677)`.
    pub diagonal_slope_rise: f32,
    /// `+0x108`: `r24(tan(p+0x08) × 16)`, the straight smoothness limit.
    pub smoothness: f32,
    /// `+0x10C`: `r24(+0x108 × 0.70710677)`.
    pub diagonal_smoothness: f32,
    /// `+0x110`: `r24(tan(p+0x04) × 16)`; no reader found (NG6).
    pub rise_04: f32,
    /// Tool `+0x23C`: the tunnel trigger rise per step, `tan(p+0x18) ×
    /// +0x238`. [inferred] Stored as a float like the others.
    pub tunnel_trigger_rise: f32,
    /// Tool `+0x2AC`: the tunnel end slope in radians.
    pub tunnel_end_slope_rad: f32,
}

impl TypeInfo {
    pub fn new(type_id: u32, params: PlacementParams) -> TypeInfo {
        let slope_rise = rise_per_cell(radians(params.max_slope));
        let smoothness = rise_per_cell(radians(params.max_slope_change));
        let diag = |v: f32| r24(f64::from(v) * f64::from(DIAGONAL_FACTOR)) as f32;
        TypeInfo {
            type_id,
            widths: widths(type_id),
            slope_rise,
            diagonal_slope_rise: diag(slope_rise),
            smoothness,
            diagonal_smoothness: diag(smoothness),
            rise_04: rise_per_cell(radians(params.slope_04)),
            tunnel_trigger_rise: rise_per_cell(radians(params.tunnel_trigger_slope)),
            tunnel_end_slope_rad: radians(params.tunnel_end_slope),
            params,
        }
    }

    /// The shipped type info (§2.4 values).
    pub fn shipped(type_id: u32) -> TypeInfo {
        TypeInfo::new(type_id, PlacementParams::shipped(type_id))
    }

    /// The type has a non-empty p+0xA0 list (§9.2's elevated types).
    pub fn is_elevated_type(&self) -> bool {
        !self.params.elevated_ids.is_empty()
    }

    /// `r24(r24(tan(+0x2AC) × 16) × length)`: the largest height difference
    /// between a tunnel's ends (§6.2 step 3). [inferred] The operation order.
    pub fn tunnel_end_tolerance(&self, length: u32) -> f32 {
        let per_cell = rise_per_cell(self.tunnel_end_slope_rad);
        r24(f64::from(per_cell) * f64::from(length)) as f32
    }
}

/// The 13 types' infos, indexed by type.
#[derive(Clone, Debug, PartialEq)]
pub struct TypeTable {
    pub infos: Vec<TypeInfo>,
}

impl TypeTable {
    pub fn new(params: Vec<PlacementParams>) -> TypeTable {
        let mut params = params;
        params.resize(TYPE_COUNT, PlacementParams::default());
        TypeTable {
            infos: params
                .into_iter()
                .enumerate()
                .map(|(t, p)| TypeInfo::new(t as u32, p))
                .collect(),
        }
    }

    pub fn shipped() -> TypeTable {
        TypeTable::new((0..TYPE_COUNT as u32).map(PlacementParams::shipped).collect())
    }

    pub fn get(&self, type_id: u32) -> Option<&TypeInfo> {
        self.infos.get(type_id as usize)
    }
}

impl Default for TypeTable {
    fn default() -> Self {
        TypeTable::shipped()
    }
}
