// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The plumbing simulator's C ABI (fc4.h "plumbing", docs/specs/plumbing.md).
//!
//! The host implements [`fc4_plumbing_host`] (every `PlumbingHost` method of
//! the core, plus the game's grids through [`fc4_grid_api`]); the plugin's
//! `cISC4PlumbingSimulator` forwards its slots to the `fc4_plumbing_*`
//! functions.
//!
//! Tokens are opaque host identities (fc4.h): occupants (also pipes and water
//! producers), lots, deals, connections, cities, departments, line items. 0 is
//! the null object.
//!
//! Re-entrancy: the core takes its state out of the simulator for the duration
//! of a call, so a host callback that calls back in would see an empty state.
//! [choice] A message delivered while a call is running is queued and handled
//! when the outermost call returns; other re-entrant calls are answered from
//! the (empty) running state and counted (`fc4_plumbing_state::reentrant_calls`).

use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_void};
use std::rc::Rc;

use fc4_core::grid::{CellRect, GridProvider, TractGrid};
use fc4_core::plumbing::{BudgetItem, PlumbingHost, PlumbingSim};
use fc4_core::property::PropertyHolder;
use fc4_core::stream::{RefValue, SaveObject};
use fc4_core::variant::Variant;

use crate::budget::fc4_token_callback;
use crate::guard;
use crate::hostgrid::{
    collect_tokens, create_grid, fc4_cell_rect, fc4_grid_api, grid_token, native_variant, read_property, resolve_grid,
    take_props,
};
use crate::props::fc4_props;
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

/// `GetBudgetItemForPurpose` (fc4.h `fc4_plumbing_budget_item`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_plumbing_budget_item {
    pub department: u32,
    pub line: u32,
    pub cost: i64,
    pub index: u32,
    pub reserved: u32,
}

/// A message for `fc4_plumbing_do_message` (fc4.h `fc4_plumbing_message`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_plumbing_message {
    pub msg_type: u32,
    pub data1: u32,
    pub data2: u32,
    pub data3: u32,
    pub reserved: u32,
    /// `GetVoid1()` as a `cISC4Occupant` (Insert / RemoveOccupant).
    pub occupant: u64,
    /// `GetIGZUnknown()` queried for `cISC4NeighborDeal` (deal status).
    pub deal: u64,
}

/// `GetWaterProducerInfo` outputs (fc4.h `fc4_plumbing_producer_info`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_plumbing_producer_info {
    pub production: u32,
    pub max_capacity: u32,
    pub efficiency: f32,
    pub polluted: bool,
    pub used: u32,
}

/// `FindWaterBuildings` results (fc4.h `fc4_plumbing_found`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_plumbing_found {
    pub has_lowest_funded: u32,
    pub lowest_funding: f32,
    pub lowest_funded: u64,
    pub has_worst_condition: u32,
    pub condition_percent: i32,
    pub worst_condition: u64,
}

/// Diagnostics (fc4.h `fc4_plumbing_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_plumbing_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub grids_initialized: u8,
    pub enabled: u8,
    pub recompute: u8,
    pub state: u32,
    pub count_x: i32,
    pub count_z: i32,
    pub pass_counter: i32,
    pub producers: u32,
    pub num_pipes: u32,
    pub num_distressed: u32,
    pub num_burst: u32,
    pub num_polluted: u32,
    pub average_water_pollution: u32,
    pub watered_cells: u32,
    pub building_cells: u32,
    pub reentrant_calls: u32,
    pub reserved: u32,
    pub pipe_condition: f32,
    pub factor: f32,
    pub demand: i64,
    pub total_water_cost: i64,
    pub last_produced: i64,
    pub last_consumed: i64,
    pub last_imported: i64,
    pub last_exported: i64,
    pub department: u64,
    pub working_grid: u64,
    pub published_grid: u64,
}

