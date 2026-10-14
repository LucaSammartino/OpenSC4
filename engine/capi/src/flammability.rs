// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The flammability simulator's C ABI (fc4.h "flammability",
//! docs/specs/flammability.md).
//!
//! The host implements [`fc4_flammability_host`] (every `FlammabilityHost`
//! method except the grids, which go through the shared [`fc4_grid_api`]); the
//! plugin's `cISC4FlammabilitySimulator` forwards its nine slots to the
//! `fc4_flammability_*` functions.
//!
//! Tokens are opaque host identities (fc4.h): occupants and lots. 0 is the
//! null object.
//!
//! The agent registration (§4.1 step 2) and the six notifications (step 3) are
//! the plugin's, as in every other subsystem: the core asks for neither.
//!
//! Re-entrancy: the core borrows its state only for short sections, but a
//! re-entrant `DoMessage` during a daily simulation would still corrupt the
//! cycle. [choice] As in power and plumbing, a message delivered while a call
//! is running is queued and handled when the outermost call returns, and other
//! re-entrant calls are counted (`fc4_flammability_state::reentrant_calls`).

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::flammability::{
    CellRect, FlammabilityHost, FlammabilityRandom, FlammabilityRng, FlammabilitySim, Message,
};
use fc4_core::grid::{GridProvider, TractGrid};
use fc4_core::property::PropertyHolder;
use fc4_core::rng::FixedSeeds;
use fc4_core::stream::{RefValue, SaveObject};
use fc4_core::variant::Variant;

use crate::budget::fc4_token_callback;
use crate::guard;
use crate::hostgrid::{
    collect_tokens, create_grid, fc4_cell_rect, fc4_grid_api, grid_token, read_property, resolve_grid, take_props,
};
use crate::props::fc4_props;
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

/// A message for `fc4_flammability_do_message` (fc4.h
/// `fc4_flammability_message`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_flammability_message {
    pub msg_type: u32,
    pub reserved: u32,
    /// `GetVoid1()` as a `cISC4Occupant`.
    pub occupant: u64,
    /// `GetIGZUnknown()` queried for `cISC4Lot`.
    pub lot: u64,
}

/// Diagnostics (fc4.h `fc4_flammability_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_flammability_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub pad0: [u8; 3],
    pub count_x: i32,
    pub count_z: i32,
    pub average: u8,
    pub pad1: [u8; 3],
    pub global_multiplier: f32,
    pub zoned_sum: i32,
    pub zoned_count: i32,
    pub candidates: u32,
    pub hotspots: u32,
    pub tract_size: i32,
    pub cycle_period: i32,
    pub tracts_per_day: i32,
    pub reserved: u32,
    pub adjusted_grid: u64,
    pub raw_grid: u64,
    pub reentrant_calls: u32,
    pub reserved2: u32,
}

/// Everything the flammability simulator asks of the game (fc4.h
/// `fc4_flammability_host`; one callback per `FlammabilityHost` method other
/// than the grids, in the trait's order).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_flammability_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    pub grids: fc4_grid_api,
    // city
    pub city_cell_counts: Option<unsafe extern "C" fn(U, *mut i32, *mut i32)>,
    pub position_to_cell: Option<unsafe extern "C" fn(U, f32, f32, *mut i32, *mut i32)>,
    pub is_tutorial: Option<unsafe extern "C" fn(U) -> bool>,
    /// A NEW fc4_props (the core destroys it), NULL if the exemplar is missing.
    pub tuning: Option<unsafe extern "C" fn(U) -> *mut fc4_props>,
    // occupants
    pub occupant_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub occupant_property: Option<unsafe extern "C" fn(U, u64, u32, *mut fc4_props) -> bool>,
    pub occupant_bounds: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    pub occupant_position: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    pub occupants_in_cells: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, fc4_token_callback, *mut c_void)>,
    // lots
    pub occupant_lot: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub lot_condition: Option<unsafe extern "C" fn(U, u64) -> u8>,
    pub lot_building: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub random_lot: Option<unsafe extern "C" fn(U) -> u64>,
    // other subsystems
    pub cumulative_factor: Option<unsafe extern "C" fn(U, u32, u32, *mut f32) -> bool>,
    pub current_season: Option<unsafe extern "C" fn(U, *mut i32) -> bool>,
    /// -1 = no plumbing simulator, 0 = not watered, 1 = watered.
    pub is_cell_watered: Option<unsafe extern "C" fn(U, i32, i32) -> i32>,
    pub zone_type: Option<unsafe extern "C" fn(U, i32, i32, *mut u32) -> bool>,
    pub record_history: Option<unsafe extern "C" fn(U, u32, i32)>,
    // fire
    pub ignite_cell: Option<unsafe extern "C" fn(U, u32, u32, bool, bool, u32) -> bool>,
    pub set_fire_cause: Option<unsafe extern "C" fn(U, u32)>,
    // messages
    pub post: Option<unsafe extern "C" fn(U, u32)>,
    /// Plumbing slot 7 `GetWateredGrid()`: the published watered grid as a
    /// token (0 = none). Its object alternates at every plumbing pass
    /// (plumbing §2), so a change tells a pass ended (flammability.md §6.5,
    /// FL5/FL6). Read by the flammability shadow only; NULL = not polled.
    pub watered_grid: Option<unsafe extern "C" fn(U) -> u64>,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_flammability_message>() == 24);
    assert!(std::mem::size_of::<fc4_flammability_state>() == 80);
    assert!(std::mem::size_of::<fc4_flammability_host>() == if cfg!(target_pointer_width = "32") { 136 } else { 272 });
};

