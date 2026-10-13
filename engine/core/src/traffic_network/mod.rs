// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The traffic network model (`docs/specs/traffic-network.md`): the cell
//! graph the traffic simulator keeps, and everything built on it. Section
//! references (`§n`) point at that spec unless they name another one.
//!
//! This module is the **network data model and its core operations**. It is
//! internal to `cSC4TrafficSimulator` in the game (§9: the two classes are
//! replaced together), so FreeCity4 keeps it as a plain data model the future
//! traffic simulator owns. The trip generation, work cycle, tuning exemplar,
//! save record and volume/congestion maps belong to `traffic-simulator.md`
//! (a later lane); the A* search belongs to `pathfinder.md`. This module
//! serves both through the accessors they read (network info, the cell
//! connection array, the transit-switch and tunnel tables, the ferry tables,
//! the edge-density map, the subnetwork labels) and the rebuild that keeps
//! them current.
//!
//! # Structure
//!
//! - [`NetworkModel`]: the state of §2 and the operations of §4. The cell
//!   connection array (§2.1), the per-cell network-info lists (§2.2), the
//!   transit-switch, tunnel and ferry tables, the subnetwork labels and info,
//!   the edge-density map (traffic-simulator.md §4.8), the neighbour
//!   connection edge records (§2.7), the capacity tables (§4.5) and the water
//!   pathfinder (§4.10).
//! - [`CellConnection`] (§2.1) and [`NetworkInfo`] (§2.2): the per-cell
//!   records.
//! - [`NetworkHost`]: everything the model reads from, or does to, the rest
//!   of the game (the city, the occupant and lot managers, network occupants
//!   and their path info, transit-switch and ferry buildings, the terrain and
//!   surface water, the neighbours simulator, property holders, messages).
//!   Game objects are opaque `u64` tokens; 0 is null.
//! - [`tuning`]: the capacity tables and intersection-effect values of §4.5
//!   and §4.6, with the shipped exemplar.
//!
//! # Arithmetic
//!
//! Foundations §18 (24-bit x87): every float add, subtract, multiply, divide
//! and square root is one `f32` rounding ([`crate::x87::r24`]), with integer
//! and `double` operands entering unrounded. `r24(e)` in the spec is exactly
//! that. The capacity round trip (§4.5), the tunnel cost (§4.8), the ferry
//! cost (§4.9.3), the water step cost (§4.10.2) and the switch entry cost
//! (§4.7.3) all reproduce their worked bit patterns this way.
//!
//! # Choices made where the spec is open (`[choice]`)
//!
//! - Out-of-city reads of the cell array, the network-info grid, the switch
//!   grids and the edge-density map return their empty/zero value and
//!   out-of-city writes are ignored (the original does not check bounds; the
//!   pathfinder's off-map switch reads of Q4 are "no switch, no info"). The
//!   cell connection array is the fixed 256 × 256 of §1, indexed by the low
//!   8 bits of each coordinate, so an off-map `x = −1` wraps to row 255 there
//!   as in the original; the network-info grid and the switch grids are
//!   city-sized and clamp instead.
//! - The car-continuity words `priorCar` start at `0xFFFF` (§1 step 4); no
//!   reader sees the difference before the first full rebuild.
//! - `invCapTable` is `1.0` before the first tuning read (§4.5).
//! - The water depth map is private (a core [`crate::grid::SimGrid<u8>`], not
//!   a game grid; §9: "a replacement may keep it private").
//! - The transit-switch ID and mode-mask grids are created through the host's
//!   [`crate::grid::GridProvider`] as the game's grid classes (§9, simgrid
//!   §6.4): they are saved.
//! - The congestion curve and the transit multiplier that
//!   [`NetworkModel::transit_switch_entry_cost`] (§4.7.3) needs are
//!   simulator-owned; the model holds copies the simulator sets.
//! - Lane continuity (§4.4) and the path-info queries it needs are exposed as
//!   host callbacks; with no flagged neighbour the words keep `0xFFFF`.
//! - `GetSubnetworksInRectangle` / `…InRegion` return `true` where the
//!   original returns garbage in AL (§7.1).
//!
//! # Threading (foundations §19)
//!
//! The model holds its state in a `RefCell`, exactly like the other simulator
//! cores (power, crime, plumbing, lot developer). §19.1: the original runs
//! the whole simulation path single-threaded with no locking; §19.3 rule 1:
//! only state the master clock exposes to FreeCity4's diagnostic threads must
//! be `Sync`, and this model is not that state — it is internal to the future
//! traffic simulator, one main-thread agent, reached only through it. No
//! `RefCell` borrow is held across a host callback: every accessor takes the
//! state out with [`NetworkModel::with`] for the duration of a call that may
//! re-enter, so a re-entrant call sees an empty state and its changes are
//! lost, but nothing panics.

