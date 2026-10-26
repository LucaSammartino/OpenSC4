// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The lot developer's C ABI (fc4.h "lot developer",
//! docs/specs/lot-developer.md).
//!
//! The host implements [`fc4_lotdev_host`]: one callback per
//! `LotDeveloperHost` method of the core, in the trait's order. The plugin's
//! `cISC4LotDeveloper` (with its message-target and serializable
//! sub-objects) forwards its slots to the `fc4_lotdev_*` functions. The
//! §17 geometry slots (16, 18–21, 39, 40, 42, 43, 48–61) are here; slots
//! 38, 41, 45 and 62–64 (and the retaining-wall helpers) stay the host's.
//!
//! The class-wide statics of §10.1 (the file-local `cRZRandom` and the
//! `rand()` state of R15) are one [`fc4_lotdev_statics`] the host creates
//! once per process and hands to every lot developer it creates.
//!
//! Tokens are opaque host identities (fc4.h): lots (`cISC4Lot*`), lot
//! configurations, occupants (`cISC4Occupant*`), buildings
//! (`cISC4BuildingOccupant*`), business units (`cISC4BusinessUnit*`),
//! growth developers and the safe lot iterator. 0 is the null object. Every
//! token the core receives is borrowed, except: the night deque holds one
//! reference per building (`building_add_ref` / `building_release`, and the
//! references a save read hands over), and a unit made by `unit_create` is
//! the host's to keep alive until the call returns (the members hold it).
//!
//! Re-entrancy: the core is one `RefCell`. A message delivered while a call
//! is running is queued and handled when the outermost call returns (the
//! host keeps its tokens alive until then). Any other call that arrives
//! while the core is busy is **refused**: it returns the slot's neutral value
//! and is counted (`fc4_lotdev_state::refused_calls`) *[choice]*. The
//! terrain, cost and §17 geometry slots (15–22, 25–32, 39–40, 42–43, 48–61)
//! never need the core: they run on a snapshot ([`TerrainView`], refreshed
//! when each outermost core call returns) and the shared statics, so they
//! answer even in the middle of a core call (the BDS asks for a lot slope
//! while `StartConstruction` places a building).
//!
//! Threads: the core is single-threaded. The plugin serialises every call
//! on one lot developer with its lock (fc4.h).

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::lot_developer::{
    answers_iid, lot_prop, lot_state, msg, BuildingProfile, BurntBuilding, CellRect, CfgObject, Cheat, ConnectionNode,
    LotDevMessage, LotDeveloper, LotDeveloperHost, LotDeveloperStatics, SharedStatics, TerrainView, TextureRecord,
    Tuning, Vec3, IID_LOT_DEVELOPER, IID_MESSAGE_TARGET2, IID_SERIALIZABLE, IID_UNKNOWN,
};
use fc4_core::property::PropertyHolder;
use fc4_core::rng::FixedSeeds;
use fc4_core::stream::{InStream, RefValue, SaveObject};

use crate::budget::fc4_token_callback;
use crate::guard;
use crate::hostgrid::{collect_tokens, fc4_cell_rect, take_props};
use crate::props::fc4_props;
use crate::stream::{fc4_istream, fc4_ostream, fc4_ref, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

/// A sink for a list of `uint32` values (fc4.h `fc4_u32s_sink`).
pub type fc4_u32s_sink = Option<unsafe extern "C" fn(ctx: *mut c_void, values: *const u32, n: usize)>;

/// One lot configuration object (fc4.h `fc4_lotdev_cfg_object`): the fields
/// of a 0x34-byte `cSC4LotConfigurationObject` the lot developer reads. The
/// field offsets are the record's (§17.1); the layout is `#[repr(C)]` so the
/// host fills it in place.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_lotdev_cfg_object {
    /// `+0x00`: 1 prop, 2 base texture, 4 flora, 7 water, … .
    pub object_type: u32,
    /// `+0x01`: the flag byte.
    pub flags: u8,
    /// `+0x02`: the orientation.
    pub orientation: u8,
    /// `+0x03`: the mirror flag / object byte 3.
    pub byte3: u8,
    /// `+0x04`, `+0x08`, `+0x0C`: the lot-space offsets x, y, z.
    pub offset: [f32; 3],
    /// `+0x10`, `+0x14`: the footprint's max x, max z.
    pub footprint_max: [f32; 2],
    /// `+0x18`, `+0x1C`: the footprint's min x, min z.
    pub footprint_min: [f32; 2],
    /// `+0x20`: the foundation type.
    pub foundation: u32,
    /// `+0x24`: the lot tag.
    pub tag: u32,
    /// `+0x28`: the ID count.
    pub n_ids: u32,
    /// `+0x2C`: the object's ID list (`n_ids` values; families not expanded).
    pub ids: *const u32,
}

/// One cfg object (`cfg_objects`), in array order.
pub type fc4_lotdev_cfg_object_callback =
    Option<unsafe extern "C" fn(ctx: *mut c_void, object: *const fc4_lotdev_cfg_object)>;

/// One base-texture record (fc4.h `fc4_lotdev_texture`, §6.6, §17.7): the
/// fields of the 14-byte record of lot-records.md §6, the cell as int32.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_lotdev_texture {
    pub texture: u32,
    pub x: i32,
    pub z: i32,
    /// The orientation (normal textures) or rotation (empty-lot textures).
    pub orientation: u8,
    /// Object byte 3 (normal textures) or the flip (empty-lot textures).
    pub byte3: u8,
    /// `0xFF` x 5 for normal textures; the zone's colour (three bytes, from
    /// the top), the display alpha and 3 for empty-lot textures.
    pub extra: [u8; 5],
    /// The frame byte (`0xFF`).
    pub frame: u8,
}

/// One network connection node of `AddNetworkConnectionNodes` (fc4.h
/// `fc4_lotdev_conn_node`, §6.5, LD17): the arguments of network manager
/// slot 20 `CreateConnectionNode(&n, type, &cell, b, c, d, orientation)`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_lotdev_conn_node {
    pub node_type: u32,
    pub x: i32,
    pub z: i32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
    pub orientation: u8,
    pub reserved: [u8; 3],
}

/// The burnt-building prop of `ExplodeLot` (fc4.h `fc4_lotdev_burnt`, §7.2).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_lotdev_burnt {
    pub occupant: u64,
    pub min: [f32; 3],
    pub max: [f32; 3],
}

/// The building profile fields the lot developer reads (fc4.h
/// `fc4_lotdev_profile`): purpose, wealth and the capacity for
/// `GetDeveloperType(purpose, wealth)`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_lotdev_profile {
    pub purpose: u32,
    pub wealth: u32,
    pub capacity: u16,
    pub reserved: u16,
}

/// A message for `fc4_lotdev_do_message` (fc4.h `fc4_lotdev_message`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_lotdev_message {
    pub msg_type: u32,
    pub data1: u32,
    pub data2: u32,
    pub data3: u32,
    pub data4: u32,
    pub reserved: u32,
    /// `GetVoid1()` as a `cISC4Occupant` (occupant messages), 0 otherwise.
    pub occupant: u64,
    /// `GetIGZUnknown()` as a `cISC4Lot` (state / condition messages).
    pub lot: u64,
}

/// The tuning block (fc4.h `fc4_lotdev_tuning`, §3.5).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_lotdev_tuning {
    pub stay_new_time: u16,
    pub attempt_lot_levelling: u8,
    pub attempt_internal_lot_levelling: u8,
    pub thresholds: [f32; 3],
    pub max_cell_altitude_delta_for_flatness: f32,
    pub max_slope_for_flatness: f32,
    pub default_lot_foundation_type: u32,
    pub default_building_foundation_type: u32,
    pub auto_level_internal_altitude_delta: f32,
    pub max_area_slope_for_levelling: f32,
    /// The cost curve's points (x, y pairs; at most 16 pairs are copied).
    pub cost_curve_points: u32,
    pub cost_curve: [f32; 32],
}

/// Diagnostics (fc4.h `fc4_lotdev_state`). The counters are cumulative
/// since the handle was created.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_lotdev_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub night_mode: u8,
    pub lit_flag: u8,
    pub force_empty: u8,
    pub daily_state: u32,
    /// Lots visited in the current pass (§4.1).
    pub visited: i32,
    pub night_len: u32,
    pub night_cursor: u32,
    pub night_acc: u32,
    pub night_batch: u32,
    pub no_zot_until_day: u32,
    pub can_check_commute: u8,
    pub zots_allowed: u8,
    pub net_changed: u8,
    pub unit_changed: u8,
    pub verts_x: i32,
    pub extent_x: f32,
    pub extent_z: f32,
    pub cell_count_x: i32,
    pub cell_count_z: i32,
    pub daily_work_budget: u32,
    // ---- counters ----
    /// Lots handed out by the safe iterator during a daily pass.
    pub lots_visited: u32,
    /// `SetState(4, …)` on a lot that was not abandoned.
    pub abandoned: u32,
    /// `SetState(3, …)` on an abandoned lot.
    pub restored: u32,
    /// `SetCondition` with a value different from the lot's.
    pub condition_changes: u32,
    /// Zot property writes that change the value.
    pub zot_changes: u32,
    /// Construction occupants started (§6.3).
    pub constructions_started: u32,
    /// `0xE8BDA2E7` messages from state 2 to 3.
    pub constructions_finished: u32,
    /// `StartDemolishLot` and `ExplodeLot` calls.
    pub demolitions: u32,
    /// Animation ticks that lit a batch (§8.3).
    pub night_batches: u32,
    /// Calls refused because the core was busy (re-entrant).
    pub refused_calls: u32,
    /// Messages queued during a call.
    pub queued_messages: u32,
    pub reserved: u32,
}

