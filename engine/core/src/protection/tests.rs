// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! Unit tests of the simulator against a scripted host. Expected values come
//! from the spec's worked examples (§18) or are derived by hand in the
//! comments.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;

use super::*;
use crate::grid::{CityDims, CoreGrids, SimGrid};
use crate::stream::{ByteReader, ByteWriter, OutStream, RefResolver, RefValue, SaveObject, SaveRegistry};
use crate::variant::{mem, Elem};

const BUILDING: u32 = OCCUPANT_TYPE_BUILDING;

#[derive(Default)]
struct Fake {
    log: RefCell<Vec<String>>,
    cells: Cell<i32>,
    props: RefCell<HashMap<(u64, u32), Variant>>,
    types: RefCell<HashMap<u64, u32>>,
    pos: RefCell<HashMap<u64, [f32; 3]>>,
    prop_occ: RefCell<Vec<u64>>,
    road: RefCell<HashMap<u64, bool>>,
    items: RefCell<HashMap<(u64, u32), BudgetItem>>,
    dept_ids: RefCell<HashMap<u64, u32>>,
    dept_funding: Cell<f32>,
    day: Cell<u32>,
    zoned: RefCell<Vec<(i32, i32)>>,
    dispatches: RefCell<Vec<(u32, DispatchInfo)>>,
    police_pct: Cell<Option<f32>>,
    tuning: RefCell<Option<Rc<PropertyHolder>>>,
    lots: RefCell<HashMap<u64, (i32, i32)>>,
    refs: RefCell<HashMap<u64, i32>>,
}

impl Fake {
    fn new(cells: i32) -> Rc<Fake> {
        let f = Fake::default();
        f.cells.set(cells);
        f.dept_funding.set(1.0);
        Rc::new(f)
    }
    fn push(&self, s: String) {
        self.log.borrow_mut().push(s);
    }
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
    fn set(&self, o: u64, id: u32, v: Variant) {
        self.props.borrow_mut().insert((o, id), v);
    }
    fn unset(&self, o: u64, id: u32) {
        self.props.borrow_mut().remove(&(o, id));
    }
    fn get_u32(&self, o: u64, id: u32) -> Option<u32> {
        self.props
            .borrow()
            .get(&(o, id))
            .and_then(|v| v.items().first().map(|e| e.as_i64() as u32))
    }
    fn get_u32s(&self, o: u64, id: u32) -> Vec<u32> {
        self.props
            .borrow()
            .get(&(o, id))
            .map_or(Vec::new(), |v| v.items().iter().map(|e| e.as_i64() as u32).collect())
    }
    /// A station building: centre, outer, radius, position, units.
    #[allow(clippy::too_many_arguments)]
    fn station(&self, o: u64, kind: ProtectionKind, centre: i32, outer: i32, radius: f32, at: [f32; 3], units: u32) {
        let p = kind.params();
        self.types.borrow_mut().insert(o, BUILDING);
        self.set(o, p.effect_props[0], Variant::Scalar(Elem::I32(centre)));
        self.set(o, p.effect_props[1], Variant::Scalar(Elem::I32(outer)));
        self.set(o, p.effect_props[2], Variant::Scalar(Elem::F32(radius)));
        self.set(o, p.unit_count_prop, Variant::Scalar(Elem::U32(units)));
        self.pos.borrow_mut().insert(o, at);
    }
    fn item(&self, o: u64, purpose: u32, local: f32, locally_funded: bool) {
        self.items.borrow_mut().insert(
            (o, purpose),
            BudgetItem {
                locally_funded,
                local_funding: local,
                cost: 1000,
                department: Some(0xD0FF),
            },
        );
    }
}

impl GridProvider<u8> for Fake {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<u8>>> {
        self.push(format!("create_grid u8 {instance:#x} {tract_size}"));
        CoreGrids {
            city: CityDims::square(self.cells.get()),
        }
        .create_grid(instance, tract_size)
    }
}

impl GridProvider<i16> for Fake {
    fn create_grid(&self, instance: u32, tract_size: i32) -> Option<Rc<dyn TractGrid<i16>>> {
        self.push(format!("create_grid i16 {instance:#x} {tract_size}"));
        CoreGrids {
            city: CityDims::square(self.cells.get()),
        }
        .create_grid(instance, tract_size)
    }
}

impl ProtectionHost for Fake {
    fn acquire_services(&self, kind: ProtectionKind) {
        self.push(format!("acquire {kind:?}"));
    }
    fn release_services(&self, kind: ProtectionKind) {
        self.push(format!("release_services {kind:?}"));
    }
    fn add_agent(&self, name: &str, levels: &[u32], flags: u32) {
        self.push(format!("add_agent {name} {levels:?} {flags}"));
    }
    fn remove_agent(&self) {
        self.push("remove_agent".into());
    }
    fn subscribe(&self, t: &[u32]) {
        self.push(format!("subscribe {}", t.len()));
    }
    fn unsubscribe(&self, t: &[u32]) {
        self.push(format!("unsubscribe {}", t.len()));
    }
    fn register_dispatch_source(&self) {
        self.push("register_dispatch".into());
    }
    fn unregister_dispatch_source(&self) {
        self.push("unregister_dispatch".into());
    }
    fn set_adjusted_instance(&self, _g: &Rc<dyn TractGrid<u8>>, i: u32) {
        self.push(format!("instance u8 {i:#x}"));
    }
    fn set_unadjusted_instance(&self, _g: &Rc<dyn TractGrid<i16>>, i: u32) {
        self.push(format!("instance i16 {i:#x}"));
    }
    fn shutdown_adjusted(&self, _g: &Rc<dyn TractGrid<u8>>) {
        self.push("shutdown u8".into());
    }
    fn shutdown_unadjusted(&self, _g: &Rc<dyn TractGrid<i16>>) {
        self.push("shutdown i16".into());
    }
    fn add_ref(&self, t: u64) {
        *self.refs.borrow_mut().entry(t).or_default() += 1;
    }
    fn release(&self, t: u64) {
        *self.refs.borrow_mut().entry(t).or_default() -= 1;
    }
    fn tuning(&self, _kind: ProtectionKind) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().clone()
    }
    fn create_department(&self, id: u32, group: u32, name: (u32, u32), max: f32) -> Option<ObjRef> {
        self.push(format!("department {id:#x} {group:#x} {:#x} {max}", name.1));
        let token = 0xD000 + u64::from(id & 0xFF);
        self.dept_ids.borrow_mut().insert(token, id);
        self.add_ref(token);
        Some(ObjRef {
            token,
            class_id: 0xDDDD,
        })
    }
    fn department_id(&self, dept: u64) -> u32 {
        self.dept_ids.borrow().get(&dept).copied().unwrap_or(0)
    }
    fn department_funding(&self, _dept: u64) -> f32 {
        self.dept_funding.get()
    }
    fn add_locally_funded(&self, _d: u64, o: u64, p: u32) {
        self.push(format!("add_funded {o} {p:#x}"));
    }
    fn remove_locally_funded(&self, _d: u64, o: u64, p: u32) {
        self.push(format!("remove_funded {o} {p:#x}"));
    }
    fn set_local_funding(&self, _d: u64, o: u64, p: f32, purpose: u32) -> bool {
        self.push(format!("set_funding {o} {p} {purpose:#x}"));
        true
    }
    fn budget_item(&self, o: u64, purpose: u32) -> Option<BudgetItem> {
        self.items.borrow().get(&(o, purpose)).copied()
    }
    fn occupant_type(&self, o: u64) -> u32 {
        self.types.borrow().get(&o).copied().unwrap_or(0)
    }
    fn occupant_property(&self, o: u64, id: u32) -> Option<Variant> {
        self.props.borrow().get(&(o, id)).cloned()
    }
    fn set_occupant_u32(&self, o: u64, id: u32, v: u32) {
        self.set(o, id, Variant::Scalar(Elem::U32(v)));
    }
    fn set_occupant_u32s(&self, o: u64, id: u32, v: &[u32]) {
        self.set(
            o,
            id,
            Variant::array(mem::UINT32, v.iter().map(|&x| Elem::U32(x)).collect()),
        );
    }
    fn occupant_position(&self, o: u64) -> [f32; 3] {
        self.pos.borrow().get(&o).copied().unwrap_or_default()
    }
    fn is_prop_occupant(&self, o: u64) -> bool {
        self.prop_occ.borrow().contains(&o)
    }
    fn lot_road_access(&self, o: u64) -> Option<bool> {
        self.road.borrow().get(&o).copied()
    }
    fn sim_day(&self) -> u32 {
        self.day.get()
    }
    fn lot_location(&self, u: u64) -> Option<(i32, i32)> {
        self.lots.borrow().get(&u).copied()
    }
    fn cell_center_to_position(&self, x: i32, z: i32) -> (f32, f32) {
        (x as f32 * 16.0 + 8.0, z as f32 * 16.0 + 8.0)
    }
    fn zone_value(&self, x: i32, z: i32) -> u32 {
        u32::from(self.zoned.borrow().contains(&(x, z)))
    }
    fn sea_level(&self) -> f32 {
        250.0
    }
    fn altitude(&self, x: f32, _z: f32) -> f32 {
        if x < 0.0 {
            200.0
        } else {
            300.0
        }
    }
    fn disaster_ongoing(&self) -> bool {
        false
    }
    fn dispatch_count(&self, _t: u32) -> u32 {
        self.dispatches.borrow().len() as u32
    }
    fn dispatch_ids(&self, _t: u32) -> Vec<u32> {
        self.dispatches.borrow().iter().map(|d| d.0).collect()
    }
    fn dispatch_info(&self, id: u32) -> Option<DispatchInfo> {
        self.dispatches.borrow().iter().find(|d| d.0 == id).map(|d| d.1)
    }
    fn add_controllers(&self, group: &str, o: u64) {
        self.push(format!("add_controllers {group} {o}"));
    }
    fn remove_controllers(&self, group: &str, o: u64) {
        self.push(format!("remove_controllers {group} {o}"));
    }
    fn police_percent_coverage(&self) -> Option<f32> {
        self.police_pct.get()
    }
    fn record_history(&self, id: u32, v: i32) {
        self.push(format!("history {id:#x} {v}"));
    }
    fn post(&self, t: u32, subject: Option<u64>) {
        self.push(format!("post {t:#x} {subject:?}"));
    }
    fn create_air_source(&self, kind: ProtectionKind) -> Option<ObjRef> {
        self.add_ref(0xA1);
        Some(ObjRef {
            token: 0xA1,
            class_id: kind.params().air_source_clsid,
        })
    }
    fn air_source_init(&self, s: u64) {
        self.push(format!("air_init {s:#x}"));
    }
    fn air_source_shutdown(&self, s: u64) {
        self.push(format!("air_shutdown {s:#x}"));
    }
}

