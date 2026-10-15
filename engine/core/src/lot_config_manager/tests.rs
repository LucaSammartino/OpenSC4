// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! A fake game and the lot configuration manager tests of prop-lotconfig.md
//! §T (7–12) and the tables of Part B.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use super::*;
use crate::property::PropertyHolder;
use crate::stream::{ByteReader, ByteWriter, RefResolver, SaveObject, SaveRegistry};
use crate::variant::{mem, Elem, Variant};

const BDS: u64 = 0x50;
const FLORA: u64 = 0x51;
const PM: u64 = 0x52;
const ME: u64 = 0xAAAA;

#[derive(Clone, Debug, Default)]
struct Cfg {
    id: u32,
    families: Vec<u32>,
    size: (u32, u32),
    objects: Vec<LotConfigObject>,
    info: Option<BuildingInfo>,
    walls: Vec<u32>,
    generic_for: Option<u32>,
}

#[derive(Default)]
struct Game {
    log: RefCell<Vec<String>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    detail: Cell<u32>,
    /// Exemplar keys and their contents (`None`: `Load` fails).
    keys: RefCell<Vec<(ResKey, Option<Rc<PropertyHolder>>)>>,
    /// Per configuration ID: (families, size).
    meta: RefCell<BTreeMap<u32, (Vec<u32>, (u32, u32))>>,
    no_resource_manager: Cell<bool>,
    next: Cell<u64>,
    serial: Cell<u32>,
    configs: RefCell<BTreeMap<u64, Cfg>>,
    /// Filter token → the configuration IDs it accepts.
    filters: RefCell<BTreeMap<u64, BTreeSet<u32>>>,
    filter_calls: Cell<u32>,
    /// Building type → (capacity, footprint, exemplar).
    building_types: RefCell<BTreeMap<u32, (u16, Option<(f32, f32)>, Option<Rc<PropertyHolder>>)>>,
    building_families: RefCell<BTreeMap<u32, Vec<u32>>>,
    flora_types: RefCell<BTreeMap<u32, f32>>,
    flora_families: RefCell<BTreeMap<u32, Vec<u32>>>,
    prop_types: RefCell<BTreeMap<u32, (f32, f32)>>,
    prop_families: RefCell<BTreeMap<u32, Vec<u32>>>,
    walls_valid: RefCell<BTreeSet<u32>>,
}

impl Game {
    fn new() -> Rc<Game> {
        let g = Rc::new(Game::default());
        g.next.set(0x1000);
        g.detail.set(3);
        g
    }
    fn log(&self, s: impl Into<String>) {
        self.log.borrow_mut().push(s.into());
    }
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
    fn refs(&self, t: u64) -> i64 {
        self.refs.borrow().get(&t).copied().unwrap_or(0)
    }
    fn bump(&self, t: u64, d: i64) {
        *self.refs.borrow_mut().entry(t).or_insert(0) += d;
    }
    fn new_token(&self) -> u64 {
        let t = self.next.get();
        self.next.set(t + 1);
        t
    }
    fn cfg(&self, t: u64) -> Cfg {
        self.configs.borrow().get(&t).cloned().unwrap_or_default()
    }
    /// A loose configuration (not from an exemplar), one reference.
    fn make_cfg(&self, id: u32, families: &[u32], size: (u32, u32), objects: Vec<LotConfigObject>) -> u64 {
        let t = self.new_token();
        self.bump(t, 1);
        self.configs.borrow_mut().insert(
            t,
            Cfg {
                id,
                families: families.to_vec(),
                size,
                objects,
                ..Cfg::default()
            },
        );
        t
    }
    /// A lot configuration exemplar in group `0xA8FBD372`.
    fn lot_exemplar(&self, id: u32, families: &[u32], size: (u32, u32), objects: &[Vec<u32>]) {
        let mut h = PropertyHolder::new();
        h.add(prop::VERSION, Variant::Scalar(Elem::U32(2)));
        for (i, v) in objects.iter().enumerate() {
            h.add(
                prop::LOT_OBJECT_FIRST + i as u32,
                Variant::array(mem::UINT32, v.iter().map(|&x| Elem::U32(x)).collect()),
            );
        }
        self.keys.borrow_mut().push((key(id), Some(Rc::new(h))));
        self.meta.borrow_mut().insert(id, (families.to_vec(), size));
    }
    fn building_type(&self, ty: u32, capacity: u16, footprint: Option<(f32, f32)>, ex: Option<PropertyHolder>) {
        self.building_types
            .borrow_mut()
            .insert(ty, (capacity, footprint, ex.map(Rc::new)));
    }
    fn object_count(&self, m: &LotConfigManager) -> usize {
        m.configuration_map()
            .iter()
            .map(|&(_, t)| self.cfg(t).objects.len())
            .sum()
    }
}

fn key(instance: u32) -> ResKey {
    ResKey {
        type_id: crate::props::EXEMPLAR_TYPE,
        group: LOT_CONFIG_GROUP,
        instance,
    }
}

/// Ten values of a lot object (+ the IDs after the tag).
fn obj_values(v0: u32, v1: u32, ids: &[u32]) -> Vec<u32> {
    let mut v = vec![v0, v1, 0, 0, 0, 0, 0, 0, 0x10_0000, 0x10_0000, 0, 0x77];
    v.extend_from_slice(ids);
    v
}

fn record(k: u32, ids: &[u32]) -> LotConfigObject {
    LotConfigObject {
        kind: k as u8,
        ids: ids.to_vec(),
        ..LotConfigObject::default()
    }
}