mod ferries;
mod model;
mod neighbours;
mod rebuild;
mod subnetworks;
mod switches;
mod tunnels;
pub mod tuning;
mod water;

#[cfg(test)]
mod tests;

pub use model::{
    CellConnection, EdgeDensityRecord, NetworkInfo, NetworkModel, NetworkState, TrafficEdgeDensity, EDGE_DENSITY_ZERO,
};
pub use switches::{Rule, TransitSwitch};
pub use tunnels::TunnelInfo;
pub use tuning::Tuning;
pub use water::WaterPathFinder;

use std::rc::Rc;

use crate::grid::{CellRect, GridProvider, TractGrid};
use crate::variant::Variant;

/// The class ID and the interfaces of the owning traffic simulator (§1). The
/// network model has no class ID of its own.
pub const CLSID_TRAFFIC_SIMULATOR: u32 = 0x6990_C1AA;
/// The IID `QueryInterface` answers on read (foundations §3.6).
pub const IID_TRAFFIC_SIMULATOR: u32 = 0x6802_7F37;

/// The transit-switch grids (§1 step 5, §2.3): a `uint8` mode-mask grid and a
/// `uint32` switch-ID grid, both created through GZCOM and saved.
pub const CLSID_SWITCH_MODE_GRID: u32 = 0x69D5_C424;
pub const CLSID_SWITCH_ID_GRID: u32 = 0x69D5_C42D;
/// The water depth map's grid type and instance ID (§4.10.1).
pub const CLSID_WATER_DEPTH_GRID: u32 = 0x49B9_E602;
pub const WATER_DEPTH_INSTANCE: u32 = 0xCBDF_6BF8;

/// Messages the network part handles (§3) and posts (§5).
pub mod msg {
    /// Occupant inserted / removed (data3 = the network-occupant type).
    pub const OCCUPANT_INSERTED: u32 = 0x99EF_1142;
    pub const OCCUPANT_REMOVED: u32 = 0x99EF_1143;
    /// Lot state changed (data1 = 3 deactivated, else data2 = 3 activated).
    pub const LOT_STATE_CHANGED: u32 = 0xE8BD_A2E7;
    /// Post city init: the full rebuild (§1).
    pub const POST_CITY_INIT: u32 = 0x26D3_1EC1;
    /// Neighbour connection added / removed (traffic-simulator.md).
    pub const NEIGHBOUR_ADDED: u32 = 0x8A0D_15DA;
    pub const NEIGHBOUR_REMOVED: u32 = 0xEA0D_163E;
    /// Terrain change (the water pathfinder's depth map, §4.10.1).
    pub const TERRAIN_CHANGED: u32 = 0x278E_6003;
    pub const TERRAIN_CHANGED_ALT: u32 = 0x678E_6E3A;
    /// Posted when the periodic ferry pass has finished all terminals (§5).
    pub const FERRY_ROUTES_DONE: u32 = 0x2C06_BA82;
}

