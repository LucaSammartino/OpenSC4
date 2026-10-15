// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The zone developer / lot manager C ABI against a fake game: the identity,
//! `Init` / `Shutdown` (services, subscriptions, the maximum-capacity grid),
//! lot creation and deletion with their references and the master-list
//! mirror, a host that cannot construct lots, the lot lists of §8.3 (tests 11
//! and 12, the corner rule, uniqueness against the caller's list), the save
//! record byte for byte and its round trip, shadow mode,
//! `DoParcellization` of an agricultural rect, a residential rect with
//! streets and a dezone, and the re-specs11 table entries (ABI 54): slot 21
//! `CanCreatePloppedLot` with configuration objects, `UpdateZoneType`'s
//! dezone and the street joins.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashMap};
use std::ffi::c_void;

use crate::budget::fc4_token_callback;
use crate::demand::fc4_u32_callback;
use crate::development::fc4_growth_items_sink;
use crate::hostgrid::{fc4_cell_rect, fc4_grid_api};
use crate::landvalue::fc4_grid_geometry;
use crate::stream::{fc4_istream, fc4_ref};
use crate::test_streams::{istream, ostream, In, Out};
use crate::zonedev::*;
use crate::zones::fc4_zone_region;

const GRID: u64 = 0x6000;
const LOT_BASE: u64 = 0x1_0000;
const NET_BASE: u64 = 0x9_0000_0000;
const NETWORK_OCCUPANT_TYPE: u32 = 0xC772_BF98;

#[derive(Clone, Copy, Default)]
struct Lot {
    rect: (i32, i32, i32, i32),
    facing: i32,
    zone: i32,
    state: u32,
    inits: u32,
    shutdowns: u32,
}

/// A street Place call: (kind, ax, az, bx, bz, real mode).
type Place = (u32, i32, i32, i32, i32, bool);

/// A configuration object record: (type, orientation, [min x, min z, max x, max z], values).
type CfgObject = (u8, u8, [f32; 4], Vec<u32>);
/// A network_tool_join call: (kind, ax, az, bx, bz).
type Join = (u32, i32, i32, i32, i32);

