// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The power subsystem (`docs/specs/power.md`): the power simulator
//! (`cISC4PowerSimulator`, class `0x0990C005`). Section references (`§n`)
//! point at that spec.
//!
//! # Structure
//!
//! - [`PowerSim`]: the state of §3, the lifecycle of §4, the messages of §5
//!   (occupant changes, dirty wiring, terrain, Nuclear Free Zone, lot state,
//!   deal status, PostCityInit), the recompute cycle of §6, plants and poles
//!   (§7), the save record of §9 and the interface methods of §10.1.
//! - [`PowerState`]: every plain field of §3 (bitmaps, consumption map,
//!   ring, counters, plant and pole lists) as data, for snapshots and tests.
//!   The bitmaps are the shared [`Bitmap`] (§2.1 layout and quirks,
//!   `selection_grow`).
//! - [`PowerTuning`]: the tuning exemplar of §8 with its code defaults and
//!   the "default" curve pattern (foundations §4.1); [`shipped_tuning`]
//!   holds the shipped values.
//! - [`PowerHost`]: everything the simulator reads from, or does to, the
//!   rest of the game (city, occupants and their properties, poles, lines,
//!   network occupants, the occupant, zone and lot managers, the power-line
//!   tool, demolition, pollution, the 3D view, the neighbours simulator with
//!   its deals and connections, the budget with its departments, line items
//!   and budget items, the ordinance factor, history, messages, agents).
//!   Its supertrait [`GridProvider<u8>`] creates the powered grid and
//!   resolves the grid reference of a save (in game: the game's own
//!   `cSC4SimGrid<uint8>`, simgrid §6.4).
//!
//! Host objects (occupants, lots, deals, connections, neighbour cities,
//! departments, line items) are opaque `u64` tokens; 0 is the null object.
//! Host callbacks must not call back into the power simulator (a re-entrant
//! call sees an empty state, and its changes are lost when the outer call
//! returns).
//!
//! The powered grid is held as `Rc<dyn TractGrid<u8>>`; dropping the handle
//! is the grid's `Shutdown` + `Release` (the plugin's wrapper does this in
//! its `Drop`).
//!
//! # Arithmetic
//!
//! Foundations §18 (24-bit x87): every float add, subtract, multiply and
//! divide is one `f32` rounding; an integer operand (a capacity, a counter,
//! a base cost) enters its operation unrounded (computed in `f64`, rounded
//! once to `f32`). `trunc` is `_ftol`: values that fit truncate toward
//! zero, NaN and out-of-range values give the integer indefinite
//! (`0x80000000` for 32-bit results, §6.4's NaN quirk; `0x8000000000000000`
//! for 64-bit results, of which 32-bit fields keep the low half).
//!
//! # Choices made where the spec is open (`[choice]`)
//!
//! - §15 test 5 lists history `0xAA11F7D2` = 449 for `500 × 0.9f`. At 24
//!   bits that product (449.999988…) rounds to 450.0 (foundations §18, the
//!   rule §15 tests 13 and 15 rely on), so the core gives **450**; spec
//!   question. The other values of test 5 agree with both readings.
//! - Counters are 32-bit and wrap. The pass and last totals, `consumers`,
//!   `powered`, `unpowered` and `requirement` are held as `u32` (saved as
//!   `s32`, same bits); the tests the spec writes as `> 0` or `x > y` on them
//!   (§6.4 production, §6.6 steps 4, 5, 7, 8, §7.5 deficit) compare them as
//!   signed `int32`. `avail` and `total` of §6.3 are unsigned (as the spec
//!   says), `give` / `take` use signed minimums, and `avail > 0` (the export
//!   `light` flag) is a signed test. `max(supply, remaining)` (§6.3 step 9)
//!   is unsigned.
//! - §6.3 step 1: after the grow, `rowMax` is clamped to `CellCountZ − 1`,
//!   `rowMin` to 0, and an inverted range is reset to `rowMin := rowMax`
//!   (both 0 when the city has no rows).
//! - Plant types outside 0..9 (the `0x27812853` property) are not counted
//!   in `perType` nor added to the ring (the original writes out of bounds);
//!   slots 10 and 23 return 0 for them. `ringIndex` is used modulo 12
//!   (`rem_euclid`), so a hostile saved index cannot index out of range; the
//!   stored value advances as the spec says.
//! - Consumption-map cells outside the map are not changed (§5.1, §5.2) and
//!   read 0 (slot 16, §6.3).
//! - A failed `QueryInterface` (pole, line, network occupant) ignores the
//!   occupant. A building whose budget item names a department the budget
//!   does not have skips the budget hookup (the original does not check).
//!   The budget hookup's line item, `AddLocallyFundedObject`, the secondary
//!   info field and the base cost `0x2A58D350` are all inside "if
//!   `line ≠ 0`" (§5.1).
//! - The pole list holds one host reference per pole (`AddRef` on insert,
//!   `Release` on removal, PostCityInit and `Shutdown`). On removal
//!   `costBase` and `poleCount` change only when the pole was in the list;
//!   `poleCount`, `consumers` and `requirement` never go below 0.
//! - `Init` step 1 looks the department up again even after a load: a found
//!   department replaces (and releases) the one held; otherwise the held one
//!   is kept.
//! - Without a power department: line item 1 is not written, department
//!   funding reads 1.0 (100 %, ageing and pole decay), `GetFunding` reads 0
//!   (history `0xAA11F7DC`), `GetFundingPercent` (slot 48) returns `None`.
//! - §5.4: the poles and lines "in the rect" are the occupant manager's
//!   occupants in that cell rectangle, listed once before anything is
//!   removed; each line is detached at most once, from both of its poles.
//! - §5.5: `DemolishLots` is called once even when there is no nuclear plant
//!   (with an empty list).
//! - §6 state 2: at most [`MAX_CONNECTIONS_PER_DEAL`] connections are taken
//!   per deal, in case a host never returns null. Unknown `DoIdle` states
//!   (only reachable through a hostile save) go to state 4.
//! - §6.6 step 7 and §7.5 step 3: a zero production cannot divide (the
//!   deficit is then 0); the division wraps (`int32::MIN / −1`).
//! - §7.6: a NaN `brokenFraction` clamps to 0.
//! - §8: `f32 × 0.01` tuning values use the `float` constant 0.01 (one
//!   rounding). A curve property that is not numeric, or a single scalar,
//!   counts as absent (foundations §4.1); a present property with no points
//!   gives an empty curve, which evaluates to 0.0. The per-building curves
//!   follow the same rule per call.
//! - §9: `Read` returns true except for a version above 6 (the spec); on a
//!   stream error nothing but `running` changes (references already taken
//!   are released). Null or unresolvable plant and pole references are
//!   skipped (the node is not kept). The bitmaps are sized from the city's
//!   cell counts and filled from the saved words (missing words 0, extra
//!   words dropped). A consumption map of more than [`MAX_MAP_CELLS`] cells
//!   fails the read. A loaded grid reference that does not resolve leaves no
//!   grid (§5.3 and §6.5 then skip the grid writes; `Init` does not recreate
//!   it on a loaded city, as the spec says).
//! - `GetStrikeLength` subtracts as `u32` and returns the result as `int32`.

