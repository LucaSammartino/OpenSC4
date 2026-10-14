// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests for the plumbing simulator. Expected values are derived by
//! hand from `docs/specs/plumbing.md` (section and §14 test numbers in the
//! comments).

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use super::*;
use crate::grid::{CityDims, GridProvider, SimGrid, TractGrid};
use crate::property::PropertyHolder;
use crate::rng::{FixedSeeds, RzRandom};
use crate::stream::{ByteReader, ByteWriter, RefResolver, RefValue, SaveObject, SaveRegistry};
use crate::variant::{mem, Elem, Variant};

const DEPT: u64 = 900;
const OCC_CLASS: u32 = 0x0CC0_0001;
const DEPT_CLASS: u32 = 0x0CC0_0002;

#[derive(Default)]
struct Occ {
    building: bool,
    network: bool,
    props: PropertyHolder,
    cell: (i32, i32),
    rect: CellRect,
    net_flags: u32,
    connects: Vec<u32>,
}

#[derive(Clone, Copy, Default)]
struct Lot {
    rect: CellRect,
    needs_power: bool,
    has_power: bool,
}

#[derive(Clone, Copy, Default)]
struct Deal {
    t: u32,
    amount: i32,
    used: i32,
    city: u64,
    flags: u32,
}

#[derive(Clone, Copy, Default)]
struct Conn {
    cell: (i32, i32),
    city: u64,
    active: bool,
    visited: bool,
    other: Option<u64>,
}

#[derive(Default)]
struct Fake {
    cells: Cell<(i32, i32)>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    lots: RefCell<BTreeMap<u64, Lot>>,
    lot_conditions: RefCell<Vec<(u64, u8)>>,
    water: RefCell<BTreeSet<(i32, i32)>>,
    water_values: RefCell<BTreeMap<(i32, i32), i32>>,
    funding: Cell<f32>,
    refs: RefCell<BTreeMap<u64, i64>>,
    log: RefCell<Vec<String>>,
    history: RefCell<Vec<(u32, u32)>>,
    expenses: RefCell<BTreeMap<u32, i64>>,
    deals: RefCell<BTreeMap<u64, Deal>>,
    conns: RefCell<BTreeMap<u64, Conn>>,
    bursts: RefCell<Vec<(i32, i32)>>,
    watered_pipes: RefCell<BTreeMap<u64, bool>>,
    grids: RefCell<Vec<(u32, GridRc)>>,
}

impl Fake {
    fn new(n: i32) -> Rc<Fake> {
        let f = Fake::default();
        f.cells.set((n, n));
        f.funding.set(1.0);
        *f.tuning.borrow_mut() = Some(Rc::new(Tuning::shipped_exemplar()));
        Rc::new(f)
    }
    fn note(&self, s: String) {
        self.log.borrow_mut().push(s);
    }
    fn prop(&self, occ: u64, id: u32) -> Option<Variant> {
        self.occs.borrow().get(&occ)?.props.get(id).cloned()
    }
    fn prop_f32(&self, occ: u64, id: u32) -> f32 {
        match self.prop(occ, id) {
            Some(Variant::Scalar(Elem::F32(v))) => v,
            other => panic!("property {id:08X}: {other:?}"),
        }
    }
    fn prop_u32(&self, occ: u64, id: u32) -> u32 {
        match self.prop(occ, id) {
            Some(Variant::Scalar(Elem::U32(v))) => v,
            other => panic!("property {id:08X}: {other:?}"),
        }
    }
    fn set_prop(&self, occ: u64, id: u32, v: Variant) {
        self.occs.borrow_mut().get_mut(&occ).unwrap().props.add(id, v);
    }
    fn inside(r: CellRect, (x, z): (i32, i32)) -> bool {
        x >= r.x1 && x <= r.x2 && z >= r.z1 && z <= r.z2
    }
}

impl GridProvider<u8> for Fake {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        let (x, z) = self.cells.get();
        let city = CityDims {
            cells_x: x,
            cells_z: z,
            ..CityDims::square(1)
        };
        let g = SimGrid::<u8>::create(&city, instance, tract_size) as GridRc;
        self.grids.borrow_mut().push((instance, g.clone()));
        Some(g)
    }
}