#[derive(Default)]
struct Game {
    cx: i32,
    cz: i32,
    no_services: Cell<bool>,
    services: Cell<i32>,
    subs: RefCell<Vec<u32>>,
    unsubs: RefCell<Vec<u32>>,
    grid_args: RefCell<Vec<(u32, u32, i32)>>,
    grid_shutdowns: Cell<u32>,
    grid_releases: Cell<u32>,
    tools: Cell<i32>,
    zones: RefCell<HashMap<(i32, i32), i32>>,
    /// Network flags per cell (bit n = network type n).
    nets: RefCell<HashMap<(i32, i32), u32>>,
    water: RefCell<BTreeSet<(i32, i32)>>,
    lots: RefCell<HashMap<u64, Lot>>,
    next_lot: Cell<u64>,
    refuse_lots: Cell<bool>,
    refs: RefCell<HashMap<u64, i32>>,
    /// The master-list mirror (master_insert / master_remove / master_reset).
    mirror: RefCell<Vec<u64>>,
    resets: Cell<u32>,
    places: RefCell<Vec<Place>>,
    resets_tool: RefCell<Vec<u32>>,
    demolished: RefCell<Vec<Vec<(i32, i32)>>>,
    defers: RefCell<Vec<bool>>,
    textures: RefCell<Vec<u64>>,
    levellings: Cell<u32>,
    removed_occupants: RefCell<Vec<u64>>,
    // ABI 54 (re-specs11)
    slope: Cell<f32>,
    /// Lot configuration 2's object records (configuration 1 has none).
    objects: RefCell<Vec<CfgObject>>,
    no_land: RefCell<BTreeSet<(i32, i32)>>,
    /// zone_manager_dezone calls: the cells of each region.
    dezones: RefCell<Vec<Vec<(i32, i32)>>>,
    /// network_tool_join calls: (kind, ax, az, bx, bz).
    joins: RefCell<Vec<Join>>,
    zone_updates: Cell<u32>,
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

impl Game {
    fn new(cx: i32, cz: i32) -> Box<Game> {
        Box::new(Game {
            cx,
            cz,
            next_lot: Cell::new(LOT_BASE),
            ..Default::default()
        })
    }
    fn user(&self) -> *mut c_void {
        self as *const Game as *mut c_void
    }
    fn refs_of(&self, t: u64) -> i32 {
        self.refs.borrow().get(&t).copied().unwrap_or(0)
    }
    fn zone(&self, x1: i32, z1: i32, x2: i32, z2: i32, t: i32) {
        for x in x1..=x2 {
            for z in z1..=z2 {
                self.zones.borrow_mut().insert((x, z), t);
            }
        }
    }
    fn road(&self, x1: i32, z1: i32, x2: i32, z2: i32, bit: u32) {
        for x in x1..=x2 {
            for z in z1..=z2 {
                *self.nets.borrow_mut().entry((x, z)).or_insert(0) |= bit;
            }
        }
    }
    fn net_cell(t: u64) -> Option<(i32, i32)> {
        (t & NET_BASE == NET_BASE).then_some((((t >> 12) & 0xFFF) as i32, (t & 0xFFF) as i32))
    }
}

// ---- grids ------------------------------------------------------------------------------------------

unsafe extern "C" fn g_create(u: *mut c_void, kind: u32, instance: u32, tract: i32) -> u64 {
    game(u).grid_args.borrow_mut().push((kind, instance, tract));
    GRID
}
unsafe extern "C" fn g_class(_: *mut c_void, _t: u64) -> u32 {
    0x49B9_E604
}
unsafe extern "C" fn g_geometry(u: *mut c_void, _t: u64, out: *mut fc4_grid_geometry) {
    let g = game(u);
    *out = fc4_grid_geometry {
        tract_size: 4,
        shift: 2,
        count_x: g.cx / 4,
        count_z: g.cz / 4,
        width_x: 64.0,
        width_z: 64.0,
        inv_width_x: 1.0 / 64.0,
        inv_width_z: 1.0 / 64.0,
    };
}
unsafe extern "C" fn g_get(_: *mut c_void, _t: u64, _k: u32, _x: i32, _z: i32) -> f64 {
    0.0
}
unsafe extern "C" fn g_shutdown(u: *mut c_void, _t: u64) {
    let g = game(u);
    g.grid_shutdowns.set(g.grid_shutdowns.get() + 1);
}
unsafe extern "C" fn g_release(u: *mut c_void, _t: u64) {
    let g = game(u);
    g.grid_releases.set(g.grid_releases.get() + 1);
}

fn grid_api(g: &Game) -> fc4_grid_api {
    fc4_grid_api {
        struct_size: std::mem::size_of::<fc4_grid_api>() as u32,
        user: g.user(),
        create: Some(g_create),
        class_of: Some(g_class),
        geometry: Some(g_geometry),
        get: Some(g_get),
        set: None,
        fill: None,
        shutdown: Some(g_shutdown),
        release: Some(g_release),
    }
}

// ---- the host ---------------------------------------------------------------------------------------

unsafe extern "C" fn acquire_services(u: *mut c_void) -> bool {
    let g = game(u);
    if g.no_services.get() {
        return false;
    }
    g.services.set(g.services.get() + 1);
    true
}
unsafe extern "C" fn release_services(u: *mut c_void) {
    let g = game(u);
    g.services.set(g.services.get() - 1);
}
unsafe extern "C" fn subscribe(u: *mut c_void, m: u32) {
    game(u).subs.borrow_mut().push(m);
}
unsafe extern "C" fn unsubscribe(u: *mut c_void, m: u32) {
    game(u).unsubs.borrow_mut().push(m);
}
unsafe extern "C" fn cell_counts(u: *mut c_void, x: *mut i32, z: *mut i32) {
    *x = game(u).cx;
    *z = game(u).cz;
}
unsafe extern "C" fn acquire_tools(u: *mut c_void) {
    let g = game(u);
    g.tools.set(g.tools.get() + 1);
}
unsafe extern "C" fn release_tools(u: *mut c_void) {
    let g = game(u);
    g.tools.set(g.tools.get() - 1);
}
unsafe extern "C" fn defer(u: *mut c_void, begin: bool) {
    game(u).defers.borrow_mut().push(begin);
}
unsafe extern "C" fn zone_type(u: *mut c_void, x: i32, z: i32) -> i32 {
    game(u).zones.borrow().get(&(x, z)).copied().unwrap_or(0)
}
fn cells_of(r: &fc4_zone_region) -> Vec<(i32, i32)> {
    let w = r.x1 - r.x0 + 1;
    let mask = unsafe { std::slice::from_raw_parts(r.mask, r.len) };
    let mut v = Vec::new();
    for (i, &b) in mask.iter().enumerate() {
        if b != 0 {
            v.push((r.x0 + i as i32 % w, r.z0 + i as i32 / w));
        }
    }
    v.sort_unstable();
    v
}
unsafe extern "C" fn demolish_region(u: *mut c_void, r: *const fc4_zone_region) {
    game(u).demolished.borrow_mut().push(cells_of(&*r));
}
/// The conformance stub (zones.md §15, tests 19–29): Place(a, b) succeeds and
/// puts a network of type `kind` on every cell of a..b that has none.
unsafe extern "C" fn street_place(u: *mut c_void, kind: u32, ax: i32, az: i32, bx: i32, bz: i32, real: bool) -> bool {
    let g = game(u);
    g.places.borrow_mut().push((kind, ax, az, bx, bz, real));
    if real {
        for x in ax.min(bx)..=ax.max(bx) {
            for z in az.min(bz)..=az.max(bz) {
                g.nets.borrow_mut().entry((x, z)).or_insert(1 << kind);
            }
        }
    }
    true
}
unsafe extern "C" fn reset_tool(u: *mut c_void, kind: u32) {
    game(u).resets_tool.borrow_mut().push(kind);
}
unsafe extern "C" fn connects(u: *mut c_void, occ: u64, edge: i32, t: u32) -> bool {
    let g = game(u);
    let Some((x, z)) = Game::net_cell(occ) else {
        return false;
    };
    let (dx, dz) = match edge {
        0 => (-1, 0),
        1 => (0, -1),
        2 => (1, 0),
        3 => (0, 1),
        _ => return false,
    };
    let nets = g.nets.borrow();
    let mine = nets.get(&(x, z)).copied().unwrap_or(0) & (1 << t) != 0;
    let theirs = nets.get(&(x + dx, z + dz)).copied().unwrap_or(0) & (1 << t) != 0;
    mine && theirs
}
unsafe extern "C" fn tool_exists(_: *mut c_void, _k: u32, _x: i32, _z: i32, _f: u32, _m: *mut u8) -> bool {
    false
}
unsafe extern "C" fn cell_has_water(u: *mut c_void, x: i32, z: i32) -> bool {
    game(u).water.borrow().contains(&(x, z))
}
unsafe extern "C" fn terrain_altitude(_: *mut c_void, _x: f32, _z: f32) -> f32 {
    100.0
}
unsafe extern "C" fn terrain_vertex_altitude(_: *mut c_void, _x: i32, _z: i32) -> f32 {
    100.0
}
unsafe extern "C" fn location_in_bounds(u: *mut c_void, p: *const f32) -> bool {
    let g = game(u);
    let (x, z) = (*p, *p.add(2));
    x >= 0.0 && z >= 0.0 && x < (g.cx * 16) as f32 && z < (g.cz * 16) as f32
}
unsafe extern "C" fn position_to_cell(u: *mut c_void, p: *const f32, x: *mut i32, z: *mut i32) -> bool {
    *x = (*p / 16.0).floor() as i32;
    *z = (*p.add(2) / 16.0).floor() as i32;
    location_in_bounds(u, p)
}
unsafe extern "C" fn cell_occupants(
    u: *mut c_void,
    x: i32,
    z: i32,
    filtered: bool,
    t: u32,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) {
    let g = game(u);
    if g.nets.borrow().contains_key(&(x, z)) && (!filtered || t == NETWORK_OCCUPANT_TYPE) {
        cb.unwrap()(ctx, NET_BASE | ((x as u64) << 12) | z as u64);
    }
}
unsafe extern "C" fn occupant_type(_: *mut c_void, occ: u64) -> u32 {
    if Game::net_cell(occ).is_some() {
        NETWORK_OCCUPANT_TYPE
    } else {
        0
    }
}
unsafe extern "C" fn occupant_position(_: *mut c_void, occ: u64, out: *mut f32) {
    let (x, z) = Game::net_cell(occ).unwrap_or(((occ >> 8) as i32 & 0xFF, occ as i32 & 0xFF));
    *out = x as f32 * 16.0 + 8.0;
    *out.add(1) = 0.0;
    *out.add(2) = z as f32 * 16.0 + 8.0;
}
unsafe extern "C" fn network_has_any_flag(u: *mut c_void, occ: u64, flags: u32) -> i32 {
    match Game::net_cell(occ) {
        Some(c) => i32::from(game(u).nets.borrow().get(&c).copied().unwrap_or(0) & flags != 0),
        None => -1,
    }
}
unsafe extern "C" fn remove_occupant(u: *mut c_void, occ: u64) {
    game(u).removed_occupants.borrow_mut().push(occ);
}
unsafe extern "C" fn add_ref(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_insert(0) += 1;
}
unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_insert(0) -= 1;
}
unsafe extern "C" fn new_lot(u: *mut c_void, x: i32, z: i32, w: i32, h: i32, facing: i32) -> u64 {
    let g = game(u);
    if g.refuse_lots.get() {
        return 0;
    }
    let t = g.next_lot.get();
    g.next_lot.set(t + 1);
    let zone = zone_type(u, x, z);
    g.lots.borrow_mut().insert(
        t,
        Lot {
            rect: (x, z, x + w - 1, z + h - 1),
            facing,
            zone,
            ..Lot::default()
        },
    );
    t
}
fn with_lot<T: Default>(u: *mut c_void, t: u64, f: impl FnOnce(&mut Lot) -> T) -> T {
    game(u).lots.borrow_mut().get_mut(&t).map(f).unwrap_or_default()
}
unsafe extern "C" fn lot_init(u: *mut c_void, t: u64) {
    with_lot(u, t, |l| l.inits += 1)
}
unsafe extern "C" fn lot_shutdown(u: *mut c_void, t: u64) {
    with_lot(u, t, |l| l.shutdowns += 1)
}
unsafe extern "C" fn lot_noop(_: *mut c_void, _t: u64) {}
unsafe extern "C" fn lot_cfg(_: *mut c_void, _t: u64) -> u64 {
    0
}
unsafe extern "C" fn lot_flags(_: *mut c_void, _t: u64) -> u32 {
    0
}
unsafe extern "C" fn lot_rect(u: *mut c_void, t: u64, out: *mut fc4_cell_rect) {
    let (x1, z1, x2, z2) = with_lot(u, t, |l| l.rect);
    *out = fc4_cell_rect { x1, z1, x2, z2 };
}
unsafe extern "C" fn lot_altitude(_: *mut c_void, _t: u64) -> f32 {
    50.0
}
unsafe extern "C" fn lot_location(u: *mut c_void, t: u64, x: *mut i32, z: *mut i32) {
    let (x1, z1, _, _) = with_lot(u, t, |l| l.rect);
    *x = x1;
    *z = z1;
}
unsafe extern "C" fn lot_facing(u: *mut c_void, t: u64) -> i32 {
    with_lot(u, t, |l| l.facing)
}
unsafe extern "C" fn lot_bu(_: *mut c_void, _t: u64) -> bool {
    false
}
unsafe extern "C" fn lot_zone(u: *mut c_void, t: u64) -> i32 {
    with_lot(u, t, |l| l.zone)
}
unsafe extern "C" fn lot_state(u: *mut c_void, t: u64) -> u32 {
    with_lot(u, t, |l| l.state)
}
unsafe extern "C" fn lot_set_state(u: *mut c_void, t: u64, s: u32, _a: u32, _b: u32) {
    with_lot(u, t, |l| l.state = s)
}
unsafe extern "C" fn lot_locked(_: *mut c_void, _t: u64, _v: bool) {}
unsafe extern "C" fn cfg_bool(_: *mut c_void, _c: u64) -> bool {
    false
}
unsafe extern "C" fn cfg_bool_i(_: *mut c_void, _c: u64, _v: i32) -> bool {
    false
}
unsafe extern "C" fn cfg_bool_u(_: *mut c_void, _c: u64, _v: u32) -> bool {
    false
}
unsafe extern "C" fn cfg_id(_: *mut c_void, _c: u64) -> u32 {
    0
}
unsafe extern "C" fn cfg_size(_: *mut c_void, c: u64, x: *mut u32, z: *mut u32) {
    // Configuration 2 (the plop test's) is 2 x 2.
    let n = if c == 2 { 2 } else { 0 };
    *x = n;
    *z = n;
}
unsafe extern "C" fn cfg_stage(_: *mut c_void, _c: u64) -> u8 {
    0
}
unsafe extern "C" fn cfg_ids(_: *mut c_void, _c: u64, _cb: fc4_u32_callback, _ctx: *mut c_void) -> bool {
    false
}
unsafe extern "C" fn configurations(_: *mut c_void, _cb: fc4_token_callback, _ctx: *mut c_void) {}
unsafe extern "C" fn levelling(u: *mut c_void, _r: *const fc4_cell_rect) {
    let g = game(u);
    g.levellings.set(g.levellings.get() + 1);
}
unsafe extern "C" fn base_texture(_: *mut c_void, _t: u64) -> u64 {
    0
}
unsafe extern "C" fn texture(u: *mut c_void, t: u64) {
    game(u).textures.borrow_mut().push(t);
}
unsafe extern "C" fn insertion(_: *mut c_void, _n: *mut u64, _l: *mut bool) -> bool {
    false
}
unsafe extern "C" fn set_insertion(_: *mut c_void, _n: u64) {}
unsafe extern "C" fn bds_developer(_: *mut c_void, _t: u32) -> u64 {
    0
}
unsafe extern "C" fn by_family(_: *mut c_void, _f: u32, _cb: fc4_u32_callback, _ctx: *mut c_void) {}
unsafe extern "C" fn profile(_: *mut c_void, _t: u32, _o: *mut fc4_zonedev_profile) -> bool {
    false
}
unsafe extern "C" fn dev_i16(_: *mut c_void, _d: u64, _x: i32, _z: i32) -> i16 {
    0
}
unsafe extern "C" fn dev_demand(_: *mut c_void, _d: u64, _o: *mut f32) -> bool {
    false
}
unsafe extern "C" fn dev_list(_: *mut c_void, _d: u64, _s: fc4_growth_items_sink, _c: *mut c_void) -> bool {
    false
}
unsafe extern "C" fn dev_u32(_: *mut c_void, _d: u64) -> u32 {
    0
}
unsafe extern "C" fn master_insert(u: *mut c_void, lot: u64, before: u64) {
    let mut m = game(u).mirror.borrow_mut();
    let pos = if before == 0 {
        m.len()
    } else {
        m.iter().position(|&l| l == before).expect("`before` is in the mirror")
    };
    m.insert(pos, lot);
}
unsafe extern "C" fn master_remove(u: *mut c_void, lot: u64) {
    game(u).mirror.borrow_mut().retain(|&l| l != lot);
}
unsafe extern "C" fn master_reset(u: *mut c_void, lots: *const u64, n: usize) {
    let g = game(u);
    g.resets.set(g.resets.get() + 1);
    *g.mirror.borrow_mut() = if n == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(lots, n).to_vec()
    };
}

