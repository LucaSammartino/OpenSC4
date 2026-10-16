// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The city situation manager over the C ABI (fc4.h "city situation
//! manager", docs/specs/city-situation.md).
//!
//! The host implements [`fc4_citysit_host`]; the plugin's
//! `cISC4CitySituationManager` (with its message-target and serializable
//! sub-objects) forwards its slots here. Tokens are opaque host identities:
//! advices, automata (vehicles), targets, automata managers and windows.
//! The core holds one reference on the active advice, the held window and
//! the automata managers (`acquire_automata_managers` hands them over
//! held); everything else is borrowed. The mission gameplay itself (targets,
//! `CheckForSuccess`, pursuit, coverage, the player-drive camera, the
//! windows and effects) stays the host's.
//!
//! Slots 13 to 15 of the interface return `float`; the C getters here
//! return `float` too (the C++ shim returns them in ST0, §12 item 8).
//!
//! Threads: the core is single-threaded (`RefCell`). The plugin serialises
//! every call on one object (fc4.h).

use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::city_situation::{
    query_interface, AdviceInfo, CitySituationHost, CitySituationManager, Popup, Script, SituationMessage,
};
use fc4_core::stream::SaveObject;

use crate::budget::fc4_token_callback;
use crate::guard;
use crate::hostgrid::collect_tokens;
use crate::stream::{fc4_bytes_sink, fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;
type Tokens = fc4_token_callback;

#[repr(C)]
pub struct fc4_citysit {
    _private: [u8; 0],
}

/// What the core needs to know about a situation advice (fc4.h
/// `fc4_citysit_advice_info`, §6).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_citysit_advice_info {
    pub guid: u32,
    /// 8: remember a window; 0x40: a reward.
    pub flags: u32,
    /// 2: no target; 5: coverage; others: target missions.
    pub mission_type: u32,
    /// Seconds (used when the vehicle has flag 0x8000000 and it is > 1.0).
    pub end_delay: f32,
    pub success_money: i32,
    pub failure_money: i32,
    pub success_radius: f32,
    pub success_magnitude: f32,
    pub failure_radius: f32,
    pub failure_magnitude: f32,
}

impl From<fc4_citysit_advice_info> for AdviceInfo {
    fn from(a: fc4_citysit_advice_info) -> Self {
        AdviceInfo {
            guid: a.guid,
            flags: a.flags,
            mission_type: a.mission_type,
            end_delay: a.end_delay,
            success_money: a.success_money,
            failure_money: a.failure_money,
            success_aura: (a.success_radius, a.success_magnitude),
            failure_aura: (a.failure_radius, a.failure_magnitude),
        }
    }
}

/// Scripts (`run_script`).
pub const FC4_CITYSIT_SCRIPT_START: u32 = 0;
pub const FC4_CITYSIT_SCRIPT_SUCCESS: u32 = 1;
pub const FC4_CITYSIT_SCRIPT_FAILURE: u32 = 2;
/// Popups (`show_popup`): the briefing; success (and "could not start");
/// failure.
pub const FC4_CITYSIT_POPUP_BRIEFING: u32 = 0;
pub const FC4_CITYSIT_POPUP_SUCCESS: u32 = 1;
pub const FC4_CITYSIT_POPUP_FAILURE: u32 = 2;

