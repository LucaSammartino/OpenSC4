// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! `cSC4MySimDispatch` (class `0xCBC14674`, read as IID `0xEB8B6F9A`), the
//! per-MySim trip controller. Behavior source: mysims.md §§4.6.1–4.6.2, 4.7,
//! 4.11, 6.2.1 and 7.2.1. Transport handoffs remain foreign (MS18).
//!
//! - `FindNewJob` → `SetCurrentCommuteDestination(default loc, notify)` →
//!   `validateCommuteLocation` (§4.7.1): one filter-0 search on the
//!   dispatch's **long-lived** pathfinder (flags `0x11`, destination flags by
//!   wealth OR-accumulated *[quirk]*, density 1, home lot rect, travel
//!   `0x1CF` allowed and starting, one `FindPath`), then the end cell is
//!   resolved to a city lot's building or a neighbour city.
//! - `ChangeJob(b)` sets the commute destination to building `b`; the
//!   result is the dispatch's (§4.7.1).
//! - `CheckCommute` belongs to the MySim (§4.7), not this owner.
//! - `CheckFlags(0x100)` is the tracking flag of §4.1 step 3.
//! - `Init` only takes and drops the clock reference (clock24.md).
//! - the `0x2BA06B99` notification sub-IDs the MySim reacts to (§3.3):
//!   `0x4BB2D020`/`0x4BB2CFC0` trip states, `0xEBA06BF4` job found,
//!   `0xEBA06C12` arrival, `0xEBA06BD4`/`0x4BB71A7E` state transitions.
//!
//! Requests never manufacture arrivals. Unsettled transport branches return
//! [`TripOutcome::UnsupportedMs18`]; the walking character and traffic stay foreign.

use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;
use std::sync::RwLock;

use crate::pathfinder::{filter, flag, result, CellRect, Pathfinder, PathfinderHost, RouteData, SharedWorkCheck};
use crate::traffic_network::NetworkModel;

use super::{JobDestination, DISPATCH_CLSID, DISPATCH_IID};

mod callbacks;
mod ready;
mod record;
pub mod scripted;
mod transport;
mod trip;
pub use ready::model_selection;
pub use record::DispatchRecordRefs;
pub use transport::{BoardingAutomaton, TransitStop, TransportTail};
pub use trip::{DispatchTuning, TripOutcome, TripSearch, TripServices};

#[cfg(test)]
mod trip_tests;

/// The commuter travel-type mask of the job search (§4.7.1 step 8: Walk,
/// Car, Bus, Passenger train, Subway, Elevated train, Monorail; allowed
/// **and** starting).
pub const COMMUTER_TRAVEL_MASK: u16 = 0x1CF;
/// §4.7.1 step 3: `SetFlag(1)` then `SetFlag(0x10)`.
pub const JOB_SEARCH_FLAGS: [u32; 2] = [flag::NO_BUDGET, flag::NO_FERRIES];
/// §4.7.1 step 6.
pub const JOB_SEARCH_DENSITY: u32 = 1;
/// Dispatch flag `0x80`: a valid default destination (home) is set
/// (`SetDefaultDestination`, §4.7.1 step 2).
pub const FLAG_HAS_HOME: u32 = 0x80;
/// The dispatch flags `resetPath` clears (§4.7.1 step 0).
pub const RESET_PATH_FLAGS: u32 = 0x800 | 0x1000 | 0x4000;
/// The dispatch state (`+0x40`) at which an invalid location is stored
/// unvalidated (§4.7.1 step 2).
pub const STATE_STORE_UNVALIDATED: i32 = 2;

// `CheckCommute` uses a separate, fresh pathfinder owned by the MySim
// (§4.7/4.7.2); it must not reuse this dispatch's persistent job search.

/// `SetDestinationFlag` value by the MySim's `GetWealth()` (§4.7.1 step 5):
/// 1 → `0x1`, 2 → `0x2`, 3 → `0x4`, anything else → `0x1`.
pub fn destination_flag_for_wealth(wealth: u32) -> u8 {
    match wealth {
        2 => 0x2,
        3 => 0x4,
        _ => 0x1,
    }
}

/// The neighbour connection types `IsValid` tries for a location outside
/// the city, in order, stopping at the first non-empty list (§4.7.1, table
/// `0x00B07DD0`): Highway 3, GroundHighway 12, Avenue 8, Road 2, DirtRoad
/// 10, Rail 1, LightRail 7, Monorail 11, Subway 6. Ferry, power and pipe
/// connections are not in the table.
pub const NEIGHBOR_VALID_CONNECTION_TYPES: [u32; 9] = [3, 12, 8, 2, 10, 1, 7, 11, 6];