unsafe extern "C" fn area_slope(u: *mut c_void, _r: *const fc4_cell_rect, _w: bool) -> f32 {
    game(u).slope.get()
}
unsafe extern "C" fn cfg_max_slope(_: *mut c_void, _c: u64) -> f32 {
    10.0
}
unsafe extern "C" fn cfg_objects(u: *mut c_void, c: u64, cb: fc4_zonedev_cfg_object_callback, ctx: *mut c_void) {
    if c != 2 {
        return;
    }
    for (kind, orientation, r, values) in game(u).objects.borrow().iter() {
        let o = fc4_zonedev_cfg_object {
            kind: *kind,
            orientation: *orientation,
            reserved: [0; 2],
            min_x: r[0],
            min_z: r[1],
            max_x: r[2],
            max_z: r[3],
            values: values.as_ptr(),
            n_values: values.len(),
        };
        cb.unwrap()(ctx, &o);
    }
}
unsafe extern "C" fn network_flags(u: *mut c_void, occ: u64) -> u32 {
    Game::net_cell(occ).map_or(0, |c| game(u).nets.borrow().get(&c).copied().unwrap_or(0))
}
unsafe extern "C" fn network_has_all_flags(u: *mut c_void, occ: u64, mask: u32) -> bool {
    network_flags(u, occ) & mask == mask
}
unsafe extern "C" fn network_edge_mask(_: *mut c_void, _occ: u64, _t: u32) -> u32 {
    0
}
unsafe extern "C" fn network_is_of_type(u: *mut c_void, occ: u64, t: u32) -> i32 {
    match Game::net_cell(occ) {
        Some(_) => i32::from(network_flags(u, occ) & (1 << t) != 0),
        None => -1,
    }
}
unsafe extern "C" fn network_is_flag_0x400000(u: *mut c_void, occ: u64) -> bool {
    network_flags(u, occ) & 0x40_0000 != 0
}
unsafe extern "C" fn occupant_bounding_cells(_: *mut c_void, occ: u64, out: *mut fc4_cell_rect) {
    if let Some((x, z)) = Game::net_cell(occ) {
        *out = fc4_cell_rect {
            x1: x,
            z1: z,
            x2: x,
            z2: z,
        };
    }
}
unsafe extern "C" fn network_tool_present(u: *mut c_void, _kind: u32) -> bool {
    game(u).tools.get() > 0
}
unsafe extern "C" fn network_tool_join(u: *mut c_void, kind: u32, ax: i32, az: i32, bx: i32, bz: i32) {
    game(u).joins.borrow_mut().push((kind, ax, az, bx, bz));
}
unsafe extern "C" fn cell_has_land(u: *mut c_void, x: i32, z: i32) -> bool {
    !game(u).no_land.borrow().contains(&(x, z))
}
unsafe extern "C" fn cell_is_in_bounds(u: *mut c_void, x: i32, z: i32) -> bool {
    let g = game(u);
    (0..g.cx).contains(&x) && (0..g.cz).contains(&z)
}
unsafe extern "C" fn is_navigable(_: *mut c_void, _x: i32, _z: i32) -> bool {
    false
}
unsafe extern "C" fn lot_update_zone_type(u: *mut c_void, t: u64) {
    let g = game(u);
    g.zone_updates.set(g.zone_updates.get() + 1);
    let (x1, z1, _, _) = with_lot(u, t, |l| l.rect);
    let zone = zone_type(u, x1, z1);
    with_lot(u, t, |l| l.zone = zone);
}
unsafe extern "C" fn force_empty(_: *mut c_void) -> bool {
    false
}
/// The zone manager's dezone: the region's cells become 0.
unsafe extern "C" fn zone_manager_dezone(u: *mut c_void, r: *const fc4_zone_region) {
    let g = game(u);
    let cells = cells_of(&*r);
    for &c in &cells {
        g.zones.borrow_mut().insert(c, 0);
    }
    g.dezones.borrow_mut().push(cells);
}

