// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests of the park manager against a fake host: the §12 tests of
//! `docs/specs/parks.md` (named `pk12_01_…` to `pk12_08_…`), the §4.6
//! worked examples, the lifecycle and the record.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};
use std::rc::Rc;

use super::*;
use crate::property::PropertyHolder;
use crate::stream::{ByteReader, ByteWriter, RefResolver, RefValue, SaveObject, SaveRegistry};
use crate::variant::{Elem, Variant};

#[derive(Clone, Debug, PartialEq)]
enum Ev {
    AddAgent(u32, String, u32),
    RemoveAgent,
    AcquireMessageServer,
    Subscribe(u32),
    Unsubscribe(u32),
    ReleaseMessageServer,
    AcquireCity,
    ReleaseCity,
    AcquireBudget,
    ReleaseBudget,
    AcquireLotManager,
    ReleaseLotManager,
    AddRef(u64),
    Release(u64),
    CreateDepartment(u32, u32),
    DepartmentName(u64, u32, u32),
    MaxFunding(u64, f32),
    ChangeLine(u64, i32, u32),
    CreateLine(u64, u32, bool),
    DisplayFlag(u64, u32, bool),
    SetCondition(u64, u32),
    ZooEscape(u64, u64),
    Controllers(String, u64),
}

#[derive(Clone, Debug, Default)]
struct Line {
    full_expenses: i64,
    secondary: i64,
}

#[derive(Clone, Debug, Default)]
struct Occ {
    occ_type: u32,
    building: bool,
    building_type: u32,
    props: Vec<u32>,
    groups: Vec<u32>,
    lot: Option<u64>,
    item: Option<BudgetItem>,
}

struct Host {
    events: RefCell<Vec<Ev>>,
    departments: RefCell<BTreeMap<u32, u64>>,
    funding: RefCell<BTreeMap<u64, f32>>,
    lines: RefCell<BTreeMap<(u64, u32), u64>>,
    line_data: RefCell<BTreeMap<u64, Line>>,
    occupants: RefCell<BTreeMap<u64, Occ>>,
    conditions: RefCell<BTreeMap<u64, u32>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    next: Cell<u64>,
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
            conditions: RefCell::new(BTreeMap::new()),
            refs: RefCell::new(BTreeMap::new()),
            tuning: RefCell::new(None),
            next: Cell::new(0x1000),
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
    fn refs(&self, t: u64) -> i64 {
        self.refs.borrow().get(&t).copied().unwrap_or(0)
    }
    fn dept(&self) -> u64 {
        self.departments.borrow()[&DEPARTMENT_ID]
    }
    fn set_funding(&self, f: f32) {
        let d = self.dept();
        self.funding.borrow_mut().insert(d, f);
    }
    fn line(&self, line: u32) -> Option<Line> {
        let t = *self.lines.borrow().get(&(self.dept(), line))?;
        self.line_data.borrow().get(&t).cloned()
    }
    /// A park building of `ty` with a lot `token + 0x10000` and a budget item
    /// (department Parks and Recreation, line 0, `cost`).
    fn park(&self, token: u64, ty: u32, cost: i64, groups: &[u32]) -> u64 {
        let lot = token + 0x1_0000;
        self.occupants.borrow_mut().insert(
            token,
            Occ {
                occ_type: OCCUPANT_TYPE_BUILDING,
                building: true,
                building_type: ty,
                groups: groups.to_vec(),
                lot: Some(lot),
                item: Some(BudgetItem {
                    department_id: DEPARTMENT_ID,
                    line_item: 0,
                    purpose: PURPOSE_PARK,
                    cost,
                    index: 0,
                    local_funding: 1.0,
                }),
                ..Occ::default()
            },
        );
        self.conditions.borrow_mut().insert(lot, 0);
        lot
    }
}