mod cycle;
mod record;
mod simulator;
#[cfg(test)]
mod tests;
mod tuning;

pub use simulator::{FoundPlants, PlantNode, PowerSim, PowerState};
pub use tuning::{prop as tuning_prop, shipped_tuning, PowerTuning, TUNING_KEY};

pub use crate::grid::{Bitmap, CellRect, GridProvider, TractGrid};

use std::rc::Rc;

use crate::property::PropertyHolder;
use crate::stream::RefValue;
use crate::variant::Variant;

/// Class and interface IDs (§1).
pub const CLSID_POWER_SIMULATOR: u32 = 0x0990_C005;
pub const IID_POWER_SIMULATOR: u32 = 0xE9E7_BFF5;
pub const IID_MESSAGE_TARGET2: u32 = 0x4522_94AA;
pub const IID_SERIALIZABLE: u32 = 0xE4FD_A3D4;
pub const IID_UNKNOWN: u32 = 1;
/// IIDs used when reading references (§9.1).
pub const IID_DEPARTMENT_BUDGET: u32 = 0x223E_2B67;
pub const IID_SIM_GRID: u32 = 0xC9BD_939A;
pub const IID_OCCUPANT: u32 = 0x26BB_767E;
pub const IID_POWER_POLE: u32 = 0xA8C8_6142;

/// The record version written (§9.1); versions above it are rejected.
pub const RECORD_VERSION: u16 = 6;

/// Agent registration (§4.3): month, day, idle, in that order, flags 2.
pub const AGENT_NAME: &str = "cSC4PowerSimulator";
pub const AGENT_LEVELS: [u32; 3] = [6, 4, 10];
pub const AGENT_FLAGS: u32 = 2;

/// The powered grid (§2): `cSC4SimGrid<uint8>`, tract size 1.
pub const GRID_CLSID: u32 = 0x49B9_E602;
pub const GRID_INSTANCE: u32 = 0x49D5_BC86;
pub const GRID_TRACT_SIZE: i32 = 1;

