// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The dispatch manager over the C ABI (fc4.h "dispatch manager",
//! docs/specs/dispatch.md).
//!
//! The host implements [`fc4_dispatch_host`]; the plugin's
//! `cISC4DispatchManager` (with its message-target and serializable
//! sub-objects) forwards its slots here. Tokens are opaque host identities:
//! dispatch sources, station occupants, vehicle automata, pylons and the
//! vehicle view. The core holds one reference on every record's station,
//! automaton and pylon, on the active pylon, the available pylons and the
//! registered sources (`add_ref` / `release`); a token from `create_pylon`,
//! `create_vehicle_view`, a created `begin_vehicle` or `resolve_ref` comes
//! with one reference for the core. Tokens from queries are borrowed.
//!
//! Threads: state reads/writes use a mutex. The plugin must serialize
//! every call on one object (fc4.h).

use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::dispatch::{query_interface, DispatchHost, DispatchInfo, DispatchManager, DispatchMessage, StationUnits};
use fc4_core::stream::{RefValue, SaveObject};

use crate::guard;
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

#[repr(C)]
pub struct fc4_dispatch {
    _private: [u8; 0],
}

/// A dispatch record (fc4.h `fc4_dispatch_info`; `DispatchInfo`, §2).
/// Tokens: 0 = none.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_dispatch_info {
    pub dispatch_type: u32,
    pub id: u32,
    pub call_id: u32,
    pub view_handle: u32,
    pub active: u8,
    pub reserved: [u8; 3],
    pub target_x: f32,
    pub target_y: f32,
    pub target_z: f32,
    pub station: u64,
    pub automaton: u64,
    pub pylon: u64,
}

impl From<&DispatchInfo> for fc4_dispatch_info {
    fn from(r: &DispatchInfo) -> Self {
        fc4_dispatch_info {
            dispatch_type: r.dispatch_type,
            id: r.id,
            call_id: r.call_id,
            view_handle: r.view_handle,
            active: r.active.into(),
            reserved: [0; 3],
            target_x: r.target.0,
            target_y: r.target.1,
            target_z: r.target.2,
            station: r.station.unwrap_or(0),
            automaton: r.automaton.unwrap_or(0),
            pylon: r.pylon.unwrap_or(0),
        }
    }
}

/// One station of a source with its unit state (fc4.h
/// `fc4_station_units`): bit `i` of `mask` set = unit `i` free; `base` =
/// the call number of unit 0.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_station_units {
    pub station: u64,
    pub mask: u32,
    pub base: u32,
    pub count: u32,
    pub reserved: u32,
}

