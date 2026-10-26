// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! One `cSC4MySimDispatch` (mysims.md §4.6.1, §4.7.1, §6.2.1, §7.2.1,
//! §7.2.3) over the C ABI: the plugin's class-factory front for CLSID
//! `0xCBC14674` (`enable = mysims`) owns one handle. See the "MySim
//! dispatch" section of `include/fc4.h`.
//!
//! The dispatch's foreign services are the host's: the city cells, lots and
//! occupant positions of the job search, the owner MySim's wealth, and the
//! synchronous `0x2BA06B99` owner notification (§3.3). Transport (automata,
//! transit stops, route search, the player vehicle) is MS18 and not
//! supplied: every such request reports "unsupported", never an arrival
//! (§10). The job-search pathfinder runs on an empty network model, so a
//! search finds no path until a traffic network is wired to it.

use std::cell::Cell;
use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::grid::{CellRect, CityDims, CoreGrids, GridProvider, TractGrid};
use fc4_core::mysims::dispatch::{
    DispatchHost, DispatchLocation, DispatchRecordRefs, LotAt, MySimDispatch, TripOutcome, TripServices,
};
use fc4_core::mysims::JobDestination;
use fc4_core::pathfinder::{shared_work_check, PathfinderHost, TravelTables};
use fc4_core::stream::RefValue;
use fc4_core::traffic_network::{NetworkHost, NetworkModel, PathInfo};
use fc4_core::variant::Variant;

use crate::advisors::{fc4_mysim_dispatch, fc4_mysim_job_destination, FC4_MYSIM_JOB_LOT, FC4_MYSIM_JOB_NEIGHBOR};
use crate::guard;
use crate::stream::{fc4_istream, fc4_ostream, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

/// The one dispatch key inside the handle's core (one owner per object).
const SIM: i32 = 1;

#[repr(C)]
pub struct fc4_mysim_dispatch_obj {
    _private: [u8; 0],
}

/// What one dispatch needs from the game (fc4.h `fc4_mysim_dispatch_host`).
/// Missing callbacks read as absent / 0 / do nothing.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_mysim_dispatch_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    /// `cISC4City::CellCountX/Z`.
    pub cell_counts: Option<unsafe extern "C" fn(U, *mut i32, *mut i32)>,
    /// An occupant's lot bounding cells (x1, z1, x2, z2); false = no lot.
    pub occupant_lot_rect: Option<unsafe extern "C" fn(U, u64, *mut i32) -> bool>,
    /// The bounding cells of the lot at a cell; false = no lot.
    pub lot_rect_at: Option<unsafe extern "C" fn(U, i32, i32, *mut i32) -> bool>,
    /// `GetLot(x, z)` → `GetBuilding()`: 0 no lot, 1 no building, 2 building (`*building`).
    pub lot_at_cell: Option<unsafe extern "C" fn(U, i32, i32, *mut u64) -> u32>,
    /// An occupant's `GetPosition()` x, z.
    pub occupant_position: Option<unsafe extern "C" fn(U, u64, *mut f32, *mut f32) -> bool>,
    /// The owner MySim's `GetWealth()`.
    pub wealth: Option<unsafe extern "C" fn(U) -> u32>,
    /// `FindCityOnOtherSideOfConnection(x, z)` → the city's region position.
    pub city_across_connection: Option<unsafe extern "C" fn(U, i32, i32, *mut i32, *mut i32) -> bool>,
    /// The region's `GetCity(x, z)` (0 = none).
    pub region_city: Option<unsafe extern "C" fn(U, i32, i32) -> u64>,
    /// `GetConnectionsOfTypeToCity(type, city)` → the list length.
    pub connections_of_type_to_city: Option<unsafe extern "C" fn(U, u32, u64) -> u32>,
    /// Whether the dispatch has a MySim owner (§3.3).
    pub has_owner: Option<unsafe extern "C" fn(U) -> bool>,
    /// Synchronous `0x2BA06B99` delivery to the owner: (sub-ID, old, new).
    pub notify: Option<unsafe extern "C" fn(U, u32, u32, u32)>,
}

/// Diagnostics (`fc4_mysim_dispatch_get_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_mysim_dispatch_state {
    pub struct_size: u32,
    pub state: i32,
    pub prev_state: i32,
    pub flags: u32,
    pub travel_type: u32,
    pub minutes: i32,
    pub change_counter: u32,
    pub queue_len: u32,
    pub has_pathfinder: u8,
    pub home_in_city: u8,
    pub commute_by_commute: u8,
    pub reserved: u8,
    pub home: u64,
    pub commute: u64,
}

