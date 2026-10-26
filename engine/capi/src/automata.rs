// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The automata layer over the C ABI (fc4.h "automata layer",
//! docs/specs/automata.md, short form).
//!
//! Six handle types:
//!
//! - `fc4_automata_object`: one controller (generator, attractor) or
//!   automaton (pedestrian, vehicle, emergency vehicle, patrol car, trailer)
//!   of the core, the referenced objects of §3.5 and §6. A handle owns one
//!   reference on the shared object; the core's managers hold their own.
//!   When the host's stream reads a reference to one of these classes, it
//!   passes the object's handle as `fc4_ref::core_handle`.
//! - `fc4_automata_scripts`: component 40, the script system (§2): the
//!   templates and occupant groups the Lua scripts define, and the record.
//! - `fc4_automata_cm`: component 41, the controller manager (§3, §7.1).
//! - `fc4_automata_manager`: components 43–46 (§4, §5, §7.3); U-Drive-It's
//!   pause and resume (§9).
//! - `fc4_automata_riot` / `fc4_automata_fire`: the riot leader and fire
//!   occupant hooks (§8) as host-facing operations; every game call goes
//!   through the host table, in the spec's order.
//!
//! References: tokens are opaque host identities. A reference read from a
//! stream comes with one reference owned by the reader (foundations §2.7).
//! The controller manager and the automata managers release, through their
//! host tables, every host reference they drop (removal, a replaced record
//! on Read, a failed Read, destroy) and take one (`add_ref`) on every host
//! object they start listing. The references inside an object's record are
//! the host's: `fc4_automata_object_refs` lists them.
//!
//! Threads: main thread only (the core uses `RefCell`).

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::{c_char, c_void, CStr, CString};
use std::rc::Rc;

use fc4_core::automata::{
    pause_all, resume_all, AutomataManager, Automaton, Controller, ControllerManager, ControllerManagerHost,
    ControllerManagerRecord, ControllerTemplate, ControllerTime, CreateArgs, FireHook, FireHost, ManagerKind,
    ManagerPart, ManagerRecord, OccupantGroup, RiotHost, RiotLeaders, ScriptSource, ScriptSystem, SetStateArgs,
    TemplateKind, Vec3, CLSID_ATTRACTOR, CLSID_GENERATOR, CLSID_PATROL_CAR, CLSID_PEDESTRIAN, CLSID_TRAILER,
    CLSID_VEHICLE,
};
use fc4_core::lot_records::{construction::ConstructionRecord, OccupantRecord};
use fc4_core::stream::{InStream, OutStream, RefValue, ResKey, SaveObject};

use crate::guard;
use crate::stream::{fc4_istream, fc4_ostream, fc4_ref, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

/// Shadow-only construction record comparison (lot-records §9). No lifecycle
/// calls: seed the live crane-presence bit from the captured writer's bit.
/// Release every reference acquired by the input, including on failed reads.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_construction_roundtrip(
    input: *const fc4_istream,
    output: *const fc4_ostream,
    refs: *const fc4_automata_ref_host,
) -> bool {
    guard(false, || {
        let (Some(input), Some(output), Some(refs)) = (in_stream(input), out_stream(output), table(refs)) else {
            return false;
        };
        let mut stream = tracking(input);
        let mut record = ConstructionRecord::new();
        let read = record.read(&mut stream);
        // Read stores this in the load-only flag; native Write uses whether a
        // live crane exists. This is an explicit same-input record comparison,
        // not a claim to reproduce CreateCrane or the post-load refresh.
        record.has_crane = record.loaded_has_crane;
        let ok = read && record.write(&mut COut(output));
        for token in stream.tokens {
            refs.release(token);
        }
        ok
    })
}

#[repr(C)]
pub struct fc4_automata_object {
    _private: [u8; 0],
}
#[repr(C)]
pub struct fc4_automata_scripts {
    _private: [u8; 0],
}
#[repr(C)]
pub struct fc4_automata_cm {
    _private: [u8; 0],
}
#[repr(C)]
pub struct fc4_automata_manager {
    _private: [u8; 0],
}
#[repr(C)]
pub struct fc4_automata_riot {
    _private: [u8; 0],
}
#[repr(C)]
pub struct fc4_automata_fire {
    _private: [u8; 0],
}

/// `fc4_automata_scripts_define_template` kinds.
pub const FC4_AUTOMATA_GENERATOR: u32 = 0;
pub const FC4_AUTOMATA_ATTRACTOR: u32 = 1;

/// No state (`fc4_automata_automaton_state::current_state`).
pub const FC4_AUTOMATA_NO_STATE: u32 = 0xFFFF_FFFF;

/// A controller time (§1 `time`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_automata_time {
    pub hour: f32,
    pub day: i16,
    pub reserved: u16,
}

/// `CreateAutomaton`'s seven arguments (§7.2).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_automata_create_args {
    /// Argument 1, the automata group ID.
    pub group: u32,
    /// Argument 2, the position, city metres.
    pub position: [f32; 3],
    /// Argument 3, the automata flags (bit 2 = self-controlling, `0x80`
    /// lets the creation through when the manager is full).
    pub flags: u32,
    /// Argument 4, the model key (`key3`: type, group, instance). All zero
    /// = the exemplar is chosen for the group.
    pub model_key: [u32; 3],
    /// Argument 5, the callback target (a host token); 0 = null.
    pub callback: u64,
    /// Argument 6, the priority; `0xFF` forces the creation.
    pub priority: u8,
    /// Argument 7's present flag (`const vec3*`; 0 = null).
    pub has_orientation: u8,
    pub reserved: u16,
    pub orientation: [f32; 3],
}

/// The C struct's seven arguments as the core's [`CreateArgs`] (§7.2).
fn to_create_args(a: &fc4_automata_create_args) -> CreateArgs {
    CreateArgs {
        group: a.group,
        position: a.position,
        flags: a.flags,
        model_key: ResKey {
            type_id: a.model_key[0],
            group: a.model_key[1],
            instance: a.model_key[2],
        },
        callback: a.callback,
        priority: a.priority,
        orientation: if a.has_orientation != 0 {
            Some(a.orientation)
        } else {
            None
        },
    }
}

/// The core's [`CreateArgs`] as the C struct (§7.2).
fn from_create_args(a: &CreateArgs) -> fc4_automata_create_args {
    fc4_automata_create_args {
        group: a.group,
        position: a.position,
        flags: a.flags,
        model_key: [a.model_key.type_id, a.model_key.group, a.model_key.instance],
        callback: a.callback,
        priority: a.priority,
        has_orientation: u8::from(a.orientation.is_some()),
        reserved: 0,
        orientation: a.orientation.unwrap_or([0.0; 3]),
    }
}

/// AddRef / Release on host objects (the managers and object reads).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_automata_ref_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
}

/// What the controller manager needs from the game.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_automata_cm_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    /// A log line (§3.4 step 2), UTF-8, NUL-terminated.
    pub log: Option<unsafe extern "C" fn(U, *const c_char)>,
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    /// A host controller's flags (its `CheckFlag`, slot 12).
    pub controller_flags: Option<unsafe extern "C" fn(U, u64) -> u32>,
}

/// The game calls of the riot leader hooks (§8.1).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_automata_riot_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    /// The nearest road tile's position; false = none.
    pub nearest_road_tile: Option<unsafe extern "C" fn(U, i32, i32, *mut f32) -> bool>,
    /// Whether the leader count `leaders` has reached the cap.
    pub leader_cap_reached: Option<unsafe extern "C" fn(U, u32) -> bool>,
    /// Pedestrian manager slot 15; 0 = null.
    pub create_automaton: Option<unsafe extern "C" fn(U, *const fc4_automata_create_args) -> u64>,
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    /// Automaton slot 51 `SetState(type, a, b, c, d, e, f)`.
    pub set_state: Option<unsafe extern "C" fn(U, u64, u32, u32, f32, u32, u32, u32, u32)>,
    /// Automaton slot 9.
    pub set_visible: Option<unsafe extern "C" fn(U, u64, bool)>,
    /// Automaton slot 5 `AsOccupant`.
    pub as_occupant: Option<unsafe extern "C" fn(U, u64) -> u64>,
    /// Disaster layer (city slot 72): register the leader's location.
    pub register_location: Option<unsafe extern "C" fn(U, u64, *const f32)>,
    /// Controller manager slot 5.
    pub add_controller_from_template_name: Option<unsafe extern "C" fn(U, u64, *const c_char, u32) -> bool>,
    /// Controller manager slot 14.
    pub remove_controllers: Option<unsafe extern "C" fn(U, u64)>,
    /// Disaster layer slot 15: unregister the location by occupant.
    pub unregister_location: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
}

/// The game calls of the fire occupant hooks (§8.2).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_automata_fire_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    pub occupant_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    /// Controller manager slot 10 `AddControllersFromOccupantGroup(name, occupant, extra)`.
    pub add_controllers_from_occupant_group_name: Option<unsafe extern "C" fn(U, *const c_char, u64, u32) -> bool>,
    /// Controller manager slot 14.
    pub remove_controllers: Option<unsafe extern "C" fn(U, u64)>,
}

/// A controller's state (`fc4_automata_controller_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_automata_controller_state {
    pub struct_size: u32,
    pub template_bound: u8,
    pub initialized: u8,
    pub in_grid: u8,
    pub shut_down: u8,
    pub flags: u32,
    /// The automata it controls.
    pub automata: u32,
    /// The occupant it is attached to (a core occupant: its host token).
    pub occupant_token: u64,
    pub occupant_class: u32,
    pub reserved: u32,
}