/// What the dispatch manager needs from the game (fc4.h
/// `fc4_dispatch_host`). A missing callback reads as 0 / false / none.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_dispatch_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    // messages
    pub acquire_message_server: Option<unsafe extern "C" fn(U) -> bool>,
    pub subscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub release_message_server: Option<unsafe extern "C" fn(U)>,
    /// `MessagePost` (queued): the manager, data1 = type, data2 = ID.
    pub post: Option<unsafe extern "C" fn(U, u32, u32, u32)>,
    // services and the vehicle view
    pub acquire_city: Option<unsafe extern "C" fn(U) -> bool>,
    pub release_city: Option<unsafe extern "C" fn(U)>,
    pub create_vehicle_view: Option<unsafe extern "C" fn(U) -> u64>,
    pub destroy_vehicle_view: Option<unsafe extern "C" fn(U, u64)>,
    pub refresh_vehicle_view: Option<unsafe extern "C" fn(U, u64)>,
    pub track_vehicle: Option<unsafe extern "C" fn(U, u64, u32) -> u32>,
    pub untrack_vehicle: Option<unsafe extern "C" fn(U, u32)>,
    // references and the terrain
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    pub altitude: Option<unsafe extern "C" fn(U, f32, f32) -> f32>,
    // dispatch sources (protection.md §8.2)
    pub source_can_handle: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    /// The result; `*all_busy` the out flag.
    pub source_can_dispatch: Option<unsafe extern "C" fn(U, u64, f32, f32, *mut bool) -> bool>,
    /// The station (borrowed; 0 = none) and `*call_id`.
    pub source_dispatch_from_nearest_station: Option<unsafe extern "C" fn(U, u64, f32, f32, *mut u32) -> u64>,
    /// `station` 0 = none; `*call_id` in and out.
    pub source_dispatch_from_station: Option<unsafe extern "C" fn(U, u64, u64, *mut u32) -> bool>,
    pub source_revoke_dispatch_from_station: Option<unsafe extern "C" fn(U, u64, u64, u32)>,
    /// The number of stations; copies at most `capacity` (`out` may be
    /// NULL). The core asks for the count first.
    pub source_stations: Option<unsafe extern "C" fn(U, u64, *mut fc4_station_units, usize) -> usize>,
    // vehicles
    /// The vehicle (0 = none): `info.automaton` re-targeted, or a created
    /// one with one reference for the core.
    pub begin_vehicle: Option<unsafe extern "C" fn(U, u64, *const fc4_dispatch_info) -> u64>,
    pub automaton_despawn: Option<unsafe extern "C" fn(U, u64)>,
    pub automaton_go_home: Option<unsafe extern "C" fn(U, u64)>,
    // pylons
    pub create_pylon: Option<unsafe extern "C" fn(U) -> u64>,
    pub pylon_source: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub pylon_set_source: Option<unsafe extern "C" fn(U, u64, u64)>,
    pub pylon_call_number: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub pylon_set_call_number: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub pylon_dispatch_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub pylon_set_dispatch_type: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub pylon_set_dispatch_state: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub pylon_set_ui_state: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub pylon_move_to: Option<unsafe extern "C" fn(U, u64, f32, f32, f32)>,
    pub pylon_place_at_unit: Option<unsafe extern "C" fn(U, u64, u64, u32, u32)>,
    /// `(source, pylon, station)`: true when the station's unit state was
    /// available and the pylon was placed (§4.4).
    pub pylon_return_to_station: Option<unsafe extern "C" fn(U, u64, u64, u64) -> bool>,
    pub pylon_shutdown: Option<unsafe extern "C" fn(U, u64)>,
    // save references (§7)
    /// A reference read from a record (`token`, `iid`): the object with
    /// one reference for the core, 0 = none.
    pub resolve_ref: Option<unsafe extern "C" fn(U, u64, u32) -> u64>,
    // vehicles, continued (§4.2 step 5)
    /// `(automaton, group)`: the vehicle's occupant is in that occupant group.
    pub vehicle_in_group: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
}

/// Diagnostics (`fc4_dispatch_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_dispatch_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub destroyed: u8,
    pub loaded: u8,
    pub message_server_held: u8,
    pub city_held: u8,
    pub reserved: [u8; 3],
    pub ref_count: u32,
    pub records: u32,
    pub next_id: u32,
    /// The type whose tool is open; 8 = none.
    pub tool_type: u32,
    pub available_pylons: u32,
    pub sources: u32,
    pub vehicle_view: u64,
    pub active_pylon: u64,
}

/// Number of callbacks in [`fc4_dispatch_host`].
const HOST_CALLBACKS: usize = 39;

const _: () = {
    assert!(std::mem::size_of::<fc4_dispatch_info>() == 56);
    assert!(std::mem::size_of::<fc4_station_units>() == 24);
    assert!(std::mem::size_of::<fc4_dispatch_state>() == 56);
    assert!(
        std::mem::size_of::<fc4_dispatch_host>()
            == if cfg!(target_pointer_width = "32") {
                8 + 4 * HOST_CALLBACKS
            } else {
                16 + 8 * HOST_CALLBACKS
            }
    );
};

// ---- host adapter -----------------------------------------------------------------------------------