fn host(g: &Game) -> fc4_zonedev_host {
    fc4_zonedev_host {
        struct_size: std::mem::size_of::<fc4_zonedev_host>() as u32,
        user: g.user(),
        grids: grid_api(g),
        acquire_services: Some(acquire_services),
        release_services: Some(release_services),
        subscribe: Some(subscribe),
        unsubscribe: Some(unsubscribe),
        cell_counts: Some(cell_counts),
        acquire_network_tools: Some(acquire_tools),
        release_network_tools: Some(release_tools),
        post_defer_occupant_operations: Some(defer),
        zone_type: Some(zone_type),
        demolish_region: Some(demolish_region),
        street_place: Some(street_place),
        reset_network_tool: Some(reset_tool),
        connects_to_neighbor: Some(connects),
        network_tool_occupant_exists: Some(tool_exists),
        street_joining_helper: None,
        cell_has_water: Some(cell_has_water),
        terrain_altitude: Some(terrain_altitude),
        terrain_vertex_altitude: Some(terrain_vertex_altitude),
        location_in_bounds: Some(location_in_bounds),
        position_to_cell: Some(position_to_cell),
        cell_occupants: Some(cell_occupants),
        occupant_type: Some(occupant_type),
        occupant_position: Some(occupant_position),
        network_has_any_flag: Some(network_has_any_flag),
        remove_occupant: Some(remove_occupant),
        add_ref: Some(add_ref),
        release: Some(release),
        new_lot: Some(new_lot),
        lot_init: Some(lot_init),
        lot_shutdown: Some(lot_shutdown),
        lot_post_city_init: Some(lot_noop),
        lot_configuration: Some(lot_cfg),
        lot_flags: Some(lot_flags),
        lot_bounding_rect: Some(lot_rect),
        lot_altitude: Some(lot_altitude),
        lot_location: Some(lot_location),
        lot_facing: Some(lot_facing),
        lot_has_business_unit: Some(lot_bu),
        lot_zone_type: Some(lot_zone),
        lot_state: Some(lot_state),
        lot_set_state: Some(lot_set_state),
        lot_set_vertices_locked: Some(lot_locked),
        cfg_compatible_with_water: Some(cfg_bool),
        cfg_compatible_with_zone: Some(cfg_bool_i),
        cfg_id: Some(cfg_id),
        cfg_size: Some(cfg_size),
        cfg_growth_stage: Some(cfg_stage),
        cfg_compatible_with_wealth: Some(cfg_bool_u),
        cfg_compatible_with_purpose: Some(cfg_bool_u),
        cfg_primary_building_ids: Some(cfg_ids),
        lot_configurations: Some(configurations),
        attempt_area_levelling: Some(levelling),
        remove_building_zot_prop: Some(lot_noop),
        lot_base_texture_occupant: Some(base_texture),
        remove_lot_from_business_unit: Some(lot_noop),
        set_empty_lot_base_texture: Some(texture),
        desired_lot_insertion_point: Some(insertion),
        set_lot_insertion_point: Some(set_insertion),
        bds_developer: Some(bds_developer),
        building_types_by_family: Some(by_family),
        building_profile: Some(profile),
        dev_estimate_desirability: Some(dev_i16),
        dev_source_demand: Some(dev_demand),
        dev_growth_list: Some(dev_list),
        dev_desirability_value: Some(dev_i16),
        dev_desirability_shift: Some(dev_u32),
        dev_purpose: Some(dev_u32),
        dev_wealth: Some(dev_u32),
        master_insert: Some(master_insert),
        master_remove: Some(master_remove),
        master_reset: Some(master_reset),
        area_slope: Some(area_slope),
        cfg_max_slope: Some(cfg_max_slope),
        cfg_objects: Some(cfg_objects),
        network_flags: Some(network_flags),
        network_has_all_flags: Some(network_has_all_flags),
        network_edge_mask: Some(network_edge_mask),
        network_is_of_type: Some(network_is_of_type),
        network_is_flag_0x400000: Some(network_is_flag_0x400000),
        occupant_bounding_cells: Some(occupant_bounding_cells),
        network_tool_present: Some(network_tool_present),
        network_tool_join: Some(network_tool_join),
        cell_has_land: Some(cell_has_land),
        cell_is_in_bounds: Some(cell_is_in_bounds),
        is_navigable: Some(is_navigable),
        lot_update_zone_type: Some(lot_update_zone_type),
        force_empty_lot_base_textures: Some(force_empty),
        zone_manager_dezone: Some(zone_manager_dezone),
    }
}

// ---- helpers ----------------------------------------------------------------------------------------

/// The caller gives a reference back.
fn drop_ref(g: &Game, t: u64) {
    *g.refs.borrow_mut().entry(t).or_insert(0) -= 1;
}

unsafe extern "C" fn push(ctx: *mut c_void, t: u64) {
    (*(ctx as *mut Vec<u64>)).push(t);
}

struct Dev {
    p: *mut fc4_zonedev,
}

impl Drop for Dev {
    fn drop(&mut self) {
        unsafe { fc4_zonedev_destroy(self.p) };
    }
}

fn make(g: &Game) -> Dev {
    let h = host(g);
    let p = unsafe { fc4_zonedev_create(&h, 0) };
    assert!(!p.is_null());
    Dev { p }
}

fn init(g: &Game) -> Dev {
    let d = make(g);
    assert!(unsafe { fc4_zonedev_init(d.p) });
    d
}

