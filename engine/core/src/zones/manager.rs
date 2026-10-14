// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The zone manager (`cSC4ZoneManager`, class `0x298F9B2D`): the zone grid
//! and its counts, the zoning tuning (§4), `PlaceZone` (§6), the dezone under
//! new networks (§5.1), the queries of §11.1 and the save record (§10.1).
//!
//! Every method takes `&self` and the state lives behind a [`RefCell`]: the
//! zone manager and the zone developer reach each other through the host
//! (§13.1), so a `PlaceZone` commit re-enters `GetZoneType` through
//! `DoParcellization` → `CanCreateLot`. No borrow is held across a host call.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use super::{
    is_rci_zone, place_error, ZonesHost, ZonesMessage, CLSID_ZONE_MANAGER, DEMOLISH_COMMIT_FLAGS, DEMOLISH_PRICE_FLAGS,
    IID_SIM_GRID, MSG_INSERT_OCCUPANT, NETWORK_OCCUPANT_TYPE, NO_DEZONE_FLAGS, ZONE_GRID_CLASS,
};
use crate::grid::{CellRect, GridProvider, TractGrid};
use crate::property::PropertyHolder;
use crate::props;
use crate::stream::{InStream, OutStream, RefValue, ResKey, SaveObject};
use crate::variant::{mem, Elem, Variant};
use std::any::Any;

/// The zone manager's record version (§10.1 field 1).
pub const MANAGER_RECORD_VERSION: u16 = 1;

/// Tuning property IDs (§4), in the "Zone Manager" exemplar.
pub mod prop {
    pub const MIN_CELL_SIZE: u32 = 0xE948_25B2;
    pub const MAX_CELL_SIZE: u32 = 0xE948_25B3;
    pub const CREATION_COST: u32 = 0xE948_25B4;
    pub const DESTRUCTION_COST: u32 = 0xE948_25B5;
    pub const TEXTURE_INSTANCE: u32 = 0xE948_25B6;
    pub const DISPLAY_ALPHA: u32 = 0xE948_25B7;
    pub const DRAG_COLOR: u32 = 0xE948_25B8;
    pub const MAX_SLOPE: u32 = 0xE948_25B9;
}

/// The tuning exemplar key (§4): `{type, group, instance}`. ZG1: the type
/// is `0x6534284A`; `0xA52160F5` is the interface ID the original asks of the
/// resource manager with this key, not a type.
pub const TUNING_KEY: (u32, u32, u32) = (0x6534_284A, 0xE7E2_C2DB, 0xE948_2490);

/// `GetTextureForZone`'s fixed key type and group (§11.1 slot 14).
pub const TEXTURE_KEY_TYPE: u32 = 0x7AB5_0E44;
pub const TEXTURE_KEY_GROUP: u32 = 0x1ABE_787D;

/// Slot 9's bounds (§11.1): `zone < 16` and `result < 19`, signed.
const FAILURE_MAX_ZONE: i32 = 16;
const FAILURE_MAX_RESULT: i32 = 19;

/// The zoning tuning (§4). Floats are float32 values; the arrays are indexed
/// by zone type 0..15.
#[derive(Clone, Debug, PartialEq)]
pub struct Tuning {
    pub min_size: [i32; 16],
    pub max_size: [i32; 16],
    pub create_cost: [i64; 16],
    pub destroy_cost: [i64; 16],
    pub texture: [u32; 16],
    pub drag_color: [u32; 16],
    /// Not saved (§3.1).
    pub max_slope: [f32; 16],
    pub display_alpha: u8,
}

impl Default for Tuning {
    /// The constructor / code-default state (§4): sizes 0 [choice: the
    /// original leaves them uninitialised], costs 0, alpha 0xFF.
    fn default() -> Tuning {
        Tuning {
            min_size: [0; 16],
            max_size: [0; 16],
            create_cost: [0; 16],
            destroy_cost: [0; 16],
            texture: [0; 16],
            drag_color: [0; 16],
            max_slope: [0.0; 16],
            display_alpha: 0xFF,
        }
    }
}

