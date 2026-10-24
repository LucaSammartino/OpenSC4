// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The lot configuration manager component (§B1–§B6).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use crate::props;
use crate::stl::StlHashSet;
use crate::stream::{InStream, OutStream, RefValue, ResKey, SaveObject};

use super::catalogue::SharedLotConfigCatalogue;
use super::{
    kind, prop, BuildingInfo, LcmService, LotConfigHost, LotConfigObject, Rect2D, CELL_METRES,
    CLSID_LOT_CONFIG_MANAGER, FIX_EDGE_BITS, FIX_EPS_BITS, FIX_MARGIN_BITS, IID_BUILDING_DEVELOPMENT, IID_FLORA,
    IID_LOT_CONFIG_MANAGER, IID_PROP_MANAGER, IID_SERIALIZABLE, IID_UNKNOWN, LOT_CONFIG_GROUP, MSG_POST_CITY_INIT,
    RECORD_VERSION,
};

/// `QueryInterface` (§B1): `1`, `0xC8EDBEC0` and `cIGZSerializable`; the
/// message target is not offered.
pub fn query_interface(iid: u32) -> bool {
    matches!(iid, IID_UNKNOWN | IID_LOT_CONFIG_MANAGER | IID_SERIALIZABLE)
}

/// Every instance field of §B2.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LcmState {
    /// +0x0C.
    pub initialized: bool,
    /// +0x10.
    pub ref_count: u32,
    /// The last `Release` destroyed the object.
    pub destroyed: bool,
    /// +0x14 (saved).
    pub building_development: Option<u64>,
    /// +0x18 (saved).
    pub flora: Option<u64>,
    /// +0x1C (saved).
    pub prop_manager: Option<u64>,
    /// +0x20, +0x24: registered filter → cached result (`None` until first
    /// used).
    pub filters: BTreeMap<u64, Option<Vec<u64>>>,
}

/// The lot configuration manager.
pub struct LotConfigManager {
    host: Rc<dyn LotConfigHost>,
    catalogue: SharedLotConfigCatalogue,
    token: Cell<u64>,
    s: RefCell<LcmState>,
}

/// Raises `w` and `d` separately when strictly larger (NaN never).
fn raise(r: &mut Rect2D, w: f32, d: f32) {
    if w > r[0] {
        r[0] = w;
    }
    if d > r[1] {
        r[1] = d;
    }
}

impl LotConfigManager {
    /// A constructed manager (§B1) sharing the process catalogue.
    pub fn new(host: Rc<dyn LotConfigHost>, catalogue: SharedLotConfigCatalogue, token: u64) -> Rc<LotConfigManager> {
        Rc::new(LotConfigManager {
            host,
            catalogue,
            token: Cell::new(token),
            s: RefCell::new(LcmState::default()),
        })
    }

    pub fn token(&self) -> u64 {
        self.token.get()
    }

    pub fn set_token(&self, token: u64) {
        self.token.set(token);
    }

    pub fn catalogue(&self) -> SharedLotConfigCatalogue {
        self.catalogue.clone()
    }

    pub fn snapshot(&self) -> LcmState {
        self.s.borrow().clone()
    }

    pub fn with_state<R>(&self, f: impl FnOnce(&mut LcmState) -> R) -> R {
        f(&mut self.s.borrow_mut())
    }

    // ---- identity (§B1) ----

    pub fn add_ref(&self) -> u32 {
        let mut s = self.s.borrow_mut();
        s.ref_count = s.ref_count.wrapping_add(1);
        s.ref_count
    }

    /// `Release` (`0x004897C0`): the last one calls `Shutdown` and destroys
    /// the object.
    pub fn release(&self) -> u32 {
        let c = self.s.borrow().ref_count;
        if c > 1 {
            self.s.borrow_mut().ref_count = c - 1;
            return c - 1;
        }
        self.shutdown();
        let mut s = self.s.borrow_mut();
        s.ref_count = 0;
        s.destroyed = true;
        0
    }

