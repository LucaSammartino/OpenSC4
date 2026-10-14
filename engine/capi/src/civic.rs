// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The civic building simulator over the C ABI (fc4.h "civic building
//! simulator", docs/specs/civic-buildings.md).
//!
//! The host implements [`fc4_civic_host`]; the plugin's
//! `cISC4CivicBuildingSimulator` (with its message-target and serializable
//! sub-objects) forwards its slots here.
//!
//! Tokens are opaque host identities: occupants (a building and its
//! occupant are one token), departments, line items and lots. The core holds
//! one reference on the Landmarks department and on every list entry.
//!
//! **Statuses** (§4.7): the original's slot 6 returns a pointer callers write
//! through. The core keeps the statuses; [`fc4_civic_get_status`] copies one
//! out and [`fc4_civic_update_status`] writes one back, so a plugin that must
//! hand out a stable `ConditionalBuildingStatus*` keeps its own mirror and
//! writes it back after the caller changed it.
//!
//! Threads: the core is single-threaded (`RefCell`). The plugin serialises
//! every call on one object (fc4.h).

use std::ffi::{c_char, c_void, CString};
use std::rc::Rc;

use fc4_core::budget::BudgetItem;
use fc4_core::civic::{query_interface, CivicHost, CivicMessage, CivicSim, ConditionalBuildingStatus};
use fc4_core::stream::{RefValue, SaveObject};

use crate::budget::fc4_budget_item;
use crate::guard;
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

#[repr(C)]
pub struct fc4_civic {
    _private: [u8; 0],
}

/// What the civic building simulator needs from the game (fc4.h
/// `fc4_civic_host`). A missing callback reads as 0 / false / none.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_civic_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    // agents, messages, Lua
    pub add_agent: Option<unsafe extern "C" fn(U, u32, *const c_char, u32)>,
    pub remove_agent: Option<unsafe extern "C" fn(U)>,
    pub subscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub register_lua: Option<unsafe extern "C" fn(U)>,
    pub unregister_lua: Option<unsafe extern "C" fn(U)>,
    // references
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    pub resolve_ref: Option<unsafe extern "C" fn(U, u64, u32) -> u64>,
    // departments
    pub department_budget: Option<unsafe extern "C" fn(U, u32) -> u64>,
    pub create_department_budget: Option<unsafe extern "C" fn(U, u32, u32) -> u64>,
    pub set_department_name: Option<unsafe extern "C" fn(U, u64, u32, u32)>,
    pub set_max_funding: Option<unsafe extern "C" fn(U, u64, f32)>,
    pub department_funding: Option<unsafe extern "C" fn(U, u64, *mut f32) -> bool>,
    // budget items and line items
    pub budget_item: Option<unsafe extern "C" fn(U, u64, u32, *mut fc4_budget_item) -> bool>,
    pub change_budget_item_line: Option<unsafe extern "C" fn(U, u64, i32, u32)>,
    pub remove_budget_item: Option<unsafe extern "C" fn(U, u64, i32)>,
    pub line_item: Option<unsafe extern "C" fn(U, u64, u32) -> u64>,
    pub create_line_item: Option<unsafe extern "C" fn(U, u64, u32, bool) -> u64>,
    pub remove_line_item: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub add_to_full_expenses: Option<unsafe extern "C" fn(U, u64, i64)>,
    pub add_to_income: Option<unsafe extern "C" fn(U, u64, i64)>,
    pub line_income: Option<unsafe extern "C" fn(U, u64) -> i64>,
    pub set_line_income: Option<unsafe extern "C" fn(U, u64, i64)>,
    pub set_line_type: Option<unsafe extern "C" fn(U, u64, u32)>,
    // occupants and buildings
    pub occupant_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub occupant_bool_property: Option<unsafe extern "C" fn(U, u64, u32, *mut bool) -> bool>,
    pub occupant_u32_property: Option<unsafe extern "C" fn(U, u64, u32, *mut u32) -> bool>,
    pub is_building_occupant: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub building_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub building_key_instance: Option<unsafe extern "C" fn(U, u64, *mut u32) -> bool>,
    pub is_occupant_group: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    // lots
    pub message_lot: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub lot_building: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub occupant_lot: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub lot_condition: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub set_lot_condition: Option<unsafe extern "C" fn(U, u64, u32)>,
    // automata
    pub add_controllers: Option<unsafe extern "C" fn(U, *const c_char, u64)>,
}

