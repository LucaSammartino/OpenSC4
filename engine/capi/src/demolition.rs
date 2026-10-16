// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The demolition utility over the C ABI (fc4.h "demolition utility",
//! docs/specs/demolition.md).
//!
//! The host implements [`fc4_demolition_host`]; the plugin's
//! `cISC4DemolitionUtility` (with its message-target and serializable
//! sub-objects) forwards its slots here. Tokens are opaque host identities:
//! occupants, lots, filters, the caller's occupant list and set. Tokens
//! handed to the core are borrowed; the city, occupant manager, lot
//! developer and terrain are held through the `acquire_*` / `release_*`
//! pairs. The two tuning exemplars are fetched through the host each time
//! the core reads them (an `fc4_props` the core takes over).
//!
//! Threads: the core is single-threaded (`RefCell`). The plugin serialises
//! every call on one object (fc4.h).

use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::demolition::{query_interface, DemolitionHost, DemolitionMessage, DemolitionUtility, Orphans, Outcome};
use fc4_core::grid::CellRect;
use fc4_core::property::PropertyHolder;
use fc4_core::stream::SaveObject;

use crate::budget::fc4_token_callback;
use crate::guard;
use crate::hostgrid::{collect_tokens, fc4_cell_rect, take_props};
use crate::props::fc4_props;
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};
use crate::zones::{fc4_zone_region, region_in};

type U = *mut c_void;
type Tokens = fc4_token_callback;

#[repr(C)]
pub struct fc4_demolition {
    _private: [u8; 0],
}

