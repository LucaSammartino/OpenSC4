// SPDX-License-Identifier: GPL-3.0-or-later
//
// FreeCity4 - an open-source reimplementation of the SimCity 4 engine.
// Copyright (C) 2026 FreeCity4 contributors
// Licensed under GPL-3.0-or-later; see LICENSE at the repository root.

//! A fake city behind [`DisasterHost`] and the §18 tests (numbered as in
//! the spec), plus the behaviours around them.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::rc::{Rc, Weak};

use super::*;
use crate::property::PropertyHolder;
use crate::variant::{Elem, Variant};

#[derive(Clone, Debug, Default)]
struct Occ {
    otype: u32,
    cell: (i32, i32),
    in_manager: bool,
    lot: Option<u64>,
    flags: u32,
    props: BTreeMap<u32, f64>,
    burning: bool,
    coverage: u8,
}

#[derive(Default)]
struct Fake {
    me: RefCell<Weak<Fake>>,
    sys: RefCell<Option<Weak<Disasters>>>,
    now: Cell<u64>,
    log: RefCell<Vec<String>>,
    paused: Cell<bool>,
    emergency: Cell<i32>,
    save_disabled: Cell<i32>,
    established: Cell<bool>,
    occs: RefCell<BTreeMap<u64, Occ>>,
    next: Cell<u64>,
    flammability: RefCell<BTreeMap<(i32, i32), u8>>,
    police: RefCell<BTreeMap<(i32, i32), u8>>,
    roads: RefCell<BTreeSet<(i32, i32)>>,
    network: RefCell<BTreeMap<(i32, i32), u64>>,
    automata: RefCell<BTreeMap<u64, Option<(i32, i32)>>>,
    tuning: RefCell<BTreeMap<u32, Rc<PropertyHolder>>>,
    pipes: RefCell<BTreeMap<(i32, i32), u64>>,
    pipe_neighbours: RefCell<BTreeMap<u64, Vec<(i32, i32)>>>,
    view_level: Cell<u32>,
    refs: RefCell<BTreeMap<u64, i64>>,
    no_burn_down: Cell<bool>,
    /// Remove the occupant (and notify the layer) on demolish.
    demolish_notifies: Cell<bool>,
    follow: Cell<bool>,
    /// Fire occupant -> its target (a fire's cells and box are its target's).
    fire_targets: RefCell<BTreeMap<u64, u64>>,
    fire_dispatch: Cell<u32>,
    police_dispatch: Cell<u32>,
    wind: Cell<(f32, f32)>,
}

impl Fake {
    fn new() -> Rc<Fake> {
        let f = Rc::new(Fake::default());
        *f.me.borrow_mut() = Rc::downgrade(&f);
        f.next.set(0x1000);
        f.established.set(true);
        f.view_level.set(1);
        f.tuning
            .borrow_mut()
            .insert(tuning_instance::FLAME, Rc::new(shipped_flame()));
        f.tuning
            .borrow_mut()
            .insert(tuning_instance::RIOT, Rc::new(shipped_riot()));
        f
    }
    fn log(&self, s: impl Into<String>) {
        self.log.borrow_mut().push(s.into());
    }
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.log.borrow_mut())
    }
    fn count(&self, prefix: &str) -> usize {
        self.log.borrow().iter().filter(|l| l.starts_with(prefix)).count()
    }
    fn token(&self) -> u64 {
        let t = self.next.get();
        self.next.set(t + 1);
        t
    }
    fn add(&self, otype: u32, x: i32, z: i32) -> u64 {
        let t = self.token();
        let mut props = BTreeMap::new();
        props.insert(prop::FLAMMABILITY, 100.0);
        self.occs.borrow_mut().insert(
            t,
            Occ {
                otype,
                cell: (x, z),
                in_manager: true,
                lot: Some(t + 0x10_0000),
                props,
                ..Occ::default()
            },
        );
        t
    }
    fn set_prop(&self, o: u64, id: u32, v: f64) {
        self.occs.borrow_mut().get_mut(&o).unwrap().props.insert(id, v);
    }
    fn set_coverage(&self, o: u64, c: u8) {
        self.occs.borrow_mut().get_mut(&o).unwrap().coverage = c;
    }
    fn occ<R>(&self, o: u64, f: impl FnOnce(&Occ) -> R) -> Option<R> {
        self.occs.borrow().get(&o).map(f)
    }
    /// A fire occupant answers with its target.
    fn placed(&self, o: u64) -> u64 {
        self.fire_targets.borrow().get(&o).copied().unwrap_or(o)
    }
    fn advance(&self, ms: u64) {
        self.now.set(self.now.get() + ms);
    }
}

impl DisasterHost for Fake {
    fn now_ms(&self) -> u64 {
        self.now.get()
    }
    fn emergency_pause(&self) {
        self.emergency.set(self.emergency.get() + 1);
        self.log("emergency_pause");
    }
    fn emergency_resume(&self) {
        self.emergency.set(self.emergency.get() - 1);
        self.log("emergency_resume");
    }
    fn disable_save(&self) {
        self.save_disabled.set(self.save_disabled.get() + 1);
        self.log("disable_save");
    }
    fn enable_save(&self) {
        self.save_disabled.set(self.save_disabled.get() - 1);
        self.log("enable_save");
    }
    fn sim_paused(&self) -> bool {
        self.paused.get() || self.emergency.get() > 0
    }
    fn sim_emergency_paused(&self) -> bool {
        self.emergency.get() > 0
    }
    fn city_established(&self) -> bool {
        self.established.get()
    }
    fn city_cells(&self) -> (u32, u32) {
        (64, 64)
    }
    fn cell_width(&self) -> f32 {
        16.0
    }
    fn post(&self, p: &DisasterPost) {
        self.log(format!(
            "post {:08X} {} {} {} {:X} {}",
            p.msg_type, p.data1, p.data2, p.data3, p.data4, p.void1
        ));
    }
    fn subscribe(&self, target: Target, m: u32) {
        self.log(format!("subscribe {} {m:08X}", tgt(target)));
    }
    fn unsubscribe(&self, target: Target, m: u32) {
        self.log(format!("unsubscribe {} {m:08X}", tgt(target)));
    }
    fn register_tick(&self, i: u32, p: u32) {
        self.log(format!("register_tick {i} {p:X}"));
    }
    fn unregister_tick(&self, i: u32) {
        self.log(format!("unregister_tick {i}"));
    }
    fn record_history(&self, s: u32, v: i32) {
        self.log(format!("history {s:08X} {v}"));
    }
    fn tuning(&self, i: u32) -> Option<Rc<PropertyHolder>> {
        self.tuning.borrow().get(&i).cloned()
    }
    fn add_ref(&self, t: u64) {
        *self.refs.borrow_mut().entry(t).or_default() += 1;
    }
    fn release(&self, t: u64) {
        *self.refs.borrow_mut().entry(t).or_default() -= 1;
    }
    fn follow_disasters(&self) -> bool {
        self.follow.get()
    }
    fn go_to_location(&self, x: i32, z: i32, zoom: u32) {
        self.log(format!("go_to {x} {z} {zoom}"));
    }
    fn fire_dispatch_count(&self) -> u32 {
        self.fire_dispatch.get()
    }
    fn police_dispatch_count(&self) -> u32 {
        self.police_dispatch.get()
    }
    fn building_type_flags(&self, o: u64) -> u32 {
        self.occ(o, |x| x.flags).unwrap_or(0)
    }
    fn occupant_type(&self, o: u64) -> u32 {
        self.occ(o, |x| x.otype).unwrap_or(0)
    }
    fn occupant_in_manager(&self, o: u64) -> bool {
        self.occ(o, |x| x.in_manager).unwrap_or(false)
    }
    fn occupant_cell(&self, o: u64) -> Option<(i32, i32)> {
        self.occ(o, |x| x.in_manager.then_some(x.cell)).flatten()
    }
    fn occupant_cell_rect(&self, o: u64) -> CellRect {
        self.occ(self.placed(o), |x| CellRect::cell(x.cell.0, x.cell.1))
            .unwrap_or_default()
    }
    fn occupant_position(&self, o: u64) -> (f32, f32) {
        self.occ(o, |x| (x.cell.0 as f32 * 16.0 + 8.0, x.cell.1 as f32 * 16.0 + 8.0))
            .unwrap_or((0.0, 0.0))
    }
    fn occupant_bbox(&self, o: u64) -> WorldBox {
        let (x, z) = self.occ(self.placed(o), |x| x.cell).unwrap_or((0, 0));
        WorldBox {
            x0: x as f32 * 16.0,
            z0: z as f32 * 16.0,
            x1: x as f32 * 16.0 + 16.0,
            z1: z as f32 * 16.0 + 16.0,
        }
    }
    fn occupant_bbox_areas(&self, _o: u64) -> (f32, f32) {
        (256.0, 512.0)
    }
    fn occupant_prop_u32(&self, o: u64, id: u32) -> Option<u32> {
        self.occ(o, |x| x.props.get(&id).map(|v| *v as u32)).flatten()
    }
    fn occupant_prop_i64(&self, o: u64, id: u32) -> Option<i64> {
        self.occ(o, |x| x.props.get(&id).map(|v| *v as i64)).flatten()
    }
    fn occupant_prop_f32(&self, o: u64, id: u32) -> Option<f32> {
        self.occ(o, |x| x.props.get(&id).map(|v| *v as f32)).flatten()
    }
    fn occupant_prop_bool(&self, o: u64, id: u32) -> Option<bool> {
        if id == prop::BURNING {
            return self.occ(o, |x| x.burning);
        }
        self.occ(o, |x| x.props.get(&id).map(|v| *v != 0.0)).flatten()
    }
    fn occupants_in_rect(&self, r: CellRect) -> Vec<u64> {
        self.occs
            .borrow()
            .iter()
            .filter(|(_, o)| o.in_manager && r.contains(o.cell.0, o.cell.1))
            .map(|(t, _)| *t)
            .collect()
    }
    fn remove_occupant(&self, o: u64) {
        self.log(format!("remove_occupant {o}"));
        if let Some(x) = self.occs.borrow_mut().get_mut(&o) {
            x.in_manager = false;
        }
    }
    fn occupant_lot(&self, o: u64) -> Option<u64> {
        self.occ(o, |x| x.lot).flatten()
    }
    fn set_lot_condition(&self, lot: u64, c: u32) {
        self.log(format!("condition {lot} {c}"));
    }
    fn fire_coverage(&self, o: u64) -> u8 {
        self.occ(o, |x| x.coverage).unwrap_or(0)
    }
    fn flammability_at(&self, x: i32, z: i32) -> u8 {
        self.flammability.borrow().get(&(x, z)).copied().unwrap_or(0)
    }
    fn create_fire_occupant(&self, _i: u32, target: u64, id: u32, flames: (u32, u32)) -> Option<u64> {
        let t = self.token();
        if let Some(x) = self.occs.borrow_mut().get_mut(&target) {
            x.burning = true;
        }
        self.fire_targets.borrow_mut().insert(t, target);
        self.log(format!("create_fire {t} {target} {id:08X} {} {}", flames.0, flames.1));
        Some(t)
    }
    fn remove_fire_occupant(&self, fire: u64, target: u64) {
        if let Some(x) = self.occs.borrow_mut().get_mut(&target) {
            x.burning = false;
        }
        self.fire_targets.borrow_mut().remove(&fire);
        self.log(format!("remove_fire {fire} {target}"));
    }
    fn add_fire_aura(&self, _o: u64) {}
    fn tutorial_no_burn_down(&self) -> bool {
        self.no_burn_down.get()
    }
    fn wind(&self) -> (f32, f32) {
        self.wind.get()
    }
    fn play_effect(&self, name: &str, o: u64) {
        self.log(format!("effect {name} {o}"));
    }
    fn play_effect_at(&self, name: &str, _x: f32, _z: f32) {
        self.log(format!("effect_at {name}"));
    }
    fn burn_flora(&self, o: u64) {
        self.log(format!("burn_flora {o}"));
    }
    fn demolish_lot(&self, o: u64, lot: u64, flags: u32) {
        self.log(format!("demolish {o} {lot} {flags:X}"));
        if self.demolish_notifies.get() {
            if let Some(x) = self.occs.borrow_mut().get_mut(&o) {
                x.in_manager = false;
            }
            // The occupant manager notifies synchronously, inside our call.
            let sys = self.sys.borrow().as_ref().and_then(Weak::upgrade);
            if let Some(s) = sys {
                let m = DisasterMessage {
                    void1: o,
                    ..DisasterMessage::of_type(msg::OCCUPANT_REMOVED)
                };
                s.do_message(Target::Layer, &m);
            }
        }
    }
    fn spew_radiation(&self, o: u64, a: f32, r: f32) {
        self.log(format!("radiation {o} {a} {r}"));
    }
    fn police_coverage(&self, x: i32, z: i32) -> u8 {
        self.police.borrow().get(&(x, z)).copied().unwrap_or(0)
    }
    fn is_road_cell(&self, x: i32, z: i32) -> bool {
        self.roads.borrow().contains(&(x, z))
    }
    fn cell_corner_position(&self, x: i32, z: i32) -> (f32, f32) {
        (x as f32 * 16.0, z as f32 * 16.0)
    }
    fn create_riot_leader(&self, x: f32, z: f32) -> Option<u64> {
        let t = self.token();
        let cell = ((x / 16.0) as i32, (z / 16.0) as i32);
        self.automata.borrow_mut().insert(t, Some(cell));
        self.log(format!("leader {t} {x} {z}"));
        Some(t)
    }
    fn network_occupant_at(&self, x: i32, z: i32) -> Option<u64> {
        self.network.borrow().get(&(x, z)).copied()
    }
    fn unset_removal_flags(&self, o: u64, flags: u32) {
        self.log(format!("unset_removal {o} {flags}"));
    }
    fn set_removal_flags(&self, o: u64, flags: u32) {
        self.log(format!("set_removal {o} {flags}"));
    }
    fn automaton_cell(&self, a: u64) -> Option<(i32, i32)> {
        self.automata.borrow().get(&a).copied().flatten()
    }
    fn set_riot_effect(&self, a: u64, s: i32) {
        self.log(format!("riot_effect {a} {s}"));
    }
    fn send_riot_message(&self, a: u64, s: i32) {
        self.log(format!("riot_message {a} {s}"));
    }
    fn end_riot_leader(&self, a: u64) {
        self.log(format!("end_leader {a}"));
    }
    fn riot_leader_tick(&self, _a: u64) {}
    fn create_toxic_occupant(&self, _i: u32, x: i32, z: i32, amount: i32) -> Option<u64> {
        let t = self.token();
        self.log(format!("toxic {t} {x} {z} {amount}"));
        Some(t)
    }
    fn set_toxic_amount(&self, _o: u64, _a: i32) {}
    fn remove_toxic_occupant(&self, o: u64) {
        self.log(format!("remove_toxic {o}"));
    }
    fn apply_terrain_brush(&self, b: u32, _x: f32, _z: f32, _s: f32, _w: f32) {
        self.log(format!("brush {b:X}"));
    }
    fn redraw_terrain(&self, _r: CellRect) {}
    fn end_long_terrain_operation(&self) {
        self.log("end_long_terrain");
    }
    fn stop_terrain_texture_updates(&self) {
        self.log("stop_textures");
    }
    fn place_repulsor(&self, _x: f32, _z: f32) {
        self.log("repulsor");
    }
    fn find_watered_pipe(&self, x: i32, z: i32) -> Option<u64> {
        self.pipes.borrow().get(&(x, z)).copied()
    }
    fn pipe_cell(&self, p: u64) -> (i32, i32) {
        self.pipes
            .borrow()
            .iter()
            .find(|(_, t)| **t == p)
            .map_or((0, 0), |(c, _)| *c)
    }
    fn view_level(&self) -> u32 {
        self.view_level.get()
    }
    fn create_leak_effect(&self, p: u64, level: u32) -> bool {
        self.log(format!("leak_effect {p} {level}"));
        level == 1 || level == 3
    }
    fn connected_pipe_cells(&self, p: u64) -> Vec<(i32, i32)> {
        self.pipe_neighbours.borrow().get(&p).cloned().unwrap_or_default()
    }
    fn demolish_pipe(&self, p: u64, flags: u32) {
        self.log(format!("demolish_pipe {p} {flags:X}"));
        self.pipes.borrow_mut().retain(|_, t| *t != p);
    }
    fn volcano_start_deformation(&self, _i: u32, _x: f32, _z: f32) {
        self.log("volcano_deform");
    }
    fn volcano_start_lava(&self, _i: u32, _x: f32, _z: f32, r: f32) {
        self.log(format!("lava {r}"));
    }
    fn volcano_lava_rates(&self, _i: u32, _p: f32, _s: f32) {}
    fn volcano_end_lava(&self, _i: u32) {
        self.log("lava_end");
    }
}

