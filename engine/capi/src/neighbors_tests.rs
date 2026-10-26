// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The neighbours C ABI (neighbors.rs) and its shadow (neighbors_shadow.rs)
//! on a fake game: the current city `R` (1×1 at region cell (3, 3), 64×64
//! cells) and the 2×2 neighbour `W` at (1, 3); one original road connection
//! loaded from the save, our objects made through the callbacks.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::{c_char, c_void};

use fc4_core::neighbors::{conn_type, deal_type, flag, msg, status};

use crate::budget::fc4_token_callback;
use crate::neighbors::*;
use crate::neighbors_shadow::*;
use crate::props::fc4_props;
use crate::stream::fc4_ref;
use crate::test_streams::{istream, ostream, In, Out};

const R: u64 = 0x5001;
const W: u64 = 0x5002;
const CITY: u64 = 0xC17;
const ORIGINAL_ROAD: u64 = 0xA001;
const W_POLE: u64 = 0xB001;

/// A city's region position and size.
type Rect = ((i32, i32), (i32, i32));

#[derive(Default)]
struct Game {
    cities: HashMap<u64, Rect>,
    budget: RefCell<HashMap<u64, f64>>,
    stats: RefCell<HashMap<(u64, u32, i32), f64>>,
    foreign: RefCell<HashMap<u64, fc4_neighbor_connection_state>>,
    released: RefCell<Vec<u64>>,
    repaired: RefCell<Vec<(u64, u64)>>,
    /// Our connection objects: token -> handle.
    conn_objects: RefCell<HashMap<u64, *mut fc4_neighbor_connection>>,
    deal_objects: RefCell<HashMap<u64, *mut fc4_neighbor_deal>>,
    next_token: Cell<u64>,
    symmetric: RefCell<Vec<(u64, u64, bool)>>,
    posts: RefCell<Vec<(u64, u32)>>,
    sends: RefCell<Vec<(u64, u32)>>,
    set_local: RefCell<Vec<(u64, usize)>>,
    /// The region drops the deal at this index from each SetLocalDeals list.
    prune: Cell<Option<usize>>,
    agents: RefCell<Vec<(u32, String, u32)>>,
    subscriptions: RefCell<Vec<u32>>,
    sim: Cell<*mut fc4_neighbors>,
    /// Foreign deals (NG4): state by token, the calls made (token, op, a, b)
    /// and the references released.
    foreign_deals: RefCell<HashMap<u64, fc4_neighbor_deal_data>>,
    deal_calls: RefCell<Vec<(u64, u32, i64, i64)>>,
    released_deals: RefCell<Vec<u64>>,
}

fn g(u: *mut c_void) -> &'static Game {
    // SAFETY: the tests pass a live Game as `user`.
    unsafe { &*(u as *const Game) }
}

