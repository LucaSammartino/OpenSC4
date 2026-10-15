// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The building development simulator's C ABI (fc4.h "development",
//! docs/specs/building-development.md).
//!
//! The host implements [`fc4_development_host`]: every `DevelopmentHost`
//! method of the core, the growth developers (`GrowthDevelopers`, called on
//! the game's `cISC4GrowthDeveloper` interface, so the BDS works with any
//! developer object) and the game's grids through [`fc4_grid_api`]. The
//! plugin's `cISC4BuildingDevelopmentSimulator` (with its message-target and
//! serializable sub-objects) forwards its slots to the `fc4_development_*`
//! functions.
//!
//! The process-wide statics of §2 (the building catalogue, the tried-tract
//! marks and the BDS random generator) are one [`fc4_development_process`]
//! the host creates once per process and hands to every BDS it creates. The
//! catalogue is exported from it (`fc4_development_catalogue_*`) so the host
//! can keep the game-layout copies slots 11–22, 27–28, 37–40, 42–43 and 45
//! return.
//!
//! Tokens are opaque host identities (fc4.h): developers
//! (`cISC4GrowthDeveloper*`), lots, occupants, the construction node
//! (`cISC4Demand*`) and grids. 0 is the null object. The core owns one
//! reference on every developer, special building and node token it keeps,
//! and on every occupant `create_building_occupant` or
//! `create_foundation_occupant` hands over (slot 23 releases its foundation
//! once the occupant manager holds it). A saved
//! reference read back for one of the host's objects arrives with that
//! reference; one that names a FreeCity4 object (`fc4_ref::core_handle` set)
//! is given one here (`add_ref`), so both cases end up the same. Lot tokens
//! (`lot_at`, `lots_in_cell_rect`, message lots) are borrowed.
//!
//! Re-entrancy: the core holds no borrow across a host callback, so the
//! tract developer may call the BDS slots during `tract_grow` /
//! `tract_decline`. A message delivered while a call is running is queued and
//! handled when the outermost call returns ([choice], as protection); other
//! re-entrant calls run at once and are counted
//! (`fc4_development_state::reentrant_calls`).
//!
//! Threads: the core is single-threaded (`RefCell`). The plugin serialises
//! every call on the BDS, its process statics and every growth developer with
//! one process-wide lock (fc4.h).

use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashMap};
use std::ffi::{c_char, c_void};
use std::rc::Rc;

use fc4_core::development::{building_map_key, developer_type, wealth_of, ConstructionProfile, GrowthItem};
use fc4_core::development_simulator::{
    prop, query_interface, AdviceSubject, BdsInterface, BdsProcess, BuildingConfigObject, CellRect, CommandParam,
    ConstructionNode, DeveloperId, DevelopmentHost, DevelopmentSim, GrowthDevelopers, GrowthProfile, LotConfigInfo,
    LotId, Message, OccupantId, Outcome, SharedProcess, Vec3,
};
use fc4_core::grid::{GridProvider, TractGrid};
use fc4_core::growth_developer::CLSID_GROWTH_DEVELOPER;
use fc4_core::property::PropertyHolder;
use fc4_core::rng::FixedSeeds;
use fc4_core::stream::{InStream, RefValue, ResKey, SaveObject};

use crate::budget::fc4_token_callback;
use crate::guard;
use crate::hostgrid::{collect_tokens, create_grid, fc4_cell_rect, fc4_grid_api, grid_token, resolve_grid, take_props};
use crate::props::fc4_props;
use crate::stream::{fc4_bytes_sink, fc4_istream, fc4_ostream, fc4_ref, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

/// `cSC4Demand`, the class of the construction node (§1.3 step 7; demand
/// spec §3): the class ID its saved reference carries.
pub const CLSID_DEMAND_NODE: u32 = 0x8990_C12D;

/// A resource key (fc4.h `fc4_res_key`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_res_key {
    pub type_id: u32,
    pub group: u32,
    pub instance: u32,
}

impl From<ResKey> for fc4_res_key {
    fn from(k: ResKey) -> Self {
        fc4_res_key {
            type_id: k.type_id,
            group: k.group,
            instance: k.instance,
        }
    }
}

impl From<fc4_res_key> for ResKey {
    fn from(k: fc4_res_key) -> Self {
        ResKey {
            type_id: k.type_id,
            group: k.group,
            instance: k.instance,
        }
    }
}

/// `GrowthProfile` (fc4.h `fc4_growth_profile`): the developer token, the
/// tract and its stage (the tract developer reads the low byte).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_growth_profile {
    pub developer: u64,
    pub x: i32,
    pub z: i32,
    pub stage: i32,
    pub reserved: u32,
}

impl From<&GrowthProfile> for fc4_growth_profile {
    fn from(p: &GrowthProfile) -> Self {
        fc4_growth_profile {
            developer: p.developer,
            x: p.x,
            z: p.z,
            stage: p.stage,
            reserved: 0,
        }
    }
}

/// One growth or decline list entry (fc4.h `fc4_growth_item`): the game's
/// 12-byte `{int32 x, int32 z, stage}` (the stage's low byte is the game's
/// `uint8`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_growth_item {
    pub x: i32,
    pub z: i32,
    pub max_stage: i32,
}

impl From<GrowthItem> for fc4_growth_item {
    fn from(i: GrowthItem) -> Self {
        fc4_growth_item {
            x: i.x,
            z: i.z,
            max_stage: i.max_stage,
        }
    }
}

impl From<fc4_growth_item> for GrowthItem {
    fn from(i: fc4_growth_item) -> Self {
        GrowthItem {
            x: i.x,
            z: i.z,
            max_stage: i.max_stage,
        }
    }
}

/// `ConstructionProfile` in the game's layout (fc4.h
/// `fc4_construction_profile`, BDS §4.8: 0x24 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_construction_profile {
    pub zone: i32,
    pub unknown2: i32,
    pub wealth: u32,
    pub purpose: u32,
    pub land_value_min: u8,
    pub land_value_max: u8,
    pub reserved: [u8; 2],
    pub building_type: u32,
    pub orientation: i32,
    pub x: i32,
    pub z: i32,
}

impl From<&fc4_construction_profile> for ConstructionProfile {
    fn from(p: &fc4_construction_profile) -> Self {
        ConstructionProfile {
            zone: p.zone,
            unknown2: p.unknown2,
            wealth: p.wealth,
            purpose: p.purpose,
            land_value_min: p.land_value_min,
            land_value_max: p.land_value_max,
            building_type: p.building_type,
            orientation: p.orientation,
            x: p.x,
            z: p.z,
        }
    }
}

impl fc4_construction_profile {
    /// Writes the fields `GetConstructionInfo` sets (`zone` … the land value
    /// range); the rest stays as the caller had it.
    pub(crate) fn assign_info(&mut self, p: &ConstructionProfile) {
        self.zone = p.zone;
        self.unknown2 = p.unknown2;
        self.wealth = p.wealth;
        self.purpose = p.purpose;
        self.land_value_min = p.land_value_min;
        self.land_value_max = p.land_value_max;
    }

    pub(crate) fn to_core(self) -> ConstructionProfile {
        ConstructionProfile::from(&self)
    }
}

/// One lot configuration as the maximum density table reads it (fc4.h
/// `fc4_lot_config`, §4.7.4).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_lot_config {
    pub size_x: u32,
    pub size_z: u32,
    /// Bit z set ⇔ `IsCompatibleWithZoneType(z)`, z in 0..16.
    pub zones: u32,
    /// 1 when the configuration has a primary building configuration object.
    pub has_primary: u32,
    pub orientation: u32,
    pub x: f32,
    pub z: f32,
    pub n_ids: u32,
    /// The object's building or family IDs (`n_ids`).
    pub ids: *const u32,
}

/// The building profile of the catalogue (fc4.h `fc4_building_profile`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_building_profile {
    pub purpose: u32,
    pub wealth: u32,
    pub capacity: u16,
    pub value_499afa38: u8,
    pub flag_27812836: u8,
    pub category_flags: u32,
    pub occupant_size: [f32; 3],
    pub reserved0: u32,
    pub plop_cost: i64,
    /// Entries of the capacity table (handed to the callback).
    pub n_capacities: u32,
    /// 1 when property `0x27812834` exists (DG11): the game profile then
    /// holds exactly one capacity map, even an empty one.
    pub capacities_present: u32,
}

/// A message for `fc4_development_do_message` (fc4.h
/// `fc4_development_message`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_development_message {
    pub msg_type: u32,
    pub data1: u32,
    pub data2: u32,
    pub data3: u32,
    pub data4: u32,
    pub reserved: u32,
    /// `GetVoid1()` as a `cISC4Occupant` (occupant messages), 0 otherwise.
    pub occupant: u64,
    /// `GetIGZUnknown()` as a `cISC4Lot` (StateChanged), 0 otherwise.
    pub lot: u64,
}

/// One PlaceBuilding command parameter (fc4.h `fc4_command_param`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_command_param {
    pub as_u32: u32,
    pub as_f32: f32,
}

/// The advice subject §4.10 builds and discards (fc4.h
/// `fc4_advice_subject`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_advice_subject {
    pub x: f32,
    pub z: f32,
    pub data1: u32,
    pub reserved: u32,
    pub lot: u64,
}

/// One decline plan entry (fc4.h `fc4_plan_entry`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_plan_entry {
    pub day: u32,
    pub x: i32,
    pub z: i32,
    pub stage: u32,
    pub developer: u64,
}

