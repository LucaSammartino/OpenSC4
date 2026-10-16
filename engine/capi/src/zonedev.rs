// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The zone developer / lot manager's C ABI (fc4.h "zone developer / lot
//! manager", docs/specs/zones.md §1.2, §3.2, §5.2, §7–§10.2, §11.2, §11.3).
//!
//! The host implements [`fc4_zonedev_host`]: every `ZonesHost` method the
//! zone developer reaches (the zone manager's methods are neutral here), the
//! maximum-capacity grid through the shared [`fc4_grid_api`], and three
//! calls that mirror the master lot list (`master_insert`, `master_remove`,
//! `master_reset`) so the plugin can keep the game's raw list and safe lot
//! iterators (lot-developer.md §15.1). The plugin's object (class
//! `0x498F9B01`: `cISC4ZoneDeveloper`, `cISC4LotManager` at +4, the message
//! target at +8, `cIGZSerializable` at +0xC) forwards its slots to the
//! `fc4_zonedev_*` functions.
//!
//! Tokens are opaque host identities (lots, lot configurations, occupants,
//! growth developers, the grid); 0 is null. Tokens the host returns are
//! **borrowed** unless a callback says otherwise; tokens the lot list
//! functions hand out carry one reference each for the caller, as the game's
//! slots do (§8.3).
//!
//! # The generator (§9)
//!
//! The zone developer's file-local `cRZRandom` is shared by every instance
//! of a process: the first `fc4_zonedev_create` seeds it with
//! `derive_seed(session_seed, 0x8C09C68A)` (foundations §6.3); later objects
//! share it and their seed argument is not used.
//!
//! # Re-entrancy
//!
//! The core holds no borrow across a host call, so a re-entrant call (an
//! InsertOccupant delivered while a street is placed, a lot list asked for by
//! the game inside a lot's `Init`) runs at once; such calls are counted
//! (`fc4_zonedev_state::reentrant_calls`).
//!
//! # Shadow mode
//!
//! [`fc4_zonedev_create_shadow`] makes a core whose host table reads the game
//! but **never writes it**: no subscription, service, reference, lot call,
//! network tool, levelling, message or master-list mirror reaches the host,
//! and no grid is created. References read from a save are borrowed (the
//! duplicate sightings of `Read` are not released). It answers the lot
//! manager's lookups from the record it read.

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::grid::{CellRect, GridProvider, TractGrid};
use fc4_core::property::PropertyHolder;
use fc4_core::rng::FixedSeeds;
use fc4_core::stream::{InStream, RefValue, SaveObject};
use fc4_core::zones::{
    developer_answers_iid, BuildingProfile, CellRegion, Component, GrowthItem, InsertionPoint, LotObject, Position,
    Service, SharedZoneStatics, ZoneDeveloper, ZoneDeveloperStatics, ZonesHost, ZonesMessage, CLSID_ZONE_DEVELOPER,
    IID_LOT,
};

use crate::budget::fc4_token_callback;
use crate::demand::fc4_u32_callback;
use crate::development::{fc4_growth_item, fc4_growth_items_sink};
use crate::guard;
use crate::hostgrid::{collect_tokens, create_grid, fc4_cell_rect, fc4_grid_api, grid_token};
use crate::stream::{fc4_istream, fc4_ostream, fc4_ref, istream_ok, ostream_ok, CIn, COut};
use crate::zones::{fc4_zone_region, region_in, region_out};

type U = *mut c_void;

/// What `building_profile` reads of BDS slot 39 (fc4.h
/// `fc4_zonedev_profile`, §7.2.1 step 6).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_zonedev_profile {
    pub purpose: u32,
    pub wealth: u32,
    pub capacity: u16,
    pub reserved: [u8; 2],
}

/// What one parcellization or message did (fc4.h
/// `fc4_zonedev_parcel_result`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_zonedev_parcel_result {
    pub struct_size: u32,
    /// Lots entered into the master list (`CreateLot`).
    pub lots_created: u32,
    /// Lots removed from the master list (`DeleteLot`, real mode).
    pub lots_deleted: u32,
    /// `CreateLot` calls the host could not construct a lot for.
    pub lot_failures: u32,
    /// Street `Place` calls that returned true / false (§7.10).
    pub streets_placed: u32,
    pub streets_failed: u32,
    /// `DemolishRegion` calls (§7.9).
    pub demolish_regions: u32,
    /// Game writes swallowed (shadow mode; 0 in live mode).
    pub swallowed: u32,
}

/// Diagnostics and the getters' snapshot (fc4.h `fc4_zonedev_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_zonedev_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub loaded: u8,
    pub real_mode: u8,
    pub reserved: u8,
    /// Slot 16.
    pub lot_count: u32,
    /// The cell → lot map's rows (x) and row size (z).
    pub map_x: i32,
    pub map_z: i32,
    /// Since creation: master-list inserts and removals.
    pub lots_created: u32,
    pub lots_deleted: u32,
    /// `fc4_zonedev_do_parcellization` calls; of them, dezones (type not RCI).
    pub parcellizations: u32,
    pub dezones: u32,
    pub reentrant_calls: u32,
    /// The value of the last message `0xA6B79602` (§5.2).
    pub view_value: u32,
    /// Bit 0 alternateOrientation, bit 1 placeStreets, bit 2 customZoneSize.
    pub options: u32,
    /// The maximum-capacity grid token (slot 27, borrowed); 0 = none.
    pub max_capacity_grid: u64,
}

/// One lot configuration object record (fc4.h `fc4_zonedev_cfg_object`,
/// lot configuration slot 23, §8.4.1).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_zonedev_cfg_object {
    pub kind: u8,
    pub orientation: u8,
    pub reserved: [u8; 2],
    pub min_x: f32,
    pub min_z: f32,
    pub max_x: f32,
    pub max_z: f32,
    /// `values[0..n_values]`, borrowed for the call.
    pub values: *const u32,
    pub n_values: usize,
}

pub type fc4_zonedev_cfg_object_callback =
    Option<unsafe extern "C" fn(ctx: *mut c_void, obj: *const fc4_zonedev_cfg_object)>;