/// `fc4_mysim_dispatch_commute` / `_go_home` / `_u_drive` results.
pub const FC4_MYSIM_TRIP_ACCEPTED: u32 = 0;
pub const FC4_MYSIM_TRIP_REJECTED: u32 = 1;
pub const FC4_MYSIM_TRIP_UNSUPPORTED: u32 = 2;

fn outcome(o: TripOutcome) -> u32 {
    match o {
        TripOutcome::Accepted => FC4_MYSIM_TRIP_ACCEPTED,
        TripOutcome::Rejected => FC4_MYSIM_TRIP_REJECTED,
        TripOutcome::UnsupportedMs18 => FC4_MYSIM_TRIP_UNSUPPORTED,
    }
}

struct CHost {
    h: fc4_mysim_dispatch_host,
}

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $a:expr)*) => {
        // SAFETY: the host's callbacks are valid for the handle's lifetime (fc4.h).
        match $s.h.$f {
            Some(f) => unsafe { f($s.h.user $(, $a)*) },
            None => $default,
        }
    };
}

impl CHost {
    fn rect(&self, f: impl FnOnce(*mut i32) -> bool) -> Option<CellRect> {
        let mut r = [0i32; 4];
        f(r.as_mut_ptr()).then(|| CellRect::new(r[0], r[1], r[2], r[3]))
    }
    fn counts(&self) -> (i32, i32) {
        let (mut x, mut z) = (0i32, 0i32);
        cb!(self, cell_counts, (), &mut x, &mut z);
        (x, z)
    }
}

fn dims(c: (i32, i32)) -> CoreGrids {
    CoreGrids {
        city: CityDims {
            cells_x: c.0.max(1),
            cells_z: c.1.max(1),
            ..CityDims::square(1)
        },
    }
}

impl GridProvider<u32> for CHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u32>>> {
        GridProvider::<u32>::create_grid(&dims(self.counts()), instance, tract_size)
    }
}

impl GridProvider<u8> for CHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        GridProvider::<u8>::create_grid(&dims(self.counts()), instance, tract_size)
    }
}

/// No network occupants: the model the job search reads stays empty.
impl NetworkHost for CHost {
    fn cell_counts(&self) -> (i32, i32) {
        self.counts()
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        ((x / 16.0).floor() as i32, (z / 16.0).floor() as i32)
    }
    fn network_occupants_in_rect(&self, _rect: CellRect) -> Vec<u64> {
        Vec::new()
    }
    fn subway_occupants_in_rect(&self, _rect: CellRect) -> Vec<u64> {
        Vec::new()
    }
    fn prop_occupants_in_rect(&self, _rect: CellRect) -> Vec<u64> {
        Vec::new()
    }
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        let (x, z) = DispatchHost::occupant_position(self, occ);
        [x, 0.0, z]
    }
    fn occupant_bounding_cells(&self, occ: u64) -> CellRect {
        self.occupant_lot_rect(occ).unwrap_or(CellRect::new(0, 0, -1, -1))
    }
    fn prop_type(&self, _occ: u64) -> u32 {
        0
    }
    fn occupant_property(&self, _occ: u64, _id: u32) -> Option<Variant> {
        None
    }
    fn has_network_flag(&self, _occ: u64, _flag: u32) -> bool {
        false
    }
    fn has_any_network_flag(&self, _occ: u64, _mask: u32) -> bool {
        false
    }
    fn is_of_type(&self, _occ: u64, _network_type: u32) -> bool {
        false
    }
    fn path_connection(&self, _occ: u64, _path_type: u32, _entry: u32, _exit: u32) -> bool {
        false
    }
    fn path_info(&self, _occ: u64) -> Option<PathInfo> {
        None
    }
    fn path_info_turn_around(&self, _pi: PathInfo, _path_type: u32, _entry: u32) -> bool {
        false
    }
    fn path_info_exits_from_entry(&self, _pi: PathInfo, _path_type: u32, _entry: u32) -> Vec<(u8, u64)> {
        Vec::new()
    }
    fn path_info_entries_near(&self, _pi: PathInfo, _t: u32, _vertex: u64, _exit: u32, _radius: f32) -> Vec<u32> {
        Vec::new()
    }
    fn is_tunnel(&self, _occ: u64) -> bool {
        false
    }
    fn tunnel_paired_portal(&self, _occ: u64) -> Option<u64> {
        None
    }
    fn lot_at(&self, _x: i32, _z: i32, _anchor_only: bool) -> Option<u64> {
        None
    }
    fn lot_bounding_rect(&self, _lot: u64) -> CellRect {
        CellRect::new(0, 0, -1, -1)
    }
    fn lot_location(&self, _lot: u64) -> (i32, i32) {
        (0, 0)
    }
    fn lot_facing(&self, _lot: u64) -> u32 {
        0
    }
    fn lot_building(&self, _lot: u64) -> Option<u64> {
        None
    }
    fn lot_populations(&self, _lot: u64) -> Vec<Vec<(u32, u32)>> {
        Vec::new()
    }
    fn lot_holder(&self, _lot: u64) -> u64 {
        0
    }
    fn holder_property(&self, _holder: u64, _id: u32) -> Option<Variant> {
        None
    }
    fn set_holder_property(&self, _holder: u64, _id: u32, _value: Variant) {}
    fn remove_holder_property(&self, _holder: u64, _id: u32) -> bool {
        false
    }
    fn max_trip_capacity(&self, _holder: u64, _k: u32) -> u32 {
        0
    }
    fn neighbor_pool_holder(&self, _x: i32, _z: i32) -> Option<u64> {
        None
    }
    fn is_road_damaged(&self, _x: i32, _z: i32) -> bool {
        false
    }
    fn cell_under_water(&self, _x: i32, _z: i32) -> bool {
        false
    }
    fn is_navigable(&self, _x: i32, _z: i32) -> bool {
        false
    }
    fn water_depth(&self, _x: i32, _z: i32) -> f32 {
        0.0
    }
    fn body_of_water(&self, _x: i32, _z: i32) -> Option<u16> {
        None
    }
    fn connections_of_type(&self, _connection_type: u32) -> Vec<u64> {
        Vec::new()
    }
    fn all_connections(&self) -> Vec<u64> {
        Vec::new()
    }
    fn connection_cell(&self, _conn: u64) -> (i32, i32) {
        (0, 0)
    }
    fn post(&self, _msg_type: u32) {}
    fn add_ref(&self, _token: u64) {}
    fn release(&self, _token: u64) {}
}