fn tgt(t: Target) -> String {
    match t {
        Target::Layer => "layer".into(),
        Target::Manager(m) => format!("manager {m:08X}"),
        Target::Instance(i) => format!("instance {i}"),
    }
}

/// A scripted generator: queued values, then defaults (`raw` 1, `u32`
/// n − 1, doubles 0.99, ranges `a + 0.99 (b − a)`, Gaussian ranges the
/// midpoint).
#[derive(Default)]
struct Script {
    u32s: Rc<RefCell<VecDeque<u32>>>,
    f64s: Rc<RefCell<VecDeque<f64>>>,
    draws: Rc<Cell<u32>>,
}

impl DisasterRandom for Script {
    fn raw_u32(&mut self) -> u32 {
        self.draws.set(self.draws.get() + 1);
        self.u32s.borrow_mut().pop_front().unwrap_or(1)
    }
    fn uniform_u32(&mut self, n: u32) -> u32 {
        self.draws.set(self.draws.get() + 1);
        self.u32s.borrow_mut().pop_front().unwrap_or(n.saturating_sub(1))
    }
    fn uniform_f64(&mut self) -> f64 {
        self.draws.set(self.draws.get() + 1);
        self.f64s.borrow_mut().pop_front().unwrap_or(0.99)
    }
    fn range_f64(&mut self, a: f64, b: f64) -> f64 {
        self.draws.set(self.draws.get() + 1);
        let u = self.f64s.borrow_mut().pop_front().unwrap_or(0.99);
        a + u * (b - a)
    }
    fn range_gaussian_fast(&mut self, a: f64, b: f64) -> f64 {
        self.draws.set(self.draws.get() + 1);
        let u = self.f64s.borrow_mut().pop_front().unwrap_or(0.5);
        a + u * (b - a)
    }
}

struct World {
    host: Rc<Fake>,
    sys: Rc<Disasters>,
    u32s: Rc<RefCell<VecDeque<u32>>>,
    f64s: Rc<RefCell<VecDeque<f64>>>,
    draws: Rc<Cell<u32>>,
}

fn world() -> World {
    let host = Fake::new();
    let sys = Rc::new(Disasters::new(host.clone(), 0x1234_5678));
    *host.sys.borrow_mut() = Some(Rc::downgrade(&sys));
    let s = Script::default();
    let (u32s, f64s, draws) = (s.u32s.clone(), s.f64s.clone(), s.draws.clone());
    sys.set_random(Generator::Fire, Box::new(s));
    let r = Script {
        u32s: u32s.clone(),
        f64s: f64s.clone(),
        draws: draws.clone(),
    };
    sys.set_random(Generator::Riot, Box::new(r));
    let p = Script {
        u32s: u32s.clone(),
        f64s: f64s.clone(),
        draws: draws.clone(),
    };
    sys.set_random(Generator::PipeBurst, Box::new(p));
    assert!(sys.init());
    host.take();
    World {
        host,
        sys,
        u32s,
        f64s,
        draws,
    }
}

impl World {
    /// The riot instances in the manager's list (`AttemptDisasterStart`
    /// returns null, §6.2).
    fn riots(&self) -> Vec<u32> {
        self.sys.manager_instances(kind::RIOT)
    }
    /// Starts a riot and returns the new instance, if one was made.
    fn riot(&self, x: i32, z: i32) -> Option<u32> {
        let before = self.riots();
        assert!(self.sys.attempt_start(kind::RIOT, x, z).is_none(), "always null");
        self.riots().into_iter().find(|i| !before.contains(i))
    }
    /// One riot tick after `ms` of wall clock.
    fn riot_tick(&self, id: u32, ms: u64) {
        self.host.advance(ms);
        self.sys.tick(id, 16_000);
    }
    /// One fire step: a second of wall clock and a tick.
    fn second(&self) {
        self.host.advance(1000);
        self.sys.fire_update();
    }
    fn fire(&self) -> FireEntry {
        self.sys.fire_snapshot().unwrap().fires[0]
    }
    fn posts(&self, t: u32) -> Vec<String> {
        let p = format!("post {t:08X}");
        self.host
            .log
            .borrow()
            .iter()
            .filter(|l| l.starts_with(&p))
            .cloned()
            .collect()
    }
}

// ---- §18 test 1 ----

#[test]
fn t01_coverage_threshold() {
    let t = FireTuning::from_holder(Some(&shipped_flame()));
    assert_eq!(t.d9.eval(50.0), 0.5);
    assert_eq!(t.resistance(50), 0);
    assert!((t.d9.eval(51.0) - 0.54).abs() < 1e-6);
    assert_eq!(t.resistance(51), 1);
    assert_eq!(t.resistance(0), 0);

    let w = world();
    let a = w.host.add(occupant::BUILDING, 5, 5);
    let b = w.host.add(occupant::BUILDING, 6, 5);
    let c = w.host.add(occupant::BUILDING, 7, 5);
    w.host.set_coverage(a, 50);
    w.host.set_coverage(b, 51);
    assert!(w.sys.ignite(a, false, 0, false));
    assert!(!w.sys.ignite(b, false, 0, false));
    assert!(w.sys.ignite(c, false, 0, false));
    assert_eq!(w.sys.fire_count(), 2);
    // A forced ignition skips the coverage test.
    assert!(w.sys.ignite(b, true, 7, false));
}