    // ---- the catalogue (§B3) ----

    /// `StaticInit` (`0x0048A540`, §B3.1): builds the catalogue, or rebuilds
    /// it when the detail preference changed since the last build.
    pub fn static_init(&self) {
        let h = &*self.host;
        let d = h.detail_level();
        let (loaded, stored) = {
            let c = self.catalogue.borrow();
            (c.loaded, c.detail)
        };
        if loaded && d == stored {
            return;
        }
        if loaded {
            self.unload();
        }
        {
            let mut c = self.catalogue.borrow_mut();
            c.loaded = true;
            c.callback_registered = true;
        }
        h.register_shutdown_callback();
        if let Some(keys) = h.exemplar_keys() {
            for key in keys {
                if key.group == LOT_CONFIG_GROUP {
                    self.add_by_key(key);
                }
            }
        }
        self.catalogue.borrow_mut().detail = d;
    }

    /// The unload (`0x0048A240`, also the process shutdown callback): clears
    /// the flag, unregisters the callback, releases every configuration and
    /// empties the map (the bucket count stays). The family set is kept
    /// (quirk 10).
    pub fn unload(&self) {
        let configs: Vec<u64> = {
            let mut c = self.catalogue.borrow_mut();
            c.loaded = false;
            c.callback_registered = false;
            let v = c.configs.iter().map(|(_, t)| *t).collect();
            c.configs.clear();
            v
        };
        self.host.unregister_shutdown_callback();
        for t in configs {
            self.host.release(t);
        }
    }

    /// Slot 5 `AddLotConfiguration(key)` (§B3.2).
    pub fn add_by_key(&self, key: ResKey) -> bool {
        let h = &*self.host;
        let d = h.detail_level();
        let cfg = h.create_configuration();
        if !h.load_configuration(cfg, key, d) {
            h.release(cfg);
            return false;
        }
        let r = self.add_configuration(cfg, true);
        h.release(cfg);
        r
    }

    /// Slot 6 `AddLotConfiguration(cfg, add)` (§B3.4). Always true.
    pub fn add_configuration(&self, cfg: u64, add: bool) -> bool {
        let h = &*self.host;
        let id = h.config_id(cfg);
        let existing = self.catalogue.borrow().get(id);
        match (existing, add) {
            (None, true) => {
                h.add_ref(cfg);
                let families = h.config_families(cfg);
                {
                    let mut c = self.catalogue.borrow_mut();
                    for f in families {
                        c.families.insert(f);
                    }
                }
                self.flush();
                self.catalogue.borrow_mut().configs.entry(id, || cfg);
            }
            (None, false) => {}
            // [quirk] No flush, no family-set update.
            (Some(old), true) => {
                h.release(old);
                h.add_ref(cfg);
                if let Some(v) = self.catalogue.borrow_mut().configs.get_mut(id) {
                    *v = cfg;
                }
            }
            // [quirk] Not released; the family set is not updated.
            (Some(_), false) => {
                self.flush();
                self.catalogue.borrow_mut().configs.remove(id);
            }
        }
        true
    }

    /// Slot 7 `RemoveLotConfiguration(id)`.
    pub fn remove_configuration(&self, id: u32) -> bool {
        let cfg = self.catalogue.borrow().get(id);
        match cfg {
            Some(c) => self.add_configuration(c, false),
            None => false,
        }
    }

    /// Slot 8 `GetLotConfiguration(id)`: no reference is added.
    pub fn configuration(&self, id: u32) -> Option<u64> {
        self.catalogue.borrow().get(id)
    }

    /// Slot 20: the map in bucket order.
    pub fn configuration_map(&self) -> Vec<(u32, u64)> {
        self.catalogue.borrow().map()
    }

