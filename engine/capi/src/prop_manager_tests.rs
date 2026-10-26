// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The prop manager through the C ABI (docs/specs/prop-lotconfig.md Part
//! A): a fake game behind `fc4_prop_manager_host`.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::{c_char, c_void};

use crate::budget::fc4_token_callback;
use crate::development::fc4_res_key;
use crate::hostgrid::fc4_cell_rect;
use crate::ordinance::fc4_key_visitor;
use crate::prop_manager::*;
use crate::props::{fc4_props, fc4_props_create, fc4_props_set_f32s, fc4_props_set_u32s};
use crate::test_streams::{istream, ostream, In, Out};

const OM: u64 = 0x50;
const LM: u64 = 0x51;
const RM: u64 = 0x52;
const VIEW: u64 = 0x53;
const EXEMPLAR: u32 = 0x6534_284A;
const GROUP: u32 = 0x1234;
const EXEMPLAR_TYPE: u32 = 0x10;
const FAMILIES: u32 = 0x2781_2870;
const SIZE: u32 = 0x2781_2810;
const CLASS: u32 = 0xA977_A86B;
const POST_VIEW_FILTER_CHANGE: u32 = 0x06CD_B65B;
const CREATE_TEXT_DECAL: u32 = 0x0BEA_F1A2;

#[derive(Clone, Default)]
struct Ex {
    exemplar_type: u32,
    families: Vec<u32>,
    size: Option<Vec<f32>>,
    class: Option<u32>,
}

#[derive(Clone, Default)]
struct Occ {
    flags: u32,
    pos: [f32; 3],
    usage: u32,
    prop_type: u32,
    bbox: Option<([f32; 3], [f32; 3])>,
}

#[derive(Default)]
struct Game {
    log: RefCell<Vec<String>>,
    refs: RefCell<BTreeMap<u64, i32>>,
    exemplars: RefCell<BTreeMap<u32, Ex>>,
    no_rm: Cell<bool>,
    next: Cell<u64>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    buildings: RefCell<BTreeMap<(i32, i32), u64>>,
    /// building → None (no holder), Some(None) (absent), Some(Some(v)).
    building_props: RefCell<BTreeMap<u64, Option<Option<Vec<u64>>>>>,
    lots: RefCell<BTreeMap<u64, fc4_cell_rect>>,
}