struct Script(VecDeque<f64>);

impl ProtectionRandom for Script {
    fn uniform_f64(&mut self) -> f64 {
        self.0.pop_front().expect("scripted draw")
    }
    fn range_f64(&mut self, _a: f64, _b: f64) -> f64 {
        self.0.pop_front().expect("scripted draw")
    }
}

fn setup(kind: ProtectionKind, cells: i32, draws: &[f64]) -> (Rc<Fake>, Rc<ProtectionSim>) {
    let host = Fake::new(cells);
    let sim = ProtectionSim::new(
        host.clone(),
        kind,
        0x51,
        Box::new(Script(draws.iter().copied().collect())),
    );
    (host, sim)
}

fn shipped(host: &Fake, kind: ProtectionKind) {
    *host.tuning.borrow_mut() = Some(Rc::new(shipped_tuning(kind)));
}

fn obj(token: u64) -> ObjRef {
    ObjRef { token, class_id: 0xB0 }
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() <= 1e-6 * b.abs().max(1.0)
}

#[test]
fn lifecycle_fire_and_police() {
    let (host, sim) = setup(ProtectionKind::Fire, 64, &[]);
    assert!(sim.init());
    let log = host.take();
    assert_eq!(
        log,
        [
            "acquire Fire",
            "create_grid u8 0x49d5b8f9 2",
            "create_grid i16 0x89d5b91a 2",
            "add_agent cSC4ProtectionSimulator [4, 6] 2",
            "subscribe 9",
            "register_dispatch",
            "department 0x28f55a9f 0x4a357b40 0xaa4935fe 1.2",
            "instance u8 0x49d5b8f9",
            "instance i16 0x89d5b91a",
            "air_init 0xa1",
        ]
    );
    assert_eq!(sim.data().tracts_per_day, 20);
    assert_eq!(sim.data().purpose, 0xEA56_7BC3);
    assert!(sim.init());
    assert!(host.take().is_empty(), "second Init does nothing");
    assert_eq!(sim.add_ref(), 1);
    assert_eq!(sim.add_ref(), 2);
    assert_eq!(sim.release(), 1);
    assert_eq!(sim.release(), 0);
    assert!(sim.data().destroyed);
    let log = host.take();
    assert_eq!(
        log,
        [
            "air_shutdown 0xa1",
            "unregister_dispatch",
            "unsubscribe 9",
            "remove_agent",
            "shutdown u8",
            "shutdown i16",
            "release_services Fire",
        ]
    );
    assert!(host.refs.borrow().values().all(|&r| r == 0));

    let (host, sim) = setup(ProtectionKind::Police, 256, &[]);
    shipped(&host, ProtectionKind::Police);
    sim.init();
    let log = host.take();
    assert!(log.contains(&"department 0xa2963983 0x4a357b40 0xaa493855 1.1999999".to_string()));
    assert!(log.contains(&"department 0xa2963984 0x4a357b40 0x4a4fc955 1.1999999".to_string()));
    assert_eq!(log.iter().filter(|l| l.starts_with("subscribe")).count(), 2);
    assert_eq!(sim.data().tracts_per_day, 409);
    sim.shutdown();
    assert!(host.refs.borrow().values().all(|&r| r == 0));
}

#[test]
fn cadence() {
    // §18 test 18.
    for (kind, cells, n) in [
        (ProtectionKind::Fire, 256, 327),
        (ProtectionKind::Police, 256, 409),
        (ProtectionKind::Fire, 64, 20),
        (ProtectionKind::Police, 64, 25),
        (ProtectionKind::Fire, 128, 81),
        (ProtectionKind::Police, 128, 102),
    ] {
        let (_h, sim) = setup(kind, cells, &[]);
        sim.init();
        assert_eq!(sim.data().tracts_per_day, n, "{kind:?} {cells}");
    }
}

