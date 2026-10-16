// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The crime simulator's C ABI (fc4.h "crime", docs/specs/crime.md).
//!
//! The host implements [`fc4_crime_host`] (one callback per `CrimeHost`
//! method; the grids go through the shared [`fc4_grid_api`]); the plugin's
//! `cISC4CrimeSimulator` forwards its slots to the `fc4_crime_*` functions.
//!
//! Tokens are opaque host identities (fc4.h): lots, building occupants,
//! occupants and grids. 0 is the null object.
//!
//! Process-wide state (§7, §9): the crime type list and the generator are
//! shared by every crime simulator of the process, as in the original. They
//! live in a thread-local [`CrimeShared`] (every call that reaches the core
//! is made on the game's main thread); the first `fc4_crime_create` seeds the
//! generator with `derive_seed(session_seed, 0x6C09C3F6)` (foundations §6.3:
//! the original seeds it with its `cRZTime` value, seconds since 1901-01-01
//! UTC, XOR that constant, §9 / CQ5; the plugin's session seed is that
//! value, `time() + 2177452800` (`fc4_core::rng::rz_time_from_unix`, shared
//! by every clock-seeded generator, CQ21), unless `crime_seed` fixes it).
//!
//! Threads (foundations §19): the game calls getters from a second thread
//! while the main thread is inside a call. The getters marked "any thread"
//! below never touch the core (whose state is in `RefCell`s): they read a
//! snapshot behind a `Mutex`, which the main thread publishes at the end of
//! every outermost call (init, shutdown, do_message, read, write, the prop
//! requester calls). The lock is never held across a host callback.
//!
//! Re-entrancy: as in flammability, a message delivered while a call is
//! running is queued and handled when the outermost call returns, and other
//! re-entrant calls are counted (`fc4_crime_state::reentrant_calls`).

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::rc::Rc;
use std::sync::{Mutex, MutexGuard};

use fc4_core::crime::{
    decay_grid, nearest_of, stamp_rect, zoned_average, BuildingInfo, CrimeHost, CrimeMessage, CrimeShared, CrimeSim,
    CrimeTuning, CrimeType, DemandValues, LotBits, LotRef, LtextLoad, NearbyOccupant, NearestKind, CRIME_TYPE_GROUP,
};
use fc4_core::grid::{GridProvider, SimGrid, TractGeometry, TractGrid};
use fc4_core::property::PropertyHolder;
use fc4_core::stream::{RefValue, ResKey, SaveObject};

use crate::guard;
use crate::hostgrid::{create_grid, fc4_cell_rect, fc4_grid_api, grid_token, resolve_grid, take_props, HostGrid};
use crate::landvalue::fc4_grid_geometry;
use crate::ordinance::fc4_key_visitor;
use crate::props::fc4_props;
use crate::stream::{fc4_bytes_sink, fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

/// The exemplar type of crime types and the tuning (§7, §8).
const EXEMPLAR_TYPE: u32 = 0x6534_284A;

/// A message for `fc4_crime_do_message` (fc4.h `fc4_crime_message`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_crime_message {
    pub msg_type: u32,
    /// `GetData1()` (exemplar changed: the group).
    pub data1: u32,
    /// `GetData2()` (exemplar changed: the instance).
    pub data2: u32,
    pub reserved: u32,
    /// `GetVoid1()` as a `cISC4Occupant` (occupant inserted / removed).
    pub occupant: u64,
}

/// One element of `GetCrimeIncidentArray` (fc4.h `fc4_crime_event`; the
/// game's `tCrimeEvent`, 28 bytes, §11.4, with `has_lot` in a padding byte).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_crime_event {
    pub cell_x: u32,
    pub cell_z: u32,
    pub type_id: u32,
    pub visuals_created: u8,
    /// 0: an incident without a lot; the game's element keeps its previous
    /// content (§11.4).
    pub has_lot: u8,
    pub pad: [u8; 2],
    pub icon_type: u32,
    pub icon_group: u32,
    pub icon_instance: u32,
}

/// One occupant of the bounding-box query (fc4.h `fc4_crime_nearby`, the
/// core's `NearbyOccupant`, §11.3).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_crime_nearby {
    pub token: u64,
    pub occupant_type: u32,
    pub in_crime_group: u8,
    pub has_incident_property: u8,
    pub answers_iid: u8,
    pub pad: u8,
    pub position: [f32; 3],
    pub prop_state: u32,
}

/// Diagnostics (fc4.h `fc4_crime_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_crime_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub loaded_from_save: u8,
    pub has_message_server: u8,
    pub pad0: u8,
    pub pipeline_state: u32,
    pub monthly_crime_count: u32,
    /// The stored `float32` (the default NaN `0xFFC00000` when unzoned).
    pub average_bits: u32,
    pub hotspot_x: i32,
    pub hotspot_z: i32,
    pub hotspot_value: i32,
    pub ordinance_factor: f32,
    pub criminal_base: i32,
    pub crimes_to_commit: i32,
    pub incidents: u32,
    /// The process-wide type list's length (0 = not loaded).
    pub types: u32,
    pub tract_size: u32,
    pub decay: f32,
    pub reoffend: f32,
    /// `pop[wealth][bucket]` (§6.1), wealth-major.
    pub census: [u32; 12],
    pub jobless: [f32; 3],
    pub incident_epoch: u32,
    pub reentrant_calls: u32,
    pub reserved: u32,
    pub crime_grid: u64,
    pub effect_grid: u64,
}

pub type fc4_crime_nearby_callback = Option<unsafe extern "C" fn(ctx: *mut c_void, o: *const fc4_crime_nearby)>;

/// `fc4_crime_host::occupants_in_box` kinds.
pub const FC4_CRIME_NEAREST_SIM: u32 = 0;
pub const FC4_CRIME_NEAREST_PROP: u32 = 1;