unsafe extern "C" fn tuning(_: *mut c_void) -> *mut fc4_props {
    std::ptr::null_mut()
}
unsafe extern "C" fn add_agent(u: *mut c_void, level: u32, name: *const c_char, len: usize, flags: u32) {
    let s = std::str::from_utf8(std::slice::from_raw_parts(name as *const u8, len)).unwrap();
    g(u).agents.borrow_mut().push((level, s.to_string(), flags));
}
unsafe extern "C" fn subscribe(u: *mut c_void, t: u32) {
    g(u).subscriptions.borrow_mut().push(t);
}
unsafe extern "C" fn cell_counts(_: *mut c_void, x: *mut i32, z: *mut i32) {
    *x = 64;
    *z = 64;
}
unsafe extern "C" fn today(_: *mut c_void) -> u32 {
    1000
}
unsafe extern "C" fn current_city(_: *mut c_void) -> u64 {
    R
}
unsafe extern "C" fn current_city_object(_: *mut c_void) -> u64 {
    CITY
}
unsafe extern "C" fn city_at(u: *mut c_void, rx: i32, rz: i32) -> u64 {
    g(u).cities
        .iter()
        .find(|(_, ((px, pz), (sx, sz)))| rx >= *px && rx < px + sx && rz >= *pz && rz < pz + sz)
        .map_or(0, |(&t, _)| t)
}
unsafe extern "C" fn city_position(u: *mut c_void, c: u64, x: *mut i32, z: *mut i32) {
    let ((px, pz), _) = g(u).cities[&c];
    *x = px;
    *z = pz;
}
unsafe extern "C" fn city_size(u: *mut c_void, c: u64, x: *mut i32, z: *mut i32) {
    let (_, (sx, sz)) = g(u).cities[&c];
    *x = sx;
    *z = sz;
}
unsafe extern "C" fn city_established(_: *mut c_void, _: u64) -> bool {
    true
}
unsafe extern "C" fn city_stat(u: *mut c_void, c: u64, s: u32, k: i32) -> f64 {
    g(u).stats.borrow().get(&(c, s, k)).copied().unwrap_or(0.0)
}
unsafe extern "C" fn set_city_stat(u: *mut c_void, c: u64, s: u32, k: i32, v: f64) {
    g(u).stats.borrow_mut().insert((c, s, k), v);
}
unsafe extern "C" fn city_budget(u: *mut c_void, c: u64) -> f64 {
    g(u).budget.borrow().get(&c).copied().unwrap_or(0.0)
}
unsafe extern "C" fn set_city_budget(u: *mut c_void, c: u64, v: f64) {
    g(u).budget.borrow_mut().insert(c, v);
}
unsafe extern "C" fn city_find_connection(_: *mut c_void, c: u64, t: u32, x: i32, z: i32, active: *mut bool) -> u64 {
    // W holds a power pole at its east-side cell (−1, 64·… ) seen from R:
    // R's (−1, 5) is W's (128, 5).
    if c == W && t == conn_type::POWER_POLE && (x, z) == (128, 5) {
        *active = true;
        return W_POLE;
    }
    0
}
unsafe extern "C" fn change_symmetric_connection(u: *mut c_void, city: u64, conn: u64, add: bool) {
    g(u).symmetric.borrow_mut().push((city, conn, add));
}
unsafe extern "C" fn set_local_deals(u: *mut c_void, city: u64, deals: *const u64, n: usize, kept: *mut u8) {
    let game = g(u);
    if let Some(i) = game.prune.get() {
        if i < n {
            *kept.add(i) = 0;
        }
    }
    let _ = deals;
    game.set_local.borrow_mut().push((city, n));
}
unsafe extern "C" fn post_status(u: *mut c_void, deal: u64, s: u32) {
    g(u).posts.borrow_mut().push((deal, s));
}
unsafe extern "C" fn send_status(u: *mut c_void, deal: u64, s: u32) {
    let game = g(u);
    game.sends.borrow_mut().push((deal, s));
    // The message server delivers to the simulator at once (§5.3).
    let m = fc4_neighbors_message {
        msg_type: msg::DEAL_STATUS,
        data1: s,
        ..Default::default()
    };
    fc4_neighbors_do_message(game.sim.get(), &m);
}
unsafe extern "C" fn connection_object(u: *mut c_void, h: *mut fc4_neighbor_connection) -> u64 {
    let game = g(u);
    let t = game.next_token.get();
    game.next_token.set(t + 1);
    game.conn_objects.borrow_mut().insert(t, h);
    t
}
unsafe extern "C" fn deal_object(u: *mut c_void, h: *mut fc4_neighbor_deal) -> u64 {
    let game = g(u);
    let t = game.next_token.get();
    game.next_token.set(t + 1);
    game.deal_objects.borrow_mut().insert(t, h);
    t
}
unsafe extern "C" fn foreign_connection(u: *mut c_void, t: u64, out: *mut fc4_neighbor_connection_state) -> bool {
    match g(u).foreign.borrow().get(&t) {
        Some(s) => {
            *out = *s;
            true
        }
        None => false,
    }
}
unsafe extern "C" fn set_foreign_connection_flags(u: *mut c_void, t: u64, visited: i32, active: i32) {
    if let Some(s) = g(u).foreign.borrow_mut().get_mut(&t) {
        if visited >= 0 {
            s.visited = visited as u8;
        }
        if active >= 0 {
            s.active = active as u8;
        }
    }
}
unsafe extern "C" fn repair_foreign_connection(u: *mut c_void, t: u64, city: u64) {
    g(u).repaired.borrow_mut().push((t, city));
}
unsafe extern "C" fn release_connection(u: *mut c_void, t: u64) {
    let game = g(u);
    game.released.borrow_mut().push(t);
    // Our object dies with its last reference and destroys its handle.
    if let Some(h) = game.conn_objects.borrow_mut().remove(&t) {
        fc4_neighbor_connection_destroy(h);
    }
}
unsafe extern "C" fn marker_props(_: *mut c_void, _: i32, _: i32, _: fc4_token_callback, _: *mut c_void) {}
unsafe extern "C" fn foreign_deal(u: *mut c_void, t: u64, out: *mut fc4_neighbor_deal_data) -> bool {
    match g(u).foreign_deals.borrow().get(&t) {
        Some(d) => {
            *out = *d;
            true
        }
        None => false,
    }
}
unsafe extern "C" fn foreign_deal_call(u: *mut c_void, t: u64, op: u32, a: i64, b: i64) -> bool {
    let game = g(u);
    game.deal_calls.borrow_mut().push((t, op, a, b));
    let mut all = game.foreign_deals.borrow_mut();
    let Some(d) = all.get_mut(&t) else { return false };
    match op {
        10 => d.current_date = a as u32,
        12 => d.amount = a as i32,
        17 if b != 0 => d.flags |= a as u32,
        17 => d.flags &= !(a as u32),
        _ => {}
    }
    true
}
unsafe extern "C" fn release_deal(u: *mut c_void, t: u64) {
    g(u).released_deals.borrow_mut().push(t);
}

