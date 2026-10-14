// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests of the civic building simulator against a fake host: the §12
//! tests of `docs/specs/civic-buildings.md` (named `cb12_01_…` to
//! `cb12_10_…`), the §4.8 worked example, the lifecycle and the record.
//!
//! The fake host models a small budget simulator (departments, budget items
//! per occupant and purpose, line items with full expenses, income, type),
//! occupants (type, properties, groups, building type and key), lots and
//! their conditions, reference counts, and an event log.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

use super::*;
use crate::stream::{ByteReader, ByteWriter, RefResolver, RefValue, SaveObject, SaveRegistry};

const HOST_CLASS_OCC: u32 = OCCUPANT_CLASS;
const HOST_CLASS_DEPT: u32 = DEPARTMENT_CLASS;

/// One call the simulator made on the host (only the ones the tests check).
#[derive(Clone, Debug, PartialEq)]
enum Ev {
    AddAgent(u32, String, u32),
    RemoveAgent,
    Subscribe(u32),
    Unsubscribe(u32),
    RegisterLua,
    UnregisterLua,
    AddRef(u64),
    Release(u64),
    CreateDepartment(u32, u32),
    DepartmentName(u64, u32, u32),
    MaxFunding(u64, f32),
    ChangeLine(u64, i32, u32),
    RemoveItem(u64, i32),
    CreateLine(u64, u32, bool),
    RemoveLine(u64, u32),
    SetLineType(u64, u32),
    SetCondition(u64, u32),
    Controllers(String, u64),
}

#[derive(Clone, Debug, Default)]
struct Line {
    full_expenses: i64,
    income: i64,
    line_type: u32,
}

#[derive(Clone, Debug, Default)]
struct Occ {
    occ_type: u32,
    building: bool,
    building_type: u32,
    key_instance: Option<u32>,
    bools: BTreeMap<u32, bool>,
    u32s: BTreeMap<u32, u32>,
    groups: Vec<u32>,
    lot: Option<u64>,
    /// purpose → item
    items: BTreeMap<u32, BudgetItem>,
}

struct Host {
    events: RefCell<Vec<Ev>>,
    /// department ID → token
    departments: RefCell<BTreeMap<u32, u64>>,
    funding: RefCell<BTreeMap<u64, f32>>,
    /// (department token, line) → line token
    lines: RefCell<BTreeMap<(u64, u32), u64>>,
    line_data: RefCell<BTreeMap<u64, Line>>,
    occupants: RefCell<BTreeMap<u64, Occ>>,
    /// lot → building occupant
    lots: RefCell<BTreeMap<u64, Option<u64>>>,
    conditions: RefCell<BTreeMap<u64, u32>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    automata: Cell<bool>,
    next: Cell<u64>,
    /// Classes of the references resolved on load.
    classes: RefCell<BTreeMap<u64, u32>>,
}

impl Host {
    fn new() -> Rc<Host> {
        Rc::new(Host {
            events: RefCell::new(Vec::new()),
            departments: RefCell::new(BTreeMap::new()),
            funding: RefCell::new(BTreeMap::new()),
            lines: RefCell::new(BTreeMap::new()),
            line_data: RefCell::new(BTreeMap::new()),
            occupants: RefCell::new(BTreeMap::new()),
            lots: RefCell::new(BTreeMap::new()),
            conditions: RefCell::new(BTreeMap::new()),
            refs: RefCell::new(BTreeMap::new()),
            automata: Cell::new(true),
            next: Cell::new(0x1000),
            classes: RefCell::new(BTreeMap::new()),
        })
    }

    fn ev(&self, e: Ev) {
        self.events.borrow_mut().push(e);
    }

    fn take(&self) -> Vec<Ev> {
        std::mem::take(&mut *self.events.borrow_mut())
    }

    fn fresh(&self) -> u64 {
        let t = self.next.get();
        self.next.set(t + 1);
        t
    }

    fn add_department(&self, id: u32) -> u64 {
        let t = self.fresh();
        self.departments.borrow_mut().insert(id, t);
        self.funding.borrow_mut().insert(t, 1.0);
        t
    }

    fn dept(&self, id: u32) -> u64 {
        self.departments.borrow()[&id]
    }

    fn line_of(&self, dept_id: u32, line: u32) -> Option<Line> {
        let d = *self.departments.borrow().get(&dept_id)?;
        let t = *self.lines.borrow().get(&(d, line))?;
        self.line_data.borrow().get(&t).cloned()
    }

    fn set_occ(&self, token: u64, o: Occ) {
        self.occupants.borrow_mut().insert(token, o);
    }

    fn occ<R>(&self, token: u64, f: impl FnOnce(&mut Occ) -> R) -> R {
        f(self.occupants.borrow_mut().get_mut(&token).expect("occupant"))
    }

    /// A building occupant with a lot `token + 0x10000` (condition 0).
    fn building(&self, token: u64, building_type: u32) -> u64 {
        let lot = token + 0x1_0000;
        self.set_occ(
            token,
            Occ {
                occ_type: OCCUPANT_TYPE_BUILDING,
                building: true,
                building_type,
                lot: Some(lot),
                ..Occ::default()
            },
        );
        self.lots.borrow_mut().insert(lot, Some(token));
        self.conditions.borrow_mut().insert(lot, 0);
        lot
    }

    fn item(&self, occ: u64, purpose: u32, dept: u32, line: u32, cost: i64) {
        self.occ(occ, |o| {
            o.items.insert(
                purpose,
                BudgetItem {
                    department_id: dept,
                    line_item: line,
                    purpose,
                    cost,
                    index: 0,
                    local_funding: 1.0,
                },
            )
        });
    }

