// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The power simulator's C ABI (fc4.h "Power simulator", docs/specs/power.md).
//!
//! The host implements [`fc4_power_host`] (every `PowerHost` method of the
//! core, plus the game's grids through [`fc4_grid_api`]); the plugin's
//! `cISC4PowerSimulator` forwards its slots to the `fc4_power_*` functions.
//!
//! Tokens are opaque host identities (fc4.h): occupants (also poles, lines,
//! plants), lots, deals, connections, cities, departments, line items. 0 is
//! the null object.
//!
//! Re-entrancy: the core runs every call on its state taken out of the
//! simulator, so a host callback that calls back in would see an empty state.
//! [choice] A message delivered while a call is running is queued and handled
//! when the outermost call returns; other re-entrant calls are answered from
//! the (empty) running state and counted (`fc4_power_state::reentrant_calls`).

use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_void};
use std::rc::Rc;

use fc4_core::grid::{CellRect, GridProvider, TractGrid};
use fc4_core::power::{BudgetItem, Deal, LineInfo, PoleInfo, Posted, PowerHost, PowerMessage, PowerSim};
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
use crate::stream::{fc4_bytes_sink, fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

/// A message the simulator posts (fc4.h `fc4_power_post`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_power_post {
    pub msg_type: u32,
    pub data: [u32; 4],
    pub reserved: u32,
    /// The IGZUnknown (0 = none).
    pub unknown: u64,
}

/// `cISC4PowerPoleOccupant` facts (fc4.h `fc4_power_pole`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_power_pole {
    /// Pole slot 13 (nonzero = the simulator ignores the pole).
    pub ignored: u32,
    pub cell_x: i32,
    pub cell_z: i32,
}

/// `cISC4PowerLineOccupant` facts (fc4.h `fc4_power_line`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_power_line {
    pub preview: u32,
    pub cell_x: i32,
    pub cell_z: i32,
}

/// One deal of `FindDeals` (fc4.h `fc4_power_deal`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_power_deal {
    pub token: u64,
    pub city: u64,
    pub amount: i32,
    pub reserved: u32,
}

pub type fc4_power_deal_callback = Option<unsafe extern "C" fn(ctx: *mut c_void, deal: *const fc4_power_deal)>;

/// `GetBudgetItemForPurpose` (fc4.h `fc4_power_budget_item`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_power_budget_item {
    pub department: u32,
    pub line: u32,
    pub cost: i64,
    pub index: u32,
    pub local_funding: f32,
}

/// A message for `fc4_power_do_message` (fc4.h `fc4_power_message`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_power_message {
    pub msg_type: u32,
    pub data1: u32,
    pub data2: u32,
    pub data3: u32,
    pub data4: u32,
    pub reserved: u32,
    /// void1 / the IGZUnknown as a token (0 = null).
    pub void1: u64,
    /// The four int32 at void2 (zoning changed).
    pub rect: fc4_cell_rect,
}

/// `FindPowerPlants` (slot 30) results (fc4.h `fc4_power_found`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_power_found {
    pub has_lowest_funded: u32,
    pub lowest_funding: f32,
    /// With one reference for the caller.
    pub lowest_funded: u64,
    pub has_most_worn: u32,
    pub wear_percent: i32,
    /// With one reference for the caller.
    pub most_worn: u64,
}

/// Diagnostics (fc4.h `fc4_power_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_power_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub running: u8,
    pub enabled: u8,
    pub recompute: u8,
    pub state: u32,
    pub plant_count: i32,
    pub pole_count: u32,
    pub consumers: u32,
    pub powered: u32,
    pub unpowered: u32,
    pub requirement: u32,
    pub last_production: u32,
    pub last_consumption: u32,
    pub last_import: u32,
    pub last_export: u32,
    pub last_demand: u32,
    pub deficit: f32,
    pub reentrant_calls: u32,
    pub cost_base: i64,
    pub department: u64,
    pub grid: u64,
}