/// Everything the lot developer asks of the game (fc4.h `fc4_lotdev_host`;
/// one callback per `LotDeveloperHost` method, in the trait's order).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_lotdev_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    // lifecycle
    pub acquire_services: Option<unsafe extern "C" fn(U) -> bool>,
    pub release_services: Option<unsafe extern "C" fn(U)>,
    /// A NEW fc4_props (the core destroys it), NULL if the exemplar is missing.
    pub tuning: Option<unsafe extern "C" fn(U) -> *mut fc4_props>,
    pub has_simulator: Option<unsafe extern "C" fn(U) -> bool>,
    pub add_agent: Option<unsafe extern "C" fn(U, u32, u32)>,
    pub remove_agent: Option<unsafe extern "C" fn(U)>,
    pub acquire_message_server: Option<unsafe extern "C" fn(U) -> bool>,
    pub release_message_server: Option<unsafe extern "C" fn(U)>,
    pub subscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub register_animation_tick: Option<unsafe extern "C" fn(U)>,
    pub unregister_animation_tick: Option<unsafe extern "C" fn(U)>,
    /// `(type, data1, lot)`: a queued standard message, data2–4 = 0.
    pub post_message: Option<unsafe extern "C" fn(U, u32, u32, u64)>,
    // city, simulator, terrain
    /// Cell counts into `out[2]`.
    pub city_cell_counts: Option<unsafe extern "C" fn(U, *mut i32)>,
    /// `CellWidthX`, `CellWidthZ` into `out[2]`.
    pub city_cell_widths: Option<unsafe extern "C" fn(U, *mut f32)>,
    /// `PositionToCell(x, z)` into `out[2]`; returns its AL.
    pub position_to_cell: Option<unsafe extern "C" fn(U, f32, f32, *mut i32)>,
    /// City slot 96 `CellCornerToPosition(x, z)`: position into `out[2]`.
    pub cell_corner_to_position: Option<unsafe extern "C" fn(U, i32, i32, *mut f32)>,
    /// City slot 97 `CellCenterToPosition(x, z)`: position into `out[2]`.
    pub cell_center_to_position: Option<unsafe extern "C" fn(U, i32, i32, *mut f32)>,
    pub sim_date: Option<unsafe extern "C" fn(U) -> u32>,
    pub terrain_cell_counts: Option<unsafe extern "C" fn(U, *mut i32)>,
    pub terrain_cell_width: Option<unsafe extern "C" fn(U) -> f32>,
    pub terrain_altitude: Option<unsafe extern "C" fn(U, i32) -> f32>,
    pub terrain_water_level: Option<unsafe extern "C" fn(U, i32) -> f32>,
    /// Terrain slot 13 `GetAltitude(x, z)` at a position in metres.
    pub terrain_altitude_at: Option<unsafe extern "C" fn(U, f32, f32) -> f32>,
    /// Terrain slot 30 `GetWaterLevel(x, z)` at a position in metres.
    pub terrain_water_level_at: Option<unsafe extern "C" fn(U, f32, f32) -> f32>,
    /// Terrain slot 37 `IsWaterVertex(x, z)`.
    pub terrain_is_water_vertex: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    /// Terrain slot 34 `CellHasWater(x, z)`.
    pub terrain_cell_has_water: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    /// Zone manager slot 15 `GetZoneDragColor(zone)`.
    pub zone_drag_color: Option<unsafe extern "C" fn(U, i32) -> u32>,
    /// Zone manager slot 16 `GetZoneDisplayAlpha()`.
    pub zone_display_alpha: Option<unsafe extern "C" fn(U) -> u8>,
    pub view_rotation: Option<unsafe extern "C" fn(U) -> i32>,
    pub visual_time_mode: Option<unsafe extern "C" fn(U) -> u32>,
    pub resource_exists: Option<unsafe extern "C" fn(U, u32, u32, u32) -> bool>,
    // lot manager
    pub lot_count: Option<unsafe extern "C" fn(U) -> i32>,
    pub safe_lot_iterator: Option<unsafe extern "C" fn(U) -> u64>,
    pub safe_lot_iterator_rewind: Option<unsafe extern "C" fn(U, u64)>,
    /// The current lot (borrowed), advancing; 0 at the end.
    pub safe_lot_iterator_next: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub safe_lot_iterator_free: Option<unsafe extern "C" fn(U, u64)>,
    pub get_lot: Option<unsafe extern "C" fn(U, i32, i32, bool) -> u64>,
    pub lots_in_cell_rect: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, bool, fc4_token_callback, *mut c_void)>,
    /// `GetLotsSurroundingLot(lot, list, flags)` on a fresh list: the lots
    /// in list order (the core drops those it already has).
    pub lots_surrounding_lot: Option<unsafe extern "C" fn(U, u64, u32, fc4_token_callback, *mut c_void)>,
    pub lots_surrounding_rect:
        Option<unsafe extern "C" fn(U, *const fc4_cell_rect, u32, fc4_token_callback, *mut c_void)>,
    pub delete_lot: Option<unsafe extern "C" fn(U, u64) -> bool>,
    // lots
    pub lot_is_initialized: Option<unsafe extern "C" fn(U, u64) -> bool>,
    /// `(x, z)` into `out[2]`.
    pub lot_location: Option<unsafe extern "C" fn(U, u64, *mut i32)>,
    pub lot_size: Option<unsafe extern "C" fn(U, u64, *mut u32)>,
    pub lot_bounding_rect: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    pub lot_center: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    pub lot_zone_type: Option<unsafe extern "C" fn(U, u64) -> i32>,
    /// Lot slots 16 / 17 `GetAltitude` / `SetAltitude`.
    pub lot_altitude: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub lot_set_altitude: Option<unsafe extern "C" fn(U, u64, f32)>,
    /// Lot slot 23 `GetFacing` (0-3).
    pub lot_facing: Option<unsafe extern "C" fn(U, u64) -> i32>,
    pub lot_flags: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub lot_set_flags: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub lot_set_vertices_locked: Option<unsafe extern "C" fn(U, u64, bool)>,
    pub lot_update_slope: Option<unsafe extern "C" fn(U, u64)>,
    pub lot_state: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub lot_set_state: Option<unsafe extern "C" fn(U, u64, u32, u32, u32) -> bool>,
    pub lot_condition: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub lot_set_condition: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub lot_is_livable: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub lot_days_since_state_change: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub lot_building_type: Option<unsafe extern "C" fn(U, u64, bool) -> u32>,
    pub lot_set_building_type: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub lot_building: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub lot_configuration: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub lot_set_configuration: Option<unsafe extern "C" fn(U, u64, u64)>,
    pub lot_occupant_wealth: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub lot_occupancy: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub lot_needs_power: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub lot_has_power: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub lot_set_has_power: Option<unsafe extern "C" fn(U, u64, bool)>,
    pub lot_has_job_access: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub lot_has_road_access: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub lot_has_organic_road_access: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub lot_update_road_access: Option<unsafe extern "C" fn(U, u64)>,
    pub lot_travel_jobs: Option<unsafe extern "C" fn(U, u64, u32) -> f32>,
    pub lot_travel_designate: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub lot_set_travel_designate: Option<unsafe extern "C" fn(U, u64, u64)>,
    pub lot_unit: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub lot_reset_travel_population: Option<unsafe extern "C" fn(U, u64)>,
    pub lot_add_travel_population: Option<unsafe extern "C" fn(U, u64, u64)>,
    pub lot_is_historical: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub lot_set_historical: Option<unsafe extern "C" fn(U, u64, bool)>,
    pub lot_available_capacity: Option<unsafe extern "C" fn(U, u64, u32, bool) -> u16>,
    pub lot_set_population: Option<unsafe extern "C" fn(U, u64, u32, u16) -> bool>,
    /// A uint32 lot property into `*out`; false when absent.
    pub lot_property: Option<unsafe extern "C" fn(U, u64, u32, *mut u32) -> bool>,
    pub lot_set_property: Option<unsafe extern "C" fn(U, u64, u32, u32)>,
    // lot configurations and their manager
    pub cfg_id: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub cfg_size: Option<unsafe extern "C" fn(U, u64, *mut u32)>,
    pub cfg_max_slope_before_foundation: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub cfg_growth_stage: Option<unsafe extern "C" fn(U, u64) -> u8>,
    pub cfg_is_compatible_with_water: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub cfg_objects: Option<unsafe extern "C" fn(U, u64, fc4_lotdev_cfg_object_callback, *mut c_void)>,
    /// Slot 24 `GetPrimaryBuildingConfigurationObject`: the object to `cb`
    /// (once); false without one.
    pub cfg_primary_building_object:
        Option<unsafe extern "C" fn(U, u64, fc4_lotdev_cfg_object_callback, *mut c_void) -> bool>,
    /// Slot 26 `GetPrimaryFoundationConfigurationObject`, likewise.
    pub cfg_primary_foundation_object:
        Option<unsafe extern "C" fn(U, u64, fc4_lotdev_cfg_object_callback, *mut c_void) -> bool>,
    pub cfg_has_objects_of_type: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub get_lot_configuration: Option<unsafe extern "C" fn(U, u32) -> u64>,
    /// Manager slot 27 for object `index`: the IDs, families expanded.
    pub object_id_array: Option<unsafe extern "C" fn(U, u64, u32, fc4_u32s_sink, *mut c_void)>,
    // slots and helpers §17 does not cover
    /// Slot 38 `GetFirstBuilding(lot)` (borrowed).
    pub first_building: Option<unsafe extern "C" fn(U, u64) -> u64>,
    /// Slot 62 `UpdateLotRetainingWalls(lot)`.
    pub update_lot_retaining_walls: Option<unsafe extern "C" fn(U, u64) -> bool>,
    /// Slot 45 `UpdateLotBaseTextureOccupant(lot, hadFoundation)`.
    pub update_lot_base_texture_occupant: Option<unsafe extern "C" fn(U, u64, bool) -> bool>,
    /// Slot 41 `GetLotBaseTextureOccupant(lot, &occ, create)`: the occupant
    /// (borrowed), 0 when the slot returns false.
    pub base_texture_occupant: Option<unsafe extern "C" fn(U, u64, bool) -> u64>,
    /// Base-texture slot 8 `SetTextureSpecification(records, n, 0)`.
    pub base_texture_set: Option<unsafe extern "C" fn(U, u64, *const fc4_lotdev_texture, usize)>,
    /// `AddRetainingWallOccupant(lot, flag)` (`0x005E3070`).
    pub add_retaining_wall_occupant: Option<unsafe extern "C" fn(U, u64, u32)>,
    /// `UpdateSurroundingRetainingWalls(lot)` (`0x005E3960`).
    pub update_surrounding_retaining_walls: Option<unsafe extern "C" fn(U, u64)>,
    /// `AddNetworkConnectionNodes` steps 2-3 (§6.5, LD17): network manager
    /// slot 20 for each of the `n` nodes, every result in one list, then
    /// slot 21 `AutoconnectLotNodes(lot, &list)` (also with `n` = 0).
    pub add_network_connection_nodes: Option<unsafe extern "C" fn(U, u64, *const fc4_lotdev_conn_node, usize)>,
    /// `RemoveNetworkConnectionNodes(lot)` (§6.5, LD17).
    pub remove_network_connection_nodes: Option<unsafe extern "C" fn(U, u64)>,
    pub start_construction_occupant: Option<unsafe extern "C" fn(U, u64)>,
    // buildings and other occupants
    /// QueryInterface for `cISC4BuildingOccupant`: borrowed, 0 = not one.
    pub occupant_as_building: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub building_add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub building_release: Option<unsafe extern "C" fn(U, u64)>,
    pub building_set_lit: Option<unsafe extern "C" fn(U, u64, bool)>,
    /// Occupant slot 7 `GetType`.
    pub occupant_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    /// Occupant slot 8 `GetPosition` into `out[3]`.
    pub occupant_position: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    /// Min then max corner into `out[6]`.
    pub occupant_bounding_box: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    pub occupant_city_cells: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    pub occupant_in_group: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub occupant_post_message: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub occupant_set_removal_flags: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub occupant_set_lot_tag: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub occupant_property: Option<unsafe extern "C" fn(U, u64, u32, *mut u32) -> bool>,
    pub occupant_set_property: Option<unsafe extern "C" fn(U, u64, u32, u32)>,
    pub occupant_init: Option<unsafe extern "C" fn(U, u64)>,
    pub occupant_set_flag: Option<unsafe extern "C" fn(U, u64, u32)>,
    /// Occupant slot 32 `IsFlagSet(flag)`.
    pub occupant_is_flag_set: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    /// Occupant slot 4 `Shutdown`.
    pub occupant_shutdown: Option<unsafe extern "C" fn(U, u64)>,
    pub occupant_set_orientation: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub occupant_set_position: Option<unsafe extern "C" fn(U, u64, *const f32)>,
    pub occupant_insert: Option<unsafe extern "C" fn(U, u64)>,
    pub occupant_remove: Option<unsafe extern "C" fn(U, u64, u32, u32)>,
    pub occupant_changed: Option<unsafe extern "C" fn(U, u64)>,
    pub occupants_in_bbox: Option<unsafe extern "C" fn(U, *const f32, *const f32, fc4_token_callback, *mut c_void)>,
    pub occupants_in_cells: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, u32, fc4_token_callback, *mut c_void)>,
    /// Occupant manager slot 37: the occupants of a type whose box meets
    /// the closed box `min[3]`..`max[3]`, in list order (borrowed).
    pub occupants_in_box: Option<unsafe extern "C" fn(U, *const f32, *const f32, u32, fc4_token_callback, *mut c_void)>,
    /// Building slot 14 `GetOrientation`.
    pub building_orientation: Option<unsafe extern "C" fn(U, u64) -> i32>,
    // foundations (cISC4FoundationOccupant, §17.5)
    /// `QueryInterface(0xC8FCE101)` then slot 9 `GetKind()` into `*out`;
    /// false when the occupant is not a foundation.
    pub foundation_kind: Option<unsafe extern "C" fn(U, u64, *mut i32) -> bool>,
    /// Slot 4 `GetFoundationType()`.
    pub foundation_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    /// BDS slot 23 `AddNewFoundationOccupant(type, kind, pos, dx, dz,
    /// orient, bottom)`: `pos[3]` in/out; AL.
    pub add_new_foundation_occupant: Option<unsafe extern "C" fn(U, u32, i32, *mut f32, f32, f32, i32, f32) -> bool>,
    // prop manager
    pub prop_create: Option<unsafe extern "C" fn(U, u32) -> u64>,
    pub lot_props: Option<unsafe extern "C" fn(U, u64, fc4_token_callback, *mut c_void)>,
    pub prop_add_to_lot: Option<unsafe extern "C" fn(U, u64, u64) -> bool>,
    pub prop_add: Option<unsafe extern "C" fn(U, u32, *const f32, i32) -> u64>,
    pub remove_lot_props: Option<unsafe extern "C" fn(U, u64)>,
    pub prop_remove: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub remove_building_props: Option<unsafe extern "C" fn(U, u64)>,
    pub make_burnt_building: Option<unsafe extern "C" fn(U, u64, *mut fc4_lotdev_burnt) -> bool>,
    pub add_rubble_decal: Option<unsafe extern "C" fn(U, u64, *const fc4_lotdev_burnt, u32, f32)>,
    // BDS
    pub building_profile: Option<unsafe extern "C" fn(U, u32, *mut fc4_lotdev_profile)>,
    pub developer_type: Option<unsafe extern "C" fn(U, u32, u32) -> u32>,
    pub developer_of_type: Option<unsafe extern "C" fn(U, u32) -> u64>,
    pub wealth_of_type: Option<unsafe extern "C" fn(U, u32) -> u32>,
    pub developer_for: Option<unsafe extern "C" fn(U, u32, u32) -> u64>,
    pub is_building_type: Option<unsafe extern "C" fn(U, u32) -> bool>,
    pub building_types_by_family: Option<unsafe extern "C" fn(U, u32, fc4_u32s_sink, *mut c_void)>,
    pub add_new_building_occupant: Option<unsafe extern "C" fn(U, u32, *const f32, i32) -> u64>,
    pub claim_construction_resources: Option<unsafe extern "C" fn(U, u16) -> bool>,
    /// 0 ignore plumbing (slot 54), 1 ignore power (55), 2 coffee bar (58).
    pub cheat: Option<unsafe extern "C" fn(U, u32) -> bool>,
    // growth developers
    pub dev_purpose: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub dev_desirability: Option<unsafe extern "C" fn(U, u64, i32, i32) -> i16>,
    pub dev_water_supply_cap: Option<unsafe extern "C" fn(U, u64) -> u8>,
    // power, plumbing, traffic, demand
    pub is_powered: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    pub is_cell_watered: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    pub trip_length: Option<unsafe extern "C" fn(U, i32, i32) -> u8>,
    /// The jobs demand into `*out`; false without the node.
    pub jobs_demand: Option<unsafe extern "C" fn(U, *mut f32) -> bool>,
    // flora
    pub flora_type_valid: Option<unsafe extern "C" fn(U, u32) -> bool>,
    pub flora_types_by_family: Option<unsafe extern "C" fn(U, u32, fc4_u32s_sink, *mut c_void) -> bool>,
    pub flora_add: Option<unsafe extern "C" fn(U, u32, *const f32) -> u64>,
    pub flora_demolish_in_cell_area: Option<unsafe extern "C" fn(U, *const fc4_cell_rect)>,
    // demolition
    pub demolish_occupant: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub demolish_lot: Option<unsafe extern "C" fn(U, u64) -> bool>,
    /// Slot 9 `ModifyTerrainHeight(1, perVertex, x0, z0, x1, z1, h, exempt,
    /// 5, null)` (§17.3): EAX, the count.
    pub modify_terrain_height: Option<unsafe extern "C" fn(U, bool, i32, i32, i32, i32, f32, u64) -> i32>,
    // business units
    pub unit_members: Option<unsafe extern "C" fn(U, u64, fc4_token_callback, *mut c_void)>,
    pub unit_create: Option<unsafe extern "C" fn(U) -> u64>,
    pub unit_add: Option<unsafe extern "C" fn(U, u64, u64) -> bool>,
    pub unit_remove: Option<unsafe extern "C" fn(U, u64, u64) -> bool>,
    // UpdateLotObjectAlitudes (§6.5, LD17; ABI 60)
    /// Lot slot 49 `GetLotOccupants`: the occupants in list order
    /// (references kept for the call).
    pub lot_occupants: Option<unsafe extern "C" fn(U, u64, fc4_token_callback, *mut c_void)>,
    /// Lot slot 11 `GetLotConfigurationObject(occ)`: the object through the
    /// callback; false without one.
    pub lot_configuration_object:
        Option<unsafe extern "C" fn(U, u64, u64, fc4_lotdev_cfg_object_callback, *mut c_void) -> bool>,
    /// Occupant manager slot 24 `(occ, 1)`: the occupant moved.
    pub occupant_moved: Option<unsafe extern "C" fn(U, u64)>,
}