/// An automaton's state (`fc4_automata_automaton_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_automata_automaton_state {
    pub struct_size: u32,
    /// The automata group it was created for (0 = unknown; not saved).
    pub group: u32,
    pub position: [f32; 3],
    pub visible: u8,
    pub removed: u8,
    pub returning: u8,
    pub emergency: u8,
    /// The state stack's depth and top type (`FC4_AUTOMATA_NO_STATE`).
    pub states: u32,
    pub current_state: u32,
}

/// The controller manager's state (`fc4_automata_cm_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_automata_cm_state {
    pub struct_size: u32,
    pub loaded: u8,
    pub reserved: [u8; 3],
    pub occupants: u32,
    pub controllers: u32,
    pub sets: u32,
    pub time_a: fc4_automata_time,
    pub time_b: fc4_automata_time,
    pub clock_class: u32,
    pub clock_token: u64,
}

/// An automata manager's state (`fc4_automata_manager_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_automata_manager_state {
    pub struct_size: u32,
    /// The manager's class ID (§0).
    pub class_id: u32,
    pub initialized: u8,
    pub active: u8,
    pub paused: u8,
    pub loaded: u8,
    pub old_version: u8,
    pub time_scaling: u8,
    pub reserved: [u8; 2],
    pub max_count: i32,
    pub hard_limit: i32,
    pub automata: u32,
    pub group_counts: u32,
    /// The record and base versions a Write uses as read.
    pub version: u16,
    pub base_version: u16,
}

/// One riot leader (`fc4_automata_riot_leader`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_automata_riot_leader {
    pub automaton: u64,
    pub position: [f32; 3],
    pub stage: i32,
    pub max_stage: i32,
    pub reserved: u32,
}

const _: () = {
    const P: usize = std::mem::size_of::<*const c_void>();
    assert!(std::mem::size_of::<fc4_automata_time>() == 8);
    assert!(std::mem::size_of::<fc4_automata_create_args>() == 56);
    assert!(std::mem::size_of::<fc4_automata_ref_host>() == 2 * P + 2 * P);
    assert!(std::mem::size_of::<fc4_automata_cm_host>() == 2 * P + 4 * P);
    assert!(std::mem::size_of::<fc4_automata_riot_host>() == 2 * P + 12 * P);
    assert!(std::mem::size_of::<fc4_automata_fire_host>() == 2 * P + 3 * P);
    assert!(std::mem::size_of::<fc4_automata_controller_state>() == 32);
    assert!(std::mem::size_of::<fc4_automata_automaton_state>() == 32);
    assert!(std::mem::size_of::<fc4_automata_cm_state>() == 48);
    assert!(std::mem::size_of::<fc4_automata_manager_state>() == 36);
    assert!(std::mem::size_of::<fc4_automata_riot_leader>() == 32);
};

/// A host table is usable when it is present and at least as large as ours
/// (every table starts with its `u32` size).
unsafe fn table<T: Copy>(p: *const T) -> Option<T> {
    if p.is_null() || (*(p as *const u32) as usize) < std::mem::size_of::<T>() {
        return None;
    }
    Some(*p)
}

unsafe fn out_stream<'a>(p: *const fc4_ostream) -> Option<&'a fc4_ostream> {
    p.as_ref().filter(|s| ostream_ok(s))
}

unsafe fn in_stream<'a>(p: *const fc4_istream) -> Option<&'a fc4_istream> {
    p.as_ref().filter(|s| istream_ok(s))
}

fn time_to_c(t: ControllerTime) -> fc4_automata_time {
    fc4_automata_time {
        hour: t.hour,
        day: t.day,
        reserved: 0,
    }
}

fn time_from_c(t: &fc4_automata_time) -> ControllerTime {
    ControllerTime {
        hour: t.hour,
        day: t.day,
    }
}

unsafe fn c_str(p: *const c_char) -> Option<String> {
    (!p.is_null()).then(|| CStr::from_ptr(p).to_string_lossy().into_owned())
}

// ---- objects --------------------------------------------------------------------------------------------

enum Obj {
    Controller(Rc<Controller>),
    Automaton(Rc<Automaton>),
}

impl Obj {
    fn save(&self) -> Rc<dyn SaveObject> {
        match self {
            Obj::Controller(c) => c.clone(),
            Obj::Automaton(a) => a.clone(),
        }
    }

    fn from_ref(r: &RefValue) -> Option<Obj> {
        let RefValue::Object(o) = r else { return None };
        let any = o.clone().as_any_rc();
        match any.clone().downcast::<Controller>() {
            Ok(c) => Some(Obj::Controller(c)),
            Err(_) => any.downcast::<Automaton>().ok().map(Obj::Automaton),
        }
    }

    fn into_handle(self) -> *mut fc4_automata_object {
        Box::into_raw(Box::new(self)) as *mut fc4_automata_object
    }
}

/// The classes whose `fc4_ref::core_handle` is an `fc4_automata_object*`.
fn is_object_class(class: u32) -> bool {
    matches!(
        class,
        CLSID_GENERATOR | CLSID_ATTRACTOR | CLSID_PEDESTRIAN | CLSID_VEHICLE | CLSID_PATROL_CAR | CLSID_TRAILER
    )
}

/// Turns a core handle received through `fc4_ref::core_handle` into its
/// object. SAFETY: for the classes of [`is_object_class`], `handle` is NULL or
/// a live `fc4_automata_object*`.
pub(crate) unsafe fn decode_handle(class_id: u32, handle: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    if handle.is_null() || !is_object_class(class_id) {
        return None;
    }
    Some((*(handle as *const Obj)).save())
}

fn decoder(class_id: u32, handle: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    // SAFETY: the host passes only our own handles for our classes (fc4.h).
    unsafe { decode_handle(class_id, handle) }
}

/// A reference given by the host: null, one of our objects, or a host object.
unsafe fn ref_from_c(r: *const fc4_ref) -> RefValue {
    let Some(r) = r.as_ref() else { return RefValue::Null };
    if !r.core_handle.is_null() {
        if let Some(o) = decode_handle(r.class_id, r.core_handle) {
            return RefValue::Object(o);
        }
    }
    if r.token == 0 {
        return RefValue::Null;
    }
    RefValue::Host {
        token: r.token,
        class_id: r.class_id,
    }
}

/// A reference as the host sees it (core objects: their host token; no core
/// handle).
fn ref_to_c(r: &RefValue) -> fc4_ref {
    match r {
        RefValue::Null => fc4_ref::default(),
        RefValue::Object(o) => fc4_ref {
            token: o.host_token(),
            class_id: o.class_id(),
            ..Default::default()
        },
        RefValue::Host { token, class_id } => fc4_ref {
            token: *token,
            class_id: *class_id,
            ..Default::default()
        },
    }
}

fn host_token(r: &RefValue) -> Option<u64> {
    match r {
        RefValue::Host { token, .. } => Some(*token),
        _ => None,
    }
}

/// An `OutStream` that keeps only the host references a record writes.
#[derive(Default)]
struct RefRecorder {
    tokens: Vec<u64>,
}

impl OutStream for RefRecorder {
    fn put_u8(&mut self, _: u8) -> bool {
        true
    }
    fn put_u16(&mut self, _: u16) -> bool {
        true
    }
    fn put_u32(&mut self, _: u32) -> bool {
        true
    }
    fn put_u64(&mut self, _: u64) -> bool {
        true
    }
    fn put_f32(&mut self, _: f32) -> bool {
        true
    }
    fn put_f64(&mut self, _: f64) -> bool {
        true
    }
    fn put_bytes(&mut self, _: &[u8]) -> bool {
        true
    }
    fn put_str(&mut self, _: &[u8]) -> bool {
        true
    }
    fn put_ref(&mut self, r: &RefValue) -> bool {
        if let Some(t) = host_token(r) {
            self.tokens.push(t);
        }
        true
    }
    fn error(&self) -> i32 {
        0
    }
}

/// An `InStream` that remembers the host references it hands out, so a
/// failed read can give them back.
struct Tracking<'a> {
    inner: CIn<'a>,
    tokens: Vec<u64>,
}

impl InStream for Tracking<'_> {
    fn get_u8(&mut self) -> Option<u8> {
        self.inner.get_u8()
    }
    fn get_u16(&mut self) -> Option<u16> {
        self.inner.get_u16()
    }
    fn get_u32(&mut self) -> Option<u32> {
        self.inner.get_u32()
    }
    fn get_u64(&mut self) -> Option<u64> {
        self.inner.get_u64()
    }
    fn get_f32(&mut self) -> Option<f32> {
        self.inner.get_f32()
    }
    fn get_f64(&mut self) -> Option<f64> {
        self.inner.get_f64()
    }
    fn get_bytes(&mut self, n: usize) -> Option<Vec<u8>> {
        self.inner.get_bytes(n)
    }
    fn get_str(&mut self) -> Option<Vec<u8>> {
        self.inner.get_str()
    }
    fn get_ref(&mut self) -> Option<RefValue> {
        self.get_ref_as(0)
    }
    fn get_ref_as(&mut self, iid: u32) -> Option<RefValue> {
        let r = self.inner.get_ref_as(iid)?;
        if let Some(t) = host_token(&r) {
            self.tokens.push(t);
        }
        Some(r)
    }
    fn skip(&mut self, n: usize) -> bool {
        self.inner.skip(n)
    }
    fn error(&self) -> i32 {
        self.inner.error()
    }
}