impl Tuning {
    /// `ReadSimulatorTunableValues` (§4). The two cost arrays are zeroed
    /// first; each array property is read as **exactly 16** values, taking
    /// the first `min(16, count)` and leaving the rest at their previous
    /// value [choice: the original reinterprets 16 elements through `Ref*`
    /// with no count or type check]. A missing exemplar or property leaves the
    /// array unchanged. Returns whether the Zone Display Alpha was read (ZG3:
    /// only then is it copied to the default alpha).
    pub fn read(&mut self, h: Option<&PropertyHolder>) -> bool {
        self.create_cost = [0; 16];
        self.destroy_cost = [0; 16];
        let Some(h) = h else { return false };

        fn take16<T: Copy>(dst: &mut [T; 16], src: Option<Vec<T>>) {
            if let Some(v) = src {
                for (d, s) in dst.iter_mut().zip(v) {
                    *d = s;
                }
            }
        }
        take16(
            &mut self.min_size,
            props::i64s(h, prop::MIN_CELL_SIZE).map(|v| v.into_iter().map(|x| x as i32).collect()),
        );
        take16(
            &mut self.max_size,
            props::i64s(h, prop::MAX_CELL_SIZE).map(|v| v.into_iter().map(|x| x as i32).collect()),
        );
        take16(&mut self.create_cost, props::i64s(h, prop::CREATION_COST));
        take16(&mut self.destroy_cost, props::i64s(h, prop::DESTRUCTION_COST));
        take16(&mut self.texture, props::u32s(h, prop::TEXTURE_INSTANCE));
        take16(&mut self.drag_color, props::u32s(h, prop::DRAG_COLOR));
        take16(&mut self.max_slope, props::f32s(h, prop::MAX_SLOPE));
        if let Some(a) = props::u32_of(h, prop::DISPLAY_ALPHA) {
            self.display_alpha = a as u8;
            return true;
        }
        false
    }
}

/// The shipped "Zone Manager" exemplar (§4 "Shipped" column), for hosts and
/// tests without the game's data files.
pub fn shipped_tuning() -> PropertyHolder {
    fn i32a(h: &mut PropertyHolder, id: u32, v: [i32; 16]) {
        h.add(id, Variant::array(mem::SINT32, v.into_iter().map(Elem::I32).collect()));
    }
    fn i64a(h: &mut PropertyHolder, id: u32, v: [i64; 16]) {
        h.add(id, Variant::array(mem::SINT64, v.into_iter().map(Elem::I64).collect()));
    }
    fn u32a(h: &mut PropertyHolder, id: u32, v: [u32; 16]) {
        h.add(id, Variant::array(mem::UINT32, v.into_iter().map(Elem::U32).collect()));
    }
    fn f32a(h: &mut PropertyHolder, id: u32, v: [f32; 16]) {
        h.add(id, Variant::array(mem::FLOAT32, v.into_iter().map(Elem::F32).collect()));
    }
    let mut h = PropertyHolder::new();

    i32a(
        &mut h,
        prop::MIN_CELL_SIZE,
        [0, 1, 1, 1, 1, 1, 1, 4, 1, 1, 1, 1, 1, 1, 2, 1],
    );
    i32a(
        &mut h,
        prop::MAX_CELL_SIZE,
        [32768, 32, 32, 32, 32, 32, 32, 48, 32, 32, 32, 32, 32, 32, 32, 32],
    );
    i64a(
        &mut h,
        prop::CREATION_COST,
        [0, 10, 20, 50, 10, 20, 50, 10, 20, 50, 1, 1, 1, 1, 50, 1],
    );
    i64a(
        &mut h,
        prop::DESTRUCTION_COST,
        [0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    );
    u32a(
        &mut h,
        prop::TEXTURE_INSTANCE,
        [
            4121, 4117, 4118, 4116, 4099, 4100, 4098, 4112, 4113, 4111, 4114, 4096, 4120, 4123, 4101, 4122,
        ],
    );
    h.add(prop::DISPLAY_ALPHA, Variant::Scalar(Elem::U8(170)));
    u32a(
        &mut h,
        prop::DRAG_COLOR,
        [
            4278190335, 12845311, 10092799, 7471359, 1333264383, 542375935, 337699583, 4292555519, 4289868799,
            3465551871, 0, 0, 0, 0, 0, 0,
        ],
    );
    f32a(
        &mut h,
        prop::MAX_SLOPE,
        [
            256.0, 15.0, 15.0, 15.0, 15.0, 15.0, 15.0, 15.0, 15.0, 15.0, 15.0, 15.0, 15.0, 15.0, 15.0, 15.0,
        ],
    );
    h
}

/// The result of [`ZoneManager::place_zone`] (§6). The outputs are always
/// returned [choice]: the original writes `*outCost` / `*outError` only when
/// the pointers are given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlaceZoneResult {
    /// AL: whether the zoning succeeded (and, with `commit`, was applied).
    pub ok: bool,
    /// `*outCost`: the total cost (0 after an early fail or `skipFunds`).
    pub cost: i64,
    /// `*outError`: 0 ok, 1 funds, 2 landfill on water, 3 too small, 4 too
    /// large, 5 blocked.
    pub error: i32,
    /// `*outLots`: the lots to demolish, ordered by token [compat-limit: the
    /// original orders a `std::set` by address]; only membership is
    /// observable.
    pub out_lots: Vec<u64>,
}