impl ParkHost for Host {
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        self.ev(Ev::AddAgent(level, name.into(), flags));
    }
    fn remove_agent(&self) {
        self.ev(Ev::RemoveAgent);
    }
    fn acquire_message_server(&self) -> bool {
        self.ev(Ev::AcquireMessageServer);
        true
    }
    fn subscribe(&self, m: u32) {
        self.ev(Ev::Subscribe(m));
    }
    fn unsubscribe(&self, m: u32) {
        self.ev(Ev::Unsubscribe(m));
    }
    fn release_message_server(&self) {
        self.ev(Ev::ReleaseMessageServer);
    }
    fn acquire_city(&self) -> bool {
        self.ev(Ev::AcquireCity);
        true
    }
    fn release_city(&self) {
        self.ev(Ev::ReleaseCity);
    }
    fn acquire_budget(&self) -> bool {
        self.ev(Ev::AcquireBudget);
        true
    }
    fn release_budget(&self) {
        self.ev(Ev::ReleaseBudget);
    }
    fn acquire_lot_manager(&self) -> bool {
        self.ev(Ev::AcquireLotManager);
        true
    }
    fn release_lot_manager(&self) {
        self.ev(Ev::ReleaseLotManager);
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
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
        let t = self.fresh();
        self.departments.borrow_mut().insert(id, t);
        self.funding.borrow_mut().insert(t, 1.0);
        Some(t)
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
        let o = self.occupants.borrow();
        let i = o.get(&occupant)?.item?;
        (i.purpose == purpose).then_some(i)
    }
    fn change_budget_item_line(&self, occupant: u64, index: i32, line: u32) {
        self.ev(Ev::ChangeLine(occupant, index, line));
        if let Some(i) = self
            .occupants
            .borrow_mut()
            .get_mut(&occupant)
            .and_then(|o| o.item.as_mut())
        {
            i.line_item = line;
        }
    }
    fn line_item_from_budget_item(&self, item: &BudgetItem) -> Option<u64> {
        let d = self.department_budget(item.department_id)?;
        self.line_item(d, item.line_item)
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
    fn set_line_display_flag(&self, line: u64, flag: u32, on: bool) {
        self.ev(Ev::DisplayFlag(line, flag, on));
    }
    fn add_to_full_expenses(&self, line: u64, amount: i64) {
        self.line_data.borrow_mut().get_mut(&line).unwrap().full_expenses += amount;
    }
    fn line_secondary(&self, line: u64) -> i64 {
        self.line_data.borrow()[&line].secondary
    }
    fn set_line_secondary(&self, line: u64, value: i64) {
        self.line_data.borrow_mut().get_mut(&line).unwrap().secondary = value;
    }
    fn occupant_type(&self, occupant: u64) -> u32 {
        self.occupants.borrow().get(&occupant).map_or(0, |o| o.occ_type)
    }
    fn occupant_has_property(&self, occupant: u64, id: u32) -> bool {
        self.occupants
            .borrow()
            .get(&occupant)
            .is_some_and(|o| o.props.contains(&id))
    }
    fn is_occupant_group(&self, occupant: u64, group: u32) -> bool {
        self.occupants
            .borrow()
            .get(&occupant)
            .is_some_and(|o| o.groups.contains(&group))
    }
    fn is_building_occupant(&self, occupant: u64) -> bool {
        self.occupants.borrow().get(&occupant).is_some_and(|o| o.building)
    }
    fn building_type(&self, building: u64) -> u32 {
        self.occupants.borrow()[&building].building_type
    }
    fn occupant_position_lot(&self, occupant: u64) -> Option<u64> {
        self.occupants.borrow().get(&occupant)?.lot
    }
    fn set_lot_condition(&self, lot: u64, condition: u32) {
        self.ev(Ev::SetCondition(lot, condition));
        self.conditions.borrow_mut().insert(lot, condition);
    }
    fn post_zoo_escape(&self, manager: u64, zoo: u64) {
        self.ev(Ev::ZooEscape(manager, zoo));
    }
    fn add_controllers(&self, name: &str, occupant: u64) {
        self.ev(Ev::Controllers(name.into(), occupant));
    }
    fn object_ref(&self, token: u64) -> RefValue {
        RefValue::Host {
            token,
            class_id: OCCUPANT_CLASS,
        }
    }
    fn resolve_ref(&self, r: &RefValue, _iid: u32) -> Option<u64> {
        match r {
            RefValue::Host { token, .. } if *token != 0xDEAD => Some(*token),
            _ => None,
        }
    }
}