/// Everything the power simulator asks of the game (fc4.h `fc4_power_host`;
/// one callback per `PowerHost` method, in the trait's order).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_power_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    pub grids: fc4_grid_api,
    // city, tuning, ordinance, simulator
    pub cell_counts: Option<unsafe extern "C" fn(U, *mut i32, *mut i32)>,
    pub cell_widths: Option<unsafe extern "C" fn(U, *mut f32, *mut f32)>,
    /// A NEW fc4_props (the core destroys it), NULL if the exemplar is missing.
    pub tuning: Option<unsafe extern "C" fn(U) -> *mut fc4_props>,
    pub cumulative_factor: Option<unsafe extern "C" fn(U, u32, u32) -> f32>,
    pub sim_time: Option<unsafe extern "C" fn(U) -> u32>,
    pub acquire_services: Option<unsafe extern "C" fn(U)>,
    pub release_services: Option<unsafe extern "C" fn(U)>,
    pub add_agent: Option<unsafe extern "C" fn(U, u32, *const c_char, usize, u32)>,
    pub remove_agent: Option<unsafe extern "C" fn(U)>,
    pub subscribe: Option<unsafe extern "C" fn(U, *const u32, usize)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, *const u32, usize)>,
    pub post: Option<unsafe extern "C" fn(U, *const fc4_power_post)>,
    pub record_history: Option<unsafe extern "C" fn(U, u32, i32)>,
    pub set_history_queue_length: Option<unsafe extern "C" fn(U, u32, u32)>,
    // references
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    /// A reference read from a save (QI'd to `iid`, one reference) to the
    /// canonical token of the object (with one reference); 0 drops it.
    pub resolve_ref: Option<unsafe extern "C" fn(U, u64, u32) -> u64>,
    // occupants
    pub occupant_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    /// Fills `out` with property `id` (own or inherited); false if absent.
    pub occupant_property: Option<unsafe extern "C" fn(U, u64, u32, *mut fc4_props) -> bool>,
    /// `AddProperty(id, value)`: element type (`mem::*`), array flag, elements.
    pub set_occupant_property: Option<unsafe extern "C" fn(U, u64, u32, u16, bool, *const c_void, usize)>,
    pub occupant_position: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    pub occupant_bounding_box: Option<unsafe extern "C" fn(U, u64, *mut f32, *mut f32)>,
    pub occupant_bounding_cells: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    pub occupant_in_manager: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub building_type: Option<unsafe extern "C" fn(U, u64, *mut u32) -> bool>,
    pub power_pole: Option<unsafe extern "C" fn(U, u64, *mut fc4_power_pole) -> bool>,
    pub break_all_connections: Option<unsafe extern "C" fn(U, u64, bool)>,
    pub pole_is_distressed: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub set_pole_distressed: Option<unsafe extern "C" fn(U, u64, bool)>,
    pub power_line: Option<unsafe extern "C" fn(U, u64, *mut fc4_power_line) -> bool>,
    pub line_connected_poles: Option<unsafe extern "C" fn(U, u64, *mut u64, *mut u64) -> bool>,
    pub remove_pole_connection: Option<unsafe extern "C" fn(U, u64, u64)>,
    pub network_occupant_cell: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32) -> bool>,
    pub has_network_flag: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub network_flags: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub occupants_in_cells: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, fc4_token_callback, *mut c_void)>,
    pub remove_occupant: Option<unsafe extern "C" fn(U, u64)>,
    // zones, tools
    pub zone_type: Option<unsafe extern "C" fn(U, i32, i32) -> u32>,
    pub power_line_tool_zone_change: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, u32)>,
    // lots
    pub lot_at: Option<unsafe extern "C" fn(U, i32, i32) -> u64>,
    pub occupant_lot: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub as_lot: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub lot_bounding_rect: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    pub lot_state: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub lot_needs_power: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub lot_has_power: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub set_lot_has_power: Option<unsafe extern "C" fn(U, u64, bool)>,
    pub set_lot_condition: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub demolish_lots: Option<unsafe extern "C" fn(U, *const u64, usize, u32)>,
    pub add_radiation: Option<unsafe extern "C" fn(U, *const f32, f32, f32)>,
    pub current_view_input_control: Option<unsafe extern "C" fn(U, *mut u32) -> bool>,
    pub remove_current_view_input_control: Option<unsafe extern "C" fn(U)>,
    // neighbours
    pub find_deals: Option<unsafe extern "C" fn(U, u32, fc4_power_deal_callback, *mut c_void)>,
    pub deal_amount_used: Option<unsafe extern "C" fn(U, u64) -> i32>,
    pub set_deal_amount_used: Option<unsafe extern "C" fn(U, u64, i32)>,
    pub set_deal_flags: Option<unsafe extern "C" fn(U, u64, u32, bool)>,
    pub deal_type: Option<unsafe extern "C" fn(U, u64, *mut u32) -> bool>,
    pub mark_deals_dirty: Option<unsafe extern "C" fn(U)>,
    pub connections_of_type: Option<unsafe extern "C" fn(U, u32, fc4_token_callback, *mut c_void)>,
    pub connections_to_city: Option<unsafe extern "C" fn(U, u32, u64, fc4_token_callback, *mut c_void)>,
    pub find_unvisited_connection: Option<unsafe extern "C" fn(U, u32, u64) -> u64>,
    pub connection_on_other_side: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub connection_cell: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32)>,
    pub set_connection_visited: Option<unsafe extern "C" fn(U, u64, bool)>,
    pub connection_is_active: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub set_connection_active: Option<unsafe extern "C" fn(U, u64, bool)>,
    // budget
    pub department: Option<unsafe extern "C" fn(U, u32) -> u64>,
    pub create_department: Option<unsafe extern "C" fn(U, u32, u32) -> u64>,
    pub set_department_name: Option<unsafe extern "C" fn(U, u64, u32, u32)>,
    pub set_department_max_funding: Option<unsafe extern "C" fn(U, u64, f32)>,
    pub department_funding: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub budget_funding: Option<unsafe extern "C" fn(U, u64) -> i64>,
    pub create_line_item: Option<unsafe extern "C" fn(U, u64, u32, bool) -> u64>,
    pub create_line_item_for_building_type: Option<unsafe extern "C" fn(U, u64, u32, bool) -> u64>,
    pub line_item: Option<unsafe extern "C" fn(U, u64, u32) -> u64>,
    pub set_line_item_name: Option<unsafe extern "C" fn(U, u64, u32, u32)>,
    pub set_line_item_display_flag: Option<unsafe extern "C" fn(U, u64, u32, bool)>,
    pub line_item_secondary_info: Option<unsafe extern "C" fn(U, u64) -> i64>,
    pub set_line_item_secondary_info: Option<unsafe extern "C" fn(U, u64, i64)>,
    pub set_line_item_full_expenses: Option<unsafe extern "C" fn(U, u64, i64)>,
    pub budget_item_for_purpose: Option<unsafe extern "C" fn(U, u64, u32, *mut fc4_power_budget_item) -> bool>,
    pub change_budget_item_line: Option<unsafe extern "C" fn(U, u64, u32, u32)>,
    pub change_budget_item_cost: Option<unsafe extern "C" fn(U, u64, u32, i64)>,
    pub add_locally_funded_object: Option<unsafe extern "C" fn(U, u64, u64, u32)>,
    pub remove_locally_funded_object: Option<unsafe extern "C" fn(U, u64, u64, u32)>,
    pub is_locally_funded_object: Option<unsafe extern "C" fn(U, u64, u64, u32) -> bool>,
    pub set_local_funding_percent: Option<unsafe extern "C" fn(U, u64, u64, f32, u32) -> bool>,
    pub set_local_full_funding: Option<unsafe extern "C" fn(U, u64, u64, i64, u32)>,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_power_post>() == 32);
    assert!(std::mem::size_of::<fc4_power_pole>() == 12);
    assert!(std::mem::size_of::<fc4_power_line>() == 12);
    assert!(std::mem::size_of::<fc4_power_deal>() == 24);
    assert!(std::mem::size_of::<fc4_power_budget_item>() == 24);
    assert!(std::mem::size_of::<fc4_power_message>() == 48);
    assert!(std::mem::size_of::<fc4_power_found>() == 32);
    assert!(std::mem::size_of::<fc4_power_state>() == 88);
};