/// What the city situation manager needs from the game (fc4.h
/// `fc4_citysit_host`). A missing callback reads as 0 / false / none.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_citysit_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    // lifecycle (§4.1)
    pub debug_enabled: Option<unsafe extern "C" fn(U) -> bool>,
    pub register_cheat: Option<unsafe extern "C" fn(U) -> bool>,
    pub unregister_cheat: Option<unsafe extern "C" fn(U)>,
    pub subscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub acquire_indicator_view: Option<unsafe extern "C" fn(U) -> bool>,
    pub release_indicator_view: Option<unsafe extern "C" fn(U)>,
    pub register_animation_tick: Option<unsafe extern "C" fn(U)>,
    pub unregister_animation_tick: Option<unsafe extern "C" fn(U)>,
    /// The automata managers, each with one reference for the core.
    pub acquire_automata_managers: Option<unsafe extern "C" fn(U, Tokens, *mut c_void)>,
    pub acquire_lot_manager: Option<unsafe extern "C" fn(U) -> bool>,
    pub release_lot_manager: Option<unsafe extern "C" fn(U)>,
    pub acquire_mysim: Option<unsafe extern "C" fn(U) -> bool>,
    pub release_mysim: Option<unsafe extern "C" fn(U)>,
    pub acquire_lua: Option<unsafe extern "C" fn(U) -> bool>,
    pub release_lua: Option<unsafe extern "C" fn(U)>,
    pub create_lua_table: Option<unsafe extern "C" fn(U)>,
    pub remove_lua_table: Option<unsafe extern "C" fn(U)>,
    pub release_mission_resources: Option<unsafe extern "C" fn(U)>,
    pub show_all_indicators: Option<unsafe extern "C" fn(U, u32, bool)>,
    /// The preferences' reward list: returns its length, copies at most
    /// `capacity` (`out` may be NULL).
    pub preferences_rewards: Option<unsafe extern "C" fn(U, *mut u32, usize) -> usize>,
    pub set_preferences_reward: Option<unsafe extern "C" fn(U, u32, u32)>,
    pub save_preferences: Option<unsafe extern "C" fn(U)>,
    // references
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    // advices
    pub is_situation_advice: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub advisor_advices: Option<unsafe extern "C" fn(U, Tokens, *mut c_void)>,
    pub find_advice: Option<unsafe extern "C" fn(U, u32) -> u64>,
    /// Fills `*out` (zeroed by the core first).
    pub advice_info: Option<unsafe extern "C" fn(U, u64, *mut fc4_citysit_advice_info)>,
    pub retire_advice: Option<unsafe extern "C" fn(U, u64)>,
    /// `FC4_CITYSIT_SCRIPT_*`.
    pub run_script: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub time_limit: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub bind_mission_script: Option<unsafe extern "C" fn(U, u64, u64)>,
    pub build_coverage_cells: Option<unsafe extern "C" fn(U, u64)>,
    pub first_target: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub target_count: Option<unsafe extern "C" fn(U, u64) -> u32>,
    /// The name (UTF-8) through the sink.
    pub target_name: Option<unsafe extern "C" fn(U, u64, u64, fc4_bytes_sink, *mut c_void)>,
    /// Advice slot 53 (`+0xD4`): the advice accepts the automaton (slot 22).
    pub advice_accepts_automaton: Option<unsafe extern "C" fn(U, u64, u64) -> bool>,
    // automata
    pub is_automaton: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub occupant_has_property: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub candidate_vehicles: Option<unsafe extern "C" fn(U, u64, Tokens, *mut c_void)>,
    pub automaton_flags: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub automaton_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub track_indicator: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub untrack_indicator: Option<unsafe extern "C" fn(U, u32)>,
    pub add_listener: Option<unsafe extern "C" fn(U, u64)>,
    pub remove_listener: Option<unsafe extern "C" fn(U, u64)>,
    pub freeze_automata: Option<unsafe extern "C" fn(U, u64, bool)>,
    pub play_sound: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub position: Option<unsafe extern "C" fn(U, u64, *mut f32, *mut f32)>,
    // the mission (out of the spec's scope)
    pub check_for_success: Option<unsafe extern "C" fn(U, u64, f32) -> u32>,
    pub run_pursuit: Option<unsafe extern "C" fn(U, u64, f32)>,
    pub start_countdown_effect: Option<unsafe extern "C" fn(U)>,
    pub end_countdown_effect: Option<unsafe extern "C" fn(U)>,
    pub outcome_effect: Option<unsafe extern "C" fn(U, u64, bool)>,
    pub player_drive_mode: Option<unsafe extern "C" fn(U, u64, bool)>,
    pub start_joyride: Option<unsafe extern "C" fn(U, u64)>,
    // other components
    pub add_transient_effect: Option<unsafe extern "C" fn(U, f32, f32, f32, f32)>,
    pub deposit_funds: Option<unsafe extern "C" fn(U, i64)>,
    // messages
    pub send_activation: Option<unsafe extern "C" fn(U, u64, bool)>,
    pub post_show_popup: Option<unsafe extern "C" fn(U, u64, u64)>,
    pub post_popup_closed: Option<unsafe extern "C" fn(U, u64)>,
    pub post_initiate: Option<unsafe extern "C" fn(U, u32, bool)>,
    // the popup (§4.3)
    pub hide_mission_status_panel: Option<unsafe extern "C" fn(U)>,
    pub end_mysim_dialog: Option<unsafe extern "C" fn(U)>,
    /// `FC4_CITYSIT_POPUP_*`, the advice and the vehicle (0 = none): the
    /// button that closed it (0 = none).
    pub show_popup: Option<unsafe extern "C" fn(U, u32, u64, u64) -> u32>,
}