impl PlaceZoneResult {
    fn fail(error: i32) -> PlaceZoneResult {
        PlaceZoneResult {
            ok: false,
            cost: 0,
            error,
            out_lots: Vec::new(),
        }
    }
}

/// The zone manager's mutable state (§3.1).
struct Mgr {
    grid: Option<Rc<dyn TractGrid<i8>>>,
    zone_count: [u32; 16],
    undeveloped: [u32; 16],
    unnamed_per_type: [u32; 16],
    undeveloped_total: u32,
    unnamed1: u32,
    abandoned_tile_count: u32,
    unnamed2: u32,
    tuning: Tuning,
    default_alpha: u8,
    /// Development-failure map 1 (§11.1 slot 9), keyed `(result, zone)` so
    /// iteration is "ordered by result, then zone" (§10.1 field 11).
    failure1: BTreeMap<(u8, u8), u32>,
    /// Development-failure map 2 (only saved, §10.1 field 12).
    failure2: BTreeMap<(u8, u8), u32>,
    initialised: bool,
    loaded: bool,
}

impl Default for Mgr {
    fn default() -> Mgr {
        Mgr {
            grid: None,
            zone_count: [0; 16],
            undeveloped: [0; 16],
            unnamed_per_type: [0; 16],
            undeveloped_total: 0,
            unnamed1: 0,
            abandoned_tile_count: 0,
            unnamed2: 0,
            tuning: Tuning::default(),
            default_alpha: 0xFF,
            failure1: BTreeMap::new(),
            failure2: BTreeMap::new(),
            initialised: false,
            loaded: false,
        }
    }
}

/// One zone manager (§1).
pub struct ZoneManager {
    host: Rc<dyn ZonesHost>,
    st: RefCell<Mgr>,
}

fn rect_inside(inner: CellRect, outer: CellRect) -> bool {
    inner.x1 >= outer.x1 && inner.x2 <= outer.x2 && inner.z1 >= outer.z1 && inner.z2 <= outer.z2
}

/// Whether a cell of `region` that PlaceZone would examine (inside the
/// region's bitmap, its bit set) lies outside the `cx` × `cz` city (ZG2).
fn has_cell_outside_city(region: &super::CellRegion, cx: i32, cz: i32) -> bool {
    let r = region.rect;
    if r.x1 > r.x2 || r.z1 > r.z2 {
        return false; // an empty rectangle has no cell
    }
    if r.x1 >= 0 && r.z1 >= 0 && r.x2 < cx && r.z2 < cz {
        return false; // the whole rectangle is inside
    }
    region
        .cells()
        .into_iter()
        .any(|(x, z)| x < 0 || z < 0 || x >= cx || z >= cz)
}

impl ZoneManager {
    /// The constructor's state (§3.1): no grid, counts 0, alpha 0xFF, flags
    /// false. The member `cRZRandom` is never created (§9: it is never drawn).
    pub fn new(host: Rc<dyn ZonesHost>) -> ZoneManager {
        ZoneManager {
            host,
            st: RefCell::new(Mgr::default()),
        }
    }

    pub fn host(&self) -> &Rc<dyn ZonesHost> {
        &self.host
    }

    pub fn is_initialised(&self) -> bool {
        self.st.borrow().initialised
    }

    pub fn is_loaded(&self) -> bool {
        self.st.borrow().loaded
    }

    fn grid(&self) -> Option<Rc<dyn TractGrid<i8>>> {
        self.st.borrow().grid.clone()
    }

    // ---- lifecycle (§1.1) ----

    /// `Init` (slot 3, §1.1). Idempotent.
    pub fn init(&self) -> bool {
        if self.st.borrow().initialised {
            return true;
        }
        // 1. ReadSimulatorTunableValues (§4).
        let tuning = self.host.tuning();
        {
            let mut s = self.st.borrow_mut();
            // ZG3 (§10.1): the exemplar's Zone Display Alpha overwrites the
            // alpha a Read set and is copied to the default alpha. [question]
            // Literal reading: with no exemplar or no such property the saved
            // alpha survives and the default alpha is left as it was (the
            // copy is of the exemplar's value, so there is nothing to copy).
            if s.tuning.read(tuning.as_deref()) {
                s.default_alpha = s.tuning.display_alpha;
            }
        }
        // 2. Take the services and subscribe.
        self.host.manager_acquire_services();
        self.host
            .subscribe(super::Component::ZoneManager, &[MSG_INSERT_OCCUPANT]);
        // 3. If the city was not loaded from a save, reset the counts.
        let loaded = self.st.borrow().loaded;
        if !loaded {
            let (cx, cz) = self.host.city_cell_counts();
            let n = (cx.max(0) as u32).wrapping_mul(cz.max(0) as u32);
            let mut s = self.st.borrow_mut();
            s.zone_count = [0; 16];
            s.undeveloped = [n; 16];
            s.unnamed_per_type = [n; 16];
            s.zone_count[0] = n;
            s.undeveloped_total = n;
        }
        // 4. If there is no zone grid, construct an int8 grid (tract size 1,
        //    instance 0) and clear it.
        if self.st.borrow().grid.is_none() {
            if let Some(g) = <dyn GridProvider<i8>>::create_grid(&*self.host, 0, 1) {
                g.fill(0);
                self.st.borrow_mut().grid = Some(g);
            }
        }
        // 5. Set the initialised flag.
        self.st.borrow_mut().initialised = true;
        true
    }

