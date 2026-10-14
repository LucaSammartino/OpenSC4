// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The landfill zone `cSC4LandfillZone` (§2.2) and its heaps: the zoning and
//! dezoning primitives (§4.4), the terrain response (§4.5.2, §4.5.3), capacity
//! (§4.6.1–§4.6.4), the monthly per-zone pass (§4.7.2), the tile models and
//! their `rand()` draws (§4.8), maintenance (§4.9.1) and the inline record
//! (§6.2).
//!
//! A zone is a plain refcounted object, **not** a GZCOM class (§1): it has no
//! CLSID and is written inline in the developer's record. The original gives it
//! a `cIGZMessageTarget2`-shaped vtable whose `DoMessage` returns true and does
//! nothing, and whose slot 4 is the destructor; FreeCity4 models that as
//! [`LandfillZone::do_message`] and [`LandfillZone::shutdown`].
//!
//! Zones are `Rc`-held, refcounted objects, as in the original.
//!
//! **No re-entrancy.** `DezoneCell` with `changeZone` calls the zone
//! manager's `PlaceZone`, which **posts** `0x07699455` for the 1×1 cell
//! (§4.4.8 step 4 *[compat]*): the type-0 `DoZoneChange` runs on a later
//! message-server tick, after the pass that dezoned the cell has completed.
//! So [`LandfillZone::dezone_cell`] performs all five steps itself.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::grid::CellRect;
use crate::rng::CrtRand;
use crate::stream::{InStream, OutStream, RefValue};
use crate::x87::r24;

use super::algo::{
    decay_amount, heap_capacity, neighbour_mask, redistribute, redistribute_share, spiral_cells, tile_of,
};
use super::levelling::FLT_MAX;
use super::{
    msg, tile, CellRegion, LandfillHost, LandfillOccupant, ZoneEffects, CONNECTIVITY_DEVELOPER_TYPES,
    NETWORK_TILE_MASK, OCCUPANT_TYPE_BASE_TEXTURE, OCCUPANT_TYPE_LANDFILL, TERRAIN_MOVE_THRESHOLD, ZONE_TYPE_LANDFILL,
    ZOT_HEIGHT, ZOT_PROP,
};

/// The zone record's version (§6.2).
pub const ZONE_RECORD_VERSION: u16 = 4;
/// The heap record's version (§6.2); a heap with any other version fails the
/// zone read.
pub const HEAP_RECORD_VERSION: u16 = 2;

/// One garbage heap: a 2×2 block of zoned cells (§2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Heap {
    /// Local coordinates, relative to the zone origin **at the time the heap
    /// was made** (§4.4.4, quirk 4).
    pub hx: u32,
    pub hz: u32,
    /// The heap's four cells are zoned; cleared by `InvalidateSurroundingHeaps`.
    pub valid: bool,
    /// The heap currently shows tile models (`contained ≠ 0` at the last check).
    pub active: bool,
    pub capacity: u32,
    /// Garbage held.
    pub contained: u32,
}

impl Heap {
    /// A new heap at `(hx, hz)`: valid, inactive, empty (§4.4.4 step 4).
    pub fn new(hx: u32, hz: u32) -> Heap {
        Heap {
            hx,
            hz,
            valid: true,
            active: false,
            capacity: 0,
            contained: 0,
        }
    }
}

/// The zone's state (§2.2), as plain data for snapshots and tests.
#[derive(Clone, Debug)]
pub struct ZoneData {
    pub region: CellRegion,
    /// The heap grid size (`x1 − x0`, `z1 − z0`).
    pub w: i32,
    pub h: i32,
    /// The heaps in vector order: index `hz × W + hx` (§2.2).
    pub heaps: Vec<Heap>,
    pub connected: bool,
    /// The not-connected prop (§4.7.5), saved by reference (§6.2).
    pub zot: Option<u64>,
    /// The target altitude for levelling and zoning cost. **Not saved** (§2.2,
    /// §4.4.9, quirk 3).
    pub level_altitude: f32,
    /// The sticky `IsCompletelyDezoned` cache (§4.4.7, quirk 6).
    pub completely_dezoned: bool,
    /// The zone effects (§4.10); rendering only, not saved.
    pub effects: ZoneEffects,
}

/// What one cell of a terrain change did (§4.5.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerrainAction {
    /// Nothing.
    Nothing,
    /// The cell was dezoned with `DezoneCell(x, z, true, true)` (§4.5.2
    /// step 1, or step 2's empty-tile case).
    Dezoned,
    /// The tile was moved with the terrain (§4.5.2 step 2's
    /// `kbMoveWithTerrain` case): position set, occupant manager slot 24 and
    /// occupant slot 15 called.
    Moved,
}

/// One landfill zone (§2.2).
pub struct LandfillZone {
    host: Rc<dyn LandfillHost>,
    d: RefCell<ZoneData>,
    /// The cached number of mask cells and its dirty flag (§2.2, §4.6.4).
    cell_count: Cell<i32>,
    cell_count_dirty: Cell<bool>,
    initialised: Cell<bool>,
}

impl LandfillZone {
    /// A zone for a zoning region (§2.2): the region as zoned, a heap grid of
    /// `W × H = (x1−x0) × (z1−z0)` heaps all valid, not connected, no zot,
    /// `levelAltitude` 0.0, `cellCount` dirty.
    pub fn new(host: Rc<dyn LandfillHost>, region: CellRegion) -> Rc<LandfillZone> {
        let (w, h) = (region.x1 - region.x0, region.z1 - region.z0);
        let heaps = heaps_for(w, h);
        Rc::new(LandfillZone {
            host,
            d: RefCell::new(ZoneData {
                region,
                w,
                h,
                heaps,
                connected: false,
                zot: None,
                level_altitude: 0.0,
                completely_dezoned: false,
                effects: ZoneEffects::default(),
            }),
            cell_count: Cell::new(0),
            cell_count_dirty: Cell::new(true),
            initialised: Cell::new(false),
        })
    }

    /// An empty zone, as §6.1's `Read` constructs one before reading it.
    pub fn empty(host: Rc<dyn LandfillHost>) -> Rc<LandfillZone> {
        LandfillZone::new(host, CellRegion::cleared(CellRect::new(0, 0, 0, 0)))
    }

    pub fn data(&self) -> ZoneData {
        self.d.borrow().clone()
    }