fn tracking(s: &fc4_istream) -> Tracking<'_> {
    Tracking {
        inner: CIn {
            stream: s,
            decode: decoder,
        },
        tokens: Vec::new(),
    }
}

impl fc4_automata_ref_host {
    fn add_ref(&self, t: u64) {
        if let Some(f) = self.add_ref {
            // SAFETY: the creator promised valid callbacks.
            unsafe { f(self.user, t) }
        }
    }
    fn release(&self, t: u64) {
        if let Some(f) = self.release {
            // SAFETY: as above.
            unsafe { f(self.user, t) }
        }
    }
}

unsafe fn obj_of<'a>(o: *const fc4_automata_object) -> Option<&'a Obj> {
    (o as *const Obj).as_ref()
}

fn with_obj<R>(o: *const fc4_automata_object, fallback: R, f: impl FnOnce(&Obj) -> R) -> R {
    // SAFETY: documented precondition (NULL or a live handle).
    match unsafe { obj_of(o) } {
        Some(x) => guard(fallback, || f(x)),
        None => fallback,
    }
}

fn with_controller<R>(o: *const fc4_automata_object, fallback: R, f: impl FnOnce(&Controller) -> R) -> R {
    // SAFETY: as `with_obj`.
    match unsafe { obj_of(o) } {
        Some(Obj::Controller(c)) => guard(fallback, || f(c)),
        _ => fallback,
    }
}

fn with_automaton<R>(o: *const fc4_automata_object, fallback: R, f: impl FnOnce(&Automaton) -> R) -> R {
    // SAFETY: as `with_obj`.
    match unsafe { obj_of(o) } {
        Some(Obj::Automaton(a)) => guard(fallback, || f(a)),
        _ => fallback,
    }
}

/// A new object of `class_id` (generator, attractor, pedestrian, vehicle,
/// patrol car, trailer): an empty record of the current versions. NULL for
/// any other class. An emergency vehicle loads as a vehicle (§12 quirk 3);
/// make one with [`fc4_automata_object_create_emergency`].
#[no_mangle]
pub extern "C" fn fc4_automata_object_create(class_id: u32) -> *mut fc4_automata_object {
    guard(std::ptr::null_mut(), || {
        if let Some(c) = Controller::for_class(class_id) {
            return Obj::Controller(c).into_handle();
        }
        match Automaton::for_class(class_id) {
            Some(a) => Obj::Automaton(a).into_handle(),
            None => std::ptr::null_mut(),
        }
    })
}

/// A new emergency vehicle (§6.4; class `0x896E714A`, never saved by the
/// managers, §12 quirk 3).
#[no_mangle]
pub extern "C" fn fc4_automata_object_create_emergency() -> *mut fc4_automata_object {
    guard(std::ptr::null_mut(), || {
        Obj::Automaton(Automaton::new_emergency()).into_handle()
    })
}

/// Another handle to the same object.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_object_clone(o: *const fc4_automata_object) -> *mut fc4_automata_object {
    with_obj(o, std::ptr::null_mut(), |x| match x {
        Obj::Controller(c) => Obj::Controller(c.clone()).into_handle(),
        Obj::Automaton(a) => Obj::Automaton(a.clone()).into_handle(),
    })
}

/// Drops this handle's reference (the object lives on while the core holds it).
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_object_destroy(o: *mut fc4_automata_object) {
    if !o.is_null() {
        guard((), || drop(Box::from_raw(o as *mut Obj)));
    }
}

/// Whether two handles name the same object.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_object_same(
    a: *const fc4_automata_object,
    b: *const fc4_automata_object,
) -> bool {
    match (obj_of(a), obj_of(b)) {
        (Some(x), Some(y)) => std::ptr::eq(Rc::as_ptr(&x.save()) as *const (), Rc::as_ptr(&y.save()) as *const ()),
        _ => false,
    }
}

/// `GetGZCLSID`.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_object_class(o: *const fc4_automata_object) -> u32 {
    with_obj(o, 0, |x| x.save().class_id())
}

/// Whether it is a controller (else an automaton).
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_object_is_controller(o: *const fc4_automata_object) -> bool {
    with_obj(o, false, |x| matches!(x, Obj::Controller(_)))
}

/// `QueryInterface`: controllers answer `0x6A1857F8`, automata `0xA9B40F05`
/// and the occupant `0x26BB767E`; both `cIGZSerializable` and `cIGZUnknown`.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_object_query_interface(o: *const fc4_automata_object, iid: u32) -> bool {
    with_obj(o, false, |x| match x {
        Obj::Controller(c) => c.answers_iid(iid),
        Obj::Automaton(a) => a.answers_iid(iid),
    })
}

/// The host identity written for references to this object.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_object_set_token(o: *const fc4_automata_object, token: u64) {
    with_obj(o, (), |x| match x {
        Obj::Controller(c) => c.set_host_token(token),
        Obj::Automaton(a) => a.set_host_token(token),
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_automata_object_token(o: *const fc4_automata_object) -> u64 {
    with_obj(o, 0, |x| x.save().host_token())
}

/// Write (§3.5, §6): the record of the object's class.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_object_write(o: *const fc4_automata_object, out: *const fc4_ostream) -> bool {
    let Some(s) = out_stream(out) else { return false };
    with_obj(o, false, |x| x.save().write(&mut COut(s)))
}

/// Read: a record of the object's class; on failure nothing changes and the
/// host references read so far are released through `refs` (NULL = none).
/// The references the record holds after a successful read are the host's
/// (`fc4_automata_object_refs`).
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_object_read(
    o: *const fc4_automata_object,
    input: *const fc4_istream,
    refs: *const fc4_automata_ref_host,
) -> bool {
    let Some(s) = in_stream(input) else { return false };
    let refs = table(refs);
    with_obj(o, false, |x| {
        let mut t = tracking(s);
        let ok = x.save().read(&mut t);
        if !ok {
            if let Some(h) = refs {
                t.tokens.iter().for_each(|&k| h.release(k));
            }
        }
        ok
    })
}

/// The host references the object's record writes, in record order (each
/// once per occurrence). Returns the count; copies at most `capacity`.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_object_refs(
    o: *const fc4_automata_object,
    out: *mut u64,
    capacity: usize,
) -> usize {
    let tokens = with_obj(o, Vec::new(), |x| {
        let mut r = RefRecorder::default();
        x.save().write(&mut r);
        r.tokens
    });
    copy_out(&tokens, out, capacity)
}

unsafe fn copy_out<T: Copy>(v: &[T], out: *mut T, capacity: usize) -> usize {
    if !out.is_null() {
        for (i, x) in v.iter().take(capacity).enumerate() {
            *out.add(i) = *x;
        }
    }
    v.len()
}

/// Controller slot 12 `CheckFlag` (all flags); flag 8 marks a transient
/// controller that is never saved (§3.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_controller_flags(o: *const fc4_automata_object) -> u32 {
    with_controller(o, 0, |c| c.flags())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_automata_controller_set_flags(o: *const fc4_automata_object, flags: u32) {
    with_controller(o, (), |c| c.set_flags(flags))
}

/// The template name (UTF-8): returns its length; copies at most
/// `capacity - 1` bytes and a NUL into `out` (may be NULL).
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_controller_template(
    o: *const fc4_automata_object,
    out: *mut c_char,
    capacity: usize,
) -> usize {
    let name = with_controller(o, String::new(), |c| c.template_name());
    if !out.is_null() && capacity > 0 {
        let n = name.len().min(capacity - 1);
        std::ptr::copy_nonoverlapping(name.as_ptr() as *const c_char, out, n);
        *out.add(n) = 0;
    }
    name.len()
}

#[no_mangle]
pub unsafe extern "C" fn fc4_automata_controller_get_state(
    o: *const fc4_automata_object,
    out: *mut fc4_automata_controller_state,
) -> bool {
    let Some(out) = out.as_mut() else { return false };
    with_controller(o, false, |c| {
        let occ = ref_to_c(&c.occupant());
        *out = fc4_automata_controller_state {
            struct_size: std::mem::size_of::<fc4_automata_controller_state>() as u32,
            template_bound: c.template_bound() as u8,
            initialized: c.is_initialized() as u8,
            in_grid: c.in_grid() as u8,
            shut_down: c.is_shut_down() as u8,
            flags: c.flags(),
            automata: c.record().base.automata.len() as u32,
            occupant_token: occ.token,
            occupant_class: occ.class_id,
            reserved: 0,
        };
        true
    })
}

/// Automaton slot 51 `SetState(type, controller, timeout_ms, final, animId,
/// sfxId, template)` (§6.2): the state stack becomes one new state of that
/// type. **False, with nothing changed, while the top state has `final`**
/// (§12 quirk 11) — the riot end's `SetState(1, …)` is such a call (§8.1).
#[allow(clippy::too_many_arguments)]
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_automaton_set_state(
    o: *const fc4_automata_object,
    state_type: u32,
    a: u32,
    b: f32,
    c: u32,
    d: u32,
    e: u32,
    f: u32,
) -> bool {
    with_automaton(o, false, |x| {
        x.set_state(
            state_type,
            SetStateArgs {
                controller: a,
                timeout_ms: b,
                final_flag: c,
                anim_id: d,
                sfx_id: e,
                template: f,
            },
        )
    })
}

