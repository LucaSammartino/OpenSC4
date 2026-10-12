// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests of the pollution simulator against a small fake host.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use super::*;
use crate::grid::{CityDims, CoreGrids};
use crate::property::PropertyHolder;
use crate::rng::FixedSeeds;
use crate::stream::{ByteReader, ByteWriter, RefResolver, RefValue, SaveObject, SaveRegistry};
use crate::variant::{Elem, Variant};

const OCC_CLASS: u32 = 0x1234_0001;

#[derive(Default)]
struct Host {
    cells: Cell<i32>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    zones: RefCell<BTreeMap<(i32, i32), u32>>,
    props: RefCell<BTreeMap<u64, PropertyHolder>>,
    posts: RefCell<Vec<(u32, u32)>>,
    history: RefCell<Vec<(u32, i32)>>,
    refs: RefCell<BTreeMap<u64, i64>>,
    effects: Cell<u64>,
    funding: Cell<f32>,
    landfill: Cell<(u32, u32)>,
    tiles: Cell<u32>,
    population: Cell<i32>,
}

impl GridProvider<i16> for Host {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<i16>>> {
        CoreGrids {
            city: CityDims::square(self.cells.get()),
        }
        .create_grid(instance, tract_size)
    }
}

impl PollutionHost for Host {
    fn cell_counts(&self) -> (i32, i32) {
        (self.cells.get(), self.cells.get())
    }
    fn cell_widths(&self) -> (f32, f32) {
        (16.0, 16.0)
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        ((x / 16.0).floor() as i32, (z / 16.0).floor() as i32)
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn altitude(&self, _x: f32, _z: f32) -> f32 {
        0.0
    }
    fn add_agent(&self, _level: u32, _name: &str, _flags: u32) {}
    fn remove_agent(&self) {}
    fn subscribe(&self, _t: &[u32]) {}
    fn unsubscribe(&self, _t: &[u32]) {}
    fn post(&self, msg_type: u32, data1: u32) {
        self.posts.borrow_mut().push((msg_type, data1));
    }
    fn record_history(&self, series: u32, value: i32) {
        self.history.borrow_mut().push((series, value));
    }
    fn set_history_queue_length(&self, _series: u32, _length: u32) {}
    fn add_prop_requester(&self) {}
    fn remove_prop_requester(&self) {}
    fn zone_type(&self, x: i32, z: i32) -> u32 {
        self.zones.borrow().get(&(x, z)).copied().unwrap_or(0)
    }
    fn air_traffic(&self, _x: i32, _z: i32) -> u8 {
        0
    }
    fn cumulative_factor(&self, _id: u32, _index: u32) -> f32 {
        1.0
    }
    fn find_deals(&self, _deal_type: u32) -> Vec<Deal> {
        Vec::new()
    }
    fn set_deal_amount_used(&self, _deal: u64, _used: i32) {}
    fn mark_deals_dirty(&self) {}
    fn is_occupant_powered(&self, _occ: u64) -> bool {
        true
    }
    fn population(&self) -> i32 {
        self.population.get()
    }
    fn landfill_usage(&self) -> (u32, u32) {
        self.landfill.get()
    }
    fn landfill_tile_count(&self) -> u32 {
        self.tiles.get()
    }
    fn set_lot_condition(&self, _x: i32, _z: i32, _c: u32) {}
    fn occupant_property(&self, occ: u64, id: u32) -> Option<Variant> {
        self.props.borrow().get(&occ)?.get(id).cloned()
    }
    fn set_occupant_property(&self, occ: u64, id: u32, value: Variant) {
        self.props.borrow_mut().entry(occ).or_default().add(id, value);
    }
    fn remove_occupant_property(&self, _occ: u64, _id: u32) {}
    fn occupant_position(&self, _occ: u64) -> [f32; 3] {
        [336.0, 0.0, 336.0]
    }
    fn occupant_bounding_box(&self, _occ: u64) -> ([f32; 3], [f32; 3]) {
        ([336.0, 0.0, 336.0], [336.0, 0.0, 336.0])
    }
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
    fn create_department(&self, _id: u32, _group: u32) -> Option<u64> {
        Some(900)
    }
    fn set_department_name(&self, _d: u64, _g: u32, _i: u32) {}
    fn set_department_max_funding(&self, _d: u64, _f: f32) {}
    fn create_line_item(&self, _d: u64, _l: u32, _lf: bool, _g: u32, _i: u32) {}
    fn department_funding(&self, _d: u64) -> f32 {
        self.funding.get()
    }
    fn set_line_item_full_expenses(&self, _d: u64, _l: u32, _v: i64) {}
    fn budget_item_for_purpose(&self, _occ: u64, _p: u32) -> Option<BudgetItem> {
        None
    }
    fn add_budget_item_to_budget(&self, _occ: u64, _i: u32) {}
    fn remove_budget_item_from_budget(&self, _occ: u64, _i: u32) {}
    fn change_budget_item_cost(&self, _occ: u64, _i: u32, _c: i64) {}
    fn add_to_budget_item_line_expenses(&self, _item: &BudgetItem, _d: i64) {}
    fn create_visual_effect(&self, _id: u32, _kind: u32) -> Option<u64> {
        let n = self.effects.get() + 1;
        self.effects.set(n);
        Some(10_000 + n)
    }
    fn set_effect_position(&self, _e: u64, _p: [f32; 3]) {}
    fn set_effect_param(&self, _e: u64, _p: u32, _v: f32) {}
    fn start_effect(&self, _e: u64, _t: u32) {}
    fn stop_effect(&self, _e: u64, _t: u32) {}
    fn is_effect_started(&self, _e: u64) -> bool {
        true
    }
}

fn host(cells: i32) -> Rc<Host> {
    let h = Host {
        cells: Cell::new(cells),
        funding: Cell::new(1.0),
        ..Default::default()
    };
    *h.tuning.borrow_mut() = Some(Rc::new(shipped_tuning()));
    Rc::new(h)
}

fn sim(h: &Rc<Host>) -> Rc<PollutionSim> {
    let s = PollutionSim::new(
        h.clone() as Rc<dyn PollutionHost>,
        1,
        1,
        &FixedSeeds {
            wall_clock: 1,
            process_clock: 1,
        },
    );
    s.init();
    s
}

fn u32v(v: u32) -> Variant {
    Variant::Scalar(Elem::U32(v))
}

fn i32s(v: &[i32]) -> Variant {
    Variant::array(crate::variant::mem::SINT32, v.iter().map(|&x| Elem::I32(x)).collect())
}

/// Visits the tract of cell (x, z) once.
fn visit(s: &PollutionSim, x: u32, z: u32) {
    s.with_state(|st| {
        st.tracts_per_day = 1;
        st.cell_x = x;
        st.cell_z = z;
    });
    s.do_message(msg::SIM_NEW_DAY, 0, 0);
}

#[test]
fn init_sizes_and_cadence() {
    let h = host(128);
    let s = sim(&h);
    let st = s.snapshot();
    assert!(st.initialized && !st.loaded);
    assert_eq!(st.tracts_per_day, 273);
    assert_eq!((st.blocks_x, st.blocks_z), (16, 16));
    assert_eq!(st.iterator.table.len(), 4096);
    assert!(st.grids.iter().all(Option::is_some));
    assert_eq!(st.department, Some(900));
    assert_eq!(h.refs.borrow().get(&900), Some(&1));
    s.shutdown();
    assert_eq!(h.refs.borrow().get(&900), Some(&0));
}

#[test]
fn tuning_patterns() {
    let mut t = Tuning::default();
    assert_eq!(t.max_air_delta, 50);
    assert_eq!(t.funding_decay_curve.eval(0.3), 1.0);
    t.reload(Some(&shipped_tuning()));
    assert_eq!(t.max_air_delta, 150);
    assert_eq!(t.cost_per_unit, 0.005);
    // "keep": costPerUnit survives a reload without the property.
    t.reload(Some(&PropertyHolder::new()));
    assert_eq!(t.cost_per_unit, 0.005);
    assert_eq!(t.max_air_delta, 50);
    assert!(t.prop_curve.is_empty());
    assert_eq!(t.usage_decay_curve.eval(0.7), 1.0);
    t.reload(None);
    assert_eq!(t.cost_per_unit, 0.005);
}

#[test]
fn cone_and_removal() {
    let h = host(128);
    let s = sim(&h);
    h.set_occupant_property(5, prop::CENTRE, i32s(&[100, 0, 0, 0]));
    s.do_message(msg::OCCUPANT_INSERTED, 5, 0);
    let g = s.snapshot().grids[G_AIR_SRC].clone().unwrap();
    assert_eq!(g.get(10, 10), 100);
    assert_eq!(g.get(11, 10), 60);
    assert_eq!(g.get(11, 11), 43);
    assert_eq!(g.get(12, 10), 20);
    assert_eq!(g.get(12, 11), 11);
    assert_eq!(g.get(12, 12), 0);
    s.do_message(msg::OCCUPANT_REMOVED, 5, 0);
    assert_eq!(g.get(10, 10), 0);
    assert_eq!(g.get(12, 11), 0);
}

#[test]
fn preview_occupant_is_skipped() {
    let h = host(128);
    let s = sim(&h);
    h.set_occupant_property(5, prop::PREVIEW, Variant::Scalar(Elem::Bool(true)));
    h.set_occupant_property(5, prop::CENTRE, i32s(&[100, 0, 0, 0]));
    h.set_occupant_property(5, prop::INCINERATOR, u32v(10));
    s.do_message(msg::OCCUPANT_INSERTED, 5, 0);
    assert!(s.snapshot().lists[L_INC].is_empty());
    assert_eq!(s.snapshot().grids[G_AIR_SRC].clone().unwrap().get(10, 10), 0);
    assert_eq!(h.refs.borrow().get(&5), None);
}

#[test]
fn relaxation_and_water_stop_at_one() {
    let h = host(128);
    let s = sim(&h);
    h.zones.borrow_mut().insert((20, 20), 1);
    let st = s.snapshot();
    st.grids[G_AIR_SRC].as_ref().unwrap().set(10, 10, 1000);
    let air = st.grids[G_AIR_VAL].clone().unwrap();
    for want in [150, 300, 450, 600, 750, 900, 1000, 1000] {
        visit(&s, 20, 20);
        assert_eq!(air.get(10, 10), want);
    }
    let water = st.grids[G_WATER_VAL].clone().unwrap();
    water.set(1, 1, 100);
    visit(&s, 2, 2);
    assert_eq!(water.get(1, 1), 1);
    visit(&s, 2, 2);
    assert_eq!(water.get(1, 1), 0);
    water.set(40, 40, 100);
    visit(&s, 80, 80);
    assert_eq!(water.get(40, 40), 0);
}

#[test]
fn dispose_quirk_and_landfill() {
    let h = host(64);
    let s = sim(&h);
    s.with_state(|st| st.decomposed = 2000);
    let mut rem = Remaining {
        wte: 50,
        ..Default::default()
    };
    assert_eq!(s.dispose(100.0, &mut rem), 0.0);
    assert_eq!(rem.wte, 0);
    let mut rem = Remaining {
        wte: 150,
        ..Default::default()
    };
    assert_eq!(s.dispose(100.0, &mut rem), 100.0);
    assert_eq!(rem.wte, 50);
    assert_eq!(s.snapshot().month.wte, 100);
    // Landfill: efficiency 1, one tile -> 5000 a day of room 10000.
    h.landfill.set((0, 10_000));
    h.tiles.set(1);
    s.with_state(|st| st.efficiency = 1.0);
    let mut rem = Remaining::default();
    assert_eq!(s.dispose(7000.0, &mut rem), 5000.0);
    assert_eq!(s.snapshot().garbage_supply, 5000);
    // Usage above capacity gives no room ([choice] signed difference).
    h.landfill.set((20_000, 10_000));
    s.with_state(|st| st.garbage_supply = 0);
    assert_eq!(s.dispose(10.0, &mut rem), 0.0);
    // NaN and negative offers take nothing.
    assert_eq!(s.dispose(f32::NAN, &mut rem), 0.0);
    assert_eq!(s.dispose(-5.0, &mut rem), 0.0);
}

#[test]
fn valve_limits_supply() {
    let h = host(128);
    let s = sim(&h);
    s.with_state(|st| st.garbage_supply = 700_000);
    s.do_message(msg::SIM_NEW_DAY, 0, 0);
    let st = s.snapshot();
    assert_eq!(st.garbage_supply, 65_536);
    assert_eq!(st.decomposed, 2000);
}

#[test]
fn end_of_cycle_averages_and_history() {
    let h = host(128);
    let s = sim(&h);
    s.with_state(|st| {
        st.air_sum = 50_000;
        st.developed_tracts = 100;
        st.radiation_sum = 2;
        st.total_uncollected = 1000;
    });
    s.run_end_of_cycle();
    let hist = h.history.borrow().clone();
    assert_eq!(hist[0], (history::AIR, 500));
    assert_eq!(hist[2], (history::GARBAGE, 10));
    assert_eq!(hist[3], (history::RADIATION, 2));
    assert_eq!(hist[4], (history::AIR_PLUS_RADIATION, 502));
    assert_eq!(h.posts.borrow().last(), Some(&(msg::DEAL_STATUS, 8)));
    let st = s.snapshot();
    assert_eq!(
        (st.avg_air, st.avg_garbage, st.air_sum, st.developed_tracts),
        (500, 10, 0, 0)
    );
    assert!(st.garbage_uncollected);
    assert_eq!(st.efficiency, 1.0);
}

#[test]
fn slots_and_quirks() {
    let h = host(128);
    let s = sim(&h);
    let st = s.snapshot();
    st.grids[G_GARBAGE_VAL].as_ref().unwrap().set(30, 30, 0x8064u16 as i16);
    assert_eq!(s.garbage_value(30, 30), -65_436);
    assert!(s.is_active_landfill(30, 30));
    st.grids[G_AIR_VAL].as_ref().unwrap().set(6, 6, -1);
    assert!(s.is_air_polluted(12, 12));
    st.grids[G_AIR_SRC].as_ref().unwrap().set(5, 5, -5);
    assert_eq!(s.pollution_value(0, 10, 10), 0);
    assert_eq!(s.pollution_value(9, 10, 10), 0);
    assert!(s.pollution_grid(7).is_none());
    s.with_state(|st| st.garbage_supply = i32::MAX - 1);
    assert_eq!(s.add_to_garbage_supply(10), 1);
    s.with_state(|st| st.garbage_supply = -4);
    assert_eq!(s.garbage_supply(), 0);
    // Prop share: 2048 everywhere -> 5; inverted rectangle -> 0.
    st.grids[G_GARBAGE_VAL].as_ref().unwrap().fill(2048);
    assert_eq!(s.requested_share(0, 0, 3, 3), 5);
    assert_eq!(s.requested_share(3, 3, 0, 0), 0);
    // A 2^32-wide rectangle has a 32-bit cell count of 0 ([choice]).
    assert_eq!(s.requested_share(i32::MIN, i32::MIN, i32::MAX, i32::MAX), 0);
}

#[test]
fn current_capacity_rounding() {
    let h = host(64);
    let s = sim(&h);
    s.with_state(|st| st.efficiency = 0.8);
    h.set_occupant_property(7, prop::INCINERATOR, u32v(1000));
    h.set_occupant_property(
        7,
        prop::CONDITION_CURVE,
        Variant::array(
            crate::variant::mem::FLOAT32,
            [0.0f32, 1.0, 100.0, 0.5].iter().map(|&v| Elem::F32(v)).collect(),
        ),
    );
    h.set_occupant_property(7, prop::CONDITION, Variant::Scalar(Elem::F32(50.0)));
    assert_eq!(s.incinerator_current_capacity(7), (true, 600));
    assert_eq!(s.water_treatment_current_capacity(7), (false, 0));
}

#[test]
fn monthly_cost_uses_f32_products() {
    let h = host(64);
    let s = sim(&h);
    s.with_state(|st| {
        st.month.incinerated = 1000;
        st.month.landfilled = 4000;
    });
    s.do_message(msg::SIM_NEW_MONTH, 0, 0);
    assert_eq!(s.snapshot().last_month_cost, 25);
}

#[test]
fn recycling_effect_weights_served_population() {
    let h = host(64);
    let s = sim(&h);
    s.with_state(|st| st.efficiency = 1.0);
    h.population.set(1000);
    for (occ, pct, pop) in [(1u64, 50.0f32, 600.0f32), (2, 20.0, 1000.0)] {
        h.set_occupant_property(occ, prop::RECYCLING, Variant::Scalar(Elem::F32(pct)));
        h.set_occupant_property(occ, prop::POPULATION_SERVED, Variant::Scalar(Elem::F32(pop)));
        s.do_message(msg::OCCUPANT_INSERTED, occ, 0);
    }
    s.run_end_of_cycle();
    // Sorted by capacity (0.5 first): served 600 + 1000 = 1600, weighted
    // 0.5 x 600 + 0.2 x 1000 = 500 -> 500 / max(1600, 1000).
    let st = s.snapshot();
    assert_eq!(st.lists[L_REC], vec![1, 2]);
    assert!((st.recycling_effect - 0.3125).abs() < 1e-6);
}

#[test]
fn effects_follow_visibility() {
    let h = host(64);
    let s = sim(&h);
    let st = s.snapshot();
    let air = st.grids[G_AIR_VAL].clone().unwrap();
    for x in 0..4 {
        for z in 0..4 {
            air.set(x, z, 2000);
        }
    }
    s.do_message(msg::POST_CITY_INIT, 0, 0);
    assert_eq!(s.snapshot().effects[effect::AIR].iter().flatten().count(), 1);
    s.do_message(msg::PRE_VIEW_LEVEL_CHANGE, 0, 3);
    assert_eq!(s.snapshot().effects[effect::AIR].iter().flatten().count(), 0);
    assert_eq!(
        s.worst_air_tract(),
        WorstTract {
            x: 4,
            z: 4,
            value: 2000
        }
    );
}

/// Resolves references like the class-object store: host objects come
/// with a reference for the reader.
struct Resolver(Vec<crate::stream::RegisteredObject>, Rc<Host>);

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

fn record(s: &PollutionSim) -> (Vec<u8>, Vec<crate::stream::RegisteredObject>) {
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(reg.clone());
    assert!(s.write(&mut w));
    let entries = reg.borrow().entries.clone();
    (w.into_bytes(), entries)
}

#[test]
fn record_round_trip_and_versions() {
    let h = host(16);
    let s = sim(&h);
    h.set_occupant_property(3, prop::WATER_TREATMENT, u32v(10));
    s.do_message(msg::OCCUPANT_INSERTED, 3, 0);
    s.with_state(|st| {
        st.avg_air = 11;
        st.decomposed_last_month = 77;
        st.radiation_sum = 9;
    });
    let (bytes, entries) = record(&s);
    // The p11_22 layout with one occupant reference fewer (8 bytes).
    assert_eq!(bytes.len(), 504 - 8);
    let th = host(16);
    let t = sim(&th);
    let mut res = Resolver(entries.clone(), th.clone());
    assert!(t.read(&mut ByteReader::with_resolver(&bytes, &mut res)));
    let st = t.snapshot();
    assert!(st.loaded);
    assert_eq!((st.avg_air, st.decomposed_last_month, st.radiation_sum), (11, 77, 0));
    assert_eq!(st.lists[L_WTP], vec![3]);
    let (again, _) = record(&t);
    assert_eq!(again, bytes);

    // Version 5 is rejected and changes nothing.
    let mut v5 = bytes.clone();
    v5[0] = 5;
    let uh = host(16);
    let u = sim(&uh);
    let mut res = Resolver(entries.clone(), uh.clone());
    assert!(!u.read(&mut ByteReader::with_resolver(&v5, &mut res)));
    assert!(!u.snapshot().loaded);

    // Truncated records fail cleanly at every length, releasing what they took.
    let hh = host(16);
    for n in 0..bytes.len() {
        let u = sim(&hh);
        let mut res = Resolver(entries.clone(), hh.clone());
        assert!(!u.read(&mut ByteReader::with_resolver(&bytes[..n], &mut res)));
        assert!(!u.snapshot().loaded);
    }
    // Every reference taken (occupant 3, department 900) was released again.
    assert_eq!(hh.refs.borrow().get(&3).copied().unwrap_or(0), 0);
    assert_eq!(hh.refs.borrow().get(&900).copied(), Some(bytes.len() as i64));
}

#[test]
fn hostile_counts_do_not_panic() {
    let h = host(16);
    let s = sim(&h);
    s.with_state(|st| {
        st.tracts_per_day = u32::MAX;
        st.cell_x = u32::MAX;
        st.cell_z = 0x8000_0000;
        st.garbage_supply = i32::MIN;
        st.garbage_capacity = i32::MAX;
        st.developed_tracts = 1;
        st.air_sum = i32::MIN;
        st.recycling_count = 5;
        st.total_uncollected = u32::MAX;
    });
    s.do_message(msg::SIM_NEW_DAY, 0, 0);
    s.do_message(msg::SIM_NEW_MONTH, 0, 0);
    s.run_end_of_cycle();
    s.add_radiation_to_city([f32::NAN, 0.0, f32::INFINITY], 1e30, 1e30);
    s.add_radiation_to_city([1e30, 0.0, -1e30], -5.0, 3.0);
    let _ = s.requested_share(-5, -5, 1 << 20, 1 << 20);
    let _ = s.pollution_value(u32::MAX, i32::MIN, i32::MAX);
}