    pub fn rect(&self) -> CellRect {
        let d = self.d.borrow();
        d.region.rect()
    }

    pub fn region(&self) -> CellRegion {
        self.d.borrow().region.clone()
    }

    /// Whether `(x, z)` is one of this zone's mask cells.
    pub fn is_mask_cell(&self, x: i32, z: i32) -> bool {
        self.d.borrow().region.is_set(x, z)
    }

    /// Whether `(x, z)` lies in the zone's rectangle (§7.1 slot 12 uses the
    /// rect alone).
    pub fn contains_cell(&self, x: i32, z: i32) -> bool {
        let d = self.d.borrow();
        d.region.x0 <= x && x < d.region.x1 && d.region.z0 <= z && z < d.region.z1
    }

    pub fn w(&self) -> i32 {
        self.d.borrow().w
    }

    pub fn h(&self) -> i32 {
        self.d.borrow().h
    }

    pub fn heaps(&self) -> Vec<Heap> {
        self.d.borrow().heaps.clone()
    }

    pub fn connected(&self) -> bool {
        self.d.borrow().connected
    }

    pub fn set_connected(&self, connected: bool) {
        self.d.borrow_mut().connected = connected;
    }

    pub fn zot(&self) -> Option<u64> {
        self.d.borrow().zot
    }

    pub fn level_altitude(&self) -> f32 {
        self.d.borrow().level_altitude
    }

    pub fn set_level_altitude(&self, altitude: f32) {
        self.d.borrow_mut().level_altitude = altitude;
    }

    pub fn effects(&self) -> ZoneEffects {
        self.d.borrow().effects
    }

    /// Records whether the seagull effect is started (§3.2
    /// PreViewLevelChange: `Start(0)` sets it, `Stop(1)` clears it).
    pub fn set_seagull_started(&self, started: bool) {
        self.d.borrow_mut().effects.seagull_started = started;
    }

    /// The zone's `cIGZMessageTarget2`-shaped `DoMessage` (§1): returns true
    /// and does nothing.
    pub fn do_message(&self, _m: &super::LandfillMessage) -> bool {
        true
    }

    // ---- Init / Shutdown (§4.4.9) ----

    /// Zone `Init` (once): acquire the services, `InitZoneEffects()`, and —
    /// because `levelAltitude` is not saved — set it to the altitude at the
    /// centre cell when it is exactly 0.0 (§4.4.9, quirk 3).
    pub fn init(&self) -> bool {
        if self.initialised.get() {
            return true;
        }
        self.initialised.set(true);
        let rect = self.rect();
        let centre = CellRegion::centre_cell(rect);
        let effects = self.host.init_zone_effects(centre, rect);
        self.d.borrow_mut().effects = effects;
        // An exact compare (LF2: `FUCOMPP` against +0.0, ordered equal
        // only): −0.0 == 0.0, a NaN is not 0.0.
        if self.level_altitude() == 0.0 {
            let a = self.host.cell_altitude(centre.0, centre.1);
            self.set_level_altitude(a);
        }
        true
    }

    /// Zone `Shutdown` (§4.4.9, `0x006BB750`):
    /// 1. stop and release the effects, even when the zone was never
    ///    initialised;
    /// 2. only if initialised: clear the flag, then *[compat]* (LG13) remove a
    ///    held zot through the prop manager (`RemovePropA`, slot 29) and
    ///    release it ([`LandfillZone::remove_zot`]).
    ///
    /// A zone that was never initialised (loaded, no PostCityInit) keeps its
    /// zot reference here, as the original does; the C ABI layer releases it
    /// when the developer shuts down.
    pub fn shutdown(&self) -> bool {
        let effects = std::mem::take(&mut self.d.borrow_mut().effects);
        self.host.shutdown_zone_effects(effects);
        if self.initialised.get() {
            self.initialised.set(false);
            self.remove_zot();
        }
        true
    }

    pub fn is_initialised(&self) -> bool {
        self.initialised.get()
    }

    // ---- cell count (§4.6.4) ----

    /// `GetCellCount()`: the number of mask cells, cached and recounted when
    /// dirty (§4.6.4).
    pub fn cell_count(&self) -> i32 {
        if self.cell_count_dirty.get() {
            let n = self.d.borrow().region.mask_count();
            self.cell_count.set(n);
            self.cell_count_dirty.set(false);
        }
        self.cell_count.get()
    }

    fn dirty_cell_count(&self) {
        self.cell_count_dirty.set(true);
    }

    // ---- heaps ----

    /// The heap at local `(hx, hz)` — vector index `hz × W + hx` — or `None`
    /// when outside `0 ≤ hx < W`, `0 ≤ hz < H`.
    pub fn heap(&self, hx: i32, hz: i32) -> Option<Heap> {
        let d = self.d.borrow();
        heap_at(&d, hx, hz).copied()
    }

    /// The number of valid heaps (§4.6.2).
    pub fn num_valid_heaps(&self) -> i32 {
        self.d.borrow().heaps.iter().filter(|h| h.valid).count() as i32
    }

    /// `InvalidateSurroundingHeaps(x, z)` (§4.4.8): with `lx = x − x0` and
    /// `lz = z − z0`, set `valid := false` on heaps `(lx, lz)`, `(lx, lz−1)`,
    /// `(lx−1, lz)`, `(lx−1, lz−1)` that lie inside `0 ≤ hx < W`, `0 ≤ hz < H`.
    /// Nothing else is touched (capacity, garbage, active state and tiles stay).
    pub fn invalidate_surrounding_heaps(&self, x: i32, z: i32) {
        let mut d = self.d.borrow_mut();
        let (lx, lz) = (x - d.region.x0, z - d.region.z0);
        for (hx, hz) in [(lx, lz), (lx, lz - 1), (lx - 1, lz), (lx - 1, lz - 1)] {
            if let Some(h) = heap_at_mut(&mut d, hx, hz) {
                h.valid = false;
            }
        }
    }

    // ---- IsCompletelyDezoned (§4.4.7) ----

    /// `IsCompletelyDezoned()` (§4.4.7). If the cache is already true it
    /// returns true **without rescanning** *[quirk 6]*. Otherwise it sets the
    /// flag, scans the mask, and clears the flag again if any bit is set.
    pub fn is_completely_dezoned(&self) -> bool {
        let mut d = self.d.borrow_mut();
        if d.completely_dezoned {
            return true;
        }
        d.completely_dezoned = true;
        if d.region.mask().iter().any(|&m| m) {
            d.completely_dezoned = false;
            return false;
        }
        true
    }