#[test]
fn tuning_shipped_and_defaults() {
    let fire = ProtectionTuning::read(ProtectionKind::Fire, Some(&shipped_tuning(ProtectionKind::Fire)));
    // §6.7: 120 × 0.01f = 1.1999999 (0x3F999999), not 1.2f.
    assert_eq!(fire.max_funding.to_bits(), 0x3F99_9999);
    assert_eq!(fire.strike_efficiency, 0.2);
    assert_eq!(fire.min_funding_to_end.to_bits(), 0x3F66_6666);
    assert_eq!(fire.max_strike_duration, 548);
    // §18 test 1 (bit-exact under 100.0f and 0.01f).
    for (f, e) in [
        (0.5f32, 0x3E80_0000u32),
        (0.0, 0x3DCC_CCCC),
        (1.1, 0x3F86_6666),
        (1.2, 0x3F8C_CCCD),
        (fire.max_funding, 0x3F8C_CCCD),
    ] {
        let got = curve_pct(&fire.funding_curve, f);
        assert_eq!(got.to_bits(), e, "{f} → {got}");
    }
    let police = ProtectionTuning::read(ProtectionKind::Police, Some(&shipped_tuning(ProtectionKind::Police)));
    assert_eq!(police.escapee_range, (algo::pct(10.0), 0.5));
    assert_eq!(police.rehab_rate.to_bits(), 0x3DCC_CCCC);
    assert_eq!(police.strike_efficiency.to_bits(), 0x3E4C_CCCC);
    assert_eq!(police.end_jailbreak_capacity.to_bits(), 0x3F4C_CCCC);
    assert_eq!(police.arrest_threshold, 20);
    assert_eq!(police.overflow_factor, 1.1);
    // §18 test 3, test 4 and test 8.
    assert!(close(curve_pct(&police.jail_curve, 0.55), 0.75));
    assert!(close(curve_pct(&police.jail_curve, 0.05), 0.25));
    assert_eq!(curve_pct(&police.strike_curve, 0.2).to_bits(), 0x3ECC_CCCC);
    assert!(close(police.strike_curve.eval(50.0), 10.0));
    assert_eq!(police.strike_curve.eval(70.0), 0.0);
    assert_eq!(police.strike_curve.eval(100.0), 0.0);
    assert_eq!(curve_pct(&police.jailbreak_curve, 0.95).to_bits(), 0x3F0C_CCCD);
    assert_eq!(curve_pct(&police.jailbreak_curve, 0.85).to_bits(), 0x3D4C_CCCC);
    assert_eq!(curve_pct(&police.jailbreak_curve, 0.80), 0.0);
    // Defaults (quirk 11).
    let d = ProtectionTuning::read(ProtectionKind::Police, None);
    assert_eq!(d.escapee_range, (1.0, 5.0));
    assert_eq!(d.max_strike_duration, 10);
    assert!(close(d.strike_curve.eval(0.0), 20.0));
    // An empty array replaces the curve with an empty one; a scalar is ignored.
    let mut h = PropertyHolder::new();
    h.add(prop::FIRE_FUNDING_CURVE, Variant::array(mem::FLOAT32, vec![]));
    h.add(prop::FIRE_STRIKE_CURVE, Variant::Scalar(Elem::F32(3.0)));
    let t = ProtectionTuning::read(ProtectionKind::Fire, Some(&h));
    assert!(t.funding_curve.is_empty());
    assert!(!t.strike_curve.is_empty());
}

fn curve_pct(c: &crate::curve::ResponseCurve, x: f32) -> f32 {
    algo::pct(c.eval(algo::times_hundred(x)))
}

#[test]
fn station_efficiency_modifiers() {
    // §18 test 2.
    let (host, sim) = setup(ProtectionKind::Fire, 64, &[]);
    shipped(&host, ProtectionKind::Fire);
    sim.init();
    let purpose = ProtectionKind::Fire.params().purpose;
    host.item(7, purpose, 1.0, true);
    host.road.borrow_mut().insert(7, false);
    host.set(7, prop::STRIKE_START, Variant::Scalar(Elem::U32(5)));
    assert!(close(sim.station_efficiency(7), 0.16));
    host.road.borrow_mut().insert(7, true);
    assert!(close(sim.station_efficiency(7), 0.2));
    host.road.borrow_mut().remove(&7);
    assert!(close(sim.station_efficiency(7), 0.2), "no lot: no road factor");

    let (host, sim) = setup(ProtectionKind::Police, 64, &[]);
    shipped(&host, ProtectionKind::Police);
    sim.init();
    let purpose = ProtectionKind::Police.params().purpose;
    host.item(7, purpose, 1.0, true);
    host.road.borrow_mut().insert(7, false);
    host.set(7, prop::STRIKE_START, Variant::Scalar(Elem::U32(5)));
    assert!(close(sim.station_efficiency(7), 0.2));
    // §6.1: no item → 0; not locally funded → the department's funding.
    assert_eq!(sim.funding_of(8, purpose), 0.0);
    host.dept_funding.set(0.75);
    host.item(8, purpose, 0.1, false);
    assert_eq!(sim.funding_of(8, purpose), 0.75);
}

#[test]
fn station_effects_and_coverage() {
    let (host, sim) = setup(ProtectionKind::Police, 64, &[]);
    sim.init();
    let purpose = ProtectionKind::Police.params().purpose;
    // §18 test 14 at efficiency 1 (default curve at funding 1.0 → 1.0).
    host.station(9, ProtectionKind::Police, 100, 20, 400.0, [100.0, 0.0, 200.0], 2);
    host.item(9, purpose, 1.0, true);
    let (ok, e) = sim.station_effect(9);
    assert!(ok);
    assert_eq!((e.x, e.z, e.centre, e.outer, e.radius), (100.0, 200.0, 100, 20, 400.0));
    let mut out = CoverageOutputs {
        cover_radius: -1.0,
        station_radius: -1.0,
    };
    assert!(sim.occupant_coverage(9, 0.5, &mut out));
    assert_eq!((out.cover_radius, out.station_radius), (250.0, 400.0));
    // Funding 0.5 → efficiency 0.5: centre 50, outer 10, radius 200.
    host.item(9, purpose, 0.5, true);
    assert!(sim.occupant_coverage(9, 0.5, &mut out));
    assert_eq!((out.cover_radius, out.station_radius), (0.0, 200.0));
    // Police centre 0 fails and leaves the outputs alone (§7.3 quirk).
    host.set(
        9,
        ProtectionKind::Police.params().effect_props[0],
        Variant::Scalar(Elem::I32(0)),
    );
    let (ok, e) = sim.station_effect(9);
    assert!(!ok);
    assert_eq!(e, Effect::default(), "a zeroed record stays zeroed");
    // §18 test 27: the caller's outer, radius and position survive.
    let mut rec = Effect {
        x: 1.0,
        z: 2.0,
        radius: 250.0,
        centre: 7,
        outer: 3,
        original_centre: 7,
    };
    assert!(!sim.station_effect_into(9, &mut rec));
    assert_eq!(
        rec,
        Effect {
            centre: 0,
            ..Effect {
                x: 1.0,
                z: 2.0,
                radius: 250.0,
                centre: 7,
                outer: 3,
                original_centre: 7,
            }
        }
    );
    assert!(!sim.occupant_coverage(9, 0.5, &mut out));
    assert_eq!(out.station_radius, 200.0);

    // Fire zeroes the outputs; centre 0 is accepted.
    let (host, sim) = setup(ProtectionKind::Fire, 64, &[]);
    sim.init();
    host.station(9, ProtectionKind::Fire, 0, 0, 100.0, [0.0; 3], 1);
    let mut out = CoverageOutputs {
        cover_radius: 5.0,
        station_radius: 5.0,
    };
    // No item → funding 0 → default curve 0 → effect scaled to radius 0.
    assert!(sim.occupant_coverage(9, 0.5, &mut out));
    host.props
        .borrow_mut()
        .remove(&(9, ProtectionKind::Fire.params().effect_props[2]));
    let mut out = CoverageOutputs {
        cover_radius: 5.0,
        station_radius: 5.0,
    };
    assert!(!sim.occupant_coverage(9, 0.5, &mut out));
    assert_eq!(out, CoverageOutputs::default());
    // A prop station uses the department funding only (quirk 4).
    host.station(10, ProtectionKind::Fire, 100, 21, 300.0, [0.0; 3], 1);
    host.prop_occ.borrow_mut().push(10);
    host.dept_funding.set(0.75);
    host.set(10, prop::STRIKE_START, Variant::Scalar(Elem::U32(3)));
    let (ok, e) = sim.station_effect(10);
    assert!(ok);
    // §18 test 15 (default curve: 75 → 75 → 0.75).
    assert_eq!((e.centre, e.outer, e.radius, e.original_centre), (75, 15, 225.0, 100));
}

