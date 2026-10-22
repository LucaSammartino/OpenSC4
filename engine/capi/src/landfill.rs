// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The landfill developer's C ABI (fc4.h "landfill", docs/specs/landfill.md).
//!
//! The host implements [`fc4_landfill_host`] (every `LandfillHost` method of
//! the core that reaches the game, plus two pollution reads); the plugin's
//! `cISC4LandfillDeveloper` (with its message-target and serializable
//! sub-objects) forwards its slots to the `fc4_landfill_*` functions.
//!
//! Tokens are opaque host identities (fc4.h): the sanitation department, the
//! zot props and every game occupant by its `cISC4Occupant` identity. 0 is
//! the null object. The core owns one reference on the department (taken on
//! PostCityInit) and on every zot (taken when it is placed, or handed over by
//! the save stream); FreeCity4's own objects read back from a save
//! (`fc4_ref::core_handle` set) are given one here, so both cases end up the
//! same.
//!
//! **Landfill tiles** (§4.8.1 step 3, LG3): the core builds each
//! `cSC4LandfillOccupant` and hands it over to be inserted into the occupant
//! manager. With the host's `insert_landfill_tile` (live mode) the host
//! builds the GAME occupant from the tile ID, orientation and position, with
//! the tile exemplar attached (§4.8.1's [choice] route through public
//! interfaces), and inserts it; from then on the tile is a game occupant like
//! one the original made: found through `occupant_at`, priced from its tile
//! exemplar (§4.9.1, LG11), removed through `occupant_remove`, and saved by
//! the occupant manager (§6.4, LG8). Without the callback, when it makes no
//! occupant, and always in shadow mode, the tile lives in a registry here
//! under a token with bit 63 set, which every occupant callback of the core
//! understands (`occupant_at` of the landfill type finds it first, removal,
//! position, flags, the item cost of §4.9.1); the game then sees no tile.
//!
//! **Tiles across a load** (§6.4, LG8): the original saves its tiles and does
//! not rebuild them, and neither does this layer for a cell that has one.
//! [choice] A mask cell whose §4.8 rule gives a tile but which has no
//! landfill occupant (LG23) (a city saved by an older FreeCity4, which kept its
//! tiles here) gets its tile back on PostCityInit, before the core's own
//! handling, chosen as §4.8 would from the saved heaps with orientation 0 and
//! no `rand()` draw ([`Handle::rebuild_tiles`]).
//!
//! Re-entrancy: the core holds no borrow across a host callback. A message
//! delivered while a call is running is queued and handled when the outermost
//! call returns ([choice], as plumbing); other re-entrant calls run at once
//! and are counted (`fc4_landfill_state::reentrant_calls`).
//!
//! Threads: the core is single-threaded (`RefCell`). The plugin serialises
//! every call on one object with its own lock (fc4.h).
//!
//! The shadow (`landfill_shadow.rs`) runs a developer of its own over the same
//! table in **shadow mode**: every write is recorded instead of reaching the
//! game ([`Rec`]), reads go to the game, and no reference is taken or
//! released.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{c_char, c_void};
use std::rc::Rc;

use fc4_core::landfill::{
    msg, neighbour_mask, tile, tile_of, CellRect, CellRegion, LandfillDeveloper, LandfillHost, LandfillMessage,
    LandfillOccupant, ZoneEffects, CLSID_LANDFILL_DEVELOPER, IID_OCCUPANT, OCCUPANT_TYPE_LANDFILL,
};
use fc4_core::property::{ExemplarHolder, PropertyHolder};
use fc4_core::stream::{InStream, RefValue, ResKey, SaveObject};

use crate::budget::fc4_token_callback;
use crate::guard;
use crate::hostgrid::{collect_tokens, take_props};
use crate::props::fc4_props;
use crate::stream::{fc4_istream, fc4_ostream, fc4_ref, istream_ok, ostream_ok, CIn, COut};

type U = *mut c_void;

/// A cell region (fc4.h `fc4_landfill_region`): the game's
/// `SC4CellRegion<long>` as a rectangle plus one byte per cell.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct fc4_landfill_region {
    /// The inclusive rectangle.
    pub x0: i32,
    pub z0: i32,
    pub x1: i32,
    pub z1: i32,
    /// `(x1 − x0 + 1) × (z1 − z0 + 1)` bytes, index `(z − z0) × width +
    /// (x − x0)`, non-zero = a mask cell. NULL: every cell is set. A short
    /// mask is padded with cleared cells.
    pub mask: *const u8,
    pub len: usize,
}

/// The zone effects the host created (fc4.h `fc4_landfill_effects`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_landfill_effects {
    /// The garbage-truck automaton (0 = none), one reference for the core.
    pub truck: u64,
    /// The seagull effect (0 = none), one reference for the core.
    pub seagull: u64,
}

/// Diagnostics (fc4.h `fc4_landfill_state`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct fc4_landfill_state {
    pub struct_size: u32,
    pub initialized: u8,
    pub loaded: u8,
    /// 1 when the last SimNewMonth reported a free capacity
    /// (`SetGarbageCapacity`).
    pub capacity_reported: u8,
    pub reserved0: u8,
    pub zones: u32,
    pub connected_zones: u32,
    /// Mask cells over all zones.
    pub cells: u32,
    pub heaps: u32,
    pub valid_heaps: u32,
    pub active_heaps: u32,
    /// Zones with a zot prop.
    pub zots: u32,
    /// Landfill tiles kept here (see the module notes).
    pub virtual_tiles: u32,
    /// Cells whose active-landfill flag this object set and did not clear
    /// (plus the cells found flagged when tiles were rebuilt).
    pub active_cells: u32,
    pub reentrant_calls: u32,
    pub capacity_total: u32,
    pub used_total: u32,
    /// The last value given to `SetGarbageCapacity`.
    pub free_capacity: i32,
    /// Tiles rebuilt at the last PostCityInit after a load.
    pub rebuilt_tiles: u32,
    /// Garbage stored: Σ contained over every heap of every zone.
    pub stored: u64,
    /// The last SimNewMonth: Σ `GarbagePutInLandfill`.
    pub moved: i64,
    /// The last SimNewMonth: Σ `AddToGarbageSupply`.
    pub supply_delta: i64,
    /// `maintenance` (slot 7).
    pub maintenance: i64,
    /// The sanitation department held (borrowed token; 0 = none).
    pub department: u64,
    /// Tiles inserted into the game as `cSC4LandfillOccupant`s through
    /// `insert_landfill_tile` since the object was created (§4.8.1, LG3).
    pub game_tiles: u32,
    /// `insert_landfill_tile` calls that made no game tile (the tile was kept
    /// here instead).
    pub game_tile_failures: u32,
}

