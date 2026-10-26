// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! A fake game and the prop manager tests of prop-lotconfig.md §T (1–6, 12).

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use super::*;
use crate::stream::{ByteReader, ByteWriter, RefResolver, SaveObject, SaveRegistry};
use crate::variant::{mem, Elem, Variant};

const CLASS_OM: u32 = 0x098F_964D;
const CLASS_LM: u32 = 0x498F_9B01;
const OM: u64 = 0x50;
const LM: u64 = 0x51;
const RM: u64 = 0x52;
const VF: u64 = 0x53;
const ME: u64 = 0xAAAA;

#[derive(Clone, Debug, Default)]
struct Occ {
    flags: u32,
    pos: [f32; 3],
    usage: u32,
    prop_type: u32,
    bbox: Option<([f32; 3], [f32; 3])>,
    builtin: Option<BuiltinProp>,
    class_id: u32,
    inits: u32,
    shutdowns: u32,
}

/// Buildings: token → (cell, props property).
type Buildings = BTreeMap<u64, ((i32, i32), BuildingProps)>;

#[derive(Default)]
struct Game {
    log: RefCell<Vec<String>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    exemplars: RefCell<Vec<(ResKey, Rc<PropertyHolder>)>>,
    no_resource_manager: Cell<bool>,
    gzcom_classes: RefCell<Vec<u32>>,
    no_sink: Cell<bool>,
    next: Cell<u64>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    /// Lots: token → cell rect.
    lots: RefCell<BTreeMap<u64, CellRect>>,
    cell_candidates: RefCell<Option<Vec<u64>>>,
    world_positions: RefCell<BTreeMap<(i32, i32), (f32, f32)>>,
    /// Buildings: token → (cell, props property).
    buildings: RefCell<Buildings>,
    services: Cell<bool>,
}