#[test]
fn station_effect_partial_reads() {
    // §18 test 27 (PR2).
    let (host, sim) = setup(ProtectionKind::Police, 64, &[]);
    sim.init();
    let pp = ProtectionKind::Police.params();
    host.station(5, ProtectionKind::Police, 100, 20, 1.0, [10.0, 0.0, 20.0], 1);
    host.item(5, pp.purpose, 1.0, true);
    host.props.borrow_mut().remove(&(5, pp.effect_props[2]));
    let mut rec = Effect {
        radius: 250.0,
        ..Effect::default()
    };
    assert!(sim.station_effect_into(5, &mut rec), "no radius: still succeeds");
    assert_eq!(
        (rec.radius, rec.centre, rec.outer, rec.x, rec.z),
        (250.0, 100, 20, 10.0, 20.0)
    );
    // FreeCity4's own callers use a zeroed record: radius 0 ([choice]).
    assert_eq!(sim.station_effect(5).1.radius, 0.0);

    let (host, sim) = setup(ProtectionKind::Fire, 64, &[]);
    sim.init();
    let fp = ProtectionKind::Fire.params();
    host.station(6, ProtectionKind::Fire, 100, 20, 300.0, [40.0, 0.0, 50.0], 1);
    host.props.borrow_mut().remove(&(6, fp.effect_props[0]));
    let mut rec = Effect {
        centre: 9,
        outer: 9,
        radius: 9.0,
        original_centre: 9,
        ..Effect::default()
    };
    assert!(!sim.station_effect_into(6, &mut rec));
    assert_eq!(
        rec,
        Effect {
            x: 40.0,
            z: 50.0,
            radius: 300.0,
            centre: 0,
            outer: 20,
            original_centre: 0,
        },
        "raw, unscaled, position written"
    );
    // §5.1 [choice]: a station whose effect fails draws nothing.
    sim.data_mut().stations.push(obj(6));
    sim.update_unadjusted();
    let g = sim.unadjusted_grid().unwrap();
    assert!((0..32).all(|x| (0..32).all(|z| g.get(x, z) == 0)));
}

#[test]
fn max_possible_dispatch_count_carries_the_count() {
    // §18 test 26 (PR1).
    let (host, sim) = setup(ProtectionKind::Fire, 64, &[]);
    sim.init();
    let fp = ProtectionKind::Fire.params();
    for (o, n) in [(1u64, 3u32), (2, 0), (3, 2)] {
        host.station(o, ProtectionKind::Fire, 10, 0, 50.0, [0.0; 3], n);
    }
    host.props.borrow_mut().remove(&(2, fp.unit_count_prop));
    sim.data_mut().stations = vec![obj(1), obj(2), obj(3)];
    assert_eq!(sim.max_possible_dispatch_count(), 8);
    assert_eq!(sim.total_dispatch_count(), 5);
    sim.data_mut().stations = vec![obj(2), obj(1), obj(3)];
    assert_eq!(sim.max_possible_dispatch_count(), 5);
    sim.data_mut().stations.clear();
    assert_eq!(sim.max_possible_dispatch_count(), 0);
}

#[test]
fn station_statistics_lengths() {
    // §18 test 29 (PR12).
    let (host, sim) = setup(ProtectionKind::Police, 64, &[]);
    shipped(&host, ProtectionKind::Police);
    sim.init();
    host.station(7, ProtectionKind::Police, 100, 20, 64.0, [8.0, 0.0, 8.0], 1);
    sim.data_mut().stations.push(obj(7));
    host.lots.borrow_mut().insert(0x77, (0, 0));
    let stats = |v: &[u32]| Variant::array(mem::UINT32, v.iter().map(|&x| Elem::U32(x)).collect());
    sim.crime_event(Some(0x77));
    assert_eq!(host.get_u32s(7, prop::STATION_STATS), [0, 0, 1, 0]);
    host.set(7, prop::STATION_STATS, stats(&[5, 6, 7, 8, 9]));
    sim.coverage_grid().unwrap().set(0, 0, 200);
    sim.crime_event(Some(0x77));
    assert_eq!(host.get_u32s(7, prop::STATION_STATS), [5, 6, 8, 9, 9]);
    sim.process_monthly_crime_stats();
    assert_eq!(host.get_u32s(7, prop::STATION_STATS), [8, 9, 0, 0, 9]);
    host.set(7, prop::STATION_STATS, stats(&[1, 2]));
    sim.coverage_grid().unwrap().set(0, 0, 0);
    sim.crime_event(Some(0x77));
    assert_eq!(host.get_u32s(7, prop::STATION_STATS), [1, 2, 1, 0]);
    host.set(7, prop::STATION_STATS, stats(&[]));
    assert_eq!((sim.station_crime_count(7), sim.station_arrest_count(7)), (0, 0));
}

#[test]
fn strike_state_and_chance() {
    // §18 test 6 (fire shipped: max duration 548, min to end 0.9).
    let (host, sim) = setup(ProtectionKind::Fire, 64, &[]);
    shipped(&host, ProtectionKind::Fire);
    sim.init();
    host.take();
    let purpose = ProtectionKind::Fire.params().purpose;
    host.item(3, purpose, 0.2, true);
    host.set(3, prop::STRIKE_START, Variant::Scalar(Elem::U32(1000)));
    host.day.set(1549);
    assert!(sim.update_striking_state(3));
    assert_eq!(host.get_u32(3, prop::STRIKE_START), Some(0));
    assert_eq!(host.take(), ["remove_controllers striking_fire 3"]);
    host.set(3, prop::STRIKE_START, Variant::Scalar(Elem::U32(1000)));
    host.day.set(1548);
    host.item(3, purpose, 0.89, true);
    assert!(!sim.update_striking_state(3));
    host.item(3, purpose, 0.9, true);
    assert!(sim.update_striking_state(3));

    // §18 test 5 (police shipped strike curve).
    let (host, sim) = setup(ProtectionKind::Police, 64, &[]);
    shipped(&host, ProtectionKind::Police);
    sim.init();
    assert_eq!(sim.chance_of_strike(), 0.0);
    let purpose = ProtectionKind::Police.params().purpose;
    for (o, f) in [(1u64, 0.5f32), (2, 0.2)] {
        host.station(o, ProtectionKind::Police, 50, 10, 100.0, [0.0; 3], 1);
        host.types.borrow_mut().insert(o, BUILDING);
        host.item(o, purpose, f, true);
        sim.occupant_inserted(obj(o));
    }
    assert!(close(sim.chance_of_strike(), 0.46));
    sim.data_mut().strike_flag = true;
    assert_eq!(sim.chance_of_strike(), 1.0);
}

#[test]
fn monthly_strike_pass() {
    // §18 test 7: A already striking, B starts this month.
    let (host, sim) = setup(ProtectionKind::Police, 64, &[0.3]);
    shipped(&host, ProtectionKind::Police);
    sim.init();
    let purpose = ProtectionKind::Police.params().purpose;
    for o in [1u64, 2] {
        host.station(o, ProtectionKind::Police, 50, 10, 64.0, [100.0, 0.0, 100.0], 1);
        host.item(o, purpose, 0.2, true);
        sim.occupant_inserted(obj(o));
    }
    host.set(1, prop::STRIKE_START, Variant::Scalar(Elem::U32(90)));
    host.day.set(100);
    let adjusted_before = sim.coverage_grid().unwrap().get(3, 3);
    host.take();
    // Police draw 0.3 < 0.40 → B starts.
    sim.do_monthly();
    let log = host.take();
    assert!(log.contains(&"add_controllers striking_police 2".to_string()));
    assert_eq!(
        log.iter().filter(|l| l.starts_with("post")).collect::<Vec<_>>(),
        ["post 0xa8daf511 Some(2)"]
    );
    assert!(sim.is_on_strike());
    assert_eq!(sim.strike_start_date(), 100);
    assert_eq!(sim.strike_length(), 0);
    // Unadjusted rebuilt with both stations at strike efficiency; adjusted unchanged.
    // Each cone: e = 20 × 0.01f = 0.19999999, centre trunc(9.9999995) = 9
    // before; trunc(50 × f32(0.19999999²)) = trunc(1.9999998) = 1 after.
    assert_eq!(adjusted_before, 18);
    assert_eq!(sim.coverage_grid().unwrap().get(3, 3), 18);
    assert_eq!(sim.unadjusted_grid().unwrap().get(3, 3), 2);
    let mut out = vec![obj(99)];
    assert_eq!(sim.striking_stations(&mut out), 2);
    assert_eq!(out.len(), 3, "appends");
    assert!(sim.end_strike());
    assert!(!sim.end_strike());
    assert_eq!(sim.strike_length(), u32::MAX);
    assert_eq!(sim.striking_stations(&mut out), 0, "police: none without the city flag");
}