/// Powered grid values (§2).
pub const UNWIRED: u8 = 0;
pub const WIRED: u8 = 1;
pub const POWERED: u8 = 2;

/// The `DoIdle` states (§6).
pub mod state {
    pub const RESET: u32 = 0;
    pub const PLANTS: u32 = 1;
    pub const NEIGHBOURS: u32 = 2;
    pub const STATISTICS: u32 = 3;
    pub const IDLE: u32 = 4;
}

/// The selection-grow iteration cap (§2.1).
pub const GROW_ITERATIONS: u32 = crate::grid::GROW_MAX_ITERATIONS;
/// The hard-coded production ratio (§6.3 step 4; tuning `0xE911E2FC` is dead).
pub const PRODUCTION_RATIO: f32 = 1.1;

/// [choice] The most connections state 2 takes per deal (§6.5).
pub const MAX_CONNECTIONS_PER_DEAL: u32 = 65_536;
/// [choice] The largest consumption map a save may hold (4096 × 4096).
pub const MAX_MAP_CELLS: u64 = 4096 * 4096;

/// Messages handled (§4.3, §5) and posted (§6.5, §6.6, §7.4).
pub mod msg {
    pub const SIM_NEW_MONTH: u32 = 0x6695_6816;
    pub const SIM_NEW_DAY: u32 = 0x6695_6814;
    pub const SIM_IDLE: u32 = 0x6695_681A;
    pub const OCCUPANT_INSERTED: u32 = 0x99EF_1142;
    pub const OCCUPANT_REMOVED: u32 = 0x99EF_1143;
    pub const ZONING_CHANGED: u32 = 0x0769_9455;
    pub const TUNING_RELOAD: u32 = 0xCA1B_151C;
    pub const TERRAIN_CHANGED: u32 = 0x278E_6003;
    pub const ORDINANCE_UPDATED: u32 = 0xC2C0_9554;
    pub const STATE_CHANGED: u32 = 0xE8BD_A2E7;
    pub const DEAL_STATUS: u32 = 0xEA41_8D8F;
    pub const CONNECTIONS_READY: u32 = 0x6AC2_84F3;
    pub const POST_CITY_INIT: u32 = 0x26D3_1EC1;
    /// The ten subscriptions, in the order of §4.1 step 2.
    pub const SUBSCRIPTIONS: [u32; 10] = [
        OCCUPANT_INSERTED,
        OCCUPANT_REMOVED,
        ZONING_CHANGED,
        TUNING_RELOAD,
        TERRAIN_CHANGED,
        ORDINANCE_UPDATED,
        STATE_CHANGED,
        DEAL_STATUS,
        CONNECTIONS_READY,
        POST_CITY_INIT,
    ];
    /// Posted per changed 8 × 8 block with the block rectangle (§6.5).
    pub const POWER_BLOCK_CHANGED: u32 = 0x8A07_AE00;
    /// Posted once when the powered set changed (§6.5).
    pub const POWER_CHANGED: u32 = 0x6A07_C2EE;
    /// Posted with data1 = 8 at the end of a cycle (§6.6, neighbors §5.3).
    pub const DEAL_STATUS_DATA1: u32 = 8;
    /// Posted when a nuclear plant is destroyed by wear (§7.4).
    pub const NUCLEAR_MELTDOWN: u32 = 0xAA55_2F77;
    /// The Nuclear Free Zone ordinance (§5.5).
    pub const NUCLEAR_FREE_ZONE: u32 = 0xE0D0_722E;
    /// Terrain changes with these data4 values are ignored (§5).
    pub const TERRAIN_IGNORED: [u32; 2] = [4, 5];
}

/// Occupant types (§5.1).
pub mod occupant_type {
    pub const POWER_POLE: u32 = 0x2890_D4DE;
    pub const POWER_LINE: u32 = 0x0990_B2EA;
    pub const NETWORK: u32 = 0xC772_BF98;
    pub const BUILDING: u32 = 0x2781_28A0;
}

/// Network occupant flags (§5.1, §5.3).
pub mod network_flag {
    pub const SKIP: u32 = 0x8000_0000;
    pub const CONDUCTS: u32 = 0x0004_0000;
}

