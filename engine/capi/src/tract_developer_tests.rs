// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The tract developer C ABI against a small fake game: the identity, `Init`
//! and `Shutdown` (agents, message server, tuning, the filter freed), the
//! style slots, the save record byte for byte and its round trip, `Grow` on
//! an empty tract and on a tract with lots (one suitable configuration, the
//! demolish-then-build path), `Decline`, `UpdatePopulation`, the
//! configuration filter, re-entrancy and shadow mode (writes swallowed,
//! `SetPopulation` captured, the probes).

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::c_void;

use fc4_core::tract_developer::Tuning;

use crate::budget::fc4_token_callback;
use crate::demand::fc4_u32_callback;
use crate::hostgrid::fc4_cell_rect;
use crate::props::fc4_props;
use crate::test_streams::{istream, ostream, In, Out};
use crate::tract_developer::*;

const DEV: u64 = 1;
const CFG: u64 = 100;
const FILTER: u64 = 50;
const FAMILY: u32 = 600;
const BUILDING: u32 = 0x300;

#[derive(Clone, Copy)]
struct Lot {
    x: i32,
    z: i32,
    state: u32,
    capacity: u16,
    building: u32,
    wealth: u32,
}

/// Residential developer 1, 1 x 1 lots, one configuration (100, family 600
/// = {0x300}, capacity 2), demand 3, unlimited resources.
#[derive(Default)]
struct World {
    lots: RefCell<BTreeMap<u64, Lot>>,
    next: Cell<u64>,
    log: RefCell<Vec<String>>,
    no_tuning: Cell<bool>,
    /// A handle the growth developer calls back into (re-entrancy test).
    reenter: Cell<usize>,
    reentered: RefCell<Vec<u32>>,
}

fn w(u: *mut c_void) -> &'static World {
    unsafe { &*(u as *const World) }
}

impl World {
    fn with_lots(cells: &[(i32, i32)]) -> Box<World> {
        let g = Box::new(World {
            next: Cell::new(1000),
            ..Default::default()
        });
        for (i, &(x, z)) in cells.iter().enumerate() {
            g.lots.borrow_mut().insert(
                i as u64 + 1,
                Lot {
                    x,
                    z,
                    state: 1,
                    capacity: 0,
                    building: 0,
                    wealth: 1,
                },
            );
        }
        g
    }
    fn say(&self, s: String) {
        self.log.borrow_mut().push(s);
    }
    fn count(&self, prefix: &str) -> usize {
        self.log.borrow().iter().filter(|s| s.starts_with(prefix)).count()
    }
    fn lot<T>(&self, l: u64, f: impl FnOnce(&Lot) -> T, def: T) -> T {
        self.lots.borrow().get(&l).map_or(def, f)
    }
}

// ---- callbacks ---------------------------------------------------------------------------------------