/// Everything the zone developer asks of the game (fc4.h
/// `fc4_zonedev_host`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_zonedev_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    /// The maximum-capacity grid (uint16 `0x49B9E604`, §1.2 step 4).
    pub grids: fc4_grid_api,
    // lifecycle, services and messages (§1.2, §5)
    /// `Init` steps 1–2 without the 3D view: the lot developer, BDS, city,
    /// occupant manager, terrain, traffic simulator, zone manager and message
    /// server. False if any is missing.
    pub acquire_services: Option<unsafe extern "C" fn(U) -> bool>,
    pub release_services: Option<unsafe extern "C" fn(U)>,
    pub subscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, u32)>,
    /// City slots 93, 94.
    pub cell_counts: Option<unsafe extern "C" fn(U, *mut i32, *mut i32)>,
    /// PostCityInit (§5.2): the street, dirt-road and road network tools.
    pub acquire_network_tools: Option<unsafe extern "C" fn(U)>,
    pub release_network_tools: Option<unsafe extern "C" fn(U)>,
    /// DeferOccupantOperations `0x49D968C4` (data1 = begin), posted (§5.3).
    pub post_defer_occupant_operations: Option<unsafe extern "C" fn(U, bool)>,
    /// Zone manager slot 12 `GetZoneType(x, z)`.
    pub zone_type: Option<unsafe extern "C" fn(U, i32, i32) -> i32>,
    /// Demolition slot 6 `DemolishRegion(1, &region, 1, 1, 0, …)` (§7.9).
    pub demolish_region: Option<unsafe extern "C" fn(U, *const fc4_zone_region)>,
    // networks (§7.10–§7.13, §8.8)
    /// The street tool's `PlaceNetworkOccupants(a, b, 0xC49, tool, realMode,
    /// −1)` for network kind `kind` (3); AL.
    pub street_place: Option<unsafe extern "C" fn(U, u32, i32, i32, i32, i32, bool) -> bool>,
    /// Network tool slot 5 `Reset()` of tool `kind` (3 street, 0 road).
    pub reset_network_tool: Option<unsafe extern "C" fn(U, u32)>,
    /// Network occupant slot 47 `ConnectsToNeighbor(edge, type)`.
    pub connects_to_neighbor: Option<unsafe extern "C" fn(U, u64, i32, u32) -> bool>,
    /// Network tool `kind` (3 or 11) slot 9 `OccupantExists(x, z, flag,
    /// mask)`: true with `mask[0..4]` written, or false (mask untouched).
    pub network_tool_occupant_exists: Option<unsafe extern "C" fn(U, u32, i32, i32, u32, *mut u8) -> bool>,
    /// Not called since ABI 55 (the joins are the core's, §5.2.1–§5.2.2).
    pub street_joining_helper: Option<unsafe extern "C" fn(U, u64, u32)>,
    // terrain
    pub cell_has_water: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    /// Terrain slot 13 `GetAltitude(x, z)` (ST0).
    pub terrain_altitude: Option<unsafe extern "C" fn(U, f32, f32) -> f32>,
    /// Terrain slot 14 `GetAltitudeAtVertex(vx, vz)` (ST0).
    pub terrain_vertex_altitude: Option<unsafe extern "C" fn(U, i32, i32) -> f32>,
    // city and occupants
    /// City slot 98 `LocationIsInBounds(pos)`.
    pub location_in_bounds: Option<unsafe extern "C" fn(U, *const f32) -> bool>,
    /// City slot 95 `PositionToCell(pos, &x, &z)`.
    pub position_to_cell: Option<unsafe extern "C" fn(U, *const f32, *mut i32, *mut i32) -> bool>,
    /// Occupant manager slot 41 (+ 48): the cell's occupants (of `type` when
    /// `filtered`), borrowed, `cb(ctx, occupant)` each.
    pub cell_occupants: Option<unsafe extern "C" fn(U, i32, i32, bool, u32, fc4_token_callback, U)>,
    /// Occupant slot 7 `GetType`.
    pub occupant_type: Option<unsafe extern "C" fn(U, u64) -> u32>,
    /// Occupant slot 8 `GetPosition` (x, y, z).
    pub occupant_position: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    /// `HasAnyNetworkFlag(flags)`: 1 / 0, or −1 when not a network occupant.
    pub network_has_any_flag: Option<unsafe extern "C" fn(U, u64, u32) -> i32>,
    /// Occupant manager slot 21 `RemoveOccupant(occupant, 1, 0)`.
    pub remove_occupant: Option<unsafe extern "C" fn(U, u64)>,
    // references
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    // lots (cISC4Lot)
    /// A new game lot `(x, z, w, h, facing)` with no reference yet; 0 when
    /// the host cannot construct one (`CreateLot` then fails).
    pub new_lot: Option<unsafe extern "C" fn(U, i32, i32, i32, i32, i32) -> u64>,
    pub lot_init: Option<unsafe extern "C" fn(U, u64)>,
    pub lot_shutdown: Option<unsafe extern "C" fn(U, u64)>,
    pub lot_post_city_init: Option<unsafe extern "C" fn(U, u64)>,
    /// Slot 9, borrowed; 0 = none.
    pub lot_configuration: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub lot_flags: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub lot_bounding_rect: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    pub lot_altitude: Option<unsafe extern "C" fn(U, u64) -> f32>,
    pub lot_location: Option<unsafe extern "C" fn(U, u64, *mut i32, *mut i32)>,
    pub lot_facing: Option<unsafe extern "C" fn(U, u64) -> i32>,
    pub lot_has_business_unit: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub lot_zone_type: Option<unsafe extern "C" fn(U, u64) -> i32>,
    pub lot_state: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub lot_set_state: Option<unsafe extern "C" fn(U, u64, u32, u32, u32)>,
    pub lot_set_vertices_locked: Option<unsafe extern "C" fn(U, u64, bool)>,
    // lot configurations
    pub cfg_compatible_with_water: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub cfg_compatible_with_zone: Option<unsafe extern "C" fn(U, u64, i32) -> bool>,
    pub cfg_id: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub cfg_size: Option<unsafe extern "C" fn(U, u64, *mut u32, *mut u32)>,
    pub cfg_growth_stage: Option<unsafe extern "C" fn(U, u64) -> u8>,
    pub cfg_compatible_with_wealth: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub cfg_compatible_with_purpose: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    /// Slot 24's object: its IDs, `cb(ctx, id)` each; false = null object.
    pub cfg_primary_building_ids: Option<unsafe extern "C" fn(U, u64, fc4_u32_callback, U) -> bool>,
    /// Every configuration of the lot configuration manager (slot 9, the
    /// unregistered filter), borrowed, in its order.
    pub lot_configurations: Option<unsafe extern "C" fn(U, fc4_token_callback, U)>,
    // lot developer
    pub attempt_area_levelling: Option<unsafe extern "C" fn(U, *const fc4_cell_rect)>,
    pub remove_building_zot_prop: Option<unsafe extern "C" fn(U, u64)>,
    /// Slot 41: the occupant with ONE reference for the caller; 0 = none.
    pub lot_base_texture_occupant: Option<unsafe extern "C" fn(U, u64) -> u64>,
    pub remove_lot_from_business_unit: Option<unsafe extern "C" fn(U, u64)>,
    pub set_empty_lot_base_texture: Option<unsafe extern "C" fn(U, u64)>,
    // traffic simulator (§8.5 step 4)
    /// Slot 22: false = a null iterator; else `*node` (0: the end / head) and
    /// `*list` (whether its list is set).
    pub desired_lot_insertion_point: Option<unsafe extern "C" fn(U, *mut u64, *mut bool) -> bool>,
    /// Move that iterator to the node of lot `node` (0: the list head).
    pub set_lot_insertion_point: Option<unsafe extern "C" fn(U, u64)>,
    // BDS and growth developers (§7.2.1)
    /// BDS slot 32 `GetDeveloper(type)`, borrowed; 0 = none.
    pub bds_developer: Option<unsafe extern "C" fn(U, u32) -> u64>,
    pub building_types_by_family: Option<unsafe extern "C" fn(U, u32, fc4_u32_callback, U)>,
    /// BDS slot 39; false = no profile (zeros).
    pub building_profile: Option<unsafe extern "C" fn(U, u32, *mut fc4_zonedev_profile) -> bool>,
    pub dev_estimate_desirability: Option<unsafe extern "C" fn(U, u64, i32, i32) -> i16>,
    /// `GetSourceDemand().QueryActiveDemandValue()`; false = no demand.
    pub dev_source_demand: Option<unsafe extern "C" fn(U, u64, *mut f32) -> bool>,
    /// The growth list in one `sink` call when slot 10 says it is valid;
    /// false = not valid.
    pub dev_growth_list: Option<unsafe extern "C" fn(U, u64, fc4_growth_items_sink, U) -> bool>,
    pub dev_desirability_value: Option<unsafe extern "C" fn(U, u64, i32, i32) -> i16>,
    pub dev_desirability_shift: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub dev_purpose: Option<unsafe extern "C" fn(U, u64) -> u32>,
    pub dev_wealth: Option<unsafe extern "C" fn(U, u64) -> u32>,
    // the master lot list mirror (§8.1, lot-developer.md §15.1)
    /// `lot` inserted before `before` (0: appended).
    pub master_insert: Option<unsafe extern "C" fn(U, u64, u64)>,
    pub master_remove: Option<unsafe extern "C" fn(U, u64)>,
    /// The list replaced by `lots[0..n]` (`Read`; empty at `Shutdown`).
    pub master_reset: Option<unsafe extern "C" fn(U, *const u64, usize)>,
    // ABI 55 (re-specs11)
    /// Lot developer slot 29 `GetAreaSlope(rect, withWater)` (ST0).
    pub area_slope: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, bool) -> f32>,
    /// Lot configuration slot 8 `GetMaximumSlope` (ST0).
    pub cfg_max_slope: Option<unsafe extern "C" fn(U, u64) -> f32>,
    /// Lot configuration slot 23: `cb(ctx, obj)` once per record, in order.
    pub cfg_objects: Option<unsafe extern "C" fn(U, u64, fc4_zonedev_cfg_object_callback, U)>,
    /// Network occupant slot 12 `GetNetworkFlags`.
    pub network_flags: Option<unsafe extern "C" fn(U, u64) -> u32>,
    /// Network occupant slot 8 (every bit of `mask`).
    pub network_has_all_flags: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    /// Network occupant slot 38 `GetEdgeTypesFlag(type)`.
    pub network_edge_mask: Option<unsafe extern "C" fn(U, u64, u32) -> u32>,
    /// Network occupant slot 22 `IsOfType(type)`: 1 / 0, or −1 when not a
    /// network occupant.
    pub network_is_of_type: Option<unsafe extern "C" fn(U, u64, u32) -> i32>,
    /// Network occupant slot 17 (the flag `0x400000` test).
    pub network_is_flag_0x400000: Option<unsafe extern "C" fn(U, u64) -> bool>,
    /// Occupant slot 11 `GetBoundingCityCells`.
    pub occupant_bounding_cells: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    /// Whether the network tool of `kind` (0, 3, 11) is held.
    pub network_tool_present: Option<unsafe extern "C" fn(U, u32) -> bool>,
    /// Tool `kind`'s `PlaceNetworkOccupants(a, b, 0, null, 1, −1)`.
    pub network_tool_join: Option<unsafe extern "C" fn(U, u32, i32, i32, i32, i32)>,
    /// Terrain slot 35 `CellHasLand`.
    pub cell_has_land: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    /// City slot 99 `CellIsInBounds`.
    pub cell_is_in_bounds: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    /// Surface water slot 8 `IsNavigable(x, z, 0, null)`.
    pub is_navigable: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    /// Lot slot 32 `UpdateZoneType`.
    pub lot_update_zone_type: Option<unsafe extern "C" fn(U, u64)>,
    /// Lot developer slot 46 `GetForceEmptyLotBaseTextures`.
    pub force_empty_lot_base_textures: Option<unsafe extern "C" fn(U) -> bool>,
    /// Zone manager slot 11 `PlaceZone(region, 0, 1, 1, 0, 1, 0, null, null,
    /// null)` (§7.2.2 step 3).
    pub zone_manager_dezone: Option<unsafe extern "C" fn(U, *const fc4_zone_region)>,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_zonedev_profile>() == 12);
    assert!(std::mem::size_of::<fc4_zonedev_parcel_result>() == 32);
    assert!(std::mem::size_of::<fc4_zonedev_state>() == 56);
    assert!(std::mem::size_of::<fc4_zonedev_host>() == if cfg!(target_pointer_width = "32") { 400 } else { 800 });
    assert!(std::mem::size_of::<fc4_zonedev_cfg_object>() == if cfg!(target_pointer_width = "32") { 28 } else { 40 });
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

