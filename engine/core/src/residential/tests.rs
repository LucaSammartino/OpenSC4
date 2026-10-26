// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests of the residential simulator against a fake host: the §15
//! tests of `docs/specs/residential.md` (named `t01_…` to `t24_…`), the 24
//! tests of `residential-education-health.md` §11 (named `reh01_…` to
//! `reh24_…`), the RS answers, the lifecycle and the save record.
//!
//! The fake host keeps core [`SimGrid`]s, logs every call the simulator
//! makes, models a small budget simulator (items, departments, line items),
//! the occupants' properties and lots and the ordinances that are on, and
//! hands the simulator a scripted generator that records each
//! `RandomDoubleRangeUniform(a, b)` and returns queued values (`b` when the
//! queue is empty, which never starts a strike or an effect).

#![allow(clippy::needless_range_loop)]

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;

use super::*;
use crate::grid::{CityDims, CoreGrids, GridElem, ScanState, SimGrid};
use crate::property::PropertyHolder;
use crate::stream::{ByteReader, ByteWriter, RefResolver, RefValue, RegisteredObject, SaveObject, SaveRegistry};
use crate::variant::{mem, Elem, Variant};
use crate::x87::rhe;

/// The class ID the fake host writes for its objects (occupants and
/// departments).
const HOST_CLASS: u32 = 0x5EED_0001;

/// One call the simulator made on the host.
#[derive(Clone, Debug, PartialEq)]
enum Ev {
    Services,
    Developers,
    DemandNode,
    MessageServer,
    Budget,
    Simulator,
    AddAgent(u32, String, u32),
    RemoveAgent,
    Subscribe(u32),
    Unsubscribe(u32),
    ReleaseMessageServerAndBudget,
    ReleaseServices,
    QueueLength(u32, u32),
    CreateDepartment(u32, u32, f32, f32),
    ReleaseDepartment(u64),
    /// `Shutdown` of a grid, by instance ID (the temporary grid is 0).
    ShutdownGrid(u32),
    /// `GetOrdinanceOnArray` (one per ordinance-effect read, REH §4.3).
    Ordinance,
    History(u32, i32),
    Effect,
    Transform([f32; 3]),
    StartEffect(u64),
    Strike(u32, Option<u64>, u32, u32),
    AddRef(u64),
    Release(u64),
    // ---- the budget simulator (REH §4.1, §4.2) ----
    ChangeLine(u64, u32, u32),
    AddLocal(u64, u64, u32),
    RemoveLocal(u64, u64, u32),
    SetLocal(u64, u64, f32, u32),
    CreateLine(u64, u32, bool),
    RemoveLine(u64, u32),
    LineCount(u64, i64),
    DisplayFlag(u64, u32, bool),
    /// Automata controllers: (name, occupant, added).
    Controllers(String, u64, bool),
}

struct Host {
    precision: Cell<crate::x87::Precision>,
    city: CityDims,
    events: RefCell<Vec<Ev>>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    month: Cell<u32>,
    month_reads: Cell<u32>,
    today: Cell<u32>,
    demand_node: Cell<bool>,
    message_server: Cell<bool>,
    simulator: Cell<bool>,
    /// `GetPollutionValue(kind, …)` at every cell.
    pollution: Cell<[u16; 3]>,
    /// The number of `GetPollutionValue` calls.
    pollution_calls: Cell<u32>,
    /// The growth developers' population maps (`int16`).
    developers: [Rc<SimGrid<i16>>; 3],
    building_map: RefCell<Option<Rc<SimGrid<u8>>>>,
    /// What `CreateVisualEffect` returns.
    effect: Cell<CreatedEffect>,
    altitude: Cell<f32>,
    /// Occupant → the groups it belongs to; every occupant listed here is a
    /// building occupant.
    groups: RefCell<BTreeMap<u64, Vec<u32>>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    bool_props: RefCell<BTreeMap<(u64, u32), bool>>,
    next_department: Cell<u64>,
    /// Occupant properties other than the bool ones.
    props: RefCell<BTreeMap<(u64, u32), Variant>>,
    building_types: RefCell<BTreeMap<u64, u32>>,
    slot11: RefCell<BTreeMap<u64, u32>>,
    lots: RefCell<BTreeMap<u64, (f32, f32)>>,
    positions: RefCell<BTreeMap<u64, (f32, f32)>>,
    /// Budget items by (occupant, purpose).
    items: RefCell<BTreeMap<(u64, u32), BudgetItem>>,
    /// `GetDepartmentBudget`: department ID → token.
    departments: RefCell<BTreeMap<u32, u64>>,
    funding: RefCell<BTreeMap<u64, f32>>,
    locally_funded: RefCell<BTreeSet<(u64, u64, u32)>>,
    /// Line items by (department, line) and their counts.
    line_items: RefCell<BTreeMap<(u64, u32), u64>>,
    line_counts: RefCell<BTreeMap<u64, i64>>,
    next_line_item: Cell<u64>,
    /// The ordinances that are on, by ID, with their misc properties.
    ordinances: RefCell<BTreeMap<u32, Rc<PropertyHolder>>>,
    draws: RefCell<Vec<(f64, f64)>>,
    queue: RefCell<VecDeque<f64>>,
}

impl Host {
    fn new(city: CityDims, dev_size: i32) -> Rc<Host> {
        Rc::new(Host {
            precision: Cell::new(crate::x87::Precision::Bits24),
            city,
            events: RefCell::new(Vec::new()),
            tuning: RefCell::new(Some(Rc::new(shipped_tuning()))),
            month: Cell::new(6),
            month_reads: Cell::new(0),
            today: Cell::new(1000),
            demand_node: Cell::new(true),
            message_server: Cell::new(true),
            simulator: Cell::new(true),
            pollution: Cell::new([0; 3]),
            pollution_calls: Cell::new(0),
            developers: std::array::from_fn(|_| SimGrid::<i16>::create(&city, 0, dev_size)),
            building_map: RefCell::new(None),
            effect: Cell::new(CreatedEffect::Effect(500)),
            altitude: Cell::new(7.0),
            groups: RefCell::new(BTreeMap::new()),
            refs: RefCell::new(BTreeMap::new()),
            bool_props: RefCell::new(BTreeMap::new()),
            next_department: Cell::new(900),
            props: RefCell::new(BTreeMap::new()),
            building_types: RefCell::new(BTreeMap::new()),
            slot11: RefCell::new(BTreeMap::new()),
            lots: RefCell::new(BTreeMap::new()),
            positions: RefCell::new(BTreeMap::new()),
            items: RefCell::new(BTreeMap::new()),
            departments: RefCell::new(BTreeMap::new()),
            funding: RefCell::new(BTreeMap::new()),
            locally_funded: RefCell::new(BTreeSet::new()),
            line_items: RefCell::new(BTreeMap::new()),
            line_counts: RefCell::new(BTreeMap::new()),
            next_line_item: Cell::new(700),
            ordinances: RefCell::new(BTreeMap::new()),
            draws: RefCell::new(Vec::new()),
            queue: RefCell::new(VecDeque::new()),
        })
    }

    fn ev(&self, e: Ev) {
        self.events.borrow_mut().push(e);
    }

    fn take_events(&self) -> Vec<Ev> {
        std::mem::take(&mut *self.events.borrow_mut())
    }

    /// The `RecordHistoryData` calls so far, in order.
    fn history(&self) -> Vec<(u32, i32)> {
        self.events
            .borrow()
            .iter()
            .filter_map(|e| match e {
                Ev::History(id, v) => Some((*id, *v)),
                _ => None,
            })
            .collect()
    }

    fn ref_count(&self, token: u64) -> i64 {
        self.refs.borrow().get(&token).copied().unwrap_or(0)
    }

    /// The developers' population maps at tract (x, z).
    fn set_new(&self, x: i32, z: i32, new: [u16; 3]) {
        for w in 0..3 {
            self.developers[w].set(x, z, new[w] as i16);
        }
    }

    /// Makes `occ` a building occupant in `groups`.
    fn building(&self, occ: u64, groups: &[u32]) {
        self.groups.borrow_mut().insert(occ, groups.to_vec());
    }

    fn set_prop(&self, occ: u64, id: u32, v: Variant) {
        self.props.borrow_mut().insert((occ, id), v);
    }

    fn prop(&self, occ: u64, id: u32) -> Option<Variant> {
        self.props.borrow().get(&(occ, id)).cloned()
    }

    fn prop_u32(&self, occ: u64, id: u32) -> Option<u32> {
        match self.prop(occ, id)? {
            Variant::Scalar(e) => Some(e.as_i64() as u32),
            _ => None,
        }
    }

    fn prop_f32(&self, occ: u64, id: u32) -> Option<f32> {
        match self.prop(occ, id)? {
            Variant::Scalar(Elem::F32(x)) => Some(x),
            _ => None,
        }
    }

    /// A department `GetDepartmentBudget(id)` returns, at `funding`.
    fn department(&self, id: u32, token: u64, funding: f32) {
        self.departments.borrow_mut().insert(id, token);
        self.funding.borrow_mut().insert(token, funding);
    }

    fn item(&self, occ: u64, purpose: u32, item: BudgetItem) {
        self.items.borrow_mut().insert((occ, purpose), item);
    }

    fn enact(&self, id: u32, p: PropertyHolder) {
        self.ordinances.borrow_mut().insert(id, Rc::new(p));
    }
}

impl GridProvider<u8> for Host {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        GridProvider::<u8>::create_grid(&CoreGrids { city: self.city }, instance, tract_size)
    }
}

impl GridProvider<u16> for Host {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u16>>> {
        GridProvider::<u16>::create_grid(&CoreGrids { city: self.city }, instance, tract_size)
    }
}

impl GridProvider<f32> for Host {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<f32>>> {
        GridProvider::<f32>::create_grid(&CoreGrids { city: self.city }, instance, tract_size)
    }
}

/// The instance ID of a core grid.
fn instance_of<T: GridElem>(g: &Rc<dyn TractGrid<T>>) -> u32 {
    g.save_ref()
        .object::<SimGrid<T>>()
        .map_or(u32::MAX, |g| g.instance_id())
}

impl ResidentialHost for Host {
    fn invocation_precision(&self) -> crate::x87::Precision {
        self.precision.get()
    }

    fn acquire_services(&self) {
        self.ev(Ev::Services);
    }
    fn acquire_developers(&self) {
        self.ev(Ev::Developers);
    }
    fn acquire_demand_node(&self) -> bool {
        self.ev(Ev::DemandNode);
        self.demand_node.get()
    }
    fn acquire_message_server(&self) -> bool {
        self.ev(Ev::MessageServer);
        self.message_server.get()
    }
    fn acquire_budget_simulator(&self) -> bool {
        self.ev(Ev::Budget);
        true
    }
    fn has_simulator(&self) -> bool {
        self.ev(Ev::Simulator);
        self.simulator.get()
    }
    fn add_agent(&self, level: u32, name: &str, flags: u32) {
        self.ev(Ev::AddAgent(level, name.to_string(), flags));
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
    fn release_message_server_and_budget(&self) {
        self.ev(Ev::ReleaseMessageServerAndBudget);
    }
    fn release_services(&self) {
        self.ev(Ev::ReleaseServices);
    }
    fn set_history_queue_length(&self, series: u32, length: u32) {
        self.ev(Ev::QueueLength(series, length));
    }
    fn create_department(&self, id: u32, name: u32, max_funding: f32, init_funding: f32) -> Option<u64> {
        self.ev(Ev::CreateDepartment(id, name, max_funding, init_funding));
        let t = self.next_department.get();
        self.next_department.set(t + 1);
        Some(t)
    }
    fn release_department(&self, department: u64) {
        self.ev(Ev::ReleaseDepartment(department));
        *self.refs.borrow_mut().entry(department).or_insert(0) -= 1;
    }
    fn shutdown_grid_u8(&self, grid: &Rc<dyn TractGrid<u8>>) {
        self.ev(Ev::ShutdownGrid(instance_of(grid)));
    }
    fn shutdown_grid_u16(&self, grid: &Rc<dyn TractGrid<u16>>) {
        self.ev(Ev::ShutdownGrid(instance_of(grid)));
    }
    fn shutdown_grid_f32(&self, grid: &Rc<dyn TractGrid<f32>>) {
        self.ev(Ev::ShutdownGrid(instance_of(grid)));
    }
    fn add_ref(&self, token: u64) {
        self.ev(Ev::AddRef(token));
        *self.refs.borrow_mut().entry(token).or_insert(0) += 1;
    }
    fn release(&self, token: u64) {
        self.ev(Ev::Release(token));
        *self.refs.borrow_mut().entry(token).or_insert(0) -= 1;
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn building_map(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        self.building_map.borrow().clone().map(|g| g as Rc<dyn TractGrid<u8>>)
    }
    fn building_map_key(&self, purpose: u32, wealth: u32) -> u8 {
        ((purpose << 2) + wealth + 1) as u8
    }
    fn developer_population_map(&self, dev_type: u32) -> Option<Rc<dyn TractGrid<i16>>> {
        let w = DEVELOPERS.iter().position(|&d| d == dev_type)?;
        Some(self.developers[w].clone() as Rc<dyn TractGrid<i16>>)
    }
    fn pollution_value(&self, kind: u32, _cell_x: i32, _cell_z: i32) -> u16 {
        self.pollution_calls.set(self.pollution_calls.get() + 1);
        self.pollution.get()[kind as usize]
    }
    fn sim_month(&self) -> u32 {
        self.month_reads.set(self.month_reads.get() + 1);
        self.month.get()
    }
    fn sim_day_number(&self) -> u32 {
        self.today.get()
    }
    fn altitude(&self, _x: f32, _z: f32) -> f32 {
        self.altitude.get()
    }
    fn create_visual_effect(&self, name: &str) -> CreatedEffect {
        assert_eq!(name, BUZZARDS_EFFECT);
        self.ev(Ev::Effect);
        self.effect.get()
    }
    fn set_effect_transform(&self, _effect: u64, translation: [f32; 3]) {
        self.ev(Ev::Transform(translation));
    }
    fn start_and_release_effect(&self, effect: u64) {
        self.ev(Ev::StartEffect(effect));
    }
    fn record_history(&self, series: u32, value: i32) {
        self.ev(Ev::History(series, value));
    }
    fn post_strike_message(&self, msg_type: u32, occupant: Option<u64>, data2: u32, data3: u32) {
        self.ev(Ev::Strike(msg_type, occupant, data2, data3));
    }
    fn occupant_as_building(&self, occupant: u64) -> Option<u64> {
        self.groups.borrow().contains_key(&occupant).then_some(occupant)
    }
    fn is_occupant_group(&self, occupant: u64, group: u32) -> bool {
        self.groups.borrow().get(&occupant).is_some_and(|g| g.contains(&group))
    }
    fn occupant_bool_property(&self, occupant: u64, id: u32) -> Option<bool> {
        self.bool_props.borrow().get(&(occupant, id)).copied()
    }
    fn set_occupant_bool_property(&self, occupant: u64, id: u32, value: bool) -> bool {
        self.bool_props.borrow_mut().insert((occupant, id), value);
        true
    }
    fn object_ref(&self, token: u64) -> RefValue {
        RefValue::Host {
            token,
            class_id: HOST_CLASS,
        }
    }
    fn resolve_ref(&self, r: &RefValue, _iid: u32) -> Option<u64> {
        match r {
            RefValue::Host { token, .. } => Some(*token),
            _ => None,
        }
    }
}

impl EducationHealthHost for Host {
    fn budget_item(&self, occupant: u64, purpose: u32) -> Option<BudgetItem> {
        self.items.borrow().get(&(occupant, purpose)).copied()
    }
    fn change_budget_item_line(&self, occupant: u64, index: u32, line: u32) {
        self.ev(Ev::ChangeLine(occupant, index, line));
        for ((o, _), item) in self.items.borrow_mut().iter_mut() {
            if *o == occupant && item.index == index {
                item.line = line;
            }
        }
    }
    fn department_budget(&self, department_id: u32) -> Option<u64> {
        self.departments.borrow().get(&department_id).copied()
    }
    fn department_funding(&self, department: u64) -> Option<f32> {
        let f = self.funding.borrow().get(&department).copied().unwrap_or(1.0);
        // NaN marks a department whose funding cannot be read (RG10).
        (!f.is_nan()).then_some(f)
    }
    fn is_locally_funded(&self, department: u64, occupant: u64, purpose: u32) -> bool {
        self.locally_funded.borrow().contains(&(department, occupant, purpose))
    }
    fn add_locally_funded(&self, department: u64, occupant: u64, purpose: u32) {
        self.ev(Ev::AddLocal(department, occupant, purpose));
        self.locally_funded.borrow_mut().insert((department, occupant, purpose));
    }
    fn remove_locally_funded(&self, department: u64, occupant: u64, purpose: u32) {
        self.ev(Ev::RemoveLocal(department, occupant, purpose));
        self.locally_funded
            .borrow_mut()
            .remove(&(department, occupant, purpose));
    }
    fn set_local_funding(&self, department: u64, occupant: u64, fraction: f32, purpose: u32) {
        self.ev(Ev::SetLocal(department, occupant, fraction, purpose));
        if let Some(item) = self.items.borrow_mut().get_mut(&(occupant, purpose)) {
            item.local_funding = fraction;
        }
    }
    fn line_item(&self, department: u64, line: u32) -> Option<u64> {
        self.line_items.borrow().get(&(department, line)).copied()
    }
    fn create_line_item_for_building_type(&self, department: u64, line: u32, local: bool) -> Option<u64> {
        self.ev(Ev::CreateLine(department, line, local));
        let t = self.next_line_item.get();
        self.next_line_item.set(t + 1);
        self.line_items.borrow_mut().insert((department, line), t);
        Some(t)
    }
    fn remove_line_item(&self, department: u64, line: u32) {
        self.ev(Ev::RemoveLine(department, line));
        self.line_items.borrow_mut().remove(&(department, line));
    }
    fn line_item_count(&self, line_item: u64) -> i64 {
        self.line_counts.borrow().get(&line_item).copied().unwrap_or(0)
    }
    fn set_line_item_count(&self, line_item: u64, count: i64) {
        self.ev(Ev::LineCount(line_item, count));
        self.line_counts.borrow_mut().insert(line_item, count);
    }
    fn set_line_item_display_flag(&self, line_item: u64, flag: u32, on: bool) {
        self.ev(Ev::DisplayFlag(line_item, flag, on));
    }
    fn occupant_property(&self, occupant: u64, id: u32) -> Option<Variant> {
        self.prop(occupant, id)
    }
    fn set_occupant_u32(&self, occupant: u64, id: u32, value: u32) {
        self.set_prop(occupant, id, Variant::Scalar(Elem::U32(value)));
    }
    fn set_occupant_f32(&self, occupant: u64, id: u32, value: f32) {
        self.set_prop(occupant, id, f(value));
    }
    fn building_type(&self, building: u64) -> u32 {
        self.building_types.borrow().get(&building).copied().unwrap_or(0)
    }
    fn building_slot11(&self, building: u64) -> u32 {
        self.slot11.borrow().get(&building).copied().unwrap_or(0)
    }
    fn occupant_lot_center(&self, occupant: u64) -> Option<(f32, f32)> {
        self.lots.borrow().get(&occupant).copied()
    }
    fn occupant_position(&self, occupant: u64) -> (f32, f32) {
        self.positions.borrow().get(&occupant).copied().unwrap_or((0.0, 0.0))
    }
    fn ordinances_on(&self) -> Vec<u32> {
        self.ev(Ev::Ordinance);
        self.ordinances.borrow().keys().copied().collect()
    }
    fn ordinance_properties(&self, id: u32) -> Option<Rc<PropertyHolder>> {
        self.ordinances.borrow().get(&id).cloned()
    }
    fn add_controllers(&self, name: &str, occupant: u64) {
        self.ev(Ev::Controllers(name.to_string(), occupant, true));
    }
    fn remove_controllers(&self, name: &str, occupant: u64) {
        self.ev(Ev::Controllers(name.to_string(), occupant, false));
    }
}

/// The class's generator, scripted: records `(a, b)` and returns the next
/// queued value, or `b` when the queue is empty.
struct Script(Rc<Host>);

impl ResidentialRandom for Script {
    fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        self.0.draws.borrow_mut().push((a, b));
        self.0.queue.borrow_mut().pop_front().unwrap_or(b)
    }
}

// ---- tuning helpers ----

fn f(v: f32) -> Variant {
    Variant::Scalar(Elem::F32(v))
}

fn curve(points: &[f32]) -> Variant {
    Variant::array(mem::FLOAT32, points.iter().map(|&x| Elem::F32(x)).collect())
}

/// The shipped exemplar, edited.
fn tuned(edit: impl FnOnce(&mut PropertyHolder)) -> PropertyHolder {
    let mut t = shipped_tuning();
    edit(&mut t);
    t
}

/// A flat life-expectancy curve: LE = `le` at every HQ.
fn flat_le(t: &mut PropertyHolder, le: f32) {
    t.add(id::HQ_TO_LIFE_EXPECTANCY, curve(&[0.0, le, 200.0, le]));
}

/// No EQ or HQ decay.
fn no_decay(t: &mut PropertyHolder) {
    t.add(id::EQ_DECAY, f(0.0));
    t.add(id::HQ_DECAY, f(0.0));
}

#[test]
fn vc29_newcomers_and_global_query_preserve_float_stores() {
    use crate::x87::Precision::{Bits24, Bits53};
    for (pc, bits, global) in [
        (Bits24, 0x3DCC_CCCE, 0.100_000_008_940_696_72),
        (Bits53, 0x3DCC_CCCD, 0.099_999_998_297_010_15),
    ] {
        let fx = Fx::with(
            4,
            Some(tuned(|t| {
                no_decay(t);
                flat_le(t, 100.0);
            })),
        );
        fx.h.precision.set(pc);
        fx.s.with_state(|s| {
            s.population_tuning.new_eq[0] = 0.1;
            s.population_tuning.new_hq[0] = 0.1;
        });
        fx.set_tract(
            0,
            0,
            Tract {
                old: [3, 0, 0],
                new: [7, 0, 0],
                eq: [0.1, 0.0, 0.0],
                hq: [0.1, 0.0, 0.0],
                age: 30,
            },
        );
        fx.s.do_population_change(0, 0);
        assert_eq!(fx.eq(0, 0)[0].to_bits(), bits);
        assert_eq!(fx.hq(0, 0)[0].to_bits(), bits);
        fx.s.with_state(|s| s.population = 7);
        assert_eq!(fx.s.global_quotient_register(true), global);
        assert_eq!(fx.s.global_quotient_register(false), global);
        assert_eq!(fx.s.get_global_eq().to_bits(), bits);
        assert_eq!(fx.s.get_global_hq().to_bits(), bits);
    }
}

// ---- the fixture ----

/// One tract's inputs: the current populations, the developers' new
/// populations, EQ, HQ and the average age.
#[derive(Clone, Copy, Default)]
struct Tract {
    old: [u16; 3],
    new: [u16; 3],
    eq: [f32; 3],
    hq: [f32; 3],
    age: u8,
}

/// A tract whose population does not change.
fn steady(pop: [u16; 3]) -> Tract {
    Tract {
        old: pop,
        new: pop,
        ..Tract::default()
    }
}

struct Fx {
    h: Rc<Host>,
    s: Rc<ResidentialSim>,
}

impl Fx {
    /// A constructed (not initialised) simulator over `city`, whose
    /// developers' population maps have tract size `dev`.
    fn constructed(city: CityDims, dev: i32, tuning: Option<PropertyHolder>) -> Fx {
        let h = Host::new(city, dev);
        *h.tuning.borrow_mut() = tuning.map(Rc::new);
        let s = ResidentialSim::new(h.clone(), 77, Box::new(Script(h.clone())));
        Fx { h, s }
    }

