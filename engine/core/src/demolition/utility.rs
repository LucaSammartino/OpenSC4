// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The demolition utility (§2–§7).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashSet, VecDeque};
use std::rc::Rc;

use crate::grid::CellRect;
use crate::stream::{InStream, OutStream, SaveObject};

use super::{
    effect_key_candidates, flag, group_size_class, model_size_class, msg, occ_type, prop, CellRegion, DemolitionHost,
    DemolitionMessage, DemolitionTuning, Orphans, Outcome, CELL_METRES, CLSID_DEMOLITION, GROUP_PROP_FLORA_LIKE,
    IID_DEMOLITION, IID_MESSAGE_TARGET2, IID_SERIALIZABLE, IID_UNKNOWN, LEVEL_TOLERANCE, LOT_STATE_DESTROYED,
    LOT_STATE_EMPTY, NETWORK_SURFACE_MASK, OCCUPANT_FLAG_IN_MANAGER, OCCUPANT_FLAG_SKIPPABLE, PRIVILEGE_PLAYER,
    PRIVILEGE_SYSTEM, PROP_EFFECT_PROP, PROP_MOVABLE, RECORD_VERSION, VERTEX_HALF_SIDE, ZONES_REPARCELLED,
    ZONE_AIRPORT, ZONE_PLOPPED, ZONE_SEAPORT,
};

/// `QueryInterface` (§1): the sub-object offset an IID answers with (`1`
/// and `0x26DDE2EA` at +0, `cIGZMessageTarget2` at +4, `cIGZSerializable`
/// at +8), `None` for anything else (no `cIGZMessageTarget`).
pub fn query_interface(iid: u32) -> Option<u32> {
    match iid {
        IID_UNKNOWN | IID_DEMOLITION => Some(0),
        IID_MESSAGE_TARGET2 => Some(4),
        IID_SERIALIZABLE => Some(8),
        _ => None,
    }
}

/// The per-call fields of the demolition filter Φ (§4.2).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Filter {
    /// `boundsActive` and `(x0, z0, x1, z1)` (world metres, inclusive).
    pub bounds: Option<(f32, f32, f32, f32)>,
    /// `privilegeActive` and `level`.
    pub level: Option<u32>,
    pub caller: Option<u64>,
    pub view: Option<u64>,
}

/// Every field of §2.
#[derive(Clone, Debug)]
pub struct DemolitionState {
    pub initialized: bool,
    pub ref_count: u32,
    /// `Release` destroyed the object.
    pub destroyed: bool,
    pub cells_x: i32,
    pub cells_z: i32,
    /// Φ exists (created at `Init`, released at `Shutdown`).
    pub filter_created: bool,
    pub filter: Filter,
    pub city_held: bool,
    pub occupant_manager_held: bool,
    pub lot_developer_held: bool,
    pub terrain_held: bool,
    /// `+0x30`: the region of the `DemolishRegion` call in progress.
    pub region: Option<CellRegion>,
    pub tuning: DemolitionTuning,
}

impl Default for DemolitionState {
    /// The constructor's values (§2) and the code-default tuning (§6).
    fn default() -> DemolitionState {
        DemolitionState {
            initialized: false,
            ref_count: 0,
            destroyed: false,
            cells_x: 0,
            cells_z: 0,
            filter_created: false,
            filter: Filter::default(),
            city_held: false,
            occupant_manager_held: false,
            lot_developer_held: false,
            terrain_held: false,
            region: None,
            tuning: DemolitionTuning::DEFAULT,
        }
    }
}

/// The demolition utility.
pub struct DemolitionUtility {
    host: Rc<dyn DemolitionHost>,
    token: Cell<u64>,
    s: RefCell<DemolitionState>,
}

