// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The network tool's geometry over the C ABI (fc4.h "network geometry",
//! docs/specs/network-geometry.md §3-§9): the drag, the validation passes,
//! the cell infos, the smoothing loop, `CanTilesBeSupportedOnTerrain` and
//! the grading, plus the standalone height satisfier ([`fc4_vhcs`]) and the
//! pure helpers of §10, §11 and §13. No plugin glue yet.
//!
//! The terrain is read through the host table, or through a core terrain
//! ([`fc4_netgeom_set_terrain`]): then the geometry reads it under its lock
//! and the grading writes it (its messages go to the terrain's `post`).
//! Everything else the geometry asks the world goes through the host table;
//! a missing callback is the empty world (no occupants, no connection nodes,
//! no bridge types; the bridge checks pass, the highlight pass keeps the
//! count).
//!
//! Not exposed yet: the land-bridge pass (NG4) and the piece solver's extra
//! constraints (NG5); both keep the core's empty defaults.
//!
//! Threads: one thread at a time. A callback that calls back into the same
//! geometry is refused (fallback value, counted in
//! `fc4_netgeom_state.reentrant_refused`).

use std::cell::{Cell, RefCell};
use std::ffi::c_void;

use fc4_core::network_geometry::extras::{self, cost_of_solution};
use fc4_core::network_geometry::params::{PlacementParams, TYPE_COUNT};
use fc4_core::network_geometry::smoothing::terrain_vertex_constraint;
use fc4_core::network_geometry::{
    break_into_segments, BridgeChoice, CellInfo, CellOccupant, ExistingPiece, GeometryHost, GeometryTerrain,
    GradingTerrain, NetworkGeometry, Run, StepKind, TerrainVertex, TypeInfo, TypeTable, VertexHtConstraintSatisfier,
};
use fc4_core::terrain::{Terrain, VertRect};

use crate::guard;
use crate::props::{fc4_props, props_ref};
use crate::terrain::{fc4_terrain, fc4_terrain_rect, read_terrain, write_terrain};

type U = *mut c_void;

/// Satisfier selectors (`fc4_netgeom_sat_*`).
pub const FC4_NETGEOM_SAT_NETWORK: u32 = 0;
pub const FC4_NETGEOM_SAT_TERRAIN: u32 = 1;

/// `fc4_netgeom_occupant.kind`.
pub const FC4_NETGEOM_OCC_OTHER: u32 = 0;
pub const FC4_NETGEOM_OCC_LANDFILL: u32 = 1;
pub const FC4_NETGEOM_OCC_NETWORK: u32 = 2;
pub const FC4_NETGEOM_OCC_BUILDING: u32 = 3;

/// `fc4_netgeom_terrain_vertex` results (§9.6 step 3).
pub const FC4_NETGEOM_TV_FIX: u32 = 0;
pub const FC4_NETGEOM_TV_RANGE: u32 = 1;
pub const FC4_NETGEOM_TV_UNSUPPORTED: u32 = 2;
pub const FC4_NETGEOM_TV_INVALID: u32 = 0xFFFF_FFFF;

#[repr(C)]
pub struct fc4_netgeom {
    _private: [u8; 0],
}

#[repr(C)]
pub struct fc4_vhcs {
    _private: [u8; 0],
}

/// One type's placement parameters (§2.4, exemplar group `0x084344E0`):
/// angles in degrees, heights in metres.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_netgeom_params {
    pub struct_size: u32,
    pub max_slope: f32,
    pub slope_04: f32,
    pub max_slope_change: f32,
    pub max_raise: f32,
    pub max_cut: f32,
    pub resolve_tolerance: f32,
    pub tunnel_trigger_slope: f32,
    pub tunnel_end_slope: f32,
    pub new_cell_tolerance: f32,
    pub support_threshold: f32,
    pub max_support_height: f32,
    /// The first entry of `0xE974EAA6` (the only one read).
    pub tunnel_clearance: f32,
    pub earthwork_cost: f32,
    pub cell_cost: i32,
    pub bridge_cost: i32,
    pub tunnel_cost: i32,
    /// The length of the `0xE974EAA7` list (only whether it is empty is
    /// read, §9.2).
    pub elevated_id_count: u32,
    pub straight_only: u8,
    pub reserved: [u8; 3],
}

/// The per-type limits (§2.3).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_netgeom_type_info {
    pub struct_size: u32,
    pub type_id: u32,
    pub width1: u32,
    pub width2: u32,
    pub slope_rise: f32,
    pub diagonal_slope_rise: f32,
    pub smoothness: f32,
    pub diagonal_smoothness: f32,
    pub rise_04: f32,
    pub tunnel_trigger_rise: f32,
    pub tunnel_end_slope_rad: f32,
}

/// `BreakIntoStraightAndDiagSegments`'s outputs (§3.2).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_netgeom_segments {
    pub struct_size: u32,
    pub end_x: i32,
    pub end_z: i32,
    pub dragged_count: u32,
    pub straight_count: u32,
    pub axis0: u8,
    pub axis1: u8,
    pub pen_direction: u8,
    pub end_direction: u8,
}

/// A cell (x, z).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_netgeom_cell {
    pub x: i32,
    pub z: i32,
}

/// A dragged step (§2.2): the first and last index into the cells; kind 0
/// regular, 1 bridge, 2 tunnel portal, 3 tunnel interior.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_netgeom_step {
    pub first: u32,
    pub last: u32,
    pub kind: u32,
}

/// A tunnel or bridge: its end steps (and a bridge's type; 0 for tunnels).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_netgeom_span {
    pub start: u32,
    pub end: u32,
    pub bridge_type: u32,
}

/// A solved vertex height.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_netgeom_vertex_height {
    pub vertex: i32,
    pub height: f32,
}

/// A cell's occupant for the reserved-tile test (§5.1); the host fills it.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_netgeom_occupant {
    /// `FC4_NETGEOM_OCC_*`.
    pub kind: u32,
    pub immovable: u8,
    pub connection_node: u8,
    pub reserved: [u8; 2],
    pub type_id: u32,
}

/// The existing network piece in a cell (§8.1); the host fills it.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_netgeom_piece {
    pub flags: u32,
    pub immovable: u8,
    pub has_supports: u8,
    pub reserved: [u8; 2],
    pub corners: [f32; 4],
    pub edges: [u32; TYPE_COUNT],
}

