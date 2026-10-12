// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests for the flammability simulator. Expected values are derived by
//! hand from `docs/specs/flammability.md` (section numbers in the comments).

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::Rc;

use super::*;
use crate::grid::{CityDims, SimGrid, TractGrid};
use crate::property::PropertyHolder;
use crate::rng::{CrtRand, FixedSeeds};
use crate::store::{load_store, save_store, LoadRegistry, Package};
use crate::stream::{ByteReader, ByteWriter, InStream, OutStream, RefValue, SaveObject, SaveRegistry};
use crate::variant::{Elem, Variant};

const B: u32 = occupant_type::BUILDING;
const P: u32 = occupant_type::PROP;
const F: u32 = occupant_type::FLORA;

#[derive(Clone)]
struct Occ {
    t: u32,
    flam: Option<i64>,
    preview: bool,
    lot: Option<LotId>,
    bounds: CellRect,
    pos: [f32; 3],
}

fn occ(t: u32, flam: i64, x: i32, z: i32) -> Occ {
    Occ {
        t,
        flam: Some(flam),
        preview: false,
        lot: None,
        bounds: (x, z, x, z),
        pos: [0.0; 3],
    }
}

#[derive(Default)]
struct Fake {
    cells: Cell<(i32, i32)>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    lots: RefCell<BTreeMap<u64, (u8, Option<u64>)>>,
    random_lot: Cell<Option<u64>>,
    factor: Cell<Option<f32>>,
    season: Cell<Option<i32>>,
    unwatered: RefCell<BTreeSet<(i32, i32)>>,
    all_zoned: Cell<bool>,
    zoned: RefCell<BTreeSet<(i32, i32)>>,
    tutorial: Cell<bool>,
    no_fire_manager: Cell<bool>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    events: RefCell<Vec<String>>,
    visits: RefCell<BTreeMap<(i32, i32), u32>>,
    /// Occupants the manager no longer lists (being removed).
    unlisted: RefCell<BTreeSet<u64>>,
}

impl Fake {
    fn new(n: i32) -> Rc<Fake> {
        let f = Fake::default();
        f.cells.set((n, n));
        f.factor.set(Some(1.0));
        Rc::new(f)
    }
    fn add(&self, id: u64, o: Occ) {
        self.occs.borrow_mut().insert(id, o);
    }
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut self.events.borrow_mut())
    }
    fn log(&self, s: String) {
        self.events.borrow_mut().push(s);
    }
}

impl crate::grid::GridProvider<u8> for Fake {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        let (x, z) = self.cells.get();
        let city = CityDims {
            cells_x: x,
            cells_z: z,
            ..CityDims::square(1)
        };
        self.log(format!("create {instance:08X} {tract_size}"));
        Some(SimGrid::<u8>::create(&city, instance, tract_size) as Rc<dyn TractGrid<u8>>)
    }
}

impl FlammabilityHost for Fake {
    fn city_cell_counts(&self) -> (i32, i32) {
        self.cells.get()
    }
    fn position_to_cell(&self, x: f32, z: f32) -> (i32, i32) {
        ((x / 16.0).floor() as i32, (z / 16.0).floor() as i32)
    }
    fn is_tutorial(&self) -> bool {
        self.tutorial.get()
    }
    fn tuning(&self) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn occupant_type(&self, o: OccupantId) -> u32 {
        self.occs.borrow().get(&o).map_or(0, |o| o.t)
    }
    fn occupant_property(&self, o: OccupantId, id: u32) -> Option<Variant> {
        let occs = self.occs.borrow();
        let o = occs.get(&o)?;
        match id {
            prop::FLAMMABILITY => o.flam.map(|v| Variant::Scalar(Elem::U8(v as u8))),
            prop::PREVIEW_OCCUPANT if o.preview => Some(Variant::Scalar(Elem::Bool(true))),
            _ => None,
        }
    }
    fn occupant_bounds(&self, o: OccupantId) -> CellRect {
        self.occs.borrow().get(&o).map_or((0, 0, -1, -1), |o| o.bounds)
    }
    fn occupant_position(&self, o: OccupantId) -> [f32; 3] {
        self.occs.borrow().get(&o).map_or([0.0; 3], |o| o.pos)
    }
    fn occupants_in_cells(&self, (x0, z0, x1, z1): CellRect) -> Vec<OccupantId> {
        self.occs
            .borrow()
            .iter()
            .filter(|(id, o)| {
                !self.unlisted.borrow().contains(id)
                    && [B, P, F].contains(&o.t)
                    && o.bounds.0 <= x1
                    && o.bounds.2 >= x0
                    && o.bounds.1 <= z1
                    && o.bounds.3 >= z0
            })
            .map(|(&id, _)| id)
            .collect()
    }
    fn occupant_lot(&self, o: OccupantId) -> Option<LotId> {
        self.occs.borrow().get(&o).and_then(|o| o.lot)
    }
    fn lot_condition(&self, lot: LotId) -> u8 {
        self.lots.borrow().get(&lot).map_or(0, |l| l.0)
    }
    fn lot_building(&self, lot: LotId) -> Option<OccupantId> {
        self.lots.borrow().get(&lot).and_then(|l| l.1)
    }
    fn random_lot(&self) -> Option<LotId> {
        self.log("random_lot".into());
        self.random_lot.get()
    }
    fn cumulative_factor(&self, _prop: u32, _index: u32) -> Option<f32> {
        self.factor.get()
    }
    fn current_season(&self) -> Option<i32> {
        self.season.get()
    }
    fn is_cell_watered(&self, x: i32, z: i32) -> bool {
        !self.unwatered.borrow().contains(&(x, z))
    }
    fn zone_type(&self, x: i32, z: i32) -> u32 {
        *self.visits.borrow_mut().entry((x, z)).or_default() += 1;
        u32::from(self.all_zoned.get() || self.zoned.borrow().contains(&(x, z)))
    }
    fn record_history(&self, t: u32, v: i32) {
        self.log(format!("history {t:08X} {v}"));
    }
    fn ignite_cell(&self, x: u32, z: u32, p: bool, t: bool, iid: u32) -> bool {
        if self.no_fire_manager.get() {
            return false;
        }
        self.log(format!("ignite {x} {z} {p} {t} {iid}"));
        true
    }
    fn set_fire_cause(&self, c: u32) {
        self.log(format!("cause {c}"));
    }
    fn post(&self, t: u32) {
        self.log(format!("post {t:08X}"));
    }
    fn grid_shutdown(&self, g: &Rc<dyn TractGrid<u8>>) {
        self.log(format!("grid_shutdown {}", g.count_x()));
    }
}

