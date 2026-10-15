// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! The civic building simulator through the C ABI
//! (docs/specs/civic-buildings.md): a fake game behind `fc4_civic_host`.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::ffi::{c_char, c_void, CStr};

use fc4_core::civic::{default_random_factory, temporary_seed};

use crate::budget::fc4_budget_item;
use crate::civic::*;
use crate::test_streams::{istream, ostream, In, Out};

const LANDMARKS: u32 = 0x8A41_6368;
const BM: u32 = 0xEA59_717A;
const BD: u32 = 0x2A5A_723F;
const MAINTENANCE: u32 = 0xAA59_670C;
const INCOME: u32 = 0x4A5A_495E;
const BUILDING: u32 = 0x2781_28A0;
const CONDITIONAL: u32 = 0xEA32_09F8;
const EXCLUSION: u32 = 0xEA2E_078B;
const DISEASE: u32 = 0x1917;
const NEW_MONTH: u32 = 0x6695_6816;
const INSERTED: u32 = 0x99EF_1142;
const REMOVED: u32 = 0x99EF_1143;
const LOT_STATE: u32 = 0xE8BD_A2E7;

#[derive(Default, Clone)]
struct Occ {
    building_type: u32,
    key_instance: Option<u32>,
    conditional: Option<bool>,
    exclusion: Option<u32>,
    groups: Vec<u32>,
    lot: u64,
}

#[derive(Default, Clone, Copy)]
struct Line {
    expenses: i64,
    income: i64,
    line_type: u32,
}

