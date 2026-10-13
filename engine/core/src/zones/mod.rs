// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The zone manager (`cSC4ZoneManager`, class `0x298F9B2D`) and the zone
//! developer / lot manager (`cSC4ZoneDeveloper`, class `0x498F9B01`),
//! `docs/specs/zones.md`. Section references (`§n`) point at that spec.
//!
//! # Structure
//!
//! - [`ZoneManager`] (`manager.rs`): the zone grid and its counts, the
//!   zoning tuning (§4), `PlaceZone` (§6), the dezone under new networks
//!   (§5.1), the queries of §11.1 and its save record (§10.1).
//! - [`ZoneDeveloper`] (`developer.rs`, `parcel.rs`): the lot manager (the
//!   cell → lot map, the master lot list, lot creation and deletion, the
//!   lookups and lot lists of §8, the safe lot iterators, its save record
//!   §10.2) and the zone developer (parcellization, §7: `DoParcellization`,
//!   `HighlightParcels`, `OnZone`, `DetermineLotSize`, `ExamineBoundaries`,
//!   `CreateParcellizableRegion`, `SimpleParcellize`, `Parcellize`,
//!   `CreateLotToFaceRoad`, `CreateLotRemnants`, `OnDezone`), and its
//!   messages (§5.2).
//! - [`ZoneDeveloperStatics`]: the zone developer's file-local `cRZRandom`
//!   (`0x00B4B398`, §9), one object shared by every instance and by whoever
//!   else needs it (the plugin glue hands the same [`SharedZoneStatics`] to
//!   the tract developer's host, which models R9 / Z2c on it), with a draw
//!   trace ([`DrawKind`]).
//! - [`CellRegion`] (`region.rs`): `SC4CellRegion<long>` (§11.4).
//! - [`ZonesHost`]: everything outside the two classes. Objects (lots,
//!   lot configurations, occupants, network tiles, growth developers) are
//!   host tokens (`u64`). **The two classes reach each other through the
//!   host too** (`ZonesHost::lot_manager_get_lot`,
//!   `ZonesHost::zone_developer_do_parcellization`, `ZonesHost::zone_type`,
//!   …), as the original does through its global pointers (§13.1); a host
//!   wires them to a [`ZoneManager`] and a [`ZoneDeveloper`]. Both classes
//!   are therefore re-entrant: every method takes `&self` and no internal
//!   borrow is held across a host call.
//!
//! # Headless limits, left-outs and placeholders
//!
//! - **No 3D view** *[headless-limited]* (§1.2 step 2, §13): the original
//!   `Init` fails without the 3D-view global; this one does not take it.
//!   The cached view value, `InitBoundaryRules` (lot-boundary textures),
//!   the highlight textures, the cheats "PlopAllLots" / "PlopAllBuildings"
//!   (§1.2 step 7, §5.2) and `CanCreatePloppedLot` (slot 21, §8.4) are UI or
//!   debug and are left out. The `0xA6B79602` value is stored
//!   ([`ZoneDeveloper::view_message_value`]) but unused.
//! - **Street geometry of round 6** (§7.10–§7.13) is in the core: `DrawStreet`
//!   (§7.10), `ExtendAsStreets` (§7.11), `LayInJogStreets` (§7.12) and
//!   `ExistsConnectionOfType` / `GetNetwork` (§7.13) are computed here and
//!   clip to the restriction region. The only host primitives they need are
//!   the street tool's `PlaceNetworkOccupants` ([`ZonesHost::street_place`],
//!   the "Place(a, b)" of §7.10), a network occupant's `ConnectsToNeighbor`
//!   ([`ZonesHost::connects_to_neighbor`], network occupant slot 47) and the
//!   highlight-mode draft mask ([`ZonesHost::network_tool_occupant_exists`],
//!   network tool slot 9). `GetNetwork` and `ExistsNetworkOfType` are derived
//!   from [`ZonesHost::cell_occupants`] and [`ZonesHost::network_has_any_flag`].
//!   A host whose `street_place` inserts nothing draws no visible streets; the
//!   core still logs every `Place` call in order.
//! - **Untraced helpers** (§5.2 step 3, §7.2 step 7): the street-joining
//!   helpers `0x00731F60` / `0x00732450` are one host call
//!   ([`ZonesHost::street_joining_helper`]); `0x0072B180` at the end of
//!   `OnZone` is left out (no behaviour is specified; no RNG).
//!
//! # Choices (`[choice]`), limits (`[compat-limit]`) and inferences (`[inferred]`)
//!
//! - **Return values** where the original leaves EAX undefined (§11.3:
//!   slots 11, 24, 25, 26) are true (§11.3 [choice]).
//! - **Out-of-range indexes** where the original has no bounds check
//!   (`GetZoneCount`, `GetZoneType`, `GetMinZoneSize`, `GetLotSavvyCellAltitude`
//!   on the map) read 0 / null; `GetZoneType` outside the grid reads 0.
//! - **Tuning** (§4): array properties take their first `min(16, count)`
//!   elements converted by value (the original reinterprets 16 elements
//!   through `Ref*` with no count or type check); missing elements keep
//!   their previous value. Sizes start at 0 when the exemplar is missing
//!   (uninitialised in the original).
//! - **`PlaceZone`** (§6): the outputs are always returned
//!   ([`PlaceZoneResult`]); a type ≥ 16 fails with error 0 and cost 0. After
//!   an `ABORT`, the funds check still runs and can replace the error code
//!   with 1 (the pseudo-code's order) *[inferred]*. The commit
//!   `DemolishLots(true, …)` is called whenever cells were accepted, also
//!   with an empty lot list (as written). `outLots` is a set ordered by token
//!   *[compat-limit]* (the original orders by address).
//! - **`GetLotsInCellRect(…, false)`** (§8.3): both neighbour comparisons
//!   (`(x, z−1)` and `(x−1, z)`) look only inside the clamped rectangle
//!   *[inferred]*, so a lot crossing the rectangle's low edge is listed.
//! - **`GetLotsSurroundingRect`** (§8.3) walks the ring row by row
//!   (z outer, x inner) *[inferred]*: row `z1−1`, then the two side cells of
//!   each row `z1..z2`, then row `z2+1`.
//! - **`CreateLotRemnants`** (§7.8), zones other than 7–9: each strip is a
//!   line of cells running away from the facing side, one cell wide (facing
//!   3: for each z ascending, x ascending; 1: z ascending, x descending; 0:
//!   for each x ascending, z ascending; 2: x ascending, z descending)
//!   *[inferred]*; "buildable" is `CanCreateLot`'s cell code 8. A facing
//!   outside 0–3 picks the scan of `facing mod 4`.
//! - **`CreateParcellizableRegion`** (§7.4) and `OnDezone` (§7.9) visit
//!   cells in the order stated (x outer, z inner for the former
//!   *[inferred]*).
//! - **Components** (§7.2 step 5): seeded in x-outer, z-inner order and
//!   flood-filled; with no component, one empty region (no cells, so
//!   `SimpleParcellize` draws nothing for it) *[inferred]*.
//! - **`HighlightParcels`** (§7.1): `*out` starts as a copy of the cropped
//!   parcellizable region; skipped components clear their cells from it
//!   *[inferred]*. Virtual lots and virtually deleted cells are kept in
//!   two maps until the next highlight.
//! - **`DetermineLotSize`** (§7.2.1) with no best developer (zones 7–9, or
//!   no developer from the BDS) lists no configurations: lot size (2, 2),
//!   depths widened to `[1, 3]` *[inferred]*. A configuration with width 0
//!   is skipped in the depth widening (the original divides by zero). A
//!   missing source demand reads as 0.
//! - **Parcellize** (§7.6) clamps lot width and depth to at least 1 (a
//!   width of 0 never ends the row walk) and stops a walk after
//!   [`MAX_WALK_STEPS`] positions.
//! - **Lot insertion** (§8.5 step 4): with a desired insertion point whose
//!   list is set, the lot is inserted after its node (appended when the node
//!   is the end or not in the list) and the host is told the iterator's new
//!   node, the node's predecessor (`None` = the head / end).
//! - **Safe lot iterators** (§8.3 slot 15): handles registered with the
//!   manager; [`ZoneDeveloper::free_safe_lot_iterator`] unregisters one
//!   (freeing is not traced in the original, §14.3).
//! - **Shutdown** releases each lot of the map once (first sighting, x outer,
//!   z inner) *[inferred]* and empties the map and the master list.
//! - **Save read** (§10.1, §10.2): a stream error fails the read and keeps
//!   the previous state; references already read are released. Map sides
//!   above [`region::MAX_REGION_SIDE`] fail the read.
//! - **Pointer-ordered containers** (`outLots`, the dedupe `hash_set` of the
//!   zone developer's `Read`) are ordered by token *[compat-limit]*; only
//!   membership is observable.