/// The number of callbacks in [`fc4_lotdev_host`].
pub const LOTDEV_HOST_CALLBACKS: usize = 178;

const _: () = {
    let ptr = std::mem::size_of::<usize>();
    assert!(std::mem::size_of::<fc4_lotdev_cfg_object>() == 48 + ptr);
    assert!(std::mem::size_of::<fc4_lotdev_texture>() == 20);
    assert!(std::mem::size_of::<fc4_lotdev_burnt>() == 32);
    assert!(std::mem::size_of::<fc4_lotdev_conn_node>() == 28);
    assert!(std::mem::size_of::<fc4_lotdev_profile>() == 12);
    assert!(std::mem::size_of::<fc4_lotdev_message>() == 40);
    assert!(std::mem::size_of::<fc4_lotdev_tuning>() == 172);
    assert!(std::mem::size_of::<fc4_lotdev_state>() == 112);
    assert!(std::mem::size_of::<fc4_lotdev_host>() == if ptr == 4 { 8 } else { 16 } + LOTDEV_HOST_CALLBACKS * ptr);
};

// ---- host adapter -----------------------------------------------------------------------------------

/// The counters of [`fc4_lotdev_state`] the host adapter keeps.
#[derive(Default)]
struct Counters {
    in_daily: Cell<bool>,
    lots_visited: Cell<u32>,
    abandoned: Cell<u32>,
    restored: Cell<u32>,
    condition_changes: Cell<u32>,
    zot_changes: Cell<u32>,
    constructions_started: Cell<u32>,
}

fn bump(c: &Cell<u32>) {
    c.set(c.get().wrapping_add(1));
}

struct CHost {
    t: fc4_lotdev_host,
    counters: Counters,
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

unsafe extern "C" fn push_u32s(ctx: *mut c_void, values: *const u32, n: usize) {
    if let Some(v) = (ctx as *mut Vec<u32>).as_mut() {
        if !values.is_null() && n > 0 {
            v.extend_from_slice(std::slice::from_raw_parts(values, n));
        }
    }
}

/// Calls a host list callback with a sink that collects `uint32` values.
fn collect_u32s<R>(f: impl FnOnce(fc4_u32s_sink, *mut c_void) -> R) -> (R, Vec<u32>) {
    let mut v: Vec<u32> = Vec::new();
    let r = f(Some(push_u32s), &mut v as *mut Vec<u32> as *mut c_void);
    (r, v)
}

unsafe extern "C" fn push_cfg_object(ctx: *mut c_void, o: *const fc4_lotdev_cfg_object) {
    let (Some(v), Some(o)) = ((ctx as *mut Vec<CfgObject>).as_mut(), o.as_ref()) else {
        return;
    };
    let ids = if o.ids.is_null() || o.n_ids == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(o.ids, o.n_ids as usize).to_vec()
    };
    v.push(CfgObject {
        object_type: o.object_type,
        flags: o.flags,
        orientation: o.orientation,
        byte3: o.byte3,
        offset: o.offset,
        footprint_max: o.footprint_max,
        footprint_min: o.footprint_min,
        foundation: o.foundation,
        tag: o.tag,
        ids,
    });
}

fn rect_arg(r: CellRect) -> fc4_cell_rect {
    fc4_cell_rect::from(r)
}

impl CHost {
    fn rect_out(&self, f: impl FnOnce(*mut fc4_cell_rect)) -> CellRect {
        let mut r = fc4_cell_rect::default();
        f(&mut r);
        CellRect::from(r)
    }
}

impl LotDeveloperHost for CHost {
    fn invocation_precision(&self) -> fc4_core::x87::Precision {
        crate::invocation::precision()
    }

