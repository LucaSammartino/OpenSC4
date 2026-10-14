// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests of the power simulator against a small fake host.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use super::*;
use crate::grid::{CityDims, CoreGrids};
use crate::property::PropertyHolder;
use crate::stream::{ByteReader, ByteWriter, RefResolver, RefValue, RegisteredObject, SaveObject, SaveRegistry};
use crate::variant::{Elem, Variant};

const OCC_CLASS: u32 = 0x1234_0001;
const DEPT: u64 = 900;
const LINE_ITEM: u64 = 901;

#[derive(Clone, Default)]
struct Occ {
    t: u32,
    props: PropertyHolder,
    cells: CellRect,
    budget: Option<BudgetItem>,
    in_manager: bool,
}

struct Host {
    cells: Cell<(i32, i32)>,
    factor: Cell<f32>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    has_department: Cell<bool>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    posts: RefCell<Vec<Posted>>,
    history: RefCell<Vec<(u32, i32)>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    distressed: RefCell<BTreeSet<u64>>,
    expenses: Cell<i64>,
    agents: Cell<u32>,
    /// `GetDepartmentBudget` calls.
    lookups: Cell<u32>,
}

impl Host {
    fn new(cells: i32) -> Rc<Host> {
        Rc::new(Host {
            cells: Cell::new((cells, cells)),
            factor: Cell::new(1.0),
            tuning: RefCell::new(Some(Rc::new(shipped_tuning()))),
            has_department: Cell::new(false),
            occs: RefCell::default(),
            posts: RefCell::default(),
            history: RefCell::default(),
            refs: RefCell::default(),
            distressed: RefCell::default(),
            expenses: Cell::new(-1),
            agents: Cell::new(0),
            lookups: Cell::new(0),
        })
    }

    fn add(&self, id: u64, o: Occ) {
        self.occs.borrow_mut().insert(id, Occ { in_manager: true, ..o });
    }

    fn building(&self, id: u64, cells: CellRect, props: &[(u32, Variant)], budget: Option<BudgetItem>) {
        let mut h = PropertyHolder::new();
        for (k, v) in props {
            h.add(*k, v.clone());
        }
        self.add(
            id,
            Occ {
                t: occupant_type::BUILDING,
                props: h,
                cells,
                budget,
                in_manager: true,
            },
        );
    }

    fn pole(&self, id: u64, x: i32, z: i32) {
        let mut h = PropertyHolder::new();
        h.add(prop::POLE_COST, Variant::Scalar(Elem::I64(1)));
        self.add(
            id,
            Occ {
                t: occupant_type::POWER_POLE,
                props: h,
                cells: CellRect::new(x, z, x, z),
                ..Default::default()
            },
        );
    }

    fn refs_of(&self, t: u64) -> i64 {
        self.refs.borrow().get(&t).copied().unwrap_or(0)
    }
}

fn u32v(v: u32) -> Variant {
    Variant::Scalar(Elem::U32(v))
}

fn i32v(v: i32) -> Variant {
    Variant::Scalar(Elem::I32(v))
}

fn f32v(v: f32) -> Variant {
    Variant::Scalar(Elem::F32(v))
}

fn curve(points: &[f32]) -> Variant {
    Variant::array(
        crate::variant::mem::FLOAT32,
        points.iter().map(|&p| Elem::F32(p)).collect(),
    )
}

fn funded() -> Option<BudgetItem> {
    Some(BudgetItem {
        department: budget::DEPARTMENT,
        line: 7,
        cost: 250,
        index: 0,
        local_funding: 1.0,
    })
}

/// A plant of `capacity` with a flat condition curve at 1.0.
fn plant(h: &Host, id: u64, cells: CellRect, capacity: u32, plant_type: i32) {
    h.building(
        id,
        cells,
        &[
            (prop::CAPACITY, u32v(capacity)),
            (prop::PLANT_TYPE, i32v(plant_type)),
            (prop::CONDITION_CURVE, curve(&[0.0, 1.0, 1.0, 1.0])),
        ],
        funded(),
    );
}

impl GridProvider<u8> for Host {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        let (x, z) = self.cells.get();
        CoreGrids {
            city: CityDims {
                cells_x: x,
                cells_z: z,
                cell_width_x: 16.0,
                cell_width_z: 16.0,
            },
        }
        .create_grid(instance, tract_size)
    }
}