/// Everything the crime simulator asks of the game (fc4.h `fc4_crime_host`;
/// one callback per `CrimeHost` method other than the grids, in the trait's
/// order).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_crime_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    pub grids: fc4_grid_api,
    // lifecycle glue (§4)
    pub add_agent: Option<unsafe extern "C" fn(U, *const u8, usize, *const u32, u32, u32)>,
    pub remove_agent: Option<unsafe extern "C" fn(U)>,
    pub subscribe: Option<unsafe extern "C" fn(U, *const u32, u32)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, *const u32, u32)>,
    pub acquire_services: Option<unsafe extern "C" fn(U)>,
    pub release_services: Option<unsafe extern "C" fn(U)>,
    pub add_prop_requester: Option<unsafe extern "C" fn(U)>,
    // resources (§7, §8)
    /// A NEW fc4_props (the core destroys it), NULL if the exemplar is missing.
    pub tuning: Option<unsafe extern "C" fn(U) -> *mut fc4_props>,
    /// Every crime type exemplar key (type 0x6534284A, group 0xEA12F32C).
    pub for_each_type_key: Option<unsafe extern "C" fn(U, fc4_key_visitor, *mut c_void)>,
    /// A NEW fc4_props with the exemplar's properties, NULL if missing.
    pub exemplar: Option<unsafe extern "C" fn(U, u32, u32, u32) -> *mut fc4_props>,
    /// §7 steps 2-3 of a type name (CQ8; fc4.h says how the result reads).
    pub ltext: Option<unsafe extern "C" fn(U, u32, u32, u32, fc4_bytes_sink, *mut c_void) -> bool>,
    // residential and demand (§6.1, §6.2)
    pub population_grids: Option<unsafe extern "C" fn(U, *mut u64)>,
    pub eq_grids: Option<unsafe extern "C" fn(U, *mut u64)>,
    pub demand: Option<unsafe extern "C" fn(U, u32, *mut f32, *mut f32, *mut f32) -> bool>,
    // police and ordinances (§6.4, §6.6)
    pub police_grid: Option<unsafe extern "C" fn(U) -> u64>,
    pub jail_population: Option<unsafe extern "C" fn(U) -> u32>,
    pub reset_unjailed_criminals: Option<unsafe extern "C" fn(U) -> i32>,
    pub process_monthly_crime_stats: Option<unsafe extern "C" fn(U)>,
    pub ordinance_crime_factor: Option<unsafe extern "C" fn(U, *mut f32) -> bool>,
    // lots and zones (§6.6-§6.9, §11)
    pub random_lot: Option<unsafe extern "C" fn(U) -> u64>,
    pub lot_at: Option<unsafe extern "C" fn(U, i32, i32) -> u64>,
    pub lot_bounding_rect: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    pub lot_location: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32) -> bool>,
    pub lot_condition: Option<unsafe extern "C" fn(U, u64) -> u8>,
    pub lot_building: Option<unsafe extern "C" fn(U, u64, *mut u64, *mut u32, *mut u32) -> bool>,
    pub lot_center_cell: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32)>,
    pub zone_type: Option<unsafe extern "C" fn(U, i32, i32) -> u32>,
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    // occupants and props (§6.10, §11.3, §11.6)
    pub occupant_has_property: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub occupant_property_bytes: Option<unsafe extern "C" fn(U, u64, u32, fc4_bytes_sink, *mut c_void) -> bool>,
    pub occupant_position: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    pub occupants_in_box:
        Option<unsafe extern "C" fn(U, *const f32, *const f32, u32, fc4_crime_nearby_callback, *mut c_void)>,
    pub prop_crime_incident_id: Option<unsafe extern "C" fn(U, u32) -> u32>,
    // arson (§6.8)
    pub disaster_layer_present: Option<unsafe extern "C" fn(U) -> bool>,
    pub ignite: Option<unsafe extern "C" fn(U, u64, u32)>,
    // messages (§10)
    pub post_crime_committed: Option<unsafe extern "C" fn(U, u32, u64, *const u8, usize)>,
    pub post_crime_map_updated: Option<unsafe extern "C" fn(U)>,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_crime_message>() == 24);
    assert!(std::mem::size_of::<fc4_crime_event>() == 28);
    assert!(std::mem::size_of::<fc4_crime_nearby>() == 32);
    assert!(std::mem::size_of::<fc4_crime_state>() == 152);
    assert!(std::mem::size_of::<fc4_crime_host>() == if cfg!(target_pointer_width = "32") { 200 } else { 400 });
};

// ---- host adapter -----------------------------------------------------------------------------------

struct CHost(fc4_crime_host);

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.0.$f { Some(f) => unsafe { f($s.0.user $(, $arg)*) }, None => $default }
    };
}

/// A lot reference: the class ID is informational (the plugin's stream
/// writer saves the object behind the token).
fn lot_of(t: u64) -> Option<LotRef> {
    (t != 0).then_some(LotRef { token: t, class_id: 0 })
}

unsafe extern "C" fn collect_bytes(ctx: *mut c_void, data: *const u8, len: usize) {
    if let Some(v) = (ctx as *mut Vec<u8>).as_mut() {
        if !data.is_null() && len > 0 {
            v.extend_from_slice(std::slice::from_raw_parts(data, len));
        }
    }
}

/// Bytes from a sink, and whether the sink was called at all (an empty text
/// is still text).
#[derive(Default)]
struct SinkText {
    bytes: Vec<u8>,
    delivered: bool,
}

unsafe extern "C" fn collect_sink_text(ctx: *mut c_void, data: *const u8, len: usize) {
    if let Some(t) = (ctx as *mut SinkText).as_mut() {
        t.delivered = true;
        if !data.is_null() && len > 0 {
            t.bytes.extend_from_slice(std::slice::from_raw_parts(data, len));
        }
    }
}