    /// Slot 21: the family set in bucket order.
    pub fn family_id_set(&self) -> Vec<u32> {
        self.catalogue.borrow().family_set()
    }

    fn all_configs(&self) -> Vec<u64> {
        self.catalogue.borrow().configs.iter().map(|(_, t)| *t).collect()
    }

    // ---- lifecycle (§B4) ----

    /// Slot 3 `Init` (§B4.1). True.
    pub fn init(&self) -> bool {
        let h = &*self.host;
        {
            let mut s = self.s.borrow_mut();
            if s.initialized {
                return true;
            }
            s.initialized = true;
        }
        self.static_init();
        if self.s.borrow().building_development.is_none() {
            let t = h.acquire_service(LcmService::BuildingDevelopment);
            self.s.borrow_mut().building_development = t;
        }
        if self.s.borrow().flora.is_none() {
            let t = h.acquire_service(LcmService::Flora);
            self.s.borrow_mut().flora = t;
        }
        if self.s.borrow().prop_manager.is_none() {
            let t = h.acquire_service(LcmService::PropManager);
            self.s.borrow_mut().prop_manager = t;
        }
        h.subscribe(MSG_POST_CITY_INIT);
        true
    }

    /// Slot 4 `Shutdown` (§B4.2). The catalogue stays. True.
    pub fn shutdown(&self) -> bool {
        let h = &*self.host;
        if !self.s.borrow().initialized {
            return true;
        }
        h.unsubscribe(MSG_POST_CITY_INIT);
        let held = {
            let mut s = self.s.borrow_mut();
            [s.building_development.take(), s.flora.take(), s.prop_manager.take()]
        };
        for t in held.into_iter().flatten() {
            h.release(t);
        }
        self.s.borrow_mut().initialized = false;
        true
    }

    /// `DoMessage` (§B4.3). True.
    pub fn do_message(&self, msg_type: u32) -> bool {
        if msg_type == MSG_POST_CITY_INIT {
            for cfg in self.all_configs() {
                self.fill_building_information(cfg);
            }
        }
        true
    }

    /// `0x0048AD40` (§B4.3): the building information of one configuration.
    pub fn fill_building_information(&self, cfg: u64) {
        let h = &*self.host;
        let mut info = BuildingInfo::default();
        if let Some(obj) = h.config_primary_building(cfg) {
            let mut groups = BTreeSet::new();
            let mut range: Option<(u16, u16)> = None;
            for &id in &obj.ids {
                let mut set = StlHashSet::with_hint(100);
                h.building_types_into_set(&mut set, id);
                for ty in set.iter_order() {
                    match h.building_exemplar(ty) {
                        Some(ex) => {
                            groups.extend(props::u32s(&ex, prop::OCCUPANT_GROUPS).unwrap_or_default());
                            if !props::bool_of(&ex, prop::MONOPOLY).unwrap_or(false) {
                                info.non_monopoly = true;
                            }
                        }
                        // [inferred] (PL7) As an absent monopoly property.
                        None => info.non_monopoly = true,
                    }
                    let c = h.building_capacity(ty);
                    range = Some(match range {
                        None => (c, c),
                        Some((a, b)) => (a.min(c), b.max(c)),
                    });
                }
            }
            if let Some((a, b)) = range {
                info.min_capacity = a;
                info.max_capacity = b;
            }
            info.groups = groups.into_iter().collect();
        }
        h.set_config_building_info(cfg, &info);
    }

    // ---- set queries (§B5.2) ----

    fn with_refs(&self, v: Vec<u64>) -> Vec<u64> {
        for &t in &v {
            self.host.add_ref(t);
        }
        v
    }