fn host(game: &Game) -> fc4_neighbors_host {
    fc4_neighbors_host {
        struct_size: std::mem::size_of::<fc4_neighbors_host>() as u32,
        user: game as *const Game as *mut c_void,
        tuning: Some(tuning),
        add_agent: Some(add_agent),
        remove_agents: None,
        subscribe: Some(subscribe),
        unsubscribe: None,
        cell_counts: Some(cell_counts),
        today: Some(today),
        utility_figures: None,
        landfill_usage: None,
        send_history: None,
        current_city: Some(current_city),
        current_city_object: Some(current_city_object),
        city_at: Some(city_at),
        city_position: Some(city_position),
        city_size: Some(city_size),
        city_established: Some(city_established),
        city_stat: Some(city_stat),
        set_city_stat: Some(set_city_stat),
        city_budget: Some(city_budget),
        set_city_budget: Some(set_city_budget),
        set_city_income: None,
        city_find_connection: Some(city_find_connection),
        city_connections: None,
        change_symmetric_connection: Some(change_symmetric_connection),
        reload_city: None,
        update_local_deals: None,
        local_deals: None,
        set_local_deals: Some(set_local_deals),
        update_import_export: None,
        post_status: Some(post_status),
        send_status: Some(send_status),
        post_connection_message: None,
        cell_center: None,
        marker_altitude: None,
        max_surface_level: None,
        add_marker_prop: None,
        marker_props: Some(marker_props),
        remove_prop: None,
        move_prop: None,
        occupant_type: None,
        occupant_networks: None,
        occupant_cell: None,
        power_pole_flag: None,
        connection_object: Some(connection_object),
        deal_object: Some(deal_object),
        foreign_connection: Some(foreign_connection),
        set_foreign_connection_flags: Some(set_foreign_connection_flags),
        repair_foreign_connection: Some(repair_foreign_connection),
        release_connection: Some(release_connection),
        foreign_deal: Some(foreign_deal),
        foreign_deal_call: Some(foreign_deal_call),
        release_deal: Some(release_deal),
    }
}

fn game() -> Box<Game> {
    let game = Box::new(Game::default());
    let mut cities = HashMap::new();
    cities.insert(R, ((3, 3), (1, 1)));
    cities.insert(W, ((1, 3), (2, 2)));
    let game = Box::new(Game { cities, ..*game });
    game.next_token.set(0x7000);
    game.foreign.borrow_mut().insert(
        ORIGINAL_ROAD,
        fc4_neighbor_connection_state {
            conn_type: conn_type::ROAD,
            x: -1,
            z: 10,
            other_x: 2,
            other_z: 3,
            visited: 0,
            active: 1,
            reserved: [0; 2],
        },
    );
    game
}

unsafe extern "C" fn push(ctx: *mut c_void, t: u64) {
    (*(ctx as *mut Vec<u64>)).push(t);
}

fn collect(f: impl FnOnce(fc4_token_callback, *mut c_void) -> usize) -> Vec<u64> {
    let mut v: Vec<u64> = Vec::new();
    let n = f(Some(push), &mut v as *mut Vec<u64> as *mut c_void);
    assert_eq!(n, v.len());
    v
}

/// The simulator record with one reference to the original road.
fn record_with_original() -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&2u16.to_le_bytes());
    b.extend_from_slice(&1u32.to_le_bytes());
    b.extend_from_slice(&ORIGINAL_ROAD.to_le_bytes());
    b.extend_from_slice(&fc4_neighbor_connection_class_id().to_le_bytes());
    b
}

#[test]
fn neighbors_create_rejects_a_short_table() {
    let game = game();
    let mut h = host(&game);
    h.struct_size -= 4;
    assert!(unsafe { fc4_neighbors_create(&h, 1, 0) }.is_null());
}