impl Game {
    fn new() -> Box<Game> {
        let g = Box::new(Game::default());
        g.next.set(0x1000);
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
    fn exemplar(&self, instance: u32, ex: Ex) {
        self.exemplars.borrow_mut().insert(instance, ex);
    }
    fn occ(&self, t: u64) -> Occ {
        self.occs.borrow().get(&t).cloned().unwrap_or_default()
    }
    fn new_occ(&self) -> u64 {
        let t = self.next.get();
        self.next.set(t + 0x10);
        self.occs.borrow_mut().insert(t, Occ::default());
        *self.refs.borrow_mut().entry(t).or_default() += 1;
        t
    }
    fn props_of(&self, instance: u32) -> *mut fc4_props {
        let Some(ex) = self.exemplars.borrow().get(&instance).cloned() else {
            return std::ptr::null_mut();
        };
        unsafe {
            let p = fc4_props_create();
            fc4_props_set_u32s(p, EXEMPLAR_TYPE, &ex.exemplar_type, 1);
            fc4_props_set_u32s(p, FAMILIES, ex.families.as_ptr(), ex.families.len());
            if let Some(s) = &ex.size {
                fc4_props_set_f32s(p, SIZE, s.as_ptr(), s.len());
            }
            if let Some(c) = ex.class {
                fc4_props_set_u32s(p, CLASS, &c, 1);
            }
            p
        }
    }
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

unsafe fn give(u: *mut c_void, list: &[u64], f: fc4_token_callback, ctx: *mut c_void) {
    for &t in list {
        add_ref(u, t);
        f.unwrap()(ctx, t);
    }
}

unsafe extern "C" fn for_each_exemplar_key(u: *mut c_void, visit: fc4_key_visitor, ctx: *mut c_void) -> bool {
    let g = game(u);
    if g.no_rm.get() {
        return false;
    }
    for &i in g.exemplars.borrow().keys() {
        visit.unwrap()(ctx, EXEMPLAR, GROUP, i);
    }
    true
}
unsafe extern "C" fn load_exemplar(u: *mut c_void, _t: u32, _g: u32, i: u32) -> *mut fc4_props {
    game(u).props_of(i)
}
unsafe extern "C" fn register_shutdown_callback(u: *mut c_void) {
    game(u).log("shutdown callback");
}
unsafe extern "C" fn post_exemplar_data_updated(u: *mut c_void, m: u64, data2: u32) {
    game(u).log(format!("post {m:X} {data2}"));
}
unsafe extern "C" fn register_command(u: *mut c_void, id: u32) {
    game(u).log(format!("command {id:08X}"));
}
unsafe extern "C" fn unregister_command(u: *mut c_void, id: u32) {
    game(u).log(format!("uncommand {id:08X}"));
}
unsafe extern "C" fn subscribe(u: *mut c_void, m: u32) {
    game(u).log(format!("subscribe {m:08X}"));
}
unsafe extern "C" fn unsubscribe(u: *mut c_void, m: u32) {
    game(u).log(format!("unsubscribe {m:08X}"));
}
unsafe extern "C" fn acquire_service(u: *mut c_void, s: u32) -> u64 {
    let t = match s {
        FC4_PROP_SERVICE_OCCUPANT_MANAGER => OM,
        FC4_PROP_SERVICE_RESOURCE_MANAGER if !game(u).no_rm.get() => RM,
        FC4_PROP_SERVICE_LOT_MANAGER => LM,
        _ => return 0,
    };
    add_ref(u, t);
    t
}
unsafe extern "C" fn view_filter(u: *mut c_void) -> u64 {
    add_ref(u, VIEW);
    VIEW
}
unsafe extern "C" fn add_ref(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_default() += 1;
}
unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_default() -= 1;
}
unsafe extern "C" fn create_prop_instance(_u: *mut c_void, _class: u32) -> u64 {
    0
}
unsafe extern "C" fn create_builtin_prop(u: *mut c_void, kind: u32) -> u64 {
    game(u).log(format!("builtin {kind}"));
    game(u).new_occ()
}
unsafe extern "C" fn set_prop_type(u: *mut c_void, p: u64, id: u32) {
    game(u).occs.borrow_mut().get_mut(&p).unwrap().prop_type = id;
}
unsafe extern "C" fn attach_exemplar(
    u: *mut c_void,
    _p: u64,
    _t: u32,
    _g: u32,
    i: u32,
    _global: bool,
) -> *mut fc4_props {
    game(u).props_of(i)
}
unsafe extern "C" fn set_bounding_box(u: *mut c_void, p: u64, min: *const f32, max: *const f32) {
    let r = |p: *const f32| [*p, *p.add(1), *p.add(2)];
    game(u).occs.borrow_mut().get_mut(&p).unwrap().bbox = Some((r(min), r(max)));
}
unsafe extern "C" fn occupant_flag(u: *mut c_void, o: u64, flag: u32) -> bool {
    game(u).occ(o).flags & flag != 0
}
unsafe extern "C" fn occupant_init(u: *mut c_void, o: u64) -> bool {
    game(u).log(format!("init {o:X}"));
    true
}
unsafe extern "C" fn occupant_shutdown(u: *mut c_void, o: u64) {
    game(u).log(format!("shutdown {o:X}"));
}
unsafe extern "C" fn set_prop_usage(u: *mut c_void, o: u64, usage: u32) {
    if let Some(occ) = game(u).occs.borrow_mut().get_mut(&o) {
        occ.usage = usage;
    }
}
unsafe extern "C" fn set_position(u: *mut c_void, o: u64, p: *const f32) {
    game(u).occs.borrow_mut().get_mut(&o).unwrap().pos = [*p, *p.add(1), *p.add(2)];
}
unsafe extern "C" fn set_orientation(_u: *mut c_void, _o: u64, _r: i32) {}
unsafe extern "C" fn position(u: *mut c_void, o: u64, out: *mut f32) {
    let p = game(u).occ(o).pos;
    std::ptr::copy_nonoverlapping(p.as_ptr(), out, 3);
}
unsafe extern "C" fn hide(u: *mut c_void, o: u64) {
    game(u).log(format!("hide {o:X}"));
    game(u).occs.borrow_mut().get_mut(&o).unwrap().flags &= !2;
}
unsafe extern "C" fn insert_occupant(u: *mut c_void, o: u64) -> bool {
    game(u).occs.borrow_mut().get_mut(&o).unwrap().flags |= 1 | 2;
    true
}
unsafe extern "C" fn remove_occupant(u: *mut c_void, o: u64) {
    game(u).log(format!("remove {o:X}"));
    game(u).occs.borrow_mut().get_mut(&o).unwrap().flags &= !1;
}
fn inserted(g: &Game, usage: u32, keep: impl Fn(&Occ) -> bool) -> Vec<u64> {
    g.occs
        .borrow()
        .iter()
        .filter(|(_, o)| o.flags & 1 != 0 && o.usage == usage && keep(o))
        .map(|(t, _)| *t)
        .collect()
}
unsafe extern "C" fn occupants_in_box(
    u: *mut c_void,
    min: *const f32,
    max: *const f32,
    usage: u32,
    f: fc4_token_callback,
    ctx: *mut c_void,
) -> bool {
    let (lo, hi) = ([*min, *min.add(2)], [*max, *max.add(2)]);
    let v = inserted(game(u), usage, |o| {
        lo[0] <= o.pos[0] && o.pos[0] <= hi[0] && lo[1] <= o.pos[2] && o.pos[2] <= hi[1]
    });
    give(u, &v, f, ctx);
    true
}
unsafe extern "C" fn occupants_in_cell_rect(
    u: *mut c_void,
    r: *const fc4_cell_rect,
    usage: u32,
    f: fc4_token_callback,
    ctx: *mut c_void,
) -> bool {
    let r = *r;
    // Within one cell of the rectangle (as an occupant manager's coarse query).
    let v = inserted(game(u), usage, |o| {
        let (x, z) = ((o.pos[0] / 16.0).floor() as i32, (o.pos[2] / 16.0).floor() as i32);
        r.x1 - 1 <= x && x <= r.x2 + 1 && r.z1 - 1 <= z && z <= r.z2 + 1
    });
    give(u, &v, f, ctx);
    true
}
unsafe extern "C" fn building_at_cell(u: *mut c_void, x: i32, z: i32) -> u64 {
    match game(u).buildings.borrow().get(&(x, z)) {
        Some(&b) => {
            add_ref(u, b);
            b
        }
        None => 0,
    }
}
unsafe extern "C" fn lot_at_cell(u: *mut c_void, x: i32, z: i32) -> u64 {
    game(u)
        .lots
        .borrow()
        .iter()
        .find(|(_, r)| r.x1 <= x && x <= r.x2 && r.z1 <= z && z <= r.z2)
        .map_or(0, |(t, _)| *t)
}
unsafe extern "C" fn lot_cell_rect(u: *mut c_void, lot: u64, out: *mut fc4_cell_rect) {
    *out = game(u).lots.borrow()[&lot];
}
unsafe extern "C" fn cell_to_world(_u: *mut c_void, x: i32, z: i32, wx: *mut f32, wz: *mut f32) {
    *wx = x as f32 * 16.0;
    *wz = z as f32 * 16.0;
}
unsafe extern "C" fn world_to_cell(_u: *mut c_void, x: f32, z: f32, cx: *mut i32, cz: *mut i32) {
    *cx = (x / 16.0).floor() as i32;
    *cz = (z / 16.0).floor() as i32;
}
unsafe extern "C" fn building_props(u: *mut c_void, b: u64, f: fc4_token_callback, ctx: *mut c_void) -> u32 {
    let entry = game(u).building_props.borrow().get(&b).cloned();
    match entry {
        None | Some(None) => FC4_PROP_BUILDING_NO_HOLDER,
        Some(Some(None)) => FC4_PROP_BUILDING_ABSENT,
        Some(Some(Some(v))) => {
            for t in v {
                f.unwrap()(ctx, t);
            }
            FC4_PROP_BUILDING_PRESENT
        }
    }
}
unsafe extern "C" fn set_building_props(u: *mut c_void, b: u64, p: *const u64, n: usize) {
    let v = if n == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(p, n).to_vec()
    };
    game(u).building_props.borrow_mut().insert(b, Some(Some(v)));
}
unsafe extern "C" fn add_building_props_property(u: *mut c_void, b: u64, p: *const u64, n: usize) {
    game(u).log(format!("add property {b:X}"));
    set_building_props(u, b, p, n);
}
unsafe extern "C" fn remove_building_props_property(u: *mut c_void, b: u64) {
    game(u).log(format!("remove property {b:X}"));
    game(u).building_props.borrow_mut().insert(b, Some(None));
}
unsafe extern "C" fn create_text_decal(u: *mut c_void) -> u64 {
    game(u).log("decal");
    game(u).new_occ()
}
unsafe extern "C" fn terrain_altitude(_u: *mut c_void, _x: f32, _z: f32) -> f32 {
    7.0
}
unsafe extern "C" fn decal_set_text(u: *mut c_void, _d: u64, t: *const c_char, n: usize) {
    let s = String::from_utf8_lossy(std::slice::from_raw_parts(t as *const u8, n)).into_owned();
    game(u).log(format!("text {s}"));
}
unsafe extern "C" fn decal_slot8(u: *mut c_void, _d: u64, v: *const f32) {
    game(u).log(format!("slot8 {} {} {}", *v, *v.add(1), *v.add(2)));
}
unsafe extern "C" fn decal_slot10(u: *mut c_void, _d: u64, v: f32) {
    game(u).log(format!("slot10 {v}"));
}
unsafe extern "C" fn decal_slot6(u: *mut c_void, _d: u64, v: f32) {
    game(u).log(format!("slot6 {v}"));
}
unsafe extern "C" fn resolve_ref(u: *mut c_void, t: u64, iid: u32) -> u64 {
    game(u).log(format!("resolve {t:X} {iid:08X}"));
    t
}