// ---- host adapter -----------------------------------------------------------------------------------

struct CHost(fc4_power_host);

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.0.$f { Some(f) => unsafe { f($s.0.user $(, $arg)*) }, None => $default }
    };
}

fn opt(t: u64) -> Option<u64> {
    (t != 0).then_some(t)
}

unsafe extern "C" fn push_deal(ctx: *mut c_void, d: *const fc4_power_deal) {
    if let (Some(v), Some(d)) = ((ctx as *mut Vec<Deal>).as_mut(), d.as_ref()) {
        v.push(Deal {
            token: d.token,
            city: d.city,
            amount: d.amount,
        });
    }
}

impl GridProvider<u8> for CHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        create_grid::<u8>(&self.0.grids, instance, tract_size, true)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u8>>> {
        resolve_grid::<u8>(&self.0.grids, r, true)
    }
}

impl PowerHost for CHost {
    fn cell_counts(&self) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        cb!(self, cell_counts, (), &mut x, &mut z);
        (x, z)
    }
    fn cell_widths(&self) -> (f32, f32) {
        let (mut x, mut z) = (0.0, 0.0);
        cb!(self, cell_widths, (), &mut x, &mut z);
        (x, z)
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, tuning, std::ptr::null_mut());
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }
    fn cumulative_factor(&self, id: u32, index: u32) -> f32 {
        cb!(self, cumulative_factor, 1.0, id, index)
    }
    fn sim_time(&self) -> u32 {
        cb!(self, sim_time, 0)
    }
    fn acquire_services(&self) {
        cb!(self, acquire_services, ())
    }
    fn release_services(&self) {
        cb!(self, release_services, ())
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
    fn post(&self, msg: &Posted) {
        let p = fc4_power_post {
            msg_type: msg.msg_type,
            data: msg.data,
            reserved: 0,
            unknown: msg.unknown.unwrap_or(0),
        };
        cb!(self, post, (), &p)
    }
    fn record_history(&self, series: u32, value: i32) {
        cb!(self, record_history, (), series, value)
    }
    fn set_history_queue_length(&self, series: u32, length: u32) {
        cb!(self, set_history_queue_length, (), series, length)
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
    fn occupant_in_manager(&self, occ: u64) -> bool {
        cb!(self, occupant_in_manager, false, occ)
    }
    fn building_type(&self, occ: u64) -> Option<u32> {
        let mut t = 0;
        cb!(self, building_type, false, occ, &mut t).then_some(t)
    }
    fn power_pole(&self, occ: u64) -> Option<PoleInfo> {
        let mut p = fc4_power_pole::default();
        cb!(self, power_pole, false, occ, &mut p).then_some(PoleInfo {
            ignored: p.ignored != 0,
            cell: (p.cell_x, p.cell_z),
        })
    }
    fn break_all_connections(&self, pole: u64, flag: bool) {
        cb!(self, break_all_connections, (), pole, flag)
    }
    fn pole_is_distressed(&self, pole: u64) -> bool {
        cb!(self, pole_is_distressed, false, pole)
    }
    fn set_pole_distressed(&self, pole: u64, on: bool) {
        cb!(self, set_pole_distressed, (), pole, on)
    }
    fn power_line(&self, occ: u64) -> Option<LineInfo> {
        let mut l = fc4_power_line::default();
        cb!(self, power_line, false, occ, &mut l).then_some(LineInfo {
            preview: l.preview != 0,
            cell: (l.cell_x, l.cell_z),
        })
    }
    fn line_connected_poles(&self, line: u64) -> Option<(u64, u64)> {
        let (mut a, mut b) = (0u64, 0u64);
        let ok = cb!(self, line_connected_poles, false, line, &mut a, &mut b);
        (ok && a != 0 && b != 0).then_some((a, b))
    }
    fn remove_pole_connection(&self, first: u64, second: u64) {
        cb!(self, remove_pole_connection, (), first, second)
    }
    fn network_occupant_cell(&self, occ: u64) -> Option<(i32, i32)> {
        let (mut x, mut z) = (0, 0);
        cb!(self, network_occupant_cell, false, occ, &mut x, &mut z).then_some((x, z))
    }
    fn has_network_flag(&self, occ: u64, flag: u32) -> bool {
        cb!(self, has_network_flag, false, occ, flag)
    }
    fn network_flags(&self, occ: u64) -> u32 {
        cb!(self, network_flags, 0, occ)
    }
    fn occupants_in_cells(&self, rect: CellRect) -> Vec<u64> {
        let r = fc4_cell_rect::from(rect);
        collect_tokens(|sink, ctx| cb!(self, occupants_in_cells, (), &r, sink, ctx))
    }
    fn remove_occupant(&self, occ: u64) {
        cb!(self, remove_occupant, (), occ)
    }
    fn zone_type(&self, x: i32, z: i32) -> u32 {
        cb!(self, zone_type, 0, x, z)
    }
    fn power_line_tool_zone_change(&self, rect: CellRect, data3: u32) {
        let r = fc4_cell_rect::from(rect);
        cb!(self, power_line_tool_zone_change, (), &r, data3)
    }
    fn lot_at(&self, x: i32, z: i32) -> Option<u64> {
        opt(cb!(self, lot_at, 0, x, z))
    }
    fn occupant_lot(&self, occ: u64) -> Option<u64> {
        opt(cb!(self, occupant_lot, 0, occ))
    }
    fn as_lot(&self, token: u64) -> Option<u64> {
        opt(cb!(self, as_lot, 0, token))
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        let mut r = fc4_cell_rect::default();
        cb!(self, lot_bounding_rect, (), lot, &mut r);
        r.into()
    }
    fn lot_state(&self, lot: u64) -> u32 {
        cb!(self, lot_state, 0, lot)
    }
    fn lot_needs_power(&self, lot: u64) -> bool {
        cb!(self, lot_needs_power, false, lot)
    }
    fn lot_has_power(&self, lot: u64) -> bool {
        cb!(self, lot_has_power, false, lot)
    }
    fn set_lot_has_power(&self, lot: u64, on: bool) {
        cb!(self, set_lot_has_power, (), lot, on)
    }
    fn set_lot_condition(&self, lot: u64, condition: u32) {
        cb!(self, set_lot_condition, (), lot, condition)
    }
    fn demolish_lots(&self, lots: &[u64], flags: u32) {
        cb!(self, demolish_lots, (), lots.as_ptr(), lots.len(), flags)
    }
    fn add_radiation(&self, position: [f32; 3], strength: f32, radius: f32) {
        cb!(self, add_radiation, (), position.as_ptr(), strength, radius)
    }
    fn current_view_input_control(&self) -> Option<u32> {
        let mut id = 0;
        cb!(self, current_view_input_control, false, &mut id).then_some(id)
    }
    fn remove_current_view_input_control(&self) {
        cb!(self, remove_current_view_input_control, ())
    }
    fn find_deals(&self, deal_type: u32) -> Vec<Deal> {
        let mut v: Vec<Deal> = Vec::new();
        cb!(
            self,
            find_deals,
            (),
            deal_type,
            Some(push_deal),
            &mut v as *mut Vec<Deal> as *mut c_void
        );
        v
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
    fn deal_type(&self, token: u64) -> Option<u32> {
        let mut t = 0;
        cb!(self, deal_type, false, token, &mut t).then_some(t)
    }
    fn mark_deals_dirty(&self) {
        cb!(self, mark_deals_dirty, ())
    }
    fn connections_of_type(&self, connection_type: u32) -> Vec<u64> {
        collect_tokens(|sink, ctx| cb!(self, connections_of_type, (), connection_type, sink, ctx))
    }
    fn connections_to_city(&self, connection_type: u32, city: u64) -> Vec<u64> {
        collect_tokens(|sink, ctx| cb!(self, connections_to_city, (), connection_type, city, sink, ctx))
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
    fn set_connection_visited(&self, conn: u64, on: bool) {
        cb!(self, set_connection_visited, (), conn, on)
    }
    fn connection_is_active(&self, conn: u64) -> bool {
        cb!(self, connection_is_active, false, conn)
    }
    fn set_connection_active(&self, conn: u64, on: bool) {
        cb!(self, set_connection_active, (), conn, on)
    }
    fn department(&self, id: u32) -> Option<u64> {
        opt(cb!(self, department, 0, id))
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
    fn department_funding(&self, dept: u64) -> f32 {
        cb!(self, department_funding, 1.0, dept)
    }
    fn budget_funding(&self, dept: u64) -> i64 {
        cb!(self, budget_funding, 0, dept)
    }
    fn create_line_item(&self, dept: u64, line: u32, locally_funded: bool) -> Option<u64> {
        opt(cb!(self, create_line_item, 0, dept, line, locally_funded))
    }
    fn create_line_item_for_building_type(&self, dept: u64, line: u32, flag: bool) -> Option<u64> {
        opt(cb!(self, create_line_item_for_building_type, 0, dept, line, flag))
    }
    fn line_item(&self, dept: u64, line: u32) -> Option<u64> {
        opt(cb!(self, line_item, 0, dept, line))
    }
    fn set_line_item_name(&self, item: u64, group: u32, instance: u32) {
        cb!(self, set_line_item_name, (), item, group, instance)
    }
    fn set_line_item_display_flag(&self, item: u64, flag: u32, on: bool) {
        cb!(self, set_line_item_display_flag, (), item, flag, on)
    }
    fn line_item_secondary_info(&self, item: u64) -> i64 {
        cb!(self, line_item_secondary_info, 0, item)
    }
    fn set_line_item_secondary_info(&self, item: u64, value: i64) {
        cb!(self, set_line_item_secondary_info, (), item, value)
    }
    fn set_line_item_full_expenses(&self, item: u64, value: i64) {
        cb!(self, set_line_item_full_expenses, (), item, value)
    }
    fn budget_item_for_purpose(&self, occ: u64, purpose: u32) -> Option<BudgetItem> {
        let mut b = fc4_power_budget_item::default();
        cb!(self, budget_item_for_purpose, false, occ, purpose, &mut b).then_some(BudgetItem {
            department: b.department,
            line: b.line,
            cost: b.cost,
            index: b.index,
            local_funding: b.local_funding,
        })
    }
    fn change_budget_item_line(&self, occ: u64, index: u32, line: u32) {
        cb!(self, change_budget_item_line, (), occ, index, line)
    }
    fn change_budget_item_cost(&self, occ: u64, index: u32, cost: i64) {
        cb!(self, change_budget_item_cost, (), occ, index, cost)
    }
    fn add_locally_funded_object(&self, dept: u64, occ: u64, purpose: u32) {
        cb!(self, add_locally_funded_object, (), dept, occ, purpose)
    }
    fn remove_locally_funded_object(&self, dept: u64, occ: u64, purpose: u32) {
        cb!(self, remove_locally_funded_object, (), dept, occ, purpose)
    }
    fn is_locally_funded_object(&self, dept: u64, occ: u64, purpose: u32) -> bool {
        cb!(self, is_locally_funded_object, false, dept, occ, purpose)
    }
    fn set_local_funding_percent(&self, dept: u64, occ: u64, fraction: f32, purpose: u32) -> bool {
        cb!(self, set_local_funding_percent, false, dept, occ, fraction, purpose)
    }
    fn set_local_full_funding(&self, dept: u64, occ: u64, amount: i64, purpose: u32) {
        cb!(self, set_local_full_funding, (), dept, occ, amount, purpose)
    }
}

// ---- handles ----------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_power`).
#[repr(C)]
pub struct fc4_power {
    _private: [u8; 0],
}

struct Handle {
    host: Rc<CHost>,
    sim: Rc<PowerSim>,
    depth: Cell<u32>,
    queue: RefCell<Vec<PowerMessage>>,
    reentrant: Cell<u32>,
}

impl Handle {
    /// Runs `f` with the call depth counted; at the end of the outermost
    /// call, the queued messages are handled.
    fn call<R>(&self, f: impl FnOnce(&PowerSim) -> R) -> R {
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
fn with_power<R>(p: *const fc4_power, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_power*.
    let Some(h) = (unsafe { (p as *const Handle).as_ref() }) else {
        return fallback;
    };
    guard(fallback, || f(h))
}

fn sim<R>(p: *const fc4_power, fallback: R, f: impl FnOnce(&PowerSim) -> R) -> R {
    with_power(p, fallback, |h| h.call(f))
}

/// Creates the core object (constructor values, §3). The host table is
/// copied. `token` is the host identity of the simulator itself.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_create(host: *const fc4_power_host, token: u64) -> *mut fc4_power {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_power_host>() && h.grids.complete() => {
            let host = Rc::new(CHost(*h));
            let sim = PowerSim::new(host.clone(), token);
            Box::into_raw(Box::new(Handle {
                host,
                sim,
                depth: Cell::new(0),
                queue: RefCell::new(Vec::new()),
                reentrant: Cell::new(0),
            })) as *mut fc4_power
        }
        _ => std::ptr::null_mut(),
    })
}

/// Destroys the core object. The powered grid it still holds is shut down
/// and released; other references still held (a loaded object that was never
/// initialised) are released too.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_destroy(p: *mut fc4_power) {
    if p.is_null() {
        return;
    }
    guard((), || {
        let b = Box::from_raw(p as *mut Handle);
        let (dept, plants, poles) = {
            let s = b.sim.state();
            (
                s.department,
                s.plants.iter().map(|n| n.occupant).collect::<Vec<_>>(),
                s.poles.clone(),
            )
        };
        // [choice] Shutdown releases these when the object was initialised;
        // otherwise (a read without Init) they are released here.
        if !b.sim.state().initialized {
            for t in dept.into_iter().chain(plants).chain(poles) {
                b.host.release(t);
            }
            b.sim.update_state(|s| {
                s.department = None;
                s.plants.clear();
                s.poles.clear();
            });
        }
        drop(b);
    });
}

/// `Init` (§4.1): always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_init(p: *mut fc4_power) -> bool {
    sim(p, false, PowerSim::init)
}