#[test]
fn neighbors_lifecycle_connections_and_save() {
    let game = game();
    let h = host(&game);
    unsafe {
        let p = fc4_neighbors_create(&h, 0x51, 7);
        assert!(!p.is_null());
        game.sim.set(p);
        // Read before Init: the original road is accepted, its reference kept.
        let bytes = record_with_original();
        let mut inp = In::new(&bytes);
        assert!(fc4_neighbors_read(p, &istream(&mut inp)));
        assert!(fc4_neighbors_init(p));
        assert_eq!(*game.repaired.borrow(), vec![(ORIGINAL_ROAD, R)]);
        assert_eq!(
            *game.agents.borrow(),
            vec![
                (6, "cSC4NeighborsSimulator".to_string(), 2),
                (4, "cSC4NeighborsSimulator".to_string(), 2),
                (10, "cSC4NeighborsSimulator".to_string(), 2)
            ]
        );
        assert_eq!(game.subscriptions.borrow().len(), 7);
        let mut st = fc4_neighbors_state::default();
        assert!(fc4_neighbors_get_state(p, &mut st));
        assert_eq!((st.connections, st.foreign_connections), (1, 1));

        // The original road answers live.
        assert_eq!(fc4_neighbors_find_connection(p, conn_type::ROAD, -1, 10), ORIGINAL_ROAD);
        assert!(fc4_neighbors_has_connection(p, conn_type::ROAD, -1, 10));
        game.foreign.borrow_mut().get_mut(&ORIGINAL_ROAD).unwrap().z = 11;
        assert_eq!(fc4_neighbors_find_connection(p, conn_type::ROAD, -1, 10), 0);
        assert_eq!(fc4_neighbors_find_connection(p, conn_type::ROAD, -1, 11), ORIGINAL_ROAD);

        // A new connection of ours: its object is made before the region sees it.
        let pole = fc4_neighbors_create_connection(p, conn_type::POWER_POLE, -1, 5, false);
        assert_eq!(pole, 0x7000);
        assert_eq!(*game.symmetric.borrow(), vec![(W, 0x7000, true)]);
        assert_eq!(
            fc4_neighbors_create_connection(p, conn_type::POWER_POLE, -1, 5, false),
            0
        );
        let all = collect(|cb, ctx| fc4_neighbors_connections(p, 0, 0, 0, cb, ctx));
        assert_eq!(all, vec![ORIGINAL_ROAD, 0x7000]);
        let to_w = collect(|cb, ctx| fc4_neighbors_connections(p, 3, 0, W, cb, ctx));
        assert_eq!(to_w, vec![ORIGINAL_ROAD, 0x7000]);
        // The other side (§2.2): W's (128, 5).
        let mut active = false;
        assert_eq!(
            fc4_neighbors_find_connection_on_other_side(p, conn_type::POWER_POLE, -1, 5, &mut active),
            W_POLE
        );
        assert!(active);
        let (mut rx, mut rz) = (0, 0);
        assert!(fc4_neighbors_neighbouring_region_cell(p, -1, 5, &mut rx, &mut rz));
        assert_eq!((rx, rz), (2, 3));
        assert_eq!(fc4_neighbors_find_city_on_other_side_of_connection(p, -1, 5), W);
        // A buy-power deal can use the pole (the other side is active).
        assert_eq!(
            fc4_neighbors_find_connection_suitable_for_deal(p, deal_type::BUY_POWER, W),
            0x7000
        );
        let adjacent = collect(|cb, ctx| fc4_neighbors_cities(p, 0, false, cb, ctx));
        assert_eq!(adjacent, vec![W]);

        // Our connection's handle: state, set, write.
        let ch = game.conn_objects.borrow()[&0x7000];
        let mut cs = fc4_neighbor_connection_state::default();
        assert!(fc4_neighbor_connection_get(ch, &mut cs));
        assert_eq!((cs.conn_type, cs.x, cs.z, cs.other_x, cs.other_z), (4, -1, 5, 2, 3));
        assert_eq!((cs.visited, cs.active), (0, 1));
        fc4_neighbor_connection_set_field(ch, 4, 0, 0);
        let mut o = Out::default();
        assert!(fc4_neighbor_connection_write(ch, &ostream(&mut o)));
        assert_eq!(&o.bytes[..2], &1u16.to_le_bytes());
        assert_eq!(o.bytes[22], 2, "flags byte: inactive");

        // Write: the original as itself, ours by its object.
        let mut o = Out::default();
        assert!(fc4_neighbors_write(p, &ostream(&mut o)));
        assert_eq!(
            o.refs,
            vec![
                (ORIGINAL_ROAD, fc4_neighbor_connection_class_id()),
                (0x7000, fc4_neighbor_connection_class_id())
            ]
        );

        // Delete: the list's reference goes (and our object with it).
        fc4_neighbors_delete_connection(p, conn_type::POWER_POLE, -1, 5);
        assert_eq!(*game.released.borrow(), vec![0x7000]);
        assert!(game.conn_objects.borrow().is_empty());
        assert!(fc4_neighbors_shutdown(p));
        assert_eq!(*game.released.borrow(), vec![0x7000, ORIGINAL_ROAD]);
        fc4_neighbors_destroy(p);
    }
}

#[test]
fn neighbors_destroy_releases_a_loaded_list() {
    let game = game();
    let h = host(&game);
    unsafe {
        let p = fc4_neighbors_create(&h, 0x51, 7);
        let bytes = record_with_original();
        let mut inp = In::new(&bytes);
        assert!(fc4_neighbors_read(p, &istream(&mut inp)));
        fc4_neighbors_destroy(p);
    }
    assert_eq!(*game.released.borrow(), vec![ORIGINAL_ROAD]);
}