    // ---- lifecycle ----
    fn acquire_services(&self) -> bool {
        cb!(self, acquire_services, false)
    }
    fn release_services(&self) {
        cb!(self, release_services, ())
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, tuning, std::ptr::null_mut());
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }
    fn has_simulator(&self) -> bool {
        cb!(self, has_simulator, false)
    }
    fn add_agent(&self, level: u32, message: u32) {
        cb!(self, add_agent, (), level, message)
    }
    fn remove_agent(&self) {
        cb!(self, remove_agent, ())
    }
    fn acquire_message_server(&self) -> bool {
        cb!(self, acquire_message_server, false)
    }
    fn release_message_server(&self) {
        cb!(self, release_message_server, ())
    }
    fn subscribe(&self, message: u32) {
        cb!(self, subscribe, (), message)
    }
    fn unsubscribe(&self, message: u32) {
        cb!(self, unsubscribe, (), message)
    }
    fn register_animation_tick(&self) {
        cb!(self, register_animation_tick, ())
    }
    fn unregister_animation_tick(&self) {
        cb!(self, unregister_animation_tick, ())
    }
    fn post_message(&self, msg_type: u32, data1: u32, lot: Option<u64>) {
        cb!(self, post_message, (), msg_type, data1, lot.unwrap_or(0))
    }

    // ---- city, simulator, terrain ----
    fn city_cell_counts(&self) -> (i32, i32) {
        let mut v = [0i32; 2];
        cb!(self, city_cell_counts, (), v.as_mut_ptr());
        (v[0], v[1])
    }
    fn city_cell_widths(&self) -> (f32, f32) {
        let mut v = [16.0f32; 2];
        cb!(self, city_cell_widths, (), v.as_mut_ptr());
        (v[0], v[1])
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        let mut v = [0i32; 2];
        cb!(self, position_to_cell, (), x, z, v.as_mut_ptr());
        (v[0], v[1])
    }
    fn cell_corner_to_position(&self, x: i32, z: i32) -> (f32, f32) {
        let mut v = [0.0f32; 2];
        cb!(self, cell_corner_to_position, (), x, z, v.as_mut_ptr());
        (v[0], v[1])
    }
    fn cell_center_to_position(&self, x: i32, z: i32) -> (f32, f32) {
        let mut v = [0.0f32; 2];
        cb!(self, cell_center_to_position, (), x, z, v.as_mut_ptr());
        (v[0], v[1])
    }
    fn sim_date(&self) -> u32 {
        cb!(self, sim_date, 0)
    }
    fn terrain_cell_counts(&self) -> (i32, i32) {
        let mut v = [0i32; 2];
        cb!(self, terrain_cell_counts, (), v.as_mut_ptr());
        (v[0], v[1])
    }
    fn terrain_cell_width(&self) -> f32 {
        cb!(self, terrain_cell_width, 16.0)
    }
    fn terrain_altitude(&self, index: i32) -> f32 {
        cb!(self, terrain_altitude, 0.0, index)
    }
    fn terrain_water_level(&self, index: i32) -> f32 {
        cb!(self, terrain_water_level, 0.0, index)
    }
    fn terrain_altitude_at(&self, x: f32, z: f32) -> f32 {
        cb!(self, terrain_altitude_at, 0.0, x, z)
    }
    fn terrain_water_level_at(&self, x: f32, z: f32) -> f32 {
        cb!(self, terrain_water_level_at, 0.0, x, z)
    }
    fn terrain_is_water_vertex(&self, x: i32, z: i32) -> bool {
        cb!(self, terrain_is_water_vertex, false, x, z)
    }
    fn terrain_cell_has_water(&self, x: i32, z: i32) -> bool {
        cb!(self, terrain_cell_has_water, false, x, z)
    }
    fn zone_drag_color(&self, zone: i32) -> u32 {
        cb!(self, zone_drag_color, 0, zone)
    }
    fn zone_display_alpha(&self) -> u8 {
        cb!(self, zone_display_alpha, 0)
    }
    fn view_rotation(&self) -> i32 {
        cb!(self, view_rotation, 0)
    }
    fn visual_time_mode(&self) -> u32 {
        cb!(self, visual_time_mode, 0)
    }
    fn resource_exists(&self, type_id: u32, group: u32, instance: u32) -> bool {
        cb!(self, resource_exists, false, type_id, group, instance)
    }

    // ---- lot manager ----
    fn lot_count(&self) -> i32 {
        cb!(self, lot_count, 0)
    }
    fn safe_lot_iterator(&self) -> u64 {
        cb!(self, safe_lot_iterator, 0)
    }
    fn safe_lot_iterator_rewind(&self, it: u64) {
        cb!(self, safe_lot_iterator_rewind, (), it)
    }
    fn safe_lot_iterator_next(&self, it: u64) -> Option<u64> {
        let lot = opt(cb!(self, safe_lot_iterator_next, 0, it));
        if lot.is_some() && self.counters.in_daily.get() {
            bump(&self.counters.lots_visited);
        }
        lot
    }
    fn safe_lot_iterator_free(&self, it: u64) {
        cb!(self, safe_lot_iterator_free, (), it)
    }
    fn get_lot(&self, x: i32, z: i32, origin_only: bool) -> Option<u64> {
        opt(cb!(self, get_lot, 0, x, z, origin_only))
    }
    fn lots_in_cell_rect(&self, rect: CellRect, origin_only: bool) -> Vec<u64> {
        let r = rect_arg(rect);
        collect_tokens(|f, ctx| cb!(self, lots_in_cell_rect, (), &r, origin_only, f, ctx))
    }
    fn lots_surrounding_lot(&self, lot: u64, list: &mut Vec<u64>, flags: u32) {
        // The game's walk skips lots already in the list it appends to
        // (§5.7 step 2): the host fills a fresh list, the core drops the
        // ones it already has, keeping the order.
        for t in collect_tokens(|f, ctx| cb!(self, lots_surrounding_lot, (), lot, flags, f, ctx)) {
            if t != 0 && !list.contains(&t) {
                list.push(t);
            }
        }
    }
    fn lots_surrounding_rect(&self, rect: CellRect, list: &mut Vec<u64>, flags: u32) {
        let r = rect_arg(rect);
        for t in collect_tokens(|f, ctx| cb!(self, lots_surrounding_rect, (), &r, flags, f, ctx)) {
            if t != 0 && !list.contains(&t) {
                list.push(t);
            }
        }
    }
    fn delete_lot(&self, lot: u64) -> bool {
        cb!(self, delete_lot, false, lot)
    }

    // ---- lots ----
    fn lot_is_initialized(&self, lot: u64) -> bool {
        cb!(self, lot_is_initialized, false, lot)
    }
    fn lot_location(&self, lot: u64) -> (i32, i32) {
        let mut v = [0i32; 2];
        cb!(self, lot_location, (), lot, v.as_mut_ptr());
        (v[0], v[1])
    }
    fn lot_size(&self, lot: u64) -> (u32, u32) {
        let mut v = [0u32; 2];
        cb!(self, lot_size, (), lot, v.as_mut_ptr());
        (v[0], v[1])
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        self.rect_out(|r| cb!(self, lot_bounding_rect, (), lot, r))
    }
    fn lot_center(&self, lot: u64) -> Vec3 {
        let mut v = [0.0f32; 3];
        cb!(self, lot_center, (), lot, v.as_mut_ptr());
        v
    }
    fn lot_zone_type(&self, lot: u64) -> i32 {
        cb!(self, lot_zone_type, 0, lot)
    }
    fn lot_altitude(&self, lot: u64) -> f32 {
        cb!(self, lot_altitude, 0.0, lot)
    }
    fn lot_set_altitude(&self, lot: u64, altitude: f32) {
        cb!(self, lot_set_altitude, (), lot, altitude)
    }
    fn lot_facing(&self, lot: u64) -> i32 {
        cb!(self, lot_facing, 0, lot)
    }
    fn lot_flags(&self, lot: u64) -> u32 {
        cb!(self, lot_flags, 0, lot)
    }
    fn lot_set_flags(&self, lot: u64, flags: u32) {
        cb!(self, lot_set_flags, (), lot, flags)
    }
    fn lot_set_vertices_locked(&self, lot: u64, locked: bool) {
        cb!(self, lot_set_vertices_locked, (), lot, locked)
    }
    fn lot_update_slope(&self, lot: u64) {
        cb!(self, lot_update_slope, (), lot)
    }
    fn lot_state(&self, lot: u64) -> u32 {
        cb!(self, lot_state, 0, lot)
    }
    fn lot_set_state(&self, lot: u64, state: u32, wealth: u32, reason: u32) -> bool {
        let old = self.lot_state(lot);
        let r = cb!(self, lot_set_state, false, lot, state, wealth, reason);
        if state == lot_state::ABANDONED && old != lot_state::ABANDONED {
            bump(&self.counters.abandoned);
        } else if state == lot_state::OCCUPIED && old == lot_state::ABANDONED {
            bump(&self.counters.restored);
        }
        r
    }
    fn lot_condition(&self, lot: u64) -> u32 {
        cb!(self, lot_condition, 0, lot)
    }
    fn lot_set_condition(&self, lot: u64, condition: u32) {
        if self.lot_condition(lot) != condition {
            bump(&self.counters.condition_changes);
        }
        cb!(self, lot_set_condition, (), lot, condition)
    }
    fn lot_is_livable(&self, lot: u64) -> bool {
        cb!(self, lot_is_livable, false, lot)
    }
    fn lot_days_since_state_change(&self, lot: u64) -> u32 {
        cb!(self, lot_days_since_state_change, 0, lot)
    }
    fn lot_building_type(&self, lot: u64, flag: bool) -> u32 {
        cb!(self, lot_building_type, 0, lot, flag)
    }
    fn lot_set_building_type(&self, lot: u64, building: u32) {
        cb!(self, lot_set_building_type, (), lot, building)
    }
    fn lot_building(&self, lot: u64) -> Option<u64> {
        opt(cb!(self, lot_building, 0, lot))
    }
    fn lot_configuration(&self, lot: u64) -> Option<u64> {
        opt(cb!(self, lot_configuration, 0, lot))
    }
    fn lot_set_configuration(&self, lot: u64, cfg: Option<u64>) {
        cb!(self, lot_set_configuration, (), lot, cfg.unwrap_or(0))
    }
    fn lot_occupant_wealth(&self, lot: u64) -> u32 {
        cb!(self, lot_occupant_wealth, 0, lot)
    }
    fn lot_occupancy(&self, lot: u64) -> f32 {
        cb!(self, lot_occupancy, 0.0, lot)
    }
    fn lot_needs_power(&self, lot: u64) -> bool {
        cb!(self, lot_needs_power, false, lot)
    }
    fn lot_has_power(&self, lot: u64) -> bool {
        cb!(self, lot_has_power, false, lot)
    }
    fn lot_set_has_power(&self, lot: u64, powered: bool) {
        cb!(self, lot_set_has_power, (), lot, powered)
    }
    fn lot_has_job_access(&self, lot: u64) -> bool {
        cb!(self, lot_has_job_access, false, lot)
    }
    fn lot_has_road_access(&self, lot: u64) -> bool {
        cb!(self, lot_has_road_access, false, lot)
    }
    fn lot_has_organic_road_access(&self, lot: u64, purpose: u32) -> bool {
        cb!(self, lot_has_organic_road_access, false, lot, purpose)
    }
    fn lot_update_road_access(&self, lot: u64) {
        cb!(self, lot_update_road_access, (), lot)
    }
    fn lot_travel_jobs(&self, lot: u64, wealth: u32) -> f32 {
        cb!(self, lot_travel_jobs, 0.0, lot, wealth)
    }
    fn lot_travel_designate(&self, lot: u64) -> Option<u64> {
        opt(cb!(self, lot_travel_designate, 0, lot))
    }
    fn lot_set_travel_designate(&self, lot: u64, designate: Option<u64>) {
        cb!(self, lot_set_travel_designate, (), lot, designate.unwrap_or(0))
    }
    fn lot_unit(&self, lot: u64) -> Option<u64> {
        opt(cb!(self, lot_unit, 0, lot))
    }
    fn lot_reset_travel_population(&self, lot: u64) {
        cb!(self, lot_reset_travel_population, (), lot)
    }
    fn lot_add_travel_population(&self, lot: u64, from: u64) {
        cb!(self, lot_add_travel_population, (), lot, from)
    }
    fn lot_is_historical(&self, lot: u64) -> bool {
        cb!(self, lot_is_historical, false, lot)
    }
    fn lot_set_historical(&self, lot: u64, historical: bool) {
        cb!(self, lot_set_historical, (), lot, historical)
    }
    fn lot_available_capacity(&self, lot: u64, dev_type: u32, flag: bool) -> u16 {
        cb!(self, lot_available_capacity, 0, lot, dev_type, flag)
    }
    fn lot_set_population(&self, lot: u64, dev_type: u32, population: u16) -> bool {
        cb!(self, lot_set_population, false, lot, dev_type, population)
    }
    fn lot_property(&self, lot: u64, id: u32) -> Option<u32> {
        let mut v = 0u32;
        cb!(self, lot_property, false, lot, id, &mut v).then_some(v)
    }
    fn lot_set_property(&self, lot: u64, id: u32, value: u32) {
        if id == lot_prop::ZOT && self.lot_property(lot, id).unwrap_or(0) != value {
            bump(&self.counters.zot_changes);
        }
        cb!(self, lot_set_property, (), lot, id, value)
    }

    // ---- lot configurations ----
    fn cfg_id(&self, cfg: u64) -> u32 {
        cb!(self, cfg_id, 0, cfg)
    }
    fn cfg_size(&self, cfg: u64) -> (u32, u32) {
        let mut v = [0u32; 2];
        cb!(self, cfg_size, (), cfg, v.as_mut_ptr());
        (v[0], v[1])
    }
    fn cfg_max_slope_before_foundation(&self, cfg: u64) -> f32 {
        cb!(self, cfg_max_slope_before_foundation, 0.0, cfg)
    }
    fn cfg_growth_stage(&self, cfg: u64) -> u8 {
        cb!(self, cfg_growth_stage, 0, cfg)
    }
    fn cfg_is_compatible_with_water(&self, cfg: u64) -> bool {
        cb!(self, cfg_is_compatible_with_water, false, cfg)
    }
    fn cfg_objects(&self, cfg: u64) -> Vec<CfgObject> {
        let mut v: Vec<CfgObject> = Vec::new();
        cb!(
            self,
            cfg_objects,
            (),
            cfg,
            Some(push_cfg_object),
            &mut v as *mut Vec<CfgObject> as *mut c_void
        );
        v
    }
    fn cfg_primary_building_object(&self, cfg: u64) -> Option<CfgObject> {
        let mut v: Vec<CfgObject> = Vec::new();
        let ctx = &mut v as *mut Vec<CfgObject> as *mut c_void;
        let found = cb!(
            self,
            cfg_primary_building_object,
            false,
            cfg,
            Some(push_cfg_object),
            ctx
        );
        if found {
            v.into_iter().next()
        } else {
            None
        }
    }
    fn cfg_primary_foundation_object(&self, cfg: u64) -> Option<CfgObject> {
        let mut v: Vec<CfgObject> = Vec::new();
        let ctx = &mut v as *mut Vec<CfgObject> as *mut c_void;
        let found = cb!(
            self,
            cfg_primary_foundation_object,
            false,
            cfg,
            Some(push_cfg_object),
            ctx
        );
        if found {
            v.into_iter().next()
        } else {
            None
        }
    }
    fn cfg_has_objects_of_type(&self, cfg: u64, object_type: u32) -> bool {
        cb!(self, cfg_has_objects_of_type, false, cfg, object_type)
    }
    fn get_lot_configuration(&self, id: u32) -> Option<u64> {
        opt(cb!(self, get_lot_configuration, 0, id))
    }
    fn object_id_array(&self, cfg: u64, index: usize) -> Vec<u32> {
        let i = u32::try_from(index).unwrap_or(u32::MAX);
        collect_u32s(|f, ctx| cb!(self, object_id_array, (), cfg, i, f, ctx)).1
    }

    // ---- slots and helpers §17 does not cover ----
    fn first_building(&self, lot: u64) -> Option<u64> {
        opt(cb!(self, first_building, 0, lot))
    }
    fn update_lot_retaining_walls(&self, lot: u64) -> bool {
        cb!(self, update_lot_retaining_walls, false, lot)
    }
    fn lot_occupants(&self, lot: u64) -> Vec<u64> {
        collect_tokens(|f, ctx| cb!(self, lot_occupants, (), lot, f, ctx))
    }
    fn lot_configuration_object(&self, lot: u64, occ: u64) -> Option<CfgObject> {
        let mut v: Vec<CfgObject> = Vec::new();
        let ctx = &mut v as *mut Vec<CfgObject> as *mut c_void;
        let found = cb!(
            self,
            lot_configuration_object,
            false,
            lot,
            occ,
            Some(push_cfg_object),
            ctx
        );
        if found {
            v.into_iter().next()
        } else {
            None
        }
    }
    fn update_lot_base_texture_occupant(&self, lot: u64, had_foundation: bool) -> bool {
        cb!(self, update_lot_base_texture_occupant, false, lot, had_foundation)
    }
    fn base_texture_occupant(&self, lot: u64, create: bool) -> Option<u64> {
        opt(cb!(self, base_texture_occupant, 0, lot, create))
    }
    fn base_texture_set(&self, occ: u64, records: &[TextureRecord]) {
        let v: Vec<fc4_lotdev_texture> = records
            .iter()
            .map(|r| fc4_lotdev_texture {
                texture: r.texture,
                x: r.x,
                z: r.z,
                orientation: r.orientation,
                byte3: r.byte3,
                extra: r.extra,
                frame: r.frame,
            })
            .collect();
        cb!(self, base_texture_set, (), occ, v.as_ptr(), v.len())
    }
    fn add_retaining_wall_occupant(&self, lot: u64, flag: u32) {
        cb!(self, add_retaining_wall_occupant, (), lot, flag)
    }
    fn update_surrounding_retaining_walls(&self, lot: u64) {
        cb!(self, update_surrounding_retaining_walls, (), lot)
    }
    fn add_network_connection_nodes(&self, lot: u64, nodes: &[ConnectionNode]) {
        let v: Vec<fc4_lotdev_conn_node> = nodes
            .iter()
            .map(|n| fc4_lotdev_conn_node {
                node_type: n.node_type,
                x: n.x,
                z: n.z,
                b: n.b,
                c: n.c,
                d: n.d,
                orientation: n.orientation,
                reserved: [0; 3],
            })
            .collect();
        cb!(self, add_network_connection_nodes, (), lot, v.as_ptr(), v.len())
    }
    fn remove_network_connection_nodes(&self, lot: u64) {
        cb!(self, remove_network_connection_nodes, (), lot)
    }
    fn start_construction_occupant(&self, lot: u64) {
        bump(&self.counters.constructions_started);
        cb!(self, start_construction_occupant, (), lot)
    }

    // ---- buildings and other occupants ----
    fn occupant_as_building(&self, occ: u64) -> Option<u64> {
        opt(cb!(self, occupant_as_building, 0, occ))
    }
    fn building_add_ref(&self, building: u64) {
        cb!(self, building_add_ref, (), building)
    }
    fn building_release(&self, building: u64) {
        cb!(self, building_release, (), building)
    }
    fn building_set_lit(&self, building: u64, lit: bool) {
        cb!(self, building_set_lit, (), building, lit)
    }
    fn occupant_type(&self, occ: u64) -> u32 {
        cb!(self, occupant_type, 0, occ)
    }
    fn occupant_position(&self, occ: u64) -> Vec3 {
        let mut v = [0.0f32; 3];
        cb!(self, occupant_position, (), occ, v.as_mut_ptr());
        v
    }
    fn occupant_bounding_box(&self, occ: u64) -> (Vec3, Vec3) {
        let mut v = [0.0f32; 6];
        cb!(self, occupant_bounding_box, (), occ, v.as_mut_ptr());
        ([v[0], v[1], v[2]], [v[3], v[4], v[5]])
    }
    fn occupant_city_cells(&self, occ: u64) -> CellRect {
        self.rect_out(|r| cb!(self, occupant_city_cells, (), occ, r))
    }
    fn occupant_in_group(&self, occ: u64, group: u32) -> bool {
        cb!(self, occupant_in_group, false, occ, group)
    }
    fn occupant_post_message(&self, occ: u64, message: u32) {
        cb!(self, occupant_post_message, (), occ, message)
    }
    fn occupant_set_removal_flags(&self, occ: u64, flags: u32) {
        cb!(self, occupant_set_removal_flags, (), occ, flags)
    }
    fn occupant_set_lot_tag(&self, occ: u64, tag: u32) {
        cb!(self, occupant_set_lot_tag, (), occ, tag)
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<u32> {
        let mut v = 0u32;
        cb!(self, occupant_property, false, occ, id, &mut v).then_some(v)
    }
    fn occupant_set_property(&self, occ: u64, id: u32, value: u32) {
        cb!(self, occupant_set_property, (), occ, id, value)
    }
    fn occupant_init(&self, occ: u64) {
        cb!(self, occupant_init, (), occ)
    }
    fn occupant_set_flag(&self, occ: u64, flag: u32) {
        cb!(self, occupant_set_flag, (), occ, flag)
    }
    fn occupant_is_flag_set(&self, occ: u64, flag: u32) -> bool {
        cb!(self, occupant_is_flag_set, false, occ, flag)
    }
    fn occupant_shutdown(&self, occ: u64) {
        cb!(self, occupant_shutdown, (), occ)
    }
    fn occupant_set_orientation(&self, occ: u64, orientation: u32) {
        cb!(self, occupant_set_orientation, (), occ, orientation)
    }
    fn occupant_set_position(&self, occ: u64, pos: Vec3) {
        cb!(self, occupant_set_position, (), occ, pos.as_ptr())
    }
    fn occupant_insert(&self, occ: u64) {
        cb!(self, occupant_insert, (), occ)
    }
    fn occupant_remove(&self, occ: u64, a: u32, flags: u32) {
        cb!(self, occupant_remove, (), occ, a, flags)
    }
    fn occupant_changed(&self, occ: u64) {
        cb!(self, occupant_changed, (), occ)
    }
    fn occupant_moved(&self, occ: u64) {
        cb!(self, occupant_moved, (), occ)
    }
    fn occupants_in_bbox(&self, min: Vec3, max: Vec3) -> Vec<u64> {
        collect_tokens(|f, ctx| cb!(self, occupants_in_bbox, (), min.as_ptr(), max.as_ptr(), f, ctx))
    }
    fn occupants_in_cells(&self, rect: CellRect, occupant_type: u32) -> Vec<u64> {
        let r = rect_arg(rect);
        collect_tokens(|f, ctx| cb!(self, occupants_in_cells, (), &r, occupant_type, f, ctx))
    }
    fn occupants_in_box(&self, min: Vec3, max: Vec3, occupant_type: u32) -> Vec<u64> {
        collect_tokens(|f, ctx| {
            cb!(
                self,
                occupants_in_box,
                (),
                min.as_ptr(),
                max.as_ptr(),
                occupant_type,
                f,
                ctx
            )
        })
    }
    fn building_orientation(&self, building: u64) -> i32 {
        cb!(self, building_orientation, 0, building)
    }

    // ---- foundations ----
    fn foundation_kind(&self, occ: u64) -> Option<i32> {
        let mut k = 0i32;
        cb!(self, foundation_kind, false, occ, &mut k).then_some(k)
    }
    fn foundation_type(&self, occ: u64) -> u32 {
        cb!(self, foundation_type, 0, occ)
    }
    fn add_new_foundation_occupant(
        &self,
        foundation_type: u32,
        kind: i32,
        pos: &mut Vec3,
        dx: f32,
        dz: f32,
        orient: i32,
        bottom: f32,
    ) -> bool {
        cb!(
            self,
            add_new_foundation_occupant,
            false,
            foundation_type,
            kind,
            pos.as_mut_ptr(),
            dx,
            dz,
            orient,
            bottom
        )
    }

    // ---- prop manager ----
    fn prop_create(&self, prop_type: u32) -> Option<u64> {
        opt(cb!(self, prop_create, 0, prop_type))
    }
    fn lot_props(&self, lot: u64) -> Vec<u64> {
        collect_tokens(|f, ctx| cb!(self, lot_props, (), lot, f, ctx))
    }
    fn prop_add_to_lot(&self, lot: u64, occ: u64) -> bool {
        cb!(self, prop_add_to_lot, false, lot, occ)
    }
    fn prop_add(&self, prop_type: u32, pos: Vec3, orientation: i32) -> Option<u64> {
        opt(cb!(self, prop_add, 0, prop_type, pos.as_ptr(), orientation))
    }
    fn remove_lot_props(&self, lot: u64) {
        cb!(self, remove_lot_props, (), lot)
    }
    fn prop_remove(&self, occ: u64) -> bool {
        cb!(self, prop_remove, false, occ)
    }
    fn remove_building_props(&self, building: u64) {
        cb!(self, remove_building_props, (), building)
    }
    fn make_burnt_building(&self, building: u64) -> Option<BurntBuilding> {
        let mut b = fc4_lotdev_burnt::default();
        if !cb!(self, make_burnt_building, false, building, &mut b) || b.occupant == 0 {
            return None;
        }
        Some(BurntBuilding {
            occupant: b.occupant,
            min: b.min,
            max: b.max,
        })
    }
    fn add_rubble_decal(&self, lot: u64, burnt: &BurntBuilding, instance: u32, size: f32) {
        let b = fc4_lotdev_burnt {
            occupant: burnt.occupant,
            min: burnt.min,
            max: burnt.max,
        };
        cb!(self, add_rubble_decal, (), lot, &b, instance, size)
    }

    // ---- BDS ----
    fn building_profile(&self, building: u32) -> BuildingProfile {
        let mut p = fc4_lotdev_profile::default();
        cb!(self, building_profile, (), building, &mut p);
        BuildingProfile::basic(p.purpose.min(255) as u8, p.wealth.min(255) as u8, p.capacity)
    }
    fn developer_type(&self, purpose: u32, wealth: u32) -> u32 {
        cb!(self, developer_type, 0, purpose, wealth)
    }
    fn developer_of_type(&self, dev_type: u32) -> Option<u64> {
        opt(cb!(self, developer_of_type, 0, dev_type))
    }
    fn wealth_of_type(&self, dev_type: u32) -> u32 {
        cb!(self, wealth_of_type, 0, dev_type)
    }
    fn developer_for(&self, purpose: u32, wealth: u32) -> Option<u64> {
        opt(cb!(self, developer_for, 0, purpose, wealth))
    }
    fn is_building_type(&self, id: u32) -> bool {
        cb!(self, is_building_type, false, id)
    }
    fn building_types_by_family(&self, family: u32) -> Vec<u32> {
        collect_u32s(|f, ctx| cb!(self, building_types_by_family, (), family, f, ctx)).1
    }
    fn add_new_building_occupant(&self, building: u32, pos: Vec3, orientation: i32) -> Option<u64> {
        opt(cb!(
            self,
            add_new_building_occupant,
            0,
            building,
            pos.as_ptr(),
            orientation
        ))
    }
    fn claim_construction_resources(&self, amount: u16) -> bool {
        cb!(self, claim_construction_resources, false, amount)
    }
    fn cheat(&self, cheat: Cheat) -> bool {
        let which = match cheat {
            Cheat::IgnorePlumbing => 0,
            Cheat::IgnorePower => 1,
            Cheat::CoffeeBar => 2,
        };
        cb!(self, cheat, false, which)
    }

    // ---- growth developers ----
    fn dev_purpose(&self, dev: u64) -> u32 {
        cb!(self, dev_purpose, 0, dev)
    }
    fn dev_desirability(&self, dev: u64, x: i32, z: i32) -> i16 {
        cb!(self, dev_desirability, 0, dev, x, z)
    }
    fn dev_water_supply_cap(&self, dev: u64) -> u8 {
        cb!(self, dev_water_supply_cap, 0, dev)
    }

    // ---- power, plumbing, traffic, demand ----
    fn is_powered(&self, x: i32, z: i32) -> bool {
        cb!(self, is_powered, false, x, z)
    }
    fn is_cell_watered(&self, x: i32, z: i32) -> bool {
        cb!(self, is_cell_watered, false, x, z)
    }
    fn trip_length(&self, x: i32, z: i32) -> u8 {
        cb!(self, trip_length, 0, x, z)
    }
    fn jobs_demand(&self) -> Option<f32> {
        let mut v = 0.0f32;
        cb!(self, jobs_demand, false, &mut v).then_some(v)
    }

    // ---- flora ----
    fn flora_type_valid(&self, id: u32) -> bool {
        cb!(self, flora_type_valid, false, id)
    }
    fn flora_types_by_family(&self, id: u32) -> Option<Vec<u32>> {
        let (found, ids) = collect_u32s(|f, ctx| cb!(self, flora_types_by_family, false, id, f, ctx));
        found.then_some(ids)
    }
    fn flora_add(&self, flora_type: u32, pos: Vec3) -> Option<u64> {
        opt(cb!(self, flora_add, 0, flora_type, pos.as_ptr()))
    }
    fn flora_demolish_in_cell_area(&self, rect: CellRect) {
        let r = rect_arg(rect);
        cb!(self, flora_demolish_in_cell_area, (), &r)
    }

    // ---- demolition ----
    fn demolish_occupant(&self, occ: u64) -> bool {
        cb!(self, demolish_occupant, false, occ)
    }
    fn demolish_lot(&self, lot: u64) -> bool {
        cb!(self, demolish_lot, false, lot)
    }
    fn modify_terrain_height(
        &self,
        per_vertex: bool,
        x0: i32,
        z0: i32,
        x1: i32,
        z1: i32,
        h: f32,
        exempt: Option<u64>,
    ) -> i32 {
        cb!(
            self,
            modify_terrain_height,
            0,
            per_vertex,
            x0,
            z0,
            x1,
            z1,
            h,
            exempt.unwrap_or(0)
        )
    }

    // ---- business units ----
    fn unit_members(&self, unit: u64) -> Vec<u64> {
        collect_tokens(|f, ctx| cb!(self, unit_members, (), unit, f, ctx))
    }
    fn unit_create(&self) -> u64 {
        cb!(self, unit_create, 0)
    }
    fn unit_add(&self, unit: u64, lot: u64) -> bool {
        unit != 0 && cb!(self, unit_add, false, unit, lot)
    }
    fn unit_remove(&self, unit: u64, lot: u64) -> bool {
        unit != 0 && cb!(self, unit_remove, false, unit, lot)
    }

    // ---- persistence ----
    fn building_ref(&self, building: u64) -> RefValue {
        // The stream writes the object behind the token (fc4.h fc4_ref).
        RefValue::Host {
            token: building,
            class_id: 0,
        }
    }
    fn resolve_building(&self, r: &RefValue) -> Option<u64> {
        match r {
            RefValue::Host { token, .. } => opt(*token),
            _ => None,
        }
    }
}