/// The world coordinate of a default location (§4.7.1: −16.0 = 16 × −1).
pub const DEFAULT_WORLD: f32 = -16.0;

/// A dispatch location (`cLocation`, 0x28 bytes; home `+0x84`, commute
/// destination `+0xD4`), field by field per the §4.7.1 contract.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DispatchLocation {
    /// `+0x00`: the referenced occupant (`None` = null).
    pub occupant: Option<u64>,
    /// `+0x04`, `+0x08`: world x, z.
    pub world: (f32, f32),
    /// `+0x0C`, `+0x10`: cell x, z.
    pub cell: (i32, i32),
    /// `+0x14`, `+0x18`: the neighbour city's region position.
    pub neighbor_city: (i32, i32),
    /// `+0x1C`: the "in this city" flag.
    pub in_city: bool,
    /// `+0x1D`: "by commute".
    pub by_commute: bool,
    /// `+0x20`, `+0x24`: duration and deadline, signed bits (§4.6.1).
    pub reserved: (u32, u32),
}

impl Default for DispatchLocation {
    /// The default location (ctor `0x00431320`): no occupant, world −16.0,
    /// cells −1, neighbour city −1, in this city, not by commute.
    fn default() -> Self {
        DispatchLocation {
            occupant: None,
            world: (DEFAULT_WORLD, DEFAULT_WORLD),
            cell: (-1, -1),
            neighbor_city: (-1, -1),
            in_city: true,
            by_commute: false,
            reserved: (0, 0),
        }
    }
}

/// MS13 world-to-cell bit arithmetic under 24-bit precision (§4.7.1).
pub fn world_to_cell(world: f32) -> i32 {
    // MS13: each operation rounds to a 24-bit significand. Bit arithmetic
    // also defines NaN/infinity and large-coordinate results, unlike casts.
    let q = world * 0.0625;
    let d = q - 0.5;
    let s = d + 12_582_912.0;
    let n = s.to_bits().wrapping_add(0xB4C0_0000) as i32;
    let next = n.wrapping_add(1);
    if q == next as f32 {
        next
    } else {
        n
    }
}

impl DispatchLocation {
    /// `SetByOccupant(o)` (`0x00431430`): every field reset to the default
    /// (byCommute 0, neighbour city −1), in this city, `o` held; with an
    /// occupant, world := its `GetPosition()` x, z and cell := `floor(world
    /// / 16)` with the MS13 rounding quirks. `o` is the occupant and its position (the caller reads it
    /// through cISC4Occupant slot 8); `None` = null (cells stay −1).
    pub fn set_by_occupant(&mut self, o: Option<(u64, (f32, f32))>) {
        *self = DispatchLocation::default();
        self.in_city = true;
        if let Some((occ, (x, z))) = o {
            self.occupant = Some(occ);
            self.world = (x, z);
            self.cell = (world_to_cell(x), world_to_cell(z));
        }
    }

    /// A fresh location after `SetByOccupant(o)`.
    pub fn by_occupant(o: Option<(u64, (f32, f32))>) -> Self {
        let mut l = DispatchLocation::default();
        l.set_by_occupant(o);
        l
    }

    /// `SetCellPosition(x, z)` (§4.7.1, MS13): cells and world fields;
    /// shift as wrapping int32 before converting to float32.
    pub fn set_cell_position(&mut self, x: i32, z: i32) {
        self.cell = (x, z);
        self.world = (x.wrapping_shl(4) as f32, z.wrapping_shl(4) as f32);
    }

    /// A default location with the in-city flag clear and a cell (test and
    /// conformance setups of a cell home).
    pub fn by_cell(x: i32, z: i32) -> Self {
        let mut l = DispatchLocation {
            in_city: false,
            ..DispatchLocation::default()
        };
        l.set_cell_position(x, z);
        l
    }

    /// `IsValid` (`0x00434F90`, answers MS11 a). In this city: both cells
    /// within the city's cell counts (the occupant is not consulted).
    /// Outside: the region has a city at the neighbour-city position and
    /// the neighbours simulator lists a connection to it of one of
    /// [`NEIGHBOR_VALID_CONNECTION_TYPES`] (first hit stops).
    pub fn is_valid(&self, world: &dyn JobSearchWorld) -> bool {
        if self.in_city {
            let (cx, cz) = world.cell_counts();
            (0..cx).contains(&self.cell.0) && (0..cz).contains(&self.cell.1)
        } else {
            let Some(city) = world.region_city(self.neighbor_city.0, self.neighbor_city.1) else {
                return false;
            };
            NEIGHBOR_VALID_CONNECTION_TYPES
                .iter()
                .any(|&t| world.connections_of_type_to_city(t, city) > 0)
        }
    }
}