    /// An initialised new city of `cells` × `cells` cells, P = E = 4, with
    /// `tuning`; the host log is cleared after `Init`.
    fn with(cells: i32, tuning: Option<PropertyHolder>) -> Fx {
        let fx = Fx::constructed(CityDims::square(cells), 4, tuning);
        assert!(fx.s.init());
        fx.h.take_events();
        fx
    }

    /// The same with the shipped tuning (§15 "unless stated").
    fn new(cells: i32) -> Fx {
        Fx::with(cells, Some(shipped_tuning()))
    }

    fn grids(&self) -> Grids {
        self.s.snapshot().grids
    }

    fn state(&self) -> u32 {
        self.s.snapshot().state
    }

    /// One NewDay step that must finish; the state it leaves.
    fn step(&self) -> u32 {
        assert!(self.s.do_daily_simulation());
        self.state()
    }

    fn set_state(&self, st: u32) {
        self.s.with_state(|s| s.state = st);
    }

    fn set_tract(&self, x: i32, z: i32, t: Tract) {
        let g = self.grids();
        for w in 0..3 {
            g.population[w].as_ref().unwrap().set(x, z, t.old[w]);
            g.eq[w].as_ref().unwrap().set(x, z, t.eq[w]);
            g.hq[w].as_ref().unwrap().set(x, z, t.hq[w]);
        }
        g.age.as_ref().unwrap().set(x, z, t.age);
        self.h.set_new(x, z, t.new);
    }

    fn eq(&self, x: i32, z: i32) -> [f32; 3] {
        let g = self.grids();
        std::array::from_fn(|w| g.eq[w].as_ref().unwrap().get(x, z))
    }

    fn hq(&self, x: i32, z: i32) -> [f32; 3] {
        let g = self.grids();
        std::array::from_fn(|w| g.hq[w].as_ref().unwrap().get(x, z))
    }

    fn pop(&self, x: i32, z: i32) -> [u16; 3] {
        let g = self.grids();
        std::array::from_fn(|w| g.population[w].as_ref().unwrap().get(x, z))
    }

    fn age(&self, x: i32, z: i32) -> u8 {
        self.grids().age.as_ref().unwrap().get(x, z)
    }
}

/// Address-only identity of a shared grid.
fn addr<T: GridElem>(g: &Rc<dyn TractGrid<T>>) -> *const () {
    Rc::as_ptr(g).cast::<()>()
}

// ---- education / health helpers ----

fn u(v: u32) -> Variant {
    Variant::Scalar(Elem::U32(v))
}

/// A property holder (an ordinance's misc properties).
fn props(entries: &[(u32, Variant)]) -> PropertyHolder {
    let mut h = PropertyHolder::default();
    for (id, v) in entries {
        h.add(*id, v.clone());
    }
    h
}

/// A service building: a building occupant in group `svc.groups[i]` with
/// capacity, coverage radius and lot centre, placed straight into group list
/// `i` with one reference (as state 0 would after `DoBuildingChanged`).
fn service(fx: &Fx, occ: u64, svc: &Service, i: usize, capacity: u32, radius: f32, centre: (f32, f32)) {
    fx.h.building(occ, &[svc.groups[i]]);
    fx.h.set_prop(occ, svc.capacity, u(capacity));
    fx.h.set_prop(occ, svc.radius, f(radius));
    fx.h.lots.borrow_mut().insert(occ, centre);
    // The building stands at its lot's centre ("served" reads GetPosition,
    // EH6).
    fx.h.positions.borrow_mut().insert(occ, centre);
    fx.h.add_ref(occ);
    fx.s.with_state(|s| {
        let l = if svc.school {
            &mut s.lists.school_groups[i]
        } else {
            &mut s.lists.hospital_groups[i]
        };
        l.push(Some(occ));
    });
    fx.h.take_events();
}

/// A service building in the service's "can strike" group with a grade.
fn striker(fx: &Fx, occ: u64, svc: &Service, i: usize, grade: f32) {
    service(fx, occ, svc, i, 100, 10.0, (32.0, 32.0));
    fx.h.groups.borrow_mut().get_mut(&occ).unwrap().push(svc.strike_group);
    fx.h.set_prop(occ, svc.grade, f(grade));
}

/// The query properties (REH §4.11) of a building.
fn graded(fx: &Fx, occ: u64, svc: &Service, eff_cap: u32, served: u32, grade: f32) {
    fx.h.set_prop(occ, svc.effective_capacity, u(eff_cap));
    fx.h.set_prop(occ, svc.served, u(served));
    fx.h.set_prop(occ, svc.grade, f(grade));
}

/// A `Read` that failed left the simulator as constructed (RS9 [choice]).
fn assert_constructed(fx: &Fx) {
    let (s, c) = (fx.s.snapshot(), ResidentialState::default());
    assert!(!s.city_started && !s.initialized);
    assert_eq!(
        (
            s.state,
            s.population,
            s.running_population,
            s.le_sum,
            s.population_size,
            s.eq_size
        ),
        (
            c.state,
            c.population,
            c.running_population,
            c.le_sum,
            c.population_size,
            c.eq_size
        )
    );
    assert_eq!(s.lists, c.lists);
    assert_eq!((s.health, s.education), (c.health, c.education));
    assert!(s.grids.eq[0].is_none() && s.grids.school_coverage[0].is_none());
}

// ================================================================ §15

/// §15 test 1 through the states: 0 → 13 → 1 → 14 → 2 → 15 → 3 build the
/// three proximity maps from the BDS building map; R$$ and R$$$ reuse the
/// temporary grid, which state 3 shuts down and frees.
#[test]
fn t01_proximity_one_source_through_the_states() {
    let fx = Fx::new(5);
    let map = SimGrid::<u8>::create(&fx.h.city, 0, 1);
    map.set(2, 2, 6);
    *fx.h.building_map.borrow_mut() = Some(map);
    assert_eq!(fx.step(), state::SEED_LOW);
    assert_eq!(fx.step(), state::PROXIMITY_LOW);
    let temp = fx.grids().temporary.expect("state 13 creates the temporary grid");
    assert_eq!(fx.step(), state::SEED_MEDIUM);
    assert_eq!(fx.step(), state::PROXIMITY_MEDIUM);
    assert_eq!(addr(fx.grids().temporary.as_ref().unwrap()), addr(&temp));
    assert_eq!(fx.step(), state::SEED_HIGH);
    assert_eq!(fx.step(), state::PROXIMITY_HIGH);
    assert_eq!(addr(fx.grids().temporary.as_ref().unwrap()), addr(&temp));
    assert_eq!(fx.step(), state::HISTOGRAM_RESET);
    assert!(fx.grids().temporary.is_none());
    assert!(fx.h.take_events().contains(&Ev::ShutdownGrid(0)));

    // [compat] (RG23): the seed reads `map[x][0]` and writes (x, 0) only, so
    // the key at (2, 2) is never seen: every map is 1 on the z = 0 edge and 0
    // elsewhere (RS5: the running `d` does not wrap, so the edge is 1, not 0).
    let g = fx.grids();
    for w in 0..3 {
        for x in 0..5 {
            for z in 0..5 {
                let want = u8::from(z == 0);
                assert_eq!(g.proximity[w].as_ref().unwrap().get(x, z), want, "w {w} at ({x}, {z})");
            }
        }
    }
    let r = g.proximity[0].as_ref().unwrap();
    assert_eq!((r.get(0, 0), r.get(2, 2), r.get(4, 2)), (1, 0, 0));
}

/// §15 test 1, second half, and the §5.3 worked example through the states
/// (RG23): a key on the z = 0 edge is the only one the seed sees.
#[test]
fn t01b_proximity_worked_example_through_the_states() {
    let rows = |g: &Rc<dyn TractGrid<u8>>, n: i32| -> Vec<Vec<u8>> {
        (0..n).map(|x| (0..n).map(|z| g.get(x, z)).collect()).collect()
    };
    // Rows x of length n: `e[x]` at z = 0, 0 elsewhere.
    let edge = |e: &[u8]| -> Vec<Vec<u8>> {
        let mut g = vec![vec![0u8; e.len()]; e.len()];
        for (row, &v) in g.iter_mut().zip(e) {
            row[0] = v;
        }
        g
    };
    // 5 × 5, key 6 at (2, 2) and (3, 0): R$ (3, 0) = 0, R$$ unchanged.
    let fx = Fx::new(5);
    let map = SimGrid::<u8>::create(&fx.h.city, 0, 1);
    map.set(2, 2, 6);
    map.set(3, 0, 6);
    *fx.h.building_map.borrow_mut() = Some(map);
    while fx.state() != state::HISTOGRAM_RESET {
        fx.step();
    }
    let g = fx.grids();
    assert_eq!(rows(g.proximity[0].as_ref().unwrap(), 5), edge(&[1, 1, 1, 0, 1]));
    assert_eq!(rows(g.proximity[1].as_ref().unwrap(), 5), edge(&[1, 1, 1, 1, 1]));

    // 4 × 4: key 6 at (1, 0), (1, 1), (2, 3), key 7 at (3, 0), no key 8.
    let fx = Fx::new(4);
    let map = SimGrid::<u8>::create(&fx.h.city, 0, 1);
    for (x, z) in [(1, 0), (1, 1), (2, 3)] {
        map.set(x, z, 6);
    }
    map.set(3, 0, 7);
    *fx.h.building_map.borrow_mut() = Some(map);
    while fx.state() != state::HISTOGRAM_RESET {
        fx.step();
    }
    let g = fx.grids();
    assert_eq!(rows(g.proximity[0].as_ref().unwrap(), 4), edge(&[1, 0, 1, 1]), "R$");
    assert_eq!(rows(g.proximity[1].as_ref().unwrap(), 4), edge(&[1, 1, 1, 0]), "R$$");
    assert_eq!(rows(g.proximity[2].as_ref().unwrap(), 4), edge(&[1, 1, 1, 1]), "R$$$");
}

/// §15 test 2 through the states: 300 × 1 map, key 7 at (0, 0).
#[test]
fn t02_proximity_cap_through_the_states() {
    let city = CityDims {
        cells_x: 300,
        cells_z: 1,
        cell_width_x: 16.0,
        cell_width_z: 16.0,
    };
    let fx = Fx::constructed(city, 4, Some(shipped_tuning()));
    assert!(fx.s.init());
    let map = SimGrid::<u8>::create(&city, 0, 1);
    map.set(0, 0, 7);
    *fx.h.building_map.borrow_mut() = Some(map);
    while fx.state() != state::HISTOGRAM_RESET {
        fx.step();
    }
    let p = fx.grids().proximity[1].clone().unwrap();
    assert_eq!(p.get(200, 0), 200);
    assert_eq!(p.get(280, 0), 255);
    assert_eq!(fx.grids().proximity[0].as_ref().unwrap().get(200, 0), 255);
}

/// §15 test 3 through the states (RG23): proximity size 2, a 4 × 4
/// building map of size 1, key at map cell (3, 1). Tract (1, 1) finds no key
/// and overwrites tract (1, 0)'s 0 in the same cell (1, 0).
#[test]
fn t03_proximity_coarser_target_through_the_states() {
    let fx = Fx::constructed(CityDims::square(4), 4, Some(shipped_tuning()));
    fx.s.with_state(|s| s.proximity_size = 2);
    assert!(fx.s.init());
    let map = SimGrid::<u8>::create(&fx.h.city, 0, 1);
    map.set(3, 1, 6);
    *fx.h.building_map.borrow_mut() = Some(map);
    while fx.state() != state::HISTOGRAM_RESET {
        fx.step();
    }
    let p = fx.grids().proximity[0].clone().unwrap();
    assert_eq!(p.geometry().tract_size, 2);
    assert_eq!((p.count_x(), p.count_z()), (2, 2));
    assert_eq!((p.get(0, 0), p.get(1, 0), p.get(0, 1), p.get(1, 1)), (1, 1, 0, 0));
}

/// §5.3 step 1 *[compat]* (RG24): with no building map the seed writes
/// nothing, not even the z = 0 edge. State 13's fresh temporary grid stays all
/// 0, states 14 and 15 keep what the grid holds (the previous wealth's
/// result), and every proximity grid ends all 0 (seeding FAR on the edge
/// would have given 1 on the whole edge).
#[test]
fn rg24_proximity_without_a_building_map() {
    let fx = Fx::new(4);
    assert!(fx.h.building_map.borrow().is_none());
    for w in 0..3 {
        fx.grids().proximity[w].as_ref().unwrap().fill(9);
    }
    let all = |g: &Rc<dyn TractGrid<u8>>| -> Vec<u8> {
        (0..4)
            .flat_map(|x| (0..4).map(move |z| (x, z)))
            .map(|(x, z)| g.get(x, z))
            .collect()
    };
    assert_eq!(fx.step(), state::SEED_LOW);
    assert_eq!(fx.step(), state::PROXIMITY_LOW);
    let temp = fx.grids().temporary.expect("state 13 still creates the temporary grid");
    assert_eq!(all(&temp), vec![0; 16], "nothing seeded, not even the edge");
    assert_eq!(fx.step(), state::SEED_MEDIUM);
    // A value the previous wealth left survives state 14's seed.
    temp.set(2, 0, 3);
    assert_eq!(fx.step(), state::PROXIMITY_MEDIUM);
    assert_eq!(temp.get(2, 0), 3, "a reused grid keeps the previous wealth's result");
    temp.set(2, 0, 0);
    while fx.state() != state::HISTOGRAM_RESET {
        fx.step();
    }
    let g = fx.grids();
    for w in 0..3 {
        assert_eq!(all(g.proximity[w].as_ref().unwrap()), vec![0; 16], "proximity {w}");
    }
}

/// §15 test 4 / REH test 14: EQdecay = HQdecay = 0.2.
#[test]
fn t04_decay() {
    let fx = Fx::new(4);
    fx.set_tract(
        0,
        0,
        Tract {
            eq: [10.0, 0.1, 0.0],
            hq: [10.0, 0.1, 0.0],
            age: 30,
            ..steady([100, 100, 0])
        },
    );
    fx.s.do_population_change(0, 0);
    let (eq, hq) = (fx.eq(0, 0), fx.hq(0, 0));
    assert_eq!(eq[0], 9.8);
    assert_eq!(hq[0], 9.8);
    // EQ 0.1 → 0 (the residents are still there, so this is the clamp).
    assert_eq!(eq[1], 0.0);
    assert_eq!(hq[1], 0.0);
}

/// §15 test 5: (100, 0, 0) → (150, 0, 0), age 40, LE 100 → 35.
#[test]
fn t05_newcomers_age() {
    let fx = Fx::with(4, Some(tuned(|t| flat_le(t, 100.0))));
    fx.set_tract(
        0,
        0,
        Tract {
            old: [100, 0, 0],
            new: [150, 0, 0],
            eq: [50.0; 3],
            hq: [50.0; 3],
            age: 40,
        },
    );
    fx.s.do_population_change(0, 0);
    assert_eq!(fx.age(0, 0), 35);
    assert_eq!(fx.pop(0, 0), [150, 0, 0]);
}

/// §15 test 6: the age quirk, (100, 0, 0) → (50, 10, 0) → 71.
#[test]
fn t06_age_quirk() {
    let fx = Fx::with(4, Some(tuned(|t| flat_le(t, 100.0))));
    fx.set_tract(
        0,
        0,
        Tract {
            old: [100, 0, 0],
            new: [50, 10, 0],
            age: 40,
            ..Tract::default()
        },
    );
    fx.s.do_population_change(0, 0);
    assert_eq!(fx.age(0, 0), 71);
}

/// §15 test 7: December ages by one (mod 256); an empty tract has age 0.
/// RS2: the month is read per tract with total ≠ 0, never for an empty one.
#[test]
fn t07_december() {
    let fx = Fx::with(4, Some(tuned(|t| flat_le(t, 100.0))));
    fx.h.month.set(12);
    fx.set_tract(
        0,
        0,
        Tract {
            age: 40,
            ..steady([100, 0, 0])
        },
    );
    fx.s.do_population_change(0, 0);
    assert_eq!(fx.age(0, 0), 41);
    assert_eq!(fx.h.month_reads.get(), 1);

    fx.set_tract(
        0,
        0,
        Tract {
            age: 255,
            ..steady([100, 0, 0])
        },
    );
    fx.s.do_population_change(0, 0);
    assert_eq!(fx.age(0, 0), 0);

    fx.set_tract(
        0,
        0,
        Tract {
            old: [100, 0, 0],
            new: [0, 0, 0],
            age: 40,
            ..Tract::default()
        },
    );
    fx.s.do_population_change(0, 0);
    assert_eq!(fx.age(0, 0), 0);
    // The empty tract did not read the date (RS2).
    assert_eq!(fx.h.month_reads.get(), 2);

    // Not December: unchanged.
    fx.h.month.set(11);
    fx.set_tract(
        0,
        0,
        Tract {
            age: 40,
            ..steady([100, 0, 0])
        },
    );
    fx.s.do_population_change(0, 0);
    assert_eq!(fx.age(0, 0), 40);
}

/// RS2: a host that changes the month in the middle of a pass is seen by the
/// tracts after the change.
#[test]
fn rs2_month_read_per_tract() {
    let fx = Fx::with(8, Some(tuned(|t| flat_le(t, 100.0))));
    for (x, z) in [(0, 0), (1, 0)] {
        fx.set_tract(
            x,
            z,
            Tract {
                age: 40,
                ..steady([10, 0, 0])
            },
        );
    }
    fx.h.month.set(11);
    fx.s.do_population_change(0, 0);
    fx.h.month.set(12);
    fx.s.do_population_change(1, 0);
    assert_eq!((fx.age(0, 0), fx.age(1, 0)), (40, 41));
}

/// §15 test 8 (corrected for RS1 and RS7): EQ/HQ stored as 50.2 / 60.2
/// decay by 0.2f to exactly 50.0 / 60.0; the shipped curve gives LE 65 at HQ
/// 60, so leSum += 13000; age 70 > 65 → age 40, EQ and HQ times the float32
/// 0.79999995; the histogram adds trunc(EQ) = 39.
#[test]
fn t08_generational_reset() {
    let fx = Fx::new(4);
    let reset = fx.s.snapshot().population_tuning.eq_reset;
    assert_eq!(reset.to_bits(), 0x3F4C_CCCC);
    fx.set_tract(
        0,
        0,
        Tract {
            eq: [f32::from_bits(0x4248_CCCD), 0.0, 0.0],
            hq: [f32::from_bits(0x4270_CCCD), 0.0, 0.0],
            age: 70,
            ..steady([200, 0, 0])
        },
    );
    fx.s.do_population_change(0, 0);
    assert_eq!(fx.age(0, 0), 40);
    assert_eq!(fx.eq(0, 0)[0].to_bits(), 0x421F_FFFF);
    assert_eq!(fx.hq(0, 0)[0].to_bits(), 0x423F_FFFF);
    let s = fx.s.snapshot();
    assert_eq!(s.le_sum, 13_000);
    assert_eq!(age_bucket(40), 6);
    assert_eq!(s.age_count[6], 200);
    assert_eq!(s.age_eq_sum[6], 39 * 200);
    assert_eq!(s.age_eq_sum[6], 7800);
}

/// §15 test 9: the histogram buckets.
#[test]
fn t09_buckets() {
    for (age, b) in [
        (35, 5),
        (17, 19),
        (18, 0),
        (22, 0),
        (76, 18),
        (77, 19),
        (0, 19),
        (23, 1),
        (255, 19),
    ] {
        assert_eq!(age_bucket(age), b, "age {age}");
    }
}

/// §15 test 10: one bucket (age 30, b = 3), ageCount 200, ageEqSum 10000.
/// Each n needs the float32 rounding of the product (1200 × 0.01f = 12).
#[test]
fn t10_age_groups() {
    let fx = Fx::new(4);
    fx.s.with_state(|s| {
        s.age_count[3] = 200;
        s.age_eq_sum[3] = 10_000;
        s.state = state::POST_POPULATION;
    });
    assert_eq!(fx.step(), state::SCHOOL_COVERAGE);
    let pops = [12, 48, 54, 40, 24, 14, 8, 0, 0];
    let mut want = vec![(history::AVERAGE_LIFE_EXPECTANCY, 0)];
    for g in 0..9 {
        want.push((history::AGE_GROUP_POPULATION + g as u32, pops[g]));
        want.push((history::AGE_GROUP_EQ + g as u32, if pops[g] > 0 { 50 } else { 0 }));
    }
    assert_eq!(fx.h.history(), want);
}

/// §15 test 11: tracts (total 100, LE 61) and (total 300, LE 75) → 71,
/// through states 11, 4 and 12.
#[test]
fn t11_average_life_expectancy() {
    let fx = Fx::with(
        8,
        Some(tuned(|t| {
            t.add(id::HQ_TO_LIFE_EXPECTANCY, curve(&[0.0, 0.0, 200.0, 200.0]));
            no_decay(t);
        })),
    );
    fx.set_tract(
        0,
        0,
        Tract {
            hq: [61.0, 0.0, 0.0],
            age: 30,
            ..steady([100, 0, 0])
        },
    );
    fx.set_tract(
        1,
        0,
        Tract {
            hq: [75.0, 0.0, 0.0],
            age: 30,
            ..steady([300, 0, 0])
        },
    );
    fx.set_state(state::HISTOGRAM_RESET);
    assert_eq!(fx.step(), state::POPULATION);
    assert_eq!(fx.step(), state::POST_POPULATION);
    assert_eq!(fx.s.snapshot().le_sum, 28_600);
    assert_eq!(fx.step(), state::SCHOOL_COVERAGE);
    assert_eq!(fx.s.get_population(), 400);
    assert_eq!(fx.h.history()[0], (history::AVERAGE_LIFE_EXPECTANCY, 71));
}

/// §15 test 12 (REH test 11 is `reh11_…`): the per-mille history, rounded half to even,
/// with the served counters filled by the coverage states (5, 7): three
/// schools and one hospital of units 0 on a tract of population 1 count it
/// once each (REH §4.4 [quirk]), and their EQ / HQ boosts of 1.0 per building
/// leave the global values below 1.
#[test]
fn t12_per_mille() {
    let fx = Fx::new(4);
    fx.set_tract(0, 0, steady([1, 0, 0]));
    for occ in 1..=3 {
        service(&fx, occ, &SCHOOL, 0, 100, 10.0, (32.0, 32.0));
    }
    service(&fx, 4, &HOSPITAL, 0, 100, 10.0, (32.0, 32.0));
    fx.s.with_state(|s| {
        s.population = 2000;
        s.state = state::SCHOOL_COVERAGE;
    });
    while fx.state() != state::IDLE {
        fx.step();
    }
    let s = fx.s.snapshot();
    assert_eq!((s.school_covered, s.hospital_covered), (3, 1));
    assert_eq!(fx.eq(0, 0)[0], 3.0);
    assert_eq!(fx.hq(0, 0)[0], 1.0);
    assert_eq!(
        fx.h.history(),
        vec![
            (history::GLOBAL_EQ, 0),
            (history::GLOBAL_HQ, 0),
            (history::POPULATION, 2000),
            (history::SCHOOL_COVERAGE, 2),
            (history::HOSPITAL_COVERAGE, 0),
        ]
    );
    // Population 0: both 0, 0x4A8A3FE2 first.
    let fx = Fx::new(4);
    fx.s.with_state(|s| {
        s.school_covered = 7;
        s.state = state::HISTORY;
    });
    fx.step();
    assert_eq!(
        fx.h.history()[3..],
        [(history::HOSPITAL_COVERAGE, 0), (history::SCHOOL_COVERAGE, 0)]
    );
}

/// §15 test 13: the population sum wraps in 16 bits.
#[test]
fn t13_quotient_wrap() {
    let fx = Fx::new(4);
    fx.set_tract(
        0,
        0,
        Tract {
            old: [40_000, 30_000, 0],
            eq: [10.0; 3],
            ..Tract::default()
        },
    );
    let g = fx.grids();
    let q = ResidentialSim::tract_quotient(&g.eq, &g.population, 0, 0);
    assert_eq!(q, (700_000.0f64 / 4464.0) as f32);
    assert!((q - 156.810).abs() < 1e-3);
    assert_eq!(fx.s.get_cell_eq(3, 3), q);
}

/// §15 test 14 / RS3: 16 cells of air source 300 → p0 = 44; REH test 15:
/// average 600 → index 88, average 256 → index 0.
#[test]
fn t14_pollution_byte() {
    let fx = Fx::new(4);
    let t = fx.s.snapshot().population_tuning;
    fx.h.pollution.set([300, 0, 0]);
    let loss = fx.s.hq_loss(&t, 4, 0, 0);
    assert!((loss - 0.208_593_8).abs() < 1e-6, "{loss}");
    assert_eq!(loss, 0.2 + t.pollution_curves[0].eval(44.0));
    fx.h.pollution.set([600, 0, 0]);
    let loss = fx.s.hq_loss(&t, 4, 0, 0);
    assert!((loss - 0.217_187_5).abs() < 1e-6, "{loss}");
    fx.h.pollution.set([256, 0, 0]);
    assert_eq!(fx.s.hq_loss(&t, 4, 0, 0), 0.2);
    // Through the pass: HQ 10 loses 0.2085938.
    fx.h.pollution.set([300, 0, 0]);
    fx.set_tract(
        0,
        0,
        Tract {
            hq: [10.0, 0.0, 0.0],
            ..steady([100, 0, 0])
        },
    );
    fx.s.do_population_change(0, 0);
    let l = 0.2f32 + t.pollution_curves[0].eval(44.0);
    assert_eq!(fx.hq(0, 0)[0], 10.0 - l);
}

/// §15 test 15 / REH test 13: life expectancy from the shipped curve.
#[test]
fn t15_life_expectancy() {
    let fx = Fx::new(4);
    for (hq, le) in [(100.0, 75), (22.0, 55), (30.0, 57), (0.0, 50)] {
        fx.set_tract(
            0,
            0,
            Tract {
                hq: [hq, 0.0, 0.0],
                ..steady([1, 0, 0])
            },
        );
        assert_eq!(fx.s.get_cell_life_expectancy(0, 0), le, "HQ {hq}");
    }
    // No exemplar: 0.
    *fx.h.tuning.borrow_mut() = None;
    assert_eq!(fx.s.get_cell_life_expectancy(0, 0), 0);
    assert_eq!(fx.s.get_global_le(), 0);
}

/// §15 test 16: workforce, × 0.01f with one float32 rounding.
#[test]
fn t16_workforce() {
    let fx = Fx::new(4);
    fx.set_tract(
        0,
        0,
        Tract {
            hq: [100.0, 0.0, 0.0],
            ..steady([1, 0, 0])
        },
    );
    assert_eq!(fx.s.get_cell_workforce_percent(0, 0), 0.5);
    fx.s.with_state(|s| s.population = 1);
    assert_eq!(fx.s.get_global_le(), 75);
    assert_eq!(fx.s.get_global_workforce_percent(), 0.5);
    // HQ 44 → LE 61 → curve 44.4 → ≈ 0.444.
    fx.set_tract(
        0,
        0,
        Tract {
            hq: [44.0, 0.0, 0.0],
            ..steady([1, 0, 0])
        },
    );
    assert_eq!(fx.s.get_cell_life_expectancy(0, 0), 61);
    let w = fx.s.get_cell_workforce_percent(0, 0);
    assert!((w - 0.444).abs() < 1e-6, "{w}");
    // No exemplar: the static 0.0.
    *fx.h.tuning.borrow_mut() = None;
    assert_eq!(fx.s.get_cell_workforce_percent(0, 0), 0.0);
    assert_eq!(fx.s.get_global_workforce_percent(), 0.0);
}

/// §15 test 17: slot 42.
#[test]
fn t17_new_age() {
    let fx = Fx::new(4);
    assert_eq!(fx.s.get_average_new_age_by_wealth(3), 35);
    assert_eq!(fx.s.get_average_new_age_by_wealth(1), 25);
    assert_eq!(fx.s.get_average_new_age_by_wealth(2), 30);
    // [choice] outside 1..=3: 0.
    assert_eq!(fx.s.get_average_new_age_by_wealth(0), 0);
    assert_eq!(fx.s.get_average_new_age_by_wealth(4), 0);
    *fx.h.tuning.borrow_mut() = None;
    assert_eq!(fx.s.get_average_new_age_by_wealth(3), 25);
    assert_eq!(SLOTS[42], (42, "GetAverageNewAgeByWealth", 4, SlotReturn::Al));
}

/// §15 test 18: NewMonth.
#[test]
fn t18_new_month() {
    let fx = Fx::new(4);
    let m = ResidentialMessage::of_type(msg::SIM_NEW_MONTH);
    fx.s.with_state(|s| {
        s.state = state::IDLE;
        s.dirty = true;
    });
    assert!(fx.s.do_message(&m));
    let s = fx.s.snapshot();
    assert_eq!((s.state, s.dirty), (state::STRIKES_AND_LISTS, false));
    fx.s.with_state(|s| {
        s.state = state::SCHOOL_COVERAGE;
        s.dirty = false;
    });
    assert!(fx.s.do_message(&m));
    let s = fx.s.snapshot();
    assert_eq!((s.state, s.dirty), (state::SCHOOL_COVERAGE, true));
    // SimBegin, SimIdle and unknown messages: nothing, true.
    for t in [msg::SIM_BEGIN, msg::SIM_IDLE, 0x1234_5678] {
        assert!(fx.s.do_message(&ResidentialMessage::of_type(t)));
    }
    assert_eq!(fx.state(), state::SCHOOL_COVERAGE);
}

/// §15 test 19: a state-4 step whose budget runs out returns false and
/// resumes at the next tract; the result equals an uninterrupted pass.
#[test]
fn t19_new_day_unfinished() {
    let build = |budget: u32| {
        let fx = Fx::new(8);
        for (i, (x, z)) in [(0, 0), (1, 0), (0, 1), (1, 1)].into_iter().enumerate() {
            let p = 10 * (i as u16 + 1);
            fx.set_tract(
                x,
                z,
                Tract {
                    old: [p, 0, 0],
                    new: [p + 5, 0, 0],
                    eq: [20.0; 3],
                    hq: [20.0; 3],
                    age: 30,
                },
            );
        }
        fx.s.set_work_budget(budget);
        fx.set_state(state::POPULATION);
        fx
    };
    let a = build(DEFAULT_WORK_BUDGET);
    assert_eq!(a.step(), state::POST_POPULATION);

    let b = build(1);
    // x inner, z outer: (0, 0), (1, 0), (0, 1), (1, 1).
    for (done, (x, z)) in [(0, 0), (1, 0), (0, 1), (1, 1)].into_iter().enumerate() {
        assert!(!b.s.do_daily_simulation(), "tract {done}");
        assert_eq!(b.state(), state::POPULATION);
        assert_eq!(b.pop(x, z), a.pop(x, z));
        let rest = [(0, 0), (1, 0), (0, 1), (1, 1)];
        if let Some(&(nx, nz)) = rest.get(done + 1) {
            assert_ne!(b.pop(nx, nz), a.pop(nx, nz), "tract {done} + 1 not yet visited");
        }
    }
    assert!(b.s.do_daily_simulation());
    assert_eq!(b.state(), state::POST_POPULATION);
    for (x, z) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        assert_eq!(b.eq(x, z), a.eq(x, z));
        assert_eq!(b.hq(x, z), a.hq(x, z));
        assert_eq!(b.age(x, z), a.age(x, z));
    }
    let (sa, sb) = (a.s.snapshot(), b.s.snapshot());
    assert_eq!(sa.running_population, sb.running_population);
    assert_eq!(sa.le_sum, sb.le_sum);
    assert_eq!((sa.age_count, sa.age_eq_sum), (sb.age_count, sb.age_eq_sum));
}

