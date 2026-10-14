// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests of the aura simulator over a small fake city. Expected values
//! are worked out from the spec by hand (see each comment).

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use super::*;
use crate::grid::{CityDims, CoreGrids, SimGrid};
use crate::stream::{ByteReader, ByteWriter, RefValue, SaveObject};

#[derive(Clone, Default)]
struct Occ {
    kind: u32,
    pos: [f32; 3],
    props: BTreeMap<u32, Vec<i32>>,
    dept: Option<u32>,
}

struct Fake {
    grids: CoreGrids,
    date: Cell<u32>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    lots: RefCell<Vec<IteratedLot>>,
    funding: RefCell<BTreeMap<u32, f32>>,
    tax: RefCell<[f32; 12]>,
    offset: Cell<i32>,
    occupants: RefCell<BTreeMap<u64, Occ>>,
    land_value_type: Cell<Option<i32>>,
    crime: Rc<SimGrid<u8>>,
    trip: Rc<SimGrid<u8>>,
    pops: [Rc<SimGrid<u16>>; 3],
    log: RefCell<Vec<String>>,
}

impl Fake {
    fn new(cells: i32) -> Rc<Fake> {
        let city = CityDims::square(cells);
        Rc::new(Fake {
            grids: CoreGrids { city },
            date: Cell::new(100),
            tuning: RefCell::new(Some(Rc::new(shipped_tuning()))),
            lots: RefCell::new(Vec::new()),
            funding: RefCell::new(BTreeMap::new()),
            tax: RefCell::new([9.0; 12]),
            offset: Cell::new(0),
            occupants: RefCell::new(BTreeMap::new()),
            land_value_type: Cell::new(Some(1)),
            crime: SimGrid::create(&city, 0, 4),
            trip: SimGrid::create(&city, 0, 4),
            pops: [
                SimGrid::create(&city, 0, 4),
                SimGrid::create(&city, 0, 4),
                SimGrid::create(&city, 0, 4),
            ],
            log: RefCell::new(Vec::new()),
        })
    }
    fn log(&self, s: String) {
        self.log.borrow_mut().push(s);
    }
    fn take_log(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
}

impl GridProvider<i8> for Fake {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<i8>>> {
        self.grids.create_grid(instance, tract_size)
    }
}

impl GridProvider<i16> for Fake {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<i16>>> {
        self.grids.create_grid(instance, tract_size)
    }
}

struct VecLots(std::vec::IntoIter<IteratedLot>);

impl LotIterator for VecLots {
    fn next_lot(&mut self) -> Option<IteratedLot> {
        self.0.next()
    }
}

impl AuraHost for Fake {
    fn cell_width(&self) -> (f32, f32) {
        (16.0, 16.0)
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        ((x / 16.0) as i32, (z / 16.0) as i32)
    }
    fn sim_date_number(&self) -> u32 {
        self.date.get()
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn publish_base_aura(&self, base: [i32; 3]) {
        self.log(format!("base {base:?}"));
    }
    fn lot_count(&self) -> u32 {
        self.lots.borrow().len() as u32
    }
    fn lot_iterator(&self) -> Box<dyn LotIterator> {
        Box::new(VecLots(self.lots.borrow().clone().into_iter()))
    }
    fn lot_at(&self, x: i32, z: i32) -> Option<LotInfo> {
        self.lots
            .borrow()
            .iter()
            .map(|l| l.info)
            .find(|r| (r.x1..=r.x2).contains(&x) && (r.z1..=r.z2).contains(&z))
    }
    fn land_value_type(&self, _x: i32, _z: i32) -> Option<i32> {
        self.land_value_type.get()
    }
    fn occupant_has_property(&self, occ: u64, prop: u32) -> bool {
        self.occupants
            .borrow()
            .get(&occ)
            .is_some_and(|o| o.props.contains_key(&prop))
    }
    fn occupant_sint32s(&self, occ: u64, prop: u32) -> Option<Vec<i32>> {
        self.occupants.borrow().get(&occ)?.props.get(&prop).cloned()
    }
    fn occupant_type(&self, occ: u64) -> u32 {
        self.occupants.borrow().get(&occ).map_or(0, |o| o.kind)
    }
    fn occupant_position(&self, occ: u64) -> [f32; 3] {
        self.occupants.borrow().get(&occ).map_or([0.0; 3], |o| o.pos)
    }
    fn budget_item_department(&self, occ: u64, _purpose: u32) -> Option<u32> {
        self.occupants.borrow().get(&occ)?.dept
    }
    fn department_funding(&self, dept: u32) -> Option<f32> {
        self.funding.borrow().get(&dept).copied()
    }
    fn tax_rate(&self, index: i32) -> f32 {
        self.tax.borrow()[index as usize]
    }
    fn cumulative_offset(&self, _prop: u32, _index: u32) -> i32 {
        self.offset.get()
    }
    fn pollution_grid(&self, _kind: i32) -> Option<Rc<dyn TractGrid<i16>>> {
        None
    }
    fn school_grade_map(&self) -> Option<Rc<dyn TractGrid<f32>>> {
        None
    }
    fn hospital_grade_map(&self) -> Option<Rc<dyn TractGrid<f32>>> {
        None
    }
    fn crime_grid(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        Some(self.crime.clone())
    }
    fn trip_length_map(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        Some(self.trip.clone())
    }
    fn congestion_map(&self) -> Option<Rc<dyn TractGrid<u8>>> {
        None
    }
    fn population_grids(&self) -> Option<[Rc<dyn TractGrid<u16>>; 3]> {
        let [a, b, c] = &self.pops;
        Some([a.clone(), b.clone(), c.clone()])
    }
    fn record_history(&self, series: u32, value: i32) {
        self.log(format!("history {series:#X} {value}"));
    }
    fn attempt_disaster_start(&self, manager: u32, x: i32, z: i32) {
        self.log(format!("disaster {manager:#X} {x} {z}"));
    }
    fn end_strike(&self, strike: Strike) {
        self.log(format!("strike {strike:?}"));
    }
    fn post(&self, msg_type: u32) {
        self.log(format!("post {msg_type:#X}"));
    }
    fn limit_timer_start(&self, _ms: u32) {}
    /// A clock that never reaches the deadline: with the initial step 1024
    /// the clock is read before every lot (§6.5), so an expired timer would
    /// stop every call before its first lot.
    fn limit_timer_expired(&self) -> bool {
        false
    }
    fn subscribe(&self, msgs: &[u32]) {
        self.log(format!("subscribe {}", msgs.len()));
    }
    fn unsubscribe(&self, msgs: &[u32]) {
        self.log(format!("unsubscribe {}", msgs.len()));
    }
    fn add_agent(&self, level: u32, _name: &str, flags: u32) {
        self.log(format!("agent {level} {flags}"));
    }
    fn remove_agent(&self) {
        self.log("remove agent".into());
    }
    fn registry_add(&self) -> bool {
        self.log("registry add".into());
        true
    }
    fn registry_remove(&self) {
        self.log("registry remove".into());
    }
}

/// A scripted generator: queued uniform values and integers.
struct Stub {
    uniform: Vec<f64>,
    ints: Vec<u32>,
    log: Rc<RefCell<Vec<String>>>,
}

impl AuraRandom for Stub {
    fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        let u = if self.uniform.is_empty() {
            0.5
        } else {
            self.uniform.remove(0)
        };
        self.log.borrow_mut().push("double".into());
        a + u * (b - a)
    }
    fn uniform_u32(&mut self, n: u32) -> u32 {
        self.log.borrow_mut().push(format!("int {n}"));
        if self.ints.is_empty() {
            0
        } else {
            self.ints.remove(0)
        }
    }
}

fn stub(uniform: Vec<f64>) -> (Box<dyn AuraRandom>, Rc<RefCell<Vec<String>>>) {
    let log = Rc::new(RefCell::new(Vec::new()));
    (
        Box::new(Stub {
            uniform,
            ints: Vec::new(),
            log: log.clone(),
        }),
        log,
    )
}

fn sim(f: &Rc<Fake>) -> Rc<AuraSim> {
    let (rng, _) = stub(vec![0.99]);
    let s = AuraSim::new(f.clone() as Rc<dyn AuraHost>, 7, rng);
    assert!(s.init());
    s
}

fn residential_lot(x1: i32, z1: i32, x2: i32, z2: i32, pops: Vec<(u32, i32)>) -> IteratedLot {
    IteratedLot {
        info: LotInfo {
            x1,
            z1,
            x2,
            z2,
            zone_type: 1,
        },
        building: Some(1),
        occupant_wealth: 1,
        populations: pops,
    }
}

#[test]
fn init_order_and_grids() {
    let f = Fake::new(64);
    let s = sim(&f);
    let log = f.take_log();
    assert_eq!(
        log,
        [
            "base [0, 0, 0]",
            "subscribe 5",
            "agent 0 2",
            "agent 10 2",
            "agent 4 2",
            "agent 6 2",
            "registry add"
        ]
    );
    assert_eq!(s.data().state, 1);
    let base = s.base_grid().unwrap();
    assert_eq!((base.count_x(), base.get(3, 3)), (16, -128));
    assert_eq!(s.work_grid().unwrap().count_x(), 64);
    assert!(s.init());
    assert!(f.take_log().is_empty(), "second Init does nothing");
    assert!(s.shutdown());
    assert_eq!(f.take_log(), ["registry remove", "remove agent", "unsubscribe 5"]);
    assert!(s.aura_grid().is_none() && s.data().tuning.park.is_none());
}

#[test]
fn park_and_landmark_funding() {
    let f = Fake::new(64);
    let s = sim(&f);
    let raw = s.raw_park_grid().unwrap();
    raw.set(0, 0, 100);
    raw.set(1, 0, -20);
    raw.set(2, 0, 7);
    f.funding.borrow_mut().insert(ids::DEPARTMENT_PARKS, 0.8);
    s.data_mut().state = 2;
    s.do_idle(5);
    let park = s.park_map().unwrap();
    // trunc(100 × 0.8f32) = 80; negatives are kept; trunc(7 × 0.8) = 5.
    assert_eq!((park.get(0, 0), park.get(1, 0), park.get(2, 0)), (80, -20, 5));
    assert_eq!(s.data().state, 3);

    // Two departments (§17 test 3): 0x10 at 0.5 holding 30, 0x20 at 1.2 holding 10.
    for (id, v, fund) in [(0x10u32, 30i16, 0.5f32), (0x20, 10, 1.2)] {
        let g = GridProvider::<i16>::create_grid(&*f, id, 4).unwrap();
        g.set(1, 1, v);
        s.g.borrow_mut().departments.insert(id, g);
        f.funding.borrow_mut().insert(id, fund);
    }
    s.do_idle(5);
    assert_eq!(s.landmark_work_grid().unwrap().get(1, 1), 15);
    s.do_idle(5);
    assert_eq!(s.landmark_work_grid().unwrap().get(1, 1), 27);
    assert_eq!(s.landmark_map().unwrap().get(1, 1), 0, "not published yet");
    s.do_idle(5);
    assert_eq!(s.landmark_map().unwrap().get(1, 1), 27);
    assert_eq!((s.data().state, s.data().department_cursor), (4, None));

    // A cursor whose department disappeared: no publish.
    s.data_mut().state = 3;
    s.data_mut().department_cursor = Some(0x15);
    s.landmark_work_grid().unwrap().set(1, 1, 99);
    s.do_idle(5);
    assert_eq!(s.landmark_map().unwrap().get(1, 1), 27);
    assert_eq!(s.data().state, 4);
}

#[test]
fn tax_change_and_lot_base_aura() {
    let f = Fake::new(64);
    let s = sim(&f);
    s.tax_change();
    assert_eq!(s.data().previous_tax_rate[0], 9.0);
    f.tax.borrow_mut()[0] = 10.0;
    s.tax_change();
    assert_eq!(s.data().effective_base_aura, [-5, 0, 0]);
    f.tax.borrow_mut()[0] = 7.0;
    s.tax_change();
    assert_eq!(s.data().effective_base_aura, [7, 0, 0]);
    f.tax.borrow_mut()[0] = 9.0;
    s.tax_change();
    assert_eq!(s.data().effective_base_aura, [0, 0, 0]);

    s.data_mut().effective_base_aura = [-5, 0, 0];
    s.data_mut().tuning.value_wealth = None;
    let mut lot = residential_lot(0, 0, 1, 1, vec![(0x1010, 30), (0x1020, 10)]);
    assert_eq!(s.lot_base_aura(&lot), -3); // trunc(−150 / 40)
    lot.populations.push((0x3110, 40));
    assert_eq!(s.lot_base_aura(&lot), -1); // trunc(−150 / 80)
    s.read_tuning();
    s.data_mut().effective_base_aura = [-5, 0, 0];
    f.land_value_type.set(Some(2));
    assert_eq!(s.lot_base_aura(&lot), -4); // −1 + trunc(curve(2 − 1))

    // [choice] AU10 (§6.5.1): no land value simulator → no value/wealth
    // term (the average), not type 0 (−1 + trunc(curve(0 − 1)) = 2).
    f.land_value_type.set(None);
    assert_eq!(s.lot_base_aura(&lot), -1);
    f.land_value_type.set(Some(0));
    assert_eq!(s.lot_base_aura(&lot), 2);
    f.land_value_type.set(Some(2));
    // §6.5.1 test 5: a residential entry with count 0 counts as seen
    // (trunc(0 / 40) = 0, then the term for d = 2 − 2 = 0), and the
    // entries of several maps add up ({R$ 30} and {R$ 10}: trunc(−200 / 40)).
    let mut zero = residential_lot(0, 0, 1, 1, vec![(0x1010, 0), (0x3110, 40)]);
    zero.occupant_wealth = 2;
    assert_eq!(s.lot_base_aura(&zero), 0);
    zero.populations = vec![(0x1010, 30), (0x1010, 10)];
    assert_eq!(s.lot_base_aura(&zero), -5);
    lot.populations = vec![(0x3110, 5)];
    assert_eq!(s.lot_base_aura(&lot), -128);
    lot.building = None;
    lot.populations = vec![(0x1010, 5)];
    assert_eq!(s.lot_base_aura(&lot), -128);
    // Hostile counts wrap without panicking.
    lot.building = Some(1);
    lot.populations = vec![(0x1010, i32::MAX), (0x1010, i32::MAX), (0x1020, i32::MIN)];
    s.data_mut().effective_base_aura = [i32::MAX, i32::MIN, 0];
    let _ = s.lot_base_aura(&lot);
}

/// Runs one whole cycle (state 1 to 0) and returns the number of calls.
fn run_cycle(s: &AuraSim) -> u32 {
    s.data_mut().state = 1;
    let mut n = 0;
    while s.data().state != 0 && n < 1000 {
        s.do_idle(5);
        n += 1;
    }
    n
}

#[test]
fn full_cycle_rating_and_decay() {
    let f = Fake::new(16);
    let s = sim(&f);
    // R$$ lot over cells 0..3 × 0..3 (tract (0, 0)), base aura 20 via R$$.
    let mut t = shipped_tuning();
    t.add(
        tuning::prop::BASE_AURA[1],
        crate::variant::Variant::Scalar(crate::variant::Elem::I32(20)),
    );
    *f.tuning.borrow_mut() = Some(Rc::new(t));
    s.read_tuning();
    *f.lots.borrow_mut() = vec![residential_lot(0, 0, 3, 3, vec![(0x1020, 10)])];
    f.pops[1].set(0, 0, 300);
    // Crime 40 on tract (0, 0): curve 5 per cell; trip 128 (residential) −0.0275 → trunc.
    f.crime.set(0, 0, 40);
    f.trip.set(0, 0, 128);
    let calls = run_cycle(&s);
    assert_eq!(calls, 17, "states 1..17 with one department-free landmark step");
    // work: 20 + park curve(0)=0 → 20; crime → 25; trip: trunc(25 − 0.0275) = 24.
    assert_eq!(s.work_grid().unwrap().get(2, 2), 24);
    assert_eq!(s.work_grid().unwrap().get(5, 5), -128);
    // base was −128 → clamp(24); aura = base + transient 0.
    assert_eq!(s.base_grid().unwrap().get(0, 0), 24);
    assert_eq!(s.aura_value(1, 1), 24);
    assert_eq!(s.aura_value(5, 5), 0);
    assert_eq!(s.mayor_rating(), 24);
    assert_eq!(s.max_mayor_rating(), (24, (2, 2)));
    let log = f.take_log();
    assert!(log.contains(&"history 0xA5CBF37 24".to_string()));
    assert_eq!(log.last().unwrap(), "post 0x2A09B21A");

    // Long decay with longDecay 0.9 (m = 0.100000024): old 24, new 74 + 4 = 78 → step
    // ceil(54 × m) = ceil(5.4000013) = 6.
    s.data_mut().tuning.base_aura = [0, 70, 0];
    f.offset.set(4);
    run_cycle(&s);
    assert_eq!(s.base_grid().unwrap().get(0, 0), 30);
    assert_eq!(s.data().global_effect, 4);
}

#[test]
fn long_decay_steps() {
    let f = Fake::new(4);
    let s = sim(&f);
    let check = |old: i8, new: i8, long: f32, want: i8| {
        s.data_mut().tuning.long_decay = long;
        s.work_grid().unwrap().fill(new);
        s.base_grid().unwrap().set(0, 0, old);
        s.data_mut().state = 14;
        s.do_idle(5);
        assert_eq!(
            s.base_grid().unwrap().get(0, 0),
            want,
            "old {old} new {new} decay {long}"
        );
    };
    check(10, 50, 0.9, 15);
    check(0, 10, 0.9, 2);
    check(0, -10, 0.9, -2);
    check(0, 1, 0.9, 1);
    check(-128, 77, 0.9, 77);
    check(0, 10, 0.6, 4);
    check(5, -128, 0.9, -128);
    check(120, 127, f32::NAN, 120);
}

#[test]
fn transient_effects_and_decay() {
    let f = Fake::new(64);
    let s = sim(&f);
    s.add_transient_effect(100.0, 100.0, 48.0, 10.0);
    let t = s.transient_grid().unwrap();
    assert_eq!((t.get(1, 1), t.get(2, 1)), (10, 0));
    let mut seen = Vec::new();
    for _ in 0..6 {
        s.data_mut().state = 17;
        s.do_idle(5);
        seen.push(t.get(1, 1));
    }
    assert_eq!(seen, [7, 5, 3, 2, 1, 0]);
    // Type 2 = (128 m, 25); type 3 = (0, 0); type 9 ignored.
    s.add_transient_effect_type(100.0, 100.0, 2);
    let a = t.get(1, 1);
    s.add_transient_effect_type(100.0, 100.0, 3);
    s.add_transient_effect_type(100.0, 100.0, 9);
    assert_eq!((a, t.get(1, 1)), (25, 25));
    // Hostile position: no panic. `_ftol` maps NaN and ±inf to tract 0
    // (foundations §18.1), so the effect lands at (0, 0).
    s.add_transient_effect_type(f32::NAN, f32::INFINITY, 0);
}

#[test]
fn hotspots_and_riot() {
    let f = Fake::new(256);
    let (rng, rlog) = stub(vec![0.075, 0.08]);
    let s = AuraSim::new(f.clone() as Rc<dyn AuraHost>, 1, rng);
    s.init();
    f.take_log();
    let low = vec![(10, 10, 1), (11, 10, 1), (10, 12, 1), (20, 20, 1), (9, 11, 1)];
    s.data_mut().low_tracts = low.clone();
    s.data_mut().state = 16;
    s.do_idle(5);
    assert_eq!(s.data().hotspots, [Hotspot { x: 9, z: 10, weight: 4 }]);
    assert_eq!(f.take_log(), ["disaster 0xCA380B06 38 42"]);
    s.data_mut().low_tracts = low;
    s.data_mut().state = 16;
    s.do_idle(5);
    assert!(f.take_log().is_empty(), "roll 8.0 is not below chance 8");
    assert_eq!(*rlog.borrow(), ["double", "int 1", "double", "int 1"]);
    s.data_mut().low_tracts.clear();
    s.data_mut().state = 16;
    s.do_idle(5);
    assert_eq!(rlog.borrow().len(), 5, "only the double draw without hotspots");
}

#[test]
fn cadence() {
    let f = Fake::new(16);
    let s = sim(&f);
    let day = |state: i32| {
        s.data_mut().state = state;
        s.data_mut().cycle_start_day = 100;
        f.date.set(105);
        s.do_message(&AuraMessage {
            msg_type: msg::SIM_NEW_DAY,
            ..Default::default()
        })
    };
    assert!(!day(3), "3 → 4 after the step, 4 < 5");
    assert!(day(6));
    assert!(day(0));
    s.data_mut().state = 9;
    s.do_message(&AuraMessage {
        msg_type: msg::SIM_NEW_MONTH,
        ..Default::default()
    });
    assert_eq!(s.data().state, 9);
    // Empty lot list: the cycle ends at state 4, silently.
    f.take_log();
    assert!(run_cycle(&s) <= 4);
    assert!(f.take_log().is_empty());
}

fn all_zero<T: crate::grid::GridElem>(g: &dyn TractGrid<T>) -> bool {
    (0..g.count_x()).all(|x| (0..g.count_z()).all(|z| g.get(x, z) == T::default()))
}

#[test]
fn occupant_cones() {
    let f = Fake::new(64);
    let s = sim(&f);
    *f.lots.borrow_mut() = vec![residential_lot(10, 20, 11, 21, vec![])];
    let mut occ = Occ {
        pos: [170.0, 0.0, 330.0],
        ..Default::default()
    };
    occ.props.insert(ids::PROP_PARK_EFFECT, vec![50, 22]);
    f.occupants.borrow_mut().insert(5, occ.clone());
    assert_eq!(s.centre_cell(170.0, 330.0), (10, 20));
    assert_eq!(s.centre_cell(176.0, 336.0), (11, 21));
    s.occupant_changed(5, true);
    let raw = s.raw_park_grid().unwrap();
    // Centre tract (2, 5), radius 5.5: +50 at the centre, 41 at distance 1.
    assert_eq!((raw.get(2, 5), raw.get(3, 5), raw.get(8, 5)), (50, 41, 0));
    s.occupant_changed(5, false);
    assert!(all_zero(&*raw));
    // Landmark: department grid created on insert only.
    occ.props.clear();
    occ.props.insert(ids::PROP_LANDMARK_EFFECT, vec![40, 20]);
    occ.dept = Some(0x8A41_6368);
    f.occupants.borrow_mut().insert(6, occ);
    s.occupant_changed(6, false);
    assert!(s.department_ids().is_empty());
    s.occupant_changed(6, true);
    assert_eq!(s.department_ids(), [0x8A41_6368]);
    assert_eq!(s.department_grid(0x8A41_6368).unwrap().get(2, 5), 40);
    // Preview occupants are ignored; [choice] AU4: fewer than two values is
    // absent (no cone; the original reads past the array).
    let mut p = Occ::default();
    p.props.insert(ids::PROP_PREVIEW_OCCUPANT, vec![1]);
    p.props.insert(ids::PROP_MAYOR_RATING_EFFECT, vec![7]);
    f.occupants.borrow_mut().insert(7, p);
    s.do_message(&AuraMessage {
        msg_type: msg::OCCUPANT_INSERTED,
        void1: 7,
        ..Default::default()
    });
    s.occupant_changed(7, true);
    assert!(all_zero(&*s.building_effect_grid().unwrap()));
    let mut short = Occ::default();
    short.props.insert(ids::PROP_PARK_EFFECT, vec![50]);
    short.props.insert(ids::PROP_MAYOR_RATING_EFFECT, vec![]);
    f.occupants.borrow_mut().insert(8, short);
    s.occupant_changed(8, true);
    assert!(all_zero(&*s.raw_park_grid().unwrap()));
    assert!(all_zero(&*s.building_effect_grid().unwrap()));
    // Two values or more: the first two (extra values ignored).
    let mut long = Occ::default();
    long.props.insert(ids::PROP_MAYOR_RATING_EFFECT, vec![7, 4, 99]);
    f.occupants.borrow_mut().insert(9, long);
    s.occupant_changed(9, true);
    assert_eq!(s.building_effect_grid().unwrap().get(0, 0), 7);
}

/// §11.3 (AU1), test 16: `Read` into a used object merges the departments
/// (an ID already held gets the record's grid, other IDs stay) and appends
/// the hotspots.
#[test]
fn read_into_a_used_object_merges() {
    let f = Fake::new(16);
    let src = sim(&f);
    for (id, v) in [(0x10u32, 11i16), (0x20, 22)] {
        let g = GridProvider::<i16>::create_grid(&*f, id, 4).unwrap();
        g.fill(v);
        src.g.borrow_mut().departments.insert(id, g);
    }
    src.data_mut().hotspots = vec![Hotspot { x: 9, z: 10, weight: 4 }];
    let b = record(&src);
    // The record's references in write order (§11.2): seven grids, then the
    // departments by ID.
    let refs = {
        let g = src.g.borrow();
        let mut v: Vec<RefValue> = [&g.aura, &g.base, &g.transient, &g.building_effect]
            .iter()
            .map(|x| x.as_ref().unwrap().save_ref())
            .collect();
        let i16s: Vec<RefValue> = [&g.park_map, &g.raw_park, &g.landmark_map]
            .iter()
            .map(|x| x.as_ref().unwrap().save_ref())
            .collect();
        // aura, park map, raw park, base, transient, building effect, landmark.
        v = vec![
            v[0].clone(),
            i16s[0].clone(),
            i16s[1].clone(),
            v[1].clone(),
            v[2].clone(),
            v[3].clone(),
            i16s[2].clone(),
        ];
        v.extend(g.departments.values().map(|d| d.save_ref()));
        v
    };
    struct Order(Vec<RefValue>);
    impl crate::stream::RefResolver for Order {
        fn resolve(&mut self, _class_id: u32, object_id: u32) -> Option<RefValue> {
            self.0.get((object_id as usize).wrapping_sub(1)).cloned()
        }
    }
    let mut order = Order(refs);

    let used = sim(&f);
    for (id, v) in [(0x10u32, 1i16), (0x30, 3)] {
        let g = GridProvider::<i16>::create_grid(&*f, id, 4).unwrap();
        g.fill(v);
        used.g.borrow_mut().departments.insert(id, g);
    }
    used.data_mut().hotspots = vec![Hotspot { x: 1, z: 2, weight: 5 }];
    assert!(used.read(&mut ByteReader::with_resolver(&b, &mut order)));
    assert_eq!(used.department_ids(), [0x10, 0x20, 0x30]);
    let at = |id: u32| used.department_grid(id).unwrap().get(0, 0);
    assert_eq!((at(0x10), at(0x20), at(0x30)), (11, 22, 3));
    assert_eq!(
        used.data().hotspots,
        [Hotspot { x: 1, z: 2, weight: 5 }, Hotspot { x: 9, z: 10, weight: 4 }]
    );
}

#[test]
fn bust_strike_order() {
    let f = Fake::new(16);
    let s = sim(&f);
    f.take_log();
    s.bust_strike(0);
    assert_eq!(
        f.take_log(),
        ["strike Police", "strike Fire", "strike Health", "strike School"]
    );
    s.bust_strike(3);
    s.bust_strike(5);
    assert_eq!(f.take_log(), ["strike Health"]);
    assert!(s.registry_end_strike(&[crate::variant::Elem::F32(1.0)]));
    assert_eq!(f.take_log(), ["strike Police"]);
    assert_eq!(query_interface(IID_SERIALIZABLE), Some(AuraInterface::Serializable));
    assert_eq!(query_interface(0x1234), None);
    assert_eq!((SLOT_TABLE[6].2, SLOT_TABLE[7].2), (12, 16));
}

fn record(s: &AuraSim) -> Vec<u8> {
    let reg = Rc::new(RefCell::new(crate::stream::SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(reg);
    assert!(s.write(&mut w));
    w.into_bytes()
}

#[test]
fn record_layout_and_hostile_reads() {
    let f = Fake::new(16);
    let s = AuraSim::new(f.clone() as Rc<dyn AuraHost>, 1, stub(vec![]).0);
    // Before Init every reference is null (4 bytes each).
    s.data_mut().hotspots = vec![Hotspot { x: 1, z: 2, weight: 3 }];
    s.data_mut().max_rating = -5;
    let b = record(&s);
    assert_eq!(b.len(), 2 + 7 * 4 + 4 + 8 + 2 * 10 + 4 + 12);
    assert_eq!(&b[..2], &[9, 0]);
    let t = AuraSim::new(f.clone() as Rc<dyn AuraHost>, 2, stub(vec![]).0);
    assert!(t.read(&mut ByteReader::new(&b)));
    assert_eq!(record(&t), b);
    // Versions 7 and 10 are rejected; truncated records fail without changes.
    for v in [7u8, 10] {
        let mut bad = b.clone();
        bad[0] = v;
        assert!(!t.read(&mut ByteReader::new(&bad)));
    }
    let u = AuraSim::new(f.clone() as Rc<dyn AuraHost>, 3, stub(vec![]).0);
    for n in 0..b.len() {
        assert!(!u.read(&mut ByteReader::new(&b[..n])), "truncated at {n}");
    }
    assert!(u.data().hotspots.is_empty());
    // A huge hotspot count fails at the end of the stream.
    let mut huge = b.clone();
    let at = b.len() - 16;
    huge[at..at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(!u.read(&mut ByteReader::new(&huge)));
    assert_eq!(u.data().hotspots.len(), 1, "the one whole entry before the end");
    // Version 8: one extra reference, no department block.
    let mut v8 = b[..2 + 28].to_vec();
    v8[0] = 8;
    v8.extend_from_slice(&[0, 0, 0, 0]);
    v8.extend_from_slice(&b[2 + 28 + 4..]);
    assert!(u.read(&mut ByteReader::new(&v8)));
    assert!(u.data().re_add_landmarks);
    assert_eq!(u.data().hotspots.len(), 2, "appended (§11.3, AU1)");
}

#[test]
fn hostile_host_values() {
    let f = Fake::new(16);
    let s = sim(&f);
    f.funding.borrow_mut().insert(ids::DEPARTMENT_PARKS, f32::NAN);
    *f.tax.borrow_mut() = [f32::NAN; 12];
    *f.lots.borrow_mut() = vec![
        residential_lot(i32::MIN, i32::MIN, i32::MAX, i32::MAX, vec![(0x1010, 1)]),
        residential_lot(5, 5, 2, 2, vec![(0x1010, 1)]),
    ];
    f.offset.set(i32::MIN);
    s.raw_park_grid().unwrap().set(0, 0, i16::MAX);
    s.data_mut().tuning.long_decay = f32::INFINITY;
    s.data_mut().tuning.short_decay = 1e30;
    s.data_mut().tuning.hotspot = [127, i32::MAX, i32::MIN];
    s.transient_grid().unwrap().set(0, 0, -100);
    run_cycle(&s);
    run_cycle(&s);
    assert_eq!(s.data().state, 0);
    s.occupant_changed(12345, true);
    s.data_mut().state = 99;
    s.do_idle(5);
    assert_eq!(s.data().state, 0);
}

/// §6.5: the adaptation uses `n = count >> 10`, the number of clock reads.
#[test]
fn slice_step_adaptation() {
    let mut s = SliceStep::default();
    s.adapt(31);
    assert_eq!(s, SliceStep { step: 1024, misses: 1 });
    for _ in 0..15 {
        s.adapt(21);
    }
    assert_eq!(s, SliceStep { step: 512, misses: 0 });
    s.adapt(0);
    s.adapt(5);
    s.adapt(20);
    assert_eq!(s, SliceStep { step: 512, misses: 0 }, "n = 0 and 5..=20 change nothing");
    for _ in 0..4 {
        s.adapt(1);
    }
    assert_eq!(s, SliceStep { step: 1024, misses: 0 });
    for _ in 0..4 {
        s.adapt(4);
    }
    assert_eq!(s, SliceStep { step: 1024, misses: 0 }, "the step stays at most 1024");
    let mut one = SliceStep { step: 1, misses: 15 };
    one.adapt(100);
    assert_eq!(one, SliceStep { step: 1, misses: 0 }, "the step stays at least 1");
}

/// §6, foundations §18.1: `_ftol` stores keep the low bits.
#[test]
fn integer_stores() {
    assert_eq!(store_i16(36000.0), -29536);
    assert_eq!(store_i16(40000.0), -25536);
    assert_eq!(store_i16(f32::NAN), 0);
    assert_eq!(store_i16(-2.9), -2);
    assert_eq!(clamp_store(3e9), -127);
    assert_eq!(clamp_store(-3e9), 127);
    assert_eq!(clamp_store(1e19), 0);
    assert_eq!(clamp_store(f32::NAN), 0);
    assert_eq!(clamp_store(-200.5), -127);
    assert_eq!(store_i8(128.0), -128);
    assert_eq!(store_i8(-150.0), 106);
    assert_eq!(trunc32(f64::INFINITY), 0);
}

/// §6.15 [choice]: the population tract containing the aura tract's centre.
#[test]
fn population_mapping() {
    use super::pipeline::population_tract;
    assert_eq!(population_tract(3, 5, 4, 4), (3, 5));
    assert_eq!(population_tract(3, 5, 4, 8), (1, 2));
    assert_eq!(population_tract(3, 5, 4, 2), (7, 11));
    assert_eq!(population_tract(3, 5, 4, 0), (3, 5));
    assert_eq!(population_tract(i32::MAX, 0, 4, 1), (i32::MAX, 2));
}