impl PlumbingHost for Fake {
    fn cell_counts(&self) -> (i32, i32) {
        self.cells.get()
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        ((x / 16.0).floor() as i32, (z / 16.0).floor() as i32)
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn wall_clock_seed(&self) -> u32 {
        1
    }
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        self.note(format!("agent {level} {name} {flags}"));
    }
    fn remove_agent(&self) {
        self.note("remove_agent".into());
    }
    fn subscribe(&self, t: &[u32]) {
        self.note(format!("subscribe {}", t.len()));
    }
    fn unsubscribe(&self, t: &[u32]) {
        self.note(format!("unsubscribe {}", t.len()));
    }
    fn post(&self, msg_type: u32, data1: u32) {
        self.note(format!("post {msg_type:08X} {data1}"));
    }
    fn record_history(&self, id: u32, value: u32) {
        self.history.borrow_mut().push((id, value));
    }
    fn cumulative_factor(&self, _id: u32, _index: u32) -> Option<f32> {
        Some(1.0)
    }
    fn water_value(&self, x: i32, z: i32) -> i32 {
        self.water_values.borrow().get(&(x, z)).copied().unwrap_or(0)
    }
    fn occupant_type(&self, occ: u64) -> u32 {
        match self.occs.borrow().get(&occ) {
            Some(o) if o.building => occupant_type::BUILDING,
            Some(o) if o.network => occupant_type::NETWORK,
            _ => 0,
        }
    }
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant> {
        self.prop(occ, id)
    }
    fn set_occupant_property(&self, occ: u64, id: u32, value: Variant) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.props.add(id, value);
        }
    }
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        let (x, z) = self.occs.borrow().get(&occ).map_or((0, 0), |o| o.cell);
        [x as f32 * 16.0 + 8.0, 0.0, z as f32 * 16.0 + 8.0]
    }
    fn occupant_bounding_box(&self, occ: u64) -> ([f32; 3], [f32; 3]) {
        let r = self.occs.borrow().get(&occ).map_or(CellRect::default(), |o| o.rect);
        (
            [r.x1 as f32 * 16.0, 0.0, r.z1 as f32 * 16.0],
            [(r.x2 + 1) as f32 * 16.0, 0.0, (r.z2 + 1) as f32 * 16.0],
        )
    }
    fn occupant_bounding_cells(&self, occ: u64) -> CellRect {
        self.occs
            .borrow()
            .get(&occ)
            .map_or(CellRect::new(0, 0, -1, -1), |o| o.rect)
    }
    fn building_type(&self, occ: u64) -> Option<u32> {
        self.occs.borrow().get(&occ).filter(|o| o.building).map(|_| 7)
    }
    fn buildings_in_cell(&self, x: i32, z: i32) -> Vec<u64> {
        self.occs
            .borrow()
            .iter()
            .filter(|(_, o)| o.building && Fake::inside(o.rect, (x, z)))
            .map(|(&k, _)| k)
            .collect()
    }
    fn is_network_occupant(&self, occ: u64) -> bool {
        self.occs.borrow().get(&occ).is_some_and(|o| o.network)
    }
    fn has_network_flag(&self, occ: u64, flag: u32) -> bool {
        self.network_flags(occ) & flag != 0
    }
    fn network_flags(&self, occ: u64) -> u32 {
        self.occs.borrow().get(&occ).map_or(0, |o| o.net_flags)
    }
    fn set_network_flag(&self, occ: u64, flag: u32) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.net_flags |= flag;
        }
    }
    fn clear_network_flag(&self, occ: u64, flag: u32) {
        if let Some(o) = self.occs.borrow_mut().get_mut(&occ) {
            o.net_flags &= !flag;
        }
    }
    fn connects_to_neighbor(&self, occ: u64, dir: u32, network_type: u32) -> bool {
        network_type == network::PIPE_TYPE && self.occs.borrow().get(&occ).is_some_and(|o| o.connects.contains(&dir))
    }
    fn occupied_cell(&self, occ: u64) -> (i32, i32) {
        self.occs.borrow().get(&occ).map_or((0, 0), |o| o.cell)
    }
    fn set_pipe_watered(&self, occ: u64, watered: bool) {
        self.watered_pipes.borrow_mut().insert(occ, watered);
    }
    fn lot_at(&self, x: i32, z: i32) -> Option<u64> {
        self.lots
            .borrow()
            .iter()
            .find(|(_, l)| Fake::inside(l.rect, (x, z)))
            .map(|(&k, _)| k)
    }
    fn occupant_lot(&self, occ: u64) -> Option<u64> {
        let c = self.occs.borrow().get(&occ)?.cell;
        self.lot_at(c.0, c.1)
    }
    fn lot_rect(&self, lot: u64) -> CellRect {
        self.lots.borrow().get(&lot).map_or(CellRect::default(), |l| l.rect)
    }
    fn lot_needs_power(&self, lot: u64) -> bool {
        self.lots.borrow().get(&lot).is_some_and(|l| l.needs_power)
    }
    fn lot_has_power(&self, lot: u64) -> bool {
        self.lots.borrow().get(&lot).is_some_and(|l| l.has_power)
    }
    fn set_lot_condition(&self, lot: u64, condition: u8) {
        self.lot_conditions.borrow_mut().push((lot, condition));
    }
    fn terrain_cell_counts(&self) -> Option<(i32, i32)> {
        Some(self.cells.get())
    }
    fn cell_has_water(&self, x: i32, z: i32) -> bool {
        self.water.borrow().contains(&(x, z))
    }
    fn redisplay_terrain(&self, min: (i32, i32), max: (i32, i32)) {
        self.note(format!("redisplay {min:?} {max:?}"));
    }
    fn attempt_pipe_burst(&self, x: i32, z: i32) -> bool {
        self.bursts.borrow_mut().push((x, z));
        true
    }
    fn find_deals(&self, deal_type: u32) -> Vec<u64> {
        self.deals
            .borrow()
            .iter()
            .filter(|(_, d)| d.t == deal_type)
            .map(|(&k, _)| k)
            .collect()
    }
    fn deal_type(&self, deal: u64) -> u32 {
        self.deals.borrow().get(&deal).map_or(0, |d| d.t)
    }
    fn deal_city(&self, deal: u64) -> u64 {
        self.deals.borrow().get(&deal).map_or(0, |d| d.city)
    }
    fn deal_amount(&self, deal: u64) -> i32 {
        self.deals.borrow().get(&deal).map_or(0, |d| d.amount)
    }
    fn deal_amount_used(&self, deal: u64) -> i32 {
        self.deals.borrow().get(&deal).map_or(0, |d| d.used)
    }
    fn set_deal_amount_used(&self, deal: u64, used: i32) {
        if let Some(d) = self.deals.borrow_mut().get_mut(&deal) {
            d.used = used;
        }
    }
    fn set_deal_flags(&self, deal: u64, flags: u32, on: bool) {
        if let Some(d) = self.deals.borrow_mut().get_mut(&deal) {
            if on {
                d.flags |= flags;
            } else {
                d.flags &= !flags;
            }
        }
    }
    fn connections_of_type(&self, t: u32) -> Vec<u64> {
        if t != deal::WATER_CONNECTION {
            return Vec::new();
        }
        self.conns.borrow().keys().copied().collect()
    }
    fn connections_of_type_to_city(&self, t: u32, city: u64) -> Vec<u64> {
        self.connections_of_type(t)
            .into_iter()
            .filter(|c| self.conns.borrow()[c].city == city)
            .collect()
    }
    fn find_unvisited_connection(&self, t: u32, city: u64) -> Option<u64> {
        self.connections_of_type_to_city(t, city)
            .into_iter()
            .find(|c| !self.conns.borrow()[c].visited)
    }
    fn connection_on_other_side(&self, conn: u64) -> Option<u64> {
        self.conns.borrow().get(&conn)?.other
    }
    fn connection_cell(&self, conn: u64) -> (i32, i32) {
        self.conns.borrow().get(&conn).map_or((0, 0), |c| c.cell)
    }
    fn set_connection_visited(&self, conn: u64, visited: bool) {
        if let Some(c) = self.conns.borrow_mut().get_mut(&conn) {
            c.visited = visited;
        }
    }
    fn connection_is_active(&self, conn: u64) -> bool {
        self.conns.borrow().get(&conn).is_some_and(|c| c.active)
    }
    fn set_connection_active(&self, conn: u64, active: bool) {
        if let Some(c) = self.conns.borrow_mut().get_mut(&conn) {
            c.active = active;
        }
    }
    fn mark_deals_dirty(&self) {
        self.note("deals_dirty".into());
    }
    fn create_department(&self, id: u32, group: u32) -> Option<u64> {
        self.note(format!("department {id:08X} {group:08X}"));
        Some(DEPT)
    }
    fn set_department_name(&self, _dept: u64, _group: u32, _instance: u32) {}
    fn set_department_max_funding(&self, _dept: u64, fraction: f32) {
        self.note(format!("max_funding {fraction}"));
    }
    fn create_line_item(&self, _dept: u64, line: u32, _locally: bool) -> Option<u64> {
        self.expenses.borrow_mut().insert(line, 0);
        Some(1000 + u64::from(line))
    }
    fn set_line_item_name(&self, _li: u64, _group: u32, _instance: u32) {}
    fn remove_line_item(&self, _dept: u64, line: u32) {
        self.note(format!("remove_line {line}"));
    }
    fn department_funding(&self, _dept: u64) -> f32 {
        self.funding.get()
    }
    fn department_by_id(&self, _id: u32) -> Option<u64> {
        Some(DEPT)
    }
    fn line_item(&self, _dept: u64, line: u32) -> Option<u64> {
        self.expenses
            .borrow()
            .contains_key(&line)
            .then_some(1000 + u64::from(line))
    }
    fn create_line_item_for_building_type(&self, _dept: u64, line: u32, _locally: bool) -> Option<u64> {
        self.expenses.borrow_mut().insert(line, 0);
        Some(1000 + u64::from(line))
    }
    fn set_line_item_display_flag(&self, _li: u64, _flag: u32, _on: bool) {}
    fn add_to_line_item_expenses(&self, li: u64, delta: i64) {
        *self.expenses.borrow_mut().entry((li - 1000) as u32).or_insert(0) += delta;
    }
    fn set_line_item_expenses(&self, li: u64, value: i64) {
        self.expenses.borrow_mut().insert((li - 1000) as u32, value);
    }
    fn line_item_secondary_info(&self, _li: u64) -> i64 {
        0
    }
    fn set_line_item_secondary_info(&self, _li: u64, _value: i64) {}
    fn budget_item_for_purpose(&self, _occ: u64, _purpose: u32) -> Option<BudgetItem> {
        None
    }
    fn change_budget_item_line(&self, _occ: u64, _index: u32, _line: u32) {}
    fn change_budget_item_cost(&self, _occ: u64, _index: u32, _cost: i64) {}
    fn line_item_from_budget_item(&self, _item: &BudgetItem) -> Option<u64> {
        None
    }
    fn local_funding(&self, _dept: u64, _occ: u64) -> Option<f32> {
        None
    }
    fn add_ref(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) += 1;
    }
    fn release(&self, token: u64) {
        *self.refs.borrow_mut().entry(token).or_insert(0) -= 1;
    }
    fn object_ref(&self, token: u64) -> RefValue {
        let class_id = if token == DEPT { DEPT_CLASS } else { OCC_CLASS };
        RefValue::Host { token, class_id }
    }
    fn resolve_ref(&self, r: &RefValue, iid: u32) -> Option<u64> {
        let RefValue::Host { token, class_id } = r else {
            return None;
        };
        let ok = matches!(
            (*class_id, iid),
            (OCC_CLASS, IID_OCCUPANT) | (OCC_CLASS, IID_NETWORK_OCCUPANT) | (DEPT_CLASS, IID_DEPARTMENT_BUDGET)
        );
        if ok {
            Some(*token)
        } else {
            self.release(*token);
            None
        }
    }
    fn grid_shutdown(&self, _grid: &GridRc) {
        self.note("grid_shutdown".into());
    }
}

