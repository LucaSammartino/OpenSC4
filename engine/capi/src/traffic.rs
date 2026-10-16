// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The traffic simulator, its network model and its pathfinders as one C
//! ABI unit (fc4.h "traffic"; docs/specs/traffic-simulator.md,
//! traffic-network.md, pathfinder.md). The three cores are one replacement
//! unit (traffic-simulator.md §9, pathfinder.md §9): the plugin's
//! `cISC4TrafficSimulator` forwards its 44 slots to the `fc4_traffic_*`
//! functions, and the pathfinders it hands out through slot 6 forward theirs
//! to the `fc4_traffic_pf_*` functions.
//!
//! The host implements [`fc4_traffic_host`]: every `TrafficSimHost` and
//! `NetworkHost` method, the grids through the shared [`fc4_grid_api`]
//! (u8, u32 and f32 grids, the game's own `cSC4SimGrid<T>`).
//!
//! Tokens are opaque host identities (fc4.h): lots, occupants, buildings,
//! neighbour connections, property holders, departments, grids. 0 is null.
//! Tokens the host returns are **borrowed** unless a callback says otherwise
//! (`create_department`: one reference for the core, given back through
//! `release_department`; the switch buildings the network model keeps are
//! AddRef'd and Released through `add_ref` / `release`).
//!
//! # Re-entrancy
//!
//! The core takes its state out for the duration of an operation
//! (`TrafficSimulator::run`), so a re-entrant call finds no state and
//! answers the type's default. The host's callbacks do call back in: a lot's
//! own `UpdateTrafficVolume` (traffic source slot 4, §4.1.1) asks slots 26
//! and 27 in the middle of ConfirmTrips. Re-entrant calls of the query
//! slots are therefore answered from a snapshot taken at the start of the
//! outermost call ([`fc4_traffic_state`] fields, the strategy table, the map
//! tokens), and a message delivered re-entrantly is queued and run when the
//! outermost call ends. Both are counted.
//!
//! # Threads
//!
//! The core is single-threaded (`RefCell`). The plugin serialises every call
//! on one object with its own lock and answers other threads from its own
//! snapshot (built from [`fc4_traffic_get_state`]).

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::grid::{CellRect, GridProvider, TractGrid};
use fc4_core::pathfinder::{Pathfinder, RouteData};
use fc4_core::property::PropertyHolder;
use fc4_core::stream::{InStream, RefValue};
use fc4_core::traffic_network::{FerryRouteInfo, NetworkHost, NetworkState, PathInfo};
use fc4_core::traffic_simulator::{
    self as ts, Message, NeighbourCity, SimMessage, Source, StatsSet, TrafficSimHost, TrafficSimulator, TripData,
    CLSID_TRAFFIC_SIMULATOR,
};
use fc4_core::variant::{mem, Variant};

use crate::budget::fc4_token_callback;
use crate::guard;
use crate::hostgrid::{
    collect_tokens, create_grid, fc4_cell_rect, fc4_grid_api, grid_token, native_variant, read_property, resolve_grid,
    take_props, NativeVariant,
};
use crate::props::fc4_props;
use crate::stream::{fc4_istream, fc4_ostream, fc4_ref, istream_ok, ostream_ok, CIn, COut};
use crate::zones::{fc4_zone_region, region_in};

type U = *mut c_void;

// ---- types crossing the ABI -------------------------------------------------------------------------

/// The kind of a traffic source (`fc4_traffic_trip_list` and friends).
pub const FC4_TRAFFIC_SOURCE_LOT: u32 = 0;
pub const FC4_TRAFFIC_SOURCE_CONNECTION: u32 = 1;

/// The maps of [`fc4_traffic_map`] (slots 8–10, 13) and the other grids.
pub const FC4_TRAFFIC_MAP_AIR: u32 = 0;
pub const FC4_TRAFFIC_MAP_COMMERCIAL: u32 = 1;
pub const FC4_TRAFFIC_MAP_CONGESTION: u32 = 2;
pub const FC4_TRAFFIC_MAP_TRIP_LENGTH: u32 = 3;

/// One `cSC4TripData` (§2) crossing the ABI. The two routes are byte
/// strings in the compressed route format (pathfinder.md §4.15); the
/// pointers are valid for the duration of the call that hands them over.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_traffic_trip {
    pub count: u32,
    pub trip_type: u8,
    pub strategy: u8,
    pub flags: u8,
    pub status: u8,
    pub dest_x: i16,
    pub dest_z: i16,
    pub trip_length: f32,
    pub travel_types_used: u32,
    pub route: *const u8,
    pub route_len: usize,
    pub return_route: *const u8,
    pub return_len: usize,
}

impl Default for fc4_traffic_trip {
    fn default() -> Self {
        fc4_traffic_trip {
            count: 0,
            trip_type: 0,
            strategy: 0,
            flags: 0,
            status: 0,
            dest_x: 0,
            dest_z: 0,
            trip_length: 0.0,
            travel_types_used: 0,
            route: std::ptr::null(),
            route_len: 0,
            return_route: std::ptr::null(),
            return_len: 0,
        }
    }
}

/// Receives one trip of a source's list, in list order.
pub type fc4_traffic_trip_callback = Option<unsafe extern "C" fn(ctx: *mut c_void, trip: *const fc4_traffic_trip)>;

/// An established neighbouring city (`GetConnectedCities`, traffic-network.md
/// §2.7.4): its region-cell position and its extrapolated populations.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_traffic_city {
    pub x: i32,
    pub z: i32,
    pub population: [i32; 3],
}

pub type fc4_traffic_city_callback = Option<unsafe extern "C" fn(ctx: *mut c_void, city: *const fc4_traffic_city)>;

/// One (developer type, count) entry of a lot's population maps; `map` is
/// the index of the map in the lot's vector.
pub type fc4_traffic_population_callback =
    Option<unsafe extern "C" fn(ctx: *mut c_void, map: u32, developer_type: u32, count: u32)>;

/// One path of a path info: its exit byte and the token of its first vertex.
pub type fc4_traffic_exit_callback = Option<unsafe extern "C" fn(ctx: *mut c_void, exit: u8, vertex: u64)>;

/// One u32 value (an ordinance ID, an entry side, a subnetwork, a count).
pub type fc4_traffic_u32_callback = Option<unsafe extern "C" fn(ctx: *mut c_void, value: u32)>;

/// A byte string (a compressed or water route).
pub type fc4_traffic_bytes_callback = Option<unsafe extern "C" fn(ctx: *mut c_void, bytes: *const u8, len: usize)>;

/// A message the simulator posts (§5): queued (`MessagePost`), with the
/// simulator as the `IGZUnknown` where §5 says so (the host decides).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_traffic_post {
    pub msg_type: u32,
    pub data1: u32,
    pub data2: u32,
    pub data3: u32,
}

/// The five fields of `GetTransitSwitchQueryData`'s `out` (§4.16, slot 35).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_traffic_switch_query {
    pub available: u32,
    pub max_capacity: u32,
    pub load_fraction: f32,
    pub usage: u32,
    pub cost: u32,
}

/// One `FerryRouteInfo` of slot 31 (traffic-network.md §4.9.5) without its
/// two routes, which come with the callback.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_traffic_ferry_route {
    pub lot_a: u64,
    pub lot_b: u64,
    pub ferry_type: u32,
    pub counter1: [u16; 2],
    pub counter2: [u16; 2],
    pub neighbour: u8,
    pub reserved: u8,
    pub neighbour_x: i16,
    pub neighbour_z: i16,
    pub reserved2: u16,
}

pub type fc4_traffic_ferry_callback = Option<
    unsafe extern "C" fn(
        ctx: *mut c_void,
        info: *const fc4_traffic_ferry_route,
        route_ab: *const u8,
        len_ab: usize,
        route_ba: *const u8,
        len_ba: usize,
    ),
>;

/// The state (diagnostics, the log lines and the plugin's snapshot for
/// other threads).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_traffic_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub loaded: u8,
    /// The work-cycle state (§3.2).
    pub cycle_state: u8,
    /// 2 = no strike, 0 started, 1 on strike (§4.12).
    pub strike_state: u8,
    pub day_index: u32,
    pub days: u32,
    pub total: i32,
    pub processed: i32,
    pub pass_total: i32,
    pub to_reroute: i32,
    pub to_search: i32,
    pub last_day: i32,
    pub pass_counters: [u32; 10],
    pub units_per_slice: u32,
    /// Slots 14, 15, 18, 27.
    pub trip_scale: f32,
    pub trip_scale_for_display: f32,
    pub last_monthly_income: f32,
    pub freight_scaling: f32,
    pub transit_income: f32,
    pub toll_income: f32,
    pub avg_car_commute: f32,
    /// Slot 26 for wealth 0…3, `r24(pct × 0.01)`.
    pub pct: [[f32; 3]; 4],
    pub attempted: [u32; 3],
    pub routed: [u32; 3],
    pub job_capacity: [u32; 3],
    pub free_dest: [u32; 4],
    /// The per-travel-type trip counts of the last cycle (§4.6).
    pub type_counts: [u32; 9],
    pub freight_byte: u8,
    pub reserved: [u8; 3],
    /// The accident-prone list's length (§4.9).
    pub accident_cells: u32,
    /// Accident messages posted (`0x0C05E4C3`) since creation.
    pub accidents_posted: u32,
    /// Messages posted since creation.
    pub posted: u32,
    /// `update_traffic_volume` host calls answered by the source itself.
    pub native_updates: u32,
    /// Trip-list reads and writes since creation.
    pub trip_reads: u32,
    pub trip_writes: u32,
    pub reentrant_calls: u32,
    /// Messages that arrived during another call and were run after it.
    pub queued_messages: u32,
    /// The pothole count (u16 at `0x134` of the slot-34 block).
    pub potholes: u32,
    pub road_damaged: u32,
    pub rail_damaged: u32,
    pub edge_records: u32,
    pub switches: u32,
    pub ferry_terminals: u32,
    pub reserved2: u32,
    /// The map tokens of slots 8, 9, 10, 13 (borrowed; 0 = none).
    pub air: u64,
    pub commercial: u64,
    pub congestion: u64,
    pub trip_length: u64,
    pub roads: u64,
    pub mass_transit: u64,
}