/// The lot at an end cell (§4.7.1 after `FindPath`): `GetLot(x, z, false)`
/// and the lot's `GetBuilding()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LotAt {
    NoLot,
    NoBuilding,
    Building(u64),
}

/// The pathfinder slots the job search calls (§4.7.1), so a recording
/// pathfinder can stand in for the core one (conformance).
pub trait JobPathfinder {
    /// `Reset` (slot 22).
    fn reset(&mut self);
    /// `ClearAllTravelTypeFlags` (slot 15).
    fn clear_all_travel_type_flags(&mut self);
    /// `SetRouteInfo(route deque)` (slot 8).
    fn set_route_info(&mut self, route: VecDeque<RouteData>);
    /// `SetFlag` (slot 5), OR.
    fn set_flag(&mut self, flags: u32);
    /// `SetTripDestinationFilter` (slot 9).
    fn set_trip_destination_filter(&mut self, filter: u32);
    /// `SetDestinationFlag` (slot 10), OR.
    fn set_destination_flag(&mut self, bits: u8);
    /// `SetTrafficDensity` (slot 18).
    fn set_traffic_density(&mut self, trips: u32);
    /// `SetStartRect` (slot 19).
    fn set_start_rect(&mut self, rect: CellRect);
    /// `SetTravelTypeFlag(mask, alsoStarting)` (slot 13).
    fn set_travel_type_flag(&mut self, mask: u16, also_starting: bool);
    /// `FindPath` (slot 23): 1 = found.
    fn find_path(&mut self) -> u32;
    /// `GetEndCell` (slot 27).
    fn get_end_cell(&self) -> (i32, i32);
}

/// What the job search reads besides the pathfinder (§4.7.1). Typed host
/// input; no field is derived here.
pub trait JobSearchWorld {
    /// The MySim's `GetWealth()` (cISC4MySim slot 19).
    fn wealth(&self) -> u32;
    /// The bounding rect of an occupant's lot; `None` = no lot.
    fn occupant_lot_rect(&self, occupant: u64) -> Option<CellRect>;
    /// The bounding rect of the lot at a cell; `None` = no lot.
    fn lot_rect_at(&self, x: i32, z: i32) -> Option<CellRect>;
    /// `cISC4City::CellIsInBounds` (`+0x18C`).
    fn cell_is_in_bounds(&self, x: i32, z: i32) -> bool;
    /// The city's cell counts (`+0x174`, `+0x178`).
    fn cell_counts(&self) -> (i32, i32);
    /// `GetLot(x, z, false)` and the lot's `GetBuilding()`.
    fn lot_at(&self, x: i32, z: i32) -> LotAt;
    /// `FindCityOnOtherSideOfConnection(x, z)`: the regional city's
    /// position (its slot 5); `None` = no city.
    fn city_across_connection(&self, x: i32, z: i32) -> Option<(i32, i32)>;
    /// An occupant's `GetPosition()` x, z (cISC4Occupant slot 8), read by
    /// `SetByOccupant`.
    fn occupant_position(&self, occupant: u64) -> (f32, f32);
    /// The region's `GetCity(x, z)` (app slot 35, slot 11) as an opaque
    /// handle; `None` = null (`IsValid`).
    fn region_city(&self, x: i32, z: i32) -> Option<u64>;
    /// The length of the list the neighbours simulator's
    /// `GetConnectionsOfTypeToCity(type, city, list)` (slot 18) fills.
    fn connections_of_type_to_city(&self, connection_type: u32, city: u64) -> usize;
}

/// The core pathfinder as a [`JobPathfinder`].
pub struct CoreJobPathfinder<'a> {
    pub pf: &'a mut Pathfinder,
    pub net: &'a NetworkModel,
    pub host: &'a dyn PathfinderHost,
}

impl JobPathfinder for CoreJobPathfinder<'_> {
    fn reset(&mut self) {
        self.pf.reset();
    }
    fn clear_all_travel_type_flags(&mut self) {
        self.pf.clear_all_travel_type_flags();
    }
    fn set_route_info(&mut self, route: VecDeque<RouteData>) {
        self.pf.set_route_info(Some(route));
    }
    fn set_flag(&mut self, flags: u32) {
        self.pf.set_flag(flags);
    }
    fn set_trip_destination_filter(&mut self, f: u32) {
        self.pf.set_trip_destination_filter(f);
    }
    fn set_destination_flag(&mut self, bits: u8) {
        self.pf.set_destination_flag(bits);
    }
    fn set_traffic_density(&mut self, trips: u32) {
        self.pf.set_traffic_density(trips);
    }
    fn set_start_rect(&mut self, rect: CellRect) {
        self.pf.set_start_rect(rect);
    }
    fn set_travel_type_flag(&mut self, mask: u16, also_starting: bool) {
        self.pf.set_travel_type_flag(mask, also_starting);
    }
    fn find_path(&mut self) -> u32 {
        self.pf.find_path_on(self.net, self.host)
    }
    fn get_end_cell(&self) -> (i32, i32) {
        self.pf.get_end_cell()
    }
}

