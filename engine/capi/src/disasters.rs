// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The disaster layer over the C ABI (fc4.h "disaster layer",
//! docs/specs/disasters.md).
//!
//! The host implements [`fc4_disasters_host`]; the plugin's
//! `cISC4DisasterLayer` and the managers and instances behind it forward
//! their slots here. Managers are named by their type (`GetDisasterType`);
//! instances by the core's IDs (from 1, never reused), which the host uses
//! for its own records (tick registrations, message subscriptions, the
//! toxic occupants' instance). The tuning exemplars are handed over with
//! [`fc4_disasters_set_tuning`] before the core first reads them (fire and
//! riot at their first instance, the short forms at theirs).
//!
//! The fire runs on the wall clock: the host's `now_ms` is read at every
//! update; [`fc4_disasters_tick`] passes the animation tick (µs).
//!
//! Tokens are opaque host identities: occupants, lots, fire and toxic
//! occupants, riot leaders (automata), pipes and effects. The core holds a
//! reference (`add_ref`) on every occupant it keeps in a location or the
//! fire's toast list. A missing callback reads as 0 / false / none.
//!
//! Re-entrancy: a host call made from inside a callback (for example an
//! `OccupantRemoved` message sent while the core demolishes a lot) is
//! deferred until the outer call returns when it is a message, and refused
//! (the fallback value) otherwise; `reentrant_refused` counts the refusals.
//!
//! Threads: the core is single-threaded (`RefCell`). The plugin serialises
//! every call on one object (fc4.h).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::{c_char, c_void, CString};
use std::rc::Rc;

use fc4_core::disasters::{CellRect, DisasterHost, DisasterMessage, DisasterPost, Disasters, Target, WorldBox};
use fc4_core::property::PropertyHolder;

use crate::budget::fc4_token_callback;
use crate::guard;
use crate::hostgrid::fc4_cell_rect;
use crate::props::{fc4_props, props_ref};

type U = *mut c_void;

#[repr(C)]
pub struct fc4_disasters {
    _private: [u8; 0],
}

/// A message the core posts (fc4.h `fc4_disaster_post`, disasters.md §12).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_disaster_post {
    pub msg_type: u32,
    pub data2: u32,
    /// May carry a host token.
    pub data1: u64,
    pub data3: u32,
    pub data4: u32,
    /// The instance (`0xCA1B1E64`; 0 = none).
    pub void1: u32,
    pub reserved: u32,
}

/// A message to the layer, a manager or an instance (fc4.h
/// `fc4_disaster_message`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_disaster_message {
    pub msg_type: u32,
    pub reserved: u32,
    pub data1: i64,
    pub data2: i64,
    pub data3: i64,
    /// `GetVoid1()` as a host token.
    pub void1: u64,
    /// A position in world units (the effect moved).
    pub x: f32,
    pub z: f32,
}

/// A world-space box (fc4.h `fc4_world_box`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_world_box {
    pub x0: f32,
    pub z0: f32,
    pub x1: f32,
    pub z1: f32,
}

/// Message targets (`subscribe`, `unsubscribe`, `fc4_disasters_do_message`).
pub const FC4_DISASTER_TARGET_LAYER: u32 = 0;
pub const FC4_DISASTER_TARGET_MANAGER: u32 = 1;
pub const FC4_DISASTER_TARGET_INSTANCE: u32 = 2;