/// §15 test 22: slot ABI.
#[test]
fn t22_slot_abi() {
    assert!(SLOTS.iter().enumerate().all(|(i, r)| r.0 == i as u32));
    assert_eq!(SLOTS[10], (10, "ChanceOfSchoolStrike", 4, SlotReturn::HiddenPtr));
    assert_eq!(SLOTS[23], (23, "GetEQGrids", 12, SlotReturn::Eax));
    assert_eq!(SLOTS[21], (21, "GetAverageEQGrid", 12, SlotReturn::Eax));
    assert_eq!(GRID_TRIPLE_RESULT, 1);

    let fx = Fx::new(4);
    // Slot 23: each call hands out the three grids with one more reference.
    let a = fx.s.get_eq_grids();
    let counts: Vec<usize> = a.iter().map(|g| Rc::strong_count(g.as_ref().unwrap())).collect();
    let b = fx.s.get_eq_grids();
    for w in 0..3 {
        assert_eq!(Rc::strong_count(b[w].as_ref().unwrap()), counts[w] + 1);
        assert_eq!(addr(a[w].as_ref().unwrap()), addr(b[w].as_ref().unwrap()));
    }
    assert_eq!(instance_of(a[1].as_ref().unwrap()), instance::EQ[1]);
    let p = fx.s.get_population_grids();
    assert_eq!(instance_of(p[2].as_ref().unwrap()), instance::POPULATION[2]);
    let hq = fx.s.get_hq_grids();
    assert_eq!(instance_of(hq[0].as_ref().unwrap()), instance::HQ[0]);

    // Slot 21: a new grid per call, owned by the caller alone.
    let (g1, _, _) = fx.s.average_quotient_grid(true);
    let (g2, _, _) = fx.s.average_quotient_grid(true);
    let (g1, g2) = (g1.unwrap(), g2.unwrap());
    assert_ne!(addr(&g1), addr(&g2));
    assert_eq!(Rc::strong_count(&g1), 1);
    assert_eq!(instance_of(&g1), instance::AVERAGE_EQ);
    let (g3, _, _) = fx.s.average_quotient_grid(false);
    assert_eq!(instance_of(g3.as_ref().unwrap()), instance::AVERAGE_HQ);
}

/// §15 test 23: buzzards draws.
#[test]
fn t23_buzzards_draws() {
    // HQ after decay 1.5 (< 2), residents: exactly one draw (0, 100).
    let fx = Fx::new(4);
    fx.set_tract(
        0,
        0,
        Tract {
            hq: [1.7, 0.0, 0.0],
            ..steady([100, 0, 0])
        },
    );
    fx.s.do_population_change(0, 0);
    assert_eq!(fx.hq(0, 0)[0], 1.5);
    assert_eq!(*fx.h.draws.borrow(), vec![(0.0, 100.0)]);
    assert!(!fx.h.take_events().contains(&Ev::Effect));

    // HQ exactly 2.0 after decay: no draw.
    let fx = Fx::new(4);
    fx.set_tract(
        0,
        0,
        Tract {
            hq: [2.2, 0.0, 0.0],
            ..steady([100, 0, 0])
        },
    );
    fx.s.do_population_change(0, 0);
    assert_eq!(fx.hq(0, 0)[0], 2.0);
    assert!(fx.h.draws.borrow().is_empty());

    // No residents before this pass: no draw, even with newcomers.
    let fx = Fx::new(4);
    fx.set_tract(
        0,
        0,
        Tract {
            old: [0, 0, 0],
            new: [100, 0, 0],
            hq: [1.7, 0.0, 0.0],
            ..Tract::default()
        },
    );
    fx.s.do_population_change(0, 0);
    assert!(fx.h.draws.borrow().is_empty());
}

/// §15 test 24 (RS1): the percentage getter's float32 `0.01f`.
#[test]
fn t24_percentage_getter() {
    let s = Fx::new(4).s.snapshot();
    assert_eq!(s.population_tuning.eq_reset.to_bits(), 0x3F4C_CCCC);
    assert_eq!(s.population_tuning.hq_reset.to_bits(), 0x3F4C_CCCC);
    for b in [s.health, s.education] {
        assert_eq!(b.strike_efficiency.to_bits(), 0x3E4C_CCCC);
        assert_eq!(b.max_funding.to_bits(), 0x3F99_9999);
        assert_eq!(b.min_grade.to_bits(), 0x3F66_6666);
        assert_eq!(b.init_funding, 0.25);
    }
    let fx = Fx::with(
        4,
        Some(tuned(|t| {
            t.remove(id::EQ_RESET);
            t.remove(id::HQ_RESET);
        })),
    );
    let t = fx.s.snapshot().population_tuning;
    assert_eq!(t.eq_reset.to_bits(), 0x3F33_3333);
    assert_eq!(t.hq_reset.to_bits(), 0x3F33_3333);
}

// ================================================================ REH §11

/// REH test 8 (the part this core does): slot 8 ends a city-level strike
/// with data2 = 0 and data3 = today − start.
#[test]
fn reh08_strike_end_message() {
    let fx = Fx::new(4);
    assert!(!fx.s.end_school_strike());
    assert!(!fx.s.end_health_strike());
    assert!(fx.h.take_events().is_empty());
    fx.s.with_state(|s| {
        s.education.strike_flag = 1;
        s.education.strike_start = 1000;
    });
    fx.h.today.set(1501);
    assert!(fx.s.school_is_on_strike());
    assert!(!fx.s.health_is_on_strike());
    assert!(fx.s.end_school_strike());
    assert_eq!(fx.h.take_events(), vec![Ev::Strike(msg::SCHOOL_STRIKE, None, 0, 501)]);
    let b = fx.s.snapshot().education;
    assert_eq!((b.strike_flag, b.strike_start), (0, 0));
    assert!(!fx.s.school_is_on_strike());

    fx.s.with_state(|s| {
        s.health.strike_flag = 1;
        s.health.strike_start = 1400;
    });
    assert!(fx.s.end_health_strike());
    assert_eq!(fx.h.take_events(), vec![Ev::Strike(msg::HEALTH_STRIKE, None, 0, 101)]);
}

/// REH test 9: two groups with chances 0.5 and 0.5 → 0.75; on strike →
/// 1.0. *[quirk]* A non-member reuses the previous c, and g carries over a
/// missing grade.
#[test]
fn reh09_chance_of_strike() {
    let fx = Fx::new(4);
    for (occ, i) in [(1, 0), (2, 1)] {
        striker(&fx, occ, &SCHOOL, i, 0.4);
    }
    assert_eq!(fx.s.chance_of_school_strike(), 0.75);
    assert_eq!(fx.s.chance_of_health_strike(), 0.0);
    fx.s.with_state(|s| s.education.strike_flag = 1);
    assert_eq!(fx.s.chance_of_school_strike(), 1.0);
    fx.s.with_state(|s| s.education.strike_flag = 0);

    // Group 0 gets a non-member (reuses c = 0.5) and a member without a grade
    // (g stays 0.4): q = 1 − 0.5³ = 0.875; with group 1: 1 − 0.125 × 0.5.
    service(&fx, 3, &SCHOOL, 0, 100, 10.0, (32.0, 32.0));
    striker(&fx, 4, &SCHOOL, 0, 0.4);
    fx.h.props.borrow_mut().remove(&(4, SCHOOL.grade));
    assert_eq!(fx.s.chance_of_school_strike(), 0.9375);
    // A non-member first reuses c = 0.
    let fx = Fx::new(4);
    service(&fx, 5, &HOSPITAL, 0, 100, 10.0, (32.0, 32.0));
    striker(&fx, 6, &HOSPITAL, 0, 0.4);
    assert_eq!(fx.s.chance_of_health_strike(), 0.5);
}

/// The monthly strike check runs in state 0 on the class's generator (one
/// draw per eligible non-striking building, REH §4.9), before the
/// ordinance-effect read.
#[test]
fn state0_strike_check_shares_the_generator() {
    let fx = Fx::new(4);
    fx.h.today.set(1234);
    striker(&fx, 31, &SCHOOL, 0, 0.4);
    striker(&fx, 32, &SCHOOL, 0, 0.4);
    fx.h.queue.borrow_mut().extend([10.0, 99.0]);
    assert_eq!(fx.step(), state::SEED_LOW);
    assert_eq!(*fx.h.draws.borrow(), vec![(0.0, 100.0), (0.0, 100.0)]);
    assert_eq!(
        fx.h.take_events(),
        vec![
            Ev::Controllers(STRIKING_EDUCATION.to_string(), 31, true),
            Ev::Strike(msg::SCHOOL_STRIKE, Some(31), 1, 0),
            Ev::Ordinance,
        ]
    );
    let b = fx.s.snapshot().education;
    assert_eq!((b.strike_flag, b.strike_start), (1, 1234));
    assert!(fx.s.school_is_on_strike());
    assert_eq!(fx.h.prop_u32(31, PROP_STRIKE_DAY), Some(1234));
    assert_eq!(fx.h.prop_u32(32, PROP_STRIKE_DAY), None);
}

/// REH test 12: tract quotient and the average grid.
#[test]
fn reh12_tract_quotient_and_average_grid() {
    let fx = Fx::new(4);
    fx.set_tract(
        0,
        0,
        Tract {
            old: [10, 20, 0],
            eq: [100.0, 50.0, 0.0],
            ..Tract::default()
        },
    );
    let q = fx.s.get_cell_eq(0, 0);
    assert_eq!(q, (2000.0f64 / 30.0) as f32);
    assert_eq!(q.to_bits(), 66.666_664f32.to_bits());
    let (g, min, max) = fx.s.average_quotient_grid(true);
    assert_eq!(g.unwrap().get(0, 0), 66.0);
    assert_eq!((min, max), (66.0, 66.0));
}