// ---- §18 test 2 ----

#[test]
fn t02_stage_lookup() {
    let t = FireTuning::from_holder(Some(&shipped_flame()));
    assert_eq!(t.d3cum, [30, 55, 75, 95, 120]);
    for (c, s) in [(0, 0), (29, 0), (30, 1), (54, 1), (55, 2), (119, 4), (120, 5), (500, 5)] {
        assert_eq!(t.stage_of(c), s, "counter {c}");
    }
    let d = FireTuning::from_holder(None);
    assert_eq!(d.d3cum, [10, 20, 30, 40, 50]);
    assert_eq!(d.d4, vec![10; 5]);
    assert_eq!(d.d5, vec![0, 3, 5, 7, 9]);
    assert_eq!(d.zoom, 4);
    assert!(d.d9.is_empty());
}

/// §5.1: the defaults apply only when the whole exemplar is missing; an
/// exemplar without a property leaves arrays and curves empty and D6 and
/// the zoom byte 0. Arrays keep their stored lengths.
#[test]
fn fire_tuning_defaults_only_without_the_exemplar() {
    let e = FireTuning::from_holder(Some(&PropertyHolder::new()));
    assert!(e.d3.is_empty() && e.d4.is_empty() && e.d5.is_empty());
    assert_eq!(e.d3cum, [0; 5]);
    assert_eq!((e.d6, e.zoom), (0.0, 0));
    assert!(e.d7.is_empty() && e.da.is_empty());
    let mut h = PropertyHolder::new();
    h.add(
        fire_prop::D4_SPREAD_DIVISOR,
        Variant::Array {
            elem: crate::variant::mem::UINT32,
            items: [4u32, 3, 2, 2, 1, 7].iter().map(|&x| Elem::U32(x)).collect(),
        },
    );
    h.add(
        fire_prop::D3_DURATIONS,
        Variant::Array {
            elem: crate::variant::mem::UINT32,
            items: [30u32, 25].iter().map(|&x| Elem::U32(x)).collect(),
        },
    );
    let t = FireTuning::from_holder(Some(&h));
    assert_eq!(t.d4, vec![4, 3, 2, 2, 1, 7], "the stored length");
    assert_eq!(t.d3cum, [30, 55, 80, 105, 130], "a short D3 clamps to its last entry");
}

// ---- §18 test 3 ----

#[test]
fn t03_unattended_burn_down() {
    let w = world();
    let b = w.host.add(occupant::BUILDING, 5, 5);
    w.sys.fire_instance(); // the instance's timer starts
    w.host.advance(10_000);
    assert!(w.sys.ignite(b, false, 0, true));
    assert_eq!(w.sys.fire_snapshot().unwrap().next_ms, 10_000);
    w.sys.fire_update(); // the first tick with timer ≥ 10000 ms: update 1
    assert_eq!(w.fire().counter, 1);
    // Update n at the first tick with timer ≥ 10000 + 1000 (n − 1) ms.
    w.host.advance(999);
    w.sys.fire_update();
    assert_eq!(w.fire().counter, 1, "not before 11000 ms");
    w.host.advance(1);
    w.sys.fire_update();
    assert_eq!(w.fire().counter, 2);
    for n in 3..=29 {
        w.second();
        assert_eq!(w.fire().counter, n);
    }
    assert_eq!(w.fire().counter, 29);
    assert_eq!(w.fire().stage, 0);
    w.second();
    assert_eq!(w.fire().stage, 1);
    w.host.take();
    for _ in 31..55 {
        w.second();
        assert_eq!(w.sys.fire_count(), 1);
    }
    assert_eq!(w.posts(msg::FIRE_STARTED_ENDED).len(), 0);
    w.second(); // the 55th update
    assert_eq!(w.sys.fire_snapshot().unwrap().elapsed_ms, 64_000);
    assert_eq!(w.sys.fire_count(), 0);
    let ended = w.posts(msg::FIRE_STARTED_ENDED);
    assert_eq!(ended.len(), 1);
    assert!(ended[0].ends_with(" 0 0 0 0"), "{ended:?}");
    assert_eq!(w.host.count("demolish"), 1);
    assert!(w
        .host
        .log
        .borrow()
        .contains(&format!("demolish {b} {} 300", b + 0x10_0000)));
}

#[test]
fn t03b_forced_burn_down_at_25() {
    let w = world();
    let b = w.host.add(occupant::BUILDING, 5, 5);
    assert!(w.sys.ignite(b, true, 0, true));
    assert_eq!(w.fire().counter, 30);
    w.sys.fire_update();
    for _ in 2..25 {
        w.second();
    }
    assert_eq!(w.sys.fire_count(), 1);
    assert_eq!(w.fire().counter, 54);
    w.second();
    assert_eq!(w.sys.fire_count(), 0);
    // maxStage below 2: half of D3cum[0].
    let p = w.host.add(occupant::PROP, 9, 9);
    w.host.set_prop(p, prop::MAX_STAGE, 1.0);
    w.sys.ignite(p, true, 0, false);
    assert_eq!(w.fire().counter, 15);
}

// ---- §18 test 4 ----

#[test]
fn t04_hosed_fire() {
    let w = world();
    let b = w.host.add(occupant::BUILDING, 5, 5);
    w.sys.ignite(b, false, 0, false);
    w.sys.fire_update();
    w.second();
    w.second();
    assert_eq!(w.fire().counter, 3);
    let fo = w.fire().occupant;
    assert!(w.sys.fire_set_hosed(fo, 1));
    w.u32s.borrow_mut().extend([4, 2500]);
    w.second();
    let e = w.fire();
    assert_eq!(e.counter, 0);
    let at = e.extinguish_at.expect("extinguish time");
    let elapsed = w.sys.fire_snapshot().unwrap().elapsed_ms;
    assert_eq!(at, elapsed + 2500);
    // The fire waits for its delay, then dies.
    w.second();
    w.second();
    assert_eq!(w.sys.fire_count(), 1);
    w.second();
    assert_eq!(w.sys.fire_count(), 0);
    assert_eq!(w.host.count("demolish"), 0);
}

// ---- §18 test 5 ----

#[test]
fn t05_episode_pause() {
    let w = world();
    let a = w.host.add(occupant::BUILDING, 5, 5);
    let b = w.host.add(occupant::BUILDING, 8, 8);
    w.sys.ignite(a, false, 0, false);
    let log = w.host.take();
    assert_eq!(log.iter().filter(|l| *l == "emergency_pause").count(), 1);
    assert_eq!(log.iter().filter(|l| *l == "disable_save").count(), 1);
    let fire = w.sys.fire_instance();
    let started: Vec<&String> = log.iter().filter(|l| l.starts_with("post CA1B1E64")).collect();
    assert_eq!(started.len(), 1);
    assert_eq!(*started[0], format!("post CA1B1E64 1 5 5 621CDA33 {fire}"));
    w.sys.ignite(b, false, 0, false);
    let log = w.host.take();
    assert!(!log
        .iter()
        .any(|l| l.starts_with("post CA1B1E64") || l == "emergency_pause"));
    assert!(w.sys.is_disaster_ongoing());
    // Both burn down at the 55th update; the episode ends on the next tick.
    w.sys.fire_update();
    for _ in 2..=55 {
        w.second();
    }
    assert_eq!(w.sys.fire_count(), 0);
    w.sys.fire_update();
    let log = w.host.take();
    for e in [
        "enable_save",
        "emergency_resume",
        "post AA1B2002 0 0 0 0 0",
        "history 49D15B74 1",
    ] {
        assert_eq!(log.iter().filter(|l| *l == e).count(), 1, "{e} in {log:?}");
    }
    assert!(!w.sys.is_disaster_ongoing());
    assert_eq!(w.host.emergency.get(), 0);
    assert_eq!(w.host.save_disabled.get(), 0);
    assert_eq!(w.sys.fire_snapshot().unwrap().phase, 3);
    assert!(log.iter().any(|l| l == &format!("unregister_tick {fire}")));
}

// ---- §18 test 6 ----

#[test]
fn t06_nested_enable_counter() {
    let w = world();
    assert!(w.sys.disasters_enabled());
    assert!(!w.sys.enable_disasters(false));
    assert!(!w.sys.enable_disasters(false));
    assert!(!w.sys.enable_disasters(true));
    assert!(!w.sys.disasters_enabled());
    assert!(w.sys.enable_disasters(true));
    assert!(w.sys.enable_disasters(true));
    assert_eq!(w.sys.layer_snapshot().disable_counter, 0);
}

// ---- §18 test 7 ----

fn riot_world() -> World {
    let w = world();
    for x in 0..64 {
        w.host.roads.borrow_mut().insert((x, 20));
    }
    w
}

#[test]
fn t07_riot_timing() {
    let w = riot_world();
    w.f64s.borrow_mut().extend([0.5, 0.25]);
    let id = w.riot(10, 20).expect("riot");
    let log = w.host.take();
    assert!(log.contains(&"leader 4096 168 324".to_string()), "{log:?}");
    assert!(log.contains(&"history AA8C87C7 1".to_string()));
    let first = w.sys.riot_snapshot(id).unwrap().leaders[0];
    assert_eq!((first.stage, first.max_stage, first.counter), (-1, -1, 0));
    let first = first.automaton;
    // Step n runs at riot second n − 1: the first at once (next starts at 0).
    for n in 1..=180u64 {
        w.riot_tick(id, if n == 1 { 0 } else { 1000 });
        let r = w.sys.riot_snapshot(id).unwrap();
        assert_eq!(r.elapsed_s, n - 1);
        assert_eq!(r.next_s, n);
        let log = w.host.take();
        let alive = r.leaders.iter().any(|l| l.automaton == first);
        let msgs = log
            .iter()
            .filter(|l| l.starts_with(&format!("riot_message {first} ")))
            .count();
        match n {
            1 => {
                assert_eq!(r.leaders[0].stage, 0);
                assert_eq!(msgs, 1);
                assert!(log.contains(&format!("riot_message {first} 0")), "{log:?}");
                assert_eq!(r.leaders.len(), 1, "no spawn at stage 0");
            }
            29 => assert_eq!(r.leaders.len(), 1),
            30 => {
                assert_eq!(r.leaders.len(), 2, "stage 1 spawns one");
                assert!(log.contains(&format!("riot_message {first} 1")));
            }
            // At step 90 the first leader reaches stage 3 and the leader
            // spawned at step 60 (by the one spawned at step 30) reaches
            // stage 1: two more.
            90 => {
                assert_eq!(r.leaders.len(), 5);
                assert!(log.contains(&format!("riot_message {first} 3")));
            }
            _ => {}
        }
        if n < 180 {
            assert!(alive, "first leader alive at step {n}");
        } else {
            assert!(!alive, "the first leader ends at step 180");
            assert!(log.contains(&format!("end_leader {first}")));
        }
    }
    // A tick inside the same second runs no step (next := elapsed + 1).
    let before = w.sys.riot_snapshot(id).unwrap();
    w.riot_tick(id, 500);
    assert_eq!(w.sys.riot_snapshot(id).unwrap().leaders, before.leaders);
}