/// `cSC4NetworkCellInfo` (§8.1), the fields the height solver reads. A
/// connection node's fixed vertices are not carried (inserted infos have
/// none: the four corners at their corner heights).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_netgeom_cell_info {
    pub struct_size: u32,
    pub x: i32,
    pub z: i32,
    pub flags: u32,
    pub step: i32,
    pub edges: [u32; TYPE_COUNT],
    pub edge_union: u32,
    pub in_run: u8,
    pub placed: u8,
    pub intersection: u8,
    pub fixed: u8,
    pub tunnel_interior: u8,
    pub connection_node: u8,
    pub has_supports: u8,
    pub reserved: u8,
    pub corners: [f32; 4],
    /// (lo, hi) per corner.
    pub allowed: [f32; 8],
    pub preferred: [f32; 8],
    pub vertices: [i32; 4],
}

/// Diagnostics (`fc4_netgeom_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_netgeom_state {
    pub struct_size: u32,
    pub type_id: u32,
    pub start_x: i32,
    pub start_z: i32,
    pub end_x: i32,
    pub end_z: i32,
    pub dragged_count: u32,
    pub straight_count: u32,
    pub axis0: u8,
    pub axis1: u8,
    pub pen_direction: u8,
    pub end_direction: u8,
    pub valid: u32,
    pub failure: u32,
    pub cell_count: u32,
    pub step_count: u32,
    pub extra_cell_count: u32,
    pub tunnel_count: u32,
    pub bridge_count: u32,
    pub cell_info_count: u32,
    pub solved_count: u32,
    pub smoothing_cell_count: u32,
    pub boundary_count: u32,
    /// The level the smoothing loop succeeded at; −1 = none.
    pub level: i32,
    /// §6.2 step 1's start height; NaN = none.
    pub tunnel_start_height: f32,
    pub reentrant_refused: u32,
    pub terrain_bound: u8,
    pub reserved: [u8; 3],
}

/// What the geometry needs from the game (fc4.h `fc4_netgeom_host`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_netgeom_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    // terrain (unused while a core terrain is bound)
    pub cells: Option<unsafe extern "C" fn(U, *mut u32, *mut u32)>,
    pub vertex_altitude: Option<unsafe extern "C" fn(U, i32) -> f32>,
    pub cell_has_water: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    pub surface_water_level: Option<unsafe extern "C" fn(U, i32) -> f32>,
    pub max_allowed_height: Option<unsafe extern "C" fn(U) -> f32>,
    pub set_altitude_at_vertex: Option<unsafe extern "C" fn(U, i32, f32)>,
    pub begin_long_terrain_operation: Option<unsafe extern "C" fn(U)>,
    pub redisplay_terrain: Option<unsafe extern "C" fn(U, *const fc4_terrain_rect, u32)>,
    pub end_long_terrain_operation: Option<unsafe extern "C" fn(U)>,
    // world
    pub is_connection_node: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    pub cell_occupants: Option<unsafe extern "C" fn(U, i32, i32, *mut fc4_netgeom_occupant, u32) -> u32>,
    pub existing_piece: Option<unsafe extern "C" fn(U, i32, i32, *mut fc4_netgeom_piece) -> bool>,
    pub extends_diagonal_end: Option<unsafe extern "C" fn(U, i32, i32, u32) -> bool>,
    pub update_occupant_highlights: Option<unsafe extern "C" fn(U, u32, *const fc4_netgeom_cell, u32, u32) -> u32>,
    pub bridge_type_count: Option<unsafe extern "C" fn(U, u32) -> u32>,
    pub is_step_good_for_bridge: Option<unsafe extern "C" fn(U, u32, u32, *const fc4_netgeom_cell, u32) -> bool>,
    pub bridge_step_edge_types_ok: Option<unsafe extern "C" fn(U, u32, u32, *const fc4_netgeom_cell, u32) -> bool>,
    pub adjust_bridge_span: Option<unsafe extern "C" fn(U, u32, *mut u32, *mut u32) -> bool>,
    pub find_best_bridge_type: Option<unsafe extern "C" fn(U, u32, u32, u32, *mut u32, *mut u32) -> bool>,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_netgeom_params>() == 76);
    assert!(std::mem::size_of::<fc4_netgeom_type_info>() == 44);
    assert!(std::mem::size_of::<fc4_netgeom_segments>() == 24);
    assert!(std::mem::size_of::<fc4_netgeom_cell>() == 8);
    assert!(std::mem::size_of::<fc4_netgeom_step>() == 12);
    assert!(std::mem::size_of::<fc4_netgeom_span>() == 12);
    assert!(std::mem::size_of::<fc4_netgeom_vertex_height>() == 8);
    assert!(std::mem::size_of::<fc4_netgeom_occupant>() == 12);
    assert!(std::mem::size_of::<fc4_netgeom_piece>() == 76);
    assert!(std::mem::size_of::<fc4_netgeom_cell_info>() == 180);
    assert!(std::mem::size_of::<fc4_netgeom_state>() == 96);
    assert!(std::mem::size_of::<fc4_netgeom_host>() == if cfg!(target_pointer_width = "32") { 84 } else { 168 });
};

// ---- conversions ------------------------------------------------------------------------------------

impl From<&PlacementParams> for fc4_netgeom_params {
    fn from(p: &PlacementParams) -> Self {
        fc4_netgeom_params {
            struct_size: std::mem::size_of::<fc4_netgeom_params>() as u32,
            max_slope: p.max_slope,
            slope_04: p.slope_04,
            max_slope_change: p.max_slope_change,
            max_raise: p.max_raise,
            max_cut: p.max_cut,
            resolve_tolerance: p.resolve_tolerance,
            tunnel_trigger_slope: p.tunnel_trigger_slope,
            tunnel_end_slope: p.tunnel_end_slope,
            new_cell_tolerance: p.new_cell_tolerance,
            support_threshold: p.support_threshold,
            max_support_height: p.max_support_height,
            tunnel_clearance: p.tunnel_clearance.first().copied().unwrap_or(10.0),
            earthwork_cost: p.earthwork_cost,
            cell_cost: p.cell_cost,
            bridge_cost: p.bridge_cost,
            tunnel_cost: p.tunnel_cost,
            elevated_id_count: p.elevated_ids.len() as u32,
            straight_only: u8::from(p.straight_only),
            reserved: [0; 3],
        }
    }
}