/// RS6: min and max go through every tract, populated or not.
#[test]
fn rs6_average_grid_min_max() {
    // The spec's example: a 1 × 2 grid, tract (0, 0) R$ 10 at EQ 100.0,
    // tract (0, 1) empty → values 100.0 and 0.0, min 0.0, max 100.0.
    let city = CityDims {
        cells_x: 4,
        cells_z: 8,
        cell_width_x: 16.0,
        cell_width_z: 16.0,
    };
    let fx = Fx::constructed(city, 4, Some(shipped_tuning()));
    assert!(fx.s.init());
    fx.set_tract(
        0,
        0,
        Tract {
            old: [10, 0, 0],
            eq: [100.0, 0.0, 0.0],
            ..Tract::default()
        },
    );
    let (g, min, max) = fx.s.average_quotient_grid(true);
    let g = g.unwrap();
    assert_eq!((g.count_x(), g.count_z()), (1, 2));
    assert_eq!((g.get(0, 0), g.get(0, 1)), (100.0, 0.0));
    assert_eq!((min, max), (0.0, 100.0));

    // An empty city: min = max = 0.0.
    let fx = Fx::new(8);
    assert_eq!(fx.s.average_quotient_grid(false).1, 0.0);
    assert_eq!(fx.s.average_quotient_grid(false).2, 0.0);

    // Only a grid with no tracts leaves 255.0 / 0.0.
    let fx = Fx::new(0);
    let (g, min, max) = fx.s.average_quotient_grid(true);
    assert!(g.unwrap().geometry().is_empty());
    assert_eq!((min, max), (255.0, 0.0));
}

/// RS5: one row seeded `[255, 0, 255, 255]`: the forward sweep gives
/// `[255, 0, 1, 2]` and the backward sweep `[1, 0, 1, 2]`.
#[test]
fn rs5_distance_does_not_wrap() {
    let city = CityDims {
        cells_x: 1,
        cells_z: 4,
        cell_width_x: 16.0,
        cell_width_z: 16.0,
    };
    let g = SimGrid::<u8>::create(&city, 0, 1);
    g.set_values(&[255, 0, 255, 255]);
    assert!(distance_pass(&*g, 1, 4));
    assert_eq!(g.values(), vec![1, 0, 1, 2]);
    assert!(!distance_pass(&*g, 1, 4));
}

/// REH test 16: the newcomer blend.
#[test]
fn reh16_newcomer_blend() {
    let tuning = || {
        Some(tuned(|t| {
            no_decay(t);
            flat_le(t, 100.0);
        }))
    };
    let fx = Fx::with(4, tuning());
    fx.set_tract(
        0,
        0,
        Tract {
            old: [100, 0, 0],
            new: [150, 0, 0],
            eq: [50.0, 0.0, 0.0],
            hq: [50.0, 0.0, 0.0],
            age: 30,
        },
    );
    fx.s.do_population_change(0, 0);
    assert_eq!(fx.eq(0, 0)[0], 45.0);
    assert_eq!(fx.hq(0, 0)[0], 45.0);

    let fx = Fx::with(4, tuning());
    fx.set_tract(
        0,
        0,
        Tract {
            old: [100, 0, 0],
            new: [150, 0, 0],
            age: 30,
            ..Tract::default()
        },
    );
    fx.s.do_population_change(0, 0);
    assert_eq!(fx.eq(0, 0)[0].to_bits(), 0x40D5_5555);
    assert_eq!(fx.eq(0, 0)[0], (1000.0f64 / 150.0) as f32);
}

/// The newcomer delta is int16(new − old): a rise of 40000 blends with
/// −25536 newcomers (REH §4.7 step 4).
#[test]
fn newcomer_delta_is_int16() {
    let fx = Fx::with(
        4,
        Some(tuned(|t| {
            no_decay(t);
            flat_le(t, 200.0);
        })),
    );
    fx.set_tract(
        0,
        0,
        Tract {
            old: [10_000, 0, 0],
            new: [50_000, 0, 0],
            eq: [50.0, 0.0, 0.0],
            hq: [50.0, 0.0, 0.0],
            age: 30,
        },
    );
    fx.s.do_population_change(0, 0);
    // avg = (50 + 20) / 2 = 35; (35 × −25536 + 10000 × 50) / 50000.
    let want = ((35.0f64 * -25_536.0) as f32 + 500_000.0f32) / 50_000.0;
    assert_eq!(fx.eq(0, 0)[0], want);
}

/// REH test 17 (RS1): age 60 > LE 57 → age 40, EQ 50 → 39.999996, HQ 30 →
/// 23.999998.
#[test]
fn reh17_generational_reset() {
    let fx = Fx::with(4, Some(tuned(no_decay)));
    fx.set_tract(
        0,
        0,
        Tract {
            eq: [50.0, 0.0, 0.0],
            hq: [30.0, 0.0, 0.0],
            age: 60,
            ..steady([100, 0, 0])
        },
    );
    assert_eq!(fx.s.get_cell_life_expectancy(0, 0), 57);
    fx.s.do_population_change(0, 0);
    assert_eq!(fx.age(0, 0), 40);
    assert_eq!(fx.eq(0, 0)[0].to_bits(), 0x421F_FFFF);
    assert_eq!(fx.hq(0, 0)[0].to_bits(), 0x41BF_FFFF);
}

/// REH test 22: min / max cell coordinates.
#[test]
fn reh22_min_max_coordinates() {
    let fx = Fx::new(32);
    assert_eq!(fx.s.quotient_min_max(true), None);
    fx.s.with_state(|s| s.population = 2);
    for (x, z, q) in [(2, 3, 10.0), (5, 1, 80.0)] {
        fx.set_tract(
            x,
            z,
            Tract {
                old: [1, 0, 0],
                eq: [q, 0.0, 0.0],
                ..Tract::default()
            },
        );
    }
    assert_eq!(fx.s.quotient_min_max(true), Some((10, 14, 22, 6, 10.0, 80.0)));
    // Global EQ (RS4): (10 + 80) / 2.
    assert_eq!(fx.s.get_global_eq(), 45.0);
}

/// REH §4.8 *[compat]* (AQ2, AQ3): which tract size each query uses, in a
/// hand-made save where the member E (8) differs from the grids' own size
/// (4), and the R$ HQ grid's size (2) from the R$ EQ grid's. Slots 35, 37
/// and 43 divide by the R$ EQ grid's `GetTractSize()`, slots 36, 38 and 44 by
/// the R$ HQ grid's; only slots 21 / 22 use E (the new grid's tract size).
#[test]
fn aq2_aq3_queries_ask_the_r1_grid_for_its_tract_size() {
    let fx = Fx::new(32);
    fx.s.with_state(|s| {
        s.eq_size = 8;
        s.population = 2;
    });
    // R$ population 1 at tract (1, 1) of the size-4 grids: cells 4..7.
    fx.set_tract(
        1,
        1,
        Tract {
            old: [1, 0, 0],
            eq: [50.0, 60.0, 70.0],
            ..Tract::default()
        },
    );
    assert_eq!(fx.s.get_cell_eq(5, 6), 50.0, "x / 4, not x / 8 (tract (0, 0))");
    assert_eq!(fx.s.get_cell_eq_by_wealth(5, 6, 2), 60.0);
    assert_eq!(fx.s.get_cell_eq_by_wealth(5, 6, 3), 70.0);
    // Coordinates T/2 + T × tract with T = 4: (2 + 4, 2 + 4).
    assert_eq!(fx.s.quotient_min_max(true), Some((6, 6, 6, 6, 50.0, 50.0)));
    // The new grid of slot 21 has tract size E = 8: 4 × 4 tracts.
    let (g, _, _) = fx.s.average_quotient_grid(true);
    let g = g.expect("a new grid");
    assert_eq!((g.geometry().tract_size, g.count_x(), g.count_z()), (8, 4, 4));

    // HQ: a size-2 R$ HQ grid (16 × 16 tracts); the EQ grids keep size 4.
    let hq1 = GridProvider::<f32>::create_grid(&*fx.h, instance::HQ[0], 2).unwrap();
    hq1.set(2, 3, 30.0);
    fx.s.with_state(|s| s.grids.hq[0] = Some(hq1));
    let hq2 = fx.grids().hq[1].clone().unwrap();
    // Tract (2, 3) of the size-4 R$$ HQ grid: cells 8..11, 12..15.
    hq2.set(2, 3, 40.0);
    // Cell (5, 7): R$ HQ tract (2, 3) by T = 2; R$ population read at the
    // same tract (2, 3) of its own grid (0 there) gives 0.0.
    assert_eq!(fx.s.get_cell_hq(5, 7), 0.0);
    fx.grids().population[0].as_ref().unwrap().set(2, 3, 1);
    assert_eq!(fx.s.get_cell_hq(5, 7), 30.0, "x / 2 (the R$ HQ grid), not / 4 or / 8");
    assert_eq!(fx.s.get_cell_hq_by_wealth(5, 7, 1), 30.0);
    assert_eq!(
        fx.s.get_cell_hq_by_wealth(5, 7, 2),
        40.0,
        "R$$ read at the R$ grid's tract"
    );
    // Slot 44's coordinates use T = 2: tract (2, 3) → (1 + 4, 1 + 6).
    fx.grids().population[0].as_ref().unwrap().set(1, 1, 0);
    let mm = fx.s.quotient_min_max(false).expect("populated");
    assert_eq!((mm.0, mm.1, mm.2, mm.3), (5, 7, 5, 7));
    // Slot 22 still uses E.
    let (g, _, _) = fx.s.average_quotient_grid(false);
    assert_eq!(g.expect("a new grid").geometry().tract_size, 8);
}

/// REH test 23: death's door.
#[test]
fn reh23_deaths_door() {
    // Draw 1.2 < 1.5: "buzzards" with a second draw for the height; one
    // effect per tract, so the second wealth does not roll.
    let fx = Fx::new(4);
    fx.h.queue.borrow_mut().extend([1.2, 3.0]);
    fx.set_tract(
        0,
        0,
        Tract {
            hq: [1.2, 1.2, 0.0],
            ..steady([100, 100, 0])
        },
    );
    fx.s.do_population_change(0, 0);
    assert_eq!(*fx.h.draws.borrow(), vec![(0.0, 100.0), (-10.0, 10.0)]);
    let ev = fx.h.take_events();
    let fx_ev: Vec<&Ev> = ev
        .iter()
        .filter(|e| matches!(e, Ev::Effect | Ev::Transform(_) | Ev::StartEffect(_)))
        .collect();
    // x = z = 0 × 64 + 64 × 0.5; y = (altitude 7 + height 50) + 3.
    assert_eq!(
        fx_ev,
        vec![&Ev::Effect, &Ev::Transform([32.0, 60.0, 32.0]), &Ev::StartEffect(500)]
    );

    // Draw 1.5: nothing, and the second wealth still draws.
    let fx = Fx::new(4);
    fx.h.queue.borrow_mut().extend([1.5, 1.5]);
    fx.set_tract(
        0,
        0,
        Tract {
            hq: [1.2, 1.2, 0.0],
            ..steady([100, 100, 0])
        },
    );
    fx.s.do_population_change(0, 0);
    assert_eq!(*fx.h.draws.borrow(), vec![(0.0, 100.0), (0.0, 100.0)]);
    assert!(!fx.h.take_events().contains(&Ev::Effect));

    // No effect created: nothing else, and the next wealth rolls again.
    let fx = Fx::new(4);
    fx.h.effect.set(CreatedEffect::Failed);
    fx.h.queue.borrow_mut().extend([1.0, 1.0]);
    fx.set_tract(
        0,
        0,
        Tract {
            hq: [1.2, 1.2, 0.0],
            ..steady([100, 100, 0])
        },
    );
    fx.s.do_population_change(0, 0);
    assert_eq!(*fx.h.draws.borrow(), vec![(0.0, 100.0), (0.0, 100.0)]);
    let ev = fx.h.take_events();
    assert_eq!(ev.iter().filter(|e| **e == Ev::Effect).count(), 2);
    assert!(!ev.iter().any(|e| matches!(e, Ev::Transform(_))));

    // RG1 [choice]: a host that cannot show effects counts as a create:
    // both draws, the flag set (the second wealth does not roll), nothing
    // positioned or started.
    let fx = Fx::new(4);
    fx.h.effect.set(CreatedEffect::NotShown);
    fx.h.queue.borrow_mut().extend([1.2, 3.0]);
    fx.set_tract(
        0,
        0,
        Tract {
            hq: [1.2, 1.2, 0.0],
            ..steady([100, 100, 0])
        },
    );
    fx.s.do_population_change(0, 0);
    assert_eq!(*fx.h.draws.borrow(), vec![(0.0, 100.0), (-10.0, 10.0)]);
    let ev = fx.h.take_events();
    assert_eq!(ev.iter().filter(|e| **e == Ev::Effect).count(), 1);
    assert!(!ev.iter().any(|e| matches!(e, Ev::Transform(_) | Ev::StartEffect(_))));
}

/// REH test 24: the slot ABI rows.
#[test]
fn reh24_slot_abi() {
    use SlotReturn::*;
    assert_eq!(SLOTS[34], (34, "GetGlobalLE", 0, Al));
    for slot in 10..=13 {
        assert_eq!((SLOTS[slot].2, SLOTS[slot].3), (4, HiddenPtr), "slot {slot}");
    }
    assert_eq!(SLOTS[14], (14, "GetSchoolSystemTotals", 8, Eax));
    assert_eq!(SLOTS[56], (56, "GetAutoBudget", 4, Al));
    assert_eq!(SLOTS[57], (57, "SetAutoBudget", 8, Eax));
    assert_eq!(SLOTS[30], (30, "GetCellWorkforcePercent", 12, HiddenPtr));
    assert_eq!(SLOTS[29].3, Al);
}

// ================================================================ lifecycle

fn init_events(created: bool) -> Vec<Ev> {
    let mut want = vec![
        Ev::Services,
        Ev::Developers,
        Ev::DemandNode,
        Ev::MessageServer,
        Ev::Subscribe(msg::OCCUPANT_INSERTED),
        Ev::Subscribe(msg::OCCUPANT_REMOVED),
        Ev::Budget,
        Ev::Simulator,
    ];
    for level in [0, 10, 6, 4] {
        want.push(Ev::AddAgent(level, AGENT_NAME.to_string(), 2));
    }
    for g in 0..9 {
        want.push(Ev::QueueLength(0xCA54_5CAB + g, 2));
        want.push(Ev::QueueLength(0xCA54_5CCA + g, 2));
    }
    want.push(Ev::QueueLength(0x4A8A_3FE2, 2));
    want.push(Ev::QueueLength(0x4A8A_3FE5, 2));
    if created {
        want.push(Ev::Ordinance);
        let max = f32::from_bits(0x3F99_9999);
        want.push(Ev::CreateDepartment(0x0918_8F42, department::HEALTH_NAME, max, 0.25));
        want.push(Ev::CreateDepartment(
            0xAA53_8CB3,
            department::HEALTH_SECONDARY_NAME,
            max,
            1.0,
        ));
        want.push(Ev::CreateDepartment(0x0918_8F4C, department::EDUCATION_NAME, max, 0.25));
        want.push(Ev::CreateDepartment(
            0x4A53_8CC6,
            department::EDUCATION_SECONDARY_NAME,
            max,
            1.0,
        ));
    }
    want
}

/// §4.1: the order of `Init`'s calls, the new-city sizes and the state it
/// leaves.
#[test]
fn init_order_and_new_city() {
    let fx = Fx::constructed(CityDims::square(8), 2, Some(shipped_tuning()));
    assert!(fx.s.init());
    assert_eq!(fx.h.take_events(), init_events(true));
    let s = fx.s.snapshot();
    assert!(s.initialized && s.city_started && s.message_server_held);
    // P = E = the R$ developer's population-map tract size.
    assert_eq!((s.population_size, s.eq_size, s.proximity_size), (2, 2, 1));
    assert_eq!(s.grids.population[0].as_ref().unwrap().geometry().tract_size, 2);
    assert_eq!(s.grids.eq[2].as_ref().unwrap().geometry().tract_size, 2);
    assert_eq!(s.grids.proximity[0].as_ref().unwrap().geometry().tract_size, 1);
    assert_eq!(instance_of(s.grids.age.as_ref().unwrap()), instance::AGE);
    assert_eq!(instance_of(s.grids.school_coverage[6].as_ref().unwrap()), 0x69D5_C30D);
    assert_eq!(instance_of(s.grids.hospital_grade.as_ref().unwrap()), 0x69D5_C330);
    assert_eq!((s.state, s.population, s.running_population, s.le_sum), (0, 0, 0, 0));
    assert_eq!(
        (
            s.health.department1,
            s.health.department2,
            s.education.department1,
            s.education.department2
        ),
        (Some(900), Some(901), Some(902), Some(903))
    );
    // The iterator covers the 4 × 4 population tracts, fresh at (0, 0).
    assert_eq!(s.iterator, {
        let mut it = ScanIterator::for_counts(4, 4);
        it.reset(0, 0);
        it
    });
    assert!(s.tuning_loaded);

    // A second Init: no new subscription, tuning, ordinance build or
    // department; the agent registration and the queue lengths repeat.
    assert!(fx.s.init());
    assert_eq!(fx.h.take_events(), {
        let mut w = init_events(false);
        w.retain(|e| !matches!(e, Ev::MessageServer | Ev::Subscribe(_)));
        w
    });
}

/// §4.1 steps 1, 2 and 4: `Init` fails without the demand node, the message
/// server or the simulator; a held message server is not taken twice.
#[test]
fn init_failures_and_subscribe_once() {
    let fx = Fx::constructed(CityDims::square(4), 4, Some(shipped_tuning()));
    fx.h.demand_node.set(false);
    assert!(!fx.s.init());
    assert_eq!(fx.h.take_events(), vec![Ev::Services, Ev::Developers, Ev::DemandNode]);
    fx.h.demand_node.set(true);
    fx.h.message_server.set(false);
    assert!(!fx.s.init());
    assert!(!fx.h.take_events().iter().any(|e| matches!(e, Ev::Subscribe(_))));
    fx.h.message_server.set(true);
    fx.h.simulator.set(false);
    assert!(!fx.s.init());
    let ev = fx.h.take_events();
    assert_eq!(ev.iter().filter(|e| matches!(e, Ev::Subscribe(_))).count(), 2);
    assert!(!fx.s.snapshot().initialized);
    // The message server is held now: no second subscription.
    fx.h.simulator.set(true);
    assert!(fx.s.init());
    let ev = fx.h.take_events();
    assert!(!ev.iter().any(|e| matches!(e, Ev::MessageServer | Ev::Subscribe(_))));
    assert!(fx.s.snapshot().initialized);
}

/// §4.2: `Shutdown`'s release order; a second call does nothing.
#[test]
fn shutdown_order() {
    let fx = Fx::new(4);
    fx.s.with_state(|s| {
        s.lists.pending_school = vec![Some(41), None];
        s.lists.health_affecting = vec![Some(42)];
    });
    assert!(fx.s.shutdown());
    let mut want = vec![
        Ev::RemoveAgent,
        Ev::Unsubscribe(msg::OCCUPANT_INSERTED),
        Ev::Unsubscribe(msg::OCCUPANT_REMOVED),
        Ev::ReleaseMessageServerAndBudget,
        Ev::ReleaseDepartment(900),
        Ev::ReleaseDepartment(901),
        Ev::ReleaseDepartment(902),
        Ev::ReleaseDepartment(903),
    ];
    let grids = instance::PROXIMITY
        .iter()
        .chain(&instance::POPULATION)
        .chain([&instance::AGE])
        .chain(&instance::SCHOOL_COVERAGE)
        .chain(&instance::HOSPITAL_COVERAGE)
        .chain([&instance::SCHOOL_GRADE, &instance::HOSPITAL_GRADE])
        .chain(&instance::EQ)
        .chain(&instance::HQ);
    want.extend(grids.map(|&i| Ev::ShutdownGrid(i)));
    want.push(Ev::ReleaseServices);
    want.push(Ev::Release(41));
    want.push(Ev::Release(42));
    assert_eq!(fx.h.take_events(), want);
    let s = fx.s.snapshot();
    assert!(!s.initialized && !s.message_server_held);
    assert!(s.grids.eq[0].is_none() && s.health.department1.is_none());
    assert_eq!(s.lists, OccupantLists::default());
    assert!(fx.s.shutdown());
    assert!(fx.h.take_events().is_empty());
    // Init again takes the message server and subscribes again.
    assert!(fx.s.init());
    assert!(fx.h.take_events().contains(&Ev::Subscribe(msg::OCCUPANT_INSERTED)));
}

/// §1: `AddRef`, `Release`, `QueryInterface`.
#[test]
fn identity_and_reference_count() {
    let fx = Fx::new(4);
    assert_eq!(fx.s.add_ref(), 1);
    assert_eq!(fx.s.add_ref(), 2);
    assert_eq!(fx.s.release(), 1);
    assert!(!fx.s.snapshot().destroyed);
    assert!(fx.h.take_events().is_empty());
    assert_eq!(fx.s.release(), 0);
    let s = fx.s.snapshot();
    assert!(s.destroyed && !s.initialized);
    assert!(fx.h.take_events().contains(&Ev::RemoveAgent));
    for iid in [1, 0x077A_C1EE, 0x4522_94AA, 0xE4FD_A3D4] {
        assert!(query_interface(iid));
    }
    assert!(!query_interface(0x1234_5678));
    assert_eq!(fx.s.class_id(), CLSID_RESIDENTIAL_SIMULATOR);
}

/// §5.1: one state per call, in the cycle order; state 10 waits.
#[test]
fn states_follow_the_cycle_order() {
    let fx = Fx::new(8);
    let mut seen = vec![fx.state()];
    for _ in 0..15 {
        seen.push(fx.step());
    }
    assert_eq!(seen, state::ORDER.to_vec());
    assert_eq!(fx.step(), state::IDLE);
    // [choice] an unknown state waits too.
    fx.set_state(99);
    assert_eq!(fx.step(), 99);
}

