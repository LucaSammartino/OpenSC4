// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The pathfinder object: the state of §2, creation and lifetime (§1), the
//! configuration slots (§4.2, §7) and the result getters (§4.14).

use std::collections::VecDeque;

use crate::x87::r24;

use super::nodes::{Node, NodeStore, OpenSet};
use super::{
    filter, CellRect, PathfinderHost, SharedWorkCheck, TravelTables, DEFAULT_TRAVEL_MASK, IID_PATH_FINDER, IID_UNKNOWN,
    NO_EXCLUDED_RECT, NO_PARK_AND_RIDE,
};

/// One route deque element (§7): `{u8 x, u8 z, u8 travelType}`; coordinates
/// are the low bytes (−1 becomes 255).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RouteData {
    pub x: u8,
    pub z: u8,
    pub travel_type: u8,
}

/// `cSC4PathFinder` (§1, §2). Created by the traffic simulator
/// ([`Pathfinder::create`]); transient (nothing is saved, §6).
#[derive(Clone, Debug)]
pub struct Pathfinder {
    /// The process-wide work-check statics (§4.11).
    pub(super) shared: SharedWorkCheck,
    pub(super) ref_count: u32,
    pub(super) destroyed: bool,
    pub(super) initialised: bool,
    /// The city size W, H (Init).
    pub(super) count_x: i32,
    pub(super) count_z: i32,

    // ---- configuration (§2, §4.2) ----
    pub(super) flags: u32,
    pub(super) filter: u32,
    pub(super) dest_flags: u8,
    /// Allowed travel types: gates switch transitions only (§4.8.2 (b)).
    pub(super) allowed: u16,
    /// Starting travel types.
    pub(super) starting: u16,
    /// The caller's starting cost array; `None` = the simulator's default.
    pub(super) start_costs: Option<[f32; 9]>,
    /// The shared max-trip-length / trip-length float (§4.3) *[quirk]*.
    pub(super) trip_length: f32,
    pub(super) density: u32,
    pub(super) arrived: u32,
    pub(super) start_rect: CellRect,
    pub(super) end_rect: CellRect,
    /// The route deque (`SetRouteInfo`); `None` = no route.
    pub(super) route: Option<VecDeque<RouteData>>,
    pub(super) max_work_time: u32,
    pub(super) suspended: bool,
    /// The occupant filter (filter 2): no setter in the interface (§2).
    pub(super) occupant_filter: Option<u64>,

    // ---- derived per call (§4.4) ----
    pub(super) max_cost: [f32; 9],
    pub(super) search_rect: CellRect,
    pub(super) excluded_rect: CellRect,
    pub(super) weight: f32,
    /// `dX[i]` at index `i + 1`, i = −1…W.
    pub(super) dist_x: Vec<i32>,
    /// `dZ[i]` at index `i + 1`, i = −1…H.
    pub(super) dist_z: Vec<i32>,
    pub(super) over_budget: [u32; 9],
    /// Disabled travel types (bit `t`).
    pub(super) disabled: u16,
    /// The allowed mask for switch transitions (strike-reduced, step 6).
    pub(super) switch_mask: u16,
    /// Whether the strike is on for this call (step 6).
    pub(super) strike: bool,
    /// The simulator's per-travel-type tables, read at the call's start.
    pub(super) tables: TravelTables,

    // ---- search state (§4.7) ----
    pub(super) store: NodeStore,
    pub(super) open: OpenSet,
    pub(super) result: Option<u32>,

    // ---- result (§4.10, §4.13) ----
    pub(super) end_cell: (i32, i32),
    pub(super) dest_type_reached: u8,
    pub(super) park_ride: (i32, i32),
    pub(super) park_ride_side: usize,

    // ---- work check (§4.11) ----
    /// The per-call work counter.
    pub(super) work_counter: u32,
    /// The multiplier `m` read at call entry.
    pub(super) work_step: u32,
    pub(super) deadline: u64,
}

