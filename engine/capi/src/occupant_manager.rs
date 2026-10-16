// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The occupant manager over the C ABI (fc4.h "occupant manager",
//! docs/specs/occupant-manager.md).
//!
//! The host implements [`fc4_occmgr_host`]: the occupants (and the caller's
//! occupant filters) are opaque host tokens, 0 = null. The plugin's
//! `cISC4OccupantManager` (with its serializable and message-target views)
//! forwards its slots here. The coordinate conversions of slots 10–19 need no
//! object (`fc4_occ_*`).
//!
//! Threads (§14): each handle owns a recursive lock that the core enters at
//! exactly the points of the §14 table ([`OccupantHost::lock`]). Every entry
//! point may be called from any thread; the host's callbacks must be safe to
//! call from the threads that call in. Callbacks may call back into the
//! manager (the lock is recursive and the core holds no borrow across a host
//! call); slot 42's callbacks run **without** the lock, slot 44's with it.

use std::ffi::c_void;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Condvar, Mutex};
use std::thread::ThreadId;

use fc4_core::occupant_manager::{
    self as om, query_interface, BBox, CellBox, Filter, Interface, OccupantHost, OccupantManager, Vec3,
};
use fc4_core::stream::{RefValue, SaveObject};

use crate::guard;
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

#[repr(C)]
pub struct fc4_occmgr {
    _private: [u8; 0],
}

/// An occupant's bounding box (occupant slot 10): x, y, z.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_occ_bbox {
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// An occupant's manager box (§4.3): world cells, inclusive, one byte each.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_occ_cell_box {
    pub x0: u8,
    pub z0: u8,
    pub x1: u8,
    pub z1: u8,
}

/// `fc4_occ_filter.kind`: a null filter (everything is included).
pub const FC4_OCC_FILTER_ALL: u32 = 0;
/// A caller's `cISC4OccupantFilter` (`token`, 0 = null): the host's
/// `filter_includes` decides.
pub const FC4_OCC_FILTER_HOST: u32 = 1;
/// The manager's shared one-type filter with `occ_type` (§4.4).
pub const FC4_OCC_FILTER_TYPE: u32 = 2;

/// A query's filter argument. A NULL pointer is a null filter.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_occ_filter {
    pub kind: u32,
    pub occ_type: u32,
    pub token: u64,
}

/// What the manager needs from the game (fc4.h `fc4_occmgr_host`). A missing
/// callback reads as 0 / false / none.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_occmgr_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    pub init: Option<unsafe extern "C" fn(U, u64)>,
    pub shutdown: Option<unsafe extern "C" fn(U, u64)>,
    pub has_property_holder: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub occupant_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub bounding_box: Option<unsafe extern "C" fn(U, u64, *mut fc4_occ_bbox)>,
    pub post_message: Option<unsafe extern "C" fn(U, u64, u32, u32)>,
    pub manager_bbox: Option<unsafe extern "C" fn(U, u64, *mut fc4_occ_cell_box) -> bool>,
    pub set_manager_bbox: Option<unsafe extern "C" fn(U, u64, *const fc4_occ_cell_box)>,
    pub set_flag: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub clear_flag: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub is_flag_set: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub filter_includes: Option<unsafe extern "C" fn(U, u64, u64) -> bool>,
    pub resolve_ref: Option<unsafe extern "C" fn(U, u64, u32) -> u64>,
}

/// Diagnostics (`fc4_occmgr_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_occmgr_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub was_read: u8,
    pub destroyed: u8,
    /// The shared filter exists (§4.4).
    pub filter_present: u8,
    pub ref_count: u32,
    /// The shared filter's type (0 while it is null).
    pub filter_type: u32,
    pub size_x: f32,
    pub size_z: f32,
    /// Manager cell counts X, Z, standard cell counts X, Z.
    pub counts: [i32; 4],
    /// Grid rows, cells with a list holder, list nodes.
    pub rows: u32,
    pub holders: u32,
    pub nodes: u32,
    /// Times a thread had to wait for the manager's lock.
    pub lock_waits: u32,
}

/// A list query's output: called once per appended occupant, in order,
/// after the query returns (each one holds a reference for the caller).
pub type fc4_occ_list_sink = Option<unsafe extern "C" fn(U, u64)>;
/// An iteration callback: false stops the walk.
pub type fc4_occ_iterate_fn = Option<unsafe extern "C" fn(U, u64) -> bool>;