struct CHost {
    h: fc4_dispatch_host,
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

impl DispatchHost for CHost {
    fn acquire_message_server(&self) -> bool {
        cb!(self, acquire_message_server, false)
    }
    fn subscribe(&self, msg_type: u32) {
        cb!(self, subscribe, (), msg_type)
    }
    fn unsubscribe(&self, msg_type: u32) {
        cb!(self, unsubscribe, (), msg_type)
    }
    fn release_message_server(&self) {
        cb!(self, release_message_server, ())
    }
    fn post(&self, msg_type: u32, dispatch_type: u32, id: u32) {
        cb!(self, post, (), msg_type, dispatch_type, id)
    }
    fn acquire_city(&self) -> bool {
        cb!(self, acquire_city, false)
    }
    fn release_city(&self) {
        cb!(self, release_city, ())
    }
    fn create_vehicle_view(&self) -> Option<u64> {
        opt(cb!(self, create_vehicle_view, 0))
    }
    fn destroy_vehicle_view(&self, view: u64) {
        cb!(self, destroy_vehicle_view, (), view)
    }
    fn refresh_vehicle_view(&self, view: Option<u64>) {
        cb!(self, refresh_vehicle_view, (), view.unwrap_or(0))
    }
    fn track_vehicle(&self, automaton: u64, kind: u32) -> u32 {
        cb!(self, track_vehicle, 0, automaton, kind)
    }
    fn untrack_vehicle(&self, handle: u32) {
        cb!(self, untrack_vehicle, (), handle)
    }
    fn add_ref(&self, token: u64) {
        cb!(self, add_ref, (), token)
    }
    fn release(&self, token: u64) {
        cb!(self, release, (), token)
    }
    fn altitude(&self, x: f32, z: f32) -> f32 {
        cb!(self, altitude, 0.0, x, z)
    }
    fn source_can_handle(&self, source: u64, dispatch_type: u32) -> bool {
        cb!(self, source_can_handle, false, source, dispatch_type)
    }
    fn source_can_dispatch(&self, source: u64, x: f32, z: f32) -> (bool, bool) {
        let mut busy = false;
        let r = cb!(self, source_can_dispatch, false, source, x, z, &mut busy);
        (r, busy)
    }
    fn source_dispatch_from_nearest_station(&self, source: u64, x: f32, z: f32) -> Option<(u64, u32)> {
        let mut call = 0u32;
        let st = cb!(self, source_dispatch_from_nearest_station, 0, source, x, z, &mut call);
        opt(st).map(|s| (s, call))
    }
    fn source_dispatch_from_station(&self, source: u64, station: Option<u64>, call_id: &mut u32) -> bool {
        cb!(
            self,
            source_dispatch_from_station,
            false,
            source,
            station.unwrap_or(0),
            call_id as *mut u32
        )
    }
    fn source_revoke_dispatch_from_station(&self, source: u64, station: Option<u64>, call_id: u32) {
        cb!(
            self,
            source_revoke_dispatch_from_station,
            (),
            source,
            station.unwrap_or(0),
            call_id
        )
    }
    fn source_stations(&self, source: u64) -> Vec<StationUnits> {
        let n = cb!(self, source_stations, 0, source, std::ptr::null_mut(), 0);
        if n == 0 {
            return Vec::new();
        }
        let mut v = vec![fc4_station_units::default(); n];
        let m = cb!(self, source_stations, 0, source, v.as_mut_ptr(), n);
        v.truncate(m.min(n));
        v.iter()
            .map(|s| StationUnits {
                station: s.station,
                mask: s.mask,
                base: s.base,
                count: s.count,
            })
            .collect()
    }
    fn begin_vehicle(&self, source: u64, info: &DispatchInfo) -> Option<u64> {
        let i = fc4_dispatch_info::from(info);
        opt(cb!(self, begin_vehicle, 0, source, &i))
    }
    fn automaton_despawn(&self, automaton: u64) {
        cb!(self, automaton_despawn, (), automaton)
    }
    fn automaton_go_home(&self, automaton: u64) {
        cb!(self, automaton_go_home, (), automaton)
    }
    fn create_pylon(&self) -> Option<u64> {
        opt(cb!(self, create_pylon, 0))
    }
    fn pylon_source(&self, pylon: u64) -> Option<u64> {
        opt(cb!(self, pylon_source, 0, pylon))
    }
    fn pylon_set_source(&self, pylon: u64, station: u64) {
        cb!(self, pylon_set_source, (), pylon, station)
    }
    fn pylon_call_number(&self, pylon: u64) -> u32 {
        cb!(self, pylon_call_number, 0, pylon)
    }
    fn pylon_set_call_number(&self, pylon: u64, n: u32) {
        cb!(self, pylon_set_call_number, (), pylon, n)
    }
    fn pylon_dispatch_type(&self, pylon: u64) -> u32 {
        cb!(self, pylon_dispatch_type, 0, pylon)
    }
    fn pylon_set_dispatch_type(&self, pylon: u64, t: u32) {
        cb!(self, pylon_set_dispatch_type, (), pylon, t)
    }
    fn pylon_set_dispatch_state(&self, pylon: u64, s: u32) {
        cb!(self, pylon_set_dispatch_state, (), pylon, s)
    }
    fn pylon_set_ui_state(&self, pylon: u64, s: u32) {
        cb!(self, pylon_set_ui_state, (), pylon, s)
    }
    fn pylon_move_to(&self, pylon: u64, x: f32, y: f32, z: f32) {
        cb!(self, pylon_move_to, (), pylon, x, y, z)
    }
    fn pylon_place_at_unit(&self, pylon: u64, station: u64, index: u32, count: u32) {
        cb!(self, pylon_place_at_unit, (), pylon, station, index, count)
    }
    fn pylon_return_to_station(&self, source: u64, pylon: u64, station: u64) -> bool {
        cb!(self, pylon_return_to_station, false, source, pylon, station)
    }
    fn vehicle_in_group(&self, automaton: u64, group: u32) -> bool {
        cb!(self, vehicle_in_group, false, automaton, group)
    }
    fn pylon_shutdown(&self, pylon: u64) {
        cb!(self, pylon_shutdown, (), pylon)
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
}

// ---- handles ----------------------------------------------------------------------------------------

struct Handle {
    sim: Rc<DispatchManager>,
}

fn with_p<R>(p: *const fc4_dispatch, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_dispatch*.
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
/// is copied. `token` is the manager's own token.
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_create(host: *const fc4_dispatch_host, token: u64) -> *mut fc4_dispatch {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_dispatch_host>() => {
            let c = Rc::new(CHost { h: *h });
            let sim = DispatchManager::new(c, token);
            Box::into_raw(Box::new(Handle { sim })) as *mut fc4_dispatch
        }
        _ => std::ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_destroy(p: *mut fc4_dispatch) {
    if !p.is_null() {
        guard((), || drop(Box::from_raw(p as *mut Handle)));
    }
}

/// `QueryInterface` (§1): true and the sub-object offset (0, 4 or 8) in
/// `offset` (may be NULL) for the four IIDs it answers.
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_query_interface(iid: u32, offset: *mut u32) -> bool {
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
pub unsafe extern "C" fn fc4_dispatch_add_ref(p: *mut fc4_dispatch) -> u32 {
    with_p(p, 0, |h| h.sim.add_ref())
}

/// `Release`: 0 when the object was destroyed (`Shutdown` and the
/// destructor's releases ran); the handle stays valid until
/// [`fc4_dispatch_destroy`].
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_release(p: *mut fc4_dispatch) -> u32 {
    with_p(p, 0, |h| h.sim.release())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_init(p: *mut fc4_dispatch) -> bool {
    with_p(p, false, |h| h.sim.init())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_shutdown(p: *mut fc4_dispatch) -> bool {
    with_p(p, false, |h| h.sim.shutdown())
}

/// Slot 15 `RegisterDispatchSource` (the core takes a reference).
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_register_source(p: *mut fc4_dispatch, source: u64) {
    if source != 0 {
        with_p(p, (), |h| h.sim.register_dispatch_source(source))
    }
}

/// Slot 16 `UnregisterDispatchSource`.
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_unregister_source(p: *mut fc4_dispatch, source: u64) {
    if source != 0 {
        with_p(p, (), |h| h.sim.unregister_dispatch_source(source))
    }
}

/// Slot 17 `GetDispatchSource(type)`: the first registered source that
/// handles `type` (borrowed), 0 = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_get_source(p: *const fc4_dispatch, dispatch_type: u32) -> u64 {
    with_p(p, 0, |h| h.sim.get_dispatch_source(dispatch_type).unwrap_or(0))
}

/// Slot 5 `CanDispatch(type, x, z)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_can_dispatch(p: *const fc4_dispatch, dispatch_type: u32, x: f32, z: f32) -> bool {
    with_p(p, false, |h| h.sim.can_dispatch(dispatch_type, x, z))
}

/// Slot 6 `Dispatch(type, x, z)`: the dispatch ID, 0 on failure.
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_dispatch(p: *mut fc4_dispatch, dispatch_type: u32, x: f32, z: f32) -> u32 {
    with_p(p, 0, |h| h.sim.dispatch(dispatch_type, x, z))
}

/// Slot 7 `UpdateDispatchState(id, state)` (§4.3).
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_update_state(p: *mut fc4_dispatch, id: u32, state: u32) {
    with_p(p, (), |h| h.sim.update_dispatch_state(id, state))
}

/// Slot 8 `RevokeDispatchByID`.
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_revoke_by_id(p: *mut fc4_dispatch, id: u32) -> bool {
    with_p(p, false, |h| h.sim.revoke_dispatch_by_id(id))
}

/// `RevokeDispatchByPylon`.
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_revoke_by_pylon(p: *mut fc4_dispatch, pylon: u64) -> bool {
    with_p(p, false, |h| h.sim.revoke_dispatch_by_pylon(pylon))
}

/// Slot 9 `RevokeAllDispatches(type)` (the type is not used).
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_revoke_all(p: *mut fc4_dispatch, dispatch_type: u32) -> bool {
    with_p(p, false, |h| h.sim.revoke_all_dispatches(dispatch_type))
}

/// Slot 10 `GetDispatchInfo(id)`: false when no record has that ID.
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_get_info(p: *const fc4_dispatch, id: u32, out: *mut fc4_dispatch_info) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_p(p, false, |h| match h.sim.get_dispatch_info(id) {
        Some(r) => {
            *o = fc4_dispatch_info::from(&r);
            true
        }
        None => false,
    })
}

/// Slot 11 `GetDispatchCount(type)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_get_count(p: *const fc4_dispatch, dispatch_type: u32) -> u32 {
    with_p(p, 0, |h| h.sim.get_dispatch_count(dispatch_type))
}

/// Slot 12 `GetDispatchIDArray(type, n)` (`n` 0 = no limit): returns the
/// number of IDs; copies at most `capacity` (`out` may be NULL).
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_get_id_array(
    p: *const fc4_dispatch,
    dispatch_type: u32,
    n: u32,
    out: *mut u32,
    capacity: usize,
) -> usize {
    with_p(p, 0, |h| {
        copy_out(&h.sim.get_dispatch_id_array(dispatch_type, n), out, capacity)
    })
}

/// Slot 13 `BeginDispatchTool(type)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_begin_tool(p: *mut fc4_dispatch, dispatch_type: u32) -> bool {
    with_p(p, false, |h| h.sim.begin_dispatch_tool(dispatch_type))
}

/// Slot 14 `EndDispatchTool(type)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_end_tool(p: *mut fc4_dispatch, dispatch_type: u32) -> bool {
    with_p(p, false, |h| h.sim.end_dispatch_tool(dispatch_type))
}

/// Slot 19 `SetActivePylon(p)` (0 = clear).
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_set_active_pylon(p: *mut fc4_dispatch, pylon: u64) {
    with_p(p, (), |h| h.sim.set_active_pylon(opt(pylon)))
}

/// Slot 18 `GetActivePylon` (borrowed; 0 = none).
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_get_active_pylon(p: *const fc4_dispatch) -> u64 {
    with_p(p, 0, |h| h.sim.get_active_pylon().unwrap_or(0))
}

/// Slot 20 `GetAutomatonFromPylon(p)` (borrowed; 0 = none).
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_get_automaton_from_pylon(p: *const fc4_dispatch, pylon: u64) -> u64 {
    with_p(p, 0, |h| h.sim.get_automaton_from_pylon(pylon).unwrap_or(0))
}

/// Slot 21 `GetAvailablePylonCount(type)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_get_available_pylon_count(p: *const fc4_dispatch, dispatch_type: u32) -> u32 {
    with_p(p, 0, |h| h.sim.get_available_pylon_count(dispatch_type))
}