/// The dispatch fields the job search reads and writes.
#[derive(Clone, Debug, PartialEq)]
pub struct JobSearchState {
    /// Dispatch flags (`0x80` home set, `0x800|0x1000|0x4000` path).
    pub flags: u32,
    /// `+0x40`: the dispatch's trip state (answers MS11 d), written only by
    /// [`set_state`]; §4.7.1 step 2 tests it and §4.6 receives it through
    /// `0x4BB2CFC0`.
    pub state: i32,
    /// `+0x44`: the trip state before the last [`set_state`].
    pub prev_state: i32,
    /// The home location (`+0x84`, `SetDefaultDestination`).
    pub home: DispatchLocation,
    /// The commute destination (`+0xD4`).
    pub commute_destination: DispatchLocation,
    /// The route deque handed to `SetRouteInfo`.
    pub route: VecDeque<RouteData>,
}

impl Default for JobSearchState {
    fn default() -> Self {
        Self {
            flags: 0,
            state: -1,
            prev_state: -1,
            home: DispatchLocation::default(),
            commute_destination: DispatchLocation::default(),
            route: VecDeque::new(),
        }
    }
}

/// The dispatch flag `setState(3)` ORs in (§4.7.1).
pub const FLAG_AT_LOCATION: u32 = 0x2;
/// The trip state that sets [`FLAG_AT_LOCATION`].
pub const STATE_AT_LOCATION: i32 = 3;

/// A notification `setState` posts at the MySim (`0x2BA06B99`, §3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateNote {
    /// `0x4BB2D020`: the dispatch flags changed (old, new).
    Flags { old: u32, new: u32 },
    /// `0x4BB2CFC0`: the trip state changed (old, new).
    TripState { old: i32, new: i32 },
}

/// Dispatch `setState(s)` (`0x00431C10`, §4.7.1): old := `+0x40` (kept at
/// `+0x44`), `+0x40` := s; s = 3 ORs `2` into the flags (`0x4BB2D020` when
/// they change); old ≠ s → `0x4BB2CFC0(old, s)`. The notifications in
/// posting order.
pub fn set_state(st: &mut JobSearchState, s: i32) -> Vec<StateNote> {
    let mut notes = Vec::new();
    let old = st.state;
    st.prev_state = old;
    st.state = s;
    if s == STATE_AT_LOCATION {
        let of = st.flags;
        st.flags |= FLAG_AT_LOCATION;
        if st.flags != of {
            notes.push(StateNote::Flags { old: of, new: st.flags });
        }
    }
    if old != s {
        notes.push(StateNote::TripState { old, new: s });
    }
    notes
}

/// `resetPath` (§4.7.1 step 0).
pub fn reset_path(st: &mut JobSearchState, pf: &mut dyn JobPathfinder) {
    st.route.clear();
    st.flags &= !RESET_PATH_FLAGS;
    pf.reset();
    pf.clear_all_travel_type_flags();
    pf.set_route_info(st.route.clone());
}

/// The cell rect of the home location (§4.7.1 step 7): the lot of its
/// occupant (or the lot at its cell) → the lot's bounding rect; no lot →
/// the single cell.
fn home_start_rect(home: &DispatchLocation, world: &dyn JobSearchWorld) -> CellRect {
    let lot = match home.occupant {
        Some(occ) => world.occupant_lot_rect(occ),
        None => world.lot_rect_at(home.cell.0, home.cell.1),
    };
    lot.unwrap_or(CellRect::new(home.cell.0, home.cell.1, home.cell.0, home.cell.1))
}

/// `validateCommuteLocation(loc)` (§4.7.1, `0x00432F50`).
pub fn validate_commute_location(
    st: &mut JobSearchState,
    pf: &mut dyn JobPathfinder,
    world: &dyn JobSearchWorld,
    loc: &mut DispatchLocation,
) -> bool {
    reset_path(st, pf);
    *loc = DispatchLocation::default();
    loc.by_commute = true;
    if st.flags & FLAG_HAS_HOME == 0 {
        return false;
    }
    for f in JOB_SEARCH_FLAGS {
        pf.set_flag(f);
    }
    pf.set_trip_destination_filter(filter::STANDARD);
    pf.set_destination_flag(destination_flag_for_wealth(world.wealth()));
    pf.set_traffic_density(JOB_SEARCH_DENSITY);
    pf.set_start_rect(home_start_rect(&st.home, world));
    pf.set_travel_type_flag(COMMUTER_TRAVEL_MASK, true);
    if pf.find_path() != result::FOUND {
        return false;
    }
    let (x, z) = pf.get_end_cell();
    if world.cell_is_in_bounds(x, z) {
        let LotAt::Building(b) = world.lot_at(x, z) else {
            return false;
        };
        // `SetByOccupant` resets byCommute; the search restores the value
        // it had before the call (1, set in step 1).
        let by_commute = loc.by_commute;
        loc.set_by_occupant(Some((b, world.occupant_position(b))));
        loc.by_commute = by_commute;
        true
    } else {
        let Some(city) = world.city_across_connection(x, z) else {
            return false;
        };
        let (cx, cz) = world.cell_counts();
        loc.occupant = None;
        loc.in_city = false;
        loc.set_cell_position(clamp_cell(x, cx), clamp_cell(z, cz));
        loc.neighbor_city = city;
        true
    }
}

