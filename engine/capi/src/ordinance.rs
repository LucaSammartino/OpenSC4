// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The ordinance subsystem (`docs/specs/ordinance.md`) over the C ABI: the
//! simulator (`fc4_ordinance_sim`) and the standard ordinances
//! (`fc4_ordinance`). See the "ordinance" section of `include/fc4.h`.
//!
//! Every GZCOM ordinance object of the shim owns one `fc4_ordinance`; the
//! simulator's map holds the core objects of ours and wrappers (calling back
//! through `ordinance_call`) of foreign `cISC4Ordinance` objects.

use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::ordinance::{Ordinance, OrdinanceHost, OrdinanceKind, OrdinanceSim, StandardOrdinance};
use fc4_core::property::PropertyHolder;
use fc4_core::stream::{RefValue, ResKey, SaveObject};
use fc4_core::variant::Variant;

use crate::budget::fc4_token_callback;
use crate::props::fc4_props;
use crate::stream::{fc4_bytes_sink, fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};
use crate::{guard, slice_arg};

#[repr(C)]
pub struct fc4_ordinance_sim {
    _private: [u8; 0],
}

#[repr(C)]
pub struct fc4_ordinance {
    _private: [u8; 0],
}

type U = *mut c_void;

pub type fc4_key_visitor = Option<unsafe extern "C" fn(ctx: *mut c_void, type_id: u32, group: u32, instance: u32)>;

/// `cISC4Ordinance` operations for `fc4_ordinance_call` and the host's
/// `ordinance_call` (fc4.h `FC4_ORD_*`): the interface's methods in slot order,
/// without GetName / GetDescription / GetMiscProperties. Booleans are 0 / 1;
/// the two float getters return the `f32` bits.
pub mod op {
    pub const INIT: u32 = 0;
    pub const SHUTDOWN: u32 = 1;
    pub const CURRENT_MONTHLY_INCOME: u32 = 2;
    pub const ID: u32 = 3;
    pub const YEAR_FIRST_AVAILABLE: u32 = 4;
    pub const CHANCE_AVAILABILITY: u32 = 5;
    pub const ENACTMENT_INCOME: u32 = 6;
    pub const RETRACMENT_INCOME: u32 = 7;
    pub const MONTHLY_CONSTANT_INCOME: u32 = 8;
    pub const MONTHLY_INCOME_FACTOR: u32 = 9;
    pub const ADVISOR_ID: u32 = 10;
    pub const IS_AVAILABLE: u32 = 11;
    pub const IS_ON: u32 = 12;
    pub const IS_ENABLED: u32 = 13;
    pub const MONTHLY_ADJUSTED_INCOME: u32 = 14;
    pub const CHECK_CONDITIONS: u32 = 15;
    pub const IS_INCOME_ORDINANCE: u32 = 16;
    pub const SIMULATE: u32 = 17;
    pub const SET_AVAILABLE: u32 = 18;
    pub const SET_ON: u32 = 19;
    pub const SET_ENABLED: u32 = 20;
    pub const FORCE_AVAILABLE: u32 = 21;
    pub const FORCE_ON: u32 = 22;
    pub const FORCE_MONTHLY_ADJUSTED_INCOME: u32 = 23;
}