mod developer;
mod manager;
mod parcel;
mod region;
#[cfg(test)]
mod tests;

pub use developer::{
    CreateResult, Draw, DrawKind, InsertionPoint, LotSize, Options, Orientation, SharedZoneStatics, VirtualLot,
    ZoneDeveloper, ZoneDeveloperStatics, DEVELOPER_RECORD_VERSION, MAX_RANDOM_LOT_TRIES, RNG_SEED_XOR, TEST_SEEDS,
};
pub use manager::{
    shipped_tuning, PlaceZoneResult, Tuning, ZoneManager, MANAGER_RECORD_VERSION, TEXTURE_KEY_GROUP, TEXTURE_KEY_TYPE,
    TUNING_KEY,
};
pub use parcel::{block_depth, depth_rule, zone_density, MAX_WALK_STEPS};
pub use region::{CellRegion, MAX_REGION_SIDE};

use std::rc::Rc;

pub use crate::development::{BuildingProfile, GrowthItem};
pub use crate::grid::{CellRect, GridProvider, TractGrid};
use crate::property::PropertyHolder;
pub use crate::stream::RefValue;

/// Class IDs (§1).
pub const CLSID_ZONE_MANAGER: u32 = 0x298F_9B2D;
pub const CLSID_ZONE_DEVELOPER: u32 = 0x498F_9B01;
/// Interface IDs (§1).
pub const IID_UNKNOWN: u32 = 0x0000_0001;
pub const IID_ZONE_MANAGER: u32 = 0x6765_2433;
pub const IID_ZONE_DEVELOPER: u32 = 0xE818_ED2B;
pub const IID_LOT_MANAGER: u32 = 0x6819_05BD;
pub const IID_MESSAGE_TARGET2: u32 = 0x4522_94AA;
pub const IID_SERIALIZABLE: u32 = 0xE4FD_A3D4;
/// IIDs used when reading references (§10).
pub const IID_SIM_GRID: u32 = 0xC9BD_939A;
pub const IID_LOT: u32 = 0xE818_FCED;
pub const IID_CITY: u32 = 0x66BA_4DB5;
pub const IID_OCCUPANT_MANAGER: u32 = 0xC6BB_8334;
pub const IID_TERRAIN: u32 = 0xC70B_D37E;
pub const IID_POLLUTION: u32 = 0x8771_81EC;
pub const IID_BUDGET: u32 = 0x6772_BD23;