/// Slot 52 `PushState` (the same seven arguments; §6.2). `PushState(0, 0, …)`
/// delegates to `SetState`, so it can return false the same way.
#[allow(clippy::too_many_arguments)]
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_automaton_push_state(
    o: *const fc4_automata_object,
    state_type: u32,
    a: u32,
    b: f32,
    c: u32,
    d: u32,
    e: u32,
    f: u32,
) -> bool {
    with_automaton(o, false, |x| {
        x.push_state(
            state_type,
            SetStateArgs {
                controller: a,
                timeout_ms: b,
                final_flag: c,
                anim_id: d,
                sfx_id: e,
                template: f,
            },
        )
    })
}

/// Slot 53 `PopState`: false when the stack is empty. A `final` top that is
/// not already type 1 pushes DISAPPEAR instead of revealing the state below
/// (§12 quirk 11).
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_automaton_pop_state(o: *const fc4_automata_object) -> bool {
    with_automaton(o, false, |x| x.pop_state())
}

/// Slot 9 `SetVisible`.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_automaton_set_visible(o: *const fc4_automata_object, visible: bool) {
    with_automaton(o, (), |x| x.set_visible(visible))
}

/// Slot 8 `SetPosition`.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_automaton_set_position(o: *const fc4_automata_object, position: *const f32) {
    if position.is_null() {
        return;
    }
    let p: Vec3 = [*position, *position.add(1), *position.add(2)];
    with_automaton(o, (), |x| x.set_position(p))
}

/// Slot 14 `SetOrientation`: copy the caller's vector and normalize it.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_automaton_set_orientation(
    o: *const fc4_automata_object,
    orientation: *const f32,
) {
    if orientation.is_null() {
        return;
    }
    let v = [*orientation, *orientation.add(1), *orientation.add(2)];
    with_automaton(o, (), |x| x.set_orientation(v))
}

/// Slot 30 `SetAutomataFlag`: change only the requested mask.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_automaton_set_flag(o: *const fc4_automata_object, mask: u32, on: bool) {
    with_automaton(o, (), |x| x.set_automata_flag(mask, on))
}

/// Slot 13 `Remove` (dispatch despawn).
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_automaton_remove(o: *const fc4_automata_object) {
    with_automaton(o, (), |x| x.remove())
}

/// Slot 29 `ReturnToSourceOccupant` (dispatch "go home").
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_automaton_return_to_source(o: *const fc4_automata_object) {
    with_automaton(o, (), |x| x.return_to_source_occupant())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_automata_automaton_get_state(
    o: *const fc4_automata_object,
    out: *mut fc4_automata_automaton_state,
) -> bool {
    let Some(out) = out.as_mut() else { return false };
    with_automaton(o, false, |x| {
        let states = x.record().base().states.len();
        *out = fc4_automata_automaton_state {
            struct_size: std::mem::size_of::<fc4_automata_automaton_state>() as u32,
            group: x.group(),
            position: x.position(),
            visible: x.is_visible() as u8,
            removed: x.is_removed() as u8,
            returning: x.is_returning() as u8,
            emergency: x.is_emergency() as u8,
            states: states as u32,
            current_state: x.current_state().unwrap_or(FC4_AUTOMATA_NO_STATE),
        };
        true
    })
}

// ---- the script system (component 40) ---------------------------------------------------------------------

unsafe fn scripts_of<'a>(s: *const fc4_automata_scripts) -> Option<&'a RefCell<ScriptSystem>> {
    (s as *const RefCell<ScriptSystem>).as_ref()
}

fn with_scripts<R>(s: *const fc4_automata_scripts, fallback: R, f: impl FnOnce(&mut ScriptSystem) -> R) -> R {
    // SAFETY: documented precondition.
    match unsafe { scripts_of(s) }.and_then(|c| c.try_borrow_mut().ok()) {
        Some(mut x) => guard(fallback, || f(&mut x)),
        None => fallback,
    }
}

/// A new script system with no templates and no groups.
#[no_mangle]
pub extern "C" fn fc4_automata_scripts_create() -> *mut fc4_automata_scripts {
    guard(std::ptr::null_mut(), || {
        Box::into_raw(Box::new(RefCell::new(ScriptSystem::new()))) as *mut fc4_automata_scripts
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_automata_scripts_destroy(s: *mut fc4_automata_scripts) {
    if !s.is_null() {
        guard((), || drop(Box::from_raw(s as *mut RefCell<ScriptSystem>)));
    }
}

#[no_mangle]
pub unsafe extern "C" fn fc4_automata_scripts_init(s: *mut fc4_automata_scripts) -> bool {
    with_scripts(s, false, |x| x.init())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_automata_scripts_shutdown(s: *mut fc4_automata_scripts) -> bool {
    with_scripts(s, false, |x| x.shutdown())
}

/// Script slot 16 `CreateControllerTemplate`: `kind` is
/// `FC4_AUTOMATA_GENERATOR` or `FC4_AUTOMATA_ATTRACTOR`; a template of the
/// same name is replaced. False for a NULL name or an unknown kind.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_scripts_define_template(
    s: *mut fc4_automata_scripts,
    name: *const c_char,
    kind: u32,
    flags: u32,
) -> bool {
    let Some(name) = c_str(name) else { return false };
    let kind = match kind {
        FC4_AUTOMATA_GENERATOR => TemplateKind::Generator,
        FC4_AUTOMATA_ATTRACTOR => TemplateKind::Attractor,
        _ => return false,
    };
    with_scripts(s, false, |x| {
        let mut t = ControllerTemplate::new(&name, kind);
        t.flags = flags;
        x.define_template(t);
        true
    })
}

/// Script slot 13 `CreateOccupantGroup`: an ID, a name and the template
/// names it adds, in order; a group with the same ID is replaced.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_scripts_define_group(
    s: *mut fc4_automata_scripts,
    id: u32,
    name: *const c_char,
    templates: *const *const c_char,
    count: usize,
) -> bool {
    let Some(name) = c_str(name) else { return false };
    if templates.is_null() && count > 0 {
        return false;
    }
    let mut names = Vec::with_capacity(count);
    for i in 0..count {
        match c_str(*templates.add(i)) {
            Some(n) => names.push(n),
            None => return false,
        }
    }
    with_scripts(s, false, |x| {
        x.define_occupant_group(OccupantGroup {
            id,
            name,
            templates: names,
        });
        true
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_automata_scripts_template_count(s: *const fc4_automata_scripts) -> usize {
    with_scripts(s as *mut _, 0, |x| x.templates().len())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_automata_scripts_group_count(s: *const fc4_automata_scripts) -> usize {
    with_scripts(s as *mut _, 0, |x| x.groups().len())
}

/// Write (§2): `u16` 1.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_scripts_write(s: *const fc4_automata_scripts, out: *const fc4_ostream) -> bool {
    let Some(o) = out_stream(out) else { return false };
    with_scripts(s as *mut _, false, |x| x.write(&mut COut(o)))
}

/// Read (§2): false unless the version is 1.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_scripts_read(s: *mut fc4_automata_scripts, input: *const fc4_istream) -> bool {
    let Some(i) = in_stream(input) else { return false };
    with_scripts(s, false, |x| x.read(&mut tracking(i)))
}

/// A snapshot of the script set (the core borrows it for one call).
fn script_snapshot(s: *const fc4_automata_scripts) -> Option<ScriptSystem> {
    // SAFETY: documented precondition.
    unsafe { scripts_of(s) }.and_then(|c| c.try_borrow().ok().map(|x| x.clone()))
}

// ---- the controller manager (component 41) ------------------------------------------------------------------

struct CmHost(fc4_automata_cm_host);

impl ControllerManagerHost for CmHost {
    fn log(&self, text: &str) {
        if let Some(f) = self.0.log {
            let s = CString::new(text).unwrap_or_default();
            // SAFETY: valid callbacks (creator's promise); `s` outlives the call.
            unsafe { f(self.0.user, s.as_ptr()) }
        }
    }
    fn release(&self, token: u64) {
        if let Some(f) = self.0.release {
            // SAFETY: as above.
            unsafe { f(self.0.user, token) }
        }
    }
    fn add_ref(&self, token: u64) {
        if let Some(f) = self.0.add_ref {
            // SAFETY: as above.
            unsafe { f(self.0.user, token) }
        }
    }
    fn controller_flags(&self, token: u64) -> u32 {
        match self.0.controller_flags {
            // SAFETY: as above.
            Some(f) => unsafe { f(self.0.user, token) },
            None => 0,
        }
    }
}

struct CmHandle {
    cm: RefCell<ControllerManager>,
    host: Option<CmHost>,
}

impl CmHandle {
    fn release_all(&self, tokens: &[u64]) {
        if let Some(h) = &self.host {
            tokens.iter().for_each(|&t| h.release(t));
        }
    }
}

/// Every host reference a controller manager record holds.
fn cm_tokens(r: &ControllerManagerRecord) -> Vec<u64> {
    let mut v: Vec<u64> = host_token(&r.clock).into_iter().collect();
    for e in &r.entries {
        v.extend(host_token(&e.occupant));
        v.extend(e.controllers.iter().filter_map(host_token));
    }
    for s in &r.sets {
        v.extend(host_token(&s.occupant));
        v.extend(s.automata.iter().filter_map(host_token));
    }
    v
}

unsafe fn cm_of<'a>(c: *const fc4_automata_cm) -> Option<&'a CmHandle> {
    (c as *const CmHandle).as_ref()
}

fn with_cm<R>(c: *const fc4_automata_cm, fallback: R, f: impl FnOnce(&CmHandle, &mut ControllerManager) -> R) -> R {
    // SAFETY: documented precondition.
    match unsafe { cm_of(c) } {
        Some(h) => match h.cm.try_borrow_mut() {
            Ok(mut m) => guard(fallback, || f(h, &mut m)),
            Err(_) => fallback,
        },
        None => fallback,
    }
}

/// A new controller manager (not loaded). The host table is copied; NULL
/// (or a short table) = no callbacks.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_cm_create(host: *const fc4_automata_cm_host) -> *mut fc4_automata_cm {
    let h = table(host);
    guard(std::ptr::null_mut(), || {
        let core_host: Box<dyn ControllerManagerHost> = match h {
            Some(t) => Box::new(CmHost(t)),
            None => Box::new(fc4_core::automata::NoControllerHost),
        };
        Box::into_raw(Box::new(CmHandle {
            cm: RefCell::new(ControllerManager::new(core_host)),
            host: h.map(CmHost),
        })) as *mut fc4_automata_cm
    })
}

/// Releases every host reference the record holds, then frees the manager.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_cm_destroy(c: *mut fc4_automata_cm) {
    if c.is_null() {
        return;
    }
    guard((), || {
        let h = Box::from_raw(c as *mut CmHandle);
        let tokens = h.cm.try_borrow().map(|m| cm_tokens(m.record())).unwrap_or_default();
        h.release_all(&tokens);
    });
}

/// Write (§3.1): controllers with flag 8 are left out (§3.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_cm_write(c: *const fc4_automata_cm, out: *const fc4_ostream) -> bool {
    let Some(o) = out_stream(out) else { return false };
    with_cm(c, false, |_, m| m.write(&mut COut(o)))
}

/// Read (§3.1): sets the loaded flag; a repeated occupant's controllers are
/// dropped (§12 quirk 2). On success the replaced record's references are
/// released; on failure nothing changes.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_cm_read(c: *mut fc4_automata_cm, input: *const fc4_istream) -> bool {
    let Some(i) = in_stream(input) else { return false };
    with_cm(c, false, |h, m| {
        let old = cm_tokens(m.record());
        let ok = m.read(&mut CIn {
            stream: i,
            decode: decoder,
        });
        if ok {
            h.release_all(&old);
        }
        ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_automata_cm_is_loaded(c: *const fc4_automata_cm) -> bool {
    with_cm(c, false, |_, m| m.is_loaded())
}

/// The 24-hour clock reference (§3.1 field 2); the manager takes a reference.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_cm_set_clock(c: *mut fc4_automata_cm, clock: *const fc4_ref) {
    let r = ref_from_c(clock);
    with_cm(c, (), |h, m| {
        if let Some(host) = &h.host {
            host_token(&r).into_iter().for_each(|t| host.add_ref(t));
            host_token(&m.record().clock).into_iter().for_each(|t| host.release(t));
        }
        m.set_clock(r)
    })
}

/// The two controller times (§3.1 fields 3 and 4).
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_cm_set_times(
    c: *mut fc4_automata_cm,
    a: *const fc4_automata_time,
    b: *const fc4_automata_time,
) -> bool {
    let (Some(a), Some(b)) = (a.as_ref(), b.as_ref()) else {
        return false;
    };
    with_cm(c, false, |_, m| {
        m.set_times(time_from_c(a), time_from_c(b));
        true
    })
}

/// Slot 24 `GetControllerTime`: time B.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_cm_controller_time(
    c: *const fc4_automata_cm,
    out: *mut fc4_automata_time,
) -> bool {
    let Some(out) = out.as_mut() else { return false };
    with_cm(c, false, |_, m| {
        *out = time_to_c(m.controller_time());
        true
    })
}

/// Slot 5 `AddControllerFromTemplateName(occupant, name, extra)`: NULL when
/// the scripts (NULL = none) define no such template; otherwise a new
/// handle to the new controller (the caller destroys it).
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_cm_add_controller_from_template_name(
    c: *mut fc4_automata_cm,
    scripts: *const fc4_automata_scripts,
    occupant: *const fc4_ref,
    name: *const c_char,
    extra: u32,
) -> *mut fc4_automata_object {
    let Some(name) = c_str(name) else {
        return std::ptr::null_mut();
    };
    let Some(sc) = script_snapshot(scripts) else {
        return std::ptr::null_mut();
    };
    let occ = ref_from_c(occupant);
    with_cm(c, std::ptr::null_mut(), |_, m| {
        match m.add_controller_from_template_name(&sc, &occ, &name, extra) {
            Some(ctl) => Obj::Controller(ctl).into_handle(),
            None => std::ptr::null_mut(),
        }
    })
}

/// Slot 9 `AddControllersFromOccupantGroup(groupID, occupant, extra)`: false
/// with no script system (NULL) or no such group.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_cm_add_controllers_from_group_id(
    c: *mut fc4_automata_cm,
    scripts: *const fc4_automata_scripts,
    group_id: u32,
    occupant: *const fc4_ref,
    extra: u32,
) -> bool {
    let sc = script_snapshot(scripts);
    let occ = ref_from_c(occupant);
    with_cm(c, false, |_, m| {
        m.add_controllers_from_occupant_group_id(sc.as_ref().map(|s| s as &dyn ScriptSource), group_id, &occ, extra)
    })
}

/// Slot 10, by name.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_cm_add_controllers_from_group_name(
    c: *mut fc4_automata_cm,
    scripts: *const fc4_automata_scripts,
    name: *const c_char,
    occupant: *const fc4_ref,
    extra: u32,
) -> bool {
    let Some(name) = c_str(name) else { return false };
    let sc = script_snapshot(scripts);
    let occ = ref_from_c(occupant);
    with_cm(c, false, |_, m| {
        m.add_controllers_from_occupant_group_name(sc.as_ref().map(|s| s as &dyn ScriptSource), &name, &occ, extra)
    })
}