/// One track-list entry (`fc4_citysit_track_list`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_citysit_track_entry {
    pub advice: u64,
    /// 0 = none.
    pub automaton: u64,
    /// The indicator handle (0 = none).
    pub handle: u32,
    pub reserved: u32,
}

/// One success count (`fc4_citysit_success_counts`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_citysit_success_count {
    pub guid: u32,
    pub count: u16,
    pub reserved: u16,
}

/// Diagnostics (`fc4_citysit_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_citysit_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub destroyed: u8,
    pub indicators_visible: u8,
    pub loaded_from_save: u8,
    pub just_activated: u8,
    pub warning_given: u8,
    pub max_csi: u8,
    pub cheat_registered: u8,
    pub indicator_view_held: u8,
    pub lot_manager_held: u8,
    pub mysim_held: u8,
    pub lua_held: u8,
    pub ref_count: u32,
    pub status: u32,
    pub num_targets: u32,
    pub target_index: u32,
    pub end_delay: f32,
    pub elapsed: f32,
    pub countdown: f32,
    pub time_limit: f32,
    pub track_entries: u32,
    pub tracked: u32,
    pub success_counts: u32,
    pub automata_managers: u32,
    pub active_situation: u64,
    pub active_automaton: u64,
    pub current_target: u64,
    pub window: u64,
}

/// Number of callbacks in [`fc4_citysit_host`].
const HOST_CALLBACKS: usize = 66;

const _: () = {
    assert!(std::mem::size_of::<fc4_citysit_advice_info>() == 40);
    assert!(std::mem::size_of::<fc4_citysit_track_entry>() == 24);
    assert!(std::mem::size_of::<fc4_citysit_success_count>() == 8);
    assert!(std::mem::size_of::<fc4_citysit_state>() == 96);
    assert!(
        std::mem::size_of::<fc4_citysit_host>()
            == if cfg!(target_pointer_width = "32") {
                8 + 4 * HOST_CALLBACKS
            } else {
                16 + 8 * HOST_CALLBACKS
            }
    );
};

// ---- host adapter -----------------------------------------------------------------------------------