/// A scripted fire roll: queued values of `r`, the last one repeating.
type Queue = Rc<RefCell<(VecDeque<f64>, u32)>>;

struct Script(Queue);

impl FlammabilityRandom for Script {
    fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        assert_eq!((a, b), (0.0, 100.0));
        let mut s = self.0.borrow_mut();
        s.1 += 1;
        if s.0.len() > 1 {
            s.0.pop_front().unwrap_or(50.0)
        } else {
            s.0.front().copied().unwrap_or(50.0)
        }
    }
}

fn sim_with(f: &Rc<Fake>, rs: &[f64]) -> (Rc<FlammabilitySim>, Queue) {
    let script = Rc::new(RefCell::new((rs.iter().copied().collect::<VecDeque<_>>(), 0)));
    let s = FlammabilitySim::new(
        f.clone() as Rc<dyn FlammabilityHost>,
        7,
        Box::new(Script(script.clone())),
    );
    (s, script)
}

fn sim(f: &Rc<Fake>) -> Rc<FlammabilitySim> {
    sim_with(f, &[50.0]).0
}

fn day(s: &FlammabilitySim) {
    assert!(s.do_message(&Message::of_type(msg::SIM_NEW_DAY)));
}

fn shipped(f: &Fake) {
    *f.tuning.borrow_mut() = Some(Rc::new(Tuning::shipped_exemplar()));
}

/// Number of `rand()` calls from `from` to `to` (bounded search).
fn rand_calls(from: CrtRand, to: CrtRand) -> u32 {
    let mut c = from;
    for n in 0..100_000 {
        if c == to {
            return n;
        }
        c.rand();
    }
    panic!("rand state not reached");
}

#[test]
fn constructor_and_ids() {
    let f = Fake::new(4);
    let s = sim(&f);
    let d = s.snapshot();
    assert_eq!(d.global_multiplier, 0.0);
    assert_eq!(d.tuning, Tuning::default());
    assert_eq!(d.iterator, CellIterator::constructed());
    for iid in [
        IID_FLAMMABILITY_SIMULATOR,
        IID_MESSAGE_TARGET2,
        IID_SERIALIZABLE,
        IID_UNKNOWN,
    ] {
        assert!(answers_iid(iid));
    }
    assert!(!answers_iid(IID_MESSAGE_TARGET));
    assert_eq!(s.class_id(), 0xE990_C045);
}

/// §8: code defaults, shipped values, the "exactly 3 values" rule and the
/// "delete" curve pattern.
#[test]
fn tuning() {
    let t = Tuning::read(None);
    assert_eq!(
        (t.summer_multiplier, t.abandonment_multiplier, t.no_water_multiplier),
        (1.2, 1.1, 1.1)
    );
    assert_eq!(
        (
            t.hotspot_threshold,
            t.hotspot_radius,
            t.hotspot_min_weight,
            t.tract_size,
            t.cycle_period
        ),
        (196, 8, 16, 1, 40)
    );
    assert_eq!(t.random_fire_probability, 0.01);
    assert!(t.hotspot_fire_curve.is_none());
    let t = Tuning::read(Some(&Tuning::shipped_exemplar()));
    assert_eq!(
        (t.summer_multiplier, t.abandonment_multiplier, t.no_water_multiplier),
        (1.0, 1.5, 1.25)
    );
    assert_eq!(
        (t.hotspot_threshold, t.hotspot_radius, t.hotspot_min_weight),
        (54, 8, 40)
    );
    let c = t.hotspot_fire_curve.unwrap();
    for (x, y) in [
        (39.0, 0.0),
        (40.0, 7.0),
        (45.0, 11.0),
        (60.0, 17.5),
        (100.0, 25.0),
        (500.0, 25.0),
    ] {
        assert_eq!(c.eval(x), y, "f({x})");
    }
    let mut h = PropertyHolder::new();
    h.add(
        prop::HOTSPOT_SETTINGS,
        Variant::array(crate::variant::mem::UINT32, vec![Elem::U32(1), Elem::U32(2)]),
    );
    h.add(prop::HOTSPOT_FIRE_CURVE, Variant::Scalar(Elem::F32(3.0)));
    let t = Tuning::read(Some(&h));
    assert_eq!(
        (t.hotspot_threshold, t.hotspot_radius, t.hotspot_min_weight),
        (196, 8, 16)
    );
    assert!(t.hotspot_fire_curve.is_none());
}