// ---- the save stream --------------------------------------------------------------------------------

/// The record reader: delegates to [`CIn`], except that references come back
/// as host tokens the core owns one reference on (the night deque's).
struct LIn<'a> {
    inner: CIn<'a>,
    host: &'a CHost,
}

impl InStream for LIn<'_> {
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
            self.host.building_add_ref(out.token);
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

// ---- the class statics ----------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_lotdev_statics`): the class-wide statics of
/// §10.1.
#[repr(C)]
pub struct fc4_lotdev_statics {
    _private: [u8; 0],
}

struct Statics(SharedStatics);

/// The statics, the file-local generator seeded from the session seed
/// (foundations §6.3: `derive_seed(session_seed, 0x6C09C0F4)`), `rand()` at
/// its process-start state 1 (foundations §12.6).
#[no_mangle]
pub extern "C" fn fc4_lotdev_statics_create(session_seed: u32) -> *mut fc4_lotdev_statics {
    guard(std::ptr::null_mut(), || {
        let seeds = FixedSeeds {
            wall_clock: session_seed,
            process_clock: session_seed,
        };
        Box::into_raw(Box::new(Statics(LotDeveloperStatics::from_session(
            session_seed,
            seeds,
        )))) as *mut fc4_lotdev_statics
    })
}