impl PowerHost for Host {
    fn cell_counts(&self) -> (i32, i32) {
        self.cells.get()
    }
    fn cell_widths(&self) -> (f32, f32) {
        (16.0, 16.0)
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn cumulative_factor(&self, _id: u32, _index: u32) -> f32 {
        self.factor.get()
    }
    fn sim_time(&self) -> u32 {
        0
    }
    fn acquire_services(&self) {}
    fn release_services(&self) {}
    fn add_agent(&self, _level: u32, _name: &str, _flags: u32) {
        self.agents.set(self.agents.get() + 1);
    }
    fn remove_agent(&self) {}
    fn subscribe(&self, _msg_types: &[u32]) {}
    fn unsubscribe(&self, _msg_types: &[u32]) {}
    fn post(&self, m: &Posted) {
        self.posts.borrow_mut().push(*m);
    }
    fn record_history(&self, series: u32, value: i32) {
        self.history.borrow_mut().push((series, value));
    }
    fn set_history_queue_length(&self, _series: u32, _length: u32) {}
    fn add_ref(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) += 1;
    }
    fn release(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) -= 1;
    }
    fn object_ref(&self, token: u64) -> RefValue {
        RefValue::Host {
            token,
            class_id: OCC_CLASS,
        }
    }
    fn resolve_ref(&self, r: &RefValue, _iid: u32) -> Option<u64> {
        match r {
            RefValue::Host { token, .. } => Some(*token),
            _ => None,
        }
    }
    fn occupant_type(&self, occ: u64) -> u32 {
        self.occs.borrow().get(&occ).map_or(0, |o| o.t)
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant> {
        self.occs.borrow().get(&occ)?.props.get(id).cloned()
    }
    fn set_occupant_property(&self, occ: u64, id: u32, value: Variant) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.props.add(id, value);
        }
    }
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        let (a, b) = self.occupant_bounding_box(occ);
        [(a[0] + b[0]) * 0.5, 0.0, (a[2] + b[2]) * 0.5]
    }
    fn occupant_bounding_box(&self, occ: u64) -> ([f32; 3], [f32; 3]) {
        let c = self.occupant_bounding_cells(occ);
        (
            [c.x1 as f32 * 16.0, 0.0, c.z1 as f32 * 16.0],
            [(c.x2 as f32 + 1.0) * 16.0, 0.0, (c.z2 as f32 + 1.0) * 16.0],
        )
    }
    fn occupant_bounding_cells(&self, occ: u64) -> CellRect {
        self.occs.borrow().get(&occ).map(|o| o.cells).unwrap_or_default()
    }
    fn occupant_in_manager(&self, occ: u64) -> bool {
        self.occs.borrow().get(&occ).is_some_and(|o| o.in_manager)
    }
    fn building_type(&self, occ: u64) -> Option<u32> {
        (self.occupant_type(occ) == occupant_type::BUILDING).then_some(0x1234)
    }
    fn power_pole(&self, occ: u64) -> Option<PoleInfo> {
        let o = self.occs.borrow().get(&occ)?.clone();
        (o.t == occupant_type::POWER_POLE).then_some(PoleInfo {
            ignored: false,
            cell: (o.cells.x1, o.cells.z1),
        })
    }
    fn break_all_connections(&self, _pole: u64, _flag: bool) {}
    fn pole_is_distressed(&self, pole: u64) -> bool {
        self.distressed.borrow().contains(&pole)
    }
    fn set_pole_distressed(&self, pole: u64, on: bool) {
        if on {
            self.distressed.borrow_mut().insert(pole);
        } else {
            self.distressed.borrow_mut().remove(&pole);
        }
    }
    fn power_line(&self, _occ: u64) -> Option<LineInfo> {
        None
    }
    fn line_connected_poles(&self, _line: u64) -> Option<(u64, u64)> {
        None
    }
    fn remove_pole_connection(&self, _first: u64, _second: u64) {}
    fn network_occupant_cell(&self, _occ: u64) -> Option<(i32, i32)> {
        None
    }
    fn has_network_flag(&self, _occ: u64, _flag: u32) -> bool {
        false
    }
    fn network_flags(&self, _occ: u64) -> u32 {
        0
    }
    fn occupants_in_cells(&self, rect: CellRect) -> Vec<u64> {
        self.occs
            .borrow()
            .iter()
            .filter(|(_, o)| {
                o.in_manager
                    && o.cells.x1 <= rect.x2
                    && o.cells.x2 >= rect.x1
                    && o.cells.z1 <= rect.z2
                    && o.cells.z2 >= rect.z1
            })
            .map(|(&k, _)| k)
            .collect()
    }
    fn remove_occupant(&self, _occ: u64) {}
    fn zone_type(&self, _x: i32, _z: i32) -> u32 {
        0
    }
    fn power_line_tool_zone_change(&self, _rect: CellRect, _data3: u32) {}
    fn lot_at(&self, _x: i32, _z: i32) -> Option<u64> {
        None
    }
    fn occupant_lot(&self, _occ: u64) -> Option<u64> {
        None
    }
    fn as_lot(&self, _token: u64) -> Option<u64> {
        None
    }
    fn lot_bounding_rect(&self, _lot: u64) -> CellRect {
        CellRect::default()
    }
    fn lot_state(&self, _lot: u64) -> u32 {
        0
    }
    fn lot_needs_power(&self, _lot: u64) -> bool {
        false
    }
    fn lot_has_power(&self, _lot: u64) -> bool {
        false
    }
    fn set_lot_has_power(&self, _lot: u64, _on: bool) {}
    fn set_lot_condition(&self, _lot: u64, _condition: u32) {}
    fn demolish_lots(&self, _lots: &[u64], _flags: u32) {}
    fn add_radiation(&self, _position: [f32; 3], _strength: f32, _radius: f32) {}
    fn current_view_input_control(&self) -> Option<u32> {
        None
    }
    fn remove_current_view_input_control(&self) {}
    fn find_deals(&self, _deal_type: u32) -> Vec<Deal> {
        Vec::new()
    }
    fn deal_amount_used(&self, _deal: u64) -> i32 {
        0
    }
    fn set_deal_amount_used(&self, _deal: u64, _used: i32) {}
    fn set_deal_flags(&self, _deal: u64, _flags: u32, _on: bool) {}
    fn deal_type(&self, _token: u64) -> Option<u32> {
        None
    }
    fn mark_deals_dirty(&self) {}
    fn connections_of_type(&self, _connection_type: u32) -> Vec<u64> {
        Vec::new()
    }
    fn connections_to_city(&self, _connection_type: u32, _city: u64) -> Vec<u64> {
        Vec::new()
    }
    fn find_unvisited_connection(&self, _connection_type: u32, _city: u64) -> Option<u64> {
        None
    }
    fn connection_on_other_side(&self, _conn: u64) -> Option<u64> {
        None
    }
    fn connection_cell(&self, _conn: u64) -> (i32, i32) {
        (0, 0)
    }
    fn set_connection_visited(&self, _conn: u64, _on: bool) {}
    fn connection_is_active(&self, _conn: u64) -> bool {
        false
    }
    fn set_connection_active(&self, _conn: u64, _on: bool) {}
    fn department(&self, id: u32) -> Option<u64> {
        self.lookups.set(self.lookups.get() + 1);
        (id == budget::DEPARTMENT && self.has_department.get()).then_some(DEPT)
    }
    fn create_department(&self, _id: u32, _group: u32) -> Option<u64> {
        self.has_department.set(true);
        Some(DEPT)
    }
    fn set_department_name(&self, _dept: u64, _group: u32, _instance: u32) {}
    fn set_department_max_funding(&self, _dept: u64, _fraction: f32) {}
    fn department_funding(&self, _dept: u64) -> f32 {
        1.0
    }
    fn budget_funding(&self, _dept: u64) -> i64 {
        0
    }
    fn create_line_item(&self, _dept: u64, _line: u32, _locally_funded: bool) -> Option<u64> {
        Some(LINE_ITEM)
    }
    fn create_line_item_for_building_type(&self, _dept: u64, _line: u32, _flag: bool) -> Option<u64> {
        Some(LINE_ITEM + 1)
    }
    fn line_item(&self, _dept: u64, line: u32) -> Option<u64> {
        (line == budget::LINE_POLES).then_some(LINE_ITEM)
    }
    fn set_line_item_name(&self, _item: u64, _group: u32, _instance: u32) {}
    fn set_line_item_display_flag(&self, _item: u64, _flag: u32, _on: bool) {}
    fn line_item_secondary_info(&self, _item: u64) -> i64 {
        0
    }
    fn set_line_item_secondary_info(&self, _item: u64, _value: i64) {}
    fn set_line_item_full_expenses(&self, item: u64, value: i64) {
        if item == LINE_ITEM {
            self.expenses.set(value);
        }
    }
    fn budget_item_for_purpose(&self, occ: u64, purpose: u32) -> Option<BudgetItem> {
        if purpose != budget::PURPOSE {
            return None;
        }
        self.occs.borrow().get(&occ)?.budget
    }
    fn change_budget_item_line(&self, _occ: u64, _index: u32, _line: u32) {}
    fn change_budget_item_cost(&self, occ: u64, _index: u32, cost: i64) {
        if let Some(Some(b)) = self.occs.borrow_mut().get_mut(&occ).map(|o| o.budget.as_mut()) {
            b.cost = cost;
        }
    }
    fn add_locally_funded_object(&self, _dept: u64, _occ: u64, _purpose: u32) {}
    fn remove_locally_funded_object(&self, _dept: u64, _occ: u64, _purpose: u32) {}
    fn is_locally_funded_object(&self, _dept: u64, _occ: u64, _purpose: u32) -> bool {
        true
    }
    fn set_local_funding_percent(&self, _dept: u64, _occ: u64, _fraction: f32, _purpose: u32) -> bool {
        true
    }
    fn set_local_full_funding(&self, _dept: u64, _occ: u64, _amount: i64, _purpose: u32) {}
}