/// §16 test 1 (with type IDs; see the module's label [choice]).
#[test]
fn occupant_raw_value_cases() {
    let f = Fake::new(8);
    let s = sim(&f);
    s.init();
    f.lots.borrow_mut().insert(1, (0, None));
    f.lots.borrow_mut().insert(4, (4, None));
    let on = |t, flam, lot| Occ {
        lot,
        ..occ(t, flam, 0, 0)
    };
    f.add(1, on(B, 150, Some(1)));
    f.add(2, on(B, 150, Some(4)));
    f.add(3, on(P, 150, Some(4)));
    f.add(4, on(F, 150, Some(4)));
    f.add(5, on(0x1234, 150, None));
    f.add(
        6,
        Occ {
            flam: None,
            ..on(B, 0, None)
        },
    );
    assert_eq!(s.occupant_raw(1), 150);
    assert_eq!(s.occupant_raw(2), 165); // code default 1.1
    assert_eq!(s.occupant_raw(3), 165, "props get the multiplier too");
    assert_eq!(s.occupant_raw(4), 150, "flora does not");
    assert_eq!(s.occupant_raw(5), 0);
    assert_eq!(s.occupant_raw(6), 0);
    assert_eq!(occupant_raw_value(150, 1.5), 225);
    assert_eq!(occupant_raw_value(200, 1.5), 255);
    assert_eq!(occupant_raw_value(3, 1.5), 4);
    assert_eq!(occupant_raw_value(5, 1.5), 8);
    assert_eq!(occupant_raw_value(-5, 1.0), 0);
    // Foundations §18: the product rounds to float32 first.
    assert_eq!(adjusted_value(50, 0.95), 48);
    assert_eq!(adjusted_value(150, 0.95), 142);
    assert_eq!(daily_value(50, 0.95, true, 1.0), 48);
    assert_eq!(occupant_raw_value(i64::MAX, f32::NAN), 0);
    assert_eq!(occupant_raw_value(i64::MIN, f32::INFINITY), 0);
}

/// §16 test 2.
#[test]
fn tract_priority() {
    let f = Fake::new(8);
    let s = sim(&f);
    s.init();
    f.add(1, occ(B, 0, 0, 0));
    f.add(2, occ(P, 80, 0, 0));
    f.add(3, occ(F, 120, 0, 0));
    f.add(4, occ(B, 30, 1, 0));
    f.add(5, occ(F, 200, 1, 0));
    f.add(6, occ(P, 10, 2, 0));
    f.add(7, occ(P, 40, 2, 0));
    assert_eq!(s.tract_raw(0, 0), 80);
    assert_eq!(s.tract_raw(1, 0), 30);
    assert_eq!(s.tract_raw(2, 0), 40);
    assert_eq!(s.tract_raw(3, 0), 0);
}

/// §16 tests 3 and 4.
#[test]
fn daily_values_and_candidates() {
    assert_eq!(daily_value(150, 1.2, false, 1.1), 198);
    assert_eq!(daily_value(150, 1.2, true, 1.1), 180);
    assert_eq!(daily_value(220, 1.0, false, 1.25), 255);
    assert_eq!(daily_value(100, 0.95, false, 1.25), 119);
    assert_eq!(daily_value(100, -1.0, true, 1.0), 0);
    assert_eq!(daily_value(100, f32::NAN, true, 1.0), 0);

    let f = Fake::new(2);
    let (s, _) = sim_with(&f, &[99.0]);
    s.init();
    let raw = s.raw_grid().unwrap();
    let adj = s.adjusted_grid().unwrap();
    // 2 × 2 city: tractsPerDay 0. Drive the cycle by hand.
    s.data_mut().tracts_per_day = 4;
    s.data_mut().global_multiplier = 1.0;
    raw.set(0, 0, 197);
    raw.set(1, 0, 196);
    adj.set(0, 1, 33); // raw 0: never written
    day(&s); // empty end of cycle, then one full cycle
    assert_eq!((adj.get(0, 0), adj.get(1, 0), adj.get(0, 1)), (197, 196, 33));
    assert_eq!(s.snapshot().candidates, vec![Hotspot::new(0, 0, 1)]);
    // Shipped threshold 54.
    s.data_mut().tuning.hotspot_threshold = 54;
    s.data_mut().candidates.clear();
    raw.set(0, 0, 55);
    raw.set(1, 0, 54);
    day(&s); // the wrap comes first: candidates cleared, then four visits
    assert_eq!(s.snapshot().candidates, vec![Hotspot::new(0, 0, 1)]);
}