/// Whether the zone manager's `QueryInterface` answers `iid` (§1).
pub fn manager_answers_iid(iid: u32) -> bool {
    matches!(
        iid,
        IID_UNKNOWN | IID_ZONE_MANAGER | IID_MESSAGE_TARGET2 | IID_SERIALIZABLE
    )
}

/// Whether the zone developer's `QueryInterface` answers `iid` (§1).
pub fn developer_answers_iid(iid: u32) -> bool {
    matches!(
        iid,
        IID_UNKNOWN | IID_ZONE_DEVELOPER | IID_LOT_MANAGER | IID_MESSAGE_TARGET2 | IID_SERIALIZABLE
    )
}

/// Game grid classes (simgrid §2): the zone grid (`int8`) and the
/// maximum-capacity grid (`uint16`).
pub const ZONE_GRID_CLASS: u32 = 0x49B9_E603;
pub const MAX_CAPACITY_GRID_CLASS: u32 = 0x49B9_E604;
/// Instance ID of the maximum-capacity grid (§1.2 step 4).
pub const MAX_CAPACITY_GRID_INSTANCE: u32 = 0x69D5_C456;

/// Messages (§5).
pub const MSG_INSERT_OCCUPANT: u32 = 0x99EF_1142;
pub const MSG_POST_CITY_INIT: u32 = 0x26D3_1EC1;
pub const MSG_VIEW: u32 = 0xA6B7_9602;
pub const MSG_CHEAT: u32 = 0x230E_27AC;
pub const MSG_ZONING_CHANGE: u32 = 0x0769_9455;
pub const MSG_DEFER_OCCUPANT_OPERATIONS: u32 = 0x49D9_68C4;
/// The network occupant type (InsertOccupant data3, §5.1).
pub const NETWORK_OCCUPANT_TYPE: u32 = 0xC772_BF98;
/// Network flags that keep an occupant from dezoning (§5.1): bits 4, 7, 21,
/// 31.
pub const NO_DEZONE_FLAGS: u32 = 0x8020_0090;
/// The landfill occupant type (§6).
pub const LANDFILL_TYPE: u32 = 0x6A11_BD2B;
/// Occupant types counted as obstacles by `ExamineCell` (§7.3).
pub const BUILDING_TYPE: u32 = 0x2781_28A0;
pub const OBSTACLE_TYPE_2: u32 = 0x2890_D4DE;