fn sim(h: &Rc<Host>) -> Rc<PowerSim> {
    let s = PowerSim::new(h.clone() as Rc<dyn PowerHost>, 1);
    assert!(s.init());
    s.do_message(&message(msg::CONNECTIONS_READY, 0));
    s
}

fn message(msg_type: u32, void1: u64) -> PowerMessage {
    PowerMessage {
        msg_type,
        void1,
        ..Default::default()
    }
}

/// `DoIdle` until the state machine is back at 4 after leaving it.
fn cycle(s: &PowerSim) {
    s.update_state(|st| st.recompute = true);
    let mut left = false;
    for _ in 0..16 {
        s.do_idle();
        let st = s.state().state;
        if st != state::IDLE {
            left = true;
        } else if left {
            return;
        }
    }
    panic!("the cycle did not finish");
}

// ---- arithmetic and identity ------------------------------------------------------------------------

#[test]
fn ftol_matches_ftol() {
    assert_eq!(ftol32(-2.75), -2);
    assert_eq!(ftol32(4799.9995), 4799);
    // `_ftol` keeps the low 32 bits of an int64 (foundations §18.1): NaN
    // and ±∞ give 0, values above int32 wrap.
    assert_eq!(ftol32(f32::NAN), 0);
    assert_eq!(ftol32(f32::INFINITY), 0);
    assert_eq!(ftol32(3.0e9), 3_000_000_000u32 as i32);
    assert_eq!(ftol32(-3.0e9), (-3_000_000_000i64) as i32);
    assert_eq!(ftol32(0.0 * f32::NAN), 0);
    assert_eq!(ftol64(f32::NAN), i64::MIN);
    assert_eq!(ftol64(f32::INFINITY), i64::MIN);
    assert_eq!(ftol64(3.0e9), 3_000_000_000);
    assert_eq!(ftol64(-1100.0), -1100);
}