/// `Shutdown` (§4.2): always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_shutdown(p: *mut fc4_power) -> bool {
    sim(p, false, PowerSim::shutdown)
}

/// `DoMessage` (§4.3, §5): always true. A message delivered during another
/// call is queued (module documentation).
#[no_mangle]
pub unsafe extern "C" fn fc4_power_do_message(p: *mut fc4_power, m: *const fc4_power_message) -> bool {
    let Some(m) = m.as_ref() else { return true };
    let msg = PowerMessage {
        msg_type: m.msg_type,
        data1: m.data1,
        data2: m.data2,
        data3: m.data3,
        data4: m.data4,
        void1: m.void1,
        rect: m.rect.into(),
    };
    with_power(p, true, |h| {
        if h.depth.get() > 0 {
            h.queue.borrow_mut().push(msg);
            return true;
        }
        h.call(|s| s.do_message(&msg))
    })
}

/// The argument-free slots that return a number: 9, 11–15, 17, 21, 22, 24,
/// 26–29, 39, 41–44 (the value as the slot returns it, widened). 0 for any
/// other slot.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_slot_value(p: *const fc4_power, slot: u32) -> i64 {
    sim(p, 0, |s| match slot {
        9 => i64::from(s.annual_consumption()),
        11 => i64::from(s.monthly_requirement()),
        12 => i64::from(s.monthly_production()),
        13 => i64::from(s.monthly_import()),
        14 => i64::from(s.monthly_export()),
        15 => i64::from(s.monthly_demand()),
        17 => i64::from(s.monthly_consumption()),
        21 => i64::from(s.unpowered_consumer_count()),
        22 => i64::from(s.powered_consumer_count()),
        24 => i64::from(s.power_plant_count()),
        26 => i64::from(s.power_pole_count()),
        27 => i64::from(s.overworked_plant_count()),
        28 => i64::from(s.aged_plant_count()),
        29 => i64::from(s.distressed_plant_count()),
        39 => i64::from(s.is_on_strike()),
        41 => i64::from(s.begin_strike()),
        42 => i64::from(s.end_strike()),
        43 => i64::from(s.strike_start_date()),
        44 => i64::from(s.strike_length()),
        _ => 0,
    })
}