/// Clamp to `[0, count − 1]` (§4.7.1). Low bound first, then the high
/// bound; a city has at least one cell per axis, so the order only matters
/// for a 0-cell city (then −1), which cannot occur.
fn clamp_cell(v: i32, count: i32) -> i32 {
    v.max(0).min(count.saturating_sub(1))
}

/// `SetCurrentCommuteDestination(loc, notify)` (§4.7.1, `0x004350D0`):
/// `(result, notify the MySim with 0xEBA06BF4)`.
pub fn set_current_commute_destination(
    st: &mut JobSearchState,
    pf: &mut dyn JobPathfinder,
    world: &dyn JobSearchWorld,
    mut loc: DispatchLocation,
    notify: bool,
) -> (bool, bool) {
    if !loc.is_valid(world) {
        if !notify {
            return (false, false);
        }
        if st.state != STATE_STORE_UNVALIDATED && !validate_commute_location(st, pf, world, &mut loc) {
            return (false, false);
        }
    }
    st.commute_destination = loc;
    (true, notify)
}

/// What the dispatch reads from the game beyond the pathfinder host: the
/// MySim's home, wealth and job lots, the city, and a way to post the
/// notification messages. Notifications may synchronously reenter getters or
/// request methods; no state lock is held across an owner notification.
pub trait DispatchHost: PathfinderHost {
    /// The home lot's bounding rectangle (`CheckCommute`).
    fn home_rect(&self, sim: i32) -> Option<CellRect>;
    /// The lot rectangle of a building the commute is aimed at
    /// (`CheckCommute`); `None` when there is no such building.
    fn building_rect(&self, building: u64) -> Option<CellRect>;
    /// The MySim's `GetWealth()` (§4.7.1 step 5).
    fn wealth(&self, sim: i32) -> u32;
    /// The bounding rect of an occupant's lot.
    fn occupant_lot_rect(&self, occupant: u64) -> Option<CellRect>;
    /// The bounding rect of the lot at a cell.
    fn lot_rect_at(&self, x: i32, z: i32) -> Option<CellRect>;
    /// `cISC4City::CellIsInBounds`.
    fn cell_is_in_bounds(&self, x: i32, z: i32) -> bool;
    /// `GetLot(x, z, false)` → `GetBuilding()`.
    fn lot_at_cell(&self, x: i32, z: i32) -> LotAt;
    /// `FindCityOnOtherSideOfConnection(x, z)` → the city's position.
    fn city_across_connection(&self, x: i32, z: i32) -> Option<(i32, i32)>;
    /// An occupant's `GetPosition()` x, z (`SetByOccupant`).
    fn occupant_position(&self, occupant: u64) -> (f32, f32);
    /// The region's `GetCity(x, z)`; `None` = null (`IsValid`).
    fn region_city(&self, x: i32, z: i32) -> Option<u64>;
    /// `GetConnectionsOfTypeToCity(type, city, list)` → the list length.
    fn connections_of_type_to_city(&self, connection_type: u32, city: u64) -> usize;
    /// Whether the dispatch currently has a MySim owner (§3.3).
    fn has_owner(&self, sim: i32) -> bool;
    /// Synchronously delivers a `0x2BA06B99` notification to the MySim.
    fn notify(&self, sim: i32, sub_id: u32, data1: u32, data2: u32);
}

/// One MySim's [`DispatchHost`] as a [`JobSearchWorld`].
struct SimWorld<'a> {
    host: &'a dyn DispatchHost,
    sim: i32,
}