impl From<&fc4_netgeom_params> for PlacementParams {
    fn from(p: &fc4_netgeom_params) -> Self {
        PlacementParams {
            max_slope: p.max_slope,
            slope_04: p.slope_04,
            max_slope_change: p.max_slope_change,
            max_raise: p.max_raise,
            max_cut: p.max_cut,
            resolve_tolerance: p.resolve_tolerance,
            tunnel_trigger_slope: p.tunnel_trigger_slope,
            tunnel_end_slope: p.tunnel_end_slope,
            straight_only: p.straight_only != 0,
            new_cell_tolerance: p.new_cell_tolerance,
            support_threshold: p.support_threshold,
            max_support_height: p.max_support_height,
            tunnel_clearance: vec![p.tunnel_clearance],
            cell_cost: p.cell_cost,
            bridge_cost: p.bridge_cost,
            tunnel_cost: p.tunnel_cost,
            earthwork_cost: p.earthwork_cost,
            elevated_ids: vec![0; p.elevated_id_count.min(64) as usize],
        }
    }
}

impl From<&TypeInfo> for fc4_netgeom_type_info {
    fn from(t: &TypeInfo) -> Self {
        fc4_netgeom_type_info {
            struct_size: std::mem::size_of::<fc4_netgeom_type_info>() as u32,
            type_id: t.type_id,
            width1: t.widths.0,
            width2: t.widths.1,
            slope_rise: t.slope_rise,
            diagonal_slope_rise: t.diagonal_slope_rise,
            smoothness: t.smoothness,
            diagonal_smoothness: t.diagonal_smoothness,
            rise_04: t.rise_04,
            tunnel_trigger_rise: t.tunnel_trigger_rise,
            tunnel_end_slope_rad: t.tunnel_end_slope_rad,
        }
    }
}

fn pairs(v: &[(f32, f32); 4]) -> [f32; 8] {
    std::array::from_fn(|k| if k % 2 == 0 { v[k / 2].0 } else { v[k / 2].1 })
}

fn unpairs(v: &[f32; 8]) -> [(f32, f32); 4] {
    std::array::from_fn(|k| (v[2 * k], v[2 * k + 1]))
}

impl From<&CellInfo> for fc4_netgeom_cell_info {
    fn from(c: &CellInfo) -> Self {
        fc4_netgeom_cell_info {
            struct_size: std::mem::size_of::<fc4_netgeom_cell_info>() as u32,
            x: c.x,
            z: c.z,
            flags: c.flags,
            step: c.step,
            edges: c.edges,
            edge_union: c.edge_union,
            in_run: u8::from(c.in_run),
            placed: u8::from(c.placed),
            intersection: u8::from(c.intersection),
            fixed: u8::from(c.fixed),
            tunnel_interior: u8::from(c.tunnel_interior),
            connection_node: u8::from(c.connection_node),
            has_supports: u8::from(c.has_supports),
            reserved: 0,
            corners: c.corners,
            allowed: pairs(&c.allowed),
            preferred: pairs(&c.preferred),
            vertices: c.vertices,
        }
    }
}

impl From<&fc4_netgeom_cell_info> for CellInfo {
    fn from(c: &fc4_netgeom_cell_info) -> Self {
        CellInfo {
            x: c.x,
            z: c.z,
            flags: c.flags,
            in_run: c.in_run != 0,
            step: c.step,
            edges: c.edges,
            edge_union: c.edge_union,
            placed: c.placed != 0,
            intersection: c.intersection != 0,
            fixed: c.fixed != 0,
            tunnel_interior: c.tunnel_interior != 0,
            connection_node: c.connection_node != 0,
            corners: c.corners,
            allowed: unpairs(&c.allowed),
            preferred: unpairs(&c.preferred),
            vertices: c.vertices,
            has_supports: c.has_supports != 0,
            node_fixed: Vec::new(),
        }
    }
}

fn c_cells(cells: &[(i32, i32)]) -> Vec<fc4_netgeom_cell> {
    cells.iter().map(|&(x, z)| fc4_netgeom_cell { x, z }).collect()
}

// ---- host adapters ----------------------------------------------------------------------------------

macro_rules! cb {
    ($h:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $h.$f { Some(f) => unsafe { f($h.user $(, $arg)*) }, None => $default }
    };
}

/// The terrain through the host table.
struct CTerrain<'a>(&'a fc4_netgeom_host);

impl GeometryTerrain for CTerrain<'_> {
    fn cells(&self) -> (u32, u32) {
        let (mut x, mut z) = (0u32, 0u32);
        cb!(self.0, cells, (), &mut x, &mut z);
        (x, z)
    }
    fn vertex_altitude(&self, v: i32) -> f32 {
        cb!(self.0, vertex_altitude, 0.0, v)
    }
    fn cell_has_water(&self, x: i32, z: i32) -> bool {
        cb!(self.0, cell_has_water, false, x, z)
    }
    fn surface_water_level(&self, v: i32) -> f32 {
        cb!(self.0, surface_water_level, f32::NAN, v)
    }
    fn max_allowed_height(&self) -> f32 {
        cb!(self.0, max_allowed_height, 0.0)
    }
}

impl GradingTerrain for CTerrain<'_> {
    fn set_altitude_at_vertex(&mut self, v: i32, h: f32) {
        cb!(self.0, set_altitude_at_vertex, (), v, h)
    }
    fn begin_long_terrain_operation(&mut self) {
        cb!(self.0, begin_long_terrain_operation, ())
    }
    fn redisplay_terrain(&mut self, rect: VertRect, kind: u32) {
        let r = fc4_terrain_rect::from(rect);
        cb!(self.0, redisplay_terrain, (), &r, kind)
    }
    fn end_long_terrain_operation(&mut self) {
        cb!(self.0, end_long_terrain_operation, ())
    }
}

/// The world through the host table.
struct CWorld<'a>(&'a fc4_netgeom_host);