/// Diagnostics (fc4.h `fc4_development_state`). The counters are
/// cumulative since the handle was created.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_development_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub loaded: u8,
    /// The planning state machine (0–4, §4.2).
    pub state: u8,
    /// Bit 0 ignore plumbing, 1 ignore power, 2 ignore caps, 3 debug growth,
    /// 4 coffee bar.
    pub cheats: u8,
    pub developers: u32,
    pub special_buildings: u32,
    pub highest_stages: u32,
    pub decline_plan: u32,
    pub marked_types: u32,
    pub marked_cells: u32,
    /// Building types in the process catalogue.
    pub building_types: u32,
    /// The daily growth budget (§4.4.1; `0xFFFFFFFF` = unlimited).
    pub grow_budget: u32,
    pub total_resources: i32,
    pub free_resources: i32,
    pub supply: f32,
    pub demand: f32,
    /// Tract developer `Grow` calls, and those that returned > 0.
    pub grow_calls: u32,
    pub built: u32,
    /// Tract developer `Decline` calls, and those that returned > 0.
    pub decline_calls: u32,
    pub declined: u32,
    /// Successful `Grow` calls by the developer's wealth (0–3).
    pub built_by_wealth: [u32; 4],
    /// Successful `Decline` calls by the developer's wealth (0–3).
    pub declined_by_wealth: [u32; 4],
    /// Successful `Grow` calls by the profile's stage (0–15; larger stages
    /// count as 15).
    pub built_by_stage: [u32; 16],
    pub reentrant_calls: u32,
    pub cell_count_x: i32,
    pub cell_count_z: i32,
    pub reserved: u32,
    /// Steps of the process-wide BDS generator.
    pub rng_steps: u64,
    pub building_map: u64,
    pub construction_node: u64,
}

/// One exemplar (`exemplars`): the visitor takes the NEW props (or NULL) and
/// the exemplar's localized building name (`SC4LoadExemplarString` of
/// property `0x8A416A99`), when `has_name`.
pub type fc4_development_exemplar_visitor = Option<
    unsafe extern "C" fn(
        ctx: *mut c_void,
        key: *const fc4_res_key,
        props: *mut fc4_props,
        name: *const u8,
        name_len: usize,
        has_name: bool,
    ),
>;
/// One lot configuration (`lot_configurations`).
pub type fc4_lot_config_visitor = Option<unsafe extern "C" fn(ctx: *mut c_void, config: *const fc4_lot_config)>;
/// A growth or decline list (`developer_list`), delivered in one call.
pub type fc4_growth_items_sink =
    Option<unsafe extern "C" fn(ctx: *mut c_void, items: *const fc4_growth_item, n: usize)>;
/// One (type, token) pair.
pub type fc4_type_token_callback = Option<unsafe extern "C" fn(ctx: *mut c_void, key: u32, token: u64)>;
/// One (key, value) pair.
pub type fc4_type_value_callback = Option<unsafe extern "C" fn(ctx: *mut c_void, key: u32, value: u32)>;
/// One decline plan entry.
pub type fc4_plan_entry_callback = Option<unsafe extern "C" fn(ctx: *mut c_void, e: *const fc4_plan_entry)>;
/// One maximum density table entry: zone, building map key, the raw entry
/// and `GetMaxCapacity`'s value for it (`float(trunc(entry))`).
pub type fc4_density_callback =
    Option<unsafe extern "C" fn(ctx: *mut c_void, zone: u32, key: u32, entry: f32, max_capacity: f32)>;

/// Everything the BDS asks of the game (fc4.h `fc4_development_host`; one
/// callback per `DevelopmentHost` / `GrowthDevelopers` method, in the
/// traits' order).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_development_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    pub grids: fc4_grid_api,
    // clock, city, terrain
    pub sim_day: Option<unsafe extern "C" fn(U) -> u32>,
    pub difficulty: Option<unsafe extern "C" fn(U) -> u32>,
    /// `CellWidthX`, `CellWidthZ` into `out[2]`.
    pub cell_widths: Option<unsafe extern "C" fn(U, *mut f32)>,
    /// The terrain's cell counts into `out[2]`.
    pub terrain_cell_counts: Option<unsafe extern "C" fn(U, *mut i32)>,
    pub position_to_cell: Option<unsafe extern "C" fn(U, f32, f32, *mut i32)>,
    pub cell_center: Option<unsafe extern "C" fn(U, i32, i32, *mut f32)>,
    pub altitude: Option<unsafe extern "C" fn(U, f32, f32) -> f32>,
    // resources
    /// Every exemplar whose ExemplarType is one of `types` (all when
    /// `n_types` is 0), in the resource key list's order.
    pub exemplars: Option<unsafe extern "C" fn(U, *const u32, usize, fc4_development_exemplar_visitor, *mut c_void)>,
    pub lot_configurations: Option<unsafe extern "C" fn(U, fc4_lot_config_visitor, *mut c_void)>,
    /// A NEW fc4_props (the core destroys it), NULL if the exemplar is missing.
    pub tuning: Option<unsafe extern "C" fn(U) -> *mut fc4_props>,
    // framework
    pub register_static_cleanup: Option<unsafe extern "C" fn(U)>,
    pub post_building_types_updated: Option<unsafe extern "C" fn(U, u32)>,
    pub subscribe: Option<unsafe extern "C" fn(U, *const u32, usize)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, *const u32, usize)>,
    pub register_command: Option<unsafe extern "C" fn(U, u32, *const c_char, usize)>,
    pub unregister_command: Option<unsafe extern "C" fn(U, u32)>,
    /// `AddAgent(target, level, name, flags)`.
    pub add_agent: Option<unsafe extern "C" fn(U, u32, *const c_char, usize, u32)>,
    pub remove_agent: Option<unsafe extern "C" fn(U)>,
    pub register_cheat: Option<unsafe extern "C" fn(U, u32, *const c_char, usize)>,
    pub unregister_cheat: Option<unsafe extern "C" fn(U, u32)>,
    pub acquire_services: Option<unsafe extern "C" fn(U)>,
    pub release_services: Option<unsafe extern "C" fn(U)>,
    // demand
    pub census_supply: Option<unsafe extern "C" fn(U, u32, *mut f32) -> bool>,
    /// A new `cSC4Demand` (not `Init`ed) with one reference for the core; 0 =
    /// none.
    pub create_construction_node: Option<unsafe extern "C" fn(U) -> u64>,
    pub node_init: Option<unsafe extern "C" fn(U, u64)>,
    pub node_shutdown: Option<unsafe extern "C" fn(U, u64)>,
    pub node_supply: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub node_demand: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub node_set_supply: Option<unsafe extern "C" fn(U, u64, f32)>,
    pub node_set_demand: Option<unsafe extern "C" fn(U, u64, f32)>,
    pub node_add_to_demand: Option<unsafe extern "C" fn(U, u64, f32)>,
    pub node_end_of_cycle: Option<unsafe extern "C" fn(U, u64) -> bool>,
    /// `QueryActiveDemandValue()` of a developer's source node.
    pub demand_active_value: Option<unsafe extern "C" fn(U, u64) -> f32>,
    // tract developer
    /// `Grow(profile, outcomes, flag)`: the host makes and frees the outcome
    /// list (§4.4); `flag` arrives false.
    pub tract_grow: Option<unsafe extern "C" fn(U, *const fc4_growth_profile, *mut bool) -> i32>,
    pub tract_decline: Option<unsafe extern "C" fn(U, *const fc4_growth_profile) -> i32>,
    // lots, lot manager, lot developer
    pub lot_building_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub lot_bounding_rect: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    pub lot_growth_stage: Option<unsafe extern "C" fn(U, u64, *mut u8) -> bool>,
    pub lot_building: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub lot_state: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub lot_center: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    pub lot_slope: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub lot_demolishable: Option<unsafe extern "C" fn(U, u64) -> bool>,
    /// `GetLot(x, z, true)`, borrowed; 0 = none.
    pub lot_at: Option<unsafe extern "C" fn(U, i32, i32) -> u64>,
    /// `GetLotsInCellRect(…, false)`: `cb` once per lot (borrowed tokens the
    /// host keeps alive until the outermost BDS call returns).
    pub lots_in_cell_rect: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, fc4_token_callback, *mut c_void)>,
    pub lot_developer_slope: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub update_lot_elements: Option<unsafe extern "C" fn(U, u64, u32)>,
    /// `GetLowestOrHighestAreaAltitude({x1, z1, x2, z2}, false, true)`.
    pub area_altitude: Option<unsafe extern "C" fn(U, *const f32) -> f32>,
    // zones, land value, power, demolition, budget
    pub zone_type: Option<unsafe extern "C" fn(U, i32, i32) -> i8>,
    pub dezone: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, bool)>,
    pub powered: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    pub land_value_type: Option<unsafe extern "C" fn(U, i32, i32) -> u32>,
    pub land_value: Option<unsafe extern "C" fn(U, i32, i32) -> u8>,
    /// `DemolishLots(true, lots, 0, 1, true, null, &cost, null)`: the cost.
    pub demolish_lots: Option<unsafe extern "C" fn(U, *const u64, usize) -> i64>,
    pub total_funds: Option<unsafe extern "C" fn(U) -> i64>,
    pub set_total_funds: Option<unsafe extern "C" fn(U, i64)>,
    // occupants
    pub has_occupant_manager: Option<unsafe extern "C" fn(U) -> bool>,
    pub insert_occupant: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub remove_occupant: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub building_occupant_at_cell: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    pub occupant_has_property: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub occupant_property_i64: Option<unsafe extern "C" fn(U, u64, u32, *mut i64) -> bool>,
    pub occupant_in_group: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub occupant_position: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    /// A new building occupant with one reference for the core; 0 = none.
    pub create_building_occupant: Option<unsafe extern "C" fn(U) -> u64>,
    pub set_building_type: Option<unsafe extern "C" fn(U, u64, u32)>,
    /// `GetBuildingType()` (building occupant slot 4; §4.9.1 loader step 2).
    pub building_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    /// The exemplar half of the loaders (§4.9.1, §4.9.2): `GetResource(key,
    /// 0xA52160F5)`, then `SetDefaultExemplar` on the occupant's
    /// `cISCExemplarPropertyHolder` (QI `0x0AC2B5F7`); false when the
    /// resource does not load.
    pub set_default_exemplar: Option<unsafe extern "C" fn(U, u64, *const fc4_res_key) -> bool>,
    /// The float32 values of the occupant's property `id` through its own
    /// holder: false when missing; else the value count into `*count` and at
    /// most `n` values into `out`.
    pub occupant_property_f32s: Option<unsafe extern "C" fn(U, u64, u32, *mut f32, usize, *mut usize) -> bool>,
    /// Min then max corner into `out[6]`.
    pub occupant_bounding_box: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    pub set_occupant_bounding_box: Option<unsafe extern "C" fn(U, u64, *const f32)>,
    pub set_occupant_position: Option<unsafe extern "C" fn(U, u64, *const f32)>,
    pub set_occupant_orientation: Option<unsafe extern "C" fn(U, u64, i32)>,
    pub init_occupant: Option<unsafe extern "C" fn(U, u64)>,
    /// `AddRef` / `Release` on any token the core holds (occupants,
    /// developers, the node).
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    // foundations and retaining walls (§4.9.2, DG6)
    /// A new foundation occupant (class `0x49C05C8F`) with one reference for
    /// the core (GZCOM's; not AddRef'd again); 0 = none.
    pub create_foundation_occupant: Option<unsafe extern "C" fn(U) -> u64>,
    /// `cISC4FoundationOccupant` (IID `0xC8FCE101`) slot 4.
    pub foundation_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    /// Slot 5.
    pub set_foundation_type: Option<unsafe extern "C" fn(U, u64, u32)>,
    /// Slot 7.
    pub set_foundation_max_height: Option<unsafe extern "C" fn(U, u64, f32)>,
    /// Slot 8, `size[3]`.
    pub set_foundation_size: Option<unsafe extern "C" fn(U, u64, *const f32)>,
    /// Slot 10.
    pub set_foundation_usage: Option<unsafe extern "C" fn(U, u64, i32)>,
    /// Slot 12.
    pub set_foundation_orientation: Option<unsafe extern "C" fn(U, u64, i32)>,
    /// `cISC4LotRetainingWallOccupant` slot 11 of the wall the occupant
    /// token stands for.
    pub retaining_wall_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    /// Lot developer slot 17 (the int32 overload of
    /// `GetLowestOrHighestAreaAltitude`): `(rect, lowest, with_water)`.
    pub area_altitude_cells: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, bool, bool) -> f32>,
    // advisors
    pub push_advisor_event: Option<unsafe extern "C" fn(U, u32, u32, *const fc4_advice_subject)>,
    // growth developers (the game's cISC4GrowthDeveloper slots)
    /// A new growth developer (class `0x0990C138`) with one reference for
    /// the core; 0 = none.
    pub create_developer: Option<unsafe extern "C" fn(U, *const fc4_res_key) -> u64>,
    /// `Init(holder)` (slot 3) with the game's holder of the exemplar `key`.
    pub developer_init: Option<unsafe extern "C" fn(U, u64, *const fc4_res_key) -> bool>,
    pub developer_shutdown: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub developer_sub_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub developer_purpose: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub developer_lists_valid: Option<unsafe extern "C" fn(U, u64) -> bool>,
    /// `GetGrowthList()` (which 0) or `GetDeclineList()` (1), delivered to
    /// `sink` in one call.
    pub developer_list: Option<unsafe extern "C" fn(U, u64, u32, fc4_growth_items_sink, *mut c_void)>,
    /// `GetSourceDemand()` (borrowed; 0 = none).
    pub developer_source_demand: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub developer_start_map_update: Option<unsafe extern "C" fn(U, u64)>,
    pub developer_map_update_finished: Option<unsafe extern "C" fn(U, u64) -> bool>,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_res_key>() == 12);
    assert!(std::mem::size_of::<fc4_growth_profile>() == 24);
    assert!(std::mem::size_of::<fc4_growth_item>() == 12);
    assert!(std::mem::size_of::<fc4_construction_profile>() == 0x24);
    assert!(std::mem::size_of::<fc4_lot_config>() == if cfg!(target_pointer_width = "32") { 36 } else { 40 });
    assert!(std::mem::size_of::<fc4_building_profile>() == 48);
    assert!(std::mem::size_of::<fc4_development_message>() == 40);
    assert!(std::mem::size_of::<fc4_command_param>() == 8);
    assert!(std::mem::size_of::<fc4_advice_subject>() == 24);
    assert!(std::mem::size_of::<fc4_plan_entry>() == 24);
    assert!(std::mem::size_of::<fc4_development_state>() == 208);
    assert!(std::mem::size_of::<fc4_development_host>() == if cfg!(target_pointer_width = "32") { 432 } else { 864 });
};