#[derive(Default)]
struct Game {
    log: RefCell<Vec<String>>,
    refs: RefCell<BTreeMap<u64, i32>>,
    depts: RefCell<BTreeMap<u32, u64>>,
    funding: RefCell<BTreeMap<u64, f32>>,
    lines: RefCell<BTreeMap<(u64, u32), (u64, Line)>>,
    next: Cell<u64>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    items: RefCell<BTreeMap<(u64, u32), fc4_budget_item>>,
    lots: RefCell<BTreeMap<u64, u64>>,
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
    fn fresh(&self) -> u64 {
        let t = self.next.get();
        self.next.set(t + 0x10);
        t
    }
    fn dept(&self, id: u32) -> u64 {
        self.depts.borrow()[&id]
    }
    /// A building occupant `occ` on lot `occ + 1`.
    fn building(&self, occ: u64, ty: u32) -> u64 {
        let lot = occ + 1;
        self.occs.borrow_mut().insert(
            occ,
            Occ {
                building_type: ty,
                lot,
                ..Occ::default()
            },
        );
        self.lots.borrow_mut().insert(lot, occ);
        self.conditions.borrow_mut().insert(lot, 0);
        lot
    }
    fn occ(&self, occ: u64, f: impl FnOnce(&mut Occ)) {
        f(self.occs.borrow_mut().get_mut(&occ).unwrap())
    }
    fn item(&self, occ: u64, purpose: u32, dept: u32, line: u32, cost: i64) {
        self.items.borrow_mut().insert(
            (occ, purpose),
            fc4_budget_item {
                department_id: dept,
                line_item: line,
                purpose,
                index: 0,
                cost,
                local_funding: 1.0,
                reserved: 0,
            },
        );
    }
    fn line(&self, dept: u32, line: u32) -> Option<Line> {
        let d = *self.depts.borrow().get(&dept)?;
        self.lines.borrow().get(&(d, line)).map(|(_, l)| *l)
    }
    fn line_mut(&self, li: u64, f: impl FnOnce(&mut Line)) {
        if let Some((_, l)) = self.lines.borrow_mut().values_mut().find(|(t, _)| *t == li) {
            f(l);
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
unsafe extern "C" fn subscribe(u: *mut c_void, m: u32) {
    game(u).log(format!("subscribe {m:08X}"));
}
unsafe extern "C" fn unsubscribe(u: *mut c_void, m: u32) {
    game(u).log(format!("unsubscribe {m:08X}"));
}
unsafe extern "C" fn register_lua(u: *mut c_void) {
    game(u).log("lua");
}
unsafe extern "C" fn unregister_lua(u: *mut c_void) {
    game(u).log("unlua");
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
    let t = g.fresh();
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
unsafe extern "C" fn department_funding(u: *mut c_void, d: u64, out: *mut f32) -> bool {
    match game(u).funding.borrow().get(&d) {
        Some(&f) => {
            *out = f;
            true
        }
        None => false,
    }
}
unsafe extern "C" fn budget_item(u: *mut c_void, occ: u64, purpose: u32, out: *mut fc4_budget_item) -> bool {
    match game(u).items.borrow().get(&(occ, purpose)) {
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
    for ((o, _), i) in g.items.borrow_mut().iter_mut() {
        if *o == occ && i.index == index {
            i.line_item = line;
        }
    }
}
unsafe extern "C" fn remove_budget_item(u: *mut c_void, occ: u64, index: i32) {
    let g = game(u);
    g.log(format!("remove item {occ:X} {index}"));
    g.items
        .borrow_mut()
        .retain(|(o, _), i| !(*o == occ && i.index == index));
}
unsafe extern "C" fn line_item(u: *mut c_void, d: u64, line: u32) -> u64 {
    game(u).lines.borrow().get(&(d, line)).map_or(0, |(t, _)| *t)
}
unsafe extern "C" fn create_line_item(u: *mut c_void, d: u64, line: u32, local: bool) -> u64 {
    let g = game(u);
    let t = g.fresh();
    g.lines.borrow_mut().insert((d, line), (t, Line::default()));
    g.log(format!("create line {d:X} {line:08X} {local}"));
    t
}
unsafe extern "C" fn remove_line_item(u: *mut c_void, d: u64, line: u32) {
    let g = game(u);
    g.lines.borrow_mut().remove(&(d, line));
    g.log(format!("remove line {d:X} {line:08X}"));
}
unsafe extern "C" fn add_to_full_expenses(u: *mut c_void, li: u64, a: i64) {
    game(u).line_mut(li, |l| l.expenses += a);
}
unsafe extern "C" fn add_to_income(u: *mut c_void, li: u64, a: i64) {
    game(u).line_mut(li, |l| l.income += a);
}
unsafe extern "C" fn line_income(u: *mut c_void, li: u64) -> i64 {
    let mut v = 0;
    game(u).line_mut(li, |l| v = l.income);
    v
}
unsafe extern "C" fn set_line_income(u: *mut c_void, li: u64, v: i64) {
    game(u).line_mut(li, |l| l.income = v.max(0));
}
unsafe extern "C" fn set_line_type(u: *mut c_void, li: u64, t: u32) {
    game(u).line_mut(li, |l| l.line_type = t);
}
unsafe extern "C" fn occupant_type(u: *mut c_void, occ: u64) -> u32 {
    if game(u).occs.borrow().contains_key(&occ) {
        BUILDING
    } else {
        0xC772_BF98
    }
}
unsafe extern "C" fn occupant_bool_property(u: *mut c_void, occ: u64, id: u32, out: *mut bool) -> bool {
    let o = game(u).occs.borrow().get(&occ).cloned().unwrap_or_default();
    match (id, o.conditional) {
        (CONDITIONAL, Some(v)) => {
            *out = v;
            true
        }
        _ => false,
    }
}
unsafe extern "C" fn occupant_u32_property(u: *mut c_void, occ: u64, id: u32, out: *mut u32) -> bool {
    let o = game(u).occs.borrow().get(&occ).cloned().unwrap_or_default();
    match (id, o.exclusion) {
        (EXCLUSION, Some(v)) => {
            *out = v;
            true
        }
        _ => false,
    }
}
unsafe extern "C" fn is_building_occupant(u: *mut c_void, occ: u64) -> bool {
    game(u).occs.borrow().contains_key(&occ)
}
unsafe extern "C" fn building_type(u: *mut c_void, b: u64) -> u32 {
    game(u).occs.borrow().get(&b).map_or(0, |o| o.building_type)
}
unsafe extern "C" fn building_key_instance(u: *mut c_void, b: u64, out: *mut u32) -> bool {
    match game(u).occs.borrow().get(&b).and_then(|o| o.key_instance) {
        Some(k) => {
            *out = k;
            true
        }
        None => false,
    }
}
unsafe extern "C" fn is_occupant_group(u: *mut c_void, occ: u64, group: u32) -> bool {
    game(u)
        .occs
        .borrow()
        .get(&occ)
        .is_some_and(|o| o.groups.contains(&group))
}
unsafe extern "C" fn message_lot(u: *mut c_void, unknown: u64) -> u64 {
    if game(u).lots.borrow().contains_key(&unknown) {
        add_ref(u, unknown);
        unknown
    } else {
        0
    }
}
unsafe extern "C" fn lot_building(u: *mut c_void, lot: u64) -> u64 {
    game(u).lots.borrow().get(&lot).copied().unwrap_or(0)
}
unsafe extern "C" fn occupant_lot(u: *mut c_void, occ: u64) -> u64 {
    game(u).occs.borrow().get(&occ).map_or(0, |o| o.lot)
}
unsafe extern "C" fn lot_condition(u: *mut c_void, lot: u64) -> u32 {
    game(u).conditions.borrow().get(&lot).copied().unwrap_or(0)
}
unsafe extern "C" fn set_lot_condition(u: *mut c_void, lot: u64, c: u32) {
    game(u).conditions.borrow_mut().insert(lot, c);
}
unsafe extern "C" fn add_controllers(u: *mut c_void, name: *const c_char, occ: u64) {
    let n = CStr::from_ptr(name).to_string_lossy().into_owned();
    game(u).log(format!("controllers {n} {occ:X}"));
}

fn host(g: &Game) -> fc4_civic_host {
    fc4_civic_host {
        struct_size: std::mem::size_of::<fc4_civic_host>() as u32,
        user: g.user(),
        add_agent: Some(add_agent),
        remove_agent: Some(remove_agent),
        subscribe: Some(subscribe),
        unsubscribe: Some(unsubscribe),
        register_lua: Some(register_lua),
        unregister_lua: Some(unregister_lua),
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
        remove_budget_item: Some(remove_budget_item),
        line_item: Some(line_item),
        create_line_item: Some(create_line_item),
        remove_line_item: Some(remove_line_item),
        add_to_full_expenses: Some(add_to_full_expenses),
        add_to_income: Some(add_to_income),
        line_income: Some(line_income),
        set_line_income: Some(set_line_income),
        set_line_type: Some(set_line_type),
        occupant_type: Some(occupant_type),
        occupant_bool_property: Some(occupant_bool_property),
        occupant_u32_property: Some(occupant_u32_property),
        is_building_occupant: Some(is_building_occupant),
        building_type: Some(building_type),
        building_key_instance: Some(building_key_instance),
        is_occupant_group: Some(is_occupant_group),
        message_lot: Some(message_lot),
        lot_building: Some(lot_building),
        occupant_lot: Some(occupant_lot),
        lot_condition: Some(lot_condition),
        set_lot_condition: Some(set_lot_condition),
        add_controllers: Some(add_controllers),
    }
}

const SEED: u32 = 0x1234_5678;

fn ready(g: &Game) -> *mut fc4_civic {
    let c = unsafe { fc4_civic_create(&host(g), SEED) };
    assert!(!c.is_null());
    assert!(unsafe { fc4_civic_init(c) });
    g.take();
    c
}

fn state(c: *const fc4_civic) -> fc4_civic_state {
    let mut s = fc4_civic_state::default();
    assert!(unsafe { fc4_civic_get_state(c, &mut s) });
    s
}

fn msg(c: *mut fc4_civic, t: u32, data2: u32, void1: u64, unknown: u64) {
    assert!(unsafe { fc4_civic_do_message(c, t, 0, data2, 0, void1, unknown) });
}

fn list(c: *const fc4_civic, which: u32) -> Vec<u64> {
    unsafe {
        let n = fc4_civic_list(c, which, std::ptr::null_mut(), 0);
        let mut v = vec![0u64; n];
        assert_eq!(fc4_civic_list(c, which, v.as_mut_ptr(), n), n);
        v
    }
}

fn status(c: *const fc4_civic, key: u32) -> Option<(u8, u16, Vec<u8>)> {
    unsafe {
        let (mut st, mut n, mut len) = (0u8, 0u16, 0usize);
        if !fc4_civic_get_status(c, key, &mut st, &mut n, std::ptr::null_mut(), 0, &mut len) {
            return None;
        }
        let mut buf = vec![0 as c_char; len + 1];
        assert!(fc4_civic_get_status(
            c,
            key,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            buf.as_mut_ptr(),
            buf.len(),
            std::ptr::null_mut()
        ));
        let text = CStr::from_ptr(buf.as_ptr()).to_bytes().to_vec();
        Some((st, n, text))
    }
}

fn write(c: *const fc4_civic) -> Out {
    let mut o = Out::default();
    let os = ostream(&mut o);
    assert!(unsafe { fc4_civic_write(c, &os) });
    o
}

#[test]
fn create_rejects_bad_tables_and_identity() {
    let g = Game::new();
    unsafe {
        assert!(fc4_civic_create(std::ptr::null(), 0).is_null());
        let mut h = host(&g);
        h.struct_size -= 1;
        assert!(fc4_civic_create(&h, 0).is_null());
    }
    for iid in [1, 0x8A41_322F, 0x4522_94AA, 0xE4FD_A3D4] {
        assert!(fc4_civic_query_interface(iid));
    }
    assert!(!fc4_civic_query_interface(0xAA3F_E6BC));
}

#[test]
fn cb12_01_init_and_shutdown_over_the_abi() {
    let g = Game::new();
    unsafe {
        let c = fc4_civic_create(&host(&g), SEED);
        assert!(fc4_civic_init(c));
        let lm = g.dept(LANDMARKS);
        let bm = g.dept(BM);
        let bd = g.dept(BD);
        assert_eq!(
            g.take(),
            [
                "create dept 8A416368 6A357B96".to_string(),
                format!("addref {lm:X}"),
                format!("name {lm:X} 6A3FF01C CA491DD2"),
                format!("max {lm:X} 1"),
                "create dept EA59717A EA597195".to_string(),
                format!("name {bm:X} 6A3FF01C 4A59726D"),
                format!("max {bm:X} 1"),
                "create dept 2A5A723F 0A5A72D1".to_string(),
                format!("name {bd:X} 6A3FF01C 8A5A7330"),
                format!("max {bd:X} 1"),
                "subscribe E8BDA2E7".to_string(),
                "subscribe 99EF1142".to_string(),
                "subscribe 99EF1143".to_string(),
                "agent 6 cSC4CivicBuildingSimulator 2".to_string(),
                "lua".to_string(),
            ]
        );
        let s = state(c);
        assert_eq!((s.initialized, s.landmarks), (1, lm));
        assert_eq!((g.refs(lm), g.refs(bm), g.refs(bd)), (1, 0, 0));
        assert!(fc4_civic_init(c));
        assert!(g.take().is_empty(), "a second Init does nothing");

        // Release: 2 → 1, then Shutdown.
        assert_eq!(fc4_civic_add_ref(c), 1);
        assert_eq!(fc4_civic_add_ref(c), 2);
        assert_eq!(fc4_civic_release(c), 1);
        assert_eq!(fc4_civic_release(c), 0);
        let ev = g.take();
        assert_eq!(ev.first().map(String::as_str), Some("unlua"));
        assert!(ev.contains(&"remove agent".to_string()));
        assert!(ev.contains(&"unsubscribe 99EF1143".to_string()));
        assert_eq!(g.refs(lm), 0);
        let s = state(c);
        assert_eq!((s.initialized, s.destroyed, s.landmarks), (0, 1, 0));
        fc4_civic_destroy(c);
    }
}

#[test]
fn cb12_02_cb12_03_occupant_messages() {
    let g = Game::new();
    let c = ready(&g);
    unsafe {
        let key = 0x032B_0000;
        assert!(fc4_civic_update_status(c, key, 0, 0, std::ptr::null(), 0));
        g.building(0x20, 0x0300_0001);
        g.occ(0x20, |o| {
            o.key_instance = Some(key);
            o.conditional = Some(true);
            o.exclusion = Some(0x8A9A_E664);
        });
        msg(c, INSERTED, 0, 0x20, 0);
        assert!(fc4_civic_is_building_excluded(c, 0x8A9A_E664));
        assert_eq!(status(c, key).unwrap().1, 1);
        let mut ids = [0u32; 4];
        assert_eq!(fc4_civic_exclusions(c, ids.as_mut_ptr(), 4), 1);
        assert_eq!(ids[0], 0x8A9A_E664);
        msg(c, REMOVED, 0, 0x20, 0);
        assert!(!fc4_civic_is_building_excluded(c, 0x8A9A_E664));
        assert_eq!(status(c, key).unwrap().1, 0);
        // 16-bit wrap.
        msg(c, REMOVED, 0, 0x20, 0);
        assert_eq!(status(c, key).unwrap().1, 0xFFFF);
        // Not a building occupant type: nothing.
        msg(c, INSERTED, 0, 0x999, 0);
        assert_eq!(status(c, key).unwrap().1, 0xFFFF);
        fc4_civic_destroy(c);
    }
}

#[test]
fn cb12_08_status_api_over_the_abi() {
    let g = Game::new();
    let c = ready(&g);
    unsafe {
        let k = 0x0400_0000;
        assert!(status(c, k).is_none());
        let t = b"Tooltip";
        assert!(fc4_civic_update_status(
            c,
            k,
            6,
            2,
            t.as_ptr() as *const c_char,
            t.len()
        ));
        assert_eq!(status(c, k), Some((6, 2, t.to_vec())));
        // Truncation: capacity 4 keeps 3 bytes and the NUL; the length is full.
        let mut buf = [0x7F as c_char; 4];
        let mut len = 0usize;
        assert!(fc4_civic_get_status(
            c,
            k,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            buf.as_mut_ptr(),
            4,
            &mut len
        ));
        assert_eq!(len, 7);
        assert_eq!(CStr::from_ptr(buf.as_ptr()).to_bytes(), b"Too");
        // Write-back (the shim's mirror): all three fields are replaced.
        assert!(fc4_civic_update_status(c, k, 7, 2, std::ptr::null(), 0));
        assert_eq!(status(c, k), Some((7, 2, Vec::new())));
        assert!(fc4_civic_update_status(c, k + 1, 0, 0, std::ptr::null(), 0));
        let mut keys = [0u32; 1];
        assert_eq!(
            fc4_civic_status_keys(c, keys.as_mut_ptr(), 1),
            2,
            "count, copied at most 1"
        );
        assert!(keys[0] == k || keys[0] == k + 1);
        assert_eq!(state(c).statuses, 2);
        fc4_civic_destroy(c);
    }
}

#[test]
fn cb12_04_cb12_05_cb12_06_activation_income_and_monthly() {
    let g = Game::new();
    let c = ready(&g);
    unsafe {
        let ty = 0x031F_0000;
        let lot = g.building(0x40, ty);
        g.item(0x40, MAINTENANCE, BM, 0, 30);
        // data2 = 2: the lot is queried and released, nothing else.
        msg(c, LOT_STATE, 2, 0, lot);
        assert_eq!(g.take(), [format!("addref {lot:X}"), format!("release {lot:X}")]);
        msg(c, LOT_STATE, 3, 0, lot);
        let bm = g.dept(BM);
        assert_eq!(
            g.take(),
            [
                format!("addref {lot:X}"),
                format!("change line 40 0 {ty:08X}"),
                format!("create line {bm:X} {ty:08X} false"),
                "addref 40".to_string(),
                format!("release {lot:X}"),
            ]
        );
        assert_eq!(g.line(BM, ty).unwrap().expenses, 30);
        assert_eq!(list(c, 1), [0x40]);
        assert_eq!(fc4_civic_landmark_count(c), 1);
        assert_eq!(g.refs(0x40), 1);

        // Income, keyed by the building type.
        let ty2 = 0x033A_0000;
        let lot2 = g.building(0x50, ty2);
        g.item(0x50, INCOME, BD, 0x99, 300);
        msg(c, LOT_STATE, 3, 0, lot2);
        let l = g.line(BD, ty2).unwrap();
        assert_eq!((l.income, l.line_type), (300, 1));
        assert_eq!(fc4_civic_landmark_count(c), 1, "income only: not maintained");
        msg(c, REMOVED, 0, 0x50, 0);
        assert!(g.line(BD, ty2).is_none(), "income below 1: the line is removed");

        // Monthly: below 0.5 runs the lot down; at 0.5 restores it.
        g.funding.borrow_mut().insert(bm, 0.49);
        msg(c, NEW_MONTH, 0, 0, 0);
        assert_eq!(g.conditions.borrow()[&lot], 2);
        g.funding.borrow_mut().insert(bm, 0.5);
        msg(c, NEW_MONTH, 0, 0, 0);
        assert_eq!(g.conditions.borrow()[&lot], 0);
        assert_eq!(state(c).temporaries, 0, "no Disease Research Center: no draw");

        // Removal.
        msg(c, REMOVED, 0, 0x40, 0);
        assert!(list(c, 1).is_empty());
        assert_eq!(g.refs(0x40), 0);
        fc4_civic_destroy(c);
    }
}

#[test]
fn cb12_07_ape_escape_over_the_abi() {
    let g = Game::new();
    let c = ready(&g);
    unsafe {
        for occ in [0x70u64, 0x80] {
            let lot = g.building(occ, 0x033F_0000);
            g.occ(occ, |o| o.groups.push(DISEASE));
            msg(c, LOT_STATE, 3, 0, lot);
            msg(c, LOT_STATE, 3, 0, lot);
        }
        assert_eq!(list(c, 0), [0x70, 0x80], "no duplicates");
        g.take();
        // The default generator: one temporary per month, seeded
        // derive(S, 0x0A413481 ^ n); an escape when the draw is ≤ 0.01f.
        let mut want = Vec::new();
        for n in 0..300u32 {
            let mut r = (default_random_factory())(temporary_seed(SEED, n));
            if r.uniform_f64() <= f64::from(0.01f32) {
                let i = r.uniform_u32(2) as usize;
                want.push(format!("controllers ape_escape {:X}", [0x70, 0x80][i]));
            }
        }
        for _ in 0..300 {
            msg(c, NEW_MONTH, 0, 0, 0);
        }
        assert_eq!(g.take(), want);
        assert_eq!(state(c).temporaries, 300);
        fc4_civic_destroy(c);
    }
}

#[test]
fn cb12_09_record_round_trip_over_the_abi() {
    let g = Game::new();
    let c = ready(&g);
    unsafe {
        let lm = g.dept(LANDMARKS);
        let lot = g.building(0x32, 0x0500_0000);
        g.item(0x32, MAINTENANCE, BM, 1, 10);
        g.occ(0x32, |o| o.groups.push(DISEASE));
        msg(c, LOT_STATE, 3, 0, lot);
        for (k, st) in [(1u32, 0u8), (194, 1), (2, 6)] {
            let t = format!("t{k}");
            assert!(fc4_civic_update_status(
                c,
                k,
                st,
                k as u16,
                t.as_ptr() as *const c_char,
                t.len()
            ));
        }
        g.occ(0x32, |o| o.exclusion = Some(5));
        msg(c, INSERTED, 0, 0x32, 0);
        let o = write(c);
        assert_eq!(&o.bytes[..2], &[6, 0]);
        assert_eq!(
            o.refs,
            [(lm, 0), (0x32, 0), (0x32, 0)],
            "department, disease center, maintained"
        );

        // Read into a fresh object: tokens come back through resolve_ref.
        let d = fc4_civic_create(&host(&g), SEED);
        let mut i = In::new(&o.bytes);
        let is = istream(&mut i);
        assert!(fc4_civic_read(d, &is));
        assert_eq!(
            *g.resolved.borrow(),
            [(lm, 0x223E_2B67), (0x32, 0x26BB_767E), (0x32, 0x26BB_767E)]
        );
        let s = state(d);
        assert_eq!((s.landmarks, s.exclusions, s.statuses), (lm, 1, 3));
        assert_eq!((list(d, 0), list(d, 1)), (vec![0x32], vec![0x32]));
        let mut keys = [0u32; 3];
        assert_eq!(fc4_civic_status_keys(d, keys.as_mut_ptr(), 3), 3);
        assert_eq!(
            keys,
            [1, 194, 2],
            "CB6: file order, each at the head of bucket 1's chain"
        );
        assert_eq!(status(d, 194), Some((1, 194, b"t194".to_vec())));
        let once = write(d).bytes;
        assert_eq!(once.len(), o.bytes.len());
        assert_ne!(once, o.bytes, "write → read → write reverses the chain");
        let d2 = fc4_civic_create(&host(&g), SEED);
        let mut i = In::new(&once);
        let is = istream(&mut i);
        assert!(fc4_civic_read(d2, &is));
        assert_eq!(write(d2).bytes, o.bytes, "a second round trip restores the record");
        fc4_civic_destroy(d2);

        // Version 5 is refused and changes nothing.
        let mut v5 = o.bytes.clone();
        v5[0] = 5;
        let e = fc4_civic_create(&host(&g), SEED);
        let mut i = In::new(&v5);
        let is = istream(&mut i);
        assert!(!fc4_civic_read(e, &is));
        assert_eq!(state(e).statuses, 0);
        for p in [c, d, e] {
            fc4_civic_destroy(p);
        }
    }
}

#[test]
fn null_handles_are_harmless() {
    unsafe {
        assert!(!fc4_civic_init(std::ptr::null_mut()));
        assert!(!fc4_civic_is_building_excluded(std::ptr::null(), 1));
        assert_eq!(fc4_civic_landmark_count(std::ptr::null()), 0);
        assert_eq!(fc4_civic_list(std::ptr::null(), 0, std::ptr::null_mut(), 0), 0);
        assert!(!fc4_civic_get_status(
            std::ptr::null(),
            1,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut()
        ));
        fc4_civic_destroy(std::ptr::null_mut());
    }
}

/// The save sweep (`save_sweep_tests.rs`): `rec` read into a fresh simulator
/// and written again.
pub(super) fn sweep_round_trip(rec: &[u8]) -> Result<Vec<u8>, String> {
    let g = Game::new();
    unsafe {
        let c = fc4_civic_create(&host(&g), SEED);
        if c.is_null() {
            return Err("create failed".into());
        }
        let r = crate::save_sweep_tests::read_write(rec, |i| fc4_civic_read(c, i), |o| fc4_civic_write(c, o));
        fc4_civic_destroy(c);
        r
    }
}