/// Network masks (`HasAnyNetworkFlag`; bit n = network type n).
pub mod net {
    /// Any network (§6, §7.4, §8.4).
    pub const ANY: u32 = 0x1F4F;
    /// Road, street, avenue, one-way, dirt road (§7.7).
    pub const ROADS: u32 = 0xC49;
    /// Highway, light rail (§7.3).
    pub const HIGHWAY_LIGHT_RAIL: u32 = 0x104;
    /// Street, dirt road (§7.3, §7.9).
    pub const STREET_DIRT: u32 = 0x808;
    /// Road, one-way road (§7.3).
    pub const ROAD_ONE_WAY: u32 = 0x401;
    /// Avenue (§7.3).
    pub const AVENUE: u32 = 0x40;
    /// Rail (§7.3).
    pub const RAIL: u32 = 0x2;
    /// The networks that keep a street from being removed (§7.9).
    pub const KEEP_STREET: u32 = 0x1747;
    /// Street (bit 3) and dirt road (bit 11): the network tools' drafts
    /// (§8.8).
    pub const STREET: u32 = 0x8;
    pub const DIRT: u32 = 0x800;
}

/// Network kinds (network types) used by the street steps (§7.2 step 4)
/// and the network tools (§5.2).
pub mod network_kind {
    pub const ROAD: u32 = 0;
    pub const STREET: u32 = 3;
    pub const AVENUE: u32 = 6;
    pub const ONE_WAY_ROAD: u32 = 10;
    pub const DIRT_ROAD: u32 = 11;
    /// The order of §7.2 step 4.
    pub const STREET_STEP_ORDER: [u32; 5] = [DIRT_ROAD, STREET, ONE_WAY_ROAD, ROAD, AVENUE];
}

/// `PlaceZone` error codes (§6).
pub mod place_error {
    pub const OK: i32 = 0;
    pub const FUNDS: i32 = 1;
    pub const LANDFILL_ON_WATER: i32 = 2;
    pub const TOO_SMALL: i32 = 3;
    pub const TOO_LARGE: i32 = 4;
    pub const BLOCKED: i32 = 5;
}

/// `CanCreateLot` codes (§8.4).
pub mod lot_code {
    pub const OUTSIDE: i32 = 0;
    pub const WATER: i32 = 1;
    pub const NOT_ZONED: i32 = 2;
    pub const NETWORK: i32 = 3;
    pub const OCCUPIED: i32 = 7;
    pub const FREE: i32 = 8;
}