/// What the disaster layer needs from the game (fc4.h
/// `fc4_disasters_host`), in the order of `DisasterHost`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_disasters_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    // clock, pause, save
    pub now_ms: Option<unsafe extern "C" fn(U) -> u64>,
    pub emergency_pause: Option<unsafe extern "C" fn(U)>,
    pub emergency_resume: Option<unsafe extern "C" fn(U)>,
    pub disable_save: Option<unsafe extern "C" fn(U)>,
    pub enable_save: Option<unsafe extern "C" fn(U)>,
    pub sim_paused: Option<unsafe extern "C" fn(U) -> bool>,
    pub sim_emergency_paused: Option<unsafe extern "C" fn(U) -> bool>,
    pub city_established: Option<unsafe extern "C" fn(U) -> bool>,
    pub city_cells: Option<unsafe extern "C" fn(U, *mut u32, *mut u32)>,
    pub cell_width: Option<unsafe extern "C" fn(U) -> f32>,
    // messages, ticks, history, references
    pub post: Option<unsafe extern "C" fn(U, *const fc4_disaster_post)>,
    pub subscribe: Option<unsafe extern "C" fn(U, u32, u32, u32)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, u32, u32, u32)>,
    pub register_tick: Option<unsafe extern "C" fn(U, u32, u32)>,
    pub unregister_tick: Option<unsafe extern "C" fn(U, u32)>,
    pub record_history: Option<unsafe extern "C" fn(U, u32, i32)>,
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    // the camera and the dispatch effectiveness
    pub follow_disasters: Option<unsafe extern "C" fn(U) -> bool>,
    pub go_to_location: Option<unsafe extern "C" fn(U, i32, i32, u32)>,
    pub set_fire_dispatch_effectiveness: Option<unsafe extern "C" fn(U, f32)>,
    pub set_police_dispatch_effectiveness: Option<unsafe extern "C" fn(U, f32)>,
    // occupants
    pub building_type_flags: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub network_default_cost: Option<unsafe extern "C" fn(U, u64) -> i64>,
    pub occupant_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub occupant_in_manager: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub occupant_cell: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32) -> bool>,
    pub occupant_cell_rect: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    pub occupant_position: Option<unsafe extern "C" fn(U, u64, *mut f32, *mut f32)>,
    pub occupant_bbox: Option<unsafe extern "C" fn(U, u64, *mut fc4_world_box)>,
    pub occupant_bbox_areas: Option<unsafe extern "C" fn(U, u64, *mut f32, *mut f32)>,
    pub occupant_prop_u32: Option<unsafe extern "C" fn(U, u64, u32, *mut u32) -> bool>,
    pub occupant_prop_i64: Option<unsafe extern "C" fn(U, u64, u32, *mut i64) -> bool>,
    pub occupant_prop_f32: Option<unsafe extern "C" fn(U, u64, u32, *mut f32) -> bool>,
    pub occupant_prop_bool: Option<unsafe extern "C" fn(U, u64, u32, *mut bool) -> bool>,
    pub occupants_in_rect: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, fc4_token_callback, *mut c_void)>,
    pub remove_occupant: Option<unsafe extern "C" fn(U, u64)>,
    pub occupant_lot: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub set_lot_condition: Option<unsafe extern "C" fn(U, u64, u32)>,
    // fire
    pub fire_coverage: Option<unsafe extern "C" fn(U, u64) -> u8>,
    pub flammability_at: Option<unsafe extern "C" fn(U, i32, i32) -> u8>,
    pub create_fire_occupant: Option<unsafe extern "C" fn(U, u32, u64, u32, u32, u32) -> u64>,
    pub remove_fire_occupant: Option<unsafe extern "C" fn(U, u64, u64)>,
    pub add_fire_aura: Option<unsafe extern "C" fn(U, u64)>,
    pub tutorial_no_burn_down: Option<unsafe extern "C" fn(U) -> bool>,
    pub wind: Option<unsafe extern "C" fn(U) -> f32>,
    pub play_effect: Option<unsafe extern "C" fn(U, *const c_char, u64)>,
    pub play_effect_at: Option<unsafe extern "C" fn(U, *const c_char, f32, f32)>,
    pub burn_flora: Option<unsafe extern "C" fn(U, u64)>,
    pub demolish_lot: Option<unsafe extern "C" fn(U, u64, u64, u32)>,
    pub spew_radiation: Option<unsafe extern "C" fn(U, u64, f32, f32)>,
    // riot
    pub police_coverage: Option<unsafe extern "C" fn(U, i32, i32) -> u8>,
    pub is_road_cell: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    pub cell_corner_position: Option<unsafe extern "C" fn(U, i32, i32, *mut f32, *mut f32)>,
    pub create_riot_leader: Option<unsafe extern "C" fn(U, f32, f32) -> u64>,
    /// The network occupant (road, rail, ...) at a cell (0 = none).
    pub network_occupant_at: Option<unsafe extern "C" fn(U, i32, i32) -> u64>,
    pub unset_removal_flags: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub set_removal_flags: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub automaton_cell: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32) -> bool>,
    pub set_riot_effect: Option<unsafe extern "C" fn(U, u64, i32)>,
    pub send_riot_message: Option<unsafe extern "C" fn(U, u64, i32)>,
    pub end_riot_leader: Option<unsafe extern "C" fn(U, u64)>,
    pub riot_leader_tick: Option<unsafe extern "C" fn(U, u64)>,
    // toxic spill
    pub create_toxic_occupant: Option<unsafe extern "C" fn(U, u32, i32, i32, i32) -> u64>,
    pub set_toxic_amount: Option<unsafe extern "C" fn(U, u64, i32)>,
    pub remove_toxic_occupant: Option<unsafe extern "C" fn(U, u64)>,
    // terrain (earthquake)
    pub apply_terrain_brush: Option<unsafe extern "C" fn(U, u32, f32, f32, f32, f32)>,
    pub redraw_terrain: Option<unsafe extern "C" fn(U, *const fc4_cell_rect)>,
    pub end_long_terrain_operation: Option<unsafe extern "C" fn(U)>,
    pub stop_terrain_texture_updates: Option<unsafe extern "C" fn(U)>,
    pub place_repulsor: Option<unsafe extern "C" fn(U, f32, f32)>,
    // pipe burst
    pub find_watered_pipe: Option<unsafe extern "C" fn(U, i32, i32) -> u64>,
    pub pipe_cell: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32)>,
    pub view_level: Option<unsafe extern "C" fn(U) -> u32>,
    pub create_leak_effect: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub connected_pipe_cells: Option<unsafe extern "C" fn(U, u64, *mut i32, usize) -> usize>,
    pub demolish_pipe: Option<unsafe extern "C" fn(U, u64, u32)>,
    // volcano
    pub volcano_start_deformation: Option<unsafe extern "C" fn(U, u32, f32, f32)>,
    pub volcano_end_deformation: Option<unsafe extern "C" fn(U, u32)>,
    pub volcano_start_lava: Option<unsafe extern "C" fn(U, u32, f32, f32, f32)>,
    pub volcano_lava_rates: Option<unsafe extern "C" fn(U, u32, f32, f32)>,
    pub volcano_end_lava: Option<unsafe extern "C" fn(U, u32)>,
}