unsafe extern "C" fn collect_key(ctx: *mut c_void, type_id: u32, group: u32, instance: u32) {
    if let Some(v) = (ctx as *mut Vec<ResKey>).as_mut() {
        v.push(ResKey {
            type_id,
            group,
            instance,
        });
    }
}

unsafe extern "C" fn collect_nearby(ctx: *mut c_void, o: *const fc4_crime_nearby) {
    if let (Some(v), Some(o)) = ((ctx as *mut Vec<NearbyOccupant>).as_mut(), o.as_ref()) {
        v.push(nearby_of(o));
    }
}

fn nearby_of(o: &fc4_crime_nearby) -> NearbyOccupant {
    NearbyOccupant {
        token: o.token,
        occupant_type: o.occupant_type,
        in_crime_group: o.in_crime_group != 0,
        has_incident_property: o.has_incident_property != 0,
        position: o.position,
        answers_iid: o.answers_iid != 0,
        prop_state: o.prop_state,
    }
}

impl CHost {
    /// Three grid tokens (one reference each) as core grids.
    fn grids3<T: fc4_core::grid::GridElem>(
        &self,
        f: Option<unsafe extern "C" fn(U, *mut u64)>,
    ) -> [Option<Rc<dyn TractGrid<T>>>; 3] {
        let mut t = [0u64; 3];
        if let Some(f) = f {
            // SAFETY: valid callback; `t` holds three tokens.
            unsafe { f(self.0.user, t.as_mut_ptr()) };
        }
        t.map(|t| (t != 0).then(|| HostGrid::<T>::wrap(self.0.grids, t, false) as Rc<dyn TractGrid<T>>))
    }

    /// The crime type exemplars (§7). [choice] (§7) The list is in ascending
    /// instance order, one entry per instance (a duplicated key loads the
    /// same resource, which the original would update in place). CQ10: the
    /// original's order is the resource manager's key-list order, ascending
    /// instance for the shipped types; plug-in types can differ.
    fn type_exemplars(&self) -> Vec<(u32, Rc<PropertyHolder>)> {
        let mut keys: Vec<ResKey> = Vec::new();
        cb!(
            self,
            for_each_type_key,
            (),
            Some(collect_key),
            &mut keys as *mut Vec<ResKey> as *mut c_void
        );
        keys.retain(|k| k.type_id == EXEMPLAR_TYPE && k.group == CRIME_TYPE_GROUP);
        keys.sort_by_key(|k| k.instance);
        keys.dedup_by_key(|k| k.instance);
        keys.iter()
            .filter_map(|k| {
                let p = cb!(self, exemplar, std::ptr::null_mut(), k.type_id, k.group, k.instance);
                // SAFETY: the callback returns NULL or a new fc4_props it hands over.
                unsafe { take_props(p) }.map(|h| (k.instance, h))
            })
            .collect()
    }

    /// §7 steps 2 and 3 (CQ8). The callback returns true when the name
    /// loaded (the sink got the detokenized text); false with text delivered
    /// = the detokenizer returned false; false with nothing delivered = no
    /// such LTEXT. A failed load discards whatever the sink got: the name is
    /// empty unless `0xAA12FCB0` exists (CQ16).
    fn ltext_of(&self, key: ResKey) -> LtextLoad {
        let mut text = SinkText::default();
        let loaded = cb!(
            self,
            ltext,
            false,
            key.type_id,
            key.group,
            key.instance,
            Some(collect_sink_text),
            &mut text as *mut SinkText as *mut c_void
        );
        match (loaded, text.delivered) {
            (true, _) => LtextLoad::Loaded(text.bytes),
            (false, true) => LtextLoad::DetokenizeFailed,
            (false, false) => LtextLoad::Missing,
        }
    }

    fn tuning_props(&self) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, tuning, std::ptr::null_mut());
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }
}

impl GridProvider<u8> for CHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        // Shut down explicitly by the simulator (§4.2), released on drop.
        create_grid::<u8>(&self.0.grids, instance, tract_size, false)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u8>>> {
        resolve_grid::<u8>(&self.0.grids, r, false)
    }
}

