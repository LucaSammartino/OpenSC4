// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The ordinance C ABI against a fake game: loading the standard ordinances
//! from exemplars through fc4_props, the month-end, enact/repeal accounting,
//! cumulative queries, a foreign ordinance called through the host, and
//! persistence of the simulator and its ordinances.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::c_void;

use crate::budget::fc4_token_callback;
use crate::ordinance::*;
use crate::props::*;
use crate::stream::{fc4_bytes_sink, fc4_ref};
use crate::test_streams::{istream, ostream, In, Out};

const EXEMPLAR: u32 = 0x6534_284A;
const GROUP: u32 = 0xA9C2_C209;
const CPR: u32 = 0x00D0_723D;
const WATCH: u32 = 0x62BF_1DB9;
const FOREIGN: u64 = 0xF00;
const FOREIGN_ID: u32 = 0x1234_5678;
const FACTOR_PROP: u32 = 0x28ED_0380;
const OFFSET_PROP: u32 = 0xAA5B_8407;

const NEW_MONTH: u32 = 0x6695_6816;
const UPDATED: u32 = 0xC2C0_9554;
const AVAILABILITY: u32 = 0xA2D1_C5B9;
const MONTHLY_DONE: u32 = 0xC8EA_5018;

#[derive(Clone, Copy)]
enum Val {
    U(u32),
    I(i32),
    F(f32),
}

/// (instance, properties); all in the ordinance group.
type Exemplar = (u32, Vec<(u32, Val)>);

#[derive(Default)]
struct ForeignState {
    available: bool,
    on: bool,
    adjusted: i64,
}

#[derive(Default)]
struct Game {
    host: Cell<Option<fc4_ordinance_host>>,
    /// (instance, properties); all in the ordinance group.
    exemplars: RefCell<Vec<Exemplar>>,
    /// token → (core object, references).
    objects: RefCell<BTreeMap<u64, (usize, i64)>>,
    next: Cell<u64>,
    posts: RefCell<Vec<(u32, u32, u32)>>,
    year: Cell<u32>,
    population: Cell<i32>,
    police: Cell<u32>,
    foreign: RefCell<ForeignState>,
    foreign_refs: Cell<i64>,
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

unsafe extern "C" fn for_each_key(u: *mut c_void, visit: fc4_key_visitor, ctx: *mut c_void) {
    let keys: Vec<u32> = game(u).exemplars.borrow().iter().map(|e| e.0).collect();
    for i in keys {
        visit.unwrap()(ctx, EXEMPLAR, GROUP, i);
    }
}

unsafe extern "C" fn exemplar(u: *mut c_void, t: u32, g: u32, i: u32) -> *mut fc4_props {
    if t != EXEMPLAR || g != GROUP {
        return std::ptr::null_mut();
    }
    let ex = game(u).exemplars.borrow();
    let Some((_, props)) = ex.iter().find(|e| e.0 == i) else {
        return std::ptr::null_mut();
    };
    let p = fc4_props_create();
    for (id, v) in props {
        assert!(match *v {
            Val::U(x) => fc4_props_set_u32s(p, *id, &x, 1),
            Val::I(x) => fc4_props_set_i32s(p, *id, &x, 1),
            Val::F(x) => fc4_props_set_f32s(p, *id, &x, 1),
        });
    }
    p
}

unsafe extern "C" fn create(u: *mut c_void, class: u32, token: *mut u64) -> *mut fc4_ordinance {
    let g = game(u);
    g.next.set(g.next.get() + 1);
    let t = 0x100 + g.next.get();
    let host = g.host.get().unwrap();
    let o = fc4_ordinance_create(&host, class, t);
    if o.is_null() {
        return o;
    }
    g.objects.borrow_mut().insert(t, (o as usize, 1));
    *token = t;
    o
}

unsafe extern "C" fn add_ref(u: *mut c_void, t: u64) {
    let g = game(u);
    if t == FOREIGN {
        g.foreign_refs.set(g.foreign_refs.get() + 1);
    } else if let Some(e) = g.objects.borrow_mut().get_mut(&t) {
        e.1 += 1;
    }
}

unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    let g = game(u);
    if t == FOREIGN {
        g.foreign_refs.set(g.foreign_refs.get() - 1);
        return;
    }
    let dead = {
        let mut objs = g.objects.borrow_mut();
        let Some(e) = objs.get_mut(&t) else { return };
        e.1 -= 1;
        if e.1 == 0 {
            objs.remove(&t).map(|e| e.0)
        } else {
            None
        }
    };
    if let Some(p) = dead {
        fc4_ordinance_destroy(p as *mut fc4_ordinance);
    }
}

