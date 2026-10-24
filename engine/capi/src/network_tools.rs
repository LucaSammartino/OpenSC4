// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The network tools over the C ABI (fc4.h "network tools",
//! docs/specs/network-tools.md): the tool state, `PlaceNetworkOccupants`,
//! `PlaceNetworkOccupantById`, the `PlaceNetwork` pipeline hand-off, the
//! commit, `DestroyOccupantsAndResolve` and the underground refit. No plugin
//! glue yet.
//!
//! The tool's own state lives in the handle; the world (occupants, poles,
//! the terrain, the geometry core, the crew) goes through the host table,
//! whose missing callbacks are the empty world. The solve and the geometry
//! run through `fc4_netgeom` (a bound handle on `host.geometry`) when the
//! host supplies one, or through the host's own callbacks otherwise.
//!
//! Threads: one thread at a time. A callback that calls back into the same
//! tool is refused (fallback value, counted in `fc4_nettool_state.reentrant_refused`).

use std::cell::{Cell, RefCell};
use std::ffi::c_void;

use fc4_core::network_tools::host::{Neighbour, PassResult};
use fc4_core::network_tools::{
    destroy_occupants_and_resolve, place_network, place_network_occupant_by_id, place_network_occupants,
    update_underground_occupant_terrain_heights, Listed, ToolHost, ToolRun, ToolState, UndergroundOutcome,
    UndergroundPiece,
};

use crate::guard;

type U = *mut c_void;