/// `DemolishLots` flags (§6): the pricing call and the commit call.
pub const DEMOLISH_PRICE_FLAGS: u32 = 1;
pub const DEMOLISH_COMMIT_FLAGS: u32 = 0x2000;

/// `IsRCIZone(t)` (§2): true for 1..9.
pub fn is_rci_zone(t: i32) -> bool {
    (1..=9).contains(&t)
}

/// A service whose reference the zone manager saves (§10.1 field 13).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Service {
    City,
    OccupantManager,
    Terrain,
    Pollution,
    Budget,
}

/// Which class a subscription belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Component {
    ZoneManager,
    ZoneDeveloper,
}

/// A message as both `DoMessage`s read it (§5).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ZonesMessage {
    pub id: u32,
    /// void1: the occupant of InsertOccupant.
    pub occupant: Option<u64>,
    pub data1: u32,
    pub data3: u32,
    pub data4: u32,
    /// `*void2` of message `0xA6B79602`.
    pub value: u32,
}

/// A 3D position (`cS3DVector3`): x, y, z.
pub type Position = [f32; 3];

/// Everything the zone manager and the zone developer read from, or do to,
/// the rest of the game (§12). Objects are host tokens. Lot lists the host
/// returns hold one reference per element for the caller where the original
/// AddRefs them (stated per method); the core releases them through
/// [`ZonesHost::release`].
pub trait ZonesHost: GridProvider<i8> + GridProvider<u16> {
    // ---- lifecycle, services and messages (§1, §5) ----
    /// Zone manager `Init` step 2: take the city, occupant manager, terrain,
    /// pollution simulator and budget simulator that are still null, and the
    /// message server.
    fn manager_acquire_services(&self);
    /// Zone manager `Shutdown`: release them.
    fn manager_release_services(&self);
    /// Zone developer `Init` steps 1–2 without the 3D view: the lot
    /// developer, BDS, city, occupant manager, terrain, traffic simulator,
    /// zone manager and message server. False if any is missing.
    fn developer_acquire_services(&self) -> bool;
    fn developer_release_services(&self);
    fn subscribe(&self, who: Component, messages: &[u32]);
    fn unsubscribe(&self, who: Component, messages: &[u32]);
    /// The "Zone Manager" tuning exemplar (§4), if loaded.
    fn tuning(&self) -> Option<Rc<PropertyHolder>>;
    /// City slots 93 / 94.
    fn city_cell_counts(&self) -> (i32, i32);
    /// The reference the zone manager saves for a service (§10.1).
    fn service_ref(&self, service: Service) -> RefValue;
    /// `Shutdown` and `Release` of a grid the class created (§1.1, §1.2).
    fn shutdown_grid(&self, class: u32);
    /// PostCityInit (§5.2): take the street, dirt-road and road network
    /// tools (`GetNetworkTool(type, 1)`, slot 3 each).
    fn acquire_network_tools(&self);
    fn release_network_tools(&self);
    /// ZoningChange (§5.3): `MessagePost` of a message owning `region`.
    fn post_zoning_change(&self, region: CellRegion, zone: i32, check_slope: bool);
    /// DeferOccupantOperations (§5.3), posted.
    fn post_defer_occupant_operations(&self, begin: bool);

    // ---- the other class (§13.1) ----
    /// Zone manager slot 12 `GetZoneType(x, z)`.
    fn zone_type(&self, x: i32, z: i32) -> i32;
    /// Lot manager slot 6 `GetLot(x, z, false)` (no reference).
    fn lot_manager_get_lot(&self, x: i32, z: i32) -> Option<u64>;
    /// Lot manager slot 7 `GetLotsInCellRect(list, rect, originOnly)` into
    /// a new list: one reference per element for the caller.
    fn lot_manager_lots_in_cell_rect(&self, rect: CellRect, origin_only: bool) -> Vec<u64>;
    /// Zone developer slot 6 `DoParcellization(region, type, flag)`.
    fn zone_developer_do_parcellization(&self, region: &CellRegion, zone: i32, flag: bool);