impl CrimeHost for CHost {
    fn add_agent(&self, name: &str, levels: &[u32], flags: u32) {
        cb!(
            self,
            add_agent,
            (),
            name.as_ptr(),
            name.len(),
            levels.as_ptr(),
            levels.len() as u32,
            flags
        )
    }
    fn remove_agent(&self) {
        cb!(self, remove_agent, ())
    }
    fn subscribe(&self, msg_types: &[u32]) {
        cb!(self, subscribe, (), msg_types.as_ptr(), msg_types.len() as u32)
    }
    fn unsubscribe(&self, msg_types: &[u32]) {
        cb!(self, unsubscribe, (), msg_types.as_ptr(), msg_types.len() as u32)
    }
    fn acquire_services(&self) {
        cb!(self, acquire_services, ())
    }
    fn release_services(&self) {
        cb!(self, release_services, ())
    }
    fn add_prop_requester(&self) {
        cb!(self, add_prop_requester, ())
    }
    fn shutdown_grid(&self, grid: &Rc<dyn TractGrid<u8>>) {
        if let Some(f) = self.0.grids.shutdown {
            if let RefValue::Host { token, .. } = grid.save_ref() {
                if token != 0 {
                    // SAFETY: valid callback; the token is the game's grid.
                    unsafe { f(self.0.grids.user, token) };
                }
            }
        }
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning_props()
    }
    fn crime_type_exemplars(&self) -> Vec<(u32, Rc<PropertyHolder>)> {
        self.type_exemplars()
    }
    fn ltext(&self, key: ResKey) -> LtextLoad {
        self.ltext_of(key)
    }
    fn population_grids(&self) -> [Option<Rc<dyn TractGrid<u16>>>; 3] {
        self.grids3::<u16>(self.0.population_grids)
    }
    fn eq_grids(&self) -> [Option<Rc<dyn TractGrid<f32>>>; 3] {
        self.grids3::<f32>(self.0.eq_grids)
    }
    fn demand(&self, id: u32) -> Option<DemandValues> {
        let mut v = DemandValues::default();
        cb!(self, demand, false, id, &mut v.value, &mut v.max, &mut v.min).then_some(v)
    }
    fn police_grid(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        let t = cb!(self, police_grid, 0);
        // One reference handed over (fc4.h), released on drop.
        (t != 0).then(|| HostGrid::<u8>::wrap(self.0.grids, t, false) as Rc<dyn TractGrid<u8>>)
    }
    fn jail_population(&self) -> u32 {
        cb!(self, jail_population, 0)
    }
    fn reset_unjailed_criminals(&self) -> i32 {
        cb!(self, reset_unjailed_criminals, 0)
    }
    fn process_monthly_crime_stats(&self) {
        cb!(self, process_monthly_crime_stats, ())
    }
    fn ordinance_crime_factor(&self) -> Option<f32> {
        let mut f = 0.0f32;
        cb!(self, ordinance_crime_factor, false, &mut f).then_some(f)
    }
    fn random_lot(&self) -> Option<LotRef> {
        lot_of(cb!(self, random_lot, 0))
    }
    fn lot_at(&self, cell_x: i32, cell_z: i32) -> Option<LotRef> {
        lot_of(cb!(self, lot_at, 0, cell_x, cell_z))
    }
    fn lot_bounding_rect(&self, lot: u64) -> [i32; 4] {
        let mut r = fc4_cell_rect::default();
        cb!(self, lot_bounding_rect, (), lot, &mut r);
        [r.x1, r.z1, r.x2, r.z2]
    }
    fn lot_location(&self, lot: u64) -> Option<(i32, i32)> {
        let (mut x, mut z) = (0, 0);
        cb!(self, lot_location, false, lot, &mut x, &mut z).then_some((x, z))
    }
    fn lot_condition(&self, lot: u64) -> u8 {
        cb!(self, lot_condition, 0, lot)
    }
    fn lot_building(&self, lot: u64) -> Option<BuildingInfo> {
        let (mut token, mut purpose, mut wealth) = (0u64, 0u32, 0u32);
        let found = cb!(self, lot_building, false, lot, &mut token, &mut purpose, &mut wealth);
        found.then_some(BuildingInfo {
            token,
            purpose: purpose as i32,
            wealth: wealth as i32,
        })
    }
    fn lot_center_cell(&self, lot: u64) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        cb!(self, lot_center_cell, (), lot, &mut x, &mut z);
        (x, z)
    }
    fn zone_type(&self, cell_x: i32, cell_z: i32) -> u32 {
        cb!(self, zone_type, 0, cell_x, cell_z)
    }
    fn add_ref(&self, token: u64) {
        cb!(self, add_ref, (), token)
    }
    fn release(&self, token: u64) {
        cb!(self, release, (), token)
    }
    fn occupant_has_property(&self, occupant: u64, id: u32) -> bool {
        cb!(self, occupant_has_property, false, occupant, id)
    }
    fn occupant_property_bytes(&self, occupant: u64, id: u32) -> Option<Vec<u8>> {
        let mut v: Vec<u8> = Vec::new();
        let found = cb!(
            self,
            occupant_property_bytes,
            false,
            occupant,
            id,
            Some(collect_bytes),
            &mut v as *mut Vec<u8> as *mut c_void
        );
        found.then_some(v)
    }
    fn occupant_position(&self, occupant: u64) -> [f32; 3] {
        let mut p = [0.0f32; 3];
        cb!(self, occupant_position, (), occupant, p.as_mut_ptr());
        p
    }
    fn occupants_in_box(&self, min: [f32; 3], max: [f32; 3], kind: NearestKind) -> Vec<NearbyOccupant> {
        let k = match kind {
            NearestKind::Sim => FC4_CRIME_NEAREST_SIM,
            NearestKind::Prop => FC4_CRIME_NEAREST_PROP,
        };
        let mut v: Vec<NearbyOccupant> = Vec::new();
        cb!(
            self,
            occupants_in_box,
            (),
            min.as_ptr(),
            max.as_ptr(),
            k,
            Some(collect_nearby),
            &mut v as *mut Vec<NearbyOccupant> as *mut c_void
        );
        v
    }
    fn prop_crime_incident_id(&self, prop: u32) -> u32 {
        cb!(self, prop_crime_incident_id, 0, prop)
    }
    fn disaster_layer_present(&self) -> bool {
        cb!(self, disaster_layer_present, false)
    }
    fn ignite(&self, building: u64, cause: u32) {
        cb!(self, ignite, (), building, cause)
    }
    fn post_crime_committed(&self, type_id: u32, lot: LotRef, name: &[u8]) {
        cb!(
            self,
            post_crime_committed,
            (),
            type_id,
            lot.token,
            name.as_ptr(),
            name.len()
        )
    }
    fn post_crime_map_updated(&self) {
        cb!(self, post_crime_map_updated, ())
    }
}

// ---- process-wide state -------------------------------------------------------------------------------

thread_local! {
    /// The crime type list and generator of this process (§7, §9).
    static SHARED: RefCell<Option<Rc<CrimeShared>>> = const { RefCell::new(None) };
}

/// The process-wide state, created (and the generator seeded) on first use.
fn shared(session_seed: u32) -> Rc<CrimeShared> {
    SHARED.with(|s| {
        s.borrow_mut()
            .get_or_insert_with(|| CrimeShared::from_session(session_seed))
            .clone()
    })
}

// ---- the published snapshot (any thread) --------------------------------------------------------------