fn host(g: &Game) -> fc4_prop_manager_host {
    fc4_prop_manager_host {
        struct_size: std::mem::size_of::<fc4_prop_manager_host>() as u32,
        user: g.user(),
        for_each_exemplar_key: Some(for_each_exemplar_key),
        load_exemplar: Some(load_exemplar),
        register_shutdown_callback: Some(register_shutdown_callback),
        post_exemplar_data_updated: Some(post_exemplar_data_updated),
        register_command: Some(register_command),
        unregister_command: Some(unregister_command),
        subscribe: Some(subscribe),
        unsubscribe: Some(unsubscribe),
        acquire_service: Some(acquire_service),
        view_filter: Some(view_filter),
        add_ref: Some(add_ref),
        release: Some(release),
        create_prop_instance: Some(create_prop_instance),
        create_builtin_prop: Some(create_builtin_prop),
        set_prop_type: Some(set_prop_type),
        attach_exemplar: Some(attach_exemplar),
        set_bounding_box: Some(set_bounding_box),
        occupant_flag: Some(occupant_flag),
        occupant_init: Some(occupant_init),
        occupant_shutdown: Some(occupant_shutdown),
        set_prop_usage: Some(set_prop_usage),
        set_position: Some(set_position),
        set_orientation: Some(set_orientation),
        position: Some(position),
        hide: Some(hide),
        insert_occupant: Some(insert_occupant),
        remove_occupant: Some(remove_occupant),
        occupants_in_box: Some(occupants_in_box),
        occupants_in_cell_rect: Some(occupants_in_cell_rect),
        building_at_cell: Some(building_at_cell),
        lot_at_cell: Some(lot_at_cell),
        lot_cell_rect: Some(lot_cell_rect),
        cell_to_world: Some(cell_to_world),
        world_to_cell: Some(world_to_cell),
        building_props: Some(building_props),
        set_building_props: Some(set_building_props),
        add_building_props_property: Some(add_building_props_property),
        remove_building_props_property: Some(remove_building_props_property),
        create_text_decal: Some(create_text_decal),
        terrain_altitude: Some(terrain_altitude),
        decal_set_text: Some(decal_set_text),
        decal_slot8: Some(decal_slot8),
        decal_slot10: Some(decal_slot10),
        decal_slot6: Some(decal_slot6),
        resolve_ref: Some(resolve_ref),
    }
}