/// Occupant and interface types the network asks for (§3, §8).
pub mod occupant_type {
    /// A network occupant (`cISC4NetworkOccupant`).
    pub const NETWORK: u32 = 0xC772_BF98;
    /// A subway access-point occupant (§4.7.1 step 3).
    pub const SUBWAY_ACCESS: u32 = 0x8A13_918A;
    /// A prop occupant (the ferry markers, §4.9.1).
    pub const PROP: u32 = 0xA823_821E;
}

/// Interface IDs the network `QueryInterface`s (§3, §8, §9).
pub mod iid {
    /// `cISC4NetworkOccupant`.
    pub const NETWORK_OCCUPANT: u32 = 0xA821_EF94;
    /// `cSC4NetworkTunnelOccupant` (§4.8: a tunnel).
    pub const TUNNEL_OCCUPANT: u32 = 0x8A4B_D52B;
    /// `cISC4PropOccupant` (the ferry markers).
    pub const PROP_OCCUPANT: u32 = 0xE979_3A65;
    /// `cISC4Lot` (the lot-state message's copy).
    pub const LOT: u32 = 0xE818_FCED;
}

/// Ferry marker prop types (§4.9.1 step 2).
pub mod ferry_prop {
    pub const DEPARTURE: u32 = 0x2C52_0000;
    pub const ARRIVAL: u32 = 0x2C53_0000;
    pub const WATER: u32 = 0x2C54_0000;
}

/// Building / lot properties the network reads (§2.4, §4.7, §4.9).
pub mod prop {
    /// Transit-switch rules (a u32 array, §4.7.2).
    pub const SWITCH_RULES: u32 = 0xE90E_25A1;
    /// Transit-switch time factor (f32).
    pub const SWITCH_TIME_FACTOR: u32 = 0xE90E_25A2;
    /// Transit-switch capacity (f32).
    pub const SWITCH_CAPACITY: u32 = 0xE90E_25A3;
    /// Simulator-use u32 (§2.4).
    pub const SWITCH_SIMULATOR: u32 = 0x0BFC_0A4C;
    /// "Counts as intersection" bool (§4.6).
    pub const COUNTS_AS_INTERSECTION: u32 = 0x4BFC_47B0;
    /// Ferry type (u8, §4.9.1).
    pub const FERRY_TYPE: u32 = 0x0BD8_6FD3;
    /// A lot's subnetwork list (u32 array, §4.12/§4.13).
    pub const LOT_SUBNETWORKS: u32 = 0x2A4F_9C74;
    /// Trip capacity array (12 u16, traffic-simulator.md §4.2).
    pub const TRIP_CAPACITY: u32 = 0x8A0B_4100;
}

/// Network-occupant flags the rebuild tests (§4.2, §4.4, §10 Q3).
pub mod network_flag {
    /// Sets network-info record flag bit 0 (meaning unknown, carried).
    pub const RECORD_FLAG: u32 = 0x0004_0000;
    /// Excluded from the graph.
    pub const EXCLUDED: u32 = 0x0020_0000;
    /// A preview / non-final piece: the simulator's subway skip (§3).
    pub const PREVIEW: u32 = 0x8000_0000;
    /// The lane-continuity neighbour mask (§4.4).
    pub const LANE_CONTINUITY: u32 = 0x0000_1F4F;
    /// "Has path info" is implied by the presence of a path-info object.
    pub const NO_PATH_INFO: u32 = 0x0000_2000;
}

/// Wormhole byte bits (§2.1).
pub mod wormhole {
    /// Subway station: read by the pathfinder, **never set** (§10 Q1).
    pub const SUBWAY: u8 = 0x01;
    /// Tunnel portal (§4.8).
    pub const TUNNEL: u8 = 0x02;
    /// Ferry terminal (§4.9).
    pub const FERRY: u8 = 0x04;
}

/// Travel types `t` (§2.1). The type constants are `usize` (array indices);
/// `COUNT` is the number of types.
pub mod travel {
    pub const WALK: usize = 0;
    pub const CAR: usize = 1;
    pub const BUS: usize = 2;
    pub const PASSENGER_TRAIN: usize = 3;
    pub const FREIGHT_TRUCK: usize = 4;
    pub const FREIGHT_TRAIN: usize = 5;
    pub const SUBWAY: usize = 6;
    pub const ELEVATED_TRAIN: usize = 7;
    pub const MONORAIL: usize = 8;
    pub const COUNT: usize = 9;
}