    /// Slot 9 `GetLotConfigurationsByFilter(set, filter)` (§B5.4): the
    /// matches, each with a new reference; the count is their number.
    pub fn configurations_by_filter(&self, filter: Option<u64>) -> Vec<u64> {
        let h = &*self.host;
        let matches = match filter {
            None => self.all_configs(),
            Some(f) => {
                let registered = self.s.borrow().filters.get(&f).cloned();
                match registered {
                    Some(Some(cached)) => cached,
                    Some(None) => {
                        let v: Vec<u64> = self
                            .all_configs()
                            .into_iter()
                            .filter(|&c| h.filter_matches(f, c))
                            .collect();
                        if let Some(e) = self.s.borrow_mut().filters.get_mut(&f) {
                            *e = Some(v.clone());
                        }
                        v
                    }
                    None => self
                        .all_configs()
                        .into_iter()
                        .filter(|&c| h.filter_matches(f, c))
                        .collect(),
                }
            }
        };
        self.with_refs(matches)
    }

    /// Slot 10 `GetLotConfigurationsByFamily(set, family)`.
    pub fn configurations_by_family(&self, family: u32) -> Vec<u64> {
        let v = self.matching(|c| self.host.config_belongs_to_family(c, family));
        self.with_refs(v)
    }

    /// Slot 11 `GetLotConfigurationsBySize(set, x, z)`.
    pub fn configurations_by_size(&self, x: u32, z: u32) -> Vec<u64> {
        let v = self.matching(|c| self.host.config_size(c) == (x, z));
        self.with_refs(v)
    }

    /// Slot 12 `ClearLotConfigurationSet(set)`: releases every element once
    /// (the caller empties its set).
    pub fn clear_set(&self, set: &[u64]) {
        for &t in set {
            self.host.release(t);
        }
    }

    fn matching(&self, pred: impl Fn(u64) -> bool) -> Vec<u64> {
        self.all_configs().into_iter().filter(|&c| pred(c)).collect()
    }

    fn ids_of(&self, v: Vec<u64>) -> Vec<u32> {
        v.into_iter().map(|c| self.host.config_id(c)).collect()
    }

    /// Slot 13 `GetLotConfigurationIDsByFilter(set, filter)`: null filter,
    /// or filter slot 3 true (no cache).
    pub fn ids_by_filter(&self, filter: Option<u64>) -> Vec<u32> {
        let v = self.matching(|c| filter.map_or(true, |f| self.host.filter_matches(f, c)));
        self.ids_of(v)
    }

    /// Slot 14 `GetLotConfigurationIDsByFamily(set, family)`.
    pub fn ids_by_family(&self, family: u32) -> Vec<u32> {
        let v = self.matching(|c| self.host.config_belongs_to_family(c, family));
        self.ids_of(v)
    }

    /// Slot 15 `GetLotConfigurationIDsBySize(set, x, z)`.
    pub fn ids_by_size(&self, x: u32, z: u32) -> Vec<u32> {
        let v = self.matching(|c| self.host.config_size(c) == (x, z));
        self.ids_of(v)
    }

    // ---- building tests (§B5.3) ----

    /// Slot 16 `LotConfigurationHasBuildingFamily(cfg, family, obj)`: the
    /// index (in the object array) of the first type-0 record listing
    /// `family`.
    pub fn has_building_family(&self, cfg: Option<u64>, family: u32) -> Option<usize> {
        let cfg = cfg?;
        if family == 0 {
            return None;
        }
        self.host
            .config_objects(cfg)
            .iter()
            .position(|o| u32::from(o.kind) == kind::BUILDING && o.ids.contains(&family))
    }

    /// Slot 17 `LotConfigurationHasBuildingType(cfg, type, obj)`.
    pub fn has_building_type(&self, cfg: Option<u64>, ty: u32) -> Option<usize> {
        let cfg = cfg?;
        if ty == 0 {
            return None;
        }
        let objects = self.host.config_objects(cfg);
        for (i, o) in objects.iter().enumerate() {
            if u32::from(o.kind) != kind::BUILDING {
                continue;
            }
            if o.ids.contains(&ty) {
                return Some(i);
            }
            for &f in &o.ids {
                if self.host.is_building_in_family(ty, f) {
                    return Some(i);
                }
            }
        }
        None
    }