/// A scripted generator: queued doubles (then 1.0), `uniform_u32(n)` gives
/// `n - 1`; every call is logged.
struct Script {
    doubles: Rc<RefCell<VecDeque<f64>>>,
    log: Rc<RefCell<Vec<String>>>,
}

impl ParkRandom for Script {
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
    m: Rc<ParkManager>,
    doubles: Rc<RefCell<VecDeque<f64>>>,
    log: Rc<RefCell<Vec<String>>>,
}

fn fx_with(tuning: Option<PropertyHolder>) -> Fx {
    let h = Host::new();
    *h.tuning.borrow_mut() = tuning.map(Rc::new);
    let m = ParkManager::new(h.clone(), 0xAA, 7);
    let doubles = Rc::new(RefCell::new(VecDeque::new()));
    let log = Rc::new(RefCell::new(Vec::new()));
    m.set_random(Box::new(Script {
        doubles: doubles.clone(),
        log: log.clone(),
    }));
    Fx { h, m, doubles, log }
}

/// Initialised, code-default tuning (no exemplar), event log cleared.
fn ready() -> Fx {
    let f = fx_with(None);
    assert!(f.m.init());
    f.h.take();
    f
}

fn ins(occ: u64) -> ParkMessage {
    ParkMessage {
        msg_type: msg::OCCUPANT_INSERTED,
        void1: Some(occ),
        ..ParkMessage::default()
    }
}

fn rem(occ: u64) -> ParkMessage {
    ParkMessage {
        msg_type: msg::OCCUPANT_REMOVED,
        void1: Some(occ),
        ..ParkMessage::default()
    }
}

fn month() -> ParkMessage {
    ParkMessage::of_type(msg::SIM_NEW_MONTH)
}

#[test]
fn identity() {
    for iid in [1, 0xAA3F_E6BC, 0x4522_94AA, 0xE4FD_A3D4] {
        assert!(query_interface(iid));
    }
    for iid in [0, 0x8A41_322F] {
        assert!(!query_interface(iid));
    }
    assert_eq!(fx_with(None).m.class_id(), 0xAA3F_E6BC);
}

// ================================================================ §12.1

#[test]
fn pk12_01_init() {
    let f = fx_with(None);
    assert!(f.m.init());
    let d = f.h.dept();
    assert_eq!(
        f.h.take(),
        vec![
            Ev::AcquireCity,
            Ev::AcquireBudget,
            Ev::CreateDepartment(0x8A42_60A6, 0x6A35_7B96),
            Ev::DepartmentName(d, 0x6A3F_F01C, 0xCA49_375E),
            Ev::MaxFunding(d, 1.0),
            Ev::AddRef(d),
            Ev::AcquireLotManager,
            Ev::AcquireMessageServer,
            Ev::Subscribe(0x99EF_1142),
            Ev::Subscribe(0x99EF_1143),
            Ev::AddAgent(6, "cSC4ParkManager".into(), 2),
        ]
    );
    assert!(f.m.init());
    assert!(f.h.take().is_empty());
    // An existing department is not created, only held.
    let g = fx_with(None);
    g.h.departments.borrow_mut().insert(DEPARTMENT_ID, 0x77);
    assert!(g.m.init());
    let ev = g.h.take();
    assert!(!ev.iter().any(|e| matches!(e, Ev::CreateDepartment(..))));
    assert!(ev.contains(&Ev::AddRef(0x77)));
}