    fn refs(&self, t: u64) -> i64 {
        self.refs.borrow().get(&t).copied().unwrap_or(0)
    }
}

impl CivicHost for Host {
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        self.ev(Ev::AddAgent(level, name.into(), flags));
    }
    fn remove_agent(&self) {
        self.ev(Ev::RemoveAgent);
    }
    fn subscribe(&self, msg_type: u32) {
        self.ev(Ev::Subscribe(msg_type));
    }
    fn unsubscribe(&self, msg_type: u32) {
        self.ev(Ev::Unsubscribe(msg_type));
    }
    fn register_lua(&self) {
        self.ev(Ev::RegisterLua);
    }
    fn unregister_lua(&self) {
        self.ev(Ev::UnregisterLua);
    }
    fn add_ref(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_default() += 1;
        self.ev(Ev::AddRef(token));
    }
    fn release(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_default() -= 1;
        self.ev(Ev::Release(token));
    }
    fn department_budget(&self, id: u32) -> Option<u64> {
        self.departments.borrow().get(&id).copied()
    }
    fn create_department_budget(&self, id: u32, group: u32) -> Option<u64> {
        self.ev(Ev::CreateDepartment(id, group));
        Some(self.add_department(id))
    }
    fn set_department_name(&self, dept: u64, group: u32, instance: u32) {
        self.ev(Ev::DepartmentName(dept, group, instance));
    }
    fn set_max_funding(&self, dept: u64, max: f32) {
        self.ev(Ev::MaxFunding(dept, max));
    }
    fn department_funding(&self, dept: u64) -> Option<f32> {
        self.funding.borrow().get(&dept).copied()
    }
    fn budget_item(&self, occupant: u64, purpose: u32) -> Option<BudgetItem> {
        self.occupants.borrow().get(&occupant)?.items.get(&purpose).copied()
    }
    fn change_budget_item_line(&self, occupant: u64, index: i32, line: u32) {
        self.ev(Ev::ChangeLine(occupant, index, line));
        self.occ(occupant, |o| {
            for it in o.items.values_mut() {
                if it.index == index {
                    it.line_item = line;
                }
            }
        });
    }
    /// budget.md §6: subtracts the cost from the item's line.
    fn remove_budget_item(&self, occupant: u64, index: i32) {
        self.ev(Ev::RemoveItem(occupant, index));
        let item = self.occ(occupant, |o| o.items.values().find(|i| i.index == index).copied());
        if let Some(it) = item {
            if let Some(d) = self.department_budget(it.department_id) {
                if let Some(l) = self.line_item(d, it.line_item) {
                    self.line_data.borrow_mut().get_mut(&l).unwrap().full_expenses -= it.cost;
                }
            }
        }
    }
    fn line_item(&self, dept: u64, line: u32) -> Option<u64> {
        self.lines.borrow().get(&(dept, line)).copied()
    }
    fn create_line_item(&self, dept: u64, line: u32, local: bool) -> Option<u64> {
        self.ev(Ev::CreateLine(dept, line, local));
        let t = self.fresh();
        self.lines.borrow_mut().insert((dept, line), t);
        self.line_data.borrow_mut().insert(t, Line::default());
        Some(t)
    }
    fn remove_line_item(&self, dept: u64, line: u32) {
        self.ev(Ev::RemoveLine(dept, line));
        if let Some(t) = self.lines.borrow_mut().remove(&(dept, line)) {
            self.line_data.borrow_mut().remove(&t);
        }
    }
    fn add_to_full_expenses(&self, line: u64, amount: i64) {
        self.line_data.borrow_mut().get_mut(&line).unwrap().full_expenses += amount;
    }
    fn add_to_income(&self, line: u64, amount: i64) {
        self.line_data.borrow_mut().get_mut(&line).unwrap().income += amount;
    }
    fn line_income(&self, line: u64) -> i64 {
        self.line_data.borrow()[&line].income
    }
    fn set_line_income(&self, line: u64, income: i64) {
        self.line_data.borrow_mut().get_mut(&line).unwrap().income = income.max(0);
    }
    fn set_line_type(&self, line: u64, line_type: u32) {
        self.ev(Ev::SetLineType(line, line_type));
        self.line_data.borrow_mut().get_mut(&line).unwrap().line_type = line_type;
    }
    fn occupant_type(&self, occupant: u64) -> u32 {
        self.occupants.borrow().get(&occupant).map_or(0, |o| o.occ_type)
    }
    fn occupant_bool_property(&self, occupant: u64, id: u32) -> Option<bool> {
        self.occupants.borrow().get(&occupant)?.bools.get(&id).copied()
    }
    fn occupant_u32_property(&self, occupant: u64, id: u32) -> Option<u32> {
        self.occupants.borrow().get(&occupant)?.u32s.get(&id).copied()
    }
    fn is_building_occupant(&self, occupant: u64) -> bool {
        self.occupants.borrow().get(&occupant).is_some_and(|o| o.building)
    }
    fn building_type(&self, building: u64) -> u32 {
        self.occupants.borrow()[&building].building_type
    }
    fn building_key_instance(&self, building: u64) -> Option<u32> {
        self.occupants.borrow().get(&building)?.key_instance
    }
    fn is_occupant_group(&self, occupant: u64, group: u32) -> bool {
        self.occupants
            .borrow()
            .get(&occupant)
            .is_some_and(|o| o.groups.contains(&group))
    }
    fn message_lot(&self, unknown: u64) -> Option<u64> {
        let ok = self.lots.borrow().contains_key(&unknown);
        if ok {
            self.add_ref(unknown);
        }
        ok.then_some(unknown)
    }
    fn lot_building(&self, lot: u64) -> Option<u64> {
        self.lots.borrow().get(&lot).copied().flatten()
    }
    fn occupant_lot(&self, occupant: u64) -> Option<u64> {
        self.occupants.borrow().get(&occupant)?.lot
    }
    fn lot_condition(&self, lot: u64) -> u32 {
        self.conditions.borrow()[&lot]
    }
    fn set_lot_condition(&self, lot: u64, condition: u32) {
        self.ev(Ev::SetCondition(lot, condition));
        self.conditions.borrow_mut().insert(lot, condition);
    }
    fn add_controllers(&self, name: &str, occupant: u64) {
        if self.automata.get() {
            self.ev(Ev::Controllers(name.into(), occupant));
        }
    }
    fn object_ref(&self, token: u64) -> RefValue {
        let class_id = if let Some(&c) = self.classes.borrow().get(&token) {
            c
        } else if self.departments.borrow().values().any(|&d| d == token) {
            HOST_CLASS_DEPT
        } else {
            HOST_CLASS_OCC
        };
        RefValue::Host { token, class_id }
    }
    fn resolve_ref(&self, r: &RefValue, _iid: u32) -> Option<u64> {
        match r {
            RefValue::Host { token, class_id } if *token != 0xDEAD => {
                self.classes.borrow_mut().insert(*token, *class_id);
                Some(*token)
            }
            _ => None,
        }
    }
}