const _: () = {
    assert!(std::mem::size_of::<fc4_occ_bbox>() == 24);
    assert!(std::mem::size_of::<fc4_occ_cell_box>() == 4);
    assert!(std::mem::size_of::<fc4_occ_filter>() == 16);
    assert!(std::mem::size_of::<fc4_occmgr_state>() == 56);
    assert!(std::mem::size_of::<fc4_occmgr_host>() == if cfg!(target_pointer_width = "32") { 68 } else { 136 });
};

// ---- the recursive lock (§14) -----------------------------------------------------------------------

/// A recursive lock entered and left by separate calls (the core's
/// `lock` / `unlock` hooks).
#[derive(Default)]
struct RecursiveLock {
    state: Mutex<(Option<ThreadId>, u32)>,
    cv: Condvar,
    waits: AtomicU32,
}

impl RecursiveLock {
    fn enter(&self) {
        let me = std::thread::current().id();
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let mut waited = false;
        loop {
            match s.0 {
                None => {
                    *s = (Some(me), 1);
                    return;
                }
                Some(owner) if owner == me => {
                    s.1 += 1;
                    return;
                }
                Some(_) => {
                    if !waited {
                        waited = true;
                        self.waits.fetch_add(1, Ordering::Relaxed);
                    }
                    s = self.cv.wait(s).unwrap_or_else(|e| e.into_inner());
                }
            }
        }
    }

    fn leave(&self) {
        let me = std::thread::current().id();
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.0 != Some(me) {
            return;
        }
        s.1 = s.1.saturating_sub(1);
        if s.1 == 0 {
            s.0 = None;
            self.cv.notify_one();
        }
    }

    /// The calling thread's depth.
    fn depth_here(&self) -> u32 {
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if s.0 == Some(std::thread::current().id()) {
            s.1
        } else {
            0
        }
    }
}

// ---- host adapter -----------------------------------------------------------------------------------