struct CHost {
    h: fc4_citysit_host,
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

unsafe extern "C" fn collect(ctx: *mut c_void, data: *const u8, len: usize) {
    if let Some(v) = (ctx as *mut Vec<u8>).as_mut() {
        if !data.is_null() && len > 0 {
            v.extend_from_slice(std::slice::from_raw_parts(data, len));
        }
    }
}

impl CHost {
    fn tokens(&self, f: impl FnOnce(U, Tokens, *mut c_void)) -> Vec<u64> {
        collect_tokens(|cb, ctx| f(self.h.user, cb, ctx))
    }
}

impl CitySituationHost for CHost {
    fn debug_enabled(&self) -> bool {
        cb!(self, debug_enabled, false)
    }
    fn register_cheat(&self) -> bool {
        cb!(self, register_cheat, false)
    }
    fn unregister_cheat(&self) {
        cb!(self, unregister_cheat, ())
    }
    fn subscribe(&self, msg_type: u32) {
        cb!(self, subscribe, (), msg_type)
    }
    fn unsubscribe(&self, msg_type: u32) {
        cb!(self, unsubscribe, (), msg_type)
    }
    fn acquire_indicator_view(&self) -> bool {
        cb!(self, acquire_indicator_view, false)
    }
    fn release_indicator_view(&self) {
        cb!(self, release_indicator_view, ())
    }
    fn register_animation_tick(&self) {
        cb!(self, register_animation_tick, ())
    }
    fn unregister_animation_tick(&self) {
        cb!(self, unregister_animation_tick, ())
    }
    fn acquire_automata_managers(&self) -> Vec<u64> {
        match self.h.acquire_automata_managers {
            // SAFETY: as cb!.
            Some(f) => self.tokens(|u, cb, ctx| unsafe { f(u, cb, ctx) }),
            None => Vec::new(),
        }
    }
    fn acquire_lot_manager(&self) -> bool {
        cb!(self, acquire_lot_manager, false)
    }
    fn release_lot_manager(&self) {
        cb!(self, release_lot_manager, ())
    }
    fn acquire_mysim(&self) -> bool {
        cb!(self, acquire_mysim, false)
    }
    fn release_mysim(&self) {
        cb!(self, release_mysim, ())
    }
    fn acquire_lua(&self) -> bool {
        cb!(self, acquire_lua, false)
    }
    fn release_lua(&self) {
        cb!(self, release_lua, ())
    }
    fn create_lua_table(&self) {
        cb!(self, create_lua_table, ())
    }
    fn remove_lua_table(&self) {
        cb!(self, remove_lua_table, ())
    }
    fn release_mission_resources(&self) {
        cb!(self, release_mission_resources, ())
    }
    fn show_all_indicators(&self, kind: u32, visible: bool) {
        cb!(self, show_all_indicators, (), kind, visible)
    }
    fn preferences_rewards(&self) -> Vec<u32> {
        let n = cb!(self, preferences_rewards, 0, std::ptr::null_mut(), 0);
        if n == 0 {
            return Vec::new();
        }
        let mut v = vec![0u32; n];
        let m = cb!(self, preferences_rewards, 0, v.as_mut_ptr(), n);
        v.truncate(m.min(n));
        v
    }
    fn set_preferences_reward(&self, index: usize, guid: u32) {
        cb!(self, set_preferences_reward, (), index as u32, guid)
    }
    fn save_preferences(&self) {
        cb!(self, save_preferences, ())
    }
    fn add_ref(&self, token: u64) {
        cb!(self, add_ref, (), token)
    }
    fn release(&self, token: u64) {
        cb!(self, release, (), token)
    }
    fn is_situation_advice(&self, object: u64) -> bool {
        cb!(self, is_situation_advice, false, object)
    }
    fn advisor_advices(&self) -> Vec<u64> {
        match self.h.advisor_advices {
            // SAFETY: as cb!.
            Some(f) => self.tokens(|u, cb, ctx| unsafe { f(u, cb, ctx) }),
            None => Vec::new(),
        }
    }
    fn find_advice(&self, guid: u32) -> Option<u64> {
        opt(cb!(self, find_advice, 0, guid))
    }
    fn advice_info(&self, advice: u64) -> AdviceInfo {
        let mut i = fc4_citysit_advice_info::default();
        cb!(self, advice_info, (), advice, &mut i);
        i.into()
    }
    fn retire_advice(&self, advice: u64) {
        cb!(self, retire_advice, (), advice)
    }
    fn run_script(&self, advice: u64, script: Script) {
        let s = match script {
            Script::Start => FC4_CITYSIT_SCRIPT_START,
            Script::Success => FC4_CITYSIT_SCRIPT_SUCCESS,
            Script::Failure => FC4_CITYSIT_SCRIPT_FAILURE,
        };
        cb!(self, run_script, (), advice, s)
    }
    fn time_limit(&self, advice: u64) -> f32 {
        cb!(self, time_limit, 0.0, advice)
    }
    fn bind_mission_script(&self, advice: u64, vehicle: Option<u64>) {
        cb!(self, bind_mission_script, (), advice, vehicle.unwrap_or(0))
    }
    fn build_coverage_cells(&self, advice: u64) {
        cb!(self, build_coverage_cells, (), advice)
    }
    fn first_target(&self, advice: u64) -> Option<u64> {
        opt(cb!(self, first_target, 0, advice))
    }
    fn target_count(&self, advice: u64) -> u32 {
        cb!(self, target_count, 0, advice)
    }
    fn target_name(&self, advice: Option<u64>, target: Option<u64>) -> String {
        let mut v: Vec<u8> = Vec::new();
        cb!(
            self,
            target_name,
            (),
            advice.unwrap_or(0),
            target.unwrap_or(0),
            Some(collect),
            &mut v as *mut Vec<u8> as *mut c_void
        );
        String::from_utf8_lossy(&v).into_owned()
    }
    fn advice_accepts_automaton(&self, advice: u64, automaton: u64) -> bool {
        cb!(self, advice_accepts_automaton, false, advice, automaton)
    }
    fn is_automaton(&self, object: u64) -> bool {
        cb!(self, is_automaton, false, object)
    }
    fn occupant_has_property(&self, object: u64, id: u32) -> bool {
        cb!(self, occupant_has_property, false, object, id)
    }
    fn candidate_vehicles(&self, advice: u64) -> Vec<u64> {
        match self.h.candidate_vehicles {
            // SAFETY: as cb!.
            Some(f) => self.tokens(|u, cb, ctx| unsafe { f(u, advice, cb, ctx) }),
            None => Vec::new(),
        }
    }
    fn automaton_flags(&self, automaton: u64) -> u32 {
        cb!(self, automaton_flags, 0, automaton)
    }
    fn automaton_type(&self, automaton: u64) -> u32 {
        cb!(self, automaton_type, 0, automaton)
    }
    fn track_indicator(&self, automaton: u64) -> u32 {
        cb!(self, track_indicator, 0, automaton)
    }
    fn untrack_indicator(&self, handle: u32) {
        cb!(self, untrack_indicator, (), handle)
    }
    fn add_listener(&self, automaton: u64) {
        cb!(self, add_listener, (), automaton)
    }
    fn remove_listener(&self, automaton: u64) {
        cb!(self, remove_listener, (), automaton)
    }
    fn freeze_automata(&self, manager: u64, on: bool) {
        cb!(self, freeze_automata, (), manager, on)
    }
    fn play_sound(&self, automaton: u64, sound: u32) {
        cb!(self, play_sound, (), automaton, sound)
    }
    fn position(&self, object: u64) -> (f32, f32) {
        let (mut x, mut z) = (0.0f32, 0.0f32);
        cb!(self, position, (), object, &mut x, &mut z);
        (x, z)
    }
    fn check_for_success(&self, advice: u64, t_ms: f32) -> u32 {
        // [choice] Without the callback a mission keeps running (3).
        cb!(self, check_for_success, 3, advice, t_ms)
    }
    fn run_pursuit(&self, advice: u64, t_ms: f32) {
        cb!(self, run_pursuit, (), advice, t_ms)
    }
    fn start_countdown_effect(&self) {
        cb!(self, start_countdown_effect, ())
    }
    fn end_countdown_effect(&self) {
        cb!(self, end_countdown_effect, ())
    }
    fn outcome_effect(&self, advice: u64, success: bool) {
        cb!(self, outcome_effect, (), advice, success)
    }
    fn player_drive_mode(&self, vehicle: Option<u64>, on: bool) {
        cb!(self, player_drive_mode, (), vehicle.unwrap_or(0), on)
    }
    fn start_joyride(&self, vehicle: Option<u64>) {
        cb!(self, start_joyride, (), vehicle.unwrap_or(0))
    }
    fn add_transient_effect(&self, x: f32, z: f32, radius: f32, magnitude: f32) {
        cb!(self, add_transient_effect, (), x, z, radius, magnitude)
    }
    fn deposit_funds(&self, amount: i64) {
        cb!(self, deposit_funds, (), amount)
    }
    fn send_activation(&self, advice: u64, active: bool) {
        cb!(self, send_activation, (), advice, active)
    }
    fn post_show_popup(&self, advice: Option<u64>, vehicle: Option<u64>) {
        cb!(self, post_show_popup, (), advice.unwrap_or(0), vehicle.unwrap_or(0))
    }
    fn post_popup_closed(&self, advice: u64) {
        cb!(self, post_popup_closed, (), advice)
    }
    fn post_initiate(&self, guid: u32, show_briefing: bool) {
        cb!(self, post_initiate, (), guid, show_briefing)
    }
    fn hide_mission_status_panel(&self) {
        cb!(self, hide_mission_status_panel, ())
    }
    fn end_mysim_dialog(&self) {
        cb!(self, end_mysim_dialog, ())
    }
    fn show_popup(&self, popup: Popup, advice: Option<u64>, vehicle: Option<u64>) -> u32 {
        let p = match popup {
            Popup::Briefing => FC4_CITYSIT_POPUP_BRIEFING,
            Popup::Success => FC4_CITYSIT_POPUP_SUCCESS,
            Popup::Failure => FC4_CITYSIT_POPUP_FAILURE,
        };
        cb!(self, show_popup, 0, p, advice.unwrap_or(0), vehicle.unwrap_or(0))
    }
}

// ---- handles ----------------------------------------------------------------------------------------

struct Handle {
    sim: Rc<CitySituationManager>,
}

fn with_p<R>(p: *const fc4_citysit, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_citysit*.
    match unsafe { (p as *const Handle).as_ref() } {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

unsafe fn copy_out<T: Copy>(v: &[T], out: *mut T, capacity: usize) -> usize {
    if !out.is_null() {
        for (i, x) in v.iter().take(capacity).enumerate() {
            *out.add(i) = *x;
        }
    }
    v.len()
}

/// Creates the core object (constructed, reference count 0). The host table
/// is copied. `session_seed` seeds the member generator (§5); `token` is
/// the manager's own token.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_create(
    host: *const fc4_citysit_host,
    session_seed: u32,
    token: u64,
) -> *mut fc4_citysit {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_citysit_host>() => {
            let c = Rc::new(CHost { h: *h });
            let sim = CitySituationManager::new(c, token, session_seed);
            Box::into_raw(Box::new(Handle { sim })) as *mut fc4_citysit
        }
        _ => std::ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_destroy(p: *mut fc4_citysit) {
    if !p.is_null() {
        guard((), || drop(Box::from_raw(p as *mut Handle)));
    }
}

/// `QueryInterface` (§1): true and the sub-object offset in `offset` (may
/// be NULL) for the IIDs it answers.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_query_interface(iid: u32, offset: *mut u32) -> bool {
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
pub unsafe extern "C" fn fc4_citysit_add_ref(p: *mut fc4_citysit) -> u32 {
    with_p(p, 0, |h| h.sim.add_ref())
}

/// `Release`: 0 when the object was destroyed (`Shutdown` ran); the handle
/// stays valid until [`fc4_citysit_destroy`].
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_release(p: *mut fc4_citysit) -> u32 {
    with_p(p, 0, |h| h.sim.release())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_init(p: *mut fc4_citysit) -> bool {
    with_p(p, false, |h| h.sim.init())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_shutdown(p: *mut fc4_citysit) -> bool {
    with_p(p, false, |h| h.sim.shutdown())
}

/// `DoMessage` (§3). `object` = IGZUnknown (an advice, a window) or the
/// occupant / automaton; `void1` = the vehicle of `0xEBB67EF7`; `text` /
/// `text_len` = the cheat's text of `0x230E27AC` (UTF-8, NULL = none). The
/// animation tick `0x86AD10EE` carries the milliseconds as float bits in
/// `data3`. Always true.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_do_message(
    p: *mut fc4_citysit,
    msg_type: u32,
    data1: u32,
    data2: u32,
    data3: u32,
    object: u64,
    void1: u64,
    text: *const u8,
    text_len: usize,
) -> bool {
    let text = if text.is_null() {
        None
    } else {
        Some(String::from_utf8_lossy(std::slice::from_raw_parts(text, text_len)).into_owned())
    };
    let m = SituationMessage {
        msg_type,
        data1,
        data2,
        data3,
        object: opt(object),
        void1: opt(void1),
        text,
    };
    with_p(p, true, |h| h.sim.do_message(&m))
}

/// The animation tick (§4.4) with `t_ms` milliseconds.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_tick(p: *mut fc4_citysit, t_ms: f32) {
    with_p(p, (), |h| h.sim.tick(t_ms))
}

/// A situation advice was triggered (§4.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_situation_triggered(p: *mut fc4_citysit, advice: u64) {
    if advice != 0 {
        with_p(p, (), |h| h.sim.situation_triggered(advice))
    }
}

/// A situation advice was untriggered (§4.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_situation_untriggered(p: *mut fc4_citysit, advice: u64) {
    if advice != 0 {
        with_p(p, (), |h| h.sim.situation_untriggered(advice))
    }
}

/// Slot 36 `InitiateSituation(advice, automaton, showBriefing)` (`vehicle`
/// 0 = none).
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_initiate_situation(
    p: *mut fc4_citysit,
    advice: u64,
    vehicle: u64,
    show_briefing: bool,
) -> bool {
    if advice == 0 {
        return false;
    }
    with_p(p, false, |h| {
        h.sim.initiate_situation(advice, opt(vehicle), show_briefing)
    })
}

/// Slot 21 `InitiateSituationByGuid(guid, showBriefing, data3)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_initiate_situation_by_guid(
    p: *mut fc4_citysit,
    guid: u32,
    show_briefing: bool,
    data3: u32,
) -> bool {
    with_p(p, false, |h| {
        h.sim.initiate_situation_by_guid(guid, show_briefing, data3)
    })
}

