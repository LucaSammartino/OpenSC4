// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The demolition utility through the C ABI (docs/specs/demolition.md): a
//! fake city behind `fc4_demolition_host`.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::c_void;

use crate::budget::fc4_token_callback;
use crate::demolition::*;
use crate::hostgrid::fc4_cell_rect;
use crate::props::{fc4_props, fc4_props_create, fc4_props_set_u32s};
use crate::test_streams::{istream, ostream, In, Out};
use crate::zones::fc4_zone_region;

const NETWORK: u32 = 0xC772_BF98;
const POWER_POLE: u32 = 0x2890_D4DE;
const IN_MANAGER: u32 = 1;
const DESTROYED: u32 = 6;
const UTILITY: u64 = 0xD0;

#[derive(Clone, Default)]
struct Occ {
    ty: u32,
    cell: (i32, i32),
    flags: u32,
    cost: i64,
    connections: Vec<u64>,
}

#[derive(Clone)]
struct Lot {
    rect: fc4_cell_rect,
    state: u32,
    initialized: bool,
}

#[derive(Default)]
struct Game {
    log: RefCell<Vec<String>>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    lots: RefCell<BTreeMap<u64, Lot>>,
    tile_cost: Cell<Option<u32>>,
}

impl Game {
    fn new() -> Box<Game> {
        let g = Box::new(Game::default());
        g.tile_cost.set(Some(2));
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
    fn occ(&self, t: u64, ty: u32, x: i32, z: i32, cost: i64, connections: &[u64]) {
        let o = Occ {
            ty,
            cell: (x, z),
            flags: IN_MANAGER,
            cost,
            connections: connections.to_vec(),
        };
        self.occs.borrow_mut().insert(t, o);
    }
    fn o<R>(&self, t: u64, f: impl FnOnce(&Occ) -> R) -> R {
        f(&self.occs.borrow()[&t])
    }
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

fn overlaps(a: &fc4_cell_rect, b: &fc4_cell_rect) -> bool {
    a.x1 <= b.x2 && a.x2 >= b.x1 && a.z1 <= b.z2 && a.z2 >= b.z1
}

unsafe extern "C" fn subscribe(u: *mut c_void, m: u32) {
    game(u).log(format!("subscribe {m:08X}"));
}
unsafe extern "C" fn unsubscribe(u: *mut c_void, m: u32) {
    game(u).log(format!("unsubscribe {m:08X}"));
}
unsafe extern "C" fn acquire_city(u: *mut c_void) -> bool {
    game(u).log("acquire city");
    true
}
unsafe extern "C" fn release_city(u: *mut c_void) {
    game(u).log("release city");
}
unsafe extern "C" fn city_cell_counts(_: *mut c_void, x: *mut i32, z: *mut i32) {
    *x = 64;
    *z = 128;
}
unsafe extern "C" fn acquire_terrain(u: *mut c_void) -> bool {
    game(u).log("acquire terrain");
    true
}
unsafe extern "C" fn release_terrain(u: *mut c_void) {
    game(u).log("release terrain");
}
unsafe extern "C" fn thresholds(_: *mut c_void) -> *mut fc4_props {
    let p = fc4_props_create();
    assert!(fc4_props_set_u32s(p, 0x6A8A_09D2, &20, 1));
    assert!(fc4_props_set_u32s(p, 0x6A8A_09D3, &64, 1));
    p
}
unsafe extern "C" fn building_development(u: *mut c_void) -> *mut fc4_props {
    let Some(c) = game(u).tile_cost.get() else {
        return std::ptr::null_mut();
    };
    let p = fc4_props_create();
    assert!(fc4_props_set_u32s(p, 0x2AB8_45E7, &c, 1));
    p
}
unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    game(u).log(format!("release {t:#x}"));
}
unsafe extern "C" fn view_level(_: *mut c_void, l: *mut u32) -> bool {
    *l = 1;
    true
}
unsafe extern "C" fn yes(_: *mut c_void, _: u64) -> bool {
    true
}
unsafe extern "C" fn occupant_type(u: *mut c_void, o: u64) -> u32 {
    game(u).o(o, |o| o.ty)
}
unsafe extern "C" fn occupant_position(u: *mut c_void, o: u64, p: *mut f32) {
    let (x, z) = game(u).o(o, |o| o.cell);
    *p = 16.0 * x as f32 + 8.0;
    *p.add(2) = 16.0 * z as f32 + 8.0;
}
unsafe extern "C" fn can_remove(_: *mut c_void, _: u64, _: u32) -> bool {
    true
}
unsafe extern "C" fn occupant_flag_set(u: *mut c_void, o: u64, f: u32) -> bool {
    game(u).o(o, |o| o.flags & f != 0)
}
unsafe extern "C" fn occupant_flags(u: *mut c_void, o: u64) -> u32 {
    game(u).o(o, |o| o.flags)
}
unsafe extern "C" fn occupant_bounds(u: *mut c_void, o: u64, b: *mut f32) {
    let (x, z) = game(u).o(o, |o| o.cell);
    let (cx, cz) = (16.0 * x as f32 + 8.0, 16.0 * z as f32 + 8.0);
    for (i, v) in [cx - 4.0, cz - 4.0, cx + 4.0, cz + 4.0].into_iter().enumerate() {
        *b.add(i) = v;
    }
}
unsafe extern "C" fn occupant_shutdown(u: *mut c_void, o: u64) {
    game(u).log(format!("shutdown {o:#x}"));
}
unsafe extern "C" fn remove_occupant(u: *mut c_void, o: u64, b: bool) -> bool {
    let g = game(u);
    let mut occs = g.occs.borrow_mut();
    let x = occs.get_mut(&o).unwrap();
    if x.flags & IN_MANAGER == 0 {
        return false;
    }
    x.flags &= !IN_MANAGER;
    drop(occs);
    g.log(format!("remove {o:#x} {b}"));
    true
}
unsafe extern "C" fn occupants_in_cell_rect(
    u: *mut c_void,
    r: *const fc4_cell_rect,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) {
    let r = &*r;
    let hits: Vec<u64> = game(u)
        .occs
        .borrow()
        .iter()
        .filter(|(_, o)| o.cell.0 >= r.x1 && o.cell.0 <= r.x2 && o.cell.1 >= r.z1 && o.cell.1 <= r.z2)
        .map(|(&t, _)| t)
        .collect();
    for t in hits {
        cb.unwrap()(ctx, t);
    }
}
unsafe extern "C" fn occupants_in_box(
    u: *mut c_void,
    x0: f32,
    z0: f32,
    x1: f32,
    z1: f32,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) {
    let hits: Vec<u64> = game(u)
        .occs
        .borrow()
        .iter()
        .filter(|(_, o)| {
            let (cx, cz) = (16.0 * o.cell.0 as f32 + 8.0, 16.0 * o.cell.1 as f32 + 8.0);
            cx - 4.0 <= x1 && cx + 4.0 >= x0 && cz - 4.0 <= z1 && cz + 4.0 >= z0
        })
        .map(|(&t, _)| t)
        .collect();
    for t in hits {
        cb.unwrap()(ctx, t);
    }
}
unsafe extern "C" fn is_network_occupant(u: *mut c_void, o: u64) -> bool {
    game(u).o(o, |o| o.ty == NETWORK)
}
unsafe extern "C" fn cost(u: *mut c_void, o: u64) -> i64 {
    game(u).o(o, |o| o.cost)
}
unsafe extern "C" fn is_power_pole(u: *mut c_void, o: u64) -> bool {
    game(u).o(o, |o| o.ty == POWER_POLE)
}
unsafe extern "C" fn pole_connections(u: *mut c_void, p: u64, cb: fc4_token_callback, ctx: *mut c_void) {
    for t in game(u).o(p, |o| o.connections.clone()) {
        cb.unwrap()(ctx, t);
    }
}
unsafe extern "C" fn pole_connection_count(u: *mut c_void, p: u64) -> u32 {
    game(u).o(p, |o| o.connections.len() as u32)
}
unsafe extern "C" fn pole_break_all_connections(u: *mut c_void, p: u64) {
    let g = game(u);
    let conns = g.o(p, |o| o.connections.clone());
    let mut occs = g.occs.borrow_mut();
    for c in conns {
        occs.get_mut(&c).unwrap().connections.retain(|&x| x != p);
    }
    occs.get_mut(&p).unwrap().connections.clear();
    drop(occs);
    g.log(format!("break {p:#x}"));
}
unsafe extern "C" fn send_pre_demolish(u: *mut c_void, o: u64) {
    game(u).log(format!("pre_demolish {o:#x}"));
}
unsafe extern "C" fn list_append(u: *mut c_void, l: u64, o: u64) {
    game(u).log(format!("append {l:#x} {o:#x}"));
}
unsafe extern "C" fn set_insert(u: *mut c_void, s: u64, o: u64) {
    game(u).log(format!("insert {s:#x} {o:#x}"));
}
unsafe extern "C" fn queue_removals(u: *mut c_void, on: bool) {
    game(u).log(format!("queue {on}"));
}
unsafe extern "C" fn delete_occupant_connection(u: *mut c_void, o: u64) {
    game(u).log(format!("delete_connection {o:#x}"));
}
unsafe extern "C" fn destroy_network_occupants(u: *mut c_void, list: *const u64, n: usize) {
    let v = std::slice::from_raw_parts(list, n);
    game(u).log(format!("destroy {v:x?}"));
}
unsafe extern "C" fn cell_corner_to_position(_: *mut c_void, x: i32, z: i32, px: *mut f32, pz: *mut f32) {
    *px = 16.0 * x as f32;
    *pz = 16.0 * z as f32;
}
unsafe extern "C" fn position_to_cell(_: *mut c_void, x: f32, z: f32, cx: *mut i32, cz: *mut i32) -> bool {
    *cx = (x / 16.0).floor() as i32;
    *cz = (z / 16.0).floor() as i32;
    true
}
unsafe extern "C" fn lots_in_cell_rect(
    u: *mut c_void,
    r: *const fc4_cell_rect,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) {
    let hits: Vec<u64> = game(u)
        .lots
        .borrow()
        .iter()
        .filter(|(_, l)| overlaps(&l.rect, &*r))
        .map(|(&t, _)| t)
        .collect();
    for t in hits {
        cb.unwrap()(ctx, t);
    }
}
unsafe extern "C" fn lot_at(u: *mut c_void, x: i32, z: i32) -> u64 {
    let p = fc4_cell_rect {
        x1: x,
        z1: z,
        x2: x,
        z2: z,
    };
    game(u)
        .lots
        .borrow()
        .iter()
        .find(|(_, l)| overlaps(&l.rect, &p))
        .map_or(0, |(&t, _)| t)
}
unsafe extern "C" fn lot_state(u: *mut c_void, l: u64) -> u32 {
    game(u).lots.borrow()[&l].state
}
unsafe extern "C" fn lot_size(u: *mut c_void, l: u64, x: *mut u8, z: *mut u8) {
    let r = game(u).lots.borrow()[&l].rect;
    *x = (r.x2 - r.x1 + 1) as u8;
    *z = (r.z2 - r.z1 + 1) as u8;
}
unsafe extern "C" fn lot_bounding_rect(u: *mut c_void, l: u64, out: *mut fc4_cell_rect) {
    *out = game(u).lots.borrow()[&l].rect;
}
unsafe extern "C" fn lot_water_compatible(_: *mut c_void, _: u64, v: *mut bool) -> bool {
    *v = false;
    true
}
unsafe extern "C" fn lot_initialized(u: *mut c_void, l: u64) -> bool {
    game(u).lots.borrow()[&l].initialized
}
unsafe extern "C" fn start_demolish_lot(u: *mut c_void, l: u64) {
    game(u).log(format!("start {l:#x}"));
}
unsafe extern "C" fn end_demolish_lot(u: *mut c_void, l: u64) {
    game(u).lots.borrow_mut().get_mut(&l).unwrap().state = 1;
    game(u).log(format!("end {l:#x}"));
}
unsafe extern "C" fn dezone(u: *mut c_void, r: *const fc4_cell_rect) {
    let r = &*r;
    game(u).log(format!("dezone {} {} {} {}", r.x1, r.z1, r.x2, r.z2));
}

fn host(g: &Game) -> fc4_demolition_host {
    fc4_demolition_host {
        struct_size: std::mem::size_of::<fc4_demolition_host>() as u32,
        user: g.user(),
        subscribe: Some(subscribe),
        unsubscribe: Some(unsubscribe),
        acquire_city: Some(acquire_city),
        release_city: Some(release_city),
        city_cell_counts: Some(city_cell_counts),
        acquire_occupant_manager: None,
        release_occupant_manager: None,
        acquire_lot_developer: None,
        release_lot_developer: None,
        acquire_terrain: Some(acquire_terrain),
        release_terrain: Some(release_terrain),
        thresholds_exemplar: Some(thresholds),
        building_development_exemplar: Some(building_development),
        release: Some(release),
        acquire_view_filter: None,
        view_level: Some(view_level),
        filter_includes: None,
        filter_includes_type: None,
        default_filter_includes: Some(yes),
        realistic_filter_includes: Some(yes),
        occupant_type: Some(occupant_type),
        occupant_position: Some(occupant_position),
        can_remove: Some(can_remove),
        occupant_flag_set: Some(occupant_flag_set),
        occupant_flags: Some(occupant_flags),
        is_occupant_group: None,
        occupant_bool_property: None,
        occupant_bounds: Some(occupant_bounds),
        foundation_slot9: None,
        network_has_any_flag: None,
        network_has_type: None,
        occupant_shutdown: Some(occupant_shutdown),
        remove_occupant: Some(remove_occupant),
        occupants_in_box: Some(occupants_in_box),
        occupants_in_cell_rect: Some(occupants_in_cell_rect),
        building_demolition_cost: None,
        flora_demolition_cost: None,
        is_network_occupant: Some(is_network_occupant),
        network_demolition_cost: Some(cost),
        is_power_pole: Some(is_power_pole),
        pole_demolition_cost: Some(cost),
        pole_connections: Some(pole_connections),
        pole_connection_count: Some(pole_connection_count),
        pole_break_all_connections: Some(pole_break_all_connections),
        send_pre_demolish: Some(send_pre_demolish),
        has_prop_manager: None,
        remove_prop: None,
        list_append: Some(list_append),
        set_insert: Some(set_insert),
        queue_removals: Some(queue_removals),
        delete_occupant_connection: Some(delete_occupant_connection),
        destroy_network_occupants: Some(destroy_network_occupants),
        cell_corner_to_position: Some(cell_corner_to_position),
        position_to_cell: Some(position_to_cell),
        lot_at: Some(lot_at),
        lots_in_cell_rect: Some(lots_in_cell_rect),
        lot_building: None,
        lot_state: Some(lot_state),
        lot_demolishable: Some(yes),
        lot_occupants: None,
        lot_size: Some(lot_size),
        lot_bounding_rect: Some(lot_bounding_rect),
        lot_facing: None,
        lot_zone_type: None,
        lot_water_compatible: Some(lot_water_compatible),
        lot_initialized: Some(lot_initialized),
        lot_vertices_locked: None,
        lot_condition: None,
        explode_lot: None,
        start_demolish_lot: Some(start_demolish_lot),
        end_demolish_lot: Some(end_demolish_lot),
        delete_lot: None,
        create_lot_if_possible: None,
        zone_at: None,
        dezone: Some(dezone),
        altitude_at_vertex: None,
        is_water_vertex: None,
        set_altitude_rect: None,
        set_altitude_vertex: None,
        redisplay_terrain: None,
        effect_extent: None,
        play_occupant_sound: None,
        start_occupant_effect: None,
        start_group_effect: None,
    }
}

fn ready(g: &Game) -> *mut fc4_demolition {
    let d = unsafe { fc4_demolition_create(&host(g), UTILITY) };
    assert!(!d.is_null());
    assert!(unsafe { fc4_demolition_init(d) });
    g.take();
    d
}

fn state(d: *const fc4_demolition) -> fc4_demolition_state {
    let mut s = fc4_demolition_state::default();
    assert!(unsafe { fc4_demolition_get_state(d, &mut s) });
    s
}

fn destroyed_lot(g: &Game, t: u64) {
    let l = Lot {
        rect: fc4_cell_rect {
            x1: 10,
            z1: 10,
            x2: 12,
            z2: 11,
        },
        state: DESTROYED,
        initialized: true,
    };
    g.lots.borrow_mut().insert(t, l);
}

#[test]
fn create_rejects_bad_tables_and_identity() {
    let g = Game::new();
    unsafe {
        assert!(fc4_demolition_create(std::ptr::null(), 0).is_null());
        let mut h = host(&g);
        h.struct_size -= 1;
        assert!(fc4_demolition_create(&h, 0).is_null());
        let mut off = 99u32;
        for (iid, want) in [(1, 0), (0x26DD_E2EA, 0), (0x4522_94AA, 4), (0xE4FD_A3D4, 8)] {
            assert!(fc4_demolition_query_interface(iid, &mut off), "{iid:08X}");
            assert_eq!(off, want);
        }
        assert!(!fc4_demolition_query_interface(0xE98B_2F57, &mut off));
        assert!(fc4_demolition_query_interface(1, std::ptr::null_mut()));
        // NULL handles are harmless.
        assert_eq!(fc4_demolition_add_ref(std::ptr::null_mut()), 0);
        assert!(!fc4_demolition_init(std::ptr::null_mut()));
        fc4_demolition_destroy(std::ptr::null_mut());
    }
}

#[test]
fn lifecycle_and_tuning() {
    let g = Game::new();
    let d = unsafe { fc4_demolition_create(&host(&g), UTILITY) };
    assert_eq!(state(d).small_medium_model, 64, "code defaults before Init");
    assert!(unsafe { fc4_demolition_init(d) });
    assert_eq!(
        g.take(),
        [
            "subscribe C53D10AA",
            "subscribe 26D31EC1",
            "subscribe A9DED179",
            "acquire city",
            "acquire terrain"
        ]
    );
    let s = state(d);
    assert_eq!(s.struct_size as usize, std::mem::size_of::<fc4_demolition_state>());
    assert_eq!(
        (s.initialized, s.filter_created, s.city_held, s.terrain_held),
        (1, 1, 1, 1)
    );
    assert_eq!((s.occupant_manager_held, s.lot_developer_held), (0, 0));
    assert_eq!((s.cells_x, s.cells_z), (64, 128));
    assert_eq!(
        (
            s.small_medium_model,
            s.medium_large_model,
            s.small_medium_group,
            s.medium_large_group
        ),
        (20, 64, 20, 100)
    );
    assert_eq!(s.destroyed_tile_cost, 2);
    // Any exemplar re-reads the tuning.
    g.tile_cost.set(Some(5));
    assert!(unsafe { fc4_demolition_do_message(d, 0xC53D_10AA, 1, 2, 0, false) });
    assert_eq!(state(d).destroyed_tile_cost, 5);
    // A missing exemplar keeps the current value.
    g.tile_cost.set(None);
    assert!(unsafe { fc4_demolition_do_message(d, 0xC53D_10AA, 0, 0, 0, false) });
    assert_eq!(state(d).destroyed_tile_cost, 5);
    unsafe {
        assert_eq!(fc4_demolition_add_ref(d), 1);
        assert_eq!(fc4_demolition_add_ref(d), 2);
        assert_eq!(fc4_demolition_release(d), 1);
        assert_eq!(fc4_demolition_release(d), 0);
    }
    assert_eq!(
        g.take(),
        [
            "release terrain",
            "release city",
            "unsubscribe C53D10AA",
            "unsubscribe 26D31EC1",
            "unsubscribe A9DED179"
        ]
    );
    let s = state(d);
    assert_eq!((s.destroyed, s.initialized), (1, 0));
    unsafe { fc4_demolition_destroy(d) };
}

#[test]
fn dm12_01_record() {
    let g = Game::new();
    let d = ready(&g);
    let mut o = Out::default();
    let os = ostream(&mut o);
    assert!(unsafe { fc4_demolition_write(d, &os) });
    assert_eq!(o.bytes, [0x01, 0x00]);
    for (bytes, ok) in [(&[0x01u8, 0x00][..], true), (&[0x02, 0x00], false), (&[0x01], false)] {
        let mut i = In::new(bytes);
        let is = istream(&mut i);
        assert_eq!(unsafe { fc4_demolition_read(d, &is) }, ok, "{bytes:x?}");
    }
    unsafe {
        assert!(!fc4_demolition_write(d, std::ptr::null()));
        assert!(!fc4_demolition_read(d, std::ptr::null()));
        fc4_demolition_destroy(d);
    }
}

#[test]
fn dm12_02_and_04_lots() {
    let g = Game::new();
    let d = ready(&g);
    unsafe {
        // An empty list, and a list of null lots: true, the cost untouched.
        let mut cost = -7i64;
        assert!(fc4_demolition_demolish_lots(
            d,
            true,
            std::ptr::null(),
            0,
            0,
            1,
            false,
            0,
            0,
            &mut cost
        ));
        let nulls = [0u64, 0];
        assert!(fc4_demolition_demolish_lots(
            d,
            true,
            nulls.as_ptr(),
            2,
            0,
            1,
            false,
            0,
            0,
            &mut cost
        ));
        assert_eq!(cost, -7);
        assert!(g.take().is_empty());
        // A destroyed 3 x 2 lot at 2 per tile: 12, dry run then committed.
        destroyed_lot(&g, 0x50);
        let lots = [0x50u64];
        assert!(fc4_demolition_demolish_lots(
            d,
            false,
            lots.as_ptr(),
            1,
            0,
            1,
            false,
            0,
            0,
            &mut cost
        ));
        assert_eq!(cost, 12);
        assert!(g.take().is_empty());
        cost = 0;
        assert!(fc4_demolition_demolish_lots(
            d,
            true,
            lots.as_ptr(),
            1,
            1,
            1,
            false,
            0,
            0,
            &mut cost
        ));
        assert_eq!(cost, 12);
        assert_eq!(g.take(), ["queue true", "start 0x50", "end 0x50", "queue false"]);
        // A NULL cost pointer is allowed.
        destroyed_lot(&g, 0x50);
        assert!(fc4_demolition_demolish_lots(
            d,
            false,
            lots.as_ptr(),
            1,
            0,
            1,
            false,
            0,
            0,
            std::ptr::null_mut()
        ));
        fc4_demolition_destroy(d);
    }
}

#[test]
fn dm12_10_power_pole_and_dm12_11_networks() {
    let g = Game::new();
    let d = ready(&g);
    g.occ(0x20, POWER_POLE, 3, 3, 10, &[0x21, 0x22]);
    g.occ(0x21, POWER_POLE, 9, 9, 0, &[0x20]);
    g.occ(0x22, POWER_POLE, 9, 10, 0, &[0x20, 0x23]);
    g.occ(0x23, POWER_POLE, 9, 11, 0, &[0x22]);
    let one = fc4_zone_region {
        x0: 3,
        z0: 3,
        x1: 3,
        z1: 3,
        mask: std::ptr::null_mut(),
        len: 0,
    };
    let mut cost = 0i64;
    assert!(unsafe { fc4_demolition_demolish_region(d, true, &one, 1, 0, false, 0, 0x5E7, 0, 0, 0, &mut cost) });
    assert_eq!(cost, 10);
    assert_eq!(
        g.take(),
        [
            "pre_demolish 0x20",
            "remove 0x20 false",
            "break 0x20",
            "shutdown 0x20",
            "insert 0x5e7 0x20",
            "queue true",
            "insert 0x5e7 0x21",
            "pre_demolish 0x21",
            "remove 0x21 true",
            "queue false"
        ]
    );
    // Directly, with a caller list.
    g.occ(0x25, POWER_POLE, 31, 31, 0, &[0x26]);
    g.occ(0x26, POWER_POLE, 32, 31, 0, &[0x25]);
    assert!(unsafe { fc4_demolition_demolish_occupant(d, true, 0x25, 1, 1, false, 0x1157, &mut cost) });
    assert_eq!(cost, 0);
    assert_eq!(
        g.take(),
        [
            "pre_demolish 0x25",
            "remove 0x25 false",
            "break 0x25",
            "shutdown 0x25",
            "append 0x1157 0x26"
        ]
    );
    // Three networks in one batch.
    for (t, x) in [(0x30u64, 1), (0x31, 2), (0x32, 3)] {
        g.occ(t, NETWORK, x, 7, 5, &[]);
    }
    let row = fc4_zone_region {
        x0: 1,
        z0: 7,
        x1: 3,
        z1: 7,
        mask: std::ptr::null_mut(),
        len: 0,
    };
    assert!(unsafe { fc4_demolition_demolish_region(d, true, &row, 1, 0, false, 0, 0, 0, 0, 0, &mut cost) });
    assert_eq!(cost, 15);
    assert_eq!(
        g.take(),
        [
            "queue true",
            "delete_connection 0x30",
            "delete_connection 0x31",
            "delete_connection 0x32",
            "destroy [30, 31, 32]",
            "queue false"
        ]
    );
    // UpdateNetworks over a set; a NULL region is refused.
    g.occ(0x33, NETWORK, 9, 9, 0, &[]);
    unsafe {
        assert!(!fc4_demolition_demolish_region(
            d,
            true,
            std::ptr::null(),
            1,
            0,
            false,
            0,
            0,
            0,
            0,
            0,
            &mut cost
        ));
        fc4_demolition_update_networks(d, std::ptr::null(), 0);
        fc4_demolition_destroy(d);
    }
}

#[test]
fn deferred_lot_demolition_message() {
    let g = Game::new();
    let d = ready(&g);
    destroyed_lot(&g, 0x50);
    assert!(unsafe { fc4_demolition_do_message(d, 0xA9DE_D179, 0, 0, 0x50, true) });
    assert_eq!(g.take(), ["end 0x50", "dezone 10 10 12 11", "release 0x50"]);
    g.lots.borrow_mut().get_mut(&0x50).unwrap().initialized = false;
    assert!(unsafe { fc4_demolition_do_message(d, 0xA9DE_D179, 0, 0, 0x50, false) });
    assert_eq!(g.take(), ["release 0x50"]);
    assert!(unsafe { fc4_demolition_do_message(d, 0xA9DE_D179, 0, 0, 0, true) });
    assert!(g.take().is_empty());
    // Terrain with no terrain callbacks: nothing changes.
    assert_eq!(
        unsafe { fc4_demolition_modify_terrain_height(d, false, false, 1, 1, 2, 2, 0.0, 0, 0) },
        0
    );
    unsafe { fc4_demolition_destroy(d) };
}