/// Everything the traffic unit asks of the game (fc4.h `fc4_traffic_host`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_traffic_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    /// The grids: u8 `0x49B9E602`, u32 `0x49B9E606`, f32 `0x49B9E60A`.
    pub grids: fc4_grid_api,
    // ---- framework (§1, §5) ----
    /// Simulator `AddAgent(target, level, name, flags)`.
    pub add_agent: Option<unsafe extern "C" fn(U, u32, *const u8, usize, u32)>,
    pub subscribe: Option<unsafe extern "C" fn(U, u32)>,
    /// `MessagePost` of a standard message (queued).
    pub post_message: Option<unsafe extern "C" fn(U, *const fc4_traffic_post)>,
    /// `AddRef` / `Release` of a host object the model keeps (a switch
    /// building).
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    // ---- city, tuning, dates ----
    /// City slots 93, 94.
    pub cell_counts: Option<unsafe extern "C" fn(U, *mut i32, *mut i32)>,
    /// City slot 95 `PositionToCell(x, z, &cx, &cz)`.
    pub position_to_cell: Option<unsafe extern "C" fn(U, f32, f32, *mut i32, *mut i32)>,
    /// City slot 91 cell width (ST0).
    pub cell_width: Option<unsafe extern "C" fn(U) -> f32>,
    /// The tuning exemplar (§4.14): a NEW fc4_props the core destroys, or
    /// NULL.
    pub tuning: Option<unsafe extern "C" fn(U) -> *mut fc4_props>,
    pub sim_day_of_month: Option<unsafe extern "C" fn(U) -> u32>,
    pub sim_date_number: Option<unsafe extern "C" fn(U) -> u32>,
    /// The simulation clock, in milliseconds of game time (§4.13 [choice]).
    pub sim_clock_ms: Option<unsafe extern "C" fn(U) -> u64>,
    // ---- departments (budget.md §7–§8) ----
    /// A department with line items 1…6: ONE reference for the core (given
    /// back through `release_department`); 0 = none.
    pub create_department: Option<unsafe extern "C" fn(U, u32, f32) -> u64>,
    pub release_department: Option<unsafe extern "C" fn(U, u64)>,
    /// Department slot 10, the funding fraction.
    pub department_funding: Option<unsafe extern "C" fn(U, u64) -> f32>,
    /// Line item `item`'s `SetIncome(int64)`.
    pub set_line_item_income: Option<unsafe extern "C" fn(U, u64, u32, i64)>,
    /// `UpdateRoadsBudget` / `UpdateMassTransitBudget`: 13 cell counts and
    /// 13 per-tile costs.
    pub update_network_costs: Option<unsafe extern "C" fn(U, u64, bool, *const u32, *const f32)>,
    /// §4.1.3 step 3 (building, lot or 0) → the int64 to add to `E`.
    pub activate_budget_items: Option<unsafe extern "C" fn(U, u64, u64) -> i64>,
    /// §4.1.3 DoLotDeactivated step 3 (the lot's holder) → the cost to
    /// subtract from `E`.
    pub deactivate_budget_items: Option<unsafe extern "C" fn(U, u64) -> i64>,
    /// Budget slot 51 on the building's holder: department ID and cost.
    pub budget_item_for_purpose: Option<unsafe extern "C" fn(U, u64, u32, *mut u32, *mut i64) -> bool>,
    /// Budget slot 28 `GetDepartmentBudget(id)`: borrowed; 0 = none.
    pub department_budget: Option<unsafe extern "C" fn(U, u32) -> u64>,
    /// History warehouse slot 12 `(id, value)`.
    pub record_history: Option<unsafe extern "C" fn(U, u32, u32)>,
    // ---- lots ----
    /// Every lot in the lot manager's list order: borrowed tokens.
    pub lots: Option<unsafe extern "C" fn(U, fc4_token_callback, U)>,
    /// `QueryInterface(cISC4Lot)` on a message object: borrowed; 0 = none.
    pub as_lot: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub lot_zone: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub lot_is_livable: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub lot_has_road_access: Option<unsafe extern "C" fn(U, u64) -> bool>,
    /// Lot slot 59: borrowed; 0 = none.
    pub lot_link: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub lot_travel_population: Option<unsafe extern "C" fn(U, u64, u32) -> u32>,
    pub lot_strategy_byte: Option<unsafe extern "C" fn(U, u64) -> u8>,
    pub clear_lot_flag: Option<unsafe extern "C" fn(U, u64, u32)>,
    /// Lot manager slot 6 `GetLot(x, z, anchorOnly)`: borrowed; 0 = none.
    pub lot_at: Option<unsafe extern "C" fn(U, i32, i32, bool) -> u64>,
    pub lot_bounding_rect: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    pub lot_location: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32)>,
    pub lot_facing: Option<unsafe extern "C" fn(U, u64) -> u32>,
    /// Lot slot 48: borrowed; 0 = none.
    pub lot_building: Option<unsafe extern "C" fn(U, u64) -> u64>,
    /// Lot slot 54: every (map index, developer type, count) entry.
    pub lot_populations: Option<unsafe extern "C" fn(U, u64, fc4_traffic_population_callback, U)>,
    /// Lot slot 3: borrowed holder token; 0 = none.
    pub lot_holder: Option<unsafe extern "C" fn(U, u64) -> u64>,
    // ---- traffic sources (cISC4TrafficSource) ----
    /// Slot 4 `UpdateTrafficVolume` run by the source itself; true when the
    /// host ran it (false: the core computes it).
    pub update_traffic_volume: Option<unsafe extern "C" fn(U, u32, u64) -> bool>,
    /// Slot 3: every trip of the source's list, in list order. False when
    /// the source has no list.
    pub trip_list: Option<unsafe extern "C" fn(U, u32, u64, fc4_traffic_trip_callback, U) -> bool>,
    /// Replaces the source's list with these trips (routes copied).
    pub set_trip_list: Option<unsafe extern "C" fn(U, u32, u64, *const fc4_traffic_trip, usize)>,
    // ---- property holders ----
    /// Property `id` of a holder into the scratch `fc4_props`; false when
    /// absent.
    pub holder_property: Option<unsafe extern "C" fn(U, u64, u32, *mut fc4_props) -> bool>,
    /// `AddProperty(id, value)` (mem type, array flag, element bytes,
    /// count); for an existing property of the same type and length the
    /// host writes in place (foundations §10.2).
    pub set_holder_property: Option<unsafe extern "C" fn(U, u64, u32, u16, bool, *const c_void, usize)>,
    pub remove_holder_property: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    // ---- neighbour connections ----
    /// Connection slot 3: its property holder; borrowed.
    pub connection_holder: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub take_connection_volume_dirty: Option<unsafe extern "C" fn(U, u64) -> bool>,
    /// Neighbour simulator slot 11: borrowed; 0 = none.
    pub find_connection: Option<unsafe extern "C" fn(U, u32, i32, i32) -> u64>,
    pub connection_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    /// Neighbour simulator slot 13: borrowed; 0 = none.
    pub connection_on_other_side: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub established_city_position: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32) -> bool>,
    pub connected_cities: Option<unsafe extern "C" fn(U, fc4_traffic_city_callback, U)>,
    pub regional_demand: Option<unsafe extern "C" fn(U, u32, *mut f32) -> bool>,
    /// Neighbour simulator slots 17 and 16: borrowed tokens in list order.
    pub connections_of_type: Option<unsafe extern "C" fn(U, u32, fc4_token_callback, U)>,
    pub all_connections: Option<unsafe extern "C" fn(U, fc4_token_callback, U)>,
    /// Connection slot 6: the connection's cell.
    pub connection_cell: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32)>,
    // ---- other simulators ----
    pub workforce_percent: Option<unsafe extern "C" fn(U, i32, i32) -> f32>,
    pub developer_population: Option<unsafe extern "C" fn(U, u32, i32, i32) -> i16>,
    pub is_ordinance_on: Option<unsafe extern "C" fn(U, u32) -> bool>,
    pub ordinances_on: Option<unsafe extern "C" fn(U, fc4_traffic_u32_callback, U)>,
    pub ordinance_property: Option<unsafe extern "C" fn(U, u32, u32, *mut fc4_props) -> bool>,
    // ---- occupants ----
    /// `QueryInterface(cISC4NetworkOccupant)` on a message object:
    /// borrowed; 0 = none.
    pub as_network_occupant: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub network_occupant_cell: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32)>,
    /// Occupant manager slot 40 over a cell rect (pathfinder filter 2):
    /// borrowed tokens.
    pub occupants_in_cells: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, fc4_token_callback, U)>,
    pub occupant_filter_includes: Option<unsafe extern "C" fn(U, u64, u64) -> bool>,
    pub cell_has_network_occupant: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    /// Occupant manager slot 41 with types `0xC772BF98`, `0x8A13918A`,
    /// `0xA823821E` (prop occupants: only those answering
    /// `cISC4PropOccupant`): borrowed tokens in list order.
    pub network_occupants_in_rect: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, fc4_token_callback, U)>,
    pub subway_occupants_in_rect: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, fc4_token_callback, U)>,
    pub prop_occupants_in_rect: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, fc4_token_callback, U)>,
    /// Occupant slot 8: x, y, z.
    pub occupant_position: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    pub occupant_bounding_cells: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    pub prop_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub occupant_property: Option<unsafe extern "C" fn(U, u64, u32, *mut fc4_props) -> bool>,
    pub has_network_flag: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub has_any_network_flag: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub is_of_type: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub path_connection: Option<unsafe extern "C" fn(U, u64, u32, u32, u32) -> bool>,
    /// Network occupant slot 49: borrowed; 0 = none.
    pub path_info: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub path_info_turn_around: Option<unsafe extern "C" fn(U, u64, u32, u32) -> bool>,
    pub path_info_exits_from_entry: Option<unsafe extern "C" fn(U, u64, u32, u32, fc4_traffic_exit_callback, U)>,
    pub path_info_entries_near: Option<unsafe extern "C" fn(U, u64, u32, u64, u32, f32, fc4_traffic_u32_callback, U)>,
    pub is_tunnel: Option<unsafe extern "C" fn(U, u64) -> bool>,
    /// Borrowed; 0 = none.
    pub tunnel_paired_portal: Option<unsafe extern "C" fn(U, u64) -> u64>,
    // ---- terrain and water ----
    pub cell_under_water: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    pub is_navigable: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    pub water_depth: Option<unsafe extern "C" fn(U, i32, i32) -> f32>,
    pub body_of_water: Option<unsafe extern "C" fn(U, i32, i32, *mut u16) -> bool>,
    // ---- language, view, accidents ----
    pub drives_on_right: Option<unsafe extern "C" fn(U) -> bool>,
    pub view_cell_rect: Option<unsafe extern "C" fn(U, *mut fc4_cell_rect) -> bool>,
    pub create_accident: Option<unsafe extern "C" fn(U, i32, i32, i64)>,
    // ---- ABI 61 ----
    /// `CreateLineItem(line, false)`, `SetName(0x6A3FF01C, name)`, Income
    /// when `income`; with `only_if_missing` nothing when the line exists
    /// (traffic-simulator §9.1 TG7).
    pub create_line_item: Option<unsafe extern "C" fn(U, u64, u32, u32, bool, bool)>,
}

const _: () = {
    let p = cfg!(target_pointer_width = "32");
    assert!(std::mem::size_of::<fc4_traffic_trip>() == if p { 36 } else { 56 });
    assert!(std::mem::size_of::<fc4_traffic_city>() == 20);
    assert!(std::mem::size_of::<fc4_traffic_post>() == 16);
    assert!(std::mem::size_of::<fc4_traffic_switch_query>() == 20);
    assert!(std::mem::size_of::<fc4_traffic_ferry_route>() == 40);
    assert!(std::mem::size_of::<fc4_traffic_state>() == 360);
    assert!(std::mem::size_of::<fc4_traffic_host>() == if p { 408 } else { 816 });
};

// ---- host adapter -----------------------------------------------------------------------------------

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.c.$f { Some(f) => unsafe { f($s.c.user $(, $arg)*) }, None => $default }
    };
}

fn opt(t: u64) -> Option<u64> {
    (t != 0).then_some(t)
}

unsafe extern "C" fn push_u32(ctx: *mut c_void, v: u32) {
    if let Some(out) = (ctx as *mut Vec<u32>).as_mut() {
        out.push(v);
    }
}

unsafe extern "C" fn push_exit(ctx: *mut c_void, exit: u8, vertex: u64) {
    if let Some(out) = (ctx as *mut Vec<(u8, u64)>).as_mut() {
        out.push((exit, vertex));
    }
}