#[test]
fn twenty_four_bit_products() {
    // §15 tests 1, 5, 13, 15.
    let e = 80.0f32 * 0.01f32;
    assert_eq!(e, 0.799_999_95);
    assert_eq!(ftol32(mul_i(6000.0, e)), 4799);
    assert_eq!(ftol32(mul_i(250.0, 1.05)), 262);
    assert_eq!(ftol32(mul_i(250.0, 1.3)), 325);
    assert_eq!(ftol32(mul_i(100.0, 0.02)), 2);
    assert_eq!(ftol32(mul_i(1000.0, 0.02)), 20);
    assert_eq!(ftol32(mul_i(50.0, 0.02)), 1);
    assert_eq!(ftol32(div_i(1100.0, 0.9)), 1222);
    assert_eq!(ftol32(mul_i(667.0, 0.9)), 600);
    assert_eq!(ftol32(mul_i(555.0, 0.9)), 499);
    // Test 5 (PW1): 500 x 0.9f = 449.99998808 rounds to 450.0.
    assert_eq!(ftol32(mul_i(500.0, 0.9)), 450);
    assert_eq!(div_ii(2.0, 3.0), 2.0f32 / 3.0f32);
    // §8: 120 x 0.01f = 1.1999999 (0x3F999999), not 1.2 (0x3F99999A).
    assert_eq!(super::tuning::percent(120.0).to_bits(), 0x3F99_9999);
    assert_eq!(super::tuning::percent(20.0).to_bits(), 0x3E4C_CCCC);
    assert_eq!(super::tuning::percent(60.0).to_bits(), 0x3F19_9999);
}

#[test]
fn identity_and_slots() {
    assert_eq!(query_interface(IID_POWER_SIMULATOR), Some(PowerInterface::Primary));
    assert_eq!(query_interface(1), Some(PowerInterface::Primary));
    assert_eq!(
        query_interface(IID_MESSAGE_TARGET2),
        Some(PowerInterface::MessageTarget)
    );
    assert_eq!(query_interface(IID_SERIALIZABLE), Some(PowerInterface::Serializable));
    assert_eq!(query_interface(0xDEAD_BEEF), None);
    for (i, e) in SLOT_TABLE.iter().enumerate() {
        assert_eq!(e.0, i as u32);
    }
    let pops = |slot: usize| SLOT_TABLE[slot].2;
    assert_eq!(
        (pops(45), pops(46), pops(16), pops(17), pops(23), pops(24)),
        (8, 8, 8, 0, 4, 0)
    );
    let h = Host::new(16);
    let s = PowerSim::new(h.clone() as Rc<dyn PowerHost>, 1);
    assert_eq!(s.class_id(), CLSID_POWER_SIMULATOR);
}

#[test]
fn tuning_defaults_and_shipped() {
    let mut t = PowerTuning::default();
    assert_eq!((t.radius, t.strike_duration), (2, 10));
    assert_eq!(t.strike_efficiency, 0.5);
    assert_eq!(t.funding_efficiency.eval(90.0), 90.0);
    t.read(Some(&shipped_tuning()));
    assert_eq!(t.strike_efficiency, 20.0 * 0.01f32);
    assert_eq!(t.strike_duration, 12);
    assert_eq!(t.funding_efficiency.eval(90.0), 80.0);
    assert_eq!(t.usage_decay.eval(0.5), 0.45);
    // A read without an exemplar restores the code defaults.
    t.read(None);
    assert_eq!(t.strike_efficiency, 0.5);
    assert_eq!(t.funding_efficiency.eval(90.0), 90.0);
}

// ---- lifecycle and the cycle ------------------------------------------------------------------------

#[test]
fn init_creates_department_grid_and_state() {
    let h = Host::new(16);
    let s = PowerSim::new(h.clone() as Rc<dyn PowerHost>, 1);
    s.update_state(|st| st.recompute = true);
    s.do_idle();
    assert_eq!(s.state().state, 0, "constructed state, DoIdle disabled");
    assert!(s.init());
    assert!(s.init());
    assert_eq!(h.agents.get(), 3);
    let st = s.state();
    assert_eq!(
        (st.state, st.recompute, st.department),
        (state::IDLE, false, Some(DEPT))
    );
    assert_eq!(st.consumption_map.len(), 16);
    assert_eq!(st.mark.words().len(), 8);
    drop(st);
    assert!(s.powered_grid().is_some());
    assert_eq!(h.refs_of(DEPT), 1);
    // Before ConnectionsReady nothing runs.
    s.update_state(|st| st.recompute = true);
    s.do_idle();
    assert_eq!(s.state().state, state::IDLE);
    assert!(s.shutdown());
    assert_eq!(h.refs_of(DEPT), 0);
    assert!(s.powered_grid().is_none());
    assert!(s.shutdown());
}

