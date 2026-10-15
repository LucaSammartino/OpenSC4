// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The park manager through the C ABI (docs/specs/parks.md): a fake game
//! behind `fc4_park_host`.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::{c_char, c_void, CStr};

use fc4_core::rng::{derive_seed, FixedSeeds, RzRandomSuper};

use crate::budget::fc4_budget_item;
use crate::parks::*;
use crate::props::{fc4_props, fc4_props_create, fc4_props_destroy, fc4_props_set_f32s};
use crate::test_streams::{istream, ostream, In, Out};

const DEPT: u32 = 0x8A42_60A6;
const PURPOSE: u32 = 0xCA63_9989;
const BUILDING: u32 = 0x2781_28A0;
const PREVIEW: u32 = 0x0292_44DB;
const PARK: u32 = 0x1006;
const ZOO: u32 = 0x1702;
const NEW_MONTH: u32 = 0x6695_6816;
const INSERTED: u32 = 0x99EF_1142;
const REMOVED: u32 = 0x99EF_1143;
const MANAGER: u64 = 0xAA;
const SEED: u32 = 0x1234;

#[derive(Default, Clone)]
struct Occ {
    building_type: u32,
    groups: Vec<u32>,
    preview: bool,
    lot: u64,
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
struct Line {
    expenses: i64,
    secondary: i64,
    flag: bool,
}

#[derive(Default)]
struct Game {
    log: RefCell<Vec<String>>,
    refs: RefCell<BTreeMap<u64, i32>>,
    depts: RefCell<BTreeMap<u32, u64>>,
    funding: Cell<f32>,
    lines: RefCell<BTreeMap<(u64, u32), (u64, Line)>>,
    next: Cell<u64>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    items: RefCell<BTreeMap<u64, fc4_budget_item>>,
    conditions: RefCell<BTreeMap<u64, u32>>,
    resolved: RefCell<Vec<(u64, u32)>>,
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
    fn dept(&self) -> u64 {
        self.depts.borrow()[&DEPT]
    }
    /// A building occupant on lot `occ + 1` with a park budget item (line 0,
    /// `cost`) and `groups`.
    fn park(&self, occ: u64, ty: u32, cost: i64, groups: &[u32]) -> u64 {
        let lot = occ + 1;
        self.occs.borrow_mut().insert(
            occ,
            Occ {
                building_type: ty,
                groups: groups.to_vec(),
                preview: false,
                lot,
            },
        );
        self.items.borrow_mut().insert(
            occ,
            fc4_budget_item {
                department_id: DEPT,
                line_item: 0,
                purpose: PURPOSE,
                index: 0,
                cost,
                local_funding: 1.0,
                reserved: 0,
            },
        );
        self.conditions.borrow_mut().insert(lot, 0);
        lot
    }
    fn line(&self, ty: u32) -> Option<Line> {
        self.lines.borrow().get(&(self.dept(), ty)).map(|(_, l)| *l)
    }
    fn line_mut(&self, li: u64, f: impl FnOnce(&mut Line)) {
        for (t, l) in self.lines.borrow_mut().values_mut() {
            if *t == li {
                f(l);
                return;
            }
        }
    }
}

fn game(u: *mut c_void) -> &'static Game {
    unsafe { &*(u as *const Game) }
}