    /// Sets the sticky flag (§4.4.3: `DezoneArea` sets it when the zone keeps
    /// no valid heap).
    pub fn set_completely_dezoned(&self, v: bool) {
        self.d.borrow_mut().completely_dezoned = v;
    }

    // ---- AddRegionToZone (§4.4.4) ----

    /// `AddRegionToZone(region)` (§4.4.4).
    ///
    /// 1. Remove and release an existing zot.
    /// 2. New rect := the union of the zone rect and the region rect; new mask
    ///    := old mask cells ∪ region mask cells.
    /// 3. For each region mask cell (`x` outer, `z` inner):
    ///    `LevelRect((x, z, x+1, z+1), levelAltitude, false)` — including cells
    ///    already in the zone. The rect is inclusive, so each cell levels the
    ///    2×2-cell block (vertices `x..x+2 × z..z+2`), chained cell by cell
    ///    and reaching one cell past the region's +x and +z edges (LF5,
    ///    `0x006BC150`).
    /// 4. Rebuild the heap grid for the new `W × H`: all heaps new, then each
    ///    **old** heap copied with all its fields to index
    ///    `old.hz × W + old.hx`. *[quirk 4]* The old local coordinates are not
    ///    translated, so when the zone grows toward smaller `x` or `z` the old
    ///    heaps' garbage, capacity and active state move by the growth amount.
    /// 5. For every cell of the new rect that is **not** a mask cell:
    ///    `InvalidateSurroundingHeaps(x, z)`.
    /// 6. `cellCount` dirty.
    pub fn add_region_to_zone(&self, region: &CellRegion) {
        // 1.
        self.remove_zot();
        // 2.
        let old = self.d.borrow().clone();
        let new_rect = CellRegion::union(old.region.rect(), region.rect());
        let mut new_region = CellRegion::cleared(new_rect);
        for (x, z) in old.region.mask_cells() {
            new_region.set(x, z, true);
        }
        for (x, z) in region.mask_cells() {
            new_region.set(x, z, true);
        }
        // 3. LevelRect per region mask cell, in (x outer, z inner) order.
        let level_altitude = old.level_altitude;
        for (x, z) in region.mask_cells() {
            self.level_rect(CellRect::new(x, z, x + 1, z + 1), level_altitude, false);
        }
        // 4. The new heap grid, then the old heaps at their old local indices.
        let (w, h) = (new_rect.x2 - new_rect.x1, new_rect.z2 - new_rect.z1);
        let mut heaps = heaps_for(w, h);
        for oh in &old.heaps {
            let index = (oh.hz as i32).checked_mul(w).and_then(|v| v.checked_add(oh.hx as i32));
            // The union only grows, so the index is in range; guard anyway
            // ([choice], mod.rs).
            if let Some(i) = index {
                if let Some(slot) = heaps.get_mut(i as usize) {
                    *slot = *oh;
                }
            }
        }
        {
            let mut d = self.d.borrow_mut();
            d.region = new_region;
            d.w = w;
            d.h = h;
            d.heaps = heaps;
        }
        // 5.
        let rect = self.rect();
        for (x, z) in CellRegion::cleared(rect).cells() {
            if !self.is_mask_cell(x, z) {
                self.invalidate_surrounding_heaps(x, z);
            }
        }
        // 6.
        self.dirty_cell_count();
    }

    // ---- PopulateZone (§4.4.5) ----

    /// `PopulateZone()` (§4.4.5). For each cell of the rect (`x` outer, `z`
    /// inner):
    ///
    /// - a mask cell whose zone type is not 14, or that has a network tile
    ///   (`FindNetworkTile(x, z, 0x1F4F, true)`), loses its mask bit;
    /// - a mask cell that already has a landfill (`0x6A11BD2B`) or base-texture
    ///   (`0x497F6D9D`) occupant is kept as is;
    /// - any other mask cell gets `RemoveIncompatibleOccupants` (flora
    ///   `0x74758926`) and `SetEmptyCellTexture`;
    /// - a cell that was **not** a mask cell at the start of the pass gets
    ///   `InvalidateSurroundingHeaps`.
    ///
    /// *[quirk 5]* A cell this pass drops from the mask does **not** invalidate
    /// its heaps, because the second test uses the mask as it was at the start.
    pub fn populate_zone(&self) {
        let snapshot = self.d.borrow().region.clone();
        let rect = self.rect();
        for (x, z) in CellRegion::cleared(rect).cells() {
            let was_mask = snapshot.is_set(x, z);
            if was_mask {
                // `GetZoneType(x, z) ≠ 14 or FindNetworkTile(…) ≠ null`, in
                // that order (the network map is asked only for a zone-14 cell).
                let drop = self.host.zone_type(x, z) != ZONE_TYPE_LANDFILL
                    || self.host.find_network_tile(x, z, NETWORK_TILE_MASK, true).is_some();
                if drop {
                    self.d.borrow_mut().region.set(x, z, false);
                } else if self.host.occupant_at(x, z, OCCUPANT_TYPE_LANDFILL).is_some()
                    || self.host.occupant_at(x, z, OCCUPANT_TYPE_BASE_TEXTURE).is_some()
                {
                    // Kept as is.
                } else {
                    self.host.remove_incompatible_occupants(x, z);
                    self.host.set_empty_cell_texture(x, z);
                }
            } else {
                self.invalidate_surrounding_heaps(x, z);
            }
        }
        self.dirty_cell_count();
    }

    // ---- DezoneCell (§4.4.8) ----