/// What the demolition utility needs from the game (fc4.h
/// `fc4_demolition_host`). A missing callback reads as 0 / false / none.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_demolition_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    // lifecycle
    pub subscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub acquire_city: Option<unsafe extern "C" fn(U) -> bool>,
    pub release_city: Option<unsafe extern "C" fn(U)>,
    pub city_cell_counts: Option<unsafe extern "C" fn(U, *mut i32, *mut i32)>,
    pub acquire_occupant_manager: Option<unsafe extern "C" fn(U) -> bool>,
    pub release_occupant_manager: Option<unsafe extern "C" fn(U)>,
    pub acquire_lot_developer: Option<unsafe extern "C" fn(U) -> bool>,
    pub release_lot_developer: Option<unsafe extern "C" fn(U)>,
    pub acquire_terrain: Option<unsafe extern "C" fn(U) -> bool>,
    pub release_terrain: Option<unsafe extern "C" fn(U)>,
    // tuning
    pub thresholds_exemplar: Option<unsafe extern "C" fn(U) -> *mut fc4_props>,
    pub building_development_exemplar: Option<unsafe extern "C" fn(U) -> *mut fc4_props>,
    // references
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    // the view and the filters
    pub acquire_view_filter: Option<unsafe extern "C" fn(U) -> u64>,
    pub view_level: Option<unsafe extern "C" fn(U, *mut u32) -> bool>,
    pub filter_includes: Option<unsafe extern "C" fn(U, u64, u64) -> bool>,
    pub filter_includes_type: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub default_filter_includes: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub realistic_filter_includes: Option<unsafe extern "C" fn(U, u64) -> bool>,
    // occupants
    pub occupant_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub occupant_position: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    pub can_remove: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub occupant_flag_set: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub occupant_flags: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub is_occupant_group: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub occupant_bool_property: Option<unsafe extern "C" fn(U, u64, u32, *mut bool) -> bool>,
    pub occupant_bounds: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    pub foundation_slot9: Option<unsafe extern "C" fn(U, u64, *mut u32) -> bool>,
    pub network_has_any_flag: Option<unsafe extern "C" fn(U, u64, u32, *mut bool) -> bool>,
    pub network_has_type: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub occupant_shutdown: Option<unsafe extern "C" fn(U, u64)>,
    pub remove_occupant: Option<unsafe extern "C" fn(U, u64, bool) -> bool>,
    pub occupants_in_box: Option<unsafe extern "C" fn(U, f32, f32, f32, f32, Tokens, *mut c_void)>,
    pub occupants_in_cell_rect: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, Tokens, *mut c_void)>,
    // prices and removals
    pub building_demolition_cost: Option<unsafe extern "C" fn(U, u64, *mut i64) -> bool>,
    pub flora_demolition_cost: Option<unsafe extern "C" fn(U, u64, *mut i64) -> bool>,
    pub is_network_occupant: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub network_demolition_cost: Option<unsafe extern "C" fn(U, u64) -> i64>,
    pub is_power_pole: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub pole_demolition_cost: Option<unsafe extern "C" fn(U, u64) -> i64>,
    pub pole_connections: Option<unsafe extern "C" fn(U, u64, Tokens, *mut c_void)>,
    pub pole_connection_count: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub pole_break_all_connections: Option<unsafe extern "C" fn(U, u64)>,
    pub send_pre_demolish: Option<unsafe extern "C" fn(U, u64)>,
    pub has_prop_manager: Option<unsafe extern "C" fn(U) -> bool>,
    pub remove_prop: Option<unsafe extern "C" fn(U, u64) -> bool>,
    // caller-owned containers
    pub list_append: Option<unsafe extern "C" fn(U, u64, u64)>,
    pub set_insert: Option<unsafe extern "C" fn(U, u64, u64)>,
    // networks
    pub queue_removals: Option<unsafe extern "C" fn(U, bool)>,
    pub delete_occupant_connection: Option<unsafe extern "C" fn(U, u64)>,
    pub destroy_network_occupants: Option<unsafe extern "C" fn(U, *const u64, usize)>,
    // city geometry
    pub cell_corner_to_position: Option<unsafe extern "C" fn(U, i32, i32, *mut f32, *mut f32)>,
    pub position_to_cell: Option<unsafe extern "C" fn(U, f32, f32, *mut i32, *mut i32) -> bool>,
    // lots
    pub lot_at: Option<unsafe extern "C" fn(U, i32, i32) -> u64>,
    pub lots_in_cell_rect: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, Tokens, *mut c_void)>,
    pub lot_building: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub lot_state: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub lot_demolishable: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub lot_occupants: Option<unsafe extern "C" fn(U, u64, Tokens, *mut c_void)>,
    pub lot_size: Option<unsafe extern "C" fn(U, u64, *mut u8, *mut u8)>,
    pub lot_bounding_rect: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    pub lot_facing: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub lot_zone_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub lot_water_compatible: Option<unsafe extern "C" fn(U, u64, *mut bool) -> bool>,
    pub lot_initialized: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub lot_vertices_locked: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub lot_condition: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub explode_lot: Option<unsafe extern "C" fn(U, u64, bool)>,
    pub start_demolish_lot: Option<unsafe extern "C" fn(U, u64)>,
    pub end_demolish_lot: Option<unsafe extern "C" fn(U, u64)>,
    pub delete_lot: Option<unsafe extern "C" fn(U, u64)>,
    pub create_lot_if_possible: Option<unsafe extern "C" fn(U, i32, i32, u32) -> bool>,
    pub zone_at: Option<unsafe extern "C" fn(U, i32, i32) -> u32>,
    pub dezone: Option<unsafe extern "C" fn(U, *const fc4_cell_rect)>,
    // terrain
    pub altitude_at_vertex: Option<unsafe extern "C" fn(U, i32, i32) -> f32>,
    pub is_water_vertex: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    pub set_altitude_rect: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, f32, u32)>,
    pub set_altitude_vertex: Option<unsafe extern "C" fn(U, i32, i32, f32)>,
    pub redisplay_terrain: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, u32)>,
    // effects
    pub effect_extent: Option<unsafe extern "C" fn(U, u64, *mut f32) -> bool>,
    pub play_occupant_sound: Option<unsafe extern "C" fn(U, u64)>,
    pub start_occupant_effect: Option<unsafe extern "C" fn(U, u64, *const u32, usize, bool, i32, i32, bool)>,
    pub start_group_effect: Option<unsafe extern "C" fn(U, u32, i32, i32)>,
}

