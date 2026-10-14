// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The zone manager C ABI against a fake game: the identity, `Init` on a new
//! city (grid, counts, tuning, subscription), the save record byte for byte
//! and its round trip (grid and service references, the slot 9 quirk),
//! `PlaceZone`'s accept, reject and abort paths with the funds checks, the
//! outLots callback at the pricing call, the dezone handler, the alpha
//! quirk, re-entrancy and shadow mode.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::ffi::c_void;

use fc4_core::zones::shipped_tuning;

use crate::budget::fc4_token_callback;
use crate::hostgrid::{fc4_cell_rect, fc4_grid_api};
use crate::landvalue::fc4_grid_geometry;
use crate::props::fc4_props;
use crate::stream::{fc4_istream, fc4_ref};
use crate::test_streams::{istream, ostream, In, Out};
use crate::zones::*;

const GRID: u64 = 0x6000;
const CITY: u64 = 0xC1;
const OCC_MGR: u64 = 0xC2;
const TERRAIN: u64 = 0xC3;
const POLLUTION: u64 = 0xC4;
const BUDGET: u64 = 0xC5;
const LOT_A: u64 = 0x1_0001;
const LOT_B: u64 = 0x1_0002;
const CFG_A: u64 = 0x2_0001;
const BLDG_A: u64 = 0x3_0001;
/// Landfill occupants and network tiles: `base | x << 8 | z`.
const LANDFILL: u64 = 0x4_0000;
const TILE: u64 = 0x5_0000;
const ROAD: u64 = 0x7_0001;
const RAIL: u64 = 0x7_0002;

#[derive(Clone, Copy, Default)]
struct Lot {
    rect: Rect,
    zone: i32,
    state: u32,
    cfg: u64,
    compatible: bool,
    building: u64,
}

type Rect = (i32, i32, i32, i32);
/// (occupant flag 1 set, street, demolition cost).
type Tile = (bool, bool, i64);
type Cells = Vec<(i32, i32)>;
/// (commit, lots, checkSlope, flags).
type Demolish = (bool, Vec<u64>, bool, u32);
/// (rect, cells, type, checkSlope).
type Post = (fc4_cell_rect, Cells, i32, bool);