impl LotConfigHost for Game {
    fn detail_level(&self) -> u32 {
        self.detail.get()
    }
    fn exemplar_keys(&self) -> Option<Vec<ResKey>> {
        if self.no_resource_manager.get() {
            return None;
        }
        Some(self.keys.borrow().iter().map(|(k, _)| *k).collect())
    }
    fn register_shutdown_callback(&self) {
        self.log("register callback");
    }
    fn unregister_shutdown_callback(&self) {
        self.log("unregister callback");
    }
    fn create_configuration(&self) -> u64 {
        let t = self.new_token();
        self.bump(t, 1);
        self.configs.borrow_mut().insert(t, Cfg::default());
        t
    }
    fn load_configuration(&self, cfg: u64, key: ResKey, detail: u32) -> bool {
        let h = self
            .keys
            .borrow()
            .iter()
            .find(|(k, _)| *k == key)
            .and_then(|(_, h)| h.clone());
        let Some(h) = h else {
            return false;
        };
        let mut serial = self.serial.get();
        let objects = lot_object_array(&h, detail, &mut serial);
        self.serial.set(serial);
        let (families, size) = self.meta.borrow().get(&key.instance).cloned().unwrap_or_default();
        let mut c = self.configs.borrow_mut();
        let c = c.get_mut(&cfg).unwrap();
        c.id = key.instance;
        c.families = families;
        c.size = size;
        c.objects = objects;
        true
    }
    fn config_id(&self, cfg: u64) -> u32 {
        self.cfg(cfg).id
    }
    fn config_belongs_to_family(&self, cfg: u64, family: u32) -> bool {
        self.cfg(cfg).families.contains(&family)
    }
    fn config_families(&self, cfg: u64) -> Vec<u32> {
        self.cfg(cfg).families
    }
    fn config_size(&self, cfg: u64) -> (u32, u32) {
        self.cfg(cfg).size
    }
    fn config_objects(&self, cfg: u64) -> Vec<LotConfigObject> {
        self.cfg(cfg).objects
    }
    fn set_config_objects(&self, cfg: u64, objects: Vec<LotConfigObject>) {
        self.configs.borrow_mut().get_mut(&cfg).unwrap().objects = objects;
    }
    fn config_primary_building(&self, cfg: u64) -> Option<LotConfigObject> {
        self.cfg(cfg)
            .objects
            .into_iter()
            .find(|o| u32::from(o.kind) == kind::BUILDING)
    }
    fn set_config_building_info(&self, cfg: u64, info: &BuildingInfo) {
        self.configs.borrow_mut().get_mut(&cfg).unwrap().info = Some(info.clone());
    }
    fn config_retaining_walls(&self, cfg: u64) -> Vec<u32> {
        self.cfg(cfg).walls
    }
    fn set_config_retaining_walls(&self, cfg: u64, ids: Vec<u32>) {
        self.configs.borrow_mut().get_mut(&cfg).unwrap().walls = ids;
    }
    fn is_retaining_wall_valid(&self, id: u32) -> bool {
        self.walls_valid.borrow().contains(&id)
    }
    fn create_generic_configuration(&self, ty: u32, size: (u32, u32), objects: Vec<LotConfigObject>) -> Option<u64> {
        let t = self.make_cfg(0, &[], size, objects);
        self.configs.borrow_mut().get_mut(&t).unwrap().generic_for = Some(ty);
        Some(t)
    }
    fn acquire_service(&self, service: LcmService) -> Option<u64> {
        let t = match service {
            LcmService::BuildingDevelopment => BDS,
            LcmService::Flora => FLORA,
            LcmService::PropManager => PM,
        };
        self.bump(t, 1);
        Some(t)
    }
    fn subscribe(&self, m: u32) {
        self.log(format!("subscribe {m:08X}"));
    }
    fn unsubscribe(&self, m: u32) {
        self.log(format!("unsubscribe {m:08X}"));
    }
    fn add_ref(&self, t: u64) {
        self.bump(t, 1);
    }
    fn release(&self, t: u64) {
        self.bump(t, -1);
    }
    fn filter_matches(&self, filter: u64, cfg: u64) -> bool {
        self.filter_calls.set(self.filter_calls.get() + 1);
        let id = self.cfg(cfg).id;
        self.filters.borrow().get(&filter).is_some_and(|s| s.contains(&id))
    }
    fn building_exemplar(&self, ty: u32) -> Option<Rc<PropertyHolder>> {
        self.building_types.borrow().get(&ty).and_then(|t| t.2.clone())
    }
    fn is_building_type_valid(&self, id: u32) -> bool {
        self.building_types.borrow().contains_key(&id)
    }
    fn is_building_family_type_valid(&self, id: u32) -> bool {
        self.is_building_type_valid(id) || self.building_families.borrow().contains_key(&id)
    }
    fn building_types_by_family(&self, vec: &mut Vec<u32>, id: u32) -> u32 {
        if let Some(f) = self.building_families.borrow().get(&id) {
            *vec = f.clone();
            return f.len() as u32;
        }
        if self.is_building_type_valid(id) {
            *vec = vec![id];
        }
        0
    }
    fn building_types_into_set(&self, set: &mut crate::stl::StlHashSet, id: u32) -> u32 {
        let f = self.building_families.borrow().get(&id).cloned().unwrap_or_default();
        for &t in &f {
            set.insert(t);
        }
        f.len() as u32
    }
    fn is_building_in_family(&self, ty: u32, family: u32) -> bool {
        self.building_families
            .borrow()
            .get(&family)
            .is_some_and(|f| f.contains(&ty))
    }
    fn building_capacity(&self, ty: u32) -> u16 {
        self.building_types.borrow().get(&ty).map_or(0, |t| t.0)
    }
    fn building_footprint(&self, ty: u32) -> Option<(f32, f32)> {
        self.building_types.borrow().get(&ty).and_then(|t| t.1)
    }
    fn flora_radius(&self, id: u32) -> Option<f32> {
        self.flora_types.borrow().get(&id).copied()
    }
    fn is_flora_type_valid(&self, id: u32) -> bool {
        self.flora_types.borrow().contains_key(&id)
    }
    fn flora_types_into_set(&self, set: &mut crate::stl::StlHashSet, id: u32) -> u32 {
        let f = self.flora_families.borrow().get(&id).cloned().unwrap_or_default();
        for &t in &f {
            set.insert(t);
        }
        f.len() as u32
    }
    fn is_prop_type_valid(&self, id: u32) -> bool {
        self.prop_types.borrow().contains_key(&id)
    }
    fn prop_types_by_family(&self, vec: &mut Vec<u32>, id: u32) -> u32 {
        if let Some(f) = self.prop_families.borrow().get(&id) {
            vec.extend_from_slice(f);
            return f.len() as u32;
        }
        if self.is_prop_type_valid(id) {
            vec.push(id);
        }
        0
    }
    fn is_prop_family_type_valid(&self, id: u32) -> bool {
        self.is_prop_type_valid(id) || self.prop_families.borrow().contains_key(&id)
    }
    fn prop_size(&self, id: u32) -> Option<(f32, f32)> {
        self.prop_types.borrow().get(&id).copied()
    }
    fn object_ref(&self, t: u64) -> RefValue {
        let class_id = match t {
            BDS => 0x8990_BE7A,
            FLORA => 0xC990_C034,
            PM => 0xA990_BD10,
            _ => 0,
        };
        RefValue::Host { token: t, class_id }
    }
    fn resolve_ref(&self, r: &RefValue, _iid: u32) -> Option<u64> {
        match r {
            RefValue::Host { token, .. } => {
                self.bump(*token, 1);
                Some(*token)
            }
            _ => None,
        }
    }
}