/// Slot 22 `GetDispatchVehicleView` (borrowed; 0 = none).
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_get_vehicle_view(p: *const fc4_dispatch) -> u64 {
    with_p(p, 0, |h| h.sim.get_dispatch_vehicle_view().unwrap_or(0))
}

/// `DoMessage` (§3): `0x99EF1143` (`object` = the removed occupant),
/// `0xAA1B2002`, `0xA6B79602`, and the automaton event `0x09D92FC3`
/// (`object` = the automaton, `data2` = the code, `data3` = the state).
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_do_message(
    p: *mut fc4_dispatch,
    msg_type: u32,
    data2: u32,
    data3: u32,
    object: u64,
) -> bool {
    let m = DispatchMessage {
        msg_type,
        data2,
        data3,
        object: opt(object),
    };
    with_p(p, false, |h| h.sim.do_message(&m))
}

/// The records in list order: returns the count; copies at most
/// `capacity` (`out` may be NULL).
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_records(
    p: *const fc4_dispatch,
    out: *mut fc4_dispatch_info,
    capacity: usize,
) -> usize {
    with_p(p, 0, |h| {
        let v: Vec<fc4_dispatch_info> = h.sim.snapshot().records.iter().map(fc4_dispatch_info::from).collect();
        copy_out(&v, out, capacity)
    })
}