/// Everything the plumbing simulator asks of the game (fc4.h
/// `fc4_plumbing_host`; one callback per `PlumbingHost` method, in the trait's
/// order).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_plumbing_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    pub grids: fc4_grid_api,
    // city
    pub cell_counts: Option<unsafe extern "C" fn(U, *mut i32, *mut i32)>,
    pub position_to_cell: Option<unsafe extern "C" fn(U, f32, f32, *mut i32, *mut i32)>,
    /// A NEW fc4_props (the core destroys it), NULL if the exemplar is missing.
    pub tuning: Option<unsafe extern "C" fn(U) -> *mut fc4_props>,
    pub wall_clock_seed: Option<unsafe extern "C" fn(U) -> u32>,
    // simulator, messages, history
    pub add_agent: Option<unsafe extern "C" fn(U, u32, *const c_char, usize, u32)>,
    pub remove_agent: Option<unsafe extern "C" fn(U)>,
    pub subscribe: Option<unsafe extern "C" fn(U, *const u32, usize)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, *const u32, usize)>,
    pub post: Option<unsafe extern "C" fn(U, u32, u32)>,
    pub record_history: Option<unsafe extern "C" fn(U, u32, u32)>,
    pub cumulative_factor: Option<unsafe extern "C" fn(U, u32, u32, *mut f32) -> bool>,
    pub water_value: Option<unsafe extern "C" fn(U, i32, i32) -> i32>,
    // occupants
    pub occupant_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub occupant_property: Option<unsafe extern "C" fn(U, u64, u32, *mut fc4_props) -> bool>,
    pub set_occupant_property: Option<unsafe extern "C" fn(U, u64, u32, u16, bool, *const c_void, usize)>,
    pub occupant_position: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    pub occupant_bounding_box: Option<unsafe extern "C" fn(U, u64, *mut f32, *mut f32)>,
    pub occupant_bounding_cells: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    pub building_type: Option<unsafe extern "C" fn(U, u64, *mut u32) -> bool>,
    pub buildings_in_cell: Option<unsafe extern "C" fn(U, i32, i32, fc4_token_callback, *mut c_void)>,
    // network occupants (pipes)
    pub is_network_occupant: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub has_network_flag: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub network_flags: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub set_network_flag: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub clear_network_flag: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub connects_to_neighbor: Option<unsafe extern "C" fn(U, u64, u32, u32) -> bool>,
    pub occupied_cell: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32)>,
    pub set_pipe_watered: Option<unsafe extern "C" fn(U, u64, bool)>,
    // lots
    pub lot_at: Option<unsafe extern "C" fn(U, i32, i32) -> u64>,
    pub occupant_lot: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub lot_rect: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    pub lot_needs_power: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub lot_has_power: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub set_lot_condition: Option<unsafe extern "C" fn(U, u64, u8)>,
    // terrain
    pub terrain_cell_counts: Option<unsafe extern "C" fn(U, *mut i32, *mut i32) -> bool>,
    pub cell_has_water: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    pub redisplay_terrain: Option<unsafe extern "C" fn(U, i32, i32, i32, i32)>,
    // disasters
    pub attempt_pipe_burst: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    // neighbours
    pub find_deals: Option<unsafe extern "C" fn(U, u32, fc4_token_callback, *mut c_void)>,
    pub deal_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub deal_city: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub deal_amount: Option<unsafe extern "C" fn(U, u64) -> i32>,
    pub deal_amount_used: Option<unsafe extern "C" fn(U, u64) -> i32>,
    pub set_deal_amount_used: Option<unsafe extern "C" fn(U, u64, i32)>,
    pub set_deal_flags: Option<unsafe extern "C" fn(U, u64, u32, bool)>,
    pub connections_of_type: Option<unsafe extern "C" fn(U, u32, fc4_token_callback, *mut c_void)>,
    pub connections_of_type_to_city: Option<unsafe extern "C" fn(U, u32, u64, fc4_token_callback, *mut c_void)>,
    pub find_unvisited_connection: Option<unsafe extern "C" fn(U, u32, u64) -> u64>,
    pub connection_on_other_side: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub connection_cell: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32)>,
    pub set_connection_visited: Option<unsafe extern "C" fn(U, u64, bool)>,
    pub connection_is_active: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub set_connection_active: Option<unsafe extern "C" fn(U, u64, bool)>,
    pub mark_deals_dirty: Option<unsafe extern "C" fn(U)>,
    // budget
    pub create_department: Option<unsafe extern "C" fn(U, u32, u32) -> u64>,
    pub set_department_name: Option<unsafe extern "C" fn(U, u64, u32, u32)>,
    pub set_department_max_funding: Option<unsafe extern "C" fn(U, u64, f32)>,
    pub create_line_item: Option<unsafe extern "C" fn(U, u64, u32, bool) -> u64>,
    pub set_line_item_name: Option<unsafe extern "C" fn(U, u64, u32, u32)>,
    pub remove_line_item: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub department_funding: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub department_by_id: Option<unsafe extern "C" fn(U, u32) -> u64>,
    pub line_item: Option<unsafe extern "C" fn(U, u64, u32) -> u64>,
    pub create_line_item_for_building_type: Option<unsafe extern "C" fn(U, u64, u32, bool) -> u64>,
    pub set_line_item_display_flag: Option<unsafe extern "C" fn(U, u64, u32, bool)>,
    pub add_to_line_item_expenses: Option<unsafe extern "C" fn(U, u64, i64)>,
    pub set_line_item_expenses: Option<unsafe extern "C" fn(U, u64, i64)>,
    pub line_item_secondary_info: Option<unsafe extern "C" fn(U, u64) -> i64>,
    pub set_line_item_secondary_info: Option<unsafe extern "C" fn(U, u64, i64)>,
    pub budget_item_for_purpose: Option<unsafe extern "C" fn(U, u64, u32, *mut fc4_plumbing_budget_item) -> bool>,
    pub change_budget_item_line: Option<unsafe extern "C" fn(U, u64, u32, u32)>,
    pub change_budget_item_cost: Option<unsafe extern "C" fn(U, u64, u32, i64)>,
    pub line_item_from_budget_item: Option<unsafe extern "C" fn(U, *const fc4_plumbing_budget_item) -> u64>,
    pub local_funding: Option<unsafe extern "C" fn(U, u64, u64, *mut f32) -> bool>,
    // references
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    pub resolve_ref: Option<unsafe extern "C" fn(U, u64, u32) -> u64>,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_plumbing_budget_item>() == 24);
    assert!(std::mem::size_of::<fc4_plumbing_message>() == 40);
    assert!(std::mem::size_of::<fc4_plumbing_producer_info>() == 20);
    assert!(std::mem::size_of::<fc4_plumbing_found>() == 32);
    assert!(std::mem::size_of::<fc4_plumbing_state>() == 144);
    assert!(std::mem::size_of::<fc4_plumbing_host>() == if cfg!(target_pointer_width = "32") { 356 } else { 712 });
};