fn manager(g: &Rc<Game>) -> Rc<LotConfigManager> {
    LotConfigManager::new(g.clone(), LotConfigCatalogue::shared(), ME)
}

fn holder_with_objects(version: Option<u32>, objects: &[Vec<u32>]) -> PropertyHolder {
    let mut h = PropertyHolder::new();
    if let Some(v) = version {
        h.add(prop::VERSION, Variant::Scalar(Elem::U32(v)));
    }
    for (i, v) in objects.iter().enumerate() {
        h.add(
            prop::LOT_OBJECT_FIRST + i as u32,
            Variant::array(mem::UINT32, v.iter().map(|&x| Elem::U32(x)).collect()),
        );
    }
    h
}

#[test]
fn identity() {
    for iid in [1, 0xC8ED_BEC0, 0xE4FD_A3D4] {
        assert!(query_interface(iid));
    }
    assert!(!query_interface(0x4522_94AA));
    assert!(!query_interface(0x2832_6F3E));
    let g = Game::new();
    assert_eq!(manager(&g).class_id(), 0x2990_62C0);
}

/// §T 7: the detail filter.
#[test]
fn t7_detail_filter() {
    let objs = [
        obj_values(0, 0x30, &[0xB]),
        obj_values(1, 0x00, &[0xA]),
        obj_values(1, 0x10, &[0xA]),
        obj_values(2, 0x20, &[0xC]),
        obj_values(4, 0x30, &[0xF]),
    ];
    let h = holder_with_objects(Some(2), &objs);
    let kinds = |d: u32| -> Vec<(u8, u8)> {
        let mut s = 0;
        lot_object_array(&h, d, &mut s)
            .iter()
            .map(|o| (o.kind, o.flags))
            .collect()
    };
    assert_eq!(kinds(1), vec![(0, 0x30), (1, 0x00)]);
    assert_eq!(kinds(2), vec![(0, 0x30), (1, 0x00), (1, 0x10)]);
    assert_eq!(kinds(3), vec![(0, 0x30), (1, 0x00), (1, 0x10), (2, 0x20)]);
    assert_eq!(lod_of(0x1F), 1);
    assert!(is_kept(1, 0x1F, 2) && !is_kept(1, 0x1F, 1));
    assert!(is_kept(0, 0x30, 1));
}

#[test]
fn object_reader_details() {
    // Version 2: the tag is v11 and the IDs start at v12.
    let h = holder_with_objects(Some(2), &[obj_values(1, 0, &[0xA, 0xB])]);
    let mut s = 5;
    let o = &lot_object_array(&h, 3, &mut s)[0];
    assert_eq!((o.tag, o.ids.clone(), s), (0x77, vec![0xA, 0xB], 5));
    assert_eq!((o.max_x, o.max_z), (16.0, 16.0));
    // Version 1 (and absent): a generated tag, the IDs from v11.
    for v in [Some(1), None] {
        let h = holder_with_objects(v, &[obj_values(1, 0, &[0xA]), obj_values(2, 0, &[])]);
        let mut s = 5;
        let a = lot_object_array(&h, 3, &mut s);
        assert_eq!(a[0].tag, 5);
        assert_eq!(a[0].ids, vec![0x77, 0xA]);
        assert_eq!(a[1].tag, 6);
        assert_eq!(s, 7);
    }
    // Fewer than 10 values are skipped; a missing property stops the scan.
    let mut h = holder_with_objects(Some(2), &[vec![1, 0, 0], obj_values(1, 0, &[1])]);
    h.add(
        prop::LOT_OBJECT_FIRST + 3,
        Variant::array(mem::UINT32, obj_values(1, 0, &[2]).into_iter().map(Elem::U32).collect()),
    );
    let mut s = 0;
    let a = lot_object_array(&h, 3, &mut s);
    assert_eq!(a.len(), 1);
    assert_eq!(a[0].ids, vec![1]);
    // Negative 16.16 offsets.
    assert_eq!(fixed_16_16(0xFFFF_8000), -0.5);
    let o = object_from_values(&[1, 0, 0x8000_0003, 0, 0, 0, 0, 0, 0, 0], None);
    assert_eq!((o.orientation, o.mirror, o.tag), (3, 1, 0));
    assert!(o.ids.is_empty());
}