// ---- helpers ----------------------------------------------------------------------------------------

fn u32v(v: u32) -> Variant {
    Variant::Scalar(Elem::U32(v))
}

fn f32s(v: &[f32]) -> Variant {
    Variant::array(mem::FLOAT32, v.iter().map(|&x| Elem::F32(x)).collect())
}

fn seeds() -> FixedSeeds {
    FixedSeeds {
        wall_clock: 1,
        process_clock: 0,
    }
}

/// An initialized, enabled simulator.
fn sim(f: &Rc<Fake>) -> Rc<PlumbingSim> {
    let s = PlumbingSim::new(f.clone(), 1);
    assert!(s.init());
    s.do_message(&Message::of_type(msg::CONNECTIONS_READY));
    s
}

/// Runs one complete pass (from state 0 or a recompute) to state 9.
fn pass(s: &PlumbingSim) {
    s.with_state(|st| {
        if st.state == 9 {
            st.recompute = true;
        }
    });
    for _ in 0..10_000 {
        s.do_idle(5);
        if s.snapshot().state == 9 {
            return;
        }
    }
    panic!("the pass did not end");
}

/// A building occupant over `rect` anchored at its first cell, on its own lot.
fn building(f: &Fake, id: u64, rect: CellRect, props: &[(u32, Variant)]) {
    let mut o = Occ {
        building: true,
        cell: (rect.x1, rect.z1),
        rect,
        ..Occ::default()
    };
    for (k, v) in props {
        o.props.add(*k, v.clone());
    }
    f.occs.borrow_mut().insert(id, o);
    f.lots.borrow_mut().insert(id + 10_000, Lot { rect, ..Lot::default() });
}

fn pump(s: &PlumbingSim, f: &Fake, id: u64, x: i32, z: i32, cap: u32) {
    building(f, id, CellRect::new(x, z, x, z), &[(prop::CAPACITY, u32v(cap))]);
    s.occupant_changed(id, true);
}

fn consumer(s: &PlumbingSim, f: &Fake, id: u64, x: i32, z: i32, c: u32) {
    building(f, id, CellRect::new(x, z, x, z), &[(prop::CONSUMPTION, u32v(c))]);
    s.occupant_changed(id, true);
}

fn pipe(s: &PlumbingSim, f: &Fake, id: u64, x: i32, z: i32, dirs: &[u32]) {
    f.occs.borrow_mut().insert(
        id,
        Occ {
            network: true,
            cell: (x, z),
            rect: CellRect::new(x, z, x, z),
            connects: dirs.to_vec(),
            ..Occ::default()
        },
    );
    s.occupant_changed(id, true);
}

/// Pipes along z = `z` from `x0` to `x1`, connected to each other.
fn pipe_row(s: &PlumbingSim, f: &Fake, first_id: u64, x0: i32, x1: i32, z: i32) {
    for x in x0..=x1 {
        let mut dirs = Vec::new();
        if x > x0 {
            dirs.push(0);
        }
        if x < x1 {
            dirs.push(2);
        }
        pipe(s, f, first_id + (x - x0) as u64, x, z, &dirs);
    }
}

/// The 9-test city: pump 1000 at (5,5), pipes x = 5..9 at z = 5, a consumer
/// at (12,5).
fn basic(consumption: u32) -> (Rc<Fake>, Rc<PlumbingSim>) {
    let f = Fake::new(64);
    f.water.borrow_mut().insert((5, 4));
    let s = sim(&f);
    pump(&s, &f, 1, 5, 5, 1000);
    pipe_row(&s, &f, 100, 5, 9, 5);
    consumer(&s, &f, 2, 12, 5, consumption);
    (f, s)
}

// ---- pure helpers -----------------------------------------------------------------------------------

#[test]
fn truncation_and_pipe_cost() {
    assert_eq!(trunc_i64(2.9), 2);
    assert_eq!(trunc_i64(-2.9), -2);
    assert_eq!(trunc_i64(f32::NAN), i64::MIN);
    assert_eq!(trunc_i64(1e30), i64::MIN);
    assert_eq!(trunc_i64(f32::NEG_INFINITY), i64::MIN);
    // §14 test 4: 12 × 0.1f = 1.2 → 1; 18 → 1.8 → 2; 0 → 0.
    assert_eq!(pipe_cost(12, 0.1), 1);
    assert_eq!(pipe_cost(18, 0.1), 2);
    assert_eq!(pipe_cost(0, 0.1), 0);
    // Ties to even: f32(5 × 0.1f) = 0.5 → 0; f32(15 × 0.1f) = 1.5 → 2.
    assert_eq!(pipe_cost(5, 0.1), 0);
    assert_eq!(pipe_cost(15, 0.1), 2);
    assert_eq!(pipe_cost(u32::MAX, f32::INFINITY), i64::MIN);
}