fn step_cells(run: &Run, step: u32) -> Vec<fc4_netgeom_cell> {
    match run.steps.get(step as usize) {
        Some(s) if s.first <= s.last && (s.last as usize) < run.cells.len() => {
            c_cells(&run.cells[s.first as usize..=s.last as usize])
        }
        _ => Vec::new(),
    }
}

impl GeometryHost for CWorld<'_> {
    fn is_connection_node(&self, x: i32, z: i32) -> bool {
        cb!(self.0, is_connection_node, false, x, z)
    }
    fn cell_occupants(&self, x: i32, z: i32) -> Vec<CellOccupant> {
        let Some(f) = self.0.cell_occupants else {
            return Vec::new();
        };
        let mut buf = vec![fc4_netgeom_occupant::default(); 16];
        // SAFETY: the host promised a valid callback; the buffer holds `len`.
        let mut n = unsafe { f(self.0.user, x, z, buf.as_mut_ptr(), buf.len() as u32) } as usize;
        if n > buf.len() {
            buf = vec![fc4_netgeom_occupant::default(); n.min(1 << 16)];
            // SAFETY: as above.
            n = unsafe { f(self.0.user, x, z, buf.as_mut_ptr(), buf.len() as u32) } as usize;
        }
        buf.truncate(n.min(buf.len()));
        buf.iter()
            .map(|o| match o.kind {
                FC4_NETGEOM_OCC_LANDFILL => CellOccupant::Landfill,
                FC4_NETGEOM_OCC_NETWORK => CellOccupant::Network {
                    immovable: o.immovable != 0,
                    connection_node: o.connection_node != 0,
                    type_id: o.type_id,
                },
                FC4_NETGEOM_OCC_BUILDING => CellOccupant::Building,
                _ => CellOccupant::Other,
            })
            .collect()
    }
    fn existing_piece(&self, x: i32, z: i32) -> Option<ExistingPiece> {
        let mut p = fc4_netgeom_piece::default();
        cb!(self.0, existing_piece, false, x, z, &mut p).then_some(ExistingPiece {
            flags: p.flags,
            immovable: p.immovable != 0,
            corners: p.corners,
            edges: p.edges,
            has_supports: p.has_supports != 0,
        })
    }
    fn extends_diagonal_end(&self, x: i32, z: i32, type_id: u32) -> bool {
        cb!(self.0, extends_diagonal_end, false, x, z, type_id)
    }
    fn update_occupant_highlights(&mut self, run: &Run, v: u32) -> u32 {
        if self.0.update_occupant_highlights.is_none() {
            return v;
        }
        let c = c_cells(&run.cells);
        let n = cb!(
            self.0,
            update_occupant_highlights,
            v,
            run.type_id,
            c.as_ptr(),
            c.len() as u32,
            v
        );
        n.min(v)
    }
    fn bridge_type_count(&self, type_id: u32) -> u32 {
        cb!(self.0, bridge_type_count, 0, type_id)
    }
    fn is_step_good_for_bridge(&self, run: &Run, step: u32) -> bool {
        if self.0.is_step_good_for_bridge.is_none() {
            return true;
        }
        let c = step_cells(run, step);
        cb!(
            self.0,
            is_step_good_for_bridge,
            true,
            run.type_id,
            step,
            c.as_ptr(),
            c.len() as u32
        )
    }
    fn bridge_step_edge_types_ok(&self, run: &Run, step: u32) -> bool {
        if self.0.bridge_step_edge_types_ok.is_none() {
            return true;
        }
        let c = step_cells(run, step);
        cb!(
            self.0,
            bridge_step_edge_types_ok,
            true,
            run.type_id,
            step,
            c.as_ptr(),
            c.len() as u32
        )
    }
    fn adjust_bridge_span(&mut self, run: &Run, start: u32, end: u32) -> Option<(u32, u32)> {
        let (mut s, mut e) = (start, end);
        cb!(self.0, adjust_bridge_span, true, run.type_id, &mut s, &mut e).then_some((s, e))
    }
    fn find_best_bridge_type(&mut self, run: &Run, start: u32, end: u32) -> Option<BridgeChoice> {
        let (mut bridge_type, mut min_span) = (0u32, 0u32);
        cb!(
            self.0,
            find_best_bridge_type,
            false,
            run.type_id,
            start,
            end,
            &mut bridge_type,
            &mut min_span
        )
        .then_some(BridgeChoice { bridge_type, min_span })
    }
}

// ---- handles ----------------------------------------------------------------------------------------

struct Handle {
    host: fc4_netgeom_host,
    /// The bound core terrain (NULL = the host table).
    terrain: Cell<*mut fc4_terrain>,
    geo: RefCell<NetworkGeometry>,
    refused: Cell<u32>,
}

impl Handle {
    fn refuse(&self) {
        self.refused.set(self.refused.get().wrapping_add(1));
    }
}