impl Pathfinder {
    /// The constructor (§1): count 0, not initialised, no result node,
    /// heuristic weight 1.0, trip length 0.0, both travel masks Car, density
    /// and arrived 0, flags 0, no route, filter 3, destination flags 0, no
    /// occupant filter, no starting cost array, max work time 0, not
    /// suspended, an empty open set and node store. The fields the original
    /// leaves uninitialised are zero-filled *[choice]*.
    pub fn new(shared: SharedWorkCheck) -> Pathfinder {
        let zero = CellRect::new(0, 0, 0, 0);
        Pathfinder {
            shared,
            ref_count: 0,
            destroyed: false,
            initialised: false,
            count_x: 0,
            count_z: 0,
            flags: 0,
            filter: filter::END_RECT,
            dest_flags: 0,
            allowed: DEFAULT_TRAVEL_MASK,
            starting: DEFAULT_TRAVEL_MASK,
            start_costs: None,
            trip_length: 0.0,
            density: 0,
            arrived: 0,
            start_rect: zero,
            end_rect: zero,
            route: None,
            max_work_time: 0,
            suspended: false,
            occupant_filter: None,
            max_cost: [0.0; 9],
            search_rect: zero,
            excluded_rect: NO_EXCLUDED_RECT,
            weight: 1.0,
            dist_x: Vec::new(),
            dist_z: Vec::new(),
            over_budget: [0; 9],
            disabled: 0,
            switch_mask: DEFAULT_TRAVEL_MASK,
            strike: false,
            tables: TravelTables::default(),
            store: NodeStore::new(),
            open: OpenSet::new(),
            result: None,
            end_cell: (0, 0),
            dest_type_reached: 0,
            park_ride: NO_PARK_AND_RIDE,
            park_ride_side: 0,
            work_counter: 0,
            work_step: 0,
            deadline: 0,
        }
    }

    /// `cISC4TrafficSimulator::CreatePathFinder` (§1): construct, `AddRef`
    /// (count 0 → 1), then `Init`. The reference goes to the caller.
    pub fn create(host: &dyn PathfinderHost, shared: SharedWorkCheck) -> Pathfinder {
        let mut pf = Pathfinder::new(shared);
        pf.add_ref();
        pf.init(host);
        pf
    }

    // ---- lifetime (§1, slots 0–4) ----

    /// `QueryInterface` (slot 0): IIDs 1 and `0xAA0DB766`, with an `AddRef`.
    pub fn query_interface(&mut self, iid: u32) -> bool {
        if iid == IID_UNKNOWN || iid == IID_PATH_FINDER {
            self.add_ref();
            true
        } else {
            false
        }
    }

    /// `AddRef` (slot 1).
    pub fn add_ref(&mut self) -> u32 {
        self.ref_count = self.ref_count.wrapping_add(1);
        self.ref_count
    }

    /// `Release` (slot 2): above 1, decrement and return the new count.
    /// Otherwise (0 or 1) the count is not decremented: `Shutdown` if
    /// initialised, then the object is destroyed and 0 returned. The caller
    /// drops the object when this returns 0 ([`Pathfinder::is_destroyed`]).
    pub fn release(&mut self) -> u32 {
        if self.ref_count > 1 {
            self.ref_count -= 1;
            return self.ref_count;
        }
        if self.initialised {
            self.shutdown();
        }
        self.destroyed = true;
        0
    }

    /// `Init` (slot 3): false if already initialised; otherwise reads the city
    /// size and sizes the two distance tables to W + 2 and H + 2 zeroed
    /// entries.
    pub fn init(&mut self, host: &dyn PathfinderHost) -> bool {
        if self.initialised {
            return false;
        }
        self.initialised = true;
        let (w, h) = host.cell_counts();
        self.count_x = w;
        self.count_z = h;
        self.dist_x = vec![0; (w.max(0) + 2) as usize];
        self.dist_z = vec![0; (h.max(0) + 2) as usize];
        true
    }

    /// `Shutdown` (slot 4): false if not initialised; otherwise clears the
    /// flag, `ReleasePathNodes`, and forgets the distance tables.
    pub fn shutdown(&mut self) -> bool {
        if !self.initialised {
            return false;
        }
        self.initialised = false;
        self.release_path_nodes();
        self.dist_x.clear();
        self.dist_z.clear();
        true
    }

    // ---- configuration (§4.2, slots 5–22) ----

    /// `SetFlag` (slot 5): OR into the flags.
    pub fn set_flag(&mut self, flags: u32) {
        self.flags |= flags;
    }

    /// `ClearFlag` (slot 6).
    pub fn clear_flag(&mut self, flags: u32) {
        self.flags &= !flags;
    }

    /// `ClearAllFlags` (slot 7).
    pub fn clear_all_flags(&mut self) {
        self.flags = 0;
    }

    /// `SetRouteInfo` (slot 8): the route deque; `None` = no route. The
    /// pathfinder holds the deque; [`Pathfinder::route`] reads it back.
    pub fn set_route_info(&mut self, route: Option<VecDeque<RouteData>>) {
        self.route = route;
    }

    /// `SetTripDestinationFilter` (slot 9).
    pub fn set_trip_destination_filter(&mut self, filter: u32) {
        self.filter = filter;
    }