/// Slot 5 `PositionToCell`.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_position_to_cell(p: *const fc4_power, x: f32, z: f32, cx: *mut i32, cz: *mut i32) {
    let (a, b) = sim(p, (0, 0), |s| s.position_to_cell(x, z));
    if let Some(o) = cx.as_mut() {
        *o = a;
    }
    if let Some(o) = cz.as_mut() {
        *o = b;
    }
}

/// Slot 6 `CellToPosition`.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_cell_to_position(p: *const fc4_power, cx: i32, cz: i32, x: *mut f32, z: *mut f32) {
    let (a, b) = sim(p, (0.0, 0.0), |s| s.cell_to_position(cx, cz));
    if let Some(o) = x.as_mut() {
        *o = a;
    }
    if let Some(o) = z.as_mut() {
        *o = b;
    }
}

/// Slot 10 `GetAnnualConsumptionByPowerType`.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_annual_consumption_by_type(p: *const fc4_power, t: i32) -> i32 {
    sim(p, 0, |s| s.annual_consumption_by_type(t))
}

/// Slot 16 `GetMonthlyConsumption(x, z)` (sign-extended int16).
#[no_mangle]
pub unsafe extern "C" fn fc4_power_consumption_at(p: *const fc4_power, x: i32, z: i32) -> i32 {
    sim(p, 0, |s| s.monthly_consumption_at(x, z))
}