/// A scripted generator: queued doubles (then 1.0), and a recorder of the
/// `RandomUint32Uniform(n)` calls answering `n - 1`.
struct Script {
    doubles: Rc<RefCell<VecDeque<f64>>>,
    log: Rc<RefCell<Vec<String>>>,
}

impl CivicRandom for Script {
    fn uniform_f64(&mut self) -> f64 {
        self.log.borrow_mut().push("double".into());
        self.doubles.borrow_mut().pop_front().unwrap_or(1.0)
    }
    fn uniform_u32(&mut self, n: u32) -> u32 {
        self.log.borrow_mut().push(format!("u32({n})"));
        n.saturating_sub(1)
    }
}

struct Fx {
    h: Rc<Host>,
    s: Rc<CivicSim>,
    doubles: Rc<RefCell<VecDeque<f64>>>,
    log: Rc<RefCell<Vec<String>>>,
}

fn fx() -> Fx {
    let h = Host::new();
    let s = CivicSim::new(h.clone(), 0xC1, 0x1234_5678);
    let doubles = Rc::new(RefCell::new(VecDeque::new()));
    let log = Rc::new(RefCell::new(Vec::new()));
    let (d, l) = (doubles.clone(), log.clone());
    s.set_random_factory(Box::new(move |seed| {
        l.borrow_mut().push(format!("seed({seed:#X})"));
        Box::new(Script {
            doubles: d.clone(),
            log: l.clone(),
        })
    }));
    Fx { h, s, doubles, log }
}

fn ready() -> Fx {
    let f = fx();
    assert!(f.s.init());
    f.h.take();
    f
}

fn occ_msg(t: u32, occ: u64) -> CivicMessage {
    CivicMessage {
        msg_type: t,
        void1: Some(occ),
        ..CivicMessage::default()
    }
}

fn lot_msg(lot: u64, state: u32) -> CivicMessage {
    CivicMessage {
        msg_type: msg::LOT_STATE_CHANGED,
        data2: state,
        unknown: Some(lot),
        ..CivicMessage::default()
    }
}

const BM: u32 = 0xEA59_717A;
const BD: u32 = 0x2A5A_723F;

// ================================================================ identity

#[test]
fn identity() {
    for iid in [1, 0x8A41_322F, 0x4522_94AA, 0xE4FD_A3D4] {
        assert!(query_interface(iid));
    }
    for iid in [0, 0x077A_BEB2, 0xAA3F_E6BC] {
        assert!(!query_interface(iid));
    }
    assert_eq!(fx().s.class_id(), 0x0A41_3481);
}

// ================================================================ §12.1

#[test]
fn cb12_01_init() {
    let f = fx();
    assert!(f.s.init());
    let ev = f.h.take();
    let lm = f.h.dept(0x8A41_6368);
    let bm = f.h.dept(BM);
    let bd = f.h.dept(BD);
    assert_eq!(
        ev,
        vec![
            Ev::CreateDepartment(0x8A41_6368, 0x6A35_7B96),
            Ev::AddRef(lm),
            Ev::DepartmentName(lm, 0x6A3F_F01C, 0xCA49_1DD2),
            Ev::MaxFunding(lm, 1.0),
            Ev::CreateDepartment(BM, 0xEA59_7195),
            Ev::DepartmentName(bm, 0x6A3F_F01C, 0x4A59_726D),
            Ev::MaxFunding(bm, 1.0),
            Ev::CreateDepartment(BD, 0x0A5A_72D1),
            Ev::DepartmentName(bd, 0x6A3F_F01C, 0x8A5A_7330),
            Ev::MaxFunding(bd, 1.0),
            Ev::Subscribe(0xE8BD_A2E7),
            Ev::Subscribe(0x99EF_1142),
            Ev::Subscribe(0x99EF_1143),
            Ev::AddAgent(6, "cSC4CivicBuildingSimulator".into(), 2),
            Ev::RegisterLua,
        ]
    );
    assert_eq!(f.s.snapshot().landmarks, Some(lm));
    assert_eq!(
        (f.h.refs(lm), f.h.refs(bm), f.h.refs(bd)),
        (1, 0, 0),
        "only Landmarks held"
    );
    // A second Init does nothing.
    assert!(f.s.init());
    assert!(f.h.take().is_empty());
}

