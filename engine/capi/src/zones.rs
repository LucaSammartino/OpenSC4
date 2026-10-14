// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The zone manager's C ABI (fc4.h "zones", docs/specs/zones.md §1.1, §2–§6,
//! §10.1, §11.1).
//!
//! The host implements [`fc4_zones_host`] (every `ZonesHost` method the zone
//! manager reaches, the zone grid through the shared [`fc4_grid_api`]); the
//! plugin's `cISC4ZoneManager` (with its message-target and serializable
//! sub-objects) forwards its slots to the `fc4_zones_*` functions. The zone
//! developer / lot manager stays the game's: the host reaches it through
//! `cISC4ZoneDeveloper` / `cISC4LotManager` (`get_lot`, `lots_in_cell_rect`,
//! `do_parcellization`); the developer-only methods of `ZonesHost` are
//! neutral stubs here.
//!
//! Tokens are opaque host identities (fc4.h): lots (`cISC4Lot*`), lot
//! configurations, occupants and network tiles (any interface of the object:
//! the host queries what it needs), services, the grid. 0 is the null object.
//! Tokens the host returns are **borrowed** unless a callback says otherwise.
//!
//! # PlaceZone's outputs (§6)
//!
//! `outLots` is filled through a token callback **at the pricing call**
//! (`DemolishLots(false, …)`), where the original inserts into the set: the
//! commit's `DemolishLots(true, …)` comes later and may free the lots, so the
//! caller's references must be taken before it. The callback is armed only
//! for the `fc4_zones_place_zone` call that passed it (a nested PlaceZone,
//! e.g. a dezone delivered during the commit, does not see it).
//!
//! # Re-entrancy
//!
//! The core holds no borrow across a host call (`ZoneManager`), so every
//! re-entrant call runs at once: a dezone delivered during a commit
//! (`DemolishLots`, `DoParcellization` laying streets), `GetZoneType` from
//! the zone developer during `DoParcellization`. They are counted
//! (`fc4_zones_state::reentrant_calls`).
//!
//! # Threads
//!
//! The core is single-threaded (`RefCell`). The plugin serialises every call
//! on one object with its own lock and publishes a snapshot
//! ([`fc4_zones_get_state`], [`fc4_zones_get_tuning`]) for the getters other
//! threads call.
//!
//! # Shadow mode
//!
//! [`fc4_zones_create_shadow`] makes a core whose host table reads the game
//! but **never writes it**: services, subscriptions, funds, the commit's
//! demolition, parcellization, ZoningChange and the lot textures are recorded
//! and swallowed; references read from a save are borrowed (none is taken,
//! adopted or released). The shadow's grid table should leave `set`, `fill`,
//! `shutdown` and `release` NULL (the grid it reads is the original's).

use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::rc::Rc;

use fc4_core::grid::{CellRect, GridProvider, TractGrid};
use fc4_core::property::PropertyHolder;
use fc4_core::stream::{InStream, RefValue, SaveObject};
use fc4_core::zones::{
    manager_answers_iid, BuildingProfile, CellRegion, Component, GrowthItem, InsertionPoint, Position, Service,
    ZoneManager, ZonesHost, ZonesMessage, CLSID_ZONE_MANAGER, IID_BUDGET, IID_CITY, IID_OCCUPANT_MANAGER,
    IID_POLLUTION, IID_TERRAIN, MAX_REGION_SIDE,
};

use crate::budget::fc4_token_callback;
use crate::guard;
use crate::hostgrid::{collect_tokens, create_grid, fc4_cell_rect, fc4_grid_api, grid_token, resolve_grid, take_props};
use crate::props::fc4_props;
use crate::stream::{fc4_istream, fc4_ostream, fc4_ref, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

/// The services whose references the zone manager saves (§10.1 field 13), in
/// record order: the `which` of `service_ref` / `adopt_service`.
pub const FC4_ZONES_SERVICE_CITY: u32 = 0;
pub const FC4_ZONES_SERVICE_OCCUPANT_MANAGER: u32 = 1;
pub const FC4_ZONES_SERVICE_TERRAIN: u32 = 2;
pub const FC4_ZONES_SERVICE_POLLUTION: u32 = 3;
pub const FC4_ZONES_SERVICE_BUDGET: u32 = 4;

/// The IIDs `Read` asks for the five service references (§10.1), by `which`.
const SERVICE_IIDS: [u32; 5] = [IID_CITY, IID_OCCUPANT_MANAGER, IID_TERRAIN, IID_POLLUTION, IID_BUDGET];

/// The "Zone Manager" tuning exemplar (§4): type, group, instance. [question]
/// The core's `TUNING_KEY` names type `0xA52160F5`; the spec's §4 names
/// `0x6534284A`, which the host loads (the core only reads the holder).
pub const FC4_ZONES_TUNING_TYPE: u32 = 0x6534_284A;
pub const FC4_ZONES_TUNING_GROUP: u32 = 0xE7E2_C2DB;
pub const FC4_ZONES_TUNING_INSTANCE: u32 = 0xE948_2490;

/// A cell region crossing the ABI (fc4.h `fc4_zone_region`): the game's
/// `SC4CellRegion<long>` (§11.4) as an inclusive rectangle plus one byte per
/// cell.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_zone_region {
    pub x0: i32,
    pub z0: i32,
    pub x1: i32,
    pub z1: i32,
    /// `(x1 − x0 + 1) × (z1 − z0 + 1)` bytes, index `(z − z0) × width +
    /// (x − x0)`, non-zero = a set cell. NULL: every cell is set. A short mask
    /// reads as cleared cells past its end. `fc4_zones_place_zone` writes the
    /// region back here (rejected cells cleared).
    pub mask: *mut u8,
    pub len: usize,
}

/// `PlaceZone`'s outputs (fc4.h `fc4_zones_place_result`, §6).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_zones_place_result {
    pub struct_size: u32,
    /// AL.
    pub ok: u8,
    /// 1 when the call committed a change and posted ZoningChange.
    pub committed: u8,
    pub reserved: [u8; 2],
    /// `*outError`: 0 ok, 1 funds, 2 landfill on water, 3 too small, 4 too
    /// large, 5 blocked.
    pub error: i32,
    /// Cells of the ZoningChange region (the accepted cells; 0 unless
    /// committed).
    pub accepted: u32,
    /// Region bits this call cleared (rejected cells).
    pub cleared: u32,
    /// Lots to demolish (`*outLots`' new elements).
    pub lots: u32,
    /// `*outCost`.
    pub cost: i64,
}