/// Slot 23 `GetPowerPlantCount(type)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_plant_count_of_type(p: *const fc4_power, t: i32) -> i32 {
    sim(p, 0, |s| s.power_plant_count_of_type(t))
}

/// Slot 25 `GetPowerPlants`: every plant in list order, each with one
/// reference for the caller. Returns the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_plants(p: *const fc4_power, cb: fc4_token_callback, ctx: *mut c_void) -> usize {
    let plants = sim(p, Vec::new(), PowerSim::power_plants);
    match cb {
        Some(f) => {
            for &t in &plants {
                f(ctx, t);
            }
        }
        // Nobody takes the references: give them back.
        None => with_power(p, (), |h| {
            for &t in &plants {
                h.host.release(t);
            }
        }),
    }
    plants.len()
}

/// Slot 30 `FindPowerPlants` (the outputs come with one reference each).
#[no_mangle]
pub unsafe extern "C" fn fc4_power_find_plants(p: *const fc4_power, out: *mut fc4_power_found) -> bool {
    let Some(o) = out.as_mut() else { return false };
    sim(p, false, |s| {
        let f = s.find_power_plants();
        *o = fc4_power_found::default();
        if let Some((t, funding)) = f.lowest_funded {
            o.has_lowest_funded = 1;
            o.lowest_funded = t;
            o.lowest_funding = funding;
        }
        if let Some((t, wear)) = f.most_worn {
            o.has_most_worn = 1;
            o.most_worn = t;
            o.wear_percent = wear;
        }
        true
    })
}