/// §16 test 5, §4.1 step 7, §6.5.
#[test]
fn cadence() {
    for (n, per_day) in [(64, 102), (128, 409), (256, 1638)] {
        let f = Fake::new(n);
        let s = sim(&f);
        s.init();
        assert_eq!(s.snapshot().tracts_per_day, per_day);
        assert!(f.take().iter().all(|e| e.starts_with("create")));
        if n == 64 {
            let mut ends = Vec::new();
            let mut crt = s.crt();
            for d in 1..=161 {
                day(&s);
                let posts = f.take().iter().filter(|e| e.starts_with("post")).count();
                let calls = rand_calls(crt, s.crt());
                crt = s.crt();
                assert_eq!(calls as usize, posts, "one rand() per wrap, no hotspots");
                if posts > 0 {
                    ends.push(d);
                }
            }
            assert_eq!(ends, vec![1, 41, 81, 121, 161]);
        } else {
            for _ in 0..40 {
                day(&s);
            }
            let posts = f.take().iter().filter(|e| e.starts_with("post")).count();
            assert_eq!(posts, 1, "only the empty first end in days 1..40");
            day(&s);
            assert_eq!(f.take().iter().filter(|e| e.starts_with("post")).count(), 1, "day 41");
        }
    }
}

/// §6.5: each tract once per cycle (driven visit by visit).
#[test]
fn every_tract_once_per_cycle() {
    let f = Fake::new(16);
    let s = sim(&f);
    s.init();
    s.data_mut().tracts_per_day = 256;
    f.take();
    day(&s); // the empty end, then exactly one full cycle
    assert_eq!(f.take().iter().filter(|e| e.starts_with("post")).count(), 1);
    {
        let v = f.visits.borrow();
        assert_eq!(v.len(), 256);
        assert!(v.values().all(|&c| c == 1));
    }
    day(&s); // the wrap first, then the next full cycle
    assert_eq!(f.take().iter().filter(|e| e.starts_with("post")).count(), 1);
    assert!(f.visits.borrow().values().all(|&c| c == 2));
}

/// §16 test 6.
#[test]
fn average_and_history() {
    let f = Fake::new(4);
    let s = sim(&f);
    s.init();
    f.take();
    s.data_mut().zoned_sum = 150;
    s.data_mut().zoned_count = 3;
    s.end_of_cycle();
    assert_eq!(s.average_flammability(), 50);
    let ev = f.take();
    assert_eq!(ev[0], "post 08F415FE");
    assert_eq!(ev[1], "history 2A11CB48 50");
    s.end_of_cycle();
    assert_eq!(s.average_flammability(), 0);
    assert_eq!(f.take()[1], "history 2A11CB48 0");
    // Through the daily refresh: zoned tracts with 100, 50, 0, one unzoned.
    let f = Fake::new(2);
    let s = sim(&f);
    s.init();
    s.data_mut().global_multiplier = 1.0;
    let raw = s.raw_grid().unwrap();
    raw.set(0, 0, 100);
    raw.set(1, 0, 50);
    raw.set(1, 1, 250);
    f.zoned.borrow_mut().extend([(0, 0), (1, 0), (0, 1)]);
    s.data_mut().tracts_per_day = 4;
    day(&s);
    day(&s);
    let ev = f.take();
    assert!(ev.contains(&"history 2A11CB48 50".to_string()), "{ev:?}");
}

/// §16 test 7.
#[test]
fn hotspots() {
    let c = [
        Hotspot::new(10, 10, 1),
        Hotspot::new(12, 13, 1),
        Hotspot::new(30, 30, 1),
        Hotspot::new(11, 10, 1),
    ];
    assert_eq!(compute_hotspots(&c[..2], 8, 0), vec![Hotspot::new(11, 11, 2)]);
    assert_eq!(
        compute_hotspots(&c, 8, 0),
        vec![Hotspot::new(11, 10, 3), Hotspot::new(30, 30, 1)]
    );
    assert_eq!(compute_hotspots(&c, 8, 2), vec![Hotspot::new(11, 10, 3)]);
    assert_eq!(compute_hotspots(&c, 0, 1).len(), 4);
    let far = [Hotspot::new(i32::MIN, 0, 1), Hotspot::new(i32::MAX, 0, 1)];
    assert_eq!(compute_hotspots(&far, i32::MAX, 1).len(), 2);
}

/// §16 test 8.
#[test]
fn flammability_hotspot_slot() {
    let f = Fake::new(32);
    let s = sim(&f);
    assert_eq!(s.flammability_hotspot(), None, "no grid");
    s.init();
    let adj = s.adjusted_grid().unwrap();
    s.data_mut().hotspots = vec![Hotspot::new(5, 5, 40), Hotspot::new(20, 20, 45)];
    adj.set(5, 5, 70);
    assert_eq!(s.flammability_hotspot(), None);
    adj.set(20, 20, 90);
    assert_eq!(s.flammability_hotspot(), Some((90, 20, 20)));
    s.data_mut().hotspots = vec![Hotspot::new(5, 5, 45), Hotspot::new(20, 20, 45)];
    assert_eq!(s.flammability_hotspot(), Some((70, 5, 5)));
    s.data_mut().hotspots = vec![Hotspot::new(20, 20, 0), Hotspot::new(20, 20, -3)];
    assert_eq!(s.flammability_hotspot(), None, "weights must be > 0");
    s.data_mut().hotspots.clear();
    assert_eq!(s.flammability_hotspot(), None);
}