/// Slot 22 `InitiateSituationForOccupant(occupant, advice)` (§8; `advice`
/// 0 = none): false unless the occupant is an automaton; that advice, else
/// the first track-list entry whose vehicle it is or whose advice accepts
/// it, starts with a briefing.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_initiate_situation_for_occupant(
    p: *mut fc4_citysit,
    occupant: u64,
    advice: u64,
) -> bool {
    if occupant == 0 {
        return false;
    }
    with_p(p, false, |h| {
        h.sim.initiate_situation_for_occupant(occupant, opt(advice))
    })
}

/// Slot 37 `ActivateSituation` (§4.3).
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_activate_situation(p: *mut fc4_citysit) {
    with_p(p, (), |h| h.sim.activate_situation())
}

/// Slot 38 `DeactivateSituation`.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_deactivate_situation(p: *mut fc4_citysit) {
    with_p(p, (), |h| h.sim.deactivate_situation())
}

/// Slot 39 `EndSituation`.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_end_situation(p: *mut fc4_citysit) {
    with_p(p, (), |h| h.sim.end_situation())
}

/// The situation popup (§4.3) for the advice and vehicle (0 = none).
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_show_situation_popup(p: *mut fc4_citysit, advice: u64, vehicle: u64) {
    with_p(p, (), |h| h.sim.show_situation_popup(opt(advice), opt(vehicle)))
}