/// Transit networks `n` (§2.1).
pub mod net {
    pub const PEDESTRIAN: u32 = 0;
    pub const CAR: u32 = 1;
    pub const TRAIN: u32 = 2;
    pub const SUBWAY: u32 = 3;
    pub const LIGHT_RAIL: u32 = 4;
    pub const MONORAIL: u32 = 5;
    pub const COUNT: usize = 6;
}

/// Network types (`eNetworkType`, §2.2).
pub mod net_type {
    pub const ROAD: u32 = 0;
    pub const RAIL: u32 = 1;
    pub const HIGHWAY: u32 = 2;
    pub const STREET: u32 = 3;
    pub const WATER_PIPE: u32 = 4;
    pub const POWER_POLE: u32 = 5;
    pub const AVENUE: u32 = 6;
    pub const SUBWAY: u32 = 7;
    pub const LIGHT_RAIL: u32 = 8;
    pub const MONORAIL: u32 = 9;
    pub const ONE_WAY_ROAD: u32 = 10;
    pub const DIRT_ROAD: u32 = 11;
    pub const GROUND_HIGHWAY: u32 = 12;
    pub const COUNT: usize = 13;
    /// The "no type matched" sentinel of §4.2 step 3 (not a real type).
    pub const NONE: u32 = 0xFF;
}

/// Sides / directions (§2.1): 0 west (−x), 1 north (−z), 2 east (+x),
/// 3 south (+z).
pub mod side {
    pub const WEST: usize = 0;
    pub const NORTH: usize = 1;
    pub const EAST: usize = 2;
    pub const SOUTH: usize = 3;
    pub const COUNT: usize = 4;
}

/// The neighbour offset of each side (§2.1): `DX = {−1, 0, 1, 0}`,
/// `DZ = {0, −1, 0, 1}`.
pub const DX: [i32; 4] = [-1, 0, 1, 0];
pub const DZ: [i32; 4] = [0, -1, 0, 1];

/// The opposite side `(d + 2) mod 4` (§4.1 table `0x00AB3394` = {2, 3, 0, 1}).
pub const OPPOSITE_SIDE: [usize; 4] = [2, 3, 0, 1];

/// The switch rule rotation `ROT[facing][i]` (§4.1, table `0x00AB3280`).
pub const ROT: [[usize; 4]; 4] = [
    [2, 3, 0, 1], // facing 0
    [1, 2, 3, 0], // facing 1
    [0, 1, 2, 3], // facing 2
    [3, 0, 1, 2], // facing 3
];

/// `net(t)` for t = 0…8 (§2.1, table `0x00AB31A8`).
pub const NET_OF_TRAVEL: [u32; 9] = [0, 1, 1, 2, 1, 2, 3, 4, 5];

/// `pt(n)`, the path type used to ask occupants, for n = 0…5 (§2.1, table
/// `0x00AB3214`).
pub const PATH_TYPE_OF_NET: [u32; 6] = [2, 1, 3, 4, 6, 7];

/// `rep(n)`, the representative travel type, for n = 0…5 (§2.1, table
/// `0x00AB3244`).
pub const REP_OF_NET: [u32; 6] = [0, 1, 3, 6, 7, 8];

/// Whether a network type takes part in traffic (§4.1, table `0x00AB3164`):
/// every type but WaterPipe (4) and PowerPole (5).
pub const TAKES_PART: [bool; 13] = [
    true, true, true, true, false, false, true, true, true, true, true, true, true,
];