// ---- host adapter -----------------------------------------------------------------------------------

struct CHost(fc4_plumbing_host);

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.0.$f { Some(f) => unsafe { f($s.0.user $(, $arg)*) }, None => $default }
    };
}

fn opt(t: u64) -> Option<u64> {
    (t != 0).then_some(t)
}

impl GridProvider<u8> for CHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        create_grid::<u8>(&self.0.grids, instance, tract_size, true)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u8>>> {
        resolve_grid::<u8>(&self.0.grids, r, true)
    }
}

impl PlumbingHost for CHost {
    fn cell_counts(&self) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        cb!(self, cell_counts, (), &mut x, &mut z);
        (x, z)
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        let (mut cx, mut cz) = (0, 0);
        cb!(self, position_to_cell, (), x, z, &mut cx, &mut cz);
        (cx, cz)
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, tuning, std::ptr::null_mut());
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }
    fn wall_clock_seed(&self) -> u32 {
        cb!(self, wall_clock_seed, 0)
    }
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        cb!(
            self,
            add_agent,
            (),
            level,
            name.as_ptr() as *const c_char,
            name.len(),
            flags
        )
    }
    fn remove_agent(&self) {
        cb!(self, remove_agent, ())
    }
    fn subscribe(&self, msg_types: &[u32]) {
        cb!(self, subscribe, (), msg_types.as_ptr(), msg_types.len())
    }
    fn unsubscribe(&self, msg_types: &[u32]) {
        cb!(self, unsubscribe, (), msg_types.as_ptr(), msg_types.len())
    }
    fn post(&self, msg_type: u32, data1: u32) {
        cb!(self, post, (), msg_type, data1)
    }
    fn record_history(&self, id: u32, value: u32) {
        cb!(self, record_history, (), id, value)
    }
    fn cumulative_factor(&self, id: u32, index: u32) -> Option<f32> {
        let mut v = 0.0f32;
        cb!(self, cumulative_factor, false, id, index, &mut v).then_some(v)
    }
    fn water_value(&self, x: i32, z: i32) -> i32 {
        cb!(self, water_value, 0, x, z)
    }
    fn occupant_type(&self, occ: u64) -> u32 {
        cb!(self, occupant_type, 0, occ)
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant> {
        let f = self.0.occupant_property?;
        // SAFETY: valid callback; the scratch holder outlives the call.
        read_property(id, |p| unsafe { f(self.0.user, occ, id, p) })
    }
    fn set_occupant_property(&self, occ: u64, id: u32, value: Variant) {
        let Some(n) = native_variant(&value) else { return };
        cb!(
            self,
            set_occupant_property,
            (),
            occ,
            id,
            n.mem_type,
            n.is_array,
            n.bytes.as_ptr() as *const c_void,
            n.count
        )
    }
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        let mut p = [0.0f32; 3];
        cb!(self, occupant_position, (), occ, p.as_mut_ptr());
        p
    }
    fn occupant_bounding_box(&self, occ: u64) -> ([f32; 3], [f32; 3]) {
        let (mut a, mut b) = ([0.0f32; 3], [0.0f32; 3]);
        cb!(self, occupant_bounding_box, (), occ, a.as_mut_ptr(), b.as_mut_ptr());
        (a, b)
    }
    fn occupant_bounding_cells(&self, occ: u64) -> CellRect {
        let mut r = fc4_cell_rect::default();
        cb!(self, occupant_bounding_cells, (), occ, &mut r);
        r.into()
    }
    fn building_type(&self, occ: u64) -> Option<u32> {
        let mut t = 0;
        cb!(self, building_type, false, occ, &mut t).then_some(t)
    }
    fn buildings_in_cell(&self, x: i32, z: i32) -> Vec<u64> {
        collect_tokens(|sink, ctx| cb!(self, buildings_in_cell, (), x, z, sink, ctx))
    }
    fn is_network_occupant(&self, occ: u64) -> bool {
        cb!(self, is_network_occupant, false, occ)
    }
    fn has_network_flag(&self, occ: u64, flag: u32) -> bool {
        cb!(self, has_network_flag, false, occ, flag)
    }
    fn network_flags(&self, occ: u64) -> u32 {
        cb!(self, network_flags, 0, occ)
    }
    fn set_network_flag(&self, occ: u64, flag: u32) {
        cb!(self, set_network_flag, (), occ, flag)
    }
    fn clear_network_flag(&self, occ: u64, flag: u32) {
        cb!(self, clear_network_flag, (), occ, flag)
    }
    fn connects_to_neighbor(&self, occ: u64, dir: u32, network_type: u32) -> bool {
        cb!(self, connects_to_neighbor, false, occ, dir, network_type)
    }
    fn occupied_cell(&self, occ: u64) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        cb!(self, occupied_cell, (), occ, &mut x, &mut z);
        (x, z)
    }
    fn set_pipe_watered(&self, occ: u64, watered: bool) {
        cb!(self, set_pipe_watered, (), occ, watered)
    }
    fn lot_at(&self, x: i32, z: i32) -> Option<u64> {
        opt(cb!(self, lot_at, 0, x, z))
    }
    fn occupant_lot(&self, occ: u64) -> Option<u64> {
        opt(cb!(self, occupant_lot, 0, occ))
    }
    fn lot_rect(&self, lot: u64) -> CellRect {
        let mut r = fc4_cell_rect::default();
        cb!(self, lot_rect, (), lot, &mut r);
        r.into()
    }
    fn lot_needs_power(&self, lot: u64) -> bool {
        cb!(self, lot_needs_power, false, lot)
    }
    fn lot_has_power(&self, lot: u64) -> bool {
        cb!(self, lot_has_power, false, lot)
    }
    fn set_lot_condition(&self, lot: u64, condition: u8) {
        cb!(self, set_lot_condition, (), lot, condition)
    }
    fn terrain_cell_counts(&self) -> Option<(i32, i32)> {
        let (mut x, mut z) = (0, 0);
        cb!(self, terrain_cell_counts, false, &mut x, &mut z).then_some((x, z))
    }
    fn cell_has_water(&self, x: i32, z: i32) -> bool {
        cb!(self, cell_has_water, false, x, z)
    }
    fn redisplay_terrain(&self, min: (i32, i32), max: (i32, i32)) {
        cb!(self, redisplay_terrain, (), min.0, min.1, max.0, max.1)
    }
    fn attempt_pipe_burst(&self, x: i32, z: i32) -> bool {
        cb!(self, attempt_pipe_burst, false, x, z)
    }
    fn find_deals(&self, deal_type: u32) -> Vec<u64> {
        collect_tokens(|sink, ctx| cb!(self, find_deals, (), deal_type, sink, ctx))
    }
    fn deal_type(&self, deal: u64) -> u32 {
        cb!(self, deal_type, 0, deal)
    }
    fn deal_city(&self, deal: u64) -> u64 {
        cb!(self, deal_city, 0, deal)
    }
    fn deal_amount(&self, deal: u64) -> i32 {
        cb!(self, deal_amount, 0, deal)
    }
    fn deal_amount_used(&self, deal: u64) -> i32 {
        cb!(self, deal_amount_used, 0, deal)
    }
    fn set_deal_amount_used(&self, deal: u64, used: i32) {
        cb!(self, set_deal_amount_used, (), deal, used)
    }
    fn set_deal_flags(&self, deal: u64, flags: u32, on: bool) {
        cb!(self, set_deal_flags, (), deal, flags, on)
    }
    fn connections_of_type(&self, connection_type: u32) -> Vec<u64> {
        collect_tokens(|sink, ctx| cb!(self, connections_of_type, (), connection_type, sink, ctx))
    }
    fn connections_of_type_to_city(&self, connection_type: u32, city: u64) -> Vec<u64> {
        collect_tokens(|sink, ctx| cb!(self, connections_of_type_to_city, (), connection_type, city, sink, ctx))
    }
    fn find_unvisited_connection(&self, connection_type: u32, city: u64) -> Option<u64> {
        opt(cb!(self, find_unvisited_connection, 0, connection_type, city))
    }
    fn connection_on_other_side(&self, conn: u64) -> Option<u64> {
        opt(cb!(self, connection_on_other_side, 0, conn))
    }
    fn connection_cell(&self, conn: u64) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        cb!(self, connection_cell, (), conn, &mut x, &mut z);
        (x, z)
    }
    fn set_connection_visited(&self, conn: u64, visited: bool) {
        cb!(self, set_connection_visited, (), conn, visited)
    }
    fn connection_is_active(&self, conn: u64) -> bool {
        cb!(self, connection_is_active, false, conn)
    }
    fn set_connection_active(&self, conn: u64, active: bool) {
        cb!(self, set_connection_active, (), conn, active)
    }
    fn mark_deals_dirty(&self) {
        cb!(self, mark_deals_dirty, ())
    }
    fn create_department(&self, id: u32, group: u32) -> Option<u64> {
        opt(cb!(self, create_department, 0, id, group))
    }
    fn set_department_name(&self, dept: u64, group: u32, instance: u32) {
        cb!(self, set_department_name, (), dept, group, instance)
    }
    fn set_department_max_funding(&self, dept: u64, fraction: f32) {
        cb!(self, set_department_max_funding, (), dept, fraction)
    }
    fn create_line_item(&self, dept: u64, line: u32, locally_funded: bool) -> Option<u64> {
        opt(cb!(self, create_line_item, 0, dept, line, locally_funded))
    }
    fn set_line_item_name(&self, line_item: u64, group: u32, instance: u32) {
        cb!(self, set_line_item_name, (), line_item, group, instance)
    }
    fn remove_line_item(&self, dept: u64, line: u32) {
        cb!(self, remove_line_item, (), dept, line)
    }
    fn department_funding(&self, dept: u64) -> f32 {
        cb!(self, department_funding, 1.0, dept)
    }
    fn department_by_id(&self, id: u32) -> Option<u64> {
        opt(cb!(self, department_by_id, 0, id))
    }
    fn line_item(&self, dept: u64, line: u32) -> Option<u64> {
        opt(cb!(self, line_item, 0, dept, line))
    }
    fn create_line_item_for_building_type(&self, dept: u64, line: u32, locally_funded: bool) -> Option<u64> {
        opt(cb!(
            self,
            create_line_item_for_building_type,
            0,
            dept,
            line,
            locally_funded
        ))
    }
    fn set_line_item_display_flag(&self, line_item: u64, flag: u32, on: bool) {
        cb!(self, set_line_item_display_flag, (), line_item, flag, on)
    }
    fn add_to_line_item_expenses(&self, line_item: u64, delta: i64) {
        cb!(self, add_to_line_item_expenses, (), line_item, delta)
    }
    fn set_line_item_expenses(&self, line_item: u64, value: i64) {
        cb!(self, set_line_item_expenses, (), line_item, value)
    }
    fn line_item_secondary_info(&self, line_item: u64) -> i64 {
        cb!(self, line_item_secondary_info, 0, line_item)
    }
    fn set_line_item_secondary_info(&self, line_item: u64, value: i64) {
        cb!(self, set_line_item_secondary_info, (), line_item, value)
    }
    fn budget_item_for_purpose(&self, occ: u64, purpose: u32) -> Option<BudgetItem> {
        let mut i = fc4_plumbing_budget_item::default();
        cb!(self, budget_item_for_purpose, false, occ, purpose, &mut i).then_some(BudgetItem {
            department_id: i.department,
            line_item: i.line,
            cost: i.cost,
            index: i.index,
        })
    }
    fn change_budget_item_line(&self, occ: u64, index: u32, line: u32) {
        cb!(self, change_budget_item_line, (), occ, index, line)
    }
    fn change_budget_item_cost(&self, occ: u64, index: u32, cost: i64) {
        cb!(self, change_budget_item_cost, (), occ, index, cost)
    }
    fn line_item_from_budget_item(&self, item: &BudgetItem) -> Option<u64> {
        let i = fc4_plumbing_budget_item {
            department: item.department_id,
            line: item.line_item,
            cost: item.cost,
            index: item.index,
            reserved: 0,
        };
        opt(cb!(self, line_item_from_budget_item, 0, &i))
    }
    fn local_funding(&self, dept: u64, occ: u64) -> Option<f32> {
        let mut f = 0.0f32;
        cb!(self, local_funding, false, dept, occ, &mut f).then_some(f)
    }
    fn add_ref(&self, token: u64) {
        if token != 0 {
            cb!(self, add_ref, (), token)
        }
    }
    fn release(&self, token: u64) {
        if token != 0 {
            cb!(self, release, (), token)
        }
    }
    fn object_ref(&self, token: u64) -> RefValue {
        if token == 0 {
            RefValue::Null
        } else {
            RefValue::Host { token, class_id: 0 }
        }
    }
    fn resolve_ref(&self, r: &RefValue, iid: u32) -> Option<u64> {
        match *r {
            RefValue::Host { token, .. } if token != 0 => opt(cb!(self, resolve_ref, 0, token, iid)),
            _ => None,
        }
    }
    fn grid_shutdown(&self, grid: &Rc<dyn TractGrid<u8>>) {
        if let Some(f) = self.0.grids.shutdown {
            let t = grid.save_ref();
            if let RefValue::Host { token, .. } = t {
                if token != 0 {
                    // SAFETY: valid callback; the token is the game's grid.
                    unsafe { f(self.0.grids.user, token) };
                }
            }
        }
    }
}