/// Destroys the statics (the lot developers made with them keep their own
/// share).
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_statics_destroy(s: *mut fc4_lotdev_statics) {
    if s.is_null() {
        return;
    }
    guard((), || drop(Box::from_raw(s as *mut Statics)));
}

/// `QueryInterface` (§1.1): 1 the primary interface (and `cIGZUnknown`),
/// 2 the message target (sub-object +4), 3 the serializable (+8), 0 none.
#[no_mangle]
pub extern "C" fn fc4_lotdev_query_interface(iid: u32) -> u32 {
    if !answers_iid(iid) {
        return 0;
    }
    match iid {
        IID_UNKNOWN | IID_LOT_DEVELOPER => 1,
        IID_MESSAGE_TARGET2 => 2,
        IID_SERIALIZABLE => 3,
        _ => 0,
    }
}

// ---- handles ----------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_lotdev`).
#[repr(C)]
pub struct fc4_lotdev {
    _private: [u8; 0],
}

struct Handle {
    host: Rc<CHost>,
    statics: SharedStatics,
    core: RefCell<LotDeveloper>,
    terrain: RefCell<TerrainView>,
    depth: Cell<u32>,
    queue: RefCell<VecDeque<LotDevMessage>>,
    refused: Cell<u32>,
    queued: Cell<u32>,
    finished: Cell<u32>,
    demolitions: Cell<u32>,
    night_batches: Cell<u32>,
}

impl Handle {
    /// Runs `f` on the core with the call depth counted; `fallback` (and a
    /// count) when the core is busy. At the end of the outermost call, the
    /// queued messages are handled.
    fn call<R>(&self, fallback: R, f: impl FnOnce(&mut LotDeveloper) -> R) -> R {
        let Ok(mut core) = self.core.try_borrow_mut() else {
            bump(&self.refused);
            return fallback;
        };
        self.depth.set(self.depth.get() + 1);
        let r = f(&mut core);
        drop(core);
        if self.depth.get() == 1 {
            loop {
                let next = self.queue.borrow_mut().pop_front();
                let Some(m) = next else { break };
                if let Ok(mut core) = self.core.try_borrow_mut() {
                    self.message(&mut core, &m);
                }
            }
            if let Ok(core) = self.core.try_borrow() {
                self.refresh_terrain(&core);
            }
        }
        self.depth.set(self.depth.get() - 1);
        r
    }

    /// One message, with the counters the month line reports.
    fn message(&self, core: &mut LotDeveloper, m: &LotDevMessage) -> bool {
        match m.msg_type {
            msg::SIM_NEW_DAY => {
                self.host.counters.in_daily.set(true);
                let r = core.do_message(m);
                self.host.counters.in_daily.set(false);
                r
            }
            msg::ANIMATION_TICK => {
                let mode = core.night_mode();
                let r = core.do_message(m);
                if mode != 0 && core.night_acc() == 0 {
                    bump(&self.night_batches);
                }
                r
            }
            msg::LOT_STATE_CHANGED => {
                if m.data1 == lot_state::CONSTRUCTING && m.data2 == lot_state::OCCUPIED {
                    bump(&self.finished);
                }
                core.do_message(m)
            }
            _ => core.do_message(m),
        }
    }

    /// Refreshes the snapshot (skipped while a geometry slot is reading it:
    /// the next outermost call refreshes it).
    fn refresh_terrain(&self, core: &LotDeveloper) {
        if let Ok(mut t) = self.terrain.try_borrow_mut() {
            *t = core.terrain_view();
        }
    }

    /// Runs a terrain or geometry slot on the snapshot (never the core).
    fn terrain<R>(&self, f: impl FnOnce(&TerrainView, &dyn LotDeveloperHost) -> R) -> R {
        let t = self.terrain.borrow();
        f(&t, &*self.host)
    }

    /// Like [`Handle::terrain`], with the class statics (R8).
    fn geometry<R>(&self, f: impl FnOnce(&TerrainView, &dyn LotDeveloperHost, &SharedStatics) -> R) -> R {
        let t = self.terrain.borrow();
        f(&t, &*self.host, &self.statics)
    }
}

/// Runs `f` on the handle; the fallback for NULL or a panic.
fn with_handle<R>(p: *const fc4_lotdev, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_lotdev*.
    let Some(h) = (unsafe { (p as *const Handle).as_ref() }) else {
        return fallback;
    };
    guard(fallback, || f(h))
}

/// Runs `f` on the core (refused with `fallback` when busy).
fn core<R: Copy>(p: *const fc4_lotdev, fallback: R, f: impl FnOnce(&mut LotDeveloper) -> R) -> R {
    with_handle(p, fallback, |h| h.call(fallback, f))
}

fn host_ok(host: *const fc4_lotdev_host) -> Option<fc4_lotdev_host> {
    // SAFETY: documented precondition: NULL or a valid host table.
    let h = unsafe { host.as_ref() }?;
    ((h.struct_size as usize) >= std::mem::size_of::<fc4_lotdev_host>()).then_some(*h)
}

/// SAFETY: `r` must be NULL or valid.
unsafe fn rect_in(r: *const fc4_cell_rect) -> Option<CellRect> {
    r.as_ref().map(|r| CellRect::from(*r))
}

/// Creates a lot developer (constructor values, §1.2) over the host table
/// (copied) and the class statics `s` (shared). NULL when the table is
/// incomplete or `s` is NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_create(
    host: *const fc4_lotdev_host,
    s: *const fc4_lotdev_statics,
) -> *mut fc4_lotdev {
    guard(std::ptr::null_mut(), || {
        let Some(h) = host_ok(host) else {
            return std::ptr::null_mut();
        };
        let Some(statics) = (s as *const Statics).as_ref().map(|s| s.0.clone()) else {
            return std::ptr::null_mut();
        };
        let host = Rc::new(CHost {
            t: h,
            counters: Counters::default(),
        });
        let core = LotDeveloper::new(host.clone(), statics.clone());
        let terrain = core.terrain_view();
        Box::into_raw(Box::new(Handle {
            host,
            statics,
            core: RefCell::new(core),
            terrain: RefCell::new(terrain),
            depth: Cell::new(0),
            queue: RefCell::new(VecDeque::new()),
            refused: Cell::new(0),
            queued: Cell::new(0),
            finished: Cell::new(0),
            demolitions: Cell::new(0),
            night_batches: Cell::new(0),
        })) as *mut fc4_lotdev
    })
}

/// Destroys the lot developer. An initialised one is shut down first
/// (§1.4); a loaded one that was never initialised releases the night
/// deque's references.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_destroy(p: *mut fc4_lotdev) {
    if p.is_null() {
        return;
    }
    guard((), || {
        let b = Box::from_raw(p as *mut Handle);
        if let Ok(mut core) = b.core.try_borrow_mut() {
            if core.is_initialised() {
                core.shutdown();
            } else {
                for bld in core.night().iter().flatten() {
                    b.host.building_release(*bld);
                }
            }
        }
        drop(b);
    });
}

/// `Init` (slot 3, §1.3).
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_init(p: *mut fc4_lotdev) -> bool {
    with_handle(p, false, |h| h.call(false, LotDeveloper::init))
}

/// `Shutdown` (slot 4, §1.4): always true for a live object.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_shutdown(p: *mut fc4_lotdev) -> bool {
    core(p, false, LotDeveloper::shutdown)
}

/// `DoMessage` (§3.2). A message delivered during another call is queued
/// (the host keeps its tokens alive until the outermost call returns) and
/// answers true.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_do_message(p: *mut fc4_lotdev, m: *const fc4_lotdev_message) -> bool {
    let Some(m) = m.as_ref() else { return true };
    let msg = LotDevMessage {
        msg_type: m.msg_type,
        data1: m.data1,
        data2: m.data2,
        data3: m.data3,
        data4: m.data4,
        void1: opt(m.occupant),
        lot: opt(m.lot),
    };
    with_handle(p, true, |h| {
        if h.depth.get() > 0 {
            bump(&h.queued);
            h.queue.borrow_mut().push_back(msg);
            return true;
        }
        h.call(true, |c| h.message(c, &msg))
    })
}

/// The daily work budget (§4.1 [choice]): at most `n` lots per delivery of
/// the day message; 0 = unlimited.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_set_daily_work_budget(p: *mut fc4_lotdev, n: u32) {
    core(p, (), |c| c.set_daily_work_budget(if n == 0 { u32::MAX } else { n }))
}

// ---- construction, demolition, buildings (§6, §7) -----------------------------------------------------

/// Slot 6 `StartConstruction(lot, cfg, type, animate)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_start_construction(
    p: *mut fc4_lotdev,
    lot: u64,
    cfg: u64,
    building: u32,
    animate: bool,
) -> bool {
    core(p, false, |c| c.start_construction(lot, opt(cfg), building, animate))
}

/// Slot 7 `ConstructDefaultLot(lot, cfg, type, animate)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_construct_default_lot(
    p: *mut fc4_lotdev,
    lot: u64,
    cfg: u64,
    building: u32,
    animate: bool,
) -> bool {
    core(p, false, |c| c.construct_default_lot(lot, opt(cfg), building, animate))
}