fn create_lot(d: &Dev, x: i32, z: i32, w: i32, h: i32, facing: i32) -> u64 {
    let mut out = 0u64;
    assert!(unsafe { fc4_zonedev_create_lot(d.p, x, z, w, h, facing, &mut out) });
    assert_ne!(out, 0);
    out
}

fn rect(x1: i32, z1: i32, x2: i32, z2: i32) -> fc4_cell_rect {
    fc4_cell_rect { x1, z1, x2, z2 }
}

fn in_rect(d: &Dev, r: fc4_cell_rect, origin_only: bool) -> (u32, Vec<u64>) {
    let mut v: Vec<u64> = Vec::new();
    let n = unsafe { fc4_zonedev_lots_in_cell_rect(d.p, &r, origin_only, Some(push), &mut v as *mut _ as *mut c_void) };
    (n, v)
}

fn surrounding(d: &Dev, r: fc4_cell_rect, flags: u32, existing: &[u64]) -> (u32, Vec<u64>) {
    let mut v: Vec<u64> = Vec::new();
    let n = unsafe {
        fc4_zonedev_lots_surrounding_rect(
            d.p,
            &r,
            flags,
            existing.as_ptr(),
            existing.len(),
            Some(push),
            &mut v as *mut _ as *mut c_void,
        )
    };
    (n, v)
}

fn state(d: &Dev) -> fc4_zonedev_state {
    let mut s = fc4_zonedev_state::default();
    assert!(unsafe { fc4_zonedev_get_state(d.p, &mut s) });
    s
}

fn region(x0: i32, z0: i32, x1: i32, z1: i32) -> fc4_zone_region {
    fc4_zone_region {
        x0,
        z0,
        x1,
        z1,
        mask: std::ptr::null_mut(),
        len: 0,
    }
}

fn master(d: &Dev) -> Vec<u64> {
    let mut v: Vec<u64> = Vec::new();
    unsafe { fc4_zonedev_master_lots(d.p, Some(push), &mut v as *mut _ as *mut c_void) };
    v
}

// ---- identity and lifecycle -------------------------------------------------------------------------

#[test]
fn identity() {
    for iid in [1, 0xE818_ED2B, 0x6819_05BD, 0x4522_94AA, 0xE4FD_A3D4] {
        assert!(fc4_zonedev_query_interface(iid), "{iid:#x}");
    }
    assert!(!fc4_zonedev_query_interface(0x6765_2433));
    assert_eq!(fc4_zonedev_class_id(), 0x498F_9B01);
}

#[test]
fn init_sizes_the_map_creates_the_grid_and_subscribes() {
    let g = Game::new(16, 12);
    let d = init(&g);
    let s = state(&d);
    assert_eq!((s.initialized, s.map_x, s.map_z, s.lot_count), (1, 16, 12, 0));
    assert_eq!(*g.grid_args.borrow(), vec![(0x49B9_E604, 0x69D5_C456, 4)]);
    assert_eq!(unsafe { fc4_zonedev_max_capacity_grid(d.p) }, GRID);
    assert_eq!(s.max_capacity_grid, GRID);
    assert_eq!(*g.subs.borrow(), vec![0x99EF_1142, 0x26D3_1EC1, 0xA6B7_9602]);
    assert_eq!(g.services.get(), 1);
    // Idempotent.
    assert!(unsafe { fc4_zonedev_init(d.p) });
    assert_eq!(g.grid_args.borrow().len(), 1);
    // PostCityInit takes the network tools; Shutdown gives back everything.
    unsafe { fc4_zonedev_do_message(d.p, 0x26D3_1EC1, 0, 0, 0, 0, 0, std::ptr::null_mut()) };
    assert_eq!(g.tools.get(), 1);
    unsafe { fc4_zonedev_do_message(d.p, 0xA6B7_9602, 0, 0, 0, 0, 77, std::ptr::null_mut()) };
    assert_eq!(state(&d).view_value, 77);
    assert!(unsafe { fc4_zonedev_shutdown(d.p) });
    assert_eq!(*g.unsubs.borrow(), vec![0x99EF_1142, 0x26D3_1EC1, 0xA6B7_9602]);
    assert_eq!((g.grid_shutdowns.get(), g.grid_releases.get()), (1, 1));
    assert_eq!((g.services.get(), g.tools.get()), (0, 0));
    assert_eq!(state(&d).initialized, 0);
}

#[test]
fn init_fails_without_a_service() {
    let g = Game::new(8, 8);
    g.no_services.set(true);
    let d = make(&g);
    assert!(!unsafe { fc4_zonedev_init(d.p) });
    assert_eq!(state(&d).initialized, 0);
    assert!(g.subs.borrow().is_empty());
}

// ---- lots -------------------------------------------------------------------------------------------

#[test]
fn create_and_delete_a_lot_with_references_and_the_mirror() {
    let g = Game::new(16, 16);
    let d = init(&g);
    let lot = create_lot(&d, 2, 3, 2, 2, 1);
    // One reference for the caller, one for the manager (§8.5 step 2).
    assert_eq!(g.refs_of(lot), 2);
    let l = g.lots.borrow()[&lot];
    assert_eq!((l.inits, l.state), (1, 1));
    assert_eq!(*g.textures.borrow(), vec![lot]);
    unsafe {
        assert_eq!(fc4_zonedev_get_lot(d.p, 3, 4, false), lot);
        assert_eq!(fc4_zonedev_get_lot(d.p, 3, 4, true), 0);
        assert_eq!(fc4_zonedev_get_lot(d.p, 2, 3, true), lot);
        assert_eq!(fc4_zonedev_get_lot(d.p, 4, 3, false), 0);
        assert_eq!(fc4_zonedev_get_lot(d.p, -1, 3, false), 0);
        assert_eq!(fc4_zonedev_lot_count(d.p), 1);
        let pos = [3.0f32 * 16.0 + 1.0, 0.0, 4.0 * 16.0 + 15.0];
        assert_eq!(fc4_zonedev_get_lot_at_position(d.p, pos.as_ptr()), lot);
    }
    assert_eq!(*g.mirror.borrow(), vec![lot]);
    let second = create_lot(&d, 6, 6, 1, 1, 0);
    assert_eq!(*g.mirror.borrow(), vec![lot, second]);
    // The caller gives its reference back; DeleteLot drops the manager's.
    drop_ref(&g, lot);
    unsafe { fc4_zonedev_delete_lot(d.p, lot) };
    assert_eq!(g.refs_of(lot), 0);
    assert_eq!(g.lots.borrow()[&lot].shutdowns, 1);
    assert_eq!(unsafe { fc4_zonedev_get_lot(d.p, 3, 4, false) }, 0);
    assert_eq!(*g.mirror.borrow(), vec![second]);
    assert_eq!(master(&d), vec![second]);
    let s = state(&d);
    assert_eq!((s.lot_count, s.lots_created, s.lots_deleted), (1, 2, 1));
    // Shutdown: the mirror is emptied first, then the lot released.
    drop_ref(&g, second);
    assert!(unsafe { fc4_zonedev_shutdown(d.p) });
    assert!(g.mirror.borrow().is_empty());
    assert_eq!(g.refs_of(second), 0);
}