fn three_configs(g: &Game) {
    g.lot_exemplar(
        0x11,
        &[0x900],
        (2, 2),
        &[
            obj_values(0, 0, &[0xB1]),
            obj_values(1, 0x10, &[0xA1]),
            obj_values(1, 0x20, &[0xA1]),
        ],
    );
    g.lot_exemplar(0x12, &[0x900, 0x901], (1, 2), &[obj_values(0, 0, &[0xB2])]);
    g.lot_exemplar(
        0x13,
        &[],
        (2, 2),
        &[
            obj_values(0, 0, &[0xB3]),
            obj_values(2, 0, &[0xC1]),
            obj_values(4, 0x20, &[0xF1]),
        ],
    );
}

#[test]
fn init_and_shutdown() {
    let g = Game::new();
    three_configs(&g);
    // A key of another group and a failing load are ignored.
    g.keys.borrow_mut().push((
        ResKey {
            type_id: crate::props::EXEMPLAR_TYPE,
            group: 0x1234,
            instance: 0x99,
        },
        Some(Rc::new(PropertyHolder::new())),
    ));
    g.keys.borrow_mut().push((key(0x14), None));
    let m = manager(&g);
    assert!(m.init());
    let s = m.snapshot();
    assert!(s.initialized);
    assert_eq!(
        (s.building_development, s.flora, s.prop_manager),
        (Some(BDS), Some(FLORA), Some(PM))
    );
    assert_eq!(g.take(), vec!["register callback", "subscribe 26D31EC1"]);
    assert_eq!(
        m.configuration_map().iter().map(|e| e.0).collect::<Vec<_>>(),
        vec![0x11, 0x12, 0x13]
    );
    for &(_, t) in &m.configuration_map() {
        assert_eq!(g.refs(t), 1, "the map holds one reference");
    }
    assert_eq!(m.family_id_set(), vec![0x900, 0x901]);
    // The failed configuration was released.
    let failed: Vec<u64> = g
        .configs
        .borrow()
        .iter()
        .filter(|(_, c)| c.id == 0)
        .map(|(t, _)| *t)
        .collect();
    assert_eq!(failed.len(), 1);
    assert_eq!(g.refs(failed[0]), 0);
    // A second Init does nothing.
    assert!(m.init());
    assert!(g.take().is_empty());
    assert!(m.shutdown());
    assert_eq!(g.take(), vec!["unsubscribe 26D31EC1"]);
    let s = m.snapshot();
    assert!(!s.initialized && s.building_development.is_none() && s.prop_manager.is_none());
    assert_eq!((g.refs(BDS), g.refs(FLORA), g.refs(PM)), (0, 0, 0));
    assert_eq!(m.configuration_map().len(), 3, "the catalogue stays");
    assert!(m.shutdown());
    assert!(g.take().is_empty());
}

#[test]
fn catalogue_without_resource_manager() {
    let g = Game::new();
    three_configs(&g);
    g.no_resource_manager.set(true);
    let m = manager(&g);
    m.init();
    assert!(m.configuration_map().is_empty());
    assert!(m.catalogue().borrow().is_loaded());
}

/// §T 8: the rebuild on a detail change (a small catalogue).
#[test]
fn t8_rebuild_on_detail_change() {
    let g = Game::new();
    three_configs(&g);
    let cat = LotConfigCatalogue::shared();
    let m = LotConfigManager::new(g.clone(), cat.clone(), ME);
    m.init();
    assert_eq!(m.configuration_map().len(), 3);
    assert_eq!(g.object_count(&m), 7);
    let first = m.configuration(0x11).unwrap();
    m.shutdown();
    g.take();
    // The configuration 0x12 and its family 0x901 disappear from the data.
    g.keys.borrow_mut().retain(|(k, _)| k.instance != 0x12);
    g.detail.set(1);
    m.init();
    assert_eq!(
        g.take(),
        vec!["unregister callback", "register callback", "subscribe 26D31EC1"]
    );
    assert_eq!(g.refs(first), 0, "the unload released it");
    assert_eq!(m.configuration_map().len(), 2);
    // Building + LOD-0 objects only: 0x11 keeps 1, 0x13 keeps 2.
    assert_eq!(g.object_count(&m), 3);
    assert_eq!(cat.borrow().detail(), 1);
    // Quirk 10: the family set keeps 0x901.
    assert_eq!(m.family_id_set(), vec![0x900, 0x901]);
    let again = m.configuration(0x11).unwrap();
    assert_ne!(again, first);
    m.shutdown();
    m.init();
    assert_eq!(m.configuration(0x11), Some(again), "no reload at the same detail");
    // A second manager of the process sees the same catalogue.
    let m2 = LotConfigManager::new(g.clone(), cat.clone(), ME + 1);
    m2.init();
    assert_eq!(m2.configuration(0x11), Some(again));
    // The process shutdown callback.
    m.unload();
    assert!(!cat.borrow().is_loaded());
    assert_eq!(g.refs(again), 0);
    assert!(m.configuration_map().is_empty());
    assert_eq!(cat.borrow().configs.bucket_count(), 193, "the bucket count stays");
}