impl JobSearchWorld for SimWorld<'_> {
    fn wealth(&self) -> u32 {
        self.host.wealth(self.sim)
    }
    fn occupant_lot_rect(&self, occupant: u64) -> Option<CellRect> {
        self.host.occupant_lot_rect(occupant)
    }
    fn lot_rect_at(&self, x: i32, z: i32) -> Option<CellRect> {
        self.host.lot_rect_at(x, z)
    }
    fn cell_is_in_bounds(&self, x: i32, z: i32) -> bool {
        self.host.cell_is_in_bounds(x, z)
    }
    fn cell_counts(&self) -> (i32, i32) {
        PathfinderHost::cell_counts(self.host)
    }
    fn lot_at(&self, x: i32, z: i32) -> LotAt {
        self.host.lot_at_cell(x, z)
    }
    fn city_across_connection(&self, x: i32, z: i32) -> Option<(i32, i32)> {
        self.host.city_across_connection(x, z)
    }
    fn occupant_position(&self, occupant: u64) -> (f32, f32) {
        self.host.occupant_position(occupant)
    }
    fn region_city(&self, x: i32, z: i32) -> Option<u64> {
        self.host.region_city(x, z)
    }
    fn connections_of_type_to_city(&self, connection_type: u32, city: u64) -> usize {
        self.host.connections_of_type_to_city(connection_type, city)
    }
}

/// One MySim's dispatch state: the job-search fields (§4.7.1), whose
/// `state` is the trip state (§4.6). `CheckCommute`'s time is the MySim's
/// `lastCommuteTime` (§4.7), not a dispatch field.
#[derive(Clone, Debug, PartialEq)]
pub struct DispatchState {
    /// The job search's dispatch fields (§4.7.1), incl. the trip state.
    pub search: JobSearchState,
    pub current: DispatchLocation,
    pub destination: DispatchLocation,
    pub queue: VecDeque<DispatchLocation>,
    pub tuning: DispatchTuning,
    pub auxiliary_flag: bool,
    pub opaque_28: u32,
    pub path_indices: (u32, u32),
    pub change_counter: u32,
    pub previous_change_counter: u32,
    pub tracking_value: u32,
    pub minutes: i32,
    /// Stored as a full u32 in the record, including opaque upper bits.
    pub travel_type: u32,
    pub wait_timestamp: i32,
    pub neighbor_retries: u32,
    pub auxiliary_occupant: Option<u64>,
    pub auxiliary_value: u32,
    pub auxiliary_string: Vec<u8>,
    pub automata_occupant: Option<u64>,
    pub owner: Option<u64>,
    pub simulation_service: Option<u64>,
    pub models: BTreeMap<i32, (crate::stream::ResKey, i32)>,
    pub route_marker: i32,
    pub read_pending: bool,
    pub read_succeeded: bool,
}

/// The per-MySim trip controller.
pub struct MySimDispatch {
    host: Rc<dyn DispatchHost>,
    net: Rc<NetworkModel>,
    shared: SharedWorkCheck,
    sims: RwLock<BTreeMap<i32, DispatchState>>,
    /// The long-lived job-search pathfinder of each dispatch (created in
    /// `Init`, §4.7.1).
    pathfinders: RefCell<BTreeMap<i32, Pathfinder>>,
}

impl MySimDispatch {
    /// Owner calls are synchronous and absent owners receive nothing (§3.3).
    /// Call only after releasing dispatch state locks.
    fn notify_owner(&self, sim: i32, sub_id: u32, old: u32, new: u32) {
        if self.host.has_owner(sim) {
            self.host.notify(sim, sub_id, old, new);
        }
    }
    pub fn new(host: Rc<dyn DispatchHost>, net: Rc<NetworkModel>, shared: SharedWorkCheck) -> Rc<MySimDispatch> {
        Rc::new(MySimDispatch {
            host,
            net,
            shared,
            sims: RwLock::new(BTreeMap::new()),
            pathfinders: RefCell::new(BTreeMap::new()),
        })
    }

    /// The class and interface IDs (§1.1).
    pub fn class_id(&self) -> u32 {
        DISPATCH_CLSID
    }

    pub fn interface_id(&self) -> u32 {
        DISPATCH_IID
    }

    /// `Init` (clock24.md: only takes and drops the clock reference). The
    /// job-search pathfinder is created here once and kept for the
    /// dispatch's lifetime (§4.7.1).
    pub fn init(&self, sim: i32) -> bool {
        self.sims.write().unwrap().entry(sim).or_default();
        self.pathfinders
            .borrow_mut()
            .entry(sim)
            .or_insert_with(|| Pathfinder::create(&*self.host, self.shared.clone()));
        true
    }

    /// `Shutdown` (the MySim's Shutdown and LeaveCity call it, §4.7).
    pub fn shutdown(&self, sim: i32) {
        self.sims.write().unwrap().remove(&sim);
        self.pathfinders.borrow_mut().remove(&sim);
    }