    // ---- filters (§B5.4) ----

    /// Slot 18: adds the filter with no cached result (a second
    /// registration does nothing).
    pub fn register_filter(&self, filter: u64) {
        self.s.borrow_mut().filters.entry(filter).or_insert(None);
    }

    /// Slot 19: frees the cached result and removes the filter.
    pub fn unregister_filter(&self, filter: u64) {
        self.s.borrow_mut().filters.remove(&filter);
    }

    /// The flush (`0x0048A080`): drops every cached result.
    pub fn flush(&self) {
        for v in self.s.borrow_mut().filters.values_mut() {
            *v = None;
        }
    }

    // ---- footprints (§B5.5) ----

    /// Slot 24 `GetMaxFootprintForBuildingFamilies`.
    pub fn footprint_buildings(&self, ids: &[u32]) -> Rect2D {
        let h = &*self.host;
        let mut r = [4.0, 4.0, 0.0, 0.0];
        let one = |r: &mut Rect2D, t: u32| -> bool {
            if !h.is_building_type_valid(t) {
                return false;
            }
            match h.building_footprint(t) {
                Some((w, d)) => {
                    raise(r, w, d);
                    true
                }
                None => false,
            }
        };
        for &id in ids {
            if one(&mut r, id) {
                continue;
            }
            let mut types = Vec::new();
            if h.building_types_by_family(&mut types, id) != 0 {
                for t in types {
                    one(&mut r, t);
                }
            }
        }
        r
    }

    /// Slot 25 `GetMaxFootprintForFloraFamilies` (the global flora
    /// simulator).
    pub fn footprint_flora(&self, ids: &[u32]) -> Rect2D {
        let h = &*self.host;
        let mut r = [2.0, 2.0, 0.0, 0.0];
        for &id in ids {
            if let Some(rad) = h.flora_radius(id) {
                raise(&mut r, 2.0 * rad, 2.0 * rad);
                continue;
            }
            let mut set = StlHashSet::with_hint(100);
            h.flora_types_into_set(&mut set, id);
            for t in set.iter_order() {
                if let Some(rad) = h.flora_radius(t) {
                    raise(&mut r, 2.0 * rad, 2.0 * rad);
                }
            }
        }
        r
    }

    /// Slot 26 `GetMaxFootprintForPropFamilies` (the global prop manager).
    pub fn footprint_props(&self, ids: &[u32]) -> Rect2D {
        let h = &*self.host;
        let mut r = [2.0, 2.0, 0.0, 0.0];
        for &id in ids {
            if let Some((x, z)) = h.prop_size(id) {
                raise(&mut r, x, z);
                continue;
            }
            let mut types = Vec::new();
            if h.prop_types_by_family(&mut types, id) != 0 {
                for t in types {
                    if let Some((x, z)) = h.prop_size(t) {
                        raise(&mut r, x, z);
                    }
                }
            }
        }
        r
    }

    // ---- GetObjectIDArray (§B5.6) ----

    /// Slot 27 `GetObjectIDArray(obj, vec)`: the record's IDs with families
    /// expanded (the global managers); a building family assigns the vector
    /// (quirk 13).
    pub fn object_id_array(&self, obj: Option<&LotConfigObject>) -> Vec<u32> {
        let h = &*self.host;
        let mut vec = Vec::new();
        let Some(o) = obj else {
            return vec;
        };
        for &id in &o.ids {
            match u32::from(o.kind) {
                kind::BUILDING => {
                    if h.is_building_type_valid(id) {
                        vec.push(id);
                    } else {
                        h.building_types_by_family(&mut vec, id);
                    }
                }
                kind::PROP => {
                    if h.is_prop_type_valid(id) {
                        vec.push(id);
                    } else {
                        let mut tmp = Vec::new();
                        h.prop_types_by_family(&mut tmp, id);
                        vec.extend(tmp);
                    }
                }
                kind::FLORA => {
                    if h.is_flora_type_valid(id) {
                        vec.push(id);
                    } else {
                        let mut set = StlHashSet::with_hint(100);
                        h.flora_types_into_set(&mut set, id);
                        vec.extend(set.iter_order());
                    }
                }
                _ => {}
            }
        }
        vec
    }