/// What the ordinance subsystem needs from the game (fc4.h:
/// `fc4_ordinance_host`). Missing callbacks read as "nothing / 0 / false".
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_ordinance_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    // resources
    pub for_each_exemplar_key: Option<unsafe extern "C" fn(U, fc4_key_visitor, *mut c_void)>,
    pub exemplar: Option<unsafe extern "C" fn(U, u32, u32, u32) -> *mut fc4_props>,
    pub ltext: Option<unsafe extern "C" fn(U, u32, u32, u32, fc4_bytes_sink, *mut c_void) -> bool>,
    // objects
    pub create_ordinance: Option<unsafe extern "C" fn(U, u32, *mut u64) -> *mut fc4_ordinance>,
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    pub foreign_ordinance: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub ordinance_call: Option<unsafe extern "C" fn(U, u64, u32, i64) -> i64>,
    pub ordinance_text: Option<unsafe extern "C" fn(U, u64, u32, fc4_bytes_sink, *mut c_void) -> bool>,
    pub ordinance_misc_property: Option<unsafe extern "C" fn(U, u64, u32, *mut fc4_props) -> bool>,
    // messages
    pub post: Option<unsafe extern "C" fn(U, u32, u32, u32)>,
    // city
    pub sim_year: Option<unsafe extern "C" fn(U, *mut u32) -> bool>,
    pub residential_population: Option<unsafe extern "C" fn(U) -> i32>,
    pub has_budget_simulator: Option<unsafe extern "C" fn(U) -> bool>,
    pub department_total_expenses: Option<unsafe extern "C" fn(U, u32, *mut i64) -> bool>,
    pub line_item_full_expenses: Option<unsafe extern "C" fn(U, u32, u32, *mut i64) -> bool>,
    pub landfill_capacity: Option<unsafe extern "C" fn(U) -> u32>,
    pub power_plant_count: Option<unsafe extern "C" fn(U, i32) -> i32>,
    pub police_station_count: Option<unsafe extern "C" fn(U) -> u32>,
    pub recycling_center_effect: Option<unsafe extern "C" fn(U) -> f32>,
    // LegalizeGambling
    pub lots_with_building_type: Option<unsafe extern "C" fn(U, u32, fc4_token_callback, *mut c_void)>,
    pub demolish_lots: Option<unsafe extern "C" fn(U, *const u64, usize)>,
    pub hide_conditional_building: Option<unsafe extern "C" fn(U, u32)>,
    pub remove_view_input_control: Option<unsafe extern "C" fn(U, u32)>,
}

/// Diagnostics (`fc4_ordinance_sim_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_ordinance_sim_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub loaded_from_save: u8,
    pub _pad: u16,
    pub count: u32,
    pub _pad2: u32,
    pub monthly_expense: i64,
    pub monthly_income: i64,
    pub pending: i64,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_ordinance_sim_state>() == 40);
    assert!(std::mem::size_of::<fc4_ordinance_host>() == if cfg!(target_pointer_width = "32") { 104 } else { 208 });
};

// ---- host adapter -----------------------------------------------------------------------------------

macro_rules! cb {
    ($h:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $h.$f { Some(f) => unsafe { f($h.user $(, $arg)*) }, None => $default }
    };
}

unsafe extern "C" fn collect_bytes(ctx: *mut c_void, data: *const u8, len: usize) {
    let v = &mut *(ctx as *mut Vec<u8>);
    if !data.is_null() && len > 0 {
        v.extend_from_slice(std::slice::from_raw_parts(data, len));
    }
}

unsafe extern "C" fn collect_key(ctx: *mut c_void, type_id: u32, group: u32, instance: u32) {
    (*(ctx as *mut Vec<ResKey>)).push(ResKey {
        type_id,
        group,
        instance,
    });
}

unsafe extern "C" fn collect_token(ctx: *mut c_void, token: u64) {
    (*(ctx as *mut Vec<u64>)).push(token);
}

/// Takes ownership of a holder the host built.
unsafe fn take_props(p: *mut fc4_props) -> Option<PropertyHolder> {
    (!p.is_null()).then(|| *Box::from_raw(p as *mut PropertyHolder))
}

struct CHost(fc4_ordinance_host);