/// §7.1: the population tuning is re-read at the first state-4 call of each
/// cycle, with the decays scaled by the ordinance decay products (REH §4.3:
/// school 50 % and health 200 %, read at `Init`).
#[test]
fn population_tuning_reread_with_ordinance_products() {
    let fx = Fx::constructed(CityDims::square(4), 4, Some(shipped_tuning()));
    fx.h.enact(
        0x10,
        props(&[(SCHOOL.ord_decay, f(50.0)), (HOSPITAL.ord_decay, f(200.0))]),
    );
    assert!(fx.s.init());
    assert_eq!(fx.s.ordinance_decay_products(), (0.5, 2.0));
    *fx.h.tuning.borrow_mut() = Some(Rc::new(tuned(|t| t.add(id::EQ_DECAY, f(1.0)))));
    fx.set_tract(
        0,
        0,
        Tract {
            eq: [10.0; 3],
            hq: [10.0; 3],
            age: 30,
            ..steady([100, 0, 0])
        },
    );
    fx.set_state(state::HISTOGRAM_RESET);
    assert_eq!(fx.step(), state::POPULATION);
    assert!(!fx.s.snapshot().tuning_loaded);
    assert_eq!(fx.step(), state::POST_POPULATION);
    let t = fx.s.snapshot().population_tuning;
    assert_eq!((t.eq_decay, t.hq_decay), (0.5, 0.4));
    assert_eq!(fx.eq(0, 0)[0], 9.5);
    assert_eq!(fx.hq(0, 0)[0], 9.6);
}

/// State 0 distributes the pending lists (REH §4.1) and moves the other
/// list; slots 46–49 and 61.
#[test]
fn state0_lists_and_building_slots() {
    let fx = Fx::new(4);
    fx.h.groups.borrow_mut().extend([
        (1, vec![0x1510]),
        // In two groups: the first in table order (0x150F) wins.
        (2, vec![0x151B, 0x150F]),
        (3, vec![0x9999]),
        (4, vec![0x1513]),
    ]);
    fx.s.with_state(|s| {
        s.lists.pending_school = vec![Some(1), Some(2), None, Some(3)];
        s.lists.pending_hospital = vec![Some(4)];
        s.lists.other = vec![Some(5)];
        s.lists.health_affecting = vec![Some(6)];
    });
    assert_eq!(fx.s.school_building_count(), 4);
    assert_eq!(fx.s.hospital_building_count(), 1);
    assert_eq!(fx.step(), state::SEED_LOW);
    let l = fx.s.snapshot().lists;
    assert_eq!(l.school_groups[0], vec![Some(2)]);
    assert_eq!(l.school_groups[1], vec![Some(1)]);
    assert!(l.pending_school.is_empty() && l.pending_hospital.is_empty());
    assert_eq!(l.hospital_groups[1], vec![Some(4)]);
    assert!(l.other.is_empty());
    assert_eq!(l.health_affecting, vec![Some(6), Some(5)]);
    // Occupant 3 is in no group: dropped without Release [quirk].
    assert!(!fx.h.take_events().contains(&Ev::Release(3)));
    assert_eq!(fx.s.school_building_count(), 2);
    assert_eq!(fx.s.hospital_building_count(), 1);

    // Slot 47: the selected groups in table order, then the pending list,
    // each AddRef'ed.
    fx.s.with_state(|s| s.lists.pending_school.push(Some(7)));
    assert_eq!(fx.s.school_buildings(None), vec![2, 1, 7]);
    assert_eq!(fx.s.school_buildings(Some(&[0x1510][..])), vec![1, 7]);
    assert_eq!(fx.s.school_buildings(Some(&[][..])), vec![7]);
    assert_eq!((fx.h.ref_count(1), fx.h.ref_count(2), fx.h.ref_count(7)), (2, 1, 3));
    assert_eq!(fx.s.hospital_buildings(None), vec![4]);
    // Slot 61: the health-affecting list, then the other list.
    fx.s.with_state(|s| s.lists.other = vec![Some(8), None]);
    assert_eq!(fx.s.health_affecting_buildings(), vec![6, 5, 8]);
    assert_eq!(fx.h.ref_count(8), 1);
}

/// §4.3: occupant messages reach `DoBuildingChanged` only for buildings; a
/// school recognised by its group (no budget item, REH §4.1) is added to
/// the pending list with one reference and removed with its release.
#[test]
fn occupant_messages() {
    let fx = Fx::new(4);
    fx.h.building(8, &[0x1503]);
    fx.h.set_prop(8, SCHOOL.capacity, u(1500));
    fx.s.with_state(|s| s.dirty = false);
    let mut m = ResidentialMessage {
        msg_type: msg::OCCUPANT_INSERTED,
        data3: OCCUPANT_TYPE_BUILDING,
        void1: Some(8),
        ..ResidentialMessage::default()
    };
    assert!(fx.s.do_message(&m));
    assert_eq!(fx.h.take_events(), vec![Ev::AddRef(8)]);
    let s = fx.s.snapshot();
    assert_eq!(s.lists.pending_school, vec![Some(8)]);
    assert!(s.lists.pending_hospital.is_empty());
    assert!(s.dirty);
    assert_eq!(fx.h.prop_u32(8, SCHOOL.served), Some(NOT_GRADED));
    assert_eq!(fx.h.prop_u32(8, SCHOOL.effective_capacity), Some(1500));
    assert_eq!(fx.h.prop_f32(8, SCHOOL.grade), Some(1.0));
    assert_eq!(fx.h.prop_f32(8, SCHOOL.staff_ratio), Some(1.0));
    // Removed: erased from the pending list, released, dirty.
    fx.s.with_state(|s| s.dirty = false);
    m.msg_type = msg::OCCUPANT_REMOVED;
    assert!(fx.s.do_message(&m));
    assert_eq!(fx.h.take_events(), vec![Ev::Release(8)]);
    let s = fx.s.snapshot();
    assert!(s.lists.pending_school.is_empty() && s.dirty);
    // Another occupant type, or an occupant that is not a building: nothing.
    fx.s.with_state(|s| s.dirty = false);
    m.data3 = 0x1234;
    assert!(fx.s.do_message(&m));
    m.data3 = OCCUPANT_TYPE_BUILDING;
    m.void1 = Some(9);
    assert!(fx.s.do_message(&m));
    m.void1 = None;
    assert!(fx.s.do_message(&m));
    assert!(fx.h.take_events().is_empty());
    assert!(!fx.s.snapshot().dirty);
    // A building that is no service: only the slot-11 test (REH §4.1 step 4).
    fx.h.building(10, &[0x9999]);
    m.void1 = Some(10);
    assert!(fx.s.do_message(&m));
    assert!(!fx.s.snapshot().dirty);
    fx.h.slot11.borrow_mut().insert(10, 1);
    assert!(fx.s.do_message(&m));
    assert!(fx.s.snapshot().dirty);
    assert!(fx.h.take_events().is_empty());
}

/// Slots 5, 17/18, 19, 37/38, 50–57.
#[test]
fn small_slots() {
    let fx = Fx::new(8);
    assert!(fx.s.get_proximity_map(0).is_none());
    assert!(fx.s.get_proximity_map(4).is_none());
    assert_eq!(instance_of(&fx.s.get_proximity_map(2).unwrap()), instance::PROXIMITY[1]);
    assert_eq!(fx.s.max_quotient(), 200);
    assert_eq!(fx.s.total_city_upkeep_cost(), 0);
    assert_eq!(instance_of(&fx.s.average_age_map().unwrap()), instance::AGE);
    assert_eq!(instance_of(&fx.s.school_grade_map().unwrap()), instance::SCHOOL_GRADE);
    assert_eq!(
        instance_of(&fx.s.hospital_grade_map().unwrap()),
        instance::HOSPITAL_GRADE
    );

    // 37/38: cell → tract by unsigned division by E; wealth 0 or > 3 → 0.0.
    fx.set_tract(
        1,
        1,
        Tract {
            eq: [1.0, 2.0, 3.0],
            hq: [4.0, 5.0, 6.0],
            ..steady([1, 1, 1])
        },
    );
    assert_eq!(fx.s.get_cell_eq_by_wealth(5, 6, 2), 2.0);
    assert_eq!(fx.s.get_cell_hq_by_wealth(7, 4, 3), 6.0);
    assert_eq!(fx.s.get_cell_eq_by_wealth(5, 6, 0), 0.0);
    assert_eq!(fx.s.get_cell_eq_by_wealth(5, 6, 4), 0.0);
    assert_eq!(fx.s.get_cell_eq_by_wealth(-4, 6, 2), 0.0);

    // 52–55.
    assert!(!fx.s.global_auto_budget_schools());
    assert!(fx.s.set_global_auto_budget_schools(true));
    assert!(fx.s.global_auto_budget_schools());
    assert!(!fx.s.set_global_auto_budget_hospitals(false));
    assert!(!fx.s.global_auto_budget_hospitals());
    // 56/57: the occupant's bool property 0x6C05FC4D, default false.
    assert!(!fx.s.get_auto_budget(5));
    assert!(fx.s.set_auto_budget(5, true));
    assert!(fx.s.get_auto_budget(5));
    assert_eq!(fx.h.bool_props.borrow().get(&(5, 0x6C05_FC4D)), Some(&true));

    // 19: no budget item sets the dirty flag; true.
    fx.s.with_state(|s| s.dirty = false);
    assert!(fx.s.set_occupant_funding(5, 1.0, 1.0, true));
    assert!(fx.s.snapshot().dirty);
    // The education/health values of a city without service buildings.
    assert_eq!(fx.s.occupant_funding(5, true), (1.0, 1.0));
    assert_eq!(fx.s.school_system_rating(), 0.0);
    assert_eq!(fx.s.hospital_system_totals(&[0x1512], &mut QueryData::default()), 0);
    assert!(fx.s.school_query_data(5).is_none());
    assert_eq!(fx.s.estimate_occupant_capacity(5), (0, 0));
    assert!(fx.s.occupant_coverage(5, 0.5).is_none());
    assert!(fx.s.estimate_ideal_funding(5).is_none());
}

/// A month end to end: the population is copied from the developers, and
/// states 12 and 9 record their history in order.
#[test]
fn a_month_end_to_end() {
    let fx = Fx::new(8);
    fx.set_tract(
        0,
        0,
        Tract {
            old: [0, 0, 0],
            new: [100, 20, 0],
            age: 0,
            ..Tract::default()
        },
    );
    while fx.state() != state::IDLE {
        fx.step();
    }
    assert_eq!(fx.s.get_population(), 120);
    assert_eq!(fx.pop(0, 0), [100, 20, 0]);
    // Newcomers: EQ = the low and medium wealth bonuses 20 and 40; age =
    // (100 × 25 + 20 × 30) / 120 = 25.
    assert_eq!(fx.eq(0, 0), [20.0, 40.0, 0.0]);
    assert_eq!(fx.age(0, 0), 25);
    let h = fx.h.history();
    assert_eq!(h.len(), 1 + 18 + 5);
    assert_eq!(h[0].0, history::AVERAGE_LIFE_EXPECTANCY);
    assert_eq!(h[1].0, history::AGE_GROUP_POPULATION);
    assert_eq!(h[2].0, history::AGE_GROUP_EQ);
    let tail: Vec<u32> = h[19..].iter().map(|e| e.0).collect();
    assert_eq!(
        tail,
        vec![
            history::GLOBAL_EQ,
            history::GLOBAL_HQ,
            history::POPULATION,
            history::SCHOOL_COVERAGE,
            history::HOSPITAL_COVERAGE
        ]
    );
    // Global EQ: (20 × 100 + 40 × 20) / 120 = 23.33 → 23.
    assert_eq!(h[19].1, 23);
    assert_eq!(h[21].1, 120);
}

// ================================================================ the record (§9)

/// Resolves references like the class-object store: host objects come with
/// a reference for the reader.
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

fn record(s: &ResidentialSim) -> (Vec<u8>, Vec<RegisteredObject>) {
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(reg.clone());
    assert!(s.write(&mut w));
    let entries = reg.borrow().entries.clone();
    (w.into_bytes(), entries)
}

fn load(fx: &Fx, bytes: &[u8], entries: &[RegisteredObject]) -> bool {
    let mut res = Resolver(entries.to_vec(), fx.h.clone());
    fx.s.read(&mut ByteReader::with_resolver(bytes, &mut res))
}

/// A record with a different version and length.
fn as_version(bytes: &[u8], version: u16, len: usize) -> Vec<u8> {
    let mut v = bytes[..len].to_vec();
    v[..2].copy_from_slice(&version.to_le_bytes());
    v
}

/// Byte layout of a version-7 record whose grids and departments are all
/// present and whose lists are empty (§9.1): references are 8 bytes (object
/// ID, class ID), department blocks 53, the iterator 29.
const STATE_AT: usize = 2 + 4 + 3 * 8 + 4 + 3 * 8 + 8 + (4 + 7 * 8) + (4 + 3 * 8) + 2 * 8 + 4 + 6 * 8 + 2 * 4;
const V2_LEN: usize = STATE_AT + 4 + 1 + 4 + 2 * 53 + 3 * 4 + (4 + 7 * 4) + (4 + 3 * 4) + 4;
const V3_LEN: usize = V2_LEN + 29 + 8;
const V4_LEN: usize = V3_LEN + 4;
const V7_LEN: usize = V4_LEN + 2;

#[test]
fn record_layout_and_sizes() {
    assert_eq!((STATE_AT, V2_LEN, V3_LEN, V4_LEN, V7_LEN), (230, 409, 446, 450, 452));
    let fx = Fx::new(8);
    fx.s.with_state(|s| {
        s.state = state::POPULATION;
        s.dirty = false;
        s.population = 0x0102_0304;
    });
    let (bytes, entries) = record(&fx.s);
    assert_eq!(bytes.len(), V7_LEN);
    assert_eq!(bytes[..2], 7u16.to_le_bytes());
    assert_eq!(bytes[STATE_AT..STATE_AT + 4], 4u32.to_le_bytes());
    // dirty ∨ state ≠ 10.
    assert_eq!(bytes[STATE_AT + 4], 1);
    assert_eq!(bytes[STATE_AT + 5..STATE_AT + 9], 0x0102_0304i32.to_le_bytes());
    // 25 grids and 4 departments.
    assert_eq!(entries.len(), 29);
    assert_eq!(entries.iter().filter(|e| e.2.is_none()).count(), 4);

    fx.s.with_state(|s| s.state = state::IDLE);
    assert_eq!(record(&fx.s).0[STATE_AT + 4], 0);
    fx.s.with_state(|s| s.dirty = true);
    assert_eq!(record(&fx.s).0[STATE_AT + 4], 1);

    // Lists: u32 count, then the references (a null one is 4 bytes).
    fx.s.with_state(|s| {
        s.lists.pending_school = vec![Some(11)];
        s.lists.other = vec![Some(12), None];
        s.lists.school_groups[2] = vec![Some(13)];
        s.lists.health_affecting = vec![Some(14)];
    });
    assert_eq!(record(&fx.s).0.len(), V7_LEN + 4 * 8 + 4);
}

#[test]
fn record_round_trip() {
    let a = Fx::new(8);
    a.s.with_state(|s| {
        s.state = state::SCHOOL_GRADING;
        s.population = 1234;
        s.running_population = 99;
        s.le_sum = 0x1_0000_0005;
        s.unused_float1 = 1.5;
        s.unused_float2 = -2.0;
        s.health.strike_flag = 1;
        s.health.strike_start = 777;
        s.education.min_grade = 0.5;
        s.lists.pending_school = vec![Some(11)];
        s.lists.other = vec![Some(12), None];
        s.lists.school_groups[2] = vec![Some(13)];
        s.lists.hospital_groups[0] = vec![Some(15)];
        s.lists.health_affecting = vec![Some(14)];
        s.iterator.set_step(3);
        s.iterator.next_cell();
        s.iterator.next_cell();
        s.school_auto_budget = true;
    });
    a.grids().eq[0].as_ref().unwrap().set(1, 1, 42.0);
    let (bytes, entries) = record(&a.s);

    // Into a constructed object, as the class store does. The null entry of
    // the other list is dropped (RS12).
    let b = Fx::constructed(CityDims::square(8), 4, Some(shipped_tuning()));
    assert!(load(&b, &bytes, &entries));
    a.s.with_state(|s| s.lists.other.retain(Option::is_some));
    let (sa, sb) = (a.s.snapshot(), b.s.snapshot());
    assert_eq!(sb.lists.other, vec![Some(12)]);
    assert_eq!(sb.lists, sa.lists);
    assert_eq!(sb.health, sa.health);
    assert_eq!(sb.education, sa.education);
    assert_eq!(sb.iterator, sa.iterator);
    assert_eq!(
        (sb.state, sb.population, sb.running_population, sb.le_sum),
        (state::SCHOOL_GRADING, 1234, 99, 0x1_0000_0005)
    );
    assert_eq!((sb.unused_float1, sb.unused_float2), (1.5, -2.0));
    assert!(sb.school_auto_budget && !sb.hospital_auto_budget);
    assert!(sb.city_started && !sb.tuning_loaded && !sb.initialized);
    assert_eq!(b.eq(1, 1)[0], 42.0);
    // Every occupant and department reference came with one reference.
    for t in [11, 12, 13, 14, 15, 900, 901, 902, 903] {
        assert_eq!(b.h.ref_count(t), 1, "token {t}");
    }
    // Written again: the record without the null entry.
    assert_eq!(record(&b.s).0.len(), bytes.len() - 4);
    assert_eq!(record(&b.s).0, record(&a.s).0);
}

/// §15 test 20: version fix-ups; every version is read (RS9), and a record
/// cut short returns false and leaves the simulator as constructed.
#[test]
fn t20_read_versions() {
    let a = Fx::new(8);
    a.s.with_state(|s| {
        s.state = state::POPULATION;
        s.le_sum = 500;
        s.running_population = 60;
        s.school_auto_budget = true;
    });
    let (bytes, entries) = record(&a.s);
    assert_eq!(bytes.len(), V7_LEN);

    // Version 2, state 4 → 11, leSum 0, running population 0, a fresh
    // iterator at (0, 0); the auto-budget flags 0.
    let b = Fx::new(8);
    b.s.with_state(|s| {
        s.le_sum = 9;
        s.running_population = 9;
        s.iterator.next_cell();
        s.iterator.next_cell();
        s.school_auto_budget = true;
    });
    assert!(load(&b, &as_version(&bytes, 2, V2_LEN), &entries));
    let s = b.s.snapshot();
    assert_eq!(
        (s.state, s.le_sum, s.running_population),
        (state::HISTOGRAM_RESET, 0, 0)
    );
    assert_eq!((s.iterator.state, s.iterator.x, s.iterator.z), (ScanState::Fresh, 0, 0));
    assert!(!s.school_auto_budget);

    // Version 3: the iterator and leSum are read; the running population
    // keeps the constructed value.
    let b = Fx::constructed(CityDims::square(8), 4, None);
    assert!(load(&b, &as_version(&bytes, 3, V3_LEN), &entries));
    let s = b.s.snapshot();
    assert_eq!((s.state, s.le_sum, s.running_population), (state::POPULATION, 500, 0));

    // Version 4–6: the running population; no auto-budget bytes.
    for v in 4..=6 {
        let b = Fx::constructed(CityDims::square(8), 4, None);
        assert!(load(&b, &as_version(&bytes, v, V4_LEN), &entries));
        let s = b.s.snapshot();
        assert_eq!((s.running_population, s.school_auto_budget), (60, false), "version {v}");
    }

    // Version 7: state 1 → 13, 2 → 14, 3 → 15.
    for (from, to) in [(1u32, 13), (2, 14), (3, 15), (4, 4), (10, 10)] {
        let mut v7 = bytes.clone();
        v7[STATE_AT..STATE_AT + 4].copy_from_slice(&from.to_le_bytes());
        let b = Fx::constructed(CityDims::square(8), 4, None);
        assert!(load(&b, &v7, &entries));
        assert_eq!(b.state(), to, "state {from}");
    }

    // Any version with a complete record returns true.
    for v in [0u16, 1, 5, 6, 8, 0xFFFF] {
        let b = Fx::constructed(CityDims::square(8), 4, None);
        assert!(load(&b, &as_version(&bytes, v, V7_LEN), &entries), "version {v}");
        assert!(b.s.snapshot().city_started);
    }
    // A record cut short: false, as constructed, and every reference the
    // parse acquired is released (RS9 [choice]).
    for n in 0..bytes.len() {
        let b = Fx::constructed(CityDims::square(8), 4, None);
        assert!(!load(&b, &bytes[..n], &entries), "length {n}");
        assert_constructed(&b);
        for t in 900..904 {
            assert_eq!(b.h.ref_count(t), 0, "length {n}, department {t}");
        }
    }
}

/// §9.2: department blocks of version < 5 / < 6 records are percentages,
/// converted with the float32 `0.01f` (RS1).
#[test]
fn read_converts_old_department_blocks() {
    let a = Fx::new(4);
    a.s.with_state(|s| {
        for b in [&mut s.health, &mut s.education] {
            b.f0 = 60.0;
            b.max_funding = 120.0;
            b.strike_efficiency = 20.0;
            b.min_grade = 90.0;
            b.init_funding = 25.0;
        }
    });
    let (bytes, entries) = record(&a.s);
    let read = |v: u16| {
        let b = Fx::constructed(CityDims::square(4), 4, None);
        assert!(load(&b, &as_version(&bytes, v, V4_LEN), &entries));
        b.s.snapshot()
    };
    let s = read(4);
    for b in [s.health, s.education] {
        assert_eq!(b.f0, pct(60.0));
        assert_eq!(b.max_funding.to_bits(), 0x3F99_9999);
        assert_eq!(b.strike_efficiency.to_bits(), 0x3E4C_CCCC);
        assert_eq!(b.min_grade.to_bits(), 0x3F66_6666);
        assert_eq!(b.init_funding, 0.25);
        assert_eq!(b.hundred, 100.0);
    }
    let s = read(5);
    assert_eq!(
        (s.health.f0, s.health.max_funding, s.health.init_funding),
        (60.0, 120.0, 25.0)
    );
    assert_eq!(s.health.min_grade.to_bits(), 0x3F66_6666);
    let s = read(6);
    assert_eq!((s.education.min_grade, s.education.strike_efficiency), (90.0, 20.0));
}