fn with_h<R>(g: *const fc4_netgeom, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_netgeom*.
    match unsafe { (g as *const Handle).as_ref() } {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

/// The geometry alone (no terrain, no world).
fn with_geo<R: Copy>(g: *const fc4_netgeom, fallback: R, f: impl FnOnce(&mut NetworkGeometry) -> R) -> R {
    with_h(g, fallback, |h| match h.geo.try_borrow_mut() {
        Ok(mut geo) => f(&mut geo),
        Err(_) => {
            h.refuse();
            fallback
        }
    })
}

/// The geometry with the terrain (bound or the host's) and the world.
fn with_world<R: Copy>(
    g: *const fc4_netgeom,
    fallback: R,
    f: impl FnOnce(&mut NetworkGeometry, &dyn GeometryTerrain, &mut CWorld) -> R,
) -> R {
    with_h(g, fallback, |h| {
        let Ok(mut geo) = h.geo.try_borrow_mut() else {
            h.refuse();
            return fallback;
        };
        let mut world = CWorld(&h.host);
        let t = h.terrain.get();
        if t.is_null() {
            return f(&mut geo, &CTerrain(&h.host), &mut world);
        }
        let mut out = fallback;
        if !read_terrain(t, |ter: &Terrain| out = f(&mut geo, ter, &mut world)) {
            h.refuse();
        }
        out
    })
}

/// The geometry with a writable terrain (the grading).
fn with_grading<R: Copy>(
    g: *const fc4_netgeom,
    fallback: R,
    f: impl FnOnce(&mut NetworkGeometry, &mut dyn GradingTerrain) -> R,
) -> R {
    with_h(g, fallback, |h| {
        let Ok(mut geo) = h.geo.try_borrow_mut() else {
            h.refuse();
            return fallback;
        };
        let t = h.terrain.get();
        if t.is_null() {
            return f(&mut geo, &mut CTerrain(&h.host));
        }
        let mut out = fallback;
        if !write_terrain(t, |ter: &mut Terrain| out = f(&mut geo, ter)) {
            h.refuse();
        }
        out
    })
}

unsafe fn put<T>(p: *mut T, v: T) {
    if let Some(r) = p.as_mut() {
        *r = v;
    }
}

/// Copies `src` to `out` when it fits `capacity`; returns `src.len()`.
unsafe fn copy_out<T: Copy>(src: &[T], out: *mut T, capacity: u32) -> u32 {
    if !out.is_null() && capacity as usize >= src.len() {
        std::ptr::copy_nonoverlapping(src.as_ptr(), out, src.len());
    }
    src.len() as u32
}

/// A caller's struct whose `struct_size` is at least ours.
unsafe fn sized<'a, T>(p: *const T) -> Option<&'a T> {
    let r = p.as_ref()?;
    let size = *(p as *const u32);
    ((size as usize) >= std::mem::size_of::<T>()).then_some(r)
}

/// The params of `type_id` from `params` (NULL = shipped).
unsafe fn info_of(type_id: u32, params: *const fc4_netgeom_params) -> Option<TypeInfo> {
    if type_id as usize >= TYPE_COUNT {
        return None;
    }
    if params.is_null() {
        return Some(TypeInfo::shipped(type_id));
    }
    sized(params).map(|p| TypeInfo::new(type_id, p.into()))
}

// ---- parameters and pure helpers --------------------------------------------------------------------

/// The shipped placement parameters of a type (§2.4); false for a type ≥ 13.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_params_shipped(type_id: u32, out: *mut fc4_netgeom_params) -> bool {
    guard(false, || {
        if type_id as usize >= TYPE_COUNT || out.is_null() {
            return false;
        }
        put(out, (&PlacementParams::shipped(type_id)).into());
        true
    })
}

/// The placement parameters of a type read from its exemplar's properties
/// (NULL = none: every property absent, the defaults of §2.4).
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_params_from_props(
    type_id: u32,
    props: *const fc4_props,
    out: *mut fc4_netgeom_params,
) -> bool {
    guard(false, || {
        if type_id as usize >= TYPE_COUNT || out.is_null() {
            return false;
        }
        put(out, (&PlacementParams::from_holder(type_id, props_ref(props))).into());
        true
    })
}

/// The per-type limits (§2.3) of `params` (NULL = shipped).
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_params_type_info(
    type_id: u32,
    params: *const fc4_netgeom_params,
    out: *mut fc4_netgeom_type_info,
) -> bool {
    guard(false, || match info_of(type_id, params) {
        Some(i) if !out.is_null() => {
            put(out, (&i).into());
            true
        }
        _ => false,
    })
}

/// `BreakIntoStraightAndDiagSegments` (§3.2).
#[allow(clippy::too_many_arguments)]
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_break_into_segments(
    start_x: i32,
    start_z: i32,
    end_x: i32,
    end_z: i32,
    snap: bool,
    straight_only: bool,
    width1: u32,
    width2: u32,
    out: *mut fc4_netgeom_segments,
) -> bool {
    guard(false, || {
        if out.is_null() {
            return false;
        }
        let s = break_into_segments((start_x, start_z), (end_x, end_z), snap, straight_only, width1, width2);
        put(
            out,
            fc4_netgeom_segments {
                struct_size: std::mem::size_of::<fc4_netgeom_segments>() as u32,
                end_x: s.end.0,
                end_z: s.end.1,
                dragged_count: s.dragged_count,
                straight_count: s.straight_count,
                axis0: s.axis.0,
                axis1: s.axis.1,
                pen_direction: s.pen_direction,
                end_direction: s.end_direction,
            },
        );
        true
    })
}

/// §9.6 step 3 for one vertex: network height `n`, terrain altitude `t`.
/// `FC4_NETGEOM_TV_FIX` (`*lo` = `*hi` = `*preferred` = the height),
/// `_RANGE`, `_UNSUPPORTED`, or `_INVALID` (bad type or params).
#[allow(clippy::too_many_arguments)]
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_terrain_vertex(
    n: f32,
    t: f32,
    has_supports: bool,
    fixed: bool,
    type_id: u32,
    params: *const fc4_netgeom_params,
    lo: *mut f32,
    hi: *mut f32,
    preferred: *mut f32,
) -> u32 {
    guard(FC4_NETGEOM_TV_INVALID, || {
        let Some(info) = info_of(type_id, params) else {
            return FC4_NETGEOM_TV_INVALID;
        };
        match terrain_vertex_constraint(n, t, has_supports, fixed, &info) {
            TerrainVertex::Fix(h) => {
                put(lo, h);
                put(hi, h);
                put(preferred, h);
                FC4_NETGEOM_TV_FIX
            }
            TerrainVertex::Range {
                lo: a,
                hi: b,
                preferred: p,
            } => {
                put(lo, a);
                put(hi, b);
                put(preferred, p);
                FC4_NETGEOM_TV_RANGE
            }
            TerrainVertex::Unsupported => FC4_NETGEOM_TV_UNSUPPORTED,
        }
    })
}

/// The support level of a cell (§10.2): 0, 1 or 2. NULL arrays read as 0.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_support_level(corners: *const f32, terrain: *const f32, threshold: f32) -> u32 {
    guard(0, || {
        if corners.is_null() || terrain.is_null() {
            return 0;
        }
        let c: [f32; 4] = std::ptr::read_unaligned(corners as *const [f32; 4]);
        let t: [f32; 4] = std::ptr::read_unaligned(terrain as *const [f32; 4]);
        extras::support_level(c, t, threshold)
    })
}

/// The occupant flags of a support level (§10.2).
#[no_mangle]
pub extern "C" fn fc4_netgeom_support_flags(level: u32) -> u32 {
    extras::support_flags(level)
}