unsafe extern "C" fn add_agent(u: *mut c_void, level: u32, name: *const c_char, flags: u32) {
    let n = CStr::from_ptr(name).to_string_lossy().into_owned();
    game(u).log(format!("agent {level} {n} {flags}"));
}
unsafe extern "C" fn remove_agent(u: *mut c_void) {
    game(u).log("remove agent");
}
unsafe extern "C" fn acquire_message_server(u: *mut c_void) -> bool {
    game(u).log("acquire ms");
    true
}
unsafe extern "C" fn subscribe(u: *mut c_void, m: u32) {
    game(u).log(format!("subscribe {m:08X}"));
}
unsafe extern "C" fn unsubscribe(u: *mut c_void, m: u32) {
    game(u).log(format!("unsubscribe {m:08X}"));
}
unsafe extern "C" fn release_message_server(u: *mut c_void) {
    game(u).log("release ms");
}
unsafe extern "C" fn acquire_city(u: *mut c_void) -> bool {
    game(u).log("acquire city");
    true
}
unsafe extern "C" fn release_city(u: *mut c_void) {
    game(u).log("release city");
}
unsafe extern "C" fn acquire_budget(u: *mut c_void) -> bool {
    game(u).log("acquire budget");
    true
}
unsafe extern "C" fn release_budget(u: *mut c_void) {
    game(u).log("release budget");
}
unsafe extern "C" fn acquire_lot_manager(u: *mut c_void) -> bool {
    game(u).log("acquire lots");
    true
}
unsafe extern "C" fn release_lot_manager(u: *mut c_void) {
    game(u).log("release lots");
}
unsafe extern "C" fn add_ref(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_default() += 1;
    game(u).log(format!("addref {t:X}"));
}
unsafe extern "C" fn release(u: *mut c_void, t: u64) {
    *game(u).refs.borrow_mut().entry(t).or_default() -= 1;
    game(u).log(format!("release {t:X}"));
}
unsafe extern "C" fn resolve_ref(u: *mut c_void, t: u64, iid: u32) -> u64 {
    game(u).resolved.borrow_mut().push((t, iid));
    t
}
unsafe extern "C" fn department_budget(u: *mut c_void, id: u32) -> u64 {
    game(u).depts.borrow().get(&id).copied().unwrap_or(0)
}
unsafe extern "C" fn create_department_budget(u: *mut c_void, id: u32, group: u32) -> u64 {
    let g = game(u);
    let t = g.next.get();
    g.next.set(t + 0x10);
    g.depts.borrow_mut().insert(id, t);
    g.log(format!("create dept {id:08X} {group:08X}"));
    t
}
unsafe extern "C" fn set_department_name(u: *mut c_void, d: u64, group: u32, instance: u32) {
    game(u).log(format!("name {d:X} {group:08X} {instance:08X}"));
}
unsafe extern "C" fn set_max_funding(u: *mut c_void, d: u64, max: f32) {
    game(u).log(format!("max {d:X} {max}"));
}
unsafe extern "C" fn department_funding(u: *mut c_void, _d: u64, out: *mut f32) -> bool {
    *out = game(u).funding.get();
    true
}
unsafe extern "C" fn budget_item(u: *mut c_void, occ: u64, purpose: u32, out: *mut fc4_budget_item) -> bool {
    assert_eq!(purpose, PURPOSE);
    match game(u).items.borrow().get(&occ) {
        Some(i) => {
            *out = *i;
            true
        }
        None => false,
    }
}
unsafe extern "C" fn change_budget_item_line(u: *mut c_void, occ: u64, index: i32, line: u32) {
    let g = game(u);
    g.log(format!("change line {occ:X} {index} {line:08X}"));
    if let Some(i) = g.items.borrow_mut().get_mut(&occ) {
        i.line_item = line;
    }
}
unsafe extern "C" fn line_item_from_budget_item(u: *mut c_void, item: *const fc4_budget_item) -> u64 {
    let g = game(u);
    let i = &*item;
    let Some(&d) = g.depts.borrow().get(&i.department_id) else {
        return 0;
    };
    g.lines.borrow().get(&(d, i.line_item)).map_or(0, |(t, _)| *t)
}
unsafe extern "C" fn line_item(u: *mut c_void, d: u64, line: u32) -> u64 {
    game(u).lines.borrow().get(&(d, line)).map_or(0, |(t, _)| *t)
}
unsafe extern "C" fn create_line_item(u: *mut c_void, d: u64, line: u32, local: bool) -> u64 {
    let g = game(u);
    let t = g.next.get();
    g.next.set(t + 0x10);
    g.lines.borrow_mut().insert((d, line), (t, Line::default()));
    g.log(format!("create line {d:X} {line:08X} {local}"));
    t
}
unsafe extern "C" fn set_line_display_flag(u: *mut c_void, li: u64, flag: u32, on: bool) {
    assert_eq!(flag, 2);
    game(u).line_mut(li, |l| l.flag = on);
}
unsafe extern "C" fn add_to_full_expenses(u: *mut c_void, li: u64, a: i64) {
    game(u).line_mut(li, |l| l.expenses += a);
}
unsafe extern "C" fn line_secondary(u: *mut c_void, li: u64) -> i64 {
    let mut v = 0;
    game(u).line_mut(li, |l| v = l.secondary);
    v
}
unsafe extern "C" fn set_line_secondary(u: *mut c_void, li: u64, v: i64) {
    game(u).line_mut(li, |l| l.secondary = v);
}
unsafe extern "C" fn occupant_type(u: *mut c_void, occ: u64) -> u32 {
    if game(u).occs.borrow().contains_key(&occ) {
        BUILDING
    } else {
        0xC772_BF98
    }
}
unsafe extern "C" fn occupant_has_property(u: *mut c_void, occ: u64, id: u32) -> bool {
    id == PREVIEW && game(u).occs.borrow().get(&occ).is_some_and(|o| o.preview)
}
unsafe extern "C" fn is_occupant_group(u: *mut c_void, occ: u64, group: u32) -> bool {
    game(u)
        .occs
        .borrow()
        .get(&occ)
        .is_some_and(|o| o.groups.contains(&group))
}
unsafe extern "C" fn is_building_occupant(u: *mut c_void, occ: u64) -> bool {
    game(u).occs.borrow().contains_key(&occ)
}
unsafe extern "C" fn building_type(u: *mut c_void, b: u64) -> u32 {
    game(u).occs.borrow().get(&b).map_or(0, |o| o.building_type)
}
unsafe extern "C" fn occupant_position_lot(u: *mut c_void, occ: u64) -> u64 {
    game(u).occs.borrow().get(&occ).map_or(0, |o| o.lot)
}
unsafe extern "C" fn set_lot_condition(u: *mut c_void, lot: u64, c: u32) {
    game(u).conditions.borrow_mut().insert(lot, c);
    game(u).log(format!("condition {lot:X} {c}"));
}
unsafe extern "C" fn post_zoo_escape(u: *mut c_void, manager: u64, zoo: u64) {
    game(u).log(format!("zoo escape {manager:X} {zoo:X}"));
}
unsafe extern "C" fn add_controllers(u: *mut c_void, name: *const c_char, occ: u64) {
    let n = CStr::from_ptr(name).to_string_lossy().into_owned();
    game(u).log(format!("controllers {n} {occ:X}"));
}