#[test]
fn cb12_01_init_reuses_loaded_and_existing() {
    let f = fx();
    let loaded = f.h.add_department(0x8A41_6368);
    f.h.add_department(BM);
    f.h.add_department(BD);
    f.s.with_state(|s| s.landmarks = Some(loaded));
    assert!(f.s.init());
    let ev = f.h.take();
    assert!(!ev.iter().any(|e| matches!(e, Ev::CreateDepartment(..) | Ev::AddRef(_))));
    assert_eq!(f.s.snapshot().landmarks, Some(loaded));
}

#[test]
fn shutdown_order_and_kept_containers() {
    let f = ready();
    let lm = f.s.snapshot().landmarks.unwrap();
    f.s.with_state(|s| {
        s.exclusions.insert(5);
        s.statuses.entry(7, ConditionalBuildingStatus::default);
        s.disease_centers = vec![Some(0x11), None];
        s.maintained = vec![Some(0x12), Some(0x12)];
    });
    assert!(f.s.shutdown());
    assert_eq!(
        f.h.take(),
        vec![
            Ev::UnregisterLua,
            Ev::Release(0x11),
            Ev::Release(0x12),
            Ev::Release(0x12),
            Ev::RemoveAgent,
            Ev::Release(lm),
            Ev::Unsubscribe(0xE8BD_A2E7),
            Ev::Unsubscribe(0x99EF_1142),
            Ev::Unsubscribe(0x99EF_1143),
        ]
    );
    let s = f.s.snapshot();
    assert!(!s.initialized && s.landmarks.is_none());
    assert!(s.disease_centers.is_empty() && s.maintained.is_empty());
    assert!(s.exclusions.contains(5) && s.statuses.contains(7), "not cleared");
    assert!(f.s.shutdown());
    assert!(f.h.take().is_empty());
    // Release.
    let g = ready();
    g.s.add_ref();
    g.s.add_ref();
    assert_eq!(g.s.release(), 1);
    assert_eq!(g.s.release(), 0);
    assert!(g.s.snapshot().destroyed);
    assert!(g.h.take().contains(&Ev::RemoveAgent));
}

// ================================================================ §12.2

#[test]
fn cb12_02_exclusion() {
    let f = ready();
    f.h.building(0x20, 0x0339_0000);
    f.h.occ(0x20, |o| o.u32s.insert(prop::EXCLUSION_ID, 0x8A9A_E664));
    f.s.do_message(&occ_msg(msg::OCCUPANT_INSERTED, 0x20));
    assert!(f.s.is_building_excluded(0x8A9A_E664));
    f.s.do_message(&occ_msg(msg::OCCUPANT_REMOVED, 0x20));
    assert!(!f.s.is_building_excluded(0x8A9A_E664));
    // A non-building occupant with the property changes nothing.
    f.h.set_occ(
        0x21,
        Occ {
            occ_type: 0xC772_BF98,
            ..Occ::default()
        },
    );
    f.h.occ(0x21, |o| o.u32s.insert(prop::EXCLUSION_ID, 0x8A9A_E664));
    f.s.do_message(&occ_msg(msg::OCCUPANT_INSERTED, 0x21));
    assert!(!f.s.is_building_excluded(0x8A9A_E664));
    // [quirk] A preview occupant is not tested; and erasing removes the ID
    // even when another building carries it.
    f.s.do_message(&occ_msg(msg::OCCUPANT_INSERTED, 0x20));
    f.h.building(0x22, 0x0339_0000);
    f.h.occ(0x22, |o| o.u32s.insert(prop::EXCLUSION_ID, 0x8A9A_E664));
    f.s.do_message(&occ_msg(msg::OCCUPANT_INSERTED, 0x22));
    f.s.do_message(&occ_msg(msg::OCCUPANT_REMOVED, 0x22));
    assert!(!f.s.is_building_excluded(0x8A9A_E664));
}

// ================================================================ §12.3

#[test]
fn cb12_03_counts() {
    let f = ready();
    let key = 0x032B_0000;
    f.s.update_conditional_building_status(key, &ConditionalBuildingStatus::default());
    f.h.building(0x30, 0x0300_0001);
    f.h.occ(0x30, |o| {
        o.key_instance = Some(key);
        o.bools.insert(prop::CONDITIONAL_BUILDING, true);
    });
    let count = || f.s.conditional_building_status(key).unwrap().building_count;
    f.s.do_message(&occ_msg(msg::OCCUPANT_INSERTED, 0x30));
    assert_eq!(count(), 1);
    f.s.do_message(&occ_msg(msg::OCCUPANT_REMOVED, 0x30));
    assert_eq!(count(), 0);
    // 16-bit wrap.
    f.s.do_message(&occ_msg(msg::OCCUPANT_REMOVED, 0x30));
    assert_eq!(count(), 0xFFFF);
    f.s.do_message(&occ_msg(msg::OCCUPANT_INSERTED, 0x30));
    assert_eq!(count(), 0);
    // A building without a status entry changes nothing.
    f.h.building(0x31, 0x0300_0002);
    f.h.occ(0x31, |o| {
        o.key_instance = Some(0x0999_0000);
        o.bools.insert(prop::CONDITIONAL_BUILDING, true);
    });
    f.s.do_message(&occ_msg(msg::OCCUPANT_INSERTED, 0x31));
    assert!(f.s.conditional_building_status(0x0999_0000).is_none());
    // Conditional false changes nothing.
    f.h.occ(0x30, |o| o.bools.insert(prop::CONDITIONAL_BUILDING, false));
    f.s.do_message(&occ_msg(msg::OCCUPANT_INSERTED, 0x30));
    assert_eq!(count(), 0);
    // No key: nothing.
    f.h.occ(0x30, |o| {
        o.bools.insert(prop::CONDITIONAL_BUILDING, true);
        o.key_instance = None;
    });
    f.s.do_message(&occ_msg(msg::OCCUPANT_INSERTED, 0x30));
    assert_eq!(count(), 0);
}