// ---- host adapter -----------------------------------------------------------------------------------

pub(crate) struct CHost(pub(crate) fc4_flammability_host);

impl CHost {
    /// The plumbing simulator's published watered grid (FL5/FL6); `None`
    /// when the host does not answer it.
    pub(crate) fn watered_grid(&self) -> Option<u64> {
        // SAFETY: whoever created the object promised valid callbacks.
        self.0.watered_grid.map(|f| unsafe { f(self.0.user) })
    }
}

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.0.$f { Some(f) => unsafe { f($s.0.user $(, $arg)*) }, None => $default }
    };
}

fn opt(t: u64) -> Option<u64> {
    (t != 0).then_some(t)
}

/// The core's inclusive cell rectangle is a plain tuple, so the ABI's
/// `fc4_cell_rect` maps field by field.
fn rect_of(r: fc4_cell_rect) -> CellRect {
    (r.x1, r.z1, r.x2, r.z2)
}

impl GridProvider<u8> for CHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        create_grid::<u8>(&self.0.grids, instance, tract_size, true)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u8>>> {
        resolve_grid::<u8>(&self.0.grids, r, true)
    }
}

impl FlammabilityHost for CHost {
    fn city_cell_counts(&self) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        cb!(self, city_cell_counts, (), &mut x, &mut z);
        (x, z)
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        let (mut cx, mut cz) = (0, 0);
        cb!(self, position_to_cell, (), x, z, &mut cx, &mut cz);
        (cx, cz)
    }
    fn is_tutorial(&self) -> bool {
        cb!(self, is_tutorial, false)
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, tuning, std::ptr::null_mut());
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }
    fn occupant_type(&self, occ: u64) -> u32 {
        cb!(self, occupant_type, 0, occ)
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant> {
        let f = self.0.occupant_property?;
        // SAFETY: valid callback; the scratch holder outlives the call.
        read_property(id, |p| unsafe { f(self.0.user, occ, id, p) })
    }
    fn occupant_bounds(&self, occ: u64) -> CellRect {
        let mut r = fc4_cell_rect::default();
        cb!(self, occupant_bounds, (), occ, &mut r);
        rect_of(r)
    }
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        let mut p = [0.0f32; 3];
        cb!(self, occupant_position, (), occ, p.as_mut_ptr());
        p
    }
    fn occupants_in_cells(&self, rect: CellRect) -> Vec<u64> {
        let r = fc4_cell_rect {
            x1: rect.0,
            z1: rect.1,
            x2: rect.2,
            z2: rect.3,
        };
        collect_tokens(|sink, ctx| cb!(self, occupants_in_cells, (), &r, sink, ctx))
    }
    fn occupant_lot(&self, occ: u64) -> Option<u64> {
        opt(cb!(self, occupant_lot, 0, occ))
    }
    fn lot_condition(&self, lot: u64) -> u8 {
        cb!(self, lot_condition, 0, lot)
    }
    fn lot_building(&self, lot: u64) -> Option<u64> {
        opt(cb!(self, lot_building, 0, lot))
    }
    fn random_lot(&self) -> Option<u64> {
        opt(cb!(self, random_lot, 0))
    }
    fn cumulative_factor(&self, prop: u32, index: u32) -> Option<f32> {
        let mut v = 0.0f32;
        cb!(self, cumulative_factor, false, prop, index, &mut v).then_some(v)
    }
    fn current_season(&self) -> Option<i32> {
        let mut s = 0i32;
        cb!(self, current_season, false, &mut s).then_some(s)
    }
    fn is_cell_watered(&self, x: i32, z: i32) -> Option<bool> {
        // -1 = no plumbing simulator (§4.1 step 5 [choice]).
        match cb!(self, is_cell_watered, -1, x, z) {
            -1 => None,
            v => Some(v != 0),
        }
    }
    fn zone_type(&self, x: i32, z: i32) -> Option<u32> {
        let mut t = 0u32;
        cb!(self, zone_type, false, x, z, &mut t).then_some(t)
    }
    fn record_history(&self, history_type: u32, value: i32) {
        cb!(self, record_history, (), history_type, value)
    }
    fn ignite_cell(&self, x: u32, z: u32, use_probability: bool, teleport: bool, fire_iid: u32) -> bool {
        cb!(self, ignite_cell, false, x, z, use_probability, teleport, fire_iid)
    }
    fn set_fire_cause(&self, cause: u32) {
        cb!(self, set_fire_cause, (), cause)
    }
    fn post(&self, msg_type: u32) {
        cb!(self, post, (), msg_type)
    }
    fn grid_shutdown(&self, grid: &Rc<dyn TractGrid<u8>>) {
        if let Some(f) = self.0.grids.shutdown {
            if let RefValue::Host { token, .. } = grid.save_ref() {
                if token != 0 {
                    // SAFETY: valid callback; the token is the game's grid.
                    unsafe { f(self.0.grids.user, token) };
                }
            }
        }
    }
}