// ---- handles ----------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_plumbing`).
#[repr(C)]
pub struct fc4_plumbing {
    _private: [u8; 0],
}

struct Handle {
    host: Rc<CHost>,
    sim: Rc<PlumbingSim>,
    depth: Cell<u32>,
    queue: RefCell<Vec<fc4_core::plumbing::Message>>,
    reentrant: Cell<u32>,
}

impl Handle {
    /// Runs `f` with the call depth counted; at the end of the outermost
    /// call, the queued messages are handled.
    fn call<R>(&self, f: impl FnOnce(&PlumbingSim) -> R) -> R {
        if self.depth.get() > 0 {
            self.reentrant.set(self.reentrant.get().wrapping_add(1));
        }
        self.depth.set(self.depth.get() + 1);
        let r = f(&self.sim);
        if self.depth.get() == 1 {
            loop {
                let next = {
                    let mut q = self.queue.borrow_mut();
                    if q.is_empty() {
                        None
                    } else {
                        Some(q.remove(0))
                    }
                };
                let Some(m) = next else { break };
                self.sim.do_message(&m);
            }
        }
        self.depth.set(self.depth.get() - 1);
        r
    }
}

/// Runs `f` on the handle; the fallback for NULL or a panic.
fn with_plumbing<R>(p: *const fc4_plumbing, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_plumbing*.
    let Some(h) = (unsafe { (p as *const Handle).as_ref() }) else {
        return fallback;
    };
    guard(fallback, || f(h))
}