// ================================================================ §12.4

#[test]
fn cb12_04_activation() {
    let f = ready();
    let ty = 0x031F_0000;
    let lot = f.h.building(0x40, ty);
    f.h.item(0x40, PURPOSE_MAINTENANCE, BM, 0, 30);
    // data2 = 2 does nothing (the lot is still queried and released).
    f.s.do_message(&lot_msg(lot, 2));
    assert_eq!(f.h.take(), vec![Ev::AddRef(lot), Ev::Release(lot)]);
    assert_eq!(f.s.landmark_count(), 0);
    // data2 = 3.
    f.s.do_message(&lot_msg(lot, 3));
    let bm = f.h.dept(BM);
    assert_eq!(
        f.h.take(),
        vec![
            Ev::AddRef(lot),
            Ev::ChangeLine(0x40, 0, ty),
            Ev::CreateLine(bm, ty, false),
            Ev::AddRef(0x40),
            Ev::Release(lot),
        ]
    );
    assert_eq!(f.h.budget_item(0x40, PURPOSE_MAINTENANCE).unwrap().line_item, ty);
    assert_eq!(f.h.line_of(BM, ty).unwrap().full_expenses, 30);
    assert_eq!(f.s.snapshot().maintained, vec![Some(0x40)]);
    // [quirk] A second activation: two entries, +60.
    f.s.do_message(&lot_msg(lot, 3));
    assert_eq!(f.h.line_of(BM, ty).unwrap().full_expenses, 60);
    assert_eq!(f.s.snapshot().maintained, vec![Some(0x40), Some(0x40)]);
    assert_eq!(f.s.landmark_count(), 2);
    assert_eq!(f.h.refs(0x40), 2);
    // A lot without a building: nothing.
    f.h.lots.borrow_mut().insert(0x777, None);
    f.h.take();
    f.s.do_message(&lot_msg(0x777, 3));
    assert_eq!(f.h.take(), vec![Ev::AddRef(0x777), Ev::Release(0x777)]);
}

#[test]
fn activation_without_department_still_lists() {
    let f = ready();
    let lot = f.h.building(0x41, 0x0400_0000);
    f.h.item(0x41, PURPOSE_MAINTENANCE, 0x1234_5678, 7, 30);
    f.s.do_lot_activated(lot);
    assert_eq!(f.s.snapshot().maintained, vec![Some(0x41)]);
    assert!(!f
        .h
        .take()
        .iter()
        .any(|e| matches!(e, Ev::CreateLine(..) | Ev::ChangeLine(..))));
}

// ================================================================ §12.5

#[test]
fn cb12_05_income() {
    let f = ready();
    let ty = 0x033A_0000;
    let lot = f.h.building(0x50, ty);
    f.h.item(0x50, PURPOSE_INCOME, BD, 0x99, 300);
    f.s.do_message(&lot_msg(lot, 3));
    let bd = f.h.dept(BD);
    let line = f.h.line_of(BD, ty).expect("keyed by the building type");
    assert_eq!((line.income, line.line_type), (300, 1));
    assert!(f.h.line_of(BD, 0x99).is_none(), "not item.line");
    let ev = f.h.take();
    assert!(ev.contains(&Ev::CreateLine(bd, ty, false)));
    assert_eq!(f.s.landmark_count(), 0, "income only: not maintained");
    // Removal deletes the line.
    f.s.do_message(&occ_msg(msg::OCCUPANT_REMOVED, 0x50));
    assert!(f.h.line_of(BD, ty).is_none());
    assert!(f.h.take().contains(&Ev::RemoveLine(bd, ty)));
    // A line with 500 drops to 200 and stays.
    f.s.do_lot_activated(lot);
    f.s.do_lot_activated(lot);
    let lt = f.h.line_item(bd, ty).unwrap();
    f.h.set_line_income(lt, 500);
    f.s.do_building_removed(0x50);
    assert_eq!(f.h.line_of(BD, ty).unwrap().income, 200);
    // An existing line keeps its type (SetType only when created).
    f.h.line_data.borrow_mut().get_mut(&lt).unwrap().line_type = 0;
    f.h.take();
    f.s.do_lot_activated(lot);
    assert!(!f.h.take().iter().any(|e| matches!(e, Ev::SetLineType(..))));
    assert_eq!(f.h.line_of(BD, ty).unwrap().income, 500);
}

// ================================================================ §12.6

