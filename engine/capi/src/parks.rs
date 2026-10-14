// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The park manager over the C ABI (fc4.h "park manager",
//! docs/specs/parks.md).
//!
//! The host implements [`fc4_park_host`]; the plugin's `cISC4ParkManager`
//! (with its message-target and serializable sub-objects) forwards its slots
//! here. The tuning exemplar is handed over at `Init` as an `fc4_props`
//! (NULL = no "Parks Manager" exemplar).
//!
//! Tokens are opaque host identities: occupants (a building and its
//! occupant are one token), the department, line items and lots. The core
//! holds one reference on the department and on every list entry; the
//! city, budget simulator, message server and lot manager are held through
//! the `acquire_*` / `release_*` pairs. The manager's own token (void1 of the
//! ZooEscape message) is given at creation.
//!
//! Threads: the core is single-threaded (`RefCell`). The plugin serialises
//! every call on one object (fc4.h).

use std::cell::RefCell;
use std::ffi::{c_char, c_void, CString};
use std::rc::Rc;

use fc4_core::budget::BudgetItem;
use fc4_core::parks::{escape_probability, query_interface, ParkHost, ParkManager, ParkMessage};
use fc4_core::property::PropertyHolder;
use fc4_core::stream::{RefValue, SaveObject};

use crate::budget::fc4_budget_item;
use crate::civic::{item_from_c, item_to_c};
use crate::guard;
use crate::props::{fc4_props, props_ref};
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

#[repr(C)]
pub struct fc4_park {
    _private: [u8; 0],
}

/// What the park manager needs from the game (fc4.h `fc4_park_host`). A
/// missing callback reads as 0 / false / none.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_park_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    // agents and messages
    pub add_agent: Option<unsafe extern "C" fn(U, u32, *const c_char, u32)>,
    pub remove_agent: Option<unsafe extern "C" fn(U)>,
    pub acquire_message_server: Option<unsafe extern "C" fn(U) -> bool>,
    pub subscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub release_message_server: Option<unsafe extern "C" fn(U)>,
    // services
    pub acquire_city: Option<unsafe extern "C" fn(U) -> bool>,
    pub release_city: Option<unsafe extern "C" fn(U)>,
    pub acquire_budget: Option<unsafe extern "C" fn(U) -> bool>,
    pub release_budget: Option<unsafe extern "C" fn(U)>,
    pub acquire_lot_manager: Option<unsafe extern "C" fn(U) -> bool>,
    pub release_lot_manager: Option<unsafe extern "C" fn(U)>,
    // references
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    pub resolve_ref: Option<unsafe extern "C" fn(U, u64, u32) -> u64>,
    // the department
    pub department_budget: Option<unsafe extern "C" fn(U, u32) -> u64>,
    pub create_department_budget: Option<unsafe extern "C" fn(U, u32, u32) -> u64>,
    pub set_department_name: Option<unsafe extern "C" fn(U, u64, u32, u32)>,
    pub set_max_funding: Option<unsafe extern "C" fn(U, u64, f32)>,
    pub department_funding: Option<unsafe extern "C" fn(U, u64, *mut f32) -> bool>,
    // budget items and line items
    pub budget_item: Option<unsafe extern "C" fn(U, u64, u32, *mut fc4_budget_item) -> bool>,
    pub change_budget_item_line: Option<unsafe extern "C" fn(U, u64, i32, u32)>,
    pub line_item_from_budget_item: Option<unsafe extern "C" fn(U, *const fc4_budget_item) -> u64>,
    pub line_item: Option<unsafe extern "C" fn(U, u64, u32) -> u64>,
    pub create_line_item: Option<unsafe extern "C" fn(U, u64, u32, bool) -> u64>,
    pub set_line_display_flag: Option<unsafe extern "C" fn(U, u64, u32, bool)>,
    pub add_to_full_expenses: Option<unsafe extern "C" fn(U, u64, i64)>,
    pub line_secondary: Option<unsafe extern "C" fn(U, u64) -> i64>,
    pub set_line_secondary: Option<unsafe extern "C" fn(U, u64, i64)>,
    // occupants
    pub occupant_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub occupant_has_property: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub is_occupant_group: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub is_building_occupant: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub building_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    // lots
    pub occupant_position_lot: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub set_lot_condition: Option<unsafe extern "C" fn(U, u64, u32)>,
    // the escape
    pub post_zoo_escape: Option<unsafe extern "C" fn(U, u64, u64)>,
    pub add_controllers: Option<unsafe extern "C" fn(U, *const c_char, u64)>,
}

