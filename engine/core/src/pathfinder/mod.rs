// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The trip pathfinder `cSC4PathFinder` (`docs/specs/pathfinder.md`). Section
//! references (`§n`) point at that spec unless they name another one.
//!
//! The pathfinder finds one route for one batch of trips over the cell graph
//! of the traffic network ([`crate::traffic_network`]): a modified A\* with a
//! binary-heap open set whose tie order is part of the contract (§4.7).
//!
//! # Structure
//!
//! - [`Pathfinder`] (`object.rs`): the state of §2, creation and lifetime
//!   (§1), the configuration slots (§4.2) and the result getters (§4.14).
//! - `nodes.rs`: search nodes, the node store and the open set (§4.7).
//! - `search.rs`: `FindPath` (§4.3–§4.13, §4.18).
//! - `route.rs`: the compressed route (§4.15) and `WasTripReversible`
//!   (§4.16).
//! - `traffic.rs`: `ApplyTraffic` (§4.17).
//! - [`PathfinderHost`]: what the pathfinder reads from, or does to, its
//!   owning traffic simulator and the city (§8) that the traffic network core
//!   does not model.
//!
//! # Where the inputs come from
//!
//! The pathfinder reads the network model of `traffic-network.md` directly
//! (§8 "direct member reads of the owner"): the cell connection records, the
//! network-info lists, the transit-switch ID map and table
//! (`TransitSwitchEntryCost`), the tunnel and ferry tables (the terminal list
//! with its neighbour links, which also feeds the ferry-arrival start nodes
//! of §4.6), the edge-density map, the neighbour connection edge records
//! (`GetTrafficNeighborConnection`, traffic-network.md §2.7.1, the trip-length
//! cost of §4.13 step 3), the inverse max-speed table (`0x00B4B110`, the class
//! static the tunnel costs also read) and the congestion curve
//! (`0x00B4B2F0`). FreeCity4 keeps all of these in [`NetworkState`], so the
//! search takes the network state as part of its context ([`PathCx`]) instead
//! of copying it. Everything else (volume grids, the destination bitmask map
//! and free counts, the per-travel-type tables, lots, neighbour pools, the
//! income accumulator, the strike state, the work timer) comes through
//! [`PathfinderHost`].
//!
//! # Re-entrancy
//!
//! The pathfinder holds no `RefCell`. Its operations take `&mut self` and a
//! context; host callbacks are made with no borrow of the pathfinder's own
//! cells held. Callers that keep the network in a
//! [`crate::traffic_network::NetworkModel`] use [`Pathfinder::find_path_on`] /
//! [`Pathfinder::apply_traffic_on`], which take the network state out of the
//! model for the duration of the call (`NetworkModel::run`), so no network
//! borrow is held across a host callback either.
//!
//! # Arithmetic
//!
//! Foundations §18 (24-bit x87): every float add, subtract and multiply is one
//! `f32` rounding ([`crate::x87::r24`]) with integer and `double` operands
//! entering unrounded (§4.19).
//!
//! # Random draws (§4.20)
//!
//! None. Nothing in this module draws from a generator.
//!
//! # Choices made where the spec is open (`[choice]`)
//!
//! - The constructor's uninitialised fields (start and end rect, end cell,
//!   park-and-ride cell, destination type reached) are zero-filled (§1, §10
//!   Q1).
//! - `FindPath` on an object that is not initialised (no `Init`, or after
//!   `Shutdown`) returns 0 without doing anything; the original reads through
//!   unset or null distance tables (PF11, §4.3; unreachable in the game).
//! - A null occupant filter makes filter 2 never match (§4.10.3).
//! - Off-map switch moves read "no switch, no info" (§10 Q4).
//! - The cost of an unknown transit-switch ID is the network model's choice
//!   (TN23, traffic-network.md §4.7.3): its inserted switch is all zeros, so
//!   the step costs `0x7EFFFFFF`; the original's holds stack garbage.
//! - A switch nibble naming a type above 8 is ignored (PF13, §4.8.2 (b); the
//!   original indexes its 9-entry tables with it, but no game caller sets
//!   such a bit in the allowed mask).
//! - Distance-table indices outside −1…W / −1…H (unreachable, PF14) use the
//!   table formula instead of reading past the table.
//! - An Update whose recorded heap slot does not hold the node is a no-op
//!   (PF9, §4.7 "Position"); the original would overwrite that slot. In the
//!   game an open node's recorded slot always holds it.
//! - The inner cell of an off-map route node whose
//!   `GetNeighborConnectionInnerCell` fails (a corner cell) is the node's own
//!   cell, low bytes (PF10, §4.15); the original writes stale stack bytes.
//!   Unreachable in the game.
//! - The process-wide work-check statics (§4.11) are one shared object
//!   ([`SharedWorkCheck`]) handed to every pathfinder, like the other cores'
//!   class statics.