#[test]
fn a_host_without_a_lot_constructor_fails_create_lot_cleanly() {
    let g = Game::new(16, 16);
    let d = init(&g);
    g.refuse_lots.set(true);
    let mut out = 0xDEAD_u64;
    assert!(!unsafe { fc4_zonedev_create_lot(d.p, 2, 3, 2, 2, 1, &mut out) });
    assert_eq!(out, 0);
    assert_eq!(unsafe { fc4_zonedev_get_lot(d.p, 2, 3, false) }, 0);
    assert_eq!(unsafe { fc4_zonedev_lot_count(d.p) }, 0);
    assert!(g.mirror.borrow().is_empty() && g.textures.borrow().is_empty());
    // CreateLotIfPossible fails too (code 8 but no lot).
    g.zone(2, 3, 3, 4, 1);
    assert!(!unsafe { fc4_zonedev_create_lot_if_possible(d.p, 2, 3, 2, 2, 0, false, false, &mut out) });
    assert_eq!(out, 0);
}

#[test]
fn can_create_lot_codes_test_10() {
    let g = Game::new(64, 64);
    let d = init(&g);
    g.zone(5, 5, 6, 6, 1);
    g.road(5, 6, 5, 6, 1);
    let lot = create_lot(&d, 6, 6, 1, 1, 0);
    let ccl = |x, z, w, h, znn, water| unsafe { fc4_zonedev_can_create_lot(d.p, x, z, w, h, znn, water, 0) };
    assert_eq!(ccl(5, 5, 2, 2, false, false), 3);
    g.nets.borrow_mut().clear();
    assert_eq!(ccl(5, 5, 2, 2, false, false), 7);
    drop_ref(&g, lot);
    unsafe { fc4_zonedev_delete_lot(d.p, lot) };
    assert_eq!(ccl(5, 5, 2, 2, false, false), 8);
    g.zones.borrow_mut().remove(&(6, 5));
    assert_eq!(ccl(5, 5, 2, 2, false, false), 2);
    assert_eq!(ccl(5, 5, 2, 2, true, false), 8);
    g.water.borrow_mut().insert((5, 5));
    assert_eq!(ccl(5, 5, 2, 2, true, false), 1);
    assert_eq!(ccl(5, 5, 2, 2, true, true), 8);
    assert_eq!(ccl(63, 5, 2, 2, true, true), 0);
}

#[test]
fn lots_in_cell_rect_test_11() {
    let g = Game::new(64, 64);
    let d = init(&g);
    let a = create_lot(&d, 2, 2, 1, 1, 0);
    let b = create_lot(&d, 3, 2, 2, 2, 0);
    let (n, v) = in_rect(&d, rect(0, 0, 5, 5), false);
    assert_eq!((n, v.clone()), (2, vec![a, b]));
    // One extra reference each for the caller (2 + 1).
    assert_eq!((g.refs_of(a), g.refs_of(b)), (3, 3));
    for l in v {
        drop_ref(&g, l);
    }
    let (n, v) = in_rect(&d, rect(0, 0, 5, 5), true);
    assert_eq!((n, v.clone()), (2, vec![a, b]));
    for l in v {
        drop_ref(&g, l);
    }
    // Clamped to the city; an empty rect gives nothing.
    let (n, v) = in_rect(&d, rect(-5, -5, 2, 2), false);
    assert_eq!((n, v.clone()), (1, vec![a]));
    for l in v {
        drop_ref(&g, l);
    }
    assert_eq!(in_rect(&d, rect(10, 10, 9, 9), false).0, 0);
    // The creators' references and the manager's.
    assert_eq!((g.refs_of(a), g.refs_of(b)), (2, 2));
}

#[test]
fn lots_surrounding_rect_test_12_corners_and_existing() {
    let g = Game::new(64, 64);
    let d = init(&g);
    let a = create_lot(&d, 2, 2, 1, 1, 0);
    let b = create_lot(&d, 3, 2, 2, 2, 0);
    let r = rect(3, 2, 4, 3);
    let take = |flags, existing: &[u64]| {
        let (n, v) = surrounding(&d, r, flags, existing);
        for &l in &v {
            drop_ref(&g, l);
        }
        (n, v)
    };
    assert_eq!(take(0xF, &[]), (1, vec![a]));
    assert_eq!(take(2, &[]), (0, vec![]));
    assert_eq!(take(8, &[]), (1, vec![a]));
    // A list already holding A: nothing appended.
    assert_eq!(take(0xF, &[a]), (0, vec![]));
    // The lot of the rect itself is never appended.
    assert!(!take(0xF, &[]).1.contains(&b));
    // A corner lot (x1−1, z1−1) needs both bits (§8.3, verified walk).
    let c = create_lot(&d, 2, 1, 1, 1, 0);
    assert_eq!(take(1, &[]), (0, vec![]));
    assert_eq!(take(8, &[]), (1, vec![a]));
    assert_eq!(take(9, &[]), (2, vec![c, a]));
    assert_eq!(take(0xF, &[]), (2, vec![c, a]));
    // Slot 9 and slot 8 go through the lot's rect.
    let mut v: Vec<u64> = Vec::new();
    let n = unsafe {
        fc4_zonedev_lots_surrounding_cell(
            d.p,
            4,
            3,
            0xF,
            std::ptr::null(),
            0,
            Some(push),
            &mut v as *mut _ as *mut c_void,
        )
    };
    assert_eq!((n, v.clone()), (2, vec![c, a]));
    for l in v {
        drop_ref(&g, l);
    }
    assert_eq!((g.refs_of(a), g.refs_of(b), g.refs_of(c)), (2, 2, 2));
}

#[test]
fn geometry_and_altitude_helpers() {
    let g = Game::new(64, 64);
    let d = init(&g);
    let mut out = rect(9, 9, 9, 9);
    unsafe {
        assert!(fc4_zonedev_vertices_at_edge_of_lot(d.p, &rect(2, 3, 4, 5), 4, &mut out));
        assert_eq!(out, rect(2, 6, 5, 6));
        assert!(!fc4_zonedev_vertices_at_edge_of_lot(
            d.p,
            &rect(2, 3, 4, 5),
            0,
            &mut out
        ));
        assert_eq!(out, rect(2, 6, 5, 6));
        assert!(!fc4_zonedev_neighbor_cells_at_edge_of_lot(
            d.p,
            &rect(0, 0, 4, 5),
            8,
            &mut out
        ));
        assert_eq!(out, rect(0, 0, -1, -1));
        let mut ok = true;
        assert_eq!(fc4_zonedev_lot_savvy_terrain_altitude(d.p, 40.0, 40.0, &mut ok), 100.0);
        assert!(!ok);
        let (mut a, mut b) = (0.0f32, 0.0f32);
        fc4_zonedev_lot_vertex_altitude_adjustment_range(d.p, 3, 3, &mut a, &mut b);
        assert_eq!((a, b), (f32::MAX, f32::MAX));
    }
}

// ---- the save record (§10.2) ------------------------------------------------------------------------