/// Per-building properties (§8).
pub mod prop {
    pub const CAPACITY: u32 = 0x2781_2852;
    pub const REQUIREMENT: u32 = 0x2781_2854;
    pub const PLANT_TYPE: u32 = 0x2781_2853;
    pub const USED: u32 = 0x08D9_763D;
    pub const WEAR: u32 = 0xC910_B4CF;
    pub const FUEL_COST: u32 = 0xC910_B4CE;
    pub const AGEING_RATE: u32 = 0xEA1C_F220;
    pub const AGEING_T1: u32 = 0xEA1C_F221;
    pub const AGEING_T2: u32 = 0xEA1C_F222;
    pub const DESTROY_FLAGS: u32 = 0x6A43_150F;
    pub const RADIATION_STRENGTH: u32 = 0xAA5C_04C5;
    pub const RADIATION_RADIUS: u32 = 0xAA5C_04C8;
    pub const CONDITION_CURVE: u32 = 0x8A1E_07EE;
    pub const MAINTENANCE_CURVE: u32 = 0x0911_E2E7;
    pub const BASE_MAINTENANCE: u32 = 0x2A58_D350;
    pub const POLE_COST: u32 = 0x0913_2DE8;
}

/// Plant type indices (§8).
pub const PLANT_TYPES: usize = 10;
/// The nuclear plant type (§5.5, §7.4).
pub const PLANT_TYPE_NUCLEAR: i32 = 5;

/// Ageing code defaults (§7.4).
pub const DEFAULT_T1: f32 = 99.0;
pub const DEFAULT_T2: f32 = 99.0;
/// Wear from which a plant counts as aged (§7.4).
pub const AGED_WEAR: f32 = 0.8;
/// Demolition flags added at t2 (§7.4).
pub const DESTROY_EXTRA_FLAGS: u32 = 0x300;
/// The `DemolishLots` flags of the Nuclear Free Zone (§5.5).
pub const NFZ_DEMOLISH_FLAGS: u32 = 1;
/// The nuclear-plant placement tool (§5.5).
pub const NUCLEAR_TOOL_CONTROL: u32 = 0x88F1_54FB;

/// Budget IDs (§4.1, §5.1).
pub mod budget {
    pub const DEPARTMENT: u32 = 0x8910_BC8A;
    pub const DEPARTMENT_GROUP: u32 = 0x4A35_7EAF;
    pub const NAME_GROUP: u32 = 0x6A3F_F01C;
    pub const DEPARTMENT_NAME: u32 = 0x4A49_3980;
    /// Line item 1 (poles and fuel), not locally funded.
    pub const LINE_POLES: u32 = 1;
    pub const LINE_POLES_NAME: u32 = 0x2A75_F757;
    /// The power-plant budget purpose.
    pub const PURPOSE: u32 = 0xCA58_E540;
    /// The line-item display flag set on a new plant line (§5.1).
    pub const DISPLAY_FLAG: u32 = 2;
}

/// Neighbour deals and connections (§6).
pub mod deal {
    pub const BUY_POWER: u32 = 0x4773_FFE0;
    pub const SELL_POWER: u32 = 0x4773_FFE1;
    /// The deal interface (QI) of the deal-status message (§5).
    pub const IID_DEAL: u32 = 0x0773_FEF1;
    /// The "cannot supply" flag (§6.3, §6.6).
    pub const FLAG_UNSERVED: u32 = 0x20;
    /// The PowerPole connection type.
    pub const CONNECTION_POWER_POLE: u32 = 4;
}

/// The ordinance factor (§7.5, §8).
pub const ORDINANCE_FACTOR_ID: u32 = 0x0911_E117;

/// History series (§6.6, §9.2).
pub mod history {
    pub const PRODUCTION: u32 = 0xAA11_F7CE;
    pub const IMPORT: u32 = 0x0AB2_F224;
    pub const EXPORT: u32 = 0x4AB2_F236;
    pub const CONSUMPTION_NET: u32 = 0xAA11_F7D2;
    pub const DEMAND: u32 = 0xAA11_F7D5;
    pub const SURPLUS: u32 = 0xAA11_F7D8;
    pub const COST_PER_UNIT: u32 = 0xAA11_F7DC;
    pub const UNPOWERED_PERCENT: u32 = 0xAA11_FB59;
    /// Plant types 1..9 (queue length 2 at `Init`).
    pub const PER_TYPE: [u32; 9] = [
        0xAA12_0D5E,
        0xAA12_0D61,
        0xAA12_0D63,
        0xAA12_0D65,
        0xAA12_0D66,
        0xAA12_0D68,
        0xAA12_0D6A,
        0xAA12_0D6B,
        0xAA12_0D6D,
    ];
    pub const PER_TYPE_QUEUE_LENGTH: u32 = 2;
}