/// The traffic tables and grids of an empty network (no path is found).
impl PathfinderHost for CHost {
    fn cell_counts(&self) -> (i32, i32) {
        self.counts()
    }
    fn work_deadline(&self, _ms: u32) -> u64 {
        0
    }
    fn work_deadline_reached(&self, _deadline: u64) -> bool {
        false
    }
    fn travel_tables(&self) -> TravelTables {
        TravelTables::default()
    }
    fn max_distance_origin_to_network(&self) -> u32 {
        0
    }
    fn nearest_destination_attractiveness(&self) -> f32 {
        0.0
    }
    fn strike_state(&self) -> u8 {
        0
    }
    fn trip_scale_for_display(&self) -> f32 {
        1.0
    }
    fn volume(&self, _n: u32, _x: i32, _z: i32) -> f32 {
        0.0
    }
    fn set_volume(&self, _n: u32, _x: i32, _z: i32, _v: f32) {}
    fn destination_byte(&self, _x: i32, _z: i32) -> u8 {
        0
    }
    fn set_destination_byte(&self, _x: i32, _z: i32, _bits: u8) {}
    fn free_destinations(&self, _k: u32) -> u32 {
        0
    }
    fn set_free_destinations(&self, _k: u32, _n: u32) {}
    fn max_trip_capacity(&self, _holder: u64, _k: u32) -> u32 {
        0
    }
    fn used_trip_capacity(&self, _holder: u64, _k: u32) -> u32 {
        0
    }
    fn neighbor_pool_holder(&self, _t: u32, _x: i32, _z: i32) -> Option<u64> {
        None
    }
    fn income_accumulator(&self) -> f64 {
        0.0
    }
    fn set_income_accumulator(&self, _v: f64) {}
    fn lot_at(&self, _x: i32, _z: i32) -> Option<u64> {
        None
    }
    fn lot_holder(&self, _lot: u64) -> u64 {
        0
    }
    fn lot_bounding_rect(&self, _lot: u64) -> CellRect {
        CellRect::new(0, 0, -1, -1)
    }
    fn occupants_in_cells(&self, _rect: CellRect) -> Vec<u64> {
        Vec::new()
    }
    fn occupant_filter_includes(&self, _filter: u64, _occupant: u64) -> bool {
        false
    }
    fn is_ordinance_on(&self, _id: u32) -> bool {
        false
    }
}