#[derive(Default)]
struct Game {
    cx: i32,
    cz: i32,
    grid: RefCell<Vec<i8>>,
    grids_made: Cell<u32>,
    grid_shutdowns: Cell<u32>,
    grid_releases: Cell<u32>,
    refs: RefCell<HashMap<u64, i32>>,
    services: Cell<i32>,
    subs: RefCell<Vec<u32>>,
    unsubs: RefCell<Vec<u32>>,
    adopted: RefCell<Vec<(u32, u64)>>,
    no_tuning: Cell<bool>,
    funds: Cell<i64>,
    withdrawn: RefCell<Vec<i64>>,
    refuse_withdraw: Cell<bool>,
    price: Cell<i64>,
    landfill_cost: Cell<i64>,
    /// (cell) → (occupant flag 1 set, street, demolition cost).
    tiles: RefCell<HashMap<(i32, i32), Tile>>,
    water: RefCell<BTreeSet<(i32, i32)>>,
    slope: Cell<f32>,
    lots: RefCell<HashMap<u64, Lot>>,
    lot_at: RefCell<HashMap<(i32, i32), u64>>,
    /// cell → removable.
    landfills: RefCell<HashMap<(i32, i32), bool>>,
    removable: RefCell<HashMap<u64, bool>>,
    /// occupant → (network flags or not a network occupant, bounding rect).
    occupants: RefCell<HashMap<u64, (Option<u32>, Rect)>>,
    textures: RefCell<Vec<u64>>,
    /// Ordered events: "price", "lot:<token>", "commit", "parcel", "post".
    events: RefCell<Vec<String>>,
    demolished: RefCell<Vec<Demolish>>,
    parcels: RefCell<Vec<(Cells, i32, bool)>>,
    posts: RefCell<Vec<Post>>,
    /// Re-entrancy: the handle a parcellization delivers a dezone to, once.
    reenter: Cell<usize>,
    /// The IIDs asked by `get_ref`, in order.
    iids: RefCell<Vec<u32>>,
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

impl Game {
    fn new(cx: i32, cz: i32) -> Box<Game> {
        let g = Box::new(Game {
            cx,
            cz,
            funds: Cell::new(1_000_000),
            ..Default::default()
        });
        for (t, rc) in [(CITY, 1), (OCC_MGR, 1), (TERRAIN, 1), (POLLUTION, 1), (BUDGET, 1)] {
            g.refs.borrow_mut().insert(t, rc);
        }
        g
    }
    fn user(&self) -> *mut c_void {
        self as *const Game as *mut c_void
    }
    fn refs_of(&self, t: u64) -> i32 {
        self.refs.borrow().get(&t).copied().unwrap_or(0)
    }
    fn cell(&self, x: i32, z: i32) -> i8 {
        self.grid.borrow()[(x * self.cz + z) as usize]
    }
    fn add_lot(&self, token: u64, lot: Lot) {
        let (x1, z1, x2, z2) = lot.rect;
        for x in x1..=x2 {
            for z in z1..=z2 {
                self.lot_at.borrow_mut().insert((x, z), token);
            }
        }
        self.lots.borrow_mut().insert(token, lot);
        self.refs.borrow_mut().insert(token, 1);
    }
    fn event(&self, e: impl Into<String>) {
        self.events.borrow_mut().push(e.into());
    }
}

// ---- grids ------------------------------------------------------------------------------------------

unsafe extern "C" fn g_create(u: *mut c_void, kind: u32, instance: u32, tract: i32) -> u64 {
    let g = game(u);
    assert_eq!(
        (kind, instance, tract),
        (0x49B9_E603, 0, 1),
        "int8, instance 0, tract size 1"
    );
    g.grids_made.set(g.grids_made.get() + 1);
    *g.grid.borrow_mut() = vec![7; (g.cx * g.cz) as usize];
    g.refs.borrow_mut().insert(GRID, 1);
    GRID
}
unsafe extern "C" fn g_class(_: *mut c_void, t: u64) -> u32 {
    if t == GRID {
        0x49B9_E603
    } else {
        0x49B9_E602
    }
}
unsafe extern "C" fn g_geometry(u: *mut c_void, _t: u64, out: *mut fc4_grid_geometry) {
    let g = game(u);
    *out = fc4_grid_geometry {
        tract_size: 1,
        shift: 0,
        count_x: g.cx,
        count_z: g.cz,
        width_x: 16.0,
        width_z: 16.0,
        inv_width_x: 1.0 / 16.0,
        inv_width_z: 1.0 / 16.0,
    };
}
unsafe extern "C" fn g_get(u: *mut c_void, _t: u64, _k: u32, x: i32, z: i32) -> f64 {
    f64::from(game(u).cell(x, z))
}
unsafe extern "C" fn g_set(u: *mut c_void, _t: u64, _k: u32, x: i32, z: i32, v: f64) {
    let g = game(u);
    g.grid.borrow_mut()[(x * g.cz + z) as usize] = v as i8;
}
unsafe extern "C" fn g_fill(u: *mut c_void, _t: u64, _k: u32, v: f64) {
    game(u).grid.borrow_mut().iter_mut().for_each(|c| *c = v as i8);
}
unsafe extern "C" fn g_shutdown(u: *mut c_void, _t: u64) {
    let g = game(u);
    g.grid_shutdowns.set(g.grid_shutdowns.get() + 1);
}
unsafe extern "C" fn g_release(u: *mut c_void, t: u64) {
    let g = game(u);
    g.grid_releases.set(g.grid_releases.get() + 1);
    *g.refs.borrow_mut().entry(t).or_insert(0) -= 1;
}

fn grid_api(g: &Game) -> fc4_grid_api {
    fc4_grid_api {
        struct_size: std::mem::size_of::<fc4_grid_api>() as u32,
        user: g.user(),
        create: Some(g_create),
        class_of: Some(g_class),
        geometry: Some(g_geometry),
        get: Some(g_get),
        set: Some(g_set),
        fill: Some(g_fill),
        shutdown: Some(g_shutdown),
        release: Some(g_release),
    }
}

// ---- the host ---------------------------------------------------------------------------------------

unsafe extern "C" fn acquire_services(u: *mut c_void) {
    let g = game(u);
    g.services.set(g.services.get() + 1);
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
unsafe extern "C" fn tuning(u: *mut c_void) -> *mut fc4_props {
    if game(u).no_tuning.get() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(shipped_tuning())) as *mut fc4_props
}
unsafe extern "C" fn cell_counts(u: *mut c_void, x: *mut i32, z: *mut i32) {
    *x = game(u).cx;
    *z = game(u).cz;
}
unsafe extern "C" fn service_ref(_u: *mut c_void, which: u32) -> u64 {
    [CITY, OCC_MGR, TERRAIN, POLLUTION, BUDGET][which as usize]
}
unsafe extern "C" fn adopt_service(u: *mut c_void, which: u32, token: u64) {
    game(u).adopted.borrow_mut().push((which, token));
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
unsafe extern "C" fn post_zoning_change(u: *mut c_void, r: *const fc4_zone_region, t: i32, slope: bool) {
    let r = &*r;
    let g = game(u);
    g.event("post");
    let rect = fc4_cell_rect {
        x1: r.x0,
        z1: r.z0,
        x2: r.x1,
        z2: r.z1,
    };
    g.posts.borrow_mut().push((rect, cells_of(r), t, slope));
}
unsafe extern "C" fn get_lot(u: *mut c_void, x: i32, z: i32) -> u64 {
    game(u).lot_at.borrow().get(&(x, z)).copied().unwrap_or(0)
}
unsafe extern "C" fn lots_in_cell_rect(
    u: *mut c_void,
    _r: *const fc4_cell_rect,
    _origin_only: bool,
    cb: fc4_token_callback,
    ctx: *mut c_void,
) {
    let g = game(u);
    let lots: BTreeSet<u64> = g.lots.borrow().keys().copied().collect();
    for l in lots {
        *g.refs.borrow_mut().entry(l).or_insert(0) += 1;
        cb.unwrap()(ctx, l);
    }
}
unsafe extern "C" fn do_parcellization(u: *mut c_void, r: *const fc4_zone_region, t: i32, flag: bool) {
    let g = game(u);
    g.event("parcel");
    g.parcels.borrow_mut().push((cells_of(&*r), t, flag));
    let h = g.reenter.replace(0);
    if h != 0 {
        // The zone developer lays a street: InsertOccupant reaches the zone
        // manager during its own commit.
        let mut d = fc4_zones_dezone::default();
        assert!(fc4_zones_do_message(
            h as *mut fc4_zones,
            0x99EF_1142,
            RAIL,
            0xC772_BF98,
            0,
            &mut d
        ));
        assert_eq!(d.dezoned, 1);
    }
}
unsafe extern "C" fn total_funds(u: *mut c_void) -> i64 {
    game(u).funds.get()
}
unsafe extern "C" fn withdraw_funds(u: *mut c_void, amount: i64) -> bool {
    let g = game(u);
    g.withdrawn.borrow_mut().push(amount);
    if g.refuse_withdraw.get() {
        return false;
    }
    g.funds.set(g.funds.get() - amount);
    true
}
unsafe extern "C" fn demolish_lots(
    u: *mut c_void,
    commit: bool,
    lots: *const u64,
    n: usize,
    slope: bool,
    f: u32,
) -> i64 {
    let g = game(u);
    let v = if n == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(lots, n).to_vec()
    };
    g.event(if commit { "commit" } else { "price" });
    g.demolished.borrow_mut().push((commit, v.clone(), slope, f));
    if commit {
        // The demolition frees the lots.
        for l in v {
            g.lots.borrow_mut().remove(&l);
            g.lot_at.borrow_mut().retain(|_, t| *t != l);
            *g.refs.borrow_mut().entry(l).or_insert(0) -= 1;
        }
        0
    } else {
        g.price.get() * n as i64
    }
}
unsafe extern "C" fn landfill_zoning_cost(u: *mut c_void, _r: *const fc4_zone_region) -> i64 {
    game(u).landfill_cost.get()
}
unsafe extern "C" fn find_network_tile(u: *mut c_void, x: i32, z: i32) -> u64 {
    if game(u).tiles.borrow().contains_key(&(x, z)) {
        TILE | ((x as u64) << 8) | z as u64
    } else {
        0
    }
}
fn tile_of(g: &Game, t: u64) -> Tile {
    let (x, z) = (((t >> 8) & 0xFF) as i32, (t & 0xFF) as i32);
    g.tiles.borrow()[&(x, z)]
}
unsafe extern "C" fn tile_occupant_flag_set(u: *mut c_void, t: u64, flag: u32) -> bool {
    assert_eq!(flag, 1);
    tile_of(game(u), t).0
}
unsafe extern "C" fn tile_is_of_type(u: *mut c_void, t: u64, ty: u32) -> bool {
    assert_eq!(ty, 3);
    tile_of(game(u), t).1
}
unsafe extern "C" fn tile_demolition_cost(u: *mut c_void, t: u64) -> i64 {
    tile_of(game(u), t).2
}
unsafe extern "C" fn cell_has_water(u: *mut c_void, x: i32, z: i32) -> bool {
    game(u).water.borrow().contains(&(x, z))
}
unsafe extern "C" fn area_slope(u: *mut c_void, r: *const fc4_cell_rect, with_water: bool) -> f32 {
    let r = &*r;
    assert!(with_water);
    assert_eq!((r.x1, r.z1), (r.x2, r.z2), "one cell");
    game(u).slope.get()
}
unsafe extern "C" fn set_empty_lot_base_texture(u: *mut c_void, lot: u64) {
    game(u).textures.borrow_mut().push(lot);
}
unsafe extern "C" fn landfill_at(u: *mut c_void, x: i32, z: i32) -> u64 {
    if game(u).landfills.borrow().contains_key(&(x, z)) {
        LANDFILL | ((x as u64) << 8) | z as u64
    } else {
        0
    }
}
unsafe extern "C" fn occupant_bounding_cells(u: *mut c_void, o: u64, out: *mut fc4_cell_rect) {
    let (x1, z1, x2, z2) = game(u).occupants.borrow()[&o].1;
    *out = fc4_cell_rect { x1, z1, x2, z2 };
}
unsafe extern "C" fn occupant_can_remove(u: *mut c_void, o: u64, flags: u32) -> bool {
    assert_eq!(flags, 2);
    let g = game(u);
    if o >> 16 == LANDFILL >> 16 {
        let (x, z) = (((o >> 8) & 0xFF) as i32, (o & 0xFF) as i32);
        return g.landfills.borrow()[&(x, z)];
    }
    g.removable.borrow().get(&o).copied().unwrap_or(true)
}
unsafe extern "C" fn network_has_any_flag(u: *mut c_void, o: u64, flags: u32) -> i32 {
    assert_eq!(flags, 0x8020_0090);
    match game(u).occupants.borrow().get(&o).and_then(|v| v.0) {
        None => -1,
        Some(f) => i32::from(f & flags != 0),
    }
}
unsafe extern "C" fn add_ref(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_insert(0) += 1;
}
unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_insert(0) -= 1;
}
fn lot(u: *mut c_void, l: u64) -> Lot {
    game(u).lots.borrow().get(&l).copied().expect("a live lot")
}
unsafe extern "C" fn lot_configuration(u: *mut c_void, l: u64) -> u64 {
    lot(u, l).cfg
}
unsafe extern "C" fn lot_bounding_rect(u: *mut c_void, l: u64, out: *mut fc4_cell_rect) {
    let (x1, z1, x2, z2) = lot(u, l).rect;
    *out = fc4_cell_rect { x1, z1, x2, z2 };
}
unsafe extern "C" fn lot_zone_type(u: *mut c_void, l: u64) -> i32 {
    lot(u, l).zone
}
unsafe extern "C" fn lot_state(u: *mut c_void, l: u64) -> u32 {
    lot(u, l).state
}
unsafe extern "C" fn lot_building(u: *mut c_void, l: u64) -> u64 {
    lot(u, l).building
}
unsafe extern "C" fn cfg_compatible_with_zone(u: *mut c_void, cfg: u64, t: i32) -> bool {
    let g = game(u);
    let lots = g.lots.borrow();
    let l = lots.values().find(|l| l.cfg == cfg).expect("a configuration of a lot");
    l.compatible && t == l.zone
}

