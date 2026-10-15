// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The commercial simulator and the industrial stub over the C ABI (fc4.h
//! "commercial and industrial", docs/specs/commercial-industrial.md).
//!
//! The host implements [`fc4_commercial_host`]; the plugin's GZCOM objects
//! (`cISC4CommercialSimulator` with its message-target and serializable
//! sub-objects, `cISC4IndustrialSimulator`) forward their slots here.
//!
//! Tokens are opaque host identities: occupants, the two demand nodes (one
//! reference each for the core) and grids. The two proximity grids are the
//! game's own `cSC4SimGrid<uint8>` objects made through
//! [`fc4_grid_api`](crate::hostgrid::fc4_grid_api) (the core holds one
//! reference on each), so `GetProximityMap` can hand out the game's
//! interface pointer: [`fc4_commercial_proximity_map`] returns the grid's
//! token. The building map is read through the same table without a
//! reference (a borrowed view).
//!
//! Threads: the core is single-threaded (`RefCell`). The plugin serialises
//! every call on one object (fc4.h).

use std::ffi::{c_char, c_void, CString};
use std::rc::Rc;

use fc4_core::commercial::{query_interface, CommercialHost, CommercialMessage, CommercialSim, IndustrialSim};
use fc4_core::grid::{GridProvider, TractGrid};
use fc4_core::stream::{RefValue, SaveObject};

use crate::guard;
use crate::hostgrid::{create_grid, fc4_grid_api, grid_token, resolve_grid};
use crate::residential::BorrowedGrid;
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

#[repr(C)]
pub struct fc4_commercial {
    _private: [u8; 0],
}

#[repr(C)]
pub struct fc4_industrial {
    _private: [u8; 0],
}

/// What the commercial simulator needs from the game (fc4.h
/// `fc4_commercial_host`). A missing callback reads as 0 / false / none.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_commercial_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    pub add_agent: Option<unsafe extern "C" fn(U, u32, *const c_char, u32)>,
    pub remove_agent: Option<unsafe extern "C" fn(U)>,
    pub acquire_message_server: Option<unsafe extern "C" fn(U) -> bool>,
    pub subscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub release_message_server: Option<unsafe extern "C" fn(U)>,
    pub acquire_city: Option<unsafe extern "C" fn(U) -> bool>,
    pub release_city: Option<unsafe extern "C" fn(U)>,
    pub acquire_bds: Option<unsafe extern "C" fn(U) -> bool>,
    pub release_bds: Option<unsafe extern "C" fn(U)>,
    pub has_demand_simulator: Option<unsafe extern "C" fn(U) -> bool>,
    pub demand_node: Option<unsafe extern "C" fn(U, u32) -> u64>,
    pub release_demand_node: Option<unsafe extern "C" fn(U, u64)>,
    pub building_map: Option<unsafe extern "C" fn(U) -> u64>,
    pub building_map_key: Option<unsafe extern "C" fn(U, u32, u32) -> u8>,
    pub building_purpose: Option<unsafe extern "C" fn(U, u64, *mut u32) -> bool>,
    pub grids: fc4_grid_api,
}

/// Diagnostics (`fc4_commercial_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_commercial_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub dirty: u8,
    pub city_held: u8,
    pub bds_held: u8,
    pub message_server_held: u8,
    pub destroyed: u8,
    /// The saved flag byte (§8.1): `dirty || state ≠ 3`.
    pub flag: u8,
    pub reserved0: u8,
    pub state: i32,
    pub ref_count: u32,
    pub reserved1: u32,
    pub demand_nodes: [u64; 2],
    /// The CO$$ / CO$$$ grid tokens (0 = none).
    pub co2_grid: u64,
    pub co3_grid: u64,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_commercial_state>() == 56);
    assert!(std::mem::size_of::<fc4_commercial_host>() == if cfg!(target_pointer_width = "32") { 112 } else { 224 });
};

// ---- host adapter -----------------------------------------------------------------------------------

struct CHost {
    h: fc4_commercial_host,
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

impl GridProvider<u8> for CHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        create_grid::<u8>(&self.h.grids, instance, tract_size, false)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u8>>> {
        resolve_grid::<u8>(&self.h.grids, r, false)
    }
}

