// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The zone developer / lot manager (`cSC4ZoneDeveloper`, class
//! `0x498F9B01`): the lot manager (cell → lot map, master lot list, lot
//! creation and deletion, the lookups and lot lists of §8, the safe lot
//! iterators, the save record §10.2) and the zone developer (parcellization,
//! §7: `DoParcellization`, `HighlightParcels`, `OnZone`, `DetermineLotSize`,
//! `ExamineBoundaries`, `CreateParcellizableRegion`, `SimpleParcellize`,
//! `Parcellize`, `DrawStreet`, `ExtendAsStreets`, `LayInJogStreets`,
//! `ExistsConnectionOfType` / `GetNetwork`, `CreateLotToFaceRoad`,
//! `CreateLotRemnants`, `OnDezone`) and its messages (§5.2).
//!
//! Every method takes `&self` and the state lives behind a [`RefCell`]: the
//! two classes reach each other through the host (§13.1), and no borrow is
//! held across a host call or a self-call. The parcellization working state
//! (the restriction region, the orientation, the lot-size parameters, the
//! current zone type, the real-mode flag and the virtual-lot maps) lives in
//! the state for the whole `OnZone` / `OnDezone`, so the leaf methods that
//! re-enter it (`CreateLotIfPossible`, `CanCreateLot`, `DrawStreet`) read it
//! with short borrows.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use super::parcel::{block_depth, depth_rule, zone_density, DEFAULT_BLOCK_DEPTH, MAX_WALK_STEPS};
use super::{
    is_rci_zone, lot_code, net, network_kind, ZonesHost, ZonesMessage, IID_LOT, MSG_INSERT_OCCUPANT, MSG_POST_CITY_INIT,
    MSG_VIEW, NETWORK_OCCUPANT_TYPE, NO_DEZONE_FLAGS, BUILDING_TYPE, OBSTACLE_TYPE_2, MAX_CAPACITY_GRID_CLASS,
    MAX_CAPACITY_GRID_INSTANCE,
};
use crate::development::{dev_type, BuildingProfile};
use crate::grid::{CellRect, GridProvider, TractGrid};
use crate::rng::{derive_seed, file_seed, FixedSeeds, RzRandom, SeedSource};
use crate::stl::StlHashSet;
use crate::stream::{InStream, OutStream, RefValue};
use crate::x87::{ftol, r24, rhe};

/// The zone developer's record version (§10.2 field 1).
pub const DEVELOPER_RECORD_VERSION: u16 = 1;

/// `GetRandomLot`'s try limit (§8.2 slot 13, §9 Z1).
pub const MAX_RANDOM_LOT_TRIES: u32 = 999;

/// The XOR constant the zone developer's file-local generator mixes into its
/// seed (§9, foundations §6.3): `0x8C09C68A`.
pub const RNG_SEED_XOR: u32 = file_seed::ZONE_DEVELOPER;

/// Which random draw of §9 a [`Draw`] record is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawKind {
    /// `GetRandomLot` (lot manager slot 13): `U(cellsX)` then `U(cellsZ)` per
    /// try (§9 Z1).
    Z1,
    /// `CreateLotToFaceRoad` with an empty list: `U(4)` on entry (§9 Z2, and
    /// its contexts Z2a, Z2b, Z2c).
    Z2,
}

impl DrawKind {
    pub fn name(self) -> &'static str {
        match self {
            DrawKind::Z1 => "Z1",
            DrawKind::Z2 => "Z2",
        }
    }
}

/// One random draw (the debug trace of [`ZoneDeveloperStatics`]): the
/// argument `n` of `RandomUint32Uniform(n)` and the value drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Draw {
    pub kind: DrawKind,
    pub arg: u32,
    pub value: u32,
}

/// The zone developer's file-local `cRZRandom` (`0x00B4B398`, §9): one object
/// shared by every instance and by whoever else needs it (the plugin glue
/// hands the same [`SharedZoneStatics`] to the tract developer's host, which
/// models R9 / Z2c on it), with a draw trace.
#[derive(Clone, Debug)]
pub struct ZoneDeveloperStatics {
    /// The file-local `cRZRandom`.
    pub rng: RzRandom,
    /// Every draw, when tracing is on (debug and conformance).
    pub trace: Option<Vec<Draw>>,
}

/// The statics as one object handed to every instance.
pub type SharedZoneStatics = Rc<RefCell<ZoneDeveloperStatics>>;

impl ZoneDeveloperStatics {
    /// The statics with the file-local generator seeded with `seed` and no
    /// trace. The original's seed is the local wall clock XOR
    /// [`RNG_SEED_XOR`]; a clock value of 0 gives `0x8C09C68A`.
    pub fn new(seed: u32, seeds: &dyn SeedSource) -> SharedZoneStatics {
        Rc::new(RefCell::new(ZoneDeveloperStatics {
            rng: RzRandom::new(seed, seeds),
            trace: None,
        }))
    }

    /// The statics seeded from the session seed (foundations §6.3 *[choice]*):
    /// `derive_seed(session, 0x8C09C68A)`, the original's clock XOR
    /// [`RNG_SEED_XOR`] with the session seed in place of the clock. Session 0
    /// gives seed `0x8C09C68A`.
    pub fn from_session(session: u32, seeds: &dyn SeedSource) -> SharedZoneStatics {
        ZoneDeveloperStatics::new(derive_seed(session, RNG_SEED_XOR), seeds)
    }

    /// Appends a draw to the trace (when tracing is on).
    pub fn record(&mut self, kind: DrawKind, arg: u32, value: u32) {
        if let Some(t) = self.trace.as_mut() {
            t.push(Draw { kind, arg, value });
        }
    }

    /// `RandomUint32Uniform(n)` on the file-local generator, traced as `kind`.
    pub fn uniform(&mut self, kind: DrawKind, n: u32) -> u32 {
        let v = self.rng.uniform_u32(n);
        self.record(kind, n, v);
        v
    }
}

/// `SetOptions(alternateOrientation, placeStreets, customZoneSize)` (§7 slot
/// 7). Defaults (§3.2): false, **true**, false.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub alternate_orientation: bool,
    pub place_streets: bool,
    pub custom_zone_size: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            alternate_orientation: false,
            place_streets: true,
            custom_zone_size: false,
        }
    }
}

/// The lot-size parameters (§3.2), overwritten by `DetermineLotSize` (§7.2.1)
/// before every use. Defaults: width 2, depth 2, min depth 1, max depth 3,
/// block depth 6.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LotSize {
    pub width: i32,
    pub depth: i32,
    pub min_depth: i32,
    pub max_depth: i32,
    pub block: i32,
}

impl Default for LotSize {
    fn default() -> LotSize {
        LotSize {
            width: 2,
            depth: 2,
            min_depth: 1,
            max_depth: 3,
            block: DEFAULT_BLOCK_DEPTH,
        }
    }
}

/// The orientation result of `ExamineBoundaries` (§3.2, §7.3): the flag, the
/// facing side D, the start side Q (always perpendicular to D), the four side
/// scores S and the four side conditions C.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Orientation {
    pub flag: bool,
    pub d: i32,
    pub q: i32,
    pub scores: [i32; 4],
    pub conditions: [i32; 4],
}

impl Default for Orientation {
    fn default() -> Orientation {
        Orientation {
            flag: false,
            d: 0,
            q: 0,
            scores: [0; 4],
            conditions: [0; 4],
        }
    }
}

/// A virtual lot (§7.1, §8.5): the `{x, z, facing}` recorded in highlight
/// mode instead of creating a lot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VirtualLot {
    pub x: i32,
    pub z: i32,
    pub facing: i32,
}

/// The traffic simulator's desired lot insertion point (§8.5 step 4): the
/// iterator's node and whether its list is set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InsertionPoint {
    /// The node the iterator points at (`None`: the list head / end).
    pub node: Option<u64>,
    /// Whether the iterator's list is set (a null list inserts nothing and
    /// leaves the count unchanged *[quirk]*, §8.5 step 4).
    pub list: bool,
}

/// A safe lot iterator handle (§8.3 slot 15): its position in the master list.
#[derive(Clone, Copy, Debug)]
struct SafeIter {
    handle: u64,
    index: usize,
}

/// The zone developer / lot manager's mutable state (§3.2).
struct Dev {
    // ---- lot manager ----
    /// Cell → lot map: `map_x` rows of `map_z` entries (x outer, z inner).
    cell_map: Vec<Option<u64>>,
    map_x: i32,
    map_z: i32,
    /// The master lot list (holds the manager's one reference per lot).
    master: Vec<u64>,
    safe_iters: Vec<SafeIter>,
    next_iter: u64,
    max_capacity: Option<Rc<dyn TractGrid<u16>>>,
    // ---- zone developer working state ----
    options: Options,
    lot_size: LotSize,
    /// Test/conformance bypass of `DetermineLotSize` (§7.2.1): when set,
    /// `on_zone` keeps the preset [`Dev::lot_size`] instead of recomputing it.
    bypass_lot_size: bool,
    orientation: Orientation,
    current_zone: i32,
    real_mode: bool,
    restriction: Option<super::CellRegion>,
    highlight_point: Option<(i32, i32)>,
    /// Virtual lots created in highlight mode, keyed by cell.
    virtual_created: BTreeMap<(i32, i32), VirtualLot>,
    /// Cells virtually deleted in highlight mode, keyed by cell.
    virtual_deleted: BTreeMap<(i32, i32), VirtualLot>,
    view_message_value: u32,
    initialised: bool,
    loaded: bool,
}

impl Default for Dev {
    fn default() -> Dev {
        Dev {
            cell_map: Vec::new(),
            map_x: 0,
            map_z: 0,
            master: Vec::new(),
            safe_iters: Vec::new(),
            next_iter: 1,
            max_capacity: None,
            options: Options::default(),
            lot_size: LotSize::default(),
            bypass_lot_size: false,
            orientation: Orientation::default(),
            current_zone: 0,
            real_mode: true,
            restriction: None,
            highlight_point: None,
            virtual_created: BTreeMap::new(),
            virtual_deleted: BTreeMap::new(),
            view_message_value: 0,
            initialised: false,
            loaded: false,
        }
    }
}

/// One zone developer / lot manager (§1).
pub struct ZoneDeveloper {
    host: Rc<dyn ZonesHost>,
    statics: SharedZoneStatics,
    st: RefCell<Dev>,
}

// ---- small geometry helpers --------------------------------------------------------------------