// ---- host adapter -----------------------------------------------------------------------------------

/// The counters of [`fc4_development_state`].
#[derive(Clone, Copy, Default)]
struct Counters {
    grow_calls: u32,
    built: u32,
    decline_calls: u32,
    declined: u32,
    built_by_wealth: [u32; 4],
    declined_by_wealth: [u32; 4],
    built_by_stage: [u32; 16],
}

struct CHost {
    t: fc4_development_host,
    /// Holder → localized name of the last exemplar enumeration
    /// (`exemplar_string`).
    names: RefCell<HashMap<usize, Vec<u8>>>,
    counters: RefCell<Counters>,
}

macro_rules! cb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.t.$f { Some(f) => unsafe { f($s.t.user $(, $arg)*) }, None => $default }
    };
}

fn opt(t: u64) -> Option<u64> {
    (t != 0).then_some(t)
}

fn str_arg(s: &str) -> (*const c_char, usize) {
    (s.as_ptr() as *const c_char, s.len())
}

fn rect_of(r: CellRect) -> fc4_cell_rect {
    fc4_cell_rect {
        x1: r.0,
        z1: r.1,
        x2: r.2,
        z2: r.3,
    }
}

/// The visitor context of `exemplars`.
#[derive(Default)]
struct ExemplarCtx {
    out: Vec<(ResKey, Rc<PropertyHolder>)>,
    names: HashMap<usize, Vec<u8>>,
}

unsafe extern "C" fn visit_exemplar(
    ctx: *mut c_void,
    key: *const fc4_res_key,
    props: *mut fc4_props,
    name: *const u8,
    name_len: usize,
    has_name: bool,
) {
    let Some(c) = (ctx as *mut ExemplarCtx).as_mut() else {
        return;
    };
    // The props are the core's whatever happens next.
    let holder = take_props(props);
    let (Some(k), Some(h)) = (key.as_ref(), holder) else {
        return;
    };
    if has_name {
        let bytes = if name.is_null() || name_len == 0 {
            Vec::new()
        } else {
            std::slice::from_raw_parts(name, name_len).to_vec()
        };
        c.names.insert(Rc::as_ptr(&h) as usize, bytes);
    }
    c.out.push((ResKey::from(*k), h));
}

unsafe extern "C" fn visit_lot_config(ctx: *mut c_void, config: *const fc4_lot_config) {
    let (Some(v), Some(c)) = ((ctx as *mut Vec<LotConfigInfo>).as_mut(), config.as_ref()) else {
        return;
    };
    let ids = if c.ids.is_null() || c.n_ids == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(c.ids, c.n_ids as usize).to_vec()
    };
    v.push(LotConfigInfo {
        size: (c.size_x, c.size_z),
        zones: c.zones as u16,
        primary: (c.has_primary != 0).then_some(BuildingConfigObject {
            ids,
            orientation: c.orientation,
            x: c.x,
            z: c.z,
        }),
    });
}

unsafe extern "C" fn collect_items(ctx: *mut c_void, items: *const fc4_growth_item, n: usize) {
    let Some(v) = (ctx as *mut Vec<GrowthItem>).as_mut() else {
        return;
    };
    if !items.is_null() && n > 0 {
        v.extend(
            std::slice::from_raw_parts(items, n)
                .iter()
                .map(|&i| GrowthItem::from(i)),
        );
    }
}

impl CHost {
    fn new(t: fc4_development_host) -> CHost {
        CHost {
            t,
            names: RefCell::new(HashMap::new()),
            counters: RefCell::new(Counters::default()),
        }
    }

    fn release_token(&self, token: u64) {
        if token != 0 {
            cb!(self, release, (), token)
        }
    }