unsafe extern "C" fn foreign_call(u: *mut c_void, t: u64, op: u32, arg: i64) -> i64 {
    assert_eq!(t, FOREIGN);
    let mut f = game(u).foreign.borrow_mut();
    match op {
        op::ID => i64::from(FOREIGN_ID),
        op::IS_AVAILABLE => i64::from(f.available),
        op::IS_ON => i64::from(f.available && f.on),
        op::IS_ENABLED | op::CHECK_CONDITIONS | op::SIMULATE => 1,
        op::ENACTMENT_INCOME => -5,
        op::RETRACMENT_INCOME => -1,
        op::MONTHLY_ADJUSTED_INCOME => f.adjusted,
        op::SET_AVAILABLE | op::FORCE_AVAILABLE => {
            f.available = arg != 0;
            1
        }
        op::SET_ON | op::FORCE_ON => {
            f.on = arg != 0;
            1
        }
        op::FORCE_MONTHLY_ADJUSTED_INCOME => {
            f.adjusted = arg;
            1
        }
        _ => 0,
    }
}

unsafe extern "C" fn foreign_text(_: *mut c_void, _t: u64, which: u32, sink: fc4_bytes_sink, ctx: *mut c_void) -> bool {
    let s: &[u8] = if which == 0 { b"Plugin Ordinance" } else { b"" };
    sink.unwrap()(ctx, s.as_ptr(), s.len());
    true
}

unsafe extern "C" fn foreign_misc(_: *mut c_void, _t: u64, prop: u32, out: *mut fc4_props) -> bool {
    if prop == FACTOR_PROP {
        let v = 0.5f32;
        fc4_props_set_f32s(out, prop, &v, 1)
    } else {
        false
    }
}

unsafe extern "C" fn post(u: *mut c_void, m: u32, d2: u32, d3: u32) {
    game(u).posts.borrow_mut().push((m, d2, d3));
}
unsafe extern "C" fn sim_year(u: *mut c_void, y: *mut u32) -> bool {
    *y = game(u).year.get();
    true
}
unsafe extern "C" fn population(u: *mut c_void) -> i32 {
    game(u).population.get()
}
unsafe extern "C" fn police(u: *mut c_void) -> u32 {
    game(u).police.get()
}
unsafe extern "C" fn no_lots(_: *mut c_void, _t: u32, _cb: fc4_token_callback, _ctx: *mut c_void) {}

fn host(g: &Game) -> fc4_ordinance_host {
    fc4_ordinance_host {
        struct_size: std::mem::size_of::<fc4_ordinance_host>() as u32,
        user: g as *const Game as *mut c_void,
        for_each_exemplar_key: Some(for_each_key),
        exemplar: Some(exemplar),
        ltext: None,
        create_ordinance: Some(create),
        add_ref: Some(add_ref),
        release: Some(release),
        foreign_ordinance: None,
        ordinance_call: Some(foreign_call),
        ordinance_text: Some(foreign_text),
        ordinance_misc_property: Some(foreign_misc),
        post: Some(post),
        sim_year: Some(sim_year),
        residential_population: Some(population),
        has_budget_simulator: None,
        department_total_expenses: None,
        line_item_full_expenses: None,
        landfill_capacity: None,
        power_plant_count: None,
        police_station_count: Some(police),
        recycling_center_effect: None,
        lots_with_building_type: Some(no_lots),
        demolish_lots: None,
        hide_conditional_building: None,
        remove_view_input_control: None,
    }
}

/// Exemplar type 14 ordinances: CPR (enact -20, repeal -10, constant -50,
/// factor -0.005, year 1990, a cumulative factor and offset) and
/// NeighborhoodWatch; a type-5 exemplar with a registered instance and a
/// type-14 one without a class produce nothing (§8.1).
fn city() -> Box<Game> {
    let g = Box::new(Game::default());
    g.year.set(1985);
    g.population.set(20_000);
    g.host.set(Some(host(&g)));
    let ord = |id: u32, extra: &[(u32, Val)]| {
        let mut v = vec![
            (0x10, Val::U(14)),
            (0x28EB_BC30, Val::U(id)),
            (0x28EB_BC35, Val::U(1990)),
            (0x28EB_BC37, Val::I(-20)),
            (0x28EB_BC38, Val::I(-10)),
            (0x28EB_BC39, Val::I(-50)),
            (0x28EB_BC3A, Val::F(-0.005)),
        ];
        v.extend_from_slice(extra);
        (id, v)
    };
    let mut ex = g.exemplars.borrow_mut();
    ex.push(ord(CPR, &[(FACTOR_PROP, Val::F(0.95)), (OFFSET_PROP, Val::I(-10))]));
    ex.push(ord(WATCH, &[]));
    ex.push((0xA2BF_1DDC, vec![(0x10, Val::U(5))]));
    ex.push(ord(0x1234_0000, &[]));
    drop(ex);
    g
}