/// The available pylons in order (0 = a null entry read from a record):
/// returns the count; copies at most `capacity` (`out` may be NULL).
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_available_pylons(
    p: *const fc4_dispatch,
    out: *mut u64,
    capacity: usize,
) -> usize {
    with_p(p, 0, |h| {
        let v: Vec<u64> = h
            .sim
            .snapshot()
            .available_pylons
            .iter()
            .map(|e| e.unwrap_or(0))
            .collect();
        copy_out(&v, out, capacity)
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_get_state(p: *const fc4_dispatch, out: *mut fc4_dispatch_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_p(p, false, |h| {
        let s = h.sim.snapshot();
        *o = fc4_dispatch_state {
            struct_size: std::mem::size_of::<fc4_dispatch_state>() as u32,
            initialized: s.initialized.into(),
            destroyed: s.destroyed.into(),
            loaded: s.loaded.into(),
            message_server_held: s.message_server_held.into(),
            city_held: s.city_held.into(),
            reserved: [0; 3],
            ref_count: s.ref_count,
            records: s.records.len() as u32,
            next_id: s.next_id,
            tool_type: s.tool_type,
            available_pylons: s.available_pylons.len() as u32,
            sources: s.sources.len() as u32,
            vehicle_view: s.vehicle_view.unwrap_or(0),
            active_pylon: s.active_pylon.unwrap_or(0),
        };
        true
    })
}

/// `Write` (§7): references are written as host references (the token).
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_write(p: *const fc4_dispatch, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_p(p, false, |h| h.sim.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

/// `Read` (§7): false unless the version is 2. References come back
/// through the host's `resolve_ref`.
#[no_mangle]
pub unsafe extern "C" fn fc4_dispatch_read(p: *mut fc4_dispatch, inp: *const fc4_istream) -> bool {
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