fn sim<R>(p: *const fc4_plumbing, fallback: R, f: impl FnOnce(&PlumbingSim) -> R) -> R {
    with_plumbing(p, fallback, |h| h.call(f))
}

fn message_of(m: &fc4_plumbing_message) -> fc4_core::plumbing::Message {
    fc4_core::plumbing::Message {
        msg_type: m.msg_type,
        data1: m.data1,
        data2: m.data2,
        data3: m.data3,
        occupant: opt(m.occupant),
        deal: opt(m.deal),
    }
}

/// Creates the core object (constructor values, §3). The host table is
/// copied. `token` is the host identity of the simulator itself.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_create(host: *const fc4_plumbing_host, token: u64) -> *mut fc4_plumbing {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_plumbing_host>() && h.grids.complete() => {
            let host = Rc::new(CHost(*h));
            let sim = PlumbingSim::new(host.clone(), token);
            Box::into_raw(Box::new(Handle {
                host,
                sim,
                depth: Cell::new(0),
                queue: RefCell::new(Vec::new()),
                reentrant: Cell::new(0),
            })) as *mut fc4_plumbing
        }
        _ => std::ptr::null_mut(),
    })
}

/// Destroys the core object. The references it still holds (a loaded object
/// that was never initialised) are released.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_destroy(p: *mut fc4_plumbing) {
    if p.is_null() {
        return;
    }
    guard((), || {
        let b = Box::from_raw(p as *mut Handle);
        if !b.sim.is_initialized() {
            let s = b.sim.snapshot();
            for t in s
                .department
                .into_iter()
                .chain(s.producers.iter().map(|p| p.occupant))
                .chain(s.pipes.iter().copied())
                .chain(s.leaked.iter().copied())
            {
                b.host.release(t);
            }
        }
        drop(b);
    });
}