struct CHost {
    h: fc4_occmgr_host,
    lock: RecursiveLock,
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

fn box_from_c(b: &fc4_occ_cell_box) -> CellBox {
    CellBox::new(b.x0, b.z0, b.x1, b.z1)
}

impl OccupantHost for CHost {
    fn lock(&self) {
        self.lock.enter();
    }
    fn unlock(&self) {
        self.lock.leave();
    }
    fn add_ref(&self, occ: u64) {
        cb!(self, add_ref, (), occ)
    }
    fn release(&self, occ: u64) {
        cb!(self, release, (), occ)
    }
    fn init(&self, occ: u64) {
        cb!(self, init, (), occ)
    }
    fn shutdown(&self, occ: u64) {
        cb!(self, shutdown, (), occ)
    }
    fn has_property_holder(&self, occ: u64) -> bool {
        cb!(self, has_property_holder, false, occ)
    }
    fn occupant_type(&self, occ: u64) -> u32 {
        cb!(self, occupant_type, 0, occ)
    }
    fn bounding_box(&self, occ: u64) -> BBox {
        let mut b = fc4_occ_bbox::default();
        cb!(self, bounding_box, (), occ, &mut b);
        BBox { min: b.min, max: b.max }
    }
    fn post_message(&self, occ: u64, msg_type: u32, data4: u32) {
        cb!(self, post_message, (), occ, msg_type, data4)
    }
    fn manager_bbox(&self, occ: u64) -> Option<CellBox> {
        let mut b = fc4_occ_cell_box::default();
        cb!(self, manager_bbox, false, occ, &mut b).then(|| box_from_c(&b))
    }
    fn set_manager_bbox(&self, occ: u64, b: CellBox) {
        let c = fc4_occ_cell_box {
            x0: b.x0,
            z0: b.z0,
            x1: b.x1,
            z1: b.z1,
        };
        cb!(self, set_manager_bbox, (), occ, &c)
    }
    fn set_flag(&self, occ: u64, flag: u32) {
        cb!(self, set_flag, (), occ, flag)
    }
    fn clear_flag(&self, occ: u64, flag: u32) {
        cb!(self, clear_flag, (), occ, flag)
    }
    fn is_flag_set(&self, occ: u64, flag: u32) -> bool {
        cb!(self, is_flag_set, false, occ, flag)
    }
    fn filter_includes(&self, filter: u64, occ: u64) -> bool {
        cb!(self, filter_includes, false, filter, occ)
    }
    fn object_ref(&self, occ: u64) -> RefValue {
        if occ == 0 {
            RefValue::Null
        } else {
            RefValue::Host {
                token: occ,
                class_id: 0,
            }
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

/// SAFETY of the cross-thread use: the manager's `RefCell` is touched only
/// between the core's `lock` and `unlock` calls (the recursive lock above);
/// everything it reads outside them is atomic. The two `Rc`s are cloned only
/// at creation and dropped only by `fc4_occmgr_destroy`, which the caller
/// must not race with other calls.
struct Handle {
    m: OccupantManager,
    host: Rc<CHost>,
}

fn with_m<R>(p: *const fc4_occmgr, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_occmgr*.
    match unsafe { (p as *const Handle).as_ref() } {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

/// SAFETY: `f` NULL or valid.
unsafe fn filter_of(f: *const fc4_occ_filter) -> Filter {
    match f.as_ref() {
        Some(f) if f.kind == FC4_OCC_FILTER_HOST && f.token != 0 => Filter::Host(f.token),
        Some(f) if f.kind == FC4_OCC_FILTER_TYPE => Filter::Type(f.occ_type),
        _ => Filter::All,
    }
}

/// SAFETY: `p` NULL or valid for 2 values.
unsafe fn pair(p: *const i32) -> Option<(i32, i32)> {
    (!p.is_null()).then(|| (*p, *p.add(1)))
}

/// SAFETY: `p` NULL or valid for 3 values.
unsafe fn vec3(p: *const f32) -> Option<Vec3> {
    (!p.is_null()).then(|| [*p, *p.add(1), *p.add(2)])
}

/// SAFETY: `p` NULL or valid for 3 values.
unsafe fn ivec3(p: *const i32) -> Option<[i32; 3]> {
    (!p.is_null()).then(|| [*p, *p.add(1), *p.add(2)])
}

/// SAFETY: each pointer NULL or valid.
unsafe fn put2<T: Copy>(a: *mut T, b: *mut T, v: (T, T)) {
    if let Some(a) = a.as_mut() {
        *a = v.0;
    }
    if let Some(b) = b.as_mut() {
        *b = v.1;
    }
}

/// SAFETY: `out` NULL or valid.
unsafe fn put_first(r: Option<u64>, out: *mut u64) -> bool {
    if let Some(o) = out.as_mut() {
        *o = r.unwrap_or(0);
    }
    r.is_some()
}

/// SAFETY: `sink` a valid callback or None.
unsafe fn emit(list: &[u64], sink: fc4_occ_list_sink, ctx: U) {
    if let Some(f) = sink {
        for &o in list {
            f(ctx, o);
        }
    }
}

/// An iteration callback as the core's closure.
fn walker(f: unsafe extern "C" fn(U, u64) -> bool, ctx: U) -> impl FnMut(u64) -> bool {
    // SAFETY: the caller promised a valid callback for the call.
    move |o| unsafe { f(ctx, o) }
}

/// Creates the core object (constructed: reference count 0, not
/// initialised, no grid). The host table is copied. NULL when `host` is
/// NULL or too small.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_create(host: *const fc4_occmgr_host) -> *mut fc4_occmgr {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_occmgr_host>() => {
            let c = Rc::new(CHost {
                h: *h,
                lock: RecursiveLock::default(),
            });
            let m = OccupantManager::new(c.clone());
            Box::into_raw(Box::new(Handle { m, host: c })) as *mut fc4_occmgr
        }
        _ => std::ptr::null_mut(),
    })
}

/// Frees the object. No other call may be running on it.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_destroy(m: *mut fc4_occmgr) {
    if !m.is_null() {
        guard((), || drop(Box::from_raw(m as *mut Handle)));
    }
}

/// `QueryInterface` (§1): the view's offset (0 for IIDs 1 and 0xC6BB8334,
/// 4 for 0xE4FDA3D4, 8 for 0x452294AA), −1 for any other IID.
#[no_mangle]
pub extern "C" fn fc4_occmgr_query_interface(iid: u32) -> i32 {
    match query_interface(iid) {
        Some(Interface::OccupantManager) => 0,
        Some(Interface::Serializable) => 4,
        Some(Interface::MessageTarget2) => 8,
        None => -1,
    }
}

#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_add_ref(m: *mut fc4_occmgr) -> u32 {
    with_m(m, 0, |h| h.m.add_ref())
}

/// `Release` (§2): at the last reference the object shuts down and is
/// marked destroyed; the handle stays valid until [`fc4_occmgr_destroy`].
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_release(m: *mut fc4_occmgr) -> u32 {
    with_m(m, 0, |h| h.m.release())
}