#[test]
fn t07b_riot_suppressed_and_stays() {
    let w = riot_world();
    let id = w.riot(10, 20).expect("riot");
    let a = w.sys.riot_snapshot(id).unwrap().leaders[0].automaton;
    // Step 1: stage 0.
    w.riot_tick(id, 0);
    // Coverage 255 (curve 5): the next step adds −4, stage −1, it ends.
    w.host.police.borrow_mut().insert((10, 20), 255);
    w.host.take();
    w.riot_tick(id, 1000);
    let log = w.host.take();
    assert!(log.contains(&format!("riot_effect {a} -1")), "{log:?}");
    assert!(log.contains(&format!("end_leader {a}")));
    assert!(log.contains(&"post AA1B2002 0 0 0 0 0".to_string()));
    // The instance stays in its manager, on the tick, in phase 3.
    let r = w.sys.riot_snapshot(id).expect("the riot stays in its manager");
    assert_eq!(r.phase, 3);
    assert!(r.leaders.is_empty());
    assert_eq!(w.riots(), vec![id]);
    assert_eq!(w.sys.active_count(kind::RIOT), 0);
    assert!(w.sys.instance_info(id).unwrap().tick_registered);
    assert!(w.sys.attempt_start(kind::RIOT, 30, 20).is_none(), "still null");
    w.riot_tick(id, 1000);
    assert_eq!(
        w.sys.riot_snapshot(id).unwrap().next_s,
        3,
        "an empty update every second"
    );
    // A leader whose automaton has gone ends at its next step.
    let b = w.riots()[1];
    let l = w.sys.riot_snapshot(b).unwrap().leaders[0].automaton;
    w.host.automata.borrow_mut().insert(l, None);
    w.riot_tick(b, 0);
    assert!(w.host.log.borrow().contains(&format!("end_leader {l}")));
    assert_eq!(w.sys.riot_snapshot(b).unwrap().phase, 3);
}

/// network-manager.md §10 (disasters.md Q4, §6.5 step 6, §6.7): the riots
/// lock the network pieces around their starts and wherever a leader walks
/// against the player's bulldozer; an ended riot stays in its manager, so
/// the locks hold until the manager's `Shutdown`.
#[test]
fn riot_locks_the_network_for_the_session() {
    let w = riot_world();
    for x in 0..64 {
        w.host.network.borrow_mut().insert((x, 20), 0x7000 + x as u64);
    }
    let a = w.riot(10, 20).unwrap();
    let row = |r: std::ops::RangeInclusive<u64>| r.map(|x| 0x7000 + x).collect::<Vec<_>>();
    assert_eq!(w.sys.riot_locked(), row(7..=13), "each piece once");
    assert_eq!(w.host.count("unset_removal"), 7);
    assert_eq!(w.host.refs.borrow().get(&0x700A).copied(), Some(1));
    w.riot(12, 20).unwrap();
    assert_eq!(w.sys.riot_locked(), row(7..=15));
    // Within 3 cells of the west edge nothing is locked.
    w.riot(2, 20).unwrap();
    assert_eq!(w.sys.riot_locked().len(), 9);
    // The first step remembers the piece under the leader (held).
    let la = w.sys.riot_snapshot(a).unwrap().leaders[0].automaton;
    w.riot_tick(a, 0);
    assert_eq!(w.sys.riot_locked().len(), 9);
    assert_eq!(w.host.refs.borrow().get(&0x700A).copied(), Some(2));
    // A walking leader locks the road around its new piece.
    w.host.automata.borrow_mut().insert(la, Some((20, 20)));
    w.riot_tick(a, 1000);
    assert_eq!(w.sys.riot_locked(), [row(7..=15), row(17..=23)].concat());
    assert_eq!(w.host.refs.borrow().get(&0x700A).copied(), Some(1));
    // The riot ends; its locks stay.
    w.host.police.borrow_mut().insert((20, 20), 255);
    w.host.take();
    w.riot_tick(a, 1000);
    assert_eq!(w.sys.riot_snapshot(a).unwrap().phase, 3);
    assert_eq!(w.host.count("set_removal"), 0, "an ended riot keeps its locks");
    assert_eq!(w.sys.riot_locked().len(), 16);
    // The manager's Shutdown unlocks everything.
    assert!(w.sys.shutdown());
    assert_eq!(w.host.count("set_removal"), 16);
    assert!(w.sys.riot_locked().is_empty());
    assert!(
        w.host.refs.borrow().values().all(|r| *r == 0),
        "{:?}",
        w.host.refs.borrow()
    );
}

// ---- §18 test 8 ----

#[test]
fn t08_riot_leader_limit() {
    let t = RiotTuning::from_holder(Some(&shipped_riot()));
    let c112 = t.police.eval(112.0);
    assert!((c112 - 1.989_473_7).abs() < 1e-5);
    assert!((t.police.eval(113.0) - 2.010_526_3).abs() < 1e-5);
    let w = riot_world();
    w.host.police.borrow_mut().insert((10, 20), 112);
    assert!(w.riot(10, 20).is_some());
    assert_eq!(w.sys.active_count(kind::RIOT), 1);
    w.host.police.borrow_mut().insert((30, 20), 113);
    let id = w.riot(30, 20).expect("instance");
    let r = w.sys.riot_snapshot(id).unwrap();
    assert!(r.leaders.is_empty(), "refused leader");
    assert_eq!(r.phase, 0, "stays in phase 0, ticking");
    assert_eq!(w.sys.active_count(kind::RIOT), 1);
    assert_eq!(w.riots().len(), 2);
}

/// §6.1: the defaults. Without the exemplar: 3 riots, 5 leaders, 6 stages
/// of 30 s, spawned 0, 1, 0, 1, 0, 0, an empty police curve and zoom 0. An
/// exemplar whose police curve has no points takes (64, 0) (65, 1) (255, 3).
#[test]
fn riot_tuning_defaults() {
    let d = RiotTuning::from_holder(None);
    assert_eq!((d.max_riots, d.max_leaders), (3, 5));
    assert_eq!((d.stages, d.seconds_per_stage), (6, 30));
    assert_eq!(d.leaders_spawned, vec![0, 1, 0, 1, 0, 0]);
    assert!(d.police.is_empty());
    assert_eq!(d.zoom, 0);
    let e = RiotTuning::from_holder(Some(&PropertyHolder::new()));
    assert_eq!(e.max_leaders, 5);
    assert_eq!(e.police.eval(64.0), 0.0);
    assert_eq!(e.police.eval(65.0), 1.0);
    assert_eq!(e.police.eval(255.0), 3.0);
    assert_eq!(e.zoom, 0);
}

#[test]
fn t08b_riot_limits_and_search() {
    let w = riot_world();
    // At most three active riots.
    for x in [5, 15, 25] {
        assert!(w.riot(x, 20).is_some());
    }
    assert_eq!(w.sys.active_count(kind::RIOT), 3);
    assert!(w.riot(35, 20).is_none());
    // The nearest road within 7 cells; none further.
    let w = riot_world();
    let id = w.riot(10, 27).unwrap();
    // Ring 7 starts at (17, 21) and runs +z, −x, then −z down x = 3: the
    // first road cell of the row z = 20 it meets is (3, 20).
    assert_eq!(w.sys.riot_snapshot(id).unwrap().leaders[0].cell, (3, 20));
    let id = w.riot(10, 28).unwrap();
    assert!(w.sys.riot_snapshot(id).unwrap().leaders.is_empty());
}

// ---- §18 test 13 ----