/// Slot 8 `AbandonBuilding(lot, reason)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_abandon_building(p: *mut fc4_lotdev, lot: u64, reason: u32) -> bool {
    core(p, false, |c| c.abandon_building(lot, reason))
}

/// Slot 9 `ReoccupyBuilding(lot, devType)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_reoccupy_building(p: *mut fc4_lotdev, lot: u64, dev_type: u32) -> bool {
    core(p, false, |c| c.reoccupy_building(lot, dev_type))
}

/// Slot 10 `ChangeBuilding(lot, type)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_change_building(p: *mut fc4_lotdev, lot: u64, building: u32) -> bool {
    core(p, false, |c| c.change_building(lot, building))
}

/// Slot 11 `RemoveBuildingZotProp(lot)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_remove_building_zot_prop(p: *mut fc4_lotdev, lot: u64) -> bool {
    core(p, false, |c| c.remove_zot(lot))
}

/// Slot 12 `StartDemolishLot(lot)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_start_demolish_lot(p: *mut fc4_lotdev, lot: u64) -> bool {
    with_handle(p, false, |h| {
        h.call(false, |c| {
            bump(&h.demolitions);
            c.start_demolish_lot(lot)
        })
    })
}

/// Slot 13 `EndDemolishLot(lot)`: true (§11.1, the original's is undefined).
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_end_demolish_lot(p: *mut fc4_lotdev, lot: u64) -> bool {
    core(p, true, |c| c.end_demolish_lot(lot))
}

/// Slot 14 `ExplodeLot(lot, burnt)`: true (§11.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_explode_lot(p: *mut fc4_lotdev, lot: u64, burnt: bool) -> bool {
    with_handle(p, true, |h| {
        h.call(true, |c| {
            bump(&h.demolitions);
            c.explode_lot(lot, burnt)
        })
    })
}

/// Slot 37 `UpdateLotElements(lot, kind)`: true.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_update_lot_elements(p: *mut fc4_lotdev, lot: u64, kind: u32) -> bool {
    core(p, true, |c| c.update_lot_elements(lot, kind))
}

/// Slot 44 `SetNormalLotBaseTexture(lot, cfg, flag)` (the plugin returns the
/// lot).
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_set_normal_lot_base_texture(p: *mut fc4_lotdev, lot: u64, cfg: u64, flag: bool) {
    core(p, (), |c| c.set_normal_lot_base_texture(lot, opt(cfg), flag))
}

/// Slot 46 `GetForceEmptyLotBaseTextures`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_get_force_empty(p: *const fc4_lotdev) -> bool {
    with_handle(p, false, |h| {
        h.core.try_borrow().map(|c| c.force_empty()).unwrap_or(false)
    })
}

/// Slot 47 `SetForceEmptyLotBaseTextures(f)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_set_force_empty(p: *mut fc4_lotdev, f: bool) {
    core(p, (), |c| c.set_force_empty_lot_base_texture(f))
}

/// `UpdateLot(lot)` (§4.2): the daily visit of one lot (tests and tools).
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_update_lot(p: *mut fc4_lotdev, lot: u64) -> bool {
    core(p, false, |c| c.update_lot(lot))
}

// ---- business units (§5.7) ------------------------------------------------------------------------

/// Slot 33 `RemoveLotFromItsBusinessUnit(lot)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_remove_lot_from_its_business_unit(p: *mut fc4_lotdev, lot: u64) -> bool {
    core(p, false, |c| c.remove_lot_from_its_business_unit(lot))
}

/// Slot 34 `UpdateTravelDesignates(u&)`: `*unit` is replaced by the unit a
/// dissolved member joins, or 0.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_update_travel_designates(p: *mut fc4_lotdev, unit: *mut u64) {
    let Some(u) = unit.as_mut() else { return };
    let mut v = opt(*u);
    core(p, (), |c| c.update_travel_designates(&mut v));
    *u = v.unwrap_or(0);
}

/// Slot 35 `UpdateTravelPopulations(u)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_update_travel_populations(p: *mut fc4_lotdev, unit: u64) {
    if unit != 0 {
        core(p, (), |c| c.update_travel_populations(unit))
    }
}

/// Slot 36 `NeedsAnchorBuilding(rect, purpose)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_needs_anchor_building(
    p: *mut fc4_lotdev,
    rect: *const fc4_cell_rect,
    purpose: i32,
) -> bool {
    let Some(r) = rect_in(rect) else { return false };
    core(p, false, |c| c.needs_anchor_building(r, purpose))
}

// ---- terrain and cost slots (§11.1, §11.4) --------------------------------------------------------

/// Slot 15 `GetTerrainAltitudeByVertex(x, z, withWater)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_terrain_altitude_by_vertex(
    p: *const fc4_lotdev,
    x: i32,
    z: i32,
    with_water: bool,
) -> f32 {
    with_handle(p, 0.0, |h| {
        h.terrain(|t, host| t.altitude_by_vertex(host, x, z, with_water))
    })
}

/// Slot 16 `GetLowestOrHighestAreaAltitude(SC4Rect<float>, lowest,
/// withWater)` (§17.2): `rect4 = {x1, z1, x2, z2}` in metres.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_area_altitude_f(
    p: *const fc4_lotdev,
    rect4: *const f32,
    lowest: bool,
    with_water: bool,
) -> f32 {
    if rect4.is_null() {
        return 0.0;
    }
    let f = std::slice::from_raw_parts(rect4, 4);
    let r = [f[0], f[1], f[2], f[3]];
    with_handle(p, 0.0, |h| {
        h.terrain(|t, host| t.lowest_or_highest_area_altitude_f(host, r, lowest, with_water))
    })
}

/// Slot 17 `GetLowestOrHighestAreaAltitude(SC4Rect<int32>, lowest,
/// withWater)` over the vertices `x1 ..= x2 + 1`, `z1 ..= z2 + 1`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_area_altitude(
    p: *const fc4_lotdev,
    rect: *const fc4_cell_rect,
    lowest: bool,
    with_water: bool,
) -> f32 {
    let Some(r) = rect_in(rect) else { return 0.0 };
    with_handle(p, 0.0, |h| {
        h.terrain(|t, host| t.lowest_or_highest_area_altitude(host, r, lowest, with_water))
    })
}

/// Slot 18 `GetAverageAreaAltitude(SC4Rect<int32>, withWater)` (§11.1: the
/// SDK's float rectangle is wrong).
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_average_area_altitude(
    p: *const fc4_lotdev,
    rect: *const fc4_cell_rect,
    with_water: bool,
) -> f32 {
    let Some(r) = rect_in(rect) else { return 0.0 };
    with_handle(p, 0.0, |h| {
        h.terrain(|t, host| t.average_area_altitude(host, r, with_water))
    })
}

// ---- levelling (§17.4) --------------------------------------------------------------------------------

/// Slot 19 `LevelAreaInternalVertices(rect, target, skipWater, force, lot)`:
/// the number of `ModifyTerrainHeight` calls (EAX, not a bool).
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_level_area_internal_vertices(
    p: *mut fc4_lotdev,
    rect: *const fc4_cell_rect,
    target: f32,
    skip_water: bool,
    force: bool,
    lot: u64,
) -> i32 {
    let Some(r) = rect_in(rect) else { return 0 };
    with_handle(p, 0, |h| {
        h.terrain(|t, host| t.level_area_internal_vertices(host, r, target, skip_water, force, opt(lot)))
    })
}

/// Slot 20 `LevelBorderingVertices(rect, h, unused, lot)`:
/// `ModifyTerrainHeight`'s EAX.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_level_bordering_vertices(
    p: *mut fc4_lotdev,
    rect: *const fc4_cell_rect,
    height: f32,
    lot: u64,
) -> i32 {
    let Some(r) = rect_in(rect) else { return 0 };
    with_handle(p, 0, |h| {
        h.terrain(|t, host| t.level_bordering_vertices(host, r, height, opt(lot)))
    })
}

/// Slot 21 `AttemptAreaLevelling(rect, unused, withWater, perVertex,
/// maxAreaSlope, lot)`: 0 or `ModifyTerrainHeight`'s EAX.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_attempt_area_levelling(
    p: *mut fc4_lotdev,
    rect: *const fc4_cell_rect,
    with_water: bool,
    per_vertex: bool,
    max_area_slope: f32,
    lot: u64,
) -> i32 {
    let Some(r) = rect_in(rect) else { return 0 };
    with_handle(p, 0, |h| {
        h.terrain(|t, host| t.attempt_area_levelling(host, r, with_water, per_vertex, max_area_slope, opt(lot)))
    })
}

/// Slot 22 `GetOccupantCellCoverage(occupant, rect&)`: writes the
/// rectangle. The slot has no defined result (LD18, LD27), so none is
/// returned.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_occupant_cell_coverage(p: *const fc4_lotdev, occ: u64, out: *mut fc4_cell_rect) {
    let Some(o) = out.as_mut() else { return };
    let r = with_handle(p, CellRect::new(0, 0, 0, 0), |h| {
        h.terrain(|t, host| t.occupant_cell_coverage(host, occ))
    });
    *o = fc4_cell_rect::from(r);
}

/// Slot 25 `IsAreaFlat(rect, withWater)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_is_area_flat(
    p: *const fc4_lotdev,
    rect: *const fc4_cell_rect,
    with_water: bool,
) -> bool {
    let Some(r) = rect_in(rect) else { return false };
    with_handle(p, false, |h| h.terrain(|t, host| t.is_area_flat(host, r, with_water)))
}

/// Slot 26 `IsLotFlat(lot, withWater)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_is_lot_flat(p: *const fc4_lotdev, lot: u64, with_water: bool) -> bool {
    with_handle(p, false, |h| h.terrain(|t, host| t.is_lot_flat(host, lot, with_water)))
}

/// Slot 27 `GetAreaAltitudeRange(rect, withWater)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_area_altitude_range(
    p: *const fc4_lotdev,
    rect: *const fc4_cell_rect,
    with_water: bool,
) -> f32 {
    let Some(r) = rect_in(rect) else { return 0.0 };
    with_handle(p, 0.0, |h| {
        h.terrain(|t, host| t.area_altitude_range(host, r, with_water))
    })
}

/// Slot 28 `GetLotAltitudeRange(lot, withWater)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_lot_altitude_range(p: *const fc4_lotdev, lot: u64, with_water: bool) -> f32 {
    with_handle(p, 0.0, |h| {
        h.terrain(|t, host| t.lot_altitude_range(host, lot, with_water))
    })
}

/// Slot 29 `GetAreaSlope(rect, withWater)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_area_slope(
    p: *const fc4_lotdev,
    rect: *const fc4_cell_rect,
    with_water: bool,
) -> f32 {
    let Some(r) = rect_in(rect) else { return 0.0 };
    with_handle(p, 0.0, |h| h.terrain(|t, host| t.area_slope(host, r, with_water)))
}

/// Slot 30 `GetLotSlope(lot, withWater)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_lot_slope(p: *const fc4_lotdev, lot: u64, with_water: bool) -> f32 {
    with_handle(p, 0.0, |h| h.terrain(|t, host| t.lot_slope(host, lot, with_water)))
}

/// Slot 31 `GetOccupantSlope(occupant, withWater)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_occupant_slope(p: *const fc4_lotdev, occ: u64, with_water: bool) -> f32 {
    with_handle(p, 0.0, |h| h.terrain(|t, host| t.occupant_slope(host, occ, with_water)))
}

/// Slot 32 `GetLotDevelopmentCost(rect)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_lot_development_cost(p: *const fc4_lotdev, rect: *const fc4_cell_rect) -> i64 {
    let Some(r) = rect_in(rect) else { return 0 };
    with_handle(p, 0, |h| h.terrain(|t, host| t.lot_development_cost(host, r)))
}

// ---- object positions (§17.6) -----------------------------------------------------------------------

/// SAFETY: `o` must be valid, its `ids` NULL or `n_ids` values.
unsafe fn cfg_object_in(o: &fc4_lotdev_cfg_object) -> CfgObject {
    let mut v: Vec<CfgObject> = Vec::new();
    push_cfg_object(&mut v as *mut Vec<CfgObject> as *mut c_void, o);
    v.pop().unwrap_or_default()
}