/// §B3.4: the add/remove table (quirk 11) and the flush.
#[test]
fn add_remove_table() {
    let g = Game::new();
    let m = manager(&g);
    m.register_filter(0xF1);
    g.filters.borrow_mut().insert(0xF1, [7u32].into_iter().collect());
    let a = g.make_cfg(7, &[0x900], (1, 1), vec![]);
    // New, not added: nothing.
    assert!(m.add_configuration(a, false));
    assert!(m.configuration(7).is_none());
    // New, added: a reference, the families, a flush.
    assert!(m.add_configuration(a, true));
    assert_eq!(g.refs(a), 2);
    assert_eq!(m.family_id_set(), vec![0x900]);
    let got = m.configurations_by_filter(Some(0xF1));
    assert_eq!(got, vec![a]);
    assert_eq!(m.snapshot().filters[&0xF1], Some(vec![a]));
    // Existing, added: the old one released, the new one stored, no flush,
    // no family update.
    let b = g.make_cfg(7, &[0x901], (1, 1), vec![]);
    assert!(m.add_configuration(b, true));
    assert_eq!((g.refs(a), g.refs(b)), (2, 2));
    assert_eq!(m.configuration(7), Some(b));
    assert_eq!(m.family_id_set(), vec![0x900]);
    assert_eq!(m.snapshot().filters[&0xF1], Some(vec![a]), "not flushed");
    // Existing, not added: flush and erase, no release.
    assert!(m.add_configuration(b, false));
    assert!(m.configuration(7).is_none());
    assert_eq!(g.refs(b), 2);
    assert_eq!(m.snapshot().filters[&0xF1], None);
    // Slot 7.
    assert!(!m.remove_configuration(7));
    m.add_configuration(a, true);
    assert!(m.remove_configuration(7));
    assert!(m.configuration(7).is_none());
}

/// §B3.2: slot 5 with a key.
#[test]
fn add_by_key() {
    let g = Game::new();
    three_configs(&g);
    let m = manager(&g);
    g.detail.set(2);
    assert!(m.add_by_key(key(0x11)));
    let t = m.configuration(0x11).unwrap();
    assert_eq!(g.refs(t), 1);
    assert_eq!(g.cfg(t).objects.len(), 2, "loaded at the current detail");
    assert!(!m.add_by_key(key(0x55)));
}

/// §B5.2: set queries (quirk 12) and slot 12.
#[test]
fn set_queries() {
    let g = Game::new();
    let a = g.make_cfg(1, &[0x900], (1, 2), vec![]);
    let b = g.make_cfg(2, &[0x900, 0x901], (2, 2), vec![]);
    let c = g.make_cfg(3, &[], (1, 2), vec![]);
    let m = manager(&g);
    for t in [a, b, c] {
        m.add_configuration(t, true);
    }
    assert_eq!(m.configurations_by_family(0x900), vec![a, b]);
    assert_eq!((g.refs(a), g.refs(b), g.refs(c)), (3, 3, 2));
    // The same configuration again takes another reference.
    assert_eq!(m.configurations_by_family(0x901), vec![b]);
    assert_eq!(g.refs(b), 4);
    let by_size = m.configurations_by_size(1, 2);
    assert_eq!(by_size, vec![a, c]);
    m.clear_set(&by_size);
    assert_eq!((g.refs(a), g.refs(c)), (3, 2));
    assert_eq!(m.ids_by_family(0x900), vec![1, 2]);
    assert_eq!(m.ids_by_size(2, 2), vec![2]);
    assert_eq!(m.ids_by_size(3, 3), Vec::<u32>::new());
    assert_eq!(m.ids_by_filter(None), vec![1, 2, 3]);
    g.filters.borrow_mut().insert(0xF1, [3u32].into_iter().collect());
    assert_eq!(m.ids_by_filter(Some(0xF1)), vec![3]);
    assert_eq!(m.configurations_by_filter(None), vec![a, b, c]);
}

/// §B5.4: filters and their caches.
#[test]
fn filters() {
    let g = Game::new();
    let a = g.make_cfg(1, &[], (1, 1), vec![]);
    let b = g.make_cfg(2, &[], (1, 1), vec![]);
    let m = manager(&g);
    m.add_configuration(a, true);
    m.add_configuration(b, true);
    g.filters.borrow_mut().insert(0xF1, [2u32].into_iter().collect());
    // Unregistered: asked every time.
    assert_eq!(m.configurations_by_filter(Some(0xF1)), vec![b]);
    assert_eq!(m.configurations_by_filter(Some(0xF1)), vec![b]);
    assert_eq!(g.filter_calls.get(), 4);
    // Registered: asked once, then the cache.
    m.register_filter(0xF1);
    m.register_filter(0xF1);
    assert_eq!(m.snapshot().filters.len(), 1);
    assert_eq!(m.configurations_by_filter(Some(0xF1)), vec![b]);
    g.filters.borrow_mut().insert(0xF1, [1u32, 2].into_iter().collect());
    assert_eq!(m.configurations_by_filter(Some(0xF1)), vec![b], "cached");
    assert_eq!(g.filter_calls.get(), 6);
    // Slot 13 does not use the cache.
    assert_eq!(m.ids_by_filter(Some(0xF1)), vec![1, 2]);
    // A new configuration flushes.
    let c = g.make_cfg(3, &[], (1, 1), vec![]);
    m.add_configuration(c, true);
    assert_eq!(m.configurations_by_filter(Some(0xF1)), vec![a, b]);
    m.flush();
    assert_eq!(m.snapshot().filters[&0xF1], None);
    m.unregister_filter(0xF1);
    assert!(m.snapshot().filters.is_empty());
}