    // ---- editing helpers (§B5.7) ----

    /// Slot 22 `CreateGenericLotConfiguration(cfg, buildingType)`: `None`
    /// when BDS slot 16 rejects the type; else a configuration (not added to
    /// the map, one reference for the caller) whose only object is the
    /// building, sized from slot 24 and centred on a lot of
    /// `1 + floor(w / 16) × 1 + floor(d / 16)` cells (PL12).
    pub fn create_generic(&self, ty: u32) -> Option<u64> {
        let h = &*self.host;
        if !h.is_building_family_type_valid(ty) {
            return None;
        }
        let fp = self.footprint_buildings(&[ty]);
        let (w, d) = (fp[0], fp[1]);
        // [PL12] The lot side is `1 − trunc(m × −0.0625)` (`0x0048BD66`,
        // float multiply then `_ftol` truncation toward zero): `1 +
        // floor(m / 16)` for `m ≥ 0`, one cell *more* than `ceil(m / 16)`
        // when `m` is a multiple of 16. w = 4 → 1, 15.9 → 1, 16 → 2,
        // 24 → 2, 32 → 3, 40 → 3. Slot 24 gives at least 4 × 4 (§B5.5), so
        // the side is at least 1.
        let cells = |m: f32| 1.0f32 - (m * -0.0625f32).trunc();
        let size = (cells(w) as u32, cells(d) as u32);
        let (cx, cz) = (size.0 as f32 * CELL_METRES * 0.5, size.1 as f32 * CELL_METRES * 0.5);
        let obj = LotConfigObject {
            kind: kind::BUILDING as u8,
            offset: [cx, 0.0, cz],
            min_x: cx - w * 0.5,
            min_z: cz - d * 0.5,
            max_x: cx + w * 0.5,
            max_z: cz + d * 0.5,
            ids: vec![ty],
            ..LotConfigObject::default()
        };
        h.create_generic_configuration(ty, size, vec![obj])
    }

    /// Whether `FixLotConfiguration` keeps an ID of a record of type `k`
    /// [inferred] (PL9): building — BDS slot 16 (a family or a type); prop —
    /// prop manager slot 11; flora — flora slot 11 or a non-empty family
    /// (slot 14).
    fn fix_id_valid(&self, k: u32, id: u32) -> bool {
        let h = &*self.host;
        match k {
            kind::BUILDING => h.is_building_family_type_valid(id),
            kind::PROP => h.is_prop_family_type_valid(id),
            _ => {
                if h.is_flora_type_valid(id) {
                    return true;
                }
                let mut set = StlHashSet::with_hint(100);
                h.flora_types_into_set(&mut set, id) != 0
            }
        }
    }