#[test]
fn shutdown_and_release() {
    let f = ready();
    let d = f.h.dept();
    f.m.with_state(|s| {
        s.parks = vec![Some(0x10), None];
        s.zoos = vec![Some(0x10)];
        s.underfunded = true;
    });
    assert!(f.m.shutdown());
    assert_eq!(
        f.h.take(),
        vec![
            Ev::Release(0x10),
            Ev::Release(0x10),
            Ev::RemoveAgent,
            Ev::Unsubscribe(0x99EF_1142),
            Ev::Unsubscribe(0x99EF_1143),
            Ev::ReleaseMessageServer,
            Ev::Release(d),
            Ev::ReleaseLotManager,
            Ev::ReleaseBudget,
            Ev::ReleaseCity,
        ]
    );
    let s = f.m.snapshot();
    assert!(s.parks.is_empty() && s.zoos.is_empty() && s.department.is_none());
    assert!(s.underfunded, "the latch is kept");
    assert!(f.m.shutdown());
    assert!(f.h.take().is_empty());
    // Release.
    let g = ready();
    g.m.add_ref();
    g.m.add_ref();
    assert_eq!(g.m.release(), 1);
    assert_eq!(g.m.release(), 0);
    assert!(g.m.snapshot().destroyed);
    assert_eq!(g.m.snapshot().ref_count, 1);
}

// ================================================================ §12.2

#[test]
fn pk12_02_tuning() {
    let f = fx_with(Some(shipped_tuning()));
    f.m.init();
    let s = f.m.snapshot();
    assert_eq!(s.distress_threshold, 25.0);
    assert_eq!(s.escape_curve.points(), vec![(0.0, 40.0), (25.0, 10.0), (100.0, 1.0)]);
    // Without the curve property: the code default.
    let mut t = shipped_tuning();
    t.remove(prop::ESCAPE_CURVE);
    let g = fx_with(Some(t));
    g.m.init();
    assert_eq!(g.m.snapshot().escape_curve.points(), vec![(0.0, 50.0), (100.0, 1.0)]);
    // A single value is treated as absent (props::curve): the code default.
    let mut t = PropertyHolder::new();
    t.add(prop::ESCAPE_CURVE, Variant::Scalar(Elem::F32(3.0)));
    let g = fx_with(Some(t));
    g.m.init();
    assert_eq!(g.m.snapshot().distress_threshold, 0.25, "missing: keeps 0.25");
    assert_eq!(g.m.snapshot().escape_curve.points(), vec![(0.0, 50.0), (100.0, 1.0)]);
    // No exemplar: 0.25 and an empty curve (evaluates to 0).
    let h = fx_with(None);
    h.m.init();
    let s = h.m.snapshot();
    assert_eq!(s.distress_threshold, 0.25);
    assert!(s.escape_curve.is_empty());
    assert_eq!(escape_probability(&s.escape_curve, 0.0), 0.0);
    // Read once per object: a later Init does not re-read.
    *f.h.tuning.borrow_mut() = None;
    f.m.shutdown();
    f.m.init();
    assert_eq!(f.m.snapshot().distress_threshold, 25.0);
}

// ================================================================ §12.3

#[test]
fn pk12_03_insert() {
    let f = ready();
    let ty = 0x0400_1000;
    f.h.park(0x20, ty, 10, &[GROUP_PARK]);
    f.h.park(0x21, ty, 10, &[GROUP_PARK]);
    f.m.do_message(&ins(0x20));
    let d = f.h.dept();
    let ev = f.h.take();
    let lt = f.h.line_item(d, ty).unwrap();
    assert_eq!(
        ev,
        vec![
            Ev::AddRef(0x20),
            Ev::ChangeLine(0x20, 0, ty),
            Ev::CreateLine(d, ty, false),
            Ev::DisplayFlag(lt, 2, true),
        ]
    );
    let l = f.h.line(ty).unwrap();
    assert_eq!((l.full_expenses, l.secondary), (10, 1));
    f.m.do_message(&ins(0x21));
    let l = f.h.line(ty).unwrap();
    assert_eq!((l.full_expenses, l.secondary), (20, 2));
    assert_eq!(f.m.snapshot().parks, vec![Some(0x20), Some(0x21)]);
    // A preview occupant (property present, any value) changes nothing.
    f.h.park(0x22, ty, 10, &[GROUP_PARK]);
    f.h.occupants
        .borrow_mut()
        .get_mut(&0x22)
        .unwrap()
        .props
        .push(PROP_PREVIEW_OCCUPANT);
    f.h.take();
    f.m.do_message(&ins(0x22));
    f.m.do_message(&rem(0x22));
    assert!(f.h.take().is_empty());
    assert_eq!(f.h.line(ty).unwrap().secondary, 2);
}