fn host(g: &Game) -> fc4_zones_host {
    fc4_zones_host {
        struct_size: std::mem::size_of::<fc4_zones_host>() as u32,
        user: g.user(),
        grids: grid_api(g),
        acquire_services: Some(acquire_services),
        release_services: Some(release_services),
        subscribe: Some(subscribe),
        unsubscribe: Some(unsubscribe),
        tuning: Some(tuning),
        cell_counts: Some(cell_counts),
        service_ref: Some(service_ref),
        adopt_service: Some(adopt_service),
        post_zoning_change: Some(post_zoning_change),
        get_lot: Some(get_lot),
        lots_in_cell_rect: Some(lots_in_cell_rect),
        do_parcellization: Some(do_parcellization),
        total_funds: Some(total_funds),
        withdraw_funds: Some(withdraw_funds),
        demolish_lots: Some(demolish_lots),
        landfill_zoning_cost: Some(landfill_zoning_cost),
        find_network_tile: Some(find_network_tile),
        tile_occupant_flag_set: Some(tile_occupant_flag_set),
        tile_is_of_type: Some(tile_is_of_type),
        tile_demolition_cost: Some(tile_demolition_cost),
        cell_has_water: Some(cell_has_water),
        area_slope: Some(area_slope),
        set_empty_lot_base_texture: Some(set_empty_lot_base_texture),
        landfill_at: Some(landfill_at),
        occupant_bounding_cells: Some(occupant_bounding_cells),
        occupant_can_remove: Some(occupant_can_remove),
        network_has_any_flag: Some(network_has_any_flag),
        add_ref: Some(add_ref),
        release: Some(release),
        lot_configuration: Some(lot_configuration),
        lot_bounding_rect: Some(lot_bounding_rect),
        lot_zone_type: Some(lot_zone_type),
        lot_state: Some(lot_state),
        lot_building: Some(lot_building),
        cfg_compatible_with_zone: Some(cfg_compatible_with_zone),
    }
}

/// A new city's zone manager, initialised.
fn new_city(g: &Game) -> *mut fc4_zones {
    let z = unsafe { fc4_zones_create(&host(g)) };
    assert!(!z.is_null());
    assert!(unsafe { fc4_zones_init(z) });
    z
}

/// A region over a rect with the listed cells set (all when `cells` is
/// None), as the plugin builds it.
struct Region {
    raw: fc4_zone_region,
    mask: Vec<u8>,
}

impl Region {
    fn new(x0: i32, z0: i32, x1: i32, z1: i32) -> Region {
        let n = ((x1 - x0 + 1) * (z1 - z0 + 1)).max(0) as usize;
        let mut r = Region {
            raw: fc4_zone_region {
                x0,
                z0,
                x1,
                z1,
                mask: std::ptr::null_mut(),
                len: n,
            },
            mask: vec![1; n],
        };
        r.raw.mask = r.mask.as_mut_ptr();
        r
    }
    fn get(&self, x: i32, z: i32) -> bool {
        let w = self.raw.x1 - self.raw.x0 + 1;
        self.mask[((z - self.raw.z0) * w + (x - self.raw.x0)) as usize] != 0
    }
}

unsafe extern "C" fn collect_lot(ctx: *mut c_void, t: u64) {
    let g = &*(ctx as *const Game);
    g.event(format!("lot:{t:x}"));
}