unsafe extern "C" fn push_city(ctx: *mut c_void, c: *const fc4_traffic_city) {
    if let (Some(out), Some(c)) = ((ctx as *mut Vec<NeighbourCity>).as_mut(), c.as_ref()) {
        out.push(NeighbourCity {
            position: (c.x, c.z),
            population: c.population,
        });
    }
}

unsafe extern "C" fn push_population(ctx: *mut c_void, map: u32, dev: u32, count: u32) {
    if let Some(out) = (ctx as *mut Vec<Vec<(u32, u32)>>).as_mut() {
        let i = map as usize;
        // [choice] A host that skips a map index gets empty maps in between;
        // more than 64 maps are cut (a lot holds 0 or 1, layouts.md).
        if i >= 64 {
            return;
        }
        if out.len() <= i {
            out.resize(i + 1, Vec::new());
        }
        out[i].push((dev, count));
    }
}

unsafe fn bytes_of(p: *const u8, n: usize) -> Vec<u8> {
    if p.is_null() || n == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(p, n).to_vec()
    }
}

unsafe extern "C" fn push_trip(ctx: *mut c_void, t: *const fc4_traffic_trip) {
    if let (Some(out), Some(t)) = ((ctx as *mut Vec<TripData>).as_mut(), t.as_ref()) {
        out.push(TripData {
            route: bytes_of(t.route, t.route_len),
            return_route: bytes_of(t.return_route, t.return_len),
            count: t.count,
            trip_type: t.trip_type,
            strategy: t.strategy,
            flags: t.flags,
            status: t.status,
            dest_x: t.dest_x,
            dest_z: t.dest_z,
            trip_length: t.trip_length,
            travel_types_used: t.travel_types_used,
        });
    }
}

/// A holder property in the game's native layout, arrays included (the
/// trip capacity `0x8A0B4100` is a u16 or u32 array; the shared
/// `native_variant` delivers scalars only, its type check sees the array
/// flag). `None` for empty, pointer and object values.
fn holder_value(v: &Variant) -> Option<NativeVariant> {
    let Variant::Array { elem, items } = v else {
        return native_variant(v);
    };
    if !(mem::BOOL..=mem::RZ_CHAR).contains(elem) {
        return None;
    }
    let mut bytes = Vec::new();
    for e in items {
        bytes.extend(native_variant(&Variant::Scalar(*e))?.bytes);
    }
    Some(NativeVariant {
        mem_type: *elem,
        is_array: true,
        count: items.len(),
        bytes,
    })
}

fn source_raw(s: Source) -> (u32, u64) {
    match s {
        Source::Lot(t) => (FC4_TRAFFIC_SOURCE_LOT, t),
        Source::Connection(t) => (FC4_TRAFFIC_SOURCE_CONNECTION, t),
    }
}

/// Counters the host adapter keeps (diagnostics, the month line).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Counters {
    pub posted: u32,
    pub accidents_posted: u32,
    pub native_updates: u32,
    pub trip_reads: u32,
    pub trip_writes: u32,
}

pub(crate) struct CHost {
    c: fc4_traffic_host,
    counters: Cell<Counters>,
    /// Inside an outermost call ([`Handle::call`]): the lot list read
    /// first in it is handed out again (`lots_shared`). The plugin keeps
    /// one lot walk per outermost call, so the list cannot differ.
    in_call: Cell<bool>,
    lots_cache: RefCell<Option<Rc<[u64]>>>,
}

impl CHost {
    pub(crate) fn new(c: fc4_traffic_host) -> CHost {
        CHost {
            c,
            counters: Cell::new(Counters::default()),
            in_call: Cell::new(false),
            lots_cache: RefCell::new(None),
        }
    }

    /// The outermost call starts or ends: the lot list is read afresh.
    fn set_in_call(&self, on: bool) {
        self.in_call.set(on);
        self.lots_cache.borrow_mut().take();
    }

    fn note(&self, f: impl FnOnce(&mut Counters)) {
        let mut c = self.counters.get();
        f(&mut c);
        self.counters.set(c);
    }

    fn rect_of(&self, f: impl FnOnce(*mut fc4_cell_rect)) -> CellRect {
        let mut r = fc4_cell_rect::default();
        f(&mut r);
        r.into()
    }

    fn cell_of(&self, f: impl FnOnce(*mut i32, *mut i32)) -> (i32, i32) {
        let (mut x, mut z) = (0i32, 0i32);
        f(&mut x, &mut z);
        (x, z)
    }

    fn tokens_in(
        &self,
        f: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, fc4_token_callback, U)>,
        rect: CellRect,
    ) -> Vec<u64> {
        let Some(f) = f else { return Vec::new() };
        let r = fc4_cell_rect::from(rect);
        // SAFETY: valid callback; `r` outlives the call.
        collect_tokens(|cb, ctx| unsafe { f(self.c.user, &r, cb, ctx) })
            .into_iter()
            .filter(|&t| t != 0)
            .collect()
    }

    fn u32s(&self, f: impl FnOnce(fc4_traffic_u32_callback, U)) -> Vec<u32> {
        let mut v: Vec<u32> = Vec::new();
        f(Some(push_u32), &mut v as *mut Vec<u32> as U);
        v
    }
}

impl GridProvider<u8> for CHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        // `Shutdown` then `Release` when the core drops it (§1 Shutdown).
        create_grid::<u8>(&self.c.grids, instance, tract_size, true)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u8>>> {
        resolve_grid::<u8>(&self.c.grids, r, true)
    }
}

impl GridProvider<u32> for CHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u32>>> {
        create_grid::<u32>(&self.c.grids, instance, tract_size, true)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u32>>> {
        resolve_grid::<u32>(&self.c.grids, r, true)
    }
}

impl GridProvider<f32> for CHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<f32>>> {
        create_grid::<f32>(&self.c.grids, instance, tract_size, true)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<f32>>> {
        resolve_grid::<f32>(&self.c.grids, r, true)
    }
}

impl NetworkHost for CHost {
    fn cell_counts(&self) -> (i32, i32) {
        self.cell_of(|x, z| cb!(self, cell_counts, (), x, z))
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        self.cell_of(|cx, cz| cb!(self, position_to_cell, (), x, z, cx, cz))
    }
    fn network_occupants_in_rect(&self, rect: CellRect) -> Vec<u64> {
        self.tokens_in(self.c.network_occupants_in_rect, rect)
    }
    fn subway_occupants_in_rect(&self, rect: CellRect) -> Vec<u64> {
        self.tokens_in(self.c.subway_occupants_in_rect, rect)
    }
    fn prop_occupants_in_rect(&self, rect: CellRect) -> Vec<u64> {
        self.tokens_in(self.c.prop_occupants_in_rect, rect)
    }
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        let mut p = [0f32; 3];
        cb!(self, occupant_position, (), occ, p.as_mut_ptr());
        p
    }
    fn occupant_bounding_cells(&self, occ: u64) -> CellRect {
        self.rect_of(|r| cb!(self, occupant_bounding_cells, (), occ, r))
    }
    fn prop_type(&self, occ: u64) -> u32 {
        cb!(self, prop_type, 0, occ)
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant> {
        let f = self.c.occupant_property?;
        // SAFETY: valid callback; the scratch holder outlives the call.
        read_property(id, |p| unsafe { f(self.c.user, occ, id, p) })
    }
    fn has_network_flag(&self, occ: u64, flag: u32) -> bool {
        cb!(self, has_network_flag, false, occ, flag)
    }
    fn has_any_network_flag(&self, occ: u64, mask: u32) -> bool {
        cb!(self, has_any_network_flag, false, occ, mask)
    }
    fn is_of_type(&self, occ: u64, network_type: u32) -> bool {
        cb!(self, is_of_type, false, occ, network_type)
    }
    fn path_connection(&self, occ: u64, path_type: u32, entry: u32, exit: u32) -> bool {
        cb!(self, path_connection, false, occ, path_type, entry, exit)
    }
    fn path_info(&self, occ: u64) -> Option<PathInfo> {
        opt(cb!(self, path_info, 0, occ))
    }
    fn path_info_turn_around(&self, pi: PathInfo, path_type: u32, entry: u32) -> bool {
        cb!(self, path_info_turn_around, false, pi, path_type, entry)
    }
    fn path_info_exits_from_entry(&self, pi: PathInfo, path_type: u32, entry: u32) -> Vec<(u8, u64)> {
        let mut v: Vec<(u8, u64)> = Vec::new();
        cb!(
            self,
            path_info_exits_from_entry,
            (),
            pi,
            path_type,
            entry,
            Some(push_exit),
            &mut v as *mut Vec<(u8, u64)> as U
        );
        v
    }
    fn path_info_entries_near(&self, pi: PathInfo, path_type: u32, vertex: u64, exit: u32, radius: f32) -> Vec<u32> {
        self.u32s(|f, ctx| {
            cb!(
                self,
                path_info_entries_near,
                (),
                pi,
                path_type,
                vertex,
                exit,
                radius,
                f,
                ctx
            )
        })
    }
    fn is_tunnel(&self, occ: u64) -> bool {
        cb!(self, is_tunnel, false, occ)
    }
    fn tunnel_paired_portal(&self, occ: u64) -> Option<u64> {
        opt(cb!(self, tunnel_paired_portal, 0, occ))
    }
    fn lot_at(&self, x: i32, z: i32, anchor_only: bool) -> Option<u64> {
        opt(cb!(self, lot_at, 0, x, z, anchor_only))
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        self.rect_of(|r| cb!(self, lot_bounding_rect, (), lot, r))
    }
    fn lot_location(&self, lot: u64) -> (i32, i32) {
        self.cell_of(|x, z| cb!(self, lot_location, (), lot, x, z))
    }
    fn lot_facing(&self, lot: u64) -> u32 {
        cb!(self, lot_facing, 0, lot)
    }
    fn lot_building(&self, lot: u64) -> Option<u64> {
        opt(cb!(self, lot_building, 0, lot))
    }
    fn lot_populations(&self, lot: u64) -> Vec<Vec<(u32, u32)>> {
        let mut v: Vec<Vec<(u32, u32)>> = Vec::new();
        cb!(
            self,
            lot_populations,
            (),
            lot,
            Some(push_population),
            &mut v as *mut Vec<Vec<(u32, u32)>> as U
        );
        v
    }
    fn lot_holder(&self, lot: u64) -> u64 {
        cb!(self, lot_holder, 0, lot)
    }
    fn holder_property(&self, holder: u64, id: u32) -> Option<Variant> {
        if holder == 0 {
            return None;
        }
        let f = self.c.holder_property?;
        // SAFETY: valid callback; the scratch holder outlives the call.
        read_property(id, |p| unsafe { f(self.c.user, holder, id, p) })
    }
    fn set_holder_property(&self, holder: u64, id: u32, value: Variant) {
        if holder == 0 {
            return;
        }
        let Some(nv) = holder_value(&value) else { return };
        cb!(
            self,
            set_holder_property,
            (),
            holder,
            id,
            nv.mem_type,
            nv.is_array,
            nv.bytes.as_ptr() as *const c_void,
            nv.count
        )
    }
    fn remove_holder_property(&self, holder: u64, id: u32) -> bool {
        if holder == 0 {
            return false;
        }
        cb!(self, remove_holder_property, false, holder, id)
    }
    /// Never called: the simulator answers it (`NetAdapter`).
    fn max_trip_capacity(&self, _holder: u64, _k: u32) -> u32 {
        0
    }
    /// Never called: the simulator keeps the pools.
    fn neighbor_pool_holder(&self, _x: i32, _z: i32) -> Option<u64> {
        None
    }
    fn neighbor_pool_holder_in(&self, _s: &mut NetworkState, _x: i32, _z: i32) -> Option<u64> {
        None
    }
    /// Never called: the simulator keeps the damage sets.
    fn is_road_damaged(&self, _x: i32, _z: i32) -> bool {
        false
    }
    fn cell_under_water(&self, x: i32, z: i32) -> bool {
        cb!(self, cell_under_water, false, x, z)
    }
    fn is_navigable(&self, x: i32, z: i32) -> bool {
        cb!(self, is_navigable, false, x, z)
    }
    fn water_depth(&self, x: i32, z: i32) -> f32 {
        cb!(self, water_depth, 0.0, x, z)
    }
    fn body_of_water(&self, x: i32, z: i32) -> Option<u16> {
        let mut v = 0u16;
        cb!(self, body_of_water, false, x, z, &mut v).then_some(v)
    }
    fn connections_of_type(&self, connection_type: u32) -> Vec<u64> {
        let Some(f) = self.c.connections_of_type else {
            return Vec::new();
        };
        // SAFETY: valid callback.
        collect_tokens(|cb, ctx| unsafe { f(self.c.user, connection_type, cb, ctx) })
            .into_iter()
            .filter(|&t| t != 0)
            .collect()
    }
    fn all_connections(&self) -> Vec<u64> {
        let Some(f) = self.c.all_connections else {
            return Vec::new();
        };
        // SAFETY: valid callback.
        collect_tokens(|cb, ctx| unsafe { f(self.c.user, cb, ctx) })
            .into_iter()
            .filter(|&t| t != 0)
            .collect()
    }
    fn connection_cell(&self, conn: u64) -> (i32, i32) {
        self.cell_of(|x, z| cb!(self, connection_cell, (), conn, x, z))
    }
    fn post(&self, msg_type: u32) {
        TrafficSimHost::post_message(self, SimMessage::of_type(msg_type));
    }
    fn add_ref(&self, token: u64) {
        if token != 0 {
            cb!(self, add_ref, (), token)
        }
    }
    fn release(&self, token: u64) {
        if token != 0 {
            cb!(self, release, (), token)
        }
    }
}

