// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The network manager and the network lot manager through the C ABI
//! (docs/specs/network-manager.md): a fake game behind `fc4_netmgr_host`
//! and `fc4_netlot_host`.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::c_void;

use crate::budget::fc4_token_callback;
use crate::demand::fc4_u32_callback;
use crate::hostgrid::fc4_cell_rect;
use crate::network_manager::*;
use crate::props::{fc4_props_create, fc4_props_destroy, fc4_props_set_u32s};
use crate::stream::fc4_ref;
use crate::test_streams::{istream, ostream, In, Out};

const ROAD: u32 = 0x0001;
const PIPE: u32 = 0x0010;
const USABLE: u32 = 0x1000_0000;
const TRANSLUCENT: u32 = 0x4000;

const OCCUPANT_INSERTED: u32 = 0x99EF_1142;
const OCCUPANT_REMOVED: u32 = 0x99EF_1143;
const DUSK: u32 = 0xC9DA_96E9;
const DAWN: u32 = 0xC9DA_96E8;
const PRE_CITY_SHUTDOWN: u32 = 0x26D3_1EC2;

const MANAGER_SUBSCRIPTIONS: usize = 11;
const LOT_SUBSCRIPTIONS: usize = 6;

/// A network occupant of the fake city.
#[derive(Clone, Copy, Default)]
struct Occ {
    flags: u32,
    managed: bool,
    lit: bool,
    cell: (i32, i32),
    creation: u64,
    piece: u32,
}

#[derive(Default)]
struct Game {
    occs: RefCell<BTreeMap<u64, Occ>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    subs: RefCell<Vec<u32>>,
    unsubs: RefCell<Vec<u32>>,
    tools: RefCell<BTreeMap<u64, u32>>,
    next: Cell<u64>,
    destroyed: RefCell<Vec<(u32, Vec<u64>)>>,
    lit: RefCell<Vec<(u64, bool)>>,
    zones: RefCell<BTreeMap<(i32, i32), i32>>,
    slope: Cell<f32>,
    object_tokens: RefCell<Vec<u64>>,
}

impl Game {
    fn new() -> Box<Game> {
        let g = Box::new(Game::default());
        g.next.set(0x7000);
        g
    }
    fn user(&self) -> *mut c_void {
        self as *const Game as *mut c_void
    }
    fn refs(&self, t: u64) -> i64 {
        self.refs.borrow().get(&t).copied().unwrap_or(0)
    }
    fn bump(&self, t: u64, d: i64) {
        *self.refs.borrow_mut().entry(t).or_default() += d;
    }
    fn add(&self, t: u64, o: Occ) {
        self.occs.borrow_mut().insert(t, o);
    }
    fn occ(&self, t: u64) -> Occ {
        self.occs.borrow().get(&t).copied().unwrap_or_default()
    }
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

// ---- shared callbacks ----

unsafe extern "C" fn subscribe(u: *mut c_void, m: u32) {
    game(u).subs.borrow_mut().push(m);
}
unsafe extern "C" fn unsubscribe(u: *mut c_void, m: u32) {
    game(u).unsubs.borrow_mut().push(m);
}
unsafe extern "C" fn city_size(_: *mut c_void, x: *mut i32, z: *mut i32) {
    *x = 64;
    *z = 64;
}
unsafe extern "C" fn add_ref(u: *mut c_void, t: u64) {
    game(u).bump(t, 1);
}
unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    game(u).bump(t, -1);
}
unsafe extern "C" fn resolve_ref(u: *mut c_void, t: u64, _iid: u32) -> u64 {
    let g = game(u);
    if g.occs.borrow().contains_key(&t) {
        g.bump(t, 1);
        t
    } else {
        0
    }
}
unsafe extern "C" fn query_network_occupant(u: *mut c_void, o: u64) -> u64 {
    let g = game(u);
    if g.occ(o).flags & 0x1FFF != 0 {
        g.bump(o, 1);
        o
    } else {
        0
    }
}
unsafe extern "C" fn network_flags(u: *mut c_void, n: u64) -> u32 {
    game(u).occ(n).flags
}
unsafe extern "C" fn occupied_cell(u: *mut c_void, n: u64, x: *mut i32, z: *mut i32) {
    let (a, b) = game(u).occ(n).cell;
    *x = a;
    *z = b;
}

// ---- the network manager's callbacks ----