/// §4.6: three grass parks → one line, 15 and 3; removing all three → 0 and
/// 0, the line stays.
#[test]
fn worked_example_grass_parks() {
    let f = ready();
    let ty = 0x0500_0001;
    for o in [0x30, 0x31, 0x32] {
        f.h.park(o, ty, 5, &[GROUP_PARK]);
        f.m.do_message(&ins(o));
    }
    let l = f.h.line(ty).unwrap();
    assert_eq!((l.full_expenses, l.secondary), (15, 3));
    assert_eq!(f.h.lines.borrow().len(), 1);
    for o in [0x30, 0x31, 0x32] {
        f.m.do_message(&rem(o));
    }
    let l = f.h.line(ty).unwrap();
    assert_eq!((l.full_expenses, l.secondary), (0, 0));
}

#[test]
fn insert_edge_cases() {
    // Not a building occupant (QI fails): return before the zoo test.
    let f = ready();
    f.h.park(0x40, 0x0600_0000, 10, &[GROUP_PARK, GROUP_ZOO]);
    f.h.occupants.borrow_mut().get_mut(&0x40).unwrap().building = false;
    f.m.do_message(&ins(0x40));
    let s = f.m.snapshot();
    assert_eq!(s.parks, vec![Some(0x40)]);
    assert!(s.zoos.is_empty(), "[compat] the zoo test is skipped");
    // No budget item: go on to the zoo test.
    let f = ready();
    f.h.park(0x41, 0x0600_0000, 10, &[GROUP_PARK, GROUP_ZOO]);
    f.h.occupants.borrow_mut().get_mut(&0x41).unwrap().item = None;
    f.m.do_message(&ins(0x41));
    assert_eq!(f.m.snapshot().zoos, vec![Some(0x41)]);
    // A department that does not exist: steps 5–7 skipped [choice].
    let f = ready();
    f.h.park(0x42, 0x0600_0000, 10, &[GROUP_PARK]);
    f.h.occupants
        .borrow_mut()
        .get_mut(&0x42)
        .unwrap()
        .item
        .as_mut()
        .unwrap()
        .department_id = 0x1;
    f.h.take();
    f.m.do_message(&ins(0x42));
    assert_eq!(f.h.take(), vec![Ev::AddRef(0x42)]);
    // Building type 0: the line change happens, nothing is charged.
    let f = ready();
    f.h.park(0x43, 0, 10, &[GROUP_PARK]);
    f.h.take();
    f.m.do_message(&ins(0x43));
    assert_eq!(f.h.take(), vec![Ev::AddRef(0x43), Ev::ChangeLine(0x43, 0, 0)]);
    // A non-building occupant: nothing.
    let f = ready();
    f.h.park(0x44, 1, 10, &[GROUP_PARK]);
    f.h.occupants.borrow_mut().get_mut(&0x44).unwrap().occ_type = 0xC772_BF98;
    f.h.take();
    f.m.do_message(&ins(0x44));
    assert!(f.h.take().is_empty());
}

// ================================================================ §12.4