#[test]
fn nearest_water_rings() {
    // §14 test 21: water at (x+2, z+1) → 2; only (x+3, z) with reach 2 → −1.
    let w = |cells: &[(i32, i32)]| {
        let set: BTreeSet<_> = cells.iter().copied().collect();
        move |x: i32, z: i32| set.contains(&(x, z))
    };
    let mut a = w(&[(12, 11)]);
    assert_eq!(nearest_water_ring(10, 10, 2, (64, 64), &mut a), 2);
    let mut b = w(&[(13, 10)]);
    assert_eq!(nearest_water_ring(10, 10, 2, (64, 64), &mut b), -1);
    let mut c = w(&[(10, 10)]);
    assert_eq!(nearest_water_ring(10, 10, 0, (64, 64), &mut c), 0);
    // Clipped at the map edge: the ring is still the Chebyshev distance.
    let mut d = w(&[(0, 3)]);
    assert_eq!(nearest_water_ring(0, 0, 5, (64, 64), &mut d), 3);
    // PL1 (§14 test 21): clipped edges on 64 × 64.
    assert_eq!(nearest_water_ring(0, 0, 256, (64, 64), &mut w(&[(5, 3)])), 5);
    assert_eq!(nearest_water_ring(62, 10, 256, (64, 64), &mut w(&[(55, 10)])), 7);
    assert_eq!(nearest_water_ring(1, 1, 2, (64, 64), &mut w(&[(0, 3)])), 2);
    assert_eq!(nearest_water_ring(1, 1, 2, (64, 64), &mut w(&[(4, 1)])), -1);
    assert_eq!(nearest_water_ring(63, 63, 0, (64, 64), &mut w(&[(63, 63)])), 0);
    // The spiral order inside the box, each box cell once.
    let mut order = Vec::new();
    assert_eq!(
        nearest_water_ring(0, 0, 2, (64, 64), &mut |x, z| {
            order.push((x, z));
            false
        }),
        -1
    );
    assert_eq!(
        order,
        [(0, 0), (1, 0), (1, 1), (0, 1), (2, 0), (2, 1), (2, 2), (1, 2), (0, 2)]
    );
    let mut order = Vec::new();
    nearest_water_ring(10, 10, 1, (64, 64), &mut |x, z| {
        order.push((x, z));
        false
    });
    assert_eq!(
        order,
        [
            (10, 10),
            (11, 10),
            (11, 11),
            (10, 11),
            (9, 11),
            (9, 10),
            (9, 9),
            (10, 9),
            (11, 9)
        ]
    );
    let mut n = 0;
    nearest_water_ring(30, 40, 7, (64, 64), &mut |_, _| {
        n += 1;
        false
    });
    assert_eq!(n, 15 * 15);
    // An anchor outside the box (outside the terrain) → −1 with no test;
    // with reach 0 the raised box holds the anchor, which is tested.
    let mut calls = Vec::new();
    let mut log = |x, z| {
        calls.push((x, z));
        true
    };
    assert_eq!(nearest_water_ring(64, 10, 256, (64, 64), &mut log), -1);
    assert_eq!(nearest_water_ring(-100, -100, 10, (64, 64), &mut log), -1);
    assert_eq!(nearest_water_ring(64, 10, 0, (64, 64), &mut log), 0);
    assert_eq!(calls, [(64, 10)]);
    // int32 arithmetic: a reach of 2^32 − 1 is −1, so the box [1, 1]
    // excludes the anchor.
    let mut e = w(&[(63, 63)]);
    assert_eq!(nearest_water_ring(0, 0, u32::MAX, (64, 64), &mut e), -1);
    assert_eq!(nearest_water_ring(0, 0, i32::MAX as u32, (64, 64), &mut e), 63);
    assert_eq!(nearest_water_ring(5, 5, 3, (0, 0), &mut e), -1);
}

#[test]
fn tuning_defaults_and_shipped() {
    // §14 test 1.
    let mut t = Tuning::default();
    t.reload(None);
    assert_eq!((t.radius, t.fresh_reach, t.salt_reach, t.distance_loss), (7, 2, 1, 20));
    assert_eq!(
        (t.max_pumped_pollution, t.burst_probability, t.max_funding),
        (100, 0.3, 1.2)
    );
    assert_eq!(t.cost_per_pipe, 0.0);
    assert!(t.efficiency_curve.is_empty());
    assert_eq!(t.funding_decay_curve.eval(0.5), 1.0);
    assert_eq!(t.usage_decay_curve.eval(0.5), 1.0);
    assert_eq!(t.pipe_decay_curve.eval(0.0), -0.02);
    assert_eq!(t.pipe_decay_curve.eval(120.0), 0.01);
    // §14 test 2.
    t.reload(Some(&Tuning::shipped_exemplar()));
    assert_eq!(
        (t.radius, t.fresh_reach, t.salt_reach, t.distance_loss),
        (7, 256, 256, 100)
    );
    assert_eq!((t.max_pumped_pollution, t.burst_probability), (512, 0.03));
    // PL6: 120 × 0.01f = 1.1999999 (0x3F999999), not 1.2 (0x3F99999A).
    assert_eq!(t.max_funding.to_bits(), 0x3F99_9999);
    assert_eq!(t.cost_per_pipe, 0.1);
    assert_eq!(t.efficiency_curve.eval(90.0), 80.0);
    // An exemplar with empty curves: "empty → (0,0)(100,100)" for three.
    let mut h = PropertyHolder::new();
    for id in [
        tuning_id::EFFICIENCY_CURVE,
        tuning_id::FUNDING_DECAY_CURVE,
        tuning_id::USAGE_DECAY_CURVE,
        tuning_id::PIPE_DECAY_CURVE,
    ] {
        h.add(id, f32s(&[]));
    }
    let mut t = Tuning::default();
    t.reload(Some(&h));
    assert_eq!(t.efficiency_curve.eval(40.0), 40.0);
    assert_eq!(t.funding_decay_curve.eval(40.0), 40.0);
    assert_eq!(t.usage_decay_curve.eval(40.0), 40.0);
    assert!(t.pipe_decay_curve.is_empty());
}

// ---- lifecycle --------------------------------------------------------------------------------------