// ---- handles ----------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_flammability`).
#[repr(C)]
pub struct fc4_flammability {
    _private: [u8; 0],
}

struct Handle {
    sim: Rc<FlammabilitySim>,
    depth: Cell<u32>,
    queue: RefCell<Vec<Message>>,
    reentrant: Cell<u32>,
}

impl Handle {
    /// Runs `f` with the call depth counted; at the end of the outermost call,
    /// the queued messages are handled.
    fn call<R>(&self, f: impl FnOnce(&FlammabilitySim) -> R) -> R {
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
fn with_flam<R>(p: *const fc4_flammability, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_flammability*.
    let Some(h) = (unsafe { (p as *const Handle).as_ref() }) else {
        return fallback;
    };
    guard(fallback, || f(h))
}

fn sim<R>(p: *const fc4_flammability, fallback: R, f: impl FnOnce(&FlammabilitySim) -> R) -> R {
    with_flam(p, fallback, |h| h.call(f))
}

fn message_of(m: &fc4_flammability_message) -> Message {
    Message {
        msg_type: m.msg_type,
        occupant: opt(m.occupant),
        lot: opt(m.lot),
    }
}

/// Creates the core object (constructor values, §3). The host table is copied.
/// `token` is the host identity of the simulator itself; the two clock values
/// seed its own `cRZRandom` (§7, foundations §6.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_flammability_create(
    host: *const fc4_flammability_host,
    token: u64,
    wall_clock_seed: u32,
    process_clock: u32,
) -> *mut fc4_flammability {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_flammability_host>() && h.grids.complete() => {
            // The simulator holds the host, which is where every grid callback
            // goes, so the handle needs no reference of its own.
            let host = Rc::new(CHost(*h));
            let seeds = FixedSeeds {
                wall_clock: wall_clock_seed,
                process_clock,
            };
            let rng: Box<dyn FlammabilityRandom> = Box::new(FlammabilityRng::new(&seeds));
            let sim = FlammabilitySim::new(host, token, rng);
            Box::into_raw(Box::new(Handle {
                sim,
                depth: Cell::new(0),
                queue: RefCell::new(Vec::new()),
                reentrant: Cell::new(0),
            })) as *mut fc4_flammability
        }
        _ => std::ptr::null_mut(),
    })
}

/// Destroys the core object. Both grids' references are released by the
/// `HostGrid` drop (which also shuts each one down).
#[no_mangle]
pub unsafe extern "C" fn fc4_flammability_destroy(p: *mut fc4_flammability) {
    if p.is_null() {
        return;
    }
    guard((), || drop(Box::from_raw(p as *mut Handle)));
}