/// What a `DoMessage` did (fc4.h `fc4_zones_dezone`, §5.1).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_zones_dezone {
    pub struct_size: u32,
    /// 1 when the message dezoned (`PlaceZone(rect, 0, …)` ran).
    pub dezoned: u8,
    pub reserved: [u8; 3],
    /// The occupant's bounding cells (`GetBoundingCityCells`).
    pub rect: fc4_cell_rect,
    /// Cells re-zoned (the ZoningChange region; 0 when nothing was accepted
    /// or the dezone was blocked).
    pub changed: u32,
    /// Lots handed to the commit's `DemolishLots(true, …)`.
    pub lots: u32,
}

/// Diagnostics and the getters' snapshot (fc4.h `fc4_zones_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_zones_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub loaded: u8,
    pub display_alpha: u8,
    pub default_alpha: u8,
    /// Slot 6 per type.
    pub zone_count: [u32; 16],
    /// Slot 7 per type (never updated after `Init`, §2).
    pub undeveloped: [u32; 16],
    /// Slot 8.
    pub undeveloped_total: u32,
    /// Slot 10.
    pub abandoned_tile_count: u32,
    /// Entries of development-failure map 1 (slot 9 inserts zero entries).
    pub failure1_entries: u32,
    pub reentrant_calls: u32,
    /// `fc4_zones_place_zone` calls.
    pub place_calls: u32,
    /// InsertOccupant messages that dezoned.
    pub dezones: u32,
    /// The zone grid's tract counts (0 × 0 without a grid, or before the game
    /// read a loaded grid's record).
    pub grid_count_x: i32,
    pub grid_count_z: i32,
    /// The zone grid token (borrowed; slot 5); 0 = none.
    pub grid: u64,
}

/// The tuning (fc4.h `fc4_zones_tuning`, §4), indexed by zone type.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct fc4_zones_tuning {
    pub struct_size: u32,
    pub display_alpha: u8,
    pub default_alpha: u8,
    pub reserved: [u8; 2],
    pub min_size: [i32; 16],
    pub max_size: [i32; 16],
    pub create_cost: [i64; 16],
    pub destroy_cost: [i64; 16],
    pub texture: [u32; 16],
    pub drag_color: [u32; 16],
    pub max_slope: [f32; 16],
}

/// Everything the zone manager asks of the game (fc4.h `fc4_zones_host`).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_zones_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    /// The zone grid (int8 `0x49B9E603`).
    pub grids: fc4_grid_api,
    // lifecycle and services (§1.1, §10.1)
    /// `Init` step 2: take the city, occupant manager, terrain, pollution and
    /// budget simulators that are still null, and the message server.
    pub acquire_services: Option<unsafe extern "C" fn(U)>,
    /// `Shutdown`: release them.
    pub release_services: Option<unsafe extern "C" fn(U)>,
    pub subscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, u32)>,
    /// A NEW fc4_props (the core destroys it), NULL if the exemplar is missing.
    pub tuning: Option<unsafe extern "C" fn(U) -> *mut fc4_props>,
    /// City slots 93, 94.
    pub cell_counts: Option<unsafe extern "C" fn(U, *mut i32, *mut i32)>,
    /// The token `Write` saves for service `which` (`FC4_ZONES_SERVICE_*`);
    /// 0 = null.
    pub service_ref: Option<unsafe extern "C" fn(U, u32) -> u64>,
    /// `Read`: service `which` as read from the save (its IID's interface;
    /// 0 = null). The host takes the token's ONE reference and replaces the
    /// service it held.
    pub adopt_service: Option<unsafe extern "C" fn(U, u32, u64)>,
    // messages posted (§5.3)
    /// ZoningChange `0x07699455` (region, type, checkSlope): `MessagePost` of
    /// a message owning a copy of the region.
    pub post_zoning_change: Option<unsafe extern "C" fn(U, *const fc4_zone_region, i32, bool)>,
    // the zone developer / lot manager (§13.1)
    /// Lot manager slot 6 `GetLot(x, z, false)`: borrowed; 0 = none.
    pub get_lot: Option<unsafe extern "C" fn(U, i32, i32) -> u64>,
    /// Lot manager slot 7 `GetLotsInCellRect`: `cb(ctx, lot)` per lot, ONE
    /// reference each for the core.
    pub lots_in_cell_rect: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, bool, fc4_token_callback, U)>,
    /// Zone developer slot 6 `DoParcellization(region, type, flag)`.
    pub do_parcellization: Option<unsafe extern "C" fn(U, *const fc4_zone_region, i32, bool)>,
    // budget, demolition, landfill (§6)
    pub total_funds: Option<unsafe extern "C" fn(U) -> i64>,
    pub withdraw_funds: Option<unsafe extern "C" fn(U, i64) -> bool>,
    /// Demolition slot 7 `DemolishLots(commit, lots, checkSlope, flags, 0, 0,
    /// &cost, 0)` → the cost.
    pub demolish_lots: Option<unsafe extern "C" fn(U, bool, *const u64, usize, bool, u32) -> i64>,
    /// `city.GetLandfillDeveloper().GetZoningCost(region)`.
    pub landfill_zoning_cost: Option<unsafe extern "C" fn(U, *const fc4_zone_region) -> i64>,
    // network tiles (§6)
    /// `FindNetworkTile(x, z, 0x1F4F, 1)`: borrowed; 0 = none.
    pub find_network_tile: Option<unsafe extern "C" fn(U, i32, i32) -> u64>,
    /// `tile.AsOccupant().IsFlagSet(flag)`.
    pub tile_occupant_flag_set: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub tile_is_of_type: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    pub tile_demolition_cost: Option<unsafe extern "C" fn(U, u64) -> i64>,
    // terrain, lot developer
    pub cell_has_water: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    /// Lot developer slot 29 `GetAreaSlope(rect, withWater)` (ST0).
    pub area_slope: Option<unsafe extern "C" fn(U, *const fc4_cell_rect, bool) -> f32>,
    /// Lot developer slot 43 `SetEmptyLotBaseTexture(lot)`.
    pub set_empty_lot_base_texture: Option<unsafe extern "C" fn(U, u64)>,
    // occupants (§5.1, §6)
    /// Occupant manager slot 33, the landfill occupant of the cell:
    /// borrowed; 0 = none.
    pub landfill_at: Option<unsafe extern "C" fn(U, i32, i32) -> u64>,
    /// Occupant slot 11 `GetBoundingCityCells`.
    pub occupant_bounding_cells: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    /// Occupant slot 14 `CanRemove(flags)`.
    pub occupant_can_remove: Option<unsafe extern "C" fn(U, u64, u32) -> bool>,
    /// `QueryInterface(cISC4NetworkOccupant)` then slot 9
    /// `HasAnyNetworkFlag(flags)`: 1 / 0, or −1 when the object is not a
    /// network occupant.
    pub network_has_any_flag: Option<unsafe extern "C" fn(U, u64, u32) -> i32>,
    // references
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    // lots and lot configurations
    /// Lot slot 9 `GetLotConfiguration`: borrowed; 0 = none.
    pub lot_configuration: Option<unsafe extern "C" fn(U, u64) -> u64>,
    /// Lot slot 14 `GetBoundingRect`.
    pub lot_bounding_rect: Option<unsafe extern "C" fn(U, u64, *mut fc4_cell_rect)>,
    /// Lot slot 31 `GetZoneType`.
    pub lot_zone_type: Option<unsafe extern "C" fn(U, u64) -> i32>,
    /// Lot slot 33 `GetState`.
    pub lot_state: Option<unsafe extern "C" fn(U, u64) -> u32>,
    /// Lot slot 48 `GetBuilding`: borrowed; 0 = none.
    pub lot_building: Option<unsafe extern "C" fn(U, u64) -> u64>,
    /// Lot configuration slot 15 `IsCompatibleWithZoneType(type)`.
    pub cfg_compatible_with_zone: Option<unsafe extern "C" fn(U, u64, i32) -> bool>,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_zone_region>() == if cfg!(target_pointer_width = "32") { 24 } else { 32 });
    assert!(std::mem::size_of::<fc4_zones_place_result>() == 32);
    assert!(std::mem::size_of::<fc4_zones_dezone>() == 32);
    assert!(std::mem::size_of::<fc4_zones_state>() == 176);
    assert!(std::mem::size_of::<fc4_zones_tuning>() == 584);
    assert!(std::mem::size_of::<fc4_zones_host>() == if cfg!(target_pointer_width = "32") { 188 } else { 376 });
};