#[test]
fn one_network_powers_its_consumer() {
    let h = Host::new(32);
    plant(&h, 10, CellRect::new(8, 0, 9, 1), 1000, 1);
    h.building(11, CellRect::new(12, 0, 13, 1), &[(prop::REQUIREMENT, u32v(500))], None);
    let s = sim(&h);
    s.occupant_inserted(10);
    s.occupant_inserted(11);
    assert_eq!(h.refs_of(10), 1);
    cycle(&s);
    let st = s.state();
    assert_eq!(
        (st.last_production, st.last_consumption, st.last_demand),
        (1000, 500, 500)
    );
    assert_eq!((st.powered, st.unpowered, st.consumers), (1, 0, 1));
    assert_eq!(st.ring[st.ring_row()][1], 500);
    drop(st);
    assert!(s.is_powered(13, 1) && s.is_wired(13, 1));
    assert!(!s.is_powered(16, 1));
    let g = s.powered_grid().unwrap();
    assert_eq!((g.get(13, 1), g.get(16, 1)), (2, 0));
    assert_eq!(s.occupant_power_produced(10), Some((1000, 500)));
    assert_eq!(h.expenses.get(), 0);
    // Block (1, 0) changed (cells 6..15 lie in blocks 0 and 1).
    let posts = h.posts.borrow();
    let blocks: Vec<[u32; 4]> = posts
        .iter()
        .filter(|p| p.msg_type == msg::POWER_BLOCK_CHANGED)
        .map(|p| p.data)
        .collect();
    assert_eq!(blocks, vec![[0, 0, 7, 7], [8, 0, 15, 7]]);
    assert_eq!(posts.iter().filter(|p| p.msg_type == msg::POWER_CHANGED).count(), 1);
}

#[test]
fn pole_decay_shuffles_and_repairs() {
    let h = Host::new(32);
    for i in 0..100 {
        h.pole(100 + i, (i % 30) as i32, (i / 30) as i32);
    }
    let s = sim(&h);
    for i in 0..100 {
        s.occupant_inserted(100 + i);
    }
    assert_eq!(s.state().cost_base, 100);
    let before = s.crt();
    s.apply_pole_funding(0.0);
    let mut probe = before;
    for _ in 0..99 {
        probe.rand();
    }
    assert_eq!(probe, s.crt());
    assert_eq!(s.state().broken_count, 2);
    assert_eq!(h.distressed.borrow().len(), 2);
    s.apply_pole_funding(1.0);
    assert_eq!(s.state().broken_count, 0);
    assert!(h.distressed.borrow().is_empty());
    // A NaN funding clamps the fraction to 0 rather than poisoning it.
    s.apply_pole_funding(f32::NAN);
    assert_eq!(s.state().broken_fraction, 0.0);
}

// ---- the block probe (tooling, the power shadow) ---------------------------------------------------

/// One plant of 1000 (1100 available) and consumers of 600 in blocks 1, 2
/// and 3 of one network: only one consumer block can be powered, and which
/// one depends on step 6's order.
fn shortfall_city(seed: u32) -> (Rc<Host>, Rc<PowerSim>) {
    let h = Host::new(32);
    plant(&h, 10, CellRect::new(8, 0, 9, 1), 1000, 1);
    for (id, x) in [(11u64, 12), (12, 18), (13, 24)] {
        h.building(
            id,
            CellRect::new(x, 0, x + 1, 1),
            &[(prop::REQUIREMENT, u32v(600))],
            None,
        );
    }
    let s = sim(&h);
    for id in 10..=13 {
        s.occupant_inserted(id);
    }
    s.set_crt(crate::rng::CrtRand::new(seed));
    (h, s)
}

fn powered_consumers(s: &PowerSim) -> [bool; 3] {
    [s.is_powered(12, 0), s.is_powered(18, 0), s.is_powered(24, 0)]
}

#[test]
fn block_probe_finds_the_order_a_shortfall_took() {
    // Two seeds that power different consumer blocks.
    let mut seen: Vec<(u32, [bool; 3])> = Vec::new();
    for seed in 1..200 {
        let (_h, s) = shortfall_city(seed);
        cycle(&s);
        let p = powered_consumers(&s);
        assert_eq!(p.iter().filter(|&&b| b).count(), 1, "seed {seed}: one consumer block");
        if seen.iter().all(|(_, q)| *q != p) {
            seen.push((seed, p));
        }
        if seen.len() == 2 {
            break;
        }
    }
    assert_eq!(seen.len(), 2, "two seeds with different orders");
    let ((seed_a, powered_a), (seed_b, _)) = (seen[0], seen[1]);
    let (_ha, a) = shortfall_city(seed_a);
    cycle(&a);
    let target = a.state().powered_current.clone();

    // Without a target the probe only records: the drawn start, a shortfall.
    let (_hb, b) = shortfall_city(seed_b);
    b.set_block_probe(true, None);
    let drawn = b.crt();
    cycle(&b);
    let draws = b.take_block_draws();
    assert_eq!(draws.len(), 1);
    let d = &draws[0];
    assert!(d.order_dependent && d.shortfall && !d.from_target && d.tried == 0);
    let mut r = drawn;
    assert_eq!(d.start, (r.rand() as i32) % d.count, "the drawn start");
    assert_ne!(powered_consumers(&b), powered_a);

    // With A's powered bits as the target, B takes A's order.
    let (_hb, b) = shortfall_city(seed_b);
    b.set_block_probe(true, Some(target.clone()));
    let crt = b.crt();
    cycle(&b);
    assert_eq!(powered_consumers(&b), powered_a);
    assert_eq!(b.state().powered_current, target);
    let mut after = crt;
    after.rand();
    assert_eq!(b.crt(), after, "the rand() call is still made");
    let draws = b.take_block_draws();
    assert_eq!(draws.len(), 1);
    let d = &draws[0];
    assert!(d.order_dependent && d.shortfall && d.from_target && d.unmatched_mark.is_none());
    assert_eq!(d.tried, d.count as u32);
    assert!(d.matching >= 1 && d.matching < d.tried);
    assert!(b.take_block_draws().is_empty(), "taken");

    // A target no order can give (every consumer powered): drawn, the mark kept.
    let mut all = target;
    for x in [12, 13, 18, 19, 24, 25] {
        all.set(x, 0);
        all.set(x, 1);
    }
    let (_hc, c) = shortfall_city(seed_b);
    c.set_block_probe(true, Some(all));
    cycle(&c);
    let d = &c.take_block_draws()[0];
    assert!(!d.from_target && d.matching == 0 && d.unmatched_mark.is_some());

    // Off: nothing recorded.
    c.set_block_probe(false, None);
    cycle(&c);
    assert!(c.take_block_draws().is_empty());

    // Enough supply: no order matters, no search.
    let h = Host::new(32);
    plant(&h, 10, CellRect::new(8, 0, 9, 1), 5000, 1);
    h.building(11, CellRect::new(12, 0, 13, 1), &[(prop::REQUIREMENT, u32v(600))], None);
    let s = sim(&h);
    s.occupant_inserted(10);
    s.occupant_inserted(11);
    s.set_block_probe(true, Some(Bitmap::new(32, 32)));
    cycle(&s);
    let d = &s.take_block_draws()[0];
    assert!(!d.order_dependent && !d.shortfall && d.tried == 0 && !d.from_target);
}