unsafe fn text(o: *const fc4_ordinance, which: u32) -> Vec<u8> {
    unsafe extern "C" fn sink(ctx: *mut c_void, p: *const u8, n: usize) {
        (*(ctx as *mut Vec<u8>)).extend_from_slice(std::slice::from_raw_parts(p, n));
    }
    let mut v = Vec::new();
    assert!(fc4_ordinance_text(
        o,
        which,
        Some(sink),
        &mut v as *mut Vec<u8> as *mut c_void
    ));
    v
}

fn object(g: &Game, token: u64) -> *mut fc4_ordinance {
    g.objects
        .borrow()
        .get(&token)
        .map_or(std::ptr::null_mut(), |e| e.0 as *mut fc4_ordinance)
}

fn state(s: *const fc4_ordinance_sim) -> fc4_ordinance_sim_state {
    let mut st = fc4_ordinance_sim_state {
        struct_size: std::mem::size_of::<fc4_ordinance_sim_state>() as u32,
        ..Default::default()
    };
    assert!(unsafe { fc4_ordinance_sim_get_state(s, &mut st) });
    st
}

#[test]
fn null_safety() {
    unsafe {
        assert!(fc4_ordinance_sim_create(std::ptr::null(), 1).is_null());
        let g = Game::default();
        let mut short = host(&g);
        short.struct_size = 8;
        assert!(fc4_ordinance_sim_create(&short, 1).is_null());
        assert!(fc4_ordinance_create(&short, CPR, 1).is_null());
        let h = host(&g);
        assert!(fc4_ordinance_create(&h, 0x1234_0000, 1).is_null()); // not a standard class
        fc4_ordinance_sim_destroy(std::ptr::null_mut());
        fc4_ordinance_destroy(std::ptr::null_mut());
        let null = std::ptr::null_mut();
        assert!(!fc4_ordinance_sim_init(null));
        assert!(!fc4_ordinance_sim_add(null, 5, std::ptr::null()));
        assert_eq!(
            fc4_ordinance_sim_array(null, 0, std::ptr::null_mut(), std::ptr::null_mut()),
            0
        );
        assert_eq!(fc4_ordinance_sim_cumulative_factor(null, 1, 0), 1.0);
        assert!(fc4_ordinance_sim_handle_message(null, NEW_MONTH));
        assert_eq!(fc4_ordinance_call(std::ptr::null_mut(), op::ID, 0), 0);
        assert!(!fc4_ordinance_write(std::ptr::null(), std::ptr::null()));
        // Typed property values (the shim's exemplar copy).
        let p = fc4_props_create();
        let u16v: u16 = 7;
        assert!(fc4_props_set_typed(
            p,
            1,
            0x04,
            false,
            &u16v as *const u16 as *const c_void,
            1
        ));
        let fs = [1.5f32, 2.5];
        assert!(fc4_props_set_typed(p, 2, 0x0A, true, fs.as_ptr() as *const c_void, 2));
        assert!(fc4_props_set_typed(p, 3, 0x0A, true, std::ptr::null(), 0));
        assert!(!fc4_props_set_typed(p, 4, 0x0A, false, fs.as_ptr() as *const c_void, 2));
        assert!(!fc4_props_set_typed(p, 5, 0x10, false, fs.as_ptr() as *const c_void, 1));
        assert_eq!(fc4_props_count(p), 3);
        fc4_props_destroy(p);
    }
}