/// Two props (0x100 in family 0xF1 with a size; 0x101 in 0xF1 and 0xF2 with
/// the terrain decal class) and an exemplar of another type.
fn catalogue_game() -> Box<Game> {
    let g = Game::new();
    g.exemplar(
        0x100,
        Ex {
            exemplar_type: 0x1E,
            families: vec![0xF1],
            size: Some(vec![2.0, 3.0, 4.0]),
            class: None,
        },
    );
    g.exemplar(
        0x101,
        Ex {
            exemplar_type: 0x1E,
            families: vec![0xF1, 0xF2],
            size: None,
            class: Some(0x2977_AA48),
        },
    );
    g.exemplar(
        0x102,
        Ex {
            exemplar_type: 0x02,
            families: vec![0xF3],
            ..Ex::default()
        },
    );
    g
}

fn ready(g: &Game, cat: *const fc4_prop_catalogue) -> *mut fc4_prop_manager {
    let m = unsafe { fc4_prop_manager_create(&host(g), cat, 0x900) };
    assert!(!m.is_null());
    assert!(unsafe { fc4_prop_manager_init(m) });
    m
}

fn state(m: *const fc4_prop_manager) -> fc4_prop_manager_state {
    let mut s = fc4_prop_manager_state::default();
    assert!(unsafe { fc4_prop_manager_get_state(m, &mut s) });
    s
}