/// Diagnostics (`fc4_demolition_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_demolition_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub destroyed: u8,
    pub filter_created: u8,
    pub city_held: u8,
    pub occupant_manager_held: u8,
    pub lot_developer_held: u8,
    pub terrain_held: u8,
    pub reserved: u8,
    pub ref_count: u32,
    pub cells_x: i32,
    pub cells_z: i32,
    /// The tuning (§6): small/medium and medium/large model sizes,
    /// small/medium and medium/large group sizes, the destroyed-tile cost.
    pub small_medium_model: u32,
    pub medium_large_model: u32,
    pub small_medium_group: u32,
    pub medium_large_group: u32,
    pub destroyed_tile_cost: u32,
}

/// Number of callbacks in [`fc4_demolition_host`].
const HOST_CALLBACKS: usize = 84;

const _: () = {
    assert!(std::mem::size_of::<fc4_demolition_state>() == 44);
    assert!(
        std::mem::size_of::<fc4_demolition_host>()
            == if cfg!(target_pointer_width = "32") {
                8 + 4 * HOST_CALLBACKS
            } else {
                16 + 8 * HOST_CALLBACKS
            }
    );
};

// ---- host adapter -----------------------------------------------------------------------------------

struct CHost {
    h: fc4_demolition_host,
}

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.h.$f { Some(f) => unsafe { f($s.h.user $(, $arg)*) }, None => $default }
    };
}

fn opt(t: u64) -> Option<u64> {
    (t != 0).then_some(t)
}

impl CHost {
    fn tokens(&self, f: impl FnOnce(U, Tokens, *mut c_void)) -> Vec<u64> {
        collect_tokens(|cb, ctx| f(self.h.user, cb, ctx))
    }
}