/// The `cISC4PowerSimulator` compiled slot table (§10.1): (slot, method,
/// callee-pop bytes). Overloads in compiled order (slot 16 = `(x, z)`,
/// slot 23 = `(type)`); slots 45 / 46 return through a hidden pointer.
pub const SLOT_TABLE: [(u32, &str, u32); 53] = [
    (0, "QueryInterface", 8),
    (1, "AddRef", 0),
    (2, "Release", 0),
    (3, "Init", 0),
    (4, "Shutdown", 0),
    (5, "PositionToCell", 16),
    (6, "CellToPosition", 16),
    (7, "CellToStandardCityCell", 16),
    (8, "StandardCityCellToCell", 16),
    (9, "GetAnnualConsumption", 0),
    (10, "GetAnnualConsumptionByPowerType", 4),
    (11, "GetMonthlyRequirement", 0),
    (12, "GetMonthlyProduction", 0),
    (13, "GetMonthlyImport", 0),
    (14, "GetMonthlyExport", 0),
    (15, "GetMonthlyDemand", 0),
    (16, "GetMonthlyConsumption(x, z)", 8),
    (17, "GetMonthlyConsumption()", 0),
    (18, "GetMonthlyConsumptionMapPtr", 4),
    (19, "GetSupplyValue", 8),
    (20, "GetSupplyValueMapPtr", 4),
    (21, "GetUnpoweredConsumerCount", 0),
    (22, "GetPoweredConsumerCount", 0),
    (23, "GetPowerPlantCount(type)", 4),
    (24, "GetPowerPlantCount()", 0),
    (25, "GetPowerPlants", 4),
    (26, "GetPowerPoleCount", 0),
    (27, "GetOverworkedPowerPlantCount", 0),
    (28, "GetAgedPowerPlantCount", 0),
    (29, "GetDistressedPowerPlantCount", 0),
    (30, "FindPowerPlants", 16),
    (31, "IsOccupantPowered", 4),
    (32, "GetOccupantPowerConsumed", 8),
    (33, "GetOccupantPowerProduced", 12),
    (34, "GetOccupantMaxPowerProduced", 8),
    (35, "GetPoweredGrid() const", 0),
    (36, "GetPoweredGrid()", 0),
    (37, "IsPowered", 8),
    (38, "IsWired", 8),
    (39, "IsOnStrike", 0),
    (40, "GetChanceOfStrike", 4),
    (41, "BeginStrike", 0),
    (42, "EndStrike", 0),
    (43, "GetStrikeStartDate", 0),
    (44, "GetStrikeLength", 0),
    (45, "GetCurrentPlantEfficiencyPercent", 8),
    (46, "GetCurrentPlantConditionPercent", 8),
    (47, "GetStrikeEfficiencyPercent", 0),
    (48, "GetFundingPercent", 0),
    (49, "GetMonthlyPowerGenerationCost", 8),
    (50, "GetPowerPlantFundingPercentage", 8),
    (51, "SetPowerPlantFundingPercentage", 8),
    (52, "destructor", 4),
];

/// Which sub-object `QueryInterface` returns (§1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerInterface {
    /// `cISC4PowerSimulator` and `cIGZUnknown`: the primary pointer.
    Primary,
    MessageTarget,
    Serializable,
}

/// `QueryInterface` (§1): the sub-object for `iid`, or `None` (false, the
/// out pointer untouched).
pub fn query_interface(iid: u32) -> Option<PowerInterface> {
    match iid {
        IID_POWER_SIMULATOR | IID_UNKNOWN => Some(PowerInterface::Primary),
        IID_MESSAGE_TARGET2 => Some(PowerInterface::MessageTarget),
        IID_SERIALIZABLE => Some(PowerInterface::Serializable),
        _ => None,
    }
}

/// A message as `DoMessage` sees it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PowerMessage {
    pub msg_type: u32,
    pub data1: u32,
    pub data2: u32,
    pub data3: u32,
    pub data4: u32,
    /// void1 / the IGZUnknown (occupant, lot or deal) as a host token; 0 =
    /// null.
    pub void1: u64,
    /// The four `int32` at void2 (zoning changed, §5).
    pub rect: CellRect,
}

/// A message the simulator posts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Posted {
    pub msg_type: u32,
    pub data: [u32; 4],
    /// The IGZUnknown (§7.4: the plant), if any.
    pub unknown: Option<u64>,
}

/// A budget item as `GetBudgetItemForPurpose` reports it (budget spec).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BudgetItem {
    pub department: u32,
    pub line: u32,
    pub cost: i64,
    pub index: u32,
    /// The item's local funding fraction (`SC4Percentage`).
    pub local_funding: f32,
}

/// A neighbour deal as `FindDeals` returns it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Deal {
    /// Host identity of the `cISC4NeighborDeal`.
    pub token: u64,
    /// `GetCityOnOtherSideOfConnection()`.
    pub city: u64,
    /// `GetAmount()`.
    pub amount: i32,
}