// ---- hostile input -------------------------------------------------------------------------------

#[test]
fn hostile_occupants_and_state_do_not_panic() {
    let h = Host::new(16);
    let s = sim(&h);
    h.building(
        1,
        CellRect::new(i32::MIN, i32::MIN, i32::MAX, i32::MAX),
        &[(prop::REQUIREMENT, u32v(u32::MAX)), (prop::CAPACITY, u32v(0))],
        None,
    );
    h.building(
        2,
        CellRect::new(-50, -50, -40, -40),
        &[
            (prop::CAPACITY, u32v(u32::MAX)),
            (prop::PLANT_TYPE, i32v(99)),
            (prop::CONDITION_CURVE, curve(&[0.0, 1.0, 1.0, 1.0])),
            (prop::WEAR, f32v(f32::NAN)),
        ],
        funded(),
    );
    h.building(
        3,
        CellRect::new(5, 5, 6, 6),
        &[(prop::CAPACITY, u32v(10)), (prop::PLANT_TYPE, i32v(-1))],
        None,
    );
    for occ in [1, 2, 3, 0, 77] {
        s.occupant_inserted(occ);
    }
    assert_eq!(s.state().per_type, [0; 10]);
    assert_eq!(s.state().plant_count, 2);
    h.factor.set(f32::NAN);
    s.read_tuning();
    s.update_state(|st| st.ring_index = i32::MIN);
    cycle(&s);
    s.monthly();
    cycle(&s);
    s.age_plant(2);
    s.update_power_grid(i32::MAX, i32::MIN);
    s.flood(-5, 1000);
    assert_eq!(s.monthly_consumption_at(-1, 99), 0);
    assert_eq!(s.annual_consumption_by_type(-3), 0);
    assert_eq!(s.power_plant_count_of_type(10), 0);
    s.update_state(|st| st.state = 77);
    s.do_idle();
    assert_eq!(s.state().state, state::IDLE);
    for occ in [1, 2, 3, 0, 77] {
        s.occupant_removed(occ);
    }
    assert_eq!(s.state().plant_count, 0);
    assert_eq!(h.refs_of(2), 0);
    s.cell_changed(CellRect::new(i32::MIN, i32::MIN, i32::MAX, i32::MAX));
    s.do_idle();
}

#[test]
fn empty_city_does_not_panic() {
    let h = Host::new(0);
    let s = sim(&h);
    plant(&h, 1, CellRect::new(0, 0, 0, 0), 100, 1);
    s.occupant_inserted(1);
    cycle(&s);
    s.monthly();
    // §6.3 step 1: rowMax is clamped to CellCountZ − 1 = −1.
    assert_eq!(s.flood(0, 0), (0, -1));
    assert!(!s.is_powered(0, 0));
}

// ---- persistence ---------------------------------------------------------------------------------

struct Resolver(Vec<RegisteredObject>, Rc<Host>);

impl RefResolver for Resolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        let e = self.0.iter().find(|e| e.0 == class_id && e.1 == object_id)?;
        Some(match &e.2 {
            Some(o) => RefValue::Object(o.clone()),
            None => {
                self.1.add_ref(e.3);
                RefValue::Host { token: e.3, class_id }
            }
        })
    }
}

fn record(s: &PowerSim) -> (Vec<u8>, Vec<RegisteredObject>) {
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(reg.clone());
    assert!(s.write(&mut w));
    let entries = reg.borrow().entries.clone();
    (w.into_bytes(), entries)
}