#[test]
fn call_numbers_and_dispatch() {
    // §18 test 17.
    let (host, sim) = setup(ProtectionKind::Fire, 64, &[]);
    sim.init();
    let p = ProtectionKind::Fire.params();
    host.station(1, ProtectionKind::Fire, 10, 0, 50.0, [0.0; 3], 2);
    host.station(2, ProtectionKind::Fire, 10, 0, 50.0, [500.0, 0.0, 0.0], 3);
    host.station(3, ProtectionKind::Fire, 10, 0, 50.0, [900.0, 0.0, 0.0], 2);
    sim.occupant_inserted(obj(1));
    sim.occupant_inserted(obj(2));
    assert_eq!(host.get_u32(1, p.base_prop), Some(1));
    assert_eq!(host.get_u32(2, p.base_prop), Some(3));
    assert_eq!(host.get_u32(2, p.mask_prop), Some(0b111));
    assert_eq!(sim.data().next_call, 6);
    sim.occupant_removed(obj(1));
    sim.occupant_inserted(obj(3));
    assert_eq!(host.get_u32(3, p.base_prop), Some(1));
    assert_eq!(sim.total_dispatch_count(), 5);

    // §18 test 16 through the simulator.
    host.set_occupant_u32(2, p.base_prop, 5);
    assert_eq!(sim.dispatch_from_station(2, 0), Some(5));
    assert_eq!(sim.dispatch_from_station(2, 0), Some(6));
    assert_eq!(host.get_u32(2, p.mask_prop), Some(0b100));
    assert!(sim.revoke_dispatch_from_station(2, 5));
    assert!(!sim.revoke_dispatch_from_station(2, 5));
    host.set_occupant_u32(2, p.mask_prop, 0);
    assert_eq!(sim.dispatch_from_station(2, 0), Some(8));
    // Nearest: station 2 has no free unit, station 3 is the only candidate.
    host.road.borrow_mut().insert(3, true);
    assert_eq!(sim.dispatch_from_nearest_station(450.0, 0.0), Some((obj(3), 1)));
    host.road.borrow_mut().insert(3, false);
    assert_eq!(sim.dispatch_from_nearest_station(450.0, 0.0), None);
    // CanDispatch: sea level 250 must be below the altitude.
    assert_eq!(sim.can_dispatch(10.0, 0.0), Some(false));
    assert_eq!(sim.can_dispatch(-10.0, 0.0), None);
    assert_eq!(sim.dispatch_automata_properties(), (0x4202, [0.0; 3], 0x11882));
    // Dispatch end returns the unit.
    host.set_occupant_u32(3, p.mask_prop, 0b10);
    host.dispatches.borrow_mut().push((
        1,
        DispatchInfo {
            station: 3,
            call_id: 1,
            active: false,
            x: 0.0,
            z: 0.0,
        },
    ));
    let mut m = ProtectionMessage::of_type(msg::DISPATCH_END);
    m.data1 = 1;
    m.data2 = 1;
    assert!(sim.handle_message(m));
    assert_eq!(host.get_u32(3, p.mask_prop), Some(0b11));
}

/// §5.1 / §7.2 (PR2, PR14): the dispatch cone comes from the source station's
/// dispatch effect (the record's +0x20), scaled by its efficiency.
#[test]
fn dispatch_cone_and_box_copy() {
    let (host, sim) = setup(ProtectionKind::Fire, 64, &[]);
    shipped(&host, ProtectionKind::Fire);
    sim.init();
    let p = ProtectionKind::Fire.params();
    // Full local funding: efficiency 1 with the shipped curve (PR14 scales
    // the dispatch effect; a station without an item has funding 0).
    host.item(77, p.purpose, 1.0, true);
    host.set(77, p.dispatch_effect_props[0], Variant::Scalar(Elem::I32(40)));
    host.set(77, p.dispatch_effect_props[1], Variant::Scalar(Elem::I32(40)));
    host.set(77, p.dispatch_effect_props[2], Variant::Scalar(Elem::F32(64.0)));
    host.dispatches.borrow_mut().push((
        4,
        DispatchInfo {
            station: 77,
            call_id: 4,
            active: true,
            x: 330.0,
            z: 330.0,
        },
    ));
    let mut m = ProtectionMessage::of_type(msg::DISPATCH_ACTIVE);
    m.data1 = 1;
    m.data2 = 4;
    sim.handle_message(m);
    // Tract (10, 10), radius 2 tracts, flat value 40 inside d < 2.
    let adj = sim.coverage_grid().unwrap();
    assert_eq!(adj.get(10, 10), 40);
    assert_eq!(adj.get(11, 10), 40);
    assert_eq!(adj.get(12, 10), 0);
    // Wrong dispatch type: nothing.
    adj.set(10, 10, 0);
    m.data1 = 0;
    sim.handle_message(m);
    assert_eq!(adj.get(10, 10), 0);
    // UpdateUnadjusted reads the station's effect too: the vehicle-like
    // occupant 78 carries other values, which are never used.
    host.set(78, p.dispatch_effect_props[0], Variant::Scalar(Elem::I32(99)));
    host.set(78, p.dispatch_effect_props[1], Variant::Scalar(Elem::I32(99)));
    host.set(78, p.dispatch_effect_props[2], Variant::Scalar(Elem::F32(160.0)));
    sim.update_unadjusted();
    let unadj = sim.unadjusted_grid().unwrap();
    assert_eq!(unadj.get(10, 10), 40);
    assert_eq!(unadj.get(12, 10), 0);
    // A station without the properties draws a zero-radius cone of 0.
    host.dispatches.borrow_mut()[0].1.station = 78 + 1;
    sim.update_unadjusted();
    assert_eq!(unadj.get(10, 10), 0);
}

/// §7.2 corrected (PR14): `GetDispatchEffect` fails only on an absent or
/// zero centre, keeps the caller's outer and radius when absent, and scales
/// by `GetStationEfficiency`; the active / inactive handler skips the update
/// when it fails ([choice]).
#[test]
fn dispatch_effect_is_scaled_and_fails_on_the_centre_only() {
    let (host, sim) = setup(ProtectionKind::Police, 64, &[]);
    shipped(&host, ProtectionKind::Police);
    sim.init();
    let p = ProtectionKind::Police.params();
    host.station(5, ProtectionKind::Police, 10, 0, 50.0, [0.0, 0.0, 0.0], 1);
    // The spec's example: centre 100, outer 20, radius 300 at efficiency 0.5
    // (shipped police curve: local funding 0.5 -> 50 -> 0.5).
    host.item(5, p.purpose, 0.5, true);
    host.set(5, p.dispatch_effect_props[0], Variant::Scalar(Elem::I32(100)));
    host.set(5, p.dispatch_effect_props[1], Variant::Scalar(Elem::I32(20)));
    host.set(5, p.dispatch_effect_props[2], Variant::Scalar(Elem::F32(300.0)));
    assert!(close(sim.station_efficiency(5), 0.5));
    let (ok, e) = sim.dispatch_effect(5);
    assert!(ok);
    assert_eq!((e.centre, e.outer, e.original_centre), (50, 10, 100));
    assert!(close(e.radius, 150.0));
    assert!(close(sim.station_dispatch_radius(5), 150.0));
    // An absent outer and radius keep the caller's record; still AL = 1.
    host.unset(5, p.dispatch_effect_props[1]);
    host.unset(5, p.dispatch_effect_props[2]);
    let mut rec = Effect {
        x: 7.0,
        z: 8.0,
        outer: 6,
        radius: 64.0,
        original_centre: 3,
        ..Effect::default()
    };
    assert!(sim.dispatch_effect_into(5, &mut rec));
    assert_eq!((rec.x, rec.z), (0.0, 0.0));
    assert_eq!((rec.centre, rec.outer, rec.original_centre), (50, 3, 100));
    assert!(close(rec.radius, 32.0));
    // A zero centre fails: x, z and the centre are 0, the rest is the caller's.
    host.set(5, p.dispatch_effect_props[0], Variant::Scalar(Elem::I32(0)));
    let mut rec = Effect {
        x: 7.0,
        z: 8.0,
        centre: 9,
        outer: 6,
        radius: 64.0,
        original_centre: 3,
    };
    assert!(!sim.dispatch_effect_into(5, &mut rec));
    assert_eq!((rec.x, rec.z, rec.centre), (0.0, 0.0, 0));
    assert_eq!((rec.outer, rec.radius, rec.original_centre), (6, 64.0, 3));
    assert_eq!(sim.station_dispatch_radius(5), 0.0);
    // The handler skips the box update when the effect fails: a mark in the
    // adjusted grid stays.
    host.dispatches.borrow_mut().push((
        4,
        DispatchInfo {
            station: 5,
            call_id: 4,
            active: true,
            x: 330.0,
            z: 330.0,
        },
    ));
    let adj = sim.coverage_grid().unwrap();
    adj.set(10, 10, 77);
    let mut m = ProtectionMessage::of_type(msg::DISPATCH_ACTIVE);
    m.data1 = p.dispatch_type;
    m.data2 = 4;
    sim.handle_message(m);
    assert_eq!(adj.get(10, 10), 77);
}