    // ---- budget, demolition, landfill (§6) ----
    /// Budget slot 6.
    fn total_funds(&self) -> i64;
    /// Budget slot 8.
    fn withdraw_funds(&self, amount: i64) -> bool;
    /// Demolition slot 7 `DemolishLots(commit, lots, checkSlope, flags, 0, 0,
    /// &cost, 0)`: the cost (used by the pricing call).
    fn demolish_lots(&self, commit: bool, lots: &[u64], check_slope: bool, flags: u32) -> i64;
    /// Demolition slot 6 `DemolishRegion(1, &region, 1, 1, 0, …)` (§7.9).
    fn demolish_region(&self, region: &CellRegion);
    /// `city.GetLandfillDeveloper().GetZoningCost(region)`.
    fn landfill_zoning_cost(&self, region: &CellRegion) -> i64;

    // ---- networks (§6, §7, §8.8) ----
    /// `city.GetTrafficNetwork().FindNetworkTile(x, z, 0x1F4F, 1)`.
    fn find_network_tile(&self, x: i32, z: i32) -> Option<u64>;
    /// `tile.AsOccupant().IsFlagSet(flag)`.
    fn tile_occupant_flag_set(&self, tile: u64, flag: u32) -> bool;
    /// `tile.IsOfType(t)`.
    fn tile_is_of_type(&self, tile: u64, t: u32) -> bool;
    /// `tile.GetCostOfDemolition()`.
    fn tile_demolition_cost(&self, tile: u64) -> i64;
    /// Network tool slot 9: the street (3) or dirt-road (11) tool's draft
    /// covers the cell (highlight mode, §8.8).
    fn network_tool_draft_at(&self, tool: u32, x: i32, z: i32) -> bool;
    /// `DrawStreet`'s "Place(a, b)" (§7.10): the street tool's slot 7
    /// `PlaceNetworkOccupants(a.x, a.z, b.x, b.z, 0xC49, callback = the street
    /// tool, realMode, −1)`. One street segment from cell `a` to cell `b`
    /// (inclusive); returns AL. In real mode the streets must become visible
    /// to the next `ExistsNetworkOfType` (§7.6.2); in highlight mode they are
    /// drafts (seen through [`ZonesHost::network_tool_draft_at`]). `kind` is
    /// the network kind of the caller (`ExtendAsStreets` / `LayInJogStreets`
    /// pass their kind, `DrawStreet` the street kind 3) so the host can
    /// insert the right occupant type. The restriction region is applied by
    /// the core, so `a`..`b` may include cells outside it only through the
    /// §7.10 fallback chain.
    fn street_place(&self, kind: u32, a: (i32, i32), b: (i32, i32)) -> bool;
    /// A network occupant's slot 47 `ConnectsToNeighbor(edge, type)`
    /// (§7.13): true when the occupant is of network type `type` and has a
    /// connection on `edge` for that type. `edge` is 0..3 (0 = −x, 1 = −z,
    /// 2 = +x, 3 = +z), or −1 for a facing side outside 0..3.
    fn connects_to_neighbor(&self, occupant: u64, edge: i32, net_type: u32) -> bool;
    /// Highlight mode only (§7.13): the street (tool kind 3) or dirt-road
    /// (kind 11) tool's slot 9 `OccupantExists(x, z, flag, &mask)` → `Some(mask)`
    /// when it reports a draft at the cell, `None` otherwise. `mask` is the
    /// four network-edge bytes (bit set per connected edge). `flag` is `0x8`
    /// (street) or `0x800` (dirt road). Not consulted in real mode.
    fn network_tool_occupant_exists(&self, tool_kind: u32, x: i32, z: i32, flag: u32) -> Option<[u8; 4]>;
    /// InsertOccupant step 3 (§5.2): `0x00731F60(occupant)` for data4 1 or
    /// 2, `0x00732450(occupant)` for 3 (not traced; no RNG).
    fn street_joining_helper(&self, occupant: u64, data4: u32);

    // ---- terrain (§12) ----
    /// Terrain slot 34 `CellHasWater` (zone developer slot 9).
    fn cell_has_water(&self, x: i32, z: i32) -> bool;
    /// Terrain slot 13 `GetAltitude(x, z)`.
    fn terrain_altitude(&self, x: f32, z: f32) -> f32;
    /// Terrain slot 14 `GetAltitudeAtVertex(vx, vz)`.
    fn terrain_vertex_altitude(&self, vx: i32, vz: i32) -> f32;