/// What the network tool needs from the game beyond its own state.
#[repr(C)]
pub struct fc4_nettool_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    /// §3.3/§5: the drag and the pipeline, through the geometry core (the
    /// host binds an `fc4_netgeom` and replays the calls on it).
    pub draw_network_line: Option<unsafe extern "C" fn(user: U, run: *mut fc4_nettool_run, snap: bool) -> bool>,
    pub do_network_specific_placement: Option<
        unsafe extern "C" fn(user: U, run: *mut fc4_nettool_run, arg3: u32, neighbour_mode: u32, real: bool) -> bool,
    >,
    pub validate_and_solve: Option<
        unsafe extern "C" fn(user: U, run: *mut fc4_nettool_run, real: bool, out: *mut fc4_nettool_pass) -> bool,
    >,
    pub attempt_neighbor_connections: Option<unsafe extern "C" fn(user: U, run: *mut fc4_nettool_run) -> bool>,
    pub place_neighbor_connections: Option<unsafe extern "C" fn(user: U, run: *mut fc4_nettool_run)>,
    /// §3.3: `GetNetworkOccupant(x, z)` with the OR filter on `mask`: the
    /// occupant's network type, or 0xFFFFFFFF = none.
    pub get_network_occupant: Option<unsafe extern "C" fn(user: U, x: i32, z: i32, mask: u32) -> u32>,
    /// §3.3 step 6.2: the callback tool's `PlaceNetworkOccupants`.
    pub callback_place:
        Option<unsafe extern "C" fn(user: U, callback: u32, ax: i32, az: i32, bx: i32, bz: i32, real: bool)>,
    // ---- the commit (§6) ----
    pub commit_pre: Option<unsafe extern "C" fn(user: U, run: *const fc4_nettool_run, real: bool)>,
    pub insert_occupants_for_solved_cells:
        Option<unsafe extern "C" fn(user: U, run: *const fc4_nettool_run, preview_only: bool)>,
    pub insert_bridge_pieces: Option<unsafe extern "C" fn(user: U, run: *const fc4_nettool_run)>,
    pub insert_tunnel_pieces: Option<unsafe extern "C" fn(user: U, run: *const fc4_nettool_run)>,
    pub insert_highway_intersection_pieces: Option<unsafe extern "C" fn(user: U, run: *const fc4_nettool_run)>,
    pub crew_parameters_both_one: Option<unsafe extern "C" fn(user: U, type_id: u32) -> bool>,
    pub crew_add_network_segment: Option<unsafe extern "C" fn(user: U, crew: u64, run: *const fc4_nettool_run)>,
    pub mark_occupants_usable: Option<unsafe extern "C" fn(user: U, run: *const fc4_nettool_run, release: bool)>,
    pub commit_draft: Option<unsafe extern "C" fn(user: U, run: *const fc4_nettool_run)>,
    // ---- §4 / §6.4 ----
    pub insert_network_occupant_by_id: Option<
        unsafe extern "C" fn(
            user: U,
            run: *const fc4_nettool_run,
            x: i32,
            z: i32,
            piece: u32,
            usable: bool,
            type_id: u32,
        ),
    >,
    /// Slot 9 `OccupantExists(x, z, flags, &mask)`: 1 with mask written,
    /// 0 with mask untouched (zones.md §8.8).
    pub occupant_exists: Option<unsafe extern "C" fn(user: U, x: i32, z: i32, flags: u32, mask: *mut u8) -> bool>,
    // ---- §10 ----
    pub occupied_cell: Option<unsafe extern "C" fn(user: U, occ: u64, out: *mut fc4_nettool_cell) -> bool>,
    /// §10 step 3: the occupant's neighbours by network type. Writes up to
    /// `capacity` neighbours, returns the total.
    pub neighbours_by_network_type:
        Option<unsafe extern "C" fn(user: U, occ: u64, out: *mut fc4_nettool_neighbour, capacity: u32) -> u32>,
    pub is_intersection_piece: Option<unsafe extern "C" fn(user: U, occ: u64) -> bool>,
    pub intersection_structure: Option<unsafe extern "C" fn(user: U, occ: u64, out: *mut u64, capacity: u32) -> u32>,
    pub is_tunnel_portal: Option<unsafe extern "C" fn(user: U, occ: u64) -> bool>,
    pub paired_portal: Option<unsafe extern "C" fn(user: U, occ: u64) -> u64>,
    pub send_pre_demolish_sync: Option<unsafe extern "C" fn(user: U, occ: u64)>,
    pub remove_occupant: Option<unsafe extern "C" fn(user: U, occ: u64, shutdown: bool)>,
    pub remove_or_replace_occupant: Option<unsafe extern "C" fn(user: U, occ: u64) -> bool>,
    pub occupant_shutdown: Option<unsafe extern "C" fn(user: U, occ: u64)>,
    pub release: Option<unsafe extern "C" fn(user: U, occ: u64)>,
    pub solve_network_resolve: Option<unsafe extern "C" fn(user: U, run: *mut fc4_nettool_run)>,
    // ---- §11 ----
    pub corner_altitudes: Option<unsafe extern "C" fn(user: U, x: i32, z: i32, out: *mut f32)>,
    pub smoothen_underground: Option<unsafe extern "C" fn(user: U, run: *mut fc4_nettool_run, level: i32) -> bool>,
    pub find_underground_solved:
        Option<unsafe extern "C" fn(user: U, x: i32, z: i32, out: *mut fc4_nettool_solved) -> bool>,
    pub destroy_network_occupants: Option<unsafe extern "C" fn(user: U, occ: *const u64, count: usize)>,
    pub release_occupant_list: Option<unsafe extern "C" fn(user: U, occ: *const u64, count: usize)>,
    pub underground_rebuild: Option<
        unsafe extern "C" fn(user: U, occ: u64, outcome: u32, vertex_heights: *const f32, stored_terrain: *const f32),
    >,
}

/// One run of the drag/solve/commit pipeline (fc4.h `fc4_nettool_run`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct fc4_nettool_run {
    pub struct_size: u32,
    pub type_id: u32,
    pub start_x: i32,
    pub start_z: i32,
    pub end_x: i32,
    pub end_z: i32,
    pub dragged_count: u32,
    pub straight_count: u32,
    pub valid_count: u32,
    /// The current `+0x208` (the insert `data4`).
    pub data4: u32,
    pub cell_count: u32,
    /// The dragged cells, up to `FC4_NETTOOL_MAX_CELLS`.
    pub cells: [fc4_nettool_cell; FC4_NETTOOL_MAX_CELLS],
    pub resolve_count: u32,
    pub resolve_cells: [fc4_nettool_cell; FC4_NETTOOL_MAX_CELLS],
}