// ---- regions ----------------------------------------------------------------------------------------

/// [choice] The most cells a region crossing the ABI may have (a 4096 × 256
/// rect); a side is at most [`MAX_REGION_SIDE`]. A larger region fails
/// `PlaceZone` with error 0 (the game never builds one: a city is at most
/// 256 × 256 cells).
const MAX_REGION_CELLS: i64 = 1 << 20;

/// The region a host hands in. `None` when it is too large. An empty
/// rectangle (x1 < x0 or z1 < z0) is a region with no cells, as the game's
/// would be.
unsafe fn region_in(r: &fc4_zone_region) -> Option<CellRegion> {
    let w = i64::from(r.x1) - i64::from(r.x0) + 1;
    let h = i64::from(r.z1) - i64::from(r.z0) + 1;
    if w > MAX_REGION_SIDE || h > MAX_REGION_SIDE || w.max(0) * h.max(0) > MAX_REGION_CELLS {
        return None;
    }
    let rect = CellRect::new(r.x0, r.z0, r.x1, r.z1);
    if w <= 0 || h <= 0 {
        return Some(CellRegion::new(rect, false));
    }
    if r.mask.is_null() {
        return Some(CellRegion::new(rect, true));
    }
    let n = r.len.min((w * h) as usize);
    let bytes = std::slice::from_raw_parts(r.mask as *const u8, n);
    let mut out = CellRegion::new(rect, false);
    for (i, &b) in bytes.iter().enumerate() {
        if b != 0 {
            let (dz, dx) = ((i as i64) / w, (i as i64) % w);
            out.set(r.x0.wrapping_add(dx as i32), r.z0.wrapping_add(dz as i32), true);
        }
    }
    Some(out)
}

/// Writes a region's bits back into the host's mask (nothing for a NULL
/// mask).
unsafe fn region_out(region: &CellRegion, r: &mut fc4_zone_region) {
    if r.mask.is_null() {
        return;
    }
    let w = i64::from(r.x1) - i64::from(r.x0) + 1;
    let h = i64::from(r.z1) - i64::from(r.z0) + 1;
    if w <= 0 || h <= 0 {
        return;
    }
    let n = r.len.min((w * h) as usize);
    let bytes = std::slice::from_raw_parts_mut(r.mask, n);
    for (i, b) in bytes.iter_mut().enumerate() {
        let (dz, dx) = ((i as i64) / w, (i as i64) % w);
        *b = u8::from(region.includes(r.x0.wrapping_add(dx as i32), r.z0.wrapping_add(dz as i32)));
    }
}

/// A region for a host call: the mask bytes and the struct pointing at them
/// (keep the bytes alive while the struct is used).
fn region_raw(r: &CellRegion) -> (Vec<u8>, fc4_zone_region) {
    let (w, h) = (r.width(), r.height());
    let mut bytes = vec![0u8; (w.max(0) as usize) * (h.max(0) as usize)];
    for (x, z) in r.cells() {
        let (dx, dz) = (x.wrapping_sub(r.rect.x1), z.wrapping_sub(r.rect.z1));
        bytes[dz as usize * w as usize + dx as usize] = 1;
    }
    // The rectangle as stored: `CellRegion` keeps at most MAX_REGION_SIDE
    // columns / rows of bits, so a wider rectangle is cut there (the regions
    // this module hands in never are); an empty one stays empty.
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

/// What the core asked of the game during one top-level call (a PlaceZone or
/// a DoMessage); nested calls keep their own (see `Handle::traced`).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Trace {
    /// `GetBoundingCityCells` of the InsertOccupant occupant (the dezone).
    pub bounding: Option<CellRect>,
    /// Cells of the last ZoningChange region.
    pub changed: u32,
    pub posted: u32,
    /// Lots of the last `DemolishLots(true, …)`.
    pub demolished: u32,
    /// Shadow: game writes swallowed.
    pub swallowed: u32,
}