/// Slot 39 `GetPositionForLotConfigurationObject(lot, cfg, obj, pos,
/// orient)`: the position into `pos3`, the orientation returned (the slot's
/// EAX is the address of `orient`, §17.6; the plugin returns that).
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_object_position(
    p: *const fc4_lotdev,
    lot: u64,
    cfg: u64,
    obj: *const fc4_lotdev_cfg_object,
    pos3: *mut f32,
) -> i32 {
    let Some(o) = obj.as_ref() else { return 0 };
    let o = cfg_object_in(o);
    let (pos, orient) = with_handle(p, ([0.0f32; 3], 0), |h| {
        h.terrain(|t, host| t.object_position(host, lot, opt(cfg), &o))
    });
    if !pos3.is_null() {
        std::ptr::copy_nonoverlapping(pos.as_ptr(), pos3, 3);
    }
    orient
}

/// Slot 40 `GetPositionForBuilding(lot, pos, orient)`: false (EAX 0) and
/// nothing written without a configuration or primary building object.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_position_for_building(
    p: *const fc4_lotdev,
    lot: u64,
    pos3: *mut f32,
    orient: *mut i32,
) -> bool {
    let got = with_handle(p, None, |h| h.terrain(|t, host| t.position_for_building(host, lot)));
    let Some((pos, o)) = got else { return false };
    if !pos3.is_null() {
        std::ptr::copy_nonoverlapping(pos.as_ptr(), pos3, 3);
    }
    if let Some(r) = orient.as_mut() {
        *r = o;
    }
    true
}

// ---- boundary textures (§17.7) ------------------------------------------------------------------------

/// Slot 42 `GetBoundaryLotTexture(sides, texture, rotation, flip)`: false
/// and nothing written while the boundary map is empty (before `Init`,
/// after `Shutdown`; LD-Q3). The slot has no defined return.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_boundary_lot_texture(
    p: *const fc4_lotdev,
    sides4: *const i32,
    texture: *mut u32,
    rotation: *mut u8,
    flip: *mut bool,
) -> bool {
    if sides4.is_null() {
        return false;
    }
    let a = std::slice::from_raw_parts(sides4, 4);
    let sides = [a[0], a[1], a[2], a[3]];
    let Some(rule) = with_handle(p, None, |h| h.terrain(|t, _| t.boundary_lot_texture(sides))) else {
        return false;
    };
    if let Some(r) = texture.as_mut() {
        *r = rule.texture;
    }
    if let Some(r) = rotation.as_mut() {
        *r = rule.rotation;
    }
    if let Some(r) = flip.as_mut() {
        *r = rule.flip;
    }
    true
}

/// Slot 43 `SetEmptyLotBaseTexture(lot)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_set_empty_lot_base_texture(p: *mut fc4_lotdev, lot: u64) {
    with_handle(p, (), |h| h.terrain(|t, host| t.set_empty_lot_base_texture(host, lot)))
}

// ---- foundations (§17.5) ------------------------------------------------------------------------------

/// Slot 48 `GetFoundationByOccupant(occ)`: borrowed, 0 = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_foundation_by_occupant(p: *const fc4_lotdev, occ: u64) -> u64 {
    with_handle(p, 0, |h| {
        h.terrain(|t, host| t.foundation_by_occupant(host, occ).unwrap_or(0))
    })
}

/// Slot 49 `GetFoundationByStandardCityCell(kind, x, z)`: borrowed.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_foundation_by_cell(p: *const fc4_lotdev, kind: i32, x: i32, z: i32) -> u64 {
    with_handle(p, 0, |h| {
        h.terrain(|t, host| t.foundation_by_cell(host, kind, x, z).unwrap_or(0))
    })
}

/// Slot 50 `GetFoundationByPosition(kind, x, z)`: borrowed.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_foundation_by_position(p: *const fc4_lotdev, kind: i32, x: f32, z: f32) -> u64 {
    with_handle(p, 0, |h| {
        h.terrain(|t, host| t.foundation_by_position(host, kind, x, z).unwrap_or(0))
    })
}

/// Slot 51 `GetLotFoundation(lot)`: borrowed.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_lot_foundation(p: *const fc4_lotdev, lot: u64) -> u64 {
    with_handle(p, 0, |h| h.terrain(|t, host| t.lot_foundation(host, lot).unwrap_or(0)))
}

/// Slot 52 `RemoveFoundation(kind, x, z)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_remove_foundation(p: *mut fc4_lotdev, kind: i32, x: f32, z: f32) -> bool {
    with_handle(p, false, |h| h.terrain(|t, host| t.remove_foundation(host, kind, x, z)))
}

/// Slot 53 `RemoveLotFoundation(x, z)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_remove_lot_foundation(p: *mut fc4_lotdev, x: i32, z: i32) -> bool {
    with_handle(p, false, |h| {
        h.terrain(|t, host| t.remove_lot_foundation_at(host, x, z))
    })
}

/// Slot 54 `AddLotFoundation(lot, cfg, useLotAltitude, level, walls)`
/// (draws R8).
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_add_lot_foundation(
    p: *mut fc4_lotdev,
    lot: u64,
    cfg: u64,
    use_lot_altitude: bool,
    level: bool,
    walls: bool,
) -> bool {
    with_handle(p, false, |h| {
        h.geometry(|t, host, s| t.add_lot_foundation(host, s, lot, opt(cfg), use_lot_altitude, level, walls))
    })
}

/// Slot 55 `AddLotFoundation(lot, type, useLotAltitude, level, walls)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_add_lot_foundation_type(
    p: *mut fc4_lotdev,
    lot: u64,
    foundation_type: u32,
    use_lot_altitude: bool,
    level: bool,
    walls: bool,
) -> bool {
    with_handle(p, false, |h| {
        h.geometry(|t, host, s| {
            t.add_lot_foundation_type(host, s, lot, foundation_type, use_lot_altitude, level, walls)
        })
    })
}

/// Slot 56 `IsLotObjectFoundationRequired(occ)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_is_occupant_foundation_required(p: *const fc4_lotdev, occ: u64) -> bool {
    with_handle(p, false, |h| {
        h.terrain(|t, host| t.is_occupant_foundation_required(host, occ))
    })
}

/// Slot 57 `IsLotObjectFoundationRequired(min, max)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_is_box_foundation_required(
    p: *const fc4_lotdev,
    min3: *const f32,
    max3: *const f32,
) -> bool {
    if min3.is_null() || max3.is_null() {
        return false;
    }
    let a = std::slice::from_raw_parts(min3, 3);
    let b = std::slice::from_raw_parts(max3, 3);
    let (min, max) = ([a[0], a[1], a[2]], [b[0], b[1], b[2]]);
    with_handle(p, false, |h| {
        h.terrain(|t, host| t.is_box_foundation_required(host, min, max))
    })
}

/// Slot 58 `AddBuildingFoundation(building, cfg)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_add_building_foundation(p: *mut fc4_lotdev, building: u64, cfg: u64) -> bool {
    with_handle(p, false, |h| {
        h.terrain(|t, host| t.add_building_foundation(host, building, cfg))
    })
}

/// Slot 59 `AddBuildingFoundation(building, type)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_add_building_foundation_type(
    p: *mut fc4_lotdev,
    building: u64,
    foundation_type: u32,
) -> bool {
    with_handle(p, false, |h| {
        h.terrain(|t, host| t.add_building_foundation_type(host, building, foundation_type))
    })
}

/// Slot 60 `RemoveBuildingFoundation(occ)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_remove_building_foundation(p: *mut fc4_lotdev, occ: u64) -> bool {
    with_handle(p, false, |h| {
        h.terrain(|t, host| t.remove_building_foundation(host, occ))
    })
}

/// Slot 61 `UpdateLotFoundation(lot)` (may draw R8).
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_update_lot_foundation(p: *mut fc4_lotdev, lot: u64) -> bool {
    with_handle(p, false, |h| {
        h.geometry(|t, host, s| t.update_lot_foundation(host, s, lot))
    })
}

// ---- persistence (§13) ------------------------------------------------------------------------------

/// `Write` (§13). References go out as `fc4_ref`s with the building token.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_write(p: *const fc4_lotdev, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    with_handle(p, false, |h| match h.core.try_borrow() {
        Ok(c) => c.write(&mut COut(o)),
        Err(_) => {
            bump(&h.refused);
            false
        }
    })
}

/// `Read` (§13). Runs before `Init`: nothing is sized from the city here.
/// The night deque's buildings arrive with the reference the deque keeps.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_read(p: *mut fc4_lotdev, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_handle(p, false, |h| {
        let host = h.host.clone();
        h.call(false, |c| {
            // References a previous read left in the deque are dropped first.
            for b in c.night().iter().flatten() {
                host.building_release(*b);
            }
            c.read(&mut LIn {
                inner: CIn {
                    stream: i,
                    decode: no_core_objects,
                },
                host: &host,
            })
        })
    })
}

// ---- inspection ---------------------------------------------------------------------------------------

/// The state and counters (`out.struct_size` must be set). False when the
/// core is busy.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_get_state(p: *const fc4_lotdev, out: *mut fc4_lotdev_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    if (o.struct_size as usize) < std::mem::size_of::<fc4_lotdev_state>() {
        return false;
    }
    with_handle(p, false, |h| {
        let Ok(c) = h.core.try_borrow() else {
            return false;
        };
        let k = &h.host.counters;
        let (cx, cz) = c.cell_counts();
        *o = fc4_lotdev_state {
            struct_size: std::mem::size_of::<fc4_lotdev_state>() as u32,
            initialized: u8::from(c.is_initialised()),
            night_mode: c.night_mode().min(255) as u8,
            lit_flag: u8::from(c.lit_flag()),
            force_empty: u8::from(c.force_empty()),
            daily_state: c.daily_state(),
            visited: c.visited(),
            night_len: c.night_len() as u32,
            night_cursor: c.night_cursor() as u32,
            night_acc: c.night_acc(),
            night_batch: c.night_batch(),
            no_zot_until_day: c.no_zot_until_day(),
            can_check_commute: u8::from(c.can_check_commute()),
            zots_allowed: u8::from(c.zots_allowed()),
            net_changed: u8::from(c.net_changed()),
            unit_changed: u8::from(c.unit_changed()),
            verts_x: c.verts_x(),
            extent_x: c.extent_x(),
            extent_z: c.extent_z(),
            cell_count_x: cx,
            cell_count_z: cz,
            daily_work_budget: c.daily_work_budget(),
            lots_visited: k.lots_visited.get(),
            abandoned: k.abandoned.get(),
            restored: k.restored.get(),
            condition_changes: k.condition_changes.get(),
            zot_changes: k.zot_changes.get(),
            constructions_started: k.constructions_started.get(),
            constructions_finished: h.finished.get(),
            demolitions: h.demolitions.get(),
            night_batches: h.night_batches.get(),
            refused_calls: h.refused.get(),
            queued_messages: h.queued.get(),
            reserved: 0,
        };
        true
    })
}

/// The tuning block (§3.5). False when the core is busy.
#[no_mangle]
pub unsafe extern "C" fn fc4_lotdev_get_tuning(p: *const fc4_lotdev, out: *mut fc4_lotdev_tuning) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_handle(p, false, |h| {
        let Ok(c) = h.core.try_borrow() else {
            return false;
        };
        *o = tuning_out(c.tuning());
        true
    })
}

fn tuning_out(t: &Tuning) -> fc4_lotdev_tuning {
    let mut o = fc4_lotdev_tuning {
        stay_new_time: t.stay_new_time,
        attempt_lot_levelling: u8::from(t.attempt_lot_levelling),
        attempt_internal_lot_levelling: u8::from(t.attempt_internal_lot_levelling),
        thresholds: t.thresholds,
        max_cell_altitude_delta_for_flatness: t.max_cell_altitude_delta_for_flatness,
        max_slope_for_flatness: t.max_slope_for_flatness,
        default_lot_foundation_type: t.default_lot_foundation_type,
        default_building_foundation_type: t.default_building_foundation_type,
        auto_level_internal_altitude_delta: t.auto_level_internal_altitude_delta,
        max_area_slope_for_levelling: t.max_area_slope_for_levelling,
        cost_curve_points: 0,
        cost_curve: [0.0; 32],
    };
    let pts = t.cost_curve.points();
    let n = pts.len().min(16);
    for (i, (x, y)) in pts.iter().take(n).enumerate() {
        o.cost_curve[2 * i] = *x;
        o.cost_curve[2 * i + 1] = *y;
    }
    o.cost_curve_points = n as u32;
    o
}