/// Slot 31 `IsOccupantPowered`.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_is_occupant_powered(p: *const fc4_power, occ: u64) -> bool {
    sim(p, false, |s| s.is_occupant_powered(occ))
}

/// Slot 32 `GetOccupantPowerConsumed`: false (out untouched) if not found.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_occupant_consumed(p: *const fc4_power, occ: u64, out: *mut u32) -> bool {
    match sim(p, None, |s| s.occupant_power_consumed(occ)) {
        Some(v) => {
            if let Some(o) = out.as_mut() {
                *o = v;
            }
            true
        }
        None => false,
    }
}

/// Slot 33 `GetOccupantPowerProduced`.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_occupant_produced(
    p: *const fc4_power,
    occ: u64,
    produced: *mut u32,
    used: *mut u32,
) -> bool {
    match sim(p, None, |s| s.occupant_power_produced(occ)) {
        Some((a, b)) => {
            if let Some(o) = produced.as_mut() {
                *o = a;
            }
            if let Some(o) = used.as_mut() {
                *o = b;
            }
            true
        }
        None => false,
    }
}

/// Slot 34 `GetOccupantMaxPowerProduced`.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_occupant_max_produced(p: *const fc4_power, occ: u64, out: *mut u32) -> bool {
    match sim(p, None, |s| s.occupant_max_power_produced(occ)) {
        Some(v) => {
            if let Some(o) = out.as_mut() {
                *o = v;
            }
            true
        }
        None => false,
    }
}

/// Slots 20, 35, 36: the powered grid's token (borrowed), 0 if none.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_powered_grid(p: *const fc4_power) -> u64 {
    sim(p, 0, |s| grid_token(&s.powered_grid()))
}

/// Slots 19 / 37 `IsPowered`.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_is_powered(p: *const fc4_power, x: i32, z: i32) -> bool {
    sim(p, false, |s| s.is_powered(x, z))
}