mod nodes;
mod object;
mod route;
mod search;
mod traffic;

#[cfg(test)]
mod tests;

pub use nodes::{
    heap_order, key_cell, node_key, Node, NodeStore, OpenSet, BUCKETS, EXIT_KEY_BIT, HASH_MULTIPLIER, PAGE_NODES,
};
pub use object::{Pathfinder, RouteData};
pub use route::{encode_route, RouteMap, RouteNode};
pub use search::{distance_to_rect, replaces_node_g, start_ring_cells};

use std::cell::Cell;
use std::rc::Rc;

pub use crate::grid::CellRect;
use crate::traffic_network::NetworkState;

/// `cISC4PathFinder` (§1). QueryInterface also answers `cIGZUnknown` (1).
pub const IID_PATH_FINDER: u32 = 0xAA0D_B766;
/// `cIGZUnknown`.
pub const IID_UNKNOWN: u32 = 1;

/// `FindPath` results (§4.3).
pub mod result {
    pub const NOT_FOUND: u32 = 0;
    pub const FOUND: u32 = 1;
    pub const SUSPENDED: u32 = 2;
}

/// The flag bits (§4.2).
pub mod flag {
    /// Do not reject nodes whose cost exceeds the max cost (§4.9).
    pub const NO_BUDGET: u32 = 1;
    /// With filter 3 and a route deque, an exhausted search returns the node
    /// nearest to the end rect (§4.12).
    pub const NEAREST_ON_FAILURE: u32 = 2;
    /// Keep the node store and open set of the previous call (§4.4).
    pub const CONTINUE: u32 = 4;
    /// Keep the park-and-ride cell of the previous trip (§4.8.4).
    pub const PARK_AND_RIDE: u32 = 8;
    /// No ferries (§4.8.3).
    pub const NO_FERRIES: u32 = 16;
}

/// Destination filters (§4.2).
pub mod filter {
    /// The nearest lot (or neighbour pool) with free capacity (§4.10.1).
    pub const STANDARD: u32 = 0;
    /// Never matches.
    pub const NEVER: u32 = 1;
    /// The occupant filter (§4.10.3).
    pub const OCCUPANT: u32 = 2;
    /// The end rectangle (§4.10.2).
    pub const END_RECT: u32 = 3;
    /// Never matches.
    pub const NEVER4: u32 = 4;
}

/// `network(t)` (§4.1, table `0x00AB20F8`): the same values as the traffic
/// network's `net(t)` (`0x00AB31A8`), so the network core's table is used.
pub const NETWORK_OF_TRAVEL: [u32; 9] = crate::traffic_network::NET_OF_TRAVEL;

/// `ProhibitSharpTurns[t]` (§4.1, `0x00AB211C`).
pub const PROHIBIT_SHARP_TURNS: [bool; 9] = [false, false, false, true, false, true, false, true, true];

/// `LegalStartingSubtype[type]` (§4.1, `0x00AB2128`).
pub const LEGAL_STARTING_SUBTYPE: [bool; 13] = [
    true, true, false, true, false, false, true, true, true, true, true, true, false,
];

/// `LegalEndingSubtype[type]` (§4.1, `0x00AB2138`).
pub const LEGAL_ENDING_SUBTYPE: [bool; 13] = [
    true, false, false, true, false, false, true, false, false, false, true, true, false,
];