/// Slot 11 `RemoveControllersFromOccupantGroup(groupID, occupant)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_cm_remove_controllers_from_group_id(
    c: *mut fc4_automata_cm,
    scripts: *const fc4_automata_scripts,
    group_id: u32,
    occupant: *const fc4_ref,
) -> bool {
    let sc = script_snapshot(scripts);
    let occ = ref_from_c(occupant);
    with_cm(c, false, |_, m| {
        m.remove_controllers_from_occupant_group_id(sc.as_ref().map(|s| s as &dyn ScriptSource), group_id, &occ)
    })
}

/// Slot 12, by name.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_cm_remove_controllers_from_group_name(
    c: *mut fc4_automata_cm,
    scripts: *const fc4_automata_scripts,
    name: *const c_char,
    occupant: *const fc4_ref,
) -> bool {
    let Some(name) = c_str(name) else { return false };
    let sc = script_snapshot(scripts);
    let occ = ref_from_c(occupant);
    with_cm(c, false, |_, m| {
        m.remove_controllers_from_occupant_group_name(sc.as_ref().map(|s| s as &dyn ScriptSource), &name, &occ)
    })
}

/// Slot 14 `RemoveControllers(occupant)`: the count removed.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_cm_remove_controllers(
    c: *mut fc4_automata_cm,
    occupant: *const fc4_ref,
) -> usize {
    let occ = ref_from_c(occupant);
    with_cm(c, 0, |_, m| m.remove_controllers(&occ))
}

/// `RemoveOccupant` (`0x99EF1143`, §10): the occupant's controllers and
/// automaton sets go.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_cm_occupant_removed(c: *mut fc4_automata_cm, occupant: *const fc4_ref) {
    let occ = ref_from_c(occupant);
    with_cm(c, (), |_, m| m.occupant_removed(&occ))
}

/// `PostCityInit` (`0x26D31EC1`, §3.4): only when loaded, every saved
/// controller is bound to its template; unknown templates are logged and
/// dropped. Returns the number dropped; -1 when `scripts` is NULL (nothing
/// happens).
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_cm_post_city_init(
    c: *mut fc4_automata_cm,
    scripts: *const fc4_automata_scripts,
) -> i32 {
    let Some(sc) = script_snapshot(scripts) else { return -1 };
    with_cm(c, -1, |_, m| m.post_city_init(&sc).len() as i32)
}

/// Every controller, occupant entry by entry.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_cm_controller_count(c: *const fc4_automata_cm) -> usize {
    with_cm(c, 0, |_, m| m.all_controllers().len())
}