/// §16 test 9.
#[test]
fn global_multiplier_cases() {
    let g = global_multiplier(Some(0.95), Some(SUMMER), 1.2);
    assert_eq!(g, (0.95f64 as f32 as f64 * 1.2f32 as f64) as f32);
    assert!((g - 1.14).abs() < 1e-6);
    assert_eq!(global_multiplier(Some(0.95), Some(SUMMER), 1.0), 0.95);
    assert_eq!(global_multiplier(Some(0.95), Some(1), 1.2), 0.95);
    assert_eq!(global_multiplier(None, None, 1.2), 1.0);
    let f = Fake::new(4);
    shipped(&f);
    let s = sim(&f);
    s.init();
    f.add(1, occ(B, 100, 1, 1));
    s.do_message(&Message {
        msg_type: msg::INSERT_OCCUPANT,
        occupant: Some(1),
        lot: None,
    });
    assert_eq!(s.raw_grid().unwrap().get(1, 1), 100);
    assert_eq!(s.adjusted_grid().unwrap().get(1, 1), 0, "a new city: global 0");
    assert!(!s.is_cell_flammable(1, 1));
    f.factor.set(Some(0.95));
    f.season.set(Some(SUMMER));
    s.do_message(&Message::of_type(msg::SEASON_CHANGE));
    assert_eq!(s.snapshot().global_multiplier, 0.95);
    f.factor.set(Some(1.0));
    s.do_message(&Message::of_type(msg::ORDINANCE_UPDATED));
    assert_eq!(s.snapshot().global_multiplier, 1.0);
    assert_eq!(s.adjusted_grid().unwrap().get(1, 1), 0, "grids are not touched");
}

fn fire_setup(rs: &[f64]) -> (Rc<Fake>, Rc<FlammabilitySim>, Queue) {
    let f = Fake::new(32);
    shipped(&f);
    let (s, script) = sim_with(&f, rs);
    s.init();
    f.take();
    (f, s, script)
}

/// §16 test 10.
#[test]
fn fire_roll() {
    // r = 3 < 5, a lot with a building at world (100, 200).
    let (f, s, _) = fire_setup(&[3.0]);
    f.add(
        9,
        Occ {
            pos: [100.0, 0.0, 200.0],
            ..occ(B, 10, 6, 12)
        },
    );
    f.lots.borrow_mut().insert(1, (0, Some(9)));
    f.random_lot.set(Some(1));
    s.fire_roll();
    assert_eq!(f.take(), vec!["random_lot", "ignite 6 12 true false 0", "cause 5"]);
    // r = 3, no lot, one hotspot (7, 9, 40): hotspot fire without probability.
    f.random_lot.set(None);
    s.data_mut().hotspots = vec![Hotspot::new(7, 9, 40)];
    s.fire_roll();
    assert_eq!(f.take(), vec!["random_lot", "ignite 7 9 false false 0", "cause 5"]);
    // r = 6 with weight 40 (curve 7) → fire; r = 7 → none.
    let (f, s, _) = fire_setup(&[6.0, 7.0]);
    s.data_mut().hotspots = vec![Hotspot::new(7, 9, 40)];
    s.fire_roll();
    assert_eq!(f.take(), vec!["ignite 7 9 false false 0", "cause 5"]);
    s.fire_roll();
    assert!(f.take().is_empty());
    // Tutorial: no fire and no draw.
    let (f, s, script) = fire_setup(&[0.0]);
    f.tutorial.set(true);
    s.data_mut().hotspots = vec![Hotspot::new(7, 9, 100)];
    s.fire_roll();
    assert!(f.take().is_empty());
    assert_eq!(script.borrow().1, 0);
    // No curve: only random fires.
    let (f, s, _) = fire_setup(&[0.0]);
    s.data_mut().tuning.hotspot_fire_curve = None;
    s.data_mut().hotspots = vec![Hotspot::new(7, 9, 100)];
    let crt = s.crt();
    s.fire_roll();
    assert_eq!(f.take(), vec!["random_lot"]);
    assert_eq!(s.crt(), crt, "no shuffle without a curve");
    // A missing fire manager skips the ignition and SetCause.
    let (f, s, _) = fire_setup(&[50.0]);
    f.no_fire_manager.set(true);
    s.data_mut().hotspots = vec![Hotspot::new(7, 9, 100)];
    s.fire_roll();
    assert!(f.take().is_empty());
}