/// Diagnostics (`fc4_disasters_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_disasters_state {
    pub struct_size: u32,
    pub initialised: u8,
    pub enabled: u8,
    pub assessing: u8,
    pub reserved: u8,
    pub disable_counter: u32,
    pub ongoing: u32,
    pub location_count: u32,
    /// The fire instance (0 = none yet).
    pub fire_instance: u32,
    pub fire_phase: u32,
    pub fire_count: u32,
    /// The toxic instance (0 = none).
    pub toxic_instance: u32,
    pub toxic_spills: u32,
    pub reentrant_refused: u32,
    pub fire_timers: u32,
    pub police_timers: u32,
    pub fire_effectiveness: f32,
    pub police_effectiveness: f32,
    pub damaged_buildings: u32,
    /// How many network occupants the riots hold locked (network-manager.md §10).
    pub riot_locked: u32,
    pub disaster_cost: i64,
}

/// One fire (`fc4_disasters_get_fire`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_disaster_fire {
    pub occupant: u64,
    pub target: u64,
    pub counter: i32,
    pub stage: u32,
    pub max_stage: u32,
    pub fire_id: u32,
    pub hosed: u32,
    pub top_flames: u32,
    pub side_flames: u32,
    /// 1 when the fire is being put out (`extinguish_at` is valid).
    pub extinguishing: u32,
    /// The extinguish time on the fire instance's timer (ms).
    pub extinguish_at: u64,
}

/// An instance (`fc4_disasters_get_instance`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_disaster_instance {
    pub manager_type: u32,
    pub class_id: u32,
    pub phase: u32,
    pub tick_registered: u8,
    /// `GetLocation` answers (x, z valid).
    pub has_location: u8,
    pub reserved: u16,
    pub x: i32,
    pub z: i32,
}

/// One riot leader (`fc4_disasters_riot_leaders`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_riot_leader {
    pub automaton: u64,
    pub counter: i32,
    pub stage: i32,
    pub max_stage: i32,
    pub x: i32,
    pub z: i32,
    pub reserved: u32,
}

const CALLBACKS: usize = 82;

const _: () = {
    assert!(std::mem::size_of::<fc4_disaster_post>() == 32);
    assert!(std::mem::size_of::<fc4_disaster_message>() == 48);
    assert!(std::mem::size_of::<fc4_world_box>() == 16);
    assert!(std::mem::size_of::<fc4_disasters_state>() == 80);
    assert!(std::mem::size_of::<fc4_disaster_fire>() == 56);
    assert!(std::mem::size_of::<fc4_disaster_instance>() == 24);
    assert!(std::mem::size_of::<fc4_riot_leader>() == 32);
    assert!(
        std::mem::size_of::<fc4_disasters_host>()
            == if cfg!(target_pointer_width = "32") {
                8 + 4 * CALLBACKS
            } else {
                16 + 8 * CALLBACKS
            }
    );
};

// ---- host adapter -----------------------------------------------------------------------------------

struct CHost {
    h: fc4_disasters_host,
    tuning: RefCell<BTreeMap<u32, Rc<PropertyHolder>>>,
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

fn c_name(name: &str) -> CString {
    CString::new(name).unwrap_or_default()
}

fn rect_to_c(r: CellRect) -> fc4_cell_rect {
    fc4_cell_rect {
        x1: r.x0,
        z1: r.z0,
        x2: r.x1,
        z2: r.z1,
    }
}

fn rect_from_c(r: &fc4_cell_rect) -> CellRect {
    CellRect {
        x0: r.x1,
        z0: r.z1,
        x1: r.x2,
        z1: r.z2,
    }
}

fn target_to_c(t: Target) -> (u32, u32) {
    match t {
        Target::Layer => (FC4_DISASTER_TARGET_LAYER, 0),
        Target::Manager(m) => (FC4_DISASTER_TARGET_MANAGER, m),
        Target::Instance(i) => (FC4_DISASTER_TARGET_INSTANCE, i),
    }
}

unsafe extern "C" fn push_token(ctx: *mut c_void, token: u64) {
    // SAFETY: ctx is the Vec handed to occupants_in_rect below.
    if let Some(v) = (ctx as *mut Vec<u64>).as_mut() {
        v.push(token);
    }
}

type CellQuery = Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32) -> bool>;

impl CHost {
    fn cell_pair(&self, f: CellQuery, t: u64) -> Option<(i32, i32)> {
        let f = f?;
        let (mut x, mut z) = (0, 0);
        // SAFETY: as cb!.
        unsafe { f(self.h.user, t, &mut x, &mut z) }.then_some((x, z))
    }
}