fn host(g: &Game) -> fc4_park_host {
    fc4_park_host {
        struct_size: std::mem::size_of::<fc4_park_host>() as u32,
        user: g.user(),
        add_agent: Some(add_agent),
        remove_agent: Some(remove_agent),
        acquire_message_server: Some(acquire_message_server),
        subscribe: Some(subscribe),
        unsubscribe: Some(unsubscribe),
        release_message_server: Some(release_message_server),
        acquire_city: Some(acquire_city),
        release_city: Some(release_city),
        acquire_budget: Some(acquire_budget),
        release_budget: Some(release_budget),
        acquire_lot_manager: Some(acquire_lot_manager),
        release_lot_manager: Some(release_lot_manager),
        add_ref: Some(add_ref),
        release: Some(release),
        resolve_ref: Some(resolve_ref),
        department_budget: Some(department_budget),
        create_department_budget: Some(create_department_budget),
        set_department_name: Some(set_department_name),
        set_max_funding: Some(set_max_funding),
        department_funding: Some(department_funding),
        budget_item: Some(budget_item),
        change_budget_item_line: Some(change_budget_item_line),
        line_item_from_budget_item: Some(line_item_from_budget_item),
        line_item: Some(line_item),
        create_line_item: Some(create_line_item),
        set_line_display_flag: Some(set_line_display_flag),
        add_to_full_expenses: Some(add_to_full_expenses),
        line_secondary: Some(line_secondary),
        set_line_secondary: Some(set_line_secondary),
        occupant_type: Some(occupant_type),
        occupant_has_property: Some(occupant_has_property),
        is_occupant_group: Some(is_occupant_group),
        is_building_occupant: Some(is_building_occupant),
        building_type: Some(building_type),
        occupant_position_lot: Some(occupant_position_lot),
        set_lot_condition: Some(set_lot_condition),
        post_zoo_escape: Some(post_zoo_escape),
        add_controllers: Some(add_controllers),
    }
}