    /// `Shutdown` (slot 4, §1.1). Returns 1.
    pub fn shutdown(&self) -> bool {
        if !self.st.borrow().initialised {
            return true;
        }
        self.st.borrow_mut().initialised = false;
        self.host
            .unsubscribe(super::Component::ZoneManager, &[MSG_INSERT_OCCUPANT]);
        self.host.manager_release_services();
        self.host.shutdown_grid(ZONE_GRID_CLASS);
        self.st.borrow_mut().grid = None;
        true
    }

    // ---- tuning access ----

    pub fn tuning(&self) -> Tuning {
        self.st.borrow().tuning.clone()
    }

    /// Replaces the tuning (tests and hosts that read it themselves).
    pub fn set_tuning(&self, t: Tuning) {
        self.st.borrow_mut().tuning = t;
    }

    // ---- queries (§11.1) ----

    /// Slot 5 `GetZoneGrid` (no AddRef).
    pub fn zone_grid(&self) -> Option<Rc<dyn TractGrid<i8>>> {
        self.grid()
    }

    /// Slot 6 `GetZoneCount(type)`. [choice] An out-of-range type reads 0
    /// (the original has no bounds check).
    pub fn zone_count(&self, t: i32) -> u32 {
        let s = self.st.borrow();
        s.zone_count.get(t as usize).copied().unwrap_or(0)
    }

    /// Slot 7 `GetUndevelopedTileCount(type)`; never updated after `Init`
    /// (§2 *[quirk]*).
    pub fn undeveloped_tile_count(&self, t: i32) -> u32 {
        let s = self.st.borrow();
        s.undeveloped.get(t as usize).copied().unwrap_or(0)
    }

    /// Slot 8 `GetUndevelopedTileCount()`; never updated (§2 *[quirk]*).
    pub fn undeveloped_total(&self) -> u32 {
        self.st.borrow().undeveloped_total
    }

    /// Slot 9 `GetDevelopmentFailureCount(zone, result)`. Returns 0 unless
    /// `zone < 16` and `result < 19` (signed); otherwise map 1's count,
    /// **inserting a 0 entry** when absent (§11.1 *[quirk]*: the entry is then
    /// saved).
    pub fn development_failure_count(&self, zone: i32, result: i32) -> u32 {
        if zone >= FAILURE_MAX_ZONE || result >= FAILURE_MAX_RESULT || zone < 0 || result < 0 {
            return 0;
        }
        let mut s = self.st.borrow_mut();
        // Slot 9 inserts a zero entry when the key is absent (§11.1 [quirk]);
        // that entry is then saved (§10.1 field 11).
        *s.failure1.entry((result as u8, zone as u8)).or_insert(0)
    }

    /// Slot 10 `GetAbandonedTileCount()`; never updated (§2 *[quirk]*).
    pub fn abandoned_tile_count(&self) -> u32 {
        self.st.borrow().abandoned_tile_count
    }

    /// Slot 12 `GetZoneType(x, z)`: the grid value sign-extended to `int32`,
    /// with no bounds check (out of grid → 0, the grid's [choice]).
    pub fn zone_type(&self, x: i32, z: i32) -> i32 {
        match self.grid() {
            Some(g) => i32::from(g.get_cell_value(x, z)),
            None => 0,
        }
    }

    /// Slot 13 `IsRCIZone(type)` (§2).
    pub fn is_rci_zone(&self, t: i32) -> bool {
        is_rci_zone(t)
    }

    /// Slot 14 `GetTextureForZone(type)`: `None` for `type ≥ 16`, else the
    /// resource key `{0x7AB50E44, 0x1ABE787D, texture[type]}`.
    pub fn texture_for_zone(&self, t: i32) -> Option<ResKey> {
        let s = self.st.borrow();
        s.tuning.texture.get(t as usize).map(|&inst| ResKey {
            type_id: TEXTURE_KEY_TYPE,
            group: TEXTURE_KEY_GROUP,
            instance: inst,
        })
    }