#[test]
fn pk12_04_remove() {
    let f = ready();
    let ty = 0x0400_1000;
    f.h.park(0x20, ty, 10, &[GROUP_PARK]);
    f.h.park(0x21, ty, 10, &[GROUP_PARK]);
    f.m.do_message(&ins(0x20));
    f.m.do_message(&ins(0x21));
    // A duplicate entry of 0x20.
    f.m.do_message(&ins(0x20));
    assert_eq!(f.m.snapshot().parks, vec![Some(0x20), Some(0x21), Some(0x20)]);
    f.h.take();
    f.m.do_message(&rem(0x20));
    assert_eq!(f.m.snapshot().parks, vec![Some(0x21)], "every duplicate removed");
    assert_eq!(f.h.take(), vec![Ev::Release(0x20)], "released once");
    f.m.do_message(&rem(0x21));
    let l = f.h.line(ty).unwrap();
    assert_eq!((l.full_expenses, l.secondary), (10, 1), "the third insert charged too");
    f.m.do_message(&rem(0x20));
    let l = f.h.line(ty).unwrap();
    assert_eq!((l.full_expenses, l.secondary), (0, 0));
    assert!(f.h.line(ty).is_some(), "the line item stays");
    // [quirk] The duplicate's reference leaked at the first removal; the
    // second removal (0x20 no longer listed) still releases once.
    assert_eq!(f.h.refs(0x20), 0);
    // Zoo removal.
    f.h.park(0x50, 0x0339_0000, 10, &[GROUP_PARK, GROUP_ZOO]);
    f.m.do_message(&ins(0x50));
    assert_eq!(f.m.snapshot().zoos, vec![Some(0x50)]);
    f.h.take();
    f.m.do_message(&rem(0x50));
    let s = f.m.snapshot();
    assert!(s.zoos.is_empty() && s.parks.is_empty());
    let rel = f.h.take().iter().filter(|e| **e == Ev::Release(0x50)).count();
    assert_eq!(rel, 2, "once per list");
}

// ================================================================ §12.5

#[test]
fn pk12_05_distress_latch() {
    let f = ready();
    let la = f.h.park(0x60, 1, 10, &[GROUP_PARK]);
    let lb = f.h.park(0x61, 1, 10, &[GROUP_PARK]);
    f.m.do_message(&ins(0x60));
    f.m.do_message(&ins(0x61));
    f.m.with_state(|s| s.parks.insert(1, None));
    f.h.take();
    let conds = || {
        let c = f.h.conditions.borrow();
        (c[&la], c[&lb])
    };
    f.h.set_funding(0.25);
    f.m.do_message(&month());
    assert_eq!(conds(), (2, 2));
    assert!(f.m.snapshot().underfunded);
    assert_eq!(f.h.take(), vec![Ev::SetCondition(la, 2), Ev::SetCondition(lb, 2)]);
    f.h.set_funding(0.2);
    f.m.do_message(&month());
    assert!(f.h.take().is_empty());
    f.h.set_funding(0.3);
    f.m.do_message(&month());
    assert_eq!(conds(), (0, 0));
    assert!(!f.m.snapshot().underfunded);
    // NaN: nothing in either direction.
    f.h.set_funding(f32::NAN);
    f.m.do_message(&month());
    assert!(!f.m.snapshot().underfunded);
    f.m.with_state(|s| s.underfunded = true);
    f.m.do_message(&month());
    assert!(f.m.snapshot().underfunded);
    f.h.take();
    // §4.6: 20 % → 2, then 30 % → 0 with the code-default threshold.
    f.m.with_state(|s| s.underfunded = false);
    f.h.set_funding(0.2);
    f.m.monthly();
    assert_eq!(conds(), (2, 2));
    f.h.set_funding(0.3);
    f.m.monthly();
    assert_eq!(conds(), (0, 0));
}

// ================================================================ §12.6