    /// The job-search pathfinder's configuration as it stands (for tests
    /// and shadows): `(flags, filter, destination flags, density, start
    /// rect, allowed, starting)`.
    pub fn job_pathfinder_config(&self, sim: i32) -> Option<(u32, u32, u8, u32, CellRect, u16, u16)> {
        self.pathfinders.borrow().get(&sim).map(|pf| {
            (
                pf.flags(),
                pf.trip_destination_filter(),
                pf.destination_flags(),
                pf.traffic_density(),
                pf.start_rect(),
                pf.allowed_travel_types(),
                pf.starting_travel_types(),
            )
        })
    }

    /// `SetDefaultDestination(home)`: the home location; sets dispatch
    /// flag `0x80` (§4.7.1 step 2).
    pub fn set_default_destination(&self, sim: i32, home: DispatchLocation) {
        self.edit(sim, |st| st.search.home = home);
        self.change_flags(sim, |f| f | FLAG_HAS_HOME);
    }

    /// Dispatch `setState(s)` (§4.7.1, `0x00431C10`): the trip state
    /// `+0x40`, with its notifications (`0x4BB2D020` flags, `0x4BB2CFC0`
    /// trip state; data (old, new) as §3.3's handler reads them).
    pub fn set_state(&self, sim: i32, s: i32) {
        let notes = {
            let mut sims = self.sims.write().unwrap();
            let st = sims.entry(sim).or_default();
            let notes = set_state(&mut st.search, s);
            st.change_counter = st.change_counter.wrapping_add(notes.len() as u32);
            notes
        };
        for n in notes {
            match n {
                StateNote::Flags { old, new } => self.notify_owner(sim, super::dispatch_note::TRIP_STATE_A, old, new),
                StateNote::TripState { old, new } => {
                    self.notify_owner(sim, super::dispatch_note::TRIP_STATE_B, old as u32, new as u32)
                }
            }
        }
    }

    /// `SetCurrentCommuteDestination(loc, notify)` (§4.7.1). On success with
    /// `notify`, the MySim is notified with `0xEBA06BF4`. Steps 1–2 need no
    /// pathfinder; a dispatch never `Init`ed has none, so a search it would
    /// run fails *[choice]* (the original always has one after `Init`).
    pub fn set_current_commute_destination(&self, sim: i32, loc: DispatchLocation, notify: bool) -> bool {
        let old_flags = self.get_state(sim).search.flags;
        let (ok, fire) = {
            let mut pfs = self.pathfinders.borrow_mut();
            let mut sims = self.sims.write().unwrap();
            let st = &mut sims.entry(sim).or_default().search;
            let world = SimWorld { host: &*self.host, sim };
            match pfs.get_mut(&sim) {
                Some(pf) => {
                    let mut core = CoreJobPathfinder {
                        pf,
                        net: &self.net,
                        host: &*self.host,
                    };
                    set_current_commute_destination(st, &mut core, &world, loc, notify)
                }
                None if loc.is_valid(&world) || (notify && st.state == STATE_STORE_UNVALIDATED) => {
                    st.commute_destination = loc;
                    (true, notify)
                }
                None => (false, false),
            }
        };
        let new_flags = self.get_state(sim).search.flags;
        if old_flags != new_flags {
            self.edit(sim, |st| st.change_counter = st.change_counter.wrapping_add(1));
            self.notify_owner(sim, super::dispatch_note::TRIP_STATE_A, old_flags, new_flags);
        }
        if fire {
            self.notify_owner(sim, super::dispatch_note::JOB_FOUND, 0, 0);
        }
        ok
    }

    /// The state the simulation side reads (§4.6 trip variables).
    pub fn get_state(&self, sim: i32) -> DispatchState {
        self.sims.read().unwrap().get(&sim).cloned().unwrap_or_default()
    }

    /// Tests the dispatch's flags (§4.1 step 3 tracking flag): all bits of
    /// `mask` are set.
    pub fn check_flags(&self, sim: i32, mask: u32) -> bool {
        self.get_state(sim).search.flags & mask == mask
    }

    /// `FindNewJob` (§4.7.1, `0x00425A30`): a default location marked "by
    /// commute" through `SetCurrentCommuteDestination(loc, notify = true)`.
    /// `FailCommute` and `SetResidence` pass the same location (§4.7.1
    /// callers table). The destination is reported by
    /// [`Self::job_destination`] when `0xEBA06BF4` fires.
    pub fn find_new_job(&self, sim: i32) -> bool {
        let loc = DispatchLocation {
            by_commute: true,
            ..DispatchLocation::default()
        };
        self.set_current_commute_destination(sim, loc, true)
    }