#[test]
fn save_record_byte_for_byte_and_first_cell_order_on_load() {
    let g = Game::new(3, 2);
    let d = init(&g);
    let l1 = create_lot(&d, 2, 0, 1, 2, 0);
    let l2 = create_lot(&d, 0, 1, 1, 1, 0);
    drop_ref(&g, l1);
    drop_ref(&g, l2);
    assert_eq!(*g.mirror.borrow(), vec![l1, l2]);
    let mut o = Out::default();
    let os = ostream(&mut o);
    assert!(unsafe { fc4_zonedev_write(d.p, &os) });
    // u16 1, u32 countX 3, u32 countZ 2, six references x outer, z inner.
    let mut want: Vec<u8> = Vec::new();
    want.extend_from_slice(&1u16.to_le_bytes());
    want.extend_from_slice(&3u32.to_le_bytes());
    want.extend_from_slice(&2u32.to_le_bytes());
    let cells = [0, l2, 0, 0, l1, l1];
    for t in cells {
        want.extend_from_slice(&t.to_le_bytes());
        want.extend_from_slice(&0u32.to_le_bytes());
    }
    assert_eq!(o.bytes, want);
    assert_eq!(o.refs.iter().map(|r| r.0).collect::<Vec<_>>(), cells.to_vec());

    // Load into a new object: each reference read hands one over; Read keeps
    // one per lot and releases the duplicate (§10.2).
    let g2 = Game::new(3, 2);
    for (t, l) in [(l1, Lot::default()), (l2, Lot::default())] {
        g2.lots.borrow_mut().insert(t, l);
    }
    g2.lots.borrow_mut().get_mut(&l1).unwrap().rect = (2, 0, 2, 1);
    g2.lots.borrow_mut().get_mut(&l2).unwrap().rect = (0, 1, 0, 1);
    let d2 = make(&g2);
    let resolve = |r: &mut fc4_ref| {
        if r.token != 0 {
            *g2.refs.borrow_mut().entry(r.token).or_insert(0) += 1;
        }
    };
    let mut input = In::new(&o.bytes);
    input.resolve = Some(&resolve);
    let is: fc4_istream = istream(&mut input);
    assert!(unsafe { fc4_zonedev_read(d2.p, &is) });
    assert_eq!((g2.refs_of(l1), g2.refs_of(l2)), (1, 1));
    // First-cell order (x outer, z inner): l2 at (0, 1) before l1 at (2, 0).
    assert_eq!(master(&d2), vec![l2, l1]);
    assert_eq!(*g2.mirror.borrow(), vec![l2, l1]);
    let s = state(&d2);
    assert_eq!(
        (s.loaded, s.initialized, s.map_x, s.map_z, s.lot_count),
        (1, 0, 3, 2, 2)
    );
    // Init on a loaded city: lot.Init() at each origin cell, the map kept.
    assert!(unsafe { fc4_zonedev_init(d2.p) });
    assert_eq!((g2.lots.borrow()[&l1].inits, g2.lots.borrow()[&l2].inits), (1, 1));
    assert_eq!(unsafe { fc4_zonedev_get_lot(d2.p, 2, 1, false) }, l1);
    // A second save gives the same bytes.
    let mut o2 = Out::default();
    let os2 = ostream(&mut o2);
    assert!(unsafe { fc4_zonedev_write(d2.p, &os2) });
    assert_eq!(o2.bytes, o.bytes);
    // A wrong version is refused.
    let mut bad = o.bytes.clone();
    bad[0] = 2;
    let d3 = make(&g2);
    let mut input3 = In::new(&bad);
    let is3: fc4_istream = istream(&mut input3);
    assert!(!unsafe { fc4_zonedev_read(d3.p, &is3) });
}

#[test]
fn shadow_mode_reads_borrowed_and_writes_nothing() {
    let g = Game::new(3, 2);
    let d = init(&g);
    let l1 = create_lot(&d, 2, 0, 1, 2, 0);
    drop_ref(&g, l1);
    let mut o = Out::default();
    let os = ostream(&mut o);
    assert!(unsafe { fc4_zonedev_write(d.p, &os) });

    let g2 = Game::new(3, 2);
    g2.lots.borrow_mut().insert(
        l1,
        Lot {
            rect: (2, 0, 2, 1),
            ..Lot::default()
        },
    );
    let h = host(&g2);
    let s = unsafe { fc4_zonedev_create_shadow(&h, 0) };
    assert!(!s.is_null());
    let mut input = In::new(&o.bytes);
    let is: fc4_istream = istream(&mut input);
    assert!(unsafe { fc4_zonedev_read(s, &is) });
    unsafe {
        assert_eq!(fc4_zonedev_get_lot(s, 2, 1, false), l1);
        assert_eq!(fc4_zonedev_lot_count(s), 1);
        let mut v: Vec<u64> = Vec::new();
        let r = rect(0, 0, 2, 1);
        assert_eq!(
            fc4_zonedev_lots_in_cell_rect(s, &r, false, Some(push), &mut v as *mut _ as *mut c_void),
            1
        );
        // Init, a delete and a create reach nothing of the game.
        assert!(fc4_zonedev_init(s));
        fc4_zonedev_delete_lot(s, l1);
        let mut out = 0u64;
        assert!(!fc4_zonedev_create_lot(s, 0, 0, 1, 1, 0, &mut out));
        fc4_zonedev_destroy(s);
    }
    assert!(g2.refs.borrow().is_empty(), "no reference taken or released");
    assert!(g2.subs.borrow().is_empty() && g2.mirror.borrow().is_empty() && g2.grid_args.borrow().is_empty());
    assert_eq!((g2.services.get(), g2.resets.get()), (0, 0));
}

// ---- parcellization (§7) ------------------------------------------------------------------------------

fn covered(g: &Game, x: i32, z: i32) -> Option<u64> {
    g.lots.borrow().iter().find_map(|(&t, l)| {
        let (x1, z1, x2, z2) = l.rect;
        (x >= x1 && x <= x2 && z >= z1 && z <= z2 && g.refs_of(t) > 0).then_some(t)
    })
}