/// The travel-type masks the constructor sets (§1): Car.
pub const DEFAULT_TRAVEL_MASK: u16 = 0x0002;
/// The allowed mask for switch transitions during a strike (§4.4 step 6):
/// Walk, Car, Freight truck, Freight train.
pub const STRIKE_SWITCH_MASK: u16 = 0x0033;
/// The strike state that counts as "on strike" (§4.4 step 6,
/// traffic-simulator.md §4.12).
pub const STRIKE_ON: u8 = 1;
/// The Carpool Incentive ordinance (§4.17).
pub const CARPOOL_ORDINANCE: u32 = 0xA2BF_1DDC;
/// The carpool factor `0.8f` (§4.17).
pub const CARPOOL_FACTOR_BITS: u32 = 0x3F4C_CCCD;
/// Over-budget rejections a travel type survives in one call; the next one
/// disables it (§4.9 step 4).
pub const OVER_BUDGET_LIMIT: u32 = 64;
/// The work counter is checked against the timer at multiples of this
/// (§4.11).
pub const WORK_CHECK_INTERVAL: u32 = 1024;
/// The search rectangle margin (§4.4 step 7).
pub const SEARCH_RECT_MARGIN: i32 = 4;
/// The park-and-ride cell meaning "none" (§4.4 step 3).
pub const NO_PARK_AND_RIDE: (i32, i32) = (-2, -2);
/// The excluded edge rect meaning "none" (§4.4 step 5).
pub const NO_EXCLUDED_RECT: CellRect = CellRect {
    x1: -2,
    z1: -2,
    x2: -2,
    z2: -2,
};
/// The start-node key prefix (`0x10 | side`, §4.7).
pub const START_KEY_PREFIX: u32 = 0x10;

/// The per-travel-type tables the pathfinder reads from its owning traffic
/// simulator (§4.1; traffic-simulator.md §4.14), indexed by travel type.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TravelTables {
    /// The simulator's default starting cost array (used when no array was
    /// set, §4.4 step 4).
    pub default_start_cost: [f32; 9],
    /// `canReachDestination[t]`.
    pub can_reach_destination: [bool; 9],
    /// `generatesTraffic[t]`.
    pub generates_traffic: [bool; 9],
    /// `incomePerTile[t]`.
    pub income_per_tile: [f32; 9],
    /// `affectedByTraffic[t]` (class static `0x00B4B100`).
    pub affected_by_traffic: [bool; 9],
}

impl Default for TravelTables {
    fn default() -> Self {
        TravelTables {
            default_start_cost: [0.0; 9],
            can_reach_destination: [false; 9],
            generates_traffic: [false; 9],
            income_per_tile: [0.0; 9],
            affected_by_traffic: [false; 9],
        }
    }
}

/// Everything the pathfinder reads from, or does to, its owning traffic
/// simulator and the city that the traffic network model does not hold (§8).
/// Game objects are opaque `u64` tokens; 0 is null. Coordinates are city
/// cells; the destination map also answers for the off-map ring (−1…W,
/// −1…H).
///
/// Host callbacks must not call back into the pathfinder that is calling
/// them.
pub trait PathfinderHost {
    // ---- city (§1 Init) ----
    /// `cISC4City::CellCountX()` / `CellCountZ()` (slots 93, 94).
    fn cell_counts(&self) -> (i32, i32);

    // ---- work timer (§4.11) ----
    /// Starts a work timer of `ms` milliseconds (`0x0068BF50`: now +
    /// `_ftol(ms × ticksPerMs)`) and returns its deadline.
    fn work_deadline(&self, ms: u32) -> u64;
    /// Whether the counter has reached `deadline` (`0x0068BF80`).
    fn work_deadline_reached(&self, deadline: u64) -> bool;

    // ---- the simulator's tables and settings (traffic-simulator.md) ----
    /// The per-travel-type tables (§4.1).
    fn travel_tables(&self) -> TravelTables;
    /// `MaxDistanceOriginToNetwork` (rings, §4.6).
    fn max_distance_origin_to_network(&self) -> u32;
    /// `NearestDestinationAttractiveness` (§4.4 step 7).
    fn nearest_destination_attractiveness(&self) -> f32;
    /// The mass-transit strike state (1 = on strike, §4.4 step 6).
    fn strike_state(&self) -> u8;
    /// `GetTripScaleForDisplay()` (traffic simulator slot 15).
    fn trip_scale_for_display(&self) -> f32;

    // ---- the volume grids (one f32 grid per transit network n = 0…5) ----
    /// The volume of transit network `n` at an in-city cell.
    fn volume(&self, n: u32, x: i32, z: i32) -> f32;
    /// Writes the volume of transit network `n` at an in-city cell
    /// (ApplyTraffic, §4.17).
    fn set_volume(&self, n: u32, x: i32, z: i32, v: f32);