#[test]
fn neighbors_deal_objects_and_reports() {
    let game = game();
    let h = host(&game);
    unsafe {
        let p = fc4_neighbors_create(&h, 0x51, 7);
        game.sim.set(p);
        assert!(fc4_neighbors_init(p));
        let m = fc4_neighbors_message {
            msg_type: msg::CONNECTIONS_READY,
            ..Default::default()
        };
        assert!(fc4_neighbors_do_message(p, &m));
        // A deal added from the outside (a foreign deal's state).
        let data = fc4_neighbor_deal_data {
            deal_type: deal_type::SELL_WATER,
            other_x: 1,
            other_z: 3,
            city_x: 3,
            city_z: 3,
            amount: 40,
            flags: flag::ACTIVE,
            cost: 77,
            start_date: 900,
            expiration_date: 2000,
            current_date: 990,
            ..Default::default()
        };
        game.set_local.borrow_mut().clear();
        let t = fc4_neighbors_add_deal_data(p, &data);
        assert_ne!(t, 0);
        // AddDeal pushes to W and R; the deal got its object first.
        assert_eq!(*game.set_local.borrow(), vec![(W, 1), (R, 1)]);
        let deals = collect(|cb, ctx| fc4_neighbors_find_deals(p, 0, 0, 0, cb, ctx));
        assert_eq!(deals, vec![t]);
        assert_eq!(
            collect(|cb, ctx| fc4_neighbors_find_deals(p, 1, deal_type::SELL_WATER, 0, cb, ctx)),
            vec![t]
        );
        assert_eq!(
            collect(|cb, ctx| fc4_neighbors_find_deals(p, 2, deal_type::SELL_WATER, W, cb, ctx)),
            vec![t]
        );
        assert!(collect(|cb, ctx| fc4_neighbors_find_deals(p, 2, deal_type::SELL_WATER, R, cb, ctx)).is_empty());
        // §10.1: a sell type without any of 0x3D counts, connection or not.
        assert_eq!(fc4_neighbors_total_income(p), 77);
        assert_eq!(fc4_neighbors_total_expense(p), 0);
        let dh = game.deal_objects.borrow()[&t];
        assert!(fc4_neighbors_holds_deal(p, dh));
        let mut d = fc4_neighbor_deal_data::default();
        assert!(fc4_neighbor_deal_get(dh, &mut d));
        assert_eq!((d.deal_type, d.amount, d.cost), (deal_type::SELL_WATER, 40, 77));
        assert_eq!(d.flags, flag::ACTIVE, "AddDeal cleared Silent; Copy keeps the rest");

        // SetAmount sends status 0 synchronously; the simulator re-runs.
        game.posts.borrow_mut().clear();
        fc4_neighbor_deal_set_amount(dh, 50);
        assert_eq!(game.sends.borrow()[0], (t, status::CHANGED));
        assert!(fc4_neighbor_deal_get(dh, &mut d));
        assert_eq!(d.amount, 50);

        // Write → read is byte-identical (§5.6).
        let mut o = Out::default();
        assert!(fc4_neighbor_deal_write(dh, &ostream(&mut o)));
        assert_eq!(&o.bytes[..2], &5u16.to_le_bytes());
        let fresh = fc4_neighbor_deal_create();
        let mut inp = In::new(&o.bytes);
        assert!(fc4_neighbor_deal_read(fresh, &istream(&mut inp)));
        let mut o2 = Out::default();
        assert!(fc4_neighbor_deal_write(fresh, &ostream(&mut o2)));
        assert_eq!(o.bytes, o2.bytes);
        fc4_neighbor_deal_destroy(fresh);

        // Clone (§5.5): city side R, so not swapped; Silent as asked.
        let c = fc4_neighbor_deal_clone(dh, true);
        let mut cd = fc4_neighbor_deal_data::default();
        assert!(fc4_neighbor_deal_get(c, &mut cd));
        assert_eq!(cd.deal_type, deal_type::SELL_WATER);
        assert_ne!(cd.flags & flag::SILENT, 0);
        fc4_neighbor_deal_destroy(c);

        // The region prunes the first deal: it leaves the simulator.
        game.prune.set(Some(0));
        fc4_neighbors_mark_deals_dirty(p);
        let day = fc4_neighbors_message {
            msg_type: msg::SIM_NEW_DAY,
            ..Default::default()
        };
        fc4_neighbors_do_message(p, &day);
        assert!(collect(|cb, ctx| fc4_neighbors_find_deals(p, 0, 0, 0, cb, ctx)).is_empty());
        assert!(!fc4_neighbors_holds_deal(p, dh));

        // The object dies: the deal forgets its token.
        let dh = game.deal_objects.borrow_mut().remove(&t).unwrap();
        fc4_neighbor_deal_destroy(dh);
        assert!(fc4_neighbors_shutdown(p));
        fc4_neighbors_destroy(p);
    }
}