impl DispatchHost for CHost {
    fn home_rect(&self, _sim: i32) -> Option<CellRect> {
        None
    }
    fn building_rect(&self, building: u64) -> Option<CellRect> {
        self.occupant_lot_rect(building)
    }
    fn wealth(&self, _sim: i32) -> u32 {
        cb!(self, wealth, 0)
    }
    fn occupant_lot_rect(&self, occupant: u64) -> Option<CellRect> {
        self.rect(|r| cb!(self, occupant_lot_rect, false, occupant, r))
    }
    fn lot_rect_at(&self, x: i32, z: i32) -> Option<CellRect> {
        self.rect(|r| cb!(self, lot_rect_at, false, x, z, r))
    }
    fn cell_is_in_bounds(&self, x: i32, z: i32) -> bool {
        let (cx, cz) = self.counts();
        (0..cx).contains(&x) && (0..cz).contains(&z)
    }
    fn lot_at_cell(&self, x: i32, z: i32) -> LotAt {
        let mut b = 0u64;
        match cb!(self, lot_at_cell, 0, x, z, &mut b) {
            2 if b != 0 => LotAt::Building(b),
            1 | 2 => LotAt::NoBuilding,
            _ => LotAt::NoLot,
        }
    }
    fn city_across_connection(&self, x: i32, z: i32) -> Option<(i32, i32)> {
        let (mut cx, mut cz) = (0i32, 0i32);
        cb!(self, city_across_connection, false, x, z, &mut cx, &mut cz).then_some((cx, cz))
    }
    fn occupant_position(&self, occupant: u64) -> (f32, f32) {
        let (mut x, mut z) = (0.0f32, 0.0f32);
        if cb!(self, occupant_position, false, occupant, &mut x, &mut z) {
            (x, z)
        } else {
            (0.0, 0.0)
        }
    }
    fn region_city(&self, x: i32, z: i32) -> Option<u64> {
        let c = cb!(self, region_city, 0, x, z);
        (c != 0).then_some(c)
    }
    fn connections_of_type_to_city(&self, connection_type: u32, city: u64) -> usize {
        cb!(self, connections_of_type_to_city, 0, connection_type, city) as usize
    }
    fn has_owner(&self, _sim: i32) -> bool {
        cb!(self, has_owner, false)
    }
    fn notify(&self, _sim: i32, sub_id: u32, data1: u32, data2: u32) {
        cb!(self, notify, (), sub_id, data1, data2)
    }
}

/// Transport, automata and the player vehicle are MS18: every handoff is
/// the trait's "unsupported" answer (no arrival is manufactured, §10).
struct NoTransport;
impl TripServices for NoTransport {}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn fc4_core::stream::SaveObject>> {
    None
}

/// Occupant references cross as the host's tokens (the stream adapter writes
/// and resolves them).
struct HostRefs;
impl DispatchRecordRefs for HostRefs {
    fn reference(&self, token: u64) -> RefValue {
        RefValue::Host { token, class_id: 0 }
    }
    fn token(&self, reference: RefValue) -> Option<u64> {
        match reference {
            RefValue::Host { token, .. } if token != 0 => Some(token),
            _ => None,
        }
    }
}

struct Handle {
    core: Rc<MySimDispatch>,
    host: Rc<CHost>,
    initialised: Cell<bool>,
}

fn with<R>(p: *const fc4_mysim_dispatch_obj, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live handle.
    match unsafe { (p as *const Handle).as_ref() } {
        Some(h) => guard(fallback, || f(h)),
        None => fallback,
    }
}