/// The occupant flags of a cell's piece (§10.1): the draft flag for a draft
/// occupant, cleared for a real one.
#[no_mangle]
pub extern "C" fn fc4_netgeom_occupant_flags(cell_flags: u32, real: bool, tunnel_occupant: bool) -> u32 {
    extras::occupant_flags(cell_flags, real, tunnel_occupant)
}

/// The power line's `GetCostOfSolution` (§11).
#[no_mangle]
pub extern "C" fn fc4_netgeom_power_line_cost(poles: u32, wire_cells: u32, per_pole: i32, per_wire: f32) -> i32 {
    extras::power_line_cost(poles, wire_cells, per_pole, per_wire)
}

/// The cell key of `DestroyOccupantsAndResolve`, `(x << 16) + z` (§13).
#[no_mangle]
pub extern "C" fn fc4_netgeom_destroy_key(x: i32, z: i32) -> i32 {
    extras::destroy_key(x, z)
}

// ---- the geometry -----------------------------------------------------------------------------------

/// A geometry over `host`. `params` (NULL = the shipped table) holds
/// `count` types in type order; missing types get all-zero parameters (the
/// defaults of §2.4). NULL on a bad host table or params size.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_create(
    host: *const fc4_netgeom_host,
    params: *const fc4_netgeom_params,
    count: u32,
) -> *mut fc4_netgeom {
    guard(std::ptr::null_mut(), || {
        let Some(h) = sized(host) else {
            return std::ptr::null_mut();
        };
        let types = if params.is_null() {
            TypeTable::shipped()
        } else {
            let mut v = Vec::new();
            for k in 0..(count as usize).min(TYPE_COUNT) {
                match sized(params.add(k)) {
                    Some(p) => v.push(PlacementParams::from(p)),
                    None => return std::ptr::null_mut(),
                }
            }
            TypeTable::new(v)
        };
        Box::into_raw(Box::new(Handle {
            host: *h,
            terrain: Cell::new(std::ptr::null_mut()),
            geo: RefCell::new(NetworkGeometry::new(types)),
            refused: Cell::new(0),
        })) as *mut fc4_netgeom
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_destroy(g: *mut fc4_netgeom) {
    if !g.is_null() {
        guard((), || drop(Box::from_raw(g as *mut Handle)));
    }
}

/// Binds a core terrain (NULL = back to the host table's terrain
/// callbacks). The terrain must outlive the binding.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_set_terrain(g: *mut fc4_netgeom, terrain: *mut fc4_terrain) -> bool {
    with_h(g, false, |h| {
        h.terrain.set(terrain);
        true
    })
}

/// The type info of a type in the geometry's table.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_get_type_info(
    g: *const fc4_netgeom,
    type_id: u32,
    out: *mut fc4_netgeom_type_info,
) -> bool {
    with_geo(g, false, |geo| match geo.types.get(type_id) {
        Some(i) if !out.is_null() => {
            put(out, i.into());
            true
        }
        _ => false,
    })
}

/// Tool `+0x354` per type: the land-bridge pass is on (NG4; the pass itself
/// is not exposed yet).
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_set_land_bridges(g: *mut fc4_netgeom, type_id: u32, on: bool) -> bool {
    with_geo(g, false, |geo| match geo.land_bridges.get_mut(type_id as usize) {
        Some(b) => {
            *b = on;
            true
        }
        None => false,
    })
}

/// `Reset`: clears the run and the solve state.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_reset(g: *mut fc4_netgeom) -> bool {
    with_geo(g, false, |geo| {
        geo.reset();
        true
    })
}

/// `DrawNetworkLine` (slot 18, §3.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_draw_line(
    g: *mut fc4_netgeom,
    type_id: u32,
    start_x: i32,
    start_z: i32,
    end_x: i32,
    end_z: i32,
    snap: bool,
) -> bool {
    with_world(g, false, |geo, t, w| {
        geo.draw_network_line(type_id, (start_x, start_z), (end_x, end_z), snap, t, w)
    })
}

/// The validation passes (§4); returns the valid step count. The failure
/// code is in `fc4_netgeom_state.failure`.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_validate(g: *mut fc4_netgeom) -> u32 {
    with_world(g, 0, |geo, t, w| geo.validate(t, w).valid)
}

/// The cell infos of the valid steps (slot 71, §8.1) and the solved cells.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_create_cell_infos(g: *mut fc4_netgeom) -> bool {
    with_world(g, false, |geo, t, w| {
        geo.create_cell_infos(t, w);
        true
    })
}

/// Adds or replaces a cell info (placeholders, existing occupants).
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_insert_cell_info(g: *mut fc4_netgeom, info: *const fc4_netgeom_cell_info) -> bool {
    match sized(info) {
        Some(i) => {
            let ci = CellInfo::from(i);
            with_geo(g, false, move |geo| {
                geo.insert_cell_info(ci);
                true
            })
        }
        None => false,
    }
}

/// A cell info by cell.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_get_cell_info(
    g: *const fc4_netgeom,
    x: i32,
    z: i32,
    out: *mut fc4_netgeom_cell_info,
) -> bool {
    with_geo(g, false, |geo| {
        match geo.cells.get(&fc4_core::network_geometry::cell_key(x, z)) {
            Some(c) if !out.is_null() => {
                put(out, c.into());
                true
            }
            _ => false,
        }
    })
}

/// `SmoothenNetwork(level)` (§9.4) alone.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_smoothen(g: *mut fc4_netgeom, level: i32) -> bool {
    with_world(g, false, |geo, t, w| geo.smoothen(level, t, w))
}

/// Slot 56 `CanTilesBeSupportedOnTerrain` (§9.6) alone.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_can_tiles_be_supported(g: *mut fc4_netgeom) -> bool {
    with_world(g, false, |geo, t, w| geo.can_tiles_be_supported(t, w))
}

/// The smoothing loop (§9.4): 0 and `*level` on success, else the failure
/// code (`0xF0000006`, `0xF0000008`).
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_smooth(g: *mut fc4_netgeom, level: *mut i32) -> u32 {
    with_world(g, u32::MAX, |geo, t, w| match geo.smooth(t, w) {
        Ok(l) => {
            put(level, l);
            0
        }
        Err(c) => c,
    })
}