#[test]
fn t13_riot_road_search_spiral() {
    let w = world();
    w.host.roads.borrow_mut().insert((11, 10));
    let id = w.riot(10, 10).unwrap();
    assert_eq!(w.sys.riot_snapshot(id).unwrap().leaders[0].cell, (11, 10));
    let w = world();
    w.host.roads.borrow_mut().extend([(10, 11), (11, 10)]);
    let id = w.riot(10, 10).unwrap();
    assert_eq!(w.sys.riot_snapshot(id).unwrap().leaders[0].cell, (11, 10), "+z first");
    // Ring 1 in full, and cells outside the city are skipped, not clamped.
    assert_eq!(
        super::riot::riot_spiral(10, 10, 1),
        vec![
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
    let w = world();
    w.host.roads.borrow_mut().insert((0, 0));
    let id = w.riot(-3, -3).unwrap();
    assert_eq!(w.sys.riot_snapshot(id).unwrap().leaders[0].cell, (0, 0));
    let w = world();
    w.host.roads.borrow_mut().insert((0, 5));
    let id = w.riot(-8, 5).unwrap();
    assert!(w.sys.riot_snapshot(id).unwrap().leaders.is_empty(), "no clamping");
}

// ---- §18 test 9 ----

#[test]
fn t09_toxic_spill() {
    let w = world();
    let id = w.sys.attempt_start(kind::TOXIC, 3, 4).expect("toxic");
    let log = w.host.take();
    assert!(log.contains(&"emergency_pause".to_string()));
    assert!(log.contains(&format!("register_tick {id} FF")));
    for n in 1..=119 {
        w.host.advance(1000);
        w.sys.tick(id, 16_000);
        assert_eq!(w.sys.toxic_snapshot().unwrap().spills[0].amount, 120 - n);
    }
    assert_eq!(w.sys.toxic_snapshot().unwrap().spills[0].amount, 1);
    w.host.take();
    w.host.advance(1000);
    w.sys.tick(id, 16_000);
    let log = w.host.take();
    assert!(log.iter().any(|l| l.starts_with("remove_toxic")));
    assert!(log.contains(&"post AA1B2002 0 0 0 0 0".to_string()));
    assert!(w.sys.toxic_snapshot().is_none());

    // One hose: 30 updates.
    let id = w.sys.spill_toxic_waste(3, 4, 120).unwrap();
    let o = w.sys.toxic_snapshot().unwrap().spills[0].occupant;
    assert!(w.sys.toxic_set_hosed(o, 1));
    for _ in 0..29 {
        w.host.advance(1000);
        w.sys.tick(id, 16_000);
    }
    assert_eq!(w.sys.toxic_snapshot().unwrap().spills[0].amount, 4);
    w.host.advance(1000);
    w.sys.tick(id, 16_000);
    assert!(w.sys.toxic_snapshot().is_none());
}

// ---- §18 test 10 ----

#[test]
fn t10_dispatch_effectiveness() {
    let w = world();
    w.host.fire_dispatch.set(4);
    w.host.police_dispatch.set(3);
    let b = w.host.add(occupant::BUILDING, 5, 5);
    w.sys.ignite(b, false, 0, false);
    let d = |t: u32, ty: i64, src: i64| {
        w.sys.do_message(
            Target::Layer,
            &DisasterMessage {
                data1: ty,
                data2: src,
                ..DisasterMessage::of_type(t)
            },
        );
    };
    d(msg::DISPATCH_BEGIN, 1, 1);
    d(msg::DISPATCH_ACTIVE, 1, 1);
    d(msg::DISPATCH_BEGIN, 5, 2);
    d(msg::DISPATCH_ACTIVE, 5, 2);
    w.host.advance(30_000);
    d(msg::DISPATCH_INACTIVE, 1, 1);
    w.host.advance(20_000);
    d(msg::DISPATCH_END, 5, 2);
    d(msg::DISPATCH_ACTIVE, 3, 9); // another type: ignored
    w.host.advance(50_000);
    w.sys.extinguish(b);
    w.host.advance(5000);
    w.sys.fire_update();
    w.host.take();
    // End the episode at exactly 100 s of overall time.
    w.host.now.set(100_000);
    let snap = w.sys.layer_snapshot();
    assert_eq!(snap.fire_timers, vec![(1, 30), (2, 50)]);
    assert!(snap.police_timers.is_empty());
    w.sys.fire_update();
    let log = w.host.take();
    assert!(log.contains(&"post AA1B2002 0 0 0 0 0".to_string()), "{log:?}");
    // (30 + 50) / (4 × 100); an empty map gives 0; stored only.
    let s = w.sys.layer_snapshot();
    assert_eq!(s.fire_effectiveness, 0.2);
    assert_eq!(s.police_effectiveness, 0.0);
    assert!(!log.iter().any(|l| l.contains("effectiveness")));
}

/// §18 test 10: a dispatch count of 0 gives 0, and the seconds are
/// fractional (a 2.5 s timer counts 2.5).
#[test]
fn t10b_effectiveness_count_and_fractions() {
    let run = |count: u32, timer_ms: u64| -> f32 {
        let w = world();
        w.host.fire_dispatch.set(count);
        let b = w.host.add(occupant::BUILDING, 5, 5);
        w.sys.ignite(b, false, 0, false);
        let m = |t: u32| DisasterMessage {
            data1: 1,
            data2: 7,
            ..DisasterMessage::of_type(t)
        };
        w.sys.do_message(Target::Layer, &m(msg::DISPATCH_ACTIVE));
        w.host.advance(timer_ms);
        w.sys.do_message(Target::Layer, &m(msg::DISPATCH_INACTIVE));
        w.host.now.set(10_000);
        w.sys.burn_down(b);
        w.sys.fire_update();
        assert!(!w.sys.is_disaster_ongoing());
        w.sys.layer_snapshot().fire_effectiveness
    };
    assert_eq!(run(0, 5000), 0.0);
    assert_eq!(run(1, 2500), 0.25);
    assert_eq!(run(2, 2500), 0.125);
}

// ---- around the tests ----

#[test]
fn layer_init_order_and_managers() {
    let w = world();
    let s = w.sys.layer_snapshot();
    assert_eq!(
        s.managers,
        vec![
            kind::EARTHQUAKE,
            kind::PIPE_BURST,
            kind::FIRE,
            kind::EFFECT,
            kind::VOLCANO,
            kind::RIOT,
            kind::TOXIC
        ]
    );
    for (t, _) in MANAGER_TABLE {
        assert!(w.sys.has_manager(t));
        assert!(is_layer_manager(t));
    }
    assert!(!w.sys.has_manager(0x6A6D_0EE7));
    assert!(w.sys.shutdown());
    assert!(!w.sys.has_manager(kind::FIRE));
}

#[test]
fn ignite_occupant_quirk_and_pause() {
    let w = world();
    let b = w.host.add(occupant::BUILDING, 5, 5);
    w.host.set_coverage(b, 200);
    assert!(w.sys.ignite_occupant(b), "true although refused");
    assert_eq!(w.sys.fire_count(), 0);
    assert!(!w.sys.ignite_occupant(0));
    // A manual pause blocks a non-forced ignition; the emergency pause does not.
    w.host.set_coverage(b, 0);
    w.host.paused.set(true);
    assert!(!w.sys.ignite(b, false, 0, false));
    w.host.emergency.set(1);
    assert!(w.sys.ignite(b, false, 0, false));
    // Not burnable: a network piece; no flammability.
    let n = w.host.add(occupant::NETWORK, 1, 1);
    assert!(!w.sys.ignite(n, true, 0, false));
    let f = w.host.add(occupant::FLORA, 2, 2);
    w.host.set_prop(f, prop::FLAMMABILITY, 0.0);
    assert!(!w.sys.ignite(f, true, 0, false));
    // Already burning: forced adds 1, otherwise false.
    let c = w.fire().counter;
    assert!(w.sys.ignite(b, true, 0, false));
    assert_eq!(w.fire().counter, c + 1);
    assert!(!w.sys.ignite(b, false, 0, false));
}

#[test]
fn ignite_cell_probability_and_ids() {
    let w = world();
    let a = w.host.add(occupant::BUILDING, 4, 4);
    let b = w.host.add(occupant::FLORA, 4, 4);
    w.host.flammability.borrow_mut().insert((4, 4), 60);
    // DA(60) = 14: a draw of 14.5 refuses, 14.0 passes.
    w.f64s.borrow_mut().push_back(0.145);
    assert!(!w.sys.ignite_cell(4, 4, true, false, 0));
    assert_eq!(w.draws.get(), 1);
    w.f64s.borrow_mut().push_back(0.13);
    w.u32s.borrow_mut().push_back(0xABCD);
    assert!(w.sys.ignite_cell(4, 4, true, false, 0));
    let s = w.sys.fire_snapshot().unwrap();
    assert_eq!(s.fires.len(), 2);
    assert!(s.fires.iter().all(|f| f.fire_id == 0xABCD), "one ID per call");
    assert_eq!(s.fire_ids, vec![0xABCD]);
    assert_eq!(s.ignited_buildings, 1);
    assert_eq!(s.ignited_flora, 1);
    // One location for the ID (the first occupant's).
    let l = w.sys.layer_snapshot().locations;
    assert_eq!(l.len(), 1);
    assert_eq!(l[0].occupant, a);
    let _ = b;
    w.host.take();
    w.sys.fire_update();
    assert!(w.host.take().contains(&"post 49D15B73 1 1 0 0 0".to_string()));
    // The manager slot returns true whatever happened.
    assert!(w.sys.manager_ignite_cell(9, 9, false, false, 0));
}

#[test]
fn lot_conditions_follow_the_stage() {
    let w = world();
    let b = w.host.add(occupant::BUILDING, 5, 5);
    let lot = b + 0x10_0000;
    w.sys.ignite(b, false, 0, false);
    w.sys.fire_update();
    assert!(
        w.host.take().contains(&format!("condition {lot} 5")),
        "maxStage 2: 5 at once"
    );
    for _ in 2..30 {
        w.second();
    }
    w.host.take();
    w.second();
    assert!(w.host.take().contains(&format!("condition {lot} 6")), "stage 1: 6");
    // A prop takes conditions only from the step that crosses 20.
    let w = world();
    let p = w.host.add(occupant::PROP, 5, 5);
    w.sys.ignite(p, false, 0, false);
    w.sys.fire_update();
    for _ in 2..20 {
        w.second();
    }
    assert_eq!(w.host.count("condition"), 0);
    w.second();
    assert_eq!(w.host.count("condition"), 1);
    w.second();
    assert_eq!(w.host.count("condition"), 1);
}

#[test]
fn no_burn_down_flag_keeps_the_fire() {
    let w = world();
    let b = w.host.add(occupant::BUILDING, 5, 5);
    w.host.no_burn_down.set(true);
    w.sys.ignite(b, false, 0, false);
    w.sys.fire_update();
    for _ in 2..=70 {
        w.second();
    }
    assert_eq!(w.sys.fire_count(), 1);
    assert_eq!(w.fire().stage, 1);
    assert_eq!(w.fire().counter, 70);
}

#[test]
fn spread_and_kill_fire_moves_the_location() {
    let w = world();
    let a = w.host.add(occupant::BUILDING, 5, 5);
    let b = w.host.add(occupant::BUILDING, 6, 5);
    w.host.flammability.borrow_mut().insert((6, 5), 255);
    w.u32s.borrow_mut().push_back(0x77);
    w.sys.ignite(a, false, 0, false);
    // Spread: draw 0.1 < 1/4, Gaussians toward cell (6, 5), probability 0.
    w.f64s.borrow_mut().extend([0.1, 0.65, 0.5, 0.0]);
    w.sys.fire_update();
    let s = w.sys.fire_snapshot().unwrap();
    assert_eq!(s.fires.len(), 2);
    assert_eq!(s.fires[1].target, b);
    assert_eq!(s.fires[1].fire_id, 0x77);
    assert_eq!(w.sys.layer_snapshot().locations.len(), 1);
    // Killing the first moves the ID's location to the second.
    let fo = s.fires[0].occupant;
    w.host.occs.borrow_mut().get_mut(&a).unwrap().in_manager = false;
    w.second();
    let l = w.sys.layer_snapshot().locations;
    assert_eq!(l.len(), 1);
    assert_eq!(l[0].occupant, b);
    assert!(w.host.log.borrow().contains(&format!("remove_fire {fo} {a}")));
    assert_eq!(w.sys.fire_snapshot().unwrap().fire_ids, vec![0x77]);
}

/// §5.10 step 9: only a step that reaches the spread draws; the box is the
/// fire's, widened by the stage's radius plus the wind vector × D6 (a
/// negative component narrows it).
#[test]
fn spread_draw_and_wind_vector() {
    let w = world();
    let a = w.host.add(occupant::BUILDING, 5, 5);
    w.sys.ignite(a, false, 0, false);
    let d = w.draws.get();
    w.sys.fire_update();
    assert_eq!(w.draws.get(), d + 1, "one spread draw, no spread");
    for _ in 2..55 {
        w.second();
    }
    let d = w.draws.get();
    w.second(); // the burn-down step stops before the spread
    assert_eq!(w.sys.fire_count(), 0);
    assert_eq!(w.draws.get(), d);
    // Wind (−16, 0), D6 1, D5[0] 32: x in [80 − 16, 96 + 16].
    let w = world();
    w.host.wind.set((-16.0, 0.0));
    let a = w.host.add(occupant::BUILDING, 5, 5);
    let b = w.host.add(occupant::BUILDING, 4, 5);
    w.host.flammability.borrow_mut().insert((4, 5), 255);
    w.sys.ignite(a, false, 0, false);
    w.f64s.borrow_mut().extend([0.1, 0.0, 0.5, 0.0]);
    w.sys.fire_update();
    let s = w.sys.fire_snapshot().unwrap();
    assert_eq!(s.fires.len(), 2);
    assert_eq!(s.fires[1].target, b);
}

#[test]
fn toast_list_and_radius() {
    let w = world();
    let flora = w.host.add(occupant::FLORA, 10, 10);
    let near = w.host.add(occupant::BUILDING, 10, 10);
    let mid = w.host.add(occupant::BUILDING, 12, 10);
    let far = w.host.add(occupant::BUILDING, 15, 10);
    w.host.set_prop(near, prop::RADIATION_RADIUS, 5.0);
    w.host.set_prop(near, prop::RADIATION_AMOUNT, 2.0);
    w.sys.ignite_cell_radius(10, 10, 1, 3);
    let s = w.sys.fire_snapshot().unwrap();
    // The second entry draws Uniform(1) = 0 and goes first (§5.7).
    assert_eq!(s.toast, vec![near, flora]);
    assert_eq!(s.fires.len(), 1);
    assert_eq!(s.fires[0].target, mid);
    let _ = far;
    w.host.take();
    w.sys.fire_update();
    let log = w.host.take();
    assert!(log.contains(&format!("effect burndown_flora {flora}")));
    assert!(log.contains(&format!("effect fire_flora_scorch {flora}")));
    assert!(log.contains(&format!("burn_flora {flora}")));
    assert!(log.contains(&format!("effect burndown_building {near}")));
    assert!(log.contains(&format!("radiation {near} 2 5")));
    assert_eq!(w.host.refs.borrow().get(&flora).copied(), Some(0));
    // Five per update, from the front. The script's draws are n − 1: each
    // entry goes before the last one, so the list is p1 .. p6, p0.
    let w = world();
    let list: Vec<u64> = (0..7).map(|_| w.host.add(occupant::PROP, 3, 3)).collect();
    w.host.set_prop(list[1], prop::BURN_DOWN_DEMOLISHES, 0.0);
    w.sys.quick_burn_cell(3, 3);
    let mut want = list[1..].to_vec();
    want.push(list[0]);
    assert_eq!(w.sys.fire_snapshot().unwrap().toast, want);
    w.sys.fire_update();
    assert_eq!(w.sys.fire_snapshot().unwrap().toast, vec![list[6], list[0]]);
    assert!(w.host.log.borrow().contains(&format!("remove_occupant {}", list[1])));
}

// ---- §18 test 12 ----

#[test]
fn t12_toast_insertion() {
    let w = world();
    let p = w.host.add(occupant::PROP, 3, 3);
    let q = w.host.add(occupant::PROP, 4, 4);
    let before = w.draws.get();
    w.sys.quick_burn_cell(3, 3);
    assert_eq!(w.draws.get(), before, "an empty list takes no draw");
    w.sys.quick_burn_cell(4, 4);
    assert_eq!(w.draws.get(), before + 1, "Uniform(1)");
    assert_eq!(w.sys.fire_snapshot().unwrap().toast, vec![q, p]);
    // A draw of i puts the entry before element i.
    let r = w.host.add(occupant::PROP, 5, 5);
    w.u32s.borrow_mut().push_back(1);
    w.sys.quick_burn_cell(5, 5);
    assert_eq!(w.sys.fire_snapshot().unwrap().toast, vec![q, r, p]);
}

#[test]
fn extinguish_cell_and_counts() {
    let w = world();
    let a = w.host.add(occupant::BUILDING, 5, 5);
    let b = w.host.add(occupant::BUILDING, 9, 9);
    w.sys.ignite(a, true, 0, false);
    w.sys.ignite(b, true, 0, false);
    assert_eq!(w.sys.fire_count(), 2);
    assert_eq!(
        w.sys.fire_count_in_rect(CellRect {
            x0: 0,
            z0: 0,
            x1: 6,
            z1: 6
        }),
        1
    );
    w.sys.extinguish_cell(5, 5, 10);
    assert_eq!(w.fire().counter, 20);
    w.u32s.borrow_mut().push_back(100);
    w.sys.extinguish_cell(5, 5, 20);
    assert_eq!(w.fire().counter, 0);
    assert!(w.fire().extinguish_at.is_some());
    w.sys.set_cause(2);
    assert_eq!(w.sys.cause(), 2);
    w.sys.burn_down(b);
    assert_eq!(w.sys.fire_count(), 1);
    // A fire with an extinguish time still counts and is still hit.
    assert_eq!(w.sys.fire_count_in_rect(CellRect::cell(5, 5)), 1);
    w.sys.extinguish_cell(5, 5, 1);
    assert_eq!(w.fire().counter, 0);
}

/// §5.7, §5.9: `SetCause` keeps the first cause of an episode; the next
/// episode's first fire resets it, so a cause set before it is lost.
#[test]
fn set_cause_first_wins_per_episode() {
    let w = world();
    let a = w.host.add(occupant::BUILDING, 5, 5);
    assert_eq!(w.sys.set_cause(2), 2, "before any fire");
    w.sys.ignite(a, false, 0, false);
    assert_eq!(w.sys.cause(), 0, "the episode start resets it");
    assert_eq!(w.sys.set_cause(2), 2);
    assert_eq!(w.sys.set_cause(1), 2, "the first wins");
    assert_eq!(w.sys.cause(), 2);
    let b = w.host.add(occupant::BUILDING, 9, 9);
    w.sys.ignite(b, false, 0, false);
    assert_eq!(w.sys.cause(), 2, "a second fire of the episode keeps it");
    w.sys.burn_down(a);
    w.sys.burn_down(b);
    w.sys.fire_update();
    assert_eq!(w.sys.fire_snapshot().unwrap().phase, 3);
    assert_eq!(w.sys.cause(), 2, "kept after the episode");
    let c = w.host.add(occupant::BUILDING, 20, 20);
    w.sys.ignite(c, false, 0, false);
    assert_eq!(w.sys.cause(), 0);
}

#[test]
fn damage_assessment() {
    let w = world();
    let a = w.host.add(occupant::BUILDING, 5, 5);
    w.sys.ignite(a, false, 0, false);
    let b = w.host.add(occupant::BUILDING, 1, 1);
    w.host.set_prop(b, prop::DEMOLITION_COST, 500.0);
    w.host.occs.borrow_mut().get_mut(&b).unwrap().flags = 0x400 | 0x2 | 0x20;
    let n = w.host.add(occupant::NETWORK, 2, 2);
    let m = |o: u64| DisasterMessage {
        void1: o,
        ..DisasterMessage::of_type(msg::OCCUPANT_REMOVED)
    };
    w.sys.do_message(Target::Layer, &m(b));
    w.sys.do_message(Target::Layer, &m(n));
    let d = w.sys.layer_snapshot().damage;
    assert_eq!(d.building_count, 1);
    assert_eq!(d.total_cost, 510, "a network piece without a cost counts 10");
    assert_eq!(d.kinds, [(0, 0), (1, 500), (0, 0), (1, 500)]);
    assert_eq!(d.special.len(), 1);
    assert_eq!(w.sys.disaster_cost(), 510);
    // Not established: nothing counts.
    w.host.established.set(false);
    w.sys.do_message(Target::Layer, &m(b));
    assert_eq!(w.sys.disaster_cost(), 510);
}

#[test]
fn reentrant_occupant_removal_is_deferred() {
    let w = world();
    w.host.demolish_notifies.set(true);
    let b = w.host.add(occupant::BUILDING, 5, 5);
    w.host.set_prop(b, prop::DEMOLITION_COST, 900.0);
    w.sys.burn_down(b); // no fire: nothing
    w.sys.ignite(b, true, 0, false);
    w.sys.burn_down(b);
    assert_eq!(w.host.count("demolish"), 1);
    assert_eq!(w.sys.disaster_cost(), 900, "the deferred removal counted");
    assert_eq!(w.sys.deferred_messages(), 0);
    assert_eq!(w.sys.reentrant_refused(), 0);
}

#[test]
fn locations_and_go_to_next() {
    let w = world();
    let o = w.host.add(occupant::BUILDING, 7, 8);
    w.sys.register_location(9, 1, 2, 0, 3, false);
    w.sys.register_location(9, 0, 0, o, 3, false);
    w.sys.register_location(4, 30, 31, 0, 2, false);
    assert_eq!(w.sys.most_recent_location(0), Some((30, 31)));
    assert_eq!(w.sys.most_recent_location(9), Some((7, 8)));
    w.host.occs.borrow_mut().get_mut(&o).unwrap().in_manager = false;
    // §2.7 (DS35): the previous call resolved the entry and wrote (7, 8)
    // back into it; a gone occupant changes nothing, so the entry keeps the
    // cell of its last resolution rather than the (0, 0) it was registered
    // with (§18 test 15: slot 12 writes it, slot 17 then reads it).
    assert_eq!(
        w.sys.most_recent_location(9),
        Some((7, 8)),
        "the last resolved cell once gone"
    );
    assert_eq!(w.sys.most_recent_location(5), None);
    assert_eq!(w.host.refs.borrow().get(&o).copied(), Some(1));
    w.sys.go_to_next_disaster();
    w.sys.go_to_next_disaster();
    assert_eq!(w.host.take(), vec!["go_to 30 31 2", "go_to 7 8 3"], "newest first");
    w.sys.unregister_location_by_occupant(9, o);
    assert_eq!(w.host.refs.borrow().get(&o).copied(), Some(0));
    assert_eq!(w.sys.location_count(), 2);
    w.sys.unregister_locations(9);
    assert_eq!(w.sys.location_count(), 1);
    // focus + follow moves the camera.
    w.host.follow.set(true);
    w.sys.register_location(4, 5, 6, 0, 1, true);
    assert!(!w.host.take().is_empty());
}

// ---- §18 test 11 ----

#[test]
fn t11_location_cursor() {
    let w = world();
    for (i, x) in [(1, 10), (2, 20), (3, 30)] {
        w.sys.register_location(i, x, 0, 0, 1, false);
    }
    let go = |n: usize| {
        for _ in 0..n {
            w.sys.go_to_next_disaster();
        }
        w.host.take()
    };
    assert_eq!(
        go(4),
        vec!["go_to 30 0 1", "go_to 20 0 1", "go_to 10 0 1", "go_to 30 0 1"]
    );
    let w = world();
    for (i, x) in [(1, 10), (2, 20), (3, 30)] {
        w.sys.register_location(i, x, 0, 0, 1, false);
    }
    w.sys.go_to_next_disaster();
    w.host.take();
    w.sys.register_location(4, 40, 0, 0, 1, false);
    w.sys.go_to_next_disaster();
    assert_eq!(w.host.take(), vec!["go_to 40 0 1"], "the new entry is next");
    // The unregister calls leave the cursor; its use clamps it.
    assert_eq!(w.sys.layer_snapshot().cursor, Some(2));
    w.sys.unregister_locations(3);
    w.sys.unregister_locations(4);
    w.sys.go_to_next_disaster();
    assert_eq!(w.host.take(), vec!["go_to 20 0 1"]);
    // No entry: nothing.
    w.sys.unregister_locations(1);
    w.sys.unregister_locations(2);
    w.sys.go_to_next_disaster();
    assert!(w.host.take().is_empty());
}

#[test]
fn ended_without_begin_does_nothing() {
    let w = world();
    w.sys.disaster_has_ended();
    assert!(w.host.take().is_empty());
    w.sys.disaster_has_begun(0);
    w.sys.disaster_has_begun(0);
    assert_eq!(w.host.emergency.get(), 1);
    w.sys.disaster_has_ended();
    assert_eq!(w.host.emergency.get(), 1);
    w.sys.disaster_has_ended();
    assert_eq!(w.host.emergency.get(), 0);
}

#[test]
fn earthquake_runs_to_its_end() {
    let w = world();
    let id = w.sys.attempt_start(kind::EARTHQUAKE, 500, 9000).expect("quake");
    assert!(w.sys.attempt_start(kind::EARTHQUAKE, 1, 1).is_none(), "one at a time");
    let q = w.sys.quake_snapshot(id).unwrap();
    assert_eq!(q.state, 2);
    assert_eq!(q.epicentre, (500.0, 1023.0), "clamped to the city");
    assert_eq!(q.cell, (31, 63));
    let log = w.host.take();
    assert!(log.contains(&"effect_at QuakeStart".to_string()));
    assert!(log.iter().any(|l| l == "repulsor"));
    assert!(log.contains(&"emergency_pause".to_string()));
    let mut ticks = 0;
    while w.sys.quake_snapshot(id).is_some() && ticks < 1000 {
        w.sys.tick(id, 100_000);
        ticks += 1;
    }
    assert!(ticks < 1000);
    let log = w.host.take();
    assert!(log.iter().any(|l| l == "brush 11") && log.iter().any(|l| l == "brush 12"));
    for e in ["end_long_terrain", "stop_textures", "emergency_resume"] {
        assert!(log.contains(&e.to_string()), "{e}");
    }
    assert_eq!(w.sys.manager_snapshot(kind::EARTHQUAKE).unwrap().instances.len(), 0);
}

#[test]
fn pipe_burst_spreads_and_ends() {
    let w = world();
    w.host.pipes.borrow_mut().insert((3, 3), 0x500);
    w.host.pipes.borrow_mut().insert((4, 3), 0x501);
    w.host.pipe_neighbours.borrow_mut().insert(0x500, vec![(4, 3), (99, 3)]);
    // The burst times: the low bound for both leaks.
    w.f64s.borrow_mut().extend([0.0, 0.0]);
    assert!(w.sys.attempt_start(kind::PIPE_BURST, 9, 9).is_none());
    let id = w.sys.attempt_start(kind::PIPE_BURST, 3, 3).expect("burst");
    assert!(w.sys.attempt_start(kind::PIPE_BURST, 3, 3).is_none(), "already leaks");
    let p = w.sys.pipe_snapshot(id).unwrap();
    assert_eq!(p.leaks[0].burst_us, 30_000_000);
    let log = w.host.take();
    assert!(log.contains(&"post EA1F0DCD 3 3 0 0 0".to_string()));
    w.sys.tick(id, 30_000_000);
    let p = w.sys.pipe_snapshot(id).unwrap();
    assert!(p.leaks.is_empty());
    assert_eq!(p.waits, vec![(vec![(4, 3)], 0)]);
    for _ in 0..4 {
        w.sys.tick(id, 1);
    }
    assert_eq!(w.sys.pipe_snapshot(id).unwrap().leaks.len(), 1);
    w.sys.tick(id, 30_000_000);
    for _ in 0..4 {
        w.sys.tick(id, 1);
    }
    assert!(w.sys.pipe_snapshot(id).is_none());
    let log = w.host.take();
    assert_eq!(log.iter().filter(|l| l.starts_with("demolish_pipe")).count(), 2);
    assert!(log.contains(&"emergency_resume".to_string()));
}

#[test]
fn pipe_leak_without_effect_waits_for_the_view() {
    let w = world();
    w.host.view_level.set(2);
    w.host.pipes.borrow_mut().insert((3, 3), 0x500);
    let id = w.sys.attempt_start(kind::PIPE_BURST, 3, 3).unwrap();
    w.sys.tick(id, 100_000_000);
    assert_eq!(w.sys.pipe_snapshot(id).unwrap().leaks.len(), 1, "never bursts");
    let m = DisasterMessage {
        data1: 3,
        ..DisasterMessage::of_type(msg::PRE_VIEW_LEVEL_CHANGE)
    };
    w.sys.do_message(Target::Instance(id), &m);
    w.sys.tick(id, 1);
    assert!(w.sys.pipe_snapshot(id).unwrap().leaks.is_empty());
}

#[test]
fn pipe_random_start_spirals() {
    let w = world();
    w.host.pipes.borrow_mut().insert((40, 40), 0x900);
    let s = Script::default();
    s.u32s.borrow_mut().extend([38, 41]);
    w.sys.set_random(Generator::PipeBurstStart, Box::new(s));
    let id = w.sys.attempt_start_any(kind::PIPE_BURST).expect("found");
    assert_eq!(w.sys.pipe_snapshot(id).unwrap().pipe, 0x900);
    assert!(w.sys.attempt_start_any(kind::RIOT).is_none());
}

// ---- §18 test 14 ----

fn volcano_world() -> World {
    let w = world();
    w.host
        .tuning
        .borrow_mut()
        .insert(tuning_instance::VOLCANO, Rc::new(shipped_volcano()));
    w
}

#[test]
fn t14_volcano_phases() {
    let w = volcano_world();
    let id = w.sys.attempt_start(kind::VOLCANO, 100, 100).unwrap();
    assert!(w.sys.attempt_start(kind::VOLCANO, 1, 1).is_none(), "one at a time");
    let v = w.sys.volcano_snapshot(id).unwrap();
    assert_eq!((v.state, v.phase), (5, 2));
    assert_eq!(w.sys.instance_info(id).unwrap().phase, 2);
    assert_eq!(w.sys.active_count(kind::VOLCANO), 0, "phase 2 is not active");
    let log = w.host.take();
    assert_eq!(log.iter().filter(|l| *l == "volcano_deform").count(), 1);
    assert!(log.contains(&"lava 100".to_string()), "{log:?}");
    assert!(log.contains(&"emergency_pause".to_string()));
    // SetLocation's entry and Init's [question, DS42].
    assert_eq!(w.sys.layer_snapshot().locations.len(), 2);
    // T = 50 s: at exactly T the lava still flows; past it, it ends.
    for _ in 0..50 {
        w.sys.tick(id, 1_000_000);
    }
    assert_eq!(w.sys.volcano_snapshot(id).unwrap().state, 5);
    assert!(w.sys.is_disaster_ongoing());
    w.host.take();
    w.sys.tick(id, 1);
    let log = w.host.take();
    assert_eq!(log.iter().filter(|l| *l == "emergency_resume").count(), 1);
    assert_eq!(
        log.iter().filter(|l| l.starts_with("post AA1B2002")).count(),
        1,
        "DisasterHasEnded once"
    );
    assert!(log.contains(&"lava_end".to_string()));
    let v = w.sys.volcano_snapshot(id).unwrap();
    assert_eq!((v.state, v.phase), (3, 1));
    assert!(v.lava_has_run && !v.lava_flowing);
    assert_eq!(w.sys.active_count(kind::VOLCANO), 1);
    assert!(w.sys.layer_snapshot().locations.is_empty());
    // It waits in state 3 for the end request: the effect's completion for
    // this instance (another instance's is not it).
    w.sys.tick(id, 50_000_000);
    assert_eq!(w.sys.volcano_snapshot(id).unwrap().state, 3);
    let done = |d1: i64| DisasterMessage {
        data1: d1,
        ..DisasterMessage::of_type(msg::VOLCANO_MESSAGE)
    };
    assert!(!w.sys.do_message(Target::Instance(id), &done(i64::from(id) + 1)));
    assert!(w.sys.volcano_snapshot(id).is_some());
    assert!(w.sys.do_message(Target::Instance(id), &done(i64::from(id))));
    assert!(w.sys.volcano_snapshot(id).is_none(), "state 8 removes it");
    assert!(w.sys.manager_instances(kind::VOLCANO).is_empty());
    assert!(!w.sys.is_disaster_ongoing());
    assert!(w.sys.attempt_start(kind::VOLCANO, 10, 10).is_some());
}

#[test]
fn volcano_end_request_and_no_lava() {
    // Asked to end during the flow: it stays in the flow, then goes.
    let w = volcano_world();
    let id = w.sys.attempt_start(kind::VOLCANO, 100, 100).unwrap();
    assert!(w.sys.volcano_request_end(id));
    let v = w.sys.volcano_snapshot(id).unwrap();
    assert_eq!(v.state, 5, "it stays in the flow");
    assert!(v.end_requested);
    w.sys.tick(id, 25_000_000);
    w.sys.tick(id, 25_000_001);
    assert!(w.sys.volcano_snapshot(id).is_none());
    let log = w.host.take();
    assert_eq!(log.iter().filter(|l| l.starts_with("post AA1B2002")).count(), 1);
    // Without the exemplar the lava is disabled: the volcano waits in state
    // 3 (phase 1); the end request removes it and its Shutdown ends the
    // disaster.
    let w = world();
    let id = w.sys.attempt_start(kind::VOLCANO, 100, 100).unwrap();
    let v = w.sys.volcano_snapshot(id).unwrap();
    assert_eq!((v.state, v.phase), (3, 1));
    w.sys.tick(id, 100_000_000);
    assert_eq!(w.sys.volcano_snapshot(id).unwrap().state, 3);
    assert!(w.sys.is_disaster_ongoing());
    assert!(w.sys.volcano_request_end(id));
    assert!(w.sys.volcano_snapshot(id).is_none());
    assert!(!w.sys.is_disaster_ongoing());
    assert!(!w.host.take().iter().any(|l| l.starts_with("lava")));
    // The rates; both 0 without production points.
    let t = VolcanoTuning::from_holder(Some(&shipped_volcano()));
    let (p, s) = t.rates(25_000_000);
    assert!((p - 0.25 * 6.0 / 100_000.0).abs() < 1e-9);
    assert!((s - 0.002 / 100_000.0).abs() < 1e-12);
    assert_eq!(VolcanoTuning::default().rates(1), (0.0, 0.0));
    let d = VolcanoTuning::default();
    assert_eq!(
        (d.lava_seconds, d.scale, d.vent_radius, d.lava_flows),
        (20.0, 1.0, 100.0, false)
    );
    // The volcano's GetPhase.
    let phases: Vec<u32> = (0..=8).map(volcano_phase).collect();
    assert_eq!(phases, vec![0, 2, 1, 1, 3, 2, 2, 2, 2]);
}

#[test]
fn effect_disaster_lifecycle() {
    let w = world();
    let id = w.sys.start_disaster_effect(0xE1, 160.0, 320.0).unwrap();
    assert_eq!(w.sys.instance_info(id).unwrap().location, Some((10, 20)));
    assert!(w.sys.is_disaster_ongoing());
    let moved = DisasterMessage {
        void1: 0xE1,
        x: 32.0,
        z: 48.0,
        ..DisasterMessage::of_type(msg::EFFECT_MOVED)
    };
    assert!(w.sys.do_message(Target::Instance(id), &moved));
    assert_eq!(w.sys.instance_info(id).unwrap().location, Some((2, 3)));
    let other = DisasterMessage {
        void1: 0xE2,
        ..DisasterMessage::of_type(msg::EFFECT_FINISHED)
    };
    assert!(!w.sys.do_message(Target::Instance(id), &other));
    let done = DisasterMessage {
        void1: 0xE1,
        ..DisasterMessage::of_type(msg::EFFECT_FINISHED)
    };
    assert!(w.sys.do_message(Target::Instance(id), &done));
    assert!(w.sys.instance_info(id).is_none());
    assert!(!w.sys.is_disaster_ongoing());
}

#[test]
fn toxic_cheat_and_shutdown() {
    let w = world();
    let cheat = DisasterMessage {
        data1: 5,
        data2: 6,
        data3: 40,
        ..DisasterMessage::of_type(msg::TOXIC_SPILL_CHEAT)
    };
    assert!(w.sys.do_message(Target::Manager(kind::TOXIC), &cheat));
    assert!(w.sys.do_message(Target::Manager(kind::TOXIC), &cheat));
    let s = w.sys.toxic_snapshot().unwrap();
    assert_eq!(s.spills.len(), 1);
    assert_eq!(s.spills[0].amount, 80);
    assert!(w.sys.shutdown());
    assert!(w.sys.toxic_snapshot().is_none());
    let log = w.host.take();
    assert!(log.iter().any(|l| l.starts_with("remove_toxic")));
    assert!(log.contains(&"unsubscribe manager EBFB0404 0BFB41BA".to_string()));
}

#[test]
fn default_generators_are_seeded_from_the_session() {
    // Two systems with the same session seed draw the same fire IDs.
    let ids: Vec<u32> = (0..2)
        .map(|_| {
            let host = Fake::new();
            let sys = Disasters::new(host.clone(), 0xC0FFEE);
            sys.init();
            let b = host.add(occupant::BUILDING, 1, 1);
            sys.ignite(b, false, 0, false);
            sys.fire_snapshot().unwrap().fires[0].fire_id
        })
        .collect();
    assert_eq!(ids[0], ids[1]);
    let mut r = crate::rng::RzRandom::new(
        crate::rng::derive_seed(0xC0FFEE, seed::FIRE),
        &DerivedSeeds(crate::rng::derive_seed(0xC0FFEE, seed::FIRE)),
    );
    assert_eq!(ids[0], r.next_raw());
}

#[test]
fn flame_counts_from_the_curves() {
    let w = world();
    let b = w.host.add(occupant::BUILDING, 1, 1);
    w.sys.ignite(b, false, 0, false);
    // D7(256) = 1 + 4 × 256/10000 = 1.1 → 1; D8(512) = 6 × 512/10000 → 0.
    assert_eq!((w.fire().top_flames, w.fire().side_flames), (1, 0));
    let mut h = PropertyHolder::new();
    h.add(prop::ZOOM_LEVEL, Variant::Scalar(Elem::U8(7)));
    assert_eq!(FireTuning::from_holder(Some(&h)).zoom, 7);
}

#[test]
fn mirror_random_numbers_ids_and_never_spreads() {
    let w = world();
    w.sys.set_random(Generator::Fire, Box::new(MirrorRandom::default()));
    let a = w.host.add(occupant::BUILDING, 5, 5);
    let b = w.host.add(occupant::BUILDING, 6, 5);
    w.host.flammability.borrow_mut().insert((6, 5), 255);
    assert!(w.sys.ignite(a, false, 0, false));
    assert_eq!(w.fire().fire_id, MIRROR_FIRST_ID);
    // The next drawn ID follows; a given ID is kept.
    assert!(w.sys.ignite(b, false, 0, false));
    assert_eq!(w.sys.fire_snapshot().unwrap().fires[1].fire_id, MIRROR_FIRST_ID + 1);
    w.sys.extinguish(b);
    w.sys.fire_update();
    for _ in 0..53 {
        w.second();
    }
    // No spread in 54 updates (the probability draw fails), and the
    // extinguished fire died at once (a zero Random(4000)).
    let s = w.sys.fire_snapshot().unwrap();
    assert_eq!(s.fires.len(), 1, "{:?}", s.fires);
    assert_eq!(s.fires[0].target, a);
    // IgniteCell with the probability test never ignites.
    assert!(!w.sys.ignite_cell(6, 5, true, false, 9));
    let mut m = MirrorRandom::default();
    assert_eq!(m.uniform_u32(5), 0);
    assert_eq!(m.uniform_f64(), 1.0);
    assert_eq!(m.range_f64(0.0, 100.0), 100.0);
    assert_eq!(m.range_gaussian_fast(-3.0, 4.0), -3.0);
}

// ---- §18 tests 15, 17 and 20 (DS35, DS34, DS40) ----

/// §5.8 (DS40): `UpdateAllFires` remembers the next node before each step, so
/// a fire a spread lights is stepped in the same pass when its spawner was not
/// the last node when that step began. Fires [A, B]; A's step lights C:
/// C is stepped after B (§18 test 17).
#[test]
fn a_fire_lit_by_a_spread_is_stepped_in_the_same_pass_when_it_was_not_last() {
    let w = world();
    let a = w.host.add(occupant::BUILDING, 5, 5);
    let b = w.host.add(occupant::BUILDING, 8, 8);
    let c = w.host.add(occupant::BUILDING, 6, 5);
    w.host.flammability.borrow_mut().insert((6, 5), 255);
    w.sys.ignite(a, false, 0x10, false);
    w.sys.ignite(b, false, 0x10, false);
    // A's step: spread draw 0.1 < 1/4, the Gaussians toward (6, 5), then the
    // IgniteCell probability draw. B's and C's steps draw 0.9 (no spread).
    w.f64s.borrow_mut().extend([0.1, 0.65, 0.5, 0.0, 0.9, 0.9]);
    w.sys.fire_update();
    let s = w.sys.fire_snapshot().unwrap();
    let targets: Vec<u64> = s.fires.iter().map(|f| f.target).collect();
    assert_eq!(targets, vec![a, b, c], "{:?}", s.fires);
    assert_eq!(s.fires[0].counter, 1);
    assert_eq!(s.fires[1].counter, 1);
    assert_eq!(s.fires[2].counter, 1, "C stepped in the pass that lit it, after B");
}

/// The other half of §18 test 17: a fire lit by the **last** node is not
/// stepped in that pass (its remembered next was the end).
#[test]
fn a_fire_lit_by_the_last_node_waits_for_the_next_pass() {
    let w = world();
    let a = w.host.add(occupant::BUILDING, 5, 5);
    let c = w.host.add(occupant::BUILDING, 6, 5);
    w.host.flammability.borrow_mut().insert((6, 5), 255);
    w.sys.ignite(a, false, 0x10, false);
    w.f64s.borrow_mut().extend([0.1, 0.65, 0.5, 0.0, 0.9, 0.9]);
    w.sys.fire_update();
    let s = w.sys.fire_snapshot().unwrap();
    assert_eq!(s.fires.len(), 2, "{:?}", s.fires);
    assert_eq!(s.fires[1].target, c);
    assert_eq!(s.fires[1].counter, 0, "not stepped in the pass that lit it");
    w.second();
    let s = w.sys.fire_snapshot().unwrap();
    assert_eq!(s.fires[0].counter, 2);
    assert_eq!(s.fires[1].counter, 1, "stepped in the next pass");
}

/// §2.7 (DS35): only `GetMostRecentDisasterLocation` (slot 12) and
/// `GetNextDisasterLocation` resolve, and a resolution writes the cell back
/// into the entry. `GetDisasterLocationByIndex` (slot 17) reads the stored
/// values (§18 test 15).
#[test]
fn only_the_resolving_reads_change_a_location_entry() {
    let w = world();
    let o = w.host.add(occupant::BUILDING, 7, 8);
    w.sys.register_location(9, 0, 0, o, 3, false);
    let l = w.sys.location_by_index(0).unwrap();
    assert_eq!((l.x, l.z), (0, 0), "slot 17 does not resolve");
    assert_eq!(w.sys.most_recent_location(9), Some((7, 8)));
    let l = w.sys.location_by_index(0).unwrap();
    assert_eq!((l.x, l.z), (7, 8), "slot 12 wrote the cell back (§18 test 15)");
}

/// §5.4 step 3 (DS38): the flammability property is read as an unsigned
/// integer (variant slot 25, converting), so a fractional value truncates.
#[test]
fn flammability_is_read_as_an_unsigned_integer() {
    let w = world();
    let a = w.host.add(occupant::BUILDING, 5, 5);
    w.host.set_prop(a, prop::FLAMMABILITY, 0.5);
    assert!(!w.sys.ignite(a, true, 0, false), "0.5 truncates to 0 and refuses");
    w.host.set_prop(a, prop::FLAMMABILITY, 75.9);
    assert!(w.sys.ignite(a, true, 0, false), "75.9 truncates to 75");
}

/// §5.6 step 4 (DS34): only a flammable occupant inside `r0` is queued for
/// the toast list (§18 test 20).
#[test]
fn a_radius_toast_needs_a_flammable_occupant() {
    let w = world();
    let a = w.host.add(occupant::BUILDING, 4, 4);
    let b = w.host.add(occupant::BUILDING, 4, 4);
    w.host.set_prop(a, prop::FLAMMABILITY, 0.0);
    w.host.set_prop(b, prop::FLAMMABILITY, 100.0);
    w.sys.ignite_cell_radius(4, 4, 1, 1);
    let s = w.sys.fire_snapshot().unwrap();
    assert!(s.toast.contains(&b), "{:?}", s.toast);
    assert!(!s.toast.contains(&a), "flammability 0 is not queued (§18 test 20)");
}

/// §18 test 15 in the shadow's shape: an `Ignite` registers (0, 0) and
/// `DisasterHasBegun` runs inside the same call, so slot 17 already reads the
/// target's cell when `Ignite` returns (DS35).
#[test]
fn an_ignite_resolves_its_location_before_it_returns() {
    let w = world();
    let o = w.host.add(occupant::BUILDING, 58, 81);
    assert!(w.sys.ignite(o, false, 0x10, true));
    let l = w.sys.location_by_index(0).unwrap();
    assert_eq!((l.x, l.z), (58, 81), "DisasterHasBegun resolved it in the same Ignite");
    assert_eq!(l.occupant, o);
    assert_eq!(
        w.sys.location_by_index(0).unwrap().x,
        58,
        "slot 17 reads the stored cell"
    );
}

/// §6.5 (DS40's riot half): a leader step 4 spawns is processed in the same
/// pass when its spawner was not the last node when that step began - the
/// §5.8 rule of the fire list, applied to the leader list (`0x0057E026`).
#[test]
fn a_leader_spawned_by_a_non_last_leader_is_processed_in_the_same_pass() {
    let w = riot_world();
    w.f64s.borrow_mut().extend([0.5, 0.25]);
    let id = w.riot(10, 20).expect("riot");
    // Step 30: the only leader reaches stage 1 and spawns. It was the last
    // node, so the new one waits for the next pass.
    for n in 1..=30 {
        w.riot_tick(id, if n == 1 { 0 } else { 1000 });
        w.host.take();
    }
    let r = w.sys.riot_snapshot(id).unwrap();
    assert_eq!(r.leaders.len(), 2, "{:?}", r.leaders);
    assert_eq!(r.leaders[1].counter, 0, "the spawner was the last node");
    // Step 90: the first leader reaches stage 3 and spawns again. It is not
    // the last node (the step-30 leader is behind it), so the new one is
    // processed in this pass and its counter is 1, not 0.
    for _ in 31..=90 {
        w.riot_tick(id, 1000);
        w.host.take();
    }
    let r = w.sys.riot_snapshot(id).unwrap();
    assert_eq!(r.leaders.len(), 5, "{:?}", r.leaders);
    assert_eq!(
        r.leaders[4].counter, 1,
        "spawned by a non-last leader, stepped in the same pass"
    );
}