#[test]
fn load_month_end_accounting_and_queries() {
    let g = city();
    let h = g.host.get().unwrap();
    unsafe {
        let s = fc4_ordinance_sim_create(&h, 0x51);
        assert!(fc4_ordinance_sim_init(s));
        // §8.1: the two type-14 exemplars with a class, keyed by ID, unavailable.
        let st = state(s);
        assert_eq!((st.initialized, st.count), (1, 2));
        let mut ids = [0u32; 4];
        let mut n = 4u32;
        assert_eq!(fc4_ordinance_sim_array(s, 2, ids.as_mut_ptr(), &mut n), 2);
        assert_eq!((n, ids[0], ids[1]), (2, CPR, WATCH));
        assert_eq!(fc4_ordinance_sim_count(s, 0), 0);
        let cpr_token = fc4_ordinance_sim_by_id(s, CPR);
        let cpr = object(&g, cpr_token);
        assert!(!cpr.is_null());
        assert_eq!(fc4_ordinance_class_id(cpr), CPR);
        assert_eq!(text(cpr, 0), b"0x00d0723d"); // no LTEXT, no string: the hex name
        assert_eq!(fc4_ordinance_call(cpr, op::IS_ENABLED, 0), 1);
        assert_eq!(fc4_ordinance_call(cpr, op::ENACTMENT_INCOME, 0), -20);
        assert_eq!(
            f32::from_bits(fc4_ordinance_call(cpr, op::MONTHLY_INCOME_FACTOR, 0) as u32),
            -0.005
        );
        let (mut t, mut gr, mut i) = (0, 0, 0);
        assert!(fc4_ordinance_get_key(cpr, &mut t, &mut gr, &mut i));
        assert_eq!((t, gr, i), (EXEMPLAR, GROUP, CPR));
        assert_eq!(fc4_ordinance_sim_by_name(s, b"0X00D0723D".as_ptr(), 10), cpr_token);
        assert_eq!(fc4_ordinance_sim_id_by_name(s, b"nope".as_ptr(), 4), 0);
        let misc = fc4_props_create();
        assert!(fc4_ordinance_misc_property(cpr, FACTOR_PROP, misc));
        assert!(!fc4_ordinance_misc_property(cpr, 0x99, misc));
        fc4_props_destroy(misc);

        // Before the first year: nothing becomes available.
        assert!(fc4_ordinance_sim_handle_message(s, NEW_MONTH));
        assert_eq!(fc4_ordinance_sim_count(s, 0), 0);
        assert_eq!(*g.posts.borrow(), vec![(MONTHLY_DONE, 0, 0)]);
        g.posts.borrow_mut().clear();

        // §5: available at the month-end; NeighborhoodWatch needs a station.
        g.year.set(2000);
        fc4_ordinance_sim_handle_message(s, NEW_MONTH);
        assert!(fc4_ordinance_sim_query(s, 0, CPR));
        assert!(!fc4_ordinance_sim_query(s, 0, WATCH));
        assert!(fc4_ordinance_sim_query(s, 2, CPR));
        assert_eq!(*g.posts.borrow(), vec![(AVAILABILITY, CPR, 1), (MONTHLY_DONE, 0, 0)]);
        g.posts.borrow_mut().clear();

        // §4.4: enacting charges -20 into pending; the month-end books it and
        // the monthly amount -50 + 20,000 × -0.005 = -150 as expenses.
        assert!(fc4_ordinance_sim_set(s, 1, CPR, true));
        assert!(fc4_ordinance_sim_set(s, 1, CPR, true)); // no change: no message
        assert_eq!(*g.posts.borrow(), vec![(UPDATED, CPR, 1)]);
        assert_eq!(state(s).pending, -20);
        fc4_ordinance_sim_handle_message(s, NEW_MONTH);
        assert_eq!(fc4_ordinance_sim_money(s, 0), 170);
        assert_eq!(fc4_ordinance_sim_money(s, 1), 0);
        assert_eq!(fc4_ordinance_sim_count(s, 1), 1);

        // §6.1: the on ordinance's factor and offset.
        assert_eq!(fc4_ordinance_sim_cumulative_factor(s, FACTOR_PROP, 0), 0.95);
        assert_eq!(fc4_ordinance_sim_cumulative_offset(s, OFFSET_PROP, 0), -10);
        assert_eq!(fc4_ordinance_sim_cumulative_factor(s, 0x77, 0), 1.0);

        // A foreign ordinance through the host: added (one reference for the
        // map), made available, enacted, counted in the factor, removed.
        assert!(fc4_ordinance_sim_add(s, FOREIGN, std::ptr::null()));
        assert_eq!(g.foreign_refs.get(), 1);
        assert!(fc4_ordinance_sim_query(s, 0, FOREIGN_ID)); // CheckConditions → available
        assert_eq!(fc4_ordinance_sim_by_id(s, FOREIGN_ID), FOREIGN);
        assert_eq!(
            fc4_ordinance_sim_id_by_name(s, b"plugin ORDINANCE".as_ptr(), 16),
            FOREIGN_ID
        );
        assert!(fc4_ordinance_sim_set(s, 1, FOREIGN_ID, true));
        assert_eq!(state(s).pending, -5);
        assert_eq!(fc4_ordinance_sim_cumulative_factor(s, FACTOR_PROP, 0), 0.95 * 0.5);
        let mut on = [0u32; 1];
        let mut n = 1u32;
        assert_eq!(fc4_ordinance_sim_array(s, 1, on.as_mut_ptr(), &mut n), 2);
        assert_eq!((n, on[0]), (1, CPR));
        assert!(fc4_ordinance_sim_remove(s, FOREIGN, std::ptr::null()));
        assert_eq!(g.foreign_refs.get(), 0);
        assert_eq!(state(s).pending, -6); // the repeal amount
        assert!(!fc4_ordinance_sim_remove(s, FOREIGN, std::ptr::null()));

        // Our own object added again (the caller holding a reference)
        // replaces the stored entry: repealed (-10), re-added, available again.
        add_ref(h.user, cpr_token);
        assert!(fc4_ordinance_sim_add(s, cpr_token, cpr));
        release(h.user, cpr_token);
        assert_eq!(g.objects.borrow()[&cpr_token].1, 1);
        assert_eq!(state(s).count, 2);
        assert_eq!(state(s).pending, -16);
        assert!(fc4_ordinance_sim_query(s, 2, CPR));

        assert!(fc4_ordinance_sim_shutdown(s));
        fc4_ordinance_sim_destroy(s);
        assert!(
            g.objects.borrow().is_empty(),
            "every ordinance released with the simulator"
        );
    }
}