unsafe extern "C" fn acquire_services(u: *mut c_void) -> bool {
    w(u).say("acquire".into());
    true
}
unsafe extern "C" fn add_agent(u: *mut c_void, level: u32, msg: u32) -> bool {
    w(u).say(format!("agent {level} {msg:#x}"));
    true
}
unsafe extern "C" fn acquire_message_server(u: *mut c_void) -> bool {
    w(u).say("ms".into());
    true
}
unsafe extern "C" fn release_message_server(u: *mut c_void) {
    w(u).say("ms release".into());
}
unsafe extern "C" fn remove_agent(u: *mut c_void) {
    w(u).say("agent remove".into());
}
unsafe extern "C" fn release_services(u: *mut c_void) {
    w(u).say("release services".into());
}
unsafe extern "C" fn tuning(u: *mut c_void) -> *mut fc4_props {
    if w(u).no_tuning.get() {
        return std::ptr::null_mut();
    }
    Box::into_raw(Box::new(Tuning::shipped_exemplar())) as *mut fc4_props
}
unsafe extern "C" fn dev_target_zone(u: *mut c_void, _: u64) -> i32 {
    let g = w(u);
    if g.reenter.get() != 0 {
        let p = g.reenter.get() as *mut fc4_tract;
        g.reentered.borrow_mut().push(fc4_tract_current_style(p));
        g.reentered.borrow_mut().push(fc4_tract_grow(
            p,
            &fc4_tract_profile::default(),
            None,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        ) as u32);
    }
    1
}
unsafe extern "C" fn dev_subtype(_: *mut c_void, _: u64) -> u32 {
    0x1010
}
unsafe extern "C" fn one_u(_: *mut c_void, _: u64) -> u32 {
    1
}
unsafe extern "C" fn dev_construction_info(_: *mut c_void, _: u64, o: *mut fc4_tract_construction) {
    *o = fc4_tract_construction {
        zone: 1,
        wealth: 1,
        purpose: 1,
        ..Default::default()
    };
}
unsafe extern "C" fn tract_size(_: *mut c_void, _: u64) -> i32 {
    4
}
unsafe extern "C" fn tract_shift(_: *mut c_void, _: u64) -> u32 {
    2
}
unsafe extern "C" fn desirability(_: *mut c_void, _: u64, _: i32, _: i32) -> i16 {
    100
}
unsafe extern "C" fn source_demand(_: *mut c_void, _: u64, out: *mut f32) -> bool {
    *out = 3.0;
    true
}
unsafe extern "C" fn dev_cap(_: *mut c_void, _: u64, _: u32) -> u8 {
    255
}
unsafe extern "C" fn monopoly(_: *mut c_void, _: u64) -> i32 {
    -1
}
unsafe extern "C" fn building_profile(_: *mut c_void, _: u32, o: *mut fc4_tract_building_profile) {
    *o = fc4_tract_building_profile {
        purpose: 1,
        wealth: 1,
        capacity: 2,
        reserved: 0,
    };
}
unsafe extern "C" fn types_by_family(_: *mut c_void, family: u32, cb: fc4_u32_callback, ctx: *mut c_void) {
    if family == FAMILY {
        cb.unwrap()(ctx, BUILDING);
    }
}
unsafe extern "C" fn in_group(_: *mut c_void, _: u32, _: u32) -> bool {
    true
}
unsafe extern "C" fn developer_for(_: *mut c_void, _: u32, _: u32) -> u64 {
    DEV
}
unsafe extern "C" fn claim(u: *mut c_void, amount: u16) -> bool {
    w(u).say(format!("claim {amount}"));
    true
}
unsafe extern "C" fn release(u: *mut c_void, amount: u16) {
    w(u).say(format!("release {amount}"));
}
unsafe extern "C" fn start_construction(u: *mut c_void, lot: u64, cfg: u64, building: u32, _: bool) -> bool {
    let g = w(u);
    g.say(format!("start {lot} {cfg} {building:#x}"));
    if let Some(l) = g.lots.borrow_mut().get_mut(&lot) {
        l.state = 2;
        l.building = building;
        l.capacity = 2;
    }
    true
}
unsafe extern "C" fn abandon(u: *mut c_void, lot: u64, reason: u32) -> bool {
    let g = w(u);
    g.say(format!("abandon {lot} {reason}"));
    if let Some(l) = g.lots.borrow_mut().get_mut(&lot) {
        l.state = 4;
    }
    true
}
unsafe extern "C" fn reoccupy(u: *mut c_void, lot: u64, _: u32) -> bool {
    w(u).say(format!("reoccupy {lot}"));
    true
}
unsafe extern "C" fn get_lot(u: *mut c_void, x: i32, z: i32) -> u64 {
    w(u).lots
        .borrow()
        .iter()
        .find(|(_, l)| l.x == x && l.z == z)
        .map_or(0, |(&k, _)| k)
}
unsafe extern "C" fn lots_in_rect(u: *mut c_void, r: *const fc4_cell_rect, cb: fc4_token_callback, ctx: *mut c_void) {
    let r = &*r;
    let v: Vec<u64> = w(u)
        .lots
        .borrow()
        .iter()
        .filter(|(_, l)| l.x >= r.x1 && l.x <= r.x2 && l.z >= r.z1 && l.z <= r.z2)
        .map(|(&k, _)| k)
        .collect();
    for t in v {
        cb.unwrap()(ctx, t);
    }
}
unsafe extern "C" fn can_create(_: *mut c_void, _: i32, _: i32, _: i32, _: i32) -> i32 {
    7
}
unsafe extern "C" fn create_lot(u: *mut c_void, x: i32, z: i32, _: i32, _: i32, _: i32) -> u64 {
    let g = w(u);
    let t = g.next.get();
    g.next.set(t + 1);
    g.lots.borrow_mut().insert(
        t,
        Lot {
            x,
            z,
            state: 1,
            capacity: 0,
            building: 0,
            wealth: 1,
        },
    );
    t
}
unsafe extern "C" fn delete_lot(u: *mut c_void, lot: u64) -> bool {
    w(u).lots.borrow_mut().remove(&lot).is_some()
}
unsafe extern "C" fn lot_rect(u: *mut c_void, lot: u64, o: *mut fc4_cell_rect) {
    *o = w(u).lot(
        lot,
        |l| fc4_cell_rect {
            x1: l.x,
            z1: l.z,
            x2: l.x,
            z2: l.z,
        },
        fc4_cell_rect {
            x1: 0,
            z1: 0,
            x2: -1,
            z2: -1,
        },
    );
}
unsafe extern "C" fn lot_location(u: *mut c_void, lot: u64, x: *mut i32, z: *mut i32) {
    let (lx, lz) = w(u).lot(lot, |l| (l.x, l.z), (0, 0));
    *x = lx;
    *z = lz;
}
unsafe extern "C" fn one_by_one(_: *mut c_void, _: u64, a: *mut u32, b: *mut u32) {
    *a = 1;
    *b = 1;
}
unsafe extern "C" fn yes_lot(_: *mut c_void, _: u64) -> bool {
    true
}
unsafe extern "C" fn yes_lot_u(_: *mut c_void, _: u64, _: u32) -> bool {
    true
}
unsafe extern "C" fn zone_one_lot(_: *mut c_void, _: u64) -> i32 {
    1
}
unsafe extern "C" fn lot_state(u: *mut c_void, lot: u64) -> u32 {
    w(u).lot(lot, |l| l.state, 0)
}
unsafe extern "C" fn lot_set_state(u: *mut c_void, lot: u64, state: u32, _: u32, _: u32) {
    if let Some(l) = w(u).lots.borrow_mut().get_mut(&lot) {
        l.state = state;
    }
}
unsafe extern "C" fn has_capacity(u: *mut c_void, lot: u64) -> bool {
    w(u).lot(lot, |l| l.capacity > 0, false)
}
unsafe extern "C" fn days(_: *mut c_void, _: u64) -> u32 {
    100
}
unsafe extern "C" fn occupant_wealth(u: *mut c_void, lot: u64) -> u32 {
    w(u).lot(lot, |l| l.wealth, 0)
}
unsafe extern "C" fn building_type(u: *mut c_void, lot: u64) -> u32 {
    w(u).lot(lot, |l| l.building, 0)
}
unsafe extern "C" fn capacity(u: *mut c_void, lot: u64, _: u32, _: bool) -> u16 {
    w(u).lot(lot, |l| l.capacity, 0)
}
unsafe extern "C" fn set_population(u: *mut c_void, lot: u64, _: u32, n: u16) {
    w(u).say(format!("population {lot} {n}"));
}
unsafe extern "C" fn lot_stage(_: *mut c_void, _: u64) -> i32 {
    1
}
unsafe extern "C" fn min_slope(_: *mut c_void, _: u64) -> f32 {
    -100.0
}
unsafe extern "C" fn max_slope(_: *mut c_void, _: u64) -> f32 {
    100.0
}
unsafe extern "C" fn stage_one(_: *mut c_void, _: u64) -> u8 {
    1
}
unsafe extern "C" fn max_cap(_: *mut c_void, _: u64) -> u16 {
    u16::MAX
}
unsafe extern "C" fn compat_zone(_: *mut c_void, _: u64, _: i32) -> bool {
    true
}
unsafe extern "C" fn compat_one(_: *mut c_void, cfg: u64, v: u32) -> bool {
    cfg == CFG && v == 1
}
unsafe extern "C" fn compat_style(_: *mut c_void, _: u64, _: u32) -> bool {
    true
}
unsafe extern "C" fn primary_ids(_: *mut c_void, _: u64, cb: fc4_u32_callback, ctx: *mut c_void) -> bool {
    cb.unwrap()(ctx, FAMILY);
    true
}
unsafe extern "C" fn register_filter(u: *mut c_void, wealth: u32, purpose: u32) -> u64 {
    w(u).say(format!("register {wealth} {purpose}"));
    FILTER
}
unsafe extern "C" fn unregister_filter(u: *mut c_void, f: u64) {
    w(u).say(format!("unregister {f}"));
}
unsafe extern "C" fn by_filter(_: *mut c_void, f: u64, cb: fc4_token_callback, ctx: *mut c_void) {
    if f == FILTER {
        cb.unwrap()(ctx, CFG);
    }
}
unsafe extern "C" fn zone_at(_: *mut c_void, _: i32, _: i32) -> i32 {
    1
}
unsafe extern "C" fn powered(_: *mut c_void, _: i32, _: i32) -> i32 {
    2
}
unsafe extern "C" fn watered(_: *mut c_void, _: i32, _: i32) -> bool {
    true
}
unsafe extern "C" fn fire(_: *mut c_void, _: i32, _: i32) -> u8 {
    1
}
unsafe extern "C" fn park(_: *mut c_void, _: i32, _: i32) -> i16 {
    1
}
unsafe extern "C" fn road_access(_: *mut c_void, _: i32, _: i32) -> i32 {
    1
}
unsafe extern "C" fn cell_counts(_: *mut c_void, x: *mut i32, z: *mut i32) {
    *x = 64;
    *z = 64;
}
unsafe extern "C" fn demolish(u: *mut c_void, lot: u64) -> bool {
    w(u).say(format!("demolish {lot}"));
    true
}