unsafe extern "C" fn create_tool(u: *mut c_void, ty: u32) -> u64 {
    let g = game(u);
    let t = g.next.get();
    g.next.set(t + 1);
    g.tools.borrow_mut().insert(t, ty);
    g.bump(t, 1);
    t
}
unsafe extern "C" fn tool_destroy_occupants(u: *mut c_void, tool: u64, list: *const u64, n: usize) {
    let g = game(u);
    let ty = g.tools.borrow()[&tool];
    let v = std::slice::from_raw_parts(list, n).to_vec();
    g.destroyed.borrow_mut().push((ty, v));
}
unsafe extern "C" fn is_of_type(u: *mut c_void, n: u64, ty: u32) -> bool {
    game(u).occ(n).flags & (1 << ty) != 0
}
unsafe extern "C" fn is_managed_globally(u: *mut c_void, n: u64) -> bool {
    game(u).occ(n).managed
}
unsafe extern "C" fn can_be_lit(u: *mut c_void, n: u64) -> bool {
    game(u).occ(n).lit
}
unsafe extern "C" fn set_lit(u: *mut c_void, n: u64, lit: bool) {
    game(u).lit.borrow_mut().push((n, lit));
}
unsafe extern "C" fn creation_number(u: *mut c_void, o: u64) -> u64 {
    game(u).occ(o).creation
}
unsafe extern "C" fn occupants_in_rect(
    u: *mut c_void,
    r: *const fc4_cell_rect,
    mask: u32,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) {
    let r = *r;
    let hits: Vec<u64> = game(u)
        .occs
        .borrow()
        .iter()
        .filter(|(_, o)| {
            let (x, z) = o.cell;
            x >= r.x1 && x <= r.x2 && z >= r.z1 && z <= r.z2 && o.flags & mask != 0
        })
        .map(|(t, _)| *t)
        .collect();
    if let Some(f) = cb {
        for t in hits {
            f(ctx, t);
        }
    }
}

fn manager_host(g: &Game) -> fc4_netmgr_host {
    fc4_netmgr_host {
        struct_size: std::mem::size_of::<fc4_netmgr_host>() as u32,
        user: g.user(),
        subscribe: Some(subscribe),
        unsubscribe: Some(unsubscribe),
        uk_paths: None,
        city_size: Some(city_size),
        is_night: None,
        view_rotation_changed: None,
        add_ref: Some(add_ref),
        release: Some(release),
        resolve_ref: Some(resolve_ref),
        create_tool: Some(create_tool),
        tool_init: None,
        tool_shutdown: None,
        tool_destroy_occupants: Some(tool_destroy_occupants),
        tool_update_underground_heights: None,
        tool_cell_info: None,
        tool_reinsert_piece: None,
        query_network_occupant: Some(query_network_occupant),
        is_network_occupant: None,
        network_flags: Some(network_flags),
        is_of_type: Some(is_of_type),
        occupant_flags: None,
        occupant_flag_set: None,
        occupied_cell: Some(occupied_cell),
        occupant_altitude: None,
        is_managed_globally: Some(is_managed_globally),
        can_be_lit: Some(can_be_lit),
        set_lit: Some(set_lit),
        check_for_retrofit: None,
        retrofit: None,
        creation_number: Some(creation_number),
        remove_occupant: None,
        occupants_in_rect: Some(occupants_in_rect),
        network_occupants: None,
        post_refresh: None,
        terrain_altitude: None,
        crew_shutdown: None,
    }
}

fn new_manager(g: &Game) -> *mut fc4_netmgr {
    let host = manager_host(g);
    let m = unsafe { fc4_netmgr_create(&host, 0xA000) };
    assert!(!m.is_null());
    m
}

fn managed(m: *const fc4_netmgr) -> Vec<u64> {
    unsafe {
        let n = fc4_netmgr_managed(m, std::ptr::null_mut(), 0);
        let mut v = vec![0u64; n];
        assert_eq!(fc4_netmgr_managed(m, v.as_mut_ptr(), n), n);
        v
    }
}

fn state(m: *const fc4_netmgr) -> fc4_netmgr_state {
    let mut s = fc4_netmgr_state::default();
    assert!(unsafe { fc4_netmgr_get_state(m, &mut s) });
    s
}

fn message(m: *mut fc4_netmgr, ty: u32, void1: u64) {
    assert!(unsafe { fc4_netmgr_do_message(m, ty, 0, 0, 0, 0, void1, 0, 0) });
}

fn bridge(creation: u64) -> Occ {
    Occ {
        flags: ROAD | USABLE,
        managed: true,
        lit: true,
        cell: (1, 1),
        creation,
        ..Occ::default()
    }
}

fn road(cell: (i32, i32), creation: u64) -> Occ {
    Occ {
        flags: ROAD | USABLE,
        cell,
        creation,
        ..Occ::default()
    }
}

