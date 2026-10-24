// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The lot configuration manager through the C ABI
//! (docs/specs/prop-lotconfig.md Part B): a fake game behind
//! `fc4_lot_config_host`. Configurations load through the core's object
//! reader (`fc4_lot_config_object_array`), as a host would.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::c_void;

use crate::budget::fc4_token_callback;
use crate::lot_config::*;
use crate::lotdev::{fc4_lotdev_cfg_object, fc4_lotdev_cfg_object_callback, fc4_u32s_sink};
use crate::ordinance::fc4_key_visitor;
use crate::props::{fc4_props, fc4_props_create, fc4_props_destroy, fc4_props_set_bools, fc4_props_set_u32s};
use crate::test_streams::{istream, ostream, In, Out};

const BDS: u64 = 0x50;
const FLORA: u64 = 0x51;
const PM: u64 = 0x52;
const EXEMPLAR: u32 = 0x6534_284A;
const LCM_GROUP: u32 = 0xA8FB_D372;
const POST_CITY_INIT: u32 = 0x26D3_1EC1;
const VERSION: u32 = 0x88ED_C789;
const LOT_OBJECT: u32 = 0x88ED_C900;
const MONOPOLY: u32 = 0x2781_283F;
const GROUPS: u32 = 0xAA1D_D396;

#[test]
fn configuration_projection_abi_layout_and_failed_inputs() {
    use fc4_core::lot_config_manager::ConfigurationFields;
    use std::ptr::{null, null_mut};
    assert_eq!(std::mem::size_of::<ConfigurationFields>(), 48);
    assert_eq!(std::mem::align_of::<ConfigurationFields>(), 4);
    unsafe {
        let mut fields = ConfigurationFields::default();
        assert!(!fc4_lot_configuration_project(null(), 3, &mut fields, None, null_mut()));
        let h = fc4_props_create();
        assert!(!fc4_lot_configuration_project(h, 3, &mut fields, None, null_mut()));
        assert!(fc4_props_set_u32s(h, VERSION, &2, 1));
        assert!(!fc4_lot_configuration_project(h, 3, null_mut(), None, null_mut()));
        assert!(fc4_lot_configuration_project(h, 3, &mut fields, None, null_mut()));
        assert_eq!((fields.growth_stage, fields.roads, fields.max_slope), (1, 8, 90.0));
        fc4_props_destroy(h);
    }
}

#[test]
fn configuration_capacity_selects_developer_entry_and_stores_u16() {
    unsafe {
        assert_eq!(fc4_lot_configuration_capacity(std::ptr::null()), 0);
        let h = fc4_props_create();
        assert_eq!(fc4_lot_configuration_capacity(h), 0);
        assert!(fc4_props_set_u32s(h, 0x2781_2833, &1, 1)); // residential
        assert!(fc4_props_set_u32s(h, 0x2781_2832, &2, 1)); // medium wealth
        let capacities = [0x1010, 17, 0x1020, 0x1_8001, 0x1030, 29];
        assert!(fc4_props_set_u32s(
            h,
            0x2781_2834,
            capacities.as_ptr(),
            capacities.len()
        ));
        assert_eq!(fc4_lot_configuration_capacity(h), 0x8001);
        assert!(fc4_props_set_u32s(h, 0x2781_2832, &3, 1));
        assert_eq!(fc4_lot_configuration_capacity(h), 29);
        assert!(fc4_props_set_u32s(h, 0x2781_2833, &3, 1)); // no matching entry
        assert_eq!(fc4_lot_configuration_capacity(h), 0);
        fc4_props_destroy(h);
    }
}

/// One object as the host keeps it.
#[derive(Clone, Debug, Default, PartialEq)]
struct Obj {
    kind: u32,
    flags: u8,
    orientation: u8,
    offset: [f32; 3],
    max: [f32; 2],
    min: [f32; 2],
    tag: u32,
    ids: Vec<u32>,
}