/// NG4 through the C ABI: the foreign deal's token is listed as is, read and
/// driven through the callbacks, and released when the list lets it go.
/// NG11: its cost past 2^32 comes back whole from the int64 total.
#[test]
fn neighbors_foreign_deal_is_kept_and_released() {
    const F: u64 = 0xD00D;
    let game = game();
    game.foreign_deals.borrow_mut().insert(
        F,
        fc4_neighbor_deal_data {
            deal_type: deal_type::SELL_WATER,
            other_x: 1,
            other_z: 3,
            city_x: 3,
            city_z: 3,
            amount: 40,
            flags: flag::ACTIVE | flag::SILENT,
            cost: 6_000_000_000,
            start_date: 900,
            expiration_date: 2000,
            current_date: 990,
            ..Default::default()
        },
    );
    let h = host(&game);
    unsafe {
        let p = fc4_neighbors_create(&h, 0x51, 7);
        game.sim.set(p);
        assert!(fc4_neighbors_init(p));
        let m = fc4_neighbors_message {
            msg_type: msg::CONNECTIONS_READY,
            ..Default::default()
        };
        assert!(fc4_neighbors_do_message(p, &m));
        assert!(!fc4_neighbors_add_foreign_deal(p, 0));
        game.set_local.borrow_mut().clear();
        assert!(fc4_neighbors_add_foreign_deal(p, F));
        // SetFlags(Silent, false) went to the object (op 17).
        assert_eq!(game.deal_calls.borrow()[0], (F, 17, i64::from(flag::SILENT), 0));
        assert_eq!(game.foreign_deals.borrow()[&F].flags, flag::ACTIVE);
        assert_eq!(*game.set_local.borrow(), vec![(W, 1), (R, 1)]);
        // FindDeals hands out the caller's object itself; no object of ours was made.
        assert_eq!(
            collect(|cb, ctx| fc4_neighbors_find_deals(p, 0, 0, 0, cb, ctx)),
            vec![F]
        );
        assert!(game.deal_objects.borrow().is_empty());
        assert_eq!(fc4_neighbors_total_income(p), 6_000_000_000);
        let mut out = [fc4_neighbor_deal_data::default(); 2];
        assert_eq!(fc4_neighbors_deal_data(p, out.as_mut_ptr(), 2), 1);
        assert_eq!((out[0].cost, out[0].amount), (6_000_000_000, 40));
        assert!(game.released_deals.borrow().is_empty());
        // Shutdown releases the reference AddDeal was given, once.
        assert!(fc4_neighbors_shutdown(p));
        assert_eq!(*game.released_deals.borrow(), vec![F]);
        fc4_neighbors_destroy(p);
        assert_eq!(*game.released_deals.borrow(), vec![F]);
    }
}

fn shadow_city(token: u64, pos: (i32, i32), size: (i32, i32)) -> fc4_neighbors_shadow_city {
    fc4_neighbors_shadow_city {
        token,
        pos_x: pos.0,
        pos_z: pos.1,
        size_x: size.0,
        size_z: size.1,
        established: 1,
        reserved: [0; 7],
        budget: 100_000.0,
        income: 0.0,
        stats: [[0.0; 3]; 5],
    }
}

/// The shadow snapshot of `neighbors_shadow_month_on_a_snapshot`: R imports
/// 30 water from W over R's pipe (−1, 6); W's pipe answers.
struct ShadowSnapshot {
    cities: [fc4_neighbors_shadow_city; 2],
    conns: [fc4_neighbors_shadow_conn; 2],
    deals: [fc4_neighbor_deal_data; 1],
}

/// The original's state bytes of the shadow input (NG7).
#[derive(Clone, Copy)]
struct OriginalBytes {
    ready: u8,
    ui_disabled: u8,
    deals_dirty: u8,
    stats_dirty: u8,
    busy: u8,
}

const LOADED_CITY: OriginalBytes = OriginalBytes {
    ready: 1,
    ui_disabled: 0,
    deals_dirty: 0,
    stats_dirty: 0,
    busy: 0,
};

impl ShadowSnapshot {
    fn new() -> ShadowSnapshot {
        let mut r = shadow_city(R, (3, 3), (1, 1));
        let mut w = shadow_city(W, (1, 3), (2, 2));
        // Stored values: the L contribution (30 imported by R, 30 exported by
        // W) plus an O part of 5 on each.
        r.stats[1][1] = 35.0;
        w.stats[0][1] = 35.0;
        let pipe = |city, token, x, other_x| fc4_neighbors_shadow_conn {
            city,
            token,
            state: fc4_neighbor_connection_state {
                conn_type: conn_type::PIPE,
                x,
                z: 6,
                other_x,
                other_z: 3,
                visited: 0,
                active: 1,
                reserved: [0; 2],
            },
        };
        ShadowSnapshot {
            cities: [r, w],
            conns: [pipe(R, 0xA100, -1, 2), pipe(W, 0xA200, 128, 3)],
            deals: [fc4_neighbor_deal_data {
                deal_type: deal_type::BUY_WATER,
                other_x: 1,
                other_z: 3,
                city_x: 3,
                city_z: 3,
                amount: 30,
                flags: flag::ACTIVE,
                cost: 90,
                start_date: 900,
                expiration_date: 5000,
                current_date: 990,
                ..Default::default()
            }],
        }
    }

    fn input(&self, b: OriginalBytes) -> fc4_neighbors_shadow_input {
        fc4_neighbors_shadow_input {
            struct_size: std::mem::size_of::<fc4_neighbors_shadow_input>() as u32,
            today: 1000,
            count_x: 64,
            count_z: 64,
            current_city: R,
            city_object: CITY,
            landfill_usage: 0,
            landfill_capacity: 0,
            ready: b.ready,
            ui_disabled: b.ui_disabled,
            two_curves: 0,
            deals_dirty: b.deals_dirty,
            stats_dirty: b.stats_dirty,
            busy: b.busy,
            reserved: [0; 2],
            figures: fc4_neighbors_figures {
                total_funds: 50_000.0,
                water_capacity: 100.0,
                water_demand: 120.0,
                ..Default::default()
            },
            tuning: std::ptr::null(),
            cities: self.cities.as_ptr(),
            city_count: self.cities.len(),
            conns: self.conns.as_ptr(),
            conn_count: self.conns.len(),
            deals: self.deals.as_ptr(),
            deal_count: self.deals.len(),
        }
    }
}