/// The outLots sink armed for the current `fc4_zones_place_zone` call.
#[derive(Clone, Copy)]
struct Sink {
    cb: fc4_token_callback,
    ctx: *mut c_void,
}

pub(crate) struct CHost {
    c: fc4_zones_host,
    /// Shadow mode (see the module notes).
    shadow: bool,
    trace: Cell<Trace>,
    sink: Cell<Option<Sink>>,
}

impl CHost {
    pub(crate) fn new(c: fc4_zones_host, shadow: bool) -> CHost {
        CHost {
            c,
            shadow,
            trace: Cell::new(Trace::default()),
            sink: Cell::new(None),
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

impl GridProvider<i8> for CHost {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<i8>>> {
        // `Shutdown` then `Release` when the core drops it (§1.1 Shutdown).
        create_grid::<i8>(&self.c.grids, instance, tract_size, true)
    }
    fn resolve_grid(&self, r: &RefValue) -> Option<Rc<dyn TractGrid<i8>>> {
        resolve_grid::<i8>(&self.c.grids, r, true)
    }
}

impl GridProvider<u16> for CHost {
    /// The maximum-capacity grid belongs to the zone developer (the game's).
    fn create_grid(&self, _instance: u32, _tract_size: i32) -> Option<Rc<dyn TractGrid<u16>>> {
        None
    }
    fn resolve_grid(&self, _r: &RefValue) -> Option<Rc<dyn TractGrid<u16>>> {
        None
    }
}

impl ZonesHost for CHost {
    // ---- lifecycle, services and messages ----
    fn manager_acquire_services(&self) {
        if self.shadow {
            return;
        }
        cb!(self, acquire_services, ())
    }
    fn manager_release_services(&self) {
        if self.shadow {
            return;
        }
        cb!(self, release_services, ())
    }
    fn developer_acquire_services(&self) -> bool {
        false
    }
    fn developer_release_services(&self) {}
    fn subscribe(&self, who: Component, messages: &[u32]) {
        if self.shadow || who != Component::ZoneManager {
            return;
        }
        for &m in messages {
            cb!(self, subscribe, (), m)
        }
    }
    fn unsubscribe(&self, who: Component, messages: &[u32]) {
        if self.shadow || who != Component::ZoneManager {
            return;
        }
        for &m in messages {
            cb!(self, unsubscribe, (), m)
        }
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, tuning, std::ptr::null_mut());
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }
    fn city_cell_counts(&self) -> (i32, i32) {
        let (mut x, mut z) = (0i32, 0i32);
        cb!(self, cell_counts, (), &mut x, &mut z);
        (x, z)
    }
    fn service_ref(&self, service: Service) -> RefValue {
        let which = match service {
            Service::City => FC4_ZONES_SERVICE_CITY,
            Service::OccupantManager => FC4_ZONES_SERVICE_OCCUPANT_MANAGER,
            Service::Terrain => FC4_ZONES_SERVICE_TERRAIN,
            Service::Pollution => FC4_ZONES_SERVICE_POLLUTION,
            Service::Budget => FC4_ZONES_SERVICE_BUDGET,
        };
        match cb!(self, service_ref, 0, which) {
            0 => RefValue::Null,
            token => RefValue::Host { token, class_id: 0 },
        }
    }
    /// The grid is shut down and released when the core drops it.
    fn shutdown_grid(&self, _class: u32) {}
    fn acquire_network_tools(&self) {}
    fn release_network_tools(&self) {}
    fn post_zoning_change(&self, region: CellRegion, zone: i32, check_slope: bool) {
        let n = region.count() as u32;
        self.note(|t| {
            t.changed = n;
            t.posted = t.posted.wrapping_add(1);
        });
        if self.shadow {
            return self.swallow();
        }
        let (_bytes, raw) = region_raw(&region);
        cb!(self, post_zoning_change, (), &raw, zone, check_slope)
    }
    fn post_defer_occupant_operations(&self, _begin: bool) {}

    // ---- the other class ----
    fn zone_type(&self, _x: i32, _z: i32) -> i32 {
        0
    }
    fn lot_manager_get_lot(&self, x: i32, z: i32) -> Option<u64> {
        opt(cb!(self, get_lot, 0, x, z))
    }
    fn lot_manager_lots_in_cell_rect(&self, rect: CellRect, origin_only: bool) -> Vec<u64> {
        let Some(f) = self.c.lots_in_cell_rect else {
            return Vec::new();
        };
        let r = fc4_cell_rect::from(rect);
        // SAFETY: valid callback; `r` outlives the call.
        collect_tokens(|cb, ctx| unsafe { f(self.c.user, &r, origin_only, cb, ctx) })
            .into_iter()
            .filter(|&t| t != 0)
            .collect()
    }
    fn zone_developer_do_parcellization(&self, region: &CellRegion, zone: i32, flag: bool) {
        if self.shadow {
            return self.swallow();
        }
        let (_bytes, raw) = region_raw(region);
        cb!(self, do_parcellization, (), &raw, zone, flag)
    }

    // ---- budget, demolition, landfill ----
    fn total_funds(&self) -> i64 {
        cb!(self, total_funds, 0)
    }
    fn withdraw_funds(&self, amount: i64) -> bool {
        if self.shadow {
            self.swallow();
            return true;
        }
        cb!(self, withdraw_funds, false, amount)
    }
    fn demolish_lots(&self, commit: bool, lots: &[u64], check_slope: bool, flags: u32) -> i64 {
        if commit {
            self.note(|t| t.demolished = lots.len() as u32);
            if self.shadow {
                self.swallow();
                return 0;
            }
        }
        // The pricing call (`false`) only prices the lots (§6 notes): the
        // shadow makes it too.
        let cost = cb!(
            self,
            demolish_lots,
            0,
            commit,
            lots.as_ptr(),
            lots.len(),
            check_slope,
            flags
        );
        if !commit {
            // §6: "if outLots: insert every demolish lot into *outLots", right
            // after the pricing call and before the commit.
            if let Some(Sink { cb: Some(f), ctx }) = self.sink.get() {
                for &lot in lots {
                    // SAFETY: the caller of fc4_zones_place_zone promised a
                    // valid callback for the call.
                    unsafe { f(ctx, lot) };
                }
            }
        }
        cost
    }
    fn demolish_region(&self, _region: &CellRegion) {}
    fn landfill_zoning_cost(&self, region: &CellRegion) -> i64 {
        let (_bytes, raw) = region_raw(region);
        cb!(self, landfill_zoning_cost, 0, &raw)
    }

    // ---- networks ----
    fn find_network_tile(&self, x: i32, z: i32) -> Option<u64> {
        opt(cb!(self, find_network_tile, 0, x, z))
    }
    fn tile_occupant_flag_set(&self, tile: u64, flag: u32) -> bool {
        cb!(self, tile_occupant_flag_set, false, tile, flag)
    }
    fn tile_is_of_type(&self, tile: u64, t: u32) -> bool {
        cb!(self, tile_is_of_type, false, tile, t)
    }
    fn tile_demolition_cost(&self, tile: u64) -> i64 {
        cb!(self, tile_demolition_cost, 0, tile)
    }
    fn street_place(&self, _kind: u32, _a: (i32, i32), _b: (i32, i32), _real_mode: bool) -> bool {
        false
    }
    fn reset_network_tool(&self, _tool_kind: u32) {}
    fn connects_to_neighbor(&self, _occupant: u64, _edge: i32, _net_type: u32) -> bool {
        false
    }
    fn network_tool_occupant_exists(&self, _tool_kind: u32, _x: i32, _z: i32, _flag: u32) -> Option<[u8; 4]> {
        None
    }
    fn street_joining_helper(&self, _occupant: u64, _data4: u32) {}

    // ---- terrain ----
    fn cell_has_water(&self, x: i32, z: i32) -> bool {
        cb!(self, cell_has_water, false, x, z)
    }
    fn terrain_altitude(&self, _x: f32, _z: f32) -> f32 {
        0.0
    }
    fn terrain_vertex_altitude(&self, _vx: i32, _vz: i32) -> f32 {
        0.0
    }

    // ---- city and occupants ----
    fn location_in_bounds(&self, _pos: Position) -> bool {
        false
    }
    fn position_to_cell(&self, _pos: Position) -> Option<(i32, i32)> {
        None
    }
    fn cell_occupants(&self, _x: i32, _z: i32, _type_filter: Option<u32>) -> Vec<u64> {
        Vec::new()
    }
    fn landfill_at(&self, x: i32, z: i32) -> Option<u64> {
        opt(cb!(self, landfill_at, 0, x, z))
    }
    fn occupant_type(&self, _occupant: u64) -> u32 {
        0
    }
    fn occupant_position(&self, _occupant: u64) -> Position {
        [0.0; 3]
    }
    fn occupant_bounding_cells(&self, occupant: u64) -> CellRect {
        let r = self.rect_of(|out| cb!(self, occupant_bounding_cells, (), occupant, out));
        self.note(|t| t.bounding = Some(r));
        r
    }
    fn occupant_can_remove(&self, occupant: u64, flags: u32) -> bool {
        cb!(self, occupant_can_remove, false, occupant, flags)
    }
    fn network_has_any_flag(&self, occupant: u64, flags: u32) -> Option<bool> {
        match cb!(self, network_has_any_flag, -1, occupant, flags) {
            v if v < 0 => None,
            v => Some(v != 0),
        }
    }
    fn remove_occupant(&self, _occupant: u64) {}

    // ---- reference counts ----
    fn add_ref(&self, object: u64) {
        if object != 0 {
            cb!(self, add_ref, (), object)
        }
    }
    fn release(&self, object: u64) {
        if object != 0 {
            cb!(self, release, (), object)
        }
    }

    // ---- lots ----
    fn new_lot(&self, _x: i32, _z: i32, _w: i32, _h: i32, _facing: i32) -> u64 {
        0
    }
    fn lot_init(&self, _lot: u64) {}
    fn lot_shutdown(&self, _lot: u64) {}
    fn lot_post_city_init(&self, _lot: u64) {}
    fn lot_configuration(&self, lot: u64) -> Option<u64> {
        opt(cb!(self, lot_configuration, 0, lot))
    }
    fn lot_flags(&self, _lot: u64) -> u32 {
        0
    }
    fn lot_bounding_rect(&self, lot: u64) -> CellRect {
        self.rect_of(|out| cb!(self, lot_bounding_rect, (), lot, out))
    }
    fn lot_altitude(&self, _lot: u64) -> f32 {
        0.0
    }
    fn lot_location(&self, _lot: u64) -> (i32, i32) {
        (0, 0)
    }
    fn lot_facing(&self, _lot: u64) -> i32 {
        0
    }
    fn lot_has_business_unit(&self, _lot: u64) -> bool {
        false
    }
    fn lot_zone_type(&self, lot: u64) -> i32 {
        cb!(self, lot_zone_type, 0, lot)
    }
    fn lot_state(&self, lot: u64) -> u32 {
        cb!(self, lot_state, 0, lot)
    }
    fn lot_set_state(&self, _lot: u64, _state: u32, _a: u32, _b: u32) {}
    fn lot_building(&self, lot: u64) -> Option<u64> {
        opt(cb!(self, lot_building, 0, lot))
    }
    fn lot_set_vertices_locked(&self, _lot: u64, _locked: bool) {}
    fn lot_ref(&self, _lot: u64) -> RefValue {
        RefValue::Null
    }
    fn lot_from_ref(&self, _r: &RefValue) -> Option<u64> {
        None
    }

    // ---- lot configurations ----
    fn cfg_compatible_with_water(&self, _cfg: u64) -> bool {
        false
    }
    fn cfg_compatible_with_zone(&self, cfg: u64, zone: i32) -> bool {
        cb!(self, cfg_compatible_with_zone, false, cfg, zone)
    }
    fn cfg_id(&self, _cfg: u64) -> u32 {
        0
    }
    fn cfg_size(&self, _cfg: u64) -> (u32, u32) {
        (0, 0)
    }
    fn cfg_growth_stage(&self, _cfg: u64) -> u8 {
        0
    }
    fn cfg_compatible_with_wealth(&self, _cfg: u64, _wealth: u32) -> bool {
        false
    }
    fn cfg_compatible_with_purpose(&self, _cfg: u64, _purpose: u32) -> bool {
        false
    }
    fn cfg_primary_building_ids(&self, _cfg: u64) -> Option<Vec<u32>> {
        None
    }
    fn lot_configurations(&self) -> Vec<u64> {
        Vec::new()
    }

    // ---- lot developer ----
    fn area_slope(&self, rect: CellRect, with_water: bool) -> f32 {
        let r = fc4_cell_rect::from(rect);
        cb!(self, area_slope, 0.0, &r, with_water)
    }
    fn attempt_area_levelling(&self, _rect: CellRect) {}
    fn remove_building_zot_prop(&self, _lot: u64) {}
    fn lot_base_texture_occupant(&self, _lot: u64) -> Option<u64> {
        None
    }
    fn remove_lot_from_business_unit(&self, _lot: u64) {}
    fn set_empty_lot_base_texture(&self, lot: u64) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, set_empty_lot_base_texture, (), lot)
    }