impl TrafficSimHost for CHost {
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        cb!(self, add_agent, (), level, name.as_ptr(), name.len(), flags)
    }
    fn subscribe(&self, msg_type: u32) {
        cb!(self, subscribe, (), msg_type)
    }
    fn post_message(&self, msg: SimMessage) {
        self.note(|c| {
            c.posted = c.posted.wrapping_add(1);
            if msg.msg_type == ts::msg::TRAFFIC_ACCIDENT {
                c.accidents_posted = c.accidents_posted.wrapping_add(1);
            }
        });
        let m = fc4_traffic_post {
            msg_type: msg.msg_type,
            data1: msg.data1,
            data2: msg.data2,
            data3: msg.data3,
        };
        cb!(self, post_message, (), &m)
    }
    /// The token is saved as a reference to the object (`WriteGZSerializable`
    /// on its serializable interface, done by the host's stream).
    fn object_ref(&self, token: u64) -> RefValue {
        if token == 0 {
            RefValue::Null
        } else {
            RefValue::Host { token, class_id: 0 }
        }
    }
    /// The stream already asked `iid` (`get_ref_as`); the token carries the
    /// reference the core keeps (see [`TIn`]).
    fn resolve_ref(&self, r: &RefValue, _iid: u32) -> Option<u64> {
        match *r {
            RefValue::Host { token, .. } if token != 0 => Some(token),
            _ => None,
        }
    }
    fn cell_width(&self) -> f32 {
        cb!(self, cell_width, 16.0)
    }
    fn tuning_exemplar(&self) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, tuning, std::ptr::null_mut());
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }
    fn sim_day_of_month(&self) -> u32 {
        cb!(self, sim_day_of_month, 0)
    }
    fn sim_date_number(&self) -> u32 {
        cb!(self, sim_date_number, 0)
    }
    fn sim_clock_ms(&self) -> u64 {
        cb!(self, sim_clock_ms, 0)
    }
    fn create_department(&self, id: u32, max_funding: f32) -> Option<u64> {
        opt(cb!(self, create_department, 0, id, max_funding))
    }
    fn release_department(&self, department: u64) {
        if department != 0 {
            cb!(self, release_department, (), department)
        }
    }
    fn create_line_item(&self, department: u64, item: ts::dept::LineItem, only_if_missing: bool) {
        if department != 0 {
            cb!(
                self,
                create_line_item,
                (),
                department,
                item.line,
                item.name,
                item.income,
                only_if_missing
            )
        }
    }
    fn department_funding(&self, department: u64) -> f32 {
        cb!(self, department_funding, 0.0, department)
    }
    fn set_line_item_income(&self, department: u64, item: u32, income: i64) {
        cb!(self, set_line_item_income, (), department, item, income)
    }
    fn update_network_costs(&self, department: u64, mass_transit: bool, cells: &[u32; 13], tile_costs: &[f32; 13]) {
        cb!(
            self,
            update_network_costs,
            (),
            department,
            mass_transit,
            cells.as_ptr(),
            tile_costs.as_ptr()
        )
    }
    fn activate_budget_items(&self, building: u64, lot: Option<u64>) -> i64 {
        cb!(self, activate_budget_items, 0, building, lot.unwrap_or(0))
    }
    fn deactivate_budget_items(&self, holder: u64) -> i64 {
        cb!(self, deactivate_budget_items, 0, holder)
    }
    fn budget_item_for_purpose(&self, building: u64, purpose: u32) -> Option<(u32, i64)> {
        let (mut dept, mut cost) = (0u32, 0i64);
        cb!(
            self,
            budget_item_for_purpose,
            false,
            building,
            purpose,
            &mut dept,
            &mut cost
        )
        .then_some((dept, cost))
    }
    fn department_budget(&self, department_id: u32) -> Option<u64> {
        opt(cb!(self, department_budget, 0, department_id))
    }
    fn record_history(&self, series: u32, value: u32) {
        cb!(self, record_history, (), series, value)
    }
    fn lots(&self) -> Vec<u64> {
        let Some(f) = self.c.lots else { return Vec::new() };
        // SAFETY: valid callback.
        collect_tokens(|cb, ctx| unsafe { f(self.c.user, cb, ctx) })
            .into_iter()
            .filter(|&t| t != 0)
            .collect()
    }
    fn lots_shared(&self) -> Rc<[u64]> {
        if !self.in_call.get() {
            return self.lots().into();
        }
        if let Some(l) = self.lots_cache.borrow().as_ref() {
            return l.clone();
        }
        let l: Rc<[u64]> = self.lots().into();
        *self.lots_cache.borrow_mut() = Some(l.clone());
        l
    }
    fn as_lot(&self, object: u64) -> Option<u64> {
        if object == 0 {
            return None;
        }
        opt(cb!(self, as_lot, 0, object))
    }
    fn lot_zone(&self, lot: u64) -> u32 {
        cb!(self, lot_zone, 0, lot)
    }
    fn lot_is_livable(&self, lot: u64) -> bool {
        cb!(self, lot_is_livable, false, lot)
    }
    fn lot_has_road_access(&self, lot: u64) -> bool {
        cb!(self, lot_has_road_access, false, lot)
    }
    fn lot_link(&self, lot: u64) -> Option<u64> {
        opt(cb!(self, lot_link, 0, lot))
    }
    fn lot_travel_population(&self, lot: u64, developer_type: u32) -> u32 {
        cb!(self, lot_travel_population, 0, lot, developer_type)
    }
    fn lot_strategy_byte(&self, lot: u64) -> u8 {
        cb!(self, lot_strategy_byte, 0, lot)
    }
    fn clear_lot_flag(&self, lot: u64, flag: u32) {
        cb!(self, clear_lot_flag, (), lot, flag)
    }
    fn native_update_traffic_volume(&self, source: Source) -> bool {
        let (kind, token) = source_raw(source);
        let ran = cb!(self, update_traffic_volume, false, kind, token);
        if ran {
            self.note(|c| c.native_updates = c.native_updates.wrapping_add(1));
        }
        ran
    }
    fn trip_list(&self, source: Source) -> Vec<TripData> {
        let (kind, token) = source_raw(source);
        self.note(|c| c.trip_reads = c.trip_reads.wrapping_add(1));
        let mut v: Vec<TripData> = Vec::new();
        cb!(
            self,
            trip_list,
            false,
            kind,
            token,
            Some(push_trip),
            &mut v as *mut Vec<TripData> as U
        );
        v
    }
    fn set_trip_list(&self, source: Source, trips: Vec<TripData>) {
        let (kind, token) = source_raw(source);
        self.note(|c| c.trip_writes = c.trip_writes.wrapping_add(1));
        let raw: Vec<fc4_traffic_trip> = trips
            .iter()
            .map(|t| fc4_traffic_trip {
                count: t.count,
                trip_type: t.trip_type,
                strategy: t.strategy,
                flags: t.flags,
                status: t.status,
                dest_x: t.dest_x,
                dest_z: t.dest_z,
                trip_length: t.trip_length,
                travel_types_used: t.travel_types_used,
                route: t.route.as_ptr(),
                route_len: t.route.len(),
                return_route: t.return_route.as_ptr(),
                return_len: t.return_route.len(),
            })
            .collect();
        cb!(self, set_trip_list, (), kind, token, raw.as_ptr(), raw.len())
    }
    fn connection_holder(&self, connection: u64) -> u64 {
        cb!(self, connection_holder, 0, connection)
    }
    fn take_connection_volume_dirty(&self, connection: u64) -> bool {
        cb!(self, take_connection_volume_dirty, false, connection)
    }
    fn find_connection(&self, connection_type: u32, x: i32, z: i32) -> Option<u64> {
        opt(cb!(self, find_connection, 0, connection_type, x, z))
    }
    fn connection_type(&self, connection: u64) -> u32 {
        cb!(self, connection_type, 0, connection)
    }
    fn connection_on_other_side(&self, connection: u64) -> Option<u64> {
        opt(cb!(self, connection_on_other_side, 0, connection))
    }
    fn established_city_position(&self, connection: u64) -> Option<(i32, i32)> {
        let (mut x, mut z) = (0i32, 0i32);
        cb!(self, established_city_position, false, connection, &mut x, &mut z).then_some((x, z))
    }
    fn connected_cities(&self) -> Vec<NeighbourCity> {
        let mut v: Vec<NeighbourCity> = Vec::new();
        cb!(
            self,
            connected_cities,
            (),
            Some(push_city),
            &mut v as *mut Vec<NeighbourCity> as U
        );
        v
    }
    fn regional_demand(&self, w: u32) -> Option<f32> {
        let mut v = 0f32;
        cb!(self, regional_demand, false, w, &mut v).then_some(v)
    }
    fn workforce_percent(&self, x: i32, z: i32) -> f32 {
        cb!(self, workforce_percent, 0.0, x, z)
    }
    fn developer_population(&self, developer_type: u32, x: i32, z: i32) -> i16 {
        cb!(self, developer_population, 0, developer_type, x, z)
    }
    fn is_ordinance_on(&self, id: u32) -> bool {
        cb!(self, is_ordinance_on, false, id)
    }
    fn ordinances_on(&self) -> Vec<u32> {
        self.u32s(|f, ctx| cb!(self, ordinances_on, (), f, ctx))
    }
    fn ordinance_property(&self, id: u32, prop: u32) -> Option<Variant> {
        let f = self.c.ordinance_property?;
        // SAFETY: valid callback; the scratch holder outlives the call.
        read_property(prop, |p| unsafe { f(self.c.user, id, prop, p) })
    }
    fn as_network_occupant(&self, object: u64) -> Option<u64> {
        if object == 0 {
            return None;
        }
        opt(cb!(self, as_network_occupant, 0, object))
    }
    fn network_occupant_cell(&self, occ: u64) -> (i32, i32) {
        self.cell_of(|x, z| cb!(self, network_occupant_cell, (), occ, x, z))
    }
    fn occupants_in_cells(&self, rect: CellRect) -> Vec<u64> {
        self.tokens_in(self.c.occupants_in_cells, rect)
    }
    fn occupant_filter_includes(&self, filter: u64, occupant: u64) -> bool {
        cb!(self, occupant_filter_includes, false, filter, occupant)
    }
    fn cell_has_network_occupant(&self, x: i32, z: i32) -> bool {
        cb!(self, cell_has_network_occupant, false, x, z)
    }
    fn drives_on_right(&self) -> bool {
        cb!(self, drives_on_right, true)
    }
    fn view_cell_rect(&self) -> Option<CellRect> {
        let mut r = fc4_cell_rect::default();
        cb!(self, view_cell_rect, false, &mut r).then(|| r.into())
    }
    fn create_accident(&self, x: i32, z: i32, duration_ms: i64) {
        cb!(self, create_accident, (), x, z, duration_ms)
    }
}