    /// `SetDestinationFlag` (slot 10): OR.
    pub fn set_destination_flag(&mut self, bits: u8) {
        self.dest_flags |= bits;
    }

    /// `ClearDestinationFlag` (slot 11).
    pub fn clear_destination_flag(&mut self, bits: u8) {
        self.dest_flags &= !bits;
    }

    /// `ClearAllDestinationFlags` (slot 12).
    pub fn clear_all_destination_flags(&mut self) {
        self.dest_flags = 0;
    }

    /// `SetTravelTypeFlag(mask, alsoStarting)` (slot 13): `allowed |= mask`;
    /// with `also_starting`, `starting |= mask`.
    pub fn set_travel_type_flag(&mut self, mask: u16, also_starting: bool) {
        self.allowed |= mask;
        if also_starting {
            self.starting |= mask;
        }
    }

    /// `ClearTravelTypeFlag(mask)` (slot 14): clears `mask` in both masks.
    pub fn clear_travel_type_flag(&mut self, mask: u16) {
        self.allowed &= !mask;
        self.starting &= !mask;
    }

    /// `ClearAllTravelTypeFlags` (slot 15): both masks 0.
    pub fn clear_all_travel_type_flags(&mut self) {
        self.allowed = 0;
        self.starting = 0;
    }

    /// `SetStartingCostArray` (slot 16). The original stores the caller's
    /// pointer and reads the nine floats at every `FindPath`; FreeCity4 keeps
    /// a copy, so a caller that changes its array sets it again. `None` =
    /// the simulator's default array.
    pub fn set_starting_cost_array(&mut self, costs: Option<[f32; 9]>) {
        self.start_costs = costs;
    }

    /// `SetMaxTripLength` (slot 17): the float shared with the trip length
    /// (§4.3) *[quirk]*.
    pub fn set_max_trip_length(&mut self, v: f32) {
        self.trip_length = v;
    }

    /// `SetTrafficDensity` (slot 18).
    pub fn set_traffic_density(&mut self, trips: u32) {
        self.density = trips;
    }

    /// `SetStartRect` (slot 19), inclusive.
    pub fn set_start_rect(&mut self, r: CellRect) {
        self.start_rect = r;
    }

    /// `SetEndRect` (slot 20), inclusive.
    pub fn set_end_rect(&mut self, r: CellRect) {
        self.end_rect = r;
    }

    /// `SetMaxWorkTime` (slot 21); 0 = unlimited.
    pub fn set_max_work_time(&mut self, ms: u32) {
        self.max_work_time = ms;
    }

    /// `Reset` (slot 22): clears the suspended state only.
    pub fn reset(&mut self) {
        self.suspended = false;
    }

    /// Sets the occupant filter used by filter 2. **Not part of the
    /// interface**: the original has no setter (§2, §10 Q2); FreeCity4
    /// exposes one so the documented behaviour of §4.10.3 can be exercised
    /// *[choice]*.
    pub fn set_occupant_filter(&mut self, filter: Option<u64>) {
        self.occupant_filter = filter;
    }

    // ---- the traffic simulator's direct field access (traffic-simulator.md
    //      §4.4: pf2 is configured by writing its fields) ----

    /// Writes the allowed travel-type mask.
    pub fn set_allowed_travel_types(&mut self, mask: u16) {
        self.allowed = mask;
    }

    /// Writes the starting travel-type mask.
    pub fn set_starting_travel_types(&mut self, mask: u16) {
        self.starting = mask;
    }

    /// Writes the flags.
    pub fn set_flags(&mut self, flags: u32) {
        self.flags = flags;
    }

    /// Writes the park-and-ride cell and side (§4.8.4).
    pub fn set_park_and_ride(&mut self, cell: (i32, i32), side: usize) {
        self.park_ride = cell;
        self.park_ride_side = side & 3;
    }

    // ---- result getters (§4.14, slots 24–28) ----

    /// `GetTrafficArrived` (slot 24).
    pub fn get_traffic_arrived(&self) -> u32 {
        self.arrived
    }

    /// `GetTripLength` (slot 25): the shared float (§4.3).
    pub fn get_trip_length(&self) -> f32 {
        self.trip_length
    }

    /// `GetTripLengthScaledForDisplay` (slot 26):
    /// `r24(simulator.GetTripScaleForDisplay() × tripLength)`.
    pub fn get_trip_length_scaled_for_display(&self, host: &dyn PathfinderHost) -> f32 {
        r24(f64::from(host.trip_scale_for_display()) * f64::from(self.trip_length)) as f32
    }

    /// `GetEndCell` (slot 27): the end cell of the last goal test that
    /// succeeded.
    pub fn get_end_cell(&self) -> (i32, i32) {
        self.end_cell
    }