/// Everything the landfill developer asks of the game (fc4.h
/// `fc4_landfill_host`; one callback per `LandfillHost` method that reaches
/// the game, in the trait's order). Tokens returned by the host are
/// borrowed unless a callback says otherwise.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct fc4_landfill_host {
    pub struct_size: u32,
    pub user: *mut c_void,
    // lifecycle (§1.1, §1.2)
    pub acquire_services: Option<unsafe extern "C" fn(U)>,
    pub release_services: Option<unsafe extern "C" fn(U)>,
    /// (level, name, name length, flags).
    pub add_agent: Option<unsafe extern "C" fn(U, u32, *const c_char, usize, u32)>,
    pub remove_agent: Option<unsafe extern "C" fn(U)>,
    pub subscribe: Option<unsafe extern "C" fn(U, u32)>,
    pub unsubscribe: Option<unsafe extern "C" fn(U, u32)>,
    /// A NEW fc4_props (the core destroys it), NULL if the exemplar is missing.
    pub tuning: Option<unsafe extern "C" fn(U) -> *mut fc4_props>,
    /// (type, group, instance): a NEW fc4_props, NULL if missing.
    pub exemplar: Option<unsafe extern "C" fn(U, u32, u32, u32) -> *mut fc4_props>,
    // city (slots 93, 94, 97)
    pub cell_counts: Option<unsafe extern "C" fn(U, *mut i32, *mut i32)>,
    pub cell_center_to_position: Option<unsafe extern "C" fn(U, i32, i32, *mut f32, *mut f32)>,
    // terrain (slots 13, 19, 29, 49, 68)
    /// `GetAltitude(x, z)` at a world position.
    pub altitude: Option<unsafe extern "C" fn(U, f32, f32) -> f32>,
    pub altitude_at_nearest_grid: Option<unsafe extern "C" fn(U, f32, f32) -> f32>,
    pub sea_level: Option<unsafe extern "C" fn(U) -> f32>,
    /// (x1, z1, x2, z2, altitude).
    pub set_altitude: Option<unsafe extern "C" fn(U, i32, i32, i32, i32, f32)>,
    pub redisplay_terrain: Option<unsafe extern "C" fn(U, i32, i32, i32, i32)>,
    // pollution simulator (slots 20, 21, 24, 29, 30, 50, 51; reads 22, 49)
    pub garbage_supply: Option<unsafe extern "C" fn(U) -> u32>,
    pub landfill_tile_capacity: Option<unsafe extern "C" fn(U) -> u32>,
    pub total_garbage_uncollected: Option<unsafe extern "C" fn(U) -> i32>,
    pub add_to_garbage_supply: Option<unsafe extern "C" fn(U, i32)>,
    pub set_garbage_capacity: Option<unsafe extern "C" fn(U, i32)>,
    pub set_is_active_landfill: Option<unsafe extern "C" fn(U, i32, i32, bool)>,
    pub garbage_put_in_landfill: Option<unsafe extern "C" fn(U, i32)>,
    /// Slot 22 `GetGarbageCapacity()` (the shadow's comparison).
    pub garbage_capacity: Option<unsafe extern "C" fn(U) -> i32>,
    /// Slot 49 `IsActiveLandfill(x, z)` (the tile rebuild and the shadow).
    pub is_active_landfill: Option<unsafe extern "C" fn(U, i32, i32) -> bool>,
    // budget (§4.9)
    /// `GetDepartmentBudget(id)`: borrowed; 0 = none.
    pub get_department_budget: Option<unsafe extern "C" fn(U, u32) -> u64>,
    pub add_ref: Option<unsafe extern "C" fn(U, u64)>,
    pub release: Option<unsafe extern "C" fn(U, u64)>,
    pub department_funding_percentage: Option<unsafe extern "C" fn(U, u64) -> f32>,
    /// (department, line, out): false without the line item.
    pub line_item_current_expenses: Option<unsafe extern "C" fn(U, u64, u32, *mut i64) -> bool>,
    pub set_line_item_full_expenses: Option<unsafe extern "C" fn(U, u64, u32, i64)>,
    /// (occupant, property, out): the first value of the property of the
    /// occupant's property holder; false when absent.
    pub occupant_item_cost: Option<unsafe extern "C" fn(U, u64, u32, *mut i64) -> bool>,
    // zone manager (slots 6, 11, 12)
    pub zone_count: Option<unsafe extern "C" fn(U, i32) -> i32>,
    pub zone_type: Option<unsafe extern "C" fn(U, i32, i32) -> i32>,
    pub place_zone: Option<unsafe extern "C" fn(U, *const fc4_landfill_region, i32) -> bool>,
    // occupants (game occupants only; the tiles kept here never reach these)
    /// (x, z, occupant type): borrowed; 0 = none.
    pub occupant_at: Option<unsafe extern "C" fn(U, i32, i32, u32) -> u64>,
    pub occupant_remove: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub occupant_changed: Option<unsafe extern "C" fn(U, u64)>,
    /// World x, y, z.
    pub occupant_position: Option<unsafe extern "C" fn(U, u64, *mut f32)>,
    pub occupant_set_position: Option<unsafe extern "C" fn(U, u64, *const f32)>,
    pub occupant_set_removal_flags: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub occupant_post_message: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub is_landfill_occupant: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub is_empty_landfill_tile: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub remove_incompatible_occupants: Option<unsafe extern "C" fn(U, i32, i32)>,
    pub set_empty_cell_texture: Option<unsafe extern "C" fn(U, i32, i32)>,
    pub remove_base_texture: Option<unsafe extern "C" fn(U, i32, i32)>,
    // prop manager (slots 26, 29)
    /// (prop ID, world x/y/z, orientation): the prop occupant, borrowed (the
    /// core `AddRef`s it); 0 = failed.
    pub add_lot_prop: Option<unsafe extern "C" fn(U, u32, *const f32, i32) -> u64>,
    pub remove_prop: Option<unsafe extern "C" fn(U, u64) -> bool>,
    pub zot_orientation: Option<unsafe extern "C" fn(U) -> i32>,
    // traffic
    /// (x, z, flag mask, flag): borrowed; 0 = none.
    pub find_network_tile: Option<unsafe extern "C" fn(U, i32, i32, u32, bool) -> u64>,
    /// `cb(ctx, id)` once per subnetwork ID.
    pub subnetworks_in_region: Option<unsafe extern "C" fn(U, *const fc4_landfill_region, fc4_token_callback, U)>,
    /// `cb(ctx, count)` once per subnetwork, in ID order.
    pub occupant_count_for_all_subnetworks: Option<unsafe extern "C" fn(U, u32, fc4_token_callback, U)>,
    // lot developer (slot 21)
    /// (x1, z1, x2, z2, f, with water, water flag, max area slope).
    pub attempt_area_levelling: Option<unsafe extern "C" fn(U, i32, i32, i32, i32, f32, bool, bool, f32) -> bool>,
    // 3D view
    pub current_view_level: Option<unsafe extern "C" fn(U) -> u32>,
    // zone effects (§4.10; rendering only)
    /// (centre x, z, rect x1, z1, x2, z2, out).
    pub init_zone_effects: Option<unsafe extern "C" fn(U, i32, i32, i32, i32, i32, i32, *mut fc4_landfill_effects)>,
    /// (truck, seagull, seagull started): stop and release.
    pub shutdown_zone_effects: Option<unsafe extern "C" fn(U, u64, u64, bool)>,
    pub set_truck_enabled: Option<unsafe extern "C" fn(U, u64, bool)>,
    pub start_seagull: Option<unsafe extern "C" fn(U, u64, u32)>,
    pub stop_seagull: Option<unsafe extern "C" fn(U, u64, u32)>,
    // history (§5)
    /// (series, value): `MessageSend` of `0x89EFA536`.
    pub send_history: Option<unsafe extern "C" fn(U, u32, u32)>,
    // tiles (§4.8.1 step 3, LG3; appended in ABI 33)
    /// (x, z, tile ID, orientation, world x/y/z): build a game
    /// `cSC4LandfillOccupant` with the tile exemplar attached, flag 4,
    /// removal flags 2 and that position, `Init` it and insert it into the
    /// occupant manager. Its `cISC4Occupant` token (borrowed: the occupant
    /// manager holds it), 0 when none was made (the tile is then kept here).
    pub insert_landfill_tile: Option<unsafe extern "C" fn(U, i32, i32, u32, i32, *const f32) -> u64>,
}