impl OrdinanceHost for CHost {
    fn exemplar_keys(&self) -> Vec<ResKey> {
        let mut v: Vec<ResKey> = Vec::new();
        cb!(
            self.0,
            for_each_exemplar_key,
            (),
            Some(collect_key),
            &mut v as *mut Vec<ResKey> as *mut c_void
        );
        v
    }
    fn exemplar(&self, key: ResKey) -> Option<Rc<PropertyHolder>> {
        let p = cb!(
            self.0,
            exemplar,
            std::ptr::null_mut(),
            key.type_id,
            key.group,
            key.instance
        );
        // SAFETY: the callback returns NULL or a new fc4_props it hands over.
        unsafe { take_props(p) }.map(Rc::new)
    }
    fn ltext(&self, key: ResKey) -> Option<Vec<u8>> {
        let mut v: Vec<u8> = Vec::new();
        let found = cb!(
            self.0,
            ltext,
            false,
            key.type_id,
            key.group,
            key.instance,
            Some(collect_bytes),
            &mut v as *mut Vec<u8> as *mut c_void
        );
        found.then_some(v)
    }
    fn create_ordinance(&self, class_id: u32) -> Option<Rc<StandardOrdinance>> {
        let mut token = 0u64;
        let h = cb!(self.0, create_ordinance, std::ptr::null_mut(), class_id, &mut token);
        // SAFETY: NULL or a live fc4_ordinance* (the host's new object).
        let o = unsafe { ord_of(h) }?.clone();
        if token != 0 {
            o.set_token(token);
        }
        Some(o)
    }
    fn add_ref(&self, token: u64) {
        cb!(self.0, add_ref, (), token)
    }
    fn release(&self, token: u64) {
        cb!(self.0, release, (), token)
    }
    fn foreign_ordinance(&self, token: u64) -> Option<Rc<dyn Ordinance>> {
        let t = cb!(self.0, foreign_ordinance, 0, token);
        (t != 0).then(|| Rc::new(Foreign { h: self.0, token: t }) as Rc<dyn Ordinance>)
    }
    fn post(&self, msg_type: u32, data2: u32, data3: u32) {
        cb!(self.0, post, (), msg_type, data2, data3)
    }
    fn sim_year(&self) -> Option<u32> {
        let mut y = 0u32;
        cb!(self.0, sim_year, false, &mut y).then_some(y)
    }
    fn residential_population(&self) -> i32 {
        cb!(self.0, residential_population, 0)
    }
    fn has_budget_simulator(&self) -> bool {
        cb!(self.0, has_budget_simulator, false)
    }
    fn department_total_expenses(&self, dept: u32) -> Option<i64> {
        let mut v = 0i64;
        cb!(self.0, department_total_expenses, false, dept, &mut v).then_some(v)
    }
    fn line_item_full_expenses(&self, dept: u32, line: u32) -> Option<i64> {
        let mut v = 0i64;
        cb!(self.0, line_item_full_expenses, false, dept, line, &mut v).then_some(v)
    }
    fn landfill_capacity(&self) -> u32 {
        cb!(self.0, landfill_capacity, 0)
    }
    fn power_plant_count(&self, power_type: i32) -> i32 {
        cb!(self.0, power_plant_count, 0, power_type)
    }
    fn police_station_count(&self) -> u32 {
        cb!(self.0, police_station_count, 0)
    }
    fn recycling_center_effect(&self) -> f32 {
        cb!(self.0, recycling_center_effect, 0.0)
    }
    fn lots_with_building_type(&self, building_type: u32) -> Vec<u64> {
        let mut v: Vec<u64> = Vec::new();
        cb!(
            self.0,
            lots_with_building_type,
            (),
            building_type,
            Some(collect_token),
            &mut v as *mut Vec<u64> as *mut c_void
        );
        v
    }
    fn demolish_lots(&self, lots: &[u64]) {
        cb!(self.0, demolish_lots, (), lots.as_ptr(), lots.len())
    }
    fn hide_conditional_building(&self, building_type: u32) {
        cb!(self.0, hide_conditional_building, (), building_type)
    }
    fn remove_view_input_control(&self, control_id: u32) {
        cb!(self.0, remove_view_input_control, (), control_id)
    }
}

// ---- foreign ordinances -------------------------------------------------------------------------------

/// A `cISC4Ordinance` that is not ours (a DLL plugin's), called through the
/// host. The token carries one reference, released by whoever drops the
/// wrapper's map entry (the simulator).
struct Foreign {
    h: fc4_ordinance_host,
    token: u64,
}