#[test]
fn init_and_shutdown() {
    let f = Fake::new(64);
    let s = PlumbingSim::new(f.clone(), 1);
    assert!(s.init());
    let log = f.log.borrow().clone();
    assert_eq!(
        log[..3],
        [
            "agent 6 cSC4PlumbingSimulator 2",
            "agent 4 cSC4PlumbingSimulator 2",
            "agent 10 cSC4PlumbingSimulator 2"
        ]
    );
    assert!(log.contains(&"subscribe 6".to_string()));
    assert!(log.contains(&"department AA4014B4 4A357EAF".to_string()));
    let st = s.snapshot();
    assert_eq!((st.count_x, st.count_z, st.flags.len()), (64, 64, 4096));
    assert_eq!((st.state, st.recompute, st.enabled), (0, true, false));
    assert_eq!(f.grids.borrow().len(), 2);
    assert_eq!(f.refs.borrow()[&DEPT], 1);
    // Disabled (§14 test 18): DoIdle is true and changes nothing.
    assert!(s.do_idle(5));
    assert_eq!(s.snapshot().state, 0);
    // A second Init changes nothing, not even the generator (PL2).
    f.log.borrow_mut().clear();
    s.set_random(RzRandom::new(77, &seeds()));
    assert!(s.init());
    assert!(f.log.borrow().is_empty());
    assert_eq!(s.random().map(|r| r.lcg_state()), Some(77));
    assert!(s.shutdown());
    assert_eq!(f.refs.borrow()[&DEPT], 0);
    let st = s.snapshot();
    assert!(st.flags.is_empty());
    assert!(st.iterator.is_none() && st.rng.is_none() && !st.grids_initialized);
    // [choice] Init after Shutdown rebuilds everything as on a new city.
    s.with_state(|st| (st.state, st.recompute) = (5, false));
    f.log.borrow_mut().clear();
    assert!(s.init());
    let st = s.snapshot();
    assert_eq!((st.state, st.recompute, st.flags.len()), (0, true, 4096));
    assert!(st.working.is_some() && st.published.is_some() && st.rng.is_some());
    assert_eq!(st.iterator, Some(crate::grid::ScanIterator::for_counts(64, 64)));
    assert_eq!(f.grids.borrow().len(), 4);
    assert_eq!(f.log.borrow().iter().filter(|l| l.starts_with("agent")).count(), 3);
    assert!(s.shutdown());
    assert_eq!(s.add_ref(), 1);
    assert_eq!(s.add_ref(), 2);
    assert_eq!(s.release(), 1);
    assert_eq!(s.release(), 0);
    assert!(s.snapshot().destroyed);
}

// ---- occupants --------------------------------------------------------------------------------------

#[test]
fn pipe_flags_burst_and_cost() {
    // §14 test 3.
    let f = Fake::new(64);
    let s = sim(&f);
    pipe(&s, &f, 100, 10, 20, &[0, 2]);
    assert_eq!(s.snapshot().flag(10, 20), 0x15);
    let burst = Message {
        msg_type: msg::PIPE_BURST,
        data1: 10,
        data2: 20,
        ..Message::default()
    };
    assert!(s.do_message(&burst));
    assert_eq!(s.snapshot().flag(10, 20), 0x00);
    s.with_state(|st| st.update_flag(3, 3, |_| 0x55));
    s.pipe_burst(3, 3);
    assert_eq!(s.snapshot().flag(3, 3), 0x40);
    // No bounds check, but no panic either.
    s.pipe_burst(i32::MAX, i32::MAX);
    s.pipe_burst(-5, 0);
    // §14 test 4: 12 pipes → 1, 18 → 2.
    for i in 1..12 {
        pipe(&s, &f, 100 + i, i as i32, 30, &[]);
    }
    assert_eq!(f.expenses.borrow()[&budget::PIPE_LINE], 1);
    for i in 12..18 {
        pipe(&s, &f, 100 + i, i as i32, 30, &[]);
    }
    assert_eq!(f.expenses.borrow()[&budget::PIPE_LINE], 2);
    // §14 test 16: removing a distressed, burst pipe.
    f.set_network_flag(105, network::DISTRESSED | network::BURST);
    s.with_state(|st| {
        st.num_distressed = 3;
        st.num_burst = 2;
    });
    s.occupant_changed(105, false);
    assert_eq!(s.pipe_counts(), (17, 2, 1));
    assert_eq!(f.refs.borrow()[&105], 0);
    // Removal clears the bits of the cell (keeps 0xE0).
    assert_eq!(s.snapshot().flag(5, 30), 0);
}

#[test]
fn consumers_and_producers() {
    let f = Fake::new(64);
    let s = sim(&f);
    consumer(&s, &f, 2, 12, 5, 300);
    consumer(&s, &f, 3, 12, 5, 50);
    assert_eq!(s.water_demand(), 350);
    assert_eq!(s.snapshot().flag(12, 5), flag::CONSUMER);
    // Removing one consumer clears the bit for the whole cell [quirk].
    s.occupant_changed(3, false);
    assert_eq!(s.water_demand(), 300);
    assert_eq!(s.snapshot().flag(12, 5), 0);
    // A producer over a 2×2 lot.
    building(&f, 4, CellRect::new(20, 20, 21, 21), &[(prop::CAPACITY, u32v(500))]);
    s.occupant_changed(4, true);
    let st = s.snapshot();
    assert_eq!(st.producers.len(), 1);
    assert_eq!(st.producers[0].rect, CellRect::new(20, 20, 21, 21));
    assert_eq!(st.flag(21, 21), flag::PRODUCER);
    assert_eq!(f.prop_f32(4, prop::CONDITION), 0.0);
    assert_eq!(f.refs.borrow()[&4], 1);
    // A producer without a lot leaks [quirk].
    f.occs.borrow_mut().insert(
        5,
        Occ {
            building: true,
            cell: (40, 40),
            ..Occ::default()
        },
    );
    f.set_prop(5, prop::CAPACITY, u32v(1));
    s.occupant_changed(5, true);
    assert_eq!(s.snapshot().leaked, vec![5]);
    assert_eq!(s.num_water_sources(), 1);
    s.occupant_changed(4, false);
    assert_eq!(s.num_water_sources(), 0);
    assert_eq!(s.snapshot().flag(21, 21), 0);
    assert_eq!(f.refs.borrow()[&4], 0);
}

// ---- production (§7.2) --------------------------------------------------------------------------

fn production_city(cap: u32) -> (Rc<Fake>, Rc<PlumbingSim>) {
    let f = Fake::new(64);
    let s = sim(&f);
    pump(&s, &f, 1, 10, 10, cap);
    (f, s)
}

#[test]
fn pump_capacity() {
    // §14 test 5.
    let (f, s) = production_city(3000);
    f.water.borrow_mut().insert((11, 10));
    s.update_water_production();
    assert_eq!(s.water_production(1), 3000);
    f.funding.set(0.5);
    s.update_water_production();
    assert_eq!(s.water_production(1), 899);
    assert_eq!(s.water_producer_info(1).unwrap().efficiency, 0.29999998);
    f.funding.set(1.0);
    let mut h = Tuning::shipped_exemplar();
    h.add(tuning_id::DISTANCE_LOSS, u32v(20));
    h.add(tuning_id::FRESH_REACH, u32v(2));
    *f.tuning.borrow_mut() = Some(Rc::new(h));
    s.read_tuning();
    f.water.borrow_mut().clear();
    f.water.borrow_mut().insert((12, 9));
    s.update_water_production();
    assert_eq!(s.water_production(1), 600);
    f.water.borrow_mut().clear();
    f.water.borrow_mut().insert((13, 10));
    s.update_water_production();
    assert_eq!(s.water_production(1), 0);
    // Type 2 needs no water.
    f.set_prop(1, prop::SOURCE_TYPE, Variant::Scalar(Elem::U8(2)));
    s.update_water_production();
    assert_eq!(s.water_production(1), 3000);
    // No exemplar: the efficiency curve is empty → 0 ([choice] §8).
    let g = Fake::new(64);
    *g.tuning.borrow_mut() = None;
    g.water.borrow_mut().insert((11, 10));
    let t = sim(&g);
    pump(&t, &g, 1, 10, 10, 3000);
    t.update_water_production();
    assert_eq!(t.water_production(1), 0);
}