#[test]
fn record_round_trip_versions_and_truncation() {
    let h = Host::new(16);
    plant(&h, 10, CellRect::new(2, 2, 3, 3), 1000, 1);
    h.pole(20, 8, 8);
    let s = sim(&h);
    s.occupant_inserted(10);
    s.occupant_inserted(20);
    cycle(&s);
    s.update_state(|st| {
        st.strike_flag = 1;
        st.deficit = 0.25;
    });
    let (bytes, entries) = record(&s);
    // 16 x 16: map 512 bytes, 8 words per bitmap; one plant node and one pole.
    let fixed = 2 + 8 + 4 * 3 + 4 + 4 * 4 + 8 + 4 * 5 + 8 + 8 + 512 + 480 + 40 + 4 + 40 + 12 + 4 + 4 + 8;
    assert_eq!(bytes.len(), fixed + 4 + 4 * 8 * 4 + 12);

    let t = PowerSim::new(h.clone() as Rc<dyn PowerHost>, 2);
    let mut res = Resolver(entries.clone(), h.clone());
    assert!(t.read(&mut ByteReader::with_resolver(&bytes, &mut res)));
    {
        let st = t.state();
        assert!(st.running);
        assert_eq!(st.strike_flag, 0);
        assert_eq!(st.deficit, 0.25);
        assert_eq!(st.plants.len(), 1);
        assert_eq!(st.poles, vec![20]);
    }
    t.update_state(|st| st.strike_flag = 1);
    let (again, _) = record(&t);
    assert_eq!(again, bytes);

    // Version 7 is refused after the version; running is set anyway.
    let mut v7 = bytes.clone();
    v7[0] = 7;
    let u = PowerSim::new(h.clone() as Rc<dyn PowerHost>, 3);
    assert!(!u.read(&mut ByteReader::with_resolver(&v7, &mut res)));
    assert!(u.state().running);
    assert!(u.state().plants.is_empty());

    // Version 5 reads an integer percentage.
    let mut v5 = bytes.clone();
    v5[0] = 5;
    v5[22..26].copy_from_slice(&50i32.to_le_bytes());
    let u = PowerSim::new(h.clone() as Rc<dyn PowerHost>, 4);
    assert!(u.read(&mut ByteReader::with_resolver(&v5, &mut res)));
    assert_eq!(u.state().deficit, 0.5);

    // [choice] §9.1: a truncated record fails, leaves `running` clear, changes
    // nothing and releases what it took.
    let refs_before = (h.refs_of(10), h.refs_of(20), h.refs_of(DEPT));
    for n in 0..bytes.len() {
        let u = PowerSim::new(h.clone() as Rc<dyn PowerHost>, 5);
        let mut res = Resolver(entries.clone(), h.clone());
        assert!(!u.read(&mut ByteReader::with_resolver(&bytes[..n], &mut res)));
        let st = u.state();
        assert_eq!(*st, PowerState::default());
        drop(st);
        assert!(u.powered_grid().is_none());
    }
    assert_eq!((h.refs_of(10), h.refs_of(20), h.refs_of(DEPT)), refs_before);

    // An oversized consumption map fails the read before allocating it.
    let mut big = bytes.clone();
    big[78..82].copy_from_slice(&u32::MAX.to_le_bytes());
    big[82..86].copy_from_slice(&u32::MAX.to_le_bytes());
    let u = PowerSim::new(h.clone() as Rc<dyn PowerHost>, 6);
    assert!(!u.read(&mut ByteReader::with_resolver(&big, &mut res)));
    assert!(u.state().consumption_map.is_empty() && !u.state().running);
}

#[test]
fn init_keeps_a_loaded_department() {
    // §4.1 (PW11): a held department is not looked up again.
    let h = Host::new(16);
    let s = sim(&h);
    assert_eq!((s.state().department, h.refs_of(DEPT)), (Some(DEPT), 1));
    let (bytes, entries) = record(&s);
    let t = PowerSim::new(h.clone() as Rc<dyn PowerHost>, 2);
    let mut res = Resolver(entries, h.clone());
    assert!(t.read(&mut ByteReader::with_resolver(&bytes, &mut res)));
    assert_eq!(h.refs_of(DEPT), 2);
    let lookups = h.lookups.get();
    assert!(t.init());
    assert_eq!(h.lookups.get(), lookups);
    assert_eq!((t.state().department, h.refs_of(DEPT)), (Some(DEPT), 2));
}

#[test]
fn removal_counters_and_poles() {
    let h = Host::new(16);
    let s = sim(&h);
    // §3: signed clamps at removal (PW5).
    h.building(30, CellRect::new(4, 4, 5, 5), &[(prop::REQUIREMENT, u32v(300))], None);
    s.update_state(|st| {
        st.consumers = 0;
        st.requirement = 100;
    });
    s.occupant_removed(30);
    assert_eq!((s.state().consumers, s.state().requirement), (0, 0));
    s.update_state(|st| {
        st.consumers = 0x8000_0001;
        st.requirement = 0x9000_0000;
    });
    s.occupant_removed(30);
    assert_eq!((s.state().consumers, s.state().requirement), (0, 0));
    // A plant whose type changed while in the city.
    plant(&h, 31, CellRect::new(8, 8, 9, 9), 1000, 1);
    s.occupant_inserted(31);
    h.occs
        .borrow_mut()
        .get_mut(&31)
        .unwrap()
        .props
        .add(prop::PLANT_TYPE, i32v(3));
    s.occupant_removed(31);
    assert_eq!(
        (s.power_plant_count_of_type(1), s.power_plant_count_of_type(3)),
        (1, -1)
    );
    assert_eq!(s.power_plant_count(), 0);
    // §5.2 (PW10): a pole not in the list still lowers costBase.
    h.pole(40, 2, 2);
    h.pole(41, 3, 3);
    s.occupant_inserted(40);
    s.update_state(|st| {
        st.recompute = false;
        st.broken_count = 1;
    });
    s.occupant_removed(41);
    let st = s.state();
    assert_eq!((st.cost_base, st.pole_count, st.recompute), (0, 1, true));
    drop(st);
    s.occupant_removed(40);
    let st = s.state();
    assert_eq!((st.cost_base, st.pole_count, st.broken_count), (-1, 0, 1));
}

