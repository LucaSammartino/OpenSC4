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
    fn set_fire_dispatch_effectiveness(&self, v: f32) {
        self.log(format!("fire_effectiveness {v}"));
    }
    fn set_police_dispatch_effectiveness(&self, v: f32) {
        self.log(format!("police_effectiveness {v}"));
    }
    fn building_type_flags(&self, o: u64) -> u32 {
        self.occ(o, |x| x.flags).unwrap_or(0)
    }
    fn network_default_cost(&self, _o: u64) -> i64 {
        77
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
        self.occ(o, |x| CellRect::cell(x.cell.0, x.cell.1)).unwrap_or_default()
    }
    fn occupant_position(&self, o: u64) -> (f32, f32) {
        self.occ(o, |x| (x.cell.0 as f32 * 16.0 + 8.0, x.cell.1 as f32 * 16.0 + 8.0))
            .unwrap_or((0.0, 0.0))
    }
    fn occupant_bbox(&self, o: u64) -> WorldBox {
        let (x, z) = self.occ(o, |x| x.cell).unwrap_or((0, 0));
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
        self.log(format!("create_fire {t} {target} {id:08X} {} {}", flames.0, flames.1));
        Some(t)
    }
    fn remove_fire_occupant(&self, fire: u64, target: u64) {
        if let Some(x) = self.occs.borrow_mut().get_mut(&target) {
            x.burning = false;
        }
        self.log(format!("remove_fire {fire} {target}"));
    }
    fn add_fire_aura(&self, _o: u64) {}
    fn tutorial_no_burn_down(&self) -> bool {
        self.no_burn_down.get()
    }
    fn wind(&self) -> f32 {
        0.0
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
    fn volcano_end_deformation(&self, _i: u32) {
        self.log("volcano_end_deform");
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
/// n − 1, doubles 0.99, ranges `a`, Gaussian ranges the midpoint).
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
        let u = self.f64s.borrow_mut().pop_front().unwrap_or(0.0);
        a + u * (b - a)
    }
    fn range_gaussian(&mut self, a: f64, b: f64) -> f64 {
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
    assert_eq!(d.d5, [0, 3, 5, 7, 9]);
    assert_eq!(d.zoom, 4);
    assert!(d.d9.is_empty());
}

// ---- §18 test 3 ----

#[test]
fn t03_unattended_burn_down() {
    let w = world();
    let b = w.host.add(occupant::BUILDING, 5, 5);
    assert!(w.sys.ignite(b, false, 0, true));
    w.sys.fire_update(); // the tick right after the ignition: update 1
    for n in 2..=29 {
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
    let id = w.sys.attempt_start(kind::RIOT, 10, 20).expect("riot");
    let log = w.host.take();
    assert!(log.contains(&"leader 4096 168 324".to_string()), "{log:?}");
    assert!(log.contains(&"history AA8C87C7 1".to_string()));
    let first = w.sys.riot_snapshot(id).unwrap().leaders[0].automaton;
    let tick = || {
        w.host.advance(1000);
        w.sys.tick(id, 16_000);
    };
    for s in 1..=180 {
        tick();
        let r = w.sys.riot_snapshot(id).unwrap();
        let alive = r.leaders.iter().any(|l| l.automaton == first);
        match s {
            30 => assert_eq!(r.leaders.len(), 2, "stage 1 spawns one"),
            // At 90 s the first leader reaches stage 3 and the leader spawned at
            // 60 s (by the one spawned at 30 s) reaches stage 1: two more.
            90 => assert_eq!(r.leaders.len(), 5),
            _ => {}
        }
        if s < 180 {
            assert!(alive, "first leader alive at {s}");
        } else {
            assert!(!alive, "first leader ends at 180 s");
        }
    }
    let log = w.host.take();
    assert!(log.contains(&format!("riot_message {first} 1")));
    assert!(log.contains(&format!("riot_message {first} 3")));
    assert!(log.contains(&format!("end_leader {first}")));
}

#[test]
fn t07b_riot_suppressed() {
    let w = riot_world();
    for x in 0..64 {
        w.host.police.borrow_mut().insert((x, 20), 255);
    }
    // Coverage 255 refuses the leader (curve 5 ≥ 2): start at coverage 0,
    // then raise it.
    w.host.police.borrow_mut().insert((10, 20), 0);
    let id = w.sys.attempt_start(kind::RIOT, 10, 20).expect("riot");
    let a = w.sys.riot_snapshot(id).unwrap().leaders[0].automaton;
    w.host.police.borrow_mut().insert((10, 20), 255);
    w.host.take();
    w.host.advance(1000);
    w.sys.tick(id, 16_000);
    let log = w.host.take();
    assert!(log.contains(&format!("riot_effect {a} -1")));
    assert!(log.contains(&format!("end_leader {a}")));
    assert!(log.contains(&"post AA1B2002 0 0 0 0 0".to_string()));
    assert!(w.sys.riot_snapshot(id).is_none(), "the riot left its manager");
    assert_eq!(w.sys.active_count(kind::RIOT), 0);
}

/// network-manager.md §10 (disasters.md Q4): the riots lock the network
/// pieces around their starts and leaders against the player's bulldozer
/// until the last riot leaves the manager.
#[test]
fn riot_locks_the_network_until_the_last_riot_ends() {
    let w = riot_world();
    for x in 0..64 {
        w.host.network.borrow_mut().insert((x, 20), 0x7000 + x as u64);
    }
    let a = w.sys.attempt_start(kind::RIOT, 10, 20).unwrap();
    let row = |r: std::ops::RangeInclusive<u64>| r.map(|x| 0x7000 + x).collect::<Vec<_>>();
    assert_eq!(w.sys.riot_locked(), row(7..=13), "each piece once");
    assert_eq!(w.host.count("unset_removal"), 7);
    assert_eq!(w.host.refs.borrow().get(&0x700A).copied(), Some(1));
    let b = w.sys.attempt_start(kind::RIOT, 12, 20).unwrap();
    assert_eq!(w.sys.riot_locked(), row(7..=15));
    // Within 3 cells of the west edge nothing is locked.
    let c = w.sys.attempt_start(kind::RIOT, 2, 20).unwrap();
    assert_eq!(w.sys.riot_locked().len(), 9);
    w.host.take();
    // Each riot ends at coverage 255 on its leader's cell; nothing unlocks
    // while one remains.
    let end = |id: u32, cell: (i32, i32)| {
        w.host.police.borrow_mut().insert(cell, 255);
        w.host.advance(1000);
        w.sys.tick(id, 16_000);
        assert!(w.sys.riot_snapshot(id).is_none());
    };
    end(a, (10, 20));
    end(b, (12, 20));
    assert_eq!(w.host.count("set_removal"), 0);
    assert_eq!(w.sys.riot_locked().len(), 9);
    end(c, (2, 20));
    assert!(w.sys.riot_locked().is_empty());
    assert_eq!(w.host.count("set_removal"), 9);
    assert!(w.host.refs.borrow().values().all(|r| *r == 0));
    // The manager's Shutdown unlocks too.
    w.host.police.borrow_mut().clear();
    w.sys.attempt_start(kind::RIOT, 30, 20).unwrap();
    assert_eq!(w.sys.riot_locked().len(), 7);
    w.host.take();
    assert!(w.sys.shutdown());
    assert_eq!(w.host.count("set_removal"), 7);
    assert!(w.sys.riot_locked().is_empty());
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
    assert!(w.sys.attempt_start(kind::RIOT, 10, 20).is_some());
    assert_eq!(w.sys.active_count(kind::RIOT), 1);
    w.host.police.borrow_mut().insert((30, 20), 113);
    let id = w.sys.attempt_start(kind::RIOT, 30, 20).expect("instance");
    assert!(w.sys.riot_snapshot(id).unwrap().leaders.is_empty(), "refused leader");
    assert_eq!(w.sys.active_count(kind::RIOT), 1);
}

#[test]
fn t08b_riot_limits_and_search() {
    let w = riot_world();
    // At most three active riots.
    for x in [5, 15, 25] {
        assert!(w.sys.attempt_start(kind::RIOT, x, 20).is_some());
    }
    assert_eq!(w.sys.active_count(kind::RIOT), 3);
    assert!(w.sys.attempt_start(kind::RIOT, 35, 20).is_none());
    // The nearest road within 7 cells; none further.
    let w = riot_world();
    let id = w.sys.attempt_start(kind::RIOT, 10, 27).unwrap();
    // Ring 7, row order: the first road cell of the ring's first row.
    assert_eq!(w.sys.riot_snapshot(id).unwrap().leaders[0].cell, (3, 20));
    let id = w.sys.attempt_start(kind::RIOT, 10, 28).unwrap();
    assert!(w.sys.riot_snapshot(id).unwrap().leaders.is_empty());
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
    assert!(log.contains(&"fire_effectiveness 0.4".to_string()), "{log:?}");
    assert!(!log.iter().any(|l| l.starts_with("police_effectiveness")));
    assert_eq!(w.sys.layer_snapshot().fire_effectiveness, 0.4);
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
    assert_eq!(s.toast, vec![flora, near]);
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
    // Five per update.
    let w = world();
    let list: Vec<u64> = (0..7).map(|_| w.host.add(occupant::PROP, 3, 3)).collect();
    w.host.set_prop(list[0], prop::BURN_DOWN_DEMOLISHES, 0.0);
    w.sys.quick_burn_cell(3, 3);
    w.sys.fire_update();
    assert_eq!(w.sys.fire_snapshot().unwrap().toast.len(), 2);
    assert!(w.host.log.borrow().contains(&format!("remove_occupant {}", list[0])));
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
    assert_eq!(d.total_cost, 577);
    assert_eq!(d.kinds, [(0, 0), (1, 500), (0, 0), (1, 500)]);
    assert_eq!(d.special.len(), 1);
    assert_eq!(w.sys.disaster_cost(), 577);
    // Not established: nothing counts.
    w.host.established.set(false);
    w.sys.do_message(Target::Layer, &m(b));
    assert_eq!(w.sys.disaster_cost(), 577);
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
    assert_eq!(w.sys.most_recent_location(9), Some((0, 0)), "stored x, z once gone");
    assert_eq!(w.sys.most_recent_location(5), None);
    assert_eq!(w.host.refs.borrow().get(&o).copied(), Some(1));
    w.sys.go_to_next_disaster();
    w.sys.go_to_next_disaster();
    assert_eq!(w.host.take(), vec!["go_to 1 2 3", "go_to 0 0 3"]);
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

#[test]
fn volcano_cycles_until_asked_to_end() {
    let w = world();
    let id = w.sys.attempt_start(kind::VOLCANO, 100, 100).unwrap();
    assert!(w.sys.attempt_start(kind::VOLCANO, 1, 1).is_none());
    assert_eq!(w.sys.volcano_snapshot(id).unwrap().state, 3);
    w.sys.tick(id, 1);
    assert_eq!(w.sys.volcano_snapshot(id).unwrap().state, 5);
    w.sys.tick(id, 50_000_000);
    assert_eq!(w.sys.volcano_snapshot(id).unwrap().state, 3);
    w.sys.tick(id, 1);
    assert!(w.sys.volcano_request_end(id));
    w.sys.tick(id, 50_000_000);
    assert!(w.sys.volcano_snapshot(id).is_none());
    let log = w.host.take();
    assert_eq!(log.iter().filter(|l| *l == "lava 20").count(), 2);
    assert!(log.contains(&"volcano_end_deform".to_string()));
    assert!(log.contains(&"emergency_resume".to_string()));
    let t = VolcanoTuning::default();
    let (p, s) = t.rates(25_000_000);
    assert!((p - 0.25 * 6.0 / 100_000.0).abs() < 1e-9);
    assert!((s - 0.002 / 100_000.0).abs() < 1e-12);
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