impl Foreign {
    fn call(&self, op: u32, arg: i64) -> i64 {
        cb!(self.h, ordinance_call, 0, self.token, op, arg)
    }
    fn flag(&self, op: u32) -> bool {
        self.call(op, 0) != 0
    }
    fn set(&self, op: u32, b: bool) -> bool {
        self.call(op, i64::from(b)) != 0
    }
    fn float(&self, op: u32) -> f32 {
        f32::from_bits(self.call(op, 0) as u32)
    }
    fn text(&self, which: u32) -> Vec<u8> {
        let mut v: Vec<u8> = Vec::new();
        cb!(
            self.h,
            ordinance_text,
            false,
            self.token,
            which,
            Some(collect_bytes),
            &mut v as *mut Vec<u8> as *mut c_void
        );
        v
    }
}

impl Ordinance for Foreign {
    fn token(&self) -> u64 {
        self.token
    }
    fn init(&self) -> bool {
        self.flag(op::INIT)
    }
    fn shutdown(&self) -> bool {
        self.flag(op::SHUTDOWN)
    }
    fn current_monthly_income(&self) -> i64 {
        self.call(op::CURRENT_MONTHLY_INCOME, 0)
    }
    fn id(&self) -> u32 {
        self.call(op::ID, 0) as u32
    }
    fn name(&self) -> Vec<u8> {
        self.text(0)
    }
    fn description(&self) -> Vec<u8> {
        self.text(1)
    }
    fn year_first_available(&self) -> u32 {
        self.call(op::YEAR_FIRST_AVAILABLE, 0) as u32
    }
    fn chance_availability(&self) -> f32 {
        self.float(op::CHANCE_AVAILABILITY)
    }
    fn enactment_income(&self) -> i64 {
        self.call(op::ENACTMENT_INCOME, 0)
    }
    fn retracment_income(&self) -> i64 {
        self.call(op::RETRACMENT_INCOME, 0)
    }
    fn monthly_constant_income(&self) -> i64 {
        self.call(op::MONTHLY_CONSTANT_INCOME, 0)
    }
    fn monthly_income_factor(&self) -> f32 {
        self.float(op::MONTHLY_INCOME_FACTOR)
    }
    fn advisor_id(&self) -> u32 {
        self.call(op::ADVISOR_ID, 0) as u32
    }
    fn misc_property(&self, id: u32) -> Option<Variant> {
        let mut h = PropertyHolder::new();
        let ok = cb!(
            self.h,
            ordinance_misc_property,
            false,
            self.token,
            id,
            &mut h as *mut PropertyHolder as *mut fc4_props
        );
        if ok {
            h.get(id).cloned()
        } else {
            None
        }
    }
    fn is_available(&self) -> bool {
        self.flag(op::IS_AVAILABLE)
    }
    fn is_on(&self) -> bool {
        self.flag(op::IS_ON)
    }
    fn is_enabled(&self) -> bool {
        self.flag(op::IS_ENABLED)
    }
    fn monthly_adjusted_income(&self) -> i64 {
        self.call(op::MONTHLY_ADJUSTED_INCOME, 0)
    }
    fn check_conditions(&self) -> bool {
        self.flag(op::CHECK_CONDITIONS)
    }
    fn is_income_ordinance(&self) -> bool {
        self.flag(op::IS_INCOME_ORDINANCE)
    }
    fn simulate(&self) -> bool {
        self.flag(op::SIMULATE)
    }
    fn set_available(&self, available: bool) -> bool {
        self.set(op::SET_AVAILABLE, available)
    }
    fn set_on(&self, on: bool) -> bool {
        self.set(op::SET_ON, on)
    }
    fn set_enabled(&self, enabled: bool) -> bool {
        self.set(op::SET_ENABLED, enabled)
    }
    fn force_available(&self, available: bool) -> bool {
        self.set(op::FORCE_AVAILABLE, available)
    }
    fn force_on(&self, on: bool) -> bool {
        self.set(op::FORCE_ON, on)
    }
    fn force_monthly_adjusted_income(&self, v: i64) -> bool {
        self.call(op::FORCE_MONTHLY_ADJUSTED_INCOME, v) != 0
    }
    /// [choice] The class ID is not asked for: the shim writes references by
    /// token alone (fc4_ref.class_id is informational).
    fn save_ref(self: Rc<Self>) -> RefValue {
        RefValue::Host {
            token: self.token,
            class_id: 0,
        }
    }
}