    fn enumerate(&self, types: &[u32]) -> Vec<(ResKey, Rc<PropertyHolder>)> {
        let mut ctx = ExemplarCtx::default();
        cb!(
            self,
            exemplars,
            (),
            types.as_ptr(),
            types.len(),
            Some(visit_exemplar),
            &mut ctx as *mut ExemplarCtx as *mut c_void
        );
        *self.names.borrow_mut() = ctx.names;
        ctx.out
    }

    fn developer_wealth(&self, dev: u64) -> usize {
        (wealth_of(self.developer_sub_type(dev)) as usize).min(3)
    }
}

impl GridProvider<u8> for CHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        // The BDS releases its building map without a Shutdown (§1.4).
        create_grid::<u8>(&self.t.grids, instance, tract_size, false)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<u8>>> {
        resolve_grid::<u8>(&self.t.grids, r, false)
    }
}

/// The construction node: a game `cISC4Demand` token holding one reference.
struct HostNode {
    t: fc4_development_host,
    token: u64,
}

macro_rules! ncb {
    ($s:expr, $f:ident, $default:expr $(, $arg:expr)*) => {
        // SAFETY: whoever created the object promised valid callbacks.
        match $s.t.$f { Some(f) => unsafe { f($s.t.user, $s.token $(, $arg)*) }, None => $default }
    };
}

impl Drop for HostNode {
    fn drop(&mut self) {
        ncb!(self, release, ())
    }
}

impl ConstructionNode for HostNode {
    fn init(&self) {
        ncb!(self, node_init, ())
    }
    fn shutdown(&self) {
        ncb!(self, node_shutdown, ())
    }
    fn supply(&self) -> f32 {
        ncb!(self, node_supply, 0.0)
    }
    fn demand(&self) -> f32 {
        ncb!(self, node_demand, 0.0)
    }
    fn set_supply(&self, v: f32) {
        ncb!(self, node_set_supply, (), v)
    }
    fn set_demand(&self, v: f32) {
        ncb!(self, node_set_demand, (), v)
    }
    fn add_to_demand(&self, v: f32) {
        ncb!(self, node_add_to_demand, (), v)
    }
    fn end_of_cycle(&self) -> bool {
        ncb!(self, node_end_of_cycle, false)
    }
    fn save_ref(self: Rc<Self>) -> RefValue {
        RefValue::Host {
            token: self.token,
            class_id: CLSID_DEMAND_NODE,
        }
    }
}

fn node_token(n: &Option<Rc<dyn ConstructionNode>>) -> u64 {
    match n.as_ref().map(|n| n.clone().save_ref()) {
        Some(RefValue::Host { token, .. }) => token,
        _ => 0,
    }
}

impl DevelopmentHost for CHost {
    fn sim_day(&self) -> u32 {
        cb!(self, sim_day, 0)
    }
    fn difficulty(&self) -> u32 {
        cb!(self, difficulty, 0)
    }
    fn cell_widths(&self) -> (f32, f32) {
        let mut v = [16.0f32; 2];
        cb!(self, cell_widths, (), v.as_mut_ptr());
        (v[0], v[1])
    }
    fn terrain_cell_counts(&self) -> (i32, i32) {
        let mut v = [0i32; 2];
        cb!(self, terrain_cell_counts, (), v.as_mut_ptr());
        (v[0], v[1])
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        let mut v = [0i32; 2];
        cb!(self, position_to_cell, (), x, z, v.as_mut_ptr());
        (v[0], v[1])
    }
    fn cell_center(&self, cx: i32, cz: i32) -> (f32, f32) {
        let mut v = [0.0f32; 2];
        cb!(self, cell_center, (), cx, cz, v.as_mut_ptr());
        (v[0], v[1])
    }
    fn altitude(&self, x: f32, z: f32) -> f32 {
        cb!(self, altitude, 0.0, x, z)
    }