/// What the core did during one top-level call; nested calls keep their own.
#[derive(Clone, Copy, Debug, Default)]
struct Trace {
    created: u32,
    deleted: u32,
    lot_failures: u32,
    streets_placed: u32,
    streets_failed: u32,
    demolish_regions: u32,
    swallowed: u32,
}

impl Trace {
    fn result(&self) -> fc4_zonedev_parcel_result {
        fc4_zonedev_parcel_result {
            struct_size: std::mem::size_of::<fc4_zonedev_parcel_result>() as u32,
            lots_created: self.created,
            lots_deleted: self.deleted,
            lot_failures: self.lot_failures,
            streets_placed: self.streets_placed,
            streets_failed: self.streets_failed,
            demolish_regions: self.demolish_regions,
            swallowed: self.swallowed,
        }
    }
}

struct CHost {
    c: fc4_zonedev_host,
    shadow: bool,
    trace: Cell<Trace>,
    /// Since creation.
    created: Cell<u32>,
    deleted: Cell<u32>,
}

impl CHost {
    fn new(c: fc4_zonedev_host, shadow: bool) -> CHost {
        CHost {
            c,
            shadow,
            trace: Cell::new(Trace::default()),
            created: Cell::new(0),
            deleted: Cell::new(0),
        }
    }

    fn note(&self, f: impl FnOnce(&mut Trace)) {
        let mut t = self.trace.get();
        f(&mut t);
        self.trace.set(t);
    }

    fn swallow(&self) {
        self.note(|t| t.swallowed = t.swallowed.wrapping_add(1));
    }

    fn rect_of(&self, f: impl FnOnce(*mut fc4_cell_rect)) -> CellRect {
        let mut r = fc4_cell_rect::default();
        f(&mut r);
        r.into()
    }
}

unsafe extern "C" fn push_u32(ctx: *mut c_void, v: u32) {
    if let Some(out) = (ctx as *mut Vec<u32>).as_mut() {
        out.push(v);
    }
}

unsafe extern "C" fn push_cfg_object(ctx: *mut c_void, obj: *const fc4_zonedev_cfg_object) {
    let (Some(out), Some(o)) = ((ctx as *mut Vec<LotObject>).as_mut(), obj.as_ref()) else {
        return;
    };
    let values = if o.values.is_null() || o.n_values == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(o.values, o.n_values).to_vec()
    };
    out.push(LotObject {
        kind: o.kind,
        orientation: o.orientation,
        min_x: o.min_x,
        min_z: o.min_z,
        max_x: o.max_x,
        max_z: o.max_z,
        values,
    });
}

unsafe extern "C" fn take_growth_items(ctx: *mut c_void, items: *const fc4_growth_item, n: usize) {
    let Some(out) = (ctx as *mut Vec<GrowthItem>).as_mut() else {
        return;
    };
    if items.is_null() || n == 0 {
        return;
    }
    for it in std::slice::from_raw_parts(items, n) {
        out.push(GrowthItem {
            x: it.x,
            z: it.z,
            max_stage: it.max_stage,
        });
    }
}

/// A core region for a host call: the mask bytes and the struct pointing at
/// them (keep the bytes alive while the struct is used).
fn region_raw(r: &CellRegion) -> (Vec<u8>, fc4_zone_region) {
    let (w, h) = (r.width(), r.height());
    let mut bytes = vec![0u8; (w.max(0) as usize) * (h.max(0) as usize)];
    for (x, z) in r.cells() {
        let (dx, dz) = (x.wrapping_sub(r.rect.x1), z.wrapping_sub(r.rect.z1));
        bytes[dz as usize * w as usize + dx as usize] = 1;
    }
    let raw = fc4_zone_region {
        x0: r.rect.x1,
        z0: r.rect.z1,
        x1: r.rect.x1.wrapping_add(w - 1),
        z1: r.rect.z1.wrapping_add(h - 1),
        mask: bytes.as_mut_ptr(),
        len: bytes.len(),
    };
    (bytes, raw)
}

impl GridProvider<i8> for CHost {
    /// The zone grid belongs to the zone manager.
    fn create_grid(&self, _instance: u32, _tract_size: i32) -> Option<Rc<dyn TractGrid<i8>>> {
        None
    }
    fn resolve_grid(&self, _r: &RefValue) -> Option<Rc<dyn TractGrid<i8>>> {
        None
    }
}

impl GridProvider<u16> for CHost {
    /// `Init` step 4: the maximum-capacity grid, shut down and released when
    /// the core drops it (`Shutdown`). None in shadow mode.
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u16>>> {
        if self.shadow {
            self.swallow();
            return None;
        }
        create_grid::<u16>(&self.c.grids, instance, tract_size, true)
    }
    /// Not saved (§10.2).
    fn resolve_grid(&self, _r: &RefValue) -> Option<Rc<dyn TractGrid<u16>>> {
        None
    }
}