    /// `DezoneCell(x, z, changeZone, removeTexture)` (§4.4.8).
    ///
    /// 1. Remove and release an existing zot.
    /// 2. If `removeTexture`: `RemoveLandfillTile(x, z)` and
    ///    `RemoveBaseTexture(x, z)`.
    /// 3. Clear the cell's mask bit; `InvalidateSurroundingHeaps(x, z)`.
    /// 4. If `changeZone`: `zoneManager.PlaceZone(1×1 region at (x, z),
    ///    zoneType 0, …)`. The zone manager **posts** `0x07699455`; nothing
    ///    re-enters here (*[compat]*).
    /// 5. `cellCount` dirty.
    pub fn dezone_cell(&self, x: i32, z: i32, change_zone: bool, remove_texture: bool) {
        // 1.
        self.remove_zot();
        // 2.
        if remove_texture {
            self.remove_landfill_tile(x, z);
            self.host.remove_base_texture(x, z);
        }
        // 3.
        self.d.borrow_mut().region.set(x, z, false);
        self.invalidate_surrounding_heaps(x, z);
        // 4. The result is not used.
        if change_zone {
            let cell = CellRegion::all_set(CellRect::new(x, z, x, z));
            self.host.place_zone(&cell, 0);
        }
        // 5.
        self.dirty_cell_count();
    }

    // ---- terrain (§4.5.2, §4.5.3) ----

    /// `HandleTerrainChange(rect)` (§4.5.2): `HandleCellTerrainChange` for
    /// every mask cell of `rect ∩ zone rect`, `x` outer and `z` inner.
    pub fn handle_terrain_change(&self, rect: CellRect, move_with_terrain: bool) {
        let Some(i) = CellRegion::intersection(self.rect(), rect) else {
            return;
        };
        for x in i.x1..=i.x2 {
            for z in i.z1..=i.z2 {
                if self.is_mask_cell(x, z) {
                    self.handle_cell_terrain_change(x, z, move_with_terrain);
                }
            }
        }
    }

    /// `HandleCellTerrainChange(x, z)` (§4.5.2).
    ///
    /// 1. No landfill occupant at the cell → `DezoneCell(x, z, true, true)`
    ///    *[compat]*: any terraforming over an empty landfill cell dezones it,
    ///    whatever the height change.
    /// 2. Else, if it answers `cISC4LandfillOccupant` (`0x2A15F262`):
    ///    `δ := p.y − GetAltitudeAtNearestGrid(p.x, p.z)`; if `|δ| ≥ 0.25`:
    ///    an empty tile dezones, else with `kbMoveWithTerrain` the tile is
    ///    moved to `max(sea, a)` (`p.y := (sea < a) ? a : sea`), re-indexed
    ///    (occupant manager slot 24) and given the display notification
    ///    (occupant slot 15, `0x99EF1145`).
    pub fn handle_cell_terrain_change(&self, x: i32, z: i32, move_with_terrain: bool) -> TerrainAction {
        let Some(occ) = self.host.occupant_at(x, z, OCCUPANT_TYPE_LANDFILL) else {
            // 1.
            self.dezone_cell(x, z, true, true);
            return TerrainAction::Dezoned;
        };
        if !self.host.is_landfill_occupant(occ) {
            return TerrainAction::Nothing;
        }
        // 2.
        let p = self.host.occupant_position(occ);
        let a = self.host.altitude_at_nearest_grid(p[0], p[2]);
        let delta = r24(f64::from(p[1]) - f64::from(a)) as f32;
        // `|δ| ≥ 0.25`: a NaN δ does not pass.
        if delta.is_nan() || delta.abs() < TERRAIN_MOVE_THRESHOLD {
            return TerrainAction::Nothing;
        }
        if self.host.is_empty_landfill_tile(occ) {
            self.dezone_cell(x, z, true, true);
            return TerrainAction::Dezoned;
        }
        if !move_with_terrain {
            return TerrainAction::Nothing;
        }
        let sea = self.host.sea_level();
        let y = if sea < a { a } else { sea };
        self.host.occupant_set_position(occ, [p[0], y, p[2]]);
        self.host.occupant_changed(occ);
        self.host.occupant_post_message(occ, msg::OCCUPANT_CHANGED);
        TerrainAction::Moved
    }

    /// `LevelRect(rect, altitude, direct)` (§4.5.3): `levelAltitude :=
    /// altitude`, then either `terrain.SetAltitude(clamped rect, altitude, 1, 5)`
    /// (`direct = true`, never used by the shipped code) or
    /// `buildingDevelopment.GetLotDeveloper().AttemptAreaLevelling(rect, 0.0,
    /// true, true, FLT_MAX, null)`, whose result is ignored. *[quirk 10]* The
    /// requested altitude is **not** passed (the float argument is 0.0 and is
    /// never read): the lot developer levels to the rect's **vertex** average,
    /// or not at all when its slope exceeds 40, while `levelAltitude` keeps
    /// the caller's cell-centre mean.
    pub fn level_rect(&self, rect: CellRect, altitude: f32, direct: bool) {
        self.set_level_altitude(altitude);
        if direct {
            let (cx, cz) = self.host.cell_counts();
            let clamped = CellRect::new(
                rect.x1.clamp(0, cx),
                rect.z1.clamp(0, cz),
                rect.x2.clamp(0, cx),
                rect.z2.clamp(0, cz),
            );
            self.host.set_altitude(clamped, altitude);
        } else {
            let _ignored = self.host.attempt_area_levelling(rect, 0.0, true, true, FLT_MAX);
        }
    }

    // ---- capacity (§4.6.1–§4.6.3) ----

    /// `CalculateGarbageHeapCapacity(v)` (§4.6.1): `v` is the number of valid
    /// heaps, `tile_capacity` is pollution's `GetLandfillTileCapacity()` (read
    /// each time it is needed, §4.1) and `cell_count` the zone's mask cells.
    pub fn calculate_garbage_heap_capacity(&self, tile_capacity: u32, valid_heaps: i32) -> u32 {
        let cells = self.cell_count();
        heap_capacity(tile_capacity, cells, valid_heaps)
    }

    /// `RecalculateCapacity()` (§4.6.2): `c := CalculateGarbageHeapCapacity(v)`;
    /// for each heap in vector order `capacity := valid ? c : 0`, and if
    /// `contained > capacity` (unsigned) the excess goes back to pollution's
    /// supply — one `AddToGarbageSupply(contained − capacity)` per such heap,
    /// as an `int32` — and `contained := capacity` (the `landfilled` counter
    /// is **not** reduced). `totalCapacity := v × c`. Returns the total
    /// capacity.
    pub fn recalculate_capacity(&self, tile_capacity: u32) -> u32 {
        let v = self.num_valid_heaps();
        let c = self.calculate_garbage_heap_capacity(tile_capacity, v);
        let n = self.d.borrow().heaps.len();
        for i in 0..n {
            let spill = {
                let mut d = self.d.borrow_mut();
                let h = &mut d.heaps[i];
                h.capacity = if h.valid { c } else { 0 };
                if h.contained > h.capacity {
                    let excess = h.contained - h.capacity;
                    h.contained = h.capacity;
                    Some(excess)
                } else {
                    None
                }
            };
            if let Some(excess) = spill {
                self.host.add_to_garbage_supply(excess as i32);
            }
        }
        v.wrapping_mul(c as i32) as u32
    }