/// §T 9: GetObjectIDArray (quirk 13).
#[test]
fn t9_object_id_array() {
    let g = Game::new();
    for t in [0x71, 0x72, 0x73] {
        g.building_type(t, 0, None, None);
    }
    g.building_families.borrow_mut().insert(0xF1, vec![0x71, 0x72]);
    for p in [0xA1, 0xA2, 0xA3] {
        g.prop_types.borrow_mut().insert(p, (1.0, 1.0));
    }
    g.prop_families.borrow_mut().insert(0xAF, vec![0xA2, 0xA3]);
    g.flora_types.borrow_mut().insert(0xE1, 1.0);
    g.flora_families.borrow_mut().insert(0xEF, vec![0xE3, 0xE2]);
    let m = manager(&g);
    assert_eq!(m.object_id_array(Some(&record(0, &[0x73, 0xF1]))), vec![0x71, 0x72]);
    assert_eq!(
        m.object_id_array(Some(&record(0, &[0xF1, 0x73]))),
        vec![0x71, 0x72, 0x73]
    );
    assert_eq!(
        m.object_id_array(Some(&record(1, &[0xA1, 0xAF]))),
        vec![0xA1, 0xA2, 0xA3]
    );
    assert_eq!(m.object_id_array(Some(&record(2, &[0xA1]))), Vec::<u32>::new());
    // Flora families in the set's bucket order.
    assert_eq!(
        m.object_id_array(Some(&record(4, &[0xE1, 0xEF]))),
        vec![0xE1, 0xE2, 0xE3]
    );
    assert_eq!(m.object_id_array(None), Vec::<u32>::new());
}

/// §T 10: has building family / type.
#[test]
fn t10_has_building_type() {
    let g = Game::new();
    g.building_type(0x71, 0, None, None);
    g.building_families.borrow_mut().insert(0xF1, vec![0x71]);
    let cfg = g.make_cfg(1, &[], (1, 1), vec![record(1, &[0xF1]), record(0, &[0xF1])]);
    let m = manager(&g);
    assert_eq!(m.has_building_family(Some(cfg), 0xF1), Some(1));
    assert_eq!(m.has_building_family(Some(cfg), 0x71), None);
    assert_eq!(m.has_building_type(Some(cfg), 0x71), Some(1));
    assert_eq!(m.has_building_type(Some(cfg), 0xF1), Some(1));
    assert_eq!(m.has_building_type(Some(cfg), 0), None);
    assert_eq!(m.has_building_family(None, 0xF1), None);
    assert_eq!(m.has_building_type(Some(cfg), 0x72), None);
}

/// §T 11 and the other footprints.
#[test]
fn t11_footprints() {
    let g = Game::new();
    g.prop_types.borrow_mut().insert(0xA2, (3.0, 5.0));
    g.prop_types.borrow_mut().insert(0xA3, (4.0, 2.0));
    g.prop_families.borrow_mut().insert(0xAF, vec![0xA2, 0xA3]);
    let m = manager(&g);
    assert_eq!(m.footprint_props(&[0xAF]), [4.0, 5.0, 0.0, 0.0]);
    assert_eq!(m.footprint_props(&[]), [2.0, 2.0, 0.0, 0.0]);
    assert_eq!(m.footprint_props(&[0x999]), [2.0, 2.0, 0.0, 0.0]);
    // Buildings: start (4, 4); a type without a profile is skipped.
    g.building_type(0x71, 0, Some((20.0, 3.0)), None);
    g.building_type(0x72, 0, Some((f32::NAN, 9.0)), None);
    g.building_type(0x73, 0, None, None);
    g.building_families.borrow_mut().insert(0xF1, vec![0x72, 0x73]);
    assert_eq!(m.footprint_buildings(&[]), [4.0, 4.0, 0.0, 0.0]);
    assert_eq!(m.footprint_buildings(&[0x71]), [20.0, 4.0, 0.0, 0.0]);
    assert_eq!(m.footprint_buildings(&[0x71, 0xF1]), [20.0, 9.0, 0.0, 0.0]);
    assert_eq!(m.footprint_buildings(&[0x73]), [4.0, 4.0, 0.0, 0.0]);
    // Flora: twice the radius.
    g.flora_types.borrow_mut().insert(0xE1, 0.5);
    g.flora_types.borrow_mut().insert(0xE2, 3.0);
    g.flora_families.borrow_mut().insert(0xEF, vec![0xE1, 0xE2]);
    assert_eq!(m.footprint_flora(&[0xE1]), [2.0, 2.0, 0.0, 0.0]);
    assert_eq!(m.footprint_flora(&[0xEF]), [6.0, 6.0, 0.0, 0.0]);
}

fn building_ex(monopoly: Option<bool>, groups: &[u32]) -> PropertyHolder {
    let mut h = PropertyHolder::new();
    if let Some(b) = monopoly {
        h.add(prop::MONOPOLY, Variant::Scalar(Elem::Bool(b)));
    }
    if !groups.is_empty() {
        h.add(
            prop::OCCUPANT_GROUPS,
            Variant::array(mem::UINT32, groups.iter().map(|&x| Elem::U32(x)).collect()),
        );
    }
    h
}