impl DemolitionHost for CHost {
    fn subscribe(&self, msg_type: u32) {
        cb!(self, subscribe, (), msg_type)
    }
    fn unsubscribe(&self, msg_type: u32) {
        cb!(self, unsubscribe, (), msg_type)
    }
    fn acquire_city(&self) -> bool {
        cb!(self, acquire_city, false)
    }
    fn release_city(&self) {
        cb!(self, release_city, ())
    }
    fn city_cell_counts(&self) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        cb!(self, city_cell_counts, (), &mut x, &mut z);
        (x, z)
    }
    fn acquire_occupant_manager(&self) -> bool {
        cb!(self, acquire_occupant_manager, false)
    }
    fn release_occupant_manager(&self) {
        cb!(self, release_occupant_manager, ())
    }
    fn acquire_lot_developer(&self) -> bool {
        cb!(self, acquire_lot_developer, false)
    }
    fn release_lot_developer(&self) {
        cb!(self, release_lot_developer, ())
    }
    fn acquire_terrain(&self) -> bool {
        cb!(self, acquire_terrain, false)
    }
    fn release_terrain(&self) {
        cb!(self, release_terrain, ())
    }
    fn thresholds_exemplar(&self) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, thresholds_exemplar, std::ptr::null_mut());
        // SAFETY: the host hands over an fc4_props it created (or NULL).
        unsafe { take_props(p) }
    }
    fn building_development_exemplar(&self) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, building_development_exemplar, std::ptr::null_mut());
        // SAFETY: as above.
        unsafe { take_props(p) }
    }
    fn release(&self, token: u64) {
        cb!(self, release, (), token)
    }
    fn acquire_view_filter(&self) -> Option<u64> {
        opt(cb!(self, acquire_view_filter, 0))
    }
    fn view_level(&self) -> Option<u32> {
        let mut l = 0u32;
        cb!(self, view_level, false, &mut l).then_some(l)
    }
    fn filter_includes(&self, filter: u64, occupant: u64) -> bool {
        cb!(self, filter_includes, false, filter, occupant)
    }
    fn filter_includes_type(&self, filter: u64, occupant_type: u32) -> bool {
        cb!(self, filter_includes_type, false, filter, occupant_type)
    }
    fn default_filter_includes(&self, occupant: u64) -> bool {
        cb!(self, default_filter_includes, false, occupant)
    }
    fn realistic_filter_includes(&self, occupant: u64) -> bool {
        cb!(self, realistic_filter_includes, false, occupant)
    }
    fn occupant_type(&self, occupant: u64) -> u32 {
        cb!(self, occupant_type, 0, occupant)
    }
    fn occupant_position(&self, occupant: u64) -> (f32, f32, f32) {
        let mut p = [0.0f32; 3];
        cb!(self, occupant_position, (), occupant, p.as_mut_ptr());
        (p[0], p[1], p[2])
    }
    fn can_remove(&self, occupant: u64, level: u32) -> bool {
        cb!(self, can_remove, false, occupant, level)
    }
    fn occupant_flag_set(&self, occupant: u64, flag: u32) -> bool {
        cb!(self, occupant_flag_set, false, occupant, flag)
    }
    fn occupant_flags(&self, occupant: u64) -> u32 {
        cb!(self, occupant_flags, 0, occupant)
    }
    fn is_occupant_group(&self, occupant: u64, group: u32) -> bool {
        cb!(self, is_occupant_group, false, occupant, group)
    }
    fn occupant_bool_property(&self, occupant: u64, id: u32) -> Option<bool> {
        let mut v = false;
        cb!(self, occupant_bool_property, false, occupant, id, &mut v).then_some(v)
    }
    fn occupant_bounds(&self, occupant: u64) -> (f32, f32, f32, f32) {
        let mut b = [0.0f32; 4];
        cb!(self, occupant_bounds, (), occupant, b.as_mut_ptr());
        (b[0], b[1], b[2], b[3])
    }
    fn foundation_slot9(&self, occupant: u64) -> Option<u32> {
        let mut v = 0u32;
        cb!(self, foundation_slot9, false, occupant, &mut v).then_some(v)
    }
    fn network_has_any_flag(&self, occupant: u64, mask: u32) -> Option<bool> {
        let mut v = false;
        cb!(self, network_has_any_flag, false, occupant, mask, &mut v).then_some(v)
    }
    fn network_has_type(&self, occupant: u64, t: u32) -> bool {
        cb!(self, network_has_type, false, occupant, t)
    }
    fn occupant_shutdown(&self, occupant: u64) {
        cb!(self, occupant_shutdown, (), occupant)
    }
    fn remove_occupant(&self, occupant: u64, b: bool) -> bool {
        cb!(self, remove_occupant, false, occupant, b)
    }
    fn occupants_in_box(&self, x0: f32, z0: f32, x1: f32, z1: f32) -> Vec<u64> {
        match self.h.occupants_in_box {
            // SAFETY: as cb!.
            Some(f) => self.tokens(|u, cb, ctx| unsafe { f(u, x0, z0, x1, z1, cb, ctx) }),
            None => Vec::new(),
        }
    }
    fn occupants_in_cell_rect(&self, rect: CellRect) -> Vec<u64> {
        let r = fc4_cell_rect::from(rect);
        match self.h.occupants_in_cell_rect {
            // SAFETY: as cb!.
            Some(f) => self.tokens(|u, cb, ctx| unsafe { f(u, &r, cb, ctx) }),
            None => Vec::new(),
        }
    }
    fn building_demolition_cost(&self, building: u64) -> Option<i64> {
        let mut c = 0i64;
        cb!(self, building_demolition_cost, false, building, &mut c).then_some(c)
    }
    fn flora_demolition_cost(&self, occupant: u64) -> Option<i64> {
        let mut c = 0i64;
        cb!(self, flora_demolition_cost, false, occupant, &mut c).then_some(c)
    }
    fn is_network_occupant(&self, occupant: u64) -> bool {
        cb!(self, is_network_occupant, false, occupant)
    }
    fn network_demolition_cost(&self, occupant: u64) -> i64 {
        cb!(self, network_demolition_cost, 0, occupant)
    }
    fn is_power_pole(&self, occupant: u64) -> bool {
        cb!(self, is_power_pole, false, occupant)
    }
    fn pole_demolition_cost(&self, pole: u64) -> i64 {
        cb!(self, pole_demolition_cost, 0, pole)
    }
    fn pole_connections(&self, pole: u64) -> Vec<u64> {
        match self.h.pole_connections {
            // SAFETY: as cb!.
            Some(f) => self.tokens(|u, cb, ctx| unsafe { f(u, pole, cb, ctx) }),
            None => Vec::new(),
        }
    }
    fn pole_connection_count(&self, pole: u64) -> u32 {
        cb!(self, pole_connection_count, 0, pole)
    }
    fn pole_break_all_connections(&self, pole: u64) {
        cb!(self, pole_break_all_connections, (), pole)
    }
    fn send_pre_demolish(&self, occupant: u64) {
        cb!(self, send_pre_demolish, (), occupant)
    }
    fn has_prop_manager(&self) -> bool {
        cb!(self, has_prop_manager, false)
    }
    fn remove_prop(&self, occupant: u64) -> bool {
        cb!(self, remove_prop, false, occupant)
    }
    fn list_append(&self, list: u64, occupant: u64) {
        cb!(self, list_append, (), list, occupant)
    }
    fn set_insert(&self, set: u64, occupant: u64) {
        cb!(self, set_insert, (), set, occupant)
    }
    fn queue_removals(&self, on: bool) {
        cb!(self, queue_removals, (), on)
    }
    fn delete_occupant_connection(&self, occupant: u64) {
        cb!(self, delete_occupant_connection, (), occupant)
    }
    fn destroy_network_occupants(&self, list: &[u64]) {
        cb!(self, destroy_network_occupants, (), list.as_ptr(), list.len())
    }
    fn cell_corner_to_position(&self, x: i32, z: i32) -> (f32, f32) {
        let (mut px, mut pz) = (0.0f32, 0.0f32);
        cb!(self, cell_corner_to_position, (), x, z, &mut px, &mut pz);
        (px, pz)
    }
    fn position_to_cell(&self, x: f32, z: f32) -> Option<(i32, i32)> {
        let (mut cx, mut cz) = (0, 0);
        cb!(self, position_to_cell, false, x, z, &mut cx, &mut cz).then_some((cx, cz))
    }
    fn lot_at(&self, x: i32, z: i32) -> Option<u64> {
        opt(cb!(self, lot_at, 0, x, z))
    }
    fn lots_in_cell_rect(&self, rect: CellRect) -> Vec<u64> {
        let r = fc4_cell_rect::from(rect);
        match self.h.lots_in_cell_rect {
            // SAFETY: as cb!.
            Some(f) => self.tokens(|u, cb, ctx| unsafe { f(u, &r, cb, ctx) }),
            None => Vec::new(),
        }
    }
    fn lot_building(&self, lot: u64) -> Option<u64> {
        opt(cb!(self, lot_building, 0, lot))
    }
    fn lot_state(&self, lot: u64) -> u32 {
        cb!(self, lot_state, 0, lot)
    }
    fn lot_demolishable(&self, lot: u64) -> bool {
        cb!(self, lot_demolishable, false, lot)
    }
    fn lot_occupants(&self, lot: u64) -> Vec<u64> {
        match self.h.lot_occupants {
            // SAFETY: as cb!.
            Some(f) => self.tokens(|u, cb, ctx| unsafe { f(u, lot, cb, ctx) }),
            None => Vec::new(),
        }
    }
    fn lot_size(&self, lot: u64) -> (u8, u8) {
        let (mut x, mut z) = (0u8, 0u8);
        cb!(self, lot_size, (), lot, &mut x, &mut z);
        (x, z)
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        let mut r = fc4_cell_rect::default();
        cb!(self, lot_bounding_rect, (), lot, &mut r);
        r.into()
    }
    fn lot_facing(&self, lot: u64) -> u32 {
        cb!(self, lot_facing, 0, lot)
    }
    fn lot_zone_type(&self, lot: u64) -> u32 {
        cb!(self, lot_zone_type, 0, lot)
    }
    fn lot_water_compatible(&self, lot: u64) -> Option<bool> {
        let mut v = false;
        cb!(self, lot_water_compatible, false, lot, &mut v).then_some(v)
    }
    fn lot_initialized(&self, lot: u64) -> bool {
        cb!(self, lot_initialized, false, lot)
    }
    fn lot_vertices_locked(&self, lot: u64) -> bool {
        cb!(self, lot_vertices_locked, false, lot)
    }
    fn lot_condition(&self, lot: u64) -> u32 {
        cb!(self, lot_condition, 0, lot)
    }
    fn explode_lot(&self, lot: u64, b: bool) {
        cb!(self, explode_lot, (), lot, b)
    }
    fn start_demolish_lot(&self, lot: u64) {
        cb!(self, start_demolish_lot, (), lot)
    }
    fn end_demolish_lot(&self, lot: u64) {
        cb!(self, end_demolish_lot, (), lot)
    }
    fn delete_lot(&self, lot: u64) {
        cb!(self, delete_lot, (), lot)
    }
    fn create_lot_if_possible(&self, x: i32, z: i32, facing: u32) -> bool {
        cb!(self, create_lot_if_possible, false, x, z, facing)
    }
    fn zone_at(&self, x: i32, z: i32) -> u32 {
        cb!(self, zone_at, 0, x, z)
    }
    fn dezone(&self, rect: CellRect) {
        let r = fc4_cell_rect::from(rect);
        cb!(self, dezone, (), &r)
    }
    fn altitude_at_vertex(&self, x: i32, z: i32) -> f32 {
        cb!(self, altitude_at_vertex, 0.0, x, z)
    }
    fn is_water_vertex(&self, x: i32, z: i32) -> bool {
        cb!(self, is_water_vertex, false, x, z)
    }
    fn set_altitude_rect(&self, rect: CellRect, h: f32, reason: u32) {
        let r = fc4_cell_rect::from(rect);
        cb!(self, set_altitude_rect, (), &r, h, reason)
    }
    fn set_altitude_vertex(&self, x: i32, z: i32, h: f32) {
        cb!(self, set_altitude_vertex, (), x, z, h)
    }
    fn redisplay_terrain(&self, rect: CellRect, reason: u32) {
        let r = fc4_cell_rect::from(rect);
        cb!(self, redisplay_terrain, (), &r, reason)
    }
    fn effect_extent(&self, occupant: u64) -> Option<f32> {
        let mut s = 0.0f32;
        cb!(self, effect_extent, false, occupant, &mut s).then_some(s)
    }
    fn play_occupant_sound(&self, occupant: u64) {
        cb!(self, play_occupant_sound, (), occupant)
    }
    fn start_occupant_effect(&self, occupant: u64, keys: &[u32], ray: Option<(i32, i32)>, grey: bool) {
        let (has, rx, ry) = ray.map_or((false, 0, 0), |(x, y)| (true, x, y));
        cb!(
            self,
            start_occupant_effect,
            (),
            occupant,
            keys.as_ptr(),
            keys.len(),
            has,
            rx,
            ry,
            grey
        )
    }
    fn start_group_effect(&self, instance: u32, x: i32, z: i32) {
        cb!(self, start_group_effect, (), instance, x, z)
    }
}