fn host(g: &World) -> fc4_tract_host {
    fc4_tract_host {
        struct_size: std::mem::size_of::<fc4_tract_host>() as u32,
        user: g as *const World as *mut c_void,
        acquire_services: Some(acquire_services),
        add_agent: Some(add_agent),
        acquire_message_server: Some(acquire_message_server),
        release_message_server: Some(release_message_server),
        remove_agent: Some(remove_agent),
        release_services: Some(release_services),
        tuning: Some(tuning),
        dev_target_zone: Some(dev_target_zone),
        dev_subtype: Some(dev_subtype),
        dev_purpose: Some(one_u),
        dev_wealth: Some(one_u),
        dev_construction_info: Some(dev_construction_info),
        dev_desirability_tract_size: Some(tract_size),
        dev_desirability_shift: Some(tract_shift),
        dev_desirability_value: Some(desirability),
        dev_capacity_value: None,
        dev_source_demand: Some(source_demand),
        dev_cap: Some(dev_cap),
        dev_monopoly_use: Some(monopoly),
        dev_style_percentages: None,
        building_profile: Some(building_profile),
        building_types_by_family: Some(types_by_family),
        building_in_occupant_group: Some(in_group),
        building_is_monopoly: None,
        building_field_ids: None,
        developer_for: Some(developer_for),
        claim_construction_resources: Some(claim),
        release_construction_resources: Some(release),
        cheat: None,
        needs_anchor_building: None,
        start_construction: Some(start_construction),
        abandon_building: Some(abandon),
        reoccupy_building: Some(reoccupy),
        area_slope: None,
        get_lot: Some(get_lot),
        lots_in_cell_rect: Some(lots_in_rect),
        lots_surrounding_lot: None,
        lots_surrounding_rect: None,
        can_create_lot: Some(can_create),
        create_lot_if_possible: Some(create_lot),
        delete_lot: Some(delete_lot),
        lot_bounding_rect: Some(lot_rect),
        lot_slope: None,
        lot_location: Some(lot_location),
        lot_size: Some(one_by_one),
        lot_facing: None,
        lot_set_facing: None,
        lot_has_road_access: Some(yes_lot),
        lot_has_organic_road_access: Some(yes_lot_u),
        lot_zone_type: Some(zone_one_lot),
        lot_state: Some(lot_state),
        lot_set_state: Some(lot_set_state),
        lot_is_livable: None,
        lot_has_capacity: Some(has_capacity),
        lot_days_since_state_change: Some(days),
        lot_occupant_wealth: Some(occupant_wealth),
        lot_is_historical: None,
        lot_building_type: Some(building_type),
        lot_capacity: Some(capacity),
        lot_available_capacity: Some(capacity),
        lot_set_population: Some(set_population),
        lot_configuration_stage: Some(lot_stage),
        cfg_size: Some(one_by_one),
        cfg_min_slope: Some(min_slope),
        cfg_max_slope: Some(max_slope),
        cfg_growth_stage: Some(stage_one),
        cfg_min_capacity: None,
        cfg_max_capacity: Some(max_cap),
        cfg_compatible_zone: Some(compat_zone),
        cfg_compatible_wealth: Some(compat_one),
        cfg_compatible_purpose: Some(compat_one),
        cfg_compatible_style: Some(compat_style),
        cfg_required_roads: None,
        cfg_primary_building_ids: Some(primary_ids),
        get_lot_configuration: None,
        register_configuration_filter: Some(register_filter),
        unregister_configuration_filter: Some(unregister_filter),
        configurations_by_filter: Some(by_filter),
        zone_at: Some(zone_at),
        land_value: None,
        powered_value: Some(powered),
        is_cell_watered: Some(watered),
        fire_coverage: Some(fire),
        park_value: Some(park),
        road_access_value: Some(road_access),
        city_cell_counts: Some(cell_counts),
        find_network_tile: None,
        network_is_of_type: None,
        demolish_lot: Some(demolish),
        redisplay_terrain: None,
    }
}