/// Slot 38 `IsWired`.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_is_wired(p: *const fc4_power, x: i32, z: i32) -> bool {
    sim(p, false, |s| s.is_wired(x, z))
}

/// Slot 40 `GetChanceOfStrike` (the fraction written).
#[no_mangle]
pub unsafe extern "C" fn fc4_power_chance_of_strike(p: *const fc4_power) -> f32 {
    sim(p, 0.0, PowerSim::chance_of_strike)
}

/// Slot 45 `GetCurrentPlantEfficiencyPercent`.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_plant_efficiency(p: *const fc4_power, occ: u64) -> f32 {
    sim(p, 0.0, |s| s.current_plant_efficiency(occ))
}

/// Slot 46 `GetCurrentPlantConditionPercent`.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_plant_condition(p: *const fc4_power, occ: u64) -> f32 {
    sim(p, 0.0, |s| s.current_plant_condition(occ))
}

/// Slot 47: the strike efficiency value (the host keeps the member it
/// returns the address of).
#[no_mangle]
pub unsafe extern "C" fn fc4_power_strike_efficiency(p: *const fc4_power) -> f32 {
    sim(p, 0.0, PowerSim::strike_efficiency)
}

/// The power department's token (borrowed; slot 48 forwards to its
/// `GetFundingPercentage`), 0 if none.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_department(p: *const fc4_power) -> u64 {
    sim(p, 0, |s| s.state().department.unwrap_or(0))
}

/// Slot 49 `GetMonthlyPowerGenerationCost`.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_generation_cost(p: *const fc4_power, occ: u64, out: *mut i64) -> bool {
    match sim(p, None, |s| s.monthly_generation_cost(occ)) {
        Some(v) => {
            if let Some(o) = out.as_mut() {
                *o = v;
            }
            true
        }
        None => false,
    }
}

/// Slot 50 `GetPowerPlantFundingPercentage` (a fraction).
#[no_mangle]
pub unsafe extern "C" fn fc4_power_plant_funding(p: *const fc4_power, occ: u64, out: *mut f32) -> bool {
    match sim(p, None, |s| s.plant_funding_percentage(occ)) {
        Some(v) => {
            if let Some(o) = out.as_mut() {
                *o = v;
            }
            true
        }
        None => false,
    }
}

/// Slot 51 `SetPowerPlantFundingPercentage` (a fraction).
#[no_mangle]
pub unsafe extern "C" fn fc4_power_set_plant_funding(p: *const fc4_power, occ: u64, fraction: f32) -> bool {
    sim(p, false, |s| s.set_plant_funding_percentage(occ, fraction))
}

/// Diagnostics; false for NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_get_state(p: *const fc4_power, out: *mut fc4_power_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_power(p, false, |h| {
        let st = h.sim.state();
        *o = fc4_power_state {
            struct_size: std::mem::size_of::<fc4_power_state>() as u32,
            initialized: st.initialized.into(),
            running: st.running.into(),
            enabled: st.enabled.into(),
            recompute: st.recompute.into(),
            state: st.state,
            plant_count: st.plant_count,
            pole_count: st.pole_count,
            consumers: st.consumers,
            powered: st.powered,
            unpowered: st.unpowered,
            requirement: st.requirement,
            last_production: st.last_production,
            last_consumption: st.last_consumption,
            last_import: st.last_import,
            last_export: st.last_export,
            last_demand: st.last_demand,
            deficit: st.deficit,
            reentrant_calls: h.reentrant.get(),
            cost_base: st.cost_base,
            department: st.department.unwrap_or(0),
            grid: 0,
        };
        drop(st);
        o.grid = grid_token(&h.sim.powered_grid());
        true
    })
}

/// Tooling: switches the recompute report on or off.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_set_report(p: *const fc4_power, on: bool) {
    with_power(p, (), |h| h.sim.set_report(on))
}

/// Tooling: the last complete recompute report as text, once; false when
/// there is none (or for NULL).
#[no_mangle]
pub unsafe extern "C" fn fc4_power_take_report(p: *const fc4_power, sink: fc4_bytes_sink, ctx: *mut c_void) -> bool {
    with_power(p, false, |h| match h.sim.take_report() {
        Some(r) => {
            let text = r.text();
            if let Some(f) = sink {
                // SAFETY: the caller's sink and context.
                unsafe { f(ctx, text.as_ptr(), text.len()) };
            }
            true
        }
        None => false,
    })
}

/// `Write` (§9). References go out as `fc4_ref`s with the token.
#[no_mangle]
pub unsafe extern "C" fn fc4_power_write(p: *const fc4_power, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    sim(p, false, |s| s.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

/// `Read` (§9): false only for a version above 6 (or a NULL argument).
#[no_mangle]
pub unsafe extern "C" fn fc4_power_read(p: *mut fc4_power, inp: *const fc4_istream) -> bool {
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