impl CommercialHost for CHost {
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        let n = CString::new(name).unwrap_or_default();
        cb!(self, add_agent, (), level, n.as_ptr(), flags)
    }
    fn remove_agent(&self) {
        cb!(self, remove_agent, ())
    }
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
    fn acquire_city(&self) -> bool {
        cb!(self, acquire_city, false)
    }
    fn release_city(&self) {
        cb!(self, release_city, ())
    }
    fn acquire_bds(&self) -> bool {
        cb!(self, acquire_bds, false)
    }
    fn release_bds(&self) {
        cb!(self, release_bds, ())
    }
    fn has_demand_simulator(&self) -> bool {
        cb!(self, has_demand_simulator, false)
    }
    fn demand_node(&self, demand_id: u32) -> Option<u64> {
        opt(cb!(self, demand_node, 0, demand_id))
    }
    fn release_demand_node(&self, node: u64) {
        cb!(self, release_demand_node, (), node)
    }
    fn shutdown_grid(&self, grid: &Rc<dyn TractGrid<u8>>) {
        if let RefValue::Host { token, .. } = grid.save_ref() {
            if token != 0 {
                if let Some(f) = self.h.grids.shutdown {
                    // SAFETY: valid callback; the token is a grid the core holds.
                    unsafe { f(self.h.grids.user, token) };
                }
            }
        }
    }
    fn building_map(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        BorrowedGrid::<u8>::wrap(self.h.grids, cb!(self, building_map, 0))
    }
    fn building_map_key(&self, purpose: u32, wealth: u32) -> u8 {
        cb!(self, building_map_key, 0, purpose, wealth)
    }
    fn building_purpose(&self, occupant: u64) -> Option<u32> {
        let mut p = 0u32;
        cb!(self, building_purpose, false, occupant, &mut p).then_some(p)
    }
}

// ---- the commercial handle --------------------------------------------------------------------------

struct Handle {
    sim: Rc<CommercialSim>,
}

fn with_c<R>(c: *const fc4_commercial, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_commercial*.
    match unsafe { (c as *const Handle).as_ref() } {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

/// Creates the core object (constructed, not initialised; reference count
/// 0). The host table is copied; NULL or a short table gives NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_commercial_create(host: *const fc4_commercial_host) -> *mut fc4_commercial {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_commercial_host>() && h.grids.complete() => {
            let sim = CommercialSim::new(Rc::new(CHost { h: *h }), 0);
            Box::into_raw(Box::new(Handle { sim })) as *mut fc4_commercial
        }
        _ => std::ptr::null_mut(),
    })
}

/// Frees the handle (and the grids' references if they are still held).
#[no_mangle]
pub unsafe extern "C" fn fc4_commercial_destroy(c: *mut fc4_commercial) {
    if !c.is_null() {
        guard((), || drop(Box::from_raw(c as *mut Handle)));
    }
}

/// `QueryInterface` (§1): the IIDs the object answers.
#[no_mangle]
pub extern "C" fn fc4_commercial_query_interface(iid: u32) -> bool {
    query_interface(iid)
}

#[no_mangle]
pub unsafe extern "C" fn fc4_commercial_add_ref(c: *mut fc4_commercial) -> u32 {
    with_c(c, 0, |h| h.sim.add_ref())
}