impl DemolitionUtility {
    /// A constructed (not initialised) utility, reference count 0.
    pub fn new(host: Rc<dyn DemolitionHost>, token: u64) -> Rc<DemolitionUtility> {
        Rc::new(DemolitionUtility {
            host,
            token: Cell::new(token),
            s: RefCell::new(DemolitionState::default()),
        })
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn snapshot(&self) -> DemolitionState {
        self.s.borrow().clone()
    }

    pub fn with_state<R>(&self, f: impl FnOnce(&mut DemolitionState) -> R) -> R {
        f(&mut self.s.borrow_mut())
    }

    // ---- identity (§1, §8) ----

    /// Slot 1: increments and returns the new count.
    pub fn add_ref(&self) -> u32 {
        let mut s = self.s.borrow_mut();
        s.ref_count = s.ref_count.wrapping_add(1);
        s.ref_count
    }

    /// Slot 2: above 1 it decrements; otherwise `Shutdown`, destroy (here:
    /// mark) and return 0.
    pub fn release(&self) -> u32 {
        let c = self.s.borrow().ref_count;
        if c > 1 {
            self.s.borrow_mut().ref_count = c - 1;
            return c - 1;
        }
        self.shutdown();
        self.s.borrow_mut().destroyed = true;
        0
    }

    // ---- lifecycle (§4.1) and tuning (§6) ----

    /// Slot 3 `Init` (§4.1). Returns true.
    pub fn init(&self) -> bool {
        let h = &*self.host;
        {
            let mut s = self.s.borrow_mut();
            if s.initialized {
                return true;
            }
            s.initialized = true;
        }
        for m in msg::SUBSCRIPTIONS {
            h.subscribe(m);
        }
        if !self.s.borrow().city_held && h.acquire_city() {
            let (x, z) = h.city_cell_counts();
            let mut s = self.s.borrow_mut();
            s.city_held = true;
            s.cells_x = x;
            s.cells_z = z;
        }
        if !self.s.borrow().occupant_manager_held && h.acquire_occupant_manager() {
            self.s.borrow_mut().occupant_manager_held = true;
        }
        if !self.s.borrow().lot_developer_held && h.acquire_lot_developer() {
            self.s.borrow_mut().lot_developer_held = true;
        }
        if !self.s.borrow().terrain_held && h.acquire_terrain() {
            self.s.borrow_mut().terrain_held = true;
        }
        {
            let mut s = self.s.borrow_mut();
            s.filter_created = true;
            s.filter = Filter::default();
        }
        self.read_properties();
        true
    }

    /// Slot 4 `Shutdown` (§4.1): releases Φ, the terrain, the lot developer,
    /// the occupant manager and the city, then unsubscribes. Returns true.
    pub fn shutdown(&self) -> bool {
        let h = &*self.host;
        let held = {
            let mut s = self.s.borrow_mut();
            if !s.initialized {
                return true;
            }
            s.initialized = false;
            s.filter_created = false;
            s.filter = Filter::default();
            let held = (
                s.terrain_held,
                s.lot_developer_held,
                s.occupant_manager_held,
                s.city_held,
            );
            s.terrain_held = false;
            s.lot_developer_held = false;
            s.occupant_manager_held = false;
            s.city_held = false;
            held
        };
        if held.0 {
            h.release_terrain();
        }
        if held.1 {
            h.release_lot_developer();
        }
        if held.2 {
            h.release_occupant_manager();
        }
        if held.3 {
            h.release_city();
        }
        for m in msg::SUBSCRIPTIONS {
            h.unsubscribe(m);
        }
        true
    }

    /// `ReadProperties` (§6): missing exemplars or properties keep the
    /// current values.
    pub fn read_properties(&self) {
        let h = &*self.host;
        let thresholds = h.thresholds_exemplar();
        let bd = h.building_development_exemplar();
        let mut s = self.s.borrow_mut();
        if let Some(ex) = thresholds.as_deref() {
            let t = &mut s.tuning;
            for (id, field) in [
                (prop::SMALL_MEDIUM_MODEL, &mut t.small_medium_model),
                (prop::MEDIUM_LARGE_MODEL, &mut t.medium_large_model),
                (prop::SMALL_MEDIUM_GROUP, &mut t.small_medium_group),
                (prop::MEDIUM_LARGE_GROUP, &mut t.medium_large_group),
            ] {
                if let Some(v) = crate::props::u32_of(ex, id) {
                    *field = v;
                }
            }
        }
        if let Some(ex) = bd.as_deref() {
            if let Some(v) = crate::props::u32_of(ex, prop::DESTROYED_TILE_COST) {
                s.tuning.destroyed_tile_cost = v;
            }
        }
    }

    /// Slot 5 `SetDefaultOccupantFilter`: does nothing.
    pub fn set_default_occupant_filter(&self, _filter: Option<u64>) {}

    // ---- messages (§3) ----

    /// `DoMessage` (§3). Returns true.
    pub fn do_message(&self, m: &DemolitionMessage) -> bool {
        match m.msg_type {
            // [quirk] Any exemplar: data1 / data2 are not compared.
            msg::EXEMPLAR_CHANGED => self.read_properties(),
            msg::DEFERRED_LOT_DEMOLITION => self.deferred_lot_demolition(m.lot, m.dezone),
            _ => {}
        }
        true
    }

    /// The `0xA9DED179` handler (§3). The lot is released whenever it is
    /// non-null: the spec's *[choice]* (DM10; the original leaks it when the
    /// lot is not initialised, which nothing can observe).
    pub fn deferred_lot_demolition(&self, lot: Option<u64>, dezone: bool) {
        let h = &*self.host;
        let Some(lot) = lot else { return };
        if h.lot_initialized(lot) {
            h.end_demolish_lot(lot);
            if dezone {
                h.dezone(h.lot_bounding_rect(lot));
            }
        }
        h.release(lot);
    }

    // ---- the filter Φ (§4.2) ----

    /// `Φ.IsOccupantIncluded(o)` with the current per-call fields.
    pub fn filter_includes(&self, o: u64) -> bool {
        let f = self.s.borrow().filter;
        self.phi(&f, o)
    }

    fn phi(&self, f: &Filter, o: u64) -> bool {
        let h = &*self.host;
        // 1. Realism.
        if !h.realistic_filter_includes(o) && h.default_filter_includes(o) {
            return false;
        }
        // 2.
        if let Some(c) = f.caller {
            if !h.filter_includes(c, o) {
                return false;
            }
        }
        // 3. Position in the box (x and z, inclusive).
        if let Some((x0, z0, x1, z1)) = f.bounds {
            let (x, _, z) = h.occupant_position(o);
            if !(x >= x0 && x <= x1 && z >= z0 && z <= z1) {
                return false;
            }
        }
        // 4.
        if let Some(level) = f.level {
            if !h.can_remove(o, level) {
                return false;
            }
        }
        // 5. Type.
        let t = h.occupant_type(o);
        match t {
            // DM5 [choice]: without the foundation interface, rejected.
            occ_type::FOUNDATION => return h.foundation_slot9(o) == Some(1),
            occ_type::BUILDING
            | occ_type::POWER_POLE
            | occ_type::CONSTRUCTION
            | occ_type::PROP
            | occ_type::NETWORK
            | occ_type::FLORA => {}
            _ => return false,
        }
        // 6.
        if let Some(v) = f.view {
            if !h.filter_includes(v, o) {
                return false;
            }
        }
        // 7. [choice] No view: level 1.
        if t == occ_type::NETWORK && h.view_level().unwrap_or(1) != 1 {
            if let Some(true) = h.network_has_any_flag(o, NETWORK_SURFACE_MASK) {
                return false;
            }
        }
        true
    }

    // ---- DemolishOccupant (§4.3) ----

    /// Slot 8 `DemolishOccupant` (§4.3). `privilege` is not used by the
    /// method itself.
    pub fn demolish_occupant(
        &self,
        do_it: bool,
        o: u64,
        _privilege: u32,
        flags: u32,
        skip_network_update: bool,
        orphans: &mut Orphans,
    ) -> Outcome {
        let h = &*self.host;
        // 1. Cost untouched.
        if flags & flag::SKIP_FLAG_40 != 0 && h.occupant_flag_set(o, OCCUPANT_FLAG_SKIPPABLE) {
            return Outcome { ok: false, cost: None };
        }
        // 2.
        let (ok, c) = match h.occupant_type(o) {
            occ_type::FOUNDATION => {
                let ok = if do_it && h.occupant_flag_set(o, OCCUPANT_FLAG_IN_MANAGER) {
                    h.remove_occupant(o, true)
                } else {
                    true
                };
                (ok, 0)
            }
            occ_type::CONSTRUCTION => (if do_it { h.remove_occupant(o, true) } else { true }, 0),
            // Buildings go with their lot (§4.4 step 7).
            occ_type::BUILDING => match h.building_demolition_cost(o) {
                Some(c) => (true, c),
                None => (false, 0),
            },
            occ_type::POWER_POLE if h.is_power_pole(o) => {
                let c = h.pole_demolition_cost(o);
                if !do_it {
                    (true, c)
                } else {
                    let remembered = h.pole_connections(o);
                    h.send_pre_demolish(o);
                    let removed = h.remove_occupant(o, false);
                    if removed {
                        h.pole_break_all_connections(o);
                        h.occupant_shutdown(o);
                        for p in remembered {
                            if h.pole_connection_count(p) == 0 {
                                match orphans {
                                    Orphans::None => {}
                                    Orphans::Host(list) => h.list_append(*list, p),
                                    Orphans::Local(v) => v.push(p),
                                }
                            }
                        }
                    }
                    (removed, c)
                }
            }
            occ_type::PROP => {
                if !h.has_prop_manager() {
                    (false, 0)
                } else if do_it {
                    let removed = h.remove_prop(o);
                    if removed {
                        h.occupant_shutdown(o);
                    }
                    (removed, 0)
                } else {
                    (true, 0)
                }
            }
            occ_type::NETWORK if h.is_network_occupant(o) => {
                let c = h.network_demolition_cost(o);
                if do_it && !skip_network_update && h.occupant_flags(o) & OCCUPANT_FLAG_IN_MANAGER != 0 {
                    self.update_networks(&[o]);
                }
                (true, c)
            }
            occ_type::FLORA => match h.flora_demolition_cost(o) {
                Some(c) => {
                    if do_it {
                        // The removal's result is ignored.
                        h.remove_occupant(o, true);
                    }
                    (true, c)
                }
                None => (false, 0),
            },
            _ => (false, 0),
        };
        // 3.
        Outcome { ok, cost: Some(c) }
    }

    /// `UpdateNetworks(set)` (§4.6), in the given (set) order.
    pub fn update_networks(&self, set: &[u64]) {
        let h = &*self.host;
        let mut list = Vec::new();
        for &o in set {
            if !h.occupant_flag_set(o, OCCUPANT_FLAG_IN_MANAGER) {
                continue;
            }
            // DM9: by occupant type alone (no QueryInterface, no network
            // kind filter).
            match h.occupant_type(o) {
                occ_type::POWER_POLE => {
                    h.send_pre_demolish(o);
                    h.remove_occupant(o, true);
                }
                occ_type::NETWORK => {
                    list.push(o);
                    h.delete_occupant_connection(o);
                }
                _ => {}
            }
        }
        if !list.is_empty() {
            h.destroy_network_occupants(&list);
        }
    }

    // ---- DemolishRegion (§4.4) ----

    /// Slot 6 `DemolishRegion` (§4.4). `cost` of the outcome is the value of
    /// step 9 (always written when the pointer is non-null).
    #[allow(clippy::too_many_arguments)]
    pub fn demolish_region(
        &self,
        do_it: bool,
        region: &CellRegion,
        privilege: u32,
        flags: u32,
        dezone: bool,
        caller_filter: Option<u64>,
        demolished: Option<u64>,
        clicked: Option<u64>,
        sx: i32,
        sy: i32,
    ) -> Outcome {
        let h = &*self.host;
        // 1.
        let effects = do_it && flags & flag::CAUSE != 0;
        self.s.borrow_mut().region = Some(region.clone());
        // 2. [choice] No view: no filter, level 1. DM12: the view filter is
        // fetched whatever `use_view` is; it gates only Φ, while the empty-lot
        // qualification (step 4) and the effect gate (step 5) use it directly.
        let view_filter = h.acquire_view_filter();
        let _ = h.view_level();
        let level = if privilege == PRIVILEGE_SYSTEM { 1 } else { 2 };
        let use_view = privilege != PRIVILEGE_SYSTEM && flags & flag::LOTS_ONLY == 0;
        let phi = Filter {
            bounds: None,
            level: Some(level),
            caller: caller_filter,
            view: if use_view { view_filter } else { None },
        };
        self.s.borrow_mut().filter = phi;
        // 3.
        let mut result = true;
        let mut occs: BTreeSet<u64> = BTreeSet::new();
        let mut lots: VecDeque<u64> = VecDeque::new();
        let mut seen: HashSet<u64> = HashSet::new();
        let mut count: i32 = 0;
        // 4.
        for (x, z) in region.cells() {
            let lot = h.lot_at(x, z);
            if let Some(l) = lot {
                if seen.insert(l) {
                    let b = h.lot_building(l);
                    let qualifies = match b {
                        None => {
                            matches!(h.lot_state(l), LOT_STATE_EMPTY | LOT_STATE_DESTROYED)
                                && view_filter.map_or(true, |f| h.filter_includes_type(f, occ_type::BUILDING))
                        }
                        Some(b) => h.lot_demolishable(l) && self.phi(&phi, b),
                    };
                    if qualifies {
                        let lot_occs = h.lot_occupants(l);
                        let bare = Filter { view: None, ..phi };
                        let blocked = lot_occs.iter().any(|&o| {
                            matches!(h.occupant_type(o), occ_type::BUILDING | occ_type::PROP) && !self.phi(&bare, o)
                        });
                        if blocked {
                            result = false;
                        } else {
                            lots.push_front(l);
                            for &o in &lot_occs {
                                if self.phi(&phi, o) && occs.insert(o) {
                                    count += 1;
                                }
                            }
                        }
                    } else if privilege == PRIVILEGE_PLAYER {
                        if let Some(b) = b {
                            if !h.can_remove(b, 2) {
                                result = false;
                            }
                        }
                    }
                }
            }
            // The per-cell pass.
            let (x0, z0) = h.cell_corner_to_position(x, z);
            let (x1, z1) = h.cell_corner_to_position(x + 1, z + 1);
            let cell = Filter {
                bounds: Some((x0, z0, x1, z1)),
                ..phi
            };
            self.s.borrow_mut().filter = cell;
            let accepted: Vec<u64> = h
                .occupants_in_box(x0, z0, x1, z1)
                .into_iter()
                .filter(|&o| self.phi(&cell, o))
                .collect();
            self.s.borrow_mut().filter = phi;
            for o in accepted {
                let inc = h.default_filter_includes(o);
                if (lot.is_none() || !inc) && (flags & flag::LOTS_ONLY == 0 || inc) && occs.insert(o) {
                    count += 1;
                }
            }
        }
        // 5.
        let mut total: i64 = 0;
        let mut orphans: Vec<u64> = Vec::new();
        for &o in &occs {
            let first = self.demolish_occupant(
                do_it && !effects,
                o,
                privilege,
                flags,
                true,
                &mut Orphans::Local(&mut orphans),
            );
            if !first.ok {
                // [quirk] A failed occupant does not fail the region.
                continue;
            }
            let mut c = first.cost.unwrap_or(0);
            if effects {
                if view_filter.map_or(true, |f| h.filter_includes(f, o)) {
                    let lot = self.lot_at_occupant(o);
                    self.do_demolish_occupant_effect(o, flags, lot, Some(o) == clicked, sx, sy);
                }
                let real = self.demolish_occupant(true, o, privilege, flags, true, &mut Orphans::Local(&mut orphans));
                c = real.cost.unwrap_or(0);
            }
            total = total.wrapping_add(c);
            if let Some(set) = demolished {
                h.set_insert(set, o);
            }
        }
        // 6. [quirk] n = 0: always the small group variant.
        if effects && count > 0 {
            self.do_demolish_group_effect(flags, 0, count, region.rect);
        }
        // 7.
        if do_it {
            h.queue_removals(true);
        }
        let tile_cost = i64::from(self.s.borrow().tuning.destroyed_tile_cost);
        for &lot in &lots {
            // [quirk] Added in dry runs too.
            if h.lot_state(lot) == LOT_STATE_DESTROYED {
                let (sx, sz) = h.lot_size(lot);
                total = total.wrapping_add(i64::from(sx) * i64::from(sz) * tile_cost);
            }
            if do_it {
                self.clear_lot(lot, flags, dezone);
            }
        }
        // 8.
        while let Some(o) = orphans.pop() {
            occs.insert(o);
            if let Some(set) = demolished {
                h.set_insert(set, o);
            }
        }
        if do_it {
            let set: Vec<u64> = occs.iter().copied().collect();
            self.update_networks(&set);
            h.queue_removals(false);
        }
        // 9.
        {
            let mut s = self.s.borrow_mut();
            s.filter = Filter::default();
            s.region = None;
        }
        if let Some(f) = view_filter {
            h.release(f);
        }
        Outcome {
            ok: result,
            cost: Some(total),
        }
    }

    /// The lot under the occupant's position (`PositionToCell`, `GetLot`).
    fn lot_at_occupant(&self, o: u64) -> Option<u64> {
        let h = &*self.host;
        let (x, _, z) = h.occupant_position(o);
        let (cx, cz) = h.position_to_cell(x, z)?;
        h.lot_at(cx, cz)
    }

    /// §4.4 step 7, committed: explode, or clear and dezone / re-parcel.
    fn clear_lot(&self, lot: u64, flags: u32, dezone: bool) {
        let h = &*self.host;
        if h.lot_water_compatible(lot) != Some(true) && flags & flag::EXPLODE != 0 {
            h.explode_lot(lot, flags & flag::EXPLODE_FALSE == 0);
            return;
        }
        let zone = h.lot_zone_type(lot);
        h.start_demolish_lot(lot);
        h.end_demolish_lot(lot);
        if dezone || matches!(zone, ZONE_AIRPORT | ZONE_SEAPORT | ZONE_PLOPPED) {
            h.dezone(h.lot_bounding_rect(lot));
        } else if flags & flag::NO_REPARCEL == 0 && ZONES_REPARCELLED.contains(&zone) {
            let rect = h.lot_bounding_rect(lot);
            let facing = h.lot_facing(lot);
            h.delete_lot(lot);
            for x in rect.x1..=rect.x2 {
                for z in rect.z1..=rect.z2 {
                    if h.zone_at(x, z) == zone {
                        h.create_lot_if_possible(x, z, facing);
                    }
                }
            }
        }
        // Otherwise an RCI lot stays as an empty zoned lot (state 1).
    }

    // ---- DemolishLots (§4.5) ----

    /// Slot 7 `DemolishLots` (§4.5). An empty list returns true and leaves
    /// the cost untouched. [choice] Null entries are skipped; a list of only
    /// null entries behaves like an empty one.
    #[allow(clippy::too_many_arguments)]
    pub fn demolish_lots(
        &self,
        do_it: bool,
        lots: &[Option<u64>],
        privilege: u32,
        flags: u32,
        dezone: bool,
        caller_filter: Option<u64>,
        demolished: Option<u64>,
    ) -> Outcome {
        let h = &*self.host;
        let lots: Vec<u64> = lots.iter().flatten().copied().collect();
        if lots.is_empty() {
            return Outcome { ok: true, cost: None };
        }
        let rects: Vec<CellRect> = lots.iter().map(|&l| h.lot_bounding_rect(l)).collect();
        let mut rect = rects[0];
        for r in &rects[1..] {
            rect.x1 = rect.x1.min(r.x1);
            rect.z1 = rect.z1.min(r.z1);
            rect.x2 = rect.x2.max(r.x2);
            rect.z2 = rect.z2.max(r.z2);
        }
        let mut region = CellRegion::new(rect, false);
        for (&l, r) in lots.iter().zip(&rects) {
            // Lots not in a demolishable state are silently skipped.
            if h.lot_demolishable(l) {
                for x in r.x1..=r.x2 {
                    for z in r.z1..=r.z2 {
                        region.set(x, z, true);
                    }
                }
            }
        }
        self.demolish_region(
            do_it,
            &region,
            privilege,
            flags | flag::LOTS_ONLY,
            dezone,
            caller_filter,
            demolished,
            None,
            0,
            0,
        )
    }

    // ---- ModifyTerrainHeight (§4.7) ----

    /// The immovable occupants in `q` (the immovability filter, §4.7).
    fn immovables(&self, q: CellRect) -> Vec<u64> {
        let h = &*self.host;
        h.occupants_in_cell_rect(q)
            .into_iter()
            .filter(|&o| {
                matches!(
                    h.occupant_type(o),
                    occ_type::BUILDING | occ_type::PROP | occ_type::POWER_POLE | occ_type::NETWORK
                ) && h.occupant_bool_property(o, PROP_MOVABLE) != Some(true)
            })
            .collect()
    }

    /// Slot 9 `ModifyTerrainHeight` (§4.7): the vertex count; `*cost` is
    /// never written.
    #[allow(clippy::too_many_arguments)]
    pub fn modify_terrain_height(
        &self,
        do_it: bool,
        per_vertex: bool,
        x0: i32,
        z0: i32,
        x1: i32,
        z1: i32,
        alt: f32,
        exempt: Option<u64>,
        reason: u32,
    ) -> i32 {
        let h = &*self.host;
        let (cells_x, cells_z) = {
            let s = self.s.borrow();
            (s.cells_x, s.cells_z)
        };
        let q = CellRect::new(
            (x0 - 1).max(0),
            (z0 - 1).max(0),
            x1.min(cells_x - 1),
            z1.min(cells_z - 1),
        );
        let immovable = self.immovables(q);
        // DM6: |alt − h| in f32 against 0.05f (the x87 runs at 24 bits).
        let differs = |a: f32| (a - alt).abs();
        let locked = |l: u64| Some(l) != exempt && h.lot_vertices_locked(l);
        if !per_vertex {
            // 1.
            if !immovable.is_empty() {
                return 0;
            }
            // 2.
            let mut n = 0i32;
            for x in x0..=x1 {
                for z in z0..=z1 {
                    if differs(h.altitude_at_vertex(x, z)) > LEVEL_TOLERANCE {
                        n += 1;
                    }
                }
            }
            if n == 0 {
                return 0;
            }
            // 3.
            let is_locked = h.lots_in_cell_rect(q).into_iter().any(locked);
            // 4. DM7: the redisplay gets the SetAltitude rectangle.
            if !is_locked && do_it {
                let rect = CellRect::new(x0.max(0), z0.max(0), x1.min(cells_x), z1.min(cells_z));
                h.set_altitude_rect(rect, alt, reason);
                h.redisplay_terrain(rect, reason);
            }
            return n;
        }
        let boxes: Vec<(f32, f32, f32, f32)> = immovable.iter().map(|&o| h.occupant_bounds(o)).collect();
        let mut n = 0i32;
        for x in x0..=x1 {
            for z in z0..=z1 {
                if h.is_water_vertex(x, z) {
                    continue;
                }
                // [question DM15] `<` as §4.7 states: exactly 0.05f is set.
                if differs(h.altitude_at_vertex(x, z)) < LEVEL_TOLERANCE {
                    continue;
                }
                let lot_locked = [(x - 1, z - 1), (x, z - 1), (x - 1, z), (x, z)]
                    .into_iter()
                    .filter_map(|(cx, cz)| h.lot_at(cx, cz))
                    .any(locked);
                if lot_locked {
                    continue;
                }
                let (sx0, sx1) = (
                    CELL_METRES * x as f32 - VERTEX_HALF_SIDE,
                    CELL_METRES * x as f32 + VERTEX_HALF_SIDE,
                );
                let (sz0, sz1) = (
                    CELL_METRES * z as f32 - VERTEX_HALF_SIDE,
                    CELL_METRES * z as f32 + VERTEX_HALF_SIDE,
                );
                let blocked = boxes
                    .iter()
                    .any(|&(bx0, bz0, bx1, bz1)| bx0 <= sx1 && bx1 >= sx0 && bz0 <= sz1 && bz1 >= sz0);
                if blocked {
                    continue;
                }
                n += 1;
                if do_it {
                    h.set_altitude_vertex(x, z, alt);
                }
            }
        }
        // [quirk] Also in a dry run.
        if n > 0 {
            h.redisplay_terrain(CellRect::new(q.x1, q.z1, q.x2 + 1, q.z2 + 1), reason);
        }
        n
    }

    // ---- effects (§4.8) ----

    /// Slot 10 `DoDemolishOccupantEffect` (§4.8). An occupant type without a
    /// listed nibble (foundation, construction) uses nibble 0 (DM13).
    pub fn do_demolish_occupant_effect(&self, o: u64, flags: u32, lot: Option<u64>, clicked: bool, sx: i32, sy: i32) {
        let h = &*self.host;
        let Some(s) = h.effect_extent(o) else { return };
        let nibble = match h.occupant_type(o) {
            occ_type::BUILDING | occ_type::POWER_POLE => 0x10,
            occ_type::FLORA => 0x20,
            occ_type::PROP => {
                if h.occupant_bool_property(o, PROP_EFFECT_PROP) != Some(true) {
                    return;
                }
                if h.is_occupant_group(o, GROUP_PROP_FLORA_LIKE) {
                    0x20
                } else {
                    0x30
                }
            }
            occ_type::NETWORK => {
                if h.network_has_type(o, 1) && !h.network_has_type(o, 2) {
                    0x50
                } else {
                    0x40
                }
            }
            _ => 0,
        };
        let mut k = ((flags & flag::CAUSE) << 8) + nibble;
        h.play_occupant_sound(o);
        let tuning = self.s.borrow().tuning;
        k += model_size_class(s, &tuning);
        let keys = effect_key_candidates(k);
        let grey = lot.is_some_and(|l| !matches!(h.lot_condition(l), 0 | 1));
        h.start_occupant_effect(o, &keys, clicked.then_some((sx, sy)), grey);
    }

    /// Slot 11 `DoDemolishGroupEffect(flags, n, count, rect)` (§4.8): the
    /// size compares `n` (not `count`) with the group thresholds.
    pub fn do_demolish_group_effect(&self, flags: u32, n: i32, _count: i32, rect: CellRect) {
        let tuning = self.s.borrow().tuning;
        let instance = ((flags & flag::CAUSE) << 8) | group_size_class(n, &tuning);
        let cx = ((i64::from(rect.x1) + i64::from(rect.x2)) / 2) as i32;
        let cz = ((i64::from(rect.z1) + i64::from(rect.z2)) / 2) as i32;
        self.host.start_group_effect(instance, cx, cz);
    }
}

impl SaveObject for DemolitionUtility {
    fn class_id(&self) -> u32 {
        CLSID_DEMOLITION
    }

    /// §7: `u16 1`. True unless the stream reports an error.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        out.put_u16(RECORD_VERSION);
        out.error() == 0
    }

    /// §7: any version other than 1 is refused.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        match inp.get_u16() {
            Some(RECORD_VERSION) => inp.error() == 0,
            _ => false,
        }
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