#[test]
fn parcellization_agriculture_residential_with_streets_and_a_dezone() {
    // Agriculture: SimpleParcellize(16) over a 4 × 4 rect along a road.
    let g = Game::new(32, 32);
    let d = init(&g);
    g.road(8, 9, 17, 9, 1);
    g.zone(10, 10, 13, 13, 7);
    let mut res = fc4_zonedev_parcel_result::default();
    unsafe { fc4_zonedev_do_parcellization(d.p, &region(10, 10, 13, 13), 7, false, &mut res) };
    assert!(res.lots_created >= 1, "{res:?}");
    assert_eq!((res.lots_deleted, res.lot_failures, res.streets_placed), (0, 0, 0));
    assert_eq!(*g.defers.borrow(), vec![true, false]);
    assert_eq!(unsafe { fc4_zonedev_lot_count(d.p) }, res.lots_created);
    assert_eq!(*g.mirror.borrow(), master(&d));
    for &l in g.mirror.borrow().iter() {
        let (x1, z1, x2, z2) = g.lots.borrow()[&l].rect;
        assert!(x1 >= 10 && z1 >= 10 && x2 <= 13 && z2 <= 13, "lot inside the zone");
        assert_eq!(g.refs_of(l), 1, "only the manager's reference stays");
        assert_eq!(g.lots.borrow()[&l].zone, 7);
    }
    // ClearHighlight reset the street and road tools, not the dirt tool.
    assert_eq!(*g.resets_tool.borrow(), vec![3, 0]);

    // Residential with streets (test 19's setting): Place calls and lots.
    g.zone(10, 20, 15, 23, 1);
    g.road(8, 19, 17, 19, 1);
    unsafe { fc4_zonedev_set_options(d.p, false, true, false) };
    assert_eq!(state(&d).options, 2);
    let before = unsafe { fc4_zonedev_lot_count(d.p) };
    let mut res = fc4_zonedev_parcel_result::default();
    unsafe { fc4_zonedev_do_parcellization(d.p, &region(10, 20, 15, 23), 1, false, &mut res) };
    assert!(res.streets_placed >= 1, "{res:?}");
    assert!(res.lots_created >= 1, "{res:?}");
    assert_eq!(unsafe { fc4_zonedev_lot_count(d.p) }, before + res.lots_created);
    assert!(
        g.places.borrow().iter().all(|p| p.0 == 3 && p.5),
        "real-mode street places"
    );
    let residential: Vec<u64> = g
        .mirror
        .borrow()
        .iter()
        .copied()
        .filter(|l| g.lots.borrow()[l].zone == 1)
        .collect();
    assert_eq!(residential.len() as u32, res.lots_created);
    for &l in &residential {
        let (x1, z1, x2, z2) = g.lots.borrow()[&l].rect;
        assert!(x1 >= 10 && z1 >= 20 && x2 <= 15 && z2 <= 23);
        for x in x1..=x2 {
            for z in z1..=z2 {
                assert!(!g.nets.borrow().contains_key(&(x, z)), "no lot on a street");
            }
        }
    }

    // A dezone (the zone manager wrote 0 first): every residential lot is
    // deleted, and the automatic streets go to DemolishRegion.
    g.zone(10, 20, 15, 23, 0);
    let mut res = fc4_zonedev_parcel_result::default();
    unsafe { fc4_zonedev_do_parcellization(d.p, &region(10, 20, 15, 23), 0, true, &mut res) };
    assert_eq!(res.lots_deleted as usize, residential.len());
    assert_eq!(res.lots_created, 0);
    assert_eq!(res.demolish_regions, 1);
    let demolished = g.demolished.borrow();
    assert_eq!(demolished.len(), 1);
    assert!(demolished[0].iter().all(|c| g.nets.borrow()[c] & 0x8 != 0));
    for &l in &residential {
        assert_eq!(g.refs_of(l), 0);
        assert!(!g.mirror.borrow().contains(&l));
    }
    assert_eq!(covered(&g, 12, 21), None);
    let s = state(&d);
    assert_eq!((s.parcellizations, s.dezones), (3, 1));
}

// ---- re-specs11 (ABI 54) ------------------------------------------------------------------------------

#[test]
fn can_create_plopped_lot_codes_and_configuration_objects() {
    let g = Game::new(32, 32);
    let d = init(&g);
    let r = rect(10, 10, 11, 11);
    let code = |c: u64| unsafe { fc4_zonedev_can_create_plopped_lot(d.p, &r, 0, c) };
    // Configuration 1: not water-compatible (cfg_bool), no objects.
    assert_eq!(code(1), 8);
    g.zone(11, 11, 11, 11, 15);
    assert_eq!(code(1), 3);
    g.zone(11, 11, 11, 11, 12);
    assert_eq!(code(1), 7);
    g.zone(11, 11, 11, 11, 0);
    g.water.borrow_mut().insert((10, 10));
    assert_eq!(code(1), 1);
    g.water.borrow_mut().clear();
    g.road(10, 11, 10, 11, 1);
    assert_eq!(code(1), 3);
    g.nets.borrow_mut().clear();
    g.slope.set(10.5);
    assert_eq!(code(1), 4, "GetAreaSlope above GetMaximumSlope");
    g.slope.set(10.0);
    assert_eq!(code(1), 8, "strict");
    let outside = rect(31, 10, 32, 11);
    assert_eq!(unsafe { fc4_zonedev_can_create_plopped_lot(d.p, &outside, 0, 1) }, 0);
    assert_eq!(
        unsafe { fc4_zonedev_can_create_plopped_lot(d.p, std::ptr::null(), 0, 1) },
        0
    );
    // The objects reach the core through the table: configuration 2 is not
    // water-compatible either, so its constraint records are not looked at;
    // a type-7 record on (10, 10) for road type 0 lets a road with matching
    // edges through (both masks 0 match an edge mask of 0).
    g.objects
        .borrow_mut()
        .push((7, 0, [0.0, 0.0, 16.0, 16.0], vec![0, 0, 0]));
    g.road(11, 11, 11, 11, 1);
    assert_eq!(
        code(2),
        8,
        "the facing-0 record covers lot cell (0, 0) -> city (11, 11)"
    );
    g.nets.borrow_mut().clear();
    g.road(10, 10, 10, 10, 1);
    assert_eq!(code(2), 3, "no record on (10, 10)");
    assert!(
        g.lots.borrow().is_empty() && g.refs.borrow().is_empty(),
        "writes nothing"
    );
}

#[test]
fn update_zone_type_dezones_uncovered_cells_through_the_table() {
    // Agriculture over a 6 x 6 zone with water on (12, 12): the water cell
    // gets no lot, so UpdateZoneType hands it (and any other uncovered cell)
    // to zone_manager_dezone, which writes 0.
    let g = Game::new(32, 32);
    let d = init(&g);
    g.zone(10, 10, 15, 15, 7);
    g.water.borrow_mut().insert((12, 12));
    let mut res = fc4_zonedev_parcel_result::default();
    unsafe { fc4_zonedev_do_parcellization(d.p, &region(10, 10, 15, 15), 7, false, &mut res) };
    let dezones = g.dezones.borrow();
    assert_eq!(dezones.len(), 1, "{res:?}");
    assert!(dezones[0].contains(&(12, 12)));
    for &(x, z) in &dezones[0] {
        assert!((10..=15).contains(&x) && (10..=15).contains(&z));
        assert_eq!(covered(&g, x, z), None, "only cells without a lot");
        assert_eq!(g.zones.borrow()[&(x, z)], 0);
    }
    for x in 10..=15 {
        for z in 10..=15 {
            let dezoned = dezones[0].contains(&(x, z));
            assert_eq!(covered(&g, x, z).is_some(), !dezoned, "({x}, {z})");
        }
    }
}

#[test]
fn join_dead_end_stubs_through_the_table() {
    // A new road piece on (20, 20) (data4 1) next to a dead-end street on
    // (19, 20) that connects only west, to (18, 20).
    let g = Game::new(32, 32);
    let d = init(&g);
    g.tools.set(1);
    g.road(18, 20, 19, 20, 0x8);
    g.road(20, 20, 20, 20, 0x1);
    let occ = NET_BASE | (20 << 12) | 20;
    unsafe {
        fc4_zonedev_do_message(
            d.p,
            0x99EF_1142,
            occ,
            0,
            NETWORK_OCCUPANT_TYPE,
            1,
            0,
            std::ptr::null_mut(),
        );
    }
    assert_eq!(*g.joins.borrow(), vec![(0, 20, 20, 19, 20)]);
    // Shadow mode swallows the join.
    let h = host(&g);
    let s = unsafe { fc4_zonedev_create_shadow(&h, 0) };
    g.joins.borrow_mut().clear();
    unsafe {
        assert!(fc4_zonedev_init(s));
        fc4_zonedev_do_message(
            s,
            0x99EF_1142,
            occ,
            0,
            NETWORK_OCCUPANT_TYPE,
            1,
            0,
            std::ptr::null_mut(),
        );
        fc4_zonedev_destroy(s);
    }
    assert!(g.joins.borrow().is_empty());
}