    /// `GetCapacity()` → `(available, contained, capacity)` (§4.6.3): sums over
    /// **all** heaps, valid or not, with 32-bit wrap.
    pub fn get_capacity(&self) -> (i32, u32, u32) {
        let d = self.d.borrow();
        let mut contained = 0u32;
        let mut capacity = 0u32;
        let mut available = 0u32;
        for h in &d.heaps {
            contained = contained.wrapping_add(h.contained);
            capacity = capacity.wrapping_add(h.capacity);
            available = available.wrapping_add(h.capacity.wrapping_sub(h.contained));
        }
        (available as i32, contained, capacity)
    }

    /// `GetAvailableCapacity()`: `available` as `int32` (§4.6.3).
    pub fn available_capacity(&self) -> i32 {
        self.get_capacity().0
    }

    // ---- the monthly per-zone pass (§4.7.2) ----

    /// `UpdateZone(&remaining)` (§4.7.2). `remaining` is the developer's `t`,
    /// shared across zones in list order; `rng` is the developer's C `rand()`
    /// state (§4.8). Returns the zone's available capacity as `int32`, which
    /// the developer adds to `avail` **only when the zone is connected**
    /// (§4.7.1 step 4).
    ///
    /// 1. `connected := IsZoneConnected()` (stored). Not connected → `AddZot()`;
    ///    connected with a zot → remove and release it.
    /// 2. `budget := min(remaining, GetCellCount() × kMaxGarbageToTakePerMonthPerTile)`
    ///    (signed compare).
    /// 3. For each **valid** heap in vector order: decay; take
    ///    `n := min(capacity − contained, budget)` when there is room, the zone
    ///    is connected and `budget > 0`; book it; flip the active flag and
    ///    redraw the tiles when it changes.
    /// 4. `RedistributeGarbage()`.
    pub fn update_zone(
        &self,
        remaining: &mut i32,
        per_tile_cap: u32,
        decay_rate: f32,
        decay_cap: u32,
        rng: &mut CrtRand,
    ) -> i32 {
        // 1.
        let connected = self.is_zone_connected();
        self.set_connected(connected);
        if connected {
            self.remove_zot();
        } else {
            self.add_zot();
        }
        // 2.
        let cells = self.cell_count();
        let cap = cells.wrapping_mul(per_tile_cap as i32);
        let mut budget = (*remaining).min(cap);
        // 3.
        let indices: Vec<usize> = {
            let d = self.d.borrow();
            (0..d.heaps.len()).filter(|&i| d.heaps[i].valid).collect()
        };
        for i in indices {
            // 3.1
            self.decay_garbage(i, decay_rate, decay_cap);
            // 3.2
            let (room, was_active) = {
                let d = self.d.borrow();
                let h = &d.heaps[i];
                (h.capacity.wrapping_sub(h.contained), h.active)
            };
            let n = if room != 0 && connected && budget > 0 {
                (room.min(budget as u32)) as i32
            } else {
                0
            };
            // 3.3 These calls are made with n = 0 too.
            {
                let mut d = self.d.borrow_mut();
                d.heaps[i].contained = d.heaps[i].contained.wrapping_add(n as u32);
            }
            self.host.add_to_garbage_supply(-n);
            self.host.garbage_put_in_landfill(n);
            *remaining -= n;
            budget -= n;
            // 3.4
            let contained = self.d.borrow().heaps[i].contained;
            let now_active = contained != 0;
            if now_active != was_active {
                self.d.borrow_mut().heaps[i].active = now_active;
                self.update_surrounding_tiles(i as i32, rng);
            }
        }
        // 4.
        self.redistribute_garbage();
        self.available_capacity()
    }

    /// `DecayGarbage()` of one heap (§4.7.3).
    pub fn decay_garbage(&self, index: usize, decay_rate: f32, decay_cap: u32) {
        let contained = self.d.borrow().heaps.get(index).map_or(0, |h| h.contained);
        let d = decay_amount(contained, decay_rate, decay_cap);
        if d != 0 {
            if let Some(h) = self.d.borrow_mut().heaps.get_mut(index) {
                h.contained -= d;
            }
        }
    }

    /// `RedistributeGarbage()` (§4.7.4): `share := cap / v` (signed integer
    /// division) and `rem := total`; for each **valid** heap in vector order
    /// `contained := min(share, rem)` and `rem −= x`. Invalid heaps are not
    /// changed, and the active flags are **not** updated here (they catch up at
    /// the next `UpdateZone`).
    ///
    /// *[quirk 7]* After a terrain dezone invalid heaps still hold capacity and
    /// garbage: both count in `cap` and `total`, so the valid heaps receive
    /// copies of the invalid heaps' garbage and garbage is duplicated until the
    /// next `RecalculateCapacity` returns the excess to the supply.
    pub fn redistribute_garbage(&self) {
        let (_, total, cap) = self.get_capacity();
        let v = self.num_valid_heaps();
        if total == 0 || v <= 0 {
            return;
        }
        let share = redistribute_share(cap, v);
        let mut rem = total;
        let indices: Vec<usize> = {
            let d = self.d.borrow();
            (0..d.heaps.len()).filter(|&i| d.heaps[i].valid).collect()
        };
        for i in indices {
            let (x, r) = redistribute(share, rem);
            self.d.borrow_mut().heaps[i].contained = x;
            rem = r;
        }
    }