/// The outcome (§4.5): success.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_success(p: *mut fc4_citysit, advice: u64) {
    if advice != 0 {
        with_p(p, (), |h| h.sim.success(advice))
    }
}

/// The outcome (§4.5): failure.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_failure(p: *mut fc4_citysit, advice: u64) {
    if advice != 0 {
        with_p(p, (), |h| h.sim.failure(advice))
    }
}

/// Slot 5 / Lua `get_success_count`.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_get_success_count(p: *const fc4_citysit, guid: u32) -> u16 {
    with_p(p, 0, |h| h.sim.success_count(guid))
}

/// Lua `activate_situation(guid [, showBriefing])`.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_lua_activate_situation(
    p: *mut fc4_citysit,
    guid: u32,
    show_briefing: bool,
) -> bool {
    with_p(p, false, |h| h.sim.lua_activate_situation(guid, show_briefing))
}

/// Slot 6 (borrowed; 0 = none).
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_active_situation(p: *const fc4_citysit) -> u64 {
    with_p(p, 0, |h| h.sim.active_situation().unwrap_or(0))
}

/// Slot 7.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_current_status(p: *const fc4_citysit) -> u32 {
    with_p(p, 0, |h| h.sim.current_status())
}

/// Slot 8 (borrowed; 0 = none).
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_active_auto(p: *const fc4_citysit) -> u64 {
    with_p(p, 0, |h| h.sim.active_auto().unwrap_or(0))
}