const _: () = {
    assert!(std::mem::size_of::<fc4_landfill_region>() == if cfg!(target_pointer_width = "32") { 24 } else { 32 });
    assert!(std::mem::size_of::<fc4_landfill_effects>() == 16);
    assert!(std::mem::size_of::<fc4_landfill_state>() == 112);
    assert!(std::mem::size_of::<fc4_landfill_host>() == if cfg!(target_pointer_width = "32") { 252 } else { 504 });
};

/// Token bit of the tiles kept here (host tokens are pointers).
pub(crate) const VIRTUAL_TILE: u64 = 1 << 63;

/// The most cells a region crossing the ABI may have (a 4096 × 256 rect).
const MAX_REGION_CELLS: i64 = 1 << 20;

/// The region a host hands in; `None` when it is NULL, empty or too large.
pub(crate) unsafe fn region_of(r: *const fc4_landfill_region) -> Option<CellRegion> {
    let r = r.as_ref()?;
    let w = i64::from(r.x1) - i64::from(r.x0) + 1;
    let h = i64::from(r.z1) - i64::from(r.z0) + 1;
    if w <= 0 || h <= 0 || w * h > MAX_REGION_CELLS {
        return None;
    }
    let rect = CellRect::new(r.x0, r.z0, r.x1, r.z1);
    if r.mask.is_null() {
        return Some(CellRegion::all_set(rect));
    }
    let n = r.len.min((w * h) as usize);
    let bytes = if n == 0 {
        &[][..]
    } else {
        std::slice::from_raw_parts(r.mask, n)
    };
    Some(CellRegion::from_mask(rect, bytes.iter().map(|&b| b != 0).collect()))
}