impl DisasterHost for CHost {
    fn now_ms(&self) -> u64 {
        cb!(self, now_ms, 0)
    }
    fn emergency_pause(&self) {
        cb!(self, emergency_pause, ())
    }
    fn emergency_resume(&self) {
        cb!(self, emergency_resume, ())
    }
    fn disable_save(&self) {
        cb!(self, disable_save, ())
    }
    fn enable_save(&self) {
        cb!(self, enable_save, ())
    }
    fn sim_paused(&self) -> bool {
        cb!(self, sim_paused, false)
    }
    fn sim_emergency_paused(&self) -> bool {
        cb!(self, sim_emergency_paused, false)
    }
    fn city_established(&self) -> bool {
        cb!(self, city_established, false)
    }
    fn city_cells(&self) -> (u32, u32) {
        let (mut x, mut z) = (0u32, 0u32);
        cb!(self, city_cells, (), &mut x, &mut z);
        (x, z)
    }
    fn cell_width(&self) -> f32 {
        cb!(self, cell_width, 16.0)
    }
    fn post(&self, p: &DisasterPost) {
        let c = fc4_disaster_post {
            msg_type: p.msg_type,
            data2: p.data2,
            data1: p.data1,
            data3: p.data3,
            data4: p.data4,
            void1: p.void1,
            reserved: 0,
        };
        cb!(self, post, (), &c)
    }
    fn subscribe(&self, target: Target, msg_type: u32) {
        let (k, id) = target_to_c(target);
        cb!(self, subscribe, (), k, id, msg_type)
    }
    fn unsubscribe(&self, target: Target, msg_type: u32) {
        let (k, id) = target_to_c(target);
        cb!(self, unsubscribe, (), k, id, msg_type)
    }
    fn register_tick(&self, instance: u32, priority: u32) {
        cb!(self, register_tick, (), instance, priority)
    }
    fn unregister_tick(&self, instance: u32) {
        cb!(self, unregister_tick, (), instance)
    }
    fn record_history(&self, series: u32, value: i32) {
        cb!(self, record_history, (), series, value)
    }
    fn tuning(&self, instance: u32) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().get(&instance).cloned()
    }
    fn add_ref(&self, token: u64) {
        cb!(self, add_ref, (), token)
    }
    fn release(&self, token: u64) {
        cb!(self, release, (), token)
    }
    fn follow_disasters(&self) -> bool {
        cb!(self, follow_disasters, false)
    }
    fn go_to_location(&self, x: i32, z: i32, zoom: u32) {
        cb!(self, go_to_location, (), x, z, zoom)
    }
    fn set_fire_dispatch_effectiveness(&self, value: f32) {
        cb!(self, set_fire_dispatch_effectiveness, (), value)
    }
    fn set_police_dispatch_effectiveness(&self, value: f32) {
        cb!(self, set_police_dispatch_effectiveness, (), value)
    }
    fn building_type_flags(&self, o: u64) -> u32 {
        cb!(self, building_type_flags, 0, o)
    }
    fn network_default_cost(&self, o: u64) -> i64 {
        cb!(self, network_default_cost, 0, o)
    }
    fn occupant_type(&self, o: u64) -> u32 {
        cb!(self, occupant_type, 0, o)
    }
    fn occupant_in_manager(&self, o: u64) -> bool {
        cb!(self, occupant_in_manager, false, o)
    }
    fn occupant_cell(&self, o: u64) -> Option<(i32, i32)> {
        self.cell_pair(self.h.occupant_cell, o)
    }
    fn occupant_cell_rect(&self, o: u64) -> CellRect {
        let mut r = fc4_cell_rect::default();
        cb!(self, occupant_cell_rect, (), o, &mut r);
        rect_from_c(&r)
    }
    fn occupant_position(&self, o: u64) -> (f32, f32) {
        let (mut x, mut z) = (0.0, 0.0);
        cb!(self, occupant_position, (), o, &mut x, &mut z);
        (x, z)
    }
    fn occupant_bbox(&self, o: u64) -> WorldBox {
        let mut b = fc4_world_box::default();
        cb!(self, occupant_bbox, (), o, &mut b);
        WorldBox {
            x0: b.x0,
            z0: b.z0,
            x1: b.x1,
            z1: b.z1,
        }
    }
    fn occupant_bbox_areas(&self, o: u64) -> (f32, f32) {
        let (mut t, mut s) = (0.0, 0.0);
        cb!(self, occupant_bbox_areas, (), o, &mut t, &mut s);
        (t, s)
    }
    fn occupant_prop_u32(&self, o: u64, id: u32) -> Option<u32> {
        let mut v = 0;
        cb!(self, occupant_prop_u32, false, o, id, &mut v).then_some(v)
    }
    fn occupant_prop_i64(&self, o: u64, id: u32) -> Option<i64> {
        let mut v = 0;
        cb!(self, occupant_prop_i64, false, o, id, &mut v).then_some(v)
    }
    fn occupant_prop_f32(&self, o: u64, id: u32) -> Option<f32> {
        let mut v = 0.0;
        cb!(self, occupant_prop_f32, false, o, id, &mut v).then_some(v)
    }
    fn occupant_prop_bool(&self, o: u64, id: u32) -> Option<bool> {
        let mut v = false;
        cb!(self, occupant_prop_bool, false, o, id, &mut v).then_some(v)
    }
    fn occupants_in_rect(&self, rect: CellRect) -> Vec<u64> {
        let mut v: Vec<u64> = Vec::new();
        let r = rect_to_c(rect);
        let ctx = &mut v as *mut Vec<u64> as *mut c_void;
        cb!(self, occupants_in_rect, (), &r, Some(push_token), ctx);
        v
    }
    fn remove_occupant(&self, o: u64) {
        cb!(self, remove_occupant, (), o)
    }
    fn occupant_lot(&self, o: u64) -> Option<u64> {
        opt(cb!(self, occupant_lot, 0, o))
    }
    fn set_lot_condition(&self, lot: u64, condition: u32) {
        cb!(self, set_lot_condition, (), lot, condition)
    }
    fn fire_coverage(&self, o: u64) -> u8 {
        cb!(self, fire_coverage, 0, o)
    }
    fn flammability_at(&self, x: i32, z: i32) -> u8 {
        cb!(self, flammability_at, 0, x, z)
    }
    fn create_fire_occupant(&self, instance: u32, target: u64, fire_id: u32, flames: (u32, u32)) -> Option<u64> {
        opt(cb!(
            self,
            create_fire_occupant,
            0,
            instance,
            target,
            fire_id,
            flames.0,
            flames.1
        ))
    }
    fn remove_fire_occupant(&self, fire: u64, target: u64) {
        cb!(self, remove_fire_occupant, (), fire, target)
    }
    fn add_fire_aura(&self, o: u64) {
        cb!(self, add_fire_aura, (), o)
    }
    fn tutorial_no_burn_down(&self) -> bool {
        cb!(self, tutorial_no_burn_down, false)
    }
    fn wind(&self) -> f32 {
        cb!(self, wind, 0.0)
    }
    fn play_effect(&self, name: &str, o: u64) {
        let n = c_name(name);
        cb!(self, play_effect, (), n.as_ptr(), o)
    }
    fn play_effect_at(&self, name: &str, x: f32, z: f32) {
        let n = c_name(name);
        cb!(self, play_effect_at, (), n.as_ptr(), x, z)
    }
    fn burn_flora(&self, o: u64) {
        cb!(self, burn_flora, (), o)
    }
    fn demolish_lot(&self, o: u64, lot: u64, flags: u32) {
        cb!(self, demolish_lot, (), o, lot, flags)
    }
    fn spew_radiation(&self, o: u64, amount: f32, radius: f32) {
        cb!(self, spew_radiation, (), o, amount, radius)
    }
    fn police_coverage(&self, x: i32, z: i32) -> u8 {
        cb!(self, police_coverage, 0, x, z)
    }
    fn is_road_cell(&self, x: i32, z: i32) -> bool {
        cb!(self, is_road_cell, false, x, z)
    }
    fn cell_corner_position(&self, x: i32, z: i32) -> (f32, f32) {
        let (mut wx, mut wz) = (0.0, 0.0);
        cb!(self, cell_corner_position, (), x, z, &mut wx, &mut wz);
        (wx, wz)
    }
    fn create_riot_leader(&self, x: f32, z: f32) -> Option<u64> {
        opt(cb!(self, create_riot_leader, 0, x, z))
    }
    fn network_occupant_at(&self, x: i32, z: i32) -> Option<u64> {
        opt(cb!(self, network_occupant_at, 0, x, z))
    }
    fn unset_removal_flags(&self, occupant: u64, flags: u32) {
        cb!(self, unset_removal_flags, (), occupant, flags)
    }
    fn set_removal_flags(&self, occupant: u64, flags: u32) {
        cb!(self, set_removal_flags, (), occupant, flags)
    }
    fn automaton_cell(&self, a: u64) -> Option<(i32, i32)> {
        self.cell_pair(self.h.automaton_cell, a)
    }
    fn set_riot_effect(&self, a: u64, stage: i32) {
        cb!(self, set_riot_effect, (), a, stage)
    }
    fn send_riot_message(&self, a: u64, stage: i32) {
        cb!(self, send_riot_message, (), a, stage)
    }
    fn end_riot_leader(&self, a: u64) {
        cb!(self, end_riot_leader, (), a)
    }
    fn riot_leader_tick(&self, a: u64) {
        cb!(self, riot_leader_tick, (), a)
    }
    fn create_toxic_occupant(&self, instance: u32, x: i32, z: i32, amount: i32) -> Option<u64> {
        opt(cb!(self, create_toxic_occupant, 0, instance, x, z, amount))
    }
    fn set_toxic_amount(&self, o: u64, amount: i32) {
        cb!(self, set_toxic_amount, (), o, amount)
    }
    fn remove_toxic_occupant(&self, o: u64) {
        cb!(self, remove_toxic_occupant, (), o)
    }
    fn apply_terrain_brush(&self, brush: u32, x: f32, z: f32, strength: f32, width: f32) {
        cb!(self, apply_terrain_brush, (), brush, x, z, strength, width)
    }
    fn redraw_terrain(&self, rect: CellRect) {
        let r = rect_to_c(rect);
        cb!(self, redraw_terrain, (), &r)
    }
    fn end_long_terrain_operation(&self) {
        cb!(self, end_long_terrain_operation, ())
    }
    fn stop_terrain_texture_updates(&self) {
        cb!(self, stop_terrain_texture_updates, ())
    }
    fn place_repulsor(&self, x: f32, z: f32) {
        cb!(self, place_repulsor, (), x, z)
    }
    fn find_watered_pipe(&self, x: i32, z: i32) -> Option<u64> {
        opt(cb!(self, find_watered_pipe, 0, x, z))
    }
    fn pipe_cell(&self, pipe: u64) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        cb!(self, pipe_cell, (), pipe, &mut x, &mut z);
        (x, z)
    }
    fn view_level(&self) -> u32 {
        cb!(self, view_level, 0)
    }
    fn create_leak_effect(&self, pipe: u64, view_level: u32) -> bool {
        cb!(self, create_leak_effect, false, pipe, view_level)
    }
    fn connected_pipe_cells(&self, pipe: u64) -> Vec<(i32, i32)> {
        let mut buf = vec![0i32; 32];
        let mut n = cb!(self, connected_pipe_cells, 0, pipe, buf.as_mut_ptr(), buf.len() / 2);
        if n > buf.len() / 2 {
            buf = vec![0i32; n * 2];
            n = cb!(self, connected_pipe_cells, 0, pipe, buf.as_mut_ptr(), buf.len() / 2).min(buf.len() / 2);
        }
        buf.chunks_exact(2).take(n).map(|c| (c[0], c[1])).collect()
    }
    fn demolish_pipe(&self, pipe: u64, flags: u32) {
        cb!(self, demolish_pipe, (), pipe, flags)
    }
    fn volcano_start_deformation(&self, instance: u32, x: f32, z: f32) {
        cb!(self, volcano_start_deformation, (), instance, x, z)
    }
    fn volcano_end_deformation(&self, instance: u32) {
        cb!(self, volcano_end_deformation, (), instance)
    }
    fn volcano_start_lava(&self, instance: u32, x: f32, z: f32, radius: f32) {
        cb!(self, volcano_start_lava, (), instance, x, z, radius)
    }
    fn volcano_lava_rates(&self, instance: u32, production: f32, solidification: f32) {
        cb!(self, volcano_lava_rates, (), instance, production, solidification)
    }
    fn volcano_end_lava(&self, instance: u32) {
        cb!(self, volcano_end_lava, (), instance)
    }
}