#[test]
fn pollution_power_and_condition() {
    // §14 test 6.
    let (f, s) = production_city(3000);
    f.water.borrow_mut().insert((11, 10));
    f.water_values.borrow_mut().insert((10, 10), 600);
    s.update_water_production();
    let i = s.water_producer_info(1).unwrap();
    assert_eq!((i.production, i.polluted), (0, true));
    assert_eq!(s.num_polluted_water_sources(), 1);
    f.water_values.borrow_mut().insert((10, 10), 512);
    s.update_water_production();
    assert!(!s.water_producer_info(1).unwrap().polluted);
    assert_eq!(s.water_production(1), 3000);
    f.water_values.borrow_mut().insert((10, 10), 600);
    pump(&s, &f, 2, 30, 30, 100);
    f.water_values.borrow_mut().insert((30, 30), 100);
    s.update_water_production();
    assert_eq!(s.snapshot().average_water_pollution, 350);
    // §14 test 7: no power → production 0, efficiency 0, polluted unchanged.
    let lot = f.lot_at(10, 10).unwrap();
    f.lots.borrow_mut().get_mut(&lot).unwrap().needs_power = true;
    s.update_water_production();
    let i = s.water_producer_info(1).unwrap();
    assert_eq!((i.production, i.efficiency, i.polluted), (0, 0.0, true));
    // §14 test 8: condition curve (0,1)(100,0.5) at condition 50 → 0.75.
    f.lots.borrow_mut().get_mut(&lot).unwrap().has_power = true;
    f.water_values.borrow_mut().insert((10, 10), 0);
    f.set_prop(1, prop::CONDITION_CURVE, f32s(&[0.0, 1.0, 100.0, 0.5]));
    f.set_prop(1, prop::CONDITION, Variant::Scalar(Elem::F32(50.0)));
    s.update_water_production();
    assert_eq!(s.water_production(1), 2250);
    assert_eq!(s.water_producer_condition_percent(1), Some(0.75));
    assert_eq!(s.water_producer_condition_percent(2), None);
}

// ---- the pass (§6) ------------------------------------------------------------------------------

#[test]
fn basic_flood() {
    // §14 test 9.
    let (f, s) = basic(300);
    pass(&s);
    assert!(s.are_watered_buildings_in_cell(12, 5));
    assert_eq!(s.supply_value(12, 5), 3);
    assert_eq!(s.supply_value(7, 5), 0x09);
    assert_eq!(s.supply_value(5, 5), 0x0F);
    assert_eq!(s.last_month().produced, 1000);
    assert_eq!(s.last_month().consumed, 300);
    assert_eq!(f.prop_u32(1, prop::WATER_USED), 300);
    let h = f.history.borrow().clone();
    assert_eq!(
        h,
        vec![
            (history::PRODUCED, 1000),
            (history::CONSUMED, 300),
            (history::IMPORTED, 0),
            (history::EXPORTED, 0),
            (history::PIPES, 5),
            (history::UNUSED, 0),
            (history::WATERED_BUILDINGS, 1),
            (history::UNWATERED_BUILDINGS, 0),
        ]
    );
    assert!(f.watered_pipes.borrow().values().all(|&w| w));
    assert_eq!(s.num_watered_buildings(), 1);
    // Ring 6 (x = 15) watered, ring 7 (x = 16) not.
    assert!(s.is_cell_watered(15, 5));
    assert!(!s.is_cell_watered(16, 5));
    let log = f.log.borrow().clone();
    assert!(log.contains(&format!("post {:08X} 0", msg::WATER_MAP_CHANGED)));
    assert_eq!(log.last().unwrap(), &format!("post {:08X} 8", msg::DEAL_STATUS));
}

#[test]
fn bitmap_without_room_for_every_cell() {
    // [choice]: 10 × 10 = 100 cells, but the shared bitmap holds 100 / 32 = 3
    // words = 96 bits, so (6..=9, 9) cannot be marked. The pipes there are
    // never pushed and the floods still end. (The bitmap's bit is `x & 31`
    // within word `(10 z + x) >> 5`, so cells of one word with the same x
    // also share a bit: the ring flood is aliased, as in the original.)
    let f = Fake::new(10);
    f.water.borrow_mut().insert((2, 8));
    let s = sim(&f);
    pump(&s, &f, 1, 2, 9, 1000);
    pipe_row(&s, &f, 100, 2, 9, 9);
    pass(&s);
    assert_eq!(s.last_month().produced, 1000);
    assert!(s.is_cell_watered(5, 9));
    assert!(!s.is_cell_watered(8, 9));
    assert_eq!(s.supply_value(8, 9) & super::value::WATERED, 0);
}

#[test]
fn shortage() {
    // §14 test 10.
    let (f, s) = basic(1200);
    pass(&s);
    assert!(!s.is_cell_watered(12, 5));
    assert!(!s.is_cell_watered(13, 5));
    assert_eq!(s.supply_value(12, 5), 2);
    assert_eq!(s.last_month().consumed, 1000);
    assert_eq!(f.prop_u32(1, prop::WATER_USED), 1000);
    assert_eq!(s.num_unwatered_buildings(), 1);
}

#[test]
fn ring_geometry() {
    // §14 test 11.
    let f = Fake::new(128);
    f.water.borrow_mut().insert((49, 50));
    let s = sim(&f);
    pump(&s, &f, 1, 50, 50, 1000);
    pipe(&s, &f, 100, 50, 50, &[]);
    pass(&s);
    for (x, z, w) in [
        (53, 53, true),
        (54, 54, true),
        (55, 55, false),
        (50, 56, true),
        (50, 57, false),
    ] {
        assert_eq!(s.is_cell_watered(x, z), w, "({x}, {z})");
    }
    assert_eq!(s.supply_value(50, 50), 0x0F);
}

#[test]
fn grid_values_and_swap() {
    // §14 tests 19 and 20.
    let f = Fake::new(64);
    f.water.borrow_mut().insert((0, 0));
    let s = sim(&f);
    pump(&s, &f, 1, 5, 5, 1000);
    pipe_row(&s, &f, 100, 5, 9, 5);
    building(&f, 2, CellRect::new(8, 6, 9, 7), &[(prop::CONSUMPTION, u32v(10))]);
    s.occupant_changed(2, true);
    building(&f, 3, CellRect::new(40, 40, 40, 40), &[(prop::CAPACITY, u32v(0))]);
    s.occupant_changed(3, true);
    pipe(&s, &f, 200, 9, 6, &[]);
    pass(&s);
    for (x, z) in [(8, 6), (8, 7), (9, 7)] {
        assert_eq!(s.supply_value(x, z), 3);
    }
    assert_eq!(s.supply_value(9, 6), 0x0B);
    assert_eq!(s.supply_value(40, 40), 6);
    assert_eq!(s.supply_value(5, 5), 0x0F);
    assert!(s.are_watered_buildings_in_cell(8, 6));
    assert!(!s.are_watered_buildings_in_cell(40, 40));
    assert!(s.is_building_watered(2));
    let instance_of = |g: &GridRc| f.grids.borrow().iter().find(|(_, h)| Rc::ptr_eq(h, g)).map(|(i, _)| *i);
    assert_eq!(instance_of(&s.watered_grid().unwrap()), Some(WORKING_GRID_INSTANCE));
    pass(&s);
    assert_eq!(instance_of(&s.watered_grid().unwrap()), Some(PUBLISHED_GRID_INSTANCE));
}