/// What the any-thread getters read (foundations §19).
#[derive(Default)]
struct Published {
    crime_grid: u64,
    average: u32,
    monthly: u32,
    hotspot: Option<(u8, i32, i32)>,
    /// The incidents as `GetCrimeIncidentArray` reports them, with the
    /// visuals flags (the authority for Set/GetVisualsCreated).
    events: Vec<fc4_crime_event>,
    /// The core's `incident_epoch` the events were built from.
    epoch: Option<u32>,
}

// ---- handles ------------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_crime`).
#[repr(C)]
pub struct fc4_crime {
    _private: [u8; 0],
}

struct Handle {
    sim: Rc<CrimeSim>,
    host: Rc<CHost>,
    depth: Cell<u32>,
    queue: RefCell<Vec<CrimeMessage>>,
    reentrant: Cell<u32>,
    published: Mutex<Published>,
}

impl Handle {
    fn lock(&self) -> MutexGuard<'_, Published> {
        self.published.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Runs `f` with the call depth counted; at the end of the outermost call,
    /// the queued messages are handled and the snapshot is published.
    fn call<R>(&self, f: impl FnOnce(&CrimeSim) -> R) -> R {
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
                self.sim.handle_message(m);
            }
            self.publish();
        }
        self.depth.set(self.depth.get() - 1);
        r
    }

    /// The incident list as events (host calls for the lot centres; no lock
    /// held).
    fn events(&self) -> Vec<fc4_crime_event> {
        let incidents: Vec<(Option<LotRef>, u32, bool, ResKey)> = self
            .sim
            .data()
            .incidents
            .iter()
            .map(|i| (i.lot, i.type_id, i.visuals_created, i.icon))
            .collect();
        incidents
            .iter()
            .map(|&(lot, type_id, visuals, icon)| {
                let (cx, cz) = lot.map_or((0, 0), |l| self.host.lot_center_cell(l.token));
                fc4_crime_event {
                    cell_x: cx as u32,
                    cell_z: cz as u32,
                    type_id,
                    visuals_created: u8::from(visuals),
                    has_lot: u8::from(lot.is_some()),
                    pad: [0; 2],
                    icon_type: icon.type_id,
                    icon_group: icon.group,
                    icon_instance: icon.instance,
                }
            })
            .collect()
    }

    /// Main thread, at the end of the outermost call.
    fn publish(&self) {
        let (grid, epoch) = {
            let d = self.sim.data();
            (grid_token(&d.crime_grid), d.incident_epoch)
        };
        let average = self.sim.average_crime();
        let monthly = self.sim.monthly_crime_count();
        let hotspot = self.sim.crime_hotspot();
        let stale = self.lock().epoch != Some(epoch);
        let events = stale.then(|| self.events());
        let mut p = self.lock();
        p.crime_grid = grid;
        p.average = average;
        p.monthly = monthly;
        p.hotspot = hotspot;
        if let Some(e) = events {
            p.events = e;
            p.epoch = Some(epoch);
        }
    }
}

/// The handle, or `None` for NULL (precondition: NULL or a live handle).
unsafe fn handle<'a>(p: *const fc4_crime) -> Option<&'a Handle> {
    (p as *const Handle).as_ref()
}

/// Runs `f` on the handle; the fallback for NULL or a panic. Main thread.
fn with_crime<R>(p: *const fc4_crime, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_crime*.
    let Some(h) = (unsafe { handle(p) }) else {
        return fallback;
    };
    guard(fallback, || f(h))
}

fn sim<R>(p: *const fc4_crime, fallback: R, f: impl FnOnce(&CrimeSim) -> R) -> R {
    with_crime(p, fallback, |h| h.call(f))
}

/// Any thread: reads the snapshot only.
fn snapshot<R>(p: *const fc4_crime, fallback: R, f: impl FnOnce(&mut Published) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_crime*. Only the
    // Mutex field is touched.
    let Some(h) = (unsafe { handle(p) }) else {
        return fallback;
    };
    guard(fallback, || f(&mut h.lock()))
}

/// Creates the core object (constructor values, §3). The host table is
/// copied. `token` is the host identity of the simulator itself.
/// `session_seed` seeds the process-wide generator the first time (§9,
/// foundations §6.3); later calls share it.
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_create(
    host: *const fc4_crime_host,
    token: u64,
    session_seed: u32,
) -> *mut fc4_crime {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_crime_host>() && h.grids.complete() => {
            let host = Rc::new(CHost(*h));
            let sim = CrimeSim::new(host.clone(), token, shared(session_seed));
            Box::into_raw(Box::new(Handle {
                sim,
                host,
                depth: Cell::new(0),
                queue: RefCell::new(Vec::new()),
                reentrant: Cell::new(0),
                published: Mutex::new(Published::default()),
            })) as *mut fc4_crime
        }
        _ => std::ptr::null_mut(),
    })
}

/// Destroys the core object. Grids still held are released (not shut down:
/// `Shutdown` does that, §4.2); lots of incidents still held are not released
/// (the plugin calls Shutdown first, which clears them).
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_destroy(p: *mut fc4_crime) {
    if p.is_null() {
        return;
    }
    guard((), || drop(Box::from_raw(p as *mut Handle)));
}

/// `Init` (§4.1). Always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_init(p: *mut fc4_crime) -> bool {
    sim(p, false, CrimeSim::init)
}

/// `Shutdown` (§4.2). Always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_shutdown(p: *mut fc4_crime) -> bool {
    sim(p, false, CrimeSim::shutdown)
}

/// `DoMessage` (§5): always true for a live object. A message delivered
/// during another call is queued.
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_do_message(p: *mut fc4_crime, m: *const fc4_crime_message) -> bool {
    let Some(m) = m.as_ref() else { return true };
    let msg = CrimeMessage {
        msg_type: m.msg_type,
        data1: m.data1,
        data2: m.data2,
        void1: m.occupant,
    };
    with_crime(p, true, |h| {
        if h.depth.get() > 0 {
            h.queue.borrow_mut().push(msg);
            return true;
        }
        h.call(|s| s.handle_message(msg))
    })
}