    fn exemplars(&self) -> Vec<(ResKey, Rc<PropertyHolder>)> {
        self.enumerate(&[])
    }
    fn exemplars_of_types(&self, types: &[u32]) -> Vec<(ResKey, Rc<PropertyHolder>)> {
        if types.is_empty() {
            return Vec::new();
        }
        self.enumerate(types)
    }
    fn exemplar_string(&self, holder: &PropertyHolder, id: u32) -> Option<Vec<u8>> {
        if id != prop::LOCALIZED_NAME {
            return None;
        }
        let key = holder as *const PropertyHolder as usize;
        self.names.borrow().get(&key).cloned()
    }
    fn lot_configurations(&self) -> Vec<LotConfigInfo> {
        let mut v: Vec<LotConfigInfo> = Vec::new();
        cb!(
            self,
            lot_configurations,
            (),
            Some(visit_lot_config),
            &mut v as *mut Vec<LotConfigInfo> as *mut c_void
        );
        v
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, tuning, std::ptr::null_mut());
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }

    fn register_static_cleanup(&self) {
        cb!(self, register_static_cleanup, ())
    }
    fn post_building_types_updated(&self, count: u32) {
        cb!(self, post_building_types_updated, (), count)
    }
    fn subscribe(&self, msg_types: &[u32]) {
        cb!(self, subscribe, (), msg_types.as_ptr(), msg_types.len())
    }
    fn unsubscribe(&self, msg_types: &[u32]) {
        cb!(self, unsubscribe, (), msg_types.as_ptr(), msg_types.len())
    }
    fn register_command(&self, id: u32, name: &str) {
        let (p, n) = str_arg(name);
        cb!(self, register_command, (), id, p, n)
    }
    fn unregister_command(&self, id: u32) {
        cb!(self, unregister_command, (), id)
    }
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        let (p, n) = str_arg(name);
        cb!(self, add_agent, (), level, p, n, flags)
    }
    fn remove_agent(&self) {
        cb!(self, remove_agent, ())
    }
    fn register_cheat(&self, id: u32, name: &str) {
        let (p, n) = str_arg(name);
        cb!(self, register_cheat, (), id, p, n)
    }
    fn unregister_cheat(&self, id: u32) {
        cb!(self, unregister_cheat, (), id)
    }
    fn acquire_services(&self) {
        cb!(self, acquire_services, ())
    }
    fn release_services(&self) {
        cb!(self, release_services, ())
    }

    fn census_supply(&self, id: u32) -> Option<f32> {
        let mut v = 0.0f32;
        cb!(self, census_supply, false, id, &mut v).then_some(v)
    }
    fn create_construction_node(&self) -> Option<Rc<dyn ConstructionNode>> {
        let token = opt(cb!(self, create_construction_node, 0))?;
        Some(Rc::new(HostNode { t: self.t, token }) as Rc<dyn ConstructionNode>)
    }
    fn resolve_construction_node(&self, r: &RefValue) -> Option<Rc<dyn ConstructionNode>> {
        let RefValue::Host { token, .. } = *r else {
            return None;
        };
        let token = opt(token)?;
        Some(Rc::new(HostNode { t: self.t, token }) as Rc<dyn ConstructionNode>)
    }
    fn demand_active_value(&self, node: u64) -> f32 {
        cb!(self, demand_active_value, 0.0, node)
    }

    fn tract_grow(&self, profile: &GrowthProfile, _outcomes: &mut Vec<Outcome>, flag: &mut bool) -> i32 {
        let p = fc4_growth_profile::from(profile);
        let built = cb!(self, tract_grow, 0, &p, flag as *mut bool);
        let wealth = if built > 0 {
            self.developer_wealth(profile.developer)
        } else {
            0
        };
        let mut c = self.counters.borrow_mut();
        c.grow_calls = c.grow_calls.wrapping_add(1);
        if built > 0 {
            c.built = c.built.wrapping_add(1);
            c.built_by_wealth[wealth] = c.built_by_wealth[wealth].wrapping_add(1);
            let s = profile.stage.clamp(0, 15) as usize;
            c.built_by_stage[s] = c.built_by_stage[s].wrapping_add(1);
        }
        built
    }
    fn tract_decline(&self, profile: &GrowthProfile) -> i32 {
        let p = fc4_growth_profile::from(profile);
        let result = cb!(self, tract_decline, 0, &p);
        let wealth = if result > 0 {
            self.developer_wealth(profile.developer)
        } else {
            0
        };
        let mut c = self.counters.borrow_mut();
        c.decline_calls = c.decline_calls.wrapping_add(1);
        if result > 0 {
            c.declined = c.declined.wrapping_add(1);
            c.declined_by_wealth[wealth] = c.declined_by_wealth[wealth].wrapping_add(1);
        }
        result
    }

    fn lot_building_type(&self, lot: LotId) -> u32 {
        cb!(self, lot_building_type, 0, lot)
    }
    fn lot_bounding_rect(&self, lot: LotId) -> CellRect {
        let mut r = fc4_cell_rect::default();
        cb!(self, lot_bounding_rect, (), lot, &mut r);
        (r.x1, r.z1, r.x2, r.z2)
    }
    fn lot_growth_stage(&self, lot: LotId) -> Option<u8> {
        let mut s = 0u8;
        cb!(self, lot_growth_stage, false, lot, &mut s).then_some(s)
    }
    fn lot_building(&self, lot: LotId) -> Option<OccupantId> {
        opt(cb!(self, lot_building, 0, lot))
    }
    fn lot_state(&self, lot: LotId) -> u32 {
        cb!(self, lot_state, 0, lot)
    }
    fn lot_center(&self, lot: LotId) -> Vec3 {
        let mut v = [0.0f32; 3];
        cb!(self, lot_center, (), lot, v.as_mut_ptr());
        v
    }
    fn lot_slope(&self, lot: LotId) -> f32 {
        cb!(self, lot_slope, 0.0, lot)
    }
    fn lot_demolishable(&self, lot: LotId) -> bool {
        cb!(self, lot_demolishable, false, lot)
    }
    fn lot_at(&self, x: i32, z: i32) -> Option<LotId> {
        opt(cb!(self, lot_at, 0, x, z))
    }
    fn lots_in_cell_rect(&self, rect: CellRect) -> Vec<LotId> {
        let r = rect_of(rect);
        collect_tokens(|f, ctx| cb!(self, lots_in_cell_rect, (), &r, f, ctx))
            .into_iter()
            .filter(|&t| t != 0)
            .collect()
    }
    fn lot_developer_slope(&self, lot: LotId) -> f32 {
        cb!(self, lot_developer_slope, 0.0, lot)
    }
    fn update_lot_elements(&self, lot: LotId, kind: u32) {
        cb!(self, update_lot_elements, (), lot, kind)
    }
    fn area_altitude(&self, rect: [f32; 4]) -> f32 {
        cb!(self, area_altitude, 0.0, rect.as_ptr())
    }

    fn zone_type(&self, x: i32, z: i32) -> i8 {
        cb!(self, zone_type, 0, x, z)
    }
    fn dezone(&self, rect: CellRect, flag: bool) {
        let r = rect_of(rect);
        cb!(self, dezone, (), &r, flag)
    }
    fn powered(&self, x: i32, z: i32) -> bool {
        cb!(self, powered, false, x, z)
    }
    fn land_value_type(&self, x: i32, z: i32) -> u32 {
        cb!(self, land_value_type, 0, x, z)
    }
    fn land_value(&self, x: i32, z: i32) -> u8 {
        cb!(self, land_value, 0, x, z)
    }
    fn demolish_lots(&self, lots: &[LotId]) -> i64 {
        cb!(self, demolish_lots, 0, lots.as_ptr(), lots.len())
    }
    fn total_funds(&self) -> i64 {
        cb!(self, total_funds, 0)
    }
    fn set_total_funds(&self, v: i64) {
        cb!(self, set_total_funds, (), v)
    }

    fn has_occupant_manager(&self) -> bool {
        cb!(self, has_occupant_manager, false)
    }
    fn insert_occupant(&self, occ: OccupantId) -> bool {
        cb!(self, insert_occupant, false, occ)
    }
    fn remove_occupant(&self, occ: OccupantId) -> bool {
        cb!(self, remove_occupant, false, occ)
    }
    fn building_occupant_at_cell(&self, cx: i32, cz: i32) -> bool {
        cb!(self, building_occupant_at_cell, false, cx, cz)
    }
    fn occupant_has_property(&self, occ: OccupantId, id: u32) -> bool {
        cb!(self, occupant_has_property, false, occ, id)
    }
    fn occupant_property_i64(&self, occ: OccupantId, id: u32) -> Option<i64> {
        let mut v = 0i64;
        cb!(self, occupant_property_i64, false, occ, id, &mut v).then_some(v)
    }
    fn occupant_in_group(&self, occ: OccupantId, group: u32) -> bool {
        cb!(self, occupant_in_group, false, occ, group)
    }
    fn occupant_position(&self, occ: OccupantId) -> Vec3 {
        let mut v = [0.0f32; 3];
        cb!(self, occupant_position, (), occ, v.as_mut_ptr());
        v
    }
    fn create_building_occupant(&self) -> Option<OccupantId> {
        opt(cb!(self, create_building_occupant, 0))
    }
    fn set_building_type(&self, occ: OccupantId, t: u32) {
        cb!(self, set_building_type, (), occ, t)
    }
    fn building_type(&self, occ: OccupantId) -> u32 {
        cb!(self, building_type, 0, occ)
    }
    fn set_default_exemplar(&self, occ: OccupantId, key: ResKey) -> bool {
        let k = fc4_res_key::from(key);
        cb!(self, set_default_exemplar, false, occ, &k)
    }
    fn occupant_property_f32s(&self, occ: OccupantId, id: u32) -> Option<Vec<f32>> {
        // The loaders read at most three values (§4.9.1, §4.9.2).
        let mut v = [0.0f32; 16];
        let mut count = 0usize;
        let found = cb!(
            self,
            occupant_property_f32s,
            false,
            occ,
            id,
            v.as_mut_ptr(),
            v.len(),
            &mut count
        );
        found.then(|| v[..count.min(v.len())].to_vec())
    }
    fn occupant_bounding_box(&self, occ: OccupantId) -> (Vec3, Vec3) {
        let mut v = [0.0f32; 6];
        cb!(self, occupant_bounding_box, (), occ, v.as_mut_ptr());
        ([v[0], v[1], v[2]], [v[3], v[4], v[5]])
    }
    fn set_occupant_bounding_box(&self, occ: OccupantId, min: Vec3, max: Vec3) {
        let v = [min[0], min[1], min[2], max[0], max[1], max[2]];
        cb!(self, set_occupant_bounding_box, (), occ, v.as_ptr())
    }
    fn set_occupant_position(&self, occ: OccupantId, pos: Vec3) {
        cb!(self, set_occupant_position, (), occ, pos.as_ptr())
    }
    fn set_occupant_orientation(&self, occ: OccupantId, orientation: i32) {
        cb!(self, set_occupant_orientation, (), occ, orientation)
    }
    fn init_occupant(&self, occ: OccupantId) {
        cb!(self, init_occupant, (), occ)
    }
    fn add_ref(&self, occ: OccupantId) {
        if occ != 0 {
            cb!(self, add_ref, (), occ)
        }
    }
    fn release(&self, occ: OccupantId) {
        self.release_token(occ)
    }
    fn object_ref(&self, occ: OccupantId) -> RefValue {
        if occ == 0 {
            RefValue::Null
        } else {
            RefValue::Host {
                token: occ,
                class_id: 0,
            }
        }
    }
    fn resolve_occupant(&self, r: &RefValue) -> Option<OccupantId> {
        match *r {
            RefValue::Host { token, .. } => opt(token),
            _ => None,
        }
    }

    fn create_foundation_occupant(&self) -> Option<OccupantId> {
        opt(cb!(self, create_foundation_occupant, 0))
    }
    fn foundation_type(&self, occ: OccupantId) -> u32 {
        cb!(self, foundation_type, 0, occ)
    }
    fn set_foundation_type(&self, occ: OccupantId, t: u32) {
        cb!(self, set_foundation_type, (), occ, t)
    }
    fn set_foundation_max_height(&self, occ: OccupantId, v: f32) {
        cb!(self, set_foundation_max_height, (), occ, v)
    }
    fn set_foundation_size(&self, occ: OccupantId, size: Vec3) {
        cb!(self, set_foundation_size, (), occ, size.as_ptr())
    }
    fn set_foundation_usage(&self, occ: OccupantId, usage: i32) {
        cb!(self, set_foundation_usage, (), occ, usage)
    }
    fn set_foundation_orientation(&self, occ: OccupantId, orientation: i32) {
        cb!(self, set_foundation_orientation, (), occ, orientation)
    }
    fn retaining_wall_type(&self, occ: OccupantId) -> u32 {
        cb!(self, retaining_wall_type, 0, occ)
    }
    fn area_altitude_cells(&self, rect: CellRect, lowest: bool, with_water: bool) -> f32 {
        let r = rect_of(rect);
        cb!(self, area_altitude_cells, 0.0, &r, lowest, with_water)
    }

    fn push_advisor_event(&self, advisor: u32, event: u32, discarded: &AdviceSubject) {
        let s = fc4_advice_subject {
            x: discarded.x,
            z: discarded.z,
            data1: discarded.data1,
            reserved: 0,
            lot: discarded.lot,
        };
        cb!(self, push_advisor_event, (), advisor, event, &s)
    }
}