fn cat_state(c: *const fc4_prop_catalogue) -> fc4_prop_catalogue_state {
    let mut s = fc4_prop_catalogue_state::default();
    assert!(unsafe { fc4_prop_catalogue_get_state(c, &mut s) });
    s
}

unsafe extern "C" fn push_token(ctx: *mut c_void, t: u64) {
    (*(ctx as *mut Vec<u64>)).push(t);
}
unsafe extern "C" fn take_u32s(ctx: *mut c_void, v: *const u32, n: usize) {
    *(ctx as *mut Vec<u32>) = if n == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(v, n).to_vec()
    };
}

fn list_of(f: impl FnOnce(fc4_token_callback, *mut c_void) -> bool) -> (bool, Vec<u64>) {
    let mut v: Vec<u64> = Vec::new();
    let r = f(Some(push_token), &mut v as *mut Vec<u64> as *mut c_void);
    (r, v)
}

fn sorted(mut v: Vec<u32>) -> Vec<u32> {
    v.sort_unstable();
    v
}

fn u32s(f: impl Fn(*mut u32, usize) -> usize) -> Vec<u32> {
    let n = f(std::ptr::null_mut(), 0);
    let mut v = vec![0u32; n];
    assert_eq!(f(v.as_mut_ptr(), n), n);
    v
}

#[test]
fn create_rejects_a_short_table() {
    let g = Game::new();
    let mut h = host(&g);
    h.struct_size -= 1;
    unsafe {
        assert!(fc4_prop_manager_create(&h, std::ptr::null(), 0).is_null());
        assert!(fc4_prop_manager_create(std::ptr::null(), std::ptr::null(), 0).is_null());
        assert!(fc4_prop_manager_query_interface(0x2832_6F3E));
        assert!(!fc4_prop_manager_query_interface(0x1234_5678));
        assert_eq!(
            fc4_prop_manager_all_prop_types(std::ptr::null(), std::ptr::null_mut(), 0),
            0
        );
        fc4_prop_manager_destroy(std::ptr::null_mut());
        fc4_prop_catalogue_destroy(std::ptr::null_mut());
    }
}