impl ZonesHost for CHost {
    // ---- lifecycle, services and messages ----
    fn manager_acquire_services(&self) {}
    fn manager_release_services(&self) {}
    fn developer_acquire_services(&self) -> bool {
        if self.shadow {
            self.swallow();
            return true;
        }
        cb!(self, acquire_services, false)
    }
    fn developer_release_services(&self) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, release_services, ())
    }
    fn subscribe(&self, who: Component, messages: &[u32]) {
        if who != Component::ZoneDeveloper {
            return;
        }
        if self.shadow {
            return self.swallow();
        }
        for &m in messages {
            cb!(self, subscribe, (), m)
        }
    }
    fn unsubscribe(&self, who: Component, messages: &[u32]) {
        if who != Component::ZoneDeveloper {
            return;
        }
        if self.shadow {
            return self.swallow();
        }
        for &m in messages {
            cb!(self, unsubscribe, (), m)
        }
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        None
    }
    fn city_cell_counts(&self) -> (i32, i32) {
        let (mut x, mut z) = (0i32, 0i32);
        cb!(self, cell_counts, (), &mut x, &mut z);
        (x, z)
    }
    fn service_ref(&self, _service: Service) -> RefValue {
        RefValue::Null
    }
    /// The grid is shut down and released when the core drops it.
    fn shutdown_grid(&self, _class: u32) {}
    fn acquire_network_tools(&self) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, acquire_network_tools, ())
    }
    fn release_network_tools(&self) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, release_network_tools, ())
    }
    fn post_zoning_change(&self, _region: CellRegion, _zone: i32, _check_slope: bool) {}
    fn post_defer_occupant_operations(&self, begin: bool) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, post_defer_occupant_operations, (), begin)
    }

    // ---- the other class ----
    fn zone_type(&self, x: i32, z: i32) -> i32 {
        cb!(self, zone_type, 0, x, z)
    }
    fn lot_manager_get_lot(&self, _x: i32, _z: i32) -> Option<u64> {
        None
    }
    fn lot_manager_lots_in_cell_rect(&self, _rect: CellRect, _origin_only: bool) -> Vec<u64> {
        Vec::new()
    }
    fn zone_developer_do_parcellization(&self, _region: &CellRegion, _zone: i32, _flag: bool) {}

    // ---- budget, demolition, landfill ----
    fn total_funds(&self) -> i64 {
        0
    }
    fn withdraw_funds(&self, _amount: i64) -> bool {
        false
    }
    fn demolish_lots(&self, _commit: bool, _lots: &[u64], _check_slope: bool, _flags: u32) -> i64 {
        0
    }
    fn demolish_region(&self, region: &CellRegion) {
        self.note(|t| t.demolish_regions = t.demolish_regions.wrapping_add(1));
        if self.shadow {
            return self.swallow();
        }
        let (_bytes, raw) = region_raw(region);
        cb!(self, demolish_region, (), &raw)
    }
    fn landfill_zoning_cost(&self, _region: &CellRegion) -> i64 {
        0
    }

    // ---- networks ----
    fn find_network_tile(&self, _x: i32, _z: i32) -> Option<u64> {
        None
    }
    fn tile_occupant_flag_set(&self, _tile: u64, _flag: u32) -> bool {
        false
    }
    fn tile_is_of_type(&self, _tile: u64, _t: u32) -> bool {
        false
    }
    fn tile_demolition_cost(&self, _tile: u64) -> i64 {
        0
    }
    fn street_place(&self, kind: u32, a: (i32, i32), b: (i32, i32), real_mode: bool) -> bool {
        if self.shadow {
            self.swallow();
            return false;
        }
        let ok = cb!(self, street_place, false, kind, a.0, a.1, b.0, b.1, real_mode);
        self.note(|t| {
            if ok {
                t.streets_placed = t.streets_placed.wrapping_add(1);
            } else {
                t.streets_failed = t.streets_failed.wrapping_add(1);
            }
        });
        ok
    }
    fn reset_network_tool(&self, tool_kind: u32) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, reset_network_tool, (), tool_kind)
    }
    fn connects_to_neighbor(&self, occupant: u64, edge: i32, net_type: u32) -> bool {
        cb!(self, connects_to_neighbor, false, occupant, edge, net_type)
    }
    fn network_tool_occupant_exists(&self, tool_kind: u32, x: i32, z: i32, flag: u32) -> Option<[u8; 4]> {
        let mut mask = [0u8; 4];
        let found = cb!(
            self,
            network_tool_occupant_exists,
            false,
            tool_kind,
            x,
            z,
            flag,
            mask.as_mut_ptr()
        );
        found.then_some(mask)
    }
    fn street_joining_helper(&self, occupant: u64, data4: u32) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, street_joining_helper, (), occupant, data4)
    }
    fn network_flags(&self, occupant: u64) -> u32 {
        cb!(self, network_flags, 0, occupant)
    }
    fn network_has_all_flags(&self, occupant: u64, mask: u32) -> bool {
        cb!(self, network_has_all_flags, false, occupant, mask)
    }
    fn network_edge_mask(&self, occupant: u64, net_type: u32) -> u32 {
        cb!(self, network_edge_mask, 0, occupant, net_type)
    }
    fn network_is_of_type(&self, occupant: u64, net_type: u32) -> Option<bool> {
        match cb!(self, network_is_of_type, -1, occupant, net_type) {
            v if v < 0 => None,
            v => Some(v != 0),
        }
    }
    fn network_is_flag_0x400000(&self, occupant: u64) -> bool {
        cb!(self, network_is_flag_0x400000, false, occupant)
    }
    fn network_tool_present(&self, kind: u32) -> bool {
        cb!(self, network_tool_present, false, kind)
    }
    fn network_tool_join(&self, kind: u32, a: (i32, i32), b: (i32, i32)) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, network_tool_join, (), kind, a.0, a.1, b.0, b.1)
    }

    // ---- terrain ----
    fn cell_has_water(&self, x: i32, z: i32) -> bool {
        cb!(self, cell_has_water, false, x, z)
    }
    fn terrain_altitude(&self, x: f32, z: f32) -> f32 {
        cb!(self, terrain_altitude, 0.0, x, z)
    }
    fn terrain_vertex_altitude(&self, vx: i32, vz: i32) -> f32 {
        cb!(self, terrain_vertex_altitude, 0.0, vx, vz)
    }
    fn cell_has_land(&self, x: i32, z: i32) -> bool {
        cb!(self, cell_has_land, true, x, z)
    }
    fn cell_is_in_bounds(&self, x: i32, z: i32) -> bool {
        match self.c.cell_is_in_bounds {
            // SAFETY: valid callback.
            Some(f) => unsafe { f(self.c.user, x, z) },
            None => {
                let (cx, cz) = self.city_cell_counts();
                (0..cx).contains(&x) && (0..cz).contains(&z)
            }
        }
    }
    fn is_navigable(&self, x: i32, z: i32) -> bool {
        cb!(self, is_navigable, false, x, z)
    }

    // ---- city and occupants ----
    fn location_in_bounds(&self, pos: Position) -> bool {
        cb!(self, location_in_bounds, false, pos.as_ptr())
    }
    fn position_to_cell(&self, pos: Position) -> Option<(i32, i32)> {
        let (mut x, mut z) = (0i32, 0i32);
        cb!(self, position_to_cell, false, pos.as_ptr(), &mut x, &mut z).then_some((x, z))
    }
    fn cell_occupants(&self, x: i32, z: i32, type_filter: Option<u32>) -> Vec<u64> {
        let Some(f) = self.c.cell_occupants else {
            return Vec::new();
        };
        let (filtered, t) = match type_filter {
            Some(t) => (true, t),
            None => (false, 0),
        };
        // SAFETY: valid callback.
        collect_tokens(|cb, ctx| unsafe { f(self.c.user, x, z, filtered, t, cb, ctx) })
            .into_iter()
            .filter(|&t| t != 0)
            .collect()
    }
    fn landfill_at(&self, _x: i32, _z: i32) -> Option<u64> {
        None
    }
    fn occupant_type(&self, occupant: u64) -> u32 {
        cb!(self, occupant_type, 0, occupant)
    }
    fn occupant_position(&self, occupant: u64) -> Position {
        let mut p = [0.0f32; 3];
        cb!(self, occupant_position, (), occupant, p.as_mut_ptr());
        p
    }
    fn occupant_bounding_cells(&self, occupant: u64) -> CellRect {
        if self.c.occupant_bounding_cells.is_none() {
            return CellRect::new(0, 0, -1, -1);
        }
        self.rect_of(|out| cb!(self, occupant_bounding_cells, (), occupant, out))
    }
    fn occupant_can_remove(&self, _occupant: u64, _flags: u32) -> bool {
        false
    }
    fn network_has_any_flag(&self, occupant: u64, flags: u32) -> Option<bool> {
        match cb!(self, network_has_any_flag, -1, occupant, flags) {
            v if v < 0 => None,
            v => Some(v != 0),
        }
    }
    fn remove_occupant(&self, occupant: u64) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, remove_occupant, (), occupant)
    }

    // ---- reference counts (none in shadow mode: every token is borrowed) ----
    fn add_ref(&self, object: u64) {
        if object != 0 && !self.shadow {
            cb!(self, add_ref, (), object)
        }
    }
    fn release(&self, object: u64) {
        if object != 0 && !self.shadow {
            cb!(self, release, (), object)
        }
    }

    // ---- lots ----
    fn new_lot(&self, x: i32, z: i32, w: i32, h: i32, facing: i32) -> u64 {
        if self.shadow {
            self.swallow();
            return 0;
        }
        let lot = cb!(self, new_lot, 0, x, z, w, h, facing);
        if lot == 0 {
            self.note(|t| t.lot_failures = t.lot_failures.wrapping_add(1));
        }
        lot
    }
    fn lot_init(&self, lot: u64) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, lot_init, (), lot)
    }
    fn lot_shutdown(&self, lot: u64) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, lot_shutdown, (), lot)
    }
    fn lot_post_city_init(&self, lot: u64) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, lot_post_city_init, (), lot)
    }
    fn lot_configuration(&self, lot: u64) -> Option<u64> {
        opt(cb!(self, lot_configuration, 0, lot))
    }
    fn lot_flags(&self, lot: u64) -> u32 {
        cb!(self, lot_flags, 0, lot)
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        self.rect_of(|out| cb!(self, lot_bounding_rect, (), lot, out))
    }
    fn lot_altitude(&self, lot: u64) -> f32 {
        cb!(self, lot_altitude, 0.0, lot)
    }
    fn lot_location(&self, lot: u64) -> (i32, i32) {
        let (mut x, mut z) = (0i32, 0i32);
        cb!(self, lot_location, (), lot, &mut x, &mut z);
        (x, z)
    }
    fn lot_facing(&self, lot: u64) -> i32 {
        cb!(self, lot_facing, 0, lot)
    }
    fn lot_has_business_unit(&self, lot: u64) -> bool {
        cb!(self, lot_has_business_unit, false, lot)
    }
    fn lot_zone_type(&self, lot: u64) -> i32 {
        cb!(self, lot_zone_type, 0, lot)
    }
    fn lot_state(&self, lot: u64) -> u32 {
        cb!(self, lot_state, 0, lot)
    }
    fn lot_set_state(&self, lot: u64, state: u32, a: u32, b: u32) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, lot_set_state, (), lot, state, a, b)
    }
    fn lot_building(&self, _lot: u64) -> Option<u64> {
        None
    }
    fn lot_set_vertices_locked(&self, lot: u64, locked: bool) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, lot_set_vertices_locked, (), lot, locked)
    }
    fn lot_update_zone_type(&self, lot: u64) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, lot_update_zone_type, (), lot)
    }
    /// The save writes the lot by reference (`WriteGZSerializable`, §10.2).
    fn lot_ref(&self, lot: u64) -> RefValue {
        RefValue::Host {
            token: lot,
            class_id: 0,
        }
    }
    fn lot_from_ref(&self, r: &RefValue) -> Option<u64> {
        match *r {
            RefValue::Host { token, .. } => opt(token),
            _ => None,
        }
    }

    // ---- lot configurations ----
    fn cfg_compatible_with_water(&self, cfg: u64) -> bool {
        cb!(self, cfg_compatible_with_water, false, cfg)
    }
    fn cfg_compatible_with_zone(&self, cfg: u64, zone: i32) -> bool {
        cb!(self, cfg_compatible_with_zone, false, cfg, zone)
    }
    fn cfg_id(&self, cfg: u64) -> u32 {
        cb!(self, cfg_id, 0, cfg)
    }
    fn cfg_size(&self, cfg: u64) -> (u32, u32) {
        let (mut x, mut z) = (0u32, 0u32);
        cb!(self, cfg_size, (), cfg, &mut x, &mut z);
        (x, z)
    }
    fn cfg_growth_stage(&self, cfg: u64) -> u8 {
        cb!(self, cfg_growth_stage, 0, cfg)
    }
    fn cfg_compatible_with_wealth(&self, cfg: u64, wealth: u32) -> bool {
        cb!(self, cfg_compatible_with_wealth, false, cfg, wealth)
    }
    fn cfg_compatible_with_purpose(&self, cfg: u64, purpose: u32) -> bool {
        cb!(self, cfg_compatible_with_purpose, false, cfg, purpose)
    }
    fn cfg_primary_building_ids(&self, cfg: u64) -> Option<Vec<u32>> {
        let f = self.c.cfg_primary_building_ids?;
        let mut ids: Vec<u32> = Vec::new();
        // SAFETY: valid callback; `ids` outlives the call.
        let present = unsafe {
            f(
                self.c.user,
                cfg,
                Some(push_u32),
                &mut ids as *mut Vec<u32> as *mut c_void,
            )
        };
        present.then_some(ids)
    }
    fn cfg_max_slope(&self, cfg: u64) -> f32 {
        cb!(self, cfg_max_slope, f32::MAX, cfg)
    }
    fn cfg_objects(&self, cfg: u64) -> Vec<LotObject> {
        let Some(f) = self.c.cfg_objects else {
            return Vec::new();
        };
        let mut v: Vec<LotObject> = Vec::new();
        // SAFETY: valid callback; `v` outlives the call.
        unsafe {
            f(
                self.c.user,
                cfg,
                Some(push_cfg_object),
                &mut v as *mut Vec<LotObject> as *mut c_void,
            )
        };
        v
    }
    fn lot_configurations(&self) -> Vec<u64> {
        let Some(f) = self.c.lot_configurations else {
            return Vec::new();
        };
        // SAFETY: valid callback.
        collect_tokens(|cb, ctx| unsafe { f(self.c.user, cb, ctx) })
            .into_iter()
            .filter(|&t| t != 0)
            .collect()
    }

    // ---- lot developer ----
    fn area_slope(&self, rect: CellRect, with_water: bool) -> f32 {
        let r = fc4_cell_rect::from(rect);
        cb!(self, area_slope, 0.0, &r, with_water)
    }
    fn attempt_area_levelling(&self, rect: CellRect) {
        if self.shadow {
            return self.swallow();
        }
        let r = fc4_cell_rect::from(rect);
        cb!(self, attempt_area_levelling, (), &r)
    }
    fn remove_building_zot_prop(&self, lot: u64) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, remove_building_zot_prop, (), lot)
    }
    fn lot_base_texture_occupant(&self, lot: u64) -> Option<u64> {
        if self.shadow {
            self.swallow();
            return None;
        }
        opt(cb!(self, lot_base_texture_occupant, 0, lot))
    }
    fn remove_lot_from_business_unit(&self, lot: u64) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, remove_lot_from_business_unit, (), lot)
    }
    fn set_empty_lot_base_texture(&self, lot: u64) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, set_empty_lot_base_texture, (), lot)
    }
    fn force_empty_lot_base_textures(&self) -> bool {
        cb!(self, force_empty_lot_base_textures, false)
    }
    fn zone_manager_dezone(&self, region: &CellRegion) {
        if self.shadow {
            return self.swallow();
        }
        let (_bytes, raw) = region_raw(region);
        cb!(self, zone_manager_dezone, (), &raw)
    }

    // ---- traffic simulator ----
    fn desired_lot_insertion_point(&self) -> Option<InsertionPoint> {
        if self.shadow {
            return None;
        }
        let (mut node, mut list) = (0u64, false);
        if !cb!(self, desired_lot_insertion_point, false, &mut node, &mut list) {
            return None;
        }
        Some(InsertionPoint { node: opt(node), list })
    }
    fn set_lot_insertion_point(&self, node: Option<u64>) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, set_lot_insertion_point, (), node.unwrap_or(0))
    }

    // ---- BDS and growth developers ----
    fn bds_developer(&self, dev_type: u32) -> Option<u64> {
        opt(cb!(self, bds_developer, 0, dev_type))
    }
    fn building_types_by_family(&self, family: u32) -> Vec<u32> {
        let Some(f) = self.c.building_types_by_family else {
            return Vec::new();
        };
        let mut v: Vec<u32> = Vec::new();
        // SAFETY: valid callback; `v` outlives the call.
        unsafe {
            f(
                self.c.user,
                family,
                Some(push_u32),
                &mut v as *mut Vec<u32> as *mut c_void,
            )
        };
        v
    }
    fn building_profile(&self, building: u32) -> BuildingProfile {
        let mut p = fc4_zonedev_profile::default();
        if !cb!(self, building_profile, false, building, &mut p) {
            return BuildingProfile::default();
        }
        BuildingProfile {
            purpose: p.purpose as u8,
            wealth: p.wealth as u8,
            capacity: p.capacity,
            ..BuildingProfile::default()
        }
    }
    fn dev_estimate_desirability(&self, dev: u64, x: i32, z: i32) -> i16 {
        cb!(self, dev_estimate_desirability, 0, dev, x, z)
    }
    fn dev_source_demand(&self, dev: u64) -> Option<f32> {
        let mut v = 0.0f32;
        cb!(self, dev_source_demand, false, dev, &mut v).then_some(v)
    }
    fn dev_growth_list(&self, dev: u64) -> Option<Vec<GrowthItem>> {
        let f = self.c.dev_growth_list?;
        let mut v: Vec<GrowthItem> = Vec::new();
        // SAFETY: valid callback; `v` outlives the call.
        let valid = unsafe {
            f(
                self.c.user,
                dev,
                Some(take_growth_items),
                &mut v as *mut Vec<GrowthItem> as *mut c_void,
            )
        };
        valid.then_some(v)
    }
    fn dev_desirability_value(&self, dev: u64, tx: i32, tz: i32) -> i16 {
        cb!(self, dev_desirability_value, 0, dev, tx, tz)
    }
    fn dev_desirability_shift(&self, dev: u64) -> u32 {
        cb!(self, dev_desirability_shift, 0, dev)
    }
    fn dev_purpose(&self, dev: u64) -> u32 {
        cb!(self, dev_purpose, 0, dev)
    }
    fn dev_wealth(&self, dev: u64) -> u32 {
        cb!(self, dev_wealth, 0, dev)
    }

    // ---- the master lot list mirror ----
    fn master_insert(&self, lot: u64, before: Option<u64>) {
        self.note(|t| t.created = t.created.wrapping_add(1));
        self.created.set(self.created.get().wrapping_add(1));
        if self.shadow {
            return self.swallow();
        }
        cb!(self, master_insert, (), lot, before.unwrap_or(0))
    }
    fn master_remove(&self, lot: u64) {
        self.note(|t| t.deleted = t.deleted.wrapping_add(1));
        self.deleted.set(self.deleted.get().wrapping_add(1));
        if self.shadow {
            return self.swallow();
        }
        cb!(self, master_remove, (), lot)
    }
    fn master_reset(&self, lots: &[u64]) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, master_reset, (), lots.as_ptr(), lots.len())
    }
}