/// A region for a host call: the mask bytes and the struct pointing at them
/// (keep the bytes alive while the struct is used).
fn region_raw(r: &CellRegion) -> (Vec<u8>, fc4_landfill_region) {
    let bytes: Vec<u8> = r.mask().iter().map(|&b| u8::from(b)).collect();
    let raw = fc4_landfill_region {
        x0: r.x0,
        z0: r.z0,
        x1: r.x1,
        z1: r.z1,
        mask: bytes.as_ptr(),
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

/// The tiles kept here (see the module notes).
#[derive(Default)]
struct Tiles {
    next: u64,
    by_cell: BTreeMap<(i32, i32), u64>,
    by_token: BTreeMap<u64, (Rc<LandfillOccupant>, (i32, i32))>,
}

/// What the developer did since the last reset: per SimNewMonth for the
/// status line, per run for the shadow. The `active`, `hidden` and swallowed
/// counts are only kept in shadow mode, where these writes never reach the
/// game.
#[derive(Clone, Debug, Default)]
pub(crate) struct Rec {
    /// Σ `AddToGarbageSupply`.
    pub supply_delta: i64,
    /// Σ `GarbagePutInLandfill` and the number of calls.
    pub put_in_landfill: i64,
    pub put_calls: u32,
    /// The last `SetGarbageCapacity`.
    pub capacity: Option<i32>,
    /// The last `SetFullExpenses` of line 30.
    pub expenses: Option<i64>,
    /// Shadow: the `SetIsActiveLandfill` writes, last one per cell.
    pub active: BTreeMap<(i32, i32), bool>,
    /// Shadow: game occupants removed (hidden from later reads).
    pub hidden: BTreeSet<u64>,
    pub zots_added: u32,
    pub zots_failed: u32,
    pub zots_removed: u32,
    pub place_zone: u32,
    pub history: Vec<(u32, u32)>,
    pub tiles_created: u32,
    pub tiles_removed: u32,
    /// Shadow: other game writes swallowed (terrain, textures, flora,
    /// levelling, occupant moves and messages, effects).
    pub swallowed: u32,
}

pub(crate) struct CHost {
    c: fc4_landfill_host,
    /// Shadow mode (see the module notes).
    shadow: bool,
    tiles: RefCell<Tiles>,
    rec: RefCell<Rec>,
    /// Live: the cells this object flagged (and the rebuild found flagged).
    active: RefCell<BTreeSet<(i32, i32)>>,
    /// Tiles inserted into the game, and `insert_landfill_tile` calls that
    /// made none (fc4_landfill_state).
    game_tiles: Cell<u32>,
    game_tile_failures: Cell<u32>,
}

impl CHost {
    pub(crate) fn new(c: fc4_landfill_host, shadow: bool) -> CHost {
        CHost {
            c,
            shadow,
            tiles: RefCell::new(Tiles::default()),
            rec: RefCell::new(Rec::default()),
            active: RefCell::new(BTreeSet::new()),
            game_tiles: Cell::new(0),
            game_tile_failures: Cell::new(0),
        }
    }

    /// Puts a tile the core built (§4.8.1 step 3) into its cell: a game
    /// occupant through `insert_landfill_tile` (live mode, LG3), or the
    /// registry here (no callback, the callback made none, or shadow mode).
    fn place_tile(&self, occ: &Rc<LandfillOccupant>, x: i32, z: i32) {
        if !self.shadow && self.c.insert_landfill_tile.is_some() {
            let p = occ.position();
            let token = cb!(
                self,
                insert_landfill_tile,
                0,
                x,
                z,
                occ.tile_id(),
                occ.orientation(),
                p.as_ptr()
            );
            if token != 0 && token & VIRTUAL_TILE == 0 {
                occ.set_token(token);
                self.game_tiles.set(self.game_tiles.get().wrapping_add(1));
                return;
            }
            self.game_tile_failures
                .set(self.game_tile_failures.get().wrapping_add(1));
        }
        self.insert_tile(occ, x, z);
    }

    pub(crate) fn rec(&self) -> Rec {
        self.rec.borrow().clone()
    }

    /// Clears the recorder and (shadow) the tiles created by the last run.
    pub(crate) fn reset(&self) {
        *self.rec.borrow_mut() = Rec::default();
        if self.shadow {
            *self.tiles.borrow_mut() = Tiles::default();
        }
    }

    pub(crate) fn tile_count(&self) -> usize {
        self.tiles.borrow().by_token.len()
    }

    fn active_count(&self) -> usize {
        self.active.borrow().len()
    }

    /// The tile at a cell (tile ID, orientation), if one is kept here.
    fn tile_at(&self, x: i32, z: i32) -> Option<(u32, i32)> {
        let t = self.tiles.borrow();
        let token = *t.by_cell.get(&(x, z))?;
        t.by_token.get(&token).map(|(o, _)| (o.tile_id(), o.orientation()))
    }

    fn virt(&self, token: u64) -> Option<Rc<LandfillOccupant>> {
        if token & VIRTUAL_TILE == 0 {
            return None;
        }
        self.tiles.borrow().by_token.get(&token).map(|(o, _)| o.clone())
    }

    fn insert_tile(&self, occ: &Rc<LandfillOccupant>, x: i32, z: i32) {
        let old = {
            let mut t = self.tiles.borrow_mut();
            t.next += 1;
            let token = VIRTUAL_TILE | t.next;
            occ.set_token(token);
            let old = t.by_cell.insert((x, z), token);
            let old = old.and_then(|o| t.by_token.remove(&o)).map(|(o, _)| o);
            t.by_token.insert(token, (occ.clone(), (x, z)));
            old
        };
        // A tile still in the cell (the core removes it first) is replaced.
        if let Some(o) = old {
            o.shutdown();
        }
    }

    fn remove_tile(&self, token: u64) -> bool {
        let removed = {
            let mut t = self.tiles.borrow_mut();
            match t.by_token.remove(&token) {
                Some((o, cell)) => {
                    if t.by_cell.get(&cell) == Some(&token) {
                        t.by_cell.remove(&cell);
                    }
                    Some(o)
                }
                None => None,
            }
        };
        match removed {
            Some(o) => {
                o.shutdown();
                true
            }
            None => false,
        }
    }

    pub(crate) fn live_supply(&self) -> u32 {
        cb!(self, garbage_supply, 0)
    }

    pub(crate) fn live_capacity(&self) -> Option<i32> {
        self.c.garbage_capacity.map(|_| cb!(self, garbage_capacity, 0))
    }

    /// Pollution's active-landfill flag; `None` without the read callback.
    pub(crate) fn live_active(&self, x: i32, z: i32) -> Option<bool> {
        self.c
            .is_active_landfill
            .map(|_| cb!(self, is_active_landfill, false, x, z))
    }

    fn swallow(&self) {
        self.rec.borrow_mut().swallowed += 1;
    }

    fn hidden(&self, token: u64) -> bool {
        self.shadow && self.rec.borrow().hidden.contains(&token)
    }

    /// A new tile occupant for a cell, as §4.8.1 step 3 builds it (without
    /// the pollution flag and the insertion).
    fn build_tile(&self, x: i32, z: i32, tile_id: u32, orientation: i32) -> Rc<LandfillOccupant> {
        let occ = LandfillOccupant::new(tile_id, orientation);
        if let Some(ex) = self.exemplar(tile::TYPE, tile::GROUP, tile_id) {
            let key = ResKey {
                type_id: tile::TYPE,
                group: tile::GROUP,
                instance: tile_id,
            };
            occ.attach_exemplar(ExemplarHolder {
                parent: Some((key, ex)),
                ..Default::default()
            });
        }
        occ.set_removal_flags(2);
        occ.set_empty(false);
        let (px, pz) = self.cell_center_to_position(x, z);
        let y = self.cell_altitude(x, z);
        occ.set_position([px, y, pz]);
        occ.init();
        occ
    }
}

impl LandfillHost for CHost {
    fn acquire_services(&self) {
        if !self.shadow {
            cb!(self, acquire_services, ())
        }
    }
    fn release_services(&self) {
        if !self.shadow {
            cb!(self, release_services, ())
        }
    }
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        if !self.shadow {
            cb!(
                self,
                add_agent,
                (),
                level,
                name.as_ptr() as *const c_char,
                name.len(),
                flags
            )
        }
    }
    fn remove_agent(&self) {
        if !self.shadow {
            cb!(self, remove_agent, ())
        }
    }
    fn subscribe(&self, msg_type: u32) {
        if !self.shadow {
            cb!(self, subscribe, (), msg_type)
        }
    }
    fn unsubscribe(&self, msg_type: u32) {
        if !self.shadow {
            cb!(self, unsubscribe, (), msg_type)
        }
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, tuning, std::ptr::null_mut());
        // SAFETY: the host hands over a NEW fc4_props (or NULL).
        unsafe { take_props(p) }
    }
    fn exemplar(&self, type_id: u32, group: u32, instance: u32) -> Option<Rc<PropertyHolder>> {
        let p = cb!(self, exemplar, std::ptr::null_mut(), type_id, group, instance);
        // SAFETY: as above.
        unsafe { take_props(p) }
    }

    fn cell_counts(&self) -> (i32, i32) {
        let (mut x, mut z) = (0, 0);
        cb!(self, cell_counts, (), &mut x, &mut z);
        (x, z)
    }
    fn cell_center_to_position(&self, x: i32, z: i32) -> (f32, f32) {
        let (mut px, mut pz) = (0.0f32, 0.0f32);
        cb!(self, cell_center_to_position, (), x, z, &mut px, &mut pz);
        (px, pz)
    }

    fn cell_altitude(&self, x: i32, z: i32) -> f32 {
        let (px, pz) = self.cell_center_to_position(x, z);
        cb!(self, altitude, 0.0, px, pz)
    }
    fn altitude_at_nearest_grid(&self, px: f32, pz: f32) -> f32 {
        cb!(self, altitude_at_nearest_grid, 0.0, px, pz)
    }
    fn cell_altitude_at_nearest_grid(&self, x: i32, z: i32) -> f32 {
        let (px, pz) = self.cell_center_to_position(x, z);
        self.altitude_at_nearest_grid(px, pz)
    }
    fn sea_level(&self) -> f32 {
        cb!(self, sea_level, 0.0)
    }
    fn set_altitude(&self, rect: CellRect, altitude: f32) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, set_altitude, (), rect.x1, rect.z1, rect.x2, rect.z2, altitude)
    }
    fn redisplay_terrain(&self, rect: CellRect) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, redisplay_terrain, (), rect.x1, rect.z1, rect.x2, rect.z2)
    }

    fn garbage_supply(&self) -> u32 {
        let live = self.live_supply();
        if !self.shadow {
            return live;
        }
        // [choice] The supply as the original's run would see it: the live
        // value plus our own changes, clamped to the u32 range. LG12/LG14
        // (§4.7.1): the month's intakes sum to at most the supply, so the
        // clamp never engages except for a negative supply from a damaged
        // save, and "live plus delta" equals the original's.
        let v = i64::from(live) + self.rec.borrow().supply_delta;
        v.clamp(0, i64::from(u32::MAX)) as u32
    }
    fn landfill_tile_capacity(&self) -> u32 {
        cb!(self, landfill_tile_capacity, 0)
    }
    fn total_garbage_uncollected(&self) -> i32 {
        cb!(self, total_garbage_uncollected, 0)
    }
    fn add_to_garbage_supply(&self, delta: i32) {
        self.rec.borrow_mut().supply_delta += i64::from(delta);
        if !self.shadow {
            cb!(self, add_to_garbage_supply, (), delta)
        }
    }
    fn set_garbage_capacity(&self, available: i32) {
        self.rec.borrow_mut().capacity = Some(available);
        if !self.shadow {
            cb!(self, set_garbage_capacity, (), available)
        }
    }
    fn set_is_active_landfill(&self, x: i32, z: i32, on: bool) {
        if self.shadow {
            self.rec.borrow_mut().active.insert((x, z), on);
            return;
        }
        if on {
            self.active.borrow_mut().insert((x, z));
        } else {
            self.active.borrow_mut().remove(&(x, z));
        }
        cb!(self, set_is_active_landfill, (), x, z, on)
    }
    fn garbage_put_in_landfill(&self, n: i32) {
        {
            let mut r = self.rec.borrow_mut();
            r.put_in_landfill += i64::from(n);
            r.put_calls += 1;
        }
        if !self.shadow {
            cb!(self, garbage_put_in_landfill, (), n)
        }
    }

    fn get_department_budget(&self, department: u32) -> Option<u64> {
        opt(cb!(self, get_department_budget, 0, department))
    }
    // The shadow takes and releases no reference: its tokens are borrowed for
    // the delivery.
    fn add_ref(&self, token: u64) {
        if token != 0 && !self.shadow {
            cb!(self, add_ref, (), token)
        }
    }
    fn release(&self, token: u64) {
        if token != 0 && !self.shadow {
            cb!(self, release, (), token)
        }
    }
    fn department_funding_percentage(&self, department: u64) -> f32 {
        cb!(self, department_funding_percentage, 1.0, department)
    }
    fn line_item_current_expenses(&self, department: u64, line: u32) -> Option<i64> {
        let mut v = 0i64;
        cb!(self, line_item_current_expenses, false, department, line, &mut v).then_some(v)
    }
    fn set_line_item_full_expenses(&self, department: u64, line: u32, value: i64) {
        self.rec.borrow_mut().expenses = Some(value);
        if !self.shadow {
            cb!(self, set_line_item_full_expenses, (), department, line, value)
        }
    }
    fn occupant_item_cost(&self, occupant: u64, property: u32) -> Option<i64> {
        if let Some(o) = self.virt(occupant) {
            // [choice] The tile's own holder, then its exemplar (§4.9.1: "the
            // first s64 value").
            let snap = o.snapshot();
            return snap
                .base
                .holder
                .get(property)
                .and_then(|v| v.items().first().map(|e| e.as_i64()));
        }
        let mut v = 0i64;
        cb!(self, occupant_item_cost, false, occupant, property, &mut v).then_some(v)
    }

    fn zone_count(&self, zone_type: i32) -> i32 {
        cb!(self, zone_count, 0, zone_type)
    }
    fn zone_type(&self, x: i32, z: i32) -> i32 {
        cb!(self, zone_type, 0, x, z)
    }
    fn place_zone(&self, region: &CellRegion, zone_type: i32) -> bool {
        self.rec.borrow_mut().place_zone += 1;
        if self.shadow {
            return true;
        }
        let (_bytes, raw) = region_raw(region);
        cb!(self, place_zone, false, &raw, zone_type)
    }

    fn occupant_at(&self, x: i32, z: i32, occupant_type: u32) -> Option<u64> {
        if occupant_type == OCCUPANT_TYPE_LANDFILL {
            if let Some(&t) = self.tiles.borrow().by_cell.get(&(x, z)) {
                return Some(t);
            }
        }
        let t = opt(cb!(self, occupant_at, 0, x, z, occupant_type))?;
        (!self.hidden(t)).then_some(t)
    }
    // [choice] Always true: a tile the game did not take is kept here.
    fn insert_landfill_occupant(&self, occupant: &Rc<LandfillOccupant>, x: i32, z: i32) -> bool {
        self.place_tile(occupant, x, z);
        self.rec.borrow_mut().tiles_created += 1;
        true
    }
    fn occupant_remove(&self, occupant: u64) -> bool {
        if occupant & VIRTUAL_TILE != 0 {
            let ok = self.remove_tile(occupant);
            if ok {
                self.rec.borrow_mut().tiles_removed += 1;
            }
            return ok;
        }
        if self.shadow {
            let mut r = self.rec.borrow_mut();
            r.hidden.insert(occupant);
            r.tiles_removed += 1;
            return true;
        }
        let ok = cb!(self, occupant_remove, false, occupant);
        if ok {
            self.rec.borrow_mut().tiles_removed += 1;
        }
        ok
    }
    fn occupant_changed(&self, occupant: u64) {
        if occupant & VIRTUAL_TILE != 0 {
            return;
        }
        if self.shadow {
            return self.swallow();
        }
        cb!(self, occupant_changed, (), occupant)
    }
    fn occupant_position(&self, occupant: u64) -> [f32; 3] {
        if let Some(o) = self.virt(occupant) {
            return o.position();
        }
        let mut p = [0.0f32; 3];
        cb!(self, occupant_position, (), occupant, p.as_mut_ptr());
        p
    }
    fn occupant_set_position(&self, occupant: u64, position: [f32; 3]) {
        if let Some(o) = self.virt(occupant) {
            return o.set_position(position);
        }
        if self.shadow {
            return self.swallow();
        }
        cb!(self, occupant_set_position, (), occupant, position.as_ptr())
    }
    fn occupant_set_removal_flags(&self, occupant: u64, flags: u32) {
        if let Some(o) = self.virt(occupant) {
            return o.set_removal_flags(flags);
        }
        if self.shadow {
            return self.swallow();
        }
        cb!(self, occupant_set_removal_flags, (), occupant, flags)
    }
    fn occupant_post_message(&self, occupant: u64, message: u32) {
        if occupant & VIRTUAL_TILE != 0 {
            return;
        }
        if self.shadow {
            return self.swallow();
        }
        cb!(self, occupant_post_message, (), occupant, message)
    }
    fn is_landfill_occupant(&self, occupant: u64) -> bool {
        if occupant & VIRTUAL_TILE != 0 {
            return self.virt(occupant).is_some();
        }
        cb!(self, is_landfill_occupant, false, occupant)
    }
    fn is_empty_landfill_tile(&self, occupant: u64) -> bool {
        if let Some(o) = self.virt(occupant) {
            return o.is_empty_landfill_tile();
        }
        cb!(self, is_empty_landfill_tile, false, occupant)
    }
    fn remove_incompatible_occupants(&self, x: i32, z: i32) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, remove_incompatible_occupants, (), x, z)
    }
    fn set_empty_cell_texture(&self, x: i32, z: i32) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, set_empty_cell_texture, (), x, z)
    }
    fn remove_base_texture(&self, x: i32, z: i32) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, remove_base_texture, (), x, z)
    }

    fn add_lot_prop(&self, prop_type: u32, position: [f32; 3], orientation: i32) -> Option<u64> {
        if self.shadow {
            // No prop is placed: the zone keeps none, as after a failed
            // AddLotProp (the zot is display only).
            self.rec.borrow_mut().zots_added += 1;
            return None;
        }
        let r = opt(cb!(self, add_lot_prop, 0, prop_type, position.as_ptr(), orientation));
        let mut rec = self.rec.borrow_mut();
        if r.is_some() {
            rec.zots_added += 1;
        } else {
            rec.zots_failed += 1;
        }
        r
    }
    fn remove_prop(&self, occupant: u64) -> bool {
        self.rec.borrow_mut().zots_removed += 1;
        if self.shadow {
            return true;
        }
        cb!(self, remove_prop, false, occupant)
    }
    fn zot_orientation(&self) -> i32 {
        cb!(self, zot_orientation, 0)
    }

    fn find_network_tile(&self, x: i32, z: i32, flag_mask: u32, flag: bool) -> Option<u64> {
        opt(cb!(self, find_network_tile, 0, x, z, flag_mask, flag))
    }
    fn subnetworks_in_region(&self, region: &CellRegion) -> Vec<u32> {
        let Some(f) = self.c.subnetworks_in_region else {
            return Vec::new();
        };
        let (_bytes, raw) = region_raw(region);
        // SAFETY: valid callback; `raw` and its bytes outlive the call.
        collect_tokens(|cb, ctx| unsafe { f(self.c.user, &raw, cb, ctx) })
            .into_iter()
            .map(|v| v as u32)
            .collect()
    }
    fn occupant_count_for_all_subnetworks(&self, developer_type: u32) -> Vec<u32> {
        let Some(f) = self.c.occupant_count_for_all_subnetworks else {
            return Vec::new();
        };
        // SAFETY: valid callback.
        collect_tokens(|cb, ctx| unsafe { f(self.c.user, developer_type, cb, ctx) })
            .into_iter()
            .map(|v| v as u32)
            .collect()
    }

    fn attempt_area_levelling(
        &self,
        rect: CellRect,
        f: f32,
        with_water: bool,
        water_flag: bool,
        max_area_slope: f32,
    ) -> bool {
        if self.shadow {
            self.swallow();
            return false;
        }
        cb!(
            self,
            attempt_area_levelling,
            false,
            rect.x1,
            rect.z1,
            rect.x2,
            rect.z2,
            f,
            with_water,
            water_flag,
            max_area_slope
        )
    }

    fn current_view_level(&self) -> u32 {
        cb!(self, current_view_level, 0)
    }

    fn init_zone_effects(&self, centre: (i32, i32), rect: CellRect) -> ZoneEffects {
        if self.shadow {
            return ZoneEffects::default();
        }
        let mut e = fc4_landfill_effects::default();
        cb!(
            self,
            init_zone_effects,
            (),
            centre.0,
            centre.1,
            rect.x1,
            rect.z1,
            rect.x2,
            rect.z2,
            &mut e
        );
        ZoneEffects {
            truck: opt(e.truck),
            seagull: opt(e.seagull),
            seagull_started: false,
        }
    }
    fn shutdown_zone_effects(&self, effects: ZoneEffects) {
        if self.shadow {
            return;
        }
        cb!(
            self,
            shutdown_zone_effects,
            (),
            effects.truck.unwrap_or(0),
            effects.seagull.unwrap_or(0),
            effects.seagull_started
        )
    }
    fn set_truck_enabled(&self, truck: u64, enabled: bool) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, set_truck_enabled, (), truck, enabled)
    }
    fn start_seagull(&self, seagull: u64, flag: u32) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, start_seagull, (), seagull, flag)
    }
    fn stop_seagull(&self, seagull: u64, flag: u32) {
        if self.shadow {
            return self.swallow();
        }
        cb!(self, stop_seagull, (), seagull, flag)
    }

    fn send_history(&self, series: u32, value: u32) {
        self.rec.borrow_mut().history.push((series, value));
        if !self.shadow {
            cb!(self, send_history, (), series, value)
        }
    }

    fn occupant_ref(&self, occupant: u64) -> RefValue {
        if occupant == 0 || occupant & VIRTUAL_TILE != 0 {
            return RefValue::Null;
        }
        RefValue::Host {
            token: occupant,
            class_id: 0,
        }
    }
    // The stream reader already holds the reference the zone keeps (live), or
    // the token is borrowed for the delivery (shadow).
    fn resolve_occupant(&self, r: &RefValue) -> Option<u64> {
        match r {
            RefValue::Host { token, .. } => opt(*token),
            _ => None,
        }
    }
}