/// Creates one dispatch with the constructor state (§7.2.3); NULL if
/// `host` is NULL or too short. The host table is copied.
#[no_mangle]
pub unsafe extern "C" fn fc4_mysim_dispatch_create(host: *const fc4_mysim_dispatch_host) -> *mut fc4_mysim_dispatch_obj {
    guard(std::ptr::null_mut(), || match host.as_ref() {
        Some(h) if (h.struct_size as usize) >= std::mem::size_of::<fc4_mysim_dispatch_host>() => {
            let host = Rc::new(CHost { h: *h });
            let net = NetworkModel::new(host.clone() as Rc<dyn NetworkHost>);
            let core = MySimDispatch::new(host.clone() as Rc<dyn DispatchHost>, net, shared_work_check());
            Box::into_raw(Box::new(Handle {
                core,
                host,
                initialised: Cell::new(false),
            })) as *mut fc4_mysim_dispatch_obj
        }
        _ => std::ptr::null_mut(),
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_mysim_dispatch_destroy(p: *mut fc4_mysim_dispatch_obj) {
    if !p.is_null() {
        guard((), || drop(Box::from_raw(p as *mut Handle)));
    }
}

/// `Init`: creates the job-search pathfinder (§4.7.2) and clears the MS22
/// auxiliary flag unless a Read is pending.
#[no_mangle]
pub unsafe extern "C" fn fc4_mysim_dispatch_init(p: *mut fc4_mysim_dispatch_obj) -> bool {
    with(p, false, |h| {
        let ok = h.core.init(SIM);
        h.initialised.set(ok);
        ok
    })
}

/// `Shutdown`: the state and the pathfinder go; the object stays usable.
#[no_mangle]
pub unsafe extern "C" fn fc4_mysim_dispatch_shutdown(p: *mut fc4_mysim_dispatch_obj) {
    with(p, (), |h| {
        h.core.shutdown(SIM);
        h.initialised.set(false);
    })
}

/// `SetDefaultDestination(SetByOccupant(home))` (§4.7.1 step 2): flag 0x80.
/// `home` 0 = a null occupant.
#[no_mangle]
pub unsafe extern "C" fn fc4_mysim_dispatch_set_home(p: *mut fc4_mysim_dispatch_obj, home: u64) {
    with(p, (), |h| {
        let o = (home != 0).then(|| (home, DispatchHost::occupant_position(&*h.host, home)));
        h.core.set_default_destination(SIM, DispatchLocation::by_occupant(o));
    })
}

/// `DispatchCommute` (§7.2.1) for the MySim's state and the hour.
#[no_mangle]
pub unsafe extern "C" fn fc4_mysim_dispatch_commute(
    p: *mut fc4_mysim_dispatch_obj,
    has_residence: bool,
    mysim_state: u8,
    hour: i32,
) -> u32 {
    with(p, FC4_MYSIM_TRIP_REJECTED, |h| {
        outcome(h.core.dispatch_commute(SIM, has_residence, mysim_state, hour))
    })
}

/// `FindNewJob` (§4.7.1): a by-commute default location; true if stored.
#[no_mangle]
pub unsafe extern "C" fn fc4_mysim_dispatch_find_new_job(p: *mut fc4_mysim_dispatch_obj) -> bool {
    with(p, false, |h| h.core.find_new_job(SIM))
}

/// `ChangeJob(b)` (§4.7.1); `building` 0 = null.
#[no_mangle]
pub unsafe extern "C" fn fc4_mysim_dispatch_change_job(p: *mut fc4_mysim_dispatch_obj, building: u64) -> bool {
    with(p, false, |h| h.core.change_job(SIM, (building != 0).then_some(building)))
}

/// The destination `0xEBA06BF4` reports (§4.7); false = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_mysim_dispatch_job_destination(
    p: *const fc4_mysim_dispatch_obj,
    out: *mut fc4_mysim_job_destination,
) -> bool {
    with(p, false, |h| {
        let Some(out) = out.as_mut() else { return false };
        *out = fc4_mysim_job_destination::default();
        match h.core.job_destination(SIM) {
            Some(JobDestination::Lot { building, cell }) => {
                out.kind = FC4_MYSIM_JOB_LOT;
                out.building = building;
                (out.cell_x, out.cell_z) = cell;
                true
            }
            Some(JobDestination::Neighbor { cell }) => {
                out.kind = FC4_MYSIM_JOB_NEIGHBOR;
                (out.cell_x, out.cell_z) = cell;
                true
            }
            None => false,
        }
    })
}

/// `CheckFlags(mask)`: all bits set.
#[no_mangle]
pub unsafe extern "C" fn fc4_mysim_dispatch_check_flags(p: *const fc4_mysim_dispatch_obj, mask: u32) -> bool {
    with(p, false, |h| h.core.check_flags(SIM, mask))
}

/// The trip fields the MySim's trip variables read (§4.6): the current
/// location, its occupant, the travelling destination's occupant, and the
/// reported wait value (the mutating getter of §4.6.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_mysim_dispatch_snapshot(
    p: *const fc4_mysim_dispatch_obj,
    out: *mut fc4_mysim_dispatch,
) -> bool {
    with(p, false, |h| {
        let Some(out) = out.as_mut() else { return false };
        let wait = h.core.wait_value(SIM);
        let st = h.core.get_state(SIM);
        *out = fc4_mysim_dispatch {
            state: st.search.state,
            flags: st.search.flags,
            cell_x: st.current.cell.0,
            cell_z: st.current.cell.1,
            position: [st.current.world.0, 0.0, st.current.world.1],
            travel_type: st.travel_type,
            wait_minutes: wait as u32,
            at_home: u8::from(st.current.same_location(&st.search.home)),
            at_work: u8::from(st.current.same_location(&st.search.commute_destination)),
            // [inferred] the arrival handler's "by commute" is the reached location's flag.
            by_commute: u8::from(st.current.by_commute),
            reserved: 0,
            destination: st.destination.occupant.unwrap_or(0),
            location: st.current.occupant.unwrap_or(0),
        };
        true
    })
}

/// Message `0x44778231`: +5 minutes once a pathfinder exists.
#[no_mangle]
pub unsafe extern "C" fn fc4_mysim_dispatch_minute(p: *mut fc4_mysim_dispatch_obj) {
    with(p, (), |h| h.core.minute_message(SIM))
}

/// Message `0x86AD10EE`: one state update of §4.6.1.
#[no_mangle]
pub unsafe extern "C" fn fc4_mysim_dispatch_animation(p: *mut fc4_mysim_dispatch_obj) -> u32 {
    with(p, FC4_MYSIM_TRIP_REJECTED, |h| outcome(h.core.animation_message(SIM, &NoTransport)))
}

/// Primary slots 6-8 (§7.2.3): 0 `CanUDriveIt`, 1 `EnterUDriveIt`, 2
/// `ExitUDriveIt`; the boolean the slot returns.
#[no_mangle]
pub unsafe extern "C" fn fc4_mysim_dispatch_u_drive(p: *mut fc4_mysim_dispatch_obj, op: u32) -> bool {
    with(p, false, |h| match op {
        0 => h.core.can_u_drive_it(SIM, &NoTransport),
        1 => h.core.enter_u_drive_it(SIM, &NoTransport) == TripOutcome::Accepted,
        2 => h.core.exit_u_drive_it(SIM, &NoTransport),
        _ => false,
    })
}

/// The dispatch record (§6.2.1); occupant references are host tokens.
#[no_mangle]
pub unsafe extern "C" fn fc4_mysim_dispatch_write(p: *const fc4_mysim_dispatch_obj, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with(p, false, |h| h.core.write_record(SIM, &mut COut(o), &HostRefs))
}

/// Reads the record; the restored values are replayed by the deferred
/// setup (`fc4_mysim_dispatch_deferred_setup`, PostCityInit).
#[no_mangle]
pub unsafe extern "C" fn fc4_mysim_dispatch_read(p: *mut fc4_mysim_dispatch_obj, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with(p, false, |h| {
        let mut input = CIn {
            stream: i,
            decode: no_core_objects,
        };
        h.core.read_record(SIM, &mut input, &HostRefs, &NoTransport)
    })
}

/// Message `0x26D31EC1` after a Read: Init, then the restored flags and
/// state notifications.
#[no_mangle]
pub unsafe extern "C" fn fc4_mysim_dispatch_deferred_setup(p: *mut fc4_mysim_dispatch_obj) -> bool {
    with(p, false, |h| {
        let ok = h.core.deferred_setup_message(SIM, &NoTransport);
        if ok {
            h.initialised.set(true);
        }
        ok
    })
}

#[no_mangle]
pub unsafe extern "C" fn fc4_mysim_dispatch_get_state(
    p: *const fc4_mysim_dispatch_obj,
    out: *mut fc4_mysim_dispatch_state,
) -> bool {
    with(p, false, |h| {
        let Some(out) = out.as_mut() else { return false };
        let st = h.core.get_state(SIM);
        *out = fc4_mysim_dispatch_state {
            struct_size: std::mem::size_of::<fc4_mysim_dispatch_state>() as u32,
            state: st.search.state,
            prev_state: st.search.prev_state,
            flags: st.search.flags,
            travel_type: st.travel_type,
            minutes: st.minutes,
            change_counter: st.change_counter,
            queue_len: st.queue.len() as u32,
            has_pathfinder: u8::from(h.initialised.get()),
            home_in_city: u8::from(st.search.home.in_city),
            commute_by_commute: u8::from(st.search.commute_destination.by_commute),
            reserved: 0,
            home: st.search.home.occupant.unwrap_or(0),
            commute: st.search.commute_destination.occupant.unwrap_or(0),
        };
        true
    })
}