#[test]
fn a_city_loaded_before_its_cell_counts_keeps_its_wires() {
    // Run 22: the plugin's host knows no city before `Init`, so `Read` saw
    // 0 × 0 cells. The bitmaps keep the saved words and take the city's sides
    // at `Init`; the first recompute after the load powers the network again.
    let h = Host::new(32);
    plant(&h, 10, CellRect::new(8, 0, 9, 1), 1000, 1);
    h.building(11, CellRect::new(12, 0, 13, 1), &[(prop::REQUIREMENT, u32v(500))], None);
    let s = sim(&h);
    s.occupant_inserted(10);
    s.occupant_inserted(11);
    cycle(&s);
    assert_eq!(s.state().last_demand, 500);
    let wire = s.state().wire.clone();
    let (bytes, entries) = record(&s);

    h.cells.set((0, 0));
    let t = PowerSim::new(h.clone() as Rc<dyn PowerHost>, 2);
    let mut res = Resolver(entries, h.clone());
    assert!(t.read(&mut ByteReader::with_resolver(&bytes, &mut res)));
    assert_eq!(t.state().wire.words(), wire.words());
    h.cells.set((32, 32));
    assert!(t.init());
    assert_eq!(t.state().wire, wire);
    t.do_message(&message(msg::CONNECTIONS_READY, 0));
    t.set_report(true);
    cycle(&t);
    {
        let st = t.state();
        assert_eq!(
            (st.last_production, st.last_consumption, st.last_demand, st.powered),
            (1000, 500, 500, 1)
        );
    }
    assert!(t.is_powered(13, 1));

    // The report of that cycle.
    let r = t.take_report().expect("a complete report");
    assert!(r.complete);
    assert_eq!((r.cells, r.wire_size), ((32, 32), (32, 32)));
    assert_eq!(r.wire_bits, wire.count_ones());
    assert_eq!(r.consumption_cells, 1);
    assert_eq!(r.plants.len(), 1);
    let p = &r.plants[0];
    assert_eq!(
        (p.capacity, p.funding, p.produced, p.connected),
        (Some(1000), Some(1.0), 1000, true)
    );
    assert_eq!(r.floods.len(), 1);
    let f = &r.floods[0];
    assert!(f.seed_wired && f.reached > 0);
    assert_eq!(
        (f.supply, f.consumers, f.need, f.powered, f.shortfall),
        (1000, 1, 500, 1, false)
    );
    assert_eq!((r.production, r.demand, r.powered), (1000, 500, 1));
    let text = r.text();
    assert!(text.starts_with("power recompute 1: cells 32x32"), "{text}");
    assert!(
        text.contains("\n  plant 0xa: ") && text.contains("\n  flood from ("),
        "{text}"
    );
    assert!(t.take_report().is_none());

    // Off: nothing is collected.
    t.set_report(false);
    cycle(&t);
    assert!(t.take_report().is_none());
}

#[test]
fn saved_bitmaps_of_another_size_never_panic() {
    // A record from a 32 x 32 city loaded where the city has more cells (the
    // saved words are fewer than Init needs) or fewer (more words saved): the
    // load, Init, recomputes, a month, cell changes and the queries run.
    let h = Host::new(32);
    plant(&h, 10, CellRect::new(8, 0, 9, 1), 1000, 1);
    h.building(11, CellRect::new(12, 0, 13, 1), &[(prop::REQUIREMENT, u32v(500))], None);
    let s = sim(&h);
    s.occupant_inserted(10);
    s.occupant_inserted(11);
    cycle(&s);
    let (bytes, entries) = record(&s);
    for (at_read, at_init) in [(0, 64), (64, 64), (0, 16), (16, 16), (0, 40), (128, 8)] {
        h.cells.set((at_read, at_read));
        let t = PowerSim::new(h.clone() as Rc<dyn PowerHost>, 2);
        let mut res = Resolver(entries.clone(), h.clone());
        assert!(t.read(&mut ByteReader::with_resolver(&bytes, &mut res)));
        h.cells.set((at_init, at_init));
        assert!(t.init());
        let need = Bitmap::word_count(at_init, at_init);
        assert!(t.state().wire.words().len() >= need.min(32), "{at_read} -> {at_init}");
        t.do_message(&message(msg::CONNECTIONS_READY, 0));
        t.set_report(true);
        for _ in 0..3 {
            cycle(&t);
            t.cell_changed(CellRect::new(0, 0, at_init + 4, at_init + 4));
        }
        t.do_message(&message(msg::SIM_NEW_MONTH, 0));
        cycle(&t);
        let r = t.take_report().expect("a report");
        assert_eq!(r.wire_size, (at_init, at_init));
        let _ = r.text();
        for (x, z) in [(0, 0), (13, 1), (at_init - 1, at_init - 1), (at_init + 5, -3)] {
            let _ = (t.is_powered(x, z), t.is_wired(x, z));
        }
        let (again, _) = record(&t);
        assert!(!again.is_empty());
        h.cells.set((32, 32));
    }
}