// ---- the save stream --------------------------------------------------------------------------------

/// The record reader (§10.2): delegates to [`CIn`]; lots are read with IID
/// `0xE818FCED`. A lot comes with the one reference the stream hands over;
/// a FreeCity4 object (`fc4_ref::core_handle` set) comes without, so one is
/// taken. In shadow mode every reference is borrowed and nothing is taken.
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
        self.get_ref_as(IID_LOT)
    }
    fn get_ref_as(&mut self, iid: u32) -> Option<RefValue> {
        let f = self.inner.stream.get_ref?;
        let mut out = fc4_ref::default();
        // SAFETY: valid callback; `out` outlives the call.
        if !unsafe { f(self.inner.stream.user, iid, &mut out) } {
            return None;
        }
        if out.token != 0 && !out.core_handle.is_null() {
            self.host.add_ref(out.token);
        }
        Some(match out.token {
            0 => RefValue::Null,
            token => RefValue::Host {
                token,
                class_id: out.class_id,
            },
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

// ---- process-wide state -----------------------------------------------------------------------------

thread_local! {
    /// The zone developer's file-local generator of this process (§9).
    static STATICS: RefCell<Option<SharedZoneStatics>> = const { RefCell::new(None) };
}

/// The process-wide statics, created (and the generator seeded) on first use.
fn statics(session_seed: u32) -> SharedZoneStatics {
    STATICS.with(|s| {
        s.borrow_mut()
            .get_or_insert_with(|| {
                // derive_seed never returns 0xFFFFFFFF, so the clock values
                // are never read (foundations §6.3).
                let seeds = FixedSeeds {
                    wall_clock: session_seed,
                    process_clock: session_seed,
                };
                ZoneDeveloperStatics::from_session(session_seed, &seeds)
            })
            .clone()
    })
}

// ---- handles ----------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_zonedev`).
#[repr(C)]
pub struct fc4_zonedev {
    _private: [u8; 0],
}

struct Handle {
    host: Rc<CHost>,
    dev: ZoneDeveloper,
    depth: Cell<u32>,
    reentrant: Cell<u32>,
    parcellizations: Cell<u32>,
    dezones: Cell<u32>,
}

impl Handle {
    fn call<R>(&self, f: impl FnOnce(&Handle) -> R) -> R {
        if self.depth.get() > 0 {
            self.reentrant.set(self.reentrant.get().wrapping_add(1));
        }
        self.depth.set(self.depth.get() + 1);
        let r = f(self);
        self.depth.set(self.depth.get() - 1);
        r
    }

    /// Runs `f` with a trace of its own (a nested call neither sees nor
    /// disturbs its caller's); returns its trace.
    fn traced<R>(&self, f: impl FnOnce(&Handle) -> R) -> (R, Trace) {
        let saved = self.host.trace.replace(Trace::default());
        let r = self.call(f);
        let mine = self.host.trace.replace(saved);
        (r, mine)
    }
}

fn with_handle<R>(p: *const fc4_zonedev, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_zonedev*.
    let Some(h) = (unsafe { (p as *const Handle).as_ref() }) else {
        return fallback;
    };
    guard(fallback, || f(h))
}

fn dev<R>(p: *const fc4_zonedev, fallback: R, f: impl FnOnce(&ZoneDeveloper) -> R) -> R {
    with_handle(p, fallback, |h| h.call(|h| f(&h.dev)))
}

fn host_ok(host: *const fc4_zonedev_host) -> Option<fc4_zonedev_host> {
    // SAFETY: documented precondition: NULL or a valid host table.
    let h = unsafe { host.as_ref() }?;
    let ok = (h.struct_size as usize) >= std::mem::size_of::<fc4_zonedev_host>() && h.grids.complete();
    ok.then_some(*h)
}

fn create(host: *const fc4_zonedev_host, session_seed: u32, shadow: bool) -> *mut fc4_zonedev {
    guard(std::ptr::null_mut(), || {
        let Some(h) = host_ok(host) else {
            return std::ptr::null_mut();
        };
        let chost = Rc::new(CHost::new(h, shadow));
        let dev = ZoneDeveloper::new(chost.clone(), statics(session_seed));
        Box::into_raw(Box::new(Handle {
            host: chost,
            dev,
            depth: Cell::new(0),
            reentrant: Cell::new(0),
            parcellizations: Cell::new(0),
            dezones: Cell::new(0),
        })) as *mut fc4_zonedev
    })
}

fn write_result(out: *mut fc4_zonedev_parcel_result, t: &Trace) {
    // SAFETY: documented precondition: NULL or a valid pointer.
    if let Some(o) = unsafe { out.as_mut() } {
        *o = t.result();
    }
}

fn rect_in(r: *const fc4_cell_rect) -> Option<CellRect> {
    // SAFETY: documented precondition: NULL or a valid pointer.
    unsafe { r.as_ref() }.map(|r| CellRect::from(*r))
}

/// Hands each lot to `cb` (with the reference the core took for it); without
/// a callback the references are given back.
fn deliver(h: &Handle, lots: &[u64], cb: fc4_token_callback, ctx: *mut c_void) -> u32 {
    match cb {
        Some(f) => {
            for &l in lots {
                // SAFETY: the caller promised a valid callback for the call.
                unsafe { f(ctx, l) };
            }
        }
        None => {
            for &l in lots {
                h.host.release(l);
            }
        }
    }
    lots.len() as u32
}

/// `existing[0..n]`: the caller's list before the call (NULL = empty).
fn existing_of<'a>(existing: *const u64, n: usize) -> &'a [u64] {
    if existing.is_null() || n == 0 {
        &[]
    } else {
        // SAFETY: documented precondition: `n` readable tokens.
        unsafe { std::slice::from_raw_parts(existing, n) }
    }
}

/// `QueryInterface(iid)` (§1): whether the zone developer answers the IID
/// (`1`, `0xE818ED2B`, `0x681905BD`, `0x452294AA`, `0xE4FDA3D4`).
#[no_mangle]
pub extern "C" fn fc4_zonedev_query_interface(iid: u32) -> bool {
    developer_answers_iid(iid)
}

/// The class ID (`GetGZCLSID`): `0x498F9B01`.
#[no_mangle]
pub extern "C" fn fc4_zonedev_class_id() -> u32 {
    CLSID_ZONE_DEVELOPER
}

/// Creates the core object (§3.2's initial state). The host table is copied.
/// `session_seed` seeds the process-wide generator the first time (§9). NULL
/// when the table is incomplete.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_create(host: *const fc4_zonedev_host, session_seed: u32) -> *mut fc4_zonedev {
    create(host, session_seed, false)
}

/// Creates a core object in shadow mode (see the module notes).
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_create_shadow(
    host: *const fc4_zonedev_host,
    session_seed: u32,
) -> *mut fc4_zonedev {
    create(host, session_seed, true)
}

/// Destroys the core object. Lots it still holds are not released (call
/// `Shutdown` first); a grid it holds is shut down and released.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_destroy(p: *mut fc4_zonedev) {
    if p.is_null() {
        return;
    }
    guard((), || drop(Box::from_raw(p as *mut Handle)));
}

/// `Init` (slot 3, §1.2) without the 3D view: false when a service is
/// missing (the initialised flag stays unset).
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_init(p: *mut fc4_zonedev) -> bool {
    dev(p, false, ZoneDeveloper::init)
}

/// `Shutdown` (slot 4, §1.2): AL = 1.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_shutdown(p: *mut fc4_zonedev) -> bool {
    dev(p, false, ZoneDeveloper::shutdown)
}