    // ---- traffic simulator, BDS and growth developers (zone developer only) ----
    fn desired_lot_insertion_point(&self) -> Option<InsertionPoint> {
        None
    }
    fn set_lot_insertion_point(&self, _node: Option<u64>) {}
    fn bds_developer(&self, _dev_type: u32) -> Option<u64> {
        None
    }
    fn building_types_by_family(&self, _family: u32) -> Vec<u32> {
        Vec::new()
    }
    fn building_profile(&self, _building: u32) -> BuildingProfile {
        BuildingProfile::default()
    }
    fn dev_estimate_desirability(&self, _dev: u64, _x: i32, _z: i32) -> i16 {
        0
    }
    fn dev_source_demand(&self, _dev: u64) -> Option<f32> {
        None
    }
    fn dev_growth_list(&self, _dev: u64) -> Option<Vec<GrowthItem>> {
        None
    }
    fn dev_desirability_value(&self, _dev: u64, _tx: i32, _tz: i32) -> i16 {
        0
    }
    fn dev_desirability_shift(&self, _dev: u64) -> u32 {
        0
    }
    fn dev_purpose(&self, _dev: u64) -> u32 {
        0
    }
    fn dev_wealth(&self, _dev: u64) -> u32 {
        0
    }
}

// ---- the save stream --------------------------------------------------------------------------------