/// §B4.3: the building information.
#[test]
fn post_city_init_building_info() {
    let g = Game::new();
    g.building_type(0x71, 30, None, Some(building_ex(Some(true), &[0x1003, 0x1001])));
    g.building_type(0x72, 10, None, Some(building_ex(Some(true), &[0x1002, 0x1001])));
    g.building_type(0x73, 50, None, Some(building_ex(Some(false), &[])));
    g.building_families.borrow_mut().insert(0xF1, vec![0x71, 0x72]);
    g.building_families.borrow_mut().insert(0xF2, vec![0x73]);
    let a = g.make_cfg(1, &[], (1, 1), vec![record(0, &[0xF1])]);
    let b = g.make_cfg(2, &[], (1, 1), vec![record(0, &[0xF1, 0xF2])]);
    let c = g.make_cfg(3, &[], (1, 1), vec![record(1, &[0xF1])]);
    let d = g.make_cfg(4, &[], (1, 1), vec![record(0, &[0x71])]);
    let m = manager(&g);
    for t in [a, b, c, d] {
        m.add_configuration(t, true);
    }
    assert!(m.do_message(0x1234_5678));
    assert!(g.cfg(a).info.is_none());
    assert!(m.do_message(MSG_POST_CITY_INIT));
    assert_eq!(
        g.cfg(a).info,
        Some(BuildingInfo {
            min_capacity: 10,
            max_capacity: 30,
            groups: vec![0x1001, 0x1002, 0x1003],
            non_monopoly: false,
        })
    );
    assert_eq!(
        g.cfg(b).info,
        Some(BuildingInfo {
            min_capacity: 10,
            max_capacity: 50,
            groups: vec![0x1001, 0x1002, 0x1003],
            non_monopoly: true,
        })
    );
    // No building record; a type listed directly (no family): zeros.
    assert_eq!(g.cfg(c).info, Some(BuildingInfo::default()));
    assert_eq!(g.cfg(d).info, Some(BuildingInfo::default()));
    // An absent monopoly property, or no exemplar (PL7), sets the flag.
    g.building_type(0x74, 5, None, Some(building_ex(None, &[])));
    g.building_type(0x75, 6, None, None);
    for (fam, ty) in [(0xF4, 0x74), (0xF5, 0x75)] {
        g.building_families.borrow_mut().insert(fam, vec![ty]);
        let e = g.make_cfg(fam, &[], (1, 1), vec![record(0, &[fam])]);
        m.fill_building_information(e);
        let info = g.cfg(e).info.unwrap();
        assert!(info.non_monopoly);
        assert_eq!(info.min_capacity, info.max_capacity);
    }
}

/// §B5.7: CreateGenericLotConfiguration (PL8).
#[test]
fn create_generic() {
    let g = Game::new();
    g.building_type(0x71, 0, Some((20.0, 10.0)), None);
    let m = manager(&g);
    assert_eq!(m.create_generic(0x99), None);
    let t = m.create_generic(0x71).unwrap();
    let c = g.cfg(t);
    assert_eq!(c.generic_for, Some(0x71));
    assert_eq!(c.size, (2, 1));
    assert_eq!(g.refs(t), 1);
    assert!(m.configuration_map().is_empty(), "not added to the map");
    let o = &c.objects[0];
    assert_eq!((o.kind, o.ids.clone()), (0, vec![0x71]));
    assert_eq!(o.offset, [16.0, 0.0, 8.0]);
    assert_eq!((o.min_x, o.min_z, o.max_x, o.max_z), (6.0, 3.0, 26.0, 13.0));
    // A type without a profile: the (4, 4) start, one cell.
    g.building_type(0x72, 0, None, None);
    let c = g.cfg(m.create_generic(0x72).unwrap());
    assert_eq!(c.size, (1, 1));
    assert_eq!(c.objects[0].max_x, 10.0);
}

fn placed(k: u32, ids: &[u32], rect: (f32, f32, f32, f32), centre: (f32, f32)) -> LotConfigObject {
    LotConfigObject {
        kind: k as u8,
        ids: ids.to_vec(),
        min_x: rect.0,
        min_z: rect.1,
        max_x: rect.2,
        max_z: rect.3,
        offset: [centre.0, 0.0, centre.1],
        ..LotConfigObject::default()
    }
}