/// One `cISC4Ordinance` operation on any ordinance (ours or foreign).
fn call_op(o: &dyn Ordinance, op: u32, arg: i64) -> i64 {
    let b = i64::from;
    match op {
        op::INIT => b(o.init()),
        op::SHUTDOWN => b(o.shutdown()),
        op::CURRENT_MONTHLY_INCOME => o.current_monthly_income(),
        op::ID => i64::from(o.id()),
        op::YEAR_FIRST_AVAILABLE => i64::from(o.year_first_available()),
        op::CHANCE_AVAILABILITY => i64::from(o.chance_availability().to_bits()),
        op::ENACTMENT_INCOME => o.enactment_income(),
        op::RETRACMENT_INCOME => o.retracment_income(),
        op::MONTHLY_CONSTANT_INCOME => o.monthly_constant_income(),
        op::MONTHLY_INCOME_FACTOR => i64::from(o.monthly_income_factor().to_bits()),
        op::ADVISOR_ID => i64::from(o.advisor_id()),
        op::IS_AVAILABLE => b(o.is_available()),
        op::IS_ON => b(o.is_on()),
        op::IS_ENABLED => b(o.is_enabled()),
        op::MONTHLY_ADJUSTED_INCOME => o.monthly_adjusted_income(),
        op::CHECK_CONDITIONS => b(o.check_conditions()),
        op::IS_INCOME_ORDINANCE => b(o.is_income_ordinance()),
        op::SIMULATE => b(o.simulate()),
        op::SET_AVAILABLE => b(o.set_available(arg != 0)),
        op::SET_ON => b(o.set_on(arg != 0)),
        op::SET_ENABLED => b(o.set_enabled(arg != 0)),
        op::FORCE_AVAILABLE => b(o.force_available(arg != 0)),
        op::FORCE_ON => b(o.force_on(arg != 0)),
        op::FORCE_MONTHLY_ADJUSTED_INCOME => b(o.force_monthly_adjusted_income(arg)),
        _ => 0,
    }
}

// ---- handles ----------------------------------------------------------------------------------------

struct SimHandle {
    sim: Rc<OrdinanceSim>,
    host: fc4_ordinance_host,
}

unsafe fn ord_of<'a>(o: *const fc4_ordinance) -> Option<&'a Rc<StandardOrdinance>> {
    (o as *const Rc<StandardOrdinance>).as_ref()
}