/// The record reader (§10.1): delegates to [`CIn`], except references. The
/// grid comes back as a host token with the one reference the core keeps;
/// the five service references (read in record order with their IIDs) are
/// handed to the host (`adopt_service`, live mode) and the core gets them
/// only to drop. FreeCity4's own objects (`fc4_ref::core_handle` set) come
/// without a reference: one is taken. In shadow mode every reference is
/// borrowed and nothing is taken, adopted or released.
struct ZIn<'a> {
    inner: CIn<'a>,
    host: &'a CHost,
    services: usize,
}

impl ZIn<'_> {
    fn raw(&mut self, iid: u32) -> Option<fc4_ref> {
        let f = self.inner.stream.get_ref?;
        let mut out = fc4_ref::default();
        // SAFETY: valid callback; `out` outlives the call.
        if !unsafe { f(self.inner.stream.user, iid, &mut out) } {
            return None;
        }
        if out.token != 0 && !out.core_handle.is_null() && !self.host.shadow {
            self.host.add_ref(out.token);
        }
        Some(out)
    }
    fn value(r: &fc4_ref) -> RefValue {
        match r.token {
            0 => RefValue::Null,
            token => RefValue::Host {
                token,
                class_id: r.class_id,
            },
        }
    }
}

impl InStream for ZIn<'_> {
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
    /// The service references (the only untyped references of the record).
    fn get_ref(&mut self) -> Option<RefValue> {
        let which = self.services;
        self.services += 1;
        let iid = SERVICE_IIDS.get(which).copied().unwrap_or(0);
        let r = self.raw(iid)?;
        if !self.host.shadow {
            if which < SERVICE_IIDS.len() {
                cb!(self.host, adopt_service, (), which as u32, r.token);
            } else {
                self.host.release(r.token);
            }
        }
        // The core only drops it; the host holds the service now.
        Some(Self::value(&r))
    }
    fn get_ref_as(&mut self, iid: u32) -> Option<RefValue> {
        let r = self.raw(iid)?;
        Some(Self::value(&r))
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

// ---- handles ----------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_zones`).
#[repr(C)]
pub struct fc4_zones {
    _private: [u8; 0],
}

struct Handle {
    host: Rc<CHost>,
    mgr: ZoneManager,
    depth: Cell<u32>,
    reentrant: Cell<u32>,
    place_calls: Cell<u32>,
    dezones: Cell<u32>,
    /// The last call's trace (diagnostics; `fc4_zones_last_trace`).
    last: RefCell<Trace>,
}

impl Handle {
    fn new(host: Rc<CHost>) -> Handle {
        let mgr = ZoneManager::new(host.clone());
        Handle {
            host,
            mgr,
            depth: Cell::new(0),
            reentrant: Cell::new(0),
            place_calls: Cell::new(0),
            dezones: Cell::new(0),
            last: RefCell::new(Trace::default()),
        }
    }

    /// Runs `f` with the call depth counted.
    fn call<R>(&self, f: impl FnOnce(&Handle) -> R) -> R {
        if self.depth.get() > 0 {
            self.reentrant.set(self.reentrant.get().wrapping_add(1));
        }
        self.depth.set(self.depth.get() + 1);
        let r = f(self);
        self.depth.set(self.depth.get() - 1);
        r
    }

    /// Runs `f` with a trace and an outLots sink of its own (a nested call
    /// neither sees nor disturbs its caller's); returns its trace.
    fn traced<R>(&self, sink: Option<Sink>, f: impl FnOnce(&Handle) -> R) -> (R, Trace) {
        let saved_trace = self.host.trace.replace(Trace::default());
        let saved_sink = self.host.sink.replace(sink);
        let r = f(self);
        self.host.sink.set(saved_sink);
        let mine = self.host.trace.replace(saved_trace);
        *self.last.borrow_mut() = mine;
        (r, mine)
    }
}

/// Runs `f` on the handle; the fallback for NULL or a panic.
fn with_handle<R>(p: *const fc4_zones, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_zones*.
    let Some(h) = (unsafe { (p as *const Handle).as_ref() }) else {
        return fallback;
    };
    guard(fallback, || f(h))
}

fn mgr<R>(p: *const fc4_zones, fallback: R, f: impl FnOnce(&ZoneManager) -> R) -> R {
    with_handle(p, fallback, |h| h.call(|h| f(&h.mgr)))
}

pub(crate) fn host_ok(host: *const fc4_zones_host) -> Option<fc4_zones_host> {
    // SAFETY: documented precondition: NULL or a valid host table.
    let h = unsafe { host.as_ref() }?;
    let ok = (h.struct_size as usize) >= std::mem::size_of::<fc4_zones_host>() && h.grids.complete();
    ok.then_some(*h)
}

/// `QueryInterface(iid)` (§1): whether the zone manager answers the IID (`1`,
/// `0x67652433`, `0x452294AA`, `0xE4FDA3D4`).
#[no_mangle]
pub extern "C" fn fc4_zones_query_interface(iid: u32) -> bool {
    manager_answers_iid(iid)
}

/// The class ID (`GetGZCLSID`, §1): `0x298F9B2D`.
#[no_mangle]
pub extern "C" fn fc4_zones_class_id() -> u32 {
    CLSID_ZONE_MANAGER
}

/// Slot 13 `IsRCIZone(type)` (§2): true for 1..9.
#[no_mangle]
pub extern "C" fn fc4_zones_is_rci_zone(zone_type: i32) -> bool {
    fc4_core::zones::is_rci_zone(zone_type)
}

fn create(host: *const fc4_zones_host, shadow: bool) -> *mut fc4_zones {
    guard(std::ptr::null_mut(), || {
        let Some(h) = host_ok(host) else {
            return std::ptr::null_mut();
        };
        Box::into_raw(Box::new(Handle::new(Rc::new(CHost::new(h, shadow))))) as *mut fc4_zones
    })
}

/// Creates the core object (the constructor's state, §3.1). The host table
/// is copied. NULL when the table is incomplete.
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_create(host: *const fc4_zones_host) -> *mut fc4_zones {
    create(host, false)
}

/// Creates a core object in shadow mode (see the module notes).
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_create_shadow(host: *const fc4_zones_host) -> *mut fc4_zones {
    create(host, true)
}

/// Destroys the core object. A grid still held (a loaded object never
/// initialised, or not shut down) is shut down and released.
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_destroy(p: *mut fc4_zones) {
    if p.is_null() {
        return;
    }
    guard((), || drop(Box::from_raw(p as *mut Handle)));
}

/// `Init` (§1.1): always true.
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_init(p: *mut fc4_zones) -> bool {
    mgr(p, false, ZoneManager::init)
}

/// `Shutdown` (§1.1): always true.
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_shutdown(p: *mut fc4_zones) -> bool {
    mgr(p, false, ZoneManager::shutdown)
}

/// `DoMessage` (§5.1): always true. `occupant` is void1 (any interface of
/// it; 0 = none). `out` (may be NULL) tells what the message did.
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_do_message(
    p: *mut fc4_zones,
    msg_type: u32,
    occupant: u64,
    data3: u32,
    data4: u32,
    out: *mut fc4_zones_dezone,
) -> bool {
    let m = ZonesMessage {
        id: msg_type,
        occupant: opt(occupant),
        data3,
        data4,
        ..Default::default()
    };
    let (r, trace) = with_handle(p, (true, Trace::default()), |h| {
        h.call(|h| {
            let (r, t) = h.traced(None, |h| h.mgr.do_message(&m));
            if t.bounding.is_some() {
                h.dezones.set(h.dezones.get().wrapping_add(1));
            }
            (r, t)
        })
    });
    if let Some(o) = out.as_mut() {
        *o = fc4_zones_dezone {
            struct_size: std::mem::size_of::<fc4_zones_dezone>() as u32,
            dezoned: u8::from(trace.bounding.is_some()),
            rect: trace.bounding.map(fc4_cell_rect::from).unwrap_or_default(),
            changed: trace.changed,
            lots: trace.demolished,
            ..Default::default()
        };
    }
    r
}

/// Slot 11 `PlaceZone` (§6). `region` is read, and written back with the
/// rejected cells cleared (a NULL mask cannot report them). `lots` (may be
/// NULL) receives every lot to demolish (`*outLots`, borrowed tokens; the
/// caller takes its own references), called once per lot at the pricing
/// call, before the commit. `out` (may be NULL) receives the outputs.
/// [choice] A NULL region or one larger than 1,048,576 cells (or a side
/// above 4096) fails with error 0 and cost 0, calling nothing.
#[no_mangle]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn fc4_zones_place_zone(
    p: *mut fc4_zones,
    region: *mut fc4_zone_region,
    zone_type: i32,
    commit: bool,
    skip_funds: bool,
    check_slope: bool,
    skip_size_check: bool,
    check_lot_inside: bool,
    lots: fc4_token_callback,
    ctx: *mut c_void,
    out: *mut fc4_zones_place_result,
) -> bool {
    let mut res = fc4_zones_place_result {
        struct_size: std::mem::size_of::<fc4_zones_place_result>() as u32,
        ..Default::default()
    };
    let parsed = region.as_mut().and_then(|r| region_in(r).map(|reg| (r, reg)));
    if let Some((raw, mut reg)) = parsed {
        let before = reg.count();
        let sink = Sink { cb: lots, ctx };
        let done = with_handle(p, None, |h| {
            h.call(|h| {
                h.place_calls.set(h.place_calls.get().wrapping_add(1));
                let (r, t) = h.traced(Some(sink), |h| {
                    h.mgr.place_zone(
                        &mut reg,
                        zone_type,
                        commit,
                        skip_funds,
                        check_slope,
                        skip_size_check,
                        check_lot_inside,
                    )
                });
                Some((r, t))
            })
        });
        if let Some((r, t)) = done {
            region_out(&reg, raw);
            let committed = r.ok && commit && t.posted > 0;
            res.ok = u8::from(r.ok);
            res.committed = u8::from(committed);
            res.error = r.error;
            res.cost = r.cost;
            res.lots = r.out_lots.len() as u32;
            res.accepted = if committed { t.changed } else { 0 };
            res.cleared = before.saturating_sub(reg.count()) as u32;
        }
    }
    if let Some(o) = out.as_mut() {
        *o = res;
    }
    res.ok != 0
}