// ---- the save stream --------------------------------------------------------------------------------

/// The record reader: delegates to [`CIn`], except that references come back
/// as host tokens the core owns one reference on (the zot props, read with
/// `cISC4Occupant`).
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
        self.get_ref_as(IID_OCCUPANT)
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
            self.host.add_ref(out.token);
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

// ---- handles ----------------------------------------------------------------------------------------

/// Opaque handle (fc4.h `fc4_landfill`).
#[repr(C)]
pub struct fc4_landfill {
    _private: [u8; 0],
}

struct Handle {
    host: Rc<CHost>,
    dev: Rc<LandfillDeveloper>,
    depth: Cell<u32>,
    queue: RefCell<Vec<LandfillMessage>>,
    reentrant: Cell<u32>,
    rebuilt: Cell<u32>,
}

impl Handle {
    fn new(host: Rc<CHost>, token: u64) -> Handle {
        let dev = LandfillDeveloper::new(host.clone(), token);
        Handle {
            host,
            dev,
            depth: Cell::new(0),
            queue: RefCell::new(Vec::new()),
            reentrant: Cell::new(0),
            rebuilt: Cell::new(0),
        }
    }

    /// Runs `f` with the call depth counted; at the end of the outermost
    /// call, the queued messages are handled.
    fn call<R>(&self, f: impl FnOnce(&Handle) -> R) -> R {
        if self.depth.get() > 0 {
            self.reentrant.set(self.reentrant.get().wrapping_add(1));
        }
        self.depth.set(self.depth.get() + 1);
        let r = f(self);
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
                self.handle(&m);
            }
        }
        self.depth.set(self.depth.get() - 1);
        r
    }

    fn handle(&self, m: &LandfillMessage) {
        if m.msg_type == msg::SIM_NEW_MONTH {
            self.host.reset();
        }
        if m.msg_type == msg::POST_CITY_INIT && self.dev.state().loaded {
            self.rebuild_tiles();
        }
        self.dev.do_message(m);
    }

    /// After a load ([choice], §6.4 LG8, LG23): game tiles are saved by the
    /// occupant manager and found here, so only a mask cell whose rule tile
    /// is not the empty tile and that has no landfill occupant (a city saved
    /// while the tiles were kept in this layer) gets its tile back,
    /// chosen as §4.8 would from the saved heaps (the interior tile without a
    /// `rand()` draw: orientation 0), as a game occupant when the host can
    /// make one ([`CHost::place_tile`]). The pollution flag is left as it
    /// is. Returns the tiles made.
    fn rebuild_tiles(&self) -> u32 {
        let host = &self.host;
        if host.c.is_active_landfill.is_none() {
            return 0;
        }
        let mut made = 0u32;
        for zone in self.dev.zone_list() {
            let region = zone.region();
            let (w, h) = (zone.w(), zone.h());
            for (x, z) in region.mask_cells().collect::<Vec<_>>() {
                // LG23 (§6.4, answered; the bracket is acceptable): a cell
                // needs a tile when the §4.8 rule gives one other than
                // 0x02F10000 for the saved heaps and slot 33 finds no
                // landfill occupant; its active-landfill flag is not a test.
                if host.live_active(x, z) == Some(true) {
                    host.active.borrow_mut().insert((x, z));
                }
                if host.occupant_at(x, z, OCCUPANT_TYPE_LANDFILL).is_some() {
                    continue;
                }
                let (cx, cz) = (x - region.x0, z - region.z0);
                let mask = neighbour_mask(cx, cz, w, h, |i, j| zone.heap(i, j).is_some_and(|hp| hp.active));
                let (tile_id, orientation) = tile_of(mask, 0);
                if tile_id == tile::EMPTY {
                    continue;
                }
                let occ = host.build_tile(x, z, tile_id, orientation);
                host.place_tile(&occ, x, z);
                made += 1;
            }
        }
        self.rebuilt.set(made);
        made
    }

    /// The zot tokens the zones hold.
    fn zots(&self) -> Vec<u64> {
        self.dev.zone_list().iter().filter_map(|z| z.zot()).collect()
    }
}