impl GrowthDevelopers for CHost {
    fn create_developer(&self, key: &ResKey, _exemplar: &Rc<PropertyHolder>) -> Option<DeveloperId> {
        let k = fc4_res_key::from(*key);
        opt(cb!(self, create_developer, 0, &k))
    }
    // The developer is a game object: the host hands it the game's own
    // holder of `key` (the core's copy of the exemplar is not passed on).
    fn developer_init(&self, dev: DeveloperId, key: &ResKey, _exemplar: &Rc<PropertyHolder>) -> bool {
        let k = fc4_res_key::from(*key);
        cb!(self, developer_init, false, dev, &k)
    }
    fn developer_shutdown(&self, dev: DeveloperId) -> bool {
        cb!(self, developer_shutdown, false, dev)
    }
    fn developer_release(&self, dev: DeveloperId) {
        self.release_token(dev)
    }
    fn developer_sub_type(&self, dev: DeveloperId) -> u32 {
        cb!(self, developer_sub_type, 0, dev)
    }
    fn developer_purpose(&self, dev: DeveloperId) -> u32 {
        cb!(self, developer_purpose, 0, dev)
    }
    fn developer_lists_valid(&self, dev: DeveloperId) -> bool {
        cb!(self, developer_lists_valid, false, dev)
    }
    fn developer_growth_list(&self, dev: DeveloperId) -> Vec<GrowthItem> {
        let mut v: Vec<GrowthItem> = Vec::new();
        cb!(
            self,
            developer_list,
            (),
            dev,
            0,
            Some(collect_items),
            &mut v as *mut Vec<GrowthItem> as *mut c_void
        );
        v
    }
    fn developer_decline_list(&self, dev: DeveloperId) -> Vec<GrowthItem> {
        let mut v: Vec<GrowthItem> = Vec::new();
        cb!(
            self,
            developer_list,
            (),
            dev,
            1,
            Some(collect_items),
            &mut v as *mut Vec<GrowthItem> as *mut c_void
        );
        v
    }
    fn developer_source_demand(&self, dev: DeveloperId) -> Option<u64> {
        opt(cb!(self, developer_source_demand, 0, dev))
    }
    fn developer_start_map_update(&self, dev: DeveloperId) {
        cb!(self, developer_start_map_update, (), dev)
    }
    fn developer_map_update_finished(&self, dev: DeveloperId) -> bool {
        cb!(self, developer_map_update_finished, true, dev)
    }
    fn developer_ref(&self, dev: DeveloperId) -> RefValue {
        if dev == 0 {
            RefValue::Null
        } else {
            RefValue::Host {
                token: dev,
                class_id: CLSID_GROWTH_DEVELOPER,
            }
        }
    }
    fn resolve_developer(&self, r: &RefValue) -> Option<DeveloperId> {
        match *r {
            RefValue::Host { token, .. } => opt(token),
            _ => None,
        }
    }
}

// ---- the save stream --------------------------------------------------------------------------------

/// The record reader: delegates to [`CIn`], except that references come back
/// as host tokens the core owns one reference on.
struct DIn<'a> {
    inner: CIn<'a>,
    host: &'a CHost,
}

impl InStream for DIn<'_> {
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
        let f = self.inner.stream.get_ref?;
        let mut out = fc4_ref::default();
        // SAFETY: valid callback; `out` outlives the call.
        if !unsafe { f(self.inner.stream.user, iid, &mut out) } {
            return None;
        }
        if out.token == 0 {
            return Some(RefValue::Null);
        }
        // One of FreeCity4's own objects comes without a reference (fc4.h
        // fc4_ref): take the one the core will own.
        if !out.core_handle.is_null() {
            cb!(self.host, add_ref, (), out.token);
        }
        Some(RefValue::Host {
            token: out.token,
            class_id: out.class_id,
        })
    }
    fn skip(&mut self, n: usize) -> bool {
        self.inner.skip(n)
    }
    fn error(&self) -> i32 {
        self.inner.error()
    }
}

fn no_core_objects(_: u32, _: *mut c_void) -> Option<Rc<dyn SaveObject>> {
    None
}

// ---- the process statics -------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_development_process`): the process-wide statics
/// of §2.
#[repr(C)]
pub struct fc4_development_process {
    _private: [u8; 0],
}

struct Process(SharedProcess);

fn with_process<R>(pp: *const fc4_development_process, fallback: R, f: impl FnOnce(&SharedProcess) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_development_process*.
    let Some(p) = (unsafe { (pp as *const Process).as_ref() }) else {
        return fallback;
    };
    guard(fallback, || f(&p.0))
}

/// The process statics, the BDS generator seeded from the session seed
/// (foundations §6.3 *[choice]*: `derive_seed(session_seed, 0x0C09C0B8)`).
#[no_mangle]
pub extern "C" fn fc4_development_process_create(session_seed: u32) -> *mut fc4_development_process {
    guard(std::ptr::null_mut(), || {
        let seeds = FixedSeeds {
            wall_clock: session_seed,
            process_clock: session_seed,
        };
        Box::into_raw(Box::new(Process(BdsProcess::from_session(session_seed, &seeds)))) as *mut fc4_development_process
    })
}

/// Destroys the process statics (the BDS objects made with them keep their
/// own share).
#[no_mangle]
pub unsafe extern "C" fn fc4_development_process_destroy(pp: *mut fc4_development_process) {
    if pp.is_null() {
        return;
    }
    guard((), || drop(Box::from_raw(pp as *mut Process)));
}

/// Whether the static catalogue has been loaded (the first `Init` of the
/// process, §1.3 step 1).
#[no_mangle]
pub unsafe extern "C" fn fc4_development_catalogue_loaded(pp: *const fc4_development_process) -> bool {
    with_process(pp, false, |p| p.catalogue().is_loaded())
}

/// Copies one type list of the catalogue (in the original's hash order) into
/// `out` (at most `n`); the list's length. `which`: 0 building types (slot
/// 11), 1 family types (13), 2 foundation types (21), 3 retaining wall types
/// (27), 4 the types of the profile map (zero profiles included).
#[no_mangle]
pub unsafe extern "C" fn fc4_development_catalogue_types(
    pp: *const fc4_development_process,
    which: u32,
    out: *mut u32,
    n: usize,
) -> usize {
    let v = with_process(pp, Vec::new(), |p| {
        let c = p.catalogue();
        match which {
            0 => c.all_building_types(),
            1 => c.all_family_types(),
            2 => c.all_foundation_types(),
            3 => c.all_retaining_wall_types(),
            4 => c.profile_types(),
            _ => Vec::new(),
        }
    });
    if !out.is_null() {
        std::ptr::copy_nonoverlapping(v.as_ptr(), out, v.len().min(n));
    }
    v.len()
}

/// The members of family `f` in load order into `out` (at most `n`); their
/// number (0 when `f` is not a family).
#[no_mangle]
pub unsafe extern "C" fn fc4_development_catalogue_family(
    pp: *const fc4_development_process,
    f: u32,
    out: *mut u32,
    n: usize,
) -> usize {
    let v = with_process(pp, Vec::new(), |p| {
        p.catalogue().family(f).map(<[u32]>::to_vec).unwrap_or_default()
    });
    if !out.is_null() {
        std::ptr::copy_nonoverlapping(v.as_ptr(), out, v.len().min(n));
    }
    v.len()
}

/// The exemplar key of a building (`which` 0, slot 14), foundation (2, slot
/// 22) or retaining wall (3, slot 28) type; false when unknown.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_catalogue_key(
    pp: *const fc4_development_process,
    which: u32,
    t: u32,
    out: *mut fc4_res_key,
) -> bool {
    let k = with_process(pp, None, |p| {
        let c = p.catalogue();
        match which {
            0 => c.building_key(t),
            2 => c.foundation_key(t),
            3 => c.retaining_wall_key(t),
            _ => None,
        }
    });
    let Some(k) = k else { return false };
    if let Some(o) = out.as_mut() {
        *o = k.into();
    }
    true
}

/// The exemplar name (`which` 0, slot 37) or building name (1, slot 38) of a
/// building type, delivered to `sink`; false (and no call) for a type that is
/// not a building (the host's copy inserts "" then, §4.7.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_development_catalogue_name(
    pp: *const fc4_development_process,
    which: u32,
    t: u32,
    sink: fc4_bytes_sink,
    ctx: *mut c_void,
) -> bool {
    let name = with_process(pp, None, |p| {
        if !p.catalogue().is_building_type_valid(t) {
            return None;
        }
        let mut c = p.catalogue_mut();
        match which {
            0 => Some(c.exemplar_name(t)),
            1 => Some(c.building_name(t)),
            _ => None,
        }
    });
    let Some(name) = name else { return false };
    if let Some(f) = sink {
        f(ctx, name.as_ptr(), name.len());
    }
    true
}

/// The profile of type `t` as the catalogue holds it (without inserting);
/// `cb` once per capacity table entry (developer type, amount; ascending).
/// False when the type has no profile.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_catalogue_profile(
    pp: *const fc4_development_process,
    t: u32,
    out: *mut fc4_building_profile,
    cb: fc4_type_value_callback,
    ctx: *mut c_void,
) -> bool {
    let Some(p) = with_process(pp, None, |p| p.catalogue().profile(t).cloned()) else {
        return false;
    };
    if let Some(o) = out.as_mut() {
        *o = fc4_building_profile {
            purpose: u32::from(p.purpose),
            wealth: u32::from(p.wealth),
            capacity: p.capacity,
            value_499afa38: p.value_499afa38,
            flag_27812836: u8::from(p.flag_27812836),
            category_flags: p.category_flags,
            occupant_size: p.occupant_size,
            reserved0: 0,
            plop_cost: p.plop_cost,
            n_capacities: p.capacities.len() as u32,
            capacities_present: u32::from(p.capacities_present),
        };
    }
    if let Some(f) = cb {
        for (&k, &v) in &p.capacities {
            f(ctx, k, u32::from(v));
        }
    }
    true
}

/// The maximum density table (§4.7.4) in key order: `cb` once per entry.
/// Returns the number of entries.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_catalogue_density(
    pp: *const fc4_development_process,
    cb: fc4_density_callback,
    ctx: *mut c_void,
) -> usize {
    let entries: Vec<(u32, u8, f32)> = with_process(pp, Vec::new(), |p| {
        p.catalogue()
            .density_table()
            .iter()
            .map(|(&(z, k), &v)| (z, k, v))
            .collect()
    });
    if let Some(f) = cb {
        for &(z, k, v) in &entries {
            let cap = fc4_core::x87::ftol32(f64::from(v)) as f32;
            f(ctx, z, u32::from(k), v, cap);
        }
    }
    entries.len()
}