#[test]
fn daily_cycle_and_end_of_cycle() {
    // §18 test 19 and quirk 12: 64-cell city, fire 20 tracts per day.
    let (host, sim) = setup(ProtectionKind::Fire, 64, &[]);
    sim.init();
    host.police_pct.set(Some(0.5));
    let unadj = sim.unadjusted_grid().unwrap();
    for (i, v) in [0i16, 10, 20, 50].into_iter().enumerate() {
        let t = i as i32 * 3;
        unadj.set(t, 1, v);
        host.zoned.borrow_mut().push((t * 2 + 1, 3));
    }
    unadj.set(20, 20, 99); // unzoned
    host.take();
    sim.handle_message(ProtectionMessage::of_type(msg::SIM_NEW_DAY));
    let log = host.take();
    assert_eq!(
        log,
        [
            "history 0x2a11d203 0",
            "history 0x2a11d204 0",
            "history 0xaaad4d87 25",
            "post 0x68f54ce0 None"
        ]
    );
    assert_eq!(
        sim.crt().state(),
        CrtRand::new(1).state().wrapping_mul(214_013).wrapping_add(2_531_011)
    );
    for _ in 2..=51 {
        sim.do_daily();
    }
    assert!(host.take().is_empty());
    sim.do_daily();
    assert_eq!(
        host.take(),
        [
            "history 0x2a11d203 75",
            "history 0x2a11d204 20",
            "history 0xaaad4d87 62",
            "post 0x68f54ce0 None"
        ]
    );
    assert_eq!(sim.average_coverage(), 20.0);
    assert_eq!(sim.percent_coverage(), 0.75);
    assert_eq!(sim.coverage_grid().unwrap().get(20, 20), 99);
    assert_eq!(sim.coverage_grid().unwrap().get(9, 1), 50);
}

#[test]
fn adjusted_copy_clamps() {
    // §18 test 20 through the box copy.
    let (host, sim) = setup(ProtectionKind::Police, 64, &[]);
    sim.init();
    host.station(5, ProtectionKind::Police, 300, 300, 40.0, [16.0, 0.0, 16.0], 0);
    host.item(5, ProtectionKind::Police.params().purpose, 1.0, true);
    sim.occupant_inserted(obj(5));
    let adj = sim.coverage_grid().unwrap();
    let unadj = sim.unadjusted_grid().unwrap();
    assert_eq!(unadj.get(0, 0), 300);
    assert_eq!(adj.get(0, 0), 255);
    unadj.set(1, 1, -5);
    unadj.set(1, 0, 128);
    sim.handle_message(ProtectionMessage::of_type(msg::SIM_NEW_DAY));
    for _ in 0..60 {
        sim.do_daily();
    }
    // The next station change rebuilds the unadjusted grid; the daily copy clamps.
    let mut seen = [adj.get(1, 0), 0];
    seen[1] = algo::adjusted_value(-5, 1.0);
    assert_eq!(seen, [128, 0]);
}

#[test]
fn jails_recalculate_rehab_and_reset() {
    let (host, sim) = setup(ProtectionKind::Police, 64, &[]);
    shipped(&host, ProtectionKind::Police);
    sim.init();
    for (o, cap) in [(21u64, 400u32), (22, 600)] {
        host.set(o, prop::JAIL_CAPACITY, Variant::Scalar(Elem::U32(cap)));
        host.item(o, JAIL_PURPOSE, 1.0, true);
        sim.occupant_inserted(obj(o));
    }
    assert_eq!(sim.jail_count(), 2);
    assert_eq!(sim.data().jail.funded_jails.len(), 2);
    // §18 test 11.
    sim.data_mut().jail.jailed = 600;
    sim.recalculate();
    assert_eq!(sim.jail_capacity(), 1000);
    assert_eq!(host.get_u32(21, prop::JAIL_POPULATION), Some(240));
    assert_eq!(host.get_u32(22, prop::JAIL_POPULATION), Some(360));
    assert_eq!(sim.jail_population(), 600);
    sim.data_mut().jail.jailed = 1500;
    sim.recalculate();
    assert_eq!(sim.jail_population(), 1000);
    assert_eq!(sim.jailed_percentage(), 1.0);
    // §18 test 10.
    {
        let mut d = sim.data_mut();
        d.jail.jailed = 1200;
        d.jail.unarrested_this = 7;
    }
    assert_eq!(sim.reset_unjailed_criminals(), 107);
    assert_eq!(sim.criminal_count(), 1100);
    assert_eq!(sim.data().jail.unarrested_this, 0);
    // §18 test 9 through DoMonthly (no stations, jailbreak chances 0 below 0.8).
    // Shipped rate 0.099999994: 135 → 122, and 5 stays (§9.5).
    for (jailed, after) in [(600, 540), (125, 113), (135, 122), (6, 5), (5, 5)] {
        sim.data_mut().jail.jailed = jailed;
        sim.do_monthly();
        assert_eq!(sim.criminal_count(), after);
    }
    // Upkeep: trunc(1000 × 1.0).
    assert_eq!(sim.jail_upkeep_cost(21), 1000);
    assert_eq!(sim.jail_upkeep_cost(99), 0);
    // Removal (§9.4): only buildings.
    host.take();
    sim.occupant_removed(obj(21));
    assert!(host.take().is_empty(), "not a building: nothing");
    assert_eq!(sim.jail_count(), 2);
    host.types.borrow_mut().insert(21, BUILDING);
    sim.occupant_removed(obj(21));
    assert_eq!(host.take(), [format!("remove_funded 21 {JAIL_PURPOSE:#x}")]);
    assert_eq!(sim.jail_capacity(), 600);
}

#[test]
fn jailbreaks() {
    // §18 test 12 and the §9.5 jailbreak passes.
    let (host, sim) = setup(ProtectionKind::Police, 64, &[0.25, 0.01, 0.9]);
    shipped(&host, ProtectionKind::Police);
    sim.init();
    host.set(21, prop::JAIL_CAPACITY, Variant::Scalar(Elem::U32(600)));
    host.item(21, JAIL_PURPOSE, 1.0, true);
    sim.occupant_inserted(obj(21));
    sim.data_mut().jail.jailed = 400;
    host.take();
    assert!(sim.begin_jailbreak(obj(21)));
    assert_eq!(sim.criminal_count(), 250);
    assert!(sim.is_in_jailbreak(21));
    assert_eq!(
        host.take(),
        ["post 0x2a614891 Some(21)", "add_controllers jailbreak 21"]
    );
    assert_eq!(sim.chance_of_jailbreak(21), 0.0, "already breaking out");
    let mut list = vec![obj(5)];
    assert_eq!(sim.jailbreak_buildings(&mut list), 1);
    assert_eq!(list, [obj(21)]);
    // 250 / 600 < 0.8 → the month ends it; then the chance at 0.375 is 0.
    sim.data_mut().jail.jailed = 250;
    sim.data_mut().tuning.rehab_rate = 0.0;
    sim.do_monthly();
    assert!(!sim.is_in_jailbreak(21));
    let log = host.take();
    assert!(log.contains(&"post 0x2a614892 Some(21)".to_string()));
    assert!(!sim.end_jailbreak(21), "always false");
    let mut list = vec![obj(5)];
    assert_eq!(sim.jailbreak_buildings(&mut list), 0);
    assert_eq!(list, [obj(5)], "untouched when empty");
    // Full jail: chance curve(100) = 1.0; scripted draw 0.01 < 1 → begins; r = 0.9.
    sim.data_mut().jail.jailed = 700;
    sim.recalculate();
    assert!(close(sim.chance_of_jailbreak(21), 1.0));
    sim.do_monthly();
    assert!(sim.is_in_jailbreak(21));
    // escaped = rhe(600 × 0.9) = 540 → 160.
    assert_eq!(sim.criminal_count(), 160);
    sim.shutdown();
    assert!(host.refs.borrow().values().all(|&r| r == 0));
}