#[test]
fn pk12_06_shipped_threshold() {
    let f = fx_with(Some(shipped_tuning()));
    f.m.init();
    let la = f.h.park(0x60, 1, 10, &[GROUP_PARK]);
    f.m.do_message(&ins(0x60));
    f.h.set_funding(1.2);
    f.m.monthly();
    assert_eq!(f.h.conditions.borrow()[&la], 2);
    // Funding is at most 1.2 (120 %), never above 25.0.
    for f_ in [1.2, 0.5, 1.0, 0.0] {
        f.h.set_funding(f_);
        f.m.monthly();
        assert_eq!(f.h.conditions.borrow()[&la], 2, "never restored");
    }
    // The latch is not saved: a new session (Shutdown keeps it, a fresh
    // object starts clear) runs it down again.
    assert!(f.m.snapshot().underfunded);
}

// ================================================================ §12.7

#[test]
fn pk12_07_escape_draw() {
    let curve = ResponseCurve::from_points(&SHIPPED_ESCAPE_CURVE);
    let p = escape_probability(&curve, 0.5);
    assert_eq!(p, f64::from(7.0f32 * 0.01f32));
    assert_eq!(p, 0.070_000_000_298_023_22);
    // §4.6's table.
    assert_eq!(escape_probability(&curve, 0.0), f64::from(0.399_999_98f32));
    assert_eq!(escape_probability(&curve, 0.25), f64::from(0.099_999_994f32));
    assert_eq!(escape_probability(&curve, 1.0), f64::from(0.01f32));
    assert_eq!(escape_probability(&curve, 1.2), f64::from(0.01f32));

    let f = fx_with(Some(shipped_tuning()));
    f.m.init();
    f.h.park(0x70, 1, 10, &[GROUP_PARK]);
    f.h.park(0x71, 0x0339_0000, 10, &[GROUP_PARK, GROUP_ZOO]);
    f.m.do_message(&ins(0x70));
    f.m.do_message(&ins(0x71));
    f.h.set_funding(0.5);
    f.h.take();
    f.doubles.borrow_mut().push_back(0.07);
    f.m.monthly();
    assert_eq!(*f.log.borrow(), vec!["double", "u32(1)"]);
    let ev: Vec<Ev> =
        f.h.take()
            .into_iter()
            .filter(|e| !matches!(e, Ev::SetCondition(..)))
            .collect();
    assert_eq!(
        ev,
        vec![Ev::ZooEscape(0xAA, 0x71), Ev::Controllers("zoomanji".into(), 0x71)]
    );
    f.log.borrow_mut().clear();
    f.doubles.borrow_mut().push_back(0.070_000_1);
    f.m.monthly();
    assert_eq!(*f.log.borrow(), vec!["double"]);
    assert!(!f.h.take().iter().any(|e| matches!(e, Ev::ZooEscape(..))));
    // No zoos → no draw.
    f.log.borrow_mut().clear();
    f.m.with_state(|s| s.zoos.clear());
    f.m.monthly();
    assert!(f.log.borrow().is_empty());
    // No parks → no draw (no zoo check either).
    f.m.with_state(|s| {
        s.parks.clear();
        s.zoos = vec![Some(0x71)];
    });
    f.m.monthly();
    assert!(f.log.borrow().is_empty());
    // A null zoo entry: the escape stops after the index draw.
    f.m.with_state(|s| {
        s.parks = vec![Some(0x70)];
        s.zoos = vec![None];
    });
    f.doubles.borrow_mut().push_back(0.0);
    f.h.take();
    f.m.monthly();
    assert_eq!(*f.log.borrow(), vec!["double", "u32(1)"]);
    assert!(!f.h.take().iter().any(|e| matches!(e, Ev::ZooEscape(..))));
}