// ---- the save stream --------------------------------------------------------------------------------

/// The record reader (§6.1): delegates to [`CIn`], except references.
/// Host objects come with the one reference the stream hands over, which
/// the core keeps (grids, switch buildings, departments). FreeCity4's own
/// objects (`fc4_ref::core_handle` set: our departments) come without a
/// reference, so one is taken for the core.
struct TIn<'a> {
    inner: CIn<'a>,
    host: &'a CHost,
}

impl TIn<'_> {
    fn read(&mut self, iid: u32) -> Option<RefValue> {
        let f = self.inner.stream.get_ref?;
        let mut out = fc4_ref::default();
        // SAFETY: valid callback; `out` outlives the call.
        if !unsafe { f(self.inner.stream.user, iid, &mut out) } {
            return None;
        }
        if out.token == 0 {
            return Some(RefValue::Null);
        }
        if !out.core_handle.is_null() {
            NetworkHost::add_ref(self.host, out.token);
        }
        Some(RefValue::Host {
            token: out.token,
            class_id: out.class_id,
        })
    }
}

impl InStream for TIn<'_> {
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
        self.read(0)
    }
    fn get_ref_as(&mut self, iid: u32) -> Option<RefValue> {
        self.read(iid)
    }
    fn skip(&mut self, n: usize) -> bool {
        self.inner.skip(n)
    }
    fn error(&self) -> i32 {
        self.inner.error()
    }
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn fc4_core::stream::SaveObject>> {
    None
}

// ---- handles ----------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_traffic`).
#[repr(C)]
pub struct fc4_traffic {
    _private: [u8; 0],
}

/// Opaque pathfinder handle (fc4.h `fc4_traffic_pf`): one `cISC4PathFinder`
/// handed out through slot 6. It belongs to the simulator that created it
/// (pathfinder.md §1); every call names both.
#[repr(C)]
pub struct fc4_traffic_pf {
    _private: [u8; 0],
}

/// What re-entrant calls are answered from (see the module notes).
#[derive(Clone, Default)]
struct Snapshot {
    state: fc4_traffic_state,
    /// Slot 26 for wealth 0…4 (4 and above: zeros, §4.1.2 TS28).
    pct: [[f32; 3]; 4],
}

struct Handle {
    host: Rc<CHost>,
    sim: Rc<TrafficSimulator>,
    depth: Cell<u32>,
    reentrant: Cell<u32>,
    queued: Cell<u32>,
    pending: RefCell<Vec<Message>>,
    snap: RefCell<Snapshot>,
}

impl Handle {
    fn new(host: Rc<CHost>, seed: u32) -> Handle {
        let sim = TrafficSimulator::new(host.clone() as Rc<dyn TrafficSimHost>, seed);
        let h = Handle {
            host,
            sim,
            depth: Cell::new(0),
            reentrant: Cell::new(0),
            queued: Cell::new(0),
            pending: RefCell::new(Vec::new()),
            snap: RefCell::new(Snapshot::default()),
        };
        h.refresh();
        h
    }

    fn reentrant_now(&self) -> bool {
        self.depth.get() > 0
    }

    /// Runs `f` with the call depth counted; the outermost call refreshes the
    /// snapshot before it starts and runs the queued messages after it.
    fn call<R>(&self, f: impl FnOnce(&Handle) -> R) -> R {
        let outer = self.depth.get() == 0;
        if outer {
            self.host.set_in_call(true);
            self.refresh();
        } else {
            self.reentrant.set(self.reentrant.get().wrapping_add(1));
        }
        self.depth.set(self.depth.get() + 1);
        let r = f(self);
        if outer {
            // Messages that arrived during the call, in arrival order (each
            // may queue more).
            loop {
                let next = {
                    let mut p = self.pending.borrow_mut();
                    if p.is_empty() {
                        None
                    } else {
                        Some(p.remove(0))
                    }
                };
                let Some(m) = next else { break };
                self.sim.do_message(&m);
            }
        }
        self.depth.set(self.depth.get() - 1);
        if outer {
            self.host.set_in_call(false);
            self.refresh();
        }
        r
    }

    /// The state as the getters see it (a re-entrant call: the snapshot).
    fn state(&self) -> fc4_traffic_state {
        if self.reentrant_now() {
            return self.snap.borrow().state;
        }
        self.compute_state()
    }

    fn refresh(&self) {
        let st = self.compute_state();
        let mut s = self.snap.borrow_mut();
        s.state = st;
        s.pct = st.pct;
    }

    fn compute_state(&self) -> fc4_traffic_state {
        let sim = &self.sim;
        let c = self.host.counters.get();
        let mut out = fc4_traffic_state {
            struct_size: std::mem::size_of::<fc4_traffic_state>() as u32,
            reentrant_calls: self.reentrant.get(),
            queued_messages: self.queued.get(),
            posted: c.posted,
            accidents_posted: c.accidents_posted,
            native_updates: c.native_updates,
            trip_reads: c.trip_reads,
            trip_writes: c.trip_writes,
            ..Default::default()
        };
        let got = sim.read_state(|s| {
            Self::fill_state(&mut out, s);
            true
        });
        if !got {
            // An operation is running: keep the last snapshot's values.
            let last = self.snap.borrow().state;
            return fc4_traffic_state {
                reentrant_calls: out.reentrant_calls,
                queued_messages: out.queued_messages,
                posted: out.posted,
                accidents_posted: out.accidents_posted,
                native_updates: out.native_updates,
                trip_reads: out.trip_reads,
                trip_writes: out.trip_writes,
                ..last
            };
        }
        {
            let d = sim.damage();
            out.road_damaged = d.road.damaged_count;
            out.rail_damaged = d.rail.damaged_count;
        }
        let n = sim.network().state();
        out.edge_records = n.edge_density.len() as u32;
        out.switches = n.switches.len() as u32;
        out.ferry_terminals = n.ferries.len() as u32;
        out
    }

    fn fill_state(out: &mut fc4_traffic_state, s: &ts::State) {
        out.initialized = u8::from(s.initialised);
        out.loaded = u8::from(s.loaded);
        out.cycle_state = s.cycle_state;
        out.strike_state = s.strike_state;
        out.day_index = s.day_index;
        out.days = s.days;
        out.total = s.total;
        out.processed = s.processed;
        out.pass_total = s.pass_total;
        out.to_reroute = s.to_reroute;
        out.to_search = s.to_search;
        out.last_day = s.last_day;
        out.pass_counters = s.pass_counters;
        out.units_per_slice = s.tuning.traffic_units_per_slice;
        out.trip_scale = s.tuning.commute_max_time;
        out.trip_scale_for_display = s.tuning.trip_display_multiplier;
        out.last_monthly_income = s.last_monthly_income;
        out.freight_scaling = s.tuning.freight_scaling;
        out.transit_income = s.transit_income;
        out.toll_income = s.toll_income;
        out.avg_car_commute = s.avg_car_commute;
        for w in 0..4 {
            out.pct[w] = s.tuning.strategy_percentages(w as u32);
        }
        out.attempted = s.attempted;
        out.routed = s.routed;
        out.job_capacity = s.job_capacity;
        out.free_dest = std::array::from_fn(|k| s.free_dest[k].get());
        out.type_counts = s.type_count_published;
        out.freight_byte = s.stats_last.byte;
        out.accident_cells = s.accident_check.len() as u32;
        out.potholes = u32::from(s.stats_last.potholes);
        out.air = grid_token(&s.air);
        out.commercial = grid_token(&s.commercial);
        out.congestion = grid_token(&s.congestion);
        out.trip_length = grid_token(&s.trip_length);
        out.roads = s.roads.unwrap_or(0);
        out.mass_transit = s.mass_transit.unwrap_or(0);
    }
}

/// Runs `f` on the handle; the fallback for NULL or a panic.
fn with_handle<R>(p: *const fc4_traffic, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_traffic*.
    let Some(h) = (unsafe { (p as *const Handle).as_ref() }) else {
        return fallback;
    };
    guard(fallback, || f(h))
}

/// Runs `f` on the core with the call depth counted.
fn sim<R>(p: *const fc4_traffic, fallback: R, f: impl FnOnce(&TrafficSimulator) -> R) -> R {
    with_handle(p, fallback, |h| h.call(|h| f(&h.sim)))
}

/// A getter: re-entrantly from the snapshot (`snap`), else from the core.
fn getter<R>(
    p: *const fc4_traffic,
    fallback: R,
    snap: impl FnOnce(&Snapshot) -> R,
    f: impl FnOnce(&TrafficSimulator) -> R,
) -> R {
    with_handle(p, fallback, |h| {
        if h.reentrant_now() {
            h.reentrant.set(h.reentrant.get().wrapping_add(1));
            return snap(&h.snap.borrow());
        }
        h.call(|h| f(&h.sim))
    })
}

pub(crate) fn host_ok(host: *const fc4_traffic_host) -> Option<fc4_traffic_host> {
    // SAFETY: documented precondition: NULL or a valid host table.
    let h = unsafe { host.as_ref() }?;
    let ok = (h.struct_size as usize) >= std::mem::size_of::<fc4_traffic_host>() && h.grids.complete();
    ok.then_some(*h)
}

/// `QueryInterface(iid)` (§1): `1`, `0x68027F37`, `0x452294AA`,
/// `0xE4FDA3D4`.
#[no_mangle]
pub extern "C" fn fc4_traffic_query_interface(iid: u32) -> bool {
    matches!(
        iid,
        ts::IID_UNKNOWN | ts::IID_TRAFFIC_SIMULATOR | ts::IID_MESSAGE_TARGET2 | ts::IID_SERIALIZABLE
    )
}

/// The class ID (`0x6990C1AA`).
#[no_mangle]
pub extern "C" fn fc4_traffic_class_id() -> u32 {
    CLSID_TRAFFIC_SIMULATOR
}