/// `Init` (slot 3, §2).
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_init(m: *mut fc4_occmgr) -> bool {
    with_m(m, false, |h| h.m.init())
}

/// `Shutdown` (slot 4, §2).
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_shutdown(m: *mut fc4_occmgr) -> bool {
    with_m(m, false, |h| h.m.shutdown())
}

/// `SetCitySize(fx, fz)` (slot 5, §2).
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_set_city_size(m: *mut fc4_occmgr, fx: f32, fz: f32) -> bool {
    with_m(m, false, |h| h.m.set_city_size(fx, fz))
}

/// Slot 6 (64.0, 64.0). Outputs may be NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_get_cell_sizes(m: *const fc4_occmgr, x: *mut f32, z: *mut f32) -> bool {
    with_m(m, false, |h| {
        put2(x, z, h.m.cell_sizes());
        true
    })
}

/// Slot 7 (192, 192).
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_get_world_cell_count(m: *const fc4_occmgr, x: *mut i32, z: *mut i32) -> bool {
    with_m(m, false, |h| {
        put2(x, z, h.m.world_cell_count());
        true
    })
}

/// Slot 8: the manager cell counts.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_get_cell_count(m: *const fc4_occmgr, x: *mut i32, z: *mut i32) -> bool {
    with_m(m, false, |h| {
        put2(x, z, h.m.cell_count());
        true
    })
}

/// The standard cell counts.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_get_standard_cell_count(m: *const fc4_occmgr, x: *mut i32, z: *mut i32) -> bool {
    with_m(m, false, |h| {
        put2(x, z, h.m.standard_cell_count());
        true
    })
}

/// The city size in metres.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_get_city_size(m: *const fc4_occmgr, x: *mut f32, z: *mut f32) -> bool {
    with_m(m, false, |h| {
        put2(x, z, h.m.city_size());
        true
    })
}

/// Slot 9: `out` = (64, 64, count X + 63, count Z + 63).
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_get_cell_bounds(m: *const fc4_occmgr, out: *mut i32) -> bool {
    if out.is_null() {
        return false;
    }
    with_m(m, false, |h| {
        let (a, b, c, d) = h.m.cell_bounds();
        for (i, v) in [a, b, c, d].into_iter().enumerate() {
            *out.add(i) = v;
        }
        true
    })
}

// ---- conversions (slots 10–19, §3.2): no object --------------------------------------------------

/// Slot 10.
#[no_mangle]
pub unsafe extern "C" fn fc4_occ_world_cell_to_manager_cell(x: i32, z: i32, ox: *mut i32, oz: *mut i32) {
    put2(ox, oz, om::world_cell_to_occupant_manager_cell(x, z))
}

/// Slot 11.
#[no_mangle]
pub unsafe extern "C" fn fc4_occ_manager_cell_to_world_cell(x: i32, z: i32, ox: *mut i32, oz: *mut i32) {
    put2(ox, oz, om::occupant_manager_cell_to_world_cell(x, z))
}

/// Slot 12 *[quirk]* (§15 quirk 1): `*ox = 4·z − 256`, `*oz = 4·(*oz)` (the
/// value already in `*oz`; NULL reads 0). `x` is ignored.
#[no_mangle]
pub unsafe extern "C" fn fc4_occ_world_cell_to_standard_city_cell(x: i32, z: i32, ox: *mut i32, oz: *mut i32) {
    let prior = oz.as_ref().copied().unwrap_or(0);
    put2(ox, oz, om::world_cell_to_standard_city_cell(x, z, prior))
}

/// Slot 13 (C division: −1 → 64).
#[no_mangle]
pub unsafe extern "C" fn fc4_occ_standard_city_cell_to_world_cell(x: i32, z: i32, ox: *mut i32, oz: *mut i32) {
    put2(ox, oz, om::standard_city_cell_to_world_cell(x, z))
}

/// Slot 14.
#[no_mangle]
pub unsafe extern "C" fn fc4_occ_world_cell_to_position(x: i32, z: i32, fx: *mut f32, fz: *mut f32) {
    put2(fx, fz, om::world_cell_to_position(x, z))
}

/// Slot 15 (§3.3).
#[no_mangle]
pub unsafe extern "C" fn fc4_occ_position_to_world_cell(fx: f32, fz: f32, x: *mut i32, z: *mut i32) {
    put2(x, z, om::position_to_world_cell(fx, fz))
}