#[test]
fn slicing_gives_the_same_result() {
    let run = |slice: Option<u32>| {
        let (_f, s) = basic(300);
        s.set_slice_cells(slice);
        let mut calls = 0;
        s.with_state(|st| st.state = 0);
        loop {
            s.do_idle(5);
            calls += 1;
            if s.snapshot().state == 9 {
                break;
            }
        }
        let g = s.watered_grid().unwrap();
        let v: Vec<u8> = (0..64)
            .flat_map(|z| (0..64).map(move |x| (x, z)))
            .map(|(x, z)| g.get(x, z))
            .collect();
        (v, calls)
    };
    let (a, n) = run(None);
    let (b, m) = run(Some(7));
    assert_eq!(a, b);
    // 2N + 7 calls for one network; slicing needs more.
    assert_eq!(n, 9);
    assert!(m > n);
}

#[test]
fn export_and_import() {
    // §14 test 12.
    let (f, s) = basic(100);
    f.conns.borrow_mut().insert(
        3000,
        Conn {
            cell: (9, 5),
            city: 77,
            ..Conn::default()
        },
    );
    f.deals.borrow_mut().insert(
        2000,
        Deal {
            t: deal::EXPORT,
            amount: 400,
            city: 77,
            ..Deal::default()
        },
    );
    pass(&s);
    assert_eq!(s.last_month().exported, 400);
    assert_eq!(f.deals.borrow()[&2000].used, 400);
    assert_eq!(f.prop_u32(1, prop::WATER_USED), 500);
    // Export 1500: take 1000, the first consumer stops the flood.
    f.deals.borrow_mut().get_mut(&2000).unwrap().amount = 1500;
    pass(&s);
    assert_eq!(f.deals.borrow()[&2000].used, 1000);
    assert_eq!(f.prop_u32(1, prop::WATER_USED), 1000);
    assert!(!s.is_cell_watered(12, 5));
    assert_eq!(f.deals.borrow()[&2000].flags, deal::FLAG_NOT_DELIVERED);
    // Not on the network.
    f.conns.borrow_mut().get_mut(&3000).unwrap().cell = (40, 40);
    f.deals.borrow_mut().get_mut(&2000).unwrap().amount = 400;
    pass(&s);
    assert_eq!(s.last_month().exported, 0);
    assert_ne!(f.deals.borrow()[&2000].flags & deal::FLAG_NO_CONNECTION, 0);
    assert_eq!(f.prop_u32(1, prop::WATER_USED), 100);

    // §14 test 13: an import seeds a network.
    let g = Fake::new(64);
    let t = sim(&g);
    g.conns.borrow_mut().insert(
        3000,
        Conn {
            cell: (0, 10),
            city: 5,
            other: Some(3001),
            ..Conn::default()
        },
    );
    g.conns.borrow_mut().insert(
        3001,
        Conn {
            cell: (-1, 10),
            city: 99,
            active: true,
            ..Conn::default()
        },
    );
    g.deals.borrow_mut().insert(
        2000,
        Deal {
            t: deal::IMPORT,
            amount: 500,
            city: 5,
            ..Deal::default()
        },
    );
    pipe_row(&t, &g, 100, 0, 5, 10);
    consumer(&t, &g, 2, 7, 10, 200);
    pass(&t);
    assert!(t.are_watered_buildings_in_cell(7, 10));
    assert_eq!(t.last_month().imported, 200);
    assert_eq!(t.last_month().produced, 0);
    assert_eq!(g.deals.borrow()[&2000].used, 200);
}

#[test]
fn deal_message_aborts_the_pass() {
    // §14 test 17.
    let (f, s) = basic(300);
    f.deals.borrow_mut().insert(
        2000,
        Deal {
            t: deal::IMPORT,
            ..Deal::default()
        },
    );
    while s.snapshot().state != 4 {
        s.do_idle(5);
    }
    let counter = s.snapshot().pass_counter;
    let mut m = Message {
        msg_type: msg::DEAL_STATUS,
        data1: 8,
        deal: Some(2000),
        ..Message::default()
    };
    s.do_message(&m);
    assert_eq!(s.snapshot().state, 4);
    m.data1 = 0;
    s.do_message(&m);
    let st = s.snapshot();
    assert_eq!((st.state, st.recompute), (9, true));
    s.do_idle(5);
    let st = s.snapshot();
    assert_eq!((st.state, st.pass_counter), (1, counter + 1));
}

// ---- the month (§7.1) ---------------------------------------------------------------------------

#[test]
fn pipe_funding_and_bursts() {
    // §14 tests 14 and 15.
    let f = Fake::new(64);
    let mut h = Tuning::shipped_exemplar();
    h.add(tuning_id::PIPE_DECAY_CURVE, f32s(&[0.0, -0.25, 120.0, 0.25]));
    h.add(tuning_id::BURST_PROBABILITY, Variant::Scalar(Elem::F32(0.5)));
    *f.tuning.borrow_mut() = Some(Rc::new(h));
    f.funding.set(0.0);
    let s = sim(&f);
    for i in 0..100u64 {
        pipe(&s, &f, 100 + i, (i % 64) as i32, (i / 64) as i32, &[]);
    }
    s.set_random(RzRandom::new(1, &seeds()));
    s.monthly_simulation();
    assert_eq!(s.snapshot().pipe_condition, 0.25);
    assert_eq!(s.pipe_counts(), (100, 25, 10));
    assert_eq!(f.bursts.borrow().len(), 10);
    let distressed = f
        .occs
        .borrow()
        .values()
        .filter(|o| o.net_flags & network::DISTRESSED != 0)
        .count();
    assert_eq!(distressed, 25);
    s.monthly_simulation();
    assert_eq!(s.pipe_counts(), (100, 50, 25));
    // The generator advanced 8 draws: the 9th raw of seed 1.
    let mut r = RzRandom::new(1, &seeds());
    for _ in 0..8 {
        r.next_raw();
    }
    assert_eq!(s.random().unwrap().next_raw(), r.next_raw());
    // No pipes: cond 0, target 0, hi 0, still 4 draws.
    let g = Fake::new(64);
    let t = sim(&g);
    t.set_random(RzRandom::new(1, &seeds()));
    t.monthly_simulation();
    let mut r = RzRandom::new(1, &seeds());
    for _ in 0..4 {
        r.next_raw();
    }
    assert_eq!(t.random().unwrap().next_raw(), r.next_raw());
    assert_eq!(t.pipe_counts(), (0, 0, 0));
    // NaN funding does not panic and gives condition 0.
    g.funding.set(f32::NAN);
    t.monthly_simulation();
    assert_eq!(t.snapshot().pipe_condition, 0.0);
}