    // ---- city and occupants ----
    /// City slot 98 `LocationIsInBounds`.
    fn location_in_bounds(&self, pos: Position) -> bool;
    /// City slot 95 `PositionToCell` (`None` when it returns false).
    fn position_to_cell(&self, pos: Position) -> Option<(i32, i32)>;
    /// Occupant manager slot 41 (+ slot 48): the occupants of the cell, of
    /// `type_filter` or of every type.
    fn cell_occupants(&self, x: i32, z: i32, type_filter: Option<u32>) -> Vec<u64>;
    /// Occupant manager slot 33: the landfill occupant of the cell.
    fn landfill_at(&self, x: i32, z: i32) -> Option<u64>;
    /// Occupant slot 7 `GetType`.
    fn occupant_type(&self, occupant: u64) -> u32;
    /// Occupant slot 8 `GetPosition`.
    fn occupant_position(&self, occupant: u64) -> Position;
    /// Occupant slot 11 `GetBoundingCityCells`.
    fn occupant_bounding_cells(&self, occupant: u64) -> CellRect;
    /// Occupant slot 14 `CanRemove(flags)` *[inferred name]*.
    fn occupant_can_remove(&self, occupant: u64, flags: u32) -> bool;
    /// `QueryInterface(cISC4NetworkOccupant)` then slot 9
    /// `HasAnyNetworkFlag(flags)`; `None` when the occupant is not a network
    /// occupant.
    fn network_has_any_flag(&self, occupant: u64, flags: u32) -> Option<bool>;
    /// Occupant manager slot 21 `RemoveOccupant(occupant, 1, 0)`.
    fn remove_occupant(&self, occupant: u64);

    // ---- reference counts ----
    fn add_ref(&self, object: u64);
    fn release(&self, object: u64);

    // ---- lots (cISC4Lot, §8) ----
    /// Allocate and construct a lot (`0x006BF430`, `0x006C1E50(x, z, w, h,
    /// facing)`): a new token with no reference yet.
    fn new_lot(&self, x: i32, z: i32, w: i32, h: i32, facing: i32) -> u64;
    /// Slot 5.
    fn lot_init(&self, lot: u64);
    /// Slot 7.
    fn lot_shutdown(&self, lot: u64);
    /// Slot 8.
    fn lot_post_city_init(&self, lot: u64);
    /// Slot 9 `GetLotConfiguration` (no reference).
    fn lot_configuration(&self, lot: u64) -> Option<u64>;
    /// Slot 12 `GetFlags`.
    fn lot_flags(&self, lot: u64) -> u32;
    /// Slot 14 `GetBoundingRect`.
    fn lot_bounding_rect(&self, lot: u64) -> CellRect;
    /// Slot 16 `GetAltitude`.
    fn lot_altitude(&self, lot: u64) -> f32;
    /// Slot 21 `GetLocation`.
    fn lot_location(&self, lot: u64) -> (i32, i32);
    /// Slot 23 `GetFacing`.
    fn lot_facing(&self, lot: u64) -> i32;
    /// Slot 25: whether the lot has a business unit.
    fn lot_has_business_unit(&self, lot: u64) -> bool;
    /// Slot 31 `GetZoneType`.
    fn lot_zone_type(&self, lot: u64) -> i32;
    /// Slot 33 `GetState`.
    fn lot_state(&self, lot: u64) -> u32;
    /// Slot 34 `SetState(state, a, b)`.
    fn lot_set_state(&self, lot: u64, state: u32, a: u32, b: u32);
    /// Slot 48 `GetBuilding` (an occupant token; no reference).
    fn lot_building(&self, lot: u64) -> Option<u64>;
    /// Slot 69 `SetVerticesAreLocked`.
    fn lot_set_vertices_locked(&self, lot: u64, locked: bool);
    /// The reference a save writes for a lot (§10.2).
    fn lot_ref(&self, lot: u64) -> RefValue;
    /// The lot a reference read with IID [`IID_LOT`] points at (the read
    /// gave the caller one reference).
    fn lot_from_ref(&self, r: &RefValue) -> Option<u64>;