/// `GetSimulatorType` (slot 5): `0x88027F0D`.
#[no_mangle]
pub extern "C" fn fc4_traffic_simulator_type() -> u32 {
    ts::SIMULATOR_TYPE
}

/// The default `trafficUnitsPerSlice` (§3.4 [choice]).
#[no_mangle]
pub extern "C" fn fc4_traffic_default_units_per_slice() -> u32 {
    ts::tuning::DEFAULT_UNITS_PER_SLICE
}

/// Creates the core objects (the constructor's state, §1): the generator is
/// seeded with `derive_seed(session_seed, 0x6990C1AA)`. The host table is
/// copied. NULL when the table is incomplete.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_create(host: *const fc4_traffic_host, session_seed: u32) -> *mut fc4_traffic {
    guard(std::ptr::null_mut(), || {
        let Some(h) = host_ok(host) else {
            return std::ptr::null_mut();
        };
        Box::into_raw(Box::new(Handle::new(Rc::new(CHost::new(h)), session_seed))) as *mut fc4_traffic
    })
}

/// Destroys the core objects. Grids, departments and switch buildings still
/// held are released (a loaded object never initialised, or not shut down).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_destroy(p: *mut fc4_traffic) {
    if p.is_null() {
        return;
    }
    guard((), || {
        let h = Box::from_raw(p as *mut Handle);
        // The departments and switch buildings are host references the core
        // only gives back in Shutdown and Read.
        if h.sim.read_state(|s| s.initialised) {
            h.sim.shutdown();
        } else {
            let depts = h.sim.update_state(|s| vec![s.roads.take(), s.mass_transit.take()]);
            for d in depts.into_iter().flatten() {
                h.host.release_department(d);
            }
        }
        let buildings: Vec<u64> = h
            .sim
            .network()
            .state()
            .switches
            .iter()
            .map(|(_, s)| s.building)
            .filter(|&b| b != 0)
            .collect();
        for b in buildings {
            NetworkHost::release(&*h.host, b);
        }
        drop(h);
    });
}

/// `Init` (slot 3, §1); `units_per_slice` 0 keeps the default (§3.4). Returns
/// true (also when already initialised).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_init(p: *mut fc4_traffic, units_per_slice: u32) -> bool {
    sim(p, false, |s| {
        let r = s.init();
        if units_per_slice != 0 {
            s.set_traffic_units_per_slice(units_per_slice);
        }
        r
    })
}

/// `Shutdown` (slot 4): releases the pathfinders, grids and departments.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_shutdown(p: *mut fc4_traffic) -> bool {
    sim(p, false, TrafficSimulator::shutdown)
}

/// `DoMessage` (§3.1): the handler's answer (true except the day message's
/// "not finished"). `object` is void1 / the `IGZUnknown` (any interface of
/// it; 0 = none). A message delivered during another call is queued and run
/// when that call ends (answer true).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_do_message(
    p: *mut fc4_traffic,
    msg_type: u32,
    data1: u32,
    data2: u32,
    data3: u32,
    object: u64,
) -> bool {
    let m = Message {
        msg_type,
        data1,
        data2,
        data3,
        object,
    };
    with_handle(p, true, |h| {
        if h.reentrant_now() {
            h.queued.set(h.queued.get().wrapping_add(1));
            h.pending.borrow_mut().push(m);
            return true;
        }
        h.call(|h| h.sim.do_message(&m))
    })
}

/// `Write` (§6.1). Grid, building and department references go out as
/// `fc4_ref`s with the token.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_write(p: *const fc4_traffic, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    sim(p, false, |s| s.write(&mut COut(o)))
}

/// `Read` (§6.1). Runs before `Init`: nothing is sized from the city here.
/// A failed Read changes nothing (§6.1 [choice]).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_read(p: *mut fc4_traffic, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_handle(p, false, |h| {
        let host = h.host.clone();
        h.call(|h| {
            h.sim.read(&mut TIn {
                inner: CIn {
                    stream: i,
                    decode: no_core_objects,
                },
                host: &host,
            })
        })
    })
}

/// The state (diagnostics, the log lines, the plugin's snapshot). During a
/// call (re-entrantly) the snapshot taken at its start.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_get_state(p: *const fc4_traffic, out: *mut fc4_traffic_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_handle(p, false, |h| {
        *o = h.state();
        true
    })
}

/// The token of map `which` (`FC4_TRAFFIC_MAP_*`; slots 8, 9, 10, 13; no
/// AddRef), 0 = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_map(p: *const fc4_traffic, which: u32) -> u64 {
    let pick = move |st: &fc4_traffic_state| match which {
        FC4_TRAFFIC_MAP_AIR => st.air,
        FC4_TRAFFIC_MAP_COMMERCIAL => st.commercial,
        FC4_TRAFFIC_MAP_CONGESTION => st.congestion,
        FC4_TRAFFIC_MAP_TRIP_LENGTH => st.trip_length,
        _ => 0,
    };
    getter(
        p,
        0,
        |s| pick(&s.state),
        |s| match which {
            FC4_TRAFFIC_MAP_AIR => grid_token(&s.air_polluting_traffic_map()),
            FC4_TRAFFIC_MAP_COMMERCIAL => grid_token(&s.commercial_traffic_map()),
            FC4_TRAFFIC_MAP_CONGESTION => grid_token(&s.congestion_map()),
            FC4_TRAFFIC_MAP_TRIP_LENGTH => grid_token(&s.trip_length_map()),
            _ => 0,
        },
    )
}

/// The mean of map `which` over its tracts (the month line), 0 without a
/// map or tracts.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_map_average(p: *const fc4_traffic, which: u32) -> f32 {
    sim(p, 0.0, |s| {
        let g = match which {
            FC4_TRAFFIC_MAP_AIR => s.air_polluting_traffic_map(),
            FC4_TRAFFIC_MAP_COMMERCIAL => s.commercial_traffic_map(),
            FC4_TRAFFIC_MAP_CONGESTION => s.congestion_map(),
            FC4_TRAFFIC_MAP_TRIP_LENGTH => s.trip_length_map(),
            _ => None,
        };
        let Some(g) = g else { return 0.0 };
        let (cx, cz) = (g.count_x(), g.count_z());
        if cx <= 0 || cz <= 0 {
            return 0.0;
        }
        let mut sum = 0f64;
        for x in 0..cx {
            for z in 0..cz {
                sum += f64::from(g.get(x, z));
            }
        }
        (sum / (f64::from(cx) * f64::from(cz))) as f32
    })
}

/// `GetBackgroundTraffic(x, z)` (slot 11, §4.9). Re-entrantly 0.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_background_traffic(p: *const fc4_traffic, x: i32, z: i32) -> f32 {
    getter(p, 0.0, |_| 0.0, |s| s.background_traffic(x, z))
}

/// `GetTrafficEdgeDensity(x, z, t, evening)` (slot 12, §4.8): the record's
/// 33 u16 into `out` (`last[16]`, `usage`, `current[16]`); false (and `out`
/// zeroed) for the shared zero record.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_edge_density(
    p: *const fc4_traffic,
    x: u8,
    z: u8,
    travel_type: u32,
    evening: bool,
    out: *mut u16,
) -> bool {
    let rec = sim(p, None, |s| {
        let key = NetworkState::edge_key(i32::from(x), i32::from(z), travel_type, evening);
        s.network().state().edge_density.get(key).copied()
    });
    if !out.is_null() {
        let o = std::slice::from_raw_parts_mut(out, 33);
        match rec {
            Some(r) => {
                o[..16].copy_from_slice(&r.last);
                o[16] = r.usage;
                o[17..].copy_from_slice(&r.current);
            }
            None => o.fill(0),
        }
    }
    rec.is_some()
}

/// Slots 14, 15, 18, 27 (`which` 0…3): trip scale, trip scale for display,
/// last monthly income, freight scaling factor.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_scalar(p: *const fc4_traffic, which: u32) -> f32 {
    let pick = move |st: &fc4_traffic_state| match which {
        0 => st.trip_scale,
        1 => st.trip_scale_for_display,
        2 => st.last_monthly_income,
        3 => st.freight_scaling,
        _ => 0.0,
    };
    getter(
        p,
        0.0,
        |s| pick(&s.state),
        |s| match which {
            0 => s.trip_scale(),
            1 => s.trip_scale_for_display(),
            2 => s.last_monthly_income(),
            3 => s.freight_scaling_factor(),
            _ => 0.0,
        },
    )
}

/// `IsRoadDamaged(x, z)` (slot 16).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_is_road_damaged(p: *const fc4_traffic, x: i32, z: i32) -> bool {
    with_handle(p, false, |h| h.sim.is_road_damaged(x, z))
}

/// `CheckRailAccident(x, z)` (slot 17): one draw for a damaged rail cell.
/// Re-entrantly false (no draw).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_check_rail_accident(p: *mut fc4_traffic, x: i32, z: i32) -> bool {
    getter(p, false, |_| false, |s| s.check_rail_accident(x, z))
}

/// `SetMaxTripCapacity(holder, k, value)` (slot 19), through the element
/// store (§4.2): the host writes the holder's `0x8A0B4100` (in place when
/// the type and length allow, foundations §10.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_set_max_trip_capacity(p: *mut fc4_traffic, holder: u64, k: u32, value: u32) {
    with_handle(p, (), |h| h.sim.set_max_trip_capacity(holder, k, value))
}

/// `GetMaxTripCapacity(holder, k)` (slot 20).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_max_trip_capacity(p: *const fc4_traffic, holder: u64, k: u32) -> u32 {
    with_handle(p, 0, |h| h.sim.max_trip_capacity(holder, k))
}

/// `GetTrafficArrived(holder, k)` (slot 21).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_traffic_arrived(p: *const fc4_traffic, holder: u64, k: u32) -> u32 {
    with_handle(p, 0, |h| h.sim.traffic_arrived(holder, k))
}

/// `GetDesiredLotInsertionPoint` (slot 22): whether the cycle is in state 1
/// (the original then returns the lot manager's safe lot iterator;
/// traffic-network.md §4.14).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_desired_lot_insertion_point(p: *const fc4_traffic) -> bool {
    getter(
        p,
        false,
        |s| s.state.cycle_state == ts::cycle_state::LOT_SWEEP,
        TrafficSimulator::desired_lot_insertion_point,
    )
}

/// `GetCapacity(n, x, z)` (slot 23).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_capacity(p: *const fc4_traffic, n: u32, x: i32, z: i32) -> f32 {
    with_handle(p, 0.0, |h| h.sim.capacity(n, x, z))
}

/// `GetTravelTimeRatio(x, z, t)` (slot 24).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_travel_time_ratio(p: *const fc4_traffic, x: i32, z: i32, t: u32) -> f32 {
    getter(p, 0.0, |_| 0.0, |s| s.travel_time_ratio(x, z, t))
}

/// `GetConnectionCount(n, x, z)` (slot 25).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_connection_count(p: *const fc4_traffic, n: u32, x: i32, z: i32) -> u32 {
    with_handle(p, 0, |h| h.sim.connection_count(n, x, z))
}

/// `GetTravelStrategyPercentages(w)` (slot 26, §4.1.2) into `out[3]`;
/// re-entrantly (a lot's own `UpdateTrafficVolume`) from the snapshot.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_travel_strategy_percentages(p: *const fc4_traffic, wealth: u32, out: *mut f32) {
    let v = getter(
        p,
        [0.0f32; 3],
        |s| s.pct.get(wealth as usize).copied().unwrap_or([0.0; 3]),
        |s| s.travel_strategy_percentages(wealth),
    );
    if !out.is_null() {
        std::slice::from_raw_parts_mut(out, 3).copy_from_slice(&v);
    }
}