/// `cISC4PowerPoleOccupant` facts (§5.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PoleInfo {
    /// Slot 13: true = the power simulator ignores the pole.
    pub ignored: bool,
    /// `GetCell(&x, &z)`.
    pub cell: (i32, i32),
}

/// `cISC4PowerLineOccupant` facts (§5.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LineInfo {
    pub preview: bool,
    /// `GetCell(&x, &z)`.
    pub cell: (i32, i32),
}

/// Everything the power simulator reads from, or does to, the rest of the
/// game. See the module documentation for tokens and re-entrancy.
pub trait PowerHost: GridProvider<u8> {
    // ---- city, tuning, ordinance, simulator ----
    /// `cISC4City::CellCountX()` / `CellCountZ()`.
    fn cell_counts(&self) -> (i32, i32);
    /// `cISC4City::CellWidthX()` / `CellWidthZ()`.
    fn cell_widths(&self) -> (f32, f32);
    /// The "Utilities" exemplar ([`TUNING_KEY`]), if it loads.
    fn tuning(&self) -> Option<Rc<PropertyHolder>>;
    /// `cISC4OrdinanceSimulator::GetCumulativeFactor(id, index)`.
    fn cumulative_factor(&self, id: u32, index: u32) -> f32;
    /// `cISC4Simulator::GetSimTime()`.
    fn sim_time(&self) -> u32;
    /// §4.1 step 1: take the city and its managers, the neighbours and
    /// budget simulators and the demolition utility.
    fn acquire_services(&self);
    /// §4.2: release what [`PowerHost::acquire_services`] took.
    fn release_services(&self);

    // ---- framework ----
    /// `cISC4Simulator::AddAgent(target, level, name, flags)`.
    fn add_agent(&self, level: u32, name: &str, flags: u32);
    /// `cISC4Simulator::RemoveAgent(target)`.
    fn remove_agent(&self);
    fn subscribe(&self, msg_types: &[u32]);
    fn unsubscribe(&self, msg_types: &[u32]);
    /// `MessagePost(msg, false)` of a standard message.
    fn post(&self, msg: &Posted);
    /// `cISC4HistoryWarehouse::RecordHistoryData(id, value)`.
    fn record_history(&self, series: u32, value: i32);
    /// `cISC4HistoryWarehouse::SetHistoryDataQueueLength(id, n)`.
    fn set_history_queue_length(&self, series: u32, length: u32);

    // ---- host object references ----
    fn add_ref(&self, token: u64);
    fn release(&self, token: u64);
    /// How a save refers to a host object (department, occupant, pole).
    fn object_ref(&self, token: u64) -> RefValue;
    /// The token of a reference read from a save, if it answers `iid`. The
    /// reference the stream handed over passes to the simulator; on `None`
    /// the host drops it.
    fn resolve_ref(&self, r: &RefValue, iid: u32) -> Option<u64>;

    // ---- occupants ----
    /// `cISC4Occupant::GetType()`.
    fn occupant_type(&self, occ: u64) -> u32;
    /// A property of the occupant's property holder (own or inherited).
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant>;
    /// `AddProperty(id, value)` on the occupant's property holder.
    fn set_occupant_property(&self, occ: u64, id: u32, value: Variant);
    /// `cISC4Occupant::GetPosition()` (x, y, z).
    fn occupant_position(&self, occ: u64) -> [f32; 3];
    /// `cISC4Occupant::GetBoundingBox()`: (min corner, max corner).
    fn occupant_bounding_box(&self, occ: u64) -> ([f32; 3], [f32; 3]);
    /// `cISC4Occupant::GetBoundingCityCells()` (inclusive).
    fn occupant_bounding_cells(&self, occ: u64) -> CellRect;
    /// `cISC4Occupant::IsFlagSet(1)` (in the occupant manager).
    fn occupant_in_manager(&self, occ: u64) -> bool;
    /// QI `cISC4BuildingOccupant` (`0x87DDA3A9`) and `GetBuildingType()`;
    /// `None` when the QI fails.
    fn building_type(&self, occ: u64) -> Option<u32>;
    /// QI `cISC4PowerPoleOccupant` (`0xA8C86142`); `None` when it fails.
    fn power_pole(&self, occ: u64) -> Option<PoleInfo>;
    /// Pole slot 6 `BreakAllConnections(flag)`.
    fn break_all_connections(&self, pole: u64, flag: bool);
    /// Pole slot 11 `IsDistressed()`.
    fn pole_is_distressed(&self, pole: u64) -> bool;
    /// Pole slot 12 `SetDistressed(on)`.
    fn set_pole_distressed(&self, pole: u64, on: bool);
    /// QI `cISC4PowerLineOccupant` (`0xC98F3B24`); `None` when it fails.
    fn power_line(&self, occ: u64) -> Option<LineInfo>;
    /// Line slot 3 `GetConnectedPoles`: both poles, or `None` unless both
    /// ends are connected.
    fn line_connected_poles(&self, line: u64) -> Option<(u64, u64)>;
    /// The non-virtual `cSC4PowerPoleOccupant::RemoveConnection` of `pole`
    /// for `line` (§5.4, §11.2).
    fn remove_pole_connection(&self, pole: u64, line: u64);
    /// QI `cISC4NetworkOccupant` (`0xA821EF94`) and `GetOccupiedCell()`;
    /// `None` when the QI fails.
    fn network_occupant_cell(&self, occ: u64) -> Option<(i32, i32)>;
    /// Network occupant `HasNetworkFlag(flag)`.
    fn has_network_flag(&self, occ: u64, flag: u32) -> bool;
    /// Network occupant `GetNetworkFlag()`.
    fn network_flags(&self, occ: u64) -> u32;
    /// `GetOccupantsByStandardCityCells(list, &min, &max, 0, −1)`.
    fn occupants_in_cells(&self, rect: CellRect) -> Vec<u64>;
    /// `cISC4OccupantManager::RemoveOccupant(occ, true, 0)`.
    fn remove_occupant(&self, occ: u64);