#[test]
fn cb12_06_monthly_condition() {
    let f = ready();
    let lot_a = f.h.building(0x60, 0x0500_0000);
    let lot_b = f.h.building(0x61, 0x0500_0001);
    for o in [0x60, 0x61] {
        f.h.item(o, PURPOSE_MAINTENANCE, BM, 1, 10);
    }
    f.s.with_state(|s| s.maintained = vec![Some(0x60), None, Some(0x61)]);
    f.h.conditions.borrow_mut().insert(lot_b, 1);
    let bm = f.h.dept(BM);
    let cond = |l| f.h.conditions.borrow()[&l];
    f.h.funding.borrow_mut().insert(bm, 0.49);
    f.s.do_message(&CivicMessage::of_type(msg::SIM_NEW_MONTH));
    assert_eq!((cond(lot_a), cond(lot_b)), (2, 1));
    f.h.funding.borrow_mut().insert(bm, 0.5);
    f.s.do_monthly_simulation();
    assert_eq!((cond(lot_a), cond(lot_b)), (0, 1));
    // NaN behaves like ≥ 0.5.
    f.h.conditions.borrow_mut().insert(lot_a, 2);
    f.h.funding.borrow_mut().insert(bm, f32::NAN);
    f.s.do_monthly_simulation();
    assert_eq!(cond(lot_a), 0);
    f.h.funding.borrow_mut().insert(bm, f32::NAN);
    f.s.do_monthly_simulation();
    assert_eq!(cond(lot_a), 0, "NaN never runs down");
    // Nothing is drawn without a Disease Research Center.
    assert!(f.log.borrow().is_empty());
}

// ================================================================ §12.7

#[test]
fn cb12_07_ape_escape() {
    let f = ready();
    f.s.with_state(|s| s.disease_centers = vec![Some(0x70), Some(0x71)]);
    f.doubles.borrow_mut().push_back(0.009_999_999_776_482_582);
    f.s.do_monthly_simulation();
    assert_eq!(
        *f.log.borrow(),
        vec![
            format!("seed({:#X})", temporary_seed(0x1234_5678, 0)),
            "double".to_string(),
            "u32(2)".to_string()
        ]
    );
    assert_eq!(f.h.take(), vec![Ev::Controllers("ape_escape".into(), 0x71)]);
    // The double 0.01 (slightly larger) does not escape.
    f.log.borrow_mut().clear();
    f.doubles.borrow_mut().push_back(0.01);
    f.s.do_monthly_simulation();
    assert_eq!(
        *f.log.borrow(),
        vec![
            format!("seed({:#X})", temporary_seed(0x1234_5678, 1)),
            "double".to_string()
        ]
    );
    assert!(f.h.take().is_empty());
    // Without an automata controller manager nothing happens after the draws.
    f.h.automata.set(false);
    f.log.borrow_mut().clear();
    f.doubles.borrow_mut().push_back(0.0);
    f.s.do_monthly_simulation();
    assert_eq!(f.log.borrow().len(), 3);
    assert!(f.h.take().is_empty());
    // A null entry escapes nothing [choice].
    f.h.automata.set(true);
    f.s.with_state(|s| s.disease_centers = vec![Some(0x70), None]);
    f.doubles.borrow_mut().push_back(0.0);
    f.s.do_monthly_simulation();
    assert!(f.h.take().is_empty());
    assert_eq!(f.s.snapshot().temporaries, 4);
}

#[test]
fn ape_escape_seeds_and_default_generator() {
    assert_eq!(temporary_seed(0, 0), 0x0A41_3481);
    assert_eq!(temporary_seed(0x0A41_3481, 0), 1, "0 moves by XOR 1");
    // The default factory draws from a cRZRandom with that seed.
    let mut a = (default_random_factory())(0x55);
    let mut b = crate::rng::RzRandom::new(
        0x55,
        &crate::rng::FixedSeeds {
            wall_clock: 0,
            process_clock: 0,
        },
    );
    assert_eq!(a.uniform_f64(), b.uniform_f64());
    assert_eq!(a.uniform_u32(7), b.uniform_u32(7));
}

#[test]
fn disease_center_list() {
    let f = ready();
    let lot = f.h.building(0x80, 0x033F_0000);
    f.h.occ(0x80, |o| o.groups.push(GROUP_DISEASE_CENTER));
    f.s.do_lot_activated(lot);
    f.s.do_lot_activated(lot);
    assert_eq!(f.s.snapshot().disease_centers, vec![Some(0x80)], "no duplicate");
    assert_eq!(f.h.refs(0x80), 1);
    f.s.do_message(&occ_msg(msg::OCCUPANT_REMOVED, 0x80));
    assert!(f.s.snapshot().disease_centers.is_empty());
    assert_eq!(f.h.refs(0x80), 0);
    // Not listed: no release.
    f.h.take();
    f.s.do_building_removed(0x80);
    assert!(f.h.take().is_empty());
}

// ================================================================ §12.8

#[test]
fn cb12_08_status_api() {
    let f = ready();
    let k = 0x0400_0000;
    assert!(f.s.conditional_building_status(k).is_none());
    let want = ConditionalBuildingStatus {
        status: 6,
        building_count: 2,
        tooltip: b"x".to_vec(),
    };
    assert!(f.s.update_conditional_building_status(k, &want));
    assert_eq!(f.s.conditional_building_status(k), Some(want.clone()));
    assert!(f.s.conditional_building_status(k + 1).is_none());
    // Writes through the "pointer" persist.
    f.s.with_status_mut(k, |st| st.status |= status_bits::HIDDEN);
    assert_eq!(f.s.conditional_building_status(k).unwrap().status, 7);
    assert!(f.s.with_status_mut(k + 1, |_| ()).is_none());
    // Update overwrites all three fields.
    f.s.update_conditional_building_status(k, &ConditionalBuildingStatus::default());
    assert_eq!(
        f.s.conditional_building_status(k),
        Some(ConditionalBuildingStatus::default())
    );
}

// ================================================================ §4.8