impl Default for fc4_nettool_run {
    fn default() -> Self {
        fc4_nettool_run {
            struct_size: 0,
            type_id: 0,
            start_x: 0,
            start_z: 0,
            end_x: 0,
            end_z: 0,
            dragged_count: 0,
            straight_count: 0,
            valid_count: 0,
            data4: 0,
            cell_count: 0,
            cells: [fc4_nettool_cell::default(); FC4_NETTOOL_MAX_CELLS],
            resolve_count: 0,
            resolve_cells: [fc4_nettool_cell::default(); FC4_NETTOOL_MAX_CELLS],
        }
    }
}

/// A cell.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_nettool_cell {
    pub x: i32,
    pub z: i32,
}

/// A neighbour of the destroy closure (§10 step 3).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_nettool_neighbour {
    pub occ: u64,
    pub x: i32,
    pub z: i32,
    pub immovable: u8,
    pub intersection: u8,
    pub reserved: [u8; 6],
}

/// The pass/solve result (§5.1 steps 4-5).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_nettool_pass {
    pub valid_count: u32,
    pub failure: u32,
}

/// `FindUndergroundOccupant`'s solved record (§11.1 step 4).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_nettool_solved {
    pub occ: u64,
    pub corners: [f32; 4],
}

/// The largest cell list one run carries (a city is at most 4096 cells wide;
/// a straight drag never exceeds that, and §18's runs stay tiny).
pub const FC4_NETTOOL_MAX_CELLS: usize = 4096;

/// `UndergroundOutcome` as a C value.
pub const FC4_NETTOOL_U_UPDATED: u32 = 0;
pub const FC4_NETTOOL_U_KEPT: u32 = 1;
pub const FC4_NETTOOL_U_DESTROY: u32 = 2;
#[repr(C)]
pub struct fc4_nettool {
    _private: [u8; 0],
}

/// Diagnostics (`fc4_nettool_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_nettool_state {
    pub struct_size: u32,
    pub type_id: u32,
    pub real: u8,
    pub stop_on_failure: u8,
    pub destroyed_flag: u8,
    pub preview_update: u8,
    pub reserved: [u8; 4],
    pub data4: u32,
    pub failure: u32,
    pub demolition_cost: i32,
    pub layer_mask: u32,
    pub depth: f32,
    pub cover_margin_a: f32,
    pub cover_margin_b: f32,
    pub city_x: u32,
    pub city_z: u32,
    pub cache_count: u32,
    pub reentrant_refused: u32,
}

struct Handle {
    host: Box<fc4_nettool_host>,
    st: RefCell<ToolState>,
    refused: Cell<u32>,
}

macro_rules! cb {
    ($h:expr, $f:ident $(, $arg:expr)* $(,)?) => {
        match $h.$f {
            Some(f) => unsafe { f($h.user $(, $arg)*) },
            None => Default::default(),
        }
    };
}

fn run_to_c(run: &ToolRun) -> fc4_nettool_run {
    let mut r = fc4_nettool_run {
        struct_size: std::mem::size_of::<fc4_nettool_run>() as u32,
        type_id: run.type_id,
        start_x: run.start.0,
        start_z: run.start.1,
        end_x: run.end.0,
        end_z: run.end.1,
        dragged_count: run.dragged_count,
        straight_count: run.straight_count,
        valid_count: run.valid_count,
        data4: run.data4,
        cell_count: run.cells.len().min(FC4_NETTOOL_MAX_CELLS) as u32,
        cells: [fc4_nettool_cell::default(); FC4_NETTOOL_MAX_CELLS],
        resolve_count: run.resolve_cells.len().min(FC4_NETTOOL_MAX_CELLS) as u32,
        resolve_cells: [fc4_nettool_cell::default(); FC4_NETTOOL_MAX_CELLS],
    };
    for (i, &(x, z)) in run.cells.iter().take(FC4_NETTOOL_MAX_CELLS).enumerate() {
        r.cells[i] = fc4_nettool_cell { x, z };
    }
    for (i, &(x, z)) in run.resolve_cells.iter().take(FC4_NETTOOL_MAX_CELLS).enumerate() {
        r.resolve_cells[i] = fc4_nettool_cell { x, z };
    }
    r
}