fn save_manager(m: *const fc4_netmgr) -> Out {
    let mut o = Out::default();
    let s = ostream(&mut o);
    assert!(unsafe { fc4_netmgr_write(m, &s) });
    o
}

// ---- the network manager ----

#[test]
fn create_rejects_a_short_host_and_null_handles_fall_back() {
    let g = Game::new();
    let mut host = manager_host(&g);
    host.struct_size -= 4;
    unsafe {
        assert!(fc4_netmgr_create(&host, 1).is_null());
        assert!(fc4_netmgr_create(std::ptr::null(), 1).is_null());
        assert!(!fc4_netmgr_init(std::ptr::null_mut()));
        assert_eq!(fc4_netmgr_get_network_tool(std::ptr::null_mut(), 0, false), 0);
        assert_eq!(fc4_netmgr_managed(std::ptr::null(), std::ptr::null_mut(), 0), 0);
        assert!(fc4_netmgr_do_message(std::ptr::null_mut(), DUSK, 0, 0, 0, 0, 0, 0, 0));
        fc4_netmgr_destroy(std::ptr::null_mut());
    }
}

#[test]
fn identity() {
    assert!(fc4_netmgr_query_interface(0xA976_4C8C));
    assert!(fc4_netmgr_query_interface(0x0000_0001));
    assert!(!fc4_netmgr_query_interface(0x89A5_BA3F));
    assert!(fc4_netlot_query_interface(0x89A5_BA3F));
    assert!(!fc4_netlot_query_interface(0xA976_4C8C));
}

#[test]
fn init_subscribes_and_creates_the_tools_shutdown_releases_them() {
    let g = Game::new();
    let m = new_manager(&g);
    unsafe {
        assert!(fc4_netmgr_init(m));
        assert_eq!(g.subs.borrow().len(), MANAGER_SUBSCRIPTIONS);
        assert_eq!(state(m).tools, 13);
        assert_eq!(
            *g.tools.borrow().values().copied().collect::<Vec<_>>(),
            (0..13).collect::<Vec<u32>>()
        );
        // §7.1: the cached tool, borrowed; a unique one is new; type 13 has none.
        let t5 = fc4_netmgr_get_network_tool(m, 5, false);
        assert_eq!(g.tools.borrow()[&t5], 5);
        assert_eq!(fc4_netmgr_get_network_tool(m, 5, false), t5);
        assert_eq!(g.refs(t5), 1);
        let u = fc4_netmgr_get_network_tool(m, 5, true);
        assert_ne!(u, t5);
        assert_eq!(g.refs(u), 1);
        assert_eq!(fc4_netmgr_get_network_tool(m, 13, false), 0);
        assert!(fc4_netmgr_shutdown(m));
        assert_eq!(g.unsubs.borrow().len(), MANAGER_SUBSCRIPTIONS);
        assert_eq!(g.refs(t5), 0);
        assert_eq!(state(m).tools, 0);
        assert_eq!(fc4_netmgr_add_ref(m), 1);
        assert_eq!(fc4_netmgr_add_ref(m), 2);
        assert_eq!(fc4_netmgr_release(m), 1);
        assert_eq!(fc4_netmgr_release(m), 0);
        assert_eq!(state(m).destroyed, 1);
        fc4_netmgr_destroy(m);
    }
}

#[test]
fn managed_list_swap_and_pop_record_round_trip_and_read_appends() {
    // §16 tests 3 and 1.
    let g = Game::new();
    let m = new_manager(&g);
    unsafe {
        fc4_netmgr_init(m);
        let empty = save_manager(m);
        assert_eq!(empty.bytes, [1, 0, 0, 0, 0, 0]);
        for (t, c) in [(11, 1), (12, 2), (13, 3)] {
            g.add(t, bridge(c));
            message(m, OCCUPANT_INSERTED, t);
        }
        g.add(14, road((3, 3), 4));
        message(m, OCCUPANT_INSERTED, 14);
        assert_eq!(g.refs(14), 0, "a plain road is not kept");
        message(m, OCCUPANT_REMOVED, 11);
        assert_eq!(managed(m), [13, 12]);
        assert_eq!(g.refs(11), 0);
        assert_eq!(state(m).managed, 2);

        let rec = save_manager(m);
        let mut want = vec![1, 0, 2, 0, 0, 0];
        for t in [13u64, 12] {
            want.extend_from_slice(&t.to_le_bytes());
            want.extend_from_slice(&0u32.to_le_bytes());
        }
        assert_eq!(rec.bytes, want);
        assert_eq!(rec.refs, [(13, 0), (12, 0)]);

        // Read appends; each entry comes back through resolve_ref with a reference.
        let mut i = In::new(&rec.bytes);
        let s = istream(&mut i);
        assert!(fc4_netmgr_read(m, &s));
        assert_eq!(i.pos, rec.bytes.len());
        assert_eq!(managed(m), [13, 12, 13, 12]);
        assert_eq!(g.refs(13), 2);

        // A fresh manager reads the same record and writes the same bytes.
        let m2 = new_manager(&g);
        let mut i = In::new(&rec.bytes);
        let s = istream(&mut i);
        assert!(fc4_netmgr_read(m2, &s));
        assert_eq!(save_manager(m2).bytes, rec.bytes);
        // Version 2 is rejected.
        let mut i = In::new(&[2, 0, 0, 0, 0, 0]);
        let s = istream(&mut i);
        assert!(!fc4_netmgr_read(m2, &s));
        // A reference that does not resolve fails the read.
        let mut bad = vec![1, 0, 1, 0, 0, 0];
        bad.extend_from_slice(&0x99u64.to_le_bytes());
        bad.extend_from_slice(&0u32.to_le_bytes());
        let mut i = In::new(&bad);
        let s = istream(&mut i);
        assert!(!fc4_netmgr_read(m2, &s));
        fc4_netmgr_destroy(m2);
        fc4_netmgr_destroy(m);
    }
}