/// NG7 (neighbors.md §15.4): the shadow takes the original's state bytes as
/// given. Before ConnectionsReady (ready 0) the month does nothing; while the
/// budget window's neighbour tab is open (uiDisabled) the state machine is
/// skipped (the deal keeps its date) but UpdateCommodityStats still pushes; a
/// busy original only gets the retry report. The dirty bytes decide a day's
/// push.
#[test]
fn neighbors_shadow_uses_the_originals_state_bytes() {
    let snap = ShadowSnapshot::new();
    let message = |msg_type| fc4_neighbors_message {
        msg_type,
        ..Default::default()
    };
    let counters = |p| {
        let mut k = fc4_neighbors_shadow_counters::default();
        assert!(unsafe { fc4_neighbors_shadow_get_counters(p, &mut k) });
        k
    };
    // (bytes, the deal's current date after the month, the month pushed, retries)
    let months = [
        (LOADED_CITY, 1000, true, 0),
        (
            OriginalBytes {
                ready: 0,
                ..LOADED_CITY
            },
            990,
            false,
            0,
        ),
        (
            OriginalBytes {
                ui_disabled: 1,
                ..LOADED_CITY
            },
            990,
            true,
            0,
        ),
        (OriginalBytes { busy: 1, ..LOADED_CITY }, 990, false, 1),
    ];
    for (i, (bytes, date, pushed, retries)) in months.into_iter().enumerate() {
        let input = snap.input(bytes);
        unsafe {
            let p = fc4_neighbors_shadow_create(&input);
            assert!(!p.is_null());
            let mut s = fc4_neighbors_state::default();
            assert!(fc4_neighbors_get_state(p, &mut s));
            assert_eq!(
                (s.ready, s.ui_disabled, s.busy),
                (bytes.ready, bytes.ui_disabled, bytes.busy),
                "month {i}: the bytes as given"
            );
            assert!(fc4_neighbors_do_message(p, &message(msg::SIM_NEW_MONTH)));
            let mut out = [fc4_neighbor_deal_data::default(); 2];
            assert_eq!(fc4_neighbors_deal_data(p, out.as_mut_ptr(), out.len()), 1);
            assert_eq!(out[0].current_date, date, "month {i}: the deal's current date");
            let k = counters(p);
            assert_eq!(k.set_local_deals > 0, pushed, "month {i}: the push");
            assert_eq!(k.retries, retries, "month {i}: the retry report");
            if !pushed {
                // A not-ready or busy month never pushes deals. Shadow setup
                // must preserve both cities' captured L + O contributions.
                for original in snap.cities {
                    let mut after = fc4_neighbors_shadow_city::default();
                    assert!(fc4_neighbors_shadow_city_get(p, original.token, &mut after));
                    assert_eq!(after.stats, original.stats, "month {i}: unchanged trade statistics");
                }
            }
            fc4_neighbors_destroy(p);
        }
    }
    // A day pushes what is dirty (§11.1): (bytes, SetLocalDeals, UpdateImportExport).
    let days = [
        (LOADED_CITY, false, false),
        (
            OriginalBytes {
                deals_dirty: 1,
                ..LOADED_CITY
            },
            true,
            false,
        ),
        (
            OriginalBytes {
                stats_dirty: 1,
                ..LOADED_CITY
            },
            false,
            true,
        ),
    ];
    for (i, (bytes, deals, stats)) in days.into_iter().enumerate() {
        let input = snap.input(bytes);
        unsafe {
            let p = fc4_neighbors_shadow_create(&input);
            assert!(!p.is_null());
            let mut s = fc4_neighbors_state::default();
            assert!(fc4_neighbors_get_state(p, &mut s));
            assert_eq!(
                (s.deals_dirty, s.stats_dirty),
                (bytes.deals_dirty, bytes.stats_dirty),
                "day {i}: the bytes as given"
            );
            assert!(fc4_neighbors_do_message(p, &message(msg::SIM_NEW_DAY)));
            let k = counters(p);
            assert_eq!(k.set_local_deals > 0, deals, "day {i}: SetLocalDeals");
            assert_eq!(k.update_import_export > 0, stats, "day {i}: UpdateImportExport");
            fc4_neighbors_destroy(p);
        }
    }
}