fn run_from_c(r: &fc4_nettool_run) -> ToolRun {
    let mut run = ToolRun::new(r.type_id, (r.start_x, r.start_z), (r.end_x, r.end_z));
    run.dragged_count = r.dragged_count;
    run.straight_count = r.straight_count;
    run.valid_count = r.valid_count;
    run.data4 = r.data4;
    run.cells = r.cells[..(r.cell_count as usize).min(FC4_NETTOOL_MAX_CELLS)]
        .iter()
        .map(|c| (c.x, c.z))
        .collect();
    run.resolve_cells = r.resolve_cells[..(r.resolve_count as usize).min(FC4_NETTOOL_MAX_CELLS)]
        .iter()
        .map(|c| (c.x, c.z))
        .collect();
    run
}

/// The host table as a [`ToolHost`].
struct CHost<'a>(&'a fc4_nettool_host);

impl ToolHost for CHost<'_> {
    fn draw_network_line(&mut self, run: &mut ToolRun, snap: bool) -> bool {
        let mut r = run_to_c(run);
        let ok = cb!(self.0, draw_network_line, &mut r, snap);
        *run = run_from_c(&r);
        ok
    }
    fn do_network_specific_placement(&mut self, run: &mut ToolRun, arg3: u32, neighbour_mode: u32, real: bool) -> bool {
        let mut r = run_to_c(run);
        let ok = cb!(
            self.0,
            do_network_specific_placement,
            &mut r,
            arg3,
            neighbour_mode,
            real
        );
        *run = run_from_c(&r);
        ok
    }
    fn validate_and_solve(&mut self, run: &mut ToolRun, stop_on_failure: bool, real: bool) -> PassResult {
        let mut r = run_to_c(run);
        let mut p = fc4_nettool_pass::default();
        let ok = cb!(self.0, validate_and_solve, &mut r, real, &mut p);
        *run = run_from_c(&r);
        let _ = stop_on_failure;
        match ok {
            true => PassResult {
                valid_count: p.valid_count,
                failure: (p.failure != 0).then_some(p.failure),
            },
            false => PassResult::default(),
        }
    }
    fn attempt_neighbor_connections(&mut self, run: &mut ToolRun) -> bool {
        let mut r = run_to_c(run);
        let ok = cb!(self.0, attempt_neighbor_connections, &mut r);
        *run = run_from_c(&r);
        ok
    }
    fn place_neighbor_connections(&mut self, run: &mut ToolRun) {
        let mut r = run_to_c(run);
        cb!(self.0, place_neighbor_connections, &mut r);
        *run = run_from_c(&r);
    }
    fn get_network_occupant(&self, x: i32, z: i32, mask: u32) -> Option<u32> {
        let t = cb!(self.0, get_network_occupant, x, z, mask);
        (t != 0xFFFF_FFFF).then_some(t)
    }
    fn callback_place(&mut self, callback: u32, ax: i32, az: i32, bx: i32, bz: i32, real: bool) {
        cb!(self.0, callback_place, callback, ax, az, bx, bz, real);
    }
    fn commit_pre(&mut self, run: &mut ToolRun, real: bool) {
        let r = run_to_c(run);
        cb!(self.0, commit_pre, &r, real);
    }
    fn insert_occupants_for_solved_cells(&mut self, run: &mut ToolRun, preview_only: bool) {
        let r = run_to_c(run);
        cb!(self.0, insert_occupants_for_solved_cells, &r, preview_only);
    }
    fn insert_bridge_pieces(&mut self, run: &mut ToolRun) {
        let r = run_to_c(run);
        cb!(self.0, insert_bridge_pieces, &r);
    }
    fn insert_tunnel_pieces(&mut self, run: &mut ToolRun) {
        let r = run_to_c(run);
        cb!(self.0, insert_tunnel_pieces, &r);
    }
    fn insert_highway_intersection_pieces(&mut self, run: &mut ToolRun) {
        let r = run_to_c(run);
        cb!(self.0, insert_highway_intersection_pieces, &r);
    }
    fn crew_parameters_both_one(&self, type_id: u32) -> bool {
        cb!(self.0, crew_parameters_both_one, type_id)
    }
    fn crew_add_network_segment(&mut self, crew: u64, run: &mut ToolRun) {
        let r = run_to_c(run);
        cb!(self.0, crew_add_network_segment, crew, &r);
    }
    fn mark_occupants_usable(&mut self, run: &mut ToolRun, release: bool) {
        let r = run_to_c(run);
        cb!(self.0, mark_occupants_usable, &r, release);
    }
    fn commit_draft(&mut self, run: &mut ToolRun) {
        let r = run_to_c(run);
        cb!(self.0, commit_draft, &r);
    }
    fn insert_network_occupant_by_id(
        &mut self,
        run: &mut ToolRun,
        x: i32,
        z: i32,
        piece: u32,
        usable: bool,
        type_id: u32,
    ) {
        let r = run_to_c(run);
        cb!(self.0, insert_network_occupant_by_id, &r, x, z, piece, usable, type_id);
    }
    fn occupied_cell(&self, occ: u64) -> Option<(i32, i32)> {
        let mut c = fc4_nettool_cell::default();
        cb!(self.0, occupied_cell, occ, &mut c).then_some((c.x, c.z))
    }
    fn neighbours_by_network_type(&self, occ: u64) -> Vec<Neighbour> {
        let Some(f) = self.0.neighbours_by_network_type else {
            return Vec::new();
        };
        let mut buf = vec![fc4_nettool_neighbour::default(); 16];
        // SAFETY: the host promised a valid callback; the buffer holds `len`.
        let mut n = unsafe { f(self.0.user, occ, buf.as_mut_ptr(), buf.len() as u32) } as usize;
        if n > buf.len() {
            buf = vec![fc4_nettool_neighbour::default(); n.min(1 << 16)];
            // SAFETY: as above.
            n = unsafe { f(self.0.user, occ, buf.as_mut_ptr(), buf.len() as u32) } as usize;
        }
        buf.truncate(n.min(buf.len()));
        buf.iter()
            .map(|nb| Neighbour {
                occ: nb.occ,
                x: nb.x,
                z: nb.z,
                immovable: nb.immovable != 0,
                intersection: nb.intersection != 0,
            })
            .collect()
    }
    fn is_intersection_piece(&self, occ: u64) -> bool {
        cb!(self.0, is_intersection_piece, occ)
    }
    fn intersection_structure(&self, occ: u64) -> Vec<u64> {
        let Some(f) = self.0.intersection_structure else {
            return Vec::new();
        };
        let mut buf = vec![0u64; 16];
        // SAFETY: the host promised a valid callback; the buffer holds `len`.
        let mut n = unsafe { f(self.0.user, occ, buf.as_mut_ptr(), buf.len() as u32) } as usize;
        if n > buf.len() {
            buf = vec![0u64; n.min(1 << 16)];
            // SAFETY: as above.
            n = unsafe { f(self.0.user, occ, buf.as_mut_ptr(), buf.len() as u32) } as usize;
        }
        buf.truncate(n.min(buf.len()));
        buf
    }
    fn is_tunnel_portal(&self, occ: u64) -> bool {
        cb!(self.0, is_tunnel_portal, occ)
    }
    fn paired_portal(&self, occ: u64) -> Option<u64> {
        let t = cb!(self.0, paired_portal, occ);
        (t != 0).then_some(t)
    }
    fn send_pre_demolish_sync(&mut self, occ: u64) {
        cb!(self.0, send_pre_demolish_sync, occ);
    }
    fn remove_occupant(&mut self, occ: u64, shutdown: bool) {
        cb!(self.0, remove_occupant, occ, shutdown);
    }
    fn remove_or_replace_occupant(&mut self, occ: u64) -> bool {
        cb!(self.0, remove_or_replace_occupant, occ)
    }
    fn occupant_shutdown(&self, occ: u64) {
        cb!(self.0, occupant_shutdown, occ);
    }
    fn release(&self, occ: u64) {
        cb!(self.0, release, occ);
    }
    fn solve_network_resolve(&mut self, run: &mut ToolRun) {
        let mut r = run_to_c(run);
        cb!(self.0, solve_network_resolve, &mut r);
        *run = run_from_c(&r);
    }
    fn corner_altitudes(&self, x: i32, z: i32) -> [f32; 4] {
        let mut a = [0.0f32; 4];
        cb!(self.0, corner_altitudes, x, z, a.as_mut_ptr());
        a
    }
    fn smoothen_underground(&mut self, run: &mut ToolRun, level: i32) -> bool {
        let mut r = run_to_c(run);
        let ok = cb!(self.0, smoothen_underground, &mut r, level);
        *run = run_from_c(&r);
        ok
    }
    fn find_underground_solved(&self, x: i32, z: i32) -> Option<(u64, [f32; 4])> {
        let mut s = fc4_nettool_solved::default();
        cb!(self.0, find_underground_solved, x, z, &mut s).then_some((s.occ, s.corners))
    }
    fn destroy_network_occupants(&mut self, occ: &[u64]) {
        cb!(self.0, destroy_network_occupants, occ.as_ptr(), occ.len());
    }
    fn release_occupant_list(&self, occ: &[u64]) {
        cb!(self.0, release_occupant_list, occ.as_ptr(), occ.len());
    }
    fn underground_rebuild(&mut self, occ: u64, outcome: &UndergroundOutcome) {
        let (kind, vh, st) = match outcome {
            UndergroundOutcome::Updated {
                vertex_heights,
                stored_terrain,
            } => (FC4_NETTOOL_U_UPDATED, vertex_heights.as_ptr(), stored_terrain.as_ptr()),
            UndergroundOutcome::Kept { stored_terrain } => {
                (FC4_NETTOOL_U_KEPT, std::ptr::null(), stored_terrain.as_ptr())
            }
            UndergroundOutcome::Destroy => (FC4_NETTOOL_U_DESTROY, std::ptr::null(), std::ptr::null()),
        };
        cb!(self.0, underground_rebuild, occ, kind, vh, st);
    }
}