/// Controller `index` (in `fc4_automata_cm_controller_count` order): `out`
/// (may be NULL) gets its reference; returns a new handle for a core
/// controller (the caller destroys it), NULL for a host controller or out of
/// range.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_cm_controller_at(
    c: *const fc4_automata_cm,
    index: usize,
    out: *mut fc4_ref,
) -> *mut fc4_automata_object {
    with_cm(c, std::ptr::null_mut(), |_, m| {
        let all = m.all_controllers();
        let Some(r) = all.get(index) else {
            return std::ptr::null_mut();
        };
        if let Some(o) = out.as_mut() {
            *o = ref_to_c(r);
        }
        Obj::from_ref(r).map_or(std::ptr::null_mut(), Obj::into_handle)
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_automata_cm_get_state(c: *const fc4_automata_cm, out: *mut fc4_automata_cm_state) -> bool {
    let Some(out) = out.as_mut() else { return false };
    with_cm(c, false, |_, m| {
        let r = m.record();
        let clock = ref_to_c(&r.clock);
        *out = fc4_automata_cm_state {
            struct_size: std::mem::size_of::<fc4_automata_cm_state>() as u32,
            loaded: m.is_loaded() as u8,
            reserved: [0; 3],
            occupants: r.entries.len() as u32,
            controllers: m.all_controllers().len() as u32,
            sets: r.sets.len() as u32,
            time_a: time_to_c(r.time_a),
            time_b: time_to_c(r.time_b),
            clock_class: clock.class_id,
            clock_token: clock.token,
        };
        true
    })
}

// ---- the automata managers (components 43–46) ---------------------------------------------------------------

struct MgrHandle {
    m: RefCell<AutomataManager>,
    host: Option<fc4_automata_ref_host>,
}

impl MgrHandle {
    fn release_all(&self, tokens: &[u64]) {
        if let Some(h) = &self.host {
            tokens.iter().for_each(|&t| h.release(t));
        }
    }

    /// Releases the references `before` has and `after` no longer has.
    fn release_dropped(&self, before: &[u64], after: &[u64]) {
        let mut left: BTreeMap<u64, i64> = BTreeMap::new();
        for &t in before {
            *left.entry(t).or_default() += 1;
        }
        for &t in after {
            *left.entry(t).or_default() -= 1;
        }
        for (t, n) in left {
            for _ in 0..n.max(0) {
                self.release_all(&[t]);
            }
        }
    }
}

/// Every host reference a manager record holds.
fn mgr_tokens(r: &ManagerRecord) -> Vec<u64> {
    let mut v: Vec<u64> = r.base.automata.iter().filter_map(host_token).collect();
    match &r.part {
        ManagerPart::Traffic(t) => v.extend(t.incidents.iter().filter_map(|i| host_token(&i.network))),
        ManagerPart::Aircraft { pairs, .. } => {
            for (o, a) in pairs {
                v.extend(host_token(o));
                v.extend(host_token(a));
            }
        }
        ManagerPart::Watercraft {
            lots,
            occupants,
            ferries,
        } => {
            for (l, o) in lots {
                v.extend(host_token(l));
                v.extend(host_token(o));
            }
            v.extend(occupants.iter().filter_map(host_token));
            for f in ferries {
                v.extend(host_token(&f.lot));
                v.extend(host_token(&f.occupant));
                for e in &f.route {
                    v.extend(host_token(&e.from_lot));
                    v.extend(host_token(&e.to_lot));
                }
            }
        }
    }
    v
}

unsafe fn mgr_of<'a>(m: *const fc4_automata_manager) -> Option<&'a MgrHandle> {
    (m as *const MgrHandle).as_ref()
}

fn with_mgr<R>(
    m: *const fc4_automata_manager,
    fallback: R,
    f: impl FnOnce(&MgrHandle, &mut AutomataManager) -> R,
) -> R {
    // SAFETY: documented precondition.
    match unsafe { mgr_of(m) } {
        Some(h) => match h.m.try_borrow_mut() {
            Ok(mut x) => guard(fallback, || f(h, &mut x)),
            Err(_) => fallback,
        },
        None => fallback,
    }
}

/// A mutation after which the references the record no longer holds are
/// released.
fn mutate_mgr<R>(m: *const fc4_automata_manager, fallback: R, f: impl FnOnce(&mut AutomataManager) -> R) -> R {
    with_mgr(m, fallback, |h, x| {
        let before = mgr_tokens(x.record());
        let r = f(x);
        h.release_dropped(&before, &mgr_tokens(x.record()));
        r
    })
}

/// A new manager of the class `class_id` (vehicle `0xA98F9D3B`, pedestrian
/// `0xC98F9D17`, aircraft `0xC98F9D4A`, watercraft `0xA98F9D28`); NULL for
/// any other class. `refs` (copied; NULL = none) takes and releases host
/// references.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_create(
    class_id: u32,
    refs: *const fc4_automata_ref_host,
) -> *mut fc4_automata_manager {
    let host = table(refs);
    guard(std::ptr::null_mut(), || match ManagerKind::from_class(class_id) {
        Some(k) => Box::into_raw(Box::new(MgrHandle {
            m: RefCell::new(AutomataManager::new(k)),
            host,
        })) as *mut fc4_automata_manager,
        None => std::ptr::null_mut(),
    })
}

/// Releases every host reference the record holds, then frees the manager.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_destroy(m: *mut fc4_automata_manager) {
    if m.is_null() {
        return;
    }
    guard((), || {
        let h = Box::from_raw(m as *mut MgrHandle);
        // AT24: the aircraft deleting destructor frees storage only. Public
        // final Release must enter virtual Shutdown before reaching this API.
        let tokens =
            h.m.try_borrow()
                .map(|x| {
                    if x.kind() == ManagerKind::Aircraft {
                        Vec::new()
                    } else {
                        mgr_tokens(x.record())
                    }
                })
                .unwrap_or_default();
        h.release_all(&tokens);
    });
}

/// Write (§4, §5): the version read (raised only when the data needs it).
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_write(m: *const fc4_automata_manager, out: *const fc4_ostream) -> bool {
    let Some(o) = out_stream(out) else { return false };
    with_mgr(m, false, |_, x| x.write(&mut COut(o)))
}

/// Read (§4, §5): sets the loaded and old-version flags. On success the
/// replaced record's references are released; on failure nothing changes
/// and the references read are released.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_read(m: *mut fc4_automata_manager, input: *const fc4_istream) -> bool {
    let Some(i) = in_stream(input) else { return false };
    with_mgr(m, false, |h, x| {
        let old = mgr_tokens(x.record());
        let mut t = tracking(i);
        let ok = x.read(&mut t);
        // The core's Read is atomic: on failure the record is unchanged.
        h.release_all(if ok { &old } else { &t.tokens });
        ok
    })
}

/// Slot 3 `Init`: applies the saved max count (§4 field 2).
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_init(m: *mut fc4_automata_manager) -> bool {
    mutate_mgr(m, false, |x| x.init())
}

#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_shutdown(m: *mut fc4_automata_manager) -> bool {
    if with_mgr(m, false, |_, x| x.kind() == ManagerKind::Aircraft) {
        return fc4_automata_aircraft_lifecycle(m, 1, std::ptr::null(), None, std::ptr::null_mut());
    }
    with_mgr(m, false, |_, x| x.shutdown())
}

/// Slot 5 `Activate`.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_activate(m: *mut fc4_automata_manager, on: bool) {
    with_mgr(m, (), |_, x| x.activate(on))
}

/// Slot 6 `IsActive`.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_is_active(m: *const fc4_automata_manager) -> bool {
    with_mgr(m, false, |_, x| x.is_active())
}

/// Slot 9 `Pause` (U-Drive-It, §9).
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_pause(m: *mut fc4_automata_manager) {
    with_mgr(m, (), |_, x| x.pause())
}

/// Slot 10 `Resume`.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_resume(m: *mut fc4_automata_manager) {
    with_mgr(m, (), |_, x| x.resume())
}

/// Slot 11 `IsPaused`.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_is_paused(m: *const fc4_automata_manager) -> bool {
    with_mgr(m, false, |_, x| x.is_paused())
}

/// Slot 13 `IsDrivingMode`: the current view is U-Drive-It's (`0x2B709676`).
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_is_driving_mode(m: *const fc4_automata_manager, view: u32) -> bool {
    with_mgr(m, false, |_, x| x.is_driving_mode(view))
}

/// Slot 23 `SetMaxCount(n)` (§7.3): clamps to 0–5000, removes the automata
/// beyond it (their host references are released), sets the hard limit to
/// `trunc(n × 1.25)`. Returns the number removed.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_set_max_count(m: *mut fc4_automata_manager, n: i32) -> usize {
    mutate_mgr(m, 0, |x| x.set_max_count(n).len())
}

/// Slot 12 `GetAutomataCount`.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_automata_count(m: *const fc4_automata_manager) -> usize {
    with_mgr(m, 0, |_, x| x.automata_count())
}

/// Automaton `index`: `out` (may be NULL) gets its reference; returns a new
/// handle for a core automaton (the caller destroys it), NULL for a host
/// automaton or out of range.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_automaton_at(
    m: *const fc4_automata_manager,
    index: usize,
    out: *mut fc4_ref,
) -> *mut fc4_automata_object {
    with_mgr(m, std::ptr::null_mut(), |_, x| {
        let Some(r) = x.automata().get(index) else {
            return std::ptr::null_mut();
        };
        if let Some(o) = out.as_mut() {
            *o = ref_to_c(r);
        }
        Obj::from_ref(r).map_or(std::ptr::null_mut(), Obj::into_handle)
    })
}

/// Slot 7 `AddAutomaton`: false when NULL or listed already; a host
/// automaton gets a reference.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_add_automaton(m: *mut fc4_automata_manager, a: *const fc4_ref) -> bool {
    let r = ref_from_c(a);
    with_mgr(m, false, |h, x| {
        let t = host_token(&r);
        let added = x.add_automaton(r);
        if added {
            if let (Some(host), Some(t)) = (&h.host, t) {
                host.add_ref(t);
            }
        }
        added
    })
}