    // ---- zones, network tool ----
    /// `cISC4ZoneManager::GetZoneType(x, z)`.
    fn zone_type(&self, x: i32, z: i32) -> u32;
    /// The power-line tool (`GetNetworkTool(5, false)`): `AddRef`, `Init`,
    /// `UpdateOnZoneChange(rect, data3)`, `Shutdown`, `Release` (§5).
    fn power_line_tool_zone_change(&self, rect: CellRect, data3: u32);

    // ---- lots, demolition, pollution, view ----
    /// `cISC4LotManager::GetLot(x, z, false)`.
    fn lot_at(&self, x: i32, z: i32) -> Option<u64>;
    /// `cISC4LotManager::GetOccupantLot(occ)`.
    fn occupant_lot(&self, occ: u64) -> Option<u64>;
    /// QI `cISC4Lot` (`0xE818FCED`) of a message's IGZUnknown.
    fn as_lot(&self, token: u64) -> Option<u64>;
    /// `cISC4Lot::GetBoundingRect()` (inclusive cells).
    fn lot_bounding_rect(&self, lot: u64) -> CellRect;
    /// `cISC4Lot::GetState()`.
    fn lot_state(&self, lot: u64) -> u32;
    /// `cISC4Lot::GetNeedsPower()`.
    fn lot_needs_power(&self, lot: u64) -> bool;
    /// `cISC4Lot::GetHasPower()`.
    fn lot_has_power(&self, lot: u64) -> bool;
    /// `cISC4Lot::SetHasPower(on)`.
    fn set_lot_has_power(&self, lot: u64, on: bool);
    /// `cISC4Lot::SetCondition(c)`.
    fn set_lot_condition(&self, lot: u64, condition: u32);
    /// `DemolishLots(true, lots, 0, flags, false, null, null, 0)`.
    fn demolish_lots(&self, lots: &[u64], flags: u32);
    /// `cISC4PollutionSimulator::AddRadiationToCity(&pos, strength, radius)`.
    fn add_radiation(&self, position: [f32; 3], strength: f32, radius: f32);
    /// The 3D view's current input control ID (control slot 5), if any.
    fn current_view_input_control(&self) -> Option<u32>;
    /// `RemoveCurrentViewInputControl(false)`.
    fn remove_current_view_input_control(&self);

    // ---- neighbours ----
    /// `FindDeals(type)`, in list order.
    fn find_deals(&self, deal_type: u32) -> Vec<Deal>;
    /// `GetAmountUsed()`.
    fn deal_amount_used(&self, deal: u64) -> i32;
    /// `SetAmountUsed(v)`.
    fn set_deal_amount_used(&self, deal: u64, used: i32);
    /// `SetFlags(flags, on)`.
    fn set_deal_flags(&self, deal: u64, flags: u32, on: bool);
    /// QI the deal interface (`0x0773FEF1`) and `GetType()`; `None` if the
    /// object is not a deal.
    fn deal_type(&self, token: u64) -> Option<u32>;
    /// `MarkDealsDirty()`.
    fn mark_deals_dirty(&self);
    /// `GetConnectionsOfType(type)`.
    fn connections_of_type(&self, connection_type: u32) -> Vec<u64>;
    /// `GetConnectionsOfTypeToCity(type, city)`.
    fn connections_to_city(&self, connection_type: u32, city: u64) -> Vec<u64>;
    /// `FindUnvisitedConnection(type, city)`.
    fn find_unvisited_connection(&self, connection_type: u32, city: u64) -> Option<u64>;
    /// `FindConnectionOnOtherSide(conn)`.
    fn connection_on_other_side(&self, conn: u64) -> Option<u64>;
    /// `GetConnectionCell(&x, &z)` (not clamped).
    fn connection_cell(&self, conn: u64) -> (i32, i32);
    /// `SetVisitedThisCycle(on)`.
    fn set_connection_visited(&self, conn: u64, on: bool);
    /// `GetIsActive()`.
    fn connection_is_active(&self, conn: u64) -> bool;
    /// `SetIsActive(on)`.
    fn set_connection_active(&self, conn: u64, on: bool);