/// Slot 16.
#[no_mangle]
pub unsafe extern "C" fn fc4_occ_manager_cell_to_position(x: i32, z: i32, fx: *mut f32, fz: *mut f32) {
    put2(fx, fz, om::occupant_manager_cell_to_position(x, z))
}

/// Slot 17.
#[no_mangle]
pub unsafe extern "C" fn fc4_occ_position_to_manager_cell(fx: f32, fz: f32, x: *mut i32, z: *mut i32) {
    put2(x, z, om::position_to_occupant_manager_cell(fx, fz))
}

/// Slot 18.
#[no_mangle]
pub unsafe extern "C" fn fc4_occ_standard_city_cell_to_position(x: i32, z: i32, fx: *mut f32, fz: *mut f32) {
    put2(fx, fz, om::standard_city_cell_to_position(x, z))
}

/// Slot 19 (§3.4).
#[no_mangle]
pub unsafe extern "C" fn fc4_occ_position_to_standard_city_cell(fx: f32, fz: f32, x: *mut i32, z: *mut i32) {
    put2(x, z, om::position_to_standard_city_cell(fx, fz))
}

/// The manager box of a bounding box (§5.1 steps 5–6; bytes, no clamp).
#[no_mangle]
pub unsafe extern "C" fn fc4_occ_cell_box_of(b: *const fc4_occ_bbox, out: *mut fc4_occ_cell_box) -> bool {
    let (Some(b), Some(o)) = (b.as_ref(), out.as_mut()) else {
        return false;
    };
    let c = om::cell_box_of(&BBox { min: b.min, max: b.max });
    *o = fc4_occ_cell_box {
        x0: c.x0,
        z0: c.z0,
        x1: c.x1,
        z1: c.z1,
    };
    true
}

/// Slot 26 `GetBoundingCells(x, z)` (§6.2): writes up to 8 (x, z) pairs to
/// `out` (16 values) and returns their number.
#[no_mangle]
pub unsafe extern "C" fn fc4_occ_bounding_cells(x: i32, z: i32, out: *mut i32) -> u32 {
    let cells = om::bounding_cells(x, z);
    if !out.is_null() {
        for (i, (a, b)) in cells.iter().enumerate() {
            *out.add(2 * i) = *a;
            *out.add(2 * i + 1) = *b;
        }
    }
    cells.len() as u32
}

// ---- insert / remove / move (§5) --------------------------------------------------------------------

/// Slot 20 `InsertOccupant(occ, data4)` (§5.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_insert(m: *mut fc4_occmgr, occ: u64, data4: u32) -> bool {
    with_m(m, false, |h| h.m.insert(occ, data4))
}

/// Slot 21 `RemoveOccupant(occ, shutdown, data4)` (§5.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_remove(m: *mut fc4_occmgr, occ: u64, shutdown: bool, data4: u32) -> bool {
    with_m(m, false, |h| h.m.remove(occ, shutdown, data4))
}

/// Slot 22 `RemoveOccupants(filter, shutdown, data4)` (§5.3).
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_remove_occupants(
    m: *mut fc4_occmgr,
    filter: *const fc4_occ_filter,
    shutdown: bool,
    data4: u32,
) -> bool {
    let f = filter_of(filter);
    with_m(m, false, |h| h.m.remove_occupants(f, shutdown, data4))
}

/// Slot 23: the typed thunk of slot 22.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_remove_occupants_of_type(
    m: *mut fc4_occmgr,
    occ_type: u32,
    shutdown: bool,
    data4: u32,
) -> bool {
    with_m(m, false, |h| h.m.remove_occupants_of_type(occ_type, shutdown, data4))
}

/// Slot 24 `MoveOccupant(occ, post)` (§5.4).
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_move(m: *mut fc4_occmgr, occ: u64, post: bool) -> bool {
    with_m(m, false, |h| h.m.move_occupant(occ, post))
}

/// Slot 25 `IsCellEmpty(x, z)` (§6.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_is_cell_empty(m: *const fc4_occmgr, x: i32, z: i32) -> bool {
    with_m(m, true, |h| h.m.is_cell_empty(x, z))
}

/// Slot 27 `FindOccupant(occ, searchAll, &x, &z)` (§6.3). `x` and `z` are
/// written only by the full scan (§15 quirk 7); NULL ones are skipped.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_find_occupant(
    m: *const fc4_occmgr,
    occ: u64,
    search_all: bool,
    x: *mut i32,
    z: *mut i32,
) -> bool {
    with_m(m, false, |h| {
        let mut a = x.as_ref().copied().unwrap_or(0);
        let mut b = z.as_ref().copied().unwrap_or(0);
        let r = h.m.find_occupant(occ, search_all, &mut a, &mut b);
        put2(x, z, (a, b));
        r
    })
}