/// `Init` (§4.1): always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_init(p: *mut fc4_plumbing) -> bool {
    sim(p, false, PlumbingSim::init)
}

/// `Shutdown` (§4.2): always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_shutdown(p: *mut fc4_plumbing) -> bool {
    sim(p, false, PlumbingSim::shutdown)
}

/// `DoMessage` (§5.1): true except for the day and idle messages, which return
/// `DoIdle`'s result. A message delivered during another call is queued.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_do_message(p: *mut fc4_plumbing, m: *const fc4_plumbing_message) -> bool {
    let Some(m) = m.as_ref() else { return true };
    let msg = message_of(m);
    with_plumbing(p, true, |h| {
        if h.depth.get() > 0 {
            h.queue.borrow_mut().push(msg);
            return true;
        }
        h.call(|s| s.do_message(&msg))
    })
}

/// The argument-free slots that return a number: 12, 13, 14, 15, 16, 17, 18,
/// 19, 20, 21, 22, 23 (§10.1). 0 for any other slot. Slots 14-17 and 23 are
/// 64-bit totals whose low 32 bits are the EAX the game returns (§10.4).
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_slot_value(p: *const fc4_plumbing, slot: u32) -> i64 {
    sim(p, 0, |s| match slot {
        12 => i64::from(s.num_water_sources()),
        13 => i64::from(s.num_polluted_water_sources()),
        14 => s.last_month().produced,
        15 => s.last_month().consumed,
        16 => s.last_month().imported,
        17 => s.last_month().exported,
        18 => i64::from(s.pipe_counts().0),
        19 => i64::from(s.pipe_counts().1),
        20 => i64::from(s.pipe_counts().2),
        21 => i64::from(s.num_watered_buildings()),
        22 => i64::from(s.num_unwatered_buildings()),
        23 => s.water_demand(),
        _ => 0,
    })
}