    /// `IsZoneConnected()` (§4.7.2): dilate the zone's cell region by one cell
    /// in the four orthogonal directions (cells only, not diagonals), take the
    /// traffic subnetworks in it, and be connected as soon as some subnetwork
    /// has a non-zero developed-occupant count for one of the twelve developer
    /// types of [`CONNECTIVITY_DEVELOPER_TYPES`]. So a landfill is connected iff
    /// a transport subnetwork touching it reaches at least one developed RCI
    /// building.
    pub fn is_zone_connected(&self) -> bool {
        let dilated = self.dilated_region();
        let ids = self.host.subnetworks_in_region(&dilated);
        // Every type is asked until one connects, even with no subnetwork.
        for &dev_type in CONNECTIVITY_DEVELOPER_TYPES.iter() {
            let counts = self.host.occupant_count_for_all_subnetworks(dev_type);
            for &id in &ids {
                if (id as usize) < counts.len() && counts[id as usize] != 0 {
                    return true;
                }
            }
        }
        false
    }

    /// The zone's cell region dilated by one cell orthogonally (§4.7.2).
    pub fn dilated_region(&self) -> CellRegion {
        let region = self.d.borrow().region.clone();
        let rect = CellRegion::inflate(region.rect());
        let mut out = CellRegion::cleared(rect);
        for (x, z) in region.mask_cells() {
            for (dx, dz) in [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
                out.set(x + dx, z + dz, true);
            }
        }
        out
    }

    // ---- the zot (§4.7.5) ----

    /// `AddZot()` (§4.7.5). If there is no zot: spiral outward from the centre
    /// cell (clipped to the zone rect, centre first) to the first mask cell; if
    /// none, use the centre cell. Position := the cell centre with
    /// `y := terrain.GetAltitudeAtNearestGrid(x, z) + 3.0`. Then
    /// `propManager.AddLotProp(0x107A0000, position, orientation)` (the
    /// `(propID, pos, orientation)` overload; the orientation comes from a
    /// global view service), `AddRef`, store, and `SetRemovalFlags(3)`.
    ///
    /// *[quirk]* The original does not null-check the result: a missing prop
    /// exemplar crashes. FreeCity4 leaves no zot instead ([choice], mod.rs).
    pub fn add_zot(&self) {
        if self.d.borrow().zot.is_some() {
            return;
        }
        let rect = self.rect();
        let centre = CellRegion::centre_cell(rect);
        let cell = spiral_cells(rect, centre)
            .into_iter()
            .find(|&(x, z)| self.is_mask_cell(x, z))
            .unwrap_or(centre);
        let (px, pz) = self.host.cell_center_to_position(cell.0, cell.1);
        let y = r24(f64::from(self.host.cell_altitude_at_nearest_grid(cell.0, cell.1)) + f64::from(ZOT_HEIGHT)) as f32;
        let orientation = self.host.zot_orientation();
        let Some(prop) = self.host.add_lot_prop(ZOT_PROP, [px, y, pz], orientation) else {
            return;
        };
        self.host.add_ref(prop);
        self.d.borrow_mut().zot = Some(prop);
        self.host.occupant_set_removal_flags(prop, 3);
    }

    /// Removes and releases the zot (§4.4.4 step 1, §4.4.8 step 1, §4.7.2
    /// step 1).
    pub fn remove_zot(&self) {
        let Some(zot) = self.d.borrow_mut().zot.take() else {
            return;
        };
        self.host.remove_prop(zot);
        self.host.release(zot);
    }

    // ---- tile models (§4.8) ----

    /// `UpdateSurroundingTiles(heap)` (§4.8). A heap's state change redraws the
    /// four cells it covers, in the order `(hx, hz)`, `(hx+1, hz)`,
    /// `(hx, hz+1)`, `(hx+1, hz+1)` (absolute = zone origin + local).
    ///
    /// For a cell at local `(cx, cz)`:
    /// `mask := 1·A(cx−1,cz−1) + 2·A(cx−1,cz) + 4·A(cx,cz) + 8·A(cx,cz−1)`.
    /// Then the rule table gives the tile and orientation; mask 15 (the interior
    /// tile `0x02FA0000`) draws **one** C `rand()` from `rng` and sets the
    /// orientation to `2 × (rand() & 3)` *[compat]* (§4.8: one draw per
    /// redrawn cell whose four surrounding heaps are all active, so 0–4 per
    /// heap state change). Then `RemoveLandfillTile(x, z)` and
    /// `CreateLandfillOccupant(x, z, tile, orientation)`.
    ///
    /// `index` is the heap's vector index, whose stale local `hx`, `hz` decide
    /// which cells are redrawn (quirk 4).
    pub fn update_surrounding_tiles(&self, index: i32, rng: &mut CrtRand) {
        let (origin, hx, hz, w, h) = {
            let d = self.d.borrow();
            let Some(heap) = d.heaps.get(index as usize) else {
                return;
            };
            ((d.region.x0, d.region.z0), heap.hx as i32, heap.hz as i32, d.w, d.h)
        };
        // `A(i, j)`: heap `(i, j)` (vector index `j × W + i`) is active.
        let active = |i: i32, j: i32| heap_at(&self.d.borrow(), i, j).is_some_and(|hp| hp.active);
        for (dx, dz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let (cx, cz) = (hx + dx, hz + dz);
            let mask = neighbour_mask(cx, cz, w, h, active);
            // Only mask 15's tile draws (§4.8).
            let rand_value = if mask == 15 { rng.rand() } else { 0 };
            let (tile_id, orientation) = tile_of(mask, rand_value);
            let (x, z) = (origin.0 + cx, origin.1 + cz);
            self.remove_landfill_tile(x, z);
            self.create_landfill_occupant(x, z, tile_id, orientation);
        }
    }