/// Slot 9 (borrowed; 0 = none).
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_current_target(p: *const fc4_citysit) -> u64 {
    with_p(p, 0, |h| h.sim.current_target().unwrap_or(0))
}

/// Slot 10.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_current_target_index(p: *const fc4_citysit) -> u32 {
    with_p(p, 0, |h| h.sim.current_target_index())
}

/// Slot 11.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_num_targets(p: *const fc4_citysit) -> u32 {
    with_p(p, 0, |h| h.sim.num_targets())
}

/// Slot 12.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_num_targets_remaining(p: *const fc4_citysit) -> u32 {
    with_p(p, 0, |h| h.sim.num_targets_remaining())
}

/// Slot 13 (`float`).
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_time_limit_seconds(p: *const fc4_citysit) -> f32 {
    with_p(p, 0.0, |h| h.sim.time_limit_seconds())
}

/// Slot 14 (`float`).
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_situation_time(p: *const fc4_citysit) -> f32 {
    with_p(p, 0.0, |h| h.sim.situation_time())
}

/// Slot 15 (`float`): slot 13 − slot 14.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_time_remaining_seconds(p: *const fc4_citysit) -> f32 {
    with_p(p, 0.0, |h| h.sim.time_remaining_seconds())
}

/// Slot 16: the current target's name (UTF-8) through the sink.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_current_target_name(
    p: *const fc4_citysit,
    sink: fc4_bytes_sink,
    ctx: *mut c_void,
) -> bool {
    let Some(name) = with_p(p, None, |h| Some(h.sim.current_target_name())) else {
        return false;
    };
    if let Some(f) = sink {
        f(ctx, name.as_ptr(), name.len());
    }
    true
}

/// Slot 17: the active advice's success aura (false without a mission).
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_situation_aura(
    p: *const fc4_citysit,
    radius: *mut f32,
    magnitude: *mut f32,
) -> bool {
    match with_p(p, None, |h| h.sim.situation_aura()) {
        Some((r, m)) => {
            if !radius.is_null() {
                *radius = r;
            }
            if !magnitude.is_null() {
                *magnitude = m;
            }
            true
        }
        None => false,
    }
}

/// Slot 18: the active advice's success money (false without a mission).
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_situation_money(p: *const fc4_citysit, money: *mut i64) -> bool {
    match with_p(p, None, |h| h.sim.situation_money()) {
        Some(m) => {
            if !money.is_null() {
                *money = m;
            }
            true
        }
        None => false,
    }
}

/// Slots 19, 20: the automaton carries an indicator.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_is_situation_available(p: *const fc4_citysit, automaton: u64) -> bool {
    with_p(p, false, |h| h.sim.is_situation_available(automaton))
}

/// Slot 26.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_set_max_csi(p: *mut fc4_citysit, n: u8) {
    with_p(p, (), |h| h.sim.set_max_csi(n))
}