fn profile(x: i32, z: i32, stage: i32) -> fc4_tract_profile {
    fc4_tract_profile {
        developer: DEV,
        x,
        z,
        stage,
        reserved: 0,
    }
}

fn state(p: *const fc4_tract) -> fc4_tract_state {
    let mut s = fc4_tract_state::default();
    assert!(unsafe { fc4_tract_get_state(p, &mut s) });
    s
}

unsafe extern "C" fn push_code(ctx: *mut c_void, v: u32) {
    (*(ctx as *mut Vec<u32>)).push(v);
}

unsafe extern "C" fn push_population(ctx: *mut c_void, lot: u64, dev_type: u32, n: u16) {
    (*(ctx as *mut Vec<(u64, u32, u16)>)).push((lot, dev_type, n));
}

fn write(p: *const fc4_tract) -> Vec<u8> {
    let mut o = Out::default();
    let s = ostream(&mut o);
    assert!(unsafe { fc4_tract_write(p, &s) });
    o.bytes
}

// ---- tests -------------------------------------------------------------------------------------------

#[test]
fn identity_and_incomplete_hosts() {
    for iid in [1, 0xA818_ED1B, 0x4522_94AA, 0xE4FD_A3D4] {
        assert!(fc4_tract_query_interface(iid), "{iid:#x}");
    }
    for iid in [0, 0x2990_C142, 0x6990_C1AA, 0xFFFF_FFFF] {
        assert!(!fc4_tract_query_interface(iid), "{iid:#x}");
    }
    assert_eq!(fc4_tract_class_id(), 0x2990_C142);
    let g = World::with_lots(&[]);
    let mut h = host(&g);
    h.struct_size -= 4;
    unsafe {
        assert!(fc4_tract_create(&h, 1).is_null());
        assert!(fc4_tract_create_shadow(&h).is_null());
        assert!(fc4_tract_create(std::ptr::null(), 1).is_null());
        // NULL objects answer the fallbacks.
        let n = std::ptr::null_mut();
        assert!(!fc4_tract_init(n));
        assert!(fc4_tract_shutdown(n));
        assert_eq!(fc4_tract_current_style(n), 0);
        assert_eq!(fc4_tract_grow(n, &profile(0, 0, 1), None, n as _, n as _, n as _), 0);
        assert_eq!(fc4_tract_probe_lot_suitable(n, 1), -1);
        fc4_tract_destroy(n);
    }
}