/// §9.2 [choice]: references the record replaces are released.
#[test]
fn read_releases_what_it_replaces() {
    let a = Fx::new(4);
    a.s.with_state(|s| s.lists.pending_school = vec![Some(11)]);
    let (bytes, entries) = record(&a.s);
    let b = Fx::constructed(CityDims::square(4), 4, Some(shipped_tuning()));
    b.h.next_department.set(950);
    assert!(b.s.init());
    b.s.with_state(|s| s.lists.other = vec![Some(77), None]);
    b.h.take_events();
    assert!(load(&b, &bytes, &entries));
    let ev = b.h.take_events();
    // Departments are released as `Shutdown` releases them, then the list
    // entries.
    let released: Vec<&Ev> = ev
        .iter()
        .filter(|e| matches!(e, Ev::Release(_) | Ev::ReleaseDepartment(_)))
        .collect();
    assert_eq!(
        released,
        vec![
            &Ev::ReleaseDepartment(950),
            &Ev::ReleaseDepartment(951),
            &Ev::ReleaseDepartment(952),
            &Ev::ReleaseDepartment(953),
            &Ev::Release(77),
        ]
    );
    let s = b.s.snapshot();
    assert_eq!(s.health.department1, Some(900));
    assert_eq!(s.lists.pending_school, vec![Some(11)]);
    assert!(s.lists.other.is_empty());
}

/// §15 test 21 and §9.3: `Init` after a load clears the strike flags and
/// overwrites the blocks' scalars, resets the iterator and leSum, and keeps
/// the saved tract sizes.
#[test]
fn t21_init_after_load() {
    let a = Fx::new(8);
    a.s.with_state(|s| {
        s.education.strike_flag = 1;
        s.education.strike_start = 1000;
        s.health.max_funding = 5.0;
        s.le_sum = 77;
        s.iterator.set_step(3);
        s.iterator.next_cell();
    });
    let (bytes, entries) = record(&a.s);
    // The developers' maps of the loading city have tract size 2.
    let b = Fx::constructed(CityDims::square(8), 2, Some(shipped_tuning()));
    assert!(load(&b, &bytes, &entries));
    assert_eq!(b.s.snapshot().education.strike_flag, 1);
    assert!(b.s.init());
    let s = b.s.snapshot();
    assert_eq!((s.education.strike_flag, s.education.strike_start), (0, 0));
    assert!(!b.s.school_is_on_strike());
    assert_eq!(s.health.max_funding.to_bits(), 0x3F99_9999);
    assert_eq!(s.le_sum, 0);
    assert_eq!(
        (s.iterator.state, s.iterator.x, s.iterator.z, s.iterator.step),
        (ScanState::Fresh, 0, 0, 3)
    );
    assert_eq!((s.population_size, s.eq_size), (4, 4));
    // The loaded departments are kept: none is created.
    assert!(!b.h.take_events().iter().any(|e| matches!(e, Ev::CreateDepartment(..))));
    assert_eq!(s.education.department1, Some(902));
}

// ================================================================ REH §11 (education and health)

/// Runs the coverage state (5 or 7) of a service and the grading state that
/// follows it.
fn cover_and_grade(fx: &Fx, school: bool) {
    fx.set_state(if school {
        state::SCHOOL_COVERAGE
    } else {
        state::HOSPITAL_COVERAGE
    });
    fx.step();
    fx.step();
}

fn coverage(fx: &Fx, i: usize, x: i32, z: i32) -> u8 {
    fx.grids().school_coverage[i].as_ref().unwrap().get(x, z)
}

/// REH test 1: coverage units (K-6 chunk 500); saturation at 255 with the
/// low-16-bit test of the sum [quirk].
#[test]
fn reh01_coverage_units() {
    let fx = Fx::new(16);
    service(&fx, 1, &SCHOOL, 0, 1500, 10.0, (32.0, 32.0));
    fx.set_state(state::SCHOOL_COVERAGE);
    assert_eq!(fx.step(), state::SCHOOL_GRADING);
    assert_eq!(coverage(&fx, 0, 0, 0), 3);
    assert_eq!((coverage(&fx, 0, 1, 0), coverage(&fx, 0, 0, 1)), (0, 0));
    service(&fx, 2, &SCHOOL, 0, 1500, 10.0, (32.0, 32.0));
    fx.set_state(state::SCHOOL_COVERAGE);
    fx.step();
    assert_eq!(coverage(&fx, 0, 0, 0), 6);

    // 250 + 10 → 255; 250 + 65286 = 0x10000 → low 16 bits 0 → 0 [quirk].
    for (second, want) in [(5000, 255), (65286 * 500, 0)] {
        let fx = Fx::new(16);
        service(&fx, 1, &SCHOOL, 0, 125_000, 10.0, (32.0, 32.0));
        service(&fx, 2, &SCHOOL, 0, second, 10.0, (32.0, 32.0));
        fx.set_state(state::SCHOOL_COVERAGE);
        fx.step();
        assert_eq!(coverage(&fx, 0, 0, 0), want, "second capacity {second}");
    }
}

/// REH test 2: units 0 still serve, once per building [quirk]; a tract
/// covered by a lower group is not counted.
#[test]
fn reh02_units_zero_still_serve() {
    let fx = Fx::new(16);
    fx.set_tract(0, 0, steady([10, 20, 0]));
    service(&fx, 1, &SCHOOL, 0, 400, 10.0, (32.0, 32.0));
    fx.set_state(state::SCHOOL_COVERAGE);
    fx.step();
    assert_eq!(coverage(&fx, 0, 0, 0), 0);
    assert_eq!(fx.s.snapshot().school_covered, 30);
    service(&fx, 2, &SCHOOL, 0, 400, 10.0, (32.0, 32.0));
    fx.set_state(state::SCHOOL_COVERAGE);
    fx.step();
    assert_eq!(fx.s.snapshot().school_covered, 60);

    // Group 0 covers the tract (units 1): the group-1 school does not count it.
    let fx = Fx::new(16);
    fx.set_tract(0, 0, steady([10, 20, 0]));
    service(&fx, 1, &SCHOOL, 0, 500, 10.0, (32.0, 32.0));
    service(&fx, 2, &SCHOOL, 1, 400, 10.0, (32.0, 32.0));
    fx.set_state(state::SCHOOL_COVERAGE);
    fx.step();
    assert_eq!(fx.s.snapshot().school_covered, 30);
    // The hospitals' state counts its own way.
    service(&fx, 3, &HOSPITAL, 0, 400, 10.0, (32.0, 32.0));
    fx.set_state(state::HOSPITAL_COVERAGE);
    fx.step();
    assert_eq!(fx.s.snapshot().hospital_covered, 30);
}

/// REH test 3: the tract list of `GetTractsInRadius` (width 64 m, 8 × 8).
#[test]
fn reh03_tract_list() {
    assert_eq!(tracts_in_radius(32.0, 32.0, 60.0, 64.0, 8, 8), vec![(0, 0)]);
    assert_eq!(
        tracts_in_radius(32.0, 32.0, 100.0, 64.0, 8, 8),
        vec![(0, 0), (0, 1), (1, 0), (1, 1)]
    );
    // x0 = trunc(−1.71875) = −1, clamped to 0.
    assert_eq!(tracts_in_radius(-10.0, 32.0, 100.0, 64.0, 8, 8), vec![(0, 0), (0, 1)]);
}

/// REH test 4: staff ratio and grade of capacity 1000 (units 2, so k = 1
/// and served = the covered population).
#[test]
fn reh04_staff_ratio_and_grade() {
    for (pop, staff, grade) in [(1250, 0.8, 0.8), (0, 1.0, 1.0), (500, 1.0, 1.0)] {
        let fx = Fx::new(16);
        fx.set_tract(0, 0, steady([pop, 0, 0]));
        service(&fx, 1, &SCHOOL, 0, 1000, 10.0, (32.0, 32.0));
        cover_and_grade(&fx, true);
        assert_eq!(fx.h.prop_u32(1, SCHOOL.served), Some(u32::from(pop)));
        assert_eq!(fx.h.prop_f32(1, SCHOOL.staff_ratio), Some(staff), "pop {pop}");
        assert_eq!(fx.h.prop_u32(1, SCHOOL.effective_capacity), Some(1000));
        assert_eq!(fx.h.prop_f32(1, SCHOOL.grade), Some(grade), "pop {pop}");
    }
    // Hospitals grade the same way.
    let fx = Fx::new(16);
    fx.set_tract(0, 0, steady([1250, 0, 0]));
    service(&fx, 1, &HOSPITAL, 0, 1000, 10.0, (32.0, 32.0));
    cover_and_grade(&fx, false);
    assert_eq!(fx.h.prop_f32(1, HOSPITAL.grade), Some(0.8));
    assert_eq!(fx.h.prop_f32(1, HOSPITAL.staff_ratio), Some(0.8));
}

/// REH test 5: the grade-grid blend [quirk]. A (group 0, units 3, grade
/// 0.5, n = 3, previous 0) → 0.5; B (group 2, units 3 at 50 % funding,
/// grade 0.5, n = 3, previous 0.5) → 2 × 0.5 + 0.5 = 1.5, clamped to 1.0.
#[test]
fn reh05_grade_grid_blend() {
    let grade_grid = |fx: &Fx| fx.grids().school_grade.as_ref().unwrap().get(0, 0);
    let fx = Fx::new(16);
    fx.set_tract(0, 0, steady([3000, 0, 0]));
    service(&fx, 1, &SCHOOL, 0, 1500, 10.0, (32.0, 32.0));
    cover_and_grade(&fx, true);
    assert_eq!(fx.h.prop_f32(1, SCHOOL.grade), Some(0.5));
    assert_eq!(grade_grid(&fx), 0.5);

    service(&fx, 2, &SCHOOL, 2, 3000, 10.0, (32.0, 32.0));
    fx.h.department(0x77, 950, 0.5);
    fx.h.item(
        2,
        SCHOOL.staff_purpose,
        BudgetItem {
            department: 0x77,
            ..BudgetItem::default()
        },
    );
    assert_eq!(fx.s.efficiency_ratings(2, true, false), (0.5, 1.0));
    cover_and_grade(&fx, true);
    assert_eq!(coverage(&fx, 2, 0, 0), 3);
    assert_eq!(fx.h.prop_u32(2, SCHOOL.effective_capacity), Some(1500));
    assert_eq!(fx.h.prop_f32(2, SCHOOL.grade), Some(0.5));
    assert_eq!(grade_grid(&fx), 1.0);
}

/// REH test 6: the served population is rounded half to even. A wealth
/// curve of 50 % (0.5 per resident) over 5 residents → 2.5 → 2; 7 → 3.5 →
/// 4; 49.998 % over 5 → 2.4999 → 2.
#[test]
fn reh06_served_rounding() {
    assert_eq!((rhe(2.5), rhe(3.5), rhe(2.4999)), (2, 4, 2));
    for (pop, level, served) in [(5, 50.0, 2), (7, 50.0, 4), (5, 49.998, 2)] {
        let fx = Fx::new(16);
        fx.set_tract(0, 0, steady([pop, 0, 0]));
        service(&fx, 1, &SCHOOL, 0, 500, 10.0, (32.0, 32.0));
        fx.h.set_prop(1, SCHOOL.wealth_curve, curve(&[0.0, level, 2.0, level]));
        cover_and_grade(&fx, true);
        assert_eq!(fx.h.prop_u32(1, SCHOOL.served), Some(served), "pop {pop}, {level} %");
    }
}

/// REH test 7: the strike roll, one draw per eligible non-striking building
/// (a building outside the can-strike group does not draw).
#[test]
fn reh07_strike_roll() {
    let fx = Fx::new(4);
    fx.h.today.set(1234);
    striker(&fx, 1, &SCHOOL, 0, 0.4);
    service(&fx, 2, &SCHOOL, 0, 100, 10.0, (32.0, 32.0));
    striker(&fx, 3, &HOSPITAL, 0, 0.4);
    // p = curve(40) = 50: 49.9 starts a strike, 50.0 does not.
    fx.h.queue.borrow_mut().extend([49.9, 50.0]);
    assert_eq!(fx.step(), state::SEED_LOW);
    assert_eq!(*fx.h.draws.borrow(), vec![(0.0, 100.0), (0.0, 100.0)]);
    assert_eq!(
        fx.h.take_events(),
        vec![
            Ev::Controllers(STRIKING_EDUCATION.to_string(), 1, true),
            Ev::Strike(msg::SCHOOL_STRIKE, Some(1), 1, 0),
            Ev::Ordinance,
        ]
    );
    assert_eq!(fx.h.prop_u32(1, PROP_STRIKE_DAY), Some(1234));
    assert_eq!(fx.h.prop_u32(3, PROP_STRIKE_DAY), None);
    assert!(fx.s.school_is_on_strike() && !fx.s.health_is_on_strike());
}

/// REH test 8: a building's strike ends after more than `duration` days or
/// when its staff ratio reaches "min grade" [quirks]; with no striking
/// building left the city-level strike ends (data2 0, data3 today − start).
#[test]
fn reh08_strike_end() {
    for (today, staff, ends) in [(1501, 0.5, true), (1500, 0.9, true), (1500, 0.89, false)] {
        let fx = Fx::new(4);
        striker(&fx, 1, &SCHOOL, 0, 0.4);
        fx.h.set_prop(1, PROP_STRIKE_DAY, u(1000));
        fx.h.set_prop(1, SCHOOL.staff_ratio, f(staff));
        fx.s.with_state(|s| {
            s.education.strike_flag = 1;
            s.education.strike_start = 1000;
        });
        fx.h.today.set(today);
        fx.step();
        assert!(fx.h.draws.borrow().is_empty());
        let ev = fx.h.take_events();
        if ends {
            assert_eq!(
                ev,
                vec![
                    Ev::Controllers(STRIKING_EDUCATION.to_string(), 1, false),
                    Ev::Strike(msg::SCHOOL_STRIKE, None, 0, today - 1000),
                    Ev::Ordinance,
                ],
                "today {today}, staff {staff}"
            );
            assert_eq!(fx.h.prop_u32(1, PROP_STRIKE_DAY), Some(0));
            assert!(!fx.s.school_is_on_strike());
        } else {
            assert_eq!(ev, vec![Ev::Ordinance]);
            assert_eq!(fx.h.prop_u32(1, PROP_STRIKE_DAY), Some(1000));
            assert!(fx.s.school_is_on_strike());
        }
    }
}

/// REH test 10: the system ratings, with the stale query record [quirk].
#[test]
fn reh10_system_rating() {
    let fx = Fx::new(4);
    service(&fx, 1, &SCHOOL, 0, 1000, 10.0, (32.0, 32.0));
    service(&fx, 2, &SCHOOL, 1, 2000, 10.0, (32.0, 32.0));
    graded(&fx, 1, &SCHOOL, 1000, 0, 1.0);
    graded(&fx, 2, &SCHOOL, 2000, 0, 1.0);
    fx.s.with_state(|s| s.population = 4000);
    assert_eq!(fx.s.school_system_rating(), 0.75);
    fx.s.with_state(|s| s.population = 0);
    assert_eq!(fx.s.school_system_rating(), 1.0);
    fx.s.with_state(|s| s.population = 4000);
    graded(&fx, 1, &SCHOOL, 0, 0, 1.0);
    graded(&fx, 2, &SCHOOL, 0, 0, 1.0);
    assert_eq!(fx.s.school_system_rating(), 0.0);
    // A building whose query fails adds the previous building's value again.
    graded(&fx, 1, &SCHOOL, 1000, 0, 1.0);
    fx.h.props.borrow_mut().remove(&(2, SCHOOL.effective_capacity));
    assert_eq!(fx.s.school_system_rating(), 0.5);
    // Hospitals: no buildings → 0.
    assert_eq!(fx.s.health_system_rating(), 0.0);
}

/// REH test 11: the coverage history per mille, rounded half to even.
#[test]
fn reh11_history_per_mille() {
    let fx = Fx::new(4);
    fx.s.with_state(|s| {
        s.population = 5000;
        s.school_covered = 1234;
        s.state = state::HISTORY;
    });
    fx.step();
    assert_eq!(
        fx.h.history()[3..],
        [(history::SCHOOL_COVERAGE, 247), (history::HOSPITAL_COVERAGE, 0)]
    );
    let fx = Fx::new(4);
    fx.s.with_state(|s| {
        s.school_covered = 1234;
        s.state = state::HISTORY;
    });
    fx.step();
    assert_eq!(
        fx.h.history()[3..],
        [(history::HOSPITAL_COVERAGE, 0), (history::SCHOOL_COVERAGE, 0)]
    );
}

/// REH test 13: life expectancy from the shipped curve, as a byte.
#[test]
fn reh13_life_expectancy() {
    let fx = Fx::new(4);
    for (hq, le) in [(30.0, 57), (100.0, 75), (0.0, 50)] {
        fx.set_tract(
            0,
            0,
            Tract {
                hq: [hq, 0.0, 0.0],
                ..steady([1, 0, 0])
            },
        );
        assert_eq!(fx.s.get_cell_life_expectancy(0, 0), le, "HQ {hq}");
    }
}

/// REH test 14: EQ decay 0.2 (10 → 9.8; 0.1 → 0).
#[test]
fn reh14_eq_decay() {
    let fx = Fx::new(4);
    fx.set_tract(
        0,
        0,
        Tract {
            eq: [10.0, 0.1, 0.0],
            age: 30,
            ..steady([100, 100, 0])
        },
    );
    fx.s.do_population_change(0, 0);
    assert_eq!(fx.eq(0, 0)[..2], [9.8, 0.0]);
}

/// REH test 15: the HQ pollution decay byte (average 600 → index 88; 256 →
/// index 0).
#[test]
fn reh15_hq_pollution_byte() {
    let fx = Fx::new(4);
    let t = fx.s.snapshot().population_tuning;
    fx.h.pollution.set([600, 0, 0]);
    assert_eq!(fx.s.hq_loss(&t, 4, 0, 0), 0.2 + t.pollution_curves[0].eval(88.0));
    assert!((fx.s.hq_loss(&t, 4, 0, 0) - 0.217_187_5).abs() < 1e-6);
    fx.h.pollution.set([256, 0, 0]);
    assert_eq!(fx.s.hq_loss(&t, 4, 0, 0), 0.2);
}

/// REH test 18: ideal funding against the efficiency curve (0,0) (100,100)
/// (120,110), limited to the max funding 1.2 (0x3F999999).
#[test]
fn reh18_ideal_funding() {
    let fx = Fx::new(4);
    service(&fx, 1, &SCHOOL, 0, 1000, 10.0, (32.0, 32.0));
    graded(&fx, 1, &SCHOOL, 1000, 500, 1.0);
    assert_eq!(fx.s.estimate_ideal_funding(1), Some(0.5));
    graded(&fx, 1, &SCHOOL, 1000, 1500, 1.0);
    assert_eq!(fx.s.estimate_ideal_funding(1).map(f32::to_bits), Some(0x3F99_9999));
    // Not graded yet, or no query data: AL = 0.
    graded(&fx, 1, &SCHOOL, 1000, NOT_GRADED, 1.0);
    assert_eq!(fx.s.estimate_ideal_funding(1), None);
    fx.h.props.borrow_mut().remove(&(1, SCHOOL.grade));
    assert_eq!(fx.s.estimate_ideal_funding(1), None);
    assert_eq!(fx.s.estimate_ideal_funding(99), None);
    // A hospital needs its staff item for the query (question 10).
    service(&fx, 2, &HOSPITAL, 0, 1000, 10.0, (32.0, 32.0));
    graded(&fx, 2, &HOSPITAL, 1000, 500, 1.0);
    assert_eq!(fx.s.estimate_ideal_funding(2), None);
    fx.h.item(2, HOSPITAL.staff_purpose, BudgetItem::default());
    assert_eq!(fx.s.estimate_ideal_funding(2), Some(0.5));
    // EH7: the school query first; it fails here (no school effective
    // capacity), so the hospital query decides.
    fx.h.set_prop(2, SCHOOL.capacity, u(1000));
    assert_eq!(fx.s.estimate_ideal_funding(2), Some(0.5));
}

/// REH test 19: an ordinance's effect factor 110 → m3 = 1.1 for every
/// school; the EQ boost scales by 1.1 from the next state 0 (the lists are
/// re-read there, not when the ordinance is enacted).
#[test]
fn reh19_ordinance() {
    let fx = Fx::new(16);
    fx.set_tract(0, 0, steady([100, 0, 0]));
    service(&fx, 1, &SCHOOL, 0, 500, 10.0, (32.0, 32.0));
    cover_and_grade(&fx, true);
    assert_eq!(fx.eq(0, 0)[0], 1.0);
    fx.h.enact(0x10, props(&[(SCHOOL.ord_effect, f(110.0))]));
    fx.set_tract(0, 0, steady([100, 0, 0]));
    cover_and_grade(&fx, true);
    assert_eq!(fx.eq(0, 0)[0], 1.0);
    fx.set_state(state::STRIKES_AND_LISTS);
    fx.step();
    let s = fx.s.snapshot();
    assert_eq!(ordinance_modifiers(&s.school_effects), (1.0, 1.0, 1.1));
    assert!(s.health_effects.is_empty());
    fx.set_tract(0, 0, steady([100, 0, 0]));
    cover_and_grade(&fx, true);
    assert_eq!(fx.eq(0, 0)[0], 1.1);
    assert_eq!(fx.h.prop_f32(1, SCHOOL.grade), Some(1.0));
}