/// Runs `f` on the handle; the fallback for NULL or a panic.
fn with_handle<R>(p: *const fc4_landfill, fallback: R, f: impl FnOnce(&Handle) -> R) -> R {
    // SAFETY: documented precondition: NULL or a live fc4_landfill*.
    let Some(h) = (unsafe { (p as *const Handle).as_ref() }) else {
        return fallback;
    };
    guard(fallback, || f(h))
}

fn dev<R>(p: *const fc4_landfill, fallback: R, f: impl FnOnce(&LandfillDeveloper) -> R) -> R {
    with_handle(p, fallback, |h| h.call(|h| f(&h.dev)))
}

pub(crate) fn host_ok(host: *const fc4_landfill_host) -> Option<fc4_landfill_host> {
    // SAFETY: documented precondition: NULL or a valid host table.
    let h = unsafe { host.as_ref() }?;
    ((h.struct_size as usize) >= std::mem::size_of::<fc4_landfill_host>()).then_some(*h)
}

/// `QueryInterface(iid)` (§1): whether the class answers the IID (`1`,
/// `0x8A09D502`, `0x452294AA`, `0xE4FDA3D4`).
#[no_mangle]
pub extern "C" fn fc4_landfill_query_interface(iid: u32) -> bool {
    fc4_core::landfill::answers_iid(iid)
}