#[test]
fn crime_events_and_monthly_stats() {
    // §18 test 13.
    let (host, sim) = setup(ProtectionKind::Police, 64, &[]);
    shipped(&host, ProtectionKind::Police);
    sim.init();
    host.station(1, ProtectionKind::Police, 50, 10, 64.0, [8.0, 0.0, 8.0], 1);
    host.station(2, ProtectionKind::Police, 50, 10, 64.0, [800.0, 0.0, 800.0], 1);
    sim.occupant_inserted(obj(1));
    sim.occupant_inserted(obj(2));
    let adj = sim.coverage_grid().unwrap();
    adj.set(1, 1, 21);
    host.lots.borrow_mut().insert(500, (2, 2));
    let mut m = ProtectionMessage::of_type(msg::CRIME);
    m.unknown = Some(500);
    sim.handle_message(m);
    assert_eq!(sim.data().jail.arrests_this, 1);
    assert_eq!(host.get_u32s(1, prop::STATION_STATS), [0, 0, 1, 1]);
    adj.set(1, 1, 20);
    sim.handle_message(m);
    assert_eq!(sim.data().jail.unarrested_this, 1);
    assert_eq!(host.get_u32s(1, prop::STATION_STATS), [0, 0, 2, 1]);
    m.unknown = Some(501); // not a lot: ignored
    sim.handle_message(m);
    assert_eq!(sim.data().jail.crimes_this, 2);
    {
        let mut d = sim.data_mut();
        d.jail.arrests_this = 3;
        d.jail.jailed = 10;
    }
    host.take();
    assert!(sim.process_monthly_crime_stats());
    assert_eq!(sim.criminal_count(), 13);
    assert_eq!(sim.arrest_count(), 3);
    assert_eq!(host.get_u32s(1, prop::STATION_STATS), [2, 1, 0, 0]);
    assert_eq!(host.get_u32s(2, prop::STATION_STATS), [0, 0, 0, 0]);
    assert_eq!(sim.station_arrest_count(1), 1);
    assert_eq!(sim.station_crime_count(1), 2);
    let log = host.take();
    assert_eq!(&log[log.len() - 2..], ["history 0xa5cc155 2", "history 0xa5cc156 3"]);
}

#[test]
fn messages_filtering() {
    let (host, sim) = setup(ProtectionKind::Police, 64, &[]);
    sim.init();
    host.station(1, ProtectionKind::Police, 50, 10, 64.0, [8.0, 0.0, 8.0], 1);
    host.set(1, prop::PREVIEW_OCCUPANT, Variant::Scalar(Elem::Bool(true)));
    let mut m = ProtectionMessage::of_type(msg::OCCUPANT_INSERTED);
    m.occupant = Some(obj(1));
    assert!(sim.handle_message(m));
    assert_eq!(sim.station_count(), 0, "preview occupants are skipped");
    host.props.borrow_mut().remove(&(1, prop::PREVIEW_OCCUPANT));
    host.take();
    sim.handle_message(m);
    assert_eq!(sim.station_count(), 1);
    assert!(host.take().contains(&"post 0xa949569a None".to_string()));
    // Reload only for the right group and instance.
    host.take();
    shipped(&host, ProtectionKind::Police);
    let mut r = ProtectionMessage::of_type(msg::RELOAD_TUNING);
    r.data1 = TUNING_GROUP;
    r.data2 = 0x68F5_524A;
    sim.handle_message(r);
    assert_eq!(sim.tuning().arrest_threshold, 16);
    r.data2 = 0x28DF_CF89;
    sim.handle_message(r);
    assert_eq!(sim.tuning().arrest_threshold, 20);
    // Funding change of another department: no rebuild.
    let unadj = sim.unadjusted_grid().unwrap();
    unadj.set(30, 30, 7);
    let mut f = ProtectionMessage::of_type(msg::FUNDING_CHANGED);
    f.data2 = 1;
    sim.handle_message(f);
    assert_eq!(unadj.get(30, 30), 7);
    f.data2 = ProtectionKind::Police.params().department;
    sim.handle_message(f);
    assert_eq!(unadj.get(30, 30), 0);
}

#[derive(Clone)]
struct MapResolver {
    map: HashMap<(u32, u32), RefValue>,
    host: Rc<Fake>,
}

impl RefResolver for MapResolver {
    fn resolve(&mut self, class_id: u32, object_id: u32) -> Option<RefValue> {
        let r = self.map.get(&(class_id, object_id)).cloned()?;
        if let RefValue::Host { token, .. } = r {
            // The reader owns one reference (foundations §2.7).
            self.host.add_ref(token);
        }
        Some(r)
    }
}

fn save(sim: &ProtectionSim, reader_host: &Rc<Fake>) -> (Vec<u8>, MapResolver) {
    let reg = Rc::new(RefCell::new(SaveRegistry::new()));
    let mut w = ByteWriter::with_registry(reg.clone());
    assert!(sim.write(&mut w));
    let map = reg
        .borrow()
        .entries
        .iter()
        .map(|(class, id, o, token)| {
            let r = match o {
                Some(o) => RefValue::Object(o.clone()),
                None => RefValue::Host {
                    token: *token,
                    class_id: *class,
                },
            };
            ((*class, *id), r)
        })
        .collect();
    (
        w.into_bytes(),
        MapResolver {
            map,
            host: reader_host.clone(),
        },
    )
}

fn all_released(host: &Fake) -> bool {
    host.refs.borrow().values().all(|&r| r == 0)
}

#[test]
fn persistence_round_trip() {
    // §18 test 21.
    let (host, sim) = setup(ProtectionKind::Police, 64, &[]);
    shipped(&host, ProtectionKind::Police);
    sim.init();
    for o in [1u64, 2] {
        host.station(o, ProtectionKind::Police, 50, 10, 64.0, [8.0 * o as f32, 0.0, 8.0], 2);
        sim.occupant_inserted(obj(o));
    }
    host.set(21, prop::JAIL_CAPACITY, Variant::Scalar(Elem::U32(600)));
    host.item(21, JAIL_PURPOSE, 1.0, true);
    sim.occupant_inserted(obj(21));
    {
        let mut d = sim.data_mut();
        d.pct = 0.5;
        d.avg = 3.25;
        d.jail.jailed = 42;
        d.jail.crimes_last = 9;
    }
    let host2 = Fake::new(64);
    let (bytes, mut resolver) = save(&sim, &host2);
    let sim2 = ProtectionSim::new(
        host2.clone(),
        ProtectionKind::Police,
        0x52,
        Box::new(Script(VecDeque::new())),
    );
    assert!(sim2.read(&mut ByteReader::with_resolver(&bytes, &mut resolver)));
    assert!(sim2.data().loaded);
    assert_eq!(sim2.data().stations, sim.data().stations);
    assert_eq!(sim2.data().jail, sim.data().jail);
    assert_eq!(sim2.data().pct, 0.5);
    assert_eq!(sim2.data().air_source, sim.data().air_source);
    assert_eq!(sim2.data().department, sim.data().department);
    sim2.init();
    let log = host2.take();
    assert!(log
        .iter()
        .all(|l| !l.starts_with("create_grid") && !l.starts_with("department")));
    let (bytes2, _) = save(&sim2, &host2);
    assert_eq!(bytes, bytes2, "identical bytes on re-save");
    // A second read releases what the simulator held.
    assert!(sim2.read(&mut ByteReader::with_resolver(&bytes, &mut resolver)));
    sim2.shutdown();
    assert!(all_released(&host2));
}