    /// Slot 23 `FixLotConfiguration(cfg)` (§B5.7): the code of the first
    /// problem found (0 none, 1 an ID removed, 2 a record removed, 3 a
    /// footprint changed, 4 outside the lot, 5 a centre moved, 6 an ID of
    /// the slot 28 list removed). For each building, prop and flora record,
    /// in order:
    ///
    /// 1. IDs that are not valid types or families are removed (code 1);
    /// 2. a record left with no IDs is removed (code 2);
    /// 3. the footprint is recomputed from slots 24–26 (`w`, `d` swapped for
    ///    orientations 1 and 3) **around the record's offset** (code 3 when
    ///    it changed) [PL13];
    /// 4. a footprint outside the lot (`size × 16` m, 0.049 m margin) only
    ///    sets code 4 — **nothing is moved** [PL13];
    /// 5. an offset (x, z) outside its footprint is moved to the
    ///    footprint's **centre** (code 5) [PL13].
    ///
    /// Finally the first invalid ID of the slot 28 list is removed (code 6).
    /// A null configuration gives 0 [choice]. Questions PL9, PL10, PL13.
    pub fn fix(&self, cfg: Option<u64>) -> u32 {
        let h = &*self.host;
        let Some(cfg) = cfg else {
            return 0;
        };
        let mut objects = h.config_objects(cfg);
        let mut walls = h.config_retaining_walls(cfg);
        let (code, objects_changed, walls_changed) =
            self.fix_records(h.config_size(cfg), &mut objects, &mut walls, |o| o);
        if objects_changed {
            h.set_config_objects(cfg, objects);
        }
        if walls_changed {
            h.set_config_retaining_walls(cfg, walls);
        }
        code
    }

    /// Same slot-23 algorithm over typed owned records. The accessor preserves
    /// editor pointer IDs and raw decoder fields; dependencies stay on this manager.
    /// Returns (first problem, object mutation, retaining-wall mutation).
    pub fn fix_records<R>(
        &self,
        size: (u32, u32),
        objects: &mut Vec<R>,
        walls: &mut Vec<u32>,
        data: impl Fn(&mut R) -> &mut LotConfigObject,
    ) -> (u32, bool, bool) {
        let h = &*self.host;
        let mut code = 0u32;
        let mut note = |c: u32| {
            if code == 0 {
                code = c;
            }
        };
        let (sx, sz) = size;
        let (lw, ld) = (sx as f32 * CELL_METRES, sz as f32 * CELL_METRES);
        let mut changed = false;
        objects.retain_mut(|record| {
            let o = data(record);
            let k = u32::from(o.kind);
            if !matches!(k, kind::BUILDING | kind::PROP | kind::FLORA) {
                return true;
            }
            let before = o.ids.len();
            o.ids.retain(|&id| self.fix_id_valid(k, id));
            if o.ids.len() != before {
                note(1);
                changed = true;
            }
            if o.ids.is_empty() {
                note(2);
                changed = true;
                return false;
            }
            let fp = match k {
                kind::BUILDING => self.footprint_buildings(&o.ids),
                kind::PROP => self.footprint_props(&o.ids),
                _ => self.footprint_flora(&o.ids),
            };
            let (mut w, mut d) = (fp[0], fp[1]);
            if o.orientation == 1 || o.orientation == 3 {
                std::mem::swap(&mut w, &mut d);
            }
            // [PL13] The rebuild is **gated on the size** (`|oldW − w| > 0`
            // or `|oldD − d| > 0`, strict, NaN false), and then centres the
            // footprint on the record's **offset** (+0x04, +0x0C), not on
            // the old footprint's centre: `min := (x − w × 0.5,
            // z − d × 0.5)` and `max := (x + w × 0.5, z + d × 0.5)`. A
            // footprint of the right size is left where it is, even when it
            // is not centred on the offset (that is step 5's case).
            let (old_w, old_d) = (o.max_x - o.min_x, o.max_z - o.min_z);
            if (old_w - w).abs() > 0.0 || (old_d - d).abs() > 0.0 {
                let (x, z) = (o.offset[0], o.offset[2]);
                (o.min_x, o.min_z, o.max_x, o.max_z) = (x - w * 0.5, z - d * 0.5, x + w * 0.5, z + d * 0.5);
                note(3);
                changed = true;
            }
            // [PL13] `ex := f32(lotW − 0.05)`, `ez := f32(lotD − 0.05)`;
            // code 4 when `minX < 0.049` or `minZ < 0.049` or
            // `ex + 0.001 < maxX` or `ez + 0.001 < maxZ` (strict, NaN
            // never). **Nothing is moved**: the record stays outside the
            // lot. The three constants are the stored float bit patterns of
            // §B5.7 (`0x3D48B43A`, `0x3D4CCCCD`, `0x3A83126F`), so the
            // footprint must sit ~0.049 m inside every lot edge.
            let margin = f32::from_bits(FIX_MARGIN_BITS);
            let (ex, ez) = (lw - f32::from_bits(FIX_EDGE_BITS), ld - f32::from_bits(FIX_EDGE_BITS));
            let eps = f32::from_bits(FIX_EPS_BITS);
            if o.min_x < margin || o.min_z < margin || ex + eps < o.max_x || ez + eps < o.max_z {
                note(4);
            }
            // [PL13] A centre outside its footprint is moved to the
            // footprint's **centre**, not clamped to its nearest point:
            // `x := (maxX + minX) × 0.5`, `z := (minZ + maxZ) × 0.5`. A
            // rebuilt footprint is centred on the offset, so this fires
            // only for an unchanged footprint.
            if o.offset[0] < o.min_x || o.offset[0] > o.max_x || o.offset[2] < o.min_z || o.offset[2] > o.max_z {
                o.offset[0] = (o.max_x + o.min_x) * 0.5;
                o.offset[2] = (o.min_z + o.max_z) * 0.5;
                note(5);
                changed = true;
            }
            true
        });
        let mut walls_changed = false;
        if let Some(i) = walls.iter().position(|&id| !h.is_retaining_wall_valid(id)) {
            walls.remove(i);
            walls_changed = true;
            note(6);
        }
        (code, changed, walls_changed)
    }