/// The transit-network mask per network type (§4.1, table `0x00AB3174`): bit
/// `n` set = the type carries transit network `n`.
pub const TRANSIT_MASK: [u32; 13] = [
    0x03, // Road: pedestrian, car
    0x04, // Rail: train
    0x02, // Highway: car
    0x03, // Street: pedestrian, car
    0x00, // WaterPipe
    0x00, // PowerPole
    0x03, // Avenue: pedestrian, car
    0x08, // Subway
    0x10, // LightRail
    0x20, // Monorail
    0x03, // OneWayRoad: pedestrian, car
    0x03, // DirtRoad: pedestrian, car
    0x02, // GroundHighway: car
];

/// The ferry travel mask by ferry type (§4.1, table `0x00AB32C8`): type 0 =
/// Walk, type 1 = Walk and Car. Type 2 reads past the table and gets
/// `0xD70A` *[quirk]* (§2.6).
pub const FERRY_MASK: [u16; 2] = [1, 3];
/// The travel mask a ferry type-2 terminal reads past the table (§2.6).
pub const FERRY_MASK_TYPE2: u16 = 0xD70A;

/// The tunnel travel-type mask: every type but Walk (§4.8 step 3) *[quirk]*.
pub const TUNNEL_MASK: u16 = 0x01FE;

/// The tunnel network-type preference order (§4.8 step 2): the first the
/// tunnel `IsOfType`.
pub const TUNNEL_TYPE_ORDER: [u32; 7] = [2, 6, 0, 1, 10, 12, 9];

/// The default damaged-road extra step cost (§2.3, §4.5).
pub const DEFAULT_DAMAGED_ROAD_COST: f32 = 32.0;

/// The `1.7014117e38` (`0x7EFFFFFF`) "unreachable" switch entry cost (§4.7.3).
pub const SWITCH_COST_BLOCKED: f32 = f32::from_bits(0x7EFF_FFFF);

/// The largest city side the model allocates for its city-sized grids
/// ([choice]: a lying host cannot make it allocate more; the cell array is
/// the fixed 256 × 256 regardless).
pub const MAX_CITY_SIDE: i32 = crate::grid::MAX_TRACTS;

/// The side of cell `b` that faces cell `a` (§4.1 `SideOf`, the same rule as
/// pathfinder.md's `EdgeTo`): `a.x < b.x → 0`, `a.x > b.x → 2`, otherwise
/// `a.z ≥ b.z → 3`, else `1`. Equal cells give 3.
pub fn side_of(a: (i32, i32), b: (i32, i32)) -> usize {
    if a.0 < b.0 {
        side::WEST
    } else if a.0 > b.0 {
        side::EAST
    } else if a.1 >= b.1 {
        side::SOUTH
    } else {
        side::NORTH
    }
}

/// The cell key `(x << 8) | z`, using only the low 8 bits of each coordinate
/// (§2.3 "Keys").
pub fn cell_key(x: i32, z: i32) -> u16 {
    (((x & 0xFF) << 8) | (z & 0xFF)) as u16
}

/// The cell connection array index of a cell key (§1: `(x << 8) | z`).
pub fn array_index(x: i32, z: i32) -> usize {
    usize::from(cell_key(x, z))
}

/// A grid held by the model (the transit-switch grids).
pub type GridU32 = Rc<dyn TractGrid<u32>>;
pub type GridU8 = Rc<dyn TractGrid<u8>>;

/// A path-info object of a network occupant (§4.2, §4.4), as a host token.
pub type PathInfo = u64;