// ---- handles ----------------------------------------------------------------------------------------

struct Handle {
    sys: Rc<Disasters>,
    host: Rc<CHost>,
}

fn with_d<R>(d: *const fc4_disasters, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_disasters*.
    match unsafe { (d as *const Handle).as_ref() } {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

fn target_from_c(kind: u32, id: u32) -> Option<Target> {
    match kind {
        FC4_DISASTER_TARGET_LAYER => Some(Target::Layer),
        FC4_DISASTER_TARGET_MANAGER => Some(Target::Manager(id)),
        FC4_DISASTER_TARGET_INSTANCE => Some(Target::Instance(id)),
        _ => None,
    }
}

/// Creates the layer (not initialised). The host table is copied.
/// `session_seed` seeds the generators (foundations §6.3, disasters §13).
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_create(
    host: *const fc4_disasters_host,
    session_seed: u32,
) -> *mut fc4_disasters {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_disasters_host>() => {
            let c = Rc::new(CHost {
                h: *h,
                tuning: RefCell::new(BTreeMap::new()),
            });
            let sys = Rc::new(Disasters::new(c.clone(), session_seed));
            Box::into_raw(Box::new(Handle { sys, host: c })) as *mut fc4_disasters
        }
        _ => std::ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_destroy(d: *mut fc4_disasters) {
    if !d.is_null() {
        guard((), || drop(Box::from_raw(d as *mut Handle)));
    }
}

/// The tuning exemplar of `instance` (group `0x8A5971C5`: 1 volcano, 2
/// earthquake, 3 pipe burst, `0xC9B6B73D` flame, `0x4A38206D` riot); copied,
/// NULL removes it. The core reads each once, at the first use.
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_set_tuning(
    d: *mut fc4_disasters,
    instance: u32,
    tuning: *const fc4_props,
) -> bool {
    let t = props_ref(tuning).cloned().map(Rc::new);
    with_d(d, false, |h| {
        match t {
            Some(t) => h.host.tuning.borrow_mut().insert(instance, t),
            None => h.host.tuning.borrow_mut().remove(&instance),
        };
        true
    })
}

/// `Init` (§2.2): creates and inits the seven managers.
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_init(d: *mut fc4_disasters) -> bool {
    with_d(d, false, |h| h.sys.init())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_shutdown(d: *mut fc4_disasters) -> bool {
    with_d(d, false, |h| h.sys.shutdown())
}

/// `EnableDisasters` (§2.2, nested): the state after the call.
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_enable(d: *mut fc4_disasters, enable: bool) -> bool {
    with_d(d, false, |h| h.sys.enable_disasters(enable))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_enabled(d: *const fc4_disasters) -> bool {
    with_d(d, false, |h| h.sys.disasters_enabled())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_has_manager(d: *const fc4_disasters, manager_type: u32) -> bool {
    with_d(d, false, |h| h.sys.has_manager(manager_type))
}

/// `DisasterHasBegun(instance)` (§2.3).
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_has_begun(d: *mut fc4_disasters, instance: u32) {
    with_d(d, (), |h| h.sys.disaster_has_begun(instance))
}

/// `DisasterHasEnded()` (§2.3).
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_has_ended(d: *mut fc4_disasters) {
    with_d(d, (), |h| h.sys.disaster_has_ended())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_is_ongoing(d: *const fc4_disasters) -> bool {
    with_d(d, false, |h| h.sys.is_disaster_ongoing())
}

/// `RegisterDisasterLocation` (§2.6). `occupant` 0 = a cell location.
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_register_location(
    d: *mut fc4_disasters,
    instance: u32,
    x: i32,
    z: i32,
    occupant: u64,
    zoom: u32,
    focus: bool,
) {
    with_d(d, (), |h| {
        h.sys.register_location(instance, x, z, occupant, zoom, focus)
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_unregister_locations(d: *mut fc4_disasters, instance: u32) {
    with_d(d, (), |h| h.sys.unregister_locations(instance))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_unregister_location_by_occupant(
    d: *mut fc4_disasters,
    instance: u32,
    occupant: u64,
) {
    with_d(d, (), |h| h.sys.unregister_location_by_occupant(instance, occupant))
}

/// `GetMostRecentDisasterLocation(instance)` (0 = any); false = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_most_recent_location(
    d: *const fc4_disasters,
    instance: u32,
    x: *mut i32,
    z: *mut i32,
) -> bool {
    with_d(d, false, |h| match h.sys.most_recent_location(instance) {
        Some((lx, lz)) => {
            if let Some(x) = x.as_mut() {
                *x = lx;
            }
            if let Some(z) = z.as_mut() {
                *z = lz;
            }
            true
        }
        None => false,
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_location_count(d: *const fc4_disasters) -> usize {
    with_d(d, 0, |h| h.sys.location_count())
}

/// `GoToNextDisaster` (§2.6).
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_go_to_next(d: *mut fc4_disasters) {
    with_d(d, (), |h| h.sys.go_to_next_disaster())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_cost(d: *const fc4_disasters) -> i64 {
    with_d(d, 0, |h| h.sys.disaster_cost())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_reset_cost(d: *mut fc4_disasters) {
    with_d(d, (), |h| h.sys.reset_disaster_cost())
}

/// `DoMessage` on the layer, a manager (`id` = its type) or an instance.
/// False for an unknown target kind or a NULL message.
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_do_message(
    d: *mut fc4_disasters,
    target_kind: u32,
    id: u32,
    m: *const fc4_disaster_message,
) -> bool {
    let Some(m) = m.as_ref() else { return false };
    let Some(t) = target_from_c(target_kind, id) else {
        return false;
    };
    let msg = DisasterMessage {
        msg_type: m.msg_type,
        data1: m.data1,
        data2: m.data2,
        data3: m.data3,
        void1: m.void1,
        x: m.x,
        z: m.z,
    };
    with_d(d, false, |h| h.sys.do_message(t, &msg))
}

/// One animation tick of an instance (`us` microseconds), for the ticks
/// the core registered through `register_tick`.
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_tick(d: *mut fc4_disasters, instance: u32, us: u32) {
    with_d(d, (), |h| h.sys.tick(instance, us))
}

/// `AttemptDisasterStart(x, z)` on a manager: the instance, 0 = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_attempt_start(d: *mut fc4_disasters, manager_type: u32, x: i32, z: i32) -> u32 {
    with_d(d, 0, |h| h.sys.attempt_start(manager_type, x, z).unwrap_or(0))
}

/// `AttemptDisasterStart()` (a random place): the instance, 0 = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_attempt_start_any(d: *mut fc4_disasters, manager_type: u32) -> u32 {
    with_d(d, 0, |h| h.sys.attempt_start_any(manager_type).unwrap_or(0))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_active_count(d: *const fc4_disasters, manager_type: u32) -> u32 {
    with_d(d, 0, |h| h.sys.active_count(manager_type))
}

/// The instance's `Shutdown` (it leaves its manager).
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_instance_shutdown(d: *mut fc4_disasters, instance: u32) {
    with_d(d, (), |h| h.sys.instance_shutdown(instance))
}

/// The fire instance (created at first use; 0 = none).
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_fire_instance(d: *const fc4_disasters) -> u32 {
    with_d(d, 0, |h| h.sys.fire_instance())
}

/// The fire manager's `IgniteOccupant` (true even when refused, §5.4).
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_ignite_occupant(d: *mut fc4_disasters, occupant: u64) -> bool {
    with_d(d, false, |h| h.sys.ignite_occupant(occupant))
}

/// The fire's `Ignite(occupant, force, fireID, focus)` (§5.4).
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_ignite(
    d: *mut fc4_disasters,
    occupant: u64,
    force: bool,
    fire_id: u32,
    focus: bool,
) -> bool {
    with_d(d, false, |h| h.sys.ignite(occupant, force, fire_id, focus))
}

/// The fire's `IgniteCell` (§5.5).
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_ignite_cell(
    d: *mut fc4_disasters,
    x: i32,
    z: i32,
    use_probability: bool,
    teleport: bool,
    fire_id: u32,
) -> bool {
    with_d(d, false, |h| {
        h.sys.ignite_cell(x, z, use_probability, teleport, fire_id)
    })
}

/// The fire manager's `IgniteCell` slot (always true).
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_manager_ignite_cell(
    d: *mut fc4_disasters,
    x: i32,
    z: i32,
    use_probability: bool,
    teleport: bool,
    fire_id: u32,
) -> bool {
    with_d(d, false, |h| {
        h.sys.manager_ignite_cell(x, z, use_probability, teleport, fire_id)
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_extinguish_cell(d: *mut fc4_disasters, x: i32, z: i32, amount: i32) {
    with_d(d, (), |h| h.sys.extinguish_cell(x, z, amount))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_quick_burn_cell(d: *mut fc4_disasters, x: i32, z: i32) {
    with_d(d, (), |h| h.sys.quick_burn_cell(x, z))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_ignite_cell_radius(d: *mut fc4_disasters, x: i32, z: i32, r0: i32, r1: i32) {
    with_d(d, (), |h| h.sys.ignite_cell_radius(x, z, r0, r1))
}

/// `ExtinguishFire` on the fire burning `occupant`.
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_extinguish(d: *mut fc4_disasters, occupant: u64) {
    with_d(d, (), |h| h.sys.extinguish(occupant))
}

/// `BurnDownFire` on the fire burning `occupant`.
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_burn_down(d: *mut fc4_disasters, occupant: u64) {
    with_d(d, (), |h| h.sys.burn_down(occupant))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_set_cause(d: *mut fc4_disasters, cause: u32) {
    with_d(d, (), |h| h.sys.set_cause(cause))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_cause(d: *const fc4_disasters) -> u32 {
    with_d(d, 0, |h| h.sys.cause())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_fire_count(d: *const fc4_disasters) -> u32 {
    with_d(d, 0, |h| h.sys.fire_count())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_fire_count_in_rect(d: *const fc4_disasters, rect: *const fc4_cell_rect) -> u32 {
    let Some(r) = rect.as_ref() else { return 0 };
    let r = rect_from_c(r);
    with_d(d, 0, |h| h.sys.fire_count_in_rect(r))
}

/// The fire instance's tick (`Update` once its second is due, §5.9).
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_fire_update(d: *mut fc4_disasters) {
    with_d(d, (), |h| h.sys.fire_update())
}

/// A hose on (+1) or off (−1) the fire occupant; false = not a fire.
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_fire_set_hosed(d: *mut fc4_disasters, fire_occupant: u64, delta: i32) -> bool {
    with_d(d, false, |h| h.sys.fire_set_hosed(fire_occupant, delta))
}

/// `SpillToxicWaste(x, z, amount)` (§9): the toxic instance, 0 = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_spill_toxic_waste(d: *mut fc4_disasters, x: i32, z: i32, amount: i32) -> u32 {
    with_d(d, 0, |h| h.sys.spill_toxic_waste(x, z, amount).unwrap_or(0))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_toxic_set_hosed(d: *mut fc4_disasters, toxic_occupant: u64, delta: i32) -> bool {
    with_d(d, false, |h| h.sys.toxic_set_hosed(toxic_occupant, delta))
}

/// Asks the volcano to end after its lava phase (§10).
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_volcano_request_end(d: *mut fc4_disasters, instance: u32) -> bool {
    with_d(d, false, |h| h.sys.volcano_request_end(instance))
}

/// An effect disaster (§11) following `effect` from (x, z): the instance,
/// 0 = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_start_effect(d: *mut fc4_disasters, effect: u64, x: f32, z: f32) -> u32 {
    with_d(d, 0, |h| h.sys.start_disaster_effect(effect, x, z).unwrap_or(0))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_get_state(d: *const fc4_disasters, out: *mut fc4_disasters_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_d(d, false, |h| {
        let l = h.sys.layer_snapshot();
        let f = h.sys.fire_snapshot();
        let t = h.sys.toxic_snapshot();
        *o = fc4_disasters_state {
            struct_size: std::mem::size_of::<fc4_disasters_state>() as u32,
            initialised: l.initialised.into(),
            enabled: (l.disable_counter == 0).into(),
            assessing: l.assessing.into(),
            reserved: 0,
            disable_counter: l.disable_counter,
            ongoing: l.ongoing,
            location_count: l.locations.len() as u32,
            fire_instance: h.sys.fire_instance(),
            fire_phase: f.as_ref().map_or(0, |f| f.phase),
            fire_count: h.sys.fire_count(),
            toxic_instance: t.as_ref().map_or(0, |t| t.id),
            toxic_spills: t.as_ref().map_or(0, |t| t.spills.len() as u32),
            reentrant_refused: h.sys.reentrant_refused(),
            fire_timers: l.fire_timers.len() as u32,
            police_timers: l.police_timers.len() as u32,
            fire_effectiveness: l.fire_effectiveness,
            police_effectiveness: l.police_effectiveness,
            damaged_buildings: l.damage.building_count,
            riot_locked: h.sys.riot_locked().len() as u32,
            disaster_cost: h.sys.disaster_cost(),
        };
        true
    })
}

/// The fire at `index` of the fire list; false = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_get_fire(
    d: *const fc4_disasters,
    index: usize,
    out: *mut fc4_disaster_fire,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_d(d, false, |h| {
        let Some(e) = h.sys.fire_snapshot().and_then(|f| f.fires.get(index).copied()) else {
            return false;
        };
        *o = fc4_disaster_fire {
            occupant: e.occupant,
            target: e.target,
            counter: e.counter,
            stage: e.stage,
            max_stage: e.max_stage,
            fire_id: e.fire_id,
            hosed: e.hosed,
            top_flames: e.top_flames,
            side_flames: e.side_flames,
            extinguishing: e.extinguish_at.is_some().into(),
            extinguish_at: e.extinguish_at.unwrap_or(0),
        };
        true
    })
}

/// An instance's identity and state; false = no such instance.
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_get_instance(
    d: *const fc4_disasters,
    instance: u32,
    out: *mut fc4_disaster_instance,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_d(d, false, |h| {
        let Some(i) = h.sys.instance_info(instance) else {
            return false;
        };
        let (x, z) = i.location.unwrap_or((0, 0));
        *o = fc4_disaster_instance {
            manager_type: i.mtype,
            class_id: i.class,
            phase: i.phase,
            tick_registered: i.tick_registered.into(),
            has_location: i.location.is_some().into(),
            reserved: 0,
            x,
            z,
        };
        true
    })
}

/// A riot's leaders. Returns the number of leaders (0 for no such riot);
/// copies at most `capacity` (`out` may be NULL).
#[no_mangle]
pub unsafe extern "C" fn fc4_disasters_riot_leaders(
    d: *const fc4_disasters,
    instance: u32,
    out: *mut fc4_riot_leader,
    capacity: usize,
) -> usize {
    with_d(d, 0, |h| {
        let Some(r) = h.sys.riot_snapshot(instance) else {
            return 0;
        };
        if !out.is_null() {
            for (i, l) in r.leaders.iter().take(capacity).enumerate() {
                *out.add(i) = fc4_riot_leader {
                    automaton: l.automaton,
                    counter: l.counter,
                    stage: l.stage,
                    max_stage: l.max_stage,
                    x: l.cell.0,
                    z: l.cell.1,
                    reserved: 0,
                };
            }
        }
        r.leaders.len()
    })
}