/// The shipped "Parks Manager" exemplar (§6): threshold 25.0 and the curve
/// (0, 40), (25, 10), (100, 1).
fn shipped() -> *mut fc4_props {
    let p = fc4_props_create();
    unsafe {
        assert!(fc4_props_set_f32s(p, 0xCA48_0830, &25.0, 1));
        let c = [0.0f32, 40.0, 25.0, 10.0, 100.0, 1.0];
        assert!(fc4_props_set_f32s(p, 0x4A66_76AD, c.as_ptr(), c.len()));
    }
    p
}

fn create(g: &Game) -> *mut fc4_park {
    let p = unsafe { fc4_park_create(&host(g), SEED, MANAGER) };
    assert!(!p.is_null());
    p
}

/// Initialised with `tuning` (NULL = none), the log cleared.
fn ready(g: &Game, tuning: *const fc4_props) -> *mut fc4_park {
    let p = create(g);
    assert!(unsafe { fc4_park_init(p, tuning) });
    g.take();
    p
}

fn state(p: *const fc4_park) -> fc4_park_state {
    let mut s = fc4_park_state::default();
    assert!(unsafe { fc4_park_get_state(p, &mut s) });
    s
}

fn msg(p: *mut fc4_park, t: u32, void1: u64) {
    assert!(unsafe { fc4_park_do_message(p, t, 0, 0, 0, void1) });
}

fn list(p: *const fc4_park, which: u32) -> Vec<u64> {
    unsafe {
        let n = fc4_park_list(p, which, std::ptr::null_mut(), 0);
        let mut v = vec![0u64; n];
        assert_eq!(fc4_park_list(p, which, v.as_mut_ptr(), n), n);
        v
    }
}

fn write(p: *const fc4_park) -> Out {
    let mut o = Out::default();
    let os = ostream(&mut o);
    assert!(unsafe { fc4_park_write(p, &os) });
    o
}

#[test]
fn create_rejects_bad_tables_and_identity() {
    let g = Game::new();
    unsafe {
        assert!(fc4_park_create(std::ptr::null(), 0, 0).is_null());
        let mut h = host(&g);
        h.struct_size -= 1;
        assert!(fc4_park_create(&h, 0, 0).is_null());
    }
    for iid in [1, 0xAA3F_E6BC, 0x4522_94AA, 0xE4FD_A3D4] {
        assert!(fc4_park_query_interface(iid), "{iid:08X}");
    }
    assert!(!fc4_park_query_interface(0x8A41_322F));
}

#[test]
fn pk12_01_init_and_shutdown_over_the_abi() {
    let g = Game::new();
    unsafe {
        let p = create(&g);
        assert!(fc4_park_init(p, std::ptr::null()));
        let d = g.dept();
        assert_eq!(
            g.take(),
            [
                "acquire city".to_string(),
                "acquire budget".to_string(),
                "create dept 8A4260A6 6A357B96".to_string(),
                format!("name {d:X} 6A3FF01C CA49375E"),
                format!("max {d:X} 1"),
                format!("addref {d:X}"),
                "acquire lots".to_string(),
                "acquire ms".to_string(),
                "subscribe 99EF1142".to_string(),
                "subscribe 99EF1143".to_string(),
                "agent 6 cSC4ParkManager 2".to_string(),
            ]
        );
        let s = state(p);
        assert_eq!((s.initialized, s.tuning_read, s.department), (1, 1, d));
        assert_eq!(
            (s.distress_threshold, s.curve_points),
            (0.25, 0),
            "constructor values: no exemplar, nothing read"
        );
        assert!(fc4_park_init(p, std::ptr::null()));
        assert!(g.take().is_empty(), "a second Init does nothing");

        // Release: 2 → 1, then Shutdown (in §4.1's order).
        g.park(0x60, 1, 5, &[PARK]);
        msg(p, INSERTED, 0x60);
        g.take();
        assert_eq!(fc4_park_add_ref(p), 1);
        assert_eq!(fc4_park_add_ref(p), 2);
        assert_eq!(fc4_park_release(p), 1);
        assert_eq!(fc4_park_release(p), 0);
        assert_eq!(
            g.take(),
            [
                "release 60".to_string(),
                "remove agent".to_string(),
                "unsubscribe 99EF1142".to_string(),
                "unsubscribe 99EF1143".to_string(),
                "release ms".to_string(),
                format!("release {d:X}"),
                "release lots".to_string(),
                "release budget".to_string(),
                "release city".to_string(),
            ]
        );
        let s = state(p);
        assert_eq!((s.initialized, s.destroyed, s.department, s.parks), (0, 1, 0, 0));
        fc4_park_destroy(p);
    }
}