#[allow(clippy::too_many_arguments)]
fn place(
    z: *mut fc4_zones,
    g: &Game,
    r: &mut Region,
    t: i32,
    commit: bool,
    skip_funds: bool,
    check_slope: bool,
    skip_size: bool,
    inside: bool,
) -> fc4_zones_place_result {
    let mut out = fc4_zones_place_result::default();
    let ok = unsafe {
        fc4_zones_place_zone(
            z,
            &mut r.raw,
            t,
            commit,
            skip_funds,
            check_slope,
            skip_size,
            inside,
            Some(collect_lot),
            g as *const Game as *mut c_void,
            &mut out,
        )
    };
    assert_eq!(ok, out.ok != 0);
    out
}

fn state(z: *const fc4_zones) -> fc4_zones_state {
    let mut s = fc4_zones_state::default();
    assert!(unsafe { fc4_zones_get_state(z, &mut s) });
    s
}

// ---- identity and Init ------------------------------------------------------------------------------

#[test]
fn identity_and_query_interface_table() {
    assert_eq!(fc4_zones_class_id(), 0x298F_9B2D);
    for iid in [1u32, 0x6765_2433, 0x4522_94AA, 0xE4FD_A3D4] {
        assert!(fc4_zones_query_interface(iid), "{iid:#x}");
    }
    for iid in [0u32, 0xE818_ED2B, 0x6819_05BD, 0xC9BD_939A] {
        assert!(!fc4_zones_query_interface(iid), "{iid:#x}");
    }
    for t in 0..=17 {
        assert_eq!(fc4_zones_is_rci_zone(t), (1..=9).contains(&t), "{t}");
    }
    // An incomplete table makes nothing.
    let g = Game::new(4, 4);
    let mut h = host(&g);
    h.struct_size -= 4;
    assert!(unsafe { fc4_zones_create(&h) }.is_null());
    let mut h = host(&g);
    h.grids.struct_size = 0;
    assert!(unsafe { fc4_zones_create(&h) }.is_null());
}

#[test]
fn init_of_a_new_city_makes_the_grid_counts_and_subscription() {
    let g = Game::new(8, 6);
    let z = new_city(&g);
    let s = state(z);
    assert_eq!((s.initialized, s.loaded), (1, 0));
    assert_eq!(g.grids_made.get(), 1);
    assert!(g.grid.borrow().iter().all(|&c| c == 0), "SetTractValues(0)");
    assert_eq!((s.grid, s.grid_count_x, s.grid_count_z), (GRID, 8, 6));
    assert_eq!(unsafe { fc4_zones_grid(z) }, GRID);
    assert_eq!(s.zone_count[0], 48);
    assert!(s.zone_count[1..].iter().all(|&c| c == 0));
    assert!(s.undeveloped.iter().all(|&c| c == 48), "N per type");
    assert_eq!((s.undeveloped_total, s.abandoned_tile_count), (48, 0));
    assert_eq!(*g.subs.borrow(), vec![0x99EF_1142]);
    assert_eq!(g.services.get(), 1);
    // The tuning (§4, shipped).
    let mut t = fc4_zones_tuning::default();
    assert!(unsafe { fc4_zones_get_tuning(z, &mut t) });
    assert_eq!((t.display_alpha, t.default_alpha), (170, 170));
    assert_eq!(t.min_size[7], 4);
    assert_eq!(t.max_size[0], 32768);
    assert_eq!(t.create_cost[14], 50);
    assert_eq!(t.max_slope[0], 256.0);
    unsafe {
        assert_eq!(fc4_zones_min_size(z, 14), 2);
        assert_eq!(fc4_zones_max_size(z, 7), 48);
        assert_eq!(fc4_zones_min_size(z, 16), 0, "out of range reads 0");
        assert_eq!(fc4_zones_drag_color(z, 1), 12_845_311);
        assert_eq!(fc4_zones_display_alpha(z), 170);
        assert_eq!(fc4_zones_default_display_alpha(z), 170);
        let mut key = [0u32; 3];
        assert!(fc4_zones_texture_for_zone(z, 3, key.as_mut_ptr()));
        assert_eq!(key, [0x7AB5_0E44, 0x1ABE_787D, 4116]);
        assert!(!fc4_zones_texture_for_zone(z, 16, key.as_mut_ptr()));
        assert_eq!(fc4_zones_zone_type(z, 100, 100), 0, "outside the grid");
        // Init is idempotent.
        assert!(fc4_zones_init(z));
        assert_eq!(g.grids_made.get(), 1);
        // Shutdown: unsubscribe, release, grid Shutdown + Release.
        assert!(fc4_zones_shutdown(z));
        assert_eq!(*g.unsubs.borrow(), vec![0x99EF_1142]);
        assert_eq!(g.services.get(), 0);
        assert_eq!((g.grid_shutdowns.get(), g.grid_releases.get()), (1, 1));
        assert_eq!(g.refs_of(GRID), 0);
        fc4_zones_destroy(z);
    }
}

// ---- persistence --------------------------------------------------------------------------------------

fn write(z: *const fc4_zones) -> Out {
    let mut o = Out::default();
    assert!(unsafe { fc4_zones_write(z, &ostream(&mut o)) });
    o
}

unsafe extern "C" fn recording_get_ref(u: *mut c_void, iid: u32, out: *mut fc4_ref) -> bool {
    let (inner, g): &(fc4_istream, &Game) = &*(u as *const (fc4_istream, &Game));
    g.iids.borrow_mut().push(iid);
    inner.get_ref.unwrap()(inner.user, iid, out)
}

/// Reads a record, recording the IIDs of its references.
fn read(z: *mut fc4_zones, g: &Game, bytes: &[u8]) -> bool {
    let mut i = In::new(bytes);
    let inner = istream(&mut i);
    let pair = (inner, g);
    let mut s = inner;
    s.user = &pair as *const (fc4_istream, &Game) as *mut c_void;
    s.get_ref = Some(recording_get_ref);
    macro_rules! fwd {
        ($($f:ident: $t:ty),*) => {$(
            unsafe extern "C" fn $f(u: *mut c_void, v: $t) -> bool {
                let (inner, _): &(fc4_istream, &Game) = &*(u as *const (fc4_istream, &Game));
                inner.$f.unwrap()(inner.user, v)
            }
            s.$f = Some($f);
        )*};
    }
    fwd!(get_u8: *mut u8, get_u16: *mut u16, get_u32: *mut u32, get_u64: *mut u64, get_f32: *mut f32,
        get_f64: *mut f64, skip: usize);
    unsafe extern "C" fn get_bytes(u: *mut c_void, p: *mut u8, n: usize) -> bool {
        let (inner, _): &(fc4_istream, &Game) = &*(u as *const (fc4_istream, &Game));
        inner.get_bytes.unwrap()(inner.user, p, n)
    }
    unsafe extern "C" fn error(u: *mut c_void) -> i32 {
        let (inner, _): &(fc4_istream, &Game) = &*(u as *const (fc4_istream, &Game));
        inner.error.unwrap()(inner.user)
    }
    s.get_bytes = Some(get_bytes);
    s.error = Some(error);
    unsafe { fc4_zones_read(z, &s) }
}