#[test]
fn read_resolves_through_the_stream_token() {
    // The stream hands the token back; resolve_ref sees the token and the IID.
    let g = Game::new();
    g.add(21, bridge(1));
    let m = new_manager(&g);
    let mut bytes = vec![1, 0, 1, 0, 0, 0];
    bytes.extend_from_slice(&21u64.to_le_bytes());
    bytes.extend_from_slice(&7u32.to_le_bytes());
    let seen = RefCell::new(Vec::new());
    let hook = |r: &mut fc4_ref| seen.borrow_mut().push((r.token, r.class_id));
    let mut i = In::new(&bytes);
    i.resolve = Some(&hook);
    let s = istream(&mut i);
    unsafe {
        assert!(fc4_netmgr_read(m, &s));
        assert_eq!(managed(m), [21]);
        fc4_netmgr_destroy(m);
    }
    assert_eq!(*seen.borrow(), [(21, 7)]);
}

#[test]
fn dusk_and_dawn_light_the_managed_pieces() {
    // §16 test 6.
    let g = Game::new();
    let m = new_manager(&g);
    unsafe {
        fc4_netmgr_init(m);
        g.add(11, bridge(1));
        g.add(
            12,
            Occ {
                lit: false,
                ..bridge(2)
            },
        );
        message(m, OCCUPANT_INSERTED, 11);
        message(m, OCCUPANT_INSERTED, 12);
        message(m, DUSK, 0);
        message(m, DAWN, 0);
        assert_eq!(*g.lit.borrow(), [(11, true), (11, false)]);
        fc4_netmgr_destroy(m);
    }
}

#[test]
fn queue_removals_dedupes_in_creation_order() {
    // §16 test 8.
    let g = Game::new();
    let m = new_manager(&g);
    unsafe {
        fc4_netmgr_init(m);
        g.add(21, road((1, 1), 2));
        g.add(22, road((2, 2), 1));
        fc4_netmgr_queue_removals(m, true);
        fc4_netmgr_destroy_network_occupants(m, [21u64, 22].as_ptr(), 2);
        fc4_netmgr_destroy_network_occupants(m, [21u64].as_ptr(), 1);
        assert!(g.destroyed.borrow().is_empty());
        assert_eq!(state(m).pending, 3, "pending keeps every request; the flush dedupes");
        assert_eq!(state(m).queueing, 1);
        fc4_netmgr_queue_removals(m, false);
        assert_eq!(*g.destroyed.borrow(), [(0, vec![22, 21])]);
        assert_eq!(state(m).pending, 0);
        fc4_netmgr_destroy_network_occupants(m, std::ptr::null(), 0);
        assert_eq!(g.destroyed.borrow().len(), 1, "an empty list destroys nothing");
        fc4_netmgr_destroy(m);
    }
}