/// §16 test 11, and the shuffle order (foundations §12.3).
#[test]
fn rand_count() {
    let (f, s, _) = fire_setup(&[99.0]);
    s.data_mut().hotspots = (0..4).map(|i| Hotspot::new(i, i, 40)).collect();
    s.data_mut().tracts_per_day = 1;
    let crt = s.crt();
    day(&s); // the empty end rebuilds an empty list → no shuffle
    assert_eq!(rand_calls(crt, s.crt()), 1);
    assert!(s.snapshot().hotspots.is_empty());
    // An EndOfCycle that reaches the hotspot test with 4 hotspots (r = 99:
    // no fire): 3 shuffle calls, then the wrap's call.
    s.data_mut().hotspots = (0..4).map(|i| Hotspot::new(i, i, 40)).collect();
    let before = s.crt();
    s.fire_roll();
    assert!(f.take().iter().all(|e| !e.starts_with("ignite")));
    assert_eq!(rand_calls(before, s.crt()), 3);
    // With r < prob and a building: no shuffle.
    let (f, s, _) = fire_setup(&[1.0]);
    f.add(9, occ(B, 10, 0, 0));
    f.lots.borrow_mut().insert(1, (0, Some(9)));
    f.random_lot.set(Some(1));
    s.data_mut().hotspots = (0..4).map(|i| Hotspot::new(i, i, 40)).collect();
    let before = s.crt();
    s.fire_roll();
    assert_eq!(s.crt(), before);
    // The shuffled order picks the hotspot: CRT state 1 → rand 41, 18467,
    // 6334 → [0, 1, 3, 2]; only index 3 qualifies → it is found anyway.
    let (f, s, _) = fire_setup(&[50.0]);
    s.data_mut().hotspots = vec![
        Hotspot::new(1, 1, 1),
        Hotspot::new(2, 2, 1),
        Hotspot::new(3, 3, 1),
        Hotspot::new(4, 4, 100),
    ];
    s.data_mut().tuning.hotspot_fire_curve = Some(ResponseCurve::from_points(&[(0.0, 0.0), (100.0, 60.0)]));
    s.fire_roll();
    assert_eq!(f.take(), vec!["ignite 4 4 false false 0", "cause 5"]);
    // All qualify: the first of the shuffled order. From state 10:
    // rand() = 71, 16899, 3272 → i=1: 71%2=1 (stay); i=2: 16899%3=0
    // → [2, 1, 0, 3]; i=3: 3272%4=0 → [3, 1, 0, 2]: index 3 first.
    let (f, s, _) = fire_setup(&[50.0]);
    s.set_crt(CrtRand::new(10));
    s.data_mut().hotspots = (1..=4).map(|i| Hotspot::new(i, i, 100)).collect();
    s.data_mut().tuning.hotspot_fire_curve = Some(ResponseCurve::from_points(&[(0.0, 60.0)]));
    s.fire_roll();
    assert_eq!(f.take()[0], "ignite 4 4 false false 0");
}

/// §16 test 12, §14.3, §14.5.
#[test]
fn occupant_update() {
    let f = Fake::new(8);
    let s = sim(&f);
    s.init();
    let g = 0.95f32;
    s.data_mut().global_multiplier = g;
    let (raw, adj) = (s.raw_grid().unwrap(), s.adjusted_grid().unwrap());
    let ins = |id: u64, t| {
        s.do_message(&Message {
            msg_type: t,
            occupant: Some(id),
            lot: None,
        })
    };
    f.add(1, occ(B, 100, 2, 3));
    ins(1, msg::INSERT_OCCUPANT);
    assert_eq!((raw.get(2, 3), adj.get(2, 3)), (100, 95));
    f.add(2, occ(F, 120, 2, 3));
    ins(2, msg::INSERT_OCCUPANT);
    assert_eq!((raw.get(2, 3), adj.get(2, 3)), (120, 114));
    f.add(3, occ(F, 50, 2, 3));
    ins(3, msg::CHANGE_OCCUPANT);
    // 50 × 0.95f32 = 47.4999994 rounds to float32 47.5 (foundations §18),
    // roundHE(47.5) = 48.
    assert_eq!(
        (raw.get(2, 3), adj.get(2, 3)),
        (120, 48),
        "adjusted from the new occupant's v"
    );
    // Remove the building: a full recompute from the occupants the manager
    // still lists (the building is no longer listed, §6.3 [inferred]).
    f.unlisted.borrow_mut().insert(1);
    ins(1, msg::REMOVE_OCCUPANT);
    assert_eq!((raw.get(2, 3), adj.get(2, 3)), (120, 114), "flora 120 still present");
    f.unlisted.borrow_mut().insert(2);
    ins(2, msg::REMOVE_OCCUPANT);
    assert_eq!((raw.get(2, 3), adj.get(2, 3)), (50, 48));
    // A handle the host no longer knows has type 0: ignored.
    f.occs.borrow_mut().remove(&3);
    ins(3, msg::REMOVE_OCCUPANT);
    assert_eq!(raw.get(2, 3), 50);
    // Preview occupants are ignored.
    f.add(
        4,
        Occ {
            preview: true,
            ..occ(B, 250, 5, 5)
        },
    );
    for t in [msg::INSERT_OCCUPANT, msg::CHANGE_OCCUPANT, msg::REMOVE_OCCUPANT] {
        ins(4, t);
    }
    assert_eq!(raw.get(5, 5), 0);
    // Insert with v = 0 does nothing; out-of-city rectangles.
    f.add(5, occ(F, 0, 6, 6));
    adj.set(6, 6, 9);
    ins(5, msg::INSERT_OCCUPANT);
    assert_eq!(adj.get(6, 6), 9);
    f.add(
        6,
        Occ {
            bounds: (6, 6, 9, 7),
            ..occ(F, 40, 0, 0)
        },
    );
    ins(6, msg::INSERT_OCCUPANT);
    assert_eq!((raw.get(7, 7), raw.get(6, 6), raw.get(7, 8)), (40, 40, 0));
    f.add(
        7,
        Occ {
            bounds: (20, 20, 30, 30),
            ..occ(F, 77, 0, 0)
        },
    );
    ins(7, msg::INSERT_OCCUPANT);
    assert_eq!(raw.get(0, 0), 77, "disjoint collapses to tract (0, 0)");
    f.add(
        8,
        Occ {
            bounds: (i32::MIN, i32::MIN, i32::MAX, i32::MAX),
            ..occ(F, 1, 0, 0)
        },
    );
    ins(8, msg::INSERT_OCCUPANT);
    assert_eq!(raw.get(7, 0), 1);
    // A null occupant is ignored.
    s.do_message(&Message::of_type(msg::INSERT_OCCUPANT));
}