fn clamp_i32(v: i64) -> i32 {
    v.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

fn rect_overlaps(a: CellRect, b: CellRect) -> bool {
    a.x1 <= b.x2 && b.x1 <= a.x2 && a.z1 <= b.z2 && b.z1 <= a.z2
}

/// The `rhe(r24(r24(x × 0.0625) − 0.5))` cell of `GetOccupantLot` (§8.2 slot
/// 14): `c = ⌊x / 16⌋`, with the `+1` when `a = c + 1` exactly.
fn occupant_cell(p: f32) -> i32 {
    let a = r24(f64::from(p) * 0.0625) as f32;
    let mut c = rhe(r24(f64::from(a) - 0.5) as f32);
    if f64::from(a) == f64::from(c + 1) {
        c += 1;
    }
    c
}

/// The `rhe(r24(r24(x × 0.0625) − 0.499))` cell of
/// `GetLotSavvyTerrainAltitude` (§8.7 slot 18), constant `0x3EFF7CEE` =
/// 0.49900001.
fn savvy_cell(p: f32) -> i32 {
    rhe((r24(f64::from(p) * 0.0625) - f64::from(0.499_000_011_920_929_f32)) as f32)
}

impl ZoneDeveloper {
    /// The constructor's state (§3.2): empty maps, default options and
    /// lot-size parameters, real mode, no restriction.
    pub fn new(host: Rc<dyn ZonesHost>, statics: SharedZoneStatics) -> ZoneDeveloper {
        ZoneDeveloper {
            host,
            statics,
            st: RefCell::new(Dev::default()),
        }
    }

    pub fn host(&self) -> &Rc<dyn ZonesHost> {
        &self.host
    }

    pub fn statics(&self) -> SharedZoneStatics {
        self.statics.clone()
    }

    pub fn is_initialised(&self) -> bool {
        self.st.borrow().initialised
    }

    pub fn is_loaded(&self) -> bool {
        self.st.borrow().loaded
    }

    pub fn is_real_mode(&self) -> bool {
        self.st.borrow().real_mode
    }

    pub fn options(&self) -> Options {
        self.st.borrow().options
    }

    pub fn lot_size(&self) -> LotSize {
        self.st.borrow().lot_size
    }

    /// Replaces the lot-size parameters (tests and hosts that bypass
    /// `DetermineLotSize`, §7.2.1) and sets the bypass flag, so `on_zone`
    /// keeps them instead of recomputing them.
    pub fn set_lot_size(&self, ls: LotSize) {
        let mut s = self.st.borrow_mut();
        s.lot_size = ls;
        s.bypass_lot_size = true;
    }

    /// Clears the bypass flag of [`ZoneDeveloper::set_lot_size`], so the next
    /// `on_zone` runs `DetermineLotSize` again.
    pub fn set_lot_size_from_developers(&self) {
        self.st.borrow_mut().bypass_lot_size = false;
    }

    pub fn orientation(&self) -> Orientation {
        self.st.borrow().orientation
    }

    pub fn current_zone(&self) -> i32 {
        self.st.borrow().current_zone
    }

    /// The stored `0xA6B79602` value (§5.2; unused headless).
    pub fn view_message_value(&self) -> u32 {
        self.st.borrow().view_message_value
    }

    /// The master lot list, in order (§8.1).
    pub fn master_lots(&self) -> Vec<u64> {
        self.st.borrow().master.clone()
    }

    /// The virtual lots recorded in highlight mode, keyed by cell (§7.1).
    pub fn virtual_created(&self) -> Vec<((i32, i32), VirtualLot)> {
        self.st.borrow().virtual_created.iter().map(|(&k, &v)| (k, v)).collect()
    }

    pub fn virtual_deleted(&self) -> Vec<((i32, i32), VirtualLot)> {
        self.st.borrow().virtual_deleted.iter().map(|(&k, &v)| (k, v)).collect()
    }

    fn map_index(&self, x: i32, z: i32) -> Option<usize> {
        let s = self.st.borrow();
        if x < 0 || z < 0 || x >= s.map_x || z >= s.map_z {
            return None;
        }
        Some(x as usize * s.map_z as usize + z as usize)
    }

    /// The lot at a cell (no origin check), or `None`.
    fn lot_at(&self, x: i32, z: i32) -> Option<u64> {
        self.map_index(x, z)
            .and_then(|i| self.st.borrow().cell_map.get(i).copied())
            .flatten()
    }

    // ---- lifecycle (§1.2) ----

    /// `Init` (slot 3, §1.2) without the 3D view. Idempotent. False if a
    /// service is missing.
    pub fn init(&self) -> bool {
        if self.st.borrow().initialised {
            return true;
        }
        // 1–2. Take the services (no 3D view).
        if !self.host.developer_acquire_services() {
            return false;
        }
        let (cx, cz) = self.host.city_cell_counts();
        {
            let mut s = self.st.borrow_mut();
            // 3. If the cell → lot map is empty (new city), size it; else
            //    Init every lot at its origin cell, x outer, z inner.
            if s.cell_map.is_empty() {
                s.map_x = cx.max(0);
                s.map_z = cz.max(0);
                s.cell_map = vec![None; (s.map_x as usize) * (s.map_z as usize)];
            } else {
                let (mx, mz) = (s.map_x, s.map_z);
                let origins: Vec<u64> = {
                    let mut v = Vec::new();
                    for x in 0..mx {
                        for z in 0..mz {
                            let i = x as usize * mz as usize + z as usize;
                            if let Some(lot) = s.cell_map[i] {
                                if (x, z) == self.host.lot_location(lot) {
                                    v.push(lot);
                                }
                            }
                        }
                    }
                    v
                };
                for lot in origins {
                    self.host.lot_init(lot);
                }
            }
        }
        // 4. If there is no maximum-capacity grid, construct a uint16 grid
        //    (tract size 4, instance 0x69D5C456).
        if self.st.borrow().max_capacity.is_none() {
            if let Some(g) = <dyn GridProvider<u16>>::create_grid(&*self.host, MAX_CAPACITY_GRID_INSTANCE, 4) {
                self.st.borrow_mut().max_capacity = Some(g);
            }
        }
        // 5. InitBoundaryRules: UI, left out (headless).
        // 6. Subscribe.
        self.host.subscribe(
            super::Component::ZoneDeveloper,
            &[MSG_INSERT_OCCUPANT, MSG_POST_CITY_INIT, MSG_VIEW],
        );
        // 7. Cheats: UI/debug, left out (headless).
        // 8. Set the initialised flag.
        self.st.borrow_mut().initialised = true;
        true
    }

    /// `Shutdown` (slot 4, §1.2). Returns AL = 1.
    pub fn shutdown(&self) -> bool {
        if !self.st.borrow().initialised {
            return true;
        }
        self.st.borrow_mut().initialised = false;
        self.host.unsubscribe(
            super::Component::ZoneDeveloper,
            &[MSG_INSERT_OCCUPANT, MSG_POST_CITY_INIT, MSG_VIEW],
        );
        self.host.release_network_tools();
        // Release each lot of the map once (first sighting, x outer, z inner)
        // [inferred], and empty the map and the master list.
        let mut seen = BTreeSet::new();
        {
            let s = self.st.borrow();
            for x in 0..s.map_x {
                for z in 0..s.map_z {
                    let i = x as usize * s.map_z as usize + z as usize;
                    if let Some(lot) = s.cell_map[i] {
                        if seen.insert(lot) {
                            self.host.lot_shutdown(lot);
                            self.host.release(lot);
                        }
                    }
                }
            }
        }
        {
            let mut s = self.st.borrow_mut();
            s.cell_map.clear();
            s.map_x = 0;
            s.map_z = 0;
            s.master.clear();
            s.safe_iters.clear();
        }
        if self.st.borrow().max_capacity.is_some() {
            self.host.shutdown_grid(MAX_CAPACITY_GRID_CLASS);
            self.st.borrow_mut().max_capacity = None;
        }
        self.host.developer_release_services();
        true
    }

    // ---- messages (§5.2) ----

    /// `DoMessage` (§5.2). Always returns true (AL = 1).
    pub fn do_message(&self, m: &ZonesMessage) -> bool {
        match m.id {
            MSG_POST_CITY_INIT => {
                self.host.acquire_network_tools();
                if self.st.borrow().loaded {
                    let (mx, mz) = {
                        let s = self.st.borrow();
                        (s.map_x, s.map_z)
                    };
                    let origins: Vec<u64> = {
                        let s = self.st.borrow();
                        let mut v = Vec::new();
                        for x in 0..mx {
                            for z in 0..mz {
                                let i = x as usize * mz as usize + z as usize;
                                if let Some(lot) = s.cell_map[i] {
                                    if (x, z) == self.host.lot_location(lot) {
                                        v.push(lot);
                                    }
                                }
                            }
                        }
                        v
                    };
                    for lot in origins {
                        self.host.lot_post_city_init(lot);
                    }
                }
            }
            MSG_VIEW => {
                self.st.borrow_mut().view_message_value = m.value;
            }
            MSG_INSERT_OCCUPANT if m.data3 == NETWORK_OCCUPANT_TYPE => {
                self.on_insert_occupant(m);
            }
            // 0x230E27AC cheat: UI/debug, left out.
            _ => {}
        }
        true
    }

    /// InsertOccupant of a network occupant (§5.2 step 1–3).
    fn on_insert_occupant(&self, m: &ZonesMessage) {
        let Some(occ) = m.occupant else { return };
        // 1. If it has any flag of 0x80200090, stop.
        if self.host.network_has_any_flag(occ, NO_DEZONE_FLAGS) == Some(true) {
            return;
        }
        // 2. cell := PositionToCell(GetPosition()); if a lot covers that one
        //    cell, delete it and (for an RCI zone) create remnants.
        let pos = self.host.occupant_position(occ);
        if let Some((x, z)) = self.host.position_to_cell(pos) {
            if let Some(lot) = self.lot_at(x, z) {
                let rect = self.host.lot_bounding_rect(lot);
                let facing = self.host.lot_facing(lot);
                let zone = self.host.lot_zone_type(lot);
                self.st.borrow_mut().current_zone = zone;
                self.delete_lot(lot);
                if is_rci_zone(zone) {
                    self.create_lot_remnants(rect, facing);
                }
                self.st.borrow_mut().current_zone = 0;
            }
        }
        // 3. By data4: 1 or 2 → 0x00731F60; 3 → 0x00732450.
        if matches!(m.data4, 1 | 2 | 3) {
            self.host.street_joining_helper(occ, m.data4);
        }
    }

    // ---- zone developer entry points (§7.1) ----

    /// Slot 7 `SetOptions(alternateOrientation, placeStreets,
    /// customZoneSize)`.
    pub fn set_options(&self, alternate_orientation: bool, place_streets: bool, custom_zone_size: bool) {
        self.st.borrow_mut().options = Options {
            alternate_orientation,
            place_streets,
            custom_zone_size,
        };
    }

    /// Slot 6 `DoParcellization(region, type, flag)` (§7.1).
    pub fn do_parcellization(&self, region: &super::CellRegion, zone_type: i32, flag: bool) {
        self.st.borrow_mut().current_zone = zone_type;
        if is_rci_zone(zone_type) {
            self.clear_highlight();
            self.on_zone(region, None);
        } else {
            self.on_dezone(region, flag);
        }
        self.st.borrow_mut().current_zone = 0;
    }

    /// Slot 5 `HighlightParcels(region, type, point, out)` (§7.1): the UI
    /// preview. `out` receives the highlighted region.
    pub fn highlight_parcels(
        &self,
        region: &super::CellRegion,
        zone_type: i32,
        point: Option<(i32, i32)>,
    ) -> Option<super::CellRegion> {
        {
            let mut s = self.st.borrow_mut();
            s.real_mode = false;
            s.current_zone = zone_type;
            s.highlight_point = point;
        }
        let mut out = None;
        if is_rci_zone(zone_type) {
            self.clear_highlight();
            out = self.on_zone(region, Some(region.compress()));
            // UpdateHighlightTextures: UI, left out.
        }
        self.st.borrow_mut().real_mode = true;
        out
    }

    fn clear_highlight(&self) {
        let mut s = self.st.borrow_mut();
        s.virtual_created.clear();
        s.virtual_deleted.clear();
    }

    /// `OnZone(region, out)` (§7.2). Returns the highlighted region when `out`
    /// was requested.
    fn on_zone(&self, region: &super::CellRegion, out: Option<super::CellRegion>) -> Option<super::CellRegion> {
        let custom = self.st.borrow().options.custom_zone_size;
        if custom {
            // Custom-size path (§7.2): the region becomes the restriction,
            // the orientation list is {D, Q, (D+2)%4, (Q+2)%4}, and
            // CreateLotToFaceRoad runs on the whole region rect (no draw: the
            // list is not empty). On failure with `out` given, *out := a new
            // all-false region over the rect.
            let rect = region.rect;
            let (d, q) = {
                let s = self.st.borrow();
                (s.orientation.d, s.orientation.q)
            };
            self.st.borrow_mut().restriction = Some(region.clone());
            let list = [d, q, (d + 2).rem_euclid(4), (q + 2).rem_euclid(4)];
            let created = self.create_lot_to_face_road(
                rect.x1,
                rect.z1,
                rect.x2 - rect.x1 + 1,
                rect.z2 - rect.z1 + 1,
                &list,
            );
            if let Some(lot) = created {
                self.host.release(lot);
            }
            self.st.borrow_mut().restriction = None;
            return match out {
                Some(_) if created.is_none() => Some(super::CellRegion::new(rect, false)),
                Some(r) => Some(r),
                None => None,
            };
        }

        // Normal path.
        self.host.post_defer_occupant_operations(true); // 1.
        // 2. CreateParcellizableRegion, cropped (CompressRegion).
        let cropped = self.create_parcellizable_region(region).compress();
        // 3. DetermineLotSize (unless a test/host bypasses it).
        if !self.st.borrow().bypass_lot_size {
            self.determine_lot_size(&cropped);
        }
        let rect = cropped.rect;
        // 4. placeStreets: ExtendAsStreets then LayInJogStreets.
        let place_streets = self.st.borrow().options.place_streets;
        if place_streets {
            self.st.borrow_mut().restriction = Some(cropped.clone());
            for &k in &network_kind::STREET_STEP_ORDER {
                self.extend_as_streets(rect, k);
            }
            let w = rect.x2 - rect.x1 + 1;
            let h = rect.z2 - rect.z1 + 1;
            if w > 2 && h > 2 {
                for &k in &network_kind::STREET_STEP_ORDER {
                    self.lay_in_jog_streets(rect, k);
                }
            }
            self.st.borrow_mut().restriction = None;
        }
        // 5. SubdivideTheZone: recompute the parcellizable region, split into
        //    components (each cropped), one empty region if none.
        let parcellizable = self.create_parcellizable_region(region).compress();
        let mut components = parcellizable.components();
        if components.is_empty() {
            components.push(super::CellRegion::empty());
        }
        // 6. For each component.
        let mut result = out;
        let highlight = !self.st.borrow().real_mode;
        let point = self.st.borrow().highlight_point;
        for comp in components {
            let skip = match (highlight, point) {
                (true, Some((px, pz))) => {
                    !comp.includes(px, pz)
                        && !comp.includes(px + 1, pz)
                        && !comp.includes(px - 1, pz)
                        && !comp.includes(px, pz + 1)
                        && !comp.includes(px, pz - 1)
                }
                _ => false,
            };
            if skip {
                if let Some(r) = result.as_mut() {
                    for &(x, z) in &comp.cells() {
                        r.set(x, z, false);
                    }
                }
                continue;
            }
            self.st.borrow_mut().restriction = Some(comp.clone());
            self.examine_boundaries();
            let zone = self.st.borrow().current_zone;
            match zone {
                1..=6 => self.parcellize(),
                7 => self.simple_parcellize(16),
                _ => self.simple_parcellize(6),
            }
            self.st.borrow_mut().restriction = None;
        }
        // 7. real mode: 0x0072B180(region) — not traced, left out.
        self.host.post_defer_occupant_operations(false); // 8.
        result
    }

    /// `DetermineLotSize(region)` (§7.2.1). No random draws.
    fn determine_lot_size(&self, region: &super::CellRegion) {
        let rect = region.rect;
        // 1. Centre.
        let cx = (i64::from(rect.x1) + i64::from(rect.x2)) / 2;
        let cz = (i64::from(rect.z1) + i64::from(rect.z2)) / 2;
        let (cx, cz) = (clamp_i32(cx), clamp_i32(cz));
        let zone = self.st.borrow().current_zone;

        // 2. Candidate developers by zone; shallow flag for 5–6.
        let (candidates, shallow) = match zone {
            1..=3 => (vec![dev_type::R1, dev_type::R2, dev_type::R3], false),
            4 => (vec![dev_type::CS1, dev_type::CS2, dev_type::CS3, dev_type::CO2, dev_type::CO3], false),
            5 | 6 => (vec![dev_type::CS1, dev_type::CS2, dev_type::CS3, dev_type::CO2, dev_type::CO3], true),
            _ => (Vec::new(), false),
        };
        let devs: Vec<u64> = candidates.iter().filter_map(|&t| self.host.bds_developer(t)).collect();

        // 3. Best developer.
        let mut best: Option<u64> = None;
        let mut best_desir = i16::MIN;
        let mut best_demand: i64 = 0;
        for &dev in &devs {
            let d = self.host.dev_estimate_desirability(dev, cx, cz);
            let demand = self.host.dev_source_demand(dev).map(|v| ftol(f64::from(v))).unwrap_or(0);
            match best {
                None => {
                    best = Some(dev);
                    best_desir = d;
                    best_demand = demand;
                }
                Some(_) => {
                    if i32::from(d) > i32::from(best_desir) || (d == best_desir && demand > best_demand) {
                        best = Some(dev);
                        best_desir = d;
                        best_demand = demand;
                    }
                }
            }
        }

        let mut ls = LotSize {
            width: 2,
            depth: 2,
            min_depth: 0,
            max_depth: 0,
            block: DEFAULT_BLOCK_DEPTH,
        };

        // 4–7 only run with a best developer.
        if let Some(best) = best {
            // 4. Stage.
            let mut stage = 1i32;
            if let Some(list) = self.host.dev_growth_list(best) {
                let mut found = false;
                for item in list {
                    let v = self.host.dev_desirability_value(best, item.x, item.z);
                    if i32::from(v) <= i32::from(best_desir) {
                        found = true;
                        break;
                    }
                    stage = item.max_stage;
                }
                let _ = found;
            }
            // 5. Lot configurations into a hash_set keyed by cfg id.
            let purpose = self.host.dev_purpose(best);
            let wealth = self.host.dev_wealth(best);
            let list = self.host.configurations_by_filter(purpose, wealth, stage);
            let mut set = StlHashSet::with_hint(100);
            let mut by_id: BTreeMap<u32, u64> = BTreeMap::new();
            for cfg in list {
                let id = self.host.cfg_id(cfg);
                set.insert(id);
                by_id.entry(id).or_insert(cfg);
            }
            let order: Vec<u32> = set.iter_order().into_iter().filter(|id| by_id.contains_key(id)).collect();
            let configs: Vec<u64> = order.iter().filter_map(|id| by_id.get(id).copied()).collect();

            // 6. Best configuration by capacity score.
            let shift = self.host.dev_desirability_shift(best);
            let k = (1i64 << shift) << shift;
            let mut chosen: Option<u64> = None;
            let mut best_score: i64 = 0;
            for &cfg in &configs {
                let (sx, sz) = self.host.cfg_size(cfg);
                let (sx, sz) = (sx as i64, sz as i64);
                if !(2 <= sz && sz <= 3 && sx <= 3) {
                    continue;
                }
                if shallow && sz > 2 {
                    continue;
                }
                let families = match self.host.cfg_primary_building_families(cfg) {
                    Some(f) => f,
                    None => continue,
                };
                let (mut mn, mut mx, mut any) = (u16::MAX, 0u16, false);
                for fam in families {
                    for bt in self.host.building_types_by_family(fam) {
                        let prof: BuildingProfile = self.host.building_profile(bt);
                        if u32::from(prof.purpose) == purpose && u32::from(prof.wealth) == wealth {
                            any = true;
                            mn = mn.min(prof.capacity);
                            mx = mx.max(prof.capacity);
                        }
                    }
                }
                if !any {
                    continue;
                }
                let prod = (sx * sz) as f64;
                let mean = f64::from((u32::from(mn) + u32::from(mx)) / 2);
                let v = ftol(r24(r24(f64::from(k as i32) / prod) * mean)) ;
                let score = (v - best_demand).abs();
                if chosen.is_none() || score < best_score {
                    chosen = Some(cfg);
                    best_score = score;
                }
            }

            // 7. Lot width/depth, then the depth widening.
            if let Some(cfg) = chosen {
                let (sx, sz) = self.host.cfg_size(cfg);
                ls.width = sx as i32;
                ls.depth = sz as i32;
            } else {
                ls.width = 2;
                ls.depth = 2;
            }
            ls.min_depth = ls.depth;
            ls.max_depth = ls.depth;
            let width = ls.width as u32;
            for &cfg in &configs {
                let (sx, sz) = self.host.cfg_size(cfg);
                if sx == 0 {
                    continue; // [choice] a width of 0 divides by zero in the original
                }
                if width % sx == 0 {
                    ls.min_depth = ls.min_depth.min(sz as i32);
                    ls.max_depth = ls.max_depth.max(sz as i32);
                }
            }
            if (ls.max_depth as u32).wrapping_sub(ls.min_depth as u32) < 2 {
                ls.min_depth = if ls.min_depth > 2 { ls.max_depth - 2 } else { 1 };
                ls.max_depth = ls.min_depth + 2;
            }
            // Release every configuration.
            for &cfg in &configs {
                self.host.release(cfg);
            }
        }

        // 8. Block depth.
        let density = zone_density(zone);
        let residential = best.is_some_and(|b| self.host.dev_purpose(b) == 1);
        ls.block = block_depth(density, ls.depth, residential);
        if ls.min_depth == 0 && ls.max_depth == 0 {
            // No best developer (zones 7–9, or none from the BDS): no
            // configurations listed [choice], lot size (2, 2), depths widened
            // to [1, 3].
            ls.min_depth = 1;
            ls.max_depth = 3;
        }
        self.st.borrow_mut().lot_size = ls;
    }

    /// `ExamineBoundaries()` (§7.3) on the restriction region; writes the
    /// orientation into the state.
    fn examine_boundaries(&self) {
        let (rect, place_streets) = {
            let s = self.st.borrow();
            (s.restriction.as_ref().map(|r| r.rect), s.options.place_streets)
        };
        let Some(rect) = rect else { return };
        let mut scores = [0i32; 4];
        let mut conds = [if place_streets { 0 } else { 1 }; 4];
        let (cx, cz) = self.host.city_cell_counts();

        // The outside rect: R's rect grown by one cell, not past the city edge.
        let ox1 = (rect.x1 - 1).max(0);
        let oz1 = (rect.z1 - 1).max(0);
        let ox2 = (rect.x2 + 1).min(cx - 1);
        let oz2 = (rect.z2 + 1).min(cz - 1);
        // At the city edge the "outside" coordinate stays on R's own edge.
        let gx1 = if rect.x1 == 0 { rect.x1 } else { ox1 };
        let gz1 = if rect.z1 == 0 { rect.z1 } else { oz1 };
        let gx2 = if rect.x2 == cx - 1 { rect.x2 } else { ox2 };
        let gz2 = if rect.z2 == cz - 1 { rect.z2 } else { oz2 };

        let in_region = |x: i32, z: i32| {
            self.st
                .borrow()
                .restriction
                .as_ref()
                .is_some_and(|r| r.includes(x, z))
        };

        for x in rect.x1..=rect.x2 {
            if in_region(x, rect.z1) {
                self.examine_cell(x, gz1, 0, &mut scores, &mut conds);
            }
            if in_region(x, rect.z2) {
                self.examine_cell(x, gz2, 2, &mut scores, &mut conds);
            }
        }
        for z in rect.z1..=rect.z2 {
            if in_region(rect.x1, z) {
                self.examine_cell(gx1, z, 3, &mut scores, &mut conds);
            }
            if in_region(rect.x2, z) {
                self.examine_cell(gx2, z, 1, &mut scores, &mut conds);
            }
        }

        // D := argmax S (first of equal maxima).
        let mut d = 0i32;
        for i in 1..4 {
            if scores[i] > scores[d as usize] {
                d = i as i32;
            }
        }
        // Q.
        let q = match d {
            0 | 2 => {
                if scores[1] <= scores[3] {
                    3
                } else {
                    1
                }
            }
            _ => {
                if scores[0] > scores[2] {
                    0
                } else {
                    2
                }
            }
        };
        let mut flag = true;
        let mut d = d;
        let mut q = q;
        let zone = self.st.borrow().current_zone;
        let w = rect.x2 - rect.x1 + 1;
        let h = rect.z2 - rect.z1 + 1;
        if place_streets && (zone == 1 || zone == 2) && w > 2 && h > 2 {
            flag = false;
            std::mem::swap(&mut d, &mut q);
        }
        let alternate = self.st.borrow().options.alternate_orientation;
        if alternate {
            flag = !flag;
            std::mem::swap(&mut d, &mut q);
        }
        self.st.borrow_mut().orientation = Orientation {
            flag,
            d,
            q,
            scores,
            conditions: conds,
        };
    }

    /// `ExamineCell(x, z, side)` (§7.3): updates the side's score and
    /// condition.
    fn examine_cell(&self, x: i32, z: i32, side: i32, scores: &mut [i32; 4], conds: &mut [i32; 4]) {
        let i = side as usize;
        // Network tests with ExistsNetworkOfType, first match wins.
        if self.exists_network_of_type(x, z, net::HIGHWAY_LIGHT_RAIL) {
            conds[i] = 2;
            scores[i] += 5;
        } else if self.exists_network_of_type(x, z, net::STREET_DIRT) {
            conds[i] = conds[i].max(1);
            scores[i] += 30;
        } else if self.exists_network_of_type(x, z, net::ROAD_ONE_WAY) {
            conds[i] = conds[i].max(1);
            scores[i] += 100;
        } else if self.exists_network_of_type(x, z, net::AVENUE) {
            conds[i] = conds[i].max(1);
            scores[i] += 400;
        } else if self.exists_network_of_type(x, z, net::RAIL) {
            conds[i] = 2;
            scores[i] += 5;
        } else if self.exists_network_of_type(x, z, net::ANY) {
            // other network of 0x1F4F: no score change.
        } else {
            // No network: a lot on the cell, or water.
            if self.lot_at(x, z).is_some() || self.host.cell_has_water(x, z) {
                conds[i] = 2;
                scores[i] += 5;
            } else {
                // Each building / obstacle occupant in the cell.
                for occ in self.host.cell_occupants(x, z, None) {
                    let t = self.host.occupant_type(occ);
                    if t == BUILDING_TYPE || t == OBSTACLE_TYPE_2 {
                        conds[i] = 2;
                        scores[i] += 5;
                    }
                }
            }
        }
        // Then, in every case: if C[side] = 0, S += 1.
        if conds[i] == 0 {
            scores[i] += 1;
        }
    }

    /// `CreateParcellizableRegion(region)` (§7.4): a new region over the input
    /// rect, all false, with the parcellizable cells set.
    fn create_parcellizable_region(&self, region: &super::CellRegion) -> super::CellRegion {
        let rect = region.rect;
        let mut out = super::CellRegion::new(rect, false);
        let zone = self.st.borrow().current_zone;
        let real = self.st.borrow().real_mode;
        for &(x, z) in &region.cells() {
            if zone != 0 && self.exists_network_of_type(x, z, net::ANY) {
                continue;
            }
            if self.is_cell_water(x, z) {
                continue;
            }
            let mut keep = true;
            if let Some(lot) = self.lot_at(x, z) {
                let occupied = if real { true } else { self.can_create_lot(x, z, 1, 1, false, false) == lot_code::OCCUPIED };
                if occupied {
                    match self.host.lot_configuration(lot) {
                        Some(cfg) => {
                            if self.host.cfg_compatible_with_zone(cfg, zone) {
                                keep = false; // the lot survives
                            } else {
                                self.delete_lot(lot);
                                keep = true;
                            }
                        }
                        None => {
                            let lz = self.host.lot_zone_type(lot);
                            if zone >= 1 && zone <= 6 && lz >= 1 && lz <= 6 {
                                keep = false;
                            } else {
                                self.delete_lot(lot);
                                keep = true;
                            }
                        }
                    }
                }
            }
            if keep {
                out.set(x, z, true);
            }
        }
        out
    }

    /// `SimpleParcellize(n)` (§7.5): agriculture (n = 16) and industry (n = 6).
    fn simple_parcellize(&self, n: i32) {
        let (rect, place_streets) = {
            let s = self.st.borrow();
            (s.restriction.as_ref().map(|r| r.rect), s.options.place_streets)
        };
        let Some(rect) = rect else { return };
        let q = self.st.borrow().orientation.q;
        let d = self.st.borrow().orientation.d;

        // 1. Street lines across the rect (if placeStreets).
        if place_streets {
            // x lines.
            let (x_start, x_dir) = match q {
                0 => {
                    if d == 3 {
                        (rect.x1, 1)
                    } else {
                        (rect.x2, -1)
                    }
                }
                1 | 2 => (rect.x2, -1),
                _ => (rect.x1, 1),
            };
            self.simple_street_pass(rect, true, x_start, x_dir, n);
            // z lines.
            let (z_start, z_dir) = match q {
                0 => (rect.z1, 1),
                1 => {
                    if d == 0 {
                        (rect.z1, 1)
                    } else {
                        (rect.z2, -1)
                    }
                }
                2 => (rect.z2, -1),
                _ => {
                    if d == 0 {
                        (rect.z1, 1)
                    } else {
                        (rect.z2, -1)
                    }
                }
            };
            self.simple_street_pass(rect, false, z_start, z_dir, n);
        }

        // 2. Every cell of the restriction rect, x ascending outer, z
        //    ascending inner: CreateLotToFaceRoad(cell, 1, 1, empty) → one draw.
        let empty: [i32; 0] = [];
        for x in rect.x1..=rect.x2 {
            for z in rect.z1..=rect.z2 {
                let lot = self.create_lot_to_face_road(x, z, 1, 1, &empty);
                if let Some(l) = lot {
                    self.host.release(l);
                }
            }
        }
        // 3. Free the restriction region (the caller clears it).
    }

    /// One pass of `SimpleParcellize`'s street lines (§7.5 step 1). `x_lines`
    /// selects walking x (lines at constant x, spanning z) vs z.
    fn simple_street_pass(&self, rect: CellRect, x_lines: bool, start: i32, dir: i32, n: i32) {
        let (lo, hi) = if x_lines { (rect.x1, rect.x2) } else { (rect.z1, rect.z2) };
        let mut counter = 0i32;
        let mut i = start;
        loop {
            let in_range = if dir > 0 { i <= hi } else { i >= lo };
            if !in_range {
                break;
            }
            let dist_lo = i - lo;
            let dist_hi = hi - i;
            if counter >= n && dist_lo >= 2 && dist_hi >= 2 {
                // Draw a street along the whole line, perCell = 0.
                if x_lines {
                    self.draw_street(i, rect.z1, i, rect.z2, false);
                } else {
                    self.draw_street(rect.x1, i, rect.x2, i, false);
                }
                counter = 0;
            } else {
                counter += 1;
            }
            i += dir;
        }
    }

    /// `Parcellize()` (§7.6): residential and commercial. No random draws.
    fn parcellize(&self) {
        let (rect, orient, ls, place_streets) = {
            let s = self.st.borrow();
            (
                s.restriction.as_ref().map(|r| r.rect),
                s.orientation,
                s.lot_size,
                s.options.place_streets,
            )
        };
        let Some(rect) = rect else { return };
        let (d, q, flag) = (orient.d, orient.q, orient.flag);
        let c = orient.conditions;
        let lw = ls.width.max(1);
        let ld = ls.depth.max(1);
        let (min_depth, max_depth, block) = (ls.min_depth, ls.max_depth, ls.block);
        let ps = i32::from(place_streets);

        // Frame (§7.6.1). Q ∈ {0,2} → depth axis x; Q ∈ {1,3} → depth axis z.
        let depth_is_x = q == 0 || q == 2;
        let (a1, a2, b1, b2) = if depth_is_x {
            (rect.x1, rect.x2, rect.z1, rect.z2)
        } else {
            (rect.z1, rect.z2, rect.x1, rect.x2)
        };
        // Side 0 = −z, 1 = +x, 2 = +z, 3 = −x. On the depth axis the low side
        // is −x (3) when the axis is x, or −z (0) when it is z; the high side
        // is the other (1 or 2).
        let d_low = if depth_is_x { d == 3 } else { d == 0 };

        let p0 = if d_low { a1 } else { (a2 - ld + 1).max(0) };
        let sp = if d_low { ld } else { -ld };
        let j = if d_low { ld } else { -1 };
        let rem = |p: i32| if d_low { a2 - p + 1 } else { p - a1 + ld };
        let depth_keep = |p: i32| {
            if d_low {
                p >= a1 && p <= a2
            } else {
                p >= a1 - ld + 1 && p <= p0
            }
        };

        let o0 = if q == 0 || q == 3 { b1 } else { b2 - lw + 1 };
        let so = if q == 0 || q == 3 { lw } else { -lw };
        let k = if q == 0 || q == 3 { lw } else { -1 };
        let row_keep = |o: i32| {
            if q == 0 || q == 3 {
                o >= b1 && o <= b2
            } else {
                o >= b1 - lw + 1 && o <= o0
            }
        };

        let a = d; // front facing
        let b = (d + 2).rem_euclid(4); // back facing
        let first = if c[d as usize] == 1 { a } else { b };
        let far_road = c[b as usize] == 1;
        let lrow = b2 - b1 + 1;
        let ldepth = a2 - a1 + 1;

        // Middle row.
        let mut n_rows = 0i32;
        {
            let mut o = o0;
            while row_keep(o) && n_rows < MAX_WALK_STEPS {
                n_rows += 1;
                o += so;
            }
        }
        let mid = if n_rows / 2 >= 1 { o0 + (n_rows / 2 - 1) * so } else { o0 };

        // The lot rect at (p, o) with depth size n (§7.6.1).
        let lot_rect = |p: i32, o: i32, n: i32| -> CellRect {
            if depth_is_x {
                CellRect::new(p, o, p + n - 1, o + lw - 1)
            } else {
                CellRect::new(o, p, o + lw - 1, p + n - 1)
            }
        };

        let mut acc = 0i32;
        let mut back_street = false;
        let mut locked: Vec<u64> = Vec::new();
        let mut o = o0;
        let mut guard = 0i32;
        while row_keep(o) && guard < MAX_WALK_STEPS {
            guard += 1;
            let mut f = first;
            let mut p = p0;
            let mut dguard = 0i32;
            while depth_keep(p) && dguard < MAX_WALK_STEPS {
                dguard += 1;
                let mut n = ld;
                let mut delta = 0i32;
                let r = rem(p);
                if r > 0 {
                    let front = f == a;
                    let mut np = depth_rule(r, front, far_road, min_depth, max_depth, ld, ps);
                    if (ldepth as u32) <= (max_depth as u32) && (lrow as u32) < ((ps + 2 * lw) as u32) {
                        np = ldepth;
                    }
                    if np != ld {
                        delta = np - ld;
                        n = np;
                        if sp < 0 {
                            p -= delta;
                        }
                    }
                }
                let lr = lot_rect(p, o, n);
                let created = self.create_lot_if_possible(lr.x1, lr.z1, lr.x2 - lr.x1 + 1, lr.z2 - lr.z1 + 1, f);
                match created {
                    Some(_lot) => {}
                    None => self.create_lot_remnants(lr, f),
                }
                if delta != 0 && sp > 0 {
                    p += delta;
                }
                if f == a {
                    f = b;
                } else {
                    f = a;
                    if place_streets {
                        let s = p + j;
                        if s >= a1 && s <= a2 {
                            if o == o0 {
                                // Draw the back street across b1..b2.
                                if depth_is_x {
                                    self.draw_street(s, b1, s, b2, s == a1 || s == a2);
                                } else {
                                    self.draw_street(b1, s, b2, s, s == a1 || s == a2);
                                }
                                back_street = true;
                            }
                            p += if sp > 0 { 1 } else { -1 };
                        }
                    }
                }
                if self.st.borrow().real_mode {
                    let (cx, cz) = self.host.city_cell_counts();
                    let clamped = CellRect::new(lr.x1.max(0), lr.z1.max(0), lr.x2.min(cx - 1), lr.z2.min(cz - 1));
                    self.host.attempt_area_levelling(clamped);
                }
                if let Some(lot) = created {
                    self.host.lot_set_vertices_locked(lot, true);
                    locked.push(lot);
                }
                p += sp;
            }
            if place_streets {
                acc += lw;
                let force = flag && back_street && (lrow as u32) <= ((block + lw + 1) as u32) && o == mid;
                if (acc as u32) >= (block as u32) || force {
                    let t = o + k;
                    if force || ((b1 + lw) as u32) <= (t as u32) && (t as u32) <= ((b2 - lw) as u32) {
                        acc = 0;
                        if depth_is_x {
                            self.draw_street(a1, t, a2, t, t == b1 || t == b2);
                        } else {
                            self.draw_street(t, a1, t, a2, t == b1 || t == b2);
                        }
                        o += if so > 0 { 1 } else { -1 };
                    }
                }
            }
            o += so;
        }
        for lot in locked {
            self.host.lot_set_vertices_locked(lot, false);
            self.host.release(lot);
        }
    }

    /// `DrawStreet(x1, z1, x2, z2, perCell)` (§7.10). Draws an automatic
    /// street along one cell line, only on cells of the restriction region R.
    /// The callers pass a horizontal or vertical line. "Place(a, b)" is the
    /// street tool's `PlaceNetworkOccupants` ([`ZonesHost::street_place`]);
    /// it always lays a street (kind 3), whatever the caller's network kind.
    fn draw_street(&self, x1: i32, z1: i32, x2: i32, z2: i32, per_cell: bool) {
        let in_r = |x: i32, z: i32| {
            self.st
                .borrow()
                .restriction
                .as_ref()
                .is_some_and(|r| r.includes(x, z))
        };
        // Unit step per axis (sign of the difference).
        let ux = (x2 - x1).signum();
        let uz = (z2 - z1).signum();
        let step = |(x, z): (i32, i32)| (x + ux, z + uz);
        let back = |(x, z): (i32, i32)| (x - ux, z - uz);
        let place = |a: (i32, i32), b: (i32, i32)| self.host.street_place(network_kind::STREET, a, b);

        // 1. Single cell (start = end).
        if x1 == x2 && z1 == z2 {
            if in_r(x1, z1) {
                place((x1, z1), (x1, z1));
            }
            return;
        }
        let end = (x2, z2);
        let mut c = (x1, z1);
        let mut guard = 0i32;
        while guard < MAX_WALK_STEPS {
            guard += 1;
            // Skip cells not in R (advance c by u while c ∉ R and c ≠ end).
            while !in_r(c.0, c.1) && c != end {
                c = step(c);
            }
            // c = end: the end-cell fallback chain.
            if c == end {
                if !in_r(end.0, end.1) {
                    break;
                }
                let eu = step(end);
                let eb = back(end);
                if place(end, end) || place(eu, end) || place(end, eb) || place(eu, eb) {
                    break;
                }
                break;
            }
            // The run: the maximal set of consecutive in-R cells from c
            // towards the end, [c, r].
            let mut r_end = c;
            let mut nxt = step(c);
            while in_r(nxt.0, nxt.1) && nxt != end {
                r_end = nxt;
                nxt = step(nxt);
            }
            if nxt == end && in_r(end.0, end.1) {
                r_end = end;
            }
            // Unless perCell and the run is a single cell: the segment chain
            // (fallbacks use the line's end, not the run's end *[quirk]*).
            if !(per_cell && r_end == c) {
                let eb = back(end);
                let _ = place(c, r_end) || place(step(c), end) || place(c, eb) || place(step(c), eb);
            }
            // If perCell: Place(cell, cell) for every cell of the run.
            if per_cell {
                let mut cell = c;
                loop {
                    place(cell, cell);
                    if cell == r_end {
                        break;
                    }
                    cell = step(cell);
                }
            }
            if r_end == end {
                break;
            }
            c = step(r_end);
        }
    }

    /// `ExtendAsStreets(rect, k)` (§7.11).
    fn extend_as_streets(&self, rect: CellRect, k: u32) {
        let flags = 1u32 << k;
        for x in rect.x1..=rect.x2 {
            for z in rect.z1..=rect.z2 {
                if !self.exists_network_of_type(x, z, flags) {
                    continue;
                }
                // Count connected sides (stop at 2).
                let mut count = 0i32;
                let mut dead_side = -1i32;
                for s in 0..4i32 {
                    let connects = [
                        network_kind::ROAD,
                        network_kind::ONE_WAY_ROAD,
                        network_kind::STREET,
                        network_kind::DIRT_ROAD,
                    ]
                    .iter()
                    .any(|&t| self.exists_connection_of_type(x, z, flags, s, t))
                    || ((x + z) % 2 != 0 && self.exists_connection_of_type(x, z, flags, s, network_kind::AVENUE));
                    if connects {
                        count += 1;
                        dead_side = s;
                        if count >= 2 {
                            break;
                        }
                    }
                }
                if count == 1 {
                    // Dead end: draw to the rect edge in that direction.
                    match dead_side {
                        0 => {
                            if z != rect.z2 {
                                self.draw_street(x, z, x, rect.z2, false);
                            }
                        }
                        1 => {
                            if x != rect.x1 {
                                self.draw_street(x, z, rect.x1, z, false);
                            }
                        }
                        2 => {
                            if z != rect.z1 {
                                self.draw_street(x, z, x, rect.z1, false);
                            }
                        }
                        _ => {
                            if x != rect.x2 {
                                self.draw_street(x, z, rect.x2, z, false);
                            }
                        }
                    }
                }
            }
        }
    }

    /// `LayInJogStreets(rect, k)` (§7.12).
    fn lay_in_jog_streets(&self, rect: CellRect, k: u32) {
        let flags = 1u32 << k;
        let (cx, cz) = self.host.city_cell_counts();
        // The four outside lines.
        struct Line {
            wanted: i32,
            horiz: bool,
            fixed: i32,
            lo: i32,
            hi: i32,
            street: CellRect,
        }
        let lines: Vec<Option<Line>> = vec![
            if rect.z1 > 0 {
                Some(Line {
                    wanted: 0,
                    horiz: true,
                    fixed: rect.z1 - 1,
                    lo: rect.x1,
                    hi: rect.x2,
                    street: CellRect::new(rect.x1, rect.z1, rect.x2, rect.z1),
                })
            } else {
                None
            },
            if rect.z2 < cz - 1 {
                Some(Line {
                    wanted: 2,
                    horiz: true,
                    fixed: rect.z2 + 1,
                    lo: rect.x1,
                    hi: rect.x2,
                    street: CellRect::new(rect.x1, rect.z2, rect.x2, rect.z2),
                })
            } else {
                None
            },
            if rect.x1 > 0 {
                Some(Line {
                    wanted: 3,
                    horiz: false,
                    fixed: rect.x1 - 1,
                    lo: rect.z1,
                    hi: rect.z2,
                    street: CellRect::new(rect.x1, rect.z1, rect.x1, rect.z2),
                })
            } else {
                None
            },
            if rect.x2 < cx - 1 {
                Some(Line {
                    wanted: 1,
                    horiz: false,
                    fixed: rect.x2 + 1,
                    lo: rect.z1,
                    hi: rect.z2,
                    street: CellRect::new(rect.x2, rect.z1, rect.x2, rect.z2),
                })
            } else {
                None
            },
        ];
        for line in lines.into_iter().flatten() {
            let mut dead = false;
            let mut parallel = false;
            for t in line.lo..=line.hi {
                let (x, z) = if line.horiz { (t, line.fixed) } else { (line.fixed, t) };
                if !self.exists_network_of_type(x, z, flags) {
                    continue;
                }
                // Count sides with ExistsConnectionOfType(x, z, flags, s, k).
                let mut count = 0i32;
                let mut one_side = -1i32;
                for s in 0..4i32 {
                    if self.exists_connection_of_type(x, z, flags, s, k) {
                        count += 1;
                        one_side = s;
                        if count >= 2 {
                            break;
                        }
                    }
                }
                if count == 1 && one_side == line.wanted {
                    dead = true;
                }
                // Parallel test (real network only).
                for &(pflag, ptype) in &[
                    (net::AVENUE, network_kind::AVENUE),
                    (0x1u32, network_kind::ROAD),
                    (0x400u32, network_kind::ONE_WAY_ROAD),
                    (net::STREET, network_kind::STREET),
                    (net::DIRT, network_kind::DIRT_ROAD),
                ] {
                    if let Some(occ) = self.get_network(x, z, pflag) {
                        let e1 = if line.horiz { 0 } else { 1 };
                        let e2 = if line.horiz { 2 } else { 3 };
                        if self.host.connects_to_neighbor(occ, e1, ptype)
                            || self.host.connects_to_neighbor(occ, e2, ptype)
                        {
                            parallel = true;
                            break;
                        }
                    }
                }
                if parallel {
                    break;
                }
            }
            if dead && !parallel {
                let r = line.street;
                self.draw_street(r.x1, r.z1, r.x2, r.z2, true);
            }
        }
    }

    /// `GetNetwork(x, z, flags)` (§7.13): the first network occupant in the
    /// cell with any of `flags`; `None` if none.
    fn get_network(&self, x: i32, z: i32, flags: u32) -> Option<u64> {
        for occ in self.host.cell_occupants(x, z, Some(NETWORK_OCCUPANT_TYPE)) {
            if self.host.network_has_any_flag(occ, flags) == Some(true) {
                return Some(occ);
            }
        }
        None
    }

    /// `ExistsConnectionOfType(x, z, flags, side, type)` (§7.13).
    fn exists_connection_of_type(&self, x: i32, z: i32, flags: u32, side: i32, net_type: u32) -> bool {
        let e = if (0..4).contains(&side) { (side + 1).rem_euclid(4) } else { -1 };
        if let Some(occ) = self.get_network(x, z, flags) {
            if self.host.connects_to_neighbor(occ, e, net_type) {
                return true;
            }
        }
        // Highlight mode only, and only when flags & 0x808.
        if !self.st.borrow().real_mode && (flags & 0x808) != 0 {
            if e < 0 {
                return false;
            }
            for &(tool_kind, flag) in &[(network_kind::STREET, net::STREET), (network_kind::DIRT_ROAD, net::DIRT)] {
                if let Some(mask) = self.host.network_tool_occupant_exists(tool_kind, x, z, flag) {
                    return mask[e as usize] != 0;
                }
            }
        }
        false
    }

    /// Slot 8 `ExistsNetworkOfType(x, z, flags)` (§8.8).
    pub fn exists_network_of_type(&self, x: i32, z: i32, flags: u32) -> bool {
        if self.get_network(x, z, flags).is_some() {
            return true;
        }
        // Highlight mode: also accept the street / dirt-road tool's draft.
        if !self.st.borrow().real_mode {
            if flags & net::STREET != 0 && self.host.network_tool_draft_at(network_kind::STREET, x, z) {
                return true;
            }
            if flags & net::DIRT != 0 && self.host.network_tool_draft_at(network_kind::DIRT_ROAD, x, z) {
                return true;
            }
        }
        false
    }

    /// Slot 9 `IsCellWater(x, z)` (§8.8): tail jump to terrain slot 34.
    pub fn is_cell_water(&self, x: i32, z: i32) -> bool {
        self.host.cell_has_water(x, z)
    }

    /// `CreateLotToFaceRoad(x, z, w, h, list)` (§7.7). An empty list draws
    /// `U(4)` on entry (§9 Z2). Returns the created lot's caller reference, or
    /// `None`.
    fn create_lot_to_face_road(&self, x: i32, z: i32, w: i32, h: i32, list: &[i32]) -> Option<u64> {
        // 1. Empty list → one U(4) draw, list := r, r+1, r+2, r+3 (mod 4).
        let mut orientations: Vec<i32> = list.to_vec();
        if orientations.is_empty() {
            let r = self
                .statics
                .borrow_mut()
                .uniform(DrawKind::Z2, 4) as i32;
            orientations = vec![r, (r + 1) % 4, (r + 2) % 4, (r + 3) % 4];
        }
        let (cx, cz) = self.host.city_cell_counts();
        // 2. For each orientation, test the cells just outside side o for a
        //    network of 0xC49; the side must lie inside the city.
        for &o in &orientations {
            let (ok, has_road) = match o {
                0 => (z > 0, (x..x + w).any(|xx| self.exists_network_of_type(xx, z - 1, net::ROADS))),
                1 => (x + w - 1 < cx - 1, (z..z + h).any(|zz| self.exists_network_of_type(x + w, zz, net::ROADS))),
                2 => (z + h - 1 < cz - 1, (x..x + w).any(|xx| self.exists_network_of_type(xx, z + h, net::ROADS))),
                3 => (x > 0, (z..z + h).any(|zz| self.exists_network_of_type(x - 1, zz, net::ROADS))),
                _ => (false, false),
            };
            if ok && has_road {
                return self.create_lot_if_possible(x, z, w, h, o);
            }
        }
        // 3. No side has a road: use the list front.
        let front = orientations.first().copied().unwrap_or(0);
        self.create_lot_if_possible(x, z, w, h, front)
    }

    /// `CreateLotRemnants(rect, facing)` (§7.8): re-creates lots on the cells
    /// of a deleted lot. By the current zone type.
    fn create_lot_remnants(&self, rect: CellRect, facing: i32) {
        let zone = self.st.borrow().current_zone;
        if zone == 7 || zone == 8 || zone == 9 {
            // Every cell, z outer, x inner: one draw per cell.
            let empty: [i32; 0] = [];
            for z in rect.z1..=rect.z2 {
                for x in rect.x1..=rect.x2 {
                    let lot = self.create_lot_to_face_road(x, z, 1, 1, &empty);
                    if let Some(l) = lot {
                        self.host.release(l);
                    }
                }
            }
            return;
        }
        // Otherwise: strips one cell deep, perpendicular to the facing side,
        // with the list {facing} (no draw). Scan order by facing.
        let list = [facing.rem_euclid(4)];
        let f = facing.rem_euclid(4);
        let buildable = |x: i32, z: i32| -> bool {
            self.can_create_lot(x, z, 1, 1, false, false) == lot_code::FREE
        };
        // Collect runs of consecutive buildable cells and create a lot per run.
        match f {
            3 => {
                // rows of z, x ascending
                for z in rect.z1..=rect.z2 {
                    self.remnant_run(rect.x1, rect.x2, z, true, &list, buildable);
                }
            }
            1 => {
                // rows of z, x descending
                for z in rect.z1..=rect.z2 {
                    self.remnant_run(rect.x2, rect.x1, z, true, &list, buildable);
                }
            }
            0 => {
                // columns of x, z ascending
                for x in rect.x1..=rect.x2 {
                    self.remnant_run(rect.z1, rect.z2, x, false, &list, buildable);
                }
            }
            _ => {
                // columns of x, z descending (facing 2)
                for x in rect.x1..=rect.x2 {
                    self.remnant_run(rect.z2, rect.z1, x, false, &list, buildable);
                }
            }
        }
    }

    /// One remnant run along an axis (a line of cells running away from the
    /// facing side), creating one 1-cell-deep lot per consecutive buildable
    /// run *[inferred]*. `horiz` = the run varies x (fixed z = `line`); else
    /// it varies z (fixed x = `line`).
    fn remnant_run(&self, from: i32, to: i32, line: i32, horiz: bool, list: &[i32], buildable: impl Fn(i32, i32) -> bool) {
        let step = if to >= from { 1 } else { -1 };
        let mut i = from;
        let mut run: Vec<(i32, i32)> = Vec::new();
        let mut done = false;
        while !done {
            let (x, z) = if horiz { (i, line) } else { (line, i) };
            if buildable(x, z) {
                run.push((x, z));
            } else if !run.is_empty() {
                self.flush_remnant(&run, horiz, list);
                run.clear();
            }
            if i == to {
                done = true;
            }
            i += step;
        }
        if !run.is_empty() {
            self.flush_remnant(&run, horiz, list);
        }
    }

    fn flush_remnant(&self, run: &[(i32, i32)], horiz: bool, list: &[i32]) {
        if run.is_empty() {
            return;
        }
        let (x0, z0) = run[0];
        let (x1, z1) = run[run.len() - 1];
        let (x, z, w, h) = if horiz {
            (x0.min(x1), z0, (x1 - x0).abs() + 1, 1)
        } else {
            (x0, z0.min(z1), 1, (z1 - z0).abs() + 1)
        };
        // A strip one cell deep: the run is one cell wide perpendicular to the
        // facing, so (w, h) is (len, 1) or (1, len).
        let facing = list[0];
        let lot = self.create_lot_to_face_road(x, z, w, h, list);
        let _ = (facing, lot);
        if let Some(l) = lot {
            self.host.release(l);
        }
    }

    /// `OnDezone(region, flag)` (§7.9).
    fn on_dezone(&self, region: &super::CellRegion, flag: bool) {
        self.host.post_defer_occupant_operations(true); // 1.
        let rect = region.rect;
        // 2. For each set cell, z outer, x inner, with a lot.
        for z in rect.z1..=rect.z2 {
            for x in rect.x1..=rect.x2 {
                if !region.includes(x, z) {
                    continue;
                }
                if let Some(lot) = self.lot_at(x, z) {
                    let zone = self.host.lot_zone_type(lot);
                    if is_rci_zone(zone) || zone == 11 || zone == 12 {
                        let lrect = self.host.lot_bounding_rect(lot);
                        let facing = self.host.lot_facing(lot);
                        self.st.borrow_mut().current_zone = zone;
                        self.delete_lot(lot);
                        self.create_lot_remnants(lrect, facing);
                        self.st.borrow_mut().current_zone = 0;
                    }
                }
            }
        }
        // 3. If flag: collect the set cells with a 0x808 network and no 0x1747
        //    network into a region; if any, DemolishRegion.
        if flag {
            let mut any = false;
            let mut sub = super::CellRegion::new(rect, false);
            for z in rect.z1..=rect.z2 {
                for x in rect.x1..=rect.x2 {
                    if !region.includes(x, z) {
                        continue;
                    }
                    if self.exists_network_of_type(x, z, net::STREET_DIRT)
                        && !self.exists_network_of_type(x, z, net::KEEP_STREET)
                    {
                        sub.set(x, z, true);
                        any = true;
                    }
                }
            }
            if any {
                self.host.demolish_region(&sub);
            }
        }
        self.host.post_defer_occupant_operations(false); // 4.
    }

    // ---- lot manager: lookups (§8.2) ----

    /// Slot 5 `GetLot(const cS3DVector3&)` (§8.2).
    pub fn get_lot_at_position(&self, pos: super::Position) -> Option<u64> {
        if !self.host.location_in_bounds(pos) {
            return None;
        }
        let (x, z) = self.host.position_to_cell(pos)?;
        self.get_lot(x, z, false)
    }

    /// Slot 6 `GetLot(x, z, originOnly)` (§8.2). No AddRef.
    pub fn get_lot(&self, x: i32, z: i32, origin_only: bool) -> Option<u64> {
        let lot = self.lot_at(x, z)?;
        if origin_only && self.host.lot_location(lot) != (x, z) {
            return None;
        }
        Some(lot)
    }

    /// Slot 13 `GetRandomLot()` (§8.2, §9 Z1): up to 999 tries of
    /// `U(cellsX)` then `U(cellsZ)`; first non-null cell lot; no AddRef.
    pub fn get_random_lot(&self) -> Option<u64> {
        let (cx, cz) = self.host.city_cell_counts();
        let (cx, cz) = (cx.max(0) as u32, cz.max(0) as u32);
        for _ in 0..MAX_RANDOM_LOT_TRIES {
            let x = self.statics.borrow_mut().uniform(DrawKind::Z1, cx) as i32;
            let z = self.statics.borrow_mut().uniform(DrawKind::Z1, cz) as i32;
            if let Some(lot) = self.lot_at(x, z) {
                return Some(lot);
            }
        }
        None
    }

    /// Slot 14 `GetOccupantLot(occupant)` (§8.2).
    pub fn get_occupant_lot(&self, occupant: u64) -> Option<u64> {
        let p = self.host.occupant_position(occupant);
        let cx = occupant_cell(p[0]);
        let cz = occupant_cell(p[2]);
        self.get_lot(cx, cz, false)
    }

    // ---- lot manager: lists (§8.3) ----

    /// Slot 7 `GetLotsInCellRect(rect, originOnly)` (§8.3). Returns the lots
    /// appended (one reference each for the caller), in order.
    pub fn get_lots_in_cell_rect(&self, rect: CellRect, origin_only: bool) -> Vec<u64> {
        let (cx, cz) = self.host.city_cell_counts();
        let x1 = rect.x1.max(0);
        let z1 = rect.z1.max(0);
        let x2 = rect.x2.min(cx - 1);
        let z2 = rect.z2.min(cz - 1);
        let mut out: Vec<u64> = Vec::new();
        if x1 > x2 || z1 > z2 {
            return out;
        }
        if origin_only {
            // z outer, x inner: lots whose origin is the cell.
            for z in z1..=z2 {
                for x in x1..=x2 {
                    if let Some(lot) = self.lot_at(x, z) {
                        if self.host.lot_location(lot) == (x, z) {
                            self.host.add_ref(lot);
                            out.push(lot);
                        }
                    }
                }
            }
        } else {
            // x outer, z inner: append when the lot differs from (x, z−1) and
            // (x−1, z), both looked up only inside the clamped rectangle
            // [inferred].
            for x in x1..=x2 {
                for z in z1..=z2 {
                    let Some(lot) = self.lot_at(x, z) else { continue };
                    let prev_z = if z - 1 >= z1 { self.lot_at(x, z - 1) } else { None };
                    let prev_x = if x - 1 >= x1 { self.lot_at(x - 1, z) } else { None };
                    if Some(lot) != prev_z && Some(lot) != prev_x {
                        self.host.add_ref(lot);
                        out.push(lot);
                    }
                }
            }
        }
        out
    }

    /// Slot 10 `GetLotsSurroundingRect(rect, flags)` (§8.3): the lots it
    /// appends (one reference each), walking the ring row by row (z outer, x
    /// inner) *[inferred]*.
    pub fn get_lots_surrounding_rect(&self, rect: CellRect, flags: u32) -> Vec<u64> {
        let (cx, cz) = self.host.city_cell_counts();
        let mut out: Vec<u64> = Vec::new();
        let mut seen: Vec<u64> = Vec::new();
        // Side bits: 1 = row z1−1, 2 = column x2+1, 4 = row z2+1, 8 = column x1−1.
        let bit1 = rect.z1 - 1 >= 0;
        let bit2 = rect.x2 + 1 < cx;
        let bit4 = rect.z2 + 1 < cz;
        let bit8 = rect.x1 - 1 >= 0;
        let all = flags == 0xF;
        let try_cell = |x: i32, z: i32, out: &mut Vec<u64>, seen: &mut Vec<u64>| {
            if let Some(lot) = self.lot_at(x, z) {
                if !rect_overlaps(self.host.lot_bounding_rect(lot), rect) && !seen.contains(&lot) {
                    seen.push(lot);
                    self.host.add_ref(lot);
                    out.push(lot);
                }
            }
        };
        // Row z1−1 (bit 1): x1−1 .. x2+1.
        if bit1 && (all || flags & 1 != 0) {
            for x in (rect.x1 - 1).max(0)..=(rect.x2 + 1).min(cx - 1) {
                try_cell(x, rect.z1 - 1, &mut out, &mut seen);
            }
        }
        // Side columns for rows z1..z2 (bits 2 = x2+1, 8 = x1−1).
        for z in rect.z1..=rect.z2 {
            if bit8 && (all || flags & 8 != 0) {
                try_cell(rect.x1 - 1, z, &mut out, &mut seen);
            }
            if bit2 && (all || flags & 2 != 0) {
                try_cell(rect.x2 + 1, z, &mut out, &mut seen);
            }
        }
        // Row z2+1 (bit 4).
        if bit4 && (all || flags & 4 != 0) {
            for x in (rect.x1 - 1).max(0)..=(rect.x2 + 1).min(cx - 1) {
                try_cell(x, rect.z2 + 1, &mut out, &mut seen);
            }
        }
        out
    }

    /// Slot 8 `GetLotsSurroundingLot(x, z, flags)` (§8.3).
    pub fn get_lots_surrounding_lot_at(&self, x: i32, z: i32, flags: u32) -> Vec<u64> {
        match self.get_lot(x, z, false) {
            Some(lot) => self.get_lots_surrounding_lot(lot, flags),
            None => Vec::new(),
        }
    }

    /// Slot 9 `GetLotsSurroundingLot(lot, flags)` (§8.3).
    pub fn get_lots_surrounding_lot(&self, lot: u64, flags: u32) -> Vec<u64> {
        let rect = self.host.lot_bounding_rect(lot);
        self.get_lots_surrounding_rect(rect, flags)
    }

    /// Slot 15 `GetSafeLotIterator()` (§8.3): a handle registered with the
    /// manager, advanced by lot removal.
    pub fn get_safe_lot_iterator(&self) -> u64 {
        let mut s = self.st.borrow_mut();
        let handle = s.next_iter;
        s.next_iter += 1;
        s.safe_iters.push(SafeIter { handle, index: 0 });
        handle
    }

    /// Unregisters a safe lot iterator (§8.3 slot 15; freeing not traced,
    /// §14.3).
    pub fn free_safe_lot_iterator(&self, handle: u64) {
        self.st.borrow_mut().safe_iters.retain(|it| it.handle != handle);
    }

    /// Slot 16 `GetLotCount()` (§8.1).
    pub fn lot_count(&self) -> usize {
        self.st.borrow().master.len()
    }

    // ---- lot manager: geometry and altitude (§8.7) ----

    /// Slot 11 `GetVerticesAtEdgeOfLot(rect, flags)` (§8.7): the lowest set
    /// flag bit only; `None` when no bit is set (out untouched).
    pub fn vertices_at_edge_of_lot(&self, rect: CellRect, flags: u32) -> Option<CellRect> {
        if flags & 1 != 0 {
            Some(CellRect::new(rect.x1, rect.z1, rect.x2 + 1, rect.z1))
        } else if flags & 2 != 0 {
            Some(CellRect::new(rect.x2 + 1, rect.z1, rect.x2 + 1, rect.z2 + 1))
        } else if flags & 4 != 0 {
            Some(CellRect::new(rect.x1, rect.z2 + 1, rect.x2 + 1, rect.z2 + 1))
        } else if flags & 8 != 0 {
            Some(CellRect::new(rect.x1, rect.z1, rect.x1, rect.z2 + 1))
        } else {
            None
        }
    }

    /// Slot 12 `GetNeighborCellsAtEdgeOfLot(rect, flags)` (§8.7) → (out, AL).
    pub fn neighbor_cells_at_edge_of_lot(&self, rect: CellRect, flags: u32) -> (CellRect, bool) {
        let (cx, cz) = self.host.city_cell_counts();
        let empty = CellRect::new(0, 0, -1, -1);
        let out = if flags & 1 != 0 {
            if rect.z1 > 0 {
                CellRect::new(rect.x1, rect.z1 - 1, rect.x2, rect.z1 - 1)
            } else {
                empty
            }
        } else if flags & 2 != 0 {
            if rect.x2 < cx - 1 {
                CellRect::new(rect.x2 + 1, rect.z1, rect.x2 + 1, rect.z2)
            } else {
                empty
            }
        } else if flags & 4 != 0 {
            if rect.z2 < cz - 1 {
                CellRect::new(rect.x1, rect.z2 + 1, rect.x2, rect.z2 + 1)
            } else {
                empty
            }
        } else if flags & 8 != 0 {
            if rect.x1 > 0 {
                CellRect::new(rect.x1 - 1, rect.z1, rect.x1 - 1, rect.z2)
            } else {
                empty
            }
        } else {
            empty
        };
        (out, out.x2 >= out.x1)
    }

    /// Slot 17 `GetLotSavvyTerrainVertexAltitude(vx, vz, exclude)` (§8.7) →
    /// (altitude, ok).
    pub fn lot_savvy_terrain_vertex_altitude(&self, vx: i32, vz: i32, exclude: Option<u64>) -> (f32, bool) {
        let (cx, cz) = self.host.city_cell_counts();
        for &(x, z) in &[(vx - 1, vz - 1), (vx, vz - 1), (vx - 1, vz), (vx, vz)] {
            if x < 0 || x >= cx || z < 0 || z >= cz {
                continue;
            }
            if let Some(lot) = self.lot_at(x, z) {
                if Some(lot) != exclude && self.host.lot_flags(lot) & 1 != 0 {
                    return (self.host.lot_altitude(lot), true);
                }
            }
        }
        (self.host.terrain_vertex_altitude(vx, vz), false)
    }

    /// Slot 18 `GetLotSavvyTerrainAltitude(x, z)` (§8.7) → (altitude, ok).
    pub fn lot_savvy_terrain_altitude(&self, x: f32, z: f32) -> (f32, bool) {
        let (cx, cz) = self.host.city_cell_counts();
        let clamp = |c: i32, n: i32| c.clamp(0, (n - 1).max(0));
        let cellx = clamp(savvy_cell(x), cx);
        let cellz = clamp(savvy_cell(z), cz);
        if let Some(lot) = self.lot_at(cellx, cellz) {
            if self.host.lot_flags(lot) & 1 != 0 {
                return (self.host.lot_altitude(lot), true);
            }
        }
        (self.host.terrain_altitude(x, z), false)
    }

    /// Slot 19 `GetLotSavvyCellAltitude(cx, cz)` (§8.7): no bounds check.
    pub fn lot_savvy_cell_altitude(&self, cx: i32, cz: i32) -> f32 {
        if let Some(lot) = self.lot_at(cx, cz) {
            if self.host.lot_flags(lot) & 1 != 0 {
                return self.host.lot_altitude(lot);
            }
        }
        let wx = r24(r24(f64::from(cx) + 0.5) * 16.0) as f32;
        let wz = r24(r24(f64::from(cz) + 0.5) * 16.0) as f32;
        self.host.terrain_altitude(wx, wz)
    }

    /// Slot 26 `GetLotVertexAltitudeAdjustmentRange(vx, vz)` (§8.7) → (a, b).
    pub fn lot_vertex_altitude_adjustment_range(&self, vx: i32, vz: i32) -> (f32, f32) {
        let (cx, cz) = self.host.city_cell_counts();
        let mut lots: Vec<u64> = Vec::new();
        for x in (vx - 1).max(0)..=vx.min(cx - 1) {
            for z in (vz - 1).max(0)..=vz.min(cz - 1) {
                if let Some(lot) = self.lot_at(x, z) {
                    if !lots.contains(&lot) {
                        lots.push(lot);
                    }
                }
            }
        }
        let any_advanced = lots.iter().any(|&l| self.host.lot_state(l) > 1);
        if any_advanced {
            (32.0, 32.0)
        } else {
            (f32::MAX, f32::MAX)
        }
    }

    /// Slot 27 `GetMaximumCapacityGrid()` (§8.7). No AddRef.
    pub fn maximum_capacity_grid(&self) -> Option<Rc<dyn TractGrid<u16>>> {
        self.st.borrow().max_capacity.clone()
    }

    // ---- lot manager: CanCreateLot and creation (§8.4, §8.5) ----

    /// Slot 20 `CanCreateLot(x, z, w, h, zoneNotNeeded, waterAllowed, cfg)`
    /// (§8.4): the minimum cell code over the rect (x outer, z inner), from 8.
    pub fn can_create_lot(&self, x: i32, z: i32, w: i32, h: i32, zone_not_needed: bool, water_allowed: bool) -> i32 {
        self.can_create_lot_cfg(x, z, w, h, zone_not_needed, water_allowed, None)
    }

    fn can_create_lot_cfg(
        &self,
        x: i32,
        z: i32,
        w: i32,
        h: i32,
        zone_not_needed: bool,
        water_allowed: bool,
        cfg: Option<u64>,
    ) -> i32 {
        let water_ok = water_allowed || cfg.is_some_and(|c| self.host.cfg_compatible_with_water(c));
        let (cx, cz) = self.host.city_cell_counts();
        let mut min_code = lot_code::FREE;
        for xx in x..x + w {
            for zz in z..z + h {
                let code = self.cell_code(xx, zz, cx, cz, zone_not_needed, water_ok);
                min_code = min_code.min(code);
            }
        }
        min_code
    }

    fn cell_code(&self, x: i32, z: i32, cx: i32, cz: i32, zone_not_needed: bool, water_ok: bool) -> i32 {
        // 0: outside the city, or a restriction region is set and excludes it.
        if x < 0 || x >= cx || z < 0 || z >= cz {
            return lot_code::OUTSIDE;
        }
        let restriction_excludes = {
            let s = self.st.borrow();
            s.restriction.as_ref().is_some_and(|r| !r.includes(x, z))
        };
        if restriction_excludes {
            return lot_code::OUTSIDE;
        }
        // 1: water.
        if !water_ok && self.is_cell_water(x, z) {
            return lot_code::WATER;
        }
        // 2: real mode, not zoneNotNeeded, cell not zoned.
        let real = self.st.borrow().real_mode;
        if real && !zone_not_needed {
            let zone = self.st.borrow().current_zone;
            let zoned = if zone != 0 {
                self.host.zone_type(x, z) == zone
            } else {
                is_rci_zone(self.host.zone_type(x, z))
            };
            if !zoned {
                return lot_code::NOT_ZONED;
            }
        }
        // 3: network.
        if self.exists_network_of_type(x, z, net::ANY) {
            return lot_code::NETWORK;
        }
        // 7: occupied.
        let occupied = if real {
            self.lot_at(x, z).is_some()
        } else {
            let virt_deleted = self.st.borrow().virtual_deleted.contains_key(&(x, z));
            let real_lot = self.lot_at(x, z).is_some();
            let virt_created = self.st.borrow().virtual_created.contains_key(&(x, z));
            (real_lot && !virt_deleted) || virt_created
        };
        if occupied {
            return lot_code::OCCUPIED;
        }
        lot_code::FREE
    }

    /// Slot 22 `CreateLotIfPossible(x, z, w, h, facing, zoneNotNeeded,
    /// waterAllowed)` (§8.5). The `cfg` argument is ignored *[quirk]*. Returns
    /// the created lot's caller reference (or a virtual-lot success as `None`
    /// in highlight mode is signalled through [`CreateResult`]).
    pub fn create_lot_if_possible(&self, x: i32, z: i32, w: i32, h: i32, facing: i32) -> Option<u64> {
        self.create_lot_if_possible_full(x, z, w, h, facing, false, false).lot
    }

    /// The full `CreateLotIfPossible` (§8.5), returning both the lot and
    /// whether it succeeded (highlight mode returns AL = 1 with no lot).
    pub fn create_lot_if_possible_full(
        &self,
        x: i32,
        z: i32,
        w: i32,
        h: i32,
        facing: i32,
        zone_not_needed: bool,
        water_allowed: bool,
    ) -> CreateResult {
        let code = self.can_create_lot_cfg(x, z, w, h, zone_not_needed, water_allowed, None);
        if code == lot_code::FREE {
            if !self.st.borrow().real_mode {
                // Highlight mode: record a virtual lot for every cell.
                let mut s = self.st.borrow_mut();
                for xx in x..x + w {
                    for zz in z..z + h {
                        s.virtual_created.insert((xx, zz), VirtualLot { x, z, facing });
                    }
                }
                return CreateResult {
                    lot: None,
                    ok: true,
                };
            }
            if facing == -1 {
                // CreateLotToFaceRoad with a fresh empty list (one draw).
                let empty: [i32; 0] = [];
                let lot = self.create_lot_to_face_road(x, z, w, h, &empty);
                return CreateResult {
                    ok: lot.is_some(),
                    lot,
                };
            }
            let lot = self.create_lot(x, z, w, h, facing);
            return CreateResult {
                ok: lot.is_some(),
                lot,
            };
        }
        CreateResult { lot: None, ok: false }
    }

    /// Slot 23 `CreateLot(x, z, w, h, facing)` (§8.5). No checks. Returns the
    /// lot's caller reference.
    pub fn create_lot(&self, x: i32, z: i32, w: i32, h: i32, facing: i32) -> Option<u64> {
        // 1. Allocate and construct a lot (a new token with no reference).
        let lot = self.host.new_lot(x, z, w, h, facing);
        // 2. One AddRef for the caller; Init; SetState(1,0,0); one more AddRef
        //    for the manager.
        self.host.add_ref(lot); // caller's reference (returned)
        self.host.lot_init(lot);
        self.host.lot_set_state(lot, 1, 0, 0);
        self.host.add_ref(lot); // manager's reference
        // 3. Write the lot into every cell of the rect.
        {
            let mut s = self.st.borrow_mut();
            for xx in x..x + w {
                for zz in z..z + h {
                    if let Some(i) = Self::index_in(xx, zz, s.map_x, s.map_z) {
                        if i < s.cell_map.len() {
                            s.cell_map[i] = Some(lot);
                        }
                    }
                }
            }
        }
        // 4. Insertion point.
        let it = self.host.desired_lot_insertion_point();
        match it {
            Some(point) if point.list => {
                let mut s = self.st.borrow_mut();
                let len = s.master.len();
                let pos = match point.node {
                    Some(node) => match s.master.iter().position(|&l| l == node) {
                        Some(i) => i + 1, // insert after the node
                        None => len,      // not in the list → append
                    },
                    None => len, // the end → append
                };
                let pos = pos.min(len);
                s.master.insert(pos, lot);
                // Move the iterator to its node's predecessor.
                let pred = if pos >= 2 { Some(s.master[pos - 2]) } else { None };
                drop(s);
                self.host.set_lot_insertion_point(pred);
            }
            Some(_point) => {
                // list is null: nothing inserted, count unchanged [quirk].
            }
            None => {
                // Append to the master list and count it.
                self.st.borrow_mut().master.push(lot);
            }
        }
        // 5. SetEmptyLotBaseTexture.
        self.host.set_empty_lot_base_texture(lot);
        Some(lot)
    }

    fn index_in(x: i32, z: i32, mx: i32, mz: i32) -> Option<usize> {
        if x < 0 || z < 0 || x >= mx || z >= mz {
            return None;
        }
        Some(x as usize * mz as usize + z as usize)
    }

    // ---- lot manager: deletion (§8.6) ----

    /// Slot 24 `DeleteLot(lot)` (§8.6). Highlight mode records the lot's cells
    /// as virtually deleted; real mode runs the six steps.
    pub fn delete_lot(&self, lot: u64) {
        let rect = self.host.lot_bounding_rect(lot);
        let facing = self.host.lot_facing(lot);
        if !self.st.borrow().real_mode {
            let mut s = self.st.borrow_mut();
            for x in rect.x1..=rect.x2 {
                for z in rect.z1..=rect.z2 {
                    s.virtual_deleted.insert((x, z), VirtualLot { x: rect.x1, z: rect.z1, facing });
                }
            }
            return;
        }
        // 1. RemoveBuildingZotProp.
        self.host.remove_building_zot_prop(lot);
        // 2. GetLotBaseTextureOccupant → RemoveOccupant + release.
        if let Some(occ) = self.host.lot_base_texture_occupant(lot) {
            self.host.remove_occupant(occ);
            self.host.release(occ);
        }
        // 3. Null every cell of the lot.
        {
            let mut s = self.st.borrow_mut();
            for x in rect.x1..=rect.x2 {
                for z in rect.z1..=rect.z2 {
                    if let Some(i) = Self::index_in(x, z, s.map_x, s.map_z) {
                        if i < s.cell_map.len() && s.cell_map[i] == Some(lot) {
                            s.cell_map[i] = None;
                        }
                    }
                }
            }
        }
        // 4. Remove from the master list; advance safe iterators.
        {
            let mut s = self.st.borrow_mut();
            if let Some(pos) = s.master.iter().position(|&l| l == lot) {
                s.master.remove(pos);
                for it in s.safe_iters.iter_mut() {
                    if it.index > pos {
                        it.index -= 1;
                    }
                }
            }
        }
        // 5. Business unit.
        if self.host.lot_has_business_unit(lot) {
            self.host.remove_lot_from_business_unit(lot);
        }
        // 6. Shutdown and release the manager's reference.
        self.host.lot_shutdown(lot);
        self.host.release(lot);
    }

    /// Slot 25 `DeleteLots(x1, z1, x2, z2)` (§8.6).
    pub fn delete_lots(&self, rect: CellRect) {
        let lots = self.get_lots_in_cell_rect(rect, false);
        for lot in lots {
            self.delete_lot(lot);
            self.host.release(lot); // the extra reference from the list
        }
    }

    // ---- persistence (§10.2) ----

    /// `Write` (§10.2). Returns `GetError() = 0`.
    pub fn write(&self, out: &mut dyn OutStream) -> bool {
        let s = self.st.borrow();
        let mut ok = out.put_u16(DEVELOPER_RECORD_VERSION);
        ok &= out.put_u32(s.map_x as u32); // countX (rows)
        ok &= out.put_u32(if s.map_x > 0 { s.map_z as u32 } else { 0 }); // countZ (size of row 0)
        // countX × countZ references, x outer, z inner.
        for x in 0..s.map_x {
            for z in 0..s.map_z {
                let i = x as usize * s.map_z as usize + z as usize;
                let r = match s.cell_map.get(i).copied().flatten() {
                    Some(lot) => self.host.lot_ref(lot),
                    None => RefValue::Null,
                };
                ok &= out.put_ref(&r);
            }
        }
        ok & (out.error() == 0)
    }

    /// `Read` (§10.2). Sets the loaded flag; version must be 1. Resizes the
    /// map, reads each cell's lot (IID `0xE818FCED`), rebuilds the master list
    /// in first-cell order (x outer, z inner) with a dedupe set.
    pub fn read(&self, inp: &mut dyn InStream) -> bool {
        let Some(version) = inp.get_u16() else { return false };
        if version != DEVELOPER_RECORD_VERSION {
            return false;
        }
        let Some(count_x) = inp.get_u32() else { return false };
        let Some(count_z) = inp.get_u32() else { return false };
        let cx = count_x as i64;
        let cz = count_z as i64;
        if cx > i64::from(super::region::MAX_REGION_SIDE) || cz > i64::from(super::region::MAX_REGION_SIDE) {
            return false;
        }
        let (mx, mz) = (count_x as i32, count_z as i32);
        let mut cell_map: Vec<Option<u64>> = Vec::new();
        let mut master: Vec<u64> = Vec::new();
        let mut seen: BTreeSet<u64> = BTreeSet::new();
        for _x in 0..mx {
            for _z in 0..mz {
                let r = match inp.get_ref_as(IID_LOT) {
                    Some(r) => r,
                    None => return false,
                };
                let lot = if r.is_null() {
                    None
                } else {
                    self.host.lot_from_ref(&r)
                };
                if let Some(lot) = lot {
                    if seen.insert(lot) {
                        master.push(lot); // first sighting: one reference kept
                    } else {
                        self.host.release(lot); // later sighting: release the extra
                    }
                }
                cell_map.push(lot);
                if inp.error() != 0 {
                    return false;
                }
            }
        }
        let mut s = self.st.borrow_mut();
        s.cell_map = cell_map;
        s.map_x = mx;
        s.map_z = mz;
        s.master = master;
        s.loaded = true;
        true
    }
}

/// The result of [`ZoneDeveloper::create_lot_if_possible_full`] (§8.5): the
/// created lot's caller reference (if any) and EAX = (`*out` ≠ null), which is
/// AL = 1 in highlight mode with no lot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CreateResult {
    pub lot: Option<u64>,
    pub ok: bool,
}

/// The fixed seeds used where a real clock is not wanted (tests).
pub const TEST_SEEDS: FixedSeeds = FixedSeeds {
    wall_clock: 0,
    process_clock: 0,
};