// ---- handles ----------------------------------------------------------------------------------------

struct Handle {
    sim: Rc<DemolitionUtility>,
}

fn with_p<R>(p: *const fc4_demolition, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_demolition*.
    match unsafe { (p as *const Handle).as_ref() } {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

/// Writes an outcome's cost (when the core wrote one) and returns its bool.
unsafe fn outcome(o: Outcome, cost: *mut i64) -> bool {
    if let (Some(c), false) = (o.cost, cost.is_null()) {
        *cost = c;
    }
    o.ok
}

/// Creates the core object (constructed, reference count 0). The host table
/// is copied. `token` is the utility's own token.
#[no_mangle]
pub unsafe extern "C" fn fc4_demolition_create(host: *const fc4_demolition_host, token: u64) -> *mut fc4_demolition {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_demolition_host>() => {
            let c = Rc::new(CHost { h: *h });
            let sim = DemolitionUtility::new(c, token);
            Box::into_raw(Box::new(Handle { sim })) as *mut fc4_demolition
        }
        _ => std::ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_demolition_destroy(p: *mut fc4_demolition) {
    if !p.is_null() {
        guard((), || drop(Box::from_raw(p as *mut Handle)));
    }
}

/// `QueryInterface` (§1): true and the sub-object offset (0, 4 or 8) in
/// `offset` (may be NULL) for the four IIDs it answers.
#[no_mangle]
pub unsafe extern "C" fn fc4_demolition_query_interface(iid: u32, offset: *mut u32) -> bool {
    match query_interface(iid) {
        Some(o) => {
            if !offset.is_null() {
                *offset = o;
            }
            true
        }
        None => false,
    }
}

#[no_mangle]
pub unsafe extern "C" fn fc4_demolition_add_ref(p: *mut fc4_demolition) -> u32 {
    with_p(p, 0, |h| h.sim.add_ref())
}

/// `Release`: 0 when the object was destroyed (`Shutdown` ran); the handle
/// stays valid until [`fc4_demolition_destroy`].
#[no_mangle]
pub unsafe extern "C" fn fc4_demolition_release(p: *mut fc4_demolition) -> u32 {
    with_p(p, 0, |h| h.sim.release())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_demolition_init(p: *mut fc4_demolition) -> bool {
    with_p(p, false, |h| h.sim.init())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_demolition_shutdown(p: *mut fc4_demolition) -> bool {
    with_p(p, false, |h| h.sim.shutdown())
}

/// `DoMessage` (§3): `0xC53D10AA` re-reads the tuning; `0xA9DED179` (the
/// record's `lot`, holding one reference, and `dezone`) finishes a deferred
/// lot demolition. Always true.
#[no_mangle]
pub unsafe extern "C" fn fc4_demolition_do_message(
    p: *mut fc4_demolition,
    msg_type: u32,
    data1: u32,
    data2: u32,
    lot: u64,
    dezone: bool,
) -> bool {
    let m = DemolitionMessage {
        msg_type,
        data1,
        data2,
        lot: opt(lot),
        dezone,
    };
    with_p(p, true, |h| h.sim.do_message(&m))
}

/// `DemolishOccupant` (§4.3). `orphans` = the caller's occupant list (0 =
/// none). `*cost` is written only when the core writes it (`cost` may be
/// NULL).
#[no_mangle]
pub unsafe extern "C" fn fc4_demolition_demolish_occupant(
    p: *mut fc4_demolition,
    do_it: bool,
    occupant: u64,
    privilege: u32,
    flags: u32,
    skip_network_update: bool,
    orphans: u64,
    cost: *mut i64,
) -> bool {
    with_p(p, false, |h| {
        let mut o = match opt(orphans) {
            Some(l) => Orphans::Host(l),
            None => Orphans::None,
        };
        outcome(
            h.sim
                .demolish_occupant(do_it, occupant, privilege, flags, skip_network_update, &mut o),
            cost,
        )
    })
}

/// `DemolishRegion` (§4.4). `caller_filter`, `demolished` (the caller's
/// set) and `clicked` are tokens (0 = none); `sx`, `sy` the clicked screen
/// point. False for a NULL or oversized region.
#[no_mangle]
pub unsafe extern "C" fn fc4_demolition_demolish_region(
    p: *mut fc4_demolition,
    do_it: bool,
    region: *const fc4_zone_region,
    privilege: u32,
    flags: u32,
    dezone: bool,
    caller_filter: u64,
    demolished: u64,
    clicked: u64,
    sx: i32,
    sy: i32,
    cost: *mut i64,
) -> bool {
    let Some(r) = region.as_ref() else { return false };
    let Some(region) = region_in(r) else { return false };
    with_p(p, false, |h| {
        outcome(
            h.sim.demolish_region(
                do_it,
                &region,
                privilege,
                flags,
                dezone,
                opt(caller_filter),
                opt(demolished),
                opt(clicked),
                sx,
                sy,
            ),
            cost,
        )
    })
}

/// `DemolishLots` (§4.5): `count` lot tokens (0 = a null entry; `lots` may
/// be NULL when `count` is 0).
#[no_mangle]
pub unsafe extern "C" fn fc4_demolition_demolish_lots(
    p: *mut fc4_demolition,
    do_it: bool,
    lots: *const u64,
    count: usize,
    privilege: u32,
    flags: u32,
    dezone: bool,
    caller_filter: u64,
    demolished: u64,
    cost: *mut i64,
) -> bool {
    let v: Vec<Option<u64>> = if lots.is_null() || count == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(lots, count)
            .iter()
            .map(|&t| opt(t))
            .collect()
    };
    with_p(p, false, |h| {
        outcome(
            h.sim
                .demolish_lots(do_it, &v, privilege, flags, dezone, opt(caller_filter), opt(demolished)),
            cost,
        )
    })
}

/// `ModifyTerrainHeight` (§4.7): the number of vertices changed (or that
/// would change). `exempt` = a lot token (0 = none).
#[no_mangle]
pub unsafe extern "C" fn fc4_demolition_modify_terrain_height(
    p: *mut fc4_demolition,
    do_it: bool,
    per_vertex: bool,
    x0: i32,
    z0: i32,
    x1: i32,
    z1: i32,
    altitude: f32,
    exempt: u64,
    reason: u32,
) -> i32 {
    with_p(p, 0, |h| {
        h.sim
            .modify_terrain_height(do_it, per_vertex, x0, z0, x1, z1, altitude, opt(exempt), reason)
    })
}

/// `UpdateNetworks(set)` (§4.6) over `count` occupant tokens.
#[no_mangle]
pub unsafe extern "C" fn fc4_demolition_update_networks(p: *mut fc4_demolition, set: *const u64, count: usize) {
    let v: Vec<u64> = if set.is_null() {
        Vec::new()
    } else {
        std::slice::from_raw_parts(set, count).to_vec()
    };
    with_p(p, (), |h| h.sim.update_networks(&v))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_demolition_get_state(p: *const fc4_demolition, out: *mut fc4_demolition_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_p(p, false, |h| {
        let s = h.sim.snapshot();
        *o = fc4_demolition_state {
            struct_size: std::mem::size_of::<fc4_demolition_state>() as u32,
            initialized: s.initialized.into(),
            destroyed: s.destroyed.into(),
            filter_created: s.filter_created.into(),
            city_held: s.city_held.into(),
            occupant_manager_held: s.occupant_manager_held.into(),
            lot_developer_held: s.lot_developer_held.into(),
            terrain_held: s.terrain_held.into(),
            reserved: 0,
            ref_count: s.ref_count,
            cells_x: s.cells_x,
            cells_z: s.cells_z,
            small_medium_model: s.tuning.small_medium_model,
            medium_large_model: s.tuning.medium_large_model,
            small_medium_group: s.tuning.small_medium_group,
            medium_large_group: s.tuning.medium_large_group,
            destroyed_tile_cost: s.tuning.destroyed_tile_cost,
        };
        true
    })
}

/// `Write` (§7): the version alone.
#[no_mangle]
pub unsafe extern "C" fn fc4_demolition_write(p: *const fc4_demolition, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_p(p, false, |h| h.sim.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

/// `Read` (§7): false unless the version is 1.
#[no_mangle]
pub unsafe extern "C" fn fc4_demolition_read(p: *mut fc4_demolition, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_p(p, false, |h| {
        h.sim.read(&mut CIn {
            stream: i,
            decode: no_core_objects,
        })
    })
}