/// `GetMaxCapacity(zone, p, w)` (slot 45, §4.7.4) from the process catalogue.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_catalogue_max_capacity(
    pp: *const fc4_development_process,
    zone: u32,
    purpose: u32,
    wealth: u32,
) -> f32 {
    with_process(pp, 0.0, |p| p.catalogue().max_capacity(zone, purpose, wealth))
}

// ---- pure helpers (§4.7.3) --------------------------------------------------------------------------

/// `GetDeveloperType(p, w)` (slot 30).
#[no_mangle]
pub extern "C" fn fc4_development_developer_type(purpose: u32, wealth: u32) -> u32 {
    developer_type(purpose, wealth)
}

/// `GetWealth(devType)` (slot 31).
#[no_mangle]
pub extern "C" fn fc4_development_wealth_of(dev_type: u32) -> u32 {
    wealth_of(dev_type)
}

/// `GetBuildingMapKey(p, w)` (slot 42), zero-extended.
#[no_mangle]
pub extern "C" fn fc4_development_building_map_key(purpose: u32, wealth: u32) -> u32 {
    u32::from(building_map_key(purpose, wealth))
}

/// Which sub-object `QueryInterface(iid)` answers with (§1.1): 0 = none (AL
/// = 0, `*ppv` unchanged), 1 = the primary interface (also `cIGZUnknown`),
/// 2 = `cIGZMessageTarget` (+4), 3 = `cIGZMessageTarget2` (+8), 4 =
/// `cIGZSerializable` (+0xC).
#[no_mangle]
pub extern "C" fn fc4_development_query_interface(iid: u32) -> u32 {
    match query_interface(iid) {
        None => 0,
        Some(BdsInterface::Primary) => 1,
        Some(BdsInterface::MessageTarget) => 2,
        Some(BdsInterface::MessageTarget2) => 3,
        Some(BdsInterface::Serializable) => 4,
    }
}

// ---- handles ----------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_development`).
#[repr(C)]
pub struct fc4_development {
    _private: [u8; 0],
}

struct Handle {
    host: Rc<CHost>,
    sim: Rc<DevelopmentSim>,
    depth: Cell<u32>,
    queue: RefCell<Vec<Message>>,
    reentrant: Cell<u32>,
}

impl Handle {
    /// Runs `f` with the call depth counted; at the end of the outermost
    /// call, the queued messages are handled.
    fn call<R>(&self, f: impl FnOnce(&DevelopmentSim) -> R) -> R {
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
                self.sim.do_message(&m);
            }
        }
        self.depth.set(self.depth.get() - 1);
        r
    }
}

/// Runs `f` on the handle; the fallback for NULL or a panic.
fn with_handle<R>(p: *const fc4_development, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_development*.
    let Some(h) = (unsafe { (p as *const Handle).as_ref() }) else {
        return fallback;
    };
    guard(fallback, || f(h))
}

fn sim<R>(p: *const fc4_development, fallback: R, f: impl FnOnce(&DevelopmentSim) -> R) -> R {
    with_handle(p, fallback, |h| h.call(f))
}

fn host_ok(host: *const fc4_development_host) -> Option<fc4_development_host> {
    // SAFETY: documented precondition: NULL or a valid host table.
    let h = unsafe { host.as_ref() }?;
    ((h.struct_size as usize) >= std::mem::size_of::<fc4_development_host>()).then_some(*h)
}

/// Creates the BDS (constructor values, §2) over the host table (copied) and
/// the process statics `pp` (shared). `token` is the host identity of the
/// BDS itself. NULL when the table or the grid API is incomplete or `pp` is
/// NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_create(
    host: *const fc4_development_host,
    pp: *const fc4_development_process,
    token: u64,
) -> *mut fc4_development {
    guard(std::ptr::null_mut(), || {
        let Some(h) = host_ok(host) else {
            return std::ptr::null_mut();
        };
        if !h.grids.complete() {
            return std::ptr::null_mut();
        }
        let Some(process) = (pp as *const Process).as_ref().map(|p| p.0.clone()) else {
            return std::ptr::null_mut();
        };
        let host = Rc::new(CHost::new(h));
        let sim = DevelopmentSim::new(host.clone(), host.clone(), process, token);
        Box::into_raw(Box::new(Handle {
            host,
            sim,
            depth: Cell::new(0),
            queue: RefCell::new(Vec::new()),
            reentrant: Cell::new(0),
        })) as *mut fc4_development
    })
}

/// Destroys the BDS. The references it still holds (a loaded object that was
/// never initialised, or one never shut down) are released: developers,
/// special buildings, the building map and the construction node.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_destroy(p: *mut fc4_development) {
    if p.is_null() {
        return;
    }
    guard((), || {
        let b = Box::from_raw(p as *mut Handle);
        let (devs, special) = {
            let mut d = b.sim.data_mut();
            (
                std::mem::take(&mut d.developers),
                std::mem::take(&mut d.special_buildings),
            )
        };
        for dev in devs.into_values() {
            b.host.release_token(dev);
        }
        for occ in special {
            b.host.release_token(occ);
        }
        b.sim.set_building_map(None);
        b.sim.set_construction_node(None);
        drop(b);
    });
}

/// `Init` (§1.3): always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_init(p: *mut fc4_development) -> bool {
    sim(p, false, DevelopmentSim::init)
}

/// `Shutdown` (§1.4): always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_shutdown(p: *mut fc4_development) -> bool {
    sim(p, false, DevelopmentSim::shutdown)
}

/// `DoMessage` (§3): always true. `params` are the PlaceBuilding command's
/// parameters. A message delivered during another call is queued (the host
/// keeps its tokens alive until the outermost call returns).
#[no_mangle]
pub unsafe extern "C" fn fc4_development_do_message(
    p: *mut fc4_development,
    m: *const fc4_development_message,
    params: *const fc4_command_param,
    n_params: usize,
) -> bool {
    let Some(m) = m.as_ref() else { return true };
    let params = if params.is_null() || n_params == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(params, n_params)
            .iter()
            .map(|c| CommandParam {
                as_u32: c.as_u32,
                as_f32: c.as_f32,
            })
            .collect()
    };
    let msg = Message {
        msg_type: m.msg_type,
        data1: m.data1,
        data2: m.data2,
        data3: m.data3,
        data4: m.data4,
        occupant: opt(m.occupant),
        lot: opt(m.lot),
        params,
    };
    with_handle(p, true, |h| {
        if h.depth.get() > 0 {
            h.queue.borrow_mut().push(msg);
            return true;
        }
        h.call(|s| s.do_message(&msg))
    })
}

/// The daily growth budget (§4.4.1): at most `n` tract developer `Grow`
/// calls per day; 0 = unlimited.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_set_grow_budget(p: *mut fc4_development, n: u32) {
    sim(p, (), |s| s.set_grow_attempt_budget(if n == 0 { u32::MAX } else { n }))
}

// ---- the primary interface (§7.1) ------------------------------------------------------------------

/// Slot 5 `LocationIsOKForNewBuilding(type, x, z)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_location_is_ok(p: *const fc4_development, t: u32, x: f32, z: f32) -> bool {
    sim(p, false, |s| s.location_is_ok_for_new_building(t, x, z))
}

/// Slot 6 `AddNewBuildingOccupant(type, cx, cz, orientation)`: the occupant
/// (borrowed: the occupant manager holds it) or 0.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_add_building_at_cell(
    p: *mut fc4_development,
    t: u32,
    cx: i32,
    cz: i32,
    orientation: i32,
) -> u64 {
    sim(p, None, |s| s.add_new_building_occupant_at_cell(t, cx, cz, orientation)).unwrap_or(0)
}

/// Slot 7 `AddNewBuildingOccupant(type, pos[3], orientation)`: as slot 6.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_add_building(
    p: *mut fc4_development,
    t: u32,
    pos: *const f32,
    orientation: i32,
) -> u64 {
    if pos.is_null() {
        return 0;
    }
    let v = [*pos, *pos.add(1), *pos.add(2)];
    sim(p, None, |s| s.add_new_building_occupant(t, v, orientation)).unwrap_or(0)
}

/// Slots 8 and 24 `AddNewBuildingOccupant(occ)` / `AddNewFoundationOccupant(occ)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_insert_occupant(p: *mut fc4_development, occ: u64) -> bool {
    sim(p, false, |s| s.insert_building_occupant(occ))
}

/// Slot 10 `GetCostOfOccupantDemolition(building, u)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_demolition_cost(p: *const fc4_development, occ: u64) -> i64 {
    sim(p, 0, |s| s.cost_of_building_demolition(occ))
}

/// Slot 25 `DemolishFoundationOccupant(occ, u)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_demolish_foundation(p: *mut fc4_development, occ: u64) -> bool {
    sim(p, false, |s| s.demolish_foundation_occupant(occ))
}

/// Slot 23 `AddNewFoundationOccupant(type, usage, pos[3], sizeX, sizeZ,
/// orientation, base)` (§4.9.2, DG6): `pos[1]` is written when it was
/// below 0. False only when the occupant cannot be created, the type's
/// properties do not load or the insert fails.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fc4_development_add_foundation(
    p: *mut fc4_development,
    t: u32,
    usage: i32,
    pos: *mut f32,
    size_x: f32,
    size_z: f32,
    orientation: i32,
    base: f32,
) -> bool {
    if pos.is_null() {
        return false;
    }
    let mut v = [*pos, *pos.add(1), *pos.add(2)];
    let ok = sim(p, false, |s| {
        s.add_new_foundation_occupant(t, usage, &mut v, size_x, size_z, orientation, base)
    });
    *pos.add(1) = v[1];
    ok
}