/// §16 test 13.
#[test]
fn condition_changed() {
    let f = Fake::new(8);
    let s = sim(&f);
    s.init();
    s.data_mut().global_multiplier = 1.0;
    f.lots.borrow_mut().insert(1, (4, Some(1)));
    f.add(
        1,
        Occ {
            lot: Some(1),
            bounds: (1, 1, 2, 1),
            ..occ(B, 150, 0, 0)
        },
    );
    s.do_message(&Message {
        msg_type: msg::CONDITION_CHANGED,
        occupant: None,
        lot: Some(1),
    });
    let raw = s.raw_grid().unwrap();
    assert_eq!((raw.get(1, 1), raw.get(2, 1)), (165, 165));
    assert_eq!(s.adjusted_grid().unwrap().get(2, 1), 165);
    raw.set(1, 1, 3);
    s.do_message(&Message::of_type(msg::CONDITION_CHANGED));
    s.do_message(&Message {
        msg_type: msg::CONDITION_CHANGED,
        occupant: None,
        lot: Some(99),
    });
    assert_eq!(raw.get(1, 1), 3);
}

fn store_round_trip(s: &Rc<FlammabilitySim>, f: &Rc<Fake>) -> (Package, Rc<FlammabilitySim>) {
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut root = ByteWriter::with_registry(reg.clone());
    root.put_ref(&RefValue::Object(s.clone() as Rc<dyn SaveObject>));
    let mut pkg = Package::new();
    let rep = save_store(&reg, &mut pkg);
    assert!(!rep.aborted);
    let root = root.into_bytes();
    let fc = f.clone();
    let mut factory = move |class: u32, _id: u32| -> Option<RefValue> {
        if class == CLSID_FLAMMABILITY_SIMULATOR {
            let s = FlammabilitySim::new(
                fc.clone() as Rc<dyn FlammabilityHost>,
                9,
                Box::new(Script(Rc::default())),
            );
            return Some(RefValue::Object(s as Rc<dyn SaveObject>));
        }
        crate::grid::create_for_clsid(class).map(RefValue::Object)
    };
    let mut lr = LoadRegistry::new(BTreeMap::new(), Box::new(&mut factory));
    let report = load_store(&pkg, &mut lr);
    assert!(report.index_ok && report.failed_classes.is_empty());
    let mut r = ByteReader::with_resolver(&root, &mut lr);
    let loaded = r.get_ref().unwrap().object::<FlammabilitySim>().unwrap();
    (pkg, loaded)
}

/// §16 test 14, §10.
#[test]
fn persistence() {
    let f = Fake::new(16);
    let s = sim(&f);
    s.init();
    s.raw_grid().unwrap().set(11, 10, 200);
    s.adjusted_grid().unwrap().set(11, 10, 190);
    {
        let mut d = s.data_mut();
        d.global_multiplier = 0.95;
        d.average = 50;
        d.hotspots = vec![Hotspot::new(11, 10, 3)];
    }
    let (pkg, l) = store_round_trip(&s, &f);
    let (pkg2, _) = store_round_trip(&l, &f);
    assert_eq!(pkg, pkg2, "write → read → write is byte-identical");
    assert_eq!(l.raw_grid().unwrap().get(11, 10), 200);
    assert_eq!(l.adjusted_grid().unwrap().get(11, 10), 190);
    let rec = &pkg[&(CLSID_FLAMMABILITY_SIMULATOR, crate::store::STORE_GROUP, 0)];
    let payload = &rec[12..];
    assert_eq!(&payload[..2], &[2, 0]);
    assert_eq!(
        &payload[18..],
        &[0x33, 0x33, 0x73, 0x3F, 0x32, 1, 0, 0, 0, 0x0B, 0, 0, 0, 0x0A, 0, 0, 0, 3, 0, 0, 0]
    );
    // Versions 1 and 3 are rejected.
    for v in [1u16, 3] {
        let mut b = payload.to_vec();
        b[..2].copy_from_slice(&v.to_le_bytes());
        let t = sim(&f);
        assert!(!t.read(&mut ByteReader::new(&b)));
        assert!(t.snapshot().hotspots.is_empty());
    }
    // A truncated record: false, no garbage loop.
    let t = sim(&f);
    let mut w = ByteWriter::new();
    w.put_u16(2);
    w.put_u32(0);
    w.put_u32(0);
    w.put_f32(1.0);
    w.put_u8(7);
    w.put_u32(u32::MAX);
    w.put_i32(1);
    let b = w.into_bytes();
    assert!(!t.read(&mut ByteReader::new(&b)));
    let d = t.snapshot();
    assert_eq!((d.global_multiplier, d.average, d.hotspots.len()), (1.0, 7, 0));
    assert!(!t.read(&mut ByteReader::new(&b[..1])));
    // Init on the loaded object keeps the grids; the first day clears the
    // hotspots and the average (§14.1).
    f.take();
    l.init();
    assert!(f.take().is_empty(), "no grid created after a load");
    assert_eq!(l.snapshot().hotspots.len(), 1);
    day(&l);
    let d = l.snapshot();
    assert_eq!((d.average, d.hotspots.len()), (0, 0));
    assert_eq!(d.global_multiplier, 0.95, "kept from the save (§14.2)");
}