#[test]
fn persistence_versions() {
    let (host, sim) = setup(ProtectionKind::Fire, 64, &[]);
    sim.init();
    sim.data_mut().pct = 75.0;
    let (mut bytes, mut resolver) = save(&sim, &host);
    // Fire u16 3, then the base u16 at offset 2: make it a v2 base record.
    assert_eq!(&bytes[..4], [3, 0, 3, 0]);
    bytes[2] = 2;
    let (_h, sim2) = setup(ProtectionKind::Fire, 64, &[]);
    assert!(sim2.read(&mut ByteReader::with_resolver(&bytes, &mut resolver)));
    assert_eq!(sim2.data().pct, 0.75);
    // A v4 base record is rejected after its 2 bytes; the fire record reads
    // the plane ref from there and still returns true (quirk 9).
    let mut w = ByteWriter::new();
    w.put_u16(3);
    w.put_u16(4);
    w.put_u32(0);
    let (_h, sim3) = setup(ProtectionKind::Fire, 64, &[]);
    let mut r = ByteReader::new(w.bytes());
    assert!(sim3.read(&mut r));
    assert_eq!(r.position(), 8);
    assert!(sim3.data().air_source.is_none());
    let mut w = ByteWriter::new();
    w.put_u16(4);
    let mut r = ByteReader::new(w.bytes());
    assert!(!sim3.read(&mut r));
    assert_eq!(r.position(), 2);

    // Police v7: no helicopter ref.
    let (host, sim) = setup(ProtectionKind::Police, 64, &[]);
    sim.init();
    sim.data_mut().jail.jailed = 5;
    let (mut bytes, mut resolver) = save(&sim, &host);
    bytes[0] = 7;
    // The helicopter ref is the last 8 bytes (class, id): drop it.
    bytes.truncate(bytes.len() - 8);
    let (_h, sim4) = setup(ProtectionKind::Police, 64, &[]);
    let mut r = ByteReader::with_resolver(&bytes, &mut resolver);
    assert!(sim4.read(&mut r));
    assert_eq!(r.position(), bytes.len());
    assert_eq!(sim4.data().jail.jailed, 5);
    assert!(sim4.data().air_source.is_none());
    // Police v9 is rejected; a failed base record skips the police fields
    // and the read is still true (quirk 9).
    let mut w = ByteWriter::new();
    w.put_u16(9);
    assert!(!sim4.read(&mut ByteReader::new(w.bytes())));
    let mut w = ByteWriter::new();
    w.put_u16(8);
    w.put_u16(4);
    assert!(sim4.read(&mut ByteReader::new(w.bytes())));
    assert_eq!(sim4.data().jail.jailed, 5, "unchanged");
}

/// The references a simulator holds (each list entry and each object).
fn held(s: &ProtectionSim) -> Vec<u64> {
    let d = s.data();
    let mut v: Vec<u64> = [d.department, d.jail.jail_department, d.air_source]
        .iter()
        .flatten()
        .map(|o| o.token)
        .collect();
    for list in [&d.stations, &d.jail.jails, &d.jail.funded_jails] {
        v.extend(list.iter().map(|o| o.token));
    }
    v
}

#[test]
fn hostile_inputs() {
    let (host, sim) = setup(ProtectionKind::Police, 64, &[0.5; 8]);
    sim.init();
    host.station(
        1,
        ProtectionKind::Police,
        i32::MAX,
        i32::MIN,
        f32::MAX,
        [f32::NAN, 0.0, f32::INFINITY],
        u32::MAX,
    );
    host.item(1, ProtectionKind::Police.params().purpose, f32::NAN, true);
    sim.occupant_inserted(obj(1));
    host.station(2, ProtectionKind::Police, 5, 5, 1e30, [-1e30, 0.0, 1e30], 40);
    sim.occupant_inserted(obj(2));
    host.set(3, prop::JAIL_CAPACITY, Variant::Scalar(Elem::U32(u32::MAX)));
    host.item(3, JAIL_PURPOSE, 1.0, true);
    sim.occupant_inserted(obj(3));
    sim.data_mut().jail.jailed = i32::MAX;
    sim.recalculate();
    sim.do_monthly();
    sim.do_daily();
    sim.data_mut().tracts_per_day = i32::MAX;
    sim.do_daily();
    let _ = sim.reset_unjailed_criminals();
    let _ = sim.process_monthly_crime_stats();
    let _ = sim.dispatch_from_nearest_station(f32::NAN, 0.0);
    // Every truncation of a save either loads or fails without leaking a
    // reference: what the reader's host holds equals what the simulator
    // holds.
    let reader = Fake::new(64);
    let (bytes, resolver) = save(&sim, &reader);
    for cut in 0..bytes.len() {
        let mut map = resolver.clone();
        let s2 = ProtectionSim::new(
            reader.clone(),
            ProtectionKind::Police,
            9,
            Box::new(Script(VecDeque::new())),
        );
        let _ = s2.read(&mut ByteReader::with_resolver(&bytes[..cut], &mut map));
        let tokens = held(&s2);
        let outstanding: i32 = reader.refs.borrow().values().sum();
        assert_eq!(outstanding, tokens.len() as i32, "cut {cut}");
        for t in tokens {
            reader.release(t);
        }
    }
    // A huge list count with nothing behind it.
    let mut w = ByteWriter::new();
    w.put_u16(8);
    w.put_u16(3);
    w.put_u32(0);
    let _ = SimGrid::<u8>::new();
    assert!(!sim.read(&mut ByteReader::new(w.bytes())) || sim.data().loaded);
}

/// §5.1 (PR17, PR18): `UpdateUnadjusted` keeps ONE effect record across the
/// station and dispatch loops. A dispatch whose source station has no
/// dispatch centre fails `GetDispatchEffect`, which zeroes only x, z and the
/// centre; its AL is ignored and the cone is drawn with centre 0 and the
/// previous dispatch's outer and radius: an inverted cone, 0 at the
/// dispatch's tract.
#[test]
fn pr17_a_failed_dispatch_draws_the_leftover_inverted_cone() {
    let (host, sim) = setup(ProtectionKind::Fire, 64, &[]);
    shipped(&host, ProtectionKind::Fire);
    sim.init();
    let p = ProtectionKind::Fire.params();
    host.item(77, p.purpose, 1.0, true);
    host.set(77, p.dispatch_effect_props[0], Variant::Scalar(Elem::I32(40)));
    host.set(77, p.dispatch_effect_props[1], Variant::Scalar(Elem::I32(40)));
    host.set(77, p.dispatch_effect_props[2], Variant::Scalar(Elem::F32(64.0)));
    let dispatch = |id: u32, station: u64, x: f32| {
        (
            id,
            DispatchInfo {
                station,
                call_id: id,
                active: true,
                x,
                z: 330.0,
            },
        )
    };
    // Station 79 has no dispatch properties at all.
    host.dispatches.borrow_mut().push(dispatch(1, 79, 650.0));
    sim.update_unadjusted();
    let unadj = sim.unadjusted_grid().unwrap();
    // Alone, it reads the zeroed record (the original's is uninitialised;
    // zero is the only reproducible value): radius 0, nothing added.
    assert!((15..26).all(|x| unadj.get(x, 10) == 0));
    // After a dispatch from station 77 (centre 40, outer 40, radius 64 m =
    // 2 tracts at (10, 10)), the failed one keeps outer 40 and radius 2.
    host.dispatches.borrow_mut().insert(0, dispatch(2, 77, 330.0));
    sim.update_unadjusted();
    assert_eq!(unadj.get(10, 10), 40);
    assert_eq!(unadj.get(11, 10), 40);
    // The inverted cone at tract (20, 10): 0 at the centre, rising towards
    // the outer 40 within the leftover radius of 2 tracts.
    assert_eq!(unadj.get(20, 10), 0);
    let near = unadj.get(21, 10);
    assert!(near > 0 && near < 40, "{near}");
    assert_eq!(near, unadj.get(19, 10));
    assert_eq!(near, unadj.get(20, 11));
    assert_eq!(unadj.get(22, 10), 0);
    // A zeroed record per dispatch (the old [choice]) would add nothing there.
    let (ok, e) = sim.dispatch_effect(79);
    assert!(!ok);
    assert_eq!((e.centre, e.outer, e.radius), (0, 0, 0.0));
}