/// The class ID (`GetGZCLSID`, §1): `0xEA09D4AE`.
#[no_mangle]
pub extern "C" fn fc4_landfill_class_id() -> u32 {
    CLSID_LANDFILL_DEVELOPER
}

/// Creates the core object (no zones, the code-default tuning, §2.1, §4.1).
/// The host table is copied. `token` is the host identity of the developer
/// itself.
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_create(host: *const fc4_landfill_host, token: u64) -> *mut fc4_landfill {
    guard(std::ptr::null_mut(), || {
        let Some(h) = host_ok(host) else {
            return std::ptr::null_mut();
        };
        Box::into_raw(Box::new(Handle::new(Rc::new(CHost::new(h, false)), token))) as *mut fc4_landfill
    })
}

/// Destroys the core object. The references it still holds (a loaded object
/// that was never initialised: the zots read back) are released.
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_destroy(p: *mut fc4_landfill) {
    if p.is_null() {
        return;
    }
    guard((), || {
        let b = Box::from_raw(p as *mut Handle);
        // An initialised object released everything at Shutdown (the plugin
        // shuts it down first), so there is nothing left then; a developer
        // that was never initialised still holds its zones' zots.
        for z in b.zots() {
            b.host.release(z);
        }
        if let Some(d) = b.dev.state().department {
            b.host.release(d);
        }
        drop(b);
    });
}

/// `Init` (§1.1): always true.
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_init(p: *mut fc4_landfill) -> bool {
    dev(p, false, LandfillDeveloper::init)
}

/// `Shutdown` (§1.2): always true. Each initialised zone removes its zot prop
/// (`RemovePropA`) and releases it in the core (§4.4.9, LG13). A zone that
/// was never initialised skips its zot at its Shutdown; the original's zone
/// destructor then releases it without `RemovePropA` (LG24, answered; the
/// bracket matches), so its reference is released here, after the core's
/// Shutdown, without `RemovePropA`. A developer that was never initialised keeps its zones, and
/// [`fc4_landfill_destroy`] releases them. The tiles kept here are dropped.
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_shutdown(p: *mut fc4_landfill) -> bool {
    with_handle(p, false, |h| {
        h.call(|h| {
            let kept: Vec<u64> = if h.dev.is_initialised() {
                h.dev
                    .zone_list()
                    .iter()
                    .filter(|z| !z.is_initialised())
                    .filter_map(|z| z.zot())
                    .collect()
            } else {
                Vec::new()
            };
            let ok = h.dev.shutdown();
            for z in kept {
                h.host.release(z);
            }
            *h.host.tiles.borrow_mut() = Tiles::default();
            h.host.active.borrow_mut().clear();
            ok
        })
    })
}

/// `DoMessage` (§3.2): always true. `region` is the zoning message's region
/// (`GetVoid2`, NULL otherwise); it is copied. A message delivered during
/// another call is queued.
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_do_message(
    p: *mut fc4_landfill,
    msg_type: u32,
    data1: u32,
    data2: u32,
    data3: u32,
    data4: u32,
    region: *const fc4_landfill_region,
) -> bool {
    let m = LandfillMessage {
        msg_type,
        data1,
        data2,
        data3,
        data4,
        region: region_of(region),
    };
    with_handle(p, true, |h| {
        if h.depth.get() > 0 {
            h.queue.borrow_mut().push(m);
            return true;
        }
        h.call(|h| h.handle(&m));
        true
    })
}

/// Slot 5 `GetEfficiency` (a fraction, §4.9.2).
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_efficiency(p: *const fc4_landfill) -> f32 {
    dev(p, 1.0, LandfillDeveloper::get_efficiency)
}

/// Slot 7 `GetMaintenanceCosts`.
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_maintenance_costs(p: *const fc4_landfill) -> i64 {
    dev(p, 0, LandfillDeveloper::get_maintenance_costs)
}

/// Slot 8 `GetCurrentBudget`.
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_current_budget(p: *const fc4_landfill) -> i64 {
    dev(p, 0, LandfillDeveloper::get_current_budget)
}

/// Slot 9 `GetZoningCost(region)` (§4.3; the high dword is always 0). 0 for
/// a NULL or empty region.
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_zoning_cost(p: *const fc4_landfill, region: *const fc4_landfill_region) -> i64 {
    let Some(r) = region_of(region) else { return 0 };
    dev(p, 0, |d| d.get_zoning_cost(&r))
}

/// Slot 10 `GetLandfillUsage(usage&, capacity&)` (each written when not NULL).
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_usage(p: *const fc4_landfill, usage: *mut u32, capacity: *mut u32) {
    let (u, c) = dev(p, (0, 0), LandfillDeveloper::get_landfill_usage);
    if let Some(o) = usage.as_mut() {
        *o = u;
    }
    if let Some(o) = capacity.as_mut() {
        *o = c;
    }
}