// ---- any-thread getters (the snapshot) ----------------------------------------------------------------

/// Slot 5 `GetCrimeGrid()`: the grid's token (borrowed; 0 = none). Any thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_grid(p: *const fc4_crime) -> u64 {
    snapshot(p, 0, |s| s.crime_grid)
}

/// Slot 6 `GetAverageCrime()` (`trunc`, NaN → 0, §6.9). Any thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_average(p: *const fc4_crime) -> u32 {
    snapshot(p, 0, |s| s.average)
}

/// Slot 7 `GetMonthlyCrimeCount()`. Any thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_monthly_count(p: *const fc4_crime) -> u32 {
    snapshot(p, 0, |s| s.monthly)
}

/// Slot 8 `GetCrimeHotspot` (§11.2): false and nothing written when the
/// value is 0 or less; otherwise each non-null pointer gets the value's low
/// byte and the corner cell `S·x`, `S·z`. Any thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_hotspot(p: *const fc4_crime, value: *mut u8, x: *mut i32, z: *mut i32) -> bool {
    let Some((v, hx, hz)) = snapshot(p, None, |s| s.hotspot) else {
        return false;
    };
    if let Some(o) = value.as_mut() {
        *o = v;
    }
    if let Some(o) = x.as_mut() {
        *o = hx;
    }
    if let Some(o) = z.as_mut() {
        *o = hz;
    }
    true
}

/// The incident count (§11.4). Any thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_incident_count(p: *const fc4_crime) -> u32 {
    snapshot(p, 0, |s| s.events.len() as u32)
}

/// Slot 11 `GetCrimeIncidentArray` (§11.4): copies up to `capacity` events
/// into `out` and returns the incident count. Any thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_incidents(p: *const fc4_crime, out: *mut fc4_crime_event, capacity: u32) -> u32 {
    snapshot(p, 0, |s| {
        if !out.is_null() {
            let n = s.events.len().min(capacity as usize);
            // SAFETY: the caller provides room for `capacity` elements.
            std::ptr::copy_nonoverlapping(s.events.as_ptr(), out, n);
        }
        s.events.len() as u32
    })
}

/// Slot 12 `SetVisualsCreated(i, b)` (§11.5): false for an index out of
/// range. Any thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_set_visuals(p: *const fc4_crime, index: u32, value: bool) -> bool {
    snapshot(p, false, |s| match s.events.get_mut(index as usize) {
        Some(e) => {
            e.visuals_created = u8::from(value);
            true
        }
        None => false,
    })
}

/// Slot 13 `GetVisualsCreated(i)` (§11.5). Any thread.
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_get_visuals(p: *const fc4_crime, index: u32) -> bool {
    snapshot(p, false, |s| {
        s.events.get(index as usize).is_some_and(|e| e.visuals_created != 0)
    })
}

/// The selection of §11.3 over the occupants the host's box query returned
/// (in its order): the index of the candidate, or -1. Pure; any thread.
/// `kind`: FC4_CRIME_NEAREST_SIM or FC4_CRIME_NEAREST_PROP.
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_nearest_of(
    kind: u32,
    pos: *const f32,
    radius: f32,
    occupants: *const fc4_crime_nearby,
    count: u32,
) -> i32 {
    guard(-1, || {
        if pos.is_null() || (occupants.is_null() && count != 0) {
            return -1;
        }
        let kind = if kind == FC4_CRIME_NEAREST_PROP {
            NearestKind::Prop
        } else {
            NearestKind::Sim
        };
        let pos = [*pos, *pos.add(1), *pos.add(2)];
        let found: Vec<NearbyOccupant> = if count == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(occupants, count as usize)
                .iter()
                .map(nearby_of)
                .collect()
        };
        nearest_of(kind, pos, radius, &found).map_or(-1, |i| i as i32)
    })
}

// ---- prop requester (§11.6; main thread) --------------------------------------------------------------

/// Prop requester slot 3 `GetRequestedShare(rect)` (rect `x1, z1, x2, z2`).
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_requested_share(p: *const fc4_crime, x1: i32, z1: i32, x2: i32, z2: i32) -> i32 {
    sim(p, 0, |s| s.requested_share(x1, z1, x2, z2))
}

/// Prop requester slot 6 `SelectProp(props, cellX, cellZ)`: 0 = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_select_prop(
    p: *const fc4_crime,
    props: *const u32,
    count: u32,
    cell_x: i32,
    cell_z: i32,
) -> u32 {
    let list: Vec<u32> = if props.is_null() || count == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(props, count as usize).to_vec()
    };
    sim(p, 0, |s| s.select_prop(&list, cell_x, cell_z))
}

/// Diagnostics (main thread).
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_get_state(p: *const fc4_crime, out: *mut fc4_crime_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_crime(p, false, |h| {
        let d = h.sim.data();
        let mut census = [0u32; 12];
        for (w, row) in d.census.iter().enumerate() {
            census[w * 4..w * 4 + 4].copy_from_slice(row);
        }
        *o = fc4_crime_state {
            struct_size: std::mem::size_of::<fc4_crime_state>() as u32,
            initialized: u8::from(d.initialized),
            loaded_from_save: u8::from(d.loaded_from_save),
            has_message_server: u8::from(d.has_message_server),
            pad0: 0,
            pipeline_state: d.state,
            monthly_crime_count: d.monthly_crime_count,
            average_bits: d.average_crime.to_bits(),
            hotspot_x: d.hotspot_x,
            hotspot_z: d.hotspot_z,
            hotspot_value: d.hotspot_value,
            ordinance_factor: d.ordinance_factor,
            criminal_base: d.criminal_base,
            crimes_to_commit: d.crimes_to_commit,
            incidents: d.incidents.len() as u32,
            types: h.sim.shared().types().map_or(0, |t| t.len() as u32),
            tract_size: d.tuning.tract_size,
            decay: d.tuning.decay,
            reoffend: d.tuning.reoffend,
            census,
            jobless: d.jobless,
            incident_epoch: d.incident_epoch,
            reentrant_calls: h.reentrant.get(),
            reserved: 0,
            crime_grid: grid_token(&d.crime_grid),
            effect_grid: grid_token(&d.effect_grid),
        };
        true
    })
}