/// `Release` (§2.3): 0 when the object was destroyed (`Shutdown` ran); the
/// handle stays valid until [`fc4_commercial_destroy`].
#[no_mangle]
pub unsafe extern "C" fn fc4_commercial_release(c: *mut fc4_commercial) -> u32 {
    with_c(c, 0, |h| h.sim.release())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_commercial_init(c: *mut fc4_commercial) -> bool {
    with_c(c, false, |h| h.sim.init())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_commercial_shutdown(c: *mut fc4_commercial) -> bool {
    with_c(c, false, |h| h.sim.shutdown())
}

/// `DoMessage` (§3): the idle and day messages run one `DoIdle` step; an
/// occupant insert / remove (data3 = occupant type, void1 = occupant token)
/// may mark the maps dirty. Always true.
#[no_mangle]
pub unsafe extern "C" fn fc4_commercial_do_message(
    c: *mut fc4_commercial,
    msg_type: u32,
    data1: u32,
    data2: u32,
    data3: u32,
    void1: u64,
) -> bool {
    let m = CommercialMessage {
        msg_type,
        data1,
        data2,
        data3,
        void1: opt(void1),
    };
    with_c(c, true, |h| h.sim.do_message(&m))
}

/// One `DoIdle` step (§4.3).
#[no_mangle]
pub unsafe extern "C" fn fc4_commercial_do_idle(c: *mut fc4_commercial) {
    with_c(c, (), |h| h.sim.do_idle())
}

/// Tooling (the commercial shadow, ABI 58): sets the `DoIdle` machine to the
/// original's `dirty` and `state` (§2.1, §4.3). False for a state outside
/// 0..3, which changes nothing.
#[no_mangle]
pub unsafe extern "C" fn fc4_commercial_set_machine(c: *mut fc4_commercial, dirty: bool, state: i32) -> bool {
    if !(0..=3).contains(&state) {
        return false;
    }
    with_c(c, false, |h| {
        h.sim.with_state(|s| {
            s.dirty = dirty;
            s.state = state;
        });
        true
    })
}

/// Slot 5 `GetProximityMap(developerType)` (§4.6): the grid's token (the
/// game's `cISC4SimGrid<uint8>`), 0 for any other type or no grid. No
/// reference is added.
#[no_mangle]
pub unsafe extern "C" fn fc4_commercial_proximity_map(c: *const fc4_commercial, developer_type: u32) -> u64 {
    with_c(c, 0, |h| grid_token::<u8>(&h.sim.proximity_map(developer_type)))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_commercial_get_state(c: *const fc4_commercial, out: *mut fc4_commercial_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_c(c, false, |h| {
        let s = h.sim.snapshot();
        *o = fc4_commercial_state {
            struct_size: std::mem::size_of::<fc4_commercial_state>() as u32,
            initialized: s.initialized.into(),
            dirty: s.dirty.into(),
            city_held: s.city_held.into(),
            bds_held: s.bds_held.into(),
            message_server_held: s.message_server_held.into(),
            destroyed: s.destroyed.into(),
            flag: s.flag().into(),
            reserved0: 0,
            state: s.state,
            ref_count: s.ref_count,
            reserved1: 0,
            demand_nodes: [s.demand_nodes[0].unwrap_or(0), s.demand_nodes[1].unwrap_or(0)],
            co2_grid: grid_token::<u8>(&s.co2),
            co3_grid: grid_token::<u8>(&s.co3),
        };
        true
    })
}

/// `Write` (§8.1): 19 bytes when both grids exist.
#[no_mangle]
pub unsafe extern "C" fn fc4_commercial_write(c: *const fc4_commercial, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_c(c, false, |h| h.sim.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

/// `Read` (§8.1): false unless the version is 1. The grid references are
/// read with IID `0xC9BD939A`; each one the stream hands over becomes the
/// core's reference.
#[no_mangle]
pub unsafe extern "C" fn fc4_commercial_read(c: *mut fc4_commercial, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_c(c, false, |h| {
        h.sim.read(&mut CIn {
            stream: i,
            decode: no_core_objects,
        })
    })
}

// ---- the industrial stub ----------------------------------------------------------------------------

fn with_i<R>(p: *const fc4_industrial, fallback: R, f: impl FnOnce(&IndustrialSim) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_industrial*.
    match unsafe { (p as *const Rc<IndustrialSim>).as_ref() } {
        Some(s) => guard(fallback, || f(s)),
        None => fallback,
    }
}

/// Creates the industrial stub (constructed, reference count 0).
#[no_mangle]
pub extern "C" fn fc4_industrial_create() -> *mut fc4_industrial {
    guard(std::ptr::null_mut(), || {
        Box::into_raw(Box::new(IndustrialSim::new(0))) as *mut fc4_industrial
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_industrial_destroy(p: *mut fc4_industrial) {
    if !p.is_null() {
        guard((), || drop(Box::from_raw(p as *mut Rc<IndustrialSim>)));
    }
}

#[no_mangle]
pub extern "C" fn fc4_industrial_query_interface(iid: u32) -> bool {
    IndustrialSim::query_interface(iid)
}

#[no_mangle]
pub unsafe extern "C" fn fc4_industrial_add_ref(p: *mut fc4_industrial) -> u32 {
    with_i(p, 0, |s| s.add_ref())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_industrial_release(p: *mut fc4_industrial) -> u32 {
    with_i(p, 0, |s| s.release())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_industrial_init(p: *mut fc4_industrial) -> bool {
    with_i(p, false, |s| s.init())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_industrial_shutdown(p: *mut fc4_industrial) -> bool {
    with_i(p, false, |s| s.shutdown())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_industrial_do_message(p: *mut fc4_industrial, msg_type: u32) -> bool {
    with_i(p, true, |s| s.do_message(msg_type))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_industrial_is_initialized(p: *const fc4_industrial) -> bool {
    with_i(p, false, |s| s.snapshot().initialized)
}

/// `Write` (§8.2): `01 00`.
#[no_mangle]
pub unsafe extern "C" fn fc4_industrial_write(p: *const fc4_industrial, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_i(p, false, |s| s.write(&mut COut(o)))
}

/// `Read` (§8.2): false unless the version is 1.
#[no_mangle]
pub unsafe extern "C" fn fc4_industrial_read(p: *mut fc4_industrial, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_i(p, false, |s| {
        s.read(&mut CIn {
            stream: i,
            decode: no_core_objects,
        })
    })
}