/// `GetTransitSwitches` (slot 28): the switch buildings in table order
/// (borrowed tokens; the host takes one reference each for the list).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_transit_switches(
    p: *const fc4_traffic,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) -> usize {
    let v = with_handle(p, Vec::new(), |h| h.sim.transit_switches());
    let Some(f) = cb else { return v.len() };
    for &t in &v {
        f(ctx, t);
    }
    v.len()
}

/// `GetFerryRouteBetweenTiles(x1, z1, x2, z2)` (slot 29): the route into
/// `cb`; false (no call) when there is none.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_ferry_route_between_tiles(
    p: *const fc4_traffic,
    x1: i32,
    z1: i32,
    x2: i32,
    z2: i32,
    cb: fc4_traffic_bytes_callback,
    ctx: *mut c_void,
) -> bool {
    let Some(r) = with_handle(p, None, |h| h.sim.ferry_route_between_tiles(x1, z1, x2, z2)) else {
        return false;
    };
    if let Some(f) = cb {
        f(ctx, r.as_ptr(), r.len());
    }
    true
}

/// `GetAllFerryRoutes` (slot 30): each route into `cb`; returns the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_all_ferry_routes(
    p: *const fc4_traffic,
    cb: fc4_traffic_bytes_callback,
    ctx: *mut c_void,
) -> usize {
    let v = with_handle(p, Vec::new(), |h| h.sim.all_ferry_routes());
    if let Some(f) = cb {
        for r in &v {
            f(ctx, r.as_ptr(), r.len());
        }
    }
    v.len()
}

fn ferry_raw(i: &FerryRouteInfo) -> fc4_traffic_ferry_route {
    fc4_traffic_ferry_route {
        lot_a: i.lot_a,
        lot_b: i.lot_b,
        ferry_type: i.ferry_type,
        counter1: i.counter1,
        counter2: i.counter2,
        neighbour: i.neighbour,
        neighbour_x: i.neighbour_x,
        neighbour_z: i.neighbour_z,
        ..Default::default()
    }
}

/// `GetFerryRoutesInUse` (slot 31): each record into `cb`; returns the
/// count.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_ferry_routes_in_use(
    p: *const fc4_traffic,
    cb: fc4_traffic_ferry_callback,
    ctx: *mut c_void,
) -> usize {
    let v = with_handle(p, Vec::new(), |h| h.sim.ferry_routes_in_use());
    if let Some(f) = cb {
        for i in &v {
            let raw = ferry_raw(i);
            f(
                ctx,
                &raw,
                i.route_ab.as_ptr(),
                i.route_ab.len(),
                i.route_ba.as_ptr(),
                i.route_ba.len(),
            );
        }
    }
    v.len()
}

/// `GetFerryTerminalCount(type)` (slot 32).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_ferry_terminal_count(p: *const fc4_traffic, ferry_type: u32) -> u32 {
    with_handle(p, 0, |h| h.sim.ferry_terminal_count(ferry_type))
}

/// `GetWaterRoute(x1, z1, x2, z2, out)` (slot 33): the route into `cb` when
/// found (the original clears `out` first either way).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_water_route(
    p: *mut fc4_traffic,
    x1: i32,
    z1: i32,
    x2: i32,
    z2: i32,
    cb: fc4_traffic_bytes_callback,
    ctx: *mut c_void,
) -> bool {
    let (ok, route) = sim(p, (false, Vec::new()), |s| {
        let mut r = Vec::new();
        let ok = s.water_route(x1, z1, x2, z2, &mut r);
        (ok, r)
    });
    if let Some(f) = cb {
        f(ctx, route.as_ptr(), route.len());
    }
    ok
}

/// The size of the slot-34 statistics block (§4.9; see
/// [`fc4_traffic_stats_block`]).
pub const FC4_TRAFFIC_STATS_BLOCK: usize = 0x23C;

fn put_row(b: &mut [u8], at: usize, r: &ts::StatRow) {
    b[at..at + 4].copy_from_slice(&r.sum.to_le_bytes());
    b[at + 4..at + 8].copy_from_slice(&r.congested.to_le_bytes());
    b[at + 8..at + 12].copy_from_slice(&r.max_x.to_le_bytes());
    b[at + 12..at + 16].copy_from_slice(&r.max_z.to_le_bytes());
    b[at + 16..at + 20].copy_from_slice(&r.max.to_le_bytes());
}

/// The published statistics as the game's block (slot 34 `GetTrafficStats`
/// returns a pointer to it, §4.9): 14 rows of `(f32 average, u32 congested,
/// s32 max x, s32 max z, f32 max)` (0x14 bytes each: rows 0–12, then the
/// bridge row), u16 × 13 records, u16 bridge records, u16 potholes at
/// `0x134` (§3.6). [question] Past `0x136` the block is laid out by the
/// record's type sequence (§6.1 row 16) with natural alignment: 2 pad bytes,
/// 12 × `(u16, pad 2, f32, s32, s32, f32)` at `0x138`, then f32 `0x228`, f32
/// `0x22C`, bool `0x230`, f32 `0x234`, u8 `0x238`; the spec gives no
/// offsets past the pothole count.
pub fn stats_block(s: &StatsSet) -> [u8; FC4_TRAFFIC_STATS_BLOCK] {
    let mut b = [0u8; FC4_TRAFFIC_STATS_BLOCK];
    for (i, r) in s.rows.iter().enumerate() {
        put_row(&mut b, i * 0x14, r);
    }
    put_row(&mut b, 13 * 0x14, &s.bridge);
    for (i, r) in s.records.iter().enumerate() {
        b[0x118 + 2 * i..0x11A + 2 * i].copy_from_slice(&r.to_le_bytes());
    }
    b[0x132..0x134].copy_from_slice(&s.bridge_records.to_le_bytes());
    b[0x134..0x136].copy_from_slice(&s.potholes.to_le_bytes());
    for (i, w) in s.switch_stats.iter().enumerate() {
        let at = 0x138 + i * 0x14;
        b[at..at + 2].copy_from_slice(&w.a.to_le_bytes());
        b[at + 4..at + 8].copy_from_slice(&w.b.to_le_bytes());
        b[at + 8..at + 12].copy_from_slice(&w.c.to_le_bytes());
        b[at + 12..at + 16].copy_from_slice(&w.d.to_le_bytes());
        b[at + 16..at + 20].copy_from_slice(&w.e.to_le_bytes());
    }
    b[0x228..0x22C].copy_from_slice(&s.f1.to_le_bytes());
    b[0x22C..0x230].copy_from_slice(&s.f2.to_le_bytes());
    b[0x230] = u8::from(s.flag);
    b[0x234..0x238].copy_from_slice(&s.f3.to_le_bytes());
    b[0x238] = s.byte;
    b
}

/// `GetTrafficStats` (slot 34): the published block (see [`stats_block`])
/// into `out` (`len` bytes, at most [`FC4_TRAFFIC_STATS_BLOCK`] written);
/// returns the block size. Re-entrantly nothing is written (0).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_stats_block(p: *const fc4_traffic, out: *mut u8, len: usize) -> usize {
    let block = getter(p, None, |_| None, |s| Some(stats_block(&s.traffic_stats())));
    let Some(b) = block else { return 0 };
    if !out.is_null() {
        let n = len.min(b.len());
        std::slice::from_raw_parts_mut(out, n).copy_from_slice(&b[..n]);
    }
    FC4_TRAFFIC_STATS_BLOCK
}

/// `GetTransitSwitchQueryData(x, z, out)` (slot 35, §4.16): false leaves
/// `out` untouched.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_transit_switch_query_data(
    p: *mut fc4_traffic,
    x: i32,
    z: i32,
    out: *mut fc4_traffic_switch_query,
) -> bool {
    let Some(d) = sim(p, None, |s| s.transit_switch_query_data(x, z)) else {
        return false;
    };
    if let Some(o) = out.as_mut() {
        *o = fc4_traffic_switch_query {
            available: d.available,
            max_capacity: d.max_capacity,
            load_fraction: d.load_fraction,
            usage: d.usage,
            cost: d.cost,
        };
    }
    true
}

/// `AreLotsConnected(a, b)` (slot 36).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_are_lots_connected(p: *const fc4_traffic, a: u64, b: u64) -> bool {
    sim(p, false, |s| s.are_lots_connected(a, b))
}

/// `GetConnectedOccupantCount(lot, developerType)` (slot 37).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_connected_occupant_count(p: *const fc4_traffic, lot: u64, dev: u32) -> u32 {
    sim(p, 0, |s| s.connected_occupant_count(lot, dev))
}

/// `GetConnectedDestinationCount(lot, k)` (slot 38).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_connected_destination_count(p: *const fc4_traffic, lot: u64, k: i32) -> u32 {
    sim(p, 0, |s| s.connected_destination_count(lot, k))
}

unsafe fn emit_u32s(v: &[u32], cb: fc4_traffic_u32_callback, ctx: *mut c_void) -> usize {
    if let Some(f) = cb {
        for &x in v {
            f(ctx, x);
        }
    }
    v.len()
}

/// `GetSubnetworksForLot(lot, out)` (slot 39): each subnetwork into `cb`;
/// the result through `ok` (may be NULL); returns the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_subnetworks_for_lot(
    p: *const fc4_traffic,
    lot: u64,
    cb: fc4_traffic_u32_callback,
    ctx: *mut c_void,
    ok: *mut bool,
) -> usize {
    let (v, r) = sim(p, (Vec::new(), false), |s| s.subnetworks_for_lot(lot));
    if let Some(o) = ok.as_mut() {
        *o = r;
    }
    emit_u32s(&v, cb, ctx)
}

/// `GetSubnetworksInRectangle(rect, out)` (slot 40).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_subnetworks_in_rectangle(
    p: *const fc4_traffic,
    rect: *const fc4_cell_rect,
    cb: fc4_traffic_u32_callback,
    ctx: *mut c_void,
) -> usize {
    let Some(r) = rect.as_ref() else { return 0 };
    let r = CellRect::from(*r);
    let v = sim(p, Vec::new(), |s| s.subnetworks_in_rectangle(r));
    emit_u32s(&v, cb, ctx)
}

/// `GetSubnetworksInRegion(region, out)` (slot 41); a region larger than
/// the zones' limit (fc4.h `fc4_zone_region`) gives nothing.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_subnetworks_in_region(
    p: *const fc4_traffic,
    region: *const fc4_zone_region,
    cb: fc4_traffic_u32_callback,
    ctx: *mut c_void,
) -> usize {
    let Some(r) = region.as_ref().and_then(|r| region_in(r)) else {
        return 0;
    };
    let v = sim(p, Vec::new(), |s| s.subnetworks_in_region(&r));
    emit_u32s(&v, cb, ctx)
}

/// `GetOccupantCountForAllSubnetworks(developerType, out)` (slot 42).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_occupant_count_for_all_subnetworks(
    p: *const fc4_traffic,
    dev: u32,
    cb: fc4_traffic_u32_callback,
    ctx: *mut c_void,
) -> usize {
    let v = sim(p, Vec::new(), |s| s.occupant_count_for_all_subnetworks(dev));
    emit_u32s(&v, cb, ctx)
}

// ---- pathfinders (pathfinder.md) --------------------------------------------------------------------