#[test]
fn init_shutdown_and_tuning() {
    let g = World::with_lots(&[]);
    let h = host(&g);
    unsafe {
        let p = fc4_tract_create(&h, 0x1234);
        assert!(!p.is_null());
        let s = state(p);
        assert_eq!((s.initialized, s.configured), (0, 0));
        // Constructor: years between 0 until the tuning is read.
        assert!(fc4_tract_init(p));
        let s = state(p);
        assert_eq!((s.initialized, s.configured), (1, 1));
        assert_eq!(
            g.log.borrow().as_slice(),
            ["acquire", "agent 6 0x66956816", "agent 7 0x66956817", "ms"]
        );
        // Shipped tuning (§4.21): years 5, four available styles, which
        // become the active ones.
        assert_eq!(fc4_tract_years_between_style_changes(p), 5);
        let mut t = fc4_tract_tuning::default();
        assert!(fc4_tract_get_tuning(p, &mut t));
        assert_eq!(t.overbuild, 1.5);
        assert_eq!(t.reoccupy, [0.0, 1.2]);
        assert_eq!(t.kick_out, 1);
        assert_eq!(t.available_style_count, 4);
        assert_eq!(&t.available_styles[..4], &[0x2000, 0x2001, 0x2002, 0x2003]);
        assert_eq!(fc4_tract_active_styles(p, std::ptr::null_mut(), 0), 4);
        assert_eq!(fc4_tract_current_style(p), 0x2000);
        // Grow registers one filter; Shutdown unregisters it and releases.
        let _ = fc4_tract_grow(
            p,
            &profile(10, 10, 1),
            None,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        assert_eq!(g.count("register 1 1"), 1);
        assert_eq!(state(p).filters, 1);
        g.log.borrow_mut().clear();
        assert!(fc4_tract_shutdown(p));
        let log = g.log.borrow().clone();
        assert!(log.contains(&format!("unregister {FILTER}")), "{log:?}");
        assert!(log.contains(&"agent remove".to_string()));
        assert!(log.contains(&"ms release".to_string()));
        assert!(log.contains(&"release services".to_string()));
        assert_eq!(state(p).filters, 0);
        // The style state survives Shutdown.
        assert_eq!(fc4_tract_active_styles(p, std::ptr::null_mut(), 0), 4);
        fc4_tract_destroy(p);
    }
}

#[test]
fn style_slots() {
    let g = World::with_lots(&[]);
    g.no_tuning.set(true);
    let h = host(&g);
    unsafe {
        let p = fc4_tract_create(&h, 7);
        fc4_tract_set_active_styles(p, [0x2001u32, 0x2003].as_ptr(), 2);
        let mut out = [0u32; 1];
        assert_eq!(fc4_tract_active_styles(p, out.as_mut_ptr(), out.len()), 2);
        assert_eq!(out, [0x2001]);
        assert_eq!(fc4_tract_current_style(p), 0x2001);
        assert!(!fc4_tract_is_using_all_styles(p));
        fc4_tract_use_all_styles(p, true);
        assert!(fc4_tract_is_using_all_styles(p));
        fc4_tract_set_years_between_style_changes(p, 3);
        assert_eq!(fc4_tract_years_between_style_changes(p), 3);
        // Year messages rotate after three years; Month does not.
        fc4_tract_use_all_styles(p, false);
        for _ in 0..3 {
            assert!(fc4_tract_do_message(p, 0x6695_6816));
        }
        assert_eq!(fc4_tract_current_style(p), 0x2001);
        for _ in 0..3 {
            assert!(fc4_tract_do_message(p, 0x6695_6817));
        }
        assert_eq!(fc4_tract_current_style(p), 0x2003);
        fc4_tract_set_active_styles(p, std::ptr::null(), 0);
        assert_eq!(fc4_tract_active_styles(p, std::ptr::null_mut(), 0), 0);
        assert_eq!(fc4_tract_current_style(p), 0);
        fc4_tract_destroy(p);
    }
}

#[test]
fn save_record_bytes_and_round_trip() {
    let g = World::with_lots(&[]);
    g.no_tuning.set(true);
    let h = host(&g);
    unsafe {
        let a = fc4_tract_create(&h, 7);
        fc4_tract_set_active_styles(a, [0x2000u32, 0x2002, 0x2003].as_ptr(), 3);
        fc4_tract_use_all_styles(a, true);
        fc4_tract_set_years_between_style_changes(a, 9);
        let bytes = write(a);
        // §6: u16 3, i32 index, i32 yearsSince, u8 single, u32 n, n x u32, i32 yearsBetween.
        let mut want = Vec::new();
        want.extend_from_slice(&3u16.to_le_bytes());
        want.extend_from_slice(&0i32.to_le_bytes());
        want.extend_from_slice(&0i32.to_le_bytes());
        // singleStyle = !IsUsingAllStylesAtOnce.
        want.push(0);
        want.extend_from_slice(&3u32.to_le_bytes());
        for s in [0x2000u32, 0x2002, 0x2003] {
            want.extend_from_slice(&s.to_le_bytes());
        }
        want.extend_from_slice(&9i32.to_le_bytes());
        assert_eq!(bytes, want);

        let b = fc4_tract_create(&h, 7);
        let mut i = In::new(&bytes);
        let s = istream(&mut i);
        assert!(fc4_tract_read(b, &s));
        assert_eq!(write(b), bytes);
        assert!(fc4_tract_is_using_all_styles(b));
        assert_eq!(fc4_tract_years_between_style_changes(b), 9);
        // Every truncation fails.
        for cut in 0..bytes.len() {
            let c = fc4_tract_create(&h, 7);
            let mut i = In::new(&bytes[..cut]);
            let s = istream(&mut i);
            assert!(!fc4_tract_read(c, &s), "cut {cut}");
            fc4_tract_destroy(c);
        }
        // A newer version is refused.
        let mut i = In::new(&[4, 0, 0, 0, 0, 0]);
        let s = istream(&mut i);
        assert!(!fc4_tract_read(b, &s));
        assert_eq!(fc4_tract_years_between_style_changes(b), 9);
        fc4_tract_destroy(a);
        fc4_tract_destroy(b);
    }
}

#[test]
fn grow_on_an_empty_tract() {
    let g = World::with_lots(&[]);
    let h = host(&g);
    unsafe {
        let p = fc4_tract_create(&h, 7);
        assert!(fc4_tract_init(p));
        let mut codes: Vec<u32> = Vec::new();
        let mut flag = true;
        let mut r = fc4_tract_grow_result::default();
        let built = fc4_tract_grow(
            p,
            &profile(0, 0, 1),
            Some(push_code),
            &mut codes as *mut Vec<u32> as *mut c_void,
            &mut flag,
            &mut r,
        );
        assert_eq!(built, 0);
        assert!(!flag);
        // Two passes (strict, relaxed), each without a candidate.
        assert_eq!(codes.len(), 2, "{codes:?}");
        assert!(codes.iter().all(|&c| c != 27));
        assert_eq!(r.struct_size as usize, std::mem::size_of::<fc4_tract_grow_result>());
        assert_eq!(r.outcome_count, 2);
        assert_eq!(&r.outcomes[..2], codes.as_slice());
        assert_eq!(r.scored, 0);
        assert_eq!((r.desired, r.desired_max), (3, 4));
        assert_eq!(g.count("start"), 0);
        let s = state(p);
        assert_eq!((s.grow_calls, s.built_total, s.constructions), (1, 0, 0));
        assert_eq!(s.outcomes.iter().sum::<u32>(), 2);
        fc4_tract_destroy(p);
    }
}

#[test]
fn grow_demolishes_and_builds_until_desired() {
    // Demand 3: desired 3 (max 4). Each 1 x 1 lot takes the capacity-2
    // building of the one suitable configuration: pass 1 builds once
    // (built 2), pass 2 again (built 4). Existing lots are demolished
    // before the construction starts (§4.15.1).
    let g = World::with_lots(&[(0, 0), (1, 0), (2, 0)]);
    let h = host(&g);
    unsafe {
        let p = fc4_tract_create(&h, 7);
        assert!(fc4_tract_init(p));
        fc4_tract_set_histogram(p, true);
        let mut codes: Vec<u32> = Vec::new();
        let mut r = fc4_tract_grow_result::default();
        let built = fc4_tract_grow(
            p,
            &profile(0, 0, 1),
            Some(push_code),
            &mut codes as *mut Vec<u32> as *mut c_void,
            std::ptr::null_mut(),
            &mut r,
        );
        assert_eq!(built, 4);
        assert_eq!(codes, [27, 27]);
        assert_eq!(r.chosen_cfg, CFG);
        assert_eq!(r.chosen_building, BUILDING);
        assert!(r.scored >= 2);
        let log = g.log.borrow().clone();
        let order: Vec<&String> = log
            .iter()
            .filter(|s| s.starts_with("demolish") || s.starts_with("start") || s.starts_with("claim"))
            .collect();
        assert_eq!(order.len(), 6, "{log:?}");
        for c in order.chunks(3) {
            assert_eq!(c[0], "claim 2");
            assert!(c[1].starts_with("demolish "));
            let lot = &c[1]["demolish ".len()..];
            assert_eq!(*c[2], format!("start {lot} {CFG} {BUILDING:#x}"));
        }
        let s = state(p);
        assert_eq!((s.constructions, s.demolitions, s.resource_claims), (2, 2, 2));
        assert_eq!(s.outcomes[27], 2);
        assert_eq!(s.built_total, 4);
        let mut hist = vec![0u32; 13 * 28];
        assert!(fc4_tract_histogram(p, hist.as_mut_ptr(), hist.len()));
        assert!(hist.iter().any(|&n| n > 0));
        assert!(!fc4_tract_histogram(p, hist.as_mut_ptr(), 10));
        fc4_tract_destroy(p);
    }
}

#[test]
fn decline_and_update_population() {
    let g = World::with_lots(&[(0, 0)]);
    {
        let mut lots = g.lots.borrow_mut();
        let l = lots.get_mut(&1).unwrap();
        l.state = 3;
        l.building = BUILDING;
        l.capacity = 40;
    }
    let h = host(&g);
    unsafe {
        let p = fc4_tract_create(&h, 7);
        // Constructor tuning (no Init): tract (0, 0) uses seed 1 -> 36 (§4.18).
        fc4_tract_update_population(p, &profile(0, 0, 0));
        assert_eq!(g.log.borrow().as_slice(), ["population 1 36"]);
        assert!(fc4_tract_init(p));
        assert_eq!(fc4_tract_decline(p, &profile(0, 0, 0)), 40);
        assert_eq!(g.lot(1, |l| l.state, 0), 4);
        assert_eq!(fc4_tract_decline(p, &profile(0, 0, 0)), 0);
        let s = state(p);
        assert_eq!(
            (s.decline_calls, s.declined_total, s.update_population_calls),
            (2, 40, 1)
        );
        assert_eq!(s.populations_set, 1);
        fc4_tract_destroy(p);
    }
}

#[test]
fn configuration_filter() {
    let g = World::with_lots(&[]);
    let h = host(&g);
    unsafe {
        assert!(fc4_tract_filter_includes(&h, CFG, 1, 1));
        assert!(!fc4_tract_filter_includes(&h, CFG, 2, 1));
        assert!(!fc4_tract_filter_includes(&h, CFG, 1, 2));
        assert!(!fc4_tract_filter_includes(&h, CFG + 1, 1, 1));
        assert!(!fc4_tract_filter_includes(std::ptr::null(), CFG, 1, 1));
    }
}

#[test]
fn reentrant_calls_get_the_fallback() {
    let g = World::with_lots(&[]);
    let h = host(&g);
    unsafe {
        let p = fc4_tract_create(&h, 7);
        assert!(fc4_tract_init(p));
        g.reenter.set(p as usize);
        let _ = fc4_tract_grow(
            p,
            &profile(0, 0, 1),
            None,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        g.reenter.set(0);
        let got = g.reentered.borrow().clone();
        assert!(!got.is_empty());
        assert!(got.iter().all(|&v| v == 0), "{got:?}");
        assert!(state(p).reentrant >= 2);
        // The object still works afterwards.
        assert_eq!(fc4_tract_current_style(p), 0x2000);
        fc4_tract_destroy(p);
    }
}

#[test]
fn shadow_swallows_writes_and_captures_population() {
    let g = World::with_lots(&[(0, 0), (1, 0)]);
    {
        let mut lots = g.lots.borrow_mut();
        for l in lots.values_mut() {
            l.state = 3;
            l.building = BUILDING;
            l.capacity = 40;
        }
    }
    let h = host(&g);
    unsafe {
        let real = fc4_tract_create(&h, 7);
        // Probes refuse a replacement core.
        assert!(!fc4_tract_probe_setup(real, &profile(0, 0, 1), true));
        assert_eq!(fc4_tract_probe_configuration_suitable(real, CFG), -1);
        assert_eq!(fc4_tract_probe_lot_suitable(real, 1), -1);
        assert_eq!(fc4_tract_probe_building_compatible(real, BUILDING, 0, i32::MAX), -1);
        assert_eq!(
            fc4_tract_shadow_update_population(real, &profile(0, 0, 0), None, std::ptr::null_mut()),
            -1
        );

        let sh = fc4_tract_create_shadow(&h);
        assert!(!sh.is_null());
        assert!(fc4_tract_init(sh));
        // Init reads the tuning but registers nothing.
        assert!(g.log.borrow().is_empty(), "{:?}", g.log.borrow());
        assert_eq!(fc4_tract_years_between_style_changes(sh), 5);
        let mut calls: Vec<(u64, u32, u16)> = Vec::new();
        let n = fc4_tract_shadow_update_population(
            sh,
            &profile(0, 0, 0),
            Some(push_population),
            &mut calls as *mut Vec<(u64, u32, u16)> as *mut c_void,
        );
        assert_eq!(n, 2);
        assert_eq!(calls.len(), 2);
        assert!(g.log.borrow().is_empty());
        // The replacement core computes the same populations.
        assert!(fc4_tract_init(real));
        g.log.borrow_mut().clear();
        fc4_tract_update_population(real, &profile(0, 0, 0));
        let want: Vec<String> = calls.iter().map(|(l, _, n)| format!("population {l} {n}")).collect();
        assert_eq!(*g.log.borrow(), want);
        assert!(calls.iter().all(|&(_, t, _)| t == 0x1010));
        g.log.borrow_mut().clear();

        // Grow and Decline on a shadow core change nothing in the game.
        let _ = fc4_tract_grow(
            sh,
            &profile(0, 0, 1),
            None,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        );
        let _ = fc4_tract_decline(sh, &profile(0, 0, 0));
        assert!(g.log.borrow().is_empty(), "{:?}", g.log.borrow());
        assert_eq!(g.lot(1, |l| l.state, 0), 3);
        let s = state(sh);
        assert!(s.swallowed > 0);

        // Probes: the occupied lot's own configuration and building pass the
        // checks that do not depend on the moment.
        assert!(fc4_tract_probe_setup(sh, &profile(0, 0, 255), true));
        assert_eq!(fc4_tract_probe_configuration_suitable(sh, CFG), 27);
        assert_eq!(fc4_tract_probe_configuration_suitable(sh, CFG + 1), 7);
        let r = fc4_tract_probe_building_compatible(sh, BUILDING, 0, i32::MAX);
        assert!(r != 7 && r != 8, "{r}");
        let r = fc4_tract_probe_lot_suitable(sh, 1);
        assert!(r >= 0 && r != 6 && r != 15 && r != 16, "{r}");
        assert!(g.log.borrow().is_empty());
        // Shutdown on a shadow core releases nothing it never took.
        assert!(fc4_tract_shutdown(sh));
        assert!(g.log.borrow().is_empty());
        fc4_tract_destroy(sh);
        fc4_tract_destroy(real);
    }
}