/// `DoMessage` (§5.2): always true. `occupant` is void1 (InsertOccupant),
/// `data1` the cheat ID, `value` the u32 at void2 (`0xA6B79602`). `out` (may
/// be NULL) tells what the message did.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fc4_zonedev_do_message(
    p: *mut fc4_zonedev,
    msg_type: u32,
    occupant: u64,
    data1: u32,
    data3: u32,
    data4: u32,
    value: u32,
    out: *mut fc4_zonedev_parcel_result,
) -> bool {
    let m = ZonesMessage {
        id: msg_type,
        occupant: opt(occupant),
        data1,
        data3,
        data4,
        value,
    };
    let (r, t) = with_handle(p, (true, Trace::default()), |h| h.traced(|h| h.dev.do_message(&m)));
    write_result(out, &t);
    r
}

/// Slot 6 `DoParcellization(region, type, flag)` (§7.1). A NULL region or
/// one larger than the core accepts does nothing. `out` (may be NULL) tells
/// what it did.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_do_parcellization(
    p: *mut fc4_zonedev,
    region: *const fc4_zone_region,
    zone_type: i32,
    flag: bool,
    out: *mut fc4_zonedev_parcel_result,
) {
    let mut t = Trace::default();
    if let Some(reg) = region.as_ref().and_then(|r| region_in(r)) {
        t = with_handle(p, Trace::default(), |h| {
            h.parcellizations.set(h.parcellizations.get().wrapping_add(1));
            if !fc4_core::zones::is_rci_zone(zone_type) {
                h.dezones.set(h.dezones.get().wrapping_add(1));
            }
            h.traced(|h| h.dev.do_parcellization(&reg, zone_type, flag)).1
        });
    }
    write_result(out, &t);
}