/// `Write` (§12). Both grid references go out as `fc4_ref`s with the token
/// and the grid CLSID; each incident's lot with its token.
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_write(p: *const fc4_crime, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    sim(p, false, |s| s.write(&mut COut(o)))
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

/// `Read` (§12): false for a version other than 6, or (FreeCity4 [choice])
/// a stream error, which changes nothing.
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_read(p: *mut fc4_crime, inp: *const fc4_istream) -> bool {
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

// ---- the crime shadow (tooling: plugin/FC4CrimeShadow.h) ----------------------------------------------

/// The crime shadow's view of the data (fc4.h `fc4_crime_shadow`): the
/// tuning and the type list, read through a host table that only serves
/// `tuning`, `for_each_type_key`, `exemplar` and `ltext`.
#[repr(C)]
pub struct fc4_crime_shadow {
    _private: [u8; 0],
}

struct Shadow {
    tuning: CrimeTuning,
    tuning_found: bool,
    types: Vec<CrimeType>,
}

/// fc4.h `fc4_crime_shadow_info`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_crime_shadow_info {
    pub struct_size: u32,
    pub types: u32,
    pub tuning_found: u8,
    pub pad: [u8; 3],
    pub tract_size: u32,
    pub decay: f32,
    pub reoffend: f32,
}

/// One incident to replay (fc4.h `fc4_crime_shadow_incident`): its lot's
/// bounding rect (`has_rect` = 0: no lot found, not stamped) and type.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_crime_shadow_incident {
    pub rect: fc4_cell_rect,
    pub type_id: u32,
    pub has_rect: u8,
    pub pad: [u8; 3],
}

/// One month's replay (fc4.h `fc4_crime_shadow_month_in`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_crime_shadow_month_in {
    pub struct_size: u32,
    pub geometry: fc4_grid_geometry,
    /// `count_x × count_z` values, x outer (the grid before the step).
    pub pre: *const u8,
    /// The hotspot before the step (tract x, tract z, value).
    pub hotspot_x: i32,
    pub hotspot_z: i32,
    pub hotspot_value: i32,
    pub incidents: *const fc4_crime_shadow_incident,
    pub incident_count: u32,
    /// `count_x × count_z` zone types of each tract's corner cell (x outer),
    /// or NULL: no average.
    pub zones: *const u32,
}

/// fc4.h `fc4_crime_shadow_month_out`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_crime_shadow_month_out {
    pub struct_size: u32,
    pub hotspot_x: i32,
    pub hotspot_z: i32,
    pub hotspot_value: i32,
    /// Incidents stamped (severity > 0): the monthly crime count.
    pub count: u32,
    /// Incidents of a type not in the list (not stamped, not counted).
    pub unknown_types: u32,
    /// Incidents with no rect (not stamped, counted if severity > 0).
    pub no_rect: u32,
    pub severity_sum: u32,
    /// The stored average (`float32` bits; NaN `0xFFC00000` when unzoned),
    /// 0 when `zones` was NULL.
    pub average_bits: u32,
    pub average_computed: u8,
    pub pad: [u8; 3],
}

const _: () = {
    assert!(std::mem::size_of::<fc4_crime_shadow_info>() == 24);
    assert!(std::mem::size_of::<fc4_crime_shadow_incident>() == 24);
    assert!(
        std::mem::size_of::<fc4_crime_shadow_month_in>() == if cfg!(target_pointer_width = "32") { 64 } else { 88 }
    );
    assert!(std::mem::size_of::<fc4_crime_shadow_month_out>() == 40);
};