    /// Slot 15 `GetZoneDragColor(type)`. [choice] Out of range reads 0.
    pub fn zone_drag_color(&self, t: i32) -> u32 {
        let s = self.st.borrow();
        s.tuning.drag_color.get(t as usize).copied().unwrap_or(0)
    }

    /// Slot 16 `GetZoneDisplayAlpha()`.
    pub fn zone_display_alpha(&self) -> u8 {
        self.st.borrow().tuning.display_alpha
    }

    /// Slot 17 `GetDefaultZoneDisplayAlpha()`.
    pub fn default_zone_display_alpha(&self) -> u8 {
        self.st.borrow().default_alpha
    }

    /// Slot 18 `SetZoneDisplayAlpha(a)`: when the value changes, every lot of
    /// the city in state 1 gets lot developer slot 43 `SetEmptyLotBaseTexture`
    /// (§11.1).
    pub fn set_zone_display_alpha(&self, alpha: u8) {
        let changed = self.st.borrow().tuning.display_alpha != alpha;
        self.st.borrow_mut().tuning.display_alpha = alpha;
        if !changed {
            return;
        }
        let (cx, cz) = self.host.city_cell_counts();
        let whole = CellRect::new(0, 0, cx - 1, cz - 1);
        for lot in self.host.lot_manager_lots_in_cell_rect(whole, false) {
            if self.host.lot_state(lot) == 1 {
                self.host.set_empty_lot_base_texture(lot);
            }
            self.host.release(lot);
        }
    }

    /// Slot 19 `GetMinZoneSize(type)`. [choice] Out of range reads 0.
    pub fn min_zone_size(&self, t: i32) -> i32 {
        let s = self.st.borrow();
        s.tuning.min_size.get(t as usize).copied().unwrap_or(0)
    }

    /// Slot 20 `GetMaxZoneSize(type)`. [choice] Out of range reads 0.
    pub fn max_zone_size(&self, t: i32) -> i32 {
        let s = self.st.borrow();
        s.tuning.max_size.get(t as usize).copied().unwrap_or(0)
    }

    /// The failure map 1 entries (`(result, zone) → count`), for tests.
    pub fn failure_map1(&self) -> Vec<((u8, u8), u32)> {
        self.st.borrow().failure1.iter().map(|(&k, &v)| (k, v)).collect()
    }

    // ---- messages (§5.1) ----

    /// `DoMessage` (§5.1): the dezone under a newly inserted network
    /// occupant. Always returns true (AL = 1).
    pub fn do_message(&self, m: &ZonesMessage) -> bool {
        if m.id == MSG_INSERT_OCCUPANT && m.data3 == NETWORK_OCCUPANT_TYPE {
            if let Some(occ) = m.occupant {
                // Queried for cISC4NetworkOccupant; only an occupant that has
                // none of the flags 0x80200090 is dezoned.
                if self.host.network_has_any_flag(occ, NO_DEZONE_FLAGS) == Some(false) {
                    let rect = self.host.occupant_bounding_cells(occ);
                    let mut region = super::CellRegion::new(rect, true);
                    // PlaceZone(region, 0, commit, skipFunds, !checkSlope,
                    // skipSizeCheck, checkLotInside).
                    self.place_zone(&mut region, 0, true, true, false, true, true);
                }
            }
        }
        true
    }

    // ---- PlaceZone (§6) ----