/// REH test 20: the capacity estimate; staff uses the school divisor for
/// hospitals too [quirk].
#[test]
fn reh20_capacity_estimate() {
    let fx = Fx::new(4);
    service(&fx, 1, &SCHOOL, 0, 1000, 10.0, (32.0, 32.0));
    assert_eq!(fx.s.estimate_occupant_capacity(1), (1000, 34));
    let fx = Fx::with(4, Some(tuned(|t| t.add(id::SCHOOL_STAFF_DIVISOR, u(25)))));
    service(&fx, 2, &HOSPITAL, 0, 1000, 10.0, (32.0, 32.0));
    assert_eq!(fx.s.snapshot().simulator_tuning.hospital_staff_divisor, 30);
    assert_eq!(fx.s.estimate_occupant_capacity(2), (1000, 40));
    // At 50 % funding (e1 0.5, strikes ignored): 500 and ceil(500 / 25) = 20.
    fx.h.department(0x77, 950, 0.5);
    fx.h.item(
        2,
        HOSPITAL.staff_purpose,
        BudgetItem {
            department: 0x77,
            ..BudgetItem::default()
        },
    );
    fx.h.set_prop(2, PROP_STRIKE_DAY, u(5));
    assert_eq!(fx.s.estimate_occupant_capacity(2), (500, 20));
    // EH7: neither capacity reads as a hospital with capacity 0.
    assert_eq!(fx.s.estimate_occupant_capacity(99), (0, 0));
    // EH7: the school capacity decides first (no school staff item: e1 1).
    fx.h.set_prop(2, SCHOOL.capacity, u(600));
    assert_eq!(fx.s.estimate_occupant_capacity(2), (600, 24));
}

/// REH test 21: the query data's per-staff fields: the school rounds up,
/// the hospital down [quirk].
#[test]
fn reh21_query_data() {
    let fx = Fx::new(4);
    service(&fx, 1, &SCHOOL, 0, 1000, 10.0, (32.0, 32.0));
    service(&fx, 2, &HOSPITAL, 0, 1000, 10.0, (32.0, 32.0));
    fx.h.item(
        2,
        HOSPITAL.staff_purpose,
        BudgetItem {
            cost: 250,
            local_funding: 0.75,
            ..BudgetItem::default()
        },
    );
    for (eff, school, hospital) in [(990, 33, 33), (991, 34, 33)] {
        graded(&fx, 1, &SCHOOL, eff, 77, 0.5);
        graded(&fx, 2, &HOSPITAL, eff, NOT_GRADED, 0.5);
        let q = fx.s.school_query_data(1).unwrap();
        assert_eq!((q.capacity_per_staff, q.effective_per_staff), (33, school));
        assert_eq!((q.served, q.staff_funding, q.staff_cost), (77, 1.0, 0));
        assert_eq!((q.vehicle_funding, q.vehicle_cost), (1.0, 0));
        let q = fx.s.hospital_query_data(2).unwrap();
        assert_eq!((q.capacity_per_staff, q.effective_per_staff), (33, hospital));
        assert_eq!((q.served, q.staff_funding, q.staff_cost), (0, 0.75, 250));
    }
    // No staff item: the hospital query fails; a missing field fails both.
    fx.h.items.borrow_mut().clear();
    assert!(fx.s.hospital_query_data(2).is_none());
    fx.h.props.borrow_mut().remove(&(1, SCHOOL.served));
    assert!(fx.s.school_query_data(1).is_none());
    // Slot 14: the totals of the selected groups, counting successful queries.
    graded(&fx, 1, &SCHOOL, 990, 77, 0.5);
    service(&fx, 3, &SCHOOL, 1, 600, 10.0, (32.0, 32.0));
    graded(&fx, 3, &SCHOOL, 600, 23, 0.25);
    let mut t = QueryData::default();
    assert_eq!(fx.s.school_system_totals(&[0x150F, 0x1510], &mut t), 2);
    assert_eq!(
        (
            t.capacity,
            t.effective_capacity,
            t.capacity_per_staff,
            t.effective_per_staff,
            t.served,
            t.grade
        ),
        (1600, 1590, 53, 53, 100, 0.75)
    );
    let mut t = QueryData::default();
    assert_eq!(fx.s.school_system_totals(&[0x1510], &mut t), 1);
    // EH12: the hospital's query fails (no staff item), and it still counts
    // with what its query read (served raw), after the reset of +0x14..+0x28
    // (+0x2C on are not touched).
    t.staff_funding = 0.3;
    assert_eq!(fx.s.hospital_system_totals(&[0x1512], &mut t), 1);
    assert_eq!(
        (
            t.capacity,
            t.effective_capacity,
            t.capacity_per_staff,
            t.effective_per_staff,
            t.served,
            t.grade,
            t.staff_funding
        ),
        (1000, 991, 0, 0, NOT_GRADED, 0.0, 0.3)
    );
}

/// EH12 (REH §4.11 "Totals"): every building that has one of the groups
/// counts, whatever its query returned; one record serves every building, so
/// a failed query adds the previous building's other fields; the group test
/// is per building, over every list.
#[test]
fn eh12_system_totals() {
    let fx = Fx::new(4);
    // A hospital with its staff item, then one without (the farmer's market:
    // a hospital through its groups only), in the same list.
    service(&fx, 1, &HOSPITAL, 0, 1000, 10.0, (32.0, 32.0));
    fx.h.item(1, HOSPITAL.staff_purpose, BudgetItem::default());
    graded(&fx, 1, &HOSPITAL, 900, 300, 0.5);
    service(&fx, 2, &HOSPITAL, 0, 200, 10.0, (32.0, 32.0));
    graded(&fx, 2, &HOSPITAL, 100, NOT_GRADED, 0.25);
    let mut t = QueryData {
        capacity: 7,
        served: 7,
        grade: 7.0,
        staff_funding: 0.5,
        staff_cost: 9,
        ..QueryData::default()
    };
    assert_eq!(fx.s.hospital_system_totals(&[0x1512], &mut t), 2);
    // Building 2 adds its capacity, effective capacity and raw served
    // (0xFFFFFFFF: −1 modulo 2³²), and building 1's per-staff fields (1000 /
    // 30, 900 / 30) and grade.
    assert_eq!(
        (
            t.capacity,
            t.effective_capacity,
            t.capacity_per_staff,
            t.effective_per_staff,
            t.served,
            t.grade
        ),
        (1200, 1000, 66, 60, 299, 1.0)
    );
    assert_eq!((t.staff_funding, t.staff_cost), (0.5, 9));
    // A building filed under 0x1512 that also has 0x151C counts for 0x151C.
    fx.h.groups.borrow_mut().get_mut(&1).unwrap().push(0x151C);
    assert_eq!(fx.s.hospital_system_totals(&[0x151C], &mut t), 1);
    assert_eq!((t.capacity, t.served), (1000, 300));
    assert_eq!(fx.s.hospital_system_totals(&[], &mut t), 0);
    assert_eq!((t.capacity, t.served, t.grade), (0, 0, 0.0));
}

/// AQ1 (REH §4.11): a farmer's market (a hospital without the staff item)
/// counted first adds the record's +0x1C / +0x20 before any query wrote
/// them: stack leftovers in the original, 0 in FreeCity4 (*[choice]*).
#[test]
fn aq1_totals_record_starts_at_0() {
    let fx = Fx::new(4);
    service(&fx, 2, &HOSPITAL, 0, 200, 10.0, (32.0, 32.0));
    graded(&fx, 2, &HOSPITAL, 100, NOT_GRADED, 0.25);
    let mut t = QueryData::default();
    assert_eq!(fx.s.hospital_system_totals(&[0x1512], &mut t), 1);
    assert_eq!((t.capacity, t.effective_capacity), (200, 100));
    assert_eq!((t.capacity_per_staff, t.effective_per_staff), (0, 0));
}

/// EH10 (REH §4.11 "Order of the tests and partial writes"): a failed query
/// leaves the fields it read (served raw) and every other field as the
/// caller left it.
#[test]
fn eh10_query_partial_writes() {
    let fx = Fx::new(4);
    service(&fx, 2, &HOSPITAL, 0, 1000, 10.0, (32.0, 32.0));
    graded(&fx, 2, &HOSPITAL, 990, NOT_GRADED, 0.5);
    let preset = QueryData {
        capacity: 1,
        effective_capacity: 2,
        capacity_per_staff: 3,
        effective_per_staff: 4,
        served: 5,
        grade: 6.0,
        staff_funding: 7.0,
        staff_cost: 8,
        vehicle_funding: 9.0,
        vehicle_cost: 10,
    };
    let read = |want: QueryData| {
        let mut q = preset;
        assert!(!fx.s.query_data_into(false, 2, &mut q));
        assert_eq!(q, want);
    };
    // No staff item (tested last): capacity, effective capacity, raw served.
    read(QueryData {
        capacity: 1000,
        effective_capacity: 990,
        served: NOT_GRADED,
        ..preset
    });
    // No grade: the same three.
    fx.h.item(2, HOSPITAL.staff_purpose, BudgetItem::default());
    fx.h.props.borrow_mut().remove(&(2, HOSPITAL.grade));
    read(QueryData {
        capacity: 1000,
        effective_capacity: 990,
        served: NOT_GRADED,
        ..preset
    });
    // No effective capacity: the capacity alone.
    fx.h.props.borrow_mut().remove(&(2, HOSPITAL.effective_capacity));
    read(QueryData {
        capacity: 1000,
        ..preset
    });
    // A successful query maps served 0xFFFFFFFF to 0.
    graded(&fx, 2, &HOSPITAL, 990, NOT_GRADED, 0.5);
    let mut q = preset;
    assert!(fx.s.query_data_into(false, 2, &mut q));
    assert_eq!((q.served, q.capacity_per_staff, q.effective_per_staff), (0, 33, 33));
}

/// EH6 (REH §4.5): "served" takes the building's tract from its own
/// `GetPosition`, while the tract list uses the lot centre. With the lot
/// centre at x = 128 m (tract 2) and the building at x = 120 m (tract 1),
/// the listed tract 2 gets dx = float(u32(1 − 2)) ≈ 4.29e9, so the distance
/// curve is read at its last point.
#[test]
fn eh6_served_uses_the_position() {
    let fx = Fx::new(16);
    fx.set_tract(2, 0, steady([100, 0, 0]));
    service(&fx, 1, &SCHOOL, 0, 500, 40.0, (128.0, 32.0));
    fx.h.set_prop(1, SCHOOL.distance_curve, curve(&[0.0, 100.0, 100.0, 0.0]));
    cover_and_grade(&fx, true);
    assert_eq!(fx.h.prop_u32(1, SCHOOL.served), Some(100));
    fx.h.positions.borrow_mut().insert(1, (120.0, 32.0));
    fx.set_tract(2, 0, steady([100, 0, 0]));
    cover_and_grade(&fx, true);
    assert_eq!(coverage(&fx, 0, 2, 0), 1, "the tract list still has tract 2");
    assert_eq!(fx.h.prop_u32(1, SCHOOL.served), Some(0));
}

// ---- REH §4.1: building registration ----

fn staff_item(department: u32, line: u32, index: u32) -> BudgetItem {
    BudgetItem {
        department,
        line,
        index,
        cost: 100,
        local_funding: 1.0,
    }
}

/// REH §4.1: a school with budget items is registered with the budget
/// simulator (line item, locally funded) when added and unregistered when
/// removed; a null department aborts the call.
#[test]
fn building_registration() {
    let fx = Fx::new(4);
    let (staff, vehicle) = (SCHOOL.staff_purpose, SCHOOL.vehicle_purpose);
    fx.h.department(0x77, 950, 1.0);
    fx.h.department(0x78, 951, 1.0);
    for occ in [5, 6] {
        fx.h.building(occ, &[0x150F]);
        fx.h.building_types.borrow_mut().insert(occ, 0x1234);
        fx.h.set_prop(occ, SCHOOL.capacity, u(1500));
        fx.h.item(occ, staff, staff_item(0x77, 0, 3));
        fx.h.item(occ, vehicle, staff_item(0x78, 9, 4));
    }
    fx.s.with_state(|s| s.dirty = false);
    fx.s.do_building_changed(5, true);
    assert_eq!(
        fx.h.take_events(),
        vec![
            Ev::ChangeLine(5, 3, 0x1234),
            Ev::CreateLine(950, 0x1234, true),
            Ev::DisplayFlag(700, LINE_ITEM_DISPLAY_FLAG, true),
            Ev::AddLocal(950, 5, staff),
            Ev::LineCount(700, 1),
            Ev::AddLocal(951, 5, vehicle),
            Ev::AddRef(5),
        ]
    );
    assert_eq!(fx.h.items.borrow()[&(5, staff)].line, 0x1234);
    assert_eq!(fx.h.prop_u32(5, SCHOOL.effective_capacity), Some(1500));
    let s = fx.s.snapshot();
    assert!(s.dirty && s.lists.pending_school == vec![Some(5)]);
    fx.s.do_building_changed(6, true);
    assert_eq!(
        fx.h.take_events(),
        vec![
            Ev::ChangeLine(6, 3, 0x1234),
            Ev::AddLocal(950, 6, staff),
            Ev::LineCount(700, 2),
            Ev::AddLocal(951, 6, vehicle),
            Ev::AddRef(6),
        ]
    );
    // Removed: the line already is the building type.
    fx.s.do_building_changed(5, false);
    assert_eq!(
        fx.h.take_events(),
        vec![
            Ev::RemoveLocal(950, 5, staff),
            Ev::LineCount(700, 1),
            Ev::RemoveLocal(951, 5, vehicle),
            Ev::Release(5),
        ]
    );
    fx.s.do_building_changed(6, false);
    assert_eq!(
        fx.h.take_events(),
        vec![
            Ev::RemoveLocal(950, 6, staff),
            Ev::LineCount(700, 0),
            Ev::RemoveLine(950, 0x1234),
            Ev::RemoveLocal(951, 6, vehicle),
            Ev::Release(6),
        ]
    );
    assert!(fx.s.snapshot().lists.pending_school.is_empty());
    assert_eq!((fx.h.ref_count(5), fx.h.ref_count(6)), (0, 0));

    // A null staff department aborts after the line change; a null vehicle
    // department after the staff registration.
    fx.h.item(7, staff, staff_item(0x99, 0, 3));
    fx.h.building(7, &[]);
    fx.s.do_building_changed(7, true);
    assert_eq!(fx.h.take_events(), vec![Ev::ChangeLine(7, 3, 0)]);
    fx.h.item(8, staff, staff_item(0x77, 0x55, 3));
    fx.h.item(8, vehicle, staff_item(0x99, 9, 4));
    fx.h.building(8, &[]);
    fx.s.do_building_changed(8, true);
    assert_eq!(
        fx.h.take_events(),
        vec![
            Ev::CreateLine(950, 0x55, true),
            Ev::DisplayFlag(701, LINE_ITEM_DISPLAY_FLAG, true),
            Ev::AddLocal(950, 8, staff),
            Ev::LineCount(701, 1),
        ]
    );
    assert!(fx.s.snapshot().lists.pending_school.is_empty());
    // A hospital recognised by its group goes to the pending hospital list.
    fx.h.building(9, &[0x151C]);
    fx.s.do_building_changed(9, true);
    assert_eq!(fx.h.take_events(), vec![Ev::AddRef(9)]);
    assert_eq!(fx.s.snapshot().lists.pending_hospital, vec![Some(9)]);
    assert_eq!(fx.h.prop_u32(9, HOSPITAL.effective_capacity), Some(0));
}

/// residential.md §5.8 (RS8): a building with both health-affecting
/// properties above 0 joins the pending (other) list, which state 0 moves to
/// the health-affecting list; removal erases and releases it.
#[test]
fn rs8_health_affecting() {
    let fx = Fx::new(4);
    let [p, q] = PROP_HEALTH_AFFECTING;
    for (occ, a, b) in [(9, 1.0, 2.0), (10, 1.0, 0.0), (11, f32::NAN, 1.0)] {
        fx.h.building(occ, &[]);
        fx.h.set_prop(occ, p, f(a));
        fx.h.set_prop(occ, q, f(b));
    }
    fx.s.with_state(|s| s.dirty = false);
    fx.s.do_building_changed(10, true);
    fx.s.do_building_changed(11, true);
    assert!(fx.h.take_events().is_empty() && !fx.s.snapshot().dirty);
    fx.s.do_building_changed(9, true);
    assert_eq!(fx.h.take_events(), vec![Ev::AddRef(9)]);
    let s = fx.s.snapshot();
    assert!(s.dirty && s.lists.other == vec![Some(9)]);
    assert_eq!(fx.s.health_affecting_buildings(), vec![9]);
    fx.step();
    let s = fx.s.snapshot();
    assert!(s.lists.other.is_empty());
    assert_eq!(s.lists.health_affecting, vec![Some(9)]);
    fx.h.take_events();
    fx.s.with_state(|s| s.dirty = false);
    fx.s.do_building_changed(9, false);
    assert_eq!(fx.h.take_events(), vec![Ev::Release(9)]);
    let s = fx.s.snapshot();
    assert!(s.dirty && s.lists.health_affecting.is_empty());
    // Not in either list: nothing released, dirty still set.
    fx.s.with_state(|s| s.dirty = false);
    fx.s.do_building_changed(9, false);
    assert!(fx.h.take_events().is_empty() && fx.s.snapshot().dirty);
}

/// REH §4.2: slots 19 and 20, the efficiency ratings and the base staff
/// efficiency.
#[test]
fn funding_and_efficiency() {
    let fx = Fx::new(4);
    let (staff, vehicle) = (SCHOOL.staff_purpose, SCHOOL.vehicle_purpose);
    let get = |occ: u64| {
        let (mut a, mut b) = (0.3f32, 0.4f32);
        fx.s.get_occupant_funding(occ, &mut a, &mut b, true);
        (a, b)
    };
    // No staff item: 1.0 / 1.0, and the ratings (1, 1).
    assert_eq!(get(1), (1.0, 1.0));
    assert_eq!(fx.s.efficiency_ratings(1, true, false), (1.0, 1.0));
    assert_eq!(fx.s.base_staff_efficiency(1, true), 1.0);
    // A null staff department leaves both unchanged; the ratings start at
    // (0, 0) (question 1): effCurve(0) = 0, coverageCurve(0) = 50.
    fx.h.item(1, staff, staff_item(0x99, 0, 0));
    assert_eq!(get(1), (0.3, 0.4));
    assert_eq!(fx.s.efficiency_ratings(1, true, false), (0.0, 0.5));
    // The department's funding, no vehicle item → b = 1.0.
    fx.h.department(0x77, 950, 0.8);
    fx.h.item(1, staff, staff_item(0x77, 0, 0));
    assert_eq!(get(1), (0.8, 1.0));
    // Locally funded: the item's own percentage.
    fx.h.locally_funded.borrow_mut().insert((950, 1, staff));
    fx.h.item(
        1,
        staff,
        BudgetItem {
            local_funding: 0.5,
            ..staff_item(0x77, 0, 0)
        },
    );
    assert_eq!(get(1), (0.5, 1.0));
    // A vehicle item: its department's funding; a null one leaves b.
    fx.h.department(0x78, 951, 0.75);
    fx.h.item(1, vehicle, staff_item(0x78, 0, 1));
    assert_eq!(get(1), (0.5, 0.75));
    fx.h.item(1, vehicle, staff_item(0x98, 0, 1));
    assert_eq!(get(1), (0.5, 0.4));
    fx.h.item(1, vehicle, staff_item(0x78, 0, 1));
    // Ratings: e1 = effCurve(50) × 0.01 = 0.5, e2 = coverageCurve(75) × 0.01
    // = 87.5 × 0.01f; striking: a × strike efficiency (unless ignored).
    let (e1, e2) = fx.s.efficiency_ratings(1, true, false);
    assert_eq!((e1, e2), (0.5, pct(87.5)));
    fx.h.set_prop(1, PROP_STRIKE_DAY, u(77));
    let eff = fx.s.snapshot().education.strike_efficiency;
    assert_eq!(fx.s.efficiency_ratings(1, true, false).0, 0.5 * eff);
    assert_eq!(fx.s.efficiency_ratings(1, true, true).0, 0.5);
    assert_eq!(fx.s.base_staff_efficiency(1, true), 0.5);

    // Slot 19: staff then vehicle, dirty set.
    fx.h.take_events();
    fx.s.with_state(|s| s.dirty = false);
    assert!(fx.s.set_occupant_funding(1, 0.5, 0.25, true));
    assert_eq!(
        fx.h.take_events(),
        vec![Ev::SetLocal(950, 1, 0.5, staff), Ev::SetLocal(951, 1, 0.25, vehicle)]
    );
    assert!(fx.s.snapshot().dirty);
    // No vehicle item: staff set, then dirty.
    fx.h.items.borrow_mut().remove(&(1, vehicle));
    fx.s.with_state(|s| s.dirty = false);
    assert!(fx.s.set_occupant_funding(1, 0.9, 0.25, true));
    assert_eq!(fx.h.take_events(), vec![Ev::SetLocal(950, 1, 0.9, staff)]);
    assert!(fx.s.snapshot().dirty);
    // A null staff department: nothing, dirty not set.
    fx.h.item(1, staff, staff_item(0x99, 0, 0));
    fx.s.with_state(|s| s.dirty = false);
    assert!(fx.s.set_occupant_funding(1, 0.9, 0.25, true));
    assert!(fx.h.take_events().is_empty() && !fx.s.snapshot().dirty);
}