/// Diagnostics (`fc4_park_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_park_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub destroyed: u8,
    pub tuning_read: u8,
    pub underfunded: u8,
    pub city_held: u8,
    pub budget_held: u8,
    pub message_server_held: u8,
    pub lot_manager_held: u8,
    pub ref_count: u32,
    pub parks: u32,
    pub zoos: u32,
    pub distress_threshold: f32,
    /// Kept points of the escape curve.
    pub curve_points: u32,
    pub reserved: u32,
    /// The Parks and Recreation department (0 = none).
    pub department: u64,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_park_state>() == 48);
    assert!(std::mem::size_of::<fc4_park_host>() == if cfg!(target_pointer_width = "32") { 160 } else { 320 });
};

// ---- host adapter -----------------------------------------------------------------------------------

struct CHost {
    h: fc4_park_host,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
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

impl ParkHost for CHost {
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        let n = c_name(name);
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
    fn acquire_budget(&self) -> bool {
        cb!(self, acquire_budget, false)
    }
    fn release_budget(&self) {
        cb!(self, release_budget, ())
    }
    fn acquire_lot_manager(&self) -> bool {
        cb!(self, acquire_lot_manager, false)
    }
    fn release_lot_manager(&self) {
        cb!(self, release_lot_manager, ())
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
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
    fn line_item_from_budget_item(&self, item: &BudgetItem) -> Option<u64> {
        let i = item_to_c(item);
        opt(cb!(self, line_item_from_budget_item, 0, &i))
    }
    fn line_item(&self, dept: u64, line: u32) -> Option<u64> {
        opt(cb!(self, line_item, 0, dept, line))
    }
    fn create_line_item(&self, dept: u64, line: u32, local: bool) -> Option<u64> {
        opt(cb!(self, create_line_item, 0, dept, line, local))
    }
    fn set_line_display_flag(&self, line: u64, flag: u32, on: bool) {
        cb!(self, set_line_display_flag, (), line, flag, on)
    }
    fn add_to_full_expenses(&self, line: u64, amount: i64) {
        cb!(self, add_to_full_expenses, (), line, amount)
    }
    fn line_secondary(&self, line: u64) -> i64 {
        cb!(self, line_secondary, 0, line)
    }
    fn set_line_secondary(&self, line: u64, value: i64) {
        cb!(self, set_line_secondary, (), line, value)
    }
    fn occupant_type(&self, occupant: u64) -> u32 {
        cb!(self, occupant_type, 0, occupant)
    }
    fn occupant_has_property(&self, occupant: u64, id: u32) -> bool {
        cb!(self, occupant_has_property, false, occupant, id)
    }
    fn is_occupant_group(&self, occupant: u64, group: u32) -> bool {
        cb!(self, is_occupant_group, false, occupant, group)
    }
    fn is_building_occupant(&self, occupant: u64) -> bool {
        cb!(self, is_building_occupant, false, occupant)
    }
    fn building_type(&self, building: u64) -> u32 {
        cb!(self, building_type, 0, building)
    }
    fn occupant_position_lot(&self, occupant: u64) -> Option<u64> {
        opt(cb!(self, occupant_position_lot, 0, occupant))
    }
    fn set_lot_condition(&self, lot: u64, condition: u32) {
        cb!(self, set_lot_condition, (), lot, condition)
    }
    fn post_zoo_escape(&self, manager: u64, zoo: u64) {
        cb!(self, post_zoo_escape, (), manager, zoo)
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
    sim: Rc<ParkManager>,
    host: Rc<CHost>,
}

fn with_p<R>(p: *const fc4_park, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_park*.
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
/// is copied. `session_seed` seeds the member generator (§5:
/// `derive(0xAA3FE6BC)`); `manager` is the manager's own token (void1 of
/// ZooEscape).
#[no_mangle]
pub unsafe extern "C" fn fc4_park_create(host: *const fc4_park_host, session_seed: u32, manager: u64) -> *mut fc4_park {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_park_host>() => {
            let c = Rc::new(CHost {
                h: *h,
                tuning: RefCell::new(None),
            });
            let sim = ParkManager::new(c.clone(), manager, session_seed);
            Box::into_raw(Box::new(Handle { sim, host: c })) as *mut fc4_park
        }
        _ => std::ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_park_destroy(p: *mut fc4_park) {
    if !p.is_null() {
        guard((), || drop(Box::from_raw(p as *mut Handle)));
    }
}

#[no_mangle]
pub extern "C" fn fc4_park_query_interface(iid: u32) -> bool {
    query_interface(iid)
}

#[no_mangle]
pub unsafe extern "C" fn fc4_park_add_ref(p: *mut fc4_park) -> u32 {
    with_p(p, 0, |h| h.sim.add_ref())
}

/// `Release` (§8): 0 when the object was destroyed (`Shutdown` ran); the
/// handle stays valid until [`fc4_park_destroy`].
#[no_mangle]
pub unsafe extern "C" fn fc4_park_release(p: *mut fc4_park) -> u32 {
    with_p(p, 0, |h| h.sim.release())
}

/// `Init` (§4.1). `tuning` is the "Parks Manager" exemplar (copied; NULL =
/// none); it is read at the object's first `Init` only.
#[no_mangle]
pub unsafe extern "C" fn fc4_park_init(p: *mut fc4_park, tuning: *const fc4_props) -> bool {
    let t = props_ref(tuning).cloned().map(Rc::new);
    with_p(p, false, |h| {
        *h.host.tuning.borrow_mut() = t;
        h.sim.init()
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_park_shutdown(p: *mut fc4_park) -> bool {
    with_p(p, false, |h| h.sim.shutdown())
}

/// `DoMessage` (§3): SimNewMonth, occupant inserted / removed (void1 = the
/// occupant). Always true.
#[no_mangle]
pub unsafe extern "C" fn fc4_park_do_message(
    p: *mut fc4_park,
    msg_type: u32,
    data1: u32,
    data2: u32,
    data3: u32,
    void1: u64,
) -> bool {
    let m = ParkMessage {
        msg_type,
        data1,
        data2,
        data3,
        void1: opt(void1),
    };
    with_p(p, true, |h| h.sim.do_message(&m))
}

/// The escape probability of §4.4 step 5 for a funding fraction, with the
/// object's curve.
#[no_mangle]
pub unsafe extern "C" fn fc4_park_escape_probability(p: *const fc4_park, funding: f32) -> f64 {
    with_p(p, 0.0, |h| escape_probability(&h.sim.snapshot().escape_curve, funding))
}

/// One of the two lists (`which` 0 = parks, 1 = zoos), as tokens (0 = a null
/// entry). Returns the length; copies at most `capacity` (`out` may be
/// NULL).
#[no_mangle]
pub unsafe extern "C" fn fc4_park_list(p: *const fc4_park, which: u32, out: *mut u64, capacity: usize) -> usize {
    with_p(p, 0, |h| {
        let s = h.sim.snapshot();
        let l = if which == 0 { s.parks } else { s.zoos };
        let v: Vec<u64> = l.iter().map(|e| e.unwrap_or(0)).collect();
        copy_out(&v, out, capacity)
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_park_get_state(p: *const fc4_park, out: *mut fc4_park_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_p(p, false, |h| {
        let s = h.sim.snapshot();
        *o = fc4_park_state {
            struct_size: std::mem::size_of::<fc4_park_state>() as u32,
            initialized: s.initialized.into(),
            destroyed: s.destroyed.into(),
            tuning_read: s.tuning_read.into(),
            underfunded: s.underfunded.into(),
            city_held: s.city_held.into(),
            budget_held: s.budget_held.into(),
            message_server_held: s.message_server_held.into(),
            lot_manager_held: s.lot_manager_held.into(),
            ref_count: s.ref_count,
            parks: s.parks.len() as u32,
            zoos: s.zoos.len() as u32,
            distress_threshold: s.distress_threshold,
            curve_points: s.escape_curve.points().len() as u32,
            reserved: 0,
            department: s.department.unwrap_or(0),
        };
        true
    })
}

/// `Write` (§7).
#[no_mangle]
pub unsafe extern "C" fn fc4_park_write(p: *const fc4_park, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_p(p, false, |h| h.sim.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

/// `Read` (§7): false unless the version is 2. List entries come back
/// through the host's `resolve_ref` (IID `0x26BB767E`).
#[no_mangle]
pub unsafe extern "C" fn fc4_park_read(p: *mut fc4_park, inp: *const fc4_istream) -> bool {
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