impl Handle {
    fn refuse(&self) {
        self.refused.set(self.refused.get().wrapping_add(1));
    }
}

/// A caller's struct whose `struct_size` is at least ours.
unsafe fn sized<'a, T>(p: *const T) -> Option<&'a T> {
    let r = p.as_ref()?;
    let size = *(p as *const u32);
    ((size as usize) >= std::mem::size_of::<T>()).then_some(r)
}

fn with_h<R>(g: *const fc4_nettool, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_nettool*.
    match unsafe { (g as *const Handle).as_ref() } {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

fn with_st<R: Copy>(g: *const fc4_nettool, fallback: R, f: impl FnOnce(&mut ToolState) -> R) -> R {
    with_h(g, fallback, |h| match h.st.try_borrow_mut() {
        Ok(mut st) => f(&mut st),
        Err(_) => {
            h.refuse();
            fallback
        }
    })
}

/// The tool over the host table (a stateless bind of the empty world when a
/// callback is missing). NULL on a bad host table.
#[no_mangle]
pub unsafe extern "C" fn fc4_nettool_create(host: *const fc4_nettool_host, type_id: u32) -> *mut fc4_nettool {
    guard(std::ptr::null_mut(), || {
        let Some(h) = sized(host) else {
            return std::ptr::null_mut();
        };
        Box::into_raw(Box::new(Handle {
            // SAFETY: a read of the caller's table; the handle owns the copy
            // for the tool's lifetime.
            host: Box::new(unsafe { std::ptr::read(h) }),
            st: RefCell::new(ToolState::new(type_id)),
            refused: Cell::new(0),
        })) as *mut fc4_nettool
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_nettool_destroy(t: *mut fc4_nettool) {
    if !t.is_null() {
        guard((), || drop(Box::from_raw(t as *mut Handle)));
    }
}

/// `Init` (slot 3): the city size, the underground depth fields and the
/// land-bridge flag from the host's exemplar reads.
#[allow(clippy::too_many_arguments)]
#[no_mangle]
pub unsafe extern "C" fn fc4_nettool_init(
    t: *mut fc4_nettool,
    city_x: u32,
    city_z: u32,
    depth: f32,
    cover_a: f32,
    cover_b: f32,
    piece_tolerance: f32,
    land_bridges: bool,
) -> bool {
    with_st(t, false, |st| {
        st.init(city_x, city_z, depth, cover_a, cover_b, piece_tolerance, land_bridges);
        true
    })
}

/// `Reset` (slot 5): empties the world cache and the step counts.
#[no_mangle]
pub unsafe extern "C" fn fc4_nettool_reset(t: *mut fc4_nettool) -> bool {
    with_st(t, false, |st| {
        st.reset();
        true
    })
}

/// `ClearFailureReason` (slot 13).
#[no_mangle]
pub unsafe extern "C" fn fc4_nettool_clear_failure(t: *mut fc4_nettool) -> bool {
    with_st(t, false, |st| {
        st.clear_failure();
        true
    })
}

/// `EnablePreviewUpdate` (slot 21): `+0x234`.
#[no_mangle]
pub unsafe extern "C" fn fc4_nettool_enable_preview_update(t: *mut fc4_nettool, on: bool) -> bool {
    with_st(t, false, |st| {
        st.preview_update = on;
        true
    })
}

/// `GetFailureState` (slot 14): Boolean, not the internal reason (NT28).
#[no_mangle]
pub unsafe extern "C" fn fc4_nettool_get_failure_state(t: *const fc4_nettool) -> u32 {
    with_st(t, 0, |st| u32::from(st.failure != 0))
}

/// `OccupantExists(x, z, flags, &mask)` (slot 9): the draft query. Writes
/// `mask[0..4]` only when it answers true (zones.md §8.8).
#[no_mangle]
pub unsafe extern "C" fn fc4_nettool_occupant_exists(
    t: *const fc4_nettool,
    x: i32,
    z: i32,
    flags: u32,
    mask: *mut u8,
) -> bool {
    with_st(t, false, |st| {
        let h = &*(t as *const Handle);
        let (ok, m) = CHost(&h.host).occupant_exists(st, x, z, flags);
        if ok && !mask.is_null() {
            for (i, v) in m.iter().enumerate() {
                mask.add(i).write(*v);
            }
        }
        ok
    })
}

/// `PlaceNetworkOccupants(x1, z1, x2, z2, mask, callback, real, type)`
/// (slot 7, §3): AL.
#[allow(clippy::too_many_arguments)]
#[no_mangle]
pub unsafe extern "C" fn fc4_nettool_place_network_occupants(
    t: *mut fc4_nettool,
    x1: i32,
    z1: i32,
    x2: i32,
    z2: i32,
    mask: u32,
    callback: u32,
    real: bool,
    type_id: i32,
) -> bool {
    with_h(t, false, |h| {
        let Ok(mut st) = h.st.try_borrow_mut() else {
            h.refuse();
            return false;
        };
        let mut host = CHost(&h.host);
        place_network_occupants(
            &mut st,
            &mut host,
            x1,
            z1,
            x2,
            z2,
            mask,
            (callback != 0).then_some(callback),
            real,
            type_id,
        )
    })
}

/// `PlaceNetworkOccupantById(x, z, piece, usable, type)` (slot 8, §4).
#[allow(clippy::too_many_arguments)]
#[no_mangle]
pub unsafe extern "C" fn fc4_nettool_place_network_occupant_by_id(
    t: *mut fc4_nettool,
    x: i32,
    z: i32,
    piece: u32,
    usable: bool,
    type_id: i32,
) -> bool {
    with_h(t, false, |h| {
        let Ok(mut st) = h.st.try_borrow_mut() else {
            h.refuse();
            return false;
        };
        let mut host = CHost(&h.host);
        place_network_occupant_by_id(&mut st, &mut host, x, z, piece, usable, type_id);
        true
    })
}

/// `PlaceNetwork(real, crew, arg3, neighbourMode, type)` (§5) on a caller's
/// run: 1 on success, 0 on refusal (the failure code is in the state).
#[no_mangle]
pub unsafe extern "C" fn fc4_nettool_place_network(
    t: *mut fc4_nettool,
    run: *const fc4_nettool_run,
    crew: u64,
    arg3: u32,
    neighbour_mode: u32,
    real: bool,
) -> bool {
    let Some(r) = (unsafe { sized(run) }) else {
        return false;
    };
    let r_in = run_from_c(r);
    with_h(t, false, |h| {
        let Ok(mut st) = h.st.try_borrow_mut() else {
            h.refuse();
            return false;
        };
        let mut host = CHost(&h.host);
        let type_id = r_in.type_id;
        let mut run = r_in;
        place_network(
            &mut st,
            &mut host,
            &mut run,
            real,
            (crew != 0).then_some(crew),
            arg3,
            neighbour_mode,
            type_id,
        )
    })
}

/// `DestroyOccupantsAndResolve(list)` (slot 50, §10).
#[no_mangle]
pub unsafe extern "C" fn fc4_nettool_destroy_occupants_and_resolve(
    t: *mut fc4_nettool,
    list: *const fc4_nettool_listed,
    count: u32,
) -> bool {
    let slice = if list.is_null() || count == 0 {
        &[][..]
    } else {
        // SAFETY: documented precondition: `count` live fc4_nettool_listed.
        unsafe { std::slice::from_raw_parts(list, count as usize) }
    };
    let listed: Vec<Listed> = slice
        .iter()
        .map(|l| Listed {
            occ: l.occ,
            x: l.x,
            z: l.z,
        })
        .collect();
    with_h(t, false, |h| {
        let Ok(mut st) = h.st.try_borrow_mut() else {
            h.refuse();
            return false;
        };
        let mut host = CHost(&h.host);
        destroy_occupants_and_resolve(&mut st, &mut host, &listed);
        true
    })
}

/// One collected occupant of §10 step 2.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_nettool_listed {
    pub occ: u64,
    pub x: i32,
    pub z: i32,
}

/// `UpdateUndergroundOccupantTerrainHeights(list, type)` (`0x006588D0`, §11):
/// the pipe/subway refit.
#[no_mangle]
pub unsafe extern "C" fn fc4_nettool_update_underground_heights(
    t: *mut fc4_nettool,
    pieces: *const fc4_nettool_underground,
    count: u32,
    network_type: u32,
) -> bool {
    let slice = if pieces.is_null() || count == 0 {
        &[][..]
    } else {
        // SAFETY: documented precondition: `count` live fc4_nettool_underground.
        unsafe { std::slice::from_raw_parts(pieces, count as usize) }
    };
    let list: Vec<UndergroundPiece> = slice
        .iter()
        .map(|p| UndergroundPiece {
            occ: p.occ,
            x: p.x,
            z: p.z,
            stored_terrain: p.stored_terrain,
            vertices: p.vertices,
            edges: p.edges,
        })
        .collect();
    with_h(t, false, |h| {
        let Ok(mut st) = h.st.try_borrow_mut() else {
            h.refuse();
            return false;
        };
        let mut host = CHost(&h.host);
        update_underground_occupant_terrain_heights(&mut st, &mut host, &list, network_type);
        true
    })
}

/// One underground piece of §11.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_nettool_underground {
    pub occ: u64,
    pub x: i32,
    pub z: i32,
    /// Slot 94: the stored terrain heights.
    pub stored_terrain: [f32; 4],
    /// Slot 34: the vertex heights.
    pub vertices: [f32; 4],
    /// Slot 38 with the type: the edge mask.
    pub edges: u32,
}

/// Diagnostics.
#[no_mangle]
pub unsafe extern "C" fn fc4_nettool_get_state(t: *const fc4_nettool, out: *mut fc4_nettool_state) -> bool {
    if out.is_null() {
        return false;
    }
    with_h(t, false, |h| {
        let Ok(st) = h.st.try_borrow() else {
            h.refuse();
            return false;
        };
        unsafe {
            *out = fc4_nettool_state {
                struct_size: std::mem::size_of::<fc4_nettool_state>() as u32,
                type_id: st.type_id,
                real: u8::from(st.real),
                stop_on_failure: u8::from(st.stop_on_failure),
                destroyed_flag: u8::from(st.destroyed_flag),
                preview_update: u8::from(st.preview_update),
                reserved: [0; 4],
                data4: st.data4,
                failure: st.failure,
                demolition_cost: st.demolition_cost,
                layer_mask: st.layer_mask,
                depth: st.depth,
                cover_margin_a: st.cover_margin_a,
                cover_margin_b: st.cover_margin_b,
                city_x: st.city_x,
                city_z: st.city_z,
                cache_count: st.world_cache.len() as u32,
                reentrant_refused: h.refused.get(),
            };
        }
        true
    })
}