    /// Slot 11 `PlaceZone` (§6). `region` is modified in place (rejected bits
    /// cleared). The remaining arguments are `commit`, `skipFunds`,
    /// `checkSlope`, `skipSizeCheck`, `checkLotInside`.
    #[allow(clippy::too_many_arguments)]
    pub fn place_zone(
        &self,
        region: &mut super::CellRegion,
        zone_type: i32,
        commit: bool,
        skip_funds: bool,
        check_slope: bool,
        skip_size_check: bool,
        check_lot_inside: bool,
    ) -> PlaceZoneResult {
        // ZG2 [choice] (§6 "No city-bounds test", recommended): the original
        // skips only the cells outside the region's own bitmap or with their
        // bit clear and reads (on a commit writes) the zone grid unchecked, so
        // a set cell outside the city reads and writes outside the grid. Such
        // a call fails here with error 0 before anything else (no funds read,
        // no cell examined, the region untouched). The glue refuses a bad
        // §11.4 layout or an oversized region the same way (fc4_zones_place_zone).
        let (cx, cz) = self.host.city_cell_counts();
        if has_cell_outside_city(region, cx, cz) {
            return PlaceZoneResult::fail(place_error::OK);
        }
        let funds = self.host.total_funds();
        // ZG17 (§6 "Type out of range"): a type ≥ 16 (unsigned) is not a
        // failure; the argument itself is replaced by 0 and the call proceeds
        // as a dezone, with type 0's size limits.
        let zone_type = if (zone_type as u32) >= 16 { 0 } else { zone_type };
        let idx = zone_type as usize;

        // Snapshot the tuning values needed below, so no borrow is held
        // across a host call. `destroy_cost` is read per cell in the loop,
        // because it is indexed by the cell's old type, not by `type`.
        let (min, max, create, max_slope) = {
            let s = self.st.borrow();
            (
                s.tuning.min_size[idx],
                s.tuning.max_size[idx],
                s.tuning.create_cost[idx],
                s.tuning.max_slope[idx],
            )
        };

        let rect = region.rect;
        let mut cost: i64 = 0;
        let mut accepted: Vec<(i32, i32, i32)> = Vec::new(); // (old, x, z)
        let mut demolish: Vec<u64> = Vec::new(); // unique, insertion order
        let mut error = place_error::OK;

        // ZG17 (§6): a failed size check is an ABORT, not an early return. The
        // cell loop does not run (nothing accepted, nothing to demolish, the
        // region untouched), but the post-loop block below runs in full: the
        // type-14 landfill cost of the whole request region, skipFunds, the
        // funds test (error 1 replaces 3 or 4) and both outputs.
        if !skip_size_check {
            let w = rect.x2.wrapping_sub(rect.x1).wrapping_add(1);
            let h = rect.z2.wrapping_sub(rect.z1).wrapping_add(1);
            if w < min || h < min {
                error = place_error::TOO_SMALL;
            } else if w > max || h > max {
                error = place_error::TOO_LARGE;
            }
        }

        let grid = self.grid();
        // After a size ABORT no cell is examined.
        let scan = error == place_error::OK;

        'scan: for x in rect.x1..=rect.x2 {
            if !scan {
                break;
            }
            for z in rect.z1..=rect.z2 {
                // ZG2: only the cells of the region's bitmap with their bit set
                // (every such cell is inside the city, tested above).
                if !region.includes(x, z) {
                    continue;
                }
                let mut old = grid.as_ref().map_or(0, |g| i32::from(g.get_cell_value(x, z)));
                if (old as u32) >= 16 {
                    old = 0;
                }
                let tile = self.host.find_network_tile(x, z);
                if let Some(tile) = tile {
                    if self.host.tile_occupant_flag_set(tile, 1) && zone_type != 14 {
                        if zone_type != 0 {
                            region.set(x, z, false); // reject
                            continue;
                        } else if self.host.tile_is_of_type(tile, super::network_kind::STREET) {
                            cost = cost.wrapping_add(self.host.tile_demolition_cost(tile));
                        }
                    }
                }
                if self.host.cell_has_water(x, z) {
                    if zone_type == 14 {
                        error = place_error::LANDFILL_ON_WATER;
                        break 'scan;
                    }
                    if zone_type != 0 && zone_type != 12 {
                        region.set(x, z, false); // reject
                        continue;
                    }
                }
                if check_slope && zone_type != 0 && zone_type != 15 {
                    let s = self.host.area_slope(CellRect::new(x, z, x, z), true);
                    // §6: "if not (s <= maxSlope[type]): reject (NaN rejects)",
                    // so the negated compare is the specified behaviour.
                    #[allow(clippy::neg_cmp_op_on_partial_ord)]
                    let rejects = !(s <= max_slope);
                    if rejects {
                        region.set(x, z, false); // reject (NaN rejects)
                        continue;
                    }
                }
                let lot = self.host.lot_manager_get_lot(x, z);
                match lot {
                    None => {
                        let lf = self.host.landfill_at(x, z);
                        if let Some(lf) = lf {
                            if self.host.occupant_can_remove(lf, 2) {
                                // accept
                            } else {
                                error = place_error::BLOCKED;
                                break 'scan;
                            }
                        } else if zone_type == 14 && tile.is_some() {
                            error = place_error::BLOCKED;
                            break 'scan;
                        }
                        // accept
                    }
                    Some(lot) => {
                        if check_lot_inside && !rect_inside(self.host.lot_bounding_rect(lot), rect) {
                            if zone_type == 14 {
                                error = place_error::BLOCKED;
                                break 'scan;
                            }
                            region.set(x, z, false); // reject
                            continue;
                        }
                        let zt = self.host.lot_zone_type(lot);
                        if zt == 11 || zt == 12 || zt == 15 {
                            if zone_type == 14 {
                                error = place_error::BLOCKED;
                                break 'scan;
                            }
                            region.set(x, z, false); // reject
                            continue;
                        }
                        if let Some(cfg) = self.host.lot_configuration(lot) {
                            if !self.host.cfg_compatible_with_zone(cfg, zone_type) || zone_type == 0 {
                                let mut rejected = false;
                                if check_slope {
                                    if let Some(b) = self.host.lot_building(lot) {
                                        if !self.host.occupant_can_remove(b, 2) {
                                            rejected = true;
                                        }
                                    }
                                }
                                if rejected {
                                    region.set(x, z, false); // reject
                                    continue;
                                }
                                if !demolish.contains(&lot) {
                                    demolish.push(lot);
                                }
                            }
                        }
                        // accept
                    }
                }
                // accept: append (old, x, z); cost when the type changes.
                accepted.push((old, x, z));
                if old != zone_type {
                    let dc = {
                        let s = self.st.borrow();
                        s.tuning.destroy_cost.get(old as usize).copied().unwrap_or(0)
                    };
                    cost = cost.wrapping_add(create).wrapping_add(dc);
                }
            }
        }
        // Post-loop block (also after an ABORT).
        if !demolish.is_empty() {
            let c = self
                .host
                .demolish_lots(false, &demolish, check_slope, DEMOLISH_PRICE_FLAGS);
            cost = cost.wrapping_add(c);
        }
        if zone_type == 14 {
            cost = cost.wrapping_add(self.host.landfill_zoning_cost(region));
        }
        if skip_funds {
            cost = 0;
        } else if cost > funds {
            error = place_error::FUNDS;
        }