/// Parses a record (§10.1) for the byte-level checks.
struct Record {
    grid: (u64, u32),
    counts: [[u32; 16]; 3],
    scalars: [u32; 4],
    alpha: u8,
    map1: Vec<(u8, u8, u32)>,
    map2: Vec<(u8, u8, u32)>,
    services: Vec<(u64, u32)>,
}

fn parse(b: &[u8]) -> Record {
    let mut p = 0usize;
    let mut take = |n: usize| {
        let s = &b[p..p + n];
        p += n;
        s
    };
    let u32_ = |s: &[u8]| u32::from_le_bytes(s.try_into().unwrap());
    let rf = |s: &[u8]| {
        (
            u64::from_le_bytes(s[..8].try_into().unwrap()),
            u32::from_le_bytes(s[8..].try_into().unwrap()),
        )
    };
    assert_eq!(u16::from_le_bytes(take(2).try_into().unwrap()), 1, "version 1");
    let grid = rf(take(12));
    let mut counts = [[0u32; 16]; 3];
    for a in counts.iter_mut() {
        for v in a.iter_mut() {
            *v = u32_(take(4));
        }
    }
    let mut scalars = [0u32; 4];
    for v in scalars.iter_mut() {
        *v = u32_(take(4));
    }
    take(16 * 4 * 2 + 16 * 8 * 2 + 16 * 4 * 2); // sizes, costs, textures, colours
    let alpha = take(1)[0];
    let mut maps = [Vec::new(), Vec::new()];
    for m in maps.iter_mut() {
        let n = u32_(take(4));
        for _ in 0..n {
            let e = take(6);
            m.push((e[0], e[1], u32_(&e[2..])));
        }
    }
    let services = (0..5).map(|_| rf(take(12))).collect();
    assert_eq!(p, b.len(), "nothing after the service references");
    let [map1, map2] = maps;
    Record {
        grid,
        counts,
        scalars,
        alpha,
        map1,
        map2,
        services,
    }
}

#[test]
fn the_record_is_section_10_1_byte_for_byte_and_round_trips() {
    let g = Game::new(4, 4);
    let z = new_city(&g);
    unsafe {
        // Slot 9 inserts a zero entry (saved) and is bounded.
        assert_eq!(fc4_zones_development_failure_count(z, 3, 7), 0);
        assert_eq!(fc4_zones_development_failure_count(z, 16, 0), 0);
        assert_eq!(fc4_zones_development_failure_count(z, 0, 19), 0);
        assert_eq!(fc4_zones_development_failure_count(z, -1, 0), 0);
        assert_eq!(state(z).failure1_entries, 1);
    }
    let o = write(z);
    // 2 + 12 + 3·64 + 16 + 128 + 256 + 128 + 1 + (4 + 6) + 4 + 5·12
    assert_eq!(o.bytes.len(), 2 + 12 + 192 + 16 + 128 + 256 + 128 + 1 + 10 + 4 + 60);
    let r = parse(&o.bytes);
    assert_eq!(r.grid, (GRID, 0x49B9_E603));
    assert_eq!(r.counts[0][0], 16);
    assert_eq!(r.counts[1], [16; 16]);
    assert_eq!(r.counts[2], [16; 16], "unnamedPerType = N");
    assert_eq!(r.scalars, [16, 0, 0, 0]);
    assert_eq!(r.alpha, 170);
    assert_eq!(r.map1, vec![(3, 7, 0)], "{{zone, result, count}}");
    assert!(r.map2.is_empty());
    assert_eq!(
        r.services,
        vec![(CITY, 0), (OCC_MGR, 0), (TERRAIN, 0), (POLLUTION, 0), (BUDGET, 0)],
        "city, occupant manager, terrain, pollution, budget"
    );

    // Read into a fresh object: the grid is taken, the services handed over.
    let g2 = Game::new(4, 4);
    *g2.grid.borrow_mut() = g.grid.borrow().clone();
    g2.refs.borrow_mut().insert(GRID, 1); // the stream's reference
    let z2 = unsafe { fc4_zones_create(&host(&g2)) };
    assert!(read(z2, &g2, &o.bytes));
    assert_eq!(
        *g2.iids.borrow(),
        vec![
            0xC9BD_939A,
            0x66BA_4DB5,
            0xC6BB_8334,
            0xC70B_D37E,
            0x8771_81EC,
            0x6772_BD23
        ]
    );
    assert_eq!(
        *g2.adopted.borrow(),
        vec![(0, CITY), (1, OCC_MGR), (2, TERRAIN), (3, POLLUTION), (4, BUDGET)]
    );
    let s = state(z2);
    assert_eq!((s.initialized, s.loaded, s.grid), (0, 1, GRID));
    assert_eq!(g2.grids_made.get(), 0);
    // Init keeps the loaded grid and counts; the tuning is read again.
    assert!(unsafe { fc4_zones_init(z2) });
    assert_eq!(g2.grids_made.get(), 0, "the loaded grid is kept");
    assert_eq!(state(z2).zone_count[0], 16);
    assert_eq!(write(z2).bytes, o.bytes, "the round trip is exact");
    unsafe {
        fc4_zones_destroy(z2);
        fc4_zones_destroy(z);
    }
    // A wrong version reads nothing more.
    let z3 = unsafe { fc4_zones_create(&host(&g2)) };
    let mut bad = o.bytes.clone();
    bad[0] = 2;
    g2.iids.borrow_mut().clear();
    assert!(!read(z3, &g2, &bad));
    assert!(g2.iids.borrow().is_empty());
    // §10.1: the loaded flag is set before the version is checked.
    assert_eq!(state(z3).loaded, 1);
    unsafe { fc4_zones_destroy(z3) };
    // A truncated record fails and keeps the previous state (but the flag).
    let z4 = unsafe { fc4_zones_create(&host(&g2)) };
    assert!(!read(z4, &g2, &o.bytes[..100]));
    assert_eq!(state(z4).loaded, 1);
    assert_eq!(state(z4).zone_count[0], 0);
    unsafe { fc4_zones_destroy(z4) };
}