/// Slot 5 `GetZoneGrid` (no AddRef): the grid token, 0 = none.
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_grid(p: *const fc4_zones) -> u64 {
    mgr(p, 0, |m| grid_token(&m.zone_grid()))
}

/// Slot 6 `GetZoneCount(type)`; 0 out of range.
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_zone_count(p: *const fc4_zones, zone_type: i32) -> u32 {
    mgr(p, 0, |m| m.zone_count(zone_type))
}

/// Slot 7 `GetUndevelopedTileCount(type)`.
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_undeveloped_tile_count(p: *const fc4_zones, zone_type: i32) -> u32 {
    mgr(p, 0, |m| m.undeveloped_tile_count(zone_type))
}

/// Slot 8 `GetUndevelopedTileCount()`.
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_undeveloped_total(p: *const fc4_zones) -> u32 {
    mgr(p, 0, ZoneManager::undeveloped_total)
}

/// Slot 9 `GetDevelopmentFailureCount(zone, result)`: inserts a zero entry
/// into map 1 when absent (§11.1 *[quirk]*).
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_development_failure_count(p: *mut fc4_zones, zone: i32, result: i32) -> u32 {
    mgr(p, 0, |m| m.development_failure_count(zone, result))
}

/// Slot 10 `GetAbandonedTileCount()`.
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_abandoned_tile_count(p: *const fc4_zones) -> u32 {
    mgr(p, 0, ZoneManager::abandoned_tile_count)
}