#[test]
fn the_catalogue_is_built_once_per_process() {
    let g = catalogue_game();
    let cat = fc4_prop_catalogue_create();
    assert_eq!(cat_state(cat).built, 0);
    let m = ready(&g, cat);
    assert_eq!(
        g.take(),
        [
            "shutdown callback",
            "post 900 2", // PL2: data2 = the type count
            "command 0BEAF1A2",
            "subscribe A6B79602",
            "subscribe 06CDB65B",
            "subscribe 26D31EC1",
            "subscribe 0BEAF1A2"
        ]
    );
    let c = cat_state(cat);
    assert_eq!(
        (c.built, c.callback_registered, c.types, c.families, c.profiles),
        (1, 1, 2, 2, 2)
    );
    let s = state(m);
    assert_eq!(
        (s.initialized, s.occupant_manager, s.resource_manager, s.lot_manager),
        (1, OM, RM, LM)
    );
    assert_eq!(s.view_filter, VIEW);
    unsafe {
        let types = u32s(|o, n| fc4_prop_manager_all_prop_types(m, o, n));
        assert_eq!(sorted(types), [0x100, 0x101]);
        let fams = u32s(|o, n| fc4_prop_manager_all_prop_family_types(m, o, n));
        assert_eq!(sorted(fams), [0xF1, 0xF2]);
        // Slot 9 (quirk 1).
        let mut v: Vec<u32> = Vec::new();
        let ctx = &mut v as *mut Vec<u32> as *mut c_void;
        let seed = [7u32, 8];
        assert_eq!(
            fc4_prop_manager_prop_types_by_family(m, 0xF1, seed.as_ptr(), 2, Some(take_u32s), ctx),
            2
        );
        assert_eq!(v, [0x100, 0x101]);
        assert_eq!(
            fc4_prop_manager_prop_types_by_family(m, 0x100, seed.as_ptr(), 2, Some(take_u32s), ctx),
            0
        );
        assert_eq!(v, [0x100]);
        assert_eq!(
            fc4_prop_manager_prop_types_by_family(m, 0x999, seed.as_ptr(), 2, Some(take_u32s), ctx),
            0
        );
        assert_eq!(v, [7, 8]);
        assert!(fc4_prop_manager_is_prop_type_valid(m, 0x101));
        assert!(!fc4_prop_manager_is_prop_type_valid(m, 0x102));
        assert!(fc4_prop_manager_is_prop_in_family(m, 0x101, 0xF2));
        assert!(fc4_prop_manager_is_prop_in_family(m, 0xF2, 0xF2));
        assert!(!fc4_prop_manager_is_prop_in_family(m, 0x100, 0xF2));
        assert!(fc4_prop_manager_is_prop_family_type_valid(m, 0xF1));
        assert!(fc4_prop_manager_is_prop_family_type_valid(m, 0x100));
        assert!(!fc4_prop_manager_is_prop_family_type_valid(m, 0xF3));
        let mut k = fc4_res_key::default();
        assert!(fc4_prop_manager_prop_key_from_type(m, 0x100, &mut k));
        assert_eq!((k.type_id, k.group, k.instance), (EXEMPLAR, GROUP, 0x100));
        assert!(!fc4_prop_manager_prop_key_from_type(m, 0x999, &mut k));
        // Profiles (§A3.2) and the inserting slot 13 (quirk 2).
        let mut p = fc4_prop_profile::default();
        assert!(fc4_prop_manager_prop_profile_ptr(m, 0x100, &mut p));
        assert_eq!((p.class_id, p.size), (0x2977_AA47, [2.0, 3.0, 4.0]));
        assert!(fc4_prop_manager_prop_profile_ptr(m, 0x101, &mut p));
        assert_eq!((p.class_id, p.size), (0x2977_AA48, [0.0; 3]));
        assert!(!fc4_prop_manager_prop_profile_ptr(m, 0x999, &mut p));
        assert!(fc4_prop_manager_prop_profile(m, 0x999, &mut p));
        assert_eq!(p, fc4_prop_profile::default());
        assert_eq!(cat_state(cat).profiles, 3);
        // Slot 12 (quirk 3): an empty name.
        let mut buf = [b'x' as c_char; 4];
        assert_eq!(fc4_prop_manager_prop_name(m, 0x100, buf.as_mut_ptr(), buf.len()), 0);
        assert_eq!(buf[0], 0);

        // A second manager of the process does not rebuild or post.
        let m2 = ready(&g, cat);
        assert!(!g
            .take()
            .iter()
            .any(|l| l.starts_with("post") || l.starts_with("shutdown callback")));
        assert!(fc4_prop_manager_is_prop_type_valid(m2, 0x100));
        // The application shutdown callback tears the catalogue down.
        fc4_prop_catalogue_app_shutdown(cat);
        let c = cat_state(cat);
        assert_eq!((c.built, c.types), (0, 0));
        assert!(!fc4_prop_manager_is_prop_type_valid(m2, 0x100));
        fc4_prop_manager_destroy(m2);
        fc4_prop_manager_destroy(m);
        fc4_prop_catalogue_destroy(cat);
    }
}

#[test]
fn create_prop_uses_the_builtin_classes_and_the_default_box() {
    let g = catalogue_game();
    let m = ready(&g, std::ptr::null());
    g.take();
    unsafe {
        let p = fc4_prop_manager_create_prop(m, 0x100);
        assert_eq!(g.take(), ["builtin 0"]);
        let o = g.occ(p);
        assert_eq!((o.prop_type, o.bbox), (0x100, Some(([0.0; 3], [2.0, 3.0, 4.0]))));
        assert_eq!(g.refs(p), 1);
        let q = fc4_prop_manager_create_prop(m, 0x101);
        assert_eq!(g.take(), ["builtin 1"]);
        assert_eq!(g.occ(q).bbox, Some(([0.0; 3], [1.0; 3])));
        assert_eq!(fc4_prop_manager_create_prop(m, 0x999), 0);
        // CreateGenericProp: the global resource manager, no prop type.
        let r = fc4_prop_manager_create_generic_prop(m, EXEMPLAR, GROUP, 0x101);
        assert_ne!(r, 0);
        assert_eq!(g.take(), ["builtin 1"]);
        assert_eq!(g.occ(r).prop_type, 0);
        assert_eq!(fc4_prop_manager_create_generic_prop(m, EXEMPLAR, GROUP, 0x777), 0);
        fc4_prop_manager_destroy(m);
    }
}