#[test]
fn the_tuning_comes_from_the_exemplar_at_every_init() {
    let g = Game::new(4, 4);
    let z = new_city(&g);
    let o = write(z);
    // A record with other tuning values: Init overwrites them.
    let mut b = o.bytes.clone();
    let min_at = 2 + 12 + 192 + 16;
    b[min_at + 4 * 7..min_at + 4 * 8].copy_from_slice(&9i32.to_le_bytes());
    let alpha_at = min_at + 128 + 256 + 128;
    b[alpha_at] = 33;
    let g2 = Game::new(4, 4);
    *g2.grid.borrow_mut() = vec![0; 16];
    g2.refs.borrow_mut().insert(GRID, 1);
    let z2 = unsafe { fc4_zones_create(&host(&g2)) };
    assert!(read(z2, &g2, &b));
    unsafe {
        assert_eq!(fc4_zones_min_size(z2, 7), 9, "as read");
        assert_eq!(fc4_zones_display_alpha(z2), 33);
        assert!(fc4_zones_init(z2));
        assert_eq!(fc4_zones_min_size(z2, 7), 4, "the exemplar's");
        assert_eq!(fc4_zones_display_alpha(z2), 170);
        assert_eq!(fc4_zones_default_display_alpha(z2), 170);
        fc4_zones_destroy(z2);
        fc4_zones_destroy(z);
    }
    // Without the exemplar: the code defaults (sizes 0 [choice], costs 0,
    // alpha 0xFF).
    let g3 = Game::new(4, 4);
    g3.no_tuning.set(true);
    let z3 = new_city(&g3);
    let mut t = fc4_zones_tuning::default();
    assert!(unsafe { fc4_zones_get_tuning(z3, &mut t) });
    assert_eq!((t.min_size, t.create_cost, t.display_alpha), ([0; 16], [0; 16], 0xFF));
    unsafe { fc4_zones_destroy(z3) };
}

// ---- PlaceZone ----------------------------------------------------------------------------------------

#[test]
fn place_zone_accepts_charges_writes_demolishes_parcels_and_posts() {
    let g = Game::new(16, 16);
    let z = new_city(&g);
    // A lot with an incompatible configuration inside the rect.
    g.add_lot(
        LOT_A,
        Lot {
            rect: (2, 2, 3, 3),
            zone: 4,
            state: 2,
            cfg: CFG_A,
            compatible: false,
            building: BLDG_A,
        },
    );
    g.price.set(7);
    let mut r = Region::new(1, 1, 4, 4);
    let res = place(z, &g, &mut r, 1, true, false, false, false, true);
    assert_eq!(res.ok, 1);
    assert_eq!(res.error, 0);
    // 16 cells × (create[1] 10 + destroy[0] 0) + price 7.
    assert_eq!(res.cost, 16 * 10 + 7);
    assert_eq!((res.committed, res.accepted, res.cleared, res.lots), (1, 16, 0, 1));
    assert_eq!(*g.withdrawn.borrow(), vec![167]);
    assert_eq!(g.funds.get(), 1_000_000 - 167);
    for x in 1..=4 {
        for zz in 1..=4 {
            assert_eq!(g.cell(x, zz), 1);
            assert_eq!(unsafe { fc4_zones_zone_type(z, x, zz) }, 1);
        }
    }
    let s = state(z);
    assert_eq!((s.zone_count[0], s.zone_count[1]), (256 - 16, 16));
    assert_eq!(s.undeveloped, [256; 16], "never updated");
    // The two DemolishLots calls; outLots at the pricing call, before the
    // commit; then parcellization, then ZoningChange.
    assert_eq!(
        *g.demolished.borrow(),
        vec![(false, vec![LOT_A], false, 1), (true, vec![LOT_A], false, 0x2000)]
    );
    assert_eq!(
        *g.events.borrow(),
        vec![
            "price".to_string(),
            format!("lot:{LOT_A:x}"),
            "commit".into(),
            "parcel".into(),
            "post".into()
        ]
    );
    {
        let posts = g.posts.borrow();
        assert_eq!(posts.len(), 1);
        assert_eq!(
            posts[0].0,
            fc4_cell_rect {
                x1: 1,
                z1: 1,
                x2: 4,
                z2: 4
            }
        );
        assert_eq!(posts[0].1.len(), 16);
        assert_eq!((posts[0].2, posts[0].3), (1, false));
        assert_eq!(g.parcels.borrow()[0].1, 1);
    }
    // Re-zoning to the same type: no cost, counts unchanged, still posted.
    g.events.borrow_mut().clear();
    let mut r = Region::new(1, 1, 2, 2);
    let res = place(z, &g, &mut r, 1, true, false, false, false, true);
    assert_eq!((res.ok, res.cost, res.accepted), (1, 0, 4));
    assert_eq!(state(z).zone_count[1], 16);
    assert_eq!(*g.events.borrow(), vec!["commit", "parcel", "post"]);
    unsafe { fc4_zones_destroy(z) };
}

#[test]
fn place_zone_rejects_cells_into_the_callers_region() {
    let g = Game::new(16, 16);
    let z = new_city(&g);
    g.water.borrow_mut().insert((5, 5));
    g.tiles.borrow_mut().insert((6, 5), (true, false, 0));
    g.slope.set(20.0);
    let mut r = Region::new(5, 5, 7, 5);
    // checkSlope: every cell's slope 20 > 15 rejects too.
    let res = place(z, &g, &mut r, 2, true, false, true, true, true);
    assert_eq!(res.ok, 1);
    assert_eq!((res.cleared, res.accepted, res.committed), (3, 0, 0));
    assert!(!r.get(5, 5) && !r.get(6, 5) && !r.get(7, 5));
    assert!(g.posts.borrow().is_empty(), "nothing accepted: no ZoningChange");
    assert_eq!(*g.withdrawn.borrow(), vec![0], "the commit still withdraws the cost");
    // Without the slope check only the water and the network cells reject.
    let mut r = Region::new(5, 5, 7, 5);
    let res = place(z, &g, &mut r, 2, true, false, false, true, true);
    assert_eq!((res.cleared, res.accepted), (2, 1));
    assert!(r.get(7, 5));
    assert_eq!(g.cell(7, 5), 2);
    // Dezoning (type 0) a network cell is accepted and charges a street's
    // demolition.
    g.tiles.borrow_mut().insert((6, 5), (true, true, 25));
    let mut r = Region::new(6, 5, 7, 5);
    let res = place(z, &g, &mut r, 0, false, false, false, true, true);
    // Cell (6, 5): 25 (the street's demolition; old type 0, no zone
    // charge); cell (7, 5): old 2 → create[0] 0 + destroy[2] 1.
    assert_eq!((res.ok, res.cost, res.cleared), (1, 25 + 1, 0));
    unsafe { fc4_zones_destroy(z) };
}