/// Slot 26 `LoadFoundationProperties(occ, type)` (§4.9.2, DG6).
#[no_mangle]
pub unsafe extern "C" fn fc4_development_load_foundation(p: *mut fc4_development, occ: u64, t: u32) -> bool {
    sim(p, false, |s| s.load_foundation_properties(occ, t))
}

/// Slot 29 `LoadLotRetainingWallProperties(occ, type)` (§4.9.2, DG6); `occ`
/// is the wall's occupant token (the host answers `retaining_wall_type`).
#[no_mangle]
pub unsafe extern "C" fn fc4_development_load_retaining_wall(p: *mut fc4_development, occ: u64, t: u32) -> bool {
    sim(p, false, |s| s.load_retaining_wall_properties(occ, t))
}

/// Slot 32 `GetDeveloper(devType)`: borrowed, 0 if absent.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_developer(p: *const fc4_development, dev_type: u32) -> u64 {
    sim(p, None, |s| s.developer(dev_type)).unwrap_or(0)
}

/// Slot 34 `GetDevelopers()`: `cb` once per entry in ascending type
/// (borrowed tokens); the entry count.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_developers(
    p: *const fc4_development,
    cb: fc4_type_token_callback,
    ctx: *mut c_void,
) -> u32 {
    // Without a copy of the map (the host calls this after every outermost
    // call, devperf): the callback only collects.
    sim(p, 0, |s| {
        let mut n = 0u32;
        s.for_each_developer(|t, d| {
            if let Some(f) = cb {
                f(ctx, t, d);
            }
            n += 1;
        });
        n
    })
}

/// Slot 41 `GetBuildingMap()`: the grid token (borrowed; 0 = none).
#[no_mangle]
pub unsafe extern "C" fn fc4_development_building_map(p: *const fc4_development) -> u64 {
    sim(p, 0, |s| grid_token(&s.building_map()))
}

/// Slot 44 `GetSpecialBuildings()`: `cb` once per occupant in list order
/// (borrowed); the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_special_buildings(
    p: *const fc4_development,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) -> u32 {
    // Without a copy of the list (the host calls this after every outermost
    // call, devperf): the callback only collects.
    sim(p, 0, |s| {
        let mut n = 0u32;
        s.for_each_special_building(|o| {
            if let Some(f) = cb {
                f(ctx, o);
            }
            n += 1;
        });
        n
    })
}

/// Slot 46 `IsSuitableForConstruction(profile, reason&)`: `*reason` written
/// only on failure (when not NULL).
#[no_mangle]
pub unsafe extern "C" fn fc4_development_is_suitable(
    p: *const fc4_development,
    profile: *const fc4_construction_profile,
    reason: *mut i32,
) -> bool {
    let Some(prof) = profile.as_ref().map(|p| p.to_core()) else {
        return false;
    };
    let mut r = reason.as_ref().copied().unwrap_or(0);
    let ok = sim(p, false, |s| s.is_suitable_for_construction(&prof, &mut r));
    if !ok {
        if let Some(o) = reason.as_mut() {
            *o = r;
        }
    }
    ok
}

/// Slot 47 `PlopBuilding(profile)`: the occupant (borrowed) or 0.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_plop_building(
    p: *mut fc4_development,
    profile: *const fc4_construction_profile,
) -> u64 {
    let Some(prof) = profile.as_ref().map(|p| p.to_core()) else {
        return 0;
    };
    sim(p, None, |s| s.plop_building(&prof)).unwrap_or(0)
}

/// Slot 48 `GetFreeConstructionResources()`.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_free_resources(p: *const fc4_development) -> i32 {
    sim(p, 0, DevelopmentSim::free_construction_resources)
}

/// Slot 49 `GetTotalConstructionResources()`.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_total_resources(p: *const fc4_development) -> i32 {
    sim(p, 0, DevelopmentSim::total_construction_resources)
}

/// Slot 50 `ClaimConstructionResources(n)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_claim(p: *mut fc4_development, amount: u16) -> bool {
    sim(p, false, |s| s.claim_construction_resources(amount))
}

/// Slot 51 `ReleaseConstructionResources(n)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_release_resources(p: *mut fc4_development, amount: u16) -> bool {
    sim(p, false, |s| s.release_construction_resources(amount))
}

/// Slot 52 `GetDayOfLastSuccessfulBuild(devType)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_last_successful_build(p: *const fc4_development, dev_type: u32) -> i32 {
    sim(p, 0, |s| s.day_of_last_successful_build(dev_type))
}

/// Slot 53 `GetStagePopulationMultiplier()`.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_stage_population_multiplier(p: *const fc4_development) -> f32 {
    sim(p, 1.0, DevelopmentSim::stage_population_multiplier)
}

/// Slots 54–58: `which` 0 ignore plumbing, 1 ignore power, 2 ignore caps, 3
/// debug growth, 4 coffee bar.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_cheat(p: *const fc4_development, which: u32) -> bool {
    sim(p, false, |s| match which {
        0 => s.cheat_ignore_plumbing(),
        1 => s.cheat_ignore_power(),
        2 => s.cheat_ignore_caps(),
        3 => s.cheat_debug_growth(),
        4 => s.cheat_coffee_bar(),
        _ => false,
    })
}

// ---- diagnostics and persistence -----------------------------------------------------------------------

/// The highest stage per developer type (ascending): `cb(type, stage)`; the
/// entry count.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_highest_stages(
    p: *const fc4_development,
    cb: fc4_type_value_callback,
    ctx: *mut c_void,
) -> u32 {
    let v = with_handle(p, Default::default(), |h| h.sim.snapshot().highest_stage);
    if let Some(f) = cb {
        for (&t, &s) in &v {
            f(ctx, t, u32::from(s));
        }
    }
    v.len() as u32
}

/// The decline plan (§4.3) in plan order: `cb` once per entry (borrowed
/// developer tokens); the entry count.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_decline_plan(
    p: *const fc4_development,
    cb: fc4_plan_entry_callback,
    ctx: *mut c_void,
) -> u32 {
    let v = with_handle(p, Vec::new(), |h| h.sim.snapshot().decline_plan);
    if let Some(f) = cb {
        for e in &v {
            let pe = fc4_plan_entry {
                day: e.day,
                x: e.x,
                z: e.z,
                stage: u32::from(e.stage),
                developer: e.developer,
            };
            f(ctx, &pe);
        }
    }
    v.len() as u32
}

/// Diagnostics: the state, the construction resources and the cumulative
/// tract developer counters.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_get_state(p: *const fc4_development, out: *mut fc4_development_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_handle(p, false, |h| {
        let s = &h.sim;
        let d = s.snapshot();
        let process = s.process();
        let marks = process.marks();
        let node = s.construction_node();
        let c = *h.host.counters.borrow();
        let cheats = u8::from(d.cheat_ignore_plumbing)
            | u8::from(d.cheat_ignore_power) << 1
            | u8::from(d.cheat_ignore_caps) << 2
            | u8::from(d.cheat_debug_growth) << 3
            | u8::from(d.cheat_coffee_bar) << 4;
        *o = fc4_development_state {
            struct_size: std::mem::size_of::<fc4_development_state>() as u32,
            initialized: u8::from(d.initialized),
            loaded: u8::from(d.loaded_from_save),
            state: d.state.min(255) as u8,
            cheats,
            developers: d.developers.len() as u32,
            special_buildings: d.special_buildings.len() as u32,
            highest_stages: d.highest_stage.len() as u32,
            decline_plan: d.decline_plan.len() as u32,
            marked_types: marks.len() as u32,
            marked_cells: marks.values().map(BTreeSet::len).sum::<usize>() as u32,
            building_types: process.catalogue().building_count() as u32,
            grow_budget: s.grow_attempt_budget(),
            total_resources: s.total_construction_resources(),
            free_resources: s.free_construction_resources(),
            supply: node.as_ref().map_or(0.0, |n| n.supply()),
            demand: node.as_ref().map_or(0.0, |n| n.demand()),
            grow_calls: c.grow_calls,
            built: c.built,
            decline_calls: c.decline_calls,
            declined: c.declined,
            built_by_wealth: c.built_by_wealth,
            declined_by_wealth: c.declined_by_wealth,
            built_by_stage: c.built_by_stage,
            reentrant_calls: h.reentrant.get(),
            cell_count_x: d.cell_counts.0,
            cell_count_z: d.cell_counts.1,
            reserved: 0,
            rng_steps: process.steps(),
            building_map: grid_token(&s.building_map()),
            construction_node: node_token(&node),
        };
        true
    })
}

/// `Write` (§6): the v4 record. References go out as `fc4_ref`s with the
/// token and the class ID.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_write(p: *const fc4_development, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    sim(p, false, |s| s.write(&mut COut(o)))
}

/// `Read` (§6). Runs before `Init`: nothing is sized from the city here.
#[no_mangle]
pub unsafe extern "C" fn fc4_development_read(p: *mut fc4_development, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_handle(p, false, |h| {
        let host = h.host.clone();
        h.call(|s| {
            s.read(&mut DIn {
                inner: CIn {
                    stream: i,
                    decode: no_core_objects,
                },
                host: &host,
            })
        })
    })
}