#[test]
fn insertion_by_usage_and_the_lot_query_boundary() {
    let g = catalogue_game();
    g.lots.borrow_mut().insert(
        0x70,
        fc4_cell_rect {
            x1: 0,
            z1: 0,
            x2: 0,
            z2: 0,
        },
    );
    let m = ready(&g, std::ptr::null());
    unsafe {
        let city = fc4_prop_manager_add_prop_new(m, 1, 0x100, [40.0, 0.0, 40.0].as_ptr(), 0);
        let inside = fc4_prop_manager_add_prop_new(m, 2, 0x100, [15.9, 0.0, 1.0].as_ptr(), 1);
        let edge = fc4_prop_manager_add_prop_new(m, 2, 0x100, [16.0, 0.0, 1.0].as_ptr(), 1);
        assert!(city != 0 && inside != 0 && edge != 0);
        assert_eq!((g.occ(city).usage, g.occ(inside).usage), (1, 2));
        // The manager keeps no reference to a new prop.
        assert_eq!((g.refs(city), g.refs(inside)), (0, 0));
        assert_eq!(fc4_prop_manager_add_prop_new(m, 3, 0x100, [0.0; 3].as_ptr(), 0), 0);

        let (ok, v) =
            list_of(|f, c| fc4_prop_manager_get_city_props_world(m, 32.0, 32.0, 48.0, 48.0, std::ptr::null(), 0, f, c));
        assert!(ok);
        assert_eq!(v, [city]);
        assert_eq!(g.refs(city), 1);
        // GetLotProps(list, lot) filters the caller's list too (quirk 7):
        // the city prop already in the list is dropped and released.
        let (ok, v) = list_of(|f, c| fc4_prop_manager_get_props_of(m, 2, 0x70, &city, 1, f, c));
        assert!(ok);
        assert_eq!(v, [inside, edge]); // PL24 retains equality at the upper bound.
        assert_eq!((g.refs(city), g.refs(inside), g.refs(edge)), (0, 1, 1));
        let (ok, v) = list_of(|f, c| fc4_prop_manager_get_props_at(m, 2, 0, 0, std::ptr::null(), 0, f, c));
        assert!(ok);
        assert_eq!(v, [inside, edge]);
        // GetStreetProps(list, x, z) is always false (quirk 4).
        let (ok, _) = list_of(|f, c| fc4_prop_manager_get_props_at(m, 4, 0, 0, std::ptr::null(), 0, f, c));
        assert!(!ok);
        // Removing: hide, remove, shut down.
        g.take();
        assert!(fc4_prop_manager_remove_prop(m, edge));
        assert_eq!(
            g.take(),
            [
                format!("hide {edge:X}"),
                format!("remove {edge:X}"),
                format!("shutdown {edge:X}")
            ]
        );
        fc4_prop_manager_destroy(m);
    }
}