/// Everything the network model reads from, or does to, the rest of the game
/// (§8). Coordinates are city cells. Game objects are opaque `u64` tokens;
/// 0 is null. Host callbacks should not call back into the model (a
/// re-entrant call sees an empty state and its changes are lost, but nothing
/// panics).
///
/// The supertraits create the two saved transit-switch grids (simgrid §6.4:
/// in game the plugin makes the game's own `cSC4SimGrid<uint32>` and
/// `cSC4SimGrid<uint8>`).
pub trait NetworkHost: GridProvider<u32> + GridProvider<u8> {
    // ---- city ----
    /// `cISC4City::CellCountX()` / `CellCountZ()` (slots 93, 94).
    fn cell_counts(&self) -> (i32, i32);
    /// `cISC4City::CellWidthX()` (slot 91), for the inverse-speed table.
    fn cell_width_x(&self) -> f32;
    /// `cISC4City::PositionToCell(x, z, &cx, &cz)` (slot 95).
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32);

    // ---- occupants ----
    /// `GetOccupantsByStandardCityCells(list, &nw, &se, 0xC772BF98, −1)`
    /// (occupant manager compiled slot 41): the network occupants in a cell
    /// rectangle, in the manager's list order.
    fn network_occupants_in_rect(&self, rect: CellRect) -> Vec<u64>;
    /// `GetOccupantsByStandardCityCells(list, &nw, &se, 0x8A13918A, −1)`: the
    /// subway access-point occupants in a rectangle (§4.7.1 step 3).
    fn subway_occupants_in_rect(&self, rect: CellRect) -> Vec<u64>;
    /// `GetOccupantsByStandardCityCells(list, &nw, &se, 0xA823821E, −1)` then
    /// `QueryInterface(0xE9793A65)`: the prop occupants in a rectangle
    /// (§4.9.1 step 2).
    fn prop_occupants_in_rect(&self, rect: CellRect) -> Vec<u64>;
    /// `cISC4Occupant::GetPosition()` (x, y, z) (slot 8).
    fn occupant_position(&self, occ: u64) -> [f32; 3];
    /// `cISC4Occupant::GetBoundingCityCells()` (inclusive, slot 11).
    fn occupant_bounding_cells(&self, occ: u64) -> CellRect;
    /// `cISC4PropOccupant::GetPropType()` (slot 7).
    fn prop_type(&self, occ: u64) -> u32;
    /// A property of the occupant's holder (own or from its exemplar);
    /// `None` when absent.
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant>;

    // ---- network occupants (§7.2, §8) ----
    /// `HasNetworkFlag(flag)` (slot 8).
    fn has_network_flag(&self, occ: u64, flag: u32) -> bool;
    /// `HasAnyNetworkFlag(mask)` (slot 9).
    fn has_any_network_flag(&self, occ: u64, mask: u32) -> bool;
    /// `IsOfType(networkType)` (slot 22).
    fn is_of_type(&self, occ: u64, network_type: u32) -> bool;
    /// `PathConnection(pathType, entry, exit)` (slot 53): with `0xFF`
    /// wildcards the original returns a count, otherwise a bool; the rebuild
    /// only tests concrete `(entry, exit)` pairs, so this is a bool.
    fn path_connection(&self, occ: u64, path_type: u32, entry: u32, exit: u32) -> bool;
    /// `GetPathInfo()` (slot 49); `None` (0) when there is none.
    fn path_info(&self, occ: u64) -> Option<PathInfo>;
    /// Path-info slot 4 `(p, e, 0xFF, 0, 0)`: non-zero when the piece allows
    /// a turn-around at entry `e` (§4.2 step 3).
    fn path_info_turn_around(&self, pi: PathInfo, path_type: u32, entry: u32) -> bool;
    /// Path-info slot 14: the exit bytes of the paths of `path_type` that
    /// start at entry `e`, paired with each path's first-vertex handle
    /// (slot 3) as a token (§4.4 step 1).
    fn path_info_exits_from_entry(&self, pi: PathInfo, path_type: u32, entry: u32) -> Vec<(u8, u64)>;
    /// Path-info slot 7 `(path_type, vertex, 0xFF, exit, radius)`: the entry
    /// sides of the paths of `path_type` that end within `radius` of `vertex`
    /// (§4.4 step 2). `vertex` is the first-vertex handle from
    /// [`NetworkHost::path_info_exits_from_entry`].
    fn path_info_entries_near(&self, pi: PathInfo, path_type: u32, vertex: u64, exit: u32, radius: f32)
        -> Vec<u32>;
    /// The paired portal of a `cSC4NetworkTunnelOccupant` (§9: a member of the
    /// concrete class, reached into). `None` when `occ` is not a tunnel or has
    /// no pair.
    fn tunnel_paired_portal(&self, occ: u64) -> Option<u64>;
    /// `QueryInterface(0x8A4BD52B)` succeeds (the occupant is a tunnel).
    fn is_tunnel(&self, occ: u64) -> bool;

    // ---- lots (§8) ----
    /// `cISC4LotManager::GetLot(x, z, anchorOnly)` (slot 6).
    fn lot_at(&self, x: i32, z: i32, anchor_only: bool) -> Option<u64>;
    /// `cISC4Lot::GetBoundingRect()` (inclusive cells, slot 14).
    fn lot_bounding_rect(&self, lot: u64) -> CellRect;
    /// `cISC4Lot::GetLocation(&x, &z)` (slot 21).
    fn lot_location(&self, lot: u64) -> (i32, i32);
    /// `cISC4Lot::GetFacing()` (slot 23).
    fn lot_facing(&self, lot: u64) -> u32;
    /// `cISC4Lot::GetBuilding()` (slot 48).
    fn lot_building(&self, lot: u64) -> Option<u64>;
    /// `cISC4Lot::GetPopulations()` (slot 54): a vector of
    /// `map<developerType, count>`.
    fn lot_populations(&self, lot: u64) -> Vec<Vec<(u32, u32)>>;
    /// `cISC4Lot::AsPropertyHolder()` (slot 3).
    fn lot_holder(&self, lot: u64) -> u64;

    // ---- property holders (§8) ----
    /// A property of a holder; `None` when absent.
    fn holder_property(&self, holder: u64, id: u32) -> Option<Variant>;
    /// `AddProperty(id, value)` on a holder.
    fn set_holder_property(&self, holder: u64, id: u32, value: Variant);
    /// `RemoveProperty(id)` (slot 17); whether it was present.
    fn remove_holder_property(&self, holder: u64, id: u32) -> bool;

    // ---- terrain and surface water (§8) ----
    /// `cISTETerrain::CellHasWater(x, z)` (slot 34).
    fn cell_under_water(&self, x: i32, z: i32) -> bool;
    /// `cISTESurfaceWater::IsNavigable(x, z, radius, lot)` (slot 8).
    fn is_navigable(&self, x: i32, z: i32, radius: f32) -> bool;
    /// `cISTESurfaceWater::GetWaterDepthForCell(x, z)` (slot 6, x87).
    fn water_depth(&self, x: i32, z: i32) -> f32;
    /// `cISTESurfaceWater::GetBodyOfWaterMap()` (slot 9) value at a cell: the
    /// body-of-water ID, or `None` without the map.
    fn body_of_water(&self, x: i32, z: i32) -> Option<u16>;

    // ---- neighbours (§8) ----
    /// `cISC4NeighborsSimulator::GetConnectionsOfType(type)` (slot 17), in
    /// list order.
    fn connections_of_type(&self, connection_type: u32) -> Vec<u64>;
    /// `cISC4NeighborsSimulator::GetAllConnections()` (slot 16), in list
    /// order.
    fn all_connections(&self) -> Vec<u64>;
    /// `cISC4NeighborConnection::GetConnectionCell(&x, &z)` (slot 6).
    fn connection_cell(&self, conn: u64) -> (i32, i32);

    // ---- simulator-owned values the network reads ----
    /// `IsRoadDamaged(x, z)` (slot 16): the cell key is in the simulator's
    /// pothole table (§4.15).
    fn is_road_damaged(&self, x: i32, z: i32) -> bool;

    // ---- framework ----
    /// Posts a standard message (§5). The network posts only
    /// [`msg::FERRY_ROUTES_DONE`].
    fn post(&self, msg_type: u32, data1: u32, data2: u32, data3: u32);
    /// `AddRef` / `Release` of a host object the model holds (a transit-switch
    /// building).
    fn add_ref(&self, token: u64);
    fn release(&self, token: u64);
}