        let mut out_lots = demolish.clone();
        out_lots.sort_unstable();

        let ok = error == place_error::OK;
        if ok && commit {
            if !self.host.withdraw_funds(cost) {
                return PlaceZoneResult {
                    ok: false,
                    cost,
                    error: place_error::FUNDS,
                    out_lots,
                };
            }
            if !accepted.is_empty() {
                let mut changed = super::CellRegion::new(rect, false);
                {
                    let mut s = self.st.borrow_mut();
                    for &(old, x, z) in &accepted {
                        changed.set(x, z, true);
                        if let Some(g) = &s.grid {
                            g.set(x, z, zone_type as i8);
                        }
                        if let Some(c) = s.zone_count.get_mut(old as usize) {
                            *c = c.wrapping_sub(1);
                        }
                        s.zone_count[idx] = s.zone_count[idx].wrapping_add(1);
                    }
                }
                self.host
                    .demolish_lots(true, &demolish, check_slope, DEMOLISH_COMMIT_FLAGS);
                self.host
                    .zone_developer_do_parcellization(&changed, zone_type, check_slope);
                self.host.post_zoning_change(changed, zone_type, check_slope);
            }
        }

        PlaceZoneResult {
            ok,
            cost,
            error,
            out_lots,
        }
    }

    // ---- persistence (§10.1) ----

    /// `Write` (§10.1). Returns `GetError() = 0`.
    pub fn write_record(&self, out: &mut dyn OutStream) -> bool {
        let s = self.st.borrow();
        let mut ok = out.put_u16(MANAGER_RECORD_VERSION);
        // 2. reference: zone grid.
        let gref = s.grid.as_ref().map_or(RefValue::Null, |g| g.save_ref());
        ok &= out.put_ref(&gref);
        // 3..5. counts.
        for a in [&s.zone_count, &s.undeveloped, &s.unnamed_per_type] {
            for &v in a {
                ok &= out.put_u32(v);
            }
        }
        // 6. four scalars.
        for v in [s.undeveloped_total, s.unnamed1, s.abandoned_tile_count, s.unnamed2] {
            ok &= out.put_u32(v);
        }
        // 7. min then max sizes (s32).
        for &v in &s.tuning.min_size {
            ok &= out.put_i32(v);
        }
        for &v in &s.tuning.max_size {
            ok &= out.put_i32(v);
        }
        // 8. creation then destruction costs (s64).
        for &v in &s.tuning.create_cost {
            ok &= out.put_i64(v);
        }
        for &v in &s.tuning.destroy_cost {
            ok &= out.put_i64(v);
        }
        // 9. texture instances then drag colours (u32).
        for &v in &s.tuning.texture {
            ok &= out.put_u32(v);
        }
        for &v in &s.tuning.drag_color {
            ok &= out.put_u32(v);
        }
        // 10. display alpha (u8).
        ok &= out.put_u8(s.tuning.display_alpha);
        // 11, 12. the two failure maps (ordered by result, then zone).
        for map in [&s.failure1, &s.failure2] {
            ok &= out.put_u32(map.len() as u32);
            for (&(result, zone), &count) in map.iter() {
                ok &= out.put_u8(zone);
                ok &= out.put_u8(result);
                ok &= out.put_u32(count);
            }
        }
        // 13. five service references. The state borrow is dropped first: the
        // host supplies these references and may re-enter us (foundations
        // §19.3 rule 4).
        drop(s);
        for svc in [
            super::Service::City,
            super::Service::OccupantManager,
            super::Service::Terrain,
            super::Service::Pollution,
            super::Service::Budget,
        ] {
            ok &= out.put_ref(&self.host.service_ref(svc));
        }
        ok & (out.error() == 0)
    }

    /// `Read` (§10.1). Sets the loaded flag first ("sets the loaded flag;
    /// version ≠ 1 → false"), so a failed read leaves it set too; a version
    /// other than 1 fails without reading more. A stream error fails and keeps
    /// the rest of the previous state; references already read are released.
    pub fn read_record(&self, inp: &mut dyn InStream) -> bool {
        self.st.borrow_mut().loaded = true;
        let Some(version) = inp.get_u16() else { return false };
        if version != MANAGER_RECORD_VERSION {
            return false;
        }
        // 2. zone grid reference (IID 0xC9BD939A).
        let grid = match inp.get_ref_as(IID_SIM_GRID) {
            Some(r) if !r.is_null() => <dyn GridProvider<i8>>::resolve_grid(&*self.host, &r),
            Some(_) => None,
            None => return false,
        };
        let rd = |inp: &mut dyn InStream| -> Option<Mgr> {
            let mut m = Mgr { grid, ..Mgr::default() };
            for a in [&mut m.zone_count, &mut m.undeveloped, &mut m.unnamed_per_type] {
                for slot in a.iter_mut() {
                    *slot = inp.get_u32()?;
                }
            }
            m.undeveloped_total = inp.get_u32()?;
            m.unnamed1 = inp.get_u32()?;
            m.abandoned_tile_count = inp.get_u32()?;
            m.unnamed2 = inp.get_u32()?;
            for slot in m.tuning.min_size.iter_mut() {
                *slot = inp.get_i32()?;
            }
            for slot in m.tuning.max_size.iter_mut() {
                *slot = inp.get_i32()?;
            }
            for slot in m.tuning.create_cost.iter_mut() {
                *slot = inp.get_i64()?;
            }
            for slot in m.tuning.destroy_cost.iter_mut() {
                *slot = inp.get_i64()?;
            }
            for slot in m.tuning.texture.iter_mut() {
                *slot = inp.get_u32()?;
            }
            for slot in m.tuning.drag_color.iter_mut() {
                *slot = inp.get_u32()?;
            }
            // ZG3: the saved alpha lasts until Init's tuning read (§10.1);
            // the default alpha is not saved, so Read leaves it alone.
            m.tuning.display_alpha = inp.get_u8()?;
            for map in [&mut m.failure1, &mut m.failure2] {
                let n = inp.get_u32()?;
                for _ in 0..n {
                    let zone = inp.get_u8()?;
                    let result = inp.get_u8()?;
                    let count = inp.get_u32()?;
                    map.entry((result, zone)).or_insert(count); // unique: duplicates dropped
                }
            }
            // 13. five service references: city, occupant manager, terrain,
            // pollution, budget (the host keeps them; the core does not store
            // them here).
            for _ in 0..SERVICE_REFS {
                let _ = inp.get_ref()?;
            }
            Some(m)
        };
        let Some(m) = rd(inp) else { return false };
        if inp.error() != 0 {
            return false;
        }
        let mut s = self.st.borrow_mut();
        s.grid = m.grid;
        s.zone_count = m.zone_count;
        s.undeveloped = m.undeveloped;
        s.unnamed_per_type = m.unnamed_per_type;
        s.undeveloped_total = m.undeveloped_total;
        s.unnamed1 = m.unnamed1;
        s.abandoned_tile_count = m.abandoned_tile_count;
        s.unnamed2 = m.unnamed2;
        // The max slopes and the default alpha are not saved (§10.1): Read
        // keeps the current ones.
        s.tuning = Tuning {
            max_slope: s.tuning.max_slope,
            ..m.tuning
        };
        s.failure1 = m.failure1;
        s.failure2 = m.failure2;
        s.loaded = true;
        true
    }
}

/// The number of service references the zone manager reads (§10.1 field 13):
/// city, occupant manager, terrain, pollution, budget.
const SERVICE_REFS: u32 = 5;

impl SaveObject for ZoneManager {
    fn class_id(&self) -> u32 {
        CLSID_ZONE_MANAGER
    }
    fn write(&self, out: &mut dyn OutStream) -> bool {
        self.write_record(out)
    }
    fn read(&self, inp: &mut dyn InStream) -> bool {
        self.read_record(inp)
    }
    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}