#[test]
fn pk12_02_tuning_over_the_abi() {
    let g = Game::new();
    let t = shipped();
    unsafe {
        let p = ready(&g, t);
        fc4_props_destroy(t); // copied
        let s = state(p);
        assert_eq!((s.distress_threshold, s.curve_points), (25.0, 3));
        // §4.6's table.
        assert_eq!(fc4_park_escape_probability(p, 0.5), f64::from(7.0f32 * 0.01f32));
        assert_eq!(fc4_park_escape_probability(p, 0.0), f64::from(0.399_999_98f32));
        assert_eq!(fc4_park_escape_probability(p, 1.2), f64::from(0.01f32));
        // The tuning is read at the first Init only.
        assert!(fc4_park_shutdown(p));
        assert!(fc4_park_init(p, std::ptr::null()));
        assert_eq!(state(p).distress_threshold, 25.0);
        fc4_park_destroy(p);
    }
}

#[test]
fn pk12_03_pk12_04_insert_and_remove() {
    let g = Game::new();
    let p = ready(&g, std::ptr::null());
    unsafe {
        let ty = 0x0339_0000;
        for occ in [0x60u64, 0x62, 0x64] {
            g.park(occ, ty, 5, &[PARK]);
            msg(p, INSERTED, occ);
        }
        assert_eq!(
            g.line(ty),
            Some(Line {
                expenses: 15,
                secondary: 3,
                flag: true
            })
        );
        assert_eq!(list(p, 0), [0x60, 0x62, 0x64]);
        // A preview occupant is ignored.
        g.park(0x66, ty, 5, &[PARK]);
        g.occs.borrow_mut().get_mut(&0x66).unwrap().preview = true;
        msg(p, INSERTED, 0x66);
        assert_eq!(list(p, 0).len(), 3);
        // A zoo is listed (and a park zoo in both lists).
        g.park(0x70, ty, 5, &[PARK, ZOO]);
        msg(p, INSERTED, 0x70);
        assert_eq!(list(p, 1), [0x70]);
        assert_eq!(g.refs(0x70), 2);
        // Removal: every entry, one release; the line goes down.
        for occ in [0x60u64, 0x62, 0x64, 0x70] {
            msg(p, REMOVED, occ);
        }
        assert!(list(p, 0).is_empty() && list(p, 1).is_empty());
        assert_eq!(g.refs(0x70), 0);
        assert_eq!(
            g.line(ty),
            Some(Line {
                expenses: 0,
                secondary: 0,
                flag: true
            }),
            "the line stays"
        );
        fc4_park_destroy(p);
    }
}

#[test]
fn pk12_05_pk12_06_distress_latch_over_the_abi() {
    // Code default 0.25: ≤ runs down, > restores.
    let g = Game::new();
    let p = ready(&g, std::ptr::null());
    let la = g.park(0x60, 1, 10, &[PARK]);
    msg(p, INSERTED, 0x60);
    g.funding.set(0.25);
    msg(p, NEW_MONTH, 0);
    assert_eq!(g.conditions.borrow()[&la], 2);
    assert_eq!(state(p).underfunded, 1);
    g.funding.set(0.3);
    msg(p, NEW_MONTH, 0);
    assert_eq!(g.conditions.borrow()[&la], 0);
    unsafe { fc4_park_destroy(p) };

    // PK1: the shipped 25.0 against a fraction: never restored.
    let g = Game::new();
    let t = shipped();
    let p = ready(&g, t);
    unsafe { fc4_props_destroy(t) };
    let la = g.park(0x60, 1, 10, &[PARK]);
    msg(p, INSERTED, 0x60);
    for f in [1.2, 0.5, 1.0, 0.0] {
        g.funding.set(f);
        msg(p, NEW_MONTH, 0);
        assert_eq!(g.conditions.borrow()[&la], 2, "funding {f}");
    }
    unsafe { fc4_park_destroy(p) };
}