struct PfHandle {
    pf: RefCell<Pathfinder>,
}

fn with_pf<R>(pf: *const fc4_traffic_pf, fallback: R, f: impl FnOnce(&mut Pathfinder) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_traffic_pf*.
    let Some(h) = (unsafe { (pf as *const PfHandle).as_ref() }) else {
        return fallback;
    };
    let Ok(mut b) = h.pf.try_borrow_mut() else {
        return fallback;
    };
    guard(fallback, || f(&mut b))
}

/// A pathfinder and the simulator that owns it, with the owner's call depth
/// counted.
fn with_owned<R>(
    p: *const fc4_traffic,
    pf: *const fc4_traffic_pf,
    fallback: R,
    f: impl FnOnce(&TrafficSimulator, &mut Pathfinder) -> R,
) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_traffic_pf*.
    let Some(ph) = (unsafe { (pf as *const PfHandle).as_ref() }) else {
        return fallback;
    };
    with_handle(p, None, |h| {
        let Ok(mut b) = ph.pf.try_borrow_mut() else {
            return None;
        };
        Some(h.call(|h| f(&h.sim, &mut b)))
    })
    .unwrap_or(fallback)
}

/// `CreatePathFinder` (slot 6, pathfinder.md §1): a new pathfinder,
/// initialised (`Init` with the city size), sharing the work-check statics.
/// NULL for a NULL simulator.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_create(p: *mut fc4_traffic) -> *mut fc4_traffic_pf {
    let Some(pf) = sim(p, None, |s| Some(s.create_path_finder())) else {
        return std::ptr::null_mut();
    };
    Box::into_raw(Box::new(PfHandle { pf: RefCell::new(pf) })) as *mut fc4_traffic_pf
}

#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_destroy(pf: *mut fc4_traffic_pf) {
    if pf.is_null() {
        return;
    }
    guard((), || drop(Box::from_raw(pf as *mut PfHandle)));
}

/// `Init` (slot 3): false when already initialised.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_init(p: *mut fc4_traffic, pf: *mut fc4_traffic_pf) -> bool {
    with_owned(p, pf, false, |s, pf| s.init_path_finder(pf))
}

/// `Shutdown` (slot 4): false when not initialised.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_shutdown(pf: *mut fc4_traffic_pf) -> bool {
    with_pf(pf, false, Pathfinder::shutdown)
}

/// Whether the pathfinder is initialised.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_is_initialised(pf: *const fc4_traffic_pf) -> bool {
    with_pf(pf, false, |pf| pf.is_initialised())
}

/// The setters (slots 5–22). `slot` names the pathfinder slot; `a`, `b`
/// its arguments (slot 13: mask, also-starting; slots 5, 6, 9–11, 14, 18,
/// 21: the one argument). Slots 7, 12, 15, 22 take none. Returns false for
/// a slot this function does not set (8, 16, 17, 19, 20 have their own).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_set(pf: *mut fc4_traffic_pf, slot: u32, a: u32, b: u32) -> bool {
    with_pf(pf, false, |pf| {
        match slot {
            5 => pf.set_flag(a),
            6 => pf.clear_flag(a),
            7 => pf.clear_all_flags(),
            9 => pf.set_trip_destination_filter(a),
            10 => pf.set_destination_flag(a as u8),
            11 => pf.clear_destination_flag(a as u8),
            12 => pf.clear_all_destination_flags(),
            13 => pf.set_travel_type_flag(a as u16, b != 0),
            14 => pf.clear_travel_type_flag(a as u16),
            15 => pf.clear_all_travel_type_flags(),
            18 => pf.set_traffic_density(a),
            21 => pf.set_max_work_time(a),
            22 => pf.reset(),
            _ => return false,
        }
        true
    })
}

/// `SetRouteInfo` (slot 8): `on` keeps a route of the pathfinder's own
/// (read with [`fc4_traffic_pf_route`]); off = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_set_route_info(pf: *mut fc4_traffic_pf, on: bool) {
    with_pf(pf, (), |pf| pf.set_route_info(on.then(std::collections::VecDeque::new)))
}

/// `SetStartingCostArray` (slot 16): the nine costs (copied at the call,
/// pathfinder.md §2 reads them at every FindPath: the plugin passes the
/// caller's array again before each FindPath), NULL = the simulator's
/// default.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_set_starting_cost_array(pf: *mut fc4_traffic_pf, costs: *const f32) {
    let c = (!costs.is_null()).then(|| {
        let mut a = [0f32; 9];
        a.copy_from_slice(std::slice::from_raw_parts(costs, 9));
        a
    });
    with_pf(pf, (), |pf| pf.set_starting_cost_array(c))
}

/// `SetMaxTripLength` (slot 17).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_set_max_trip_length(pf: *mut fc4_traffic_pf, v: f32) {
    with_pf(pf, (), |pf| pf.set_max_trip_length(v))
}

/// `SetStartRect` / `SetEndRect` (slots 19, 20; `end` picks 20).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_set_rect(pf: *mut fc4_traffic_pf, end: bool, rect: *const fc4_cell_rect) {
    let Some(r) = rect.as_ref() else { return };
    let r = CellRect::from(*r);
    with_pf(pf, (), |pf| if end { pf.set_end_rect(r) } else { pf.set_start_rect(r) })
}

/// `FindPath` (slot 23): 1 found, 0 not found, 2 suspended.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_find_path(p: *mut fc4_traffic, pf: *mut fc4_traffic_pf) -> u32 {
    with_owned(p, pf, 0, |s, pf| s.find_path(pf))
}

/// `GetTrafficArrived` (slot 24).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_traffic_arrived(pf: *const fc4_traffic_pf) -> u32 {
    with_pf(pf, 0, |pf| pf.get_traffic_arrived())
}

/// `GetTripLength` (slot 25).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_trip_length(pf: *const fc4_traffic_pf) -> f32 {
    with_pf(pf, 0.0, |pf| pf.get_trip_length())
}

/// `GetTripLengthScaledForDisplay` (slot 26): `r24(slot 15 × tripLength)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_trip_length_for_display(
    p: *const fc4_traffic,
    pf: *const fc4_traffic_pf,
) -> f32 {
    let scale = fc4_traffic_scalar(p, 1);
    let len = with_pf(pf, 0.0, |pf| pf.get_trip_length());
    fc4_core::x87::r24(f64::from(scale) * f64::from(len)) as f32
}

/// `GetEndCell(x, z)` (slot 27).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_end_cell(pf: *const fc4_traffic_pf, x: *mut i32, z: *mut i32) {
    let (ex, ez) = with_pf(pf, (0, 0), |pf| pf.get_end_cell());
    if let Some(o) = x.as_mut() {
        *o = ex;
    }
    if let Some(o) = z.as_mut() {
        *o = ez;
    }
}

/// `GetLastTravelType` (slot 28).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_last_travel_type(pf: *const fc4_traffic_pf) -> u8 {
    with_pf(pf, 0, |pf| pf.get_last_travel_type())
}

/// `ApplyTraffic(style)` (slot 29, pathfinder.md §4.17).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_apply_traffic(p: *mut fc4_traffic, pf: *mut fc4_traffic_pf, style: u32) {
    with_owned(p, pf, (), |s, pf| s.apply_path_finder_traffic(pf, style))
}

/// `WasTripReversible` (slot 30).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_was_trip_reversible(p: *const fc4_traffic, pf: *const fc4_traffic_pf) -> bool {
    with_owned(p, pf, true, |s, pf| s.path_finder_was_reversible(pf))
}

/// `ReleasePathNodes` (slot 31).
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_release_path_nodes(pf: *mut fc4_traffic_pf) {
    with_pf(pf, (), |pf| pf.release_path_nodes())
}

/// `GetCompressedRoute(out, mask)` (slot 32, pathfinder.md §4.15): the
/// route bytes into `cb` (always called, empty without a result); returns
/// the mask.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_compressed_route(
    p: *const fc4_traffic,
    pf: *const fc4_traffic_pf,
    cb: fc4_traffic_bytes_callback,
    ctx: *mut c_void,
) -> u32 {
    let (mask, bytes) = with_owned(p, pf, (0, Vec::new()), |s, pf| {
        let mut out = Vec::new();
        let m = s.path_finder_compressed_route(pf, &mut out);
        (m, out)
    });
    if let Some(f) = cb {
        f(ctx, bytes.as_ptr(), bytes.len());
    }
    mask
}

/// The route of the last successful `FindPath` (slot 8's deque, root →
/// result, pathfinder.md §4.13 step 5): one `(x, z, travelType)` per node
/// into `cb`, as three bytes; returns the count. 0 without a route.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_route(
    pf: *const fc4_traffic_pf,
    cb: fc4_traffic_bytes_callback,
    ctx: *mut c_void,
) -> usize {
    let route: Vec<RouteData> = with_pf(pf, Vec::new(), |pf| {
        pf.route().map(|r| r.iter().copied().collect()).unwrap_or_default()
    });
    if let Some(f) = cb {
        for r in &route {
            let b = [r.x, r.z, r.travel_type];
            f(ctx, b.as_ptr(), 3);
        }
    }
    route.len()
}

/// The configuration the pathfinder holds (diagnostics and tests).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_traffic_pf_state {
    pub struct_size: u32,
    pub initialised: u8,
    pub suspended: u8,
    pub has_route: u8,
    pub destination_flags: u8,
    pub flags: u32,
    pub filter: u32,
    pub allowed: u16,
    pub starting: u16,
    pub density: u32,
    pub max_work_time: u32,
    pub trip_length: f32,
    pub start: fc4_cell_rect,
    pub end: fc4_cell_rect,
    pub city_x: i32,
    pub city_z: i32,
}

const _: () = assert!(std::mem::size_of::<fc4_traffic_pf_state>() == 72);

#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_pf_get_state(pf: *const fc4_traffic_pf, out: *mut fc4_traffic_pf_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_pf(pf, false, |pf| {
        let (cx, cz) = pf.city_size();
        *o = fc4_traffic_pf_state {
            struct_size: std::mem::size_of::<fc4_traffic_pf_state>() as u32,
            initialised: u8::from(pf.is_initialised()),
            suspended: u8::from(pf.is_suspended()),
            has_route: u8::from(pf.route().is_some()),
            destination_flags: pf.destination_flags(),
            flags: pf.flags(),
            filter: pf.trip_destination_filter(),
            allowed: pf.allowed_travel_types(),
            starting: pf.starting_travel_types(),
            density: pf.traffic_density(),
            max_work_time: pf.max_work_time(),
            trip_length: pf.get_trip_length(),
            start: pf.start_rect().into(),
            end: pf.end_rect().into(),
            city_x: cx,
            city_z: cz,
        };
        true
    })
}

/// `SetupPathFinderForLot(pf, lot)` (slot 7, §4.4): false for lot 0 or a
/// lot with no trips.
#[no_mangle]
pub unsafe extern "C" fn fc4_traffic_setup_path_finder_for_lot(
    p: *mut fc4_traffic,
    pf: *mut fc4_traffic_pf,
    lot: u64,
) -> bool {
    with_owned(p, pf, false, |s, pf| s.setup_path_finder_for_lot(pf, lot))
}

/// The core behind a handle (tests: network fixtures).
///
/// # Safety
/// `p` must be a live `fc4_traffic*`.
#[cfg(test)]
pub(crate) unsafe fn core_of<'a>(p: *const fc4_traffic) -> &'a TrafficSimulator {
    &(*(p as *const Handle)).sim
}