// ---- first-occupant queries (§7) ---------------------------------------------------------------------
// `out` receives the occupant (holding one reference for the caller) or 0.

/// Slot 28.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_first_by_position(
    m: *const fc4_occmgr,
    fx: f32,
    fz: f32,
    filter: *const fc4_occ_filter,
    out: *mut u64,
) -> bool {
    let f = filter_of(filter);
    put_first(with_m(m, None, |h| h.m.first_by_position(fx, fz, f)), out)
}

/// Slot 29.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_first_by_position_typed(
    m: *const fc4_occmgr,
    fx: f32,
    fz: f32,
    occ_type: u32,
    out: *mut u64,
) -> bool {
    put_first(with_m(m, None, |h| h.m.first_by_position_typed(fx, fz, occ_type)), out)
}

/// Slot 30.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_first_in_cell(
    m: *const fc4_occmgr,
    x: i32,
    z: i32,
    filter: *const fc4_occ_filter,
    out: *mut u64,
) -> bool {
    let f = filter_of(filter);
    put_first(with_m(m, None, |h| h.m.first_in_cell(x, z, f)), out)
}

/// Slot 31.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_first_in_cell_typed(
    m: *const fc4_occmgr,
    x: i32,
    z: i32,
    occ_type: u32,
    out: *mut u64,
) -> bool {
    put_first(with_m(m, None, |h| h.m.first_in_cell_typed(x, z, occ_type)), out)
}

/// Slot 32.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_first_by_standard_cell(
    m: *const fc4_occmgr,
    x: i32,
    z: i32,
    filter: *const fc4_occ_filter,
    out: *mut u64,
) -> bool {
    let f = filter_of(filter);
    put_first(with_m(m, None, |h| h.m.first_by_standard_cell(x, z, f)), out)
}

/// Slot 33.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_first_by_standard_cell_typed(
    m: *const fc4_occmgr,
    x: i32,
    z: i32,
    occ_type: u32,
    out: *mut u64,
) -> bool {
    put_first(
        with_m(m, None, |h| h.m.first_by_standard_cell_typed(x, z, occ_type)),
        out,
    )
}

/// Slot 34. `min` / `max`: (x, z) or NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_first_by_standard_cells(
    m: *const fc4_occmgr,
    min: *const i32,
    max: *const i32,
    filter: *const fc4_occ_filter,
    out: *mut u64,
) -> bool {
    let (a, b, f) = (pair(min), pair(max), filter_of(filter));
    put_first(with_m(m, None, |h| h.m.first_by_standard_cells(a, b, f)), out)
}

/// Slot 35.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_first_by_standard_cells_typed(
    m: *const fc4_occmgr,
    min: *const i32,
    max: *const i32,
    occ_type: u32,
    out: *mut u64,
) -> bool {
    let (a, b) = (pair(min), pair(max));
    put_first(
        with_m(m, None, |h| h.m.first_by_standard_cells_typed(a, b, occ_type)),
        out,
    )
}

// ---- list queries (§8) -----------------------------------------------------------------------------
// The appended occupants go to `sink` after the query returns. `max_count`
// is compared unsigned (−1 = all; 0 = false).

/// Slot 36. `min` / `max`: (x, y, z) or NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_occupants_by_bbox(
    m: *const fc4_occmgr,
    min: *const f32,
    max: *const f32,
    filter: *const fc4_occ_filter,
    max_count: i32,
    sink: fc4_occ_list_sink,
    ctx: *mut c_void,
) -> bool {
    let (a, b, f) = (vec3(min), vec3(max), filter_of(filter));
    let mut l = Vec::new();
    let r = with_m(m, false, |h| h.m.occupants_by_bbox(&mut l, a, b, f, max_count));
    emit(&l, sink, ctx);
    r
}

/// Slot 37.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_occupants_by_bbox_typed(
    m: *const fc4_occmgr,
    min: *const f32,
    max: *const f32,
    occ_type: u32,
    max_count: i32,
    sink: fc4_occ_list_sink,
    ctx: *mut c_void,
) -> bool {
    let (a, b) = (vec3(min), vec3(max));
    let mut l = Vec::new();
    let r = with_m(m, false, |h| {
        h.m.occupants_by_bbox_typed(&mut l, a, b, occ_type, max_count)
    });
    emit(&l, sink, ctx);
    r
}