/// The geometry of `PlaceNetwork` (network-tools.md §5.1 steps 4 and 5
/// without the piece solver): the passes, the cell infos and the smoothing
/// loop. Returns the valid count (0 = refused; see the failure code).
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_place(g: *mut fc4_netgeom) -> u32 {
    with_world(g, 0, |geo, t, w| geo.place(t, w))
}

/// `GetCostOfSolution` (network-tools.md §8.1) of the solved cells.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_cost(g: *const fc4_netgeom, demolition: i32) -> i32 {
    with_world(g, 0, |geo, t, _| cost_of_solution(&geo.cost_inputs(t, demolition)))
}

/// Slot 57 `UpdateTerrainHeights` (§9.7): writes the terrain satisfier's
/// solution (changes above 0.001) and redisplays the rectangle (kind 4)
/// inside Begin/EndLongTerrainOperation. Returns the changed vertex count;
/// `*rect` (may be NULL) is written when it is not 0.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_update_terrain_heights(g: *mut fc4_netgeom, rect: *mut fc4_terrain_rect) -> u32 {
    let mut r = None;
    let n = with_grading(g, 0u32, |geo, t| {
        let gr = geo.update_terrain_heights(t);
        r = gr.rect;
        gr.changed.len() as u32
    });
    if let Some(v) = r {
        put(rect, v.into());
    }
    n
}

/// Diagnostics.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_get_state(g: *const fc4_netgeom, out: *mut fc4_netgeom_state) -> bool {
    if out.is_null() {
        return false;
    }
    with_h(g, false, |h| {
        let Ok(geo) = h.geo.try_borrow() else {
            h.refuse();
            return false;
        };
        let r = &geo.run;
        put(
            out,
            fc4_netgeom_state {
                struct_size: std::mem::size_of::<fc4_netgeom_state>() as u32,
                type_id: r.type_id,
                start_x: r.start.0,
                start_z: r.start.1,
                end_x: r.end.0,
                end_z: r.end.1,
                dragged_count: r.dragged_count,
                straight_count: r.straight_count,
                axis0: r.axis.0,
                axis1: r.axis.1,
                pen_direction: r.pen_direction,
                end_direction: r.end_direction,
                valid: geo.valid,
                failure: geo.failure,
                cell_count: r.cells.len() as u32,
                step_count: r.steps.len() as u32,
                extra_cell_count: r.extra_cells.len() as u32,
                tunnel_count: r.tunnels.len() as u32,
                bridge_count: r.bridges.len() as u32,
                cell_info_count: geo.cells.len() as u32,
                solved_count: geo.solved.len() as u32,
                smoothing_cell_count: geo.smoothing.cells.len() as u32,
                boundary_count: geo.smoothing.boundary.len() as u32,
                level: geo.level.unwrap_or(-1),
                tunnel_start_height: r.tunnel_start_height.unwrap_or(f32::NAN),
                reentrant_refused: h.refused.get(),
                terrain_bound: u8::from(!h.terrain.get().is_null()),
                reserved: [0; 3],
            },
        );
        true
    })
}

/// The dragged cells: copies when `capacity` suffices; returns the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_get_cells(
    g: *const fc4_netgeom,
    out: *mut fc4_netgeom_cell,
    capacity: u32,
) -> u32 {
    with_geo(g, 0, |geo| copy_out(&c_cells(&geo.run.cells), out, capacity))
}

/// The dragged steps: copies when `capacity` suffices; returns the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_get_steps(
    g: *const fc4_netgeom,
    out: *mut fc4_netgeom_step,
    capacity: u32,
) -> u32 {
    with_geo(g, 0, |geo| {
        let v: Vec<fc4_netgeom_step> = geo
            .run
            .steps
            .iter()
            .map(|s| fc4_netgeom_step {
                first: s.first,
                last: s.last,
                kind: s.kind as u32,
            })
            .collect();
        copy_out(&v, out, capacity)
    })
}

/// The recorded tunnels: copies when `capacity` suffices; returns the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_get_tunnels(
    g: *const fc4_netgeom,
    out: *mut fc4_netgeom_span,
    capacity: u32,
) -> u32 {
    with_geo(g, 0, |geo| {
        let v: Vec<fc4_netgeom_span> = geo
            .run
            .tunnels
            .iter()
            .map(|t| fc4_netgeom_span {
                start: t.start,
                end: t.end,
                bridge_type: 0,
            })
            .collect();
        copy_out(&v, out, capacity)
    })
}

/// The recorded bridges: copies when `capacity` suffices; returns the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_get_bridges(
    g: *const fc4_netgeom,
    out: *mut fc4_netgeom_span,
    capacity: u32,
) -> u32 {
    with_geo(g, 0, |geo| {
        let v: Vec<fc4_netgeom_span> = geo
            .run
            .bridges
            .iter()
            .map(|b| fc4_netgeom_span {
                start: b.start,
                end: b.end,
                bridge_type: b.bridge_type,
            })
            .collect();
        copy_out(&v, out, capacity)
    })
}

/// The solved cells: copies when `capacity` suffices; returns the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_get_solved(
    g: *const fc4_netgeom,
    out: *mut fc4_netgeom_cell,
    capacity: u32,
) -> u32 {
    with_geo(g, 0, |geo| copy_out(&c_cells(&geo.solved), out, capacity))
}

fn sat_of(geo: &NetworkGeometry, which: u32) -> Option<&VertexHtConstraintSatisfier> {
    match which {
        FC4_NETGEOM_SAT_NETWORK => Some(&geo.network),
        FC4_NETGEOM_SAT_TERRAIN => Some(&geo.terrain),
        _ => None,
    }
}

/// A satisfier's solved height of `vertex` (`FC4_NETGEOM_SAT_*`).
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_get_smoothing_cells(
    g: *const fc4_netgeom,
    out: *mut fc4_netgeom_cell,
    capacity: u32,
) -> u32 {
    with_geo(g, 0, |geo| {
        let cells: Vec<_> = geo
            .smoothing
            .cells
            .iter()
            .filter_map(|key| geo.cells.get(key))
            .map(|c| fc4_netgeom_cell { x: c.x, z: c.z })
            .collect();
        copy_out(&cells, out, capacity)
    })
}