    // ---- destinations (§4.5, §4.10.1) ----
    /// The destination bitmask byte at (x, z), x in −1…W, z in −1…H (the
    /// `[x+1][z+1]` map).
    fn destination_byte(&self, x: i32, z: i32) -> u8;
    /// Writes the destination bitmask byte at (x, z).
    fn set_destination_byte(&self, x: i32, z: i32, bits: u8);
    /// The simulator's count of free destinations of type `k` (0…3).
    fn free_destinations(&self, k: u32) -> u32;
    /// Writes the count of free destinations of type `k`.
    fn set_free_destinations(&self, k: u32, n: u32);
    /// `GetMaxTripCapacity(holder, k)` (traffic simulator slot 20).
    fn max_trip_capacity(&self, holder: u64, k: u32) -> u32;
    /// The used trip capacity (`0x0070DFF0`, "arrived this cycle").
    fn used_trip_capacity(&self, holder: u64, k: u32) -> u32;
    /// `GetNeighborPoolPropertyHolder(t, x, z)` (`0x007114E0`).
    fn neighbor_pool_holder(&self, t: u32, x: i32, z: i32) -> Option<u64>;
    /// The same question with the network state at hand, which is what the
    /// search asks: a host that keeps the pools resolves the record's pool
    /// ID here (and resets a stale one, traffic-network.md §2.7.1). The
    /// default ignores the state and asks
    /// [`PathfinderHost::neighbor_pool_holder`].
    fn neighbor_pool_holder_in(&self, net: &mut NetworkState, t: u32, x: i32, z: i32) -> Option<u64> {
        let _ = net;
        self.neighbor_pool_holder(t, x, z)
    }

    // ---- income (§4.17) ----
    /// The simulator's double income accumulator.
    fn income_accumulator(&self) -> f64;
    /// Writes the income accumulator.
    fn set_income_accumulator(&self, v: f64);

    // ---- lots and occupants (§8) ----
    /// `cISC4LotManager::GetLot(x, z, false)` (slot 6): the lot covering the
    /// cell, not only one whose origin it is. The pathfinder always passes
    /// `originOnly = false` (PF3, §8: the lot start, the filter-0 goal and
    /// §4.5 step 5).
    fn lot_at(&self, x: i32, z: i32) -> Option<u64>;
    /// `cISC4Lot::AsPropertyHolder()` (slot 3); 0 = none.
    fn lot_holder(&self, lot: u64) -> u64;
    /// `cISC4Lot::GetBoundingRect()` (slot 14, inclusive cells).
    fn lot_bounding_rect(&self, lot: u64) -> CellRect;
    /// `GetOccupantsByStandardCityCells` (occupant manager slot 40, the filter
    /// overload with a null filter and all flags) over an inclusive cell
    /// rectangle (filter 2, §4.10.3).
    fn occupants_in_cells(&self, rect: CellRect) -> Vec<u64>;
    /// `cISC4OccupantFilter::IsOccupantIncluded(occupant)` (slot 3).
    fn occupant_filter_includes(&self, filter: u64, occupant: u64) -> bool;
    /// `cISC4OrdinanceSimulator::IsOrdinanceOn(id)` (slot 8).
    fn is_ordinance_on(&self, id: u32) -> bool;
}

/// The context of a pathfinder operation: the network model's state (read,
/// and for `TransitSwitchEntryCost`, the edge densities and switch usage,
/// written) and the host.
pub struct PathCx<'a> {
    pub net: &'a mut NetworkState,
    pub host: &'a dyn PathfinderHost,
}

/// The process-wide work-check statics (§4.11, §2): the multiplier `m`
/// (1…1024), the adaptation counter `a` and the one-time-init guard.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkCheck {
    /// The one-time-init guard: false until the first `FindPath`.
    pub initialised: bool,
    /// The work-check interval multiplier `m` (1024 after the first call).
    pub multiplier: u32,
    /// The adaptation counter `a`.
    pub adaptation: i32,
}

/// The work-check statics shared by every pathfinder (§4.11). The owner
/// creates one and hands it to each pathfinder it creates.
pub type SharedWorkCheck = Rc<Cell<WorkCheck>>;

/// A fresh set of work-check statics (as at process start).
pub fn shared_work_check() -> SharedWorkCheck {
    Rc::new(Cell::new(WorkCheck::default()))
}

/// Whether (x, z) is inside the inclusive rectangle `r` (`0x0043FDE0`).
pub fn rect_contains(r: CellRect, x: i32, z: i32) -> bool {
    x >= r.x1 && x <= r.x2 && z >= r.z1 && z <= r.z2
}

/// `EdgeTo(a → b)` (§4.1): the side of `b` relative to a predecessor `a`;
/// the traffic network's `SideOf` is the same rule.
pub fn edge_to(a: (i32, i32), b: (i32, i32)) -> usize {
    crate::traffic_network::side_of(a, b)
}