#[test]
fn worked_example_4_8() {
    let f = ready();
    let ty = 0x031F_0000;
    f.s.update_conditional_building_status(ty, &ConditionalBuildingStatus::default());
    let lot = f.h.building(0x90, ty);
    f.h.occ(0x90, |o| {
        o.key_instance = Some(ty);
        o.bools.insert(prop::CONDITIONAL_BUILDING, true);
        o.u32s.insert(prop::EXCLUSION_ID, 0xAA9E_E012);
    });
    f.h.item(0x90, PURPOSE_MAINTENANCE, BM, 0, 30);
    // 1.
    f.s.do_message(&occ_msg(msg::OCCUPANT_INSERTED, 0x90));
    assert_eq!(f.s.conditional_building_status(ty).unwrap().building_count, 1);
    assert!(f.s.is_building_excluded(0xAA9E_E012));
    // 2.
    f.s.do_message(&lot_msg(lot, 3));
    assert_eq!(f.h.line_of(BM, ty).unwrap().full_expenses, 30);
    assert_eq!(f.s.landmark_count(), 1);
    // 3.
    let bm = f.h.dept(BM);
    f.h.funding.borrow_mut().insert(bm, 0.4);
    f.s.do_message(&CivicMessage::of_type(msg::SIM_NEW_MONTH));
    assert_eq!(f.h.conditions.borrow()[&lot], 2);
    f.h.funding.borrow_mut().insert(bm, 0.6);
    f.s.do_message(&CivicMessage::of_type(msg::SIM_NEW_MONTH));
    assert_eq!(f.h.conditions.borrow()[&lot], 0);
    // 4.
    f.s.do_message(&occ_msg(msg::OCCUPANT_REMOVED, 0x90));
    assert_eq!(f.s.conditional_building_status(ty).unwrap().building_count, 0);
    assert!(!f.s.is_building_excluded(0xAA9E_E012));
    assert_eq!(f.h.line_of(BM, ty).unwrap().full_expenses, 0);
    assert_eq!(f.s.landmark_count(), 0);
    assert_eq!(f.h.refs(0x90), 0);
}

// ================================================================ §12.9

struct Resolver;

impl RefResolver for Resolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        Some(RefValue::Host {
            token: u64::from(object_id),
            class_id,
        })
    }
}

/// Writes with object IDs = host tokens (so the record shows them).
fn record(s: &CivicSim) -> Vec<u8> {
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(reg);
    assert!(s.write(&mut w));
    w.into_bytes()
}

fn load(s: &CivicSim, bytes: &[u8]) -> bool {
    let mut r = Resolver;
    s.read(&mut ByteReader::with_resolver(bytes, &mut r))
}

#[test]
fn cb12_09_save_round_trip_and_order() {
    let f = ready();
    f.s.with_state(|s| {
        // Keys 1 and 194 share bucket 1; 194 is newer, so it comes first.
        s.exclusions.insert(1);
        s.exclusions.insert(194);
        s.exclusions.insert(5);
        s.disease_centers = vec![Some(0x31)];
        s.maintained = vec![Some(0x32), None, Some(0x32)];
    });
    for (k, st) in [(1u32, 0u8), (194, 1), (2, 6)] {
        f.s.update_conditional_building_status(
            k,
            &ConditionalBuildingStatus {
                status: st,
                building_count: k as u16,
                tooltip: format!("t{k}").into_bytes(),
            },
        );
    }
    let bytes = record(&f.s);
    // Version 6, 3 exclusions in bucket order: 194, 1 (bucket 1), 5.
    assert_eq!(&bytes[..2], &[6, 0]);
    let ids: Vec<u32> = (0..3)
        .map(|i| u32::from_le_bytes(bytes[6 + 4 * i..10 + 4 * i].try_into().unwrap()))
        .collect();
    assert_eq!(u32::from_le_bytes(bytes[2..6].try_into().unwrap()), 3);
    assert_eq!(ids, vec![194, 1, 5]);
    // Statuses: 194, 1, 2.
    let mut p = 18;
    assert_eq!(u32::from_le_bytes(bytes[p..p + 4].try_into().unwrap()), 3);
    p += 4;
    let mut keys = Vec::new();
    for _ in 0..3 {
        let k = u32::from_le_bytes(bytes[p..p + 4].try_into().unwrap());
        keys.push(k);
        let len = u32::from_le_bytes(bytes[p + 7..p + 11].try_into().unwrap()) as usize;
        p += 11 + len;
    }
    assert_eq!(keys, vec![194, 1, 2]);
    // Read into a fresh object; write again: byte-identical.
    let g = fx();
    assert!(load(&g.s, &bytes));
    assert_eq!(record(&g.s), bytes);
    let st = g.s.snapshot();
    assert_eq!(st.exclusions.iter_order(), vec![194, 1, 5]);
    assert_eq!(st.statuses.keys(), vec![194, 1, 2]);
    // Object IDs (the save registry's numbering) are the reader's tokens:
    // the department 1, the Disease Research Center 2, the building 3.
    assert_eq!(st.landmarks, Some(1));
    assert_eq!(st.maintained, vec![Some(3), None, Some(3)]);
    assert_eq!(st.disease_centers, vec![Some(2)]);
    // Version 5 is rejected; nothing changes.
    let mut v5 = bytes.clone();
    v5[0] = 5;
    let h = fx();
    assert!(!load(&h.s, &v5));
    assert!(h.s.snapshot().exclusions.is_empty());
    // A truncated record changes nothing and releases what it took.
    assert!(!load(&h.s, &bytes[..bytes.len() - 2]));
    let st = h.s.snapshot();
    assert!(st.exclusions.is_empty() && st.landmarks.is_none());
    let ev = h.h.take();
    assert!(ev.contains(&Ev::Release(2)));
}