#[test]
fn terrain_message_destroys_floating_networks_and_keeps_pipes() {
    // §7.6: kind 0, a road buried by the edit is destroyed; a pipe survives.
    let g = Game::new();
    let m = new_manager(&g);
    unsafe extern "C" fn altitude(u: *mut c_void, n: u64, _: f32, _: f32) -> f32 {
        if game(u).occ(n).piece == 1 {
            -1.0
        } else {
            0.0
        }
    }
    let mut host = manager_host(&g);
    host.occupant_altitude = Some(altitude);
    let m2 = unsafe { fc4_netmgr_create(&host, 0xA001) };
    unsafe {
        fc4_netmgr_destroy(m);
        fc4_netmgr_init(m2);
        g.add(
            31,
            Occ {
                piece: 1,
                ..road((5, 5), 1)
            },
        );
        g.add(32, road((6, 5), 2));
        g.add(
            33,
            Occ {
                flags: PIPE,
                piece: 1,
                cell: (5, 6),
                creation: 3,
                ..Occ::default()
            },
        );
        // Cells x 5..6, z 5..6: data1 = (6 << 16) | 6, data2 likewise.
        let d = (6 << 16) | 6;
        assert!(fc4_netmgr_do_message(m2, 0x278E_6003, d, d, 0, 0, 0, 0, 0));
        assert_eq!(*g.destroyed.borrow(), [(0, vec![31])]);
        assert_eq!(
            state(m2).dirty,
            fc4_cell_rect {
                x1: 0x7FFF_FFFF,
                z1: 0x7FFF_FFFF,
                x2: -1,
                z2: -1
            },
            "the refit ran and emptied dirty"
        );
        fc4_netmgr_destroy(m2);
    }
}

#[test]
fn pre_city_shutdown_drops_the_tools() {
    let g = Game::new();
    let m = new_manager(&g);
    unsafe {
        fc4_netmgr_init(m);
        message(m, PRE_CITY_SHUTDOWN, 0);
        assert_eq!(state(m).tools, 0);
        assert_eq!(fc4_netmgr_get_network_tool(m, 0, false), 0);
        fc4_netmgr_destroy(m);
    }
}

#[test]
fn flag_rules() {
    // §5 and §16 test 22.
    let mut c = fc4_network_flag_change::default();
    unsafe {
        assert!(fc4_network_set_flag(ROAD, USABLE | TRANSLUCENT, true, &mut c));
        assert_eq!(
            c,
            fc4_network_flag_change {
                flags: ROAD | USABLE | TRANSLUCENT,
                usable_post: 1,
                translucency: 1,
                reserved: [0; 3]
            }
        );
        assert!(fc4_network_set_flag(ROAD, USABLE, false, &mut c));
        assert_eq!(c.usable_post, -1, "not in the city: no message");
        assert!(fc4_network_set_flag(ROAD | USABLE, USABLE, true, &mut c));
        assert_eq!(c.usable_post, -1, "already usable");
        assert!(fc4_network_clear_flag(ROAD | USABLE, USABLE, &mut c));
        assert_eq!((c.flags, c.usable_post), (ROAD, 0));
        assert!(fc4_network_clear_flag(ROAD, USABLE, &mut c));
        assert_eq!(c.usable_post, -1);
        assert!(!fc4_network_set_flag(0, 0, true, std::ptr::null_mut()));
    }
    assert_eq!(fc4_network_pipe_model(PIPE), 0);
    assert_eq!(fc4_network_pipe_model(PIPE | 0x1_0000), 1);
    assert_eq!(fc4_network_pipe_model(PIPE | 0x2_0000), 2);
}

#[test]
fn terrain_rect_and_corner_rule() {
    // §16 tests 10 and 11.
    let mut r = fc4_cell_rect::default();
    unsafe {
        assert!(fc4_network_terrain_rect(0x0006_0009, 0x0000_000F, 64, 64, &mut r));
    }
    assert_eq!(
        r,
        fc4_cell_rect {
            x1: 5,
            z1: 0,
            x2: 9,
            z2: 15
        }
    );
    unsafe {
        assert!(fc4_network_terrain_rect(0x0001_00FF, 0x0001_00FF, 64, 64, &mut r));
    }
    assert_eq!((r.x2, r.z2), (63, 63));
    assert!(!fc4_network_corner_mismatch(0.005));
    assert!(fc4_network_corner_mismatch(0.02));
    assert!(fc4_network_corner_mismatch(-0.001));
    assert!(!fc4_network_corner_mismatch(0.0));
    assert!(!fc4_network_corner_mismatch(f32::NAN));
}

// ---- the network lot manager ----

unsafe extern "C" fn zone_type(u: *mut c_void, x: i32, z: i32) -> i32 {
    game(u).zones.borrow().get(&(x, z)).copied().unwrap_or(0)
}
unsafe extern "C" fn area_slope(u: *mut c_void, _: i32, _: i32) -> f32 {
    game(u).slope.get()
}
unsafe extern "C" fn piece_id(u: *mut c_void, n: u64) -> u32 {
    game(u).occ(n).piece
}
unsafe extern "C" fn object_ids(u: *mut c_void, token: u64, _: u32, _: fc4_u32_callback, _: *mut c_void) {
    game(u).object_tokens.borrow_mut().push(token);
}