    /// `CreateLandfillOccupant(x, z, tile, orientation)` (§4.8.1) → bool.
    ///
    /// 1. `RemoveIncompatibleOccupants(x, z)`.
    /// 2. The empty tile `0x02F10000`: if the cell has no base-texture
    ///    occupant, `SetEmptyCellTexture(x, z)`; return true. **No occupant is
    ///    created and the landfill flag stays cleared** (§4.8.2).
    /// 3. Otherwise: load the tile exemplar `(0x6534284A, 0x07BDDF1C, tile)`,
    ///    create a `cSC4LandfillOccupant(tile, orientation)`, attach the
    ///    exemplar to its property holder, `SetFlag(4)`, `SetRemovalFlags(2)`,
    ///    `pollution.SetIsActiveLandfill(x, z, true)`, `isEmpty := false`,
    ///    position := the cell centre with `y = altitude(x, z)`, `SetPosition`,
    ///    `Init`, and insert into the occupant manager (slot 20,
    ///    `(occupant, 0)`), returning its result.
    ///
    /// *[quirk]* A missing exemplar is dereferenced by the original; FreeCity4
    /// creates the occupant with no parent exemplar instead ([choice], mod.rs).
    pub fn create_landfill_occupant(&self, x: i32, z: i32, tile_id: u32, orientation: i32) -> bool {
        // 1.
        self.host.remove_incompatible_occupants(x, z);
        // 2.
        if tile_id == tile::EMPTY {
            if self.host.occupant_at(x, z, OCCUPANT_TYPE_BASE_TEXTURE).is_none() {
                self.host.set_empty_cell_texture(x, z);
            }
            return true;
        }
        // 3.
        let exemplar = self.host.exemplar(tile::TYPE, tile::GROUP, tile_id);
        let occ = LandfillOccupant::new(tile_id, orientation);
        if let Some(ex) = exemplar {
            let key = crate::stream::ResKey {
                type_id: tile::TYPE,
                group: tile::GROUP,
                instance: tile_id,
            };
            let holder = crate::property::ExemplarHolder {
                parent: Some((key, ex)),
                ..Default::default()
            };
            occ.attach_exemplar(holder);
        }
        // The constructor already set flag 4 (§2.3); §4.8.1 sets it again.
        occ.set_flag(super::occupant::CONSTRUCTOR_FLAG);
        occ.set_removal_flags(2);
        self.host.set_is_active_landfill(x, z, true);
        occ.set_empty(false);
        let (px, pz) = self.host.cell_center_to_position(x, z);
        let y = self.host.cell_altitude(x, z);
        occ.set_position([px, y, pz]);
        occ.init();
        self.host.insert_landfill_occupant(&occ, x, z)
    }

    /// `RemoveLandfillTile(x, z)` (§4.8.3): remove a landfill occupant at the
    /// cell (occupant manager slot 21, `(occupant, 1, 3)`), then
    /// `pollution.SetIsActiveLandfill(x, z, false)` — **always**.
    pub fn remove_landfill_tile(&self, x: i32, z: i32) {
        if let Some(occ) = self.host.occupant_at(x, z, OCCUPANT_TYPE_LANDFILL) {
            self.host.occupant_remove(occ);
        }
        self.host.set_is_active_landfill(x, z, false);
    }

    // ---- maintenance (§4.9.1) ----

    /// `GetMaintenanceCost()` (§4.9.1): `Σ over mask cells of tileCost(x, z)`
    /// as an `int64`, where `tileCost` is the first `s64` value of property
    /// [`super::PROP_ITEM_COST`] of the landfill occupant at the cell, or
    /// `kDefaultLandfillTileCost` when the cell has none (empty cells,
    /// including cells whose tile is mask 0).
    pub fn maintenance_cost(&self, default_tile_cost: i64) -> i64 {
        let region = self.d.borrow().region.clone();
        let mut sum = 0i64;
        for (x, z) in region.mask_cells() {
            let cost = match self.host.occupant_at(x, z, OCCUPANT_TYPE_LANDFILL) {
                Some(occ) => match self.host.occupant_item_cost(occ, super::PROP_ITEM_COST) {
                    Some(c) => c,
                    None => default_tile_cost,
                },
                None => default_tile_cost,
            };
            sum = sum.saturating_add(cost);
        }
        sum
    }

    // ---- effects (§4.10, §3.2) ----

    /// `InitZoneEffects()` (§4.10, §4.4.1 step 2): the automata and the
    /// seagull effect for the zone's current rect. Effects still running are
    /// shut down first.
    pub fn init_zone_effects(&self) {
        self.shutdown_zone_effects();
        let rect = self.rect();
        let centre = CellRegion::centre_cell(rect);
        let effects = self.host.init_zone_effects(centre, rect);
        self.d.borrow_mut().effects = effects;
    }

    /// `ShutdownZoneEffects()` (§4.10): releases the automata and the effect.
    pub fn shutdown_zone_effects(&self) {
        let effects = std::mem::take(&mut self.d.borrow_mut().effects);
        if effects != ZoneEffects::default() {
            self.host.shutdown_zone_effects(effects);
        }
    }

    // ---- persistence (§6.2) ----