fn with_sim<R>(s: *const fc4_ordinance_sim, fallback: R, f: impl FnOnce(&SimHandle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_ordinance_sim*.
    match unsafe { (s as *const SimHandle).as_ref() } {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

fn with_ord<R>(o: *const fc4_ordinance, fallback: R, f: impl FnOnce(&Rc<StandardOrdinance>) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_ordinance*.
    match unsafe { ord_of(o) } {
        Some(x) => guard(fallback, || f(x)),
        None => fallback,
    }
}

fn host_of(host: *const fc4_ordinance_host) -> Option<fc4_ordinance_host> {
    // SAFETY: NULL or a valid struct (precondition).
    let h = unsafe { host.as_ref() }?;
    ((h.struct_size as usize) >= std::mem::size_of::<fc4_ordinance_host>()).then_some(*h)
}

/// Turns the core handle of one of our ordinance objects into its object.
fn decoder(class_id: u32, handle: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    OrdinanceKind::of_class(class_id)?;
    // SAFETY: the host's reference reads only report core handles of our own
    // objects, and only ordinance objects have these class IDs.
    unsafe { ord_of(handle as *const fc4_ordinance) }.map(|o| o.clone() as Rc<dyn SaveObject>)
}

/// The simulator's view of the ordinance `token` (ours when `o` is given).
fn entry_of(h: &SimHandle, token: u64, o: *const fc4_ordinance) -> Option<Rc<dyn Ordinance>> {
    if token == 0 {
        return None;
    }
    // SAFETY: NULL or a live fc4_ordinance* (precondition).
    match unsafe { ord_of(o) } {
        Some(x) => Some(x.clone() as Rc<dyn Ordinance>),
        None => Some(Rc::new(Foreign { h: h.host, token }) as Rc<dyn Ordinance>),
    }
}

// ---- simulator ----------------------------------------------------------------------------------------

/// Creates the simulator's core for the GZCOM object `token` (the IGZUnknown
/// posted as void1). The host table is copied.
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_create(
    host: *const fc4_ordinance_host,
    token: u64,
) -> *mut fc4_ordinance_sim {
    guard(std::ptr::null_mut(), || match host_of(host) {
        Some(h) => {
            let sim = OrdinanceSim::new(Rc::new(CHost(h)), token);
            Box::into_raw(Box::new(SimHandle { sim, host: h })) as *mut fc4_ordinance_sim
        }
        None => std::ptr::null_mut(),
    })
}

/// Destroys the core; every map entry's reference is released (§8.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_destroy(s: *mut fc4_ordinance_sim) {
    if !s.is_null() {
        guard((), || drop(Box::from_raw(s as *mut SimHandle)));
    }
}

/// `Init` (§8.1, core part: the agent is the shim's).
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_init(s: *mut fc4_ordinance_sim) -> bool {
    with_sim(s, false, |h| h.sim.init())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_shutdown(s: *mut fc4_ordinance_sim) -> bool {
    with_sim(s, false, |h| h.sim.shutdown())
}

/// `AddOrdinance` (§4.2). `token` = the `cISC4Ordinance*`; `o` = its core
/// object when it is one of ours, NULL for a foreign one.
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_add(s: *mut fc4_ordinance_sim, token: u64, o: *const fc4_ordinance) -> bool {
    with_sim(s, false, |h| match entry_of(h, token, o) {
        Some(e) => h.sim.add_ordinance(e),
        None => false,
    })
}

/// `RemoveOrdinance` (§4.2); arguments as for `fc4_ordinance_sim_add`.
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_remove(
    s: *mut fc4_ordinance_sim,
    token: u64,
    o: *const fc4_ordinance,
) -> bool {
    with_sim(s, false, |h| match entry_of(h, token, o) {
        Some(e) => h.sim.remove_ordinance(&*e),
        None => false,
    })
}

/// `IsOrdinanceAvailable` (0), `IsOrdinanceOn` (1),
/// `IsOrdinanceAvailableButOff` (2), `IsOrdinanceEnabled` (3).
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_query(s: *const fc4_ordinance_sim, which: u32, id: u32) -> bool {
    with_sim(s, false, |h| match which {
        0 => h.sim.is_ordinance_available(id),
        1 => h.sim.is_ordinance_on(id),
        2 => h.sim.is_ordinance_available_but_off(id),
        3 => h.sim.is_ordinance_enabled(id),
        _ => false,
    })
}

/// `SetOrdinanceAvailable` (0, §4.3), `SetOrdinanceOn` (1, §4.4),
/// `SetOrdinanceEnabled` (2).
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_set(s: *mut fc4_ordinance_sim, which: u32, id: u32, value: bool) -> bool {
    with_sim(s, false, |h| match which {
        0 => h.sim.set_ordinance_available(id, value),
        1 => h.sim.set_ordinance_on(id, value),
        2 => h.sim.set_ordinance_enabled(id, value),
        _ => false,
    })
}

/// `GetOrdinanceAvailableArray` (0), `GetOrdinanceOnArray` (1),
/// `GetOrdinanceIDArray` (2): writes IDs in key order while fewer than `*n`
/// were written (out may be NULL: capacity 0), sets `*n` to the number
/// written, returns the total. With `n` NULL it is just the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_array(
    s: *const fc4_ordinance_sim,
    which: u32,
    out: *mut u32,
    n: *mut u32,
) -> u32 {
    with_sim(s, 0, |h| {
        let cap = match (out.is_null(), n.as_ref()) {
            (false, Some(&c)) => c as usize,
            _ => 0,
        };
        let (ids, total) = match which {
            0 => h.sim.available_array(cap),
            1 => h.sim.on_array(cap),
            2 => h.sim.id_array(cap),
            _ => (Vec::new(), 0),
        };
        if !out.is_null() {
            std::ptr::copy_nonoverlapping(ids.as_ptr(), out, ids.len());
        }
        if let Some(r) = n.as_mut() {
            *r = ids.len() as u32;
        }
        total
    })
}

/// `GetOrdinanceAvailableCount` (0), `GetOrdinanceOnCount` (1).
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_count(s: *const fc4_ordinance_sim, which: u32) -> u32 {
    with_sim(s, 0, |h| match which {
        0 => h.sim.available_count(),
        1 => h.sim.on_count(),
        _ => 0,
    })
}

/// `GetOrdinanceByID`: the token of the entry (no reference added), 0 if none.
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_by_id(s: *const fc4_ordinance_sim, id: u32) -> u64 {
    with_sim(s, 0, |h| h.sim.ordinance_by_id(id).map_or(0, |o| o.token()))
}

/// `GetOrdinanceByName` (ASCII case ignored): the entry's token, 0 if none.
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_by_name(s: *const fc4_ordinance_sim, name: *const u8, len: usize) -> u64 {
    let Some(n) = slice_arg(name, len) else { return 0 };
    with_sim(s, 0, |h| h.sim.ordinance_by_name(n).map_or(0, |o| o.token()))
}

/// `GetOrdinanceIDByName`.
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_id_by_name(s: *const fc4_ordinance_sim, name: *const u8, len: usize) -> u32 {
    let Some(n) = slice_arg(name, len) else { return 0 };
    with_sim(s, 0, |h| h.sim.ordinance_id_by_name(n))
}

/// `GetOrdinanceMonthlyExpense` (0), `GetOrdinanceMonthlyIncome` (1).
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_money(s: *const fc4_ordinance_sim, which: u32) -> i64 {
    with_sim(s, 0, |h| {
        if which == 0 {
            h.sim.monthly_expense()
        } else {
            h.sim.monthly_income()
        }
    })
}

/// `GetCumulativeOffset` (§6.1): the 32-bit sum, sign-extended.
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_cumulative_offset(
    s: *const fc4_ordinance_sim,
    prop: u32,
    index: u32,
) -> i64 {
    with_sim(s, 0, |h| h.sim.cumulative_offset(prop, index))
}

/// `GetCumulativeFactor` (§6.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_cumulative_factor(
    s: *const fc4_ordinance_sim,
    prop: u32,
    index: u32,
) -> f32 {
    with_sim(s, 1.0, |h| h.sim.cumulative_factor(prop, index))
}

/// `DoMessage` (§5): SimNewMonth and SimBegin. Always true.
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_handle_message(s: *mut fc4_ordinance_sim, msg_type: u32) -> bool {
    with_sim(s, true, |h| h.sim.handle_message(msg_type))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_get_state(
    s: *const fc4_ordinance_sim,
    out: *mut fc4_ordinance_sim_state,
) -> bool {
    let Some(o) = out.as_mut() else { return false };
    if (o.struct_size as usize) < std::mem::size_of::<fc4_ordinance_sim_state>() {
        return false;
    }
    with_sim(s, false, |h| {
        let snap = h.sim.snapshot();
        *o = fc4_ordinance_sim_state {
            struct_size: o.struct_size,
            initialized: u8::from(snap.initialized),
            loaded_from_save: u8::from(snap.loaded_from_save),
            _pad: 0,
            count: snap.keys.len() as u32,
            _pad2: 0,
            monthly_expense: snap.monthly_expense,
            monthly_income: snap.monthly_income,
            pending: snap.pending,
        };
        true
    })
}

/// `Write` (§9.1). Ordinances go out as references by token.
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_write(s: *const fc4_ordinance_sim, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_sim(s, false, |h| h.sim.write(&mut COut(o)))
}

/// `Read` (§9.1). References to our ordinances arrive with their core handle;
/// foreign ones with one reference, offered to `foreign_ordinance`.
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_sim_read(s: *mut fc4_ordinance_sim, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_sim(s, false, |h| {
        h.sim.read(&mut CIn {
            stream: i,
            decode: decoder,
        })
    })
}

// ---- ordinances -----------------------------------------------------------------------------------------

/// Creates the core of a standard ordinance of class `class_id` (§3.6) for the
/// GZCOM object `token` (its `cISC4Ordinance*`). NULL for any other class.
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_create(
    host: *const fc4_ordinance_host,
    class_id: u32,
    token: u64,
) -> *mut fc4_ordinance {
    guard(std::ptr::null_mut(), || {
        let Some(h) = host_of(host) else {
            return std::ptr::null_mut();
        };
        match StandardOrdinance::new(Rc::new(CHost(h)), class_id, token) {
            Some(o) => Box::into_raw(Box::new(o)) as *mut fc4_ordinance,
            None => std::ptr::null_mut(),
        }
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_destroy(o: *mut fc4_ordinance) {
    if !o.is_null() {
        guard((), || drop(Box::from_raw(o as *mut Rc<StandardOrdinance>)));
    }
}

#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_class_id(o: *const fc4_ordinance) -> u32 {
    with_ord(o, 0, |x| x.class_id())
}

/// `SetKey` (concrete class, §3.5): the exemplar the ordinance loads from.
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_set_key(o: *mut fc4_ordinance, type_id: u32, group: u32, instance: u32) {
    with_ord(o, (), |x| {
        x.set_key(ResKey {
            type_id,
            group,
            instance,
        })
    })
}

/// The exemplar key (false if none).
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_get_key(
    o: *const fc4_ordinance,
    type_id: *mut u32,
    group: *mut u32,
    instance: *mut u32,
) -> bool {
    with_ord(o, false, |x| match x.key() {
        Some(k) => {
            for (p, v) in [(type_id, k.type_id), (group, k.group), (instance, k.instance)] {
                if let Some(r) = p.as_mut() {
                    *r = v;
                }
            }
            true
        }
        None => false,
    })
}

/// One `cISC4Ordinance` operation (`FC4_ORD_*`).
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_call(o: *mut fc4_ordinance, op: u32, arg: i64) -> i64 {
    with_ord(o, 0, |x| call_op(&**x, op, arg))
}

/// `GetName` (which 0) / `GetDescription` (1): the bytes go to `sink`.
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_text(
    o: *const fc4_ordinance,
    which: u32,
    sink: fc4_bytes_sink,
    ctx: *mut c_void,
) -> bool {
    with_ord(o, false, |x| {
        let t = if which == 0 { x.name() } else { x.description() };
        if let Some(f) = sink {
            f(ctx, t.as_ptr(), t.len());
        }
        true
    })
}

/// One property of `GetMiscProperties()` (own, then the parent exemplar),
/// copied into `out`; false if absent.
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_misc_property(o: *const fc4_ordinance, prop: u32, out: *mut fc4_props) -> bool {
    let Some(dst) = (out as *mut PropertyHolder).as_mut() else {
        return false;
    };
    with_ord(o, false, |x| match x.misc_property(prop) {
        Some(v) => {
            dst.add(prop, v);
            true
        }
        None => false,
    })
}

/// `Write` (§9.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_write(o: *const fc4_ordinance, out: *const fc4_ostream) -> bool {
    let Some(s) = out.as_ref() else { return false };
    if !ostream_ok(s) {
        return false;
    }
    with_ord(o, false, |x| x.write(&mut COut(s)))
}

/// `Read` (§9.2): any version; exemplar fields are refreshed from the current
/// exemplar when it exists.
#[no_mangle]
pub unsafe extern "C" fn fc4_ordinance_read(o: *mut fc4_ordinance, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_ord(o, false, |x| {
        x.read(&mut CIn {
            stream: i,
            decode: decoder,
        })
    })
}