/// Slot 5 `GetSupplyValue(x, z)` (a raw published-grid read).
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_supply_value(p: *const fc4_plumbing, x: i32, z: i32) -> u8 {
    sim(p, 0, |s| s.supply_value(x, z))
}

/// Slot 8 `AreWateredBuildingsInCell(x, z)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_are_watered_buildings_in_cell(p: *const fc4_plumbing, x: i32, z: i32) -> bool {
    sim(p, false, |s| s.are_watered_buildings_in_cell(x, z))
}

/// Slot 9 `IsBuildingWatered(occ)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_is_building_watered(p: *const fc4_plumbing, occ: u64) -> bool {
    sim(p, false, |s| s.is_building_watered(occ))
}

/// Slot 10 `GetWaterProduction(occ)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_water_production(p: *const fc4_plumbing, occ: u64) -> u32 {
    sim(p, 0, |s| s.water_production(occ))
}

/// Slot 11 `IsCellWatered(x, z)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_is_cell_watered(p: *const fc4_plumbing, x: i32, z: i32) -> bool {
    sim(p, false, |s| s.is_cell_watered(x, z))
}

/// Slot 24 `GetWaterProducerInfo`: false = not a producer.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_get_producer_info(
    p: *const fc4_plumbing,
    occ: u64,
    out: *mut fc4_plumbing_producer_info,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    let Some(i) = sim(p, None, |s| s.water_producer_info(occ)) else {
        return false;
    };
    *o = fc4_plumbing_producer_info {
        production: i.production,
        max_capacity: i.max_capacity,
        efficiency: i.efficiency,
        polluted: i.polluted,
        used: i.used,
    };
    true
}

/// Slot 25 `GetWaterProducerConditionPercent`: false = absent.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_producer_condition_percent(
    p: *const fc4_plumbing,
    occ: u64,
    out: *mut f32,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    let Some(v) = sim(p, None, |s| s.water_producer_condition_percent(occ)) else {
        return false;
    };
    *o = v;
    true
}