#[test]
fn default_generator_is_seeded_from_the_session() {
    let h = Host::new();
    let m = ParkManager::new(h.clone(), 1, 0x1234);
    let mut want = crate::rng::RzRandomSuper::new(
        crate::rng::derive_seed(0x1234, 0xAA3F_E6BC),
        &crate::rng::FixedSeeds {
            wall_clock: 0,
            process_clock: 0,
        },
    );
    m.init();
    m.with_state(|s| {
        s.parks = vec![Some(1)];
        s.zoos = vec![Some(1)];
    });
    let d = h.dept();
    h.funding.borrow_mut().insert(d, 0.0);
    // With an empty curve p = 0: one draw, no escape; the generator moved.
    m.monthly();
    let _ = want.uniform_f64();
    assert!(!h.take().iter().any(|e| matches!(e, Ev::ZooEscape(..))));
}

// ================================================================ §12.8

struct Resolver;

impl RefResolver for Resolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        Some(RefValue::Host {
            token: u64::from(object_id),
            class_id,
        })
    }
}

fn record(m: &ParkManager) -> Vec<u8> {
    let mut w = ByteWriter::with_registry(Rc::new(RefCell::new(SaveRegistry::new())));
    assert!(m.write(&mut w));
    w.into_bytes()
}

fn load(m: &ParkManager, b: &[u8]) -> bool {
    let mut r = Resolver;
    m.read(&mut ByteReader::with_resolver(b, &mut r))
}

#[test]
fn pk12_08_save() {
    let f = fx_with(None);
    assert_eq!(record(&f.m), vec![2, 0, 0, 0, 0, 0, 0, 0, 0, 0], "the 413-save shape");
    f.m.with_state(|s| {
        s.parks = vec![Some(0x10), Some(0x11), None];
        s.zoos = vec![Some(0x11)];
    });
    let b = record(&f.m);
    assert_eq!(b.len(), 2 + 4 + 8 + 8 + 4 + 4 + 8);
    let g = fx_with(None);
    assert!(load(&g.m, &b));
    assert_eq!(record(&g.m), b);
    assert_eq!(g.m.snapshot().parks, vec![Some(1), Some(2), None]);
    assert_eq!(g.m.snapshot().zoos, vec![Some(2)]);
    for v in [1u8, 3] {
        let mut x = b.clone();
        x[0] = v;
        let h = fx_with(None);
        assert!(!load(&h.m, &x), "version {v}");
        assert!(h.m.snapshot().parks.is_empty());
    }
    // Truncated: unchanged, references given back.
    let h = fx_with(None);
    assert!(!load(&h.m, &b[..b.len() - 3]));
    assert!(h.m.snapshot().parks.is_empty());
    assert_eq!(h.h.refs(1), -1);
    // Unresolvable → null; ≥ 10,000,000 reads nothing; old entries are not
    // released.
    let mut rec = vec![2, 0];
    rec.extend(1u32.to_le_bytes());
    rec.extend(0xDEADu32.to_le_bytes());
    rec.extend(OCCUPANT_CLASS.to_le_bytes());
    rec.extend(10_000_000u32.to_le_bytes());
    let k = fx_with(None);
    k.m.with_state(|s| s.parks = vec![Some(0x99)]);
    assert!(load(&k.m, &rec));
    assert_eq!(k.m.snapshot().parks, vec![None]);
    assert!(k.m.snapshot().zoos.is_empty());
    assert_eq!(k.h.refs(0x99), 0);
}

/// §7, synthetic: the corpus' shapes (no parks; 1, 3, 6, 9 parks, no zoo).
/// [not the corpus] The 421 saves live in the user's documents folder,
/// which this lane may not read.
#[test]
fn pk12_08_corpus_shapes() {
    for n in [0u32, 1, 3, 6, 9] {
        let mut b = vec![2, 0];
        b.extend(n.to_le_bytes());
        for i in 0..n {
            b.extend((1 + i).to_le_bytes());
            b.extend(OCCUPANT_CLASS.to_le_bytes());
        }
        b.extend(0u32.to_le_bytes());
        let f = fx_with(None);
        assert!(load(&f.m, &b));
        assert_eq!(record(&f.m), b, "{n} parks");
        assert_eq!(f.m.snapshot().parks.len() as u32, n);
    }
}