    /// Writes the zone inline (§6.2).
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        if out.error() != 0 {
            return false;
        }
        let d = self.d.borrow();
        out.put_u16(ZONE_RECORD_VERSION);
        out.put_i32(d.region.x0);
        out.put_i32(d.region.z0);
        out.put_i32(d.region.x1);
        out.put_i32(d.region.z1);
        // The mask: (x1−x0) × (z1−z0) bytes, x outer and z inner. *[quirk 2]*
        // The last column (x = x1) and the last row (z = z1) are never written.
        for x in d.region.x0..d.region.x1 {
            for z in d.region.z0..d.region.z1 {
                out.put_u8(u8::from(d.region.is_set(x, z)));
            }
        }
        out.put_i32(d.w);
        out.put_i32(d.h);
        for h in &d.heaps {
            out.put_u16(HEAP_RECORD_VERSION);
            out.put_u32(h.hx);
            out.put_u32(h.hz);
            out.put_u16(u16::from(h.valid));
            out.put_u16(u16::from(h.active));
            out.put_u32(h.contained);
            out.put_u32(h.capacity);
        }
        out.put_u8(u8::from(d.connected));
        let zot = match d.zot {
            Some(t) => self.host.occupant_ref(t),
            None => RefValue::Null,
        };
        out.put_ref(&zot);
        out.error() == 0
    }

    /// Reads the zone record (§6.2). The version is **not** checked (any value
    /// is accepted): version 3 reads `connected` and no zot, versions ≥ 4 read
    /// `connected` (true iff ≠ 0) and the zot reference, versions < 3 read
    /// neither. The region is created with **all cells set**, then the
    /// `(x1−x0) × (z1−z0)` bytes are applied (1 → set, else clear) *[quirk 2]*.
    /// Each heap record must have version 2, else the read fails (the partially
    /// read heap is still appended with the previous heap's values *[quirk]*).
    /// `valid` and `active` are true iff the `u16` equals 1. `levelAltitude` is
    /// not saved (§4.4.9). Returns `!stream.GetError()`.
    pub fn read(&self, inp: &mut dyn InStream) -> bool {
        if inp.error() != 0 {
            return false;
        }
        let Some(version) = inp.get_u16() else { return false };
        let (Some(x0), Some(z0), Some(x1), Some(z1)) = (inp.get_i32(), inp.get_i32(), inp.get_i32(), inp.get_i32())
        else {
            return inp.error() == 0;
        };
        let rect = CellRect::new(x0, z0, x1, z1);
        let mut region = CellRegion::all_set(rect);
        // The mask bytes: (x1−x0) × (z1−z0), x outer and z inner.
        'mask: for x in x0..x1 {
            for z in z0..z1 {
                match inp.get_u8() {
                    Some(b) => region.set(x, z, b == 1),
                    None => break 'mask,
                }
            }
        }
        let (Some(w), Some(h)) = (inp.get_i32(), inp.get_i32()) else {
            return inp.error() == 0;
        };
        let count = (w.max(0).saturating_mul(h.max(0))) as usize;
        let mut heaps = Vec::with_capacity(count.min(1 << 16));
        // The original reads every heap into one reused object, so a heap whose
        // read stops early keeps the previous heap's remaining values.
        let mut heap = Heap::new(0, 0);
        for _ in 0..count {
            // A wrong version stops the zone read: the heap is still appended
            // with the previous heap's values, and the read fails [quirk].
            let version_ok = inp.get_u16() == Some(HEAP_RECORD_VERSION);
            let fields_ok = version_ok && read_heap_fields(inp, &mut heap);
            heaps.push(heap);
            if !fields_ok {
                self.commit(region, w, h, heaps, None, None);
                return false;
            }
        }
        let connected = if version >= 3 {
            inp.get_u8().map(|b| b != 0)
        } else {
            None
        };
        let zot = if version >= 4 {
            match inp.get_ref_as(super::IID_OCCUPANT) {
                Some(r) if !r.is_null() => self.host.resolve_occupant(&r),
                Some(_) => None,
                None => None,
            }
        } else {
            None
        };
        self.commit(region, w, h, heaps, connected, zot);
        inp.error() == 0
    }

    fn commit(&self, region: CellRegion, w: i32, h: i32, heaps: Vec<Heap>, connected: Option<bool>, zot: Option<u64>) {
        let mut d = self.d.borrow_mut();
        d.region = region;
        d.w = w;
        d.h = h;
        d.heaps = heaps;
        if let Some(c) = connected {
            d.connected = c;
        }
        // A zot read from a save is transferred to the zone (§6.2).
        d.zot = zot.or(d.zot);
        drop(d);
        self.dirty_cell_count();
    }

    // ---- tests ----

    /// Overwrites the heap grid (tests and the conformance runner).
    pub fn set_heaps(&self, heaps: Vec<Heap>) {
        let mut d = self.d.borrow_mut();
        d.heaps = heaps;
    }

    /// Overwrites one heap (tests and the conformance runner).
    pub fn set_heap(&self, index: usize, heap: Heap) {
        if let Some(h) = self.d.borrow_mut().heaps.get_mut(index) {
            *h = heap;
        }
    }

    /// Overwrites the mask (tests and the conformance runner).
    pub fn set_region(&self, region: CellRegion) {
        let mut d = self.d.borrow_mut();
        d.w = region.x1 - region.x0;
        d.h = region.z1 - region.z0;
        d.region = region;
        drop(d);
        self.dirty_cell_count();
    }

    /// Sets the zot token without a prop call (tests).
    pub fn set_zot(&self, zot: Option<u64>) {
        self.d.borrow_mut().zot = zot;
    }
}

/// The fields of one heap record after its version (§6.2): `u32 hx; u32 hz;
/// u16 valid; u16 active; u32 contained; u32 capacity`, `valid` and `active`
/// true iff the `u16` equals 1. A field that cannot be read keeps the value
/// already in `heap`; returns whether all six were read.
fn read_heap_fields(inp: &mut dyn InStream, heap: &mut Heap) -> bool {
    let hx = inp.get_u32();
    let hz = inp.get_u32();
    let valid = inp.get_u16();
    let active = inp.get_u16();
    let contained = inp.get_u32();
    let capacity = inp.get_u32();
    let all = hx.is_some() && hz.is_some() && valid.is_some() && active.is_some() && contained.is_some();
    heap.hx = hx.unwrap_or(heap.hx);
    heap.hz = hz.unwrap_or(heap.hz);
    heap.valid = valid.map_or(heap.valid, |v| v == 1);
    heap.active = active.map_or(heap.active, |v| v == 1);
    heap.contained = contained.unwrap_or(heap.contained);
    heap.capacity = capacity.unwrap_or(heap.capacity);
    all && capacity.is_some()
}

/// The heap grid of a `w × h` zone: `w × h` heaps, all valid, at their local
/// positions, in vector order `hz × W + hx` (§2.2). A non-positive side gives
/// no heaps (a zone one cell wide, §4.6.1).
fn heaps_for(w: i32, h: i32) -> Vec<Heap> {
    if w <= 0 || h <= 0 {
        return Vec::new();
    }
    let mut v = Vec::with_capacity((w * h) as usize);
    for hz in 0..h {
        for hx in 0..w {
            v.push(Heap::new(hx as u32, hz as u32));
        }
    }
    v
}

/// The vector index of local heap `(hx, hz)` in a `w × h` grid: `hz × W + hx`
/// (§2.2), or `None` outside `0 ≤ hx < W`, `0 ≤ hz < H`.
fn heap_index(hx: i32, hz: i32, w: i32, h: i32) -> Option<usize> {
    if !(0..w).contains(&hx) || !(0..h).contains(&hz) {
        return None;
    }
    Some(hz as usize * w as usize + hx as usize)
}

/// Heap `(hx, hz)`: the heap at its vector index (§2.2). A loaded zone whose
/// heap vector is shorter than `W × H` (a truncated record, §6.2) has no heap
/// there.
fn heap_at(d: &ZoneData, hx: i32, hz: i32) -> Option<&Heap> {
    d.heaps.get(heap_index(hx, hz, d.w, d.h)?)
}

fn heap_at_mut(d: &mut ZoneData, hx: i32, hz: i32) -> Option<&mut Heap> {
    let i = heap_index(hx, hz, d.w, d.h)?;
    d.heaps.get_mut(i)
}