/// Slot 12 `GetZoneType(x, z)`: the cell's int8 value (0 outside the grid).
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_zone_type(p: *const fc4_zones, x: i32, z: i32) -> i32 {
    mgr(p, 0, |m| m.zone_type(x, z))
}

/// Slot 14 `GetTextureForZone(type, key&)`: false for `type ≥ 16` (the key
/// is not written); `key` = type, group, instance.
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_texture_for_zone(p: *const fc4_zones, zone_type: i32, key: *mut u32) -> bool {
    let Some(k) = mgr(p, None, |m| m.texture_for_zone(zone_type)) else {
        return false;
    };
    if !key.is_null() {
        *key = k.type_id;
        *key.add(1) = k.group;
        *key.add(2) = k.instance;
    }
    true
}

/// Slot 15 `GetZoneDragColor(type)`; 0 out of range.
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_drag_color(p: *const fc4_zones, zone_type: i32) -> u32 {
    mgr(p, 0, |m| m.zone_drag_color(zone_type))
}

/// Slot 16 `GetZoneDisplayAlpha()`.
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_display_alpha(p: *const fc4_zones) -> u8 {
    mgr(p, 0xFF, ZoneManager::zone_display_alpha)
}

/// Slot 17 `GetDefaultZoneDisplayAlpha()`.
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_default_display_alpha(p: *const fc4_zones) -> u8 {
    mgr(p, 0xFF, ZoneManager::default_zone_display_alpha)
}

/// Slot 18 `SetZoneDisplayAlpha(a)`: when it changes, every lot of the city
/// in state 1 gets `SetEmptyLotBaseTexture` (§11.1).
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_set_display_alpha(p: *mut fc4_zones, alpha: u8) {
    mgr(p, (), |m| m.set_zone_display_alpha(alpha))
}

/// Slot 19 `GetMinZoneSize(type)`; 0 out of range.
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_min_size(p: *const fc4_zones, zone_type: i32) -> i32 {
    mgr(p, 0, |m| m.min_zone_size(zone_type))
}

/// Slot 20 `GetMaxZoneSize(type)`; 0 out of range.
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_max_size(p: *const fc4_zones, zone_type: i32) -> i32 {
    mgr(p, 0, |m| m.max_zone_size(zone_type))
}

/// The state (diagnostics and the getters' snapshot). Does not touch the
/// failure maps.
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_get_state(p: *const fc4_zones, out: *mut fc4_zones_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_handle(p, false, |h| {
        let m = &h.mgr;
        let mut s = fc4_zones_state {
            struct_size: std::mem::size_of::<fc4_zones_state>() as u32,
            initialized: u8::from(m.is_initialised()),
            loaded: u8::from(m.is_loaded()),
            display_alpha: m.zone_display_alpha(),
            default_alpha: m.default_zone_display_alpha(),
            undeveloped_total: m.undeveloped_total(),
            abandoned_tile_count: m.abandoned_tile_count(),
            failure1_entries: m.failure_map1().len() as u32,
            reentrant_calls: h.reentrant.get(),
            place_calls: h.place_calls.get(),
            dezones: h.dezones.get(),
            ..Default::default()
        };
        for t in 0..16 {
            s.zone_count[t] = m.zone_count(t as i32);
            s.undeveloped[t] = m.undeveloped_tile_count(t as i32);
        }
        let grid = m.zone_grid();
        s.grid = grid_token(&grid);
        if let Some(g) = grid {
            s.grid_count_x = g.count_x();
            s.grid_count_z = g.count_z();
        }
        *o = s;
        true
    })
}

/// The tuning (§4) and both alphas.
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_get_tuning(p: *const fc4_zones, out: *mut fc4_zones_tuning) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_handle(p, false, |h| {
        let t = h.mgr.tuning();
        *o = fc4_zones_tuning {
            struct_size: std::mem::size_of::<fc4_zones_tuning>() as u32,
            display_alpha: t.display_alpha,
            default_alpha: h.mgr.default_zone_display_alpha(),
            reserved: [0; 2],
            min_size: t.min_size,
            max_size: t.max_size,
            create_cost: t.create_cost,
            destroy_cost: t.destroy_cost,
            texture: t.texture,
            drag_color: t.drag_color,
            max_slope: t.max_slope,
        };
        true
    })
}

/// The game writes the last `fc4_zones_place_zone` / `fc4_zones_do_message`
/// call swallowed (shadow mode; 0 in live mode).
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_last_swallowed(p: *const fc4_zones) -> u32 {
    with_handle(p, 0, |h| h.last.borrow().swallowed)
}

/// `Write` (§10.1). The grid and service references go out as `fc4_ref`s
/// with the token and class 0 (the grid: its class).
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_write(p: *const fc4_zones, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    mgr(p, false, |m| m.write_record(&mut COut(o)))
}

/// `Read` (§10.1). Runs before `Init`: nothing is sized from the city here.
#[no_mangle]
pub unsafe extern "C" fn fc4_zones_read(p: *mut fc4_zones, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_handle(p, false, |h| {
        let host = h.host.clone();
        h.call(|h| {
            h.mgr.read_record(&mut ZIn {
                inner: CIn {
                    stream: i,
                    decode: no_core_objects,
                },
                host: &host,
                services: 0,
            })
        })
    })
}