/// §B5.7: FixLotConfiguration's codes (PL9, PL10).
#[test]
fn fix_codes() {
    let g = Game::new();
    g.building_type(0x71, 0, Some((8.0, 8.0)), None);
    g.prop_types.borrow_mut().insert(0xA1, (2.0, 2.0));
    g.flora_types.borrow_mut().insert(0xE1, 1.0);
    g.walls_valid.borrow_mut().insert(0x5);
    let m = manager(&g);
    let good = || {
        vec![
            placed(0, &[0x71], (4.0, 4.0, 12.0, 12.0), (8.0, 8.0)),
            placed(1, &[0xA1], (1.0, 1.0, 3.0, 3.0), (2.0, 2.0)),
            placed(4, &[0xE1], (10.0, 10.0, 12.0, 12.0), (11.0, 11.0)),
            placed(2, &[0x1234], (0.0, 0.0, 16.0, 16.0), (8.0, 8.0)),
        ]
    };
    let cfg = g.make_cfg(1, &[], (1, 1), good());
    assert_eq!(m.fix(Some(cfg)), 0);
    assert_eq!(g.cfg(cfg).objects, good());
    assert_eq!(m.fix(None), 0);
    // 1: an invalid ID removed.
    let mut o = good();
    o[1].ids = vec![0xA1, 0xBAD];
    g.set_config_objects(cfg, o);
    assert_eq!(m.fix(Some(cfg)), 1);
    assert_eq!(g.cfg(cfg).objects, good());
    // 2: a record left with no IDs removed (the first problem is 1).
    let mut o = good();
    o[2].ids = vec![0xBAD];
    g.set_config_objects(cfg, o);
    assert_eq!(m.fix(Some(cfg)), 1);
    assert_eq!(g.cfg(cfg).objects.len(), 3);
    let mut o = good();
    o[2].ids.clear();
    g.set_config_objects(cfg, o);
    assert_eq!(m.fix(Some(cfg)), 2);
    // 3: the footprint recomputed around its centre, swapped for
    // orientation 1.
    let mut o = good();
    o[0].min_x = 5.0;
    o[0].max_x = 11.0;
    g.set_config_objects(cfg, o);
    assert_eq!(m.fix(Some(cfg)), 3);
    assert_eq!(g.cfg(cfg).objects, good());
    g.building_type(0x72, 0, Some((8.0, 4.0)), None);
    let mut o = good();
    o[0] = placed(0, &[0x72], (6.0, 4.0, 10.0, 12.0), (8.0, 8.0));
    o[0].orientation = 1;
    g.set_config_objects(cfg, o.clone());
    assert_eq!(m.fix(Some(cfg)), 0);
    // 4: outside the lot beyond the margin, moved back inside.
    let mut o = good();
    o[1] = placed(1, &[0xA1], (-0.03125, 1.0, 1.96875, 3.0), (1.0, 2.0));
    g.set_config_objects(cfg, o.clone());
    assert_eq!(m.fix(Some(cfg)), 0, "within the margin");
    o[1] = placed(1, &[0xA1], (15.0, 1.0, 17.0, 3.0), (16.0, 2.0));
    g.set_config_objects(cfg, o);
    assert_eq!(m.fix(Some(cfg)), 4);
    let p = &g.cfg(cfg).objects[1];
    assert_eq!((p.min_x, p.max_x, p.offset[0]), (14.0, 16.0, 16.0));
    // 5: a centre outside its footprint.
    let mut o = good();
    o[2].offset = [9.0, 0.0, 13.0];
    g.set_config_objects(cfg, o);
    assert_eq!(m.fix(Some(cfg)), 5);
    assert_eq!(g.cfg(cfg).objects[2].offset, [10.0, 0.0, 12.0]);
    // 6: the first invalid retaining wall ID.
    g.set_config_objects(cfg, good());
    g.set_config_retaining_walls(cfg, vec![0x5, 0x6, 0x7]);
    assert_eq!(m.fix(Some(cfg)), 6);
    assert_eq!(g.cfg(cfg).walls, vec![0x5, 0x7]);
    // The first problem's code wins.
    let mut o = good();
    o[0].min_x = 5.0;
    o[1].ids.push(0xBAD);
    g.set_config_objects(cfg, o);
    assert_eq!(m.fix(Some(cfg)), 3);
}

struct Resolver;
impl RefResolver for Resolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        Some(RefValue::Host {
            token: u64::from(object_id) + 0x50 - 1,
            class_id,
        })
    }
}

/// §T 12: the record.
#[test]
fn t12_record() {
    let g = Game::new();
    let m = manager(&g);
    m.init();
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(reg);
    assert!(m.write(&mut w));
    let bytes = w.into_bytes();
    assert_eq!(
        bytes,
        vec![
            1, 0, 1, 0, 0, 0, 0x7A, 0xBE, 0x90, 0x89, 2, 0, 0, 0, 0x34, 0xC0, 0x90, 0xC9, 3, 0, 0, 0, 0x10, 0xBD, 0x90,
            0xA9
        ]
    );
    assert_eq!(bytes.len(), 26);
    let m2 = manager(&g);
    let mut res = Resolver;
    assert!(m2.read(&mut ByteReader::with_resolver(&bytes, &mut res)));
    let s = m2.snapshot();
    assert_eq!(
        (s.building_development, s.flora, s.prop_manager),
        (Some(BDS), Some(FLORA), Some(PM))
    );
    // Init keeps the loaded fields.
    let before = g.refs(PM);
    m2.init();
    assert_eq!(g.refs(PM), before);
    let mut bad = bytes.clone();
    bad[0] = 2;
    let m3 = manager(&g);
    let mut res = Resolver;
    assert!(!m3.read(&mut ByteReader::with_resolver(&bad, &mut res)));
    let before = (g.refs(BDS), g.refs(FLORA));
    let mut res = Resolver;
    assert!(!m3.read(&mut ByteReader::with_resolver(&bytes[..22], &mut res)));
    assert_eq!(
        (g.refs(BDS), g.refs(FLORA)),
        before,
        "a cut record gives back its references"
    );
    assert_eq!(m3.snapshot().building_development, None);
}

#[test]
fn release_runs_shutdown() {
    let g = Game::new();
    let m = manager(&g);
    m.add_ref();
    m.add_ref();
    m.init();
    assert_eq!(m.release(), 1);
    assert!(m.snapshot().initialized);
    assert_eq!(m.release(), 0);
    let s = m.snapshot();
    assert!(s.destroyed && !s.initialized);
    assert_eq!(g.refs(BDS), 0);
}