/// Slot 8 `RemoveAutomaton`: false when not listed.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_remove_automaton(
    m: *mut fc4_automata_manager,
    a: *const fc4_ref,
) -> bool {
    let r = ref_from_c(a);
    mutate_mgr(m, false, |x| x.remove_automaton(&r))
}

/// Slot 15 `CreateAutomaton` (§7.2), stubbed: the pedestrian manager makes a
/// pedestrian, the vehicle manager a vehicle, at the position, listed; NULL
/// at the hard limit and for the aircraft and watercraft managers. Returns a
/// new handle (the caller destroys it).
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_create_automaton(
    m: *mut fc4_automata_manager,
    args: *const fc4_automata_create_args,
) -> *mut fc4_automata_object {
    let Some(a) = args.as_ref() else {
        return std::ptr::null_mut();
    };
    let args = to_create_args(a);
    with_mgr(m, std::ptr::null_mut(), |_, x| match x.create_automaton(args) {
        Some(a) => Obj::Automaton(a).into_handle(),
        None => std::ptr::null_mut(),
    })
}

/// Slot 31 `TrackAutomataGroupInstances(group, delta)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_track_group(m: *mut fc4_automata_manager, group: u32, delta: i32) {
    with_mgr(m, (), |_, x| x.track_group_instances(group, delta))
}

/// Slot 32 `GetInstancesOfGroup`.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_instances_of_group(m: *const fc4_automata_manager, group: u32) -> u32 {
    with_mgr(m, 0, |_, x| x.instances_of_group(group))
}

/// `RemoveOccupant` (`0x99EF1143`, §10): the occupant's airport pairs and
/// ferry entries go (their host references are released).
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_occupant_removed(m: *mut fc4_automata_manager, occupant: *const fc4_ref) {
    if with_mgr(m, false, |_, x| x.kind() == ManagerKind::Aircraft) {
        fc4_automata_aircraft_lifecycle(m, 2, occupant, None, std::ptr::null_mut());
        return;
    }
    let r = ref_from_c(occupant);
    mutate_mgr(m, (), |x| x.occupant_removed(&r))
}

/// AT24 public lifecycle bridge. Operation 0 = PreCityShutdown base-list
/// cleanup, 1 = initialized Shutdown, 2 = accepted airport removal.
/// Events: 1 plane Shutdown, 2 plane Activate(false), 3 airport Release,
/// 4 map plane Release, 5 base plane Release. The callback borrows tokens;
/// the existing reference host performs all owning releases after the event.
/// It must not reenter or destroy the manager. No private refcount access.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_aircraft_lifecycle(
    m: *mut fc4_automata_manager,
    operation: u32,
    airport: *const fc4_ref,
    event: Option<unsafe extern "C" fn(U, u32, u64)>,
    user: U,
) -> bool {
    with_mgr(m, false, |h, x| {
        if x.kind() != ManagerKind::Aircraft || operation > 2 {
            return false;
        }
        if operation == 1 && !x.is_initialized() {
            return true;
        }
        let notify = |kind, r: &RefValue| {
            if let (Some(f), Some(token)) = (event, host_token(r)) {
                // SAFETY: caller supplies a valid non-reentrant callback.
                unsafe { f(user, kind, token) };
            }
        };
        let release = |kind, r: &RefValue| {
            notify(kind, r);
            h.release_all(&host_token(r).into_iter().collect::<Vec<_>>());
        };
        if operation == 2 {
            let key = ref_from_c(airport);
            if let ManagerPart::Aircraft { pairs, .. } = &mut x.record_mut().part {
                if let Some(index) = pairs
                    .iter()
                    .position(|(a, _)| host_token(a).is_some() && host_token(a) == host_token(&key))
                {
                    notify(2, &pairs[index].1);
                    let (a, p) = pairs.remove(index);
                    release(4, &p);
                    release(3, &a);
                }
            }
            return true;
        }
        if operation == 1 {
            if let ManagerPart::Aircraft { pairs, spots } = &mut x.record_mut().part {
                // Native compares unsigned x86 pointers, not stream order.
                pairs.sort_by_key(|(a, _)| host_token(a).unwrap_or(0) as u32);
                for (a, p) in pairs.iter() {
                    release(3, a);
                    release(4, p);
                }
                pairs.clear();
                spots.clear();
            }
        }
        // Separate retained base-list references survive segment disposal.
        // PreCityShutdown clears only this list; Shutdown does it after map.
        for p in &x.record().base.automata {
            notify(1, p);
            release(5, p);
        }
        x.record_mut().base.automata.clear();
        if operation == 1 {
            x.shutdown();
        }
        true
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_automata_manager_get_state(
    m: *const fc4_automata_manager,
    out: *mut fc4_automata_manager_state,
) -> bool {
    let Some(out) = out.as_mut() else { return false };
    with_mgr(m, false, |_, x| {
        let r = x.record();
        let version = match &r.part {
            ManagerPart::Traffic(t) => t.version,
            _ => r.version,
        };
        *out = fc4_automata_manager_state {
            struct_size: std::mem::size_of::<fc4_automata_manager_state>() as u32,
            class_id: x.kind().class_id(),
            initialized: x.is_initialized() as u8,
            active: x.is_active() as u8,
            paused: x.is_paused() as u8,
            loaded: x.is_loaded() as u8,
            old_version: x.is_old_version() as u8,
            time_scaling: r.base.time_scaling as u8,
            reserved: [0; 2],
            max_count: x.max_count(),
            hard_limit: x.hard_limit(),
            automata: x.automata_count() as u32,
            group_counts: r.base.group_counts.len() as u32,
            version,
            base_version: r.base.version,
        };
        true
    })
}

/// U-Drive-It (§9): `Pause` on each manager of the array (NULL entries are
/// skipped).
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_pause_all(managers: *const *mut fc4_automata_manager, count: usize) {
    each_manager(managers, count, true)
}

/// U-Drive-It (§9): `Resume` on each.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_resume_all(managers: *const *mut fc4_automata_manager, count: usize) {
    each_manager(managers, count, false)
}

unsafe fn each_manager(managers: *const *mut fc4_automata_manager, count: usize, pause: bool) {
    if managers.is_null() {
        return;
    }
    let handles: Vec<&MgrHandle> = (0..count).filter_map(|i| mgr_of(*managers.add(i))).collect();
    guard((), || {
        let mut borrows: Vec<_> = handles.iter().filter_map(|h| h.m.try_borrow_mut().ok()).collect();
        let mut refs: Vec<&mut AutomataManager> = borrows.iter_mut().map(|b| &mut **b).collect();
        if pause {
            pause_all(&mut refs)
        } else {
            resume_all(&mut refs)
        }
    })
}

// ---- riot leaders (§8.1) -------------------------------------------------------------------------------------

struct RiotCHost(fc4_automata_riot_host);

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.0.$f { Some(f) => unsafe { f($s.0.user $(, $arg)*) }, None => $default }
    };
}

impl RiotHost for RiotCHost {
    fn nearest_road_tile(&mut self, x: i32, z: i32) -> Option<Vec3> {
        let mut p = [0f32; 3];
        cb!(self, nearest_road_tile, false, x, z, p.as_mut_ptr()).then_some(p)
    }
    fn leader_cap_reached(&mut self, leaders: usize) -> bool {
        cb!(self, leader_cap_reached, false, leaders as u32)
    }
    fn create_automaton(&mut self, args: CreateArgs) -> u64 {
        let a = from_create_args(&args);
        cb!(self, create_automaton, 0, &a)
    }
    fn add_ref(&mut self, automaton: u64) {
        cb!(self, add_ref, (), automaton)
    }
    fn set_state(&mut self, automaton: u64, state_type: u32, a: SetStateArgs) {
        cb!(
            self,
            set_state,
            (),
            automaton,
            state_type,
            a.controller,
            a.timeout_ms,
            a.final_flag,
            a.anim_id,
            a.sfx_id,
            a.template
        )
    }
    fn set_visible(&mut self, automaton: u64, visible: bool) {
        cb!(self, set_visible, (), automaton, visible)
    }
    fn as_occupant(&mut self, automaton: u64) -> u64 {
        cb!(self, as_occupant, 0, automaton)
    }
    fn register_location(&mut self, automaton: u64, position: Vec3) {
        cb!(self, register_location, (), automaton, position.as_ptr())
    }
    fn add_controller_from_template_name(&mut self, occupant: u64, name: &str, extra: u32) -> bool {
        let s = CString::new(name).unwrap_or_default();
        cb!(
            self,
            add_controller_from_template_name,
            false,
            occupant,
            s.as_ptr(),
            extra
        )
    }
    fn remove_controllers(&mut self, occupant: u64) {
        cb!(self, remove_controllers, (), occupant)
    }
    fn unregister_location(&mut self, occupant: u64) {
        cb!(self, unregister_location, (), occupant)
    }
    fn release(&mut self, automaton: u64) {
        cb!(self, release, (), automaton)
    }
}

struct RiotHandle {
    leaders: RefCell<RiotLeaders>,
    host: RefCell<RiotCHost>,
}