unsafe extern "C" fn find_lot_test_tile(u: *mut c_void, x: i32, z: i32, flags: u32) -> u64 {
    game(u)
        .occs
        .borrow()
        .iter()
        .find_map(|(&token, o)| (o.cell == (x, z) && o.flags & flags != 0).then_some(token))
        .unwrap_or(0)
}

fn lot_host(g: &Game) -> fc4_netlot_host {
    fc4_netlot_host {
        struct_size: std::mem::size_of::<fc4_netlot_host>() as u32,
        user: g.user(),
        subscribe: Some(subscribe),
        unsubscribe: Some(unsubscribe),
        add_ref: Some(add_ref),
        release: Some(release),
        resolve_ref: Some(resolve_ref),
        query_network_occupant: Some(query_network_occupant),
        network_flags: Some(network_flags),
        piece_id: Some(piece_id),
        rotation: None,
        flip: None,
        position_cell: Some(occupied_cell),
        occupied_cell: Some(occupied_cell),
        occupant_altitude: None,
        is_intersection: None,
        edge_types_flag: None,
        active_edges: None,
        set_network_flag: None,
        set_under_texture: None,
        set_variation: None,
        traffic_map_exists: None,
        find_network_tile: None,
        update_intersection: None,
        city_size: Some(city_size),
        cell_center: None,
        terrain_altitude: None,
        zone_type: Some(zone_type),
        lot_at: None,
        lot_wealth: None,
        lot_is_livable: None,
        lot_building_type: None,
        building_profile_wealth: None,
        query_lot: None,
        lot_bounding_rect: None,
        area_slope: Some(area_slope),
        remove_street_props: None,
        add_street_prop: None,
        street_prop_slot28: None,
        street_prop_slot29: None,
        demolish_flora: None,
        add_flora: None,
        flora_slot28: None,
        object_ids: Some(object_ids),
        create_effect_prop: None,
        effect_set_name: None,
        effect_start: None,
        effect_stop: None,
        set_position: None,
        insert_occupant: None,
        remove_occupant: None,
        first_network_occupant_at: None,
    }
}

fn new_lots(g: &Game) -> *mut fc4_netlot {
    let host = lot_host(g);
    let l = unsafe { fc4_netlot_create(&host, 0xB000, 0x1234_5678) };
    assert!(!l.is_null());
    l
}

fn config(id: u32, piece: u32, mask: [u8; 4]) -> fc4_netlot_config {
    fc4_netlot_config {
        struct_size: std::mem::size_of::<fc4_netlot_config>() as u32,
        piece_id: piece,
        token: u64::from(id),
        id,
        zones: 0xFFFF,
        wealths: 0x0F,
        period: 4,
        mask,
        rotations: 0x0F,
        flip_rule: 0,
        reserved: 0,
        min_slope: 0.0,
        max_slope: 64.0,
        object_count: 0,
        objects: std::ptr::null(),
    }
}

fn lot_state(l: *const fc4_netlot) -> fc4_netlot_state {
    let mut s = fc4_netlot_state::default();
    assert!(unsafe { fc4_netlot_get_state(l, &mut s) });
    s
}

fn save_lots(l: *const fc4_netlot) -> Out {
    let mut o = Out::default();
    let s = ostream(&mut o);
    assert!(unsafe { fc4_netlot_write(l, &s) });
    o
}

#[test]
fn forced_configuration_uses_exact_token_when_ids_collide() {
    let g = Game::new();
    g.add(
        1,
        Occ {
            piece: 0x77,
            ..road((1, 1), 1)
        },
    );
    let mut host = lot_host(&g);
    host.find_network_tile = Some(find_lot_test_tile);
    unsafe {
        let l = fc4_netlot_create(&host, 0, 1);
        assert!(!l.is_null());
        let objects = [fc4_netlot_object {
            kind: 1,
            ..fc4_netlot_object::default()
        }];
        let mut first = config(42, 0x77, [0x0F; 4]);
        first.token = 100;
        first.object_count = 1;
        first.objects = objects.as_ptr();
        let mut second = config(42, 0x78, [0x0F; 4]);
        second.token = 200;
        second.object_count = 1;
        second.objects = objects.as_ptr();
        assert!(fc4_netlot_add_configuration(l, &first));
        assert!(fc4_netlot_add_configuration(l, &second));
        assert!(!fc4_netlot_set_next_configuration_token(l, 200));
        assert!(fc4_netlot_init(l));
        for token in [200, 100, 200] {
            assert!(fc4_netlot_set_next_configuration_token(l, token));
            assert!(!fc4_netlot_set_next_configuration_token(l, 999));
            fc4_netlot_update_roads_in_cell(l, 1, 1);
            assert_eq!(g.object_tokens.borrow().last(), Some(&token));
            assert_eq!(lot_state(l).has_forced, 0, "forced choice is consumed");
            fc4_netlot_do_message(l, OCCUPANT_REMOVED, 0, 0, 0, 1, 0, std::ptr::null());
        }
        assert!(fc4_netlot_set_next_configuration_token(l, 200));
        fc4_netlot_clear_next_configuration(l);
        assert_eq!(lot_state(l).has_forced, 0);
        fc4_netlot_destroy(l);
        assert_eq!(g.refs(1), 0);
        assert!(!fc4_netlot_set_next_configuration_token(std::ptr::null_mut(), 100));
    }
}