/// Slot 5 `HighlightParcels(region, type, point, out)` (§7.1, ZQ4). `point`
/// is NULL or two ints (x, z). `want_out`: the caller passed an out pointer;
/// then `*out_written` tells whether the core wrote a region and
/// `*out_rect` its rectangle. `out_region` (may be NULL) receives that
/// region's rect and bits so the caller can build the game's
/// `SC4CellRegion<long>` (ZG24): on entry `mask` is the caller's buffer and
/// `len` its capacity in bytes, on success `len` is the cell count. A buffer
/// that is too small writes nothing and clears `*out_written`, so the caller
/// hands out null. `res` (may be NULL) tells what the call did.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fc4_zonedev_highlight_parcels(
    p: *mut fc4_zonedev,
    region: *const fc4_zone_region,
    zone_type: i32,
    point: *const i32,
    want_out: bool,
    out_written: *mut bool,
    out_rect: *mut fc4_cell_rect,
    out_region: *mut fc4_zone_region,
    res: *mut fc4_zonedev_parcel_result,
) {
    let pt = (!point.is_null()).then(|| (*point, *point.add(1)));
    let mut written: Option<CellRegion> = None;
    let mut t = Trace::default();
    if let Some(reg) = region.as_ref().and_then(|r| region_in(r)) {
        t = with_handle(p, Trace::default(), |h| {
            h.traced(|h| {
                let out = if want_out { Some(&mut written) } else { None };
                h.dev.highlight_parcels(&reg, zone_type, pt, out);
            })
            .1
        });
    }
    if let Some(w) = out_written.as_mut() {
        *w = written.is_some();
    }
    if let (Some(r), Some(o)) = (written.as_ref(), out_rect.as_mut()) {
        *o = fc4_cell_rect::from(r.rect);
    }
    // ZG24: the caller owns the region and frees rows, row array and object
    // with C (zones.md §13.2), so the bits have to leave this module. The
    // bit grid is `CellRegion`'s (at most MAX_REGION_SIDE per side), which is
    // the rectangle handed out.
    if let (Some(r), Some(or)) = (written.as_ref(), out_region.as_mut()) {
        let (w, h) = (i64::from(r.width()), i64::from(r.height()));
        let cells = (w.max(0) * h.max(0)) as usize;
        if or.mask.is_null() || or.len < cells {
            // Too small to hold the region: hand out nothing rather than a
            // truncated one (the caller then stores null, which §13.2 allows).
            if let Some(ow) = out_written.as_mut() {
                *ow = false;
            }
        } else {
            or.x0 = r.rect.x1;
            or.z0 = r.rect.z1;
            or.x1 = r.rect.x1.wrapping_add((w - 1).max(0) as i32);
            or.z1 = r.rect.z1.wrapping_add((h - 1).max(0) as i32);
            or.len = cells;
            region_out(r, or);
        }
    }
    write_result(res, &t);
}

/// Slot 7 `SetOptions(alternateOrientation, placeStreets, customZoneSize)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_set_options(p: *mut fc4_zonedev, alternate: bool, streets: bool, custom: bool) {
    dev(p, (), |d| d.set_options(alternate, streets, custom))
}

/// Slot 8 `ExistsNetworkOfType(x, z, flags)` (§8.8).
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_exists_network_of_type(p: *const fc4_zonedev, x: i32, z: i32, flags: u32) -> bool {
    dev(p, false, |d| d.exists_network_of_type(x, z, flags))
}

/// Slot 9 `IsCellWater(x, z)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_is_cell_water(p: *const fc4_zonedev, x: i32, z: i32) -> bool {
    dev(p, false, |d| d.is_cell_water(x, z))
}

/// Lot manager slot 5 `GetLot(const cS3DVector3&)`: borrowed; 0 = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_get_lot_at_position(p: *const fc4_zonedev, pos: *const f32) -> u64 {
    if pos.is_null() {
        return 0;
    }
    let v = [*pos, *pos.add(1), *pos.add(2)];
    dev(p, 0, |d| d.get_lot_at_position(v).unwrap_or(0))
}

/// Slot 6 `GetLot(x, z, originOnly)`: borrowed; 0 = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_get_lot(p: *const fc4_zonedev, x: i32, z: i32, origin_only: bool) -> u64 {
    dev(p, 0, |d| d.get_lot(x, z, origin_only).unwrap_or(0))
}

/// Slot 7 `GetLotsInCellRect` (§8.3): the lots to append, `cb(ctx, lot)`
/// each with ONE reference for the caller (the caller clears its list
/// first). Returns the count.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_lots_in_cell_rect(
    p: *const fc4_zonedev,
    rect: *const fc4_cell_rect,
    origin_only: bool,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) -> u32 {
    let Some(r) = rect_in(rect) else { return 0 };
    with_handle(p, 0, |h| {
        let lots = h.call(|h| h.dev.get_lots_in_cell_rect(r, origin_only));
        deliver(h, &lots, cb, ctx)
    })
}

/// Slot 10 `GetLotsSurroundingRect(rect, list, flags)` (§8.3): the lots to
/// append to a list already holding `existing[0..n]`, `cb(ctx, lot)` each
/// with ONE reference. Returns the count appended.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fc4_zonedev_lots_surrounding_rect(
    p: *const fc4_zonedev,
    rect: *const fc4_cell_rect,
    flags: u32,
    existing: *const u64,
    n: usize,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) -> u32 {
    let Some(r) = rect_in(rect) else { return 0 };
    let old = existing_of(existing, n);
    with_handle(p, 0, |h| {
        let lots = h.call(|h| h.dev.get_lots_surrounding_rect_after(r, flags, old));
        deliver(h, &lots, cb, ctx)
    })
}

/// Slot 9 `GetLotsSurroundingLot(lot, list, flags)`: the lot's bounding rect,
/// then slot 10.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fc4_zonedev_lots_surrounding_lot(
    p: *const fc4_zonedev,
    lot: u64,
    flags: u32,
    existing: *const u64,
    n: usize,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) -> u32 {
    if lot == 0 {
        return 0;
    }
    let old = existing_of(existing, n);
    with_handle(p, 0, |h| {
        let lots = h.call(|h| {
            let r = h.host.lot_bounding_rect(lot);
            h.dev.get_lots_surrounding_rect_after(r, flags, old)
        });
        deliver(h, &lots, cb, ctx)
    })
}

/// Slot 8 `GetLotsSurroundingLot(x, z, list, flags)`: the lot at (x, z)
/// (slot 6), none → 0, else slot 9.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fc4_zonedev_lots_surrounding_cell(
    p: *const fc4_zonedev,
    x: i32,
    z: i32,
    flags: u32,
    existing: *const u64,
    n: usize,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) -> u32 {
    let lot = fc4_zonedev_get_lot(p, x, z, false);
    fc4_zonedev_lots_surrounding_lot(p, lot, flags, existing, n, cb, ctx)
}

/// Slot 11 `GetVerticesAtEdgeOfLot(rect, flags, out)`: false when no flag
/// bit is set (`out` untouched).
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_vertices_at_edge_of_lot(
    p: *const fc4_zonedev,
    rect: *const fc4_cell_rect,
    flags: u32,
    out: *mut fc4_cell_rect,
) -> bool {
    let Some(r) = rect_in(rect) else { return false };
    match dev(p, None, |d| d.vertices_at_edge_of_lot(r, flags)) {
        Some(v) => {
            if let Some(o) = out.as_mut() {
                *o = v.into();
            }
            true
        }
        None => false,
    }
}

/// Slot 12 `GetNeighborCellsAtEdgeOfLot(rect, flags, out)` → AL.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_neighbor_cells_at_edge_of_lot(
    p: *const fc4_zonedev,
    rect: *const fc4_cell_rect,
    flags: u32,
    out: *mut fc4_cell_rect,
) -> bool {
    let Some(r) = rect_in(rect) else { return false };
    let Some((v, al)) = dev(p, None, |d| Some(d.neighbor_cells_at_edge_of_lot(r, flags))) else {
        return false;
    };
    if let Some(o) = out.as_mut() {
        *o = v.into();
    }
    al
}

/// Slot 13 `GetRandomLot()` (§9 Z1): borrowed; 0 = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_random_lot(p: *mut fc4_zonedev) -> u64 {
    dev(p, 0, |d| d.get_random_lot().unwrap_or(0))
}