    /// `GetLastTravelType` (slot 28): the result node's type, 0 if none.
    pub fn get_last_travel_type(&self) -> u8 {
        self.result.map_or(0, |id| self.store.get(id).t)
    }

    /// `ReleasePathNodes` (slot 31, §4.18): the node store emptied (storage
    /// kept), the open set emptied, the result node forgotten.
    pub fn release_path_nodes(&mut self) {
        self.store.clear();
        self.open.clear();
        self.result = None;
    }

    // ---- inspection (not interface slots) ----

    /// The reference count.
    pub fn ref_count(&self) -> u32 {
        self.ref_count
    }

    /// Whether `Release` destroyed the object (the caller drops it).
    pub fn is_destroyed(&self) -> bool {
        self.destroyed
    }

    /// Whether `Init` ran (and no `Shutdown` since).
    pub fn is_initialised(&self) -> bool {
        self.initialised
    }

    /// The city size read at `Init`.
    pub fn city_size(&self) -> (i32, i32) {
        (self.count_x, self.count_z)
    }

    /// The route deque, if one is set.
    pub fn route(&self) -> Option<&VecDeque<RouteData>> {
        self.route.as_ref()
    }

    /// Takes the route deque out (the pathfinder then has none).
    pub fn take_route(&mut self) -> Option<VecDeque<RouteData>> {
        self.route.take()
    }

    pub fn flags(&self) -> u32 {
        self.flags
    }

    pub fn trip_destination_filter(&self) -> u32 {
        self.filter
    }

    pub fn destination_flags(&self) -> u8 {
        self.dest_flags
    }

    pub fn allowed_travel_types(&self) -> u16 {
        self.allowed
    }

    pub fn starting_travel_types(&self) -> u16 {
        self.starting
    }

    pub fn starting_cost_array(&self) -> Option<[f32; 9]> {
        self.start_costs
    }

    pub fn traffic_density(&self) -> u32 {
        self.density
    }

    pub fn start_rect(&self) -> CellRect {
        self.start_rect
    }

    /// The end rect; filter 0 overwrites it (§4.5) *[quirk]*.
    pub fn end_rect(&self) -> CellRect {
        self.end_rect
    }

    pub fn max_work_time(&self) -> u32 {
        self.max_work_time
    }

    pub fn is_suspended(&self) -> bool {
        self.suspended
    }

    /// `maxCost[t]` of the last call (§4.4 step 4).
    pub fn max_cost(&self) -> [f32; 9] {
        self.max_cost
    }

    /// The search rect of the last call (§4.4 step 7).
    pub fn search_rect(&self) -> CellRect {
        self.search_rect
    }

    /// The excluded edge rect (§4.4 step 5, §4.5 step 2).
    pub fn excluded_rect(&self) -> CellRect {
        self.excluded_rect
    }

    /// The heuristic weight `w` (§4.4 step 7).
    pub fn weight(&self) -> f32 {
        self.weight
    }

    /// The over-budget counters (§4.9 step 4).
    pub fn over_budget(&self) -> [u32; 9] {
        self.over_budget
    }

    /// The travel types disabled for the call (bit `t`).
    pub fn disabled_travel_types(&self) -> u16 {
        self.disabled
    }

    /// The destination type reached by the last filter-0 success (§4.10.1).
    pub fn destination_type_reached(&self) -> u8 {
        self.dest_type_reached
    }

    /// The park-and-ride cell and side (§4.13 step 1); (−2, −2) = none.
    pub fn park_and_ride(&self) -> ((i32, i32), usize) {
        (self.park_ride, self.park_ride_side)
    }

    /// The node store.
    pub fn nodes(&self) -> &NodeStore {
        &self.store
    }

    /// The open set.
    pub fn open_set(&self) -> &OpenSet {
        &self.open
    }

    /// The result node, if any.
    pub fn result_node(&self) -> Option<&Node> {
        self.result.map(|id| self.store.get(id))
    }

    /// The per-call work counter (§4.11).
    pub fn work_counter(&self) -> u32 {
        self.work_counter
    }

    /// The distance tables `dX` (index `i + 1`) and `dZ` (§4.4).
    pub fn distance_tables(&self) -> (&[i32], &[i32]) {
        (&self.dist_x, &self.dist_z)
    }

    /// The nodes from the root to the result node (empty without a result).
    pub fn result_path(&self) -> Vec<Node> {
        let mut out = Vec::new();
        let mut cur = self.result;
        while let Some(id) = cur {
            let n = *self.store.get(id);
            out.push(n);
            cur = n.parent;
        }
        out.reverse();
        out
    }
}