#[test]
fn lot_manager_init_loads_configurations_and_the_empty_record() {
    // §16 test 2 and §3.2.
    let g = Game::new();
    let l = new_lots(&g);
    unsafe {
        let objects = [fc4_netlot_object {
            kind: 1,
            offset: [1.0, 0.0, 2.0],
            ..fc4_netlot_object::default()
        }];
        let mut c = config(0x501, 0x77, [0x0F; 4]);
        c.object_count = 1;
        c.objects = objects.as_ptr();
        assert!(fc4_netlot_add_configuration(l, &c));
        assert!(fc4_netlot_add_configuration(l, &config(0x502, 0x77, [0x0F; 4])));
        let mut short = config(0x503, 0x78, [0x0F; 4]);
        short.struct_size -= 4;
        assert!(!fc4_netlot_add_configuration(l, &short));
        assert!(!fc4_netlot_configuration_by_id(l, 0x501), "loaded only at Init");
        assert!(fc4_netlot_init(l));
        assert_eq!(g.subs.borrow().len(), LOT_SUBSCRIPTIONS);
        assert!(fc4_netlot_configuration_by_id(l, 0x501));
        assert!(fc4_netlot_configuration_by_id(l, 0x502));
        assert_eq!(lot_state(l).configs, 2);
        assert_eq!(save_lots(l).bytes, [2, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        // Version 1: nothing else is read.
        let mut i = In::new(&[1, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        let s = istream(&mut i);
        assert!(!fc4_netlot_read(l, &s));
        assert_eq!(i.pos, 2);
        // Slot 13.
        assert!(fc4_netlot_set_next_configuration(l, 0x502));
        assert_eq!((lot_state(l).has_forced, lot_state(l).forced_id), (1, 0x502));
        assert!(!fc4_netlot_set_next_configuration(l, 0x999));
        fc4_netlot_clear_next_configuration(l);
        assert_eq!(lot_state(l).has_forced, 0);
        assert!(fc4_netlot_shutdown(l));
        assert_eq!(g.unsubs.borrow().len(), LOT_SUBSCRIPTIONS);
        fc4_netlot_destroy(l);
    }
}

#[test]
fn configuration_from_properties() {
    // §2.4: property 0x10 must be 0x21 and the piece ID present.
    let g = Game::new();
    let l = new_lots(&g);
    unsafe {
        let p = fc4_props_create();
        assert!(fc4_props_set_u32s(p, 0xC9A5_A1BE, &0x77, 1));
        assert!(fc4_props_set_u32s(p, 0x21, &0x601, 1));
        assert!(
            !fc4_netlot_add_configuration_props(l, 1, p, std::ptr::null(), 0),
            "no exemplar type"
        );
        assert!(fc4_props_set_u32s(p, 0x10, &0x21, 1));
        assert!(fc4_netlot_add_configuration_props(l, 1, p, std::ptr::null(), 0));
        fc4_props_destroy(p);
        fc4_netlot_init(l);
        assert!(fc4_netlot_configuration_by_id(l, 0x601));
        // Without a zone or wealth list it allows nothing.
        g.slope.set(1.0);
        assert!(!fc4_netlot_is_lot_compatible(l, 0x601, 0, 0, 1, 1, 0, 0));
        fc4_netlot_destroy(l);
    }
}

#[test]
fn zone_for_network_and_compatibility() {
    // §16 tests 14 and 16.
    let g = Game::new();
    let l = new_lots(&g);
    unsafe {
        assert!(fc4_netlot_add_configuration(l, &config(0x501, 0x77, [0x01, 0, 0, 0])));
        fc4_netlot_init(l);
        {
            let mut z = g.zones.borrow_mut();
            z.insert((9, 10), 1); // W: R$ (P 4)
            z.insert((11, 10), 9); // E: I-HT (P 1)
            z.insert((20, 20), 5);
        }
        assert_eq!(fc4_netlot_zone_type_for_network(l, 10, 10), 9);
        assert_eq!(fc4_netlot_zone_type_for_network(l, 20, 20), 5, "a zoned cell");
        assert_eq!(fc4_netlot_zone_type_for_network(l, 40, 40), 0);
        // Mask row 0 = bit 0: only x ≡ 0 and z ≡ 0 (mod 4).
        assert!(fc4_netlot_is_lot_compatible(l, 0x501, 4, 8, 1, 1, 0, 0));
        assert!(!fc4_netlot_is_lot_compatible(l, 0x501, 5, 8, 1, 1, 0, 0));
        assert!(!fc4_netlot_is_lot_compatible(l, 0x501, 4, 9, 1, 1, 0, 0));
        assert!(!fc4_netlot_is_lot_compatible(l, 0x501, 4, 8, 1, 1, 5, 0), "rotation 5");
        g.slope.set(65.0);
        assert!(!fc4_netlot_is_lot_compatible(l, 0x501, 4, 8, 1, 1, 0, 0), "too steep");
        g.slope.set(f32::NAN);
        assert!(!fc4_netlot_is_lot_compatible(l, 0x501, 4, 8, 1, 1, 0, 0), "NaN slope");
        assert!(!fc4_netlot_is_lot_compatible(l, 0x999, 4, 8, 1, 1, 0, 0), "not loaded");
        fc4_netlot_destroy(l);
    }
}

#[test]
fn lot_record_round_trip_through_init() {
    // §4.2 and §3.2 step 5: a lot read from the save keeps its configuration
    // ID; Init attaches the configuration, or drops the lot when none is loaded.
    let g = Game::new();
    g.add(
        0x31,
        Occ {
            piece: 0x77,
            ..road((3, 4), 1)
        },
    );
    g.add(
        0x32,
        Occ {
            piece: 0x77,
            ..road((5, 6), 2)
        },
    );
    let mut bytes = vec![2, 0, 2, 0, 0, 0];
    for (key, occ, id) in [(0x0304u16, 0x31u64, 0x501u32), (0x0506, 0x32, 0x999)] {
        bytes.extend_from_slice(&key.to_le_bytes());
        bytes.extend_from_slice(&occ.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&id.to_le_bytes());
    }
    bytes.extend_from_slice(&[0, 0, 0, 0]);
    let l = new_lots(&g);
    unsafe {
        let mut i = In::new(&bytes);
        let s = istream(&mut i);
        assert!(fc4_netlot_read(l, &s));
        assert_eq!(i.pos, bytes.len());
        let mut lot = fc4_netlot_lot::default();
        assert!(fc4_netlot_get_lot(l, 3, 4, &mut lot));
        assert_eq!(
            lot,
            fc4_netlot_lot {
                occupant: 0x31,
                config_id: 0x501,
                has_config: 0,
                locked: 0,
                reserved: 0
            }
        );
        assert!(fc4_netlot_add_configuration(l, &config(0x501, 0x77, [0x0F; 4])));
        fc4_netlot_init(l);
        assert!(fc4_netlot_get_lot(l, 3, 4, &mut lot));
        assert_eq!((lot.has_config, lot.config_id), (1, 0x501));
        assert!(
            !fc4_netlot_get_lot(l, 5, 6, std::ptr::null_mut()),
            "no configuration 0x999: dropped"
        );
        assert_eq!(lot_state(l).lots, 1);

        // Visiting.
        unsafe extern "C" fn visit(ctx: *mut c_void, x: i32, z: i32, lot: *const fc4_netlot_lot) -> bool {
            (*(ctx as *mut Vec<(i32, i32, u64)>)).push((x, z, (*lot).occupant));
            true
        }
        let mut seen: Vec<(i32, i32, u64)> = Vec::new();
        assert!(fc4_netlot_iterate_lots(
            l,
            Some(visit),
            &mut seen as *mut Vec<(i32, i32, u64)> as *mut c_void
        ));
        assert_eq!(seen, [(3, 4, 0x31)]);

        // Write: the lot that remains, with the live configuration's ID.
        let rec = save_lots(l);
        let mut want = vec![2, 0, 1, 0, 0, 0];
        want.extend_from_slice(&bytes[6..6 + 18]);
        want.extend_from_slice(&[0, 0, 0, 0]);
        assert_eq!(rec.bytes, want);
        assert_eq!(rec.refs, [(0x31, 0)]);
        fc4_netlot_destroy(l);
    }
}