#[test]
fn building_props_vector() {
    let g = catalogue_game();
    let b = 0x80;
    g.buildings.borrow_mut().insert((2, 3), b);
    g.building_props.borrow_mut().insert(b, Some(None));
    let m = ready(&g, std::ptr::null());
    unsafe {
        let p1 = fc4_prop_manager_create_prop(m, 0x100);
        let p2 = fc4_prop_manager_create_prop(m, 0x100);
        g.take();
        assert!(fc4_prop_manager_add_building_prop(m, b, p1));
        assert_eq!(g.take(), [format!("init {p1:X}"), format!("add property {b:X}")]);
        assert!(fc4_prop_manager_add_building_prop(m, b, p2));
        assert!(fc4_prop_manager_add_building_prop(m, b, p2));
        assert_eq!(g.occ(p2).usage, 3);
        let (ok, v) = list_of(|f, c| fc4_prop_manager_get_props_at(m, 3, 2, 3, std::ptr::null(), 0, f, c));
        assert!(ok);
        assert_eq!(v, [p1, p2]);
        assert_eq!((g.refs(p1), g.refs(p2), g.refs(b)), (2, 2, 0));
        assert!(fc4_prop_manager_remove_building_prop(m, b, p1));
        assert!(!fc4_prop_manager_remove_building_prop(m, b, p1));
        assert_eq!(g.building_props.borrow()[&b], Some(Some(vec![p2])));
        g.take();
        assert!(fc4_prop_manager_remove_building_prop(m, b, p2));
        assert_eq!(
            g.take(),
            [
                format!("remove {p2:X}"),
                format!("shutdown {p2:X}"),
                format!("remove property {b:X}")
            ]
        );
        // No property: GetBuildingProps is false.
        let (ok, v) = list_of(|f, c| fc4_prop_manager_get_props_of(m, 3, b, std::ptr::null(), 0, f, c));
        assert!(!ok && v.is_empty());
        assert!(!fc4_prop_manager_remove_props_of(m, 3, b));
        fc4_prop_manager_destroy(m);
    }
}

#[test]
fn messages_view_filter_and_text_decal() {
    let g = catalogue_game();
    let m = ready(&g, std::ptr::null());
    g.take();
    unsafe {
        assert!(fc4_prop_manager_do_message(
            m,
            POST_VIEW_FILTER_CHANGE,
            0x99,
            std::ptr::null(),
            0
        ));
        assert_eq!(state(m).view_filter, 0x99);
        assert_eq!((g.refs(0x99), g.refs(VIEW)), (1, 0));
        let text = b"Hi";
        let arg = |v: f32| fc4_prop_command_arg {
            text: text.as_ptr() as *const c_char,
            text_len: 2,
            value: v,
        };
        let args = [arg(0.0), arg(32.0), arg(48.0), arg(0.5)];
        assert!(fc4_prop_manager_do_message(
            m,
            CREATE_TEXT_DECAL,
            0,
            args.as_ptr(),
            args.len()
        ));
        let log = g.take();
        assert_eq!(log[0], "decal");
        assert_eq!(
            &log[log.len() - 4..],
            ["text Hi", "slot8 1 1 1", "slot10 1", "slot6 0.5"]
        );
        // Two arguments: nothing.
        assert!(fc4_prop_manager_do_message(m, CREATE_TEXT_DECAL, 0, args.as_ptr(), 2));
        assert!(g.take().is_empty());
        assert!(fc4_prop_manager_shutdown(m));
        let log = g.take();
        assert!(log.contains(&"uncommand 0BEAF1A2".to_owned()));
        let s = state(m);
        assert_eq!((s.initialized, s.occupant_manager, s.view_filter), (0, 0, 0));
        fc4_prop_manager_destroy(m);
    }
}

#[test]
fn the_component_5_record() {
    let g = catalogue_game();
    let m = ready(&g, std::ptr::null());
    unsafe {
        let mut o = Out::default();
        let os = ostream(&mut o);
        assert!(fc4_prop_manager_write(m, &os));
        assert_eq!(o.refs, [(OM, 0), (LM, 0)]);
        assert_eq!(&o.bytes[..2], [1, 0]);
        fc4_prop_manager_destroy(m);

        // A fresh manager reads it back (references through resolve_ref).
        let m = fc4_prop_manager_create(&host(&g), std::ptr::null(), 0x901);
        g.take();
        let mut i = In::new(&o.bytes);
        let is = istream(&mut i);
        assert!(fc4_prop_manager_read(m, &is));
        assert_eq!(g.take(), ["resolve 50 C6BB8334", "resolve 51 681905BD"]);
        let s = state(m);
        assert_eq!((s.occupant_manager, s.lot_manager), (OM, LM));
        // Another version: false, nothing changes.
        let mut v2 = o.bytes.clone();
        v2[0] = 2;
        let m2 = fc4_prop_manager_create(&host(&g), std::ptr::null(), 0x902);
        let mut i = In::new(&v2);
        let is = istream(&mut i);
        assert!(!fc4_prop_manager_read(m2, &is));
        assert_eq!(state(m2).occupant_manager, 0);
        assert!(!fc4_prop_manager_read(m2, std::ptr::null()));
        fc4_prop_manager_destroy(m2);
        fc4_prop_manager_destroy(m);
    }
}