/// A satisfier's solved height of `vertex` (`FC4_NETGEOM_SAT_*`).
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_sat_height(g: *const fc4_netgeom, which: u32, vertex: i32, out: *mut f32) -> bool {
    with_geo(g, false, |geo| {
        match sat_of(geo, which).and_then(|s| s.height(vertex)) {
            Some(v) => {
                put(out, v);
                true
            }
            None => false,
        }
    })
}

/// A satisfier's allowed range of `vertex`.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_sat_range(
    g: *const fc4_netgeom,
    which: u32,
    vertex: i32,
    lo: *mut f32,
    hi: *mut f32,
) -> bool {
    with_geo(g, false, |geo| match sat_of(geo, which).and_then(|s| s.range(vertex)) {
        Some((a, b)) => {
            put(lo, a);
            put(hi, b);
            true
        }
        None => false,
    })
}

/// A satisfier's solution in pick order: copies when `capacity` suffices;
/// returns the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_netgeom_sat_solution(
    g: *const fc4_netgeom,
    which: u32,
    out: *mut fc4_netgeom_vertex_height,
    capacity: u32,
) -> u32 {
    with_geo(g, 0, |geo| match sat_of(geo, which) {
        Some(s) => copy_out(&solution_of(s), out, capacity),
        None => 0,
    })
}

fn solution_of(s: &VertexHtConstraintSatisfier) -> Vec<fc4_netgeom_vertex_height> {
    s.solution()
        .iter()
        .map(|&(vertex, height)| fc4_netgeom_vertex_height { vertex, height })
        .collect()
}

// ---- the standalone satisfier -----------------------------------------------------------------------

fn with_s<R>(s: *mut fc4_vhcs, fallback: R, f: impl FnOnce(&mut VertexHtConstraintSatisfier) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_vhcs*.
    match unsafe { (s as *mut VertexHtConstraintSatisfier).as_mut() } {
        Some(v) => guard(fallback, || f(v)),
        None => fallback,
    }
}

/// A new `cSC4VertexHtConstraintSatisfier` (§9.1), not initialised.
#[no_mangle]
pub extern "C" fn fc4_vhcs_create() -> *mut fc4_vhcs {
    guard(std::ptr::null_mut(), || {
        Box::into_raw(Box::new(VertexHtConstraintSatisfier::new())) as *mut fc4_vhcs
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_vhcs_destroy(s: *mut fc4_vhcs) {
    if !s.is_null() {
        guard((), || drop(Box::from_raw(s as *mut VertexHtConstraintSatisfier)));
    }
}

/// `Init(lo, hi)`: clears everything; `hi` is every new vertex's range top.
#[no_mangle]
pub unsafe extern "C" fn fc4_vhcs_init(s: *mut fc4_vhcs, lo: f32, hi: f32) -> bool {
    with_s(s, false, |v| {
        v.init(lo, hi);
        true
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_vhcs_shutdown(s: *mut fc4_vhcs) -> bool {
    with_s(s, false, |v| {
        v.shutdown();
        true
    })
}

/// `AddVertexToSolve(v, initial)`: false when already present.
#[no_mangle]
pub unsafe extern "C" fn fc4_vhcs_add_vertex(s: *mut fc4_vhcs, vertex: i32, initial: f32) -> bool {
    with_s(s, false, |v| v.add_vertex(vertex, initial))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_vhcs_insert_range(s: *mut fc4_vhcs, vertex: i32, lo: f32, hi: f32) -> bool {
    with_s(s, false, |v| {
        v.insert_range(vertex, lo, hi);
        true
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_vhcs_insert_fixed(s: *mut fc4_vhcs, vertex: i32, height: f32) -> bool {
    with_s(s, false, |v| {
        v.insert_fixed(vertex, height);
        true
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_vhcs_insert_equality(s: *mut fc4_vhcs, a: i32, b: i32) -> bool {
    with_s(s, false, |v| {
        v.insert_equality(a, b);
        true
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_vhcs_insert_slope(s: *mut fc4_vhcs, a: i32, b: i32, limit: f32) -> bool {
    with_s(s, false, |v| {
        v.insert_slope(a, b, limit);
        true
    })
}

/// `|h(a) − 2·h(b) + h(c)| ≤ limit`, `b` the middle vertex.
#[no_mangle]
pub unsafe extern "C" fn fc4_vhcs_insert_smoothness(s: *mut fc4_vhcs, a: i32, b: i32, c: i32, limit: f32) -> bool {
    with_s(s, false, |v| {
        v.insert_smoothness(a, b, c, limit);
        true
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_vhcs_set_preferred_value(s: *mut fc4_vhcs, vertex: i32, height: f32) -> bool {
    with_s(s, false, |v| {
        v.set_preferred_value(vertex, height);
        true
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_vhcs_set_preferred_range(s: *mut fc4_vhcs, vertex: i32, lo: f32, hi: f32) -> bool {
    with_s(s, false, |v| {
        v.set_preferred_range(vertex, lo, hi);
        true
    })
}

/// `Solve()`: false when a range became empty.
#[no_mangle]
pub unsafe extern "C" fn fc4_vhcs_solve(s: *mut fc4_vhcs) -> bool {
    with_s(s, false, |v| v.solve())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_vhcs_height(s: *mut fc4_vhcs, vertex: i32, out: *mut f32) -> bool {
    with_s(s, false, |v| match v.height(vertex) {
        Some(h) => {
            put(out, h);
            true
        }
        None => false,
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_vhcs_range(s: *mut fc4_vhcs, vertex: i32, lo: *mut f32, hi: *mut f32) -> bool {
    with_s(s, false, |v| match v.range(vertex) {
        Some((a, b)) => {
            put(lo, a);
            put(hi, b);
            true
        }
        None => false,
    })
}

/// The solution in pick order: copies when `capacity` suffices; returns
/// the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_vhcs_solution(
    s: *mut fc4_vhcs,
    out: *mut fc4_netgeom_vertex_height,
    capacity: u32,
) -> u32 {
    with_s(s, 0, |v| copy_out(&solution_of(v), out, capacity))
}

// Keep the step-kind numbering in one place for fc4.h.
const _: () = {
    assert!(StepKind::Regular as u32 == 0);
    assert!(StepKind::Bridge as u32 == 1);
    assert!(StepKind::Portal as u32 == 2);
    assert!(StepKind::TunnelInterior as u32 == 3);
};