#[test]
fn pk12_07_escape_draws_follow_the_member_generator() {
    let g = Game::new();
    let t = shipped();
    let p = ready(&g, t);
    unsafe { fc4_props_destroy(t) };
    g.park(0x70, 1, 10, &[PARK]);
    g.park(0x72, 0x0339_0000, 10, &[PARK, ZOO]);
    g.park(0x74, 0x0339_0001, 10, &[ZOO]);
    msg(p, INSERTED, 0x70);
    msg(p, INSERTED, 0x72);
    msg(p, INSERTED, 0x74);
    assert_eq!(list(p, 1), [0x72, 0x74]);
    g.funding.set(0.0);
    let pr = unsafe { fc4_park_escape_probability(p, 0.0) };
    // §5: an RzRandomSuper seeded derive(S, 0xAA3FE6BC); r < p escapes.
    let seed = derive_seed(SEED, 0xAA3F_E6BC);
    let mut rng = RzRandomSuper::new(
        seed,
        &FixedSeeds {
            wall_clock: seed,
            process_clock: seed,
        },
    );
    let mut want = Vec::new();
    for _ in 0..40 {
        if rng.uniform_f64() < pr {
            let zoo = [0x72u64, 0x74][rng.uniform_u32(2) as usize];
            want.push(format!("zoo escape {MANAGER:X} {zoo:X}"));
            want.push(format!("controllers zoomanji {zoo:X}"));
        }
    }
    g.take();
    for _ in 0..40 {
        msg(p, NEW_MONTH, 0);
    }
    let got: Vec<String> = g.take().into_iter().filter(|e| !e.starts_with("condition")).collect();
    assert_eq!(got, want);
    assert!(!want.is_empty(), "p = 0.4 over 40 months");
    unsafe { fc4_park_destroy(p) };
}

#[test]
fn pk12_08_record_round_trip_over_the_abi() {
    let g = Game::new();
    let p = ready(&g, std::ptr::null());
    unsafe {
        g.park(0x60, 1, 10, &[PARK]);
        g.park(0x62, 1, 10, &[PARK, ZOO]);
        msg(p, INSERTED, 0x60);
        msg(p, INSERTED, 0x62);
        let o = write(p);
        assert_eq!(&o.bytes[..2], &[2, 0]);
        assert_eq!(o.refs, [(0x60, 0), (0x62, 0), (0x62, 0)], "parks, then zoos");

        let q = create(&g);
        let mut i = In::new(&o.bytes);
        let is = istream(&mut i);
        assert!(fc4_park_read(q, &is));
        assert_eq!(
            *g.resolved.borrow(),
            [(0x60, 0x26BB_767E), (0x62, 0x26BB_767E), (0x62, 0x26BB_767E)]
        );
        assert_eq!((list(q, 0), list(q, 1)), (vec![0x60, 0x62], vec![0x62]));
        assert_eq!(write(q).bytes, o.bytes, "write → read → write");

        // Version 1 is refused; nothing changes.
        let mut v1 = o.bytes.clone();
        v1[0] = 1;
        let r = create(&g);
        let mut i = In::new(&v1);
        let is = istream(&mut i);
        assert!(!fc4_park_read(r, &is));
        assert_eq!(state(r).parks, 0);
        for x in [p, q, r] {
            fc4_park_destroy(x);
        }
    }
}

#[test]
fn null_handles_are_harmless() {
    unsafe {
        assert!(!fc4_park_init(std::ptr::null_mut(), std::ptr::null()));
        assert_eq!(fc4_park_escape_probability(std::ptr::null(), 0.5), 0.0);
        assert_eq!(fc4_park_list(std::ptr::null(), 0, std::ptr::null_mut(), 0), 0);
        assert!(!fc4_park_get_state(std::ptr::null(), std::ptr::null_mut()));
        fc4_park_destroy(std::ptr::null_mut());
    }
}

/// The save sweep (`save_sweep_tests.rs`): `rec` read into a fresh manager
/// and written again.
pub(super) fn sweep_round_trip(rec: &[u8]) -> Result<Vec<u8>, String> {
    let g = Game::new();
    unsafe {
        let p = fc4_park_create(&host(&g), SEED, MANAGER);
        if p.is_null() {
            return Err("create failed".into());
        }
        let r = crate::save_sweep_tests::read_write(rec, |i| fc4_park_read(p, i), |o| fc4_park_write(p, o));
        fc4_park_destroy(p);
        r
    }
}