/// §14.4 [choice]: a loaded grid with S ≠ 1 is replaced at Init.
#[test]
fn loaded_tract_size() {
    let f = Fake::new(8);
    let s = sim(&f);
    let g = SimGrid::<u8>::create(&CityDims::square(8), RAW_GRID_INSTANCE, 2);
    let mut w = ByteWriter::new();
    w.put_u16(2);
    w.put_u32(0);
    w.put_u32(0);
    let _ = g;
    w.put_f32(0.0);
    w.put_u8(0);
    w.put_u32(0);
    assert!(s.read(&mut ByteReader::new(&w.into_bytes())));
    assert!(s.raw_grid().is_none());
    s.init();
    assert_eq!(s.raw_grid().unwrap().geometry().tract_size, 1);
    assert_eq!(f.take(), vec!["create 49D5B953 1", "create 49D5B964 1"]);
}

/// §16 test 15 (portable part), §4.2, §4.3.
#[test]
fn slots_and_lifecycle() {
    let f = Fake::new(8);
    let s = sim(&f);
    assert!(s.init());
    assert!(s.init());
    assert_eq!(f.take().len(), 2);
    let adj = s.adjusted_grid().unwrap();
    let raw = s.raw_grid().unwrap();
    assert!(!Rc::ptr_eq(&adj, &raw));
    adj.set(3, 4, 1);
    assert!(s.is_cell_flammable(3, 4));
    assert!(!s.is_cell_flammable(4, 3));
    assert!(!s.is_cell_flammable(-1, 4) && !s.is_cell_flammable(8, 0) && !s.is_cell_flammable(0, 8));
    s.data_mut().average = 77;
    assert_eq!(s.average_flammability(), 77);
    assert_eq!(s.add_ref(), 1);
    assert_eq!(s.add_ref(), 2);
    assert_eq!(s.release(), 1);
    s.data_mut().hotspots = vec![Hotspot::new(1, 1, 1)];
    assert_eq!(s.release(), 0);
    let d = s.snapshot();
    assert!(d.destroyed && !d.initialized);
    assert_eq!(d.hotspots.len(), 1, "Shutdown keeps the hotspots");
    assert!(d.tuning.hotspot_fire_curve.is_none());
    assert_eq!(f.take(), vec!["grid_shutdown 8", "grid_shutdown 8"]);
    assert!(s.adjusted_grid().is_none());
    assert!(s.shutdown());
    assert!(f.take().is_empty());
}

/// The real generator: seed 0, double path from the process clock.
#[test]
fn real_generator() {
    let seeds = FixedSeeds {
        wall_clock: 5,
        process_clock: 1,
    };
    let mut r = FlammabilityRng::new(&seeds);
    let v = r.range_f64(0.0, 100.0);
    // mul := 1 × 0x278DDE6D; int32 / 2^32 + 0.5.
    let want = (f64::from(0x278D_DE6D_u32 as i32) / 4_294_967_296.0 + 0.5) * 100.0;
    assert_eq!(v, want);
    assert_eq!(r.mul_state(), 0x278D_DE6D);
}

/// Hostile inputs: no panics, bounded loops.
#[test]
fn hostile() {
    let f = Fake::new(0);
    let s = sim(&f);
    s.init();
    assert_eq!(s.snapshot().tracts_per_day, 0);
    for _ in 0..3 {
        day(&s);
    }
    let f = Fake::new(i32::MAX);
    let s = sim(&f);
    s.init();
    assert_eq!(s.snapshot().count_x, crate::grid::MAX_TRACTS);
    let s = sim(&Fake::new(4));
    s.data_mut().tracts_per_day = 5;
    day(&s); // no grids, count-0 iterator: one end, then stops
    let mut b = vec![2u8, 0];
    b.extend([0xFF; 3]);
    assert!(!s.read(&mut ByteReader::new(&b)));
}