/// `Init` (§4.1, core part: steps 1, 4, 6-9). Always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_flammability_init(p: *mut fc4_flammability) -> bool {
    sim(p, false, FlammabilitySim::init)
}

/// `Shutdown` (§4.2, core part). Always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_flammability_shutdown(p: *mut fc4_flammability) -> bool {
    sim(p, false, FlammabilitySim::shutdown)
}

/// `DoMessage` (§5): always true for a live object. A message delivered during
/// another call is queued.
#[no_mangle]
pub unsafe extern "C" fn fc4_flammability_do_message(p: *mut fc4_flammability, m: *const fc4_flammability_message) -> bool {
    let Some(m) = m.as_ref() else { return true };
    let msg = message_of(m);
    with_flam(p, true, |h| {
        if h.depth.get() > 0 {
            h.queue.borrow_mut().push(msg);
            return true;
        }
        h.call(|s| s.do_message(&msg))
    })
}

/// Slot 6 `IsCellFlammable(x, z)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_flammability_is_cell_flammable(p: *const fc4_flammability, x: i32, z: i32) -> bool {
    sim(p, false, |s| s.is_cell_flammable(x, z))
}

/// Slot 7 `GetFlammabilityHotspot(&value, &x, &z)`: false = no hotspot (or its
/// tract value is 0), and no output is written (§13.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_flammability_hotspot(
    p: *const fc4_flammability,
    value: *mut u8,
    x: *mut i32,
    z: *mut i32,
) -> bool {
    let Some(h) = sim(p, None, FlammabilitySim::flammability_hotspot) else { return false };
    if let Some(v) = value.as_mut() {
        *v = h.0;
    }
    if let Some(cx) = x.as_mut() {
        *cx = h.1;
    }
    if let Some(cz) = z.as_mut() {
        *cz = h.2;
    }
    true
}

/// Slot 8 `GetAverageFlammability()`.
#[no_mangle]
pub unsafe extern "C" fn fc4_flammability_average(p: *const fc4_flammability) -> u8 {
    sim(p, 0, FlammabilitySim::average_flammability)
}

/// Slot 5 `GetFlammabilityGrid()`: the adjusted grid's token (borrowed; 0 =
/// none).
#[no_mangle]
pub unsafe extern "C" fn fc4_flammability_grid(p: *const fc4_flammability) -> u64 {
    sim(p, 0, |s| grid_token(&s.adjusted_grid()))
}

/// Diagnostics.
#[no_mangle]
pub unsafe extern "C" fn fc4_flammability_get_state(
    p: *const fc4_flammability,
    out: *mut fc4_flammability_state,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    let Some(h) = (p as *const Handle).as_ref() else { return false };
    guard(false, || {
        let d = h.sim.snapshot();
        let reentrant = h.reentrant.get();
        *o = fc4_flammability_state {
            struct_size: std::mem::size_of::<fc4_flammability_state>() as u32,
            initialized: u8::from(d.initialized),
            pad0: [0; 3],
            count_x: d.count_x,
            count_z: d.count_z,
            average: d.average,
            pad1: [0; 3],
            global_multiplier: d.global_multiplier,
            zoned_sum: d.zoned_sum,
            zoned_count: d.zoned_count,
            candidates: d.candidates.len() as u32,
            hotspots: d.hotspots.len() as u32,
            tract_size: d.tuning.tract_size,
            cycle_period: d.tuning.cycle_period,
            tracts_per_day: d.tracts_per_day,
            reserved: 0,
            adjusted_grid: grid_token(&h.sim.adjusted_grid()),
            raw_grid: grid_token(&h.sim.raw_grid()),
            reentrant_calls: reentrant,
            reserved2: 0,
        };
        true
    })
}

/// `Write` (§10). Both grid references go out as `fc4_ref`s with the token and
/// the grid CLSID.
#[no_mangle]
pub unsafe extern "C" fn fc4_flammability_write(p: *const fc4_flammability, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    sim(p, false, |s| s.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

/// `Read` (§10): false only for a version other than 2.
#[no_mangle]
pub unsafe extern "C" fn fc4_flammability_read(p: *mut fc4_flammability, inp: *const fc4_istream) -> bool {
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