#[test]
fn save_round_trip() {
    let g = city();
    let h = g.host.get().unwrap();
    unsafe {
        let s = fc4_ordinance_sim_create(&h, 0x51);
        assert!(fc4_ordinance_sim_init(s));
        g.year.set(2000);
        fc4_ordinance_sim_handle_message(s, NEW_MONTH);
        fc4_ordinance_sim_set(s, 1, CPR, true);

        let mut out = Out::default();
        assert!(fc4_ordinance_sim_write(s, &ostream(&mut out)));
        assert_eq!(out.refs.len(), 2);
        let cpr_token = fc4_ordinance_sim_by_id(s, CPR);
        assert_eq!(out.refs[0], (cpr_token, CPR));

        // The ordinance record (§9.2): write → read into a new object → write.
        let cpr = object(&g, cpr_token);
        let mut rec = Out::default();
        assert!(fc4_ordinance_write(cpr, &ostream(&mut rec)));
        assert_eq!(&rec.bytes[0..2], &4u16.to_le_bytes());
        let copy = fc4_ordinance_create(&h, CPR, 0x999);
        let mut inp = In::new(&rec.bytes);
        assert!(fc4_ordinance_read(copy, &istream(&mut inp)));
        assert_eq!(fc4_ordinance_call(copy, op::IS_ON, 0), 1);
        assert_eq!(fc4_ordinance_call(copy, op::IS_ENABLED, 0), 1);
        let mut rec2 = Out::default();
        assert!(fc4_ordinance_write(copy, &ostream(&mut rec2)));
        assert_eq!(rec.bytes, rec2.bytes);
        fc4_ordinance_destroy(copy);

        // The simulator record: references to ours resolve to their handles.
        let objs: Vec<(u64, usize)> = g.objects.borrow().iter().map(|(t, e)| (*t, e.0)).collect();
        let resolve = |r: &mut fc4_ref| {
            if let Some((_, p)) = objs.iter().find(|(t, _)| *t == r.token) {
                r.core_handle = *p as *mut c_void;
            }
        };
        let s2 = fc4_ordinance_sim_create(&h, 0x52);
        let mut inp = In::new(&out.bytes);
        inp.resolve = Some(&resolve);
        assert!(fc4_ordinance_sim_read(s2, &istream(&mut inp)));
        let st = state(s2);
        assert_eq!((st.loaded_from_save, st.count, st.pending), (1, 2, -20));
        // Init after a load keeps the loaded set (no reload).
        assert!(fc4_ordinance_sim_init(s2));
        assert!(fc4_ordinance_sim_query(s2, 1, CPR));
        let mut again = Out::default();
        assert!(fc4_ordinance_sim_write(s2, &ostream(&mut again)));
        assert_eq!(out.bytes, again.bytes);

        // Version 3 is rejected.
        let mut bad = out.bytes.clone();
        bad[0] = 3;
        let mut inp = In::new(&bad);
        assert!(!fc4_ordinance_sim_read(s2, &istream(&mut inp)));

        fc4_ordinance_sim_destroy(s2);
        fc4_ordinance_sim_destroy(s);
        assert!(g.objects.borrow().is_empty());
    }
}