/// Slot 27.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_max_csi(p: *const fc4_citysit) -> u8 {
    with_p(p, 0, |h| h.sim.max_csi())
}

/// Slot 28.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_tracked_automata_count(p: *const fc4_citysit) -> u32 {
    with_p(p, 0, |h| h.sim.tracked_automata_count())
}

/// Slot 29.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_set_csi_visible(p: *mut fc4_citysit, on: bool) {
    with_p(p, (), |h| h.sim.set_csi_visible(on))
}

/// Slot 30.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_csi_visible(p: *const fc4_citysit) -> bool {
    with_p(p, false, |h| h.sim.csi_visible())
}

/// Slot 31.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_handle_tuning_parameters_changed(p: *mut fc4_citysit) -> bool {
    with_p(p, false, |h| h.sim.handle_tuning_parameters_changed())
}

/// Slot 32.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_countdown_in_progress(p: *const fc4_citysit) -> bool {
    with_p(p, false, |h| h.sim.countdown_in_progress())
}

/// Slot 33 `ResetTutorials`.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_reset_tutorials(p: *mut fc4_citysit) {
    with_p(p, (), |h| h.sim.reset_tutorials())
}

/// Slot 35 `InitiatePlayerDriveMode(on)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_initiate_player_drive_mode(p: *mut fc4_citysit, on: bool) {
    with_p(p, (), |h| h.sim.initiate_player_drive_mode(on))
}

/// The track list in order: returns the count; copies at most `capacity`
/// (`out` may be NULL).
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_track_list(
    p: *const fc4_citysit,
    out: *mut fc4_citysit_track_entry,
    capacity: usize,
) -> usize {
    with_p(p, 0, |h| {
        let v: Vec<fc4_citysit_track_entry> = h
            .sim
            .snapshot()
            .track_list
            .iter()
            .map(|e| fc4_citysit_track_entry {
                advice: e.advice,
                automaton: e.automaton.unwrap_or(0),
                handle: e.handle,
                reserved: 0,
            })
            .collect();
        copy_out(&v, out, capacity)
    })
}

/// The success counts in ascending GUID order: returns the count; copies
/// at most `capacity` (`out` may be NULL).
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_success_counts(
    p: *const fc4_citysit,
    out: *mut fc4_citysit_success_count,
    capacity: usize,
) -> usize {
    with_p(p, 0, |h| {
        let v: Vec<fc4_citysit_success_count> = h
            .sim
            .snapshot()
            .success_counts
            .iter()
            .map(|(&guid, &count)| fc4_citysit_success_count {
                guid,
                count,
                reserved: 0,
            })
            .collect();
        copy_out(&v, out, capacity)
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_get_state(p: *const fc4_citysit, out: *mut fc4_citysit_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_p(p, false, |h| {
        let s = h.sim.snapshot();
        *o = fc4_citysit_state {
            struct_size: std::mem::size_of::<fc4_citysit_state>() as u32,
            initialized: s.initialized.into(),
            destroyed: s.destroyed.into(),
            indicators_visible: s.indicators_visible.into(),
            loaded_from_save: s.loaded_from_save.into(),
            just_activated: s.just_activated.into(),
            warning_given: s.warning_given.into(),
            max_csi: s.max_csi,
            cheat_registered: s.cheat_registered.into(),
            indicator_view_held: s.indicator_view_held.into(),
            lot_manager_held: s.lot_manager_held.into(),
            mysim_held: s.mysim_held.into(),
            lua_held: s.lua_held.into(),
            ref_count: s.ref_count,
            status: s.status,
            num_targets: s.num_targets,
            target_index: s.target_index,
            end_delay: s.end_delay,
            elapsed: s.elapsed,
            countdown: s.countdown,
            time_limit: s.time_limit,
            track_entries: s.track_list.len() as u32,
            tracked: s.tracked.len() as u32,
            success_counts: s.success_counts.len() as u32,
            automata_managers: s.automata_managers.len() as u32,
            active_situation: s.active_situation.unwrap_or(0),
            active_automaton: s.active_automaton.unwrap_or(0),
            current_target: s.current_target.unwrap_or(0),
            window: s.window.unwrap_or(0),
        };
        true
    })
}

/// `Write` (§7).
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_write(p: *const fc4_citysit, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_p(p, false, |h| h.sim.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

/// `Read` (§7).
#[no_mangle]
pub unsafe extern "C" fn fc4_citysit_read(p: *mut fc4_citysit, inp: *const fc4_istream) -> bool {
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