impl Game {
    fn new() -> Rc<Game> {
        let g = Rc::new(Game::default());
        g.next.set(0x1000);
        g.services.set(true);
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
    fn occ(&self, t: u64) -> Occ {
        self.occs.borrow().get(&t).cloned().unwrap_or_default()
    }
    fn add_occ(&self, t: u64, pos: [f32; 3], usage: u32, inserted: bool) {
        self.occs.borrow_mut().insert(
            t,
            Occ {
                pos,
                usage,
                flags: if inserted { FLAG_INSERTED } else { 0 },
                ..Occ::default()
            },
        );
    }
    fn exemplar(&self, instance: u32, props: PropertyHolder) {
        let key = ResKey {
            type_id: crate::props::EXEMPLAR_TYPE,
            group: 0x1234,
            instance,
        };
        self.exemplars.borrow_mut().push((key, Rc::new(props)));
    }
    fn inserted_with_usage(&self, u: u32) -> Vec<u64> {
        self.occs
            .borrow()
            .iter()
            .filter(|(_, o)| o.flags & FLAG_INSERTED != 0 && o.usage == u)
            .map(|(t, _)| *t)
            .collect()
    }
}

fn cell_of(x: f32, z: f32) -> (i32, i32) {
    ((x / 16.0).floor() as i32, (z / 16.0).floor() as i32)
}

impl PropHost for Game {
    fn exemplar_keys(&self) -> Option<Vec<ResKey>> {
        if self.no_resource_manager.get() {
            return None;
        }
        Some(self.exemplars.borrow().iter().map(|(k, _)| *k).collect())
    }
    fn load_exemplar(&self, key: ResKey) -> Option<Rc<PropertyHolder>> {
        self.exemplars
            .borrow()
            .iter()
            .rev()
            .find(|(k, _)| *k == key)
            .map(|(_, h)| h.clone())
    }
    fn register_shutdown_callback(&self) {
        self.log("callback");
    }
    fn post_exemplar_data_updated(&self, manager: u64, data2: u32) {
        self.log(format!("post {manager:X} {data2}"));
    }
    fn register_command(&self, id: u32) {
        self.log(format!("command {id:08X}"));
    }
    fn unregister_command(&self, id: u32) {
        self.log(format!("uncommand {id:08X}"));
    }
    fn subscribe(&self, m: u32) {
        self.log(format!("subscribe {m:08X}"));
    }
    fn unsubscribe(&self, m: u32) {
        self.log(format!("unsubscribe {m:08X}"));
    }
    fn acquire_service(&self, s: PropService) -> Option<u64> {
        if !self.services.get() {
            return None;
        }
        let t = match s {
            PropService::OccupantManager => OM,
            PropService::ResourceManager => RM,
            PropService::LotManager => LM,
        };
        self.bump(t, 1);
        Some(t)
    }
    fn view_filter(&self) -> Option<u64> {
        self.bump(VF, 1);
        Some(VF)
    }
    fn add_ref(&self, t: u64) {
        self.bump(t, 1);
    }
    fn release(&self, t: u64) {
        self.bump(t, -1);
        self.log(format!("release {t:X}"));
    }
    fn create_prop_instance(&self, class_id: u32) -> Option<u64> {
        if !self.gzcom_classes.borrow().contains(&class_id) {
            return None;
        }
        let t = self.new_token();
        self.occs.borrow_mut().insert(
            t,
            Occ {
                class_id,
                ..Occ::default()
            },
        );
        self.bump(t, 1);
        Some(t)
    }
    fn create_builtin_prop(&self, kind: BuiltinProp) -> u64 {
        let t = self.new_token();
        self.occs.borrow_mut().insert(
            t,
            Occ {
                builtin: Some(kind),
                ..Occ::default()
            },
        );
        self.bump(t, 1);
        t
    }
    fn set_prop_type(&self, p: u64, id: u32) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&p) {
            o.prop_type = id;
        }
    }
    fn attach_exemplar(&self, _p: u64, key: ResKey, _global: bool) -> Option<Rc<PropertyHolder>> {
        if self.no_sink.get() {
            return None;
        }
        self.load_exemplar(key)
    }
    fn set_bounding_box(&self, p: u64, min: [f32; 3], max: [f32; 3]) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&p) {
            o.bbox = Some((min, max));
        }
    }
    fn occupant_flag(&self, occ: u64, flag: u32) -> bool {
        self.occ(occ).flags & flag != 0
    }
    fn occupant_init(&self, occ: u64) -> bool {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.inits += 1;
        }
        true
    }
    fn occupant_shutdown(&self, occ: u64) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.shutdowns += 1;
        }
        self.log(format!("shutdown {occ:X}"));
    }
    fn set_prop_usage(&self, occ: u64, u: u32) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.usage = u;
        }
    }
    fn set_position(&self, occ: u64, pos: [f32; 3]) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.pos = pos;
        }
    }
    fn set_orientation(&self, _occ: u64, _o: i32) {}
    fn position(&self, occ: u64) -> [f32; 3] {
        self.occ(occ).pos
    }
    fn hide(&self, occ: u64) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.flags &= !FLAG_SHOWN;
        }
        self.log(format!("hide {occ:X}"));
    }
    fn insert_occupant(&self, occ: u64) -> bool {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.flags |= FLAG_INSERTED;
        }
        self.bump(occ, 1);
        true
    }
    fn remove_occupant(&self, occ: u64) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.flags &= !FLAG_INSERTED;
        }
        self.bump(occ, -1);
        self.log(format!("remove {occ:X}"));
    }
    fn occupants_in_box(&self, min: [f32; 3], max: [f32; 3], u: u32) -> Option<Vec<u64>> {
        let v: Vec<u64> = self
            .inserted_with_usage(u)
            .into_iter()
            .filter(|t| {
                let p = self.occ(*t).pos;
                min[0] <= p[0] && p[0] <= max[0] && min[2] <= p[2] && p[2] <= max[2]
            })
            .collect();
        for t in &v {
            self.bump(*t, 1);
        }
        Some(v)
    }
    fn occupants_in_cell_rect(&self, r: CellRect, u: u32) -> Option<Vec<u64>> {
        if let Some(v) = self.cell_candidates.borrow().as_ref() {
            for t in v {
                self.bump(*t, 1);
            }
            return Some(v.clone());
        }
        // Cells hit by the occupant's position, widened by one cell (a box
        // query also returns neighbours that overlap the rectangle).
        let v: Vec<u64> = self
            .inserted_with_usage(u)
            .into_iter()
            .filter(|t| {
                let p = self.occ(*t).pos;
                let (cx, cz) = cell_of(p[0], p[2]);
                r.0 - 1 <= cx && cx <= r.2 + 1 && r.1 - 1 <= cz && cz <= r.3 + 1
            })
            .collect();
        for t in &v {
            self.bump(*t, 1);
        }
        Some(v)
    }
    fn building_at_cell(&self, x: i32, z: i32) -> Option<u64> {
        let b = self
            .buildings
            .borrow()
            .iter()
            .find(|(_, (c, _))| *c == (x, z))
            .map(|(t, _)| *t)?;
        self.bump(b, 1);
        Some(b)
    }
    fn lot_at_cell(&self, x: i32, z: i32) -> Option<u64> {
        self.lots
            .borrow()
            .iter()
            .find(|(_, r)| r.0 <= x && x <= r.2 && r.1 <= z && z <= r.3)
            .map(|(t, _)| *t)
    }
    fn lot_cell_rect(&self, lot: u64) -> CellRect {
        self.lots.borrow()[&lot]
    }
    fn cell_to_world(&self, x: i32, z: i32) -> (f32, f32) {
        self.world_positions
            .borrow()
            .get(&(x, z))
            .copied()
            .unwrap_or((x as f32 * 16.0, z as f32 * 16.0))
    }
    fn world_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        cell_of(x, z)
    }
    fn building_props(&self, b: u64) -> BuildingProps {
        self.buildings
            .borrow()
            .get(&b)
            .map_or(BuildingProps::NoHolder, |(_, p)| p.clone())
    }
    fn set_building_props(&self, b: u64, props: &[u64]) {
        if let Some((_, p)) = self.buildings.borrow_mut().get_mut(&b) {
            *p = BuildingProps::Present(props.to_vec());
        }
    }
    fn add_building_props_property(&self, b: u64, props: &[u64]) {
        self.log(format!("add property {b:X}"));
        if let Some((_, p)) = self.buildings.borrow_mut().get_mut(&b) {
            *p = BuildingProps::Present(props.to_vec());
        }
    }
    fn remove_building_props_property(&self, b: u64) {
        self.log(format!("remove property {b:X}"));
        if let Some((_, p)) = self.buildings.borrow_mut().get_mut(&b) {
            *p = BuildingProps::Absent;
        }
    }
    fn create_text_decal(&self) -> Option<u64> {
        let t = self.new_token();
        self.occs.borrow_mut().insert(t, Occ::default());
        self.bump(t, 1);
        Some(t)
    }
    fn terrain_altitude(&self, x: f32, z: f32) -> f32 {
        x + z
    }
    fn decal_set_text(&self, d: u64, text: &str) {
        self.log(format!("decal {d:X} text {text}"));
    }
    fn decal_slot8(&self, d: u64, v: [f32; 3]) {
        self.log(format!("decal {d:X} slot8 {} {} {}", v[0], v[1], v[2]));
    }
    fn decal_slot10(&self, d: u64, v: f32) {
        self.log(format!("decal {d:X} slot10 {v}"));
    }
    fn decal_slot6(&self, d: u64, v: f32) {
        self.log(format!("decal {d:X} slot6 {v}"));
    }
    fn object_ref(&self, t: u64) -> RefValue {
        let class_id = match t {
            OM => CLASS_OM,
            LM => CLASS_LM,
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

fn prop_exemplar(families: &[u32], size: Option<&[f32]>) -> PropertyHolder {
    let mut h = PropertyHolder::new();
    h.add(crate::props::PROP_EXEMPLAR_TYPE, Variant::Scalar(Elem::U32(0x1E)));
    if !families.is_empty() {
        h.add(
            prop::FAMILIES,
            Variant::array(mem::UINT32, families.iter().map(|&f| Elem::U32(f)).collect()),
        );
    }
    if let Some(s) = size {
        h.add(
            prop::OCCUPANT_SIZE,
            Variant::array(mem::FLOAT32, s.iter().map(|&f| Elem::F32(f)).collect()),
        );
    }
    h
}

fn manager(g: &Rc<Game>) -> Rc<PropManager> {
    PropManager::new(g.clone(), PropCatalogue::shared(), ME)
}

#[test]
fn identity() {
    for iid in [0x2832_6F3E, 1, 0x4522_94AA, 0xE4FD_A3D4] {
        assert!(query_interface(iid));
    }
    assert!(!query_interface(0xC8ED_BEC0));
    let g = Game::new();
    assert_eq!(manager(&g).class_id(), 0xA990_BD10);
}

#[test]
fn init_builds_once_and_shutdown_keeps_catalogue() {
    let g = Game::new();
    g.exemplar(0x10, prop_exemplar(&[], None));
    let mut other = prop_exemplar(&[], None);
    other.add(crate::props::PROP_EXEMPLAR_TYPE, Variant::Scalar(Elem::U32(0x02)));
    g.exemplar(0x11, other);
    let m = manager(&g);
    assert!(m.init());
    assert_eq!(
        g.take(),
        vec![
            "callback",
            "post AAAA 1",
            "command 0BEAF1A2",
            "subscribe A6B79602",
            "subscribe 06CDB65B",
            "subscribe 26D31EC1",
            "subscribe 0BEAF1A2",
        ]
    );
    assert!(m.is_prop_type_valid(0x10));
    assert!(!m.is_prop_type_valid(0x11), "only ExemplarType 0x1E is catalogued");
    let s = m.snapshot();
    assert_eq!(
        (
            s.occupant_manager,
            s.resource_manager,
            s.lot_manager,
            s.view_filter,
            s.filter
        ),
        (Some(OM), Some(RM), Some(LM), Some(VF), true)
    );
    // A second Init does nothing.
    assert!(m.init());
    assert!(g.take().is_empty());
    assert!(m.shutdown());
    assert_eq!(
        g.take(),
        vec![
            "release 53",
            "release 51",
            "release 52",
            "release 50",
            "unsubscribe A6B79602",
            "unsubscribe 06CDB65B",
            "unsubscribe 26D31EC1",
            "unsubscribe 0BEAF1A2",
            "uncommand 0BEAF1A2",
        ]
    );
    for t in [OM, LM, RM, VF] {
        assert_eq!(g.refs(t), 0);
    }
    // The catalogue stays; a second manager of the process does not rebuild it.
    let cat = m.catalogue();
    let m2 = PropManager::new(g.clone(), cat, ME);
    m2.init();
    assert!(!g.take().iter().any(|l| l == "callback" || l.starts_with("post")));
    assert!(m2.is_prop_type_valid(0x10));
}

#[test]
fn catalogue_message_without_resource_manager() {
    let g = Game::new();
    g.no_resource_manager.set(true);
    g.exemplar(0x10, prop_exemplar(&[], None));
    let m = manager(&g);
    m.init();
    assert!(g.take().contains(&"post AAAA 0".to_string()));
    assert!(!m.is_prop_type_valid(0x10));
}

/// PL2 (re-answers18): data2 of the catalogue message is the type map's
/// count, each instance once even when a later exemplar replaces its key.
#[test]
fn catalogue_message_data2_is_the_type_count() {
    let g = Game::new();
    g.exemplar(0x10, prop_exemplar(&[], None));
    g.exemplar(0x11, prop_exemplar(&[], None));
    g.exemplars.borrow_mut().push((
        ResKey {
            type_id: crate::props::EXEMPLAR_TYPE,
            group: 0x5678,
            instance: 0x10,
        },
        Rc::new(prop_exemplar(&[], None)),
    ));
    let mut other = prop_exemplar(&[], None);
    other.add(crate::props::PROP_EXEMPLAR_TYPE, Variant::Scalar(Elem::U32(0x02)));
    g.exemplar(0x12, other);
    let m = manager(&g);
    m.init();
    assert!(g.take().contains(&"post AAAA 2".to_string()));
    assert_eq!(m.catalogue().borrow().data2(), 2);
}

/// §T 1: profile defaults.
#[test]
fn t1_profile_defaults() {
    let g = Game::new();
    g.exemplar(0x1234, prop_exemplar(&[], None));
    g.exemplar(0x1235, prop_exemplar(&[], Some(&[2.0, 6.0, 3.0])));
    let m = manager(&g);
    m.init();
    assert!(m.is_prop_type_valid(0x1234));
    let p = m.prop_profile_ptr(0x1234).unwrap();
    assert_eq!(p.class_id(), 0x2977_AA47);
    assert_eq!(p.size(), [0.0, 0.0, 0.0]);
    assert_eq!((p.satisfaction(), p.wealth(), p.crime_effect()), (0, 0, 0));
    let t = m.create_prop(0x1234).unwrap();
    let o = g.occ(t);
    assert_eq!(o.builtin, Some(BuiltinProp::Plain));
    assert_eq!(o.prop_type, 0x1234);
    assert_eq!(o.bbox, Some(([0.0; 3], [1.0, 1.0, 1.0])));
    let t = m.create_prop(0x1235).unwrap();
    assert_eq!(g.occ(t).bbox, Some(([0.0; 3], [2.0, 6.0, 3.0])));
    assert_eq!(m.prop_profile_ptr(0x1235).unwrap().size(), [2.0, 6.0, 3.0]);
}

/// §A3.2 quirk 8 and the other profile fields.
#[test]
fn profile_fields_and_unbounded_size() {
    let mut h = prop_exemplar(&[], Some(&[1.0, 2.0, 3.0, 4.0, 5.0]));
    h.add(prop::OCCUPANT_CLASS, Variant::Scalar(Elem::U32(0x2977_AA49)));
    h.add(prop::WEALTH, Variant::Scalar(Elem::U8(2)));
    let p = PropProfile::from_exemplar(&h);
    assert_eq!(p.class_id(), 0x2977_AA49);
    // Value 4 overwrites +0x10, value 5 the dword at +0x14 whose low byte the
    // wealth then replaces.
    assert_eq!(p.satisfaction(), 4.0f32.to_bits());
    assert_eq!(p.word(5), (5.0f32.to_bits() & !0xFF) | 2);
    assert_eq!(p.wealth(), 2);
    let mut h = prop_exemplar(&[], Some(&[1.0, 2.0, 3.0]));
    h.add(prop::REQUESTER_SATISFACTION, Variant::Scalar(Elem::U32(7)));
    h.add(prop::CRIME_EFFECT, Variant::Scalar(Elem::U32(9)));
    let p = PropProfile::from_exemplar(&h);
    assert_eq!((p.satisfaction(), p.crime_effect()), (7, 9));
}

/// §T 2: unknown profile.
#[test]
fn t2_unknown_profile() {
    let g = Game::new();
    let m = manager(&g);
    m.init();
    assert_eq!(m.prop_profile_ptr(0x9999), None);
    assert_eq!(m.prop_profile(0x9999), PropProfile::default());
    assert!(m.prop_profile_ptr(0x9999).is_some());
    assert!(!m.is_prop_type_valid(0x9999));
    assert_eq!(m.create_prop(0x9999), None);
    assert_eq!(m.prop_name(0x9999), "");
}

/// §T 3: families.
#[test]
fn t3_families() {
    const A: u32 = 0xA0;
    const B: u32 = 0xB0;
    const F: u32 = 0xF0;
    const G: u32 = 0xF1;
    let g = Game::new();
    g.exemplar(A, prop_exemplar(&[F, G], None));
    g.exemplar(B, prop_exemplar(&[F], None));
    g.exemplar(A, prop_exemplar(&[F, G], None));
    let m = manager(&g);
    m.init();
    let mut v = vec![7];
    assert_eq!(m.prop_types_by_family(&mut v, F), 2);
    assert_eq!(v, vec![A, B]);
    let mut v = vec![7];
    assert_eq!(m.prop_types_by_family(&mut v, G), 1);
    assert_eq!(v, vec![A]);
    let mut v = vec![7];
    assert_eq!(m.prop_types_by_family(&mut v, A), 0);
    assert_eq!(v, vec![A]);
    let mut v = vec![7];
    assert_eq!(m.prop_types_by_family(&mut v, 0x5555), 0);
    assert_eq!(v, vec![7]);
    assert!(m.is_prop_in_family(A, F));
    assert!(m.is_prop_in_family(A, A));
    assert!(!m.is_prop_in_family(B, G));
    assert!(m.is_prop_family_type_valid(A));
    assert!(m.is_prop_family_type_valid(F));
    assert!(!m.is_prop_family_type_valid(0x5555));
    let mut all = m.all_prop_types();
    all.sort_unstable();
    assert_eq!(all, vec![A, B]);
    let mut fams = m.all_prop_family_types();
    fams.sort_unstable();
    assert_eq!(fams, vec![F, G]);
}

#[test]
fn create_prop_classes() {
    let g = Game::new();
    let mut h = prop_exemplar(&[], None);
    h.add(prop::OCCUPANT_CLASS, Variant::Scalar(Elem::U32(0x2977_AA48)));
    g.exemplar(0x20, h);
    let mut h = prop_exemplar(&[], None);
    h.add(prop::OCCUPANT_CLASS, Variant::Scalar(Elem::U32(0x0000_7777)));
    g.exemplar(0x21, h);
    let m = manager(&g);
    m.init();
    assert_eq!(
        g.occ(m.create_prop(0x20).unwrap()).builtin,
        Some(BuiltinProp::TerrainDecal)
    );
    g.gzcom_classes.borrow_mut().push(0x7777);
    let t = m.create_prop(0x21).unwrap();
    assert_eq!(g.occ(t).class_id, 0x7777);
    assert_eq!(g.refs(t), 1);
    // A missing exemplar interface skips the box.
    g.no_sink.set(true);
    let t = m.create_prop(0x20).unwrap();
    assert_eq!(g.occ(t).bbox, None);
    assert_eq!(g.occ(t).prop_type, 0x20);
    // CreateGenericProp: no prop type, the global resource manager.
    g.no_sink.set(false);
    let key = g.exemplars.borrow()[0].0;
    let t = m.create_generic_prop(key).unwrap();
    assert_eq!(g.occ(t).prop_type, 0);
    assert_eq!(g.occ(t).builtin, Some(BuiltinProp::TerrainDecal));
    assert_eq!(
        m.create_generic_prop(ResKey {
            type_id: 1,
            group: 2,
            instance: 3
        }),
        None
    );
}

#[test]
fn add_props_by_usage() {
    let g = Game::new();
    g.exemplar(0x30, prop_exemplar(&[], None));
    let m = manager(&g);
    // Before Init: no occupant manager.
    g.add_occ(0x200, [0.0; 3], 0, false);
    assert!(!m.add_city_prop(0x200));
    m.init();
    assert!(m.add_city_prop(0x200));
    assert_eq!(g.occ(0x200).usage, usage::CITY);
    // Already inserted: true, usage unchanged (quirk 6).
    assert!(m.add_lot_prop(None, 0x200));
    assert_eq!(g.occ(0x200).usage, usage::CITY);
    let t = m.add_lot_prop_new(0x30, [5.0, 0.0, 6.0], 1).unwrap();
    let o = g.occ(t);
    assert_eq!(
        (o.usage, o.pos, o.flags & FLAG_INSERTED),
        (usage::LOT, [5.0, 0.0, 6.0], 1)
    );
    // The creation reference is gone; the occupant manager holds one.
    assert_eq!(g.refs(t), 1);
    let t = m.add_street_prop(0x30, [1.0, 0.0, 1.0], 0).unwrap();
    assert_eq!(g.occ(t).usage, usage::STREET);
    let t = m.add_city_prop_new(0x30, [1.0, 0.0, 1.0], 0).unwrap();
    assert_eq!(g.occ(t).usage, usage::CITY);
    assert_eq!(m.add_city_prop_new(0x31, [1.0, 0.0, 1.0], 0), None);
}

/// §T 4: GetLotProps boundary.
#[test]
fn t4_get_lot_props_boundary() {
    let g = Game::new();
    let m = manager(&g);
    m.init();
    g.lots.borrow_mut().insert(0x900, (10, 20, 11, 21));
    let inside = [(0x301, 160.0, 320.0), (0x302, 192.0, 352.0)];
    let outside = [(0x303, f32::from_bits(0x4340_0001), 330.0), (0x304, 175.0, 319.9)];
    for (t, x, z) in inside.iter().chain(&outside) {
        g.add_occ(*t, [*x, 0.0, *z], usage::LOT, true);
        g.bump(*t, 1); // the occupant manager's
    }
    g.add_occ(0x305, [0.0, 0.0, 0.0], usage::CITY, true);
    g.bump(0x305, 1); // the caller's
    let mut list = vec![0x305];
    assert!(m.get_lot_props(&mut list, Some(0x900)));
    assert_eq!(list, vec![0x301, 0x302]);
    assert_eq!(g.refs(0x301), 2);
    assert_eq!(g.refs(0x303), 1, "removed and released");
    assert_eq!(g.refs(0x305), 0, "the caller's element is filtered too");
    assert!(!m.get_lot_props(&mut list, None));
    let mut l2 = Vec::new();
    assert!(m.get_lot_props_at(&mut l2, 11, 21));
    assert_eq!(l2, vec![0x301, 0x302]);
    assert!(!m.get_lot_props_at(&mut l2, 50, 50));
    // A NaN coordinate fails.
    g.add_occ(0x306, [f32::NAN, 0.0, 330.0], usage::LOT, true);
    let mut l3 = Vec::new();
    m.get_lot_props(&mut l3, Some(0x900));
    assert!(!l3.contains(&0x306));
}

/// PL24: both upper bounds retain the observed kind-0, flags-7 props.
#[test]
fn pl24_untyped_lot_props_at_observed_upper_bounds() {
    let cases = [
        ((7, 44, 8, 45), [144.0_f32, 285.8455, 723.09265], 0),
        (
            (31, 46, 33, 47),
            [f32::from_bits(0x4401_0094), f32::from_bits(0x438a_b871), 768.0],
            2,
        ),
    ];
    for (rect, boundary, axis) in cases {
        let g = Game::new();
        let m = manager(&g);
        assert!(m.init());
        g.lots.borrow_mut().insert(0x900, rect);
        let mut inside = boundary;
        inside[axis] = f32::from_bits(boundary[axis].to_bits() - 1);
        let mut outside = boundary;
        outside[axis] = f32::from_bits(boundary[axis].to_bits() + 1);
        for (token, position) in [(0x301, inside), (0x302, boundary), (0x303, outside)] {
            g.add_occ(token, position, usage::LOT, true);
            g.occs.borrow_mut().get_mut(&token).unwrap().flags = 7;
            assert_eq!(g.occ(token).prop_type, 0);
            g.bump(token, 1); // the occupant manager's reference
        }
        let mut list = Vec::new();
        assert!(m.get_lot_props_at(&mut list, rect.0, rect.1));
        assert_eq!(list, vec![0x301, 0x302], "rect={rect:?}, axis={axis}");
        assert_eq!(g.refs(0x301), 2);
        assert_eq!(g.refs(0x302), 2, "upper-bound entry retained");
        assert_eq!(g.refs(0x303), 1, "outside entry released");
    }
}

/// §T.4: supplied spatial candidates isolate the binary32 position pass.
#[test]
fn pl24_binary32_lot_boundary_fixtures() {
    let rows = [
        // L, I, X, Z, C, OX, OZ, B, NX, NZ in §T.4 order.
        ([160.0, 0.0, 320.0], true),
        ([f32::from_bits(0x433f_ffff), 0.0, f32::from_bits(0x43af_ffff)], true),
        ([192.0, 0.0, 330.0], true),
        ([175.0, 0.0, 352.0], true),
        ([192.0, 0.0, 352.0], true),
        ([f32::from_bits(0x4340_0001), 0.0, 330.0], false),
        ([175.0, 0.0, f32::from_bits(0x43b0_0001)], false),
        ([175.0, 0.0, 319.9], false),
        ([f32::NAN, 0.0, 330.0], false),
        ([175.0, 0.0, f32::NAN], false),
        // Y is ignored; caller outside.
        ([175.0, f32::NAN, 330.0], true),
        ([0.0, 0.0, 0.0], false),
    ];
    let g = Game::new();
    let m = manager(&g);
    assert!(m.init());
    g.lots.borrow_mut().insert(0x900, (10, 20, 11, 21));
    let mut expected = Vec::new();
    let mut candidates = Vec::new();
    for (i, (position, keep)) in rows.into_iter().enumerate() {
        let token = 0x301 + i as u64;
        g.add_occ(token, position, usage::LOT, true);
        g.bump(token, 1); // occupant manager
        *g.cell_candidates.borrow_mut() = Some(vec![token]);
        let mut list = Vec::new();
        assert!(m.get_lot_props(&mut list, Some(0x900)));
        assert_eq!(list, if keep { vec![token] } else { vec![] }, "row={i}");
        assert_eq!(g.refs(token), if keep { 2 } else { 1 }, "row={i}");
        for t in list {
            g.bump(t, -1); // caller releases its query result
        }
        // The same position pass applies to existing caller entries.
        *g.cell_candidates.borrow_mut() = Some(vec![]);
        g.bump(token, 1);
        let mut list = vec![token];
        assert!(m.get_lot_props(&mut list, Some(0x900)));
        assert_eq!(list, if keep { vec![token] } else { vec![] }, "caller row={i}");
        assert_eq!(g.refs(token), if keep { 2 } else { 1 }, "caller row={i}");
        for t in list {
            g.bump(t, -1);
        }
        candidates.push(token);
        if keep {
            expected.push(token);
        }
    }
    *g.cell_candidates.borrow_mut() = Some(candidates);
    let mut list = Vec::new();
    assert!(m.get_lot_props(&mut list, Some(0x900)));
    assert_eq!(list, expected, "survivors preserve candidate order");
}

#[test]
fn pl24_nan_world_bounds_reject_and_ordered_extremes_survive() {
    let bounds = [160.0, 320.0, 192.0, 352.0];
    for axis in 0..4 {
        let g = Game::new();
        let m = manager(&g);
        assert!(m.init());
        g.lots.borrow_mut().insert(0x900, (10, 20, 11, 21));
        let mut b = bounds;
        b[axis] = f32::NAN;
        g.world_positions.borrow_mut().insert((10, 20), (b[0], b[1]));
        g.world_positions.borrow_mut().insert((12, 22), (b[2], b[3]));
        g.add_occ(0x301, [175.0, 0.0, 330.0], usage::LOT, true);
        g.bump(0x301, 1);
        *g.cell_candidates.borrow_mut() = Some(vec![0x301]);
        let mut list = Vec::new();
        assert!(m.get_lot_props(&mut list, Some(0x900)));
        assert!(list.is_empty(), "NaN bound {axis}");
        assert_eq!(g.refs(0x301), 1, "NaN bound {axis} releases once");
    }
    let g = Game::new();
    let m = manager(&g);
    assert!(m.init());
    g.lots.borrow_mut().insert(0x900, (10, 20, 11, 21));
    g.world_positions
        .borrow_mut()
        .insert((10, 20), (-0.0, f32::NEG_INFINITY));
    g.world_positions.borrow_mut().insert((12, 22), (0.0, f32::INFINITY));
    for (token, position) in [
        (0x301, [0.0, f32::NAN, f32::NEG_INFINITY]),
        (0x302, [-0.0, 0.0, f32::INFINITY]),
    ] {
        g.add_occ(token, position, usage::LOT, true);
        g.bump(token, 1);
    }
    *g.cell_candidates.borrow_mut() = Some(vec![0x301, 0x302]);
    let mut list = Vec::new();
    assert!(m.get_lot_props(&mut list, Some(0x900)));
    assert_eq!(list, vec![0x301, 0x302]);
    assert_eq!((g.refs(0x301), g.refs(0x302)), (2, 2));
}

/// PL21-U/C/R/W: grid membership and caller-list retention are distinct.
#[test]
fn pl21_lot_grid_membership_and_caller_retention() {
    let g = Game::new();
    let m = manager(&g);
    m.init();
    g.lots.borrow_mut().insert(0x900, (10, 20, 11, 21));
    let pos = [165.0, 0.0, 325.0];
    g.add_occ(0x301, pos, usage::LOT, true);
    g.add_occ(0x302, pos, usage::LOT, false);
    g.add_occ(0x303, pos, usage::STREET, true);
    let mut fresh = Vec::new();
    assert!(m.get_lot_props_at(&mut fresh, 10, 20));
    assert_eq!(fresh, vec![0x301]);
    let before = g.refs(0x301);
    // Each caller entry already owns a reference. Only the appended entry
    // acquires another one; the final pass neither filters usage nor dedups.
    for t in [0x302, 0x303] {
        g.bump(t, 1);
    }
    assert!(m.get_lot_props_at(&mut fresh, 10, 20));
    assert_eq!(fresh, vec![0x301, 0x301]);
    assert_eq!(g.refs(0x301), before + 1);
    let mut supplied = vec![0x302, 0x303];
    assert!(m.get_lot_props_at(&mut supplied, 10, 20));
    assert_eq!(supplied, vec![0x302, 0x303, 0x301]);
    assert_eq!(g.refs(0x302), 1);
    assert_eq!(g.refs(0x303), 1);
}

/// PL21-S: an unchanged manager record does not freeze grid membership.
#[test]
fn pl21_record_is_not_a_lot_query_snapshot() {
    let g = Game::new();
    let m = manager(&g);
    m.init();
    g.lots.borrow_mut().insert(0x900, (10, 20, 11, 21));
    g.add_occ(0x301, [165.0, 0.0, 325.0], usage::LOT, true);
    let record = || {
        let mut w = ByteWriter::with_registry(Rc::new(RefCell::new(SaveRegistry::new())));
        assert!(m.write(&mut w));
        w.into_bytes()
    };
    let before = record();
    g.add_occ(0x302, [166.0, 0.0, 326.0], usage::LOT, true);
    let mut list = Vec::new();
    assert!(m.get_lot_props_at(&mut list, 10, 20));
    assert_eq!(list, vec![0x301, 0x302]);
    assert_eq!(record(), before);
}

/// §T 5: street props.
#[test]
fn t5_street_props() {
    let g = Game::new();
    let m = manager(&g);
    m.init();
    g.add_occ(0x310, [5.0 * 16.0 + 3.0, 0.0, 5.0 * 16.0 + 4.0], usage::STREET, true);
    g.add_occ(0x311, [6.0 * 16.0 + 3.0, 0.0, 5.0 * 16.0 + 4.0], usage::STREET, true);
    let mut list = Vec::new();
    assert!(!m.get_street_props_at(&mut list, 5, 5));
    assert_eq!(list, vec![0x310]);
    assert_eq!(g.refs(0x311), 0);
    m.remove_street_props_from_tile(5, 5);
    assert_eq!(g.occ(0x310).flags & FLAG_INSERTED, 0);
    assert_eq!(g.occ(0x310).shutdowns, 1);
    assert_eq!(g.occ(0x311).shutdowns, 0);
}

/// §T 6: building props.
#[test]
fn t6_building_props() {
    let g = Game::new();
    let m = manager(&g);
    m.init();
    g.buildings.borrow_mut().insert(0x400, ((3, 4), BuildingProps::Absent));
    g.add_occ(0x401, [0.0; 3], 0, false);
    g.add_occ(0x402, [0.0; 3], 0, false);
    assert!(m.add_building_prop(0x400, 0x401));
    assert!(m.add_building_prop(0x400, 0x401));
    assert!(m.add_building_prop(0x400, 0x402));
    assert_eq!(g.building_props(0x400), BuildingProps::Present(vec![0x401, 0x402]));
    assert_eq!(g.occ(0x401).usage, usage::BUILDING);
    assert_eq!(g.occ(0x401).inits, 1);
    let mut list = Vec::new();
    assert!(m.get_building_props(&mut list, Some(0x400)));
    assert_eq!(list, vec![0x401, 0x402]);
    assert_eq!(g.refs(0x401), 2);
    let mut l2 = Vec::new();
    assert!(m.get_building_props_at(&mut l2, 3, 4));
    assert_eq!(l2.len(), 2);
    assert_eq!(g.refs(0x400), 0, "the building is released");
    assert!(m.remove_building_prop(Some(0x400), 0x401));
    assert_eq!(g.building_props(0x400), BuildingProps::Present(vec![0x402]));
    assert_eq!(g.occ(0x401).shutdowns, 1);
    assert!(m.remove_building_prop(Some(0x400), 0x402));
    assert_eq!(g.building_props(0x400), BuildingProps::Absent);
    assert!(!m.remove_building_prop(Some(0x400), 0x402));
    let mut l3 = Vec::new();
    assert!(!m.get_building_props(&mut l3, Some(0x400)));
    // Slot 38: removes every prop and the property; no hiding.
    m.add_building_prop(0x400, 0x401);
    g.occs.borrow_mut().get_mut(&0x401).unwrap().flags |= FLAG_SHOWN;
    g.take();
    assert!(m.remove_building_props_at(3, 4));
    let log = g.take();
    assert!(log.contains(&"remove 401".to_string()));
    assert!(log.contains(&"remove property 400".to_string()));
    assert!(!log.iter().any(|l| l.starts_with("hide")));
    assert!(!m.remove_building_props(Some(0x400)));
    // No holder.
    assert!(!m.remove_building_props(Some(0x499)));
}

#[test]
fn remove_one_prop_hides_first() {
    let g = Game::new();
    let m = manager(&g);
    m.init();
    g.add_occ(0x500, [0.0; 3], usage::CITY, true);
    g.occs.borrow_mut().get_mut(&0x500).unwrap().flags |= FLAG_SHOWN;
    g.take();
    assert!(m.remove_prop(0x500));
    assert_eq!(g.take(), vec!["hide 500", "remove 500", "shutdown 500"]);
    // City props by rectangle.
    g.add_occ(0x501, [10.0, 0.0, 10.0], usage::CITY, true);
    g.bump(0x501, 1);
    g.add_occ(0x502, [100.0, 0.0, 100.0], usage::CITY, true);
    g.bump(0x502, 1);
    assert!(m.remove_city_props_world((0.0, 0.0, 50.0, 50.0)));
    assert_eq!(g.occ(0x501).shutdowns, 1);
    assert_eq!(g.refs(0x501), 0);
    assert_eq!(g.occ(0x502).shutdowns, 0);
}

#[test]
fn view_filter_message_and_text_decal() {
    let g = Game::new();
    let m = manager(&g);
    m.init();
    let mut msg = PropMessage::of_type(msg::POST_VIEW_FILTER_CHANGE);
    msg.view_filter = Some(0x77);
    assert!(m.do_message(&msg));
    assert_eq!(m.snapshot().view_filter, Some(0x77));
    assert_eq!((g.refs(VF), g.refs(0x77)), (0, 1));
    assert!(m.do_message(&msg));
    assert_eq!(g.refs(0x77), 1);
    g.take();
    let arg = |t: &str, v: f32| CommandArg {
        text: t.into(),
        value: v,
    };
    let mut cmd = PropMessage::of_type(msg::CREATE_TEXT_DECAL);
    cmd.args = vec![arg("hi", 0.0), arg("", 32.0)];
    assert!(m.do_message(&cmd));
    assert!(g.take().is_empty(), "two arguments are not enough");
    cmd.args.push(arg("", 48.0));
    assert!(m.do_message(&cmd));
    let log = g.take();
    let d = g.next.get() - 1;
    assert_eq!(g.occ(d).pos, [32.0, 80.0, 48.0]);
    assert_eq!(g.occ(d).usage, usage::CITY);
    assert_eq!(g.refs(d), 2, "the handler's reference is never released");
    assert_eq!(
        log,
        vec![
            format!("decal {d:X} text hi"),
            format!("decal {d:X} slot8 1 1 1"),
            format!("decal {d:X} slot10 1"),
            format!("decal {d:X} slot6 1"),
        ]
    );
    cmd.args = (0..8).map(|i| arg("t", i as f32)).collect();
    m.do_message(&cmd);
    let d = g.next.get() - 1;
    assert_eq!(
        g.take(),
        vec![
            "decal ".to_string() + &format!("{d:X}") + " text t",
            format!("decal {d:X} slot8 4 5 6"),
            format!("decal {d:X} slot10 7"),
            format!("decal {d:X} slot6 3"),
        ]
    );
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
        vec![1, 0, 1, 0, 0, 0, 0x4D, 0x96, 0x8F, 0x09, 2, 0, 0, 0, 0x01, 0x9B, 0x8F, 0x49]
    );
    let m2 = manager(&g);
    let mut res = Resolver;
    let mut r = ByteReader::with_resolver(&bytes, &mut res);
    assert!(m2.read(&mut r));
    assert_eq!(m2.snapshot().occupant_manager, Some(OM));
    assert_eq!(m2.snapshot().lot_manager, Some(LM));
    // Init keeps the loaded fields.
    let before = g.refs(OM);
    m2.init();
    assert_eq!(g.refs(OM), before);
    let mut bad = bytes.clone();
    bad[0] = 2;
    let m3 = manager(&g);
    let mut res = Resolver;
    assert!(!m3.read(&mut ByteReader::with_resolver(&bad, &mut res)));
    let mut res = Resolver;
    let before = g.refs(OM);
    assert!(!m3.read(&mut ByteReader::with_resolver(&bytes[..14], &mut res)));
    assert_eq!(g.refs(OM), before, "a cut record gives back its references");
    assert_eq!(m3.snapshot().occupant_manager, None);
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
}