/// Slot 14 `GetOccupantLot(occupant)`: borrowed; 0 = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_occupant_lot(p: *const fc4_zonedev, occupant: u64) -> u64 {
    if occupant == 0 {
        return 0;
    }
    dev(p, 0, |d| d.get_occupant_lot(occupant).unwrap_or(0))
}

/// Slot 16 `GetLotCount()`.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_lot_count(p: *const fc4_zonedev) -> u32 {
    dev(p, 0, |d| d.lot_count() as u32)
}

/// The master list in order, `cb(ctx, lot)` each (borrowed). Returns the
/// count.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_master_lots(
    p: *const fc4_zonedev,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) -> u32 {
    let lots = dev(p, Vec::new(), |d| d.master_lots());
    if let Some(f) = cb {
        for &l in &lots {
            f(ctx, l);
        }
    }
    lots.len() as u32
}

/// Slot 17 `GetLotSavvyTerrainVertexAltitude(vx, vz, exclude, ok)` (ST0).
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_lot_savvy_terrain_vertex_altitude(
    p: *const fc4_zonedev,
    vx: i32,
    vz: i32,
    exclude: u64,
    ok: *mut bool,
) -> f32 {
    let (a, o) = dev(p, (0.0, false), |d| {
        d.lot_savvy_terrain_vertex_altitude(vx, vz, opt(exclude))
    });
    if let Some(w) = ok.as_mut() {
        *w = o;
    }
    a
}

/// Slot 18 `GetLotSavvyTerrainAltitude(x, z, ok)` (ST0); `ok` may be NULL.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_lot_savvy_terrain_altitude(
    p: *const fc4_zonedev,
    x: f32,
    z: f32,
    ok: *mut bool,
) -> f32 {
    let (a, o) = dev(p, (0.0, false), |d| d.lot_savvy_terrain_altitude(x, z));
    if let Some(w) = ok.as_mut() {
        *w = o;
    }
    a
}

/// Slot 19 `GetLotSavvyCellAltitude(cx, cz, ok)` (ST0); `ok` may be NULL.
/// §8.7 writes `*ok` on both paths when `ok` is non-null (ZG22): 1 for a lot
/// with flag bit 0, 0 for the terrain path. Slots 17 and 18 follow the same
/// rule; the earlier "[choice] left untouched" reading is superseded.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_lot_savvy_cell_altitude(
    p: *const fc4_zonedev,
    cx: i32,
    cz: i32,
    ok: *mut bool,
) -> f32 {
    let (a, o) = dev(p, (0.0, false), |d| d.lot_savvy_cell_altitude(cx, cz));
    if let Some(w) = ok.as_mut() {
        *w = o;
    }
    a
}

/// Slot 20 `CanCreateLot(x, z, w, h, zoneNotNeeded, waterAllowed, cfg)`
/// (§8.4): the minimum cell code.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fc4_zonedev_can_create_lot(
    p: *const fc4_zonedev,
    x: i32,
    z: i32,
    w: i32,
    h: i32,
    zone_not_needed: bool,
    water_allowed: bool,
    cfg: u64,
) -> i32 {
    dev(p, 0, |d| {
        d.can_create_lot_cfg(x, z, w, h, zone_not_needed, water_allowed, opt(cfg))
    })
}

/// Slot 21 `CanCreatePloppedLot(rect, facing, cfg)` (§8.4.1): 0 outside, 1
/// water, 3 network, 4 too steep, 5 / 6 constraint tile, 7 special zone, 8
/// free. A NULL rect gives 0. Writes nothing.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_can_create_plopped_lot(
    p: *const fc4_zonedev,
    rect: *const fc4_cell_rect,
    facing: i32,
    cfg: u64,
) -> i32 {
    let Some(r) = rect_in(rect) else { return 0 };
    dev(p, 0, |d| d.can_create_plopped_lot(r, facing, cfg))
}

/// Slot 22 `CreateLotIfPossible(x, z, w, h, facing, out, zoneNotNeeded,
/// waterAllowed, cfg)` (§8.5; `cfg` ignored): EAX. `*out` (may be NULL)
/// receives the lot with ONE reference for the caller, or 0.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fc4_zonedev_create_lot_if_possible(
    p: *mut fc4_zonedev,
    x: i32,
    z: i32,
    w: i32,
    h: i32,
    facing: i32,
    zone_not_needed: bool,
    water_allowed: bool,
    out: *mut u64,
) -> bool {
    let r = with_handle(p, None, |hd| {
        Some(hd.call(|hd| {
            hd.dev
                .create_lot_if_possible_full(x, z, w, h, facing, zone_not_needed, water_allowed)
        }))
    });
    let (lot, ok) = r.map_or((None, false), |r| (r.lot, r.ok));
    match out.as_mut() {
        Some(o) => *o = lot.unwrap_or(0),
        None => {
            if let Some(l) = lot {
                with_handle(p, (), |h| h.host.release(l));
            }
        }
    }
    ok
}

/// Slot 23 `CreateLot(x, z, w, h, facing, out)` (§8.5): EAX 0/1. `*out`
/// (may be NULL) receives the lot with ONE reference for the caller.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fc4_zonedev_create_lot(
    p: *mut fc4_zonedev,
    x: i32,
    z: i32,
    w: i32,
    h: i32,
    facing: i32,
    out: *mut u64,
) -> bool {
    let lot = dev(p, None, |d| d.create_lot(x, z, w, h, facing));
    match out.as_mut() {
        Some(o) => *o = lot.unwrap_or(0),
        None => {
            if let Some(l) = lot {
                with_handle(p, (), |h| h.host.release(l));
            }
        }
    }
    lot.is_some()
}

/// Slot 24 `DeleteLot(lot)` (§8.6).
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_delete_lot(p: *mut fc4_zonedev, lot: u64) {
    if lot == 0 {
        return;
    }
    dev(p, (), |d| d.delete_lot(lot))
}

/// Slot 25 `DeleteLots(x1, z1, x2, z2)` (§8.6).
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_delete_lots(p: *mut fc4_zonedev, x1: i32, z1: i32, x2: i32, z2: i32) {
    dev(p, (), |d| d.delete_lots(CellRect::new(x1, z1, x2, z2)))
}

/// Slot 26 `GetLotVertexAltitudeAdjustmentRange(vx, vz, a, b)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_lot_vertex_altitude_adjustment_range(
    p: *const fc4_zonedev,
    vx: i32,
    vz: i32,
    a: *mut f32,
    b: *mut f32,
) {
    let (ra, rb) = dev(p, (f32::MAX, f32::MAX), |d| {
        d.lot_vertex_altitude_adjustment_range(vx, vz)
    });
    if let Some(o) = a.as_mut() {
        *o = ra;
    }
    if let Some(o) = b.as_mut() {
        *o = rb;
    }
}

/// Slot 27 `GetMaximumCapacityGrid()` (no AddRef): the grid token, 0 = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_max_capacity_grid(p: *const fc4_zonedev) -> u64 {
    dev(p, 0, |d| grid_token(&d.maximum_capacity_grid()))
}

/// The state (diagnostics and the getters' snapshot).
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_get_state(p: *const fc4_zonedev, out: *mut fc4_zonedev_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_handle(p, false, |h| {
        let d = &h.dev;
        let opts = d.options();
        let (map_x, map_z) = d.map_size();
        *o = fc4_zonedev_state {
            struct_size: std::mem::size_of::<fc4_zonedev_state>() as u32,
            initialized: u8::from(d.is_initialised()),
            loaded: u8::from(d.is_loaded()),
            real_mode: u8::from(d.is_real_mode()),
            reserved: 0,
            lot_count: d.lot_count() as u32,
            map_x,
            map_z,
            lots_created: h.host.created.get(),
            lots_deleted: h.host.deleted.get(),
            parcellizations: h.parcellizations.get(),
            dezones: h.dezones.get(),
            reentrant_calls: h.reentrant.get(),
            view_value: d.view_message_value(),
            options: u32::from(opts.alternate_orientation)
                | (u32::from(opts.place_streets) << 1)
                | (u32::from(opts.custom_zone_size) << 2),
            max_capacity_grid: grid_token(&d.maximum_capacity_grid()),
        };
        true
    })
}

/// `Write` (§10.2). Lots go out as `fc4_ref`s with the token and class 0.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_write(p: *const fc4_zonedev, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    dev(p, false, |d| d.write_record(&mut COut(o)))
}

/// `Read` (§10.2). Runs before `Init`: nothing is sized from the city here.
/// Every lot is asked for with IID `0xE818FCED`.
#[no_mangle]
pub unsafe extern "C" fn fc4_zonedev_read(p: *mut fc4_zonedev, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_handle(p, false, |h| {
        let host = h.host.clone();
        h.call(|h| {
            h.dev.read_record(&mut DIn {
                inner: CIn {
                    stream: i,
                    decode: no_core_objects,
                },
                host: &host,
            })
        })
    })
}