#[test]
fn pipe_funding_helper_with_the_shipped_curve() {
    // §7.1 steps 1-3 with the shipped curve 0xCA7138C4 (0,-0.01)(20,-0.005)
    // (50,0)(100,0.02)(110,0.05) and burst probability 0.03.
    let mut t = Tuning::default();
    t.reload(Some(&Tuning::shipped_exemplar()));
    // Full funding repairs: 0 - 0.02 clamps to 0, nothing is distressed.
    let full = pipe_funding(&t, 0.0, 403, 1.0);
    assert_eq!(full, PipeFunding { condition: 0.0, target: 0, burst_max: 0 });
    // At 50 % the curve is 0: the condition stays.
    assert_eq!(pipe_funding(&t, 0.5, 403, 0.5).condition, 0.5);
    // No funding decays by 0.01 a month: trunc(403 × 0.01) = 4 distressed,
    // the draw bound trunc(2 × 4 × 0.03) = 0.
    let none = pipe_funding(&t, 0.0, 403, 0.0);
    assert_eq!(none.condition, 0.01);
    assert_eq!((none.target, none.burst_max), (4, 0));
    // Without pipes the condition is 0 whatever the funding.
    assert_eq!(pipe_funding(&t, 0.7, 0, 0.0), PipeFunding::default());
    // It is the month's arithmetic: the same as monthly_simulation.
    let f = Fake::new(64);
    *f.tuning.borrow_mut() = Some(Rc::new(Tuning::shipped_exemplar()));
    f.funding.set(0.0);
    let s = sim(&f);
    for i in 0..403u64 {
        pipe(&s, &f, 100 + i, (i % 64) as i32, (i / 64) as i32, &[]);
    }
    s.set_random(RzRandom::new(1, &seeds()));
    s.monthly_simulation();
    assert_eq!(s.snapshot().pipe_condition, none.condition);
    assert_eq!(s.pipe_counts(), (403, none.target, 0));
}

#[test]
fn ageing() {
    // §14 test 22.
    let (f, s) = production_city(1000);
    f.water.borrow_mut().insert((11, 10));
    s.update_water_production();
    f.set_prop(1, prop::WATER_USED, u32v(500));
    f.set_prop(1, prop::AGING_RATE, Variant::Scalar(Elem::F32(1.0)));
    s.age_water_producers();
    assert_eq!(f.prop_f32(1, prop::CONDITION), 0.45);
    s.with_state(|st| st.producers[0].production = 0);
    s.age_water_producers();
    assert_eq!(f.prop_f32(1, prop::CONDITION), 0.45f32 + 0.1f32);
    f.set_prop(1, prop::CONDITION, Variant::Scalar(Elem::F32(99.0)));
    s.age_water_producers();
    let lot = f.lot_at(10, 10).unwrap();
    assert_eq!(f.lot_conditions.borrow().last(), Some(&(lot, 4)));
    let (lowest, worst) = s.find_water_buildings();
    assert_eq!(lowest, None);
    assert_eq!(worst.map(|w| w.0), Some(1));
}

// ---- persistence (§9) ---------------------------------------------------------------------------

struct Resolver(Vec<crate::stream::RegisteredObject>, Rc<Fake>);

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

fn record(s: &PlumbingSim) -> (Vec<u8>, Vec<crate::stream::RegisteredObject>) {
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(reg.clone());
    assert!(s.write(&mut w));
    let entries = reg.borrow().entries.clone();
    (w.into_bytes(), entries)
}

#[test]
fn persistence() {
    // §14 test 23: 64 × 64, 2 producers, no current, 3 pipes → 4375 bytes.
    let f = Fake::new(64);
    let s = sim(&f);
    pump(&s, &f, 1, 5, 5, 1000);
    pump(&s, &f, 2, 20, 5, 700);
    for i in 0..3 {
        pipe(&s, &f, 100 + i, 6 + i as i32, 5, &[0, 2]);
    }
    consumer(&s, &f, 3, 12, 5, 30);
    f.water.borrow_mut().insert((0, 0));
    pass(&s);
    // PL4: the iterator rests done at (0, countZ) and is saved there.
    let it = s.snapshot().iterator.unwrap();
    assert_eq!(
        (it.state, it.x, it.z, it.step),
        (crate::grid::ScanState::Done, 0, 64, 1)
    );
    assert_eq!((it.x0, it.z0, it.x1, it.z1), (0, 0, 63, 63));
    let (bytes, entries) = record(&s);
    assert_eq!(bytes.len(), 4375);
    let at = 2 + 16 + 8 + 4096 + 4 + 4 + 8 + 4 + 4 + 80 + 36;
    let mut want = vec![2u8];
    for v in [0i32, 64, 1, 0, 0, 63, 63] {
        want.extend_from_slice(&v.to_le_bytes());
    }
    assert_eq!(&bytes[at..at + 29], &want[..]);
    assert_eq!(&bytes[..2], &[4, 0]);
    let g = Fake::new(64);
    let t = PlumbingSim::new(g.clone(), 2);
    let mut res = Resolver(entries.clone(), g.clone());
    assert!(t.read(&mut ByteReader::with_resolver(&bytes, &mut res)));
    let st = t.snapshot();
    assert_eq!((st.state, st.grids_initialized, st.num_pipes), (0, true, 3));
    assert_eq!(st.producers.len(), 2);
    assert_eq!(st.flags, s.snapshot().flags);
    let (again, _) = record(&t);
    assert_eq!(again, bytes);
    // A loaded department loses line item 1 at Init.
    assert!(t.init());
    assert!(g.log.borrow().contains(&"remove_line 1".to_string()));
    // Version 3 is rejected.
    let mut v3 = bytes.clone();
    v3[0] = 3;
    let u = PlumbingSim::new(g.clone(), 3);
    let mut res = Resolver(entries.clone(), g.clone());
    assert!(!u.read(&mut ByteReader::with_resolver(&v3, &mut res)));
    // Every truncation reads without panicking; the version decides the result.
    for n in 0..bytes.len() {
        let u = PlumbingSim::new(g.clone(), 3);
        let mut res = Resolver(entries.clone(), g.clone());
        assert_eq!(u.read(&mut ByteReader::with_resolver(&bytes[..n], &mut res)), n >= 2);
    }
    // A hostile count ends the read.
    let mut big = bytes.clone();
    big[18..22].copy_from_slice(&5000u32.to_le_bytes());
    let u = PlumbingSim::new(g.clone(), 3);
    let mut res = Resolver(entries, g.clone());
    assert!(u.read(&mut ByteReader::with_resolver(&big, &mut res)));
    assert!(u.snapshot().flags.is_empty());
}

#[test]
fn no_panics_without_init_or_grids() {
    let f = Fake::new(0);
    let s = PlumbingSim::new(f.clone(), 1);
    s.do_message(&Message::of_type(msg::CONNECTIONS_READY));
    s.monthly_simulation();
    s.with_state(|st| st.state = 0);
    for _ in 0..20 {
        s.do_idle(5);
    }
    assert_eq!(s.supply_value(3, 3), 0);
    assert!(!s.is_cell_watered(-1, 7));
    assert!(s.water_producer_info(9).is_none());
    let (b, _) = record(&s);
    assert_eq!(&b[..2], &[4, 0]);
    // Grids sized 0: the pass still ends.
    let s = sim(&f);
    pass(&s);
    assert_eq!(s.snapshot().state, 9);
}