/// Slot 38. `min` / `max`: world cells (x, z) or NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_occupants_by_cells(
    m: *const fc4_occmgr,
    min: *const i32,
    max: *const i32,
    filter: *const fc4_occ_filter,
    max_count: i32,
    sink: fc4_occ_list_sink,
    ctx: *mut c_void,
) -> bool {
    let (a, b, f) = (pair(min), pair(max), filter_of(filter));
    let mut l = Vec::new();
    let r = with_m(m, false, |h| h.m.occupants_by_cells(&mut l, a, b, f, max_count));
    emit(&l, sink, ctx);
    r
}

/// Slot 39.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_occupants_by_cells_typed(
    m: *const fc4_occmgr,
    min: *const i32,
    max: *const i32,
    occ_type: u32,
    max_count: i32,
    sink: fc4_occ_list_sink,
    ctx: *mut c_void,
) -> bool {
    let (a, b) = (pair(min), pair(max));
    let mut l = Vec::new();
    let r = with_m(m, false, |h| {
        h.m.occupants_by_cells_typed(&mut l, a, b, occ_type, max_count)
    });
    emit(&l, sink, ctx);
    r
}

/// Slot 40. `min` / `max`: standard cells (x, z) or NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_occupants_by_standard_cells(
    m: *const fc4_occmgr,
    min: *const i32,
    max: *const i32,
    filter: *const fc4_occ_filter,
    max_count: i32,
    sink: fc4_occ_list_sink,
    ctx: *mut c_void,
) -> bool {
    let (a, b, f) = (pair(min), pair(max), filter_of(filter));
    let mut l = Vec::new();
    let r = with_m(m, false, |h| {
        h.m.occupants_by_standard_cells(&mut l, a, b, f, max_count)
    });
    emit(&l, sink, ctx);
    r
}

/// Slot 41.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_occupants_by_standard_cells_typed(
    m: *const fc4_occmgr,
    min: *const i32,
    max: *const i32,
    occ_type: u32,
    max_count: i32,
    sink: fc4_occ_list_sink,
    ctx: *mut c_void,
) -> bool {
    let (a, b) = (pair(min), pair(max));
    let mut l = Vec::new();
    let r = with_m(m, false, |h| {
        h.m.occupants_by_standard_cells_typed(&mut l, a, b, occ_type, max_count)
    });
    emit(&l, sink, ctx);
    r
}

// ---- iteration (§9) --------------------------------------------------------------------------------
// A NULL callback is false (the original calls through it).

/// Slot 42 `IterateOccupantsByBBox`: callbacks run without the lock.
/// `min` and `max` (x, y, z) are required.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_iterate_by_bbox(
    m: *const fc4_occmgr,
    f: fc4_occ_iterate_fn,
    ctx: *mut c_void,
    min: *const f32,
    max: *const f32,
    filter: *const fc4_occ_filter,
) -> bool {
    let (Some(f), Some(a), Some(b)) = (f, vec3(min), vec3(max)) else {
        return false;
    };
    let flt = filter_of(filter);
    with_m(m, false, |h| h.m.iterate_by_bbox(&mut walker(f, ctx), a, b, flt))
}

/// Slot 43.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_iterate_by_bbox_typed(
    m: *const fc4_occmgr,
    f: fc4_occ_iterate_fn,
    ctx: *mut c_void,
    min: *const f32,
    max: *const f32,
    occ_type: u32,
) -> bool {
    let (Some(f), Some(a), Some(b)) = (f, vec3(min), vec3(max)) else {
        return false;
    };
    with_m(m, false, |h| {
        h.m.iterate_by_bbox_typed(&mut walker(f, ctx), a, b, occ_type)
    })
}

/// Slot 44 `IterateOccupants`: the lock is held for the whole walk.
/// `min` / `max`: world cells (x, y, z; y ignored) or NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_iterate(
    m: *const fc4_occmgr,
    f: fc4_occ_iterate_fn,
    ctx: *mut c_void,
    min: *const i32,
    max: *const i32,
    filter: *const fc4_occ_filter,
) -> bool {
    let Some(f) = f else { return false };
    let (a, b, flt) = (ivec3(min), ivec3(max), filter_of(filter));
    with_m(m, false, |h| h.m.iterate(&mut walker(f, ctx), a, b, flt))
}