/// Slot 11 `GetLandfillTileCount` (the zone manager's zone-14 cell count).
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_tile_count(p: *const fc4_landfill) -> i32 {
    dev(p, 0, LandfillDeveloper::get_landfill_tile_count)
}

/// The tile kept here for a cell (tile ID and orientation, each written when
/// not NULL); false when there is none.
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_tile_at(
    p: *const fc4_landfill,
    x: i32,
    z: i32,
    tile_id: *mut u32,
    orientation: *mut i32,
) -> bool {
    let Some((t, o)) = with_handle(p, None, |h| h.host.tile_at(x, z)) else {
        return false;
    };
    if let Some(out) = tile_id.as_mut() {
        *out = t;
    }
    if let Some(out) = orientation.as_mut() {
        *out = o;
    }
    true
}

/// Diagnostics.
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_get_state(p: *const fc4_landfill, out: *mut fc4_landfill_state) -> bool {
    let Some(o) = out.as_mut() else { return false };
    with_handle(p, false, |h| {
        let st = h.dev.state();
        let rec = h.host.rec();
        let mut s = fc4_landfill_state {
            struct_size: std::mem::size_of::<fc4_landfill_state>() as u32,
            initialized: u8::from(h.dev.is_initialised()),
            loaded: u8::from(st.loaded),
            capacity_reported: u8::from(rec.capacity.is_some()),
            virtual_tiles: h.host.tile_count() as u32,
            active_cells: h.host.active_count() as u32,
            reentrant_calls: h.reentrant.get(),
            capacity_total: st.capacity_total,
            used_total: st.used_total,
            free_capacity: rec.capacity.unwrap_or(0),
            rebuilt_tiles: h.rebuilt.get(),
            moved: rec.put_in_landfill,
            supply_delta: rec.supply_delta,
            maintenance: st.maintenance,
            department: st.department.unwrap_or(0),
            game_tiles: h.host.game_tiles.get(),
            game_tile_failures: h.host.game_tile_failures.get(),
            ..Default::default()
        };
        for z in h.dev.zone_list() {
            s.zones += 1;
            s.connected_zones += u32::from(z.connected());
            s.cells = s.cells.wrapping_add(z.cell_count().max(0) as u32);
            s.zots += u32::from(z.zot().is_some());
            for hp in z.heaps() {
                s.heaps += 1;
                s.valid_heaps += u32::from(hp.valid);
                s.active_heaps += u32::from(hp.active);
                s.stored = s.stored.wrapping_add(u64::from(hp.contained));
            }
        }
        *o = s;
        true
    })
}

/// `Write` (§6.1). The zot references go out as `fc4_ref`s with the token
/// and class 0.
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_write(p: *const fc4_landfill, out: *const fc4_ostream) -> bool {
    let Some(o) = out.as_ref() else { return false };
    if !ostream_ok(o) {
        return false;
    }
    dev(p, false, |d| d.write(&mut COut(o)))
}

/// `Read` (§6.1). Runs before `Init`: nothing is sized from the city here.
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_read(p: *mut fc4_landfill, inp: *const fc4_istream) -> bool {
    let Some(i) = inp.as_ref() else { return false };
    if !istream_ok(i) {
        return false;
    }
    with_handle(p, false, |h| {
        let host = h.host.clone();
        h.call(|h| {
            h.dev.read(&mut LIn {
                inner: CIn {
                    stream: i,
                    decode: no_core_objects,
                },
                host: &host,
            })
        })
    })
}

/// Isolated occupant verification over caller-authored public record inputs.
/// No native output is accepted as an expected state or a Rust seed.
#[repr(C)]
#[derive(Default)]
pub struct fc4_landfill_occupant_probe_result {
    pub read_ok: u32,
    pub write_ok: u32,
    pub orientation: i32,
    pub empty: u32,
    pub occupant_type: u32,
    pub flags: u32,
    pub previous_tile: u32,
    pub class_id: u32,
    pub qi_mask: u32,
    pub ref_delta: u32,
    pub position: [f32; 3],
    pub bounds: [f32; 6],
}

/// `mutate` changes orientation/position and exercises SetTileID's same-ID
/// branch. The null-parent fixtures deliberately do not mutate an exemplar.
/// Empty input means construction queries only; no record is written.
#[no_mangle]
pub unsafe extern "C" fn fc4_landfill_occupant_probe(
    bytes: *const u8,
    len: usize,
    mutate: u32,
    orientation: i32,
    position: *const f32,
    sink: crate::stream::fc4_bytes_sink,
    user: *mut c_void,
    result: *mut fc4_landfill_occupant_probe_result,
) -> bool {
    use fc4_core::stream::{ByteReader, ByteWriter};
    crate::guard(false, || {
        let Some(result) = result.as_mut() else { return false };
        if (len != 0 && bytes.is_null()) || (mutate != 0 && position.is_null()) {
            return false;
        }
        let o = LandfillOccupant::empty();
        let before = Rc::strong_count(&o);
        let extra = o.clone();
        let ref_delta = (Rc::strong_count(&o) - before) as u32;
        drop(extra);
        let mut r = fc4_landfill_occupant_probe_result {
            ref_delta,
            ..Default::default()
        };
        for (bit, iid) in [1, 0x2A15_F262, IID_OCCUPANT, 0xE4FD_A3D4, 0xDEAD_BEEF]
            .into_iter()
            .enumerate()
        {
            r.qi_mask |= u32::from(o.answers_iid(iid)) << bit;
        }
        r.class_id = o.class_id();
        if len != 0 {
            let mut input = ByteReader::new(std::slice::from_raw_parts(bytes, len));
            r.read_ok = u32::from(o.read(&mut input));
        }
        if mutate != 0 && r.read_ok != 0 {
            r.previous_tile = o.set_tile_id(o.tile_id());
            o.set_orientation(orientation);
            let p = std::slice::from_raw_parts(position, 3);
            o.set_position([p[0], p[1], p[2]]);
        }
        r.orientation = o.orientation();
        r.empty = u32::from(o.is_empty_landfill_tile());
        r.occupant_type = o.snapshot().base.occupant_type;
        r.flags = u32::from(o.snapshot().base.flags);
        r.position = o.position();
        let (lo, hi) = o.bounding_box();
        r.bounds = [lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]];
        if r.read_ok != 0 {
            let mut output = ByteWriter::new();
            r.write_ok = u32::from(o.write(&mut output));
            // Persistence uses only our own write as our reload input. Native
            // captures never enter this function.
            let reloaded = LandfillOccupant::empty();
            let mut reload_input = ByteReader::new(output.bytes());
            let mut second = ByteWriter::new();
            if !reloaded.read(&mut reload_input) || !reloaded.write(&mut second) || second.bytes() != output.bytes() {
                return false;
            }
            if let Some(sink) = sink {
                sink(user, output.bytes().as_ptr(), output.bytes().len());
            }
        }
        *result = r;
        true
    })
}