#[test]
fn place_zone_aborts_and_fails() {
    let g = Game::new(16, 16);
    let z = new_city(&g);
    // Size checks (type 7: min 4, max 48).
    let mut r = Region::new(0, 0, 2, 5);
    let res = place(z, &g, &mut r, 7, true, false, false, false, true);
    assert_eq!((res.ok, res.error, res.cost), (0, 3, 0));
    let mut r = Region::new(0, 0, 3, 3);
    assert_eq!(place(z, &g, &mut r, 7, false, false, false, false, true).error, 0);
    // Type ≥ 16: error 0, nothing called but the funds.
    let mut r = Region::new(0, 0, 1, 1);
    let res = place(z, &g, &mut r, 16, true, false, false, true, true);
    assert_eq!((res.ok, res.error), (0, 0));
    // Landfill on water: error 2, ABORT; the post-loop block still runs (the
    // landfill zoning cost), then the call fails with nothing committed.
    g.water.borrow_mut().insert((1, 0));
    g.landfill_cost.set(40);
    let mut r = Region::new(0, 0, 1, 1);
    let res = place(z, &g, &mut r, 14, true, false, false, true, true);
    assert_eq!((res.ok, res.error), (0, 2));
    // (0, 0) and (0, 1) were accepted before the abort: 2 × 50 + 40.
    assert_eq!(res.cost, 2 * 50 + 40);
    assert!(g.withdrawn.borrow().is_empty() && g.posts.borrow().is_empty());
    assert_eq!(g.cell(0, 0), 0);
    // A landfill occupant that cannot be removed blocks: error 5.
    g.water.borrow_mut().clear();
    g.landfills.borrow_mut().insert((3, 3), false);
    let mut r = Region::new(3, 3, 3, 3);
    let res = place(z, &g, &mut r, 1, true, false, false, true, true);
    assert_eq!((res.ok, res.error), (0, 5));
    // A removable one is accepted.
    g.landfills.borrow_mut().insert((3, 3), true);
    let mut r = Region::new(3, 3, 3, 3);
    assert_eq!(place(z, &g, &mut r, 1, false, false, false, true, true).ok, 1);
    // A lot crossing the rect: a landfill aborts with 5, other types reject.
    g.add_lot(
        LOT_B,
        Lot {
            rect: (8, 8, 9, 9),
            zone: 1,
            state: 1,
            cfg: 0,
            compatible: true,
            building: 0,
        },
    );
    let mut r = Region::new(8, 8, 8, 8);
    assert_eq!(place(z, &g, &mut r, 14, false, false, false, true, true).error, 5);
    let mut r = Region::new(8, 8, 8, 8);
    let res = place(z, &g, &mut r, 2, false, false, false, true, true);
    assert_eq!((res.ok, res.cleared), (1, 1));
    // Funds (signed int64): cost > funds fails with 1; skipFunds costs 0.
    g.funds.set(5);
    let mut r = Region::new(10, 10, 11, 10);
    let res = place(z, &g, &mut r, 3, true, false, false, true, true);
    assert_eq!((res.ok, res.error, res.cost), (0, 1, 100));
    g.funds.set(-1);
    let mut r = Region::new(10, 10, 11, 10);
    let res = place(z, &g, &mut r, 3, true, true, false, true, true);
    assert_eq!((res.ok, res.error, res.cost, res.accepted), (1, 0, 0, 2));
    assert_eq!(*g.withdrawn.borrow(), vec![0]);
    // WithdrawFunds refusing: error 1, nothing written.
    g.funds.set(1000);
    g.refuse_withdraw.set(true);
    let mut r = Region::new(12, 12, 12, 12);
    let res = place(z, &g, &mut r, 3, true, false, false, true, true);
    assert_eq!((res.ok, res.error, res.cost, res.committed), (0, 1, 50, 0));
    assert_eq!(g.cell(12, 12), 0);
    // A NULL or oversized region fails with error 0 and calls nothing.
    let calls = g.withdrawn.borrow().len();
    let mut out = fc4_zones_place_result::default();
    let ok = unsafe {
        fc4_zones_place_zone(
            z,
            std::ptr::null_mut(),
            1,
            true,
            true,
            false,
            true,
            true,
            None,
            std::ptr::null_mut(),
            &mut out,
        )
    };
    assert!(!ok);
    assert_eq!((out.error, out.cost), (0, 0));
    let mut huge = Region::new(0, 0, 0, 0);
    huge.raw.x1 = 5000;
    assert_eq!(place(z, &g, &mut huge, 1, true, true, false, true, true).ok, 0);
    assert_eq!(g.withdrawn.borrow().len(), calls);
    unsafe { fc4_zones_destroy(z) };
}

// ---- the dezone handler ---------------------------------------------------------------------------------

#[test]
fn a_new_network_dezones_its_bounding_cells() {
    let g = Game::new(16, 16);
    let z = new_city(&g);
    // Zone a block, with a lot of the old type inside.
    let mut r = Region::new(0, 0, 5, 5);
    assert_eq!(place(z, &g, &mut r, 4, true, false, false, true, true).ok, 1);
    g.add_lot(
        LOT_A,
        Lot {
            rect: (2, 0, 2, 0),
            zone: 4,
            state: 2,
            cfg: CFG_A,
            compatible: true,
            building: BLDG_A,
        },
    );
    g.occupants.borrow_mut().insert(ROAD, (Some(0x1), (2, 0, 2, 5)));
    g.occupants.borrow_mut().insert(RAIL, (Some(0x10), (3, 0, 3, 5)));
    g.occupants.borrow_mut().insert(BLDG_A, (None, (2, 0, 2, 0)));
    g.posts.borrow_mut().clear();
    g.withdrawn.borrow_mut().clear();
    let funds = g.funds.get();
    unsafe {
        let mut d = fc4_zones_dezone::default();
        // A road (no flag of 0x80200090): dezoned, free of charge.
        assert!(fc4_zones_do_message(z, 0x99EF_1142, ROAD, 0xC772_BF98, 0, &mut d));
        assert_eq!(d.dezoned, 1);
        assert_eq!(
            d.rect,
            fc4_cell_rect {
                x1: 2,
                z1: 0,
                x2: 2,
                z2: 5
            }
        );
        assert_eq!((d.changed, d.lots), (6, 1));
        for zz in 0..=5 {
            assert_eq!(g.cell(2, zz), 0);
        }
        assert_eq!(g.funds.get(), funds, "skipFunds");
        assert_eq!(*g.withdrawn.borrow(), vec![0]);
        assert_eq!(g.posts.borrow()[0].2, 0);
        assert!(!g.posts.borrow()[0].3, "checkSlope false");
        // The lot was incompatible with type 0: demolished at 0x2000.
        assert_eq!(g.demolished.borrow().last().unwrap().3, 0x2000);
        // Flag bit 4 keeps a network from dezoning; a non-network occupant
        // and another occupant type are ignored.
        for (occ, data3) in [(RAIL, 0xC772_BF98), (BLDG_A, 0xC772_BF98), (ROAD, 0x2781_28A0)] {
            let mut d = fc4_zones_dezone::default();
            assert!(fc4_zones_do_message(z, 0x99EF_1142, occ, data3, 0, &mut d));
            assert_eq!(d.dezoned, 0);
        }
        // Another message, or no occupant: nothing.
        assert!(fc4_zones_do_message(
            z,
            0x26D3_1EC1,
            ROAD,
            0xC772_BF98,
            0,
            std::ptr::null_mut()
        ));
        assert!(fc4_zones_do_message(
            z,
            0x99EF_1142,
            0,
            0xC772_BF98,
            0,
            std::ptr::null_mut()
        ));
        assert_eq!(state(z).dezones, 1);
        assert_eq!(g.cell(3, 0), 4);
        fc4_zones_destroy(z);
    }
}