/// REH §4.2 (RG10, answered; the bracket "1.0" differed): a missing
/// department never reads as 1.0. A department whose funding the host cannot
/// read is missing too: the staff one leaves a and b as the caller set them,
/// the vehicle one leaves b after a is written. The class's own callers
/// preset (0, 0), so the curves see 0.
#[test]
fn rg10_unreadable_department_leaves_the_outputs() {
    let fx = Fx::new(4);
    let (staff, vehicle) = (SCHOOL.staff_purpose, SCHOOL.vehicle_purpose);
    let get = |occ: u64| {
        let (mut a, mut b) = (0.3f32, 0.4f32);
        fx.s.get_occupant_funding(occ, &mut a, &mut b, true);
        (a, b)
    };
    fx.h.department(0x77, 950, f32::NAN);
    fx.h.item(1, staff, staff_item(0x77, 0, 0));
    assert_eq!(get(1), (0.3, 0.4));
    // The ratings start at (0, 0): effCurve(0) = 0, coverageCurve(0) = 50.
    assert_eq!(fx.s.efficiency_ratings(1, true, false), (0.0, 0.5));
    assert_eq!(fx.s.base_staff_efficiency(1, true), 0.0);
    // Staff readable, vehicle not: a written, b kept.
    fx.h.department(0x78, 951, 0.75);
    fx.h.item(1, staff, staff_item(0x78, 0, 0));
    fx.h.item(1, vehicle, staff_item(0x77, 0, 1));
    assert_eq!(get(1), (0.75, 0.4));
    // Locally funded staff reads the item, whatever the department.
    fx.h.locally_funded.borrow_mut().insert((950, 1, staff));
    fx.h.item(
        1,
        staff,
        BudgetItem {
            local_funding: 0.5,
            ..staff_item(0x77, 0, 0)
        },
    );
    assert_eq!(get(1), (0.5, 0.4));
}

/// REH §4.11 slot 45: the coverage distance at a level, with the missing
/// 0.01 of the x_prev term [quirk].
#[test]
fn occupant_coverage_geometry() {
    let fx = Fx::new(4);
    service(&fx, 1, &SCHOOL, 0, 1000, 200.0, (32.0, 32.0));
    fx.h.set_prop(1, SCHOOL.distance_curve, curve(&[0.0, 100.0, 50.0, 80.0, 100.0, 0.0]));
    // Level 0.5: past (50, 80), at (100, 0): (30 / 80 × 0.5) × 200 + 50 × 200.
    assert_eq!(fx.s.occupant_coverage(1, 0.5), Some((10_037.5, 200.0)));
    assert_eq!(fx.s.occupant_coverage(1, 0.9), Some((50.0, 200.0)));
    assert_eq!(fx.s.occupant_coverage(1, 0.0), Some((10_100.0, 200.0)));
    // No radius: 0; neither a school nor a hospital: AL = 0.
    fx.h.props.borrow_mut().remove(&(1, SCHOOL.radius));
    assert_eq!(fx.s.occupant_coverage(1, 0.5), Some((0.0, 0.0)));
    assert_eq!(fx.s.occupant_coverage(99, 0.5), None);
    // The service: the hospital wins when both capacities exist (§4.11).
    fx.h.set_prop(1, HOSPITAL.capacity, u(10));
    assert_eq!(fx.s.occupant_service(1), Some(&HOSPITAL));

    // EH7 [quirk]: with both capacities, the hospital's radius, but the
    // school's distance curve (and funding): the school flag stays set.
    let fx = Fx::new(4);
    service(&fx, 3, &SCHOOL, 0, 1000, 200.0, (32.0, 32.0));
    fx.h.set_prop(3, HOSPITAL.capacity, u(10));
    fx.h.set_prop(3, HOSPITAL.radius, f(100.0));
    fx.h.set_prop(3, SCHOOL.distance_curve, curve(&[0.0, 100.0, 50.0, 80.0, 100.0, 0.0]));
    fx.h.set_prop(3, HOSPITAL.distance_curve, curve(&[0.0, 100.0, 100.0, 0.0]));
    // (30 / 80 × 0.5) × 100 + 50 × 100 on the school curve (the hospital
    // curve would give 50).
    assert_eq!(fx.s.occupant_coverage(3, 0.5), Some((5018.75, 100.0)));
    // EH8 [choice]: a distance property with fewer than two values has no
    // points, so dist is 0 (the flat default would give NaN at level 1).
    fx.h.set_prop(3, SCHOOL.distance_curve, f(5.0));
    assert_eq!(fx.s.occupant_coverage(3, 1.0), Some((0.0, 100.0)));
    fx.h.props.borrow_mut().remove(&(3, SCHOOL.distance_curve));
    assert!(fx.s.occupant_coverage(3, 1.0).unwrap().0.is_nan());
}

/// REH §4.3: one effect element per service and ordinance that has at least
/// one of the six properties (a curve alone counts), read at `Init` and,
/// after clearing, in every state 0; m1 / m2 / m3 and the decay products
/// multiply in list order; m2 scales the graded capacity.
#[test]
fn ordinance_effect_lists() {
    let fx = Fx::constructed(CityDims::square(16), 4, Some(shipped_tuning()));
    fx.h.enact(
        0x10,
        props(&[
            (SCHOOL.ord_radius, f(50.0)),
            (SCHOOL.ord_capacity, f(200.0)),
            (SCHOOL.ord_effect, f(110.0)),
            (SCHOOL.ord_decay, f(50.0)),
        ]),
    );
    fx.h.enact(
        0x11,
        props(&[(HOSPITAL.ord_effect, f(110.0)), (HOSPITAL.ord_decay, f(200.0))]),
    );
    fx.h.enact(0x12, props(&[(SCHOOL.ord_age_curve, curve(&[0.0, 50.0, 100.0, 50.0]))]));
    fx.h.enact(0x13, props(&[(0x1234_5678, u(7))]));
    fx.h.enact(0x14, props(&[(SCHOOL.ord_effect, f(110.0))]));
    assert!(fx.s.init());
    let s = fx.s.snapshot();
    assert_eq!((s.school_effects.len(), s.health_effects.len()), (3, 1));
    let (m1, m2, m3) = ordinance_modifiers(&s.school_effects);
    // 1.1f × 1.0 × 1.1f = 1.21 (0x3F9AE148), one float32 rounding each.
    assert_eq!((m1, m2, m3.to_bits()), (0.5, 2.0, 0x3F9A_E148));
    assert_eq!(s.school_effects[1].age_curve.eval(30.0), 50.0);
    assert_eq!(s.school_effects[0].age_curve.eval(30.0), 100.0);
    assert_eq!(ordinance_modifiers(&s.health_effects), (1.0, 1.0, pct(110.0)));
    assert_eq!(fx.s.ordinance_decay_products(), (0.5, 2.0));
    // State 0 clears and re-reads: 0x14 is off now.
    fx.h.ordinances.borrow_mut().remove(&0x14);
    fx.set_state(state::STRIKES_AND_LISTS);
    fx.step();
    let s = fx.s.snapshot();
    assert_eq!((s.school_effects.len(), s.health_effects.len()), (2, 1));
    assert_eq!(ordinance_modifiers(&s.school_effects), (0.5, 2.0, pct(110.0)));
    // Grading: 0x12's age curve (50) halves the served population (8000 →
    // 4000) and m2 = 2 doubles the effective capacity (2000): grade 0.5.
    fx.set_tract(0, 0, steady([8000, 0, 0]));
    service(&fx, 1, &SCHOOL, 0, 1000, 10.0, (32.0, 32.0));
    cover_and_grade(&fx, true);
    assert_eq!(fx.h.prop_u32(1, SCHOOL.served), Some(4000));
    assert_eq!(fx.h.prop_u32(1, SCHOOL.effective_capacity), Some(2000));
    assert_eq!(fx.h.prop_f32(1, SCHOOL.staff_ratio), Some(0.5));
    assert_eq!(fx.h.prop_f32(1, SCHOOL.grade), Some(0.5));
}

/// REH §4.6: `AddRadiusEffectWithCurves` itself: the distance curve at the
/// 0..1 fraction, the age curves, the n-share with pop', the cap, and the z
/// centre an empty tract does not advance [quirk].
#[test]
fn add_radius_effect_curves_share_and_cap() {
    let city = CityDims::square(16);
    let target = SimGrid::<f32>::create(&city, 0, 4);
    let pop = SimGrid::<u16>::create(&city, 0, 4);
    let cov = SimGrid::<u8>::create(&city, 0, 4);
    pop.set(0, 0, 100);
    pop.set(0, 2, 50);
    pop.set(1, 0, 10);
    cov.set(0, 0, 2);
    target.set(0, 0, 190.0);
    let falling = crate::curve::ResponseCurve::from_points(&[(0.0, 100.0), (1.0, 0.0)]);
    let e = RadiusEffect {
        age: None,
        target: &*target,
        age_curves: &[],
        x: 32.0,
        z: 32.0,
        radius: 200.0,
        distance_curves: &[&falling],
        value: 10.0,
        max: 200.0,
        units: 4,
        coverage: Some(&*cov),
        population: Some(&*pop),
    };
    let mut total = 0.0;
    add_radius_effect_with_curves(&e, &mut total);
    let near = |a: f32, b: f32| (a - b).abs() < 1e-4;
    // (0, 0): d = 0 → 100 % → 10 × k (4 / 2) = 20, capped at 200; pop' 200.
    assert_eq!(target.get(0, 0), 200.0);
    // (0, 1) is empty, so (0, 2) is measured from (0, 1)'s centre: d = 64,
    // xd = 0.32 → 68 % → 6.8 (6.4 from its own centre); n = 0 → pop' = 50.
    assert_eq!(target.get(0, 1), 0.0);
    assert!(near(target.get(0, 2), 6.8), "{}", target.get(0, 2));
    assert!(near(target.get(1, 0), 6.8), "{}", target.get(1, 0));
    assert!(near(total, 200.0 * 20.0 + 50.0 * 6.8 + 10.0 * 6.8), "{total}");

    // An age curve of 50 % at every age: mult = (1.0 × 50) × 0.01.
    let age = SimGrid::<u8>::create(&city, 0, 4);
    age.fill(30);
    let half = crate::curve::ResponseCurve::from_points(&[(0.0, 50.0), (100.0, 50.0)]);
    let target = SimGrid::<f32>::create(&city, 0, 4);
    let e = RadiusEffect {
        age: Some(&*age),
        target: &*target,
        age_curves: &[&half],
        ..e
    };
    let mut total = 0.0;
    add_radius_effect_with_curves(&e, &mut total);
    assert_eq!(target.get(0, 0), 10.0);
    assert!(near(target.get(1, 0), 3.4), "{}", target.get(1, 0));
}

/// REH §4.4 / §4.5: a building without a coverage radius is skipped by the
/// coverage state; grading writes served 0, effective capacity 0 and grade
/// 0.0 and leaves the staff ratio alone.
#[test]
fn grading_without_radius() {
    let fx = Fx::new(16);
    fx.set_tract(0, 0, steady([100, 0, 0]));
    service(&fx, 1, &SCHOOL, 0, 1000, 10.0, (32.0, 32.0));
    fx.h.props.borrow_mut().remove(&(1, SCHOOL.radius));
    fx.h.set_prop(1, SCHOOL.staff_ratio, f(0.7));
    fx.h.set_prop(1, SCHOOL.grade, f(0.9));
    cover_and_grade(&fx, true);
    assert_eq!(coverage(&fx, 0, 0, 0), 0);
    assert_eq!(fx.s.snapshot().school_covered, 0);
    assert_eq!(fx.h.prop_u32(1, SCHOOL.served), Some(0));
    assert_eq!(fx.h.prop_u32(1, SCHOOL.effective_capacity), Some(0));
    assert_eq!(fx.h.prop_f32(1, SCHOOL.grade), Some(0.0));
    assert_eq!(fx.h.prop_f32(1, SCHOOL.staff_ratio), Some(0.7));
}

/// REH §4.10: the auto-budget step of states 6 / 8 runs before the building
/// is graded (so on last cycle's served population) when the service's
/// global flag or the building's own flag is set; only the staff
/// percentage changes, the vehicle one is written back.
#[test]
fn auto_budget_step() {
    let fx = Fx::new(16);
    let (staff, vehicle) = (SCHOOL.staff_purpose, SCHOOL.vehicle_purpose);
    let set_local = |fx: &Fx| -> Vec<Ev> {
        fx.h.take_events()
            .into_iter()
            .filter(|e| matches!(e, Ev::SetLocal(..)))
            .collect()
    };
    fx.set_tract(0, 0, steady([500, 0, 0]));
    service(&fx, 1, &SCHOOL, 0, 1000, 10.0, (32.0, 32.0));
    fx.h.department(0x77, 950, 0.8);
    fx.h.department(0x78, 951, 0.75);
    fx.h.item(1, staff, staff_item(0x77, 0, 0));
    fx.h.item(1, vehicle, staff_item(0x78, 0, 1));
    graded(&fx, 1, &SCHOOL, 1000, 500, 1.0);
    // Neither flag: no step.
    cover_and_grade(&fx, true);
    assert!(set_local(&fx).is_empty());
    assert_eq!(fx.h.prop_u32(1, SCHOOL.served), Some(500));
    // The building's flag: ideal funding 500 / 1000 → 0.5; b = 0.75 back.
    assert!(fx.s.set_auto_budget(1, true));
    cover_and_grade(&fx, true);
    assert_eq!(
        set_local(&fx),
        vec![Ev::SetLocal(950, 1, 0.5, staff), Ev::SetLocal(951, 1, 0.75, vehicle)]
    );
    // Not graded yet: EstimateIdealFunding fails, no step.
    fx.h.set_prop(1, SCHOOL.served, u(NOT_GRADED));
    cover_and_grade(&fx, true);
    assert!(set_local(&fx).is_empty());
    // The global flag alone, on last cycle's served 1500: r = 1.5 → the max.
    assert!(fx.s.set_auto_budget(1, false));
    fx.s.with_state(|s| s.school_auto_budget = true);
    fx.h.set_prop(1, SCHOOL.served, u(1500));
    cover_and_grade(&fx, true);
    let max = f32::from_bits(0x3F99_9999);
    assert_eq!(
        set_local(&fx),
        vec![Ev::SetLocal(950, 1, max, staff), Ev::SetLocal(951, 1, 0.75, vehicle)]
    );
    assert_eq!(fx.h.prop_u32(1, SCHOOL.served), Some(500));

    // Hospitals (state 8): no vehicle item, so only the staff percentage is
    // set before SetOccupantFundingPercentages stops.
    service(&fx, 2, &HOSPITAL, 0, 1000, 10.0, (32.0, 32.0));
    fx.h.department(0x79, 952, 1.0);
    fx.h.item(2, HOSPITAL.staff_purpose, staff_item(0x79, 0, 0));
    graded(&fx, 2, &HOSPITAL, 1000, 500, 1.0);
    cover_and_grade(&fx, false);
    assert!(set_local(&fx).is_empty());
    fx.s.with_state(|s| s.hospital_auto_budget = true);
    cover_and_grade(&fx, false);
    assert_eq!(set_local(&fx), vec![Ev::SetLocal(952, 2, 0.5, HOSPITAL.staff_purpose)]);
}

/// RS13: the grade totals accumulate during grading and are reset by state
/// 0; they are not saved.
#[test]
fn rs13_grade_totals() {
    let fx = Fx::new(16);
    fx.set_tract(0, 0, steady([100, 0, 0]));
    service(&fx, 1, &SCHOOL, 0, 500, 10.0, (32.0, 32.0));
    cover_and_grade(&fx, true);
    // 100 residents × the boost 1.0 (units 1, n 1).
    assert_eq!(fx.s.snapshot().school_grade_total, 100.0);
    let (bytes, _) = record(&fx.s);
    fx.s.with_state(|s| s.school_grade_total = 0.0);
    assert_eq!(record(&fx.s).0, bytes);
    fx.s.with_state(|s| s.hospital_grade_total = 5.0);
    fx.set_state(state::STRIKES_AND_LISTS);
    fx.step();
    let s = fx.s.snapshot();
    assert_eq!((s.school_grade_total, s.hospital_grade_total), (0.0, 0.0));
}

// ================================================================ RS answers (record)

/// RS11 / RS14 / RS15: a damaged record returns false and leaves the
/// simulator as constructed.
#[test]
fn rs11_14_15_damaged_records() {
    let a = Fx::new(8);
    let (bytes, entries) = record(&a.s);
    // Offsets: P at 30, the coverage vector counts at 66 and 126, the state
    // at 230, the building-list vector counts at 357 and 389.
    assert_eq!(bytes[66..70], 7u32.to_le_bytes());
    assert_eq!(bytes[126..130], 3u32.to_le_bytes());
    assert_eq!(bytes[357..361], 7u32.to_le_bytes());
    assert_eq!(bytes[389..393], 3u32.to_le_bytes());
    let patched = |at: usize, v: &[u8]| {
        let mut b = bytes.clone();
        b[at..at + v.len()].copy_from_slice(v);
        b
    };
    let read = |b: &[u8]| {
        let fx = Fx::constructed(CityDims::square(8), 4, None);
        let ok = load(&fx, b, &entries);
        (ok, fx)
    };
    for (what, at, v) in [
        ("school coverage count 6", 66, 6u32),
        ("school coverage count 8", 66, 8),
        ("hospital coverage count 4", 126, 4),
        ("school list count 6", 357, 6),
        ("hospital list count 2", 389, 2),
        ("state 16", STATE_AT, 16),
        ("P 0", 30, 0),
        ("P 257", 30, 257),
    ] {
        let (ok, fx) = read(&patched(at, &v.to_le_bytes()));
        assert!(!ok, "{what}");
        assert_constructed(&fx);
        for t in 900..904 {
            assert_eq!(fx.h.ref_count(t), 0, "{what}: department {t}");
        }
    }
    let (ok, fx) = read(&patched(STATE_AT, &15u32.to_le_bytes()));
    assert!(ok);
    assert_eq!(fx.state(), state::SEED_HIGH);
    let (ok, fx) = read(&patched(30, &256u32.to_le_bytes()));
    assert!(ok);
    assert_eq!(fx.s.snapshot().population_size, 256);
    // EH14 [choice]: P and E outside 1…256 as signed (E at offset 170).
    assert_eq!(bytes[E_AT..E_AT + 4], 4i32.to_le_bytes());
    for (what, at, v) in [
        ("P -1", 30, -1i32),
        ("P i32::MIN", 30, i32::MIN),
        ("E 0", E_AT, 0),
        ("E -1", E_AT, -1),
        ("E 257", E_AT, 257),
    ] {
        let (ok, fx) = read(&patched(at, &v.to_le_bytes()));
        assert!(!ok, "{what}");
        assert_constructed(&fx);
    }
    let (ok, fx) = read(&patched(E_AT, &256i32.to_le_bytes()));
    assert!(ok);
    assert_eq!(fx.s.snapshot().eq_size, 256);
    // EH14 [choice]: the loaded R$ population grid's own size (state 4's S).
    for size in [0, -4, 512] {
        let b = Fx::new(8);
        let g = SimGrid::<u16>::create(&CityDims::square(8), 0, size);
        b.s.with_state(|s| s.grids.population[0] = Some(g as Rc<dyn crate::grid::TractGrid<u16>>));
        let (bytes, entries) = record(&b.s);
        let fx = Fx::constructed(CityDims::square(8), 4, None);
        assert!(!load(&fx, &bytes, &entries), "R$ population grid size {size}");
        assert_constructed(&fx);
    }
}

/// The offset of E in a version-7 record whose grids are all present.
const E_AT: usize = STATE_AT - (2 * 4 + 6 * 8 + 4);

/// EH14 (residential.md §5.5): state 4's S is the R$ population grid's own
/// `GetTractSize()`, not the member P.
#[test]
fn eh14_state_4_tract_size_from_the_grid() {
    let fx = Fx::new(8);
    fx.s.with_state(|s| s.population_size = 2);
    fx.h.pollution_calls.set(0);
    fx.s.do_population_change(0, 0);
    // Three pollution types × S² cells with S = 4 (P = 2 would give 12).
    assert_eq!(fx.h.pollution_calls.get(), 48);
}

/// RS16: the average-LE guard is `population ≠ 0`, and the division is
/// unsigned 64-bit by the sign-extended population.
#[test]
fn rs16_negative_population() {
    let run = |running: i32, le_sum: u64| {
        let fx = Fx::new(4);
        fx.s.with_state(|s| {
            s.running_population = running;
            s.le_sum = le_sum;
            s.state = state::POST_POPULATION;
        });
        fx.step();
        fx.h.history()[0]
    };
    assert_eq!(run(-5, 28_600), (history::AVERAGE_LIFE_EXPECTANCY, 0));
    // 2⁶⁴ − 5 divided by 2⁶⁴ − 5: 1 (a `> 0` guard would give 0).
    assert_eq!(run(-5, (-5i64) as u64), (history::AVERAGE_LIFE_EXPECTANCY, 1));
    assert_eq!(run(0, 28_600), (history::AVERAGE_LIFE_EXPECTANCY, 0));
    assert_eq!(run(400, 28_600), (history::AVERAGE_LIFE_EXPECTANCY, 71));
}

/// RS17 (residential.md §9.3's example): four tracts of total 100 and LE
/// 60, saved in state 4 with running population 200. After the load and
/// `Init` the pass restarts: version ≥ 4 keeps 200 (population 600, LE
/// history 40); version 3 keeps the constructor's 0 (400, 60).
#[test]
fn rs17_version_3_running_population() {
    let tuning = || {
        Some(tuned(|t| {
            flat_le(t, 60.0);
            no_decay(t);
        }))
    };
    let a = Fx::with(8, tuning());
    a.s.with_state(|s| {
        s.state = state::POPULATION;
        s.running_population = 200;
        s.le_sum = 12_000;
    });
    let (bytes, entries) = record(&a.s);
    for (v, len, population, le) in [(7, V7_LEN, 600, 40), (4, V4_LEN, 600, 40), (3, V3_LEN, 400, 60)] {
        let b = Fx::constructed(CityDims::square(8), 4, tuning());
        assert!(load(&b, &as_version(&bytes, v, len), &entries));
        assert!(b.s.init());
        assert_eq!(b.s.snapshot().le_sum, 0);
        for (x, z) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            b.set_tract(
                x,
                z,
                Tract {
                    age: 30,
                    ..steady([100, 0, 0])
                },
            );
        }
        b.h.take_events();
        assert_eq!(b.step(), state::POST_POPULATION);
        assert_eq!(b.step(), state::SCHOOL_COVERAGE);
        assert_eq!(b.s.get_population(), population, "version {v}");
        assert_eq!(b.h.history()[0], (history::AVERAGE_LIFE_EXPECTANCY, le), "version {v}");
    }
}