fn with_riot<R>(r: *const fc4_automata_riot, fallback: R, f: impl FnOnce(&mut RiotLeaders, &mut RiotCHost) -> R) -> R {
    // SAFETY: documented precondition.
    match unsafe { (r as *const RiotHandle).as_ref() } {
        Some(h) => match (h.leaders.try_borrow_mut(), h.host.try_borrow_mut()) {
            (Ok(mut l), Ok(mut c)) => guard(fallback, || f(&mut l, &mut c)),
            _ => fallback,
        },
        None => fallback,
    }
}

/// A riot's leaders. The host table is copied; NULL for a NULL or short table.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_riot_create(host: *const fc4_automata_riot_host) -> *mut fc4_automata_riot {
    let Some(h) = table(host) else {
        return std::ptr::null_mut();
    };
    guard(std::ptr::null_mut(), || {
        Box::into_raw(Box::new(RiotHandle {
            leaders: RefCell::new(RiotLeaders::new()),
            host: RefCell::new(RiotCHost(h)),
        })) as *mut fc4_automata_riot
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_automata_riot_destroy(r: *mut fc4_automata_riot) {
    if !r.is_null() {
        guard((), || drop(Box::from_raw(r as *mut RiotHandle)));
    }
}

/// `AddRiotLeader` (§8.1 start): the road tile, the cap, `CreateAutomaton(0x4107,
/// …)`, `AddRef`, `SetState(8, …)`, `SetVisible(true)`, the location. The
/// new leader's index, or -1 when a step stops the start.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_riot_add(r: *mut fc4_automata_riot, x: i32, z: i32) -> i32 {
    with_riot(r, -1, |l, h| l.add(h, x, z).map_or(-1, |i| i as i32))
}

/// The stage update (§8.1 stage rise): a new maximum stage below 6 calls
/// `AddControllerFromTemplateName(leader as occupant, "riot_leader_<stage>",
/// 0)`. Returns whether the call was made.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_riot_set_stage(r: *mut fc4_automata_riot, index: u32, stage: i32) -> bool {
    with_riot(r, false, |l, h| l.set_stage(h, index as usize, stage))
}

/// The end (§8.1): `SetState(1, …)`, `RemoveControllers`, the location,
/// `Release`; the leader is erased (later indices move down). False for a
/// bad index.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_riot_end(r: *mut fc4_automata_riot, index: u32) -> bool {
    with_riot(r, false, |l, h| l.end(h, index as usize))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_automata_riot_count(r: *const fc4_automata_riot) -> u32 {
    with_riot(r, 0, |l, _| l.leaders().len() as u32)
}

#[no_mangle]
pub unsafe extern "C" fn fc4_automata_riot_get_leader(
    r: *const fc4_automata_riot,
    index: u32,
    out: *mut fc4_automata_riot_leader,
) -> bool {
    let Some(out) = out.as_mut() else { return false };
    with_riot(r, false, |l, _| match l.leaders().get(index as usize) {
        Some(x) => {
            *out = fc4_automata_riot_leader {
                automaton: x.automaton,
                position: x.position,
                stage: x.stage,
                max_stage: x.max_stage,
                reserved: 0,
            };
            true
        }
        None => false,
    })
}

// ---- the fire occupant (§8.2) -------------------------------------------------------------------------------

struct FireCHost(fc4_automata_fire_host);

impl FireHost for FireCHost {
    fn occupant_type(&mut self, occupant: u64) -> u32 {
        cb!(self, occupant_type, 0, occupant)
    }
    fn add_controllers_from_occupant_group_name(&mut self, name: &str, occupant: u64, extra: u32) -> bool {
        let s = CString::new(name).unwrap_or_default();
        cb!(
            self,
            add_controllers_from_occupant_group_name,
            false,
            s.as_ptr(),
            occupant,
            extra
        )
    }
    fn remove_controllers(&mut self, occupant: u64) {
        cb!(self, remove_controllers, (), occupant)
    }
}

struct FireHandle {
    hook: RefCell<FireHook>,
    host: RefCell<FireCHost>,
}

fn with_fire<R>(f: *const fc4_automata_fire, fallback: R, g: impl FnOnce(&mut FireHook, &mut FireCHost) -> R) -> R {
    // SAFETY: documented precondition.
    match unsafe { (f as *const FireHandle).as_ref() } {
        Some(h) => match (h.hook.try_borrow_mut(), h.host.try_borrow_mut()) {
            (Ok(mut k), Ok(mut c)) => guard(fallback, || g(&mut k, &mut c)),
            _ => fallback,
        },
        None => fallback,
    }
}

/// One fire occupant's hooks. The host table is copied; NULL for a NULL or
/// short table.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_fire_create(host: *const fc4_automata_fire_host) -> *mut fc4_automata_fire {
    let Some(h) = table(host) else {
        return std::ptr::null_mut();
    };
    guard(std::ptr::null_mut(), || {
        Box::into_raw(Box::new(FireHandle {
            hook: RefCell::new(FireHook::new()),
            host: RefCell::new(FireCHost(h)),
        })) as *mut fc4_automata_fire
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_automata_fire_destroy(f: *mut fc4_automata_fire) {
    if !f.is_null() {
        guard((), || drop(Box::from_raw(f as *mut FireHandle)));
    }
}

/// `SetTarget` (§8.2): a building (`0x278128A0`) gets the `"fire_occupant"`
/// controllers (slot 10). Returns whether the call was made.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_fire_set_target(f: *mut fc4_automata_fire, target: u64) -> bool {
    with_fire(f, false, |k, h| k.set_target(h, target))
}

/// `Shutdown` (§8.2): `RemoveControllers(target)` (slot 14), the target is
/// dropped. False without a target.
#[no_mangle]
pub unsafe extern "C" fn fc4_automata_fire_shutdown(f: *mut fc4_automata_fire) -> bool {
    with_fire(f, false, |k, h| k.shutdown(h))
}

#[no_mangle]
pub unsafe extern "C" fn fc4_automata_fire_target(f: *const fc4_automata_fire) -> u64 {
    with_fire(f, 0, |k, _| k.target())
}

/// Independent metadata fixture for the partial helicopter shadow (AF11).
/// Writes a fresh owner's record, returning its class ID on success, zero on
/// failure. Only the four nested version fields are constructor-independent;
/// all other default fields are deliberately NOT native comparison targets.
/// No native record, parameter or goal outputs enter this owner.
#[no_mangle]
pub unsafe extern "C" fn fc4_helicopter_shadow_fixture(output: *const fc4_ostream) -> u32 {
    let Some(out) = out_stream(output) else { return 0 };
    guard(0, || {
        let owner = fc4_core::helicopter::Helicopter::default();
        if owner.write(&mut COut(out)) {
            owner.class_id()
        } else {
            0
        }
    })
}

/// Private static-link watercraft record/getter projection. Input references
/// are tracked even on failure and released before returning to the capture.
#[no_mangle]
pub unsafe extern "C" fn fc4_watercraft_shadow_record(
    input: *const fc4_istream,
    output: *const fc4_ostream,
    refs: *const fc4_automata_ref_host,
    values: *mut f32,
    state: *mut u32,
) -> bool {
    guard(false, || {
        let (Some(input), Some(output), Some(refs)) = (in_stream(input), out_stream(output), table(refs)) else {
            return false;
        };
        if values.is_null() || state.is_null() {
            return false;
        }
        let mut stream = tracking(input);
        let record = fc4_core::watercraft::WatercraftRecord::read(&mut stream);
        let ok = if let Some(record) = record {
            let owner = fc4_core::watercraft::Watercraft::new(record.clone(), None, 0.0);
            // Watercraft::new sets constructor-only occupant fields. Preserve
            // the input record for the codec comparison instead of rebuilding it.
            for (i, value) in owner.record.base.vectors.iter().flatten().enumerate() {
                *values.add(i) = *value;
            }
            *state = owner.record.base.states.len() as u32;
            *state.add(1) = owner.top_state().unwrap_or(u32::MAX);
            record.write(&mut COut(output))
        } else {
            false
        };
        for token in stream.tokens {
            refs.release(token);
        }
        ok
    })
}

fn watercraft_probe_state(step: u32) -> u64 {
    use fc4_core::automata::AutomatonBase;
    use fc4_core::watercraft::{Watercraft, WatercraftRecord};
    let mut owner = Watercraft::new(
        WatercraftRecord {
            base: AutomatonBase::new(),
        },
        None,
        0.0,
    );
    if step >= 1 {
        owner.record.base.set_state(2, SetStateArgs::default());
    }
    if step >= 2 {
        owner.record.base.push_state(3, SetStateArgs::default());
    }
    if step >= 3 {
        owner.record.base.pop_state();
    }
    ((owner.record.base.states.len() as u64) << 32) | u64::from(owner.top_state().unwrap_or(u32::MAX))
}

/// Independent constructor/inherited stack probe; no native output seeds it.
#[no_mangle]
pub extern "C" fn fc4_watercraft_shadow_probe(step: u32) -> u64 {
    guard(u64::MAX, || watercraft_probe_state(step))
}

#[cfg(test)]
mod watercraft_shadow_tests {
    use super::watercraft_probe_state;

    #[test]
    fn private_probe_preserves_stack_order_and_restores_previous_state() {
        assert_eq!(watercraft_probe_state(0), u64::from(u32::MAX));
        assert_eq!(watercraft_probe_state(1), (1_u64 << 32) | 2);
        assert_eq!(watercraft_probe_state(2), (2_u64 << 32) | 3);
        assert_eq!(watercraft_probe_state(3), (1_u64 << 32) | 2);
    }
}