#[test]
fn a_dezone_during_a_commit_runs_at_once_and_is_counted() {
    let g = Game::new(16, 16);
    let z = new_city(&g);
    g.occupants.borrow_mut().insert(RAIL, (Some(0), (0, 3, 7, 3)));
    g.reenter.set(z as usize);
    g.add_lot(
        LOT_A,
        Lot {
            rect: (5, 5, 5, 5),
            zone: 4,
            state: 2,
            cfg: CFG_A,
            compatible: false,
            building: 0,
        },
    );
    let mut r = Region::new(0, 0, 7, 7);
    let res = place(z, &g, &mut r, 1, true, false, false, true, true);
    assert_eq!(res.ok, 1);
    // The outer call's outputs are its own: 64 accepted, one lot.
    assert_eq!((res.accepted, res.lots), (64, 1));
    // The outer commit wrote type 1 first; the nested dezone then cleared row
    // z = 3, and its ZoningChange came before the outer one.
    for x in 0..=7 {
        assert_eq!(g.cell(x, 3), 0);
        assert_eq!(g.cell(x, 4), 1);
    }
    let posts = g.posts.borrow();
    assert_eq!(posts.len(), 2);
    assert_eq!((posts[0].2, posts[0].1.len()), (0, 8));
    assert_eq!((posts[1].2, posts[1].1.len()), (1, 64));
    let s = state(z);
    assert_eq!((s.reentrant_calls, s.dezones, s.place_calls), (1, 1, 1));
    assert_eq!((s.zone_count[0], s.zone_count[1]), (256 - 64 + 8, 64 - 8));
    // The nested PlaceZone did not report into the outer outLots callback.
    let lots: Vec<_> = g
        .events
        .borrow()
        .iter()
        .filter(|e| e.starts_with("lot:"))
        .cloned()
        .collect();
    assert_eq!(lots, vec![format!("lot:{LOT_A:x}")]);
    drop(posts);
    unsafe { fc4_zones_destroy(z) };
}

// ---- slot 18 ----------------------------------------------------------------------------------------

#[test]
fn a_changed_alpha_retextures_every_lot_in_state_1() {
    let g = Game::new(16, 16);
    let z = new_city(&g);
    for (t, state) in [(LOT_A, 1), (LOT_B, 2)] {
        g.add_lot(
            t,
            Lot {
                rect: (t as i32 & 7, 0, t as i32 & 7, 0),
                zone: 1,
                state,
                ..Default::default()
            },
        );
    }
    unsafe {
        fc4_zones_set_display_alpha(z, 170);
        assert!(g.textures.borrow().is_empty(), "unchanged: nothing");
        fc4_zones_set_display_alpha(z, 99);
        assert_eq!(*g.textures.borrow(), vec![LOT_A]);
        assert_eq!(fc4_zones_display_alpha(z), 99);
        assert_eq!(fc4_zones_default_display_alpha(z), 170);
        // Every lot's reference from GetLotsInCellRect was released.
        assert_eq!((g.refs_of(LOT_A), g.refs_of(LOT_B)), (1, 1));
        fc4_zones_destroy(z);
    }
}

// ---- shadow mode ------------------------------------------------------------------------------------

#[test]
fn shadow_mode_reads_the_game_and_writes_nothing() {
    let g = Game::new(16, 16);
    let live = new_city(&g);
    let mut r = Region::new(0, 0, 3, 3);
    assert_eq!(place(live, &g, &mut r, 5, true, false, false, true, true).ok, 1);
    let o = write(live);

    // The shadow reads the same record with a borrowed grid and no services.
    let mut h = host(&g);
    h.grids.set = None;
    h.grids.fill = None;
    h.grids.shutdown = None;
    h.grids.release = None;
    let s = unsafe { fc4_zones_create_shadow(&h) };
    let refs_before: BTreeMap<u64, i32> = g.refs.borrow().iter().map(|(&k, &v)| (k, v)).collect();
    g.adopted.borrow_mut().clear();
    assert!(read(s, &g, &o.bytes));
    assert!(g.adopted.borrow().is_empty(), "nothing adopted");
    assert!(unsafe { fc4_zones_init(s) });
    assert_eq!(g.services.get(), 1, "the live one's only");
    assert_eq!(*g.subs.borrow(), vec![0x99EF_1142], "the live one's only");
    let (a, b) = (state(live), state(s));
    assert_eq!(a.zone_count, b.zone_count);
    assert_eq!(b.grid, GRID);

    // A dry run matches the live one and swallows nothing.
    g.price.set(3);
    g.add_lot(
        LOT_A,
        Lot {
            rect: (1, 1, 1, 1),
            zone: 5,
            state: 2,
            cfg: CFG_A,
            compatible: true,
            building: 0,
        },
    );
    let mut r1 = Region::new(0, 0, 5, 5);
    let mut r2 = Region::new(0, 0, 5, 5);
    let x = place(live, &g, &mut r1, 0, false, false, false, false, true);
    let y = place(s, &g, &mut r2, 0, false, false, false, false, true);
    assert_eq!(x, y);
    assert_eq!(r1.mask, r2.mask);
    assert_eq!(unsafe { fc4_zones_last_swallowed(s) }, 0);
    // A commit in shadow mode swallows its writes.
    let mut r3 = Region::new(6, 6, 7, 7);
    let before = g.grid.borrow().clone();
    let res = place(s, &g, &mut r3, 1, true, false, false, true, true);
    assert_eq!(res.ok, 1);
    assert_eq!(*g.grid.borrow(), before, "the grid is not written");
    assert_eq!(
        unsafe { fc4_zones_last_swallowed(s) },
        4,
        "withdraw, demolish, parcellize, post"
    );
    unsafe {
        fc4_zones_shutdown(s);
        fc4_zones_destroy(s);
    }
    assert_eq!(g.services.get(), 1);
    let mut refs_after: BTreeMap<u64, i32> = g.refs.borrow().iter().map(|(&k, &v)| (k, v)).collect();
    refs_after.remove(&LOT_A);
    assert_eq!(refs_after, refs_before, "no reference taken or released");
    unsafe { fc4_zones_destroy(live) };
}