impl Obj {
    unsafe fn from_c(o: &fc4_lotdev_cfg_object) -> Obj {
        Obj {
            kind: o.object_type,
            flags: o.flags,
            orientation: o.orientation,
            offset: o.offset,
            max: o.footprint_max,
            min: o.footprint_min,
            tag: o.tag,
            ids: if o.n_ids == 0 {
                Vec::new()
            } else {
                std::slice::from_raw_parts(o.ids, o.n_ids as usize).to_vec()
            },
        }
    }
    fn to_c(&self) -> fc4_lotdev_cfg_object {
        fc4_lotdev_cfg_object {
            object_type: self.kind,
            flags: self.flags,
            orientation: self.orientation,
            byte3: 0,
            offset: self.offset,
            footprint_max: self.max,
            footprint_min: self.min,
            foundation: 0,
            tag: self.tag,
            n_ids: self.ids.len() as u32,
            ids: self.ids.as_ptr(),
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Cfg {
    id: u32,
    families: Vec<u32>,
    size: (u32, u32),
    objects: Vec<Obj>,
    walls: Vec<u32>,
    info: Option<(u16, u16, Vec<u32>, bool)>,
    generic_for: Option<u32>,
}

/// A lot configuration exemplar: families, size and objects' values.
#[derive(Clone, Default)]
struct LotEx {
    families: Vec<u32>,
    size: (u32, u32),
    objects: Vec<Vec<u32>>,
}

#[derive(Clone, Default)]
struct Building {
    capacity: u16,
    footprint: (f32, f32),
    groups: Vec<u32>,
    monopoly: bool,
}

#[derive(Default)]
struct Game {
    log: RefCell<Vec<String>>,
    refs: RefCell<BTreeMap<u64, i32>>,
    detail: Cell<u32>,
    exemplars: RefCell<BTreeMap<(u32, u32), LotEx>>,
    next: Cell<u64>,
    configs: RefCell<BTreeMap<u64, Cfg>>,
    buildings: RefCell<BTreeMap<u32, Building>>,
    building_families: RefCell<BTreeMap<u32, Vec<u32>>>,
    flora: RefCell<BTreeMap<u32, f32>>,
    flora_families: RefCell<BTreeMap<u32, Vec<u32>>>,
    props: RefCell<BTreeMap<u32, (f32, f32)>>,
    prop_families: RefCell<BTreeMap<u32, Vec<u32>>>,
    filters: RefCell<BTreeMap<u64, Vec<u32>>>,
    filter_calls: Cell<u32>,
    valid_walls: RefCell<Vec<u32>>,
}

impl Game {
    fn new() -> Box<Game> {
        let g = Box::new(Game::default());
        g.next.set(0x1000);
        g.detail.set(3);
        g
    }
    fn user(&self) -> *mut c_void {
        self as *const Game as *mut c_void
    }
    fn log(&self, s: impl Into<String>) {
        self.log.borrow_mut().push(s.into());
    }
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
    fn refs(&self, t: u64) -> i32 {
        self.refs.borrow().get(&t).copied().unwrap_or(0)
    }
    fn fresh(&self, c: Cfg) -> u64 {
        let t = self.next.get();
        self.next.set(t + 0x10);
        self.configs.borrow_mut().insert(t, c);
        *self.refs.borrow_mut().entry(t).or_default() += 1;
        t
    }
    fn cfg(&self, t: u64) -> Cfg {
        self.configs.borrow()[&t].clone()
    }
    fn lot_exemplar(&self, group: u32, instance: u32, ex: LotEx) {
        self.exemplars.borrow_mut().insert((group, instance), ex);
    }
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

unsafe fn send(f: fc4_u32s_sink, ctx: *mut c_void, v: &[u32]) {
    f.unwrap()(ctx, v.as_ptr(), v.len());
}

unsafe extern "C" fn detail_level(u: *mut c_void) -> u32 {
    game(u).detail.get()
}
unsafe extern "C" fn for_each_exemplar_key(u: *mut c_void, visit: fc4_key_visitor, ctx: *mut c_void) -> bool {
    for &(g, i) in game(u).exemplars.borrow().keys() {
        visit.unwrap()(ctx, EXEMPLAR, g, i);
    }
    true
}
unsafe extern "C" fn register_shutdown_callback(u: *mut c_void) {
    game(u).log("register callback");
}
unsafe extern "C" fn unregister_shutdown_callback(u: *mut c_void) {
    game(u).log("unregister callback");
}
unsafe extern "C" fn create_configuration(u: *mut c_void) -> u64 {
    game(u).fresh(Cfg::default())
}
unsafe extern "C" fn push_obj(ctx: *mut c_void, o: *const fc4_lotdev_cfg_object) {
    (*(ctx as *mut Vec<Obj>)).push(Obj::from_c(&*o));
}
/// Configuration slot 31 `Load`: the exemplar's properties through the
/// core's object reader.
unsafe extern "C" fn load_configuration(u: *mut c_void, cfg: u64, _t: u32, g: u32, i: u32, detail: u32) -> bool {
    let Some(ex) = game(u).exemplars.borrow().get(&(g, i)).cloned() else {
        return false;
    };
    let p = fc4_props_create();
    let v2 = 2u32;
    fc4_props_set_u32s(p, VERSION, &v2, 1);
    for (n, values) in ex.objects.iter().enumerate() {
        fc4_props_set_u32s(p, LOT_OBJECT + n as u32, values.as_ptr(), values.len());
    }
    let mut objects: Vec<Obj> = Vec::new();
    let mut serial = 0u32;
    fc4_lot_config_object_array(
        p,
        detail,
        &mut serial,
        Some(push_obj),
        &mut objects as *mut Vec<Obj> as *mut c_void,
    );
    fc4_props_destroy(p);
    game(u).configs.borrow_mut().insert(
        cfg,
        Cfg {
            id: i,
            families: ex.families,
            size: ex.size,
            objects,
            ..Cfg::default()
        },
    );
    true
}
unsafe extern "C" fn config_id(u: *mut c_void, c: u64) -> u32 {
    game(u).cfg(c).id
}
unsafe extern "C" fn config_belongs_to_family(u: *mut c_void, c: u64, f: u32) -> bool {
    game(u).cfg(c).families.contains(&f)
}
unsafe extern "C" fn config_families(u: *mut c_void, c: u64, f: fc4_u32s_sink, ctx: *mut c_void) {
    send(f, ctx, &game(u).cfg(c).families);
}
unsafe extern "C" fn config_size(u: *mut c_void, c: u64, x: *mut u32, z: *mut u32) {
    let s = game(u).cfg(c).size;
    *x = s.0;
    *z = s.1;
}
unsafe extern "C" fn config_objects(u: *mut c_void, c: u64, f: fc4_lotdev_cfg_object_callback, ctx: *mut c_void) {
    for o in game(u).cfg(c).objects {
        f.unwrap()(ctx, &o.to_c());
    }
}
unsafe extern "C" fn set_config_objects(u: *mut c_void, c: u64, o: *const fc4_lotdev_cfg_object, n: usize) {
    let v = (0..n).map(|i| Obj::from_c(&*o.add(i))).collect();
    game(u).configs.borrow_mut().get_mut(&c).unwrap().objects = v;
}
unsafe extern "C" fn config_primary_building(
    u: *mut c_void,
    c: u64,
    f: fc4_lotdev_cfg_object_callback,
    ctx: *mut c_void,
) -> bool {
    match game(u).cfg(c).objects.iter().find(|o| o.kind == 0) {
        Some(o) => {
            f.unwrap()(ctx, &o.to_c());
            true
        }
        None => false,
    }
}
unsafe extern "C" fn set_config_building_info(u: *mut c_void, c: u64, i: *const fc4_lot_config_building_info) {
    let i = &*i;
    let groups = if i.n_groups == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(i.groups, i.n_groups as usize).to_vec()
    };
    game(u).configs.borrow_mut().get_mut(&c).unwrap().info =
        Some((i.min_capacity, i.max_capacity, groups, i.non_monopoly != 0));
}
unsafe extern "C" fn config_retaining_walls(u: *mut c_void, c: u64, f: fc4_u32s_sink, ctx: *mut c_void) {
    send(f, ctx, &game(u).cfg(c).walls);
}
unsafe extern "C" fn set_config_retaining_walls(u: *mut c_void, c: u64, ids: *const u32, n: usize) {
    let v = if n == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(ids, n).to_vec()
    };
    game(u).configs.borrow_mut().get_mut(&c).unwrap().walls = v;
}
unsafe extern "C" fn is_retaining_wall_valid(u: *mut c_void, id: u32) -> bool {
    game(u).valid_walls.borrow().contains(&id)
}
unsafe extern "C" fn create_generic_configuration(
    u: *mut c_void,
    ty: u32,
    sx: u32,
    sz: u32,
    o: *const fc4_lotdev_cfg_object,
    n: usize,
) -> u64 {
    let objects = (0..n).map(|i| Obj::from_c(&*o.add(i))).collect();
    game(u).fresh(Cfg {
        size: (sx, sz),
        objects,
        generic_for: Some(ty),
        ..Cfg::default()
    })
}
unsafe extern "C" fn acquire_service(u: *mut c_void, s: u32) -> u64 {
    let t = match s {
        FC4_LCM_SERVICE_BUILDING_DEVELOPMENT => BDS,
        FC4_LCM_SERVICE_FLORA => FLORA,
        FC4_LCM_SERVICE_PROP_MANAGER => PM,
        _ => return 0,
    };
    add_ref(u, t);
    t
}
unsafe extern "C" fn subscribe(u: *mut c_void, m: u32) {
    game(u).log(format!("subscribe {m:08X}"));
}
unsafe extern "C" fn unsubscribe(u: *mut c_void, m: u32) {
    game(u).log(format!("unsubscribe {m:08X}"));
}
unsafe extern "C" fn add_ref(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_default() += 1;
}
unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_default() -= 1;
}
unsafe extern "C" fn filter_matches(u: *mut c_void, f: u64, c: u64) -> bool {
    let g = game(u);
    g.filter_calls.set(g.filter_calls.get() + 1);
    let id = g.cfg(c).id;
    g.filters.borrow().get(&f).is_some_and(|ids| ids.contains(&id))
}
unsafe extern "C" fn building_exemplar(u: *mut c_void, ty: u32) -> *mut fc4_props {
    let Some(b) = game(u).buildings.borrow().get(&ty).cloned() else {
        return std::ptr::null_mut();
    };
    let p = fc4_props_create();
    fc4_props_set_u32s(p, GROUPS, b.groups.as_ptr(), b.groups.len());
    fc4_props_set_bools(p, MONOPOLY, &b.monopoly, 1);
    p
}
unsafe extern "C" fn is_building_type_valid(u: *mut c_void, id: u32) -> bool {
    game(u).buildings.borrow().contains_key(&id)
}
unsafe extern "C" fn is_building_family_type_valid(u: *mut c_void, id: u32) -> bool {
    is_building_type_valid(u, id) || game(u).building_families.borrow().contains_key(&id)
}
unsafe extern "C" fn building_types_by_family(u: *mut c_void, id: u32, f: fc4_u32s_sink, ctx: *mut c_void) -> u32 {
    if let Some(m) = game(u).building_families.borrow().get(&id) {
        send(f, ctx, m);
        return m.len() as u32;
    }
    if is_building_type_valid(u, id) {
        send(f, ctx, &[id]);
    }
    0
}
unsafe extern "C" fn building_types_into_set(u: *mut c_void, id: u32, f: fc4_u32s_sink, ctx: *mut c_void) -> u32 {
    let v = match game(u).building_families.borrow().get(&id) {
        Some(m) => m.clone(),
        None if is_building_type_valid(u, id) => vec![id],
        None => Vec::new(),
    };
    send(f, ctx, &v);
    v.len() as u32
}
unsafe extern "C" fn is_building_in_family(u: *mut c_void, ty: u32, family: u32) -> bool {
    game(u)
        .building_families
        .borrow()
        .get(&family)
        .is_some_and(|m| m.contains(&ty))
}
unsafe extern "C" fn building_capacity(u: *mut c_void, ty: u32) -> u16 {
    game(u).buildings.borrow().get(&ty).map_or(0, |b| b.capacity)
}
unsafe extern "C" fn building_footprint(u: *mut c_void, ty: u32, w: *mut f32, d: *mut f32) -> bool {
    match game(u).buildings.borrow().get(&ty) {
        Some(b) => {
            *w = b.footprint.0;
            *d = b.footprint.1;
            true
        }
        None => false,
    }
}
unsafe extern "C" fn flora_radius(u: *mut c_void, id: u32, r: *mut f32) -> bool {
    match game(u).flora.borrow().get(&id) {
        Some(&v) => {
            *r = v;
            true
        }
        None => false,
    }
}
unsafe extern "C" fn is_flora_type_valid(u: *mut c_void, id: u32) -> bool {
    game(u).flora.borrow().contains_key(&id)
}
unsafe extern "C" fn flora_types_into_set(u: *mut c_void, id: u32, f: fc4_u32s_sink, ctx: *mut c_void) -> u32 {
    let v = game(u).flora_families.borrow().get(&id).cloned().unwrap_or_default();
    send(f, ctx, &v);
    v.len() as u32
}
unsafe extern "C" fn is_prop_type_valid(u: *mut c_void, id: u32) -> bool {
    game(u).props.borrow().contains_key(&id)
}
unsafe extern "C" fn prop_types_by_family(u: *mut c_void, id: u32, f: fc4_u32s_sink, ctx: *mut c_void) -> u32 {
    if let Some(m) = game(u).prop_families.borrow().get(&id) {
        send(f, ctx, m);
        return m.len() as u32;
    }
    if is_prop_type_valid(u, id) {
        send(f, ctx, &[id]);
    }
    0
}
unsafe extern "C" fn is_prop_family_type_valid(u: *mut c_void, id: u32) -> bool {
    is_prop_type_valid(u, id) || game(u).prop_families.borrow().contains_key(&id)
}
unsafe extern "C" fn prop_size(u: *mut c_void, id: u32, x: *mut f32, z: *mut f32) -> bool {
    match game(u).props.borrow().get(&id) {
        Some(&(a, b)) => {
            *x = a;
            *z = b;
            true
        }
        None => false,
    }
}
unsafe extern "C" fn resolve_ref(u: *mut c_void, t: u64, iid: u32) -> u64 {
    game(u).log(format!("resolve {t:X} {iid:08X}"));
    t
}

fn host(g: &Game) -> fc4_lot_config_host {
    fc4_lot_config_host {
        struct_size: std::mem::size_of::<fc4_lot_config_host>() as u32,
        user: g.user(),
        detail_level: Some(detail_level),
        for_each_exemplar_key: Some(for_each_exemplar_key),
        register_shutdown_callback: Some(register_shutdown_callback),
        unregister_shutdown_callback: Some(unregister_shutdown_callback),
        create_configuration: Some(create_configuration),
        load_configuration: Some(load_configuration),
        config_id: Some(config_id),
        config_belongs_to_family: Some(config_belongs_to_family),
        config_families: Some(config_families),
        config_size: Some(config_size),
        config_objects: Some(config_objects),
        set_config_objects: Some(set_config_objects),
        config_primary_building: Some(config_primary_building),
        set_config_building_info: Some(set_config_building_info),
        config_retaining_walls: Some(config_retaining_walls),
        set_config_retaining_walls: Some(set_config_retaining_walls),
        is_retaining_wall_valid: Some(is_retaining_wall_valid),
        create_generic_configuration: Some(create_generic_configuration),
        acquire_service: Some(acquire_service),
        subscribe: Some(subscribe),
        unsubscribe: Some(unsubscribe),
        add_ref: Some(add_ref),
        release: Some(release),
        filter_matches: Some(filter_matches),
        building_exemplar: Some(building_exemplar),
        is_building_type_valid: Some(is_building_type_valid),
        is_building_family_type_valid: Some(is_building_family_type_valid),
        building_types_by_family: Some(building_types_by_family),
        building_types_into_set: Some(building_types_into_set),
        is_building_in_family: Some(is_building_in_family),
        building_capacity: Some(building_capacity),
        building_footprint: Some(building_footprint),
        flora_radius: Some(flora_radius),
        is_flora_type_valid: Some(is_flora_type_valid),
        flora_types_into_set: Some(flora_types_into_set),
        is_prop_type_valid: Some(is_prop_type_valid),
        prop_types_by_family: Some(prop_types_by_family),
        is_prop_family_type_valid: Some(is_prop_family_type_valid),
        prop_size: Some(prop_size),
        resolve_ref: Some(resolve_ref),
    }
}

/// 16.16 of a whole number of metres.
fn fx(m: i32) -> u32 {
    (m << 16) as u32
}

/// An object's values: type, v1 (LOD in bits 4–5), orientation, offset,
/// min, max, foundation, tag, IDs.
fn obj(kind: u32, lod: u32, ids: &[u32]) -> Vec<u32> {
    let mut v = vec![kind, lod << 4, 0, fx(8), 0, fx(8), fx(4), fx(4), fx(12), fx(12), 0, 7];
    v.extend_from_slice(ids);
    v
}

/// Building type 0x500 (family 0xB0 with 0x501), props, flora, and three
/// lot configurations of group 0xA8FBD372 plus one of another group.
fn city() -> Box<Game> {
    let g = Game::new();
    g.buildings.borrow_mut().insert(
        0x500,
        Building {
            capacity: 10,
            footprint: (20.0, 12.0),
            groups: vec![0x1300, 0x1001],
            monopoly: true,
        },
    );
    g.buildings.borrow_mut().insert(
        0x501,
        Building {
            capacity: 30,
            footprint: (8.0, 30.0),
            groups: vec![0x1001],
            monopoly: false,
        },
    );
    g.building_families.borrow_mut().insert(0xB0, vec![0x500, 0x501]);
    g.props.borrow_mut().insert(0x600, (3.0, 1.0));
    g.prop_families.borrow_mut().insert(0xC0, vec![0x600]);
    g.flora.borrow_mut().insert(0x700, 2.5);
    g.flora_families.borrow_mut().insert(0xD0, vec![0x700]);
    g.lot_exemplar(
        LCM_GROUP,
        0x10,
        LotEx {
            families: vec![0xF1],
            size: (2, 2),
            objects: vec![
                obj(0, 3, &[0xB0]),
                obj(1, 0, &[0xC0]),
                obj(1, 3, &[0x600]),
                obj(4, 2, &[0x700]),
            ],
        },
    );
    g.lot_exemplar(
        LCM_GROUP,
        0x11,
        LotEx {
            families: vec![0xF1, 0xF2],
            size: (1, 1),
            objects: vec![obj(0, 0, &[0x500])],
        },
    );
    g.lot_exemplar(
        LCM_GROUP,
        0x12,
        LotEx {
            families: vec![],
            size: (2, 2),
            objects: vec![obj(1, 1, &[0x600])],
        },
    );
    g.lot_exemplar(
        0x1234,
        0x13,
        LotEx {
            families: vec![0xF9],
            size: (1, 1),
            objects: vec![],
        },
    );
    g
}

fn ready(g: &Game, cat: *const fc4_lot_config_catalogue) -> *mut fc4_lot_config_manager {
    let m = unsafe { fc4_lot_config_create(&host(g), cat, 0x900) };
    assert!(!m.is_null());
    assert!(unsafe { fc4_lot_config_init(m) });
    m
}

fn state(m: *const fc4_lot_config_manager) -> fc4_lot_config_state {
    let mut s = fc4_lot_config_state::default();
    assert!(unsafe { fc4_lot_config_get_state(m, &mut s) });
    s
}

fn cat_state(c: *const fc4_lot_config_catalogue) -> fc4_lot_config_catalogue_state {
    let mut s = fc4_lot_config_catalogue_state::default();
    assert!(unsafe { fc4_lot_config_catalogue_get_state(c, &mut s) });
    s
}

fn u32s(f: impl Fn(*mut u32, usize) -> usize) -> Vec<u32> {
    let n = f(std::ptr::null_mut(), 0);
    let mut v = vec![0u32; n];
    assert_eq!(f(v.as_mut_ptr(), n), n);
    v
}

fn sorted(mut v: Vec<u32>) -> Vec<u32> {
    v.sort_unstable();
    v
}

unsafe extern "C" fn push_token(ctx: *mut c_void, t: u64) {
    (*(ctx as *mut Vec<u64>)).push(t);
}

fn tokens(f: impl FnOnce(fc4_token_callback, *mut c_void) -> u32) -> Vec<u64> {
    let mut v: Vec<u64> = Vec::new();
    let n = f(Some(push_token), &mut v as *mut Vec<u64> as *mut c_void);
    assert_eq!(n as usize, v.len());
    v
}

fn configuration(m: *const fc4_lot_config_manager, id: u32) -> u64 {
    unsafe { fc4_lot_config_configuration(m, id) }
}

#[test]
fn create_rejects_a_short_table() {
    let g = Game::new();
    let mut h = host(&g);
    h.struct_size -= 1;
    unsafe {
        assert!(fc4_lot_config_create(&h, std::ptr::null(), 0).is_null());
        assert!(fc4_lot_config_create(std::ptr::null(), std::ptr::null(), 0).is_null());
        assert!(fc4_lot_config_query_interface(0xC8ED_BEC0));
        assert!(!fc4_lot_config_query_interface(0x4522_94AA));
        assert_eq!(fc4_lot_config_fix(std::ptr::null_mut(), 1), 0);
        fc4_lot_config_destroy(std::ptr::null_mut());
        fc4_lot_config_catalogue_destroy(std::ptr::null_mut());
        assert_eq!(
            fc4_lot_config_object_array(std::ptr::null(), 3, std::ptr::null_mut(), None, std::ptr::null_mut()),
            0
        );
    }
}

#[test]
fn the_catalogue_and_its_rebuild_on_a_detail_change() {
    let g = city();
    let cat = fc4_lot_config_catalogue_create();
    let m = ready(&g, cat);
    assert_eq!(g.take(), ["register callback", "subscribe 26D31EC1"]);
    let c = cat_state(cat);
    assert_eq!((c.loaded, c.detail, c.configurations, c.families), (1, 3, 3, 2));
    let s = state(m);
    assert_eq!((s.building_development, s.flora, s.prop_manager), (BDS, FLORA, PM));
    unsafe {
        let ids = u32s(|o, n| fc4_lot_config_configuration_map(m, o, std::ptr::null_mut(), n));
        assert_eq!(sorted(ids), [0x10, 0x11, 0x12]);
        let fams = u32s(|o, n| fc4_lot_config_family_id_set(m, o, n));
        assert_eq!(sorted(fams), [0xF1, 0xF2]);
        // The detail filter (§B3.3): at detail 3 the LOD-3 prop is dropped,
        // the building always kept.
        let c10 = configuration(m, 0x10);
        assert_eq!(g.refs(c10), 1);
        let kinds: Vec<u32> = g.cfg(c10).objects.iter().map(|o| o.kind).collect();
        assert_eq!(kinds, [0, 1, 4]);
        let o = &g.cfg(c10).objects[0];
        assert_eq!(
            (o.min, o.max, o.offset, o.tag),
            ([4.0, 4.0], [12.0, 12.0], [8.0, 0.0, 8.0], 7)
        );
        // A second manager at the same detail keeps the catalogue.
        let m2 = ready(&g, cat);
        assert_eq!(configuration(m2, 0x10), c10);
        // At detail 1: the catalogue is unloaded (references released) and
        // rebuilt; the family set stays (quirk 10).
        g.detail.set(1);
        fc4_lot_config_shutdown(m2);
        let m3 = ready(&g, cat);
        assert_eq!(g.refs(c10), 0);
        let n10 = configuration(m3, 0x10);
        assert_ne!(n10, c10);
        let kinds: Vec<u32> = g.cfg(n10).objects.iter().map(|o| o.kind).collect();
        assert_eq!(kinds, [0, 1]);
        assert_eq!(cat_state(cat).detail, 1);
        // The unload callback.
        g.take();
        fc4_lot_config_unload(m3);
        assert_eq!(g.take(), ["unregister callback"]);
        let c = cat_state(cat);
        assert_eq!((c.loaded, c.configurations, c.families), (0, 0, 2));
        for x in [m, m2, m3] {
            fc4_lot_config_destroy(x);
        }
        fc4_lot_config_catalogue_destroy(cat);
    }
}

#[test]
fn the_add_remove_table_and_set_queries() {
    let g = city();
    let m = ready(&g, std::ptr::null());
    unsafe {
        // A configuration of a new ID is added with a reference.
        let mine = g.fresh(Cfg {
            id: 0x20,
            families: vec![0xF3],
            size: (2, 2),
            ..Cfg::default()
        });
        assert!(fc4_lot_config_add_configuration(m, mine, true));
        assert_eq!((configuration(m, 0x20), g.refs(mine)), (mine, 2));
        // Removing does not release it (quirk).
        assert!(fc4_lot_config_remove_configuration(m, 0x20));
        assert_eq!((configuration(m, 0x20), g.refs(mine)), (0, 2));
        assert!(!fc4_lot_config_remove_configuration(m, 0x20));

        // By family / size: tokens with references; IDs.
        let v = tokens(|f, c| fc4_lot_config_configurations(m, 1, 0xF1, 0, f, c));
        assert_eq!(v.len(), 2);
        let c11 = configuration(m, 0x11);
        assert!(v.contains(&c11));
        assert_eq!(g.refs(c11), 2);
        fc4_lot_config_clear_set(m, v.as_ptr(), v.len());
        assert_eq!(g.refs(c11), 1);
        let ids = u32s(|o, n| fc4_lot_config_ids(m, 2, 2, 2, o, n));
        assert_eq!(sorted(ids), [0x10, 0x12]);
        let ids = u32s(|o, n| fc4_lot_config_ids(m, 0, 0, 0, o, n));
        assert_eq!(ids.len(), 3);

        // A registered filter caches its result until a flush.
        g.filters.borrow_mut().insert(0x77, vec![0x11, 0x12]);
        fc4_lot_config_register_filter(m, 0x77);
        assert_eq!(state(m).filters, 1);
        let (mut cached, mut n) = (true, 0usize);
        assert!(fc4_lot_config_filter_cache(
            m,
            0x77,
            &mut cached,
            std::ptr::null_mut(),
            0,
            &mut n
        ));
        assert!(!cached);
        let v = tokens(|f, c| fc4_lot_config_configurations(m, 0, 0x77, 0, f, c));
        assert_eq!(v.len(), 2);
        let calls = g.filter_calls.get();
        let again = tokens(|f, c| fc4_lot_config_configurations(m, 0, 0x77, 0, f, c));
        assert_eq!((again, g.filter_calls.get()), (v.clone(), calls));
        assert!(fc4_lot_config_filter_cache(
            m,
            0x77,
            &mut cached,
            std::ptr::null_mut(),
            0,
            &mut n
        ));
        assert_eq!((cached, n), (true, 2));
        fc4_lot_config_flush(m);
        assert!(fc4_lot_config_filter_cache(
            m,
            0x77,
            &mut cached,
            std::ptr::null_mut(),
            0,
            &mut n
        ));
        assert!(!cached);
        fc4_lot_config_unregister_filter(m, 0x77);
        assert!(!fc4_lot_config_filter_cache(
            m,
            0x77,
            &mut cached,
            std::ptr::null_mut(),
            0,
            &mut n
        ));
        fc4_lot_config_destroy(m);
    }
}

#[test]
fn building_tests_footprints_and_object_ids() {
    let g = city();
    let m = ready(&g, std::ptr::null());
    unsafe {
        let c10 = configuration(m, 0x10);
        let mut i = 99u32;
        assert!(fc4_lot_config_has_building(m, c10, 0xB0, false, &mut i));
        assert_eq!(i, 0);
        assert!(!fc4_lot_config_has_building(m, c10, 0x500, false, &mut i));
        assert!(fc4_lot_config_has_building(m, c10, 0x501, true, &mut i));
        assert!(!fc4_lot_config_has_building(m, c10, 0x999, true, &mut i));
        assert!(!fc4_lot_config_has_building(m, 0, 0xB0, false, &mut i));

        let mut r = [0.0f32; 4];
        assert!(fc4_lot_config_max_footprint(
            m,
            0,
            [0xB0u32].as_ptr(),
            1,
            r.as_mut_ptr()
        ));
        assert_eq!(r, [20.0, 30.0, 0.0, 0.0]);
        assert!(fc4_lot_config_max_footprint(
            m,
            1,
            [0xC0u32].as_ptr(),
            1,
            r.as_mut_ptr()
        ));
        assert_eq!(r, [3.0, 2.0, 0.0, 0.0]);
        assert!(fc4_lot_config_max_footprint(
            m,
            4,
            [0x700u32].as_ptr(),
            1,
            r.as_mut_ptr()
        ));
        assert_eq!(r, [5.0, 5.0, 0.0, 0.0]);
        assert!(!fc4_lot_config_max_footprint(m, 2, std::ptr::null(), 0, r.as_mut_ptr()));

        // GetObjectIDArray: families expand (a building family assigns).
        let b = Obj {
            kind: 0,
            ids: vec![0x500, 0xB0],
            ..Obj::default()
        };
        let bc = b.to_c();
        let ids = u32s(|o, n| fc4_lot_config_object_id_array(m, &bc, o, n));
        assert_eq!(ids, [0x500, 0x501]);
        let p = Obj {
            kind: 1,
            ids: vec![0xC0, 0x600],
            ..Obj::default()
        };
        let pc = p.to_c();
        let ids = u32s(|o, n| fc4_lot_config_object_id_array(m, &pc, o, n));
        assert_eq!(ids, [0x600, 0x600]);
        assert_eq!(
            fc4_lot_config_object_id_array(m, std::ptr::null(), std::ptr::null_mut(), 0),
            0
        );
        fc4_lot_config_destroy(m);
    }
}

#[test]
fn building_information_on_post_city_init() {
    let g = city();
    let m = ready(&g, std::ptr::null());
    unsafe {
        assert!(fc4_lot_config_do_message(m, POST_CITY_INIT));
        let c10 = configuration(m, 0x10);
        assert_eq!(g.cfg(c10).info, Some((10, 30, vec![0x1001, 0x1300], true)));
        let c11 = configuration(m, 0x11);
        assert_eq!(g.cfg(c11).info, Some((10, 10, vec![0x1001, 0x1300], false)));
        // No building: the defaults.
        let c12 = configuration(m, 0x12);
        assert_eq!(g.cfg(c12).info, Some((0, 0, vec![], false)));
        fc4_lot_config_destroy(m);
    }
}

#[test]
fn create_generic_and_fix() {
    let g = city();
    let m = ready(&g, std::ptr::null());
    unsafe {
        let c = fc4_lot_config_create_generic(m, 0x500);
        assert_ne!(c, 0);
        let cfg = g.cfg(c);
        assert_eq!((cfg.size, cfg.generic_for), ((2, 1), Some(0x500)));
        assert_eq!(cfg.objects.len(), 1);
        assert_eq!((cfg.objects[0].min, cfg.objects[0].max), ([6.0, 2.0], [26.0, 14.0]));
        assert_eq!(fc4_lot_config_create_generic(m, 0x999), 0);

        // Fix: an invalid ID goes (code 1), then the record's footprint and
        // the walls are checked.
        g.valid_walls.borrow_mut().push(0x30);
        let bad = g.fresh(Cfg {
            id: 0x40,
            size: (2, 2),
            objects: vec![Obj {
                kind: 1,
                offset: [8.0, 0.0, 8.0],
                min: [6.5, 7.5],
                max: [9.5, 8.5],
                ids: vec![0x600, 0x999],
                ..Obj::default()
            }],
            walls: vec![0x30, 0x31],
            ..Cfg::default()
        });
        assert_eq!(fc4_lot_config_fix(m, bad), 1);
        let cfg = g.cfg(bad);
        assert_eq!(cfg.objects[0].ids, [0x600]);
        assert_eq!(cfg.walls, [0x30]);
        assert_eq!(fc4_lot_config_fix(m, bad), 0);
        // A record left without IDs is removed (the first problem's code, 1,
        // is returned).
        g.configs.borrow_mut().get_mut(&bad).unwrap().objects[0].ids = vec![0x998];
        assert_eq!(fc4_lot_config_fix(m, bad), 1);
        assert!(g.cfg(bad).objects.is_empty());
        fc4_lot_config_destroy(m);
    }
}

#[test]
fn the_component_9_record() {
    let g = city();
    let m = ready(&g, std::ptr::null());
    unsafe {
        let mut o = Out::default();
        let os = ostream(&mut o);
        assert!(fc4_lot_config_write(m, &os));
        assert_eq!(o.refs, [(BDS, 0), (FLORA, 0), (PM, 0)]);
        assert_eq!(&o.bytes[..2], [1, 0]);
        fc4_lot_config_destroy(m);

        let m = fc4_lot_config_create(&host(&g), std::ptr::null(), 0x901);
        g.take();
        let mut i = In::new(&o.bytes);
        let is = istream(&mut i);
        assert!(fc4_lot_config_read(m, &is));
        assert_eq!(
            g.take(),
            ["resolve 50 07813EAE", "resolve 51 06C66177", "resolve 52 28326F3E"]
        );
        let s = state(m);
        assert_eq!(
            (s.building_development, s.flora, s.prop_manager, s.initialized),
            (BDS, FLORA, PM, 0)
        );
        let mut v0 = o.bytes.clone();
        v0[0] = 0;
        let m2 = fc4_lot_config_create(&host(&g), std::ptr::null(), 0x902);
        let mut i = In::new(&v0);
        let is = istream(&mut i);
        assert!(!fc4_lot_config_read(m2, &is));
        assert_eq!(state(m2).flora, 0);
        // Release of the last reference runs Shutdown.
        assert_eq!(fc4_lot_config_add_ref(m), 1);
        assert_eq!(fc4_lot_config_release(m), 0);
        assert_eq!(state(m).destroyed, 1);
        fc4_lot_config_destroy(m2);
        fc4_lot_config_destroy(m);
    }
}