/// Slot 45.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_iterate_typed(
    m: *const fc4_occmgr,
    f: fc4_occ_iterate_fn,
    ctx: *mut c_void,
    min: *const i32,
    max: *const i32,
    occ_type: u32,
) -> bool {
    let Some(f) = f else { return false };
    let (a, b) = (ivec3(min), ivec3(max));
    with_m(m, false, |h| h.m.iterate_typed(&mut walker(f, ctx), a, b, occ_type))
}

/// Slot 47 (the filter overload). `min` / `max`: standard cells (x, z) or
/// NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_iterate_by_standard_cells(
    m: *const fc4_occmgr,
    f: fc4_occ_iterate_fn,
    ctx: *mut c_void,
    min: *const i32,
    max: *const i32,
    filter: *const fc4_occ_filter,
) -> bool {
    let Some(f) = f else { return false };
    let (a, b, flt) = (pair(min), pair(max), filter_of(filter));
    with_m(m, false, |h| {
        h.m.iterate_by_standard_cells(&mut walker(f, ctx), a, b, flt)
    })
}

/// Slot 46 (the typed overload).
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_iterate_by_standard_cells_typed(
    m: *const fc4_occmgr,
    f: fc4_occ_iterate_fn,
    ctx: *mut c_void,
    min: *const i32,
    max: *const i32,
    occ_type: u32,
) -> bool {
    let Some(f) = f else { return false };
    let (a, b) = (pair(min), pair(max));
    with_m(m, false, |h| {
        h.m.iterate_by_standard_cells_typed(&mut walker(f, ctx), a, b, occ_type)
    })
}

/// Slot 48 `ReleaseOccupantList`: releases each of the `n` occupants once.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_release_list(m: *const fc4_occmgr, list: *const u64, n: usize) {
    if list.is_null() && n != 0 {
        return;
    }
    let mut v: Vec<u64> = if n == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(list, n).to_vec()
    };
    with_m(m, (), |h| h.m.release_occupant_list(&mut v))
}

// ---- inspection --------------------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_get_state(m: *const fc4_occmgr, out: *mut fc4_occmgr_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_m(m, false, |h| {
        let s = h.m.snapshot();
        *o = fc4_occmgr_state {
            struct_size: std::mem::size_of::<fc4_occmgr_state>() as u32,
            initialized: s.initialized.into(),
            was_read: s.was_read.into(),
            destroyed: s.destroyed.into(),
            filter_present: s.filter_type.is_some().into(),
            ref_count: s.ref_count,
            filter_type: s.filter_type.unwrap_or(0),
            size_x: s.size.0,
            size_z: s.size.1,
            counts: s.counts,
            rows: s.rows as u32,
            holders: s.holders as u32,
            nodes: s.nodes as u32,
            lock_waits: h.host.lock.waits.load(Ordering::Relaxed),
        };
        true
    })
}

/// A cell's list, front first: copies at most `capacity` occupants to
/// `out` and their single-cell bits to `single` (either may be NULL).
/// Returns the list's length, −1 for a null cell or one outside the grid.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_cell(
    m: *const fc4_occmgr,
    x: i32,
    z: i32,
    out: *mut u64,
    single: *mut u8,
    capacity: usize,
) -> i32 {
    with_m(m, -1, |h| match h.m.cell(x, z) {
        None => -1,
        Some(nodes) => {
            for (i, n) in nodes.iter().take(capacity).enumerate() {
                if !out.is_null() {
                    *out.add(i) = n.occupant;
                }
                if !single.is_null() {
                    *single.add(i) = n.single.into();
                }
            }
            nodes.len() as i32
        }
    })
}

/// Diagnostics: the calling thread holds the manager's lock this deep (0 =
/// not held). For hosts checking where callbacks run.
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_lock_depth(m: *const fc4_occmgr) -> u32 {
    with_m(m, 0, |h| h.host.lock.depth_here())
}

// ---- save record (§13) ------------------------------------------------------------------------------

/// `Write` (§13): version 1; occupants as host references (token, class 0).
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_write(m: *const fc4_occmgr, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_m(m, false, |h| h.m.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

/// `Read` (§13): false unless the version is 1. Occupants come back through
/// the host's `resolve_ref` (IID `0x26BB767E`), appended to their cells in
/// the saved order. [choice] Atomic: the sizes, counts and grid change only
/// when the whole record parses (the was-read flag is set either way).
#[no_mangle]
pub unsafe extern "C" fn fc4_occmgr_read(m: *mut fc4_occmgr, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_m(m, false, |h| {
        h.m.read(&mut CIn {
            stream: i,
            decode: no_core_objects,
        })
    })
}