    // ---- the record (§B6) ----

    fn parse(&self, inp: &mut dyn InStream, acquired: &mut Vec<u64>) -> Option<[Option<u64>; 3]> {
        if inp.get_u16()? != RECORD_VERSION {
            return None;
        }
        let mut read = |iid: u32| -> Option<Option<u64>> {
            let r = inp.get_ref_as(iid)?;
            let t = if r.is_null() {
                None
            } else {
                self.host.resolve_ref(&r, iid)
            };
            if let Some(t) = t {
                acquired.push(t);
            }
            Some(t)
        };
        let bds = read(IID_BUILDING_DEVELOPMENT)?;
        let flora = read(IID_FLORA)?;
        let pm = read(IID_PROP_MANAGER)?;
        if inp.error() != 0 {
            return None;
        }
        Some([bds, flora, pm])
    }
}

impl SaveObject for LotConfigManager {
    fn class_id(&self) -> u32 {
        CLSID_LOT_CONFIG_MANAGER
    }

    /// §B6: version 1 and three references. True when the stream has no
    /// error.
    fn write(&self, out: &mut dyn OutStream) -> bool {
        let fields = {
            let s = self.s.borrow();
            [s.building_development, s.flora, s.prop_manager]
        };
        out.put_u16(RECORD_VERSION);
        for t in fields {
            out.put_ref(&t.map_or(RefValue::Null, |t| self.host.object_ref(t)));
        }
        out.error() == 0
    }

    /// §B6: another version gives false. [choice] Atomic.
    fn read(&self, inp: &mut dyn InStream) -> bool {
        let mut acquired = Vec::new();
        match self.parse(inp, &mut acquired) {
            Some([bds, flora, pm]) => {
                let old = {
                    let mut s = self.s.borrow_mut();
                    [
                        std::mem::replace(&mut s.building_development, bds),
                        std::mem::replace(&mut s.flora, flora),
                        std::mem::replace(&mut s.prop_manager, pm),
                    ]
                };
                for t in old.into_iter().flatten() {
                    self.host.release(t);
                }
                true
            }
            None => {
                for t in acquired {
                    self.host.release(t);
                }
                false
            }
        }
    }

    fn as_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }

    fn host_token(&self) -> u64 {
        self.token.get()
    }
}