/// Slot 26 `GetOccupantMonthlyUpkeepCost`: false = no budget item.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_occupant_monthly_upkeep_cost(
    p: *const fc4_plumbing,
    occ: u64,
    out: *mut i64,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    let Some(v) = sim(p, None, |s| s.occupant_monthly_upkeep_cost(occ)) else {
        return false;
    };
    *o = v;
    true
}

/// Slot 27 `FindWaterBuildings`. Each occupant is a BORROWED token; the plugin
/// AddRefs one it writes to a non-null output pointer.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_find_water_buildings(
    p: *const fc4_plumbing,
    out: *mut fc4_plumbing_found,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    let (lowest, worst) = sim(p, (None, None), PlumbingSim::find_water_buildings);
    o.has_lowest_funded = 0;
    o.lowest_funding = 0.0;
    o.lowest_funded = 0;
    o.has_worst_condition = 0;
    o.condition_percent = 0;
    o.worst_condition = 0;
    if let Some((occ, f)) = lowest {
        o.has_lowest_funded = 1;
        o.lowest_funding = f;
        o.lowest_funded = occ;
    }
    if let Some((occ, pct)) = worst {
        o.has_worst_condition = 1;
        o.condition_percent = pct;
        o.worst_condition = occ;
    }
    true
}

/// Slot 28 `GetWaterProducers`: the producer occupants in list order, as
/// **borrowed** tokens (the core's producer list holds the references). The
/// caller AddRefs each one it appends. Returns the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_water_producers(
    p: *const fc4_plumbing,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) -> usize {
    let producers = sim(p, Vec::new(), PlumbingSim::water_producers);
    if let Some(f) = cb {
        for &t in &producers {
            f(ctx, t);
        }
    }
    producers.len()
}

/// Slots 6 / 7: the published grid's token (borrowed; 0 = none).
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_watered_grid(p: *const fc4_plumbing) -> u64 {
    sim(p, 0, |s| grid_token(&s.watered_grid()))
}

/// Test seam (no C entry point): cells per `DoIdle` in states 6 and 7
/// (`None`: the whole state), so a test's stand-in for the original slices
/// them as the original does (plumbing.md §6).
#[cfg(test)]
pub(crate) fn test_set_slice_cells(p: *mut fc4_plumbing, cells: Option<u32>) {
    sim(p, (), |s| s.set_slice_cells(cells))
}

/// Diagnostics.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_get_state(p: *const fc4_plumbing, out: *mut fc4_plumbing_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    let Some(h) = (p as *const Handle).as_ref() else {
        return false;
    };
    guard(false, || {
        let s = h.sim.snapshot();
        let reentrant = h.reentrant.get();
        *o = fc4_plumbing_state {
            struct_size: std::mem::size_of::<fc4_plumbing_state>() as u32,
            initialized: u8::from(s.initialized),
            grids_initialized: u8::from(s.grids_initialized),
            enabled: u8::from(s.enabled),
            recompute: u8::from(s.recompute),
            state: s.state,
            count_x: s.count_x,
            count_z: s.count_z,
            pass_counter: s.pass_counter,
            producers: s.producers.len() as u32,
            num_pipes: s.num_pipes,
            num_distressed: s.num_distressed,
            num_burst: s.num_burst,
            num_polluted: s.num_polluted,
            average_water_pollution: s.average_water_pollution,
            watered_cells: s.last_watered_cells,
            building_cells: s.last_building_cells,
            reentrant_calls: reentrant,
            reserved: 0,
            pipe_condition: s.pipe_condition,
            factor: s.factor,
            demand: s.demand,
            total_water_cost: s.total_water_cost,
            last_produced: s.last.produced,
            last_consumed: s.last.consumed,
            last_imported: s.last.imported,
            last_exported: s.last.exported,
            department: s.department.unwrap_or(0),
            working_grid: grid_token(&s.working),
            published_grid: grid_token(&s.published),
        };
        true
    })
}

/// `Write` (§9). Grid, occupant and department references go out as `fc4_ref`s
/// with the token and the class ID.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_write(p: *const fc4_plumbing, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    sim(p, false, |s| s.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

/// `Read` (§9): false only for a version other than 4.
#[no_mangle]
pub unsafe extern "C" fn fc4_plumbing_read(p: *mut fc4_plumbing, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    sim(p, false, |s| {
        s.read(&mut CIn {
            stream: i,
            decode: no_core_objects,
        })
    })
}