    /// `ChangeJob(b)` (§4.7.1, `0x00425D70`): `SetByOccupant(b's occupant)`,
    /// byCommute 1, `SetCurrentCommuteDestination(loc, notify = true)`
    /// (answers MS11 e). `building` is `b`'s cISC4Occupant (`None` = null:
    /// the location is invalid and a normal job search runs).
    pub fn change_job(&self, sim: i32, building: Option<u64>) -> bool {
        let o = building.map(|b| (b, self.host.occupant_position(b)));
        let mut loc = DispatchLocation::by_occupant(o);
        loc.by_commute = true;
        self.set_current_commute_destination(sim, loc, true)
    }

    /// The destination the `0xEBA06BF4` notification reports (§4.7): a city
    /// lot's building at the location's cell (`SetByOccupant`'s
    /// `floor(position / 16)`, answers MS11 c), or the neighbour city at
    /// the location's cell. An in-city location without an occupant (the
    /// default stored at trip state 2) reports nothing.
    pub fn job_destination(&self, sim: i32) -> Option<JobDestination> {
        let dest = self.get_state(sim).search.commute_destination;
        match (dest.in_city, dest.occupant) {
            (true, Some(building)) => Some(JobDestination::Lot {
                building,
                cell: dest.cell,
            }),
            (false, _) => Some(JobDestination::Neighbor { cell: dest.cell }),
            (true, None) => None,
        }
    }

    /// Posts a flags-change notification (`0x4BB2D020`, data (old, new))
    /// after writing the flags. Only `setState` is specified as a writer
    /// (§4.7.1); this is the host's entry for the other flag writers of the
    /// trip automaton (not specified here).
    pub fn report_flags(&self, sim: i32, new_flags: u32, old_flags: u32) {
        // old_flags is retained for source compatibility; the authoritative old
        // value is the dispatch's stored value (§4.6.1), not caller input.
        let _ = old_flags;
        self.change_flags(sim, |_| new_flags);
    }

    /// Posts an arrival (§3.3 `0xEBA06C12`). The trip state is written only
    /// by [`Self::set_state`]; byCommute is not touched here (who clears it
    /// is the trip automaton's, not specified).
    pub fn report_arrival(&self, sim: i32, at_home: bool) {
        self.notify_owner(sim, super::dispatch_note::ARRIVAL, u32::from(at_home), 0);
    }

    /// Posts a MySim state transition (`0xEBA06BD4`, §3.3). This is the
    /// MySim's state, not the dispatch trip state, so nothing is written.
    pub fn report_sim_state(&self, sim: i32, state: u32) {
        self.notify_owner(sim, super::dispatch_note::STATE_1, state, 0);
    }
}

/// A [`JobPathfinder`] that records its configuration and the slot calls,
/// with a scripted `FindPath` answer (tests, conformance). The OR
/// semantics of slots 5, 10 and 13 are the pathfinder's (pathfinder.md
/// §4.2); `Reset` does not clear them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RecordingJobPathfinder {
    pub flags: u32,
    pub filter: u32,
    pub dest_flags: u8,
    pub density: u32,
    pub start_rect: Option<CellRect>,
    pub allowed: u16,
    pub starting: u16,
    /// Slot names in call order.
    pub calls: Vec<&'static str>,
    /// What `FindPath` answers.
    pub find_result: u32,
    /// What `GetEndCell` answers.
    pub end_cell: (i32, i32),
}

impl JobPathfinder for RecordingJobPathfinder {
    fn reset(&mut self) {
        self.calls.push("Reset");
    }
    fn clear_all_travel_type_flags(&mut self) {
        self.calls.push("ClearAllTravelTypeFlags");
        self.allowed = 0;
        self.starting = 0;
    }
    fn set_route_info(&mut self, _route: VecDeque<RouteData>) {
        self.calls.push("SetRouteInfo");
    }
    fn set_flag(&mut self, flags: u32) {
        self.calls.push("SetFlag");
        self.flags |= flags;
    }
    fn set_trip_destination_filter(&mut self, f: u32) {
        self.calls.push("SetTripDestinationFilter");
        self.filter = f;
    }
    fn set_destination_flag(&mut self, bits: u8) {
        self.calls.push("SetDestinationFlag");
        self.dest_flags |= bits;
    }
    fn set_traffic_density(&mut self, trips: u32) {
        self.calls.push("SetTrafficDensity");
        self.density = trips;
    }
    fn set_start_rect(&mut self, rect: CellRect) {
        self.calls.push("SetStartRect");
        self.start_rect = Some(rect);
    }
    fn set_travel_type_flag(&mut self, mask: u16, also_starting: bool) {
        self.calls.push("SetTravelTypeFlag");
        self.allowed |= mask;
        if also_starting {
            self.starting |= mask;
        }
    }
    fn find_path(&mut self) -> u32 {
        self.calls.push("FindPath");
        self.find_result
    }
    fn get_end_cell(&self) -> (i32, i32) {
        self.end_cell
    }
}