/// Diagnostics (`fc4_civic_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_civic_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub destroyed: u8,
    pub reserved0: u16,
    pub ref_count: u32,
    pub exclusions: u32,
    pub statuses: u32,
    pub disease_centers: u32,
    /// `GetLandmarkCount()`.
    pub maintained: u32,
    /// Temporary generators made so far (§5).
    pub temporaries: u32,
    /// The Landmarks department (0 = none).
    pub landmarks: u64,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_civic_state>() == 40);
    assert!(std::mem::size_of::<fc4_civic_host>() == if cfg!(target_pointer_width = "32") { 160 } else { 320 });
};

// ---- host adapter -----------------------------------------------------------------------------------

struct CHost {
    h: fc4_civic_host,
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

pub(crate) fn item_from_c(i: &fc4_budget_item) -> BudgetItem {
    BudgetItem {
        department_id: i.department_id,
        line_item: i.line_item,
        purpose: i.purpose,
        cost: i.cost,
        index: i.index,
        local_funding: i.local_funding,
    }
}

pub(crate) fn item_to_c(i: &BudgetItem) -> fc4_budget_item {
    fc4_budget_item {
        department_id: i.department_id,
        line_item: i.line_item,
        purpose: i.purpose,
        index: i.index,
        cost: i.cost,
        local_funding: i.local_funding,
        reserved: 0,
    }
}

impl CivicHost for CHost {
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        let n = c_name(name);
        cb!(self, add_agent, (), level, n.as_ptr(), flags)
    }
    fn remove_agent(&self) {
        cb!(self, remove_agent, ())
    }
    fn subscribe(&self, msg_type: u32) {
        cb!(self, subscribe, (), msg_type)
    }
    fn unsubscribe(&self, msg_type: u32) {
        cb!(self, unsubscribe, (), msg_type)
    }
    fn register_lua(&self) {
        cb!(self, register_lua, ())
    }
    fn unregister_lua(&self) {
        cb!(self, unregister_lua, ())
    }
    fn add_ref(&self, token: u64) {
        cb!(self, add_ref, (), token)
    }
    fn release(&self, token: u64) {
        cb!(self, release, (), token)
    }
    fn department_budget(&self, id: u32) -> Option<u64> {
        opt(cb!(self, department_budget, 0, id))
    }
    fn create_department_budget(&self, id: u32, group: u32) -> Option<u64> {
        opt(cb!(self, create_department_budget, 0, id, group))
    }
    fn set_department_name(&self, dept: u64, group: u32, instance: u32) {
        cb!(self, set_department_name, (), dept, group, instance)
    }
    fn set_max_funding(&self, dept: u64, max: f32) {
        cb!(self, set_max_funding, (), dept, max)
    }
    fn department_funding(&self, dept: u64) -> Option<f32> {
        let mut f = 0.0f32;
        cb!(self, department_funding, false, dept, &mut f).then_some(f)
    }
    fn budget_item(&self, occupant: u64, purpose: u32) -> Option<BudgetItem> {
        let mut i = fc4_budget_item {
            purpose,
            ..Default::default()
        };
        cb!(self, budget_item, false, occupant, purpose, &mut i).then(|| item_from_c(&i))
    }
    fn change_budget_item_line(&self, occupant: u64, index: i32, line: u32) {
        cb!(self, change_budget_item_line, (), occupant, index, line)
    }
    fn remove_budget_item(&self, occupant: u64, index: i32) {
        cb!(self, remove_budget_item, (), occupant, index)
    }
    fn line_item(&self, dept: u64, line: u32) -> Option<u64> {
        opt(cb!(self, line_item, 0, dept, line))
    }
    fn create_line_item(&self, dept: u64, line: u32, local: bool) -> Option<u64> {
        opt(cb!(self, create_line_item, 0, dept, line, local))
    }
    fn remove_line_item(&self, dept: u64, line: u32) {
        cb!(self, remove_line_item, (), dept, line)
    }
    fn add_to_full_expenses(&self, line: u64, amount: i64) {
        cb!(self, add_to_full_expenses, (), line, amount)
    }
    fn add_to_income(&self, line: u64, amount: i64) {
        cb!(self, add_to_income, (), line, amount)
    }
    fn line_income(&self, line: u64) -> i64 {
        cb!(self, line_income, 0, line)
    }
    fn set_line_income(&self, line: u64, income: i64) {
        cb!(self, set_line_income, (), line, income)
    }
    fn set_line_type(&self, line: u64, line_type: u32) {
        cb!(self, set_line_type, (), line, line_type)
    }
    fn occupant_type(&self, occupant: u64) -> u32 {
        cb!(self, occupant_type, 0, occupant)
    }
    fn occupant_bool_property(&self, occupant: u64, id: u32) -> Option<bool> {
        let mut v = false;
        cb!(self, occupant_bool_property, false, occupant, id, &mut v).then_some(v)
    }
    fn occupant_u32_property(&self, occupant: u64, id: u32) -> Option<u32> {
        let mut v = 0u32;
        cb!(self, occupant_u32_property, false, occupant, id, &mut v).then_some(v)
    }
    fn is_building_occupant(&self, occupant: u64) -> bool {
        cb!(self, is_building_occupant, false, occupant)
    }
    fn building_type(&self, building: u64) -> u32 {
        cb!(self, building_type, 0, building)
    }
    fn building_key_instance(&self, building: u64) -> Option<u32> {
        let mut v = 0u32;
        cb!(self, building_key_instance, false, building, &mut v).then_some(v)
    }
    fn is_occupant_group(&self, occupant: u64, group: u32) -> bool {
        cb!(self, is_occupant_group, false, occupant, group)
    }
    fn message_lot(&self, unknown: u64) -> Option<u64> {
        opt(cb!(self, message_lot, 0, unknown))
    }
    fn lot_building(&self, lot: u64) -> Option<u64> {
        opt(cb!(self, lot_building, 0, lot))
    }
    fn occupant_lot(&self, occupant: u64) -> Option<u64> {
        opt(cb!(self, occupant_lot, 0, occupant))
    }
    fn lot_condition(&self, lot: u64) -> u32 {
        cb!(self, lot_condition, 0, lot)
    }
    fn set_lot_condition(&self, lot: u64, condition: u32) {
        cb!(self, set_lot_condition, (), lot, condition)
    }
    fn add_controllers(&self, name: &str, occupant: u64) {
        let n = c_name(name);
        cb!(self, add_controllers, (), n.as_ptr(), occupant)
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
    sim: Rc<CivicSim>,
}

fn with_c<R>(c: *const fc4_civic, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_civic*.
    match unsafe { (c as *const Handle).as_ref() } {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

unsafe fn put<T>(p: *mut T, v: T) {
    if let Some(r) = p.as_mut() {
        *r = v;
    }
}

/// Creates the core object (constructed, reference count 0). The host table
/// is copied. `session_seed` is foundations §6.3's `S` (the ape escape's
/// temporaries, §5).
#[no_mangle]
pub unsafe extern "C" fn fc4_civic_create(host: *const fc4_civic_host, session_seed: u32) -> *mut fc4_civic {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_civic_host>() => {
            let sim = CivicSim::new(Rc::new(CHost { h: *h }), 0, session_seed);
            Box::into_raw(Box::new(Handle { sim })) as *mut fc4_civic
        }
        _ => std::ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_civic_destroy(c: *mut fc4_civic) {
    if !c.is_null() {
        guard((), || drop(Box::from_raw(c as *mut Handle)));
    }
}

#[no_mangle]
pub extern "C" fn fc4_civic_query_interface(iid: u32) -> bool {
    query_interface(iid)
}

#[no_mangle]
pub unsafe extern "C" fn fc4_civic_add_ref(c: *mut fc4_civic) -> u32 {
    with_c(c, 0, |h| h.sim.add_ref())
}

/// `Release`: 0 when the object was destroyed (`Shutdown` ran); the handle
/// stays valid until [`fc4_civic_destroy`].
#[no_mangle]
pub unsafe extern "C" fn fc4_civic_release(c: *mut fc4_civic) -> u32 {
    with_c(c, 0, |h| h.sim.release())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_civic_init(c: *mut fc4_civic) -> bool {
    with_c(c, false, |h| h.sim.init())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_civic_shutdown(c: *mut fc4_civic) -> bool {
    with_c(c, false, |h| h.sim.shutdown())
}

/// `DoMessage` (§3): SimNewMonth, occupant inserted / removed (void1 = the
/// occupant), lot state changed (data2 = the new state, `unknown` = the
/// lot message's IGZUnknown). Always true.
#[no_mangle]
pub unsafe extern "C" fn fc4_civic_do_message(
    c: *mut fc4_civic,
    msg_type: u32,
    data1: u32,
    data2: u32,
    data3: u32,
    void1: u64,
    unknown: u64,
) -> bool {
    let m = CivicMessage {
        msg_type,
        data1,
        data2,
        data3,
        void1: opt(void1),
        unknown: opt(unknown),
    };
    with_c(c, true, |h| h.sim.do_message(&m))
}

/// Slot 5 `IsBuildingExcluded(id)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_civic_is_building_excluded(c: *const fc4_civic, id: u32) -> bool {
    with_c(c, false, |h| h.sim.is_building_excluded(id))
}

/// Slot 6 `GetConditionalBuildingStatus(key)`, copied out: false when there
/// is no entry. The tooltip is copied NUL-terminated into `text` (truncated
/// to `capacity − 1`; `text` may be NULL) and its full byte length stored in
/// `text_len`. Every out pointer may be NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_civic_get_status(
    c: *const fc4_civic,
    key: u32,
    status: *mut u8,
    building_count: *mut u16,
    text: *mut c_char,
    capacity: usize,
    text_len: *mut usize,
) -> bool {
    with_c(c, false, |h| {
        let Some(s) = h.sim.conditional_building_status(key) else {
            return false;
        };
        put(status, s.status);
        put(building_count, s.building_count);
        put(text_len, s.tooltip.len());
        if !text.is_null() && capacity > 0 {
            let n = s.tooltip.len().min(capacity - 1);
            std::ptr::copy_nonoverlapping(s.tooltip.as_ptr(), text as *mut u8, n);
            *text.add(n) = 0;
        }
        true
    })
}

/// Slot 7 `UpdateConditionalBuildingStatus(key, status)`: find or insert
/// `key`, then copy the status, count and `text_len` bytes of `text` (NULL
/// = empty). Returns true.
#[no_mangle]
pub unsafe extern "C" fn fc4_civic_update_status(
    c: *mut fc4_civic,
    key: u32,
    status: u8,
    building_count: u16,
    text: *const c_char,
    text_len: usize,
) -> bool {
    let tooltip = if text.is_null() || text_len == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(text as *const u8, text_len).to_vec()
    };
    let st = ConditionalBuildingStatus {
        status,
        building_count,
        tooltip,
    };
    with_c(c, false, |h| h.sim.update_conditional_building_status(key, &st))
}

/// The status keys in hash order (the record's order). Returns the number
/// of keys; copies at most `capacity` (`out` may be NULL).
#[no_mangle]
pub unsafe extern "C" fn fc4_civic_status_keys(c: *const fc4_civic, out: *mut u32, capacity: usize) -> usize {
    with_c(c, 0, |h| copy_out(&h.sim.snapshot().statuses.keys(), out, capacity))
}

/// The exclusion IDs in hash order. As [`fc4_civic_status_keys`].
#[no_mangle]
pub unsafe extern "C" fn fc4_civic_exclusions(c: *const fc4_civic, out: *mut u32, capacity: usize) -> usize {
    with_c(c, 0, |h| {
        copy_out(&h.sim.snapshot().exclusions.iter_order(), out, capacity)
    })
}

unsafe fn copy_out<T: Copy>(v: &[T], out: *mut T, capacity: usize) -> usize {
    if !out.is_null() {
        for (i, x) in v.iter().take(capacity).enumerate() {
            *out.add(i) = *x;
        }
    }
    v.len()
}

/// Slot 8 `GetLandmarkCount()`.
#[no_mangle]
pub unsafe extern "C" fn fc4_civic_landmark_count(c: *const fc4_civic) -> u32 {
    with_c(c, 0, |h| h.sim.landmark_count())
}

/// One of the two lists (`which` 0 = disease centers, 1 = maintained), as
/// tokens (0 = a null entry). As [`fc4_civic_status_keys`].
#[no_mangle]
pub unsafe extern "C" fn fc4_civic_list(c: *const fc4_civic, which: u32, out: *mut u64, capacity: usize) -> usize {
    with_c(c, 0, |h| {
        let s = h.sim.snapshot();
        let l = if which == 0 { s.disease_centers } else { s.maintained };
        let v: Vec<u64> = l.iter().map(|e| e.unwrap_or(0)).collect();
        copy_out(&v, out, capacity)
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_civic_get_state(c: *const fc4_civic, out: *mut fc4_civic_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_c(c, false, |h| {
        let s = h.sim.snapshot();
        *o = fc4_civic_state {
            struct_size: std::mem::size_of::<fc4_civic_state>() as u32,
            initialized: s.initialized.into(),
            destroyed: s.destroyed.into(),
            reserved0: 0,
            ref_count: s.ref_count,
            exclusions: s.exclusions.len() as u32,
            statuses: s.statuses.len() as u32,
            disease_centers: s.disease_centers.len() as u32,
            maintained: s.maintained.len() as u32,
            temporaries: s.temporaries,
            landmarks: s.landmarks.unwrap_or(0),
        };
        true
    })
}

/// `Write` (§7).
#[no_mangle]
pub unsafe extern "C" fn fc4_civic_write(c: *const fc4_civic, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_c(c, false, |h| h.sim.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

/// `Read` (§7): false unless the version is 6. References come back through
/// the host's `resolve_ref` (the department with IID `0x223E2B67`, the list
/// entries with `0x26BB767E`).
#[no_mangle]
pub unsafe extern "C" fn fc4_civic_read(c: *mut fc4_civic, inp: *const fc4_istream) -> bool {
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