#[test]
fn neighbors_shadow_month_on_a_snapshot() {
    // R imports 30 water from W over R's pipe (−1, 6); W's pipe answers.
    let snap = ShadowSnapshot::new();
    let input = snap.input(LOADED_CITY);
    unsafe {
        let p = fc4_neighbors_shadow_create(&input);
        assert!(!p.is_null());
        // The snapshot's connections are the simulator's list.
        let all = collect(|cb, ctx| fc4_neighbors_connections(p, 0, 0, 0, cb, ctx));
        assert_eq!(all, vec![0xA100]);
        assert_eq!(
            fc4_neighbors_find_connection_suitable_for_deal(p, deal_type::BUY_WATER, W),
            0xA100
        );
        let m = fc4_neighbors_message {
            msg_type: msg::SIM_NEW_MONTH,
            ..Default::default()
        };
        assert!(fc4_neighbors_do_message(p, &m));
        // Budget from the figures; imports = O part (5) + the deal (30).
        let mut rc = fc4_neighbors_shadow_city::default();
        assert!(fc4_neighbors_shadow_city_get(p, R, &mut rc));
        assert_eq!(rc.budget, 50_000.0);
        assert_eq!(rc.stats[1][1], 35.0);
        assert_eq!(rc.stats[2][1], 100.0, "produced = capacity");
        assert_eq!(rc.stats[3][1], 120.0, "demanded");
        let mut wc = fc4_neighbors_shadow_city::default();
        assert!(fc4_neighbors_shadow_city_get(p, W, &mut wc));
        assert_eq!(wc.stats[0][1], 35.0);
        // The active deal stays; the month's update changed the date.
        let mut out = [fc4_neighbor_deal_data::default(); 2];
        assert_eq!(fc4_neighbors_deal_data(p, out.as_mut_ptr(), out.len()), 1);
        assert_eq!(out[0].current_date, 1000);
        assert_eq!(fc4_neighbors_total_expense(p), 90);
        let mut k = fc4_neighbors_shadow_counters::default();
        assert!(fc4_neighbors_shadow_get_counters(p, &mut k));
        assert_eq!(k.sends, 0);
        assert!(k.set_local_deals >= 2);
        // Nothing reached the game: there is no game.
        fc4_neighbors_destroy(p);
    }
    // A real handle is not a shadow.
    let game = game();
    let h = host(&game);
    unsafe {
        let p = fc4_neighbors_create(&h, 1, 0);
        let mut rc = fc4_neighbors_shadow_city::default();
        assert!(!fc4_neighbors_shadow_city_get(p, R, &mut rc));
        fc4_neighbors_destroy(p);
    }
    let _ = fc4_ref::default();
}

type GotProps = Vec<(u32, u16, bool, usize, Vec<u8>)>;

unsafe extern "C" fn collect_prop(
    ctx: *mut c_void,
    id: u32,
    mem_type: u16,
    is_array: bool,
    data: *const c_void,
    n: usize,
) {
    let got = &mut *(ctx as *mut GotProps);
    let size = match mem_type {
        0 => 1,
        4 => 2,
        6 => 4,
        _ => 0,
    };
    let bytes = if data.is_null() || n * size == 0 {
        Vec::new()
    } else {
        std::slice::from_raw_parts(data as *const u8, n * size).to_vec()
    };
    got.push((id, mem_type, is_array, n, bytes));
}

/// The connection's holder after a `Read` (foundations §10.2): arrays reach
/// the host too (the trip capacity is a u16 array), with their element type;
/// scalars as before.
#[test]
fn neighbors_connection_props_deliver_arrays() {
    unsafe {
        let c = fc4_neighbor_connection_create();
        let p = crate::props::fc4_props_create();
        let caps: [u16; 3] = [5, 4321, 0];
        assert!(crate::props::fc4_props_set_typed(
            p,
            0x8A0B_4100,
            4,
            true,
            caps.as_ptr() as *const c_void,
            3
        ));
        let seven: u32 = 7;
        assert!(crate::props::fc4_props_set_typed(
            p,
            0xAA9E_A81D,
            6,
            false,
            &seven as *const u32 as *const c_void,
            1
        ));
        // NG37: raw bytes (what AddPropertyData stores), the void array.
        let raw: [u8; 3] = [1, 2, 3];
        assert!(crate::props::fc4_props_set_typed(
            p,
            0x5AFE_0300,
            0,
            true,
            raw.as_ptr() as *const c_void,
            3
        ));
        assert!(!crate::props::fc4_props_set_typed(
            p,
            0x5AFE_0301,
            0,
            false,
            raw.as_ptr() as *const c_void,
            1
        ));
        fc4_neighbor_connection_set_props(c, p);
        crate::props::fc4_props_destroy(p);
        let mut got: GotProps = Vec::new();
        let n = fc4_neighbor_connection_props(c, Some(collect_prop), &mut got as *mut GotProps as *mut c_void);
        assert_eq!(n, 3);
        assert_eq!(
            got,
            vec![
                (0x5AFE_0300, 0, true, 3, vec![1, 2, 3]),
                (0x8A0B_4100, 4, true, 3, vec![5, 0, 0xE1, 0x10, 0, 0]),
                (0xAA9E_A81D, 6, false, 1, vec![7, 0, 0, 0]),
            ]
        );
        fc4_neighbor_connection_destroy(c);
    }
}