    // ---- lot configurations ----
    /// Slot 14 `IsCompatibleWithWater`.
    fn cfg_compatible_with_water(&self, cfg: u64) -> bool;
    /// Slot 15 `IsCompatibleWithZoneType(type)`.
    fn cfg_compatible_with_zone(&self, cfg: u64, zone: i32) -> bool;
    /// The configuration ID (the hash-set key, §7.2.1 step 5).
    fn cfg_id(&self, cfg: u64) -> u32;
    /// Slot 7: the size (width, depth).
    fn cfg_size(&self, cfg: u64) -> (u32, u32);
    /// The families of the primary building object (`None`: no primary
    /// building object).
    fn cfg_primary_building_families(&self, cfg: u64) -> Option<Vec<u32>>;
    /// Lot configuration manager slot 9 with the filter `{purpose, wealth,
    /// stage}`: one reference per configuration for the caller.
    fn configurations_by_filter(&self, purpose: u32, wealth: u32, stage: i32) -> Vec<u64>;

    // ---- lot developer (lot-developer.md) ----
    /// Slot 29 `GetAreaSlope(rect, withWater)`.
    fn area_slope(&self, rect: CellRect, with_water: bool) -> f32;
    /// Slot 21 `AttemptAreaLevelling(rect, 0.0, true, true, FLT_MAX, null)`.
    fn attempt_area_levelling(&self, rect: CellRect);
    /// Slot 11 `RemoveBuildingZotProp(lot)`.
    fn remove_building_zot_prop(&self, lot: u64);
    /// Slot 41 `GetLotBaseTextureOccupant(lot, &occ, 0)`: the occupant, with
    /// one reference for the caller.
    fn lot_base_texture_occupant(&self, lot: u64) -> Option<u64>;
    /// Slot 33 `RemoveLotFromItsBusinessUnit(lot)`.
    fn remove_lot_from_business_unit(&self, lot: u64);
    /// Slot 43 `SetEmptyLotBaseTexture(lot)` (through BDS slot 36).
    fn set_empty_lot_base_texture(&self, lot: u64);

    // ---- traffic simulator (§8.5) ----
    /// Slot 22 `GetDesiredLotInsertionPoint`.
    fn desired_lot_insertion_point(&self) -> Option<InsertionPoint>;
    /// The insertion iterator moved to `node` (`None`: the list head).
    fn set_lot_insertion_point(&self, node: Option<u64>);

    // ---- BDS and growth developers (§7.2.1) ----
    /// BDS slot 32 `GetDeveloper(type)`.
    fn bds_developer(&self, dev_type: u32) -> Option<u64>;
    /// BDS slot 19 `GetBuildingTypesByFamily`.
    fn building_types_by_family(&self, family: u32) -> Vec<u32>;
    /// BDS slot 39 `GetBuildingProfile`.
    fn building_profile(&self, building: u32) -> BuildingProfile;
    /// Growth developer `EstimateDesirability(x, z)` (growth-developer §4.6).
    fn dev_estimate_desirability(&self, dev: u64, x: i32, z: i32) -> i16;
    /// `GetSourceDemand().QueryActiveDemandValue()` (`None`: no demand).
    fn dev_source_demand(&self, dev: u64) -> Option<f32>;
    /// The growth list, when the growth/decline lists are valid.
    fn dev_growth_list(&self, dev: u64) -> Option<Vec<GrowthItem>>;
    /// The desirability map's tract value.
    fn dev_desirability_value(&self, dev: u64, tx: i32, tz: i32) -> i16;
    /// The desirability map's tract shift.
    fn dev_desirability_shift(&self, dev: u64) -> u32;
    /// Growth developer slot 7 `GetPurpose`.
    fn dev_purpose(&self, dev: u64) -> u32;
    fn dev_wealth(&self, dev: u64) -> u32;
}