#[test]
fn save_layout_and_read_semantics() {
    let f = fx();
    // Empty: version, 0, 0, null department, 0, 0.
    let bytes = record(&f.s);
    assert_eq!(
        bytes,
        vec![6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
    );
    // References: the department's class and the occupants' class.
    let g = ready();
    g.s.with_state(|s| s.maintained = vec![Some(0x32)]);
    let b = record(&g.s);
    let n = b.len();
    assert_eq!(&b[n - 12..n - 8], &[1, 0, 0, 0], "maintained count");
    assert_eq!(u32::from_le_bytes(b[n - 4..].try_into().unwrap()), OCCUPANT_CLASS);
    assert_eq!(
        u32::from_le_bytes(b[n - 20..n - 16].try_into().unwrap()),
        DEPARTMENT_CLASS
    );
    // Read: the old department is released; old list entries are not;
    // existing hash entries are kept; an unresolvable reference is null.
    let lm = g.s.snapshot().landmarks.unwrap();
    g.s.with_state(|s| {
        s.disease_centers = vec![Some(0x99)];
        s.exclusions.insert(42);
    });
    let mut rec = vec![6, 0, 1, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0];
    rec.extend(0u32.to_le_bytes()); // null department
    rec.extend(1u32.to_le_bytes());
    rec.extend(0xDEADu32.to_le_bytes());
    rec.extend(OCCUPANT_CLASS.to_le_bytes());
    rec.extend(10_000_000u32.to_le_bytes());
    g.h.take();
    assert!(load(&g.s, &rec));
    let ev = g.h.take();
    assert_eq!(ev, vec![Ev::Release(lm)]);
    let st = g.s.snapshot();
    assert_eq!(st.exclusions.iter_order(), vec![9, 42]);
    assert_eq!(st.disease_centers, vec![None]);
    assert!(st.maintained.is_empty());
    assert!(st.landmarks.is_none());
}

// ================================================================ §12.10

/// §7.1, synthetic: the corpus' record shapes (46 statuses with text and no
/// exclusions; exclusions with maintained buildings) read and re-write
/// byte-identically. [not the corpus] The 421 saves live in the user's
/// documents folder, which this lane may not read.
#[test]
fn cb12_10_corpus_shapes() {
    let mk = |n_ex: u32, n_st: u32, maint: u32| {
        let mut set = StlHashSet::with_hint(HASH_HINT);
        let ex = [0x4A9A_E483u32, 0xAA9E_E012, 0xAA9A_E59F, 0xEA9A_DE07, 0x8A9A_DCB0];
        for &e in ex.iter().take(n_ex as usize) {
            set.insert(e);
        }
        let mut map: StlHashMap<ConditionalBuildingStatus> = StlHashMap::with_hint(HASH_HINT);
        for i in 0..n_st {
            let key = 0x0320_0000 + i * 0x1_0000;
            let st = if i % 2 == 0 { 0 } else { 1 };
            let text = if st == 0 {
                format!("Building {i} is not available.\nRequirements: ...").into_bytes()
            } else {
                Vec::new()
            };
            map.entry(key, || ConditionalBuildingStatus {
                status: st,
                building_count: 0,
                tooltip: text,
            });
        }
        let mut b = vec![6, 0];
        b.extend((set.len() as u32).to_le_bytes());
        for k in set.iter_order() {
            b.extend(k.to_le_bytes());
        }
        b.extend((map.len() as u32).to_le_bytes());
        for (k, st) in map.iter() {
            b.extend(k.to_le_bytes());
            b.push(st.status);
            b.extend(st.building_count.to_le_bytes());
            b.extend((st.tooltip.len() as u32).to_le_bytes());
            b.extend(&st.tooltip);
        }
        b.extend(1u32.to_le_bytes());
        b.extend(DEPARTMENT_CLASS.to_le_bytes());
        b.extend(0u32.to_le_bytes());
        b.extend(maint.to_le_bytes());
        for i in 0..maint {
            b.extend((2 + i).to_le_bytes());
            b.extend(OCCUPANT_CLASS.to_le_bytes());
        }
        b
    };
    for (n_ex, n_st, maint) in [(0, 46, 0), (0, 63, 1), (1, 60, 1), (3, 60, 1), (5, 61, 3)] {
        let bytes = mk(n_ex, n_st, maint);
        let f = fx();
        assert!(load(&f.s, &bytes));
        assert_eq!(record(&f.s), bytes, "({n_ex}, {n_st}, {maint})");
        let st = f.s.snapshot();
        assert_eq!(st.exclusions.len() as u32, n_ex);
        assert_eq!(st.statuses.len() as u32, n_st);
        assert_eq!(st.landmarks, Some(1));
        // Init reuses the loaded Landmarks department.
        assert!(f.s.init());
        assert!(!f.h.take().contains(&Ev::CreateDepartment(0x8A41_6368, 0x6A35_7B96)));
    }
}

/// More than 193 keys: the table grows to 389 buckets, and a reader rebuilds
/// the same order.
#[test]
fn large_tables_keep_order() {
    let f = fx();
    f.s.with_state(|s| {
        for k in 0..250u32 {
            s.exclusions.insert(k.wrapping_mul(0x9E37_79B9));
        }
    });
    assert_eq!(f.s.snapshot().exclusions.bucket_count(), 389);
    let bytes = record(&f.s);
    let g = fx();
    assert!(load(&g.s, &bytes));
    assert_eq!(record(&g.s), bytes);
}