    // ---- budget ----
    /// `GetDepartmentBudget(id)` (borrowed, no reference added).
    fn department(&self, id: u32) -> Option<u64>;
    /// `CreateDepartmentBudget(id, group)` (borrowed).
    fn create_department(&self, id: u32, group: u32) -> Option<u64>;
    fn set_department_name(&self, dept: u64, group: u32, instance: u32);
    fn set_department_max_funding(&self, dept: u64, fraction: f32);
    /// `*GetFundingPercentage()` (a fraction).
    fn department_funding(&self, dept: u64) -> f32;
    /// `cISC4BudgetSimulator::GetFunding(dept)` (slot 35).
    fn budget_funding(&self, dept: u64) -> i64;
    /// `CreateLineItem(line, locallyFunded)`.
    fn create_line_item(&self, dept: u64, line: u32, locally_funded: bool) -> Option<u64>;
    /// `CreateLineItemForBuildingType(line, flag)`.
    fn create_line_item_for_building_type(&self, dept: u64, line: u32, flag: bool) -> Option<u64>;
    /// `GetLineItem(line)`.
    fn line_item(&self, dept: u64, line: u32) -> Option<u64>;
    fn set_line_item_name(&self, item: u64, group: u32, instance: u32);
    fn set_line_item_display_flag(&self, item: u64, flag: u32, on: bool);
    fn line_item_secondary_info(&self, item: u64) -> i32;
    fn set_line_item_secondary_info(&self, item: u64, value: i32);
    fn set_line_item_full_expenses(&self, item: u64, value: i64);
    /// `GetBudgetItemForPurpose(holder, purpose)`.
    fn budget_item_for_purpose(&self, occ: u64, purpose: u32) -> Option<BudgetItem>;
    fn change_budget_item_line(&self, occ: u64, index: u32, line: u32);
    fn change_budget_item_cost(&self, occ: u64, index: u32, cost: i64);
    fn add_locally_funded_object(&self, dept: u64, occ: u64, purpose: u32);
    fn remove_locally_funded_object(&self, dept: u64, occ: u64, purpose: u32);
    fn is_locally_funded_object(&self, dept: u64, occ: u64, purpose: u32) -> bool;
    fn set_local_funding_percent(&self, dept: u64, occ: u64, fraction: f32, purpose: u32) -> bool;
    fn set_local_full_funding(&self, dept: u64, occ: u64, amount: i64, purpose: u32);
}

// ---- arithmetic (foundations §18) -------------------------------------------------------------------

/// `_ftol` to 32 bits: truncation toward zero; NaN and values outside
/// `int32` give `0x80000000`.
pub fn ftol32(v: f32) -> i32 {
    let d = f64::from(v);
    if d.is_nan() || d >= 2_147_483_648.0 || d <= -2_147_483_649.0 {
        i32::MIN
    } else {
        d as i32
    }
}

/// `_ftol` to 64 bits: truncation toward zero; NaN and values outside
/// `int64` give `0x8000000000000000`.
pub fn ftol64(v: f32) -> i64 {
    let d = f64::from(v);
    const TWO_63: f64 = 9_223_372_036_854_775_808.0;
    if d.is_nan() || !(-TWO_63..TWO_63).contains(&d) {
        i64::MIN
    } else {
        d as i64
    }
}

/// `int × float`: the integer enters unrounded, one `f32` rounding.
pub(crate) fn mul_i(i: f64, f: f32) -> f32 {
    (i * f64::from(f)) as f32
}

/// `int / float`: one `f32` rounding.
pub(crate) fn div_i(i: f64, f: f32) -> f32 {
    (i / f64::from(f)) as f32
}

/// `int / int` evaluated in floating point: one `f32` rounding.
pub(crate) fn div_ii(a: f64, b: f64) -> f32 {
    (a / b) as f32
}