/// Reads the tuning and the type list through `host` (only `tuning`,
/// `for_each_type_key`, `exemplar` and `ltext` are called; the rest may be
/// NULL, `grids` included). NULL if `host` is NULL or too small.
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_shadow_create(host: *const fc4_crime_host) -> *mut fc4_crime_shadow {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_crime_host>() => {
            let host = CHost(*h);
            let exemplar = host.tuning_props();
            let mut tuning = CrimeTuning::default();
            tuning.read(exemplar.as_deref());
            let ltext = |k: ResKey| host.ltext_of(k);
            let types = host
                .type_exemplars()
                .iter()
                .map(|(instance, holder)| CrimeType::from_exemplar(*instance, holder, &ltext))
                .collect();
            Box::into_raw(Box::new(Shadow {
                tuning,
                tuning_found: exemplar.is_some(),
                types,
            })) as *mut fc4_crime_shadow
        }
        _ => std::ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_crime_shadow_destroy(s: *mut fc4_crime_shadow) {
    if s.is_null() {
        return;
    }
    guard((), || drop(Box::from_raw(s as *mut Shadow)));
}

#[no_mangle]
pub unsafe extern "C" fn fc4_crime_shadow_get_info(
    s: *const fc4_crime_shadow,
    out: *mut fc4_crime_shadow_info,
) -> bool {
    let (Some(s), Some(o)) = ((s as *const Shadow).as_ref(), out.as_mut()) else {
        return false;
    };
    *o = fc4_crime_shadow_info {
        struct_size: std::mem::size_of::<fc4_crime_shadow_info>() as u32,
        types: s.types.len() as u32,
        tuning_found: u8::from(s.tuning_found),
        pad: [0; 3],
        tract_size: s.tuning.tract_size,
        decay: s.tuning.decay,
        reoffend: s.tuning.reoffend,
    };
    true
}

/// Whether type `type_id` is a candidate for a lot with these facts (§6.7
/// masks): 1 yes, 0 no, -1 unknown type. `has_building` = 0: no building.
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_shadow_candidate(
    s: *const fc4_crime_shadow,
    type_id: u32,
    zone_type: u32,
    has_building: bool,
    purpose: u32,
    wealth: u32,
    condition: u8,
) -> i32 {
    let Some(s) = (s as *const Shadow).as_ref() else {
        return -1;
    };
    let Some(t) = s.types.iter().find(|t| t.id == type_id) else {
        return -1;
    };
    let building = has_building.then_some((purpose as i32, wealth as i32));
    i32::from(t.matches(&LotBits::new(zone_type, building, condition)))
}

/// Replays pipeline state 4's grid work (§6.5, §6.6 step 7, §6.9) on the
/// given grid and incidents: decay with the tuning's factor, the hotspot
/// value re-read at its tract, each incident's severity stamped in order,
/// then the zoned average. `post` receives `count_x × count_z` values.
#[no_mangle]
pub unsafe extern "C" fn fc4_crime_shadow_month(
    s: *const fc4_crime_shadow,
    input: *const fc4_crime_shadow_month_in,
    post: *mut u8,
    out: *mut fc4_crime_shadow_month_out,
) -> bool {
    let (Some(s), Some(i), Some(o)) = ((s as *const Shadow).as_ref(), input.as_ref(), out.as_mut()) else {
        return false;
    };
    guard(false, || {
        let g = i.geometry;
        let geom = TractGeometry {
            tract_size: g.tract_size,
            shift: g.shift,
            count_x: g.count_x,
            count_z: g.count_z,
            width_x: g.width_x,
            width_z: g.width_z,
            inv_width_x: g.inv_width_x,
            inv_width_z: g.inv_width_z,
        };
        let n = geom.len();
        if n > 0 && (i.pre.is_null() || post.is_null()) {
            return false;
        }
        let pre = if n == 0 {
            &[][..]
        } else {
            std::slice::from_raw_parts(i.pre, n)
        };
        let grid = SimGrid::<u8>::from_values(geom, pre);
        decay_grid(&*grid, s.tuning.decay);
        let mut hot = (i.hotspot_x, i.hotspot_z, i32::from(grid.get(i.hotspot_x, i.hotspot_z)));
        let incidents = if i.incidents.is_null() || i.incident_count == 0 {
            &[][..]
        } else {
            std::slice::from_raw_parts(i.incidents, i.incident_count as usize)
        };
        let mut r = fc4_crime_shadow_month_out {
            struct_size: std::mem::size_of::<fc4_crime_shadow_month_out>() as u32,
            ..Default::default()
        };
        for inc in incidents {
            let Some(t) = s.types.iter().find(|t| t.id == inc.type_id) else {
                r.unknown_types += 1;
                continue;
            };
            if t.severity == 0 {
                continue; // recorded but neither stamped nor counted (§6.8)
            }
            r.count += 1;
            r.severity_sum += u32::from(t.severity);
            if inc.has_rect == 0 {
                r.no_rect += 1;
                continue;
            }
            let rc = inc.rect;
            stamp_rect(&*grid, [rc.x1, rc.z1, rc.x2, rc.z2], t.severity, &mut hot);
        }
        if !i.zones.is_null() && n > 0 {
            let zones = std::slice::from_raw_parts(i.zones, n);
            let s_ = geom.tract_size.max(1);
            let average = zoned_average(&*grid, |cx, cz| {
                let (tx, tz) = (cx / s_, cz / s_);
                geom.index(tx, tz).map_or(0, |k| zones[k])
            });
            r.average_bits = average.to_bits();
            r.average_computed = 1;
        }
        let values = grid.values();
        if n > 0 {
            std::ptr::copy_nonoverlapping(values.as_ptr(), post, n.min(values.len()));
        }
        r.hotspot_x = hot.0;
        r.hotspot_z = hot.1;
        r.hotspot_value = hot.2;
        *o = r;
        true
    })
}

#[cfg(test)]
mod ltext_tests {
    use super::*;

    /// Instance 1: loaded; 2: the detokenizer returned false (text sent,
    /// false); 3: an empty text that failed to detokenize; else missing.
    unsafe extern "C" fn ltext(_: U, t: u32, _g: u32, i: u32, sink: fc4_bytes_sink, ctx: *mut c_void) -> bool {
        assert_eq!(t, fc4_core::crime::NAME_LTEXT_TYPE);
        let send = |s: &[u8]| sink.unwrap()(ctx, s.as_ptr(), s.len());
        match i {
            1 => {
                send(b"ok");
                true
            }
            2 => {
                send(b"#tok#");
                false
            }
            3 => {
                send(b"");
                false
            }
            _ => false,
        }
    }

    /// CQ8: the three outcomes of the host's LTEXT load (fc4.h `ltext`).
    #[test]
    fn ltext_outcomes() {
        // SAFETY: an all-null host is valid (missing callbacks are defaults).
        let mut h: fc4_crime_host = unsafe { std::mem::zeroed() };
        h.ltext = Some(ltext);
        let host = CHost(h);
        let key = |instance| ResKey {
            type_id: fc4_core::crime::NAME_LTEXT_TYPE,
            group: 7,
            instance,
        };
        assert_eq!(host.ltext_of(key(1)), LtextLoad::Loaded(b"ok".to_vec()));
        // CQ16: the text a failed detokenizer left is discarded.
        assert_eq!(host.ltext_of(key(2)), LtextLoad::DetokenizeFailed);
        assert_eq!(host.ltext_of(key(3)), LtextLoad::DetokenizeFailed);
        assert_eq!(host.ltext_of(key(4)), LtextLoad::Missing);
        // No callback: missing.
        h.ltext = None;
        assert_eq!(CHost(h).ltext_of(key(1)), LtextLoad::Missing);
    }
}
